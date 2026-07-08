use crate::{
    SIM_CONTROL_SIGNAL_DEPENDENCY_REVIEW_REQUIRED_CODE, SIM_CONTROL_SIGNAL_MISSING_METADATA_CODE,
    SIM_CONTROL_SIGNAL_TYPE_MISMATCH_CODE, SIM_CONTROL_SIGNAL_UNSUPPORTED_BACKEND_CODE,
    SimControlSignalBackendStatus, SimControlSignalKind, SimControlSignalMetadata,
    SimControlSignalNodeAdapter, SimControlTargetKind, SimMediaPortType,
};

#[test]
fn control_signal_adapter_exposes_typed_outputs() {
    let adapter = SimControlSignalNodeAdapter::new();

    let pose = adapter
        .analyze(
            "control://pose",
            SimControlSignalKind::Pose,
            SimControlSignalMetadata::new("image://frame.png", 1024, 768)
                .with_confidence_basis("openpose")
                .with_frames(1),
        )
        .expect("pose signal");
    assert_eq!(pose.port_type, SimMediaPortType::Pose);
    assert_eq!(pose.metadata.frames, Some(1));
    assert_eq!(pose.metadata.confidence_basis.as_deref(), Some("openpose"));

    let boxes = adapter
        .analyze(
            "control://boxes",
            SimControlSignalKind::Detection,
            SimControlSignalMetadata::new("image://frame.png", 1024, 768),
        )
        .expect("detection signal");
    assert_eq!(boxes.port_type, SimMediaPortType::BoundingBoxes);

    let depth = adapter
        .analyze(
            "control://depth",
            SimControlSignalKind::Depth,
            SimControlSignalMetadata::new("image://frame.png", 1024, 768),
        )
        .expect("depth signal");
    assert_eq!(depth.port_type, SimMediaPortType::DepthMap);
}

#[test]
fn control_signal_adapter_validates_generation_target_compatibility() {
    let adapter = SimControlSignalNodeAdapter::new();
    let pose = adapter
        .analyze(
            "control://pose",
            SimControlSignalKind::Pose,
            SimControlSignalMetadata::new("image://frame.png", 512, 512),
        )
        .expect("pose signal");
    adapter
        .validate_compatibility(&pose, SimControlTargetKind::PoseToImage)
        .expect("pose-to-image");
    adapter
        .validate_compatibility(&pose, SimControlTargetKind::ControlNet)
        .expect("controlnet accepts pose");

    let diagnostic = adapter
        .validate_compatibility(&pose, SimControlTargetKind::DepthToVideo)
        .expect_err("pose cannot feed depth target");
    assert_eq!(diagnostic.code, SIM_CONTROL_SIGNAL_TYPE_MISMATCH_CODE);
    assert_eq!(diagnostic.signal_kind, Some(SimControlSignalKind::Pose));
    assert_eq!(
        diagnostic.target_kind,
        Some(SimControlTargetKind::DepthToVideo)
    );
}

#[test]
fn control_signal_adapter_rejects_missing_metadata() {
    let adapter = SimControlSignalNodeAdapter::new();
    let diagnostic = adapter
        .analyze(
            "control://bad",
            SimControlSignalKind::Canny,
            SimControlSignalMetadata::new("", 0, 512),
        )
        .expect_err("missing metadata");
    assert_eq!(diagnostic.code, SIM_CONTROL_SIGNAL_MISSING_METADATA_CODE);
}

#[test]
fn control_signal_adapter_reports_backend_diagnostics() {
    let adapter = SimControlSignalNodeAdapter::new();

    let reviewed = adapter
        .backend_diagnostic(
            SimControlSignalKind::Segmentation,
            SimControlSignalBackendStatus::DependencyReviewRequired,
            "SAM3 segmentation",
        )
        .expect("review diagnostic");
    assert_eq!(
        reviewed.code,
        SIM_CONTROL_SIGNAL_DEPENDENCY_REVIEW_REQUIRED_CODE
    );

    let unsupported = adapter
        .backend_diagnostic(
            SimControlSignalKind::Tracking,
            SimControlSignalBackendStatus::Unsupported,
            "multi-object tracking",
        )
        .expect("unsupported diagnostic");
    assert_eq!(
        unsupported.code,
        SIM_CONTROL_SIGNAL_UNSUPPORTED_BACKEND_CODE
    );

    assert!(
        adapter
            .backend_diagnostic(
                SimControlSignalKind::Depth,
                SimControlSignalBackendStatus::Native,
                "depth"
            )
            .is_none()
    );
}
