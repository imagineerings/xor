use std::collections::BTreeMap;

use serde_json::json;

use crate::{
    ComfyConditioningRuntime, ComfyExecutionRegistry, ConditioningArea, ConditioningMask,
    ConditioningMode, ConditioningRegion, ConditioningRuntimeContext, ConditioningTransform,
    ConditioningTransformKind, ControlAttachment, ControlAttachmentKind, DeviceBackend,
    EncoderIdentity, EncoderKind, GuidanceMode, InpaintConditioning, LatentFormat, ModelFamilyKind,
    PromptMetadata, PromptRole, SamplerKind, TensorDescriptor, TensorDtype,
};

#[test]
fn runtime_preserves_prompt_attention_region_and_control_metadata() {
    let bundle = conditioning_bundle();

    assert_eq!(bundle.encoder.kind, EncoderKind::Clip);
    assert_eq!(bundle.token_embeddings.shape, vec![1, 77, 2048]);
    assert_eq!(
        bundle.attention_metadata.metadata.get("clip_skip"),
        Some(&json!(2))
    );
    assert_eq!(bundle.source_prompts[0].text, "a cinematic city");
    assert_eq!(bundle.regions[0].area.expect("area").width, 512);
    assert!(bundle.regions[0].inpaint.is_some());
    assert_eq!(
        bundle.control_attachments[0].kind,
        ControlAttachmentKind::ControlNet
    );
    assert_eq!(
        bundle.transforms[0].kind,
        ConditioningTransformKind::SetRange
    );
}

#[test]
fn runtime_derives_required_modes_from_native_bundle() {
    let runtime = ComfyConditioningRuntime::new();
    let bundle = conditioning_bundle();
    let modes = runtime.required_modes(&bundle);

    assert!(modes.contains(&ConditioningMode::Text));
    assert!(modes.contains(&ConditioningMode::Image));
    assert!(modes.contains(&ConditioningMode::Mask));
    assert!(modes.contains(&ConditioningMode::Control));
}

#[test]
fn runtime_accepts_supported_image_conditioning_context() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let context = ConditioningRuntimeContext {
        sampler: SamplerKind::Dpmpp2M,
        guidance: GuidanceMode::ClassifierFree,
        latent_format: LatentFormat::StableDiffusionXl,
        backend: DeviceBackend::Cuda,
        worker_supports_control_attachments: true,
    };

    ComfyConditioningRuntime::new()
        .validate(&conditioning_bundle(), family, &context)
        .expect("conditioning is valid");
}

#[test]
fn runtime_rejects_unsupported_family_conditioning_modes() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::Segmentation)
        .expect("segmentation family");
    let context = ConditioningRuntimeContext {
        sampler: SamplerKind::Euler,
        guidance: GuidanceMode::ClassifierFree,
        latent_format: LatentFormat::None,
        backend: DeviceBackend::Cuda,
        worker_supports_control_attachments: true,
    };

    let diagnostics = ComfyConditioningRuntime::new()
        .validate(&conditioning_bundle(), family, &context)
        .expect_err("control conditioning is unsupported");

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_conditioning::UNSUPPORTED_CONDITIONING_CODE
    }));
}

#[test]
fn runtime_rejects_latent_sampler_guidance_and_backend_mismatches() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::Flux)
        .expect("flux family");
    let context = ConditioningRuntimeContext {
        sampler: SamplerKind::Dpmpp2M,
        guidance: GuidanceMode::ClassifierFree,
        latent_format: LatentFormat::StableDiffusionXl,
        backend: DeviceBackend::Cuda,
        worker_supports_control_attachments: false,
    };

    let diagnostics = ComfyConditioningRuntime::new()
        .validate(&conditioning_bundle(), family, &context)
        .expect_err("mismatches rejected");

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::comfy_conditioning::LATENT_MISMATCH_CODE
        })
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::comfy_conditioning::SAMPLER_MISMATCH_CODE
        })
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::comfy_conditioning::BACKEND_MISMATCH_CODE
        })
    );
}

#[test]
fn runtime_rejects_empty_tensors_and_invalid_regions() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let context = ConditioningRuntimeContext {
        sampler: SamplerKind::Dpmpp2M,
        guidance: GuidanceMode::ClassifierFree,
        latent_format: LatentFormat::StableDiffusionXl,
        backend: DeviceBackend::Cuda,
        worker_supports_control_attachments: true,
    };
    let mut bundle = conditioning_bundle();
    bundle.token_embeddings.shape.clear();
    bundle.source_prompts[0].text.clear();
    bundle.regions[0].strength = 1.5;
    bundle.regions[0].start_percent = Some(0.8);
    bundle.regions[0].end_percent = Some(0.2);
    bundle.regions[0].area = Some(ConditioningArea {
        x: 0,
        y: 0,
        width: 0,
        height: 512,
    });

    let diagnostics = ComfyConditioningRuntime::new()
        .validate(&bundle, family, &context)
        .expect_err("invalid bundle rejected");

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == crate::comfy_conditioning::EMPTY_TENSOR_CODE })
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == crate::comfy_conditioning::EMPTY_BUNDLE_CODE })
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::comfy_conditioning::INVALID_REGION_CODE
        })
    );
}

fn conditioning_bundle() -> crate::ConditioningBundle {
    crate::ConditioningBundle {
        id: "conditioning-positive".to_string(),
        encoder: EncoderIdentity {
            kind: EncoderKind::Clip,
            model_family: ModelFamilyKind::StableDiffusionXl,
            model_ref: "clip-large".to_string(),
            tokenizer: Some("open_clip".to_string()),
            layer_skip: Some(2),
        },
        token_embeddings: TensorDescriptor {
            id: "tokens".to_string(),
            shape: vec![1, 77, 2048],
            dtype: TensorDtype::F16,
        },
        pooled_output: Some(TensorDescriptor {
            id: "pooled".to_string(),
            shape: vec![1, 1280],
            dtype: TensorDtype::F16,
        }),
        attention_metadata: crate::AttentionMetadata {
            clip_layer: Some(-2),
            token_weights: BTreeMap::from([("cinematic".to_string(), 1.2)]),
            metadata: BTreeMap::from([("clip_skip".to_string(), json!(2))]),
        },
        source_prompts: vec![PromptMetadata {
            node_id: "6".to_string(),
            role: PromptRole::Positive,
            text: "a cinematic city".to_string(),
        }],
        regions: vec![ConditioningRegion {
            area: Some(ConditioningArea {
                x: 0,
                y: 0,
                width: 512,
                height: 512,
            }),
            mask: Some(ConditioningMask {
                tensor: TensorDescriptor {
                    id: "mask".to_string(),
                    shape: vec![1, 512, 512],
                    dtype: TensorDtype::F32,
                },
                strength: 0.75,
            }),
            strength: 0.8,
            start_percent: Some(0.0),
            end_percent: Some(0.7),
            inpaint: Some(InpaintConditioning {
                image_ref: "image://source".to_string(),
                mask_ref: "mask://source".to_string(),
                preserve_masked_latent: true,
            }),
        }],
        control_attachments: vec![ControlAttachment {
            kind: ControlAttachmentKind::ControlNet,
            source_ref: "control://canny".to_string(),
            strength: 0.9,
            start_percent: Some(0.0),
            end_percent: Some(1.0),
            metadata: BTreeMap::from([("preprocessor".to_string(), json!("canny"))]),
        }],
        transforms: vec![ConditioningTransform {
            kind: ConditioningTransformKind::SetRange,
            source_node_id: "12".to_string(),
            metadata: BTreeMap::from([("start_percent".to_string(), json!(0.0))]),
        }],
    }
}
