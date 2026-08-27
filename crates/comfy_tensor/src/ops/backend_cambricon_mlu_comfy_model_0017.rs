#[cfg(feature = "mlu")]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/backends/cambricon_mlu_comfy_model_0017.rs"
));
