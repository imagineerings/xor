use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use comfy_plugin_host::{
    CapabilityServiceContext, ComponentLimits, InvocationResult, PluginCapabilityServices,
    PluginError, PluginHost, WorkerPluginInvocation,
};
use comfy_plugin_sdk::{CapabilityKind, InvocationError, ModelValue, PluginManifest};
use comfy_runtime::{
    AssetIdentity, NativeProviderRegistryPin, PluginAuthorization, PluginAuthorizationVerifier,
    PluginServiceWireFailure, PluginServiceWireRequest, PluginServiceWireResponse, SecretId,
};
use comfy_tensor::CancellationToken;
use comfy_types::{
    MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES, MAX_WORKER_PLUGIN_RESULT_BYTES, ProfileId,
    WorkerPluginExecutionFailure, WorkerPluginExecutionOutcome,
    WorkerRegistryDeploymentRejectionReason,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::AssembledWorkerRegistry;

const CAPABILITY_BRIDGE_POLL_INTERVAL: Duration = Duration::from_millis(5);

pub(crate) struct WorkerCapabilityBridgeRequest {
    pub call_id: u64,
    pub request: Vec<u8>,
    pub response_sender: async_channel::Sender<Vec<u8>>,
}

#[derive(Clone)]
pub(crate) struct WorkerCapabilityBridge {
    sender: async_channel::Sender<WorkerCapabilityBridgeRequest>,
    next_call_id: Arc<AtomicU64>,
}

impl WorkerCapabilityBridge {
    pub fn new(sender: async_channel::Sender<WorkerCapabilityBridgeRequest>) -> Self {
        Self {
            sender,
            next_call_id: Arc::new(AtomicU64::new(1)),
        }
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "the private worker process has no GPUI dispatcher; its blocking component thread must cooperatively poll the async IPC bridge"
    )]
    fn call(
        &self,
        request: PluginServiceWireRequest,
        context: &CapabilityServiceContext,
        kind: CapabilityKind,
        scope: &str,
    ) -> Result<PluginServiceWireResponse, InvocationError> {
        context.check_active()?;
        let request = request.to_bytes().map_err(|_| {
            InvocationError::InvalidCapabilityRequest(
                "plugin capability request cannot be represented".to_owned(),
            )
        })?;
        if request.len() > MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES {
            return Err(InvocationError::InvalidCapabilityRequest(
                "plugin capability request exceeds the worker transport bound".to_owned(),
            ));
        }
        let call_id = self.next_call_id.fetch_add(1, Ordering::Relaxed);
        if call_id == 0 {
            return Err(InvocationError::HostFailure(
                "plugin capability call identifier overflowed".to_owned(),
            ));
        }
        let (response_sender, response_receiver) = async_channel::bounded(1);
        let mut bridge_request = WorkerCapabilityBridgeRequest {
            call_id,
            request,
            response_sender,
        };
        loop {
            context.check_active()?;
            match self.sender.try_send(bridge_request) {
                Ok(()) => break,
                Err(async_channel::TrySendError::Full(returned)) => {
                    bridge_request = returned;
                    smol::block_on(async_io::Timer::after(CAPABILITY_BRIDGE_POLL_INTERVAL));
                }
                Err(async_channel::TrySendError::Closed(_)) => {
                    return Err(InvocationError::HostFailure(
                        "private worker capability bridge is unavailable".to_owned(),
                    ));
                }
            }
        }
        let response = loop {
            context.check_active()?;
            let received = smol::block_on(smol::future::race(
                async { Some(response_receiver.recv().await) },
                async {
                    async_io::Timer::after(CAPABILITY_BRIDGE_POLL_INTERVAL).await;
                    None
                },
            ));
            match received {
                Some(Ok(response)) => break response,
                Some(Err(_)) => {
                    return Err(InvocationError::HostFailure(
                        "private worker capability response was lost".to_owned(),
                    ));
                }
                None => {}
            }
        };
        let transport_maximum = context
            .maximum_response_bytes()
            .saturating_add(64 * 1024)
            .min(u64::try_from(MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES).unwrap_or(u64::MAX));
        let response = PluginServiceWireResponse::from_bytes(&response, transport_maximum)
            .map_err(|_| {
                InvocationError::HostFailure(
                    "private worker capability response is malformed".to_owned(),
                )
            })?;
        context.check_active()?;
        match response {
            PluginServiceWireResponse::Failure(failure) => {
                Err(map_wire_failure(failure, kind, scope))
            }
            response => Ok(response),
        }
    }
}

