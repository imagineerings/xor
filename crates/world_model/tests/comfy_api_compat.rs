use std::collections::BTreeMap;

use serde::Deserialize;
use world_model::{
    ComfyClientSessionId, ComfyExecutionEventTranslator, ComfyFeatureFlags, ComfyHttpMethod,
    ComfyJobBridge, ComfyJobStatus, ComfyPromptId, ComfyRouteCatalog, ComfyRouteKind,
    ComfyRuntimeEvent, ComfyWebSocketEventName, ComfyWebSocketFrame, ComfyWebSocketPayload,
    ComfyWebSocketSessionRegistry, PREVIEW_METADATA_FEATURE, PreviewPayload, PromptExtraData,
    PromptSubmission, QueueNumber,
};

const BASIC_API_PROMPT: &str = include_str!("../fixtures/comfy/basic_api_prompt.json");

#[derive(Debug, Deserialize)]
struct BasicApiPromptFixture {
    schema_version: u32,
    native_sim_records: bool,
    comfyui_passthrough: bool,
    client_id: String,
    prompt_id: String,
    target_node: usize,
    http: HttpFixture,
    websocket: WebSocketFixture,
    prompt: serde_json::Value,
    extra_data: BTreeMap<String, String>,
    sensitive_keys: Vec<String>,
    expected: ExpectedFixture,
}

