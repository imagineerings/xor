use comfy_media::{PngLimits, encode_png_frame};
use comfy_model::ModelTokenizerDescriptor;
use comfy_model::clip::NativeTokenizer;
use comfy_model::generated_native_diffusion::{
    Sd1Tokenizer, empty_sd15_latent, encode_sd15_prompt,
};
use comfy_runtime::{NativeDiffusionBundle, NativeDiffusionProvider, Sd15GuidanceAdapter};
use comfy_sampler::NoiseRequest;
use comfy_sampler::generated_native_diffusion::{
    checked_native_diffusion_plan, normal_noise, normal_sigmas, sample_euler, scale_initial_noise,
    scale_model_input, sd15_interpret_prediction, sd15_model_time,
};
use comfy_tensor::{
    CancellationToken, CpuBackend, CpuWorkspaceAuthority, DeviceId, ExecutionContext,
    RngCheckpoint, StreamId,
    generated_native_diffusion::{tensor_from_f32, tensor_to_f32},
};
use comfy_test_support::{NativeDiffusionFixture, NativeDiffusionFixtureError};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

const MEMORY_LIMIT: u64 = 2 * 1024 * 1024 * 1024;
const SEED: u64 = 0x0123_4567_89ab_cdef;
const FIXTURE_PROMPT_ID: &str = "53494d00-0000-0000-0000-000000003702";
const FIXTURE_KSAMPLER_NODE_ID: &str = "5";

#[test]
fn native_diffusion_fixture_catalog_and_provenance_are_exact()
-> Result<(), Box<dyn std::error::Error>> {
    let workspace = workspace()?;
    let fixture = NativeDiffusionFixture::checked_in();
    let catalog: Value = serde_json::from_slice(&fs::read(
        workspace.join(".agents/specs/comfy-parity/catalogs/native-diffusion-fixture.json"),
    )?)?;
    let required = catalog
        .get("required_checkpoints")
        .and_then(Value::as_array)
        .ok_or("native diffusion catalog has no required checkpoints")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or("native diffusion checkpoint name is not a string")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let actual = fs::read_dir(fixture.root())?
        .map(|entry| {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                return Err(std::io::Error::other(format!(
                    "native diffusion fixture contains non-file entry {:?}",
                    entry.path()
                )));
            }
            entry.file_name().into_string().map_err(|name| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("native diffusion fixture name is not UTF-8: {name:?}"),
                )
            })
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    assert_eq!(actual, required);

    assert_eq!(
        catalog
            .pointer("/source_family/feature_id")
            .and_then(Value::as_str),
        Some("COMFY-MODEL-0117")
    );
    assert_eq!(
        catalog
            .pointer("/algorithm_features/latent")
            .and_then(Value::as_str),
        Some("COMFY-MODEL-0045")
    );
    assert_eq!(
        catalog
            .pointer("/algorithm_features/sampler")
            .and_then(Value::as_str),
        Some("COMFY-MODEL-0179")
    );
    assert_eq!(
        catalog
            .pointer("/algorithm_features/scheduler")
            .and_then(Value::as_str),
        Some("COMFY-MODEL-0209")
    );
    assert_eq!(
        digest(&fixture.root().join("vocab.json"))?,
        catalog
            .pointer("/tokenizer/vocab_sha256")
            .and_then(Value::as_str)
            .ok_or("native diffusion catalog has no vocabulary digest")?
    );
    assert_eq!(
        digest(&fixture.root().join("merges.txt"))?,
        catalog
            .pointer("/tokenizer/merges_sha256")
            .and_then(Value::as_str)
            .ok_or("native diffusion catalog has no merges digest")?
    );

    let provenance: Value = serde_json::from_slice(&fixture.read("oracle-provenance.json")?)?;
    assert_eq!(
        provenance
            .get("production_dependency")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        provenance.get("oracle_kind").and_then(Value::as_str),
        Some("pinned-comfyui-source-equation-fixture")
    );
    let sources = provenance
        .get("sources")
        .and_then(Value::as_object)
        .ok_or("native diffusion provenance has no sources")?;
    for (source, expected_digest) in sources {
        assert_eq!(
            digest(&workspace.join(source))?,
            expected_digest
                .as_str()
                .ok_or("native diffusion source digest is not a string")?,
            "stale native diffusion source provenance for {source}"
        );
    }
    assert_eq!(
        digest(&workspace.join("projects/comfy/ComfyUI/comfy/sd1_clip_config.json"))?,
        catalog
            .pointer("/clip/source_config_sha256")
            .and_then(Value::as_str)
            .ok_or("native diffusion catalog has no CLIP config digest")?
    );
    Ok(())
}

