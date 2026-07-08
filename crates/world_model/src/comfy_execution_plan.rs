use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    ComfyCachePolicy, DiffusionGraph, GraphValidationError, NodeCacheSnapshot, graph::NodeId,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPlanRequest {
    pub partial_targets: BTreeSet<NodeId>,
    pub cache_policy: ComfyCachePolicy,
    pub cache_snapshot: NodeCacheSnapshot,
}

impl ExecutionPlanRequest {
    pub fn new(partial_targets: impl IntoIterator<Item = NodeId>) -> Self {
        Self {
            partial_targets: partial_targets.into_iter().collect(),
            cache_policy: ComfyCachePolicy::Classic,
            cache_snapshot: NodeCacheSnapshot::default(),
        }
    }

    pub fn with_cache_policy(mut self, cache_policy: ComfyCachePolicy) -> Self {
        self.cache_policy = cache_policy;
        self
    }

    pub fn with_cache_snapshot(mut self, cache_snapshot: NodeCacheSnapshot) -> Self {
        self.cache_snapshot = cache_snapshot;
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub target_nodes: Vec<NodeId>,
    pub dependency_closure: Vec<NodeId>,
    pub execution_order: Vec<NodeId>,
    pub reusable_nodes: Vec<NodeId>,
    pub dirty_nodes: Vec<NodeId>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyExecutionPlanner;

impl ComfyExecutionPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        graph: &DiffusionGraph,
        request: ExecutionPlanRequest,
    ) -> Result<ExecutionPlan, Vec<GraphValidationError>> {
        let target_nodes = target_nodes(graph, &request.partial_targets);
        let mut errors = Vec::new();
        for target in &target_nodes {
            if graph.node_by_id(*target).is_none() {
                errors.push(GraphValidationError::MissingNode {
                    node_id: *target,
                    context: "execution plan target".to_string(),
                });
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        let dependencies_by_node = dependencies_by_node(graph);
        let dependency_closure = dependency_closure(&target_nodes, &dependencies_by_node);
        let reusable_nodes = request
            .cache_snapshot
            .reusable_nodes(graph, &request.cache_policy);
        let execution_order = topological_order(
            graph,
            &dependency_closure,
            &reusable_nodes,
            &dependencies_by_node,
        );
        let dirty_nodes = dependency_closure
            .iter()
            .copied()
            .filter(|node_id| !reusable_nodes.contains(node_id))
            .collect::<Vec<_>>();

        Ok(ExecutionPlan {
            target_nodes: target_nodes.into_iter().collect(),
            dependency_closure: dependency_closure.into_iter().collect(),
            execution_order,
            reusable_nodes: reusable_nodes.into_iter().collect(),
            dirty_nodes,
        })
    }
}

fn target_nodes(graph: &DiffusionGraph, partial_targets: &BTreeSet<NodeId>) -> BTreeSet<NodeId> {
    if partial_targets.is_empty() {
        graph.nodes.iter().map(|node| node.id).collect()
    } else {
        partial_targets.clone()
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

fn dependency_closure(
    target_nodes: &BTreeSet<NodeId>,
    dependencies_by_node: &BTreeMap<NodeId, BTreeSet<NodeId>>,
) -> BTreeSet<NodeId> {
    let mut closure = BTreeSet::new();
    for target in target_nodes {
        collect_dependencies(*target, dependencies_by_node, &mut closure);
    }
    closure
}

fn collect_dependencies(
    node_id: NodeId,
    dependencies_by_node: &BTreeMap<NodeId, BTreeSet<NodeId>>,
    closure: &mut BTreeSet<NodeId>,
) {
    if !closure.insert(node_id) {
        return;
    }
    if let Some(dependencies) = dependencies_by_node.get(&node_id) {
        for dependency in dependencies {
            collect_dependencies(*dependency, dependencies_by_node, closure);
        }
    }
}

fn topological_order(
    graph: &DiffusionGraph,
    dependency_closure: &BTreeSet<NodeId>,
    reusable_nodes: &BTreeSet<NodeId>,
    dependencies_by_node: &BTreeMap<NodeId, BTreeSet<NodeId>>,
) -> Vec<NodeId> {
    let mut ordered = Vec::new();
    let mut visited = BTreeSet::new();
    let node_ids = graph
        .nodes
        .iter()
        .map(|node| node.id)
        .filter(|node_id| dependency_closure.contains(node_id))
        .collect::<BTreeSet<_>>();

    for node_id in &node_ids {
        visit_node(
            *node_id,
            dependencies_by_node,
            dependency_closure,
            reusable_nodes,
            &mut visited,
            &mut ordered,
        );
    }
    ordered
}

fn visit_node(
    node_id: NodeId,
    dependencies_by_node: &BTreeMap<NodeId, BTreeSet<NodeId>>,
    dependency_closure: &BTreeSet<NodeId>,
    reusable_nodes: &BTreeSet<NodeId>,
    visited: &mut BTreeSet<NodeId>,
    ordered: &mut Vec<NodeId>,
) {
    if !visited.insert(node_id) {
        return;
    }
    if let Some(dependencies) = dependencies_by_node.get(&node_id) {
        for dependency in dependencies {
            if dependency_closure.contains(dependency) {
                visit_node(
                    *dependency,
                    dependencies_by_node,
                    dependency_closure,
                    reusable_nodes,
                    visited,
                    ordered,
                );
            }
        }
    }
    if !reusable_nodes.contains(&node_id) {
        ordered.push(node_id);
    }
}
