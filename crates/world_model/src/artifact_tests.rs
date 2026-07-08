use crate::{
    ArtifactRecord, ArtifactType, GeneratedWorldArtifact, GeneratedWorldArtifactError,
    GenerationProvenance, WorldGenerationRequest, WorldModelProfile,
};

#[test]
fn generated_artifact_requires_provenance_artifact_metadata() {
    let artifact = ArtifactRecord::new("outputs/world.mp4", ArtifactType::Video);
    let request = WorldGenerationRequest::new(
        "walk through a canyon",
        WorldModelProfile::new("lingbot-video", "lingbot"),
        "outputs/world.mp4",
    );
    let provenance = GenerationProvenance::new(request);

    let error = GeneratedWorldArtifact::new(artifact, provenance).expect_err("missing provenance");
    assert_eq!(
        error,
        GeneratedWorldArtifactError::MissingProvenanceArtifact
    );
}

#[test]
fn generated_artifact_attaches_provenance() {
    let artifact = ArtifactRecord::new("outputs/world.mp4", ArtifactType::Video);
    let request = WorldGenerationRequest::new(
        "walk through a canyon",
        WorldModelProfile::new("lingbot-video", "lingbot"),
        "outputs/world.mp4",
    );
    let provenance = GenerationProvenance::new(request).with_artifact(artifact.clone());
    let generated_artifact =
        GeneratedWorldArtifact::new(artifact.clone(), provenance).expect("valid provenance");

    assert_eq!(generated_artifact.artifact, artifact);
    assert_eq!(generated_artifact.provenance.artifacts.len(), 1);
}
