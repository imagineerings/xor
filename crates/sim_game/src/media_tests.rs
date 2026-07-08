use crate::{SimGameMediaClassifier, SimGameMediaKind};

#[test]
fn media_classifier_routes_supported_preview_types() {
    let classifier = SimGameMediaClassifier::new();

    let texture = classifier.classify_path("textures/albedo.png");
    let shader = classifier.classify_path("shaders/water.gdshader");
    let video = classifier.classify_path("renders/preview.webm");

    assert_eq!(texture.kind, SimGameMediaKind::Texture);
    assert!(texture.preview_supported);
    assert_eq!(shader.kind, SimGameMediaKind::Shader);
    assert!(shader.preview_supported);
    assert_eq!(video.kind, SimGameMediaKind::Video);
    assert!(video.preview_supported);
}

#[test]
fn media_classifier_excludes_render_backend_features() {
    let classification = SimGameMediaClassifier::new().classify_extension("vulkan");

    assert_eq!(classification.kind, SimGameMediaKind::RenderBackend);
    assert!(!classification.preview_supported);
    assert!(
        classification
            .unsupported_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("excluded"))
    );
}

#[test]
fn media_classifier_preserves_unsupported_preview_reasons() {
    let classification = SimGameMediaClassifier::new().classify_extension("res");

    assert_eq!(classification.kind, SimGameMediaKind::Unknown);
    assert!(!classification.preview_supported);
    assert_eq!(
        classification.unsupported_reason.as_deref(),
        Some("binary or imported resources require engine inspection")
    );
}