#[test]
fn native_diffusion_fixture_rejects_tampering_before_weight_parsing()
-> Result<(), Box<dyn std::error::Error>> {
    let checked_in = NativeDiffusionFixture::checked_in();
    let cancellation = CancellationToken::default();

    let invalid_key_directory = tempfile::tempdir()?;
    copy_fixture_admission_files(&checked_in, invalid_key_directory.path())?;
    copy_model_with_replacement(
        &checked_in,
        invalid_key_directory.path(),
        b"model.diffusion_model.input_blocks.0.0.weight",
        b"model.diffusion_model.input_blocks.0.0.weighx",
    )?;
    let invalid_key_error = NativeDiffusionFixture::at(invalid_key_directory.path())
        .load_model(MEMORY_LIMIT, &cancellation)
        .expect_err("tampered weight key must fail model admission");
    assert!(
        matches!(
            &invalid_key_error,
            NativeDiffusionFixtureError::ModelDigestMismatch { .. }
        ),
        "unexpected invalid-key error: {invalid_key_error:?}"
    );

    let invalid_shape_directory = tempfile::tempdir()?;
    copy_fixture_admission_files(&checked_in, invalid_shape_directory.path())?;
    copy_model_with_replacement(
        &checked_in,
        invalid_shape_directory.path(),
        b"\"shape\":[32,4,3,3]",
        b"\"shape\":[16,8,3,3]",
    )?;
    let invalid_shape_error = NativeDiffusionFixture::at(invalid_shape_directory.path())
        .load_model(MEMORY_LIMIT, &cancellation)
        .expect_err("tampered weight shape must fail model admission");
    assert!(
        matches!(
            &invalid_shape_error,
            NativeDiffusionFixtureError::ModelDigestMismatch { .. }
        ),
        "unexpected invalid-shape error: {invalid_shape_error:?}"
    );
    Ok(())
}

#[test]
fn canonical_clip_load_is_failure_atomic_and_workspace_bounded()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = NativeDiffusionFixture::checked_in();
    let cancellation = CancellationToken::default();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
    let backend = Arc::new(backend);
    let workspace = workspace_authority.authorize_workspace(1024 * 1024)?;
    let context = backend.execution_context(StreamId::DEFAULT, workspace.clone(), &cancellation);
    assert!(
        fixture
            .load_clip_with_context(backend.clone(), &context)
            .is_err()
    );
    assert_eq!(workspace.in_use_bytes(), 0);
    assert_eq!(backend.memory_snapshot().current_bytes, 0);
    Ok(())
}

