use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    SimAssetCacheState, SimAssetQueryDiagnostic, SimAssetReferenceId, SimAssetReferenceRequest,
    SimAssetValidatedHash, normalize_asset_tag,
};

pub const ASSET_UPLOAD_INVALID_FIELD_CODE: &str = "world_model.sim_assets.invalid_upload_field";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAssetUploadDiagnostic {
    pub code: String,
    pub field: String,
    pub message: String,
}

impl From<SimAssetQueryDiagnostic> for SimAssetUploadDiagnostic {
    fn from(error: SimAssetQueryDiagnostic) -> Self {
        Self {
            code: error.code,
            field: error.field,
            message: error.message,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimAssetUploadRequest {
    pub file_name: String,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub known_hash: Option<SimAssetValidatedHash>,
    pub tags: Vec<String>,
    pub user_metadata: BTreeMap<String, serde_json::Value>,
    pub system_metadata: BTreeMap<String, serde_json::Value>,
    pub job_id: Option<String>,
    pub provenance_id: Option<String>,
    pub preview_id: Option<SimAssetReferenceId>,
    pub cache_state: SimAssetCacheState,
}

impl SimAssetUploadRequest {
    pub fn new(
        file_name: impl Into<String>,
        size_bytes: u64,
    ) -> Result<Self, SimAssetUploadDiagnostic> {
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
            system_metadata: BTreeMap::new(),
            job_id: None,
            provenance_id: None,
            preview_id: None,
            cache_state: SimAssetCacheState::default(),
        })
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_known_hash(mut self, hash: &str) -> Result<Self, SimAssetUploadDiagnostic> {
        self.known_hash = Some(SimAssetValidatedHash::parse(hash)?);
        Ok(self)
    }

    pub fn with_tag(mut self, tag: &str) -> Result<Self, SimAssetUploadDiagnostic> {
        self.tags.push(normalize_asset_tag(tag)?);
        self.tags.sort();
        self.tags.dedup();
        Ok(self)
    }

    pub fn with_user_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.user_metadata.insert(key.into(), value);
        self
    }

    pub fn with_system_metadata(
        mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        self.system_metadata.insert(key.into(), value);
        self
    }

    pub fn with_job_id(mut self, job_id: impl Into<String>) -> Self {
        self.job_id = Some(job_id.into());
        self
    }

    pub fn with_provenance_id(mut self, provenance_id: impl Into<String>) -> Self {
        self.provenance_id = Some(provenance_id.into());
        self
    }

    pub fn with_preview_id(mut self, preview_id: SimAssetReferenceId) -> Self {
        self.preview_id = Some(preview_id);
        self
    }

    pub fn with_cache_state(mut self, cache_state: SimAssetCacheState) -> Self {
        self.cache_state = cache_state;
        self
    }

    pub fn into_reference_request(self) -> SimAssetReferenceRequest {
        let mut request = SimAssetReferenceRequest::new(self.file_name, self.size_bytes)
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
        for (key, value) in self.system_metadata {
            request = request.with_system_metadata(key, value);
        }
        if let Some(job_id) = self.job_id {
            request = request.with_job_id(job_id);
        }
        if let Some(provenance_id) = self.provenance_id {
            request = request.with_provenance_id(provenance_id);
        }
        request
    }
}

fn invalid_upload_field(
    field: impl Into<String>,
    message: impl Into<String>,
) -> SimAssetUploadDiagnostic {
    SimAssetUploadDiagnostic {
        code: ASSET_UPLOAD_INVALID_FIELD_CODE.to_string(),
        field: field.into(),
        message: message.into(),
    }
}
