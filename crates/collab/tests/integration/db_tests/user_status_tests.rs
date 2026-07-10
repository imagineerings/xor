use super::new_test_user;
use crate::test_both_dbs;
use collab::{
    db::{Database, UserId, user_status_store::UserStatusStore},
};
use std::sync::Arc;
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

test_both_dbs!(
    test_user_status_store_lifecycle,
    test_user_status_store_lifecycle_postgres,
    test_user_status_store_lifecycle_sqlite
);

async fn test_user_status_store_lifecycle(db: &Arc<Database>) {
    let user_id = new_test_user(db).await;
    let store = UserStatusStore::new(db.clone());
    let expires_at = utc_now() + Duration::hours(1);

    let status = store
        .upsert_custom_status(
            user_id,
            Some(":speech_balloon:".to_string()),
            "Reviewing changes".to_string(),
            Some(expires_at),
        )
        .await
        .unwrap();
    assert_eq!(status.user_id, user_id);
    assert_eq!(status.emoji.as_deref(), Some(":speech_balloon:"));
    assert_eq!(status.status_text, "Reviewing changes");
    assert_eq!(status.expires_at, Some(expires_at));

    let updated = store
        .upsert_custom_status(user_id, None, "Available".to_string(), None)
        .await
        .unwrap();
    assert_eq!(updated.user_id, user_id);
    assert_eq!(updated.emoji, None);
    assert_eq!(updated.status_text, "Available");
    assert_eq!(updated.expires_at, None);

    assert!(store.delete_custom_status(user_id).await.unwrap());
    assert!(!store.delete_custom_status(user_id).await.unwrap());
}

test_both_dbs!(
    test_user_status_store_expires_statuses,
    test_user_status_store_expires_statuses_postgres,
    test_user_status_store_expires_statuses_sqlite
);

async fn test_user_status_store_expires_statuses(db: &Arc<Database>) {
    let expired_user_id = new_test_user(db).await;
    let active_user_id = new_test_user(db).await;
    let store = UserStatusStore::new(db.clone());
    let now = utc_now();

    store
        .upsert_custom_status(
            expired_user_id,
            None,
            "Away".to_string(),
            Some(now - Duration::seconds(1)),
        )
        .await
        .unwrap();
    store
        .upsert_custom_status(
            active_user_id,
            None,
            "In a meeting".to_string(),
            Some(now + Duration::hours(1)),
        )
        .await
        .unwrap();

    assert_eq!(store.delete_expired_custom_statuses(now).await.unwrap(), vec![expired_user_id]);
    assert!(!store.delete_custom_status(expired_user_id).await.unwrap());
    assert!(store.delete_custom_status(active_user_id).await.unwrap());
}

fn utc_now() -> PrimitiveDateTime {
    let now = OffsetDateTime::now_utc();
    PrimitiveDateTime::new(now.date(), now.time())
}
