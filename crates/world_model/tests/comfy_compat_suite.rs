use std::collections::BTreeSet;

use serde_json::Value;
use world_model::{
    ComfyHttpMethod, SimApiRouteSupport, SimApiSchemaCatalog, SimPackagingQualityBacklogCatalog,
};

const ASSET_LIBRARY_BACKLOG: &str = include_str!("../fixtures/comfy/asset_library_backlog.json");
const README: &str = include_str!("../fixtures/comfy/README.md");
const API_ROUTES: &str = include_str!("../fixtures/comfy/api_routes.json");
const BASIC_API_PROMPT: &str = include_str!("../fixtures/comfy/basic_api_prompt.json");
const BLUEPRINTS_MANIFEST: &str = include_str!("../fixtures/comfy/blueprints_manifest.json");
const CORE_NODES: &str = include_str!("../fixtures/comfy/core_nodes.json");
const DIFFUSION_WORLD_MODEL_BACKLOG: &str =
    include_str!("../fixtures/comfy/diffusion_world_model_backlog.json");
const EXTENSION_BACKLOG: &str = include_str!("../fixtures/comfy/extension_backlog.json");
const MEDIA_NODE_BACKLOG: &str = include_str!("../fixtures/comfy/media_node_backlog.json");
const MODEL_EXECUTION_MANIFEST: &str =
    include_str!("../fixtures/comfy/model_execution_manifest.json");
const MODEL_MEMORY_BACKLOG: &str = include_str!("../fixtures/comfy/model_memory_backlog.json");
const PACKAGING_QUALITY_BACKLOG: &str =
    include_str!("../fixtures/comfy/packaging_quality_backlog.json");
const PROVIDER_BACKLOG: &str = include_str!("../fixtures/comfy/provider_backlog.json");
const PROVIDER_NODES: &str = include_str!("../fixtures/comfy/provider_nodes.json");
const RUNTIME_CONTROL_PLANE_BACKLOG: &str =
    include_str!("../fixtures/comfy/runtime_control_plane_backlog.json");
const WORKFLOWS_BLUEPRINTS_BACKLOG: &str =
    include_str!("../fixtures/comfy/workflows_blueprints_backlog.json");

const IMPLEMENTED_FIXTURES: &[(&str, &str)] = &[
    ("api_routes", API_ROUTES),
    ("asset_library_backlog", ASSET_LIBRARY_BACKLOG),
    ("basic_api_prompt", BASIC_API_PROMPT),
    ("blueprints_manifest", BLUEPRINTS_MANIFEST),
    ("core_nodes", CORE_NODES),
    (
        "diffusion_world_model_backlog",
        DIFFUSION_WORLD_MODEL_BACKLOG,
    ),
    ("extension_backlog", EXTENSION_BACKLOG),
    ("media_node_backlog", MEDIA_NODE_BACKLOG),
    ("model_execution_manifest", MODEL_EXECUTION_MANIFEST),
    ("model_memory_backlog", MODEL_MEMORY_BACKLOG),
    ("packaging_quality_backlog", PACKAGING_QUALITY_BACKLOG),
    ("provider_backlog", PROVIDER_BACKLOG),
    ("provider_nodes", PROVIDER_NODES),
    (
        "runtime_control_plane_backlog",
        RUNTIME_CONTROL_PLANE_BACKLOG,
    ),
    ("workflows_blueprints_backlog", WORKFLOWS_BLUEPRINTS_BACKLOG),
];

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
    for (name, fixture) in implemented_fixtures() {
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
fn compatibility_suite_fixtures_preserve_source_attribution() {
    for (name, fixture) in implemented_fixtures() {
        assert_eq!(
            fixture["captured_at"], "2026-07-09",
            "{name} must record fixture capture date"
        );
        assert!(
            fixture["implementation_owner"]
                .as_str()
                .is_some_and(|owner| owner.starts_with(".agents/specs/godot-migration/comfy-")),
            "{name} must record a Comfy migration owner"
        );
        assert!(
            fixture["source_path"]
                .as_str()
                .or_else(|| fixture["source_root"].as_str())
                .or_else(|| fixture["source"].as_str())
                .is_some_and(|source| source.starts_with("projects/comfy")),
            "{name} must preserve projects/comfy source attribution"
        );
    }
}

#[test]
fn compatibility_suite_validates_provider_api_key_fixture_safety() {
    let fixture = parse_fixture(PROVIDER_NODES);
    let provider_keys = &fixture["safety"]["provider_api_keys"];

    assert_eq!(provider_keys["requires_secret"], true);
    assert_eq!(provider_keys["mode"], "mock_secret_records");
    assert_eq!(provider_keys["dependency_review"], "not_required");
    assert_eq!(
        provider_keys["diagnostic_code"],
        "sim.provider.requires_policy"
    );

    let diagnostics = fixture["catalog"]["unsupported_diagnostics"]
        .as_array()
        .expect("provider unsupported diagnostics");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["node"] == "SAM3Segment"
                && diagnostic["diagnostic_code"]
                    == "sim.provider.unsupported_without_connector_policy"
                && diagnostic["boundary_reason"]
                    .as_str()
                    .is_some_and(|reason| reason.contains("connector policy"))
        }),
        "provider fixture must include unsupported boundary diagnostics"
    );
}