#[test]
fn native_diffusion_fixture_matches_all_checkpoints() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = NativeDiffusionFixture::checked_in();
    let cancellation = CancellationToken::default();
    let tokenizer = fixture.tokenizer()?;
    let positive_tokens = encode_sd15_prompt(&tokenizer, "a test", &cancellation)?;
    let negative_tokens = encode_sd15_prompt(&tokenizer, "", &cancellation)?;
    assert_eq!(&positive_tokens[..4], &[49_406, 320, 1_628, 49_407]);
    assert_eq!(negative_tokens[0], 49_406);
    assert!(negative_tokens[1..].iter().all(|token| *token == 49_407));

    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let backend = Arc::new(backend);
    let workspace = workspace_authority.authorize_workspace(MEMORY_LIMIT)?;
    let context = backend.execution_context(StreamId::DEFAULT, workspace.clone(), &cancellation);
    let cache_identities = fixture.cache_identities(context.cancellation)?;
    assert_eq!(
        cache_identities.tokenizer_digest(),
        tokenizer.identity().digest()
    );
    let bundle = fixture.load_bundle_with_context(backend.clone(), &context)?;
    assert_eq!(&cache_identities, bundle.cache_identities());
    let model = bundle.model().clone();
    let model_digest = cache_identities.model_digest().to_owned();
    let vocabulary = String::from_utf8(fixture.read("vocab.json")?)?;
    let merges = String::from_utf8(fixture.read("merges.txt")?)?;
    let wrong_descriptor = Sd1Tokenizer::from_json_and_merges(
        ModelTokenizerDescriptor::checked("comfy.sd2.tokenizer")?,
        &vocabulary,
        &merges,
    )?;
    assert!(
        NativeDiffusionBundle::new_with_vae(
            "sd15-tiny-v1",
            model_digest.clone(),
            model.clone(),
            Arc::new(wrong_descriptor),
            bundle.clip().clone(),
            bundle.vae().clone(),
        )
        .is_err()
    );
    let mut alternate_vocabulary = serde_json::from_str::<BTreeMap<String, u32>>(&vocabulary)?;
    let ordinary_keys = alternate_vocabulary
        .iter()
        .filter(|(_, token)| **token < comfy_model::clip::SD1_START_TOKEN)
        .take(2)
        .map(|(piece, _)| piece.clone())
        .collect::<Vec<_>>();
    let [first_key, second_key] = ordinary_keys.as_slice() else {
        return Err("SD1 vocabulary has fewer than two ordinary tokens".into());
    };
    let first_value = *alternate_vocabulary
        .get(first_key)
        .ok_or("first ordinary SD1 token disappeared")?;
    let second_value = *alternate_vocabulary
        .get(second_key)
        .ok_or("second ordinary SD1 token disappeared")?;
    alternate_vocabulary.insert(first_key.clone(), second_value);
    alternate_vocabulary.insert(second_key.clone(), first_value);
    let alternate_vocabulary = serde_json::to_string(&alternate_vocabulary)?;
    let alternate_tokenizer = Sd1Tokenizer::from_json_and_merges(
        ModelTokenizerDescriptor::checked("comfy.sd1.tokenizer")?,
        &alternate_vocabulary,
        &merges,
    )?;
    assert_ne!(
        alternate_tokenizer.identity().digest(),
        cache_identities.tokenizer_digest()
    );
    assert!(
        NativeDiffusionBundle::new_with_vae(
            "sd15-tiny-v1",
            model_digest,
            model.clone(),
            Arc::new(alternate_tokenizer),
            bundle.clip().clone(),
            bundle.vae().clone(),
        )
        .is_err()
    );
    let (canonical_positive_tokens, positive) = bundle.encode_text("a test", &context)?;
    let (canonical_negative_tokens, negative) = bundle.encode_text("", &context)?;
    assert_eq!(canonical_positive_tokens, positive_tokens);
    assert_eq!(canonical_negative_tokens, negative_tokens);
    assert_tensor_file(
        fixture.root().join("clip-conditioning.safetensors"),
        "positive",
        &positive,
        &backend,
        &context,
    )?;
    assert_tensor_file(
        fixture.root().join("clip-conditioning.safetensors"),
        "negative",
        &negative,
        &backend,
        &context,
    )?;

    let sigmas = normal_sigmas(&backend, &context, 4, 1.0)?;
    let sigma_bytes = fs::read(fixture.root().join("normal-sigmas.f64le"))?;
    let expected_sigmas = sigma_bytes
        .chunks_exact(8)
        .map(|chunk| {
            let encoded: [u8; 8] = chunk.try_into().map_err(|_| "invalid sigma fixture")?;
            Ok(f64::from_le_bytes(encoded))
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    assert_eq!(
        expected_sigmas,
        sigmas
            .iter()
            .map(|value| f64::from(*value))
            .collect::<Vec<_>>()
    );

    let stream = NoiseRequest::native_diffusion(FIXTURE_PROMPT_ID, FIXTURE_KSAMPLER_NODE_ID)?
        .stream(SEED, DeviceId::CPU)?;
    let noise = normal_noise(&backend, &[1, 4, 4, 4], &stream, &context)?;
    let expected_checkpoint: RngCheckpoint = serde_json::from_slice(&fs::read(
        fixture.root().join("rng-state-before-noise.bin"),
    )?)?;
    assert_eq!(noise.before, expected_checkpoint);
    assert_tensor_file(
        fixture.root().join("initial-noise.safetensors"),
        "noise",
        &noise.noise,
        &backend,
        &context,
    )?;
    let latent = empty_sd15_latent(&backend, 1, 32, 32, &context)?;
    let initial = scale_initial_noise(&backend, &noise.noise, &latent, sigmas[0], &context)?;
    let plan = checked_native_diffusion_plan("euler", "normal", SEED, 4, 7.0, 1.0)?;
    let mut guidance = Sd15GuidanceAdapter::checked(&model, &positive, &negative, &context)?;
    let trace = sample_euler(
        &backend,
        initial,
        &sigmas,
        &context,
        |latent, sigma, _step| {
            let model_input = scale_model_input(&backend, latent, sigma, &context)
                .map_err(|error| error.to_string())?;
            let prediction = guidance
                .execute(&backend, &model_input, sigma, &plan, &context)
                .map_err(|error| error.to_string())?;
            sd15_interpret_prediction(&backend, prediction.guided(), latent, sigma, &context)
                .map_err(|error| error.to_string())
        },
    )?;
    assert_eq!(trace.denoiser_evaluations.len(), 4);
    assert_eq!(trace.latents.len(), 5);
    for (index, denoised) in trace.denoiser_evaluations.iter().enumerate() {
        assert_tensor_file(
            fixture
                .root()
                .join(format!("denoiser-eval-{index:03}.safetensors")),
            "denoised",
            denoised,
            &backend,
            &context,
        )?;
    }
    for (index, latent) in trace.latents.iter().enumerate() {
        assert_tensor_file(
            fixture
                .root()
                .join(format!("latent-step-{index:03}.safetensors")),
            "latent",
            latent,
            &backend,
            &context,
        )?;
    }
    let decoded = bundle.vae().decode(
        backend.as_ref(),
        trace.latents.last().ok_or("missing final latent")?,
        &context,
    )?;
    let decoded_values = tensor_to_f32(&backend, &decoded, &context)?;
    let expected_decoded = fs::read(fixture.root().join("vae-decoded.f32le"))?;
    assert_eq!(f32_bytes(&decoded_values), expected_decoded);
    drop(decoded);
    let bhwc = nchw_to_bhwc(&decoded_values, 3, 32, 32)?;
    let png = encode_png_frame(
        &bhwc,
        1,
        32,
        32,
        3,
        0,
        &BTreeMap::new(),
        PngLimits::default(),
    )?;
    assert_eq!(png, fs::read(fixture.root().join("output.png"))?);

    let workspace_before_cancel = workspace.in_use_bytes();
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    let cancelled_context =
        backend.execution_context(StreamId::DEFAULT, workspace.clone(), &cancelled);
    assert!(bundle.encode_text("a test", &cancelled_context).is_err());
    assert!(
        fixture
            .load_model(1024, &CancellationToken::default())
            .is_err()
    );
    assert_eq!(workspace.in_use_bytes(), workspace_before_cancel);
    Ok(())
}

#[test]
fn native_diffusion_guidance_adapter_preserves_cfg_and_failure_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let cancellation = CancellationToken::default();
    let fixture = NativeDiffusionFixture::checked_in();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let backend = Arc::new(backend);
    let workspace = workspace_authority.authorize_workspace(MEMORY_LIMIT)?;
    let context = backend.execution_context(StreamId::DEFAULT, workspace, &cancellation);
    let bundle = fixture.load_bundle_with_context(backend.clone(), &context)?;
    let model = bundle.model();
    let (_, positive) = bundle.encode_text("a test", &context)?;
    let (_, negative) = bundle.encode_text("", &context)?;
    let latent = tensor_from_f32(&backend, &[1, 4, 4, 4], &[0.0; 64], &context)?;
    let sigma = normal_sigmas(&backend, &context, 4, 1.0)?[0];
    let model_time = sd15_model_time(sigma)?;

    let scale_one = checked_native_diffusion_plan("euler", "normal", SEED, 4, 1.0, 1.0)?;
    let mut adapter = Sd15GuidanceAdapter::checked(model.as_ref(), &positive, &negative, &context)?;
    let guided = adapter.execute(&backend, &latent, sigma, &scale_one, &context)?;
    let conditional = model.denoise_at_model_time(&latent, model_time, &positive, &context)?;
    assert!(guided.unconditional_skipped());
    assert_eq!(guided.denoiser_evaluations(), 1);
    assert_eq!(guided.guided().descriptor(), latent.descriptor());
    let guided_values = tensor_to_f32(&backend, guided.guided(), &context)?;
    let conditional_values = tensor_to_f32(&backend, &conditional, &context)?;
    assert_eq!(&guided_values[..], &conditional_values[..]);
    drop(conditional_values);
    drop(guided_values);

    let scale_zero = checked_native_diffusion_plan("euler", "normal", SEED, 4, 0.0, 1.0)?;
    let guided = adapter.execute(&backend, &latent, sigma, &scale_zero, &context)?;
    let unconditional = model.denoise_at_model_time(&latent, model_time, &negative, &context)?;
    assert!(!guided.unconditional_skipped());
    assert_eq!(guided.denoiser_evaluations(), 2);
    let guided_values = tensor_to_f32(&backend, guided.guided(), &context)?;
    let unconditional_values = tensor_to_f32(&backend, &unconditional, &context)?;
    assert_eq!(&guided_values[..], &unconditional_values[..]);
    drop(unconditional_values);
    drop(guided_values);

    let wrong_shape = tensor_from_f32(&backend, &[1, 77, 31], &[0.0; 77 * 31], &context)?;
    let mut wrong = Sd15GuidanceAdapter::checked(&model, &wrong_shape, &negative, &context)?;
    assert!(matches!(
        wrong.execute(&backend, &latent, sigma, &scale_zero, &context),
        Err(comfy_runtime::NativeImageRuntimeError::Execution(message))
            if message.contains("SD15 conditioning")
    ));

    let cancelled = CancellationToken::default();
    cancelled.cancel();
    let cancelled_context =
        backend.execution_context(StreamId::DEFAULT, context.scratch.clone(), &cancelled);
    assert!(matches!(
        Sd15GuidanceAdapter::checked(&model, &positive, &negative, &cancelled_context),
        Err(comfy_runtime::NativeImageRuntimeError::Cancelled)
    ));
    assert_eq!(cancelled_context.scratch.in_use_bytes(), 0);

    let constrained_workspace = workspace_authority.authorize_workspace(64)?;
    let constrained_context = backend.execution_context(
        StreamId::DEFAULT,
        constrained_workspace.clone(),
        &cancellation,
    );
    let mut constrained =
        Sd15GuidanceAdapter::checked(&model, &positive, &negative, &constrained_context)?;
    let constrained_error = constrained
        .execute(&backend, &latent, sigma, &scale_zero, &constrained_context)
        .expect_err("undersized workspace authorization must reject guidance");
    assert!(
        matches!(
            &constrained_error,
            comfy_runtime::NativeImageRuntimeError::ResourceExhausted(message)
                if message.contains("workspace request of")
                    && message.contains("64-byte authorization")
        ),
        "unexpected constrained-workspace error: {constrained_error:?}"
    );
    assert_eq!(constrained_workspace.in_use_bytes(), 0);
    assert_eq!(context.scratch.in_use_bytes(), 0);
    Ok(())
}

