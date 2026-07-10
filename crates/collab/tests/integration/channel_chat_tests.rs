use crate::TestServer;
use client::{
    MessagePriority, RECEIVE_TIMEOUT,
    channel_chat::{
        DEFAULT_THREAD_REPLY_LIMIT, ScheduleChannelMessage, SearchChannelMessages,
        SendChannelMessage, UpdateChannelMessage,
    },
    file_upload::GetFileUploadUrl,
    proto,
};
use collab::{
    db::{
        ChannelId as DbChannelId, ChannelRole as DbChannelRole, GroupId as DbGroupId,
        ScheduledMessageId as DbScheduledMessageId, UserId as DbUserId, channel_file,
        scheduled_message_store::ScheduledMessageStore, user_status_store::UserStatusStore,
    },
    executor::Executor,
    rpc::{ConnectionPool, RECONNECT_TIMEOUT, Server},
    status_expiry_sweeper::StatusExpirySweeper,
};
use gpui::{AppContext, BackgroundExecutor, TestAppContext};
use pretty_assertions::assert_eq;
use rpc::{ErrorExt as _, Notification};
use sea_orm::EntityTrait as _;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use time::{Duration as TimeDuration, OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

#[derive(Default)]
struct ReactionHandlerEntity;

#[derive(Default)]
struct ScheduledMessageHandlerEntity;

#[derive(Default)]
struct BookmarkHandlerEntity;

#[derive(Default)]
struct ChannelMessageHandlerEntity;

#[gpui::test]
async fn test_channel_chat_core_flow(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    let initial_messages = client_a
        .join_channel_chat(channel_id.0)
        .await
        .unwrap()
        .messages;
    assert!(initial_messages.is_empty());
    client_b.join_channel_chat(channel_id.0).await.unwrap();

    let sent = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "hello from a".to_string(),
            nonce: 1,
            mentions: vec![mention_for(client_b.user_id().unwrap(), 6, 10)],
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();

    let history = client_b
        .get_channel_messages(channel_id.0, None)
        .await
        .unwrap()
        .messages;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, sent.id);
    assert_eq!(history[0].body, "hello from a");
    assert_eq!(history[0].mentions.len(), 1);

    client_a
        .update_channel_message(UpdateChannelMessage {
            channel_id: channel_id.0,
            message_id: sent.id,
            body: "edited from a".to_string(),
            nonce: 2,
            mentions: Vec::new(),
        })
        .await
        .unwrap();

    let updated = client_b
        .get_channel_messages_by_id(vec![sent.id])
        .await
        .unwrap()
        .messages;
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].body, "edited from a");
    assert!(updated[0].edited_at.is_some());
    assert!(updated[0].mentions.is_empty());

    client_b
        .acknowledge_channel_message(channel_id.0, sent.id)
        .unwrap();

    client_a
        .remove_channel_message(channel_id.0, sent.id)
        .await
        .unwrap();

    let deleted = client_b
        .get_channel_messages_by_id(vec![sent.id])
        .await
        .unwrap()
        .messages;
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].body, "");
    assert!(deleted[0].mentions.is_empty());
}

#[gpui::test]
async fn join_request_approve_flow_adds_requester_to_channel(
    executor: BackgroundExecutor,
    cx_admin: &mut TestAppContext,
    cx_requester: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let admin = server.create_client(cx_admin, "admin").await;
    let requester = server.create_client(cx_requester, "requester").await;
    let channel_id = server
        .make_channel("private-chat", None, (&admin, cx_admin), &mut [])
        .await;

    requester
        .client()
        .request(proto::RequestJoinChannel {
            channel_id: channel_id.0,
            reason: Some("I need access to coordinate releases".to_string()),
        })
        .await
        .unwrap();

    let pending = admin
        .client()
        .request(proto::GetPendingJoinRequests {
            channel_id: channel_id.0,
        })
        .await
        .unwrap();
    assert_eq!(pending.requests.len(), 1);
    assert_eq!(pending.requests[0].user_id, requester.user_id().unwrap());
    assert_eq!(
        pending.requests[0].reason.as_deref(),
        Some("I need access to coordinate releases")
    );

    admin
        .client()
        .request(proto::RespondToJoinRequest {
            channel_id: channel_id.0,
            requesting_user_id: requester.user_id().unwrap(),
            approve: true,
            denial_reason: None,
        })
        .await
        .unwrap();

    requester.join_channel_chat(channel_id.0).await.unwrap();
}

#[gpui::test]
async fn join_request_deny_flow_keeps_requester_out_of_channel(
    executor: BackgroundExecutor,
    cx_admin: &mut TestAppContext,
    cx_requester: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let admin = server.create_client(cx_admin, "admin").await;
    let requester = server.create_client(cx_requester, "requester").await;
    let channel_id = server
        .make_channel("private-chat", None, (&admin, cx_admin), &mut [])
        .await;

    requester
        .client()
        .request(proto::RequestJoinChannel {
            channel_id: channel_id.0,
            reason: Some("Please let me in".to_string()),
        })
        .await
        .unwrap();
    admin
        .client()
        .request(proto::RespondToJoinRequest {
            channel_id: channel_id.0,
            requesting_user_id: requester.user_id().unwrap(),
            approve: false,
            denial_reason: Some("Please ask an administrator first".to_string()),
        })
        .await
        .unwrap();

    assert!(requester.join_channel_chat(channel_id.0).await.is_err());
}

