use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    GameAssetPreviewRoute, SimGameImportLink, SimGameImportMetadataLinker,
    SimGameMediaClassification, SimGameMediaClassifier, SimGameProjectDescriptor,
    detect_game_project_roots, is_game_project_manifest,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameCommand {
    pub id: String,
    pub label: String,
}

impl SimGameCommand {
    fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameCommandProvider;

impl SimGameCommandProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn commands_for_paths(&self, workspace_paths: &[PathBuf]) -> Vec<SimGameCommand> {
        if detect_game_project_roots(workspace_paths).is_empty() {
            return Vec::new();
        }

        vec![
            SimGameCommand::new("sim_game.open_authoring", "Open Game Authoring"),
            SimGameCommand::new("sim_game.refresh_assets", "Refresh Game Assets"),
            SimGameCommand::new("sim_game.run_external", "Run Game"),
            SimGameCommand::new("sim_game.debug_external", "Debug Game"),
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameProjectPanelMetadata {
    pub path: PathBuf,
    pub is_project_manifest: bool,
    pub media: SimGameMediaClassification,
    pub preview_route: Option<GameAssetPreviewRoute>,
    pub import_link: Option<SimGameImportLink>,
}

impl SimGameProjectPanelMetadata {
    pub fn classify(path: impl AsRef<Path>, import_metadata: Option<&str>) -> Self {
        let path = path.as_ref();
        let media = SimGameMediaClassifier::new().classify_path(path);
        let preview_route = media
            .preview_supported
            .then(|| match media.extension.as_str() {
                "mp4" | "webm" | "mov" => GameAssetPreviewRoute::media(media.extension.clone()),
                "tscn" | "scn" => GameAssetPreviewRoute::scene(media.extension.clone()),
                _ => GameAssetPreviewRoute::native(media.extension.clone()),
            });
        let import_link = if media.extension == "import" {
            import_metadata.map(|metadata| SimGameImportMetadataLinker::new().link(path, metadata))
        } else {
            None
        };

        Self {
            path: path.to_path_buf(),
            is_project_manifest: is_game_project_manifest(path),
            media,
            preview_route,
            import_link,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameRunDebugTemplateKind {
    Run,
    Debug,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameRunDebugTemplate {
    pub kind: SimGameRunDebugTemplateKind,
    pub project_root: PathBuf,
    pub command_template: Option<String>,
    pub diagnostics: Vec<SimGameSetupDiagnostic>,
}

impl SimGameRunDebugTemplate {
    pub fn run(
        project: &SimGameProjectDescriptor,
        godot_executable: Option<impl Into<PathBuf>>,
    ) -> Self {
        Self::new(SimGameRunDebugTemplateKind::Run, project, godot_executable)
    }

    pub fn debug(
        project: &SimGameProjectDescriptor,
        godot_executable: Option<impl Into<PathBuf>>,
    ) -> Self {
        Self::new(
            SimGameRunDebugTemplateKind::Debug,
            project,
            godot_executable,
        )
    }

    fn new(
        kind: SimGameRunDebugTemplateKind,
        project: &SimGameProjectDescriptor,
        godot_executable: Option<impl Into<PathBuf>>,
    ) -> Self {
        let Some(executable) = godot_executable.map(Into::into) else {
            return Self {
                kind,
                project_root: project.root_path.clone(),
                command_template: None,
                diagnostics: vec![SimGameSetupDiagnostic {
                    code: "sim_game.editor.missing_godot_executable".to_string(),
                    message: "configure a Godot executable before using external run/debug"
                        .to_string(),
                }],
            };
        };

        let action = match kind {
            SimGameRunDebugTemplateKind::Run => "--path",
            SimGameRunDebugTemplateKind::Debug => "--debug --path",
        };

        Self {
            kind,
            project_root: project.root_path.clone(),
            command_template: Some(format!(
                "{} {action} {}",
                executable.display(),
                project.root_path.display()
            )),
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameSetupDiagnostic {
    pub code: String,
    pub message: String,
}
