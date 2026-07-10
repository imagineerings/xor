use super::new_test_user;
use crate::test_both_dbs;
use collab::db::{ChannelId, ChannelRole, Database, UserId, join_request_store::JoinRequestStore};
use std::sync::Arc;
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

test_both_dbs!(
    test_join_request_store_lifecycle,
    test_join_request_store_lifecycle_postgres,
    test_join_request_store_lifecycle_sqlite
);

async fn test_join_request_store_lifecycle(db: &Arc<Database>) {
    let (store, owner_id, requester_id, channel_id) = setup(db).await;

    let request = store
        .request_join(
            channel_id,
            requester_id,
            Some("I can help with releases".to_string()),
        )
        .await
        .unwrap();
    assert_eq!(request.channel_id, channel_id);
    assert_eq!(request.user_id, requester_id);
    assert_eq!(request.reason.as_deref(), Some("I can help with releases"));
    assert!(
        store
            .pending_join_request_exists(channel_id, requester_id)
            .await
            .unwrap()
    );
    assert_eq!(store.count_pending_requests(channel_id).await.unwrap(), 1);
    assert!(
        store
            .request_join(channel_id, requester_id, None)
            .await
            .is_err()
    );

    let pending = store.get_pending_requests(channel_id).await.unwrap();
    assert_eq!(pending, vec![request]);

    assert!(
        store
            .approve_join_request(channel_id, requester_id)
            .await
            .unwrap()
    );
    assert!(
        !store
            .pending_join_request_exists(channel_id, requester_id)
            .await
            .unwrap()
    );
    assert_eq!(store.count_pending_requests(channel_id).await.unwrap(), 0);

    let channel = db.get_channel(channel_id, owner_id).await.unwrap();
    let members = db.get_channel_members(&channel, 10).await.unwrap();
    assert!(members.iter().any(|member| {
        member.user_id == requester_id && member.accepted && member.role == ChannelRole::Member
    }));

    store
        .request_join(channel_id, requester_id, None)
        .await
        .unwrap();
    assert!(
        store
            .deny_join_request(channel_id, requester_id)
            .await
            .unwrap()
    );
    assert!(
        !store
            .deny_join_request(channel_id, requester_id)
            .await
            .unwrap()
    );
}

test_both_dbs!(
    test_join_request_store_expires_requests,
    test_join_request_store_expires_requests_postgres,
    test_join_request_store_expires_requests_sqlite
);

async fn test_join_request_store_expires_requests(db: &Arc<Database>) {
    let (store, _, requester_id, channel_id) = setup(db).await;
    store
        .request_join(channel_id, requester_id, Some("Please add me".to_string()))
        .await
        .unwrap();

    let threshold = utc_now() + Duration::seconds(1);
    let expired = store.expire_old_requests(threshold).await.unwrap();

    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].channel_id, channel_id);
    assert_eq!(expired[0].channel_name, "join-requests");
    assert_eq!(expired[0].user_id, requester_id);
    assert_eq!(store.count_pending_requests(channel_id).await.unwrap(), 0);
}

async fn setup(db: &Arc<Database>) -> (JoinRequestStore, UserId, UserId, ChannelId) {
    let owner_id = new_test_user(db).await;
    let requester_id = new_test_user(db).await;
    let channel_id = db
        .create_root_channel("join-requests", owner_id)
        .await
        .unwrap();
    (
        JoinRequestStore::new(db.clone()),
        owner_id,
        requester_id,
        channel_id,
    )
}

fn utc_now() -> PrimitiveDateTime {
    let now = OffsetDateTime::now_utc();
    PrimitiveDateTime::new(now.date(), now.time())
}
