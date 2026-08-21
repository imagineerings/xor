#[cfg(feature = "cuda")]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/backends/nvidia_cuda_comfy_model_0022.rs"
));

