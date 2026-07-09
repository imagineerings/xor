use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub const SIM_EXTENSION_DISABLED_PACK_CODE: &str = "world_model.extensions.disabled_pack";
pub const SIM_EXTENSION_NOT_WHITELISTED_CODE: &str = "world_model.extensions.not_whitelisted";
pub const SIM_EXTENSION_ROOT_UNREADABLE_CODE: &str = "world_model.extensions.root_unreadable";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionBacklogCatalog {
    pub schema_version: u32,
    pub source_root: String,
    pub source_category: String,
    pub captured_at: String,
    pub implementation_owner: String,
    pub native_sim_records: bool,
    pub comfyui_passthrough: bool,
    pub expected_record_count: usize,
    pub records: Vec<SimExtensionBacklogRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionBacklogRecord {
    pub source_id: String,
    pub source_path: String,
    pub source_kind: String,
    pub hook_name: String,
    pub native_surface: String,
    pub evidence_module: String,
    pub evidence_kind: String,
    pub executes_extension_code: bool,
    pub metadata_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionBacklogDiagnostic {
    pub code: String,
    pub message: String,
}

impl SimExtensionBacklogCatalog {
    pub fn validate(&self) -> Result<(), Vec<SimExtensionBacklogDiagnostic>> {
        let mut diagnostics = Vec::new();

        if self.schema_version != 1 {
            diagnostics.push(sim_extension_backlog_diagnostic(
                "world_model.extensions.backlog.invalid_schema",
                "extension backlog fixture must use schema version 1",
            ));
        }
        if self.source_root != "projects/comfy" {
            diagnostics.push(sim_extension_backlog_diagnostic(
                "world_model.extensions.backlog.invalid_source_root",
                "extension backlog fixture must preserve projects/comfy source attribution",
            ));
        }
        if !self.native_sim_records || self.comfyui_passthrough {
            diagnostics.push(sim_extension_backlog_diagnostic(
                "world_model.extensions.backlog.not_native",
                "extension backlog fixture must describe native Sim records only",
            ));
        }
        if self.records.len() != self.expected_record_count {
            diagnostics.push(sim_extension_backlog_diagnostic(
                "world_model.extensions.backlog.count_mismatch",
                format!(
                    "expected {} extension backlog records but found {}",
                    self.expected_record_count,
                    self.records.len()
                ),
            ));
        }

        let mut source_ids = BTreeSet::new();
        for record in &self.records {
            if !source_ids.insert(&record.source_id) {
                diagnostics.push(sim_extension_backlog_diagnostic(
                    "world_model.extensions.backlog.duplicate_record",
                    format!("duplicate extension source id `{}`", record.source_id),
                ));
            }
            if !record.source_path.starts_with("projects/comfy/") {
                diagnostics.push(sim_extension_backlog_diagnostic(
                    "world_model.extensions.backlog.invalid_source_path",
                    format!(
                        "source path `{}` does not preserve projects/comfy attribution",
                        record.source_path
                    ),
                ));
            }
            if record.hook_name.is_empty()
                || record.native_surface.is_empty()
                || record.evidence_module.is_empty()
                || record.evidence_kind.is_empty()
            {
                diagnostics.push(sim_extension_backlog_diagnostic(
                    "world_model.extensions.backlog.missing_evidence",
                    format!(
                        "record `{}` is missing extension evidence",
                        record.source_id
                    ),
                ));
            }
            if record.executes_extension_code || !record.metadata_only {
                diagnostics.push(sim_extension_backlog_diagnostic(
                    "world_model.extensions.backlog.unsafe_record",
                    format!(
                        "record `{}` must stay metadata-only and must not execute extension code",
                        record.source_id
                    ),
                ));
            }
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }

    pub fn surfaces(&self) -> BTreeSet<String> {
        self.records
            .iter()
            .map(|record| record.native_surface.clone())
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SimExtensionId(String);

impl SimExtensionId {
    pub fn new(value: impl AsRef<str>) -> Self {
        let mut normalized = String::new();
        let mut previous_was_separator = false;
        for character in value.as_ref().chars() {
            if character.is_ascii_alphanumeric() {
                normalized.push(character.to_ascii_lowercase());
                previous_was_separator = false;
            } else if !previous_was_separator {
                normalized.push('-');
                previous_was_separator = true;
            }
        }
        let normalized = normalized.trim_matches('-');
        if normalized.is_empty() {
            Self("extension".to_string())
        } else {
            Self(normalized.to_string())
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimExtensionSourceKind {
    Directory,
    PythonFile,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionRecord {
    pub id: SimExtensionId,
    pub display_name: String,
    pub source_path: PathBuf,
    pub source_kind: SimExtensionSourceKind,
    pub root_index: usize,
    pub load_order: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionDiagnostic {
    pub code: String,
    pub root: Option<PathBuf>,
    pub source_path: Option<PathBuf>,
    pub extension_id: Option<SimExtensionId>,
    pub message: String,
}

impl SimExtensionDiagnostic {
    fn new(
        code: impl Into<String>,
        root: Option<PathBuf>,
        source_path: Option<PathBuf>,
        extension_id: Option<SimExtensionId>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            root,
            source_path,
            extension_id,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionDiscoveryConfig {
    pub custom_nodes_enabled: bool,
    pub whitelist: BTreeSet<SimExtensionId>,
    pub disabled_suffixes: Vec<String>,
}

impl Default for SimExtensionDiscoveryConfig {
    fn default() -> Self {
        Self {
            custom_nodes_enabled: true,
            whitelist: BTreeSet::new(),
            disabled_suffixes: vec![".disabled".to_string()],
        }
    }
}

impl SimExtensionDiscoveryConfig {
    pub fn with_custom_nodes_enabled(mut self, enabled: bool) -> Self {
        self.custom_nodes_enabled = enabled;
        self
    }

    pub fn with_whitelisted_pack(mut self, pack_name: impl AsRef<str>) -> Self {
        self.whitelist.insert(SimExtensionId::new(pack_name));
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionDiscoveryReport {
    pub extensions: Vec<SimExtensionRecord>,
    pub diagnostics: Vec<SimExtensionDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimExtensionDiscovery {
    config: SimExtensionDiscoveryConfig,
}

impl Default for SimExtensionDiscovery {
    fn default() -> Self {
        Self::new(SimExtensionDiscoveryConfig::default())
    }
}

impl SimExtensionDiscovery {
    pub fn new(config: SimExtensionDiscoveryConfig) -> Self {
        Self { config }
    }

    pub fn discover_roots(&self, roots: &[PathBuf]) -> SimExtensionDiscoveryReport {
        let mut extensions = Vec::new();
        let mut diagnostics = Vec::new();

        for (root_index, root) in roots.iter().enumerate() {
            let entries = match fs::read_dir(root) {
                Ok(entries) => entries,
                Err(error) => {
                    diagnostics.push(SimExtensionDiagnostic::new(
                        SIM_EXTENSION_ROOT_UNREADABLE_CODE,
                        Some(root.clone()),
                        None,
                        None,
                        format!(
                            "extension root `{}` could not be read: {error}",
                            root.display()
                        ),
                    ));
                    continue;
                }
            };

            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        diagnostics.push(SimExtensionDiagnostic::new(
                            SIM_EXTENSION_ROOT_UNREADABLE_CODE,
                            Some(root.clone()),
                            None,
                            None,
                            format!(
                                "extension root `{}` contained an unreadable entry: {error}",
                                root.display()
                            ),
                        ));
                        continue;
                    }
                };
                let path = entry.path();
                let Some(source) = self.source_candidate(root, root_index, &path) else {
                    continue;
                };
                match source {
                    SourceCandidate::Enabled(record) => extensions.push(record),
                    SourceCandidate::Skipped(diagnostic) => diagnostics.push(diagnostic),
                }
            }
        }

        extensions.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then(left.source_kind.cmp(&right.source_kind))
                .then(left.source_path.cmp(&right.source_path))
        });
        for (load_order, record) in extensions.iter_mut().enumerate() {
            record.load_order = load_order;
        }

        SimExtensionDiscoveryReport {
            extensions,
            diagnostics,
        }
    }

    fn source_candidate(
        &self,
        root: &Path,
        root_index: usize,
        path: &Path,
    ) -> Option<SourceCandidate> {
        let file_name = path.file_name()?.to_str()?;
        if file_name.starts_with('.') {
            return None;
        }

        let disabled_name = self.disabled_name(file_name);
        let candidate_name = disabled_name.unwrap_or(file_name);
        let source_kind = if path.is_dir() {
            SimExtensionSourceKind::Directory
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("py") {
            SimExtensionSourceKind::PythonFile
        } else {
            return None;
        };
        let display_name = display_name(candidate_name, source_kind);
        let id = SimExtensionId::new(&display_name);

        if disabled_name.is_some() {
            return Some(SourceCandidate::Skipped(SimExtensionDiagnostic::new(
                SIM_EXTENSION_DISABLED_PACK_CODE,
                Some(root.to_path_buf()),
                Some(path.to_path_buf()),
                Some(id),
                "extension pack is disabled by filename suffix",
            )));
        }

        if !self.is_whitelisted_or_enabled(&id) {
            return Some(SourceCandidate::Skipped(SimExtensionDiagnostic::new(
                SIM_EXTENSION_NOT_WHITELISTED_CODE,
                Some(root.to_path_buf()),
                Some(path.to_path_buf()),
                Some(id),
                "extension pack is not enabled by custom node settings or whitelist",
            )));
        }

        Some(SourceCandidate::Enabled(SimExtensionRecord {
            id,
            display_name,
            source_path: path.to_path_buf(),
            source_kind,
            root_index,
            load_order: 0,
        }))
    }

    fn disabled_name<'a>(&self, file_name: &'a str) -> Option<&'a str> {
        self.config
            .disabled_suffixes
            .iter()
            .find_map(|suffix| file_name.strip_suffix(suffix))
    }

    fn is_whitelisted_or_enabled(&self, id: &SimExtensionId) -> bool {
        if self.config.whitelist.contains(id) {
            return true;
        }
        self.config.custom_nodes_enabled && self.config.whitelist.is_empty()
    }
}

enum SourceCandidate {
    Enabled(SimExtensionRecord),
    Skipped(SimExtensionDiagnostic),
}

fn display_name(file_name: &str, source_kind: SimExtensionSourceKind) -> String {
    match source_kind {
        SimExtensionSourceKind::Directory => file_name.to_string(),
        SimExtensionSourceKind::PythonFile => Path::new(file_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(file_name)
            .to_string(),
    }
}

fn sim_extension_backlog_diagnostic(
    code: impl Into<String>,
    message: impl Into<String>,
) -> SimExtensionBacklogDiagnostic {
    SimExtensionBacklogDiagnostic {
        code: code.into(),
        message: message.into(),
    }
}
