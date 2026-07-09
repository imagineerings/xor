use super::new_test_user;
use crate::test_both_dbs;
use collab::db::{
    BookmarkId, ChannelId, ChannelRole, Database, MessageId, UserId,
    bookmark_store::{BookmarkStore, BookmarkUpdate, NewBookmark},
};
use rpc::proto;
use std::sync::Arc;

test_both_dbs!(
    test_bookmark_store_crud_and_empty_results,
    test_bookmark_store_crud_and_empty_results_postgres,
    test_bookmark_store_crud_and_empty_results_sqlite
);

async fn test_bookmark_store_crud_and_empty_results(db: &Arc<Database>) {
    let (store, user_id, channel_id) = setup(db).await;

    assert!(store.get_bookmarks(channel_id).await.unwrap().is_empty());

    let first = store
        .create(new_bookmark(
            channel_id,
            user_id,
            "Docs",
            "https://sim.dev/docs",
        ))
        .await
        .unwrap();
    let second = store
        .create(NewBookmark {
            description: Some("Release checklist".to_string()),
            ..new_bookmark(channel_id, user_id, "Release", "https://sim.dev/release")
        })
        .await
        .unwrap();

    assert_eq!(first.sort_order, 0);
    assert_eq!(second.sort_order, 1);

    let updated = store
        .update(BookmarkUpdate {
            channel_id,
            bookmark_id: first.id,
            label: "Docs v2".to_string(),
            description: Some("Updated docs".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(updated.label, "Docs v2");
    assert_eq!(updated.description.as_deref(), Some("Updated docs"));

    let bookmarks = store.get_bookmarks(channel_id).await.unwrap();
    assert_eq!(bookmark_ids(&bookmarks), vec![first.id, second.id]);
    assert_eq!(bookmarks[0].label, "Docs v2");

    assert!(store.delete(channel_id, first.id).await.unwrap());
    assert!(!store.delete(channel_id, first.id).await.unwrap());
    assert_eq!(
        bookmark_ids(&store.get_bookmarks(channel_id).await.unwrap()),
        vec![second.id]
    );
}

test_both_dbs!(
    test_bookmark_store_reorders_consistently,
    test_bookmark_store_reorders_consistently_postgres,
    test_bookmark_store_reorders_consistently_sqlite
);

async fn test_bookmark_store_reorders_consistently(db: &Arc<Database>) {
    let (store, user_id, channel_id) = setup(db).await;
    let first = store
        .create(new_bookmark(
            channel_id,
            user_id,
            "First",
            "https://sim.dev/1",
        ))
        .await
        .unwrap();
    let second = store
        .create(new_bookmark(
            channel_id,
            user_id,
            "Second",
            "https://sim.dev/2",
        ))
        .await
        .unwrap();
    let third = store
        .create(new_bookmark(
            channel_id,
            user_id,
            "Third",
            "https://sim.dev/3",
        ))
        .await
        .unwrap();

    store
        .reorder(channel_id, vec![third.id, first.id, second.id])
        .await
        .unwrap();
    assert_eq!(
        bookmark_ids(&store.get_bookmarks(channel_id).await.unwrap()),
        vec![third.id, first.id, second.id]
    );

    store
        .reorder(channel_id, vec![second.id, third.id, first.id])
        .await
        .unwrap();
    let reordered = store.get_bookmarks(channel_id).await.unwrap();
    assert_eq!(
        bookmark_ids(&reordered),
        vec![second.id, third.id, first.id]
    );
    assert_eq!(
        reordered
            .iter()
            .map(|bookmark| bookmark.sort_order)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    let first_concurrent_order = vec![first.id, second.id, third.id];
    let second_concurrent_order = vec![third.id, second.id, first.id];
    let (first_result, second_result) = futures::join!(
        store.reorder(channel_id, first_concurrent_order.clone()),
        store.reorder(channel_id, second_concurrent_order.clone())
    );
    first_result.unwrap();
    second_result.unwrap();

    let after_concurrent_reorder = store.get_bookmarks(channel_id).await.unwrap();
    let after_concurrent_ids = bookmark_ids(&after_concurrent_reorder);
    assert!(
        after_concurrent_ids == first_concurrent_order
            || after_concurrent_ids == second_concurrent_order
    );
    assert_eq!(
        after_concurrent_reorder
            .iter()
            .map(|bookmark| bookmark.sort_order)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}

test_both_dbs!(
    test_bookmark_store_rejects_cross_channel_reorder,
    test_bookmark_store_rejects_cross_channel_reorder_postgres,
    test_bookmark_store_rejects_cross_channel_reorder_sqlite
);

async fn test_bookmark_store_rejects_cross_channel_reorder(db: &Arc<Database>) {
    let (store, user_id, channel_id) = setup(db).await;
    let other_channel_id = db
        .create_root_channel("bookmarks-other", user_id)
        .await
        .unwrap();

    let first = store
        .create(new_bookmark(
            channel_id,
            user_id,
            "First",
            "https://sim.dev/1",
        ))
        .await
        .unwrap();
    let other = store
        .create(new_bookmark(
            other_channel_id,
            user_id,
            "Other",
            "https://sim.dev/other",
        ))
        .await
        .unwrap();

    assert!(
        store
            .reorder(channel_id, vec![other.id, first.id])
            .await
            .is_err()
    );
    assert_eq!(
        bookmark_ids(&store.get_bookmarks(channel_id).await.unwrap()),
        vec![first.id]
    );
}

test_both_dbs!(
    test_bookmark_store_deletes_channel_bookmarks,
    test_bookmark_store_deletes_channel_bookmarks_postgres,
    test_bookmark_store_deletes_channel_bookmarks_sqlite
);

async fn test_bookmark_store_deletes_channel_bookmarks(db: &Arc<Database>) {
    let (store, user_id, channel_id) = setup(db).await;
    store
        .create(new_bookmark(
            channel_id,
            user_id,
            "First",
            "https://sim.dev/1",
        ))
        .await
        .unwrap();
    store
        .create(new_bookmark(
            channel_id,
            user_id,
            "Second",
            "https://sim.dev/2",
        ))
        .await
        .unwrap();

    assert_eq!(store.delete_channel_bookmarks(channel_id).await.unwrap(), 2);
    assert!(store.get_bookmarks(channel_id).await.unwrap().is_empty());
}

async fn setup(db: &Arc<Database>) -> (BookmarkStore, UserId, ChannelId) {
    let user_id = new_test_user(db).await;
    let other_user_id = new_test_user(db).await;
    let channel_id = db.create_root_channel("bookmarks", user_id).await.unwrap();
    db.invite_channel_member(channel_id, other_user_id, user_id, ChannelRole::Member)
        .await
        .unwrap();
    db.respond_to_channel_invite(channel_id, other_user_id, true)
        .await
        .unwrap();

    (BookmarkStore::new(db.clone()), user_id, channel_id)
}

fn new_bookmark(channel_id: ChannelId, created_by: UserId, label: &str, url: &str) -> NewBookmark {
    NewBookmark {
        channel_id,
        label: label.to_string(),
        description: None,
        bookmark_type: proto::BookmarkType::BookmarkLink,
        url: url.to_string(),
        file_id: None,
        message_id: None::<MessageId>,
        created_by,
    }
}

fn bookmark_ids(bookmarks: &[collab::db::bookmark_store::Bookmark]) -> Vec<BookmarkId> {
    bookmarks.iter().map(|bookmark| bookmark.id).collect()
}
