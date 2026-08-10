use crate::{
    ComponentLimits, InvocationInputs, InvocationResult, PluginCapabilityServices, PluginError,
    PluginHost,
};
use comfy_plugin_sdk::PluginManifest;
use comfy_runtime::{
    NativeNodeRegistry, NodeContext, PermissionPolicy, PluginAuthorization,
    PluginAuthorizationSealer, PluginAuthorizationVerifier, PluginTrustPolicy,
    WorkerRegistryDeploymentPlan,
};
use comfy_types::{
    CancellationToken, MAX_WORKER_COMPONENT_CHUNK_BYTES,
    MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES, MAX_WORKER_PLUGIN_INVOCATION_BYTES,
    WorkerComponentContent, WorkerComponentDescriptor, WorkerRegistryDeploymentBegin,
    WorkerRegistryDeploymentChunk, WorkerRegistryGeneration, WorkerSha256Digest,
};
use extension_host::{ComponentLifecycleAdapter, ComponentRuntime, InstalledComponent};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    sync::{Arc, Condvar, Mutex, RwLock},
};
use thiserror::Error;

pub const COMFY_COMPONENT_ADAPTER_ID: &str = "sim.comfy.component-host.v1";
pub const MAX_WORKER_PLUGIN_TIMEOUT_MILLISECONDS: u64 = 60_000;

#[derive(Debug, Error)]
pub enum ComponentHostError {
    #[error("component host state is unavailable")]
    StateUnavailable,
    #[error("component manifest for extension `{extension_id}` is invalid: {message}")]
    InvalidManifest {
        extension_id: Arc<str>,
        message: String,
    },
    #[error("component verification for extension `{extension_id}` failed: {message}")]
    Verification {
        extension_id: Arc<str>,
        message: String,
    },
    #[error("extension `{0}` repeated a component identity")]
    DuplicatePlugin(String),
    #[error("component node `{0}` is owned by more than one installed component")]
    DuplicateNode(String),
    #[error("extension `{0}` has no active verified component")]
    MissingExtension(String),
    #[error("component handle for extension `{0}` was revoked")]
    Revoked(String),
    #[error("plugin execution boundary failed: {0}")]
    ExecutionBoundary(String),
    #[error(
        "installed extension `{extension_id}` version `{extension_version}` does not match signed plugin version `{plugin_version}`"
    )]
    ExtensionVersionMismatch {
        extension_id: Arc<str>,
        extension_version: Arc<str>,
        plugin_version: String,
    },
    #[error(transparent)]
    Plugin(#[from] PluginError),
}

pub trait PluginInvocationExecutor: Send + Sync {
    fn execute<'a>(
        &'a self,
        invocation: PreparedPluginInvocation,
    ) -> Pin<Box<dyn Future<Output = Result<InvocationResult, ComponentHostError>> + Send + 'a>>;
}

pub enum ComponentExecutionBoundary {
    PrivateWorker(Arc<dyn PluginInvocationExecutor>),
    ConformanceInProcess(Arc<dyn PluginCapabilityServices>),
}

impl ComponentExecutionBoundary {
    pub fn private_worker(executor: Arc<dyn PluginInvocationExecutor>) -> Self {
        Self::PrivateWorker(executor)
    }

    pub fn conformance_in_process(services: Arc<dyn PluginCapabilityServices>) -> Self {
        Self::ConformanceInProcess(services)
    }
}

struct VerifiedPlugin {
    binding: InstalledComponentBinding,
    manifest: Arc<PluginManifest>,
    authorization: Arc<PluginAuthorization>,
    compiled: Arc<crate::CompiledPlugin>,
    manifest_bytes: Arc<[u8]>,
    component_bytes: Arc<[u8]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedComponentDeployment {
    extension_id: Arc<str>,
    extension_version: Arc<str>,
    plugin_identifier: Arc<str>,
    plugin_version: Arc<str>,
    manifest_sha256: Arc<str>,
    component_sha256: Arc<str>,
    authorization_generation: Arc<str>,
    manifest_bytes: Arc<[u8]>,
    authorization_bytes: Arc<[u8]>,
    component_bytes: Arc<[u8]>,
}

impl VerifiedComponentDeployment {
    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub fn extension_version(&self) -> &str {
        &self.extension_version
    }

    pub fn plugin_identifier(&self) -> &str {
        &self.plugin_identifier
    }

    pub fn plugin_version(&self) -> &str {
        &self.plugin_version
    }

    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }

    pub fn component_sha256(&self) -> &str {
        &self.component_sha256
    }

    pub fn authorization_generation(&self) -> &str {
        &self.authorization_generation
    }

    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    pub fn component_bytes(&self) -> &[u8] {
        &self.component_bytes
    }

