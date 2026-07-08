use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    SimAssetApi, SimAssetApiDiagnostic, SimAssetCacheState, SimAssetOwnerId,
    SimAssetReferenceDetail, SimAssetReferenceId, SimAssetUpdateRequest, SimAssetUploadRequest,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimAssetOutputRegistrationRequest {
    pub owner_id: SimAssetOwnerId,
    pub file_name: String,
    pub file_path: PathBuf,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub hash: Option<String>,
    pub job_id: Option<String>,
    pub provenance_id: Option<String>,
    pub extracted_metadata: BTreeMap<String, Value>,
    pub enqueue_enrichment: bool,
}

impl SimAssetOutputRegistrationRequest {
    pub fn new(
        owner_id: SimAssetOwnerId,
        file_name: impl Into<String>,
        file_path: impl Into<PathBuf>,
        size_bytes: u64,
    ) -> Self {
        Self {
            owner_id,
            file_name: file_name.into(),
            file_path: file_path.into(),
            size_bytes,
            mime_type: None,
            hash: None,
            job_id: None,
            provenance_id: None,
            extracted_metadata: BTreeMap::new(),
            enqueue_enrichment: true,
        }
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.hash = Some(hash.into());
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

    pub fn with_extracted_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extracted_metadata.insert(key.into(), value);
        self
    }

    pub fn without_enrichment(mut self) -> Self {
        self.enqueue_enrichment = false;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAssetEnrichmentJob {
    pub owner_id: SimAssetOwnerId,
    pub reference_id: SimAssetReferenceId,
    pub target_enrichment_level: u8,
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAssetEnrichmentQueue {
    pending: VecDeque<SimAssetEnrichmentJob>,
}

impl SimAssetEnrichmentQueue {
    pub fn push(&mut self, job: SimAssetEnrichmentJob) {
        self.pending.push_back(job);
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn process_next(
        &mut self,
        api: &mut SimAssetApi,
    ) -> Result<Option<SimAssetReferenceDetail>, SimAssetApiDiagnostic> {
        let Some(job) = self.pending.pop_front() else {
            return Ok(None);
        };
        let detail = match api.detail(&job.owner_id, &job.reference_id) {
            Ok(Some(detail)) => detail,
            Ok(None) => return Ok(None),
            Err(error) if error.reference_id.as_ref() == Some(&job.reference_id) => {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let mut system_metadata = detail.reference.system_metadata.clone();
        for (key, value) in job.metadata {
            system_metadata.insert(key, value);
        }
        let cache_state = detail
            .reference
            .cache_state
            .with_enrichment_level(job.target_enrichment_level)
            .verified();
        api.update(
            &job.owner_id,
            &job.reference_id,
            SimAssetUpdateRequest::default()
                .with_system_metadata(system_metadata)
                .with_cache_state(cache_state),
        )
    }
}

pub struct SimAssetOutputRegistrar<'a> {
    api: &'a mut SimAssetApi,
    queue: &'a mut SimAssetEnrichmentQueue,
}

impl<'a> SimAssetOutputRegistrar<'a> {
    pub fn new(api: &'a mut SimAssetApi, queue: &'a mut SimAssetEnrichmentQueue) -> Self {
        Self { api, queue }
    }

    pub fn register_output(
        &mut self,
        request: SimAssetOutputRegistrationRequest,
    ) -> Result<SimAssetReferenceDetail, SimAssetApiDiagnostic> {
        let mut upload = SimAssetUploadRequest::new(&request.file_name, request.size_bytes)?
            .with_cache_state(
                SimAssetCacheState::default()
                    .with_file_path(request.file_path)
                    .verified(),
            );
        if let Some(mime_type) = request.mime_type {
            upload = upload.with_mime_type(mime_type);
        }
        if let Some(hash) = &request.hash {
            upload = upload.with_known_hash(hash)?;
        }
        if let Some(job_id) = &request.job_id {
            upload = upload.with_job_id(job_id);
        }
        if let Some(provenance_id) = &request.provenance_id {
            upload = upload.with_provenance_id(provenance_id);
        }
        for (key, value) in &request.extracted_metadata {
            upload = upload.with_system_metadata(key, value.clone());
        }

        let detail = self.api.upload(request.owner_id.clone(), upload)?;
        if request.enqueue_enrichment {
            self.queue.push(SimAssetEnrichmentJob {
                owner_id: request.owner_id,
                reference_id: detail.reference.id.clone(),
                target_enrichment_level: 1,
                metadata: request.extracted_metadata,
            });
        }
        Ok(detail)
    }
}
