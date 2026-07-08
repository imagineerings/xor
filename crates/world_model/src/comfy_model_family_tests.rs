use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{
    AdapterKind, ComfyModelCatalog, ComfyModelFamilyDetector, ComfyModelFolderRegistry,
    ConditioningMode, LatentFormat, ModelCategory, ModelFamilyDiagnostic, ModelFamilyKind,
    ModelMediaCapability, SafetensorsHeaderMetadata, TextEncoderRequirement, VaeRequirement,
};

#[test]
fn detector_recognizes_sdxl_from_safetensors_metadata() {
    let file = model_file(ModelCategory::Checkpoints, "sdxl/base.safetensors");
    let metadata = metadata([("modelspec.architecture", "stable-diffusion-xl-v1-base")]);

    let profile = ComfyModelFamilyDetector::new()
        .detect(&file, Some(&metadata))
        .expect("sdxl detected");

    assert_eq!(profile.family, ModelFamilyKind::StableDiffusionXl);
    assert_eq!(
        profile.capability.latent_format,
        LatentFormat::StableDiffusionXl
    );
    assert_eq!(profile.capability.vae, VaeRequirement::Required);
    assert!(profile.supports_media(ModelMediaCapability::Image));
    assert!(
        profile
            .capability
            .text_encoders
            .contains(&TextEncoderRequirement::OpenClip)
    );
}

#[test]
fn detector_recognizes_flux_and_stable_diffusion_3_profiles() {
    let detector = ComfyModelFamilyDetector::new();
    let flux = detector
        .detect(
            &model_file(ModelCategory::DiffusionModels, "flux-dev.safetensors"),
            Some(&metadata([("architecture", "Flux diffusion transformer")])),
        )
        .expect("flux detected");
    let sd3 = detector
        .detect(
            &model_file(ModelCategory::Checkpoints, "sd3-medium.safetensors"),
            Some(&metadata([(
                "modelspec.architecture",
                "stable-diffusion-3",
            )])),
        )
        .expect("sd3 detected");

    assert_eq!(flux.family, ModelFamilyKind::Flux);
    assert_eq!(flux.capability.latent_format, LatentFormat::Flux);
    assert!(
        flux.capability
            .text_encoders
            .contains(&TextEncoderRequirement::T5)
    );
    assert_eq!(sd3.family, ModelFamilyKind::StableDiffusion3);
    assert!(
        sd3.capability
            .text_encoders
            .contains(&TextEncoderRequirement::TripleClip)
    );
}

#[test]
fn detector_recognizes_video_families_from_metadata() {
    let detector = ComfyModelFamilyDetector::new();
    let families = [
        ("wan2.1", ModelFamilyKind::WanVideo),
        ("hunyuan video", ModelFamilyKind::HunyuanVideo),
        ("ltx-video", ModelFamilyKind::LtxVideo),
    ];

    for (architecture, family) in families {
        let profile = detector
            .detect(
                &model_file(ModelCategory::DiffusionModels, "video.safetensors"),
                Some(&metadata([("architecture", architecture)])),
            )
            .expect("video family detected");

        assert_eq!(profile.family, family);
        assert!(profile.supports_media(ModelMediaCapability::Video));
        assert!(
            profile
                .capability
                .conditioning
                .contains(&ConditioningMode::Video)
        );
    }
}

#[test]
fn detector_creates_native_records_for_audio_3d_segmentation_depth_and_detection() {
    let detector = ComfyModelFamilyDetector::new();
    let cases = [
        (
            ModelCategory::AudioEncoders,
            ModelFamilyKind::Audio,
            ModelMediaCapability::Audio,
        ),
        (
            ModelCategory::BackgroundRemoval,
            ModelFamilyKind::Segmentation,
            ModelMediaCapability::Segmentation,
        ),
        (
            ModelCategory::GeometryEstimation,
            ModelFamilyKind::Depth,
            ModelMediaCapability::ThreeD,
        ),
        (
            ModelCategory::GeometryEstimation,
            ModelFamilyKind::Depth,
            ModelMediaCapability::Depth,
        ),
        (
            ModelCategory::Detection,
            ModelFamilyKind::Detection,
            ModelMediaCapability::Detection,
        ),
    ];

    for (category, family, capability) in cases {
        let profile = detector
            .detect(&model_file(category, "model.safetensors"), None)
            .expect("category family detected");

        assert_eq!(profile.family, family);
        assert!(
            profile.supports_media(capability),
            "{family:?} should support {capability:?}"
        );
    }
}

