use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Deserialize;
use world_model::{
    ComfyExecutionPlanner, ComfyExecutorDispatch, ComfyNodeExecutionOutcome, ComfyNodeRegistry,
    ComfyNodeRuntime, ComfyPromptGraphValidator, DataType, DiffusionGraph, ExecutionPlanRequest,
    GraphEdge, GraphNode, NodePort, PortDirection, graph::NodeId,
};

const CORE_NODES: &str = include_str!("../fixtures/comfy/core_nodes.json");

#[derive(Debug, Deserialize)]
struct CoreNodeFixture {
    schema_version: u32,
    native_sim_records: bool,
    comfyui_passthrough: bool,
    object_info: ObjectInfoFixture,
    prompts: Vec<PromptFixture>,
}

#[derive(Debug, Deserialize)]
struct ObjectInfoFixture {
    minimum_node_count: usize,
    required_nodes: Vec<String>,
    required_categories: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PromptFixture {
    id: String,
    target_node: NodeId,
    nodes: Vec<NodeFixture>,
    edges: Vec<EdgeFixture>,
    expected_dispatch_nodes: Vec<NodeId>,
}

#[derive(Debug, Deserialize)]
struct NodeFixture {
    id: NodeId,
    #[serde(rename = "type")]
    node_type: String,
    literals: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct EdgeFixture {
    from: NodeId,
    from_port: String,
    to: NodeId,
    to_port: String,
}

#[test]
fn core_node_fixture_matches_native_object_info_registry() {
    let fixture = core_node_fixture();
    assert_eq!(fixture.schema_version, 1);
    assert!(fixture.native_sim_records);
    assert!(!fixture.comfyui_passthrough);

    let registry = ComfyNodeRegistry::with_core_nodes();
    let object_info = registry.object_info(None);
    assert!(object_info.nodes.len() >= fixture.object_info.minimum_node_count);

    for node_id in &fixture.object_info.required_nodes {
        let node = object_info
            .nodes
            .get(node_id)
            .unwrap_or_else(|| panic!("missing fixture node {node_id}"));
        assert!(
            !node.api_node,
            "core fixture node {node_id} must be a native Sim core node, not an API proxy"
        );
    }

    let categories = object_info
        .nodes
        .values()
        .map(|node| node.category.as_str())
        .collect::<BTreeSet<_>>();
    for category in &fixture.object_info.required_categories {
        assert!(
            categories.contains(category.as_str()),
            "missing core fixture category {category}"
        );
    }
}

#[test]
fn core_node_fixture_prompts_validate_plan_and_execute_natively() {
    let fixture = core_node_fixture();
    let registry = ComfyNodeRegistry::with_core_nodes();
    let validator = ComfyPromptGraphValidator::new(registry.clone());
    let planner = ComfyExecutionPlanner::new();

    for prompt in &fixture.prompts {
        let graph = graph_from_prompt(prompt, &registry);
        let validation = validator.validate(&graph, [prompt.target_node]);
        assert!(
            validation.is_valid(),
            "prompt {} failed validation: {:?}",
            prompt.id,
            validation.errors
        );

        let plan = planner
            .plan(&graph, ExecutionPlanRequest::new([prompt.target_node]))
            .unwrap_or_else(|errors| panic!("prompt {} failed planning: {errors:?}", prompt.id));
        assert_eq!(plan.target_nodes, vec![prompt.target_node]);

        let mut runtime = FixtureRuntime;
        let report = world_model::ComfyNodeExecutor::new().execute(&graph, &plan, &mut runtime);
        assert!(
            report.diagnostics.is_empty(),
            "prompt {} produced executor diagnostics: {:?}",
            prompt.id,
            report.diagnostics
        );

        let dispatch_nodes = report
            .records
            .iter()
            .filter(|record| {
                matches!(
                    record.dispatch,
                    Some(ComfyExecutorDispatch::DiffusionWorldModel { .. })
                )
            })
            .map(|record| record.node_id)
            .collect::<BTreeSet<_>>();
        let expected_dispatch_nodes = prompt
            .expected_dispatch_nodes
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            dispatch_nodes, expected_dispatch_nodes,
            "prompt {} dispatch coverage differed",
            prompt.id
        );
    }
}

fn core_node_fixture() -> CoreNodeFixture {
    serde_json::from_str(CORE_NODES).expect("core node fixture should deserialize")
}

fn graph_from_prompt(prompt: &PromptFixture, registry: &ComfyNodeRegistry) -> DiffusionGraph {
    let nodes = prompt
        .nodes
        .iter()
        .map(|node| graph_node_from_fixture(node, registry))
        .collect::<Vec<_>>();
    let edges = prompt
        .edges
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            GraphEdge::new(
                index + 1,
                edge.from,
                edge.from_port.clone(),
                edge.to,
                edge.to_port.clone(),
            )
        })
        .collect::<Vec<_>>();

    edges.into_iter().fold(
        nodes
            .into_iter()
            .fold(DiffusionGraph::new(), |graph, node| graph.with_node(node)),
        |graph, edge| graph.with_edge(edge),
    )
}

fn graph_node_from_fixture(node: &NodeFixture, registry: &ComfyNodeRegistry) -> GraphNode {
    let definition = registry
        .get(&node.node_type)
        .unwrap_or_else(|| panic!("fixture references unregistered node {}", node.node_type));
    let graph_node =
        GraphNode::new(node.id, node.node_type.clone()).with_label(definition.display_name.clone());
    let graph_node = definition
        .inputs
        .iter()
        .fold(graph_node, |graph_node, input| {
            graph_node.with_port(NodePort::new(
                input.name.clone(),
                PortDirection::Input,
                input.data_type,
                input.required,
            ))
        });
    let graph_node = definition
        .outputs
        .iter()
        .fold(graph_node, |graph_node, output| {
            graph_node.with_port(NodePort::new(
                output.name.clone(),
                PortDirection::Output,
                output.data_type,
                false,
            ))
        });

    node.literals
        .iter()
        .fold(graph_node, |graph_node, (key, value)| {
            graph_node.with_metadata(key.clone(), value.clone())
        })
}

struct FixtureRuntime;

impl ComfyNodeRuntime for FixtureRuntime {
    fn execute_node(
        &mut self,
        node: &GraphNode,
    ) -> Result<ComfyNodeExecutionOutcome, world_model::ComfyExecutorDiagnostic> {
        let outputs = node
            .output_ports()
            .map(|port| (port.name.clone(), output_value(node, port.data_type)))
            .collect::<BTreeMap<_, _>>();

        Ok(ComfyNodeExecutionOutcome {
            outputs,
            provenance: vec![format!("native-sim:{}", node.node_type)],
            ..ComfyNodeExecutionOutcome::completed()
        })
    }
}

fn output_value(node: &GraphNode, data_type: DataType) -> String {
    format!("{}:{}", node.id, data_type.label())
}
