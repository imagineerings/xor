use super::*;
use anyhow::{Context as _, anyhow};
use sea_orm::DbBackend;

const NONCE_LEN: usize = 16;
const DEFAULT_CHANNEL_MESSAGE_LIMIT: usize = 50;
const MAX_REACTION_EMOJI_NAME_LEN: usize = 100;
const DEFAULT_SEARCH_LIMIT: usize = 20;
const MAX_SEARCH_LIMIT: usize = 100;
const MAX_SEARCH_QUERY_LEN: usize = 200;

pub struct NewChannelMessage {
    pub channel_id: ChannelId,
    pub sender_id: UserId,
    pub body: String,
    pub nonce: proto::Nonce,
    pub mentions: Vec<proto::ChatMention>,
    pub reply_to_message_id: Option<MessageId>,
    pub scheduled_at: Option<PrimitiveDateTime>,
    pub priority: i16,
}

pub struct ChannelMessageUpdate {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub editor_id: UserId,
    pub body: String,
    pub nonce: proto::Nonce,
    pub mentions: Vec<proto::ChatMention>,
}

pub struct SearchChannelMessagesParams {
    pub channel_id: Option<ChannelId>,
    pub query: String,
    pub before_message_id: Option<MessageId>,
    pub limit: usize,
    pub filter_channel_id: Option<ChannelId>,
    pub filter_sender_id: Option<UserId>,
    pub filter_after: Option<PrimitiveDateTime>,
    pub filter_before: Option<PrimitiveDateTime>,
}

pub struct ChannelMessageSearchResult {
    pub message: proto::ChannelMessage,
    pub channel_id: ChannelId,
    pub channel_name: String,
    pub sender_id: UserId,
    pub match_positions: Vec<u64>,
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
                    scheduled_at: ActiveValue::Set(message.scheduled_at),
                    priority: ActiveValue::Set(message.priority),
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
                    scheduled_at: ActiveValue::Unchanged(row.scheduled_at),
                    priority: ActiveValue::Unchanged(row.priority),
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
                scheduled_at: ActiveValue::Unchanged(row.scheduled_at),
                priority: ActiveValue::Unchanged(row.priority),
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

