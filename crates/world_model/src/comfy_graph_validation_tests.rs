use crate::{
    ComfyNodeRegistry, ComfyPromptGraphValidator, ComfyValidationCapabilities, DataType,
    DiffusionGraph, GraphEdge, GraphNode, GraphValidationError, NodePort, PortDirection,
};

#[test]
fn validator_accepts_registered_graph_with_literal_required_inputs_and_partial_target() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let graph = DiffusionGraph::new()
        .with_node(
            GraphNode::new(1, "LoadImage")
                .with_port(NodePort::new(
                    "image",
                    PortDirection::Input,
                    DataType::String,
                    true,
                ))
                .with_port(NodePort::new(
                    "IMAGE",
                    PortDirection::Output,
                    DataType::Image,
                    false,
                ))
                .with_metadata("image", "input.png"),
        )
        .with_node(
            GraphNode::new(2, "VAEEncode")
                .with_port(NodePort::new(
                    "pixels",
                    PortDirection::Input,
                    DataType::Image,
                    true,
                ))
                .with_port(NodePort::new(
                    "LATENT",
                    PortDirection::Output,
                    DataType::Latent,
                    false,
                )),
        )
        .with_edge(GraphEdge::new(1, 1, "IMAGE", 2, "pixels"));

    let result = ComfyPromptGraphValidator::new(registry).validate(&graph, [2]);

    assert!(result.is_valid(), "got errors: {:?}", result.errors);
}

#[test]
fn validator_reports_unknown_nodes_required_inputs_bad_links_and_type_mismatches() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let graph = DiffusionGraph::new()
        .with_node(
            GraphNode::new(1, "MissingComfyNode")
                .with_port(NodePort::new(
                    "out",
                    PortDirection::Output,
                    DataType::Latent,
                    false,
                ))
                .with_port(NodePort::new(
                    "required",
                    PortDirection::Input,
                    DataType::String,
                    true,
                )),
        )
        .with_node(GraphNode::new(2, "VAEDecode").with_port(NodePort::new(
            "samples",
            PortDirection::Input,
            DataType::Latent,
            true,
        )))
        .with_node(GraphNode::new(3, "SaveImage").with_port(NodePort::new(
            "images",
            PortDirection::Input,
            DataType::Image,
            true,
        )))
        .with_edge(GraphEdge::new(1, 1, "missing_output", 2, "samples"))
        .with_edge(GraphEdge::new(2, 1, "out", 3, "images"));

    let result = ComfyPromptGraphValidator::new(registry).validate(&graph, [99]);

    assert!(result.errors.iter().any(|error| matches!(
        error,
        GraphValidationError::UnknownNodeType { node_id: 1, .. }
    )));
    assert!(result.errors.iter().any(|error| matches!(
        error,
        GraphValidationError::UnconnectedRequiredPort {
            node_id: 1,
            port_name
        } if port_name == "required"
    )));
    assert!(
        result
            .errors
            .iter()
            .any(|error| matches!(error, GraphValidationError::MissingPort { .. }))
    );
    assert!(result.errors.iter().any(|error| matches!(
        error,
        GraphValidationError::PortTypeMismatch { edge_id: 2, .. }
    )));
    assert!(result.errors.iter().any(|error| matches!(
        error,
        GraphValidationError::MissingNode {
            node_id: 99,
            context
        } if context == "partial execution target"
    )));
}

#[test]
fn validator_preserves_cycle_detection() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let graph = DiffusionGraph::new()
        .with_node(
            GraphNode::new(1, "LoadImage")
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
        .with_edge(GraphEdge::new(1, 1, "out", 1, "in"));

    let result = ComfyPromptGraphValidator::new(registry).validate(&graph, []);

    assert!(
        result
            .errors
            .iter()
            .any(|error| matches!(error, GraphValidationError::CycleDetected { .. }))
    );
}

#[test]
fn validator_reports_missing_provider_model_folder_and_asset_capabilities() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let graph = DiffusionGraph::new().with_node(
        GraphNode::new(1, "LoadImage")
            .with_metadata("requires_provider", "openai")
            .with_metadata("requires_model_folder", "checkpoints")
            .with_metadata("requires_asset_capability", "uploads"),
    );
    let capabilities = ComfyValidationCapabilities::default().with_provider("anthropic");

    let result = ComfyPromptGraphValidator::new(registry)
        .with_capabilities(capabilities)
        .validate(&graph, []);

    assert!(result.errors.iter().any(|error| matches!(
        error,
        GraphValidationError::UnsupportedBackend { backend, .. } if backend == "provider:openai"
    )));
    assert!(result.errors.iter().any(|error| matches!(
        error,
        GraphValidationError::UnsupportedBackend { backend, .. } if backend == "model_folder:checkpoints"
    )));
    assert!(result.errors.iter().any(|error| matches!(
        error,
        GraphValidationError::UnsupportedBackend { backend, .. } if backend == "asset:uploads"
    )));
}

#[test]
fn validator_accepts_declared_provider_model_folder_and_asset_capabilities() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let graph = DiffusionGraph::new().with_node(
        GraphNode::new(1, "LoadImage")
            .with_metadata("requires_provider", "openai")
            .with_metadata("requires_model_folder", "checkpoints")
            .with_metadata("requires_asset_capability", "uploads"),
    );
    let capabilities = ComfyValidationCapabilities::default()
        .with_provider("openai")
        .with_model_folder("checkpoints")
        .with_asset_capability("uploads");

    let result = ComfyPromptGraphValidator::new(registry)
        .with_capabilities(capabilities)
        .validate(&graph, []);

    assert!(result.is_valid(), "got errors: {:?}", result.errors);
}
