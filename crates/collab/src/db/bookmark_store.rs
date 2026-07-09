use super::*;
use anyhow::{Context as _, anyhow};
use std::sync::Arc;
use time::PrimitiveDateTime;

#[derive(Clone)]
pub struct BookmarkStore {
    db: Arc<Database>,
}

impl BookmarkStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn create(&self, bookmark: NewBookmark) -> Result<Bookmark> {
        self.db
            .transaction(|tx| {
                let label = bookmark.label.clone();
                let url = bookmark.url.clone();
                let file_id = bookmark.file_id.clone();
                let description = bookmark.description.clone();

                async move {
                    let next_sort_order = next_sort_order(bookmark.channel_id, &tx).await?;
                    let row = channel_bookmark::ActiveModel {
                        id: ActiveValue::NotSet,
                        channel_id: ActiveValue::Set(bookmark.channel_id),
                        label: ActiveValue::Set(label),
                        description: ActiveValue::Set(description),
                        bookmark_type: ActiveValue::Set(bookmark_type_to_i16(
                            bookmark.bookmark_type,
                        )),
                        url: ActiveValue::Set(url),
                        file_id: ActiveValue::Set(file_id),
                        message_id: ActiveValue::Set(bookmark.message_id),
                        created_by: ActiveValue::Set(bookmark.created_by),
                        created_at: ActiveValue::NotSet,
                        updated_at: ActiveValue::NotSet,
                        sort_order: ActiveValue::Set(next_sort_order),
                    }
                    .insert(&*tx)
                    .await?;

                    bookmark_from_row(row)
                }
            })
            .await
    }

    pub async fn get_bookmarks(&self, channel_id: ChannelId) -> Result<Vec<Bookmark>> {
        self.db
            .transaction(|tx| async move {
                let rows = channel_bookmark::Entity::find()
                    .filter(channel_bookmark::Column::ChannelId.eq(channel_id))
                    .order_by_asc(channel_bookmark::Column::SortOrder)
                    .order_by_asc(channel_bookmark::Column::Id)
                    .all(&*tx)
                    .await?;

                rows.into_iter().map(bookmark_from_row).collect()
            })
            .await
    }

    pub async fn update(&self, update: BookmarkUpdate) -> Result<Bookmark> {
        self.db
            .transaction(|tx| {
                let label = update.label.clone();
                let description = update.description.clone();

                async move {
                    let row = channel_bookmark::Entity::find_by_id(update.bookmark_id)
                        .filter(channel_bookmark::Column::ChannelId.eq(update.channel_id))
                        .one(&*tx)
                        .await?
                        .context("bookmark does not exist in channel")?;

                    let updated = channel_bookmark::Entity::update(channel_bookmark::ActiveModel {
                        id: ActiveValue::Unchanged(row.id),
                        channel_id: ActiveValue::Unchanged(row.channel_id),
                        label: ActiveValue::Set(label),
                        description: description
                            .map(|description| ActiveValue::Set(Some(description)))
                            .unwrap_or_else(|| ActiveValue::Unchanged(row.description)),
                        bookmark_type: ActiveValue::Unchanged(row.bookmark_type),
                        url: ActiveValue::Unchanged(row.url),
                        file_id: ActiveValue::Unchanged(row.file_id),
                        message_id: ActiveValue::Unchanged(row.message_id),
                        created_by: ActiveValue::Unchanged(row.created_by),
                        created_at: ActiveValue::Unchanged(row.created_at),
                        updated_at: ActiveValue::Set(now()),
                        sort_order: ActiveValue::Unchanged(row.sort_order),
                    })
                    .exec(&*tx)
                    .await?;

                    bookmark_from_row(updated)
                }
            })
            .await
    }

    pub async fn delete(&self, channel_id: ChannelId, bookmark_id: BookmarkId) -> Result<bool> {
        self.db
            .transaction(|tx| async move {
                let result = channel_bookmark::Entity::delete_many()
                    .filter(channel_bookmark::Column::ChannelId.eq(channel_id))
                    .filter(channel_bookmark::Column::Id.eq(bookmark_id))
                    .exec(&*tx)
                    .await?;

                Ok(result.rows_affected > 0)
            })
            .await
    }

    pub async fn reorder(
        &self,
        channel_id: ChannelId,
        bookmark_ids: Vec<BookmarkId>,
    ) -> Result<()> {
        self.db
            .transaction(|tx| {
                let bookmark_ids = bookmark_ids.clone();

                async move {
                    if bookmark_ids.is_empty() {
                        return Ok(());
                    }

                    let existing = channel_bookmark::Entity::find()
                        .filter(channel_bookmark::Column::ChannelId.eq(channel_id))
                        .filter(channel_bookmark::Column::Id.is_in(bookmark_ids.clone()))
                        .all(&*tx)
                        .await?;

                    if existing.len() != bookmark_ids.len() {
                        return Err(anyhow!("all reordered bookmarks must belong to channel").into());
                    }

                    for (index, bookmark_id) in bookmark_ids.into_iter().enumerate() {
                        let sort_order = i32::try_from(index)
                            .context("too many bookmarks to store sort order")?;
                        channel_bookmark::Entity::update(channel_bookmark::ActiveModel {
                            id: ActiveValue::Unchanged(bookmark_id),
                            channel_id: ActiveValue::Unchanged(channel_id),
                            sort_order: ActiveValue::Set(sort_order),
                            updated_at: ActiveValue::Set(now()),
                            ..Default::default()
                        })
                        .exec(&*tx)
                        .await?;
                    }

                    Ok(())
                }
            })
            .await
    }

    pub async fn delete_channel_bookmarks(&self, channel_id: ChannelId) -> Result<u64> {
        self.db
            .transaction(|tx| async move {
                let result = channel_bookmark::Entity::delete_many()
                    .filter(channel_bookmark::Column::ChannelId.eq(channel_id))
                    .exec(&*tx)
                    .await?;
                Ok(result.rows_affected)
            })
            .await
    }
}

