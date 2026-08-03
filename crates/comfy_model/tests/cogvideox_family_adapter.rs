use comfy_model::{
    CogVideoXLatentVariant, CogVideoXLayout, ModelFamilyError, ModelProbe,
    cogvideox_configuration_for_probe,
};
use std::{collections::BTreeMap, fs, path::Path};

#[test]
fn val_model_family_row_001_cogvideox_adapter_preserves_spatial_and_temporal_profiles() {
    let spatial = cogvideox_configuration_for_probe(
        &probe("native", 32, 1, PatchVariant::Spatial, false),
        32,
        "CogVideoX_I2V",
    )
    .expect("native spatial configuration");
    assert_eq!(spatial.layout, CogVideoXLayout::Native);
    assert_eq!(spatial.number_of_attention_heads, 1);
    assert_eq!(spatial.temporal_patch_size, None);
    assert_eq!(
        (
            spatial.sample_height,
            spatial.sample_width,
            spatial.sample_frames
        ),
        (60, 90, 49)
    );
    assert_eq!(spatial.latent_variant, CogVideoXLatentVariant::CogVideoX);

    let temporal = cogvideox_configuration_for_probe(
        &probe("diffusers", 48, 48, PatchVariant::Temporal, true),
        48,
        "CogVideoX_Inpaint",
    )
    .expect("diffusers temporal configuration");
    assert_eq!(temporal.layout, CogVideoXLayout::Diffusers);
    assert_eq!(temporal.temporal_patch_size, Some(2));
    assert_eq!(
        (
            temporal.sample_height,
            temporal.sample_width,
            temporal.sample_frames
        ),
        (96, 170, 81)
    );
    assert_eq!(temporal.text_embedding_dimension, Some(4_096));
    assert_eq!(temporal.ofs_embedding_dimension, Some(2));
    assert!(temporal.learned_positional_embeddings);
    assert_eq!(
        temporal.latent_variant,
        CogVideoXLatentVariant::CogVideoX1_5
    );
}

#[test]
fn val_model_family_row_001_cogvideox_adapter_rejects_malformed_and_cross_family_probes() {
    let wrong_channel = probe("native", 16, 1, PatchVariant::Spatial, false);
    assert!(matches!(
        cogvideox_configuration_for_probe(&wrong_channel, 32, "CogVideoX_I2V"),
        Err(ModelFamilyError::InvalidSelectorOutput(message))
            if message == "CogVideoX_I2V configuration in_channels 16; requires 32"
    ));

    let mut malformed = probe("diffusers", 16, 1, PatchVariant::Spatial, false);
    malformed
        .tensor_shapes
        .insert("blocks.0.norm1.linear.weight".to_string(), vec![65, 2]);
    assert!(matches!(
        cogvideox_configuration_for_probe(&malformed, 16, "CogVideoX_T2V"),
        Err(ModelFamilyError::InvalidSelectorOutput(message))
            if message == "CogVideoX_T2V configuration blocks.0.norm1.linear.weight shape"
    ));

    let mut malformed_optional = probe("diffusers", 16, 1, PatchVariant::Spatial, false);
    malformed_optional
        .tensor_shapes
        .insert("patch_embed.text_proj.weight".to_string(), vec![2]);
    assert!(matches!(
        cogvideox_configuration_for_probe(&malformed_optional, 16, "CogVideoX_T2V"),
        Err(ModelFamilyError::InvalidSelectorOutput(message))
            if message.contains("missing dimension 1 for patch_embed.text_proj.weight")
    ));
}

#[test]
fn val_model_family_row_001_cogvideox_adapter_is_the_single_configuration_owner() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let owner = fs::read_to_string(crate_root.join("src/cogvideox_family.rs"))
        .expect("canonical CogVideoX adapter source");
    assert_eq!(owner.matches("pub enum CogVideoXLayout").count(), 1);
    assert_eq!(owner.matches("pub enum CogVideoXLatentVariant").count(), 1);
    assert_eq!(
        owner.matches("pub struct CogVideoXConfiguration").count(),
        1
    );

    for row in [
        "cogvideox_i2v_comfy_model_0068.rs",
        "cogvideox_inpaint_comfy_model_0069.rs",
        "cogvideox_t2v_comfy_model_0070.rs",
    ] {
        let source = fs::read_to_string(crate_root.join("src/families").join(row))
            .expect("CogVideoX family row source");
        assert!(!source.contains("pub enum CogVideoXLayout"));
        assert!(!source.contains("pub enum CogVideoXLatentVariant"));
        assert!(!source.contains("pub enum CogVideoXT2VLayout"));
        assert!(!source.contains("pub enum CogVideoXT2VLatentVariant"));
        assert!(!source.contains("fn optional_dimension("));
        assert!(source.contains("cogvideox_configuration_for_probe("));
    }
}

#[derive(Clone, Copy)]
enum PatchVariant {
    Spatial,
    Temporal,
}

fn probe(
    layout: &str,
    channels: u64,
    heads: u64,
    patch: PatchVariant,
    optional_dimensions: bool,
) -> ModelProbe {
    let prefix = if layout == "native" {
        "model.diffusion_model."
    } else {
        ""
    };
    let patch_shape = match patch {
        PatchVariant::Spatial => vec![2, channels, 2, 2],
        PatchVariant::Temporal => vec![2, channels * 8],
    };
    let mut tensor_shapes = BTreeMap::from([
        (
            format!("{prefix}blocks.0.norm1.linear.weight"),
            vec![heads * 64 * 6, 2],
        ),
        (format!("{prefix}patch_embed.proj.weight"), patch_shape),
    ]);
    if optional_dimensions {
        tensor_shapes.extend([
            (
                format!("{prefix}patch_embed.text_proj.weight"),
                vec![2, 4_096],
            ),
            (format!("{prefix}ofs_embedding_linear_1.weight"), vec![2, 2]),
            (format!("{prefix}patch_embed.pos_embedding"), vec![1, 2, 2]),
        ]);
    }
    ModelProbe {
        tensor_shapes,
        metadata: BTreeMap::new(),
    }
}
