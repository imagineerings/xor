use comfy_media::{PngLimits, encode_png_frame};
use comfy_model::clip::{NativeClipProfile, NativeClipResource, NativeTokenizer};
use comfy_model::generated_native_diffusion::{
    Sd1Tokenizer, empty_sd15_latent, encode_sd15_prompt,
};
use comfy_model::model_family::{
    NativeFamilyDenoiserContext, NativeFamilyDenoiserInvocation, NativeFamilyModelResource,
};
use comfy_model::{
    LatentFormatIdentity, ModelFamilyError, NativeFamilyBuildOptions, NativeFamilyModel,
    PatchGraph, PatchPayload, PatchTensor, PatchValueTransform, SemanticPatchOperation,
    build_model_family,
    conditioning::{
        ConditioningEntry, ConditioningEntryOptions, ConditioningIdentity, ConditioningSet,
        ConditioningValue,
    },
    generated_auraflow_comfy_model_0064 as aura, generated_qwenimage_comfy_model_0113 as qwen,
    map_model_weights,
};
use comfy_model::{ModelTokenizerDescriptor, NativeModelPayload};
use comfy_runtime::{NativeDiffusionBundle, NativeDiffusionProvider, Sd15GuidanceAdapter};
use comfy_sampler::generated_native_diffusion::interpret_prediction_for_profile;
use comfy_sampler::generated_native_diffusion::{
    checked_native_diffusion_plan, checked_native_diffusion_plan_for_profile, normal_noise,
    normal_sigmas, normal_sigmas_for_profile, sample_euler, scale_initial_noise, scale_model_input,
    sd15_interpret_prediction, sd15_model_time,
};
use comfy_sampler::{
    GuidanceOptions, NativeConditioningPayload, NativeDiffusionPayload,
    NativeFamilyGuidanceDenoiser, NativeGuiderPayload, NoiseRequest, profile_for_model,
};
use comfy_tensor::{
    CancellationToken, CpuBackend, CpuWorkspaceAuthority, DType, DeviceId, ExecutionContext,
    RngCheckpoint, StreamId, Tensor,
    generated_native_diffusion::{tensor_from_f32, tensor_to_f32},
};
use comfy_test_support::{NativeDiffusionFixture, NativeDiffusionFixtureError};
use comfy_types::DeviceKind;
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
const FAMILY_ARTIFACT_DIGEST: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn family_patterned_values(key: &str, elements: usize) -> Vec<f32> {
    let normalization = key.contains("norm_") || key.ends_with("txt_norm.weight");
    let bias = key.ends_with(".bias");
    let aura_mlp =
        key.contains(".mlpC.") || key.contains(".mlpX.") || key.contains("single_layers.0.mlp.");
    let scale = if aura_mlp { 0.5 } else { 0.000_75 };
    (0..elements)
        .map(|index| {
            if normalization {
                0.95 + (index % 7) as f32 * 0.01
            } else if bias {
                ((index % 11) as f32 - 5.0) * 0.002
            } else {
                ((index % 17) as f32 - 8.0) * scale
            }
        })
        .collect()
}

