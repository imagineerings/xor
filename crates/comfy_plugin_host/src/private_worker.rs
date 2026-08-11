use crate::{
    ComponentHostError, InvocationResult, PluginError, PluginInvocationExecutor,
    PreparedPluginInvocation, ProviderInvocationResult,
};
use comfy_plugin_sdk::InvocationError;
use comfy_runtime::{
    PluginAuthorizationVerifier, PluginCapabilityBroker, PluginCapabilityInvocation,
    PluginServiceInvocationContext, RetainedPluginExecution, RuntimeSupervisor,
    RuntimeSupervisorError, WorkerLaunchConfig, WorkerRegistryDeploymentPlan,
};
use comfy_types::{
    AttemptId, ProfileId, PromptId, WorkerPluginExecutionFailure, WorkerPluginExecutionOutcome,
    WorkerRegistryGeneration,
};
use std::{future::Future, pin::Pin, sync::Arc, time::Instant};

pub struct PrivateWorkerPluginExecutor {
    launch: WorkerLaunchConfig,
    broker: PluginCapabilityBroker,
    commands: async_channel::Sender<PrivateWorkerCommand>,
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
        let (commands, receiver) = async_channel::bounded(64);
        let actor_launch = launch.clone();
        std::thread::Builder::new()
            .name("comfy-plugin-worker".to_owned())
            .spawn(move || run_private_worker_actor(actor_launch, receiver))
            .map_err(worker_boundary_error)?;
        Ok(Arc::new(Self {
            launch,
            broker,
            commands,
        }))
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
        let service_context = PluginServiceInvocationContext::new(
            self.launch.profile_id,
            context.prompt_id,
            context.attempt_id,
            context.node_id.clone(),
            invocation.authorization().clone(),
            context.cancellation.clone(),
            deadline,
            invocation.worker_invocation().maximum_response_bytes(),
        )
        .map_err(worker_boundary_error)?;
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
            let retained = self.execute_prepared(invocation).await?;
            match decode_worker_provider_result(retained.outcome()) {
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
}
