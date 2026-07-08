use crate::{
    SIM_PROVIDER_IO_MISSING_MIME_CODE, SIM_PROVIDER_IO_MISSING_OUTPUT_CODE,
    SIM_PROVIDER_SIGNED_URL_PLACEHOLDER, SimProviderId, SimProviderIoService, SimProviderOutput,
    SimProviderOutputKind, SimProviderRemoteTaskHandle, SimProviderSourceMedia,
};

#[test]
fn provider_io_prepares_uploads_without_exposing_signed_urls() {
    let service = SimProviderIoService::new();
    let upload = service.prepare_upload(
        SimProviderId::new("runway"),
        SimProviderSourceMedia::new("asset:source-video", "video/mp4").with_signed_upload_url(
            "https://upload.example.com/file.mp4?X-Amz-Signature=secret&Expires=1",
        ),
    );

    assert_eq!(upload.provider_id.as_str(), "runway");
    assert_eq!(upload.source_asset_ref, "asset:source-video");
    assert_eq!(upload.upload_ref, "sim-upload:asset:source-video");
    assert_eq!(
        upload.redacted_upload_url.as_deref(),
        Some(SIM_PROVIDER_SIGNED_URL_PLACEHOLDER)
    );
}

#[test]
fn provider_io_imports_outputs_with_media_metadata_and_provenance() {
    let service = SimProviderIoService::new();
    let handle = handle();
    let report = service.import_outputs(
        &handle,
        vec!["asset:prompt-image".to_string()],
        vec![
            SimProviderOutput::new("image-1", SimProviderOutputKind::Image, "image/png")
                .with_remote_url("https://cdn.example.com/image.png?sig=secret")
                .with_metadata("width", "1024")
                .with_metadata("height", "1024"),
        ],
    );

    assert!(report.is_complete());
    assert_eq!(report.assets.len(), 1);
    let asset = &report.assets[0];
    assert_eq!(asset.asset_ref, "sim-asset:image-1");
    assert_eq!(asset.mime_type, "image/png");
    assert_eq!(
        asset.redacted_remote_url.as_deref(),
        Some(SIM_PROVIDER_SIGNED_URL_PLACEHOLDER)
    );
    assert_eq!(asset.metadata["width"], "1024");
    assert_eq!(asset.provenance.provider_id.as_str(), "openai");
    assert_eq!(asset.provenance.remote_task_id.as_str(), "remote-1");
    assert_eq!(
        asset.provenance.source_asset_refs,
        vec!["asset:prompt-image".to_string()]
    );
}

#[test]
fn provider_io_reports_missing_outputs() {
    let service = SimProviderIoService::new();
    let report = service.import_outputs(&handle(), Vec::new(), Vec::new());

    assert!(!report.is_complete());
    assert_eq!(
        report.diagnostics[0].code,
        SIM_PROVIDER_IO_MISSING_OUTPUT_CODE
    );
}

#[test]
fn provider_io_rejects_outputs_without_mime_metadata() {
    let service = SimProviderIoService::new();
    let report = service.import_outputs(
        &handle(),
        Vec::new(),
        vec![SimProviderOutput::new(
            "bad-output",
            SimProviderOutputKind::Video,
            "",
        )],
    );

    assert!(report.assets.is_empty());
    assert_eq!(
        report.diagnostics[0].code,
        SIM_PROVIDER_IO_MISSING_MIME_CODE
    );
}

fn handle() -> SimProviderRemoteTaskHandle {
    SimProviderRemoteTaskHandle::new(
        SimProviderId::new("openai"),
        "remote-1",
        "OpenAIImageGenerate",
        "sim.provider.openai.OpenAIImageGenerate",
    )
}
