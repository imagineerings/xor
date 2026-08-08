use crate::{
    AttemptEvent, AttemptEventKind, AttemptState, CacheEntry, CacheKey, CompiledNode, CompiledPlan,
    EffectClass, EventBusError, ExecutionEventBus, InputBinding, InputMode, NativeCache,
    PromptCompileError, RuntimeCachePolicy, RuntimeNodeDescriptor, RuntimeNodePresentation,
    RuntimeOutputDescriptor,
};
use chrono::Utc;
use comfy_tensor::{BackendCapabilityMatrix, ScratchReservation};
#[cfg(test)]
use comfy_tensor::{CpuWorkspaceAuthority, DeviceId, NativeDeviceProperties};
use comfy_types::{AttemptId, CancellationToken, DeviceKind, NodeId, ProfileId, PromptId};
use futures::future::BoxFuture;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use thiserror::Error;
use uuid::Uuid;

pub const MAX_EXPANSION_DEPTH: usize = 64;
pub const MAX_EFFECTS_PER_NODE: usize = 4_096;
pub const MAX_EFFECT_METADATA_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_NATIVE_COMPILE_OPTIONS: usize = 64;
pub const MAX_NATIVE_COMPILE_TEXT_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCompileBackend {
    Graph,
    CudaGraphs,
}

impl NativeCompileBackend {
    pub fn from_source_name(value: &str) -> Result<Self, NativeCompileError> {
        match value {
            "inductor" | "native" => Ok(Self::Graph),
            "cudagraphs" => Ok(Self::CudaGraphs),
            _ => Err(NativeCompileError::UnsupportedBackend(value.to_owned())),
        }
    }

