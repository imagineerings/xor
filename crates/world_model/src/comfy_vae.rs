use serde::{Deserialize, Serialize};

use crate::{ComfyLatentRuntime, LatentArtifact, LatentMask, ModelFamilyExecutionProfile};

pub const VAE_UNSUPPORTED_CODE: &str = "world_model.vae.unsupported";
pub const VAE_MODEL_MISSING_CODE: &str = "world_model.vae.model_missing";
pub const VAE_IMAGE_MISSING_CODE: &str = "world_model.vae.image_missing";
pub const VAE_TILE_MISMATCH_CODE: &str = "world_model.vae.tile_mismatch";
pub const VAE_TEMPORAL_MISMATCH_CODE: &str = "world_model.vae.temporal_mismatch";
pub const VAE_INPAINT_MISMATCH_CODE: &str = "world_model.vae.inpaint_mismatch";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum VaeOperationKind {
    Encode,
    Decode,
    TiledEncode,
    TiledDecode,
    TemporalEncode,
    TemporalDecode,
    InpaintEncode,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VaeTilingMetadata {
    pub tile_width: u32,
    pub tile_height: u32,
    pub overlap: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VaeRuntimeRequest {
    pub operation: VaeOperationKind,
    pub node_id: String,
    pub vae_model_ref: String,
    pub image_ref: Option<String>,
    pub input_latent: Option<LatentArtifact>,
    pub output_latent: LatentArtifact,
    pub mask: Option<LatentMask>,
    pub tiling: Option<VaeTilingMetadata>,
    pub temporal_frames: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VaeValidationDiagnostic {
    pub code: String,
    pub node_id: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyVaeRuntime {
    latents: ComfyLatentRuntime,
}

impl ComfyVaeRuntime {
    pub fn new() -> Self {
        Self {
            latents: ComfyLatentRuntime::new(),
        }
    }

    pub fn validate(
        &self,
        request: &VaeRuntimeRequest,
        family_profile: &ModelFamilyExecutionProfile,
    ) -> Result<(), Vec<VaeValidationDiagnostic>> {
        let mut diagnostics = Vec::new();

        if !family_profile.supports_vae {
            diagnostics.push(diagnostic(
                VAE_UNSUPPORTED_CODE,
                &request.node_id,
                format!(
                    "model family {:?} does not support VAE operations",
                    family_profile.family
                ),
            ));
        }
        if request.vae_model_ref.trim().is_empty() {
            diagnostics.push(diagnostic(
                VAE_MODEL_MISSING_CODE,
                &request.node_id,
                "VAE model reference is required",
            ));
        }
        if matches!(
            request.operation,
            VaeOperationKind::Encode
                | VaeOperationKind::TiledEncode
                | VaeOperationKind::TemporalEncode
                | VaeOperationKind::InpaintEncode
        ) && request.image_ref.as_deref().unwrap_or("").trim().is_empty()
        {
            diagnostics.push(diagnostic(
                VAE_IMAGE_MISSING_CODE,
                &request.node_id,
                "VAE encode operations require an image reference",
            ));
        }
        if matches!(
            request.operation,
            VaeOperationKind::Decode
                | VaeOperationKind::TiledDecode
                | VaeOperationKind::TemporalDecode
        ) && request.input_latent.is_none()
        {
            diagnostics.push(diagnostic(
                VAE_IMAGE_MISSING_CODE,
                &request.node_id,
                "VAE decode operations require an input latent",
            ));
        }

        validate_tiling(request, &mut diagnostics);
        validate_temporal(request, family_profile, &mut diagnostics);
        validate_inpaint(request, &mut diagnostics);

        if let Err(latent_diagnostics) = self
            .latents
            .validate(&request.output_latent, family_profile)
        {
            diagnostics.extend(latent_diagnostics.into_iter().map(|latent_diagnostic| {
                diagnostic(
                    &latent_diagnostic.code,
                    &request.node_id,
                    latent_diagnostic.message,
                )
            }));
        }
        if let Some(input_latent) = &request.input_latent
            && let Err(latent_diagnostics) = self.latents.validate(input_latent, family_profile)
        {
            diagnostics.extend(latent_diagnostics.into_iter().map(|latent_diagnostic| {
                diagnostic(
                    &latent_diagnostic.code,
                    &request.node_id,
                    latent_diagnostic.message,
                )
            }));
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }
}

fn validate_tiling(request: &VaeRuntimeRequest, diagnostics: &mut Vec<VaeValidationDiagnostic>) {
    if matches!(
        request.operation,
        VaeOperationKind::TiledEncode | VaeOperationKind::TiledDecode
    ) {
        match &request.tiling {
            Some(tiling)
                if tiling.tile_width > 0
                    && tiling.tile_height > 0
                    && tiling.overlap < tiling.tile_width
                    && tiling.overlap < tiling.tile_height => {}
            _ => diagnostics.push(diagnostic(
                VAE_TILE_MISMATCH_CODE,
                &request.node_id,
                "tiled VAE operations require tile dimensions and overlap smaller than the tile",
            )),
        }
    } else if request.tiling.is_some() {
        diagnostics.push(diagnostic(
            VAE_TILE_MISMATCH_CODE,
            &request.node_id,
            "non-tiled VAE operations cannot carry tiled metadata",
        ));
    }
}

fn validate_temporal(
    request: &VaeRuntimeRequest,
    family_profile: &ModelFamilyExecutionProfile,
    diagnostics: &mut Vec<VaeValidationDiagnostic>,
) {
    if matches!(
        request.operation,
        VaeOperationKind::TemporalEncode | VaeOperationKind::TemporalDecode
    ) {
        if !family_profile.temporal || request.temporal_frames.unwrap_or(0) == 0 {
            diagnostics.push(diagnostic(
                VAE_TEMPORAL_MISMATCH_CODE,
                &request.node_id,
                "temporal VAE operations require a temporal model family and frame metadata",
            ));
        }
    } else if request.temporal_frames.is_some() {
        diagnostics.push(diagnostic(
            VAE_TEMPORAL_MISMATCH_CODE,
            &request.node_id,
            "non-temporal VAE operations cannot carry temporal frame metadata",
        ));
    }
}

fn validate_inpaint(request: &VaeRuntimeRequest, diagnostics: &mut Vec<VaeValidationDiagnostic>) {
    if matches!(request.operation, VaeOperationKind::InpaintEncode) {
        if request.mask.is_none() {
            diagnostics.push(diagnostic(
                VAE_INPAINT_MISMATCH_CODE,
                &request.node_id,
                "inpaint VAE encode requires mask metadata",
            ));
        }
    } else if request.mask.is_some() {
        diagnostics.push(diagnostic(
            VAE_INPAINT_MISMATCH_CODE,
            &request.node_id,
            "non-inpaint VAE operations cannot carry inpaint mask metadata",
        ));
    }
}

fn diagnostic(code: &str, node_id: &str, message: impl Into<String>) -> VaeValidationDiagnostic {
    VaeValidationDiagnostic {
        code: if code == crate::comfy_latents::LATENT_FORMAT_MISMATCH_CODE {
            crate::comfy_latents::LATENT_FORMAT_MISMATCH_CODE.to_string()
        } else {
            code.to_string()
        },
        node_id: node_id.to_string(),
        message: message.into(),
    }
}
