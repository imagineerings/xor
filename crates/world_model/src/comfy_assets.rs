use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const ASSET_CONTENT_NOT_FOUND_CODE: &str = "world_model.comfy_assets.content_not_found";
pub const ASSET_REFERENCE_NOT_FOUND_CODE: &str = "world_model.comfy_assets.reference_not_found";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ComfyAssetContentId(String);

impl ComfyAssetContentId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ComfyAssetReferenceId(String);

impl ComfyAssetReferenceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ComfyAssetOwnerId(String);

impl ComfyAssetOwnerId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ComfyAssetHash(String);

impl ComfyAssetHash {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetContentRecord {
    pub id: ComfyAssetContentId,
    pub hash: Option<ComfyAssetHash>,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub created_at_ms: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetCacheState {
    pub file_path: Option<PathBuf>,
    pub modified_at_ms: Option<u64>,
    pub is_missing: bool,
    pub verified: bool,
    pub enrichment_level: u8,
}

impl ComfyAssetCacheState {
    pub fn with_file_path(mut self, file_path: impl Into<PathBuf>) -> Self {
        self.file_path = Some(file_path.into());
        self
    }

    pub fn with_modified_at_ms(mut self, modified_at_ms: u64) -> Self {
        self.modified_at_ms = Some(modified_at_ms);
        self
    }

    pub fn verified(mut self) -> Self {
        self.verified = true;
        self
    }

    pub fn missing(mut self) -> Self {
        self.is_missing = true;
        self
    }

