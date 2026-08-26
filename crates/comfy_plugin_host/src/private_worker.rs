use crate::{
    ComponentHostError, InvocationResult, PluginError, PluginInvocationExecutor,
    PreparedPluginInvocation, ProviderInvocationResult,
};
use comfy_plugin_sdk::InvocationError;
use comfy_plugin_sdk::ProviderResultReceiptSet;
use comfy_runtime::{
    NativeProviderWorkerBridgeAttachment, PluginAuthorizationVerifier, PluginCapabilityBroker,
    PluginCapabilityInvocation, PluginServiceInvocationContext, ProviderCostAuthorizationAuthority,
    ProviderResultReceiptAuthority, ProviderResultReceiptIssuer, RetainedPluginExecution,
    RuntimeSupervisor, RuntimeSupervisorError, WorkerLaunchConfig, WorkerRegistryDeploymentPlan,
};
use comfy_types::{
    AttemptId, ProfileId, PromptId, WorkerPluginExecutionFailure, WorkerPluginExecutionOutcome,
    WorkerRegistryGeneration,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct PrivateWorkerPluginExecutor {
    launch: WorkerLaunchConfig,
    broker: PluginCapabilityBroker,
    provider_result_receipts: Option<PrivateWorkerProviderResultReceipts>,
    provider_worker_bridge: Mutex<Option<NativeProviderWorkerBridgeAttachment>>,
    commands: async_channel::Sender<PrivateWorkerCommand>,
}

#[derive(Clone)]
struct PrivateWorkerProviderResultReceipts {
    principal_id: Arc<str>,
    issuer: Arc<ProviderResultReceiptIssuer>,
    lifetime: Duration,
    cost_authority: Option<Arc<dyn ProviderCostAuthorizationAuthority>>,
}

struct PrivateWorkerCommand {
    deployment: WorkerRegistryDeploymentPlan,
    invocation: Vec<u8>,
    prompt_id: PromptId,
    attempt_id: AttemptId,
    capability_invocation: PluginCapabilityInvocation,
    response: async_channel::Sender<Result<RetainedPluginExecution, RuntimeSupervisorError>>,
}

struct PrivateWorkerState {
    supervisor: Option<RuntimeSupervisor>,
    deployed_registry_identity: Option<(WorkerRegistryGeneration, String)>,
    authorization_verifier: Option<PluginAuthorizationVerifier>,
}

impl PrivateWorkerPluginExecutor {
    pub fn new(
        launch: WorkerLaunchConfig,
        broker: PluginCapabilityBroker,
    ) -> Result<Arc<Self>, ComponentHostError> {
        Self::new_internal(launch, broker, None)
    }

    pub fn new_with_provider_result_receipts(
        launch: WorkerLaunchConfig,
        broker: PluginCapabilityBroker,
        principal_id: impl Into<Arc<str>>,
        issuer: Arc<ProviderResultReceiptIssuer>,
        lifetime: Duration,
    ) -> Result<Arc<Self>, ComponentHostError> {
        let principal_id = principal_id.into();
        if principal_id.is_empty() || lifetime.is_zero() {
            return Err(ComponentHostError::ExecutionBoundary(
                "provider result receipt authority is invalid".to_owned(),
            ));
        }
        Self::new_internal(
            launch,
            broker,
            Some(PrivateWorkerProviderResultReceipts {
                principal_id,
                issuer,
                lifetime,
                cost_authority: None,
            }),
        )
    }

    pub fn new_with_provider_authorities(
        launch: WorkerLaunchConfig,
        broker: PluginCapabilityBroker,
        principal_id: impl Into<Arc<str>>,
        issuer: Arc<ProviderResultReceiptIssuer>,
        lifetime: Duration,
        cost_authority: Arc<dyn ProviderCostAuthorizationAuthority>,
    ) -> Result<Arc<Self>, ComponentHostError> {
        let principal_id = principal_id.into();
        if principal_id.is_empty() || lifetime.is_zero() {
            return Err(ComponentHostError::ExecutionBoundary(
                "provider authority configuration is invalid".to_owned(),
            ));
        }
        Self::new_internal(
            launch,
            broker,
            Some(PrivateWorkerProviderResultReceipts {
                principal_id,
                issuer,
                lifetime,
                cost_authority: Some(cost_authority),
            }),
        )
    }

    fn new_internal(
        launch: WorkerLaunchConfig,
        broker: PluginCapabilityBroker,
        provider_result_receipts: Option<PrivateWorkerProviderResultReceipts>,
    ) -> Result<Arc<Self>, ComponentHostError> {
        let (commands, receiver) = async_channel::bounded(64);
        let actor_launch = launch.clone();
        std::thread::Builder::new()
            .name("comfy-plugin-worker".to_owned())
            .spawn(move || run_private_worker_actor(actor_launch, receiver))
            .map_err(worker_boundary_error)?;
        Ok(Arc::new(Self {
            launch,
            broker,
            provider_result_receipts,
            provider_worker_bridge: Mutex::new(None),
            commands,
        }))
    }

    pub fn attach_provider_worker_bridge(
        &self,
        attachment: NativeProviderWorkerBridgeAttachment,
    ) -> Result<(), ComponentHostError> {
        let attachment = attachment
            .bind_to_worker_profile(self.launch.profile_id)
            .map_err(worker_boundary_error)?;
        let mut current = self.provider_worker_bridge.lock().map_err(|error| {
            ComponentHostError::ExecutionBoundary(format!(
                "private worker provider bridge slot is unavailable: {error}"
            ))
        })?;
        if current
            .as_ref()
            .is_some_and(NativeProviderWorkerBridgeAttachment::is_live)
        {
            return Err(ComponentHostError::ExecutionBoundary(
                "private worker already has a live native provider bridge".to_owned(),
            ));
        }
        *current = Some(attachment);
        Ok(())
    }

    async fn execute_prepared(
        &self,
        invocation: PreparedPluginInvocation,
    ) -> Result<RetainedPluginExecution, ComponentHostError> {
        let context = invocation.context();
        context.cancellation.check().map_err(|_| {
            ComponentHostError::Plugin(PluginError::Invocation(InvocationError::Cancelled))
        })?;
        verify_profile(
            self.launch.profile_id,
            invocation.authorization().capabilities().profile_id(),
        )?;
        let timeout =
            std::time::Duration::from_millis(invocation.worker_invocation().timeout_milliseconds());
        let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
            ComponentHostError::ExecutionBoundary(
                "plugin invocation deadline overflowed".to_owned(),
            )
        })?;
        let service_context = if invocation.worker_invocation().provider_request().is_some() {
            let receipt_configuration =
                self.provider_result_receipts.as_ref().ok_or_else(|| {
                    ComponentHostError::ExecutionBoundary(
                        "provider invocation requires app-owned result receipt authority"
                            .to_owned(),
                    )
                })?;
            let provider_execution = context
                .provider_execution()
                .map_err(worker_boundary_error)?;
            let provider_binding_sha256 =
                invocation.provider_binding_sha256().ok_or_else(|| {
                    ComponentHostError::ExecutionBoundary(
                        "provider invocation is missing its signed binding identity".to_owned(),
                    )
                })?;
            let authority = ProviderResultReceiptAuthority::new(
                receipt_configuration.principal_id.as_ref(),
                provider_execution.compiled_plan_sha256(),
                provider_binding_sha256,
                receipt_configuration.issuer.clone(),
                receipt_configuration.lifetime,
            )
            .map_err(worker_boundary_error)?;
            let service_context = PluginServiceInvocationContext::new_with_principal(
                self.launch.profile_id,
                context.prompt_id,
                context.attempt_id,
                context.node_id.clone(),
                receipt_configuration.principal_id.as_ref(),
                invocation.authorization().clone(),
                context.cancellation.clone(),
                deadline,
                invocation.worker_invocation().maximum_response_bytes(),
            )
            .and_then(|context| context.with_provider_result_authority(authority))
            .map_err(worker_boundary_error)?;
            match invocation.provider_price_badge() {
                Some(price_badge) => service_context
                    .with_provider_cost_requirement(
                        price_badge.clone(),
                        receipt_configuration.cost_authority.clone(),
                    )
                    .map_err(worker_boundary_error)?,
                None => service_context,
            }
        } else {
            PluginServiceInvocationContext::new(
                self.launch.profile_id,
                context.prompt_id,
                context.attempt_id,
                context.node_id.clone(),
                invocation.authorization().clone(),
                context.cancellation.clone(),
                deadline,
                invocation.worker_invocation().maximum_response_bytes(),
            )
            .map_err(worker_boundary_error)?
        };
        let capability_invocation = self
            .broker
            .begin_invocation(service_context)
            .map_err(worker_boundary_error)?;
        let (response, result) = async_channel::bounded(1);
        let command = PrivateWorkerCommand {
            deployment: invocation.deployment().clone(),
            invocation: invocation.worker_invocation().to_bytes()?,
            prompt_id: context.prompt_id,
            attempt_id: context.attempt_id,
            capability_invocation,
            response,
        };
        self.commands.send(command).await.map_err(|_| {
            ComponentHostError::ExecutionBoundary(
                "private plugin worker actor is unavailable".to_owned(),
            )
        })?;
        result
            .recv()
            .await
            .map_err(|_| {
                ComponentHostError::ExecutionBoundary(
                    "private plugin worker actor closed its response".to_owned(),
                )
            })?
            .map_err(worker_boundary_error)
    }
}

