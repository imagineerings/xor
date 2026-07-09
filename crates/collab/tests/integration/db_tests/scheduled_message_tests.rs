use super::new_test_user;
use crate::test_both_dbs;
use collab::db::{
    ChannelId, ChannelRole, Database, UserId,
    scheduled_message_store::{NewScheduledMessage, ScheduledMessageStore, ScheduledMessageUpdate},
};
use rpc::proto;
use std::sync::Arc;
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

const STATE_PROCESSING: i16 = 1;
const STATE_FAILED: i16 = 3;

test_both_dbs!(
    test_scheduled_message_create_validation_and_dedup,
    test_scheduled_message_create_validation_and_dedup_postgres,
    test_scheduled_message_create_validation_and_dedup_sqlite
);

async fn test_scheduled_message_create_validation_and_dedup(db: &Arc<Database>) {
    let (store, sender_id, _, channel_id) = setup(db).await;

    assert!(
        store
            .create(new_scheduled_message(
                channel_id,
                sender_id,
                "too soon",
                in_utc(Duration::seconds(30)),
                1,
            ))
            .await
            .is_err()
    );
    assert!(
        store
            .create(new_scheduled_message(
                channel_id,
                sender_id,
                "too late",
                in_utc(Duration::days(31)),
                2,
            ))
            .await
            .is_err()
    );

    let first_id = store
        .create(new_scheduled_message(
            channel_id,
            sender_id,
            "first",
            in_utc(Duration::minutes(5)),
            3,
        ))
        .await
        .unwrap();
    let duplicate_id = store
        .create(new_scheduled_message(
            channel_id,
            sender_id,
            "duplicate",
            in_utc(Duration::minutes(10)),
            3,
        ))
        .await
        .unwrap();

    assert_eq!(first_id, duplicate_id);
    assert_eq!(store.count_pending_for_user(sender_id).await.unwrap(), 1);
    let messages = store.list_for_user(sender_id, channel_id).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].body, "first");
}

test_both_dbs!(
    test_scheduled_message_cancel_ownership_and_pending_state,
    test_scheduled_message_cancel_ownership_and_pending_state_postgres,
    test_scheduled_message_cancel_ownership_and_pending_state_sqlite
);

async fn test_scheduled_message_cancel_ownership_and_pending_state(db: &Arc<Database>) {
    let (store, sender_id, other_user_id, channel_id) = setup(db).await;
    let message_id = store
        .create(new_scheduled_message(
            channel_id,
            sender_id,
            "cancel me",
            in_utc(Duration::minutes(5)),
            4,
        ))
        .await
        .unwrap();

    assert!(
        store
            .cancel(message_id, channel_id, other_user_id)
            .await
            .is_err()
    );
    let cancelled = store
        .cancel(message_id, channel_id, sender_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cancelled.id, message_id);
    assert!(
        store
            .cancel(message_id, channel_id, sender_id)
            .await
            .unwrap()
            .is_none()
    );

    let processing_id = store
        .create(new_scheduled_message(
            channel_id,
            sender_id,
            "processing",
            in_utc(Duration::minutes(5)),
            5,
        ))
        .await
        .unwrap();
    store
        .set_state_for_test(processing_id, STATE_PROCESSING)
        .await
        .unwrap();
    assert!(
        store
            .cancel(processing_id, channel_id, sender_id)
            .await
            .unwrap()
            .is_none()
    );
}

test_both_dbs!(
    test_scheduled_message_update_validation_and_pending_state,
    test_scheduled_message_update_validation_and_pending_state_postgres,
    test_scheduled_message_update_validation_and_pending_state_sqlite
);

