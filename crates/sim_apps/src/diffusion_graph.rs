use serde::{Deserialize, Serialize};
use world_model::{
    ComfyExecutionPlanner, DefaultGraphValidator, DiffusionGraph, DiffusionGraphValidator,
    ExecutionPlan, ExecutionPlanRequest, GraphValidationError, GraphValidationResult,
    graph::NodeId,
};

pub const DIFFUSION_GRAPH_EXECUTION_BLOCKED_CODE: &str =
    "sim_apps.diffusion_graph.execution_blocked";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiffusionGraphArtifactOutput {
    pub node_id: NodeId,
    pub port_name: String,
    pub artifact_id: String,
}

impl DiffusionGraphArtifactOutput {
    pub fn new(
        node_id: NodeId,
        port_name: impl Into<String>,
        artifact_id: impl Into<String>,
    ) -> Self {
        Self {
            node_id,
            port_name: port_name.into(),
            artifact_id: artifact_id.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiffusionGraphEditorState {
    pub graph: DiffusionGraph,
    pub validation: GraphValidationResult,
    pub artifact_outputs: Vec<DiffusionGraphArtifactOutput>,
    pub execution_plan: Option<ExecutionPlan>,
}

impl DiffusionGraphEditorState {
    pub fn new(graph: DiffusionGraph) -> Self {
        let validation = DefaultGraphValidator.validate(&graph);
        Self {
            graph,
            validation,
            artifact_outputs: Vec::new(),
            execution_plan: None,
        }
    }

    pub fn set_graph(&mut self, graph: DiffusionGraph) {
        self.graph = graph;
        self.validation = DefaultGraphValidator.validate(&self.graph);
        self.execution_plan = None;
    }

    pub fn register_artifact_output(&mut self, output: DiffusionGraphArtifactOutput) {
        self.artifact_outputs.push(output);
    }

    pub fn plan_execution(
        &mut self,
        request: ExecutionPlanRequest,
    ) -> Result<&ExecutionPlan, DiffusionGraphEditorDiagnostic> {
        self.validation = DefaultGraphValidator.validate(&self.graph);
        if !self.validation.is_valid() {
            return Err(DiffusionGraphEditorDiagnostic {
                code: DIFFUSION_GRAPH_EXECUTION_BLOCKED_CODE.to_string(),
                validation_errors: self.validation.errors.clone(),
                message: "diffusion graph execution is blocked until validation succeeds"
                    .to_string(),
            });
        }

        let plan = ComfyExecutionPlanner::new()
            .plan(&self.graph, request)
            .map_err(|errors| DiffusionGraphEditorDiagnostic {
                code: DIFFUSION_GRAPH_EXECUTION_BLOCKED_CODE.to_string(),
                validation_errors: errors,
                message: "diffusion graph execution plan could not be created".to_string(),
            })?;
        self.execution_plan = Some(plan);
        self.execution_plan
            .as_ref()
            .ok_or_else(|| DiffusionGraphEditorDiagnostic {
                code: DIFFUSION_GRAPH_EXECUTION_BLOCKED_CODE.to_string(),
                validation_errors: Vec::new(),
                message: "diffusion graph execution plan was not retained".to_string(),
            })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiffusionGraphEditorDiagnostic {
    pub code: String,
    pub validation_errors: Vec<GraphValidationError>,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_model::{DataType, GraphEdge, GraphNode, NodePort, PortDirection};

    #[test]
    fn editor_state_exposes_validation_and_artifact_outputs() {
        let mut state = DiffusionGraphEditorState::new(valid_graph());
        state.register_artifact_output(DiffusionGraphArtifactOutput::new(
            2,
            "IMAGE",
            "asset-image-1",
        ));

        assert!(state.validation.is_valid());
        assert_eq!(state.artifact_outputs.len(), 1);
        assert_eq!(state.artifact_outputs[0].artifact_id, "asset-image-1");
    }

    #[test]
    fn editor_state_blocks_execution_for_invalid_graphs() {
        let mut state = DiffusionGraphEditorState::new(
            DiffusionGraph::new().with_edge(GraphEdge::new(1, 1, "out", 2, "in")),
        );

        let diagnostic = state
            .plan_execution(ExecutionPlanRequest::new([]))
            .expect_err("invalid graph should block execution");

        assert_eq!(diagnostic.code, DIFFUSION_GRAPH_EXECUTION_BLOCKED_CODE);
        assert!(!diagnostic.validation_errors.is_empty());
        assert!(state.execution_plan.is_none());
    }

    #[test]
    fn editor_state_plans_valid_graph_execution() {
        let mut state = DiffusionGraphEditorState::new(valid_graph());

        let plan = state
            .plan_execution(ExecutionPlanRequest::new([2]))
            .expect("valid graph should plan");

        assert_eq!(plan.target_nodes, vec![2]);
        assert_eq!(plan.execution_order, vec![1, 2]);
        assert!(state.execution_plan.is_some());
    }

    fn valid_graph() -> DiffusionGraph {
        DiffusionGraph::new()
            .with_node(GraphNode::new(1, "LoadImage").with_port(NodePort::new(
                "IMAGE",
                PortDirection::Output,
                DataType::Image,
                false,
            )))
            .with_node(GraphNode::new(2, "SaveImage").with_port(NodePort::new(
                "images",
                PortDirection::Input,
                DataType::Image,
                true,
            )))
            .with_edge(GraphEdge::new(1, 1, "IMAGE", 2, "images"))
    }
}
