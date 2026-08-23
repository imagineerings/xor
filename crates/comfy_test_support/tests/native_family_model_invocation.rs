use comfy_model::{
    LatentFormatIdentity, ModelFamilyError, NativeFamilyBuildOptions, NativeFamilyModel,
    PatchGraph, PatchPayload, PatchTensor, PatchValueTransform, SemanticPatchOperation,
    build_model_family,
    conditioning::{
        ConditioningEntry, ConditioningEntryOptions, ConditioningIdentity, ConditioningSet,
        ConditioningValue, ResolvedConditioningEntry,
    },
    generated_auraflow_comfy_model_0064 as aura, generated_qwenimage_comfy_model_0113 as qwen,
    generated_sd15_comfy_model_0117 as sd15, map_model_weights,
    model_family::{NativeFamilyDenoiserContext, NativeFamilyDenoiserInvocation},
};
use comfy_tensor::{
    CancellationToken, CpuBackend, CpuWorkspaceAuthority, DType, DeviceId, ExecutionContext,
    StreamId, Tensor,
    generated_comfy_operator_indirection_01::tensor_from_f32_with_backend_exact_native,
    generated_native_diffusion::{tensor_from_f32, tensor_to_f32},
};
use comfy_types::DeviceKind;
use std::{collections::BTreeMap, error::Error};

const MEMORY_LIMIT: u64 = 2 * 1024 * 1024 * 1024;
const ARTIFACT_DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const ORACLE_GENERATOR_SHA256: &str =
    "a3f204112a0b5847f0265c98421ed72c3c3f8b6f2487081e52bfe5dcc1ba104f";
const ORACLE_PROVENANCE: &str = concat!(
    "Aura mmdit.py=0104396eda01a9f78e8aa5b9d15470fc551aa8b0e05137d264f3515fd1739db1; ",
    "Qwen model.py=14c805af8da13d31094c2c704e413cc282ef43f4afe17f77de9070bdca301f28; ",
    "attention.py=436e1d91f8d5d84c5667e051cdf3ab2f91d8db25b66d88a084c89a202de0579e; ",
    "lightricks/model.py=edcdab6083e4e2b4c7c1ff51323174ba0308b7626895e5874238ca09c0bd5f43; ",
    "flux/layers.py=35f2dfbadb8b59de79c306f365f26c2baa0a7d54836414737cd781fc24e6a2bc; ",
    "flux/math.py=ee3473e262894884b12eb3af4caa22d069273eb87db3549fb46c42c37f910ece; ",
    "Aura raw=d4b51dc633cbc296d70d5eba222ccac3a6e4a29133e8caca50ae886e2a6208bc; ",
    "Qwen raw=1fd6d5c61affabe7e2dd7ce573eb9d60eb77037d194586eb433ac916fe69609b",
);
const AURA_ORACLE_BITS: [u32; 36] = [
    3095416078, 3090981402, 914129098, 3086046258, 3075655776, 902724040, 953881305, 950040291,
    956513960, 913596046, 933657556, 3040353286, 941045227, 945499627, 3057863399, 3069562817,
    3088314850, 3071275844, 920390576, 3093208602, 920319014, 3088754202, 3081591856, 906792414,
    917896896, 951960798, 913136096, 3063263421, 927270822, 3020644479, 938111956, 943272427,
    3055417838, 920588265, 3081939931, 923306101,
];
const QWEN_ORACLE_BITS: [u32; 144] = [
    3175500817, 3179397992, 3185267444, 3188796646, 3186615850, 3188077193, 3197060046, 1048646547,
    3192630721, 3170374433, 1020659261, 3172326504, 1034019298, 1040358983, 1030512909, 3191656769,
    1035536972, 3173054300, 3165169644, 1003618558, 1024443595, 3188076838, 3181473579, 3165613052,
    3165118175, 1016341975, 1040605518, 3173300548, 1026599319, 1002571354, 1039656729, 1038608954,
    1034753966, 1036733220, 3190636007, 1043168312, 1040193463, 3172279595, 1033572788, 3176292330,
    3187991340, 3175190608, 1052610727, 3196657394, 1048204735, 3185005238, 3163931980, 3183795020,
    1015290547, 1032677119, 3149294495, 1036627927, 3190851461, 3169652122, 1039188399, 3170538354,
    1040640020, 3144374814, 3188747927, 1033465987, 1040231762, 3170486879, 3172487116, 3179862965,
    3169264827, 3164329684, 1029820545, 1040727368, 976110772, 3188474000, 1038343830, 3190084836,
    1040203477, 1038857356, 3138328926, 3174963947, 3178976680, 3184999008, 3191302397, 1052275184,
    3191089712, 3188662429, 3186347414, 3187942974, 3169300695, 1021733001, 3171789632, 3197551079,
    1035285751, 3189244592, 1034287736, 1040493201, 1031049779, 3164095902, 1007273247, 1024980465,
    3176856907, 1041037068, 1033623689, 3189419015, 3181205142, 3173622149, 3172763678, 1027136189,
    1006749649, 3167354346, 3189145088, 1040475122, 1039925161, 1038877390, 1035022402, 1040327681,
    3171742721, 1033841222, 1049258408, 3191973485, 1047232361, 3175755462, 3187857122, 3174653735,
    3184736802, 3172781612, 3183526580, 1049116308, 3197148427, 1040436114, 1016364289, 1032945555,
    3144271054, 1039456835, 3169464614, 1040774238, 1034463232, 3179411848, 3180732609, 3134230364,
    3188613710, 1033734423, 3179594529, 3168191077, 3163255948, 1009072444, 3159318581, 3182595130,
];