#[gpui::test]
async fn join_request_response_requires_channel_admin(
    executor: BackgroundExecutor,
    cx_admin: &mut TestAppContext,
    cx_requester: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let admin = server.create_client(cx_admin, "admin").await;
    let requester = server.create_client(cx_requester, "requester").await;
    let channel_id = server
        .make_channel("private-chat", None, (&admin, cx_admin), &mut [])
        .await;

    requester
        .client()
        .request(proto::RequestJoinChannel {
            channel_id: channel_id.0,
            reason: None,
        })
        .await
        .unwrap();

    let error = requester
        .client()
        .request(proto::RespondToJoinRequest {
            channel_id: channel_id.0,
            requesting_user_id: requester.user_id().unwrap(),
            approve: true,
            denial_reason: None,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("admin"));
}

#[gpui::test]
async fn join_request_rejects_reason_over_500_characters(
    executor: BackgroundExecutor,
    cx_admin: &mut TestAppContext,
    cx_requester: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor).await;
    let admin = server.create_client(cx_admin, "admin").await;
    let requester = server.create_client(cx_requester, "requester").await;
    let channel_id = server
        .make_channel("private-chat", None, (&admin, cx_admin), &mut [])
        .await;

    let error = requester
        .client()
        .request(proto::RequestJoinChannel {
            channel_id: channel_id.0,
            reason: Some("x".repeat(501)),
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("at most 500"));
}

#[gpui::test]
async fn direct_invite_prevents_duplicate_join_request(
    executor: BackgroundExecutor,
    cx_admin: &mut TestAppContext,
    cx_requester: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor).await;
    let admin = server.create_client(cx_admin, "admin").await;
    let requester = server.create_client(cx_requester, "requester").await;
    let channel_id = server
        .make_channel("private-chat", None, (&admin, cx_admin), &mut [])
        .await;
    server
        .app_state
        .db
        .invite_channel_member(
            DbChannelId::from_proto(channel_id.0),
            DbUserId::from_proto(requester.user_id().unwrap()),
            DbUserId::from_proto(admin.user_id().unwrap()),
            DbChannelRole::Member,
        )
        .await
        .unwrap();
    server
        .app_state
        .db
        .respond_to_channel_invite(
            DbChannelId::from_proto(channel_id.0),
            DbUserId::from_proto(requester.user_id().unwrap()),
            true,
        )
        .await
        .unwrap();

    let error = requester
        .client()
        .request(proto::RequestJoinChannel {
            channel_id: channel_id.0,
            reason: None,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("already"));
}

#[gpui::test]
async fn custom_status_expiry_sweeper_deletes_expired_statuses(
    executor: BackgroundExecutor,
    cx: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client = server.create_client(cx, "status-owner").await;
    let expired_at = OffsetDateTime::now_utc() - TimeDuration::minutes(1);
    let expired_at = PrimitiveDateTime::new(expired_at.date(), expired_at.time());
    UserStatusStore::new(server.app_state.db.clone())
        .upsert_custom_status(
            DbUserId::from_proto(client.user_id().unwrap()),
            Some("📅".to_string()),
            "Expired".to_string(),
            Some(expired_at),
        )
        .await
        .unwrap();

    let sweeper = StatusExpirySweeper::new(
        server.app_state.db.clone(),
        Executor::Deterministic(executor.clone()),
        rpc::Peer::new(0),
        Arc::new(parking_lot::Mutex::new(ConnectionPool::default())),
    );
    let expired_users = sweeper.sweep().await.unwrap();
    assert_eq!(
        expired_users,
        vec![DbUserId::from_proto(client.user_id().unwrap())]
    );
    assert!(
        UserStatusStore::new(server.app_state.db.clone())
            .get_custom_statuses(vec![DbUserId::from_proto(client.user_id().unwrap())])
            .await
            .unwrap()
            .is_empty()
    );
}

#[gpui::test]
async fn custom_status_expiry_sweeper_ignores_active_statuses(
    executor: BackgroundExecutor,
    cx: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client = server.create_client(cx, "status-owner").await;
    UserStatusStore::new(server.app_state.db.clone())
        .upsert_custom_status(
            DbUserId::from_proto(client.user_id().unwrap()),
            None,
            "Active".to_string(),
            Some(PrimitiveDateTime::new(
                (OffsetDateTime::now_utc() + TimeDuration::hours(1)).date(),
                (OffsetDateTime::now_utc() + TimeDuration::hours(1)).time(),
            )),
        )
        .await
        .unwrap();

    let sweeper = StatusExpirySweeper::new(
        server.app_state.db.clone(),
        Executor::Deterministic(executor.clone()),
        rpc::Peer::new(0),
        Arc::new(parking_lot::Mutex::new(ConnectionPool::default())),
    );
    assert!(sweeper.sweep().await.unwrap().is_empty());
}

#[gpui::test]
async fn custom_status_expiry_clears_connected_peers(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "status-owner").await;
    let client_b = server.create_client(cx_b, "status-observer").await;
    server
        .make_contacts(&mut [(&client_a, cx_a), (&client_b, cx_b)])
        .await;

    client_a
        .client()
        .request(proto::SetStatus {
            emoji: Some("📅".to_string()),
            text: "Temporary".to_string(),
            clear_after_minutes: None,
        })
        .await
        .unwrap();
    executor.run_until_parked();
    client_b.user_store().read_with(cx_b, |store, _| {
        assert!(store.custom_status_for_user(client_a.id()).is_some());
    });

    let expired_at = OffsetDateTime::now_utc() - TimeDuration::minutes(1);
    let expired_at = PrimitiveDateTime::new(expired_at.date(), expired_at.time());
    UserStatusStore::new(server.app_state.db.clone())
        .upsert_custom_status(
            DbUserId::from_proto(client_a.user_id().unwrap()),
            Some("📅".to_string()),
            "Temporary".to_string(),
            Some(expired_at),
        )
        .await
        .unwrap();

    server.sweep_expired_statuses().await;
    executor.run_until_parked();
    client_b.user_store().read_with(cx_b, |store, _| {
        assert!(store.custom_status_for_user(client_a.id()).is_none());
    });
}

#[gpui::test]
async fn custom_status_reconnect_syncs_persisted_contact_status(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    server
        .make_contacts(&mut [(&client_a, cx_a), (&client_b, cx_b)])
        .await;

    client_a
        .client()
        .request(proto::SetStatus {
            emoji: Some("📅".to_string()),
            text: "In a meeting".to_string(),
            clear_after_minutes: None,
        })
        .await
        .unwrap();
    executor.run_until_parked();

    server.disconnect_client(client_b.peer_id().unwrap());
    executor.advance_clock(RECEIVE_TIMEOUT + RECONNECT_TIMEOUT);
    executor.run_until_parked();

    client_b.user_store().read_with(cx_b, |store, _| {
        let status = store.custom_status_for_user(client_a.id()).unwrap();
        assert_eq!(status.text, "In a meeting");
        assert_eq!(status.emoji.as_deref(), Some("📅"));
    });
}

#[gpui::test]
async fn group_mentions_create_notifications_for_members_except_sender(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel(
            "mentions",
            None,
            (&client_a, cx_a),
            &mut [(&client_b, cx_b)],
        )
        .await;

    let group = client_a
        .create_group(
            "eng-team".to_string(),
            "Engineering".to_string(),
            vec![client_b.id()],
        )
        .await
        .unwrap();
    let message = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "@eng-team please review".to_string(),
            nonce: 1,
            mentions: vec![proto::ChatMention {
                range: Some(proto::Range { start: 0, end: 9 }),
                user_id: 0,
                group_id: group.id,
            }],
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();

    for _ in 0..5 {
        executor.run_until_parked();
    }
    client_b.notification_store().read_with(cx_b, |store, _| {
        let entry = (0..store.notification_count())
            .filter_map(|index| store.notification_at(index))
            .find(|entry| matches!(entry.notification, Notification::GroupMention { .. }))
            .unwrap();
        assert_eq!(
            entry.notification,
            Notification::GroupMention {
                message_id: message.id,
                channel_id: channel_id.0,
                sender_id: client_a.id(),
                group_id: group.id,
                message_preview: "@eng-team please review".to_string(),
            }
        );
    });
    client_a.notification_store().read_with(cx_a, |store, _| {
        assert_eq!(store.notification_count(), 0);
    });
}

