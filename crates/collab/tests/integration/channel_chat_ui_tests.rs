use crate::TestServer;
use client::channel_chat::SendChannelMessage;
use collab_ui::channel_chat::ChannelChat;

#[gpui::test]
async fn test_channel_chat_view_live_insert_and_send_states(
    cx_a: &mut gpui::TestAppContext,
    cx_b: &mut gpui::TestAppContext,
) {
    let (server, client_a, client_b, channel_id) = TestServer::start2(cx_a, cx_b).await;
    let (workspace, cx_a) = client_a.build_test_workspace(cx_a).await;

    let chat = cx_a
        .update(|window, cx| ChannelChat::open(channel_id, workspace.clone(), window, cx))
        .await
        .unwrap();
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.message_bodies_for_test()),
        Vec::<String>::new()
    );

    client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "from b".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
        })
        .await
        .unwrap();
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.message_bodies_for_test()),
        vec!["from b".to_string()]
    );

    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_draft_for_test("from a", window, cx);
        chat.send_for_test(window, cx);
    });
    cx_a.run_until_parked();

    assert_eq!(chat.read_with(cx_a, |chat, cx| chat.draft_for_test(cx)), "");
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.message_bodies_for_test()),
        vec!["from b".to_string(), "from a".to_string()]
    );

    server.forbid_connections();
    server.disconnect_client(client_a.client().peer_id().unwrap());
    cx_a.run_until_parked();

    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_draft_for_test("will fail", window, cx);
        chat.send_for_test(window, cx);
    });
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, cx| chat.draft_for_test(cx)),
        "will fail"
    );
    assert!(
        chat.read_with(cx_a, |chat, _| chat.send_error_for_test())
            .is_some()
    );
}