impl PluginInvocationExecutor for PrivateWorkerPluginExecutor {
    fn execute<'a>(
        &'a self,
        invocation: PreparedPluginInvocation,
    ) -> Pin<Box<dyn Future<Output = Result<InvocationResult, ComponentHostError>> + Send + 'a>>
    {
        Box::pin(async move {
            let retained = self.execute_prepared(invocation).await?;
            match decode_worker_result(retained.outcome()) {
                Ok(result) => {
                    retained.finish().map_err(worker_boundary_error)?;
                    Ok(result)
                }
                Err(error) => {
                    retained.abort();
                    Err(error)
                }
            }
        })
    }

    fn execute_provider<'a>(
        &'a self,
        invocation: PreparedPluginInvocation,
    ) -> Pin<
        Box<dyn Future<Output = Result<ProviderInvocationResult, ComponentHostError>> + Send + 'a>,
    > {
        Box::pin(async move {
            let mut retained = self.execute_prepared(invocation).await?;
            match decode_worker_provider_result(retained.outcome()) {
                Ok(mut result) => {
                    let receipt_set = ProviderResultReceiptSet::new(result.receipts().to_vec())
                        .map_err(worker_boundary_error)?;
                    let resolved = retained
                        .resolve_provider_result_receipt_set(&receipt_set)
                        .map_err(worker_boundary_error)?;
                    result.set_resolved_provider_results(resolved);
                    retained.finish().map_err(worker_boundary_error)?;
                    Ok(result)
                }
                Err(error) => {
                    retained.abort();
                    Err(error)
                }
            }
        })
    }
}

