use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ASSET_CONTENT_NOT_FOUND_CODE, ASSET_REFERENCE_NOT_FOUND_CODE, SimAssetCacheState,
    SimAssetContentRecord, SimAssetDiagnostic, SimAssetHash, SimAssetListQuery,
    SimAssetMetadataFilter, SimAssetMetadataNamespace, SimAssetOrder, SimAssetOwnerId,
    SimAssetReferenceId, SimAssetReferencePatch, SimAssetReferenceRecord, SimAssetReferenceRequest,
    SimAssetRepository, SimAssetSort, SimAssetUploadDiagnostic, SimAssetUploadRequest,
    SimAssetValidatedHash, normalize_asset_tag,
};

pub const ASSET_API_FORBIDDEN_CODE: &str = "world_model.sim_assets.forbidden";
pub const ASSET_API_HASH_NOT_FOUND_CODE: &str = "world_model.sim_assets.hash_not_found";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAssetApiDiagnostic {
    pub code: String,
    pub reference_id: Option<SimAssetReferenceId>,
    pub message: String,
}

impl From<SimAssetDiagnostic> for SimAssetApiDiagnostic {
    fn from(error: SimAssetDiagnostic) -> Self {
        Self {
            code: error.code,
            reference_id: error.reference_id,
            message: error.message,
        }
    }
}

