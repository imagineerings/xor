use crate::SimGeneratedAssetRegistry;
use world_model::{MeshArtifactMetadata, MeshFormat};

#[test]
fn generated_asset_registry_requires_mesh_provenance() {
    let mut registry = SimGeneratedAssetRegistry::new();

    let diagnostic = registry
        .register_mesh(MeshArtifactMetadata::new(
            "outputs/tree.glb",
            MeshFormat::Glb,
        ))
        .expect_err("missing provenance should fail");

    assert_eq!(
        diagnostic.code,
        "sim_game.generated_asset.missing_provenance"
    );
}

#[test]
fn generated_asset_registry_preserves_preview_export_and_provenance() {
    let mut registry = SimGeneratedAssetRegistry::new();
    let metadata = MeshArtifactMetadata::new("outputs/tree.glb", MeshFormat::Glb)
        .with_preview("previews/tree.png")
        .with_export("exports/tree.obj", MeshFormat::Obj)
        .with_source_asset("inputs/tree.png")
        .with_provenance("prov-tree");

    let record = registry.register_mesh(metadata).expect("registered asset");

    assert_eq!(record.asset_path, "outputs/tree.glb");
    assert_eq!(record.preview_path.as_deref(), Some("previews/tree.png"));
    assert_eq!(record.export_format, Some(MeshFormat::Obj));
    assert_eq!(record.provenance_id, "prov-tree");
    assert_eq!(registry.assets().len(), 1);
}
