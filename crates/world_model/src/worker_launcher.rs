use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    DiagnosticCategory, ModelServingTarget, ServingBackend, ServingDiagnostic,
    ServingDiagnosticReport,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkerLaunchMode {
    Local,
    Persistent,
    Remote,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistentWorkerConfig {
    pub session_id: Option<String>,
    pub cache_key: Option<String>,
    pub fast_inference: bool,
    pub shutdown_after_idle_secs: Option<u64>,
}

impl PersistentWorkerConfig {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            ..Default::default()
        }
    }

    pub fn with_cache_key(mut self, cache_key: impl Into<String>) -> Self {
        self.cache_key = Some(cache_key.into());
        self
    }

    pub fn with_fast_inference(mut self, enabled: bool) -> Self {
        self.fast_inference = enabled;
        self
    }

    pub fn with_shutdown_after_idle_secs(mut self, secs: u64) -> Self {
        self.shutdown_after_idle_secs = Some(secs);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerLaunchRequest {
    pub target: ModelServingTarget,
    pub mode: WorkerLaunchMode,
    pub persistent: Option<PersistentWorkerConfig>,
    pub explicit_download_approved: bool,
    pub dependency_review_approved: bool,
}

impl WorkerLaunchRequest {
    pub fn new(target: ModelServingTarget, mode: WorkerLaunchMode) -> Self {
        Self {
            target,
            mode,
            persistent: None,
            explicit_download_approved: false,
            dependency_review_approved: false,
        }
    }

    pub fn with_persistent_config(mut self, config: PersistentWorkerConfig) -> Self {
        self.persistent = Some(config);
        self
    }

    pub fn with_explicit_download_approval(mut self, approved: bool) -> Self {
        self.explicit_download_approved = approved;
        self
    }

    pub fn with_dependency_review(mut self, approved: bool) -> Self {
        self.dependency_review_approved = approved;
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LocalWorkerEnvironment {
    pub python_available: bool,
    pub installed_packages: BTreeSet<String>,
    pub checkpoint_available: bool,
    pub gpu_vram_available_mib: Option<u64>,
    pub disk_available_mib: Option<u64>,
}

impl LocalWorkerEnvironment {
    pub fn with_python(mut self, available: bool) -> Self {
        self.python_available = available;
        self
    }

    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.installed_packages.insert(package.into());
        self
    }

    pub fn with_checkpoint(mut self, available: bool) -> Self {
        self.checkpoint_available = available;
        self
    }

    pub fn with_gpu_vram(mut self, mib: u64) -> Self {
        self.gpu_vram_available_mib = Some(mib);
        self
    }

    pub fn with_disk(mut self, mib: u64) -> Self {
        self.disk_available_mib = Some(mib);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RemoteWorkerEnvironment {
    pub endpoint_reachable: bool,
    pub auth_available: bool,
    pub capabilities: BTreeSet<String>,
    pub quota_remaining: Option<u64>,
}

impl RemoteWorkerEnvironment {
    pub fn with_endpoint_reachable(mut self, reachable: bool) -> Self {
        self.endpoint_reachable = reachable;
        self
    }

    pub fn with_auth(mut self, available: bool) -> Self {
        self.auth_available = available;
        self
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.insert(capability.into());
        self
    }

    pub fn with_quota_remaining(mut self, remaining: u64) -> Self {
        self.quota_remaining = Some(remaining);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerLaunchEnvironment {
    pub local: LocalWorkerEnvironment,
    pub remote: RemoteWorkerEnvironment,
}

impl WorkerLaunchEnvironment {
    pub fn with_local(mut self, local: LocalWorkerEnvironment) -> Self {
        self.local = local;
        self
    }

    pub fn with_remote(mut self, remote: RemoteWorkerEnvironment) -> Self {
        self.remote = remote;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorldModelWorkerLauncher;

impl WorldModelWorkerLauncher {
    pub fn validate(
        request: &WorkerLaunchRequest,
        environment: &WorkerLaunchEnvironment,
    ) -> ServingDiagnosticReport {
        let mut diagnostics = Vec::new();

        match request.mode {
            WorkerLaunchMode::Local => {
                validate_backend(
                    ServingBackend::Local,
                    request.target.backend,
                    &mut diagnostics,
                );
                validate_local(request, environment, &mut diagnostics);
            }
            WorkerLaunchMode::Persistent => {
                validate_backend(
                    ServingBackend::Local,
                    request.target.backend,
                    &mut diagnostics,
                );
                validate_local(request, environment, &mut diagnostics);
                validate_persistent(request, &mut diagnostics);
            }
            WorkerLaunchMode::Remote => {
                validate_backend(
                    ServingBackend::Remote,
                    request.target.backend,
                    &mut diagnostics,
                );
                validate_remote(request, environment, &mut diagnostics);
            }
        }

        ServingDiagnosticReport::with_diagnostics(diagnostics)
    }
}

fn validate_backend(
    expected: ServingBackend,
    actual: ServingBackend,
    diagnostics: &mut Vec<ServingDiagnostic>,
) {
    if actual != expected {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Other,
            format!("worker mode requires {expected:?} serving target, got {actual:?}"),
        ));
    }
}

fn validate_local(
    request: &WorkerLaunchRequest,
    environment: &WorkerLaunchEnvironment,
    diagnostics: &mut Vec<ServingDiagnostic>,
) {
    let config = &request.target.local_config;
    if config.python_path.is_none() || !environment.local.python_available {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Environment,
            "Python interpreter is not available for local model serving",
        ));
    }

    for package in &config.required_packages {
        if !environment.local.installed_packages.contains(package) {
            diagnostics.push(ServingDiagnostic::error(
                DiagnosticCategory::Package,
                format!("required Python package `{package}` is not available"),
            ));
        }
    }

    if (config.checkpoint_path.is_some() || config.checkpoint_file.is_some())
        && !environment.local.checkpoint_available
    {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Checkpoint,
            "configured model checkpoint is not available",
        ));
    }

    if let Some(required) = config.gpu_vram_mib {
        match environment.local.gpu_vram_available_mib {
            Some(available) if available >= required => {}
            Some(available) => diagnostics.push(ServingDiagnostic::error(
                DiagnosticCategory::Gpu,
                format!("GPU VRAM requirement is {required} MiB, only {available} MiB available"),
            )),
            None => diagnostics.push(ServingDiagnostic::error(
                DiagnosticCategory::Gpu,
                "GPU availability is unknown for local model serving",
            )),
        }
    }

    if let Some(required) = config.min_disk_mib {
        match environment.local.disk_available_mib {
            Some(available) if available >= required => {}
            Some(available) => diagnostics.push(ServingDiagnostic::error(
                DiagnosticCategory::Disk,
                format!("disk requirement is {required} MiB, only {available} MiB available"),
            )),
            None => diagnostics.push(ServingDiagnostic::error(
                DiagnosticCategory::Disk,
                "disk availability is unknown for local model serving",
            )),
        }
    }

    if config.allow_downloads && !request.explicit_download_approved {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Download,
            "model downloads require explicit user approval",
        ));
    }

    if requires_dependency_review(config) && !request.dependency_review_approved {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Download,
            "local worker dependencies require dependency review before launch",
        ));
    }
}

