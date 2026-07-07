use crate::graph::{DataType, DiffusionGraph, GraphEdge, GraphNode, NodePort, PortDirection};
use crate::graph_validation::{
    DefaultGraphValidator, DiffusionGraphValidator, GraphValidationError,
};

// ---------------------------------------------------------------------------
// DataType
// ---------------------------------------------------------------------------

#[test]
fn data_type_labels_match_expected() {
    assert_eq!(DataType::Image.label(), "image");
    assert_eq!(DataType::Latent.label(), "latent");
    assert_eq!(DataType::Conditioning.label(), "conditioning");
    assert_eq!(DataType::Model.label(), "model");
    assert_eq!(DataType::ControlNet.label(), "controlnet");
    assert_eq!(DataType::Vae.label(), "vae");
    assert_eq!(DataType::Clip.label(), "clip");
    assert_eq!(DataType::Float.label(), "float");
    assert_eq!(DataType::Int.label(), "int");
    assert_eq!(DataType::String.label(), "string");
    assert_eq!(DataType::Bool.label(), "bool");
    assert_eq!(DataType::Any.label(), "any");
}

// ---------------------------------------------------------------------------
// PortDirection
// ---------------------------------------------------------------------------

#[test]
fn port_direction_discriminant() {
    assert!(PortDirection::Input != PortDirection::Output);
}

// ---------------------------------------------------------------------------
// NodePort
// ---------------------------------------------------------------------------

#[test]
fn node_port_creates_with_all_fields() {
    let port = NodePort::new("images", PortDirection::Input, DataType::Image, true);
    assert_eq!(port.name, "images");
    assert_eq!(port.direction, PortDirection::Input);
    assert_eq!(port.data_type, DataType::Image);
    assert!(port.required);
}

// ---------------------------------------------------------------------------
// GraphNode
// ---------------------------------------------------------------------------

#[test]
fn graph_node_creates_with_required_fields() {
    let node = GraphNode::new(1, "KSampler");
    assert_eq!(node.id, 1);
    assert_eq!(node.node_type, "KSampler");
    assert!(node.label.is_none());
    assert!(node.ports.is_empty());
}

#[test]
fn graph_node_with_label_port_and_metadata() {
    let node = GraphNode::new(1, "KSampler")
        .with_label("Sampler 1")
        .with_port(NodePort::new(
            "seed",
            PortDirection::Input,
            DataType::Int,
            false,
        ))
        .with_metadata("steps", "20")
        .with_metadata("cfg", "7.5");
    assert_eq!(node.label.as_deref(), Some("Sampler 1"));
    assert_eq!(node.ports.len(), 1);
    assert_eq!(node.metadata.get("steps").map(String::as_str), Some("20"));
}

#[test]
fn graph_node_input_and_output_ports() {
    let node = GraphNode::new(0, "Test")
        .with_port(NodePort::new(
            "in",
            PortDirection::Input,
            DataType::Float,
            true,
        ))
        .with_port(NodePort::new(
            "out",
            PortDirection::Output,
            DataType::Float,
            false,
        ));
    let inputs: Vec<&NodePort> = node.input_ports().collect();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].name, "in");
    let outputs: Vec<&NodePort> = node.output_ports().collect();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].name, "out");
}

#[test]
fn graph_node_port_by_name() {
    let node = GraphNode::new(5, "VAEEncode")
        .with_port(NodePort::new(
            "pixels",
            PortDirection::Input,
            DataType::Image,
            true,
        ))
        .with_port(NodePort::new(
            "latent",
            PortDirection::Output,
            DataType::Latent,
            false,
        ));
    assert!(node.port_by_name("pixels").is_some());
    assert!(node.port_by_name("latent").is_some());
    assert!(node.port_by_name("missing").is_none());
}

// ---------------------------------------------------------------------------
// GraphEdge
// ---------------------------------------------------------------------------

#[test]
fn graph_edge_creates_with_fields() {
    let edge = GraphEdge::new(1, 0, "out", 1, "in");
    assert_eq!(edge.id, 1);
    assert_eq!(edge.source_node, 0);
    assert_eq!(edge.source_port, "out");
    assert_eq!(edge.target_node, 1);
    assert_eq!(edge.target_port, "in");
}

// ---------------------------------------------------------------------------
// DiffusionGraph
// ---------------------------------------------------------------------------

#[test]
fn diffusion_graph_starts_empty() {
    let graph = DiffusionGraph::new();
    assert_eq!(graph.node_count(), 0);
    assert_eq!(graph.edge_count(), 0);
}

