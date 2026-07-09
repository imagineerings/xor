use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::migration::SimGameProjectDescriptor;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameProjectMetadata {
    pub descriptor: Option<SimGameProjectDescriptor>,
    pub display_name: Option<String>,
    pub config_version: Option<String>,
    pub enabled_features: Vec<String>,
    pub diagnostics: Vec<SimGameProjectDiagnostic>,
}

impl SimGameProjectMetadata {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameProjectDiagnostic {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
}

impl SimGameProjectDiagnostic {
    fn new(code: impl Into<String>, message: impl Into<String>, line: Option<usize>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            line,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameProjectMetadataParser;

impl SimGameProjectMetadataParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, manifest_path: impl AsRef<Path>, source: &str) -> SimGameProjectMetadata {
        let manifest_path = manifest_path.as_ref();
        let mut diagnostics = Vec::new();
        let descriptor =
            match SimGameProjectDescriptor::from_godot_compatible_manifest_path(manifest_path) {
                Some(descriptor) => Some(descriptor),
                None => {
                    diagnostics.push(SimGameProjectDiagnostic::new(
                        "sim_game.project.invalid_manifest_path",
                        "Godot-compatible project metadata requires a project.godot manifest",
                        None,
                    ));
                    None
                }
            };

        if source.trim().is_empty() {
            diagnostics.push(SimGameProjectDiagnostic::new(
                "sim_game.project.empty_manifest",
                "project.godot metadata is empty",
                None,
            ));
        }

        let mut display_name = None;
        let mut config_version = None;
        let mut enabled_features = Vec::new();

        for (index, line) in source.lines().enumerate() {
            let line_number = index + 1;
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }

            if line.starts_with('[') && !line.ends_with(']') {
                diagnostics.push(SimGameProjectDiagnostic::new(
                    "sim_game.project.invalid_section",
                    "project.godot section header is missing a closing bracket",
                    Some(line_number),
                ));
                continue;
            }

            if let Some(value) = parse_assignment(line, "config/name") {
                display_name = Some(value);
            } else if let Some(value) = parse_assignment(line, "config_version") {
                config_version = Some(value);
            } else if let Some(value) = parse_assignment(line, "features") {
                enabled_features = parse_array_value(&value);
            }
        }

        SimGameProjectMetadata {
            descriptor,
            display_name,
            config_version,
            enabled_features,
            diagnostics,
        }
    }
}

fn parse_assignment(line: &str, key: &str) -> Option<String> {
    let value = line
        .strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    Some(value.trim_matches('"').to_string()).filter(|value| !value.is_empty())
}

fn parse_array_value(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches("PackedStringArray(")
        .trim_start_matches('[')
        .trim_end_matches(')')
        .trim_end_matches(']')
        .split(',')
        .map(|item| item.trim().trim_matches('"'))
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub fn project_manifest_path(root: impl Into<PathBuf>) -> PathBuf {
    root.into().join("project.godot")
}
