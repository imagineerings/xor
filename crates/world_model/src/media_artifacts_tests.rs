use std::path::PathBuf;

use crate::{
    ArtifactRecord, ArtifactType, GENERATED_MEDIA_MISSING_PREVIEW_CODE,
    GENERATED_MEDIA_UNSUPPORTED_PREVIEW_CODE, GeneratedMediaPreviewKind,
    GeneratedMediaPreviewRouter, GeneratedWorldArtifact, GenerationProvenance,
    WorldGenerationRequest, WorldModelProfile,
};

#[test]
fn generated_media_preview_route_preserves_provenance_metadata() {
    let artifact = ArtifactRecord::new("outputs/frame.png", ArtifactType::Image)
        .with_preview("previews/frame.png");
    let provenance = GenerationProvenance::new(WorldGenerationRequest::new(
        "a sunset",
        WorldModelProfile::new("native-sim", "diffusion"),
        "outputs/frame.png",
    ))
    .with_artifact(artifact.clone())
    .with_backend("native-sim-worker")
    .with_workflow("image-preview");
    let generated = GeneratedWorldArtifact::new(artifact, provenance).expect("provenance");

    let route = GeneratedMediaPreviewRouter::new()
        .route(&generated)
        .expect("preview route");

    assert_eq!(route.kind, GeneratedMediaPreviewKind::Image);
    assert_eq!(route.artifact_path, PathBuf::from("outputs/frame.png"));
    assert_eq!(route.preview_path, PathBuf::from("previews/frame.png"));
    assert_eq!(
        route.provenance_backend.as_deref(),
        Some("native-sim-worker")
    );
    assert_eq!(route.provenance_workflow.as_deref(), Some("image-preview"));
}

#[test]
fn generated_media_preview_route_reports_missing_preview_metadata() {
    let mut artifact = ArtifactRecord::new("outputs/movie.mp4", ArtifactType::Video);
    artifact.preview_supported = true;
    let provenance = GenerationProvenance::new(WorldGenerationRequest::new(
        "a moving castle",
        WorldModelProfile::new("native-sim", "video"),
        "outputs/movie.mp4",
    ))
    .with_artifact(artifact.clone());
    let generated = GeneratedWorldArtifact::new(artifact, provenance).expect("provenance");

    let diagnostic = GeneratedMediaPreviewRouter::new()
        .route(&generated)
        .expect_err("missing preview should fail");

    assert_eq!(diagnostic.code, GENERATED_MEDIA_MISSING_PREVIEW_CODE);
    assert_eq!(diagnostic.artifact_path, PathBuf::from("outputs/movie.mp4"));
}

#[test]
fn generated_media_preview_route_reports_unsupported_artifact_type() {
    let artifact = ArtifactRecord::new("outputs/control.json", ArtifactType::Control)
        .with_preview("previews/control.json");
    let provenance = GenerationProvenance::new(WorldGenerationRequest::new(
        "control map",
        WorldModelProfile::new("native-sim", "control"),
        "outputs/control.json",
    ))
    .with_artifact(artifact.clone());
    let generated = GeneratedWorldArtifact::new(artifact, provenance).expect("provenance");

    let diagnostic = GeneratedMediaPreviewRouter::new()
        .route(&generated)
        .expect_err("unsupported generated artifact should fail");

    assert_eq!(diagnostic.code, GENERATED_MEDIA_UNSUPPORTED_PREVIEW_CODE);
    assert_eq!(
        diagnostic.artifact_path,
        PathBuf::from("outputs/control.json")
    );
}
