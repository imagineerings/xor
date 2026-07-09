use crate::{SimGameFormatClassifier, SimGameFormatKind, SimGameTextResourceParser};

#[test]
fn format_classifier_identifies_text_resources_without_binary_imports() {
    let classifier = SimGameFormatClassifier::new();

    let scene = classifier.classify_path("levels/main.tscn");
    let binary = classifier.classify_path("levels/main.scn");

    assert_eq!(scene.kind, SimGameFormatKind::Scene);
    assert!(scene.text_parse_supported);
    assert_eq!(binary.kind, SimGameFormatKind::Unknown);
    assert!(!binary.text_parse_supported);
    assert!(binary.unsupported_reason.is_some());
}

#[test]
fn text_resource_parser_extracts_references_without_executing_scripts() {
    let source = r#"
[gd_scene load_steps=2 format=3]
[ext_resource type="Texture2D" path="res://textures/hero.png" id="1"]
[node name="Hero" type="Sprite2D"]
texture = ExtResource("1")
script = ExtResource("uid://abc123")
"#;

    let parsed = SimGameTextResourceParser::new().parse(source);

    assert_eq!(parsed.references.len(), 2);
    assert_eq!(parsed.references[0].value, "res://textures/hero.png");
    assert_eq!(parsed.references[1].value, "uid://abc123");
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn text_resource_parser_reports_missing_external_resource_path() {
    let parsed = SimGameTextResourceParser::new().parse("[ext_resource type=\"Texture2D\"]");

    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(
        parsed.diagnostics[0].code,
        "sim_game.format.missing_resource_path"
    );
}
