use anyhow::{Context as _, Result, bail};
use client::{ChannelId, Client, FileAttachment, FileUploadUrl, GetFileUploadUrl};
use collections::HashMap;
use futures::{FutureExt as _, future::BoxFuture};
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
    backend: Arc<dyn FileUploadBackend>,
    active_uploads: HashMap<FileId, UploadProgress>,
    next_pending_upload_id: u64,
}

impl UploadManager {
    pub fn new(client: Arc<Client>) -> Self {
        Self::new_with_backend(Arc::new(ClientFileUploadBackend { client }))
    }

    fn new_with_backend(backend: Arc<dyn FileUploadBackend>) -> Self {
        Self {
            backend,
            active_uploads: HashMap::default(),
            next_pending_upload_id: 0,
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
        let backend = self.backend.clone();
        let pending_file_id = self.next_pending_file_id();
        let filename = match filename_for_path(&file_path) {
            Ok(filename) => filename,
            Err(error) => return Task::ready(Err(error)),
        };
        self.active_uploads.insert(
            pending_file_id.clone(),
            UploadProgress {
                file_id: pending_file_id.clone(),
                channel_id,
                filename: SharedString::from(filename.clone()),
                file_path: file_path.clone(),
                progress: 0.0,
                status: UploadStatus::Pending,
            },
        );
        cx.notify();

        cx.spawn(async move |this, cx| {
            let mime_type = mime_type_for_path(&file_path);
            let read_result = cx
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
                .await;
            let (file_size, bytes) = match read_result {
                Ok(file) => file,
                Err(error) => {
                    mark_upload_failed(&this, &pending_file_id, format!("{error:#}"), cx)?;
                    return Err(error);
                }
            };

            update_upload_status(
                &this,
                &pending_file_id,
                UploadStatus::RequestingUrl,
                0.0,
                cx,
            )?;
            ensure_not_cancelled(&this, &pending_file_id, cx)?;

            let upload_url = match backend
                .get_file_upload_url(GetFileUploadUrl {
                    channel_id,
                    filename: filename.clone(),
                    file_size,
                    mime_type,
                })
                .await
            {
                Ok(upload_url) => upload_url,
                Err(error) => {
                    mark_upload_failed(&this, &pending_file_id, format!("{error:#}"), cx)?;
                    return Err(error);
                }
            };
            ensure_not_cancelled(&this, &pending_file_id, cx)?;
            let file_id = upload_url.file_id.clone();

            this.update(cx, |this, cx| {
                this.active_uploads.remove(&pending_file_id);
                this.active_uploads.insert(
                    file_id.clone(),
                    UploadProgress {
                        file_id: file_id.clone(),
                        channel_id,
                        filename: SharedString::from(filename.clone()),
                        file_path: file_path.clone(),
                        progress: 0.0,
                        status: UploadStatus::Uploading,
                    },
                );
                cx.notify();
            })?;
            ensure_not_cancelled(&this, &file_id, cx)?;

            let upload_result = backend.upload_file_to_s3(upload_url, bytes).await;
            if let Err(error) = upload_result {
                mark_upload_failed(&this, &file_id, format!("{error:#}"), cx)?;
                return Err(error);
            }

            ensure_not_cancelled(&this, &file_id, cx)?;
            update_upload_status(&this, &file_id, UploadStatus::Confirming, 1.0, cx)?;

            let attachment = match backend.confirm_file_upload(file_id.clone()).await {
                Ok(attachment) => attachment,
                Err(error) => {
                    mark_upload_failed(&this, &file_id, format!("{error:#}"), cx)?;
                    return Err(error);
                }
            };

            update_upload_status(&this, &file_id, UploadStatus::Completed, 1.0, cx)?;
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

    fn next_pending_file_id(&mut self) -> FileId {
        let file_id = format!("pending-upload-{}", self.next_pending_upload_id);
        self.next_pending_upload_id += 1;
        file_id
    }
}

struct GlobalUploadManager(Entity<UploadManager>);

impl Global for GlobalUploadManager {}

trait FileUploadBackend: Send + Sync {
    fn get_file_upload_url(
        &self,
        request: GetFileUploadUrl,
    ) -> BoxFuture<'static, Result<FileUploadUrl>>;
    fn upload_file_to_s3(
        &self,
        upload_url: FileUploadUrl,
        bytes: Vec<u8>,
    ) -> BoxFuture<'static, Result<()>>;
    fn confirm_file_upload(&self, file_id: String) -> BoxFuture<'static, Result<FileAttachment>>;
}

struct ClientFileUploadBackend {
    client: Arc<Client>,
}

impl FileUploadBackend for ClientFileUploadBackend {
    fn get_file_upload_url(
        &self,
        request: GetFileUploadUrl,
    ) -> BoxFuture<'static, Result<FileUploadUrl>> {
        let client = self.client.clone();
        async move { client.get_file_upload_url(request).await }.boxed()
    }

    fn upload_file_to_s3(
        &self,
        upload_url: FileUploadUrl,
        bytes: Vec<u8>,
    ) -> BoxFuture<'static, Result<()>> {
        let client = self.client.clone();
        async move {
            client
                .upload_file_to_s3(&upload_url, bytes, |_, _| {})
                .await
        }
        .boxed()
    }

    fn confirm_file_upload(&self, file_id: String) -> BoxFuture<'static, Result<FileAttachment>> {
        let client = self.client.clone();
        async move { client.confirm_file_upload(file_id).await }.boxed()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UploadProgress {
    pub file_id: FileId,
    pub channel_id: ChannelId,
    pub filename: SharedString,
    pub file_path: PathBuf,
    pub progress: f32,
    pub status: UploadStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UploadStatus {
    Pending,
    RequestingUrl,
    Uploading,
    Confirming,
    Completed,
    Failed(String),
    Cancelled,
}

fn filename_for_path(file_path: &Path) -> Result<String> {
    file_path
        .file_name()
        .and_then(|filename| filename.to_str())
        .context("file path has no valid filename")
        .map(str::to_string)
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

fn update_upload_status(
    this: &gpui::WeakEntity<UploadManager>,
    file_id: &str,
    status: UploadStatus,
    progress: f32,
    cx: &mut gpui::AsyncApp,
) -> Result<()> {
    this.update(cx, |this, cx| {
        if let Some(upload) = this.active_uploads.get_mut(file_id) {
            upload.progress = progress;
            upload.status = status;
        }
        cx.notify();
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::oneshot;
    use gpui::TestAppContext;
    use std::{
        collections::VecDeque,
        fs,
        sync::{Arc, Mutex},
    };

    #[gpui::test]
    async fn upload_file_tracks_state_machine(cx: &mut TestAppContext) {
        let backend = Arc::new(TestUploadBackend::default());
        let upload = backend.queue_upload();
        let channel_id = ChannelId(7);
        let file_path = write_test_file("state-machine.txt", b"hello");
        let manager = cx.update(|cx| cx.new(|_| UploadManager::new_with_backend(backend.clone())));

        let task = cx.update(|cx| {
            manager.update(cx, |manager, cx| {
                manager.upload_file(channel_id, file_path.clone(), cx)
            })
        });
        assert_upload_status(
            cx,
            &manager,
            channel_id,
            "pending-upload-0",
            UploadStatus::Pending,
        );

        cx.run_until_parked();
        assert_upload_status(
            cx,
            &manager,
            channel_id,
            "pending-upload-0",
            UploadStatus::RequestingUrl,
        );
        assert_eq!(
            backend.upload_url_requests(),
            vec![GetFileUploadUrl {
                channel_id,
                filename: "state-machine.txt".to_string(),
                file_size: 5,
                mime_type: "text/plain".to_string(),
            }]
        );

        upload
            .upload_url
            .send(Ok(upload_url("file-state-machine")))
            .expect("send upload url");
        cx.run_until_parked();
        assert_upload_status(
            cx,
            &manager,
            channel_id,
            "file-state-machine",
            UploadStatus::Uploading,
        );
        assert_eq!(
            backend.upload_requests(),
            vec![("file-state-machine".to_string(), b"hello".to_vec())]
        );

        upload.upload.send(Ok(())).expect("send upload completion");
        cx.run_until_parked();
        assert_upload_status(
            cx,
            &manager,
            channel_id,
            "file-state-machine",
            UploadStatus::Confirming,
        );
        assert_eq!(
            backend.confirm_requests(),
            vec!["file-state-machine".to_string()]
        );

        upload
            .confirm
            .send(Ok(file_attachment("file-state-machine")))
            .expect("send confirmation");
        let attachment = task.await.expect("upload completes");
        assert_eq!(attachment.id, "file-state-machine");
        assert_upload_status(
            cx,
            &manager,
            channel_id,
            "file-state-machine",
            UploadStatus::Completed,
        );
    }

    #[gpui::test]
    async fn cancellation_keeps_upload_cancelled(cx: &mut TestAppContext) {
        let backend = Arc::new(TestUploadBackend::default());
        let upload = backend.queue_upload();
        let channel_id = ChannelId(7);
        let file_path = write_test_file("cancel.txt", b"cancel me");
        let manager = cx.update(|cx| cx.new(|_| UploadManager::new_with_backend(backend.clone())));

        let task = cx.update(|cx| {
            manager.update(cx, |manager, cx| {
                manager.upload_file(channel_id, file_path.clone(), cx)
            })
        });
        cx.run_until_parked();
        upload
            .upload_url
            .send(Ok(upload_url("file-cancel")))
            .expect("send upload url");
        cx.run_until_parked();

        cx.update(|cx| {
            manager.update(cx, |manager, cx| {
                assert!(manager.cancel_upload("file-cancel", cx));
            });
        });
        assert_upload_status(
            cx,
            &manager,
            channel_id,
            "file-cancel",
            UploadStatus::Cancelled,
        );

        upload.upload.send(Ok(())).expect("send upload completion");
        let error = task.await.expect_err("cancelled upload fails");
        assert!(format!("{error:#}").contains("file upload cancelled"));
        assert_upload_status(
            cx,
            &manager,
            channel_id,
            "file-cancel",
            UploadStatus::Cancelled,
        );
    }

    #[gpui::test]
    async fn concurrent_uploads_are_tracked_separately(cx: &mut TestAppContext) {
        let backend = Arc::new(TestUploadBackend::default());
        let upload_a = backend.queue_upload();
        let upload_b = backend.queue_upload();
        let channel_id = ChannelId(7);
        let file_a = write_test_file("concurrent-a.txt", b"a");
        let file_b = write_test_file("concurrent-b.txt", b"bb");
        let manager = cx.update(|cx| cx.new(|_| UploadManager::new_with_backend(backend.clone())));

        let _task_a = cx.update(|cx| {
            manager.update(cx, |manager, cx| {
                manager.upload_file(channel_id, file_a.clone(), cx)
            })
        });
        let _task_b = cx.update(|cx| {
            manager.update(cx, |manager, cx| {
                manager.upload_file(channel_id, file_b.clone(), cx)
            })
        });
        cx.run_until_parked();
        assert_eq!(
            upload_ids_for_channel(cx, &manager, channel_id),
            vec![
                "pending-upload-0".to_string(),
                "pending-upload-1".to_string()
            ]
        );

        upload_a
            .upload_url
            .send(Ok(upload_url("file-a")))
            .expect("send upload url a");
        upload_b
            .upload_url
            .send(Ok(upload_url("file-b")))
            .expect("send upload url b");
        cx.run_until_parked();

        assert_eq!(
            upload_ids_for_channel(cx, &manager, channel_id),
            vec!["file-a".to_string(), "file-b".to_string()]
        );
        assert_upload_status(cx, &manager, channel_id, "file-a", UploadStatus::Uploading);
        assert_upload_status(cx, &manager, channel_id, "file-b", UploadStatus::Uploading);
    }

    #[derive(Default)]
    struct TestUploadBackend {
        state: Mutex<TestUploadBackendState>,
    }

    #[derive(Default)]
    struct TestUploadBackendState {
        upload_url_requests: Vec<GetFileUploadUrl>,
        upload_requests: Vec<(String, Vec<u8>)>,
        confirm_requests: Vec<String>,
        upload_url_responses: VecDeque<oneshot::Receiver<Result<FileUploadUrl>>>,
        upload_responses: VecDeque<oneshot::Receiver<Result<()>>>,
        confirm_responses: VecDeque<oneshot::Receiver<Result<FileAttachment>>>,
    }

    struct QueuedUpload {
        upload_url: oneshot::Sender<Result<FileUploadUrl>>,
        upload: oneshot::Sender<Result<()>>,
        confirm: oneshot::Sender<Result<FileAttachment>>,
    }

    impl TestUploadBackend {
        fn queue_upload(&self) -> QueuedUpload {
            let (upload_url_sender, upload_url_receiver) = oneshot::channel();
            let (upload_sender, upload_receiver) = oneshot::channel();
            let (confirm_sender, confirm_receiver) = oneshot::channel();
            let mut state = self.state.lock().expect("lock test backend");
            state.upload_url_responses.push_back(upload_url_receiver);
            state.upload_responses.push_back(upload_receiver);
            state.confirm_responses.push_back(confirm_receiver);
            QueuedUpload {
                upload_url: upload_url_sender,
                upload: upload_sender,
                confirm: confirm_sender,
            }
        }

        fn upload_url_requests(&self) -> Vec<GetFileUploadUrl> {
            self.state
                .lock()
                .expect("lock test backend")
                .upload_url_requests
                .clone()
        }

        fn upload_requests(&self) -> Vec<(String, Vec<u8>)> {
            self.state
                .lock()
                .expect("lock test backend")
                .upload_requests
                .clone()
        }

        fn confirm_requests(&self) -> Vec<String> {
            self.state
                .lock()
                .expect("lock test backend")
                .confirm_requests
                .clone()
        }
    }

    impl FileUploadBackend for TestUploadBackend {
        fn get_file_upload_url(
            &self,
            request: GetFileUploadUrl,
        ) -> BoxFuture<'static, Result<FileUploadUrl>> {
            let response = {
                let mut state = self.state.lock().expect("lock test backend");
                state.upload_url_requests.push(request);
                state
                    .upload_url_responses
                    .pop_front()
                    .expect("queued upload url response")
            };
            async move { response.await.context("upload URL sender dropped")? }.boxed()
        }

        fn upload_file_to_s3(
            &self,
            upload_url: FileUploadUrl,
            bytes: Vec<u8>,
        ) -> BoxFuture<'static, Result<()>> {
            let response = {
                let mut state = self.state.lock().expect("lock test backend");
                state
                    .upload_requests
                    .push((upload_url.file_id.clone(), bytes));
                state
                    .upload_responses
                    .pop_front()
                    .expect("queued upload response")
            };
            async move { response.await.context("upload sender dropped")? }.boxed()
        }

        fn confirm_file_upload(
            &self,
            file_id: String,
        ) -> BoxFuture<'static, Result<FileAttachment>> {
            let response = {
                let mut state = self.state.lock().expect("lock test backend");
                state.confirm_requests.push(file_id);
                state
                    .confirm_responses
                    .pop_front()
                    .expect("queued confirm response")
            };
            async move { response.await.context("confirm sender dropped")? }.boxed()
        }
    }

    fn assert_upload_status(
        cx: &mut TestAppContext,
        manager: &Entity<UploadManager>,
        channel_id: ChannelId,
        file_id: &str,
        status: UploadStatus,
    ) {
        let upload = cx.update(|cx| {
            manager
                .read(cx)
                .uploads_for_channel(channel_id)
                .into_iter()
                .find(|upload| upload.file_id == file_id)
                .with_context(|| format!("missing upload {file_id}"))
        });
        assert_eq!(upload.expect("upload exists").status, status);
    }

    fn upload_ids_for_channel(
        cx: &mut TestAppContext,
        manager: &Entity<UploadManager>,
        channel_id: ChannelId,
    ) -> Vec<String> {
        let mut file_ids = cx.update(|cx| {
            manager
                .read(cx)
                .uploads_for_channel(channel_id)
                .into_iter()
                .map(|upload| upload.file_id)
                .collect::<Vec<_>>()
        });
        file_ids.sort();
        file_ids
    }

    fn write_test_file(name: &str, bytes: &[u8]) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("sim-upload-manager-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create test directory");
        let path = directory.join(name);
        fs::write(&path, bytes).expect("write test file");
        path
    }

    fn upload_url(file_id: &str) -> FileUploadUrl {
        FileUploadUrl {
            url: format!("https://example.com/upload/{file_id}"),
            file_id: file_id.to_string(),
            headers: Default::default(),
        }
    }

    fn file_attachment(file_id: &str) -> FileAttachment {
        FileAttachment {
            id: file_id.to_string(),
            filename: format!("{file_id}.txt"),
            file_size: 5,
            mime_type: "text/plain".to_string(),
            url: format!("https://example.com/download/{file_id}"),
            uploader_id: 1,
            uploaded_at: None,
            image_width: None,
            image_height: None,
            duration_ms: None,
            thumbnail_url: None,
        }
    }
}
