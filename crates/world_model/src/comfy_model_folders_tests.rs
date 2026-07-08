use std::{collections::BTreeSet, path::PathBuf};

use crate::{
    ComfyModelFolderRegistry, ExtraModelPathConfig, ExtraModelPathRoot, ModelCategory,
    ModelFolderError,
};

#[test]
fn registry_registers_all_required_comfy_categories() {
    let registry = ComfyModelFolderRegistry::new("/project/assets");
    let categories = registry
        .folders()
        .into_iter()
        .map(|folder| folder.category)
        .collect::<BTreeSet<_>>();

    for category in ModelCategory::all() {
        assert!(categories.contains(category), "missing {category:?}");
    }

    assert_eq!(categories.len(), ModelCategory::all().len());
}

#[test]
fn registry_registers_allowed_extensions() {
    let registry = ComfyModelFolderRegistry::new("/project/assets");
    let checkpoints = registry
        .folder(ModelCategory::Checkpoints)
        .expect("checkpoints folder registered");
    assert!(checkpoints.allowed_extensions.contains("safetensors"));
    assert!(checkpoints.allowed_extensions.contains("ckpt"));

    let configs = registry
        .folder(ModelCategory::Configs)
        .expect("configs folder registered");
    assert!(configs.allowed_extensions.contains("yaml"));
    assert!(!configs.allowed_extensions.contains("ckpt"));
}

#[test]
fn registry_maps_legacy_names_to_canonical_category() {
    let registry = ComfyModelFolderRegistry::new("/project/assets");

    assert_eq!(
        registry.category_for_name("ckpt"),
        Ok(ModelCategory::Checkpoints)
    );
    assert_eq!(
        registry.category_for_name("control_net"),
        Ok(ModelCategory::ControlNet)
    );
    assert_eq!(
        registry.category_for_name("unet"),
        Ok(ModelCategory::DiffusionModels)
    );
    assert_eq!(
        registry.category_for_name("clip"),
        Ok(ModelCategory::TextEncoders)
    );
}

#[test]
fn registry_merges_extra_paths_without_replacing_project_root() {
    let mut registry = ComfyModelFolderRegistry::new("/project/assets");
    registry
        .add_extra_paths(
            ExtraModelPathConfig::new()
                .with_root(ExtraModelPathRoot::new(
                    "checkpoints",
                    "/shared/comfy/checkpoints",
                ))
                .with_root(ExtraModelPathRoot::new("ckpt", "/legacy/checkpoints")),
        )
        .expect("extra paths merge");

    let checkpoints = registry
        .folder(ModelCategory::Checkpoints)
        .expect("checkpoints folder registered");

    assert_eq!(
        checkpoints.roots[0],
        PathBuf::from("/project/assets/models/checkpoints")
    );
    assert!(
        checkpoints
            .roots
            .contains(&PathBuf::from("/shared/comfy/checkpoints"))
    );
    assert!(
        checkpoints
            .roots
            .contains(&PathBuf::from("/legacy/checkpoints"))
    );
}

#[test]
fn registry_deduplicates_extra_paths() {
    let mut registry = ComfyModelFolderRegistry::new("/project/assets");
    registry
        .add_extra_paths(
            ExtraModelPathConfig::new()
                .with_root(ExtraModelPathRoot::new("vae", "/shared/vae"))
                .with_root(ExtraModelPathRoot::new("vae", "/shared/vae")),
        )
        .expect("extra paths merge");

    let vae = registry
        .folder(ModelCategory::Vae)
        .expect("vae folder registered");
    assert_eq!(
        vae.roots
            .iter()
            .filter(|root| *root == &PathBuf::from("/shared/vae"))
            .count(),
        1
    );
}

#[test]
fn registry_resolves_safe_relative_paths_inside_category_root() {
    let registry = ComfyModelFolderRegistry::new("/project/assets");
    let file = registry
        .resolve("checkpoints", "sdxl/base.safetensors")
        .expect("safe path resolves");

    assert_eq!(file.category, ModelCategory::Checkpoints);
    assert_eq!(file.root_index, 0);
    assert_eq!(
        file.full_path,
        PathBuf::from("/project/assets/models/checkpoints/sdxl/base.safetensors")
    );
}

#[test]
fn registry_rejects_paths_that_escape_registered_roots() {
    let registry = ComfyModelFolderRegistry::new("/project/assets");
    let error = registry
        .resolve("checkpoints", "../outside.safetensors")
        .expect_err("escaping path rejected");

    assert!(matches!(error, ModelFolderError::UnsafeRelativePath { .. }));
}

#[test]
fn registry_rejects_disallowed_extensions() {
    let registry = ComfyModelFolderRegistry::new("/project/assets");
    let error = registry
        .resolve("configs", "not-a-config.safetensors")
        .expect_err("extension rejected");

    assert!(matches!(
        error,
        ModelFolderError::ExtensionNotAllowed {
            category: ModelCategory::Configs,
            ..
        }
    ));
}

#[test]
fn registry_reports_unknown_extra_path_categories() {
    let mut registry = ComfyModelFolderRegistry::new("/project/assets");
    let error = registry
        .add_extra_paths(
            ExtraModelPathConfig::new()
                .with_root(ExtraModelPathRoot::new("unknown", "/tmp/models")),
        )
        .expect_err("unknown category rejected");

    assert_eq!(
        error,
        ModelFolderError::UnknownCategory("unknown".to_string())
    );
}
