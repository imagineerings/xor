use crate::{
    ASSET_DOWNLOAD_FILE_NOT_FOUND_CODE, ASSET_DOWNLOAD_PREVIEW_NOT_FOUND_CODE, ComfyAssetApi,
    ComfyAssetCacheState, ComfyAssetContentDispositionKind, ComfyAssetDownloadResolver,
    ComfyAssetOwnerId, ComfyAssetReferenceId, ComfyAssetUpdateRequest, ComfyAssetUploadRequest,
    content_disposition, safe_content_type,
};

fn upload_png(
    api: &mut ComfyAssetApi,
    owner: &ComfyAssetOwnerId,
    name: &str,
    file_path: &str,
) -> ComfyAssetReferenceId {
    api.upload(
        owner.clone(),
        ComfyAssetUploadRequest::new(name, 100)
            .expect("upload")
            .with_mime_type("image/png")
            .with_cache_state(ComfyAssetCacheState::default().with_file_path(file_path)),
    )
    .expect("upload")
    .reference
    .id
}

#[test]
fn download_response_uses_safe_content_type_and_disposition() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let mut api = ComfyAssetApi::default();
    let reference_id = api
        .upload(
            owner.clone(),
            ComfyAssetUploadRequest::new("unsafe/name\".html", 12)
                .expect("upload")
                .with_mime_type("text/html")
                .with_cache_state(ComfyAssetCacheState::default().with_file_path("outputs/a.html")),
        )
        .expect("upload")
        .reference
        .id;

    let response = ComfyAssetDownloadResolver::new(&api)
        .download(
            &owner,
            &reference_id,
            ComfyAssetContentDispositionKind::Attachment,
        )
        .expect("download")
        .expect("visible");

    assert_eq!(response.content_type, "application/octet-stream");
    assert_eq!(
        response.content_disposition,
        "attachment; filename=\"unsafe_name_.html\""
    );
    assert_eq!(
        response.file_path.as_deref(),
        Some("outputs/a.html".as_ref())
    );
}

#[test]
fn download_respects_owner_scope() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let other_owner = ComfyAssetOwnerId::new("user-b");
    let mut api = ComfyAssetApi::default();
    let reference_id = upload_png(&mut api, &owner, "castle.png", "outputs/castle.png");

    assert!(
        ComfyAssetDownloadResolver::new(&api)
            .download(
                &other_owner,
                &reference_id,
                ComfyAssetContentDispositionKind::Inline
            )
            .expect("download")
            .is_none()
    );
}

#[test]
fn download_reports_missing_cached_files_without_host_path_leak() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let mut api = ComfyAssetApi::default();
    let reference_id = api
        .upload(
            owner.clone(),
            ComfyAssetUploadRequest::new("missing.png", 100)
                .expect("upload")
                .with_mime_type("image/png")
                .with_cache_state(
                    ComfyAssetCacheState::default()
                        .with_file_path("/private/sim/output/missing.png")
                        .missing(),
                ),
        )
        .expect("upload")
        .reference
        .id;

    let error = ComfyAssetDownloadResolver::new(&api)
        .download(
            &owner,
            &reference_id,
            ComfyAssetContentDispositionKind::Attachment,
        )
        .expect_err("missing file should fail");

    assert_eq!(error.code, ASSET_DOWNLOAD_FILE_NOT_FOUND_CODE);
    assert!(!error.message.contains("/private"));
}

#[test]
fn preview_resolution_uses_explicit_preview_reference_and_sim_media_route() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let mut api = ComfyAssetApi::default();
    let source_id = upload_png(&mut api, &owner, "source.png", "outputs/source.png");
    let preview_id = upload_png(&mut api, &owner, "preview.png", "outputs/preview.png");
    api.update(
        &owner,
        &source_id,
        ComfyAssetUpdateRequest::default().with_preview_id(Some(preview_id.clone())),
    )
    .expect("update")
    .expect("visible");

    let preview = ComfyAssetDownloadResolver::new(&api)
        .resolve_preview(&owner, &source_id)
        .expect("preview")
        .expect("visible");

    assert_eq!(preview.source_reference_id, source_id);
    assert_eq!(preview.preview_reference_id, preview_id);
    assert_eq!(preview.media_route.route_name, "sim.media.preview");
    assert_eq!(preview.media_route.content_type, "image/png");
}

#[test]
fn preview_resolution_reports_missing_preview_reference() {
    let owner = ComfyAssetOwnerId::new("user-a");
    let mut api = ComfyAssetApi::default();
    let source_id = upload_png(&mut api, &owner, "source.png", "outputs/source.png");
    api.update(
        &owner,
        &source_id,
        ComfyAssetUpdateRequest::default()
            .with_preview_id(Some(ComfyAssetReferenceId::new("asset-reference-missing"))),
    )
    .expect("update")
    .expect("visible");

    let error = ComfyAssetDownloadResolver::new(&api)
        .resolve_preview(&owner, &source_id)
        .expect_err("missing preview should fail");

    assert_eq!(error.code, ASSET_DOWNLOAD_PREVIEW_NOT_FOUND_CODE);
}

#[test]
fn safe_content_type_and_disposition_helpers_are_deterministic() {
    assert_eq!(safe_content_type(Some("image/jpg")), "image/jpeg");
    assert_eq!(
        safe_content_type(Some("image/svg+xml")),
        "application/octet-stream"
    );
    assert_eq!(
        content_disposition(ComfyAssetContentDispositionKind::Inline, "asset.png"),
        "inline; filename=\"asset.png\""
    );
}