fn family_tensor(
    backend: &CpuBackend,
    shape: &[u64],
    key: &str,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, Box<dyn std::error::Error>> {
    let elements = shape.iter().try_fold(1_usize, |total, dimension| {
        total.checked_mul(usize::try_from(*dimension).ok()?)
    });
    let elements = elements.ok_or("family fixture tensor shape overflow")?;
    Ok(tensor_from_f32(
        backend,
        shape,
        &family_patterned_values(key, elements),
        context,
    )?)
}

fn family_weight_shape(
    key: &str,
    qwen_model: bool,
) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    let shape = if qwen_model {
        match key {
            "native.img_in.weight" => vec![128, 64],
            "native.img_in.bias"
            | "native.txt_in.bias"
            | "native.time_text_embed.timestep_embedder.linear_1.bias"
            | "native.time_text_embed.timestep_embedder.linear_2.bias" => vec![128],
            "native.txt_norm.weight" => vec![3_584],
            "native.txt_in.weight" => vec![128, 3_584],
            "native.time_text_embed.timestep_embedder.linear_1.weight" => vec![128, 256],
            "native.time_text_embed.timestep_embedder.linear_2.weight" => vec![128, 128],
            key if key.ends_with("img_mod.1.weight") || key.ends_with("txt_mod.1.weight") => {
                vec![768, 128]
            }
            key if key.ends_with("img_mod.1.bias") || key.ends_with("txt_mod.1.bias") => vec![768],
            key if key.contains("attn.norm_") => vec![128],
            key if key.contains("attn.") && key.ends_with(".weight") => vec![128, 128],
            key if key.contains("attn.") && key.ends_with(".bias") => vec![128],
            key if key.ends_with("mlp.net.0.proj.weight") => vec![512, 128],
            key if key.ends_with("mlp.net.0.proj.bias") => vec![512],
            key if key.ends_with("mlp.net.2.weight") => vec![128, 512],
            key if key.ends_with("mlp.net.2.bias") => vec![128],
            "native.norm_out.linear.weight" => vec![256, 128],
            "native.norm_out.linear.bias" => vec![256],
            "native.proj_out.weight" => vec![64, 128],
            "native.proj_out.bias" => vec![64],
            _ => return Err(format!("missing Qwen fixture shape for {key}").into()),
        }
    } else {
        match key {
            "native.init_x_linear.weight" => vec![2, 16],
            "native.init_x_linear.bias" => vec![2],
            "native.positional_encoding" => vec![1, 16, 2],
            "native.register_tokens" => vec![1, 8, 2],
            "native.cond_seq_linear.weight" => vec![2, 2_048],
            "native.t_embedder.mlp.0.weight" => vec![2, 256],
            "native.t_embedder.mlp.0.bias" | "native.t_embedder.mlp.2.bias" => vec![2],
            "native.t_embedder.mlp.2.weight" => vec![2, 2],
            "native.double_layers.0.modC.1.weight"
            | "native.double_layers.0.modX.1.weight"
            | "native.single_layers.0.modCX.1.weight" => vec![12, 2],
            "native.double_layers.0.attn.w1q.weight"
            | "native.double_layers.0.attn.w1k.weight"
            | "native.double_layers.0.attn.w1v.weight"
            | "native.double_layers.0.attn.w1o.weight"
            | "native.double_layers.0.attn.w2q.weight"
            | "native.double_layers.0.attn.w2k.weight"
            | "native.double_layers.0.attn.w2v.weight"
            | "native.double_layers.0.attn.w2o.weight"
            | "native.single_layers.0.attn.w1q.weight"
            | "native.single_layers.0.attn.w1k.weight"
            | "native.single_layers.0.attn.w1v.weight"
            | "native.single_layers.0.attn.w1o.weight" => vec![2, 2],
            key if key.ends_with("c_fc1.weight") || key.ends_with("c_fc2.weight") => vec![256, 2],
            key if key.ends_with("c_proj.weight") => vec![2, 256],
            "native.modF.1.weight" => vec![4, 2],
            "native.final_linear.weight" => vec![16, 2],
            _ => return Err(format!("missing Aura fixture shape for {key}").into()),
        }
    };
    Ok(shape)
}

fn family_fixture_model(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    qwen_model: bool,
    source_bound: bool,
) -> Result<NativeFamilyModel, Box<dyn std::error::Error>> {
    if source_bound {
        return task377::bound_model(backend, context, qwen_model, MEMORY_LIMIT);
    }
    let (definition, keys): (_, &[&str]) = if qwen_model {
        (&qwen::MODEL_FAMILY, qwen::DENOISER_INVOCATION_REQUIRED_KEYS)
    } else {
        (&aura::MODEL_FAMILY, aura::DENOISER_INVOCATION_REQUIRED_KEYS)
    };
    let mut source = BTreeMap::new();
    for key in keys {
        let shape = family_weight_shape(key, qwen_model)?;
        source.insert(
            key.replacen("native.", "model.diffusion_model.", 1),
            family_tensor(backend, &shape, key, context)?,
        );
    }
    if qwen_model {
        for key in [
            "native.__reference_method__",
            "native.__additional_timestep_condition__",
        ] {
            source.insert(
                key.replacen("native.", "model.diffusion_model.", 1),
                tensor_from_f32(backend, &[1], &[0.0], context)?,
            );
        }
    }
    let options = NativeFamilyBuildOptions {
        dtype: DType::F32,
        device: DeviceKind::Cpu,
        activation_elements: 1,
        memory_budget_bytes: MEMORY_LIMIT,
        allow_unexpected_weights: true,
    };
    Ok(build_model_family(
        definition,
        map_model_weights(definition, FAMILY_ARTIFACT_DIGEST, source)?,
        options,
    )?)
}

