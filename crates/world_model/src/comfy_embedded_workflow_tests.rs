use std::collections::BTreeMap;

use serde_json::json;

use crate::{
    ArtifactRecord, ArtifactType, ComfyEmbeddedWorkflowExtractor, ComfyEmbeddedWorkflowFormat,
    INVALID_EMBEDDED_PROMPT_METADATA_CODE, INVALID_EMBEDDED_WORKFLOW_METADATA_CODE,
    UNSUPPORTED_EMBEDDED_WORKFLOW_FORMAT_CODE, WorldGenerationRequest, WorldModelProfile,
};

#[test]
fn embedded_extractor_recovers_png_workflow_prompt_and_links_provenance() {
    let artifact = ArtifactRecord::new("outputs/castle.png", ArtifactType::Image)
        .with_label("Castle workflow");
    let provenance = crate::GenerationProvenance::new(request());
    let metadata = metadata([
        (
            "workflow",
            json!({
                "nodes": [{"id": 1, "type": "KSampler"}],
                "links": [],
                "extra": {"ds": {"offset": [10, 20], "scale": 1.5}}
            })
            .to_string(),
        ),
        (
            "prompt",
            json!({"1": {"class_type": "KSampler"}}).to_string(),
        ),
    ]);

    let report = ComfyEmbeddedWorkflowExtractor.extract(&artifact, Some(&provenance), &metadata);

    assert_eq!(report.format, ComfyEmbeddedWorkflowFormat::Png);
    assert!(report.diagnostics.is_empty());
    assert_eq!(
        report.prompt_json.as_ref().expect("prompt")["1"]["class_type"],
        "KSampler"
    );

    let workflow = report.workflow.expect("workflow");
    assert_eq!(workflow.name, "Castle workflow");
    assert_eq!(
        workflow.provenance_artifact_id.as_deref(),
        Some("outputs/castle.png")
    );
    assert_eq!(workflow.default_view.x, 10);
    assert_eq!(workflow.default_view.scale_millis, 1500);

    let provenance = report.provenance.expect("linked provenance");
    assert_eq!(provenance.workflow_name.as_deref(), Some("Castle workflow"));
    assert_eq!(
        provenance.artifacts[0].relative_path,
        artifact.relative_path
    );
}

#[test]
fn embedded_extractor_supports_webp_and_flac_metadata_keys() {
    let webp = ArtifactRecord::new("outputs/image.webp", ArtifactType::Image);
    let flac = ArtifactRecord::new("outputs/audio.flac", ArtifactType::Audio);
    let metadata = metadata([(
        "extra_pnginfo.workflow",
        json!({"nodes": [], "links": [], "extra": {"workflow_name": "Recovered"}}).to_string(),
    )]);

    let webp_report = ComfyEmbeddedWorkflowExtractor.extract(&webp, None, &metadata);
    let flac_report = ComfyEmbeddedWorkflowExtractor.extract(&flac, None, &metadata);

    assert_eq!(webp_report.format, ComfyEmbeddedWorkflowFormat::WebP);
    assert_eq!(flac_report.format, ComfyEmbeddedWorkflowFormat::Flac);
    assert_eq!(
        webp_report.workflow.expect("webp workflow").name,
        "Recovered"
    );
    assert_eq!(
        flac_report.workflow.expect("flac workflow").name,
        "Recovered"
    );
}

#[test]
fn embedded_extractor_reports_invalid_metadata_without_dropping_asset() {
    let artifact = ArtifactRecord::new("outputs/bad.png", ArtifactType::Image);
    let metadata = metadata([
        ("workflow", "{not-json}".to_string()),
        ("prompt", "[".to_string()),
    ]);

    let report = ComfyEmbeddedWorkflowExtractor.extract(&artifact, None, &metadata);

    assert_eq!(report.artifact.relative_path, artifact.relative_path);
    assert!(report.workflow.is_none());
    assert!(report.prompt_json.is_none());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == INVALID_EMBEDDED_WORKFLOW_METADATA_CODE
            && diagnostic.metadata_key.as_deref() == Some("workflow")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == INVALID_EMBEDDED_PROMPT_METADATA_CODE
            && diagnostic.metadata_key.as_deref() == Some("prompt")
    }));
}

#[test]
fn embedded_extractor_reports_unsupported_format_nonfatally() {
    let artifact = ArtifactRecord::new("outputs/mesh.obj", ArtifactType::Mesh);
    let metadata = metadata([("workflow", json!({"nodes": [], "links": []}).to_string())]);

    let report = ComfyEmbeddedWorkflowExtractor.extract(&artifact, None, &metadata);

    assert_eq!(report.format, ComfyEmbeddedWorkflowFormat::Unknown);
    assert_eq!(report.artifact.relative_path, artifact.relative_path);
    assert!(report.workflow.is_none());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == UNSUPPORTED_EMBEDDED_WORKFLOW_FORMAT_CODE })
    );
}

#[test]
fn embedded_extractor_preserves_asset_when_no_metadata_exists() {
    let artifact = ArtifactRecord::new("outputs/empty.png", ArtifactType::Image);

    let report = ComfyEmbeddedWorkflowExtractor.extract(&artifact, None, &BTreeMap::new());

    assert_eq!(report.artifact.relative_path, artifact.relative_path);
    assert_eq!(report.format, ComfyEmbeddedWorkflowFormat::Png);
    assert!(report.workflow.is_none());
    assert!(report.prompt_json.is_none());
    assert!(report.diagnostics.is_empty());
}

fn metadata(entries: impl IntoIterator<Item = (&'static str, String)>) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

fn request() -> WorldGenerationRequest {
    WorldGenerationRequest::new(
        "a castle",
        WorldModelProfile::new("test", "image").with_variant("base"),
        "outputs/castle.png",
    )
}
