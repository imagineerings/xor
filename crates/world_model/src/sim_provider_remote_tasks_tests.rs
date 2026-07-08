use crate::{
    SIM_PROVIDER_CONNECTOR_CANCEL_UNSUPPORTED_CODE, SIM_PROVIDER_TASK_TERMINAL_UPDATE_CODE,
    SIM_PROVIDER_TASK_TIMEOUT_CODE, SimMockProviderConnector, SimProviderCapability,
    SimProviderConnector, SimProviderId, SimProviderPolicyRequest, SimProviderRemoteTaskRecord,
    SimProviderRemoteTaskStatus, SimProviderRemoteTaskTracker,
};

#[test]
fn provider_connector_starts_and_polls_native_remote_task() {
    let mut connector = SimMockProviderConnector::new(
        SimProviderId::new("runway"),
        vec![SimProviderCapability::TextToVideo],
    )
    .with_status(SimProviderRemoteTaskStatus::Running {
        progress: Some(0.25),
        message: Some("rendering".to_string()),
    })
    .with_status(SimProviderRemoteTaskStatus::Completed {
        output_refs: vec!["asset:video-1".to_string()],
    });
    let request = SimProviderPolicyRequest::new(
        SimProviderId::new("runway"),
        SimProviderCapability::TextToVideo,
        "RunwayTextToVideo",
        "sim.provider.runway.RunwayTextToVideo",
    );

    let handle = connector.start(request).expect("remote task starts");
    assert_eq!(handle.remote_task_id.as_str(), "sim-remote-task-1");

    let mut tracker = SimProviderRemoteTaskTracker::new();
    tracker.register(SimProviderRemoteTaskRecord::new(handle.clone(), 10));
    tracker
        .update_status(&handle.remote_task_id, connector.poll(&handle).unwrap(), 20)
        .expect("running status updates");
    assert_eq!(
        tracker
            .record(&handle.remote_task_id)
            .unwrap()
            .provider_progress,
        Some(0.25)
    );
    tracker
        .update_status(&handle.remote_task_id, connector.poll(&handle).unwrap(), 30)
        .expect("completed status updates");
    assert!(
        tracker
            .record(&handle.remote_task_id)
            .unwrap()
            .status
            .is_terminal()
    );
}

#[test]
fn provider_tracker_rejects_updates_after_terminal_status() {
    let handle = handle();
    let mut tracker = SimProviderRemoteTaskTracker::new();
    tracker.register(SimProviderRemoteTaskRecord::new(handle.clone(), 10));
    tracker
        .update_status(
            &handle.remote_task_id,
            SimProviderRemoteTaskStatus::Completed {
                output_refs: vec!["asset:image-1".to_string()],
            },
            20,
        )
        .expect("terminal status updates");

    let diagnostic = tracker
        .update_status(
            &handle.remote_task_id,
            SimProviderRemoteTaskStatus::Running {
                progress: Some(0.9),
                message: None,
            },
            30,
        )
        .expect_err("terminal updates should fail");

    assert_eq!(diagnostic.code, SIM_PROVIDER_TASK_TERMINAL_UPDATE_CODE);
}

#[test]
fn provider_tracker_expires_timed_out_tasks() {
    let handle = handle();
    let mut tracker = SimProviderRemoteTaskTracker::new();
    tracker.register(SimProviderRemoteTaskRecord::new(handle.clone(), 10).with_timeout_at_ms(50));

    assert!(tracker.expire_timed_out(49).is_empty());
    let diagnostics = tracker.expire_timed_out(50);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, SIM_PROVIDER_TASK_TIMEOUT_CODE);
    assert!(matches!(
        tracker.record(&handle.remote_task_id).unwrap().status,
        SimProviderRemoteTaskStatus::TimedOut { .. }
    ));
}

#[test]
fn provider_connector_reports_unsupported_cancellation() {
    let mut connector = SimMockProviderConnector::new(
        SimProviderId::new("openai"),
        vec![SimProviderCapability::TextToImage],
    )
    .with_cancellation_supported(false);
    let handle = handle();

    let error = connector
        .cancel(&handle)
        .expect_err("cancellation should be unsupported");

    assert_eq!(error.code, SIM_PROVIDER_CONNECTOR_CANCEL_UNSUPPORTED_CODE);
}

fn handle() -> crate::SimProviderRemoteTaskHandle {
    crate::SimProviderRemoteTaskHandle::new(
        SimProviderId::new("openai"),
        "remote-1",
        "OpenAIImageGenerate",
        "sim.provider.openai.OpenAIImageGenerate",
    )
}