    const fn cache_dimension(self) -> &'static str {
        match self {
            Self::Graph => "native-graph",
            Self::CudaGraphs => "native-cudagraphs",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NativeCompilePolicy {
    backend: NativeCompileBackend,
    guard_policy: NativeCompileGuardPolicy,
    mode: Option<String>,
    fullgraph: bool,
    dynamic: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCompileGuardPolicy {
    ExactTypedInputs,
    SkipTransformerOptionsDictionary,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCompilePhase {
    #[default]
    Eager,
    CapturingGraph,
}

pub fn compiler_is_compiling_exact_native(
    phase: NativeCompilePhase,
    cancellation: &CancellationToken,
) -> Result<bool, NativeCompileError> {
    cancellation
        .check()
        .map_err(|_| NativeCompileError::Cancelled)?;
    Ok(phase == NativeCompilePhase::CapturingGraph)
}

impl NativeCompilePolicy {
    pub fn from_source_configuration(
        backend: &str,
        mut options: BTreeMap<String, String>,
        mode: Option<String>,
        fullgraph: bool,
        dynamic: Option<bool>,
    ) -> Result<Self, NativeCompileError> {
        if options.len() > MAX_NATIVE_COMPILE_OPTIONS {
            return Err(NativeCompileError::TooManyOptions);
        }
        for (key, value) in &options {
            validate_compile_text("option key", key, false)?;
            validate_compile_text("option value", value, true)?;
        }
        let guard_policy = match options.remove("guard_filter_fn") {
            None => NativeCompileGuardPolicy::ExactTypedInputs,
            Some(value) if value == "skip_torch_compile_dict" => {
                NativeCompileGuardPolicy::SkipTransformerOptionsDictionary
            }
            Some(value) => {
                return Err(NativeCompileError::UnsupportedOption {
                    key: "guard_filter_fn".to_owned(),
                    value,
                });
            }
        };
        if let Some((key, value)) = options.into_iter().next() {
            return Err(NativeCompileError::UnsupportedOption { key, value });
        }
        if let Some(mode) = mode.as_deref() {
            validate_compile_text("mode", mode, false)?;
            if !matches!(
                mode,
                "default" | "reduce-overhead" | "max-autotune" | "max-autotune-no-cudagraphs"
            ) {
                return Err(NativeCompileError::UnsupportedMode(mode.to_owned()));
            }
        }
        Ok(Self {
            backend: NativeCompileBackend::from_source_name(backend)?,
            guard_policy,
            mode,
            fullgraph,
            dynamic,
        })
    }

    pub const fn backend(&self) -> NativeCompileBackend {
        self.backend
    }

    pub const fn guard_policy(&self) -> NativeCompileGuardPolicy {
        self.guard_policy
    }

    pub fn mode(&self) -> Option<&str> {
        self.mode.as_deref()
    }

    pub const fn fullgraph(&self) -> bool {
        self.fullgraph
    }

    pub const fn dynamic(&self) -> Option<bool> {
        self.dynamic
    }

    pub fn cache_token(&self) -> Result<String, NativeCompileError> {
        let encoded = serde_json::to_vec(self).map_err(NativeCompileError::Encode)?;
        Ok(format!("native-compile-v2:{:x}", Sha256::digest(encoded)))
    }
}

#[derive(Clone, Debug)]
pub struct NativeCompiledModel<Model> {
    model: Model,
    policy: NativeCompilePolicy,
}

impl<Model> NativeCompiledModel<Model> {
    pub fn policy(&self) -> &NativeCompilePolicy {
        &self.policy
    }

    pub fn invoke<Output>(
        &self,
        cancellation: &CancellationToken,
        operation: impl FnOnce(&Model) -> Output,
    ) -> Result<Output, NativeCompileError> {
        cancellation
            .check()
            .map_err(|_| NativeCompileError::Cancelled)?;
        let output = operation(&self.model);
        cancellation
            .check()
            .map_err(|_| NativeCompileError::Cancelled)?;
        Ok(output)
    }

    pub fn into_inner(self) -> Model {
        self.model
    }
}

pub fn compile_exact_native<Model>(
    model: Model,
    policy: NativeCompilePolicy,
    capabilities: Option<&BackendCapabilityMatrix>,
    cancellation: &CancellationToken,
) -> Result<NativeCompiledModel<Model>, NativeCompileError> {
    cancellation
        .check()
        .map_err(|_| NativeCompileError::Cancelled)?;
    policy.cache_token()?;
    if policy.backend() == NativeCompileBackend::CudaGraphs {
        let capabilities = capabilities.ok_or(NativeCompileError::UncertifiedCudaGraphs)?;
        if !matches!(
            capabilities.device().kind(),
            DeviceKind::Cuda | DeviceKind::Rocm
        ) || capabilities.device_properties().is_none()
        {
            return Err(NativeCompileError::UncertifiedCudaGraphs);
        }
    }
    cancellation
        .check()
        .map_err(|_| NativeCompileError::Cancelled)?;
    Ok(NativeCompiledModel { model, policy })
}

fn validate_compile_text(
    field: &'static str,
    value: &str,
    allow_empty: bool,
) -> Result<(), NativeCompileError> {
    if (!allow_empty && value.is_empty())
        || value.len() > MAX_NATIVE_COMPILE_TEXT_BYTES
        || value.contains('\0')
    {
        return Err(NativeCompileError::InvalidText { field });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum NativeCompileError {
    #[error("source compile backend {0:?} has no native Rust execution strategy")]
    UnsupportedBackend(String),
    #[error("source compile mode {0:?} has no native Rust execution strategy")]
    UnsupportedMode(String),
    #[error("source compile option {key:?}={value:?} has no native Rust execution strategy")]
    UnsupportedOption { key: String, value: String },
    #[error("native compile policy has too many options")]
    TooManyOptions,
    #[error("native compile {field} is empty, oversized, or contains NUL")]
    InvalidText { field: &'static str },
    #[error("native compile policy serialization failed: {0}")]
    Encode(serde_json::Error),
    #[error("native CUDA-graphs compilation requires a certified CUDA or ROCm capability matrix")]
    UncertifiedCudaGraphs,
    #[error("native compile operation was cancelled")]
    Cancelled,
}

#[derive(Clone, Debug)]
pub struct NodeContext {
    pub prompt_id: PromptId,
    pub attempt_id: AttemptId,
    pub node_id: NodeId,
    pub cancellation: CancellationToken,
    pub scratch: ScratchReservation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreparedEffectRequest {
    pub transaction_id: Uuid,
    pub metadata: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreparedEffect {
    pub prompt_id: PromptId,
    pub attempt_id: AttemptId,
    pub node_id: NodeId,
    pub transaction_id: Uuid,
    pub metadata: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CacheDependencies {
    pub artifact_digests: BTreeMap<String, String>,
    pub plugin_digest: Option<String>,
    pub rng_phase: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeOutcome {
    Values {
        outputs: Vec<Value>,
        ui: Option<Value>,
        effects: Vec<PreparedEffectRequest>,
    },
    Blocked {
        reason: String,
    },
    Expansion {
        plan: CompiledPlan,
        output_node: NodeId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeFailureKind {
    Failure,
    Interrupted,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("{code}: {message}")]
pub struct NodeFailure {
    pub code: String,
    pub message: String,
    pub kind: NodeFailureKind,
    pub retryable: bool,
}

pub trait NativeNode: Send + Sync {
    fn class_type(&self) -> &str;
    fn implementation_version(&self) -> &str;

    fn implementation_namespace(&self) -> &str {
        "sim.native_rust"
    }

    fn demanded_lazy_inputs(
        &self,
        _context: &NodeContext,
        _available_inputs: &BTreeMap<String, Value>,
    ) -> Result<BTreeSet<String>, NodeFailure> {
        Ok(BTreeSet::new())
    }

    fn cache_change_token(&self, _inputs: &BTreeMap<String, Value>) -> Result<String, NodeFailure> {
        Ok("stable".to_owned())
    }

    fn cache_dependencies(
        &self,
        _context: &NodeContext,
        _inputs: &BTreeMap<String, Value>,
    ) -> Result<CacheDependencies, NodeFailure> {
        Ok(CacheDependencies::default())
    }

    fn execute<'a>(
        &'a self,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
    ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>>;
}

#[derive(Clone, Default)]
pub struct NativeNodeRegistry {
    nodes: BTreeMap<String, Arc<dyn NativeNode>>,
    descriptors: BTreeMap<String, RuntimeNodeDescriptor>,
    presentations: BTreeMap<String, RuntimeNodePresentation>,
}

impl NativeNodeRegistry {
    pub fn register(&mut self, node: Arc<dyn NativeNode>) -> Result<(), ExecutionError> {
        let class_type = node.class_type().to_owned();
        let implementation_version = node.implementation_version().to_owned();
        if class_type.is_empty() || implementation_version.is_empty() {
            return Err(ExecutionError::InvalidNodeImplementation(class_type));
        }
        if self.nodes.contains_key(&class_type) {
            return Err(ExecutionError::DuplicateNodeImplementation(class_type));
        }
        if let Some(descriptor) = self.descriptors.get(&class_type)
            && descriptor.implementation_version != implementation_version
        {
            return Err(ExecutionError::DescriptorImplementationVersionMismatch {
                class_type,
                expected: descriptor.implementation_version.clone(),
                actual: implementation_version,
            });
        }
        if node.implementation_namespace().trim().is_empty() {
            return Err(ExecutionError::InvalidNodeImplementation(class_type));
        }
        self.nodes.insert(class_type, node);
        Ok(())
    }

    pub fn node(&self, class_type: &str) -> Option<Arc<dyn NativeNode>> {
        self.nodes.get(class_type).cloned()
    }

    pub fn node_len(&self) -> usize {
        self.nodes.len()
    }

    pub fn register_descriptor(
        &mut self,
        descriptor: RuntimeNodeDescriptor,
    ) -> Result<(), PromptCompileError> {
        if descriptor.class_type.is_empty()
            || descriptor.implementation_version.is_empty()
            || descriptor.inputs.iter().any(|(name, _)| name.is_empty())
        {
            return Err(PromptCompileError::InvalidRuntimeDescriptor(
                descriptor.class_type,
            ));
        }
        if self.descriptors.contains_key(&descriptor.class_type) {
            return Err(PromptCompileError::DuplicateRuntimeDescriptor(
                descriptor.class_type,
            ));
        }
        if let Some(node) = self.nodes.get(&descriptor.class_type)
            && node.implementation_version() != descriptor.implementation_version
        {
            return Err(PromptCompileError::InvalidRuntimeDescriptor(
                descriptor.class_type,
            ));
        }
        self.descriptors
            .insert(descriptor.class_type.clone(), descriptor);
        Ok(())
    }

    pub fn descriptor(&self, class_type: &str) -> Option<&RuntimeNodeDescriptor> {
        self.descriptors.get(class_type)
    }

    pub fn descriptors(&self) -> impl Iterator<Item = (&str, &RuntimeNodeDescriptor)> {
        self.descriptors
            .iter()
            .map(|(class_type, descriptor)| (class_type.as_str(), descriptor))
    }

    pub fn descriptor_len(&self) -> usize {
        self.descriptors.len()
    }

    pub fn descriptors_are_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    pub fn descriptors_are_fully_bound(&self) -> bool {
        !self.descriptors.is_empty()
            && self
                .descriptors
                .keys()
                .all(|class_type| self.nodes.contains_key(class_type))
    }

    pub fn descriptor_is_bound(&self, class_type: &str) -> bool {
        self.descriptors.contains_key(class_type) && self.nodes.contains_key(class_type)
    }

    pub fn implementation_namespace(&self, class_type: &str) -> Option<&str> {
        self.nodes
            .get(class_type)
            .map(|node| node.implementation_namespace())
    }

    pub fn presentation(&self, class_type: &str) -> Option<&RuntimeNodePresentation> {
        self.presentations.get(class_type)
    }

    pub fn register_bound_batch(
        &mut self,
        bindings: impl IntoIterator<Item = (RuntimeNodeDescriptor, Arc<dyn NativeNode>)>,
    ) -> Result<(), NativeNodeRegistryError> {
        self.register_bound_batch_internal(
            bindings
                .into_iter()
                .map(|(descriptor, node)| (descriptor, node, None)),
        )
    }

    pub fn register_bound_batch_with_presentations(
        &mut self,
        bindings: impl IntoIterator<
            Item = (
                RuntimeNodeDescriptor,
                Arc<dyn NativeNode>,
                RuntimeNodePresentation,
            ),
        >,
    ) -> Result<(), NativeNodeRegistryError> {
        self.register_bound_batch_internal(
            bindings
                .into_iter()
                .map(|(descriptor, node, presentation)| (descriptor, node, Some(presentation))),
        )
    }

    fn register_bound_batch_internal(
        &mut self,
        bindings: impl IntoIterator<
            Item = (
                RuntimeNodeDescriptor,
                Arc<dyn NativeNode>,
                Option<RuntimeNodePresentation>,
            ),
        >,
    ) -> Result<(), NativeNodeRegistryError> {
        let mut next = self.clone();
        for (descriptor, node, presentation) in bindings {
            let class_type = descriptor.class_type.clone();
            if descriptor.class_type != node.class_type()
                || descriptor.implementation_version != node.implementation_version()
            {
                return Err(NativeNodeRegistryError::BindingMismatch(class_type));
            }
            if let Some(presentation) = presentation.as_ref() {
                if presentation.display_name.is_empty()
                    || presentation.display_name.len() > 256
                    || presentation.category.is_empty()
                    || presentation.category.len() > 512
                    || presentation.output_names.len() != descriptor.outputs.len()
                    || presentation
                        .output_names
                        .iter()
                        .any(|name| name.is_empty() || name.len() > 256)
                {
                    return Err(NativeNodeRegistryError::InvalidPresentation(class_type));
                }
                if next.presentations.contains_key(&class_type) {
                    return Err(NativeNodeRegistryError::DuplicatePresentation(class_type));
                }
            }
            next.register_descriptor(descriptor)?;
            next.register(node)?;
            if let Some(presentation) = presentation {
                next.presentations.insert(class_type, presentation);
            }
        }
        *self = next;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum NativeNodeRegistryError {
    #[error(transparent)]
    Descriptor(#[from] PromptCompileError),
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error("native node binding for `{0}` does not match its implementation")]
    BindingMismatch(String),
    #[error("native node presentation for `{0}` is invalid")]
    InvalidPresentation(String),
    #[error("native node presentation for `{0}` is already registered")]
    DuplicatePresentation(String),
}

pub trait EffectCoordinator: Send + Sync {
    fn prepare(&self, effect: PreparedEffect) -> Result<PreparedEffect, String>;
    fn commit_batch(&self, effects: &[PreparedEffect]) -> Result<(), String>;
    fn rollback_batch(&self, effects: &[PreparedEffect]) -> Result<(), String>;
}

#[cfg(test)]
#[derive(Default)]
struct EffectCoordinatorCalls {
    prepared: Vec<PreparedEffect>,
    committed_batches: Vec<Vec<PreparedEffect>>,
    rolled_back_batches: Vec<Vec<PreparedEffect>>,
}

#[cfg(test)]
#[derive(Default)]
struct RecordingEffectCoordinator {
    calls: Mutex<EffectCoordinatorCalls>,
}

#[cfg(test)]
impl RecordingEffectCoordinator {
    pub fn prepared(&self) -> BTreeSet<Uuid> {
        self.calls
            .lock()
            .prepared
            .iter()
            .map(|effect| effect.transaction_id)
            .collect()
    }

    pub fn committed(&self) -> BTreeSet<Uuid> {
        self.calls
            .lock()
            .committed_batches
            .iter()
            .flatten()
            .map(|effect| effect.transaction_id)
            .collect()
    }

    pub fn rolled_back(&self) -> BTreeSet<Uuid> {
        self.calls
            .lock()
            .rolled_back_batches
            .iter()
            .flatten()
            .map(|effect| effect.transaction_id)
            .collect()
    }
}

#[cfg(test)]
impl EffectCoordinator for RecordingEffectCoordinator {
    fn prepare(&self, effect: PreparedEffect) -> Result<PreparedEffect, String> {
        self.calls.lock().prepared.push(effect.clone());
        Ok(effect)
    }

    fn commit_batch(&self, effects: &[PreparedEffect]) -> Result<(), String> {
        self.calls.lock().committed_batches.push(effects.to_vec());
        Ok(())
    }

    fn rollback_batch(&self, effects: &[PreparedEffect]) -> Result<(), String> {
        self.calls.lock().rolled_back_batches.push(effects.to_vec());
        Ok(())
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ExecutionError {
    #[error("native node implementation `{0}` is invalid")]
    InvalidNodeImplementation(String),
    #[error("native node implementation `{0}` is already registered")]
    DuplicateNodeImplementation(String),
    #[error("compiled node {node:?} has no native implementation for `{class_type}`")]
    MissingNodeImplementation { node: NodeId, class_type: String },
    #[error(
        "compiled node {node:?} expects `{class_type}` version `{expected}`, but the registered implementation is `{actual}`"
    )]
    ImplementationVersionMismatch {
        node: NodeId,
        class_type: String,
        expected: String,
        actual: String,
    },
    #[error(
        "native node `{class_type}` descriptor expects implementation version `{expected}`, but registration supplied `{actual}`"
    )]
    DescriptorImplementationVersionMismatch {
        class_type: String,
        expected: String,
        actual: String,
    },
    #[error("compiled plan references unknown node {0:?}")]
    UnknownNode(NodeId),
    #[error("runtime dependency cycle reached node {0:?}")]
    DependencyCycle(NodeId),
    #[error("node {node:?} demanded invalid lazy input `{input}`")]
    InvalidLazyDemand { node: NodeId, input: String },
    #[error("node {node:?} output {output_index} is missing")]
    MissingOutput { node: NodeId, output_index: usize },
    #[error("node {node:?} returned {actual} outputs, expected {expected}")]
    OutputArity {
        node: NodeId,
        expected: usize,
        actual: usize,
    },
    #[error("node {node:?} output {output_index} violates its declared type or list cardinality")]
    InvalidOutput { node: NodeId, output_index: usize },
    #[error("node {node:?} returned transactional effects while declared {effect:?}")]
    UnexpectedEffect { node: NodeId, effect: EffectClass },
    #[error("node {node:?} was blocked: {reason}")]
    Blocked { node: NodeId, reason: String },
    #[error("node {node:?} failed: {failure}")]
    Node { node: NodeId, failure: NodeFailure },
    #[error("node {node:?} was interrupted: {failure}")]
    Interrupted { node: NodeId, failure: NodeFailure },
    #[error("execution was cancelled")]
    Cancelled,
    #[error("expansion depth exceeds {MAX_EXPANSION_DEPTH}")]
    ExpansionDepth,
    #[error("expanded plan does not contain output node {0:?}")]
    InvalidExpansionOutput(NodeId),
    #[error("cache operation failed: {0}")]
    Cache(String),
    #[error("effect coordination failed: {0}")]
    Effect(String),
    #[error("execution event publication failed: {0}")]
    EventBus(String),
    #[error("execution event sequence is exhausted")]
    SequenceExhausted,
    #[error("execution progress exceeds the supported counter range")]
    ProgressOverflow,
    #[error("execution expansion scope sequence is exhausted")]
    ExpansionSequenceExhausted,
    #[error("node {node:?} returned too many transactional effects")]
    TooManyEffects { node: NodeId },
    #[error("node {node:?} returned oversized effect metadata for transaction {transaction_id}")]
    OversizedEffect { node: NodeId, transaction_id: Uuid },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub profile_id: ProfileId,
    pub prompt_id: PromptId,
    pub attempt_id: AttemptId,
    pub state: AttemptState,
    pub outputs: BTreeMap<NodeId, Vec<Value>>,
    #[serde(default)]
    pub ui_outputs: BTreeMap<NodeId, Value>,
    pub events: Vec<AttemptEvent>,
    pub cache_hits: usize,
    pub error: Option<String>,
}

pub struct ExecutionEngine {
    profile_id: ProfileId,
    nodes: Arc<NativeNodeRegistry>,
    cache: Arc<Mutex<NativeCache>>,
    effects: Arc<dyn EffectCoordinator>,
    event_bus: Option<ExecutionEventBus>,
    registry_version: String,
    backend: String,
    dtype_policy: String,
    configuration_token: String,
    scratch: ScratchReservation,
}

impl ExecutionEngine {
    pub fn new_with_workspace_authorization(
        profile_id: ProfileId,
        nodes: Arc<NativeNodeRegistry>,
        cache: Arc<Mutex<NativeCache>>,
        effects: Arc<dyn EffectCoordinator>,
        registry_version: impl Into<String>,
        scratch: ScratchReservation,
    ) -> Result<Self, ExecutionError> {
        let registry_version = registry_version.into();
        if registry_version.is_empty() {
            return Err(ExecutionError::Cache(
                "registry version is empty".to_owned(),
            ));
        }
        Ok(Self {
            profile_id,
            nodes,
            cache,
            effects,
            event_bus: None,
            registry_version,
            backend: "cpu".to_owned(),
            dtype_policy: "default".to_owned(),
            configuration_token: "default".to_owned(),
            scratch,
        })
    }

    pub fn with_event_bus(mut self, event_bus: ExecutionEventBus) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub fn with_backend(mut self, backend: impl Into<String>) -> Result<Self, ExecutionError> {
        self.backend = nonempty_cache_dimension("backend", backend.into())?;
        Ok(self)
    }

    pub fn with_dtype_policy(
        mut self,
        dtype_policy: impl Into<String>,
    ) -> Result<Self, ExecutionError> {
        self.dtype_policy = nonempty_cache_dimension("dtype policy", dtype_policy.into())?;
        Ok(self)
    }

    pub fn with_configuration_token(
        mut self,
        configuration_token: impl Into<String>,
    ) -> Result<Self, ExecutionError> {
        self.configuration_token =
            nonempty_cache_dimension("configuration token", configuration_token.into())?;
        Ok(self)
    }

    pub fn with_native_compile_policy(
        mut self,
        policy: &NativeCompilePolicy,
    ) -> Result<Self, ExecutionError> {
        self.backend = policy.backend().cache_dimension().to_owned();
        self.configuration_token = policy
            .cache_token()
            .map_err(|error| ExecutionError::Cache(error.to_string()))?;
        Ok(self)
    }

    pub async fn execute(
        &self,
        plan: &CompiledPlan,
        attempt_id: AttemptId,
        cancellation: CancellationToken,
    ) -> ExecutionReport {
        let mut state = RunState::new(self.profile_id, plan.prompt_id, attempt_id, cancellation);
        let result = async {
            state.emit(self.event_bus.as_ref(), None, AttemptEventKind::Started)?;
            self.run_plan(plan, &mut state, 0).await?;
            if state.cancellation.is_cancelled() {
                return Err(ExecutionError::Cancelled);
            }
            self.effects
                .commit_batch(&state.prepared_effects)
                .map_err(ExecutionError::Effect)?;
            if let Err(error) =
                state.emit(self.event_bus.as_ref(), None, AttemptEventKind::Succeeded)
            {
                state.diagnostics.push(error.to_string());
            }
            Ok(())
        }
        .await;

        match result {
            Ok(()) => ExecutionReport {
                profile_id: self.profile_id,
                prompt_id: plan.prompt_id,
                attempt_id,
                state: AttemptState::Succeeded,
                outputs: state.outputs,
                ui_outputs: state.ui_outputs,
                events: state.events,
                cache_hits: state.cache_hits,
                error: (!state.diagnostics.is_empty()).then(|| state.diagnostics.join("; ")),
            },
            Err(error) => {
                let mut error_message = error.to_string();
                if let Err(rollback_error) = self.effects.rollback_batch(&state.prepared_effects) {
                    error_message.push_str("; rollback failed: ");
                    error_message.push_str(&rollback_error);
                }
                let (terminal_state, terminal_event) = match &error {
                    ExecutionError::Cancelled => {
                        (AttemptState::Cancelled, AttemptEventKind::Cancelled)
                    }
                    ExecutionError::Interrupted { .. } => (
                        AttemptState::Interrupted,
                        AttemptEventKind::Interrupted {
                            reason: error.to_string(),
                        },
                    ),
                    _ => (
                        AttemptState::Failed,
                        AttemptEventKind::Failed {
                            failure: crate::ExecutionFailure::new(
                                "native_execution_failed",
                                error.to_string(),
                            ),
                        },
                    ),
                };
                if let Err(event_error) = state.emit(self.event_bus.as_ref(), None, terminal_event)
                {
                    error_message.push_str("; terminal event failed: ");
                    error_message.push_str(&event_error.to_string());
                }
                ExecutionReport {
                    profile_id: self.profile_id,
                    prompt_id: plan.prompt_id,
                    attempt_id,
                    state: terminal_state,
                    outputs: BTreeMap::new(),
                    ui_outputs: BTreeMap::new(),
                    events: state.events,
                    cache_hits: state.cache_hits,
                    error: Some(error_message),
                }
            }
        }
    }

    fn run_plan<'a>(
        &'a self,
        plan: &'a CompiledPlan,
        state: &'a mut RunState,
        expansion_depth: usize,
    ) -> BoxFuture<'a, Result<(), ExecutionError>> {
        Box::pin(async move {
            if expansion_depth > MAX_EXPANSION_DEPTH {
                return Err(ExecutionError::ExpansionDepth);
            }
            for output_node in &plan.output_nodes {
                self.run_node(plan, output_node, state, expansion_depth)
                    .await?;
            }
            Ok(())
        })
    }

    fn run_node<'a>(
        &'a self,
        plan: &'a CompiledPlan,
        node_id: &'a NodeId,
        state: &'a mut RunState,
        expansion_depth: usize,
    ) -> BoxFuture<'a, Result<Vec<Value>, ExecutionError>> {
        Box::pin(async move {
            if state.cancellation.is_cancelled() {
                return Err(ExecutionError::Cancelled);
            }
            if let Some(outputs) = state.outputs.get(node_id) {
                return Ok(outputs.clone());
            }
            if !state.visiting.insert(node_id.clone()) {
                return Err(ExecutionError::DependencyCycle(node_id.clone()));
            }
            let result = self
                .run_node_inner(plan, node_id, state, expansion_depth)
                .await;
            state.visiting.remove(node_id);
            result
        })
    }

    async fn run_node_inner(
        &self,
        plan: &CompiledPlan,
        node_id: &NodeId,
        state: &mut RunState,
        expansion_depth: usize,
    ) -> Result<Vec<Value>, ExecutionError> {
        let node = plan
            .nodes
            .get(node_id)
            .cloned()
            .ok_or_else(|| ExecutionError::UnknownNode(node_id.clone()))?;
        let implementation = self.nodes.node(&node.class_type).ok_or_else(|| {
            ExecutionError::MissingNodeImplementation {
                node: node_id.clone(),
                class_type: node.class_type.clone(),
            }
        })?;
        if implementation.implementation_version() != node.descriptor.implementation_version {
            return Err(ExecutionError::ImplementationVersionMismatch {
                node: node_id.clone(),
                class_type: node.class_type.clone(),
                expected: node.descriptor.implementation_version.clone(),
                actual: implementation.implementation_version().to_owned(),
            });
        }
        let context = NodeContext {
            prompt_id: plan.prompt_id,
            attempt_id: state.attempt_id,
            node_id: node_id.clone(),
            cancellation: state.cancellation.clone(),
            scratch: self.scratch.clone(),
        };
        let mut inputs = BTreeMap::new();
        for (name, binding) in &node.inputs {
            match binding {
                InputBinding::Literal { value } => {
                    inputs.insert(name.clone(), value.clone());
                }
                InputBinding::Link {
                    source,
                    output_index,
                    lazy: false,
                    mode,
                } => {
                    let outputs = self.run_node(plan, source, state, expansion_depth).await?;
                    inputs.insert(
                        name.clone(),
                        linked_value(source, *output_index, *mode, &outputs)?,
                    );
                }
                InputBinding::Link { lazy: true, .. } => {}
            }
        }
        let demanded = implementation
            .demanded_lazy_inputs(&context, &inputs)
            .map_err(|failure| execution_node_failure(node_id.clone(), failure))?;
        for name in demanded {
            let binding =
                node.inputs
                    .get(&name)
                    .ok_or_else(|| ExecutionError::InvalidLazyDemand {
                        node: node_id.clone(),
                        input: name.clone(),
                    })?;
            let InputBinding::Link {
                source,
                output_index,
                lazy: true,
                mode,
            } = binding
            else {
                return Err(ExecutionError::InvalidLazyDemand {
                    node: node_id.clone(),
                    input: name,
                });
            };
            let outputs = self.run_node(plan, source, state, expansion_depth).await?;
            inputs.insert(name, linked_value(source, *output_index, *mode, &outputs)?);
        }

        if context.cancellation.is_cancelled() {
            return Err(ExecutionError::Cancelled);
        }
        let change_token = implementation
            .cache_change_token(&inputs)
            .map_err(|failure| execution_node_failure(node_id.clone(), failure))?;
        let cache_dependencies = implementation
            .cache_dependencies(&context, &inputs)
            .map_err(|failure| execution_node_failure(node_id.clone(), failure))?;
        let demanded_dependencies = demanded_dependency_identities(&node, &inputs, state)?;
        let cache_key = CacheKey::from_inputs_with_dependencies(
            &node.class_type,
            &node.descriptor.implementation_version,
            &inputs,
            demanded_dependencies,
            cache_dependencies.artifact_digests,
            &self.backend,
            &self.dtype_policy,
            cache_dependencies.plugin_digest,
            cache_dependencies.rng_phase,
            &self.configuration_token,
            &self.registry_version,
            change_token,
        )
        .map_err(|error| ExecutionError::Cache(error.to_string()))?;
        let cache_identity = cache_key
            .identity()
            .map_err(|error| ExecutionError::Cache(error.to_string()))?;
        if node.descriptor.cache == RuntimeCachePolicy::InputIdentity
            && let Some(entry) = self.cache.lock().get(&cache_key)
        {
            state.cache_hits = state.cache_hits.saturating_add(1);
            state.emit(
                self.event_bus.as_ref(),
                Some(node_id.clone()),
                AttemptEventKind::CacheHit,
            )?;
            state.outputs.insert(node_id.clone(), entry.outputs.clone());
            if let Some(ui) = entry.ui {
                state.ui_outputs.insert(node_id.clone(), ui);
            }
            state
                .cache_identities
                .insert(node_id.clone(), cache_identity);
            emit_progress(plan, state, self.event_bus.as_ref(), node_id.clone())?;
            return Ok(entry.outputs);
        }

        let (outputs, ui, effects) = self
            .execute_mapped(
                &node,
                implementation.as_ref(),
                context,
                inputs,
                state,
                expansion_depth,
            )
            .await?;
        if outputs.len() != node.descriptor.outputs.len() {
            return Err(ExecutionError::OutputArity {
                node: node_id.clone(),
                expected: node.descriptor.outputs.len(),
                actual: outputs.len(),
            });
        }
        if !effects.is_empty()
            && matches!(
                node.descriptor.effect,
                EffectClass::Pure | EffectClass::ReadsArtifact
            )
        {
            return Err(ExecutionError::UnexpectedEffect {
                node: node_id.clone(),
                effect: node.descriptor.effect,
            });
        }
        if effects.len() > MAX_EFFECTS_PER_NODE {
            return Err(ExecutionError::TooManyEffects {
                node: node_id.clone(),
            });
        }
        for effect in effects {
            if effect.metadata.len() > MAX_EFFECT_METADATA_BYTES {
                return Err(ExecutionError::OversizedEffect {
                    node: node_id.clone(),
                    transaction_id: effect.transaction_id,
                });
            }
            let prepared = self
                .effects
                .prepare(PreparedEffect {
                    prompt_id: plan.prompt_id,
                    attempt_id: state.attempt_id,
                    node_id: node_id.clone(),
                    transaction_id: effect.transaction_id,
                    metadata: effect.metadata,
                })
                .map_err(ExecutionError::Effect)?;
            state.prepared_effects.push(prepared.clone());
            state.emit(
                self.event_bus.as_ref(),
                Some(node_id.clone()),
                AttemptEventKind::OutputPrepared {
                    transaction_id: prepared.transaction_id,
                },
            )?;
        }
        if node.descriptor.cache == RuntimeCachePolicy::InputIdentity
            && state
                .prepared_effects
                .iter()
                .all(|effect| effect.node_id != *node_id)
        {
            self.cache.lock().insert(
                cache_key,
                CacheEntry {
                    outputs: outputs.clone(),
                    ui: ui.clone(),
                },
            );
        }
        state.outputs.insert(node_id.clone(), outputs.clone());
        if let Some(ui) = ui {
            state.ui_outputs.insert(node_id.clone(), ui);
        }
        state
            .cache_identities
            .insert(node_id.clone(), cache_identity);
        emit_progress(plan, state, self.event_bus.as_ref(), node_id.clone())?;
        Ok(outputs)
    }

    async fn execute_mapped(
        &self,
        node: &CompiledNode,
        implementation: &dyn NativeNode,
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
        state: &mut RunState,
        expansion_depth: usize,
    ) -> Result<(Vec<Value>, Option<Value>, Vec<PreparedEffectRequest>), ExecutionError> {
        let mapped = node
            .descriptor
            .inputs
            .iter()
            .filter(|(_, descriptor)| descriptor.mode == InputMode::Mapped)
            .filter_map(|(name, _)| {
                inputs
                    .get(name)
                    .and_then(Value::as_array)
                    .map(|values| (name.clone(), values.len()))
            })
            .collect::<Vec<_>>();
        if mapped.is_empty() {
            return self
                .execute_once(
                    implementation,
                    &node.descriptor.outputs,
                    context,
                    inputs,
                    state,
                    expansion_depth,
                )
                .await;
        }
        if mapped.iter().any(|(_, length)| *length == 0) {
            return Ok((
                node.descriptor
                    .outputs
                    .iter()
                    .map(|_| Value::Array(Vec::new()))
                    .collect(),
                None,
                Vec::new(),
            ));
        }
        let iterations = mapped
            .iter()
            .map(|(_, length)| *length)
            .max()
            .unwrap_or_default();
        let mut collected = vec![Vec::with_capacity(iterations); node.descriptor.outputs.len()];
        let mut combined_ui = Vec::new();
        let mut effects = Vec::new();
        for index in 0..iterations {
            if state.cancellation.is_cancelled() {
                return Err(ExecutionError::Cancelled);
            }
            let mut iteration_inputs = inputs.clone();
            for (name, _) in &mapped {
                let values = inputs.get(name).and_then(Value::as_array).ok_or_else(|| {
                    ExecutionError::InvalidLazyDemand {
                        node: node.id.clone(),
                        input: name.clone(),
                    }
                })?;
                let value_index = index.min(values.len().saturating_sub(1));
                let value = values.get(value_index).cloned().ok_or_else(|| {
                    ExecutionError::InvalidLazyDemand {
                        node: node.id.clone(),
                        input: name.clone(),
                    }
                })?;
                iteration_inputs.insert(name.clone(), value);
            }
            let (outputs, ui, iteration_effects) = self
                .execute_once(
                    implementation,
                    &node.descriptor.outputs,
                    context.clone(),
                    iteration_inputs,
                    state,
                    expansion_depth,
                )
                .await?;
            if outputs.len() != collected.len() {
                return Err(ExecutionError::OutputArity {
                    node: node.id.clone(),
                    expected: collected.len(),
                    actual: outputs.len(),
                });
            }
            for (output_index, ((output, values), descriptor)) in outputs
                .into_iter()
                .zip(&mut collected)
                .zip(&node.descriptor.outputs)
                .enumerate()
            {
                if descriptor.is_list {
                    let output_values =
                        output
                            .as_array()
                            .ok_or_else(|| ExecutionError::InvalidOutput {
                                node: node.id.clone(),
                                output_index,
                            })?;
                    values.extend(output_values.iter().cloned());
                } else {
                    values.push(output);
                }
            }
            if let Some(ui) = ui {
                combined_ui.push(ui);
            }
            effects.extend(iteration_effects);
        }
        Ok((
            collected.into_iter().map(Value::Array).collect(),
            (!combined_ui.is_empty()).then_some(Value::Array(combined_ui)),
            effects,
        ))
    }

    async fn execute_once(
        &self,
        implementation: &dyn NativeNode,
        output_descriptors: &[RuntimeOutputDescriptor],
        context: NodeContext,
        inputs: BTreeMap<String, Value>,
        state: &mut RunState,
        expansion_depth: usize,
    ) -> Result<(Vec<Value>, Option<Value>, Vec<PreparedEffectRequest>), ExecutionError> {
        if state.cancellation.is_cancelled() {
            return Err(ExecutionError::Cancelled);
        }
        let node_id = context.node_id.clone();
        let outcome = implementation.execute(context, inputs).await;
        if state.cancellation.is_cancelled() {
            return Err(ExecutionError::Cancelled);
        }
        let outcome =
            outcome.map_err(|failure| execution_node_failure(node_id.clone(), failure))?;
        let result = match outcome {
            NodeOutcome::Values {
                outputs,
                ui,
                effects,
            } => Ok((outputs, ui, effects)),
            NodeOutcome::Blocked { reason } => Err(ExecutionError::Blocked {
                node: node_id.clone(),
                reason,
            }),
            NodeOutcome::Expansion { plan, output_node } => {
                if expansion_depth >= MAX_EXPANSION_DEPTH {
                    return Err(ExecutionError::ExpansionDepth);
                }
                let (plan, output_node) = namespace_expansion(
                    &plan,
                    &output_node,
                    &node_id,
                    state.prompt_id,
                    expansion_depth,
                    state.next_expansion_scope()?,
                )?;
                self.run_node(&plan, &output_node, state, expansion_depth + 1)
                    .await
                    .map(|outputs| (outputs, None, Vec::new()))
            }
        }?;
        validate_outputs(&node_id, output_descriptors, &result.0)?;
        Ok(result)
    }
}

fn emit_progress(
    plan: &CompiledPlan,
    state: &mut RunState,
    event_bus: Option<&ExecutionEventBus>,
    node_id: NodeId,
) -> Result<(), ExecutionError> {
    let completed = u64::try_from(
        plan.static_required_nodes
            .iter()
            .filter(|node_id| state.outputs.contains_key(*node_id))
            .count(),
    )
    .map_err(|_| ExecutionError::ProgressOverflow)?;
    let total = u64::try_from(plan.static_required_nodes.len())
        .map_err(|_| ExecutionError::ProgressOverflow)?;
    state.emit(
        event_bus,
        Some(node_id),
        AttemptEventKind::Progress { completed, total },
    )
}

fn execution_node_failure(node: NodeId, failure: NodeFailure) -> ExecutionError {
    if failure.kind == NodeFailureKind::Interrupted {
        ExecutionError::Interrupted { node, failure }
    } else {
        ExecutionError::Node { node, failure }
    }
}

fn nonempty_cache_dimension(name: &'static str, value: String) -> Result<String, ExecutionError> {
    if value.is_empty() {
        Err(ExecutionError::Cache(format!("{name} is empty")))
    } else {
        Ok(value)
    }
}

fn demanded_dependency_identities(
    node: &CompiledNode,
    inputs: &BTreeMap<String, Value>,
    state: &RunState,
) -> Result<BTreeMap<String, String>, ExecutionError> {
    node.inputs
        .iter()
        .filter_map(|(name, binding)| {
            if !inputs.contains_key(name) {
                return None;
            }
            let InputBinding::Link {
                source,
                output_index,
                mode,
                ..
            } = binding
            else {
                return None;
            };
            Some((name, source, output_index, mode))
        })
        .map(|(name, source, output_index, mode)| {
            let source_identity = state.cache_identities.get(source).ok_or_else(|| {
                ExecutionError::Cache(format!(
                    "demanded dependency {source:?} has no cache identity"
                ))
            })?;
            let mode = match mode {
                InputMode::Scalar => "scalar",
                InputMode::List => "list",
                InputMode::Mapped => "mapped",
            };
            Ok((
                name.clone(),
                format!("{source_identity}:output={output_index}:mode={mode}"),
            ))
        })
        .collect()
}

fn validate_outputs(
    node_id: &NodeId,
    descriptors: &[RuntimeOutputDescriptor],
    outputs: &[Value],
) -> Result<(), ExecutionError> {
    if outputs.len() != descriptors.len() {
        return Err(ExecutionError::OutputArity {
            node: node_id.clone(),
            expected: descriptors.len(),
            actual: outputs.len(),
        });
    }
    for (output_index, (descriptor, output)) in descriptors.iter().zip(outputs).enumerate() {
        let valid = if descriptor.is_list {
            output.as_array().is_some_and(|values| {
                values
                    .iter()
                    .all(|value| descriptor.value_type.accepts_runtime_output(value))
            })
        } else {
            descriptor.value_type.accepts_runtime_output(output)
        };
        if !valid {
            return Err(ExecutionError::InvalidOutput {
                node: node_id.clone(),
                output_index,
            });
        }
    }
    Ok(())
}

fn linked_value(
    source: &NodeId,
    output_index: usize,
    mode: InputMode,
    outputs: &[Value],
) -> Result<Value, ExecutionError> {
    let value =
        outputs
            .get(output_index)
            .cloned()
            .ok_or_else(|| ExecutionError::MissingOutput {
                node: source.clone(),
                output_index,
            })?;
    if mode == InputMode::List && !value.is_array() {
        Ok(Value::Array(vec![value]))
    } else {
        Ok(value)
    }
}

fn namespace_expansion(
    plan: &CompiledPlan,
    output_node: &NodeId,
    owner: &NodeId,
    prompt_id: PromptId,
    depth: usize,
    scope: u64,
) -> Result<(CompiledPlan, NodeId), ExecutionError> {
    if !plan.nodes.contains_key(output_node) {
        return Err(ExecutionError::InvalidExpansionOutput(output_node.clone()));
    }
    let prefix = format!("{}::expansion-{depth}-{scope}::", owner.0);
    let translate = |identifier: &NodeId| NodeId(format!("{prefix}{}", identifier.0));
    let mut nodes = BTreeMap::new();
    for node in plan.nodes.values() {
        let mut node = node.clone();
        node.id = translate(&node.id);
        for binding in node.inputs.values_mut() {
            if let InputBinding::Link { source, .. } = binding {
                *source = translate(source);
            }
        }
        nodes.insert(node.id.clone(), node);
    }
    Ok((
        CompiledPlan {
            prompt_id,
            client_id: plan.client_id.clone(),
            prompt_number: plan.prompt_number,
            extra_data: plan.extra_data.clone(),
            unknown: plan.unknown.clone(),
            nodes,
            topological_order: plan.topological_order.iter().map(translate).collect(),
            static_required_nodes: plan.static_required_nodes.iter().map(translate).collect(),
            output_nodes: plan.output_nodes.iter().map(translate).collect(),
            persistence_unknown_fields: plan.persistence_unknown_fields.clone(),
        },
        translate(output_node),
    ))
}

struct RunState {
    profile_id: ProfileId,
    prompt_id: PromptId,
    attempt_id: AttemptId,
    cancellation: CancellationToken,
    outputs: BTreeMap<NodeId, Vec<Value>>,
    ui_outputs: BTreeMap<NodeId, Value>,
    cache_identities: BTreeMap<NodeId, String>,
    visiting: BTreeSet<NodeId>,
    prepared_effects: Vec<PreparedEffect>,
    events: Vec<AttemptEvent>,
    next_sequence: u64,
    next_expansion_scope: u64,
    cache_hits: usize,
    diagnostics: Vec<String>,
}

impl RunState {
    fn new(
        profile_id: ProfileId,
        prompt_id: PromptId,
        attempt_id: AttemptId,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            profile_id,
            prompt_id,
            attempt_id,
            cancellation,
            outputs: BTreeMap::new(),
            ui_outputs: BTreeMap::new(),
            cache_identities: BTreeMap::new(),
            visiting: BTreeSet::new(),
            prepared_effects: Vec::new(),
            events: Vec::new(),
            next_sequence: 0,
            next_expansion_scope: 0,
            cache_hits: 0,
            diagnostics: Vec::new(),
        }
    }

    fn next_expansion_scope(&mut self) -> Result<u64, ExecutionError> {
        let scope = self.next_expansion_scope;
        self.next_expansion_scope = self
            .next_expansion_scope
            .checked_add(1)
            .ok_or(ExecutionError::ExpansionSequenceExhausted)?;
        Ok(scope)
    }

    fn emit(
        &mut self,
        event_bus: Option<&ExecutionEventBus>,
        node_id: Option<NodeId>,
        kind: AttemptEventKind,
    ) -> Result<(), ExecutionError> {
        let event = AttemptEvent {
            profile_id: self.profile_id,
            prompt_id: self.prompt_id,
            attempt_id: self.attempt_id,
            sequence: self.next_sequence,
            node_id,
            at: Utc::now(),
            kind,
            data: None,
        };
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(ExecutionError::SequenceExhausted)?;
        self.events.push(event.clone());
        if let Some(event_bus) = event_bus {
            event_bus
                .publish(event)
                .map_err(|error| ExecutionError::EventBus(error.to_string()))?;
        }
        Ok(())
    }
}

impl From<EventBusError> for ExecutionError {
    fn from(error: EventBusError) -> Self {
        Self::EventBus(error.to_string())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        InputMode, PromptCompiler, RuntimeAvailability, RuntimeInputDescriptor,
        RuntimeNodeDescriptor, RuntimeOutputDescriptor, ValueType,
    };
    use comfy_types::{ApiPrompt, PromptNode, PromptSubmission};
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const TEST_WORKSPACE_BYTES: u64 = 64 * 1024 * 1024;

    struct FixtureNode {
        class_type: String,
        calls: Arc<AtomicUsize>,
    }

    struct WorkspaceRecordingNode {
        observed: Arc<Mutex<Vec<u64>>>,
    }

    impl NativeNode for WorkspaceRecordingNode {
        fn class_type(&self) -> &str {
            "WorkspaceRecording"
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn demanded_lazy_inputs(
            &self,
            context: &NodeContext,
            _available: &BTreeMap<String, Value>,
        ) -> Result<BTreeSet<String>, NodeFailure> {
            self.observed.lock().push(context.scratch.bytes());
            Ok(BTreeSet::new())
        }

        fn execute<'a>(
            &'a self,
            context: NodeContext,
            _inputs: BTreeMap<String, Value>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            self.observed.lock().push(context.scratch.bytes());
            Box::pin(async {
                Ok(NodeOutcome::Values {
                    outputs: vec![json!(1)],
                    ui: None,
                    effects: Vec::new(),
                })
            })
        }
    }

    impl NativeNode for FixtureNode {
        fn class_type(&self) -> &str {
            &self.class_type
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn demanded_lazy_inputs(
            &self,
            _context: &NodeContext,
            available: &BTreeMap<String, Value>,
        ) -> Result<BTreeSet<String>, NodeFailure> {
            if self.class_type == "Choose" && available.get("condition") == Some(&json!(true)) {
                Ok(BTreeSet::from(["value".to_owned()]))
            } else {
                Ok(BTreeSet::new())
            }
        }

        fn execute<'a>(
            &'a self,
            context: NodeContext,
            inputs: BTreeMap<String, Value>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                smol::future::yield_now().await;
                if context.cancellation.is_cancelled() {
                    return Err(NodeFailure {
                        code: "cancelled".to_owned(),
                        message: "fixture cancelled".to_owned(),
                        kind: NodeFailureKind::Interrupted,
                        retryable: true,
                    });
                }
                let outputs = match self.class_type.as_str() {
                    "Source" => vec![json!([1, 2, 3])],
                    "LazySource" | "InnerOutput" => vec![json!(42)],
                    "Double" => vec![json!(
                        inputs
                            .get("value")
                            .and_then(Value::as_i64)
                            .unwrap_or_default()
                            * 2
                    )],
                    "Pair" => vec![json!(
                        inputs
                            .get("left")
                            .and_then(Value::as_i64)
                            .unwrap_or_default()
                            * 10
                            + inputs
                                .get("right")
                                .and_then(Value::as_i64)
                                .unwrap_or_default()
                    )],
                    "ListMap" => vec![json!([inputs
                        .get("value")
                        .and_then(Value::as_i64)
                        .unwrap_or_default()])],
                    "Choose" => vec![inputs.get("value").cloned().unwrap_or_else(|| json!(0))],
                    "Output" => vec![inputs.get("value").cloned().unwrap_or(Value::Null)],
                    "Write" => {
                        return Ok(NodeOutcome::Values {
                            outputs: vec![json!("prepared")],
                            ui: None,
                            effects: vec![PreparedEffectRequest {
                                transaction_id: Uuid::from_u128(99),
                                metadata: b"output".to_vec(),
                            }],
                        });
                    }
                    "Block" => {
                        return Ok(NodeOutcome::Blocked {
                            reason: "fixture blocker".to_owned(),
                        });
                    }
                    _ => vec![Value::Null],
                };
                Ok(NodeOutcome::Values {
                    outputs,
                    ui: None,
                    effects: Vec::new(),
                })
            })
        }
    }

    struct ExpansionNode {
        plan: CompiledPlan,
    }

    impl NativeNode for ExpansionNode {
        fn class_type(&self) -> &str {
            "Expand"
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn execute<'a>(
            &'a self,
            _context: NodeContext,
            _inputs: BTreeMap<String, Value>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            let plan = self.plan.clone();
            Box::pin(async move {
                Ok(NodeOutcome::Expansion {
                    plan,
                    output_node: NodeId::from("inner"),
                })
            })
        }
    }

    struct CancellingNode;

    impl NativeNode for CancellingNode {
        fn class_type(&self) -> &str {
            "CancelDuringExecution"
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn execute<'a>(
            &'a self,
            context: NodeContext,
            _inputs: BTreeMap<String, Value>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            Box::pin(async move {
                context.cancellation.cancel();
                Err(NodeFailure {
                    code: "interrupted".to_owned(),
                    message: "node observed cancellation".to_owned(),
                    kind: NodeFailureKind::Interrupted,
                    retryable: true,
                })
            })
        }
    }

    struct CancelBeforeCacheNode {
        phases: Arc<Mutex<Vec<&'static str>>>,
    }

    impl NativeNode for CancelBeforeCacheNode {
        fn class_type(&self) -> &str {
            "CancelBeforeCache"
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn demanded_lazy_inputs(
            &self,
            context: &NodeContext,
            _available_inputs: &BTreeMap<String, Value>,
        ) -> Result<BTreeSet<String>, NodeFailure> {
            self.phases.lock().push("demand");
            context.cancellation.cancel();
            Ok(BTreeSet::new())
        }

        fn cache_change_token(
            &self,
            _inputs: &BTreeMap<String, Value>,
        ) -> Result<String, NodeFailure> {
            self.phases.lock().push("change");
            Ok("stable".to_owned())
        }

        fn cache_dependencies(
            &self,
            _context: &NodeContext,
            _inputs: &BTreeMap<String, Value>,
        ) -> Result<CacheDependencies, NodeFailure> {
            self.phases.lock().push("dependencies");
            Ok(CacheDependencies::default())
        }

        fn execute<'a>(
            &'a self,
            _context: NodeContext,
            _inputs: BTreeMap<String, Value>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            self.phases.lock().push("execute");
            Box::pin(async move {
                Ok(NodeOutcome::Values {
                    outputs: vec![json!(1)],
                    ui: Some(json!({"preview": "must-not-publish"})),
                    effects: vec![PreparedEffectRequest {
                        transaction_id: Uuid::from_u128(100),
                        metadata: b"must-not-publish".to_vec(),
                    }],
                })
            })
        }
    }

    struct UiNode {
        calls: Arc<AtomicUsize>,
    }

    impl NativeNode for UiNode {
        fn class_type(&self) -> &str {
            "Ui"
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn execute<'a>(
            &'a self,
            _context: NodeContext,
            _inputs: BTreeMap<String, Value>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Ok(NodeOutcome::Values {
                    outputs: vec![json!(1)],
                    ui: Some(json!({"preview": "ready"})),
                    effects: Vec::new(),
                })
            })
        }
    }

    struct VersionedSourceNode {
        version: &'static str,
    }

    impl NativeNode for VersionedSourceNode {
        fn class_type(&self) -> &str {
            "VersionedSource"
        }

        fn implementation_version(&self) -> &str {
            self.version
        }

        fn execute<'a>(
            &'a self,
            _context: NodeContext,
            _inputs: BTreeMap<String, Value>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            Box::pin(async {
                Ok(NodeOutcome::Values {
                    outputs: vec![json!(7)],
                    ui: None,
                    effects: Vec::new(),
                })
            })
        }
    }

    struct PassthroughNode {
        calls: Arc<AtomicUsize>,
    }

    impl NativeNode for PassthroughNode {
        fn class_type(&self) -> &str {
            "Passthrough"
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn execute<'a>(
            &'a self,
            _context: NodeContext,
            inputs: BTreeMap<String, Value>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                Ok(NodeOutcome::Values {
                    outputs: vec![inputs.get("value").cloned().unwrap_or(Value::Null)],
                    ui: None,
                    effects: Vec::new(),
                })
            })
        }
    }

    fn runtime_descriptor(
        class_type: &str,
        output_node: bool,
        inputs: BTreeMap<String, RuntimeInputDescriptor>,
        effect: EffectClass,
    ) -> RuntimeNodeDescriptor {
        RuntimeNodeDescriptor {
            class_type: class_type.to_owned(),
            implementation_version: "1".to_owned(),
            inputs,
            outputs: vec![RuntimeOutputDescriptor {
                value_type: if class_type == "Write" {
                    ValueType::Any
                } else {
                    ValueType::Number
                },
                is_list: class_type == "Source" || class_type == "Output",
            }],
            output_node,
            availability: RuntimeAvailability::Native,
            effect,
            cache: if effect == EffectClass::Pure {
                RuntimeCachePolicy::InputIdentity
            } else {
                RuntimeCachePolicy::Never
            },
        }
    }

    fn input(
        value_type: ValueType,
        lazy: bool,
        mode: InputMode,
        literal: bool,
    ) -> RuntimeInputDescriptor {
        RuntimeInputDescriptor {
            value_type,
            required: true,
            hidden: false,
            lazy,
            mode,
            allows_literal: literal,
        }
    }

    #[test]
    fn component_presentation_registration_is_checked_and_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        let calls = Arc::new(AtomicUsize::new(0));
        let descriptor = runtime_descriptor("Component", false, BTreeMap::new(), EffectClass::Pure);
        let node: Arc<dyn NativeNode> = Arc::new(FixtureNode {
            class_type: "Component".to_owned(),
            calls,
        });
        let mut registry = NativeNodeRegistry::default();
        let error = registry
            .register_bound_batch_with_presentations([(
                descriptor.clone(),
                node.clone(),
                RuntimeNodePresentation {
                    display_name: "Signed Component".to_owned(),
                    category: "signed/category".to_owned(),
                    output_names: Vec::new(),
                },
            )])
            .expect_err("output-name arity must match the execution descriptor");
        assert!(matches!(
            error,
            NativeNodeRegistryError::InvalidPresentation(class_type)
                if class_type == "Component"
        ));
        assert_eq!(registry.node_len(), 0);
        assert_eq!(registry.descriptor_len(), 0);
        assert!(registry.presentation("Component").is_none());

        let presentation = RuntimeNodePresentation {
            display_name: "Signed Component".to_owned(),
            category: "signed/category".to_owned(),
            output_names: vec!["Signed output".to_owned()],
        };
        registry.register_bound_batch_with_presentations([(
            descriptor,
            node,
            presentation.clone(),
        )])?;
        assert!(registry.descriptor_is_bound("Component"));
        assert_eq!(registry.presentation("Component"), Some(&presentation));
        Ok(())
    }

