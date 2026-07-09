use crate::TestServer;
use client::channel_chat::SendChannelMessage;
use collab_ui::{channel_chat::ChannelChat, draft_store::DraftStore};
use gpui::TaskExt;
use std::time::Duration;

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

#[gpui::test]
async fn test_channel_chat_restores_saved_draft_and_clears_on_send(
    cx_a: &mut gpui::TestAppContext,
    cx_b: &mut gpui::TestAppContext,
) {
    let (_server, client_a, _client_b, channel_id) = TestServer::start2(cx_a, cx_b).await;
    let (workspace, cx_a) = client_a.build_test_workspace(cx_a).await;

    cx_a.update(|_, cx| {
        let save_draft = DraftStore::global(cx).update(cx, |draft_store, cx| {
            draft_store.save_draft_in_background(channel_id, "restore me".into(), cx)
        });
        save_draft.detach_and_log_err(cx);
    });
    cx_a.run_until_parked();

    let chat = cx_a
        .update(|window, cx| ChannelChat::open(channel_id, workspace.clone(), window, cx))
        .await
        .unwrap();
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, cx| chat.draft_for_test(cx)),
        "restore me"
    );

    chat.update_in(cx_a, |chat, window, cx| {
        chat.send_for_test(window, cx);
    });
    cx_a.run_until_parked();

    assert_eq!(chat.read_with(cx_a, |chat, cx| chat.draft_for_test(cx)), "");
    assert!(!cx_a.update(|_, cx| DraftStore::global(cx).read(cx).has_draft(channel_id)));
}

#[gpui::test]
async fn test_channel_chat_markdown_preview_toolbar_and_sent_rendering(
    cx_a: &mut gpui::TestAppContext,
    cx_b: &mut gpui::TestAppContext,
) {
    let (_server, client_a, client_b, channel_id) = TestServer::start2(cx_a, cx_b).await;
    let (workspace, cx_a) = client_a.build_test_workspace(cx_a).await;

    let chat = cx_a
        .update(|window, cx| ChannelChat::open(channel_id, workspace.clone(), window, cx))
        .await
        .unwrap();
    cx_a.run_until_parked();

    let markdown_draft = "# Heading\n**bold** [link](https://example.com)";
    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_draft_for_test(markdown_draft, window, cx);
        chat.focus_composer_for_test(window, cx);
    });
    cx_a.run_until_parked();

    assert!(chat.update_in(cx_a, |chat, window, cx| {
        chat.formatting_toolbar_visible_for_test(window, cx)
    }));

    chat.update_in(cx_a, |chat, window, cx| {
        chat.toggle_preview_for_test(window, cx);
    });
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.compose_mode_for_test()),
        "preview"
    );
    assert_eq!(
        ChannelChat::rendered_compose_preview_for_test(chat.clone(), cx_a),
        Some("Heading\nbold link".to_string())
    );

    chat.update_in(cx_a, |chat, window, cx| {
        chat.toggle_preview_for_test(window, cx);
    });
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.compose_mode_for_test()),
        "source"
    );
    assert_eq!(
        chat.read_with(cx_a, |chat, cx| chat.draft_for_test(cx)),
        markdown_draft
    );

    chat.update_in(cx_a, |chat, window, cx| {
        chat.blur_for_test(window);
        assert!(!chat.formatting_toolbar_visible_for_test(window, cx));
    });
    cx_a.run_until_parked();

    client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: markdown_draft.to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
        })
        .await
        .unwrap();
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.message_bodies_for_test()),
        vec![markdown_draft.to_string()]
    );
    assert_eq!(
        ChannelChat::rendered_message_texts_for_test(chat.clone(), cx_a),
        vec!["Heading\nbold link".to_string()]
    );
}

