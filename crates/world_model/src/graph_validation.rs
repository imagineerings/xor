use serde::{Deserialize, Serialize};

use crate::graph::{DataType, DiffusionGraph, GraphNode, NodeId, NodePort, PortDirection};

// ---------------------------------------------------------------------------
// Validation error
// ---------------------------------------------------------------------------

/// An error found during graph validation.
///
/// Each variant carries enough context to report the issue to the user or
/// agent without requiring a separate lookup.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GraphValidationError {
    /// A referenced node does not exist in the graph.
    MissingNode { node_id: NodeId, context: String },
    /// A port connected to an edge does not exist on the referenced node.
    MissingPort {
        node_id: NodeId,
        port_name: String,
        context: String,
    },
    /// The data types of the source and target port are incompatible.
    PortTypeMismatch {
        edge_id: usize,
        source_type: DataType,
        target_type: DataType,
    },
    /// A required input port has no incoming connection.
    UnconnectedRequiredPort { node_id: NodeId, port_name: String },
    /// The graph contains a cycle that would prevent topological execution.
    CycleDetected {
        /// IDs of the nodes involved in the cycle, in traversal order.
        node_ids: Vec<NodeId>,
    },
    /// A backend referenced by the graph is not available.
    UnsupportedBackend { backend: String, node_id: NodeId },
    /// A node type is unknown or unsupported.
    UnknownNodeType { node_id: NodeId, node_type: String },
    /// Duplicate edge ID.
    DuplicateEdge { edge_id: usize },
    /// Catch-all for other validation failures.
    Other {
        node_id: Option<NodeId>,
        message: String,
    },
}

impl GraphValidationError {
    /// A short human-readable label for the error category.
    pub fn category(&self) -> &'static str {
        match self {
            Self::MissingNode { .. } => "missing_node",
            Self::MissingPort { .. } => "missing_port",
            Self::PortTypeMismatch { .. } => "port_type_mismatch",
            Self::UnconnectedRequiredPort { .. } => "unconnected_required_port",
            Self::CycleDetected { .. } => "cycle_detected",
            Self::UnsupportedBackend { .. } => "unsupported_backend",
            Self::UnknownNodeType { .. } => "unknown_node_type",
            Self::DuplicateEdge { .. } => "duplicate_edge",
            Self::Other { .. } => "other",
        }
    }
}

impl std::fmt::Display for GraphValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingNode { node_id, context } => {
                write!(f, "missing node {node_id} referenced by {context}")
            }
            Self::MissingPort {
                node_id,
                port_name,
                context,
            } => {
                write!(
                    f,
                    "missing port `{port_name}` on node {node_id} referenced by {context}"
                )
            }
            Self::PortTypeMismatch {
                edge_id,
                source_type,
                target_type,
            } => {
                write!(
                    f,
                    "edge {edge_id}: source type {source_type:?} does not match target type {target_type:?}"
                )
            }
            Self::UnconnectedRequiredPort { node_id, port_name } => {
                write!(
                    f,
                    "node {node_id}: required input port `{port_name}` is not connected"
                )
            }
            Self::CycleDetected { node_ids } => {
                write!(f, "cycle detected involving nodes {node_ids:?}")
            }
            Self::UnsupportedBackend { backend, node_id } => {
                write!(f, "node {node_id}: backend `{backend}` is not available")
            }
            Self::UnknownNodeType { node_id, node_type } => {
                write!(f, "node {node_id}: unknown node type `{node_type}`")
            }
            Self::DuplicateEdge { edge_id } => {
                write!(f, "duplicate edge ID {edge_id}")
            }
            Self::Other {
                node_id: Some(id),
                message,
            } => write!(f, "node {id}: {message}"),
            Self::Other {
                node_id: None,
                message,
            } => write!(f, "{message}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Validation result
// ---------------------------------------------------------------------------

/// The result of validating a diffusion graph.
///
/// Use `is_valid()` to check whether the graph is structurally sound enough
/// for execution. Warnings are non-blocking diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphValidationResult {
    pub errors: Vec<GraphValidationError>,
    pub warnings: Vec<String>,
}

