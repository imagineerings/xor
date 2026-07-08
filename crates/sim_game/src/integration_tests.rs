use std::path::PathBuf;

use crate::integration::{
    ExternalGameTaskProvider, GameAssetPreviewRoute, PreviewKind, default_game_preview_routes,
    default_game_task_providers, detect_game_project_roots, is_game_project_manifest,
    simscript_language_config,
};

#[test]
fn simscript_config_returns_expected_values() {
    let config = simscript_language_config();
    assert_eq!(config.name, "SimScript");
    assert_eq!(config.extensions, vec!["simscript", "gd"]);
    assert_eq!(config.line_comment.as_deref(), Some("#"));
    assert!(config.block_comment.is_none());
    assert_eq!(config.lsp_adapter.as_deref(), Some("simscript-lsp"));
}

#[test]
fn detect_game_project_roots_detects_project_godot() {
    let roots = detect_game_project_roots(&[PathBuf::from("game/project.godot")]);
    assert_eq!(roots, vec![PathBuf::from("game")]);
}

#[test]
fn detect_game_project_roots_ignores_non_manifest() {
    let roots = detect_game_project_roots(&[
        PathBuf::from("game/main.gd"),
        PathBuf::from("game/scene.tscn"),
        PathBuf::from("README.md"),
    ]);
    assert!(roots.is_empty());
}

#[test]
fn is_game_project_manifest_predicate() {
    assert!(is_game_project_manifest(std::path::Path::new(
        "project.godot"
    )));
    assert!(is_game_project_manifest(std::path::Path::new(
        "game/project.godot"
    )));
    assert!(!is_game_project_manifest(std::path::Path::new("main.gd")));
    assert!(!is_game_project_manifest(std::path::Path::new(
        "project.godot.bak"
    )));
}

#[test]
fn detect_game_project_roots_returns_all_matches() {
    let roots = detect_game_project_roots(&[
        PathBuf::from("game_a/project.godot"),
        PathBuf::from("game_b/project.godot"),
        PathBuf::from("not_a_project.txt"),
    ]);
    assert_eq!(
        roots,
        vec![PathBuf::from("game_a"), PathBuf::from("game_b")]
    );
}

#[test]
fn external_game_task_provider_builder() {
    let provider =
        ExternalGameTaskProvider::new("godot-run", "Run game project", "godot --path {project}")
            .with_terminal();
    assert_eq!(provider.id, "godot-run");
    assert_eq!(provider.label, "Run game project");
    assert_eq!(provider.command_template, "godot --path {project}");
    assert!(provider.requires_terminal);
}

#[test]
fn preview_route_constructors() {
    let native = GameAssetPreviewRoute::native("png");
    assert_eq!(native.kind, PreviewKind::Native);
    assert!(native.unsupported_reason.is_none());

    let media = GameAssetPreviewRoute::media("mp4");
    assert_eq!(media.kind, PreviewKind::Media);

    let scene = GameAssetPreviewRoute::scene("tscn");
    assert_eq!(scene.kind, PreviewKind::Scene);

    let unsupported = GameAssetPreviewRoute::unsupported("res", "binary");
    assert_eq!(unsupported.kind, PreviewKind::Unsupported);
    assert_eq!(unsupported.unsupported_reason.as_deref(), Some("binary"));
}

#[test]
fn default_task_providers_are_populated() {
    let providers = default_game_task_providers();
    assert_eq!(providers.len(), 2);
    assert_eq!(providers[0].id, "godot-run");
    assert_eq!(providers[1].id, "godot-export");
}

#[test]
fn default_preview_routes_are_populated() {
    let routes = default_game_preview_routes();
    assert_eq!(routes.len(), 8);
    assert!(routes.iter().any(|r| r.extension == "png"));
    assert!(routes.iter().any(|r| r.extension == "mp4"));
    assert!(routes.iter().any(|r| r.extension == "tscn"));
    assert!(routes.iter().any(|r| r.extension == "res"));
}

#[test]
fn config_round_trip_serde() {
    let config = simscript_language_config();
    let json = serde_json::to_string(&config).expect("serialize");
    let restored: crate::integration::SimScriptLanguageConfig =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, config);
}

#[test]
fn preview_route_round_trip_serde() {
    let route = GameAssetPreviewRoute::native("simscript");
    let json = serde_json::to_string(&route).expect("serialize");
    let restored: GameAssetPreviewRoute = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.extension, "simscript");
}

#[test]
fn task_provider_round_trip_serde() {
    let provider = ExternalGameTaskProvider::new("test", "Test", "echo {project}");
    let json = serde_json::to_string(&provider).expect("serialize");
    let restored: ExternalGameTaskProvider = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.id, "test");
}