impl PluginCapabilityServices for WorkerCapabilityBridge {
    fn read_asset(
        &self,
        identity: &AssetIdentity,
        context: &CapabilityServiceContext,
    ) -> Result<Vec<u8>, InvocationError> {
        let reference = identity.to_reference().map_err(|_| {
            InvocationError::InvalidCapabilityRequest(
                "asset reference is not available to this plugin".to_owned(),
            )
        })?;
        let namespace = identity.namespace.locator_type();
        match self.call(
            PluginServiceWireRequest::ReadAsset {
                namespace: namespace.to_owned(),
                asset_reference: reference,
            },
            context,
            CapabilityKind::Filesystem,
            namespace,
        )? {
            PluginServiceWireResponse::Bytes(bytes) => {
                context.validate_response_length(bytes.len())?;
                Ok(bytes)
            }
            _ => Err(invalid_wire_response("asset")),
        }
    }

    fn call_provider(
        &self,
        _provider: &str,
        _endpoint: &str,
        _body: &[u8],
        _secret_id: Option<&SecretId>,
    ) -> Result<Vec<u8>, InvocationError> {
        Err(context_required("provider"))
    }

    fn secret_exists(&self, _identifier: &str) -> Result<bool, InvocationError> {
        Err(context_required("credentials"))
    }

    fn clock_milliseconds(&self, _clock: &str) -> Result<u64, InvocationError> {
        Err(context_required("clock"))
    }

    fn random_bytes(&self, _stream: &str, _length: u32) -> Result<Vec<u8>, InvocationError> {
        Err(context_required("randomness"))
    }

    fn open_model(&self, _identifier: &str) -> Result<ModelValue, InvocationError> {
        Err(context_required("model"))
    }

    fn sanitize_log(&self, _level: &str, _message: &str) -> Result<String, InvocationError> {
        Err(context_required("log"))
    }

    fn call_provider_with_context(
        &self,
        provider: &str,
        endpoint: &str,
        body: &[u8],
        secret_id: Option<&SecretId>,
        context: &CapabilityServiceContext,
    ) -> Result<Vec<u8>, InvocationError> {
        let scope = format!("{provider}|{endpoint}");
        match self.call(
            PluginServiceWireRequest::ExecuteProvider {
                provider: provider.to_owned(),
                endpoint: endpoint.to_owned(),
                secret_id: secret_id.map(|secret_id| secret_id.as_str().to_owned()),
                body: body.to_vec(),
            },
            context,
            CapabilityKind::NetworkProvider,
            &scope,
        )? {
            PluginServiceWireResponse::Bytes(bytes) => {
                context.validate_response_length(bytes.len())?;
                Ok(bytes)
            }
            _ => Err(invalid_wire_response("provider")),
        }
    }

    fn secret_exists_with_context(
        &self,
        identifier: &str,
        context: &CapabilityServiceContext,
    ) -> Result<bool, InvocationError> {
        match self.call(
            PluginServiceWireRequest::CredentialIsPresent {
                secret_id: identifier.to_owned(),
            },
            context,
            CapabilityKind::Secret,
            identifier,
        )? {
            PluginServiceWireResponse::Boolean(exists) => Ok(exists),
            _ => Err(invalid_wire_response("credentials")),
        }
    }

    fn clock_milliseconds_with_context(
        &self,
        clock: &str,
        context: &CapabilityServiceContext,
    ) -> Result<u64, InvocationError> {
        match self.call(
            PluginServiceWireRequest::MonotonicMilliseconds {
                clock_id: clock.to_owned(),
            },
            context,
            CapabilityKind::Clock,
            clock,
        )? {
            PluginServiceWireResponse::TimestampMilliseconds(milliseconds) => Ok(milliseconds),
            _ => Err(invalid_wire_response("clock")),
        }
    }

