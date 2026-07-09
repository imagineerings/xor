use crate::db::{ChannelId, MessageId, UserId};
use sea_orm::entity::prelude::*;
use time::PrimitiveDateTime;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "channel_files")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub channel_id: ChannelId,
    pub message_id: Option<MessageId>,
    pub filename: String,
    pub file_size: i64,
    pub mime_type: String,
    pub storage_path: String,
    pub uploader_id: UserId,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub duration_ms: Option<i64>,
    pub created_at: PrimitiveDateTime,
    pub uploaded_at: Option<PrimitiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::channel::Entity",
        from = "Column::ChannelId",
        to = "super::channel::Column::Id"
    )]
    Channel,
    #[sea_orm(
        belongs_to = "super::channel_message::Entity",
        from = "Column::MessageId",
        to = "super::channel_message::Column::Id"
    )]
    Message,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UploaderId",
        to = "super::user::Column::Id"
    )]
    Uploader,
}

impl Related<super::channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Channel.def()
    }
}

impl Related<super::channel_message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Message.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Uploader.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
