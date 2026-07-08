use crate::{
    SIM_VIDEO_DEPENDENCY_REVIEW_REQUIRED_CODE, SIM_VIDEO_INVALID_RANGE_CODE,
    SIM_VIDEO_UNSUPPORTED_BACKEND_CODE, SimVideoAdvancedOperation, SimVideoBackendStatus,
    SimVideoFrameRange, SimVideoMetadata, SimVideoNodeAdapter,
};

#[test]
fn video_adapter_load_create_and_save_preserve_metadata() {
    let adapter = SimVideoNodeAdapter::new();
    let loaded = adapter.load(
        "input://clip.mp4",
        SimVideoMetadata::new(1920, 1080, 240, 24, 1)
            .with_audio_reference("audio://track.wav")
            .with_field("camera", "wide"),
    );

    assert_eq!(
        loaded.metadata.audio_reference.as_deref(),
        Some("audio://track.wav")
    );
    assert_eq!(
        loaded.metadata.fields.get("camera").map(String::as_str),
        Some("wide")
    );
    assert_eq!(
        loaded
            .metadata
            .fields
            .get("sim.operation")
            .map(String::as_str),
        Some("load")
    );

    let created = adapter.create("generated://clip.mp4", 1280, 720, 48, 24, 1);
    assert_eq!(created.metadata.width, 1280);
    assert_eq!(created.metadata.frames, 48);

    let saved = adapter.save_as(&created, "output://clip.webm", "video/webm");
    assert_eq!(saved.reference, "output://clip.webm");
    assert_eq!(saved.metadata.mime_type, "video/webm");
}

#[test]
fn video_adapter_slices_and_decomposes_frame_ranges() {
    let adapter = SimVideoNodeAdapter::new();
    let artifact = adapter.create("video://source", 640, 480, 100, 30000, 1001);

    let range = SimVideoFrameRange {
        start: 10,
        end_exclusive: 40,
    };
    let sliced = adapter.slice(&artifact, range).expect("slice");
    assert_eq!(sliced.metadata.frames, 30);
    assert_eq!(
        sliced
            .metadata
            .fields
            .get("sim.frame_range")
            .map(String::as_str),
        Some("10..40")
    );

    let batch = adapter.decompose(&artifact, range).expect("decompose");
    assert_eq!(batch.source_reference, "video://source");
    assert_eq!(batch.frame_count, 30);
    assert_eq!(batch.frame_rate_num, 30000);
    assert_eq!(batch.frame_rate_den, 1001);
}

#[test]
fn video_adapter_rejects_invalid_frame_ranges() {
    let adapter = SimVideoNodeAdapter::new();
    let artifact = adapter.create("video://source", 640, 480, 10, 24, 1);

    for range in [
        SimVideoFrameRange {
            start: 2,
            end_exclusive: 2,
        },
        SimVideoFrameRange {
            start: 4,
            end_exclusive: 40,
        },
    ] {
        let diagnostic = adapter.slice(&artifact, range).expect_err("invalid range");
        assert_eq!(diagnostic.code, SIM_VIDEO_INVALID_RANGE_CODE);
    }
}

#[test]
fn video_adapter_reports_advanced_backend_diagnostics() {
    let adapter = SimVideoNodeAdapter::new();

    let reviewed = adapter
        .backend_diagnostic(
            SimVideoAdvancedOperation::FrameInterpolation,
            SimVideoBackendStatus::DependencyReviewRequired,
        )
        .expect("dependency diagnostic");
    assert_eq!(reviewed.code, SIM_VIDEO_DEPENDENCY_REVIEW_REQUIRED_CODE);
    assert_eq!(
        reviewed.operation,
        Some(SimVideoAdvancedOperation::FrameInterpolation)
    );

    let unsupported = adapter
        .backend_diagnostic(
            SimVideoAdvancedOperation::Segmentation,
            SimVideoBackendStatus::Unsupported,
        )
        .expect("unsupported diagnostic");
    assert_eq!(unsupported.code, SIM_VIDEO_UNSUPPORTED_BACKEND_CODE);

    assert!(
        adapter
            .backend_diagnostic(
                SimVideoAdvancedOperation::Merge,
                SimVideoBackendStatus::Native
            )
            .is_none()
    );
}
