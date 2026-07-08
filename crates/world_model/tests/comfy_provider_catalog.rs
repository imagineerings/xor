use std::collections::BTreeSet;

use serde::Deserialize;
use world_model::{SimProviderCapability, SimProviderNodeAvailability, SimProviderNodeRegistry};

const PROVIDER_NODES: &str = include_str!("../fixtures/comfy/provider_nodes.json");

#[derive(Debug, Deserialize)]
struct ProviderCatalogFixture {
    schema_version: u32,
    native_sim_records: bool,
    comfyui_passthrough: bool,
    catalog: ProviderCatalog,
}

#[derive(Debug, Deserialize)]
struct ProviderCatalog {
    required_providers: Vec<String>,
    required_nodes: Vec<String>,
    required_capabilities: Vec<String>,
    unsupported_nodes: Vec<String>,
}

#[test]
fn provider_catalog_fixture_maps_to_native_sim_provider_registry() {
    let fixture: ProviderCatalogFixture =
        serde_json::from_str(PROVIDER_NODES).expect("provider catalog fixture parses");
    assert_eq!(fixture.schema_version, 1);
    assert!(fixture.native_sim_records);
    assert!(!fixture.comfyui_passthrough);

    let registry = SimProviderNodeRegistry::default();
    let object_info_nodes = registry
        .object_info_nodes()
        .into_iter()
        .map(|node| node.comfy_node_id.as_str())
        .collect::<BTreeSet<_>>();

    for node_id in &fixture.catalog.required_nodes {
        let node = registry
            .node(node_id)
            .unwrap_or_else(|| panic!("missing native Sim provider node {node_id}"));
        assert!(
            node.native_handler.starts_with("sim.provider."),
            "{node_id} must use a native Sim provider handler"
        );
        assert_eq!(
            node.comfy_node_id, *node_id,
            "{node_id} must retain its Comfy-compatible object-info id"
        );
    }

    for unsupported_node_id in &fixture.catalog.unsupported_nodes {
        let node = registry
            .node(unsupported_node_id)
            .unwrap_or_else(|| panic!("missing unsupported provider node {unsupported_node_id}"));
        assert!(
            matches!(
                node.availability,
                SimProviderNodeAvailability::Unsupported { .. }
            ),
            "{unsupported_node_id} must be represented as an unsupported native Sim diagnostic"
        );
        assert!(
            !object_info_nodes.contains(unsupported_node_id.as_str()),
            "{unsupported_node_id} must not be advertised as an enabled object-info node"
        );
    }
}

#[test]
fn provider_catalog_fixture_covers_provider_families_and_capabilities() {
    let fixture: ProviderCatalogFixture =
        serde_json::from_str(PROVIDER_NODES).expect("provider catalog fixture parses");
    let registry = SimProviderNodeRegistry::default();

    let providers = fixture
        .catalog
        .required_nodes
        .iter()
        .map(|node_id| {
            registry
                .node(node_id)
                .unwrap_or_else(|| panic!("missing native Sim provider node {node_id}"))
                .provider_id
                .as_str()
                .to_string()
        })
        .collect::<BTreeSet<_>>();

    for provider_id in &fixture.catalog.required_providers {
        assert!(
            providers.contains(provider_id),
            "fixture provider {provider_id} must be backed by a native Sim registry node"
        );
    }

    let capabilities = fixture
        .catalog
        .required_nodes
        .iter()
        .map(|node_id| {
            let node = registry
                .node(node_id)
                .unwrap_or_else(|| panic!("missing native Sim provider node {node_id}"));
            capability_name(node.capability).to_string()
        })
        .collect::<BTreeSet<_>>();

    for capability in &fixture.catalog.required_capabilities {
        assert!(
            capabilities.contains(capability),
            "fixture capability {capability} must be backed by a native Sim registry node"
        );
    }
}

fn capability_name(capability: SimProviderCapability) -> &'static str {
    match capability {
        SimProviderCapability::TextToImage => "TextToImage",
        SimProviderCapability::ImageToImage => "ImageToImage",
        SimProviderCapability::ImageEdit => "ImageEdit",
        SimProviderCapability::Inpaint => "Inpaint",
        SimProviderCapability::Outpaint => "Outpaint",
        SimProviderCapability::BackgroundRemoval => "BackgroundRemoval",
        SimProviderCapability::Upscale => "Upscale",
        SimProviderCapability::Relight => "Relight",
        SimProviderCapability::StyleTransfer => "StyleTransfer",
        SimProviderCapability::Vectorization => "Vectorization",
        SimProviderCapability::TextToVideo => "TextToVideo",
        SimProviderCapability::ImageToVideo => "ImageToVideo",
        SimProviderCapability::VideoEdit => "VideoEdit",
        SimProviderCapability::VideoExtend => "VideoExtend",
        SimProviderCapability::LipSync => "LipSync",
        SimProviderCapability::Avatar => "Avatar",
        SimProviderCapability::VideoEnhancement => "VideoEnhancement",
        SimProviderCapability::TextToAudio => "TextToAudio",
        SimProviderCapability::SpeechToText => "SpeechToText",
        SimProviderCapability::TextToSpeech => "TextToSpeech",
        SimProviderCapability::SpeechToSpeech => "SpeechToSpeech",
        SimProviderCapability::SoundEffects => "SoundEffects",
        SimProviderCapability::Music => "Music",
        SimProviderCapability::AudioIsolation => "AudioIsolation",
        SimProviderCapability::Llm => "Llm",
        SimProviderCapability::PromptEnhancement => "PromptEnhancement",
        SimProviderCapability::TextToThreeD => "TextToThreeD",
        SimProviderCapability::ImageToThreeD => "ImageToThreeD",
        SimProviderCapability::MultiviewToThreeD => "MultiviewToThreeD",
        SimProviderCapability::Texture => "Texture",
        SimProviderCapability::Rig => "Rig",
        SimProviderCapability::Animate => "Animate",
        SimProviderCapability::Retarget => "Retarget",
        SimProviderCapability::Convert => "Convert",
        SimProviderCapability::Topology => "Topology",
        SimProviderCapability::ModelImport => "ModelImport",
    }
}
