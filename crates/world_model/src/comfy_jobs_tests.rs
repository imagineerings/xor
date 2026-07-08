use std::collections::BTreeMap;

use serde_json::json;

use crate::{
    ComfyJobBridge, ComfyJobListFilter, ComfyJobSort, ComfyJobStatus, ComfyPromptId,
    DUPLICATE_JOB_CODE, HistoryAction, PromptExtraData, PromptSubmission, QueueAction, QueueNumber,
};

#[test]
fn job_bridge_submits_prompts_as_native_sim_jobs() {
    let mut bridge = ComfyJobBridge::default();
    let prompt_id = prompt_id("550e8400-e29b-41d4-a716-446655440000");
    let response = bridge
        .submit_prompt(
            PromptSubmission::new(json!({"1": {"class_type": "KSampler"}}))
                .with_prompt_id(prompt_id.clone())
                .with_client_id("client-a")
                .with_queue_number(QueueNumber(7.9))
                .with_extra_data(
                    PromptExtraData::default()
                        .with_public("workflow", "smoke")
                        .with_public("api_key", "secret")
                        .with_sensitive_key("api_key"),
                ),
        )
        .expect("prompt should be submitted");

    assert_eq!(response.prompt_id, prompt_id);
    assert_eq!(response.number, 7);
    assert!(response.node_errors.is_empty());

    let record = bridge
        .record(&response.prompt_id)
        .expect("record should be present");
    assert_eq!(record.status, ComfyJobStatus::Pending);
    assert_eq!(record.client_id.as_deref(), Some("client-a"));
    assert_eq!(record.prompt, json!({"1": {"class_type": "KSampler"}}));
}

#[test]
fn queue_status_sorts_pending_jobs_and_redacts_extra_data() {
    let mut bridge = ComfyJobBridge::default();
    let first = submit_with_number(&mut bridge, "550e8400-e29b-41d4-a716-446655440001", 10.0);
    let second = submit_with_number(&mut bridge, "550e8400-e29b-41d4-a716-446655440002", 2.0);
    let running = submit_with_number(&mut bridge, "550e8400-e29b-41d4-a716-446655440003", 4.0);
    bridge
        .update_status(&running, ComfyJobStatus::Running)
        .expect("job should update");

    let status = bridge.queue_status();

    assert_eq!(status.pending[0].prompt_id, second);
    assert_eq!(status.pending[0].queue_position, Some(1));
    assert_eq!(status.pending[1].prompt_id, first);
    assert_eq!(status.pending[1].queue_position, Some(2));
    assert_eq!(status.running[0].prompt_id, running);
    assert_eq!(status.running[0].queue_position, None);
    assert_eq!(
        status.pending[0].public_extra_data,
        BTreeMap::from([(
            "workflow".to_string(),
            "550e8400-e29b-41d4-a716-446655440002".to_string()
        )])
    );
    assert!(!status.pending[0].public_extra_data.contains_key("secret"));
}

#[test]
fn history_reads_include_only_terminal_jobs() {
    let mut bridge = ComfyJobBridge::default();
    let pending = submit_with_number(&mut bridge, "550e8400-e29b-41d4-a716-446655440010", 1.0);
    let completed = submit_with_number(&mut bridge, "550e8400-e29b-41d4-a716-446655440011", 2.0);
    let failed = submit_with_number(&mut bridge, "550e8400-e29b-41d4-a716-446655440012", 3.0);
    bridge
        .update_status(&completed, ComfyJobStatus::Completed)
        .expect("job should update");
    bridge
        .update_status(&failed, ComfyJobStatus::Failed)
        .expect("job should update");
    bridge
        .add_output(&completed, "artifact:image-1")
        .expect("output should append");

    let history = bridge.history();

    assert_eq!(history.len(), 2);
    assert_eq!(history[0].prompt_id, completed);
    assert_eq!(history[0].outputs, vec!["artifact:image-1"]);
    assert_eq!(history[1].prompt_id, failed);
    assert!(bridge.history_for_prompt(&pending).is_none());
    assert_eq!(
        bridge.history_for_prompt(&completed).unwrap().status,
        ComfyJobStatus::Completed
    );
}

