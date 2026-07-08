use serde::{Deserialize, Serialize};

use crate::{LatentFormat, ModelFamilyExecutionProfile};

pub const LATENT_FORMAT_MISMATCH_CODE: &str = "world_model.latent.format_mismatch";
pub const LATENT_DIMENSION_MISMATCH_CODE: &str = "world_model.latent.dimension_mismatch";
pub const LATENT_TEMPORAL_MISMATCH_CODE: &str = "world_model.latent.temporal_mismatch";
pub const LATENT_MASK_MISMATCH_CODE: &str = "world_model.latent.mask_mismatch";
pub const LATENT_COMPRESSION_MISMATCH_CODE: &str = "world_model.latent.compression_mismatch";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum LatentMediaKind {
    Image,
    Video,
    Audio,
    Geometry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum LatentCompressionKind {
    None,
    VaeScale,
    TemporalVaeScale,
    AudioCodec,
    GeometryCodec,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LatentCompressionMetadata {
    pub kind: LatentCompressionKind,
    pub scale_factor: f32,
    pub channels_last: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LatentMask {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub frames: Option<u32>,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LatentArtifact {
    pub id: String,
    pub format: LatentFormat,
    pub media: LatentMediaKind,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub frames: Option<u32>,
    pub batch: u32,
    pub compression: LatentCompressionMetadata,
    pub mask: Option<LatentMask>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LatentValidationDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyLatentRuntime;

impl ComfyLatentRuntime {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(
        &self,
        latent: &LatentArtifact,
        family_profile: &ModelFamilyExecutionProfile,
    ) -> Result<(), Vec<LatentValidationDiagnostic>> {
        let mut diagnostics = Vec::new();

        if latent.format != family_profile.latent_format {
            diagnostics.push(diagnostic(
                LATENT_FORMAT_MISMATCH_CODE,
                format!(
                    "latent format {:?} does not match model family format {:?}",
                    latent.format, family_profile.latent_format
                ),
            ));
        }
        if latent.id.trim().is_empty()
            || latent.width == 0
            || latent.height == 0
            || latent.channels == 0
            || latent.batch == 0
        {
            diagnostics.push(diagnostic(
                LATENT_DIMENSION_MISMATCH_CODE,
                "latent id, width, height, channels, and batch must be present",
            ));
        }
        if family_profile.temporal && latent.frames.unwrap_or(0) == 0 {
            diagnostics.push(diagnostic(
                LATENT_TEMPORAL_MISMATCH_CODE,
                "temporal model families require latent frame metadata",
            ));
        }
        if !family_profile.temporal && latent.frames.unwrap_or(1) > 1 {
            diagnostics.push(diagnostic(
                LATENT_TEMPORAL_MISMATCH_CODE,
                "non-temporal model families cannot accept multi-frame latents",
            ));
        }
        if latent.compression.scale_factor <= 0.0 {
            diagnostics.push(diagnostic(
                LATENT_COMPRESSION_MISMATCH_CODE,
                "latent compression scale factor must be greater than zero",
            ));
        }
        if let Some(mask) = &latent.mask {
            validate_mask(mask, latent, &mut diagnostics);
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }
}

fn validate_mask(
    mask: &LatentMask,
    latent: &LatentArtifact,
    diagnostics: &mut Vec<LatentValidationDiagnostic>,
) {
    if mask.id.trim().is_empty()
        || mask.width != latent.width
        || mask.height != latent.height
        || mask.frames.unwrap_or(1) != latent.frames.unwrap_or(1)
        || !(0.0..=1.0).contains(&mask.strength)
    {
        diagnostics.push(diagnostic(
            LATENT_MASK_MISMATCH_CODE,
            "latent mask must match latent width, height, frame count, and strength range",
        ));
    }
}

fn diagnostic(code: &str, message: impl Into<String>) -> LatentValidationDiagnostic {
    LatentValidationDiagnostic {
        code: code.to_string(),
        message: message.into(),
    }
}
