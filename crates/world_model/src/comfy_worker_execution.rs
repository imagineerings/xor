use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{ModelFamilyKind, SamplingProgress, SamplingRunRequest};

pub const WORKER_UNSUPPORTED_FAMILY_CODE: &str = "world_model.worker.unsupported_family";
pub const WORKER_DETERMINISM_UNSUPPORTED_CODE: &str = "world_model.worker.determinism_unsupported";
pub const WORKER_PREVIEW_UNSUPPORTED_CODE: &str = "world_model.worker.preview_unsupported";
pub const WORKER_CANCEL_UNSUPPORTED_CODE: &str = "world_model.worker.cancel_unsupported";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerCapabilityProfile {
    pub supported_families: BTreeSet<ModelFamilyKind>,
    pub supports_previews: bool,
    pub supports_cancellation: bool,
    pub supports_determinism: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerExecutionRequest {
    pub job_id: String,
    pub sampling: SamplingRunRequest,
    pub previews_requested: bool,
    pub cancellation_requested: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WorkerTerminalState {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerPreview {
    pub step: u32,
    pub artifact_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerOutputArtifact {
    pub artifact_ref: String,
    pub media_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerExecutionReport {
    pub job_id: String,
    pub terminal_state: WorkerTerminalState,
    pub progress: Vec<SamplingProgress>,
    pub previews: Vec<WorkerPreview>,
    pub outputs: Vec<WorkerOutputArtifact>,
    pub provenance: Vec<String>,
    pub diagnostics: Vec<WorkerExecutionDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerExecutionDiagnostic {
    pub code: String,
    pub message: String,
}

pub trait ComfyWorker {
    fn capabilities(&self) -> WorkerCapabilityProfile;
    fn execute(
        &mut self,
        request: WorkerExecutionRequest,
    ) -> Result<WorkerExecutionReport, WorkerExecutionDiagnostic>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyWorkerExecutionAdapter;

impl ComfyWorkerExecutionAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        worker: &mut impl ComfyWorker,
        request: WorkerExecutionRequest,
    ) -> Result<WorkerExecutionReport, Vec<WorkerExecutionDiagnostic>> {
        let capabilities = worker.capabilities();
        let mut diagnostics = Vec::new();

        if !capabilities
            .supported_families
            .contains(&request.sampling.family_profile.family)
        {
            diagnostics.push(diagnostic(
                WORKER_UNSUPPORTED_FAMILY_CODE,
                format!(
                    "worker does not support model family {:?}",
                    request.sampling.family_profile.family
                ),
            ));
        }
        if request.sampling.deterministic.is_some() && !capabilities.supports_determinism {
            diagnostics.push(diagnostic(
                WORKER_DETERMINISM_UNSUPPORTED_CODE,
                "worker does not support deterministic execution",
            ));
        }
        if request.previews_requested && !capabilities.supports_previews {
            diagnostics.push(diagnostic(
                WORKER_PREVIEW_UNSUPPORTED_CODE,
                "worker does not support preview events",
            ));
        }
        if request.cancellation_requested && !capabilities.supports_cancellation {
            diagnostics.push(diagnostic(
                WORKER_CANCEL_UNSUPPORTED_CODE,
                "worker does not support cancellation",
            ));
        }

        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }

        worker
            .execute(request)
            .map_err(|diagnostic| vec![diagnostic])
    }
}

fn diagnostic(code: &str, message: impl Into<String>) -> WorkerExecutionDiagnostic {
    WorkerExecutionDiagnostic {
        code: code.to_string(),
        message: message.into(),
    }
}
