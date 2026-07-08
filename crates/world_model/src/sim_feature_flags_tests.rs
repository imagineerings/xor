use crate::{
    ASSETS_FLAG, FEATURE_FLAG_CORE_OVERRIDE_CODE, FEATURE_FLAG_INVALID_VALUE_CODE,
    FEATURE_PACKAGE_MISSING_CODE, FEATURE_PACKAGE_OUTDATED_CODE, MANAGER_SUPPORT_FLAG,
    NODE_REPLACEMENTS_FLAG, PREVIEW_METADATA_FLAG, SimFeatureFlagDiagnosticSeverity,
    SimFeatureFlagRegistry, SimFeatureFlags, SimPackageKind, SimPackageRequirement,
    UPLOAD_SIZE_FLAG,
};

#[test]
fn feature_registry_exposes_core_and_cli_server_flags() {
    let mut registry = SimFeatureFlagRegistry::new();
    registry.apply_cli_flag("custom_preview", "on").unwrap();
    registry.apply_cli_flag("experimental", "0").unwrap();

    let features = registry.server_features();
    assert!(features.enabled(PREVIEW_METADATA_FLAG));
    assert!(features.enabled(UPLOAD_SIZE_FLAG));
    assert!(!features.enabled(MANAGER_SUPPORT_FLAG));
    assert!(features.enabled(NODE_REPLACEMENTS_FLAG));
    assert!(features.enabled(ASSETS_FLAG));
    assert!(features.enabled("custom_preview"));
    assert!(!features.enabled("experimental"));
}

#[test]
fn feature_registry_protects_core_flags_from_cli_overrides() {
    let mut registry = SimFeatureFlagRegistry::new();
    let diagnostic = registry
        .apply_cli_flag(PREVIEW_METADATA_FLAG, "false")
        .unwrap_err();

    assert_eq!(diagnostic.code, FEATURE_FLAG_CORE_OVERRIDE_CODE);
    assert_eq!(
        diagnostic.severity,
        SimFeatureFlagDiagnosticSeverity::Warning
    );
    assert!(registry.server_features().enabled(PREVIEW_METADATA_FLAG));
}

#[test]
fn feature_registry_rejects_invalid_typed_flag_values() {
    let mut registry = SimFeatureFlagRegistry::new();
    let diagnostic = registry.apply_cli_flag("custom", "maybe").unwrap_err();

    assert_eq!(diagnostic.code, FEATURE_FLAG_INVALID_VALUE_CODE);
    assert_eq!(diagnostic.severity, SimFeatureFlagDiagnosticSeverity::Error);
    assert!(!registry.server_features().enabled("custom"));
}

#[test]
fn feature_registry_stores_connection_specific_negotiation() {
    let mut registry = SimFeatureFlagRegistry::new();
    registry.apply_cli_flag("custom_preview", "true").unwrap();
    let requested = SimFeatureFlags::default()
        .with_flag(PREVIEW_METADATA_FLAG, true)
        .with_flag("custom_preview", true)
        .with_flag("unknown_client_flag", true)
        .with_flag(ASSETS_FLAG, false);

    let negotiation = registry.negotiate_client_features("client-a", requested.clone());

    assert_eq!(negotiation.client_id, "client-a");
    assert_eq!(negotiation.requested, requested);
    assert!(negotiation.accepted.enabled(PREVIEW_METADATA_FLAG));
    assert!(negotiation.accepted.enabled("custom_preview"));
    assert!(!negotiation.accepted.enabled("unknown_client_flag"));
    assert!(!negotiation.accepted.enabled(ASSETS_FLAG));
    assert_eq!(
        registry.client_features("client-a"),
        Some(&negotiation.accepted)
    );
}

#[test]
fn feature_registry_reports_missing_and_outdated_packages() {
    let registry = SimFeatureFlagRegistry::new();
    let diagnostics = registry.diagnose_packages([
        SimPackageRequirement::new("web", SimPackageKind::Frontend, "1.2.0"),
        SimPackageRequirement::new("templates", SimPackageKind::WorkflowTemplates, "2.0.0")
            .with_installed_version("1.9.9"),
        SimPackageRequirement::new("docs", SimPackageKind::EmbeddedDocs, "3.0.0")
            .with_installed_version("3.0.0"),
    ]);

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].code, FEATURE_PACKAGE_MISSING_CODE);
    assert_eq!(diagnostics[0].name, "web");
    assert_eq!(diagnostics[1].code, FEATURE_PACKAGE_OUTDATED_CODE);
    assert_eq!(diagnostics[1].name, "templates");
}
