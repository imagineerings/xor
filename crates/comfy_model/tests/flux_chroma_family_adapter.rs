use comfy_model::{
    FluxChromaFinalHead, FluxChromaLayout, FluxChromaVariant, ModelFamilyError, ModelProbe,
    flux_chroma_configuration_for_probe,
};
use std::{collections::BTreeMap, fs, path::Path};

#[test]
fn val_model_family_row_001_flux_chroma_adapter_preserves_variant_precedence_and_profiles() {
    let flux = flux_chroma_configuration_for_probe(
        &flux_probe("native", false),
        FluxChromaVariant::Flux,
        "Flux",
    )
    .expect("Flux configuration");
    assert_eq!(flux.layout, FluxChromaLayout::Native);
    assert_eq!((flux.in_channels, flux.out_channels), (16, 16));
    assert_eq!((flux.patch_size, flux.hidden_size), (2, 3_072));
    assert_eq!(flux.vector_input_dimension, Some(768));
    assert!(flux.guidance_embedding);

    let flux2 = flux_chroma_configuration_for_probe(
        &flux_probe("unprefixed", true),
        FluxChromaVariant::Flux2,
        "Flux2",
    )
    .expect("Flux2 configuration");
    assert_eq!(flux2.layout, FluxChromaLayout::Unprefixed);
    assert_eq!((flux2.patch_size, flux2.out_channels), (1, 128));

    let chroma =
        flux_chroma_configuration_for_probe(&chroma_probe(), FluxChromaVariant::Chroma, "Chroma")
            .expect("Chroma configuration");
    assert_eq!((chroma.in_channels, chroma.out_channels), (64, 64));
    assert_eq!(chroma.hidden_size, 5_120);

    let radiance = flux_chroma_configuration_for_probe(
        &radiance_probe(),
        FluxChromaVariant::ChromaRadiance,
        "ChromaRadiance",
    )
    .expect("ChromaRadiance configuration");
    assert_eq!(radiance.patch_size, 4);
    assert_eq!(radiance.context_input_dimension, 4_096);
    assert_eq!(radiance.final_head, FluxChromaFinalHead::Linear);
    assert!(radiance.use_x0_prediction);
    assert!(radiance.use_sequential_text_ids);
}

#[test]
fn val_model_family_row_001_flux_chroma_adapter_rejects_malformed_and_cross_family_probes() {
    assert!(matches!(
        flux_chroma_configuration_for_probe(
            &chroma_probe(),
            FluxChromaVariant::Flux,
            "Flux",
        ),
        Err(ModelFamilyError::InvalidSelectorOutput(message))
            if message.contains("selected Chroma, expected Flux")
    ));

    let mut duplicate_key_norm = chroma_probe();
    duplicate_key_norm.tensor_shapes.insert(
        "double_blocks.0.img_attn.norm.key_norm.scale".to_string(),
        vec![128],
    );
    assert!(matches!(
        flux_chroma_configuration_for_probe(
            &duplicate_key_norm,
            FluxChromaVariant::Chroma,
            "Chroma",
        ),
        Err(ModelFamilyError::InvalidSelectorOutput(message))
            if message.contains("exactly one weight/scale")
    ));

    let mut malformed_patch = radiance_probe();
    malformed_patch
        .tensor_shapes
        .insert("img_in_patch.weight".to_string(), vec![64, 3, 4, 3]);
    assert!(matches!(
        flux_chroma_configuration_for_probe(
            &malformed_patch,
            FluxChromaVariant::ChromaRadiance,
            "ChromaRadiance",
        ),
        Err(ModelFamilyError::InvalidSelectorOutput(message))
            if message.contains("img_in_patch.weight shape")
    ));
}