#[gpui::test]
async fn group_rpc_lifecycle_updates_members_and_deletes_group(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
    cx_c: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let client_c = server.create_client(cx_c, "user_c").await;

    let group = client_a
        .create_group(
            "platform".to_string(),
            "Platform".to_string(),
            vec![client_b.id()],
        )
        .await
        .unwrap();
    let created = server
        .app_state
        .db
        .get_group(DbGroupId::from_proto(group.id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(created.member_ids.len(), 2);
    assert!(
        created
            .member_ids
            .contains(&DbUserId::from_proto(client_a.user_id().unwrap()))
    );
    assert!(
        created
            .member_ids
            .contains(&DbUserId::from_proto(client_b.user_id().unwrap()))
    );

    let updated = client_a
        .update_group_members(group.id, vec![client_c.id()], vec![client_b.id()])
        .await
        .unwrap();
    assert!(updated.member_ids.contains(&client_a.id()));
    assert!(updated.member_ids.contains(&client_c.id()));
    assert!(!updated.member_ids.contains(&client_b.id()));

    client_c.leave_group(group.id).await.unwrap();
    let after_leave = server
        .app_state
        .db
        .get_group(DbGroupId::from_proto(group.id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        after_leave.member_ids,
        vec![DbUserId::from_proto(client_a.user_id().unwrap())]
    );

    client_a.delete_group(group.id).await.unwrap();
    assert!(
        server
            .app_state
            .db
            .get_group(DbGroupId::from_proto(group.id))
            .await
            .unwrap()
            .is_none()
    );
}

#[gpui::test]
async fn group_mentions_fan_out_mixed_mentions_and_stop_after_leave(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
    cx_c: &mut TestAppContext,
    cx_d: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let client_c = server.create_client(cx_c, "user_c").await;
    let client_d = server.create_client(cx_d, "user_d").await;
    let mut channel_members = [
        (&client_b, &mut *cx_b),
        (&client_c, &mut *cx_c),
        (&client_d, &mut *cx_d),
    ];
    let channel_id = server
        .make_channel("mentions", None, (&client_a, cx_a), &mut channel_members)
        .await;

    let group = client_a
        .create_group(
            "eng-team".to_string(),
            "Engineering".to_string(),
            vec![client_b.id(), client_c.id(), client_d.id()],
        )
        .await
        .unwrap();
    let message = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "@eng-team and @user_c".to_string(),
            nonce: 1,
            mentions: vec![
                proto::ChatMention {
                    range: Some(proto::Range { start: 0, end: 9 }),
                    user_id: 0,
                    group_id: group.id,
                },
                mention_for(client_c.id(), 14, 21),
            ],
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();
    assert!(
        message
            .mentions
            .iter()
            .any(|mention| mention.group_id == group.id)
    );
    assert!(
        message
            .mentions
            .iter()
            .any(|mention| mention.user_id == client_c.id() && mention.group_id == 0)
    );

    for _ in 0..5 {
        executor.run_until_parked();
    }
    for client in [&client_b, &client_c, &client_d] {
        client.notification_store().read_with(
            match client.username.as_str() {
                "user_b" => cx_b,
                "user_c" => cx_c,
                _ => cx_d,
            },
            |store, _| {
                assert_eq!(
                    (0..store.notification_count())
                        .filter_map(|index| store.notification_at(index))
                        .filter(|entry| {
                            matches!(entry.notification, Notification::GroupMention { .. })
                        })
                        .count(),
                    1
                );
            },
        );
    }

    client_c.leave_group(group.id).await.unwrap();
    client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "@eng-team follow-up".to_string(),
            nonce: 2,
            mentions: vec![proto::ChatMention {
                range: Some(proto::Range { start: 0, end: 9 }),
                user_id: 0,
                group_id: group.id,
            }],
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();
    for _ in 0..5 {
        executor.run_until_parked();
    }
    client_c.notification_store().read_with(cx_c, |store, _| {
        assert_eq!(
            (0..store.notification_count())
                .filter_map(|index| store.notification_at(index))
                .filter(|entry| matches!(entry.notification, Notification::GroupMention { .. }))
                .count(),
            1
        );
    });
    for (client, cx) in [(&client_b, cx_b), (&client_d, cx_d)] {
        client.notification_store().read_with(cx, |store, _| {
            assert_eq!(
                (0..store.notification_count())
                    .filter_map(|index| store.notification_at(index))
                    .filter(|entry| {
                        matches!(entry.notification, Notification::GroupMention { .. })
                    })
                    .count(),
                2
            );
        });
    }
}

#[gpui::test]
async fn concurrent_group_membership_updates_retain_both_additions(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
    cx_c: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let client_c = server.create_client(cx_c, "user_c").await;
    let group = client_a
        .create_group("platform".to_string(), "Platform".to_string(), Vec::new())
        .await
        .unwrap();

    let (add_b, add_c) = futures::future::join(
        client_a.update_group_members(group.id, vec![client_b.id()], Vec::new()),
        client_a.update_group_members(group.id, vec![client_c.id()], Vec::new()),
    )
    .await;
    add_b.unwrap();
    add_c.unwrap();

    let updated = server
        .app_state
        .db
        .get_group(DbGroupId::from_proto(group.id))
        .await
        .unwrap()
        .unwrap();
    assert!(
        updated
            .member_ids
            .contains(&DbUserId::from_proto(client_a.user_id().unwrap()))
    );
    assert!(
        updated
            .member_ids
            .contains(&DbUserId::from_proto(client_b.user_id().unwrap()))
    );
    assert!(
        updated
            .member_ids
            .contains(&DbUserId::from_proto(client_c.user_id().unwrap()))
    );
}

#[gpui::test]
async fn channel_message_priority_persists_and_is_immutable(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    let sent = client_a
        .send_channel_message_with_priority(
            SendChannelMessage {
                channel_id: channel_id.0,
                body: "please review".to_string(),
                nonce: 1,
                mentions: Vec::new(),
                reply_to_message_id: None,
                file_ids: Vec::new(),
            },
            MessagePriority::Urgent,
        )
        .await
        .unwrap();
    assert_eq!(sent.priority, proto::ChannelMessagePriority::Urgent as i32);

    let retrieved = client_b
        .get_channel_messages_by_id(vec![sent.id])
        .await
        .unwrap()
        .messages;
    assert_eq!(
        retrieved[0].priority,
        proto::ChannelMessagePriority::Urgent as i32
    );

    client_a
        .update_channel_message(UpdateChannelMessage {
            channel_id: channel_id.0,
            message_id: sent.id,
            body: "please review the updated document".to_string(),
            nonce: 2,
            mentions: Vec::new(),
        })
        .await
        .unwrap();

    let updated = client_b
        .get_channel_messages_by_id(vec![sent.id])
        .await
        .unwrap()
        .messages;
    assert_eq!(
        updated[0].priority,
        proto::ChannelMessagePriority::Urgent as i32
    );
}

#[gpui::test]
async fn test_channel_file_upload_lifecycle_rpc(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a.join_channel_chat(channel_id.0).await.unwrap();
    client_b.join_channel_chat(channel_id.0).await.unwrap();

    let too_large = client_a
        .client()
        .get_file_upload_url(GetFileUploadUrl {
            channel_id,
            filename: "too-large.txt".to_string(),
            file_size: 101 * 1024 * 1024,
            mime_type: "text/plain".to_string(),
        })
        .await
        .unwrap_err();
    assert_eq!(too_large.error_code(), proto::ErrorCode::FileTooLarge);

    let missing_file = client_a
        .client()
        .confirm_file_upload(Uuid::new_v4().to_string())
        .await
        .unwrap_err();
    assert_eq!(missing_file.error_code(), proto::ErrorCode::Internal);

    let upload = client_a
        .client()
        .get_file_upload_url(GetFileUploadUrl {
            channel_id,
            filename: "deploy.txt".to_string(),
            file_size: 12,
            mime_type: "text/plain".to_string(),
        })
        .await
        .unwrap();
    assert!(upload.url.contains("file-store.test"));
    assert!(upload.headers.is_empty());

    let confirmed = client_a
        .client()
        .confirm_file_upload(upload.file_id.clone())
        .await
        .unwrap();
    assert_eq!(confirmed.id, upload.file_id);
    assert_eq!(confirmed.filename, "deploy.txt");
    assert_eq!(confirmed.file_size, 12);
    assert_eq!(confirmed.mime_type, "text/plain");
    assert_eq!(confirmed.uploader_id, client_a.user_id().unwrap());
    assert!(confirmed.uploaded_at.is_some());

    let sent = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "see attachment".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: vec![upload.file_id.clone()],
        })
        .await
        .unwrap();
    assert_eq!(sent.files.len(), 1);
    assert_eq!(sent.files[0].id, upload.file_id);
    assert_eq!(sent.files[0].filename, "deploy.txt");

    let history = client_b
        .get_channel_messages(channel_id.0, None)
        .await
        .unwrap()
        .messages;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, sent.id);
    assert_eq!(history[0].files.len(), 1);
    assert_eq!(history[0].files[0].id, upload.file_id);

    let file_id = Uuid::parse_str(&upload.file_id).unwrap();
    client_a
        .remove_channel_message(channel_id.0, sent.id)
        .await
        .unwrap();
    let stored_file = server
        .app_state
        .db
        .transaction(|tx| async move {
            channel_file::Entity::find_by_id(file_id)
                .one(&*tx)
                .await
                .map_err(Into::into)
        })
        .await
        .unwrap();
    assert!(stored_file.is_none());
}

#[gpui::test]
async fn test_custom_status_rpc_validation_and_clear_idempotency(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let _client_b = server.create_client(cx_b, "user_b").await;

    let too_long = client_a
        .client()
        .request(proto::SetStatus {
            emoji: None,
            text: "a".repeat(101),
            clear_after_minutes: None,
        })
        .await
        .unwrap_err();
    assert!(too_long.to_string().contains("between 1 and 100"));

    let invalid_emoji = client_a
        .client()
        .request(proto::SetStatus {
            emoji: Some("not-an-emoji".to_string()),
            text: "Available".to_string(),
            clear_after_minutes: None,
        })
        .await
        .unwrap_err();
    assert!(
        invalid_emoji
            .to_string()
            .contains("emoji is not recognized")
    );

    let invalid_duration = client_a
        .client()
        .request(proto::SetStatus {
            emoji: Some("📅".to_string()),
            text: "In a meeting".to_string(),
            clear_after_minutes: Some(31),
        })
        .await
        .unwrap_err();
    assert!(
        invalid_duration
            .to_string()
            .contains("unsupported status clear-after duration")
    );

    client_a
        .client()
        .request(proto::SetStatus {
            emoji: Some("📅".to_string()),
            text: "  In a meeting  ".to_string(),
            clear_after_minutes: Some(30),
        })
        .await
        .unwrap();

    client_a
        .client()
        .request(proto::ClearStatus {})
        .await
        .unwrap();
    client_a
        .client()
        .request(proto::ClearStatus {})
        .await
        .unwrap();
}

