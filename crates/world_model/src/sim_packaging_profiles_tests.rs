use std::path::PathBuf;

use crate::{
    DeviceBackend, MemoryMode, PrecisionPolicy, SIM_PACKAGING_PROFILE_API_DISABLED,
    SIM_PACKAGING_PROFILE_ASSET_ENABLED, SIM_PACKAGING_PROFILE_CPU_ONLY,
    SIM_PACKAGING_PROFILE_CUDA_GPU, SIM_PACKAGING_PROFILE_CUSTOM_NODE_DISABLED,
    SIM_PACKAGING_PROFILE_METAL_GPU, SIM_PACKAGING_PROFILE_PORTABLE_LIKE,
    SIM_PACKAGING_PROFILE_REMOTE_WORKER, SimPackagingExecutionTarget, SimPackagingProfileCatalog,
    SimPackagingProfileKind, SimPackagingScope,
};

#[test]
fn packaging_catalog_contains_required_native_sim_profiles() {
    let catalog = SimPackagingProfileCatalog::default_profiles();
    let ids = catalog.ids();

    for required in [
        SIM_PACKAGING_PROFILE_CPU_ONLY,
        SIM_PACKAGING_PROFILE_CUDA_GPU,
        SIM_PACKAGING_PROFILE_METAL_GPU,
        SIM_PACKAGING_PROFILE_API_DISABLED,
        SIM_PACKAGING_PROFILE_CUSTOM_NODE_DISABLED,
        SIM_PACKAGING_PROFILE_ASSET_ENABLED,
        SIM_PACKAGING_PROFILE_PORTABLE_LIKE,
        SIM_PACKAGING_PROFILE_REMOTE_WORKER,
    ] {
        assert!(
            ids.contains(required),
            "missing packaging profile {required}"
        );
    }
    assert_eq!(ids.len(), catalog.profiles().len());
}

#[test]
fn packaging_catalog_configures_cpu_and_gpu_runtime_profiles() {
    let catalog = SimPackagingProfileCatalog::default_profiles();

    let cpu = catalog
        .profile(SIM_PACKAGING_PROFILE_CPU_ONLY)
        .expect("cpu profile");
    assert_eq!(cpu.kind, SimPackagingProfileKind::CpuOnly);
    assert_eq!(cpu.launch_profile.runtime_policy.device, DeviceBackend::Cpu);
    assert_eq!(
        cpu.launch_profile.runtime_policy.precision,
        PrecisionPolicy::Fp32
    );
    assert_eq!(cpu.launch_profile.runtime_policy.memory, MemoryMode::NoVram);

    let cuda = catalog
        .profile(SIM_PACKAGING_PROFILE_CUDA_GPU)
        .expect("cuda profile");
    assert_eq!(cuda.kind, SimPackagingProfileKind::GpuSpecific);
    assert_eq!(
        cuda.launch_profile.runtime_policy.device,
        DeviceBackend::Cuda
    );
    assert_eq!(
        cuda.launch_profile.runtime_policy.precision,
        PrecisionPolicy::Fp16
    );
    assert!(cuda.launch_profile.runtime_policy.pinned_memory);

    let metal = catalog
        .profile(SIM_PACKAGING_PROFILE_METAL_GPU)
        .expect("metal profile");
    assert_eq!(
        metal.launch_profile.runtime_policy.device,
        DeviceBackend::Metal
    );
    assert_eq!(
        metal.launch_profile.runtime_policy.memory,
        MemoryMode::DynamicVram
    );
}

#[test]
fn packaging_catalog_configures_api_custom_node_and_asset_modes() {
    let catalog = SimPackagingProfileCatalog::default_profiles();

    let api_disabled = catalog
        .profile(SIM_PACKAGING_PROFILE_API_DISABLED)
        .expect("api disabled profile");
    assert!(!api_disabled.launch_profile.api_nodes.enabled);

    let custom_node_disabled = catalog
        .profile(SIM_PACKAGING_PROFILE_CUSTOM_NODE_DISABLED)
        .expect("custom node disabled profile");
    assert!(!custom_node_disabled.launch_profile.custom_nodes.enabled);
    assert!(!custom_node_disabled.launch_profile.manager.enabled);

    let asset_enabled = catalog
        .profile(SIM_PACKAGING_PROFILE_ASSET_ENABLED)
        .expect("asset enabled profile");
    assert!(asset_enabled.launch_profile.assets.enabled);
    assert_eq!(
        asset_enabled
            .launch_profile
            .feature_flags
            .get("assets")
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn packaging_catalog_models_portable_like_without_platform_packaging_duplication() {
    let catalog = SimPackagingProfileCatalog::default_profiles();
    let portable = catalog
        .profile(SIM_PACKAGING_PROFILE_PORTABLE_LIKE)
        .expect("portable profile");

    assert_eq!(portable.kind, SimPackagingProfileKind::PortableLike);
    assert_eq!(
        portable.packaging_scope,
        SimPackagingScope::UsesExistingSimPlatformPackaging
    );
    assert_eq!(
        portable.launch_profile.directories.user_directory,
        Some(PathBuf::from("./sim/user"))
    );
    assert!(
        portable
            .notes
            .iter()
            .any(|note| note.contains("installer packaging remains external"))
    );
}

#[test]
fn packaging_catalog_models_remote_worker_as_launch_profile_only() {
    let catalog = SimPackagingProfileCatalog::default_profiles();
    let remote = catalog
        .profile(SIM_PACKAGING_PROFILE_REMOTE_WORKER)
        .expect("remote profile");

    assert_eq!(remote.kind, SimPackagingProfileKind::RemoteWorker);
    assert_eq!(
        remote.execution_target,
        SimPackagingExecutionTarget::RemoteWorker
    );
    assert_eq!(
        remote.packaging_scope,
        SimPackagingScope::UsesExistingSimPlatformPackaging
    );
    assert!(!remote.launch_profile.runtime_policy.model_available);
    assert!(!remote.launch_profile.runtime_policy.allow_downloads);
}
