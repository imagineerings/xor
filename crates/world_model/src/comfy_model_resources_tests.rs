use crate::{
    ComfyModelResourceBridge, DiagnosticCategory, FreeMemoryScope, ModelResourceIntent,
    ModelResourceIntentResult, ModelResourceReleaseRequest, ModelResourceWorker,
    ModelResourceWorkerError,
};

#[derive(Default)]
struct RecordingWorker {
    calls: Vec<String>,
    fail_free_memory: bool,
}

impl ModelResourceWorker for RecordingWorker {
    fn unload_models(
        &mut self,
        worker_id: &str,
        model_keys: &[String],
    ) -> Result<ModelResourceIntentResult, ModelResourceWorkerError> {
        self.calls
            .push(format!("unload:{worker_id}:{}", model_keys.join(",")));
        Ok(ModelResourceIntentResult {
            intent: ModelResourceIntent::UnloadModels {
                model_keys: model_keys.to_vec(),
            },
            released: true,
            detail: Some(format!("unloaded {} models", model_keys.len())),
        })
    }

    fn free_memory(
        &mut self,
        worker_id: &str,
        scope: FreeMemoryScope,
    ) -> Result<ModelResourceIntentResult, ModelResourceWorkerError> {
        self.calls.push(format!("free:{worker_id}:{scope:?}"));
        if self.fail_free_memory {
            return Err(ModelResourceWorkerError::new(
                "cuda_oom",
                "worker failed to release GPU memory",
            ));
        }

        Ok(ModelResourceIntentResult {
            intent: ModelResourceIntent::FreeMemory { scope },
            released: true,
            detail: Some("memory released".to_string()),
        })
    }
}

#[test]
fn bridge_wires_unload_and_free_memory_intents_to_worker() {
    let mut worker = RecordingWorker::default();
    let report = ComfyModelResourceBridge::new().release(
        &mut worker,
        ModelResourceReleaseRequest::new(
            "worker-1",
            vec![
                ModelResourceIntent::UnloadModels {
                    model_keys: vec!["checkpoints/sdxl.safetensors".to_string()],
                },
                ModelResourceIntent::FreeMemory {
                    scope: FreeMemoryScope::GpuCache,
                },
            ],
        ),
    );

    assert!(report.is_success());
    assert_eq!(
        worker.calls,
        vec![
            "unload:worker-1:checkpoints/sdxl.safetensors",
            "free:worker-1:GpuCache"
        ]
    );
    assert_eq!(report.results.len(), 2);
}

#[test]
fn bridge_reports_missing_intents_as_diagnostic_failure() {
    let mut worker = RecordingWorker::default();
    let report = ComfyModelResourceBridge::new().release(
        &mut worker,
        ModelResourceReleaseRequest::new("worker-1", Vec::new()),
    );

    assert!(!report.is_success());
    assert!(worker.calls.is_empty());
    assert_eq!(
        report.diagnostics.diagnostics[0].detail.as_deref(),
        Some(crate::comfy_model_resources::NO_RESOURCE_INTENTS_CODE)
    );
}

#[test]
fn bridge_reports_missing_unload_targets_without_calling_worker() {
    let mut worker = RecordingWorker::default();
    let report = ComfyModelResourceBridge::new().release(
        &mut worker,
        ModelResourceReleaseRequest::new(
            "worker-1",
            vec![ModelResourceIntent::UnloadModels {
                model_keys: Vec::new(),
            }],
        ),
    );

    assert!(!report.is_success());
    assert!(worker.calls.is_empty());
    assert_eq!(
        report.diagnostics.diagnostics[0].detail.as_deref(),
        Some(crate::comfy_model_resources::MISSING_UNLOAD_TARGETS_CODE)
    );
}

#[test]
fn bridge_maps_worker_failure_to_gpu_diagnostic() {
    let mut worker = RecordingWorker {
        fail_free_memory: true,
        ..Default::default()
    };
    let report = ComfyModelResourceBridge::new().release(
        &mut worker,
        ModelResourceReleaseRequest::new(
            "worker-1",
            vec![ModelResourceIntent::FreeMemory {
                scope: FreeMemoryScope::All,
            }],
        ),
    );

    assert!(!report.is_success());
    let diagnostic = &report.diagnostics.diagnostics[0];
    assert_eq!(diagnostic.category, DiagnosticCategory::Gpu);
    assert_eq!(diagnostic.message, "worker failed to release GPU memory");
    assert!(diagnostic.detail.as_deref().is_some_and(|detail| {
        detail.contains(crate::comfy_model_resources::WORKER_RESOURCE_FAILURE_CODE)
    }));
}
