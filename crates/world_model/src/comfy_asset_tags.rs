use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ComfyAssetApi, ComfyAssetApiDiagnostic, ComfyAssetListQuery, ComfyAssetOrder,
    ComfyAssetOwnerId, ComfyAssetReferenceId, ComfyAssetUpdateRequest, normalize_asset_tag,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetTagMutationReport {
    pub tag: String,
    pub added: bool,
    pub already_present: bool,
    pub removed: bool,
    pub missing: bool,
    pub total_tags: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetTagCount {
    pub tag: String,
    pub count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetTagListQuery {
    pub owner_id: ComfyAssetOwnerId,
    pub prefix: Option<String>,
    pub limit: usize,
    pub offset: usize,
    pub order: ComfyAssetOrder,
    pub include_zero: bool,
}

impl ComfyAssetTagListQuery {
    pub fn new(owner_id: ComfyAssetOwnerId) -> Self {
        Self {
            owner_id,
            prefix: None,
            limit: 100,
            offset: 0,
            order: ComfyAssetOrder::Ascending,
            include_zero: false,
        }
    }

    pub fn with_prefix(mut self, prefix: &str) -> Result<Self, ComfyAssetApiDiagnostic> {
        self.prefix = Some(normalize_asset_tag(prefix).map_err(query_error)?);
        Ok(self)
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.clamp(1, 500);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_order(mut self, order: ComfyAssetOrder) -> Self {
        self.order = order;
        self
    }

    pub fn include_zero(mut self) -> Self {
        self.include_zero = true;
        self
    }
}

pub struct ComfyAssetTagService<'a> {
    api: &'a mut ComfyAssetApi,
}

impl<'a> ComfyAssetTagService<'a> {
    pub fn new(api: &'a mut ComfyAssetApi) -> Self {
        Self { api }
    }

    pub fn add_tag(
        &mut self,
        owner_id: &ComfyAssetOwnerId,
        reference_id: &ComfyAssetReferenceId,
        tag: &str,
    ) -> Result<Option<ComfyAssetTagMutationReport>, ComfyAssetApiDiagnostic> {
        let tag = normalize_asset_tag(tag).map_err(query_error)?;
        let Some(detail) = self.api.detail(owner_id, reference_id)? else {
            return Ok(None);
        };
        let mut tags = detail.reference.tags.iter().cloned().collect::<Vec<_>>();
        let already_present = tags.iter().any(|existing| existing == &tag);
        if !already_present {
            tags.push(tag.clone());
            tags.sort();
        }
        let updated = self
            .api
            .update(
                owner_id,
                reference_id,
                ComfyAssetUpdateRequest::default().with_tags(tags.clone())?,
            )?
            .ok_or_else(|| tag_update_lost_visibility(reference_id.clone()))?;
        Ok(Some(ComfyAssetTagMutationReport {
            tag,
            added: !already_present,
            already_present,
            removed: false,
            missing: false,
            total_tags: updated.reference.tags.len(),
        }))
    }

    pub fn remove_tag(
        &mut self,
        owner_id: &ComfyAssetOwnerId,
        reference_id: &ComfyAssetReferenceId,
        tag: &str,
    ) -> Result<Option<ComfyAssetTagMutationReport>, ComfyAssetApiDiagnostic> {
        let tag = normalize_asset_tag(tag).map_err(query_error)?;
        let Some(detail) = self.api.detail(owner_id, reference_id)? else {
            return Ok(None);
        };
        let mut tags = detail.reference.tags.iter().cloned().collect::<Vec<_>>();
        let original_len = tags.len();
        tags.retain(|existing| existing != &tag);
        let removed = tags.len() != original_len;
        let updated = self
            .api
            .update(
                owner_id,
                reference_id,
                ComfyAssetUpdateRequest::default().with_tags(tags.clone())?,
            )?
            .ok_or_else(|| tag_update_lost_visibility(reference_id.clone()))?;
        Ok(Some(ComfyAssetTagMutationReport {
            tag,
            added: false,
            already_present: false,
            removed,
            missing: !removed,
            total_tags: updated.reference.tags.len(),
        }))
    }

    pub fn list_tags(
        &self,
        query: &ComfyAssetTagListQuery,
    ) -> Result<Vec<ComfyAssetTagCount>, ComfyAssetApiDiagnostic> {
        let list_query = ComfyAssetListQuery::new(crate::ComfyAssetOwnerScope {
            owner_id: query.owner_id.clone(),
        });
        let page = self.api.list(&list_query)?;
        let mut counts = BTreeMap::<String, usize>::new();
        for item in page.items {
            for tag in item.reference.tags {
                *counts.entry(tag).or_default() += 1;
            }
        }
        let mut tags = counts
            .into_iter()
            .filter(|(tag, count)| {
                (query.include_zero || *count > 0)
                    && query
                        .prefix
                        .as_ref()
                        .is_none_or(|prefix| tag.starts_with(prefix))
            })
            .map(|(tag, count)| ComfyAssetTagCount { tag, count })
            .collect::<Vec<_>>();
        sort_tags(&mut tags, query.order);
        Ok(tags
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect())
    }

    pub fn refine_tags(
        &self,
        query: &ComfyAssetListQuery,
    ) -> Result<Vec<ComfyAssetTagCount>, ComfyAssetApiDiagnostic> {
        let page = self.api.list(query)?;
        let mut counts = BTreeMap::<String, usize>::new();
        for item in page.items {
            for tag in item.reference.tags {
                *counts.entry(tag).or_default() += 1;
            }
        }
        let mut tags = counts
            .into_iter()
            .map(|(tag, count)| ComfyAssetTagCount { tag, count })
            .collect::<Vec<_>>();
        sort_tags(&mut tags, ComfyAssetOrder::Ascending);
        Ok(tags)
    }
}

fn sort_tags(tags: &mut [ComfyAssetTagCount], order: ComfyAssetOrder) {
    tags.sort_by(|left, right| {
        let ordering = left.tag.cmp(&right.tag);
        match order {
            ComfyAssetOrder::Ascending => ordering,
            ComfyAssetOrder::Descending => ordering.reverse(),
        }
    });
}

fn query_error(error: crate::ComfyAssetQueryDiagnostic) -> ComfyAssetApiDiagnostic {
    ComfyAssetApiDiagnostic {
        code: error.code,
        reference_id: None,
        message: error.message,
    }
}

fn tag_update_lost_visibility(reference_id: ComfyAssetReferenceId) -> ComfyAssetApiDiagnostic {
    ComfyAssetApiDiagnostic {
        code: crate::ASSET_API_FORBIDDEN_CODE.to_string(),
        reference_id: Some(reference_id),
        message: "asset reference became inaccessible during tag update".to_string(),
    }
}