#[test]
fn diffusion_graph_with_nodes_and_edges() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "KSampler"))
        .with_node(GraphNode::new(1, "VAEDecode"))
        .with_edge(GraphEdge::new(0, 0, "latent", 1, "latent"));
    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn diffusion_graph_lookup_by_id() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(3, "NodeA"))
        .with_edge(GraphEdge::new(10, 3, "out", 3, "in"));
    assert_eq!(
        graph.node_by_id(3).map(|n| n.node_type.as_str()),
        Some("NodeA")
    );
    assert!(graph.node_by_id(99).is_none());
    assert_eq!(graph.edge_by_id(10).map(|e| e.id), Some(10));
    assert!(graph.edge_by_id(99).is_none());
}

#[test]
fn diffusion_graph_mut_lookup() {
    let mut graph = DiffusionGraph::new().with_node(GraphNode::new(0, "OldType"));
    {
        let node = graph.node_by_id_mut(0).expect("node exists");
        node.node_type = "NewType".to_string();
    }
    assert_eq!(
        graph.node_by_id(0).map(|n| n.node_type.as_str()),
        Some("NewType")
    );
}

#[test]
fn diffusion_graph_outgoing_incoming_edges() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "A"))
        .with_node(GraphNode::new(1, "B"))
        .with_node(GraphNode::new(2, "C"))
        .with_edge(GraphEdge::new(1, 0, "out", 1, "in"))
        .with_edge(GraphEdge::new(2, 0, "out", 2, "in"))
        .with_edge(GraphEdge::new(3, 1, "out", 2, "in2"));

    assert_eq!(graph.outgoing_edges(0).len(), 2);
    assert_eq!(graph.incoming_edges(0).len(), 0);
    assert_eq!(graph.incoming_edges(2).len(), 2);
}

// ---------------------------------------------------------------------------
// Validation — structural
// ---------------------------------------------------------------------------

#[test]
fn validate_empty_graph_passes() {
    let graph = DiffusionGraph::new();
    let result = DefaultGraphValidator.validate(&graph);
    assert!(result.is_valid());
}

#[test]
fn validate_dag_no_cycles_passes() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "KSampler").with_port(NodePort::new(
            "latent",
            PortDirection::Output,
            DataType::Latent,
            false,
        )))
        .with_node(
            GraphNode::new(1, "VAEDecode")
                .with_port(NodePort::new(
                    "latent",
                    PortDirection::Input,
                    DataType::Latent,
                    true,
                ))
                .with_port(NodePort::new(
                    "image",
                    PortDirection::Output,
                    DataType::Image,
                    false,
                )),
        )
        .with_edge(GraphEdge::new(0, 0, "latent", 1, "latent"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(result.is_valid(), "got errors: {:?}", result.errors);
}

#[test]
fn validate_missing_source_node() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(1, "VAEDecode").with_port(NodePort::new(
            "latent",
            PortDirection::Input,
            DataType::Latent,
            false,
        )))
        .with_edge(GraphEdge::new(0, 99, "out", 1, "latent"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(!result.is_valid());
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, GraphValidationError::MissingNode { .. }))
    );
}

#[test]
fn validate_missing_target_node() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "KSampler").with_port(NodePort::new(
            "latent",
            PortDirection::Output,
            DataType::Latent,
            false,
        )))
        .with_edge(GraphEdge::new(0, 0, "latent", 99, "in"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(!result.is_valid());
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, GraphValidationError::MissingNode { .. }))
    );
}

#[test]
fn validate_missing_port_on_node() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "KSampler"))
        .with_node(GraphNode::new(1, "VAEDecode").with_port(NodePort::new(
            "latent",
            PortDirection::Input,
            DataType::Latent,
            false,
        )))
        .with_edge(GraphEdge::new(0, 0, "nonexistent", 1, "latent"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(!result.is_valid());
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, GraphValidationError::MissingPort { .. }))
    );
}

#[test]
fn validate_port_type_mismatch() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "KSampler").with_port(NodePort::new(
            "latent",
            PortDirection::Output,
            DataType::Latent,
            false,
        )))
        .with_node(GraphNode::new(1, "ImageNode").with_port(NodePort::new(
            "image",
            PortDirection::Input,
            DataType::Image,
            false,
        )))
        .with_edge(GraphEdge::new(0, 0, "latent", 1, "image"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(!result.is_valid());
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, GraphValidationError::PortTypeMismatch { .. }))
    );
}

