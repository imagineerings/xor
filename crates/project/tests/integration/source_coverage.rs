use std::{path::Path, sync::Arc};

use project::{
    ProjectPath,
    rust_coverage_provider::{MAX_RUST_COVERAGE_ARTIFACT_BYTES, RustCoverageArtifactProvider},
    source_coverage::{
        SourceCoverageFile, SourceCoveragePoint, SourceCoverageProviderId, SourceCoverageRange,
        SourceCoverageSnapshot, SourceCoverageState, SourceCoverageStatus, snapshot_from_proto,
        snapshot_to_proto,
    },
};
use settings::WorktreeId;
use util::rel_path::RelPath;

fn project_path(path: &str, worktree: usize) -> ProjectPath {
    ProjectPath {
        worktree_id: WorktreeId::from_usize(worktree),
        path: Arc::from(RelPath::from_unix_str(path).expect("fixture path should be relative")),
    }
}

#[test]
fn source_coverage_acceptance_covers_rust_non_rust_privacy_and_failures() {
    let mut rust_provider = RustCoverageArtifactProvider::new(11);
    let token = rust_provider.begin_run();
    let rust_report = include_bytes!("../../test_data/source_coverage/rust-report.json");
    let rust_snapshot = rust_provider
        .parse_artifact(token, rust_report, |absolute_path| {
            let relative = absolute_path.strip_prefix(Path::new("/workspace")).ok()?;
            Some(project_path(relative.to_str()?, 1))
        })
        .expect("supported cargo-llvm-cov export should parse");
    assert_eq!(rust_snapshot.files.len(), 1);
    assert_eq!(rust_snapshot.files[0].path, project_path("src/lib.rs", 1));
    assert_eq!(rust_snapshot.status, SourceCoverageStatus::Partial);

    let non_rust = SourceCoverageSnapshot {
        project_generation: 11,
        provider_id: SourceCoverageProviderId("fake-typescript".to_string()),
        generation: 1,
        status: SourceCoverageStatus::Current,
        files: vec![
            SourceCoverageFile {
                path: project_path("src/app.ts", 1),
                ranges: vec![SourceCoverageRange {
                    start: SourceCoveragePoint { line: 0, column: 0 },
                    end: SourceCoveragePoint { line: 0, column: 8 },
                    hit_count: 2,
                }],
                covered_lines: 1,
                uncovered_lines: 0,
                truncated: false,
            },
            SourceCoverageFile {
                path: project_path("hidden/secret.ts", 2),
                ranges: Vec::new(),
                covered_lines: 0,
                uncovered_lines: 1,
                truncated: false,
            },
        ],
        truncated: false,
        diagnostic: None,
    };
    let mut state = SourceCoverageState::new(11);
    state
        .publish(non_rust.clone())
        .expect("language-neutral provider should publish");
    let visible = snapshot_to_proto(
        &non_rust,
        Some(&[WorktreeId::from_usize(1)].into_iter().collect()),
    );
    assert_eq!(visible.files.len(), 1);
    assert!(visible.truncated);
    let round_trip = snapshot_from_proto(visible).expect("bounded remote facts should decode");
    assert_eq!(round_trip.files[0].path, project_path("src/app.ts", 1));
    assert!(!format!("{round_trip:?}").contains("/workspace"));

    rust_provider.cancel(token);
    assert!(
        rust_provider
            .parse_artifact(token, rust_report, |_| None)
            .is_err()
    );
    let next_token = rust_provider.begin_run();
    assert!(
        rust_provider
            .parse_artifact(
                next_token,
                &vec![b' '; MAX_RUST_COVERAGE_ARTIFACT_BYTES + 1],
                |_| None,
            )
            .is_err()
    );
}
