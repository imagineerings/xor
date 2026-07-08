use std::path::Path;

use pretty_assertions::assert_eq;

use crate::{
    RuntimeBoundaryDecision, SimGameFeatureArea, SimGameMetadata, SimGameProjectDescriptor,
    SimGameProjectFormat, SimGameSourceReference, migration::is_godot_compatible_manifest,
};

#[test]
fn detects_godot_compatible_project_descriptor() {
    let descriptor = SimGameProjectDescriptor::from_godot_compatible_manifest_path(
        "/workspace/game/project.godot",
    );

    assert_eq!(
        descriptor,
        Some(SimGameProjectDescriptor {
            root_path: Path::new("/workspace/game").to_path_buf(),
            manifest_path: Path::new("/workspace/game/project.godot").to_path_buf(),
            format: SimGameProjectFormat::GodotCompatible,
        })
    );
}

#[test]
fn rejects_non_godot_manifest_paths() {
    assert!(!is_godot_compatible_manifest(Path::new(
        "/workspace/game/project.cfg"
    )));
    assert_eq!(
        SimGameProjectDescriptor::from_godot_compatible_manifest_path("/workspace/project.cfg"),
        None
    );
}

#[test]
fn records_sim_owned_metadata_for_source_features() {
    let source = SimGameSourceReference::new("res://player.gd").with_position(12, 4);
    let metadata = SimGameMetadata::new(
        SimGameFeatureArea::ScriptMetadata,
        source.clone(),
        RuntimeBoundaryDecision::SimAdapter {
            owner: "language".to_string(),
        },
    );

    assert_eq!(metadata.source, source);
    assert_eq!(metadata.feature_area, SimGameFeatureArea::ScriptMetadata);
    assert!(metadata.boundary.is_executable_inside_sim());
}

#[test]
fn keeps_duplicate_runtime_systems_outside_sim() {
    let decisions = [
        RuntimeBoundaryDecision::ExternalCommand {
            command: "godot --headless".to_string(),
        },
        RuntimeBoundaryDecision::Excluded {
            reason: "Sim already owns platform and rendering runtime integration".to_string(),
        },
    ];

    for decision in decisions {
        assert!(!decision.is_executable_inside_sim());
    }
}
