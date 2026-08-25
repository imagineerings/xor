use crate::{
    ComponentLimits, InvocationInputs, InvocationResult, PluginCapabilityServices, PluginError,
    PluginHost, ProviderInvocationResult, WasmPluginInstance,
};
use comfy_nodes::NativeSchemaValue;
use comfy_plugin_sdk::{PluginManifest, ProviderPluginManifestV2};
use comfy_runtime::{
    NativeNodeRegistry, NativeProviderInvocationAuthority, NativeProviderInvocationScope,
    NativeProviderRegistryPin, NodeContext, PermissionPolicy, PluginAuthorization,
    PluginAuthorizationSealer, PluginAuthorizationVerifier, PluginCapabilityBroker,
    PluginCapabilityInvocation, PluginServiceError, PluginServiceInvocationContext,
    PluginTrustPolicy, PreflightedProviderRuntimeActivationGrant,
    ProviderCostAuthorizationAuthority, ProviderManifestAuthorizationV2, ProviderPolicy,
    ProviderResultReceiptAuthority, ProviderResultReceiptIssuer, ProviderRuntimeActivationGrant,
    ProviderRuntimeAuthorityInput, WorkerRegistryDeploymentPlan,
};
use comfy_types::{
    CancellationToken, MAX_WORKER_COMPONENT_CHUNK_BYTES,
    MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES, MAX_WORKER_PLUGIN_INVOCATION_BYTES,
    WorkerComponentContent, WorkerComponentDescriptor, WorkerProviderInvocationContext,
    WorkerRegistryDeploymentBegin, WorkerRegistryDeploymentChunk, WorkerRegistryGeneration,
    WorkerSha256Digest,
};
use extension_host::{ComponentLifecycleAdapter, ComponentRuntime, InstalledComponent};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    sync::{Arc, Condvar, Mutex, RwLock},
    time::{Duration, Instant},
};
use thiserror::Error;

pub const COMFY_COMPONENT_ADAPTER_ID: &str = "zed.comfy.component-host.v1";
pub const MAX_WORKER_PLUGIN_TIMEOUT_MILLISECONDS: u64 = 60_000;

#[derive(Debug, Error)]
pub enum ComponentHostError {
    #[error("component invocation was cancelled")]
    Cancelled,
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

    fn execute_provider<'a>(
        &'a self,
        invocation: PreparedPluginInvocation,
    ) -> Pin<
        Box<dyn Future<Output = Result<ProviderInvocationResult, ComponentHostError>> + Send + 'a>,
    >;
}

