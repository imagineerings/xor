use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{ComfyJobBridge, ComfyJobStatus, ComfyPromptId};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyCancellationRequest {
    pub prompt_ids: BTreeSet<ComfyPromptId>,
}

impl ComfyCancellationRequest {
    pub fn single(prompt_id: ComfyPromptId) -> Self {
        Self {
            prompt_ids: BTreeSet::from([prompt_id]),
        }
    }

    pub fn batch(prompt_ids: impl IntoIterator<Item = ComfyPromptId>) -> Self {
        Self {
            prompt_ids: prompt_ids.into_iter().collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyCancellationMode {
    Cancel,
    InterruptRunning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyCancellationAction {
    CancelledPending,
    InterruptedRunning,
    TerminalNoOp { status: ComfyJobStatus },
    NotRunningNoOp { status: ComfyJobStatus },
    UnknownNoOp,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyCancellationOutcome {
    pub prompt_id: ComfyPromptId,
    pub mode: ComfyCancellationMode,
    pub action: ComfyCancellationAction,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyCancellationReport {
    pub outcomes: Vec<ComfyCancellationOutcome>,
}

impl ComfyCancellationReport {
    pub fn changed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome.action,
                    ComfyCancellationAction::CancelledPending
                        | ComfyCancellationAction::InterruptedRunning
                )
            })
            .count()
    }

    pub fn no_op_count(&self) -> usize {
        self.outcomes.len().saturating_sub(self.changed_count())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComfyCancellationController;

impl ComfyCancellationController {
    pub fn cancel_one(
        bridge: &mut ComfyJobBridge,
        prompt_id: ComfyPromptId,
    ) -> ComfyCancellationOutcome {
        Self::cancel_prompt(bridge, prompt_id)
    }

    pub fn cancel_many(
        bridge: &mut ComfyJobBridge,
        request: ComfyCancellationRequest,
    ) -> ComfyCancellationReport {
        let outcomes = request
            .prompt_ids
            .into_iter()
            .map(|prompt_id| Self::cancel_prompt(bridge, prompt_id))
            .collect();

        ComfyCancellationReport { outcomes }
    }

    pub fn interrupt_running(
        bridge: &mut ComfyJobBridge,
        prompt_id: ComfyPromptId,
    ) -> ComfyCancellationOutcome {
        let action = match bridge.record(&prompt_id).map(|job| job.status) {
            Some(ComfyJobStatus::Running) => {
                if bridge
                    .update_status(&prompt_id, ComfyJobStatus::Cancelled)
                    .is_ok()
                {
                    ComfyCancellationAction::InterruptedRunning
                } else {
                    ComfyCancellationAction::UnknownNoOp
                }
            }
            Some(status) if status.is_terminal() => {
                ComfyCancellationAction::TerminalNoOp { status }
            }
            Some(status) => ComfyCancellationAction::NotRunningNoOp { status },
            None => ComfyCancellationAction::UnknownNoOp,
        };

        ComfyCancellationOutcome {
            prompt_id,
            mode: ComfyCancellationMode::InterruptRunning,
            action,
        }
    }

    fn cancel_prompt(
        bridge: &mut ComfyJobBridge,
        prompt_id: ComfyPromptId,
    ) -> ComfyCancellationOutcome {
        let action = match bridge.record(&prompt_id).map(|job| job.status) {
            Some(ComfyJobStatus::Pending) => {
                if bridge
                    .update_status(&prompt_id, ComfyJobStatus::Cancelled)
                    .is_ok()
                {
                    ComfyCancellationAction::CancelledPending
                } else {
                    ComfyCancellationAction::UnknownNoOp
                }
            }
            Some(ComfyJobStatus::Running) => {
                if bridge
                    .update_status(&prompt_id, ComfyJobStatus::Cancelled)
                    .is_ok()
                {
                    ComfyCancellationAction::InterruptedRunning
                } else {
                    ComfyCancellationAction::UnknownNoOp
                }
            }
            Some(status) if status.is_terminal() => {
                ComfyCancellationAction::TerminalNoOp { status }
            }
            Some(status) => ComfyCancellationAction::NotRunningNoOp { status },
            None => ComfyCancellationAction::UnknownNoOp,
        };

        ComfyCancellationOutcome {
            prompt_id,
            mode: ComfyCancellationMode::Cancel,
            action,
        }
    }
}