pub struct NewBookmark {
    pub channel_id: ChannelId,
    pub label: String,
    pub description: Option<String>,
    pub bookmark_type: proto::BookmarkType,
    pub url: String,
    pub file_id: Option<String>,
    pub message_id: Option<MessageId>,
    pub created_by: UserId,
}

pub struct BookmarkUpdate {
    pub channel_id: ChannelId,
    pub bookmark_id: BookmarkId,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bookmark {
    pub id: BookmarkId,
    pub channel_id: ChannelId,
    pub label: String,
    pub description: Option<String>,
    pub bookmark_type: proto::BookmarkType,
    pub url: String,
    pub file_id: Option<String>,
    pub message_id: Option<MessageId>,
    pub created_by: UserId,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub sort_order: i32,
}

impl Bookmark {
    pub fn to_proto(self) -> proto::Bookmark {
        proto::Bookmark {
            id: self.id.to_proto(),
            channel_id: self.channel_id.to_proto(),
            label: self.label,
            url: self.url,
            file_id: self.file_id,
            message_id: self.message_id.map(MessageId::to_proto),
            r#type: self.bookmark_type as i32,
            created_by: self.created_by.to_proto(),
            created_at: unix_timestamp_millis(self.created_at),
            description: self.description,
            sort_order: self.sort_order as u32,
        }
    }
}

async fn next_sort_order(channel_id: ChannelId, tx: &TransactionHandle) -> Result<i32> {
    let latest = channel_bookmark::Entity::find()
        .filter(channel_bookmark::Column::ChannelId.eq(channel_id))
        .order_by_desc(channel_bookmark::Column::SortOrder)
        .order_by_desc(channel_bookmark::Column::Id)
        .one(&**tx)
        .await?;

    match latest {
        Some(bookmark) => bookmark
            .sort_order
            .checked_add(1)
            .context("bookmark sort order overflow")
            .map_err(Into::into),
        None => Ok(0),
    }
}

fn bookmark_from_row(row: channel_bookmark::Model) -> Result<Bookmark> {
    Ok(Bookmark {
        id: row.id,
        channel_id: row.channel_id,
        label: row.label,
        description: row.description,
        bookmark_type: bookmark_type_from_i16(row.bookmark_type)?,
        url: row.url,
        file_id: row.file_id,
        message_id: row.message_id,
        created_by: row.created_by,
        created_at: row.created_at,
        updated_at: row.updated_at,
        sort_order: row.sort_order,
    })
}

fn bookmark_type_to_i16(bookmark_type: proto::BookmarkType) -> i16 {
    bookmark_type as i16
}

fn bookmark_type_from_i16(bookmark_type: i16) -> Result<proto::BookmarkType> {
    proto::BookmarkType::from_i32(bookmark_type.into())
        .context("invalid bookmark type")
        .map_err(Into::into)
}

fn now() -> PrimitiveDateTime {
    let now = time::OffsetDateTime::now_utc();
    PrimitiveDateTime::new(now.date(), now.time())
}

fn unix_timestamp_millis(timestamp: PrimitiveDateTime) -> u64 {
    (timestamp.assume_utc().unix_timestamp_nanos() / 1_000_000) as u64
}