impl GraphValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the graph is structurally valid (no blocking errors).
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: GraphValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

// ---------------------------------------------------------------------------
// Validator trait
// ---------------------------------------------------------------------------

/// Validates the structural and semantic correctness of a `DiffusionGraph`.
///
/// Implementations check node existence, port connectivity, type matching,
/// required-port completeness, cycle detection, backend availability, and
/// node-type support.
///
/// This trait matches `DiffusionGraphValidator` from the design and satisfies
/// Requirements 6.1 (WHEN a graph is edited, validate node types, ports,
/// dependencies, and cycles), 6.2 (agents use the same validation), and 6.3
/// (block execution with diagnostics when backends are unavailable).
pub trait DiffusionGraphValidator {
    /// Validate a graph and return the result.
    fn validate(&self, graph: &DiffusionGraph) -> GraphValidationResult;
}

// ---------------------------------------------------------------------------
// Default validator
// ---------------------------------------------------------------------------

/// A default graph validator that checks the structural invariants mandated
/// by the requirements.
///
/// Checks performed:
/// 1. Node existence for every edge endpoint.
/// 2. Port existence on the referenced nodes.
/// 3. Port data-type compatibility.
/// 4. Required input ports are connected.
/// 5. No duplicate edge IDs.
/// 6. No cycles in the directed graph (DFS-based).
pub struct DefaultGraphValidator;

impl DiffusionGraphValidator for DefaultGraphValidator {
    fn validate(&self, graph: &DiffusionGraph) -> GraphValidationResult {
        let mut result = GraphValidationResult::new();

        // --- Edge-level checks ---
        let mut seen_edge_ids = std::collections::HashSet::new();

        for edge in &graph.edges {
            // Duplicate edge ID.
            if !seen_edge_ids.insert(edge.id) {
                result
                    .errors
                    .push(GraphValidationError::DuplicateEdge { edge_id: edge.id });
                continue;
            }

            // Source node exists.
            let source_node = match graph.node_by_id(edge.source_node) {
                Some(n) => n,
                None => {
                    result.errors.push(GraphValidationError::MissingNode {
                        node_id: edge.source_node,
                        context: format!("edge {} source", edge.id),
                    });
                    continue;
                }
            };

            // Target node exists.
            let target_node = match graph.node_by_id(edge.target_node) {
                Some(n) => n,
                None => {
                    result.errors.push(GraphValidationError::MissingNode {
                        node_id: edge.target_node,
                        context: format!("edge {} target", edge.id),
                    });
                    continue;
                }
            };

            // Source port exists and is an output port.
            let source_port =
                match check_port_exists(source_node, &edge.source_port, PortDirection::Output) {
                    Ok(p) => p,
                    Err(e) => {
                        result.errors.push(e);
                        continue;
                    }
                };

            // Target port exists and is an input port.
            let target_port =
                match check_port_exists(target_node, &edge.target_port, PortDirection::Input) {
                    Ok(p) => p,
                    Err(e) => {
                        result.errors.push(e);
                        continue;
                    }
                };

            // Port type compatibility (Any matches anything).
            if source_port.data_type != DataType::Any
                && target_port.data_type != DataType::Any
                && source_port.data_type != target_port.data_type
            {
                result.errors.push(GraphValidationError::PortTypeMismatch {
                    edge_id: edge.id,
                    source_type: source_port.data_type,
                    target_type: target_port.data_type,
                });
            }
        }

        // --- Per-node checks ---
        for node in &graph.nodes {
            // Required input ports must have incoming edges.
            for port in node.input_ports() {
                if port.required
                    && !graph
                        .edges
                        .iter()
                        .any(|e| e.target_node == node.id && e.target_port == port.name)
                {
                    result
                        .errors
                        .push(GraphValidationError::UnconnectedRequiredPort {
                            node_id: node.id,
                            port_name: port.name.clone(),
                        });
                }
            }
        }

        // --- Cycle detection ---
        detect_cycles(graph, &mut result);

        result
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn check_port_exists<'a>(
    node: &'a GraphNode,
    port_name: &str,
    expected_direction: PortDirection,
) -> Result<&'a NodePort, GraphValidationError> {
    let port = node
        .port_by_name(port_name)
        .ok_or_else(|| GraphValidationError::MissingPort {
            node_id: node.id,
            port_name: port_name.to_string(),
            context: format!("{expected_direction:?} port"),
        })?;
    if port.direction != expected_direction {
        return Err(GraphValidationError::Other {
            node_id: Some(node.id),
            message: format!(
                "port `{port_name}` has direction {:?}, expected {expected_direction:?}",
                port.direction
            ),
        });
    }
    Ok(port)
}

