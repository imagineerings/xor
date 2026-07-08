use super::*;
use anyhow::{Context as _, anyhow};

const NONCE_LEN: usize = 16;
const DEFAULT_CHANNEL_MESSAGE_LIMIT: usize = 50;
const MAX_REACTION_EMOJI_NAME_LEN: usize = 100;

pub struct NewChannelMessage {
    pub channel_id: ChannelId,
    pub sender_id: UserId,
    pub body: String,
    pub nonce: proto::Nonce,
    pub mentions: Vec<proto::ChatMention>,
    pub reply_to_message_id: Option<MessageId>,
}

pub struct ChannelMessageUpdate {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub editor_id: UserId,
    pub body: String,
    pub nonce: proto::Nonce,
    pub mentions: Vec<proto::ChatMention>,
}

impl Database {
    pub async fn join_channel_chat(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        connection: ConnectionId,
    ) -> Result<(Vec<proto::ChannelMessage>, ChannelRole)> {
        self.transaction(|tx| async move {
            let channel = self.get_channel_internal(channel_id, &tx).await?;
            let role = self
                .check_user_is_channel_participant(&channel, user_id, &tx)
                .await?;

            channel_chat_participant::Entity::delete_many()
                .filter(channel_chat_participant::Column::ChannelId.eq(channel_id))
                .filter(channel_chat_participant::Column::ConnectionId.eq(connection.id as i32))
                .filter(
                    channel_chat_participant::Column::ConnectionServerId
                        .eq(ServerId(connection.owner_id as i32)),
                )
                .exec(&*tx)
                .await?;

            channel_chat_participant::ActiveModel {
                id: ActiveValue::NotSet,
                channel_id: ActiveValue::Set(channel_id),
                user_id: ActiveValue::Set(user_id),
                connection_id: ActiveValue::Set(connection.id as i32),
                connection_server_id: ActiveValue::Set(ServerId(connection.owner_id as i32)),
            }
            .insert(&*tx)
            .await?;

            let mut rows = channel_message::Entity::find()
                .filter(channel_message::Column::ChannelId.eq(channel_id))
                .order_by_desc(channel_message::Column::Id)
                .limit(DEFAULT_CHANNEL_MESSAGE_LIMIT as u64)
                .all(&*tx)
                .await?;
            rows.reverse();
            let messages = self.channel_messages_to_proto(rows, &tx).await?;
            Ok((messages, role))
        })
        .await
    }

    pub async fn leave_channel_chat(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        connection: ConnectionId,
    ) -> Result<()> {
        self.transaction(|tx| async move {
            channel_chat_participant::Entity::delete_many()
                .filter(channel_chat_participant::Column::ChannelId.eq(channel_id))
                .filter(channel_chat_participant::Column::UserId.eq(user_id))
                .filter(channel_chat_participant::Column::ConnectionId.eq(connection.id as i32))
                .filter(
                    channel_chat_participant::Column::ConnectionServerId
                        .eq(ServerId(connection.owner_id as i32)),
                )
                .exec(&*tx)
                .await?;
            Ok(())
        })
        .await
    }

    pub async fn channel_chat_participant_connection_ids(
        &self,
        channel_id: ChannelId,
    ) -> Result<Vec<ConnectionId>> {
        self.transaction(|tx| async move {
            let rows = channel_chat_participant::Entity::find()
                .filter(channel_chat_participant::Column::ChannelId.eq(channel_id))
                .all(&*tx)
                .await?;
            Ok(rows.into_iter().map(|row| row.connection()).collect())
        })
        .await
    }

    pub async fn create_channel_message(
        &self,
        message: NewChannelMessage,
    ) -> Result<proto::ChannelMessage> {
        self.transaction(|tx| {
            let body = message.body.clone();
            let nonce = message.nonce.clone();
            let mentions = message.mentions.clone();

            async move {
                let channel = self.get_channel_internal(message.channel_id, &tx).await?;
                self.check_user_is_channel_participant(&channel, message.sender_id, &tx)
                    .await?;

                if let Some(reply_to_message_id) = message.reply_to_message_id {
                    self.get_channel_message_model(message.channel_id, reply_to_message_id, &tx)
                        .await?;
                }

                let row = channel_message::ActiveModel {
                    id: ActiveValue::NotSet,
                    channel_id: ActiveValue::Set(message.channel_id),
                    sender_id: ActiveValue::Set(message.sender_id),
                    body: ActiveValue::Set(body),
                    nonce: ActiveValue::Set(nonce_to_bytes(nonce)),
                    reply_to_message_id: ActiveValue::Set(message.reply_to_message_id),
                    created_at: ActiveValue::NotSet,
                    edited_at: ActiveValue::Set(None),
                    deleted_at: ActiveValue::Set(None),
                }
                .insert(&*tx)
                .await?;

                insert_mentions(row.id, mentions, &tx).await?;
                self.channel_message_to_proto(row, &tx).await
            }
        })
        .await
    }

