use crate::TestServer;
use client::{
    channel_chat::{SendChannelMessage, UpdateChannelMessage},
    proto,
};
use gpui::{BackgroundExecutor, TestAppContext};
use pretty_assertions::assert_eq;

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
