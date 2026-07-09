use serde_json::json;

use crate::{
    SimSourceDiagnostic, SimSourceDiagnosticSeverity, SimSourceExtractionStatus,
    SimSourceInventory, SimSourceItem, SimSourceKind,
};

#[test]
fn inventory_round_trips_representative_comfy_feature_surfaces() {
    let items = vec![
        SimSourceItem::classified(SimSourceKind::Route, "server.py", "POST /prompt")
            .with_category("runtime")
            .with_metadata(json!({"method": "POST", "path": "/prompt"})),
        SimSourceItem::classified(SimSourceKind::WebSocketProtocol, "server.py", "progress")
            .with_category("runtime")
            .with_metadata(json!({"event": "progress"})),
        SimSourceItem::classified(SimSourceKind::CoreNode, "nodes.py", "KSampler")
            .with_category("sampling")
            .with_metadata(json!({"outputs": ["LATENT"]})),
        SimSourceItem::classified(
            SimSourceKind::ExtraNode,
            "comfy_extras/nodes_video.py",
            "LoadVideo",
        )
        .with_category("video"),
        SimSourceItem::classified(
            SimSourceKind::ApiProviderNode,
            "comfy_api_nodes/nodes_openai.py",
            "OpenAIImageNode",
        )
        .with_category("api/openai"),
        SimSourceItem::classified(
            SimSourceKind::ModelFamily,
            "comfy/supported_models.py",
            "SDXL",
        )
        .with_metadata(json!({"latent_format": "sdxl"})),
        SimSourceItem::classified(SimSourceKind::ModelFolder, "folder_paths.py", "checkpoints"),
        SimSourceItem::classified(
            SimSourceKind::Blueprint,
            "blueprints/image-to-video.json",
            "image-to-video",
        ),
        SimSourceItem::classified(
            SimSourceKind::AssetApi,
            "app/assets/routes.py",
            "GET /api/assets",
        ),
        SimSourceItem::classified(
            SimSourceKind::ExtensionHook,
            "custom_nodes/example/__init__.py",
            "NODE_CLASS_MAPPINGS",
        ),
        SimSourceItem::classified(SimSourceKind::CliFlag, "main.py", "--listen"),
        SimSourceItem::classified(
            SimSourceKind::OpenApiOperation,
            "api_server/openapi.yaml",
            "getQueue",
        ),
        SimSourceItem::classified(
            SimSourceKind::TestSurface,
            "tests-unit/test_execution.py",
            "test_execution",
        ),
        SimSourceItem::classified(
            SimSourceKind::PackagingSurface,
            ".github/workflows/test.yml",
            "ci",
        ),
        SimSourceItem::classified(
            SimSourceKind::FrontendSurface,
            "web/package.json",
            "frontend-package",
        ),
    ];

    let inventory = SimSourceInventory::new(
        1,
        "/Users/ahmad.vegah/repos/projects/sim/projects/comfy",
        "2026-07-09",
        items,
    );
    let json = serde_json::to_string(&inventory).expect("serialize inventory");
    let restored: SimSourceInventory = serde_json::from_str(&json).expect("deserialize inventory");

    assert_eq!(restored, inventory);
    assert_eq!(restored.summary.total_items, 15);
    assert_eq!(restored.summary.count_for_kind(SimSourceKind::Route), 1);
    assert_eq!(
        restored
            .summary
            .count_for_status(SimSourceExtractionStatus::Classified),
        15
    );
}

#[test]
fn inventory_preserves_unclassified_paths_with_diagnostics() {
    let source_path = "comfy_extras/nodes_unusual.py";
    let item = SimSourceItem::unclassified(source_path, "unsupported ast pattern");
    let diagnostic = SimSourceDiagnostic::warning(
        "world_model.sim_inventory.unclassified",
        source_path,
        "unsupported AST pattern",
    );
    let inventory = SimSourceInventory::new(1, "projects/comfy", "2026-07-09", [item.clone()])
        .with_diagnostic(diagnostic.clone());

    assert_eq!(item.source_kind, SimSourceKind::Unknown);
    assert_eq!(
        item.extraction_status,
        SimSourceExtractionStatus::Unclassified
    );
    assert_eq!(inventory.diagnostics, vec![diagnostic]);
    assert_eq!(
        inventory
            .summary
            .count_for_status(SimSourceExtractionStatus::Unclassified),
        1
    );
    assert_eq!(
        inventory.diagnostics[0].severity,
        SimSourceDiagnosticSeverity::Warning
    );
}
