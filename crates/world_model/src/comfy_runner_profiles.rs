use serde::{Deserialize, Serialize};

use crate::{
    ComfyExecutionRegistry, LatentFormat, ModelFamilyExecutionProfile, ModelFamilyKind,
    ModelMediaCapability,
};

pub const RUNNER_PROFILE_UNSUPPORTED_CODE: &str = "world_model.runner_profile.unsupported";
pub const RUNNER_PROFILE_MEDIA_MISMATCH_CODE: &str = "world_model.runner_profile.media_mismatch";
pub const RUNNER_PROFILE_LATENT_MISMATCH_CODE: &str = "world_model.runner_profile.latent_mismatch";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RunnerKind {
    ImageDiffusion,
    VideoWorldModel,
    AudioGeneration,
    ThreeDGeneration,
    DepthEstimation,
    Segmentation,
    Detection,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyRunnerProfile {
    pub family: ModelFamilyKind,
    pub runner: RunnerKind,
    pub media: ModelMediaCapability,
    pub latent_format: LatentFormat,
    pub execution_profile: ModelFamilyExecutionProfile,
    pub native_sim_runner: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunnerProfileDiagnostic {
    pub code: String,
    pub family: ModelFamilyKind,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyRunnerProfileRegistry {
    execution_registry: ComfyExecutionRegistry,
}

impl ComfyRunnerProfileRegistry {
    pub fn new() -> Self {
        Self {
            execution_registry: ComfyExecutionRegistry::new(),
        }
    }

    pub fn profile(
        &self,
        family: ModelFamilyKind,
    ) -> Result<ComfyRunnerProfile, RunnerProfileDiagnostic> {
        let execution_profile = self
            .execution_registry
            .model_family(family)
            .ok_or_else(|| RunnerProfileDiagnostic {
                code: RUNNER_PROFILE_UNSUPPORTED_CODE.to_string(),
                family,
                message: format!(
                    "model family {:?} has no native Sim execution profile",
                    family
                ),
            })?;
        let (runner, media, latent_format, native_sim_runner) =
            profile_shape(family).ok_or_else(|| RunnerProfileDiagnostic {
                code: RUNNER_PROFILE_UNSUPPORTED_CODE.to_string(),
                family,
                message: format!("model family {:?} has no native Sim runner profile", family),
            })?;

        if !execution_profile.media.contains(&media) {
            return Err(RunnerProfileDiagnostic {
                code: RUNNER_PROFILE_MEDIA_MISMATCH_CODE.to_string(),
                family,
                message: format!(
                    "runner media {:?} is not supported by model family {:?}",
                    media, family
                ),
            });
        }
        if execution_profile.latent_format != latent_format {
            return Err(RunnerProfileDiagnostic {
                code: RUNNER_PROFILE_LATENT_MISMATCH_CODE.to_string(),
                family,
                message: format!(
                    "runner latent format {:?} does not match model family latent format {:?}",
                    latent_format, execution_profile.latent_format
                ),
            });
        }

        Ok(ComfyRunnerProfile {
            family,
            runner,
            media,
            latent_format,
            execution_profile: execution_profile.clone(),
            native_sim_runner: native_sim_runner.to_string(),
        })
    }

    pub fn supported_profiles(&self) -> Vec<ComfyRunnerProfile> {
        self.execution_registry
            .model_families()
            .into_iter()
            .filter_map(|profile| self.profile(profile.family).ok())
            .collect()
    }
}

fn profile_shape(
    family: ModelFamilyKind,
) -> Option<(RunnerKind, ModelMediaCapability, LatentFormat, &'static str)> {
    match family {
        ModelFamilyKind::StableDiffusion1
        | ModelFamilyKind::StableDiffusion2
        | ModelFamilyKind::StableDiffusionXl => Some((
            RunnerKind::ImageDiffusion,
            ModelMediaCapability::Image,
            image_latent_format(family),
            "sim.image_diffusion",
        )),
        ModelFamilyKind::StableDiffusion3 => Some((
            RunnerKind::ImageDiffusion,
            ModelMediaCapability::Image,
            LatentFormat::StableDiffusion3,
            "sim.image_diffusion",
        )),
        ModelFamilyKind::Flux => Some((
            RunnerKind::ImageDiffusion,
            ModelMediaCapability::Image,
            LatentFormat::Flux,
            "sim.flux_diffusion",
        )),
        ModelFamilyKind::WanVideo | ModelFamilyKind::HunyuanVideo | ModelFamilyKind::LtxVideo => {
            Some((
                RunnerKind::VideoWorldModel,
                ModelMediaCapability::Video,
                LatentFormat::Video,
                "sim.video_world_model",
            ))
        }
        ModelFamilyKind::Audio => Some((
            RunnerKind::AudioGeneration,
            ModelMediaCapability::Audio,
            LatentFormat::Audio,
            "sim.audio_generation",
        )),
        ModelFamilyKind::ThreeD => Some((
            RunnerKind::ThreeDGeneration,
            ModelMediaCapability::ThreeD,
            LatentFormat::Geometry,
            "sim.three_d_generation",
        )),
        ModelFamilyKind::Depth => Some((
            RunnerKind::DepthEstimation,
            ModelMediaCapability::Depth,
            LatentFormat::Geometry,
            "sim.depth_estimation",
        )),
        ModelFamilyKind::Segmentation => Some((
            RunnerKind::Segmentation,
            ModelMediaCapability::Segmentation,
            LatentFormat::None,
            "sim.segmentation",
        )),
        ModelFamilyKind::Detection => Some((
            RunnerKind::Detection,
            ModelMediaCapability::Detection,
            LatentFormat::None,
            "sim.detection",
        )),
        ModelFamilyKind::Adapter => None,
    }
}

fn image_latent_format(family: ModelFamilyKind) -> LatentFormat {
    match family {
        ModelFamilyKind::StableDiffusion1 | ModelFamilyKind::StableDiffusion2 => {
            LatentFormat::StableDiffusion
        }
        ModelFamilyKind::StableDiffusionXl => LatentFormat::StableDiffusionXl,
        _ => LatentFormat::None,
    }
}
