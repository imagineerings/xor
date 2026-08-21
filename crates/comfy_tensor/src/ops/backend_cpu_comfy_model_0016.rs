#[cfg(feature = "cpu")]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/backends/cpu_comfy_model_0016.rs"
));