#[gpui::test]
async fn custom_status_broadcasts_set_and_clear_to_multiple_clients(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
    cx_c: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let client_c = server.create_client(cx_c, "user_c").await;
    let mut clients = [
        (&client_a, &mut *cx_a),
        (&client_b, &mut *cx_b),
        (&client_c, &mut *cx_c),
    ];
    server.make_contacts(&mut clients).await;

    client_a
        .client()
        .request(proto::SetStatus {
            emoji: Some("📅".to_string()),
            text: "In a meeting".to_string(),
            clear_after_minutes: None,
        })
        .await
        .unwrap();
    for _ in 0..3 {
        executor.run_until_parked();
    }
    for (client, cx) in [(&client_b, &mut *cx_b), (&client_c, &mut *cx_c)] {
        client.user_store().read_with(cx, |store, _| {
            let status = store.custom_status_for_user(client_a.id()).unwrap();
            assert_eq!(status.text, "In a meeting");
            assert_eq!(status.emoji.as_deref(), Some("📅"));
        });
    }

    client_a
        .client()
        .request(proto::ClearStatus {})
        .await
        .unwrap();
    for _ in 0..3 {
        executor.run_until_parked();
    }
    for (client, cx) in [(&client_b, &mut *cx_b), (&client_c, &mut *cx_c)] {
        client.user_store().read_with(cx, |store, _| {
            assert!(store.custom_status_for_user(client_a.id()).is_none());
        });
    }
}

#[gpui::test]
async fn test_channel_bookmark_rpc_flow_and_permissions(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
    cx_c: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let client_c = server.create_client(cx_c, "user_c").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;
    client_a
        .channel_store()
        .update(cx_a, |channel_store, cx| {
            channel_store.set_channel_visibility(channel_id, proto::ChannelVisibility::Public, cx)
        })
        .await
        .unwrap();
    let db_channel_id = DbChannelId::from_proto(channel_id.0);
    let guest_id = DbUserId::from_proto(client_c.user_id().unwrap());
    let admin_id = DbUserId::from_proto(client_a.user_id().unwrap());
    server
        .app_state
        .db
        .invite_channel_member(db_channel_id, guest_id, admin_id, DbChannelRole::Guest)
        .await
        .unwrap();
    server
        .app_state
        .db
        .respond_to_channel_invite(db_channel_id, guest_id, true)
        .await
        .unwrap();

    client_a.join_channel_chat(channel_id.0).await.unwrap();
    client_b.join_channel_chat(channel_id.0).await.unwrap();

    let handler_entity = cx_b.new(|_| BookmarkHandlerEntity);
    let (bookmark_tx, bookmark_rx) = async_channel::bounded(8);
    let _bookmark_subscription = client_b.add_channel_bookmarks_update_handler(
        handler_entity.downgrade(),
        move |_, update, _| {
            bookmark_tx.try_send(update.payload).unwrap();
            async { Ok(()) }
        },
    );
    let message_handler_entity = cx_b.new(|_| ChannelMessageHandlerEntity);
    let (message_tx, message_rx) = async_channel::bounded(8);
    let _message_subscription = client_b.add_channel_message_sent_handler(
        message_handler_entity.downgrade(),
        move |_, message, _| {
            message_tx.try_send(message.payload).unwrap();
            async { Ok(()) }
        },
    );

    client_a
        .client()
        .request(proto::AddBookmark {
            channel_id: channel_id.0,
            label: "Deploy Guide".to_string(),
            r#type: proto::BookmarkType::BookmarkLink as i32,
            url: "https://sim.dev/deploy".to_string(),
            file_id: None,
            message_id: None,
            description: Some("How to deploy".to_string()),
        })
        .await
        .unwrap();
    let update = bookmark_rx.recv().await.unwrap();
    assert_eq!(update.channel_id, channel_id.0);
    assert_eq!(update.bookmarks.len(), 1);
    assert_eq!(update.bookmarks[0].label, "Deploy Guide");
    assert_eq!(update.removed_bookmark_ids, Vec::<u64>::new());
    let bookmark_id = update.bookmarks[0].id;
    let message = message_rx.recv().await.unwrap();
    assert_eq!(message.channel_id, channel_id.0);
    assert_eq!(
        message.message.as_ref().unwrap().body,
        "Pinned a link bookmark: Deploy Guide"
    );

    client_b
        .client()
        .request(proto::UpdateBookmark {
            channel_id: channel_id.0,
            bookmark_id,
            label: "Deploy Guide v2".to_string(),
            description: Some("Updated".to_string()),
        })
        .await
        .unwrap();
    let update = bookmark_rx.recv().await.unwrap();
    assert_eq!(update.bookmarks[0].label, "Deploy Guide v2");
    let message = message_rx.recv().await.unwrap();
    assert_eq!(
        message.message.as_ref().unwrap().body,
        "Updated bookmark: Deploy Guide v2"
    );

    client_a
        .client()
        .request(proto::AddBookmark {
            channel_id: channel_id.0,
            label: "Runbook".to_string(),
            r#type: proto::BookmarkType::BookmarkLink as i32,
            url: "https://sim.dev/runbook".to_string(),
            file_id: None,
            message_id: None,
            description: None,
        })
        .await
        .unwrap();
    let update = bookmark_rx.recv().await.unwrap();
    assert_eq!(update.bookmarks.len(), 2);
    let message = message_rx.recv().await.unwrap();
    assert_eq!(
        message.message.as_ref().unwrap().body,
        "Pinned a link bookmark: Runbook"
    );
    let second_bookmark_id = update
        .bookmarks
        .iter()
        .find(|bookmark| bookmark.label == "Runbook")
        .unwrap()
        .id;

    client_a
        .client()
        .request(proto::ReorderBookmarks {
            channel_id: channel_id.0,
            bookmark_ids: vec![second_bookmark_id, bookmark_id],
        })
        .await
        .unwrap();
    client_a
        .client()
        .request(proto::ReorderBookmarks {
            channel_id: channel_id.0,
            bookmark_ids: vec![bookmark_id, second_bookmark_id],
        })
        .await
        .unwrap();
    executor.run_until_parked();
    assert!(bookmark_rx.try_recv().is_err());
    executor.advance_clock(StdDuration::from_millis(200));
    executor.run_until_parked();
    let update = bookmark_rx.recv().await.unwrap();
    assert_eq!(
        update
            .bookmarks
            .iter()
            .map(|bookmark| bookmark.id)
            .collect::<Vec<_>>(),
        vec![bookmark_id, second_bookmark_id]
    );
    assert!(bookmark_rx.try_recv().is_err());
    let fetched_bookmarks = client_b
        .get_bookmarks(client::ChannelId(channel_id.0))
        .await
        .unwrap();
    assert_eq!(
        fetched_bookmarks
            .iter()
            .map(|bookmark| bookmark.id.to_proto())
            .collect::<Vec<_>>(),
        vec![bookmark_id, second_bookmark_id]
    );
    let guest_bookmarks = client_c
        .get_bookmarks(client::ChannelId(channel_id.0))
        .await
        .unwrap();
    assert_eq!(guest_bookmarks.len(), 2);

    let guest_result = client_c
        .client()
        .request(proto::AddBookmark {
            channel_id: channel_id.0,
            label: "Guest Link".to_string(),
            r#type: proto::BookmarkType::BookmarkLink as i32,
            url: "https://sim.dev/guest".to_string(),
            file_id: None,
            message_id: None,
            description: None,
        })
        .await;
    assert!(guest_result.is_err());
    let guest_result = client_c
        .client()
        .request(proto::UpdateBookmark {
            channel_id: channel_id.0,
            bookmark_id,
            label: "Guest Update".to_string(),
            description: None,
        })
        .await;
    assert!(guest_result.is_err());
    let guest_result = client_c
        .client()
        .request(proto::ReorderBookmarks {
            channel_id: channel_id.0,
            bookmark_ids: vec![second_bookmark_id, bookmark_id],
        })
        .await;
    assert!(guest_result.is_err());
    let guest_result = client_c
        .client()
        .request(proto::RemoveBookmark {
            channel_id: channel_id.0,
            bookmark_id,
        })
        .await;
    assert!(guest_result.is_err());

    let first_concurrent_order = vec![second_bookmark_id, bookmark_id];
    let second_concurrent_order = vec![bookmark_id, second_bookmark_id];
    let (first_result, second_result) = futures::join!(
        client_a.client().request(proto::ReorderBookmarks {
            channel_id: channel_id.0,
            bookmark_ids: first_concurrent_order.clone(),
        }),
        client_b.client().request(proto::ReorderBookmarks {
            channel_id: channel_id.0,
            bookmark_ids: second_concurrent_order.clone(),
        })
    );
    first_result.unwrap();
    second_result.unwrap();
    executor.advance_clock(StdDuration::from_millis(200));
    executor.run_until_parked();
    let update = bookmark_rx.recv().await.unwrap();
    let concurrent_order = update
        .bookmarks
        .iter()
        .map(|bookmark| bookmark.id)
        .collect::<Vec<_>>();
    assert!(
        concurrent_order == first_concurrent_order || concurrent_order == second_concurrent_order
    );

    client_a
        .client()
        .request(proto::RemoveBookmark {
            channel_id: channel_id.0,
            bookmark_id,
        })
        .await
        .unwrap();
    let update = bookmark_rx.recv().await.unwrap();
    assert_eq!(update.bookmarks.len(), 1);
    assert_eq!(update.removed_bookmark_ids, vec![bookmark_id]);
    let message = message_rx.recv().await.unwrap();
    assert_eq!(
        message.message.as_ref().unwrap().body,
        "Removed bookmark: Deploy Guide v2"
    );
}

