use crate::{
    ComfyNodeRegistry, DataType, DiffusionGraph, GraphEdge, GraphNode, NodePort,
    NodeReplacementEngine, NodeReplacementRule, PortDirection,
};

#[test]
fn replacement_engine_rewrites_missing_node_type_ports_metadata_and_links() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let graph = DiffusionGraph::new()
        .with_node(
            GraphNode::new(1, "LegacyTextEncode")
                .with_port(NodePort::new(
                    "prompt",
                    PortDirection::Input,
                    DataType::String,
                    true,
                ))
                .with_port(NodePort::new(
                    "old_conditioning",
                    PortDirection::Output,
                    DataType::Conditioning,
                    false,
                ))
                .with_metadata("prompt", "a small castle"),
        )
        .with_node(GraphNode::new(2, "KSampler").with_port(NodePort::new(
            "positive",
            PortDirection::Input,
            DataType::Conditioning,
            true,
        )))
        .with_edge(GraphEdge::new(1, 1, "old_conditioning", 2, "positive"));
    let engine = NodeReplacementEngine::new([NodeReplacementRule::new(
        "LegacyTextEncode",
        "CLIPTextEncode",
    )
    .with_input_mapping("prompt", "text")
    .with_output_mapping("old_conditioning", "CONDITIONING")]);

    let report = engine.apply(&graph, &registry);

    assert_eq!(report.replaced_nodes, vec![1]);
    assert!(report.diagnostics.is_empty());

    let node = report.graph.node_by_id(1).expect("rewritten node");
    assert_eq!(node.node_type, "CLIPTextEncode");
    assert!(node.port_by_name("text").is_some());
    assert_eq!(
        node.metadata.get("text").map(String::as_str),
        Some("a small castle")
    );

    let edge = report.graph.edge_by_id(1).expect("rewritten edge");
    assert_eq!(edge.source_port, "CONDITIONING");
    assert_eq!(edge.target_port, "positive");
}

#[test]
fn replacement_engine_rewrites_target_input_links_when_replaced_node_consumes_links() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let graph = DiffusionGraph::new()
        .with_node(
            GraphNode::new(1, "CheckpointLoaderSimple").with_port(NodePort::new(
                "MODEL",
                PortDirection::Output,
                DataType::Model,
                false,
            )),
        )
        .with_node(GraphNode::new(2, "LegacySampler").with_port(NodePort::new(
            "old_model",
            PortDirection::Input,
            DataType::Model,
            true,
        )))
        .with_edge(GraphEdge::new(7, 1, "MODEL", 2, "old_model"));
    let engine =
        NodeReplacementEngine::new([NodeReplacementRule::new("LegacySampler", "KSampler")
            .with_input_mapping("old_model", "model")]);

    let report = engine.apply(&graph, &registry);

    assert_eq!(report.replaced_nodes, vec![2]);
    assert_eq!(
        report.graph.node_by_id(2).expect("node").node_type,
        "KSampler"
    );
    assert_eq!(
        report.graph.edge_by_id(7).expect("edge").target_port,
        "model"
    );
}

#[test]
fn replacement_engine_leaves_registered_nodes_unchanged_even_when_rule_exists() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let graph = DiffusionGraph::new().with_node(GraphNode::new(1, "KSampler"));
    let engine = NodeReplacementEngine::new([NodeReplacementRule::new("KSampler", "VAEDecode")]);

    let report = engine.apply(&graph, &registry);

    assert!(report.replaced_nodes.is_empty());
    assert!(report.diagnostics.is_empty());
    assert_eq!(
        report
            .graph
            .node_by_id(1)
            .map(|node| node.node_type.as_str()),
        Some("KSampler")
    );
}

#[test]
fn replacement_engine_reports_missing_rule_and_invalid_target_without_rewriting() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(1, "MissingWithoutRule"))
        .with_node(GraphNode::new(2, "MissingWithBadTarget"));
    let engine = NodeReplacementEngine::new([NodeReplacementRule::new(
        "MissingWithBadTarget",
        "StillMissing",
    )]);

    let report = engine.apply(&graph, &registry);

    assert!(report.replaced_nodes.is_empty());
    assert_eq!(report.diagnostics.len(), 2);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_node_replacement::REPLACEMENT_NOT_APPLIED_CODE
            && diagnostic.node_id == 1
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_node_replacement::REPLACEMENT_INVALID_TARGET_CODE
            && diagnostic.node_id == 2
    }));
    assert_eq!(
        report
            .graph
            .node_by_id(2)
            .map(|node| node.node_type.as_str()),
        Some("MissingWithBadTarget")
    );
}
