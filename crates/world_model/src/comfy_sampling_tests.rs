use crate::{
    ComfyExecutionRegistry, ComfySamplingRequestBuilder, DenoiseRange, DeviceBackend,
    LatentDescriptor, ModelFamilyKind, NoisePolicy, PrecisionPolicy, SamplingNodeKind,
    SamplingRunInput,
};

#[test]
fn builder_captures_ksampler_inputs_and_deterministic_metadata() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let request = ComfySamplingRequestBuilder::new()
        .build(&registry, family, image_input())
        .expect("request builds");

    assert_eq!(request.node_kind, SamplingNodeKind::KSampler);
    assert_eq!(request.seed, 42);
    assert_eq!(request.steps, 30);
    assert_eq!(request.cfg_scale, 7.5);
    assert_eq!(request.denoise.amount, 0.8);
    assert_eq!(request.positive_conditioning, "positive-conditioning");
    assert_eq!(
        request.negative_conditioning.as_deref(),
        Some("negative-conditioning")
    );
    let deterministic = request.deterministic.expect("deterministic metadata");
    assert_eq!(deterministic.seed, 42);
    assert_eq!(deterministic.noise_seed, Some(123));
    assert_eq!(deterministic.backend, DeviceBackend::Cuda);
    assert_eq!(deterministic.precision, PrecisionPolicy::Fp16);
    assert_eq!(deterministic.model_hash.as_deref(), Some("model-hash"));
}

#[test]
fn builder_rejects_unknown_sampler_scheduler_and_guidance() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let mut input = image_input();
    input.sampler_name = "mystery".to_string();
    input.scheduler_name = "strange".to_string();
    input.guidance_name = "opaque".to_string();
    let diagnostics = ComfySamplingRequestBuilder::new()
        .build(&registry, family, input)
        .expect_err("unsupported settings rejected");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_sampling::UNSUPPORTED_SAMPLER_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_sampling::UNSUPPORTED_SCHEDULER_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_sampling::UNSUPPORTED_GUIDANCE_CODE)
    );
}

#[test]
fn builder_rejects_family_unsupported_guidance() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::Flux)
        .expect("flux family");
    let mut input = image_input();
    input.guidance_name = "classifier_free".to_string();
    let diagnostics = ComfySamplingRequestBuilder::new()
        .build(&registry, family, input)
        .expect_err("guidance rejected");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_sampling::UNSUPPORTED_GUIDANCE_CODE)
    );
}

#[test]
fn builder_rejects_invalid_steps_denoise_and_latent() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let mut input = image_input();
    input.steps = 0;
    input.denoise.amount = 1.5;
    input.denoise.start_step = Some(20);
    input.denoise.end_step = Some(10);
    input.latent.width = 0;
    let diagnostics = ComfySamplingRequestBuilder::new()
        .build(&registry, family, input)
        .expect_err("invalid input rejected");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_sampling::INVALID_STEPS_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_sampling::INVALID_DENOISE_CODE)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_sampling::INVALID_LATENT_CODE)
    );
}

#[test]
fn builder_requires_frames_for_temporal_model_families() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::WanVideo)
        .expect("wan family");
    let mut input = image_input();
    input.guidance_name = "video_guidance".to_string();
    input.latent.frames = None;
    let diagnostics = ComfySamplingRequestBuilder::new()
        .build(&registry, family, input)
        .expect_err("missing frames rejected");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == crate::comfy_sampling::INVALID_LATENT_CODE)
    );
}

#[test]
fn builder_rejects_deterministic_runs_when_worker_cannot_reproduce() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let mut input = image_input();
    input.worker_supports_determinism = false;
    let diagnostics = ComfySamplingRequestBuilder::new()
        .build(&registry, family, input)
        .expect_err("determinism rejected");

    assert_eq!(
        diagnostics[0].code,
        crate::comfy_sampling::DETERMINISM_UNSUPPORTED_CODE
    );
}

#[test]
fn progress_clamps_current_step_and_preserves_preview_and_cancel_state() {
    let progress = ComfySamplingRequestBuilder::new().progress(12, 10, true, true);

    assert_eq!(progress.current_step, 10);
    assert_eq!(progress.total_steps, 10);
    assert!(progress.preview_available);
    assert!(progress.cancellation_requested);
}

fn image_input() -> SamplingRunInput {
    SamplingRunInput {
        node_kind: SamplingNodeKind::KSampler,
        sampler_name: "dpm++ 2m".to_string(),
        scheduler_name: "karras".to_string(),
        guidance_name: "classifier_free".to_string(),
        seed: 42,
        noise_policy: NoisePolicy::Fixed { noise_seed: 123 },
        steps: 30,
        cfg_scale: 7.5,
        denoise: DenoiseRange {
            amount: 0.8,
            start_step: Some(0),
            end_step: Some(29),
        },
        latent: LatentDescriptor {
            width: 1024,
            height: 1024,
            channels: 4,
            frames: None,
        },
        positive_conditioning: "positive-conditioning".to_string(),
        negative_conditioning: Some("negative-conditioning".to_string()),
        model_profile: "sdxl-base".to_string(),
        model_hash: Some("model-hash".to_string()),
        deterministic: true,
        worker_supports_determinism: true,
        backend: DeviceBackend::Cuda,
        precision: PrecisionPolicy::Fp16,
    }
}
