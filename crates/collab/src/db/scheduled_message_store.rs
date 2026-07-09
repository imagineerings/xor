use super::*;
use anyhow::{Context as _, anyhow};
use sea_orm::DbBackend;
use std::sync::Arc;
use time::{Duration, PrimitiveDateTime};

const NONCE_LEN: usize = 16;
const STATE_PENDING: i16 = 0;
const STATE_PROCESSING: i16 = 1;
const STATE_FAILED: i16 = 3;
const MIN_SCHEDULE_LEAD: Duration = Duration::minutes(1);
const MAX_SCHEDULE_LEAD: Duration = Duration::days(30);
const DEFAULT_POP_DUE_LIMIT: u64 = 100;

#[derive(Clone)]
pub struct ScheduledMessageStore {
    db: Arc<Database>,
}

impl ScheduledMessageStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn create(&self, message: NewScheduledMessage) -> Result<ScheduledMessageId> {
        validate_scheduled_at(message.scheduled_at)?;

        self.db
            .transaction(|tx| {
                let body = message.body.clone();
                let mentions = message.mentions.clone();
                let nonce = nonce_to_bytes(message.nonce.clone());

                async move {
                    let channel = self
                        .db
                        .get_channel_internal(message.channel_id, &tx)
                        .await?;
                    self.db
                        .check_user_is_channel_participant(&channel, message.sender_id, &tx)
                        .await?;

                    if let Some(existing) = scheduled_message::Entity::find()
                        .filter(scheduled_message::Column::ChannelId.eq(message.channel_id))
                        .filter(scheduled_message::Column::SenderId.eq(message.sender_id))
                        .filter(scheduled_message::Column::Nonce.eq(nonce.clone()))
                        .one(&*tx)
                        .await?
                    {
                        return Ok(existing.id);
                    }

                    let row = scheduled_message::ActiveModel {
                        id: ActiveValue::NotSet,
                        channel_id: ActiveValue::Set(message.channel_id),
                        sender_id: ActiveValue::Set(message.sender_id),
                        body: ActiveValue::Set(body),
                        scheduled_at: ActiveValue::Set(message.scheduled_at),
                        created_at: ActiveValue::NotSet,
                        state: ActiveValue::Set(STATE_PENDING),
                        nonce: ActiveValue::Set(nonce),
                        mentions: ActiveValue::Set(mentions_to_json(&mentions)?),
                        delivered_message_id: ActiveValue::Set(None),
                        failure_reason: ActiveValue::Set(None),
                        updated_at: ActiveValue::NotSet,
                    }
                    .insert(&*tx)
                    .await?;

                    Ok(row.id)
                }
            })
            .await
    }

    pub async fn cancel(
        &self,
        scheduled_message_id: ScheduledMessageId,
        channel_id: ChannelId,
        sender_id: UserId,
    ) -> Result<Option<ScheduledMessage>> {
        self.db
            .transaction(|tx| async move {
                let Some(row) = scheduled_message::Entity::find_by_id(scheduled_message_id)
                    .one(&*tx)
                    .await?
                else {
                    return Ok(None);
                };

                ensure_owner_and_channel(&row, sender_id, channel_id)?;
                if row.state != STATE_PENDING {
                    return Ok(None);
                }

                let message = scheduled_message_to_model(row)?;
                scheduled_message::Entity::delete_by_id(scheduled_message_id)
                    .exec(&*tx)
                    .await?;
                Ok(Some(message))
            })
            .await
    }

    pub async fn update(&self, update: ScheduledMessageUpdate) -> Result<ScheduledMessage> {
        if let Some(scheduled_at) = update.scheduled_at {
            validate_scheduled_at(scheduled_at)?;
        }

        self.db
            .transaction(|tx| {
                let body = update.body.clone();
                let mentions = update.mentions.clone();

                async move {
                    let row = scheduled_message::Entity::find_by_id(update.scheduled_message_id)
                        .one(&*tx)
                        .await?
                        .context(
                            "scheduled message does not exist or has already been delivered",
                        )?;

                    ensure_owner_and_channel(&row, update.sender_id, update.channel_id)?;
                    if row.state != STATE_PENDING {
                        let reason = match row.state {
                            STATE_PROCESSING => "scheduled message is already being delivered",
                            STATE_FAILED => "scheduled message has already failed",
                            _ => "scheduled message is not pending",
                        };
                        return Err(anyhow!(reason).into());
                    }

                    let updated =
                        scheduled_message::Entity::update(scheduled_message::ActiveModel {
                            id: ActiveValue::Unchanged(row.id),
                            channel_id: ActiveValue::Unchanged(row.channel_id),
                            sender_id: ActiveValue::Unchanged(row.sender_id),
                            body: body
                                .map(ActiveValue::Set)
                                .unwrap_or_else(|| ActiveValue::Unchanged(row.body)),
                            scheduled_at: update
                                .scheduled_at
                                .map(ActiveValue::Set)
                                .unwrap_or_else(|| ActiveValue::Unchanged(row.scheduled_at)),
                            created_at: ActiveValue::Unchanged(row.created_at),
                            state: ActiveValue::Unchanged(row.state),
                            nonce: ActiveValue::Unchanged(row.nonce),
                            mentions: match mentions {
                                Some(mentions) => ActiveValue::Set(mentions_to_json(&mentions)?),
                                None => ActiveValue::Unchanged(row.mentions),
                            },
                            delivered_message_id: ActiveValue::Unchanged(row.delivered_message_id),
                            failure_reason: ActiveValue::Set(None),
                            updated_at: ActiveValue::Set(now()),
                        })
                        .exec(&*tx)
                        .await?;

                    scheduled_message_to_model(updated)
                }
            })
            .await
    }

    pub async fn list_for_user(
        &self,
        sender_id: UserId,
        channel_id: ChannelId,
    ) -> Result<Vec<ScheduledMessage>> {
        self.db
            .transaction(|tx| async move {
                let rows = scheduled_message::Entity::find()
                    .filter(scheduled_message::Column::SenderId.eq(sender_id))
                    .filter(scheduled_message::Column::ChannelId.eq(channel_id))
                    .filter(scheduled_message::Column::State.eq(STATE_PENDING))
                    .order_by_asc(scheduled_message::Column::ScheduledAt)
                    .order_by_asc(scheduled_message::Column::Id)
                    .all(&*tx)
                    .await?;

                rows.into_iter().map(scheduled_message_to_model).collect()
            })
            .await
    }

    pub async fn pop_due(&self) -> Result<Vec<ScheduledMessage>> {
        self.pop_due_with_limit(DEFAULT_POP_DUE_LIMIT).await
    }

    pub async fn pop_due_with_limit(&self, limit: u64) -> Result<Vec<ScheduledMessage>> {
        self.db
            .transaction(|tx| async move {
                let backend = tx.get_database_backend();
                let now = now();
                let rows = match backend {
                    DbBackend::Postgres => {
                        scheduled_message::Model::find_by_statement(Statement::from_sql_and_values(
                            backend,
                            "
                            UPDATE scheduled_messages
                            SET state = 1, updated_at = NOW()
                            WHERE id IN (
                                SELECT id
                                FROM scheduled_messages
                                WHERE state = 0 AND scheduled_at <= $1
                                ORDER BY scheduled_at, id
                                LIMIT $2
                            )
                            AND state = 0
                            RETURNING *
                            ",
                            [now.into(), (limit as i64).into()],
                        ))
                        .all(&*tx)
                        .await?
                    }
                    DbBackend::Sqlite => {
                        scheduled_message::Model::find_by_statement(Statement::from_sql_and_values(
                            backend,
                            "
                            UPDATE scheduled_messages
                            SET state = 1, updated_at = CURRENT_TIMESTAMP
                            WHERE id IN (
                                SELECT id
                                FROM scheduled_messages
                                WHERE state = 0 AND scheduled_at <= ?
                                ORDER BY scheduled_at, id
                                LIMIT ?
                            )
                            AND state = 0
                            RETURNING *
                            ",
                            [now.into(), (limit as i64).into()],
                        ))
                        .all(&*tx)
                        .await?
                    }
                    other => return Err(anyhow!("unsupported database backend {other:?}").into()),
                };

                rows.into_iter().map(scheduled_message_to_model).collect()
            })
            .await
    }

    pub async fn delete_delivered(&self, scheduled_message_id: ScheduledMessageId) -> Result<()> {
        self.db
            .transaction(|tx| async move {
                scheduled_message::Entity::delete_by_id(scheduled_message_id)
                    .exec(&*tx)
                    .await?;
                Ok(())
            })
            .await
    }

    pub async fn count_pending_for_user(&self, sender_id: UserId) -> Result<u64> {
        self.db
            .transaction(|tx| async move {
                Ok(scheduled_message::Entity::find()
                    .filter(scheduled_message::Column::SenderId.eq(sender_id))
                    .filter(scheduled_message::Column::State.eq(STATE_PENDING))
                    .count(&*tx)
                    .await?)
            })
            .await
    }

    pub async fn mark_failed(
        &self,
        scheduled_message_id: ScheduledMessageId,
        reason: String,
    ) -> Result<()> {
        self.db
            .transaction(|tx| {
                let reason = reason.clone();

                async move {
                    scheduled_message::Entity::update(scheduled_message::ActiveModel {
                        id: ActiveValue::Unchanged(scheduled_message_id),
                        state: ActiveValue::Set(STATE_FAILED),
                        failure_reason: ActiveValue::Set(Some(reason)),
                        updated_at: ActiveValue::Set(now()),
                        ..Default::default()
                    })
                    .exec(&*tx)
                    .await?;
                    Ok(())
                }
            })
            .await
    }

    pub async fn reset_stale_processing(&self, stale_before: PrimitiveDateTime) -> Result<u64> {
        self.db
            .transaction(|tx| async move {
                let result = scheduled_message::Entity::update_many()
                    .col_expr(scheduled_message::Column::State, Expr::value(STATE_PENDING))
                    .col_expr(scheduled_message::Column::UpdatedAt, Expr::value(now()))
                    .filter(scheduled_message::Column::State.eq(STATE_PROCESSING))
                    .filter(scheduled_message::Column::UpdatedAt.lt(stale_before))
                    .exec(&*tx)
                    .await?;
                Ok(result.rows_affected)
            })
            .await
    }
}