#[derive(Clone)]
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
    provider_manifest_v2: Option<Arc<ProviderPluginManifestV2>>,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production consumer is comfy-parity-provider-worker-stream-bridge"
        )
    )]
    provider_authorization_v2: Option<Arc<ProviderManifestAuthorizationV2>>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider_request: Option<Vec<u8>>,
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

    pub fn provider_request(&self) -> Option<&[u8]> {
        self.provider_request.as_deref()
    }

    pub fn into_execution_parts(self) -> (InvocationInputs, Option<Vec<u8>>) {
        (self.inputs, self.provider_request)
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
            || self.provider_request.as_ref().is_some_and(|request| {
                request.is_empty() || request.len() > MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES
            })
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

    pub fn provider_registry_pin(
        &self,
    ) -> Result<Option<NativeProviderRegistryPin>, ComponentHostError> {
        let mut binding_digests = Vec::new();
        for component in self.components.iter() {
            let (manifest, _) =
                parse_component_manifest(component.manifest_bytes()).map_err(|error| {
                    ComponentHostError::InvalidManifest {
                        extension_id: component.extension_id.clone(),
                        message: error.to_string(),
                    }
                })?;
            if let Some(provider_binding) = manifest.provider_binding {
                binding_digests.push(provider_binding.bindings_sha256);
            }
        }
        if binding_digests.is_empty() {
            return Ok(None);
        }
        binding_digests.sort();
        binding_digests.dedup();
        let deployment = self.worker_deployment_plan()?;
        NativeProviderRegistryPin::checked(
            deployment.begin().generation().get(),
            deployment
                .begin()
                .registry_digest_sha256()
                .as_str()
                .to_owned(),
            binding_digests,
        )
        .map(Some)
        .map_err(worker_deployment_error)
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
        self.prepare_worker_invocation_with_provider_request(
            extension_id,
            node_id,
            inputs,
            None,
            timeout_milliseconds,
            maximum_response_bytes,
            component_limits,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prepare_worker_provider_invocation(
        &self,
        extension_id: &str,
        node_id: &str,
        inputs: InvocationInputs,
        provider_request: Vec<u8>,
        timeout_milliseconds: u64,
        maximum_response_bytes: u64,
        component_limits: ComponentLimits,
    ) -> Result<WorkerPluginInvocation, ComponentHostError> {
        self.prepare_worker_invocation_with_provider_request(
            extension_id,
            node_id,
            inputs,
            Some(provider_request),
            timeout_milliseconds,
            maximum_response_bytes,
            component_limits,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_worker_invocation_with_provider_request(
        &self,
        extension_id: &str,
        node_id: &str,
        inputs: InvocationInputs,
        provider_request: Option<Vec<u8>>,
        timeout_milliseconds: u64,
        maximum_response_bytes: u64,
        component_limits: ComponentLimits,
    ) -> Result<WorkerPluginInvocation, ComponentHostError> {
        let component = self
            .components
            .iter()
            .find(|component| component.extension_id() == extension_id)
            .ok_or_else(|| ComponentHostError::MissingExtension(extension_id.to_owned()))?;
        let (manifest, provider_manifest_v2) = parse_component_manifest(component.manifest_bytes())
            .map_err(|error| ComponentHostError::InvalidManifest {
                extension_id: component.extension_id.clone(),
                message: error.to_string(),
            })?;
        if provider_manifest_v2.is_some() {
            return Err(ComponentHostError::ExecutionBoundary(
                "provider-v8 worker routing is unavailable until the canonical bridge is active"
                    .to_owned(),
            ));
        }
        if !manifest.nodes.iter().any(|node| node.id == node_id) {
            return Err(ComponentHostError::Plugin(PluginError::UndeclaredNode(
                node_id.to_owned(),
            )));
        }
        let node_is_provider_bound = manifest.provider_binding.as_ref().is_some_and(|binding| {
            binding
                .bindings
                .iter()
                .any(|claim| claim.node_id == node_id)
        });
        if node_is_provider_bound != provider_request.is_some() {
            return Err(ComponentHostError::ExecutionBoundary(
                "worker invocation mode disagrees with the signed provider binding".to_owned(),
            ));
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
            provider_request,
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

    pub(crate) fn is_provider_v2(&self) -> bool {
        self.inner.provider_manifest_v2.is_some()
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production consumer is comfy-parity-provider-worker-stream-bridge"
        )
    )]
    pub(crate) fn provider_manifest_v2(&self) -> Option<&ProviderPluginManifestV2> {
        self.inner.provider_manifest_v2.as_deref()
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production consumer is comfy-parity-provider-worker-stream-bridge"
        )
    )]
    pub(crate) fn provider_authorization_v2(&self) -> Option<&ProviderManifestAuthorizationV2> {
        self.inner.provider_authorization_v2.as_deref()
    }

    fn require_legacy_invocation_route(&self) -> Result<(), ComponentHostError> {
        if self.is_provider_v2() {
            return Err(ComponentHostError::ExecutionBoundary(
                "provider-v8 worker routing is unavailable until the canonical bridge is active"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

pub struct PreparedPluginInvocation {
    worker_invocation: WorkerPluginInvocation,
    deployment: WorkerRegistryDeploymentPlan,
    authorization: PluginAuthorization,
    context: NodeContext,
    plugin: InstalledVerifiedPlugin,
    provider_price_badge: Option<NativeSchemaValue>,
    _lease: InvocationLease,
}

#[allow(dead_code)]
pub(crate) struct PreflightedProviderComponentCapsule {
    grant: PreflightedProviderRuntimeActivationGrant,
    worker_context: WorkerProviderInvocationContext,
    manifest_authorization: ProviderManifestAuthorizationV2,
    plugin: InstalledVerifiedPlugin,
    generation: VerifiedComponentGeneration,
    node_id: Arc<str>,
    plugin_host: Arc<PluginHost>,
    _lease: InvocationLease,
}

#[allow(dead_code)]
impl PreflightedProviderComponentCapsule {
    pub(crate) fn new(
        host: &ComponentHost,
        grant: ProviderRuntimeActivationGrant,
        worker_context: WorkerProviderInvocationContext,
        worker_start: &comfy_runtime::NativeProviderWorkerSessionStart,
        manifest_authorization: ProviderManifestAuthorizationV2,
    ) -> Result<Self, ComponentHostError> {
        let plugin = host.installed_plugin(&worker_start.extension_id)?;
        let generation = host.verified_generation()?;
        let deployment = generation.worker_deployment_plan()?;
        let node_id: Arc<str> = Arc::from(worker_start.node_id.as_str());
        if !plugin
            .manifest()
            .nodes
            .iter()
            .any(|node| node.id == node_id.as_ref())
        {
            return Err(ComponentHostError::Plugin(PluginError::UndeclaredNode(
                node_id.to_string(),
            )));
        }
        let grant = grant
            .preflight_installed_component(
                &worker_context,
                &deployment,
                worker_start,
                manifest_authorization.clone(),
            )
            .map_err(|error| match error {
                PluginServiceError::Cancelled => ComponentHostError::Cancelled,
                error => ComponentHostError::ExecutionBoundary(format!(
                    "provider component activation preflight failed: {error}"
                )),
            })?;
        let lease = host.begin_invocation_lease(&plugin)?;
        Ok(Self {
            grant,
            worker_context,
            manifest_authorization,
            plugin,
            generation,
            node_id,
            plugin_host: host.inner.plugin_host.clone(),
            _lease: lease,
        })
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "production consumer is comfy-parity-provider-worker-stream-bridge"
    )
)]
pub(crate) struct PreparedProviderV2Invocation {
    grant: Option<PreflightedProviderRuntimeActivationGrant>,
    worker_context: WorkerProviderInvocationContext,
    manifest_authorization: ProviderManifestAuthorizationV2,
    plugin: InstalledVerifiedPlugin,
    generation: VerifiedComponentGeneration,
    node_id: Arc<str>,
    route: crate::ProviderV2StreamRouteReceiver,
    runtime: crate::ProviderV2RuntimeHost,
    cancellation: CancellationToken,
    plugin_host: Arc<PluginHost>,
    _lease: InvocationLease,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "production consumer is comfy-parity-provider-worker-stream-bridge"
    )
)]
pub(crate) struct ProviderV2ComponentInvocation {
    instance: WasmPluginInstance,
    node_id: Arc<str>,
    _plugin: InstalledVerifiedPlugin,
    _generation: VerifiedComponentGeneration,
    _lease: InvocationLease,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "production consumer is comfy-parity-provider-worker-stream-bridge"
    )
)]
pub(crate) struct ProviderV2AppRoute {
    grant: Option<PreflightedProviderRuntimeActivationGrant>,
    worker_context: WorkerProviderInvocationContext,
    manifest_authorization: ProviderManifestAuthorizationV2,
    route: crate::ProviderV2StreamRouteReceiver,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "production consumer is comfy-parity-provider-worker-stream-bridge"
    )
)]
impl PreflightedProviderComponentCapsule {
    pub(crate) fn prepare_stream_route(
        self,
        cancellation: CancellationToken,
    ) -> Result<PreparedProviderV2Invocation, ComponentHostError> {
        let contract =
            crate::worker_streaming_contract(self.manifest_authorization.streaming_contract());
        let (route, receiver) = crate::provider_v2_stream_route();
        let runtime = crate::ProviderV2RuntimeHost::checked_from_certified_capsule(
            self.worker_context.clone(),
            contract,
            cancellation.clone(),
            route,
        )
        .map_err(|error| ComponentHostError::ExecutionBoundary(error.to_string()))?;
        Ok(PreparedProviderV2Invocation {
            grant: Some(self.grant),
            worker_context: self.worker_context,
            manifest_authorization: self.manifest_authorization,
            plugin: self.plugin,
            generation: self.generation,
            node_id: self.node_id,
            route: receiver,
            runtime,
            cancellation,
            plugin_host: self.plugin_host,
            _lease: self._lease,
        })
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "production consumer is comfy-parity-provider-worker-stream-bridge"
    )
)]
impl PreparedProviderV2Invocation {
    pub(crate) fn into_execution(
        self,
        inputs: InvocationInputs,
        services: Arc<dyn PluginCapabilityServices>,
    ) -> Result<(ProviderV2ComponentInvocation, ProviderV2AppRoute), ComponentHostError> {
        let invocation = self.plugin_host.begin_invocation(
            self.plugin.manifest(),
            self.plugin.authorization(),
            self.node_id.as_ref(),
            inputs,
            services,
            self.cancellation,
        )?;
        let instance = self.plugin_host.instantiate_provider_component_v2(
            &self.plugin.inner.compiled,
            invocation,
            self.runtime,
        )?;
        Ok((
            ProviderV2ComponentInvocation {
                instance,
                node_id: self.node_id,
                _plugin: self.plugin,
                _generation: self.generation,
                _lease: self._lease,
            },
            ProviderV2AppRoute {
                grant: self.grant,
                worker_context: self.worker_context,
                manifest_authorization: self.manifest_authorization,
                route: self.route,
            },
        ))
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "production consumer is comfy-parity-provider-worker-stream-bridge"
    )
)]
impl ProviderV2ComponentInvocation {
    pub(crate) fn invoke(self) -> Result<crate::ProviderV2InvocationProposal, ComponentHostError> {
        self.instance
            .invoke_provider_v2(self.node_id.as_ref())
            .map_err(ComponentHostError::from)
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "production consumer is comfy-parity-provider-worker-stream-bridge"
    )
)]
impl ProviderV2AppRoute {
    pub(crate) fn bind_start_request(
        &mut self,
        call: crate::ProviderV2StreamRouteCall,
        policy: &ProviderPolicy,
    ) -> Result<
        (
            ProviderRuntimeAuthorityInput,
            crate::ProviderV2BoundStartCall,
        ),
        ComponentHostError,
    > {
        let (call_id, context, head, reply) = call
            .into_start()
            .map_err(|error| ComponentHostError::ExecutionBoundary(error.to_string()))?;
        if context != self.worker_context {
            return Err(ComponentHostError::ExecutionBoundary(
                "provider-v2 route context differs from its certified activation".to_owned(),
            ));
        }
        let authority = self
            .grant
            .take()
            .ok_or_else(|| {
                ComponentHostError::ExecutionBoundary(
                    "provider-v2 activation grant was already consumed".to_owned(),
                )
            })?
            .bind(&crate::sdk_request_head(&head), policy)
            .map_err(|error| ComponentHostError::ExecutionBoundary(error.to_string()))?;
        Ok((
            authority,
            crate::ProviderV2BoundStartCall { call_id, reply },
        ))
    }

