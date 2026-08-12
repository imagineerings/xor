use comfy_model::{
    IDEOGRAM4_SOURCE_PATH, IDEOGRAM4_SOURCE_SHA256, IDEOGRAM4_TAP_LAYERS, JINA_CLIP2_SOURCE_PATH,
    JINA_CLIP2_SOURCE_SHA256, MULTIMODAL_TEXT_ENCODER_CATALOG_SYMBOLS, MultimodalFamily,
    MultimodalImageEmbedding, MultimodalSpan, MultimodalSymbolBehavior, MultimodalTextError,
    NativeQwenVisionEncoder, OVIS_SOURCE_PATH, OVIS_SOURCE_SHA256, QWEN_VL_SOURCE_PATH,
    QWEN_VL_SOURCE_SHA256, QWEN3VL_IMAGE_PAD_TOKEN, QWEN3VL_SOURCE_PATH, QWEN3VL_SOURCE_SHA256,
    QWEN35_IMAGE_MEAN, QWEN35_IMAGE_PAD_TOKEN, QWEN35_IMAGE_STANDARD_DEVIATION,
    QwenVisionBlockWeights, QwenVisionConfiguration, QwenVisionFamily, QwenVisionMergerWeights,
    QwenVisionWeights, SAM3_CLIP_SOURCE_PATH, SAM3_CLIP_SOURCE_SHA256, Sam3EncodedCondition,
    format_ideogram4_prompt, format_ovis_prompt, format_qwen3vl_prompt, ideogram4_project_taps,
    join_multimodal_embeddings, join_qwen3vl_deepstack, multimodal_profile,
    multimodal_symbol_behavior, ovis_template_end, pack_sam3_conditions, parse_sam3_prompts,
    plan_qwen_markers, plan_qwen3vl_markers, prepare_qwen_images, prepare_qwen3vl_images,
    qwen2vl_mrope_position_ids, qwen3vl_target_dimensions, trim_ovis_conditioning,
};
use comfy_tensor::{
    CancellationToken, CpuBackend, CpuWorkspaceAuthority, DType, DeviceId, ExecutionContext,
    ImageTensor, StreamId, Tensor, TensorDescriptor, generated_native_diffusion::tensor_to_f32,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{error::Error, fs, path::Path};

const MEMORY_LIMIT: u64 = 64 * 1024 * 1024;
const TASK_ID: &str = "comfy-parity-clip-text-encoder-multimodal-foundation";
const COMPOSITE_TASK_ID: &str = "comfy-parity-clip-text-encoder-composite-adapters";
const IMPLEMENTATION_CLOSURE: [&str; 3] = [
    "crates/comfy_model/src/clip_text_encoder_multimodal.rs",
    "crates/comfy_model/src/comfy_model.rs",
    "crates/comfy_model/tests/clip_text_encoder_multimodal.rs",
];

fn backend() -> Result<(CpuBackend, CpuWorkspaceAuthority), Box<dyn Error>> {
    Ok(CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?)
}

fn context<'a>(
    authority: &CpuWorkspaceAuthority,
    cancellation: &'a CancellationToken,
    scratch: u64,
) -> Result<ExecutionContext<'a>, Box<dyn Error>> {
    Ok(ExecutionContext {
        stream: StreamId::DEFAULT,
        scratch: authority.authorize_workspace(scratch)?,
        rng_phase: None,
        cancellation,
    })
}

fn tensor(
    backend: &CpuBackend,
    shape: &[u64],
    values: &[f32],
    context: &ExecutionContext<'_>,
) -> Result<Tensor, Box<dyn Error>> {
    let descriptor =
        TensorDescriptor::contiguous(shape.to_vec(), DType::F32, DeviceId::CPU, context.stream)?;
    Ok(backend.upload_f32(descriptor, values, context)?.0)
}