#[derive(Debug, Deserialize)]
struct HttpFixture {
    submit_paths: Vec<String>,
    status_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WebSocketFixture {
    session_id: String,
    requested_features: BTreeMap<String, bool>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFixture {
    queue_number: u64,
    artifact_id: String,
    mime_type: String,
    preview_width: u32,
    preview_height: u32,
}

#[test]
fn basic_http_prompt_fixture_submits_and_reads_native_sim_job_state() {
    let fixture = basic_api_prompt_fixture();
    assert_eq!(fixture.schema_version, 1);
    assert!(fixture.native_sim_records);
    assert!(!fixture.comfyui_passthrough);

    let catalog = ComfyRouteCatalog::default_comfy_routes();
    for path in &fixture.http.submit_paths {
        let route = catalog
            .route_for_path(ComfyHttpMethod::Post, path)
            .unwrap_or_else(|| panic!("missing submit route {path}"));
        assert_eq!(route.kind, ComfyRouteKind::PromptSubmission);
    }
    for path in &fixture.http.status_paths {
        let route = catalog
            .route_for_path(ComfyHttpMethod::Get, path)
            .unwrap_or_else(|| panic!("missing status route {path}"));
        assert!(matches!(
            route.kind,
            ComfyRouteKind::Queue | ComfyRouteKind::History
        ));
    }

    let mut bridge = ComfyJobBridge::default();
    let prompt_id = ComfyPromptId::parse(&fixture.prompt_id).expect("fixture prompt id parses");
    let response = bridge
        .submit_prompt(
            PromptSubmission::new(fixture.prompt.clone())
                .with_prompt_id(prompt_id.clone())
                .with_client_id(fixture.client_id.clone())
                .with_queue_number(QueueNumber(fixture.expected.queue_number as f64))
                .with_extra_data(extra_data(&fixture)),
        )
        .expect("fixture prompt should submit");

    assert_eq!(response.prompt_id, prompt_id);
    assert_eq!(response.number, fixture.expected.queue_number);
    assert!(response.node_errors.is_empty());

    let queue = bridge.queue_status();
    assert_eq!(queue.pending.len(), 1);
    assert_eq!(queue.pending[0].prompt_id.as_str(), fixture.prompt_id);
    assert_eq!(queue.pending[0].queue_position, Some(1));
    assert_eq!(
        queue.pending[0].public_extra_data,
        BTreeMap::from([("workflow".to_string(), "basic-api-prompt".to_string())])
    );
    assert!(!queue.pending[0].public_extra_data.contains_key("secret"));

    bridge
        .update_status(&prompt_id, ComfyJobStatus::Completed)
        .expect("fixture job should complete");
    bridge
        .add_output(&prompt_id, fixture.expected.artifact_id.clone())
        .expect("fixture output should append");

    let history = bridge
        .history_for_prompt(&prompt_id)
        .expect("history exists");
    assert_eq!(history.status, ComfyJobStatus::Completed);
    assert_eq!(history.outputs, vec![fixture.expected.artifact_id]);
    assert!(!history.public_extra_data.contains_key("secret"));
}

#[test]
fn basic_websocket_fixture_negotiates_and_translates_native_events() {
    let fixture = basic_api_prompt_fixture();
    let prompt_id = ComfyPromptId::parse(&fixture.prompt_id).expect("fixture prompt id parses");
    let mut bridge = ComfyJobBridge::default();
    bridge
        .submit_prompt(
            PromptSubmission::new(fixture.prompt.clone())
                .with_prompt_id(prompt_id.clone())
                .with_client_id(fixture.client_id.clone())
                .with_extra_data(extra_data(&fixture)),
        )
        .expect("fixture prompt should submit");

    let mut registry = ComfyWebSocketSessionRegistry::default();
    let session_id = ComfyClientSessionId::new(fixture.websocket.session_id.clone());
    let connect = registry.connect(
        Some(session_id.clone()),
        Some(fixture.client_id.clone()),
        bridge.queue_status(),
    );
    assert_eq!(connect.session.session_id, session_id);
    assert!(matches!(
        connect.initial_frames.first(),
        Some(ComfyWebSocketFrame::Json {
            event: ComfyWebSocketEventName::Status,
            payload: ComfyWebSocketPayload::Status(_)
        })
    ));

    let feature_frame = registry
        .receive_feature_flags(&session_id, requested_features(&fixture))
        .expect("feature flags should negotiate");
    assert!(matches!(
        feature_frame,
        ComfyWebSocketFrame::Json {
            event: ComfyWebSocketEventName::FeatureFlags,
            payload: ComfyWebSocketPayload::FeatureFlags(_)
        }
    ));
    assert!(
        registry
            .session(&session_id)
            .expect("session exists")
            .accepted_features
            .enabled(PREVIEW_METADATA_FEATURE)
    );

    let executing = registry
        .translate_for_session(
            &session_id,
            ComfyRuntimeEvent::Executing {
                prompt_id: prompt_id.clone(),
                node_id: Some(fixture.target_node),
            },
        )
        .expect("executing frame should translate");
    let progress = registry
        .translate_for_session(
            &session_id,
            ComfyRuntimeEvent::Progress {
                prompt_id: prompt_id.clone(),
                node_id: fixture.target_node,
                value: 2,
                max: 4,
            },
        )
        .expect("progress frame should translate");
    assert!(matches!(
        executing,
        ComfyWebSocketFrame::Json {
            event: ComfyWebSocketEventName::Executing,
            ..
        }
    ));
    assert!(matches!(
        progress,
        ComfyWebSocketFrame::Json {
            event: ComfyWebSocketEventName::Progress,
            ..
        }
    ));

    let preview = registry
        .translate_for_session(
            &session_id,
            ComfyRuntimeEvent::Preview {
                prompt_id: prompt_id.clone(),
                node_id: fixture.target_node,
                payload: PreviewPayload::Metadata {
                    artifact_id: fixture.expected.artifact_id.clone(),
                    mime_type: fixture.expected.mime_type.clone(),
                    width: Some(fixture.expected.preview_width),
                    height: Some(fixture.expected.preview_height),
                },
            },
        )
        .expect("preview frame should translate");

    assert_eq!(
        preview,
        ComfyWebSocketFrame::Json {
            event: ComfyWebSocketEventName::Preview,
            payload: ComfyWebSocketPayload::PreviewMetadata {
                prompt_id,
                node_id: fixture.target_node,
                artifact_id: fixture.expected.artifact_id,
                mime_type: fixture.expected.mime_type,
                width: Some(fixture.expected.preview_width),
                height: Some(fixture.expected.preview_height),
            }
        }
    );
}

#[test]
fn basic_preview_fixture_falls_back_to_legacy_binary_for_legacy_clients() {
    let fixture = basic_api_prompt_fixture();
    let prompt_id = ComfyPromptId::parse(&fixture.prompt_id).expect("fixture prompt id parses");

    let frame = ComfyExecutionEventTranslator::translate(
        ComfyRuntimeEvent::Preview {
            prompt_id: prompt_id.clone(),
            node_id: fixture.target_node,
            payload: PreviewPayload::Metadata {
                artifact_id: fixture.expected.artifact_id,
                mime_type: fixture.expected.mime_type.clone(),
                width: Some(fixture.expected.preview_width),
                height: Some(fixture.expected.preview_height),
            },
        },
        &ComfyFeatureFlags::default(),
    );

    assert_eq!(
        frame,
        ComfyWebSocketFrame::BinaryPreview {
            prompt_id,
            node_id: fixture.target_node,
            mime_type: fixture.expected.mime_type,
            byte_count: 0,
        }
    );
}

fn basic_api_prompt_fixture() -> BasicApiPromptFixture {
    serde_json::from_str(BASIC_API_PROMPT).expect("basic API prompt fixture should parse")
}

fn extra_data(fixture: &BasicApiPromptFixture) -> PromptExtraData {
    fixture.extra_data.iter().fold(
        fixture
            .sensitive_keys
            .iter()
            .fold(PromptExtraData::default(), |extra_data, key| {
                extra_data.with_sensitive_key(key.clone())
            }),
        |extra_data, (key, value)| extra_data.with_public(key.clone(), value.clone()),
    )
}

fn requested_features(fixture: &BasicApiPromptFixture) -> ComfyFeatureFlags {
    fixture
        .websocket
        .requested_features
        .iter()
        .fold(ComfyFeatureFlags::default(), |features, (key, enabled)| {
            features.with_flag(key.clone(), *enabled)
        })
}
