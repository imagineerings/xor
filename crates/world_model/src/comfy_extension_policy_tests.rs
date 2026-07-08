use std::path::PathBuf;

use crate::{
    SIM_EXTENSION_POLICY_BLOCKED_CODE, SIM_EXTENSION_POLICY_DISABLED_CODE,
    SIM_EXTENSION_POLICY_INSTALL_REVIEW_REQUIRED_CODE, SIM_EXTENSION_POLICY_NETWORK_DENIED_CODE,
    SIM_EXTENSION_POLICY_SCRIPT_DENIED_CODE, SIM_EXTENSION_POLICY_WEB_ASSET_DENIED_CODE,
    SimExtensionId, SimExtensionPolicy, SimExtensionPolicyDecisionKind, SimExtensionPolicyRequest,
    SimExtensionRecord, SimExtensionSourceKind,
};

#[test]
fn extension_policy_disables_non_whitelisted_packs_when_custom_nodes_are_disabled() {
    let policy = SimExtensionPolicy::default()
        .with_custom_nodes_enabled(false)
        .with_whitelisted_pack("allowed_pack");

    let allowed = policy.evaluate(
        &record("allowed_pack"),
        &SimExtensionPolicyRequest::metadata_only(),
    );
    let denied = policy.evaluate(
        &record("other_pack"),
        &SimExtensionPolicyRequest::metadata_only(),
    );

    assert_eq!(
        allowed.decision,
        SimExtensionPolicyDecisionKind::Whitelisted
    );
    assert!(allowed.decision.allows_loading());
    assert!(allowed.diagnostics.is_empty());
    assert_eq!(denied.decision, SimExtensionPolicyDecisionKind::Disabled);
    assert!(denied.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_POLICY_DISABLED_CODE
            && diagnostic.extension_id.as_str() == "other-pack"
    }));
}

#[test]
fn extension_policy_blocked_pack_overrides_whitelist() {
    let policy = SimExtensionPolicy::default()
        .with_whitelisted_pack("unsafe_pack")
        .with_blocked_pack("unsafe_pack", "blocked by workspace policy");

    let evaluation = policy.evaluate(
        &record("unsafe_pack"),
        &SimExtensionPolicyRequest::metadata_only(),
    );

    assert_eq!(evaluation.decision, SimExtensionPolicyDecisionKind::Blocked);
    assert!(!evaluation.decision.allows_loading());
    assert!(evaluation.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_POLICY_BLOCKED_CODE
            && diagnostic.message == "blocked by workspace policy"
    }));
    assert!(!evaluation.permissions.script_allowed);
    assert!(!evaluation.permissions.web_assets_allowed);
}

#[test]
fn extension_policy_requires_explicit_script_web_and_network_permissions() {
    let request = SimExtensionPolicyRequest::metadata_only()
        .with_script(true)
        .with_web_assets(true)
        .with_network(true);

    let denied = SimExtensionPolicy::default().evaluate(&record("tools_pack"), &request);
    assert!(
        denied
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_EXTENSION_POLICY_SCRIPT_DENIED_CODE)
    );
    assert!(
        denied
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_EXTENSION_POLICY_WEB_ASSET_DENIED_CODE)
    );
    assert!(
        denied
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == SIM_EXTENSION_POLICY_NETWORK_DENIED_CODE)
    );

    let allowed = SimExtensionPolicy::default()
        .with_script_allowed("tools_pack")
        .with_web_assets_allowed("tools_pack")
        .with_network_allowed("tools_pack")
        .evaluate(&record("tools_pack"), &request);

    assert!(allowed.diagnostics.is_empty());
    assert!(allowed.permissions.script_allowed);
    assert!(allowed.permissions.web_assets_allowed);
    assert!(allowed.permissions.network_allowed);
}

#[test]
fn extension_policy_requires_dependency_review_for_install_permission() {
    let request = SimExtensionPolicyRequest::metadata_only().with_install(true);

    let missing_review = SimExtensionPolicy::default()
        .with_install_allowed("manager_pack")
        .evaluate(&record("manager_pack"), &request);
    assert!(!missing_review.permissions.install_allowed);
    assert!(missing_review.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_POLICY_INSTALL_REVIEW_REQUIRED_CODE
    }));

    let approved = SimExtensionPolicy::default()
        .with_install_allowed("manager_pack")
        .with_dependency_reviewed_install("manager_pack")
        .evaluate(&record("manager_pack"), &request);
    assert!(approved.permissions.install_allowed);
    assert!(approved.diagnostics.is_empty());
}

fn record(name: &str) -> SimExtensionRecord {
    SimExtensionRecord {
        id: SimExtensionId::new(name),
        display_name: name.to_string(),
        source_path: PathBuf::from(format!("/custom_nodes/{name}")),
        source_kind: SimExtensionSourceKind::Directory,
        root_index: 0,
        load_order: 0,
    }
}