    pub async fn update_channel_message(
        &self,
        update: ChannelMessageUpdate,
    ) -> Result<proto::ChannelMessage> {
        self.transaction(|tx| {
            let body = update.body.clone();
            let nonce = update.nonce.clone();
            let mentions = update.mentions.clone();

            async move {
                let channel = self.get_channel_internal(update.channel_id, &tx).await?;
                let row = self
                    .get_channel_message_model(update.channel_id, update.message_id, &tx)
                    .await?;
                self.check_user_can_mutate_channel_message(&channel, &row, update.editor_id, &tx)
                    .await?;

                let updated = channel_message::Entity::update(channel_message::ActiveModel {
                    id: ActiveValue::Unchanged(update.message_id),
                    channel_id: ActiveValue::Unchanged(update.channel_id),
                    sender_id: ActiveValue::Unchanged(row.sender_id),
                    body: ActiveValue::Set(body),
                    nonce: ActiveValue::Set(nonce_to_bytes(nonce)),
                    reply_to_message_id: ActiveValue::Unchanged(row.reply_to_message_id),
                    created_at: ActiveValue::Unchanged(row.created_at),
                    edited_at: ActiveValue::Set(Some(now())),
                    deleted_at: ActiveValue::Set(None),
                })
                .exec(&*tx)
                .await?;

                channel_message_mention::Entity::delete_many()
                    .filter(channel_message_mention::Column::MessageId.eq(update.message_id))
                    .exec(&*tx)
                    .await?;
                insert_mentions(updated.id, mentions, &tx).await?;
                self.channel_message_to_proto(updated, &tx).await
            }
        })
        .await
    }

