use serde_json::json;

use crate::{
    ASSET_CONTENT_NOT_FOUND_CODE, ASSET_REFERENCE_NOT_FOUND_CODE, SimAssetCacheState,
    SimAssetContentId, SimAssetCoverageCatalog, SimAssetHash, SimAssetOwnerId, SimAssetReferenceId,
    SimAssetReferenceRequest, SimAssetRepository,
};

const ASSET_LIBRARY_BACKLOG: &str = include_str!("../fixtures/comfy/asset_library_backlog.json");

#[test]
fn asset_library_backlog_fixture_maps_to_native_sim_output_registration() {
    let fixture: SimAssetCoverageCatalog =
        serde_json::from_str(ASSET_LIBRARY_BACKLOG).expect("asset backlog fixture parses");
    fixture
        .validate()
        .expect("asset backlog fixture should be internally valid");

    assert_eq!(fixture.records.len(), 1);
    assert_eq!(fixture.surfaces().len(), 1);
    assert!(fixture.surfaces().contains("asset-output-registration"));

    let record = &fixture.records[0];
    assert_eq!(record.node_name, "ComboOutputTestNode");
    assert_eq!(
        record.evidence_module,
        "crates/world_model/src/sim_asset_enrichment.rs"
    );
    assert!(record.metadata_only);
}

#[test]
fn asset_repository_reuses_content_by_hash_and_preserves_reference_metadata() {
    let mut repository = SimAssetRepository::default();
    let owner = SimAssetOwnerId::new("user-a");

    let first = repository.create_reference(
        owner.clone(),
        SimAssetReferenceRequest::new("first.png", 1024)
            .with_hash("sha256:abc")
            .with_mime_type("image/png")
            .with_tag("generated")
            .with_user_metadata("prompt", json!("a castle")),
    );
    let second = repository.create_reference(
        owner,
        SimAssetReferenceRequest::new("second.png", 1024)
            .with_hash("sha256:abc")
            .with_mime_type("image/png")
            .with_tag("favorite")
            .with_user_metadata("prompt", json!("a forest")),
    );

    assert_eq!(repository.content_len(), 1);
    assert_eq!(repository.reference_len(), 2);
    assert_eq!(first.content_id, second.content_id);
    assert_ne!(first.id, second.id);
    assert!(first.tags.contains("generated"));
    assert!(second.tags.contains("favorite"));
    assert_eq!(first.user_metadata["prompt"], "a castle");
    assert_eq!(second.user_metadata["prompt"], "a forest");

    let content = repository
        .content_by_hash(&SimAssetHash::new("sha256:abc"))
        .expect("content should exist");
    assert_eq!(content.size_bytes, 1024);
    assert_eq!(content.mime_type.as_deref(), Some("image/png"));
}

#[test]
fn asset_repository_scopes_live_references_by_owner() {
    let mut repository = SimAssetRepository::default();
    let user_a = SimAssetOwnerId::new("user-a");
    let user_b = SimAssetOwnerId::new("user-b");

    repository.create_reference(
        user_a.clone(),
        SimAssetReferenceRequest::new("a.png", 12).with_hash("sha256:a"),
    );
    repository.create_reference(
        user_b.clone(),
        SimAssetReferenceRequest::new("b.png", 12).with_hash("sha256:b"),
    );

    let user_a_refs = repository.references_for_owner(&user_a);
    let user_b_refs = repository.references_for_owner(&user_b);

    assert_eq!(user_a_refs.len(), 1);
    assert_eq!(user_a_refs[0].owner_id, user_a);
    assert_eq!(user_a_refs[0].name, "a.png");
    assert_eq!(user_b_refs.len(), 1);
    assert_eq!(user_b_refs[0].owner_id, user_b);
}

#[test]
fn asset_repository_soft_delete_preserves_shared_content_and_other_owners() {
    let mut repository = SimAssetRepository::default();
    let user_a = SimAssetOwnerId::new("user-a");
    let user_b = SimAssetOwnerId::new("user-b");
    let first = repository.create_reference(
        user_a.clone(),
        SimAssetReferenceRequest::new("a.png", 12).with_hash("sha256:shared"),
    );
    let second = repository.create_reference(
        user_b.clone(),
        SimAssetReferenceRequest::new("b.png", 12).with_hash("sha256:shared"),
    );

    assert!(
        !repository
            .soft_delete_reference(&user_b, &first.id)
            .expect("wrong-owner delete should not fail")
    );
    assert!(
        repository
            .soft_delete_reference(&user_a, &first.id)
            .expect("owner delete should succeed")
    );

    assert_eq!(repository.content_len(), 1);
    assert!(repository.reference(&first.id).expect("first").is_deleted());
    assert!(
        !repository
            .reference(&second.id)
            .expect("second")
            .is_deleted()
    );
    assert!(repository.references_for_owner(&user_a).is_empty());
    assert_eq!(repository.references_for_owner(&user_b).len(), 1);
}

#[test]
fn asset_reference_records_cache_state_preview_job_and_provenance() {
    let mut repository = SimAssetRepository::default();
    let owner = SimAssetOwnerId::new("user-a");
    let preview_id = SimAssetReferenceId::new("asset-reference-preview");
    let cache_state = SimAssetCacheState::default()
        .with_file_path("outputs/castle.png")
        .with_modified_at_ms(42)
        .with_enrichment_level(2)
        .verified();

    let reference = repository.create_reference(
        owner,
        SimAssetReferenceRequest::new("castle.png", 2048)
            .with_hash("sha256:castle")
            .with_preview_id(preview_id.clone())
            .with_job_id("job-1")
            .with_provenance_id("artifact:castle")
            .with_cache_state(cache_state.clone())
            .with_system_metadata("width", json!(1024)),
    );

    assert_eq!(reference.preview_id, Some(preview_id));
    assert_eq!(reference.job_id.as_deref(), Some("job-1"));
    assert_eq!(reference.provenance_id.as_deref(), Some("artifact:castle"));
    assert_eq!(reference.cache_state, cache_state);
    assert_eq!(reference.system_metadata["width"], 1024);
    assert!(reference.created_at_ms > 0);
    assert_eq!(reference.created_at_ms, reference.updated_at_ms);
}

#[test]
fn asset_repository_reports_missing_content_and_references() {
    let repository = SimAssetRepository::default();

    let content_error = repository
        .content(&SimAssetContentId::new("missing-content"))
        .expect_err("missing content should fail");
    let reference_error = repository
        .reference(&SimAssetReferenceId::new("missing-reference"))
        .expect_err("missing reference should fail");

    assert_eq!(content_error.code, ASSET_CONTENT_NOT_FOUND_CODE);
    assert_eq!(reference_error.code, ASSET_REFERENCE_NOT_FOUND_CODE);
}
