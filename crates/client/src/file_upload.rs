use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use rpc::proto;
use std::collections::HashMap;

use crate::{ChannelId, LegacyUserId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileAttachment {
    pub id: String,
    pub filename: String,
    pub file_size: u64,
    pub mime_type: String,
    pub url: String,
    pub uploader_id: LegacyUserId,
    pub uploaded_at: Option<DateTime<Utc>>,
    pub image_width: Option<u64>,
    pub image_height: Option<u64>,
    pub duration_ms: Option<u64>,
    pub thumbnail_url: Option<String>,
    pub download_count: u64,
}

impl TryFrom<proto::FileAttachment> for FileAttachment {
    type Error = anyhow::Error;

    fn try_from(file: proto::FileAttachment) -> Result<Self> {
        let uploaded_at = if file.uploaded_at == 0 {
            None
        } else {
            let timestamp = file
                .uploaded_at
                .try_into()
                .context("file upload time is out of range")?;
            Some(
                DateTime::<Utc>::from_timestamp_millis(timestamp)
                    .context("file upload time is invalid")?,
            )
        };

        Ok(Self {
            id: file.id,
            filename: file.filename,
            file_size: file.file_size,
            mime_type: file.mime_type,
            url: file.url,
            uploader_id: file.uploader_id as LegacyUserId,
            uploaded_at,
            image_width: file.image_width,
            image_height: file.image_height,
            duration_ms: file.duration_ms,
            thumbnail_url: file.thumbnail_url,
            download_count: file.download_count,
        })
    }
}

impl From<FileAttachment> for proto::FileAttachment {
    fn from(file: FileAttachment) -> Self {
        Self {
            id: file.id,
            filename: file.filename,
            file_size: file.file_size,
            mime_type: file.mime_type,
            url: file.url,
            uploader_id: file.uploader_id,
            uploaded_at: file
                .uploaded_at
                .map_or(0, |uploaded_at| uploaded_at.timestamp_millis() as u64),
            image_width: file.image_width,
            image_height: file.image_height,
            duration_ms: file.duration_ms,
            thumbnail_url: file.thumbnail_url,
            download_count: file.download_count,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetFileUploadUrl {
    pub channel_id: ChannelId,
    pub filename: String,
    pub file_size: u64,
    pub mime_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileUploadUrl {
    pub url: String,
    pub file_id: String,
    pub headers: HashMap<String, String>,
}

impl From<proto::GetFileUploadUrlResponse> for FileUploadUrl {
    fn from(response: proto::GetFileUploadUrlResponse) -> Self {
        Self {
            url: response.url,
            file_id: response.file_id,
            headers: response.headers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_attachment_round_trips_through_proto() {
        let uploaded_at = DateTime::<Utc>::from_timestamp_millis(1_725_000_123_456).unwrap();
        let attachment = FileAttachment {
            id: "file-id".to_string(),
            filename: "diagram.png".to_string(),
            file_size: 4096,
            mime_type: "image/png".to_string(),
            url: "https://example.com/file".to_string(),
            uploader_id: 42,
            uploaded_at: Some(uploaded_at),
            image_width: Some(800),
            image_height: Some(600),
            duration_ms: None,
            thumbnail_url: Some("https://example.com/thumbnail".to_string()),
            download_count: 3,
        };

        let proto = proto::FileAttachment::from(attachment.clone());
        assert_eq!(FileAttachment::try_from(proto).unwrap(), attachment);
    }
}
