use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use client::ChannelId;
use db::kvp::KeyValueStore;
use gpui::{App, AppContext as _, Entity, Global};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use util::ResultExt as _;

const DRAFT_NAMESPACE: &str = "channel_drafts";
const DRAFT_KEY_PREFIX: &str = "channel_draft.";

pub fn init(cx: &mut App) {
    DraftStore::init(cx);
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    pub body: String,
    pub updated_at: DateTime<Utc>,
}

pub struct DraftStore {
    kvp: Option<KeyValueStore>,
    drafts: HashMap<ChannelId, Draft>,
    active_draft_channel: Option<ChannelId>,
}

impl DraftStore {
    pub fn new(kvp: KeyValueStore) -> Self {
        Self::new_with_drafts(kvp, HashMap::default())
    }

    fn new_with_drafts(kvp: KeyValueStore, drafts: HashMap<ChannelId, Draft>) -> Self {
        Self {
            kvp: Some(kvp),
            drafts,
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

    pub fn persist_key(channel_id: ChannelId) -> String {
        format!("{}{}", DRAFT_KEY_PREFIX, channel_id.0)
    }

    pub fn init(cx: &mut App) {
        let kvp = KeyValueStore::global(cx);
        let drafts = Self::read_all_from_kvp(&kvp).log_err().unwrap_or_default();
        let store = cx.new(|_| Self::new_with_drafts(kvp, drafts));
        cx.set_global(GlobalDraftStore(store));
    }

    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalDraftStore>().0.clone()
    }

    pub async fn save_draft(&mut self, channel_id: ChannelId, body: &str) -> Result<()> {
        self.active_draft_channel = Some(channel_id);

        if body.trim().is_empty() {
            self.drafts.remove(&channel_id);
            self.delete_from_kvp(channel_id).await?;
            return Ok(());
        }

        let draft = Draft {
            body: body.to_string(),
            updated_at: Utc::now(),
        };
        self.write_to_kvp(channel_id, &draft).await?;
        self.drafts.insert(channel_id, draft);
        Ok(())
    }

    pub async fn load_draft(&mut self, channel_id: ChannelId) -> Result<Option<String>> {
        self.active_draft_channel = Some(channel_id);
        if !self.drafts.contains_key(&channel_id) {
            if let Some(draft) = self.read_from_kvp(channel_id).await? {
                self.drafts.insert(channel_id, draft);
            }
        }

        Ok(self
            .drafts
            .get(&channel_id)
            .map(|draft| draft.body.clone())
            .filter(|body| !body.trim().is_empty()))
    }

    pub async fn clear_draft(&mut self, channel_id: ChannelId) -> Result<()> {
        self.drafts.remove(&channel_id);
        self.delete_from_kvp(channel_id).await?;
        if self.active_draft_channel == Some(channel_id) {
            self.active_draft_channel = None;
        }
        Ok(())
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

    async fn write_to_kvp(&self, channel_id: ChannelId, draft: &Draft) -> Result<()> {
        let Some(kvp) = self.kvp.as_ref() else {
            return Ok(());
        };
        let payload = serde_json::to_string(draft).context("serializing channel draft")?;
        kvp.scoped(DRAFT_NAMESPACE)
            .write(Self::persist_key(channel_id), payload)
            .await
            .context("writing channel draft")
    }

    async fn read_from_kvp(&self, channel_id: ChannelId) -> Result<Option<Draft>> {
        let Some(kvp) = self.kvp.as_ref() else {
            return Ok(None);
        };
        let Some(payload) = kvp
            .scoped(DRAFT_NAMESPACE)
            .read(&Self::persist_key(channel_id))
            .context("reading channel draft")?
        else {
            return Ok(None);
        };
        serde_json::from_str(&payload)
            .map(Some)
            .context("deserializing channel draft")
    }

    async fn delete_from_kvp(&self, channel_id: ChannelId) -> Result<()> {
        let Some(kvp) = self.kvp.as_ref() else {
            return Ok(());
        };
        kvp.scoped(DRAFT_NAMESPACE)
            .delete(Self::persist_key(channel_id))
            .await
            .context("deleting channel draft")
    }

    fn read_all_from_kvp(kvp: &KeyValueStore) -> Result<HashMap<ChannelId, Draft>> {
        let mut drafts = HashMap::default();
        for (key, payload) in kvp
            .scoped(DRAFT_NAMESPACE)
            .read_all()
            .context("reading channel drafts")?
        {
            let Some(channel_id) = Self::channel_id_from_persist_key(&key) else {
                continue;
            };
            let draft = serde_json::from_str(&payload).context("deserializing channel draft")?;
            drafts.insert(channel_id, draft);
        }
        Ok(drafts)
    }

    fn channel_id_from_persist_key(key: &str) -> Option<ChannelId> {
        key.strip_prefix(DRAFT_KEY_PREFIX)
            .and_then(|channel_id| channel_id.parse::<u64>().ok())
            .map(ChannelId)
    }
}

struct GlobalDraftStore(Entity<DraftStore>);

impl Global for GlobalDraftStore {}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    async fn save_and_load_draft_from_memory() {
        let mut store = DraftStore::memory_only();
        let channel_id = ChannelId(7);

        store
            .save_draft(channel_id, "hello team")
            .await
            .expect("save draft");

        assert_eq!(
            store.load_draft(channel_id).await.expect("load draft"),
            Some("hello team".to_string())
        );
        assert!(store.has_draft(channel_id));
        assert_eq!(store.active_draft_channel(), Some(channel_id));
        assert!(!store.is_persistent());
    }

    #[gpui::test]
    async fn clear_draft_removes_memory_entry() {
        let mut store = DraftStore::memory_only();
        let channel_id = ChannelId(7);

        store
            .save_draft(channel_id, "hello team")
            .await
            .expect("save draft");
        store.clear_draft(channel_id).await.expect("clear draft");

        assert_eq!(
            store.load_draft(channel_id).await.expect("load draft"),
            None
        );
        assert!(!store.has_draft(channel_id));
    }

    #[gpui::test]
    async fn empty_draft_removes_existing_entry() {
        let mut store = DraftStore::memory_only();
        let channel_id = ChannelId(7);

        store
            .save_draft(channel_id, "hello team")
            .await
            .expect("save draft");
        store
            .save_draft(channel_id, "")
            .await
            .expect("remove empty draft");

        assert_eq!(
            store.load_draft(channel_id).await.expect("load draft"),
            None
        );
        assert!(!store.has_draft(channel_id));
    }

    #[gpui::test]
    async fn channels_with_drafts_are_sorted() {
        let mut store = DraftStore::memory_only();

        store
            .save_draft(ChannelId(3), "third")
            .await
            .expect("save third draft");
        store
            .save_draft(ChannelId(1), "first")
            .await
            .expect("save first draft");
        store
            .save_draft(ChannelId(2), "")
            .await
            .expect("remove empty draft");

        assert_eq!(
            store.channels_with_drafts(),
            vec![ChannelId(1), ChannelId(3)]
        );
    }

    #[gpui::test]
    async fn load_draft_falls_back_to_kvp() {
        let kvp = KeyValueStore::open_test_db("draft_store_load").await;
        let channel_id = ChannelId(7);
        let mut store = DraftStore::new(kvp.clone());

        store
            .save_draft(channel_id, "persisted hello")
            .await
            .expect("save draft");

        let mut store = DraftStore::new(kvp);

        assert_eq!(
            store.load_draft(channel_id).await.expect("load draft"),
            Some("persisted hello".to_string())
        );
        assert!(store.has_draft(channel_id));
    }

    #[gpui::test]
    async fn clear_draft_removes_kvp_entry() {
        let kvp = KeyValueStore::open_test_db("draft_store_clear").await;
        let channel_id = ChannelId(7);
        let mut store = DraftStore::new(kvp.clone());

        store
            .save_draft(channel_id, "persisted hello")
            .await
            .expect("save draft");
        store.clear_draft(channel_id).await.expect("clear draft");

        assert_eq!(
            kvp.scoped(DRAFT_NAMESPACE)
                .read(&DraftStore::persist_key(channel_id))
                .expect("read stored draft"),
            None
        );
    }

    #[gpui::test]
    async fn init_primes_global_store_from_kvp(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| cx.set_global(db::AppDatabase::test_new()));
        let kvp = cx.update(|cx| KeyValueStore::global(cx));
        let channel_id = ChannelId(7);
        let mut store = DraftStore::new(kvp);

        store
            .save_draft(channel_id, "persisted hello")
            .await
            .expect("save draft");

        cx.update(DraftStore::init);

        assert!(cx.update(|cx| DraftStore::global(cx).read(cx).has_draft(channel_id)));
    }

    #[gpui::test]
    async fn read_all_from_kvp_ignores_unknown_keys() {
        let kvp = KeyValueStore::open_test_db("draft_store_unknown_keys").await;
        kvp.scoped(DRAFT_NAMESPACE)
            .write("not-a-channel-draft".to_string(), "{}".to_string())
            .await
            .expect("write unknown key");

        assert!(
            DraftStore::read_all_from_kvp(&kvp)
                .expect("read drafts")
                .is_empty()
        );
    }
}