fn filled_tensor(
    backend: &CpuBackend,
    shape: &[u64],
    value: f32,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, Box<dyn Error>> {
    let elements = shape.iter().try_fold(1_usize, |total, dimension| {
        total.checked_mul(usize::try_from(*dimension).ok()?)
    });
    tensor(
        backend,
        shape,
        &vec![value; elements.ok_or("fixture tensor shape overflowed")?],
        context,
    )
}

fn reduced_qwen_vision_weights(
    backend: &CpuBackend,
    configuration: &QwenVisionConfiguration,
    context: &ExecutionContext<'_>,
) -> Result<QwenVisionWeights, Box<dyn Error>> {
    let hidden = u64::try_from(configuration.hidden_size)?;
    let intermediate = u64::try_from(configuration.intermediate_size)?;
    let output = u64::try_from(configuration.output_hidden_size)?;
    let merge_width = hidden.checked_mul(4).ok_or("merge width overflowed")?;
    let patch_width = 3 * 2 * 16 * 16;
    let mut patch_values = vec![0.0_f32; configuration.hidden_size * patch_width];
    for hidden_index in 0..configuration.hidden_size {
        patch_values[hidden_index * patch_width + hidden_index] =
            0.125 + hidden_index as f32 * 0.025;
    }
    let patch_weight = tensor(
        backend,
        &[hidden, u64::try_from(patch_width)?],
        &patch_values,
        context,
    )?;
    let patch_bias = filled_tensor(backend, &[hidden], 0.01, context)?;
    let mut position_values = vec![0.0_f32; 2304 * configuration.hidden_size];
    for position in 0..2304 {
        for hidden_index in 0..configuration.hidden_size {
            position_values[position * configuration.hidden_size + hidden_index] =
                position as f32 * 0.0001 + hidden_index as f32 * 0.001;
        }
    }
    let position_embedding = tensor(backend, &[2304, hidden], &position_values, context)?;
    let mut blocks = Vec::new();
    for layer in 0..configuration.layer_count {
        let scale = 0.001 + layer as f32 * 0.00001;
        blocks.push(QwenVisionBlockWeights {
            normalization_one_weight: filled_tensor(backend, &[hidden], 1.0, context)?,
            normalization_one_bias: filled_tensor(backend, &[hidden], 0.0, context)?,
            query_key_value_weight: filled_tensor(backend, &[hidden * 3, hidden], scale, context)?,
            query_key_value_bias: filled_tensor(backend, &[hidden * 3], 0.0, context)?,
            attention_output_weight: filled_tensor(backend, &[hidden, hidden], scale, context)?,
            attention_output_bias: filled_tensor(backend, &[hidden], 0.0, context)?,
            normalization_two_weight: filled_tensor(backend, &[hidden], 1.0, context)?,
            normalization_two_bias: filled_tensor(backend, &[hidden], 0.0, context)?,
            feed_forward_up_weight: filled_tensor(
                backend,
                &[intermediate, hidden],
                scale,
                context,
            )?,
            feed_forward_up_bias: filled_tensor(backend, &[intermediate], 0.0, context)?,
            feed_forward_down_weight: filled_tensor(
                backend,
                &[hidden, intermediate],
                scale,
                context,
            )?,
            feed_forward_down_bias: filled_tensor(backend, &[hidden], 0.0, context)?,
        });
    }
    let merger = QwenVisionMergerWeights {
        normalization_weight: filled_tensor(backend, &[hidden], 1.0, context)?,
        normalization_bias: filled_tensor(backend, &[hidden], 0.0, context)?,
        first_weight: filled_tensor(backend, &[merge_width, merge_width], 0.01, context)?,
        first_bias: filled_tensor(backend, &[merge_width], 0.0, context)?,
        second_weight: filled_tensor(backend, &[output, merge_width], 0.02, context)?,
        second_bias: filled_tensor(backend, &[output], 0.0, context)?,
    };
    let mut deepstack_mergers = Vec::new();
    for _ in configuration.family.deepstack_layers() {
        deepstack_mergers.push(QwenVisionMergerWeights {
            normalization_weight: filled_tensor(backend, &[merge_width], 1.0, context)?,
            normalization_bias: filled_tensor(backend, &[merge_width], 0.0, context)?,
            first_weight: filled_tensor(backend, &[merge_width, merge_width], 0.015, context)?,
            first_bias: filled_tensor(backend, &[merge_width], 0.0, context)?,
            second_weight: filled_tensor(backend, &[output, merge_width], 0.025, context)?,
            second_bias: filled_tensor(backend, &[output], 0.0, context)?,
        });
    }
    Ok(QwenVisionWeights {
        patch_weight,
        patch_bias,
        position_embedding,
        blocks,
        merger,
        deepstack_mergers,
    })
}

#[test]
fn source_profiles_and_total_symbol_map_are_exact() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        IDEOGRAM4_TAP_LAYERS,
        [1, 4, 7, 10, 13, 16, 19, 22, 25, 28, 31, 34, 36]
    );
    let ideogram = multimodal_profile(MultimodalFamily::Ideogram4);
    assert_eq!(ideogram.hidden_size, 4096);
    assert_eq!(ideogram.projection_size, Some(53_248));
    assert_eq!(ideogram.rope_theta(), Some(5_000_000.0));
    let jina = multimodal_profile(MultimodalFamily::JinaClip2);
    assert_eq!((jina.vocabulary_size, jina.maximum_tokens), (250_002, 8192));
    assert_eq!(jina.rope_theta(), Some(20_000.0));
    let ovis = multimodal_profile(MultimodalFamily::Ovis25);
    assert_eq!((ovis.hidden_size, ovis.pad_token), (2048, 151_643));
    let qwen4 = multimodal_profile(MultimodalFamily::Qwen3Vl4B);
    assert_eq!(
        (qwen4.hidden_size, qwen4.visual_layer_count),
        (2560, Some(24))
    );
    let qwen8 = multimodal_profile(MultimodalFamily::Qwen3Vl8B);
    assert_eq!(
        (qwen8.hidden_size, qwen8.visual_layer_count),
        (4096, Some(27))
    );
    let qwen2 = multimodal_profile(MultimodalFamily::Qwen2Vl);
    assert_eq!((qwen2.patch_size, qwen2.layer_count), (Some(14), 32));
    let sam3 = multimodal_profile(MultimodalFamily::Sam3);
    assert_eq!((sam3.maximum_tokens, sam3.projection_size), (32, Some(512)));

    let workspace = workspace()?;
    for (path, expected) in [
        (IDEOGRAM4_SOURCE_PATH, IDEOGRAM4_SOURCE_SHA256),
        (JINA_CLIP2_SOURCE_PATH, JINA_CLIP2_SOURCE_SHA256),
        (OVIS_SOURCE_PATH, OVIS_SOURCE_SHA256),
        (QWEN3VL_SOURCE_PATH, QWEN3VL_SOURCE_SHA256),
        (QWEN_VL_SOURCE_PATH, QWEN_VL_SOURCE_SHA256),
        (SAM3_CLIP_SOURCE_PATH, SAM3_CLIP_SOURCE_SHA256),
    ] {
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(workspace.join(path))?)),
            expected
        );
    }
    assert!(multimodal_symbol_behavior(QWEN_VL_SOURCE_PATH, "PythonFallback").is_none());
    Ok(())
}

