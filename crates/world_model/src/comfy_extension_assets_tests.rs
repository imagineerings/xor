use std::path::PathBuf;

use crate::{
    SIM_EXTENSION_ASSET_DEPRECATED_PATH_CODE, SIM_EXTENSION_ASSET_PATH_ESCAPE_CODE,
    SIM_EXTENSION_ASSET_UNKNOWN_ROOT_CODE, SimExtensionAssetKind, SimExtensionAssetRootId,
    SimExtensionAssetService, SimExtensionId, SimExtensionRecord, SimExtensionSourceKind,
};

#[test]
fn extension_asset_service_resolves_web_assets_as_native_sim_routes() {
    let extension = extension("Web Pack");
    let mut service = SimExtensionAssetService::new();
    let root = service.register_web_root(&extension, "/custom_nodes/web_pack/web");

    let response = service
        .resolve(&root, "assets/main.js")
        .expect("web asset should resolve");

    assert_eq!(response.extension_id, extension.id);
    assert_eq!(response.kind, SimExtensionAssetKind::Web);
    assert_eq!(response.relative_path, "assets/main.js");
    assert_eq!(
        response.file_path,
        PathBuf::from("/custom_nodes/web_pack/web/assets/main.js")
    );
    assert!(
        response
            .route_path
            .starts_with("/sim/extensions/web-pack/web/")
    );
    assert_eq!(response.content_type, "application/javascript");
    assert!(!response.attachment);
    assert_eq!(
        response.cache_control,
        "public, max-age=31536000, immutable"
    );
}

#[test]
fn extension_asset_service_resolves_template_assets_with_safe_content_types() {
    let extension = extension("Template Pack");
    let mut service = SimExtensionAssetService::new();
    let root = service.register_template_root(&extension, "/custom_nodes/template_pack/templates");

    let preview = service
        .resolve(&root, "preview.webp")
        .expect("template preview should resolve");
    let html = service
        .resolve(&root, "danger.html")
        .expect("unknown executable asset should still resolve as download");

    assert_eq!(preview.kind, SimExtensionAssetKind::Template);
    assert_eq!(preview.content_type, "image/webp");
    assert!(!preview.attachment);
    assert_eq!(html.content_type, "application/octet-stream");
    assert!(html.attachment);
}

#[test]
fn extension_asset_service_rejects_path_escape_and_unknown_roots() {
    let extension = extension("Safe Pack");
    let mut service = SimExtensionAssetService::new();
    let root = service.register_web_root(&extension, "/custom_nodes/safe_pack/web");

    let escape = service
        .resolve(&root, "../secrets.json")
        .expect_err("path escape should be rejected");
    assert_eq!(escape.code, SIM_EXTENSION_ASSET_PATH_ESCAPE_CODE);
    assert_eq!(escape.extension_id, Some(extension.id.clone()));

    let unknown = service
        .resolve(
            &SimExtensionAssetRootId::new(
                &SimExtensionId::new("missing"),
                SimExtensionAssetKind::Web,
            ),
            "asset.js",
        )
        .expect_err("unknown root should be rejected");
    assert_eq!(unknown.code, SIM_EXTENSION_ASSET_UNKNOWN_ROOT_CODE);
}

#[test]
fn extension_asset_service_records_deprecated_path_warnings() {
    let extension = extension("Legacy Pack");
    let mut service = SimExtensionAssetService::new();
    let root = service.register_web_root(&extension, "/custom_nodes/legacy_pack/web");

    service
        .resolve_deprecated_path(&root, "legacy/main.css")
        .expect("deprecated path should resolve through native service");

    assert_eq!(service.diagnostics().len(), 1);
    assert_eq!(
        service.diagnostics()[0].code,
        SIM_EXTENSION_ASSET_DEPRECATED_PATH_CODE
    );
    assert_eq!(service.diagnostics()[0].extension_id, Some(extension.id));
}

fn extension(name: &str) -> SimExtensionRecord {
    SimExtensionRecord {
        id: SimExtensionId::new(name),
        display_name: name.to_string(),
        source_path: PathBuf::from(format!("/custom_nodes/{name}")),
        source_kind: SimExtensionSourceKind::Directory,
        root_index: 0,
        load_order: 0,
    }
}