fn family_identity(
    model: &NativeFamilyModel,
    namespace: &str,
) -> Result<ConditioningIdentity, Box<dyn std::error::Error>> {
    Ok(ConditioningIdentity::new(
        namespace,
        model.identity()?,
        LatentFormatIdentity::new(
            model.profile().latent_feature_id,
            model.profile().latent_identifier,
        )?,
    )?)
}

fn family_conditioning(
    identity: ConditioningIdentity,
    tensor: Tensor,
    name: &str,
    cancellation: &CancellationToken,
) -> Result<Arc<ConditioningSet>, Box<dyn std::error::Error>> {
    Ok(Arc::new(ConditioningSet::checked(
        identity,
        vec![ConditioningEntry::checked(
            name,
            ConditioningValue::cross_attention(tensor)?,
            ConditioningEntryOptions::default(),
        )?],
        cancellation,
    )?))
}

fn tensor_f32_sha256(
    backend: &CpuBackend,
    tensor: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut digest = Sha256::new();
    for value in tensor_to_f32(backend, tensor, context)?.iter() {
        digest.update(value.to_le_bytes());
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn family_oracle() -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(include_str!(
        "../fixtures/models/native-family-model-resource-foundation/provenance.json"
    ))?)
}

fn decode_hex_nibble(value: u8) -> Result<u8, Box<dyn std::error::Error>> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(format!("invalid family oracle hex nibble {value}").into()),
    }
}

fn family_oracle_raw(
    oracle: &Value,
    family: &str,
    branch: &str,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let encoded = oracle
        .pointer(&format!("/families/{family}/{branch}_raw_f32_le_hex"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("family oracle is missing {family}.{branch} raw values"))?;
    if encoded.len() % 8 != 0 {
        return Err("family oracle raw values are not complete F32 words".into());
    }
    encoded
        .as_bytes()
        .chunks_exact(8)
        .map(|word| {
            let mut bytes = [0_u8; 4];
            for (index, pair) in word.chunks_exact(2).enumerate() {
                bytes[index] = (decode_hex_nibble(pair[0])? << 4) | decode_hex_nibble(pair[1])?;
            }
            Ok(f32::from_le_bytes(bytes))
        })
        .collect()
}

fn assert_tensor_close(
    backend: &CpuBackend,
    actual: &Tensor,
    expected: &[f32],
    context: &ExecutionContext<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let actual = tensor_to_f32(backend, actual, context)?;
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (*actual - *expected).abs() <= 1.0e-6,
            "family resource oracle diverged at {index}: {actual} != {expected}"
        );
    }
    Ok(())
}

mod task377 {
    include!("native_family_execution_projection.rs");

    pub(super) fn bound_model(
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
        qwen_model: bool,
        budget: u64,
    ) -> Result<NativeFamilyModel, Box<dyn Error>> {
        let registry =
            ModelFamilyRegistry::checked_registrations(GENERATED_MODEL_FAMILY_REGISTRATIONS)?;
        let probe = if qwen_model {
            qwen_probe()?
        } else {
            aura_probe()?
        };
        bind(
            &registry,
            &probe,
            projection_state(backend, context, qwen_model)?,
            context,
            budget,
        )
        .map_err(Into::into)
    }

    pub(super) fn assert_raw_oracle(
        model: &NativeFamilyModel,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
        qwen_model: bool,
    ) -> Result<(), Box<dyn Error>> {
        task376::assert_bound_model_oracle(model, backend, context, qwen_model)
    }
}

