use crate::{Client, Subscription};
use anyhow::{Context as _, Result};
use futures::Future;
use gpui::{AsyncApp, Entity, WeakEntity};
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
