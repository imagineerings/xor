use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{DeviceBackend, MemoryMode, PrecisionPolicy, RuntimePolicyRequest, SimLaunchProfile};

pub const SIM_PACKAGING_PROFILE_CPU_ONLY: &str = "cpu-only";
pub const SIM_PACKAGING_PROFILE_CUDA_GPU: &str = "cuda-gpu";
pub const SIM_PACKAGING_PROFILE_METAL_GPU: &str = "metal-gpu";
pub const SIM_PACKAGING_PROFILE_API_DISABLED: &str = "api-disabled";
pub const SIM_PACKAGING_PROFILE_CUSTOM_NODE_DISABLED: &str = "custom-node-disabled";
pub const SIM_PACKAGING_PROFILE_ASSET_ENABLED: &str = "asset-enabled";
pub const SIM_PACKAGING_PROFILE_PORTABLE_LIKE: &str = "portable-like";
pub const SIM_PACKAGING_PROFILE_REMOTE_WORKER: &str = "remote-worker";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimPackagingProfileKind {
    CpuOnly,
    GpuSpecific,
    ApiDisabled,
    CustomNodeDisabled,
    AssetEnabled,
    PortableLike,
    RemoteWorker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimPackagingExecutionTarget {
    Local,
    RemoteWorker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimPackagingScope {
    LaunchProfileOnly,
    UsesExistingSimPlatformPackaging,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimPackagingProfile {
    pub id: String,
    pub name: String,
    pub kind: SimPackagingProfileKind,
    pub execution_target: SimPackagingExecutionTarget,
    pub packaging_scope: SimPackagingScope,
    pub launch_profile: SimLaunchProfile,
    pub notes: Vec<String>,
}

impl SimPackagingProfile {
    fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: SimPackagingProfileKind,
        launch_profile: SimLaunchProfile,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            execution_target: SimPackagingExecutionTarget::Local,
            packaging_scope: SimPackagingScope::LaunchProfileOnly,
            launch_profile,
            notes: Vec::new(),
        }
    }

    fn with_execution_target(mut self, execution_target: SimPackagingExecutionTarget) -> Self {
        self.execution_target = execution_target;
        self
    }

    fn with_packaging_scope(mut self, packaging_scope: SimPackagingScope) -> Self {
        self.packaging_scope = packaging_scope;
        self
    }

    fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimPackagingProfileCatalog {
    profiles: Vec<SimPackagingProfile>,
}

impl Default for SimPackagingProfileCatalog {
    fn default() -> Self {
        Self::new(default_profiles())
    }
}

impl SimPackagingProfileCatalog {
    pub fn new(profiles: impl IntoIterator<Item = SimPackagingProfile>) -> Self {
        let mut profiles = profiles.into_iter().collect::<Vec<_>>();
        profiles.sort_by(|left, right| left.id.cmp(&right.id));
        Self { profiles }
    }

    pub fn default_profiles() -> Self {
        Self::default()
    }

    pub fn profiles(&self) -> &[SimPackagingProfile] {
        &self.profiles
    }

    pub fn profile(&self, id: &str) -> Option<&SimPackagingProfile> {
        self.profiles.iter().find(|profile| profile.id == id)
    }

    pub fn ids(&self) -> BTreeSet<&str> {
        self.profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect()
    }
}

fn default_profiles() -> Vec<SimPackagingProfile> {
    vec![
        cpu_only_profile(),
        cuda_gpu_profile(),
        metal_gpu_profile(),
        api_disabled_profile(),
        custom_node_disabled_profile(),
        asset_enabled_profile(),
        portable_like_profile(),
        remote_worker_profile(),
    ]
}

fn cpu_only_profile() -> SimPackagingProfile {
    let mut launch_profile = SimLaunchProfile::default();
    launch_profile.runtime_policy = RuntimePolicyRequest::new(
        PrecisionPolicy::Fp32,
        DeviceBackend::Cpu,
        MemoryMode::NoVram,
    );
    launch_profile.performance.attention_backend = Some("sim-cpu".to_string());
    SimPackagingProfile::new(
        SIM_PACKAGING_PROFILE_CPU_ONLY,
        "CPU-only",
        SimPackagingProfileKind::CpuOnly,
        launch_profile,
    )
    .with_note("Disables GPU assumptions while leaving Sim platform packaging unchanged")
}

fn cuda_gpu_profile() -> SimPackagingProfile {
    let mut launch_profile = SimLaunchProfile::default();
    launch_profile.runtime_policy = RuntimePolicyRequest::new(
        PrecisionPolicy::Fp16,
        DeviceBackend::Cuda,
        MemoryMode::HighVram,
    );
    launch_profile.runtime_policy.pinned_memory = true;
    launch_profile.performance.attention_backend = Some("cuda".to_string());
    SimPackagingProfile::new(
        SIM_PACKAGING_PROFILE_CUDA_GPU,
        "CUDA GPU",
        SimPackagingProfileKind::GpuSpecific,
        launch_profile,
    )
}

fn metal_gpu_profile() -> SimPackagingProfile {
    let mut launch_profile = SimLaunchProfile::default();
    launch_profile.runtime_policy = RuntimePolicyRequest::new(
        PrecisionPolicy::Fp16,
        DeviceBackend::Metal,
        MemoryMode::DynamicVram,
    );
    launch_profile.performance.attention_backend = Some("metal".to_string());
    SimPackagingProfile::new(
        SIM_PACKAGING_PROFILE_METAL_GPU,
        "Metal GPU",
        SimPackagingProfileKind::GpuSpecific,
        launch_profile,
    )
}

fn api_disabled_profile() -> SimPackagingProfile {
    let mut launch_profile = SimLaunchProfile::default();
    launch_profile.api_nodes.enabled = false;
    SimPackagingProfile::new(
        SIM_PACKAGING_PROFILE_API_DISABLED,
        "API disabled",
        SimPackagingProfileKind::ApiDisabled,
        launch_profile,
    )
}

fn custom_node_disabled_profile() -> SimPackagingProfile {
    let mut launch_profile = SimLaunchProfile::default();
    launch_profile.custom_nodes.enabled = false;
    launch_profile.manager.enabled = false;
    launch_profile.manager.mode = "disabled".to_string();
    SimPackagingProfile::new(
        SIM_PACKAGING_PROFILE_CUSTOM_NODE_DISABLED,
        "Custom nodes disabled",
        SimPackagingProfileKind::CustomNodeDisabled,
        launch_profile,
    )
}

fn asset_enabled_profile() -> SimPackagingProfile {
    let mut launch_profile = SimLaunchProfile::default();
    launch_profile.assets.enabled = true;
    launch_profile
        .feature_flags
        .insert("assets".to_string(), "true".to_string());
    SimPackagingProfile::new(
        SIM_PACKAGING_PROFILE_ASSET_ENABLED,
        "Asset enabled",
        SimPackagingProfileKind::AssetEnabled,
        launch_profile,
    )
}

fn portable_like_profile() -> SimPackagingProfile {
    let mut launch_profile = SimLaunchProfile::default();
    launch_profile.directories.base_directory = Some(PathBuf::from("./sim"));
    launch_profile.directories.input_directory = Some(PathBuf::from("./sim/input"));
    launch_profile.directories.output_directory = Some(PathBuf::from("./sim/output"));
    launch_profile.directories.temp_directory = Some(PathBuf::from("./sim/temp"));
    launch_profile.directories.user_directory = Some(PathBuf::from("./sim/user"));
    launch_profile.database_url = Some("sqlite://./sim/user/sim.db".to_string());
    SimPackagingProfile::new(
        SIM_PACKAGING_PROFILE_PORTABLE_LIKE,
        "Portable-like",
        SimPackagingProfileKind::PortableLike,
        launch_profile,
    )
    .with_packaging_scope(SimPackagingScope::UsesExistingSimPlatformPackaging)
    .with_note("Defines relative runtime directories only; installer packaging remains external")
}

fn remote_worker_profile() -> SimPackagingProfile {
    let mut launch_profile = SimLaunchProfile::default();
    launch_profile.runtime_policy.model_available = false;
    launch_profile.runtime_policy.allow_downloads = false;
    launch_profile.cache.mode = "remote-worker".to_string();
    SimPackagingProfile::new(
        SIM_PACKAGING_PROFILE_REMOTE_WORKER,
        "Remote worker",
        SimPackagingProfileKind::RemoteWorker,
        launch_profile,
    )
    .with_execution_target(SimPackagingExecutionTarget::RemoteWorker)
    .with_packaging_scope(SimPackagingScope::UsesExistingSimPlatformPackaging)
    .with_note("Routes execution to remote worker infrastructure without packaging worker binaries")
}
