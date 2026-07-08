use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    ComfyModelCatalog, ComfyModelFolderRegistry, ExtraModelPathConfig, ExtraModelPathRoot,
    ModelCatalogError, ModelCategory, ModelFolderError,
};

#[test]
fn catalog_lists_visible_files_recursively_with_metadata() {
    let root = test_root("catalog_lists_visible_files_recursively_with_metadata");
    let models = root.join("assets/models/checkpoints");
    create_file(&models.join("sdxl/base.safetensors"), b"model");
    create_file(&models.join("sdxl/readme.txt"), b"not a model");
    create_file(&models.join(".hidden/secret.safetensors"), b"hidden");

    let registry = ComfyModelFolderRegistry::new(root.join("assets"));
    let catalog = ComfyModelCatalog::new(&registry);
    let snapshot = catalog
        .list_category("checkpoints")
        .expect("catalog lists checkpoints");

    assert_eq!(snapshot.category, ModelCategory::Checkpoints);
    assert_eq!(snapshot.files.len(), 1);
    let file = &snapshot.files[0];
    assert_eq!(file.path_index, 0);
    assert_eq!(file.relative_name, "sdxl/base.safetensors");
    assert_eq!(file.size_bytes, 5);
    assert!(file.created_at_ms.is_some());
    assert!(file.modified_at_ms.is_some());

    cleanup_root(&root);
}

#[test]
fn catalog_assigns_stable_path_indexes_across_roots() {
    let root = test_root("catalog_assigns_stable_path_indexes_across_roots");
    let project_root = root.join("assets/models/vae");
    let extra_root = root.join("extra-vae");
    create_file(&project_root.join("z.safetensors"), b"z");
    create_file(&project_root.join("a.safetensors"), b"a");
    create_file(&extra_root.join("b.safetensors"), b"b");

    let mut registry = ComfyModelFolderRegistry::new(root.join("assets"));
    registry
        .add_extra_paths(
            ExtraModelPathConfig::new().with_root(ExtraModelPathRoot::new("vae", extra_root)),
        )
        .expect("extra path merges");
    let catalog = ComfyModelCatalog::new(&registry);
    let snapshot = catalog.list_category("vae").expect("catalog lists vae");
    let names = snapshot
        .files
        .iter()
        .map(|file| {
            (
                file.path_index,
                file.root_index,
                file.relative_name.as_str(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            (0, 0, "a.safetensors"),
            (1, 0, "z.safetensors"),
            (2, 1, "b.safetensors"),
        ]
    );

    cleanup_root(&root);
}

#[test]
fn catalog_snapshot_key_changes_when_root_mtime_changes() {
    let root = test_root("catalog_snapshot_key_changes_when_root_mtime_changes");
    let models = root.join("assets/models/checkpoints");
    create_file(&models.join("first.safetensors"), b"first");

    let registry = ComfyModelFolderRegistry::new(root.join("assets"));
    let catalog = ComfyModelCatalog::new(&registry);
    let first_key = catalog
        .list_category("checkpoints")
        .expect("first snapshot")
        .cache_key();
    std::thread::sleep(std::time::Duration::from_millis(5));
    create_file(&models.join("second.safetensors"), b"second");
    let second_key = catalog
        .list_category("checkpoints")
        .expect("second snapshot")
        .cache_key();

    assert_ne!(first_key, second_key);

    cleanup_root(&root);
}

#[test]
fn catalog_resolves_summary_to_safe_file_reference() {
    let root = test_root("catalog_resolves_summary_to_safe_file_reference");
    let models = root.join("assets/models/checkpoints");
    create_file(&models.join("base.safetensors"), b"model");

    let registry = ComfyModelFolderRegistry::new(root.join("assets"));
    let catalog = ComfyModelCatalog::new(&registry);
    let snapshot = catalog
        .list_category("checkpoints")
        .expect("catalog lists checkpoints");
    let file_ref = catalog
        .resolve_summary(&snapshot.files[0])
        .expect("summary resolves");

    assert_eq!(file_ref.root_index, 0);
    assert_eq!(file_ref.full_path, models.join("base.safetensors"));

    cleanup_root(&root);
}

#[test]
fn catalog_rejects_unsafe_resolution_paths() {
    let root = test_root("catalog_rejects_unsafe_resolution_paths");
    let registry = ComfyModelFolderRegistry::new(root.join("assets"));
    let catalog = ComfyModelCatalog::new(&registry);
    let error = catalog
        .resolve_at_root(ModelCategory::Checkpoints, 0, "../outside.safetensors")
        .expect_err("unsafe path rejected");

    assert!(matches!(
        error,
        ModelCatalogError::Folder(ModelFolderError::UnsafeRelativePath { .. })
    ));

    cleanup_root(&root);
}

#[test]
fn catalog_reports_missing_readable_root_errors() {
    let root = test_root("catalog_reports_missing_readable_root_errors");
    let blocking_file = root.join("assets/models/checkpoints");
    create_file(&blocking_file, b"not a directory");

    let registry = ComfyModelFolderRegistry::new(root.join("assets"));
    let catalog = ComfyModelCatalog::new(&registry);
    let error = catalog
        .list_category("checkpoints")
        .expect_err("file root cannot be listed");

    assert!(matches!(error, ModelCatalogError::Io { .. }));

    cleanup_root(&root);
}

fn create_file(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(path, contents).expect("write test file");
}

fn test_root(name: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after unix epoch")
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("world-model-{name}-{}-{now}", std::process::id()));
    fs::create_dir_all(&root).expect("create test root");
    root
}

fn cleanup_root(root: &Path) {
    match fs::remove_dir_all(root) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("failed to remove test root `{}`: {error}", root.display()),
    }
}
