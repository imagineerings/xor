use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ASSET_CONTENT_NOT_FOUND_CODE, ASSET_REFERENCE_NOT_FOUND_CODE, ComfyAssetCacheState,
    ComfyAssetContentRecord, ComfyAssetDiagnostic, ComfyAssetHash, ComfyAssetListQuery,
    ComfyAssetMetadataFilter, ComfyAssetMetadataNamespace, ComfyAssetOrder, ComfyAssetOwnerId,
    ComfyAssetReferenceId, ComfyAssetReferencePatch, ComfyAssetReferenceRecord,
    ComfyAssetReferenceRequest, ComfyAssetRepository, ComfyAssetSort, ComfyAssetUploadDiagnostic,
    ComfyAssetUploadRequest, ComfyAssetValidatedHash, normalize_asset_tag,
};

pub const ASSET_API_FORBIDDEN_CODE: &str = "world_model.comfy_assets.forbidden";
pub const ASSET_API_HASH_NOT_FOUND_CODE: &str = "world_model.comfy_assets.hash_not_found";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetApiDiagnostic {
    pub code: String,
    pub reference_id: Option<ComfyAssetReferenceId>,
    pub message: String,
}

impl From<ComfyAssetDiagnostic> for ComfyAssetApiDiagnostic {
    fn from(error: ComfyAssetDiagnostic) -> Self {
        Self {
            code: error.code,
            reference_id: error.reference_id,
            message: error.message,
        }
    }
}