    pub async fn search_channel_messages(
        &self,
        user_id: UserId,
        params: SearchChannelMessagesParams,
    ) -> Result<(Vec<ChannelMessageSearchResult>, bool)> {
        let search_terms = search_terms(&params.query)?;
        let truncated_query = params
            .query
            .chars()
            .take(MAX_SEARCH_QUERY_LEN)
            .collect::<String>();
        let limit = if params.limit == 0 {
            DEFAULT_SEARCH_LIMIT
        } else {
            params.limit.min(MAX_SEARCH_LIMIT)
        };
        let row_limit = limit
            .checked_add(1)
            .context("channel message search limit overflow")?;

        self.transaction(|tx| {
            let search_terms = search_terms.clone();
            let truncated_query = truncated_query.clone();

            async move {
                let backend = tx.get_database_backend();
                let mut values = Vec::new();
                let mut filters = vec![
                    "member.user_id = $USER_ID".to_string(),
                    "member.accepted = TRUE".to_string(),
                    "member.role != 'banned'".to_string(),
                    "(member.role IN ('admin', 'member') OR c.visibility = 'public')".to_string(),
                    "cm.deleted_at IS NULL".to_string(),
                ];

                values.push(user_id.0.into());
                let user_id_placeholder = placeholder(values.len(), backend);

                if let Some(channel_id) = params.channel_id.or(params.filter_channel_id) {
                    values.push(channel_id.0.into());
                    filters.push(format!("cm.channel_id = {}", placeholder(values.len(), backend)));
                }
                if let Some(sender_id) = params.filter_sender_id {
                    values.push(sender_id.0.into());
                    filters.push(format!("cm.sender_id = {}", placeholder(values.len(), backend)));
                }
                if let Some(filter_after) = params.filter_after {
                    values.push(filter_after.into());
                    filters.push(format!("cm.created_at >= {}", placeholder(values.len(), backend)));
                }
                if let Some(filter_before) = params.filter_before {
                    values.push(filter_before.into());
                    filters.push(format!("cm.created_at <= {}", placeholder(values.len(), backend)));
                }
                if let Some(before_message_id) = params.before_message_id {
                    values.push(before_message_id.0.into());
                    filters.push(format!("cm.id < {}", placeholder(values.len(), backend)));
                }

                let rank_expression = match backend {
                    DbBackend::Postgres => {
                        values.push(tsquery_prefix_query(&search_terms).into());
                        let query_placeholder = placeholder(values.len(), backend);
                        filters.push(format!(
                            "cm.search_vector @@ to_tsquery('english', {query_placeholder})"
                        ));
                        format!("ts_rank(cm.search_vector, to_tsquery('english', {query_placeholder}))")
                    }
                    DbBackend::Sqlite => {
                        for term in &search_terms {
                            values.push(format!("%{term}%").into());
                            filters.push(format!(
                                "LOWER(cm.body) LIKE {}",
                                placeholder(values.len(), backend)
                            ));
                        }
                        "0.0".to_string()
                    }
                    other => return Err(anyhow!("unsupported database backend {other:?}").into()),
                };

                values.push((row_limit as i64).into());
                let limit_placeholder = placeholder(values.len(), backend);

                let root_channel_id = match backend {
                    DbBackend::Postgres => {
                        "CASE WHEN c.parent_path = '' THEN c.id ELSE split_part(c.parent_path, '/', 1)::integer END"
                    }
                    DbBackend::Sqlite => {
                        "CASE WHEN c.parent_path = '' THEN c.id ELSE CAST(substr(c.parent_path, 1, instr(c.parent_path, '/') - 1) AS INTEGER) END"
                    }
                    other => return Err(anyhow!("unsupported database backend {other:?}").into()),
                };

                let sql = format!(
                    "
                    SELECT
                        cm.id,
                        cm.channel_id,
                        cm.sender_id,
                        cm.body,
                        cm.nonce,
                        cm.reply_to_message_id,
                        cm.created_at,
                        cm.edited_at,
                        cm.deleted_at,
                        cm.scheduled_at,
                        cm.priority,
                        c.name AS channel_name,
                        {rank_expression} AS rank
                    FROM channel_messages cm
                    JOIN channels c ON c.id = cm.channel_id
                    JOIN channel_members member
                        ON member.channel_id = {root_channel_id}
                    WHERE {}
                    ORDER BY rank DESC, cm.id DESC
                    LIMIT {limit_placeholder}
                    ",
                    filters.join(" AND ").replace("$USER_ID", &user_id_placeholder)
                );

                let mut rows = SearchMessageRow::find_by_statement(Statement::from_sql_and_values(
                    backend, &sql, values,
                ))
                .all(&*tx)
                .await?;
                let done = rows.len() <= limit;
                rows.truncate(limit);

                let messages = rows
                    .iter()
                    .map(|row| channel_message::Model {
                        id: row.id,
                        channel_id: row.channel_id,
                        sender_id: row.sender_id,
                        body: row.body.clone(),
                        nonce: row.nonce.clone(),
                        reply_to_message_id: row.reply_to_message_id,
                        created_at: row.created_at,
                        edited_at: row.edited_at,
                        deleted_at: row.deleted_at,
                        scheduled_at: row.scheduled_at,
                        priority: row.priority,
                    })
                    .collect();
                let messages = self.channel_messages_to_proto(messages, &tx).await?;

                Ok((
                    rows.into_iter()
                        .zip(messages)
                        .map(|(row, message)| ChannelMessageSearchResult {
                            match_positions: match_positions(&message.body, &truncated_query),
                            message,
                            channel_id: row.channel_id,
                            channel_name: row.channel_name,
                            sender_id: row.sender_id,
                        })
                        .collect(),
                    done,
                ))
            }
        })
        .await
    }

    pub async fn reindex_channel_message_search(&self) -> Result<u64> {
        self.transaction(|tx| async move {
            let backend = tx.get_database_backend();
            let sql = match backend {
                DbBackend::Postgres => {
                    "UPDATE channel_messages \
                     SET search_vector = to_tsvector('english', COALESCE(body, ''))"
                }
                DbBackend::Sqlite => {
                    "UPDATE channel_messages \
                     SET search_vector = COALESCE(body, '')"
                }
                DbBackend::MySql => {
                    return Err(anyhow!("unsupported database backend").into());
                }
            };

            let result = tx
                .execute(Statement::from_sql_and_values(backend, sql, []))
                .await?;
            Ok(result.rows_affected())
        })
        .await
    }

