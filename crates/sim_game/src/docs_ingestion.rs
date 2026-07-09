use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameDocsIngestion {
    records: Vec<SimGameDocsRecord>,
}

impl SimGameDocsIngestion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_record(mut self, record: SimGameDocsRecord) -> Self {
        self.records.push(record);
        self
    }

    pub fn records(&self) -> &[SimGameDocsRecord] {
        &self.records
    }

    pub fn validate(&self) -> SimGameDocsIngestionReport {
        let diagnostics = self
            .records
            .iter()
            .enumerate()
            .flat_map(|(index, record)| record.diagnostics(index))
            .collect();

        SimGameDocsIngestionReport { diagnostics }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameDocsRecord {
    pub source: SimGameDocsSource,
    pub sim_target_path: PathBuf,
    pub source_version: Option<String>,
    pub source_license: Option<String>,
    pub source_permalink: Option<String>,
}

impl SimGameDocsRecord {
    pub fn new(source: SimGameDocsSource, sim_target_path: impl Into<PathBuf>) -> Self {
        Self {
            source,
            sim_target_path: sim_target_path.into(),
            source_version: None,
            source_license: None,
            source_permalink: None,
        }
    }

    pub fn with_source_version(mut self, source_version: impl Into<String>) -> Self {
        self.source_version = Some(source_version.into());
        self
    }

    pub fn with_source_license(mut self, source_license: impl Into<String>) -> Self {
        self.source_license = Some(source_license.into());
        self
    }

    pub fn with_source_permalink(mut self, source_permalink: impl Into<String>) -> Self {
        self.source_permalink = Some(source_permalink.into());
        self
    }

    fn diagnostics(&self, index: usize) -> Vec<SimGameDocsIngestionDiagnostic> {
        let mut diagnostics = Vec::new();

        if self.sim_target_path.as_os_str().is_empty() {
            diagnostics.push(SimGameDocsIngestionDiagnostic::new(
                index,
                "sim_target_path",
                "Sim docs target path is required",
            ));
        }

        if self.source_version.as_deref().is_none_or(str::is_empty) {
            diagnostics.push(SimGameDocsIngestionDiagnostic::new(
                index,
                "source_version",
                "source version is required",
            ));
        }

        if self.source_license.as_deref().is_none_or(str::is_empty) {
            diagnostics.push(SimGameDocsIngestionDiagnostic::new(
                index,
                "source_license",
                "source license is required",
            ));
        }

        if self.source_permalink.as_deref().is_none_or(str::is_empty) {
            diagnostics.push(SimGameDocsIngestionDiagnostic::new(
                index,
                "source_permalink",
                "source permalink is required",
            ));
        }

        diagnostics.extend(self.source.diagnostics(index));
        diagnostics
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameDocsSource {
    GodotClassReference {
        class_name: String,
        source_path: PathBuf,
    },
    GodotManual {
        title: String,
        source_path: PathBuf,
    },
    WorldModelReference {
        title: String,
        source_path: PathBuf,
    },
}

impl SimGameDocsSource {
    pub fn source_path(&self) -> &Path {
        match self {
            Self::GodotClassReference { source_path, .. }
            | Self::GodotManual { source_path, .. }
            | Self::WorldModelReference { source_path, .. } => source_path,
        }
    }

    fn diagnostics(&self, index: usize) -> Vec<SimGameDocsIngestionDiagnostic> {
        let mut diagnostics = Vec::new();

        match self {
            Self::GodotClassReference {
                class_name,
                source_path,
            } => {
                if class_name.trim().is_empty() {
                    diagnostics.push(SimGameDocsIngestionDiagnostic::new(
                        index,
                        "source.class_name",
                        "Godot class reference name is required",
                    ));
                }
                if source_path.as_os_str().is_empty() {
                    diagnostics.push(SimGameDocsIngestionDiagnostic::new(
                        index,
                        "source.source_path",
                        "source docs path is required",
                    ));
                }
            }
            Self::GodotManual { title, source_path }
            | Self::WorldModelReference { title, source_path } => {
                if title.trim().is_empty() {
                    diagnostics.push(SimGameDocsIngestionDiagnostic::new(
                        index,
                        "source.title",
                        "source docs title is required",
                    ));
                }
                if source_path.as_os_str().is_empty() {
                    diagnostics.push(SimGameDocsIngestionDiagnostic::new(
                        index,
                        "source.source_path",
                        "source docs path is required",
                    ));
                }
            }
        }

        diagnostics
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimGameDocsIngestionReport {
    pub diagnostics: Vec<SimGameDocsIngestionDiagnostic>,
}

impl SimGameDocsIngestionReport {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimGameDocsIngestionDiagnostic {
    pub index: usize,
    pub field: &'static str,
    pub message: String,
}

impl SimGameDocsIngestionDiagnostic {
    fn new(index: usize, field: &'static str, message: impl Into<String>) -> Self {
        Self {
            index,
            field,
            message: message.into(),
        }
    }
}
