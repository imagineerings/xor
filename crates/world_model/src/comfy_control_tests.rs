use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use crate::{
    ClientFeatureNegotiation, ComfyFeatureFlags, ComfyJobStatus, ComfyJobSummary, ComfyPromptId,
    ComfyRuntimeEvent, HistoryAction, INVALID_PROMPT_ID_CODE, PreviewPayload, PromptExtraData,
    PromptSubmission, QueueAction, QueueNumber, QueueStatus,
};

#[test]
fn prompt_id_accepts_only_canonical_lowercase_hyphenated_uuid() {
    let prompt_id = ComfyPromptId::parse("550e8400-e29b-41d4-a716-446655440000")
        .expect("canonical prompt id should parse");
    assert_eq!(prompt_id.as_str(), "550e8400-e29b-41d4-a716-446655440000");

    for invalid in [
        "550E8400-E29B-41D4-A716-446655440000",
        "550e8400e29b41d4a716446655440000",
        "not-a-uuid",
    ] {
        let diagnostic = ComfyPromptId::parse(invalid).expect_err("prompt id should be rejected");
        assert_eq!(diagnostic.code, INVALID_PROMPT_ID_CODE);
        assert!(
            diagnostic
                .message
                .contains("canonical lowercase hyphenated UUID")
        );
    }
}

#[test]
fn prompt_submission_preserves_comfy_fields_as_native_sim_protocol_data() {
    let prompt_id = ComfyPromptId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let submission = PromptSubmission::new(json!({"1": {"class_type": "KSampler"}}))
        .with_prompt_id(prompt_id.clone())
        .with_client_id("client-a")
        .with_front(true)
        .with_queue_number(QueueNumber(2.5))
        .with_extra_data(
            PromptExtraData::default()
                .with_public("workflow", "smoke")
                .with_public("token", "secret")
                .with_sensitive_key("token"),
        )
        .with_partial_execution_targets([7, 9]);

    assert_eq!(submission.prompt_id, Some(prompt_id));
    assert_eq!(submission.client_id.as_deref(), Some("client-a"));
    assert!(submission.front);
    assert_eq!(submission.queue_number, Some(QueueNumber(2.5)));
    assert_eq!(submission.partial_execution_targets, vec![7, 9]);
    assert_eq!(
        submission.extra_data.redacted(),
        BTreeMap::from([("workflow".to_string(), "smoke".to_string())])
    );
}

#[test]
fn queue_history_and_job_summaries_model_sim_state_without_sensitive_data() {
    let prompt_id = ComfyPromptId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let prompt_ids = BTreeSet::from([prompt_id.clone()]);

    assert_eq!(
        QueueAction::Delete {
            prompt_ids: prompt_ids.clone()
        },
        QueueAction::Delete {
            prompt_ids: BTreeSet::from([prompt_id.clone()])
        }
    );
    assert_eq!(
        HistoryAction::Delete { prompt_ids },
        HistoryAction::Delete {
            prompt_ids: BTreeSet::from([prompt_id.clone()])
        }
    );

    let summary = ComfyJobSummary {
        prompt_id,
        queue_position: Some(1),
        status: ComfyJobStatus::Running,
        client_id: Some("client-a".to_string()),
        outputs: vec!["artifact:image-1".to_string()],
        public_extra_data: BTreeMap::from([("workflow".to_string(), "smoke".to_string())]),
    };
    let status = QueueStatus {
        running: vec![summary],
        pending: Vec::new(),
        history_count: 3,
    };

    assert!(!ComfyJobStatus::Running.is_terminal());
    assert!(ComfyJobStatus::Cancelled.is_terminal());
    assert_eq!(status.running[0].public_extra_data.len(), 1);
}

#[test]
fn runtime_events_preserve_feature_flags_progress_and_preview_metadata() {
    let prompt_id = ComfyPromptId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let requested = ComfyFeatureFlags::default().with_flag("preview_metadata", true);
    let accepted = ComfyFeatureFlags::default()
        .with_flag("preview_metadata", true)
        .with_flag("legacy_preview", false);
    let feature_event = ComfyRuntimeEvent::FeatureFlags(ClientFeatureNegotiation {
        client_id: "client-a".to_string(),
        requested,
        accepted: accepted.clone(),
    });

    assert!(accepted.enabled("preview_metadata"));
    assert!(!accepted.enabled("unknown"));
    assert!(matches!(
        feature_event,
        ComfyRuntimeEvent::FeatureFlags(ClientFeatureNegotiation { .. })
    ));

    let progress_event = ComfyRuntimeEvent::Progress {
        prompt_id: prompt_id.clone(),
        node_id: 4,
        value: 2,
        max: 8,
    };
    assert!(matches!(
        progress_event,
        ComfyRuntimeEvent::Progress {
            node_id: 4,
            value: 2,
            max: 8,
            ..
        }
    ));

    let preview_event = ComfyRuntimeEvent::Preview {
        prompt_id,
        node_id: 5,
        payload: PreviewPayload::Metadata {
            artifact_id: "artifact:image-1".to_string(),
            mime_type: "image/png".to_string(),
            width: Some(512),
            height: Some(512),
        },
    };
    assert!(matches!(
        preview_event,
        ComfyRuntimeEvent::Preview {
            payload: PreviewPayload::Metadata { .. },
            ..
        }
    ));
}