#[gpui::test]
async fn test_scheduled_channel_message_delivers(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();
    client_b
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();

    let scheduled_message_id = client_a
        .client()
        .schedule_channel_message(ScheduleChannelMessage {
            channel_id: channel_id.0,
            body: "scheduled hello".to_string(),
            scheduled_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            nonce: 10,
            mentions: Vec::new(),
        })
        .await
        .unwrap();
    let store = ScheduledMessageStore::new(server.app_state.db.clone());
    store
        .set_scheduled_at_for_test(
            DbScheduledMessageId::from_proto(scheduled_message_id.to_proto()),
            primitive_datetime_in(TimeDuration::minutes(-1)),
        )
        .await
        .unwrap();

    executor.advance_clock(StdDuration::from_secs(10));
    executor.run_until_parked();

    let history = client_b
        .client()
        .get_channel_messages(channel_id.0, None)
        .await
        .unwrap()
        .messages;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].body, "scheduled hello");
    assert!(history[0].scheduled_at.is_some());
    assert!(
        client_a
            .client()
            .get_scheduled_messages(channel_id.0)
            .await
            .unwrap()
            .is_empty()
    );
}

#[gpui::test]
async fn test_cancelled_scheduled_channel_message_does_not_deliver(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();
    client_b
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();

    let scheduled_message_id = client_a
        .client()
        .schedule_channel_message(ScheduleChannelMessage {
            channel_id: channel_id.0,
            body: "cancelled hello".to_string(),
            scheduled_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            nonce: 11,
            mentions: Vec::new(),
        })
        .await
        .unwrap();
    client_a
        .client()
        .cancel_scheduled_message(channel_id.0, scheduled_message_id)
        .await
        .unwrap();

    executor.advance_clock(StdDuration::from_secs(10));
    executor.run_until_parked();

    let history = client_b
        .client()
        .get_channel_messages(channel_id.0, None)
        .await
        .unwrap()
        .messages;
    assert!(history.is_empty());
    assert!(
        client_a
            .client()
            .get_scheduled_messages(channel_id.0)
            .await
            .unwrap()
            .is_empty()
    );
}

#[gpui::test]
async fn test_scheduled_channel_message_failure_after_sender_removed(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_b, cx_b), &mut [(&client_a, cx_a)])
        .await;

    client_a
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();
    client_b
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();

    let handler_entity = cx_a.new(|_| ScheduledMessageHandlerEntity);
    let (failure_tx, failure_rx) = async_channel::bounded(1);
    let _failure_subscription = client_a.client().add_scheduled_message_failed_handler(
        handler_entity.downgrade(),
        move |_, failure, _| {
            failure_tx.try_send(failure.payload).unwrap();
            async { Ok(()) }
        },
    );

    let scheduled_message_id = client_a
        .client()
        .schedule_channel_message(ScheduleChannelMessage {
            channel_id: channel_id.0,
            body: "removed sender".to_string(),
            scheduled_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            nonce: 12,
            mentions: Vec::new(),
        })
        .await
        .unwrap();
    let store = ScheduledMessageStore::new(server.app_state.db.clone());
    store
        .set_scheduled_at_for_test(
            DbScheduledMessageId::from_proto(scheduled_message_id.to_proto()),
            primitive_datetime_in(TimeDuration::minutes(-1)),
        )
        .await
        .unwrap();

    client_b
        .channel_store()
        .update(cx_b, |channel_store, cx| {
            channel_store.remove_member(channel_id, client_a.user_id().unwrap(), cx)
        })
        .await
        .unwrap();
    executor.run_until_parked();

    executor.advance_clock(StdDuration::from_secs(10));
    executor.run_until_parked();

    let failure = failure_rx.recv().await.unwrap();
    assert_eq!(
        failure.scheduled_message_id,
        scheduled_message_id.to_proto()
    );
    assert_eq!(failure.channel_id, channel_id.0);
    assert!(
        failure.reason.contains("not a channel participant"),
        "unexpected failure reason: {}",
        failure.reason
    );
    assert!(
        client_a
            .client()
            .get_scheduled_messages(channel_id.0)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        client_b
            .client()
            .get_channel_messages(channel_id.0, None)
            .await
            .unwrap()
            .messages
            .is_empty()
    );
}

#[gpui::test]
async fn test_scheduled_channel_messages_due_at_same_time_deliver_in_order(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();
    client_b
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();

    let store = ScheduledMessageStore::new(server.app_state.db.clone());
    let due_at = primitive_datetime_in(TimeDuration::minutes(-1));
    for (nonce, body) in [(20, "first due"), (21, "second due"), (22, "third due")] {
        let scheduled_message_id = client_a
            .client()
            .schedule_channel_message(ScheduleChannelMessage {
                channel_id: channel_id.0,
                body: body.to_string(),
                scheduled_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                nonce,
                mentions: Vec::new(),
            })
            .await
            .unwrap();
        store
            .set_scheduled_at_for_test(
                DbScheduledMessageId::from_proto(scheduled_message_id.to_proto()),
                due_at,
            )
            .await
            .unwrap();
    }

    executor.advance_clock(StdDuration::from_secs(10));
    executor.run_until_parked();

    let history = client_b
        .client()
        .get_channel_messages(channel_id.0, None)
        .await
        .unwrap()
        .messages;
    assert_eq!(
        history
            .iter()
            .map(|message| message.body.as_str())
            .collect::<Vec<_>>(),
        vec!["first due", "second due", "third due"]
    );
    assert!(
        client_a
            .client()
            .get_scheduled_messages(channel_id.0)
            .await
            .unwrap()
            .is_empty()
    );
}

