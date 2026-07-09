use crate::{SimGameFormatKind, SimGameResourceIndex, SimGameResourceParseState};

#[test]
fn resource_index_extracts_scene_and_resource_references() {
    let index = SimGameResourceIndex::new().add_resource(
        "scenes/main.tscn",
        Some(
            r#"
[gd_scene load_steps=2 format=3]
[ext_resource type="Texture2D" path="res://assets/player.png" id="1"]
[node name="Player" type="Node3D"]
script = ExtResource("uid://abc123")
"#,
        ),
    );

    let resource = &index.resources()[0];
    assert_eq!(resource.classification.kind, SimGameFormatKind::Scene);
    assert_eq!(resource.parse_state, SimGameResourceParseState::Complete);
    let values = resource
        .references
        .iter()
        .map(|reference| reference.value.as_str())
        .collect::<Vec<_>>();
    assert_eq!(values, vec!["res://assets/player.png", "uid://abc123"]);
}

#[test]
fn resource_index_preserves_partial_metadata_on_parse_diagnostics() {
    let index = SimGameResourceIndex::new().add_resource(
        "materials/player.tres",
        Some(
            r#"
[gd_resource type="StandardMaterial3D" format=3]
[ext_resource type="Texture2D" id="1"]
"#,
        ),
    );

    let resource = &index.resources()[0];
    assert_eq!(resource.classification.kind, SimGameFormatKind::Resource);
    assert_eq!(resource.parse_state, SimGameResourceParseState::Partial);
    assert_eq!(
        resource.diagnostics[0].code,
        "sim_game.format.missing_resource_path"
    );
}

#[test]
fn resource_index_reports_unsupported_binary_resource() {
    let index = SimGameResourceIndex::new().add_resource("scenes/main.res", None);
    let resource = &index.resources()[0];

    assert_eq!(resource.parse_state, SimGameResourceParseState::Unsupported);
    assert_eq!(
        resource.diagnostics[0].code,
        "sim_game.resource.unsupported_format"
    );
}
