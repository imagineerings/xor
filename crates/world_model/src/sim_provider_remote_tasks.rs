use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{SimProviderConnectorError, SimProviderId};

pub const SIM_PROVIDER_TASK_MISSING_CODE: &str = "world_model.provider_tasks.missing";
pub const SIM_PROVIDER_TASK_TIMEOUT_CODE: &str = "world_model.provider_tasks.timeout";
pub const SIM_PROVIDER_TASK_TERMINAL_UPDATE_CODE: &str =
    "world_model.provider_tasks.terminal_update";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SimProviderRemoteTaskId(String);

impl SimProviderRemoteTaskId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SimProviderRemoteTaskStatus {
    Queued,
    Running {
        progress: Option<f32>,
        message: Option<String>,
    },
    Completed {
        output_refs: Vec<String>,
    },
    Failed {
        message: String,
    },
    Cancelled {
        message: String,
    },
    TimedOut {
        message: String,
    },
}

impl SimProviderRemoteTaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. }
                | Self::Failed { .. }
                | Self::Cancelled { .. }
                | Self::TimedOut { .. }
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderRemoteTaskHandle {
    pub provider_id: SimProviderId,
    pub remote_task_id: SimProviderRemoteTaskId,
    pub comfy_node_id: String,
    pub native_handler: String,
}

impl SimProviderRemoteTaskHandle {
    pub fn new(
        provider_id: SimProviderId,
        remote_task_id: impl Into<String>,
        comfy_node_id: impl Into<String>,
        native_handler: impl Into<String>,
    ) -> Self {
        Self {
            provider_id,
            remote_task_id: SimProviderRemoteTaskId::new(remote_task_id),
            comfy_node_id: comfy_node_id.into(),
            native_handler: native_handler.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimProviderRemoteTaskRecord {
    pub handle: SimProviderRemoteTaskHandle,
    pub status: SimProviderRemoteTaskStatus,
    pub started_at_ms: u64,
    pub updated_at_ms: u64,
    pub timeout_at_ms: Option<u64>,
    pub provider_progress: Option<f32>,
}

impl SimProviderRemoteTaskRecord {
    pub fn new(handle: SimProviderRemoteTaskHandle, started_at_ms: u64) -> Self {
        Self {
            handle,
            status: SimProviderRemoteTaskStatus::Queued,
            started_at_ms,
            updated_at_ms: started_at_ms,
            timeout_at_ms: None,
            provider_progress: None,
        }
    }

    pub fn with_timeout_at_ms(mut self, timeout_at_ms: u64) -> Self {
        self.timeout_at_ms = Some(timeout_at_ms);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderRemoteTaskDiagnostic {
    pub code: String,
    pub provider_id: SimProviderId,
    pub remote_task_id: Option<SimProviderRemoteTaskId>,
    pub message: String,
}

impl SimProviderRemoteTaskDiagnostic {
    fn new(
        code: impl Into<String>,
        provider_id: SimProviderId,
        remote_task_id: Option<SimProviderRemoteTaskId>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            provider_id,
            remote_task_id,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimProviderRemoteTaskTracker {
    records: BTreeMap<SimProviderRemoteTaskId, SimProviderRemoteTaskRecord>,
}

impl SimProviderRemoteTaskTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, record: SimProviderRemoteTaskRecord) {
        self.records
            .insert(record.handle.remote_task_id.clone(), record);
    }

    pub fn record(
        &self,
        remote_task_id: &SimProviderRemoteTaskId,
    ) -> Option<&SimProviderRemoteTaskRecord> {
        self.records.get(remote_task_id)
    }

    pub fn update_status(
        &mut self,
        remote_task_id: &SimProviderRemoteTaskId,
        status: SimProviderRemoteTaskStatus,
        updated_at_ms: u64,
    ) -> Result<(), SimProviderRemoteTaskDiagnostic> {
        let Some(record) = self.records.get_mut(remote_task_id) else {
            return Err(SimProviderRemoteTaskDiagnostic::new(
                SIM_PROVIDER_TASK_MISSING_CODE,
                SimProviderId::new("unknown"),
                Some(remote_task_id.clone()),
                "provider remote task is not registered",
            ));
        };

        if record.status.is_terminal() {
            return Err(SimProviderRemoteTaskDiagnostic::new(
                SIM_PROVIDER_TASK_TERMINAL_UPDATE_CODE,
                record.handle.provider_id.clone(),
                Some(remote_task_id.clone()),
                "terminal provider task status cannot be updated",
            ));
        }

        record.provider_progress = match &status {
            SimProviderRemoteTaskStatus::Running { progress, .. } => *progress,
            _ => record.provider_progress,
        };
        record.status = status;
        record.updated_at_ms = updated_at_ms;
        Ok(())
    }

    pub fn apply_connector_error(
        &mut self,
        handle: &SimProviderRemoteTaskHandle,
        error: SimProviderConnectorError,
        updated_at_ms: u64,
    ) -> Result<(), SimProviderRemoteTaskDiagnostic> {
        self.update_status(
            &handle.remote_task_id,
            SimProviderRemoteTaskStatus::Failed {
                message: error.message,
            },
            updated_at_ms,
        )
    }

    pub fn expire_timed_out(&mut self, now_ms: u64) -> Vec<SimProviderRemoteTaskDiagnostic> {
        let mut diagnostics = Vec::new();
        for record in self.records.values_mut() {
            if record.status.is_terminal() {
                continue;
            }
            let Some(timeout_at_ms) = record.timeout_at_ms else {
                continue;
            };
            if now_ms < timeout_at_ms {
                continue;
            }
            record.status = SimProviderRemoteTaskStatus::TimedOut {
                message: "provider remote task timed out".to_string(),
            };
            record.updated_at_ms = now_ms;
            diagnostics.push(SimProviderRemoteTaskDiagnostic::new(
                SIM_PROVIDER_TASK_TIMEOUT_CODE,
                record.handle.provider_id.clone(),
                Some(record.handle.remote_task_id.clone()),
                "provider remote task timed out",
            ));
        }
        diagnostics
    }
}