fn assert_tensor_file(
    path: impl AsRef<Path>,
    name: &str,
    actual: &comfy_tensor::Tensor,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let header_length_bytes: [u8; 8] = bytes
        .get(..8)
        .ok_or("missing safetensors header")?
        .try_into()?;
    let header_length = usize::try_from(u64::from_le_bytes(header_length_bytes))?;
    let data_start = 8_usize
        .checked_add(header_length)
        .ok_or("safetensors offset overflow")?;
    let header: Value = serde_json::from_slice(
        bytes
            .get(8..data_start)
            .ok_or("truncated safetensors header")?,
    )?;
    let descriptor = header.get(name).ok_or("missing tensor descriptor")?;
    assert_eq!(descriptor.get("dtype").and_then(Value::as_str), Some("F32"));
    assert_eq!(
        descriptor.get("shape"),
        Some(&serde_json::to_value(actual.descriptor().shape())?)
    );
    let offsets = descriptor
        .get("data_offsets")
        .and_then(Value::as_array)
        .ok_or("missing offsets")?;
    let start = usize::try_from(
        offsets
            .first()
            .and_then(Value::as_u64)
            .ok_or("missing start")?,
    )?;
    let end = usize::try_from(
        offsets
            .get(1)
            .and_then(Value::as_u64)
            .ok_or("missing end")?,
    )?;
    let expected = bytes
        .get(data_start + start..data_start + end)
        .ok_or("truncated tensor data")?;
    let actual_values = tensor_to_f32(backend, actual, context)?;
    assert_eq!(f32_bytes(&actual_values), expected);
    Ok(())
}