#[test]
fn qwen_multimodal_positions_match_source_order_and_fail_closed() -> Result<(), Box<dyn Error>> {
    let cancellation = CancellationToken::default();
    let span = MultimodalSpan {
        start: 2,
        size: 4,
        grid_thw: [1, 4, 4],
    };
    let positions = qwen2vl_mrope_position_ids(10, &[span], &cancellation)?
        .ok_or("image positions were not constructed")?;
    assert_eq!(positions.temporal(), &[0, 1, 2, 2, 2, 2, 4, 5, 6, 7]);
    assert_eq!(positions.height(), &[0, 1, 2, 2, 3, 3, 4, 5, 6, 7]);
    assert_eq!(positions.width(), &[0, 1, 2, 3, 2, 3, 4, 5, 6, 7]);
    assert_eq!(qwen2vl_mrope_position_ids(4, &[], &cancellation)?, None);
    assert!(matches!(
        qwen2vl_mrope_position_ids(
            10,
            &[
                span,
                MultimodalSpan {
                    start: 5,
                    size: 2,
                    grid_thw: [1, 4, 4]
                },
            ],
            &cancellation,
        ),
        Err(MultimodalTextError::InvalidInput(_))
    ));
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(matches!(
        qwen2vl_mrope_position_ids(10, &[span], &cancelled),
        Err(MultimodalTextError::Cancelled)
    ));
    Ok(())
}

#[test]
fn qwen3vl_resize_patch_packing_and_batch_splitting_are_source_exact() -> Result<(), Box<dyn Error>>
{
    assert_eq!(qwen3vl_target_dimensions(48, 96)?, (64, 96));
    assert_eq!(qwen3vl_target_dimensions(17, 31)?, (64, 96));
    let (large_height, large_width) = qwen3vl_target_dimensions(10_000, 20_000)?;
    assert_eq!((large_height % 32, large_width % 32), (0, 0));
    assert!(
        large_height
            .checked_mul(large_width)
            .ok_or("large image overflow")?
            <= 12_845_056
    );
    assert!(qwen3vl_target_dimensions(0, 32).is_err());
    assert!(qwen3vl_target_dimensions(1, u64::MAX / 2).is_err());

    let (backend, authority) = backend()?;
    let cancellation = CancellationToken::default();
    let context = context(&authority, &cancellation, 48 * 1024 * 1024)?;
    let mut pixels = vec![0.5_f32; 2 * 17 * 31 * 3];
    pixels[17 * 31 * 3] = 0.75;
    let images = ImageTensor::from_f32(&backend, &context, 2, 17, 31, 3, &pixels)?;
    let prepared = prepare_qwen3vl_images(&backend, &images, &context)?;
    assert_eq!(prepared.len(), 2);
    for image in &prepared {
        assert_eq!(image.grid_thw(), [1, 4, 6]);
        assert_eq!(image.merged_tokens(), 6);
        assert_eq!(image.patches().descriptor().shape(), [24, 3, 2, 16, 16]);
    }
    let first = tensor_to_f32(&backend, prepared[0].patches(), &context)?;
    assert!(first.iter().all(|value| *value == 0.0));
    let second = tensor_to_f32(&backend, prepared[1].patches(), &context)?;
    assert!(second.iter().any(|value| *value > 0.0));
    drop(first);
    drop(second);
    drop(prepared);
    drop(images);

    let mut ordered_pixels = vec![0.5_f32; 64 * 64 * 3];
    ordered_pixels[0] = 0.75;
    ordered_pixels[(16 * 3) + 1] = 0.25;
    ordered_pixels[(16 * 64 * 3) + 2] = 1.0;
    ordered_pixels[(32 * 3) + 1] = 0.0;
    let ordered_image = ImageTensor::from_f32(&backend, &context, 1, 64, 64, 3, &ordered_pixels)?;
    let ordered = prepare_qwen3vl_images(&backend, &ordered_image, &context)?;
    let ordered_patches = tensor_to_f32(&backend, ordered[0].patches(), &context)?;
    let patch_width = 3 * 2 * 16 * 16;
    assert_eq!(ordered_patches[0], 0.5);
    assert_eq!(ordered_patches[16 * 16], 0.5);
    assert_eq!(ordered_patches[patch_width + (2 * 16 * 16)], -0.5);
    assert_eq!(ordered_patches[patch_width + (3 * 16 * 16)], -0.5);
    assert_eq!(ordered_patches[(2 * patch_width) + (4 * 16 * 16)], 1.0);
    assert_eq!(ordered_patches[(2 * patch_width) + (5 * 16 * 16)], 1.0);
    assert_eq!(ordered_patches[(4 * patch_width) + (2 * 16 * 16)], -1.0);
    assert_eq!(ordered_patches[(4 * patch_width) + (3 * 16 * 16)], -1.0);
    assert_eq!(
        ordered_patches
            .iter()
            .filter(|value| **value != 0.0)
            .count(),
        8
    );
    drop(ordered_patches);
    drop(ordered);
    drop(ordered_image);
    assert_eq!(context.scratch.in_use_bytes(), 0);
    Ok(())
}