impl From<ComfyAssetUploadDiagnostic> for ComfyAssetApiDiagnostic {
    fn from(error: ComfyAssetUploadDiagnostic) -> Self {
        Self {
            code: error.code,
            reference_id: None,
            message: error.message,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetReferenceDetail {
    pub reference: ComfyAssetReferenceRecord,
    pub content: ComfyAssetContentRecord,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetListPage {
    pub items: Vec<ComfyAssetReferenceDetail>,
    pub total: usize,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetUpdateRequest {
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub preview_id: Option<Option<ComfyAssetReferenceId>>,
    pub user_metadata: Option<BTreeMap<String, Value>>,
    pub cache_state: Option<ComfyAssetCacheState>,
}

impl ComfyAssetUpdateRequest {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_tags(
        mut self,
        tags: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, ComfyAssetApiDiagnostic> {
        let mut normalized = Vec::new();
        for tag in tags {
            normalized.push(normalize_asset_tag(tag.as_ref()).map_err(|error| {
                ComfyAssetApiDiagnostic {
                    code: error.code,
                    reference_id: None,
                    message: error.message,
                }
            })?);
        }
        normalized.sort();
        normalized.dedup();
        self.tags = Some(normalized);
        Ok(self)
    }

    pub fn with_preview_id(mut self, preview_id: Option<ComfyAssetReferenceId>) -> Self {
        self.preview_id = Some(preview_id);
        self
    }

    pub fn with_user_metadata(mut self, user_metadata: BTreeMap<String, Value>) -> Self {
        self.user_metadata = Some(user_metadata);
        self
    }

    pub fn with_cache_state(mut self, cache_state: ComfyAssetCacheState) -> Self {
        self.cache_state = Some(cache_state);
        self
    }

    fn into_patch(self) -> ComfyAssetReferencePatch {
        let tags = self
            .tags
            .map(|tags| tags.into_iter().collect::<BTreeSet<_>>());
        ComfyAssetReferencePatch {
            name: self.name,
            tags,
            preview_id: self.preview_id,
            user_metadata: self.user_metadata,
            cache_state: self.cache_state,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetApi {
    repository: ComfyAssetRepository,
}

impl ComfyAssetApi {
    pub fn new(repository: ComfyAssetRepository) -> Self {
        Self { repository }
    }

    pub fn repository(&self) -> &ComfyAssetRepository {
        &self.repository
    }

    pub fn list(
        &self,
        query: &ComfyAssetListQuery,
    ) -> Result<ComfyAssetListPage, ComfyAssetApiDiagnostic> {
        let mut details = Vec::new();
        for reference in self
            .repository
            .references_for_owner(&query.owner_scope.owner_id)
        {
            if !self.matches_query(reference, query)? {
                continue;
            }
            details.push(self.detail_for_reference(reference)?);
        }

        sort_details(&mut details, query.sort, query.order);
        let total = details.len();
        let start = page_start(&details, query);
        let end = start.saturating_add(query.pagination.limit).min(total);
        let items = details[start..end].to_vec();
        let next_cursor = if end < total {
            items
                .last()
                .map(|detail| cursor_for_detail(detail, query.sort))
        } else {
            None
        };

        Ok(ComfyAssetListPage {
            items,
            total,
            next_cursor,
        })
    }

    pub fn detail(
        &self,
        owner_id: &ComfyAssetOwnerId,
        reference_id: &ComfyAssetReferenceId,
    ) -> Result<Option<ComfyAssetReferenceDetail>, ComfyAssetApiDiagnostic> {
        let reference = self.repository.reference(reference_id)?;
        if &reference.owner_id != owner_id || reference.is_deleted() {
            return Ok(None);
        }
        Ok(Some(self.detail_for_reference(reference)?))
    }

    pub fn hash_exists(&self, hash: &ComfyAssetValidatedHash) -> bool {
        self.repository
            .content_by_hash(&ComfyAssetHash::new(hash.as_str()))
            .is_some()
    }

    pub fn create_from_hash(
        &mut self,
        owner_id: ComfyAssetOwnerId,
        hash: &ComfyAssetValidatedHash,
        request: ComfyAssetReferenceRequest,
    ) -> Result<ComfyAssetReferenceDetail, ComfyAssetApiDiagnostic> {
        if !self.hash_exists(hash) {
            return Err(ComfyAssetApiDiagnostic {
                code: ASSET_API_HASH_NOT_FOUND_CODE.to_string(),
                reference_id: None,
                message: format!("asset content hash `{}` was not found", hash.as_str()),
            });
        }
        let reference = self
            .repository
            .create_reference(owner_id, request.with_hash(hash.as_str()));
        self.detail_for_reference(&reference)
    }

    pub fn upload(
        &mut self,
        owner_id: ComfyAssetOwnerId,
        request: ComfyAssetUploadRequest,
    ) -> Result<ComfyAssetReferenceDetail, ComfyAssetApiDiagnostic> {
        let reference = self
            .repository
            .create_reference(owner_id, request.into_reference_request());
        self.detail_for_reference(&reference)
    }

    pub fn update(
        &mut self,
        owner_id: &ComfyAssetOwnerId,
        reference_id: &ComfyAssetReferenceId,
        request: ComfyAssetUpdateRequest,
    ) -> Result<Option<ComfyAssetReferenceDetail>, ComfyAssetApiDiagnostic> {
        let Some(reference) =
            self.repository
                .update_reference(owner_id, reference_id, request.into_patch())?
        else {
            return Ok(None);
        };
        self.detail_for_reference(&reference).map(Some)
    }

    pub fn delete(
        &mut self,
        owner_id: &ComfyAssetOwnerId,
        reference_id: &ComfyAssetReferenceId,
    ) -> Result<bool, ComfyAssetApiDiagnostic> {
        self.repository
            .soft_delete_reference(owner_id, reference_id)
            .map_err(Into::into)
    }

    pub fn update_cache_state(
        &mut self,
        owner_id: &ComfyAssetOwnerId,
        reference_id: &ComfyAssetReferenceId,
        cache_state: ComfyAssetCacheState,
    ) -> Result<Option<ComfyAssetReferenceDetail>, ComfyAssetApiDiagnostic> {
        self.update(
            owner_id,
            reference_id,
            ComfyAssetUpdateRequest::default().with_cache_state(cache_state),
        )
    }

    fn detail_for_reference(
        &self,
        reference: &ComfyAssetReferenceRecord,
    ) -> Result<ComfyAssetReferenceDetail, ComfyAssetApiDiagnostic> {
        let content = self.repository.content(&reference.content_id)?.clone();
        Ok(ComfyAssetReferenceDetail {
            reference: reference.clone(),
            content,
        })
    }

    fn matches_query(
        &self,
        reference: &ComfyAssetReferenceRecord,
        query: &ComfyAssetListQuery,
    ) -> Result<bool, ComfyAssetApiDiagnostic> {
        if let Some(name_contains) = &query.name_contains {
            if !reference
                .name
                .to_ascii_lowercase()
                .contains(&name_contains.to_ascii_lowercase())
            {
                return Ok(false);
            }
        }
        if !query
            .include_tags
            .iter()
            .all(|tag| reference.tags.contains(tag))
        {
            return Ok(false);
        }
        if query
            .exclude_tags
            .iter()
            .any(|tag| reference.tags.contains(tag))
        {
            return Ok(false);
        }
        if !query
            .metadata_filters
            .iter()
            .all(|filter| metadata_matches(reference, filter))
        {
            return Ok(false);
        }
        if let Some(hash) = &query.hash {
            let content = self.repository.content(&reference.content_id)?;
            if content.hash.as_ref().map(ComfyAssetHash::as_str) != Some(hash.as_str()) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn metadata_matches(
    reference: &ComfyAssetReferenceRecord,
    filter: &ComfyAssetMetadataFilter,
) -> bool {
    let metadata = match filter.namespace {
        ComfyAssetMetadataNamespace::User => &reference.user_metadata,
        ComfyAssetMetadataNamespace::System => &reference.system_metadata,
    };
    metadata.get(&filter.key) == Some(&filter.value)
}

fn sort_details(
    details: &mut [ComfyAssetReferenceDetail],
    sort: ComfyAssetSort,
    order: ComfyAssetOrder,
) {
    details.sort_by(|left, right| {
        let ordering = compare_detail(left, right, sort);
        match order {
            ComfyAssetOrder::Ascending => ordering,
            ComfyAssetOrder::Descending => ordering.reverse(),
        }
        .then_with(|| left.reference.id.cmp(&right.reference.id))
    });
}

fn compare_detail(
    left: &ComfyAssetReferenceDetail,
    right: &ComfyAssetReferenceDetail,
    sort: ComfyAssetSort,
) -> Ordering {
    match sort {
        ComfyAssetSort::CreatedAt => left
            .reference
            .created_at_ms
            .cmp(&right.reference.created_at_ms),
        ComfyAssetSort::UpdatedAt => left
            .reference
            .updated_at_ms
            .cmp(&right.reference.updated_at_ms),
        ComfyAssetSort::Name => left
            .reference
            .name
            .to_ascii_lowercase()
            .cmp(&right.reference.name.to_ascii_lowercase()),
        ComfyAssetSort::SizeBytes => left.content.size_bytes.cmp(&right.content.size_bytes),
        ComfyAssetSort::Hash => left.content.hash.cmp(&right.content.hash),
    }
}

fn page_start(details: &[ComfyAssetReferenceDetail], query: &ComfyAssetListQuery) -> usize {
    let cursor_start = query
        .pagination
        .cursor
        .as_ref()
        .and_then(|cursor| {
            details
                .iter()
                .position(|detail| detail.reference.id == cursor.reference_id)
                .map(|position| position.saturating_add(1))
        })
        .unwrap_or_default();
    cursor_start
        .saturating_add(query.pagination.offset)
        .min(details.len())
}

fn cursor_for_detail(detail: &ComfyAssetReferenceDetail, sort: ComfyAssetSort) -> String {
    crate::ComfyAssetCursor::new(
        sort_value_for_detail(detail, sort),
        detail.reference.id.clone(),
    )
    .encode()
}

fn sort_value_for_detail(detail: &ComfyAssetReferenceDetail, sort: ComfyAssetSort) -> String {
    match sort {
        ComfyAssetSort::CreatedAt => format!("{:020}", detail.reference.created_at_ms),
        ComfyAssetSort::UpdatedAt => format!("{:020}", detail.reference.updated_at_ms),
        ComfyAssetSort::Name => detail.reference.name.to_ascii_lowercase(),
        ComfyAssetSort::SizeBytes => format!("{:020}", detail.content.size_bytes),
        ComfyAssetSort::Hash => detail
            .content
            .hash
            .as_ref()
            .map(ComfyAssetHash::as_str)
            .unwrap_or_default()
            .to_string(),
    }
}

pub fn missing_content_api_error() -> ComfyAssetApiDiagnostic {
    ComfyAssetApiDiagnostic {
        code: ASSET_CONTENT_NOT_FOUND_CODE.to_string(),
        reference_id: None,
        message: "asset content was not found".to_string(),
    }
}

pub fn missing_reference_api_error(reference_id: ComfyAssetReferenceId) -> ComfyAssetApiDiagnostic {
    ComfyAssetApiDiagnostic {
        code: ASSET_REFERENCE_NOT_FOUND_CODE.to_string(),
        reference_id: Some(reference_id),
        message: "asset reference was not found".to_string(),
    }
}
