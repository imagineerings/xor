use crate::{Client, Subscription};
use anyhow::{Context as _, Result};
use futures::Future;
use gpui::{AsyncApp, Entity, SharedString, WeakEntity};
use rpc::{TypedEnvelope, proto};
use std::sync::Arc;

pub struct SendChannelMessage {
    pub channel_id: u64,
    pub body: String,
    pub nonce: u128,
    pub mentions: Vec<proto::ChatMention>,
    pub reply_to_message_id: Option<u64>,
}

pub struct UpdateChannelMessage {
    pub channel_id: u64,
    pub message_id: u64,
    pub body: String,
    pub nonce: u128,
    pub mentions: Vec<proto::ChatMention>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChannelThread {
    pub root_message: proto::ChannelMessage,
    pub replies: Vec<proto::ChannelMessage>,
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
            })
            .await?;
        response.message.context("missing sent channel message")
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

    pub async fn get_thread(&self, channel_id: u64, message_id: u64) -> Result<ChannelThread> {
        let response = self
            .request(proto::GetThread {
                channel_id,
                message_id,
            })
            .await?;
        Ok(ChannelThread {
            root_message: response
                .root_message
                .context("missing thread root message")?,
            replies: response.replies,
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