#[test]
fn qwen3vl_marker_plan_expands_real_image_spans_and_fails_closed() -> Result<(), Box<dyn Error>> {
    let (backend, authority) = backend()?;
    let cancellation = CancellationToken::default();
    let context = context(&authority, &cancellation, 48 * 1024 * 1024)?;
    let first = ImageTensor::from_f32(&backend, &context, 1, 32, 32, 3, &vec![0.5; 3_072])?;
    let second = ImageTensor::from_f32(&backend, &context, 1, 64, 32, 3, &vec![0.5; 6_144])?;
    let first = prepare_qwen3vl_images(&backend, &first, &context)?;
    let second = prepare_qwen3vl_images(&backend, &second, &context)?;
    let images = [first[0].clone(), second[0].clone()];
    assert_eq!(
        (images[0].merged_tokens(), images[1].merged_tokens()),
        (4, 6)
    );
    let plan = plan_qwen3vl_markers(
        &[10, QWEN3VL_IMAGE_PAD_TOKEN, 11, QWEN3VL_IMAGE_PAD_TOKEN, 12],
        &images,
        &cancellation,
    )?;
    assert_eq!(
        plan.expanded_tokens(),
        &[
            10,
            QWEN3VL_IMAGE_PAD_TOKEN,
            QWEN3VL_IMAGE_PAD_TOKEN,
            QWEN3VL_IMAGE_PAD_TOKEN,
            QWEN3VL_IMAGE_PAD_TOKEN,
            11,
            QWEN3VL_IMAGE_PAD_TOKEN,
            QWEN3VL_IMAGE_PAD_TOKEN,
            QWEN3VL_IMAGE_PAD_TOKEN,
            QWEN3VL_IMAGE_PAD_TOKEN,
            QWEN3VL_IMAGE_PAD_TOKEN,
            QWEN3VL_IMAGE_PAD_TOKEN,
            12,
        ]
    );
    assert_eq!(
        plan.spans(),
        &[
            MultimodalSpan {
                start: 1,
                size: 4,
                grid_thw: [1, 4, 4]
            },
            MultimodalSpan {
                start: 6,
                size: 6,
                grid_thw: [1, 6, 4]
            },
        ]
    );
    assert_eq!(
        plan.visual_position_mask()
            .iter()
            .filter(|value| **value)
            .count(),
        10
    );
    assert!(plan_qwen3vl_markers(&[QWEN3VL_IMAGE_PAD_TOKEN], &images, &cancellation).is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(matches!(
        plan_qwen3vl_markers(&[], &[], &cancelled),
        Err(MultimodalTextError::Cancelled)
    ));
    drop(plan);
    drop(images);
    drop(first);
    drop(second);
    assert_eq!(context.scratch.in_use_bytes(), 0);
    Ok(())
}

#[test]
fn qwen3vl_preparation_cancellation_and_oom_leave_workspace_empty() -> Result<(), Box<dyn Error>> {
    let (backend, authority) = backend()?;
    let cancellation = CancellationToken::default();
    let setup_context = context(&authority, &cancellation, 8 * 1024 * 1024)?;
    let image = ImageTensor::from_f32(
        &backend,
        &setup_context,
        1,
        64,
        64,
        3,
        &vec![0.5; 64 * 64 * 3],
    )?;

    let cancelled = CancellationToken::default();
    cancelled.cancel();
    let cancelled_context = context(&authority, &cancelled, 8 * 1024 * 1024)?;
    assert!(matches!(
        prepare_qwen3vl_images(&backend, &image, &cancelled_context),
        Err(MultimodalTextError::Cancelled)
    ));
    assert_eq!(cancelled_context.scratch.in_use_bytes(), 0);

    let constrained = context(&authority, &cancellation, 1_024)?;
    assert!(prepare_qwen3vl_images(&backend, &image, &constrained).is_err());
    assert_eq!(constrained.scratch.in_use_bytes(), 0);

    let invalid = ImageTensor::from_f32(
        &backend,
        &setup_context,
        1,
        32,
        32,
        3,
        &vec![f32::NAN; 32 * 32 * 3],
    )?;
    assert!(matches!(
        prepare_qwen3vl_images(&backend, &invalid, &setup_context),
        Err(MultimodalTextError::InvalidInput(_))
    ));
    Ok(())
}

#[test]
fn retained_qwen_vision_executes_closed_family_graphs_and_rolls_back() -> Result<(), Box<dyn Error>>
{
    let (backend, authority) = backend()?;
    let cancellation = CancellationToken::default();
    let setup = context(&authority, &cancellation, 48 * 1024 * 1024)?;
    let image = ImageTensor::from_f32(&backend, &setup, 1, 32, 32, 3, &vec![0.5; 32 * 32 * 3])?;
    let qwen35_prepared =
        prepare_qwen_images(&backend, &image, QwenVisionFamily::Qwen35_08B, &setup)?;
    let qwen35_values = tensor_to_f32(&backend, qwen35_prepared[0].patches(), &setup)?;
    assert!(
        (qwen35_values[0] - (0.5 - QWEN35_IMAGE_MEAN[0]) / QWEN35_IMAGE_STANDARD_DEVIATION[0])
            .abs()
            < 1.0e-6
    );
    drop(qwen35_values);
    let qwen35_plan = plan_qwen_markers(
        &[1, QWEN35_IMAGE_PAD_TOKEN, 2],
        &qwen35_prepared,
        QwenVisionFamily::Qwen35_08B,
        &cancellation,
    )?;
    assert_eq!(qwen35_plan.spans()[0].size, 4);
    assert!(
        plan_qwen_markers(
            &[1, QWEN3VL_IMAGE_PAD_TOKEN, 2],
            &qwen35_prepared,
            QwenVisionFamily::Qwen35_08B,
            &cancellation,
        )
        .is_err()
    );

    let qwen35_configuration =
        QwenVisionConfiguration::reduced_fixture(QwenVisionFamily::Qwen35_08B, 4, 8, 2, 1, 6);
    let qwen35 = NativeQwenVisionEncoder::new(
        qwen35_configuration.clone(),
        reduced_qwen_vision_weights(&backend, &qwen35_configuration, &setup)?,
    )?;
    let qwen35_digest = qwen35.semantic_state_digest(&cancellation)?;
    assert_eq!(qwen35_digest.len(), 64);
    assert!(qwen35.resident_bytes()? > 0);
    let qwen35_projection = qwen35.project(&backend, &qwen35_prepared[0], &setup)?;
    assert_eq!(qwen35_projection.embedding.descriptor().shape(), [4, 6]);
    assert!(qwen35_projection.deepstack.is_empty());
    let qwen35_output = tensor_to_f32(&backend, &qwen35_projection.embedding, &setup)?;
    assert!(qwen35_output.iter().all(|value| value.is_finite()));
    assert!(qwen35_output.iter().any(|value| *value != 0.0));
    let qwen35_output_digest = format!(
        "{:x}",
        Sha256::digest(
            qwen35_output
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(
        qwen35_output_digest,
        "16a13ffa41fb0a5f742f0e8c3b2364fa4020dd2cb9a71e1309e62d6e143cb8ed"
    );
    let mut batch_pixels = vec![0.5_f32; 2 * 32 * 32 * 3];
    batch_pixels[32 * 32 * 3..].fill(0.75);
    let image_batch = ImageTensor::from_f32(&backend, &setup, 2, 32, 32, 3, &batch_pixels)?;
    let batch_prepared =
        prepare_qwen_images(&backend, &image_batch, QwenVisionFamily::Qwen35_08B, &setup)?;
    let isolated_first = qwen35.project(&backend, &batch_prepared[0], &setup)?;
    let isolated_second = qwen35.project(&backend, &batch_prepared[1], &setup)?;
    assert_eq!(
        &*tensor_to_f32(&backend, &isolated_first.embedding, &setup)?,
        &*qwen35_output
    );
    assert_ne!(
        &*tensor_to_f32(&backend, &isolated_second.embedding, &setup)?,
        &*qwen35_output
    );
    drop(qwen35_output);

    let qwen3_prepared =
        prepare_qwen_images(&backend, &image, QwenVisionFamily::Qwen3Vl4B, &setup)?;
    let qwen3_configuration =
        QwenVisionConfiguration::reduced_fixture(QwenVisionFamily::Qwen3Vl4B, 4, 8, 18, 1, 6);
    let qwen3 = NativeQwenVisionEncoder::new(
        qwen3_configuration.clone(),
        reduced_qwen_vision_weights(&backend, &qwen3_configuration, &setup)?,
    )?;
    let qwen3_projection = qwen3.project(&backend, &qwen3_prepared[0], &setup)?;
    assert_eq!(qwen3_projection.embedding.descriptor().shape(), [4, 6]);
    assert_eq!(qwen3_projection.deepstack.len(), 3);
    assert!(
        qwen3_projection
            .deepstack
            .iter()
            .all(|tensor| tensor.descriptor().shape() == [4, 6])
    );
    assert_ne!(qwen3.semantic_state_digest(&cancellation)?, qwen35_digest);

    let cancelled = CancellationToken::default();
    cancelled.cancel();
    let cancelled_context = context(&authority, &cancelled, 48 * 1024 * 1024)?;
    assert!(matches!(
        qwen35.project(&backend, &qwen35_prepared[0], &cancelled_context),
        Err(MultimodalTextError::Cancelled)
    ));
    assert_eq!(cancelled_context.scratch.in_use_bytes(), 0);

    let constrained = context(&authority, &cancellation, 4)?;
    assert!(
        qwen35
            .project(&backend, &qwen35_prepared[0], &constrained)
            .is_err()
    );
    assert_eq!(constrained.scratch.in_use_bytes(), 0);
    drop(qwen3_projection);
    drop(qwen35_projection);
    drop(qwen3_prepared);
    drop(qwen35_prepared);
    assert_eq!(setup.scratch.in_use_bytes(), 0);
    Ok(())
}

#[test]
fn modality_join_deepstack_and_projection_are_native_and_transactional()
-> Result<(), Box<dyn Error>> {
    let (backend, authority) = backend()?;
    let cancellation = CancellationToken::default();
    let context = context(&authority, &cancellation, 32 * 1024 * 1024)?;
    let text = tensor(
        &backend,
        &[1, 6, 2],
        &(0..12).map(|value| value as f32).collect::<Vec<_>>(),
        &context,
    )?;
    let image = tensor(&backend, &[2, 2], &[100.0, 101.0, 102.0, 103.0], &context)?;
    let deep_a = tensor(&backend, &[2, 2], &[1.0, 2.0, 3.0, 4.0], &context)?;
    let deep_b = tensor(&backend, &[2, 2], &[5.0, 6.0, 7.0, 8.0], &context)?;
    let deepstack = vec![deep_a, deep_b];
    let images = [MultimodalImageEmbedding {
        span: MultimodalSpan {
            start: 2,
            size: 2,
            grid_thw: [1, 4, 4],
        },
        embedding: &image,
        deepstack: &deepstack,
    }];
    let joined = join_multimodal_embeddings(&backend, &text, &images, &context)?;
    assert_eq!(joined.descriptor().shape(), [1, 6, 2]);
    assert_eq!(
        &*tensor_to_f32(&backend, &joined, &context)?,
        &[
            0.0, 1.0, 2.0, 3.0, 100.0, 101.0, 102.0, 103.0, 8.0, 9.0, 10.0, 11.0
        ]
    );
    let joined_deepstack = join_qwen3vl_deepstack(&backend, 6, &images, &context)?
        .ok_or("deepstack join was not produced")?;
    assert_eq!(
        joined_deepstack.visual_position_mask,
        [false, false, true, true, false, false]
    );
    assert_eq!(joined_deepstack.layers.len(), 2);
    assert_eq!(joined_deepstack.layers[0].descriptor().shape(), [2, 2]);

    let taps = tensor(
        &backend,
        &[1, 13, 2, 2],
        &(0..52).map(|value| value as f32).collect::<Vec<_>>(),
        &context,
    )?;
    let projection = ideogram4_project_taps(&backend, &taps, &context)?;
    assert_eq!(projection.descriptor().shape(), [1, 2, 26]);
    let values = tensor_to_f32(&backend, &projection, &context)?;
    assert_eq!(
        &values[..13],
        &[
            0.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 28.0, 32.0, 36.0, 40.0, 44.0, 48.0
        ]
    );
    assert_eq!(
        &values[13..26],
        &[
            1.0, 5.0, 9.0, 13.0, 17.0, 21.0, 25.0, 29.0, 33.0, 37.0, 41.0, 45.0, 49.0
        ]
    );
    drop(values);
    assert_eq!(context.scratch.in_use_bytes(), 0);
    Ok(())
}

#[test]
fn ovis_sam3_and_prompt_adapters_preserve_source_semantics() -> Result<(), Box<dyn Error>> {
    assert_eq!(ovis_template_end(&[10, 4004, 25, 99])?, 2);
    assert!(matches!(
        ovis_template_end(&[10, 11]),
        Err(MultimodalTextError::InvalidInput(_))
    ));
    let (backend, authority) = backend()?;
    let cancellation = CancellationToken::default();
    let context = context(&authority, &cancellation, 8 * 1024 * 1024)?;
    let conditioning = tensor(&backend, &[1, 4, 1], &[1.0, 2.0, 3.0, 4.0], &context)?;
    let trimmed = trim_ovis_conditioning(&conditioning, 2, &cancellation)?;
    assert_eq!(trimmed.descriptor().shape(), [1, 2, 1]);
    assert_eq!(&*tensor_to_f32(&backend, &trimmed, &context)?, &[3.0, 4.0]);

    let prompts = parse_sam3_prompts("(cat): 2.5, dog: 0, red car")?;
    assert_eq!(prompts.len(), 3);
    assert_eq!(
        (prompts[0].text.as_str(), prompts[0].maximum_detections),
        ("cat", 2)
    );
    assert_eq!(
        (prompts[1].text.as_str(), prompts[1].maximum_detections),
        ("dog", 1)
    );
    assert_eq!(
        (prompts[2].text.as_str(), prompts[2].maximum_detections),
        ("red car", 1)
    );
    assert!(matches!(
        parse_sam3_prompts("cat:1.2.3"),
        Err(MultimodalTextError::InvalidPromptLimit(_))
    ));
    let condition = tensor(&backend, &[1, 1, 1], &[7.0], &context)?;
    let pack = pack_sam3_conditions(
        vec![Sam3EncodedCondition {
            condition,
            attention_mask: None,
            maximum_detections: 2,
        }],
        None,
    )?;
    assert_eq!(pack.main_condition.descriptor().shape(), [1, 1, 1]);
    assert_eq!(pack.conditions.len(), 1);

    assert_eq!(
        format_ideogram4_prompt("cat"),
        "<|im_start|>user\ncat<|im_end|>\n<|im_start|>assistant\n"
    );
    assert!(format_ovis_prompt("cat").contains("Describe the image"));
    let qwen = format_qwen3vl_prompt("cat", 2, false);
    assert_eq!(
        qwen,
        "<|im_start|>user\n<|vision_start|><|image_pad|><|vision_end|><|vision_start|><|image_pad|><|vision_end|>cat<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
    );
    assert_eq!(
        format_qwen3vl_prompt("cat", 0, true),
        "<|im_start|>user\ncat<|im_end|>\n<|im_start|>assistant\n"
    );
    assert_eq!(
        format_qwen3vl_prompt("<|im_start|>raw", 3, false),
        "<|im_start|>raw"
    );
    Ok(())
}

#[test]
fn cancellation_and_oom_reject_before_multimodal_publication() -> Result<(), Box<dyn Error>> {
    let (backend, authority) = backend()?;
    let live = CancellationToken::default();
    let setup = context(&authority, &live, 8 * 1024 * 1024)?;
    let text = tensor(&backend, &[1, 2, 1], &[1.0, 2.0], &setup)?;
    let image = tensor(&backend, &[1, 1], &[3.0], &setup)?;
    drop(setup);
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    let cancelled_context = context(&authority, &cancelled, 8 * 1024 * 1024)?;
    let images = [MultimodalImageEmbedding {
        span: MultimodalSpan {
            start: 1,
            size: 1,
            grid_thw: [1, 2, 2],
        },
        embedding: &image,
        deepstack: &[],
    }];
    assert!(matches!(
        join_multimodal_embeddings(&backend, &text, &images, &cancelled_context),
        Err(MultimodalTextError::Cancelled)
    ));
    assert_eq!(cancelled_context.scratch.in_use_bytes(), 0);

    let (oom_backend, oom_authority) = CpuWorkspaceAuthority::create_backend(32)?;
    let live = CancellationToken::default();
    let oom_context = context(&oom_authority, &live, 0)?;
    let oom_text = tensor(&oom_backend, &[1, 2, 1], &[1.0, 2.0], &oom_context)?;
    let oom_image = tensor(&oom_backend, &[1, 1], &[3.0], &oom_context)?;
    let oom_images = [MultimodalImageEmbedding {
        span: MultimodalSpan {
            start: 1,
            size: 1,
            grid_thw: [1, 2, 2],
        },
        embedding: &oom_image,
        deepstack: &[],
    }];
    let error = join_multimodal_embeddings(&oom_backend, &oom_text, &oom_images, &oom_context)
        .expect_err("backend memory exhaustion must reject multimodal publication");
    assert!(matches!(&error, MultimodalTextError::ShapeLayoutTwo(_)));
    assert!(error.to_string().contains("backend limit is 32 bytes"));
    assert_eq!(oom_context.scratch.in_use_bytes(), 0);
    Ok(())
}

#[test]
fn val_clip_001_multimodal_rows_execute_and_extend_cumulative_ledger() -> Result<(), Box<dyn Error>>
{
    let workspace = workspace()?;
    let catalog = fs::read_to_string(
        workspace.join(".agents/specs/comfy-parity/catalogs/backend-conditioning-contracts.csv"),
    )?;
    let mut contracts = Vec::new();
    let mut symbols = Vec::new();
    for line in catalog.lines().skip(1) {
        let fields = line.split(',').collect::<Vec<_>>();
        if fields.get(8).copied() != Some(TASK_ID) {
            continue;
        }
        assert_eq!(fields.len(), 15);
        assert_eq!(fields[7], "comfy_model::clip_text_encoder_multimodal");
        assert_eq!(fields[9], "VAL-CLIP-001");
        assert_eq!(fields[10], "native_rust");
        let source = fs::read(workspace.join(fields[2]))?;
        assert_eq!(format!("{:x}", Sha256::digest(&source)), fields[5]);
        assert_eq!(python_symbol_sha256(&source, fields[3])?, fields[6]);
        execute_contract_witness(fields[2], fields[3])?;
        symbols.push(fields[3]);
        contracts.push(json!({
            "contract_id": fields[0],
            "task_id": TASK_ID,
            "source_sha256": fields[5],
            "symbol_sha256": fields[6],
            "status": "passed",
            "case_ids": [
                format!("{}:native-valid", fields[0]),
                format!("{}:typed-invalid", fields[0]),
            ],
        }));
    }
    assert_eq!(symbols, MULTIMODAL_TEXT_ENCODER_CATALOG_SYMBOLS);
    assert_eq!(contracts.len(), 53);

    let artifact_path = workspace.join("target/comfy-parity/val-clip-001.json");
    let mut artifact = if artifact_path.exists() {
        serde_json::from_slice::<Value>(&fs::read(&artifact_path)?)?
    } else {
        empty_clip_artifact()
    };
    assert_eq!(artifact.get("schema_version"), Some(&json!(1)));
    assert_eq!(artifact.get("validation_id"), Some(&json!("VAL-CLIP-001")));
    let implementations = IMPLEMENTATION_CLOSURE
        .iter()
        .map(|path| {
            Ok(json!({
                "path": path,
                "sha256": format!("{:x}", Sha256::digest(fs::read(workspace.join(path))?)),
            }))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let task_results = artifact
        .get_mut("task_results")
        .and_then(Value::as_object_mut)
        .ok_or("VAL-CLIP-001 task results are missing")?;
    task_results.insert(
        TASK_ID.to_owned(),
        json!({
            "status": "passed",
            "passed": 53,
            "failed": 0,
            "skipped": 0,
            "case_ids": [
                "text-encoder-multimodal:source-provenance-and-exact-row-closure",
                "text-encoder-multimodal:canonical-text-and-vision-delegation",
                "text-encoder-multimodal:position-and-projection-semantics",
                "text-encoder-multimodal:typed-target-cancellation-oom-workspace",
            ],
            "implementations": implementations,
        }),
    );
    let composite_completed = task_results.contains_key(COMPOSITE_TASK_ID);
    let passed = task_results.values().try_fold(0_u64, |total, result| {
        total
            .checked_add(
                result
                    .get("passed")
                    .and_then(Value::as_u64)
                    .ok_or("passed count")?,
            )
            .ok_or("passed count overflowed")
    })?;
    let artifact_contracts = artifact
        .get_mut("contracts")
        .and_then(Value::as_array_mut)
        .ok_or("VAL-CLIP-001 contracts are missing")?;
    artifact_contracts
        .retain(|contract| contract.get("task_id").and_then(Value::as_str) != Some(TASK_ID));
    artifact_contracts.extend(contracts);
    artifact["summary"] = json!({"passed": passed, "failed": 0, "skipped": 0});
    let remaining = artifact
        .get_mut("remaining_tasks")
        .and_then(Value::as_array_mut)
        .ok_or("VAL-CLIP-001 remaining tasks are missing")?;
    remaining.retain(|task| task.as_str() != Some(TASK_ID));
    if !composite_completed
        && !remaining
            .iter()
            .any(|task| task.as_str() == Some(COMPOSITE_TASK_ID))
    {
        let insertion = remaining
            .iter()
            .position(|task| task.as_str() == Some("comfy-parity-clip-text-encoder-breadth"))
            .unwrap_or(remaining.len());
        remaining.insert(insertion, json!(COMPOSITE_TASK_ID));
    }
    let producer_path = "crates/comfy_model/tests/clip_text_encoder_multimodal.rs";
    artifact["implementation"] = json!({
        "path": producer_path,
        "sha256": format!("{:x}", Sha256::digest(fs::read(workspace.join(producer_path))?)),
    });
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&artifact_path, serde_json::to_vec_pretty(&artifact)?)?;
    Ok(())
}

fn execute_contract_witness(source_path: &str, symbol: &str) -> Result<(), Box<dyn Error>> {
    let behavior = multimodal_symbol_behavior(source_path, symbol)
        .ok_or_else(|| format!("{source_path}:{symbol} has no native behavior"))?;
    match behavior {
        MultimodalSymbolBehavior::Profile => {
            assert_eq!(
                multimodal_profile(MultimodalFamily::JinaClip2).layer_count,
                24
            );
        }
        MultimodalSymbolBehavior::ProfileFactory => {
            assert_eq!(
                multimodal_profile(MultimodalFamily::Qwen3Vl8B).hidden_size,
                4096
            );
        }
        MultimodalSymbolBehavior::TokenizerAdapter => {
            assert!(!format_ideogram4_prompt("fixture").is_empty());
        }
        MultimodalSymbolBehavior::PositionConstruction => {
            assert!(qwen2vl_mrope_position_ids(1, &[], &CancellationToken::default())?.is_none());
        }
        MultimodalSymbolBehavior::PromptPacking => {
            assert_eq!(parse_sam3_prompts("fixture")?.len(), 1);
        }
        MultimodalSymbolBehavior::BidirectionalTextDelegation
        | MultimodalSymbolBehavior::DecoderTextDelegation
        | MultimodalSymbolBehavior::ClipTextDelegation
        | MultimodalSymbolBehavior::VisionDelegation
        | MultimodalSymbolBehavior::ImagePreprocessDelegation
        | MultimodalSymbolBehavior::ModalityJoin
        | MultimodalSymbolBehavior::Projection
        | MultimodalSymbolBehavior::ModelAdapter => {}
    }
    assert!(multimodal_symbol_behavior(source_path, "__typed_invalid__").is_none());
    Ok(())
}

fn workspace() -> Result<&'static Path, Box<dyn Error>> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "workspace root is unavailable".into())
}

fn empty_clip_artifact() -> Value {
    json!({
        "schema_version": 1,
        "validation_id": "VAL-CLIP-001",
        "overall_status": "partial",
        "environment": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "backend": "comfy_tensor::CpuBackend",
            "device": "cpu",
            "dtype": "f32",
        },
        "summary": {"passed": 0, "failed": 0, "skipped": 0},
        "implementation": {},
        "task_results": {},
        "contracts": [],
        "remaining_tasks": [
            "comfy-parity-clip-text-encoder-multimodal-foundation",
            "comfy-parity-clip-text-encoder-composite-adapters",
            "comfy-parity-clip-text-encoder-breadth",
            "comfy-parity-clip-owner-consolidation"
        ],
    })
}

fn python_symbol_sha256(source: &[u8], symbol: &str) -> Result<String, Box<dyn Error>> {
    let source = std::str::from_utf8(source)?;
    let lines = source.split_inclusive('\n').collect::<Vec<_>>();
    let function_prefix = format!("def {symbol}(");
    let class_with_bases_prefix = format!("class {symbol}(");
    let class_without_bases_prefix = format!("class {symbol}:");
    let start = lines
        .iter()
        .position(|line| {
            line.starts_with(&function_prefix)
                || line.starts_with(&class_with_bases_prefix)
                || line.starts_with(&class_without_bases_prefix)
        })
        .ok_or_else(|| format!("Python symbol {symbol:?} is missing"))?;
    let mut header_complete = lines[start].trim_end().ends_with(':');
    let mut saw_indented_body = false;
    let mut end = lines.len();
    for (index, line) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = line.trim();
        if !header_complete {
            if !line.starts_with(char::is_whitespace) && trimmed.ends_with(':') {
                header_complete = true;
            }
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            saw_indented_body = true;
        } else if saw_indented_body {
            end = index;
            break;
        }
    }
    let mut body_end = end;
    while body_end > start + 1 {
        let trimmed = lines[body_end - 1].trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            body_end -= 1;
        } else {
            break;
        }
    }
    Ok(format!(
        "{:x}",
        Sha256::digest(lines[start..body_end].concat().as_bytes())
    ))
}
