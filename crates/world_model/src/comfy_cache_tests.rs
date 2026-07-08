use crate::{
    ComfyCachePolicy, DataType, DiffusionGraph, GraphNode, NodeCacheEntry, NodeCacheSnapshot,
    NodePort, PortDirection, cache_key_for_node,
};

#[test]
fn cache_key_is_stable_for_metadata_order() {
    let first = GraphNode::new(1, "KSampler")
        .with_metadata("steps", "20")
        .with_metadata("seed", "42");
    let second = GraphNode::new(1, "KSampler")
        .with_metadata("seed", "42")
        .with_metadata("steps", "20");

    assert_eq!(cache_key_for_node(&first), cache_key_for_node(&second));
}

#[test]
fn classic_policy_reuses_matching_cache_entries() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(1, "LoadImage").with_metadata("image", "a.png"))
        .with_node(GraphNode::new(2, "VAEDecode").with_port(NodePort::new(
            "samples",
            PortDirection::Input,
            DataType::Latent,
            true,
        )));
    let snapshot = NodeCacheSnapshot::new([
        NodeCacheEntry::new(1, cache_key_for_node(graph.node_by_id(1).expect("node"))),
        NodeCacheEntry::new(2, "stale"),
    ]);

    let reusable = snapshot.reusable_nodes(&graph, &ComfyCachePolicy::Classic);

    assert!(reusable.contains(&1));
    assert!(!reusable.contains(&2));
}

#[test]
fn none_policy_disables_reuse() {
    let graph = DiffusionGraph::new().with_node(GraphNode::new(1, "LoadImage"));
    let snapshot = NodeCacheSnapshot::new([NodeCacheEntry::new(
        1,
        cache_key_for_node(graph.node_by_id(1).expect("node")),
    )]);

    assert!(
        snapshot
            .reusable_nodes(&graph, &ComfyCachePolicy::None)
            .is_empty()
    );
}

#[test]
fn lru_policy_keeps_most_recent_matching_entries() {
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(1, "A"))
        .with_node(GraphNode::new(2, "B"))
        .with_node(GraphNode::new(3, "C"));
    let snapshot = NodeCacheSnapshot::new([
        NodeCacheEntry::new(1, cache_key_for_node(graph.node_by_id(1).expect("node")))
            .with_last_used_tick(1),
        NodeCacheEntry::new(2, cache_key_for_node(graph.node_by_id(2).expect("node")))
            .with_last_used_tick(10),
        NodeCacheEntry::new(3, cache_key_for_node(graph.node_by_id(3).expect("node")))
            .with_last_used_tick(5),
    ]);

    let reusable = snapshot.reusable_nodes(&graph, &ComfyCachePolicy::Lru { max_entries: 2 });

    assert_eq!(reusable.into_iter().collect::<Vec<_>>(), vec![2, 3]);
}

#[test]
fn ram_pressure_policy_respects_active_and_inactive_limits() {
    let gib = 1024 * 1024 * 1024;
    let graph = DiffusionGraph::new()
        .with_node(GraphNode::new(1, "A"))
        .with_node(GraphNode::new(2, "B"))
        .with_node(GraphNode::new(3, "C"));
    let snapshot = NodeCacheSnapshot::new([
        NodeCacheEntry::new(1, cache_key_for_node(graph.node_by_id(1).expect("node")))
            .with_last_used_tick(30)
            .with_memory(gib, 0),
        NodeCacheEntry::new(2, cache_key_for_node(graph.node_by_id(2).expect("node")))
            .with_last_used_tick(20)
            .with_memory(gib, 0),
        NodeCacheEntry::new(3, cache_key_for_node(graph.node_by_id(3).expect("node")))
            .with_last_used_tick(10)
            .with_memory(0, gib),
    ]);

    let reusable = snapshot.reusable_nodes(
        &graph,
        &ComfyCachePolicy::RamPressure {
            active_gb: 1,
            inactive_gb: 1,
        },
    );

    assert_eq!(reusable.into_iter().collect::<Vec<_>>(), vec![1, 3]);
}
