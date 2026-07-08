use std::path::PathBuf;

use crate::{
    ComfyApiNodeMode, ComfyCacheClass, ComfyContentDisposition, ComfyContentSecurityPolicy,
    ComfyOriginCheck, ComfyPathRoots, ORIGIN_MISMATCH_CODE, PATH_ESCAPE_CODE, UNKNOWN_ROOT_CODE,
};

#[test]
fn loopback_origin_check_rejects_cross_site_browser_requests() {
    let allowed = ComfyOriginCheck {
        request_host: "127.0.0.1:8188".to_string(),
        origin: Some("http://127.0.0.1:8188".to_string()),
    };
    assert!(allowed.validate_loopback_origin().is_ok());

    let rejected = ComfyOriginCheck {
        request_host: "127.0.0.1:8188".to_string(),
        origin: Some("https://example.com".to_string()),
    }
    .validate_loopback_origin()
    .expect_err("cross-site loopback request should be rejected");
    assert_eq!(rejected.code, ORIGIN_MISMATCH_CODE);

    let remote_host = ComfyOriginCheck {
        request_host: "sim.internal".to_string(),
        origin: Some("https://example.com".to_string()),
    };
    assert!(remote_host.validate_loopback_origin().is_ok());
}

#[test]
fn api_node_mode_selects_content_security_policy() {
    let disabled = ComfyContentSecurityPolicy::for_api_node_mode(ComfyApiNodeMode::Disabled);
    assert!(disabled.value.contains("connect-src 'self'"));
    assert!(!disabled.value.contains("https:"));

    let enabled = ComfyContentSecurityPolicy::for_api_node_mode(ComfyApiNodeMode::Enabled);
    assert!(enabled.value.contains("https:"));
    assert!(enabled.value.contains("wss:"));
}

#[test]
fn unsafe_view_content_is_forced_to_download() {
    let html = ComfyContentDisposition::safe_for_view("text/html; charset=utf-8");
    assert_eq!(html.content_type, "application/octet-stream");
    assert!(html.attachment);

    let png = ComfyContentDisposition::safe_for_view("image/png");
    assert_eq!(png.content_type, "image/png");
    assert!(!png.attachment);
}

#[test]
fn cache_classes_map_to_explicit_cache_control_values() {
    assert_eq!(
        ComfyCacheClass::StaticAsset.cache_control(),
        "public, max-age=31536000, immutable"
    );
    assert_eq!(ComfyCacheClass::Dynamic.cache_control(), "no-cache");
    assert_eq!(ComfyCacheClass::Sensitive.cache_control(), "no-store");
}

#[test]
fn path_roots_confine_relative_paths() {
    let roots = ComfyPathRoots::new()
        .with_root("input", PathBuf::from("/safe/input"))
        .with_root("output", PathBuf::from("/safe/output"));

    assert_eq!(
        roots.resolve("input", "nested/image.png").unwrap(),
        PathBuf::from("/safe/input/nested/image.png")
    );

    let escape = roots
        .resolve("input", "../secret.txt")
        .expect_err("parent directory escape should be rejected");
    assert_eq!(escape.code, PATH_ESCAPE_CODE);

    let absolute = roots
        .resolve("input", "/etc/passwd")
        .expect_err("absolute path should be rejected");
    assert_eq!(absolute.code, PATH_ESCAPE_CODE);

    let unknown = roots
        .resolve("temp", "file.png")
        .expect_err("unknown roots should be rejected");
    assert_eq!(unknown.code, UNKNOWN_ROOT_CODE);
}