#[test]
fn family_model_resource_transport_guidance_and_reconstruction_are_canonical()
-> Result<(), Box<dyn std::error::Error>> {
    let cancellation = CancellationToken::default();
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(MEMORY_LIMIT)?,
        &cancellation,
    );

    let legacy = Arc::new(family_fixture_model(&backend, &context, false, false)?);
    let legacy_patch = Arc::new(PatchGraph::checked_semantic(
        FAMILY_ARTIFACT_DIGEST,
        Vec::new(),
    )?);
    assert!(matches!(
        NativeFamilyModelResource::materialize(
            legacy.clone(),
            legacy_patch,
            family_identity(&legacy, "native-family-model")?,
            &backend,
            &context,
        ),
        Err(ModelFamilyError::FamilyResourceSourceBindingMissing)
    ));

    for qwen_model in [false, true] {
        let base = Arc::new(family_fixture_model(&backend, &context, qwen_model, true)?);
        let patch_graph = Arc::new(PatchGraph::checked_semantic(
            FAMILY_ARTIFACT_DIGEST,
            Vec::new(),
        )?);
        let identity = family_identity(&base, "native-family-model")?;
        let resource = Arc::new(NativeFamilyModelResource::materialize(
            base,
            patch_graph.clone(),
            identity,
            &backend,
            &context,
        )?);
        let reconstructed = resource.reconstruct_in_memory(&backend, &context)?;
        assert_eq!(
            reconstructed.semantic_digest_sha256(),
            resource.semantic_digest_sha256()
        );
        assert_eq!(reconstructed.resident_bytes()?, resource.resident_bytes()?);
        assert!(!Arc::ptr_eq(reconstructed.model(), resource.model()));

        let model = Arc::new(NativeModelPayload::native_family_model(resource.clone())?);
        let execution = Arc::new(NativeConditioningPayload::checked_family(
            &model,
            patch_graph,
            None,
        )?);
        let diffusion = NativeDiffusionPayload::model(model.clone(), execution)?;
        diffusion.validate()?;
        assert!(
            diffusion
                .model_resources()
                .is_some_and(|(stored, _)| Arc::ptr_eq(stored, &model))
        );
        assert!(model.resident_parts()?.resident_bytes()? > 0);
        assert!(diffusion.resident_parts()?.resident_bytes()? > 0);
    }

    let base = Arc::new(family_fixture_model(&backend, &context, false, true)?);
    let patch_graph = Arc::new(PatchGraph::checked_semantic(
        FAMILY_ARTIFACT_DIGEST,
        Vec::new(),
    )?);
    let resource = Arc::new(NativeFamilyModelResource::materialize(
        base.clone(),
        patch_graph,
        family_identity(&base, "native-family-model")?,
        &backend,
        &context,
    )?);
    let model = Arc::new(NativeModelPayload::native_family_model(resource)?);
    let latent = tensor_from_f32(
        &backend,
        &[1, 4, 3, 3],
        &(0..36)
            .map(|index| (index as f32 - 18.0) * 0.025)
            .collect::<Vec<_>>(),
        &context,
    )?;
    let positive = family_conditioning(
        family_identity(&base, "aura-positive")?,
        tensor_from_f32(
            &backend,
            &[1, 2, 2_048],
            &family_patterned_values("positive", 2 * 2_048),
            &context,
        )?,
        "positive",
        &cancellation,
    )?;
    let negative = family_conditioning(
        family_identity(&base, "aura-negative")?,
        tensor_from_f32(
            &backend,
            &[1, 2, 2_048],
            &family_patterned_values("negative.bias", 2 * 2_048),
            &context,
        )?,
        "negative",
        &cancellation,
    )?;
    let profile = profile_for_model(&model)?;
    let plan =
        checked_native_diffusion_plan_for_profile(&profile, "euler", "normal", 7, 4, 2.0, 1.0)?;
    let guider = NativeGuiderPayload::cfg(model.clone(), positive, negative, 2.0)?;
    let mut denoiser = NativeFamilyGuidanceDenoiser::checked(&model, &backend)?;
    let guided = guider.execute(
        &backend,
        &latent,
        0.5,
        &profile,
        &plan,
        GuidanceOptions::default(),
        &mut denoiser,
        &mut [],
        &context,
    )?;
    assert_eq!(guided.guided().descriptor(), latent.descriptor());
    assert!(
        tensor_to_f32(&backend, guided.guided(), &context)?
            .iter()
            .all(|value| value.is_finite())
    );

    let wrong_identity = ConditioningIdentity::new(
        "wrong-family",
        comfy_model::ModelFamilyIdentity::new("COMFY-MODEL-9999", "wrong", "v1")?,
        family_identity(&base, "temporary")?.latent_format().clone(),
    )?;
    let wrong = family_conditioning(
        wrong_identity,
        tensor_from_f32(&backend, &[1, 2, 2_048], &[0.0; 4_096], &context)?,
        "wrong",
        &cancellation,
    )?;
    assert!(NativeGuiderPayload::basic(model, wrong).is_err());
    Ok(())
}

