use crate::{
    DefaultBoundaryPolicy, RuntimeBoundaryDecision, RuntimeBoundaryPolicy, SimGameDebugEndpoint,
    SimGameDebugMetadata, SimGameDebugProtocol, SimGameNetworkBoundary,
    SimGameNetworkBoundaryDecision, SimGameNetworkFeature, SimGameNetworkFeatureKind,
};

#[test]
fn network_boundary_excludes_runtime_protocols() {
    let boundary = SimGameNetworkBoundary::new();
    for kind in [
        SimGameNetworkFeatureKind::MultiplayerRuntime,
        SimGameNetworkFeatureKind::EnetProtocol,
        SimGameNetworkFeatureKind::UpnpDiscovery,
        SimGameNetworkFeatureKind::PacketPeerProtocol,
    ] {
        let decision = boundary.classify(&SimGameNetworkFeature::new("runtime networking", kind));
        assert!(
            matches!(decision, SimGameNetworkBoundaryDecision::Excluded { .. }),
            "expected exclusion for {kind:?}, got {decision:?}"
        );
    }
}

#[test]
fn network_boundary_allows_native_debug_metadata() {
    let boundary = SimGameNetworkBoundary::new();
    let decision = boundary.classify(&SimGameNetworkFeature::new(
        "debug endpoint",
        SimGameNetworkFeatureKind::DebugMetadata,
    ));

    assert_eq!(
        decision,
        SimGameNetworkBoundaryDecision::NativeDebugMetadata
    );
}

#[test]
fn default_boundary_policy_excludes_godot_network_runtime_terms() {
    let policy = DefaultBoundaryPolicy;
    for scope in [
        "Godot multiplayer runtime",
        "ENet protocol adapter",
        "UPNP discovery",
        "packet peer transport",
    ] {
        let decision = policy.classify("network", scope);
        assert!(
            matches!(decision, RuntimeBoundaryDecision::Excluded { .. }),
            "expected exclusion for {scope}, got {decision:?}"
        );
    }
}

#[test]
fn debug_metadata_preserves_external_process_and_dap_records() {
    let metadata = SimGameDebugMetadata::new()
        .with_endpoint(SimGameDebugEndpoint::external_process("run-game"))
        .with_endpoint(SimGameDebugEndpoint::dap("debug-game", "127.0.0.1", 6007))
        .validate();

    assert!(metadata.diagnostics.is_empty());
    assert_eq!(metadata.endpoints.len(), 2);
    assert_eq!(metadata.endpoints[1].protocol, SimGameDebugProtocol::Dap);
}

#[test]
fn debug_metadata_reports_incomplete_dap_records() {
    let metadata = SimGameDebugMetadata::new()
        .with_endpoint(SimGameDebugEndpoint {
            name: "debug-game".to_string(),
            host: None,
            port: None,
            protocol: SimGameDebugProtocol::Dap,
        })
        .validate();

    assert_eq!(
        metadata.diagnostics,
        vec!["debug endpoint 'debug-game' requires host and port metadata"]
    );
}
