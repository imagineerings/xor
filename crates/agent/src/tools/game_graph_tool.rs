use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use world_model::{
    DataType, DefaultGraphValidator, DiffusionGraph, DiffusionGraphValidator, GraphEdge, GraphNode,
    GraphValidationResult, NodePort, PortDirection, graph::NodeId,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimGameGraphToolInput {
    #[serde(default)]
    pub base_graph: SimDiffusionGraphInput,
    #[serde(default)]
    pub add_nodes: Vec<SimGraphNodeInput>,
    #[serde(default)]
    pub add_edges: Vec<SimGraphEdgeInput>,
    #[serde(default)]
    pub remove_node_ids: Vec<NodeId>,
    #[serde(default)]
    pub remove_edge_ids: Vec<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SimDiffusionGraphInput {
    #[serde(default)]
    pub nodes: Vec<SimGraphNodeInput>,
    #[serde(default)]
    pub edges: Vec<SimGraphEdgeInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimGraphNodeInput {
    pub id: NodeId,
    pub node_type: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub ports: Vec<SimNodePortInput>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimNodePortInput {
    pub name: String,
    pub direction: SimPortDirectionInput,
    pub data_type: SimDataTypeInput,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimGraphEdgeInput {
    pub id: usize,
    pub source_node: NodeId,
    pub source_port: String,
    pub target_node: NodeId,
    pub target_port: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimPortDirectionInput {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimDataTypeInput {
    Image,
    Latent,
    Conditioning,
    Model,
    ControlNet,
    Vae,
    Clip,
    Float,
    Int,
    String,
    Bool,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimGameGraphToolOutput {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<DiffusionGraph>,
    pub validation: GraphValidationResult,
    pub message: String,
}

impl From<SimGameGraphToolOutput> for LanguageModelToolResultContent {
    fn from(output: SimGameGraphToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| format!("failed to serialize Sim graph tool output: {error}"))
            .into()
    }
}

pub struct SimGameGraphTool;

impl AgentTool for SimGameGraphTool {
    type Input = SimGameGraphToolInput;
    type Output = SimGameGraphToolOutput;

    const NAME: &'static str = "sim_game_graph";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Other
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Editing Sim graph".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| match input.recv().await {
            Ok(input) => {
                let output = run_sim_game_graph_tool(input);
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().content(vec![output.message.clone().into()]),
                );
                if output.success {
                    Ok(output)
                } else {
                    Err(output)
                }
            }
            Err(error) => Err(SimGameGraphToolOutput {
                success: false,
                graph: None,
                validation: GraphValidationResult::new(),
                message: format!("Failed to read Sim graph tool input: {error}"),
            }),
        })
    }
}

pub fn run_sim_game_graph_tool(input: SimGameGraphToolInput) -> SimGameGraphToolOutput {
    let graph = apply_graph_edit(input);
    let validation = DefaultGraphValidator.validate(&graph);
    if validation.is_valid() {
        SimGameGraphToolOutput {
            success: true,
            graph: Some(graph),
            validation,
            message: "Sim graph edit validated".to_string(),
        }
    } else {
        SimGameGraphToolOutput {
            success: false,
            graph: None,
            message: format!(
                "Sim graph edit rejected with {} validation error(s)",
                validation.errors.len()
            ),
            validation,
        }
    }
}

fn apply_graph_edit(input: SimGameGraphToolInput) -> DiffusionGraph {
    let mut graph = DiffusionGraph::from(input.base_graph);
    graph
        .nodes
        .retain(|node| !input.remove_node_ids.contains(&node.id));
    graph.edges.retain(|edge| {
        !input.remove_edge_ids.contains(&edge.id)
            && !input.remove_node_ids.contains(&edge.source_node)
            && !input.remove_node_ids.contains(&edge.target_node)
    });
    graph
        .nodes
        .extend(input.add_nodes.into_iter().map(GraphNode::from));
    graph
        .edges
        .extend(input.add_edges.into_iter().map(GraphEdge::from));
    graph
}

impl From<SimDiffusionGraphInput> for DiffusionGraph {
    fn from(input: SimDiffusionGraphInput) -> Self {
        Self {
            nodes: input.nodes.into_iter().map(GraphNode::from).collect(),
            edges: input.edges.into_iter().map(GraphEdge::from).collect(),
        }
    }
}

impl From<SimGraphNodeInput> for GraphNode {
    fn from(input: SimGraphNodeInput) -> Self {
        Self {
            id: input.id,
            node_type: input.node_type,
            label: input.label,
            ports: input.ports.into_iter().map(NodePort::from).collect(),
            metadata: input.metadata,
        }
    }
}

impl From<SimNodePortInput> for NodePort {
    fn from(input: SimNodePortInput) -> Self {
        Self {
            name: input.name,
            direction: PortDirection::from(input.direction),
            data_type: DataType::from(input.data_type),
            required: input.required,
        }
    }
}

impl From<SimGraphEdgeInput> for GraphEdge {
    fn from(input: SimGraphEdgeInput) -> Self {
        Self {
            id: input.id,
            source_node: input.source_node,
            source_port: input.source_port,
            target_node: input.target_node,
            target_port: input.target_port,
        }
    }
}

impl From<SimPortDirectionInput> for PortDirection {
    fn from(input: SimPortDirectionInput) -> Self {
        match input {
            SimPortDirectionInput::Input => Self::Input,
            SimPortDirectionInput::Output => Self::Output,
        }
    }
}

impl From<SimDataTypeInput> for DataType {
    fn from(input: SimDataTypeInput) -> Self {
        match input {
            SimDataTypeInput::Image => Self::Image,
            SimDataTypeInput::Latent => Self::Latent,
            SimDataTypeInput::Conditioning => Self::Conditioning,
            SimDataTypeInput::Model => Self::Model,
            SimDataTypeInput::ControlNet => Self::ControlNet,
            SimDataTypeInput::Vae => Self::Vae,
            SimDataTypeInput::Clip => Self::Clip,
            SimDataTypeInput::Float => Self::Float,
            SimDataTypeInput::Int => Self::Int,
            SimDataTypeInput::String => Self::String,
            SimDataTypeInput::Bool => Self::Bool,
            SimDataTypeInput::Any => Self::Any,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_game_graph_tool_applies_validated_graph_edit() {
        let output = run_sim_game_graph_tool(SimGameGraphToolInput {
            base_graph: SimDiffusionGraphInput::default(),
            add_nodes: vec![
                node(
                    1,
                    "Prompt",
                    vec![port("out", SimPortDirectionInput::Output)],
                ),
                node(2, "Preview", vec![port("in", SimPortDirectionInput::Input)]),
            ],
            add_edges: vec![SimGraphEdgeInput {
                id: 1,
                source_node: 1,
                source_port: "out".to_string(),
                target_node: 2,
                target_port: "in".to_string(),
            }],
            remove_node_ids: Vec::new(),
            remove_edge_ids: Vec::new(),
        });

        assert!(output.success);
        assert_eq!(output.graph.expect("graph").edge_count(), 1);
    }

    #[test]
    fn sim_game_graph_tool_rejects_unvalidated_graph_edit() {
        let output = run_sim_game_graph_tool(SimGameGraphToolInput {
            base_graph: SimDiffusionGraphInput::default(),
            add_nodes: vec![node(
                2,
                "Preview",
                vec![port("in", SimPortDirectionInput::Input)],
            )],
            add_edges: vec![SimGraphEdgeInput {
                id: 1,
                source_node: 99,
                source_port: "out".to_string(),
                target_node: 2,
                target_port: "in".to_string(),
            }],
            remove_node_ids: Vec::new(),
            remove_edge_ids: Vec::new(),
        });

        assert!(!output.success);
        assert!(output.graph.is_none());
        assert!(!output.validation.errors.is_empty());
    }

    fn node(id: NodeId, node_type: &str, ports: Vec<SimNodePortInput>) -> SimGraphNodeInput {
        SimGraphNodeInput {
            id,
            node_type: node_type.to_string(),
            label: None,
            ports,
            metadata: HashMap::new(),
        }
    }

    fn port(name: &str, direction: SimPortDirectionInput) -> SimNodePortInput {
        SimNodePortInput {
            name: name.to_string(),
            direction,
            data_type: SimDataTypeInput::Any,
            required: false,
        }
    }
}
