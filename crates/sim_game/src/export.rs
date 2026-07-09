use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::SimGameExecutableSettings;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameExportPreset {
    pub index: usize,
    pub name: Option<String>,
    pub platform: Option<String>,
    pub export_path: Option<PathBuf>,
    pub runnable: bool,
    pub diagnostics: Vec<SimGameExportTaskDiagnostic>,
}

impl SimGameExportPreset {
    fn new(index: usize) -> Self {
        Self {
            index,
            name: None,
            platform: None,
            export_path: None,
            runnable: false,
            diagnostics: Vec::new(),
        }
    }

    fn finish(mut self) -> Self {
        if self.name.as_deref().is_none_or(str::is_empty) {
            self.diagnostics.push(SimGameExportTaskDiagnostic {
                code: "sim_game.export.missing_name".to_string(),
                message: "export preset is missing a name".to_string(),
            });
        }
        if self.platform.as_deref().is_none_or(str::is_empty) {
            self.diagnostics.push(SimGameExportTaskDiagnostic {
                code: "sim_game.export.missing_platform".to_string(),
                message: "export preset is missing a platform".to_string(),
            });
        }
        if self.export_path.is_none() {
            self.diagnostics.push(SimGameExportTaskDiagnostic {
                code: "sim_game.export.missing_export_path".to_string(),
                message: "export preset is missing an export_path".to_string(),
            });
        }
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameExportPresetParser;

impl SimGameExportPresetParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, source: &str) -> Vec<SimGameExportPreset> {
        let mut presets = Vec::new();
        let mut current: Option<SimGameExportPreset> = None;

        for line in source.lines().map(str::trim) {
            if line.is_empty() || line.starts_with(';') {
                continue;
            }

            if let Some(index) = parse_preset_header(line) {
                if let Some(preset) = current.take() {
                    presets.push(preset.finish());
                }
                current = Some(SimGameExportPreset::new(index));
                continue;
            }

            let Some(preset) = current.as_mut() else {
                continue;
            };

            if let Some(value) = parse_assignment(line, "name") {
                preset.name = Some(value);
            } else if let Some(value) = parse_assignment(line, "platform") {
                preset.platform = Some(value);
            } else if let Some(value) = parse_assignment(line, "export_path") {
                preset.export_path = Some(PathBuf::from(value));
            } else if let Some(value) = parse_assignment(line, "runnable") {
                preset.runnable = value == "true";
            }
        }

        if let Some(preset) = current.take() {
            presets.push(preset.finish());
        }

        presets
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameExportTaskTemplate {
    pub id: String,
    pub label: String,
    pub project_root: PathBuf,
    pub command_template: Option<String>,
    pub diagnostics: Vec<SimGameExportTaskDiagnostic>,
}

impl SimGameExportTaskTemplate {
    pub fn from_preset(
        project_root: impl AsRef<Path>,
        executable_settings: &SimGameExecutableSettings,
        preset: &SimGameExportPreset,
    ) -> Self {
        let project_root = project_root.as_ref().to_path_buf();
        let mut diagnostics = preset.diagnostics.clone();

        let Some(executable_path) = executable_settings.executable_path.as_ref() else {
            diagnostics.push(SimGameExportTaskDiagnostic {
                code: "sim_game.export.missing_executable".to_string(),
                message: "configure a game engine executable before creating export tasks"
                    .to_string(),
            });
            return Self {
                id: task_id(preset),
                label: task_label(preset),
                project_root,
                command_template: None,
                diagnostics,
            };
        };

        let command_template = if diagnostics.is_empty() {
            match (&preset.name, &preset.export_path) {
                (Some(name), Some(export_path)) => Some(format!(
                    "{} --headless --path {} --export-release \"{}\" {}",
                    executable_path.display(),
                    project_root.display(),
                    name,
                    export_path.display()
                )),
                _ => None,
            }
        } else {
            None
        };

        Self {
            id: task_id(preset),
            label: task_label(preset),
            project_root,
            command_template,
            diagnostics,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameExportTaskDiagnostic {
    pub code: String,
    pub message: String,
}

fn parse_preset_header(line: &str) -> Option<usize> {
    let raw = line.strip_prefix("[preset.")?.strip_suffix(']')?;
    raw.parse().ok()
}

fn parse_assignment(line: &str, key: &str) -> Option<String> {
    let value = line
        .strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    Some(value.trim_matches('"').to_string()).filter(|value| !value.is_empty())
}

fn task_id(preset: &SimGameExportPreset) -> String {
    let name = preset.name.as_deref().unwrap_or("unnamed");
    let slug = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("sim_game.export.{slug}")
}

fn task_label(preset: &SimGameExportPreset) -> String {
    match &preset.name {
        Some(name) => format!("Export {name}"),
        None => format!("Export preset {}", preset.index),
    }
}
