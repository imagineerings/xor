use crate::{
    ArtifactRecord, ArtifactType, GenerationProvenance, ProvenanceCollection, WorldActionControl,
    WorldControl, WorldGenerationRequest, WorldModelProfile,
};

// ---------------------------------------------------------------------------
// WorldModelProfile
// ---------------------------------------------------------------------------

#[test]
fn profile_creates_with_name_and_family() {
    let profile = WorldModelProfile::new("sd-xl", "stable-diffusion");
    assert_eq!(profile.name, "sd-xl");
    assert_eq!(profile.family, "stable-diffusion");
    assert!(profile.variant.is_none());
}

#[test]
fn profile_with_variant() {
    let profile = WorldModelProfile::new("sd-xl", "stable-diffusion").with_variant("fp16");
    assert_eq!(profile.variant.as_deref(), Some("fp16"));
}

// ---------------------------------------------------------------------------
// WorldActionControl
// ---------------------------------------------------------------------------

#[test]
fn action_control_creates_with_name_value_frame() {
    let ctrl = WorldActionControl::new("w", 1.0, 42);
    assert_eq!(ctrl.name, "w");
    assert_eq!(ctrl.value, 1.0);
    assert_eq!(ctrl.frame, 42);
}

// ---------------------------------------------------------------------------
// WorldControl
// ---------------------------------------------------------------------------

#[test]
fn control_validate_accepts_valid_wasd() {
    let control = WorldControl::new(
        vec![
            WorldActionControl::new("w", 1.0, 0),
            WorldActionControl::new("d", 0.5, 0),
        ],
        60,
    );
    let errors = control.validate();
    assert!(errors.is_empty(), "Expected no errors, got: {errors:?}");
}

#[test]
fn control_validate_rejects_opposing_wasd() {
    let control = WorldControl::new(
        vec![
            WorldActionControl::new("w", 1.0, 0),
            WorldActionControl::new("s", 0.8, 0),
        ],
        60,
    );
    let errors = control.validate();
    assert!(
        errors.iter().any(|e| e.contains("W") && e.contains("S")),
        "Expected error about W and S, got: {errors:?}"
    );
}

