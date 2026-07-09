use std::path::Path;

use crate::{SimGameDocsIngestion, SimGameDocsRecord, SimGameDocsSource};

#[test]
fn docs_ingestion_preserves_godot_class_source_metadata() {
    let ingestion = SimGameDocsIngestion::new().with_record(
        SimGameDocsRecord::new(
            SimGameDocsSource::GodotClassReference {
                class_name: "Node3D".to_string(),
                source_path: Path::new("godot/doc/classes/Node3D.xml").to_path_buf(),
            },
            "docs/game/classes/node3d.md",
        )
        .with_source_version("godot-4.4-stable")
        .with_source_license("CC-BY-3.0")
        .with_source_permalink("https://docs.godotengine.org/en/stable/classes/class_node3d.html"),
    );

    let report = ingestion.validate();

    assert!(report.is_valid());
    assert_eq!(
        ingestion.records()[0].source_version.as_deref(),
        Some("godot-4.4-stable")
    );
}

#[test]
fn docs_ingestion_requires_source_metadata() {
    let ingestion = SimGameDocsIngestion::new().with_record(SimGameDocsRecord::new(
        SimGameDocsSource::WorldModelReference {
            title: String::new(),
            source_path: Path::new("").to_path_buf(),
        },
        "",
    ));

    let report = ingestion.validate();
    let fields = report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.field)
        .collect::<Vec<_>>();

    assert_eq!(
        fields,
        vec![
            "sim_target_path",
            "source_version",
            "source_license",
            "source_permalink",
            "source.title",
            "source.source_path",
        ]
    );
}