    fn random_bytes_with_context(
        &self,
        stream: &str,
        length: u32,
        context: &CapabilityServiceContext,
    ) -> Result<Vec<u8>, InvocationError> {
        match self.call(
            PluginServiceWireRequest::RandomBytes {
                stream_id: stream.to_owned(),
                length,
            },
            context,
            CapabilityKind::Randomness,
            stream,
        )? {
            PluginServiceWireResponse::Bytes(bytes) => {
                context.validate_response_length(bytes.len())?;
                Ok(bytes)
            }
            _ => Err(invalid_wire_response("randomness")),
        }
    }

    fn open_model_with_context(
        &self,
        identifier: &str,
        context: &CapabilityServiceContext,
    ) -> Result<ModelValue, InvocationError> {
        match self.call(
            PluginServiceWireRequest::LoadModel {
                model_id: identifier.to_owned(),
            },
            context,
            CapabilityKind::Model,
            identifier,
        )? {
            PluginServiceWireResponse::Model {
                identifier,
                format,
                digest_sha256,
            } => ModelValue::new(identifier, format, digest_sha256).map_err(|_| {
                InvocationError::HostFailure(
                    "canonical model handle cannot be represented by the plugin ABI".to_owned(),
                )
            }),
            _ => Err(invalid_wire_response("model")),
        }
    }

    fn sanitize_log_with_context(
        &self,
        level: &str,
        message: &str,
        context: &CapabilityServiceContext,
    ) -> Result<String, InvocationError> {
        match self.call(
            PluginServiceWireRequest::SanitizeLog {
                level: level.to_owned(),
                message: message.to_owned(),
            },
            context,
            CapabilityKind::SanitizedLog,
            level,
        )? {
            PluginServiceWireResponse::SanitizedLog(message) => {
                context.validate_response_length(message.len())?;
                Ok(message)
            }
            _ => Err(invalid_wire_response("log")),
        }
    }
}

pub(crate) struct WorkerPluginRegistry {
    profile_id: ProfileId,
    generation: comfy_types::WorkerRegistryGeneration,
    registry_digest_sha256: comfy_types::WorkerSha256Digest,
    host: Arc<PluginHost>,
    component_limits: ComponentLimits,
    components: BTreeMap<String, WorkerCompiledPlugin>,
}

struct WorkerCompiledPlugin {
    extension_version: String,
    plugin_identifier: String,
    plugin_version: String,
    manifest_digest_sha256: comfy_types::WorkerSha256Digest,
    component_digest_sha256: comfy_types::WorkerSha256Digest,
    authorization_generation: comfy_types::WorkerSha256Digest,
    manifest: Arc<PluginManifest>,
    authorization: Arc<PluginAuthorization>,
    compiled: Arc<comfy_plugin_host::CompiledPlugin>,
}

