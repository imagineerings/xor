use crate::{
    DEPENDENCY_REVIEW_AUDIT_MISSING_CODE, DEPENDENCY_REVIEW_INCOMPLETE_CODE,
    DEPENDENCY_REVIEW_MISSING_CODE, DEPENDENCY_REVIEW_NOT_APPROVED_CODE, SimDependencyAuditKind,
    SimDependencyAuditRecord, SimDependencyKind, SimDependencyProposal, SimDependencyReviewGate,
    SimDependencyReviewRecord, SimDependencyReviewStatus,
};

#[test]
fn review_gate_blocks_governed_dependency_without_review() {
    let report = SimDependencyReviewGate::new().evaluate([SimDependencyProposal::new(
        "native-video-codec",
        "Native video codec",
        SimDependencyKind::Codec,
    )]);

    assert!(!report.is_allowed());
    let diagnostics = report.diagnostics().collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, DEPENDENCY_REVIEW_MISSING_CODE);
}

#[test]
fn review_gate_allows_approved_native_sim_review_records() {
    let proposal = SimDependencyProposal::new(
        "provider-sdk",
        "Provider SDK",
        SimDependencyKind::ProviderSdk,
    );
    let report = SimDependencyReviewGate::new()
        .with_review(complete_review("provider-sdk"))
        .evaluate([proposal]);

    assert!(report.is_allowed());
    assert_eq!(report.diagnostics().count(), 0);
}

#[test]
fn review_gate_reports_incomplete_or_unapproved_metadata() {
    let incomplete = SimDependencyReviewRecord::approved(
        "python-package",
        "platform-review",
        "2026-07-08T19:10:00Z",
    )
    .with_license("MIT")
    .with_status(SimDependencyReviewStatus::Pending);

    let report = SimDependencyReviewGate::new()
        .with_review(incomplete)
        .evaluate([SimDependencyProposal::new(
            "python-package",
            "Python package",
            SimDependencyKind::PythonPackage,
        )]);

    let codes = report
        .diagnostics()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        vec![
            DEPENDENCY_REVIEW_INCOMPLETE_CODE,
            DEPENDENCY_REVIEW_NOT_APPROVED_CODE
        ]
    );
}

#[test]
fn review_gate_requires_network_and_large_download_audit_records() {
    let proposal = SimDependencyProposal::new(
        "model-weights",
        "Model weights",
        SimDependencyKind::ModelDependency,
    )
    .with_network_access(true)
    .with_estimated_download_bytes(512 * 1024 * 1024);

    let report = SimDependencyReviewGate::new()
        .with_review(complete_review("model-weights"))
        .evaluate([proposal.clone()]);

    let codes = report
        .diagnostics()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        vec![
            DEPENDENCY_REVIEW_AUDIT_MISSING_CODE,
            DEPENDENCY_REVIEW_AUDIT_MISSING_CODE
        ]
    );

    let approved_report = SimDependencyReviewGate::new()
        .with_review(complete_review("model-weights"))
        .with_audit_record(SimDependencyAuditRecord::new(
            "model-weights",
            SimDependencyAuditKind::NetworkAccess,
            "user",
            "2026-07-08T19:11:00Z",
            "approved model download host",
        ))
        .with_audit_record(SimDependencyAuditRecord::new(
            "model-weights",
            SimDependencyAuditKind::LargeDownload,
            "user",
            "2026-07-08T19:11:00Z",
            "approved large model download",
        ))
        .evaluate([proposal]);

    assert!(approved_report.is_allowed());
}

#[test]
fn review_gate_enforces_all_comfy_migration_dependency_categories() {
    let categories = [
        SimDependencyKind::NativeLibrary,
        SimDependencyKind::Codec,
        SimDependencyKind::PythonPackage,
        SimDependencyKind::ProviderSdk,
        SimDependencyKind::ModelDependency,
        SimDependencyKind::FrontendPackage,
        SimDependencyKind::VendoredCode,
        SimDependencyKind::NetworkAccess,
        SimDependencyKind::LargeDownload,
    ];

    for kind in categories {
        assert!(kind.requires_review(), "{kind:?} must require review");
    }
}

fn complete_review(dependency_id: &str) -> SimDependencyReviewRecord {
    SimDependencyReviewRecord::approved(dependency_id, "platform-review", "2026-07-08T19:10:00Z")
        .with_license("license acceptable")
        .with_maintenance("maintained")
        .with_security("no known critical advisories")
        .with_binary_size("acceptable package size")
        .with_platform_impact("supported target platforms documented")
        .with_runtime_impact("runtime cost documented")
        .with_fallback_strategy("feature remains disabled without dependency")
}
