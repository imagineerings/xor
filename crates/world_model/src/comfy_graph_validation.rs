use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    ComfyNodeRegistry, DefaultGraphValidator, DiffusionGraph, DiffusionGraphValidator,
    GraphValidationError, GraphValidationResult, graph::NodeId,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyValidationCapabilities {
    pub providers: BTreeSet<String>,
    pub model_folders: BTreeSet<String>,
    pub asset_capabilities: BTreeSet<String>,
}

impl ComfyValidationCapabilities {
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.providers.insert(provider.into());
        self
    }

    pub fn with_model_folder(mut self, model_folder: impl Into<String>) -> Self {
        self.model_folders.insert(model_folder.into());
        self
    }

    pub fn with_asset_capability(mut self, asset_capability: impl Into<String>) -> Self {
        self.asset_capabilities.insert(asset_capability.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComfyPromptGraphValidator {
    registry: ComfyNodeRegistry,
    capabilities: ComfyValidationCapabilities,
}

impl ComfyPromptGraphValidator {
    pub fn new(registry: ComfyNodeRegistry) -> Self {
        Self {
            registry,
            capabilities: ComfyValidationCapabilities::default(),
        }
    }

    pub fn with_capabilities(mut self, capabilities: ComfyValidationCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn validate(
        &self,
        graph: &DiffusionGraph,
        partial_targets: impl IntoIterator<Item = NodeId>,
    ) -> GraphValidationResult {
        let mut result = validate_structure_with_literal_inputs(graph);

        for node in &graph.nodes {
            if self.registry.availability(&node.node_type).is_err() {
                result.errors.push(GraphValidationError::UnknownNodeType {
                    node_id: node.id,
                    node_type: node.node_type.clone(),
                });
            }
            self.validate_capabilities(node, &mut result);
        }

        for target in partial_targets {
            if graph.node_by_id(target).is_none() {
                result.errors.push(GraphValidationError::MissingNode {
                    node_id: target,
                    context: "partial execution target".to_string(),
                });
            }
        }

        result
    }

    fn validate_capabilities(&self, node: &crate::GraphNode, result: &mut GraphValidationResult) {
        validate_capability(
            node,
            result,
            "requires_provider",
            "provider",
            &self.capabilities.providers,
        );
        validate_capability(
            node,
            result,
            "requires_model_folder",
            "model_folder",
            &self.capabilities.model_folders,
        );
        validate_capability(
            node,
            result,
            "requires_asset_capability",
            "asset",
            &self.capabilities.asset_capabilities,
        );
    }
}

fn validate_structure_with_literal_inputs(graph: &DiffusionGraph) -> GraphValidationResult {
    let mut result = DefaultGraphValidator.validate(graph);
    result.errors.retain(|error| match error {
        GraphValidationError::UnconnectedRequiredPort { node_id, port_name } => graph
            .node_by_id(*node_id)
            .is_none_or(|node| !node.metadata.contains_key(port_name)),
        _ => true,
    });
    result
}

fn validate_capability(
    node: &crate::GraphNode,
    result: &mut GraphValidationResult,
    metadata_key: &str,
    label: &str,
    available: &BTreeSet<String>,
) {
    if let Some(required) = node.metadata.get(metadata_key) {
        if !available.contains(required) {
            result
                .errors
                .push(GraphValidationError::UnsupportedBackend {
                    backend: format!("{label}:{required}"),
                    node_id: node.id,
                });
        }
    }
}
