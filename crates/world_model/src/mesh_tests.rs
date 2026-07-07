use crate::mesh::{
    BackendOptions, MeshArtifactMetadata, MeshBackend, MeshFormat, MeshGenerationRequest,
    TextureOptions,
};

// ---------------------------------------------------------------------------
// MeshBackend
// ---------------------------------------------------------------------------

#[test]
fn mesh_backend_labels() {
    assert_eq!(MeshBackend::Python.label(), "python");
    assert_eq!(MeshBackend::RemoteApi.label(), "remote_api");
    assert_eq!(MeshBackend::Native.label(), "native");
    assert_eq!(MeshBackend::Automatic.label(), "automatic");
}

#[test]
fn mesh_backend_dependency_review() {
    assert!(MeshBackend::Native.requires_dependency_review());
    assert!(!MeshBackend::Python.requires_dependency_review());
    assert!(!MeshBackend::RemoteApi.requires_dependency_review());
    assert!(!MeshBackend::Automatic.requires_dependency_review());
}

// ---------------------------------------------------------------------------
// MeshFormat
// ---------------------------------------------------------------------------

#[test]
fn mesh_format_extensions() {
    assert_eq!(MeshFormat::Obj.extension(), "obj");
    assert_eq!(MeshFormat::Glb.extension(), "glb");
    assert_eq!(MeshFormat::Gltf.extension(), "gltf");
    assert_eq!(MeshFormat::Fbx.extension(), "fbx");
    assert_eq!(MeshFormat::Ply.extension(), "ply");
    assert_eq!(MeshFormat::Usd.extension(), "usd");
    assert_eq!(MeshFormat::Abc.extension(), "abc");
    assert_eq!(MeshFormat::Stl.extension(), "stl");
}

#[test]
fn mesh_format_labels() {
    assert_eq!(MeshFormat::Obj.label(), "Wavefront OBJ");
    assert_eq!(MeshFormat::Glb.label(), "glTF Binary");
    assert_eq!(MeshFormat::Stl.label(), "STL");
}

#[test]
fn mesh_format_dependency_review() {
    assert!(MeshFormat::Fbx.requires_dependency_review());
    assert!(MeshFormat::Usd.requires_dependency_review());
    assert!(MeshFormat::Abc.requires_dependency_review());
    assert!(!MeshFormat::Obj.requires_dependency_review());
    assert!(!MeshFormat::Glb.requires_dependency_review());
    assert!(!MeshFormat::Ply.requires_dependency_review());
    assert!(!MeshFormat::Stl.requires_dependency_review());
    assert!(!MeshFormat::Gltf.requires_dependency_review());
}

// ---------------------------------------------------------------------------
// TextureOptions
// ---------------------------------------------------------------------------

#[test]
fn texture_options_default() {
    let opts = TextureOptions::default();
    assert!(!opts.enabled);
    assert!(opts.resolution.is_none());
    assert!(!opts.bake_ao);
    assert!(!opts.normal_map);
    assert!(!opts.pbr_maps);
    assert!(opts.extra.is_empty());
}

#[test]
fn texture_options_configured() {
    let opts = TextureOptions {
        enabled: true,
        resolution: Some(2048),
        bake_ao: true,
        normal_map: true,
        pbr_maps: true,
        extra: std::collections::HashMap::new(),
    };
    assert!(opts.enabled);
    assert_eq!(opts.resolution, Some(2048));
    assert!(opts.bake_ao);
    assert!(opts.normal_map);
    assert!(opts.pbr_maps);
}

// ---------------------------------------------------------------------------
// BackendOptions
// ---------------------------------------------------------------------------

#[test]
fn backend_options_default() {
    let opts = BackendOptions::default();
    assert!(opts.model.is_none());
    assert!(opts.params.is_empty());
}

#[test]
fn backend_options_configured() {
    let mut params = std::collections::HashMap::new();
    params.insert("steps".to_string(), "50".to_string());
    let opts = BackendOptions {
        model: Some("tripo-sr-v2".to_string()),
        params,
    };
    assert_eq!(opts.model.as_deref(), Some("tripo-sr-v2"));
    assert_eq!(opts.params.get("steps").map(String::as_str), Some("50"));
}

