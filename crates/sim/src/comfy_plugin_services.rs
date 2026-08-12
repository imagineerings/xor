use comfy_model::{ModelStore, ParserLimits};
use comfy_plugin_host::{
    ComponentExecutionBoundary, ComponentHost, ComponentHostError,
    ComponentHostProviderInvocationAuthority, PrivateWorkerPluginExecutor,
};
use comfy_runtime::{
    AuthorizedCredentialPresenceRequest, AuthorizedProviderRequest, CredentialPresenceActuator,
    PluginCapabilityBroker, PluginRngPolicy, PluginServiceActuatorError,
    PluginServiceOperationContext, ProviderCostAcceptanceIssuer, ProviderCostApprovalAuthority,
    ProviderPolicy, ProviderRequestActuator, ProviderResultReceiptIssuer, SecretValue,
    SharedAssetService, WorkerLaunchConfig,
};
use comfy_tensor::{RngAlgorithm, RngProfileVersion};
use futures::AsyncReadExt as _;
use gpui::{App, BackgroundExecutor};
use http_client::{AsyncBody, HttpClient, Request, http};
use std::sync::Arc;
use std::{
    future::Future,
    time::{Duration, Instant},
};

pub struct SimComfyPluginServices {
    pub boundary: ComponentExecutionBoundary,
    broker: PluginCapabilityBroker,
    principal_id: String,
    receipt_issuer: Arc<ProviderResultReceiptIssuer>,
    receipt_lifetime: Duration,
    cost_authority: Arc<ProviderCostApprovalAuthority>,
}

impl SimComfyPluginServices {
    pub fn invocation_authority(
        &self,
        host: ComponentHost,
    ) -> Result<Arc<ComponentHostProviderInvocationAuthority>, ComponentHostError> {
        Ok(Arc::new(
            ComponentHostProviderInvocationAuthority::new(
                host,
                self.broker.clone(),
                &self.principal_id,
                self.receipt_issuer.clone(),
                self.receipt_lifetime,
            )
            .map_err(component_boundary_error)?
            .with_cost_authority(self.cost_authority.clone()),
        ))
    }

    pub fn cost_authority(&self) -> Arc<ProviderCostApprovalAuthority> {
        self.cost_authority.clone()
    }
}

pub fn private_worker_services(
    launch: WorkerLaunchConfig,
    assets: SharedAssetService,
    provider_policy: ProviderPolicy,
    profile_seed: u64,
    cx: &mut App,
) -> Result<SimComfyPluginServices, ComponentHostError> {
    let credentials = credential_bridge(cx);
    let clock: Arc<dyn clock::SystemClock> = Arc::new(clock::RealSystemClock);
    let cost_issuer = Arc::new(
        ProviderCostAcceptanceIssuer::generate(clock.utc_now())
            .map_err(component_boundary_error)?,
    );
    let cost_authority = Arc::new(ProviderCostApprovalAuthority::new(
        cost_issuer,
        clock.clone(),
    ));
    let broker = PluginCapabilityBroker::new_with_provider_cost_acceptance(
        assets,
        ModelStore::new(ParserLimits::default()).map_err(component_boundary_error)?,
        provider_policy,
        cost_authority
            .verifier()
            .map_err(component_boundary_error)?,
        Arc::new(SimProviderActuator {
            client: cx.http_client(),
            executor: cx.background_executor().clone(),
        }),
        Arc::new(SimCredentialActuator { credentials }),
        clock,
        PluginRngPolicy::new(
            RngProfileVersion::V2,
            RngAlgorithm::Philox4x32_10,
            profile_seed,
        ),
    );
    let receipt_issuer = Arc::new(
        ProviderResultReceiptIssuer::generate(Instant::now()).map_err(component_boundary_error)?,
    );
    let receipt_lifetime = Duration::from_secs(5 * 60);
    let principal_id = launch.profile_id.0.to_string();
    let boundary = ComponentExecutionBoundary::private_worker(
        PrivateWorkerPluginExecutor::new_with_provider_authorities(
            launch,
            broker.clone(),
            principal_id.clone(),
            receipt_issuer.clone(),
            receipt_lifetime,
            cost_authority.clone(),
        )?,
    );
    Ok(SimComfyPluginServices {
        boundary,
        broker,
        principal_id,
        receipt_issuer,
        receipt_lifetime,
        cost_authority,
    })
}

struct CredentialCommand {
    secret_id: String,
    response: async_channel::Sender<Result<Option<Vec<u8>>, String>>,
}

#[derive(Clone)]
struct CredentialBridge {
    commands: async_channel::Sender<CredentialCommand>,
}

impl CredentialBridge {
    fn read(&self, secret_id: &str) -> Result<Option<Vec<u8>>, PluginServiceActuatorError> {
        let (response, result) = async_channel::bounded(1);
        self.commands
            .send_blocking(CredentialCommand {
                secret_id: secret_id.to_owned(),
                response,
            })
            .map_err(|_| PluginServiceActuatorError::new("credential bridge is unavailable"))?;
        result
            .recv_blocking()
            .map_err(|_| PluginServiceActuatorError::new("credential response is unavailable"))?
            .map_err(PluginServiceActuatorError::new)
    }
}

