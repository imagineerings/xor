use serde::{Deserialize, Serialize};

use crate::{ComfyRunnerProfile, ModelFamilyKind, RunnerKind};

pub const WORLD_MODEL_PROFILE_UNSUPPORTED_CODE: &str = "world_model.profile.unsupported";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldModelRunnerProfile {
    pub runner_profile: ComfyRunnerProfile,
    pub supports_reference_frames: bool,
    pub supports_camera_controls: bool,
    pub supports_action_controls: bool,
    pub minimum_frames: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldModelProfileDiagnostic {
    pub code: String,
    pub family: ModelFamilyKind,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyWorldModelProfileBuilder;

impl ComfyWorldModelProfileBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn build(
        &self,
        runner_profile: ComfyRunnerProfile,
    ) -> Result<WorldModelRunnerProfile, WorldModelProfileDiagnostic> {
        if runner_profile.runner != RunnerKind::VideoWorldModel {
            return Err(WorldModelProfileDiagnostic {
                code: WORLD_MODEL_PROFILE_UNSUPPORTED_CODE.to_string(),
                family: runner_profile.family,
                message: "world-model profile requires a video/world-model runner".to_string(),
            });
        }

        Ok(WorldModelRunnerProfile {
            runner_profile,
            supports_reference_frames: true,
            supports_camera_controls: true,
            supports_action_controls: true,
            minimum_frames: 2,
        })
    }
}