#[test]
fn job_listing_filters_and_sorts_without_exposing_sensitive_data() {
    let mut bridge = ComfyJobBridge::default();
    let alpha = submit_for_client(
        &mut bridge,
        "550e8400-e29b-41d4-a716-446655440020",
        "alpha",
        1.0,
    );
    let beta = submit_for_client(
        &mut bridge,
        "550e8400-e29b-41d4-a716-446655440021",
        "beta",
        2.0,
    );
    let alpha_completed = submit_for_client(
        &mut bridge,
        "550e8400-e29b-41d4-a716-446655440022",
        "alpha",
        3.0,
    );
    bridge
        .update_status(&alpha_completed, ComfyJobStatus::Completed)
        .expect("job should update");

    let listed = bridge.list_jobs(
        ComfyJobListFilter::default()
            .with_client_id("alpha")
            .with_sort(ComfyJobSort::QueueDescending),
    );

    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].prompt_id, alpha_completed);
    assert_eq!(listed[1].prompt_id, alpha);
    assert!(
        listed
            .iter()
            .all(|job| job.client_id.as_deref() == Some("alpha"))
    );
    assert!(
        listed
            .iter()
            .all(|job| !job.public_extra_data.contains_key("secret"))
    );

    let filtered = bridge.list_jobs(ComfyJobListFilter::default().with_prompt_id(beta.clone()));
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].prompt_id, beta);
}

#[test]
fn queue_and_history_actions_remove_only_requested_state_domains() {
    let mut bridge = ComfyJobBridge::default();
    let pending = submit_with_number(&mut bridge, "550e8400-e29b-41d4-a716-446655440030", 1.0);
    let running = submit_with_number(&mut bridge, "550e8400-e29b-41d4-a716-446655440031", 2.0);
    let completed = submit_with_number(&mut bridge, "550e8400-e29b-41d4-a716-446655440032", 3.0);
    bridge
        .update_status(&running, ComfyJobStatus::Running)
        .expect("job should update");
    bridge
        .update_status(&completed, ComfyJobStatus::Completed)
        .expect("job should update");

    bridge.apply_queue_action(QueueAction::Delete {
        prompt_ids: [pending.clone(), running.clone(), completed.clone()].into(),
    });

    assert!(bridge.record(&pending).is_none());
    assert!(bridge.record(&running).is_some());
    assert!(bridge.record(&completed).is_some());

    bridge.apply_history_action(HistoryAction::Clear);
    assert!(bridge.record(&running).is_some());
    assert!(bridge.record(&completed).is_none());
}

#[test]
fn duplicate_prompt_ids_are_rejected_before_overwriting_jobs() {
    let mut bridge = ComfyJobBridge::default();
    let prompt_id = prompt_id("550e8400-e29b-41d4-a716-446655440040");

    bridge
        .submit_prompt(PromptSubmission::new(json!({})).with_prompt_id(prompt_id.clone()))
        .expect("first submission should be accepted");
    let diagnostic = bridge
        .submit_prompt(PromptSubmission::new(json!({})).with_prompt_id(prompt_id.clone()))
        .expect_err("duplicate should be rejected");

    assert_eq!(diagnostic.code, DUPLICATE_JOB_CODE);
    assert_eq!(diagnostic.prompt_id, Some(prompt_id));
}

fn submit_with_number(
    bridge: &mut ComfyJobBridge,
    raw_prompt_id: &str,
    number: f64,
) -> ComfyPromptId {
    submit_for_client(bridge, raw_prompt_id, "client-a", number)
}

fn submit_for_client(
    bridge: &mut ComfyJobBridge,
    raw_prompt_id: &str,
    client_id: &str,
    number: f64,
) -> ComfyPromptId {
    let prompt_id = prompt_id(raw_prompt_id);
    bridge
        .submit_prompt(
            PromptSubmission::new(json!({"prompt": raw_prompt_id}))
                .with_prompt_id(prompt_id.clone())
                .with_client_id(client_id)
                .with_queue_number(QueueNumber(number))
                .with_extra_data(
                    PromptExtraData::default()
                        .with_public("workflow", raw_prompt_id)
                        .with_public("secret", "hidden")
                        .with_sensitive_key("secret"),
                ),
        )
        .expect("prompt should submit");
    prompt_id
}

fn prompt_id(value: &str) -> ComfyPromptId {
    ComfyPromptId::parse(value).expect("prompt id should parse")
}
