use serde::{Deserialize, Serialize};

use crate::WorldModelProfile;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorldModelSessionState {
    Starting,
    Ready,
    Busy,
    Failed,
    Closed,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldModelCacheMetadata {
    pub model_loaded: bool,
    pub cache_key: Option<String>,
    pub cached_frame_count: u64,
    pub warmed_at_epoch_ms: Option<u64>,
}

impl WorldModelCacheMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cache_key(mut self, cache_key: impl Into<String>) -> Self {
        self.cache_key = Some(cache_key.into());
        self
    }

    pub fn with_cached_frame_count(mut self, cached_frame_count: u64) -> Self {
        self.cached_frame_count = cached_frame_count;
        self
    }

    pub fn mark_model_loaded(mut self, warmed_at_epoch_ms: u64) -> Self {
        self.model_loaded = true;
        self.warmed_at_epoch_ms = Some(warmed_at_epoch_ms);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldModelSession {
    pub session_id: String,
    pub worker_id: String,
    pub model_profile: WorldModelProfile,
    pub state: WorldModelSessionState,
    pub fast_inference_enabled: bool,
    pub cache_metadata: WorldModelCacheMetadata,
}

impl WorldModelSession {
    pub fn new(
        session_id: impl Into<String>,
        worker_id: impl Into<String>,
        model_profile: WorldModelProfile,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            worker_id: worker_id.into(),
            model_profile,
            state: WorldModelSessionState::Starting,
            fast_inference_enabled: false,
            cache_metadata: WorldModelCacheMetadata::new(),
        }
    }

    pub fn with_state(mut self, state: WorldModelSessionState) -> Self {
        self.state = state;
        self
    }

    pub fn with_fast_inference(mut self, cache_metadata: WorldModelCacheMetadata) -> Self {
        self.fast_inference_enabled = true;
        self.cache_metadata = cache_metadata;
        self
    }

    pub fn is_reusable(&self) -> bool {
        self.fast_inference_enabled
            && self.state == WorldModelSessionState::Ready
            && self.cache_metadata.model_loaded
    }
}
