use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{ComfyQuantizationMetadata, ModelFamilyKind, ModelFamilyProfile};

pub const UNSUPPORTED_PRECISION_CODE: &str = "world_model.runtime_policy.unsupported_precision";
pub const MISSING_QUANTIZATION_CODE: &str = "world_model.runtime_policy.missing_quantization";
pub const UNSUPPORTED_DEVICE_CODE: &str = "world_model.runtime_policy.unsupported_device";
pub const UNSUPPORTED_MEMORY_CODE: &str = "world_model.runtime_policy.unsupported_memory";
pub const EXPLICIT_DOWNLOAD_REQUIRED_CODE: &str =
    "world_model.runtime_policy.explicit_download_required";
pub const DEPENDENCY_REVIEW_REQUIRED_CODE: &str =
    "world_model.runtime_policy.dependency_review_required";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PrecisionPolicy {
    Fp32,
    Fp16,
    Bf16,
    Fp8,
    Quantized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DeviceBackend {
    Cpu,
    Cuda,
    Hip,
    DirectMl,
    OneApi,
    Ascend,
    Metal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum MemoryMode {
    GpuOnly,
    HighVram,
    LowVram,
    NoVram,
    DynamicVram,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimePolicyRequest {
    pub precision: PrecisionPolicy,
    pub device: DeviceBackend,
    pub memory: MemoryMode,
    pub multi_gpu: bool,
    pub async_offload: bool,
    pub pinned_memory: bool,
    pub mmap_weights: bool,
    pub release_cache_before_load: bool,
    pub model_available: bool,
    pub allow_downloads: bool,
    pub dependency_reviewed: bool,
}

impl RuntimePolicyRequest {
    pub fn new(precision: PrecisionPolicy, device: DeviceBackend, memory: MemoryMode) -> Self {
        Self {
            precision,
            device,
            memory,
            multi_gpu: false,
            async_offload: false,
            pinned_memory: false,
            mmap_weights: false,
            release_cache_before_load: false,
            model_available: true,
            allow_downloads: false,
            dependency_reviewed: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BackendSupport {
    pub devices: BTreeSet<DeviceBackend>,
    pub precisions: BTreeSet<PrecisionPolicy>,
    pub quantization: bool,
    pub dynamic_vram: bool,
    pub async_offload: bool,
    pub pinned_memory: bool,
    pub mmap_weights: bool,
    pub multi_gpu: bool,
}

impl BackendSupport {
    pub fn local_cuda() -> Self {
        Self {
            devices: [DeviceBackend::Cpu, DeviceBackend::Cuda]
                .into_iter()
                .collect(),
            precisions: [
                PrecisionPolicy::Fp32,
                PrecisionPolicy::Fp16,
                PrecisionPolicy::Bf16,
                PrecisionPolicy::Fp8,
                PrecisionPolicy::Quantized,
            ]
            .into_iter()
            .collect(),
            quantization: true,
            dynamic_vram: true,
            async_offload: true,
            pinned_memory: true,
            mmap_weights: true,
            multi_gpu: true,
        }
    }

    pub fn cpu_only() -> Self {
        Self {
            devices: [DeviceBackend::Cpu].into_iter().collect(),
            precisions: [PrecisionPolicy::Fp32].into_iter().collect(),
            quantization: false,
            dynamic_vram: false,
            async_offload: false,
            pinned_memory: false,
            mmap_weights: true,
            multi_gpu: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum RuntimePolicyDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimePolicyDiagnostic {
    pub code: String,
    pub severity: RuntimePolicyDiagnosticSeverity,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimRuntimePolicy {
    pub precision: PrecisionPolicy,
    pub device: DeviceBackend,
    pub memory: MemoryMode,
    pub multi_gpu: bool,
    pub async_offload: bool,
    pub pinned_memory: bool,
    pub mmap_weights: bool,
    pub release_cache_before_load: bool,
    pub quantization: Option<ComfyQuantizationMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimePolicyResolution {
    pub policy: Option<SimRuntimePolicy>,
    pub diagnostics: Vec<RuntimePolicyDiagnostic>,
}

impl RuntimePolicyResolution {
    pub fn is_ready(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != RuntimePolicyDiagnosticSeverity::Error)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimePolicyResolver;

impl RuntimePolicyResolver {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve(
        &self,
        model: &ModelFamilyProfile,
        quantization: Option<ComfyQuantizationMetadata>,
        request: RuntimePolicyRequest,
        support: &BackendSupport,
    ) -> RuntimePolicyResolution {
        let mut diagnostics = Vec::new();
        self.validate_download_policy(&request, &mut diagnostics);
        self.validate_device(&request, support, &mut diagnostics);
        self.validate_precision(
            model,
            quantization.as_ref(),
            &request,
            support,
            &mut diagnostics,
        );
        self.validate_memory(&request, support, &mut diagnostics);

        let has_errors = diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == RuntimePolicyDiagnosticSeverity::Error);
        let policy = if has_errors {
            None
        } else {
            Some(SimRuntimePolicy {
                precision: request.precision,
                device: request.device,
                memory: request.memory,
                multi_gpu: request.multi_gpu,
                async_offload: request.async_offload,
                pinned_memory: request.pinned_memory,
                mmap_weights: request.mmap_weights,
                release_cache_before_load: request.release_cache_before_load,
                quantization,
            })
        };

        RuntimePolicyResolution {
            policy,
            diagnostics,
        }
    }

    fn validate_download_policy(
        &self,
        request: &RuntimePolicyRequest,
        diagnostics: &mut Vec<RuntimePolicyDiagnostic>,
    ) {
        if request.model_available {
            return;
        }

        if !request.allow_downloads {
            diagnostics.push(error(
                EXPLICIT_DOWNLOAD_REQUIRED_CODE,
                "model weights are unavailable and downloads require explicit user action",
            ));
        } else if !request.dependency_reviewed {
            diagnostics.push(error(
                DEPENDENCY_REVIEW_REQUIRED_CODE,
                "model or package downloads require dependency review before execution",
            ));
        }
    }

    fn validate_device(
        &self,
        request: &RuntimePolicyRequest,
        support: &BackendSupport,
        diagnostics: &mut Vec<RuntimePolicyDiagnostic>,
    ) {
        if !support.devices.contains(&request.device) {
            diagnostics.push(error(
                UNSUPPORTED_DEVICE_CODE,
                format!("device backend {:?} is not available", request.device),
            ));
        }

        if request.multi_gpu && !support.multi_gpu {
            diagnostics.push(error(
                UNSUPPORTED_DEVICE_CODE,
                "multi-GPU execution is not supported by this backend",
            ));
        }
    }

    fn validate_precision(
        &self,
        model: &ModelFamilyProfile,
        quantization: Option<&ComfyQuantizationMetadata>,
        request: &RuntimePolicyRequest,
        support: &BackendSupport,
        diagnostics: &mut Vec<RuntimePolicyDiagnostic>,
    ) {
        if !support.precisions.contains(&request.precision) {
            diagnostics.push(error(
                UNSUPPORTED_PRECISION_CODE,
                format!(
                    "precision {:?} is not supported by this backend",
                    request.precision
                ),
            ));
        }

        match request.precision {
            PrecisionPolicy::Quantized => {
                if !support.quantization {
                    diagnostics.push(error(
                        UNSUPPORTED_PRECISION_CODE,
                        "quantized execution is not supported by this backend",
                    ));
                }
                if !quantization.is_some_and(|metadata| metadata.has_quantized_weights()) {
                    diagnostics.push(error(
                        MISSING_QUANTIZATION_CODE,
                        "quantized precision was selected but no quantization metadata was found",
                    ));
                }
            }
            PrecisionPolicy::Fp8 => {
                if model.family == ModelFamilyKind::StableDiffusion1 {
                    diagnostics.push(warning(
                        UNSUPPORTED_PRECISION_CODE,
                        "fp8 precision may be unsafe for Stable Diffusion 1 checkpoints",
                    ));
                }
            }
            PrecisionPolicy::Fp32 | PrecisionPolicy::Fp16 | PrecisionPolicy::Bf16 => {}
        }
    }

    fn validate_memory(
        &self,
        request: &RuntimePolicyRequest,
        support: &BackendSupport,
        diagnostics: &mut Vec<RuntimePolicyDiagnostic>,
    ) {
        if request.memory == MemoryMode::DynamicVram && !support.dynamic_vram {
            diagnostics.push(error(
                UNSUPPORTED_MEMORY_CODE,
                "dynamic VRAM is not supported by this backend",
            ));
        }

        if request.async_offload && !support.async_offload {
            diagnostics.push(error(
                UNSUPPORTED_MEMORY_CODE,
                "async offload is not supported by this backend",
            ));
        }
        if request.pinned_memory && !support.pinned_memory {
            diagnostics.push(error(
                UNSUPPORTED_MEMORY_CODE,
                "pinned memory is not supported by this backend",
            ));
        }
        if request.mmap_weights && !support.mmap_weights {
            diagnostics.push(error(
                UNSUPPORTED_MEMORY_CODE,
                "mmap weight loading is not supported by this backend",
            ));
        }
    }
}

fn error(code: &str, message: impl Into<String>) -> RuntimePolicyDiagnostic {
    RuntimePolicyDiagnostic {
        code: code.to_string(),
        severity: RuntimePolicyDiagnosticSeverity::Error,
        message: message.into(),
    }
}

fn warning(code: &str, message: impl Into<String>) -> RuntimePolicyDiagnostic {
    RuntimePolicyDiagnostic {
        code: code.to_string(),
        severity: RuntimePolicyDiagnosticSeverity::Warning,
        message: message.into(),
    }
}
