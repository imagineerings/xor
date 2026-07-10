use super::*;

impl Database {
    pub async fn bypass_dnd_for_urgent(&self, user_id: UserId) -> Result<bool> {
        self.transaction(|tx| async move {
            Ok(user_notification_preference::Entity::find_by_id(user_id)
                .one(&*tx)
                .await?
                .is_some_and(|preference| preference.bypass_dnd_for_urgent))
        })
        .await
    }

    pub async fn set_bypass_dnd_for_urgent(&self, user_id: UserId, bypass: bool) -> Result<()> {
        self.transaction(|tx| async move {
            user_notification_preference::Entity::insert(
                user_notification_preference::ActiveModel {
                    user_id: ActiveValue::Set(user_id),
                    bypass_dnd_for_urgent: ActiveValue::Set(bypass),
                },
            )
            .on_conflict(
                OnConflict::column(user_notification_preference::Column::UserId)
                    .update_column(user_notification_preference::Column::BypassDndForUrgent)
                    .to_owned(),
            )
            .exec(&*tx)
            .await?;
            Ok(())
        })
        .await
    }
}
