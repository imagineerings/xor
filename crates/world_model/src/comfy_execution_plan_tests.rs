use crate::{
    ComfyCachePolicy, ComfyExecutionPlanner, DataType, DiffusionGraph, ExecutionPlanRequest,
    GraphEdge, GraphNode, GraphValidationError, NodeCacheEntry, NodeCacheSnapshot, NodePort,
    PortDirection, cache_key_for_node,
};

#[test]
fn planner_builds_dependency_closure_for_partial_targets() {
    let graph = sampling_graph();

    let plan = ComfyExecutionPlanner::new()
        .plan(&graph, ExecutionPlanRequest::new([4]))
        .expect("plan");

    assert_eq!(plan.target_nodes, vec![4]);
    assert_eq!(plan.dependency_closure, vec![1, 2, 3, 4]);
    assert_eq!(plan.execution_order, vec![1, 2, 3, 4]);
    assert_eq!(plan.dirty_nodes, vec![1, 2, 3, 4]);
}

#[test]
fn planner_defaults_to_all_nodes_when_partial_targets_are_empty() {
    let graph = sampling_graph();

    let plan = ComfyExecutionPlanner::new()
        .plan(&graph, ExecutionPlanRequest::new([]))
        .expect("plan");

    assert_eq!(plan.target_nodes, vec![1, 2, 3, 4, 5]);
    assert_eq!(plan.dependency_closure, vec![1, 2, 3, 4, 5]);
    assert_eq!(plan.execution_order, vec![1, 2, 3, 4, 5]);
}

#[test]
fn planner_reuses_cached_nodes_and_only_executes_dirty_nodes() {
    let graph = sampling_graph();
    let snapshot = NodeCacheSnapshot::new([
        NodeCacheEntry::new(1, cache_key_for_node(graph.node_by_id(1).expect("node"))),
        NodeCacheEntry::new(2, cache_key_for_node(graph.node_by_id(2).expect("node"))),
    ]);

    let plan = ComfyExecutionPlanner::new()
        .plan(
            &graph,
            ExecutionPlanRequest::new([4]).with_cache_snapshot(snapshot),
        )
        .expect("plan");

    assert_eq!(plan.reusable_nodes, vec![1, 2]);
    assert_eq!(plan.execution_order, vec![3, 4]);
    assert_eq!(plan.dirty_nodes, vec![3, 4]);
}

#[test]
fn planner_executes_all_dependency_nodes_when_cache_policy_is_none() {
    let graph = sampling_graph();
    let snapshot = NodeCacheSnapshot::new([NodeCacheEntry::new(
        1,
        cache_key_for_node(graph.node_by_id(1).expect("node")),
    )]);

    let plan = ComfyExecutionPlanner::new()
        .plan(
            &graph,
            ExecutionPlanRequest::new([4])
                .with_cache_snapshot(snapshot)
                .with_cache_policy(ComfyCachePolicy::None),
        )
        .expect("plan");

    assert!(plan.reusable_nodes.is_empty());
    assert_eq!(plan.execution_order, vec![1, 2, 3, 4]);
}

#[test]
fn planner_reports_missing_partial_targets() {
    let graph = sampling_graph();

    let errors = ComfyExecutionPlanner::new()
        .plan(&graph, ExecutionPlanRequest::new([99]))
        .expect_err("missing target");

    assert!(errors.iter().any(|error| matches!(
        error,
        GraphValidationError::MissingNode {
            node_id: 99,
            context
        } if context == "execution plan target"
    )));
}

fn sampling_graph() -> DiffusionGraph {
    DiffusionGraph::new()
        .with_node(
            GraphNode::new(1, "CheckpointLoaderSimple").with_port(NodePort::new(
                "MODEL",
                PortDirection::Output,
                DataType::Model,
                false,
            )),
        )
        .with_node(
            GraphNode::new(2, "CLIPTextEncode")
                .with_port(NodePort::new(
                    "CONDITIONING",
                    PortDirection::Output,
                    DataType::Conditioning,
                    false,
                ))
                .with_metadata("text", "a castle"),
        )
        .with_node(
            GraphNode::new(3, "KSampler")
                .with_port(NodePort::new(
                    "model",
                    PortDirection::Input,
                    DataType::Model,
                    true,
                ))
                .with_port(NodePort::new(
                    "positive",
                    PortDirection::Input,
                    DataType::Conditioning,
                    true,
                ))
                .with_port(NodePort::new(
                    "LATENT",
                    PortDirection::Output,
                    DataType::Latent,
                    false,
                )),
        )
        .with_node(
            GraphNode::new(4, "VAEDecode")
                .with_port(NodePort::new(
                    "samples",
                    PortDirection::Input,
                    DataType::Latent,
                    true,
                ))
                .with_port(NodePort::new(
                    "IMAGE",
                    PortDirection::Output,
                    DataType::Image,
                    false,
                )),
        )
        .with_node(GraphNode::new(5, "SaveImage").with_port(NodePort::new(
            "images",
            PortDirection::Input,
            DataType::Image,
            true,
        )))
        .with_edge(GraphEdge::new(1, 1, "MODEL", 3, "model"))
        .with_edge(GraphEdge::new(2, 2, "CONDITIONING", 3, "positive"))
        .with_edge(GraphEdge::new(3, 3, "LATENT", 4, "samples"))
        .with_edge(GraphEdge::new(4, 4, "IMAGE", 5, "images"))
}