    pub(crate) fn try_receive_stream_call(
        &self,
    ) -> Result<crate::ProviderV2StreamRouteMessage, std::sync::mpsc::TryRecvError> {
        self.route.try_receive()
    }
}

#[cfg(test)]
mod activation_preflight_tests {
    #[test]
    fn provider_component_capsule_is_the_only_preflight_callsite() {
        let source = include_str!("component_host.rs");
        let production = source
            .split("#[cfg(test)]\nmod activation_preflight_tests")
            .next()
            .expect("component host production source is missing");
        let call = [".preflight_installed_", "component("].concat();
        assert_eq!(production.matches(&call).count(), 1);
        let fields = source
            .split("struct PreflightedProviderComponentCapsule")
            .nth(1)
            .and_then(|source| {
                source
                    .split("impl PreflightedProviderComponentCapsule")
                    .next()
            })
            .expect("private provider component capsule is missing");
        for retained in [
            "grant:",
            "worker_context:",
            "manifest_authorization:",
            "plugin:",
            "generation:",
            "node_id:",
            "plugin_host:",
            "_lease:",
        ] {
            assert!(fields.contains(retained));
        }
        assert!(!fields.contains("pub grant:"));
        assert!(!fields.contains("pub worker_context:"));
        assert!(!fields.contains("pub manifest_authorization:"));
        let capsule = source
            .split("impl PreflightedProviderComponentCapsule")
            .nth(1)
            .and_then(|source| source.split("#[cfg(test)]").next())
            .expect("private provider component capsule implementation is missing");
        let constructor_signature = capsule
            .split("fn new(")
            .nth(1)
            .and_then(|source| source.split(") -> Result").next())
            .expect("private capsule constructor signature is missing");
        assert_eq!(constructor_signature.matches("worker_context").count(), 1);
        assert_eq!(
            constructor_signature
                .matches("WorkerProviderInvocationContext")
                .count(),
            1
        );
        assert!(capsule.contains("let plugin = host.installed_plugin"));
        assert!(capsule.contains("let generation = host.verified_generation"));
        assert!(capsule.contains("let deployment = generation.worker_deployment_plan"));
        let compact_capsule = capsule.split_whitespace().collect::<String>();
        assert!(compact_capsule.contains("plugin.manifest().nodes.iter().any"));
        assert!(compact_capsule.contains(
            "grant.preflight_installed_component(&worker_context,&deployment,worker_start,manifest_authorization.clone(),)"
        ));
        assert!(capsule.contains(&call));
        assert!(capsule.contains("&worker_context"));
        let ordered = [
            "let plugin = host.installed_plugin",
            "let generation = host.verified_generation",
            "let deployment = generation.worker_deployment_plan",
            "node.id == node_id.as_ref()",
            call.as_str(),
            "let lease = host.begin_invocation_lease",
            "Ok(Self",
        ];
        let mut previous = 0;
        for marker in ordered {
            let position = capsule
                .find(marker)
                .expect("capsule ordering marker is missing");
            assert!(position >= previous);
            previous = position;
        }
        for forbidden in [
            "pub fn",
            "begin_invocation(",
            "get_input_state",
            "read_scalar_input",
            "take_input",
            "read_handle",
            "instantiate_component",
            "create_node",
            ".invoke(",
        ] {
            assert!(!capsule.contains(forbidden));
        }
    }

