use serde::{Deserialize, Serialize};

use crate::ComfyWorkflowDocument;

pub const INVALID_APP_MODE_METADATA_CODE: &str = "world_model.comfy_app_mode.invalid_metadata";
pub const INVALID_APP_MODE_CONTROL_CODE: &str = "world_model.comfy_app_mode.invalid_control";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyAppModeControlKind {
    Text,
    Number,
    Boolean,
    Choice,
    Image,
    Seed,
    Unknown,
}

impl ComfyAppModeControlKind {
    fn parse(value: &str) -> Self {
        match value {
            "text" | "string" | "prompt" => Self::Text,
            "number" | "float" | "int" | "integer" => Self::Number,
            "boolean" | "bool" | "toggle" => Self::Boolean,
            "choice" | "combo" | "select" => Self::Choice,
            "image" | "upload" => Self::Image,
            "seed" => Self::Seed,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyAppModeUiOwner {
    UnifiedAuthoringApp,
    DiffusionGraphEditor,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAppModeControlTarget {
    pub node_id: u64,
    pub input_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyAppModeControl {
    pub id: String,
    pub label: String,
    pub kind: ComfyAppModeControlKind,
    pub target: ComfyAppModeControlTarget,
    pub default_value: Option<serde_json::Value>,
    pub choices: Vec<String>,
    pub ui_owner: ComfyAppModeUiOwner,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAppModeDiagnostic {
    pub code: String,
    pub control_id: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyAppModeReport {
    pub workflow_name: String,
    pub title: Option<String>,
    pub controls: Vec<ComfyAppModeControl>,
    pub available_as_graph_workflow: bool,
    pub diagnostics: Vec<ComfyAppModeDiagnostic>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComfyAppModeBridge;

impl ComfyAppModeBridge {
    pub fn expose(&self, workflow: &ComfyWorkflowDocument) -> ComfyAppModeReport {
        let mut diagnostics = Vec::new();
        let Some(metadata) = app_mode_metadata(&workflow.graph_json) else {
            return ComfyAppModeReport {
                workflow_name: workflow.name.clone(),
                title: None,
                controls: Vec::new(),
                available_as_graph_workflow: true,
                diagnostics,
            };
        };

        let title = metadata
            .get("title")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        let Some(controls) = metadata
            .get("controls")
            .and_then(serde_json::Value::as_array)
        else {
            diagnostics.push(diagnostic(
                INVALID_APP_MODE_METADATA_CODE,
                None,
                "app-mode metadata must contain a controls array",
            ));
            return ComfyAppModeReport {
                workflow_name: workflow.name.clone(),
                title,
                controls: Vec::new(),
                available_as_graph_workflow: true,
                diagnostics,
            };
        };

        let controls = controls
            .iter()
            .filter_map(|control| parse_control(control, &mut diagnostics))
            .collect::<Vec<_>>();

        ComfyAppModeReport {
            workflow_name: workflow.name.clone(),
            title,
            available_as_graph_workflow: true,
            controls,
            diagnostics,
        }
    }
}

fn parse_control(
    control: &serde_json::Value,
    diagnostics: &mut Vec<ComfyAppModeDiagnostic>,
) -> Option<ComfyAppModeControl> {
    let Some(id) = control
        .get("id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
    else {
        diagnostics.push(diagnostic(
            INVALID_APP_MODE_CONTROL_CODE,
            None,
            "app-mode control is missing id",
        ));
        return None;
    };
    let Some(node_id) = control
        .get("node_id")
        .or_else(|| control.get("node"))
        .and_then(serde_json::Value::as_u64)
    else {
        diagnostics.push(diagnostic(
            INVALID_APP_MODE_CONTROL_CODE,
            Some(id),
            "app-mode control is missing target node id",
        ));
        return None;
    };
    let Some(input_name) = control
        .get("input")
        .or_else(|| control.get("input_name"))
        .and_then(serde_json::Value::as_str)
    else {
        diagnostics.push(diagnostic(
            INVALID_APP_MODE_CONTROL_CODE,
            Some(id),
            "app-mode control is missing target input name",
        ));
        return None;
    };
    let label = control
        .get("label")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&id)
        .to_string();
    let kind = control
        .get("kind")
        .or_else(|| control.get("type"))
        .and_then(serde_json::Value::as_str)
        .map(ComfyAppModeControlKind::parse)
        .unwrap_or(ComfyAppModeControlKind::Unknown);
    let choices = control
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect();

    Some(ComfyAppModeControl {
        id,
        label,
        kind,
        target: ComfyAppModeControlTarget {
            node_id,
            input_name: input_name.to_string(),
        },
        default_value: control.get("default").cloned(),
        choices,
        ui_owner: ComfyAppModeUiOwner::UnifiedAuthoringApp,
    })
}

fn app_mode_metadata(graph_json: &serde_json::Value) -> Option<&serde_json::Value> {
    graph_json
        .get("extra")
        .and_then(|extra| extra.get("app_mode").or_else(|| extra.get("app")))
}

fn diagnostic(
    code: &str,
    control_id: Option<String>,
    message: impl Into<String>,
) -> ComfyAppModeDiagnostic {
    ComfyAppModeDiagnostic {
        code: code.to_string(),
        control_id,
        message: message.into(),
    }
}
