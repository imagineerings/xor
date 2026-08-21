#![cfg(feature = "metal")]

use comfy_tensor::MetalTensorBackend;
use comfy_types::{NativeBackendBinding, NativeBackendBindingStatus};

#[test]
fn generated_metal_semantic_adapter_is_feature_registered() {
    assert!(
        comfy_tensor::GENERATED_MODULES.contains(&"ops/backend_apple_metal_mps_comfy_model_0015")
    );
    assert_eq!(
        std::any::type_name::<MetalTensorBackend>(),
        "comfy_tensor::generated_backend_apple_metal_mps_comfy_model_0015::MetalTensorBackend"
    );
}

#[test]
fn production_accepts_only_certified_runtime_and_test_support_is_dev_only() {
    let source = include_str!("../src/backends/apple_metal_mps_comfy_model_0015.rs");
    assert!(source.contains("pub fn from_certified_runtime("));
    assert!(source.contains("runtime: MetalRuntime"));
    assert!(!source.contains("pub fn from_test"));
    assert!(!source.contains("std::process"));
    assert!(!source.contains("Command::new"));

    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest
            .lines()
            .any(|line| line == "metal = [\"cpu\", \"dep:comfy_backend_metal\"]")
    );
    assert!(
        manifest
            .contains("comfy_backend_metal = { workspace = true, features = [\"test-support\"] }")
    );
    assert!(include_str!("../../comfy_backend_metal/Cargo.toml").contains("test-support = []"));

    assert!(matches!(
        NativeBackendBinding::binding_status(&comfy_backend_metal::MetalBackend),
        NativeBackendBindingStatus::Unbound {
            device: comfy_types::DeviceKind::Metal,
            ..
        }
    ));
}
