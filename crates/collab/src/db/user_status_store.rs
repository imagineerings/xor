use super::*;
use std::sync::Arc;
use time::PrimitiveDateTime;

#[derive(Clone)]
pub struct UserStatusStore {
    db: Arc<Database>,
}

impl UserStatusStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn upsert_custom_status(
        &self,
        user_id: UserId,
        emoji: Option<String>,
        status_text: String,
        expires_at: Option<PrimitiveDateTime>,
    ) -> Result<UserCustomStatus> {
        self.db
            .transaction(|tx| {
                let emoji = emoji.clone();
                let status_text = status_text.clone();
                async move {
                    let updated_at = now();
                    let status = if let Some(existing) =
                        user_custom_status::Entity::find_by_id(user_id)
                            .one(&*tx)
                            .await?
                    {
                        user_custom_status::ActiveModel {
                            user_id: ActiveValue::Unchanged(existing.user_id),
                            emoji: ActiveValue::Set(emoji),
                            status_text: ActiveValue::Set(status_text),
                            expires_at: ActiveValue::Set(expires_at),
                            updated_at: ActiveValue::Set(updated_at),
                        }
                        .update(&*tx)
                        .await?
                    } else {
                        user_custom_status::ActiveModel {
                            user_id: ActiveValue::Set(user_id),
                            emoji: ActiveValue::Set(emoji),
                            status_text: ActiveValue::Set(status_text),
                            expires_at: ActiveValue::Set(expires_at),
                            updated_at: ActiveValue::Set(updated_at),
                        }
                        .insert(&*tx)
                        .await?
                    };
                    Ok(user_custom_status_from_row(status))
                }
            })
            .await
    }

    pub async fn delete_custom_status(&self, user_id: UserId) -> Result<bool> {
        self.db
            .transaction(|tx| async move {
                Ok(user_custom_status::Entity::delete_by_id(user_id)
                    .exec(&*tx)
                    .await?
                    .rows_affected
                    > 0)
            })
            .await
    }

    pub async fn delete_expired_custom_statuses(
        &self,
        threshold: PrimitiveDateTime,
    ) -> Result<Vec<UserId>> {
        self.db
            .transaction(|tx| async move {
                let expired = user_custom_status::Entity::find()
                    .filter(user_custom_status::Column::ExpiresAt.is_not_null())
                    .filter(user_custom_status::Column::ExpiresAt.lt(threshold))
                    .all(&*tx)
                    .await?;
                let user_ids = expired
                    .iter()
                    .map(|status| status.user_id)
                    .collect::<Vec<_>>();
                if !user_ids.is_empty() {
                    user_custom_status::Entity::delete_many()
                        .filter(user_custom_status::Column::UserId.is_in(user_ids.clone()))
                        .exec(&*tx)
                        .await?;
                }
                Ok(user_ids)
            })
            .await
    }

    pub async fn get_custom_statuses(&self, user_ids: Vec<UserId>) -> Result<Vec<UserCustomStatus>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        let threshold = now();
        self.db
            .transaction(|tx| {
                let user_ids = user_ids.clone();
                async move {
                    Ok(user_custom_status::Entity::find()
                        .filter(user_custom_status::Column::UserId.is_in(user_ids))
                        .filter(
                            Condition::any()
                                .add(user_custom_status::Column::ExpiresAt.is_null())
                                .add(user_custom_status::Column::ExpiresAt.gte(threshold)),
                        )
                        .all(&*tx)
                        .await?
                        .into_iter()
                        .map(user_custom_status_from_row)
                        .collect())
                }
            })
            .await
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserCustomStatus {
    pub user_id: UserId,
    pub emoji: Option<String>,
    pub status_text: String,
    pub expires_at: Option<PrimitiveDateTime>,
}

fn user_custom_status_from_row(status: user_custom_status::Model) -> UserCustomStatus {
    UserCustomStatus {
        user_id: status.user_id,
        emoji: status.emoji,
        status_text: status.status_text,
        expires_at: status.expires_at,
    }
}

fn now() -> PrimitiveDateTime {
    let now = time::OffsetDateTime::now_utc();
    PrimitiveDateTime::new(now.date(), now.time())
}