fn credential_bridge(cx: &mut App) -> CredentialBridge {
    let credentials = sim_credentials_provider::global(cx);
    let (commands, receiver) = async_channel::bounded::<CredentialCommand>(64);
    cx.spawn(async move |cx| {
        while let Ok(command) = receiver.recv().await {
            let result = credentials
                .read_credentials(&command.secret_id, cx)
                .await
                .map(|credential| credential.map(|(_, secret)| secret))
                .map_err(|error| error.to_string());
            if command.response.send(result).await.is_err() {
                continue;
            }
        }
    })
    .detach();
    CredentialBridge { commands }
}

struct SimCredentialActuator {
    credentials: CredentialBridge,
}

impl CredentialPresenceActuator for SimCredentialActuator {
    fn is_present(
        &self,
        request: &AuthorizedCredentialPresenceRequest,
        context: &PluginServiceOperationContext<'_>,
    ) -> Result<bool, PluginServiceActuatorError> {
        check_context(context)?;
        let present = self
            .credentials
            .read(request.secret_id().as_str())?
            .is_some();
        check_context(context)?;
        Ok(present)
    }

    fn read_for_provider(
        &self,
        request: &AuthorizedCredentialPresenceRequest,
        context: &PluginServiceOperationContext<'_>,
    ) -> Result<Option<SecretValue>, PluginServiceActuatorError> {
        check_context(context)?;
        let secret = self
            .credentials
            .read(request.secret_id().as_str())?
            .map(SecretValue::new);
        check_context(context)?;
        Ok(secret)
    }
}

struct SimProviderActuator {
    client: Arc<dyn HttpClient>,
    executor: BackgroundExecutor,
}

impl ProviderRequestActuator for SimProviderActuator {
    fn execute(
        &self,
        request: &AuthorizedProviderRequest,
        secret: Option<&SecretValue>,
        body: &[u8],
        context: &PluginServiceOperationContext<'_>,
    ) -> Result<Vec<u8>, PluginServiceActuatorError> {
        check_context(context)?;
        let mut builder = Request::builder()
            .method(http::Method::POST)
            .uri(request.endpoint())
            .header(http::header::CONTENT_TYPE, "application/octet-stream")
            .header("x-sim-comfy-provider", request.provider());
        if let Some(idempotency_key) = request.idempotency_key_sha256() {
            builder = builder.header("idempotency-key", idempotency_key);
        }
        if let Some(secret) = secret {
            let authorization = secret.expose_to(|bytes| {
                let mut value = Vec::with_capacity(bytes.len().saturating_add(7));
                value.extend_from_slice(b"Bearer ");
                value.extend_from_slice(bytes);
                http::HeaderValue::from_bytes(&value)
            });
            builder = builder.header(
                http::header::AUTHORIZATION,
                authorization.map_err(|_| {
                    PluginServiceActuatorError::new(
                        "provider credential cannot be represented as an authorization header",
                    )
                })?,
            );
        } else if request.secret_id().is_some() {
            return Err(PluginServiceActuatorError::new(
                "authorized provider credential is unavailable",
            ));
        }
        let provider_request = builder
            .body(AsyncBody::from(body.to_vec()))
            .map_err(component_actuator_error)?;
        let mut response = block_on_provider_operation(
            self.client.send(provider_request),
            context,
            &self.executor,
        )?;
        if !response.status().is_success() {
            return Err(PluginServiceActuatorError::new(format!(
                "provider returned HTTP status {}",
                response.status().as_u16()
            )));
        }
        let maximum = context.maximum_response_bytes();
        let read_limit = maximum.saturating_add(1);
        let mut bytes = Vec::new();
        block_on_provider_operation(
            response.body_mut().take(read_limit).read_to_end(&mut bytes),
            context,
            &self.executor,
        )?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum {
            return Err(PluginServiceActuatorError::new(
                "provider response exceeds the invocation bound",
            ));
        }
        check_context(context)?;
        Ok(bytes)
    }
}

fn block_on_provider_operation<T, E>(
    operation: impl Future<Output = Result<T, E>>,
    context: &PluginServiceOperationContext<'_>,
    executor: &BackgroundExecutor,
) -> Result<T, PluginServiceActuatorError>
where
    E: std::fmt::Display,
{
    smol::block_on(smol::future::race(
        async { operation.await.map_err(component_actuator_error) },
        async {
            loop {
                context.check_active().map_err(component_actuator_error)?;
                executor.timer(Duration::from_millis(5)).await;
            }
        },
    ))
}

fn check_context(
    context: &PluginServiceOperationContext<'_>,
) -> Result<(), PluginServiceActuatorError> {
    context
        .check_active()
        .map_err(|error| PluginServiceActuatorError::new(error.to_string()))
}

fn component_boundary_error(error: impl std::fmt::Display) -> ComponentHostError {
    ComponentHostError::ExecutionBoundary(error.to_string())
}

fn component_actuator_error(error: impl std::fmt::Display) -> PluginServiceActuatorError {
    PluginServiceActuatorError::new(error.to_string())
}
