use std::path::Path;

use crate::{SimGameExecutableSettings, SimGameExportPresetParser, SimGameExportTaskTemplate};

#[test]
fn export_preset_parser_extracts_task_metadata() {
    let presets = SimGameExportPresetParser::new().parse(
        r#"
[preset.0]
name="Linux"
platform="Linux/X11"
runnable=true
export_path="build/linux/game.x86_64"
"#,
    );

    assert_eq!(presets.len(), 1);
    let preset = &presets[0];
    assert_eq!(preset.name.as_deref(), Some("Linux"));
    assert_eq!(preset.platform.as_deref(), Some("Linux/X11"));
    assert_eq!(
        preset.export_path.as_deref(),
        Some(Path::new("build/linux/game.x86_64"))
    );
    assert!(preset.runnable);
    assert!(preset.diagnostics.is_empty());
}

#[test]
fn export_preset_parser_reports_invalid_presets() {
    let presets = SimGameExportPresetParser::new().parse(
        r#"
[preset.0]
name="Broken"
"#,
    );

    let codes = presets[0]
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        vec![
            "sim_game.export.missing_platform",
            "sim_game.export.missing_export_path",
        ]
    );
}

#[test]
fn export_task_template_uses_native_sim_task_record() {
    let preset = SimGameExportPresetParser::new()
        .parse(
            r#"
[preset.0]
name="Web"
platform="Web"
export_path="build/web/index.html"
"#,
        )
        .pop()
        .expect("preset");
    let template = SimGameExportTaskTemplate::from_preset(
        "game",
        &SimGameExecutableSettings::configured("/usr/local/bin/godot"),
        &preset,
    );

    assert_eq!(template.id, "sim_game.export.web");
    assert_eq!(template.label, "Export Web");
    assert_eq!(
        template.command_template.as_deref(),
        Some(
            "/usr/local/bin/godot --headless --path game --export-release \"Web\" build/web/index.html"
        )
    );
    assert!(template.diagnostics.is_empty());
}

#[test]
fn export_task_template_reports_missing_executable() {
    let preset = SimGameExportPresetParser::new()
        .parse(
            r#"
[preset.0]
name="Linux"
platform="Linux/X11"
export_path="build/linux/game.x86_64"
"#,
        )
        .pop()
        .expect("preset");
    let template = SimGameExportTaskTemplate::from_preset(
        "game",
        &SimGameExecutableSettings::missing(),
        &preset,
    );

    assert!(template.command_template.is_none());
    assert_eq!(
        template.diagnostics[0].code,
        "sim_game.export.missing_executable"
    );
}