#[test]
fn val_model_family_row_001_flux_chroma_adapter_is_the_single_shared_owner() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let owner = fs::read_to_string(crate_root.join("src/flux_chroma_family.rs"))
        .expect("canonical Flux/Chroma adapter source");
    assert_eq!(owner.matches("pub enum FluxChromaVariant").count(), 1);
    assert_eq!(owner.matches("pub enum FluxChromaLayout").count(), 1);
    assert_eq!(
        owner.matches("pub struct FluxChromaConfiguration").count(),
        1
    );

    for row in [
        "flux_comfy_model_0077.rs",
        "flux2_comfy_model_0078.rs",
        "fluxinpaint_comfy_model_0079.rs",
        "fluxschnell_comfy_model_0080.rs",
        "chroma_comfy_model_0066.rs",
        "chromaradiance_comfy_model_0067.rs",
    ] {
        let source = fs::read_to_string(crate_root.join("src/families").join(row))
            .expect("Flux/Chroma family row source");
        assert!(!source.contains("fn require_vector_dimension("));
        assert!(!source.contains("fn consecutive_blocks("));
        assert!(source.contains("flux_chroma_configuration_for_probe("));
    }

    for row in [
        "flux_comfy_model_0077.rs",
        "flux2_comfy_model_0078.rs",
        "fluxinpaint_comfy_model_0079.rs",
        "fluxschnell_comfy_model_0080.rs",
    ] {
        let source = fs::read_to_string(crate_root.join("src/families").join(row))
            .expect("Flux family row source");
        for shared_owner in [
            "FLUX_COMPONENTS",
            "FLUX_WEIGHT_RULES",
            "FLUX_MODEL_REQUIRED_KEYS",
            "FLUX_MODEL_OPTIONAL_KEYS",
            "FLUX_FORWARD_PROGRAM",
            "FLUX_STATE_PLAN_CASES",
            "FLUX_COMPONENT_STATE_SCHEMAS",
        ] {
            assert!(
                source.contains(shared_owner),
                "{row} must use canonical {shared_owner}"
            );
            assert!(
                !source.contains(&format!("const {shared_owner}:")),
                "{row} must not redefine canonical {shared_owner}"
            );
        }
        assert!(!source.contains("ModelStateTransformPlanDefinition"));
    }

    for row in [
        "flux_comfy_model_0077.rs",
        "fluxinpaint_comfy_model_0079.rs",
    ] {
        let source = fs::read_to_string(crate_root.join("src/families").join(row))
            .expect("Flux architecture-sharing row source");
        assert!(source.contains("FLUX_ARCHITECTURE_VERSION"));
    }
    for row in [
        "flux_comfy_model_0077.rs",
        "fluxinpaint_comfy_model_0079.rs",
        "fluxschnell_comfy_model_0080.rs",
    ] {
        let source = fs::read_to_string(crate_root.join("src/families").join(row))
            .expect("Flux latent and memory-sharing row source");
        for shared_owner in [
            "FLUX_LATENT_FEATURE_ID",
            "FLUX_LATENT_IDENTIFIER",
            "FLUX_MEMORY_ESTIMATOR",
            "FLUX_MEMORY_USAGE_FACTOR",
        ] {
            assert!(
                source.contains(shared_owner),
                "{row} must use canonical {shared_owner}"
            );
        }
    }

    assert!(
        owner.contains("double_stream_modulation_img.\""),
        "canonical unprefixed plan must own the Flux2 image-modulation mapping"
    );
    assert!(
        owner.contains("double_stream_modulation_txt.\""),
        "canonical unprefixed plan must own the Flux2 text-modulation mapping"
    );
}

fn flux_probe(layout: &str, flux2: bool) -> ModelProbe {
    let prefix = if layout == "native" {
        "model.diffusion_model."
    } else {
        ""
    };
    let mut tensor_shapes = BTreeMap::from([
        (
            format!("{prefix}double_blocks.0.img_attn.norm.key_norm.weight"),
            vec![128],
        ),
        (format!("{prefix}single_blocks.0.linear.weight"), vec![2, 2]),
        (format!("{prefix}img_in.weight"), vec![3_072, 64]),
        (format!("{prefix}txt_in.weight"), vec![3_072, 4_096]),
        (
            format!("{prefix}vector_in.in_layer.weight"),
            vec![3_072, 768],
        ),
        (
            format!("{prefix}guidance_in.in_layer.weight"),
            vec![3_072, 256],
        ),
    ]);
    if flux2 {
        tensor_shapes.insert(
            format!("{prefix}double_stream_modulation_img.lin.weight"),
            vec![2, 2],
        );
    }
    ModelProbe {
        tensor_shapes,
        metadata: BTreeMap::new(),
    }
}

fn chroma_probe() -> ModelProbe {
    ModelProbe {
        tensor_shapes: BTreeMap::from([
            (
                "double_blocks.0.img_attn.norm.key_norm.weight".to_string(),
                vec![128],
            ),
            ("single_blocks.0.linear.weight".to_string(), vec![2, 2]),
            (
                "distilled_guidance_layer.norms.0.scale".to_string(),
                vec![5_120],
            ),
        ]),
        metadata: BTreeMap::new(),
    }
}

fn radiance_probe() -> ModelProbe {
    let mut tensor_shapes = BTreeMap::from([
        (
            "double_blocks.0.img_attn.norm.key_norm.scale".to_string(),
            vec![128],
        ),
        ("single_blocks.0.linear.weight".to_string(), vec![2, 2]),
        (
            "distilled_guidance_layer.norms.0.weight".to_string(),
            vec![5_120],
        ),
        ("img_in_patch.weight".to_string(), vec![5_120, 3, 4, 4]),
        ("txt_in.weight".to_string(), vec![5_120, 4_096]),
        ("nerf_final_layer.norm.scale".to_string(), vec![64]),
        ("nerf_final_layer.linear.weight".to_string(), vec![3, 64]),
        ("__x0__".to_string(), vec![1]),
        ("__sequential__".to_string(), vec![1]),
    ]);
    for index in 0..4 {
        tensor_shapes.insert(format!("nerf_blocks.{index}.norm.weight"), vec![64]);
    }
    ModelProbe {
        tensor_shapes,
        metadata: BTreeMap::new(),
    }
}
