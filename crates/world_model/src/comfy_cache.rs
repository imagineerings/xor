use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{DiffusionGraph, GraphNode, graph::NodeId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyCachePolicy {
    RamPressure { active_gb: u64, inactive_gb: u64 },
    Classic,
    Lru { max_entries: usize },
    None,
}

impl ComfyCachePolicy {
    pub fn allows_reuse(&self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeCacheEntry {
    pub node_id: NodeId,
    pub cache_key: String,
    pub last_used_tick: u64,
    pub active_bytes: u64,
    pub inactive_bytes: u64,
}

impl NodeCacheEntry {
    pub fn new(node_id: NodeId, cache_key: impl Into<String>) -> Self {
        Self {
            node_id,
            cache_key: cache_key.into(),
            last_used_tick: 0,
            active_bytes: 0,
            inactive_bytes: 0,
        }
    }

    pub fn with_last_used_tick(mut self, last_used_tick: u64) -> Self {
        self.last_used_tick = last_used_tick;
        self
    }

    pub fn with_memory(mut self, active_bytes: u64, inactive_bytes: u64) -> Self {
        self.active_bytes = active_bytes;
        self.inactive_bytes = inactive_bytes;
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeCacheSnapshot {
    entries: BTreeMap<NodeId, NodeCacheEntry>,
}

impl NodeCacheSnapshot {
    pub fn new(entries: impl IntoIterator<Item = NodeCacheEntry>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|entry| (entry.node_id, entry))
                .collect(),
        }
    }

    pub fn reusable_nodes(
        &self,
        graph: &DiffusionGraph,
        policy: &ComfyCachePolicy,
    ) -> BTreeSet<NodeId> {
        if !policy.allows_reuse() {
            return BTreeSet::new();
        }

        let mut candidates: Vec<&NodeCacheEntry> = graph
            .nodes
            .iter()
            .filter_map(|node| self.reusable_entry(node))
            .collect();

        match policy {
            ComfyCachePolicy::Classic => candidates.iter().map(|entry| entry.node_id).collect(),
            ComfyCachePolicy::Lru { max_entries } => {
                candidates.sort_by_key(|entry| std::cmp::Reverse(entry.last_used_tick));
                candidates
                    .into_iter()
                    .take(*max_entries)
                    .map(|entry| entry.node_id)
                    .collect()
            }
            ComfyCachePolicy::RamPressure {
                active_gb,
                inactive_gb,
            } => {
                let active_limit = active_gb.saturating_mul(1024 * 1024 * 1024);
                let inactive_limit = inactive_gb.saturating_mul(1024 * 1024 * 1024);
                candidates.sort_by_key(|entry| std::cmp::Reverse(entry.last_used_tick));
                let mut active_total: u64 = 0;
                let mut inactive_total: u64 = 0;
                let mut reusable = BTreeSet::new();

                for entry in candidates {
                    let next_active = active_total.saturating_add(entry.active_bytes);
                    let next_inactive = inactive_total.saturating_add(entry.inactive_bytes);
                    if next_active <= active_limit && next_inactive <= inactive_limit {
                        active_total = next_active;
                        inactive_total = next_inactive;
                        reusable.insert(entry.node_id);
                    }
                }
                reusable
            }
            ComfyCachePolicy::None => BTreeSet::new(),
        }
    }

    fn reusable_entry(&self, node: &GraphNode) -> Option<&NodeCacheEntry> {
        let entry = self.entries.get(&node.id)?;
        (entry.cache_key == cache_key_for_node(node)).then_some(entry)
    }
}

pub fn cache_key_for_node(node: &GraphNode) -> String {
    let metadata = node
        .metadata
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("|");
    format!("{}:{}:{}", node.id, node.node_type, metadata)
}