fn run_private_worker_actor(
    launch: WorkerLaunchConfig,
    commands: async_channel::Receiver<PrivateWorkerCommand>,
) {
    smol::block_on(async move {
        let mut state = PrivateWorkerState {
            supervisor: None,
            deployed_registry_identity: None,
            authorization_verifier: None,
        };
        while let Ok(command) = commands.recv().await {
            let response = command.response.clone();
            let result = execute_private_worker_command(&launch, &mut state, command).await;
            if result
                .as_ref()
                .is_err_and(worker_failure_requires_supervisor_reset)
            {
                state.supervisor = None;
                state.deployed_registry_identity = None;
                state.authorization_verifier = None;
            }
            if response.send(result).await.is_err() {
                state.supervisor = None;
                state.deployed_registry_identity = None;
                state.authorization_verifier = None;
            }
        }
    });
}

async fn execute_private_worker_command(
    launch: &WorkerLaunchConfig,
    state: &mut PrivateWorkerState,
    command: PrivateWorkerCommand,
) -> Result<RetainedPluginExecution, RuntimeSupervisorError> {
    let registry_identity = (
        command.deployment.begin().generation(),
        command
            .deployment
            .begin()
            .registry_digest_sha256()
            .as_str()
            .to_owned(),
    );
    if state.authorization_verifier.as_ref() != Some(command.deployment.authorization_verifier()) {
        state.supervisor = None;
        state.deployed_registry_identity = None;
        state.authorization_verifier = None;
    }
    if state.supervisor.is_none() {
        let mut launch = launch.clone();
        launch.plugin_authorization_verifier =
            Some(command.deployment.authorization_verifier().clone());
        let mut supervisor = RuntimeSupervisor::start(launch).await?;
        if let Err(error) = supervisor.deploy_registry(&command.deployment).await {
            return Err(worker_failure_with_logs(error, &supervisor));
        }
        state.supervisor = Some(supervisor);
        state.deployed_registry_identity = Some(registry_identity.clone());
        state.authorization_verifier = Some(command.deployment.authorization_verifier().clone());
    } else if state.deployed_registry_identity.as_ref() != Some(&registry_identity) {
        let supervisor = state
            .supervisor
            .as_mut()
            .ok_or(RuntimeSupervisorError::NotRunning)?;
        if let Err(error) = supervisor.deploy_registry(&command.deployment).await {
            return Err(worker_failure_with_logs(error, supervisor));
        }
        state.deployed_registry_identity = Some(registry_identity);
    }
    let supervisor = state
        .supervisor
        .as_mut()
        .ok_or(RuntimeSupervisorError::NotRunning)?;
    match supervisor
        .execute_plugin_retaining_capabilities(
            command.prompt_id,
            command.attempt_id,
            command.invocation,
            command.capability_invocation,
        )
        .await
    {
        Ok(outcome) => Ok(outcome),
        Err(error) => Err(worker_failure_with_logs(error, supervisor)),
    }
}

