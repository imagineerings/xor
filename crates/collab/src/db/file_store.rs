use super::*;
use anyhow::{Context as _, anyhow};
use aws_sdk_s3::presigning::PresigningConfig;
use rpc::proto;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use time::PrimitiveDateTime;
use uuid::Uuid;

const DEFAULT_UPLOAD_URL_LIFETIME: Duration = Duration::from_secs(10 * 60);
const DEFAULT_DOWNLOAD_URL_LIFETIME: Duration = Duration::from_secs(10 * 60);

#[derive(Clone)]
pub struct FileStore {
    db: Arc<Database>,
    blob_store_client: Option<aws_sdk_s3::Client>,
    config: FileStoreConfig,
}

impl FileStore {
    pub fn new(
        db: Arc<Database>,
        blob_store_client: Option<aws_sdk_s3::Client>,
        config: FileStoreConfig,
    ) -> Self {
        Self {
            db,
            blob_store_client,
            config,
        }
    }

    pub async fn generate_upload_url(&self, request: NewFileUpload) -> Result<FileUploadUrl> {
        validate_upload(request.file_size, &request.mime_type, &self.config)?;

        let blob_store_client = self
            .blob_store_client
            .as_ref()
            .ok_or(FileStoreError::StorageUnavailable)?;
        let bucket = self
            .config
            .storage_bucket
            .as_ref()
            .ok_or(FileStoreError::StorageUnavailable)?;
        let file_id = Uuid::new_v4();
        let storage_path = storage_path(request.channel_id, file_id, &request.filename);
        let content_length = i64::try_from(request.file_size)
            .context("file size is too large to send to blob storage")?;
        let presigning_config = PresigningConfig::expires_in(self.config.upload_url_lifetime)
            .context("creating file upload presigning config")?;

        let presigned = blob_store_client
            .put_object()
            .bucket(bucket)
            .key(storage_path.clone())
            .content_type(request.mime_type.clone())
            .content_length(content_length)
            .presigned(presigning_config)
            .await
            .map_err(|error| {
                Error::from(anyhow::Error::new(FileStoreError::PresignFailed(format!(
                    "creating presigned file upload url: {error}"
                ))))
            })?;

        let row = channel_file::ActiveModel {
            id: ActiveValue::Set(file_id),
            channel_id: ActiveValue::Set(request.channel_id),
            message_id: ActiveValue::Set(None),
            filename: ActiveValue::Set(request.filename),
            file_size: ActiveValue::Set(content_length),
            mime_type: ActiveValue::Set(request.mime_type),
            storage_path: ActiveValue::Set(storage_path),
            uploader_id: ActiveValue::Set(request.uploader_id),
            image_width: ActiveValue::Set(request.image_width),
            image_height: ActiveValue::Set(request.image_height),
            duration_ms: ActiveValue::Set(request.duration_ms),
            created_at: ActiveValue::Set(now()),
            uploaded_at: ActiveValue::Set(None),
        };

        self.db
            .transaction(|tx| {
                let row = row.clone();
                async move {
                    row.insert(&*tx).await?;
                    Ok(())
                }
            })
            .await?;

        Ok(FileUploadUrl {
            file_id,
            url: presigned.uri().to_string(),
            headers: presigned_headers(&presigned),
        })
    }

    pub async fn confirm_upload(
        &self,
        file_id: Uuid,
        uploader_id: UserId,
    ) -> Result<FileAttachment> {
        let uploaded_at = now();
        let row = self
            .db
            .transaction(|tx| async move {
                let row = channel_file::Entity::find_by_id(file_id)
                    .filter(channel_file::Column::UploaderId.eq(uploader_id))
                    .one(&*tx)
                    .await?
                    .context("file upload does not exist")?;

                channel_file::Entity::update(channel_file::ActiveModel {
                    id: ActiveValue::Unchanged(row.id),
                    channel_id: ActiveValue::Unchanged(row.channel_id),
                    message_id: ActiveValue::Unchanged(row.message_id),
                    filename: ActiveValue::Unchanged(row.filename),
                    file_size: ActiveValue::Unchanged(row.file_size),
                    mime_type: ActiveValue::Unchanged(row.mime_type),
                    storage_path: ActiveValue::Unchanged(row.storage_path),
                    uploader_id: ActiveValue::Unchanged(row.uploader_id),
                    image_width: ActiveValue::Unchanged(row.image_width),
                    image_height: ActiveValue::Unchanged(row.image_height),
                    duration_ms: ActiveValue::Unchanged(row.duration_ms),
                    created_at: ActiveValue::Unchanged(row.created_at),
                    uploaded_at: ActiveValue::Set(Some(uploaded_at)),
                })
                .exec(&*tx)
                .await
                .map_err(Into::into)
            })
            .await?;

        self.file_attachment_from_row(row).await
    }

