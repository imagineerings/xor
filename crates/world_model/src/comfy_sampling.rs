use serde::{Deserialize, Serialize};

use crate::{
    ComfyExecutionRegistry, DeviceBackend, GuidanceMode, ModelFamilyExecutionProfile,
    PrecisionPolicy, SamplerKind, SchedulerKind,
};

pub const UNSUPPORTED_SAMPLER_CODE: &str = "world_model.sampling.unsupported_sampler";
pub const UNSUPPORTED_SCHEDULER_CODE: &str = "world_model.sampling.unsupported_scheduler";
pub const UNSUPPORTED_GUIDANCE_CODE: &str = "world_model.sampling.unsupported_guidance";
pub const INVALID_STEPS_CODE: &str = "world_model.sampling.invalid_steps";
pub const INVALID_DENOISE_CODE: &str = "world_model.sampling.invalid_denoise";
pub const INVALID_LATENT_CODE: &str = "world_model.sampling.invalid_latent";
pub const DETERMINISM_UNSUPPORTED_CODE: &str = "world_model.sampling.determinism_unsupported";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum SamplingNodeKind {
    KSampler,
    AdvancedSampler,
    CustomSampler,
    SamplingHelper,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NoisePolicy {
    Random,
    Fixed { noise_seed: u64 },
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DenoiseRange {
    pub amount: f32,
    pub start_step: Option<u32>,
    pub end_step: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LatentDescriptor {
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub frames: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SamplingRunInput {
    pub node_kind: SamplingNodeKind,
    pub sampler_name: String,
    pub scheduler_name: String,
    pub guidance_name: String,
    pub seed: u64,
    pub noise_policy: NoisePolicy,
    pub steps: u32,
    pub cfg_scale: f32,
    pub denoise: DenoiseRange,
    pub latent: LatentDescriptor,
    pub positive_conditioning: String,
    pub negative_conditioning: Option<String>,
    pub model_profile: String,
    pub model_hash: Option<String>,
    pub deterministic: bool,
    pub worker_supports_determinism: bool,
    pub backend: DeviceBackend,
    pub precision: PrecisionPolicy,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SamplingRunRequest {
    pub node_kind: SamplingNodeKind,
    pub sampler: SamplerKind,
    pub scheduler: SchedulerKind,
    pub guidance: GuidanceMode,
    pub seed: u64,
    pub noise_policy: NoisePolicy,
    pub steps: u32,
    pub cfg_scale: f32,
    pub denoise: DenoiseRange,
    pub latent: LatentDescriptor,
    pub positive_conditioning: String,
    pub negative_conditioning: Option<String>,
    pub model_profile: String,
    pub family_profile: ModelFamilyExecutionProfile,
    pub deterministic: Option<DeterministicRunMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeterministicRunMetadata {
    pub seed: u64,
    pub noise_seed: Option<u64>,
    pub sampler: SamplerKind,
    pub scheduler: SchedulerKind,
    pub backend: DeviceBackend,
    pub precision: PrecisionPolicy,
    pub model_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SamplingProgress {
    pub current_step: u32,
    pub total_steps: u32,
    pub preview_available: bool,
    pub cancellation_requested: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SamplingValidationDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfySamplingRequestBuilder;

impl ComfySamplingRequestBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn build(
        &self,
        registry: &ComfyExecutionRegistry,
        family_profile: &ModelFamilyExecutionProfile,
        input: SamplingRunInput,
    ) -> Result<SamplingRunRequest, Vec<SamplingValidationDiagnostic>> {
        let mut diagnostics = Vec::new();

        let sampler = match registry.sampler(&input.sampler_name) {
            Some(sampler) => Some(sampler.kind),
            None => {
                diagnostics.push(diagnostic(
                    UNSUPPORTED_SAMPLER_CODE,
                    format!("sampler `{}` is not supported by Sim", input.sampler_name),
                ));
                None
            }
        };
        let scheduler = match registry.scheduler(&input.scheduler_name) {
            Some(scheduler) => Some(scheduler.kind),
            None => {
                diagnostics.push(diagnostic(
                    UNSUPPORTED_SCHEDULER_CODE,
                    format!(
                        "scheduler `{}` is not supported by Sim",
                        input.scheduler_name
                    ),
                ));
                None
            }
        };
        let guidance = match registry.guidance(&input.guidance_name) {
            Some(guidance) => Some(guidance.mode),
            None => {
                diagnostics.push(diagnostic(
                    UNSUPPORTED_GUIDANCE_CODE,
                    format!("guidance `{}` is not supported by Sim", input.guidance_name),
                ));
                None
            }
        };

        if let Some(sampler) = sampler
            && !family_profile.supported_samplers.contains(&sampler)
        {
            diagnostics.push(diagnostic(
                UNSUPPORTED_SAMPLER_CODE,
                format!(
                    "sampler `{}` is not supported by model family {:?}",
                    sampler.canonical_name(),
                    family_profile.family
                ),
            ));
        }

        if let Some(scheduler) = scheduler
            && !family_profile.supported_schedulers.contains(&scheduler)
        {
            diagnostics.push(diagnostic(
                UNSUPPORTED_SCHEDULER_CODE,
                format!(
                    "scheduler `{}` is not supported by model family {:?}",
                    scheduler.canonical_name(),
                    family_profile.family
                ),
            ));
        }

        if let Some(guidance) = guidance
            && !family_profile.supported_guidance.contains(&guidance)
        {
            diagnostics.push(diagnostic(
                UNSUPPORTED_GUIDANCE_CODE,
                format!(
                    "guidance `{}` is not supported by model family {:?}",
                    guidance.canonical_name(),
                    family_profile.family
                ),
            ));
        }

        if input.steps == 0 {
            diagnostics.push(diagnostic(
                INVALID_STEPS_CODE,
                "sampling steps must be greater than zero",
            ));
        }
        if !(0.0..=1.0).contains(&input.denoise.amount) {
            diagnostics.push(diagnostic(
                INVALID_DENOISE_CODE,
                "denoise amount must be between 0.0 and 1.0",
            ));
        }
        if let (Some(start_step), Some(end_step)) =
            (input.denoise.start_step, input.denoise.end_step)
            && start_step > end_step
        {
            diagnostics.push(diagnostic(
                INVALID_DENOISE_CODE,
                "denoise start step cannot be after end step",
            ));
        }
        if input.latent.width == 0 || input.latent.height == 0 || input.latent.channels == 0 {
            diagnostics.push(diagnostic(
                INVALID_LATENT_CODE,
                "latent width, height, and channels must be greater than zero",
            ));
        }
        if family_profile.temporal && input.latent.frames.unwrap_or(0) == 0 {
            diagnostics.push(diagnostic(
                INVALID_LATENT_CODE,
                "temporal model families require a latent frame count",
            ));
        }
        if input.deterministic && !input.worker_supports_determinism {
            diagnostics.push(diagnostic(
                DETERMINISM_UNSUPPORTED_CODE,
                "deterministic execution was requested but the worker does not support it",
            ));
        }

        let (Some(sampler), Some(scheduler), Some(guidance)) = (sampler, scheduler, guidance)
        else {
            return Err(diagnostics);
        };
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }

        let deterministic = input.deterministic.then(|| DeterministicRunMetadata {
            seed: input.seed,
            noise_seed: match input.noise_policy {
                NoisePolicy::Fixed { noise_seed } => Some(noise_seed),
                NoisePolicy::Random | NoisePolicy::Disabled => None,
            },
            sampler,
            scheduler,
            backend: input.backend,
            precision: input.precision,
            model_hash: input.model_hash.clone(),
        });

        Ok(SamplingRunRequest {
            node_kind: input.node_kind,
            sampler,
            scheduler,
            guidance,
            seed: input.seed,
            noise_policy: input.noise_policy,
            steps: input.steps,
            cfg_scale: input.cfg_scale,
            denoise: input.denoise,
            latent: input.latent,
            positive_conditioning: input.positive_conditioning,
            negative_conditioning: input.negative_conditioning,
            model_profile: input.model_profile,
            family_profile: family_profile.clone(),
            deterministic,
        })
    }

    pub fn progress(
        &self,
        current_step: u32,
        total_steps: u32,
        preview_available: bool,
        cancellation_requested: bool,
    ) -> SamplingProgress {
        SamplingProgress {
            current_step: current_step.min(total_steps),
            total_steps,
            preview_available,
            cancellation_requested,
        }
    }
}

fn diagnostic(code: &str, message: impl Into<String>) -> SamplingValidationDiagnostic {
    SamplingValidationDiagnostic {
        code: code.to_string(),
        message: message.into(),
    }
}
