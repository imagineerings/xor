use crate::{
    DefaultBoundaryPolicy, RuntimeBoundaryDecision, RuntimeBoundaryPolicy,
    SimGameNavigationMetadataExtractor, SimGamePhysicsMetadataExtractor, SimGameSimulationBoundary,
    SimGameSimulationBoundaryDecision, SimGameSimulationFallbackTask, SimGameSimulationFeature,
    SimGameSimulationFeatureKind,
};

#[test]
fn simulation_boundary_excludes_physics_and_navigation_servers() {
    let boundary = SimGameSimulationBoundary::new();
    for kind in [
        SimGameSimulationFeatureKind::PhysicsServerRuntime,
        SimGameSimulationFeatureKind::NavigationServerRuntime,
    ] {
        let decision = boundary.classify(&SimGameSimulationFeature::new("server runtime", kind));
        assert!(
            matches!(decision, SimGameSimulationBoundaryDecision::Excluded { .. }),
            "expected exclusion for {kind:?}, got {decision:?}"
        );
    }
}

#[test]
fn simulation_boundary_allows_native_metadata_and_external_fallback() {
    let boundary = SimGameSimulationBoundary::new();
    assert_eq!(
        boundary.classify(&SimGameSimulationFeature::new(
            "metadata",
            SimGameSimulationFeatureKind::MetadataInspection,
        )),
        SimGameSimulationBoundaryDecision::NativeMetadata
    );
    assert_eq!(
        boundary.classify(&SimGameSimulationFeature::new(
            "fallback",
            SimGameSimulationFeatureKind::ExternalSimulationFallback,
        )),
        SimGameSimulationBoundaryDecision::ExternalFallback
    );
}

#[test]
fn default_boundary_excludes_server_runtime_terms() {
    let policy = DefaultBoundaryPolicy;
    for scope in ["Physics server execution", "Navigation server execution"] {
        let decision = policy.classify("simulation", scope);
        assert!(
            matches!(decision, RuntimeBoundaryDecision::Excluded { .. }),
            "expected exclusion for {scope}, got {decision:?}"
        );
    }
}

#[test]
fn physics_metadata_extractor_collects_docs_symbols() {
    let metadata = SimGamePhysicsMetadataExtractor::new().extract(
        "scenes/main.tscn",
        r#"
[node name="Body" type="RigidBody3D"]
[node name="Collider" type="CollisionShape3D"]
"#,
    );

    assert_eq!(metadata.bodies, vec!["RigidBody3D"]);
    assert_eq!(metadata.collision_shapes, vec!["CollisionShape3D"]);
    assert_eq!(
        metadata.docs_symbols,
        vec!["CollisionShape3D", "RigidBody3D"]
    );
}

#[test]
fn navigation_metadata_extractor_collects_docs_symbols() {
    let metadata = SimGameNavigationMetadataExtractor::new().extract(
        "scenes/nav.tscn",
        r#"
[node name="Region" type="NavigationRegion3D"]
navigation_mesh = SubResource("NavigationMesh_abc")
"#,
    );

    assert_eq!(metadata.regions, vec!["NavigationRegion3D"]);
    assert_eq!(metadata.navigation_meshes, vec!["NavigationMesh"]);
    assert_eq!(
        metadata.docs_symbols,
        vec!["NavigationMesh", "NavigationRegion3D"]
    );
}

#[test]
fn simulation_fallback_task_reports_missing_configuration() {
    let task = SimGameSimulationFallbackTask::missing_configuration();
    assert!(task.command_template.is_none());
    assert_eq!(task.diagnostics.len(), 1);

    let configured = SimGameSimulationFallbackTask::external("sim-game --simulate {project}");
    assert_eq!(
        configured.command_template.as_deref(),
        Some("sim-game --simulate {project}")
    );
}