pub struct NewScheduledMessage {
    pub channel_id: ChannelId,
    pub sender_id: UserId,
    pub body: String,
    pub scheduled_at: PrimitiveDateTime,
    pub nonce: proto::Nonce,
    pub mentions: Vec<proto::ChatMention>,
}

pub struct ScheduledMessageUpdate {
    pub scheduled_message_id: ScheduledMessageId,
    pub channel_id: ChannelId,
    pub sender_id: UserId,
    pub body: Option<String>,
    pub scheduled_at: Option<PrimitiveDateTime>,
    pub mentions: Option<Vec<proto::ChatMention>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledMessage {
    pub id: ScheduledMessageId,
    pub channel_id: ChannelId,
    pub sender_id: UserId,
    pub body: String,
    pub scheduled_at: PrimitiveDateTime,
    pub created_at: PrimitiveDateTime,
    pub nonce: proto::Nonce,
    pub mentions: Vec<proto::ChatMention>,
}

impl ScheduledMessage {
    pub fn to_proto(self) -> proto::ScheduledMessage {
        proto::ScheduledMessage {
            id: self.id.to_proto(),
            channel_id: self.channel_id.to_proto(),
            body: self.body,
            sender_id: self.sender_id.to_proto(),
            scheduled_at: unix_timestamp_millis(self.scheduled_at),
            created_at: unix_timestamp_millis(self.created_at),
            nonce: Some(self.nonce),
            mentions: self.mentions,
        }
    }
}

fn scheduled_message_to_model(row: scheduled_message::Model) -> Result<ScheduledMessage> {
    Ok(ScheduledMessage {
        id: row.id,
        channel_id: row.channel_id,
        sender_id: row.sender_id,
        body: row.body,
        scheduled_at: row.scheduled_at,
        created_at: row.created_at,
        nonce: nonce_from_bytes(&row.nonce)?,
        mentions: serde_json::from_value(row.mentions)?,
    })
}

fn ensure_owner_and_channel(
    row: &scheduled_message::Model,
    sender_id: UserId,
    channel_id: ChannelId,
) -> Result<()> {
    if row.sender_id != sender_id || row.channel_id != channel_id {
        return Err(anyhow!("scheduled message does not belong to user in channel").into());
    }
    Ok(())
}

fn validate_scheduled_at(scheduled_at: PrimitiveDateTime) -> Result<()> {
    let earliest = now() + MIN_SCHEDULE_LEAD;
    if scheduled_at < earliest {
        return Err(anyhow!("scheduled message must be at least 1 minute in the future").into());
    }

    let latest = now() + MAX_SCHEDULE_LEAD;
    if scheduled_at > latest {
        return Err(anyhow!("scheduled message cannot be more than 30 days in the future").into());
    }

    Ok(())
}

fn mentions_to_json(mentions: &[proto::ChatMention]) -> Result<serde_json::Value> {
    Ok(serde_json::to_value(mentions)?)
}

fn nonce_to_bytes(nonce: proto::Nonce) -> Vec<u8> {
    let nonce: u128 = nonce.into();
    nonce.to_be_bytes().to_vec()
}

fn nonce_from_bytes(bytes: &[u8]) -> Result<proto::Nonce> {
    if bytes.len() != NONCE_LEN {
        return Err(anyhow!("invalid scheduled message nonce length {}", bytes.len()).into());
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
