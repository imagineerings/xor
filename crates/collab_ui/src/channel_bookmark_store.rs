use anyhow::Result;
use client::{Bookmark, ChannelId, Client, Subscription};
use collections::HashMap;
use gpui::{AsyncApp, Context, Entity};
use rpc::{TypedEnvelope, proto};
use std::sync::Arc;

pub struct ChannelBookmarkStore {
    bookmarks_by_channel: HashMap<ChannelId, Vec<Bookmark>>,
    _subscription: Option<Subscription>,
}

impl ChannelBookmarkStore {
    pub fn new(client: Arc<Client>, cx: &mut Context<Self>) -> Self {
        let subscription =
            client.add_channel_bookmarks_update_handler(cx.weak_entity(), Self::handle_update);
        Self {
            bookmarks_by_channel: HashMap::default(),
            _subscription: Some(subscription),
        }
    }

    pub fn bookmarks(&self, channel_id: ChannelId) -> &[Bookmark] {
        self.bookmarks_by_channel
            .get(&channel_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn set_bookmarks(
        &mut self,
        channel_id: ChannelId,
        bookmarks: Vec<Bookmark>,
        cx: &mut Context<Self>,
    ) {
        self.bookmarks_by_channel.insert(channel_id, bookmarks);
        cx.notify();
    }

    pub fn clear_channel(&mut self, channel_id: ChannelId, cx: &mut Context<Self>) {
        self.bookmarks_by_channel.remove(&channel_id);
        cx.notify();
    }

    async fn handle_update(
        this: Entity<Self>,
        message: TypedEnvelope<proto::UpdateChannelBookmarks>,
        mut cx: AsyncApp,
    ) -> Result<()> {
        let channel_id = ChannelId(message.payload.channel_id);
        let bookmarks = message
            .payload
            .bookmarks
            .into_iter()
            .map(Bookmark::try_from)
            .collect::<Result<Vec<_>>>()?;
        this.update(&mut cx, |this, cx| {
            this.apply_update(channel_id, bookmarks, cx);
        });
        Ok(())
    }

    fn apply_update(
        &mut self,
        channel_id: ChannelId,
        bookmarks: Vec<Bookmark>,
        cx: &mut Context<Self>,
    ) {
        self.bookmarks_by_channel.insert(channel_id, bookmarks);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, TestAppContext};

    #[gpui::test]
    fn store_replaces_bookmarks_for_channel(cx: &mut TestAppContext) {
        let store = cx.new(|_| ChannelBookmarkStore {
            bookmarks_by_channel: HashMap::default(),
            _subscription: None,
        });
        let channel_id = ChannelId(42);
        let bookmark = Bookmark::try_from(proto::Bookmark {
            id: 1,
            channel_id: channel_id.0,
            label: "Runbook".to_string(),
            url: "https://sim.dev/runbook".to_string(),
            file_id: None,
            message_id: None,
            r#type: proto::BookmarkType::BookmarkLink as i32,
            created_by: 7,
            created_at: 0,
            description: None,
            sort_order: 0,
        })
        .unwrap();

        store.update(cx, |store, cx| {
            store.apply_update(channel_id, vec![bookmark.clone()], cx);
        });
        store.read_with(cx, |store, _| {
            assert_eq!(store.bookmarks(channel_id), &[bookmark]);
        });
    }
}
