use crate::{
    DependencyReviewGate, SimGameDependencyKind, SimGameDependencyProposal,
    SimGameDependencyReviewRecord, SimGameDependencyReviewStatus,
};

#[test]
fn dependency_review_blocks_unreviewed_native_dependency() {
    let report = DependencyReviewGate::new().evaluate([SimGameDependencyProposal::new(
        "native-navmesh",
        "Native navigation mesh solver",
        SimGameDependencyKind::NativeLibrary,
    )]);

    assert!(!report.is_allowed());
    let diagnostics = report.diagnostics().collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].field, "review");
}

#[test]
fn dependency_review_allows_complete_approved_record() {
    let review = complete_review("mesh-codec");
    let report =
        DependencyReviewGate::new()
            .with_review(review)
            .evaluate([SimGameDependencyProposal::new(
                "mesh-codec",
                "Mesh codec",
                SimGameDependencyKind::Mesh,
            )]);

    assert!(report.is_allowed());
}

#[test]
fn dependency_review_requires_required_metadata_and_approval() {
    let incomplete = SimGameDependencyReviewRecord::approved(
        "media-runtime",
        "platform-review",
        "2026-07-09T12:00:00Z",
    )
    .with_license("acceptable")
    .with_status(SimGameDependencyReviewStatus::Pending);

    let report = DependencyReviewGate::new()
        .with_review(incomplete)
        .evaluate([SimGameDependencyProposal::new(
            "media-runtime",
            "Media runtime",
            SimGameDependencyKind::MediaRuntime,
        )]);

    let fields = report
        .diagnostics()
        .map(|diagnostic| diagnostic.field)
        .collect::<Vec<_>>();
    assert_eq!(fields, vec!["review.metadata", "review.status"]);
}

fn complete_review(dependency_id: &str) -> SimGameDependencyReviewRecord {
    SimGameDependencyReviewRecord::approved(
        dependency_id,
        "platform-review",
        "2026-07-09T12:00:00Z",
    )
    .with_license("license acceptable")
    .with_maintenance("maintained")
    .with_security("no known critical advisories")
    .with_binary_size("acceptable package size")
    .with_platform_impact("supported target platforms documented")
}
