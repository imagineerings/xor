use serde::{Deserialize, Serialize};

use crate::{
    ComfySubgraphId, ComfySubgraphIndex, ComfyWorkflowTemplateAdapter, ComfyWorkflowTemplateAsset,
    ComfyWorkflowTemplateId, SimExtensionId, SimExtensionRecord,
};

pub const SIM_EXTENSION_SUBGRAPH_INDEXED_CODE: &str = "world_model.extension_templates.subgraph";
pub const SIM_EXTENSION_TEMPLATE_INDEXED_CODE: &str = "world_model.extension_templates.template";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionTemplateDeclaration {
    pub extension_id: SimExtensionId,
    pub name: String,
    pub template_path: String,
    pub graph_json: serde_json::Value,
    pub static_assets: Vec<ComfyWorkflowTemplateAsset>,
    pub metadata: serde_json::Value,
}

impl SimExtensionTemplateDeclaration {
    pub fn new(
        extension: &SimExtensionRecord,
        name: impl Into<String>,
        template_path: impl Into<String>,
        graph_json: serde_json::Value,
    ) -> Self {
        Self {
            extension_id: extension.id.clone(),
            name: name.into(),
            template_path: template_path.into(),
            graph_json,
            static_assets: Vec::new(),
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_asset(mut self, asset: ComfyWorkflowTemplateAsset) -> Self {
        self.static_assets.push(asset);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionSubgraphDeclaration {
    pub extension_id: SimExtensionId,
    pub name: String,
    pub source_path: String,
    pub graph_json: serde_json::Value,
    pub metadata: serde_json::Value,
}

impl SimExtensionSubgraphDeclaration {
    pub fn new(
        extension: &SimExtensionRecord,
        name: impl Into<String>,
        source_path: impl Into<String>,
        graph_json: serde_json::Value,
    ) -> Self {
        Self {
            extension_id: extension.id.clone(),
            name: name.into(),
            source_path: source_path.into(),
            graph_json,
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionTemplateIndexDiagnostic {
    pub code: String,
    pub extension_id: SimExtensionId,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionTemplateIndexReport {
    pub template_ids: Vec<ComfyWorkflowTemplateId>,
    pub subgraph_ids: Vec<ComfySubgraphId>,
    pub diagnostics: Vec<SimExtensionTemplateIndexDiagnostic>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionTemplateIndexer;

impl SimExtensionTemplateIndexer {
    pub fn new() -> Self {
        Self
    }

    pub fn index(
        &self,
        templates: impl IntoIterator<Item = SimExtensionTemplateDeclaration>,
        subgraphs: impl IntoIterator<Item = SimExtensionSubgraphDeclaration>,
        template_adapter: &mut ComfyWorkflowTemplateAdapter,
        subgraph_index: &mut ComfySubgraphIndex,
    ) -> SimExtensionTemplateIndexReport {
        let mut report = SimExtensionTemplateIndexReport::default();

        for template in templates {
            let previous_diagnostic_count = template_adapter.diagnostics().len();
            match template_adapter.register_custom_node_template(
                template.extension_id.as_str(),
                template.name,
                template.template_path.clone(),
                template.graph_json,
                template.static_assets,
                template.metadata,
            ) {
                Some(id) => {
                    report.template_ids.push(id);
                    report.diagnostics.push(index_diagnostic(
                        SIM_EXTENSION_TEMPLATE_INDEXED_CODE,
                        template.extension_id,
                        template.template_path,
                        "extension workflow template indexed as a native Sim template",
                    ));
                }
                None => {
                    if template_adapter.diagnostics().len() > previous_diagnostic_count {
                        let diagnostic = template_adapter
                            .diagnostics()
                            .last()
                            .map(|diagnostic| diagnostic.message.clone())
                            .unwrap_or_else(|| {
                                "extension workflow template could not be indexed".to_string()
                            });
                        report.diagnostics.push(index_diagnostic(
                            crate::UNSAFE_WORKFLOW_TEMPLATE_PATH_CODE,
                            template.extension_id,
                            template.template_path,
                            diagnostic,
                        ));
                    }
                }
            }
        }

        for subgraph in subgraphs {
            let previous_diagnostic_count = subgraph_index.diagnostics().len();
            let id = subgraph_index.register_custom_node_subgraph(
                subgraph.extension_id.as_str(),
                subgraph.name,
                subgraph.source_path.clone(),
                subgraph.graph_json,
                subgraph.metadata,
            );
            if subgraph_index.diagnostics().len() == previous_diagnostic_count {
                report.subgraph_ids.push(id);
                report.diagnostics.push(index_diagnostic(
                    SIM_EXTENSION_SUBGRAPH_INDEXED_CODE,
                    subgraph.extension_id,
                    subgraph.source_path,
                    "extension subgraph indexed as a native Sim subgraph",
                ));
            } else {
                let diagnostic = subgraph_index
                    .diagnostics()
                    .last()
                    .map(|diagnostic| diagnostic.message.clone())
                    .unwrap_or_else(|| "extension subgraph could not be indexed".to_string());
                report.diagnostics.push(index_diagnostic(
                    crate::DUPLICATE_SUBGRAPH_ID_CODE,
                    subgraph.extension_id,
                    subgraph.source_path,
                    diagnostic,
                ));
            }
        }

        report
    }
}

fn index_diagnostic(
    code: impl Into<String>,
    extension_id: SimExtensionId,
    path: impl Into<String>,
    message: impl Into<String>,
) -> SimExtensionTemplateIndexDiagnostic {
    SimExtensionTemplateIndexDiagnostic {
        code: code.into(),
        extension_id,
        path: path.into(),
        message: message.into(),
    }
}