fn patterned_values(key: &str, elements: usize) -> Vec<f32> {
    let normalization = key.contains("norm_") || key.ends_with("txt_norm.weight");
    let bias = key.ends_with(".bias");
    let aura_mlp =
        key.contains(".mlpC.") || key.contains(".mlpX.") || key.contains("single_layers.0.mlp.");
    let aura_attention = key.contains("native.double_layers.0.attn.");
    let mut values = Vec::with_capacity(elements);
    for index in 0..elements {
        let value = if normalization {
            0.95 + (index % 7) as f32 * 0.01
        } else if bias {
            ((index % 11) as f32 - 5.0) * 0.002
        } else if aura_mlp {
            ((index % 17) as f32 - 8.0) * 0.5
        } else if aura_attention {
            ((index % 17) as f32 - 8.0) * 0.1
        } else {
            ((index % 17) as f32 - 8.0) * 0.000_75
        };
        values.push(value);
    }
    values
}

fn tensor_for_shape(
    backend: &CpuBackend,
    shape: &[u64],
    key: &str,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, Box<dyn Error>> {
    let elements = shape
        .iter()
        .try_fold(1_usize, |total, dimension| {
            total.checked_mul(usize::try_from(*dimension).ok()?)
        })
        .ok_or("fixture tensor shape overflow")?;
    Ok(tensor_from_f32(
        backend,
        shape,
        &patterned_values(key, elements),
        context,
    )?)
}

fn aura_shape(key: &str) -> Result<Vec<u64>, Box<dyn Error>> {
    let shape = match key {
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
    };
    Ok(shape)
}

fn qwen_shape(key: &str) -> Result<Vec<u64>, Box<dyn Error>> {
    let shape = match key {
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
    };
    Ok(shape)
}

fn build_fixture_model(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    qwen_model: bool,
    budget: u64,
) -> Result<NativeFamilyModel, Box<dyn Error>> {
    build_fixture_model_with_qwen_markers(
        backend, context, qwen_model, budget, 0.0, 0.0, false, false,
    )
}

fn build_fixture_model_with_qwen_markers(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    qwen_model: bool,
    budget: u64,
    reference_marker: f32,
    additional_marker: f32,
    timestep_zero: bool,
    learned_addition: bool,
) -> Result<NativeFamilyModel, Box<dyn Error>> {
    let (definition, keys): (_, &[&str]) = if qwen_model {
        (&qwen::MODEL_FAMILY, qwen::DENOISER_INVOCATION_REQUIRED_KEYS)
    } else {
        (&aura::MODEL_FAMILY, aura::DENOISER_INVOCATION_REQUIRED_KEYS)
    };
    let mut source = BTreeMap::new();
    for key in keys {
        let shape = if qwen_model {
            qwen_shape(key)?
        } else {
            aura_shape(key)?
        };
        let source_key = key.replacen("native.", "model.diffusion_model.", 1);
        source.insert(source_key, tensor_for_shape(backend, &shape, key, context)?);
    }
    if qwen_model {
        for (key, value) in [
            ("native.__reference_method__", reference_marker),
            (
                "native.__additional_timestep_condition__",
                additional_marker,
            ),
        ] {
            let source_key = key.replacen("native.", "model.diffusion_model.", 1);
            source.insert(
                source_key,
                tensor_from_f32(backend, &[1], &[value], context)?,
            );
        }
        if timestep_zero {
            source.insert(
                "model.diffusion_model.__index_timestep_zero__".to_owned(),
                tensor_from_f32(backend, &[1], &[0.0], context)?,
            );
        }
        if learned_addition {
            source.insert(
                "model.diffusion_model.time_text_embed.addition_t_embedding.weight".to_owned(),
                tensor_from_f32(backend, &[2, 128], &[0.0; 256], context)?,
            );
        }
    }
    let weights = map_model_weights(definition, ARTIFACT_DIGEST, source)?;
    Ok(build_model_family(
        definition,
        weights,
        NativeFamilyBuildOptions {
            dtype: DType::F32,
            device: DeviceKind::Cpu,
            activation_elements: 1,
            memory_budget_bytes: budget,
            allow_unexpected_weights: true,
        },
    )?)
}

fn resolved_conditioning<'a>(
    model: &NativeFamilyModel,
    latent: &Tensor,
    conditioning: Tensor,
    namespace: &str,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<(ConditioningIdentity, Vec<ResolvedConditioningEntry>), Box<dyn Error>> {
    resolved_conditioning_value(
        model,
        latent,
        ConditioningValue::cross_attention(conditioning)?,
        namespace,
        backend,
        context,
    )
}

fn resolved_conditioning_value(
    model: &NativeFamilyModel,
    latent: &Tensor,
    value: ConditioningValue,
    namespace: &str,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<(ConditioningIdentity, Vec<ResolvedConditioningEntry>), Box<dyn Error>> {
    let profile = model.profile();
    let identity = ConditioningIdentity::new(
        namespace,
        model.identity()?,
        LatentFormatIdentity::new(profile.latent_feature_id, profile.latent_identifier)?,
    )?;
    let set = ConditioningSet::checked(
        identity.clone(),
        vec![ConditioningEntry::checked(
            "positive",
            value,
            ConditioningEntryOptions::default(),
        )?],
        context.cancellation,
    )?;
    Ok((
        identity,
        set.resolve(latent.descriptor(), backend, context)?,
    ))
}

fn invoke_values(
    model: &NativeFamilyModel,
    latent: &Tensor,
    model_time: &Tensor,
    identity: &ConditioningIdentity,
    entry: &ResolvedConditioningEntry,
    attention_mask: Option<&Tensor>,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<Vec<f32>, Box<dyn Error>> {
    let family_context = NativeFamilyDenoiserContext::checked(identity, context)?;
    let output = model.invoke_denoiser(
        backend,
        NativeFamilyDenoiserInvocation {
            scaled_latent: latent,
            model_time,
            conditioning: entry,
            attention_mask,
            reference_latents: &[],
            additional_timestep_condition: None,
        },
        &family_context,
    )?;
    Ok(tensor_to_f32(backend, &output, context)?.to_vec())
}

fn assert_independent_oracle(actual: &[f32], expected_bits: &[u32]) {
    assert_eq!(
        actual.len(),
        expected_bits.len(),
        "{ORACLE_GENERATOR_SHA256}: {ORACLE_PROVENANCE}"
    );
    for (index, (actual, expected_bits)) in actual.iter().zip(expected_bits).enumerate() {
        let expected = f32::from_bits(*expected_bits);
        assert!(
            (*actual - expected).abs() <= 1.0e-6,
            "independent source oracle {ORACLE_GENERATOR_SHA256} ({ORACLE_PROVENANCE}) diverged at {index}: {actual} != {expected}",
        );
    }
}

#[test]
fn raw_f32_source_oracles_cover_complete_reduced_blocks() -> Result<(), Box<dyn Error>> {
    let cancellation = CancellationToken::default();
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let workspace = authority.authorize_workspace(MEMORY_LIMIT)?;
    let context = backend.execution_context(StreamId::DEFAULT, workspace, &cancellation);

    let aura_model = build_fixture_model(&backend, &context, false, MEMORY_LIMIT)?;
    let aura_latent_values = (0..36)
        .map(|index| (index as f32 - 18.0) * 0.025)
        .collect::<Vec<_>>();
    let aura_latent = tensor_from_f32(&backend, &[1, 4, 3, 3], &aura_latent_values, &context)?;
    let aura_conditioning_values = patterned_values("aura.conditioning", 2 * 2_048);
    let aura_conditioning = tensor_from_f32(
        &backend,
        &[1, 2, 2_048],
        &aura_conditioning_values,
        &context,
    )?;
    let (aura_identity, aura_entries) = resolved_conditioning(
        &aura_model,
        &aura_latent,
        aura_conditioning,
        "aura.oracle",
        &backend,
        &context,
    )?;
    let aura_time = tensor_from_f32(&backend, &[1], &[0.314_159_27], &context)?;
    let aura_family_context = NativeFamilyDenoiserContext::checked(&aura_identity, &context)?;
    let aura_output = aura_model.invoke_denoiser(
        &backend,
        NativeFamilyDenoiserInvocation {
            scaled_latent: &aura_latent,
            model_time: &aura_time,
            conditioning: &aura_entries[0],
            attention_mask: None,
            reference_latents: &[],
            additional_timestep_condition: None,
        },
        &aura_family_context,
    )?;
    let aura_values = tensor_to_f32(&backend, &aura_output, &context)?;
    assert_eq!(aura_output.descriptor().shape(), &[1, 4, 3, 3]);
    assert!(aura_values.iter().all(|value| value.is_finite()));
    assert_independent_oracle(&aura_values, &AURA_ORACLE_BITS);
    let position_values = (0..32)
        .map(|index| if index % 2 == 0 { 0.125 } else { -0.25 })
        .collect::<Vec<_>>();
    let register_values = (0..16)
        .map(|index| {
            if (index / 2) % 2 == 0 {
                if index % 2 == 0 { -0.375 } else { 0.0625 }
            } else if index % 2 == 0 {
                0.0625
            } else {
                -0.375
            }
        })
        .collect::<Vec<_>>();
    for (identifier, key, shape, values) in [
        (
            "mutate-aura-position",
            "native.positional_encoding",
            vec![1, 16, 2],
            position_values,
        ),
        (
            "mutate-aura-registers",
            "native.register_tokens",
            vec![1, 8, 2],
            register_values,
        ),
    ] {
        let graph = PatchGraph::checked_semantic(
            ARTIFACT_DIGEST,
            vec![SemanticPatchOperation {
                identifier: identifier.to_owned(),
                target_key: key.to_owned(),
                expected_shape: shape.clone(),
                strength: 1.0,
                strength_model: 1.0,
                slices: Vec::new(),
                transform: PatchValueTransform::default(),
                payload: PatchPayload::Set {
                    tensor: PatchTensor::checked(shape, values)?,
                },
            }],
        )?;
        let mutated = build_model_family(
            &aura::MODEL_FAMILY,
            graph.apply(&backend, aura_model.weights(), &context)?,
            NativeFamilyBuildOptions {
                dtype: DType::F32,
                device: DeviceKind::Cpu,
                activation_elements: 1,
                memory_budget_bytes: MEMORY_LIMIT,
                allow_unexpected_weights: true,
            },
        )?;
        assert_ne!(
            invoke_values(
                &mutated,
                &aura_latent,
                &aura_time,
                &aura_identity,
                &aura_entries[0],
                None,
                &backend,
                &context,
            )?,
            &aura_values[..],
            "{identifier}",
        );
    }

    let qwen_model = build_fixture_model(&backend, &context, true, MEMORY_LIMIT)?;
    let qwen_latent_values = (0..144)
        .map(|index| (index as f32 - 72.0) * 0.005)
        .collect::<Vec<_>>();
    let qwen_latent = tensor_from_f32(&backend, &[1, 16, 1, 3, 3], &qwen_latent_values, &context)?;
    let qwen_conditioning_values = patterned_values("qwen.conditioning", 2 * 3_584);
    let qwen_conditioning = tensor_from_f32(
        &backend,
        &[1, 2, 3_584],
        &qwen_conditioning_values,
        &context,
    )?;
    let (qwen_identity, qwen_entries) = resolved_conditioning(
        &qwen_model,
        &qwen_latent,
        qwen_conditioning,
        "qwen.oracle",
        &backend,
        &context,
    )?;
    let qwen_time = tensor_from_f32(&backend, &[1], &[0.271_828_18], &context)?;
    let qwen_mask = tensor_from_f32(&backend, &[1, 2], &[0.0, -0.75], &context)?;
    let qwen_family_context = NativeFamilyDenoiserContext::checked(&qwen_identity, &context)?;
    let qwen_output = qwen_model.invoke_denoiser(
        &backend,
        NativeFamilyDenoiserInvocation {
            scaled_latent: &qwen_latent,
            model_time: &qwen_time,
            conditioning: &qwen_entries[0],
            attention_mask: Some(&qwen_mask),
            reference_latents: &[],
            additional_timestep_condition: None,
        },
        &qwen_family_context,
    )?;
    let qwen_values = tensor_to_f32(&backend, &qwen_output, &context)?;
    assert_eq!(qwen_output.descriptor().shape(), &[1, 16, 1, 3, 3]);
    assert!(qwen_values.iter().all(|value| value.is_finite()));
    assert_independent_oracle(&qwen_values, &QWEN_ORACLE_BITS);

    let mut changed_latent_values = qwen_latent_values.clone();
    changed_latent_values[0] += 0.125;
    let changed_latent = tensor_from_f32(
        &backend,
        &[1, 16, 1, 3, 3],
        &changed_latent_values,
        &context,
    )?;
    assert_ne!(
        invoke_values(
            &qwen_model,
            &changed_latent,
            &qwen_time,
            &qwen_identity,
            &qwen_entries[0],
            Some(&qwen_mask),
            &backend,
            &context
        )?,
        &qwen_values[..]
    );
    let changed_time = tensor_from_f32(&backend, &[1], &[0.381_966], &context)?;
    assert_ne!(
        invoke_values(
            &qwen_model,
            &qwen_latent,
            &changed_time,
            &qwen_identity,
            &qwen_entries[0],
            Some(&qwen_mask),
            &backend,
            &context
        )?,
        &qwen_values[..]
    );
    let mut changed_conditioning_values = qwen_conditioning_values.clone();
    changed_conditioning_values[17] += 0.25;
    let changed_conditioning = tensor_from_f32(
        &backend,
        &[1, 2, 3_584],
        &changed_conditioning_values,
        &context,
    )?;
    let (_, changed_entries) = resolved_conditioning(
        &qwen_model,
        &qwen_latent,
        changed_conditioning,
        "qwen.oracle",
        &backend,
        &context,
    )?;
    assert_ne!(
        invoke_values(
            &qwen_model,
            &qwen_latent,
            &qwen_time,
            &qwen_identity,
            &changed_entries[0],
            Some(&qwen_mask),
            &backend,
            &context
        )?,
        &qwen_values[..]
    );
    let changed_mask = tensor_from_f32(&backend, &[1, 2], &[-1.25, 0.0], &context)?;
    assert_ne!(
        invoke_values(
            &qwen_model,
            &qwen_latent,
            &qwen_time,
            &qwen_identity,
            &qwen_entries[0],
            Some(&changed_mask),
            &backend,
            &context
        )?,
        &qwen_values[..]
    );
    Ok(())
}

#[test]
fn invocation_preflight_rejects_incompatible_state_atomically() -> Result<(), Box<dyn Error>> {
    let cancellation = CancellationToken::default();
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(MEMORY_LIMIT)?,
        &cancellation,
    );
    let model = build_fixture_model(&backend, &context, true, MEMORY_LIMIT)?;
    let latent = tensor_from_f32(&backend, &[1, 16, 1, 3, 3], &[0.0; 144], &context)?;
    let conditioning = tensor_from_f32(&backend, &[1, 2, 3_584], &vec![0.0; 7_168], &context)?;
    let (identity, entries) = resolved_conditioning(
        &model,
        &latent,
        conditioning.clone(),
        "qwen.preflight",
        &backend,
        &context,
    )?;
    let model_time = tensor_from_f32(&backend, &[1], &[0.5], &context)?;
    let family_context = NativeFamilyDenoiserContext::checked(&identity, &context)?;
    let invocation = NativeFamilyDenoiserInvocation {
        scaled_latent: &latent,
        model_time: &model_time,
        conditioning: &entries[0],
        attention_mask: None,
        reference_latents: &[],
        additional_timestep_condition: None,
    };

    let wrong_family = ConditioningIdentity::new(
        "qwen.preflight",
        build_fixture_model(&backend, &context, false, MEMORY_LIMIT)?.identity()?,
        identity.latent_format().clone(),
    )?;
    let wrong_family_context = NativeFamilyDenoiserContext::checked(&wrong_family, &context)?;
    assert!(matches!(
        model.invoke_denoiser(&backend, invocation, &wrong_family_context),
        Err(ModelFamilyError::DenoiserConditioningIdentity(_))
    ));
    let wrong_latent = ConditioningIdentity::new(
        "qwen.preflight",
        model.identity()?,
        LatentFormatIdentity::new("COMFY-MODEL-0047", "SDXL")?,
    )?;
    let wrong_latent_context = NativeFamilyDenoiserContext::checked(&wrong_latent, &context)?;
    assert!(matches!(
        model.invoke_denoiser(&backend, invocation, &wrong_latent_context),
        Err(ModelFamilyError::DenoiserConditioningIdentity(_))
    ));

    let (_, regular_entries) = resolved_conditioning_value(
        &model,
        &latent,
        ConditioningValue::regular(conditioning)?,
        "qwen.preflight",
        &backend,
        &context,
    )?;
    assert!(matches!(
        model.invoke_denoiser(
            &backend,
            NativeFamilyDenoiserInvocation {
                conditioning: &regular_entries[0],
                ..invocation
            },
            &family_context,
        ),
        Err(ModelFamilyError::DenoiserConditioningValue(_))
    ));

    let malformed_latent = tensor_from_f32(&backend, &[1, 16, 9], &[0.0; 144], &context)?;
    assert!(matches!(
        model.invoke_denoiser(
            &backend,
            NativeFamilyDenoiserInvocation {
                scaled_latent: &malformed_latent,
                ..invocation
            },
            &family_context,
        ),
        Err(ModelFamilyError::DenoiserTensorContract(_))
    ));
    let wrong_dtype_latent = tensor_from_f32_with_backend_exact_native(
        &backend,
        &[1, 16, 1, 3, 3],
        &[0.0; 144],
        DType::Bf16,
        DeviceId::CPU,
        &context,
    )?;
    assert!(matches!(
        model.invoke_denoiser(
            &backend,
            NativeFamilyDenoiserInvocation {
                scaled_latent: &wrong_dtype_latent,
                ..invocation
            },
            &family_context,
        ),
        Err(ModelFamilyError::DenoiserTensorContract(_))
    ));
    let nonfinite_time = tensor_from_f32(&backend, &[1], &[f32::NAN], &context)?;
    assert!(matches!(
        model.invoke_denoiser(
            &backend,
            NativeFamilyDenoiserInvocation {
                model_time: &nonfinite_time,
                ..invocation
            },
            &family_context,
        ),
        Err(ModelFamilyError::DenoiserTensorContract(_))
    ));
    assert!(matches!(
        model.invoke_denoiser(
            &backend,
            NativeFamilyDenoiserInvocation {
                reference_latents: std::slice::from_ref(&latent),
                ..invocation
            },
            &family_context,
        ),
        Err(ModelFamilyError::DenoiserUnavailable(_))
    ));
    assert!(matches!(
        model.invoke_denoiser(
            &backend,
            NativeFamilyDenoiserInvocation {
                additional_timestep_condition: Some(&model_time),
                ..invocation
            },
            &family_context,
        ),
        Err(ModelFamilyError::DenoiserUnavailable(_))
    ));
    for rejected_model in [
        build_fixture_model_with_qwen_markers(
            &backend,
            &context,
            true,
            MEMORY_LIMIT,
            1.0,
            0.0,
            false,
            false,
        )?,
        build_fixture_model_with_qwen_markers(
            &backend,
            &context,
            true,
            MEMORY_LIMIT,
            0.0,
            1.0,
            false,
            false,
        )?,
        build_fixture_model_with_qwen_markers(
            &backend,
            &context,
            true,
            MEMORY_LIMIT,
            0.0,
            0.0,
            true,
            false,
        )?,
        build_fixture_model_with_qwen_markers(
            &backend,
            &context,
            true,
            MEMORY_LIMIT,
            0.0,
            0.0,
            false,
            true,
        )?,
    ] {
        assert!(matches!(
            rejected_model.invoke_denoiser(&backend, invocation, &family_context),
            Err(ModelFamilyError::DenoiserUnavailable(_))
        ));
    }

    let stream_context = backend.execution_context(
        StreamId::new(7),
        authority.authorize_workspace(MEMORY_LIMIT)?,
        &cancellation,
    );
    let stream_latent = tensor_from_f32(&backend, &[1, 16, 1, 3, 3], &[0.0; 144], &stream_context)?;
    assert!(matches!(
        model.invoke_denoiser(
            &backend,
            NativeFamilyDenoiserInvocation {
                scaled_latent: &stream_latent,
                ..invocation
            },
            &family_context,
        ),
        Err(ModelFamilyError::DenoiserTensorContract(_))
    ));

    let cancelled = CancellationToken::default();
    let cancelled_context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(MEMORY_LIMIT)?,
        &cancelled,
    );
    let cancelled_family_context =
        NativeFamilyDenoiserContext::checked(&identity, &cancelled_context)?;
    cancelled.cancel();
    let before = cancelled_context.scratch.in_use_bytes();
    assert!(
        model
            .invoke_denoiser(&backend, invocation, &cancelled_family_context)
            .is_err()
    );
    assert_eq!(cancelled_context.scratch.in_use_bytes(), before);
    Ok(())
}

#[test]
fn invocation_budget_counts_conditioning_and_retained_patch_state() -> Result<(), Box<dyn Error>> {
    let cancellation = CancellationToken::default();
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(MEMORY_LIMIT)?,
        &cancellation,
    );
    let initial = build_fixture_model(&backend, &context, false, MEMORY_LIMIT)?;
    let build_budget = initial.memory_estimate().total_bytes;
    let model = build_model_family(
        &aura::MODEL_FAMILY,
        initial.weights().clone(),
        NativeFamilyBuildOptions {
            dtype: DType::F32,
            device: DeviceKind::Cpu,
            activation_elements: 1,
            memory_budget_bytes: build_budget,
            allow_unexpected_weights: true,
        },
    )?;
    let latent = tensor_from_f32(&backend, &[1, 4, 1, 1], &[0.0; 4], &context)?;
    let conditioning = tensor_from_f32(
        &backend,
        &[1, 1_024, 2_048],
        &vec![0.0; 1_024 * 2_048],
        &context,
    )?;
    let (identity, entries) = resolved_conditioning(
        &model,
        &latent,
        conditioning,
        "aura.budget",
        &backend,
        &context,
    )?;
    let model_time = tensor_from_f32(&backend, &[1], &[0.25], &context)?;
    let family_context = NativeFamilyDenoiserContext::checked(&identity, &context)?;
    let invocation = NativeFamilyDenoiserInvocation {
        scaled_latent: &latent,
        model_time: &model_time,
        conditioning: &entries[0],
        attention_mask: None,
        reference_latents: &[],
        additional_timestep_condition: None,
    };
    let estimate = model.denoiser_memory_estimate(invocation, &family_context)?;
    assert!(estimate.total_bytes > build_budget);
    let before = context.scratch.in_use_bytes();
    assert!(matches!(
        model.invoke_denoiser(&backend, invocation, &family_context),
        Err(ModelFamilyError::OutOfMemory { .. })
    ));
    assert_eq!(context.scratch.in_use_bytes(), before);

    let patch_graph = PatchGraph::checked_semantic(
        ARTIFACT_DIGEST,
        vec![SemanticPatchOperation {
            identifier: "replace-final-projection".to_owned(),
            target_key: "native.final_linear.weight".to_owned(),
            expected_shape: vec![16, 2],
            strength: 1.0,
            strength_model: 1.0,
            slices: Vec::new(),
            transform: PatchValueTransform::default(),
            payload: PatchPayload::Set {
                tensor: PatchTensor::checked(vec![16, 2], vec![0.25; 32])?,
            },
        }],
    )?;
    let patched_weights = patch_graph.apply(&backend, initial.weights(), &context)?;
    let patched_model = build_model_family(
        &aura::MODEL_FAMILY,
        patched_weights,
        NativeFamilyBuildOptions {
            dtype: DType::F32,
            device: DeviceKind::Cpu,
            activation_elements: 1,
            memory_budget_bytes: MEMORY_LIMIT,
            allow_unexpected_weights: true,
        },
    )?;
    let patched_estimate = patched_model.denoiser_memory_estimate(invocation, &family_context)?;
    assert!(patched_estimate.retained_weight_bytes > estimate.retained_weight_bytes);
    let patched_budget = patched_model.memory_estimate().total_bytes;
    let underbudget_patched_model = build_model_family(
        &aura::MODEL_FAMILY,
        patched_model.weights().clone(),
        NativeFamilyBuildOptions {
            dtype: DType::F32,
            device: DeviceKind::Cpu,
            activation_elements: 1,
            memory_budget_bytes: patched_budget,
            allow_unexpected_weights: true,
        },
    )?;
    let before = context.scratch.in_use_bytes();
    assert!(matches!(
        underbudget_patched_model.invoke_denoiser(&backend, invocation, &family_context),
        Err(ModelFamilyError::OutOfMemory { .. })
    ));
    assert_eq!(context.scratch.in_use_bytes(), before);
    Ok(())
}