fn worker_failure_with_logs(
    error: RuntimeSupervisorError,
    supervisor: &RuntimeSupervisor,
) -> RuntimeSupervisorError {
    if matches!(error, RuntimeSupervisorError::InvalidRegistryDeployment(_)) {
        return error;
    }
    let logs = supervisor.logs();
    if logs.is_empty() {
        error
    } else {
        RuntimeSupervisorError::Protocol(format!(
            "private worker operation failed; worker diagnostics: {}",
            logs.join(" | ")
        ))
    }
}

fn worker_failure_requires_supervisor_reset(error: &RuntimeSupervisorError) -> bool {
    !matches!(error, RuntimeSupervisorError::InvalidRegistryDeployment(_))
}

fn verify_profile(
    profile_id: ProfileId,
    authorization_profile: &str,
) -> Result<(), ComponentHostError> {
    if authorization_profile != profile_id.0.to_string() {
        return Err(ComponentHostError::ExecutionBoundary(
            "worker and plugin authorization profiles differ".to_owned(),
        ));
    }
    Ok(())
}

fn decode_worker_result(
    outcome: &WorkerPluginExecutionOutcome,
) -> Result<InvocationResult, ComponentHostError> {
    match outcome {
        WorkerPluginExecutionOutcome::Succeeded(bytes) => {
            serde_json::from_slice(&bytes).map_err(|_| {
                ComponentHostError::ExecutionBoundary(
                    "private worker returned an invalid plugin result".to_owned(),
                )
            })
        }
        WorkerPluginExecutionOutcome::Failed(failure) => Err(match failure {
            WorkerPluginExecutionFailure::Cancelled => {
                ComponentHostError::Plugin(PluginError::Invocation(InvocationError::Cancelled))
            }
            WorkerPluginExecutionFailure::TimedOut => {
                ComponentHostError::Plugin(PluginError::Invocation(InvocationError::TimedOut))
            }
            WorkerPluginExecutionFailure::Trap { diagnostic } => {
                ComponentHostError::Plugin(PluginError::WasmTrap(diagnostic.clone()))
            }
            WorkerPluginExecutionFailure::InvalidInvocation => {
                ComponentHostError::ExecutionBoundary(
                    "private worker rejected the plugin invocation".to_owned(),
                )
            }
            WorkerPluginExecutionFailure::CapabilityDenied => {
                ComponentHostError::ExecutionBoundary(
                    "private worker plugin capability was denied".to_owned(),
                )
            }
            WorkerPluginExecutionFailure::HostFailure => ComponentHostError::ExecutionBoundary(
                "private worker plugin execution failed".to_owned(),
            ),
        }),
    }
}

