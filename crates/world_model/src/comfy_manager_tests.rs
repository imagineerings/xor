use std::path::PathBuf;

use crate::{
    DEPENDENCY_REVIEW_MISSING_CODE, SIM_MANAGER_ACTION_DENIED_CODE,
    SIM_MANAGER_APPROVAL_REQUIRED_CODE, SIM_MANAGER_BACKGROUND_DENIED_CODE,
    SIM_MANAGER_DEPENDENCY_REVIEW_DENIED_CODE, SimDependencyAuditKind, SimDependencyAuditRecord,
    SimDependencyKind, SimDependencyProposal, SimDependencyReviewGate, SimDependencyReviewRecord,
    SimExtensionId, SimExtensionPolicy, SimExtensionRecord, SimExtensionSourceKind,
    SimManagerActionKind, SimManagerActionRequest, SimManagerApproval, SimManagerBoundary,
};

#[test]
fn manager_boundary_exposes_status_without_enabling_routes() {
    let status = SimManagerBoundary::new()
        .with_managed_extension(SimExtensionId::new("pack-a"))
        .status();

    assert!(!status.manager_routes_enabled);
    assert!(!status.background_operations_enabled);
    assert_eq!(status.managed_extensions[0].as_str(), "pack-a");
}

#[test]
fn manager_boundary_requires_routes_and_explicit_approval_for_writes() {
    let evaluation = SimManagerBoundary::new().evaluate(SimManagerActionRequest::new(
        SimManagerActionKind::Disable,
        extension("pack-a"),
    ));

    assert!(!evaluation.allowed);
    assert!(evaluation.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_MANAGER_ACTION_DENIED_CODE
            && diagnostic.action == SimManagerActionKind::Disable
    }));
    assert!(evaluation.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_MANAGER_APPROVAL_REQUIRED_CODE
            && diagnostic.action == SimManagerActionKind::Disable
    }));
}

#[test]
fn manager_boundary_gates_install_update_with_policy_and_dependency_review() {
    let dependency = SimDependencyProposal::new(
        "pack-a-python",
        "pack-a-python",
        SimDependencyKind::PythonPackage,
    )
    .with_network_access(true);
    let denied = SimManagerBoundary::new()
        .with_manager_routes_enabled(true)
        .with_policy(
            SimExtensionPolicy::default()
                .with_install_allowed("pack-a")
                .with_dependency_reviewed_install("pack-a")
                .with_network_allowed("pack-a"),
        )
        .evaluate(
            SimManagerActionRequest::new(SimManagerActionKind::Install, extension("pack-a"))
                .with_network(true)
                .with_dependency(dependency.clone())
                .with_approval(approval()),
        );

    assert!(!denied.allowed);
    assert!(
        denied
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == SIM_MANAGER_DEPENDENCY_REVIEW_DENIED_CODE })
    );
    assert!(denied.dependency_diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DEPENDENCY_REVIEW_MISSING_CODE
            && diagnostic.dependency_id == "pack-a-python"
    }));

    let review = SimDependencyReviewRecord::approved("pack-a-python", "maintainer", "2026-07-08")
        .with_license("MIT")
        .with_maintenance("maintained")
        .with_security("no advisories")
        .with_binary_size("small")
        .with_platform_impact("none")
        .with_runtime_impact("isolated")
        .with_fallback_strategy("disable extension");
    let allowed = SimManagerBoundary::new()
        .with_manager_routes_enabled(true)
        .with_policy(
            SimExtensionPolicy::default()
                .with_install_allowed("pack-a")
                .with_dependency_reviewed_install("pack-a")
                .with_network_allowed("pack-a"),
        )
        .with_dependency_review_gate(
            SimDependencyReviewGate::new()
                .with_review(review)
                .with_audit_record(SimDependencyAuditRecord::new(
                    "pack-a-python",
                    SimDependencyAuditKind::NetworkAccess,
                    "maintainer",
                    "2026-07-08",
                    "manager install request approved",
                )),
        )
        .evaluate(
            SimManagerActionRequest::new(SimManagerActionKind::Install, extension("pack-a"))
                .with_network(true)
                .with_dependency(dependency)
                .with_approval(approval()),
        );

    assert!(allowed.allowed);
    assert!(allowed.diagnostics.is_empty());
    assert!(allowed.dependency_diagnostics.is_empty());
}

#[test]
fn manager_boundary_allows_background_operations_only_when_policy_permits() {
    let denied = SimManagerBoundary::new()
        .with_manager_routes_enabled(true)
        .evaluate(SimManagerActionRequest::new(
            SimManagerActionKind::BackgroundOperation,
            extension("pack-a"),
        ));
    assert!(!denied.allowed);
    assert!(
        denied
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == SIM_MANAGER_BACKGROUND_DENIED_CODE })
    );

    let allowed = SimManagerBoundary::new()
        .with_manager_routes_enabled(true)
        .with_background_operations_enabled(true)
        .evaluate(SimManagerActionRequest::new(
            SimManagerActionKind::BackgroundOperation,
            extension("pack-a"),
        ));
    assert!(allowed.allowed);
}

fn approval() -> SimManagerApproval {
    SimManagerApproval::new("maintainer", "2026-07-08", "user approved manager action")
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