    pub async fn delete_channel_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        deleter_id: UserId,
    ) -> Result<proto::ChannelMessage> {
        self.transaction(|tx| async move {
            let channel = self.get_channel_internal(channel_id, &tx).await?;
            let row = self
                .get_channel_message_model(channel_id, message_id, &tx)
                .await?;
            self.check_user_can_mutate_channel_message(&channel, &row, deleter_id, &tx)
                .await?;

            channel_message_mention::Entity::delete_many()
                .filter(channel_message_mention::Column::MessageId.eq(message_id))
                .exec(&*tx)
                .await?;

            delete_message_reactions(message_id, &tx).await?;

            let deleted = channel_message::Entity::update(channel_message::ActiveModel {
                id: ActiveValue::Unchanged(message_id),
                channel_id: ActiveValue::Unchanged(channel_id),
                sender_id: ActiveValue::Unchanged(row.sender_id),
                body: ActiveValue::Set(String::new()),
                nonce: ActiveValue::Unchanged(row.nonce),
                reply_to_message_id: ActiveValue::Unchanged(row.reply_to_message_id),
                created_at: ActiveValue::Unchanged(row.created_at),
                edited_at: ActiveValue::Unchanged(row.edited_at),
                deleted_at: ActiveValue::Set(Some(now())),
            })
            .exec(&*tx)
            .await?;

            self.channel_message_to_proto(deleted, &tx).await
        })
        .await
    }

    pub async fn get_channel_messages(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        before_message_id: Option<MessageId>,
        limit: usize,
    ) -> Result<Vec<proto::ChannelMessage>> {
        self.transaction(|tx| async move {
            let channel = self.get_channel_internal(channel_id, &tx).await?;
            self.check_user_is_channel_participant(&channel, user_id, &tx)
                .await?;

            let mut filter = channel_message::Column::ChannelId.eq(channel_id);
            if let Some(before_message_id) = before_message_id {
                filter = filter.and(channel_message::Column::Id.lt(before_message_id));
            }

            let mut rows = channel_message::Entity::find()
                .filter(filter)
                .order_by_desc(channel_message::Column::Id)
                .limit(limit as u64)
                .all(&*tx)
                .await?;
            rows.reverse();
            self.channel_messages_to_proto(rows, &tx).await
        })
        .await
    }

    pub async fn get_channel_messages_by_id(
        &self,
        message_ids: Vec<MessageId>,
        user_id: UserId,
    ) -> Result<Vec<proto::ChannelMessage>> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }

        self.transaction(|tx| {
            let message_ids = message_ids.clone();

            async move {
                let rows = channel_message::Entity::find()
                    .filter(channel_message::Column::Id.is_in(message_ids))
                    .order_by_asc(channel_message::Column::Id)
                    .all(&*tx)
                    .await?;

                for row in &rows {
                    let channel = self.get_channel_internal(row.channel_id, &tx).await?;
                    self.check_user_is_channel_participant(&channel, user_id, &tx)
                        .await?;
                }

                self.channel_messages_to_proto(rows, &tx).await
            }
        })
        .await
    }

    pub async fn acknowledge_channel_message(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        message_id: MessageId,
    ) -> Result<()> {
        self.transaction(|tx| async move {
            let channel = self.get_channel_internal(channel_id, &tx).await?;
            self.check_user_is_channel_participant(&channel, user_id, &tx)
                .await?;
            self.get_channel_message_model(channel_id, message_id, &tx)
                .await?;

            use channel_message_read::Column;
            channel_message_read::Entity::insert(channel_message_read::ActiveModel {
                channel_id: ActiveValue::Set(channel_id),
                user_id: ActiveValue::Set(user_id),
                message_id: ActiveValue::Set(message_id),
                updated_at: ActiveValue::Set(now()),
            })
            .on_conflict(
                OnConflict::columns([Column::ChannelId, Column::UserId])
                    .update_columns([Column::MessageId, Column::UpdatedAt])
                    .to_owned(),
            )
            .exec_without_returning(&*tx)
            .await?;
            Ok(())
        })
        .await
    }

    pub async fn insert_channel_message_reaction(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        emoji_name: String,
    ) -> Result<Vec<proto::ReactionSummary>> {
        validate_reaction_emoji_name(&emoji_name)?;

        self.transaction(|tx| {
            let emoji_name = emoji_name.clone();

            async move {
                let channel = self.get_channel_internal(channel_id, &tx).await?;
                self.check_user_is_channel_participant(&channel, user_id, &tx)
                    .await?;
                self.get_channel_message_model(channel_id, message_id, &tx)
                    .await?;

                use channel_message_reaction::Column;
                channel_message_reaction::Entity::insert(channel_message_reaction::ActiveModel {
                    channel_id: ActiveValue::Set(channel_id),
                    message_id: ActiveValue::Set(message_id),
                    user_id: ActiveValue::Set(user_id),
                    emoji_name: ActiveValue::Set(emoji_name),
                    created_at: ActiveValue::NotSet,
                })
                .on_conflict(
                    OnConflict::columns([Column::MessageId, Column::UserId, Column::EmojiName])
                        .do_nothing()
                        .to_owned(),
                )
                .exec_without_returning(&*tx)
                .await?;

                get_message_reactions(message_id, &tx).await
            }
        })
        .await
    }

    pub async fn delete_channel_message_reaction(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        emoji_name: String,
    ) -> Result<Vec<proto::ReactionSummary>> {
        validate_reaction_emoji_name(&emoji_name)?;

        self.transaction(|tx| {
            let emoji_name = emoji_name.clone();

            async move {
                let channel = self.get_channel_internal(channel_id, &tx).await?;
                self.check_user_is_channel_participant(&channel, user_id, &tx)
                    .await?;
                self.get_channel_message_model(channel_id, message_id, &tx)
                    .await?;

                channel_message_reaction::Entity::delete_many()
                    .filter(channel_message_reaction::Column::MessageId.eq(message_id))
                    .filter(channel_message_reaction::Column::UserId.eq(user_id))
                    .filter(channel_message_reaction::Column::EmojiName.eq(emoji_name))
                    .exec(&*tx)
                    .await?;

                get_message_reactions(message_id, &tx).await
            }
        })
        .await
    }

    pub async fn get_channel_message_reactions(
        &self,
        message_id: MessageId,
    ) -> Result<Vec<proto::ReactionSummary>> {
        self.transaction(|tx| async move { get_message_reactions(message_id, &tx).await })
            .await
    }

    pub async fn delete_channel_message_reactions(&self, message_id: MessageId) -> Result<()> {
        self.transaction(|tx| async move { delete_message_reactions(message_id, &tx).await })
            .await
    }

    async fn get_channel_message_model(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        tx: &DatabaseTransaction,
    ) -> Result<channel_message::Model> {
        Ok(channel_message::Entity::find()
            .filter(channel_message::Column::Id.eq(message_id))
            .filter(channel_message::Column::ChannelId.eq(channel_id))
            .one(tx)
            .await?
            .with_context(|| format!("no channel message {message_id} in channel {channel_id}"))?)
    }

    async fn check_user_can_mutate_channel_message(
        &self,
        channel: &channel::Model,
        message: &channel_message::Model,
        user_id: UserId,
        tx: &DatabaseTransaction,
    ) -> Result<()> {
        if message.sender_id == user_id {
            self.check_user_is_channel_participant(channel, user_id, tx)
                .await?;
            return Ok(());
        }

        self.check_user_is_channel_admin(channel, user_id, tx)
            .await
            .map(|_| ())
    }

    async fn channel_messages_to_proto(
        &self,
        rows: Vec<channel_message::Model>,
        tx: &DatabaseTransaction,
    ) -> Result<Vec<proto::ChannelMessage>> {
        let mentions_by_message_id =
            mentions_by_message_id(rows.iter().map(|row| row.id), tx).await?;
        let reactions_by_message_id =
            reaction_summaries_by_message_id(rows.iter().map(|row| row.id), tx).await?;
        rows.into_iter()
            .map(|row| {
                channel_message_to_proto(row, &mentions_by_message_id, &reactions_by_message_id)
            })
            .collect()
    }

    async fn channel_message_to_proto(
        &self,
        row: channel_message::Model,
        tx: &DatabaseTransaction,
    ) -> Result<proto::ChannelMessage> {
        let mentions_by_message_id = mentions_by_message_id([row.id], tx).await?;
        let reactions_by_message_id = reaction_summaries_by_message_id([row.id], tx).await?;
        channel_message_to_proto(row, &mentions_by_message_id, &reactions_by_message_id)
    }
}

