use crate::{
    DefaultBoundaryPolicy, RuntimeBoundaryDecision, RuntimeBoundaryPolicy,
    SimGameSpatialMetadataExtractor, SimGameXrBoundary, SimGameXrBoundaryDecision,
    SimGameXrFeature, SimGameXrFeatureKind,
};

#[test]
fn xr_boundary_excludes_runtime_stacks() {
    let boundary = SimGameXrBoundary::new();
    for kind in [
        SimGameXrFeatureKind::OpenXrRuntime,
        SimGameXrFeatureKind::WebXrRuntime,
        SimGameXrFeatureKind::VrRuntime,
    ] {
        let decision = boundary.classify(&SimGameXrFeature::new("xr runtime", kind));
        assert!(
            matches!(decision, SimGameXrBoundaryDecision::Excluded { .. }),
            "expected exclusion for {kind:?}, got {decision:?}"
        );
    }
}

#[test]
fn xr_boundary_allows_native_spatial_metadata() {
    let boundary = SimGameXrBoundary::new();
    let decision = boundary.classify(&SimGameXrFeature::new(
        "spatial metadata",
        SimGameXrFeatureKind::SpatialMetadata,
    ));

    assert_eq!(decision, SimGameXrBoundaryDecision::NativeSpatialMetadata);
}

#[test]
fn default_boundary_policy_excludes_xr_runtime_terms() {
    let policy = DefaultBoundaryPolicy;
    for scope in ["OpenXR runtime", "WebXR runtime", "VR runtime"] {
        let decision = policy.classify("xr", scope);
        assert!(
            matches!(decision, RuntimeBoundaryDecision::Excluded { .. }),
            "expected exclusion for {scope}, got {decision:?}"
        );
    }
}

#[test]
fn spatial_metadata_extractor_collects_docs_and_preview_route() {
    let metadata = SimGameSpatialMetadataExtractor::new().extract(
        "scenes/xr.tscn",
        r#"
[node name="Origin" type="XROrigin3D"]
[node name="Camera" type="XRCamera3D"]
[node name="Controller" type="XRController3D"]
"#,
    );

    assert_eq!(
        metadata.spatial_classes,
        vec!["XRCamera3D", "XRController3D", "XROrigin3D"]
    );
    assert_eq!(metadata.docs_symbols, metadata.spatial_classes);
    let route = metadata.preview_route.expect("preview route");
    assert_eq!(route.route_name, "sim_game.spatial.preview.scene");
    assert_eq!(route.target, "scenes/xr.tscn");
}

#[test]
fn spatial_metadata_without_spatial_classes_has_no_preview_route() {
    let metadata = SimGameSpatialMetadataExtractor::new().extract(
        "assets/texture.tres",
        r#"
[resource]
"#,
    );

    assert!(metadata.spatial_classes.is_empty());
    assert!(metadata.preview_route.is_none());
}
