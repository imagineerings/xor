use std::collections::BTreeSet;

use crate::{
    BLUEPRINT_COUNT_MISMATCH_CODE, ComfyBlueprintCatalog, ComfyBlueprintCategory,
    ComfyBlueprintDependencyKind, MISSING_BLUEPRINT_DEPENDENCY_CODE,
    UNSUPPORTED_BLUEPRINT_NODE_CODE,
};

const BLUEPRINTS_MANIFEST: &str = include_str!("../fixtures/comfy/blueprints_manifest.json");

#[test]
fn blueprint_manifest_imports_all_shipped_blueprints_as_native_records() {
    let catalog = blueprint_catalog_with_all_dependencies();

    assert_eq!(catalog.len(), 89);
    assert!(catalog.diagnostics().iter().all(|diagnostic| {
        diagnostic.code != BLUEPRINT_COUNT_MISMATCH_CODE
            && diagnostic.code != MISSING_BLUEPRINT_DEPENDENCY_CODE
    }));

    let names = catalog
        .records()
        .map(|record| record.name.as_str())
        .collect::<BTreeSet<_>>();
    for required in [
        "Text to Image",
        "Image to Video (Wan 2.2)",
        "Image to Gaussian Splat (TripoSplat)",
        "Audio Generation (Stable Audio 3 Medium)",
        "Video Segmentation (SAM3)",
    ] {
        assert!(names.contains(required), "missing blueprint {required}");
    }

    let categories = catalog.categories();
    for required in [
        ComfyBlueprintCategory::Image,
        ComfyBlueprintCategory::Video,
        ComfyBlueprintCategory::Audio,
        ComfyBlueprintCategory::ThreeD,
        ComfyBlueprintCategory::Depth,
        ComfyBlueprintCategory::Segmentation,
        ComfyBlueprintCategory::Pose,
        ComfyBlueprintCategory::Text,
    ] {
        assert!(
            categories.contains(&required),
            "missing category {required:?}"
        );
    }
}

#[test]
fn blueprint_records_preserve_source_paths_graph_json_and_attribution() {
    let catalog = blueprint_catalog_with_all_dependencies();
    let record = catalog
        .record("Text to Image")
        .expect("text to image blueprint should exist");

    assert_eq!(
        record.source_path,
        "projects/comfy/blueprints/Text to Image.json"
    );
    assert!(record.attribution.contains("Comfy blueprint fixture"));
    assert!(record.graph_json.get("nodes").is_some());
    assert_eq!(record.node_count, 1);
    assert!(!record.node_types.is_empty());
}

#[test]
fn blueprint_importer_registers_glsl_dependencies() {
    let catalog = blueprint_catalog_with_all_dependencies();
    let record = catalog.record("Glow").expect("glow blueprint should exist");

    assert_eq!(record.dependencies.len(), 1);
    assert_eq!(
        record.dependencies[0].kind,
        ComfyBlueprintDependencyKind::Glsl
    );
    assert_eq!(
        record.dependencies[0].source_path,
        "projects/comfy/blueprints/.glsl/Glow_30.frag"
    );
}

#[test]
fn blueprint_importer_keeps_records_when_dependencies_are_missing() {
    let catalog = ComfyBlueprintCatalog::from_manifest(
        BLUEPRINTS_MANIFEST,
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
    .expect("manifest should parse");

    assert_eq!(catalog.len(), 89);
    assert!(catalog.record("Glow").is_some());
    assert!(catalog.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == MISSING_BLUEPRINT_DEPENDENCY_CODE
            && diagnostic.blueprint_name.as_deref() == Some("Glow")
    }));
}

#[test]
fn blueprint_importer_reports_unsupported_nodes_without_dropping_blueprint() {
    let catalog = blueprint_catalog_with_all_dependencies();

    assert_eq!(catalog.len(), 89);
    assert!(catalog.record("Text to Image").is_some());
    assert!(catalog.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == UNSUPPORTED_BLUEPRINT_NODE_CODE
            && diagnostic.blueprint_name.as_deref() == Some("Text to Image")
    }));
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
