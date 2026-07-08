use std::path::Path;

use crate::{
    ASSET_SEED_MISSING_ROOT_CODE, SimAssetApi, SimAssetCacheState, SimAssetOwnerId,
    SimAssetScanRoot, SimAssetScanRootKind, SimAssetScannedFile, SimAssetSeedState, SimAssetSeeder,
    SimAssetUploadRequest,
};

fn roots() -> Vec<SimAssetScanRoot> {
    vec![
        SimAssetScanRoot::new(SimAssetScanRootKind::Models, "models"),
        SimAssetScanRoot::new(SimAssetScanRootKind::Input, "input"),
        SimAssetScanRoot::new(SimAssetScanRootKind::Output, "output"),
    ]
}

fn sha256(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

#[test]
fn seeder_registers_model_input_and_output_files_as_native_assets() {
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let files = vec![
        SimAssetScannedFile::new(SimAssetScanRootKind::Models, "sd/model.safetensors", 42)
            .with_hash(sha256('a'))
            .with_mime_type("application/octet-stream"),
        SimAssetScannedFile::new(SimAssetScanRootKind::Input, "prompt.png", 12)
            .with_hash(sha256('b'))
            .with_mime_type("image/png"),
        SimAssetScannedFile::new(SimAssetScanRootKind::Output, "run/out.png", 24)
            .with_hash(sha256('c'))
            .with_mime_type("image/png")
            .with_modified_at_ms(7),
    ];

    let report = SimAssetSeeder::new(&mut api, owner.clone(), roots())
        .seed(&files, None)
        .expect("seed");

    assert_eq!(report.progress.state, SimAssetSeedState::Completed);
    assert_eq!(report.progress.scanned, 3);
    assert_eq!(report.progress.created, 3);
    assert_eq!(api.repository().reference_len(), 3);
    let output = api
        .repository()
        .references_for_owner(&owner)
        .into_iter()
        .find(|reference| reference.name == "out.png")
        .expect("output reference");
    assert_eq!(
        output.cache_state.file_path.as_deref(),
        Some(Path::new("output/run/out.png"))
    );
    assert_eq!(output.cache_state.modified_at_ms, Some(7));
}

#[test]
fn seeder_skips_existing_paths_and_reports_missing_roots() {
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let files = vec![
        SimAssetScannedFile::new(SimAssetScanRootKind::Output, "run/out.png", 24)
            .with_hash(sha256('d')),
        SimAssetScannedFile::new(SimAssetScanRootKind::Input, "input.png", 12)
            .with_hash(sha256('e')),
    ];
    let partial_roots = vec![SimAssetScanRoot::new(
        SimAssetScanRootKind::Output,
        "output",
    )];

    let first = SimAssetSeeder::new(&mut api, owner.clone(), partial_roots.clone())
        .seed(&files, None)
        .expect("first seed");
    let second = SimAssetSeeder::new(&mut api, owner, partial_roots)
        .seed(&files, None)
        .expect("second seed");

    assert_eq!(first.progress.created, 1);
    assert_eq!(first.progress.skipped, 1);
    assert_eq!(first.progress.errors[0].code, ASSET_SEED_MISSING_ROOT_CODE);
    assert_eq!(second.progress.created, 0);
    assert_eq!(second.progress.skipped, 2);
}

#[test]
fn seeder_supports_cancellation_without_discarding_created_assets() {
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let files = vec![
        SimAssetScannedFile::new(SimAssetScanRootKind::Output, "a.png", 1),
        SimAssetScannedFile::new(SimAssetScanRootKind::Output, "b.png", 1),
    ];

    let report = SimAssetSeeder::new(&mut api, owner, roots())
        .seed(&files, Some(1))
        .expect("seed");

    assert_eq!(report.progress.state, SimAssetSeedState::Cancelled);
    assert_eq!(report.progress.scanned, 1);
    assert_eq!(report.progress.created, 1);
    assert_eq!(api.repository().reference_len(), 1);
}

#[test]
fn prune_marks_references_outside_known_roots_missing_without_deleting_content() {
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let stale = api
        .upload(
            owner.clone(),
            SimAssetUploadRequest::new("stale.png", 10)
                .expect("upload")
                .with_cache_state(
                    SimAssetCacheState::default().with_file_path("old-output/stale.png"),
                ),
        )
        .expect("upload");
    api.upload(
        owner.clone(),
        SimAssetUploadRequest::new("fresh.png", 10)
            .expect("upload")
            .with_cache_state(SimAssetCacheState::default().with_file_path("output/fresh.png")),
    )
    .expect("upload");

    let pruned = SimAssetSeeder::new(&mut api, owner.clone(), roots())
        .prune_missing_outside_roots()
        .expect("prune");

    assert_eq!(pruned, 1);
    assert_eq!(api.repository().content_len(), 2);
    assert!(
        api.detail(&owner, &stale.reference.id)
            .expect("detail")
            .expect("visible")
            .reference
            .cache_state
            .is_missing
    );
}