    fn compile_plan(
        descriptors: Vec<RuntimeNodeDescriptor>,
        nodes: BTreeMap<NodeId, PromptNode>,
    ) -> Result<CompiledPlan, Box<dyn std::error::Error>> {
        let mut registry = NativeNodeRegistry::default();
        for descriptor in descriptors {
            registry.register_descriptor(descriptor)?;
        }
        Ok(PromptCompiler::new(&registry).compile(PromptSubmission {
            prompt: ApiPrompt(nodes),
            prompt_id: Some(PromptId(Uuid::from_u128(1))),
            client_id: None,
            number: None,
            extra_data: BTreeMap::new(),
            unknown: BTreeMap::new(),
        })?)
    }

    #[test]
    fn native_executor_forwards_planned_workspace_without_reauthorizing()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let descriptor = runtime_descriptor(
                "WorkspaceRecording",
                true,
                BTreeMap::new(),
                EffectClass::Pure,
            );
            let plan = compile_plan(
                vec![descriptor.clone()],
                BTreeMap::from([(
                    NodeId::from("workspace"),
                    PromptNode {
                        class_type: "WorkspaceRecording".to_owned(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let observed = Arc::new(Mutex::new(Vec::new()));
            let mut registry = NativeNodeRegistry::default();
            registry.register_descriptor(descriptor)?;
            registry.register(Arc::new(WorkspaceRecordingNode {
                observed: observed.clone(),
            }))?;
            let (_backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1024)?;
            let workspace = workspace_authority.authorize_workspace(321)?;
            let engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(registry),
                Arc::new(Mutex::new(NativeCache::new(4)?)),
                Arc::new(RecordingEffectCoordinator::default()),
                "workspace-registry-v1",
                workspace,
            )?;
            let report = engine
                .execute(
                    &plan,
                    AttemptId(Uuid::from_u128(321)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(report.state, AttemptState::Succeeded);
            assert_eq!(&*observed.lock(), &[321, 321]);
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn val_domain_004_graph_list_lazy_async_cache_and_effects_execute_transactionally()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let (_workspace_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
            let descriptors = vec![
                runtime_descriptor("Source", false, BTreeMap::new(), EffectClass::Pure),
                runtime_descriptor(
                    "Double",
                    false,
                    BTreeMap::from([(
                        "value".to_owned(),
                        input(ValueType::Number, false, InputMode::Mapped, false),
                    )]),
                    EffectClass::Pure,
                ),
                runtime_descriptor(
                    "Output",
                    true,
                    BTreeMap::from([(
                        "value".to_owned(),
                        input(ValueType::Number, false, InputMode::List, false),
                    )]),
                    EffectClass::Pure,
                ),
            ];
            let plan = compile_plan(
                descriptors,
                BTreeMap::from([
                    (
                        NodeId::from("source"),
                        PromptNode {
                            class_type: "Source".to_owned(),
                            inputs: BTreeMap::new(),
                            unknown: BTreeMap::new(),
                        },
                    ),
                    (
                        NodeId::from("double"),
                        PromptNode {
                            class_type: "Double".to_owned(),
                            inputs: BTreeMap::from([("value".to_owned(), json!(["source", 0]))]),
                            unknown: BTreeMap::new(),
                        },
                    ),
                    (
                        NodeId::from("output"),
                        PromptNode {
                            class_type: "Output".to_owned(),
                            inputs: BTreeMap::from([("value".to_owned(), json!(["double", 0]))]),
                            unknown: BTreeMap::new(),
                        },
                    ),
                ]),
            )?;
            let calls = Arc::new(AtomicUsize::new(0));
            let mut nodes = NativeNodeRegistry::default();
            for class_type in ["Source", "Double", "Output"] {
                nodes.register(Arc::new(FixtureNode {
                    class_type: class_type.to_owned(),
                    calls: calls.clone(),
                }))?;
            }
            let effects = Arc::new(RecordingEffectCoordinator::default());
            let engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(nodes),
                Arc::new(Mutex::new(NativeCache::new(32)?)),
                effects,
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?;
            let first = engine
                .execute(
                    &plan,
                    AttemptId(Uuid::from_u128(2)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(first.state, AttemptState::Succeeded);
            assert_eq!(first.outputs[&NodeId::from("output")], [json!([2, 4, 6])]);
            let first_calls = calls.load(Ordering::SeqCst);
            let second = engine
                .execute(
                    &plan,
                    AttemptId(Uuid::from_u128(3)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(second.state, AttemptState::Succeeded);
            assert!(second.cache_hits > 0);
            assert_eq!(calls.load(Ordering::SeqCst), first_calls);
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn val_domain_004_repeat_last_and_output_list_flattening_are_exact()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let (_workspace_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
            let pair = runtime_descriptor(
                "Pair",
                true,
                BTreeMap::from([
                    (
                        "left".to_owned(),
                        input(ValueType::Integer, false, InputMode::Mapped, true),
                    ),
                    (
                        "right".to_owned(),
                        input(ValueType::Integer, false, InputMode::Mapped, true),
                    ),
                ]),
                EffectClass::Pure,
            );
            let mut list_map = runtime_descriptor(
                "ListMap",
                true,
                BTreeMap::from([(
                    "value".to_owned(),
                    input(ValueType::Integer, false, InputMode::Mapped, true),
                )]),
                EffectClass::Pure,
            );
            list_map.outputs[0].is_list = true;
            let plan = compile_plan(
                vec![pair, list_map],
                BTreeMap::from([
                    (
                        NodeId::from("pair"),
                        PromptNode {
                            class_type: "Pair".to_owned(),
                            inputs: BTreeMap::from([
                                ("left".to_owned(), json!([1, 2, 3])),
                                ("right".to_owned(), json!([4, 5])),
                            ]),
                            unknown: BTreeMap::new(),
                        },
                    ),
                    (
                        NodeId::from("list"),
                        PromptNode {
                            class_type: "ListMap".to_owned(),
                            inputs: BTreeMap::from([("value".to_owned(), json!([1, 2]))]),
                            unknown: BTreeMap::new(),
                        },
                    ),
                ]),
            )?;
            let calls = Arc::new(AtomicUsize::new(0));
            let mut registry = NativeNodeRegistry::default();
            for class_type in ["Pair", "ListMap"] {
                registry.register(Arc::new(FixtureNode {
                    class_type: class_type.to_owned(),
                    calls: calls.clone(),
                }))?;
            }
            let engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(registry),
                Arc::new(Mutex::new(NativeCache::new(8)?)),
                Arc::new(RecordingEffectCoordinator::default()),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?;
            let report = engine
                .execute(
                    &plan,
                    AttemptId(Uuid::from_u128(20)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(report.state, AttemptState::Succeeded);
            assert_eq!(report.outputs[&NodeId::from("pair")], [json!([14, 25, 35])]);
            assert_eq!(report.outputs[&NodeId::from("list")], [json!([1, 2])]);
            assert_eq!(calls.load(Ordering::SeqCst), 5);
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn val_domain_004_canonical_cancellation_and_ui_cache_semantics()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let (_workspace_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
            let cancel_plan = compile_plan(
                vec![runtime_descriptor(
                    "CancelDuringExecution",
                    true,
                    BTreeMap::new(),
                    EffectClass::Pure,
                )],
                BTreeMap::from([(
                    NodeId::from("cancel"),
                    PromptNode {
                        class_type: "CancelDuringExecution".to_owned(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let mut cancel_registry = NativeNodeRegistry::default();
            cancel_registry.register(Arc::new(CancellingNode))?;
            let cancel_engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(cancel_registry),
                Arc::new(Mutex::new(NativeCache::new(4)?)),
                Arc::new(RecordingEffectCoordinator::default()),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?;
            let cancelled = cancel_engine
                .execute(
                    &cancel_plan,
                    AttemptId(Uuid::from_u128(21)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(cancelled.state, AttemptState::Cancelled);
            assert!(cancelled.outputs.is_empty());
            assert!(cancelled.ui_outputs.is_empty());

            let before_cache_plan = compile_plan(
                vec![runtime_descriptor(
                    "CancelBeforeCache",
                    true,
                    BTreeMap::new(),
                    EffectClass::WritesArtifact,
                )],
                BTreeMap::from([(
                    NodeId::from("before-cache"),
                    PromptNode {
                        class_type: "CancelBeforeCache".to_owned(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let phases = Arc::new(Mutex::new(Vec::new()));
            let mut before_cache_registry = NativeNodeRegistry::default();
            before_cache_registry.register(Arc::new(CancelBeforeCacheNode {
                phases: phases.clone(),
            }))?;
            let before_cache = Arc::new(Mutex::new(NativeCache::new(4)?));
            let before_cache_effects = Arc::new(RecordingEffectCoordinator::default());
            let before_cache_engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(before_cache_registry),
                before_cache.clone(),
                before_cache_effects.clone(),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?;

            let pre_cancelled = CancellationToken::default();
            pre_cancelled.cancel();
            let report = before_cache_engine
                .execute(
                    &before_cache_plan,
                    AttemptId(Uuid::from_u128(0x2101)),
                    pre_cancelled,
                )
                .await;
            assert_eq!(report.state, AttemptState::Cancelled);
            assert!(phases.lock().is_empty());
            assert_eq!(report.cache_hits, 0);
            assert!(before_cache.lock().is_empty());
            assert!(before_cache_effects.prepared().is_empty());
            assert!(before_cache_effects.committed().is_empty());
            assert!(before_cache_effects.rolled_back().is_empty());
            assert!(report.outputs.is_empty());
            assert!(report.ui_outputs.is_empty());
            assert_eq!(report.events.len(), 2);
            assert!(matches!(report.events[0].kind, AttemptEventKind::Started));
            assert!(matches!(report.events[1].kind, AttemptEventKind::Cancelled));

            let report = before_cache_engine
                .execute(
                    &before_cache_plan,
                    AttemptId(Uuid::from_u128(0x2102)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(report.state, AttemptState::Cancelled);
            assert_eq!(phases.lock().as_slice(), &["demand"]);
            assert_eq!(report.cache_hits, 0);
            assert!(before_cache.lock().is_empty());
            assert!(before_cache_effects.prepared().is_empty());
            assert!(before_cache_effects.committed().is_empty());
            assert!(before_cache_effects.rolled_back().is_empty());
            assert!(report.outputs.is_empty());
            assert!(report.ui_outputs.is_empty());
            assert_eq!(report.events.len(), 2);
            assert!(matches!(report.events[0].kind, AttemptEventKind::Started));
            assert!(matches!(report.events[1].kind, AttemptEventKind::Cancelled));

            let ui_plan = compile_plan(
                vec![runtime_descriptor(
                    "Ui",
                    true,
                    BTreeMap::new(),
                    EffectClass::Pure,
                )],
                BTreeMap::from([(
                    NodeId::from("ui"),
                    PromptNode {
                        class_type: "Ui".to_owned(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let ui_calls = Arc::new(AtomicUsize::new(0));
            let mut ui_registry = NativeNodeRegistry::default();
            ui_registry.register(Arc::new(UiNode {
                calls: ui_calls.clone(),
            }))?;
            let ui_engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(ui_registry),
                Arc::new(Mutex::new(NativeCache::new(4)?)),
                Arc::new(RecordingEffectCoordinator::default()),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?;
            for attempt in [22_u128, 23] {
                let report = ui_engine
                    .execute(
                        &ui_plan,
                        AttemptId(Uuid::from_u128(attempt)),
                        CancellationToken::default(),
                    )
                    .await;
                assert_eq!(report.state, AttemptState::Succeeded);
                assert_eq!(
                    report.ui_outputs[&NodeId::from("ui")],
                    json!({"preview": "ready"})
                );
            }
            assert_eq!(ui_calls.load(Ordering::SeqCst), 1);
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn val_domain_004_cache_tracks_demanded_dependency_identity()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let (_workspace_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
            let compile = |source_version: &str| {
                let mut source = runtime_descriptor(
                    "VersionedSource",
                    false,
                    BTreeMap::new(),
                    EffectClass::Pure,
                );
                source.implementation_version = source_version.to_owned();
                compile_plan(
                    vec![
                        source,
                        runtime_descriptor(
                            "Passthrough",
                            true,
                            BTreeMap::from([(
                                "value".to_owned(),
                                input(ValueType::Number, false, InputMode::Scalar, false),
                            )]),
                            EffectClass::Pure,
                        ),
                    ],
                    BTreeMap::from([
                        (
                            NodeId::from("source"),
                            PromptNode {
                                class_type: "VersionedSource".to_owned(),
                                inputs: BTreeMap::new(),
                                unknown: BTreeMap::new(),
                            },
                        ),
                        (
                            NodeId::from("output"),
                            PromptNode {
                                class_type: "Passthrough".to_owned(),
                                inputs: BTreeMap::from([(
                                    "value".to_owned(),
                                    json!(["source", 0]),
                                )]),
                                unknown: BTreeMap::new(),
                            },
                        ),
                    ]),
                )
            };
            let first_plan = compile("1")?;
            let second_plan = compile("2")?;
            let passthrough_calls = Arc::new(AtomicUsize::new(0));
            let cache = Arc::new(Mutex::new(NativeCache::new(16)?));
            for (attempt, version, plan) in
                [(30_u128, "1", &first_plan), (31_u128, "2", &second_plan)]
            {
                let mut registry = NativeNodeRegistry::default();
                registry.register(Arc::new(VersionedSourceNode { version }))?;
                registry.register(Arc::new(PassthroughNode {
                    calls: passthrough_calls.clone(),
                }))?;
                let engine = ExecutionEngine::new_with_workspace_authorization(
                    ProfileId(Uuid::nil()),
                    Arc::new(registry),
                    cache.clone(),
                    Arc::new(RecordingEffectCoordinator::default()),
                    "registry-v1",
                    workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
                )?;
                let report = engine
                    .execute(
                        plan,
                        AttemptId(Uuid::from_u128(attempt)),
                        CancellationToken::default(),
                    )
                    .await;
                assert_eq!(report.state, AttemptState::Succeeded);
                assert_eq!(report.outputs[&NodeId::from("output")], [json!(7)]);
            }
            assert_eq!(passthrough_calls.load(Ordering::SeqCst), 2);
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn val_domain_004_cancellation_and_blockers_publish_no_partial_outputs()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let (_workspace_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
            let plan = compile_plan(
                vec![runtime_descriptor(
                    "Block",
                    true,
                    BTreeMap::new(),
                    EffectClass::Pure,
                )],
                BTreeMap::from([(
                    NodeId::from("block"),
                    PromptNode {
                        class_type: "Block".to_owned(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let mut nodes = NativeNodeRegistry::default();
            nodes.register(Arc::new(FixtureNode {
                class_type: "Block".to_owned(),
                calls: Arc::new(AtomicUsize::new(0)),
            }))?;
            let engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(nodes),
                Arc::new(Mutex::new(NativeCache::new(4)?)),
                Arc::new(RecordingEffectCoordinator::default()),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?;
            let blocked = engine
                .execute(
                    &plan,
                    AttemptId(Uuid::from_u128(2)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(blocked.state, AttemptState::Failed);
            assert!(blocked.outputs.is_empty());
            let cancellation = CancellationToken::default();
            cancellation.cancel();
            let cancelled = engine
                .execute(&plan, AttemptId(Uuid::from_u128(3)), cancellation)
                .await;
            assert_eq!(cancelled.state, AttemptState::Cancelled);
            assert!(cancelled.outputs.is_empty());
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn val_domain_004_lazy_dependencies_execute_only_when_demanded()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let (_workspace_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
            let descriptors = vec![
                runtime_descriptor("LazySource", false, BTreeMap::new(), EffectClass::Pure),
                runtime_descriptor(
                    "Choose",
                    true,
                    BTreeMap::from([
                        (
                            "condition".to_owned(),
                            input(ValueType::Boolean, false, InputMode::Scalar, true),
                        ),
                        (
                            "value".to_owned(),
                            input(ValueType::Number, true, InputMode::Scalar, false),
                        ),
                    ]),
                    EffectClass::Pure,
                ),
            ];
            let nodes = |condition| {
                BTreeMap::from([
                    (
                        NodeId::from("source"),
                        PromptNode {
                            class_type: "LazySource".to_owned(),
                            inputs: BTreeMap::new(),
                            unknown: BTreeMap::new(),
                        },
                    ),
                    (
                        NodeId::from("choose"),
                        PromptNode {
                            class_type: "Choose".to_owned(),
                            inputs: BTreeMap::from([
                                ("condition".to_owned(), json!(condition)),
                                ("value".to_owned(), json!(["source", 0])),
                            ]),
                            unknown: BTreeMap::new(),
                        },
                    ),
                ])
            };
            let false_plan = compile_plan(descriptors.clone(), nodes(false))?;
            let true_plan = compile_plan(descriptors, nodes(true))?;
            let source_calls = Arc::new(AtomicUsize::new(0));
            let mut registry = NativeNodeRegistry::default();
            registry.register(Arc::new(FixtureNode {
                class_type: "LazySource".to_owned(),
                calls: source_calls.clone(),
            }))?;
            registry.register(Arc::new(FixtureNode {
                class_type: "Choose".to_owned(),
                calls: Arc::new(AtomicUsize::new(0)),
            }))?;
            let engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(registry),
                Arc::new(Mutex::new(NativeCache::new(8)?)),
                Arc::new(RecordingEffectCoordinator::default()),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?;
            let skipped = engine
                .execute(
                    &false_plan,
                    AttemptId(Uuid::from_u128(2)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(skipped.state, AttemptState::Succeeded);
            assert_eq!(source_calls.load(Ordering::SeqCst), 0);
            let demanded = engine
                .execute(
                    &true_plan,
                    AttemptId(Uuid::from_u128(3)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(demanded.state, AttemptState::Succeeded);
            assert_eq!(demanded.outputs[&NodeId::from("choose")], [json!(42)]);
            assert_eq!(source_calls.load(Ordering::SeqCst), 1);
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn val_domain_004_expansion_and_transactional_effect_batches_are_real_execution_paths()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let (_workspace_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
            let inner = compile_plan(
                vec![runtime_descriptor(
                    "InnerOutput",
                    true,
                    BTreeMap::new(),
                    EffectClass::Pure,
                )],
                BTreeMap::from([(
                    NodeId::from("inner"),
                    PromptNode {
                        class_type: "InnerOutput".to_owned(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let outer = compile_plan(
                vec![runtime_descriptor(
                    "Expand",
                    true,
                    BTreeMap::from([(
                        "value".to_owned(),
                        input(ValueType::Integer, false, InputMode::Mapped, true),
                    )]),
                    EffectClass::Pure,
                )],
                BTreeMap::from([(
                    NodeId::from("expand"),
                    PromptNode {
                        class_type: "Expand".to_owned(),
                        inputs: BTreeMap::from([("value".to_owned(), json!([1, 2]))]),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let mut expansion_registry = NativeNodeRegistry::default();
            expansion_registry.register(Arc::new(ExpansionNode { plan: inner }))?;
            expansion_registry.register(Arc::new(FixtureNode {
                class_type: "InnerOutput".to_owned(),
                calls: Arc::new(AtomicUsize::new(0)),
            }))?;
            let expansion_engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(expansion_registry),
                Arc::new(Mutex::new(NativeCache::new(8)?)),
                Arc::new(RecordingEffectCoordinator::default()),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?;
            let expanded = expansion_engine
                .execute(
                    &outer,
                    AttemptId(Uuid::from_u128(2)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(expanded.state, AttemptState::Succeeded);
            assert_eq!(expanded.outputs[&NodeId::from("expand")], [json!([42, 42])]);
            assert_eq!(
                expanded
                    .outputs
                    .keys()
                    .filter(|node_id| node_id.0.starts_with("expand::expansion-0-"))
                    .count(),
                2
            );

            let write = compile_plan(
                vec![runtime_descriptor(
                    "Write",
                    true,
                    BTreeMap::new(),
                    EffectClass::WritesArtifact,
                )],
                BTreeMap::from([(
                    NodeId::from("write"),
                    PromptNode {
                        class_type: "Write".to_owned(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let mut write_registry = NativeNodeRegistry::default();
            write_registry.register(Arc::new(FixtureNode {
                class_type: "Write".to_owned(),
                calls: Arc::new(AtomicUsize::new(0)),
            }))?;
            let effects = Arc::new(RecordingEffectCoordinator::default());
            let write_engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(write_registry),
                Arc::new(Mutex::new(NativeCache::new(8)?)),
                effects.clone(),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?;
            let committed = write_engine
                .execute(
                    &write,
                    AttemptId(Uuid::from_u128(3)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(committed.state, AttemptState::Succeeded);
            assert_eq!(effects.committed(), BTreeSet::from([Uuid::from_u128(99)]));
            assert!(effects.rolled_back().is_empty());

            let failed_write = compile_plan(
                vec![
                    runtime_descriptor(
                        "Write",
                        false,
                        BTreeMap::new(),
                        EffectClass::WritesArtifact,
                    ),
                    runtime_descriptor(
                        "Block",
                        true,
                        BTreeMap::from([(
                            "value".to_owned(),
                            input(ValueType::Any, false, InputMode::Scalar, false),
                        )]),
                        EffectClass::Pure,
                    ),
                ],
                BTreeMap::from([
                    (
                        NodeId::from("write"),
                        PromptNode {
                            class_type: "Write".to_owned(),
                            inputs: BTreeMap::new(),
                            unknown: BTreeMap::new(),
                        },
                    ),
                    (
                        NodeId::from("block"),
                        PromptNode {
                            class_type: "Block".to_owned(),
                            inputs: BTreeMap::from([("value".to_owned(), json!(["write", 0]))]),
                            unknown: BTreeMap::new(),
                        },
                    ),
                ]),
            )?;
            let mut failed_registry = NativeNodeRegistry::default();
            for class_type in ["Write", "Block"] {
                failed_registry.register(Arc::new(FixtureNode {
                    class_type: class_type.to_owned(),
                    calls: Arc::new(AtomicUsize::new(0)),
                }))?;
            }
            let failed_effects = Arc::new(RecordingEffectCoordinator::default());
            let failed_engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(failed_registry),
                Arc::new(Mutex::new(NativeCache::new(8)?)),
                failed_effects.clone(),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?;
            let failed = failed_engine
                .execute(
                    &failed_write,
                    AttemptId(Uuid::from_u128(4)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(failed.state, AttemptState::Failed);
            assert!(failed.outputs.is_empty());
            assert!(failed_effects.committed().is_empty());
            assert_eq!(
                failed_effects.rolled_back(),
                BTreeSet::from([Uuid::from_u128(99)])
            );

            let mut event_failure_registry = NativeNodeRegistry::default();
            event_failure_registry.register(Arc::new(FixtureNode {
                class_type: "Write".to_owned(),
                calls: Arc::new(AtomicUsize::new(0)),
            }))?;
            let event_failure_effects = Arc::new(RecordingEffectCoordinator::default());
            let event_bus = ExecutionEventBus::new(1)?;
            let _receiver = event_bus.subscribe();
            let event_failure_engine = ExecutionEngine::new_with_workspace_authorization(
                ProfileId(Uuid::nil()),
                Arc::new(event_failure_registry),
                Arc::new(Mutex::new(NativeCache::new(8)?)),
                event_failure_effects.clone(),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            )?
            .with_event_bus(event_bus);
            let event_failure = event_failure_engine
                .execute(
                    &write,
                    AttemptId(Uuid::from_u128(5)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(event_failure.state, AttemptState::Failed);
            assert!(event_failure.outputs.is_empty());
            assert!(event_failure_effects.committed().is_empty());
            assert_eq!(
                event_failure_effects.rolled_back(),
                BTreeSet::from([Uuid::from_u128(99)])
            );
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    pub(crate) fn val_domain_004_executor_case_results()
    -> Result<Vec<(&'static str, bool)>, Box<dyn std::error::Error>> {
        val_domain_004_graph_list_lazy_async_cache_and_effects_execute_transactionally()?;
        val_domain_004_repeat_last_and_output_list_flattening_are_exact()?;
        val_domain_004_canonical_cancellation_and_ui_cache_semantics()?;
        val_domain_004_cache_tracks_demanded_dependency_identity()?;
        val_domain_004_cancellation_and_blockers_publish_no_partial_outputs()?;
        val_domain_004_lazy_dependencies_execute_only_when_demanded()?;
        val_domain_004_expansion_and_transactional_effect_batches_are_real_execution_paths()?;
        Ok(vec![
            ("executor_graph_list_async_cache_effects", true),
            ("executor_repeat_last_output_list", true),
            ("executor_canonical_cancel_ui_cache", true),
            ("executor_demanded_dependency_cache_identity", true),
            ("executor_blocker_output_fence", true),
            ("executor_lazy_demand", true),
            ("executor_expansion_effect_atomicity", true),
        ])
    }

    #[test]
    fn native_compile_policy_seals_invocation_and_canonical_cache_dimensions()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_workspace_backend, workspace_authority) =
            CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
        assert!(!compiler_is_compiling_exact_native(
            NativeCompilePhase::Eager,
            &CancellationToken::default(),
        )?);
        assert!(compiler_is_compiling_exact_native(
            NativeCompilePhase::CapturingGraph,
            &CancellationToken::default(),
        )?);
        let policy = NativeCompilePolicy::from_source_configuration(
            "inductor",
            BTreeMap::from([(
                "guard_filter_fn".to_owned(),
                "skip_torch_compile_dict".to_owned(),
            )]),
            Some("default".to_owned()),
            true,
            Some(false),
        )?;
        let same = NativeCompilePolicy::from_source_configuration(
            "inductor",
            BTreeMap::from([(
                "guard_filter_fn".to_owned(),
                "skip_torch_compile_dict".to_owned(),
            )]),
            Some("default".to_owned()),
            true,
            Some(false),
        )?;
        assert_eq!(
            policy.guard_policy(),
            NativeCompileGuardPolicy::SkipTransformerOptionsDictionary
        );
        assert_eq!(policy.cache_token()?, same.cache_token()?);
        let compiled =
            compile_exact_native(41_u32, policy.clone(), None, &CancellationToken::default())?;
        assert_eq!(
            compiled.invoke(&CancellationToken::default(), |model| model + 1)?,
            42
        );
        let cancelled = CancellationToken::default();
        cancelled.cancel();
        assert!(compiled.invoke(&cancelled, |model| *model).is_err());
        assert!(matches!(
            compile_exact_native(
                41_u32,
                NativeCompilePolicy::from_source_configuration(
                    "cudagraphs",
                    BTreeMap::new(),
                    None,
                    false,
                    None,
                )?,
                None,
                &cancelled,
            ),
            Err(NativeCompileError::Cancelled)
        ));

        let registry = Arc::new(NativeNodeRegistry::default());
        let engine = ExecutionEngine::new_with_workspace_authorization(
            ProfileId(Uuid::nil()),
            registry,
            Arc::new(Mutex::new(NativeCache::new(1)?)),
            Arc::new(RecordingEffectCoordinator::default()),
            "registry-v1",
            workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
        )?
        .with_native_compile_policy(&policy)?;
        assert_eq!(engine.backend, "native-graph");
        assert_eq!(engine.configuration_token, policy.cache_token()?);
        assert!(
            NativeCompilePolicy::from_source_configuration(
                "python-dynamo",
                BTreeMap::new(),
                None,
                false,
                None,
            )
            .is_err()
        );
        assert!(matches!(
            NativeCompilePolicy::from_source_configuration(
                "inductor",
                BTreeMap::from([("epilogue_fusion".to_owned(), "true".to_owned())]),
                None,
                false,
                None,
            ),
            Err(NativeCompileError::UnsupportedOption { .. })
        ));
        let cuda_graphs = NativeCompilePolicy::from_source_configuration(
            "cudagraphs",
            BTreeMap::new(),
            None,
            false,
            None,
        )?;
        assert!(matches!(
            compile_exact_native(
                41_u32,
                cuda_graphs.clone(),
                None,
                &CancellationToken::default()
            ),
            Err(NativeCompileError::UncertifiedCudaGraphs)
        ));
        let cpu_capabilities = BackendCapabilityMatrix::new_with_properties(
            DeviceId::CPU,
            Vec::new(),
            Vec::new(),
            Some(NativeDeviceProperties::new(
                DeviceId::CPU,
                "CPU",
                1,
                0,
                0,
                None,
                true,
            )?),
        )?;
        assert!(matches!(
            compile_exact_native(
                41_u32,
                cuda_graphs.clone(),
                Some(&cpu_capabilities),
                &CancellationToken::default()
            ),
            Err(NativeCompileError::UncertifiedCudaGraphs)
        ));
        let cuda_device = DeviceId::new(DeviceKind::Cuda, 0);
        let cuda_capabilities = BackendCapabilityMatrix::new_with_properties(
            cuda_device,
            Vec::new(),
            Vec::new(),
            Some(NativeDeviceProperties::new(
                cuda_device,
                "Certified fixture CUDA",
                1,
                9,
                0,
                Some("sm_90".to_owned()),
                true,
            )?),
        )?;
        compile_exact_native(
            41_u32,
            cuda_graphs,
            Some(&cuda_capabilities),
            &CancellationToken::default(),
        )?;
        Ok(())
    }
}
