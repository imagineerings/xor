use crate::{
    SamplingProfile, SchedulerDefinition, SchedulerError, SchedulerRegistry, SchedulerRequest,
    build_scheduler_schedule,
};
use comfy_tensor::{CpuBackend, CpuWorkspaceVec, ExecutionContext};

pub const KARRAS_SCHEDULER_ID: &str = "karras";
pub const KARRAS_SCHEDULER_FEATURE_ID: &str = "COMFY-MODEL-0206";
pub const KARRAS_SCHEDULER_SOURCE_ORDINAL: u16 = 2;

pub const DEFINITION: SchedulerDefinition = SchedulerDefinition {
    identity: KARRAS_SCHEDULER_ID,
    feature_id: KARRAS_SCHEDULER_FEATURE_ID,
    source_ordinal: KARRAS_SCHEDULER_SOURCE_ORDINAL,
    aliases: &[],
    implementation_module: "schedulers/karras_comfy_model_0206",
};

const RHO: f64 = 7.0;

pub fn karras_schedule<P: SamplingProfile>(
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
        KARRAS_SCHEDULER_ID,
        karras_equation,
    )
}

fn karras_equation<P: SamplingProfile>(
    effective_steps: usize,
    profile: &P,
    context: &ExecutionContext<'_>,
    sigmas: &mut CpuWorkspaceVec<f32>,
) -> Result<(), SchedulerError> {
    let minimum_inverse_rho = f64::from(profile.sigma_min()).powf(1.0 / RHO) as f32;
    let maximum_inverse_rho = f64::from(profile.sigma_max()).powf(1.0 / RHO) as f32;
    if effective_steps == 1 {
        sigmas.try_push(maximum_inverse_rho.powf(RHO as f32))?;
    } else {
        for index in 0..effective_steps {
            if index.is_multiple_of(256) {
                context
                    .cancellation
                    .check()
                    .map_err(|_| SchedulerError::Cancelled)?;
            }
            let ramp = index as f32 / (effective_steps - 1) as f32;
            let inverse_rho = (minimum_inverse_rho - maximum_inverse_rho)
                .mul_add(ramp, maximum_inverse_rho);
            sigmas.try_push(inverse_rho.powf(RHO as f32))?;
        }
    }
    sigmas.try_push(0.0)?;
    Ok(())
}
