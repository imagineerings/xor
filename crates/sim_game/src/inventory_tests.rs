use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use pretty_assertions::assert_eq;

use crate::{
    SimGameMigrationInventory, SimGameSourcePath, MigrationDecision, MigrationInventory,
    MigrationSourceArea, MigrationSpecCoverage, MigrationValidationError,
};

#[test]
fn validates_complete_grouped_spec_pack_and_source_coverage() {
    let spec_root = create_spec_root("complete");
    create_grouped_spec(&spec_root, "engine-core-runtime");
    create_grouped_spec(&spec_root, "language-scripting");

    let inventory = MigrationInventory::new(&spec_root)
        .with_grouped_specs(vec![
            MigrationSpecCoverage::new(
                "engine-core-runtime",
                "Core metadata, resources, and project model",
                "engine-core-runtime",
            ),
            MigrationSpecCoverage::new(
                "language-scripting",
                "SimScript, legacy `.gd` scripts, natural-language authoring, and Godot C# language tooling",
                "language-scripting",
            ),
        ])
        .with_source_areas(vec![
            MigrationSourceArea::new(
                "Engine core and runtime metadata",
                "projects/godot/core",
                "Metadata/indexing only; no runtime port",
                MigrationDecision::NativeSimFeature,
                Some("engine-core-runtime"),
            ),
            MigrationSourceArea::new(
                "Networking/collaboration",
                "projects/godot/modules/multiplayer",
                "Godot protocol awareness and debug integration boundaries",
                MigrationDecision::Excluded {
                    reason: "Sim already owns collaboration and RPC runtime infrastructure"
                        .to_string(),
                },
                None::<PathBuf>,
            ),
        ]);

    assert!(inventory.validate_spec_pack().is_valid());
}

#[test]
fn reports_missing_grouped_spec_documents() {
    let spec_root = create_spec_root("missing-documents");
    let spec_path = spec_root.join("platform-export");
    fs::create_dir_all(&spec_path).expect("failed to create grouped spec directory");
    fs::write(spec_path.join("requirements.md"), "# Requirements\n")
        .expect("failed to write requirements document");

    let inventory =
        MigrationInventory::new(&spec_root).with_grouped_specs(vec![MigrationSpecCoverage::new(
            "platform-export",
            "Godot project run/export task integration",
            "platform-export",
        )]);

    assert_eq!(
        inventory.validate_spec_pack().errors,
        vec![
            MigrationValidationError::MissingSpecFile {
                spec: "platform-export".to_string(),
                file: "design.md".to_string(),
            },
            MigrationValidationError::MissingSpecFile {
                spec: "platform-export".to_string(),
                file: "tasks.md".to_string(),
            },
        ]
    );
}

#[test]
fn reports_source_area_inventory_gaps() {
    let spec_root = create_spec_root("source-gaps");
    create_grouped_spec(&spec_root, "rendering-media");

    let inventory = MigrationInventory::new(&spec_root)
        .with_grouped_specs(vec![MigrationSpecCoverage::new(
            "rendering-media",
            "Preview/media/shader/generated-media support without rendering-stack duplication",
            "rendering-media",
        )])
        .with_source_areas(vec![
            MigrationSourceArea::new(
                "Rendering and media",
                "projects/godot/servers/rendering",
                "",
                MigrationDecision::SimAdapter {
                    owner: "media".to_string(),
                },
                None::<PathBuf>,
            ),
            MigrationSourceArea::new(
                "XR and spatial",
                "projects/godot/modules/openxr",
                "Docs/metadata boundaries only",
                MigrationDecision::Excluded {
                    reason: " ".to_string(),
                },
                None::<PathBuf>,
            ),
        ]);

    assert_eq!(
        inventory.validate_spec_pack().errors,
        vec![
            MigrationValidationError::MissingSourceAreaScope {
                source_area: "Rendering and media".to_string(),
            },
            MigrationValidationError::MissingSourceAreaSpecCoverage {
                source_area: "Rendering and media".to_string(),
            },
            MigrationValidationError::ExcludedSourceAreaWithoutBoundaryReason {
                source_area: "XR and spatial".to_string(),
            },
        ]
    );
}

#[test]
fn classifies_source_paths_by_most_specific_inventory_prefix() {
    let inventory = MigrationInventory::new("/unused").with_source_areas(vec![
        MigrationSourceArea::new(
            "Language and scripting",
            "projects/godot/modules/gdscript",
            "Add language support and docs indexing",
            MigrationDecision::SimAdapter {
                owner: "language".to_string(),
            },
            Some("language-scripting"),
        ),
        MigrationSourceArea::new(
            "Platform and export",
            "projects/godot/platform",
            "External Godot task integration only",
            MigrationDecision::ExternalCommand {
                command: "godot --headless".to_string(),
            },
            Some("platform-export"),
        ),
    ]);

    assert_eq!(
        inventory.classify_source_area(&SimGameSourcePath::new(
            "projects/godot/modules/gdscript/parser.cpp"
        )),
        MigrationDecision::SimAdapter {
            owner: "language".to_string(),
        }
    );
    assert_eq!(
        inventory.classify_source_area(&SimGameSourcePath::new("projects/unknown")),
        MigrationDecision::Excluded {
            reason: "Source area is not listed in the Sim game migration inventory".to_string(),
        }
    );
}

fn create_grouped_spec(spec_root: &Path, name: &str) {
    let spec_path = spec_root.join(name);
    fs::create_dir_all(&spec_path).expect("failed to create grouped spec directory");
    for file in MigrationInventory::REQUIRED_SPEC_FILES {
        fs::write(spec_path.join(file), format!("# {file}\n"))
            .expect("failed to write grouped spec document");
    }
}

fn create_spec_root(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is earlier than unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("sim-game-inventory-{name}-{timestamp}"));
    fs::create_dir_all(&path).expect("failed to create spec root");
    path
}
