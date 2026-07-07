use crate::serving::{
    LocalServingConfig, ModelProfile, ModelServingTarget, RemoteServingConfig, ServingBackend,
};
use crate::serving_diagnostics::{
    DiagnosticCategory, DiagnosticSeverity, ServingDiagnostic, ServingDiagnosticReport,
};

// ---------------------------------------------------------------------------
// ServingBackend
// ---------------------------------------------------------------------------

#[test]
fn serving_backend_default() {
    let backend: ServingBackend = Default::default();
    assert_eq!(backend, ServingBackend::Local);
}

// ---------------------------------------------------------------------------
// LocalServingConfig
// ---------------------------------------------------------------------------

#[test]
fn local_config_default() {
    let config = LocalServingConfig::new();
    assert!(config.python_path.is_none());
    assert!(config.required_packages.is_empty());
    assert!(config.checkpoint_path.is_none());
}

#[test]
fn local_config_builder() {
    let config = LocalServingConfig::new()
        .with_python("/usr/bin/python3")
        .with_package("torch")
        .with_package("diffusers")
        .with_checkpoint("/models/sd-xl")
        .with_gpu_vram(8192)
        .with_min_disk(10240);
    assert_eq!(config.python_path.as_deref(), Some("/usr/bin/python3"));
    assert_eq!(config.required_packages.len(), 2);
    assert_eq!(config.checkpoint_path.as_deref(), Some("/models/sd-xl"));
    assert_eq!(config.gpu_vram_mib, Some(8192));
    assert_eq!(config.min_disk_mib, Some(10240));
}

// ---------------------------------------------------------------------------
// RemoteServingConfig
// ---------------------------------------------------------------------------

#[test]
fn remote_config_default() {
    let config = RemoteServingConfig::new();
    assert!(config.endpoint.is_none());
    assert!(config.auth_method.is_none());
}

#[test]
fn remote_config_builder() {
    let config = RemoteServingConfig::new()
        .with_endpoint("https://api.example.com/v1")
        .with_auth("bearer")
        .with_capability("text-to-image")
        .with_capability("video")
        .with_quota(1000, 42);
    assert_eq!(
        config.endpoint.as_deref(),
        Some("https://api.example.com/v1")
    );
    assert_eq!(config.auth_method.as_deref(), Some("bearer"));
    assert_eq!(config.required_capabilities.len(), 2);
    assert_eq!(config.quota_monthly, Some(1000));
    assert_eq!(config.quota_used, Some(42));
}

// ---------------------------------------------------------------------------
// ModelProfile
// ---------------------------------------------------------------------------

#[test]
fn model_profile_default() {
    let profile = ModelProfile::new("stable-diffusion");
    assert_eq!(profile.family, "stable-diffusion");
    assert!(profile.variant.is_none());
}

#[test]
fn model_profile_with_variant_and_checkpoint() {
    let profile = ModelProfile::new("wan")
        .with_variant("2.1b")
        .with_checkpoint("/ckpts/wan2.1b.safetensors");
    assert_eq!(profile.variant.as_deref(), Some("2.1b"));
    assert_eq!(
        profile.checkpoint.as_deref(),
        Some("/ckpts/wan2.1b.safetensors")
    );
}

// ---------------------------------------------------------------------------
// ModelServingTarget
// ---------------------------------------------------------------------------

#[test]
fn serving_target_creates() {
    let profile = ModelProfile::new("sd-xl");
    let target = ModelServingTarget::new(ServingBackend::Local, profile);
    assert_eq!(target.backend, ServingBackend::Local);
    assert_eq!(target.model.family, "sd-xl");
}

// ---------------------------------------------------------------------------
// DiagnosticCategory
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_category_labels() {
    assert_eq!(DiagnosticCategory::Environment.label(), "environment");
    assert_eq!(DiagnosticCategory::Package.label(), "package");
    assert_eq!(DiagnosticCategory::Checkpoint.label(), "checkpoint");
    assert_eq!(DiagnosticCategory::Gpu.label(), "gpu");
    assert_eq!(DiagnosticCategory::Disk.label(), "disk");
    assert_eq!(DiagnosticCategory::Endpoint.label(), "endpoint");
    assert_eq!(DiagnosticCategory::Authentication.label(), "authentication");
    assert_eq!(DiagnosticCategory::Capability.label(), "capability");
    assert_eq!(DiagnosticCategory::Quota.label(), "quota");
    assert_eq!(DiagnosticCategory::Download.label(), "download");
    assert_eq!(DiagnosticCategory::Other.label(), "other");
}

