use std::path::PathBuf;

use crate::{
    DeviceBackend, LAUNCH_PROFILE_INVALID_OPTION_CODE, LAUNCH_PROFILE_UNSUPPORTED_OPTION_CODE,
    MemoryMode, PrecisionPolicy, SimLaunchDiagnosticSeverity, SimLaunchProfileParser,
};

#[test]
fn launch_profile_parser_captures_native_sim_configuration() {
    let profile = SimLaunchProfileParser::new().parse_args([
        "--listen",
        "0.0.0.0",
        "--port",
        "9191",
        "--tls-keyfile",
        "key.pem",
        "--tls-certfile",
        "cert.pem",
        "--enable-cors-header",
        "https://sim.local",
        "--max-upload-size",
        "64",
        "--base-directory",
        "/sim/base",
        "--input-directory",
        "/sim/input",
        "--output-directory",
        "/sim/output",
        "--temp-directory",
        "/sim/temp",
        "--user-directory",
        "/sim/user",
        "--auto-launch",
        "--verbose",
        "--log-stdout",
        "--log-level",
        "debug",
        "--disable-assets",
        "--database-url",
        "sqlite://assets.db",
        "--disable-api-nodes",
        "--disable-custom-nodes",
        "--custom-node-directory",
        "/sim/custom",
        "--manager-mode",
        "manual",
        "--feature",
        "preview_metadata=true",
        "--enable-compress-response",
    ]);

    assert!(profile.is_valid());
    assert_eq!(profile.network.listen_address, "0.0.0.0");
    assert_eq!(profile.network.port, 9191);
    assert_eq!(profile.network.tls_keyfile, Some(PathBuf::from("key.pem")));
    assert_eq!(
        profile.network.tls_certfile,
        Some(PathBuf::from("cert.pem"))
    );
    assert_eq!(
        profile.network.cors_origin.as_deref(),
        Some("https://sim.local")
    );
    assert_eq!(profile.upload_limit_bytes, Some(64 * 1024 * 1024));
    assert_eq!(
        profile.directories.base_directory,
        Some(PathBuf::from("/sim/base"))
    );
    assert_eq!(
        profile.directories.input_directory,
        Some(PathBuf::from("/sim/input"))
    );
    assert_eq!(
        profile.directories.output_directory,
        Some(PathBuf::from("/sim/output"))
    );
    assert_eq!(
        profile.directories.temp_directory,
        Some(PathBuf::from("/sim/temp"))
    );
    assert_eq!(
        profile.directories.user_directory,
        Some(PathBuf::from("/sim/user"))
    );
    assert!(profile.auto_launch);
    assert!(profile.logging.verbose);
    assert!(profile.logging.stdout);
    assert_eq!(profile.logging.level, "debug");
    assert!(!profile.assets.enabled);
    assert_eq!(profile.database_url.as_deref(), Some("sqlite://assets.db"));
    assert!(!profile.api_nodes.enabled);
    assert!(!profile.custom_nodes.enabled);
    assert_eq!(
        profile.custom_nodes.directory,
        Some(PathBuf::from("/sim/custom"))
    );
    assert!(profile.manager.enabled);
    assert_eq!(profile.manager.mode, "manual");
    assert_eq!(
        profile
            .feature_flags
            .get("preview_metadata")
            .map(String::as_str),
        Some("true")
    );
    assert!(profile.compression.response_compression);
}

#[test]
fn launch_profile_parser_maps_runtime_options_to_policy_request() {
    let profile = SimLaunchProfileParser::new().parse_args([
        "--cuda",
        "--fp16",
        "--lowvram",
        "--multi-gpu",
        "--async-offload",
        "--pin-shared-memory",
        "--mmap-weights",
        "--free-memory-before-load",
        "--allow-downloads",
        "--dependency-reviewed",
        "--cache-lru",
        "--attention-backend",
        "sage",
    ]);

    assert_eq!(profile.runtime_policy.device, DeviceBackend::Cuda);
    assert_eq!(profile.runtime_policy.precision, PrecisionPolicy::Fp16);
    assert_eq!(profile.runtime_policy.memory, MemoryMode::LowVram);
    assert!(profile.runtime_policy.multi_gpu);
    assert!(profile.runtime_policy.async_offload);
    assert!(profile.runtime_policy.pinned_memory);
    assert!(profile.runtime_policy.mmap_weights);
    assert!(profile.runtime_policy.release_cache_before_load);
    assert!(profile.runtime_policy.allow_downloads);
    assert!(profile.runtime_policy.dependency_reviewed);
    assert_eq!(profile.cache.mode, "lru");
    assert_eq!(
        profile.performance.attention_backend.as_deref(),
        Some("sage")
    );
}

#[test]
fn launch_profile_parser_accumulates_invalid_values() {
    let profile = SimLaunchProfileParser::new().parse_args([
        "--port",
        "not-a-port",
        "--max-upload-size",
        "large",
        "--feature",
        "missing_separator",
        "--tls-keyfile",
    ]);

    assert!(!profile.is_valid());
    assert_eq!(profile.diagnostics.len(), 4);
    assert!(
        profile
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == LAUNCH_PROFILE_INVALID_OPTION_CODE)
    );
    assert!(
        profile
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == SimLaunchDiagnosticSeverity::Error)
    );
}

#[test]
fn launch_profile_parser_reports_unsupported_options_with_equivalents() {
    let profile = SimLaunchProfileParser::new().parse_args([
        "--windows-standalone-build",
        "--preview-method",
        "auto",
        "--dont-print-server",
        "--unknown-comfy-option",
    ]);

    assert!(profile.is_valid());
    assert_eq!(profile.diagnostics.len(), 4);
    assert!(
        profile
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == LAUNCH_PROFILE_UNSUPPORTED_OPTION_CODE)
    );
    assert!(
        profile
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == SimLaunchDiagnosticSeverity::Warning)
    );
    assert_eq!(
        profile.diagnostics[0].nearest_sim_equivalent.as_deref(),
        Some("Sim packaging profile catalog")
    );
    assert_eq!(
        profile.diagnostics[1].nearest_sim_equivalent.as_deref(),
        Some("Sim media preview service")
    );
    assert_eq!(profile.diagnostics[3].nearest_sim_equivalent, None);
}
