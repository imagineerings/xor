use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A message received from an external platform.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncomingMessage {
    /// Platform identifier (e.g., "telegram", "slack").
    pub platform: String,
    /// Platform-specific chat/conversation identifier.
    pub platform_id: String,
    /// Platform-specific user identifier.
    pub user_id: String,
    /// Message text content.
    pub text: String,
    /// Attachments (photos, documents, etc.).
    pub attachments: Vec<Attachment>,
    /// Timestamp of the original message.
    pub timestamp: DateTime<Utc>,
}

/// A message to be sent to an external platform.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutgoingMessage {
    /// Platform identifier.
    pub platform: String,
    /// Platform-specific chat/conversation identifier.
    pub platform_id: String,
    /// Message text content.
    pub text: String,
    /// Attachments to include.
    pub attachments: Vec<Attachment>,
    /// Optional message ID to reply to.
    pub reply_to: Option<String>,
}

/// An attachment (photo, document, etc.) carried by a message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attachment {
    /// Attachment type (e.g., "photo", "document", "audio").
    pub kind: String,
    /// MIME type of the attachment.
    pub mime_type: Option<String>,
    /// URL or file path of the attachment content.
    pub url: Option<String>,
    /// Optional file size in bytes.
    pub file_size: Option<i64>,
    /// Optional file name.
    pub file_name: Option<String>,
}

/// Per-chat state tracked by a gateway handler.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatState {
    /// ID of the last message processed in this chat.
    pub last_message_id: i64,
    /// Current pairing status for this chat's user.
    pub pairing_status: PairingStatus,
    /// Pending action awaiting user confirmation, if any.
    pub pending_action: Option<PendingAction>,
}

/// Pairing status between an external platform user and a sim user identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PairingStatus {
    /// No pairing has been attempted.
    Unpaired,
    /// Pairing is in progress (awaiting user confirmation).
    Pending,
    /// Successfully paired.
    Paired,
}

/// A pending action awaiting user confirmation via the gateway.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingAction {
    /// Unique identifier for this pending action.
    pub id: String,
    /// Human-readable description of the action.
    pub description: String,
    /// Opaque payload to be used when the action is confirmed.
    pub payload: serde_json::Value,
}