fn decode_worker_provider_result(
    outcome: &WorkerPluginExecutionOutcome,
) -> Result<ProviderInvocationResult, ComponentHostError> {
    match outcome {
        WorkerPluginExecutionOutcome::Succeeded(bytes) => {
            serde_json::from_slice(&bytes).map_err(|_| {
                ComponentHostError::ExecutionBoundary(
                    "private worker returned an invalid provider result".to_owned(),
                )
            })
        }
        WorkerPluginExecutionOutcome::Failed(failure) => decode_worker_failure(failure),
    }
}

fn decode_worker_failure<T>(
    failure: &WorkerPluginExecutionFailure,
) -> Result<T, ComponentHostError> {
    Err(match failure {
        WorkerPluginExecutionFailure::Cancelled => {
            ComponentHostError::Plugin(PluginError::Invocation(InvocationError::Cancelled))
        }
        WorkerPluginExecutionFailure::TimedOut => {
            ComponentHostError::Plugin(PluginError::Invocation(InvocationError::TimedOut))
        }
        WorkerPluginExecutionFailure::Trap { diagnostic } => {
            ComponentHostError::Plugin(PluginError::WasmTrap(diagnostic.clone()))
        }
        WorkerPluginExecutionFailure::InvalidInvocation => ComponentHostError::ExecutionBoundary(
            "private worker rejected the plugin invocation".to_owned(),
        ),
        WorkerPluginExecutionFailure::CapabilityDenied => ComponentHostError::ExecutionBoundary(
            "private worker plugin capability was denied".to_owned(),
        ),
        WorkerPluginExecutionFailure::HostFailure => ComponentHostError::ExecutionBoundary(
            "private worker plugin execution failed".to_owned(),
        ),
    })
}

fn worker_boundary_error(error: impl std::fmt::Display) -> ComponentHostError {
    ComponentHostError::ExecutionBoundary(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_registry_replacement_preserves_the_live_supervisor() {
        assert!(!worker_failure_requires_supervisor_reset(
            &RuntimeSupervisorError::InvalidRegistryDeployment(
                "component compilation failed".to_owned(),
            )
        ));
        assert!(worker_failure_requires_supervisor_reset(
            &RuntimeSupervisorError::Protocol("invalid worker frame".to_owned())
        ));
        assert!(worker_failure_requires_supervisor_reset(
            &RuntimeSupervisorError::WorkerFatal {
                code: "worker_protocol_error".to_owned(),
                message: "fatal".to_owned(),
            }
        ));
    }

    #[test]
    fn private_worker_provider_bridge_attachment_is_one_live_weak_consumer() {
        let source = include_str!("private_worker.rs");
        let production = source
            .split_once("#[cfg(test)]")
            .map(|(production, _)| production)
            .expect("private worker production source");
        for required in [
            "provider_worker_bridge: Mutex<Option<NativeProviderWorkerBridgeAttachment>>",
            "bind_to_worker_profile(self.launch.profile_id)",
            "is_some_and(NativeProviderWorkerBridgeAttachment::is_live)",
            "private worker already has a live native provider bridge",
            "*current = Some(attachment)",
        ] {
            assert!(
                production.contains(required),
                "private worker bridge attachment lacks {required}"
            );
        }
        for forbidden in [
            concat!("ProviderRuntimeStream", "Service::new()"),
            concat!("ProviderRuntimeActivation", "GrantSource"),
            concat!("provider_runtime_stream", "_service"),
            concat!("provider_runtime_activation", "_grants"),
        ] {
            assert!(
                !production.contains(forbidden),
                "private worker bridge attachment exposes {forbidden}"
            );
        }
    }
}
