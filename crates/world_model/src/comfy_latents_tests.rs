use crate::{
    ComfyExecutionRegistry, ComfyLatentRuntime, ComfyVaeRuntime, LatentArtifact,
    LatentCompressionKind, LatentCompressionMetadata, LatentFormat, LatentMask, LatentMediaKind,
    ModelFamilyKind, VaeOperationKind, VaeRuntimeRequest, VaeTilingMetadata,
};

#[test]
fn latent_runtime_accepts_matching_image_latent_with_mask_and_compression() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let latent = image_latent();

    ComfyLatentRuntime::new()
        .validate(&latent, family)
        .expect("latent is valid");
}

#[test]
fn latent_runtime_rejects_format_dimensions_temporal_mask_and_compression_mismatches() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let mut latent = image_latent();
    latent.id.clear();
    latent.format = LatentFormat::Flux;
    latent.frames = Some(2);
    latent.compression.scale_factor = 0.0;
    latent.mask = Some(LatentMask {
        id: "mask".to_string(),
        width: 256,
        height: 1024,
        frames: None,
        strength: 1.5,
    });

    let diagnostics = ComfyLatentRuntime::new()
        .validate(&latent, family)
        .expect_err("latent mismatches rejected");

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_latents::LATENT_FORMAT_MISMATCH_CODE
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_latents::LATENT_DIMENSION_MISMATCH_CODE
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_latents::LATENT_TEMPORAL_MISMATCH_CODE
    }));
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::comfy_latents::LATENT_MASK_MISMATCH_CODE
        })
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_latents::LATENT_COMPRESSION_MISMATCH_CODE
    }));
}

#[test]
fn latent_runtime_requires_frames_for_temporal_families() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::WanVideo)
        .expect("wan family");
    let mut latent = video_latent();
    latent.frames = None;

    let diagnostics = ComfyLatentRuntime::new()
        .validate(&latent, family)
        .expect_err("missing frames rejected");

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_latents::LATENT_TEMPORAL_MISMATCH_CODE
    }));
}

#[test]
fn vae_runtime_accepts_inpaint_encode_with_mask_metadata() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let request = VaeRuntimeRequest {
        operation: VaeOperationKind::InpaintEncode,
        node_id: "vae-inpaint".to_string(),
        vae_model_ref: "vae/sdxl.vae.safetensors".to_string(),
        image_ref: Some("image://source".to_string()),
        input_latent: None,
        output_latent: image_latent(),
        mask: image_latent().mask,
        tiling: None,
        temporal_frames: None,
    };

    ComfyVaeRuntime::new()
        .validate(&request, family)
        .expect("inpaint VAE request is valid");
}

#[test]
fn vae_runtime_accepts_tiled_decode_with_input_latent() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let request = VaeRuntimeRequest {
        operation: VaeOperationKind::TiledDecode,
        node_id: "vae-tiled-decode".to_string(),
        vae_model_ref: "vae/sdxl.vae.safetensors".to_string(),
        image_ref: None,
        input_latent: Some(image_latent()),
        output_latent: image_latent(),
        mask: None,
        tiling: Some(VaeTilingMetadata {
            tile_width: 512,
            tile_height: 512,
            overlap: 64,
        }),
        temporal_frames: None,
    };

    ComfyVaeRuntime::new()
        .validate(&request, family)
        .expect("tiled decode is valid");
}

#[test]
fn vae_runtime_rejects_unsupported_missing_and_mismatched_metadata() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::Segmentation)
        .expect("segmentation family");
    let request = VaeRuntimeRequest {
        operation: VaeOperationKind::Encode,
        node_id: "vae-invalid".to_string(),
        vae_model_ref: String::new(),
        image_ref: None,
        input_latent: None,
        output_latent: image_latent(),
        mask: Some(mask()),
        tiling: Some(VaeTilingMetadata {
            tile_width: 0,
            tile_height: 512,
            overlap: 512,
        }),
        temporal_frames: Some(8),
    };

    let diagnostics = ComfyVaeRuntime::new()
        .validate(&request, family)
        .expect_err("VAE request mismatches rejected");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_vae::VAE_UNSUPPORTED_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_vae::VAE_MODEL_MISSING_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_vae::VAE_IMAGE_MISSING_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_vae::VAE_TILE_MISMATCH_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_vae::VAE_TEMPORAL_MISMATCH_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_vae::VAE_INPAINT_MISMATCH_CODE)
    );
}

#[test]
fn vae_runtime_accepts_temporal_decode_for_video_family() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::WanVideo)
        .expect("wan family");
    let request = VaeRuntimeRequest {
        operation: VaeOperationKind::TemporalDecode,
        node_id: "vae-temporal-decode".to_string(),
        vae_model_ref: "vae/wan.vae.safetensors".to_string(),
        image_ref: None,
        input_latent: Some(video_latent()),
        output_latent: video_latent(),
        mask: None,
        tiling: None,
        temporal_frames: Some(16),
    };

    ComfyVaeRuntime::new()
        .validate(&request, family)
        .expect("temporal decode is valid");
}

fn image_latent() -> LatentArtifact {
    LatentArtifact {
        id: "latent-image".to_string(),
        format: LatentFormat::StableDiffusionXl,
        media: LatentMediaKind::Image,
        width: 1024,
        height: 1024,
        channels: 4,
        frames: None,
        batch: 1,
        compression: LatentCompressionMetadata {
            kind: LatentCompressionKind::VaeScale,
            scale_factor: 0.13025,
            channels_last: false,
        },
        mask: Some(mask()),
    }
}

fn video_latent() -> LatentArtifact {
    LatentArtifact {
        id: "latent-video".to_string(),
        format: LatentFormat::Video,
        media: LatentMediaKind::Video,
        width: 832,
        height: 480,
        channels: 16,
        frames: Some(16),
        batch: 1,
        compression: LatentCompressionMetadata {
            kind: LatentCompressionKind::TemporalVaeScale,
            scale_factor: 0.18215,
            channels_last: false,
        },
        mask: None,
    }
}

fn mask() -> LatentMask {
    LatentMask {
        id: "mask".to_string(),
        width: 1024,
        height: 1024,
        frames: None,
        strength: 0.8,
    }
}
