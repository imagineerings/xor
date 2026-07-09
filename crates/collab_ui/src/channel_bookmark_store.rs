use anyhow::Result;
use client::{Bookmark, BookmarkId, ChannelId, Client, Subscription};
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

    pub fn reorder_bookmarks(
        &mut self,
        channel_id: ChannelId,
        ordered_ids: &[BookmarkId],
        cx: &mut Context<Self>,
    ) -> Option<Vec<Bookmark>> {
        let bookmarks = self.bookmarks_by_channel.get_mut(&channel_id)?;
        if bookmarks.len() != ordered_ids.len() {
            return None;
        }

        let previous = bookmarks.clone();
        if previous
            .iter()
            .any(|bookmark| !ordered_ids.contains(&bookmark.id))
        {
            return None;
        }

        let mut reordered = Vec::with_capacity(bookmarks.len());
        for (sort_order, bookmark_id) in ordered_ids.iter().enumerate() {
            let Some(mut bookmark) = previous
                .iter()
                .find(|bookmark| bookmark.id == *bookmark_id)
                .cloned()
            else {
                return None;
            };
            bookmark.sort_order = sort_order as u32;
            reordered.push(bookmark);
        }

        *bookmarks = reordered;
        cx.notify();
        Some(previous)
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

    #[gpui::test]
    fn store_reorders_bookmarks_for_channel(cx: &mut TestAppContext) {
        let store = cx.new(|_| ChannelBookmarkStore {
            bookmarks_by_channel: HashMap::default(),
            _subscription: None,
        });
        let channel_id = ChannelId(42);
        let bookmarks = vec![
            bookmark_for_test(channel_id, BookmarkId(1), 0),
            bookmark_for_test(channel_id, BookmarkId(2), 1),
            bookmark_for_test(channel_id, BookmarkId(3), 2),
        ];
        store.update(cx, |store, cx| {
            store.set_bookmarks(channel_id, bookmarks.clone(), cx);
        });

        let previous = store.update(cx, |store, cx| {
            store
                .reorder_bookmarks(
                    channel_id,
                    &[BookmarkId(3), BookmarkId(1), BookmarkId(2)],
                    cx,
                )
                .unwrap()
        });

        assert_eq!(previous, bookmarks);
        store.read_with(cx, |store, _| {
            let reordered = store.bookmarks(channel_id);
            assert_eq!(
                reordered
                    .iter()
                    .map(|bookmark| bookmark.id)
                    .collect::<Vec<_>>(),
                vec![BookmarkId(3), BookmarkId(1), BookmarkId(2)]
            );
            assert_eq!(
                reordered
                    .iter()
                    .map(|bookmark| bookmark.sort_order)
                    .collect::<Vec<_>>(),
                vec![0, 1, 2]
            );
        });
    }

    #[gpui::test]
    fn store_rejects_incomplete_reorder(cx: &mut TestAppContext) {
        let store = cx.new(|_| ChannelBookmarkStore {
            bookmarks_by_channel: HashMap::default(),
            _subscription: None,
        });
        let channel_id = ChannelId(42);
        let bookmarks = vec![
            bookmark_for_test(channel_id, BookmarkId(1), 0),
            bookmark_for_test(channel_id, BookmarkId(2), 1),
        ];
        store.update(cx, |store, cx| {
            store.set_bookmarks(channel_id, bookmarks.clone(), cx);
        });

        let previous = store.update(cx, |store, cx| {
            store.reorder_bookmarks(channel_id, &[BookmarkId(2)], cx)
        });

        assert!(previous.is_none());
        store.read_with(cx, |store, _| {
            assert_eq!(store.bookmarks(channel_id), bookmarks);
        });
    }

    #[gpui::test]
    fn store_rejects_duplicate_reorder_ids(cx: &mut TestAppContext) {
        let store = cx.new(|_| ChannelBookmarkStore {
            bookmarks_by_channel: HashMap::default(),
            _subscription: None,
        });
        let channel_id = ChannelId(42);
        let bookmarks = vec![
            bookmark_for_test(channel_id, BookmarkId(1), 0),
            bookmark_for_test(channel_id, BookmarkId(2), 1),
        ];
        store.update(cx, |store, cx| {
            store.set_bookmarks(channel_id, bookmarks.clone(), cx);
        });

        let previous = store.update(cx, |store, cx| {
            store.reorder_bookmarks(channel_id, &[BookmarkId(1), BookmarkId(1)], cx)
        });

        assert!(previous.is_none());
        store.read_with(cx, |store, _| {
            assert_eq!(store.bookmarks(channel_id), bookmarks);
        });
    }

    fn bookmark_for_test(channel_id: ChannelId, id: BookmarkId, sort_order: u32) -> Bookmark {
        Bookmark::try_from(proto::Bookmark {
            id: id.0,
            channel_id: channel_id.0,
            label: format!("Bookmark {}", id.0),
            url: "https://sim.dev/runbook".to_string(),
            file_id: None,
            message_id: None,
            r#type: proto::BookmarkType::BookmarkLink as i32,
            created_by: 7,
            created_at: 0,
            description: None,
            sort_order,
        })
        .unwrap()
    }
}
