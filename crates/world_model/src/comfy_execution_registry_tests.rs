use crate::{
    ComfyExecutionRegistry, DivergenceReason, ExecutionBehaviorKey, GuidanceMode, LatentFormat,
    ModelFamilyKind, ModelMediaCapability, SamplerKind, SchedulerKind,
};

#[test]
fn registry_resolves_comfy_sampler_aliases() {
    let registry = ComfyExecutionRegistry::new();
    let sampler = registry
        .sampler("dpm++ 2m")
        .expect("sampler alias resolves");

    assert_eq!(sampler.kind, SamplerKind::Dpmpp2M);
    assert!(sampler.supports_deterministic_noise);
    assert!(sampler.supports_start_end_steps);
    assert!(
        sampler
            .supported_schedulers
            .contains(&SchedulerKind::Karras)
    );
}

#[test]
fn registry_resolves_scheduler_and_guidance_aliases() {
    let registry = ComfyExecutionRegistry::new();
    let scheduler = registry.scheduler("sgm uniform").expect("scheduler");
    let guidance = registry
        .guidance("classifier free guidance")
        .expect("guidance");

    assert_eq!(scheduler.kind, SchedulerKind::SgmUniform);
    assert_eq!(guidance.mode, GuidanceMode::ClassifierFree);
    assert!(guidance.supports_cfg_scale);
}

#[test]
fn registry_exposes_image_family_execution_profiles() {
    let registry = ComfyExecutionRegistry::new();
    let sdxl = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl profile");
    let flux = registry
        .model_family(ModelFamilyKind::Flux)
        .expect("flux profile");

    assert_eq!(sdxl.latent_format, LatentFormat::StableDiffusionXl);
    assert!(sdxl.media.contains(&ModelMediaCapability::Image));
    assert!(sdxl.supported_samplers.contains(&SamplerKind::Dpmpp2M));
    assert!(
        sdxl.supported_guidance
            .contains(&GuidanceMode::ClassifierFree)
    );
    assert_eq!(flux.latent_format, LatentFormat::Flux);
    assert!(
        flux.supported_guidance
            .contains(&GuidanceMode::FluxGuidance)
    );
}

#[test]
fn registry_exposes_temporal_world_model_profiles() {
    let registry = ComfyExecutionRegistry::new();

    for family in [
        ModelFamilyKind::WanVideo,
        ModelFamilyKind::HunyuanVideo,
        ModelFamilyKind::LtxVideo,
    ] {
        let profile = registry.model_family(family).expect("video profile");
        assert!(profile.temporal);
        assert_eq!(profile.latent_format, LatentFormat::Video);
        assert!(profile.media.contains(&ModelMediaCapability::Video));
        assert!(
            profile
                .supported_guidance
                .contains(&GuidanceMode::VideoGuidance)
        );
    }
}

#[test]
fn registry_exposes_specialized_non_sampling_profiles() {
    let registry = ComfyExecutionRegistry::new();
    let depth = registry
        .model_family(ModelFamilyKind::Depth)
        .expect("depth profile");
    let detection = registry
        .model_family(ModelFamilyKind::Detection)
        .expect("detection profile");

    assert!(depth.media.contains(&ModelMediaCapability::Depth));
    assert!(depth.media.contains(&ModelMediaCapability::ThreeD));
    assert!(depth.supported_samplers.is_empty());
    assert!(detection.media.contains(&ModelMediaCapability::Detection));
}

#[test]
fn registry_records_machine_readable_divergences() {
    let registry = ComfyExecutionRegistry::new();
    let divergence = registry
        .divergence(&ExecutionBehaviorKey::new("implicit_model_downloads"))
        .expect("divergence record");

    assert_eq!(divergence.reason, DivergenceReason::DependencyReview);
    assert!(divergence.sim_behavior.contains("explicitly approved"));
}

#[test]
fn registry_lists_all_native_execution_profiles() {
    let registry = ComfyExecutionRegistry::new();
    let families = registry
        .model_families()
        .into_iter()
        .map(|profile| profile.family)
        .collect::<std::collections::BTreeSet<_>>();

    for family in [
        ModelFamilyKind::StableDiffusion1,
        ModelFamilyKind::StableDiffusion2,
        ModelFamilyKind::StableDiffusionXl,
        ModelFamilyKind::StableDiffusion3,
        ModelFamilyKind::Flux,
        ModelFamilyKind::WanVideo,
        ModelFamilyKind::HunyuanVideo,
        ModelFamilyKind::LtxVideo,
        ModelFamilyKind::Audio,
        ModelFamilyKind::ThreeD,
        ModelFamilyKind::Segmentation,
        ModelFamilyKind::Depth,
        ModelFamilyKind::Detection,
    ] {
        assert!(families.contains(&family), "missing {family:?}");
    }
}