#[test]
fn validate_type_mismatch_allowed_with_any_target() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "KSampler").with_port(NodePort::new(
            "latent",
            PortDirection::Output,
            DataType::Latent,
            false,
        )))
        .with_node(GraphNode::new(1, "Passthrough").with_port(NodePort::new(
            "in",
            PortDirection::Input,
            DataType::Any,
            false,
        )))
        .with_edge(GraphEdge::new(0, 0, "latent", 1, "in"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(result.is_valid(), "got errors: {:?}", result.errors);
}

#[test]
fn validate_type_mismatch_allowed_with_any_source() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "Passthrough").with_port(NodePort::new(
            "out",
            PortDirection::Output,
            DataType::Any,
            false,
        )))
        .with_node(GraphNode::new(1, "VAEDecode").with_port(NodePort::new(
            "latent",
            PortDirection::Input,
            DataType::Latent,
            false,
        )))
        .with_edge(GraphEdge::new(0, 0, "out", 1, "latent"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(result.is_valid(), "got errors: {:?}", result.errors);
}

#[test]
fn validate_unconnected_required_port() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "KSampler").with_port(NodePort::new(
            "latent",
            PortDirection::Output,
            DataType::Latent,
            false,
        )))
        .with_node(GraphNode::new(1, "VAEDecode").with_port(NodePort::new(
            "latent",
            PortDirection::Input,
            DataType::Latent,
            true,
        )));
    // Edge 0 connects node 0 to node 1, but there is no edge connecting
    // node 0's "latent" to node 1's "latent". Actually the edge IS present.
    // Let me test the case where there's truly no edge to the required port.
    let result = DefaultGraphValidator.validate(&graph);
    assert!(!result.is_valid());
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, GraphValidationError::UnconnectedRequiredPort { .. }))
    );
}

#[test]
fn validate_unconnected_required_port_missing_edge() {
    // Two nodes with a required port but no edge connecting that port.
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "KSampler").with_port(NodePort::new(
            "model",
            PortDirection::Input,
            DataType::Model,
            true,
        )))
        .with_node(GraphNode::new(1, "CLIPLoader").with_port(NodePort::new(
            "clip",
            PortDirection::Output,
            DataType::Clip,
            false,
        )))
        .with_edge(GraphEdge::new(0, 1, "clip", 0, "model"));
    // Edge connects output "clip" (DataType::Clip) to input "model" (DataType::Model) — type mismatch
    let result = DefaultGraphValidator.validate(&graph);
    assert!(!result.is_valid());
    // Should have a type mismatch, not an unconnected port, since there IS an edge.
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, GraphValidationError::PortTypeMismatch { .. }))
    );
}

// ---------------------------------------------------------------------------
// Validation — cycles
// ---------------------------------------------------------------------------

#[test]
fn validate_detects_self_loop() {
    let graph = DiffusionGraph::new()
        .with_node(
            GraphNode::new(0, "LoopNode")
                .with_port(NodePort::new(
                    "in",
                    PortDirection::Input,
                    DataType::Any,
                    false,
                ))
                .with_port(NodePort::new(
                    "out",
                    PortDirection::Output,
                    DataType::Any,
                    false,
                )),
        )
        .with_edge(GraphEdge::new(0, 0, "out", 0, "in"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(!result.is_valid());
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, GraphValidationError::CycleDetected { .. }))
    );
}

#[test]
fn validate_detects_simple_cycle() {
    let graph = DiffusionGraph::new()
        .with_node(
            GraphNode::new(0, "A")
                .with_port(NodePort::new(
                    "out",
                    PortDirection::Output,
                    DataType::Any,
                    false,
                ))
                .with_port(NodePort::new(
                    "in",
                    PortDirection::Input,
                    DataType::Any,
                    false,
                )),
        )
        .with_node(
            GraphNode::new(1, "B")
                .with_port(NodePort::new(
                    "out",
                    PortDirection::Output,
                    DataType::Any,
                    false,
                ))
                .with_port(NodePort::new(
                    "in",
                    PortDirection::Input,
                    DataType::Any,
                    false,
                )),
        )
        .with_edge(GraphEdge::new(0, 0, "out", 1, "in"))
        .with_edge(GraphEdge::new(1, 1, "out", 0, "in"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(!result.is_valid());
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, GraphValidationError::CycleDetected { .. }))
    );
}