    #[test]
    fn every_legacy_component_entrypoint_uses_the_early_provider_v2_guard() {
        let source = include_str!("component_host.rs");
        assert!(source.matches("require_legacy_invocation_route()?").count() >= 4);
        for function in [
            "pub fn prepare_plugin_invocation",
            "pub fn prepare_provider_invocation",
            "pub fn execute_plugin",
            "pub fn invoke(",
        ] {
            let body = source
                .split(function)
                .nth(1)
                .and_then(|source| source.split("\n    pub fn ").next())
                .expect("legacy component entrypoint is missing");
            let guard = body
                .find("require_legacy_invocation_route")
                .expect("provider-v2 legacy-route guard is missing");
            for exposure in [
                "begin_invocation(",
                "instantiate_component(",
                "execute_prepared_plugin(",
                ".invoke(",
            ] {
                if let Some(position) = body.find(exposure) {
                    assert!(
                        guard < position,
                        "{function} exposes state before its v2 guard"
                    );
                }
            }
        }
    }

    #[test]
    fn provider_v2_prepared_invocation_has_one_consuming_selector_free_execution_split() {
        let source = include_str!("component_host.rs");
        let prepared = source
            .split("struct PreparedProviderV2Invocation")
            .nth(1)
            .and_then(|source| {
                source
                    .split("impl PreflightedProviderComponentCapsule")
                    .next()
            })
            .expect("prepared provider-v2 invocation fields are missing");
        for retained in [
            "grant:",
            "worker_context:",
            "manifest_authorization:",
            "plugin:",
            "generation:",
            "node_id:",
            "route:",
            "runtime:",
            "cancellation:",
            "plugin_host:",
            "_lease:",
        ] {
            assert!(
                prepared.contains(retained),
                "missing retained field {retained}"
            );
        }
        assert!(!prepared.contains("Clone"));

        let execution = source
            .split("fn into_execution(")
            .nth(1)
            .and_then(|source| source.split("impl ProviderV2ComponentInvocation").next())
            .expect("consuming provider-v2 execution split is missing");
        let signature = execution
            .split(") -> Result")
            .next()
            .expect("execution split signature is missing");
        for selector in [
            "CompiledPlugin",
            "WasmPluginInstance",
            "InvocationHost",
            "WorkerProviderInvocationContext",
            "ProviderManifestAuthorizationV2",
            "node_id",
            "generation",
            "plugin",
            "runtime",
        ] {
            assert!(
                !signature.contains(selector),
                "caller can replace {selector}"
            );
        }
        let compact = execution.split_whitespace().collect::<String>();
        assert!(compact.contains("self.plugin_host.begin_invocation(self.plugin.manifest(),self.plugin.authorization(),self.node_id.as_ref()"));
        assert!(compact.contains("&self.plugin.inner.compiled,invocation,self.runtime"));

        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("component-host production source must exist");
        let host_source = include_str!("comfy_plugin_host.rs");
        let production_host_source = host_source
            .split("#[cfg(test)]")
            .next()
            .expect("plugin-host production source must exist");
        assert_eq!(
            production_source
                .matches("instantiate_provider_component_v2(")
                .count()
                + production_host_source
                    .matches("instantiate_provider_component_v2(")
                    .count(),
            2,
            "v2 instantiation must have one definition and one production callsite"
        );
        assert!(
            production_host_source.contains("pub(crate) fn instantiate_provider_component_v2(")
        );
        assert!(!production_host_source.contains("pub fn instantiate_provider_component_v2("));
    }
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