fn f32_bytes(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn nchw_to_bhwc(
    values: &[f32],
    channels: usize,
    height: usize,
    width: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if values.len() != channels * height * width {
        return Err("decoded tensor shape is invalid".into());
    }
    let mut output = vec![0.0; values.len()];
    for y in 0..height {
        for x in 0..width {
            for channel in 0..channels {
                output[(y * width + x) * channels + channel] =
                    values[(channel * height + y) * width + x];
            }
        }
    }
    Ok(output)
}

fn workspace() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root is unavailable")?
        .to_path_buf())
}

fn digest(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn copy_fixture_admission_files(
    fixture: &NativeDiffusionFixture,
    destination: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::copy(
        fixture.root().join("sd15-detector-projection.json"),
        destination.join("sd15-detector-projection.json"),
    )?;
    Ok(())
}

fn copy_model_with_replacement(
    fixture: &NativeDiffusionFixture,
    destination: &Path,
    needle: &[u8],
    replacement: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    if needle.len() != replacement.len() {
        return Err("native diffusion test replacement changes safetensors length".into());
    }
    let mut bytes = fixture.read("model.safetensors")?;
    let matches = bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, candidate)| (candidate == needle).then_some(index))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "native diffusion test replacement expected one match, found {}",
            matches.len()
        )
        .into());
    }
    let start = matches[0];
    let end = start
        .checked_add(replacement.len())
        .ok_or("native diffusion test replacement overflowed")?;
    bytes
        .get_mut(start..end)
        .ok_or("native diffusion test replacement is out of bounds")?
        .copy_from_slice(replacement);
    fs::write(destination.join("model.safetensors"), bytes)?;
    Ok(())
}