#[test]
fn family_model_resources_match_source_guidance_oracles() -> Result<(), Box<dyn std::error::Error>>
{
    let cancellation = CancellationToken::default();
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(MEMORY_LIMIT)?,
        &cancellation,
    );
    let oracle = family_oracle()?;
    let expected_generator_sha256 = oracle
        .get("generator_sha256")
        .and_then(Value::as_str)
        .ok_or("family oracle is missing its generator digest")?;
    let actual_generator_sha256 = format!(
        "{:x}",
        Sha256::digest(include_bytes!(
            "../fixtures/models/native-family-model-resource-foundation/generate_oracle.py"
        ))
    );
    assert_eq!(actual_generator_sha256, expected_generator_sha256);

    for (family, qwen_model) in [("aura", false), ("qwen", true)] {
        let base = Arc::new(task377::bound_model(
            &backend,
            &context,
            qwen_model,
            MEMORY_LIMIT,
        )?);
        task377::assert_raw_oracle(&base, &backend, &context, qwen_model)?;
        let patch_graph = Arc::new(PatchGraph::checked_semantic(
            FAMILY_ARTIFACT_DIGEST,
            Vec::new(),
        )?);
        let resource = Arc::new(NativeFamilyModelResource::materialize(
            base.clone(),
            patch_graph,
            family_identity(&base, "native-family-model")?,
            &backend,
            &context,
        )?);
        let model = Arc::new(NativeModelPayload::native_family_model(resource.clone())?);
        let (latent_shape, conditioning_width, latent_values) = if qwen_model {
            (
                vec![1, 16, 1, 3, 3],
                3_584,
                (0..144)
                    .map(|index| (index as f32 - 72.0) * 0.005)
                    .collect::<Vec<_>>(),
            )
        } else {
            (
                vec![1, 4, 3, 3],
                2_048,
                (0..36)
                    .map(|index| (index as f32 - 18.0) * 0.025)
                    .collect::<Vec<_>>(),
            )
        };
        let latent = tensor_from_f32(&backend, &latent_shape, &latent_values, &context)?;
        let positive = family_conditioning(
            family_identity(&base, &format!("{family}-positive"))?,
            tensor_from_f32(
                &backend,
                &[1, 2, conditioning_width],
                &family_patterned_values("positive", 2 * conditioning_width as usize),
                &context,
            )?,
            "positive",
            &cancellation,
        )?;
        let negative = family_conditioning(
            family_identity(&base, &format!("{family}-negative"))?,
            tensor_from_f32(
                &backend,
                &[1, 2, conditioning_width],
                &family_patterned_values("negative.bias", 2 * conditioning_width as usize),
                &context,
            )?,
            "negative",
            &cancellation,
        )?;
        let model_time = tensor_from_f32(&backend, &[1], &[0.5], &context)?;
        let mut raw = Vec::new();
        for conditioning in [&positive, &negative] {
            let resolved = conditioning.resolve(latent.descriptor(), &backend, &context)?;
            let entry = resolved
                .first()
                .ok_or("family conditioning did not resolve")?;
            let family_context =
                NativeFamilyDenoiserContext::checked(conditioning.identity(), &context)?;
            raw.push(resource.invoke_denoiser(
                &backend,
                NativeFamilyDenoiserInvocation {
                    scaled_latent: &latent,
                    model_time: &model_time,
                    conditioning: entry,
                    attention_mask: None,
                    reference_latents: &[],
                    additional_timestep_condition: None,
                },
                &family_context,
            )?);
        }
        let expected_positive_raw = family_oracle_raw(&oracle, family, "positive")?;
        let expected_negative_raw = family_oracle_raw(&oracle, family, "negative")?;
        assert_tensor_close(&backend, &raw[0], &expected_positive_raw, &context)?;
        assert_tensor_close(&backend, &raw[1], &expected_negative_raw, &context)?;
        assert_ne!(
            tensor_f32_sha256(&backend, &raw[0], &context)?,
            tensor_f32_sha256(&backend, &raw[1], &context)?
        );
        let resolved_positive = positive.resolve(latent.descriptor(), &backend, &context)?;
        let positive_entry = resolved_positive
            .first()
            .ok_or("family positive conditioning did not resolve")?;
        let positive_context = NativeFamilyDenoiserContext::checked(positive.identity(), &context)?;
        let changed_time = tensor_from_f32(&backend, &[1], &[0.25], &context)?;
        let changed_time_raw = resource.invoke_denoiser(
            &backend,
            NativeFamilyDenoiserInvocation {
                scaled_latent: &latent,
                model_time: &changed_time,
                conditioning: positive_entry,
                attention_mask: None,
                reference_latents: &[],
                additional_timestep_condition: None,
            },
            &positive_context,
        )?;
        assert_ne!(
            tensor_f32_sha256(&backend, &changed_time_raw, &context)?,
            tensor_f32_sha256(&backend, &raw[0], &context)?
        );
        let mut changed_latent_values = latent_values.clone();
        let first = changed_latent_values
            .first_mut()
            .ok_or("family latent fixture is empty")?;
        *first += 0.125;
        let changed_latent =
            tensor_from_f32(&backend, &latent_shape, &changed_latent_values, &context)?;
        let changed_latent_raw = resource.invoke_denoiser(
            &backend,
            NativeFamilyDenoiserInvocation {
                scaled_latent: &changed_latent,
                model_time: &model_time,
                conditioning: positive_entry,
                attention_mask: None,
                reference_latents: &[],
                additional_timestep_condition: None,
            },
            &positive_context,
        )?;
        assert_ne!(
            tensor_f32_sha256(&backend, &changed_latent_raw, &context)?,
            tensor_f32_sha256(&backend, &raw[0], &context)?
        );

        let profile = profile_for_model(&model)?;
        let positive_interpreted =
            interpret_prediction_for_profile(&profile, &backend, &raw[0], &latent, 0.5, &context)?;
        let negative_interpreted =
            interpret_prediction_for_profile(&profile, &backend, &raw[1], &latent, 0.5, &context)?;
        let expected_positive_raw_tensor =
            tensor_from_f32(&backend, &latent_shape, &expected_positive_raw, &context)?;
        let expected_negative_raw_tensor =
            tensor_from_f32(&backend, &latent_shape, &expected_negative_raw, &context)?;
        let expected_positive_interpreted = interpret_prediction_for_profile(
            &profile,
            &backend,
            &expected_positive_raw_tensor,
            &latent,
            0.5,
            &context,
        )?;
        let expected_negative_interpreted = interpret_prediction_for_profile(
            &profile,
            &backend,
            &expected_negative_raw_tensor,
            &latent,
            0.5,
            &context,
        )?;
        assert_tensor_close(
            &backend,
            &positive_interpreted,
            &tensor_to_f32(&backend, &expected_positive_interpreted, &context)?,
            &context,
        )?;
        assert_tensor_close(
            &backend,
            &negative_interpreted,
            &tensor_to_f32(&backend, &expected_negative_interpreted, &context)?,
            &context,
        )?;

        let plan =
            checked_native_diffusion_plan_for_profile(&profile, "euler", "normal", 7, 4, 2.0, 1.0)?;
        let guider =
            NativeGuiderPayload::cfg(model.clone(), positive.clone(), negative.clone(), 2.0)?;
        let mut denoiser = NativeFamilyGuidanceDenoiser::checked(&model, &backend)?;
        let guided = guider.execute(
            &backend,
            &latent,
            0.5,
            &profile,
            &plan,
            GuidanceOptions::default(),
            &mut denoiser,
            &mut [],
            &context,
        )?;
        let expected_positive = tensor_to_f32(&backend, &expected_positive_interpreted, &context)?;
        let expected_negative = tensor_to_f32(&backend, &expected_negative_interpreted, &context)?;
        let expected_guided = expected_positive
            .iter()
            .zip(expected_negative.iter())
            .map(|(positive, negative)| *negative + (*positive - *negative) * 2.0)
            .collect::<Vec<_>>();
        assert_tensor_close(&backend, guided.guided(), &expected_guided, &context)?;

        let unit_plan =
            checked_native_diffusion_plan_for_profile(&profile, "euler", "normal", 7, 4, 1.0, 1.0)?;
        let unit_guider = NativeGuiderPayload::cfg(model.clone(), positive, negative, 1.0)?;
        let mut unit_denoiser = NativeFamilyGuidanceDenoiser::checked(&model, &backend)?;
        let unit_guided = unit_guider.execute(
            &backend,
            &latent,
            0.5,
            &profile,
            &unit_plan,
            GuidanceOptions::default(),
            &mut unit_denoiser,
            &mut [],
            &context,
        )?;
        assert_tensor_close(
            &backend,
            unit_guided.guided(),
            &tensor_to_f32(&backend, &expected_positive_interpreted, &context)?,
            &context,
        )?;

        let sigmas = normal_sigmas_for_profile(&profile, &backend, &context, 4, 1.0)?;
        let expected_sigma_bits = oracle
            .pointer(&format!("/families/{family}/normal_sigmas_bits"))
            .and_then(Value::as_array)
            .ok_or("family oracle has no normal sigma bits")?
            .iter()
            .map(|value| {
                value
                    .as_u64()
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or("family oracle sigma bit is invalid")
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            sigmas
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected_sigma_bits
        );

        let reconstructed = resource.reconstruct_in_memory(&backend, &context)?;
        assert_eq!(
            reconstructed.semantic_digest_sha256(),
            resource.semantic_digest_sha256()
        );
        assert_eq!(reconstructed.resident_bytes()?, resource.resident_bytes()?);
    }
    Ok(())
}

