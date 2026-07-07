use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Serving backend
// ---------------------------------------------------------------------------

/// The type of model serving backend.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ServingBackend {
    /// Local Python-based serving (Comfy, Diffusers, etc.).
    #[default]
    Local,
    /// Remote API-based serving (Replicate, Fal, etc.).
    Remote,
}

// ---------------------------------------------------------------------------
// Local serving config
// ---------------------------------------------------------------------------

/// Configuration for local model serving (Req 9.1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LocalServingConfig {
    /// Path to the Python interpreter.
    pub python_path: Option<String>,
    /// Required Python packages (e.g., ["torch", "diffusers"]).
    pub required_packages: Vec<String>,
    /// Path to the model checkpoint directory.
    pub checkpoint_path: Option<String>,
    /// Path to a specific model checkpoint file.
    pub checkpoint_file: Option<String>,
    /// Required GPU VRAM in MiB (e.g., 8192 for 8 GB).
    pub gpu_vram_mib: Option<u64>,
    /// Minimum disk space required in MiB for model weights/temp files.
    pub min_disk_mib: Option<u64>,
    /// Whether downloads are permitted (Req 9.3 — not silently).
    pub allow_downloads: bool,
    /// Additional local config parameters.
    pub extra: HashMap<String, String>,
}

impl LocalServingConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_python(mut self, path: impl Into<String>) -> Self {
        self.python_path = Some(path.into());
        self
    }

    pub fn with_package(mut self, pkg: impl Into<String>) -> Self {
        self.required_packages.push(pkg.into());
        self
    }

    pub fn with_checkpoint(mut self, path: impl Into<String>) -> Self {
        self.checkpoint_path = Some(path.into());
        self
    }

    pub fn with_gpu_vram(mut self, mib: u64) -> Self {
        self.gpu_vram_mib = Some(mib);
        self
    }

    pub fn with_min_disk(mut self, mib: u64) -> Self {
        self.min_disk_mib = Some(mib);
        self
    }
}

// ---------------------------------------------------------------------------
// Remote serving config
// ---------------------------------------------------------------------------

/// Configuration for remote model serving (Req 9.2).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RemoteServingConfig {
    /// API endpoint URL.
    pub endpoint: Option<String>,
    /// Authentication method (e.g., "bearer", "api-key").
    pub auth_method: Option<String>,
    /// The model capabilities required (e.g., "text-to-image", "video").
    pub required_capabilities: Vec<String>,
    /// Maximum allowed quota per month (if tracked by the provider).
    pub quota_monthly: Option<u64>,
    /// Quota used this month.
    pub quota_used: Option<u64>,
    /// Additional remote config parameters.
    pub extra: HashMap<String, String>,
}

impl RemoteServingConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_endpoint(mut self, url: impl Into<String>) -> Self {
        self.endpoint = Some(url.into());
        self
    }

    pub fn with_auth(mut self, method: impl Into<String>) -> Self {
        self.auth_method = Some(method.into());
        self
    }

    pub fn with_capability(mut self, cap: impl Into<String>) -> Self {
        self.required_capabilities.push(cap.into());
        self
    }

    pub fn with_quota(mut self, monthly: u64, used: u64) -> Self {
        self.quota_monthly = Some(monthly);
        self.quota_used = Some(used);
        self
    }
}

// ---------------------------------------------------------------------------
// Model profile
// ---------------------------------------------------------------------------

/// Identifies a model family and variant for serving.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelProfile {
    /// Model family name (e.g., "stable-diffusion", "wan").
    pub family: String,
    /// Specific model variant (e.g., "sd-xl-base-1.0", "2.1b").
    pub variant: Option<String>,
    /// Model checkpoint file or path.
    pub checkpoint: Option<String>,
}

impl ModelProfile {
    pub fn new(family: impl Into<String>) -> Self {
        Self {
            family: family.into(),
            variant: None,
            checkpoint: None,
        }
    }

    pub fn with_variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    pub fn with_checkpoint(mut self, path: impl Into<String>) -> Self {
        self.checkpoint = Some(path.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Serving target
// ---------------------------------------------------------------------------

/// A complete serving target combining backend selection, model profile, and
/// backend-specific configuration.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelServingTarget {
    /// Serving backend type.
    pub backend: ServingBackend,
    /// Model identity.
    pub model: ModelProfile,
    /// Local serving configuration (used when backend == Local).
    pub local_config: LocalServingConfig,
    /// Remote serving configuration (used when backend == Remote).
    pub remote_config: RemoteServingConfig,
}

impl ModelServingTarget {
    pub fn new(backend: ServingBackend, model: ModelProfile) -> Self {
        Self {
            backend,
            model,
            ..Default::default()
        }
    }

    pub fn with_local_config(mut self, config: LocalServingConfig) -> Self {
        self.local_config = config;
        self
    }

    pub fn with_remote_config(mut self, config: RemoteServingConfig) -> Self {
        self.remote_config = config;
        self
    }
}
