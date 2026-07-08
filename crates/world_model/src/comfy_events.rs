use serde::{Deserialize, Serialize};

use crate::{
    ClientFeatureNegotiation, ComfyFeatureFlags, ComfyPromptId, ComfyRuntimeEvent, PreviewPayload,
    QueueStatus, graph::NodeId,
};

pub const PREVIEW_METADATA_FEATURE: &str = "preview_metadata";
pub const LEGACY_PREVIEW_FEATURE: &str = "legacy_preview";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyWebSocketFrame {
    Json {
        event: ComfyWebSocketEventName,
        payload: ComfyWebSocketPayload,
    },
    BinaryPreview {
        prompt_id: ComfyPromptId,
        node_id: NodeId,
        mime_type: String,
        byte_count: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyWebSocketEventName {
    Status,
    Executing,
    Progress,
    Preview,
    FeatureFlags,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComfyWebSocketPayload {
    Status(QueueStatus),
    Executing {
        prompt_id: ComfyPromptId,
        node_id: Option<NodeId>,
    },
    Progress {
        prompt_id: ComfyPromptId,
        node_id: NodeId,
        value: u64,
        max: u64,
    },
    PreviewMetadata {
        prompt_id: ComfyPromptId,
        node_id: NodeId,
        artifact_id: String,
        mime_type: String,
        width: Option<u32>,
        height: Option<u32>,
    },
    FeatureFlags(ClientFeatureNegotiation),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComfyExecutionEventTranslator;

impl ComfyExecutionEventTranslator {
    pub fn translate(
        event: ComfyRuntimeEvent,
        accepted_features: &ComfyFeatureFlags,
    ) -> ComfyWebSocketFrame {
        match event {
            ComfyRuntimeEvent::Status(status) => ComfyWebSocketFrame::Json {
                event: ComfyWebSocketEventName::Status,
                payload: ComfyWebSocketPayload::Status(status),
            },
            ComfyRuntimeEvent::Executing { prompt_id, node_id } => ComfyWebSocketFrame::Json {
                event: ComfyWebSocketEventName::Executing,
                payload: ComfyWebSocketPayload::Executing { prompt_id, node_id },
            },
            ComfyRuntimeEvent::Progress {
                prompt_id,
                node_id,
                value,
                max,
            } => ComfyWebSocketFrame::Json {
                event: ComfyWebSocketEventName::Progress,
                payload: ComfyWebSocketPayload::Progress {
                    prompt_id,
                    node_id,
                    value,
                    max,
                },
            },
            ComfyRuntimeEvent::Preview {
                prompt_id,
                node_id,
                payload,
            } => translate_preview(prompt_id, node_id, payload, accepted_features),
            ComfyRuntimeEvent::FeatureFlags(negotiation) => ComfyWebSocketFrame::Json {
                event: ComfyWebSocketEventName::FeatureFlags,
                payload: ComfyWebSocketPayload::FeatureFlags(negotiation),
            },
        }
    }

    pub fn status(status: QueueStatus) -> ComfyWebSocketFrame {
        Self::translate(
            ComfyRuntimeEvent::Status(status),
            &ComfyFeatureFlags::default(),
        )
    }
}

fn translate_preview(
    prompt_id: ComfyPromptId,
    node_id: NodeId,
    payload: PreviewPayload,
    accepted_features: &ComfyFeatureFlags,
) -> ComfyWebSocketFrame {
    match payload {
        PreviewPayload::Metadata {
            artifact_id,
            mime_type,
            width,
            height,
        } if accepted_features.enabled(PREVIEW_METADATA_FEATURE) => ComfyWebSocketFrame::Json {
            event: ComfyWebSocketEventName::Preview,
            payload: ComfyWebSocketPayload::PreviewMetadata {
                prompt_id,
                node_id,
                artifact_id,
                mime_type,
                width,
                height,
            },
        },
        PreviewPayload::Metadata { mime_type, .. } => ComfyWebSocketFrame::BinaryPreview {
            prompt_id,
            node_id,
            mime_type,
            byte_count: 0,
        },
        PreviewPayload::LegacyBytes {
            mime_type,
            byte_count,
        } => ComfyWebSocketFrame::BinaryPreview {
            prompt_id,
            node_id,
            mime_type,
            byte_count,
        },
    }
}
