use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub const SIM_DIAGNOSTIC_UNAPPROVED_ROOT_CODE: &str = "world_model.diagnostics.unapproved_root";
pub const SIM_DIAGNOSTIC_PATH_ESCAPE_CODE: &str = "world_model.diagnostics.path_escape";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimDiagnosticRootKind {
    Input,
    Output,
    Temp,
    Model,
    User,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimDiagnosticEndpointStability {
    InternalUnstable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimTerminalMetadata {
    pub columns: u16,
    pub rows: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimDiagnosticLogStream {
    Stdout,
    Stderr,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiagnosticLogEntry {
    pub stream: SimDiagnosticLogStream,
    pub message: String,
}

impl SimDiagnosticLogEntry {
    pub fn new(stream: SimDiagnosticLogStream, message: impl Into<String>) -> Self {
        Self {
            stream,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiagnosticLogReport {
    pub raw: String,
    pub formatted: String,
    pub terminal: Option<SimTerminalMetadata>,
    pub stability: SimDiagnosticEndpointStability,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiagnosticFolder {
    pub root: SimDiagnosticRootKind,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiagnosticFolderReport {
    pub folders: Vec<SimDiagnosticFolder>,
    pub stability: SimDiagnosticEndpointStability,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimDiagnosticRecentFileKind {
    Input,
    Output,
    Temp,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiagnosticRecentFile {
    pub root: SimDiagnosticRootKind,
    pub relative_path: PathBuf,
    pub kind: SimDiagnosticRecentFileKind,
}

impl SimDiagnosticRecentFile {
    pub fn new(
        root: SimDiagnosticRootKind,
        relative_path: impl Into<PathBuf>,
        kind: SimDiagnosticRecentFileKind,
    ) -> Self {
        Self {
            root,
            relative_path: relative_path.into(),
            kind,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiagnosticResolvedRecentFile {
    pub root: SimDiagnosticRootKind,
    pub path: PathBuf,
    pub kind: SimDiagnosticRecentFileKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiagnosticRecentFilesReport {
    pub files: Vec<SimDiagnosticResolvedRecentFile>,
    pub diagnostics: Vec<SimDiagnosticsAdapterDiagnostic>,
    pub stability: SimDiagnosticEndpointStability,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiagnosticsAdapterDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimDiagnosticsAdapter {
    approved_roots: BTreeMap<SimDiagnosticRootKind, PathBuf>,
    terminal: Option<SimTerminalMetadata>,
}

impl SimDiagnosticsAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root(mut self, root: SimDiagnosticRootKind, path: impl Into<PathBuf>) -> Self {
        self.approved_roots.insert(root, path.into());
        self
    }

    pub fn with_terminal(mut self, terminal: SimTerminalMetadata) -> Self {
        self.terminal = Some(terminal);
        self
    }

    pub fn logs(
        &self,
        entries: impl IntoIterator<Item = SimDiagnosticLogEntry>,
    ) -> SimDiagnosticLogReport {
        let entries = entries.into_iter().collect::<Vec<_>>();
        SimDiagnosticLogReport {
            raw: render_raw_logs(&entries),
            formatted: render_formatted_logs(&entries),
            terminal: self.terminal,
            stability: SimDiagnosticEndpointStability::InternalUnstable,
        }
    }

    pub fn approved_folders(&self) -> SimDiagnosticFolderReport {
        let folders = self
            .approved_roots
            .iter()
            .map(|(root, path)| SimDiagnosticFolder {
                root: *root,
                path: path.clone(),
            })
            .collect();
        SimDiagnosticFolderReport {
            folders,
            stability: SimDiagnosticEndpointStability::InternalUnstable,
        }
    }

    pub fn recent_files(
        &self,
        files: impl IntoIterator<Item = SimDiagnosticRecentFile>,
    ) -> SimDiagnosticRecentFilesReport {
        let mut resolved_files = Vec::new();
        let mut diagnostics = Vec::new();

        for file in files {
            match self.resolve_recent_file(file) {
                Ok(file) => resolved_files.push(file),
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
        }

        SimDiagnosticRecentFilesReport {
            files: resolved_files,
            diagnostics,
            stability: SimDiagnosticEndpointStability::InternalUnstable,
        }
    }

    fn resolve_recent_file(
        &self,
        file: SimDiagnosticRecentFile,
    ) -> Result<SimDiagnosticResolvedRecentFile, SimDiagnosticsAdapterDiagnostic> {
        let root = self.approved_roots.get(&file.root).ok_or_else(|| {
            diagnostic(
                SIM_DIAGNOSTIC_UNAPPROVED_ROOT_CODE,
                format!("diagnostic root {:?} is not approved", file.root),
            )
        })?;

        if is_escaping_path(&file.relative_path) {
            return Err(diagnostic(
                SIM_DIAGNOSTIC_PATH_ESCAPE_CODE,
                "recent diagnostic files must stay inside approved roots",
            ));
        }

        Ok(SimDiagnosticResolvedRecentFile {
            root: file.root,
            path: root.join(file.relative_path),
            kind: file.kind,
        })
    }
}

fn render_raw_logs(entries: &[SimDiagnosticLogEntry]) -> String {
    entries
        .iter()
        .map(|entry| entry.message.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_formatted_logs(entries: &[SimDiagnosticLogEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("[{}] {}", stream_label(entry.stream), entry.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn stream_label(stream: SimDiagnosticLogStream) -> &'static str {
    match stream {
        SimDiagnosticLogStream::Stdout => "stdout",
        SimDiagnosticLogStream::Stderr => "stderr",
        SimDiagnosticLogStream::Internal => "internal",
    }
}

fn is_escaping_path(path: &Path) -> bool {
    path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn diagnostic(code: &str, message: impl Into<String>) -> SimDiagnosticsAdapterDiagnostic {
    SimDiagnosticsAdapterDiagnostic {
        code: code.to_string(),
        message: message.into(),
    }
}
