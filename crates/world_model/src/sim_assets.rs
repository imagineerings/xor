use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const ASSET_CONTENT_NOT_FOUND_CODE: &str = "world_model.sim_assets.content_not_found";
pub const ASSET_REFERENCE_NOT_FOUND_CODE: &str = "world_model.sim_assets.reference_not_found";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAssetCoverageCatalog {
    pub schema_version: u32,
    pub source_root: String,
    pub source_category: String,
    pub captured_at: String,
    pub implementation_owner: String,
    pub native_sim_records: bool,
    pub comfyui_passthrough: bool,
    pub records: Vec<SimAssetCoverageRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAssetCoverageRecord {
    pub source_id: String,
    pub source_path: String,
    pub source_kind: String,
    pub node_name: String,
    pub native_surface: String,
    pub evidence_module: String,
    pub evidence_kind: String,
    pub metadata_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAssetCoverageDiagnostic {
    pub code: String,
    pub message: String,
}

impl SimAssetCoverageCatalog {
    pub fn validate(&self) -> Result<(), Vec<SimAssetCoverageDiagnostic>> {
        let mut diagnostics = Vec::new();

        if self.schema_version != 1 {
            diagnostics.push(sim_asset_coverage_diagnostic(
                "world_model.sim_assets.coverage.invalid_schema",
                "asset coverage fixture must use schema version 1",
            ));
        }
        if self.source_root != "projects/comfy" {
            diagnostics.push(sim_asset_coverage_diagnostic(
                "world_model.sim_assets.coverage.invalid_source_root",
                "asset coverage fixture must preserve projects/comfy source attribution",
            ));
        }
        if !self.native_sim_records || self.comfyui_passthrough {
            diagnostics.push(sim_asset_coverage_diagnostic(
                "world_model.sim_assets.coverage.not_native",
                "asset coverage fixture must describe native Sim records only",
            ));
        }
        if self.records.is_empty() {
            diagnostics.push(sim_asset_coverage_diagnostic(
                "world_model.sim_assets.coverage.empty",
                "asset coverage fixture must include at least one source record",
            ));
        }

        let mut source_ids = BTreeSet::new();
        for record in &self.records {
            if !source_ids.insert(&record.source_id) {
                diagnostics.push(sim_asset_coverage_diagnostic(
                    "world_model.sim_assets.coverage.duplicate_record",
                    format!("duplicate asset coverage source id `{}`", record.source_id),
                ));
            }
            if !record.source_path.starts_with("projects/comfy") {
                diagnostics.push(sim_asset_coverage_diagnostic(
                    "world_model.sim_assets.coverage.invalid_source_path",
                    format!(
                        "source path `{}` does not preserve projects/comfy attribution",
                        record.source_path
                    ),
                ));
            }
            if record.node_name.is_empty()
                || record.native_surface.is_empty()
                || record.evidence_module.is_empty()
                || record.evidence_kind.is_empty()
            {
                diagnostics.push(sim_asset_coverage_diagnostic(
                    "world_model.sim_assets.coverage.missing_evidence",
                    format!("record `{}` is missing asset evidence", record.source_id),
                ));
            }
            if !record.metadata_only {
                diagnostics.push(sim_asset_coverage_diagnostic(
                    "world_model.sim_assets.coverage.not_metadata_only",
                    format!(
                        "record `{}` must stay metadata-only because it represents a fixture node",
                        record.source_id
                    ),
                ));
            }
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }

    pub fn surfaces(&self) -> BTreeSet<String> {
        self.records
            .iter()
            .map(|record| record.native_surface.clone())
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SimAssetContentId(String);

impl SimAssetContentId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SimAssetReferenceId(String);

impl SimAssetReferenceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SimAssetOwnerId(String);

impl SimAssetOwnerId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SimAssetHash(String);

impl SimAssetHash {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimAssetContentRecord {
    pub id: SimAssetContentId,
    pub hash: Option<SimAssetHash>,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub created_at_ms: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAssetCacheState {
    pub file_path: Option<PathBuf>,
    pub modified_at_ms: Option<u64>,
    pub is_missing: bool,
    pub verified: bool,
    pub enrichment_level: u8,
}

impl SimAssetCacheState {
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
pub struct SimAssetReferenceRecord {
    pub id: SimAssetReferenceId,
    pub content_id: SimAssetContentId,
    pub owner_id: SimAssetOwnerId,
    pub name: String,
    pub tags: BTreeSet<String>,
    pub preview_id: Option<SimAssetReferenceId>,
    pub user_metadata: BTreeMap<String, serde_json::Value>,
    pub system_metadata: BTreeMap<String, serde_json::Value>,
    pub job_id: Option<String>,
    pub provenance_id: Option<String>,
    pub cache_state: SimAssetCacheState,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub deleted_at_ms: Option<u64>,
}

impl SimAssetReferenceRecord {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at_ms.is_some()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimAssetReferenceRequest {
    pub hash: Option<SimAssetHash>,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub name: String,
    pub tags: BTreeSet<String>,
    pub preview_id: Option<SimAssetReferenceId>,
    pub user_metadata: BTreeMap<String, serde_json::Value>,
    pub system_metadata: BTreeMap<String, serde_json::Value>,
    pub job_id: Option<String>,
    pub provenance_id: Option<String>,
    pub cache_state: SimAssetCacheState,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimAssetReferencePatch {
    pub name: Option<String>,
    pub tags: Option<BTreeSet<String>>,
    pub preview_id: Option<Option<SimAssetReferenceId>>,
    pub user_metadata: Option<BTreeMap<String, serde_json::Value>>,
    pub system_metadata: Option<BTreeMap<String, serde_json::Value>>,
    pub cache_state: Option<SimAssetCacheState>,
}

impl SimAssetReferencePatch {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = Some(tags.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_preview_id(mut self, preview_id: Option<SimAssetReferenceId>) -> Self {
        self.preview_id = Some(preview_id);
        self
    }

    pub fn with_user_metadata(mut self, metadata: BTreeMap<String, serde_json::Value>) -> Self {
        self.user_metadata = Some(metadata);
        self
    }

    pub fn with_system_metadata(mut self, metadata: BTreeMap<String, serde_json::Value>) -> Self {
        self.system_metadata = Some(metadata);
        self
    }

    pub fn with_cache_state(mut self, cache_state: SimAssetCacheState) -> Self {
        self.cache_state = Some(cache_state);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.tags.is_none()
            && self.preview_id.is_none()
            && self.user_metadata.is_none()
            && self.system_metadata.is_none()
            && self.cache_state.is_none()
    }
}

impl SimAssetReferenceRequest {
    pub fn new(name: impl Into<String>, size_bytes: u64) -> Self {
        Self {
            name: name.into(),
            size_bytes,
            ..Self::default()
        }
    }

    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.hash = Some(SimAssetHash::new(hash));
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

    pub fn with_preview_id(mut self, preview_id: SimAssetReferenceId) -> Self {
        self.preview_id = Some(preview_id);
        self
    }

    pub fn with_cache_state(mut self, cache_state: SimAssetCacheState) -> Self {
        self.cache_state = cache_state;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAssetDiagnostic {
    pub code: String,
    pub reference_id: Option<SimAssetReferenceId>,
    pub content_id: Option<SimAssetContentId>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimAssetRepository {
    content: BTreeMap<SimAssetContentId, SimAssetContentRecord>,
    content_by_hash: BTreeMap<SimAssetHash, SimAssetContentId>,
    references: BTreeMap<SimAssetReferenceId, SimAssetReferenceRecord>,
    next_content_id: u64,
    next_reference_id: u64,
    clock_ms: u64,
}

impl SimAssetRepository {
    pub fn create_reference(
        &mut self,
        owner_id: SimAssetOwnerId,
        request: SimAssetReferenceRequest,
    ) -> SimAssetReferenceRecord {
        let content =
            self.create_or_reuse_content(request.hash, request.size_bytes, request.mime_type);
        self.next_reference_id = self.next_reference_id.saturating_add(1);
        let now = self.next_timestamp();
        let reference = SimAssetReferenceRecord {
            id: SimAssetReferenceId::new(format!("asset-reference-{}", self.next_reference_id)),
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

    pub fn content_by_hash(&self, hash: &SimAssetHash) -> Option<&SimAssetContentRecord> {
        self.content_by_hash
            .get(hash)
            .and_then(|content_id| self.content.get(content_id))
    }

    pub fn content(
        &self,
        content_id: &SimAssetContentId,
    ) -> Result<&SimAssetContentRecord, SimAssetDiagnostic> {
        self.content
            .get(content_id)
            .ok_or_else(|| SimAssetDiagnostic {
                code: ASSET_CONTENT_NOT_FOUND_CODE.to_string(),
                reference_id: None,
                content_id: Some(content_id.clone()),
                message: format!("asset content `{}` was not found", content_id.as_str()),
            })
    }

    pub fn reference(
        &self,
        reference_id: &SimAssetReferenceId,
    ) -> Result<&SimAssetReferenceRecord, SimAssetDiagnostic> {
        self.references
            .get(reference_id)
            .ok_or_else(|| SimAssetDiagnostic {
                code: ASSET_REFERENCE_NOT_FOUND_CODE.to_string(),
                reference_id: Some(reference_id.clone()),
                content_id: None,
                message: format!("asset reference `{}` was not found", reference_id.as_str()),
            })
    }

    pub fn references_for_owner(
        &self,
        owner_id: &SimAssetOwnerId,
    ) -> Vec<&SimAssetReferenceRecord> {
        self.references
            .values()
            .filter(|reference| &reference.owner_id == owner_id && !reference.is_deleted())
            .collect()
    }

    pub fn update_reference(
        &mut self,
        owner_id: &SimAssetOwnerId,
        reference_id: &SimAssetReferenceId,
        patch: SimAssetReferencePatch,
    ) -> Result<Option<SimAssetReferenceRecord>, SimAssetDiagnostic> {
        let now = self.next_timestamp();
        let Some(reference) = self.references.get_mut(reference_id) else {
            return Err(SimAssetDiagnostic {
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
        if let Some(system_metadata) = patch.system_metadata {
            reference.system_metadata = system_metadata;
        }
        if let Some(cache_state) = patch.cache_state {
            reference.cache_state = cache_state;
        }
        reference.updated_at_ms = now;
        Ok(Some(reference.clone()))
    }

    pub fn soft_delete_reference(
        &mut self,
        owner_id: &SimAssetOwnerId,
        reference_id: &SimAssetReferenceId,
    ) -> Result<bool, SimAssetDiagnostic> {
        let now = self.next_timestamp();
        let Some(reference) = self.references.get_mut(reference_id) else {
            return Err(SimAssetDiagnostic {
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
        hash: Option<SimAssetHash>,
        size_bytes: u64,
        mime_type: Option<String>,
    ) -> SimAssetContentRecord {
        if let Some(hash) = &hash {
            if let Some(content_id) = self.content_by_hash.get(hash) {
                if let Some(content) = self.content.get(content_id) {
                    return content.clone();
                }
            }
        }

        self.next_content_id = self.next_content_id.saturating_add(1);
        let content = SimAssetContentRecord {
            id: SimAssetContentId::new(format!("asset-content-{}", self.next_content_id)),
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

fn sim_asset_coverage_diagnostic(
    code: impl Into<String>,
    message: impl Into<String>,
) -> SimAssetCoverageDiagnostic {
    SimAssetCoverageDiagnostic {
        code: code.into(),
        message: message.into(),
    }
}
