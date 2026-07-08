use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ComfyWorkflowDocument;

pub const INVALID_WORKFLOW_GRAPH_CODE: &str = "world_model.comfy_workflows.invalid_graph";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowExportDiagnostic {
    pub code: String,
    pub message: String,
    pub node_id: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComfyWorkflowApiExporter;

impl ComfyWorkflowApiExporter {
    pub fn export_api_prompt(
        document: &ComfyWorkflowDocument,
    ) -> Result<serde_json::Value, Vec<ComfyWorkflowExportDiagnostic>> {
        let nodes = document
            .graph_json
            .get("nodes")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                vec![invalid_graph(
                    "workflow graph must contain a nodes array",
                    None,
                )]
            })?;
        let links = link_lookup(&document.graph_json);
        let mut diagnostics = Vec::new();
        let mut prompt = serde_json::Map::new();

        for node in nodes {
            let Some(node_id) = node.get("id").and_then(serde_json::Value::as_u64) else {
                diagnostics.push(invalid_graph("workflow node is missing numeric id", None));
                continue;
            };
            let Some(class_type) = node.get("type").and_then(serde_json::Value::as_str) else {
                diagnostics.push(invalid_graph(
                    "workflow node is missing type",
                    Some(node_id),
                ));
                continue;
            };

            prompt.insert(
                node_id.to_string(),
                serde_json::json!({
                    "class_type": class_type,
                    "inputs": export_inputs(node, &links),
                }),
            );
        }

        if diagnostics.is_empty() {
            Ok(serde_json::Value::Object(prompt))
        } else {
            Err(diagnostics)
        }
    }
}

fn export_inputs(
    node: &serde_json::Value,
    links: &BTreeMap<u64, (u64, u64)>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut inputs = serde_json::Map::new();
    let widgets = node
        .get("widgets_values")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut widget_index = 0;

    for input in node
        .get("inputs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(name) = input.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if let Some(link_id) = input.get("link").and_then(serde_json::Value::as_u64) {
            if let Some((source_node, source_output)) = links.get(&link_id) {
                inputs.insert(
                    name.to_string(),
                    serde_json::json!([source_node, source_output]),
                );
            }
        } else if input.get("widget").is_some() {
            if let Some(value) = widgets.get(widget_index) {
                inputs.insert(name.to_string(), value.clone());
            }
            widget_index += 1;
        }
    }

    inputs
}

fn link_lookup(graph_json: &serde_json::Value) -> BTreeMap<u64, (u64, u64)> {
    graph_json
        .get("links")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|link| {
            let values = link.as_array()?;
            Some((
                values.first()?.as_u64()?,
                (values.get(1)?.as_u64()?, values.get(2)?.as_u64()?),
            ))
        })
        .collect()
}

fn invalid_graph(
    message: impl Into<String>,
    node_id: Option<u64>,
) -> ComfyWorkflowExportDiagnostic {
    ComfyWorkflowExportDiagnostic {
        code: INVALID_WORKFLOW_GRAPH_CODE.to_string(),
        message: message.into(),
        node_id,
    }
}
