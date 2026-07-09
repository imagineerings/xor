use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{DiffusionGraph, ExecutionPlan, GraphNode, graph::NodeId};

pub const EXECUTOR_BLOCKED_DEPENDENCY_CODE: &str = "world_model.comfy_executor.blocked_dependency";
pub const EXECUTOR_MISSING_NODE_CODE: &str = "world_model.comfy_executor.missing_node";

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub enum ComfyNodeExecutionState {
    Cached,
    #[default]
    Completed,
    AsyncPending,
    ListMapped,
    Blocked,
    Interrupted,
    Failed,
    Skipped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyExecutorDispatch {
    DiffusionWorldModel { node_type: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyUiOutput {
    pub node_id: NodeId,
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyNodeExecutionOutcome {
    pub state: ComfyNodeExecutionState,
    pub outputs: BTreeMap<String, String>,
    pub ui_outputs: Vec<ComfyUiOutput>,
    pub provenance: Vec<String>,
    pub dispatch: Option<ComfyExecutorDispatch>,
}

impl ComfyNodeExecutionOutcome {
    pub fn completed() -> Self {
        Self {
            state: ComfyNodeExecutionState::Completed,
            ..Self::default()
        }
    }

    pub fn blocked() -> Self {
        Self {
            state: ComfyNodeExecutionState::Blocked,
            ..Self::default()
        }
    }

    pub fn failed() -> Self {
        Self {
            state: ComfyNodeExecutionState::Failed,
            ..Self::default()
        }
    }

    pub fn interrupted() -> Self {
        Self {
            state: ComfyNodeExecutionState::Interrupted,
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyNodeExecutionRecord {
    pub node_id: NodeId,
    pub node_type: String,
    pub state: ComfyNodeExecutionState,
    pub outputs: BTreeMap<String, String>,
    pub ui_outputs: Vec<ComfyUiOutput>,
    pub provenance: Vec<String>,
    pub dispatch: Option<ComfyExecutorDispatch>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyExecutorDiagnostic {
    pub code: String,
    pub node_id: Option<NodeId>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyExecutionReport {
    pub records: Vec<ComfyNodeExecutionRecord>,
    pub ui_outputs: Vec<ComfyUiOutput>,
    pub provenance: Vec<String>,
    pub diagnostics: Vec<ComfyExecutorDiagnostic>,
}

pub trait ComfyNodeRuntime {
    fn execute_node(
        &mut self,
        node: &GraphNode,
    ) -> Result<ComfyNodeExecutionOutcome, ComfyExecutorDiagnostic>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyNodeExecutor;

impl ComfyNodeExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        graph: &DiffusionGraph,
        plan: &ExecutionPlan,
        runtime: &mut impl ComfyNodeRuntime,
    ) -> ComfyExecutionReport {
        let dependencies = dependencies_by_node(graph);
        let mut terminal_nodes = BTreeSet::new();
        let mut report = ComfyExecutionReport::default();

        for node_id in &plan.reusable_nodes {
            if let Some(node) = graph.node_by_id(*node_id) {
                push_record(
                    &mut report,
                    record_for_outcome(
                        node,
                        ComfyNodeExecutionOutcome {
                            state: ComfyNodeExecutionState::Cached,
                            ..ComfyNodeExecutionOutcome::default()
                        },
                    ),
                );
            }
        }

        for node_id in &plan.execution_order {
            let Some(node) = graph.node_by_id(*node_id) else {
                report.diagnostics.push(diagnostic(
                    EXECUTOR_MISSING_NODE_CODE,
                    Some(*node_id),
                    "execution plan references a missing graph node",
                ));
                continue;
            };

            if has_blocked_dependency(*node_id, &dependencies, &terminal_nodes) {
                terminal_nodes.insert(*node_id);
                let outcome = ComfyNodeExecutionOutcome {
                    state: ComfyNodeExecutionState::Skipped,
                    ..ComfyNodeExecutionOutcome::default()
                };
                push_record(&mut report, record_for_outcome(node, outcome));
                report.diagnostics.push(diagnostic(
                    EXECUTOR_BLOCKED_DEPENDENCY_CODE,
                    Some(*node_id),
                    "node skipped because an upstream dependency did not complete",
                ));
                continue;
            }

            let outcome = runtime
                .execute_node(node)
                .unwrap_or_else(|diagnostic| {
                    report.diagnostics.push(diagnostic);
                    ComfyNodeExecutionOutcome::failed()
                })
                .with_dispatch_for_node(node);

            if is_terminal_failure(outcome.state) {
                terminal_nodes.insert(*node_id);
            }
            push_record(&mut report, record_for_outcome(node, outcome));
        }

        report
    }
}

trait DispatchOutcome {
    fn with_dispatch_for_node(self, node: &GraphNode) -> Self;
}

impl DispatchOutcome for ComfyNodeExecutionOutcome {
    fn with_dispatch_for_node(mut self, node: &GraphNode) -> Self {
        if self.dispatch.is_none() && requires_diffusion_world_model_dispatch(&node.node_type) {
            self.dispatch = Some(ComfyExecutorDispatch::DiffusionWorldModel {
                node_type: node.node_type.clone(),
            });
        }
        self
    }
}

fn push_record(report: &mut ComfyExecutionReport, record: ComfyNodeExecutionRecord) {
    report.ui_outputs.extend(record.ui_outputs.clone());
    report.provenance.extend(record.provenance.clone());
    report.records.push(record);
}

fn record_for_outcome(
    node: &GraphNode,
    outcome: ComfyNodeExecutionOutcome,
) -> ComfyNodeExecutionRecord {
    ComfyNodeExecutionRecord {
        node_id: node.id,
        node_type: node.node_type.clone(),
        state: outcome.state,
        outputs: outcome.outputs,
        ui_outputs: outcome.ui_outputs,
        provenance: outcome.provenance,
        dispatch: outcome.dispatch,
    }
}

fn dependencies_by_node(graph: &DiffusionGraph) -> BTreeMap<NodeId, BTreeSet<NodeId>> {
    let mut dependencies = BTreeMap::new();
    for node in &graph.nodes {
        dependencies.entry(node.id).or_insert_with(BTreeSet::new);
    }
    for edge in &graph.edges {
        dependencies
            .entry(edge.target_node)
            .or_insert_with(BTreeSet::new)
            .insert(edge.source_node);
    }
    dependencies
}

fn has_blocked_dependency(
    node_id: NodeId,
    dependencies: &BTreeMap<NodeId, BTreeSet<NodeId>>,
    terminal_nodes: &BTreeSet<NodeId>,
) -> bool {
    dependencies.get(&node_id).is_some_and(|dependencies| {
        dependencies
            .iter()
            .any(|node_id| terminal_nodes.contains(node_id))
    })
}

fn is_terminal_failure(state: ComfyNodeExecutionState) -> bool {
    matches!(
        state,
        ComfyNodeExecutionState::Blocked
            | ComfyNodeExecutionState::Interrupted
            | ComfyNodeExecutionState::Failed
            | ComfyNodeExecutionState::Skipped
    )
}

fn requires_diffusion_world_model_dispatch(node_type: &str) -> bool {
    matches!(
        node_type,
        "CLIPLoader"
            | "CLIPSetLastLayer"
            | "CLIPTextEncode"
            | "CLIPVisionEncode"
            | "CLIPVisionLoader"
            | "ControlNetApply"
            | "ControlNetApplyAdvanced"
            | "ControlNetLoader"
            | "DiffControlNetLoader"
            | "DiffusersLoader"
            | "DualCLIPLoader"
            | "GLIGENLoader"
            | "GLIGENTextBoxApply"
            | "InpaintModelConditioning"
            | "KSampler"
            | "LoraLoader"
            | "StyleModelApply"
            | "StyleModelLoader"
            | "UNETLoader"
            | "VAEDecode"
            | "VAEDecodeTiled"
            | "VAEEncode"
            | "VAELoader"
    )
}

fn diagnostic(
    code: &str,
    node_id: Option<NodeId>,
    message: impl Into<String>,
) -> ComfyExecutorDiagnostic {
    ComfyExecutorDiagnostic {
        code: code.to_string(),
        node_id,
        message: message.into(),
    }
}
