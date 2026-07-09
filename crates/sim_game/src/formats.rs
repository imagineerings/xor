use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameFormatKind {
    Project,
    Scene,
    Resource,
    ImportMetadata,
    Mesh,
    Texture,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameFormatClassification {
    pub extension: String,
    pub kind: SimGameFormatKind,
    pub text_parse_supported: bool,
    pub unsupported_reason: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameFormatClassifier;

impl SimGameFormatClassifier {
    pub fn new() -> Self {
        Self
    }

    pub fn classify_path(&self, path: impl AsRef<Path>) -> SimGameFormatClassification {
        let path = path.as_ref();
        if path.file_name().and_then(|name| name.to_str()) == Some("project.godot") {
            return SimGameFormatClassification {
                extension: "godot".to_string(),
                kind: SimGameFormatKind::Project,
                text_parse_supported: true,
                unsupported_reason: None,
            };
        }
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        self.classify_extension(extension)
    }

    pub fn classify_extension(&self, extension: impl Into<String>) -> SimGameFormatClassification {
        let extension = extension
            .into()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        let (kind, text_parse_supported, unsupported_reason) = match extension.as_str() {
            "godot" => (SimGameFormatKind::Project, true, None),
            "tscn" => (SimGameFormatKind::Scene, true, None),
            "tres" => (SimGameFormatKind::Resource, true, None),
            "import" => (SimGameFormatKind::ImportMetadata, true, None),
            "obj" | "glb" | "gltf" | "ply" | "stl" => (SimGameFormatKind::Mesh, false, None),
            "png" | "jpg" | "jpeg" | "webp" | "ktx" | "ktx2" => {
                (SimGameFormatKind::Texture, false, None)
            }
            "scn" | "res" => (
                SimGameFormatKind::Unknown,
                false,
                Some("binary Godot resources require engine inspection".to_string()),
            ),
            _ => (
                SimGameFormatKind::Unknown,
                false,
                Some("no native Sim parser is registered for this game format".to_string()),
            ),
        };

        SimGameFormatClassification {
            extension,
            kind,
            text_parse_supported,
            unsupported_reason,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameTextResourceParse {
    pub references: Vec<SimGameResourceReference>,
    pub diagnostics: Vec<SimGameFormatDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameResourceReference {
    pub value: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameFormatDiagnostic {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameTextResourceParser;

impl SimGameTextResourceParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, source: &str) -> SimGameTextResourceParse {
        let mut references = Vec::new();
        let mut diagnostics = Vec::new();
        for (index, line) in source.lines().enumerate() {
            let line_number = index + 1;
            let trimmed = line.trim();
            if trimmed.starts_with("[ext_resource") && !trimmed.contains("path=") {
                diagnostics.push(SimGameFormatDiagnostic {
                    code: "sim_game.format.missing_resource_path".to_string(),
                    message: "external resource is missing a path".to_string(),
                    line: Some(line_number),
                });
            }
            collect_quoted_references(trimmed, line_number, &mut references);
        }

        SimGameTextResourceParse {
            references,
            diagnostics,
        }
    }
}

fn collect_quoted_references(
    line: &str,
    line_number: usize,
    references: &mut Vec<SimGameResourceReference>,
) {
    let mut rest = line;
    while let Some(start) = rest.find('"') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find('"') else {
            break;
        };
        let value = &rest[..end];
        if value.starts_with("res://") || value.starts_with("uid://") {
            references.push(SimGameResourceReference {
                value: value.to_string(),
                line: line_number,
            });
        }
        rest = &rest[end + 1..];
    }
}