    pub fn provider_binding_sha256(&self) -> Option<&str> {
        self.plugin
            .manifest()
            .provider_binding
            .as_ref()
            .map(|binding| binding.bindings_sha256.as_str())
    }

    pub fn provider_price_badge(&self) -> Option<&NativeSchemaValue> {
        self.provider_price_badge.as_ref()
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

    fn execute_provider<'a>(
        &'a self,
        invocation: PreparedPluginInvocation,
    ) -> Pin<
        Box<dyn Future<Output = Result<ProviderInvocationResult, ComponentHostError>> + Send + 'a>,
    > {
        Box::pin(async move {
            invocation.context.cancellation.check().map_err(|_| {
                PluginError::Invocation(comfy_plugin_sdk::InvocationError::Cancelled)
            })?;
            let provider_request = invocation
                .worker_invocation
                .provider_request()
                .ok_or_else(|| {
                    ComponentHostError::ExecutionBoundary(
                        "provider invocation omitted its bounded request".to_owned(),
                    )
                })?
                .to_vec();
            let host_invocation = self.plugin_host.begin_invocation(
                &invocation.plugin.inner.manifest,
                &invocation.plugin.inner.authorization,
                invocation.worker_invocation.node_id(),
                invocation.worker_invocation.inputs().clone(),
                self.services.clone(),
                invocation.context.cancellation.clone(),
            )?;
            let instance = self
                .plugin_host
                .instantiate_component(&invocation.plugin.inner.compiled, host_invocation)?;
            instance
                .invoke_provider(invocation.worker_invocation.node_id(), &provider_request)
                .map_err(ComponentHostError::from)
        })
    }
}

