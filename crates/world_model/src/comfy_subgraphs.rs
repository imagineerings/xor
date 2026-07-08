use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ComfyBlueprintCatalog, ComfyBlueprintRecord};

pub const DUPLICATE_SUBGRAPH_ID_CODE: &str = "world_model.comfy_subgraphs.duplicate_id";
pub const SUBGRAPH_NOT_FOUND_CODE: &str = "world_model.comfy_subgraphs.not_found";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ComfySubgraphId(String);

impl ComfySubgraphId {
    pub fn from_source(source_type: ComfySubgraphSourceType, source_path: &str) -> Self {
        let source_label = source_type.as_str();
        Self(format!(
            "subgraph-{source_label}-{:016x}",
            stable_hash(source_label, source_path)
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ComfySubgraphSourceType {
    Blueprint,
    CustomNode,
}

impl ComfySubgraphSourceType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Blueprint => "blueprint",
            Self::CustomNode => "custom-node",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfySubgraphSource {
    Blueprint {
        blueprint_name: String,
        source_path: String,
    },
    CustomNode {
        node_pack_name: String,
        source_path: String,
    },
}

impl ComfySubgraphSource {
    pub fn source_type(&self) -> ComfySubgraphSourceType {
        match self {
            Self::Blueprint { .. } => ComfySubgraphSourceType::Blueprint,
            Self::CustomNode { .. } => ComfySubgraphSourceType::CustomNode,
        }
    }

    pub fn source_path(&self) -> &str {
        match self {
            Self::Blueprint { source_path, .. } | Self::CustomNode { source_path, .. } => {
                source_path
            }
        }
    }

    pub fn node_pack_name(&self) -> Option<&str> {
        match self {
            Self::Blueprint { .. } => None,
            Self::CustomNode { node_pack_name, .. } => Some(node_pack_name),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfySubgraphRecord {
    pub id: ComfySubgraphId,
    pub name: String,
    pub source: ComfySubgraphSource,
    pub graph_json: serde_json::Value,
    pub node_count: usize,
    pub link_count: usize,
    pub metadata: serde_json::Value,
}

impl ComfySubgraphRecord {
    pub fn from_blueprint(blueprint: &ComfyBlueprintRecord) -> Self {
        let source = ComfySubgraphSource::Blueprint {
            blueprint_name: blueprint.name.clone(),
            source_path: blueprint.source_path.clone(),
        };
        let metadata = serde_json::json!({
            "category": blueprint.category,
            "dependencies": blueprint.dependencies,
            "node_types": blueprint.node_types,
            "attribution": blueprint.attribution,
        });
        Self::new(
            blueprint.name.clone(),
            source,
            blueprint.graph_json.clone(),
            metadata,
        )
        .with_counts(blueprint.node_count, blueprint.link_count)
    }

    pub fn new(
        name: impl Into<String>,
        source: ComfySubgraphSource,
        graph_json: serde_json::Value,
        metadata: serde_json::Value,
    ) -> Self {
        let source_type = source.source_type();
        let id = ComfySubgraphId::from_source(source_type, source.source_path());
        let node_count = graph_json
            .get("nodes")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);
        let link_count = graph_json
            .get("links")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);
        Self {
            id,
            name: name.into(),
            source,
            graph_json,
            node_count,
            link_count,
            metadata: sanitize_metadata(&metadata),
        }
    }

    fn with_counts(mut self, node_count: usize, link_count: usize) -> Self {
        self.node_count = node_count;
        self.link_count = link_count;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfySubgraphListing {
    pub id: ComfySubgraphId,
    pub name: String,
    pub source_type: ComfySubgraphSourceType,
    pub source_path: String,
    pub node_pack_name: Option<String>,
    pub node_count: usize,
    pub link_count: usize,
    pub metadata: serde_json::Value,
}

impl From<&ComfySubgraphRecord> for ComfySubgraphListing {
    fn from(record: &ComfySubgraphRecord) -> Self {
        Self {
            id: record.id.clone(),
            name: record.name.clone(),
            source_type: record.source.source_type(),
            source_path: record.source.source_path().to_string(),
            node_pack_name: record.source.node_pack_name().map(str::to_string),
            node_count: record.node_count,
            link_count: record.link_count,
            metadata: record.metadata.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfySubgraphDiagnostic {
    pub code: String,
    pub subgraph_id: Option<ComfySubgraphId>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfySubgraphIndex {
    records: BTreeMap<ComfySubgraphId, ComfySubgraphRecord>,
    diagnostics: Vec<ComfySubgraphDiagnostic>,
}

impl ComfySubgraphIndex {
    pub fn from_blueprint_catalog(catalog: &ComfyBlueprintCatalog) -> Self {
        let mut index = Self::default();
        for blueprint in catalog.records() {
            index.register(ComfySubgraphRecord::from_blueprint(blueprint));
        }
        index
    }

    pub fn register_custom_node_subgraph(
        &mut self,
        node_pack_name: impl Into<String>,
        name: impl Into<String>,
        source_path: impl Into<String>,
        graph_json: serde_json::Value,
        metadata: serde_json::Value,
    ) -> ComfySubgraphId {
        let source = ComfySubgraphSource::CustomNode {
            node_pack_name: node_pack_name.into(),
            source_path: source_path.into(),
        };
        let record = ComfySubgraphRecord::new(name, source, graph_json, metadata);
        let id = record.id.clone();
        self.register(record);
        id
    }

    pub fn register(&mut self, record: ComfySubgraphRecord) {
        if self.records.contains_key(&record.id) {
            self.diagnostics.push(ComfySubgraphDiagnostic {
                code: DUPLICATE_SUBGRAPH_ID_CODE.to_string(),
                subgraph_id: Some(record.id.clone()),
                message: format!("duplicate subgraph id `{}`", record.id.as_str()),
            });
            return;
        }
        self.records.insert(record.id.clone(), record);
    }

    pub fn listings(&self) -> Vec<ComfySubgraphListing> {
        self.records
            .values()
            .map(ComfySubgraphListing::from)
            .collect()
    }

    pub fn open(
        &self,
        id: &ComfySubgraphId,
    ) -> Result<&ComfySubgraphRecord, ComfySubgraphDiagnostic> {
        self.records.get(id).ok_or_else(|| ComfySubgraphDiagnostic {
            code: SUBGRAPH_NOT_FOUND_CODE.to_string(),
            subgraph_id: Some(id.clone()),
            message: format!("subgraph `{}` was not found", id.as_str()),
        })
    }

    pub fn diagnostics(&self) -> &[ComfySubgraphDiagnostic] {
        &self.diagnostics
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
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

fn stable_hash(source_type: &str, source_path: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for byte in source_type
        .as_bytes()
        .iter()
        .chain(std::iter::once(&0))
        .chain(source_path.as_bytes().iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