#[gpui::test]
async fn test_stale_processing_scheduled_message_is_reset_and_delivered_after_restart(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();
    client_b
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();

    let scheduled_message_id = client_a
        .client()
        .schedule_channel_message(ScheduleChannelMessage {
            channel_id: channel_id.0,
            body: "stale processing".to_string(),
            scheduled_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            nonce: 23,
            mentions: Vec::new(),
        })
        .await
        .unwrap();
    let store = ScheduledMessageStore::new(server.app_state.db.clone());
    let db_scheduled_message_id = DbScheduledMessageId::from_proto(scheduled_message_id.to_proto());
    store
        .set_scheduled_at_for_test(
            db_scheduled_message_id,
            primitive_datetime_in(TimeDuration::minutes(-1)),
        )
        .await
        .unwrap();
    store
        .set_state_for_test(db_scheduled_message_id, 1)
        .await
        .unwrap();
    store
        .set_updated_at_for_test(
            db_scheduled_message_id,
            primitive_datetime_in(TimeDuration::minutes(-2)),
        )
        .await
        .unwrap();

    let epoch = server
        .app_state
        .db
        .create_server(&server.app_state.config.sim_environment)
        .await
        .unwrap();
    let restarted_server = Server::new(epoch, server.app_state.clone());
    restarted_server.start().await.unwrap();
    executor.run_until_parked();

    executor.advance_clock(StdDuration::from_secs(10));
    executor.run_until_parked();

    let history = client_b
        .client()
        .get_channel_messages(channel_id.0, None)
        .await
        .unwrap()
        .messages;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].body, "stale processing");
    assert!(
        client_a
            .client()
            .get_scheduled_messages(channel_id.0)
            .await
            .unwrap()
            .is_empty()
    );
}

#[gpui::test]
async fn test_concurrent_scheduled_message_cancel_and_delivery_is_at_most_once(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();
    client_b
        .client()
        .join_channel_chat(channel_id.0)
        .await
        .unwrap();

    let scheduled_message_id = client_a
        .client()
        .schedule_channel_message(ScheduleChannelMessage {
            channel_id: channel_id.0,
            body: "raced delivery".to_string(),
            scheduled_at: chrono::Utc::now() + chrono::Duration::minutes(5),
            nonce: 24,
            mentions: Vec::new(),
        })
        .await
        .unwrap();
    let store = ScheduledMessageStore::new(server.app_state.db.clone());
    store
        .set_scheduled_at_for_test(
            DbScheduledMessageId::from_proto(scheduled_message_id.to_proto()),
            primitive_datetime_in(TimeDuration::minutes(-1)),
        )
        .await
        .unwrap();

    let cancel = client_a
        .client()
        .cancel_scheduled_message(channel_id.0, scheduled_message_id);
    let deliver = async {
        executor.advance_clock(StdDuration::from_secs(10));
        executor.run_until_parked();
    };
    let (cancel_result, ()) = futures::future::join(cancel, deliver).await;
    cancel_result.unwrap();

    let history = client_b
        .client()
        .get_channel_messages(channel_id.0, None)
        .await
        .unwrap()
        .messages;
    assert!(history.len() <= 1);
    if let Some(message) = history.first() {
        assert_eq!(message.body, "raced delivery");
    }
    assert!(
        client_a
            .client()
            .get_scheduled_messages(channel_id.0)
            .await
            .unwrap()
            .is_empty()
    );
}

#[gpui::test]
async fn test_channel_chat_thread_queries(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a.join_channel_chat(channel_id.0).await.unwrap();
    client_b.join_channel_chat(channel_id.0).await.unwrap();

    assert!(client_a.get_threads(channel_id.0).await.unwrap().is_empty());

    let root = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "root".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();
    let reply_a = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "reply from a".to_string(),
            nonce: 2,
            mentions: Vec::new(),
            reply_to_message_id: Some(root.id),
            file_ids: Vec::new(),
        })
        .await
        .unwrap();
    let reply_b = client_b
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "reply from b".to_string(),
            nonce: 3,
            mentions: Vec::new(),
            reply_to_message_id: Some(root.id),
            file_ids: Vec::new(),
        })
        .await
        .unwrap();

    let thread = client_b.get_thread(channel_id.0, root.id).await.unwrap();
    assert_eq!(thread.root_message.id, root.id);
    assert_eq!(
        thread
            .replies
            .iter()
            .map(|message| (
                message.id,
                message.body.as_str(),
                message.reply_to_message_id
            ))
            .collect::<Vec<_>>(),
        vec![
            (reply_a.id, "reply from a", Some(root.id)),
            (reply_b.id, "reply from b", Some(root.id)),
        ]
    );

    let summaries = client_a.get_threads(channel_id.0).await.unwrap();
    assert_eq!(summaries.len(), 1);
    let summary = summaries.first().expect("missing thread summary");
    assert_eq!(summary.root_message_id, root.id);
    assert_eq!(summary.reply_count, 2);
    assert_eq!(summary.latest_reply_at, reply_b.timestamp);
    assert!(summary.has_unread);
    let mut participant_user_ids = summary.participant_user_ids.clone();
    participant_user_ids.sort_unstable();
    let mut expected_user_ids = vec![client_a.user_id().unwrap(), client_b.user_id().unwrap()];
    expected_user_ids.sort_unstable();
    assert_eq!(participant_user_ids, expected_user_ids);

    client_a
        .acknowledge_channel_thread(channel_id.0, root.id, reply_b.id)
        .unwrap();
    executor.run_until_parked();
    let summaries = client_a.get_threads(channel_id.0).await.unwrap();
    let summary = summaries.first().expect("missing thread summary");
    assert!(!summary.has_unread);

    assert!(
        client_a
            .get_thread(channel_id.0, root.id + 10_000)
            .await
            .is_err()
    );
    assert!(
        client_a
            .send_channel_message(SendChannelMessage {
                channel_id: channel_id.0,
                body: "missing root".to_string(),
                nonce: 4,
                mentions: Vec::new(),
                reply_to_message_id: Some(root.id + 10_000),
                file_ids: Vec::new(),
            })
            .await
            .is_err()
    );
}

#[gpui::test]
async fn test_channel_chat_thread_pagination(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a.join_channel_chat(channel_id.0).await.unwrap();
    client_b.join_channel_chat(channel_id.0).await.unwrap();

    let root = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "root".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();

    let mut replies = Vec::new();
    for index in 0..55 {
        replies.push(
            client_a
                .send_channel_message(SendChannelMessage {
                    channel_id: channel_id.0,
                    body: format!("reply {index}"),
                    nonce: 2 + index,
                    mentions: Vec::new(),
                    reply_to_message_id: Some(root.id),
                    file_ids: Vec::new(),
                })
                .await
                .unwrap(),
        );
    }

    let latest_page = client_b.get_thread(channel_id.0, root.id).await.unwrap();
    assert_eq!(latest_page.root_message.id, root.id);
    assert_eq!(
        latest_page.replies.len(),
        DEFAULT_THREAD_REPLY_LIMIT as usize
    );
    assert!(!latest_page.done);
    assert_eq!(latest_page.replies.first().unwrap().body, "reply 5");
    assert_eq!(latest_page.replies.last().unwrap().body, "reply 54");

    let older_page = client_b
        .get_thread_page(
            channel_id.0,
            root.id,
            latest_page.replies.first().map(|reply| reply.id),
            DEFAULT_THREAD_REPLY_LIMIT,
        )
        .await
        .unwrap();
    assert!(older_page.done);
    assert_eq!(
        older_page
            .replies
            .iter()
            .map(|reply| reply.body.as_str())
            .collect::<Vec<_>>(),
        replies
            .iter()
            .take(5)
            .map(|reply| reply.body.as_str())
            .collect::<Vec<_>>()
    );
}

