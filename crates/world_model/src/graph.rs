use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------------

/// Unique identifier for a graph node.
pub type NodeId = usize;

/// Unique identifier for a graph edge.
pub type EdgeId = usize;

// ---------------------------------------------------------------------------
// Port direction
// ---------------------------------------------------------------------------

/// Direction of a node port — data flows from an Output to an Input.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PortDirection {
    Input,
    Output,
}

// ---------------------------------------------------------------------------
// Data type
// ---------------------------------------------------------------------------

/// The type of data that flows through a port.
///
/// Covers the types used in common diffusion/world-model graph pipelines.
/// `Any` accepts any type and is used for polymorphic ports.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DataType {
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

impl DataType {
    /// Human-readable label for the data type.
    pub fn label(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Latent => "latent",
            Self::Conditioning => "conditioning",
            Self::Model => "model",
            Self::ControlNet => "controlnet",
            Self::Vae => "vae",
            Self::Clip => "clip",
            Self::Float => "float",
            Self::Int => "int",
            Self::String => "string",
            Self::Bool => "bool",
            Self::Any => "any",
        }
    }
}

// ---------------------------------------------------------------------------
// Node port
// ---------------------------------------------------------------------------

/// A single typed port on a graph node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodePort {
    pub name: String,
    pub direction: PortDirection,
    pub data_type: DataType,
    pub required: bool,
}

impl NodePort {
    pub fn new(
        name: impl Into<String>,
        direction: PortDirection,
        data_type: DataType,
        required: bool,
    ) -> Self {
        Self {
            name: name.into(),
            direction,
            data_type,
            required,
        }
    }
}

// ---------------------------------------------------------------------------
// Graph node
// ---------------------------------------------------------------------------

/// A node in the diffusion graph.
///
/// Each node has an identity (`id`), a runtime `node_type` string (e.g.,
/// `"KSampler"`, `"VAEDecode"`) that a backend maps to a concrete op, and a
/// set of typed `ports`. The `metadata` map holds implementation-specific
/// parameters (e.g., seed, steps, CFG scale).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: NodeId,
    pub node_type: String,
    pub label: Option<String>,
    pub ports: Vec<NodePort>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl GraphNode {
    pub fn new(id: NodeId, node_type: impl Into<String>) -> Self {
        Self {
            id,
            node_type: node_type.into(),
            label: None,
            ports: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_port(mut self, port: NodePort) -> Self {
        self.ports.push(port);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Return an iterator over input ports.
    pub fn input_ports(&self) -> impl Iterator<Item = &NodePort> {
        self.ports
            .iter()
            .filter(|p| p.direction == PortDirection::Input)
    }

    /// Return an iterator over output ports.
    pub fn output_ports(&self) -> impl Iterator<Item = &NodePort> {
        self.ports
            .iter()
            .filter(|p| p.direction == PortDirection::Output)
    }

    /// Look up a port by name.
    pub fn port_by_name(&self, name: &str) -> Option<&NodePort> {
        self.ports.iter().find(|p| p.name == name)
    }
}

// ---------------------------------------------------------------------------
// Graph edge
// ---------------------------------------------------------------------------

/// A directed edge connecting an output port of one node to an input port of
/// another.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: EdgeId,
    pub source_node: NodeId,
    pub source_port: String,
    pub target_node: NodeId,
    pub target_port: String,
}

impl GraphEdge {
    pub fn new(
        id: EdgeId,
        source_node: NodeId,
        source_port: impl Into<String>,
        target_node: NodeId,
        target_port: impl Into<String>,
    ) -> Self {
        Self {
            id,
            source_node,
            source_port: source_port.into(),
            target_node,
            target_port: target_port.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Diffusion graph
// ---------------------------------------------------------------------------

/// A typed diffusion pipeline graph with nodes and directed edges.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DiffusionGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl DiffusionGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_node(mut self, node: GraphNode) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn with_edge(mut self, edge: GraphEdge) -> Self {
        self.edges.push(edge);
        self
    }

    /// Look up a node by its ID.
    pub fn node_by_id(&self, id: NodeId) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Look up a mutable reference to a node by its ID.
    pub fn node_by_id_mut(&mut self, id: NodeId) -> Option<&mut GraphNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Look up an edge by its ID.
    pub fn edge_by_id(&self, id: EdgeId) -> Option<&GraphEdge> {
        self.edges.iter().find(|e| e.id == id)
    }

    /// Return all edges that have the given node as their source.
    pub fn outgoing_edges(&self, node_id: NodeId) -> Vec<&GraphEdge> {
        self.edges
            .iter()
            .filter(|e| e.source_node == node_id)
            .collect()
    }

    /// Return all edges that have the given node as their target.
    pub fn incoming_edges(&self, node_id: NodeId) -> Vec<&GraphEdge> {
        self.edges
            .iter()
            .filter(|e| e.target_node == node_id)
            .collect()
    }

    /// Return the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Return the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}
