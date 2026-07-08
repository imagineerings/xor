use serde_json::json;

use crate::{
    ComfyCancellationAction, ComfyCancellationController, ComfyCancellationRequest, ComfyJobBridge,
    ComfyJobStatus, ComfyPromptId, PromptSubmission, QueueNumber,
};

#[test]
fn cancellation_marks_pending_and_running_jobs_cancelled() {
    let mut bridge = ComfyJobBridge::default();
    let pending = submit_job(&mut bridge, "550e8400-e29b-41d4-a716-446655440100", 1.0);
    let running = submit_job(&mut bridge, "550e8400-e29b-41d4-a716-446655440101", 2.0);
    let completed = submit_job(&mut bridge, "550e8400-e29b-41d4-a716-446655440102", 3.0);
    let unknown = prompt_id("550e8400-e29b-41d4-a716-446655440103");
    bridge
        .update_status(&running, ComfyJobStatus::Running)
        .expect("running status should update");
    bridge
        .update_status(&completed, ComfyJobStatus::Completed)
        .expect("completed status should update");

    let report = ComfyCancellationController::cancel_many(
        &mut bridge,
        ComfyCancellationRequest::batch([
            pending.clone(),
            running.clone(),
            completed.clone(),
            unknown.clone(),
        ]),
    );

    assert_eq!(report.changed_count(), 2);
    assert_eq!(report.no_op_count(), 2);
    assert_eq!(
        bridge.record(&pending).unwrap().status,
        ComfyJobStatus::Cancelled
    );
    assert_eq!(
        bridge.record(&running).unwrap().status,
        ComfyJobStatus::Cancelled
    );
    assert_eq!(
        bridge.record(&completed).unwrap().status,
        ComfyJobStatus::Completed
    );
    assert!(report.outcomes.iter().any(|outcome| {
        outcome.prompt_id == completed
            && outcome.action
                == ComfyCancellationAction::TerminalNoOp {
                    status: ComfyJobStatus::Completed,
                }
    }));
    assert!(report.outcomes.iter().any(|outcome| {
        outcome.prompt_id == unknown && outcome.action == ComfyCancellationAction::UnknownNoOp
    }));
}

#[test]
fn repeated_cancellation_is_idempotent() {
    let mut bridge = ComfyJobBridge::default();
    let prompt_id = submit_job(&mut bridge, "550e8400-e29b-41d4-a716-446655440110", 1.0);

    let first = ComfyCancellationController::cancel_one(&mut bridge, prompt_id.clone());
    let second = ComfyCancellationController::cancel_one(&mut bridge, prompt_id.clone());

    assert_eq!(first.action, ComfyCancellationAction::CancelledPending);
    assert_eq!(
        second.action,
        ComfyCancellationAction::TerminalNoOp {
            status: ComfyJobStatus::Cancelled
        }
    );
    assert_eq!(
        bridge.record(&prompt_id).unwrap().status,
        ComfyJobStatus::Cancelled
    );
}

#[test]
fn targeted_interrupt_only_changes_matching_running_job() {
    let mut bridge = ComfyJobBridge::default();
    let pending = submit_job(&mut bridge, "550e8400-e29b-41d4-a716-446655440120", 1.0);
    let running = submit_job(&mut bridge, "550e8400-e29b-41d4-a716-446655440121", 2.0);
    let failed = submit_job(&mut bridge, "550e8400-e29b-41d4-a716-446655440122", 3.0);
    let unknown = prompt_id("550e8400-e29b-41d4-a716-446655440123");
    bridge
        .update_status(&running, ComfyJobStatus::Running)
        .expect("running status should update");
    bridge
        .update_status(&failed, ComfyJobStatus::Failed)
        .expect("failed status should update");

    let pending_outcome =
        ComfyCancellationController::interrupt_running(&mut bridge, pending.clone());
    let running_outcome =
        ComfyCancellationController::interrupt_running(&mut bridge, running.clone());
    let failed_outcome =
        ComfyCancellationController::interrupt_running(&mut bridge, failed.clone());
    let unknown_outcome = ComfyCancellationController::interrupt_running(&mut bridge, unknown);

    assert_eq!(
        pending_outcome.action,
        ComfyCancellationAction::NotRunningNoOp {
            status: ComfyJobStatus::Pending
        }
    );
    assert_eq!(
        running_outcome.action,
        ComfyCancellationAction::InterruptedRunning
    );
    assert_eq!(
        failed_outcome.action,
        ComfyCancellationAction::TerminalNoOp {
            status: ComfyJobStatus::Failed
        }
    );
    assert_eq!(unknown_outcome.action, ComfyCancellationAction::UnknownNoOp);
    assert_eq!(
        bridge.record(&pending).unwrap().status,
        ComfyJobStatus::Pending
    );
    assert_eq!(
        bridge.record(&running).unwrap().status,
        ComfyJobStatus::Cancelled
    );
}

#[test]
fn cancelled_jobs_leave_queue_and_enter_history() {
    let mut bridge = ComfyJobBridge::default();
    let pending = submit_job(&mut bridge, "550e8400-e29b-41d4-a716-446655440130", 1.0);
    let running = submit_job(&mut bridge, "550e8400-e29b-41d4-a716-446655440131", 2.0);
    bridge
        .update_status(&running, ComfyJobStatus::Running)
        .expect("running status should update");

    ComfyCancellationController::cancel_many(
        &mut bridge,
        ComfyCancellationRequest::batch([pending, running]),
    );

    let queue = bridge.queue_status();
    let history = bridge.history();

    assert!(queue.pending.is_empty());
    assert!(queue.running.is_empty());
    assert_eq!(queue.history_count, 2);
    assert_eq!(history.len(), 2);
    assert!(
        history
            .iter()
            .all(|job| job.status == ComfyJobStatus::Cancelled)
    );
}

fn submit_job(bridge: &mut ComfyJobBridge, raw_prompt_id: &str, number: f64) -> ComfyPromptId {
    let prompt_id = prompt_id(raw_prompt_id);
    bridge
        .submit_prompt(
            PromptSubmission::new(json!({"prompt": raw_prompt_id}))
                .with_prompt_id(prompt_id.clone())
                .with_queue_number(QueueNumber(number)),
        )
        .expect("prompt should submit");
    prompt_id
}

fn prompt_id(value: &str) -> ComfyPromptId {
    ComfyPromptId::parse(value).expect("prompt id should parse")
}
