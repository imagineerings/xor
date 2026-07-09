use crate::{
    SIM_MEDIA_DEPENDENCY_REVIEW_REQUIRED_CODE, SIM_MEDIA_UNSUPPORTED_BACKEND_CODE,
    SimMediaBackendRequirement, SimMediaCapabilityGroup, SimMediaNodeBacklogCatalog,
    SimMediaNodeCapabilityRegistry, SimMediaPortType,
};

const MEDIA_NODE_BACKLOG: &str = include_str!("../fixtures/comfy/media_node_backlog.json");

#[test]
fn media_node_backlog_fixture_maps_remaining_nodes_to_native_sim_groups() {
    let backlog: SimMediaNodeBacklogCatalog =
        serde_json::from_str(MEDIA_NODE_BACKLOG).expect("media backlog fixture parses");
    backlog
        .validate()
        .expect("media backlog fixture should be internally valid");

    assert_eq!(backlog.records.len(), 289);
    for group in [
        SimMediaCapabilityGroup::ImageMask,
        SimMediaCapabilityGroup::Video,
        SimMediaCapabilityGroup::Audio,
        SimMediaCapabilityGroup::ThreeDGeometry,
        SimMediaCapabilityGroup::AnalysisControl,
        SimMediaCapabilityGroup::Utility,
    ] {
        assert!(
            backlog.groups().contains(&group),
            "missing media backlog group {group:?}"
        );
    }

    for record in backlog.records {
        assert!(record.metadata_only);
        assert!(
            record
                .evidence_module
                .starts_with("crates/world_model/src/sim_")
        );
        assert_eq!(record.evidence_kind, "metadata-only");
    }
}

#[test]
fn media_capability_registry_covers_required_groups_and_nodes() {
    let registry = SimMediaNodeCapabilityRegistry::default_capabilities();

    for group in [
        SimMediaCapabilityGroup::ImageMask,
        SimMediaCapabilityGroup::Video,
        SimMediaCapabilityGroup::Audio,
        SimMediaCapabilityGroup::ThreeDGeometry,
        SimMediaCapabilityGroup::AnalysisControl,
        SimMediaCapabilityGroup::Utility,
    ] {
        assert!(
            registry.groups().contains(&group),
            "missing group {group:?}"
        );
        assert!(
            !registry.by_group(group).is_empty(),
            "group {group:?} must have capabilities"
        );
    }

    for node_type in [
        "LoadImage",
        "ImageToMask",
        "LoadVideo",
        "AudioVAEEncode",
        "GaussianSplatPreview",
        "CannyEdgePreprocessor",
        "DatasetShuffle",
    ] {
        assert!(
            registry.capability(node_type).is_some(),
            "missing media node {node_type}"
        );
    }
}

#[test]
fn media_capability_registry_uses_native_sim_handlers() {
    let registry = SimMediaNodeCapabilityRegistry::default_capabilities();

    for capability in registry.capabilities() {
        assert!(
            capability.native_sim_handler.starts_with("sim."),
            "{} must map to a native Sim handler",
            capability.node_type
        );
        assert!(
            !capability.schema_ref.is_empty(),
            "{} must have schema linkage",
            capability.node_type
        );
    }
}

#[test]
fn media_capability_registry_preserves_typed_ports() {
    let registry = SimMediaNodeCapabilityRegistry::default_capabilities();

    let image_to_mask = registry
        .capability("ImageToMask")
        .expect("image to mask capability");
    assert_eq!(image_to_mask.inputs, vec![SimMediaPortType::Image]);
    assert_eq!(image_to_mask.outputs, vec![SimMediaPortType::Mask]);

    let depth = registry
        .capability("DepthAnythingPreprocessor")
        .expect("depth capability");
    assert_eq!(depth.outputs, vec![SimMediaPortType::DepthMap]);

    let mesh = registry
        .capability("TexturedMeshExport")
        .expect("mesh capability");
    assert_eq!(
        mesh.backend,
        SimMediaBackendRequirement::MeshPipelineDelegation
    );
}

#[test]
fn media_capability_registry_reports_backend_diagnostics() {
    let registry = SimMediaNodeCapabilityRegistry::default_capabilities();
    let diagnostics = registry.diagnostics();

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == SIM_MEDIA_DEPENDENCY_REVIEW_REQUIRED_CODE
                && diagnostic.node_type == "FrameInterpolation"
                && diagnostic.group == SimMediaCapabilityGroup::Video
        }),
        "frame interpolation must require dependency review"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == SIM_MEDIA_UNSUPPORTED_BACKEND_CODE
                && diagnostic.node_type == "SamDetector"
                && diagnostic.group == SimMediaCapabilityGroup::AnalysisControl
        }),
        "unsupported analysis backends must report diagnostics"
    );
}

#[test]
fn media_capability_registry_hides_developer_only_nodes_by_default() {
    let registry = SimMediaNodeCapabilityRegistry::default_capabilities();

    assert!(
        registry
            .visible_capabilities(false)
            .iter()
            .all(|capability| capability.node_type != "DatasetShuffle")
    );
    assert!(
        registry
            .visible_capabilities(true)
            .iter()
            .any(|capability| capability.node_type == "DatasetShuffle")
    );
}
