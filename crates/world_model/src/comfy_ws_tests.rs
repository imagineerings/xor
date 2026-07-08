use crate::{
    ComfyClientSessionId, ComfyExecutionEventTranslator, ComfyFeatureFlags, ComfyJobBridge,
    ComfyJobStatus, ComfyPromptId, ComfyRuntimeEvent, ComfyWebSocketEventName, ComfyWebSocketFrame,
    ComfyWebSocketPayload, ComfyWebSocketSessionRegistry, LEGACY_PREVIEW_FEATURE,
    PREVIEW_METADATA_FEATURE, PreviewPayload, PromptSubmission,
};

#[test]
fn websocket_connect_assigns_session_and_sends_initial_queue_status() {
    let mut registry = ComfyWebSocketSessionRegistry::default();
    let queue_status = seeded_queue_status();

    let connect = registry.connect(None, Some("client-a".to_string()), queue_status.clone());

    assert!(!connect.session.session_id.as_str().is_empty());
    assert_eq!(
        connect.session.requested_client_id.as_deref(),
        Some("client-a")
    );
    assert_eq!(connect.initial_frames.len(), 1);
    assert_eq!(
        connect.initial_frames[0],
        ComfyWebSocketFrame::Json {
            event: ComfyWebSocketEventName::Status,
            payload: ComfyWebSocketPayload::Status(queue_status)
        }
    );
}

#[test]
fn websocket_connect_reuses_requested_session_id() {
    let mut registry = ComfyWebSocketSessionRegistry::default();
    let session_id = ComfyClientSessionId::new("session-a");

    let first = registry.connect(
        Some(session_id.clone()),
        Some("client-a".to_string()),
        seeded_queue_status(),
    );
    let second = registry.connect(
        Some(session_id.clone()),
        Some("client-b".to_string()),
        seeded_queue_status(),
    );

    assert_eq!(first.session.session_id, session_id);
    assert_eq!(second.session.session_id, session_id);
    assert_eq!(
        registry
            .session(&session_id)
            .unwrap()
            .requested_client_id
            .as_deref(),
        Some("client-b")
    );
}

#[test]
fn feature_negotiation_accepts_only_server_supported_requested_flags() {
    let mut registry = ComfyWebSocketSessionRegistry::default();
    let session_id = ComfyClientSessionId::new("session-a");
    registry.connect(Some(session_id.clone()), None, seeded_queue_status());

    let frame = registry
        .receive_feature_flags(
            &session_id,
            ComfyFeatureFlags::default()
                .with_flag(PREVIEW_METADATA_FEATURE, true)
                .with_flag(LEGACY_PREVIEW_FEATURE, false)
                .with_flag("unsupported", true),
        )
        .expect("session should exist");

    let ComfyWebSocketFrame::Json {
        event: ComfyWebSocketEventName::FeatureFlags,
        payload: ComfyWebSocketPayload::FeatureFlags(negotiation),
    } = frame
    else {
        panic!("expected feature flag frame");
    };

    assert!(negotiation.accepted.enabled(PREVIEW_METADATA_FEATURE));
    assert!(!negotiation.accepted.enabled(LEGACY_PREVIEW_FEATURE));
    assert!(!negotiation.accepted.enabled("unsupported"));
    assert!(
        registry
            .session(&session_id)
            .unwrap()
            .accepted_features
            .enabled(PREVIEW_METADATA_FEATURE)
    );
}

#[test]
fn translator_maps_status_executing_and_progress_events_to_json_frames() {
    let prompt_id = prompt_id("550e8400-e29b-41d4-a716-446655440200");
    let features = ComfyFeatureFlags::default();

    let executing = ComfyExecutionEventTranslator::translate(
        ComfyRuntimeEvent::Executing {
            prompt_id: prompt_id.clone(),
            node_id: Some(7),
        },
        &features,
    );
    let progress = ComfyExecutionEventTranslator::translate(
        ComfyRuntimeEvent::Progress {
            prompt_id,
            node_id: 7,
            value: 3,
            max: 8,
        },
        &features,
    );

    assert!(matches!(
        executing,
        ComfyWebSocketFrame::Json {
            event: ComfyWebSocketEventName::Executing,
            payload: ComfyWebSocketPayload::Executing {
                node_id: Some(7),
                ..
            }
        }
    ));
    assert!(matches!(
        progress,
        ComfyWebSocketFrame::Json {
            event: ComfyWebSocketEventName::Progress,
            payload: ComfyWebSocketPayload::Progress {
                value: 3,
                max: 8,
                ..
            }
        }
    ));
}

#[test]
fn preview_translation_uses_metadata_when_session_supports_it() {
    let mut registry = ComfyWebSocketSessionRegistry::default();
    let session_id = ComfyClientSessionId::new("session-a");
    let prompt_id = prompt_id("550e8400-e29b-41d4-a716-446655440210");
    registry.connect(Some(session_id.clone()), None, seeded_queue_status());
    registry.receive_feature_flags(
        &session_id,
        ComfyFeatureFlags::default().with_flag(PREVIEW_METADATA_FEATURE, true),
    );

    let frame = registry
        .translate_for_session(
            &session_id,
            ComfyRuntimeEvent::Preview {
                prompt_id: prompt_id.clone(),
                node_id: 9,
                payload: PreviewPayload::Metadata {
                    artifact_id: "artifact:image-1".to_string(),
                    mime_type: "image/png".to_string(),
                    width: Some(512),
                    height: Some(256),
                },
            },
        )
        .expect("session should exist");

    assert_eq!(
        frame,
        ComfyWebSocketFrame::Json {
            event: ComfyWebSocketEventName::Preview,
            payload: ComfyWebSocketPayload::PreviewMetadata {
                prompt_id,
                node_id: 9,
                artifact_id: "artifact:image-1".to_string(),
                mime_type: "image/png".to_string(),
                width: Some(512),
                height: Some(256),
            }
        }
    );
}

#[test]
fn preview_translation_falls_back_to_legacy_binary_without_metadata_support() {
    let prompt_id = prompt_id("550e8400-e29b-41d4-a716-446655440220");
    let frame = ComfyExecutionEventTranslator::translate(
        ComfyRuntimeEvent::Preview {
            prompt_id: prompt_id.clone(),
            node_id: 4,
            payload: PreviewPayload::Metadata {
                artifact_id: "artifact:image-1".to_string(),
                mime_type: "image/png".to_string(),
                width: Some(512),
                height: Some(512),
            },
        },
        &ComfyFeatureFlags::default(),
    );

    assert_eq!(
        frame,
        ComfyWebSocketFrame::BinaryPreview {
            prompt_id,
            node_id: 4,
            mime_type: "image/png".to_string(),
            byte_count: 0,
        }
    );
}

fn seeded_queue_status() -> crate::QueueStatus {
    let mut bridge = ComfyJobBridge::default();
    let prompt_id = prompt_id("550e8400-e29b-41d4-a716-446655440230");
    bridge
        .submit_prompt(
            PromptSubmission::new(serde_json::json!({})).with_prompt_id(prompt_id.clone()),
        )
        .expect("prompt should submit");
    bridge
        .update_status(&prompt_id, ComfyJobStatus::Running)
        .expect("job should update");
    bridge.queue_status()
}

fn prompt_id(value: &str) -> ComfyPromptId {
    ComfyPromptId::parse(value).expect("prompt id should parse")
}
