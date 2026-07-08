use std::{collections::BTreeMap, path::Path};

use crate::{
    BackendSupport, ComfyModelCatalog, ComfyModelFamilyDetector, ComfyModelFolderRegistry,
    ComfyQuantizationMetadata, DeviceBackend, MemoryMode, ModelCategory, PrecisionPolicy,
    RuntimePolicyDiagnosticSeverity, RuntimePolicyRequest, RuntimePolicyResolver,
    SafetensorsHeaderMetadata,
};

#[test]
fn quantization_parser_reads_global_and_layer_metadata() {
    let quantization = ComfyQuantizationMetadata::from_safetensors(&metadata([
        ("quantization.format", "fp8_e4m3fn"),
        ("quantization.layers.unet.input.format", "int8"),
        ("quantization.layers.unet.input.scale", "0.125"),
    ]))
    .expect("quantization metadata parsed");

    assert!(quantization.has_quantized_weights());
    assert_eq!(quantization.layers.len(), 1);
    assert_eq!(quantization.layers[0].layer_name, "unet.input");
    assert_eq!(quantization.layers[0].scale, Some("0.125".to_string()));
}

#[test]
fn resolver_accepts_supported_cuda_fp16_policy() {
    let model = sdxl_profile();
    let request = RuntimePolicyRequest::new(
        PrecisionPolicy::Fp16,
        DeviceBackend::Cuda,
        MemoryMode::HighVram,
    );
    let resolution =
        RuntimePolicyResolver::new().resolve(&model, None, request, &BackendSupport::local_cuda());

    assert!(resolution.is_ready());
    assert_eq!(
        resolution.policy.expect("policy created").precision,
        PrecisionPolicy::Fp16
    );
    assert!(resolution.diagnostics.is_empty());
}

#[test]
fn resolver_rejects_unsupported_backend_precision() {
    let model = sdxl_profile();
    let request = RuntimePolicyRequest::new(
        PrecisionPolicy::Fp16,
        DeviceBackend::Cpu,
        MemoryMode::NoVram,
    );
    let resolution =
        RuntimePolicyResolver::new().resolve(&model, None, request, &BackendSupport::cpu_only());

    assert!(!resolution.is_ready());
    assert!(resolution.policy.is_none());
    assert!(resolution.diagnostics.iter().any(
        |diagnostic| diagnostic.code == crate::comfy_runtime_policy::UNSUPPORTED_PRECISION_CODE
    ));
}

#[test]
fn resolver_requires_quantization_metadata_for_quantized_precision() {
    let model = sdxl_profile();
    let request = RuntimePolicyRequest::new(
        PrecisionPolicy::Quantized,
        DeviceBackend::Cuda,
        MemoryMode::LowVram,
    );
    let resolution =
        RuntimePolicyResolver::new().resolve(&model, None, request, &BackendSupport::local_cuda());

    assert!(!resolution.is_ready());
    assert!(resolution.diagnostics.iter().any(
        |diagnostic| diagnostic.code == crate::comfy_runtime_policy::MISSING_QUANTIZATION_CODE
    ));
}

#[test]
fn resolver_accepts_quantized_precision_when_metadata_exists() {
    let model = sdxl_profile();
    let quantization =
        ComfyQuantizationMetadata::from_safetensors(&metadata([("quantization.format", "nf4")]))
            .expect("quantization metadata parsed");
    let request = RuntimePolicyRequest::new(
        PrecisionPolicy::Quantized,
        DeviceBackend::Cuda,
        MemoryMode::LowVram,
    );
    let resolution = RuntimePolicyResolver::new().resolve(
        &model,
        Some(quantization),
        request,
        &BackendSupport::local_cuda(),
    );

    assert!(resolution.is_ready());
    assert!(
        resolution
            .policy
            .expect("policy created")
            .quantization
            .expect("quantization retained")
            .has_quantized_weights()
    );
}