#[test]
fn family_model_resource_patch_budget_and_cancellation_fail_atomically()
-> Result<(), Box<dyn std::error::Error>> {
    let cancellation = CancellationToken::default();
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(MEMORY_LIMIT)?,
        &cancellation,
    );
    let base = Arc::new(task377::bound_model(
        &backend,
        &context,
        false,
        MEMORY_LIMIT,
    )?);
    let identity = family_identity(&base, "native-family-model")?;
    let empty = NativeFamilyModelResource::materialize(
        base.clone(),
        Arc::new(PatchGraph::checked_semantic(
            FAMILY_ARTIFACT_DIGEST,
            Vec::new(),
        )?),
        identity.clone(),
        &backend,
        &context,
    )?;

    let patch_graph = Arc::new(PatchGraph::checked_semantic(
        FAMILY_ARTIFACT_DIGEST,
        vec![SemanticPatchOperation {
            identifier: "fixture-aura-input-bias".to_owned(),
            target_key: "native.init_x_linear.bias".to_owned(),
            expected_shape: vec![2],
            strength: 1.0,
            strength_model: 1.0,
            slices: Vec::new(),
            transform: PatchValueTransform::default(),
            payload: PatchPayload::DenseDiff {
                tensor: PatchTensor::checked(vec![2], vec![0.01, -0.01])?,
                pad_weight: false,
            },
        }],
    )?);
    let patched = NativeFamilyModelResource::materialize(
        base.clone(),
        patch_graph,
        identity.clone(),
        &backend,
        &context,
    )?;
    assert_ne!(
        patched.semantic_digest_sha256(),
        empty.semantic_digest_sha256()
    );
    assert_ne!(patched.mapped_state_sha256(), empty.mapped_state_sha256());
    assert!(
        patched.resident_tensor_allocations()?.len() > empty.resident_tensor_allocations()?.len()
    );

    let wrong_patch = Arc::new(PatchGraph::checked_semantic("f".repeat(64), Vec::new())?);
    assert!(matches!(
        NativeFamilyModelResource::materialize(
            base.clone(),
            wrong_patch,
            identity.clone(),
            &backend,
            &context,
        ),
        Err(ModelFamilyError::FamilyResourcePatchMismatch)
    ));
    assert!(matches!(
        NativeFamilyModelResource::materialize(
            base.clone(),
            Arc::new(PatchGraph::checked_semantic(
                FAMILY_ARTIFACT_DIGEST,
                Vec::new(),
            )?),
            family_identity(&base, "wrong-resource-namespace")?,
            &backend,
            &context,
        ),
        Err(ModelFamilyError::FamilyResourceConditioningMismatch)
    ));

    let required = empty.resident_bytes()?;
    let low_budget_base = Arc::new(task377::bound_model(
        &backend,
        &context,
        false,
        required.saturating_sub(1),
    )?);
    assert!(matches!(
        NativeFamilyModelResource::materialize(
            low_budget_base,
            Arc::new(PatchGraph::checked_semantic(
                FAMILY_ARTIFACT_DIGEST,
                Vec::new(),
            )?),
            identity.clone(),
            &backend,
            &context,
        ),
        Err(ModelFamilyError::OutOfMemory { .. })
    ));

    let cancelled = CancellationToken::default();
    cancelled.cancel();
    let cancelled_context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(MEMORY_LIMIT)?,
        &cancelled,
    );
    assert!(matches!(
        NativeFamilyModelResource::materialize(
            base,
            Arc::new(PatchGraph::checked_semantic(
                FAMILY_ARTIFACT_DIGEST,
                Vec::new(),
            )?),
            identity,
            &backend,
            &cancelled_context,
        ),
        Err(ModelFamilyError::Cancelled(_))
    ));
    Ok(())
}

