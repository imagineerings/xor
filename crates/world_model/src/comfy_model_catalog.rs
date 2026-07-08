use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    ComfyModelFolderRegistry, ModelCategory, ModelFileRef, ModelFolderError, ModelFolderInfo,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelFileSummary {
    pub category: ModelCategory,
    pub root_index: usize,
    pub path_index: usize,
    pub relative_name: String,
    pub relative_path: PathBuf,
    pub size_bytes: u64,
    pub created_at_ms: Option<u64>,
    pub modified_at_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelRootSnapshot {
    pub root_index: usize,
    pub root: PathBuf,
    pub exists: bool,
    pub modified_at_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelCatalogSnapshot {
    pub category: ModelCategory,
    pub roots: Vec<ModelRootSnapshot>,
    pub files: Vec<ModelFileSummary>,
}

impl ModelCatalogSnapshot {
    pub fn cache_key(&self) -> String {
        let root_keys = self
            .roots
            .iter()
            .map(|root| {
                format!(
                    "{}:{}:{}",
                    root.root_index,
                    root.exists,
                    root.modified_at_ms
                        .map(|modified_at_ms| modified_at_ms.to_string())
                        .unwrap_or_else(|| "missing".to_string())
                )
            })
            .collect::<Vec<_>>()
            .join("|");
        format!("{}:{root_keys}", self.category.canonical_name())
    }
}

#[derive(Debug)]
pub enum ModelCatalogError {
    Folder(ModelFolderError),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for ModelCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Folder(error) => write!(formatter, "{error}"),
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "failed to read model catalog path `{}`: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ModelCatalogError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Folder(error) => Some(error),
            Self::Io { source, .. } => Some(source),
        }
    }
}

impl From<ModelFolderError> for ModelCatalogError {
    fn from(error: ModelFolderError) -> Self {
        Self::Folder(error)
    }
}

pub struct ComfyModelCatalog<'a> {
    registry: &'a ComfyModelFolderRegistry,
}

impl<'a> ComfyModelCatalog<'a> {
    pub fn new(registry: &'a ComfyModelFolderRegistry) -> Self {
        Self { registry }
    }

    pub fn list_category(
        &self,
        category_name: &str,
    ) -> Result<ModelCatalogSnapshot, ModelCatalogError> {
        let category = self.registry.category_for_name(category_name)?;
        self.list_model_category(category)
    }

    pub fn list_model_category(
        &self,
        category: ModelCategory,
    ) -> Result<ModelCatalogSnapshot, ModelCatalogError> {
        let folder = self.registry.folder(category).ok_or_else(|| {
            ModelFolderError::UnknownCategory(category.canonical_name().to_string())
        })?;
        let roots = self.root_snapshots(folder)?;
        let mut files = Vec::new();

        for (root_index, root) in folder.roots.iter().enumerate() {
            if !root.exists() {
                continue;
            }
            self.collect_visible_files(folder, root_index, root, root, &mut files)?;
        }

        files.sort_by(|left, right| {
            left.root_index
                .cmp(&right.root_index)
                .then_with(|| left.relative_path.cmp(&right.relative_path))
        });
        for (path_index, file) in files.iter_mut().enumerate() {
            file.path_index = path_index;
        }

        Ok(ModelCatalogSnapshot {
            category,
            roots,
            files,
        })
    }

    pub fn resolve_summary(
        &self,
        summary: &ModelFileSummary,
    ) -> Result<ModelFileRef, ModelCatalogError> {
        self.resolve_at_root(summary.category, summary.root_index, &summary.relative_path)
    }