// ---------------------------------------------------------------------------
// ServingDiagnostic
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_create_with_severity_methods() {
    let err = ServingDiagnostic::error(DiagnosticCategory::Gpu, "CUDA not available");
    assert_eq!(err.severity, DiagnosticSeverity::Error);

    let warn = ServingDiagnostic::warning(DiagnosticCategory::Disk, "Less than 10 GiB free");
    assert_eq!(warn.severity, DiagnosticSeverity::Warning);

    let info = ServingDiagnostic::info(DiagnosticCategory::Package, "Using torch 2.1");
    assert_eq!(info.severity, DiagnosticSeverity::Info);
}

#[test]
fn diagnostic_with_detail() {
    let d = ServingDiagnostic::error(DiagnosticCategory::Checkpoint, "Checkpoint not found")
        .with_detail("Expected at /models/sdxl_v1.safetensors");
    assert_eq!(
        d.detail.as_deref(),
        Some("Expected at /models/sdxl_v1.safetensors")
    );
}

// ---------------------------------------------------------------------------
// ServingDiagnosticReport
// ---------------------------------------------------------------------------

#[test]
fn report_new_starts_not_ready() {
    let report = ServingDiagnosticReport::new();
    assert!(!report.is_ready);
    assert!(report.diagnostics.is_empty());
}

#[test]
fn report_ready_constructor() {
    let report = ServingDiagnosticReport::ready();
    assert!(report.is_ready);
    assert!(report.diagnostics.is_empty());
}

#[test]
fn report_with_diagnostics_computes_readiness() {
    let diags = vec![
        ServingDiagnostic::info(DiagnosticCategory::Environment, "Python 3.11"),
        ServingDiagnostic::error(DiagnosticCategory::Gpu, "No CUDA device"),
    ];
    let report = ServingDiagnosticReport::with_diagnostics(diags);
    assert!(!report.is_ready);
    assert_eq!(report.diagnostics.len(), 2);
}

#[test]
fn report_push_updates_readiness() {
    let mut report = ServingDiagnosticReport::ready();
    report.push(ServingDiagnostic::warning(
        DiagnosticCategory::Disk,
        "Low disk",
    ));
    assert!(report.is_ready); // warning doesn't block
    report.push(ServingDiagnostic::error(DiagnosticCategory::Gpu, "No GPU"));
    assert!(!report.is_ready);
}

#[test]
fn report_merge_combines() {
    let mut r1 = ServingDiagnosticReport::ready();
    let r2 = ServingDiagnosticReport::with_diagnostics(vec![ServingDiagnostic::error(
        DiagnosticCategory::Endpoint,
        "Timeout",
    )]);
    r1.merge(r2);
    assert!(!r1.is_ready);
    assert_eq!(r1.diagnostics.len(), 1);
}

#[test]
fn report_error_and_warning_iterators() {
    let report = ServingDiagnosticReport::with_diagnostics(vec![
        ServingDiagnostic::error(DiagnosticCategory::Gpu, "No GPU"),
        ServingDiagnostic::warning(DiagnosticCategory::Disk, "Low disk"),
        ServingDiagnostic::info(DiagnosticCategory::Package, "Using torch"),
    ]);
    assert_eq!(report.errors().count(), 1);
    assert_eq!(report.warnings().count(), 1);
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

#[test]
fn serving_target_round_trip_serde() {
    let target = ModelServingTarget::new(
        ServingBackend::Local,
        ModelProfile::new("sd-xl").with_variant("base"),
    )
    .with_local_config(LocalServingConfig::new().with_gpu_vram(8192));
    let json = serde_json::to_string(&target).expect("serialize");
    let restored: ModelServingTarget = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.model.family, "sd-xl");
    assert_eq!(restored.backend, ServingBackend::Local);
    assert_eq!(restored.local_config.gpu_vram_mib, Some(8192));
}

#[test]
fn diagnostic_report_round_trip_serde() {
    let report = ServingDiagnosticReport::with_diagnostics(vec![ServingDiagnostic::error(
        DiagnosticCategory::Gpu,
        "No CUDA",
    )]);
    let json = serde_json::to_string(&report).expect("serialize");
    let restored: ServingDiagnosticReport = serde_json::from_str(&json).expect("deserialize");
    assert!(!restored.is_ready);
    assert_eq!(restored.diagnostics.len(), 1);
}