    pub async fn get_channel_thread(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        message_id: MessageId,
        before_message_id: Option<MessageId>,
        limit: usize,
    ) -> Result<(proto::ChannelMessage, Vec<proto::ChannelMessage>, bool)> {
        self.transaction(|tx| async move {
            let channel = self.get_channel_internal(channel_id, &tx).await?;
            self.check_user_is_channel_participant(&channel, user_id, &tx)
                .await?;

            let root_message = self
                .get_channel_message_model(channel_id, message_id, &tx)
                .await?;
            let mut filter = channel_message::Column::ChannelId
                .eq(channel_id)
                .and(channel_message::Column::ReplyToMessageId.eq(message_id));
            if let Some(before_message_id) = before_message_id {
                filter = filter.and(channel_message::Column::Id.lt(before_message_id));
            }

            let mut replies = channel_message::Entity::find()
                .filter(filter)
                .order_by_desc(channel_message::Column::Id)
                .limit(limit.saturating_add(1) as u64)
                .all(&*tx)
                .await?;
            let done = replies.len() <= limit;
            replies.truncate(limit);
            replies.reverse();

            Ok((
                self.channel_message_to_proto(root_message, &tx).await?,
                self.channel_messages_to_proto(replies, &tx).await?,
                done,
            ))
        })
        .await
    }

