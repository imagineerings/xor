use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ComfyNodeRegistry, DiffusionGraph, NodeReplacementEngine, NodeReplacementReport,
    NodeReplacementRule,
};

pub const DUPLICATE_REPLACEMENT_MAPPING_CODE: &str =
    "world_model.comfy_replacements.duplicate_mapping";
pub const CONFLICTING_REPLACEMENT_MAPPING_CODE: &str =
    "world_model.comfy_replacements.conflicting_mapping";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyReplacementSource {
    BuiltIn,
    CustomNode {
        node_pack_name: String,
        source_path: String,
    },
    ImportedWorkflow {
        source_path: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyReplacementEntry {
    pub rule: NodeReplacementRule,
    pub source: ComfyReplacementSource,
    pub metadata: BTreeMap<String, String>,
}

impl ComfyReplacementEntry {
    pub fn new(rule: NodeReplacementRule, source: ComfyReplacementSource) -> Self {
        Self {
            rule,
            source,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyReplacementDiagnostic {
    pub code: String,
    pub from_node_type: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyReplacementCatalog {
    entries: BTreeMap<String, ComfyReplacementEntry>,
    diagnostics: Vec<ComfyReplacementDiagnostic>,
}

impl ComfyReplacementCatalog {
    pub fn new(entries: impl IntoIterator<Item = ComfyReplacementEntry>) -> Self {
        let mut catalog = Self::default();
        for entry in entries {
            catalog.register(entry);
        }
        catalog
    }

    pub fn register(&mut self, entry: ComfyReplacementEntry) {
        let from_node_type = entry.rule.from_node_type.clone();
        if let Some(existing) = self.entries.get(&from_node_type) {
            if existing.rule == entry.rule {
                self.diagnostics.push(diagnostic(
                    DUPLICATE_REPLACEMENT_MAPPING_CODE,
                    &from_node_type,
                    format!("duplicate replacement mapping for `{from_node_type}` was ignored"),
                ));
            } else {
                self.diagnostics.push(diagnostic(
                    CONFLICTING_REPLACEMENT_MAPPING_CODE,
                    &from_node_type,
                    format!(
                        "conflicting replacement mapping for `{from_node_type}` kept existing target `{}`",
                        existing.rule.to_node_type
                    ),
                ));
            }
            return;
        }
        self.entries.insert(from_node_type, entry);
    }

    pub fn rule_for(&self, from_node_type: &str) -> Option<&NodeReplacementRule> {
        self.entries.get(from_node_type).map(|entry| &entry.rule)
    }

    pub fn entry_for(&self, from_node_type: &str) -> Option<&ComfyReplacementEntry> {
        self.entries.get(from_node_type)
    }

    pub fn rules(&self) -> Vec<NodeReplacementRule> {
        self.entries
            .values()
            .map(|entry| entry.rule.clone())
            .collect()
    }

    pub fn engine(&self) -> NodeReplacementEngine {
        NodeReplacementEngine::new(self.rules())
    }

    pub fn apply_to_graph(
        &self,
        graph: &DiffusionGraph,
        registry: &ComfyNodeRegistry,
    ) -> NodeReplacementReport {
        self.engine().apply(graph, registry)
    }

    pub fn diagnostics(&self) -> &[ComfyReplacementDiagnostic] {
        &self.diagnostics
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn diagnostic(
    code: &str,
    from_node_type: &str,
    message: impl Into<String>,
) -> ComfyReplacementDiagnostic {
    ComfyReplacementDiagnostic {
        code: code.to_string(),
        from_node_type: from_node_type.to_string(),
        message: message.into(),
    }
}