#[gpui::test]
async fn test_channel_chat_reactions_flow(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a.join_channel_chat(channel_id.0).await.unwrap();
    client_b.join_channel_chat(channel_id.0).await.unwrap();

    let handler_entity = cx_b.new(|_| ReactionHandlerEntity);
    let (reaction_tx, reaction_rx) = async_channel::bounded(4);
    let _reaction_subscription = client_b.add_channel_message_reactions_update_handler(
        handler_entity.downgrade(),
        move |_, update, _| {
            reaction_tx.try_send(update.payload).unwrap();
            async { Ok(()) }
        },
    );

    let sent = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "hello from a".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();

    let reactions = client_b
        .add_channel_message_reaction(channel_id.0, sent.id, "thumbs_up".to_string())
        .await
        .unwrap();
    assert_reactions(&reactions, &[("thumbs_up", &[client_b.user_id().unwrap()])]);

    let update = reaction_rx.recv().await.unwrap();
    assert_eq!(update.channel_id, channel_id.0);
    assert_eq!(update.message_id, sent.id);
    assert_reactions(
        &update.reactions,
        &[("thumbs_up", &[client_b.user_id().unwrap()])],
    );

    let duplicate = client_b
        .add_channel_message_reaction(channel_id.0, sent.id, "thumbs_up".to_string())
        .await
        .unwrap();
    assert_reactions(&duplicate, &[("thumbs_up", &[client_b.user_id().unwrap()])]);

    let persisted = client_a
        .get_channel_messages_by_id(vec![sent.id])
        .await
        .unwrap()
        .messages;
    assert_eq!(persisted.len(), 1);
    assert_reactions(
        &persisted[0].reaction_summaries,
        &[("thumbs_up", &[client_b.user_id().unwrap()])],
    );

    let removed_missing = client_b
        .remove_channel_message_reaction(channel_id.0, sent.id, "heart".to_string())
        .await
        .unwrap();
    assert_reactions(
        &removed_missing,
        &[("thumbs_up", &[client_b.user_id().unwrap()])],
    );

    let removed = client_b
        .remove_channel_message_reaction(channel_id.0, sent.id, "thumbs_up".to_string())
        .await
        .unwrap();
    assert!(removed.is_empty());

    let mut cleared_update = None;
    for _ in 0..3 {
        let update = reaction_rx.recv().await.unwrap();
        if update.reactions.is_empty() {
            cleared_update = Some(update);
            break;
        }
    }
    let cleared_update = cleared_update.expect("missing cleared reaction update");
    assert_eq!(cleared_update.channel_id, channel_id.0);
    assert_eq!(cleared_update.message_id, sent.id);
}

#[gpui::test]
async fn test_channel_chat_reactions_multi_client_updates_and_reconnect(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a.join_channel_chat(channel_id.0).await.unwrap();
    client_b.join_channel_chat(channel_id.0).await.unwrap();

    let handler_entity_a = cx_a.new(|_| ReactionHandlerEntity);
    let (reaction_tx_a, reaction_rx_a) = async_channel::bounded(4);
    let _reaction_subscription_a = client_a.add_channel_message_reactions_update_handler(
        handler_entity_a.downgrade(),
        move |_, update, _| {
            reaction_tx_a.try_send(update.payload).unwrap();
            async { Ok(()) }
        },
    );

    let handler_entity_b = cx_b.new(|_| ReactionHandlerEntity);
    let (reaction_tx_b, reaction_rx_b) = async_channel::bounded(4);
    let _reaction_subscription_b = client_b.add_channel_message_reactions_update_handler(
        handler_entity_b.downgrade(),
        move |_, update, _| {
            reaction_tx_b.try_send(update.payload).unwrap();
            async { Ok(()) }
        },
    );

    let sent = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "hello from a".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();

    client_b
        .add_channel_message_reaction(channel_id.0, sent.id, "thumbs_up".to_string())
        .await
        .unwrap();
    let update_a = reaction_rx_a.recv().await.unwrap();
    assert_eq!(update_a.channel_id, channel_id.0);
    assert_eq!(update_a.message_id, sent.id);
    assert_reactions(
        &update_a.reactions,
        &[("thumbs_up", &[client_b.user_id().unwrap()])],
    );
    let update_b = reaction_rx_b.recv().await.unwrap();
    assert_eq!(update_b.channel_id, channel_id.0);
    assert_eq!(update_b.message_id, sent.id);
    assert_reactions(
        &update_b.reactions,
        &[("thumbs_up", &[client_b.user_id().unwrap()])],
    );

    client_a
        .add_channel_message_reaction(channel_id.0, sent.id, "thumbs_up".to_string())
        .await
        .unwrap();
    let update_b = reaction_rx_b.recv().await.unwrap();
    assert_eq!(update_b.channel_id, channel_id.0);
    assert_eq!(update_b.message_id, sent.id);
    assert_reactions(
        &update_b.reactions,
        &[(
            "thumbs_up",
            &[client_a.user_id().unwrap(), client_b.user_id().unwrap()],
        )],
    );

    client_a
        .remove_channel_message_reaction(channel_id.0, sent.id, "thumbs_up".to_string())
        .await
        .unwrap();
    let mut removed_update_b = None;
    for _ in 0..4 {
        let update = reaction_rx_b.recv().await.unwrap();
        if update.reactions.len() == 1
            && update.reactions[0].emoji_name == "thumbs_up"
            && update.reactions[0].user_ids == vec![client_b.user_id().unwrap()]
        {
            removed_update_b = Some(update);
            break;
        }
    }
    let removed_update_b = removed_update_b.expect("missing removed reaction update for client b");
    assert_eq!(removed_update_b.channel_id, channel_id.0);
    assert_eq!(removed_update_b.message_id, sent.id);
    assert_reactions(
        &removed_update_b.reactions,
        &[("thumbs_up", &[client_b.user_id().unwrap()])],
    );

    client_a.disconnect(&cx_a.to_async());
    client_a
        .connect(false, &cx_a.to_async())
        .await
        .into_response()
        .unwrap();
    let rejoined = client_a.join_channel_chat(channel_id.0).await.unwrap();
    assert_eq!(rejoined.messages.len(), 1);
    assert_eq!(rejoined.messages[0].id, sent.id);
    assert_reactions(
        &rejoined.messages[0].reaction_summaries,
        &[("thumbs_up", &[client_b.user_id().unwrap()])],
    );
}

#[gpui::test]
async fn test_channel_chat_reactions_reject_private_channel_access(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("private-chat", None, (&client_a, cx_a), &mut [])
        .await;

    client_a.join_channel_chat(channel_id.0).await.unwrap();
    let sent = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "private".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();

    assert!(
        client_b
            .add_channel_message_reaction(channel_id.0, sent.id, "thumbs_up".to_string())
            .await
            .is_err()
    );
    assert!(
        client_b
            .remove_channel_message_reaction(channel_id.0, sent.id, "thumbs_up".to_string())
            .await
            .is_err()
    );
}

#[gpui::test]
async fn test_channel_chat_message_delete_clears_reactions(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a.join_channel_chat(channel_id.0).await.unwrap();
    client_b.join_channel_chat(channel_id.0).await.unwrap();

    let handler_entity = cx_b.new(|_| ReactionHandlerEntity);
    let (reaction_tx, reaction_rx) = async_channel::bounded(4);
    let _reaction_subscription = client_b.add_channel_message_reactions_update_handler(
        handler_entity.downgrade(),
        move |_, update, _| {
            reaction_tx.try_send(update.payload).unwrap();
            async { Ok(()) }
        },
    );

    let sent = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "hello from a".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();

    client_b
        .add_channel_message_reaction(channel_id.0, sent.id, "thumbs_up".to_string())
        .await
        .unwrap();
    let added_update = reaction_rx.recv().await.unwrap();
    assert_reactions(
        &added_update.reactions,
        &[("thumbs_up", &[client_b.user_id().unwrap()])],
    );

    client_a
        .remove_channel_message(channel_id.0, sent.id)
        .await
        .unwrap();
    let cleared_update = reaction_rx.recv().await.unwrap();
    assert_eq!(cleared_update.channel_id, channel_id.0);
    assert_eq!(cleared_update.message_id, sent.id);
    assert!(cleared_update.reactions.is_empty());

    let deleted = client_b
        .get_channel_messages_by_id(vec![sent.id])
        .await
        .unwrap()
        .messages;
    assert_eq!(deleted.len(), 1);
    assert!(deleted[0].reaction_summaries.is_empty());
}

#[gpui::test]
async fn test_channel_chat_rejects_private_channel_access(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("private-chat", None, (&client_a, cx_a), &mut [])
        .await;

    assert!(client_b.join_channel_chat(channel_id.0).await.is_err());
    assert!(
        client_b
            .send_channel_message(SendChannelMessage {
                channel_id: channel_id.0,
                body: "nope".to_string(),
                nonce: 1,
                mentions: Vec::new(),
                reply_to_message_id: None,
                file_ids: Vec::new(),
            })
            .await
            .is_err()
    );
    assert!(
        client_b
            .get_channel_messages(channel_id.0, None)
            .await
            .is_err()
    );
}