    pub async fn get_channel_threads(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Vec<proto::ThreadSummary>> {
        self.transaction(|tx| async move {
            let channel = self.get_channel_internal(channel_id, &tx).await?;
            self.check_user_is_channel_participant(&channel, user_id, &tx)
                .await?;

            let replies = channel_message::Entity::find()
                .filter(channel_message::Column::ChannelId.eq(channel_id))
                .filter(channel_message::Column::ReplyToMessageId.is_not_null())
                .order_by_desc(channel_message::Column::CreatedAt)
                .order_by_desc(channel_message::Column::Id)
                .all(&*tx)
                .await?;

            let mut summaries_by_root_id =
                HashMap::<MessageId, ThreadSummaryAccumulator>::default();
            for reply in replies {
                let Some(root_message_id) = reply.reply_to_message_id else {
                    continue;
                };

                let summary = summaries_by_root_id
                    .entry(root_message_id)
                    .or_insert_with(|| ThreadSummaryAccumulator {
                        reply_count: 0,
                        latest_reply_at: reply.created_at,
                        latest_reply_id: reply.id,
                        participant_user_ids: BTreeSet::default(),
                    });
                summary.reply_count = summary
                    .reply_count
                    .checked_add(1)
                    .context("too many channel thread replies")?;
                if summary.latest_reply_at < reply.created_at
                    || (summary.latest_reply_at == reply.created_at
                        && summary.latest_reply_id < reply.id)
                {
                    summary.latest_reply_at = reply.created_at;
                    summary.latest_reply_id = reply.id;
                }
                summary.participant_user_ids.insert(reply.sender_id);
            }

            let thread_reads = channel_thread_read::Entity::find()
                .filter(channel_thread_read::Column::ChannelId.eq(channel_id))
                .filter(channel_thread_read::Column::UserId.eq(user_id))
                .all(&*tx)
                .await?
                .into_iter()
                .map(|read| (read.root_message_id, read.message_id))
                .collect::<HashMap<_, _>>();

            let mut summaries = summaries_by_root_id
                .into_iter()
                .map(|(root_message_id, summary)| proto::ThreadSummary {
                    root_message_id: root_message_id.to_proto(),
                    reply_count: summary.reply_count,
                    latest_reply_at: summary.latest_reply_at.assume_utc().unix_timestamp() as u64,
                    participant_user_ids: summary
                        .participant_user_ids
                        .into_iter()
                        .map(UserId::to_proto)
                        .collect(),
                    has_unread: thread_reads
                        .get(&root_message_id)
                        .is_none_or(|message_id| *message_id < summary.latest_reply_id),
                })
                .collect::<Vec<_>>();
            summaries.sort_by_key(|summary| std::cmp::Reverse(summary.latest_reply_at));
            Ok(summaries)
        })
        .await
    }

    pub async fn get_channel_message_reply_count(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        message_id: MessageId,
    ) -> Result<u32> {
        self.transaction(|tx| async move {
            let channel = self.get_channel_internal(channel_id, &tx).await?;
            self.check_user_is_channel_participant(&channel, user_id, &tx)
                .await?;
            self.get_channel_message_model(channel_id, message_id, &tx)
                .await?;

            let reply_count = channel_message::Entity::find()
                .filter(channel_message::Column::ChannelId.eq(channel_id))
                .filter(channel_message::Column::ReplyToMessageId.eq(message_id))
                .count(&*tx)
                .await?;
            reply_count
                .try_into()
                .context("too many channel thread replies")
                .map_err(Into::into)
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

    pub async fn acknowledge_channel_thread(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        root_message_id: MessageId,
        message_id: MessageId,
    ) -> Result<()> {
        self.transaction(|tx| async move {
            let channel = self.get_channel_internal(channel_id, &tx).await?;
            self.check_user_is_channel_participant(&channel, user_id, &tx)
                .await?;
            self.get_channel_message_model(channel_id, root_message_id, &tx)
                .await?;
            let reply = self
                .get_channel_message_model(channel_id, message_id, &tx)
                .await?;
            if reply.reply_to_message_id != Some(root_message_id) {
                Err(anyhow!(
                    "message is not a reply in the requested channel thread"
                ))?;
            }

            use channel_thread_read::Column;
            channel_thread_read::Entity::insert(channel_thread_read::ActiveModel {
                channel_id: ActiveValue::Set(channel_id),
                root_message_id: ActiveValue::Set(root_message_id),
                user_id: ActiveValue::Set(user_id),
                message_id: ActiveValue::Set(message_id),
                updated_at: ActiveValue::Set(now()),
            })
            .on_conflict(
                OnConflict::columns([Column::ChannelId, Column::RootMessageId, Column::UserId])
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

#[derive(Debug, FromQueryResult)]
struct SearchMessageRow {
    id: MessageId,
    channel_id: ChannelId,
    sender_id: UserId,
    body: String,
    nonce: Vec<u8>,
    reply_to_message_id: Option<MessageId>,
    created_at: PrimitiveDateTime,
    edited_at: Option<PrimitiveDateTime>,
    deleted_at: Option<PrimitiveDateTime>,
    scheduled_at: Option<PrimitiveDateTime>,
    priority: i16,
    channel_name: String,
    #[allow(dead_code)]
    rank: f64,
}

struct ThreadSummaryAccumulator {
    reply_count: u32,
    latest_reply_at: PrimitiveDateTime,
    latest_reply_id: MessageId,
    participant_user_ids: BTreeSet<UserId>,
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
        scheduled_at: row
            .scheduled_at
            .map(|scheduled_at| unix_timestamp_millis(scheduled_at)),
        priority: row.priority as i32,
        files: Vec::new(),
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

fn unix_timestamp_millis(timestamp: PrimitiveDateTime) -> u64 {
    (timestamp.assume_utc().unix_timestamp_nanos() / 1_000_000) as u64
}

fn placeholder(index: usize, backend: DbBackend) -> String {
    match backend {
        DbBackend::Postgres => format!("${index}"),
        DbBackend::Sqlite => "?".to_string(),
        DbBackend::MySql => "?".to_string(),
    }
}

fn search_terms(query: &str) -> Result<Vec<String>> {
    let truncated_query = query
        .chars()
        .take(MAX_SEARCH_QUERY_LEN)
        .collect::<String>()
        .to_lowercase();
    let terms = truncated_query
        .split_whitespace()
        .map(|term| {
            term.chars()
                .filter(|character| character.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();

    if terms.iter().map(String::len).sum::<usize>() < 2 {
        return Err(anyhow!("Query must be at least 2 characters").into());
    }

    Ok(terms)
}

fn tsquery_prefix_query(terms: &[String]) -> String {
    terms
        .iter()
        .map(|term| format!("{term}:*"))
        .collect::<Vec<_>>()
        .join(" & ")
}

fn match_positions(body: &str, query: &str) -> Vec<u64> {
    let lower_body = body.to_lowercase();
    search_terms(query)
        .unwrap_or_default()
        .into_iter()
        .flat_map(|term| {
            lower_body
                .match_indices(&term)
                .filter_map(|(index, _)| index.try_into().ok())
                .collect::<Vec<u64>>()
        })
        .collect()
}
