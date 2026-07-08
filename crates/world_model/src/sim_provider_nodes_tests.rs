use crate::{
    SIM_PROVIDER_DISABLED_CODE, SIM_PROVIDER_UNSUPPORTED_CODE, SimProviderCapability,
    SimProviderNodeDefinition, SimProviderNodeRegistry,
};

#[test]
fn provider_registry_exposes_enabled_native_provider_nodes() {
    let registry = SimProviderNodeRegistry::default();
    let object_info = registry.object_info_nodes();

    assert!(
        object_info
            .iter()
            .any(|node| node.comfy_node_id == "OpenAIImageGenerate"
                && node.provider_id.as_str() == "openai"
                && node.capability == SimProviderCapability::TextToImage
                && node.native_handler.starts_with("sim.provider.openai"))
    );
    assert!(
        object_info
            .iter()
            .any(|node| node.comfy_node_id == "RunwayTextToVideo"
                && node.capability == SimProviderCapability::TextToVideo)
    );
    assert!(
        object_info
            .iter()
            .all(|node| node.comfy_node_id != "SAM3Segment")
    );
}

#[test]
fn provider_registry_omits_nodes_when_api_nodes_are_disabled() {
    let registry = SimProviderNodeRegistry::default().with_api_nodes_enabled(false);
    assert!(registry.object_info_nodes().is_empty());

    let diagnostics = registry.diagnostics();
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == SIM_PROVIDER_DISABLED_CODE)
    );
}

#[test]
fn provider_registry_reports_unsupported_provider_capabilities() {
    let registry = SimProviderNodeRegistry::default();
    let diagnostics = registry.diagnostics();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_PROVIDER_UNSUPPORTED_CODE
            && diagnostic.comfy_node_id == "SAM3Segment"
            && diagnostic.provider_id.as_str() == "sam3"
    }));
}

#[test]
fn provider_registry_preserves_schema_cost_and_credentials() {
    let registry = SimProviderNodeRegistry::new([SimProviderNodeDefinition::new(
        "ideogram",
        "Ideogram",
        "IdeogramImage",
        SimProviderCapability::TextToImage,
    )
    .with_credential("ideogram.api_key")
    .with_cost("ideogram.image")]);

    let node = registry.node("IdeogramImage").expect("provider node");
    assert_eq!(node.provider_name, "Ideogram");
    assert_eq!(
        node.input_schema_ref,
        "#/provider_nodes/IdeogramImage/inputs"
    );
    assert_eq!(
        node.output_schema_ref,
        "#/provider_nodes/IdeogramImage/outputs"
    );
    assert_eq!(node.required_credentials, vec!["ideogram.api_key"]);
    assert!(node.cost.may_incur_cost);
    assert_eq!(node.cost.quota_key.as_deref(), Some("ideogram.image"));
}
