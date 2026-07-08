use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    ComfyAssetApi, ComfyAssetApiDiagnostic, ComfyAssetContentId, ComfyAssetOwnerId,
    ComfyAssetReferenceDetail, ComfyAssetReferenceId,
};

pub const ASSET_DOWNLOAD_FILE_NOT_FOUND_CODE: &str =
    "world_model.comfy_assets.download_file_not_found";
pub const ASSET_DOWNLOAD_PREVIEW_NOT_FOUND_CODE: &str =
    "world_model.comfy_assets.preview_not_found";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyAssetContentDispositionKind {
    Attachment,
    Inline,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetDownloadResponse {
    pub reference_id: ComfyAssetReferenceId,
    pub content_id: ComfyAssetContentId,
    pub size_bytes: u64,
    pub content_type: String,
    pub content_disposition: String,
    pub file_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetMediaPreviewRoute {
    pub route_name: String,
    pub reference_id: ComfyAssetReferenceId,
    pub content_id: ComfyAssetContentId,
    pub content_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyAssetPreviewResolution {
    pub source_reference_id: ComfyAssetReferenceId,
    pub preview_reference_id: ComfyAssetReferenceId,
    pub media_route: ComfyAssetMediaPreviewRoute,
}

pub struct ComfyAssetDownloadResolver<'a> {
    api: &'a ComfyAssetApi,
}

impl<'a> ComfyAssetDownloadResolver<'a> {
    pub fn new(api: &'a ComfyAssetApi) -> Self {
        Self { api }
    }

    pub fn download(
        &self,
        owner_id: &ComfyAssetOwnerId,
        reference_id: &ComfyAssetReferenceId,
        disposition: ComfyAssetContentDispositionKind,
    ) -> Result<Option<ComfyAssetDownloadResponse>, ComfyAssetApiDiagnostic> {
        let Some(detail) = self.api.detail(owner_id, reference_id)? else {
            return Ok(None);
        };
        if detail.reference.cache_state.is_missing {
            return Err(download_file_not_found(&detail));
        }
        let content_type = safe_content_type(detail.content.mime_type.as_deref());
        Ok(Some(ComfyAssetDownloadResponse {
            reference_id: detail.reference.id,
            content_id: detail.content.id,
            size_bytes: detail.content.size_bytes,
            content_disposition: content_disposition(disposition, &detail.reference.name),
            content_type,
            file_path: detail.reference.cache_state.file_path,
        }))
    }

    pub fn resolve_preview(
        &self,
        owner_id: &ComfyAssetOwnerId,
        reference_id: &ComfyAssetReferenceId,
    ) -> Result<Option<ComfyAssetPreviewResolution>, ComfyAssetApiDiagnostic> {
        let Some(source) = self.api.detail(owner_id, reference_id)? else {
            return Ok(None);
        };
        let preview_reference_id = source
            .reference
            .preview_id
            .clone()
            .unwrap_or_else(|| source.reference.id.clone());
        let preview = match self.api.detail(owner_id, &preview_reference_id) {
            Ok(Some(preview)) => preview,
            Ok(None) => {
                return Err(preview_not_found(preview_reference_id));
            }
            Err(error) if error.reference_id.as_ref() == Some(&preview_reference_id) => {
                return Err(preview_not_found(preview_reference_id));
            }
            Err(error) => return Err(error),
        };
        if preview.reference.cache_state.is_missing {
            return Err(download_file_not_found(&preview));
        }

        Ok(Some(ComfyAssetPreviewResolution {
            source_reference_id: source.reference.id,
            preview_reference_id: preview.reference.id.clone(),
            media_route: ComfyAssetMediaPreviewRoute {
                route_name: "sim.media.preview".to_string(),
                reference_id: preview.reference.id,
                content_id: preview.content.id,
                content_type: safe_content_type(preview.content.mime_type.as_deref()),
            },
        }))
    }
}

pub fn safe_content_type(mime_type: Option<&str>) -> String {
    match mime_type
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "image/png" => "image/png",
        "image/jpeg" | "image/jpg" => "image/jpeg",
        "image/webp" => "image/webp",
        "image/gif" => "image/gif",
        "audio/wav" | "audio/x-wav" => "audio/wav",
        "audio/mpeg" => "audio/mpeg",
        "video/mp4" => "video/mp4",
        "application/json" => "application/json",
        _ => "application/octet-stream",
    }
    .to_string()
}

pub fn content_disposition(
    disposition: ComfyAssetContentDispositionKind,
    file_name: &str,
) -> String {
    let disposition = match disposition {
        ComfyAssetContentDispositionKind::Attachment => "attachment",
        ComfyAssetContentDispositionKind::Inline => "inline",
    };
    format!(
        "{disposition}; filename=\"{}\"",
        sanitize_file_name(file_name)
    )
}

fn sanitize_file_name(file_name: &str) -> String {
    let sanitized = file_name
        .chars()
        .map(|character| match character {
            '"' | '\\' | '/' | '\0'..='\u{1f}' => '_',
            _ => character,
        })
        .collect::<String>();
    if sanitized.trim().is_empty() {
        "asset.bin".to_string()
    } else {
        sanitized
    }
}

fn download_file_not_found(detail: &ComfyAssetReferenceDetail) -> ComfyAssetApiDiagnostic {
    ComfyAssetApiDiagnostic {
        code: ASSET_DOWNLOAD_FILE_NOT_FOUND_CODE.to_string(),
        reference_id: Some(detail.reference.id.clone()),
        message: "asset content file was not found".to_string(),
    }
}

fn preview_not_found(reference_id: ComfyAssetReferenceId) -> ComfyAssetApiDiagnostic {
    ComfyAssetApiDiagnostic {
        code: ASSET_DOWNLOAD_PREVIEW_NOT_FOUND_CODE.to_string(),
        reference_id: Some(reference_id),
        message: "asset preview reference was not found".to_string(),
    }
}
