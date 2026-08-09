use crate::{
    AttemptEvent, AttemptEventKind, AttemptState, CacheEntry, CacheKey, CompiledNode, CompiledPlan,
    EventBusError, ExecutionEventBus, InputBinding, NativeCache, PromptCompileError,
};
use chrono::Utc;
pub use comfy_nodes::{
    NativeCacheDependencies as CacheDependencies, NativeCachePolicy as RuntimeCachePolicy,
    NativeEffectClass as EffectClass, NativeNode, NativeNodeBinding,
    NativeNodeContext as NodeContext, NativeNodeDescriptor as RuntimeNodeDescriptor,
    NativeNodeFailure as NodeFailure, NativeNodeFailureKind as NodeFailureKind,
    NativeNodeOutcome as NodeOutcome, NativeNodePresentation as RuntimeNodePresentation,
    NativeOutputDescriptor as RuntimeOutputDescriptor, NativePortCardinality as InputMode,
    NativePreparedEffectRequest as PreparedEffectRequest,
};
use comfy_nodes::{
    NativeHandleStore, NativeHandleStoreError, NativeHandleStoreIdentity, NativeHandleType,
    NativeNodeBindingDisposition, NativeNodeContractError, NativeOpaqueHandle, NativeStoredPayload,
    NativeValue, NodeRegistry,
};
use comfy_tensor::{BackendCapabilityMatrix, ScratchReservation};
#[cfg(test)]
use comfy_tensor::{CpuWorkspaceAuthority, DeviceId, NativeDeviceProperties};
use comfy_types::{
    AttemptId, CancellationToken, DeviceKind, NodeId, ProfileId, PromptId, PromptSubmission,
};
use futures::future::BoxFuture;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use thiserror::Error;
use uuid::Uuid;

pub const MAX_EXPANSION_DEPTH: usize = 64;
pub const MAX_EFFECTS_PER_NODE: usize = 4_096;
pub const MAX_EFFECT_METADATA_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_NATIVE_COMPILE_OPTIONS: usize = 64;
pub const MAX_NATIVE_COMPILE_TEXT_BYTES: usize = 4_096;
pub const MAX_RUNTIME_NATIVE_HANDLES: usize = 1_000_000;
pub const MAX_RUNTIME_NATIVE_HANDLE_BYTES: usize = 8 * 1024 * 1024 * 1024;

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreparedEffect {
    pub prompt_id: PromptId,
    pub attempt_id: AttemptId,
    pub node_id: NodeId,
    pub transaction_id: Uuid,
    pub metadata: Vec<u8>,
}

#[derive(Clone, Default)]
pub struct NativeNodeRegistry {
    nodes: BTreeMap<String, Arc<dyn NativeNode>>,
    descriptors: BTreeMap<String, RuntimeNodeDescriptor>,
    presentations: BTreeMap<String, RuntimeNodePresentation>,
    bindings: BTreeMap<String, RegistryBindingState>,
}

