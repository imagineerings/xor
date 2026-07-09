use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::integration::{SimScriptLanguageConfig, simscript_language_config};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimScriptFileKind {
    Native,
    ImportedGdSource,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimScriptFileClassification {
    pub path: PathBuf,
    pub kind: SimScriptFileKind,
    pub language_name: String,
    pub migration_source_format: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimScriptLanguageSupport {
    config: SimScriptLanguageConfig,
}

impl SimScriptLanguageSupport {
    pub fn new(config: SimScriptLanguageConfig) -> Self {
        Self { config }
    }

    pub fn native() -> Self {
        Self::new(simscript_language_config())
    }

    pub fn config(&self) -> &SimScriptLanguageConfig {
        &self.config
    }

    pub fn classify_path(&self, path: impl AsRef<Path>) -> Option<SimScriptFileClassification> {
        let path = path.as_ref();
        let extension = path.extension()?.to_str()?;
        match extension {
            "simscript" => Some(SimScriptFileClassification {
                path: path.to_path_buf(),
                kind: SimScriptFileKind::Native,
                language_name: self.config.name.clone(),
                migration_source_format: None,
            }),
            "gd" => Some(SimScriptFileClassification {
                path: path.to_path_buf(),
                kind: SimScriptFileKind::ImportedGdSource,
                language_name: self.config.name.clone(),
                migration_source_format: Some("gdscript".to_string()),
            }),
            _ => None,
        }
    }

    pub fn lsp_adapter_name(&self) -> Option<&str> {
        self.config.lsp_adapter.as_deref()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameDocsIndex {
    entries: Vec<SimGameDocsEntry>,
}

impl SimGameDocsIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_entry(mut self, entry: SimGameDocsEntry) -> Self {
        self.entries.push(entry);
        self
    }

    pub fn entries(&self) -> &[SimGameDocsEntry] {
        &self.entries
    }

    pub fn lookup(&self, query: impl AsRef<str>) -> Vec<&SimGameDocsEntry> {
        let query = query.as_ref().to_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                entry.symbol.to_lowercase().contains(&query)
                    || entry.summary.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn sim_api_entries(&self) -> Vec<&SimGameDocsEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.scope == SimGameDocsScope::PrimarySimApi)
            .collect()
    }

    pub fn migration_reference_entries(&self) -> Vec<&SimGameDocsEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.scope == SimGameDocsScope::MigrationReference)
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameDocsEntry {
    pub symbol: String,
    pub summary: String,
    pub scope: SimGameDocsScope,
    pub source_path: Option<PathBuf>,
}

impl SimGameDocsEntry {
    pub fn sim_api(symbol: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            summary: summary.into(),
            scope: SimGameDocsScope::PrimarySimApi,
            source_path: None,
        }
    }

    pub fn migration_reference(
        symbol: impl Into<String>,
        summary: impl Into<String>,
        source_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            summary: summary.into(),
            scope: SimGameDocsScope::MigrationReference,
            source_path: Some(source_path.into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameDocsScope {
    PrimarySimApi,
    MigrationReference,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NaturalLanguageGameAuthoring;

impl NaturalLanguageGameAuthoring {
    pub fn new() -> Self {
        Self
    }

    pub fn draft(&self, instruction: impl AsRef<str>) -> SimScriptAuthoringDraft {
        let instruction = instruction.as_ref().trim();
        if instruction.is_empty() {
            return SimScriptAuthoringDraft {
                status: SimScriptAuthoringStatus::NeedsClarification,
                instruction: String::new(),
                simscript: String::new(),
                diagnostics: vec![SimScriptAuthoringDiagnostic {
                    message: "natural-language instruction is required".to_string(),
                }],
            };
        }

        SimScriptAuthoringDraft {
            status: SimScriptAuthoringStatus::Draft,
            instruction: instruction.to_string(),
            simscript: format!(
                "behavior GeneratedBehavior:\n    intent \"{}\"\n    on ready:\n        pass\n",
                escape_simscript_string(instruction)
            ),
            diagnostics: Vec::new(),
        }
    }

    pub fn diff(
        &self,
        current_simscript: impl AsRef<str>,
        instruction: impl AsRef<str>,
    ) -> SimScriptDiff {
        let current_simscript = current_simscript.as_ref().to_string();
        let draft = self.draft(instruction);
        let kind = if draft.status == SimScriptAuthoringStatus::NeedsClarification {
            SimScriptDiffKind::NonDestructiveDraft
        } else if current_simscript.is_empty() {
            SimScriptDiffKind::Create
        } else {
            SimScriptDiffKind::Update
        };

        SimScriptDiff {
            kind,
            original: current_simscript,
            generated: draft.simscript.clone(),
            draft,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimScriptAuthoringStatus {
    Draft,
    NeedsClarification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimScriptAuthoringDraft {
    pub status: SimScriptAuthoringStatus,
    pub instruction: String,
    pub simscript: String,
    pub diagnostics: Vec<SimScriptAuthoringDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimScriptAuthoringDiagnostic {
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimScriptDiffKind {
    Create,
    Update,
    NonDestructiveDraft,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimScriptDiff {
    pub kind: SimScriptDiffKind,
    pub original: String,
    pub generated: String,
    pub draft: SimScriptAuthoringDraft,
}

fn escape_simscript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