#[gpui::test]
async fn test_channel_chat_thread_compose_sends_reply(
    cx_a: &mut gpui::TestAppContext,
    cx_b: &mut gpui::TestAppContext,
) {
    let (_server, client_a, client_b, channel_id) = TestServer::start2(cx_a, cx_b).await;
    let (workspace, cx_a) = client_a.build_test_workspace(cx_a).await;

    let root = client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "root".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    let chat = cx_a
        .update(|window, cx| ChannelChat::open(channel_id, workspace.clone(), window, cx))
        .await
        .unwrap();
    cx_a.run_until_parked();

    chat.update_in(cx_a, |chat, window, cx| {
        chat.open_thread_for_test(root.id, window, cx);
    });
    cx_a.run_until_parked();

    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_thread_draft_for_test("reply from thread panel", window, cx);
        chat.send_thread_reply_for_test(window, cx);
    });

    assert_eq!(
        chat.read_with(cx_a, |chat, cx| chat.thread_draft_for_test(cx)),
        ""
    );
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.thread_reply_bodies_for_test()),
        vec!["reply from thread panel".to_string()]
    );

    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, cx| chat.thread_draft_for_test(cx)),
        ""
    );
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.thread_reply_bodies_for_test()),
        vec!["reply from thread panel".to_string()]
    );
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.thread_reply_count_for_test(root.id)),
        Some(1)
    );

    let thread = client_b
        .client()
        .get_thread(channel_id.0, root.id)
        .await
        .unwrap();
    assert_eq!(thread.replies.len(), 1);
    assert_eq!(thread.replies[0].body, "reply from thread panel");
    assert_eq!(thread.replies[0].reply_to_message_id, Some(root.id));
}

#[gpui::test]
async fn test_channel_chat_open_thread_appends_live_replies(
    cx_a: &mut gpui::TestAppContext,
    cx_b: &mut gpui::TestAppContext,
) {
    let (_server, client_a, client_b, channel_id) = TestServer::start2(cx_a, cx_b).await;
    let (workspace, cx_a) = client_a.build_test_workspace(cx_a).await;

    let root = client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "root".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    let initial_reply = client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "initial reply".to_string(),
            nonce: 2,
            mentions: Vec::new(),
            reply_to_message_id: Some(root.id),
        })
        .await
        .unwrap();

    let other_root = client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "other root".to_string(),
            nonce: 3,
            mentions: Vec::new(),
            reply_to_message_id: None,
        })
        .await
        .unwrap();
    client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "other reply".to_string(),
            nonce: 4,
            mentions: Vec::new(),
            reply_to_message_id: Some(other_root.id),
        })
        .await
        .unwrap();

    let chat = cx_a
        .update(|window, cx| ChannelChat::open(channel_id, workspace.clone(), window, cx))
        .await
        .unwrap();
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.thread_reply_count_for_test(root.id)),
        Some(1)
    );
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.thread_has_unread_for_test(root.id)),
        Some(true)
    );
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat
            .thread_has_unread_for_test(other_root.id)),
        Some(true)
    );
    let thread = client_a
        .client()
        .get_thread(channel_id.0, root.id)
        .await
        .unwrap();
    assert_eq!(thread.root_message.id, root.id);
    assert_eq!(thread.replies.len(), 1);
    assert_eq!(thread.replies[0].id, initial_reply.id);

    chat.update_in(cx_a, |chat, window, cx| {
        chat.open_thread_for_test(root.id, window, cx);
    });
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.thread_reply_bodies_for_test()),
        vec![initial_reply.body.clone()]
    );
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.thread_has_unread_for_test(root.id)),
        Some(false)
    );
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat
            .thread_has_unread_for_test(other_root.id)),
        Some(true)
    );

    let live_reply = client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "live reply".to_string(),
            nonce: 5,
            mentions: Vec::new(),
            reply_to_message_id: Some(root.id),
        })
        .await
        .unwrap();
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.thread_reply_bodies_for_test()),
        vec![initial_reply.body, live_reply.body]
    );
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.thread_reply_count_for_test(root.id)),
        Some(2)
    );
}

#[gpui::test]
async fn test_channel_chat_thread_deleted_root_placeholder(
    cx_a: &mut gpui::TestAppContext,
    cx_b: &mut gpui::TestAppContext,
) {
    let (_server, client_a, client_b, channel_id) = TestServer::start2(cx_a, cx_b).await;
    let (workspace, cx_a) = client_a.build_test_workspace(cx_a).await;

    let root = client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "root".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
        })
        .await
        .unwrap();
    client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "reply".to_string(),
            nonce: 2,
            mentions: Vec::new(),
            reply_to_message_id: Some(root.id),
        })
        .await
        .unwrap();
    client_b
        .client()
        .remove_channel_message(channel_id.0, root.id)
        .await
        .unwrap();

    let chat = cx_a
        .update(|window, cx| ChannelChat::open(channel_id, workspace.clone(), window, cx))
        .await
        .unwrap();
    cx_a.run_until_parked();

    chat.update_in(cx_a, |chat, window, cx| {
        chat.open_thread_for_test(root.id, window, cx);
    });
    cx_a.run_until_parked();

    assert!(chat.read_with(cx_a, |chat, _| {
        chat.thread_deleted_placeholder_visible_for_test()
    }));
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.thread_reply_bodies_for_test()),
        vec!["reply".to_string()]
    );
}