    pub fn authorization_bytes(&self) -> &[u8] {
        &self.authorization_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedComponentGeneration {
    profile_id: Arc<str>,
    generation: u64,
    snapshot_sha256: Arc<str>,
    authorization_verifier: PluginAuthorizationVerifier,
    components: Arc<[VerifiedComponentDeployment]>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerPluginInvocation {
    registry_generation: WorkerRegistryGeneration,
    registry_digest_sha256: WorkerSha256Digest,
    extension_id: String,
    extension_version: String,
    plugin_identifier: String,
    plugin_version: String,
    manifest_digest_sha256: WorkerSha256Digest,
    component_digest_sha256: WorkerSha256Digest,
    authorization_generation: WorkerSha256Digest,
    node_id: String,
    inputs: InvocationInputs,
    timeout_milliseconds: u64,
    maximum_response_bytes: u64,
    component_limits: ComponentLimits,
}

impl WorkerPluginInvocation {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ComponentHostError> {
        if bytes.is_empty() || bytes.len() > MAX_WORKER_PLUGIN_INVOCATION_BYTES {
            return Err(worker_deployment_error(
                "worker plugin invocation exceeds its transport bound",
            ));
        }
        let invocation: Self = serde_json::from_slice(bytes).map_err(worker_deployment_error)?;
        invocation.validate()?;
        Ok(invocation)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ComponentHostError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(worker_deployment_error)?;
        if bytes.is_empty() || bytes.len() > MAX_WORKER_PLUGIN_INVOCATION_BYTES {
            return Err(worker_deployment_error(
                "worker plugin invocation exceeds its transport bound",
            ));
        }
        Ok(bytes)
    }

    pub const fn registry_generation(&self) -> WorkerRegistryGeneration {
        self.registry_generation
    }

    pub fn registry_digest_sha256(&self) -> &WorkerSha256Digest {
        &self.registry_digest_sha256
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub fn extension_version(&self) -> &str {
        &self.extension_version
    }

    pub fn plugin_identifier(&self) -> &str {
        &self.plugin_identifier
    }

    pub fn plugin_version(&self) -> &str {
        &self.plugin_version
    }

    pub fn manifest_digest_sha256(&self) -> &WorkerSha256Digest {
        &self.manifest_digest_sha256
    }

    pub fn component_digest_sha256(&self) -> &WorkerSha256Digest {
        &self.component_digest_sha256
    }

    pub fn authorization_generation(&self) -> &WorkerSha256Digest {
        &self.authorization_generation
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn inputs(&self) -> &InvocationInputs {
        &self.inputs
    }

    pub fn into_inputs(self) -> InvocationInputs {
        self.inputs
    }

    pub const fn timeout_milliseconds(&self) -> u64 {
        self.timeout_milliseconds
    }

    pub const fn maximum_response_bytes(&self) -> u64 {
        self.maximum_response_bytes
    }

    pub fn component_limits(&self) -> &ComponentLimits {
        &self.component_limits
    }

    fn validate(&self) -> Result<(), ComponentHostError> {
        if !valid_worker_plugin_identity(&self.extension_id)
            || !valid_worker_plugin_identity(&self.extension_version)
            || !valid_worker_plugin_identity(&self.plugin_identifier)
            || !valid_worker_plugin_identity(&self.plugin_version)
            || !valid_worker_plugin_identity(&self.node_id)
            || self.timeout_milliseconds == 0
            || self.timeout_milliseconds > MAX_WORKER_PLUGIN_TIMEOUT_MILLISECONDS
            || self.maximum_response_bytes == 0
            || self.maximum_response_bytes
                > u64::try_from(MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES)
                    .map_err(worker_deployment_error)?
        {
            return Err(worker_deployment_error(
                "worker plugin invocation metadata is invalid",
            ));
        }
        self.component_limits.validate()?;
        Ok(())
    }
}

impl VerifiedComponentGeneration {
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn snapshot_sha256(&self) -> &str {
        &self.snapshot_sha256
    }

    pub fn components(&self) -> &[VerifiedComponentDeployment] {
        &self.components
    }

    pub fn prepare_worker_invocation(
        &self,
        extension_id: &str,
        node_id: &str,
        inputs: InvocationInputs,
        timeout_milliseconds: u64,
        maximum_response_bytes: u64,
        component_limits: ComponentLimits,
    ) -> Result<WorkerPluginInvocation, ComponentHostError> {
        let component = self
            .components
            .iter()
            .find(|component| component.extension_id() == extension_id)
            .ok_or_else(|| ComponentHostError::MissingExtension(extension_id.to_owned()))?;
        let manifest: PluginManifest =
            serde_json::from_slice(component.manifest_bytes()).map_err(|error| {
                ComponentHostError::InvalidManifest {
                    extension_id: component.extension_id.clone(),
                    message: error.to_string(),
                }
            })?;
        if !manifest.nodes.iter().any(|node| node.id == node_id) {
            return Err(ComponentHostError::Plugin(PluginError::UndeclaredNode(
                node_id.to_owned(),
            )));
        }
        let deployment = self.worker_deployment_plan()?;
        let descriptor = deployment
            .begin()
            .components()
            .iter()
            .find(|descriptor| descriptor.extension_id() == extension_id)
            .ok_or_else(|| worker_deployment_error("worker descriptor vanished"))?;
        let invocation = WorkerPluginInvocation {
            registry_generation: deployment.begin().generation(),
            registry_digest_sha256: deployment.begin().registry_digest_sha256().clone(),
            extension_id: extension_id.to_owned(),
            extension_version: descriptor.extension_version().to_owned(),
            plugin_identifier: descriptor.plugin_identifier().to_owned(),
            plugin_version: descriptor.plugin_version().to_owned(),
            manifest_digest_sha256: descriptor.manifest_digest_sha256().clone(),
            component_digest_sha256: descriptor.component_digest_sha256().clone(),
            authorization_generation: descriptor.authorization_generation().clone(),
            node_id: node_id.to_owned(),
            inputs,
            timeout_milliseconds,
            maximum_response_bytes,
            component_limits,
        };
        invocation.validate()?;
        Ok(invocation)
    }

    pub fn worker_deployment_plan(
        &self,
    ) -> Result<WorkerRegistryDeploymentPlan, ComponentHostError> {
        let generation =
            WorkerRegistryGeneration::new(self.generation).map_err(worker_deployment_error)?;
        let mut descriptors = Vec::with_capacity(self.components.len());
        for component in self.components.iter() {
            descriptors.push(
                WorkerComponentDescriptor::new(
                    component.extension_id.to_string(),
                    component.extension_version.to_string(),
                    component.plugin_identifier.to_string(),
                    component.plugin_version.to_string(),
                    WorkerSha256Digest::new(component.authorization_generation.to_string())
                        .map_err(worker_deployment_error)?,
                    WorkerSha256Digest::new(component.manifest_sha256.to_string())
                        .map_err(worker_deployment_error)?,
                    WorkerSha256Digest::new(component.component_sha256.to_string())
                        .map_err(worker_deployment_error)?,
                    u64::try_from(component.manifest_bytes.len())
                        .map_err(|error| worker_deployment_error(error.to_string()))?,
                    u64::try_from(component.authorization_bytes.len())
                        .map_err(|error| worker_deployment_error(error.to_string()))?,
                    u64::try_from(component.component_bytes.len())
                        .map_err(|error| worker_deployment_error(error.to_string()))?,
                )
                .map_err(worker_deployment_error)?,
            );
        }
        descriptors.sort_by(|left, right| {
            (left.extension_id(), left.component_digest_sha256())
                .cmp(&(right.extension_id(), right.component_digest_sha256()))
        });
        let provisional = WorkerRegistryDeploymentBegin::new(
            generation,
            WorkerSha256Digest::new("0".repeat(64)).map_err(worker_deployment_error)?,
            descriptors,
        )
        .map_err(worker_deployment_error)?;
        let registry_digest = hex_sha256(&provisional.digest_material());
        let begin = WorkerRegistryDeploymentBegin::new(
            generation,
            WorkerSha256Digest::new(registry_digest).map_err(worker_deployment_error)?,
            provisional.components().to_vec(),
        )
        .map_err(worker_deployment_error)?;
        let mut chunks = Vec::new();
        for (component_index, descriptor) in begin.components().iter().enumerate() {
            let component_index = u32::try_from(component_index)
                .map_err(|error| worker_deployment_error(error.to_string()))?;
            let component = self
                .components
                .iter()
                .find(|component| component.extension_id() == descriptor.extension_id())
                .ok_or_else(|| worker_deployment_error("verified component identity vanished"))?;
            append_worker_chunks(
                &mut chunks,
                generation,
                component_index,
                WorkerComponentContent::Manifest,
                component.manifest_bytes(),
            )?;
            append_worker_chunks(
                &mut chunks,
                generation,
                component_index,
                WorkerComponentContent::Authorization,
                component.authorization_bytes(),
            )?;
            append_worker_chunks(
                &mut chunks,
                generation,
                component_index,
                WorkerComponentContent::Component,
                component.component_bytes(),
            )?;
        }
        WorkerRegistryDeploymentPlan::new(begin, chunks, self.authorization_verifier.clone())
            .map_err(worker_deployment_error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledComponentBinding {
    lifecycle_extension_id: Arc<str>,
    lifecycle_extension_version: Arc<str>,
    signed_plugin_identifier: Arc<str>,
    signed_plugin_version: Arc<str>,
    signed_digest_sha256: Arc<str>,
    signed_provenance_source: Arc<str>,
    signed_provenance_publisher: Arc<str>,
}

impl InstalledComponentBinding {
    fn checked(
        component: &InstalledComponent,
        manifest: &PluginManifest,
    ) -> Result<Self, ComponentHostError> {
        let extension_id: Arc<str> = Arc::from(component.extension_id());
        let extension_version: Arc<str> = Arc::from(component.extension_version());
        let signed_plugin_version = manifest.plugin_version.to_string();
        if extension_version.as_ref() != signed_plugin_version {
            return Err(ComponentHostError::ExtensionVersionMismatch {
                extension_id,
                extension_version,
                plugin_version: signed_plugin_version,
            });
        }
        Ok(Self {
            lifecycle_extension_id: extension_id,
            lifecycle_extension_version: extension_version,
            signed_plugin_identifier: Arc::from(manifest.identifier.as_str()),
            signed_plugin_version: Arc::from(signed_plugin_version),
            signed_digest_sha256: Arc::from(manifest.digest_sha256.as_str()),
            signed_provenance_source: Arc::from(manifest.provenance.source.as_str()),
            signed_provenance_publisher: Arc::from(manifest.provenance.publisher.as_str()),
        })
    }

    pub fn lifecycle_extension_id(&self) -> &str {
        &self.lifecycle_extension_id
    }

    pub fn lifecycle_extension_version(&self) -> &str {
        &self.lifecycle_extension_version
    }

    pub fn signed_plugin_identifier(&self) -> &str {
        &self.signed_plugin_identifier
    }

    pub fn signed_plugin_version(&self) -> &str {
        &self.signed_plugin_version
    }

    pub fn signed_digest_sha256(&self) -> &str {
        &self.signed_digest_sha256
    }

    pub fn signed_provenance_source(&self) -> &str {
        &self.signed_provenance_source
    }

    pub fn signed_provenance_publisher(&self) -> &str {
        &self.signed_provenance_publisher
    }
}

#[derive(Clone)]
pub struct InstalledVerifiedPlugin {
    inner: Arc<VerifiedPlugin>,
}

impl InstalledVerifiedPlugin {
    pub fn extension_id(&self) -> &str {
        self.inner.binding.lifecycle_extension_id()
    }

    pub fn extension_version(&self) -> &str {
        self.inner.binding.signed_plugin_version()
    }

    pub fn binding(&self) -> &InstalledComponentBinding {
        &self.inner.binding
    }

    pub fn manifest(&self) -> &PluginManifest {
        &self.inner.manifest
    }

    pub fn authorization(&self) -> &PluginAuthorization {
        &self.inner.authorization
    }
}

pub struct PreparedPluginInvocation {
    worker_invocation: WorkerPluginInvocation,
    deployment: WorkerRegistryDeploymentPlan,
    authorization: PluginAuthorization,
    context: NodeContext,
    plugin: InstalledVerifiedPlugin,
    _lease: InvocationLease,
}

impl PreparedPluginInvocation {
    pub fn worker_invocation(&self) -> &WorkerPluginInvocation {
        &self.worker_invocation
    }

    pub fn deployment(&self) -> &WorkerRegistryDeploymentPlan {
        &self.deployment
    }

    pub fn authorization(&self) -> &PluginAuthorization {
        &self.authorization
    }

    pub fn context(&self) -> &NodeContext {
        &self.context
    }
}

struct ComponentState {
    by_extension: BTreeMap<Arc<str>, InstalledVerifiedPlugin>,
    node_owners: BTreeMap<String, Arc<str>>,
    registry: NativeNodeRegistry,
    generation: VerifiedComponentGeneration,
}

struct ComponentHostInner {
    plugin_host: Arc<PluginHost>,
    trust_policy: PluginTrustPolicy,
    permission_policy: PermissionPolicy,
    authorization_sealer: PluginAuthorizationSealer,
    executor: Arc<dyn PluginInvocationExecutor>,
    conformance_services: Option<Arc<dyn PluginCapabilityServices>>,
    invocation_timeout_milliseconds: u64,
    invocation_maximum_response_bytes: u64,
    base_registry: NativeNodeRegistry,
    state: RwLock<ComponentState>,
    invocation_gate: Mutex<InvocationGate>,
    invocation_gate_changed: Condvar,
}

#[derive(Default)]
struct InvocationGate {
    active: usize,
    quiescing: bool,
}

struct InvocationLease {
    host: Arc<ComponentHostInner>,
}

struct ConformanceInProcessExecutor {
    plugin_host: Arc<PluginHost>,
    services: Arc<dyn PluginCapabilityServices>,
}

impl PluginInvocationExecutor for ConformanceInProcessExecutor {
    fn execute<'a>(
        &'a self,
        invocation: PreparedPluginInvocation,
    ) -> Pin<Box<dyn Future<Output = Result<InvocationResult, ComponentHostError>> + Send + 'a>>
    {
        Box::pin(async move {
            invocation.context.cancellation.check().map_err(|_| {
                PluginError::Invocation(comfy_plugin_sdk::InvocationError::Cancelled)
            })?;
            let host_invocation = self.plugin_host.begin_invocation(
                &invocation.plugin.inner.manifest,
                &invocation.plugin.inner.authorization,
                invocation.worker_invocation.node_id(),
                invocation.worker_invocation.inputs().clone(),
                self.services.clone(),
                invocation.context.cancellation.clone(),
            )?;
            let mut instance = self
                .plugin_host
                .instantiate_component(&invocation.plugin.inner.compiled, host_invocation)?;
            let node = instance.create_node(invocation.worker_invocation.node_id())?;
            if let Err(error) = instance.invoke(node) {
                instance.abort();
                return Err(error.into());
            }
            if let Err(error) = instance.drop_node(node) {
                instance.abort();
                return Err(error.into());
            }
            instance.finish().map_err(ComponentHostError::from)
        })
    }
}

struct ComponentHostRouterState {
    host: ComponentHost,
    extension_store_replay_snapshot: Vec<InstalledComponent>,
    next_generation: u64,
}

impl ComponentHostRouterState {
    fn revoke(&self) -> Result<(), ComponentHostError> {
        self.host.synchronize_components(Vec::new())
    }
}

#[derive(Clone)]
pub struct ComponentHostRouter {
    state: Arc<Mutex<ComponentHostRouterState>>,
}

impl ComponentHostRouter {
    pub fn new(host: ComponentHost) -> Self {
        Self {
            state: Arc::new(Mutex::new(ComponentHostRouterState {
                host,
                extension_store_replay_snapshot: Vec::new(),
                next_generation: 1,
            })),
        }
    }

    pub fn with_initial_generation(
        host: ComponentHost,
        initial_generation: u64,
    ) -> Result<Self, ComponentHostError> {
        if initial_generation == 0 {
            return Err(ComponentHostError::Verification {
                extension_id: Arc::from(COMFY_COMPONENT_ADAPTER_ID),
                message: "component generation must be nonzero".to_owned(),
            });
        }
        Ok(Self {
            state: Arc::new(Mutex::new(ComponentHostRouterState {
                host,
                extension_store_replay_snapshot: Vec::new(),
                next_generation: initial_generation,
            })),
        })
    }

    pub fn current(&self) -> Result<ComponentHost, ComponentHostError> {
        self.state
            .lock()
            .map(|state| state.host.clone())
            .map_err(|_| ComponentHostError::StateUnavailable)
    }

    pub fn replace(&self, host: ComponentHost) -> Result<(), ComponentHostError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        replace_component_host(&mut state, host)
    }

    pub fn replace_with_initial_generation(
        &self,
        host: ComponentHost,
        initial_generation: u64,
    ) -> Result<(), ComponentHostError> {
        if initial_generation == 0 {
            return Err(ComponentHostError::Verification {
                extension_id: Arc::from(COMFY_COMPONENT_ADAPTER_ID),
                message: "component generation must be nonzero".to_owned(),
            });
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        state.next_generation = state.next_generation.max(initial_generation);
        replace_component_host(&mut state, host)
    }
}

fn replace_component_host(
    state: &mut ComponentHostRouterState,
    host: ComponentHost,
) -> Result<(), ComponentHostError> {
    if Arc::ptr_eq(&state.host.inner, &host.inner) {
        return Ok(());
    }
    let generation = state.next_generation;
    host.synchronize_components_at_generation(
        state.extension_store_replay_snapshot.clone(),
        generation,
    )?;
    state.revoke()?;
    state.next_generation = state
        .next_generation
        .checked_add(1)
        .ok_or(ComponentHostError::StateUnavailable)?;
    state.host = host;
    Ok(())
}

impl Drop for InvocationLease {
    fn drop(&mut self) {
        let mut gate = match self.host.invocation_gate.lock() {
            Ok(gate) => gate,
            Err(poisoned) => poisoned.into_inner(),
        };
        gate.active = gate.active.saturating_sub(1);
        self.host.invocation_gate_changed.notify_all();
    }
}

#[derive(Clone)]
pub struct ComponentHost {
    inner: Arc<ComponentHostInner>,
}

impl ComponentHost {
    pub fn new(
        runtime: ComponentRuntime,
        trust_policy: PluginTrustPolicy,
        permission_policy: PermissionPolicy,
        execution_boundary: ComponentExecutionBoundary,
        limits: ComponentLimits,
        base_registry: NativeNodeRegistry,
    ) -> Result<Self, ComponentHostError> {
        let profile_id = permission_policy.profile_id().to_owned();
        let authorization_sealer = PluginAuthorizationSealer::generate(
            permission_policy.generation(),
        )
        .map_err(|error| ComponentHostError::Verification {
            extension_id: Arc::from(COMFY_COMPONENT_ADAPTER_ID),
            message: format!("failed to initialize authorization sealing: {error}"),
        })?;
        let authorization_verifier =
            authorization_sealer
                .verifier()
                .map_err(|error| ComponentHostError::Verification {
                    extension_id: Arc::from(COMFY_COMPONENT_ADAPTER_ID),
                    message: format!("failed to initialize authorization verification: {error}"),
                })?;
        let invocation_timeout_milliseconds = limits
            .capability_limits
            .maximum_timeout_milliseconds
            .min(MAX_WORKER_PLUGIN_TIMEOUT_MILLISECONDS);
        let invocation_maximum_response_bytes =
            limits.capability_limits.maximum_response_bytes.min(
                u64::try_from(MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES)
                    .map_err(|error| ComponentHostError::ExecutionBoundary(error.to_string()))?,
            );
        let plugin_host = PluginHost::with_component_runtime(
            runtime,
            limits,
            crate::DEFAULT_API_FEATURES
                .iter()
                .map(|feature| (*feature).to_owned()),
        )?;
        let plugin_host = Arc::new(plugin_host);
        let (executor, conformance_services): (
            Arc<dyn PluginInvocationExecutor>,
            Option<Arc<dyn PluginCapabilityServices>>,
        ) = match execution_boundary {
            ComponentExecutionBoundary::PrivateWorker(executor) => (executor, None),
            ComponentExecutionBoundary::ConformanceInProcess(services) => (
                Arc::new(ConformanceInProcessExecutor {
                    plugin_host: plugin_host.clone(),
                    services: services.clone(),
                }),
                Some(services),
            ),
        };
        Ok(Self {
            inner: Arc::new(ComponentHostInner {
                plugin_host,
                trust_policy,
                permission_policy,
                authorization_sealer,
                executor,
                conformance_services,
                invocation_timeout_milliseconds,
                invocation_maximum_response_bytes,
                base_registry: base_registry.clone(),
                state: RwLock::new(ComponentState {
                    by_extension: BTreeMap::new(),
                    node_owners: BTreeMap::new(),
                    registry: base_registry,
                    generation: empty_component_generation(&profile_id, 0, authorization_verifier),
                }),
                invocation_gate: Mutex::new(InvocationGate::default()),
                invocation_gate_changed: Condvar::new(),
            }),
        })
    }

    pub fn installed_plugins(&self) -> Result<Vec<InstalledVerifiedPlugin>, ComponentHostError> {
        let state = self
            .inner
            .state
            .read()
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        Ok(state.by_extension.values().cloned().collect())
    }

    pub fn installed_plugin(
        &self,
        extension_id: &str,
    ) -> Result<InstalledVerifiedPlugin, ComponentHostError> {
        let state = self
            .inner
            .state
            .read()
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        state
            .by_extension
            .get(extension_id)
            .cloned()
            .ok_or_else(|| ComponentHostError::MissingExtension(extension_id.to_owned()))
    }

    pub fn plugin_for_node(
        &self,
        node_id: &str,
    ) -> Result<InstalledVerifiedPlugin, ComponentHostError> {
        let state = self
            .inner
            .state
            .read()
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        let extension_id = state
            .node_owners
            .get(node_id)
            .ok_or_else(|| ComponentHostError::MissingExtension(node_id.to_owned()))?;
        state
            .by_extension
            .get(extension_id)
            .cloned()
            .ok_or(ComponentHostError::StateUnavailable)
    }

    pub fn registry_snapshot(&self) -> Result<NativeNodeRegistry, ComponentHostError> {
        self.inner
            .state
            .read()
            .map(|state| state.registry.clone())
            .map_err(|_| ComponentHostError::StateUnavailable)
    }

    pub fn verified_generation(&self) -> Result<VerifiedComponentGeneration, ComponentHostError> {
        self.inner
            .state
            .read()
            .map(|state| state.generation.clone())
            .map_err(|_| ComponentHostError::StateUnavailable)
    }

    pub(crate) fn prepare_plugin_invocation(
        &self,
        plugin: &InstalledVerifiedPlugin,
        node_id: &str,
        inputs: InvocationInputs,
        context: NodeContext,
    ) -> Result<PreparedPluginInvocation, ComponentHostError> {
        let lease = self.begin_invocation_lease(plugin)?;
        let generation = self.verified_generation()?;
        let worker_invocation = generation.prepare_worker_invocation(
            plugin.extension_id(),
            node_id,
            inputs,
            self.inner.invocation_timeout_milliseconds,
            self.inner.invocation_maximum_response_bytes,
            self.inner.plugin_host.limits().clone(),
        )?;
        let deployment = generation.worker_deployment_plan()?;
        Ok(PreparedPluginInvocation {
            worker_invocation,
            deployment,
            authorization: plugin.authorization().clone(),
            context,
            plugin: plugin.clone(),
            _lease: lease,
        })
    }

    pub(crate) fn executor(&self) -> Arc<dyn PluginInvocationExecutor> {
        self.inner.executor.clone()
    }

    pub async fn execute_plugin(
        &self,
        plugin: &InstalledVerifiedPlugin,
        node_id: &str,
        inputs: InvocationInputs,
        context: NodeContext,
    ) -> Result<InvocationResult, ComponentHostError> {
        let prepared = self.prepare_plugin_invocation(plugin, node_id, inputs, context)?;
        self.executor().execute(prepared).await
    }

    pub fn invoke(
        &self,
        plugin: &InstalledVerifiedPlugin,
        node_id: &str,
        inputs: InvocationInputs,
        cancellation: CancellationToken,
    ) -> Result<InvocationResult, ComponentHostError> {
        let _invocation_lease = self.begin_invocation_lease(plugin)?;
        let services = self.inner.conformance_services.clone().ok_or_else(|| {
            ComponentHostError::ExecutionBoundary(
                "direct component invocation is only available to the conformance oracle"
                    .to_owned(),
            )
        })?;
        cancellation
            .check()
            .map_err(|_| PluginError::Invocation(comfy_plugin_sdk::InvocationError::Cancelled))?;
        let invocation = self.inner.plugin_host.begin_invocation(
            &plugin.inner.manifest,
            &plugin.inner.authorization,
            node_id,
            inputs,
            services,
            cancellation,
        )?;
        let mut instance = self
            .inner
            .plugin_host
            .instantiate_component(&plugin.inner.compiled, invocation)?;
        let node = instance.create_node(node_id)?;
        if let Err(error) = instance.invoke(node) {
            instance.abort();
            return Err(error.into());
        }
        if let Err(error) = instance.drop_node(node) {
            instance.abort();
            return Err(error.into());
        }
        instance.finish().map_err(ComponentHostError::from)
    }

    fn begin_invocation_lease(
        &self,
        plugin: &InstalledVerifiedPlugin,
    ) -> Result<InvocationLease, ComponentHostError> {
        let mut gate = self
            .inner
            .invocation_gate
            .lock()
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        while gate.quiescing {
            gate = self
                .inner
                .invocation_gate_changed
                .wait(gate)
                .map_err(|_| ComponentHostError::StateUnavailable)?;
        }
        let state = self
            .inner
            .state
            .read()
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        let active = state
            .by_extension
            .get(plugin.extension_id())
            .ok_or_else(|| ComponentHostError::Revoked(plugin.extension_id().to_owned()))?;
        if !Arc::ptr_eq(&active.inner, &plugin.inner) {
            return Err(ComponentHostError::Revoked(
                plugin.extension_id().to_owned(),
            ));
        }
        gate.active = gate
            .active
            .checked_add(1)
            .ok_or(ComponentHostError::StateUnavailable)?;
        drop(state);
        drop(gate);
        Ok(InvocationLease {
            host: self.inner.clone(),
        })
    }

    fn synchronize_components(
        &self,
        components: Vec<InstalledComponent>,
    ) -> Result<(), ComponentHostError> {
        let generation = self
            .inner
            .state
            .read()
            .map_err(|_| ComponentHostError::StateUnavailable)?
            .generation
            .generation
            .saturating_add(1)
            .max(1);
        self.synchronize_components_at_generation(components, generation)
    }

    fn synchronize_components_at_generation(
        &self,
        components: Vec<InstalledComponent>,
        generation: u64,
    ) -> Result<(), ComponentHostError> {
        let mut by_extension = BTreeMap::new();
        let mut plugin_ids = BTreeSet::new();
        let mut node_owners = BTreeMap::new();
        for component in components {
            let extension_id: Arc<str> = Arc::from(component.extension_id());
            let manifest: PluginManifest = serde_json::from_slice(component.manifest_bytes())
                .map_err(|error| ComponentHostError::InvalidManifest {
                    extension_id: extension_id.clone(),
                    message: error.to_string(),
                })?;
            let binding = InstalledComponentBinding::checked(&component, &manifest)?;
            if !plugin_ids.insert(manifest.identifier.clone()) {
                return Err(ComponentHostError::DuplicatePlugin(manifest.identifier));
            }
            let authorization = self
                .inner
                .trust_policy
                .authorize_manifest(&manifest, &self.inner.permission_policy)
                .map_err(|error| ComponentHostError::Verification {
                    extension_id: extension_id.clone(),
                    message: error.to_string(),
                })?;
            let compiled = self
                .inner
                .plugin_host
                .compile_component(component.component_bytes(), &manifest, &authorization)
                .map_err(|error| ComponentHostError::Verification {
                    extension_id: extension_id.clone(),
                    message: error.to_string(),
                })?;
            for node in &manifest.nodes {
                if node_owners
                    .insert(node.id.clone(), extension_id.clone())
                    .is_some()
                {
                    return Err(ComponentHostError::DuplicateNode(node.id.clone()));
                }
            }
            let plugin = InstalledVerifiedPlugin {
                inner: Arc::new(VerifiedPlugin {
                    binding,
                    manifest: Arc::new(manifest),
                    authorization: Arc::new(authorization),
                    compiled: Arc::new(compiled),
                    manifest_bytes: Arc::from(component.manifest_bytes()),
                    component_bytes: Arc::from(component.component_bytes()),
                }),
            };
            if by_extension.insert(extension_id.clone(), plugin).is_some() {
                return Err(ComponentHostError::DuplicatePlugin(
                    extension_id.to_string(),
                ));
            }
        }
        let plugins = by_extension.values().cloned().collect();
        let verified_generation = component_generation(
            self.inner.permission_policy.profile_id(),
            generation,
            by_extension.values(),
            &self.inner.authorization_sealer,
        )?;
        let registry = crate::registry_adapter::registry_with_plugins(
            &self.inner.base_registry,
            self,
            plugins,
        )
        .map_err(|error| ComponentHostError::Verification {
            extension_id: Arc::from(COMFY_COMPONENT_ADAPTER_ID),
            message: error.to_string(),
        })?;
        let mut gate = self
            .inner
            .invocation_gate
            .lock()
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        while gate.quiescing {
            gate = self
                .inner
                .invocation_gate_changed
                .wait(gate)
                .map_err(|_| ComponentHostError::StateUnavailable)?;
        }
        gate.quiescing = true;
        while gate.active != 0 {
            gate = self
                .inner
                .invocation_gate_changed
                .wait(gate)
                .map_err(|_| ComponentHostError::StateUnavailable)?;
        }
        let mut state = match self.inner.state.write() {
            Ok(state) => state,
            Err(_) => {
                gate.quiescing = false;
                self.inner.invocation_gate_changed.notify_all();
                return Err(ComponentHostError::StateUnavailable);
            }
        };
        *state = ComponentState {
            by_extension,
            node_owners,
            registry,
            generation: verified_generation,
        };
        gate.quiescing = false;
        self.inner.invocation_gate_changed.notify_all();
        Ok(())
    }
}

fn empty_component_generation(
    profile_id: &str,
    generation: u64,
    authorization_verifier: PluginAuthorizationVerifier,
) -> VerifiedComponentGeneration {
    VerifiedComponentGeneration {
        profile_id: Arc::from(profile_id),
        generation,
        snapshot_sha256: Arc::from(hex_sha256(&[])),
        authorization_verifier,
        components: Arc::from([]),
    }
}

fn append_worker_chunks(
    chunks: &mut Vec<WorkerRegistryDeploymentChunk>,
    generation: WorkerRegistryGeneration,
    component_index: u32,
    content: WorkerComponentContent,
    bytes: &[u8],
) -> Result<(), ComponentHostError> {
    for (chunk_index, chunk) in bytes.chunks(MAX_WORKER_COMPONENT_CHUNK_BYTES).enumerate() {
        let chunk_index = u32::try_from(chunk_index)
            .map_err(|error| worker_deployment_error(error.to_string()))?;
        chunks.push(
            WorkerRegistryDeploymentChunk::new(
                generation,
                component_index,
                content,
                chunk_index,
                chunk.to_vec(),
            )
            .map_err(worker_deployment_error)?,
        );
    }
    Ok(())
}

fn worker_deployment_error(error: impl ToString) -> ComponentHostError {
    ComponentHostError::Verification {
        extension_id: Arc::from(COMFY_COMPONENT_ADAPTER_ID),
        message: format!("worker deployment mapping failed: {}", error.to_string()),
    }
}

fn valid_worker_plugin_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1_024
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn component_generation<'a>(
    profile_id: &str,
    generation: u64,
    plugins: impl IntoIterator<Item = &'a InstalledVerifiedPlugin>,
    authorization_sealer: &PluginAuthorizationSealer,
) -> Result<VerifiedComponentGeneration, ComponentHostError> {
    if generation == 0 {
        return Err(ComponentHostError::Verification {
            extension_id: Arc::from(COMFY_COMPONENT_ADAPTER_ID),
            message: "component generation must be nonzero".to_owned(),
        });
    }
    let mut components = Vec::new();
    for plugin in plugins {
        let manifest_sha256 = hex_sha256(&plugin.inner.manifest_bytes);
        let component_sha256 = hex_sha256(&plugin.inner.component_bytes);
        if component_sha256 != plugin.inner.binding.signed_digest_sha256() {
            return Err(ComponentHostError::Verification {
                extension_id: Arc::from(plugin.extension_id()),
                message: "verified component bytes no longer match the signed digest".to_owned(),
            });
        }
        let authorization_bytes = plugin
            .inner
            .authorization
            .sealed_bytes(authorization_sealer)
            .map_err(|error| ComponentHostError::Verification {
                extension_id: Arc::from(plugin.extension_id()),
                message: format!("failed to seal worker authorization: {error}"),
            })?;
        let authorization_generation = hex_sha256(&authorization_bytes);
        components.push(VerifiedComponentDeployment {
            extension_id: Arc::from(plugin.extension_id()),
            extension_version: Arc::from(plugin.extension_version()),
            plugin_identifier: Arc::from(plugin.inner.binding.signed_plugin_identifier()),
            plugin_version: Arc::from(plugin.inner.binding.signed_plugin_version()),
            manifest_sha256: Arc::from(manifest_sha256),
            component_sha256: Arc::from(component_sha256),
            authorization_generation: Arc::from(authorization_generation),
            manifest_bytes: plugin.inner.manifest_bytes.clone(),
            authorization_bytes: Arc::from(authorization_bytes),
            component_bytes: plugin.inner.component_bytes.clone(),
        });
    }
    components.sort_by(|left, right| left.extension_id.cmp(&right.extension_id));
    let mut snapshot = Sha256::new();
    hash_field(&mut snapshot, profile_id.as_bytes());
    hash_field(&mut snapshot, &generation.to_le_bytes());
    for component in &components {
        hash_field(&mut snapshot, component.extension_id.as_bytes());
        hash_field(&mut snapshot, component.extension_version.as_bytes());
        hash_field(&mut snapshot, component.plugin_identifier.as_bytes());
        hash_field(&mut snapshot, component.plugin_version.as_bytes());
        hash_field(&mut snapshot, component.manifest_sha256.as_bytes());
        hash_field(&mut snapshot, component.component_sha256.as_bytes());
        hash_field(&mut snapshot, component.authorization_generation.as_bytes());
    }
    Ok(VerifiedComponentGeneration {
        profile_id: Arc::from(profile_id),
        generation,
        snapshot_sha256: Arc::from(format!("{:x}", snapshot.finalize())),
        authorization_verifier: authorization_sealer.verifier().map_err(|error| {
            ComponentHostError::Verification {
                extension_id: Arc::from(COMFY_COMPONENT_ADAPTER_ID),
                message: format!("failed to project authorization verifier: {error}"),
            }
        })?,
        components: Arc::from(components),
    })
}

fn hash_field(digest: &mut Sha256, value: &[u8]) {
    digest.update(u64::try_from(value.len()).unwrap_or(u64::MAX).to_le_bytes());
    digest.update(value);
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

impl ComponentLifecycleAdapter for ComponentHostRouter {
    fn adapter_id(&self) -> &'static str {
        COMFY_COMPONENT_ADAPTER_ID
    }

    fn synchronize(
        &self,
        components: Vec<InstalledComponent>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'static>> {
        let this = self.clone();
        Box::pin(async move {
            smol::unblock(move || {
                let mut state = this
                    .state
                    .lock()
                    .map_err(|_| ComponentHostError::StateUnavailable)?;
                let generation = state.next_generation;
                state
                    .host
                    .synchronize_components_at_generation(components.clone(), generation)?;
                state.next_generation = state
                    .next_generation
                    .checked_add(1)
                    .ok_or(ComponentHostError::StateUnavailable)?;
                state.extension_store_replay_snapshot = components;
                Ok::<(), ComponentHostError>(())
            })
            .await
            .map_err(|error| error.to_string())
        })
    }
}
