use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::migration::{SimGameProjectDescriptor, SimGameProjectFormat};

/// Pure-data fields needed to construct a native `Language` for SimScript.
///
/// The app crate (`sim`) consumes this to build a `Language` instance and
/// register it via `LanguageRegistry::add` — the same path used for Rust,
/// Python, TypeScript, and every other first-class Sim language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimScriptLanguageConfig {
    /// Human-readable language name ("SimScript").
    pub name: String,
    /// File extensions without leading dot (e.g., ["simscript", "gd"]).
    pub extensions: Vec<String>,
    /// Line-comment token (e.g., "#").
    pub line_comment: Option<String>,
    /// Block-comment delimiters, if any.
    pub block_comment: Option<(String, String)>,
    /// Name of the LSP adapter to associate, if one is available.
    pub lsp_adapter: Option<String>,
}

impl SimScriptLanguageConfig {
    pub fn new() -> Self {
        Self {
            name: "SimScript".into(),
            extensions: vec!["simscript".into(), "gd".into()],
            line_comment: Some("#".into()),
            block_comment: None,
            lsp_adapter: Some("simscript-lsp".into()),
        }
    }
}

impl Default for SimScriptLanguageConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the standard SimScript language config.
///
/// Called from `sim::register_game_integration` to feed data into
/// `Language::new` + `LanguageRegistry::add`.
pub fn simscript_language_config() -> SimScriptLanguageConfig {
    SimScriptLanguageConfig::new()
}

/// Describes an external-command task provider for a game engine binary.
///
/// Per the runtime boundary policy, game engine execution (run, debug, export)
/// is external-command only — never embedded. The command template typically
/// references an engine binary such as `godot`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalGameTaskProvider {
    /// Provider identifier (e.g., "godot-run", "godot-export").
    pub id: String,
    /// Display label.
    pub label: String,
    /// The external command template, e.g. "godot --path {project} {args}".
    pub command_template: String,
    /// Whether this provider requires an interactive terminal.
    pub requires_terminal: bool,
}

impl ExternalGameTaskProvider {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        command_template: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            command_template: command_template.into(),
            requires_terminal: false,
        }
    }

    pub fn with_terminal(mut self) -> Self {
        self.requires_terminal = true;
        self
    }
}

/// The kind of preview a game asset file or artifact should route to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreviewKind {
    /// Native editor preview (e.g., image, text).
    Native,
    /// Routed to an external preview via the media system.
    Media,
    /// No preview available — show an unsupported-preview reason.
    Unsupported,
    /// Route to a scene-specific preview surface (future sub-spec).
    Scene,
}

/// Declares which file extension or artifact type routes to which preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameAssetPreviewRoute {
    /// File extension without leading dot (e.g., "png", "tscn").
    pub extension: String,
    /// Preview routing decision.
    pub kind: PreviewKind,
    /// Human-readable reason when `kind` is `Unsupported`.
    pub unsupported_reason: Option<String>,
}

impl GameAssetPreviewRoute {
    pub fn native(extension: impl Into<String>) -> Self {
        Self {
            extension: extension.into(),
            kind: PreviewKind::Native,
            unsupported_reason: None,
        }
    }

    pub fn media(extension: impl Into<String>) -> Self {
        Self {
            extension: extension.into(),
            kind: PreviewKind::Media,
            unsupported_reason: None,
        }
    }

    pub fn unsupported(extension: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            extension: extension.into(),
            kind: PreviewKind::Unsupported,
            unsupported_reason: Some(reason.into()),
        }
    }

    pub fn scene(extension: impl Into<String>) -> Self {
        Self {
            extension: extension.into(),
            kind: PreviewKind::Scene,
            unsupported_reason: None,
        }
    }
}

/// Returns the default set of external game engine task providers.
pub fn default_game_task_providers() -> Vec<ExternalGameTaskProvider> {
    vec![
        ExternalGameTaskProvider::new("godot-run", "Run game project", "godot --path {project}")
            .with_terminal(),
        ExternalGameTaskProvider::new(
            "godot-export",
            "Export game project",
            "godot --headless --export-release {preset} {output}",
        ),
    ]
}

/// Returns the default set of preview routes for game asset files.
pub fn default_game_preview_routes() -> Vec<GameAssetPreviewRoute> {
    vec![
        GameAssetPreviewRoute::native("png"),
        GameAssetPreviewRoute::native("jpg"),
        GameAssetPreviewRoute::native("webp"),
        GameAssetPreviewRoute::media("mp4"),
        GameAssetPreviewRoute::media("webm"),
        GameAssetPreviewRoute::scene("tscn"),
        GameAssetPreviewRoute::scene("scn"),
        GameAssetPreviewRoute::unsupported("res", "Binary resources require engine inspection"),
    ]
}

/// Convenience: detect game projects using the Godot manifest convention
/// (`project.godot`) and return their root directories.
pub fn detect_game_project_roots(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter_map(SimGameProjectDescriptor::from_godot_compatible_manifest_path)
        .map(|descriptor| descriptor.root_path)
        .collect()
}

/// Whether the given path is a Godot-format manifest file (`project.godot`).
pub fn is_game_project_manifest(path: &Path) -> bool {
    SimGameProjectDescriptor::from_godot_compatible_manifest_path(path).is_some()
}

/// The project format this integration targets.
pub fn target_project_format() -> SimGameProjectFormat {
    SimGameProjectFormat::GodotCompatible
}