// ---------------------------------------------------------------------------
// MeshGenerationRequest
// ---------------------------------------------------------------------------

#[test]
fn mesh_request_creates_with_prompt() {
    let req = MeshGenerationRequest::new("a wooden chair");
    assert_eq!(req.prompt, "a wooden chair");
    assert!(req.reference_image.is_none());
    assert_eq!(req.target_format, MeshFormat::Obj);
    assert!(!req.textures.enabled);
    assert_eq!(req.backend, MeshBackend::Automatic);
}

#[test]
fn mesh_request_with_all_options() {
    let req = MeshGenerationRequest::new("a stone statue")
        .with_reference_image("inputs/ref.png")
        .with_target_format(MeshFormat::Glb)
        .with_textures(TextureOptions {
            enabled: true,
            resolution: Some(1024),
            bake_ao: true,
            normal_map: false,
            pbr_maps: true,
            extra: std::collections::HashMap::new(),
        })
        .with_backend(MeshBackend::Python)
        .with_backend_options(BackendOptions {
            model: Some("tripo-sr".to_string()),
            params: std::collections::HashMap::new(),
        });
    assert_eq!(req.reference_image.as_deref(), Some("inputs/ref.png"));
    assert_eq!(req.target_format, MeshFormat::Glb);
    assert_eq!(req.backend, MeshBackend::Python);
    assert!(req.textures.enabled);
    assert_eq!(req.backend_options.model.as_deref(), Some("tripo-sr"));
}

// ---------------------------------------------------------------------------
// MeshArtifactMetadata
// ---------------------------------------------------------------------------

#[test]
fn mesh_artifact_creates_with_path_and_format() {
    let meta = MeshArtifactMetadata::new("outputs/chair.obj", MeshFormat::Obj);
    assert_eq!(meta.mesh_path, "outputs/chair.obj");
    assert_eq!(meta.format, MeshFormat::Obj);
    assert!(meta.preview_path.is_none());
    assert!(!meta.has_textures);
}

#[test]
fn mesh_artifact_with_preview_export_and_provenance() {
    let meta = MeshArtifactMetadata::new("outputs/statue.glb", MeshFormat::Glb)
        .with_preview("previews/statue.png")
        .with_export("exports/statue.fbx", MeshFormat::Fbx)
        .with_provenance("prov-001")
        .with_source_asset("inputs/ref.png")
        .with_triangle_count(12000)
        .with_vertex_count(6000)
        .with_textures(2048);
    assert_eq!(meta.preview_path.as_deref(), Some("previews/statue.png"));
    assert_eq!(meta.export_path.as_deref(), Some("exports/statue.fbx"));
    assert_eq!(meta.export_format, Some(MeshFormat::Fbx));
    assert_eq!(meta.provenance_id.as_deref(), Some("prov-001"));
    assert_eq!(meta.source_assets, vec!["inputs/ref.png".to_string()]);
    assert_eq!(meta.triangle_count, Some(12000));
    assert_eq!(meta.vertex_count, Some(6000));
    assert!(meta.has_textures);
    assert_eq!(meta.texture_resolution, Some(2048));
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

#[test]
fn mesh_request_round_trip_serde() {
    let req = MeshGenerationRequest::new("a red sports car")
        .with_backend(MeshBackend::Python)
        .with_target_format(MeshFormat::Glb);
    let json = serde_json::to_string(&req).expect("serialize");
    let restored: MeshGenerationRequest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.prompt, req.prompt);
    assert_eq!(restored.backend, req.backend);
    assert_eq!(restored.target_format, req.target_format);
}

#[test]
fn mesh_artifact_round_trip_serde() {
    let meta = MeshArtifactMetadata::new("out.obj", MeshFormat::Obj)
        .with_preview("preview.png")
        .with_textures(1024);
    let json = serde_json::to_string(&meta).expect("serialize");
    let restored: MeshArtifactMetadata = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.mesh_path, meta.mesh_path);
    assert_eq!(restored.has_textures, true);
    assert_eq!(restored.texture_resolution, Some(1024));
}
