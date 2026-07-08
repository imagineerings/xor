use crate::{
    ASSET_DOWNLOAD_FILE_NOT_FOUND_CODE, ASSET_DOWNLOAD_PREVIEW_NOT_FOUND_CODE, SimAssetApi,
    SimAssetCacheState, SimAssetContentDispositionKind, SimAssetDownloadResolver, SimAssetOwnerId,
    SimAssetReferenceId, SimAssetUpdateRequest, SimAssetUploadRequest, content_disposition,
    safe_content_type,
};

fn upload_png(
    api: &mut SimAssetApi,
    owner: &SimAssetOwnerId,
    name: &str,
    file_path: &str,
) -> SimAssetReferenceId {
    api.upload(
        owner.clone(),
        SimAssetUploadRequest::new(name, 100)
            .expect("upload")
            .with_mime_type("image/png")
            .with_cache_state(SimAssetCacheState::default().with_file_path(file_path)),
    )
    .expect("upload")
    .reference
    .id
}

#[test]
fn download_response_uses_safe_content_type_and_disposition() {
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let reference_id = api
        .upload(
            owner.clone(),
            SimAssetUploadRequest::new("unsafe/name\".html", 12)
                .expect("upload")
                .with_mime_type("text/html")
                .with_cache_state(SimAssetCacheState::default().with_file_path("outputs/a.html")),
        )
        .expect("upload")
        .reference
        .id;

    let response = SimAssetDownloadResolver::new(&api)
        .download(
            &owner,
            &reference_id,
            SimAssetContentDispositionKind::Attachment,
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
    let owner = SimAssetOwnerId::new("user-a");
    let other_owner = SimAssetOwnerId::new("user-b");
    let mut api = SimAssetApi::default();
    let reference_id = upload_png(&mut api, &owner, "castle.png", "outputs/castle.png");

    assert!(
        SimAssetDownloadResolver::new(&api)
            .download(
                &other_owner,
                &reference_id,
                SimAssetContentDispositionKind::Inline
            )
            .expect("download")
            .is_none()
    );
}

#[test]
fn download_reports_missing_cached_files_without_host_path_leak() {
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let reference_id = api
        .upload(
            owner.clone(),
            SimAssetUploadRequest::new("missing.png", 100)
                .expect("upload")
                .with_mime_type("image/png")
                .with_cache_state(
                    SimAssetCacheState::default()
                        .with_file_path("/private/sim/output/missing.png")
                        .missing(),
                ),
        )
        .expect("upload")
        .reference
        .id;

    let error = SimAssetDownloadResolver::new(&api)
        .download(
            &owner,
            &reference_id,
            SimAssetContentDispositionKind::Attachment,
        )
        .expect_err("missing file should fail");

    assert_eq!(error.code, ASSET_DOWNLOAD_FILE_NOT_FOUND_CODE);
    assert!(!error.message.contains("/private"));
}

#[test]
fn preview_resolution_uses_explicit_preview_reference_and_sim_media_route() {
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let source_id = upload_png(&mut api, &owner, "source.png", "outputs/source.png");
    let preview_id = upload_png(&mut api, &owner, "preview.png", "outputs/preview.png");
    api.update(
        &owner,
        &source_id,
        SimAssetUpdateRequest::default().with_preview_id(Some(preview_id.clone())),
    )
    .expect("update")
    .expect("visible");

    let preview = SimAssetDownloadResolver::new(&api)
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
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let source_id = upload_png(&mut api, &owner, "source.png", "outputs/source.png");
    api.update(
        &owner,
        &source_id,
        SimAssetUpdateRequest::default()
            .with_preview_id(Some(SimAssetReferenceId::new("asset-reference-missing"))),
    )
    .expect("update")
    .expect("visible");

    let error = SimAssetDownloadResolver::new(&api)
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
        content_disposition(SimAssetContentDispositionKind::Inline, "asset.png"),
        "inline; filename=\"asset.png\""
    );
}
