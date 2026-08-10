use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use clock::SystemClock;
use comfy_model::{
    LoadedModel, ModelFormat, ModelFormatError, ModelLoadAccounting, ModelStore, ModelStoreError,
};
use comfy_tensor::{
    CancellationToken, RetryRngPolicy, RngAlgorithm, RngCheckpoint, RngProfileVersion, RngStream,
    RngStreamAddress, RngTransaction,
};
use comfy_types::{AttemptId, NodeId, ProfileId, PromptId};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    AssetError, AssetNamespace, AssetOperation, AuthorizedProviderRequest, Capability,
    PluginAuthorization, ProviderCostAcceptance, ProviderCostAcceptanceScope,
    ProviderCostAcceptanceVerifier, ProviderCostNonce, ProviderPolicy, ProviderPriceBound,
    SecretId, SecretValue, SharedAssetService,
};

pub const MAX_PLUGIN_SERVICE_REQUEST_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_PLUGIN_SERVICE_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PLUGIN_SERVICE_IDENTITY_BYTES: usize = 1_024;
const MAX_CONSUMED_PROVIDER_COST_NONCES: usize = 65_536;
const RNG_DEADLINE_CHECK_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum PluginServiceWireRequest {
    ReadAsset {
        namespace: String,
        asset_reference: String,
    },
    ExecuteProvider {
        provider: String,
        endpoint: String,
        secret_id: Option<String>,
        body: Vec<u8>,
    },
    CredentialIsPresent {
        secret_id: String,
    },
    MonotonicMilliseconds {
        clock_id: String,
    },
    RandomBytes {
        stream_id: String,
        length: u32,
    },
    LoadModel {
        model_id: String,
    },
    SanitizeLog {
        level: String,
        message: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginServiceWireFailure {
    InvalidRequest,
    CapabilityDenied,
    Cancelled,
    DeadlineExceeded,
    ResponseTooLarge,
    ServiceUnavailable,
    ProviderDenied,
    ActuatorFailed,
    RandomnessFailed,
    InvocationFailed,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum PluginServiceWireResponse {
    Bytes(Vec<u8>),
    Boolean(bool),
    TimestampMilliseconds(u64),
    Model {
        identifier: String,
        format: String,
        digest_sha256: String,
    },
    SanitizedLog(String),
    Failure(PluginServiceWireFailure),
}

impl PluginServiceWireRequest {
    pub fn to_bytes(&self) -> Result<Vec<u8>, PluginServiceError> {
        let bytes =
            postcard::to_stdvec(self).map_err(|_| PluginServiceError::InvalidWirePayload)?;
        check_request_size(bytes.len())?;
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PluginServiceError> {
        if bytes.is_empty() {
            return Err(PluginServiceError::InvalidWirePayload);
        }
        check_request_size(bytes.len())?;
        postcard::from_bytes(bytes).map_err(|_| PluginServiceError::InvalidWirePayload)
    }
}

impl PluginServiceWireResponse {
    pub fn to_bytes(&self, maximum: u64) -> Result<Vec<u8>, PluginServiceError> {
        let bytes =
            postcard::to_stdvec(self).map_err(|_| PluginServiceError::InvalidWirePayload)?;
        check_response_size(bytes.len(), maximum)?;
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8], maximum: u64) -> Result<Self, PluginServiceError> {
        if bytes.is_empty() {
            return Err(PluginServiceError::InvalidWirePayload);
        }
        check_response_size(bytes.len(), maximum)?;
        postcard::from_bytes(bytes).map_err(|_| PluginServiceError::InvalidWirePayload)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PluginRngPolicy {
    profile: RngProfileVersion,
    algorithm: RngAlgorithm,
    seed: u64,
}

impl PluginRngPolicy {
    pub fn new(profile: RngProfileVersion, algorithm: RngAlgorithm, seed: u64) -> Self {
        Self {
            profile,
            algorithm,
            seed,
        }
    }

    pub fn profile(&self) -> RngProfileVersion {
        self.profile
    }

    pub fn algorithm(&self) -> RngAlgorithm {
        self.algorithm
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }
}

#[derive(Clone, Debug)]
pub struct PluginServiceInvocationContext {
    profile_id: ProfileId,
    prompt_id: PromptId,
    attempt_id: AttemptId,
    node_id: NodeId,
    principal_id: Option<String>,
    authorization: PluginAuthorization,
    cancellation: CancellationToken,
    deadline: Instant,
    maximum_response_bytes: u64,
}

impl PluginServiceInvocationContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        profile_id: ProfileId,
        prompt_id: PromptId,
        attempt_id: AttemptId,
        node_id: NodeId,
        authorization: PluginAuthorization,
        cancellation: CancellationToken,
        deadline: Instant,
        maximum_response_bytes: u64,
    ) -> Result<Self, PluginServiceError> {
        Self::checked(
            profile_id,
            prompt_id,
            attempt_id,
            node_id,
            None,
            authorization,
            cancellation,
            deadline,
            maximum_response_bytes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_principal(
        profile_id: ProfileId,
        prompt_id: PromptId,
        attempt_id: AttemptId,
        node_id: NodeId,
        principal_id: impl Into<String>,
        authorization: PluginAuthorization,
        cancellation: CancellationToken,
        deadline: Instant,
        maximum_response_bytes: u64,
    ) -> Result<Self, PluginServiceError> {
        let principal_id = principal_id.into();
        if principal_id.is_empty()
            || principal_id.len() > MAX_PLUGIN_SERVICE_IDENTITY_BYTES
            || principal_id != principal_id.trim()
            || principal_id.chars().any(char::is_control)
        {
            return Err(PluginServiceError::InvalidIdentity { kind: "principal" });
        }
        Self::checked(
            profile_id,
            prompt_id,
            attempt_id,
            node_id,
            Some(principal_id),
            authorization,
            cancellation,
            deadline,
            maximum_response_bytes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn checked(
        profile_id: ProfileId,
        prompt_id: PromptId,
        attempt_id: AttemptId,
        node_id: NodeId,
        principal_id: Option<String>,
        authorization: PluginAuthorization,
        cancellation: CancellationToken,
        deadline: Instant,
        maximum_response_bytes: u64,
    ) -> Result<Self, PluginServiceError> {
        validate_identity("node", &node_id.0)?;
        let canonical_profile_id = profile_id.0.to_string();
        if authorization.capabilities().profile_id() != canonical_profile_id {
            return Err(PluginServiceError::ProfileMismatch);
        }
        if authorization.capabilities().subject_id() != authorization.plugin_id() {
            return Err(PluginServiceError::AuthorizationIdentityMismatch);
        }
        if maximum_response_bytes == 0 || maximum_response_bytes > MAX_PLUGIN_SERVICE_RESPONSE_BYTES
        {
            return Err(PluginServiceError::InvalidResponseLimit {
                maximum: MAX_PLUGIN_SERVICE_RESPONSE_BYTES,
            });
        }
        Ok(Self {
            profile_id,
            prompt_id,
            attempt_id,
            node_id,
            principal_id,
            authorization,
            cancellation,
            deadline,
            maximum_response_bytes,
        })
    }

    pub fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    pub fn prompt_id(&self) -> PromptId {
        self.prompt_id
    }

    pub fn attempt_id(&self) -> AttemptId {
        self.attempt_id
    }

    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    pub fn principal_id(&self) -> Option<&str> {
        self.principal_id.as_deref()
    }

    pub fn plugin_id(&self) -> &str {
        self.authorization.plugin_id()
    }

    pub fn plugin_digest_sha256(&self) -> &str {
        self.authorization.digest_sha256()
    }

    pub fn authorization(&self) -> &PluginAuthorization {
        &self.authorization
    }

    pub fn cancellation(&self) -> &CancellationToken {
        &self.cancellation
    }

    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    pub fn maximum_response_bytes(&self) -> u64 {
        self.maximum_response_bytes
    }
}

pub struct PluginServiceOperationContext<'a> {
    invocation: &'a PluginServiceInvocationContext,
    clock: &'a dyn SystemClock,
    clock_origin: Instant,
}

impl PluginServiceOperationContext<'_> {
    pub fn invocation(&self) -> &PluginServiceInvocationContext {
        self.invocation
    }

    pub fn cancellation(&self) -> &CancellationToken {
        self.invocation.cancellation()
    }

    pub fn maximum_response_bytes(&self) -> u64 {
        self.invocation.maximum_response_bytes()
    }

    pub fn check_active(&self) -> Result<(), PluginServiceError> {
        if self.invocation.cancellation.is_cancelled() {
            return Err(PluginServiceError::Cancelled);
        }
        let now = self.clock.utc_now();
        now.checked_duration_since(self.clock_origin)
            .ok_or(PluginServiceError::ClockMovedBackwards)?;
        if now >= self.invocation.deadline {
            return Err(PluginServiceError::DeadlineExceeded);
        }
        Ok(())
    }

    pub fn remaining_time(&self) -> Result<Duration, PluginServiceError> {
        self.check_active()?;
        let now = self.clock.utc_now();
        self.invocation
            .deadline
            .checked_duration_since(now)
            .ok_or(PluginServiceError::DeadlineExceeded)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginServiceActuatorError {
    message: String,
}

impl PluginServiceActuatorError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: sanitize_actuator_message(&message.into()),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PluginServiceActuatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PluginServiceActuatorError {}

pub trait ProviderRequestActuator: Send + Sync {
    fn execute(
        &self,
        request: &AuthorizedProviderRequest,
        secret: Option<&SecretValue>,
        body: &[u8],
        context: &PluginServiceOperationContext<'_>,
    ) -> Result<Vec<u8>, PluginServiceActuatorError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizedCredentialPresenceRequest {
    profile_id: ProfileId,
    plugin_id: String,
    secret_id: SecretId,
}

impl AuthorizedCredentialPresenceRequest {
    pub fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn secret_id(&self) -> &SecretId {
        &self.secret_id
    }
}

pub trait CredentialPresenceActuator: Send + Sync {
    fn is_present(
        &self,
        request: &AuthorizedCredentialPresenceRequest,
        context: &PluginServiceOperationContext<'_>,
    ) -> Result<bool, PluginServiceActuatorError>;

    fn read_for_provider(
        &self,
        request: &AuthorizedCredentialPresenceRequest,
        context: &PluginServiceOperationContext<'_>,
    ) -> Result<Option<SecretValue>, PluginServiceActuatorError>;
}

#[derive(Clone)]
pub struct PluginModelHandle {
    model_id: String,
    model_identity: String,
    model_format: &'static str,
    accounting: ModelLoadAccounting,
    model: Arc<LoadedModel>,
}

impl PluginModelHandle {
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn model_identity(&self) -> &str {
        &self.model_identity
    }

    pub fn model_format(&self) -> &'static str {
        self.model_format
    }

    pub fn accounting(&self) -> &ModelLoadAccounting {
        &self.accounting
    }

    pub fn model(&self) -> Arc<LoadedModel> {
        self.model.clone()
    }
}

impl fmt::Debug for PluginModelHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PluginModelHandle")
            .field("model_id", &self.model_id)
            .field("model_identity", &self.model_identity)
            .field("model_format", &self.model_format)
            .field("accounting", &self.accounting)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct PluginCapabilityBroker {
    inner: Arc<PluginCapabilityBrokerInner>,
}

struct PluginCapabilityBrokerInner {
    assets: SharedAssetService,
    model_store: Mutex<ModelStore>,
    provider_policy: ProviderPolicy,
    provider_cost_acceptance_verifier: Option<ProviderCostAcceptanceVerifier>,
    consumed_provider_cost_nonces: Mutex<BTreeMap<ProviderCostNonce, Instant>>,
    provider_actuator: Arc<dyn ProviderRequestActuator>,
    credential_presence_actuator: Arc<dyn CredentialPresenceActuator>,
    clock: Arc<dyn SystemClock>,
    clock_origin: Instant,
    rng_policy: PluginRngPolicy,
    rng_state: Mutex<PluginRngState>,
}

#[derive(Default)]
struct PluginRngState {
    checkpoints: BTreeMap<PluginRngKey, RngCheckpoint>,
    active_streams: BTreeSet<PluginRngKey>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PluginRngKey {
    profile_id: String,
    prompt_id: String,
    attempt_id: String,
    node_id: NodeId,
    plugin_id: String,
    plugin_digest_sha256: String,
    stream_id: String,
}

impl PluginCapabilityBroker {
    pub fn new(
        assets: SharedAssetService,
        model_store: ModelStore,
        provider_policy: ProviderPolicy,
        provider_actuator: Arc<dyn ProviderRequestActuator>,
        credential_presence_actuator: Arc<dyn CredentialPresenceActuator>,
        clock: Arc<dyn SystemClock>,
        rng_policy: PluginRngPolicy,
    ) -> Self {
        Self::new_internal(
            assets,
            model_store,
            provider_policy,
            None,
            provider_actuator,
            credential_presence_actuator,
            clock,
            rng_policy,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_provider_cost_acceptance(
        assets: SharedAssetService,
        model_store: ModelStore,
        provider_policy: ProviderPolicy,
        provider_cost_acceptance_verifier: ProviderCostAcceptanceVerifier,
        provider_actuator: Arc<dyn ProviderRequestActuator>,
        credential_presence_actuator: Arc<dyn CredentialPresenceActuator>,
        clock: Arc<dyn SystemClock>,
        rng_policy: PluginRngPolicy,
    ) -> Self {
        Self::new_internal(
            assets,
            model_store,
            provider_policy,
            Some(provider_cost_acceptance_verifier),
            provider_actuator,
            credential_presence_actuator,
            clock,
            rng_policy,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_internal(
        assets: SharedAssetService,
        model_store: ModelStore,
        provider_policy: ProviderPolicy,
        provider_cost_acceptance_verifier: Option<ProviderCostAcceptanceVerifier>,
        provider_actuator: Arc<dyn ProviderRequestActuator>,
        credential_presence_actuator: Arc<dyn CredentialPresenceActuator>,
        clock: Arc<dyn SystemClock>,
        rng_policy: PluginRngPolicy,
    ) -> Self {
        let clock_origin = clock.utc_now();
        Self {
            inner: Arc::new(PluginCapabilityBrokerInner {
                assets,
                model_store: Mutex::new(model_store),
                provider_policy,
                provider_cost_acceptance_verifier,
                consumed_provider_cost_nonces: Mutex::new(BTreeMap::new()),
                provider_actuator,
                credential_presence_actuator,
                clock,
                clock_origin,
                rng_policy,
                rng_state: Mutex::new(PluginRngState::default()),
            }),
        }
    }

    pub fn begin_invocation(
        &self,
        context: PluginServiceInvocationContext,
    ) -> Result<PluginCapabilityInvocation, PluginServiceError> {
        let operation_context = PluginServiceOperationContext {
            invocation: &context,
            clock: self.inner.clock.as_ref(),
            clock_origin: self.inner.clock_origin,
        };
        operation_context.check_active()?;
        Ok(PluginCapabilityInvocation {
            broker: self.clone(),
            context,
            rng_transactions: BTreeMap::new(),
            operation_failed: Arc::new(AtomicBool::new(false)),
            terminal: false,
        })
    }
}

pub struct PluginCapabilityInvocation {
    broker: PluginCapabilityBroker,
    context: PluginServiceInvocationContext,
    rng_transactions: BTreeMap<PluginRngKey, RngTransaction>,
    operation_failed: Arc<AtomicBool>,
    terminal: bool,
}

struct ProviderCostNonceClaim<'a> {
    broker: &'a PluginCapabilityBrokerInner,
    nonce: ProviderCostNonce,
    expires_at: Instant,
    retained: bool,
}

impl ProviderCostNonceClaim<'_> {
    fn retain(mut self) {
        self.retained = true;
    }
}

impl Drop for ProviderCostNonceClaim<'_> {
    fn drop(&mut self) {
        if self.retained {
            return;
        }
        let mut consumed_nonces = self.broker.consumed_provider_cost_nonces.lock();
        if consumed_nonces.get(&self.nonce) == Some(&self.expires_at) {
            consumed_nonces.remove(&self.nonce);
        }
    }
}

impl PluginCapabilityBrokerInner {
    fn claim_provider_cost_nonce(
        &self,
        nonce: ProviderCostNonce,
        expires_at: Instant,
        now: Instant,
    ) -> Result<ProviderCostNonceClaim<'_>, PluginServiceError> {
        let mut consumed_nonces = self.consumed_provider_cost_nonces.lock();
        consumed_nonces.retain(|_, expiration| *expiration > now);
        if expires_at <= now {
            return Err(PluginServiceError::ProviderCostAcceptanceDenied);
        }
        if consumed_nonces.contains_key(&nonce) {
            return Err(PluginServiceError::ProviderCostAcceptanceReused);
        }
        if consumed_nonces.len() >= MAX_CONSUMED_PROVIDER_COST_NONCES {
            return Err(PluginServiceError::ProviderCostAcceptanceDenied);
        }
        consumed_nonces.insert(nonce, expires_at);
        Ok(ProviderCostNonceClaim {
            broker: self,
            nonce,
            expires_at,
            retained: false,
        })
    }
}

impl PluginCapabilityInvocation {
    pub fn context(&self) -> &PluginServiceInvocationContext {
        &self.context
    }

    pub fn read_asset(
        &self,
        namespace: AssetNamespace,
        asset_reference: &str,
    ) -> Result<Vec<u8>, PluginServiceError> {
        self.check_terminal()?;
        let outcome = OperationOutcomeGuard::new(self.operation_failed.clone());
        self.require_capability(&Capability::Asset {
            namespace: namespace.locator_type().to_owned(),
            action: AssetOperation::Read,
        })?;
        let operation_context = self.operation_context();
        operation_context.check_active()?;
        let assets = self
            .broker
            .inner
            .assets
            .lock()
            .map_err(|_| PluginServiceError::AssetServiceUnavailable)?;
        let identity = assets
            .roots()
            .identity_from_reference(asset_reference)
            .map_err(map_asset_error)?;
        if identity.namespace != namespace {
            return Err(PluginServiceError::AssetNamespaceMismatch);
        }
        let bytes = assets
            .read_verified(
                &identity,
                self.context.authorization.capabilities(),
                self.context.cancellation(),
                self.context.maximum_response_bytes,
            )
            .map_err(map_asset_error)?;
        check_response_size(bytes.len(), self.context.maximum_response_bytes)?;
        operation_context.check_active()?;
        Ok(outcome.succeed(bytes))
    }

    pub fn load_model(&self, model_id: &str) -> Result<PluginModelHandle, PluginServiceError> {
        self.check_terminal()?;
        let outcome = OperationOutcomeGuard::new(self.operation_failed.clone());
        validate_identity("model", model_id)?;
        self.require_capability(&Capability::ModelHandle {
            model_id: model_id.to_owned(),
        })?;
        self.require_capability(&Capability::Asset {
            namespace: AssetNamespace::Model.locator_type().to_owned(),
            action: AssetOperation::Read,
        })?;
        let operation_context = self.operation_context();
        operation_context.check_active()?;
        let assets = self
            .broker
            .inner
            .assets
            .lock()
            .map_err(|_| PluginServiceError::AssetServiceUnavailable)?;
        let identity = assets
            .roots()
            .identity_from_reference(model_id)
            .map_err(map_asset_error)?;
        if identity.namespace != AssetNamespace::Model {
            return Err(PluginServiceError::ModelNamespaceRequired);
        }
        let mut model_store = self.broker.inner.model_store.lock();
        let model = assets
            .load_model(
                &identity,
                &mut model_store,
                self.context.authorization.capabilities(),
                self.context.cancellation(),
            )
            .map_err(map_asset_error)?;
        operation_context.check_active()?;
        let model_format = model
            .documents()
            .first()
            .map(|document| model_format_name(&document.format))
            .ok_or(PluginServiceError::AssetOperationFailed {
                operation: "model_projection",
            })?;
        Ok(outcome.succeed(PluginModelHandle {
            model_id: model_id.to_owned(),
            model_identity: model.identity().to_owned(),
            model_format,
            accounting: model.accounting().clone(),
            model,
        }))
    }

    pub fn execute_provider_request(
        &self,
        provider: &str,
        endpoint: &str,
        secret_id: Option<&SecretId>,
        body: &[u8],
    ) -> Result<Vec<u8>, PluginServiceError> {
        self.check_terminal()?;
        let outcome = OperationOutcomeGuard::new(self.operation_failed.clone());
        check_request_size(body.len())?;
        self.require_capability(&Capability::ProviderNetwork {
            provider: provider.to_owned(),
            endpoint: endpoint.to_owned(),
        })?;
        if let Some(secret_id) = secret_id {
            self.require_capability(&Capability::Secret {
                secret_id: secret_id.as_str().to_owned(),
            })?;
        }
        let operation_context = self.operation_context();
        operation_context.check_active()?;
        let authorized_request = self
            .broker
            .inner
            .provider_policy
            .authorize(
                &self.context.profile_id.0.to_string(),
                self.context.plugin_id(),
                provider,
                endpoint,
                secret_id,
            )
            .map_err(|_| PluginServiceError::ProviderPolicyDenied)?;
        let secret = secret_id
            .map(|secret_id| {
                let request = AuthorizedCredentialPresenceRequest {
                    profile_id: self.context.profile_id,
                    plugin_id: self.context.plugin_id().to_owned(),
                    secret_id: secret_id.clone(),
                };
                match self
                    .broker
                    .inner
                    .credential_presence_actuator
                    .read_for_provider(&request, &operation_context)
                {
                    Ok(secret) => secret.ok_or(PluginServiceError::CredentialUnavailable),
                    Err(error) => {
                        operation_context.check_active()?;
                        Err(PluginServiceError::ActuatorFailed {
                            service: "credential_read",
                            message: error.message,
                        })
                    }
                }
            })
            .transpose()?;
        let response = match self.broker.inner.provider_actuator.execute(
            &authorized_request,
            secret.as_ref(),
            body,
            &operation_context,
        ) {
            Ok(response) => response,
            Err(error) => {
                operation_context.check_active()?;
                return Err(PluginServiceError::ActuatorFailed {
                    service: "provider",
                    message: error.message,
                });
            }
        };
        check_response_size(response.len(), self.context.maximum_response_bytes)?;
        operation_context.check_active()?;
        Ok(outcome.succeed(response))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_priced_provider_request(
        &self,
        provider_binding_sha256: &str,
        provider: &str,
        endpoint: &str,
        secret_id: Option<&SecretId>,
        price_bound: &ProviderPriceBound,
        nonce: ProviderCostNonce,
        acceptance: Option<&ProviderCostAcceptance>,
        body: &[u8],
    ) -> Result<Vec<u8>, PluginServiceError> {
        self.check_terminal()?;
        let outcome = OperationOutcomeGuard::new(self.operation_failed.clone());
        check_request_size(body.len())?;
        self.require_capability(&Capability::ProviderNetwork {
            provider: provider.to_owned(),
            endpoint: endpoint.to_owned(),
        })?;
        if let Some(secret_id) = secret_id {
            self.require_capability(&Capability::Secret {
                secret_id: secret_id.as_str().to_owned(),
            })?;
        }
        let operation_context = self.operation_context();
        operation_context.check_active()?;
        let acceptance = acceptance.ok_or(PluginServiceError::ProviderCostAcceptanceRequired)?;
        let verifier = self
            .broker
            .inner
            .provider_cost_acceptance_verifier
            .as_ref()
            .ok_or(PluginServiceError::ProviderCostAcceptanceRequired)?;
        let principal_id = self
            .context
            .principal_id()
            .ok_or(PluginServiceError::ProviderCostPrincipalRequired)?;
        let expected_scope = ProviderCostAcceptanceScope::new(
            principal_id,
            self.context.profile_id().0.to_string(),
            self.context.prompt_id().0.to_string(),
            self.context.plugin_id(),
            self.context.plugin_digest_sha256(),
            provider_binding_sha256,
            provider,
            endpoint,
            price_bound.clone(),
        )
        .map_err(|_| PluginServiceError::ProviderCostAcceptanceDenied)?;
        let verified_acceptance = verifier
            .verify(
                acceptance,
                &expected_scope,
                self.broker.inner.clock.utc_now(),
            )
            .map_err(|_| PluginServiceError::ProviderCostAcceptanceDenied)?;
        if verified_acceptance.nonce() != nonce {
            return Err(PluginServiceError::ProviderCostAcceptanceDenied);
        }
        let authorized_request = self
            .broker
            .inner
            .provider_policy
            .authorize(
                &self.context.profile_id.0.to_string(),
                self.context.plugin_id(),
                provider,
                endpoint,
                secret_id,
            )
            .map_err(|_| PluginServiceError::ProviderPolicyDenied)?;
        let cost_nonce_claim = self.broker.inner.claim_provider_cost_nonce(
            nonce,
            verified_acceptance.expires_at(),
            self.broker.inner.clock.utc_now(),
        )?;
        let secret = secret_id
            .map(|secret_id| {
                let request = AuthorizedCredentialPresenceRequest {
                    profile_id: self.context.profile_id,
                    plugin_id: self.context.plugin_id().to_owned(),
                    secret_id: secret_id.clone(),
                };
                match self
                    .broker
                    .inner
                    .credential_presence_actuator
                    .read_for_provider(&request, &operation_context)
                {
                    Ok(secret) => secret.ok_or(PluginServiceError::CredentialUnavailable),
                    Err(error) => {
                        operation_context.check_active()?;
                        Err(PluginServiceError::ActuatorFailed {
                            service: "credential_read",
                            message: error.message,
                        })
                    }
                }
            })
            .transpose()?;
        operation_context.check_active()?;
        cost_nonce_claim.retain();
        let response = match self.broker.inner.provider_actuator.execute(
            &authorized_request,
            secret.as_ref(),
            body,
            &operation_context,
        ) {
            Ok(response) => response,
            Err(error) => {
                operation_context.check_active()?;
                return Err(PluginServiceError::ActuatorFailed {
                    service: "provider",
                    message: error.message,
                });
            }
        };
        check_response_size(response.len(), self.context.maximum_response_bytes)?;
        operation_context.check_active()?;
        Ok(outcome.succeed(response))
    }

    pub fn credential_is_present(&self, secret_id: &SecretId) -> Result<bool, PluginServiceError> {
        self.check_terminal()?;
        let outcome = OperationOutcomeGuard::new(self.operation_failed.clone());
        self.require_capability(&Capability::Secret {
            secret_id: secret_id.as_str().to_owned(),
        })?;
        let operation_context = self.operation_context();
        operation_context.check_active()?;
        let request = AuthorizedCredentialPresenceRequest {
            profile_id: self.context.profile_id,
            plugin_id: self.context.plugin_id().to_owned(),
            secret_id: secret_id.clone(),
        };
        let present = match self
            .broker
            .inner
            .credential_presence_actuator
            .is_present(&request, &operation_context)
        {
            Ok(present) => present,
            Err(error) => {
                operation_context.check_active()?;
                return Err(PluginServiceError::ActuatorFailed {
                    service: "credential_presence",
                    message: error.message,
                });
            }
        };
        operation_context.check_active()?;
        Ok(outcome.succeed(present))
    }

    pub fn monotonic_milliseconds(&self, clock_id: &str) -> Result<u64, PluginServiceError> {
        self.check_terminal()?;
        let outcome = OperationOutcomeGuard::new(self.operation_failed.clone());
        validate_identity("clock", clock_id)?;
        self.require_capability(&Capability::Clock {
            clock_id: clock_id.to_owned(),
        })?;
        let operation_context = self.operation_context();
        operation_context.check_active()?;
        let elapsed = self
            .broker
            .inner
            .clock
            .utc_now()
            .checked_duration_since(self.broker.inner.clock_origin)
            .ok_or(PluginServiceError::ClockMovedBackwards)?;
        let milliseconds =
            u64::try_from(elapsed.as_millis()).map_err(|_| PluginServiceError::ClockOverflow)?;
        Ok(outcome.succeed(milliseconds))
    }

    pub fn random_bytes(
        &mut self,
        stream_id: &str,
        length: usize,
    ) -> Result<Vec<u8>, PluginServiceError> {
        self.check_terminal()?;
        let outcome = OperationOutcomeGuard::new(self.operation_failed.clone());
        validate_identity("randomness stream", stream_id)?;
        self.require_capability(&Capability::Randomness {
            stream_id: stream_id.to_owned(),
        })?;
        check_response_size(length, self.context.maximum_response_bytes)?;
        self.operation_context().check_active()?;
        let key = self.rng_key(stream_id);
        if !self.rng_transactions.contains_key(&key) {
            let checkpoint = {
                let mut state = self.broker.inner.rng_state.lock();
                if !state.active_streams.insert(key.clone()) {
                    return Err(PluginServiceError::RandomnessStreamBusy);
                }
                state.checkpoints.get(&key).cloned()
            };
            let transaction = match self.rng_stream(stream_id).and_then(|stream| {
                stream
                    .begin(checkpoint)
                    .map_err(|_| PluginServiceError::RandomnessFailed)
            }) {
                Ok(transaction) => transaction,
                Err(error) => {
                    self.broker
                        .inner
                        .rng_state
                        .lock()
                        .active_streams
                        .remove(&key);
                    return Err(error);
                }
            };
            self.rng_transactions.insert(key.clone(), transaction);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| PluginServiceError::ResponseAllocationFailed)?;
        bytes.resize(length, 0);
        for chunk in bytes.chunks_mut(RNG_DEADLINE_CHECK_CHUNK_BYTES) {
            self.rng_transactions
                .get_mut(&key)
                .ok_or(PluginServiceError::RandomnessFailed)?
                .fill_bytes(chunk, self.context.cancellation())
                .map_err(|_| {
                    if self.context.cancellation.is_cancelled() {
                        PluginServiceError::Cancelled
                    } else {
                        PluginServiceError::RandomnessFailed
                    }
                })?;
            self.operation_context().check_active()?;
        }
        Ok(outcome.succeed(bytes))
    }

    pub fn sanitize_log(&self, level: &str, message: &str) -> Result<String, PluginServiceError> {
        self.check_terminal()?;
        let outcome = OperationOutcomeGuard::new(self.operation_failed.clone());
        validate_identity("log level", level)?;
        check_request_size(message.len())?;
        self.require_capability(&Capability::SanitizedLog {
            level: level.to_owned(),
        })?;
        let operation_context = self.operation_context();
        operation_context.check_active()?;
        let redactions = self
            .context
            .authorization
            .capabilities()
            .capabilities()
            .iter()
            .filter_map(|capability| match capability {
                Capability::Secret { secret_id } => Some(secret_id.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut sanitized: String = message
            .chars()
            .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
            .take(8_192)
            .collect();
        for secret_id in redactions {
            sanitized = sanitized.replace(secret_id, "[REDACTED]");
        }
        check_response_size(sanitized.len(), self.context.maximum_response_bytes)?;
        operation_context.check_active()?;
        Ok(outcome.succeed(sanitized))
    }

    pub fn handle_wire_request(
        &mut self,
        request: PluginServiceWireRequest,
    ) -> PluginServiceWireResponse {
        let response = match request {
            PluginServiceWireRequest::ReadAsset {
                namespace,
                asset_reference,
            } => AssetNamespace::from_locator_type(&namespace)
                .map_err(|_| PluginServiceError::InvalidWirePayload)
                .and_then(|namespace| self.read_asset(namespace, &asset_reference))
                .map(PluginServiceWireResponse::Bytes),
            PluginServiceWireRequest::ExecuteProvider {
                provider,
                endpoint,
                secret_id,
                body,
            } => secret_id
                .map(SecretId::new)
                .transpose()
                .map_err(|_| PluginServiceError::InvalidWirePayload)
                .and_then(|secret_id| {
                    self.execute_provider_request(&provider, &endpoint, secret_id.as_ref(), &body)
                })
                .map(PluginServiceWireResponse::Bytes),
            PluginServiceWireRequest::CredentialIsPresent { secret_id } => SecretId::new(secret_id)
                .map_err(|_| PluginServiceError::InvalidWirePayload)
                .and_then(|secret_id| self.credential_is_present(&secret_id))
                .map(PluginServiceWireResponse::Boolean),
            PluginServiceWireRequest::MonotonicMilliseconds { clock_id } => self
                .monotonic_milliseconds(&clock_id)
                .map(PluginServiceWireResponse::TimestampMilliseconds),
            PluginServiceWireRequest::RandomBytes { stream_id, length } => usize::try_from(length)
                .map_err(|_| PluginServiceError::InvalidWirePayload)
                .and_then(|length| self.random_bytes(&stream_id, length))
                .map(PluginServiceWireResponse::Bytes),
            PluginServiceWireRequest::LoadModel { model_id } => {
                self.load_model(&model_id)
                    .map(|model| PluginServiceWireResponse::Model {
                        identifier: model.model_id().to_owned(),
                        format: model.model_format().to_owned(),
                        digest_sha256: model.model_identity().to_owned(),
                    })
            }
            PluginServiceWireRequest::SanitizeLog { level, message } => self
                .sanitize_log(&level, &message)
                .map(PluginServiceWireResponse::SanitizedLog),
        };
        if response.is_err() {
            self.operation_failed.store(true, Ordering::Release);
        }
        response.unwrap_or_else(|error| PluginServiceWireResponse::Failure(error.into()))
    }

    pub fn finish(mut self) -> Result<(), PluginServiceError> {
        self.check_terminal()?;
        if self.operation_failed.load(Ordering::Acquire) {
            return Err(PluginServiceError::InvocationFailed);
        }
        self.operation_context().check_active()?;
        let committed = std::mem::take(&mut self.rng_transactions)
            .into_iter()
            .map(|(key, transaction)| (key, transaction.commit()))
            .collect::<Vec<_>>();
        let mut state = self.broker.inner.rng_state.lock();
        for (key, checkpoint) in committed {
            state.checkpoints.insert(key.clone(), checkpoint);
            state.active_streams.remove(&key);
        }
        drop(state);
        self.terminal = true;
        Ok(())
    }

    pub fn abort(mut self) {
        self.abort_internal();
    }

    fn operation_context(&self) -> PluginServiceOperationContext<'_> {
        PluginServiceOperationContext {
            invocation: &self.context,
            clock: self.broker.inner.clock.as_ref(),
            clock_origin: self.broker.inner.clock_origin,
        }
    }

    fn check_terminal(&self) -> Result<(), PluginServiceError> {
        if self.terminal {
            Err(PluginServiceError::InvocationFinished)
        } else {
            Ok(())
        }
    }

    fn require_capability(&self, capability: &Capability) -> Result<(), PluginServiceError> {
        self.context
            .authorization
            .capabilities()
            .require(capability)
            .map_err(|_| PluginServiceError::CapabilityDenied(capability.clone()))
    }

    fn rng_key(&self, stream_id: &str) -> PluginRngKey {
        PluginRngKey {
            profile_id: self.context.profile_id.0.to_string(),
            prompt_id: self.context.prompt_id.0.to_string(),
            attempt_id: self.context.attempt_id.0.to_string(),
            node_id: self.context.node_id.clone(),
            plugin_id: self.context.plugin_id().to_owned(),
            plugin_digest_sha256: self.context.plugin_digest_sha256().to_owned(),
            stream_id: stream_id.to_owned(),
        }
    }

    fn rng_stream(&self, stream_id: &str) -> Result<RngStream, PluginServiceError> {
        let address = RngStreamAddress::new(
            self.context.prompt_id.0.to_string(),
            self.context.attempt_id.0.to_string(),
            self.context.node_id.0.clone(),
            0,
            format!("plugin:{}:{stream_id}", self.context.plugin_id()),
            0,
            0,
            RetryRngPolicy::Replay,
        )
        .map_err(|_| PluginServiceError::RandomnessFailed)?;
        RngStream::new(
            self.broker.inner.rng_policy.profile,
            self.broker.inner.rng_policy.algorithm,
            self.broker.inner.rng_policy.seed,
            address,
        )
        .map_err(|_| PluginServiceError::RandomnessFailed)
    }

    fn abort_internal(&mut self) {
        let transactions = std::mem::take(&mut self.rng_transactions);
        let mut state = self.broker.inner.rng_state.lock();
        for (key, transaction) in transactions {
            transaction.abort();
            state.active_streams.remove(&key);
        }
        drop(state);
        self.terminal = true;
    }
}

struct OperationOutcomeGuard {
    operation_failed: Arc<AtomicBool>,
    succeeded: bool,
}

impl OperationOutcomeGuard {
    fn new(operation_failed: Arc<AtomicBool>) -> Self {
        Self {
            operation_failed,
            succeeded: false,
        }
    }

    fn succeed<T>(mut self, value: T) -> T {
        self.succeeded = true;
        value
    }
}

impl Drop for OperationOutcomeGuard {
    fn drop(&mut self) {
        if !self.succeeded {
            self.operation_failed.store(true, Ordering::Release);
        }
    }
}

impl Drop for PluginCapabilityInvocation {
    fn drop(&mut self) {
        if !self.terminal {
            self.abort_internal();
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PluginServiceError {
    #[error("plugin service context belongs to a different runtime profile")]
    ProfileMismatch,
    #[error("plugin authorization subject does not match the signed plugin identity")]
    AuthorizationIdentityMismatch,
    #[error("plugin service {kind} identity is invalid")]
    InvalidIdentity { kind: &'static str },
    #[error("plugin response limit must be between 1 and {maximum} bytes")]
    InvalidResponseLimit { maximum: u64 },
    #[error("plugin capability denied: {0:?}")]
    CapabilityDenied(Capability),
    #[error("plugin invocation was cancelled")]
    Cancelled,
    #[error("plugin invocation deadline was exceeded")]
    DeadlineExceeded,
    #[error("the injected monotonic clock moved backwards")]
    ClockMovedBackwards,
    #[error("the injected monotonic clock overflowed milliseconds")]
    ClockOverflow,
    #[error("plugin request exceeds the {maximum}-byte limit")]
    RequestTooLarge { maximum: u64 },
    #[error("plugin response exceeds the {maximum}-byte invocation limit")]
    ResponseTooLarge { maximum: u64 },
    #[error("plugin response allocation failed")]
    ResponseAllocationFailed,
    #[error("the canonical asset service is unavailable")]
    AssetServiceUnavailable,
    #[error("asset reference namespace does not match the authorized namespace")]
    AssetNamespaceMismatch,
    #[error("model loading requires a canonical model asset reference")]
    ModelNamespaceRequired,
    #[error("canonical asset operation failed: {operation}")]
    AssetOperationFailed { operation: &'static str },
    #[error("provider policy denied the request")]
    ProviderPolicyDenied,
    #[error("priced provider request requires a host-issued cost acceptance")]
    ProviderCostAcceptanceRequired,
    #[error("priced provider request requires a host-bound principal")]
    ProviderCostPrincipalRequired,
    #[error("provider cost acceptance is invalid, expired, or bound to another request")]
    ProviderCostAcceptanceDenied,
    #[error("provider cost acceptance nonce was already consumed")]
    ProviderCostAcceptanceReused,
    #[error("the authorized provider credential is unavailable")]
    CredentialUnavailable,
    #[error("{service} actuator failed: {message}")]
    ActuatorFailed {
        service: &'static str,
        message: String,
    },
    #[error("randomness stream is already in use by another invocation")]
    RandomnessStreamBusy,
    #[error("canonical randomness operation failed")]
    RandomnessFailed,
    #[error("plugin invocation has already finished")]
    InvocationFinished,
    #[error("plugin invocation cannot commit after a failed capability operation")]
    InvocationFailed,
    #[error("plugin capability wire payload is malformed")]
    InvalidWirePayload,
}

impl From<PluginServiceError> for PluginServiceWireFailure {
    fn from(error: PluginServiceError) -> Self {
        match error {
            PluginServiceError::CapabilityDenied(_) => Self::CapabilityDenied,
            PluginServiceError::Cancelled => Self::Cancelled,
            PluginServiceError::DeadlineExceeded => Self::DeadlineExceeded,
            PluginServiceError::ResponseTooLarge { .. }
            | PluginServiceError::ResponseAllocationFailed => Self::ResponseTooLarge,
            PluginServiceError::ProviderPolicyDenied
            | PluginServiceError::ProviderCostAcceptanceRequired
            | PluginServiceError::ProviderCostPrincipalRequired
            | PluginServiceError::ProviderCostAcceptanceDenied
            | PluginServiceError::ProviderCostAcceptanceReused => Self::ProviderDenied,
            PluginServiceError::ActuatorFailed { .. } => Self::ActuatorFailed,
            PluginServiceError::RandomnessStreamBusy | PluginServiceError::RandomnessFailed => {
                Self::RandomnessFailed
            }
            PluginServiceError::InvocationFailed | PluginServiceError::InvocationFinished => {
                Self::InvocationFailed
            }
            PluginServiceError::AssetServiceUnavailable
            | PluginServiceError::CredentialUnavailable
            | PluginServiceError::AssetOperationFailed { .. } => Self::ServiceUnavailable,
            PluginServiceError::ProfileMismatch
            | PluginServiceError::AuthorizationIdentityMismatch
            | PluginServiceError::InvalidIdentity { .. }
            | PluginServiceError::InvalidResponseLimit { .. }
            | PluginServiceError::ClockMovedBackwards
            | PluginServiceError::ClockOverflow
            | PluginServiceError::RequestTooLarge { .. }
            | PluginServiceError::AssetNamespaceMismatch
            | PluginServiceError::ModelNamespaceRequired
            | PluginServiceError::InvalidWirePayload => Self::InvalidRequest,
        }
    }
}

fn validate_identity(kind: &'static str, value: &str) -> Result<(), PluginServiceError> {
    if value.is_empty()
        || value != value.trim()
        || value.len() > MAX_PLUGIN_SERVICE_IDENTITY_BYTES
        || value.chars().any(char::is_control)
    {
        Err(PluginServiceError::InvalidIdentity { kind })
    } else {
        Ok(())
    }
}

fn check_request_size(length: usize) -> Result<(), PluginServiceError> {
    if u64::try_from(length).unwrap_or(u64::MAX) > MAX_PLUGIN_SERVICE_REQUEST_BYTES {
        Err(PluginServiceError::RequestTooLarge {
            maximum: MAX_PLUGIN_SERVICE_REQUEST_BYTES,
        })
    } else {
        Ok(())
    }
}

fn check_response_size(length: usize, maximum: u64) -> Result<(), PluginServiceError> {
    if u64::try_from(length).unwrap_or(u64::MAX) > maximum {
        Err(PluginServiceError::ResponseTooLarge { maximum })
    } else {
        Ok(())
    }
}

fn map_asset_error(error: AssetError) -> PluginServiceError {
    match error {
        AssetError::Cancelled
        | AssetError::Model(ModelStoreError::Cancelled)
        | AssetError::Model(ModelStoreError::Format(ModelFormatError::Cancelled)) => {
            PluginServiceError::Cancelled
        }
        AssetError::PermissionDenied { namespace, action } => {
            PluginServiceError::CapabilityDenied(Capability::Asset {
                namespace: namespace.locator_type().to_owned(),
                action: action.into(),
            })
        }
        AssetError::TooLarge { limit, .. } => {
            PluginServiceError::ResponseTooLarge { maximum: limit }
        }
        AssetError::ModelNamespaceRequired(_) => PluginServiceError::ModelNamespaceRequired,
        AssetError::InvalidReference(_) => PluginServiceError::AssetOperationFailed {
            operation: "reference_validation",
        },
        _ => PluginServiceError::AssetOperationFailed {
            operation: "canonical_asset_service",
        },
    }
}

fn sanitize_actuator_message(message: &str) -> String {
    let mut sanitized = String::new();
    for character in message.chars() {
        if sanitized.len().saturating_add(character.len_utf8()) > MAX_PLUGIN_SERVICE_IDENTITY_BYTES
        {
            break;
        }
        if character.is_control() {
            sanitized.push(' ');
        } else {
            sanitized.push(character);
        }
    }
    if sanitized.trim().is_empty() {
        "operation failed".to_owned()
    } else {
        sanitized
    }
}

fn model_format_name(format: &ModelFormat) -> &'static str {
    match format {
        ModelFormat::Safetensors => "safetensors",
        ModelFormat::PytorchArchive => "pytorch-archive",
        ModelFormat::Gguf => "gguf",
        ModelFormat::JsonConfig => "json-config",
        ModelFormat::JsonTokenizer => "json-tokenizer",
        ModelFormat::YamlConfig => "yaml-config",
        ModelFormat::SentencePiece => "sentencepiece",
        ModelFormat::Tiktoken => "tiktoken",
    }
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs,
        sync::{
            Arc, Barrier,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
    };

    use comfy_model::ParserLimits;
    use comfy_plugin_sdk::{
        ApiRequirement, ApiVersion, CachePolicy, CapabilityKind, CapabilityQuota,
        CapabilityRequest, DeterminismPolicy, ED25519_SIGNATURE_BYTES, EffectPolicy,
        ManifestProvenance, ManifestSignature, PLUGIN_SIGNATURE_ALGORITHM, PluginManifest,
        PluginNode, PluginSigningKey,
    };
    use tempfile::TempDir;
    use uuid::Uuid;

    use crate::{
        AssetRoots, AssetService, CapabilitySet, PermissionGrant, PermissionPolicy,
        PluginTrustPolicy, PluginVerificationKey, ProviderCostAcceptanceIssuer, ProviderEndpoint,
        ProviderMode, TrustError,
    };

    use super::*;

    const SIGNING_KEY: &[u8; 32] = b"0123456789abcdef0123456789abcdef";
    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const PROFILE_UUID: Uuid = Uuid::from_u128(1);
    const PROMPT_UUID: Uuid = Uuid::from_u128(2);
    const ATTEMPT_UUID: Uuid = Uuid::from_u128(3);
    const PROVIDER: &str = "fixture-provider";
    const ENDPOINT: &str = "https://provider.invalid/v1";
    const SECRET: &str = "fixture-secret";

    struct TestClock {
        now: Mutex<Instant>,
    }

    impl TestClock {
        fn new(now: Instant) -> Self {
            Self {
                now: Mutex::new(now),
            }
        }

        fn now(&self) -> Instant {
            *self.now.lock()
        }

        fn advance(&self, duration: Duration) {
            *self.now.lock() += duration;
        }
    }

    impl SystemClock for TestClock {
        fn utc_now(&self) -> Instant {
            self.now()
        }
    }

    #[derive(Default)]
    struct TestProviderActuator {
        calls: AtomicUsize,
        response: Mutex<Vec<u8>>,
        cancel_during_call: AtomicBool,
        fail_call: AtomicBool,
        last_authorized_request: Mutex<Option<(String, String, Option<String>)>>,
        last_secret: Mutex<Option<Vec<u8>>>,
    }

    impl TestProviderActuator {
        fn set_response(&self, response: Vec<u8>) {
            *self.response.lock() = response;
        }

        fn set_cancel_during_call(&self, value: bool) {
            self.cancel_during_call.store(value, Ordering::Release);
        }

        fn set_fail_call(&self, value: bool) {
            self.fail_call.store(value, Ordering::Release);
        }
    }

    impl ProviderRequestActuator for TestProviderActuator {
        fn execute(
            &self,
            request: &AuthorizedProviderRequest,
            secret: Option<&SecretValue>,
            _body: &[u8],
            context: &PluginServiceOperationContext<'_>,
        ) -> Result<Vec<u8>, PluginServiceActuatorError> {
            context
                .check_active()
                .map_err(|error| PluginServiceActuatorError::new(error.to_string()))?;
            self.calls.fetch_add(1, Ordering::AcqRel);
            *self.last_authorized_request.lock() = Some((
                request.provider().to_owned(),
                request.endpoint().to_owned(),
                request
                    .secret_id()
                    .map(|secret_id| secret_id.as_str().to_owned()),
            ));
            *self.last_secret.lock() = secret.map(|secret| secret.expose_to(<[u8]>::to_vec));
            if self.cancel_during_call.load(Ordering::Acquire) {
                context.cancellation().cancel();
            }
            if self.fail_call.load(Ordering::Acquire) {
                return Err(PluginServiceActuatorError::new(
                    "injected provider failure",
                ));
            }
            Ok(self.response.lock().clone())
        }
    }

    #[derive(Default)]
    struct TestCredentialPresenceActuator {
        calls: AtomicUsize,
        present: AtomicBool,
        last_secret_id: Mutex<Option<String>>,
    }

    impl CredentialPresenceActuator for TestCredentialPresenceActuator {
        fn is_present(
            &self,
            request: &AuthorizedCredentialPresenceRequest,
            context: &PluginServiceOperationContext<'_>,
        ) -> Result<bool, PluginServiceActuatorError> {
            context
                .check_active()
                .map_err(|error| PluginServiceActuatorError::new(error.to_string()))?;
            self.calls.fetch_add(1, Ordering::AcqRel);
            *self.last_secret_id.lock() = Some(request.secret_id().as_str().to_owned());
            Ok(self.present.load(Ordering::Acquire))
        }

        fn read_for_provider(
            &self,
            request: &AuthorizedCredentialPresenceRequest,
            context: &PluginServiceOperationContext<'_>,
        ) -> Result<Option<SecretValue>, PluginServiceActuatorError> {
            context
                .check_active()
                .map_err(|error| PluginServiceActuatorError::new(error.to_string()))?;
            self.calls.fetch_add(1, Ordering::AcqRel);
            *self.last_secret_id.lock() = Some(request.secret_id().as_str().to_owned());
            Ok(self
                .present
                .load(Ordering::Acquire)
                .then(|| SecretValue::new(b"fixture-secret-value".to_vec())))
        }
    }

    fn capability(kind: CapabilityKind, scope: &str) -> CapabilityRequest {
        CapabilityRequest {
            kind,
            scope: scope.to_owned(),
            quota: CapabilityQuota {
                maximum_operations: 64,
                maximum_request_bytes: MAX_PLUGIN_SERVICE_REQUEST_BYTES,
                maximum_response_bytes: MAX_PLUGIN_SERVICE_RESPONSE_BYTES,
                maximum_total_bytes: MAX_PLUGIN_SERVICE_RESPONSE_BYTES,
                maximum_handles: 16,
                timeout_milliseconds: 60_000,
            },
        }
    }

    fn authorization(
        requested_capabilities: Vec<CapabilityRequest>,
    ) -> Result<PluginAuthorization, Box<dyn Error>> {
        let signing_key = PluginSigningKey::new("fixture.key", SIGNING_KEY)?;
        let trust = PluginTrustPolicy::new([PluginVerificationKey::new(
            "fixture.key",
            signing_key.verification_key_bytes()?,
        )?])?;
        let mut manifest = PluginManifest {
            schema_version: 1,
            identifier: "plugin.fixture".to_owned(),
            plugin_version: ApiVersion::new(1, 0, 0),
            api: ApiRequirement {
                major: 1,
                minimum_minor: 0,
                maximum_minor: 0,
                required_features: Vec::new(),
            },
            digest_sha256: DIGEST.to_owned(),
            signature: ManifestSignature {
                algorithm: PLUGIN_SIGNATURE_ALGORITHM.to_owned(),
                key_id: "fixture.key".to_owned(),
                value: "0".repeat(ED25519_SIGNATURE_BYTES * 2),
            },
            provenance: ManifestProvenance {
                source: "fixture://plugin.fixture".to_owned(),
                publisher: "fixture publisher".to_owned(),
                registry: Some("fixture://registry".to_owned()),
            },
            provider_binding: None,
            nodes: vec![PluginNode {
                id: "node.fixture".to_owned(),
                version: ApiVersion::new(1, 0, 0),
                display_name: "Fixture".to_owned(),
                category: "tests".to_owned(),
                ports: Vec::new(),
                determinism: DeterminismPolicy::Deterministic,
                cache: CachePolicy::InputIdentity,
                effects: EffectPolicy::Pure,
            }],
            capabilities: requested_capabilities.clone(),
            ui: Vec::new(),
            routes: Vec::new(),
            legacy_mappings: Vec::new(),
        };
        manifest.signature.value = signing_key.sign_manifest(&manifest)?;
        let domain_capabilities = requested_capabilities
            .iter()
            .map(Capability::from_plugin_request)
            .collect::<Result<Vec<_>, _>>()?;
        let permissions = PermissionPolicy::new(
            PROFILE_UUID.to_string(),
            [PermissionGrant::new(
                PROFILE_UUID.to_string(),
                "plugin.fixture",
                CapabilitySet::new(domain_capabilities),
                "test-profile-settings",
            )?],
        )?;
        Ok(trust.authorize_manifest(&manifest, &permissions)?)
    }

    fn asset_service() -> Result<(TempDir, SharedAssetService), Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let input_root = directory.path().join("input");
        let output_root = directory.path().join("output");
        let temporary_root = directory.path().join("temporary");
        let model_root = directory.path().join("model");
        let plugin_root = directory.path().join("plugin");
        for root in [
            &input_root,
            &output_root,
            &temporary_root,
            &model_root,
            &plugin_root,
        ] {
            fs::create_dir_all(root)?;
        }
        fs::write(input_root.join("fixture.bin"), b"canonical asset bytes")?;
        fs::write(
            model_root.join("fixture.json"),
            br#"{"model_type":"fixture","hidden_size":4}"#,
        )?;
        let roots = AssetRoots::new(
            PROFILE_UUID.to_string(),
            [
                (AssetNamespace::Input, input_root),
                (AssetNamespace::Output, output_root),
                (AssetNamespace::Temporary, temporary_root),
                (AssetNamespace::Model, model_root),
                (AssetNamespace::Plugin, plugin_root),
            ],
        )?;
        let mut service = AssetService::open(roots)?;
        let setup_capabilities = CapabilitySet::new(
            [
                AssetNamespace::Input,
                AssetNamespace::Output,
                AssetNamespace::Temporary,
                AssetNamespace::Model,
                AssetNamespace::Plugin,
            ]
            .into_iter()
            .map(|namespace| Capability::Asset {
                namespace: namespace.locator_type().to_owned(),
                action: AssetOperation::Read,
            }),
        );
        let setup_policy = PermissionPolicy::new(
            PROFILE_UUID.to_string(),
            [PermissionGrant::new(
                PROFILE_UUID.to_string(),
                "test.asset-indexer",
                setup_capabilities.clone(),
                "test-fixture-setup",
            )?],
        )?;
        let setup_authorization =
            setup_policy.authorize("test.asset-indexer", &setup_capabilities)?;
        service.scan(&setup_authorization, &CancellationToken::default())?;
        let service = Arc::new(std::sync::Mutex::new(service));
        Ok((directory, service))
    }

    type BrokerFixture = (
        TempDir,
        PluginCapabilityBroker,
        Arc<TestProviderActuator>,
        Arc<TestCredentialPresenceActuator>,
    );

    fn broker(
        authorization: &PluginAuthorization,
        clock: Arc<TestClock>,
    ) -> Result<BrokerFixture, Box<dyn Error>> {
        broker_with_cost_acceptance(authorization, clock, None)
    }

    fn broker_with_cost_acceptance(
        authorization: &PluginAuthorization,
        clock: Arc<TestClock>,
        verifier: Option<ProviderCostAcceptanceVerifier>,
    ) -> Result<BrokerFixture, Box<dyn Error>> {
        assert_eq!(
            authorization.capabilities().profile_id(),
            PROFILE_UUID.to_string()
        );
        let (directory, assets) = asset_service()?;
        let provider = Arc::new(TestProviderActuator::default());
        let credential = Arc::new(TestCredentialPresenceActuator::default());
        let provider_policy = ProviderPolicy::new(
            PROFILE_UUID.to_string(),
            ProviderMode::Enabled,
            [ProviderEndpoint::new(PROVIDER, ENDPOINT)?],
            [crate::CredentialScope::new(
                PROFILE_UUID.to_string(),
                "plugin.fixture",
                PROVIDER,
                SecretId::new(SECRET)?,
            )?],
        )?;
        let model_store = ModelStore::new(ParserLimits::default())?;
        let rng_policy =
            PluginRngPolicy::new(RngProfileVersion::V2, RngAlgorithm::Philox4x32_10, 7);
        let broker = if let Some(verifier) = verifier {
            PluginCapabilityBroker::new_with_provider_cost_acceptance(
                assets,
                model_store,
                provider_policy,
                verifier,
                provider.clone(),
                credential.clone(),
                clock,
                rng_policy,
            )
        } else {
            PluginCapabilityBroker::new(
                assets,
                model_store,
                provider_policy,
                provider.clone(),
                credential.clone(),
                clock,
                rng_policy,
            )
        };
        Ok((directory, broker, provider, credential))
    }

    fn context(
        authorization: PluginAuthorization,
        clock: &TestClock,
        cancellation: CancellationToken,
        maximum_response_bytes: u64,
    ) -> Result<PluginServiceInvocationContext, PluginServiceError> {
        PluginServiceInvocationContext::new(
            ProfileId(PROFILE_UUID),
            PromptId(PROMPT_UUID),
            AttemptId(ATTEMPT_UUID),
            NodeId("node.fixture".to_owned()),
            authorization,
            cancellation,
            clock.now() + Duration::from_secs(30),
            maximum_response_bytes,
        )
    }

    fn priced_context(
        authorization: PluginAuthorization,
        clock: &TestClock,
    ) -> Result<PluginServiceInvocationContext, PluginServiceError> {
        PluginServiceInvocationContext::new_with_principal(
            ProfileId(PROFILE_UUID),
            PromptId(PROMPT_UUID),
            AttemptId(ATTEMPT_UUID),
            NodeId("node.fixture".to_owned()),
            "principal-a",
            authorization,
            CancellationToken::default(),
            clock.now() + Duration::from_secs(30),
            1_024,
        )
    }

    fn provider_cost_scope(
        provider_binding_sha256: &str,
        provider: &str,
        endpoint: &str,
        price_bound: ProviderPriceBound,
    ) -> Result<ProviderCostAcceptanceScope, TrustError> {
        ProviderCostAcceptanceScope::new(
            "principal-a",
            PROFILE_UUID.to_string(),
            PROMPT_UUID.to_string(),
            "plugin.fixture",
            DIGEST,
            provider_binding_sha256,
            provider,
            endpoint,
            price_bound,
        )
    }

    #[test]
    fn capability_denials_happen_before_provider_or_credential_actuators()
    -> Result<(), Box<dyn Error>> {
        let authorization = authorization(Vec::new())?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, provider, credential) = broker(&authorization, clock.clone())?;
        let invocation = broker.begin_invocation(context(
            authorization,
            &clock,
            CancellationToken::default(),
            1_024,
        )?)?;
        let secret = SecretId::new(SECRET)?;

        assert!(matches!(
            invocation.execute_provider_request(PROVIDER, ENDPOINT, Some(&secret), b"request"),
            Err(PluginServiceError::CapabilityDenied(
                Capability::ProviderNetwork { .. }
            ))
        ));
        assert!(matches!(
            invocation.credential_is_present(&secret),
            Err(PluginServiceError::CapabilityDenied(
                Capability::Secret { .. }
            ))
        ));
        assert_eq!(provider.calls.load(Ordering::Acquire), 0);
        assert_eq!(credential.calls.load(Ordering::Acquire), 0);
        Ok(())
    }

    #[test]
    fn provider_cost_acceptance_denials_make_zero_priced_actuator_calls()
    -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![capability(
            CapabilityKind::NetworkProvider,
            &format!("{PROVIDER}|{ENDPOINT}"),
        )])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let issuer = ProviderCostAcceptanceIssuer::from_seed([11; 32], clock.now())?;
        let (_directory, broker, provider, credential) =
            broker_with_cost_acceptance(&authorization, clock.clone(), Some(issuer.verifier()?))?;
        let binding = "a".repeat(64);
        let other_binding = "b".repeat(64);
        let price = ProviderPriceBound::new("USD", 25_000)?;
        let nonce = ProviderCostNonce::new([21; 32])?;
        let acceptance = issuer.issue(
            provider_cost_scope(&binding, PROVIDER, ENDPOINT, price.clone())?,
            clock.now(),
            clock.now() + Duration::from_secs(10),
            nonce,
        )?;

        let invocation = broker.begin_invocation(priced_context(authorization.clone(), &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &binding, PROVIDER, ENDPOINT, None, &price, nonce, None, b"request",
            ),
            Err(PluginServiceError::ProviderCostAcceptanceRequired)
        );

        let foreign_issuer = ProviderCostAcceptanceIssuer::from_seed([12; 32], clock.now())?;
        let forged = foreign_issuer.issue(
            provider_cost_scope(&binding, PROVIDER, ENDPOINT, price.clone())?,
            clock.now(),
            clock.now() + Duration::from_secs(10),
            ProviderCostNonce::new([22; 32])?,
        )?;
        let invocation = broker.begin_invocation(priced_context(authorization.clone(), &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &binding,
                PROVIDER,
                ENDPOINT,
                None,
                &price,
                forged.nonce(),
                Some(&forged),
                b"request",
            ),
            Err(PluginServiceError::ProviderCostAcceptanceDenied)
        );

        let expired_nonce = ProviderCostNonce::new([23; 32])?;
        let expired = issuer.issue(
            provider_cost_scope(&binding, PROVIDER, ENDPOINT, price.clone())?,
            clock.now(),
            clock.now() + Duration::from_millis(1),
            expired_nonce,
        )?;
        clock.advance(Duration::from_millis(1));
        let invocation = broker.begin_invocation(priced_context(authorization.clone(), &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &binding,
                PROVIDER,
                ENDPOINT,
                None,
                &price,
                expired_nonce,
                Some(&expired),
                b"request",
            ),
            Err(PluginServiceError::ProviderCostAcceptanceDenied)
        );

        let invocation = broker.begin_invocation(priced_context(authorization.clone(), &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &other_binding,
                PROVIDER,
                ENDPOINT,
                None,
                &price,
                nonce,
                Some(&acceptance),
                b"request",
            ),
            Err(PluginServiceError::ProviderCostAcceptanceDenied)
        );
        let invocation = broker.begin_invocation(priced_context(authorization, &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &binding,
                PROVIDER,
                ENDPOINT,
                None,
                &price,
                ProviderCostNonce::new([24; 32])?,
                Some(&acceptance),
                b"request",
            ),
            Err(PluginServiceError::ProviderCostAcceptanceDenied)
        );

        assert_eq!(provider.calls.load(Ordering::Acquire), 0);
        assert_eq!(credential.calls.load(Ordering::Acquire), 0);
        Ok(())
    }

    #[test]
    fn provider_cost_acceptance_is_single_use_and_legacy_requests_stay_unpriced()
    -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![capability(
            CapabilityKind::NetworkProvider,
            &format!("{PROVIDER}|{ENDPOINT}"),
        )])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let issuer = ProviderCostAcceptanceIssuer::from_seed([31; 32], clock.now())?;
        let (_directory, broker, provider, _credential) =
            broker_with_cost_acceptance(&authorization, clock.clone(), Some(issuer.verifier()?))?;
        provider.set_response(b"provider-response".to_vec());
        let binding = "c".repeat(64);
        let price = ProviderPriceBound::new("USD", 30_000)?;
        let nonce = ProviderCostNonce::new([32; 32])?;
        let acceptance = issuer.issue(
            provider_cost_scope(&binding, PROVIDER, ENDPOINT, price.clone())?,
            clock.now(),
            clock.now() + Duration::from_secs(10),
            nonce,
        )?;

        let invocation = broker.begin_invocation(priced_context(authorization.clone(), &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &binding,
                PROVIDER,
                ENDPOINT,
                None,
                &price,
                nonce,
                Some(&acceptance),
                b"request",
            )?,
            b"provider-response"
        );
        let invocation = broker.begin_invocation(priced_context(authorization.clone(), &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &binding,
                PROVIDER,
                ENDPOINT,
                None,
                &price,
                nonce,
                Some(&acceptance),
                b"request",
            ),
            Err(PluginServiceError::ProviderCostAcceptanceReused)
        );

        let legacy = broker.begin_invocation(context(
            authorization,
            &clock,
            CancellationToken::default(),
            1_024,
        )?)?;
        assert_eq!(
            legacy.execute_provider_request(PROVIDER, ENDPOINT, None, b"request")?,
            b"provider-response"
        );
        assert_eq!(provider.calls.load(Ordering::Acquire), 2);

        let wire = serde_json::to_value(PluginServiceWireRequest::ExecuteProvider {
            provider: PROVIDER.to_owned(),
            endpoint: ENDPOINT.to_owned(),
            secret_id: None,
            body: b"request".to_vec(),
        })?;
        let wire_text = serde_json::to_string(&wire)?;
        assert!(!wire_text.contains("acceptance"));
        assert!(!wire_text.contains("nonce"));
        assert!(!wire_text.contains("price"));
        Ok(())
    }

    fn provider_cost_nonce(index: u64) -> Result<ProviderCostNonce, TrustError> {
        let mut bytes = [0_u8; 32];
        bytes[..8].copy_from_slice(&index.checked_add(1).unwrap_or(u64::MAX).to_le_bytes());
        ProviderCostNonce::new(bytes)
    }

    #[test]
    fn provider_cost_nonce_claims_expire_roll_back_and_remain_bounded() -> Result<(), Box<dyn Error>>
    {
        let authorization = authorization(vec![capability(
            CapabilityKind::NetworkProvider,
            &format!("{PROVIDER}|{ENDPOINT}"),
        )])?;
        let now = Instant::now();
        let clock = Arc::new(TestClock::new(now));
        let (_directory, broker, _provider, _credential) =
            broker_with_cost_acceptance(&authorization, clock, None)?;

        assert!(matches!(
            broker
                .inner
                .claim_provider_cost_nonce(provider_cost_nonce(0)?, now, now,),
            Err(PluginServiceError::ProviderCostAcceptanceDenied)
        ));
        assert!(broker.inner.consumed_provider_cost_nonces.lock().is_empty());

        {
            let _claim = broker.inner.claim_provider_cost_nonce(
                provider_cost_nonce(1)?,
                now + Duration::from_secs(1),
                now,
            )?;
        }
        assert!(broker.inner.consumed_provider_cost_nonces.lock().is_empty());

        {
            let mut consumed_nonces = broker.inner.consumed_provider_cost_nonces.lock();
            for index in 0..MAX_CONSUMED_PROVIDER_COST_NONCES {
                consumed_nonces.insert(
                    provider_cost_nonce(u64::try_from(index)?.checked_add(10).unwrap_or(u64::MAX))?,
                    now,
                );
            }
        }
        broker
            .inner
            .claim_provider_cost_nonce(provider_cost_nonce(2)?, now + Duration::from_secs(1), now)?
            .retain();
        let consumed_nonces = broker.inner.consumed_provider_cost_nonces.lock();
        assert_eq!(consumed_nonces.len(), 1);
        assert_eq!(
            consumed_nonces.get(&provider_cost_nonce(2)?),
            Some(&(now + Duration::from_secs(1)))
        );
        Ok(())
    }

    #[test]
    fn concurrent_provider_cost_nonce_claims_admit_one_caller() -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![capability(
            CapabilityKind::NetworkProvider,
            &format!("{PROVIDER}|{ENDPOINT}"),
        )])?;
        let now = Instant::now();
        let clock = Arc::new(TestClock::new(now));
        let (_directory, broker, _provider, _credential) =
            broker_with_cost_acceptance(&authorization, clock, None)?;
        let barrier = Arc::new(Barrier::new(3));
        let nonce = provider_cost_nonce(100)?;
        let handles = (0..2)
            .map(|_| {
                let broker = broker.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || -> Result<bool, PluginServiceError> {
                    barrier.wait();
                    match broker.inner.claim_provider_cost_nonce(
                        nonce,
                        now + Duration::from_secs(1),
                        now,
                    ) {
                        Ok(claim) => {
                            claim.retain();
                            Ok(true)
                        }
                        Err(PluginServiceError::ProviderCostAcceptanceReused) => Ok(false),
                        Err(error) => Err(error),
                    }
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let admitted = handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .map_err(|_| "provider cost claim thread panicked".to_owned())?
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        assert_eq!(admitted.into_iter().filter(|admitted| *admitted).count(), 1);
        assert_eq!(broker.inner.consumed_provider_cost_nonces.lock().len(), 1);
        Ok(())
    }

    #[test]
    fn provider_cost_nonce_rolls_back_before_actuation_and_is_retained_after_attempt()
    -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![
            capability(
                CapabilityKind::NetworkProvider,
                &format!("{PROVIDER}|{ENDPOINT}"),
            ),
            capability(CapabilityKind::Secret, SECRET),
        ])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let issuer = ProviderCostAcceptanceIssuer::from_seed([41; 32], clock.now())?;
        let (_directory, broker, provider, credential) = broker_with_cost_acceptance(
            &authorization,
            clock.clone(),
            Some(issuer.verifier()?),
        )?;
        provider.set_response(b"provider-response".to_vec());
        let binding = "d".repeat(64);
        let price = ProviderPriceBound::new("USD", 40_000)?;
        let credential_nonce = ProviderCostNonce::new([42; 32])?;
        let credential_acceptance = issuer.issue(
            provider_cost_scope(&binding, PROVIDER, ENDPOINT, price.clone())?,
            clock.now(),
            clock.now() + Duration::from_secs(10),
            credential_nonce,
        )?;
        let secret = SecretId::new(SECRET)?;

        let invocation =
            broker.begin_invocation(priced_context(authorization.clone(), &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &binding,
                PROVIDER,
                ENDPOINT,
                Some(&secret),
                &price,
                credential_nonce,
                Some(&credential_acceptance),
                b"request",
            ),
            Err(PluginServiceError::CredentialUnavailable)
        );
        assert_eq!(provider.calls.load(Ordering::Acquire), 0);
        assert_eq!(credential.calls.load(Ordering::Acquire), 1);

        credential.present.store(true, Ordering::Release);
        let invocation =
            broker.begin_invocation(priced_context(authorization.clone(), &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &binding,
                PROVIDER,
                ENDPOINT,
                Some(&secret),
                &price,
                credential_nonce,
                Some(&credential_acceptance),
                b"request",
            )?,
            b"provider-response"
        );

        let failed_nonce = ProviderCostNonce::new([43; 32])?;
        let failed_acceptance = issuer.issue(
            provider_cost_scope(&binding, PROVIDER, ENDPOINT, price.clone())?,
            clock.now(),
            clock.now() + Duration::from_secs(10),
            failed_nonce,
        )?;
        provider.set_fail_call(true);
        let invocation =
            broker.begin_invocation(priced_context(authorization.clone(), &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &binding,
                PROVIDER,
                ENDPOINT,
                None,
                &price,
                failed_nonce,
                Some(&failed_acceptance),
                b"request",
            ),
            Err(PluginServiceError::ActuatorFailed {
                service: "provider",
                message: "injected provider failure".to_owned(),
            })
        );

        provider.set_fail_call(false);
        let invocation = broker.begin_invocation(priced_context(authorization, &clock)?)?;
        assert_eq!(
            invocation.execute_priced_provider_request(
                &binding,
                PROVIDER,
                ENDPOINT,
                None,
                &price,
                failed_nonce,
                Some(&failed_acceptance),
                b"request",
            ),
            Err(PluginServiceError::ProviderCostAcceptanceReused)
        );
        assert_eq!(provider.calls.load(Ordering::Acquire), 2);
        assert_eq!(credential.calls.load(Ordering::Acquire), 2);
        Ok(())
    }

    #[test]
    fn provider_policy_seals_requests_and_cancellation_and_bounds_are_rechecked()
    -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![
            capability(
                CapabilityKind::NetworkProvider,
                &format!("{PROVIDER}|{ENDPOINT}"),
            ),
            capability(CapabilityKind::Secret, SECRET),
        ])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, provider, credential) = broker(&authorization, clock.clone())?;
        credential.present.store(true, Ordering::Release);
        let secret = SecretId::new(SECRET)?;
        provider.set_response(vec![0; 17]);
        let invocation = broker.begin_invocation(context(
            authorization.clone(),
            &clock,
            CancellationToken::default(),
            16,
        )?)?;
        assert_eq!(
            invocation.execute_provider_request(PROVIDER, ENDPOINT, Some(&secret), b"request"),
            Err(PluginServiceError::ResponseTooLarge { maximum: 16 })
        );
        assert_eq!(provider.calls.load(Ordering::Acquire), 1);
        assert_eq!(credential.calls.load(Ordering::Acquire), 1);
        assert_eq!(
            *provider.last_secret.lock(),
            Some(b"fixture-secret-value".to_vec())
        );
        assert_eq!(
            *provider.last_authorized_request.lock(),
            Some((
                PROVIDER.to_owned(),
                ENDPOINT.to_owned(),
                Some(SECRET.to_owned())
            ))
        );

        provider.set_response(b"ok".to_vec());
        provider.set_cancel_during_call(true);
        let cancellation = CancellationToken::default();
        let invocation =
            broker.begin_invocation(context(authorization, &clock, cancellation.clone(), 16)?)?;
        assert_eq!(
            invocation.execute_provider_request(PROVIDER, ENDPOINT, Some(&secret), b"request"),
            Err(PluginServiceError::Cancelled)
        );
        assert!(cancellation.is_cancelled());
        Ok(())
    }

    #[test]
    fn deadlines_and_response_limits_fail_before_work_starts() -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![capability(
            CapabilityKind::NetworkProvider,
            &format!("{PROVIDER}|{ENDPOINT}"),
        )])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, provider, _credential) = broker(&authorization, clock.clone())?;
        assert!(matches!(
            PluginServiceInvocationContext::new(
                ProfileId(PROFILE_UUID),
                PromptId(PROMPT_UUID),
                AttemptId(ATTEMPT_UUID),
                NodeId("node.fixture".to_owned()),
                authorization.clone(),
                CancellationToken::default(),
                clock.now() + Duration::from_secs(1),
                MAX_PLUGIN_SERVICE_RESPONSE_BYTES + 1,
            ),
            Err(PluginServiceError::InvalidResponseLimit {
                maximum: MAX_PLUGIN_SERVICE_RESPONSE_BYTES
            })
        ));
        let context = PluginServiceInvocationContext::new(
            ProfileId(PROFILE_UUID),
            PromptId(PROMPT_UUID),
            AttemptId(ATTEMPT_UUID),
            NodeId("node.fixture".to_owned()),
            authorization,
            CancellationToken::default(),
            clock.now() + Duration::from_millis(1),
            16,
        )?;
        clock.advance(Duration::from_millis(1));
        assert!(matches!(
            broker.begin_invocation(context),
            Err(PluginServiceError::DeadlineExceeded)
        ));
        assert_eq!(provider.calls.load(Ordering::Acquire), 0);
        Ok(())
    }

    #[test]
    fn rng_aborts_roll_back_and_successful_invocations_commit() -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![capability(CapabilityKind::Randomness, "noise")])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, _provider, _credential) = broker(&authorization, clock.clone())?;
        let invocation_context =
            context(authorization, &clock, CancellationToken::default(), 1_024)?;

        let mut aborted = broker.begin_invocation(invocation_context.clone())?;
        let first = aborted.random_bytes("noise", 32)?;
        aborted.abort();

        let mut committed = broker.begin_invocation(invocation_context.clone())?;
        let replayed = committed.random_bytes("noise", 32)?;
        assert_eq!(replayed, first);
        committed.finish()?;

        let mut advanced = broker.begin_invocation(invocation_context)?;
        let next = advanced.random_bytes("noise", 32)?;
        assert_ne!(next, first);
        advanced.finish()?;
        Ok(())
    }

    #[test]
    fn asset_and_model_operations_delegate_to_canonical_index_without_path_exposure()
    -> Result<(), Box<dyn Error>> {
        let model_id = "sim-asset://model/fixture.json";
        let authorization = authorization(vec![
            capability(CapabilityKind::Filesystem, "input"),
            capability(CapabilityKind::Filesystem, "model"),
            capability(CapabilityKind::Model, model_id),
        ])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, _provider, _credential) = broker(&authorization, clock.clone())?;
        let invocation = broker.begin_invocation(context(
            authorization,
            &clock,
            CancellationToken::default(),
            4_096,
        )?)?;

        assert_eq!(
            invocation.read_asset(AssetNamespace::Input, "sim-asset://input/fixture.bin")?,
            b"canonical asset bytes"
        );
        let model = invocation.load_model(model_id)?;
        assert_eq!(model.model_id(), model_id);
        assert_eq!(model.model().documents().len(), 1);
        let debug = format!("{model:?}");
        assert!(!debug.contains(_directory.path().to_string_lossy().as_ref()));

        let error = invocation
            .read_asset(
                AssetNamespace::Input,
                "sim-asset://input/../../outside-secret",
            )
            .expect_err("canonical root must reject traversal");
        let error_text = error.to_string();
        assert!(!error_text.contains("outside-secret"));
        assert!(!error_text.contains(_directory.path().to_string_lossy().as_ref()));
        Ok(())
    }

    #[test]
    fn credential_presence_and_clock_use_injected_operation_only_owners()
    -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![
            capability(CapabilityKind::Secret, SECRET),
            capability(CapabilityKind::Clock, "monotonic"),
        ])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, _provider, credential) = broker(&authorization, clock.clone())?;
        credential.present.store(true, Ordering::Release);
        let invocation = broker.begin_invocation(context(
            authorization,
            &clock,
            CancellationToken::default(),
            128,
        )?)?;
        let secret = SecretId::new(SECRET)?;

        assert!(invocation.credential_is_present(&secret)?);
        assert_eq!(credential.calls.load(Ordering::Acquire), 1);
        assert_eq!(*credential.last_secret_id.lock(), Some(SECRET.to_owned()));
        assert_eq!(invocation.monotonic_milliseconds("monotonic")?, 0);
        clock.advance(Duration::from_millis(25));
        assert_eq!(invocation.monotonic_milliseconds("monotonic")?, 25);
        Ok(())
    }

    #[test]
    fn cancelled_rng_work_does_not_advance_the_stream() -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![capability(CapabilityKind::Randomness, "noise")])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, _provider, _credential) = broker(&authorization, clock.clone())?;
        let baseline_context = context(
            authorization.clone(),
            &clock,
            CancellationToken::default(),
            1_024,
        )?;
        let mut baseline = broker.begin_invocation(baseline_context)?;
        let expected = baseline.random_bytes("noise", 16)?;
        baseline.abort();

        let cancellation = CancellationToken::default();
        let cancelled_context =
            context(authorization.clone(), &clock, cancellation.clone(), 1_024)?;
        let mut cancelled = broker.begin_invocation(cancelled_context)?;
        cancellation.cancel();
        assert_eq!(
            cancelled.random_bytes("noise", 16),
            Err(PluginServiceError::Cancelled)
        );
        cancelled.abort();

        let mut replay = broker.begin_invocation(context(
            authorization,
            &clock,
            CancellationToken::default(),
            1_024,
        )?)?;
        assert_eq!(replay.random_bytes("noise", 16)?, expected);
        replay.finish()?;
        Ok(())
    }

    #[test]
    fn asset_namespace_adapter_cannot_expand_a_signed_grant() -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![capability(CapabilityKind::Filesystem, "input")])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, _provider, _credential) = broker(&authorization, clock.clone())?;
        let invocation = broker.begin_invocation(context(
            authorization,
            &clock,
            CancellationToken::default(),
            1_024,
        )?)?;

        assert!(matches!(
            invocation.read_asset(
                AssetNamespace::Model,
                "sim-asset://model/fixture.json"
            ),
            Err(PluginServiceError::CapabilityDenied(Capability::Asset {
                namespace,
                action: AssetOperation::Read,
            })) if namespace == "model"
        ));
        Ok(())
    }

    #[test]
    fn model_loading_requires_both_opaque_handle_and_canonical_asset_grants()
    -> Result<(), Box<dyn Error>> {
        let model_id = "sim-asset://model/fixture.json";
        let authorization = authorization(vec![capability(CapabilityKind::Model, model_id)])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, _provider, _credential) = broker(&authorization, clock.clone())?;
        let invocation = broker.begin_invocation(context(
            authorization,
            &clock,
            CancellationToken::default(),
            4_096,
        )?)?;

        assert!(matches!(
            invocation.load_model(model_id),
            Err(PluginServiceError::CapabilityDenied(Capability::Asset {
                namespace,
                action: AssetOperation::Read,
            })) if namespace == "model"
        ));
        Ok(())
    }

    #[test]
    fn provider_endpoint_policy_cannot_be_bypassed_by_a_broader_capability()
    -> Result<(), Box<dyn Error>> {
        let denied_endpoint = "https://provider.invalid/v2";
        let authorization = authorization(vec![capability(
            CapabilityKind::NetworkProvider,
            &format!("{PROVIDER}|{denied_endpoint}"),
        )])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, provider, _credential) = broker(&authorization, clock.clone())?;
        let invocation = broker.begin_invocation(context(
            authorization,
            &clock,
            CancellationToken::default(),
            1_024,
        )?)?;

        assert_eq!(
            invocation.execute_provider_request(PROVIDER, denied_endpoint, None, b"request"),
            Err(PluginServiceError::ProviderPolicyDenied)
        );
        assert_eq!(provider.calls.load(Ordering::Acquire), 0);
        Ok(())
    }

    #[test]
    fn requested_random_response_is_bounded_before_allocation() -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![capability(CapabilityKind::Randomness, "noise")])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, _provider, _credential) = broker(&authorization, clock.clone())?;
        let mut invocation = broker.begin_invocation(context(
            authorization,
            &clock,
            CancellationToken::default(),
            8,
        )?)?;

        assert_eq!(
            invocation.random_bytes("noise", 9),
            Err(PluginServiceError::ResponseTooLarge { maximum: 8 })
        );
        invocation.abort();
        Ok(())
    }

    #[test]
    fn plugin_model_handle_does_not_expose_mutable_store_or_paths() -> Result<(), Box<dyn Error>> {
        let model_id = "sim-asset://model/fixture.json";
        let authorization = authorization(vec![
            capability(CapabilityKind::Filesystem, "model"),
            capability(CapabilityKind::Model, model_id),
        ])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (directory, broker, _provider, _credential) = broker(&authorization, clock.clone())?;
        let invocation = broker.begin_invocation(context(
            authorization,
            &clock,
            CancellationToken::default(),
            4_096,
        )?)?;
        let model = invocation.load_model(model_id)?;

        assert_eq!(model.model_id(), model_id);
        assert_eq!(model.model_format(), "json-config");
        assert!(!model.model_identity().is_empty());
        assert!(!format!("{model:?}").contains(directory.path().to_string_lossy().as_ref()));
        assert_eq!(model.model().accounting(), model.accounting());
        Ok(())
    }

    #[test]
    fn dropped_invocations_release_rng_leases_without_committing() -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![capability(CapabilityKind::Randomness, "noise")])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, _provider, _credential) = broker(&authorization, clock.clone())?;
        let invocation_context =
            context(authorization, &clock, CancellationToken::default(), 1_024)?;
        let expected = {
            let mut invocation = broker.begin_invocation(invocation_context.clone())?;
            invocation.random_bytes("noise", 8)?
        };
        let mut replay = broker.begin_invocation(invocation_context)?;
        assert_eq!(replay.random_bytes("noise", 8)?, expected);
        replay.finish()?;
        Ok(())
    }

    #[test]
    fn failed_capability_operations_prevent_rng_commit() -> Result<(), Box<dyn Error>> {
        let authorization = authorization(vec![capability(CapabilityKind::Randomness, "noise")])?;
        let clock = Arc::new(TestClock::new(Instant::now()));
        let (_directory, broker, _provider, _credential) = broker(&authorization, clock.clone())?;
        let invocation_context =
            context(authorization, &clock, CancellationToken::default(), 1_024)?;
        let mut failed = broker.begin_invocation(invocation_context.clone())?;
        let expected = failed.random_bytes("noise", 8)?;
        assert!(matches!(
            failed.read_asset(AssetNamespace::Input, "sim-asset://input/fixture.bin"),
            Err(PluginServiceError::CapabilityDenied(
                Capability::Asset { .. }
            ))
        ));
        assert_eq!(failed.finish(), Err(PluginServiceError::InvocationFailed));

        let mut replay = broker.begin_invocation(invocation_context)?;
        assert_eq!(replay.random_bytes("noise", 8)?, expected);
        replay.finish()?;
        Ok(())
    }
}
