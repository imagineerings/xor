use std::collections::BTreeMap;

use crate::{
    ComfyExecutorDiagnostic, ComfyExecutorDispatch, ComfyNodeExecutionOutcome,
    ComfyNodeExecutionState, ComfyNodeExecutor, ComfyNodeRuntime, DataType, DiffusionGraph,
    ExecutionPlan, GraphEdge, GraphNode, NodePort, PortDirection, graph::NodeId,
};

#[test]
fn executor_records_cached_nodes_and_runs_dirty_nodes_in_plan_order() {
    let graph = executor_graph();
    let mut runtime = MockRuntime::new([
        (
            2,
            ComfyNodeExecutionOutcome::completed().with_output("CONDITIONING", "positive"),
        ),
        (
            3,
            ComfyNodeExecutionOutcome::completed().with_output("LATENT", "latent"),
        ),
    ]);
    let plan = ExecutionPlan {
        reusable_nodes: vec![1],
        execution_order: vec![2, 3],
        dependency_closure: vec![1, 2, 3],
        dirty_nodes: vec![2, 3],
        target_nodes: vec![3],
    };

    let report = ComfyNodeExecutor::new().execute(&graph, &plan, &mut runtime);

    assert_eq!(runtime.executed, vec![2, 3]);
    assert_eq!(report.records[0].state, ComfyNodeExecutionState::Cached);
    assert_eq!(
        report.records[1]
            .outputs
            .get("CONDITIONING")
            .map(String::as_str),
        Some("positive")
    );
    assert_eq!(
        report.records[2].dispatch,
        Some(ComfyExecutorDispatch::DiffusionWorldModel {
            node_type: "KSampler".to_string()
        })
    );
}

#[test]
fn executor_preserves_async_list_ui_outputs_and_provenance() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(1, "LoadImage"))
        .with_node(GraphNode::new(2, "SaveImage"));
    let mut runtime = MockRuntime::new([
        (
            1,
            ComfyNodeExecutionOutcome {
                state: ComfyNodeExecutionState::AsyncPending,
                provenance: vec!["load=pending".to_string()],
                ..ComfyNodeExecutionOutcome::default()
            },
        ),
        (
            2,
            ComfyNodeExecutionOutcome {
                state: ComfyNodeExecutionState::ListMapped,
                ui_outputs: vec![crate::ComfyUiOutput {
                    node_id: 2,
                    name: "images".to_string(),
                    value: "artifact://image".to_string(),
                }],
                provenance: vec!["images=1".to_string()],
                ..ComfyNodeExecutionOutcome::default()
            },
        ),
    ]);
    let plan = ExecutionPlan {
        execution_order: vec![1, 2],
        dependency_closure: vec![1, 2],
        dirty_nodes: vec![1, 2],
        target_nodes: vec![2],
        reusable_nodes: Vec::new(),
    };

    let report = ComfyNodeExecutor::new().execute(&graph, &plan, &mut runtime);

    assert_eq!(
        report.records[0].state,
        ComfyNodeExecutionState::AsyncPending
    );
    assert_eq!(report.records[1].state, ComfyNodeExecutionState::ListMapped);
    assert_eq!(report.ui_outputs[0].value, "artifact://image");
    assert!(report.provenance.iter().any(|item| item == "images=1"));
}

#[test]
fn executor_skips_dependents_after_blocked_failed_or_interrupted_node() {
    for outcome in [
        ComfyNodeExecutionOutcome::blocked(),
        ComfyNodeExecutionOutcome::failed(),
        ComfyNodeExecutionOutcome::interrupted(),
    ] {
        let graph = executor_graph();
        let mut runtime = MockRuntime::new([(2, outcome.clone())]);
        let plan = ExecutionPlan {
            reusable_nodes: vec![1],
            execution_order: vec![2, 3],
            dependency_closure: vec![1, 2, 3],
            dirty_nodes: vec![2, 3],
            target_nodes: vec![3],
        };

        let report = ComfyNodeExecutor::new().execute(&graph, &plan, &mut runtime);

        assert_eq!(runtime.executed, vec![2]);
        assert_eq!(report.records[2].state, ComfyNodeExecutionState::Skipped);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::comfy_executor::EXECUTOR_BLOCKED_DEPENDENCY_CODE
        }));
    }
}

#[test]
fn executor_maps_runtime_errors_to_failed_records() {
    let graph = DiffusionGraph::new().with_node(GraphNode::new(1, "LoadImage"));
    let mut runtime = MockRuntime {
        outcomes: BTreeMap::new(),
        errors: BTreeMap::from([(
            1,
            ComfyExecutorDiagnostic {
                code: "runtime.failed".to_string(),
                node_id: Some(1),
                message: "boom".to_string(),
            },
        )]),
        executed: Vec::new(),
    };
    let plan = ExecutionPlan {
        execution_order: vec![1],
        dependency_closure: vec![1],
        dirty_nodes: vec![1],
        target_nodes: vec![1],
        reusable_nodes: Vec::new(),
    };

    let report = ComfyNodeExecutor::new().execute(&graph, &plan, &mut runtime);

    assert_eq!(report.records[0].state, ComfyNodeExecutionState::Failed);
    assert_eq!(report.diagnostics[0].code, "runtime.failed");
}

struct MockRuntime {
    outcomes: BTreeMap<NodeId, ComfyNodeExecutionOutcome>,
    errors: BTreeMap<NodeId, ComfyExecutorDiagnostic>,
    executed: Vec<NodeId>,
}

impl MockRuntime {
    fn new(outcomes: impl IntoIterator<Item = (NodeId, ComfyNodeExecutionOutcome)>) -> Self {
        Self {
            outcomes: outcomes.into_iter().collect(),
            errors: BTreeMap::new(),
            executed: Vec::new(),
        }
    }
}

impl ComfyNodeRuntime for MockRuntime {
    fn execute_node(
        &mut self,
        node: &GraphNode,
    ) -> Result<ComfyNodeExecutionOutcome, ComfyExecutorDiagnostic> {
        self.executed.push(node.id);
        if let Some(error) = self.errors.get(&node.id) {
            return Err(error.clone());
        }
        Ok(self
            .outcomes
            .remove(&node.id)
            .unwrap_or_else(ComfyNodeExecutionOutcome::completed))
    }
}

trait OutcomeBuilder {
    fn with_output(self, name: &str, value: &str) -> Self;
}

impl OutcomeBuilder for ComfyNodeExecutionOutcome {
    fn with_output(mut self, name: &str, value: &str) -> Self {
        self.outputs.insert(name.to_string(), value.to_string());
        self
    }
}

fn executor_graph() -> DiffusionGraph {
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
                .with_port(NodePort::new(
                    "clip",
                    PortDirection::Input,
                    DataType::Clip,
                    true,
                )),
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
                )),
        )
        .with_edge(GraphEdge::new(1, 1, "MODEL", 3, "model"))
        .with_edge(GraphEdge::new(2, 2, "CONDITIONING", 3, "positive"))
}
