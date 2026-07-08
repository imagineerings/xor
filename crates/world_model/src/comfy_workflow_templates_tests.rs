use serde_json::json;

use crate::{
    ComfyWorkflowTemplateAdapter, ComfyWorkflowTemplateAsset, ComfyWorkflowTemplateId,
    DUPLICATE_WORKFLOW_TEMPLATE_CODE, UNSAFE_WORKFLOW_TEMPLATE_PATH_CODE,
    WORKFLOW_TEMPLATE_NOT_FOUND_CODE,
};

#[test]
fn workflow_template_adapter_lists_custom_node_templates_as_native_records() {
    let mut adapter = ComfyWorkflowTemplateAdapter::default();
    let id = adapter
        .register_custom_node_template(
            "pack-a",
            "Image Starter",
            "custom_nodes/pack-a/example_workflows/image_starter.json",
            json!({"nodes": [{"id": 1, "type": "CheckpointLoaderSimple"}], "links": []}),
            vec![ComfyWorkflowTemplateAsset::new(
                "preview",
                "custom_nodes/pack-a/example_workflows/preview.webp",
                "image/webp",
            )],
            json!({"category": "image", "token": "hidden"}),
        )
        .expect("template should register");

    let listings = adapter.listings();
    assert_eq!(listings.len(), 1);
    assert_eq!(listings[0].id, id);
    assert_eq!(listings[0].name, "Image Starter");
    assert_eq!(listings[0].node_pack_name, "pack-a");
    assert_eq!(
        listings[0].template_path,
        "custom_nodes/pack-a/example_workflows/image_starter.json"
    );
    assert_eq!(listings[0].static_assets[0].content_type, "image/webp");
    assert_eq!(listings[0].metadata["category"], "image");
    assert!(listings[0].metadata.get("token").is_none());
}

#[test]
fn workflow_template_open_returns_full_graph_data() {
    let mut adapter = ComfyWorkflowTemplateAdapter::default();
    let id = adapter
        .register_custom_node_template(
            "pack-a",
            "Video Starter",
            "custom_nodes/pack-a/example_workflows/video_starter.json",
            json!({"nodes": [{"id": 3, "type": "KSampler"}], "links": [[9, 1, 0, 3, 0, "MODEL"]]}),
            Vec::new(),
            json!({}),
        )
        .expect("template should register");

    let record = adapter.open(&id).expect("template should open");

    assert_eq!(record.name, "Video Starter");
    assert_eq!(record.source.node_pack_name(), "pack-a");
    assert_eq!(record.graph_json["nodes"][0]["type"], "KSampler");
    assert_eq!(record.graph_json["links"][0][0], 9);
}

#[test]
fn workflow_template_ids_are_stable_for_node_pack_and_path() {
    let id = ComfyWorkflowTemplateId::from_custom_node_template(
        "pack-a",
        "custom_nodes/pack-a/example_workflows/image_starter.json",
    );
    let same = ComfyWorkflowTemplateId::from_custom_node_template(
        "pack-a",
        "custom_nodes/pack-a/example_workflows/image_starter.json",
    );
    let different_pack = ComfyWorkflowTemplateId::from_custom_node_template(
        "pack-b",
        "custom_nodes/pack-a/example_workflows/image_starter.json",
    );

    assert_eq!(id, same);
    assert_eq!(
        id.as_str(),
        "workflow-template-custom-node-21b0241c9fee16c3"
    );
    assert_ne!(id, different_pack);
}

#[test]
fn workflow_template_adapter_rejects_unsafe_template_and_asset_paths() {
    let mut adapter = ComfyWorkflowTemplateAdapter::default();

    assert!(
        adapter
            .register_custom_node_template(
                "pack-a",
                "Unsafe Template",
                "../outside.json",
                json!({"nodes": [], "links": []}),
                Vec::new(),
                json!({}),
            )
            .is_none()
    );
    assert!(
        adapter
            .register_custom_node_template(
                "pack-a",
                "Unsafe Asset",
                "custom_nodes/pack-a/example_workflows/safe.json",
                json!({"nodes": [], "links": []}),
                vec![ComfyWorkflowTemplateAsset::new(
                    "preview",
                    "/tmp/preview.webp",
                    "image/webp",
                )],
                json!({}),
            )
            .is_none()
    );

    assert_eq!(adapter.len(), 0);
    assert_eq!(
        adapter
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == UNSAFE_WORKFLOW_TEMPLATE_PATH_CODE)
            .count(),
        2
    );
}

#[test]
fn workflow_template_adapter_reports_duplicate_and_missing_records() {
    let mut adapter = ComfyWorkflowTemplateAdapter::default();
    for name in ["first", "second"] {
        adapter.register_custom_node_template(
            "pack-a",
            name,
            "custom_nodes/pack-a/example_workflows/reused.json",
            json!({"nodes": [], "links": []}),
            Vec::new(),
            json!({}),
        );
    }

    assert_eq!(adapter.len(), 1);
    assert!(adapter.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == DUPLICATE_WORKFLOW_TEMPLATE_CODE
            && diagnostic
                .template_id
                .as_ref()
                .is_some_and(|id| id.as_str().starts_with("workflow-template-custom-node-"))
    }));

    let missing = adapter
        .open(&ComfyWorkflowTemplateId::from_custom_node_template(
            "pack-a",
            "custom_nodes/pack-a/example_workflows/missing.json",
        ))
        .expect_err("missing template should fail");
    assert_eq!(missing.code, WORKFLOW_TEMPLATE_NOT_FOUND_CODE);
}
