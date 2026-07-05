use std::path::Path;

use pretty_assertions::assert_eq;

use crate::{
    BaymaxGameFeatureArea, BaymaxGameMetadata, BaymaxGameProjectDescriptor,
    BaymaxGameProjectFormat, BaymaxGameSourceReference, RuntimeBoundaryDecision,
    migration::is_godot_compatible_manifest,
};

#[test]
fn detects_godot_compatible_project_descriptor() {
    let descriptor = BaymaxGameProjectDescriptor::from_godot_compatible_manifest_path(
        "/workspace/game/project.godot",
    );

    assert_eq!(
        descriptor,
        Some(BaymaxGameProjectDescriptor {
            root_path: Path::new("/workspace/game").to_path_buf(),
            manifest_path: Path::new("/workspace/game/project.godot").to_path_buf(),
            format: BaymaxGameProjectFormat::GodotCompatible,
        })
    );
}

#[test]
fn rejects_non_godot_manifest_paths() {
    assert!(!is_godot_compatible_manifest(Path::new(
        "/workspace/game/project.cfg"
    )));
    assert_eq!(
        BaymaxGameProjectDescriptor::from_godot_compatible_manifest_path("/workspace/project.cfg"),
        None
    );
}

#[test]
fn records_baymax_owned_metadata_for_source_features() {
    let source = BaymaxGameSourceReference::new("res://player.gd").with_position(12, 4);
    let metadata = BaymaxGameMetadata::new(
        BaymaxGameFeatureArea::ScriptMetadata,
        source.clone(),
        RuntimeBoundaryDecision::BaymaxAdapter {
            owner: "language".to_string(),
        },
    );

    assert_eq!(metadata.source, source);
    assert_eq!(metadata.feature_area, BaymaxGameFeatureArea::ScriptMetadata);
    assert!(metadata.boundary.is_executable_inside_baymax());
}

#[test]
fn keeps_duplicate_runtime_systems_outside_baymax() {
    let decisions = [
        RuntimeBoundaryDecision::ExternalCommand {
            command: "godot --headless".to_string(),
        },
        RuntimeBoundaryDecision::Excluded {
            reason: "Baymax already owns platform and rendering runtime integration".to_string(),
        },
    ];

    for decision in decisions {
        assert!(!decision.is_executable_inside_baymax());
    }
}