    pub async fn get_file_metadata(&self, file_id: Uuid) -> Result<FileAttachment> {
        let row = self
            .db
            .transaction(|tx| async move {
                channel_file::Entity::find_by_id(file_id)
                    .one(&*tx)
                    .await?
                    .context("file does not exist")
                    .map_err(Into::into)
            })
            .await?;

        self.file_attachment_from_row(row).await
    }

    pub async fn attach_files_to_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        uploader_id: UserId,
        file_ids: Vec<Uuid>,
    ) -> Result<Vec<FileAttachment>> {
        if file_ids.is_empty() {
            return Ok(Vec::new());
        }

        let rows = self
            .db
            .transaction(|tx| {
                let file_ids = file_ids.clone();
                async move {
                    let rows = channel_file::Entity::find()
                        .filter(channel_file::Column::Id.is_in(file_ids.clone()))
                        .filter(channel_file::Column::ChannelId.eq(channel_id))
                        .filter(channel_file::Column::UploaderId.eq(uploader_id))
                        .filter(channel_file::Column::UploadedAt.is_not_null())
                        .all(&*tx)
                        .await?;

                    if rows.len() != file_ids.len() {
                        return Err(Error::from(anyhow!(
                            "one or more file uploads are unavailable"
                        )));
                    }

                    for row in &rows {
                        channel_file::Entity::update(channel_file::ActiveModel {
                            id: ActiveValue::Unchanged(row.id),
                            channel_id: ActiveValue::Unchanged(row.channel_id),
                            message_id: ActiveValue::Set(Some(message_id)),
                            filename: ActiveValue::Unchanged(row.filename.clone()),
                            file_size: ActiveValue::Unchanged(row.file_size),
                            mime_type: ActiveValue::Unchanged(row.mime_type.clone()),
                            storage_path: ActiveValue::Unchanged(row.storage_path.clone()),
                            uploader_id: ActiveValue::Unchanged(row.uploader_id),
                            image_width: ActiveValue::Unchanged(row.image_width),
                            image_height: ActiveValue::Unchanged(row.image_height),
                            duration_ms: ActiveValue::Unchanged(row.duration_ms),
                            created_at: ActiveValue::Unchanged(row.created_at),
                            uploaded_at: ActiveValue::Unchanged(row.uploaded_at),
                        })
                        .exec(&*tx)
                        .await?;
                    }

                    Ok(rows)
                }
            })
            .await?;

        let mut attachments = Vec::with_capacity(rows.len());
        for row in rows {
            attachments.push(self.file_attachment_from_row(row).await?);
        }
        Ok(attachments)
    }

    pub async fn delete_message_files(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<u64> {
        let files = self
            .db
            .transaction(|tx| async move {
                channel_file::Entity::find()
                    .filter(channel_file::Column::ChannelId.eq(channel_id))
                    .filter(channel_file::Column::MessageId.eq(message_id))
                    .all(&*tx)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        if files.is_empty() {
            return Ok(0);
        }

        let file_ids = files.iter().map(|file| file.id).collect::<Vec<_>>();
        let deleted = self
            .db
            .transaction(|tx| {
                let file_ids = file_ids.clone();
                async move {
                    channel_file::Entity::delete_many()
                        .filter(channel_file::Column::Id.is_in(file_ids))
                        .exec(&*tx)
                        .await
                        .map(|result| result.rows_affected)
                        .map_err(Into::into)
                }
            })
            .await?;

        self.delete_objects(&files).await?;
        Ok(deleted)
    }

    async fn file_attachment_from_row(&self, row: channel_file::Model) -> Result<FileAttachment> {
        let url = self.download_url(&row.storage_path).await?;
        file_attachment_from_row(row, url)
    }

    async fn download_url(&self, storage_path: &str) -> Result<String> {
        let blob_store_client = self
            .blob_store_client
            .as_ref()
            .ok_or(FileStoreError::StorageUnavailable)?;
        let bucket = self
            .config
            .storage_bucket
            .as_ref()
            .ok_or(FileStoreError::StorageUnavailable)?;
        let presigning_config = PresigningConfig::expires_in(self.config.download_url_lifetime)
            .context("creating file download presigning config")?;
        let presigned = blob_store_client
            .get_object()
            .bucket(bucket)
            .key(storage_path)
            .presigned(presigning_config)
            .await
            .map_err(|error| {
                Error::from(anyhow::Error::new(FileStoreError::PresignFailed(format!(
                    "creating presigned file download url: {error}"
                ))))
            })?;

        Ok(presigned.uri().to_string())
    }

    async fn delete_objects(&self, files: &[channel_file::Model]) -> Result<()> {
        let blob_store_client = self
            .blob_store_client
            .as_ref()
            .ok_or(FileStoreError::StorageUnavailable)?;
        let bucket = self
            .config
            .storage_bucket
            .as_ref()
            .ok_or(FileStoreError::StorageUnavailable)?;

        for file in files {
            blob_store_client
                .delete_object()
                .bucket(bucket)
                .key(&file.storage_path)
                .send()
                .await
                .map_err(|error| {
                    Error::from(anyhow::Error::new(FileStoreError::DeleteFailed(format!(
                        "deleting file object {}: {error}",
                        file.storage_path
                    ))))
                })?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct FileStoreConfig {
    pub storage_bucket: Option<String>,
    pub max_file_size: u64,
    pub allowed_mime_types: Vec<String>,
    pub upload_url_lifetime: Duration,
    pub download_url_lifetime: Duration,
}

impl FileStoreConfig {
    pub fn new(
        storage_bucket: Option<String>,
        max_file_size: u64,
        allowed_mime_types: Vec<String>,
    ) -> Self {
        Self {
            storage_bucket,
            max_file_size,
            allowed_mime_types,
            upload_url_lifetime: DEFAULT_UPLOAD_URL_LIFETIME,
            download_url_lifetime: DEFAULT_DOWNLOAD_URL_LIFETIME,
        }
    }
}

pub struct NewFileUpload {
    pub channel_id: ChannelId,
    pub filename: String,
    pub file_size: u64,
    pub mime_type: String,
    pub uploader_id: UserId,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub duration_ms: Option<i64>,
}

pub struct FileUploadUrl {
    pub file_id: Uuid,
    pub url: String,
    pub headers: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileStoreError {
    FileTooLarge { max_file_size: u64 },
    UnsupportedFileType,
    StorageUnavailable,
    PresignFailed(String),
    DeleteFailed(String),
}

impl fmt::Display for FileStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileStoreError::FileTooLarge { max_file_size } => {
                write!(
                    formatter,
                    "file size exceeds configured limit of {max_file_size} bytes"
                )
            }
            FileStoreError::UnsupportedFileType => formatter.write_str("file type is not allowed"),
            FileStoreError::StorageUnavailable => {
                formatter.write_str("file storage is unavailable")
            }
            FileStoreError::PresignFailed(message) => formatter.write_str(message),
            FileStoreError::DeleteFailed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for FileStoreError {}

impl From<FileStoreError> for Error {
    fn from(error: FileStoreError) -> Self {
        Error::from(anyhow::Error::new(error))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileAttachment {
    pub id: Uuid,
    pub filename: String,
    pub file_size: u64,
    pub mime_type: String,
    pub url: String,
    pub uploader_id: UserId,
    pub uploaded_at: Option<PrimitiveDateTime>,
    pub image_width: Option<u64>,
    pub image_height: Option<u64>,
    pub duration_ms: Option<u64>,
}

impl FileAttachment {
    pub fn to_proto(self) -> proto::FileAttachment {
        proto::FileAttachment {
            id: self.id.to_string(),
            filename: self.filename,
            file_size: self.file_size,
            mime_type: self.mime_type,
            url: self.url,
            uploader_id: self.uploader_id.to_proto(),
            uploaded_at: self.uploaded_at.map_or(0, unix_timestamp_millis),
            image_width: self.image_width,
            image_height: self.image_height,
            duration_ms: self.duration_ms,
        }
    }
}

fn validate_upload(file_size: u64, mime_type: &str, config: &FileStoreConfig) -> Result<()> {
    if file_size > config.max_file_size {
        return Err(FileStoreError::FileTooLarge {
            max_file_size: config.max_file_size,
        }
        .into());
    }

    if !config.allowed_mime_types.is_empty()
        && !config
            .allowed_mime_types
            .iter()
            .any(|allowed_mime_type| allowed_mime_type == mime_type)
    {
        return Err(FileStoreError::UnsupportedFileType.into());
    }

    Ok(())
}

fn file_attachment_from_row(row: channel_file::Model, url: String) -> Result<FileAttachment> {
    Ok(FileAttachment {
        id: row.id,
        filename: row.filename,
        file_size: u64::try_from(row.file_size).context("stored file size is negative")?,
        mime_type: row.mime_type,
        url,
        uploader_id: row.uploader_id,
        uploaded_at: row.uploaded_at,
        image_width: optional_u64(row.image_width, "stored image width is negative")?,
        image_height: optional_u64(row.image_height, "stored image height is negative")?,
        duration_ms: optional_u64(row.duration_ms, "stored duration is negative")?,
    })
}

fn optional_u64<T>(value: Option<T>, error_message: &'static str) -> Result<Option<u64>>
where
    u64: TryFrom<T>,
{
    value
        .map(|value| u64::try_from(value).map_err(|_| anyhow!(error_message).into()))
        .transpose()
}

fn storage_path(channel_id: ChannelId, file_id: Uuid, filename: &str) -> String {
    format!("channels/{channel_id}/files/{file_id}/{filename}")
}

fn presigned_headers(
    request: &aws_sdk_s3::presigning::PresignedRequest,
) -> HashMap<String, String> {
    request
        .headers()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

fn now() -> PrimitiveDateTime {
    let now = time::OffsetDateTime::now_utc();
    PrimitiveDateTime::new(now.date(), now.time())
}

fn unix_timestamp_millis(timestamp: PrimitiveDateTime) -> u64 {
    (timestamp.assume_utc().unix_timestamp_nanos() / 1_000_000) as u64
}
