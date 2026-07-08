use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const WORKFLOW_NOT_FOUND_CODE: &str = "world_model.comfy_workflows.not_found";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ComfyWorkflowId(String);

impl ComfyWorkflowId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowVersionId {
    pub workflow_id: ComfyWorkflowId,
    pub version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyWorkflowSource {
    Blueprint { source_path: String },
    User { project_path: String },
    Imported { source_path: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowView {
    pub x: i64,
    pub y: i64,
    pub scale_millis: u64,
}

impl Default for ComfyWorkflowView {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            scale_millis: 1000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowDocument {
    pub id: Option<ComfyWorkflowId>,
    pub name: String,
    pub graph_json: serde_json::Value,
    pub ui_metadata: serde_json::Value,
    pub default_view: ComfyWorkflowView,
    pub source: ComfyWorkflowSource,
    pub provenance_artifact_id: Option<String>,
}

impl ComfyWorkflowDocument {
    pub fn from_graph_json(
        name: impl Into<String>,
        graph_json: serde_json::Value,
        source: ComfyWorkflowSource,
    ) -> Self {
        let ui_metadata = graph_json
            .get("extra")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let default_view = view_from_metadata(&ui_metadata);
        Self {
            id: None,
            name: name.into(),
            graph_json,
            ui_metadata,
            default_view,
            source,
            provenance_artifact_id: None,
        }
    }

    pub fn with_id(mut self, id: ComfyWorkflowId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_provenance_artifact(mut self, artifact_id: impl Into<String>) -> Self {
        self.provenance_artifact_id = Some(artifact_id.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowDiagnostic {
    pub code: String,
    pub message: String,
    pub workflow_id: Option<ComfyWorkflowId>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowStore {
    documents: BTreeMap<ComfyWorkflowId, Vec<ComfyWorkflowDocument>>,
    next_id: u64,
}

impl ComfyWorkflowStore {
    pub fn save(&mut self, mut document: ComfyWorkflowDocument) -> ComfyWorkflowVersionId {
        let workflow_id = document.id.clone().unwrap_or_else(|| {
            self.next_id = self.next_id.saturating_add(1);
            ComfyWorkflowId::new(format!("workflow-{}", self.next_id))
        });
        document.id = Some(workflow_id.clone());

        let versions = self.documents.entry(workflow_id.clone()).or_default();
        versions.push(document);

        ComfyWorkflowVersionId {
            workflow_id,
            version: versions.len() as u64,
        }
    }

    pub fn load(
        &self,
        workflow_id: &ComfyWorkflowId,
    ) -> Result<&ComfyWorkflowDocument, ComfyWorkflowDiagnostic> {
        self.documents
            .get(workflow_id)
            .and_then(|versions| versions.last())
            .ok_or_else(|| workflow_not_found(workflow_id))
    }

    pub fn load_version(
        &self,
        version_id: &ComfyWorkflowVersionId,
    ) -> Result<&ComfyWorkflowDocument, ComfyWorkflowDiagnostic> {
        let index = version_id.version.saturating_sub(1) as usize;
        self.documents
            .get(&version_id.workflow_id)
            .and_then(|versions| versions.get(index))
            .ok_or_else(|| workflow_not_found(&version_id.workflow_id))
    }

    pub fn version_count(&self, workflow_id: &ComfyWorkflowId) -> usize {
        self.documents
            .get(workflow_id)
            .map_or(0, |versions| versions.len())
    }
}

fn view_from_metadata(ui_metadata: &serde_json::Value) -> ComfyWorkflowView {
    let Some(ds) = ui_metadata.get("ds") else {
        return ComfyWorkflowView::default();
    };
    ComfyWorkflowView {
        x: ds
            .get("offset")
            .and_then(|offset| offset.get(0))
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0),
        y: ds
            .get("offset")
            .and_then(|offset| offset.get(1))
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0),
        scale_millis: ds
            .get("scale")
            .and_then(serde_json::Value::as_f64)
            .filter(|scale| scale.is_finite() && *scale > 0.0)
            .map(|scale| (scale * 1000.0).round() as u64)
            .unwrap_or(1000),
    }
}

fn workflow_not_found(workflow_id: &ComfyWorkflowId) -> ComfyWorkflowDiagnostic {
    ComfyWorkflowDiagnostic {
        code: WORKFLOW_NOT_FOUND_CODE.to_string(),
        message: format!("workflow `{}` was not found", workflow_id.as_str()),
        workflow_id: Some(workflow_id.clone()),
    }
}
