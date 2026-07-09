use crate::{
    SIM_AUTHORING_PREVIEW_WORKER_UNAVAILABLE_CODE, SimAuthoringItem, SimAuthoringPreviewKind,
    SimAuthoringRouteKind, SimGameAuthoringApp,
};
use sim_game::SimGeneratedAssetRecord;
use world_model::{
    ArtifactRecord, ArtifactType, GeneratedWorldArtifact, GenerationProvenance,
    WorldGenerationRequest, WorldModelProfile,
};

#[test]
fn game_authoring_app_routes_typed_items_to_surfaces() {
    let app = SimGameAuthoringApp::new();

    let graph_route = app.route_item(&SimAuthoringItem::diffusion_graph(
        "graph-1",
        "Intro generation",
    ));
    let task_route = app.route_item(&SimAuthoringItem::run_export_task("run-debug", "Run"));

    assert_eq!(graph_route.route_kind, SimAuthoringRouteKind::GraphEditor);
    assert_eq!(task_route.route_kind, SimAuthoringRouteKind::TaskView);
}

#[test]
fn game_authoring_app_surfaces_generated_assets_as_items() {
    let mut app = SimGameAuthoringApp::new();

    app.register_generated_asset(SimGeneratedAssetRecord {
        asset_path: "outputs/tree.glb".to_string(),
        format: world_model::MeshFormat::Glb,
        preview_path: Some("previews/tree.png".to_string()),
        export_path: None,
        export_format: None,
        provenance_id: "prov-tree".to_string(),
        source_assets: Vec::new(),
    });

    assert_eq!(app.generated_assets.len(), 1);
    assert_eq!(
        app.items[0].kind,
        crate::SimAuthoringItemKind::GeneratedArtifact
    );
}

#[test]
fn game_authoring_preview_requires_worker_diagnostics() {
    let app = SimGameAuthoringApp::new();
    let generated = generated_mesh_artifact();

    let diagnostic = app
        .preview_generated_artifact(&generated, false)
        .expect_err("worker diagnostics are required");

    assert_eq!(
        diagnostic.code,
        SIM_AUTHORING_PREVIEW_WORKER_UNAVAILABLE_CODE
    );
}

#[test]
fn game_authoring_preview_preserves_generated_artifact_provenance() {
    let app = SimGameAuthoringApp::new();
    let generated = generated_mesh_artifact();

    let route = app
        .preview_generated_artifact(&generated, true)
        .expect("preview route");

    assert_eq!(route.preview_kind, SimAuthoringPreviewKind::Mesh);
    assert_eq!(route.artifact_path, "outputs/tree.glb");
    assert_eq!(
        route.provenance_backend.as_deref(),
        Some("native-sim-worker")
    );
}

fn generated_mesh_artifact() -> GeneratedWorldArtifact {
    let artifact = ArtifactRecord::new("outputs/tree.glb", ArtifactType::Mesh)
        .with_preview("previews/tree.png");
    let request = WorldGenerationRequest::new(
        "tree",
        WorldModelProfile::new("native-sim", "mesh"),
        "outputs/tree.glb",
    )
    .with_controls(vec![world_model::WorldControl::new(
        vec![world_model::WorldActionControl::new("w", 1.0, 0)],
        1,
    )]);
    let provenance = GenerationProvenance::new(request)
        .with_artifact(artifact.clone())
        .with_backend("native-sim-worker")
        .with_workflow("authoring-preview");
    GeneratedWorldArtifact::new(artifact, provenance).expect("generated artifact")
}
