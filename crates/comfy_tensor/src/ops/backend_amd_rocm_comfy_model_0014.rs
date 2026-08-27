#[cfg(feature = "rocm")]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/backends/amd_rocm_comfy_model_0014.rs"
));
