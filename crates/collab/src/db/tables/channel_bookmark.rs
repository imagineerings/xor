use crate::db::{BookmarkId, ChannelId, MessageId, UserId};
use sea_orm::entity::prelude::*;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "channel_bookmarks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: BookmarkId,
    pub channel_id: ChannelId,
    pub label: String,
    pub description: Option<String>,
    pub bookmark_type: i16,
    pub url: String,
    pub file_id: Option<String>,
    pub message_id: Option<MessageId>,
    pub created_by: UserId,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub sort_order: i32,
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
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
    #[sea_orm(
        belongs_to = "super::channel_message::Entity",
        from = "Column::MessageId",
        to = "super::channel_message::Column::Id"
    )]
    Message,
}

impl Related<super::channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Channel.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

impl Related<super::channel_message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Message.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