#[test]
fn detector_creates_adapter_profiles_for_comfy_adapter_categories() {
    let detector = ComfyModelFamilyDetector::new();
    let cases = [
        (ModelCategory::Loras, AdapterKind::Lora),
        (ModelCategory::ControlNet, AdapterKind::ControlNet),
        (ModelCategory::StyleModels, AdapterKind::StyleModel),
        (ModelCategory::Gligen, AdapterKind::Gligen),
        (ModelCategory::Hypernetworks, AdapterKind::Hypernetwork),
        (ModelCategory::ModelPatches, AdapterKind::ModelPatch),
        (ModelCategory::Embeddings, AdapterKind::Embedding),
        (ModelCategory::ClipVision, AdapterKind::ClipVision),
    ];

    for (category, adapter_kind) in cases {
        let profile = detector
            .detect(&model_file(category, "adapter.safetensors"), None)
            .expect("adapter detected");

        assert_eq!(profile.family, ModelFamilyKind::Adapter);
        assert_eq!(profile.adapter_kind, Some(adapter_kind));
        assert!(profile.supports_media(ModelMediaCapability::Adapter));
        assert!(!profile.compatible_base_families.is_empty());
    }
}

#[test]
fn detector_validates_adapter_compatibility() {
    let detector = ComfyModelFamilyDetector::new();
    let sdxl = detector
        .detect(
            &model_file(ModelCategory::Checkpoints, "sdxl.safetensors"),
            Some(&metadata([("architecture", "sdxl")])),
        )
        .expect("sdxl detected");
    let lora = detector
        .detect(&model_file(ModelCategory::Loras, "style.safetensors"), None)
        .expect("lora detected");

    detector
        .validate_adapter_compatibility(&sdxl, &lora)
        .expect("lora compatible with sdxl");
}

#[test]
fn detector_rejects_incompatible_adapter_base_family() {
    let detector = ComfyModelFamilyDetector::new();
    let flux = detector
        .detect(
            &model_file(ModelCategory::DiffusionModels, "flux.safetensors"),
            Some(&metadata([("architecture", "flux")])),
        )
        .expect("flux detected");
    let controlnet = detector
        .detect(
            &model_file(ModelCategory::ControlNet, "control.safetensors"),
            None,
        )
        .expect("controlnet detected");

    let diagnostic = detector
        .validate_adapter_compatibility(&flux, &controlnet)
        .expect_err("controlnet rejected for flux");

    assert_eq!(
        diagnostic.code,
        crate::comfy_model_family::INCOMPATIBLE_ADAPTER_CODE
    );
    assert_eq!(
        diagnostic.missing_capability,
        Some(ModelMediaCapability::Adapter)
    );
}

#[test]
fn detector_reports_unsupported_model_family_diagnostic() {
    let diagnostic = ComfyModelFamilyDetector::new()
        .detect(
            &model_file(ModelCategory::Checkpoints, "unknown.safetensors"),
            Some(&metadata([("architecture", "unrecognized")])),
        )
        .expect_err("unknown family rejected");

    assert_eq!(
        diagnostic,
        ModelFamilyDiagnostic {
            code: crate::comfy_model_family::UNSUPPORTED_MODEL_FAMILY_CODE.to_string(),
            category: ModelCategory::Checkpoints,
            relative_path: PathBuf::from("unknown.safetensors"),
            missing_capability: Some(ModelMediaCapability::Image),
            message: "model family is not supported by Sim's native world-model runtime"
                .to_string(),
        }
    );
}

fn model_file(category: ModelCategory, relative_path: &str) -> crate::ModelFileRef {
    let registry = ComfyModelFolderRegistry::new("/project/assets");
    let catalog = ComfyModelCatalog::new(&registry);
    catalog
        .resolve_at_root(category, 0, Path::new(relative_path))
        .expect("model resolves")
}

fn metadata(
    entries: impl IntoIterator<Item = (&'static str, &'static str)>,
) -> SafetensorsHeaderMetadata {
    SafetensorsHeaderMetadata {
        header_byte_len: 128,
        tensor_count: 1,
        metadata: entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>(),
    }
}
