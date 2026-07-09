use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use gpui::SharedString;
use rpc::proto;

use crate::{ChannelId, LegacyUserId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BookmarkId(pub u64);

impl BookmarkId {
    pub fn from_proto(id: u64) -> Self {
        Self(id)
    }

    pub fn to_proto(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bookmark {
    pub id: BookmarkId,
    pub channel_id: ChannelId,
    pub label: SharedString,
    pub description: Option<SharedString>,
    pub bookmark_type: proto::BookmarkType,
    pub url: SharedString,
    pub file_id: Option<String>,
    pub message_id: Option<u64>,
    pub created_by: LegacyUserId,
    pub created_at: DateTime<Utc>,
    pub sort_order: u32,
}

impl TryFrom<proto::Bookmark> for Bookmark {
    type Error = anyhow::Error;

    fn try_from(bookmark: proto::Bookmark) -> Result<Self> {
        let bookmark_type =
            proto::BookmarkType::from_i32(bookmark.r#type).context("invalid bookmark type")?;
        let created_at = bookmark
            .created_at
            .try_into()
            .context("bookmark created time is out of range")
            .and_then(|timestamp| {
                DateTime::<Utc>::from_timestamp_millis(timestamp)
                    .context("bookmark created time is invalid")
            })?;

        Ok(Self {
            id: BookmarkId::from_proto(bookmark.id),
            channel_id: ChannelId(bookmark.channel_id),
            label: SharedString::from(bookmark.label),
            description: bookmark.description.map(SharedString::from),
            bookmark_type,
            url: SharedString::from(bookmark.url),
            file_id: bookmark.file_id,
            message_id: bookmark.message_id,
            created_by: bookmark.created_by as LegacyUserId,
            created_at,
            sort_order: bookmark.sort_order,
        })
    }
}

impl From<Bookmark> for proto::Bookmark {
    fn from(bookmark: Bookmark) -> Self {
        Self {
            id: bookmark.id.to_proto(),
            channel_id: bookmark.channel_id.0,
            label: bookmark.label.to_string(),
            url: bookmark.url.to_string(),
            file_id: bookmark.file_id,
            message_id: bookmark.message_id,
            r#type: bookmark.bookmark_type as i32,
            created_by: bookmark.created_by,
            created_at: bookmark.created_at.timestamp_millis() as u64,
            description: bookmark.description.map(|description| description.to_string()),
            sort_order: bookmark.sort_order,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AddBookmark {
    pub channel_id: ChannelId,
    pub label: String,
    pub bookmark_type: proto::BookmarkType,
    pub url: String,
    pub file_id: Option<String>,
    pub message_id: Option<u64>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateBookmark {
    pub channel_id: ChannelId,
    pub bookmark_id: BookmarkId,
    pub label: String,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmark_round_trips_through_proto() {
        let created_at = DateTime::<Utc>::from_timestamp_millis(1_725_000_123_456).unwrap();
        let bookmark = Bookmark {
            id: BookmarkId(7),
            channel_id: ChannelId(3),
            label: SharedString::from("Deploy Guide"),
            description: Some(SharedString::from("How to deploy")),
            bookmark_type: proto::BookmarkType::BookmarkLink,
            url: SharedString::from("https://sim.dev/deploy"),
            file_id: None,
            message_id: Some(99),
            created_by: 11,
            created_at,
            sort_order: 2,
        };

        let proto = proto::Bookmark::from(bookmark.clone());
        assert_eq!(Bookmark::try_from(proto).unwrap(), bookmark);
    }

    #[test]
    fn bookmark_rejects_invalid_type() {
        let result = Bookmark::try_from(proto::Bookmark {
            id: 1,
            channel_id: 2,
            label: "Bad".to_string(),
            url: String::new(),
            file_id: None,
            message_id: None,
            r#type: 99,
            created_by: 3,
            created_at: 0,
            description: None,
            sort_order: 0,
        });

        assert!(result.is_err());
    }
}
