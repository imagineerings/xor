use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{ModelCategory, ModelFileRef};

pub const DEFAULT_SAFETENSORS_HEADER_LIMIT_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelMetadataSummary {
    pub preview: Option<ModelPreviewRef>,
    pub safetensors: Option<SafetensorsHeaderMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelPreviewRef {
    pub category: ModelCategory,
    pub root_index: usize,
    pub model_relative_path: PathBuf,
    pub preview_relative_path: PathBuf,
    pub content_type: String,
    pub route_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SafetensorsHeaderMetadata {
    pub header_byte_len: u64,
    pub tensor_count: usize,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug)]
pub enum ModelMetadataError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    HeaderTooLarge {
        path: PathBuf,
        header_byte_len: u64,
        limit_bytes: u64,
    },
    InvalidSafetensorsHeader {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl std::fmt::Display for ModelMetadataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "failed to read model metadata path `{}`: {source}",
                    path.display()
                )
            }
            Self::HeaderTooLarge {
                path,
                header_byte_len,
                limit_bytes,
            } => {
                write!(
                    formatter,
                    "safetensors header for `{}` is {header_byte_len} bytes, exceeding the {limit_bytes} byte limit",
                    path.display()
                )
            }
            Self::InvalidSafetensorsHeader { path, source } => {
                write!(
                    formatter,
                    "failed to parse safetensors header for `{}`: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ModelMetadataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidSafetensorsHeader { source, .. } => Some(source),
            Self::HeaderTooLarge { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComfyModelMetadataReader {
    safetensors_header_limit_bytes: u64,
}

impl ComfyModelMetadataReader {
    pub fn new() -> Self {
        Self {
            safetensors_header_limit_bytes: DEFAULT_SAFETENSORS_HEADER_LIMIT_BYTES,
        }
    }

    pub fn with_safetensors_header_limit_bytes(mut self, limit_bytes: u64) -> Self {
        self.safetensors_header_limit_bytes = limit_bytes;
        self
    }

    pub fn read_summary(
        &self,
        file: &ModelFileRef,
    ) -> Result<ModelMetadataSummary, ModelMetadataError> {
        Ok(ModelMetadataSummary {
            preview: self.preview_for_file(file)?,
            safetensors: self.safetensors_metadata_for_file(file)?,
        })
    }

    pub fn preview_for_file(
        &self,
        file: &ModelFileRef,
    ) -> Result<Option<ModelPreviewRef>, ModelMetadataError> {
        for candidate in preview_candidates(&file.relative_path) {
            let preview_path = file.root.join(&candidate);
            let metadata = match fs::metadata(&preview_path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(source) => {
                    return Err(ModelMetadataError::Io {
                        path: preview_path,
                        source,
                    });
                }
            };

            if metadata.is_file() {
                return Ok(
                    content_type_for_path(&candidate).map(|content_type| ModelPreviewRef {
                        category: file.category,
                        root_index: file.root_index,
                        model_relative_path: file.relative_path.clone(),
                        preview_relative_path: candidate.clone(),
                        content_type: content_type.to_string(),
                        route_path: preview_route_path(file.category, file.root_index, &candidate),
                    }),
                );
            }
        }

        Ok(None)
    }

    pub fn safetensors_metadata_for_file(
        &self,
        file: &ModelFileRef,
    ) -> Result<Option<SafetensorsHeaderMetadata>, ModelMetadataError> {
        let extension = file
            .relative_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        if extension.as_deref() != Some("safetensors") {
            return Ok(None);
        }

        let mut model_file =
            fs::File::open(&file.full_path).map_err(|source| ModelMetadataError::Io {
                path: file.full_path.clone(),
                source,
            })?;
        let mut header_len_bytes = [0_u8; 8];
        model_file
            .read_exact(&mut header_len_bytes)
            .map_err(|source| ModelMetadataError::Io {
                path: file.full_path.clone(),
                source,
            })?;

        let header_byte_len = u64::from_le_bytes(header_len_bytes);
        if header_byte_len > self.safetensors_header_limit_bytes {
            return Err(ModelMetadataError::HeaderTooLarge {
                path: file.full_path.clone(),
                header_byte_len,
                limit_bytes: self.safetensors_header_limit_bytes,
            });
        }

        let header_len =
            usize::try_from(header_byte_len).map_err(|source| ModelMetadataError::Io {
                path: file.full_path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
            })?;
        let mut header_bytes = vec![0_u8; header_len];
        model_file
            .read_exact(&mut header_bytes)
            .map_err(|source| ModelMetadataError::Io {
                path: file.full_path.clone(),
                source,
            })?;

        let header: serde_json::Value =
            serde_json::from_slice(&header_bytes).map_err(|source| {
                ModelMetadataError::InvalidSafetensorsHeader {
                    path: file.full_path.clone(),
                    source,
                }
            })?;
        Ok(Some(parse_safetensors_header(header_byte_len, &header)))
    }
}

impl Default for ComfyModelMetadataReader {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_safetensors_header(
    header_byte_len: u64,
    header: &serde_json::Value,
) -> SafetensorsHeaderMetadata {
    let mut tensor_count = 0;
    let mut metadata = BTreeMap::new();

    if let Some(entries) = header.as_object() {
        for (key, value) in entries {
            if key == "__metadata__" {
                if let Some(metadata_entries) = value.as_object() {
                    for (metadata_key, metadata_value) in metadata_entries {
                        if let Some(metadata_value) = metadata_value.as_str() {
                            metadata.insert(metadata_key.clone(), metadata_value.to_string());
                        } else {
                            metadata.insert(metadata_key.clone(), metadata_value.to_string());
                        }
                    }
                }
            } else {
                tensor_count += 1;
            }
        }
    }

    SafetensorsHeaderMetadata {
        header_byte_len,
        tensor_count,
        metadata,
    }
}

fn preview_candidates(model_relative_path: &Path) -> Vec<PathBuf> {
    let parent = model_relative_path
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let stem = model_relative_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();

    let mut candidates = Vec::new();
    for suffix in ["png", "jpg", "jpeg", "webp", "gif"] {
        candidates.push(parent.join(format!("{stem}.{suffix}")));
    }
    for suffix in ["png", "jpg", "jpeg", "webp", "gif"] {
        candidates.push(parent.join(format!("{stem}.preview.{suffix}")));
    }
    candidates
}

fn content_type_for_path(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => Some("image/png"),
        Some("jpg") | Some("jpeg") => Some("image/jpeg"),
        Some("webp") => Some("image/webp"),
        Some("gif") => Some("image/gif"),
        _ => None,
    }
}

fn preview_route_path(category: ModelCategory, root_index: usize, relative_path: &Path) -> String {
    let mut route = format!(
        "/world-model/models/{}/{root_index}/previews",
        category.canonical_name()
    );
    for component in relative_path.components() {
        if let Component::Normal(component) = component {
            route.push('/');
            route.push_str(&url_path_encode(&component.to_string_lossy()));
        }
    }
    route
}

fn url_path_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}
