use super::*;
use anyhow::Context as _;
use std::sync::Arc;
use time::PrimitiveDateTime;

#[derive(Clone)]
pub struct JoinRequestStore {
    db: Arc<Database>,
}

impl JoinRequestStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn request_join(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        reason: Option<String>,
    ) -> Result<PendingJoinRequest> {
        self.db
            .transaction(|tx| {
                let reason = reason.clone();
                async move {
                    let request = channel_join_request::ActiveModel {
                        id: ActiveValue::NotSet,
                        channel_id: ActiveValue::Set(channel_id),
                        user_id: ActiveValue::Set(user_id),
                        reason: ActiveValue::Set(reason),
                        created_at: ActiveValue::Set(now()),
                    }
                    .insert(&*tx)
                    .await?;
                    Ok(pending_join_request_from_row(request))
                }
            })
            .await
    }

    pub async fn pending_join_request_exists(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<bool> {
        self.db
            .transaction(|tx| async move {
                Ok(channel_join_request::Entity::find()
                    .filter(channel_join_request::Column::ChannelId.eq(channel_id))
                    .filter(channel_join_request::Column::UserId.eq(user_id))
                    .one(&*tx)
                    .await?
                    .is_some())
            })
            .await
    }

    pub async fn approve_join_request(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<bool> {
        self.db
            .transaction(|tx| async move {
                let deleted = channel_join_request::Entity::delete_many()
                    .filter(channel_join_request::Column::ChannelId.eq(channel_id))
                    .filter(channel_join_request::Column::UserId.eq(user_id))
                    .exec(&*tx)
                    .await?;
                if deleted.rows_affected == 0 {
                    return Ok(false);
                }

                let membership = channel_member::Entity::find()
                    .filter(channel_member::Column::ChannelId.eq(channel_id))
                    .filter(channel_member::Column::UserId.eq(user_id))
                    .one(&*tx)
                    .await?;
                if let Some(membership) = membership {
                    channel_member::Entity::update(channel_member::ActiveModel {
                        id: ActiveValue::Unchanged(membership.id),
                        channel_id: ActiveValue::Unchanged(membership.channel_id),
                        user_id: ActiveValue::Unchanged(membership.user_id),
                        accepted: ActiveValue::Set(true),
                        role: ActiveValue::Set(ChannelRole::Member),
                    })
                    .exec(&*tx)
                    .await?;
                } else {
                    channel_member::ActiveModel {
                        id: ActiveValue::NotSet,
                        channel_id: ActiveValue::Set(channel_id),
                        user_id: ActiveValue::Set(user_id),
                        accepted: ActiveValue::Set(true),
                        role: ActiveValue::Set(ChannelRole::Member),
                    }
                    .insert(&*tx)
                    .await?;
                }

                Ok(true)
            })
            .await
    }

    pub async fn deny_join_request(&self, channel_id: ChannelId, user_id: UserId) -> Result<bool> {
        self.db
            .transaction(|tx| async move {
                let deleted = channel_join_request::Entity::delete_many()
                    .filter(channel_join_request::Column::ChannelId.eq(channel_id))
                    .filter(channel_join_request::Column::UserId.eq(user_id))
                    .exec(&*tx)
                    .await?;
                Ok(deleted.rows_affected > 0)
            })
            .await
    }

    pub async fn get_pending_requests(
        &self,
        channel_id: ChannelId,
    ) -> Result<Vec<PendingJoinRequest>> {
        self.db
            .transaction(|tx| async move {
                let requests = channel_join_request::Entity::find()
                    .filter(channel_join_request::Column::ChannelId.eq(channel_id))
                    .order_by_asc(channel_join_request::Column::CreatedAt)
                    .order_by_asc(channel_join_request::Column::Id)
                    .all(&*tx)
                    .await?;
                Ok(requests
                    .into_iter()
                    .map(pending_join_request_from_row)
                    .collect())
            })
            .await
    }

    pub async fn expire_old_requests(
        &self,
        threshold: PrimitiveDateTime,
    ) -> Result<Vec<ExpiredJoinRequest>> {
        self.db
            .transaction(|tx| async move {
                let requests = channel_join_request::Entity::find()
                    .filter(channel_join_request::Column::CreatedAt.lt(threshold))
                    .find_also_related(channel::Entity)
                    .all(&*tx)
                    .await?;
                let expired = requests
                    .iter()
                    .map(|(request, channel)| {
                        let channel = channel
                            .as_ref()
                            .context("join request channel is missing")?;
                        Ok(ExpiredJoinRequest {
                            channel_id: request.channel_id,
                            channel_name: channel.name.clone(),
                            user_id: request.user_id,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let request_ids = requests
                    .into_iter()
                    .map(|(request, _)| request.id)
                    .collect::<Vec<_>>();
                if !request_ids.is_empty() {
                    channel_join_request::Entity::delete_many()
                        .filter(channel_join_request::Column::Id.is_in(request_ids))
                        .exec(&*tx)
                        .await?;
                }
                Ok(expired)
            })
            .await
    }

    pub async fn count_pending_requests(&self, channel_id: ChannelId) -> Result<u64> {
        self.db
            .transaction(|tx| async move {
                channel_join_request::Entity::find()
                    .filter(channel_join_request::Column::ChannelId.eq(channel_id))
                    .count(&*tx)
                    .await
                    .map_err(Into::into)
            })
            .await
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingJoinRequest {
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub reason: Option<String>,
    pub created_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpiredJoinRequest {
    pub channel_id: ChannelId,
    pub channel_name: String,
    pub user_id: UserId,
}

fn pending_join_request_from_row(request: channel_join_request::Model) -> PendingJoinRequest {
    PendingJoinRequest {
        channel_id: request.channel_id,
        user_id: request.user_id,
        reason: request.reason,
        created_at: request.created_at,
    }
}

fn now() -> PrimitiveDateTime {
    let now = time::OffsetDateTime::now_utc();
    PrimitiveDateTime::new(now.date(), now.time())
}
