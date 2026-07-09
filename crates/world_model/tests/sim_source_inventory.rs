use std::collections::BTreeMap;

use world_model::{SimSourceExtractionStatus, SimSourceInventory, SimSourceKind};

const SOURCE_INVENTORY: &str = include_str!("../fixtures/comfy/source_inventory.json");
const COVERAGE_LEDGER: &str = include_str!("../fixtures/comfy/coverage_ledger.json");

#[test]
fn source_inventory_fixture_parses_as_native_sim_records() {
    let inventory = source_inventory();

    assert_eq!(inventory.schema_version, 1);
    assert_eq!(inventory.source_root, "projects/comfy");
    assert_eq!(inventory.captured_at, "2026-07-09");
    assert_eq!(
        inventory.summary.total_items as usize,
        inventory.items.len()
    );
    assert_eq!(
        inventory.summary,
        world_model::SimSourceInventorySummary::from_items(&inventory.items)
    );

    for item in &inventory.items {
        assert!(
            item.source_path.starts_with("projects/comfy/"),
            "{} must preserve Comfy source attribution",
            item.source_id
        );
        assert!(
            !item.source_id.starts_with("comfy"),
            "{} must use Sim inventory identifiers rather than Comfy runtime names",
            item.source_id
        );
    }
}

#[test]
fn source_inventory_fixture_covers_required_comfy_feature_surfaces() {
    let inventory = source_inventory();
    let counts = &inventory.summary.counts_by_kind;

    for (kind, minimum) in [
        (SimSourceKind::Route, 40),
        (SimSourceKind::WebSocketProtocol, 1),
        (SimSourceKind::CoreNode, 60),
        (SimSourceKind::ExtraNode, 400),
        (SimSourceKind::ApiProviderNode, 200),
        (SimSourceKind::ModelFamily, 80),
        (SimSourceKind::ModelFolder, 20),
        (SimSourceKind::Blueprint, 89),
        (SimSourceKind::ExtensionHook, 100),
        (SimSourceKind::CliFlag, 80),
        (SimSourceKind::OpenApiOperation, 70),
        (SimSourceKind::TestSurface, 90),
        (SimSourceKind::PackagingSurface, 30),
    ] {
        let actual = counts.get(&kind).copied().unwrap_or_default();
        assert!(
            actual >= minimum,
            "expected at least {minimum} {kind:?} inventory items, got {actual}"
        );
    }
}

#[test]
fn source_inventory_fixture_preserves_unclassified_diagnostics() {
    let inventory = source_inventory();
    let unclassified = inventory
        .items
        .iter()
        .filter(|item| item.extraction_status == SimSourceExtractionStatus::Unclassified)
        .count();

    assert!(unclassified > 0);
    assert_eq!(unclassified, inventory.diagnostics.len());
    assert_eq!(
        inventory
            .summary
            .count_for_status(SimSourceExtractionStatus::Unclassified),
        unclassified as u64
    );
    assert!(
        inventory
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == "world_model.sim_inventory.unclassified")
    );
}

#[test]
fn source_inventory_fixture_contains_expected_anchor_items() {
    let inventory = source_inventory();
    let by_symbol = inventory
        .items
        .iter()
        .map(|item| {
            (
                (item.source_kind, item.symbol.as_str()),
                item.source_path.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for (kind, symbol, expected_path) in [
        (
            SimSourceKind::Route,
            "POST /prompt",
            "projects/comfy/server.py",
        ),
        (
            SimSourceKind::WebSocketProtocol,
            "GET /ws",
            "projects/comfy/server.py",
        ),
        (
            SimSourceKind::CoreNode,
            "KSampler",
            "projects/comfy/nodes.py",
        ),
        (
            SimSourceKind::ModelFolder,
            "checkpoints",
            "projects/comfy/folder_paths.py",
        ),
        (
            SimSourceKind::Blueprint,
            "Text to Image",
            "projects/comfy/blueprints/Text to Image.json",
        ),
        (
            SimSourceKind::OpenApiOperation,
            "getQueueInfo",
            "projects/comfy/openapi.yaml",
        ),
    ] {
        assert_eq!(
            by_symbol.get(&(kind, symbol)).copied(),
            Some(expected_path),
            "missing expected {kind:?} anchor {symbol}"
        );
    }
}

#[test]
fn coverage_ledger_fixture_preserves_source_attribution_and_owners() {
    let ledger: serde_json::Value =
        serde_json::from_str(COVERAGE_LEDGER).expect("coverage ledger fixture parses");
    let records = ledger["records"]
        .as_array()
        .expect("coverage ledger records array");

    assert!(!records.is_empty());
    for record in records {
        let source_id = record["source_id"].as_str().expect("source id");
        assert!(
            record["source_path"]
                .as_str()
                .is_some_and(|source_path| source_path.starts_with("projects/comfy/")),
            "{source_id} must preserve projects/comfy source path"
        );
        assert!(
            record["owner_path"].as_str().is_some_and(|owner_path| {
                owner_path.starts_with(".agents/specs/godot-migration/comfy-")
                    || owner_path.starts_with("crates/")
            }),
            "{source_id} must include a Sim owner path"
        );
    }
}

fn source_inventory() -> SimSourceInventory {
    serde_json::from_str(SOURCE_INVENTORY).expect("source inventory fixture parses")
}
