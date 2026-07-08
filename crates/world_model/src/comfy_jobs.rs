use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ComfyJobStatus, ComfyJobSummary, ComfyPromptId, HistoryAction, PromptExtraData,
    PromptSubmission, PromptSubmissionResponse, QueueAction, QueueNumber, QueueStatus,
};

pub const DUPLICATE_JOB_CODE: &str = "world_model.comfy_jobs.duplicate_job";
pub const MISSING_JOB_CODE: &str = "world_model.comfy_jobs.missing_job";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyJobBridgeDiagnostic {
    pub code: String,
    pub message: String,
    pub prompt_id: Option<ComfyPromptId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimComfyJobRecord {
    pub prompt_id: ComfyPromptId,
    pub queue_number: u64,
    pub status: ComfyJobStatus,
    pub client_id: Option<String>,
    pub prompt: serde_json::Value,
    pub extra_data: PromptExtraData,
    pub outputs: Vec<String>,
}

impl SimComfyJobRecord {
    pub fn public_summary(&self, queue_position: Option<u64>) -> ComfyJobSummary {
        ComfyJobSummary {
            prompt_id: self.prompt_id.clone(),
            queue_position,
            status: self.status,
            client_id: self.client_id.clone(),
            outputs: self.outputs.clone(),
            public_extra_data: self.extra_data.redacted(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComfyJobSort {
    QueueAscending,
    QueueDescending,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComfyJobListFilter {
    pub statuses: BTreeSet<ComfyJobStatus>,
    pub client_id: Option<String>,
    pub prompt_ids: BTreeSet<ComfyPromptId>,
    pub sort: ComfyJobSort,
}

impl Default for ComfyJobListFilter {
    fn default() -> Self {
        Self {
            statuses: BTreeSet::new(),
            client_id: None,
            prompt_ids: BTreeSet::new(),
            sort: ComfyJobSort::QueueAscending,
        }
    }
}

impl ComfyJobListFilter {
    pub fn with_status(mut self, status: ComfyJobStatus) -> Self {
        self.statuses.insert(status);
        self
    }

    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    pub fn with_prompt_id(mut self, prompt_id: ComfyPromptId) -> Self {
        self.prompt_ids.insert(prompt_id);
        self
    }

    pub fn with_sort(mut self, sort: ComfyJobSort) -> Self {
        self.sort = sort;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfyJobBridge {
    jobs: BTreeMap<ComfyPromptId, SimComfyJobRecord>,
    next_queue_number: u64,
}

impl ComfyJobBridge {
    pub fn submit_prompt(
        &mut self,
        submission: PromptSubmission,
    ) -> Result<PromptSubmissionResponse, ComfyJobBridgeDiagnostic> {
        let prompt_id = match submission.prompt_id {
            Some(prompt_id) => prompt_id,
            None => ComfyPromptId::parse(&Uuid::new_v4().to_string()).map_err(|diagnostic| {
                ComfyJobBridgeDiagnostic {
                    code: diagnostic.code,
                    message: diagnostic.message,
                    prompt_id: None,
                }
            })?,
        };

        if self.jobs.contains_key(&prompt_id) {
            return Err(duplicate_job(prompt_id));
        }

        let queue_number = self.allocate_queue_number(submission.queue_number);
        let record = SimComfyJobRecord {
            prompt_id: prompt_id.clone(),
            queue_number,
            status: ComfyJobStatus::Pending,
            client_id: submission.client_id,
            prompt: submission.prompt,
            extra_data: submission.extra_data,
            outputs: Vec::new(),
        };
        self.jobs.insert(prompt_id.clone(), record);

        Ok(PromptSubmissionResponse {
            prompt_id,
            number: queue_number,
            node_errors: BTreeMap::new(),
        })
    }

    pub fn update_status(
        &mut self,
        prompt_id: &ComfyPromptId,
        status: ComfyJobStatus,
    ) -> Result<(), ComfyJobBridgeDiagnostic> {
        let job = self
            .jobs
            .get_mut(prompt_id)
            .ok_or_else(|| missing_job(prompt_id))?;
        job.status = status;
        Ok(())
    }

    pub fn add_output(
        &mut self,
        prompt_id: &ComfyPromptId,
        output: impl Into<String>,
    ) -> Result<(), ComfyJobBridgeDiagnostic> {
        let job = self
            .jobs
            .get_mut(prompt_id)
            .ok_or_else(|| missing_job(prompt_id))?;
        job.outputs.push(output.into());
        Ok(())
    }

    pub fn queue_status(&self) -> QueueStatus {
        let running = self.summaries_for_status(ComfyJobStatus::Running, false);
        let pending = self.summaries_for_status(ComfyJobStatus::Pending, true);
        let history_count = self
            .jobs
            .values()
            .filter(|job| job.status.is_terminal())
            .count() as u64;

        QueueStatus {
            running,
            pending,
            history_count,
        }
    }

    pub fn history(&self) -> Vec<ComfyJobSummary> {
        self.list_jobs(
            ComfyJobListFilter::default()
                .with_status(ComfyJobStatus::Completed)
                .with_status(ComfyJobStatus::Failed)
                .with_status(ComfyJobStatus::Cancelled),
        )
    }

    pub fn history_for_prompt(&self, prompt_id: &ComfyPromptId) -> Option<ComfyJobSummary> {
        self.jobs
            .get(prompt_id)
            .and_then(|job| job.status.is_terminal().then(|| job.public_summary(None)))
    }

    pub fn list_jobs(&self, filter: ComfyJobListFilter) -> Vec<ComfyJobSummary> {
        let mut jobs = self
            .jobs
            .values()
            .filter(|job| filter.statuses.is_empty() || filter.statuses.contains(&job.status))
            .filter(|job| {
                filter
                    .client_id
                    .as_ref()
                    .is_none_or(|client_id| job.client_id.as_ref() == Some(client_id))
            })
            .filter(|job| {
                filter.prompt_ids.is_empty() || filter.prompt_ids.contains(&job.prompt_id)
            })
            .collect::<Vec<_>>();

        jobs.sort_by(|left, right| compare_queue_numbers(left, right, filter.sort));
        let pending_positions = self.pending_positions();

        jobs.into_iter()
            .map(|job| job.public_summary(pending_positions.get(&job.prompt_id).copied()))
            .collect()
    }

    pub fn apply_queue_action(&mut self, action: QueueAction) {
        match action {
            QueueAction::Clear => {
                self.jobs
                    .retain(|_, job| job.status != ComfyJobStatus::Pending);
            }
            QueueAction::Delete { prompt_ids } => {
                self.jobs.retain(|prompt_id, job| {
                    job.status != ComfyJobStatus::Pending || !prompt_ids.contains(prompt_id)
                });
            }
        }
    }

    pub fn apply_history_action(&mut self, action: HistoryAction) {
        match action {
            HistoryAction::Clear => {
                self.jobs.retain(|_, job| !job.status.is_terminal());
            }
            HistoryAction::Delete { prompt_ids } => {
                self.jobs.retain(|prompt_id, job| {
                    !job.status.is_terminal() || !prompt_ids.contains(prompt_id)
                });
            }
        }
    }

    pub fn record(&self, prompt_id: &ComfyPromptId) -> Option<&SimComfyJobRecord> {
        self.jobs.get(prompt_id)
    }

    fn summaries_for_status(
        &self,
        status: ComfyJobStatus,
        include_queue_position: bool,
    ) -> Vec<ComfyJobSummary> {
        let positions = self.pending_positions();
        self.list_jobs(ComfyJobListFilter::default().with_status(status))
            .into_iter()
            .map(|mut summary| {
                if !include_queue_position {
                    summary.queue_position = None;
                } else {
                    summary.queue_position = positions.get(&summary.prompt_id).copied();
                }
                summary
            })
            .collect()
    }

    fn pending_positions(&self) -> BTreeMap<ComfyPromptId, u64> {
        self.list_pending_records()
            .into_iter()
            .enumerate()
            .map(|(index, job)| (job.prompt_id.clone(), index as u64 + 1))
            .collect()
    }

    fn list_pending_records(&self) -> Vec<&SimComfyJobRecord> {
        let mut jobs = self
            .jobs
            .values()
            .filter(|job| job.status == ComfyJobStatus::Pending)
            .collect::<Vec<_>>();
        jobs.sort_by(|left, right| {
            compare_queue_numbers(left, right, ComfyJobSort::QueueAscending)
        });
        jobs
    }

    fn allocate_queue_number(&mut self, requested: Option<QueueNumber>) -> u64 {
        if let Some(QueueNumber(number)) = requested {
            if number.is_finite() && number >= 0.0 {
                let normalized = number.floor() as u64;
                self.next_queue_number = self.next_queue_number.max(normalized.saturating_add(1));
                return normalized;
            }
        }

        let queue_number = self.next_queue_number;
        self.next_queue_number = self.next_queue_number.saturating_add(1);
        queue_number
    }
}

fn compare_queue_numbers(
    left: &SimComfyJobRecord,
    right: &SimComfyJobRecord,
    sort: ComfyJobSort,
) -> Ordering {
    let ordering = left.queue_number.cmp(&right.queue_number);
    match sort {
        ComfyJobSort::QueueAscending => ordering,
        ComfyJobSort::QueueDescending => ordering.reverse(),
    }
}

fn duplicate_job(prompt_id: ComfyPromptId) -> ComfyJobBridgeDiagnostic {
    ComfyJobBridgeDiagnostic {
        code: DUPLICATE_JOB_CODE.to_string(),
        message: format!(
            "prompt id `{}` already exists in the Sim job bridge",
            prompt_id.as_str()
        ),
        prompt_id: Some(prompt_id),
    }
}

fn missing_job(prompt_id: &ComfyPromptId) -> ComfyJobBridgeDiagnostic {
    ComfyJobBridgeDiagnostic {
        code: MISSING_JOB_CODE.to_string(),
        message: format!(
            "prompt id `{}` is not present in the Sim job bridge",
            prompt_id.as_str()
        ),
        prompt_id: Some(prompt_id.clone()),
    }
}