impl WorkerPluginRegistry {
    pub fn from_assembled(
        profile_id: ProfileId,
        registry: &AssembledWorkerRegistry,
        component_limits: ComponentLimits,
        authorization_verifier: &PluginAuthorizationVerifier,
    ) -> Result<Self, WorkerPluginRuntimeError> {
        let host = Arc::new(PluginHost::with_configuration(
            component_limits.clone(),
            comfy_plugin_host::DEFAULT_API_FEATURES
                .iter()
                .map(|feature| (*feature).to_owned()),
        )?);
        let mut components = BTreeMap::new();
        for component in registry.components() {
            if !digest_matches(
                component.manifest_bytes(),
                component.manifest_digest_sha256(),
            ) || !digest_matches(
                component.authorization_bytes(),
                component.authorization_generation(),
            ) || !digest_matches(
                component.component_bytes(),
                component.component_digest_sha256(),
            ) {
                return Err(WorkerPluginRuntimeError::InvalidDeployment);
            }
            let manifest: PluginManifest = serde_json::from_slice(component.manifest_bytes())
                .map_err(|_| WorkerPluginRuntimeError::InvalidDeployment)?;
            if manifest.identifier != component.plugin_identifier()
                || manifest.plugin_version.to_string() != component.plugin_version()
                || manifest.digest_sha256 != component.component_digest_sha256().as_str()
            {
                return Err(WorkerPluginRuntimeError::InvalidDeployment);
            }
            let authorization = PluginAuthorization::from_sealed_bytes(
                component.authorization_bytes(),
                &manifest,
                authorization_verifier,
                authorization_verifier.policy_generation(),
                &profile_id.0.to_string(),
            )?;
            if authorization.capabilities().profile_id() != profile_id.0.to_string() {
                return Err(WorkerPluginRuntimeError::InvalidDeployment);
            }
            let compiled =
                host.compile_component(component.component_bytes(), &manifest, &authorization)?;
            let extension_id = component.extension_id().to_owned();
            if components
                .insert(
                    extension_id,
                    WorkerCompiledPlugin {
                        extension_version: component.extension_version().to_owned(),
                        plugin_identifier: component.plugin_identifier().to_owned(),
                        plugin_version: component.plugin_version().to_owned(),
                        manifest_digest_sha256: component.manifest_digest_sha256().clone(),
                        component_digest_sha256: component.component_digest_sha256().clone(),
                        authorization_generation: component.authorization_generation().clone(),
                        manifest: Arc::new(manifest),
                        authorization: Arc::new(authorization),
                        compiled: Arc::new(compiled),
                    },
                )
                .is_some()
            {
                return Err(WorkerPluginRuntimeError::InvalidDeployment);
            }
        }
        Ok(Self {
            profile_id,
            generation: registry.generation(),
            registry_digest_sha256: registry.registry_digest_sha256().clone(),
            host,
            component_limits,
            components,
        })
    }

    pub fn uses_component_limits(&self, component_limits: &ComponentLimits) -> bool {
        &self.component_limits == component_limits
    }

    pub fn matches_provider_registry_pin(&self, pin: &NativeProviderRegistryPin) -> bool {
        let mut binding_digests = self
            .components
            .values()
            .filter_map(|component| {
                component
                    .manifest
                    .provider_binding
                    .as_ref()
                    .map(|binding| binding.bindings_sha256.clone())
            })
            .collect::<Vec<_>>();
        binding_digests.sort();
        binding_digests.dedup();
        self.generation.get() == pin.generation()
            && self.registry_digest_sha256.as_str() == pin.registry_digest_sha256()
            && binding_digests == pin.binding_digests_sha256()
    }

    #[cfg(test)]
    pub(crate) fn empty_for_test(
        profile_id: ProfileId,
        generation: comfy_types::WorkerRegistryGeneration,
        registry_digest_sha256: comfy_types::WorkerSha256Digest,
    ) -> Result<Self, WorkerPluginRuntimeError> {
        let component_limits = ComponentLimits::default();
        let host = PluginHost::with_configuration(
            component_limits.clone(),
            comfy_plugin_host::DEFAULT_API_FEATURES
                .iter()
                .map(|feature| (*feature).to_owned()),
        )?;
        Ok(Self {
            profile_id,
            generation,
            registry_digest_sha256,
            host: Arc::new(host),
            component_limits,
            components: BTreeMap::new(),
        })
    }

    pub fn execute(
        &self,
        invocation: WorkerPluginInvocation,
        bridge: Arc<dyn PluginCapabilityServices>,
        cancellation: CancellationToken,
    ) -> Result<InvocationResult, WorkerPluginRuntimeError> {
        if invocation.registry_generation() != self.generation
            || invocation.registry_digest_sha256() != &self.registry_digest_sha256
        {
            return Err(WorkerPluginRuntimeError::StaleGeneration);
        }
        let component = self
            .components
            .get(invocation.extension_id())
            .ok_or(WorkerPluginRuntimeError::MissingComponent)?;
        if invocation.component_digest_sha256() != &component.component_digest_sha256
            || invocation.extension_version() != component.extension_version
            || invocation.plugin_identifier() != component.plugin_identifier
            || invocation.plugin_version() != component.plugin_version
            || invocation.manifest_digest_sha256() != &component.manifest_digest_sha256
            || invocation.authorization_generation() != &component.authorization_generation
            || component.authorization.capabilities().profile_id() != self.profile_id.0.to_string()
        {
            return Err(WorkerPluginRuntimeError::StaleGeneration);
        }
        let node_id = invocation.node_id().to_owned();
        let invocation_host = self.host.begin_invocation(
            &component.manifest,
            &component.authorization,
            &node_id,
            invocation.into_inputs(),
            bridge,
            cancellation,
        )?;
        let mut instance = self
            .host
            .instantiate_component(&component.compiled, invocation_host)?;
        let node = instance.create_node(&node_id)?;
        if let Err(error) = instance.invoke(node) {
            instance.abort();
            return Err(error.into());
        }
        if let Err(error) = instance.drop_node(node) {
            instance.abort();
            return Err(error.into());
        }
        instance.finish().map_err(Into::into)
    }
}

