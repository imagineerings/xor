use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameImportLink {
    pub import_path: String,
    pub source_file: Option<String>,
    pub generated_files: Vec<String>,
    pub diagnostics: Vec<SimGameImportDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameImportDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameImportMetadataLinker;

impl SimGameImportMetadataLinker {
    pub fn new() -> Self {
        Self
    }

    pub fn link(&self, import_path: impl AsRef<Path>, source: &str) -> SimGameImportLink {
        let import_path = import_path.as_ref().display().to_string();
        let source_file = parse_string_value(source, "source_file");
        let generated_files = parse_string_list(source, "dest_files");
        let mut diagnostics = Vec::new();
        if source_file.is_none() {
            diagnostics.push(SimGameImportDiagnostic {
                code: "sim_game.import.missing_source_file".to_string(),
                message: "import metadata is missing source_file".to_string(),
            });
        }
        if generated_files.is_empty() {
            diagnostics.push(SimGameImportDiagnostic {
                code: "sim_game.import.missing_dest_files".to_string(),
                message: "import metadata is missing dest_files".to_string(),
            });
        }

        SimGameImportLink {
            import_path,
            source_file,
            generated_files,
            diagnostics,
        }
    }
}

fn parse_string_value(source: &str, key: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let line = line.trim();
        let value = line
            .strip_prefix(key)?
            .trim_start()
            .strip_prefix('=')?
            .trim();
        Some(value.trim_matches('"').to_string()).filter(|value| !value.is_empty())
    })
}

fn parse_string_list(source: &str, key: &str) -> Vec<String> {
    let Some(raw) = source.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix(key)?.trim_start().strip_prefix('=')
    }) else {
        return Vec::new();
    };
    let raw = raw.trim().trim_start_matches('[').trim_end_matches(']');
    raw.split(',')
        .map(|value| value.trim().trim_matches('"'))
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}
