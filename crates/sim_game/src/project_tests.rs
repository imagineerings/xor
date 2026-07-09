use crate::SimGameProjectMetadataParser;

#[test]
fn project_metadata_parser_extracts_project_fields() {
    let metadata = SimGameProjectMetadataParser::new().parse(
        "game/project.godot",
        r#"
config_version=5
[application]
config/name="Space Builder"
features=PackedStringArray("4.4", "Forward Plus")
"#,
    );

    assert!(metadata.is_valid());
    let descriptor = metadata.descriptor.expect("descriptor");
    assert_eq!(descriptor.root_path, std::path::Path::new("game"));
    assert_eq!(metadata.display_name.as_deref(), Some("Space Builder"));
    assert_eq!(metadata.config_version.as_deref(), Some("5"));
    assert_eq!(metadata.enabled_features, vec!["4.4", "Forward Plus"]);
}

#[test]
fn project_metadata_parser_reports_invalid_metadata_without_panicking() {
    let metadata = SimGameProjectMetadataParser::new().parse(
        "game/not-project.txt",
        r#"
[application
config/name="Broken"
"#,
    );

    let codes = metadata
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        vec![
            "sim_game.project.invalid_manifest_path",
            "sim_game.project.invalid_section",
        ]
    );
    assert_eq!(metadata.display_name.as_deref(), Some("Broken"));
}
