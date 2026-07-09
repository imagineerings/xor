use crate::db::{ChannelId, MessageId, ScheduledMessageId, UserId};
use sea_orm::entity::prelude::*;
use serde_json::Value as JsonValue;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "scheduled_messages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: ScheduledMessageId,
    pub channel_id: ChannelId,
    pub sender_id: UserId,
    pub body: String,
    pub scheduled_at: PrimitiveDateTime,
    pub created_at: PrimitiveDateTime,
    pub state: i16,
    pub nonce: Vec<u8>,
    pub mentions: JsonValue,
    pub delivered_message_id: Option<MessageId>,
    pub failure_reason: Option<String>,
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
        belongs_to = "super::user::Entity",
        from = "Column::SenderId",
        to = "super::user::Column::Id"
    )]
    Sender,
    #[sea_orm(
        belongs_to = "super::channel_message::Entity",
        from = "Column::DeliveredMessageId",
        to = "super::channel_message::Column::Id"
    )]
    DeliveredMessage,
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

impl Related<super::channel_message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DeliveredMessage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