fn validate_persistent(request: &WorkerLaunchRequest, diagnostics: &mut Vec<ServingDiagnostic>) {
    let Some(config) = request.persistent.as_ref() else {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Environment,
            "persistent serving requires session configuration",
        ));
        return;
    };

    if config.session_id.as_deref().is_none_or(str::is_empty) {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Environment,
            "persistent serving requires a session id",
        ));
    }
    if config.fast_inference && config.cache_key.as_deref().is_none_or(str::is_empty) {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Other,
            "fast persistent serving requires a cache key",
        ));
    }
    if config.shutdown_after_idle_secs.is_none() {
        diagnostics.push(ServingDiagnostic::warning(
            DiagnosticCategory::Other,
            "persistent serving should configure idle shutdown",
        ));
    }
}

fn validate_remote(
    request: &WorkerLaunchRequest,
    environment: &WorkerLaunchEnvironment,
    diagnostics: &mut Vec<ServingDiagnostic>,
) {
    let config = &request.target.remote_config;
    if config.endpoint.as_deref().is_none_or(str::is_empty)
        || !environment.remote.endpoint_reachable
    {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Endpoint,
            "remote worker endpoint is not reachable",
        ));
    }
    if config.auth_method.as_deref().is_none_or(str::is_empty) || !environment.remote.auth_available
    {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Authentication,
            "remote worker authentication is not available",
        ));
    }
    for capability in &config.required_capabilities {
        if !environment.remote.capabilities.contains(capability) {
            diagnostics.push(ServingDiagnostic::error(
                DiagnosticCategory::Capability,
                format!("remote worker is missing capability `{capability}`"),
            ));
        }
    }
    if matches!(environment.remote.quota_remaining, Some(0)) {
        diagnostics.push(ServingDiagnostic::error(
            DiagnosticCategory::Quota,
            "remote worker quota is exhausted",
        ));
    }
}

fn requires_dependency_review(config: &crate::LocalServingConfig) -> bool {
    !config.required_packages.is_empty()
        || config.gpu_vram_mib.is_some()
        || config.checkpoint_path.is_some()
        || config.checkpoint_file.is_some()
}
