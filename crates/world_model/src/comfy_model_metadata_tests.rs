use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    ComfyModelCatalog, ComfyModelFolderRegistry, ComfyModelMetadataReader, ModelCategory,
    ModelMetadataError,
};

#[test]
fn metadata_reader_exposes_adjacent_preview_through_safe_route() {
    let root = test_root("metadata_reader_exposes_adjacent_preview_through_safe_route");
    let models = root.join("assets/models/checkpoints");
    create_file(
        &models.join("sdxl/base model.safetensors"),
        &safetensors_file("{}"),
    );
    create_file(&models.join("sdxl/base model.png"), b"preview");

    let file = checkpoint_file(&root, "sdxl/base model.safetensors");
    let preview = ComfyModelMetadataReader::new()
        .preview_for_file(&file)
        .expect("preview read succeeds")
        .expect("preview exists");

    assert_eq!(preview.category, ModelCategory::Checkpoints);
    assert_eq!(preview.root_index, 0);
    assert_eq!(
        preview.preview_relative_path,
        PathBuf::from("sdxl/base model.png")
    );
    assert_eq!(preview.content_type, "image/png");
    assert_eq!(
        preview.route_path,
        "/world-model/models/checkpoints/0/previews/sdxl/base%20model.png"
    );

    cleanup_root(&root);
}

#[test]
fn metadata_reader_falls_back_to_preview_suffix() {
    let root = test_root("metadata_reader_falls_back_to_preview_suffix");
    let models = root.join("assets/models/vae");
    create_file(&models.join("base.safetensors"), &safetensors_file("{}"));
    create_file(&models.join("base.preview.webp"), b"preview");

    let registry = ComfyModelFolderRegistry::new(root.join("assets"));
    let catalog = ComfyModelCatalog::new(&registry);
    let file = catalog
        .resolve_at_root(ModelCategory::Vae, 0, "base.safetensors")
        .expect("model resolves");
    let preview = ComfyModelMetadataReader::new()
        .preview_for_file(&file)
        .expect("preview read succeeds")
        .expect("preview exists");

    assert_eq!(
        preview.preview_relative_path,
        PathBuf::from("base.preview.webp")
    );
    assert_eq!(preview.content_type, "image/webp");

    cleanup_root(&root);
}

#[test]
fn metadata_reader_extracts_safetensors_header_without_weights() {
    let root = test_root("metadata_reader_extracts_safetensors_header_without_weights");
    let models = root.join("assets/models/checkpoints");
    create_file(
        &models.join("base.safetensors"),
        &safetensors_file(
            r#"{
                "__metadata__": {
                    "format": "pt",
                    "modelspec.architecture": "stable-diffusion-xl-v1-base"
                },
                "model.diffusion_model.input_blocks.0.0.weight": {
                    "dtype": "F16",
                    "shape": [320, 4, 3, 3],
                    "data_offsets": [0, 23040]
                },
                "first_stage_model.decoder.conv_out.weight": {
                    "dtype": "F16",
                    "shape": [3, 128, 3, 3],
                    "data_offsets": [23040, 29952]
                }
            }"#,
        ),
    );

    let file = checkpoint_file(&root, "base.safetensors");
    let metadata = ComfyModelMetadataReader::new()
        .safetensors_metadata_for_file(&file)
        .expect("metadata read succeeds")
        .expect("safetensors metadata exists");

    assert_eq!(metadata.tensor_count, 2);
    assert_eq!(
        metadata.metadata,
        BTreeMap::from([
            ("format".to_string(), "pt".to_string()),
            (
                "modelspec.architecture".to_string(),
                "stable-diffusion-xl-v1-base".to_string()
            )
        ])
    );
    assert!(metadata.header_byte_len > 0);

    cleanup_root(&root);
}

#[test]
fn metadata_reader_skips_safetensors_metadata_for_other_model_formats() {
    let root = test_root("metadata_reader_skips_safetensors_metadata_for_other_model_formats");
    let models = root.join("assets/models/checkpoints");
    create_file(&models.join("base.ckpt"), b"checkpoint");

    let file = checkpoint_file(&root, "base.ckpt");
    let metadata = ComfyModelMetadataReader::new()
        .safetensors_metadata_for_file(&file)
        .expect("metadata read succeeds");

    assert_eq!(metadata, None);

    cleanup_root(&root);
}

#[test]
fn metadata_reader_rejects_safetensors_headers_over_limit() {
    let root = test_root("metadata_reader_rejects_safetensors_headers_over_limit");
    let models = root.join("assets/models/checkpoints");
    let model_path = models.join("base.safetensors");
    create_file(&model_path, &12_u64.to_le_bytes());

    let file = checkpoint_file(&root, "base.safetensors");
    let error = ComfyModelMetadataReader::new()
        .with_safetensors_header_limit_bytes(4)
        .safetensors_metadata_for_file(&file)
        .expect_err("oversize header rejected");

    assert!(matches!(
        error,
        ModelMetadataError::HeaderTooLarge {
            header_byte_len: 12,
            limit_bytes: 4,
            ..
        }
    ));

    cleanup_root(&root);
}

#[test]
fn metadata_reader_combines_preview_and_safetensors_summary() {
    let root = test_root("metadata_reader_combines_preview_and_safetensors_summary");
    let models = root.join("assets/models/checkpoints");
    create_file(
        &models.join("base.safetensors"),
        &safetensors_file(r#"{"__metadata__":{"format":"pt"}}"#),
    );
    create_file(&models.join("base.jpg"), b"preview");

    let file = checkpoint_file(&root, "base.safetensors");
    let summary = ComfyModelMetadataReader::new()
        .read_summary(&file)
        .expect("summary read succeeds");

    assert_eq!(
        summary
            .preview
            .expect("preview exists")
            .preview_relative_path,
        PathBuf::from("base.jpg")
    );
    assert_eq!(
        summary
            .safetensors
            .expect("safetensors metadata exists")
            .metadata
            .get("format"),
        Some(&"pt".to_string())
    );

    cleanup_root(&root);
}

fn checkpoint_file(root: &Path, relative_path: &str) -> crate::ModelFileRef {
    let registry = ComfyModelFolderRegistry::new(root.join("assets"));
    let catalog = ComfyModelCatalog::new(&registry);
    catalog
        .resolve_at_root(ModelCategory::Checkpoints, 0, relative_path)
        .expect("model resolves")
}

fn safetensors_file(header: &str) -> Vec<u8> {
    let header_bytes = header.as_bytes();
    let header_len = u64::try_from(header_bytes.len()).expect("test header length fits u64");
    let mut file = header_len.to_le_bytes().to_vec();
    file.extend_from_slice(header_bytes);
    file.extend_from_slice(b"weights-not-read");
    file
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
