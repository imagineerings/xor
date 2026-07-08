use crate::{
    WorldModelCacheMetadata, WorldModelProfile, WorldModelSession, WorldModelSessionState,
};

#[test]
fn session_models_persistent_fast_inference_cache() {
    let profile = WorldModelProfile::new("lingbot-video", "lingbot");
    let cache_metadata = WorldModelCacheMetadata::new()
        .with_cache_key("lingbot:seed:42")
        .with_cached_frame_count(24)
        .mark_model_loaded(1_700_000);
    let session = WorldModelSession::new("session-1", "worker-1", profile)
        .with_state(WorldModelSessionState::Ready)
        .with_fast_inference(cache_metadata);

    assert!(session.fast_inference_enabled);
    assert!(session.is_reusable());
    assert_eq!(session.cache_metadata.cached_frame_count, 24);
    assert_eq!(
        session.cache_metadata.cache_key.as_deref(),
        Some("lingbot:seed:42")
    );
}

#[test]
fn session_is_not_reusable_until_ready_and_loaded() {
    let profile = WorldModelProfile::new("wan-video", "wan");
    let session = WorldModelSession::new("session-1", "worker-1", profile)
        .with_state(WorldModelSessionState::Busy)
        .with_fast_inference(WorldModelCacheMetadata::new());

    assert!(!session.is_reusable());
}
