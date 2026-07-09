use std::path::PathBuf;

use crate::{
    PreviewKind, SimGameCommandProvider, SimGameProjectDescriptor, SimGameProjectPanelMetadata,
    SimGameRunDebugTemplate, SimGameRunDebugTemplateKind,
};

#[test]
fn commands_register_only_for_detected_game_projects() {
    let provider = SimGameCommandProvider::new();

    assert!(
        provider
            .commands_for_paths(&[PathBuf::from("src/main.rs")])
            .is_empty()
    );

    let commands = provider.commands_for_paths(&[PathBuf::from("game/project.godot")]);
    let ids = commands
        .iter()
        .map(|command| command.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "sim_game.open_authoring",
            "sim_game.refresh_assets",
            "sim_game.run_external",
            "sim_game.debug_external",
        ]
    );
}

#[test]
fn project_panel_classifies_assets_and_preview_routes() {
    let texture = SimGameProjectPanelMetadata::classify("assets/player.png", None);
    assert!(texture.media.preview_supported);
    assert_eq!(
        texture.preview_route.expect("preview route").kind,
        PreviewKind::Native
    );

    let scene = SimGameProjectPanelMetadata::classify("scenes/main.tscn", None);
    assert_eq!(
        scene.preview_route.expect("scene route").kind,
        PreviewKind::Scene
    );
}

#[test]
fn project_panel_links_import_metadata() {
    let metadata = SimGameProjectPanelMetadata::classify(
        "assets/player.png.import",
        Some(
            r#"
source_file="res://assets/player.png"
dest_files=["res://.godot/imported/player.ctex"]
"#,
        ),
    );

    let import_link = metadata.import_link.expect("import link");
    assert_eq!(
        import_link.source_file.as_deref(),
        Some("res://assets/player.png")
    );
    assert_eq!(
        import_link.generated_files,
        vec!["res://.godot/imported/player.ctex"]
    );
    assert!(import_link.diagnostics.is_empty());
}

#[test]
fn run_template_uses_external_godot_command() {
    let project =
        SimGameProjectDescriptor::from_godot_compatible_manifest_path("game/project.godot")
            .expect("project");
    let template = SimGameRunDebugTemplate::run(&project, Some("/usr/local/bin/godot"));

    assert_eq!(template.kind, SimGameRunDebugTemplateKind::Run);
    let command = template.command_template.expect("command");
    assert!(command.contains("/usr/local/bin/godot --path game"));
    assert!(template.diagnostics.is_empty());
}

#[test]
fn debug_template_reports_missing_executable_setup() {
    let project =
        SimGameProjectDescriptor::from_godot_compatible_manifest_path("game/project.godot")
            .expect("project");
    let template = SimGameRunDebugTemplate::debug(&project, None::<PathBuf>);

    assert_eq!(template.kind, SimGameRunDebugTemplateKind::Debug);
    assert!(template.command_template.is_none());
    assert_eq!(
        template.diagnostics[0].code,
        "sim_game.editor.missing_godot_executable"
    );
}
