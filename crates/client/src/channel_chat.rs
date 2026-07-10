use crate::{
    AddBookmark, Bookmark, BookmarkId, ChannelId, Client, FileAttachment, FileUploadUrl,
    GetFileUploadUrl, Subscription, UpdateBookmark,
    scheduled_message::{ScheduledMessage, ScheduledMessageId},
};
use anyhow::{Context as _, Result, bail};
use chrono::{DateTime, Utc};
use futures::Future;
use gpui::{AsyncApp, Entity, SharedString, WeakEntity};
use http_client::{AsyncBody, HttpClient as _, Method};
use rpc::{TypedEnvelope, proto};
use std::sync::Arc;

pub const DEFAULT_THREAD_REPLY_LIMIT: u32 = 50;

pub struct SendChannelMessage {
    pub channel_id: u64,
    pub body: String,
    pub nonce: u128,
    pub mentions: Vec<proto::ChatMention>,
    pub reply_to_message_id: Option<u64>,
    pub file_ids: Vec<String>,
}

pub struct UpdateChannelMessage {
    pub channel_id: u64,
    pub message_id: u64,
    pub body: String,
    pub nonce: u128,
    pub mentions: Vec<proto::ChatMention>,
}

pub struct ScheduleChannelMessage {
    pub channel_id: u64,
    pub body: String,
    pub scheduled_at: DateTime<Utc>,
    pub nonce: u128,
    pub mentions: Vec<proto::ChatMention>,
}

pub struct UpdateScheduledMessage {
    pub scheduled_message_id: ScheduledMessageId,
    pub channel_id: u64,
    pub body: Option<String>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub mentions: Vec<proto::ChatMention>,
}

