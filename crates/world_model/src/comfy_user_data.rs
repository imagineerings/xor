use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ComfyAssetOwnerId;

pub const USER_DATA_FORBIDDEN_CODE: &str = "world_model.comfy_user_data.forbidden";
pub const USER_DATA_NOT_FOUND_CODE: &str = "world_model.comfy_user_data.not_found";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyUserDataDiagnostic {
    pub code: String,
    pub path: Option<PathBuf>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyUserDataEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub size_bytes: u64,
    pub is_directory: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyUserDataPathParts {
    pub directory: PathBuf,
    pub file_name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfyUserDataStore {
    files: BTreeMap<(ComfyAssetOwnerId, PathBuf), Vec<u8>>,
    settings: BTreeMap<ComfyAssetOwnerId, Value>,
}

impl ComfyUserDataStore {
    pub fn write_file(
        &mut self,
        owner_id: ComfyAssetOwnerId,
        path: &Path,
        contents: Vec<u8>,
    ) -> Result<ComfyUserDataEntry, ComfyUserDataDiagnostic> {
        let path = normalize_user_path(path)?;
        let entry = file_entry(path.clone(), contents.len() as u64);
        self.files.insert((owner_id, path), contents);
        Ok(entry)
    }

    pub fn read_file(
        &self,
        owner_id: &ComfyAssetOwnerId,
        path: &Path,
    ) -> Result<Vec<u8>, ComfyUserDataDiagnostic> {
        let path = normalize_user_path(path)?;
        self.files
            .get(&(owner_id.clone(), path.clone()))
            .cloned()
            .ok_or_else(|| not_found(path))
    }

    pub fn delete_file(
        &mut self,
        owner_id: &ComfyAssetOwnerId,
        path: &Path,
    ) -> Result<bool, ComfyUserDataDiagnostic> {
        let path = normalize_user_path(path)?;
        Ok(self.files.remove(&(owner_id.clone(), path)).is_some())
    }

    pub fn move_file(
        &mut self,
        owner_id: &ComfyAssetOwnerId,
        from: &Path,
        to: &Path,
    ) -> Result<ComfyUserDataEntry, ComfyUserDataDiagnostic> {
        let from = normalize_user_path(from)?;
        let to = normalize_user_path(to)?;
        let contents = self
            .files
            .remove(&(owner_id.clone(), from.clone()))
            .ok_or_else(|| not_found(from))?;
        let entry = file_entry(to.clone(), contents.len() as u64);
        self.files.insert((owner_id.clone(), to), contents);
        Ok(entry)
    }

    pub fn list_files(
        &self,
        owner_id: &ComfyAssetOwnerId,
        root: &Path,
        recursive: bool,
    ) -> Result<Vec<ComfyUserDataEntry>, ComfyUserDataDiagnostic> {
        let root = normalize_user_path(root)?;
        let mut entries = Vec::new();
        for ((owner, path), contents) in &self.files {
            if owner != owner_id || !path.starts_with(&root) {
                continue;
            }
            if !recursive && path.parent() != Some(root.as_path()) {
                continue;
            }
            entries.push(file_entry(path.clone(), contents.len() as u64));
        }
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(entries)
    }

    pub fn path_parts(path: &Path) -> Result<ComfyUserDataPathParts, ComfyUserDataDiagnostic> {
        let path = normalize_user_path(path)?;
        Ok(ComfyUserDataPathParts {
            directory: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
            file_name: path
                .file_name()
                .and_then(|file_name| file_name.to_str())
                .unwrap_or_default()
                .to_string(),
        })
    }

    pub fn read_settings(&self, owner_id: &ComfyAssetOwnerId) -> Value {
        self.settings
            .get(owner_id)
            .cloned()
            .unwrap_or_else(|| Value::Object(Default::default()))
    }

    pub fn write_settings(&mut self, owner_id: ComfyAssetOwnerId, settings: Value) -> Value {
        self.settings.insert(owner_id, settings.clone());
        settings
    }
}

pub fn normalize_user_path(path: &Path) -> Result<PathBuf, ComfyUserDataDiagnostic> {
    if path.is_absolute() {
        return Err(forbidden(path, "user data paths must be relative"));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(forbidden(path, "user data path escapes the storage root"));
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(forbidden(path, "user data path cannot be empty"));
    }
    Ok(normalized)
}

fn file_entry(path: PathBuf, size_bytes: u64) -> ComfyUserDataEntry {
    let file_name = path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .unwrap_or_default()
        .to_string();
    ComfyUserDataEntry {
        path,
        file_name,
        size_bytes,
        is_directory: false,
    }
}

fn forbidden(path: &Path, message: impl Into<String>) -> ComfyUserDataDiagnostic {
    ComfyUserDataDiagnostic {
        code: USER_DATA_FORBIDDEN_CODE.to_string(),
        path: Some(path.to_path_buf()),
        message: message.into(),
    }
}

fn not_found(path: PathBuf) -> ComfyUserDataDiagnostic {
    ComfyUserDataDiagnostic {
        code: USER_DATA_NOT_FOUND_CODE.to_string(),
        path: Some(path),
        message: "user data file was not found".to_string(),
    }
}