async fn get_message_reactions(
    message_id: MessageId,
    tx: &DatabaseTransaction,
) -> Result<Vec<proto::ReactionSummary>> {
    Ok(reaction_summaries_by_message_id([message_id], tx)
        .await?
        .remove(&message_id)
        .unwrap_or_default())
}

async fn delete_message_reactions(message_id: MessageId, tx: &DatabaseTransaction) -> Result<()> {
    channel_message_reaction::Entity::delete_many()
        .filter(channel_message_reaction::Column::MessageId.eq(message_id))
        .exec(tx)
        .await?;
    Ok(())
}

async fn insert_mentions(
    message_id: MessageId,
    mentions: Vec<proto::ChatMention>,
    tx: &DatabaseTransaction,
) -> Result<()> {
    let mentions = mentions
        .into_iter()
        .map(|mention| {
            let range = mention
                .range
                .context("missing channel message mention range")?;
            Ok(channel_message_mention::ActiveModel {
                message_id: ActiveValue::Set(message_id),
                range_start: ActiveValue::Set(
                    range
                        .start
                        .try_into()
                        .context("channel message mention start is out of range")?,
                ),
                range_end: ActiveValue::Set(
                    range
                        .end
                        .try_into()
                        .context("channel message mention end is out of range")?,
                ),
                user_id: ActiveValue::Set(UserId::from_proto(mention.user_id)),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    if mentions.is_empty() {
        return Ok(());
    }

    channel_message_mention::Entity::insert_many(mentions)
        .exec_without_returning(tx)
        .await?;
    Ok(())
}

async fn mentions_by_message_id(
    message_ids: impl IntoIterator<Item = MessageId>,
    tx: &DatabaseTransaction,
) -> Result<HashMap<MessageId, Vec<proto::ChatMention>>> {
    let message_ids = message_ids.into_iter().collect::<Vec<_>>();
    if message_ids.is_empty() {
        return Ok(HashMap::default());
    }

    let rows = channel_message_mention::Entity::find()
        .filter(channel_message_mention::Column::MessageId.is_in(message_ids))
        .order_by_asc(channel_message_mention::Column::MessageId)
        .order_by_asc(channel_message_mention::Column::RangeStart)
        .all(tx)
        .await?;

    let mut mentions = HashMap::default();
    for row in rows {
        mentions
            .entry(row.message_id)
            .or_insert_with(Vec::new)
            .push(proto::ChatMention {
                range: Some(proto::Range {
                    start: row
                        .range_start
                        .try_into()
                        .context("stored channel message mention start is negative")?,
                    end: row
                        .range_end
                        .try_into()
                        .context("stored channel message mention end is negative")?,
                }),
                user_id: row.user_id.to_proto(),
            });
    }
    Ok(mentions)
}

fn channel_message_to_proto(
    row: channel_message::Model,
    mentions_by_message_id: &HashMap<MessageId, Vec<proto::ChatMention>>,
    reactions_by_message_id: &HashMap<MessageId, Vec<proto::ReactionSummary>>,
) -> Result<proto::ChannelMessage> {
    Ok(proto::ChannelMessage {
        id: row.id.to_proto(),
        body: row.body,
        timestamp: row.created_at.assume_utc().unix_timestamp() as u64,
        sender_id: row.sender_id.to_proto(),
        nonce: Some(nonce_from_bytes(&row.nonce)?),
        mentions: mentions_by_message_id
            .get(&row.id)
            .cloned()
            .unwrap_or_default(),
        reply_to_message_id: row.reply_to_message_id.map(MessageId::to_proto),
        edited_at: row
            .edited_at
            .map(|edited_at| edited_at.assume_utc().unix_timestamp() as u64),
        reaction_summaries: reactions_by_message_id
            .get(&row.id)
            .cloned()
            .unwrap_or_default(),
    })
}

async fn reaction_summaries_by_message_id(
    message_ids: impl IntoIterator<Item = MessageId>,
    tx: &DatabaseTransaction,
) -> Result<HashMap<MessageId, Vec<proto::ReactionSummary>>> {
    let message_ids = message_ids.into_iter().collect::<Vec<_>>();
    if message_ids.is_empty() {
        return Ok(HashMap::default());
    }

    let rows = channel_message_reaction::Entity::find()
        .filter(channel_message_reaction::Column::MessageId.is_in(message_ids))
        .order_by_asc(channel_message_reaction::Column::MessageId)
        .order_by_asc(channel_message_reaction::Column::EmojiName)
        .order_by_asc(channel_message_reaction::Column::UserId)
        .all(tx)
        .await?;

    let mut grouped = HashMap::<MessageId, BTreeMap<String, Vec<UserId>>>::default();
    for row in rows {
        grouped
            .entry(row.message_id)
            .or_default()
            .entry(row.emoji_name)
            .or_default()
            .push(row.user_id);
    }

    let mut summaries = HashMap::default();
    for (message_id, reactions_by_emoji) in grouped {
        summaries.insert(
            message_id,
            reactions_by_emoji
                .into_iter()
                .map(|(emoji_name, user_ids)| {
                    let count = user_ids
                        .len()
                        .try_into()
                        .context("too many channel message reactions for one emoji")?;
                    Ok(proto::ReactionSummary {
                        emoji_name,
                        count,
                        user_ids: user_ids.into_iter().map(UserId::to_proto).collect(),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        );
    }
    Ok(summaries)
}

fn validate_reaction_emoji_name(emoji_name: &str) -> Result<()> {
    if emoji_name.trim().is_empty() {
        return Err(anyhow!("channel message reaction emoji name is empty").into());
    }

    if emoji_name.len() > MAX_REACTION_EMOJI_NAME_LEN {
        return Err(anyhow!(
            "channel message reaction emoji name exceeds {MAX_REACTION_EMOJI_NAME_LEN} bytes"
        )
        .into());
    }

    Ok(())
}

fn nonce_to_bytes(nonce: proto::Nonce) -> Vec<u8> {
    let nonce: u128 = nonce.into();
    nonce.to_be_bytes().to_vec()
}

fn nonce_from_bytes(bytes: &[u8]) -> Result<proto::Nonce> {
    if bytes.len() != NONCE_LEN {
        return Err(anyhow!("invalid channel message nonce length {}", bytes.len()).into());
    }

    let mut nonce = [0; NONCE_LEN];
    nonce.copy_from_slice(bytes);
    Ok(u128::from_be_bytes(nonce).into())
}

fn now() -> PrimitiveDateTime {
    let now = time::OffsetDateTime::now_utc();
    PrimitiveDateTime::new(now.date(), now.time())
}
