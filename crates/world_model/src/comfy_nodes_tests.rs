use crate::{
    ComfyNodeDefinition, ComfyNodeInput, ComfyNodeOutput, ComfyNodeRegistry, ComfyNodeSource,
    DataType,
};

#[test]
fn registry_exposes_core_node_object_info() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let object_info = registry.object_info(None);

    let sampler = object_info.nodes.get("KSampler").expect("KSampler info");
    assert_eq!(sampler.display_name, "KSampler");
    assert_eq!(sampler.category, "sampling");
    assert_eq!(sampler.source, ComfyNodeSource::Core);
    assert!(!sampler.api_node);
    assert!(sampler.search_aliases.contains(&"sampler".to_string()));
    assert!(
        sampler
            .inputs
            .iter()
            .any(|input| input.name == "positive" && input.data_type == DataType::Conditioning)
    );
    assert!(
        sampler
            .outputs
            .iter()
            .any(|output| output.name == "LATENT" && output.data_type == DataType::Latent)
    );
}

#[test]
fn registry_can_filter_object_info_to_one_node() {
    let registry = ComfyNodeRegistry::with_core_nodes();
    let object_info = registry.object_info(Some("CLIPTextEncode"));

    assert_eq!(object_info.nodes.len(), 1);
    assert!(object_info.nodes.contains_key("CLIPTextEncode"));
}

#[test]
fn registry_omits_disabled_nodes_and_reports_disabled_availability() {
    let mut registry = ComfyNodeRegistry::with_core_nodes();
    registry.disable("KSampler");

    assert!(registry.get("KSampler").is_none());
    assert!(!registry.object_info(None).nodes.contains_key("KSampler"));
    let diagnostic = registry
        .availability("KSampler")
        .expect_err("disabled node rejected");
    assert_eq!(diagnostic.code, crate::comfy_nodes::DISABLED_NODE_CODE);
}

#[test]
fn registry_rejects_duplicate_node_definitions() {
    let mut registry = ComfyNodeRegistry::new();
    let node = api_node("ProviderFoo");
    registry.register(node.clone()).expect("first register");

    let diagnostic = registry
        .register(node)
        .expect_err("duplicate node rejected");

    assert_eq!(diagnostic.code, crate::comfy_nodes::DUPLICATE_NODE_CODE);
}

#[test]
fn registry_stores_api_and_custom_node_markers_as_native_metadata() {
    let mut registry = ComfyNodeRegistry::new();
    registry
        .register(api_node("ProviderFoo"))
        .expect("api node");
    let custom = ComfyNodeDefinition {
        source: ComfyNodeSource::Custom,
        api_node: false,
        ..api_node("CustomFoo")
    };
    registry.register(custom).expect("custom node");

    let object_info = registry.object_info(None);
    assert!(object_info.nodes["ProviderFoo"].api_node);
    assert_eq!(
        object_info.nodes["ProviderFoo"].source,
        ComfyNodeSource::ApiProvider
    );
    assert_eq!(
        object_info.nodes["CustomFoo"].source,
        ComfyNodeSource::Custom
    );
}

#[test]
fn registry_search_matches_aliases_and_hides_disabled_nodes() {
    let mut registry = ComfyNodeRegistry::with_core_nodes();
    registry.disable("SaveImage");

    let loader_matches = registry.search("model loader");
    assert!(
        loader_matches
            .iter()
            .any(|node| node.id == "CheckpointLoaderSimple")
    );
    let artifact_matches = registry.search("artifact");
    assert!(
        artifact_matches
            .iter()
            .any(|node| node.id == "PreviewImage" || node.id == "SaveImageWebsocket")
    );
    assert!(artifact_matches.iter().all(|node| node.id != "SaveImage"));
}

#[test]
fn registry_reports_unknown_nodes_for_later_prompt_validation() {
    let diagnostic = ComfyNodeRegistry::with_core_nodes()
        .availability("MissingNode")
        .expect_err("missing node rejected");

    assert_eq!(diagnostic.code, crate::comfy_nodes::UNKNOWN_NODE_CODE);
}

fn api_node(id: &str) -> ComfyNodeDefinition {
    ComfyNodeDefinition {
        id: id.to_string(),
        display_name: id.to_string(),
        category: "api".to_string(),
        source: ComfyNodeSource::ApiProvider,
        api_node: true,
        search_aliases: ["provider".to_string()].into_iter().collect(),
        inputs: vec![ComfyNodeInput {
            name: "prompt".to_string(),
            data_type: DataType::String,
            required: true,
            tooltip: Some("Prompt text".to_string()),
        }],
        outputs: vec![ComfyNodeOutput {
            name: "STRING".to_string(),
            data_type: DataType::String,
            tooltip: Some("Provider output".to_string()),
        }],
        tooltip: Some("Native Sim provider node".to_string()),
    }
}