#[test]
fn unsupported_family_never_falls_back_to_unary_forward() -> Result<(), Box<dyn Error>> {
    let cancellation = CancellationToken::default();
    let (backend, authority) = CpuWorkspaceAuthority::create_backend(MEMORY_LIMIT)?;
    let context = backend.execution_context(
        StreamId::DEFAULT,
        authority.authorize_workspace(MEMORY_LIMIT)?,
        &cancellation,
    );
    let mut source = BTreeMap::new();
    for key in sd15::REQUIRED_KEYS {
        source.insert(
            key.replacen("native.", "model.diffusion_model.", 1),
            tensor_from_f32(&backend, &[2, 2], &[0.0; 4], &context)?,
        );
    }
    let weights = map_model_weights(&sd15::MODEL_FAMILY, ARTIFACT_DIGEST, source)?;
    let model = build_model_family(
        &sd15::MODEL_FAMILY,
        weights,
        NativeFamilyBuildOptions {
            dtype: DType::F32,
            device: DeviceKind::Cpu,
            activation_elements: 1,
            memory_budget_bytes: MEMORY_LIMIT,
            allow_unexpected_weights: true,
        },
    )?;
    let latent = tensor_from_f32(&backend, &[1, 4, 2, 2], &[0.0; 16], &context)?;
    let conditioning = tensor_from_f32(&backend, &[1, 1, 768], &[0.0; 768], &context)?;
    let (identity, entries) = resolved_conditioning(
        &model,
        &latent,
        conditioning,
        "sd15.unsupported",
        &backend,
        &context,
    )?;
    let model_time = tensor_from_f32(&backend, &[1], &[0.5], &context)?;
    let family_context = NativeFamilyDenoiserContext::checked(&identity, &context)?;
    let error = model
        .invoke_denoiser(
            &backend,
            NativeFamilyDenoiserInvocation {
                scaled_latent: &latent,
                model_time: &model_time,
                conditioning: &entries[0],
                attention_mask: None,
                reference_latents: &[],
                additional_timestep_condition: None,
            },
            &family_context,
        )
        .expect_err("unsupported family must not use its unary checkpoint program");
    assert!(matches!(error, ModelFamilyError::DenoiserUnavailable(_)));
    Ok(())
}