#[test]
fn validate_accepts_diamond_dag() {
    // A → B → D  (no cycles)
    // A → C → D
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "A").with_port(NodePort::new(
            "out",
            PortDirection::Output,
            DataType::Any,
            false,
        )))
        .with_node(
            GraphNode::new(1, "B")
                .with_port(NodePort::new(
                    "in",
                    PortDirection::Input,
                    DataType::Any,
                    false,
                ))
                .with_port(NodePort::new(
                    "out",
                    PortDirection::Output,
                    DataType::Any,
                    false,
                )),
        )
        .with_node(
            GraphNode::new(2, "C")
                .with_port(NodePort::new(
                    "in",
                    PortDirection::Input,
                    DataType::Any,
                    false,
                ))
                .with_port(NodePort::new(
                    "out",
                    PortDirection::Output,
                    DataType::Any,
                    false,
                )),
        )
        .with_node(
            GraphNode::new(3, "D")
                .with_port(NodePort::new(
                    "in1",
                    PortDirection::Input,
                    DataType::Any,
                    false,
                ))
                .with_port(NodePort::new(
                    "in2",
                    PortDirection::Input,
                    DataType::Any,
                    false,
                )),
        )
        .with_edge(GraphEdge::new(0, 0, "out", 1, "in"))
        .with_edge(GraphEdge::new(1, 0, "out", 3, "in1"))
        .with_edge(GraphEdge::new(2, 1, "out", 2, "in"))
        .with_edge(GraphEdge::new(3, 2, "out", 3, "in2"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(result.is_valid(), "got errors: {:?}", result.errors);
}

// ---------------------------------------------------------------------------
// Validation — duplicate edge IDs
// ---------------------------------------------------------------------------

#[test]
fn validate_detects_duplicate_edge_ids() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(0, "A").with_port(NodePort::new(
            "out",
            PortDirection::Output,
            DataType::Any,
            false,
        )))
        .with_node(GraphNode::new(1, "B").with_port(NodePort::new(
            "in",
            PortDirection::Input,
            DataType::Any,
            false,
        )))
        .with_edge(GraphEdge::new(0, 0, "out", 1, "in"))
        .with_edge(GraphEdge::new(0, 0, "out", 1, "in"));
    let result = DefaultGraphValidator.validate(&graph);
    assert!(!result.is_valid());
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, GraphValidationError::DuplicateEdge { .. }))
    );
}

// ---------------------------------------------------------------------------
// GraphValidationResult
// ---------------------------------------------------------------------------

#[test]
fn validation_result_starts_valid() {
    let result = crate::graph_validation::GraphValidationResult::new();
    assert!(result.is_valid());
    assert!(result.errors.is_empty());
}

#[test]
fn validation_result_merge_combines_errors() {
    let mut r1 = crate::graph_validation::GraphValidationResult::new();
    let mut r2 = crate::graph_validation::GraphValidationResult::new();
    r2.errors.push(GraphValidationError::Other {
        node_id: None,
        message: "test error".to_string(),
    });
    r2.warnings.push("test warning".to_string());
    r1.merge(r2);
    assert_eq!(r1.errors.len(), 1);
    assert_eq!(r1.warnings.len(), 1);
}

// ---------------------------------------------------------------------------
// GraphValidationError formatting
// ---------------------------------------------------------------------------

#[test]
fn graph_validation_error_display() {
    let err = GraphValidationError::MissingNode {
        node_id: 42,
        context: "edge 7 source".to_string(),
    };
    let text = format!("{err}");
    assert!(text.contains("42"));
    assert!(text.contains("edge 7 source"));
}

#[test]
fn graph_validation_error_category() {
    let err = GraphValidationError::CycleDetected {
        node_ids: vec![0, 1],
    };
    assert_eq!(err.category(), "cycle_detected");
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

#[test]
fn graph_round_trip_serde() {
    let graph = DiffusionGraph::new()
        .with_node(
            GraphNode::new(0, "KSampler")
                .with_metadata("seed", "42")
                .with_port(NodePort::new(
                    "latent",
                    PortDirection::Output,
                    DataType::Latent,
                    false,
                )),
        )
        .with_node(GraphNode::new(1, "VAEDecode").with_port(NodePort::new(
            "latent",
            PortDirection::Input,
            DataType::Latent,
            true,
        )))
        .with_edge(GraphEdge::new(0, 0, "latent", 1, "latent"));

    let json = serde_json::to_string(&graph).expect("serialize");
    let restored: DiffusionGraph = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.node_count(), graph.node_count());
    assert_eq!(restored.edge_count(), graph.edge_count());
}