/// Detect cycles using DFS-based topological check.
///
/// If a cycle is found, the first cycle's node IDs (in traversal order) are
/// emitted as a single `CycleDetected` error.
fn detect_cycles(graph: &DiffusionGraph, result: &mut GraphValidationResult) {
    // Build adjacency list: node ID -> list of target node IDs.
    let mut adjacency: std::collections::HashMap<NodeId, Vec<NodeId>> =
        std::collections::HashMap::new();
    for node in &graph.nodes {
        adjacency.entry(node.id).or_default();
    }
    for edge in &graph.edges {
        adjacency
            .entry(edge.source_node)
            .or_default()
            .push(edge.target_node);
    }

    // DFS state per node.
    const WHITE: u8 = 0; // unvisited
    const GRAY: u8 = 1; // in current DFS path
    const BLACK: u8 = 2; // finished
    let mut color: std::collections::HashMap<NodeId, u8> =
        graph.nodes.iter().map(|n| (n.id, WHITE)).collect();
    let mut parent: std::collections::HashMap<NodeId, Option<NodeId>> =
        graph.nodes.iter().map(|n| (n.id, None)).collect();

    fn visit(
        node: NodeId,
        adjacency: &std::collections::HashMap<NodeId, Vec<NodeId>>,
        color: &mut std::collections::HashMap<NodeId, u8>,
        parent: &mut std::collections::HashMap<NodeId, Option<NodeId>>,
        errors: &mut Vec<GraphValidationError>,
    ) {
        color.insert(node, GRAY);
        if let Some(neighbors) = adjacency.get(&node) {
            for &next in neighbors {
                let state = color.get(&next).copied().unwrap_or(WHITE);
                if state == WHITE {
                    parent.insert(next, Some(node));
                    visit(next, adjacency, color, parent, errors);
                } else if state == GRAY {
                    // Found a cycle — reconstruct the cycle path.
                    let mut cycle_nodes = vec![next, node];
                    let mut cur = node;
                    while let Some(ancestor) = parent.get(&cur).copied().flatten() {
                        if ancestor == next {
                            break;
                        }
                        cycle_nodes.push(ancestor);
                        cur = ancestor;
                    }
                    cycle_nodes.reverse();
                    errors.push(GraphValidationError::CycleDetected {
                        node_ids: cycle_nodes,
                    });
                }
                // BLACK (fully processed) — fallthrough, do nothing.
            }
        }
        color.insert(node, BLACK);
    }

    let all_ids: Vec<NodeId> = color.keys().copied().collect();
    for id in all_ids {
        if color.get(&id).copied().unwrap_or(WHITE) == WHITE {
            let mut cycle_errors: Vec<GraphValidationError> = Vec::new();
            visit(id, &adjacency, &mut color, &mut parent, &mut cycle_errors);
            for err in cycle_errors {
                if !result.errors.contains(&err) {
                    result.errors.push(err);
                }
            }
        }
    }
}