pub struct SearchChannelMessages {
    pub channel_id: Option<u64>,
    pub query: String,
    pub before_message_id: Option<u64>,
    pub limit: u32,
    pub filter_channel: Option<String>,
    pub filter_user: Option<String>,
    pub filter_after: Option<u64>,
    pub filter_before: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChannelThread {
    pub root_message: proto::ChannelMessage,
    pub replies: Vec<proto::ChannelMessage>,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadSummary {
    pub root_message_id: u64,
    pub reply_count: u32,
    pub latest_reply_at: u64,
    pub participant_user_ids: Vec<u64>,
    pub has_unread: bool,
}

impl From<proto::ThreadSummary> for ThreadSummary {
    fn from(summary: proto::ThreadSummary) -> Self {
        Self {
            root_message_id: summary.root_message_id,
            reply_count: summary.reply_count,
            latest_reply_at: summary.latest_reply_at,
            participant_user_ids: summary.participant_user_ids,
            has_unread: summary.has_unread,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactionSummary {
    pub emoji_name: SharedString,
    pub count: usize,
    pub user_ids: Vec<u64>,
    pub reacted_by_me: bool,
}

impl ReactionSummary {
    pub fn from_proto(summary: proto::ReactionSummary, current_user_id: u64) -> Self {
        let reacted_by_me = summary.user_ids.contains(&current_user_id);
        Self {
            emoji_name: SharedString::from(summary.emoji_name),
            count: summary.count as usize,
            user_ids: summary.user_ids,
            reacted_by_me,
        }
    }
}

impl Client {
    pub async fn join_channel_chat(
        &self,
        channel_id: u64,
    ) -> Result<proto::JoinChannelChatResponse> {
        self.request(proto::JoinChannelChat { channel_id }).await
    }

    pub fn leave_channel_chat(&self, channel_id: u64) -> Result<()> {
        self.send(proto::LeaveChannelChat { channel_id })
    }

    pub async fn send_channel_message(
        &self,
        message: SendChannelMessage,
    ) -> Result<proto::ChannelMessage> {
        let response = self
            .request(proto::SendChannelMessage {
                channel_id: message.channel_id,
                body: message.body,
                nonce: Some(message.nonce.into()),
                mentions: message.mentions,
                reply_to_message_id: message.reply_to_message_id,
                file_ids: message.file_ids,
            })
            .await?;
        response.message.context("missing sent channel message")
    }

    pub async fn schedule_channel_message(
        &self,
        message: ScheduleChannelMessage,
    ) -> Result<ScheduledMessageId> {
        let response = self
            .request(proto::ScheduleChannelMessage {
                channel_id: message.channel_id,
                body: message.body,
                scheduled_at: datetime_to_millis(message.scheduled_at),
                nonce: Some(message.nonce.into()),
                mentions: message.mentions,
            })
            .await?;
        Ok(ScheduledMessageId::from_proto(
            response.scheduled_message_id,
        ))
    }

    pub async fn cancel_scheduled_message(
        &self,
        channel_id: u64,
        scheduled_message_id: ScheduledMessageId,
    ) -> Result<()> {
        self.request(proto::CancelScheduledMessage {
            channel_id,
            scheduled_message_id: scheduled_message_id.to_proto(),
        })
        .await
        .map(|_: proto::Ack| ())
    }

    pub async fn update_scheduled_message(&self, message: UpdateScheduledMessage) -> Result<()> {
        self.request(proto::UpdateScheduledMessage {
            scheduled_message_id: message.scheduled_message_id.to_proto(),
            channel_id: message.channel_id,
            body: message.body,
            scheduled_at: message.scheduled_at.map(datetime_to_millis),
            mentions: message.mentions,
        })
        .await
        .map(|_: proto::Ack| ())
    }

    pub async fn get_scheduled_messages(&self, channel_id: u64) -> Result<Vec<ScheduledMessage>> {
        let response = self
            .request(proto::GetScheduledMessages { channel_id })
            .await?;
        response
            .messages
            .into_iter()
            .map(ScheduledMessage::try_from)
            .collect()
    }

    pub async fn get_bookmarks(&self, channel_id: ChannelId) -> Result<Vec<Bookmark>> {
        let response = self
            .request(proto::GetBookmarks {
                channel_id: channel_id.0,
            })
            .await?;
        response
            .bookmarks
            .into_iter()
            .map(Bookmark::try_from)
            .collect()
    }

    pub async fn get_file_upload_url(&self, request: GetFileUploadUrl) -> Result<FileUploadUrl> {
        let response = self
            .request(proto::GetFileUploadUrl {
                channel_id: request.channel_id.0,
                filename: request.filename,
                file_size: request.file_size,
                mime_type: request.mime_type,
            })
            .await?;
        Ok(response.into())
    }

    pub async fn upload_file_to_s3(
        &self,
        upload_url: &FileUploadUrl,
        bytes: Vec<u8>,
        mut report_progress: impl FnMut(u64, u64) + Send,
    ) -> Result<()> {
        let total_bytes = bytes.len() as u64;
        report_progress(0, total_bytes);

        let mut request = http_client::Request::builder()
            .method(Method::PUT)
            .uri(upload_url.url.as_str());
        for (name, value) in &upload_url.headers {
            request = request.header(name, value);
        }
        let response = self
            .http_client()
            .send(request.body(AsyncBody::from(bytes))?)
            .await?;

        if !response.status().is_success() {
            bail!("file upload failed with status {}", response.status());
        }

        report_progress(total_bytes, total_bytes);
        Ok(())
    }

    pub async fn confirm_file_upload(&self, file_id: impl Into<String>) -> Result<FileAttachment> {
        let response = self
            .request(proto::ConfirmFileUpload {
                file_id: file_id.into(),
            })
            .await?;
        response
            .attachment
            .context("missing file attachment")?
            .try_into()
    }

    pub async fn get_file_download_url(&self, file_id: impl Into<String>) -> Result<String> {
        let response = self
            .request(proto::GetFileDownloadUrl {
                file_id: file_id.into(),
            })
            .await?;
        Ok(response.url)
    }

    pub async fn add_bookmark(&self, bookmark: AddBookmark) -> Result<()> {
        self.request(proto::AddBookmark {
            channel_id: bookmark.channel_id.0,
            label: bookmark.label,
            r#type: bookmark.bookmark_type as i32,
            url: bookmark.url,
            file_id: bookmark.file_id,
            message_id: bookmark.message_id,
            description: bookmark.description,
        })
        .await
        .map(|_: proto::Ack| ())
    }

    pub async fn remove_bookmark(
        &self,
        channel_id: ChannelId,
        bookmark_id: BookmarkId,
    ) -> Result<()> {
        self.request(proto::RemoveBookmark {
            channel_id: channel_id.0,
            bookmark_id: bookmark_id.to_proto(),
        })
        .await
        .map(|_: proto::Ack| ())
    }

    pub async fn update_bookmark(&self, bookmark: UpdateBookmark) -> Result<()> {
        self.request(proto::UpdateBookmark {
            channel_id: bookmark.channel_id.0,
            bookmark_id: bookmark.bookmark_id.to_proto(),
            label: bookmark.label,
            description: bookmark.description,
        })
        .await
        .map(|_: proto::Ack| ())
    }

    pub async fn reorder_bookmarks(
        &self,
        channel_id: ChannelId,
        bookmark_ids: Vec<BookmarkId>,
    ) -> Result<()> {
        self.request(proto::ReorderBookmarks {
            channel_id: channel_id.0,
            bookmark_ids: bookmark_ids.into_iter().map(BookmarkId::to_proto).collect(),
        })
        .await
        .map(|_: proto::Ack| ())
    }

    pub async fn update_channel_message(&self, message: UpdateChannelMessage) -> Result<()> {
        self.request(proto::UpdateChannelMessage {
            channel_id: message.channel_id,
            message_id: message.message_id,
            nonce: Some(message.nonce.into()),
            body: message.body,
            mentions: message.mentions,
        })
        .await
        .map(|_: proto::Ack| ())
    }

    pub async fn remove_channel_message(&self, channel_id: u64, message_id: u64) -> Result<()> {
        self.request(proto::RemoveChannelMessage {
            channel_id,
            message_id,
        })
        .await
        .map(|_: proto::Ack| ())
    }

    pub async fn add_channel_message_reaction(
        &self,
        channel_id: u64,
        message_id: u64,
        emoji_name: String,
    ) -> Result<Vec<proto::ReactionSummary>> {
        let response = self
            .request(proto::AddReaction {
                channel_id,
                message_id,
                emoji_name,
            })
            .await?;
        Ok(response.reactions)
    }

    pub async fn remove_channel_message_reaction(
        &self,
        channel_id: u64,
        message_id: u64,
        emoji_name: String,
    ) -> Result<Vec<proto::ReactionSummary>> {
        let response = self
            .request(proto::RemoveReaction {
                channel_id,
                message_id,
                emoji_name,
            })
            .await?;
        Ok(response.reactions)
    }

    pub fn acknowledge_channel_message(&self, channel_id: u64, message_id: u64) -> Result<()> {
        self.send(proto::AckChannelMessage {
            channel_id,
            message_id,
        })
    }

    pub fn acknowledge_channel_thread(
        &self,
        channel_id: u64,
        root_message_id: u64,
        message_id: u64,
    ) -> Result<()> {
        self.send(proto::AckChannelThread {
            channel_id,
            root_message_id,
            message_id,
        })
    }

    pub async fn get_channel_messages(
        &self,
        channel_id: u64,
        before_message_id: Option<u64>,
    ) -> Result<proto::GetChannelMessagesResponse> {
        self.request(proto::GetChannelMessages {
            channel_id,
            before_message_id: before_message_id.unwrap_or_default(),
        })
        .await
    }

    pub async fn get_channel_messages_by_id(
        &self,
        message_ids: Vec<u64>,
    ) -> Result<proto::GetChannelMessagesResponse> {
        self.request(proto::GetChannelMessagesById { message_ids })
            .await
    }

    pub async fn search_channel_messages(
        &self,
        search: SearchChannelMessages,
    ) -> Result<proto::SearchChannelMessagesResponse> {
        self.request(proto::SearchChannelMessages {
            channel_id: search.channel_id.unwrap_or_default(),
            query: search.query,
            before_message_id: search.before_message_id,
            limit: search.limit,
            filter_channel: search.filter_channel,
            filter_user: search.filter_user,
            filter_after: search.filter_after,
            filter_before: search.filter_before,
        })
        .await
    }

    pub async fn get_thread(&self, channel_id: u64, message_id: u64) -> Result<ChannelThread> {
        self.get_thread_page(channel_id, message_id, None, DEFAULT_THREAD_REPLY_LIMIT)
            .await
    }

    pub async fn get_thread_page(
        &self,
        channel_id: u64,
        message_id: u64,
        before_message_id: Option<u64>,
        limit: u32,
    ) -> Result<ChannelThread> {
        let response = self
            .request(proto::GetThread {
                channel_id,
                message_id,
                before_message_id: before_message_id.unwrap_or_default(),
                limit,
            })
            .await?;
        Ok(ChannelThread {
            root_message: response
                .root_message
                .context("missing thread root message")?,
            replies: response.replies,
            done: response.done,
        })
    }

    pub async fn get_threads(&self, channel_id: u64) -> Result<Vec<ThreadSummary>> {
        let response = self.request(proto::GetThreads { channel_id }).await?;
        Ok(response.threads.into_iter().map(Into::into).collect())
    }

    pub fn add_channel_message_sent_handler<E, H, F>(
        self: &Arc<Self>,
        entity: WeakEntity<E>,
        handler: H,
    ) -> Subscription
    where
        E: 'static,
        H: 'static
            + Sync
            + Fn(Entity<E>, TypedEnvelope<proto::ChannelMessageSent>, AsyncApp) -> F
            + Send
            + Sync,
        F: 'static + Future<Output = Result<()>>,
    {
        self.add_message_handler(entity, handler)
    }

    pub fn add_channel_message_update_handler<E, H, F>(
        self: &Arc<Self>,
        entity: WeakEntity<E>,
        handler: H,
    ) -> Subscription
    where
        E: 'static,
        H: 'static
            + Sync
            + Fn(Entity<E>, TypedEnvelope<proto::ChannelMessageUpdate>, AsyncApp) -> F
            + Send
            + Sync,
        F: 'static + Future<Output = Result<()>>,
    {
        self.add_message_handler(entity, handler)
    }

    pub fn add_channel_message_reactions_update_handler<E, H, F>(
        self: &Arc<Self>,
        entity: WeakEntity<E>,
        handler: H,
    ) -> Subscription
    where
        E: 'static,
        H: 'static
            + Sync
            + Fn(Entity<E>, TypedEnvelope<proto::UpdateMessageReactions>, AsyncApp) -> F
            + Send
            + Sync,
        F: 'static + Future<Output = Result<()>>,
    {
        self.add_message_handler(entity, handler)
    }

    pub fn add_scheduled_message_sent_handler<E, H, F>(
        self: &Arc<Self>,
        entity: WeakEntity<E>,
        handler: H,
    ) -> Subscription
    where
        E: 'static,
        H: 'static
            + Sync
            + Fn(Entity<E>, TypedEnvelope<proto::ScheduledMessageSent>, AsyncApp) -> F
            + Send
            + Sync,
        F: 'static + Future<Output = Result<()>>,
    {
        self.add_message_handler(entity, handler)
    }

    pub fn add_scheduled_message_failed_handler<E, H, F>(
        self: &Arc<Self>,
        entity: WeakEntity<E>,
        handler: H,
    ) -> Subscription
    where
        E: 'static,
        H: 'static
            + Sync
            + Fn(Entity<E>, TypedEnvelope<proto::ScheduledMessageFailed>, AsyncApp) -> F
            + Send
            + Sync,
        F: 'static + Future<Output = Result<()>>,
    {
        self.add_message_handler(entity, handler)
    }

    pub fn add_channel_bookmarks_update_handler<E, H, F>(
        self: &Arc<Self>,
        entity: WeakEntity<E>,
        handler: H,
    ) -> Subscription
    where
        E: 'static,
        H: 'static
            + Sync
            + Fn(Entity<E>, TypedEnvelope<proto::UpdateChannelBookmarks>, AsyncApp) -> F
            + Send
            + Sync,
        F: 'static + Future<Output = Result<()>>,
    {
        self.add_message_handler(entity, handler)
    }
}

fn datetime_to_millis(timestamp: DateTime<Utc>) -> u64 {
    timestamp.timestamp_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reaction_summary_from_proto_marks_current_user() {
        let summary = ReactionSummary::from_proto(
            proto::ReactionSummary {
                emoji_name: "thumbs_up".to_string(),
                count: 2,
                user_ids: vec![1, 2],
            },
            2,
        );

        assert_eq!(summary.emoji_name.as_ref(), "thumbs_up");
        assert_eq!(summary.count, 2);
        assert_eq!(summary.user_ids, vec![1, 2]);
        assert!(summary.reacted_by_me);
    }
}