#[test]
fn resolver_rejects_unavailable_device_and_multi_gpu() {
    let model = sdxl_profile();
    let mut request = RuntimePolicyRequest::new(
        PrecisionPolicy::Fp32,
        DeviceBackend::Cuda,
        MemoryMode::NoVram,
    );
    request.multi_gpu = true;
    let resolution =
        RuntimePolicyResolver::new().resolve(&model, None, request, &BackendSupport::cpu_only());

    assert!(!resolution.is_ready());
    assert_eq!(
        resolution
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code
                == crate::comfy_runtime_policy::UNSUPPORTED_DEVICE_CODE)
            .count(),
        2
    );
}

#[test]
fn resolver_rejects_unsupported_memory_options() {
    let model = sdxl_profile();
    let mut request = RuntimePolicyRequest::new(
        PrecisionPolicy::Fp32,
        DeviceBackend::Cpu,
        MemoryMode::DynamicVram,
    );
    request.async_offload = true;
    request.pinned_memory = true;
    let resolution =
        RuntimePolicyResolver::new().resolve(&model, None, request, &BackendSupport::cpu_only());

    assert!(!resolution.is_ready());
    assert!(
        resolution
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == RuntimePolicyDiagnosticSeverity::Error)
    );
    assert!(
        resolution
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code
                == crate::comfy_runtime_policy::UNSUPPORTED_MEMORY_CODE)
    );
}

#[test]
fn resolver_requires_explicit_download_and_dependency_review() {
    let model = sdxl_profile();
    let mut request = RuntimePolicyRequest::new(
        PrecisionPolicy::Fp16,
        DeviceBackend::Cuda,
        MemoryMode::HighVram,
    );
    request.model_available = false;

    let no_download = RuntimePolicyResolver::new().resolve(
        &model,
        None,
        request.clone(),
        &BackendSupport::local_cuda(),
    );
    assert!(
        no_download
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code
                == crate::comfy_runtime_policy::EXPLICIT_DOWNLOAD_REQUIRED_CODE)
    );

    request.allow_downloads = true;
    let no_review =
        RuntimePolicyResolver::new().resolve(&model, None, request, &BackendSupport::local_cuda());
    assert!(
        no_review
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code
                == crate::comfy_runtime_policy::DEPENDENCY_REVIEW_REQUIRED_CODE)
    );
}

#[test]
fn resolver_preserves_supported_memory_controls() {
    let model = sdxl_profile();
    let mut request = RuntimePolicyRequest::new(
        PrecisionPolicy::Fp8,
        DeviceBackend::Cuda,
        MemoryMode::DynamicVram,
    );
    request.async_offload = true;
    request.pinned_memory = true;
    request.mmap_weights = true;
    request.release_cache_before_load = true;
    let resolution =
        RuntimePolicyResolver::new().resolve(&model, None, request, &BackendSupport::local_cuda());

    assert!(resolution.is_ready());
    let policy = resolution.policy.as_ref().expect("policy created");
    assert_eq!(policy.memory, MemoryMode::DynamicVram);
    assert!(policy.async_offload);
    assert!(policy.pinned_memory);
    assert!(policy.mmap_weights);
    assert!(policy.release_cache_before_load);
}

fn sdxl_profile() -> crate::ModelFamilyProfile {
    let file = model_file(ModelCategory::Checkpoints, "sdxl.safetensors");
    ComfyModelFamilyDetector::new()
        .detect(
            &file,
            Some(&metadata([(
                "modelspec.architecture",
                "stable-diffusion-xl",
            )])),
        )
        .expect("sdxl profile")
}

fn model_file(category: ModelCategory, relative_path: &str) -> crate::ModelFileRef {
    let registry = ComfyModelFolderRegistry::new("/project/assets");
    let catalog = ComfyModelCatalog::new(&registry);
    catalog
        .resolve_at_root(category, 0, Path::new(relative_path))
        .expect("model resolves")
}

fn metadata(
    entries: impl IntoIterator<Item = (&'static str, &'static str)>,
) -> SafetensorsHeaderMetadata {
    SafetensorsHeaderMetadata {
        header_byte_len: 128,
        tensor_count: 1,
        metadata: entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>(),
    }
}
