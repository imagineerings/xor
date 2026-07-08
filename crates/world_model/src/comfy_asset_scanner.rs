use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ComfyAssetScanRootKind {
    Models,
    Input,
    Output,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetScannedFile {
    pub root_kind: ComfyAssetScanRootKind,
    pub relative_path: PathBuf,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub hash: Option<String>,
    pub modified_at_ms: Option<u64>,
}

impl ComfyAssetScannedFile {
    pub fn new(
        root_kind: ComfyAssetScanRootKind,
        relative_path: impl Into<PathBuf>,
        size_bytes: u64,
    ) -> Self {
        Self {
            root_kind,
            relative_path: relative_path.into(),
            size_bytes,
            mime_type: None,
            hash: None,
            modified_at_ms: None,
        }
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.hash = Some(hash.into());
        self
    }

    pub fn with_modified_at_ms(mut self, modified_at_ms: u64) -> Self {
        self.modified_at_ms = Some(modified_at_ms);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetScanRoot {
    pub kind: ComfyAssetScanRootKind,
    pub path: PathBuf,
}

impl ComfyAssetScanRoot {
    pub fn new(kind: ComfyAssetScanRootKind, path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            path: path.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetFilesystemScanner {
    pub roots: Vec<ComfyAssetScanRoot>,
    pub files: Vec<ComfyAssetScannedFile>,
}

impl ComfyAssetFilesystemScanner {
    pub fn with_root(mut self, root: ComfyAssetScanRoot) -> Self {
        self.roots.push(root);
        self
    }

    pub fn with_file(mut self, file: ComfyAssetScannedFile) -> Self {
        self.files.push(file);
        self
    }

    pub fn root_path(&self, kind: ComfyAssetScanRootKind) -> Option<&PathBuf> {
        self.roots
            .iter()
            .find(|root| root.kind == kind)
            .map(|root| &root.path)
    }

    pub fn full_path(&self, file: &ComfyAssetScannedFile) -> Option<PathBuf> {
        self.root_path(file.root_kind)
            .map(|root| root.join(&file.relative_path))
    }
}
