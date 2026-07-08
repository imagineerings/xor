use chrono::{DateTime, Utc};
use client::ChannelId;
use db::kvp::KeyValueStore;
use gpui::Entity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    pub body: String,
    pub updated_at: DateTime<Utc>,
}

pub struct DraftStore {
    kvp: Option<Entity<KeyValueStore>>,
    drafts: HashMap<ChannelId, Draft>,
    active_draft_channel: Option<ChannelId>,
}

impl DraftStore {
    pub fn new(kvp: Entity<KeyValueStore>) -> Self {
        Self {
            kvp: Some(kvp),
            drafts: HashMap::default(),
            active_draft_channel: None,
        }
    }

    pub fn memory_only() -> Self {
        Self {
            kvp: None,
            drafts: HashMap::default(),
            active_draft_channel: None,
        }
    }

    pub fn save_draft(&mut self, channel_id: ChannelId, body: &str) {
        self.active_draft_channel = Some(channel_id);

        if body.trim().is_empty() {
            self.drafts.remove(&channel_id);
            return;
        }

        self.drafts.insert(
            channel_id,
            Draft {
                body: body.to_string(),
                updated_at: Utc::now(),
            },
        );
    }

    pub fn load_draft(&mut self, channel_id: ChannelId) -> Option<String> {
        self.active_draft_channel = Some(channel_id);
        self.drafts
            .get(&channel_id)
            .map(|draft| draft.body.clone())
            .filter(|body| !body.trim().is_empty())
    }

    pub fn clear_draft(&mut self, channel_id: ChannelId) {
        self.drafts.remove(&channel_id);
        if self.active_draft_channel == Some(channel_id) {
            self.active_draft_channel = None;
        }
    }

    pub fn has_draft(&self, channel_id: ChannelId) -> bool {
        self.drafts
            .get(&channel_id)
            .is_some_and(|draft| !draft.body.trim().is_empty())
    }

    pub fn channels_with_drafts(&self) -> Vec<ChannelId> {
        let mut channel_ids = self
            .drafts
            .iter()
            .filter_map(|(channel_id, draft)| {
                if draft.body.trim().is_empty() {
                    None
                } else {
                    Some(*channel_id)
                }
            })
            .collect::<Vec<_>>();
        channel_ids.sort();
        channel_ids
    }

    pub fn active_draft_channel(&self) -> Option<ChannelId> {
        self.active_draft_channel
    }

    pub fn is_persistent(&self) -> bool {
        self.kvp.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_draft_from_memory() {
        let mut store = DraftStore::memory_only();
        let channel_id = ChannelId(7);

        store.save_draft(channel_id, "hello team");

        assert_eq!(store.load_draft(channel_id), Some("hello team".to_string()));
        assert!(store.has_draft(channel_id));
        assert_eq!(store.active_draft_channel(), Some(channel_id));
        assert!(!store.is_persistent());
    }

    #[test]
    fn clear_draft_removes_memory_entry() {
        let mut store = DraftStore::memory_only();
        let channel_id = ChannelId(7);

        store.save_draft(channel_id, "hello team");
        store.clear_draft(channel_id);

        assert_eq!(store.load_draft(channel_id), None);
        assert!(!store.has_draft(channel_id));
    }

    #[test]
    fn empty_draft_removes_existing_entry() {
        let mut store = DraftStore::memory_only();
        let channel_id = ChannelId(7);

        store.save_draft(channel_id, "hello team");
        store.save_draft(channel_id, "");

        assert_eq!(store.load_draft(channel_id), None);
        assert!(!store.has_draft(channel_id));
    }

    #[test]
    fn channels_with_drafts_are_sorted() {
        let mut store = DraftStore::memory_only();

        store.save_draft(ChannelId(3), "third");
        store.save_draft(ChannelId(1), "first");
        store.save_draft(ChannelId(2), "");

        assert_eq!(
            store.channels_with_drafts(),
            vec![ChannelId(1), ChannelId(3)]
        );
    }
}