    pub fn resolve_at_root(
        &self,
        category: ModelCategory,
        root_index: usize,
        relative_path: impl AsRef<Path>,
    ) -> Result<ModelFileRef, ModelCatalogError> {
        let relative_path = relative_path.as_ref();
        if !is_safe_relative_path(relative_path) {
            return Err(ModelFolderError::UnsafeRelativePath {
                relative_path: relative_path.to_path_buf(),
            }
            .into());
        }

        let folder = self.registry.folder(category).ok_or_else(|| {
            ModelFolderError::UnknownCategory(category.canonical_name().to_string())
        })?;
        validate_extension(folder, relative_path)?;
        let root = folder
            .roots
            .get(root_index)
            .cloned()
            .ok_or(ModelFolderError::MissingRoot { category })?;

        Ok(ModelFileRef {
            category,
            root_index,
            full_path: root.join(relative_path),
            root,
            relative_path: relative_path.to_path_buf(),
        })
    }

    fn root_snapshots(
        &self,
        folder: &ModelFolderInfo,
    ) -> Result<Vec<ModelRootSnapshot>, ModelCatalogError> {
        folder
            .roots
            .iter()
            .enumerate()
            .map(|(root_index, root)| {
                if root.exists() {
                    let metadata = fs::metadata(root).map_err(|source| ModelCatalogError::Io {
                        path: root.clone(),
                        source,
                    })?;
                    Ok(ModelRootSnapshot {
                        root_index,
                        root: root.clone(),
                        exists: true,
                        modified_at_ms: metadata.modified().ok().and_then(system_time_ms),
                    })
                } else {
                    Ok(ModelRootSnapshot {
                        root_index,
                        root: root.clone(),
                        exists: false,
                        modified_at_ms: None,
                    })
                }
            })
            .collect()
    }

    fn collect_visible_files(
        &self,
        folder: &ModelFolderInfo,
        root_index: usize,
        root: &Path,
        current: &Path,
        files: &mut Vec<ModelFileSummary>,
    ) -> Result<(), ModelCatalogError> {
        let mut entries = fs::read_dir(current)
            .map_err(|source| ModelCatalogError::Io {
                path: current.to_path_buf(),
                source,
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| ModelCatalogError::Io {
                path: current.to_path_buf(),
                source,
            })?;
        entries.sort_by_key(|entry| entry.path());

        for entry in entries {
            let path = entry.path();
            if is_hidden_path(&path, root) {
                continue;
            }

            let metadata = entry.metadata().map_err(|source| ModelCatalogError::Io {
                path: path.clone(),
                source,
            })?;
            if metadata.is_dir() {
                self.collect_visible_files(folder, root_index, root, &path, files)?;
            } else if metadata.is_file() {
                let relative_path =
                    path.strip_prefix(root)
                        .map_err(|_| ModelFolderError::UnsafeRelativePath {
                            relative_path: path.clone(),
                        })?;
                if validate_extension(folder, relative_path).is_err() {
                    continue;
                }

                files.push(ModelFileSummary {
                    category: folder.category,
                    root_index,
                    path_index: 0,
                    relative_name: relative_path.to_string_lossy().replace('\\', "/"),
                    relative_path: relative_path.to_path_buf(),
                    size_bytes: metadata.len(),
                    created_at_ms: metadata.created().ok().and_then(system_time_ms),
                    modified_at_ms: metadata.modified().ok().and_then(system_time_ms),
                });
            }
        }

        Ok(())
    }
}

fn validate_extension(
    folder: &ModelFolderInfo,
    relative_path: &Path,
) -> Result<(), ModelFolderError> {
    let extension = relative_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());
    if let Some(extension) = extension.as_ref()
        && !folder.allowed_extensions.contains(extension)
    {
        return Err(ModelFolderError::ExtensionNotAllowed {
            category: folder.category,
            extension: Some(extension.clone()),
        });
    } else if extension.is_none() && !folder.allowed_extensions.is_empty() {
        return Err(ModelFolderError::ExtensionNotAllowed {
            category: folder.category,
            extension: None,
        });
    }

    Ok(())
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn is_hidden_path(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root).ok().is_some_and(|relative_path| {
        relative_path.components().any(|component| {
            matches!(
                component,
                Component::Normal(name) if name.to_string_lossy().starts_with('.')
            )
        })
    })
}

fn system_time_ms(system_time: SystemTime) -> Option<u64> {
    system_time
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}