impl From<SimAssetUploadDiagnostic> for SimAssetApiDiagnostic {
    fn from(error: SimAssetUploadDiagnostic) -> Self {
        Self {
            code: error.code,
            reference_id: None,
            message: error.message,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimAssetReferenceDetail {
    pub reference: SimAssetReferenceRecord,
    pub content: SimAssetContentRecord,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimAssetListPage {
    pub items: Vec<SimAssetReferenceDetail>,
    pub total: usize,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimAssetUpdateRequest {
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub preview_id: Option<Option<SimAssetReferenceId>>,
    pub user_metadata: Option<BTreeMap<String, Value>>,
    pub system_metadata: Option<BTreeMap<String, Value>>,
    pub cache_state: Option<SimAssetCacheState>,
}

impl SimAssetUpdateRequest {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_tags(
        mut self,
        tags: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, SimAssetApiDiagnostic> {
        let mut normalized = Vec::new();
        for tag in tags {
            normalized.push(normalize_asset_tag(tag.as_ref()).map_err(|error| {
                SimAssetApiDiagnostic {
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

    pub fn with_preview_id(mut self, preview_id: Option<SimAssetReferenceId>) -> Self {
        self.preview_id = Some(preview_id);
        self
    }

    pub fn with_user_metadata(mut self, user_metadata: BTreeMap<String, Value>) -> Self {
        self.user_metadata = Some(user_metadata);
        self
    }

    pub fn with_system_metadata(mut self, system_metadata: BTreeMap<String, Value>) -> Self {
        self.system_metadata = Some(system_metadata);
        self
    }

    pub fn with_cache_state(mut self, cache_state: SimAssetCacheState) -> Self {
        self.cache_state = Some(cache_state);
        self
    }

    fn into_patch(self) -> SimAssetReferencePatch {
        let tags = self
            .tags
            .map(|tags| tags.into_iter().collect::<BTreeSet<_>>());
        SimAssetReferencePatch {
            name: self.name,
            tags,
            preview_id: self.preview_id,
            user_metadata: self.user_metadata,
            system_metadata: self.system_metadata,
            cache_state: self.cache_state,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimAssetApi {
    repository: SimAssetRepository,
}

impl SimAssetApi {
    pub fn new(repository: SimAssetRepository) -> Self {
        Self { repository }
    }

    pub fn repository(&self) -> &SimAssetRepository {
        &self.repository
    }

    pub fn list(
        &self,
        query: &SimAssetListQuery,
    ) -> Result<SimAssetListPage, SimAssetApiDiagnostic> {
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

        Ok(SimAssetListPage {
            items,
            total,
            next_cursor,
        })
    }

    pub fn detail(
        &self,
        owner_id: &SimAssetOwnerId,
        reference_id: &SimAssetReferenceId,
    ) -> Result<Option<SimAssetReferenceDetail>, SimAssetApiDiagnostic> {
        let reference = self.repository.reference(reference_id)?;
        if &reference.owner_id != owner_id || reference.is_deleted() {
            return Ok(None);
        }
        Ok(Some(self.detail_for_reference(reference)?))
    }

    pub fn hash_exists(&self, hash: &SimAssetValidatedHash) -> bool {
        self.repository
            .content_by_hash(&SimAssetHash::new(hash.as_str()))
            .is_some()
    }

    pub fn create_from_hash(
        &mut self,
        owner_id: SimAssetOwnerId,
        hash: &SimAssetValidatedHash,
        request: SimAssetReferenceRequest,
    ) -> Result<SimAssetReferenceDetail, SimAssetApiDiagnostic> {
        if !self.hash_exists(hash) {
            return Err(SimAssetApiDiagnostic {
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
        owner_id: SimAssetOwnerId,
        request: SimAssetUploadRequest,
    ) -> Result<SimAssetReferenceDetail, SimAssetApiDiagnostic> {
        let reference = self
            .repository
            .create_reference(owner_id, request.into_reference_request());
        self.detail_for_reference(&reference)
    }

    pub fn update(
        &mut self,
        owner_id: &SimAssetOwnerId,
        reference_id: &SimAssetReferenceId,
        request: SimAssetUpdateRequest,
    ) -> Result<Option<SimAssetReferenceDetail>, SimAssetApiDiagnostic> {
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
        owner_id: &SimAssetOwnerId,
        reference_id: &SimAssetReferenceId,
    ) -> Result<bool, SimAssetApiDiagnostic> {
        self.repository
            .soft_delete_reference(owner_id, reference_id)
            .map_err(Into::into)
    }

    pub fn update_cache_state(
        &mut self,
        owner_id: &SimAssetOwnerId,
        reference_id: &SimAssetReferenceId,
        cache_state: SimAssetCacheState,
    ) -> Result<Option<SimAssetReferenceDetail>, SimAssetApiDiagnostic> {
        self.update(
            owner_id,
            reference_id,
            SimAssetUpdateRequest::default().with_cache_state(cache_state),
        )
    }

    fn detail_for_reference(
        &self,
        reference: &SimAssetReferenceRecord,
    ) -> Result<SimAssetReferenceDetail, SimAssetApiDiagnostic> {
        let content = self.repository.content(&reference.content_id)?.clone();
        Ok(SimAssetReferenceDetail {
            reference: reference.clone(),
            content,
        })
    }

    fn matches_query(
        &self,
        reference: &SimAssetReferenceRecord,
        query: &SimAssetListQuery,
    ) -> Result<bool, SimAssetApiDiagnostic> {
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
            if content.hash.as_ref().map(SimAssetHash::as_str) != Some(hash.as_str()) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn metadata_matches(reference: &SimAssetReferenceRecord, filter: &SimAssetMetadataFilter) -> bool {
    let metadata = match filter.namespace {
        SimAssetMetadataNamespace::User => &reference.user_metadata,
        SimAssetMetadataNamespace::System => &reference.system_metadata,
    };
    metadata.get(&filter.key) == Some(&filter.value)
}

fn sort_details(details: &mut [SimAssetReferenceDetail], sort: SimAssetSort, order: SimAssetOrder) {
    details.sort_by(|left, right| {
        let ordering = compare_detail(left, right, sort);
        match order {
            SimAssetOrder::Ascending => ordering,
            SimAssetOrder::Descending => ordering.reverse(),
        }
        .then_with(|| left.reference.id.cmp(&right.reference.id))
    });
}

fn compare_detail(
    left: &SimAssetReferenceDetail,
    right: &SimAssetReferenceDetail,
    sort: SimAssetSort,
) -> Ordering {
    match sort {
        SimAssetSort::CreatedAt => left
            .reference
            .created_at_ms
            .cmp(&right.reference.created_at_ms),
        SimAssetSort::UpdatedAt => left
            .reference
            .updated_at_ms
            .cmp(&right.reference.updated_at_ms),
        SimAssetSort::Name => left
            .reference
            .name
            .to_ascii_lowercase()
            .cmp(&right.reference.name.to_ascii_lowercase()),
        SimAssetSort::SizeBytes => left.content.size_bytes.cmp(&right.content.size_bytes),
        SimAssetSort::Hash => left.content.hash.cmp(&right.content.hash),
    }
}

fn page_start(details: &[SimAssetReferenceDetail], query: &SimAssetListQuery) -> usize {
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

fn cursor_for_detail(detail: &SimAssetReferenceDetail, sort: SimAssetSort) -> String {
    crate::SimAssetCursor::new(
        sort_value_for_detail(detail, sort),
        detail.reference.id.clone(),
    )
    .encode()
}

fn sort_value_for_detail(detail: &SimAssetReferenceDetail, sort: SimAssetSort) -> String {
    match sort {
        SimAssetSort::CreatedAt => format!("{:020}", detail.reference.created_at_ms),
        SimAssetSort::UpdatedAt => format!("{:020}", detail.reference.updated_at_ms),
        SimAssetSort::Name => detail.reference.name.to_ascii_lowercase(),
        SimAssetSort::SizeBytes => format!("{:020}", detail.content.size_bytes),
        SimAssetSort::Hash => detail
            .content
            .hash
            .as_ref()
            .map(SimAssetHash::as_str)
            .unwrap_or_default()
            .to_string(),
    }
}

pub fn missing_content_api_error() -> SimAssetApiDiagnostic {
    SimAssetApiDiagnostic {
        code: ASSET_CONTENT_NOT_FOUND_CODE.to_string(),
        reference_id: None,
        message: "asset content was not found".to_string(),
    }
}

pub fn missing_reference_api_error(reference_id: SimAssetReferenceId) -> SimAssetApiDiagnostic {
    SimAssetApiDiagnostic {
        code: ASSET_REFERENCE_NOT_FOUND_CODE.to_string(),
        reference_id: Some(reference_id),
        message: "asset reference was not found".to_string(),
    }
}
