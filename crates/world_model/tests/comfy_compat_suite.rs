use std::collections::BTreeSet;

use serde_json::Value;
use world_model::{ComfyHttpMethod, SimApiRouteSupport, SimApiSchemaCatalog};

const README: &str = include_str!("../fixtures/comfy/README.md");
const API_ROUTES: &str = include_str!("../fixtures/comfy/api_routes.json");
const BASIC_API_PROMPT: &str = include_str!("../fixtures/comfy/basic_api_prompt.json");
const BLUEPRINTS_MANIFEST: &str = include_str!("../fixtures/comfy/blueprints_manifest.json");
const CORE_NODES: &str = include_str!("../fixtures/comfy/core_nodes.json");
const MODEL_EXECUTION_MANIFEST: &str =
    include_str!("../fixtures/comfy/model_execution_manifest.json");
const PROVIDER_NODES: &str = include_str!("../fixtures/comfy/provider_nodes.json");

#[test]
fn compatibility_suite_documents_required_fixture_groups() {
    for required in [
        "script examples",
        "route snapshots",
        "node schema snapshots",
        "blueprint manifest",
        "provider catalog",
        "asset API",
        "media capability groups",
    ] {
        assert!(
            README.contains(required),
            "fixture README must document {required}"
        );
    }
}

#[test]
fn compatibility_suite_fixtures_are_native_sim_records() {
    for (name, fixture) in [
        ("basic_api_prompt", parse_fixture(BASIC_API_PROMPT)),
        ("blueprints_manifest", parse_fixture(BLUEPRINTS_MANIFEST)),
        ("core_nodes", parse_fixture(CORE_NODES)),
        (
            "model_execution_manifest",
            parse_fixture(MODEL_EXECUTION_MANIFEST),
        ),
        ("provider_nodes", parse_fixture(PROVIDER_NODES)),
    ] {
        assert_eq!(
            fixture["native_sim_records"], true,
            "{name} must describe native Sim records"
        );
        assert_eq!(
            fixture["comfyui_passthrough"], false,
            "{name} must not be a ComfyUI pass-through fixture"
        );
    }
}

#[test]
fn compatibility_suite_route_snapshot_covers_native_api_handlers() {
    let catalog: SimApiSchemaCatalog =
        serde_json::from_str(API_ROUTES).expect("api route fixture parses");
    catalog
        .validate()
        .expect("api route fixture should be internally valid");

    for (method, path, handler) in [
        (ComfyHttpMethod::Post, "/api/prompt", "control_plane"),
        (ComfyHttpMethod::Get, "/api/features", "control_plane"),
        (ComfyHttpMethod::Get, "/api/queue", "job_bridge"),
        (ComfyHttpMethod::Get, "/api/object_info", "object_info"),
        (ComfyHttpMethod::Get, "/api/models", "model_catalog"),
        (ComfyHttpMethod::Post, "/api/upload/image", "asset_library"),
        (ComfyHttpMethod::Get, "/api/view", "asset_library"),
        (
            ComfyHttpMethod::Get,
            "/api/extensions",
            "extension_registry",
        ),
    ] {
        let route = catalog
            .route(method, path)
            .unwrap_or_else(|| panic!("missing route fixture {method:?} {path}"));
        assert_eq!(route.support, SimApiRouteSupport::Implemented);
        assert_eq!(route.native_handler.as_deref(), Some(handler));
        assert!(
            route
                .schema_ref
                .as_deref()
                .is_some_and(|schema_ref| !schema_ref.is_empty()),
            "implemented route {path} must have schema coverage"
        );
    }
}

#[test]
fn compatibility_suite_covers_node_blueprint_asset_and_media_groups() {
    let core_nodes = parse_fixture(CORE_NODES);
    let required_nodes = core_nodes["object_info"]["required_nodes"]
        .as_array()
        .expect("required node array")
        .iter()
        .map(|node| node.as_str().expect("node").to_string())
        .collect::<BTreeSet<_>>();
    for node in [
        "CheckpointLoaderSimple",
        "CLIPTextEncode",
        "KSampler",
        "LoadImage",
        "SaveImage",
        "VAEEncode",
        "VAEDecode",
    ] {
        assert!(required_nodes.contains(node), "missing node fixture {node}");
    }

    let prompt_fixture = parse_fixture(BASIC_API_PROMPT);
    let status_paths = prompt_fixture["http"]["status_paths"]
        .as_array()
        .expect("status paths")
        .iter()
        .map(|path| path.as_str().expect("path"))
        .collect::<BTreeSet<_>>();
    assert!(status_paths.contains("/api/queue"));
    assert!(status_paths.contains("/api/history"));

    let blueprints = parse_fixture(BLUEPRINTS_MANIFEST);
    assert!(
        blueprints["blueprints"]
            .as_array()
            .expect("blueprints")
            .len()
            >= 80
    );

    let model_manifest = parse_fixture(MODEL_EXECUTION_MANIFEST);
    let categories = model_manifest["workflows"]
        .as_array()
        .expect("workflows")
        .iter()
        .map(|workflow| workflow["category"].as_str().expect("category"))
        .collect::<BTreeSet<_>>();
    for category in [
        "text-to-image",
        "image-to-image",
        "inpaint",
        "ControlNet",
        "LoRA",
        "VAE",
        "video/world-model",
    ] {
        assert!(
            categories.contains(category),
            "missing media/model fixture group {category}"
        );
    }
}

#[test]
fn compatibility_suite_tracks_future_provider_and_media_fixture_owners() {
    let owner = "comfy-media-node-pipelines task 1";
    assert!(
        README.contains(owner),
        "fixture README must name future owner {owner}"
    );
}

fn parse_fixture(fixture: &str) -> Value {
    serde_json::from_str(fixture).expect("fixture parses")
}
