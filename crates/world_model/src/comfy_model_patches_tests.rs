use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use crate::{
    ComfyExecutionRegistry, ComfyModelComponentComposer, ComfyModelPatchPipeline, LatentFormat,
    ModelCategory, ModelComponent, ModelComponentRole, ModelFamilyKind, ModelFileRef,
    ModelPatchKind, ModelPatchRecord,
};

#[test]
fn component_composer_builds_native_component_set_from_loader_outputs() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let components = vec![
        component(
            ModelComponentRole::Checkpoint,
            ModelCategory::Checkpoints,
            ModelFamilyKind::StableDiffusionXl,
            LatentFormat::StableDiffusionXl,
        ),
        component(
            ModelComponentRole::Clip,
            ModelCategory::TextEncoders,
            ModelFamilyKind::StableDiffusionXl,
            LatentFormat::StableDiffusionXl,
        ),
        component(
            ModelComponentRole::Vae,
            ModelCategory::Vae,
            ModelFamilyKind::StableDiffusionXl,
            LatentFormat::StableDiffusionXl,
        ),
    ];

    let set = ComfyModelComponentComposer::new()
        .compose("sdxl-components", family, components)
        .expect("component set builds");

    assert_eq!(set.family, ModelFamilyKind::StableDiffusionXl);
    assert_eq!(set.latent_format, LatentFormat::StableDiffusionXl);
    assert_eq!(set.components.len(), 3);
    assert!(set.components.iter().any(|component| {
        component.role == ModelComponentRole::Checkpoint
            && component.provenance == "loaded by Sim model component composer"
    }));
}

#[test]
fn component_composer_rejects_missing_base_category_family_and_latent_mismatches() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let components = vec![component(
        ModelComponentRole::Clip,
        ModelCategory::Vae,
        ModelFamilyKind::Flux,
        LatentFormat::Flux,
    )];

    let diagnostics = ComfyModelComponentComposer::new()
        .compose("bad-components", family, components)
        .expect_err("component mismatches rejected");

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_model_components::COMPONENT_MISSING_BASE_CODE
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_model_components::COMPONENT_CATEGORY_MISMATCH_CODE
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_model_components::COMPONENT_FAMILY_MISMATCH_CODE
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_model_components::COMPONENT_LATENT_MISMATCH_CODE
    }));
}

#[test]
fn patch_pipeline_applies_patches_in_deterministic_native_order() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let component_set = ComfyModelComponentComposer::new()
        .compose(
            "sdxl-components",
            family,
            vec![component(
                ModelComponentRole::Checkpoint,
                ModelCategory::Checkpoints,
                ModelFamilyKind::StableDiffusionXl,
                LatentFormat::StableDiffusionXl,
            )],
        )
        .expect("component set builds");
    let plan = ComfyModelPatchPipeline::new()
        .plan(
            component_set,
            family,
            vec![
                patch(
                    "control",
                    ModelPatchKind::ControlNet,
                    ModelCategory::ControlNet,
                    0,
                ),
                patch("lora-b", ModelPatchKind::Lora, ModelCategory::Loras, 2),
                patch(
                    "merge",
                    ModelPatchKind::ModelMerge,
                    ModelCategory::ModelPatches,
                    10,
                ),
                patch("lora-a", ModelPatchKind::Lora, ModelCategory::Loras, 1),
            ],
        )
        .expect("patch plan builds");

    let ordered_ids = plan
        .patches
        .iter()
        .map(|patch| patch.record.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ordered_ids, vec!["merge", "lora-a", "lora-b", "control"]);
    assert_eq!(plan.patches[0].sequence, 0);
    assert_eq!(plan.patches[3].sequence, 3);
    assert_eq!(plan.patches[1].record.provenance, "patch provenance");
}

#[test]
fn patch_pipeline_rejects_category_family_duplicate_and_strength_mismatches() {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::Flux)
        .expect("flux family");
    let component_set = crate::ModelComponentSet {
        id: "flux-components".to_string(),
        family: ModelFamilyKind::Flux,
        latent_format: LatentFormat::Flux,
        components: vec![component(
            ModelComponentRole::DiffusionModel,
            ModelCategory::DiffusionModels,
            ModelFamilyKind::Flux,
            LatentFormat::Flux,
        )],
    };
    let mut incompatible = patch("duplicate", ModelPatchKind::Lora, ModelCategory::Vae, 0);
    incompatible.compatible_families = BTreeSet::from([ModelFamilyKind::StableDiffusionXl]);
    incompatible.strength_model = 99.0;

    let diagnostics = ComfyModelPatchPipeline::new()
        .plan(
            component_set,
            family,
            vec![
                incompatible,
                patch("duplicate", ModelPatchKind::Lora, ModelCategory::Loras, 1),
            ],
        )
        .expect_err("patch mismatches rejected");

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::comfy_model_patches::PATCH_DUPLICATE_CODE
        })
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_model_patches::PATCH_CATEGORY_MISMATCH_CODE
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_model_patches::PATCH_FAMILY_MISMATCH_CODE
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_model_patches::PATCH_STRENGTH_MISMATCH_CODE
    }));
}

fn component(
    role: ModelComponentRole,
    category: ModelCategory,
    family: ModelFamilyKind,
    latent_format: LatentFormat,
) -> ModelComponent {
    ModelComponent {
        role,
        file: model_file(category, role.expected_category().canonical_name()),
        family,
        latent_format,
        provenance: "loaded by Sim model component composer".to_string(),
    }
}

fn patch(id: &str, kind: ModelPatchKind, category: ModelCategory, order: u32) -> ModelPatchRecord {
    ModelPatchRecord {
        id: id.to_string(),
        kind,
        file: model_file(category, id),
        compatible_families: BTreeSet::from([
            ModelFamilyKind::StableDiffusionXl,
            ModelFamilyKind::Flux,
        ]),
        strength_model: 0.8,
        strength_clip: 0.6,
        order,
        provenance: "patch provenance".to_string(),
    }
}

fn model_file(category: ModelCategory, name: &str) -> ModelFileRef {
    let relative_path = PathBuf::from(format!("{name}.safetensors"));
    let root = Path::new("/models").join(category.canonical_name());
    ModelFileRef {
        category,
        root_index: 0,
        root: root.clone(),
        relative_path: relative_path.clone(),
        full_path: root.join(relative_path),
    }
}