#[test]
fn native_clip_transport_preserves_schedule_state_and_restart_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let cancellation = CancellationToken::default();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(64 * 1024 * 1024)?;
    let context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority.authorize_workspace(64 * 1024 * 1024)?,
        &cancellation,
    );
    let resource = Arc::new(NativeClipResource::checked(
        NativeClipProfile::Sd3,
        true,
        Vec::new(),
        Vec::new(),
        &backend,
        &context,
    )?);
    let outputs = resource.execute(
        &backend,
        "unused by the all-missing SD3 source path",
        &context,
    )?;
    assert_eq!(outputs.len(), 1);
    assert_eq!(
        outputs[0].conditioning().descriptor().shape(),
        &[1, 77, 4_096]
    );
    assert_eq!(
        outputs[0]
            .pooled()
            .ok_or("SD3 pooled fallback is missing")?
            .descriptor()
            .shape(),
        &[1, 2_048]
    );
    assert!(resource.schedule_enabled());
    let restarted = resource.restart(&backend, &context)?;
    assert_eq!(
        restarted.semantic_digest_sha256(),
        resource.semantic_digest_sha256()
    );

    let model_payload = Arc::new(NativeModelPayload::native_clip(resource.clone())?);
    let diffusion = NativeDiffusionPayload::clip(model_payload.clone())?;
    diffusion.validate()?;
    assert!(
        diffusion
            .clip_payload()
            .is_some_and(|stored| Arc::ptr_eq(stored, &model_payload))
    );
    assert!(
        diffusion
            .native_clip_resource()
            .is_some_and(|stored| Arc::ptr_eq(stored, &resource))
    );
    assert!(diffusion.resident_bytes()? > 0);
    Ok(())
}

#[test]
fn native_vae_transport_preserves_the_existing_image_resource()
-> Result<(), Box<dyn std::error::Error>> {
    let cancellation = CancellationToken::default();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let context = backend.execution_context(
        StreamId::DEFAULT,
        workspace_authority.authorize_workspace(MEMORY_LIMIT)?,
        &cancellation,
    );
    let fixture = NativeDiffusionFixture::checked_in();
    let bundle = fixture.load_bundle_with_context(Arc::new(backend), &context)?;
    let model_payload = Arc::new(NativeModelPayload::native_vae(bundle.vae().clone())?);
    let diffusion = NativeDiffusionPayload::vae(model_payload.clone())?;
    diffusion.validate()?;
    assert!(
        diffusion
            .vae_payload()
            .is_some_and(|stored| Arc::ptr_eq(stored, &model_payload))
    );
    assert!(model_payload.vae().is_some());
    assert!(model_payload.structured_vae().is_none());
    Ok(())
}

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
