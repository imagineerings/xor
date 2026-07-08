use serde_json::json;

use crate::{
    ComfyWorkflowApiExporter, ComfyWorkflowDocument, ComfyWorkflowId, ComfyWorkflowSource,
    ComfyWorkflowStore, INVALID_WORKFLOW_GRAPH_CODE, WORKFLOW_NOT_FOUND_CODE,
};

#[test]
fn workflow_document_preserves_ui_metadata_source_and_default_view() {
    let document = ComfyWorkflowDocument::from_graph_json(
        "smoke",
        json!({
            "nodes": [],
            "links": [],
            "extra": {
                "ds": {
                    "offset": [120, -40],
                    "scale": 1.25
                },
                "workflow_id": "source-workflow"
            }
        }),
        ComfyWorkflowSource::Blueprint {
            source_path: "projects/comfy/blueprints/Text to Image.json".to_string(),
        },
    )
    .with_id(ComfyWorkflowId::new("workflow-a"))
    .with_provenance_artifact("artifact:image-1");

    assert_eq!(document.id.as_ref().unwrap().as_str(), "workflow-a");
    assert_eq!(document.default_view.x, 120);
    assert_eq!(document.default_view.y, -40);
    assert_eq!(document.default_view.scale_millis, 1250);
    assert_eq!(
        document.ui_metadata["workflow_id"],
        serde_json::Value::String("source-workflow".to_string())
    );
    assert_eq!(
        document.provenance_artifact_id.as_deref(),
        Some("artifact:image-1")
    );
}

#[test]
fn workflow_store_saves_versions_and_loads_latest() {
    let mut store = ComfyWorkflowStore::default();
    let workflow_id = ComfyWorkflowId::new("workflow-a");

    let first = store.save(
        ComfyWorkflowDocument::from_graph_json(
            "first",
            json!({"nodes": [], "links": []}),
            ComfyWorkflowSource::User {
                project_path: "workflows/smoke.json".to_string(),
            },
        )
        .with_id(workflow_id.clone()),
    );
    let second = store.save(
        ComfyWorkflowDocument::from_graph_json(
            "second",
            json!({"nodes": [], "links": []}),
            ComfyWorkflowSource::User {
                project_path: "workflows/smoke.json".to_string(),
            },
        )
        .with_id(workflow_id.clone()),
    );

    assert_eq!(first.version, 1);
    assert_eq!(second.version, 2);
    assert_eq!(store.version_count(&workflow_id), 2);
    assert_eq!(store.load(&workflow_id).unwrap().name, "second");
    assert_eq!(store.load_version(&first).unwrap().name, "first");
}

#[test]
fn workflow_store_reports_missing_workflows() {
    let store = ComfyWorkflowStore::default();
    let diagnostic = store
        .load(&ComfyWorkflowId::new("missing"))
        .expect_err("missing workflow should fail");

    assert_eq!(diagnostic.code, WORKFLOW_NOT_FOUND_CODE);
}

#[test]
fn workflow_export_emits_comfy_api_prompt_graph() {
    let document = ComfyWorkflowDocument::from_graph_json(
        "api",
        json!({
            "nodes": [
                {
                    "id": 1,
                    "type": "CheckpointLoaderSimple",
                    "inputs": [
                        {"name": "ckpt_name", "type": "COMBO", "widget": {"name": "ckpt_name"}, "link": null}
                    ],
                    "widgets_values": ["sdxl.safetensors"]
                },
                {
                    "id": 2,
                    "type": "KSampler",
                    "inputs": [
                        {"name": "model", "type": "MODEL", "link": 10},
                        {"name": "seed", "type": "INT", "widget": {"name": "seed"}, "link": null}
                    ],
                    "widgets_values": [42]
                }
            ],
            "links": [
                [10, 1, 0, 2, 0, "MODEL"]
            ]
        }),
        ComfyWorkflowSource::Imported {
            source_path: "fixture.json".to_string(),
        },
    );

    let prompt =
        ComfyWorkflowApiExporter::export_api_prompt(&document).expect("workflow should export");

    assert_eq!(prompt["1"]["class_type"], "CheckpointLoaderSimple");
    assert_eq!(prompt["1"]["inputs"]["ckpt_name"], "sdxl.safetensors");
    assert_eq!(prompt["2"]["class_type"], "KSampler");
    assert_eq!(prompt["2"]["inputs"]["model"], json!([1, 0]));
    assert_eq!(prompt["2"]["inputs"]["seed"], 42);
}

#[test]
fn workflow_export_reports_invalid_graphs_without_silent_partial_success() {
    let document = ComfyWorkflowDocument::from_graph_json(
        "bad",
        json!({"links": []}),
        ComfyWorkflowSource::Imported {
            source_path: "bad.json".to_string(),
        },
    );

    let diagnostics = ComfyWorkflowApiExporter::export_api_prompt(&document)
        .expect_err("invalid graph should fail");

    assert_eq!(diagnostics[0].code, INVALID_WORKFLOW_GRAPH_CODE);
}
