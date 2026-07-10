use crate::db::{ChannelId, MessageId, UserId};
use sea_orm::entity::prelude::*;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "channel_messages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub sender_id: UserId,
    pub body: String,
    pub nonce: Vec<u8>,
    pub reply_to_message_id: Option<MessageId>,
    pub created_at: PrimitiveDateTime,
    pub edited_at: Option<PrimitiveDateTime>,
    pub deleted_at: Option<PrimitiveDateTime>,
    pub scheduled_at: Option<PrimitiveDateTime>,
    pub priority: i16,
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
        from = "Column::SenderId",
        to = "super::user::Column::Id"
    )]
    Sender,
    #[sea_orm(has_many = "super::channel_message_mention::Entity")]
    Mentions,
    #[sea_orm(has_many = "super::channel_message_read::Entity")]
    Reads,
    #[sea_orm(has_many = "super::channel_thread_read::Entity")]
    ThreadReads,
    #[sea_orm(has_many = "super::channel_message_reaction::Entity")]
    Reactions,
}

impl Related<super::channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Channel.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sender.def()
    }
}

impl Related<super::channel_message_mention::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Mentions.def()
    }
}

impl Related<super::channel_message_read::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Reads.def()
    }
}

impl Related<super::channel_thread_read::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ThreadReads.def()
    }
}

impl Related<super::channel_message_reaction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Reactions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
