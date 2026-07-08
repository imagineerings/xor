use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    ComfyAssetApi, ComfyAssetApiDiagnostic, ComfyAssetCacheState, ComfyAssetListQuery,
    ComfyAssetOwnerId, ComfyAssetOwnerScope, ComfyAssetScanRoot, ComfyAssetScannedFile,
    ComfyAssetUploadRequest,
};

pub const ASSET_SEED_MISSING_ROOT_CODE: &str = "world_model.comfy_assets.seed_missing_root";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyAssetSeedState {
    #[default]
    Idle,
    Running,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetSeedDiagnostic {
    pub code: String,
    pub path: Option<PathBuf>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetSeedProgress {
    pub scanned: usize,
    pub total: usize,
    pub created: usize,
    pub skipped: usize,
    pub state: ComfyAssetSeedState,
    pub errors: Vec<ComfyAssetSeedDiagnostic>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetSeedReport {
    pub progress: ComfyAssetSeedProgress,
    pub pruned_missing: usize,
}

pub struct ComfyAssetSeeder<'a> {
    api: &'a mut ComfyAssetApi,
    owner_id: ComfyAssetOwnerId,
    roots: Vec<ComfyAssetScanRoot>,
}

impl<'a> ComfyAssetSeeder<'a> {
    pub fn new(
        api: &'a mut ComfyAssetApi,
        owner_id: ComfyAssetOwnerId,
        roots: Vec<ComfyAssetScanRoot>,
    ) -> Self {
        Self {
            api,
            owner_id,
            roots,
        }
    }

    pub fn seed(
        &mut self,
        files: &[ComfyAssetScannedFile],
        cancel_after: Option<usize>,
    ) -> Result<ComfyAssetSeedReport, ComfyAssetApiDiagnostic> {
        let mut progress = ComfyAssetSeedProgress {
            total: files.len(),
            state: ComfyAssetSeedState::Running,
            ..ComfyAssetSeedProgress::default()
        };

        for file in files {
            if cancel_after.is_some_and(|limit| progress.scanned >= limit) {
                progress.state = ComfyAssetSeedState::Cancelled;
                return Ok(ComfyAssetSeedReport {
                    progress,
                    pruned_missing: 0,
                });
            }
            progress.scanned = progress.scanned.saturating_add(1);
            let Some(full_path) = self.full_path(file) else {
                progress.errors.push(ComfyAssetSeedDiagnostic {
                    code: ASSET_SEED_MISSING_ROOT_CODE.to_string(),
                    path: Some(file.relative_path.clone()),
                    message: "asset seed file root is not registered".to_string(),
                });
                progress.skipped = progress.skipped.saturating_add(1);
                continue;
            };
            if self.reference_exists_for_path(&full_path)? {
                progress.skipped = progress.skipped.saturating_add(1);
                continue;
            }
            self.register_file(file, full_path)?;
            progress.created = progress.created.saturating_add(1);
        }

        progress.state = ComfyAssetSeedState::Completed;
        Ok(ComfyAssetSeedReport {
            progress,
            pruned_missing: 0,
        })
    }

    pub fn prune_missing_outside_roots(&mut self) -> Result<usize, ComfyAssetApiDiagnostic> {
        let query = ComfyAssetListQuery::new(ComfyAssetOwnerScope {
            owner_id: self.owner_id.clone(),
        });
        let page = self.api.list(&query)?;
        let mut pruned = 0usize;
        for item in page.items {
            let Some(file_path) = &item.reference.cache_state.file_path else {
                continue;
            };
            if self.path_is_inside_known_root(file_path) {
                continue;
            }
            let cache_state = item.reference.cache_state.clone().missing();
            if self
                .api
                .update_cache_state(&self.owner_id, &item.reference.id, cache_state)?
                .is_some()
            {
                pruned = pruned.saturating_add(1);
            }
        }
        Ok(pruned)
    }

    fn register_file(
        &mut self,
        file: &ComfyAssetScannedFile,
        full_path: PathBuf,
    ) -> Result<(), ComfyAssetApiDiagnostic> {
        let mut cache_state = ComfyAssetCacheState::default()
            .with_file_path(full_path)
            .verified();
        if let Some(modified_at_ms) = file.modified_at_ms {
            cache_state = cache_state.with_modified_at_ms(modified_at_ms).verified();
        }

        let mut upload = ComfyAssetUploadRequest::new(
            file.relative_path
                .file_name()
                .and_then(|file_name| file_name.to_str())
                .unwrap_or("asset"),
            file.size_bytes,
        )?
        .with_cache_state(cache_state);
        if let Some(mime_type) = &file.mime_type {
            upload = upload.with_mime_type(mime_type.clone());
        }
        if let Some(hash) = &file.hash {
            upload = upload.with_known_hash(hash)?;
        }
        self.api.upload(self.owner_id.clone(), upload)?;
        Ok(())
    }

    fn reference_exists_for_path(&self, full_path: &Path) -> Result<bool, ComfyAssetApiDiagnostic> {
        let query = ComfyAssetListQuery::new(ComfyAssetOwnerScope {
            owner_id: self.owner_id.clone(),
        });
        let page = self.api.list(&query)?;
        Ok(page.items.iter().any(|item| {
            item.reference.cache_state.file_path.as_deref() == Some(full_path)
                && !item.reference.cache_state.is_missing
        }))
    }

    fn full_path(&self, file: &ComfyAssetScannedFile) -> Option<PathBuf> {
        self.roots
            .iter()
            .find(|root| root.kind == file.root_kind)
            .map(|root| root.path.join(&file.relative_path))
    }

    fn path_is_inside_known_root(&self, path: &Path) -> bool {
        self.roots.iter().any(|root| path.starts_with(&root.path))
    }
}
