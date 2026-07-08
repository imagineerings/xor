use std::collections::BTreeSet;

use serde_json::json;

use crate::{
    ComfyBlueprintCatalog, ComfySubgraphId, ComfySubgraphIndex, ComfySubgraphSourceType,
    DUPLICATE_SUBGRAPH_ID_CODE, SUBGRAPH_NOT_FOUND_CODE,
};

const BLUEPRINTS_MANIFEST: &str = include_str!("../fixtures/comfy/blueprints_manifest.json");

#[test]
fn subgraph_index_lists_all_blueprints_as_native_records() {
    let catalog = blueprint_catalog_with_all_dependencies();
    let index = ComfySubgraphIndex::from_blueprint_catalog(&catalog);

    assert_eq!(index.len(), 89);
    assert!(index.diagnostics().is_empty());

    let names = index
        .listings()
        .into_iter()
        .map(|listing| listing.name)
        .collect::<BTreeSet<_>>();
    for required in [
        "Text to Image",
        "Image to Video (Wan 2.2)",
        "Image to Gaussian Splat (TripoSplat)",
    ] {
        assert!(names.contains(required), "missing subgraph {required}");
    }
}

#[test]
fn subgraph_ids_are_stable_for_source_type_and_path() {
    let id = ComfySubgraphId::from_source(
        ComfySubgraphSourceType::Blueprint,
        "projects/comfy/blueprints/Text to Image.json",
    );
    let same = ComfySubgraphId::from_source(
        ComfySubgraphSourceType::Blueprint,
        "projects/comfy/blueprints/Text to Image.json",
    );
    let different_type = ComfySubgraphId::from_source(
        ComfySubgraphSourceType::CustomNode,
        "projects/comfy/blueprints/Text to Image.json",
    );

    assert_eq!(id, same);
    assert_eq!(id.as_str(), "subgraph-blueprint-7150d8322a060200");
    assert_ne!(id, different_type);
}

#[test]
fn subgraph_listing_uses_sanitized_metadata_without_full_graph() {
    let mut index = ComfySubgraphIndex::default();
    let id = index.register_custom_node_subgraph(
        "pack-a",
        "Reusable Loader",
        "custom_nodes/pack-a/subgraphs/loader.json",
        json!({
            "nodes": [{"id": 1, "type": "CheckpointLoaderSimple"}],
            "links": []
        }),
        json!({
            "author": "Sim",
            "token": "secret-token",
            "nested": {"api_key": "hidden", "safe": true},
            "workflow": {"nodes": []}
        }),
    );

    let listing = index
        .listings()
        .into_iter()
        .find(|listing| listing.id == id)
        .expect("listing should exist");

    assert_eq!(listing.source_type, ComfySubgraphSourceType::CustomNode);
    assert_eq!(listing.node_pack_name.as_deref(), Some("pack-a"));
    assert_eq!(listing.node_count, 1);
    assert!(listing.metadata.get("token").is_none());
    assert!(listing.metadata.get("workflow").is_none());
    assert!(listing.metadata["nested"].get("api_key").is_none());
    assert_eq!(listing.metadata["nested"]["safe"], true);
    assert_eq!(listing.metadata["author"], "Sim");
}

#[test]
fn subgraph_open_returns_full_graph_data() {
    let catalog = blueprint_catalog_with_all_dependencies();
    let index = ComfySubgraphIndex::from_blueprint_catalog(&catalog);
    let id = ComfySubgraphId::from_source(
        ComfySubgraphSourceType::Blueprint,
        "projects/comfy/blueprints/Text to Image.json",
    );

    let record = index.open(&id).expect("subgraph should open");

    assert_eq!(record.name, "Text to Image");
    assert_eq!(
        record.source.source_path(),
        "projects/comfy/blueprints/Text to Image.json"
    );
    assert!(record.graph_json.get("nodes").is_some());
    assert_eq!(record.node_count, 1);
}

#[test]
fn subgraph_index_reports_duplicate_and_missing_records() {
    let mut index = ComfySubgraphIndex::default();
    for name in ["first", "second"] {
        index.register_custom_node_subgraph(
            "pack-a",
            name,
            "custom_nodes/pack-a/subgraphs/reused.json",
            json!({"nodes": [], "links": []}),
            json!({}),
        );
    }

    assert_eq!(index.len(), 1);
    assert!(index.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == DUPLICATE_SUBGRAPH_ID_CODE
            && diagnostic
                .subgraph_id
                .as_ref()
                .is_some_and(|id| id.as_str().starts_with("subgraph-custom-node-"))
    }));

    let diagnostic = index
        .open(&ComfySubgraphId::from_source(
            ComfySubgraphSourceType::Blueprint,
            "missing.json",
        ))
        .expect_err("missing subgraph should fail");
    assert_eq!(diagnostic.code, SUBGRAPH_NOT_FOUND_CODE);
}

fn blueprint_catalog_with_all_dependencies() -> ComfyBlueprintCatalog {
    ComfyBlueprintCatalog::from_manifest(
        BLUEPRINTS_MANIFEST,
        all_manifest_dependencies(),
        Vec::<String>::new(),
    )
    .expect("manifest should parse")
}

fn all_manifest_dependencies() -> Vec<String> {
    let manifest: serde_json::Value =
        serde_json::from_str(BLUEPRINTS_MANIFEST).expect("manifest should parse");
    manifest["blueprints"]
        .as_array()
        .expect("blueprints array")
        .iter()
        .flat_map(|blueprint| {
            blueprint["dependencies"]
                .as_array()
                .expect("dependencies array")
                .iter()
                .map(|dependency| {
                    dependency["source_path"]
                        .as_str()
                        .expect("dependency source path")
                        .to_string()
                })
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
