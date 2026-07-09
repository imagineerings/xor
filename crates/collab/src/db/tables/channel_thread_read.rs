use crate::db::{ChannelId, MessageId, UserId};
use sea_orm::entity::prelude::*;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "channel_thread_reads")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub channel_id: ChannelId,
    #[sea_orm(primary_key)]
    pub root_message_id: MessageId,
    #[sea_orm(primary_key)]
    pub user_id: UserId,
    pub message_id: MessageId,
    pub updated_at: PrimitiveDateTime,
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
        from = "Column::RootMessageId",
        to = "super::channel_message::Column::Id"
    )]
    RootMessage,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
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
        Relation::User.def()
    }
}

impl Related<super::channel_message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Message.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
