#[cfg(feature = "npu")]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/backends/huawei_ascend_npu_comfy_model_0019.rs"
));
