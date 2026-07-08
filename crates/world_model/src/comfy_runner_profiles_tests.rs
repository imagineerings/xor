use crate::{
    ComfyRunnerProfileRegistry, ComfyWorldModelProfileBuilder, LatentFormat, ModelFamilyKind,
    ModelMediaCapability, RunnerKind,
};

#[test]
fn registry_exposes_image_diffusion_runner_profiles() {
    let registry = ComfyRunnerProfileRegistry::new();
    let sdxl = registry
        .profile(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl profile");
    let flux = registry
        .profile(ModelFamilyKind::Flux)
        .expect("flux profile");

    assert_eq!(sdxl.runner, RunnerKind::ImageDiffusion);
    assert_eq!(sdxl.media, ModelMediaCapability::Image);
    assert_eq!(sdxl.latent_format, LatentFormat::StableDiffusionXl);
    assert_eq!(sdxl.native_sim_runner, "sim.image_diffusion");
    assert_eq!(flux.latent_format, LatentFormat::Flux);
    assert_eq!(flux.native_sim_runner, "sim.flux_diffusion");
}

#[test]
fn registry_exposes_video_world_model_profile_constraints() {
    let registry = ComfyRunnerProfileRegistry::new();
    let wan = registry
        .profile(ModelFamilyKind::WanVideo)
        .expect("wan profile");
    let world_model = ComfyWorldModelProfileBuilder::new()
        .build(wan)
        .expect("world-model profile");

    assert_eq!(
        world_model.runner_profile.runner,
        RunnerKind::VideoWorldModel
    );
    assert_eq!(
        world_model.runner_profile.latent_format,
        LatentFormat::Video
    );
    assert!(world_model.supports_reference_frames);
    assert!(world_model.supports_camera_controls);
    assert!(world_model.supports_action_controls);
    assert_eq!(world_model.minimum_frames, 2);
}

#[test]
fn registry_exposes_specialized_media_runner_profiles() {
    let registry = ComfyRunnerProfileRegistry::new();

    assert_eq!(
        registry
            .profile(ModelFamilyKind::Audio)
            .expect("audio profile")
            .runner,
        RunnerKind::AudioGeneration
    );
    assert_eq!(
        registry
            .profile(ModelFamilyKind::ThreeD)
            .expect("3d profile")
            .runner,
        RunnerKind::ThreeDGeneration
    );
    assert_eq!(
        registry
            .profile(ModelFamilyKind::Depth)
            .expect("depth profile")
            .runner,
        RunnerKind::DepthEstimation
    );
    assert_eq!(
        registry
            .profile(ModelFamilyKind::Segmentation)
            .expect("segmentation profile")
            .runner,
        RunnerKind::Segmentation
    );
    assert_eq!(
        registry
            .profile(ModelFamilyKind::Detection)
            .expect("detection profile")
            .runner,
        RunnerKind::Detection
    );
}

#[test]
fn registry_returns_explicit_unsupported_diagnostic_for_adapter_family() {
    let diagnostic = ComfyRunnerProfileRegistry::new()
        .profile(ModelFamilyKind::Adapter)
        .expect_err("adapter has no runner profile");

    assert_eq!(
        diagnostic.code,
        crate::comfy_runner_profiles::RUNNER_PROFILE_UNSUPPORTED_CODE
    );
}

#[test]
fn world_model_profile_rejects_non_video_runner() {
    let image = ComfyRunnerProfileRegistry::new()
        .profile(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl profile");
    let diagnostic = ComfyWorldModelProfileBuilder::new()
        .build(image)
        .expect_err("image profile rejected");

    assert_eq!(
        diagnostic.code,
        crate::comfy_world_model_profiles::WORLD_MODEL_PROFILE_UNSUPPORTED_CODE
    );
}

#[test]
fn registry_lists_all_supported_native_profiles() {
    let profiles = ComfyRunnerProfileRegistry::new().supported_profiles();

    assert!(
        profiles
            .iter()
            .any(|profile| profile.family == ModelFamilyKind::StableDiffusionXl)
    );
    assert!(
        profiles
            .iter()
            .any(|profile| profile.family == ModelFamilyKind::WanVideo)
    );
    assert!(
        profiles
            .iter()
            .any(|profile| profile.family == ModelFamilyKind::Audio)
    );
    assert!(
        !profiles
            .iter()
            .any(|profile| profile.family == ModelFamilyKind::Adapter)
    );
}