struct ComponentHostRouterState {
    host: ComponentHost,
    extension_store_replay_snapshot: Vec<InstalledComponent>,
    next_generation: u64,
    bundle_subscribers: Vec<async_channel::Sender<comfy_runtime::NativeExecutionRegistryBundle>>,
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
                bundle_subscribers: Vec::new(),
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
        host.synchronize_components_at_generation(Vec::new(), initial_generation)?;
        Ok(Self {
            state: Arc::new(Mutex::new(ComponentHostRouterState {
                host,
                extension_store_replay_snapshot: Vec::new(),
                next_generation: initial_generation,
                bundle_subscribers: Vec::new(),
            })),
        })
    }

    pub fn current(&self) -> Result<ComponentHost, ComponentHostError> {
        self.state
            .lock()
            .map(|state| state.host.clone())
            .map_err(|_| ComponentHostError::StateUnavailable)
    }

    pub fn active_execution_registry_bundle(
        &self,
    ) -> Result<comfy_runtime::NativeExecutionRegistryBundle, ComponentHostError> {
        self.current()?.execution_registry_bundle()
    }

    pub fn subscribe_execution_registry_bundles(
        &self,
    ) -> Result<
        async_channel::Receiver<comfy_runtime::NativeExecutionRegistryBundle>,
        ComponentHostError,
    > {
        let bundle = self.active_execution_registry_bundle()?;
        let (sender, receiver) = async_channel::bounded(8);
        sender
            .try_send(bundle)
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        self.state
            .lock()
            .map_err(|_| ComponentHostError::StateUnavailable)?
            .bundle_subscribers
            .push(sender);
        Ok(receiver)
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

pub struct ComponentHostProviderInvocationAuthority {
    host: ComponentHost,
    broker: PluginCapabilityBroker,
    principal_id: String,
    receipt_issuer: Arc<ProviderResultReceiptIssuer>,
    receipt_lifetime: Duration,
    cost_authority: Option<Arc<dyn ProviderCostAuthorizationAuthority>>,
}

impl ComponentHostProviderInvocationAuthority {
    pub fn new(
        host: ComponentHost,
        broker: PluginCapabilityBroker,
        principal_id: impl Into<String>,
        receipt_issuer: Arc<ProviderResultReceiptIssuer>,
        receipt_lifetime: Duration,
    ) -> Result<Self, PluginServiceError> {
        let principal_id = principal_id.into();
        if principal_id.is_empty() || receipt_lifetime.is_zero() {
            return Err(PluginServiceError::ProviderResultReceiptAuthorityDenied);
        }
        Ok(Self {
            host,
            broker,
            principal_id,
            receipt_issuer,
            receipt_lifetime,
            cost_authority: None,
        })
    }

    pub fn with_cost_authority(
        mut self,
        cost_authority: Arc<dyn ProviderCostAuthorizationAuthority>,
    ) -> Self {
        self.cost_authority = Some(cost_authority);
        self
    }
}

impl NativeProviderInvocationAuthority for ComponentHostProviderInvocationAuthority {
    fn begin(
        &self,
        scope: NativeProviderInvocationScope,
    ) -> Result<PluginCapabilityInvocation, PluginServiceError> {
        let state = self
            .host
            .inner
            .state
            .read()
            .map_err(|_| PluginServiceError::ProviderResultReceiptAuthorityDenied)?;
        let deployment = state
            .generation
            .worker_deployment_plan()
            .map_err(|_| PluginServiceError::ProviderResultReceiptAuthorityDenied)?;
        if deployment.begin().generation().get() != scope.start.registry_generation
            || deployment.begin().registry_digest_sha256().as_str()
                != scope.start.registry_digest_sha256
            || scope.node_id.0 != scope.start.node_id
        {
            return Err(PluginServiceError::ProviderResultReceiptAuthorityDenied);
        }
        let descriptor = deployment
            .begin()
            .components()
            .iter()
            .find(|descriptor| descriptor.extension_id() == scope.start.extension_id)
            .ok_or(PluginServiceError::ProviderResultReceiptAuthorityDenied)?;
        if descriptor.extension_version() != scope.start.extension_version
            || descriptor.plugin_identifier() != scope.start.plugin_identifier
            || descriptor.plugin_version() != scope.start.plugin_version
            || descriptor.manifest_digest_sha256().as_str() != scope.start.manifest_digest_sha256
            || descriptor.component_digest_sha256().as_str() != scope.start.component_digest_sha256
            || descriptor.authorization_generation().as_str()
                != scope.start.authorization_generation_sha256
        {
            return Err(PluginServiceError::ProviderResultReceiptAuthorityDenied);
        }
        let plugin = state
            .by_extension
            .get(scope.start.extension_id.as_str())
            .ok_or(PluginServiceError::ProviderResultReceiptAuthorityDenied)?;
        let price_badge = state
            .registry
            .descriptor(&scope.start.node_id)
            .and_then(|descriptor| descriptor.source_schema.as_ref())
            .and_then(|schema| schema.node.price_badge.clone());
        let binding_set = plugin
            .manifest()
            .provider_binding
            .as_ref()
            .ok_or(PluginServiceError::ProviderResultReceiptAuthorityDenied)?;
        if binding_set.bindings_sha256 != scope.start.binding_set_sha256
            || !binding_set
                .bindings
                .iter()
                .any(|claim| claim.node_id == scope.start.node_id)
            || scope.start.maximum_response_bytes == 0
            || scope.start.maximum_response_bytes
                > self.host.inner.invocation_maximum_response_bytes
        {
            return Err(PluginServiceError::ProviderResultReceiptAuthorityDenied);
        }
        let authority = ProviderResultReceiptAuthority::new(
            &self.principal_id,
            &scope.start.compiled_plan_sha256,
            &scope.start.binding_set_sha256,
            self.receipt_issuer.clone(),
            self.receipt_lifetime,
        )
        .map_err(|_| PluginServiceError::ProviderResultReceiptAuthorityDenied)?;
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(
                self.host.inner.invocation_timeout_milliseconds,
            ))
            .ok_or(PluginServiceError::ProviderResultReceiptAuthorityDenied)?;
        let context = PluginServiceInvocationContext::new_with_principal(
            scope.profile_id,
            scope.prompt_id,
            scope.attempt_id,
            scope.node_id,
            &self.principal_id,
            plugin.authorization().clone(),
            scope.cancellation,
            deadline,
            scope.start.maximum_response_bytes,
        )?
        .with_provider_result_authority(authority)?;
        let context = match price_badge {
            Some(price_badge) => {
                context.with_provider_cost_requirement(price_badge, self.cost_authority.clone())?
            }
            None => context,
        };
        self.broker.begin_invocation(context)
    }
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

    pub fn execution_registry_bundle(
        &self,
    ) -> Result<comfy_runtime::NativeExecutionRegistryBundle, ComponentHostError> {
        let state = self
            .inner
            .state
            .read()
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        let profile_id = uuid::Uuid::parse_str(state.generation.profile_id()).map_err(|_| {
            ComponentHostError::Verification {
                extension_id: Arc::from(COMFY_COMPONENT_ADAPTER_ID),
                message: "verified component profile identity is invalid".to_owned(),
            }
        })?;
        let deployment = state.generation.worker_deployment_plan()?;
        let provider_registry = state.generation.provider_registry_pin()?;
        comfy_runtime::NativeExecutionRegistryBundle::checked(
            comfy_types::ProfileId(profile_id),
            state.registry.clone(),
            deployment,
            provider_registry,
        )
        .map_err(|error| ComponentHostError::Verification {
            extension_id: Arc::from(COMFY_COMPONENT_ADAPTER_ID),
            message: error.to_string(),
        })
    }

    pub(crate) fn prepare_plugin_invocation(
        &self,
        plugin: &InstalledVerifiedPlugin,
        node_id: &str,
        inputs: InvocationInputs,
        context: NodeContext,
    ) -> Result<PreparedPluginInvocation, ComponentHostError> {
        plugin.require_legacy_invocation_route()?;
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
            provider_price_badge: None,
            _lease: lease,
        })
    }

    pub(crate) fn prepare_provider_invocation(
        &self,
        plugin: &InstalledVerifiedPlugin,
        node_id: &str,
        inputs: InvocationInputs,
        provider_request: Vec<u8>,
        context: NodeContext,
    ) -> Result<PreparedPluginInvocation, ComponentHostError> {
        plugin.require_legacy_invocation_route()?;
        let lease = self.begin_invocation_lease(plugin)?;
        let state = self
            .inner
            .state
            .read()
            .map_err(|_| ComponentHostError::StateUnavailable)?;
        let generation = state.generation.clone();
        let provider_price_badge = state
            .registry
            .descriptor(node_id)
            .and_then(|descriptor| descriptor.source_schema.as_ref())
            .and_then(|schema| schema.node.price_badge.clone());
        drop(state);
        let worker_invocation = generation.prepare_worker_provider_invocation(
            plugin.extension_id(),
            node_id,
            inputs,
            provider_request,
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
            provider_price_badge,
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
        plugin.require_legacy_invocation_route()?;
        let prepared = self.prepare_plugin_invocation(plugin, node_id, inputs, context)?;
        self.executor().execute(prepared).await
    }

    pub async fn execute_provider(
        &self,
        plugin: &InstalledVerifiedPlugin,
        node_id: &str,
        inputs: InvocationInputs,
        provider_request: Vec<u8>,
        context: NodeContext,
    ) -> Result<ProviderInvocationResult, ComponentHostError> {
        let prepared =
            self.prepare_provider_invocation(plugin, node_id, inputs, provider_request, context)?;
        self.executor().execute_provider(prepared).await
    }

    pub fn invoke(
        &self,
        plugin: &InstalledVerifiedPlugin,
        node_id: &str,
        inputs: InvocationInputs,
        cancellation: CancellationToken,
    ) -> Result<InvocationResult, ComponentHostError> {
        plugin.require_legacy_invocation_route()?;
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
            let (manifest, provider_manifest_v2) =
                parse_component_manifest(component.manifest_bytes()).map_err(|error| {
                    ComponentHostError::InvalidManifest {
                        extension_id: extension_id.clone(),
                        message: error.to_string(),
                    }
                })?;
            let binding = InstalledComponentBinding::checked(&component, &manifest)?;
            if !plugin_ids.insert(manifest.identifier.clone()) {
                return Err(ComponentHostError::DuplicatePlugin(manifest.identifier));
            }
            let provider_authorization_v2 = provider_manifest_v2
                .as_ref()
                .map(|provider_manifest| {
                    self.inner.trust_policy.authorize_provider_manifest_v2(
                        provider_manifest,
                        &self.inner.permission_policy,
                    )
                })
                .transpose()
                .map_err(|error| ComponentHostError::Verification {
                    extension_id: extension_id.clone(),
                    message: error.to_string(),
                })?;
            let authorization = match &provider_authorization_v2 {
                Some(authorization) => authorization.authorization().clone(),
                None => self
                    .inner
                    .trust_policy
                    .authorize_manifest(&manifest, &self.inner.permission_policy)
                    .map_err(|error| ComponentHostError::Verification {
                        extension_id: extension_id.clone(),
                        message: error.to_string(),
                    })?,
            };
            let compiled = match (&provider_manifest_v2, &provider_authorization_v2) {
                (Some(provider_manifest), Some(provider_authorization)) => {
                    self.inner.plugin_host.compile_provider_component_v2(
                        component.component_bytes(),
                        provider_manifest,
                        provider_authorization,
                    )
                }
                (None, None) => self.inner.plugin_host.compile_component(
                    component.component_bytes(),
                    &manifest,
                    &authorization,
                ),
                _ => Err(PluginError::ProviderRuntimeActivationDenied),
            }
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
                    provider_manifest_v2: provider_manifest_v2.map(Arc::new),
                    provider_authorization_v2: provider_authorization_v2.map(Arc::new),
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
            verified_generation.clone(),
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

fn parse_component_manifest(
    bytes: &[u8],
) -> Result<(PluginManifest, Option<ProviderPluginManifestV2>), serde_json::Error> {
    if let Ok(provider_manifest) = serde_json::from_slice::<ProviderPluginManifestV2>(bytes) {
        return Ok((provider_manifest.manifest.clone(), Some(provider_manifest)));
    }
    serde_json::from_slice(bytes).map(|manifest| (manifest, None))
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
                if !state.bundle_subscribers.is_empty() {
                    let bundle = state.host.execution_registry_bundle()?;
                    state.bundle_subscribers.retain(|subscriber| {
                        match subscriber.try_send(bundle.clone()) {
                            Ok(()) | Err(async_channel::TrySendError::Full(_)) => true,
                            Err(async_channel::TrySendError::Closed(_)) => false,
                        }
                    });
                }
                Ok::<(), ComponentHostError>(())
            })
            .await
            .map_err(|error| error.to_string())
        })
    }
}
