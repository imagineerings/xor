use crate::{
    SamplingProfile, SchedulerDefinition, SchedulerError, SchedulerRegistry, SchedulerRequest,
    build_scheduler_schedule,
};
use comfy_tensor::{CpuBackend, CpuWorkspaceVec, ExecutionContext};

pub const EXPONENTIAL_SCHEDULER_ID: &str = "exponential";
pub const EXPONENTIAL_SCHEDULER_FEATURE_ID: &str = "COMFY-MODEL-0205";
pub const EXPONENTIAL_SCHEDULER_SOURCE_ORDINAL: u16 = 3;

pub const DEFINITION: SchedulerDefinition = SchedulerDefinition {
    identity: EXPONENTIAL_SCHEDULER_ID,
    feature_id: EXPONENTIAL_SCHEDULER_FEATURE_ID,
    source_ordinal: EXPONENTIAL_SCHEDULER_SOURCE_ORDINAL,
    aliases: &[],
    implementation_module: "schedulers/exponential_comfy_model_0205",
};

pub fn exponential_schedule<P: SamplingProfile>(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    registry: &SchedulerRegistry,
    profile: &P,
    request: &SchedulerRequest,
) -> Result<Vec<f32>, SchedulerError> {
    build_scheduler_schedule(
        backend,
        context,
        registry,
        profile,
        request,
        EXPONENTIAL_SCHEDULER_ID,
        exponential_equation,
    )
}

fn exponential_equation<P: SamplingProfile>(
    effective_steps: usize,
    profile: &P,
    context: &ExecutionContext<'_>,
    sigmas: &mut CpuWorkspaceVec<f32>,
) -> Result<(), SchedulerError> {
    let maximum_log_sigma = (f64::from(profile.sigma_max()).ln()) as f32;
    let minimum_log_sigma = (f64::from(profile.sigma_min()).ln()) as f32;
    if effective_steps == 1 {
        sigmas.try_push(maximum_log_sigma.exp())?;
    } else {
        for index in 0..effective_steps {
            if index.is_multiple_of(256) {
                context
                    .cancellation
                    .check()
                    .map_err(|_| SchedulerError::Cancelled)?;
            }
            let fraction = index as f32 / (effective_steps - 1) as f32;
            let log_sigma =
                (minimum_log_sigma - maximum_log_sigma).mul_add(fraction, maximum_log_sigma);
            sigmas.try_push(log_sigma.exp())?;
        }
    }
    sigmas.try_push(0.0)?;
    Ok(())
}