async fn test_scheduled_message_update_validation_and_pending_state(db: &Arc<Database>) {
    let (store, sender_id, _, channel_id) = setup(db).await;
    let message_id = store
        .create(new_scheduled_message(
            channel_id,
            sender_id,
            "before",
            in_utc(Duration::minutes(5)),
            6,
        ))
        .await
        .unwrap();

    assert!(
        store
            .update(ScheduledMessageUpdate {
                scheduled_message_id: message_id,
                channel_id,
                sender_id,
                body: None,
                scheduled_at: Some(in_utc(Duration::seconds(30))),
                mentions: None,
            })
            .await
            .is_err()
    );

    let updated = store
        .update(ScheduledMessageUpdate {
            scheduled_message_id: message_id,
            channel_id,
            sender_id,
            body: Some("after".to_string()),
            scheduled_at: Some(in_utc(Duration::minutes(15))),
            mentions: Some(Vec::new()),
        })
        .await
        .unwrap();
    assert_eq!(updated.body, "after");

    store
        .set_state_for_test(message_id, STATE_FAILED)
        .await
        .unwrap();
    let error = store
        .update(ScheduledMessageUpdate {
            scheduled_message_id: message_id,
            channel_id,
            sender_id,
            body: Some("too late".to_string()),
            scheduled_at: None,
            mentions: None,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("already failed"));
}

test_both_dbs!(
    test_scheduled_message_pop_due_counts_and_stale_reset,
    test_scheduled_message_pop_due_counts_and_stale_reset_postgres,
    test_scheduled_message_pop_due_counts_and_stale_reset_sqlite
);

async fn test_scheduled_message_pop_due_counts_and_stale_reset(db: &Arc<Database>) {
    let (store, sender_id, _, channel_id) = setup(db).await;
    let first_id = store
        .create(new_scheduled_message(
            channel_id,
            sender_id,
            "first due",
            in_utc(Duration::minutes(5)),
            7,
        ))
        .await
        .unwrap();
    let second_id = store
        .create(new_scheduled_message(
            channel_id,
            sender_id,
            "second due",
            in_utc(Duration::minutes(5)),
            8,
        ))
        .await
        .unwrap();
    store
        .set_scheduled_at_for_test(first_id, in_utc(Duration::minutes(-5)))
        .await
        .unwrap();
    store
        .set_scheduled_at_for_test(second_id, in_utc(Duration::minutes(-4)))
        .await
        .unwrap();

    assert_eq!(store.count_pending_for_user(sender_id).await.unwrap(), 2);
    let first_due = store.pop_due_with_limit(1).await.unwrap();
    assert_eq!(first_due.len(), 1);
    assert_eq!(first_due[0].id, first_id);
    assert_eq!(store.count_pending_for_user(sender_id).await.unwrap(), 1);

    let second_due = store.pop_due_with_limit(10).await.unwrap();
    assert_eq!(second_due.len(), 1);
    assert_eq!(second_due[0].id, second_id);
    assert!(store.pop_due().await.unwrap().is_empty());

    store
        .set_updated_at_for_test(first_id, in_utc(Duration::minutes(-10)))
        .await
        .unwrap();
    store
        .set_updated_at_for_test(second_id, in_utc(Duration::minutes(-10)))
        .await
        .unwrap();
    assert_eq!(
        store
            .reset_stale_processing(in_utc(Duration::minutes(-1)))
            .await
            .unwrap(),
        2
    );
    assert_eq!(store.count_pending_for_user(sender_id).await.unwrap(), 2);
}

async fn setup(db: &Arc<Database>) -> (ScheduledMessageStore, UserId, UserId, ChannelId) {
    let sender_id = new_test_user(db).await;
    let other_user_id = new_test_user(db).await;
    let channel_id = db
        .create_root_channel("scheduled", sender_id)
        .await
        .unwrap();
    db.invite_channel_member(channel_id, other_user_id, sender_id, ChannelRole::Member)
        .await
        .unwrap();
    db.respond_to_channel_invite(channel_id, other_user_id, true)
        .await
        .unwrap();
    (
        ScheduledMessageStore::new(db.clone()),
        sender_id,
        other_user_id,
        channel_id,
    )
}

fn new_scheduled_message(
    channel_id: ChannelId,
    sender_id: UserId,
    body: &str,
    scheduled_at: PrimitiveDateTime,
    nonce: u128,
) -> NewScheduledMessage {
    NewScheduledMessage {
        channel_id,
        sender_id,
        body: body.to_string(),
        scheduled_at,
        nonce: proto::Nonce::from(nonce),
        mentions: Vec::new(),
    }
}

fn in_utc(offset: Duration) -> PrimitiveDateTime {
    let timestamp = OffsetDateTime::now_utc() + offset;
    PrimitiveDateTime::new(timestamp.date(), timestamp.time())
}
