use serde::{Deserialize, Serialize};

use crate::{DiagnosticCategory, DiagnosticSeverity, ServingDiagnostic, ServingDiagnosticReport};

pub const NO_RESOURCE_INTENTS_CODE: &str = "world_model.resources.no_intents";
pub const MISSING_UNLOAD_TARGETS_CODE: &str = "world_model.resources.missing_unload_targets";
pub const WORKER_RESOURCE_FAILURE_CODE: &str = "world_model.resources.worker_failure";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum FreeMemoryScope {
    ModelCache,
    GpuCache,
    All,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelResourceIntent {
    UnloadModels { model_keys: Vec<String> },
    FreeMemory { scope: FreeMemoryScope },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelResourceReleaseRequest {
    pub worker_id: String,
    pub intents: Vec<ModelResourceIntent>,
}

impl ModelResourceReleaseRequest {
    pub fn new(worker_id: impl Into<String>, intents: Vec<ModelResourceIntent>) -> Self {
        Self {
            worker_id: worker_id.into(),
            intents,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelResourceIntentResult {
    pub intent: ModelResourceIntent,
    pub released: bool,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelResourceReleaseReport {
    pub worker_id: String,
    pub results: Vec<ModelResourceIntentResult>,
    pub diagnostics: ServingDiagnosticReport,
}

impl ModelResourceReleaseReport {
    pub fn is_success(&self) -> bool {
        self.diagnostics.is_ready
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelResourceWorkerError {
    pub code: String,
    pub message: String,
}

impl ModelResourceWorkerError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub trait ModelResourceWorker {
    fn unload_models(
        &mut self,
        worker_id: &str,
        model_keys: &[String],
    ) -> Result<ModelResourceIntentResult, ModelResourceWorkerError>;

    fn free_memory(
        &mut self,
        worker_id: &str,
        scope: FreeMemoryScope,
    ) -> Result<ModelResourceIntentResult, ModelResourceWorkerError>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyModelResourceBridge;

impl ComfyModelResourceBridge {
    pub fn new() -> Self {
        Self
    }

    pub fn release<W: ModelResourceWorker>(
        &self,
        worker: &mut W,
        request: ModelResourceReleaseRequest,
    ) -> ModelResourceReleaseReport {
        let mut diagnostics = ServingDiagnosticReport::ready();
        let mut results = Vec::new();

        if request.intents.is_empty() {
            diagnostics.push(resource_error(
                NO_RESOURCE_INTENTS_CODE,
                "no model resource release intents were supplied",
            ));
        }

        for intent in &request.intents {
            match intent {
                ModelResourceIntent::UnloadModels { model_keys } if model_keys.is_empty() => {
                    diagnostics.push(resource_error(
                        MISSING_UNLOAD_TARGETS_CODE,
                        "unload-models intent requires at least one model key",
                    ));
                }
                ModelResourceIntent::UnloadModels { model_keys } => {
                    match worker.unload_models(&request.worker_id, model_keys) {
                        Ok(result) => results.push(result),
                        Err(error) => diagnostics.push(worker_error(error)),
                    }
                }
                ModelResourceIntent::FreeMemory { scope } => {
                    match worker.free_memory(&request.worker_id, *scope) {
                        Ok(result) => results.push(result),
                        Err(error) => diagnostics.push(worker_error(error)),
                    }
                }
            }
        }

        ModelResourceReleaseReport {
            worker_id: request.worker_id,
            results,
            diagnostics,
        }
    }
}

fn worker_error(error: ModelResourceWorkerError) -> ServingDiagnostic {
    ServingDiagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCategory::Gpu,
        error.message,
    )
    .with_detail(format!(
        "{WORKER_RESOURCE_FAILURE_CODE}: worker_error_code={}",
        error.code
    ))
}

fn resource_error(code: &str, message: impl Into<String>) -> ServingDiagnostic {
    ServingDiagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCategory::Capability,
        message,
    )
    .with_detail(code.to_string())
}