#[derive(Clone, Debug)]
struct RegistryBindingState {
    disposition: NativeNodeBindingDisposition,
    provider_activated: bool,
    implementation_namespace: Option<String>,
    catalog_source: String,
    reason: Option<String>,
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
        let implementation_namespace = node.implementation_namespace().to_owned();
        if implementation_namespace.trim().is_empty() {
            return Err(ExecutionError::InvalidNodeImplementation(class_type));
        }
        self.nodes.insert(class_type.clone(), node);
        self.bindings.insert(
            class_type,
            RegistryBindingState {
                disposition: NativeNodeBindingDisposition::Executable,
                provider_activated: false,
                implementation_namespace: Some(implementation_namespace.clone()),
                catalog_source: implementation_namespace,
                reason: None,
            },
        );
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
        if descriptor.validate().is_err() {
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

    pub fn validate_comprehensive_bindings(&self) -> Result<(), NativeNodeRegistryError> {
        for (class_type, descriptor) in &self.descriptors {
            descriptor.validate()?;
            let binding = self
                .bindings
                .get(class_type)
                .ok_or_else(|| NativeNodeRegistryError::IncompleteRegistry(class_type.clone()))?;
            let presentation = self
                .presentations
                .get(class_type)
                .ok_or_else(|| NativeNodeRegistryError::IncompleteRegistry(class_type.clone()))?;
            presentation.validate()?;
            let expected_output_names = descriptor
                .outputs
                .iter()
                .map(|output| output.name.as_str())
                .collect::<Vec<_>>();
            if binding.catalog_source.is_empty()
                || presentation
                    .output_names
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    != expected_output_names
            {
                return Err(NativeNodeRegistryError::IncompleteRegistry(
                    class_type.clone(),
                ));
            }
            let has_node = self.nodes.contains_key(class_type);
            let node_matches = self.nodes.get(class_type).is_none_or(|node| {
                node.class_type() == class_type
                    && node.implementation_version() == descriptor.implementation_version
                    && binding.implementation_namespace.as_deref()
                        == Some(node.implementation_namespace())
            });
            let valid = match binding.disposition {
                NativeNodeBindingDisposition::Executable => {
                    has_node
                        && node_matches
                        && !binding.provider_activated
                        && binding.reason.is_none()
                        && binding
                            .implementation_namespace
                            .as_deref()
                            .is_some_and(|value| !value.is_empty())
                }
                NativeNodeBindingDisposition::ProviderRequired => {
                    has_node == binding.provider_activated
                        && node_matches
                        && binding
                            .reason
                            .as_deref()
                            .is_some_and(|value| !value.is_empty())
                        && binding
                            .implementation_namespace
                            .as_deref()
                            .is_some_and(|value| !value.is_empty())
                }
                NativeNodeBindingDisposition::Unavailable => {
                    !has_node
                        && !binding.provider_activated
                        && binding
                            .reason
                            .as_deref()
                            .is_some_and(|value| !value.is_empty())
                        && binding.implementation_namespace.is_none()
                }
            };
            if !valid {
                return Err(NativeNodeRegistryError::IncompleteRegistry(
                    class_type.clone(),
                ));
            }
        }
        if self.nodes.keys().any(|class_type| {
            !self.descriptors.contains_key(class_type) || !self.bindings.contains_key(class_type)
        }) || self.presentations.keys().any(|class_type| {
            !self.descriptors.contains_key(class_type) || !self.bindings.contains_key(class_type)
        }) || self.bindings.keys().any(|class_type| {
            !self.descriptors.contains_key(class_type)
                || !self.presentations.contains_key(class_type)
        }) {
            return Err(NativeNodeRegistryError::IncompleteRegistry(
                "registry key sets differ".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn bindings_are_comprehensive(&self) -> bool {
        self.validate_comprehensive_bindings().is_ok()
    }

    pub fn implementation_namespace(&self, class_type: &str) -> Option<&str> {
        self.nodes
            .get(class_type)
            .map(|node| node.implementation_namespace())
    }

    pub fn binding_disposition(&self, class_type: &str) -> Option<NativeNodeBindingDisposition> {
        self.bindings.get(class_type).map(|binding| {
            if binding.provider_activated {
                NativeNodeBindingDisposition::Executable
            } else {
                binding.disposition
            }
        })
    }

    pub fn binding_declared_disposition(
        &self,
        class_type: &str,
    ) -> Option<NativeNodeBindingDisposition> {
        self.bindings
            .get(class_type)
            .map(|binding| binding.disposition)
    }

    pub fn provider_binding_is_activated(&self, class_type: &str) -> Option<bool> {
        self.bindings
            .get(class_type)
            .map(|binding| binding.provider_activated)
    }

    pub fn binding_source(&self, class_type: &str) -> Option<&str> {
        self.bindings
            .get(class_type)
            .map(|binding| binding.catalog_source.as_str())
    }

    pub fn binding_implementation_namespace(&self, class_type: &str) -> Option<&str> {
        self.bindings
            .get(class_type)
            .and_then(|binding| binding.implementation_namespace.as_deref())
    }

    pub fn unavailable_reason(&self, class_type: &str) -> Option<&str> {
        if self.nodes.contains_key(class_type) {
            return None;
        }
        self.bindings
            .get(class_type)
            .filter(|binding| binding.disposition != NativeNodeBindingDisposition::Executable)
            .and_then(|binding| binding.reason.as_deref())
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

    pub fn register_native_bindings(
        &mut self,
        bindings: impl IntoIterator<Item = NativeNodeBinding>,
    ) -> Result<(), NativeNodeRegistryError> {
        let catalog = NodeRegistry::built_in()?;
        let bindings = bindings.into_iter().collect::<Vec<_>>();
        for binding in &bindings {
            catalog.validate_native_binding(binding)?;
        }
        let mut next = self.clone();
        for binding in bindings {
            binding.validate()?;
            let class_type = binding.descriptor().class_type.clone();
            if next.descriptors.contains_key(&class_type)
                || next.nodes.contains_key(&class_type)
                || next.presentations.contains_key(&class_type)
                || next.bindings.contains_key(&class_type)
            {
                return Err(NativeNodeRegistryError::DuplicateBinding(class_type));
            }
            let descriptor = binding.descriptor().clone();
            let presentation = binding.presentation().clone();
            let catalog_source = catalog
                .descriptor(&class_type)
                .map(|descriptor| descriptor.source_file.clone())
                .ok_or_else(|| NativeNodeRegistryError::BindingMismatch(class_type.clone()))?;
            match binding {
                NativeNodeBinding::Executable { node, .. } => {
                    let source = node.implementation_namespace().to_owned();
                    next.register_descriptor(descriptor)?;
                    next.register(node)?;
                    next.presentations.insert(class_type.clone(), presentation);
                    next.bindings.insert(
                        class_type,
                        RegistryBindingState {
                            disposition: NativeNodeBindingDisposition::Executable,
                            provider_activated: false,
                            implementation_namespace: Some(source),
                            catalog_source,
                            reason: None,
                        },
                    );
                }
                NativeNodeBinding::ProviderRequired {
                    provider, reason, ..
                } => {
                    next.register_descriptor(descriptor)?;
                    next.presentations.insert(class_type.clone(), presentation);
                    next.bindings.insert(
                        class_type,
                        RegistryBindingState {
                            disposition: NativeNodeBindingDisposition::ProviderRequired,
                            provider_activated: false,
                            implementation_namespace: Some(provider),
                            catalog_source,
                            reason: Some(reason),
                        },
                    );
                }
                NativeNodeBinding::Unavailable { reason, .. } => {
                    next.register_descriptor(descriptor)?;
                    next.presentations.insert(class_type.clone(), presentation);
                    next.bindings.insert(
                        class_type,
                        RegistryBindingState {
                            disposition: NativeNodeBindingDisposition::Unavailable,
                            provider_activated: false,
                            implementation_namespace: None,
                            catalog_source,
                            reason: Some(reason),
                        },
                    );
                }
            }
        }
        *self = next;
        Ok(())
    }

    pub fn activate_provider_bindings(
        &mut self,
        bindings: impl IntoIterator<Item = (Arc<dyn NativeNode>, RuntimeNodePresentation)>,
    ) -> Result<(), NativeNodeRegistryError> {
        let mut next = self.clone();
        let mut activated = BTreeSet::new();
        for (node, presentation) in bindings {
            let class_type = node.class_type().to_owned();
            if !activated.insert(class_type.clone()) || next.nodes.contains_key(&class_type) {
                return Err(NativeNodeRegistryError::DuplicateBinding(class_type));
            }
            let descriptor = next
                .descriptors
                .get(&class_type)
                .ok_or_else(|| NativeNodeRegistryError::BindingMismatch(class_type.clone()))?;
            let expected_presentation = next
                .presentations
                .get(&class_type)
                .ok_or_else(|| NativeNodeRegistryError::BindingMismatch(class_type.clone()))?;
            let binding = next
                .bindings
                .get(&class_type)
                .ok_or_else(|| NativeNodeRegistryError::BindingMismatch(class_type.clone()))?;
            if binding.disposition != NativeNodeBindingDisposition::ProviderRequired
                || descriptor.implementation_version != node.implementation_version()
                || binding.implementation_namespace.as_deref()
                    != Some(node.implementation_namespace())
                || expected_presentation != &presentation
            {
                return Err(NativeNodeRegistryError::BindingMismatch(class_type));
            }
            next.nodes.insert(class_type.clone(), node);
            if let Some(binding) = next.bindings.get_mut(&class_type) {
                binding.provider_activated = true;
            }
        }
        next.validate_comprehensive_bindings()?;
        *self = next;
        Ok(())
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
                if presentation.validate().is_err()
                    || presentation.output_names.len() != descriptor.outputs.len()
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
    #[error("native node binding for `{0}` is already registered")]
    DuplicateBinding(String),
    #[error("native node registry is incomplete at `{0}`")]
    IncompleteRegistry(String),
    #[error(transparent)]
    Contract(#[from] NativeNodeContractError),
    #[error(transparent)]
    Catalog(#[from] comfy_nodes::NodeRegistryError),
}

#[derive(Clone)]
pub struct NativeHandleStoreGeneration {
    state: Arc<NativeHandleStoreGenerationState>,
}

struct NativeHandleStoreGenerationState {
    identity: NativeHandleStoreIdentity,
    next_generation: AtomicU64,
    data: Mutex<NativeHandleStoreData>,
    capacity: usize,
    byte_capacity: usize,
    #[cfg(test)]
    test_hooks: Mutex<NativeHandleStoreTestHooks>,
}

#[cfg(test)]
#[derive(Default)]
struct NativeHandleStoreTestHooks {
    after_publish_insert: Option<Arc<dyn Fn() + Send + Sync>>,
    after_cache_insert: Option<Arc<dyn Fn() + Send + Sync>>,
}

#[derive(Default)]
struct NativeHandleStoreData {
    values: BTreeMap<String, StoredNativeHandle>,
    resident_bytes: usize,
}

fn remove_stored_handle(
    data: &mut NativeHandleStoreData,
    identifier: &str,
) -> Option<StoredNativeHandle> {
    let resident_bytes = data.values.get(identifier)?.resident_bytes;
    let next_resident_bytes = data.resident_bytes.checked_sub(resident_bytes)?;
    let removed = data.values.remove(identifier)?;
    data.resident_bytes = next_resident_bytes;
    Some(removed)
}

#[derive(Clone)]
struct StoredNativeHandle {
    handle_type: NativeHandleType,
    generation: u64,
    digest_sha256: String,
    value: Arc<NativeStoredPayload>,
    committed: bool,
    published_by: AttemptId,
    resident_bytes: usize,
    roots: usize,
}

#[derive(Clone)]
pub struct NativeHandleLease {
    inner: Arc<NativeHandleLeaseInner>,
}

struct NativeHandleLeaseInner {
    generation: NativeHandleStoreGeneration,
    handles: Vec<NativeOpaqueHandle>,
}

impl fmt::Debug for NativeHandleLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeHandleLease")
            .field("handle_count", &self.inner.handles.len())
            .finish_non_exhaustive()
    }
}

impl Drop for NativeHandleLeaseInner {
    fn drop(&mut self) {
        self.generation.release_roots(&self.handles);
    }
}

impl PartialEq for NativeHandleLease {
    fn eq(&self, other: &Self) -> bool {
        self.inner.generation.identity() == other.inner.generation.identity()
            && self.inner.handles == other.inner.handles
    }
}

impl NativeHandleLease {
    pub(crate) fn covers_values(&self, values: &[NativeValue]) -> bool {
        let mut handles = Vec::new();
        for value in values {
            collect_native_value_handles(value, &mut handles);
        }
        let unique = handles
            .into_iter()
            .map(|handle| {
                (
                    (handle.identifier().to_owned(), handle.generation()),
                    handle.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        unique.len() == self.inner.handles.len()
            && unique
                .values()
                .zip(self.inner.handles.iter())
                .all(|(expected, actual)| expected == actual)
    }

    pub(crate) fn store_identity(&self) -> NativeHandleStoreIdentity {
        self.inner.generation.identity()
    }
}

impl fmt::Debug for NativeHandleStoreGeneration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeHandleStoreGeneration")
            .field("identity", &self.state.identity)
            .finish_non_exhaustive()
    }
}

impl NativeHandleStoreGeneration {
    pub fn new() -> Result<Self, NativeHandleStoreError> {
        Self::with_capacities(MAX_RUNTIME_NATIVE_HANDLES, MAX_RUNTIME_NATIVE_HANDLE_BYTES)
    }

    pub fn with_capacity(capacity: usize) -> Result<Self, NativeHandleStoreError> {
        Self::with_capacities(capacity, MAX_RUNTIME_NATIVE_HANDLE_BYTES)
    }

    pub fn with_capacities(
        capacity: usize,
        byte_capacity: usize,
    ) -> Result<Self, NativeHandleStoreError> {
        if capacity == 0 || byte_capacity == 0 {
            return Err(NativeHandleStoreError::Rejected(
                "native handle store capacities must be nonzero".to_owned(),
            ));
        }
        Ok(Self {
            state: Arc::new(NativeHandleStoreGenerationState {
                identity: NativeHandleStoreIdentity::new(Uuid::new_v4(), Uuid::new_v4())?,
                next_generation: AtomicU64::new(1),
                data: Mutex::new(NativeHandleStoreData::default()),
                capacity,
                byte_capacity,
                #[cfg(test)]
                test_hooks: Mutex::new(NativeHandleStoreTestHooks::default()),
            }),
        })
    }

    pub fn identity(&self) -> NativeHandleStoreIdentity {
        self.state.identity
    }

    pub fn len(&self) -> usize {
        self.state.data.lock().values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn resident_bytes(&self) -> usize {
        self.state.data.lock().resident_bytes
    }

    fn acquire_lease<'a>(
        &self,
        handles: impl IntoIterator<Item = &'a NativeOpaqueHandle>,
    ) -> Result<Option<NativeHandleLease>, NativeHandleStoreError> {
        let mut handles_by_key = BTreeMap::new();
        for handle in handles {
            handle.validate()?;
            if handle.store_identity().store_id != self.identity().store_id {
                return Err(NativeHandleStoreError::WrongStore);
            }
            if handle.store_identity().generation_id != self.identity().generation_id {
                return Err(NativeHandleStoreError::WrongGeneration);
            }
            let key = (handle.identifier().to_owned(), handle.generation());
            if let Some(existing) = handles_by_key.insert(key, handle.clone())
                && existing != *handle
            {
                return Err(NativeHandleStoreError::DigestMismatch);
            }
        }
        let handles = handles_by_key;
        if handles.is_empty() {
            return Ok(None);
        }
        let mut data = self.state.data.lock();
        for ((identifier, generation), handle) in &handles {
            let record = data
                .values
                .get(identifier)
                .ok_or_else(|| NativeHandleStoreError::Missing(identifier.clone()))?;
            if !record.committed
                || record.generation != *generation
                || record.handle_type != *handle.handle_type()
            {
                return Err(NativeHandleStoreError::WrongGeneration);
            }
            if Some(record.digest_sha256.as_str()) != handle.digest_sha256() {
                return Err(NativeHandleStoreError::DigestMismatch);
            }
            if record.roots == usize::MAX {
                return Err(NativeHandleStoreError::Rejected(
                    "native handle root count overflowed".to_owned(),
                ));
            }
        }
        for (identifier, _) in handles.keys() {
            if let Some(record) = data.values.get_mut(identifier) {
                record.roots += 1;
            }
        }
        Ok(Some(NativeHandleLease {
            inner: Arc::new(NativeHandleLeaseInner {
                generation: self.clone(),
                handles: handles.into_values().collect(),
            }),
        }))
    }

    fn release_roots(&self, handles: &[NativeOpaqueHandle]) {
        let mut data = self.state.data.lock();
        let mut removals = Vec::new();
        for handle in handles {
            if let Some(record) = data.values.get_mut(handle.identifier())
                && record.generation == handle.generation()
                && record.roots > 0
            {
                record.roots -= 1;
                if record.roots == 0 && record.committed {
                    removals.push(handle.identifier().to_owned());
                }
            }
        }
        for identifier in removals {
            remove_stored_handle(&mut data, &identifier);
        }
    }

    fn collect_unrooted_attempt(&self, attempt_id: AttemptId) {
        let mut data = self.state.data.lock();
        let identifiers = data
            .values
            .iter()
            .filter(|(_, record)| {
                record.committed && record.published_by == attempt_id && record.roots == 0
            })
            .map(|(identifier, _)| identifier.clone())
            .collect::<Vec<_>>();
        for identifier in identifiers {
            remove_stored_handle(&mut data, &identifier);
        }
    }

    fn session(&self, attempt_id: AttemptId) -> Arc<RuntimeNativeHandleStoreSession> {
        Arc::new(RuntimeNativeHandleStoreSession {
            generation: self.clone(),
            attempt_id,
            staged: Mutex::new(NativeHandleStoreSessionStage::default()),
        })
    }

    pub fn handle_store_for_attempt(&self, attempt_id: AttemptId) -> Arc<dyn NativeHandleStore> {
        self.session(attempt_id)
    }

    #[cfg(test)]
    fn set_after_publish_insert_hook(&self, hook: Arc<dyn Fn() + Send + Sync>) {
        self.state.test_hooks.lock().after_publish_insert = Some(hook);
    }

    #[cfg(test)]
    fn set_after_cache_insert_hook(&self, hook: Arc<dyn Fn() + Send + Sync>) {
        self.state.test_hooks.lock().after_cache_insert = Some(hook);
    }

    #[cfg(test)]
    fn run_after_publish_insert_hook(&self) {
        let hook = self.state.test_hooks.lock().after_publish_insert.clone();
        if let Some(hook) = hook {
            hook();
        }
    }

    #[cfg(test)]
    fn run_after_cache_insert_hook(&self) {
        let hook = self.state.test_hooks.lock().after_cache_insert.clone();
        if let Some(hook) = hook {
            hook();
        }
    }
}

struct RuntimeNativeHandleStoreSession {
    generation: NativeHandleStoreGeneration,
    attempt_id: AttemptId,
    staged: Mutex<NativeHandleStoreSessionStage>,
}

#[derive(Default)]
struct NativeHandleStoreSessionStage {
    identifiers: Vec<String>,
    closed: bool,
}

impl fmt::Debug for RuntimeNativeHandleStoreSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeNativeHandleStoreSession")
            .field("identity", &self.generation.identity())
            .field("attempt_id", &self.attempt_id)
            .finish_non_exhaustive()
    }
}

impl RuntimeNativeHandleStoreSession {
    fn checkpoint(&self) -> usize {
        self.staged.lock().identifiers.len()
    }

    fn rollback_from(&self, checkpoint: usize) {
        let identifiers = {
            let mut staged = self.staged.lock();
            if checkpoint >= staged.identifiers.len() {
                return;
            }
            staged.identifiers.split_off(checkpoint)
        };
        let mut data = self.generation.state.data.lock();
        for identifier in identifiers {
            if data
                .values
                .get(&identifier)
                .is_some_and(|record| !record.committed && record.published_by == self.attempt_id)
            {
                remove_stored_handle(&mut data, &identifier);
            }
        }
    }

    fn rollback_all(&self) {
        let identifiers = {
            let mut staged = self.staged.lock();
            staged.closed = true;
            std::mem::take(&mut staged.identifiers)
        };
        let mut data = self.generation.state.data.lock();
        for identifier in identifiers {
            if data
                .values
                .get(&identifier)
                .is_some_and(|record| !record.committed && record.published_by == self.attempt_id)
            {
                remove_stored_handle(&mut data, &identifier);
            }
        }
    }

    fn commit(&self) {
        let identifiers = {
            let mut staged = self.staged.lock();
            staged.closed = true;
            std::mem::take(&mut staged.identifiers)
        };
        let mut data = self.generation.state.data.lock();
        for identifier in identifiers {
            if let Some(record) = data.values.get_mut(&identifier)
                && record.published_by == self.attempt_id
            {
                record.committed = true;
            }
        }
    }

    fn validate_handle(
        &self,
        handle: &NativeOpaqueHandle,
        expected_type: &NativeHandleType,
        cancellation: &CancellationToken,
    ) -> Result<(), NativeHandleStoreError> {
        cancellation
            .check()
            .map_err(|_| NativeHandleStoreError::Cancelled)?;
        handle.validate()?;
        let identity = self.generation.identity();
        if handle.store_identity().store_id != identity.store_id {
            return Err(NativeHandleStoreError::WrongStore);
        }
        if handle.store_identity().generation_id != identity.generation_id {
            return Err(NativeHandleStoreError::WrongGeneration);
        }
        if handle.handle_type() != expected_type {
            return Err(NativeHandleStoreError::WrongType {
                expected: expected_type.type_id.clone(),
                actual: handle.handle_type().type_id.clone(),
            });
        }
        Ok(())
    }
}

impl NativeHandleStore for RuntimeNativeHandleStoreSession {
    fn identity(&self) -> NativeHandleStoreIdentity {
        self.generation.identity()
    }

    fn attempt_id(&self) -> AttemptId {
        self.attempt_id
    }

    fn resolve(
        &self,
        handle: &NativeOpaqueHandle,
        expected_type: &NativeHandleType,
        cancellation: &CancellationToken,
    ) -> Result<Arc<NativeStoredPayload>, NativeHandleStoreError> {
        self.validate_handle(handle, expected_type, cancellation)?;
        let data = self.generation.state.data.lock();
        let record = data
            .values
            .get(handle.identifier())
            .ok_or_else(|| NativeHandleStoreError::Missing(handle.identifier().to_owned()))?;
        if !record.committed && record.published_by != self.attempt_id {
            return Err(NativeHandleStoreError::Missing(
                handle.identifier().to_owned(),
            ));
        }
        if record.generation != handle.generation() || record.handle_type != *expected_type {
            return Err(NativeHandleStoreError::WrongGeneration);
        }
        if Some(record.digest_sha256.as_str()) != handle.digest_sha256() {
            return Err(NativeHandleStoreError::DigestMismatch);
        }
        record.value.validate()?;
        if record.value.handle_type()? != record.handle_type
            || record.value.digest_sha256() != record.digest_sha256
            || record.value.resident_bytes()? != record.resident_bytes
        {
            return Err(NativeHandleStoreError::InvalidPayload(
                comfy_nodes::NativeStoredPayloadError::ProjectionChanged,
            ));
        }
        Ok(record.value.clone())
    }

    fn publish(
        &self,
        payload: NativeStoredPayload,
        cancellation: &CancellationToken,
    ) -> Result<NativeOpaqueHandle, NativeHandleStoreError> {
        cancellation
            .check()
            .map_err(|_| NativeHandleStoreError::Cancelled)?;
        payload.validate()?;
        let handle_type = payload.handle_type()?;
        let digest_sha256 = payload.digest_sha256();
        let resident_bytes = payload.resident_bytes()?;
        let mut staged = self.staged.lock();
        if staged.closed {
            return Err(NativeHandleStoreError::Rejected(
                "native handle store attempt session is closed".to_owned(),
            ));
        }
        let mut data = self.generation.state.data.lock();
        let next_resident_bytes =
            data.resident_bytes
                .checked_add(resident_bytes)
                .ok_or_else(|| {
                    NativeHandleStoreError::Rejected(
                        "native handle resident byte count overflowed".to_owned(),
                    )
                })?;
        if data.values.len() >= self.generation.state.capacity
            || next_resident_bytes > self.generation.state.byte_capacity
        {
            return Err(NativeHandleStoreError::Rejected(
                "native handle store capacity is exhausted".to_owned(),
            ));
        }
        let generation = self
            .generation
            .state
            .next_generation
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |generation| {
                generation.checked_add(1).filter(|next| *next != 0)
            })
            .map_err(|_| {
                NativeHandleStoreError::Rejected(
                    "native handle generation was exhausted".to_owned(),
                )
            })?;
        let identifier = format!("native-{generation:016x}");
        let handle = NativeOpaqueHandle::new(
            handle_type.clone(),
            self.generation.identity(),
            identifier.clone(),
            generation,
            Some(digest_sha256.clone()),
        )?;
        data.values.insert(
            identifier.clone(),
            StoredNativeHandle {
                handle_type,
                generation,
                digest_sha256,
                value: Arc::new(payload),
                committed: false,
                published_by: self.attempt_id,
                resident_bytes,
                roots: 0,
            },
        );
        data.resident_bytes = next_resident_bytes;
        drop(data);
        staged.identifiers.push(identifier);
        #[cfg(test)]
        self.generation.run_after_publish_insert_hook();
        if cancellation.is_cancelled() {
            staged.identifiers.pop();
            let mut data = self.generation.state.data.lock();
            remove_stored_handle(&mut data, handle.identifier());
            return Err(NativeHandleStoreError::Cancelled);
        }
        drop(staged);
        Ok(handle)
    }

    fn revoke(
        &self,
        handle: &NativeOpaqueHandle,
        cancellation: &CancellationToken,
    ) -> Result<(), NativeHandleStoreError> {
        self.validate_handle(handle, handle.handle_type(), cancellation)?;
        let mut staged = self.staged.lock();
        if staged.closed {
            return Err(NativeHandleStoreError::Rejected(
                "native handle store attempt session is closed".to_owned(),
            ));
        }
        let mut data = self.generation.state.data.lock();
        let removable = data
            .values
            .get(handle.identifier())
            .is_some_and(|record| !record.committed && record.published_by == self.attempt_id);
        if !removable {
            return Err(NativeHandleStoreError::Rejected(
                "native handles may only be revoked by their publishing attempt before commit"
                    .to_owned(),
            ));
        }
        let removed = remove_stored_handle(&mut data, handle.identifier());
        if removed.is_none() {
            return Err(NativeHandleStoreError::Missing(
                handle.identifier().to_owned(),
            ));
        }
        staged
            .identifiers
            .retain(|identifier| identifier != handle.identifier());
        Ok(())
    }
}

pub trait EffectCoordinator: Send + Sync {
    fn prepare(&self, effect: PreparedEffect) -> Result<PreparedEffect, String>;
    fn commit_batch(
        &self,
        effects: &[PreparedEffect],
        cancellation: &CancellationToken,
    ) -> Result<(), String>;
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

    fn commit_batch(
        &self,
        effects: &[PreparedEffect],
        cancellation: &CancellationToken,
    ) -> Result<(), String> {
        cancellation
            .check()
            .map_err(|_| "effect commit was cancelled".to_owned())?;
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
    #[error("expanded prompt failed compilation: {0}")]
    ExpansionCompile(PromptCompileError),
    #[error("native handle store failed: {0}")]
    HandleStore(String),
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub profile_id: ProfileId,
    pub prompt_id: PromptId,
    pub attempt_id: AttemptId,
    pub state: AttemptState,
    pub outputs: BTreeMap<NodeId, Vec<NativeValue>>,
    #[serde(default)]
    pub ui_outputs: BTreeMap<NodeId, Value>,
    pub events: Vec<AttemptEvent>,
    pub cache_hits: usize,
    pub error: Option<String>,
    #[serde(skip)]
    pub(crate) handle_lease: Option<NativeHandleLease>,
}

impl PartialEq for ExecutionReport {
    fn eq(&self, other: &Self) -> bool {
        self.profile_id == other.profile_id
            && self.prompt_id == other.prompt_id
            && self.attempt_id == other.attempt_id
            && self.state == other.state
            && self.outputs == other.outputs
            && self.ui_outputs == other.ui_outputs
            && self.events == other.events
            && self.cache_hits == other.cache_hits
            && self.error == other.error
    }
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
    handle_store_generation: NativeHandleStoreGeneration,
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
        Self::new_with_handle_store_generation(
            profile_id,
            nodes,
            cache,
            effects,
            registry_version,
            scratch,
            NativeHandleStoreGeneration::new()
                .map_err(|error| ExecutionError::HandleStore(error.to_string()))?,
        )
    }

    pub fn new_with_handle_store_generation(
        profile_id: ProfileId,
        nodes: Arc<NativeNodeRegistry>,
        cache: Arc<Mutex<NativeCache>>,
        effects: Arc<dyn EffectCoordinator>,
        registry_version: impl Into<String>,
        scratch: ScratchReservation,
        handle_store_generation: NativeHandleStoreGeneration,
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
            handle_store_generation,
        })
    }

    pub fn handle_store_generation(&self) -> &NativeHandleStoreGeneration {
        &self.handle_store_generation
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

    fn publish_cache_batch(
        &self,
        entries: Vec<(CacheKey, CacheEntry, Option<NativeHandleLease>)>,
        cancellation: &CancellationToken,
    ) -> Result<(), ExecutionError> {
        if cancellation.is_cancelled() {
            return Err(ExecutionError::Cancelled);
        }
        let mut cache = self.cache.lock();
        let prior_cache = cache.clone();
        if !cache.insert_batch_with_handle_leases(entries) {
            *cache = prior_cache;
            return Err(ExecutionError::Cache(
                "native cache handle leases did not cover the committed batch".to_owned(),
            ));
        }
        #[cfg(test)]
        self.handle_store_generation.run_after_cache_insert_hook();
        if cancellation.is_cancelled() {
            *cache = prior_cache;
            return Err(ExecutionError::Cancelled);
        }
        Ok(())
    }

    pub async fn execute(
        &self,
        plan: &CompiledPlan,
        attempt_id: AttemptId,
        cancellation: CancellationToken,
    ) -> ExecutionReport {
        let handle_store = self.handle_store_generation.session(attempt_id);
        let mut report_handle_lease = None;
        let mut state = RunState::new(
            self.profile_id,
            plan.prompt_id,
            attempt_id,
            cancellation,
            handle_store,
        );
        let result = async {
            state.emit(self.event_bus.as_ref(), None, AttemptEventKind::Started)?;
            self.run_plan(plan, &mut state, 0).await?;
            if state.cancellation.is_cancelled() {
                return Err(ExecutionError::Cancelled);
            }
            state.handle_store.commit();
            let pending_cache_entries = std::mem::take(&mut state.pending_cache_entries);
            let mut leased_cache_entries = Vec::with_capacity(pending_cache_entries.len());
            for (key, entry) in pending_cache_entries {
                let mut handles = Vec::new();
                for output in &entry.outputs {
                    collect_native_value_handles(output, &mut handles);
                }
                let lease = match self.handle_store_generation.acquire_lease(handles.iter()) {
                    Ok(lease) => lease,
                    Err(error) => {
                        drop(leased_cache_entries);
                        self.handle_store_generation
                            .collect_unrooted_attempt(attempt_id);
                        return Err(ExecutionError::HandleStore(error.to_string()));
                    }
                };
                leased_cache_entries.push((key, entry, lease));
            }
            let mut report_handles = Vec::new();
            for values in state.outputs.values() {
                for output in values {
                    collect_native_value_handles(output, &mut report_handles);
                }
            }
            let report_lease = match self
                .handle_store_generation
                .acquire_lease(report_handles.iter())
            {
                Ok(lease) => lease,
                Err(error) => {
                    drop(leased_cache_entries);
                    self.handle_store_generation
                        .collect_unrooted_attempt(attempt_id);
                    return Err(ExecutionError::HandleStore(error.to_string()));
                }
            };
            if state.cancellation.is_cancelled() {
                drop(report_lease);
                drop(leased_cache_entries);
                self.handle_store_generation
                    .collect_unrooted_attempt(attempt_id);
                return Err(ExecutionError::Cancelled);
            }
            if let Err(error) = self
                .effects
                .commit_batch(&state.prepared_effects, &state.cancellation)
            {
                drop(report_lease);
                drop(leased_cache_entries);
                self.handle_store_generation
                    .collect_unrooted_attempt(attempt_id);
                return Err(ExecutionError::Effect(error));
            }
            if state.cancellation.is_cancelled() {
                drop(report_lease);
                drop(leased_cache_entries);
                self.handle_store_generation
                    .collect_unrooted_attempt(attempt_id);
                return Err(ExecutionError::Cancelled);
            }
            if let Err(error) = self.publish_cache_batch(leased_cache_entries, &state.cancellation)
            {
                drop(report_lease);
                self.handle_store_generation
                    .collect_unrooted_attempt(attempt_id);
                return Err(error);
            }
            report_handle_lease = report_lease;
            self.handle_store_generation
                .collect_unrooted_attempt(attempt_id);
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
                handle_lease: report_handle_lease,
            },
            Err(error) => {
                state.handle_store.rollback_all();
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
                    handle_lease: None,
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
    ) -> BoxFuture<'a, Result<Vec<NativeValue>, ExecutionError>> {
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
    ) -> Result<Vec<NativeValue>, ExecutionError> {
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
        let context = NodeContext::new(
            plan.prompt_id,
            state.attempt_id,
            node_id.clone(),
            state.cancellation.clone(),
            self.scratch.clone(),
            state.handle_store.clone(),
        )
        .map_err(|error| ExecutionError::HandleStore(error.to_string()))?;
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
        if node.descriptor.cache == RuntimeCachePolicy::InputIdentity {
            let cached = self.cache.lock().get_with_handle_lease(&cache_key);
            if let Some((entry, cache_lease)) = cached {
                let mut handles = Vec::new();
                for output in &entry.outputs {
                    collect_native_value_handles(output, &mut handles);
                }
                let attempt_lease = if handles.is_empty() {
                    cache_lease.is_none().then_some(None)
                } else if cache_lease.as_ref().is_some_and(|lease| {
                    lease.store_identity() == self.handle_store_generation.identity()
                        && lease.covers_values(&entry.outputs)
                }) {
                    self.handle_store_generation
                        .acquire_lease(handles.iter())
                        .ok()
                } else {
                    None
                };
                if let Some(attempt_lease) = attempt_lease {
                    if let Some(lease) = attempt_lease {
                        state.cache_handle_leases.push(lease);
                    }
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
                self.cache.lock().remove(&cache_key);
            }
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
            state.pending_cache_entries.push((
                cache_key,
                CacheEntry {
                    outputs: outputs.clone(),
                    ui: ui.clone(),
                },
            ));
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
        inputs: BTreeMap<String, NativeValue>,
        state: &mut RunState,
        expansion_depth: usize,
    ) -> Result<(Vec<NativeValue>, Option<Value>, Vec<PreparedEffectRequest>), ExecutionError> {
        let mapped = inputs
            .iter()
            .filter_map(|(name, value)| {
                crate::prompt_compiler::resolve_input_descriptor(&node.descriptor, name)
                    .filter(|descriptor| descriptor.cardinality == InputMode::Mapped)
                    .and_then(|_| {
                        native_list_values(value).map(|values| (name.clone(), values.len()))
                    })
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
                    .map(|_| NativeValue::List { values: Vec::new() })
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
                let values = inputs
                    .get(name)
                    .and_then(native_list_values)
                    .ok_or_else(|| ExecutionError::InvalidLazyDemand {
                        node: node.id.clone(),
                        input: name.clone(),
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
                    let output_values = native_list_values(&output).ok_or_else(|| {
                        ExecutionError::InvalidOutput {
                            node: node.id.clone(),
                            output_index,
                        }
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
            collected
                .into_iter()
                .map(|values| NativeValue::List { values })
                .collect(),
            (!combined_ui.is_empty()).then_some(Value::Array(combined_ui)),
            effects,
        ))
    }

    async fn execute_once(
        &self,
        implementation: &dyn NativeNode,
        output_descriptors: &[RuntimeOutputDescriptor],
        context: NodeContext,
        inputs: BTreeMap<String, NativeValue>,
        state: &mut RunState,
        expansion_depth: usize,
    ) -> Result<(Vec<NativeValue>, Option<Value>, Vec<PreparedEffectRequest>), ExecutionError> {
        if state.cancellation.is_cancelled() {
            return Err(ExecutionError::Cancelled);
        }
        let node_id = context.node_id.clone();
        let handle_checkpoint = state.handle_store.checkpoint();
        let outcome = implementation.execute(context, inputs).await;
        if state.cancellation.is_cancelled() {
            state.handle_store.rollback_from(handle_checkpoint);
            return Err(ExecutionError::Cancelled);
        }
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(failure) => {
                state.handle_store.rollback_from(handle_checkpoint);
                return Err(execution_node_failure(node_id, failure));
            }
        };
        if let Err(error) = outcome.validate() {
            state.handle_store.rollback_from(handle_checkpoint);
            return Err(ExecutionError::HandleStore(error.to_string()));
        }
        let result = async {
            match outcome {
                NodeOutcome::Values {
                    outputs,
                    ui,
                    effects,
                } => Ok((outputs, ui, effects)),
                NodeOutcome::Blocked { reason } => Err(ExecutionError::Blocked {
                    node: node_id.clone(),
                    reason,
                }),
                NodeOutcome::Expansion {
                    prompt,
                    output_node,
                } => {
                    if expansion_depth >= MAX_EXPANSION_DEPTH {
                        return Err(ExecutionError::ExpansionDepth);
                    }
                    let plan = crate::PromptCompiler::new(&self.nodes)
                        .compile(PromptSubmission {
                            prompt,
                            prompt_id: Some(state.prompt_id),
                            client_id: None,
                            number: None,
                            extra_data: BTreeMap::new(),
                            unknown: BTreeMap::new(),
                        })
                        .map_err(ExecutionError::ExpansionCompile)?;
                    if !plan.nodes.contains_key(&output_node) {
                        return Err(ExecutionError::InvalidExpansionOutput(output_node));
                    }
                    let scope = state.next_expansion_scope;
                    let next_scope = scope
                        .checked_add(1)
                        .ok_or(ExecutionError::ExpansionSequenceExhausted)?;
                    let (plan, output_node) = namespace_expansion(
                        &plan,
                        &output_node,
                        &node_id,
                        state.prompt_id,
                        expansion_depth,
                        scope,
                    )?;
                    state.next_expansion_scope = next_scope;
                    self.run_node(&plan, &output_node, state, expansion_depth + 1)
                        .await
                        .map(|outputs| (outputs, None, Vec::new()))
                }
            }
        }
        .await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                state.handle_store.rollback_from(handle_checkpoint);
                return Err(error);
            }
        };
        if let Err(error) = validate_outputs(&node_id, output_descriptors, &result.0) {
            state.handle_store.rollback_from(handle_checkpoint);
            return Err(error);
        }
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
    inputs: &BTreeMap<String, NativeValue>,
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
    outputs: &[NativeValue],
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
            native_list_values(output).is_some_and(|values| {
                values
                    .iter()
                    .all(|value| native_output_type_accepts(&descriptor.produced_type, value))
            })
        } else {
            !matches!(output, NativeValue::List { .. })
                && native_output_type_accepts(&descriptor.produced_type, output)
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
    outputs: &[NativeValue],
) -> Result<NativeValue, ExecutionError> {
    let value =
        outputs
            .get(output_index)
            .cloned()
            .ok_or_else(|| ExecutionError::MissingOutput {
                node: source.clone(),
                output_index,
            })?;
    if mode == InputMode::List && !matches!(value, NativeValue::List { .. }) {
        Ok(NativeValue::List {
            values: vec![value],
        })
    } else {
        Ok(value)
    }
}

fn native_list_values(value: &NativeValue) -> Option<&[NativeValue]> {
    match value {
        NativeValue::List { values } => Some(values),
        _ => None,
    }
}

fn collect_native_value_handles(value: &NativeValue, handles: &mut Vec<NativeOpaqueHandle>) {
    match value {
        NativeValue::Handle { value } => handles.push(value.clone()),
        NativeValue::List { values } => {
            for value in values {
                collect_native_value_handles(value, handles);
            }
        }
        NativeValue::Primitive { .. } | NativeValue::PreservedUnknown { .. } => {}
    }
}

fn native_output_type_accepts(
    expected: &comfy_nodes::NativeValueType,
    value: &NativeValue,
) -> bool {
    match (expected, value) {
        (comfy_nodes::NativeValueType::Any, _) => true,
        (comfy_nodes::NativeValueType::Primitive(expected), NativeValue::Primitive { value }) => {
            *expected == value.primitive_type()
                || (*expected == comfy_nodes::NativePrimitiveType::Number
                    && value.primitive_type() == comfy_nodes::NativePrimitiveType::Integer)
        }
        (comfy_nodes::NativeValueType::Handle(expected), NativeValue::Handle { value }) => {
            expected == value.handle_type()
        }
        (comfy_nodes::NativeValueType::PreservedUnknown, NativeValue::PreservedUnknown { .. }) => {
            true
        }
        (
            comfy_nodes::NativeValueType::NamedPreservedUnknown(expected),
            NativeValue::PreservedUnknown { type_name, .. },
        ) => expected == type_name,
        _ => false,
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
    outputs: BTreeMap<NodeId, Vec<NativeValue>>,
    ui_outputs: BTreeMap<NodeId, Value>,
    cache_identities: BTreeMap<NodeId, String>,
    visiting: BTreeSet<NodeId>,
    prepared_effects: Vec<PreparedEffect>,
    events: Vec<AttemptEvent>,
    next_sequence: u64,
    next_expansion_scope: u64,
    cache_hits: usize,
    diagnostics: Vec<String>,
    handle_store: Arc<RuntimeNativeHandleStoreSession>,
    pending_cache_entries: Vec<(CacheKey, CacheEntry)>,
    cache_handle_leases: Vec<NativeHandleLease>,
}

impl RunState {
    fn new(
        profile_id: ProfileId,
        prompt_id: PromptId,
        attempt_id: AttemptId,
        cancellation: CancellationToken,
        handle_store: Arc<RuntimeNativeHandleStoreSession>,
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
            handle_store,
            pending_cache_entries: Vec::new(),
            cache_handle_leases: Vec::new(),
        }
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
    use crate::{InputMode, PromptCompiler, RuntimeNodeDescriptor, RuntimeOutputDescriptor};
    use comfy_nodes::{
        NATIVE_NODE_CONTRACT_SCHEMA_VERSION, NativeHandleKind, NativeInputDescriptor,
        NativePrimitive, NativePrimitiveType, NativeProviderPayload, NativeTypeUnion,
        NativeValueType,
    };
    use comfy_types::{ApiPrompt, PromptNode, PromptSubmission};
    use serde_json::json;
    use std::mem;
    use std::sync::{
        Barrier,
        atomic::{AtomicUsize, Ordering},
    };

    const TEST_WORKSPACE_BYTES: u64 = 64 * 1024 * 1024;

    #[derive(Clone)]
    enum ValueType {
        Any,
        Boolean,
        Integer,
        Number,
    }

    struct RuntimeInputDescriptor {
        value_type: ValueType,
        required: bool,
        hidden: bool,
        lazy: bool,
        mode: InputMode,
        allows_literal: bool,
    }

    fn native_integer(value: i64) -> NativeValue {
        NativeValue::Primitive {
            value: NativePrimitive::Integer(value),
        }
    }

    fn native_boolean(value: bool) -> NativeValue {
        NativeValue::Primitive {
            value: NativePrimitive::Boolean(value),
        }
    }

    fn native_string(value: &str) -> NativeValue {
        NativeValue::Primitive {
            value: NativePrimitive::String(value.to_owned()),
        }
    }

    fn native_null() -> NativeValue {
        NativeValue::Primitive {
            value: NativePrimitive::Null,
        }
    }

    fn stored_test_payload(
        abi_bytes: Vec<u8>,
    ) -> Result<NativeStoredPayload, comfy_nodes::NativeStoredPayloadError> {
        let semantic_digest_sha256 = format!("{:x}", Sha256::digest(&abi_bytes));
        Ok(NativeStoredPayload::Provider(Arc::new(
            NativeProviderPayload::checked(
                NativeHandleType::new(NativeHandleKind::ProviderTask, "TEST_PROVIDER_TASK")?,
                "sim.test.provider",
                semantic_digest_sha256,
                abi_bytes,
            )?,
        )))
    }

    fn test_cache_key(node_class: &str) -> Result<CacheKey, crate::NativeCacheError> {
        CacheKey::from_inputs(
            node_class,
            "1",
            &BTreeMap::new(),
            BTreeMap::new(),
            "cpu",
            "f32",
            None,
            None,
            "config-v1",
            "registry-v1",
            "stable",
        )
    }

    fn native_integer_value(value: &NativeValue) -> Option<i64> {
        match value {
            NativeValue::Primitive {
                value: NativePrimitive::Integer(value),
            } => Some(*value),
            _ => None,
        }
    }

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
            _available: &BTreeMap<String, NativeValue>,
        ) -> Result<BTreeSet<String>, NodeFailure> {
            self.observed.lock().push(context.scratch.bytes());
            Ok(BTreeSet::new())
        }

        fn execute<'a>(
            &'a self,
            context: NodeContext,
            _inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            self.observed.lock().push(context.scratch.bytes());
            Box::pin(async {
                Ok(NodeOutcome::Values {
                    outputs: vec![native_integer(1)],
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
            available: &BTreeMap<String, NativeValue>,
        ) -> Result<BTreeSet<String>, NodeFailure> {
            if self.class_type == "Choose"
                && available.get("condition") == Some(&native_boolean(true))
            {
                Ok(BTreeSet::from(["value".to_owned()]))
            } else {
                Ok(BTreeSet::new())
            }
        }

        fn execute<'a>(
            &'a self,
            context: NodeContext,
            inputs: BTreeMap<String, NativeValue>,
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
                    "Source" => vec![NativeValue::List {
                        values: vec![native_integer(1), native_integer(2), native_integer(3)],
                    }],
                    "LazySource" | "InnerOutput" => vec![native_integer(42)],
                    "Double" => vec![native_integer(
                        inputs
                            .get("value")
                            .and_then(native_integer_value)
                            .unwrap_or_default()
                            * 2,
                    )],
                    "Pair" => vec![native_integer(
                        inputs
                            .get("left")
                            .and_then(native_integer_value)
                            .unwrap_or_default()
                            * 10
                            + inputs
                                .get("right")
                                .and_then(native_integer_value)
                                .unwrap_or_default(),
                    )],
                    "ListMap" => vec![NativeValue::List {
                        values: vec![native_integer(
                            inputs
                                .get("value")
                                .and_then(native_integer_value)
                                .unwrap_or_default(),
                        )],
                    }],
                    "Choose" => vec![
                        inputs
                            .get("value")
                            .cloned()
                            .unwrap_or_else(|| native_integer(0)),
                    ],
                    "Output" => vec![inputs.get("value").cloned().unwrap_or_else(native_null)],
                    "Write" => {
                        return Ok(NodeOutcome::Values {
                            outputs: vec![native_string("prepared")],
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
                    _ => vec![native_null()],
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
        prompt: ApiPrompt,
    }

    struct PublishingMalformedExpansionNode;

    struct PublishingHandleNode {
        calls: Arc<AtomicUsize>,
        cancel_after_publish: bool,
    }

    impl NativeNode for PublishingHandleNode {
        fn class_type(&self) -> &str {
            "PublishingHandle"
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn execute<'a>(
            &'a self,
            context: NodeContext,
            _inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                let payload =
                    stored_test_payload(call.to_le_bytes().to_vec()).map_err(|error| {
                        NodeFailure {
                            code: "test_payload".to_owned(),
                            message: error.to_string(),
                            kind: NodeFailureKind::Failure,
                            retryable: false,
                        }
                    })?;
                let handle = context
                    .handle_store()
                    .publish(payload, &context.cancellation)
                    .map_err(|error| NodeFailure {
                        code: "publish".to_owned(),
                        message: error.to_string(),
                        kind: NodeFailureKind::Failure,
                        retryable: false,
                    })?;
                if self.cancel_after_publish {
                    context.cancellation.cancel();
                }
                Ok(NodeOutcome::Values {
                    outputs: vec![NativeValue::Handle { value: handle }],
                    ui: None,
                    effects: Vec::new(),
                })
            })
        }
    }

    impl NativeNode for PublishingMalformedExpansionNode {
        fn class_type(&self) -> &str {
            "PublishingMalformedExpansion"
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn execute<'a>(
            &'a self,
            context: NodeContext,
            _inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            Box::pin(async move {
                context
                    .handle_store()
                    .publish(
                        stored_test_payload(vec![9]).map_err(|error| NodeFailure {
                            code: "payload".to_owned(),
                            message: error.to_string(),
                            kind: NodeFailureKind::Failure,
                            retryable: false,
                        })?,
                        &context.cancellation,
                    )
                    .map_err(|error| NodeFailure {
                        code: "publish".to_owned(),
                        message: error.to_string(),
                        kind: NodeFailureKind::Failure,
                        retryable: false,
                    })?;
                Ok(NodeOutcome::Expansion {
                    prompt: ApiPrompt(BTreeMap::new()),
                    output_node: NodeId("missing".to_owned()),
                })
            })
        }
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
            _inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            let prompt = self.prompt.clone();
            Box::pin(async move {
                Ok(NodeOutcome::Expansion {
                    prompt,
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
            _inputs: BTreeMap<String, NativeValue>,
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
            _available_inputs: &BTreeMap<String, NativeValue>,
        ) -> Result<BTreeSet<String>, NodeFailure> {
            self.phases.lock().push("demand");
            context.cancellation.cancel();
            Ok(BTreeSet::new())
        }

        fn cache_change_token(
            &self,
            _inputs: &BTreeMap<String, NativeValue>,
        ) -> Result<String, NodeFailure> {
            self.phases.lock().push("change");
            Ok("stable".to_owned())
        }

        fn cache_dependencies(
            &self,
            _context: &NodeContext,
            _inputs: &BTreeMap<String, NativeValue>,
        ) -> Result<CacheDependencies, NodeFailure> {
            self.phases.lock().push("dependencies");
            Ok(CacheDependencies::default())
        }

        fn execute<'a>(
            &'a self,
            _context: NodeContext,
            _inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            self.phases.lock().push("execute");
            Box::pin(async move {
                Ok(NodeOutcome::Values {
                    outputs: vec![native_integer(1)],
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
            _inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Ok(NodeOutcome::Values {
                    outputs: vec![native_integer(1)],
                    ui: Some(json!({"preview": "ready"})),
                    effects: Vec::new(),
                })
            })
        }
    }

    struct VersionedSourceNode {
        version: &'static str,
    }

    struct ConfiguredNode {
        class_type: String,
        version: String,
        namespace: String,
    }

    impl NativeNode for ConfiguredNode {
        fn class_type(&self) -> &str {
            &self.class_type
        }

        fn implementation_version(&self) -> &str {
            &self.version
        }

        fn implementation_namespace(&self) -> &str {
            &self.namespace
        }

        fn execute<'a>(
            &'a self,
            _context: NodeContext,
            _inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            Box::pin(async {
                Ok(NodeOutcome::Values {
                    outputs: Vec::new(),
                    ui: None,
                    effects: Vec::new(),
                })
            })
        }
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
            _inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            Box::pin(async {
                Ok(NodeOutcome::Values {
                    outputs: vec![native_integer(7)],
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
            inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NodeOutcome, NodeFailure>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                Ok(NodeOutcome::Values {
                    outputs: vec![inputs.get("value").cloned().unwrap_or_else(native_null)],
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
    ) -> Result<RuntimeNodeDescriptor, NativeNodeContractError> {
        let inputs = inputs
            .into_iter()
            .map(|(name, input)| {
                Ok(NativeInputDescriptor {
                    name,
                    accepted_types: NativeTypeUnion::new([match input.value_type {
                        ValueType::Any => NativeValueType::Any,
                        ValueType::Boolean => {
                            NativeValueType::Primitive(NativePrimitiveType::Boolean)
                        }
                        ValueType::Integer => {
                            NativeValueType::Primitive(NativePrimitiveType::Integer)
                        }
                        ValueType::Number => {
                            NativeValueType::Primitive(NativePrimitiveType::Number)
                        }
                    }])?,
                    required: input.required,
                    hidden: input.hidden,
                    lazy: input.lazy,
                    cardinality: input.mode,
                    allows_literal: input.allows_literal,
                })
            })
            .collect::<Result<Vec<_>, NativeNodeContractError>>()?;
        let source_schema = comfy_nodes::NativeDescriptorSchemaMetadata::synthetic(
            inputs.iter().map(|input| input.name.clone()),
            std::iter::empty(),
            ["value".to_owned()],
        );
        Ok(RuntimeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: class_type.to_owned(),
            implementation_version: "1".to_owned(),
            source_schema: Some(source_schema),
            inputs,
            dynamic_inputs: Vec::new(),
            outputs: vec![RuntimeOutputDescriptor {
                name: "value".to_owned(),
                produced_type: if class_type == "Write" {
                    NativeValueType::Any
                } else {
                    NativeValueType::Primitive(NativePrimitiveType::Number)
                },
                is_list: class_type == "Source" || class_type == "Output",
            }],
            output_node,
            effect,
            cache: if effect == EffectClass::Pure {
                RuntimeCachePolicy::InputIdentity
            } else {
                RuntimeCachePolicy::Never
            },
        })
    }

    fn publishing_handle_descriptor() -> Result<RuntimeNodeDescriptor, NativeNodeContractError> {
        let mut descriptor =
            runtime_descriptor("PublishingHandle", true, BTreeMap::new(), EffectClass::Pure)?;
        descriptor.outputs = vec![RuntimeOutputDescriptor {
            name: "value".to_owned(),
            produced_type: NativeValueType::Any,
            is_list: false,
        }];
        Ok(descriptor)
    }

    fn report_handle(report: &ExecutionReport) -> Option<NativeOpaqueHandle> {
        report
            .outputs
            .values()
            .flat_map(|values| values.iter())
            .find_map(|value| match value {
                NativeValue::Handle { value } => Some(value.clone()),
                NativeValue::Primitive { .. }
                | NativeValue::List { .. }
                | NativeValue::PreservedUnknown { .. } => None,
            })
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
        let descriptor =
            runtime_descriptor("Component", false, BTreeMap::new(), EffectClass::Pure)?;
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
                    description: String::new(),
                    output_names: Vec::new(),
                    search_aliases: Vec::new(),
                    is_deprecated: false,
                    is_experimental: false,
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
            description: String::new(),
            output_names: vec!["value".to_owned()],
            search_aliases: Vec::new(),
            is_deprecated: false,
            is_experimental: false,
        };
        registry.register_bound_batch_with_presentations([(
            descriptor,
            node,
            presentation.clone(),
        )])?;
        assert!(registry.descriptor_is_bound("Component"));
        assert_eq!(registry.presentation("Component"), Some(&presentation));
        assert_eq!(
            registry.binding_source("Component"),
            Some("sim.native_rust")
        );
        registry.validate_comprehensive_bindings()?;
        Ok(())
    }

    #[test]
    fn native_handle_store_sessions_isolate_stage_commit_and_revoke()
    -> Result<(), Box<dyn std::error::Error>> {
        let payload = stored_test_payload(vec![7])?;
        let payload_bytes = payload.resident_bytes()?;
        let generation = NativeHandleStoreGeneration::with_capacities(4, payload_bytes * 4)?;
        let first_attempt = AttemptId(Uuid::from_u128(10));
        let second_attempt = AttemptId(Uuid::from_u128(11));
        let first = generation.session(first_attempt);
        let second = generation.session(second_attempt);
        let handle_type = payload.handle_type()?;
        let handle = first.publish(payload, &CancellationToken::default())?;

        assert!(matches!(
            second.resolve(&handle, &handle_type, &CancellationToken::default()),
            Err(NativeHandleStoreError::Missing(_))
        ));
        assert!(matches!(
            first
                .resolve(&handle, &handle_type, &CancellationToken::default())?
                .as_ref(),
            NativeStoredPayload::Provider(_)
        ));

        first.commit();
        assert!(matches!(
            first.publish(stored_test_payload(vec![8])?, &CancellationToken::default(),),
            Err(NativeHandleStoreError::Rejected(_))
        ));
        assert!(
            second
                .resolve(&handle, &handle_type, &CancellationToken::default())
                .is_ok()
        );
        assert!(matches!(
            second.revoke(&handle, &CancellationToken::default()),
            Err(NativeHandleStoreError::Rejected(_))
        ));

        let lease = generation
            .acquire_lease([&handle])?
            .ok_or("committed handle did not produce a lease")?;
        generation.collect_unrooted_attempt(first_attempt);
        assert_eq!(generation.len(), 1);
        drop(lease);
        assert!(generation.is_empty());
        Ok(())
    }

    #[test]
    fn native_handle_store_capacity_and_lease_validation_are_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        let first_payload = stored_test_payload(vec![1])?;
        let second_payload = stored_test_payload(vec![2])?;
        let oversized_payload = stored_test_payload(vec![3; 4_096])?;
        let maximum_bytes = first_payload
            .resident_bytes()?
            .checked_add(second_payload.resident_bytes()?)
            .ok_or("test capacity overflowed")?;
        let generation = NativeHandleStoreGeneration::with_capacities(2, maximum_bytes)?;
        let attempt_id = AttemptId(Uuid::from_u128(12));
        let session = generation.session(attempt_id);
        let handle_type = first_payload.handle_type()?;
        let first = session.publish(first_payload, &CancellationToken::default())?;
        assert!(matches!(
            session.publish(oversized_payload, &CancellationToken::default(),),
            Err(NativeHandleStoreError::Rejected(_))
        ));
        let second = session.publish(second_payload, &CancellationToken::default())?;
        assert_eq!(generation.len(), 2);
        assert_eq!(generation.resident_bytes(), maximum_bytes);
        session.commit();

        let forged = NativeOpaqueHandle::new(
            handle_type,
            generation.identity(),
            second.identifier(),
            second.generation(),
            Some("d".repeat(64)),
        )?;
        assert!(matches!(
            generation.acquire_lease([&first, &forged]),
            Err(NativeHandleStoreError::DigestMismatch)
        ));
        let data = generation.state.data.lock();
        assert!(data.values.values().all(|record| record.roots == 0));
        drop(data);
        generation.collect_unrooted_attempt(attempt_id);
        assert!(generation.is_empty());
        Ok(())
    }

    #[test]
    fn native_handle_store_accepts_zero_byte_payloads_and_never_wraps_generation()
    -> Result<(), Box<dyn std::error::Error>> {
        let (backend, workspace_authority) =
            comfy_tensor::CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let cancellation = CancellationToken::default();
        let context = backend.execution_context(
            comfy_tensor::StreamId::DEFAULT,
            workspace_authority.authorize_workspace(1024 * 1024)?,
            &cancellation,
        );
        let tensor = comfy_tensor::generated_native_diffusion::tensor_from_f32(
            &backend,
            &[0],
            &[],
            &context,
        )?;
        let payload =
            NativeStoredPayload::Tensor(Arc::new(comfy_tensor::NativeTensorPayload::from_tensor(
                comfy_tensor::NativeTensorRole::Sigmas,
                tensor,
            )?));
        assert_eq!(payload.resident_bytes()?, 0);
        let generation = NativeHandleStoreGeneration::with_capacities(2, 1)?;
        let session = generation.session(AttemptId(Uuid::from_u128(121)));
        let first = session.publish(payload.clone(), &cancellation)?;
        let second = session.publish(payload.clone(), &cancellation)?;
        assert_eq!(generation.resident_bytes(), 0);
        assert_eq!(generation.len(), 2);
        assert!(matches!(
            session.publish(payload.clone(), &cancellation),
            Err(NativeHandleStoreError::Rejected(message))
                if message.contains("capacity is exhausted")
        ));
        session.revoke(&first, &cancellation)?;
        session.revoke(&second, &cancellation)?;
        assert!(generation.is_empty());

        generation
            .state
            .next_generation
            .store(u64::MAX - 1, Ordering::Release);
        let final_handle = session.publish(payload.clone(), &cancellation)?;
        assert_eq!(final_handle.generation(), u64::MAX - 1);
        assert_eq!(final_handle.identifier(), "native-fffffffffffffffe");
        session.revoke(&final_handle, &cancellation)?;
        assert_eq!(
            generation.state.next_generation.load(Ordering::Acquire),
            u64::MAX
        );
        assert!(matches!(
            session.publish(payload, &cancellation),
            Err(NativeHandleStoreError::Rejected(message))
                if message.contains("generation was exhausted")
        ));
        assert!(generation.is_empty());
        assert_eq!(generation.resident_bytes(), 0);
        Ok(())
    }

    #[test]
    fn native_handle_publish_cancellation_is_atomic_before_validation_and_after_insert()
    -> Result<(), Box<dyn std::error::Error>> {
        let payload = stored_test_payload(vec![1, 2, 3])?;
        let generation =
            NativeHandleStoreGeneration::with_capacities(2, payload.resident_bytes()?)?;
        let session = generation.session(AttemptId(Uuid::from_u128(124)));

        let pre_validation = CancellationToken::default();
        pre_validation.cancel();
        assert!(matches!(
            session.publish(payload.clone(), &pre_validation),
            Err(NativeHandleStoreError::Cancelled)
        ));
        assert!(generation.is_empty());
        assert_eq!(generation.state.next_generation.load(Ordering::Acquire), 1);

        let post_insert = CancellationToken::default();
        generation.set_after_publish_insert_hook(Arc::new({
            let post_insert = post_insert.clone();
            move || {
                post_insert.cancel();
            }
        }));
        assert!(matches!(
            session.publish(payload, &post_insert),
            Err(NativeHandleStoreError::Cancelled)
        ));
        assert!(generation.is_empty());
        assert_eq!(generation.resident_bytes(), 0);
        assert_eq!(generation.state.next_generation.load(Ordering::Acquire), 2);
        Ok(())
    }

    #[test]
    fn native_handle_publish_racing_session_close_never_leaks_staged_values()
    -> Result<(), Box<dyn std::error::Error>> {
        for rollback in [false, true] {
            for iteration in 0..64_u128 {
                let payload = stored_test_payload(vec![u8::try_from(iteration)?])?;
                let generation =
                    NativeHandleStoreGeneration::with_capacities(1, payload.resident_bytes()?)?;
                let session = generation.session(AttemptId(Uuid::from_u128(200 + iteration)));
                let barrier = Arc::new(Barrier::new(2));
                let publisher = std::thread::spawn({
                    let barrier = barrier.clone();
                    let session = session.clone();
                    move || {
                        barrier.wait();
                        session.publish(payload, &CancellationToken::default())
                    }
                });
                let closer = std::thread::spawn({
                    let barrier = barrier.clone();
                    let session = session.clone();
                    move || {
                        barrier.wait();
                        if rollback {
                            session.rollback_all();
                        } else {
                            session.commit();
                        }
                    }
                });
                let published = publisher.join().map_err(|_| "publisher thread panicked")?;
                closer.join().map_err(|_| "closer thread panicked")?;

                if rollback {
                    assert!(generation.is_empty());
                    assert!(matches!(
                        published,
                        Ok(_) | Err(NativeHandleStoreError::Rejected(_))
                    ));
                } else {
                    match published {
                        Ok(handle) => {
                            let reader = generation.session(AttemptId(Uuid::from_u128(10_000)));
                            assert!(
                                reader
                                    .resolve(
                                        &handle,
                                        handle.handle_type(),
                                        &CancellationToken::default(),
                                    )
                                    .is_ok()
                            );
                            let lease = generation
                                .acquire_lease([&handle])?
                                .ok_or("committed handle lease was missing")?;
                            drop(lease);
                            assert!(generation.is_empty());
                        }
                        Err(NativeHandleStoreError::Rejected(_)) => {
                            assert!(generation.is_empty());
                        }
                        Err(error) => return Err(error.into()),
                    }
                }
                assert_eq!(generation.resident_bytes(), 0);
            }
        }
        Ok(())
    }

    #[test]
    fn native_handle_resolve_rejects_forged_store_generation_type_and_digest()
    -> Result<(), Box<dyn std::error::Error>> {
        let payload = stored_test_payload(vec![9])?;
        let generation =
            NativeHandleStoreGeneration::with_capacities(1, payload.resident_bytes()?)?;
        let attempt_id = AttemptId(Uuid::from_u128(122));
        let session = generation.session(attempt_id);
        let handle = session.publish(payload, &CancellationToken::default())?;
        let expected = handle.handle_type().clone();
        let other = NativeHandleStoreGeneration::new()?;
        let wrong_store = NativeOpaqueHandle::new(
            expected.clone(),
            other.identity(),
            handle.identifier(),
            handle.generation(),
            handle.digest_sha256().map(ToOwned::to_owned),
        )?;
        let wrong_generation = NativeOpaqueHandle::new(
            expected.clone(),
            NativeHandleStoreIdentity::new(generation.identity().store_id, Uuid::from_u128(123))?,
            handle.identifier(),
            handle.generation(),
            handle.digest_sha256().map(ToOwned::to_owned),
        )?;
        let wrong_type = NativeHandleType::new(NativeHandleKind::Model, "MODEL")?;
        let wrong_digest = NativeOpaqueHandle::new(
            expected.clone(),
            generation.identity(),
            handle.identifier(),
            handle.generation(),
            Some("f".repeat(64)),
        )?;
        assert!(matches!(
            session.resolve(&wrong_store, &expected, &CancellationToken::default()),
            Err(NativeHandleStoreError::WrongStore)
        ));
        assert!(matches!(
            session.resolve(&wrong_generation, &expected, &CancellationToken::default()),
            Err(NativeHandleStoreError::WrongGeneration)
        ));
        assert!(matches!(
            session.resolve(&handle, &wrong_type, &CancellationToken::default()),
            Err(NativeHandleStoreError::WrongType { .. })
        ));
        assert!(matches!(
            session.resolve(&wrong_digest, &expected, &CancellationToken::default()),
            Err(NativeHandleStoreError::DigestMismatch)
        ));
        assert_eq!(generation.len(), 1);
        Ok(())
    }

    #[test]
    fn native_cache_leases_cover_aliases_and_release_on_lru_and_invalidation()
    -> Result<(), Box<dyn std::error::Error>> {
        let first_payload = stored_test_payload(vec![1])?;
        let second_payload = stored_test_payload(vec![2])?;
        let maximum_bytes = first_payload
            .resident_bytes()?
            .checked_add(second_payload.resident_bytes()?)
            .and_then(|bytes| bytes.checked_mul(2))
            .ok_or("test capacity overflowed")?;
        let generation = NativeHandleStoreGeneration::with_capacities(4, maximum_bytes)?;
        let attempt_id = AttemptId(Uuid::from_u128(13));
        let session = generation.session(attempt_id);
        let first = session.publish(first_payload, &CancellationToken::default())?;
        let second = session.publish(second_payload, &CancellationToken::default())?;
        session.commit();
        let first_entry = CacheEntry {
            outputs: vec![
                NativeValue::Handle {
                    value: first.clone(),
                },
                NativeValue::List {
                    values: vec![
                        NativeValue::Handle {
                            value: first.clone(),
                        },
                        NativeValue::List {
                            values: vec![NativeValue::Handle {
                                value: first.clone(),
                            }],
                        },
                    ],
                },
            ],
            ui: None,
        };
        let second_entry = CacheEntry {
            outputs: vec![NativeValue::Handle {
                value: second.clone(),
            }],
            ui: None,
        };
        let first_key = CacheKey::from_inputs(
            "First",
            "1",
            &BTreeMap::new(),
            BTreeMap::new(),
            "cpu",
            "f32",
            None,
            None,
            "config-v1",
            "registry-v1",
            "stable",
        )?;
        let second_key = CacheKey::from_inputs(
            "Second",
            "1",
            &BTreeMap::new(),
            BTreeMap::new(),
            "cpu",
            "f32",
            None,
            None,
            "config-v1",
            "registry-v1",
            "stable",
        )?;
        let mut cache = NativeCache::new(1)?;
        assert!(!cache.insert(first_key.clone(), first_entry.clone()));
        assert!(cache.is_empty());
        assert!(cache.insert_with_handle_lease(
            first_key,
            first_entry,
            generation.acquire_lease([&first])?,
        ));
        assert!(cache.insert_with_handle_lease(
            second_key,
            second_entry,
            generation.acquire_lease([&second])?,
        ));
        assert_eq!(generation.len(), 1);
        assert_eq!(cache.invalidate_node("Second"), 1);
        assert!(generation.is_empty());
        Ok(())
    }

    #[test]
    fn native_cache_same_key_replacement_transfers_exact_handle_roots()
    -> Result<(), Box<dyn std::error::Error>> {
        let first_payload = stored_test_payload(vec![1])?;
        let second_payload = stored_test_payload(vec![2])?;
        let byte_capacity = first_payload
            .resident_bytes()?
            .checked_add(second_payload.resident_bytes()?)
            .ok_or("test byte capacity overflowed")?;
        let generation = NativeHandleStoreGeneration::with_capacities(2, byte_capacity)?;
        let session = generation.session(AttemptId(Uuid::from_u128(310)));
        let first = session.publish(first_payload, &CancellationToken::default())?;
        let second = session.publish(second_payload, &CancellationToken::default())?;
        session.commit();

        let key = test_cache_key("Replacement")?;
        let first_entry = CacheEntry {
            outputs: vec![NativeValue::Handle {
                value: first.clone(),
            }],
            ui: None,
        };
        let second_entry = CacheEntry {
            outputs: vec![NativeValue::Handle {
                value: second.clone(),
            }],
            ui: None,
        };
        let mut cache = NativeCache::new(1)?;
        assert!(cache.insert_with_handle_lease(
            key.clone(),
            first_entry.clone(),
            generation.acquire_lease([&first])?,
        ));
        assert!(cache.insert_with_handle_lease(
            key.clone(),
            first_entry,
            generation.acquire_lease([&first])?,
        ));
        assert_eq!(generation.len(), 2);
        assert!(cache.insert_with_handle_lease(
            key,
            second_entry,
            generation.acquire_lease([&second])?,
        ));
        assert_eq!(generation.len(), 1);
        assert!(matches!(
            session.resolve(&first, first.handle_type(), &CancellationToken::default(),),
            Err(NativeHandleStoreError::Missing(_))
        ));
        assert!(
            session
                .resolve(&second, second.handle_type(), &CancellationToken::default(),)
                .is_ok()
        );
        cache.clear();
        assert!(generation.is_empty());
        Ok(())
    }

    #[test]
    fn cancelled_cache_publication_restores_replaced_entry_and_exact_roots()
    -> Result<(), Box<dyn std::error::Error>> {
        let first_payload = stored_test_payload(vec![1])?;
        let second_payload = stored_test_payload(vec![2])?;
        let byte_capacity = first_payload
            .resident_bytes()?
            .checked_add(second_payload.resident_bytes()?)
            .ok_or("test byte capacity overflowed")?;
        let generation = NativeHandleStoreGeneration::with_capacities(2, byte_capacity)?;
        let session = generation.session(AttemptId(Uuid::from_u128(320)));
        let first = session.publish(first_payload, &CancellationToken::default())?;
        let second = session.publish(second_payload, &CancellationToken::default())?;
        session.commit();
        let key = test_cache_key("AtomicReplacement")?;
        let first_entry = CacheEntry {
            outputs: vec![NativeValue::Handle {
                value: first.clone(),
            }],
            ui: None,
        };
        let second_entry = CacheEntry {
            outputs: vec![NativeValue::Handle {
                value: second.clone(),
            }],
            ui: None,
        };
        let cache = Arc::new(Mutex::new(NativeCache::new(1)?));
        assert!(cache.lock().insert_with_handle_lease(
            key.clone(),
            first_entry.clone(),
            generation.acquire_lease([&first])?,
        ));
        let (_backend, workspace_authority) =
            CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
        let engine = ExecutionEngine::new_with_handle_store_generation(
            ProfileId(Uuid::from_u128(321)),
            Arc::new(NativeNodeRegistry::default()),
            cache.clone(),
            Arc::new(RecordingEffectCoordinator::default()),
            "registry-v1",
            workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
            generation.clone(),
        )?;
        let pre_publication = CancellationToken::default();
        pre_publication.cancel();
        assert!(matches!(
            engine.publish_cache_batch(Vec::new(), &pre_publication),
            Err(ExecutionError::Cancelled)
        ));
        assert_eq!(cache.lock().get(&key), Some(first_entry.clone()));
        assert_eq!(generation.len(), 2);
        let cancellation = CancellationToken::default();
        generation.set_after_cache_insert_hook(Arc::new({
            let cancellation = cancellation.clone();
            move || {
                cancellation.cancel();
            }
        }));
        assert!(matches!(
            engine.publish_cache_batch(
                vec![(
                    key.clone(),
                    second_entry,
                    generation.acquire_lease([&second])?,
                )],
                &cancellation,
            ),
            Err(ExecutionError::Cancelled)
        ));
        assert_eq!(cache.lock().get(&key), Some(first_entry));
        assert_eq!(generation.len(), 1);
        assert!(
            session
                .resolve(&first, first.handle_type(), &CancellationToken::default(),)
                .is_ok()
        );
        assert!(matches!(
            session.resolve(&second, second.handle_type(), &CancellationToken::default(),),
            Err(NativeHandleStoreError::Missing(_))
        ));
        cache.lock().clear();
        assert!(generation.is_empty());
        Ok(())
    }

    #[test]
    fn cloned_native_handle_lease_releases_roots_once_under_concurrent_drop()
    -> Result<(), Box<dyn std::error::Error>> {
        let payload = stored_test_payload(vec![7])?;
        let byte_capacity = payload.resident_bytes()?;
        let generation = NativeHandleStoreGeneration::with_capacities(1, byte_capacity)?;
        let attempt_id = AttemptId(Uuid::from_u128(14));
        let session = generation.session(attempt_id);
        let handle = session.publish(payload, &CancellationToken::default())?;
        session.commit();
        let lease = generation
            .acquire_lease([&handle])?
            .ok_or("test lease was not acquired")?;
        let second = lease.clone();
        let barrier = Arc::new(Barrier::new(3));
        std::thread::scope(|scope| {
            for lease in [lease, second] {
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    drop(lease);
                });
            }
            barrier.wait();
        });
        assert!(generation.is_empty());
        assert_eq!(generation.resident_bytes(), 0);
        Ok(())
    }

    #[test]
    fn provider_activation_is_checked_atomic_and_preserves_declared_provenance()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = NodeRegistry::built_in()?;
        let catalog_descriptor = catalog
            .registered()
            .values()
            .find(|descriptor| {
                descriptor.catalog_status == comfy_nodes::CatalogNodeStatus::ProviderRequired
            })
            .ok_or("generated catalog did not include a provider-required binding")?;
        let presentation = RuntimeNodePresentation {
            display_name: catalog_descriptor.display_name.clone(),
            category: match catalog_descriptor.category.as_str() {
                "(empty root category declared by source)" => String::new(),
                category => category.to_owned(),
            },
            description: String::new(),
            output_names: vec!["value".to_owned()],
            search_aliases: Vec::new(),
            is_deprecated: false,
            is_experimental: false,
        };
        let descriptor = RuntimeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: catalog_descriptor.node_identifier.clone(),
            implementation_version: "provider-v1".to_owned(),
            source_schema: Some(comfy_nodes::NativeDescriptorSchemaMetadata::synthetic(
                std::iter::empty(),
                std::iter::empty(),
                ["value".to_owned()],
            )),
            inputs: Vec::new(),
            dynamic_inputs: Vec::new(),
            outputs: vec![RuntimeOutputDescriptor {
                name: "value".to_owned(),
                produced_type: NativeValueType::Any,
                is_list: false,
            }],
            output_node: catalog_descriptor.output_node,
            effect: EffectClass::Provider,
            cache: RuntimeCachePolicy::Never,
        };
        let binding = NativeNodeBinding::ProviderRequired {
            feature_id: catalog_descriptor.feature_id.clone(),
            descriptor,
            presentation,
            provider: "sim.provider.test".to_owned(),
            reason: "verified provider activation is required".to_owned(),
        };
        let NativeNodeBinding::ProviderRequired {
            descriptor,
            presentation,
            provider,
            ..
        } = binding.clone()
        else {
            return Err("selected binding was not provider-required".into());
        };
        let node: Arc<dyn NativeNode> = Arc::new(ConfiguredNode {
            class_type: descriptor.class_type.clone(),
            version: descriptor.implementation_version.clone(),
            namespace: provider,
        });
        let mut registry = NativeNodeRegistry::default();
        registry.register_native_bindings([binding])?;
        registry.validate_comprehensive_bindings()?;

        let mut mismatched_presentation = presentation.clone();
        mismatched_presentation.display_name.push_str(" mismatch");
        assert!(matches!(
            registry.activate_provider_bindings([(node.clone(), mismatched_presentation)]),
            Err(NativeNodeRegistryError::BindingMismatch(class_type))
                if class_type == descriptor.class_type
        ));
        assert_eq!(
            registry.provider_binding_is_activated(&descriptor.class_type),
            Some(false)
        );
        assert!(registry.node(&descriptor.class_type).is_none());

        registry.activate_provider_bindings([(node, presentation)])?;
        assert_eq!(
            registry.binding_declared_disposition(&descriptor.class_type),
            Some(NativeNodeBindingDisposition::ProviderRequired)
        );
        assert_eq!(
            registry.binding_disposition(&descriptor.class_type),
            Some(NativeNodeBindingDisposition::Executable)
        );
        assert_eq!(
            registry.provider_binding_is_activated(&descriptor.class_type),
            Some(true)
        );
        registry.validate_comprehensive_bindings()?;
        Ok(())
    }

    #[test]
    fn malformed_expansion_rolls_back_published_handles_and_cache_state()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let descriptor = runtime_descriptor(
                "PublishingMalformedExpansion",
                true,
                BTreeMap::new(),
                EffectClass::Pure,
            )?;
            let plan = compile_plan(
                vec![descriptor.clone()],
                BTreeMap::from([(
                    NodeId("expand".to_owned()),
                    PromptNode {
                        class_type: descriptor.class_type.clone(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let mut registry = NativeNodeRegistry::default();
            registry.register_descriptor(descriptor)?;
            registry.register(Arc::new(PublishingMalformedExpansionNode))?;
            let cache = Arc::new(Mutex::new(NativeCache::new(4)?));
            let effects = Arc::new(RecordingEffectCoordinator::default());
            let payload_bytes = stored_test_payload(vec![9])?.resident_bytes()?;
            let generation = NativeHandleStoreGeneration::with_capacities(4, payload_bytes * 4)?;
            let (_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
            let engine = ExecutionEngine::new_with_handle_store_generation(
                ProfileId(Uuid::from_u128(41)),
                Arc::new(registry),
                cache.clone(),
                effects.clone(),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
                generation.clone(),
            )?;
            let report = engine
                .execute(
                    &plan,
                    AttemptId(Uuid::from_u128(42)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(report.state, AttemptState::Failed);
            assert!(report.outputs.is_empty());
            assert!(report.ui_outputs.is_empty());
            assert!(generation.is_empty());
            assert!(cache.lock().is_empty());
            assert!(effects.committed().is_empty());
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn cancellation_before_store_commit_rolls_back_handles_and_cache()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let descriptor = publishing_handle_descriptor()?;
            let plan = compile_plan(
                vec![descriptor.clone()],
                BTreeMap::from([(
                    NodeId("publish".to_owned()),
                    PromptNode {
                        class_type: descriptor.class_type.clone(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let calls = Arc::new(AtomicUsize::new(0));
            let mut registry = NativeNodeRegistry::default();
            registry.register_descriptor(descriptor)?;
            registry.register(Arc::new(PublishingHandleNode {
                calls: calls.clone(),
                cancel_after_publish: true,
            }))?;
            let cache = Arc::new(Mutex::new(NativeCache::new(2)?));
            let payload_bytes =
                stored_test_payload(vec![0; mem::size_of::<usize>()])?.resident_bytes()?;
            let generation = NativeHandleStoreGeneration::with_capacities(2, payload_bytes * 2)?;
            let (_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
            let engine = ExecutionEngine::new_with_handle_store_generation(
                ProfileId(Uuid::from_u128(401)),
                Arc::new(registry),
                cache.clone(),
                Arc::new(RecordingEffectCoordinator::default()),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
                generation.clone(),
            )?;
            let report = engine
                .execute(
                    &plan,
                    AttemptId(Uuid::from_u128(402)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(report.state, AttemptState::Cancelled);
            assert!(report.outputs.is_empty());
            assert!(generation.is_empty());
            assert_eq!(generation.resident_bytes(), 0);
            assert!(cache.lock().is_empty());
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn cache_and_report_hold_independent_handle_leases() -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let descriptor = publishing_handle_descriptor()?;
            let plan = compile_plan(
                vec![descriptor.clone()],
                BTreeMap::from([(
                    NodeId("publish".to_owned()),
                    PromptNode {
                        class_type: descriptor.class_type.clone(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let calls = Arc::new(AtomicUsize::new(0));
            let mut registry = NativeNodeRegistry::default();
            registry.register_descriptor(descriptor)?;
            registry.register(Arc::new(PublishingHandleNode {
                calls,
                cancel_after_publish: false,
            }))?;
            let cache = Arc::new(Mutex::new(NativeCache::new(2)?));
            let payload_bytes =
                stored_test_payload(vec![0; mem::size_of::<usize>()])?.resident_bytes()?;
            let generation = NativeHandleStoreGeneration::with_capacities(2, payload_bytes * 2)?;
            let (_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES)?;
            let engine = ExecutionEngine::new_with_handle_store_generation(
                ProfileId(Uuid::from_u128(411)),
                Arc::new(registry),
                cache.clone(),
                Arc::new(RecordingEffectCoordinator::default()),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
                generation.clone(),
            )?;
            let report = engine
                .execute(
                    &plan,
                    AttemptId(Uuid::from_u128(412)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(report.state, AttemptState::Succeeded);
            let handle = report_handle(&report).ok_or("report handle was missing")?;
            assert_eq!(generation.len(), 1);
            assert_eq!(cache.lock().invalidate_node("PublishingHandle"), 1);
            assert_eq!(generation.len(), 1);
            assert!(
                generation
                    .session(AttemptId(Uuid::from_u128(413)))
                    .resolve(&handle, handle.handle_type(), &CancellationToken::default(),)
                    .is_ok()
            );
            drop(report);
            assert!(generation.is_empty());
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn stale_store_generation_cache_entry_is_evicted_and_recomputed()
    -> Result<(), Box<dyn std::error::Error>> {
        smol::block_on(async {
            let descriptor = publishing_handle_descriptor()?;
            let plan = compile_plan(
                vec![descriptor.clone()],
                BTreeMap::from([(
                    NodeId("publish".to_owned()),
                    PromptNode {
                        class_type: descriptor.class_type.clone(),
                        inputs: BTreeMap::new(),
                        unknown: BTreeMap::new(),
                    },
                )]),
            )?;
            let calls = Arc::new(AtomicUsize::new(0));
            let mut registry = NativeNodeRegistry::default();
            registry.register_descriptor(descriptor)?;
            registry.register(Arc::new(PublishingHandleNode {
                calls: calls.clone(),
                cancel_after_publish: false,
            }))?;
            let registry = Arc::new(registry);
            let cache = Arc::new(Mutex::new(NativeCache::new(2)?));
            let payload_bytes =
                stored_test_payload(vec![0; mem::size_of::<usize>()])?.resident_bytes()?;
            let first_generation =
                NativeHandleStoreGeneration::with_capacities(2, payload_bytes * 2)?;
            let second_generation =
                NativeHandleStoreGeneration::with_capacities(2, payload_bytes * 2)?;
            let (_backend, workspace_authority) =
                CpuWorkspaceAuthority::create_backend(TEST_WORKSPACE_BYTES * 2)?;
            let first_engine = ExecutionEngine::new_with_handle_store_generation(
                ProfileId(Uuid::from_u128(421)),
                registry.clone(),
                cache.clone(),
                Arc::new(RecordingEffectCoordinator::default()),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
                first_generation.clone(),
            )?;
            let first_report = first_engine
                .execute(
                    &plan,
                    AttemptId(Uuid::from_u128(422)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(first_report.state, AttemptState::Succeeded);
            drop(first_report);
            assert_eq!(first_generation.len(), 1);

            let second_engine = ExecutionEngine::new_with_handle_store_generation(
                ProfileId(Uuid::from_u128(423)),
                registry,
                cache.clone(),
                Arc::new(RecordingEffectCoordinator::default()),
                "registry-v1",
                workspace_authority.authorize_workspace(TEST_WORKSPACE_BYTES)?,
                second_generation.clone(),
            )?;
            let second_report = second_engine
                .execute(
                    &plan,
                    AttemptId(Uuid::from_u128(424)),
                    CancellationToken::default(),
                )
                .await;
            assert_eq!(second_report.state, AttemptState::Succeeded);
            assert_eq!(second_report.cache_hits, 0);
            assert_eq!(calls.load(Ordering::SeqCst), 2);
            assert!(first_generation.is_empty());
            assert_eq!(second_generation.len(), 1);
            drop(second_report);
            assert_eq!(second_generation.len(), 1);
            cache.lock().clear();
            assert!(second_generation.is_empty());
            Ok::<(), Box<dyn std::error::Error>>(())
        })
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
            )?;
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
                runtime_descriptor("Source", false, BTreeMap::new(), EffectClass::Pure)?,
                runtime_descriptor(
                    "Double",
                    false,
                    BTreeMap::from([(
                        "value".to_owned(),
                        input(ValueType::Number, false, InputMode::Mapped, false),
                    )]),
                    EffectClass::Pure,
                )?,
                runtime_descriptor(
                    "Output",
                    true,
                    BTreeMap::from([(
                        "value".to_owned(),
                        input(ValueType::Number, false, InputMode::List, false),
                    )]),
                    EffectClass::Pure,
                )?,
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
            assert_eq!(
                first.outputs[&NodeId::from("output")],
                [NativeValue::List {
                    values: vec![native_integer(2), native_integer(4), native_integer(6)],
                }]
            );
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
            )?;
            let mut list_map = runtime_descriptor(
                "ListMap",
                true,
                BTreeMap::from([(
                    "value".to_owned(),
                    input(ValueType::Integer, false, InputMode::Mapped, true),
                )]),
                EffectClass::Pure,
            )?;
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
            assert_eq!(
                report.outputs[&NodeId::from("pair")],
                [NativeValue::List {
                    values: vec![native_integer(14), native_integer(25), native_integer(35)],
                }]
            );
            assert_eq!(
                report.outputs[&NodeId::from("list")],
                [NativeValue::List {
                    values: vec![native_integer(1), native_integer(2)],
                }]
            );
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
                )?],
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
                )?],
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
                )?],
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
                )?;
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
                        )?,
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
                assert_eq!(report.outputs[&NodeId::from("output")], [native_integer(7)]);
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
                )?],
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
                runtime_descriptor("LazySource", false, BTreeMap::new(), EffectClass::Pure)?,
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
                )?,
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
            assert_eq!(
                demanded.outputs[&NodeId::from("choose")],
                [native_integer(42)]
            );
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
            let inner = ApiPrompt(BTreeMap::from([(
                NodeId::from("inner"),
                PromptNode {
                    class_type: "InnerOutput".to_owned(),
                    inputs: BTreeMap::new(),
                    unknown: BTreeMap::new(),
                },
            )]));
            let outer = compile_plan(
                vec![runtime_descriptor(
                    "Expand",
                    true,
                    BTreeMap::from([(
                        "value".to_owned(),
                        input(ValueType::Integer, false, InputMode::Mapped, true),
                    )]),
                    EffectClass::Pure,
                )?],
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
            expansion_registry.register_descriptor(runtime_descriptor(
                "InnerOutput",
                true,
                BTreeMap::new(),
                EffectClass::Pure,
            )?)?;
            expansion_registry.register(Arc::new(ExpansionNode { prompt: inner }))?;
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
            assert_eq!(
                expanded.outputs[&NodeId::from("expand")],
                [NativeValue::List {
                    values: vec![native_integer(42), native_integer(42)],
                }]
            );
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
                )?],
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
                    )?,
                    runtime_descriptor(
                        "Block",
                        true,
                        BTreeMap::from([(
                            "value".to_owned(),
                            input(ValueType::Any, false, InputMode::Scalar, false),
                        )]),
                        EffectClass::Pure,
                    )?,
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
