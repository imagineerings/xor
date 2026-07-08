use std::collections::BTreeSet;

use crate::{
    ComfyExecutionRegistry, ComfySamplingRequestBuilder, ComfyWorker, ComfyWorkerExecutionAdapter,
    DenoiseRange, DeviceBackend, LatentDescriptor, ModelFamilyKind, NoisePolicy, PrecisionPolicy,
    SamplingNodeKind, SamplingProgress, SamplingRunInput, WorkerCapabilityProfile,
    WorkerExecutionDiagnostic, WorkerExecutionReport, WorkerExecutionRequest, WorkerOutputArtifact,
    WorkerPreview, WorkerTerminalState,
};

#[test]
fn adapter_rejects_worker_capability_mismatches_before_execution() {
    let mut worker = MockWorker {
        capabilities: WorkerCapabilityProfile {
            supported_families: BTreeSet::from([ModelFamilyKind::Flux]),
            supports_previews: false,
            supports_cancellation: false,
            supports_determinism: false,
        },
        executed: false,
    };

    let diagnostics = ComfyWorkerExecutionAdapter::new()
        .execute(&mut worker, worker_request(true, true))
        .expect_err("capability mismatches rejected");

    assert!(!worker.executed);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_worker_execution::WORKER_UNSUPPORTED_FAMILY_CODE
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_worker_execution::WORKER_DETERMINISM_UNSUPPORTED_CODE
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_worker_execution::WORKER_PREVIEW_UNSUPPORTED_CODE
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == crate::comfy_worker_execution::WORKER_CANCEL_UNSUPPORTED_CODE
    }));
}

#[test]
fn adapter_maps_progress_previews_outputs_terminal_state_and_provenance() {
    let mut worker = MockWorker {
        capabilities: WorkerCapabilityProfile {
            supported_families: BTreeSet::from([ModelFamilyKind::StableDiffusionXl]),
            supports_previews: true,
            supports_cancellation: true,
            supports_determinism: true,
        },
        executed: false,
    };

    let report = ComfyWorkerExecutionAdapter::new()
        .execute(&mut worker, worker_request(true, false))
        .expect("worker executes");

    assert!(worker.executed);
    assert_eq!(report.terminal_state, WorkerTerminalState::Completed);
    assert_eq!(report.progress[0].current_step, 1);
    assert_eq!(report.previews[0].artifact_ref, "preview://1");
    assert_eq!(report.outputs[0].artifact_ref, "artifact://final");
    assert!(report.provenance.iter().any(|item| item == "seed=42"));
}

struct MockWorker {
    capabilities: WorkerCapabilityProfile,
    executed: bool,
}

impl ComfyWorker for MockWorker {
    fn capabilities(&self) -> WorkerCapabilityProfile {
        self.capabilities.clone()
    }

    fn execute(
        &mut self,
        request: WorkerExecutionRequest,
    ) -> Result<WorkerExecutionReport, WorkerExecutionDiagnostic> {
        self.executed = true;
        Ok(WorkerExecutionReport {
            job_id: request.job_id,
            terminal_state: if request.cancellation_requested {
                WorkerTerminalState::Cancelled
            } else {
                WorkerTerminalState::Completed
            },
            progress: vec![SamplingProgress {
                current_step: 1,
                total_steps: request.sampling.steps,
                preview_available: request.previews_requested,
                cancellation_requested: request.cancellation_requested,
            }],
            previews: vec![WorkerPreview {
                step: 1,
                artifact_ref: "preview://1".to_string(),
            }],
            outputs: vec![WorkerOutputArtifact {
                artifact_ref: "artifact://final".to_string(),
                media_type: "image/png".to_string(),
            }],
            provenance: vec![
                format!("seed={}", request.sampling.seed),
                format!("sampler={:?}", request.sampling.sampler),
            ],
            diagnostics: Vec::new(),
        })
    }
}

fn worker_request(
    previews_requested: bool,
    cancellation_requested: bool,
) -> WorkerExecutionRequest {
    let registry = ComfyExecutionRegistry::new();
    let family = registry
        .model_family(ModelFamilyKind::StableDiffusionXl)
        .expect("sdxl family");
    let sampling = ComfySamplingRequestBuilder::new()
        .build(&registry, family, sampling_input())
        .expect("sampling request builds");

    WorkerExecutionRequest {
        job_id: "job-1".to_string(),
        sampling,
        previews_requested,
        cancellation_requested,
    }
}

fn sampling_input() -> SamplingRunInput {
    SamplingRunInput {
        node_kind: SamplingNodeKind::KSampler,
        sampler_name: "dpm++ 2m".to_string(),
        scheduler_name: "karras".to_string(),
        guidance_name: "classifier_free".to_string(),
        seed: 42,
        noise_policy: NoisePolicy::Fixed { noise_seed: 123 },
        steps: 20,
        cfg_scale: 7.0,
        denoise: DenoiseRange {
            amount: 1.0,
            start_step: None,
            end_step: None,
        },
        latent: LatentDescriptor {
            width: 1024,
            height: 1024,
            channels: 4,
            frames: None,
        },
        positive_conditioning: "positive".to_string(),
        negative_conditioning: Some("negative".to_string()),
        model_profile: "sdxl".to_string(),
        model_hash: Some("model-hash".to_string()),
        deterministic: true,
        worker_supports_determinism: true,
        backend: DeviceBackend::Cuda,
        precision: PrecisionPolicy::Fp16,
    }
}