#[gpui::test]
async fn test_channel_message_search_filters_and_access(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
    cx_c: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let client_c = server.create_client(cx_c, "user_c").await;
    let general = server
        .make_channel("general", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;
    let private = server
        .make_channel("private", None, (&client_a, cx_a), &mut [])
        .await;

    client_a.join_channel_chat(general.0).await.unwrap();
    client_b.join_channel_chat(general.0).await.unwrap();
    client_a.join_channel_chat(private.0).await.unwrap();

    client_a
        .send_channel_message(SendChannelMessage {
            channel_id: general.0,
            body: "deploy alpha to staging".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();
    client_b
        .send_channel_message(SendChannelMessage {
            channel_id: general.0,
            body: "deploy beta after review".to_string(),
            nonce: 2,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();
    client_a
        .send_channel_message(SendChannelMessage {
            channel_id: private.0,
            body: "deploy private secret".to_string(),
            nonce: 3,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();

    let response = client_b
        .search_channel_messages(search("deploy"))
        .await
        .unwrap();
    assert_eq!(
        result_bodies(&response),
        vec!["deploy beta after review", "deploy alpha to staging"]
    );
    assert!(response.done);

    let response = client_b
        .search_channel_messages(search("deplo"))
        .await
        .unwrap();
    assert_eq!(response.results.len(), 2);

    let response = client_b
        .search_channel_messages(SearchChannelMessages {
            filter_channel: Some("general".to_string()),
            ..search("deploy")
        })
        .await
        .unwrap();
    assert_eq!(response.results.len(), 2);
    assert!(
        response
            .results
            .iter()
            .all(|result| result.channel_name == "general")
    );
    assert!(
        response
            .results
            .iter()
            .all(|result| result.channel_id == general.0)
    );

    let response = client_b
        .search_channel_messages(SearchChannelMessages {
            filter_user: Some("user_a".to_string()),
            ..search("deploy")
        })
        .await
        .unwrap();
    assert_eq!(result_bodies(&response), vec!["deploy alpha to staging"]);
    assert_eq!(response.results[0].sender_name, "user_a");

    let response = client_b
        .search_channel_messages(SearchChannelMessages {
            filter_channel: Some("general".to_string()),
            filter_user: Some("user_b".to_string()),
            ..search("deploy")
        })
        .await
        .unwrap();
    assert_eq!(result_bodies(&response), vec!["deploy beta after review"]);

    let response = client_b
        .search_channel_messages(SearchChannelMessages {
            filter_after: Some(1),
            ..search("deploy")
        })
        .await
        .unwrap();
    assert_eq!(response.results.len(), 2);

    let response = client_b
        .search_channel_messages(SearchChannelMessages {
            filter_before: Some(1),
            ..search("deploy")
        })
        .await
        .unwrap();
    assert!(response.results.is_empty());

    let response = client_c
        .search_channel_messages(search("deploy"))
        .await
        .unwrap();
    assert!(response.results.is_empty());
    assert!(response.done);

    let response = client_b
        .search_channel_messages(SearchChannelMessages {
            limit: 1,
            ..search("deploy")
        })
        .await
        .unwrap();
    assert_eq!(result_bodies(&response), vec!["deploy beta after review"]);
    assert!(!response.done);
}

#[gpui::test]
async fn test_channel_message_search_rejects_short_query_and_tracks_edits(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a.join_channel_chat(channel_id.0).await.unwrap();
    client_b.join_channel_chat(channel_id.0).await.unwrap();

    assert!(client_a.search_channel_messages(search("a")).await.is_err());

    let sent = client_a
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "original deploy note".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
            file_ids: Vec::new(),
        })
        .await
        .unwrap();

    client_a
        .update_channel_message(UpdateChannelMessage {
            channel_id: channel_id.0,
            message_id: sent.id,
            body: "edited launch note".to_string(),
            nonce: 2,
            mentions: Vec::new(),
        })
        .await
        .unwrap();

    let response = client_b
        .search_channel_messages(search("launch"))
        .await
        .unwrap();
    assert_eq!(result_bodies(&response), vec!["edited launch note"]);

    let response = client_b
        .search_channel_messages(search("deploy"))
        .await
        .unwrap();
    assert!(response.results.is_empty());

    client_a
        .remove_channel_message(channel_id.0, sent.id)
        .await
        .unwrap();
    let response = client_b
        .search_channel_messages(search("launch"))
        .await
        .unwrap();
    assert!(response.results.is_empty());
}

#[gpui::test]
async fn test_channel_chat_simultaneous_sends_keep_stable_order(
    executor: BackgroundExecutor,
    cx_a: &mut TestAppContext,
    cx_b: &mut TestAppContext,
) {
    let mut server = TestServer::start(executor.clone()).await;
    let client_a = server.create_client(cx_a, "user_a").await;
    let client_b = server.create_client(cx_b, "user_b").await;
    let channel_id = server
        .make_channel("chat", None, (&client_a, cx_a), &mut [(&client_b, cx_b)])
        .await;

    client_a.join_channel_chat(channel_id.0).await.unwrap();
    client_b.join_channel_chat(channel_id.0).await.unwrap();

    let send_a = client_a.send_channel_message(SendChannelMessage {
        channel_id: channel_id.0,
        body: "a".to_string(),
        nonce: 1,
        mentions: Vec::new(),
        reply_to_message_id: None,
        file_ids: Vec::new(),
    });
    let send_b = client_b.send_channel_message(SendChannelMessage {
        channel_id: channel_id.0,
        body: "b".to_string(),
        nonce: 2,
        mentions: Vec::new(),
        reply_to_message_id: None,
        file_ids: Vec::new(),
    });
    let (message_a, message_b) = futures::future::join(send_a, send_b).await;
    let message_a = message_a.unwrap();
    let message_b = message_b.unwrap();

    let history = client_a
        .get_channel_messages(channel_id.0, None)
        .await
        .unwrap()
        .messages;
    assert_eq!(history.len(), 2);
    assert_eq!(
        history.iter().map(|message| message.id).collect::<Vec<_>>(),
        {
            let mut ids = vec![message_a.id, message_b.id];
            ids.sort();
            ids
        }
    );
    let mut messages_by_id = history
        .iter()
        .map(|message| (message.id, message.body.as_str()))
        .collect::<Vec<_>>();
    messages_by_id.sort_by_key(|(id, _)| *id);
    let mut expected_messages = vec![(message_a.id, "a"), (message_b.id, "b")];
    expected_messages.sort_by_key(|(id, _)| *id);
    assert_eq!(messages_by_id, expected_messages);
}

fn mention_for(user_id: u64, start: u64, end: u64) -> proto::ChatMention {
    proto::ChatMention {
        range: Some(proto::Range { start, end }),
        user_id,
        group_id: 0,
    }
}

fn search(query: &str) -> SearchChannelMessages {
    SearchChannelMessages {
        channel_id: None,
        query: query.to_string(),
        before_message_id: None,
        limit: 20,
        filter_channel: None,
        filter_user: None,
        filter_after: None,
        filter_before: None,
    }
}

fn result_bodies(response: &proto::SearchChannelMessagesResponse) -> Vec<&str> {
    response
        .results
        .iter()
        .filter_map(|result| result.message.as_ref())
        .map(|message| message.body.as_str())
        .collect()
}

fn primitive_datetime_in(offset: TimeDuration) -> PrimitiveDateTime {
    let timestamp = OffsetDateTime::now_utc() + offset;
    PrimitiveDateTime::new(timestamp.date(), timestamp.time())
}

fn assert_reactions(reactions: &[proto::ReactionSummary], expected: &[(&str, &[u64])]) {
    assert_eq!(reactions.len(), expected.len());
    for (reaction, (emoji_name, user_ids)) in reactions.iter().zip(expected) {
        assert_eq!(reaction.emoji_name, *emoji_name);
        assert_eq!(reaction.count, u32::try_from(user_ids.len()).unwrap());
        assert_eq!(reaction.user_ids, *user_ids);
    }
}
