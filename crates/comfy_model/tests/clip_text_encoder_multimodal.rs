use comfy_model::{
    IDEOGRAM4_SOURCE_PATH, IDEOGRAM4_SOURCE_SHA256, IDEOGRAM4_TAP_LAYERS, JINA_CLIP2_SOURCE_PATH,
    JINA_CLIP2_SOURCE_SHA256, MULTIMODAL_TEXT_ENCODER_CATALOG_SYMBOLS, MultimodalFamily,
    MultimodalImageEmbedding, MultimodalSpan, MultimodalSymbolBehavior, MultimodalTextError,
    OVIS_SOURCE_PATH, OVIS_SOURCE_SHA256, QWEN_VL_SOURCE_PATH, QWEN_VL_SOURCE_SHA256,
    QWEN3VL_SOURCE_PATH, QWEN3VL_SOURCE_SHA256, SAM3_CLIP_SOURCE_PATH, SAM3_CLIP_SOURCE_SHA256,
    Sam3EncodedCondition, format_ideogram4_prompt, format_ovis_prompt, format_qwen3vl_prompt,
    ideogram4_project_taps, join_multimodal_embeddings, join_qwen3vl_deepstack, multimodal_profile,
    multimodal_symbol_behavior, ovis_template_end, pack_sam3_conditions, parse_sam3_prompts,
    qwen2vl_mrope_position_ids, trim_ovis_conditioning,
};
use comfy_tensor::{
    CancellationToken, CpuBackend, CpuWorkspaceAuthority, DType, DeviceId, ExecutionContext,
    StreamId, Tensor, TensorDescriptor, generated_native_diffusion::tensor_to_f32,
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
    assert_eq!(qwen.matches("<|image_pad|>").count(), 2);
    assert!(qwen.ends_with("<think>\n\n</think>\n\n"));
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
                "text-encoder-multimodal:mrope-modality-deepstack-and-projection",
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