    pub fn with_enrichment_level(mut self, enrichment_level: u8) -> Self {
        self.enrichment_level = enrichment_level;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetReferenceRecord {
    pub id: ComfyAssetReferenceId,
    pub content_id: ComfyAssetContentId,
    pub owner_id: ComfyAssetOwnerId,
    pub name: String,
    pub tags: BTreeSet<String>,
    pub preview_id: Option<ComfyAssetReferenceId>,
    pub user_metadata: BTreeMap<String, serde_json::Value>,
    pub system_metadata: BTreeMap<String, serde_json::Value>,
    pub job_id: Option<String>,
    pub provenance_id: Option<String>,
    pub cache_state: ComfyAssetCacheState,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub deleted_at_ms: Option<u64>,
}

impl ComfyAssetReferenceRecord {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at_ms.is_some()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetReferenceRequest {
    pub hash: Option<ComfyAssetHash>,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub name: String,
    pub tags: BTreeSet<String>,
    pub preview_id: Option<ComfyAssetReferenceId>,
    pub user_metadata: BTreeMap<String, serde_json::Value>,
    pub system_metadata: BTreeMap<String, serde_json::Value>,
    pub job_id: Option<String>,
    pub provenance_id: Option<String>,
    pub cache_state: ComfyAssetCacheState,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetReferencePatch {
    pub name: Option<String>,
    pub tags: Option<BTreeSet<String>>,
    pub preview_id: Option<Option<ComfyAssetReferenceId>>,
    pub user_metadata: Option<BTreeMap<String, serde_json::Value>>,
    pub cache_state: Option<ComfyAssetCacheState>,
}

impl ComfyAssetReferencePatch {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = Some(tags.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_preview_id(mut self, preview_id: Option<ComfyAssetReferenceId>) -> Self {
        self.preview_id = Some(preview_id);
        self
    }

    pub fn with_user_metadata(mut self, metadata: BTreeMap<String, serde_json::Value>) -> Self {
        self.user_metadata = Some(metadata);
        self
    }

    pub fn with_cache_state(mut self, cache_state: ComfyAssetCacheState) -> Self {
        self.cache_state = Some(cache_state);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.tags.is_none()
            && self.preview_id.is_none()
            && self.user_metadata.is_none()
            && self.cache_state.is_none()
    }
}

impl ComfyAssetReferenceRequest {
    pub fn new(name: impl Into<String>, size_bytes: u64) -> Self {
        Self {
            name: name.into(),
            size_bytes,
            ..Self::default()
        }
    }

    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.hash = Some(ComfyAssetHash::new(hash));
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
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

    pub fn with_preview_id(mut self, preview_id: ComfyAssetReferenceId) -> Self {
        self.preview_id = Some(preview_id);
        self
    }

    pub fn with_cache_state(mut self, cache_state: ComfyAssetCacheState) -> Self {
        self.cache_state = cache_state;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetDiagnostic {
    pub code: String,
    pub reference_id: Option<ComfyAssetReferenceId>,
    pub content_id: Option<ComfyAssetContentId>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetRepository {
    content: BTreeMap<ComfyAssetContentId, ComfyAssetContentRecord>,
    content_by_hash: BTreeMap<ComfyAssetHash, ComfyAssetContentId>,
    references: BTreeMap<ComfyAssetReferenceId, ComfyAssetReferenceRecord>,
    next_content_id: u64,
    next_reference_id: u64,
    clock_ms: u64,
}

impl ComfyAssetRepository {
    pub fn create_reference(
        &mut self,
        owner_id: ComfyAssetOwnerId,
        request: ComfyAssetReferenceRequest,
    ) -> ComfyAssetReferenceRecord {
        let content =
            self.create_or_reuse_content(request.hash, request.size_bytes, request.mime_type);
        self.next_reference_id = self.next_reference_id.saturating_add(1);
        let now = self.next_timestamp();
        let reference = ComfyAssetReferenceRecord {
            id: ComfyAssetReferenceId::new(format!("asset-reference-{}", self.next_reference_id)),
            content_id: content.id,
            owner_id,
            name: request.name,
            tags: request.tags,
            preview_id: request.preview_id,
            user_metadata: request.user_metadata,
            system_metadata: request.system_metadata,
            job_id: request.job_id,
            provenance_id: request.provenance_id,
            cache_state: request.cache_state,
            created_at_ms: now,
            updated_at_ms: now,
            deleted_at_ms: None,
        };
        self.references
            .insert(reference.id.clone(), reference.clone());
        reference
    }

    pub fn content_by_hash(&self, hash: &ComfyAssetHash) -> Option<&ComfyAssetContentRecord> {
        self.content_by_hash
            .get(hash)
            .and_then(|content_id| self.content.get(content_id))
    }

    pub fn content(
        &self,
        content_id: &ComfyAssetContentId,
    ) -> Result<&ComfyAssetContentRecord, ComfyAssetDiagnostic> {
        self.content
            .get(content_id)
            .ok_or_else(|| ComfyAssetDiagnostic {
                code: ASSET_CONTENT_NOT_FOUND_CODE.to_string(),
                reference_id: None,
                content_id: Some(content_id.clone()),
                message: format!("asset content `{}` was not found", content_id.as_str()),
            })
    }

    pub fn reference(
        &self,
        reference_id: &ComfyAssetReferenceId,
    ) -> Result<&ComfyAssetReferenceRecord, ComfyAssetDiagnostic> {
        self.references
            .get(reference_id)
            .ok_or_else(|| ComfyAssetDiagnostic {
                code: ASSET_REFERENCE_NOT_FOUND_CODE.to_string(),
                reference_id: Some(reference_id.clone()),
                content_id: None,
                message: format!("asset reference `{}` was not found", reference_id.as_str()),
            })
    }

    pub fn references_for_owner(
        &self,
        owner_id: &ComfyAssetOwnerId,
    ) -> Vec<&ComfyAssetReferenceRecord> {
        self.references
            .values()
            .filter(|reference| &reference.owner_id == owner_id && !reference.is_deleted())
            .collect()
    }

    pub fn update_reference(
        &mut self,
        owner_id: &ComfyAssetOwnerId,
        reference_id: &ComfyAssetReferenceId,
        patch: ComfyAssetReferencePatch,
    ) -> Result<Option<ComfyAssetReferenceRecord>, ComfyAssetDiagnostic> {
        let now = self.next_timestamp();
        let Some(reference) = self.references.get_mut(reference_id) else {
            return Err(ComfyAssetDiagnostic {
                code: ASSET_REFERENCE_NOT_FOUND_CODE.to_string(),
                reference_id: Some(reference_id.clone()),
                content_id: None,
                message: format!("asset reference `{}` was not found", reference_id.as_str()),
            });
        };
        if &reference.owner_id != owner_id || reference.deleted_at_ms.is_some() {
            return Ok(None);
        }
        if let Some(name) = patch.name {
            reference.name = name;
        }
        if let Some(tags) = patch.tags {
            reference.tags = tags;
        }
        if let Some(preview_id) = patch.preview_id {
            reference.preview_id = preview_id;
        }
        if let Some(user_metadata) = patch.user_metadata {
            reference.user_metadata = user_metadata;
        }
        if let Some(cache_state) = patch.cache_state {
            reference.cache_state = cache_state;
        }
        reference.updated_at_ms = now;
        Ok(Some(reference.clone()))
    }

    pub fn soft_delete_reference(
        &mut self,
        owner_id: &ComfyAssetOwnerId,
        reference_id: &ComfyAssetReferenceId,
    ) -> Result<bool, ComfyAssetDiagnostic> {
        let now = self.next_timestamp();
        let Some(reference) = self.references.get_mut(reference_id) else {
            return Err(ComfyAssetDiagnostic {
                code: ASSET_REFERENCE_NOT_FOUND_CODE.to_string(),
                reference_id: Some(reference_id.clone()),
                content_id: None,
                message: format!("asset reference `{}` was not found", reference_id.as_str()),
            });
        };
        if &reference.owner_id != owner_id || reference.deleted_at_ms.is_some() {
            return Ok(false);
        }
        reference.deleted_at_ms = Some(now);
        reference.updated_at_ms = now;
        Ok(true)
    }

    pub fn content_len(&self) -> usize {
        self.content.len()
    }

    pub fn reference_len(&self) -> usize {
        self.references.len()
    }

    fn create_or_reuse_content(
        &mut self,
        hash: Option<ComfyAssetHash>,
        size_bytes: u64,
        mime_type: Option<String>,
    ) -> ComfyAssetContentRecord {
        if let Some(hash) = &hash {
            if let Some(content_id) = self.content_by_hash.get(hash) {
                if let Some(content) = self.content.get(content_id) {
                    return content.clone();
                }
            }
        }

        self.next_content_id = self.next_content_id.saturating_add(1);
        let content = ComfyAssetContentRecord {
            id: ComfyAssetContentId::new(format!("asset-content-{}", self.next_content_id)),
            hash: hash.clone(),
            size_bytes,
            mime_type,
            created_at_ms: self.next_timestamp(),
        };
        if let Some(hash) = hash {
            self.content_by_hash.insert(hash, content.id.clone());
        }
        self.content.insert(content.id.clone(), content.clone());
        content
    }

    fn next_timestamp(&mut self) -> u64 {
        self.clock_ms = self.clock_ms.saturating_add(1);
        self.clock_ms
    }
}
