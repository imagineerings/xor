use anyhow::{Context as _, Result, bail};
use client::{ChannelId, Client, FileAttachment, GetFileUploadUrl};
use collections::HashMap;
use gpui::{App, AppContext as _, Context, Entity, Global, SharedString, Task};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub type FileId = String;

pub fn init(cx: &mut App) {
    UploadManager::init(cx);
}

pub struct UploadManager {
    client: Arc<Client>,
    active_uploads: HashMap<FileId, UploadProgress>,
}

impl UploadManager {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            active_uploads: HashMap::default(),
        }
    }

    pub fn init(cx: &mut App) {
        let client = Client::global(cx);
        let manager = cx.new(|_| Self::new(client));
        cx.set_global(GlobalUploadManager(manager));
    }

    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalUploadManager>().0.clone()
    }

    pub fn upload_file(
        &mut self,
        channel_id: ChannelId,
        file_path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Task<Result<FileAttachment>> {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let filename = file_path
                .file_name()
                .and_then(|filename| filename.to_str())
                .context("file path has no valid filename")?
                .to_string();
            let mime_type = mime_type_for_path(&file_path);
            let (file_size, bytes) = cx
                .background_spawn({
                    let file_path = file_path.clone();
                    async move {
                        let metadata = std::fs::metadata(&file_path)
                            .with_context(|| format!("reading metadata for {:?}", file_path))?;
                        let bytes = std::fs::read(&file_path)
                            .with_context(|| format!("reading file {:?}", file_path))?;
                        anyhow::Ok((metadata.len(), bytes))
                    }
                })
                .await?;

            let upload_url = client
                .get_file_upload_url(GetFileUploadUrl {
                    channel_id,
                    filename: filename.clone(),
                    file_size,
                    mime_type,
                })
                .await?;
            let file_id = upload_url.file_id.clone();

            this.update(cx, |this, cx| {
                this.active_uploads.insert(
                    file_id.clone(),
                    UploadProgress {
                        file_id: file_id.clone(),
                        channel_id,
                        filename: SharedString::from(filename.clone()),
                        progress: 0.0,
                        status: UploadStatus::Uploading,
                    },
                );
                cx.notify();
            })?;
            ensure_not_cancelled(&this, &file_id, cx)?;

            let upload_result = client
                .upload_file_to_s3(&upload_url, bytes, |_, _| {})
                .await;
            if let Err(error) = upload_result {
                mark_upload_failed(&this, &file_id, format!("{error:#}"), cx)?;
                return Err(error);
            }

            this.update(cx, |this, cx| {
                if let Some(upload) = this.active_uploads.get_mut(&file_id) {
                    upload.progress = 1.0;
                    upload.status = UploadStatus::Confirming;
                }
                cx.notify();
            })?;
            ensure_not_cancelled(&this, &file_id, cx)?;

            let attachment = match client.confirm_file_upload(file_id.clone()).await {
                Ok(attachment) => attachment,
                Err(error) => {
                    mark_upload_failed(&this, &file_id, format!("{error:#}"), cx)?;
                    return Err(error);
                }
            };

            this.update(cx, |this, cx| {
                if let Some(upload) = this.active_uploads.get_mut(&file_id) {
                    upload.progress = 1.0;
                    upload.status = UploadStatus::Completed;
                }
                cx.notify();
            })?;
            Ok(attachment)
        })
    }

    pub fn uploads_for_channel(&self, channel_id: ChannelId) -> Vec<UploadProgress> {
        self.active_uploads
            .values()
            .filter(|upload| upload.channel_id == channel_id)
            .cloned()
            .collect()
    }

    pub fn cancel_upload(&mut self, file_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(upload) = self.active_uploads.get_mut(file_id) else {
            return false;
        };
        upload.status = UploadStatus::Cancelled;
        cx.notify();
        true
    }

    pub fn remove_upload(&mut self, file_id: &str, cx: &mut Context<Self>) -> bool {
        let removed = self.active_uploads.remove(file_id).is_some();
        if removed {
            cx.notify();
        }
        removed
    }
}

struct GlobalUploadManager(Entity<UploadManager>);

impl Global for GlobalUploadManager {}

#[derive(Clone, Debug, PartialEq)]
pub struct UploadProgress {
    pub file_id: FileId,
    pub channel_id: ChannelId,
    pub filename: SharedString,
    pub progress: f32,
    pub status: UploadStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UploadStatus {
    Uploading,
    Confirming,
    Completed,
    Failed(String),
    Cancelled,
}

fn ensure_not_cancelled(
    this: &gpui::WeakEntity<UploadManager>,
    file_id: &str,
    cx: &mut gpui::AsyncApp,
) -> Result<()> {
    let is_cancelled = this.update(cx, |this, _| {
        this.active_uploads
            .get(file_id)
            .is_some_and(|upload| upload.status == UploadStatus::Cancelled)
    })?;
    if is_cancelled {
        bail!("file upload cancelled");
    }
    Ok(())
}

fn mark_upload_failed(
    this: &gpui::WeakEntity<UploadManager>,
    file_id: &str,
    error: String,
    cx: &mut gpui::AsyncApp,
) -> Result<()> {
    this.update(cx, |this, cx| {
        if let Some(upload) = this.active_uploads.get_mut(file_id) {
            upload.status = UploadStatus::Failed(error);
        }
        cx.notify();
    })
}

fn mime_type_for_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("gif") => "image/gif",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("pdf") => "application/pdf",
        Some("mp3") => "audio/mpeg",
        Some("mp4") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("txt") => "text/plain",
        Some("md") => "text/markdown",
        Some("json") => "application/json",
        Some("rs") => "text/rust",
        Some("ts" | "tsx") => "text/typescript",
        Some("js" | "jsx") => "text/javascript",
        _ => "application/octet-stream",
    }
    .to_string()
}
