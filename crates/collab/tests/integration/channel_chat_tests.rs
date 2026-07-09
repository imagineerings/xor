use crate::TestServer;
use client::{
    channel_chat::{
        DEFAULT_THREAD_REPLY_LIMIT, ScheduleChannelMessage, SearchChannelMessages,
        SendChannelMessage, UpdateChannelMessage,
    },
    proto,
};
use collab::db::{
    ScheduledMessageId as DbScheduledMessageId, scheduled_message_store::ScheduledMessageStore,
};
use gpui::{AppContext, BackgroundExecutor, TestAppContext};
use pretty_assertions::assert_eq;
use std::time::Duration as StdDuration;
use time::{Duration as TimeDuration, OffsetDateTime, PrimitiveDateTime};

#[derive(Default)]
struct ReactionHandlerEntity;

#[derive(Default)]
struct ScheduledMessageHandlerEntity;

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
    });
    let send_b = client_b.send_channel_message(SendChannelMessage {
        channel_id: channel_id.0,
        body: "b".to_string(),
        nonce: 2,
        mentions: Vec::new(),
        reply_to_message_id: None,
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
