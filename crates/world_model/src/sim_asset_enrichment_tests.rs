use serde_json::json;

use crate::{
    SimAssetApi, SimAssetEnrichmentQueue, SimAssetOutputRegistrar,
    SimAssetOutputRegistrationRequest, SimAssetOwnerId,
};

fn sha256(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

#[test]
fn output_registration_attaches_job_provenance_hash_and_metadata() {
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let mut queue = SimAssetEnrichmentQueue::default();
    let detail = SimAssetOutputRegistrar::new(&mut api, &mut queue)
        .register_output(
            SimAssetOutputRegistrationRequest::new(owner, "castle.png", "output/castle.png", 2048)
                .with_hash(sha256('a'))
                .with_mime_type("image/png")
                .with_job_id("job-1")
                .with_provenance_id("artifact:castle")
                .with_extracted_metadata("width", json!(1024)),
        )
        .expect("register output");

    assert_eq!(detail.reference.job_id.as_deref(), Some("job-1"));
    assert_eq!(
        detail.reference.provenance_id.as_deref(),
        Some("artifact:castle")
    );
    assert_eq!(
        detail.content.hash.as_ref().map(|hash| hash.as_str()),
        Some(sha256('a').as_str())
    );
    assert_eq!(detail.reference.system_metadata["width"], 1024);
    assert_eq!(
        detail.reference.cache_state.file_path.as_deref(),
        Some("output/castle.png".as_ref())
    );
    assert_eq!(queue.pending_len(), 1);
}

#[test]
fn enrichment_queue_updates_system_metadata_and_enrichment_level() {
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let mut queue = SimAssetEnrichmentQueue::default();
    let detail = SimAssetOutputRegistrar::new(&mut api, &mut queue)
        .register_output(
            SimAssetOutputRegistrationRequest::new(
                owner.clone(),
                "castle.png",
                "output/castle.png",
                2048,
            )
            .with_mime_type("image/png")
            .with_extracted_metadata("height", json!(768)),
        )
        .expect("register output");

    let enriched = queue
        .process_next(&mut api)
        .expect("process")
        .expect("updated");

    assert_eq!(queue.pending_len(), 0);
    assert_eq!(enriched.reference.id, detail.reference.id);
    assert_eq!(enriched.reference.system_metadata["height"], 768);
    assert_eq!(enriched.reference.cache_state.enrichment_level, 1);
    assert!(enriched.reference.cache_state.verified);
    assert_eq!(
        api.detail(&owner, &detail.reference.id)
            .expect("detail")
            .expect("visible")
            .reference
            .cache_state
            .enrichment_level,
        1
    );
}

#[test]
fn output_registration_can_skip_enrichment_queue() {
    let owner = SimAssetOwnerId::new("user-a");
    let mut api = SimAssetApi::default();
    let mut queue = SimAssetEnrichmentQueue::default();
    SimAssetOutputRegistrar::new(&mut api, &mut queue)
        .register_output(
            SimAssetOutputRegistrationRequest::new(owner, "raw.bin", "output/raw.bin", 10)
                .without_enrichment(),
        )
        .expect("register output");

    assert_eq!(queue.pending_len(), 0);
}

#[test]
fn enrichment_queue_ignores_missing_or_inaccessible_records() {
    let mut api = SimAssetApi::default();
    let mut queue = SimAssetEnrichmentQueue::default();
    queue.push(crate::SimAssetEnrichmentJob {
        owner_id: SimAssetOwnerId::new("user-a"),
        reference_id: crate::SimAssetReferenceId::new("missing"),
        target_enrichment_level: 1,
        metadata: Default::default(),
    });

    assert!(queue.process_next(&mut api).expect("process").is_none());
    assert_eq!(queue.pending_len(), 0);
}