#[test]
fn compatibility_suite_validates_model_download_media_codec_and_mock_runner_safety() {
    let fixture = parse_fixture(MODEL_EXECUTION_MANIFEST);

    let model_downloads = &fixture["safety"]["model_downloads"];
    assert_eq!(model_downloads["requires_downloads"], false);
    assert_eq!(model_downloads["mode"], "metadata_only");
    assert_eq!(model_downloads["dependency_review"], "not_required");

    let media_codecs = &fixture["safety"]["media_codecs"];
    assert_eq!(media_codecs["requires_native_codecs"], false);
    assert_eq!(media_codecs["mode"], "metadata_only");
    assert_eq!(media_codecs["dependency_review"], "not_required");

    let mock_runner = &fixture["safety"]["mock_runner"];
    assert_eq!(fixture["runner"], "mock");
    assert_eq!(mock_runner["mode"], "mock");
    assert_eq!(mock_runner["dependency_review"], "not_required");

    let divergences = fixture["divergences"]
        .as_array()
        .expect("model execution divergences");
    assert!(
        divergences.iter().any(|divergence| {
            divergence["behavior"] == "production_weights"
                && divergence["sim_behavior"]
                    .as_str()
                    .is_some_and(|behavior| behavior.contains("do not download weights"))
        }),
        "model execution fixture must explain production weight divergence"
    );
}

#[test]
fn compatibility_suite_validates_metadata_only_fixture_safety() {
    for (name, fixture) in [
        ("basic_api_prompt", parse_fixture(BASIC_API_PROMPT)),
        ("blueprints_manifest", parse_fixture(BLUEPRINTS_MANIFEST)),
        ("core_nodes", parse_fixture(CORE_NODES)),
    ] {
        let safety = fixture["safety"]
            .as_object()
            .unwrap_or_else(|| panic!("{name} must include safety metadata"));
        assert!(
            safety.values().all(|entry| {
                entry["dependency_review"] == "not_required"
                    && !entry
                        .as_object()
                        .expect("safety entry object")
                        .values()
                        .any(|value| value == "comfyui_passthrough")
            }),
            "{name} safety entries must avoid dependency gates and pass-through"
        );
    }
}

#[test]
fn compatibility_suite_validates_packaging_quality_backlog_surfaces() {
    let backlog: SimPackagingQualityBacklogCatalog =
        serde_json::from_str(PACKAGING_QUALITY_BACKLOG)
            .expect("packaging quality backlog fixture parses");
    backlog
        .validate()
        .expect("packaging quality backlog fixture should be internally valid");

    assert_eq!(backlog.records.len(), 683);
    for required in [
        "launch-profile",
        "api-schema-catalog",
        "quality-fixture-suite",
        "packaging-profile",
        "dependency-review-gate",
    ] {
        assert!(
            backlog.surfaces().contains(required),
            "missing packaging quality surface {required}"
        );
    }
    for record in backlog.records {
        assert!(record.metadata_only);
        assert!(record.evidence_module.starts_with("crates/world_model/"));
        assert_ne!(record.dependency_review, "comfyui_passthrough");
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

fn implemented_fixtures() -> impl Iterator<Item = (&'static str, Value)> {
    IMPLEMENTED_FIXTURES
        .iter()
        .map(|(name, fixture)| (*name, parse_fixture(fixture)))
}
