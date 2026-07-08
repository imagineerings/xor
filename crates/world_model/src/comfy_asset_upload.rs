use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ComfyAssetCacheState, ComfyAssetQueryDiagnostic, ComfyAssetReferenceId,
    ComfyAssetReferenceRequest, ComfyAssetValidatedHash, normalize_asset_tag,
};

pub const ASSET_UPLOAD_INVALID_FIELD_CODE: &str = "world_model.comfy_assets.invalid_upload_field";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetUploadDiagnostic {
    pub code: String,
    pub field: String,
    pub message: String,
}

impl From<ComfyAssetQueryDiagnostic> for ComfyAssetUploadDiagnostic {
    fn from(error: ComfyAssetQueryDiagnostic) -> Self {
        Self {
            code: error.code,
            field: error.field,
            message: error.message,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetUploadRequest {
    pub file_name: String,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub known_hash: Option<ComfyAssetValidatedHash>,
    pub tags: Vec<String>,
    pub user_metadata: BTreeMap<String, serde_json::Value>,
    pub preview_id: Option<ComfyAssetReferenceId>,
    pub cache_state: ComfyAssetCacheState,
}

impl ComfyAssetUploadRequest {
    pub fn new(
        file_name: impl Into<String>,
        size_bytes: u64,
    ) -> Result<Self, ComfyAssetUploadDiagnostic> {
        let file_name = file_name.into();
        if file_name.trim().is_empty() {
            return Err(invalid_upload_field(
                "file_name",
                "upload file name cannot be empty",
            ));
        }
        if size_bytes == 0 {
            return Err(invalid_upload_field(
                "size_bytes",
                "upload payload cannot be empty",
            ));
        }

        Ok(Self {
            file_name,
            size_bytes,
            mime_type: None,
            known_hash: None,
            tags: Vec::new(),
            user_metadata: BTreeMap::new(),
            preview_id: None,
            cache_state: ComfyAssetCacheState::default(),
        })
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_known_hash(mut self, hash: &str) -> Result<Self, ComfyAssetUploadDiagnostic> {
        self.known_hash = Some(ComfyAssetValidatedHash::parse(hash)?);
        Ok(self)
    }

    pub fn with_tag(mut self, tag: &str) -> Result<Self, ComfyAssetUploadDiagnostic> {
        self.tags.push(normalize_asset_tag(tag)?);
        self.tags.sort();
        self.tags.dedup();
        Ok(self)
    }

    pub fn with_user_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.user_metadata.insert(key.into(), value);
        self
    }

    pub fn with_preview_id(mut self, preview_id: ComfyAssetReferenceId) -> Self {
        self.preview_id = Some(preview_id);
        self
    }

    pub fn with_cache_state(mut self, cache_state: ComfyAssetCacheState) -> Self {
        self.cache_state = cache_state;
        self
    }

    pub fn into_reference_request(self) -> ComfyAssetReferenceRequest {
        let mut request = ComfyAssetReferenceRequest::new(self.file_name, self.size_bytes)
            .with_cache_state(self.cache_state);
        if let Some(hash) = self.known_hash {
            request = request.with_hash(hash.as_str());
        }
        if let Some(mime_type) = self.mime_type {
            request = request.with_mime_type(mime_type);
        }
        if let Some(preview_id) = self.preview_id {
            request = request.with_preview_id(preview_id);
        }
        for tag in self.tags {
            request = request.with_tag(tag);
        }
        for (key, value) in self.user_metadata {
            request = request.with_user_metadata(key, value);
        }
        request
    }
}

fn invalid_upload_field(
    field: impl Into<String>,
    message: impl Into<String>,
) -> ComfyAssetUploadDiagnostic {
    ComfyAssetUploadDiagnostic {
        code: ASSET_UPLOAD_INVALID_FIELD_CODE.to_string(),
        field: field.into(),
        message: message.into(),
    }
}