fn digest_matches(bytes: &[u8], expected: &comfy_types::WorkerSha256Digest) -> bool {
    let actual: [u8; 32] = Sha256::digest(bytes).into();
    actual == expected.bytes()
}

pub(crate) fn encode_plugin_outcome(
    result: Result<InvocationResult, WorkerPluginRuntimeError>,
) -> WorkerPluginExecutionOutcome {
    match result {
        Ok(result) => match serde_json::to_vec(&result) {
            Ok(bytes) if !bytes.is_empty() && bytes.len() <= MAX_WORKER_PLUGIN_RESULT_BYTES => {
                WorkerPluginExecutionOutcome::Succeeded(bytes)
            }
            Ok(_) | Err(_) => {
                WorkerPluginExecutionOutcome::Failed(WorkerPluginExecutionFailure::HostFailure)
            }
        },
        Err(error) => WorkerPluginExecutionOutcome::Failed(error.failure()),
    }
}

#[derive(Debug, Error)]
pub(crate) enum WorkerPluginRuntimeError {
    #[error("worker component deployment is invalid")]
    InvalidDeployment,
    #[error("worker plugin invocation targets a stale component generation")]
    StaleGeneration,
    #[error("worker plugin invocation targets an unavailable component")]
    MissingComponent,
    #[error(transparent)]
    Trust(#[from] comfy_runtime::TrustError),
    #[error(transparent)]
    Plugin(#[from] PluginError),
}

impl WorkerPluginRuntimeError {
    pub(crate) const fn deployment_rejection_reason(
        &self,
    ) -> WorkerRegistryDeploymentRejectionReason {
        match self {
            Self::InvalidDeployment | Self::StaleGeneration | Self::MissingComponent => {
                WorkerRegistryDeploymentRejectionReason::InvalidCandidate
            }
            Self::Trust(_) => WorkerRegistryDeploymentRejectionReason::InvalidAuthorization,
            Self::Plugin(_) => WorkerRegistryDeploymentRejectionReason::ComponentCompilationFailed,
        }
    }

    fn failure(&self) -> WorkerPluginExecutionFailure {
        match self {
            Self::StaleGeneration | Self::MissingComponent | Self::InvalidDeployment => {
                WorkerPluginExecutionFailure::InvalidInvocation
            }
            Self::Plugin(PluginError::WasmTrap(diagnostic)) => WorkerPluginExecutionFailure::Trap {
                diagnostic: diagnostic.clone(),
            },
            Self::Plugin(PluginError::Invocation(InvocationError::Cancelled)) => {
                WorkerPluginExecutionFailure::Cancelled
            }
            Self::Plugin(PluginError::Invocation(InvocationError::TimedOut)) => {
                WorkerPluginExecutionFailure::TimedOut
            }
            Self::Plugin(PluginError::Invocation(InvocationError::CapabilityDenied { .. })) => {
                WorkerPluginExecutionFailure::CapabilityDenied
            }
            Self::Trust(_) | Self::Plugin(_) => WorkerPluginExecutionFailure::HostFailure,
        }
    }
}

fn map_wire_failure(
    failure: PluginServiceWireFailure,
    kind: CapabilityKind,
    scope: &str,
) -> InvocationError {
    match failure {
        PluginServiceWireFailure::CapabilityDenied | PluginServiceWireFailure::ProviderDenied => {
            InvocationError::CapabilityDenied {
                kind,
                scope: scope.to_owned(),
            }
        }
        PluginServiceWireFailure::Cancelled => InvocationError::Cancelled,
        PluginServiceWireFailure::DeadlineExceeded => InvocationError::TimedOut,
        PluginServiceWireFailure::ResponseTooLarge => InvocationError::QuotaExceeded {
            kind,
            limit: "response-byte".to_owned(),
        },
        PluginServiceWireFailure::InvalidRequest => InvocationError::InvalidCapabilityRequest(
            "canonical plugin capability request was rejected".to_owned(),
        ),
        PluginServiceWireFailure::ServiceUnavailable
        | PluginServiceWireFailure::ActuatorFailed
        | PluginServiceWireFailure::RandomnessFailed
        | PluginServiceWireFailure::InvocationFailed => {
            InvocationError::HostFailure("canonical plugin capability service failed".to_owned())
        }
    }
}

fn invalid_wire_response(service: &str) -> InvocationError {
    InvocationError::HostFailure(format!(
        "canonical {service} service returned an incompatible response"
    ))
}

fn context_required(service: &str) -> InvocationError {
    InvocationError::HostFailure(format!(
        "canonical {service} service requires invocation context"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_registry_pin_requires_the_exact_committed_generation() {
        let registry = WorkerPluginRegistry::empty_for_test(
            ProfileId(uuid::Uuid::nil()),
            comfy_types::WorkerRegistryGeneration::new(2).expect("generation is non-zero"),
            comfy_types::WorkerSha256Digest::new("a".repeat(64)).expect("registry digest is valid"),
        )
        .expect("empty worker registry is valid");
        let pin = NativeProviderRegistryPin::checked(2, "a".repeat(64), vec!["b".repeat(64)])
            .expect("provider registry pin is valid");

        assert!(
            !registry.matches_provider_registry_pin(&pin),
            "an empty committed registry cannot satisfy a provider binding pin"
        );
    }

    #[test]
    fn stale_worker_generation_fails_before_component_or_capability_access() {
        let current_digest = comfy_types::WorkerSha256Digest::new("b".repeat(64))
            .expect("current registry digest is valid");
        let component_limits = ComponentLimits::default();
        let host = PluginHost::with_configuration(
            component_limits.clone(),
            comfy_plugin_host::DEFAULT_API_FEATURES
                .iter()
                .map(|feature| (*feature).to_owned()),
        )
        .expect("no-WASI plugin host is valid");
        let registry = WorkerPluginRegistry {
            profile_id: ProfileId(uuid::Uuid::nil()),
            generation: comfy_types::WorkerRegistryGeneration::new(2)
                .expect("generation is non-zero"),
            registry_digest_sha256: current_digest,
            host: Arc::new(host),
            component_limits: component_limits.clone(),
            components: BTreeMap::new(),
        };
        let invocation_bytes = serde_json::to_vec(&serde_json::json!({
            "registry_generation": 1,
            "registry_digest_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "extension_id": "test.echo-extension",
            "extension_version": "1.0.0",
            "plugin_identifier": "test.echo-plugin",
            "plugin_version": "1.0.0",
            "manifest_digest_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "component_digest_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "authorization_generation": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "node_id": "echo",
            "inputs": { "values": {} },
            "timeout_milliseconds": 1_000,
            "maximum_response_bytes": 1_024,
            "component_limits": component_limits
        }))
        .expect("worker invocation fixture serializes");
        let invocation = WorkerPluginInvocation::from_bytes(&invocation_bytes)
            .expect("worker invocation fixture is valid");
        let round_trip = invocation
            .to_bytes()
            .and_then(|bytes| WorkerPluginInvocation::from_bytes(&bytes))
            .expect("tagged invocation values round trip through the bounded JSON DTO");
        assert_eq!(round_trip, invocation);

        let (sender, _receiver) = async_channel::bounded(1);
        let result = registry.execute(
            invocation,
            Arc::new(WorkerCapabilityBridge::new(sender)),
            CancellationToken::default(),
        );
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("stale invocation unexpectedly executed"),
        };
        assert!(matches!(error, WorkerPluginRuntimeError::StaleGeneration));
        assert_eq!(
            error.failure(),
            WorkerPluginExecutionFailure::InvalidInvocation
        );
    }
}
