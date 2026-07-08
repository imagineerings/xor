use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use client::ChannelId;
use db::kvp::KeyValueStore;
use gpui::{App, AppContext as _, Context, Entity, Global, Task};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use util::ResultExt as _;

const DRAFT_NAMESPACE: &str = "channel_drafts";
const DRAFT_KEY_PREFIX: &str = "channel_draft.";
const MAX_DRAFTS: usize = 50;

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
        for evicted_channel_id in self.evict_oldest_drafts() {
            self.delete_from_kvp(evicted_channel_id).await?;
        }
        Ok(())
    }

    pub fn save_draft_in_background(
        &mut self,
        channel_id: ChannelId,
        body: String,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        self.active_draft_channel = Some(channel_id);
        let kvp = self.kvp.clone();

        if body.trim().is_empty() {
            self.drafts.remove(&channel_id);
            cx.notify();
            return cx.background_spawn(async move {
                Self::delete_from_kvp_with(kvp, channel_id).await
            });
        }

        let draft = Draft {
            body,
            updated_at: Utc::now(),
        };
        self.drafts.insert(channel_id, draft.clone());
        let evicted_channel_ids = self.evict_oldest_drafts();
        cx.notify();

        cx.background_spawn(async move {
            Self::write_to_kvp_with(kvp.clone(), channel_id, &draft).await?;
            for evicted_channel_id in evicted_channel_ids {
                Self::delete_from_kvp_with(kvp.clone(), evicted_channel_id).await?;
            }
            Ok(())
        })
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

    pub fn cached_draft(&mut self, channel_id: ChannelId) -> Option<String> {
        self.active_draft_channel = Some(channel_id);
        self.drafts
            .get(&channel_id)
            .map(|draft| draft.body.clone())
            .filter(|body| !body.trim().is_empty())
    }

    pub async fn clear_draft(&mut self, channel_id: ChannelId) -> Result<()> {
        self.drafts.remove(&channel_id);
        self.delete_from_kvp(channel_id).await?;
        if self.active_draft_channel == Some(channel_id) {
            self.active_draft_channel = None;
        }
        Ok(())
    }

    pub fn clear_draft_in_background(
        &mut self,
        channel_id: ChannelId,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        self.drafts.remove(&channel_id);
        if self.active_draft_channel == Some(channel_id) {
            self.active_draft_channel = None;
        }
        let kvp = self.kvp.clone();
        cx.notify();
        cx.background_spawn(async move { Self::delete_from_kvp_with(kvp, channel_id).await })
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
        Self::write_to_kvp_with(self.kvp.clone(), channel_id, draft).await
    }

    async fn write_to_kvp_with(
        kvp: Option<KeyValueStore>,
        channel_id: ChannelId,
        draft: &Draft,
    ) -> Result<()> {
        let Some(kvp) = kvp.as_ref() else {
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
        match serde_json::from_str(&payload)
            .context("deserializing channel draft")
            .log_err()
        {
            Some(draft) => Ok(Some(draft)),
            None => {
                self.delete_from_kvp(channel_id).await?;
                Ok(None)
            }
        }
    }

    async fn delete_from_kvp(&self, channel_id: ChannelId) -> Result<()> {
        Self::delete_from_kvp_with(self.kvp.clone(), channel_id).await
    }

    async fn delete_from_kvp_with(kvp: Option<KeyValueStore>, channel_id: ChannelId) -> Result<()> {
        let Some(kvp) = kvp.as_ref() else {
            return Ok(());
        };
        kvp.scoped(DRAFT_NAMESPACE)
            .delete(Self::persist_key(channel_id))
            .await
            .context("deleting channel draft")
    }

    fn evict_oldest_drafts(&mut self) -> Vec<ChannelId> {
        if self.drafts.len() <= MAX_DRAFTS {
            return Vec::new();
        }

        let mut drafts_by_age = self
            .drafts
            .iter()
            .map(|(channel_id, draft)| (*channel_id, draft.updated_at))
            .collect::<Vec<_>>();
        drafts_by_age.sort_by_key(|(channel_id, updated_at)| (*updated_at, *channel_id));

        let evicted_channel_ids = drafts_by_age
            .into_iter()
            .take(self.drafts.len() - MAX_DRAFTS)
            .map(|(channel_id, _)| channel_id)
            .collect::<Vec<_>>();

        for channel_id in &evicted_channel_ids {
            self.drafts.remove(channel_id);
            if self.active_draft_channel == Some(*channel_id) {
                self.active_draft_channel = None;
            }
        }

        evicted_channel_ids
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
            if let Some(draft) = serde_json::from_str(&payload)
                .with_context(|| format!("deserializing channel draft {key}"))
                .log_err()
            {
                drafts.insert(channel_id, draft);
            }
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
    use chrono::Duration;

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
    async fn save_draft_evicts_oldest_memory_entry() {
        let now = Utc::now();
        let mut drafts = HashMap::default();
        for index in 0..MAX_DRAFTS {
            let channel_id = ChannelId(index as u64);
            drafts.insert(
                channel_id,
                Draft {
                    body: format!("draft {index}"),
                    updated_at: now + Duration::seconds(index as i64),
                },
            );
        }
        let mut store = DraftStore {
            kvp: None,
            drafts,
            active_draft_channel: None,
        };

        store
            .save_draft(ChannelId(1000), "newest draft")
            .await
            .expect("save newest draft");

        assert_eq!(store.channels_with_drafts().len(), MAX_DRAFTS);
        assert!(!store.has_draft(ChannelId(0)));
        assert!(store.has_draft(ChannelId(1000)));
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
    async fn load_draft_discards_corrupt_kvp_entry() {
        let kvp = KeyValueStore::open_test_db("draft_store_corrupt_load").await;
        let channel_id = ChannelId(7);
        kvp.scoped(DRAFT_NAMESPACE)
            .write(
                DraftStore::persist_key(channel_id),
                "not valid json".to_string(),
            )
            .await
            .expect("write corrupt draft");
        let mut store = DraftStore::new(kvp.clone());

        assert_eq!(
            store.load_draft(channel_id).await.expect("load draft"),
            None
        );
        assert_eq!(
            kvp.scoped(DRAFT_NAMESPACE)
                .read(&DraftStore::persist_key(channel_id))
                .expect("read corrupt draft"),
            None
        );
    }

    #[gpui::test]
    async fn cached_draft_returns_in_memory_body() {
        let mut store = DraftStore::memory_only();
        let channel_id = ChannelId(7);

        store
            .save_draft(channel_id, "cached hello")
            .await
            .expect("save draft");

        assert_eq!(
            store.cached_draft(channel_id),
            Some("cached hello".to_string())
        );
    }

    #[gpui::test]
    async fn save_draft_evicts_oldest_kvp_entry() {
        let kvp = KeyValueStore::open_test_db("draft_store_limit").await;
        let now = Utc::now();
        let mut drafts = HashMap::default();
        for index in 0..MAX_DRAFTS {
            let channel_id = ChannelId(index as u64);
            let draft = Draft {
                body: format!("draft {index}"),
                updated_at: now + Duration::seconds(index as i64),
            };
            drafts.insert(channel_id, draft.clone());
            DraftStore::new(kvp.clone())
                .write_to_kvp(channel_id, &draft)
                .await
                .expect("write seeded draft");
        }
        let mut store = DraftStore::new_with_drafts(kvp.clone(), drafts);

        store
            .save_draft(ChannelId(1000), "newest draft")
            .await
            .expect("save newest draft");

        assert_eq!(
            kvp.scoped(DRAFT_NAMESPACE)
                .read(&DraftStore::persist_key(ChannelId(0)))
                .expect("read evicted draft"),
            None
        );
        assert!(
            kvp.scoped(DRAFT_NAMESPACE)
                .read(&DraftStore::persist_key(ChannelId(1000)))
                .expect("read newest draft")
                .is_some()
        );
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
    async fn clear_draft_in_background_removes_cache_and_kvp(cx: &mut gpui::TestAppContext) {
        let kvp = KeyValueStore::open_test_db("draft_store_background_clear").await;
        let channel_id = ChannelId(7);
        let store = cx.update(|cx| cx.new(|_| DraftStore::new(kvp.clone())));

        let save_task = cx.update(|cx| {
            store.update(cx, |store, cx| {
                store.save_draft_in_background(channel_id, "persisted hello".to_string(), cx)
            })
        });
        save_task.await.expect("save draft");

        let clear_task = cx.update(|cx| {
            store.update(cx, |store, cx| {
                store.clear_draft_in_background(channel_id, cx)
            })
        });
        clear_task.await.expect("clear draft");

        assert!(!cx.update(|cx| store.read(cx).has_draft(channel_id)));
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

    #[gpui::test]
    async fn read_all_from_kvp_ignores_corrupt_entries() {
        let kvp = KeyValueStore::open_test_db("draft_store_corrupt_read_all").await;
        let valid_channel_id = ChannelId(1);
        let corrupt_channel_id = ChannelId(2);
        let valid_draft = Draft {
            body: "valid draft".to_string(),
            updated_at: Utc::now(),
        };
        kvp.scoped(DRAFT_NAMESPACE)
            .write(
                DraftStore::persist_key(valid_channel_id),
                serde_json::to_string(&valid_draft).expect("serialize draft"),
            )
            .await
            .expect("write valid draft");
        kvp.scoped(DRAFT_NAMESPACE)
            .write(
                DraftStore::persist_key(corrupt_channel_id),
                "not valid json".to_string(),
            )
            .await
            .expect("write corrupt draft");

        let drafts = DraftStore::read_all_from_kvp(&kvp).expect("read drafts");

        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts.get(&valid_channel_id), Some(&valid_draft));
        assert!(!drafts.contains_key(&corrupt_channel_id));
    }
}
