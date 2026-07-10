use crate::db::{GroupId, MessageId, UserId};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "channel_message_mentions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub message_id: MessageId,
    #[sea_orm(primary_key)]
    pub range_start: i64,
    #[sea_orm(primary_key)]
    pub range_end: i64,
    #[sea_orm(primary_key)]
    pub user_id: UserId,
    pub source_group_id: Option<GroupId>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::channel_message::Entity",
        from = "Column::MessageId",
        to = "super::channel_message::Column::Id"
    )]
    Message,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::channel_message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Message.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
