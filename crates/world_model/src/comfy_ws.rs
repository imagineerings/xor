use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ClientFeatureNegotiation, ComfyExecutionEventTranslator, ComfyFeatureFlags, ComfyRuntimeEvent,
    ComfyWebSocketFrame, LEGACY_PREVIEW_FEATURE, PREVIEW_METADATA_FEATURE, QueueStatus,
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ComfyClientSessionId(String);

impl ComfyClientSessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn random() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyWebSocketSession {
    pub session_id: ComfyClientSessionId,
    pub requested_client_id: Option<String>,
    pub requested_features: ComfyFeatureFlags,
    pub accepted_features: ComfyFeatureFlags,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyWebSocketConnect {
    pub session: ComfyWebSocketSession,
    pub initial_frames: Vec<ComfyWebSocketFrame>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyWebSocketSessionRegistry {
    sessions: BTreeMap<ComfyClientSessionId, ComfyWebSocketSession>,
    server_features: ComfyFeatureFlags,
}

impl Default for ComfyWebSocketSessionRegistry {
    fn default() -> Self {
        Self {
            sessions: BTreeMap::new(),
            server_features: ComfyFeatureFlags::default()
                .with_flag(PREVIEW_METADATA_FEATURE, true)
                .with_flag(LEGACY_PREVIEW_FEATURE, true),
        }
    }
}

impl ComfyWebSocketSessionRegistry {
    pub fn connect(
        &mut self,
        requested_session_id: Option<ComfyClientSessionId>,
        requested_client_id: Option<String>,
        queue_status: QueueStatus,
    ) -> ComfyWebSocketConnect {
        let session_id = requested_session_id.unwrap_or_else(ComfyClientSessionId::random);
        let session = self
            .sessions
            .entry(session_id.clone())
            .and_modify(|session| {
                session.requested_client_id = requested_client_id.clone();
            })
            .or_insert_with(|| ComfyWebSocketSession {
                session_id,
                requested_client_id,
                requested_features: ComfyFeatureFlags::default(),
                accepted_features: ComfyFeatureFlags::default(),
            })
            .clone();

        ComfyWebSocketConnect {
            session,
            initial_frames: vec![ComfyExecutionEventTranslator::status(queue_status)],
        }
    }

    pub fn receive_feature_flags(
        &mut self,
        session_id: &ComfyClientSessionId,
        requested: ComfyFeatureFlags,
    ) -> Option<ComfyWebSocketFrame> {
        let session = self.sessions.get_mut(session_id)?;
        let accepted = negotiate_features(&requested, &self.server_features);
        session.requested_features = requested.clone();
        session.accepted_features = accepted.clone();

        Some(ComfyExecutionEventTranslator::translate(
            ComfyRuntimeEvent::FeatureFlags(ClientFeatureNegotiation {
                client_id: session
                    .requested_client_id
                    .clone()
                    .unwrap_or_else(|| session.session_id.as_str().to_string()),
                requested,
                accepted,
            }),
            &session.accepted_features,
        ))
    }

    pub fn translate_for_session(
        &self,
        session_id: &ComfyClientSessionId,
        event: ComfyRuntimeEvent,
    ) -> Option<ComfyWebSocketFrame> {
        let session = self.sessions.get(session_id)?;
        Some(ComfyExecutionEventTranslator::translate(
            event,
            &session.accepted_features,
        ))
    }

    pub fn session(&self, session_id: &ComfyClientSessionId) -> Option<&ComfyWebSocketSession> {
        self.sessions.get(session_id)
    }
}

fn negotiate_features(
    requested: &ComfyFeatureFlags,
    server_features: &ComfyFeatureFlags,
) -> ComfyFeatureFlags {
    let mut accepted = ComfyFeatureFlags::default();
    for (feature, requested_enabled) in &requested.flags {
        accepted = accepted.with_flag(
            feature.clone(),
            *requested_enabled && server_features.enabled(feature),
        );
    }
    accepted
}
