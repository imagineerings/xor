use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ComfyNodeRegistry, DiffusionGraph, GraphEdge, GraphNode, NodePort, PortDirection, graph::NodeId,
};

pub const REPLACEMENT_INVALID_TARGET_CODE: &str =
    "world_model.comfy_node_replacement.invalid_target";
pub const REPLACEMENT_NOT_APPLIED_CODE: &str = "world_model.comfy_node_replacement.not_applied";

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeReplacementRule {
    pub from_node_type: String,
    pub to_node_type: String,
    pub input_mappings: BTreeMap<String, String>,
    pub output_mappings: BTreeMap<String, String>,
}

impl NodeReplacementRule {
    pub fn new(from_node_type: impl Into<String>, to_node_type: impl Into<String>) -> Self {
        Self {
            from_node_type: from_node_type.into(),
            to_node_type: to_node_type.into(),
            input_mappings: BTreeMap::new(),
            output_mappings: BTreeMap::new(),
        }
    }

    pub fn with_input_mapping(
        mut self,
        old_input: impl Into<String>,
        new_input: impl Into<String>,
    ) -> Self {
        self.input_mappings
            .insert(old_input.into(), new_input.into());
        self
    }

    pub fn with_output_mapping(
        mut self,
        old_output: impl Into<String>,
        new_output: impl Into<String>,
    ) -> Self {
        self.output_mappings
            .insert(old_output.into(), new_output.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeReplacementDiagnostic {
    pub code: String,
    pub node_id: NodeId,
    pub node_type: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NodeReplacementReport {
    pub graph: DiffusionGraph,
    pub replaced_nodes: Vec<NodeId>,
    pub diagnostics: Vec<NodeReplacementDiagnostic>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NodeReplacementEngine {
    rules: BTreeMap<String, NodeReplacementRule>,
}

impl NodeReplacementEngine {
    pub fn new(rules: impl IntoIterator<Item = NodeReplacementRule>) -> Self {
        Self {
            rules: rules
                .into_iter()
                .map(|rule| (rule.from_node_type.clone(), rule))
                .collect(),
        }
    }

    pub fn apply(
        &self,
        graph: &DiffusionGraph,
        registry: &ComfyNodeRegistry,
    ) -> NodeReplacementReport {
        let mut graph = graph.clone();
        let mut replaced_nodes = Vec::new();
        let mut diagnostics = Vec::new();
        let mut applied_rules = BTreeMap::new();

        for node in &mut graph.nodes {
            if registry.availability(&node.node_type).is_ok() {
                continue;
            }

            let Some(rule) = self.rules.get(&node.node_type) else {
                diagnostics.push(diagnostic(
                    REPLACEMENT_NOT_APPLIED_CODE,
                    node.id,
                    &node.node_type,
                    "missing node type has no native Sim replacement rule",
                ));
                continue;
            };

            if let Err(target_diagnostic) = registry.availability(&rule.to_node_type) {
                diagnostics.push(diagnostic(
                    REPLACEMENT_INVALID_TARGET_CODE,
                    node.id,
                    &node.node_type,
                    format!(
                        "replacement target `{}` is unavailable: {}",
                        rule.to_node_type, target_diagnostic.message
                    ),
                ));
                continue;
            }

            apply_rule_to_node(node, rule);
            applied_rules.insert(node.id, rule.clone());
            replaced_nodes.push(node.id);
        }

        for edge in &mut graph.edges {
            apply_rule_to_edge(edge, &applied_rules);
        }

        NodeReplacementReport {
            graph,
            replaced_nodes,
            diagnostics,
        }
    }
}

fn apply_rule_to_node(node: &mut GraphNode, rule: &NodeReplacementRule) {
    node.node_type = rule.to_node_type.clone();
    for port in &mut node.ports {
        match port.direction {
            PortDirection::Input => remap_port(port, &rule.input_mappings),
            PortDirection::Output => remap_port(port, &rule.output_mappings),
        }
    }

    let mut rewritten_metadata = std::collections::HashMap::new();
    for (key, value) in node.metadata.drain() {
        let key = rule.input_mappings.get(&key).cloned().unwrap_or(key);
        rewritten_metadata.insert(key, value);
    }
    node.metadata = rewritten_metadata;
}

fn apply_rule_to_edge(edge: &mut GraphEdge, applied_rules: &BTreeMap<NodeId, NodeReplacementRule>) {
    if let Some(rule) = applied_rules.get(&edge.source_node) {
        if let Some(source_port) = rule.output_mappings.get(&edge.source_port) {
            edge.source_port = source_port.clone();
        }
    }
    if let Some(rule) = applied_rules.get(&edge.target_node) {
        if let Some(target_port) = rule.input_mappings.get(&edge.target_port) {
            edge.target_port = target_port.clone();
        }
    }
}

fn remap_port(port: &mut NodePort, mappings: &BTreeMap<String, String>) {
    if let Some(name) = mappings.get(&port.name) {
        port.name = name.clone();
    }
}

fn diagnostic(
    code: &str,
    node_id: NodeId,
    node_type: &str,
    message: impl Into<String>,
) -> NodeReplacementDiagnostic {
    NodeReplacementDiagnostic {
        code: code.to_string(),
        node_id,
        node_type: node_type.to_string(),
        message: message.into(),
    }
}