#[gpui::test]
async fn test_channel_chat_thread_load_retry_exhaustion(
    cx_a: &mut gpui::TestAppContext,
    cx_b: &mut gpui::TestAppContext,
) {
    let (server, client_a, client_b, channel_id) = TestServer::start2(cx_a, cx_b).await;
    let (workspace, cx_a) = client_a.build_test_workspace(cx_a).await;

    let root = client_b
        .client()
        .send_channel_message(SendChannelMessage {
            channel_id: channel_id.0,
            body: "root".to_string(),
            nonce: 1,
            mentions: Vec::new(),
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    let chat = cx_a
        .update(|window, cx| ChannelChat::open(channel_id, workspace.clone(), window, cx))
        .await
        .unwrap();
    cx_a.run_until_parked();

    server.forbid_connections();
    server.disconnect_client(client_a.client().peer_id().unwrap());
    cx_a.run_until_parked();

    chat.update_in(cx_a, |chat, window, cx| {
        chat.open_thread_for_test(root.id, window, cx);
    });
    cx_a.background_executor
        .advance_clock(Duration::from_secs(2));
    cx_a.run_until_parked();

    let error = chat
        .read_with(cx_a, |chat, _| chat.thread_load_error_for_test())
        .expect("missing thread load error");
    assert!(error.contains("Failed to load thread after 4 attempts"));
}

#[gpui::test]
async fn test_channel_chat_message_search_state_and_pagination(
    cx_a: &mut gpui::TestAppContext,
    cx_b: &mut gpui::TestAppContext,
) {
    let (_server, client_a, client_b, channel_id) = TestServer::start2(cx_a, cx_b).await;
    let (workspace, cx_a) = client_a.build_test_workspace(cx_a).await;

    for index in 0..21 {
        client_b
            .client()
            .send_channel_message(SendChannelMessage {
                channel_id: channel_id.0,
                body: format!("deploy item {index:02}"),
                nonce: index + 1,
                mentions: Vec::new(),
                reply_to_message_id: None,
            })
            .await
            .unwrap();
    }

    let chat = cx_a
        .update(|window, cx| ChannelChat::open(channel_id, workspace.clone(), window, cx))
        .await
        .unwrap();
    cx_a.run_until_parked();

    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_search_query_for_test("deploy", window, cx);
    });
    cx_a.background_executor
        .advance_clock(Duration::from_millis(300));
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.search_result_bodies_for_test())
            .len(),
        20
    );
    assert!(chat.read_with(cx_a, |chat, _| { chat.search_load_more_visible_for_test() }));
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.selected_search_result_index_for_test()),
        Some(0)
    );

    chat.update_in(cx_a, |chat, window, cx| {
        chat.select_next_search_result_for_test(window, cx);
    });
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.selected_search_result_index_for_test()),
        Some(1)
    );
    chat.update_in(cx_a, |chat, window, cx| {
        chat.select_previous_search_result_for_test(window, cx);
    });
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.selected_search_result_index_for_test()),
        Some(0)
    );

    chat.update(cx_a, |chat, cx| {
        chat.load_more_search_results_for_test(cx);
    });
    cx_a.run_until_parked();

    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.search_result_bodies_for_test())
            .len(),
        21
    );
    assert!(chat.read_with(cx_a, |chat, _| chat.search_done_for_test()));
    assert!(!chat.read_with(cx_a, |chat, _| { chat.search_load_more_visible_for_test() }));

    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_search_query_for_test("missing", window, cx);
    });
    cx_a.background_executor
        .advance_clock(Duration::from_millis(300));
    cx_a.run_until_parked();

    assert!(chat.read_with(cx_a, |chat, _| {
        chat.search_result_bodies_for_test().is_empty()
    }));
    assert!(chat.read_with(cx_a, |chat, _| chat.search_done_for_test()));
    assert!(chat.read_with(cx_a, |chat, _| chat.search_error_for_test().is_none()));

    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_search_query_for_test("d", window, cx);
    });
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.search_error_for_test())
            .as_deref(),
        Some("Query must be at least 2 characters")
    );

    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_search_query_for_test("", window, cx);
    });
    assert!(chat.read_with(cx_a, |chat, _| {
        chat.search_result_bodies_for_test().is_empty()
    }));
    assert!(chat.read_with(cx_a, |chat, _| chat.search_error_for_test().is_none()));
}

