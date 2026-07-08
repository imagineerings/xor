use serde_json::json;

use crate::{
    ComfyAppModeBridge, ComfyAppModeControlKind, ComfyAppModeUiOwner, ComfyWorkflowDocument,
    ComfyWorkflowSource, INVALID_APP_MODE_CONTROL_CODE, INVALID_APP_MODE_METADATA_CODE,
};

#[test]
fn app_mode_bridge_exposes_controls_for_unified_authoring() {
    let workflow = workflow_with_graph(json!({
        "nodes": [{"id": 1, "type": "CLIPTextEncode"}],
        "links": [],
        "extra": {
            "app_mode": {
                "title": "Prompt app",
                "controls": [
                    {
                        "id": "prompt",
                        "label": "Prompt",
                        "kind": "text",
                        "node_id": 1,
                        "input": "text",
                        "default": "a castle"
                    }
                ]
            }
        }
    }));

    let report = ComfyAppModeBridge.expose(&workflow);

    assert_eq!(report.workflow_name, "workflow");
    assert_eq!(report.title.as_deref(), Some("Prompt app"));
    assert!(report.available_as_graph_workflow);
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.controls.len(), 1);
    assert_eq!(report.controls[0].id, "prompt");
    assert_eq!(report.controls[0].kind, ComfyAppModeControlKind::Text);
    assert_eq!(report.controls[0].target.node_id, 1);
    assert_eq!(report.controls[0].target.input_name, "text");
    assert_eq!(
        report.controls[0].ui_owner,
        ComfyAppModeUiOwner::UnifiedAuthoringApp
    );
}

#[test]
fn app_mode_bridge_preserves_choices_defaults_and_graph_fallback() {
    let workflow = workflow_with_graph(json!({
        "nodes": [{"id": 2, "type": "KSampler"}],
        "links": [],
        "extra": {
            "app": {
                "controls": [
                    {
                        "id": "sampler",
                        "type": "choice",
                        "node": 2,
                        "input_name": "sampler_name",
                        "choices": ["euler", "ddim"],
                        "default": "euler"
                    },
                    {
                        "id": "seed",
                        "kind": "seed",
                        "node_id": 2,
                        "input": "seed",
                        "default": 42
                    }
                ]
            }
        }
    }));

    let report = ComfyAppModeBridge.expose(&workflow);

    assert!(report.available_as_graph_workflow);
    assert_eq!(report.controls[0].kind, ComfyAppModeControlKind::Choice);
    assert_eq!(report.controls[0].choices, vec!["euler", "ddim"]);
    assert_eq!(report.controls[0].default_value, Some(json!("euler")));
    assert_eq!(report.controls[1].kind, ComfyAppModeControlKind::Seed);
    assert_eq!(report.controls[1].default_value, Some(json!(42)));
}

#[test]
fn app_mode_bridge_keeps_plain_workflow_available_as_graph() {
    let workflow = workflow_with_graph(json!({"nodes": [], "links": []}));

    let report = ComfyAppModeBridge.expose(&workflow);

    assert!(report.controls.is_empty());
    assert!(report.available_as_graph_workflow);
    assert!(report.diagnostics.is_empty());
}

#[test]
fn app_mode_bridge_reports_invalid_metadata_without_hiding_graph_workflow() {
    let workflow = workflow_with_graph(json!({
        "nodes": [],
        "links": [],
        "extra": {"app_mode": {"title": "Broken"}}
    }));

    let report = ComfyAppModeBridge.expose(&workflow);

    assert!(report.controls.is_empty());
    assert!(report.available_as_graph_workflow);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == INVALID_APP_MODE_METADATA_CODE })
    );
}

#[test]
fn app_mode_bridge_reports_invalid_controls_nonfatally() {
    let workflow = workflow_with_graph(json!({
        "nodes": [],
        "links": [],
        "extra": {
            "app_mode": {
                "controls": [
                    {"label": "Missing id"},
                    {"id": "missing-node", "input": "text"},
                    {"id": "missing-input", "node_id": 1}
                ]
            }
        }
    }));

    let report = ComfyAppModeBridge.expose(&workflow);

    assert!(report.controls.is_empty());
    assert!(report.available_as_graph_workflow);
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == INVALID_APP_MODE_CONTROL_CODE)
            .count(),
        3
    );
}

fn workflow_with_graph(graph_json: serde_json::Value) -> ComfyWorkflowDocument {
    ComfyWorkflowDocument::from_graph_json(
        "workflow",
        graph_json,
        ComfyWorkflowSource::Imported {
            source_path: "workflow.json".to_string(),
        },
    )
}