#[test]
fn control_validate_rejects_nan() {
    let control = WorldControl::new(vec![WorldActionControl::new("w", f32::NAN, 0)], 60);
    let errors = control.validate();
    assert!(
        errors.iter().any(|e| e.contains("NaN")),
        "Expected NaN error, got: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// WorldGenerationRequest
// ---------------------------------------------------------------------------

#[test]
fn generation_request_creates_with_required_fields() {
    let profile = WorldModelProfile::new("model", "family");
    let req = WorldGenerationRequest::new("a cat", profile, "outputs/cat.png");
    assert_eq!(req.prompt, "a cat");
    assert_eq!(req.output_target, "outputs/cat.png");
    assert!(req.source_image.is_none());
    assert!(req.seed.is_none());
}

#[test]
fn generation_request_with_all_options() {
    let profile = WorldModelProfile::new("model", "family");
    let controls = vec![WorldControl::new(
        vec![WorldActionControl::new("w", 1.0, 0)],
        60,
    )];
    let req = WorldGenerationRequest::new("walk cycle", profile, "outputs/walk.mp4")
        .with_source_image("inputs/pose.png")
        .with_controls(controls)
        .with_seed(12345);
    assert_eq!(req.source_image.as_deref(), Some("inputs/pose.png"));
    assert_eq!(req.seed, Some(12345));
    assert_eq!(req.controls.len(), 1);
}

#[test]
fn generation_request_validate_accepts_complete_lingbot_request() {
    let profile = WorldModelProfile::new("lingbot-video", "lingbot");
    let controls = vec![WorldControl::new(
        vec![
            WorldActionControl::new("w", 1.0, 0),
            WorldActionControl::new("i", 0.5, 0),
        ],
        0,
    )];
    let req = WorldGenerationRequest::new("walk forward", profile, "outputs/walk.mp4")
        .with_source_image("inputs/start.png")
        .with_controls(controls)
        .with_seed(42);

    assert!(req.validate().is_empty());
}

#[test]
fn generation_request_validate_reports_missing_required_fields() {
    let req = WorldGenerationRequest::new("", WorldModelProfile::new("", ""), "");
    let errors = req.validate();

    assert!(errors.iter().any(|error| error.contains("prompt")));
    assert!(errors.iter().any(|error| error.contains("profile name")));
    assert!(errors.iter().any(|error| error.contains("profile family")));
    assert!(errors.iter().any(|error| error.contains("output target")));
    assert!(errors.iter().any(|error| error.contains("control frame")));
}

#[test]
fn generation_request_validate_reports_ijkl_control_conflicts() {
    let profile = WorldModelProfile::new("wan-video", "wan");
    let controls = vec![WorldControl::new(
        vec![
            WorldActionControl::new("i", 1.0, 0),
            WorldActionControl::new("k", 1.0, 0),
        ],
        0,
    )];
    let req = WorldGenerationRequest::new("look conflict", profile, "outputs/look.mp4")
        .with_controls(controls);
    let errors = req.validate();

    assert!(errors.iter().any(|error| error.contains("I and K")));
}

// ---------------------------------------------------------------------------
// ArtifactRecord
// ---------------------------------------------------------------------------

#[test]
fn artifact_record_default_no_preview() {
    let artifact = ArtifactRecord::new("outputs/video.mp4", ArtifactType::Video);
    assert!(!artifact.preview_supported);
    assert!(artifact.preview_path.is_none());
}

#[test]
fn artifact_record_with_preview() {
    let artifact = ArtifactRecord::new("outputs/mesh.obj", ArtifactType::Mesh)
        .with_label("Low-poly chair")
        .with_preview("previews/mesh.png")
        .with_export("exports/chair.fbx");
    assert_eq!(artifact.label.as_deref(), Some("Low-poly chair"));
    assert!(artifact.preview_supported);
    assert_eq!(
        artifact.preview_path.as_deref(),
        Some(std::path::Path::new("previews/mesh.png"))
    );
    assert_eq!(
        artifact.export_path.as_deref(),
        Some(std::path::Path::new("exports/chair.fbx"))
    );
}

#[test]
fn artifact_type_labels() {
    assert_eq!(ArtifactType::Video.label(), "video");
    assert_eq!(ArtifactType::Mesh.label(), "mesh");
    assert_eq!(ArtifactType::Image.label(), "image");
    assert_eq!(ArtifactType::Texture.label(), "texture");
}

// ---------------------------------------------------------------------------
// GenerationProvenance
// ---------------------------------------------------------------------------

#[test]
fn provenance_creates_from_request() {
    let profile = WorldModelProfile::new("model", "family");
    let req = WorldGenerationRequest::new("test", profile, "out.png");
    let prov = GenerationProvenance::new(req.clone());
    assert_eq!(prov.request, req);
    assert!(prov.artifacts.is_empty());
}

#[test]
fn provenance_with_all_metadata() {
    let profile = WorldModelProfile::new("sd", "stable-diffusion");
    let req = WorldGenerationRequest::new("test", profile, "out.png");
    let prov = GenerationProvenance::new(req)
        .with_artifact(ArtifactRecord::new("out.png", ArtifactType::Image))
        .with_graph_node("KSampler")
        .with_workflow("upscale")
        .with_backend("comfy")
        .with_generation_time(1500);
    assert_eq!(prov.graph_node.as_deref(), Some("KSampler"));
    assert_eq!(prov.workflow_name.as_deref(), Some("upscale"));
    assert_eq!(prov.generation_time_ms, Some(1500));
    assert_eq!(prov.artifacts.len(), 1);
}

// ---------------------------------------------------------------------------
// ProvenanceCollection
// ---------------------------------------------------------------------------

#[test]
fn collection_starts_empty() {
    let coll = ProvenanceCollection::new();
    assert!(coll.is_empty());
    assert_eq!(coll.len(), 0);
}

#[test]
fn collection_find_by_prompt() {
    let mut coll = ProvenanceCollection::new();
    let profile = WorldModelProfile::new("m", "f");
    coll.push(GenerationProvenance::new(WorldGenerationRequest::new(
        "a red cat",
        profile.clone(),
        "out1.png",
    )));
    coll.push(GenerationProvenance::new(WorldGenerationRequest::new(
        "a blue dog",
        profile,
        "out2.png",
    )));
    let found = coll.find_by_prompt("cat");
    assert_eq!(found.len(), 1);
    assert!(found[0].request.prompt.contains("cat"));
}

#[test]
fn provenance_collection_into_iterator() {
    let mut coll = ProvenanceCollection::new();
    let profile = WorldModelProfile::new("m", "f");
    coll.push(GenerationProvenance::new(WorldGenerationRequest::new(
        "test", profile, "out.png",
    )));
    assert_eq!(coll.into_iter().count(), 1);
}