#[gpui::test]
async fn test_channel_chat_view_updates_live_reactions(
    cx_a: &mut gpui::TestAppContext,
    cx_b: &mut gpui::TestAppContext,
) {
    let (_server, client_a, client_b, channel_id) = TestServer::start2(cx_a, cx_b).await;
    let (workspace, cx_a) = client_a.build_test_workspace(cx_a).await;

    let chat = cx_a
        .update(|window, cx| ChannelChat::open(channel_id, workspace.clone(), window, cx))
        .await
        .unwrap();
    cx_a.run_until_parked();

    let sent = client_b
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

    client_b
        .client()
        .add_channel_message_reaction(channel_id.0, sent.id, "thumbs_up".to_string())
        .await
        .unwrap();
    cx_a.run_until_parked();

    let user_id = client_b.current_user_id(cx_b).to_proto();
    let reactions = chat.read_with(cx_a, |chat, _| chat.message_reactions_for_test());
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0].len(), 1);
    assert_eq!(reactions[0][0].emoji_name, "thumbs_up");
    assert_eq!(reactions[0][0].count, 1);
    assert_eq!(reactions[0][0].user_ids, vec![user_id]);
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.reaction_chip_labels_for_test()),
        vec![vec!["👍 1".to_string()]]
    );
    assert_eq!(
        chat.update(cx_a, |chat, cx| chat.reaction_tooltips_for_test(cx)),
        vec![vec!["user_b".to_string()]]
    );
}

#[gpui::test]
async fn test_channel_chat_emoji_picker_adds_reaction(
    cx_a: &mut gpui::TestAppContext,
    cx_b: &mut gpui::TestAppContext,
) {
    let (_server, client_a, client_b, channel_id) = TestServer::start2(cx_a, cx_b).await;
    let (workspace, cx_a) = client_a.build_test_workspace(cx_a).await;

    let chat = cx_a
        .update(|window, cx| ChannelChat::open(channel_id, workspace.clone(), window, cx))
        .await
        .unwrap();
    cx_a.run_until_parked();

    let sent = client_b
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

    chat.update_in(cx_a, |chat, window, cx| {
        chat.open_emoji_picker_for_test(sent.id, window, cx);
    });
    assert!(chat.read_with(cx_a, |chat, _| chat.emoji_picker_open_for_test()));
    assert!(
        chat.read_with(cx_a, |chat, cx| chat.emoji_picker_labels_for_test(cx))
            .contains(&"👍 thumbs_up".to_string())
    );

    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_emoji_search_for_test("love", window, cx);
    });
    assert_eq!(
        chat.read_with(cx_a, |chat, cx| chat.emoji_picker_labels_for_test(cx)),
        vec!["❤️ heart".to_string()]
    );

    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_emoji_search_for_test("skin", window, cx);
    });
    assert_eq!(
        chat.read_with(cx_a, |chat, cx| chat.emoji_picker_labels_for_test(cx)),
        vec![
            "👍🏻 thumbs_up_light_skin_tone".to_string(),
            "👍🏼 thumbs_up_medium_light_skin_tone".to_string(),
            "👍🏽 thumbs_up_medium_skin_tone".to_string(),
            "👍🏾 thumbs_up_medium_dark_skin_tone".to_string(),
            "👍🏿 thumbs_up_dark_skin_tone".to_string(),
        ]
    );

    chat.update_in(cx_a, |chat, window, cx| {
        chat.set_emoji_search_for_test("definitely-not-an-emoji", window, cx);
    });
    assert!(chat.read_with(cx_a, |chat, cx| chat.emoji_picker_empty_for_test(cx)));

    chat.update_in(cx_a, |chat, window, cx| {
        chat.select_emoji_for_test(sent.id, "heart", window, cx);
    });
    cx_a.run_until_parked();

    assert!(!chat.read_with(cx_a, |chat, _| chat.emoji_picker_open_for_test()));
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.recent_emoji_names_for_test()),
        vec!["heart".to_string()]
    );
    assert_eq!(
        chat.read_with(cx_a, |chat, _| chat.reaction_chip_labels_for_test()),
        vec![vec!["❤️ 1".to_string()]]
    );
}
