use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const DUPLICATE_WORKFLOW_TEMPLATE_CODE: &str = "world_model.comfy_workflow_templates.duplicate";
pub const UNSAFE_WORKFLOW_TEMPLATE_PATH_CODE: &str =
    "world_model.comfy_workflow_templates.unsafe_path";
pub const WORKFLOW_TEMPLATE_NOT_FOUND_CODE: &str = "world_model.comfy_workflow_templates.not_found";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ComfyWorkflowTemplateId(String);

impl ComfyWorkflowTemplateId {
    pub fn from_custom_node_template(node_pack_name: &str, template_path: &str) -> Self {
        Self(format!(
            "workflow-template-custom-node-{:016x}",
            stable_hash(node_pack_name, template_path)
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyWorkflowTemplateSource {
    CustomNode {
        node_pack_name: String,
        template_path: String,
    },
}

impl ComfyWorkflowTemplateSource {
    pub fn node_pack_name(&self) -> &str {
        match self {
            Self::CustomNode { node_pack_name, .. } => node_pack_name,
        }
    }

    pub fn template_path(&self) -> &str {
        match self {
            Self::CustomNode { template_path, .. } => template_path,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowTemplateAsset {
    pub name: String,
    pub source_path: String,
    pub content_type: String,
}

impl ComfyWorkflowTemplateAsset {
    pub fn new(
        name: impl Into<String>,
        source_path: impl Into<String>,
        content_type: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source_path: source_path.into(),
            content_type: content_type.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowTemplateRecord {
    pub id: ComfyWorkflowTemplateId,
    pub name: String,
    pub source: ComfyWorkflowTemplateSource,
    pub graph_json: serde_json::Value,
    pub static_assets: Vec<ComfyWorkflowTemplateAsset>,
    pub metadata: serde_json::Value,
}

impl ComfyWorkflowTemplateRecord {
    pub fn custom_node(
        node_pack_name: impl Into<String>,
        name: impl Into<String>,
        template_path: impl Into<String>,
        graph_json: serde_json::Value,
        static_assets: Vec<ComfyWorkflowTemplateAsset>,
        metadata: serde_json::Value,
    ) -> Self {
        let node_pack_name = node_pack_name.into();
        let template_path = template_path.into();
        let id = ComfyWorkflowTemplateId::from_custom_node_template(
            node_pack_name.as_str(),
            template_path.as_str(),
        );
        Self {
            id,
            name: name.into(),
            source: ComfyWorkflowTemplateSource::CustomNode {
                node_pack_name,
                template_path,
            },
            graph_json,
            static_assets,
            metadata: sanitize_metadata(&metadata),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowTemplateListing {
    pub id: ComfyWorkflowTemplateId,
    pub name: String,
    pub node_pack_name: String,
    pub template_path: String,
    pub static_assets: Vec<ComfyWorkflowTemplateAsset>,
    pub metadata: serde_json::Value,
}

impl From<&ComfyWorkflowTemplateRecord> for ComfyWorkflowTemplateListing {
    fn from(record: &ComfyWorkflowTemplateRecord) -> Self {
        Self {
            id: record.id.clone(),
            name: record.name.clone(),
            node_pack_name: record.source.node_pack_name().to_string(),
            template_path: record.source.template_path().to_string(),
            static_assets: record.static_assets.clone(),
            metadata: record.metadata.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowTemplateDiagnostic {
    pub code: String,
    pub template_id: Option<ComfyWorkflowTemplateId>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfyWorkflowTemplateAdapter {
    records: BTreeMap<ComfyWorkflowTemplateId, ComfyWorkflowTemplateRecord>,
    diagnostics: Vec<ComfyWorkflowTemplateDiagnostic>,
}

impl ComfyWorkflowTemplateAdapter {
    pub fn register_custom_node_template(
        &mut self,
        node_pack_name: impl Into<String>,
        name: impl Into<String>,
        template_path: impl Into<String>,
        graph_json: serde_json::Value,
        static_assets: Vec<ComfyWorkflowTemplateAsset>,
        metadata: serde_json::Value,
    ) -> Option<ComfyWorkflowTemplateId> {
        let record = ComfyWorkflowTemplateRecord::custom_node(
            node_pack_name,
            name,
            template_path,
            graph_json,
            static_assets,
            metadata,
        );
        let id = record.id.clone();
        if !self.validate_paths(&record) {
            return None;
        }
        if self.records.contains_key(&id) {
            self.diagnostics.push(ComfyWorkflowTemplateDiagnostic {
                code: DUPLICATE_WORKFLOW_TEMPLATE_CODE.to_string(),
                template_id: Some(id.clone()),
                message: format!("duplicate workflow template `{}`", id.as_str()),
            });
            return None;
        }
        self.records.insert(id.clone(), record);
        Some(id)
    }

    pub fn register(&mut self, record: ComfyWorkflowTemplateRecord) {
        if !self.validate_paths(&record) {
            return;
        }
        if self.records.contains_key(&record.id) {
            self.diagnostics.push(ComfyWorkflowTemplateDiagnostic {
                code: DUPLICATE_WORKFLOW_TEMPLATE_CODE.to_string(),
                template_id: Some(record.id.clone()),
                message: format!("duplicate workflow template `{}`", record.id.as_str()),
            });
            return;
        }
        self.records.insert(record.id.clone(), record);
    }

    pub fn listings(&self) -> Vec<ComfyWorkflowTemplateListing> {
        self.records
            .values()
            .map(ComfyWorkflowTemplateListing::from)
            .collect()
    }

    pub fn open(
        &self,
        id: &ComfyWorkflowTemplateId,
    ) -> Result<&ComfyWorkflowTemplateRecord, ComfyWorkflowTemplateDiagnostic> {
        self.records
            .get(id)
            .ok_or_else(|| ComfyWorkflowTemplateDiagnostic {
                code: WORKFLOW_TEMPLATE_NOT_FOUND_CODE.to_string(),
                template_id: Some(id.clone()),
                message: format!("workflow template `{}` was not found", id.as_str()),
            })
    }

    pub fn diagnostics(&self) -> &[ComfyWorkflowTemplateDiagnostic] {
        &self.diagnostics
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    fn validate_paths(&mut self, record: &ComfyWorkflowTemplateRecord) -> bool {
        if !is_safe_relative_path(record.source.template_path()) {
            self.diagnostics.push(ComfyWorkflowTemplateDiagnostic {
                code: UNSAFE_WORKFLOW_TEMPLATE_PATH_CODE.to_string(),
                template_id: Some(record.id.clone()),
                message: format!(
                    "workflow template path `{}` is not a safe relative path",
                    record.source.template_path()
                ),
            });
            return false;
        }
        for asset in &record.static_assets {
            if !is_safe_relative_path(&asset.source_path) {
                self.diagnostics.push(ComfyWorkflowTemplateDiagnostic {
                    code: UNSAFE_WORKFLOW_TEMPLATE_PATH_CODE.to_string(),
                    template_id: Some(record.id.clone()),
                    message: format!(
                        "workflow template asset path `{}` is not a safe relative path",
                        asset.source_path
                    ),
                });
                return false;
            }
        }
        true
    }
}

fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && !path.split(['/', '\\']).any(|segment| {
            segment.is_empty() || segment == "." || segment == ".." || segment.contains(':')
        })
}

fn sanitize_metadata(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sanitized = map
                .iter()
                .filter(|(key, _)| !is_sensitive_metadata_key(key))
                .map(|(key, value)| (key.clone(), sanitize_metadata(value)))
                .collect();
            serde_json::Value::Object(sanitized)
        }
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(sanitize_metadata).collect())
        }
        _ => value.clone(),
    }
}

fn is_sensitive_metadata_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("authorization")
        || key.contains("api_key")
        || key.contains("credential")
        || key.contains("password")
        || key.contains("secret")
        || key.contains("token")
        || key == "graph_json"
        || key == "prompt"
        || key == "workflow"
}

fn stable_hash(node_pack_name: &str, template_path: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for byte in node_pack_name
        .as_bytes()
        .iter()
        .chain(std::iter::once(&0))
        .chain(template_path.as_bytes().iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
