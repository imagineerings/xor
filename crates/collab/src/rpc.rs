mod connection_pool;

use crate::api::{CloudflareIpCountryHeader, SystemIdHeader};
use crate::db::bookmark_store::{BookmarkStore, BookmarkUpdate, NewBookmark};
use crate::db::file_store::{FileStore, FileStoreConfig, FileStoreError, NewFileUpload};
use crate::db::join_request_store::JoinRequestStore;
use crate::db::queries::channel_messages::{
    ChannelMessageUpdate, NewChannelMessage, SearchChannelMessagesParams,
};
use crate::db::scheduled_message_store::{
    NewScheduledMessage, ScheduledMessageStore, ScheduledMessageUpdate,
};
use crate::db::user_status_store::{UserCustomStatus, UserStatusStore};
use crate::entities::User;
use crate::{
    AppState, Error, Result, auth,
    db::{
        self, BufferId, Capability, Channel, ChannelId, ChannelRole, ChannelsForUser, Database,
        GroupId, InviteMemberResult, MembershipUpdated, MessageId, NotificationId, ProjectId,
        RejoinedProject, RemoveChannelMemberResult, RespondToChannelInvite, RoomId, ServerId,
        SharedThreadId, UserId,
    },
    executor::Executor,
};
use anyhow::{Context as _, anyhow, bail};
use async_tungstenite::tungstenite::{
    Message as TungsteniteMessage, protocol::CloseFrame as TungsteniteCloseFrame,
};
use axum::headers::UserAgent;
use axum::{
    Extension, Router, TypedHeader,
    body::Body,
    extract::{
        ConnectInfo, WebSocketUpgrade,
        ws::{CloseFrame as AxumCloseFrame, Message as AxumMessage},
    },
    headers::{Header, HeaderName},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::get,
};
use collections::{HashMap, HashSet, TypeIdHashMap};
pub use connection_pool::{ConnectionPool, SimVersion};
use core::fmt::{self, Debug, Formatter};
use futures::TryFutureExt as _;
use rpc::proto::split_repository_update;
use tracing::Span;
use util::paths::PathStyle;

use futures::{
    FutureExt, SinkExt, StreamExt, TryStreamExt,
    channel::oneshot,
    future::BoxFuture,
    stream::{BoxStream, FuturesUnordered},
};
use prometheus::{IntGauge, register_int_gauge};
use rand::Rng as _;
use rpc::{
    Connection, ConnectionId, ErrorCode, ErrorCodeExt, ErrorExt, Notification, Peer, Receipt,
    TypedEnvelope,
    proto::{
        self, Ack, AnyTypedEnvelope, EntityMessage, EnvelopedMessage, LiveKitConnectionInfo,
        RequestMessage, ShareProject, UpdateChannelBufferCollaborators,
    },
};
use sea_orm::{ColumnTrait as _, EntityTrait as _, QueryFilter as _};
use semver::Version;
use std::{
    any::TypeId,
    future::Future,
    marker::PhantomData,
    mem,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
    },
    time::{Duration, Instant},
};
use time::PrimitiveDateTime;
use tokio::sync::{Semaphore, watch};
use tower::ServiceBuilder;
use tracing::{
    Instrument,
    field::{self},
    info_span, instrument,
};

pub const RECONNECT_TIMEOUT: Duration = Duration::from_secs(30);

// kubernetes gives terminated pods 10s to shutdown gracefully. After they're gone, we can clean up old resources.
pub const CLEANUP_TIMEOUT: Duration = Duration::from_secs(15);

const NOTIFICATION_COUNT_PER_PAGE: usize = 50;
const MAX_CONCURRENT_CONNECTIONS: usize = 512;
const SCHEDULED_MESSAGE_POLL_INTERVAL: Duration = Duration::from_secs(10);
const SCHEDULED_MESSAGE_STALE_PROCESSING_GRACE: Duration = Duration::from_secs(60);
const JOIN_REQUEST_REASON_MAX_CHARS: usize = 500;
const JOIN_REQUEST_RATE_LIMIT: usize = 10;
const JOIN_REQUEST_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const BOOKMARK_REORDER_BROADCAST_DEBOUNCE: Duration = Duration::from_millis(200);
const DEFAULT_FILE_UPLOAD_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

static CONCURRENT_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

const TOTAL_DURATION_MS: &str = "total_duration_ms";
const PROCESSING_DURATION_MS: &str = "processing_duration_ms";
const QUEUE_DURATION_MS: &str = "queue_duration_ms";
const HOST_WAITING_MS: &str = "host_waiting_ms";

type MessageHandler =
    Box<dyn Send + Sync + Fn(Box<dyn AnyTypedEnvelope>, Session, Span) -> BoxFuture<'static, ()>>;

pub struct ConnectionGuard;

impl ConnectionGuard {
    pub fn try_acquire() -> Result<Self, ()> {
        let current_connections = CONCURRENT_CONNECTIONS.fetch_add(1, SeqCst);
        if current_connections >= MAX_CONCURRENT_CONNECTIONS {
            CONCURRENT_CONNECTIONS.fetch_sub(1, SeqCst);
            tracing::error!(
                "too many concurrent connections: {}",
                current_connections + 1
            );
            return Err(());
        }
        Ok(ConnectionGuard)
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        CONCURRENT_CONNECTIONS.fetch_sub(1, SeqCst);
    }
}

struct Response<R> {
    peer: Arc<Peer>,
    receipt: Receipt<R>,
    responded: Arc<AtomicBool>,
}

impl<R: RequestMessage> Response<R> {
    fn send(self, payload: R::Response) -> Result<()> {
        self.responded.store(true, SeqCst);
        self.peer.respond(self.receipt, payload)?;
        Ok(())
    }
}

struct StreamResponse<R> {
    peer: Arc<Peer>,
    receipt: Receipt<R>,
    ended: Arc<AtomicBool>,
}

impl<R: RequestMessage> StreamResponse<R> {
    fn send(&self, payload: R::Response) -> Result<()> {
        self.peer.respond(self.receipt, payload)?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        // Always mark `ended` even if sending `EndStream` on the wire fails, so that
        // `ended` reflects "the handler intended to end the stream". The caller still
        // gets the underlying error and routes through the Err arm of the handler,
        // which sends `respond_with_error` to terminate the client-side stream.
        let result = self.peer.end_stream(self.receipt);
        self.ended.store(true, SeqCst);
        result?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Principal {
    User(User),
}

impl Principal {
    fn update_span(&self, span: &tracing::Span) {
        match &self {
            Principal::User(user) => {
                span.record("user_id", user.id.0);
                span.record("login", &user.github_login);
            }
        }
    }
}

#[derive(Clone)]
struct MessageContext {
    session: Session,
    span: tracing::Span,
}

impl Deref for MessageContext {
    type Target = Session;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl MessageContext {
    pub fn forward_request<T: RequestMessage>(
        &self,
        receiver_id: ConnectionId,
        request: T,
    ) -> impl Future<Output = anyhow::Result<T::Response>> {
        let request_start_time = Instant::now();
        let span = self.span.clone();
        tracing::info!("start forwarding request");
        self.peer
            .forward_request(self.connection_id, receiver_id, request)
            .inspect(move |_| {
                span.record(
                    HOST_WAITING_MS,
                    request_start_time.elapsed().as_micros() as f64 / 1000.0,
                );
            })
            .inspect_err(|_| tracing::error!("error forwarding request"))
            .inspect_ok(|_| tracing::info!("finished forwarding request"))
    }

    pub fn forward_request_stream<T: RequestMessage>(
        &self,
        receiver_id: ConnectionId,
        request: T,
    ) -> impl Future<Output = anyhow::Result<BoxStream<'static, anyhow::Result<T::Response>>>> {
        let request_start_time = Instant::now();
        let span = self.span.clone();
        let peer = self.peer.clone();
        let envelope = request.into_envelope(0, None, Some(self.connection_id.into()));
        async move {
            tracing::info!("start forwarding stream request");
            let stream = peer
                .request_stream_dynamic(receiver_id, envelope, T::NAME)
                .await;
            span.record(
                HOST_WAITING_MS,
                request_start_time.elapsed().as_micros() as f64 / 1000.0,
            );
            let stream = stream
                .inspect_err(|_| tracing::error!("error forwarding stream request"))?
                .map(|response| {
                    T::Response::from_envelope(response?)
                        .context("received response of the wrong type")
                })
                .boxed();
            tracing::info!("finished opening forwarded stream request");
            Ok(stream)
        }
    }
}

#[derive(Clone)]
struct Session {
    principal: Principal,
    connection_id: ConnectionId,
    db: Arc<tokio::sync::Mutex<DbHandle>>,
    peer: Arc<Peer>,
    connection_pool: Arc<parking_lot::Mutex<ConnectionPool>>,
    app_state: Arc<AppState>,
    /// The GeoIP country code for the user.
    #[allow(unused)]
    geoip_country_code: Option<String>,
    #[allow(unused)]
    system_id: Option<String>,
    _executor: Executor,
}

impl Session {
    async fn db(&self) -> tokio::sync::MutexGuard<'_, DbHandle> {
        #[cfg(feature = "test-support")]
        tokio::task::yield_now().await;
        let guard = self.db.lock().await;
        #[cfg(feature = "test-support")]
        tokio::task::yield_now().await;
        guard
    }

    async fn connection_pool(&self) -> ConnectionPoolGuard<'_> {
        #[cfg(feature = "test-support")]
        tokio::task::yield_now().await;
        let guard = self.connection_pool.lock();
        ConnectionPoolGuard {
            guard,
            _not_send: PhantomData,
        }
    }

    #[expect(dead_code)]
    fn is_staff(&self) -> bool {
        match &self.principal {
            Principal::User(user) => user.admin,
        }
    }

    fn user_id(&self) -> UserId {
        match &self.principal {
            Principal::User(user) => user.id,
        }
    }
}

impl Debug for Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut result = f.debug_struct("Session");
        match &self.principal {
            Principal::User(user) => {
                result.field("user", &user.github_login);
            }
        }
        result.field("connection_id", &self.connection_id).finish()
    }
}

struct DbHandle(Arc<Database>);

impl Deref for DbHandle {
    type Target = Database;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

pub struct Server {
    id: parking_lot::Mutex<ServerId>,
    peer: Arc<Peer>,
    pub connection_pool: Arc<parking_lot::Mutex<ConnectionPool>>,
    app_state: Arc<AppState>,
    handlers: TypeIdHashMap<MessageHandler>,
    teardown: watch::Sender<bool>,
}

struct ConnectionPoolGuard<'a> {
    guard: parking_lot::MutexGuard<'a, ConnectionPool>,
    _not_send: PhantomData<Rc<()>>,
}

impl Server {
    pub fn new(id: ServerId, app_state: Arc<AppState>) -> Arc<Self> {
        let mut server = Self {
            id: parking_lot::Mutex::new(id),
            peer: Peer::new(id.0 as u32),
            app_state,
            connection_pool: Default::default(),
            handlers: Default::default(),
            teardown: watch::channel(false).0,
        };

        server
            .add_request_handler(ping)
            .add_request_handler(create_room)
            .add_request_handler(join_room)
            .add_request_handler(rejoin_room)
            .add_request_handler(leave_room)
            .add_request_handler(set_room_participant_role)
            .add_request_handler(call)
            .add_request_handler(cancel_call)
            .add_message_handler(decline_call)
            .add_request_handler(update_participant_location)
            .add_request_handler(share_project)
            .add_message_handler(unshare_project)
            .add_request_handler(join_project)
            .add_message_handler(leave_project)
            .add_request_handler(update_project)
            .add_request_handler(update_worktree)
            .add_request_handler(update_repository)
            .add_request_handler(remove_repository)
            .add_message_handler(start_language_server)
            .add_message_handler(update_language_server)
            .add_message_handler(update_diagnostic_summary)
            .add_message_handler(update_worktree_settings)
            .add_request_handler(forward_read_only_project_request::<proto::FindSearchCandidates>)
            .add_request_handler(forward_read_only_project_request::<proto::GetDocumentHighlights>)
            .add_request_handler(forward_read_only_project_request::<proto::GetDocumentSymbols>)
            .add_request_handler(forward_read_only_project_request::<proto::GetProjectSymbols>)
            .add_request_handler(forward_read_only_project_request::<proto::OpenBufferForSymbol>)
            .add_request_handler(forward_read_only_project_request::<proto::OpenBufferById>)
            .add_request_handler(forward_read_only_project_request::<proto::SynchronizeBuffers>)
            .add_request_handler(forward_read_only_project_request::<proto::ResolveInlayHint>)
            .add_request_handler(forward_read_only_project_request::<proto::ResolveCodeAction>)
            .add_request_handler(forward_read_only_project_request::<proto::ResolveDocumentLink>)
            .add_request_handler(forward_read_only_project_request::<proto::GetColorPresentation>)
            .add_request_handler(forward_read_only_project_request::<proto::OpenBufferByPath>)
            .add_request_handler(forward_read_only_project_request::<proto::OpenImageByPath>)
            .add_request_handler(forward_read_only_project_request::<proto::DownloadFileByPath>)
            .add_request_handler(forward_read_only_project_request::<proto::GitGetBranches>)
            .add_request_handler(forward_read_only_project_request::<proto::GetDefaultBranch>)
            .add_request_handler(forward_read_only_project_request::<proto::OpenUnstagedDiff>)
            .add_request_handler(forward_read_only_project_request::<proto::OpenUncommittedDiff>)
            .add_request_handler(forward_read_only_project_request::<proto::LspExtExpandMacro>)
            .add_request_handler(forward_read_only_project_request::<proto::LspExtOpenDocs>)
            .add_request_handler(forward_mutating_project_request::<proto::LspExtRunnables>)
            .add_request_handler(
                forward_read_only_project_request::<proto::LspExtSwitchSourceHeader>,
            )
            .add_request_handler(forward_read_only_project_request::<proto::LspExtGoToParentModule>)
            .add_request_handler(forward_read_only_project_request::<proto::LspExtCancelFlycheck>)
            .add_request_handler(forward_read_only_project_request::<proto::LspExtRunFlycheck>)
            .add_request_handler(forward_read_only_project_request::<proto::LspExtClearFlycheck>)
            .add_request_handler(
                forward_mutating_project_request::<proto::RegisterBufferWithLanguageServers>,
            )
            .add_request_handler(forward_mutating_project_request::<proto::UpdateGitBranch>)
            .add_request_handler(forward_mutating_project_request::<proto::GetCompletions>)
            .add_request_handler(
                forward_mutating_project_request::<proto::ApplyCompletionAdditionalEdits>,
            )
            .add_request_handler(forward_mutating_project_request::<proto::OpenNewBuffer>)
            .add_request_handler(
                forward_mutating_project_request::<proto::ResolveCompletionDocumentation>,
            )
            .add_request_handler(forward_mutating_project_request::<proto::ApplyCodeAction>)
            .add_request_handler(forward_mutating_project_request::<proto::PrepareRename>)
            .add_request_handler(forward_mutating_project_request::<proto::PerformRename>)
            .add_request_handler(forward_mutating_project_request::<proto::ReloadBuffers>)
            .add_request_handler(forward_mutating_project_request::<proto::ApplyCodeActionKind>)
            .add_request_handler(forward_mutating_project_request::<proto::FormatBuffers>)
            .add_request_handler(forward_mutating_project_request::<proto::CreateProjectEntry>)
            .add_request_handler(forward_mutating_project_request::<proto::RenameProjectEntry>)
            .add_request_handler(forward_mutating_project_request::<proto::CopyProjectEntry>)
            .add_request_handler(forward_mutating_project_request::<proto::DeleteProjectEntry>)
            .add_request_handler(forward_mutating_project_request::<proto::ExpandProjectEntry>)
            .add_request_handler(
                forward_mutating_project_request::<proto::ExpandAllForProjectEntry>,
            )
            .add_request_handler(forward_mutating_project_request::<proto::OnTypeFormatting>)
            .add_request_handler(forward_mutating_project_request::<proto::SaveBuffer>)
            .add_request_handler(forward_mutating_project_request::<proto::BlameBuffer>)
            .add_request_handler(lsp_query)
            .add_message_handler(broadcast_project_message_from_host::<proto::LspQueryResponse>)
            .add_request_handler(forward_mutating_project_request::<proto::RestartLanguageServers>)
            .add_request_handler(forward_mutating_project_request::<proto::StopLanguageServers>)
            .add_request_handler(forward_mutating_project_request::<proto::LinkedEditingRange>)
            .add_message_handler(create_buffer_for_peer)
            .add_message_handler(create_image_for_peer)
            .add_request_handler(update_buffer)
            .add_message_handler(broadcast_project_message_from_host::<proto::RefreshInlayHints>)
            .add_message_handler(
                broadcast_project_message_from_host::<proto::RefreshSemanticTokens>,
            )
            .add_message_handler(broadcast_project_message_from_host::<proto::RefreshCodeLens>)
            .add_message_handler(broadcast_project_message_from_host::<proto::UpdateBufferFile>)
            .add_message_handler(broadcast_project_message_from_host::<proto::BufferReloaded>)
            .add_message_handler(broadcast_project_message_from_host::<proto::BufferSaved>)
            .add_message_handler(broadcast_project_message_from_host::<proto::UpdateDiffBases>)
            .add_message_handler(
                broadcast_project_message_from_host::<proto::PullWorkspaceDiagnostics>,
            )
            .add_request_handler(get_users)
            .add_request_handler(fuzzy_search_users)
            .add_request_handler(request_contact)
            .add_request_handler(remove_contact)
            .add_request_handler(respond_to_contact_request)
            .add_request_handler(set_status)
            .add_request_handler(clear_status)
            .add_request_handler(respond_to_join_request)
            .add_request_handler(request_join_channel)
            .add_message_handler(subscribe_to_channels)
            .add_request_handler(create_channel)
            .add_request_handler(create_group)
            .add_request_handler(update_group)
            .add_request_handler(delete_group)
            .add_request_handler(get_groups)
            .add_request_handler(update_group_members)
            .add_request_handler(leave_group)
            .add_request_handler(delete_channel)
            .add_request_handler(invite_channel_member)
            .add_request_handler(remove_channel_member)
            .add_request_handler(set_channel_member_role)
            .add_request_handler(set_channel_visibility)
            .add_request_handler(rename_channel)
            .add_request_handler(join_channel_buffer)
            .add_request_handler(leave_channel_buffer)
            .add_message_handler(update_channel_buffer)
            .add_request_handler(rejoin_channel_buffers)
            .add_request_handler(get_channel_members)
            .add_request_handler(respond_to_channel_invite)
            .add_request_handler(join_channel)
            .add_request_handler(join_channel_chat)
            .add_message_handler(leave_channel_chat)
            .add_request_handler(send_channel_message)
            .add_request_handler(schedule_channel_message)
            .add_request_handler(cancel_scheduled_message)
            .add_request_handler(update_scheduled_message)
            .add_request_handler(get_scheduled_messages)
            .add_request_handler(get_bookmarks)
            .add_request_handler(get_pending_join_requests)
            .add_request_handler(get_file_upload_url)
            .add_request_handler(confirm_file_upload)
            .add_request_handler(get_file_download_url)
            .add_request_handler(add_bookmark)
            .add_request_handler(remove_bookmark)
            .add_request_handler(update_bookmark)
            .add_request_handler(reorder_bookmarks)
            .add_request_handler(remove_channel_message)
            .add_request_handler(update_channel_message)
            .add_request_handler(add_reaction)
            .add_request_handler(remove_reaction)
            .add_request_handler(get_channel_messages)
            .add_request_handler(get_channel_messages_by_id)
            .add_request_handler(search_channel_messages)
            .add_request_handler(get_thread)
            .add_request_handler(get_threads)
            .add_request_handler(get_notifications)
            .add_request_handler(mark_notification_as_read)
            .add_request_handler(move_channel)
            .add_request_handler(reorder_channel)
            .add_request_handler(follow)
            .add_message_handler(unfollow)
            .add_message_handler(update_followers)
            .add_message_handler(acknowledge_channel_message)
            .add_message_handler(acknowledge_channel_thread)
            .add_message_handler(acknowledge_buffer_version)
            .add_request_handler(forward_mutating_project_request::<proto::Stage>)
            .add_request_handler(forward_mutating_project_request::<proto::Unstage>)
            .add_request_handler(forward_mutating_project_request::<proto::Stash>)
            .add_request_handler(forward_mutating_project_request::<proto::StashPop>)
            .add_request_handler(forward_mutating_project_request::<proto::StashDrop>)
            .add_request_handler(forward_mutating_project_request::<proto::Commit>)
            .add_request_handler(forward_mutating_project_request::<proto::RunGitHook>)
            .add_request_handler(forward_mutating_project_request::<proto::GitInit>)
            .add_request_handler(forward_read_only_project_request::<proto::GetRemotes>)
            .add_request_handler(forward_read_only_project_request::<proto::GitShow>)
            .add_request_handler(forward_read_only_project_request::<proto::LoadCommitDiff>)
            .add_request_handler(forward_read_only_project_request::<proto::GitReset>)
            .add_request_handler(forward_read_only_project_request::<proto::GitCheckoutFiles>)
            .add_request_handler(forward_mutating_project_request::<proto::SetIndexText>)
            .add_request_handler(forward_mutating_project_request::<proto::ToggleBreakpoint>)
            .add_message_handler(broadcast_project_message_from_host::<proto::BreakpointsForFile>)
            .add_request_handler(forward_mutating_project_request::<proto::OpenCommitMessageBuffer>)
            .add_request_handler(forward_mutating_project_request::<proto::GitDiff>)
            .add_request_handler(forward_mutating_project_request::<proto::GetTreeDiff>)
            .add_request_handler(forward_mutating_project_request::<proto::GetBlobContent>)
            .add_request_handler(forward_mutating_project_request::<proto::GitCreateBranch>)
            .add_request_handler(forward_mutating_project_request::<proto::GitChangeBranch>)
            .add_request_handler(forward_mutating_project_request::<proto::GitCreateRemote>)
            .add_request_handler(forward_mutating_project_request::<proto::GitRemoveRemote>)
            .add_request_handler(forward_read_only_project_request::<proto::GitGetWorktrees>)
            .add_request_handler(forward_read_only_project_request::<proto::GitGetHeadSha>)
            .add_request_handler(forward_read_only_project_request::<proto::GetCommitData>)
            .add_request_stream_handler(
                forward_read_only_project_stream_request::<proto::GetInitialGraphData>,
            )
            .add_request_stream_handler(
                forward_read_only_project_stream_request::<proto::SearchCommits>,
            )
            .add_request_handler(forward_mutating_project_request::<proto::GitCreateWorktree>)
            .add_request_handler(disallow_guest_request::<proto::GitRemoveWorktree>)
            .add_request_handler(disallow_guest_request::<proto::GitRenameWorktree>)
            .add_request_handler(forward_mutating_project_request::<proto::GitEditRef>)
            .add_request_handler(forward_mutating_project_request::<proto::GitRepairWorktrees>)
            .add_request_handler(disallow_guest_request::<proto::GitCreateArchiveCheckpoint>)
            .add_request_handler(disallow_guest_request::<proto::GitRestoreArchiveCheckpoint>)
            .add_request_handler(forward_mutating_project_request::<proto::CheckForPushedCommits>)
            .add_request_handler(forward_mutating_project_request::<proto::ToggleLspLogs>)
            .add_message_handler(broadcast_project_message_from_host::<proto::LanguageServerLog>)
            .add_request_handler(share_agent_thread)
            .add_request_handler(get_shared_agent_thread)
            .add_request_handler(forward_project_search_chunk);

        Arc::new(server)
    }

    pub async fn start(&self) -> Result<()> {
        let server_id = *self.id.lock();
        let app_state = self.app_state.clone();
        let peer = self.peer.clone();
        let timeout = self.app_state.executor.sleep(CLEANUP_TIMEOUT);
        let pool = self.connection_pool.clone();
        let livekit_client = self.app_state.livekit_client.clone();
        let scheduled_app_state = self.app_state.clone();
        let scheduled_peer = self.peer.clone();
        let scheduled_pool = self.connection_pool.clone();

        let span = info_span!("start server");
        self.app_state.executor.spawn_detached(
            run_scheduled_message_loop(scheduled_app_state, scheduled_peer, scheduled_pool)
                .instrument(span.clone()),
        );
        let join_request_app_state = self.app_state.clone();
        let join_request_peer = self.peer.clone();
        let join_request_pool = self.connection_pool.clone();
        self.app_state.executor.spawn_detached(
            run_join_request_expiry_loop(
                join_request_app_state,
                join_request_peer,
                join_request_pool,
            )
            .instrument(span.clone()),
        );
        crate::status_expiry_sweeper::StatusExpirySweeper::new(
            self.app_state.db.clone(),
            self.app_state.executor.clone(),
            self.peer.clone(),
            self.connection_pool.clone(),
        )
        .start();
        self.app_state.executor.spawn_detached(
            async move {
                tracing::info!("waiting for cleanup timeout");
                timeout.await;
                tracing::info!("cleanup timeout expired, retrieving stale rooms");

                app_state
                    .db
                    .delete_stale_channel_chat_participants(
                        &app_state.config.sim_environment,
                        server_id,
                    )
                    .await
                    .trace_err();

                if let Some((room_ids, channel_ids)) = app_state
                    .db
                    .stale_server_resource_ids(&app_state.config.sim_environment, server_id)
                    .await
                    .trace_err()
                {
                    tracing::info!(stale_room_count = room_ids.len(), "retrieved stale rooms");
                    tracing::info!(
                        stale_channel_buffer_count = channel_ids.len(),
                        "retrieved stale channel buffers"
                    );

                    for channel_id in channel_ids {
                        if let Some(refreshed_channel_buffer) = app_state
                            .db
                            .clear_stale_channel_buffer_collaborators(channel_id, server_id)
                            .await
                            .trace_err()
                        {
                            for connection_id in refreshed_channel_buffer.connection_ids {
                                peer.send(
                                    connection_id,
                                    proto::UpdateChannelBufferCollaborators {
                                        channel_id: channel_id.to_proto(),
                                        collaborators: refreshed_channel_buffer
                                            .collaborators
                                            .clone(),
                                    },
                                )
                                .trace_err();
                            }
                        }
                    }

                    for room_id in room_ids {
                        let mut contacts_to_update = HashSet::default();
                        let mut canceled_calls_to_user_ids = Vec::new();
                        let mut livekit_room = String::new();
                        let mut delete_livekit_room = false;

                        if let Some(mut refreshed_room) = app_state
                            .db
                            .clear_stale_room_participants(room_id, server_id)
                            .await
                            .trace_err()
                        {
                            tracing::info!(
                                room_id = room_id.0,
                                new_participant_count = refreshed_room.room.participants.len(),
                                "refreshed room"
                            );
                            room_updated(&refreshed_room.room, &peer);
                            if let Some(channel) = refreshed_room.channel.as_ref() {
                                channel_updated(channel, &refreshed_room.room, &peer, &pool.lock());
                            }
                            contacts_to_update
                                .extend(refreshed_room.stale_participant_user_ids.iter().copied());
                            contacts_to_update
                                .extend(refreshed_room.canceled_calls_to_user_ids.iter().copied());
                            canceled_calls_to_user_ids =
                                mem::take(&mut refreshed_room.canceled_calls_to_user_ids);
                            livekit_room = mem::take(&mut refreshed_room.room.livekit_room);
                            delete_livekit_room = refreshed_room.room.participants.is_empty();
                        }

                        {
                            let pool = pool.lock();
                            for canceled_user_id in canceled_calls_to_user_ids {
                                for connection_id in pool.user_connection_ids(canceled_user_id) {
                                    peer.send(
                                        connection_id,
                                        proto::CallCanceled {
                                            room_id: room_id.to_proto(),
                                        },
                                    )
                                    .trace_err();
                                }
                            }
                        }

                        for user_id in contacts_to_update {
                            let busy = app_state.db.is_user_busy(user_id).await.trace_err();
                            let contacts = app_state.db.get_contacts(user_id).await.trace_err();
                            if let Some((busy, contacts)) = busy.zip(contacts) {
                                let pool = pool.lock();
                                let updated_contact = contact_for_user(user_id, busy, &pool);
                                for contact in contacts {
                                    if let db::Contact::Accepted {
                                        user_id: contact_user_id,
                                        ..
                                    } = contact
                                    {
                                        for contact_conn_id in
                                            pool.user_connection_ids(contact_user_id)
                                        {
                                            peer.send(
                                                contact_conn_id,
                                                proto::UpdateContacts {
                                                    contacts: vec![updated_contact.clone()],
                                                    remove_contacts: Default::default(),
                                                    incoming_requests: Default::default(),
                                                    remove_incoming_requests: Default::default(),
                                                    outgoing_requests: Default::default(),
                                                    remove_outgoing_requests: Default::default(),
                                                },
                                            )
                                            .trace_err();
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(live_kit) = livekit_client.as_ref()
                            && delete_livekit_room
                        {
                            live_kit.delete_room(livekit_room).await.trace_err();
                        }
                    }
                }

                app_state
                    .db
                    .delete_stale_channel_chat_participants(
                        &app_state.config.sim_environment,
                        server_id,
                    )
                    .await
                    .trace_err();

                app_state
                    .db
                    .clear_old_worktree_entries(server_id)
                    .await
                    .trace_err();

                app_state
                    .db
                    .delete_stale_servers(&app_state.config.sim_environment, server_id)
                    .await
                    .trace_err();
            }
            .instrument(span),
        );
        Ok(())
    }

    pub fn teardown(&self) {
        self.peer.teardown();
        self.connection_pool.lock().reset();
        let _ = self.teardown.send(true);
    }

    #[cfg(feature = "test-support")]
    pub fn reset(&self, id: ServerId) {
        self.teardown();
        *self.id.lock() = id;
        self.peer.reset(id.0 as u32);
        let _ = self.teardown.send(false);
    }

    #[cfg(feature = "test-support")]
    pub fn id(&self) -> ServerId {
        *self.id.lock()
    }

    fn add_handler<F, Fut, M>(&mut self, handler: F) -> &mut Self
    where
        F: 'static + Send + Sync + Fn(TypedEnvelope<M>, MessageContext) -> Fut,
        Fut: 'static + Send + Future<Output = Result<()>>,
        M: EnvelopedMessage,
    {
        let prev_handler = self.handlers.insert(
            TypeId::of::<M>(),
            Box::new(move |envelope, session, span| {
                let envelope = envelope.into_any().downcast::<TypedEnvelope<M>>().unwrap();
                let received_at = envelope.received_at;
                tracing::info!("message received");
                let start_time = Instant::now();
                let future = (handler)(
                    *envelope,
                    MessageContext {
                        session,
                        span: span.clone(),
                    },
                );
                async move {
                    let result = future.await;
                    let total_duration_ms = received_at.elapsed().as_micros() as f64 / 1000.0;
                    let processing_duration_ms = start_time.elapsed().as_micros() as f64 / 1000.0;
                    let queue_duration_ms = total_duration_ms - processing_duration_ms;
                    span.record(TOTAL_DURATION_MS, total_duration_ms);
                    span.record(PROCESSING_DURATION_MS, processing_duration_ms);
                    span.record(QUEUE_DURATION_MS, queue_duration_ms);
                    match result {
                        Err(error) => {
                            tracing::error!(?error, "error handling message")
                        }
                        Ok(()) => tracing::info!("finished handling message"),
                    }
                }
                .boxed()
            }),
        );
        if prev_handler.is_some() {
            panic!("registered a handler for the same message twice");
        }
        self
    }

    fn add_message_handler<F, Fut, M>(&mut self, handler: F) -> &mut Self
    where
        F: 'static + Send + Sync + Fn(M, MessageContext) -> Fut,
        Fut: 'static + Send + Future<Output = Result<()>>,
        M: EnvelopedMessage,
    {
        self.add_handler(move |envelope, session| handler(envelope.payload, session));
        self
    }

    fn add_request_handler<F, Fut, M>(&mut self, handler: F) -> &mut Self
    where
        F: 'static + Send + Sync + Fn(M, Response<M>, MessageContext) -> Fut,
        Fut: Send + Future<Output = Result<()>>,
        M: RequestMessage,
    {
        let handler = Arc::new(handler);
        self.add_handler(move |envelope, session| {
            let receipt = envelope.receipt();
            let handler = handler.clone();
            async move {
                let peer = session.peer.clone();
                let responded = Arc::new(AtomicBool::default());
                let response = Response {
                    peer: peer.clone(),
                    responded: responded.clone(),
                    receipt,
                };
                match (handler)(envelope.payload, response, session).await {
                    Ok(()) => {
                        if responded.load(std::sync::atomic::Ordering::SeqCst) {
                            Ok(())
                        } else {
                            let error = anyhow!("handler did not send a response");
                            let proto_err =
                                ErrorCode::Internal.message(format!("{error}")).to_proto();
                            peer.respond_with_error(receipt, proto_err)?;
                            Err(error)?
                        }
                    }
                    Err(error) => {
                        let proto_err = match &error {
                            Error::Internal(err) => err.to_proto(),
                            _ => ErrorCode::Internal.message(format!("{error}")).to_proto(),
                        };
                        peer.respond_with_error(receipt, proto_err)?;
                        Err(error)
                    }
                }
            }
        })
    }

    fn add_request_stream_handler<F, Fut, M>(&mut self, handler: F) -> &mut Self
    where
        F: 'static + Send + Sync + Fn(M, StreamResponse<M>, MessageContext) -> Fut,
        Fut: Send + Future<Output = Result<()>>,
        M: RequestMessage,
    {
        let handler = Arc::new(handler);
        self.add_handler(move |envelope, session| {
            let receipt = envelope.receipt();
            let handler = handler.clone();
            async move {
                let peer = session.peer.clone();
                let ended = Arc::new(AtomicBool::default());
                let response = StreamResponse {
                    peer: peer.clone(),
                    ended: ended.clone(),
                    receipt,
                };
                match (handler)(envelope.payload, response, session).await {
                    Ok(()) => {
                        if ended.load(std::sync::atomic::Ordering::SeqCst) {
                            Ok(())
                        } else {
                            let error = anyhow!("handler did not end a response stream");
                            let proto_err =
                                ErrorCode::Internal.message(format!("{error}")).to_proto();
                            peer.respond_with_error(receipt, proto_err)?;
                            Err(error)?
                        }
                    }
                    Err(error) => {
                        let proto_err = match &error {
                            Error::Internal(err) => err.to_proto(),
                            _ => ErrorCode::Internal.message(format!("{error}")).to_proto(),
                        };
                        peer.respond_with_error(receipt, proto_err)?;
                        Err(error)
                    }
                }
            }
        })
    }

    pub fn handle_connection(
        self: &Arc<Self>,
        connection: Connection,
        address: String,
        principal: Principal,
        sim_version: SimVersion,
        release_channel: Option<String>,
        user_agent: Option<String>,
        geoip_country_code: Option<String>,
        system_id: Option<String>,
        send_connection_id: Option<oneshot::Sender<ConnectionId>>,
        executor: Executor,
        connection_guard: Option<ConnectionGuard>,
    ) -> impl Future<Output = ()> + use<> {
        let this = self.clone();
        let span = info_span!("handle connection", %address,
            connection_id=field::Empty,
            user_id=field::Empty,
            login=field::Empty,
            user_agent=field::Empty,
            geoip_country_code=field::Empty,
            release_channel=field::Empty,
        );
        principal.update_span(&span);
        if let Some(user_agent) = user_agent {
            span.record("user_agent", user_agent);
        }
        if let Some(release_channel) = release_channel {
            span.record("release_channel", release_channel);
        }

        if let Some(country_code) = geoip_country_code.as_ref() {
            span.record("geoip_country_code", country_code);
        }

        let mut teardown = self.teardown.subscribe();
        async move {
            if *teardown.borrow() {
                tracing::error!("server is tearing down");
                return;
            }

            let (connection_id, handle_io, mut incoming_rx) =
                this.peer.add_connection(connection, {
                    let executor = executor.clone();
                    move |duration| executor.sleep(duration)
                });
            tracing::Span::current().record("connection_id", format!("{}", connection_id));

            tracing::info!("connection opened");

            let session = Session {
                principal: principal.clone(),
                connection_id,
                db: Arc::new(tokio::sync::Mutex::new(DbHandle(this.app_state.db.clone()))),
                peer: this.peer.clone(),
                connection_pool: this.connection_pool.clone(),
                app_state: this.app_state.clone(),
                geoip_country_code,
                system_id,
                _executor: executor.clone(),
            };

            if let Err(error) = this
                .send_initial_client_update(
                    connection_id,
                    sim_version,
                    send_connection_id,
                    &session,
                )
                .await
            {
                tracing::error!(?error, "failed to send initial client update");
                return;
            }
            drop(connection_guard);

            let handle_io = handle_io.fuse();
            futures::pin_mut!(handle_io);

            // Handlers for foreground messages are pushed into the following `FuturesUnordered`.
            // This prevents deadlocks when e.g., client A performs a request to client B and
            // client B performs a request to client A. If both clients stop processing further
            // messages until their respective request completes, they won't have a chance to
            // respond to the other client's request and cause a deadlock.
            //
            // This arrangement ensures we will attempt to process earlier messages first, but fall
            // back to processing messages arrived later in the spirit of making progress.
            const MAX_CONCURRENT_HANDLERS: usize = 256;
            let mut foreground_message_handlers = FuturesUnordered::new();
            let concurrent_handlers = Arc::new(Semaphore::new(MAX_CONCURRENT_HANDLERS));
            let get_concurrent_handlers = {
                let concurrent_handlers = concurrent_handlers.clone();
                move || MAX_CONCURRENT_HANDLERS - concurrent_handlers.available_permits()
            };
            loop {
                let next_message = async {
                    let permit = concurrent_handlers.clone().acquire_owned().await.unwrap();
                    let message = incoming_rx.next().await;
                    // Cache the concurrent_handlers here, so that we know what the
                    // queue looks like as each handler starts
                    (permit, message, get_concurrent_handlers())
                }
                .fuse();
                futures::pin_mut!(next_message);
                futures::select_biased! {
                    _ = teardown.changed().fuse() => return,
                    result = handle_io => {
                        if let Err(error) = result {
                            tracing::error!(?error, "error handling I/O");
                        }
                        break;
                    }
                    _ = foreground_message_handlers.next() => {}
                    next_message = next_message => {
                        let (permit, message, concurrent_handlers) = next_message;
                        if let Some(message) = message {
                            let type_name = message.payload_type_name();
                            // note: we copy all the fields from the parent span so we can query them in the logs.
                            // (https://github.com/tokio-rs/tracing/issues/2670).
                            let span = tracing::info_span!("receive message",
                                %connection_id,
                                %address,
                                type_name,
                                concurrent_handlers,
                                user_id=field::Empty,
                                login=field::Empty,
                                lsp_query_request=field::Empty,
                                release_channel=field::Empty,
                                { TOTAL_DURATION_MS }=field::Empty,
                                { PROCESSING_DURATION_MS }=field::Empty,
                                { QUEUE_DURATION_MS }=field::Empty,
                                { HOST_WAITING_MS }=field::Empty
                            );
                            principal.update_span(&span);
                            let span_enter = span.enter();
                            if let Some(handler) = this.handlers.get(&message.payload_type_id()) {
                                let is_background = message.is_background();
                                let handle_message = (handler)(message, session.clone(), span.clone());
                                drop(span_enter);

                                let handle_message = async move {
                                    handle_message.await;
                                    drop(permit);
                                }.instrument(span);
                                if is_background {
                                    executor.spawn_detached(handle_message);
                                } else {
                                    foreground_message_handlers.push(handle_message);
                                }
                            } else {
                                tracing::error!("no message handler");
                            }
                        } else {
                            tracing::info!("connection closed");
                            break;
                        }
                    }
                }
            }

            drop(foreground_message_handlers);
            let concurrent_handlers = get_concurrent_handlers();
            tracing::info!(concurrent_handlers, "signing out");
            if let Err(error) = connection_lost(session, teardown, executor).await {
                tracing::error!(?error, "error signing out");
            }
        }
        .instrument(span)
    }

    async fn send_initial_client_update(
        &self,
        connection_id: ConnectionId,
        sim_version: SimVersion,
        mut send_connection_id: Option<oneshot::Sender<ConnectionId>>,
        session: &Session,
    ) -> Result<()> {
        self.peer.send(
            connection_id,
            proto::Hello {
                peer_id: Some(connection_id.into()),
            },
        )?;
        tracing::info!("sent hello message");
        if let Some(send_connection_id) = send_connection_id.take() {
            let _ = send_connection_id.send(connection_id);
        }

        match &session.principal {
            Principal::User(user) => {
                if !user.connected_once {
                    self.peer.send(connection_id, proto::ShowContacts {})?;
                    self.app_state
                        .db
                        .set_user_connected_once(user.id, true)
                        .await?;
                }

                let contacts = self.app_state.db.get_contacts(user.id).await?;

                {
                    let mut pool = self.connection_pool.lock();
                    pool.add_connection(connection_id, user.id, user.admin, sim_version.clone());
                    self.peer.send(
                        connection_id,
                        build_initial_contacts_update(contacts, &pool),
                    )?;
                }

                if let Some(incoming_call) =
                    self.app_state.db.incoming_call_for_user(user.id).await?
                {
                    self.peer.send(connection_id, incoming_call)?;
                }

                update_user_contacts(user.id, session).await?;
            }
        }

        Ok(())
    }
}

impl Deref for ConnectionPoolGuard<'_> {
    type Target = ConnectionPool;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl DerefMut for ConnectionPoolGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl Drop for ConnectionPoolGuard<'_> {
    fn drop(&mut self) {
        #[cfg(feature = "test-support")]
        self.check_invariants();
    }
}

fn broadcast<F>(
    sender_id: Option<ConnectionId>,
    receiver_ids: impl IntoIterator<Item = ConnectionId>,
    mut f: F,
) where
    F: FnMut(ConnectionId) -> anyhow::Result<()>,
{
    for receiver_id in receiver_ids {
        if Some(receiver_id) != sender_id
            && let Err(error) = f(receiver_id)
        {
            tracing::error!("failed to send to {:?} {}", receiver_id, error);
        }
    }
}

pub struct ProtocolVersion(u32);

impl Header for ProtocolVersion {
    fn name() -> &'static HeaderName {
        static SIM_PROTOCOL_VERSION: OnceLock<HeaderName> = OnceLock::new();
        SIM_PROTOCOL_VERSION.get_or_init(|| HeaderName::from_static("x-sim-protocol-version"))
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i axum::http::HeaderValue>,
    {
        let version = values
            .next()
            .ok_or_else(axum::headers::Error::invalid)?
            .to_str()
            .map_err(|_| axum::headers::Error::invalid())?
            .parse()
            .map_err(|_| axum::headers::Error::invalid())?;
        Ok(Self(version))
    }

    fn encode<E: Extend<axum::http::HeaderValue>>(&self, values: &mut E) {
        values.extend([self.0.to_string().parse().unwrap()]);
    }
}

pub struct AppVersionHeader(Version);
impl Header for AppVersionHeader {
    fn name() -> &'static HeaderName {
        static SIM_APP_VERSION: OnceLock<HeaderName> = OnceLock::new();
        SIM_APP_VERSION.get_or_init(|| HeaderName::from_static("x-sim-app-version"))
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i axum::http::HeaderValue>,
    {
        let version = values
            .next()
            .ok_or_else(axum::headers::Error::invalid)?
            .to_str()
            .map_err(|_| axum::headers::Error::invalid())?
            .parse()
            .map_err(|_| axum::headers::Error::invalid())?;
        Ok(Self(version))
    }

    fn encode<E: Extend<axum::http::HeaderValue>>(&self, values: &mut E) {
        values.extend([self.0.to_string().parse().unwrap()]);
    }
}

#[derive(Debug)]
pub struct ReleaseChannelHeader(String);

impl Header for ReleaseChannelHeader {
    fn name() -> &'static HeaderName {
        static SIM_RELEASE_CHANNEL: OnceLock<HeaderName> = OnceLock::new();
        SIM_RELEASE_CHANNEL.get_or_init(|| HeaderName::from_static("x-sim-release-channel"))
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i axum::http::HeaderValue>,
    {
        Ok(Self(
            values
                .next()
                .ok_or_else(axum::headers::Error::invalid)?
                .to_str()
                .map_err(|_| axum::headers::Error::invalid())?
                .to_owned(),
        ))
    }

    fn encode<E: Extend<axum::http::HeaderValue>>(&self, values: &mut E) {
        values.extend([self.0.parse().unwrap()]);
    }
}

pub fn routes(server: Arc<Server>) -> Router<(), Body> {
    Router::new()
        .route("/rpc", get(handle_websocket_request))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(server.app_state.clone()))
                .layer(middleware::from_fn(auth::validate_header)),
        )
        .route("/metrics", get(handle_metrics))
        .layer(Extension(server))
}

pub async fn handle_websocket_request(
    TypedHeader(ProtocolVersion(protocol_version)): TypedHeader<ProtocolVersion>,
    app_version_header: Option<TypedHeader<AppVersionHeader>>,
    release_channel_header: Option<TypedHeader<ReleaseChannelHeader>>,
    ConnectInfo(socket_address): ConnectInfo<SocketAddr>,
    Extension(server): Extension<Arc<Server>>,
    Extension(principal): Extension<Principal>,
    user_agent: Option<TypedHeader<UserAgent>>,
    country_code_header: Option<TypedHeader<CloudflareIpCountryHeader>>,
    system_id_header: Option<TypedHeader<SystemIdHeader>>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    if protocol_version != rpc::PROTOCOL_VERSION {
        return (
            StatusCode::UPGRADE_REQUIRED,
            "client must be upgraded".to_string(),
        )
            .into_response();
    }

    let Some(version) = app_version_header.map(|header| SimVersion(header.0.0)) else {
        return (
            StatusCode::UPGRADE_REQUIRED,
            "no version header found".to_string(),
        )
            .into_response();
    };

    let release_channel = release_channel_header.map(|header| header.0.0);

    if !version.can_collaborate() {
        return (
            StatusCode::UPGRADE_REQUIRED,
            "client must be upgraded".to_string(),
        )
            .into_response();
    }

    let socket_address = socket_address.to_string();

    // Acquire connection guard before WebSocket upgrade
    let connection_guard = match ConnectionGuard::try_acquire() {
        Ok(guard) => guard,
        Err(()) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Too many concurrent connections",
            )
                .into_response();
        }
    };

    ws.on_upgrade(move |socket| {
        let socket = socket
            .map_ok(to_tungstenite_message)
            .err_into()
            .with(|message| async move { to_axum_message(message) });
        let connection = Connection::new(Box::pin(socket));
        async move {
            server
                .handle_connection(
                    connection,
                    socket_address,
                    principal,
                    version,
                    release_channel,
                    user_agent.map(|header| header.to_string()),
                    country_code_header.map(|header| header.to_string()),
                    system_id_header.map(|header| header.to_string()),
                    None,
                    Executor::Production,
                    Some(connection_guard),
                )
                .await;
        }
    })
}

pub async fn handle_metrics(Extension(server): Extension<Arc<Server>>) -> Result<String> {
    static CONNECTIONS_METRIC: OnceLock<IntGauge> = OnceLock::new();
    let connections_metric = CONNECTIONS_METRIC
        .get_or_init(|| register_int_gauge!("connections", "number of connections").unwrap());

    let connections = server
        .connection_pool
        .lock()
        .connections()
        .filter(|connection| !connection.admin)
        .count();
    connections_metric.set(connections as _);

    static SHARED_PROJECTS_METRIC: OnceLock<IntGauge> = OnceLock::new();
    let shared_projects_metric = SHARED_PROJECTS_METRIC.get_or_init(|| {
        register_int_gauge!(
            "shared_projects",
            "number of open projects with one or more guests"
        )
        .unwrap()
    });

    let shared_projects = server.app_state.db.project_count_excluding_admins().await?;
    shared_projects_metric.set(shared_projects as _);

    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    let encoded_metrics = encoder
        .encode_to_string(&metric_families)
        .map_err(|err| anyhow!("{err}"))?;
    Ok(encoded_metrics)
}

#[instrument(err, skip(executor))]
async fn connection_lost(
    session: Session,
    mut teardown: watch::Receiver<bool>,
    executor: Executor,
) -> Result<()> {
    session.peer.disconnect(session.connection_id);
    session
        .connection_pool()
        .await
        .remove_connection(session.connection_id)?;

    session
        .db()
        .await
        .connection_lost(session.connection_id)
        .await
        .trace_err();

    futures::select_biased! {
        _ = executor.sleep(RECONNECT_TIMEOUT).fuse() => {

            log::info!("connection lost, removing all resources for user:{}, connection:{:?}", session.user_id(), session.connection_id);
            leave_room_for_session(&session, session.connection_id).await.trace_err();
            leave_channel_buffers_for_session(&session)
                .await
                .trace_err();

            if !session
                .connection_pool()
                .await
                .is_user_online(session.user_id())
            {
                let db = session.db().await;
                if let Some(room) = db.decline_call(None, session.user_id()).await.trace_err().flatten() {
                    room_updated(&room, &session.peer);
                }
            }

            update_user_contacts(session.user_id(), &session).await?;
        },
        _ = teardown.changed().fuse() => {}
    }

    Ok(())
}

/// Acknowledges a ping from a client, used to keep the connection alive.
async fn ping(
    _: proto::Ping,
    response: Response<proto::Ping>,
    _session: MessageContext,
) -> Result<()> {
    response.send(proto::Ack {})?;
    Ok(())
}

/// Creates a new room for calling (outside of channels)
async fn create_room(
    _request: proto::CreateRoom,
    response: Response<proto::CreateRoom>,
    session: MessageContext,
) -> Result<()> {
    let livekit_room = nanoid::nanoid!(30);

    let live_kit_connection_info = util::maybe!(async {
        let live_kit = session.app_state.livekit_client.as_ref();
        let live_kit = live_kit?;
        let user_id = session.user_id().to_string();

        let token = live_kit.room_token(&livekit_room, &user_id).trace_err()?;

        Some(proto::LiveKitConnectionInfo {
            server_url: live_kit.url().into(),
            token,
            can_publish: true,
        })
    })
    .await;

    let room = session
        .db()
        .await
        .create_room(session.user_id(), session.connection_id, &livekit_room)
        .await?;

    response.send(proto::CreateRoomResponse {
        room: Some(room.clone()),
        live_kit_connection_info,
    })?;

    update_user_contacts(session.user_id(), &session).await?;
    Ok(())
}

/// Join a room from an invitation. Equivalent to joining a channel if there is one.
async fn join_room(
    request: proto::JoinRoom,
    response: Response<proto::JoinRoom>,
    session: MessageContext,
) -> Result<()> {
    let room_id = RoomId::from_proto(request.id);

    let channel_id = session.db().await.channel_id_for_room(room_id).await?;

    if let Some(channel_id) = channel_id {
        return join_channel_internal(channel_id, Box::new(response), session).await;
    }

    let joined_room = {
        let room = session
            .db()
            .await
            .join_room(room_id, session.user_id(), session.connection_id)
            .await?;
        room_updated(&room.room, &session.peer);
        room.into_inner()
    };

    for connection_id in session
        .connection_pool()
        .await
        .user_connection_ids(session.user_id())
    {
        session
            .peer
            .send(
                connection_id,
                proto::CallCanceled {
                    room_id: room_id.to_proto(),
                },
            )
            .trace_err();
    }

    let live_kit_connection_info = if let Some(live_kit) = session.app_state.livekit_client.as_ref()
    {
        live_kit
            .room_token(
                &joined_room.room.livekit_room,
                &session.user_id().to_string(),
            )
            .trace_err()
            .map(|token| proto::LiveKitConnectionInfo {
                server_url: live_kit.url().into(),
                token,
                can_publish: true,
            })
    } else {
        None
    };

    response.send(proto::JoinRoomResponse {
        room: Some(joined_room.room),
        channel_id: None,
        live_kit_connection_info,
    })?;

    update_user_contacts(session.user_id(), &session).await?;
    Ok(())
}

/// Rejoin room is used to reconnect to a room after connection errors.
async fn rejoin_room(
    request: proto::RejoinRoom,
    response: Response<proto::RejoinRoom>,
    session: MessageContext,
) -> Result<()> {
    let room;
    let channel;
    {
        let mut rejoined_room = session
            .db()
            .await
            .rejoin_room(request, session.user_id(), session.connection_id)
            .await?;

        let live_kit_connection_info =
            session
                .app_state
                .livekit_client
                .as_ref()
                .and_then(|live_kit| {
                    let (can_publish, token) = if rejoined_room.role == ChannelRole::Guest {
                        (
                            false,
                            live_kit
                                .guest_token(
                                    &rejoined_room.room.livekit_room,
                                    &session.user_id().to_string(),
                                )
                                .trace_err()?,
                        )
                    } else {
                        (
                            true,
                            live_kit
                                .room_token(
                                    &rejoined_room.room.livekit_room,
                                    &session.user_id().to_string(),
                                )
                                .trace_err()?,
                        )
                    };

                    Some(LiveKitConnectionInfo {
                        server_url: live_kit.url().into(),
                        token,
                        can_publish,
                    })
                });

        response.send(proto::RejoinRoomResponse {
            room: Some(rejoined_room.room.clone()),
            reshared_projects: rejoined_room
                .reshared_projects
                .iter()
                .map(|project| proto::ResharedProject {
                    id: project.id.to_proto(),
                    collaborators: project
                        .collaborators
                        .iter()
                        .map(|collaborator| collaborator.to_proto())
                        .collect(),
                })
                .collect(),
            rejoined_projects: rejoined_room
                .rejoined_projects
                .iter()
                .map(|rejoined_project| rejoined_project.to_proto())
                .collect(),
            live_kit_connection_info,
        })?;
        room_updated(&rejoined_room.room, &session.peer);

        for project in &rejoined_room.reshared_projects {
            for collaborator in &project.collaborators {
                session
                    .peer
                    .send(
                        collaborator.connection_id,
                        proto::UpdateProjectCollaborator {
                            project_id: project.id.to_proto(),
                            old_peer_id: Some(project.old_connection_id.into()),
                            new_peer_id: Some(session.connection_id.into()),
                        },
                    )
                    .trace_err();
            }

            broadcast(
                Some(session.connection_id),
                project
                    .collaborators
                    .iter()
                    .map(|collaborator| collaborator.connection_id),
                |connection_id| {
                    session.peer.forward_send(
                        session.connection_id,
                        connection_id,
                        proto::UpdateProject {
                            project_id: project.id.to_proto(),
                            worktrees: project.worktrees.clone(),
                        },
                    )
                },
            );
        }

        notify_rejoined_projects(&mut rejoined_room.rejoined_projects, &session)?;

        let rejoined_room = rejoined_room.into_inner();

        room = rejoined_room.room;
        channel = rejoined_room.channel;
    }

    if let Some(channel) = channel {
        channel_updated(
            &channel,
            &room,
            &session.peer,
            &*session.connection_pool().await,
        );
    }

    update_user_contacts(session.user_id(), &session).await?;
    Ok(())
}

fn notify_rejoined_projects(
    rejoined_projects: &mut Vec<RejoinedProject>,
    session: &Session,
) -> Result<()> {
    for project in rejoined_projects.iter() {
        for collaborator in &project.collaborators {
            session
                .peer
                .send(
                    collaborator.connection_id,
                    proto::UpdateProjectCollaborator {
                        project_id: project.id.to_proto(),
                        old_peer_id: Some(project.old_connection_id.into()),
                        new_peer_id: Some(session.connection_id.into()),
                    },
                )
                .trace_err();
        }
    }

    for project in rejoined_projects {
        for worktree in mem::take(&mut project.worktrees) {
            // Stream this worktree's entries.
            let message = proto::UpdateWorktree {
                project_id: project.id.to_proto(),
                worktree_id: worktree.id,
                abs_path: worktree.abs_path.clone(),
                root_name: worktree.root_name,
                root_repo_common_dir: worktree.root_repo_common_dir,
                updated_entries: worktree.updated_entries,
                removed_entries: worktree.removed_entries,
                scan_id: worktree.scan_id,
                is_last_update: worktree.completed_scan_id == worktree.scan_id,
                updated_repositories: worktree.updated_repositories,
                removed_repositories: worktree.removed_repositories,
            };
            for update in proto::split_worktree_update(message) {
                session.peer.send(session.connection_id, update)?;
            }

            // Stream this worktree's diagnostics.
            let mut worktree_diagnostics = worktree.diagnostic_summaries.into_iter();
            if let Some(summary) = worktree_diagnostics.next() {
                let message = proto::UpdateDiagnosticSummary {
                    project_id: project.id.to_proto(),
                    worktree_id: worktree.id,
                    summary: Some(summary),
                    more_summaries: worktree_diagnostics.collect(),
                };
                session.peer.send(session.connection_id, message)?;
            }

            for settings_file in worktree.settings_files {
                session.peer.send(
                    session.connection_id,
                    proto::UpdateWorktreeSettings {
                        project_id: project.id.to_proto(),
                        worktree_id: worktree.id,
                        path: settings_file.path,
                        content: Some(settings_file.content),
                        kind: Some(settings_file.kind.to_proto().into()),
                        outside_worktree: Some(settings_file.outside_worktree),
                    },
                )?;
            }
        }

        for repository in mem::take(&mut project.updated_repositories) {
            for update in split_repository_update(repository) {
                session.peer.send(session.connection_id, update)?;
            }
        }

        for id in mem::take(&mut project.removed_repositories) {
            session.peer.send(
                session.connection_id,
                proto::RemoveRepository {
                    project_id: project.id.to_proto(),
                    id,
                },
            )?;
        }
    }

    Ok(())
}

/// leave room disconnects from the room.
async fn leave_room(
    _: proto::LeaveRoom,
    response: Response<proto::LeaveRoom>,
    session: MessageContext,
) -> Result<()> {
    leave_room_for_session(&session, session.connection_id).await?;
    response.send(proto::Ack {})?;
    Ok(())
}

/// Updates the permissions of someone else in the room.
async fn set_room_participant_role(
    request: proto::SetRoomParticipantRole,
    response: Response<proto::SetRoomParticipantRole>,
    session: MessageContext,
) -> Result<()> {
    let user_id = UserId::from_proto(request.user_id);
    let role = ChannelRole::from(request.role());

    let (livekit_room, can_publish) = {
        let room = session
            .db()
            .await
            .set_room_participant_role(
                session.user_id(),
                RoomId::from_proto(request.room_id),
                user_id,
                role,
            )
            .await?;

        let livekit_room = room.livekit_room.clone();
        let can_publish = ChannelRole::from(request.role()).can_use_microphone();
        room_updated(&room, &session.peer);
        (livekit_room, can_publish)
    };

    if let Some(live_kit) = session.app_state.livekit_client.as_ref() {
        live_kit
            .update_participant(
                livekit_room.clone(),
                request.user_id.to_string(),
                livekit_api::proto::ParticipantPermission {
                    can_subscribe: true,
                    can_publish,
                    can_publish_data: can_publish,
                    hidden: false,
                    recorder: false,
                },
            )
            .await
            .trace_err();
    }

    response.send(proto::Ack {})?;
    Ok(())
}

/// Call someone else into the current room
async fn call(
    request: proto::Call,
    response: Response<proto::Call>,
    session: MessageContext,
) -> Result<()> {
    let room_id = RoomId::from_proto(request.room_id);
    let calling_user_id = session.user_id();
    let calling_connection_id = session.connection_id;
    let called_user_id = UserId::from_proto(request.called_user_id);
    let initial_project_id = request.initial_project_id.map(ProjectId::from_proto);
    if !session
        .db()
        .await
        .has_contact(calling_user_id, called_user_id)
        .await?
    {
        return Err(anyhow!("cannot call a user who isn't a contact"))?;
    }

    let incoming_call = {
        let (room, incoming_call) = &mut *session
            .db()
            .await
            .call(
                room_id,
                calling_user_id,
                calling_connection_id,
                called_user_id,
                initial_project_id,
            )
            .await?;
        room_updated(room, &session.peer);
        mem::take(incoming_call)
    };
    update_user_contacts(called_user_id, &session).await?;

    let mut calls = session
        .connection_pool()
        .await
        .user_connection_ids(called_user_id)
        .map(|connection_id| session.peer.request(connection_id, incoming_call.clone()))
        .collect::<FuturesUnordered<_>>();

    while let Some(call_response) = calls.next().await {
        match call_response.as_ref() {
            Ok(_) => {
                response.send(proto::Ack {})?;
                return Ok(());
            }
            Err(_) => {
                call_response.trace_err();
            }
        }
    }

    {
        let room = session
            .db()
            .await
            .call_failed(room_id, called_user_id)
            .await?;
        room_updated(&room, &session.peer);
    }
    update_user_contacts(called_user_id, &session).await?;

    Err(anyhow!("failed to ring user"))?
}

/// Cancel an outgoing call.
async fn cancel_call(
    request: proto::CancelCall,
    response: Response<proto::CancelCall>,
    session: MessageContext,
) -> Result<()> {
    let called_user_id = UserId::from_proto(request.called_user_id);
    let room_id = RoomId::from_proto(request.room_id);
    {
        let room = session
            .db()
            .await
            .cancel_call(room_id, session.connection_id, called_user_id)
            .await?;
        room_updated(&room, &session.peer);
    }

    for connection_id in session
        .connection_pool()
        .await
        .user_connection_ids(called_user_id)
    {
        session
            .peer
            .send(
                connection_id,
                proto::CallCanceled {
                    room_id: room_id.to_proto(),
                },
            )
            .trace_err();
    }
    response.send(proto::Ack {})?;

    update_user_contacts(called_user_id, &session).await?;
    Ok(())
}

/// Decline an incoming call.
async fn decline_call(message: proto::DeclineCall, session: MessageContext) -> Result<()> {
    let room_id = RoomId::from_proto(message.room_id);
    {
        let room = session
            .db()
            .await
            .decline_call(Some(room_id), session.user_id())
            .await?
            .context("declining call")?;
        room_updated(&room, &session.peer);
    }

    for connection_id in session
        .connection_pool()
        .await
        .user_connection_ids(session.user_id())
    {
        session
            .peer
            .send(
                connection_id,
                proto::CallCanceled {
                    room_id: room_id.to_proto(),
                },
            )
            .trace_err();
    }
    update_user_contacts(session.user_id(), &session).await?;
    Ok(())
}

/// Updates other participants in the room with your current location.
async fn update_participant_location(
    request: proto::UpdateParticipantLocation,
    response: Response<proto::UpdateParticipantLocation>,
    session: MessageContext,
) -> Result<()> {
    let room_id = RoomId::from_proto(request.room_id);
    let location = request.location.context("invalid location")?;

    let db = session.db().await;
    let room = db
        .update_room_participant_location(room_id, session.connection_id, location)
        .await?;

    room_updated(&room, &session.peer);
    response.send(proto::Ack {})?;
    Ok(())
}

/// Share a project into the room.
async fn share_project(
    request: proto::ShareProject,
    response: Response<proto::ShareProject>,
    session: MessageContext,
) -> Result<()> {
    let (project_id, room) = &*session
        .db()
        .await
        .share_project(
            RoomId::from_proto(request.room_id),
            session.connection_id,
            &request.worktrees,
            request.is_ssh_project,
            request.windows_paths.unwrap_or(false),
            &request.features,
        )
        .await?;
    response.send(proto::ShareProjectResponse {
        project_id: project_id.to_proto(),
    })?;
    room_updated(room, &session.peer);

    Ok(())
}

/// Unshare a project from the room.
async fn unshare_project(message: proto::UnshareProject, session: MessageContext) -> Result<()> {
    let project_id = ProjectId::from_proto(message.project_id);
    unshare_project_internal(project_id, session.connection_id, &session).await
}

async fn unshare_project_internal(
    project_id: ProjectId,
    connection_id: ConnectionId,
    session: &Session,
) -> Result<()> {
    let delete = {
        let room_guard = session
            .db()
            .await
            .unshare_project(project_id, connection_id)
            .await?;

        let (delete, room, guest_connection_ids) = &*room_guard;

        let message = proto::UnshareProject {
            project_id: project_id.to_proto(),
        };

        broadcast(
            Some(connection_id),
            guest_connection_ids.iter().copied(),
            |conn_id| session.peer.send(conn_id, message.clone()),
        );
        if let Some(room) = room {
            room_updated(room, &session.peer);
        }

        *delete
    };

    if delete {
        let db = session.db().await;
        db.delete_project(project_id).await?;
    }

    Ok(())
}

/// Join someone elses shared project.
async fn join_project(
    request: proto::JoinProject,
    response: Response<proto::JoinProject>,
    session: MessageContext,
) -> Result<()> {
    let project_id = ProjectId::from_proto(request.project_id);

    tracing::info!(%project_id, "join project");

    let db = session.db().await;
    let project_model = db.get_project(project_id).await?;
    let host_features: Vec<String> =
        serde_json::from_str(&project_model.features).unwrap_or_default();
    let guest_features: HashSet<_> = request.features.iter().collect();
    let host_features_set: HashSet<_> = host_features.iter().collect();
    if guest_features != host_features_set {
        let host_connection_id = project_model.host_connection()?;
        let mut pool = session.connection_pool().await;
        let host_version = pool
            .connection(host_connection_id)
            .map(|c| c.sim_version.to_string());
        let guest_version = pool
            .connection(session.connection_id)
            .map(|c| c.sim_version.to_string());
        drop(pool);
        Err(anyhow!(
            "The host (v{}) and guest (v{}) are using incompatible versions of Sim. The peer with the older version must update to collaborate.",
            host_version.as_deref().unwrap_or("unknown"),
            guest_version.as_deref().unwrap_or("unknown"),
        ))?;
    }

    let (project, replica_id) = &mut *db
        .join_project(
            project_id,
            session.connection_id,
            session.user_id(),
            request.committer_name.clone(),
            request.committer_email.clone(),
        )
        .await?;
    drop(db);

    tracing::info!(%project_id, "join remote project");
    let collaborators = project
        .collaborators
        .iter()
        .filter(|collaborator| collaborator.connection_id != session.connection_id)
        .map(|collaborator| collaborator.to_proto())
        .collect::<Vec<_>>();
    let project_id = project.id;
    let guest_user_id = session.user_id();

    let worktrees = project
        .worktrees
        .iter()
        .map(|(id, worktree)| proto::WorktreeMetadata {
            id: *id,
            root_name: worktree.root_name.clone(),
            visible: worktree.visible,
            abs_path: worktree.abs_path.clone(),
            root_repo_common_dir: None,
        })
        .collect::<Vec<_>>();

    let add_project_collaborator = proto::AddProjectCollaborator {
        project_id: project_id.to_proto(),
        collaborator: Some(proto::Collaborator {
            peer_id: Some(session.connection_id.into()),
            replica_id: replica_id.0 as u32,
            user_id: guest_user_id.to_proto(),
            is_host: false,
            committer_name: request.committer_name.clone(),
            committer_email: request.committer_email.clone(),
        }),
    };

    for collaborator in &collaborators {
        session
            .peer
            .send(
                collaborator.peer_id.unwrap().into(),
                add_project_collaborator.clone(),
            )
            .trace_err();
    }

    // First, we send the metadata associated with each worktree.
    let (language_servers, language_server_capabilities) = project
        .language_servers
        .clone()
        .into_iter()
        .map(|server| (server.server, server.capabilities))
        .unzip();
    response.send(proto::JoinProjectResponse {
        project_id: project.id.0 as u64,
        worktrees,
        replica_id: replica_id.0 as u32,
        collaborators,
        language_servers,
        language_server_capabilities,
        role: project.role.into(),
        windows_paths: project.path_style == PathStyle::Windows,
        features: project.features.clone(),
    })?;

    for (worktree_id, worktree) in mem::take(&mut project.worktrees) {
        // Stream this worktree's entries.
        let message = proto::UpdateWorktree {
            project_id: project_id.to_proto(),
            worktree_id,
            abs_path: worktree.abs_path.clone(),
            root_name: worktree.root_name,
            root_repo_common_dir: worktree.root_repo_common_dir,
            updated_entries: worktree.entries,
            removed_entries: Default::default(),
            scan_id: worktree.scan_id,
            is_last_update: worktree.scan_id == worktree.completed_scan_id,
            updated_repositories: worktree.legacy_repository_entries.into_values().collect(),
            removed_repositories: Default::default(),
        };
        for update in proto::split_worktree_update(message) {
            session.peer.send(session.connection_id, update.clone())?;
        }

        // Stream this worktree's diagnostics.
        let mut worktree_diagnostics = worktree.diagnostic_summaries.into_iter();
        if let Some(summary) = worktree_diagnostics.next() {
            let message = proto::UpdateDiagnosticSummary {
                project_id: project.id.to_proto(),
                worktree_id: worktree.id,
                summary: Some(summary),
                more_summaries: worktree_diagnostics.collect(),
            };
            session.peer.send(session.connection_id, message)?;
        }

        for settings_file in worktree.settings_files {
            session.peer.send(
                session.connection_id,
                proto::UpdateWorktreeSettings {
                    project_id: project_id.to_proto(),
                    worktree_id: worktree.id,
                    path: settings_file.path,
                    content: Some(settings_file.content),
                    kind: Some(settings_file.kind.to_proto() as i32),
                    outside_worktree: Some(settings_file.outside_worktree),
                },
            )?;
        }
    }

    for repository in mem::take(&mut project.repositories) {
        for update in split_repository_update(repository) {
            session.peer.send(session.connection_id, update)?;
        }
    }

    for language_server in &project.language_servers {
        session.peer.send(
            session.connection_id,
            proto::UpdateLanguageServer {
                project_id: project_id.to_proto(),
                server_name: Some(language_server.server.name.clone()),
                language_server_id: language_server.server.id,
                variant: Some(
                    proto::update_language_server::Variant::DiskBasedDiagnosticsUpdated(
                        proto::LspDiskBasedDiagnosticsUpdated {},
                    ),
                ),
            },
        )?;
    }

    Ok(())
}

/// Leave someone elses shared project.
async fn leave_project(request: proto::LeaveProject, session: MessageContext) -> Result<()> {
    let sender_id = session.connection_id;
    let project_id = ProjectId::from_proto(request.project_id);
    let db = session.db().await;

    let (room, project) = &*db.leave_project(project_id, sender_id).await?;
    tracing::info!(
        %project_id,
        "leave project"
    );

    project_left(project, &session);
    if let Some(room) = room {
        room_updated(room, &session.peer);
    }

    Ok(())
}

/// Updates other participants with changes to the project
async fn update_project(
    request: proto::UpdateProject,
    response: Response<proto::UpdateProject>,
    session: MessageContext,
) -> Result<()> {
    let project_id = ProjectId::from_proto(request.project_id);
    let (room, guest_connection_ids) = &*session
        .db()
        .await
        .update_project(project_id, session.connection_id, &request.worktrees)
        .await?;
    broadcast(
        Some(session.connection_id),
        guest_connection_ids.iter().copied(),
        |connection_id| {
            session
                .peer
                .forward_send(session.connection_id, connection_id, request.clone())
        },
    );
    if let Some(room) = room {
        room_updated(room, &session.peer);
    }
    response.send(proto::Ack {})?;

    Ok(())
}

/// Updates other participants with changes to the worktree
async fn update_worktree(
    request: proto::UpdateWorktree,
    response: Response<proto::UpdateWorktree>,
    session: MessageContext,
) -> Result<()> {
    let guest_connection_ids = session
        .db()
        .await
        .update_worktree(&request, session.connection_id)
        .await?;

    broadcast(
        Some(session.connection_id),
        guest_connection_ids.iter().copied(),
        |connection_id| {
            session
                .peer
                .forward_send(session.connection_id, connection_id, request.clone())
        },
    );
    response.send(proto::Ack {})?;
    Ok(())
}

async fn update_repository(
    request: proto::UpdateRepository,
    response: Response<proto::UpdateRepository>,
    session: MessageContext,
) -> Result<()> {
    let guest_connection_ids = session
        .db()
        .await
        .update_repository(&request, session.connection_id)
        .await?;

    broadcast(
        Some(session.connection_id),
        guest_connection_ids.iter().copied(),
        |connection_id| {
            session
                .peer
                .forward_send(session.connection_id, connection_id, request.clone())
        },
    );
    response.send(proto::Ack {})?;
    Ok(())
}

async fn remove_repository(
    request: proto::RemoveRepository,
    response: Response<proto::RemoveRepository>,
    session: MessageContext,
) -> Result<()> {
    let guest_connection_ids = session
        .db()
        .await
        .remove_repository(&request, session.connection_id)
        .await?;

    broadcast(
        Some(session.connection_id),
        guest_connection_ids.iter().copied(),
        |connection_id| {
            session
                .peer
                .forward_send(session.connection_id, connection_id, request.clone())
        },
    );
    response.send(proto::Ack {})?;
    Ok(())
}

/// Updates other participants with changes to the diagnostics
async fn update_diagnostic_summary(
    message: proto::UpdateDiagnosticSummary,
    session: MessageContext,
) -> Result<()> {
    let guest_connection_ids = session
        .db()
        .await
        .update_diagnostic_summary(&message, session.connection_id)
        .await?;

    broadcast(
        Some(session.connection_id),
        guest_connection_ids.iter().copied(),
        |connection_id| {
            session
                .peer
                .forward_send(session.connection_id, connection_id, message.clone())
        },
    );

    Ok(())
}

/// Updates other participants with changes to the worktree settings
async fn update_worktree_settings(
    message: proto::UpdateWorktreeSettings,
    session: MessageContext,
) -> Result<()> {
    let guest_connection_ids = session
        .db()
        .await
        .update_worktree_settings(&message, session.connection_id)
        .await?;

    broadcast(
        Some(session.connection_id),
        guest_connection_ids.iter().copied(),
        |connection_id| {
            session
                .peer
                .forward_send(session.connection_id, connection_id, message.clone())
        },
    );

    Ok(())
}

/// Notify other participants that a language server has started.
async fn start_language_server(
    request: proto::StartLanguageServer,
    session: MessageContext,
) -> Result<()> {
    let guest_connection_ids = session
        .db()
        .await
        .start_language_server(&request, session.connection_id)
        .await?;

    broadcast(
        Some(session.connection_id),
        guest_connection_ids.iter().copied(),
        |connection_id| {
            session
                .peer
                .forward_send(session.connection_id, connection_id, request.clone())
        },
    );
    Ok(())
}

/// Notify other participants that a language server has changed.
async fn update_language_server(
    request: proto::UpdateLanguageServer,
    session: MessageContext,
) -> Result<()> {
    let project_id = ProjectId::from_proto(request.project_id);
    let db = session.db().await;

    if let Some(proto::update_language_server::Variant::MetadataUpdated(update)) = &request.variant
        && let Some(capabilities) = update.capabilities.clone()
    {
        db.update_server_capabilities(project_id, request.language_server_id, capabilities)
            .await?;
    }

    let project_connection_ids = db
        .project_connection_ids(project_id, session.connection_id, true)
        .await?;
    broadcast(
        Some(session.connection_id),
        project_connection_ids.iter().copied(),
        |connection_id| {
            session
                .peer
                .forward_send(session.connection_id, connection_id, request.clone())
        },
    );
    Ok(())
}

/// forward a project request to the host. These requests should be read only
/// as guests are allowed to send them.
async fn forward_read_only_project_request<T>(
    request: T,
    response: Response<T>,
    session: MessageContext,
) -> Result<()>
where
    T: EntityMessage + RequestMessage,
{
    let project_id = ProjectId::from_proto(request.remote_entity_id());
    let host_connection_id = session
        .db()
        .await
        .host_for_read_only_project_request(project_id, session.connection_id)
        .await?;
    let payload = session.forward_request(host_connection_id, request).await?;
    response.send(payload)?;
    Ok(())
}

/// forward a project stream request to the host. These requests should be read only
/// as guests are allowed to send them.
async fn forward_read_only_project_stream_request<T>(
    request: T,
    response: StreamResponse<T>,
    session: MessageContext,
) -> Result<()>
where
    T: EntityMessage + RequestMessage,
{
    let project_id = ProjectId::from_proto(request.remote_entity_id());
    let host_connection_id = session
        .db()
        .await
        .host_for_read_only_project_request(project_id, session.connection_id)
        .await?;
    let mut stream = session
        .forward_request_stream(host_connection_id, request)
        .await?;
    while let Some(payload) = stream.next().await {
        response.send(payload?)?;
    }
    response.end()?;
    Ok(())
}

/// forward a project request to the host. These requests are disallowed
/// for guests.
async fn forward_mutating_project_request<T>(
    request: T,
    response: Response<T>,
    session: MessageContext,
) -> Result<()>
where
    T: EntityMessage + RequestMessage,
{
    let project_id = ProjectId::from_proto(request.remote_entity_id());

    let host_connection_id = session
        .db()
        .await
        .host_for_mutating_project_request(project_id, session.connection_id)
        .await?;
    let payload = session.forward_request(host_connection_id, request).await?;
    response.send(payload)?;
    Ok(())
}

async fn disallow_guest_request<T>(
    _request: T,
    response: Response<T>,
    _session: MessageContext,
) -> Result<()>
where
    T: RequestMessage,
{
    response.peer.respond_with_error(
        response.receipt,
        ErrorCode::Forbidden
            .message("request is not allowed for guests".to_string())
            .to_proto(),
    )?;
    response.responded.store(true, SeqCst);
    Ok(())
}

async fn lsp_query(
    request: proto::LspQuery,
    response: Response<proto::LspQuery>,
    session: MessageContext,
) -> Result<()> {
    let (name, should_write) = request.query_name_and_write_permissions();
    tracing::Span::current().record("lsp_query_request", name);
    tracing::info!("lsp_query message received");
    if should_write {
        forward_mutating_project_request(request, response, session).await
    } else {
        forward_read_only_project_request(request, response, session).await
    }
}

/// Notify other participants that a new buffer has been created
async fn create_buffer_for_peer(
    request: proto::CreateBufferForPeer,
    session: MessageContext,
) -> Result<()> {
    session
        .db()
        .await
        .check_user_is_project_host(
            ProjectId::from_proto(request.project_id),
            session.connection_id,
        )
        .await?;
    let peer_id = request.peer_id.context("invalid peer id")?;
    session
        .peer
        .forward_send(session.connection_id, peer_id.into(), request)?;
    Ok(())
}

/// Notify other participants that a new image has been created
async fn create_image_for_peer(
    request: proto::CreateImageForPeer,
    session: MessageContext,
) -> Result<()> {
    session
        .db()
        .await
        .check_user_is_project_host(
            ProjectId::from_proto(request.project_id),
            session.connection_id,
        )
        .await?;
    let peer_id = request.peer_id.context("invalid peer id")?;
    session
        .peer
        .forward_send(session.connection_id, peer_id.into(), request)?;
    Ok(())
}

/// Notify other participants that a buffer has been updated. This is
/// allowed for guests as long as the update is limited to selections.
async fn update_buffer(
    request: proto::UpdateBuffer,
    response: Response<proto::UpdateBuffer>,
    session: MessageContext,
) -> Result<()> {
    let project_id = ProjectId::from_proto(request.project_id);
    let mut capability = Capability::ReadOnly;

    for op in request.operations.iter() {
        match op.variant {
            None | Some(proto::operation::Variant::UpdateSelections(_)) => {}
            Some(_) => capability = Capability::ReadWrite,
        }
    }

    let host = {
        let guard = session
            .db()
            .await
            .connections_for_buffer_update(project_id, session.connection_id, capability)
            .await?;

        let (host, guests) = &*guard;

        broadcast(
            Some(session.connection_id),
            guests.clone(),
            |connection_id| {
                session
                    .peer
                    .forward_send(session.connection_id, connection_id, request.clone())
            },
        );

        *host
    };

    if host != session.connection_id {
        session.forward_request(host, request.clone()).await?;
    }

    response.send(proto::Ack {})?;
    Ok(())
}

async fn forward_project_search_chunk(
    message: proto::FindSearchCandidatesChunk,
    response: Response<proto::FindSearchCandidatesChunk>,
    session: MessageContext,
) -> Result<()> {
    let peer_id = message.peer_id.context("missing peer_id")?;
    let payload = session
        .peer
        .forward_request(session.connection_id, peer_id.into(), message)
        .await?;
    response.send(payload)?;
    Ok(())
}

/// Notify other participants that a project has been updated.
async fn broadcast_project_message_from_host<T: EntityMessage<Entity = ShareProject>>(
    request: T,
    session: MessageContext,
) -> Result<()> {
    let project_id = ProjectId::from_proto(request.remote_entity_id());
    let project_connection_ids = session
        .db()
        .await
        .project_connection_ids(project_id, session.connection_id, false)
        .await?;

    broadcast(
        Some(session.connection_id),
        project_connection_ids.iter().copied(),
        |connection_id| {
            session
                .peer
                .forward_send(session.connection_id, connection_id, request.clone())
        },
    );
    Ok(())
}

/// Start following another user in a call.
async fn follow(
    request: proto::Follow,
    response: Response<proto::Follow>,
    session: MessageContext,
) -> Result<()> {
    let room_id = RoomId::from_proto(request.room_id);
    let project_id = request.project_id.map(ProjectId::from_proto);
    let leader_id = request.leader_id.context("invalid leader id")?.into();
    let follower_id = session.connection_id;

    session
        .db()
        .await
        .check_room_participants(room_id, leader_id, session.connection_id)
        .await?;

    let response_payload = session.forward_request(leader_id, request).await?;
    response.send(response_payload)?;

    if let Some(project_id) = project_id {
        let room = session
            .db()
            .await
            .follow(room_id, project_id, leader_id, follower_id)
            .await?;
        room_updated(&room, &session.peer);
    }

    Ok(())
}

/// Stop following another user in a call.
async fn unfollow(request: proto::Unfollow, session: MessageContext) -> Result<()> {
    let room_id = RoomId::from_proto(request.room_id);
    let project_id = request.project_id.map(ProjectId::from_proto);
    let leader_id = request.leader_id.context("invalid leader id")?.into();
    let follower_id = session.connection_id;

    session
        .db()
        .await
        .check_room_participants(room_id, leader_id, session.connection_id)
        .await?;

    session
        .peer
        .forward_send(session.connection_id, leader_id, request)?;

    if let Some(project_id) = project_id {
        let room = session
            .db()
            .await
            .unfollow(room_id, project_id, leader_id, follower_id)
            .await?;
        room_updated(&room, &session.peer);
    }

    Ok(())
}

/// Notify everyone following you of your current location.
async fn update_followers(request: proto::UpdateFollowers, session: MessageContext) -> Result<()> {
    let room_id = RoomId::from_proto(request.room_id);
    let database = session.db.lock().await;

    let connection_ids = if let Some(project_id) = request.project_id {
        let project_id = ProjectId::from_proto(project_id);
        database
            .project_connection_ids(project_id, session.connection_id, true)
            .await?
    } else {
        database
            .room_connection_ids(room_id, session.connection_id)
            .await?
    };

    // For now, don't send view update messages back to that view's current leader.
    let peer_id_to_omit = request.variant.as_ref().and_then(|variant| match variant {
        proto::update_followers::Variant::UpdateView(payload) => payload.leader_id,
        _ => None,
    });

    for connection_id in connection_ids.iter().cloned() {
        if Some(connection_id.into()) != peer_id_to_omit && connection_id != session.connection_id {
            session
                .peer
                .forward_send(session.connection_id, connection_id, request.clone())?;
        }
    }
    Ok(())
}

/// Get public data about users.
async fn get_users(
    request: proto::GetUsers,
    response: Response<proto::GetUsers>,
    session: MessageContext,
) -> Result<()> {
    let user_ids = request
        .user_ids
        .into_iter()
        .map(UserId::from_proto)
        .collect();
    let users = session
        .app_state
        .user_service
        .get_users_by_ids(user_ids)
        .await?;
    let users = users
        .into_iter()
        .map(|user| proto::User {
            id: user.id.to_proto(),
            avatar_url: user.avatar_url,
            github_login: user.github_login,
            name: user.name,
        })
        .collect();
    response.send(proto::UsersResponse { users })?;
    Ok(())
}

/// Search for users (to invite) buy Github login
async fn fuzzy_search_users(
    request: proto::FuzzySearchUsers,
    response: Response<proto::FuzzySearchUsers>,
    session: MessageContext,
) -> Result<()> {
    let query = request.query;
    let users = match query.len() {
        0 => vec![],
        1 | 2 => session
            .app_state
            .user_service
            .get_user_by_github_login(&query)
            .await?
            .into_iter()
            .collect(),
        _ => {
            session
                .app_state
                .user_service
                .fuzzy_search_users(&query, 10)
                .await?
        }
    };
    let users = users
        .into_iter()
        .filter(|user| user.id != session.user_id())
        .map(|user| proto::User {
            id: user.id.to_proto(),
            avatar_url: user.avatar_url,
            github_login: user.github_login,
            name: user.name,
        })
        .collect();
    response.send(proto::UsersResponse { users })?;
    Ok(())
}

/// Send a contact request to another user.
async fn request_contact(
    request: proto::RequestContact,
    response: Response<proto::RequestContact>,
    session: MessageContext,
) -> Result<()> {
    let requester_id = session.user_id();
    let responder_id = UserId::from_proto(request.responder_id);
    if requester_id == responder_id {
        return Err(anyhow!("cannot add yourself as a contact"))?;
    }

    let notifications = session
        .db()
        .await
        .send_contact_request(requester_id, responder_id)
        .await?;

    // Update outgoing contact requests of requester
    let mut update = proto::UpdateContacts::default();
    update.outgoing_requests.push(responder_id.to_proto());
    for connection_id in session
        .connection_pool()
        .await
        .user_connection_ids(requester_id)
    {
        session.peer.send(connection_id, update.clone())?;
    }

    // Update incoming contact requests of responder
    let mut update = proto::UpdateContacts::default();
    update
        .incoming_requests
        .push(proto::IncomingContactRequest {
            requester_id: requester_id.to_proto(),
        });
    let connection_pool = session.connection_pool().await;
    for connection_id in connection_pool.user_connection_ids(responder_id) {
        session.peer.send(connection_id, update.clone())?;
    }

    send_notifications(&connection_pool, &session.peer, notifications);

    response.send(proto::Ack {})?;
    Ok(())
}

/// Accept or decline a contact request
async fn respond_to_contact_request(
    request: proto::RespondToContactRequest,
    response: Response<proto::RespondToContactRequest>,
    session: MessageContext,
) -> Result<()> {
    let responder_id = session.user_id();
    let requester_id = UserId::from_proto(request.requester_id);
    let db = session.db().await;
    if request.response == proto::ContactRequestResponse::Dismiss as i32 {
        db.dismiss_contact_notification(responder_id, requester_id)
            .await?;
    } else {
        let accept = request.response == proto::ContactRequestResponse::Accept as i32;

        let notifications = db
            .respond_to_contact_request(responder_id, requester_id, accept)
            .await?;
        let requester_busy = db.is_user_busy(requester_id).await?;
        let responder_busy = db.is_user_busy(responder_id).await?;

        let pool = session.connection_pool().await;
        // Update responder with new contact
        let mut update = proto::UpdateContacts::default();
        if accept {
            update
                .contacts
                .push(contact_for_user(requester_id, requester_busy, &pool));
        }
        update
            .remove_incoming_requests
            .push(requester_id.to_proto());
        for connection_id in pool.user_connection_ids(responder_id) {
            session.peer.send(connection_id, update.clone())?;
        }

        // Update requester with new contact
        let mut update = proto::UpdateContacts::default();
        if accept {
            update
                .contacts
                .push(contact_for_user(responder_id, responder_busy, &pool));
        }
        update
            .remove_outgoing_requests
            .push(responder_id.to_proto());

        for connection_id in pool.user_connection_ids(requester_id) {
            session.peer.send(connection_id, update.clone())?;
        }

        send_notifications(&pool, &session.peer, notifications);
    }

    response.send(proto::Ack {})?;
    Ok(())
}

/// Remove a contact.
async fn remove_contact(
    request: proto::RemoveContact,
    response: Response<proto::RemoveContact>,
    session: MessageContext,
) -> Result<()> {
    let requester_id = session.user_id();
    let responder_id = UserId::from_proto(request.user_id);
    let db = session.db().await;
    let (contact_accepted, deleted_notification_id) =
        db.remove_contact(requester_id, responder_id).await?;

    let pool = session.connection_pool().await;
    // Update outgoing contact requests of requester
    let mut update = proto::UpdateContacts::default();
    if contact_accepted {
        update.remove_contacts.push(responder_id.to_proto());
    } else {
        update
            .remove_outgoing_requests
            .push(responder_id.to_proto());
    }
    for connection_id in pool.user_connection_ids(requester_id) {
        session.peer.send(connection_id, update.clone())?;
    }

    // Update incoming contact requests of responder
    let mut update = proto::UpdateContacts::default();
    if contact_accepted {
        update.remove_contacts.push(requester_id.to_proto());
    } else {
        update
            .remove_incoming_requests
            .push(requester_id.to_proto());
    }
    for connection_id in pool.user_connection_ids(responder_id) {
        session.peer.send(connection_id, update.clone())?;
        if let Some(notification_id) = deleted_notification_id {
            session.peer.send(
                connection_id,
                proto::DeleteNotification {
                    notification_id: notification_id.to_proto(),
                },
            )?;
        }
    }

    response.send(proto::Ack {})?;
    Ok(())
}

async fn subscribe_to_channels(
    _: proto::SubscribeToChannels,
    session: MessageContext,
) -> Result<()> {
    subscribe_user_to_channels(session.user_id(), &session).await?;
    Ok(())
}

async fn subscribe_user_to_channels(user_id: UserId, session: &Session) -> Result<(), Error> {
    let channels_for_user = session.db().await.get_channels_for_user(user_id).await?;
    let mut pool = session.connection_pool().await;
    for membership in &channels_for_user.channel_memberships {
        pool.subscribe_to_channel(user_id, membership.channel_id, membership.role)
    }
    session.peer.send(
        session.connection_id,
        build_update_user_channels(&channels_for_user),
    )?;
    session.peer.send(
        session.connection_id,
        build_channels_update(channels_for_user),
    )?;
    Ok(())
}

async fn create_group(
    request: proto::CreateGroup,
    response: Response<proto::CreateGroup>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let member_ids = request
        .member_ids
        .iter()
        .copied()
        .map(UserId::from_proto)
        .collect::<Vec<_>>();
    let group = db
        .create_group(
            &request.name,
            &request.display_name,
            session.user_id(),
            &member_ids,
        )
        .await
        .map_err(group_rpc_error)?;
    response.send(proto::CreateGroupResponse {
        group: Some(group.to_proto()),
    })?;
    broadcast_groups(
        &session,
        proto::UpdateGroups {
            groups: vec![group.to_proto()],
            ..Default::default()
        },
    )
    .await
}

async fn update_group(
    request: proto::UpdateGroup,
    response: Response<proto::UpdateGroup>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let group_id = GroupId::from_proto(request.group_id);
    ensure_group_admin(&db, group_id, session.user_id()).await?;
    let group = db
        .update_group(
            group_id,
            request.name.as_deref(),
            request.display_name.as_deref(),
        )
        .await
        .map_err(group_rpc_error)?;
    response.send(proto::UpdateGroupResponse {
        group: Some(group.to_proto()),
    })?;
    broadcast_groups(
        &session,
        proto::UpdateGroups {
            groups: vec![group.to_proto()],
            ..Default::default()
        },
    )
    .await
}

async fn delete_group(
    request: proto::DeleteGroup,
    response: Response<proto::DeleteGroup>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let group_id = GroupId::from_proto(request.group_id);
    ensure_group_admin(&db, group_id, session.user_id()).await?;
    db.delete_group(group_id).await.map_err(group_rpc_error)?;
    response.send(proto::DeleteGroupResponse {})?;
    broadcast_groups(
        &session,
        proto::UpdateGroups {
            delete_group_ids: vec![group_id.to_proto()],
            ..Default::default()
        },
    )
    .await
}

async fn get_groups(
    _: proto::GetGroups,
    response: Response<proto::GetGroups>,
    session: MessageContext,
) -> Result<()> {
    let groups = session
        .db()
        .await
        .get_groups()
        .await
        .map_err(group_rpc_error)?;
    response.send(proto::GetGroupsResponse {
        groups: groups
            .iter()
            .map(db::queries::groups::GroupWithMembers::to_proto)
            .collect(),
    })?;
    Ok(())
}

async fn update_group_members(
    request: proto::UpdateGroupMembers,
    response: Response<proto::UpdateGroupMembers>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let group_id = GroupId::from_proto(request.group_id);
    ensure_group_admin(&db, group_id, session.user_id()).await?;
    let add_ids = request
        .add_user_ids
        .iter()
        .copied()
        .map(UserId::from_proto)
        .collect::<Vec<_>>();
    let remove_ids = request
        .remove_user_ids
        .iter()
        .copied()
        .map(UserId::from_proto)
        .collect::<Vec<_>>();
    let group = db
        .update_group_members(group_id, &add_ids, &remove_ids)
        .await
        .map_err(group_rpc_error)?;
    response.send(proto::UpdateGroupMembersResponse {
        group: Some(group.to_proto()),
    })?;
    broadcast_groups(
        &session,
        proto::UpdateGroups {
            groups: vec![group.to_proto()],
            ..Default::default()
        },
    )
    .await
}

async fn leave_group(
    request: proto::LeaveGroup,
    response: Response<proto::LeaveGroup>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let group_id = GroupId::from_proto(request.group_id);
    db.leave_group(group_id, session.user_id())
        .await
        .map_err(group_rpc_error)?;
    let group = db
        .get_group(group_id)
        .await
        .map_err(group_rpc_error)?
        .ok_or_else(|| {
            ErrorCode::NotFound
                .message("group not found".into())
                .anyhow()
        })?;
    response.send(proto::LeaveGroupResponse {})?;
    broadcast_groups(
        &session,
        proto::UpdateGroups {
            groups: vec![group.to_proto()],
            ..Default::default()
        },
    )
    .await
}

async fn ensure_group_admin(db: &Database, group_id: GroupId, user_id: UserId) -> Result<()> {
    let group = db
        .get_group(group_id)
        .await
        .map_err(group_rpc_error)?
        .ok_or_else(|| {
            ErrorCode::NotFound
                .message("group not found".into())
                .anyhow()
        })?;
    if group.group.admin_id != user_id {
        return Err(ErrorCode::PermissionDenied
            .message("only the group admin can modify this group".into())
            .anyhow()
            .into());
    }
    Ok(())
}

fn group_rpc_error(error: Error) -> Error {
    let Error::Internal(error) = error else {
        return error;
    };
    let Some(group_error) = error.downcast_ref::<db::queries::groups::GroupError>() else {
        return Error::Internal(error);
    };
    let (code, message) = match group_error {
        db::queries::groups::GroupError::DuplicateName => {
            (ErrorCode::AlreadyExists, "group name already exists")
        }
        db::queries::groups::GroupError::EmptyDisplayName => (
            ErrorCode::InvalidArgument,
            "group display name cannot be empty",
        ),
        db::queries::groups::GroupError::InvalidName => (
            ErrorCode::InvalidArgument,
            "group name must contain only letters, numbers, and hyphens",
        ),
        db::queries::groups::GroupError::MembershipNotFound => {
            (ErrorCode::NotFound, "group membership not found")
        }
        db::queries::groups::GroupError::NotFound => (ErrorCode::NotFound, "group not found"),
        db::queries::groups::GroupError::TooManyMembers => (
            ErrorCode::InvalidArgument,
            "group exceeds maximum member count",
        ),
    };
    code.anyhow().context(message).into()
}

async fn broadcast_groups(session: &MessageContext, update: proto::UpdateGroups) -> Result<()> {
    let connection_pool = session.connection_pool().await;
    for connection_id in connection_pool.connection_ids() {
        session.peer.send(connection_id, update.clone())?;
    }
    Ok(())
}

/// Creates a new channel.
async fn create_channel(
    request: proto::CreateChannel,
    response: Response<proto::CreateChannel>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;

    let parent_id = request.parent_id.map(ChannelId::from_proto);
    let (channel, membership) = db
        .create_channel(&request.name, parent_id, session.user_id())
        .await?;

    let root_id = channel.root_id();
    let channel = Channel::from_model(channel);

    response.send(proto::CreateChannelResponse {
        channel: Some(channel.to_proto()),
        parent_id: request.parent_id,
    })?;

    let mut connection_pool = session.connection_pool().await;
    if let Some(membership) = membership {
        connection_pool.subscribe_to_channel(
            membership.user_id,
            membership.channel_id,
            membership.role,
        );
        let update = proto::UpdateUserChannels {
            channel_memberships: vec![proto::ChannelMembership {
                channel_id: membership.channel_id.to_proto(),
                role: membership.role.into(),
            }],
            ..Default::default()
        };
        for connection_id in connection_pool.user_connection_ids(membership.user_id) {
            session.peer.send(connection_id, update.clone())?;
        }
    }

    for (connection_id, role) in connection_pool.channel_connection_ids(root_id) {
        if !role.can_see_channel(channel.visibility) {
            continue;
        }

        let update = proto::UpdateChannels {
            channels: vec![channel.to_proto()],
            ..Default::default()
        };
        session.peer.send(connection_id, update.clone())?;
    }

    Ok(())
}

/// Delete a channel
async fn delete_channel(
    request: proto::DeleteChannel,
    response: Response<proto::DeleteChannel>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;

    let channel_id = request.channel_id;
    let (root_channel, removed_channels) = db
        .delete_channel(ChannelId::from_proto(channel_id), session.user_id())
        .await?;
    response.send(proto::Ack {})?;

    // Notify members of removed channels
    let mut update = proto::UpdateChannels::default();
    update
        .delete_channels
        .extend(removed_channels.into_iter().map(|id| id.to_proto()));

    let connection_pool = session.connection_pool().await;
    for (connection_id, _) in connection_pool.channel_connection_ids(root_channel) {
        session.peer.send(connection_id, update.clone())?;
    }

    Ok(())
}

/// Invite someone to join a channel.
async fn invite_channel_member(
    request: proto::InviteChannelMember,
    response: Response<proto::InviteChannelMember>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let channel_id = ChannelId::from_proto(request.channel_id);
    let invitee_id = UserId::from_proto(request.user_id);
    let InviteMemberResult {
        channel,
        notifications,
    } = db
        .invite_channel_member(
            channel_id,
            invitee_id,
            session.user_id(),
            request.role().into(),
        )
        .await?;

    let update = proto::UpdateChannels {
        channel_invitations: vec![channel.to_proto()],
        ..Default::default()
    };

    let connection_pool = session.connection_pool().await;
    for connection_id in connection_pool.user_connection_ids(invitee_id) {
        session.peer.send(connection_id, update.clone())?;
    }

    send_notifications(&connection_pool, &session.peer, notifications);

    response.send(proto::Ack {})?;
    Ok(())
}

/// remove someone from a channel
async fn remove_channel_member(
    request: proto::RemoveChannelMember,
    response: Response<proto::RemoveChannelMember>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let channel_id = ChannelId::from_proto(request.channel_id);
    let member_id = UserId::from_proto(request.user_id);

    let RemoveChannelMemberResult {
        membership_update,
        notification_id,
    } = db
        .remove_channel_member(channel_id, member_id, session.user_id())
        .await?;

    let mut connection_pool = session.connection_pool().await;
    notify_membership_updated(
        &mut connection_pool,
        membership_update,
        member_id,
        &session.peer,
    );
    for connection_id in connection_pool.user_connection_ids(member_id) {
        if let Some(notification_id) = notification_id {
            session
                .peer
                .send(
                    connection_id,
                    proto::DeleteNotification {
                        notification_id: notification_id.to_proto(),
                    },
                )
                .trace_err();
        }
    }

    response.send(proto::Ack {})?;
    Ok(())
}

/// Toggle the channel between public and private.
/// Care is taken to maintain the invariant that public channels only descend from public channels,
/// (though members-only channels can appear at any point in the hierarchy).
async fn set_channel_visibility(
    request: proto::SetChannelVisibility,
    response: Response<proto::SetChannelVisibility>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let channel_id = ChannelId::from_proto(request.channel_id);
    let visibility = request.visibility().into();

    let channel_model = db
        .set_channel_visibility(channel_id, visibility, session.user_id())
        .await?;
    let root_id = channel_model.root_id();
    let channel = Channel::from_model(channel_model);

    let mut connection_pool = session.connection_pool().await;
    for (user_id, role) in connection_pool
        .channel_user_ids(root_id)
        .collect::<Vec<_>>()
        .into_iter()
    {
        let update = if role.can_see_channel(channel.visibility) {
            connection_pool.subscribe_to_channel(user_id, channel_id, role);
            proto::UpdateChannels {
                channels: vec![channel.to_proto()],
                ..Default::default()
            }
        } else {
            connection_pool.unsubscribe_from_channel(&user_id, &channel_id);
            proto::UpdateChannels {
                delete_channels: vec![channel.id.to_proto()],
                ..Default::default()
            }
        };

        for connection_id in connection_pool.user_connection_ids(user_id) {
            session.peer.send(connection_id, update.clone())?;
        }
    }

    response.send(proto::Ack {})?;
    Ok(())
}

/// Alter the role for a user in the channel.
async fn set_channel_member_role(
    request: proto::SetChannelMemberRole,
    response: Response<proto::SetChannelMemberRole>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let channel_id = ChannelId::from_proto(request.channel_id);
    let member_id = UserId::from_proto(request.user_id);
    let result = db
        .set_channel_member_role(
            channel_id,
            session.user_id(),
            member_id,
            request.role().into(),
        )
        .await?;

    match result {
        db::SetMemberRoleResult::MembershipUpdated(membership_update) => {
            let mut connection_pool = session.connection_pool().await;
            notify_membership_updated(
                &mut connection_pool,
                membership_update,
                member_id,
                &session.peer,
            )
        }
        db::SetMemberRoleResult::InviteUpdated(channel) => {
            let update = proto::UpdateChannels {
                channel_invitations: vec![channel.to_proto()],
                ..Default::default()
            };

            for connection_id in session
                .connection_pool()
                .await
                .user_connection_ids(member_id)
            {
                session.peer.send(connection_id, update.clone())?;
            }
        }
    }

    response.send(proto::Ack {})?;
    Ok(())
}

/// Change the name of a channel
async fn rename_channel(
    request: proto::RenameChannel,
    response: Response<proto::RenameChannel>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let channel_id = ChannelId::from_proto(request.channel_id);
    let channel_model = db
        .rename_channel(channel_id, session.user_id(), &request.name)
        .await?;
    let root_id = channel_model.root_id();
    let channel = Channel::from_model(channel_model);

    response.send(proto::RenameChannelResponse {
        channel: Some(channel.to_proto()),
    })?;

    let connection_pool = session.connection_pool().await;
    let update = proto::UpdateChannels {
        channels: vec![channel.to_proto()],
        ..Default::default()
    };
    for (connection_id, role) in connection_pool.channel_connection_ids(root_id) {
        if role.can_see_channel(channel.visibility) {
            session.peer.send(connection_id, update.clone())?;
        }
    }

    Ok(())
}

/// Move a channel to a new parent.
async fn move_channel(
    request: proto::MoveChannel,
    response: Response<proto::MoveChannel>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let to = ChannelId::from_proto(request.to);

    let (root_id, channels) = session
        .db()
        .await
        .move_channel(channel_id, to, session.user_id())
        .await?;

    let connection_pool = session.connection_pool().await;
    for (connection_id, role) in connection_pool.channel_connection_ids(root_id) {
        let channels = channels
            .iter()
            .filter_map(|channel| {
                if role.can_see_channel(channel.visibility) {
                    Some(channel.to_proto())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if channels.is_empty() {
            continue;
        }

        let update = proto::UpdateChannels {
            channels,
            ..Default::default()
        };

        session.peer.send(connection_id, update.clone())?;
    }

    response.send(Ack {})?;
    Ok(())
}

async fn reorder_channel(
    request: proto::ReorderChannel,
    response: Response<proto::ReorderChannel>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let direction = request.direction();

    let updated_channels = session
        .db()
        .await
        .reorder_channel(channel_id, direction, session.user_id())
        .await?;

    if let Some(root_id) = updated_channels.first().map(|channel| channel.root_id()) {
        let connection_pool = session.connection_pool().await;
        for (connection_id, role) in connection_pool.channel_connection_ids(root_id) {
            let channels = updated_channels
                .iter()
                .filter_map(|channel| {
                    if role.can_see_channel(channel.visibility) {
                        Some(channel.to_proto())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if channels.is_empty() {
                continue;
            }

            let update = proto::UpdateChannels {
                channels,
                ..Default::default()
            };

            session.peer.send(connection_id, update.clone())?;
        }
    }

    response.send(Ack {})?;
    Ok(())
}

/// Get the list of channel members
async fn get_channel_members(
    request: proto::GetChannelMembers,
    response: Response<proto::GetChannelMembers>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let channel_id = ChannelId::from_proto(request.channel_id);
    let limit = if request.limit == 0 {
        u16::MAX as u64
    } else {
        request.limit
    };

    let channel = db.get_channel(channel_id, session.user_id()).await?;

    let (members, users) = session
        .app_state
        .user_service
        .search_channel_members(&channel, &request.query, limit as u32)
        .await?;
    let users = users.into_iter().map(proto::User::from).collect();

    response.send(proto::GetChannelMembersResponse { members, users })?;
    Ok(())
}

const STATUS_CLEAR_AFTER_MINUTES: &[u32] = &[30, 60, 240, 1_440, 10_080];

fn validate_status_request(
    text: &str,
    emoji: Option<&str>,
    clear_after_minutes: Option<u32>,
) -> Result<()> {
    if text.is_empty() || text.chars().count() > 100 {
        return Err(anyhow!("status text must contain between 1 and 100 characters").into());
    }
    if let Some(emoji) = emoji
        && emojis::get(emoji).is_none()
    {
        return Err(anyhow!("status emoji is not recognized").into());
    }
    if let Some(minutes) = clear_after_minutes
        && !STATUS_CLEAR_AFTER_MINUTES.contains(&minutes)
    {
        return Err(anyhow!("unsupported status clear-after duration").into());
    }
    Ok(())
}

async fn set_status(
    request: proto::SetStatus,
    response: Response<proto::SetStatus>,
    session: MessageContext,
) -> Result<()> {
    let text = request.text.trim();
    validate_status_request(text, request.emoji.as_deref(), request.clear_after_minutes)?;

    let now = time::OffsetDateTime::now_utc();
    let expires_at = request.clear_after_minutes.map(|minutes| {
        let expires_at = now + Duration::from_secs(u64::from(minutes) * 60);
        time::PrimitiveDateTime::new(expires_at.date(), expires_at.time())
    });
    let status = UserStatusStore::new(session.app_state.db.clone())
        .upsert_custom_status(
            session.user_id(),
            request.emoji.clone(),
            text.to_string(),
            expires_at,
        )
        .await?;
    broadcast_user_status_update(&session, session.user_id(), Some(status)).await?;
    response.send(proto::SetStatusResponse {})?;
    Ok(())
}

async fn clear_status(
    _: proto::ClearStatus,
    response: Response<proto::ClearStatus>,
    session: MessageContext,
) -> Result<()> {
    UserStatusStore::new(session.app_state.db.clone())
        .delete_custom_status(session.user_id())
        .await?;
    broadcast_user_status_update(&session, session.user_id(), None).await?;
    response.send(proto::Ack {})?;
    Ok(())
}

async fn broadcast_user_status_update(
    session: &MessageContext,
    user_id: UserId,
    status: Option<UserCustomStatus>,
) -> Result<()> {
    let update = proto::UpdateUserStatus {
        user_id: user_id.to_proto(),
        status: status.map(|status| proto::UserCustomStatus {
            emoji: status.emoji,
            text: status.status_text,
            expires_at: status
                .expires_at
                .map(|expires_at| expires_at.assume_utc().unix_timestamp() as u64),
        }),
    };
    let connection_pool = session.connection_pool().await;
    for connection_id in connection_pool.connection_ids() {
        session.peer.send(connection_id, update.clone())?;
    }
    Ok(())
}

async fn respond_to_join_request(
    request: proto::RespondToJoinRequest,
    response: Response<proto::RespondToJoinRequest>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let requester_id = UserId::from_proto(request.requesting_user_id);
    let responder_id = session.user_id();
    let db = session.app_state.db.clone();
    let channel = db
        .transaction(|tx| {
            let db = db.clone();
            async move {
                let channel = db.get_channel_internal(channel_id, &tx).await?;
                db.check_user_is_channel_admin(&channel, responder_id, &tx)
                    .await?;
                Ok(channel)
            }
        })
        .await?;

    let store = JoinRequestStore::new(db.clone());
    let handled = if request.approve {
        store.approve_join_request(channel_id, requester_id).await?
    } else {
        store.deny_join_request(channel_id, requester_id).await?
    };
    if !handled {
        return Err(anyhow!("join request no longer exists").into());
    }

    let notification = if request.approve {
        rpc::Notification::JoinRequestApproved {
            channel_id: channel_id.to_proto(),
            channel_name: channel.name.clone(),
        }
    } else {
        rpc::Notification::JoinRequestDenied {
            channel_id: channel_id.to_proto(),
            channel_name: channel.name.clone(),
            reason: request.denial_reason.clone(),
        }
    };
    let notifications = db
        .transaction(|tx| {
            let db = db.clone();
            let notification = notification.clone();
            async move {
                Ok(db
                    .create_notification(requester_id, notification, false, &tx)
                    .await?
                    .into_iter()
                    .collect())
            }
        })
        .await?;

    let pending_request_count = store.count_pending_requests(channel_id).await?;
    let membership_update = if request.approve {
        Some(MembershipUpdated {
            channel_id,
            new_channels: db.get_channels_for_user(requester_id).await?,
            removed_channels: Vec::new(),
        })
    } else {
        None
    };
    let mut connection_pool = session.connection_pool().await;
    if let Some(membership_update) = membership_update {
        notify_membership_updated(
            &mut connection_pool,
            membership_update,
            requester_id,
            &session.peer,
        );
    }
    send_notifications(&connection_pool, &session.peer, notifications);
    send_pending_join_request_count_update(
        &session.peer,
        &connection_pool,
        channel.root_id(),
        channel_id,
        pending_request_count,
    )?;
    for connection_id in connection_pool.user_connection_ids(requester_id) {
        session.peer.send(
            connection_id,
            proto::JoinRequestResponded {
                channel_id: channel_id.to_proto(),
                approved: request.approve,
                denial_reason: request.denial_reason.clone(),
            },
        )?;
    }

    response.send(proto::RespondToJoinRequestResponse { success: true })
}

/// Accept or decline a channel invitation.
async fn respond_to_channel_invite(
    request: proto::RespondToChannelInvite,
    response: Response<proto::RespondToChannelInvite>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let channel_id = ChannelId::from_proto(request.channel_id);
    let RespondToChannelInvite {
        membership_update,
        notifications,
    } = db
        .respond_to_channel_invite(channel_id, session.user_id(), request.accept)
        .await?;

    let mut connection_pool = session.connection_pool().await;
    if let Some(membership_update) = membership_update {
        notify_membership_updated(
            &mut connection_pool,
            membership_update,
            session.user_id(),
            &session.peer,
        );
    } else {
        let update = proto::UpdateChannels {
            remove_channel_invitations: vec![channel_id.to_proto()],
            ..Default::default()
        };

        for connection_id in connection_pool.user_connection_ids(session.user_id()) {
            session.peer.send(connection_id, update.clone())?;
        }
    };

    send_notifications(&connection_pool, &session.peer, notifications);

    response.send(proto::Ack {})?;

    Ok(())
}

/// Join the channels' room
async fn join_channel(
    request: proto::JoinChannel,
    response: Response<proto::JoinChannel>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    join_channel_internal(channel_id, Box::new(response), session).await
}

trait JoinChannelInternalResponse {
    fn send(self, result: proto::JoinRoomResponse) -> Result<()>;
}
impl JoinChannelInternalResponse for Response<proto::JoinChannel> {
    fn send(self, result: proto::JoinRoomResponse) -> Result<()> {
        Response::<proto::JoinChannel>::send(self, result)
    }
}
impl JoinChannelInternalResponse for Response<proto::JoinRoom> {
    fn send(self, result: proto::JoinRoomResponse) -> Result<()> {
        Response::<proto::JoinRoom>::send(self, result)
    }
}

async fn join_channel_internal(
    channel_id: ChannelId,
    response: Box<impl JoinChannelInternalResponse>,
    session: MessageContext,
) -> Result<()> {
    let joined_room = {
        let mut db = session.db().await;
        // If sim quits without leaving the room, and the user re-opens sim before the
        // RECONNECT_TIMEOUT, we need to make sure that we kick the user out of the previous
        // room they were in.
        if let Some(connection) = db.stale_room_connection(session.user_id()).await? {
            tracing::info!(
                stale_connection_id = %connection,
                "cleaning up stale connection",
            );
            drop(db);
            leave_room_for_session(&session, connection).await?;
            db = session.db().await;
        }

        let (joined_room, membership_updated, role) = db
            .join_channel(channel_id, session.user_id(), session.connection_id)
            .await?;

        let live_kit_connection_info =
            session
                .app_state
                .livekit_client
                .as_ref()
                .and_then(|live_kit| {
                    let (can_publish, token) = if role == ChannelRole::Guest {
                        (
                            false,
                            live_kit
                                .guest_token(
                                    &joined_room.room.livekit_room,
                                    &session.user_id().to_string(),
                                )
                                .trace_err()?,
                        )
                    } else {
                        (
                            true,
                            live_kit
                                .room_token(
                                    &joined_room.room.livekit_room,
                                    &session.user_id().to_string(),
                                )
                                .trace_err()?,
                        )
                    };

                    Some(LiveKitConnectionInfo {
                        server_url: live_kit.url().into(),
                        token,
                        can_publish,
                    })
                });

        response.send(proto::JoinRoomResponse {
            room: Some(joined_room.room.clone()),
            channel_id: joined_room
                .channel
                .as_ref()
                .map(|channel| channel.id.to_proto()),
            live_kit_connection_info,
        })?;

        let mut connection_pool = session.connection_pool().await;
        if let Some(membership_updated) = membership_updated {
            notify_membership_updated(
                &mut connection_pool,
                membership_updated,
                session.user_id(),
                &session.peer,
            );
        }

        room_updated(&joined_room.room, &session.peer);

        joined_room
    };

    channel_updated(
        &joined_room.channel.context("channel not returned")?,
        &joined_room.room,
        &session.peer,
        &*session.connection_pool().await,
    );

    update_user_contacts(session.user_id(), &session).await?;
    Ok(())
}

/// Start editing the channel notes
async fn join_channel_buffer(
    request: proto::JoinChannelBuffer,
    response: Response<proto::JoinChannelBuffer>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let channel_id = ChannelId::from_proto(request.channel_id);

    let open_response = db
        .join_channel_buffer(channel_id, session.user_id(), session.connection_id)
        .await?;

    let collaborators = open_response.collaborators.clone();
    response.send(open_response)?;

    let update = UpdateChannelBufferCollaborators {
        channel_id: channel_id.to_proto(),
        collaborators: collaborators.clone(),
    };
    channel_buffer_updated(
        session.connection_id,
        collaborators
            .iter()
            .filter_map(|collaborator| Some(collaborator.peer_id?.into())),
        &update,
        &session.peer,
    );

    Ok(())
}

/// Edit the channel notes
async fn update_channel_buffer(
    request: proto::UpdateChannelBuffer,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let channel_id = ChannelId::from_proto(request.channel_id);

    let (collaborators, epoch, version) = db
        .update_channel_buffer(channel_id, session.user_id(), &request.operations)
        .await?;

    channel_buffer_updated(
        session.connection_id,
        collaborators.clone(),
        &proto::UpdateChannelBuffer {
            channel_id: channel_id.to_proto(),
            operations: request.operations,
        },
        &session.peer,
    );

    let pool = &*session.connection_pool().await;

    let non_collaborators =
        pool.channel_connection_ids(channel_id)
            .filter_map(|(connection_id, _)| {
                if collaborators.contains(&connection_id) {
                    None
                } else {
                    Some(connection_id)
                }
            });

    broadcast(None, non_collaborators, |peer_id| {
        session.peer.send(
            peer_id,
            proto::UpdateChannels {
                latest_channel_buffer_versions: vec![proto::ChannelBufferVersion {
                    channel_id: channel_id.to_proto(),
                    epoch: epoch as u64,
                    version: version.clone(),
                }],
                ..Default::default()
            },
        )
    });

    Ok(())
}

/// Rejoin the channel notes after a connection blip
async fn rejoin_channel_buffers(
    request: proto::RejoinChannelBuffers,
    response: Response<proto::RejoinChannelBuffers>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let buffers = db
        .rejoin_channel_buffers(&request.buffers, session.user_id(), session.connection_id)
        .await?;

    for rejoined_buffer in &buffers {
        let collaborators_to_notify = rejoined_buffer
            .buffer
            .collaborators
            .iter()
            .filter_map(|c| Some(c.peer_id?.into()));
        channel_buffer_updated(
            session.connection_id,
            collaborators_to_notify,
            &proto::UpdateChannelBufferCollaborators {
                channel_id: rejoined_buffer.buffer.channel_id,
                collaborators: rejoined_buffer.buffer.collaborators.clone(),
            },
            &session.peer,
        );
    }

    response.send(proto::RejoinChannelBuffersResponse {
        buffers: buffers.into_iter().map(|b| b.buffer).collect(),
    })?;

    Ok(())
}

/// Stop editing the channel notes
async fn leave_channel_buffer(
    request: proto::LeaveChannelBuffer,
    response: Response<proto::LeaveChannelBuffer>,
    session: MessageContext,
) -> Result<()> {
    let db = session.db().await;
    let channel_id = ChannelId::from_proto(request.channel_id);

    let left_buffer = db
        .leave_channel_buffer(channel_id, session.connection_id)
        .await?;

    response.send(Ack {})?;

    channel_buffer_updated(
        session.connection_id,
        left_buffer.connections,
        &proto::UpdateChannelBufferCollaborators {
            channel_id: channel_id.to_proto(),
            collaborators: left_buffer.collaborators,
        },
        &session.peer,
    );

    Ok(())
}

fn channel_buffer_updated<T: EnvelopedMessage>(
    sender_id: ConnectionId,
    collaborators: impl IntoIterator<Item = ConnectionId>,
    message: &T,
    peer: &Peer,
) {
    broadcast(Some(sender_id), collaborators, |peer_id| {
        peer.send(peer_id, message.clone())
    });
}

fn send_notifications(
    connection_pool: &ConnectionPool,
    peer: &Peer,
    notifications: db::NotificationBatch,
) {
    for (user_id, notification) in notifications {
        for connection_id in connection_pool.user_connection_ids(user_id) {
            if let Err(error) = peer.send(
                connection_id,
                proto::AddNotification {
                    notification: Some(notification.clone()),
                },
            ) {
                tracing::error!(
                    "failed to send notification to {:?} {}",
                    connection_id,
                    error
                );
            }
        }
    }
}

/// Send a message to the channel
async fn send_channel_message(
    request: proto::SendChannelMessage,
    response: Response<proto::SendChannelMessage>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let priority = channel_message_priority_from_proto(request.priority)?;
    let file_ids = request
        .file_ids
        .iter()
        .map(|file_id| {
            uuid::Uuid::parse_str(file_id)
                .context("invalid file id")
                .map_err(Error::from)
        })
        .collect::<Result<Vec<_>>>()?;
    let db = session.db().await;
    let group_mentions = request
        .mentions
        .iter()
        .filter(|mention| mention.group_id != 0)
        .map(|mention| GroupId::from_proto(mention.group_id))
        .collect::<Vec<_>>();
    let mentions = expand_group_mentions(&request.mentions, &db).await?;
    let mut message = db
        .create_channel_message(NewChannelMessage {
            channel_id,
            sender_id: session.user_id(),
            body: request.body,
            nonce: request.nonce.context("missing channel message nonce")?,
            mentions,
            reply_to_message_id: request.reply_to_message_id.map(MessageId::from_proto),
            scheduled_at: None,
            priority,
        })
        .await?;
    if !file_ids.is_empty() {
        let attachments = file_store(&session)
            .attach_files_to_message(
                channel_id,
                MessageId::from_proto(message.id),
                session.user_id(),
                file_ids,
            )
            .await
            .map_err(file_store_rpc_error)?;
        message.files = attachments
            .into_iter()
            .map(db::file_store::FileAttachment::to_proto)
            .collect();
    }

    response.send(proto::SendChannelMessageResponse {
        message: Some(message.clone()),
    })?;
    drop(db);
    broadcast_channel_message_sent(&session, channel_id, message.clone()).await?;
    dispatch_group_mention_notifications(&session, channel_id, &group_mentions, &message).await?;
    if priority == 2 {
        dispatch_urgent_notifications(&session, channel_id, &message).await?;
    }
    Ok(())
}

async fn dispatch_group_mention_notifications(
    session: &MessageContext,
    channel_id: ChannelId,
    group_mentions: &[GroupId],
    message: &proto::ChannelMessage,
) -> Result<()> {
    let sender_id = session.user_id();
    let db = session.db().await;
    let mut recipient_ids = HashSet::default();
    for group_id in group_mentions.iter().copied().collect::<HashSet<_>>() {
        for recipient_id in db.get_group_member_ids(group_id).await? {
            if recipient_id != sender_id {
                recipient_ids.insert((group_id.to_proto(), recipient_id));
            }
        }
    }
    if recipient_ids.is_empty() {
        return Ok(());
    }

    let database = db.0.clone();
    let message_preview = message.body.chars().take(200).collect::<String>();
    let notifications = database
        .transaction(|tx| {
            let database = database.clone();
            let recipient_ids = recipient_ids.clone();
            let message_preview = message_preview.clone();
            async move {
                let mut notifications = Vec::new();
                for (group_id, recipient_id) in &recipient_ids {
                    if let Some(notification) = database
                        .create_notification(
                            *recipient_id,
                            Notification::GroupMention {
                                message_id: message.id,
                                channel_id: channel_id.to_proto(),
                                sender_id: sender_id.to_proto(),
                                group_id: *group_id,
                                message_preview: message_preview.clone(),
                            },
                            true,
                            &tx,
                        )
                        .await?
                    {
                        notifications.push(notification);
                    }
                }
                Ok(notifications)
            }
        })
        .await?;
    let connection_pool = session.connection_pool().await;
    send_notifications(&connection_pool, &session.peer, notifications);
    Ok(())
}

async fn dispatch_urgent_notifications(
    session: &MessageContext,
    channel_id: ChannelId,
    message: &proto::ChannelMessage,
) -> Result<()> {
    let db = session.db().await;
    let database = db.0.clone();
    let channel = db.get_channel(channel_id, session.user_id()).await?;
    let root_channel_id = channel.root_id();
    let sender_id = session.user_id();
    let message_id = MessageId::from_proto(message.id);
    let message_preview = message.body.chars().take(200).collect::<String>();
    let notifications = database
        .transaction(|tx| {
            let database = database.clone();
            let message_preview = message_preview.clone();
            async move {
                let members = db::channel_member::Entity::find()
                    .filter(db::channel_member::Column::ChannelId.eq(root_channel_id))
                    .filter(db::channel_member::Column::Accepted.eq(true))
                    .all(&*tx)
                    .await?;
                let mut notifications = Vec::new();
                for member in members {
                    if member.user_id == sender_id {
                        continue;
                    }
                    if let Some(notification) = database
                        .create_notification(
                            member.user_id,
                            Notification::UrgentMessage {
                                message_id: message_id.to_proto(),
                                channel_id: channel_id.to_proto(),
                                sender_id: sender_id.to_proto(),
                                message_preview: message_preview.clone(),
                            },
                            true,
                            &tx,
                        )
                        .await?
                    {
                        notifications.push(notification);
                    }
                }
                Ok(notifications)
            }
        })
        .await?;

    let connection_pool = session.connection_pool().await;
    for (recipient_id, notification) in &notifications {
        for connection_id in connection_pool.user_connection_ids(*recipient_id) {
            session.peer.send(
                connection_id,
                proto::UrgentMessageNotification {
                    channel_id: channel_id.to_proto(),
                    message_id: message_id.to_proto(),
                    sender_id: sender_id.to_proto(),
                    message_preview: message_preview.clone(),
                },
            )?;
            session.peer.send(
                connection_id,
                proto::AddNotification {
                    notification: Some(notification.clone()),
                },
            )?;
        }
    }
    Ok(())
}

async fn expand_group_mentions(
    mentions: &[proto::ChatMention],
    db: &Database,
) -> Result<Vec<proto::ChatMention>> {
    let mut group_members = HashMap::default();
    for group_id in mentions
        .iter()
        .filter(|mention| mention.group_id != 0)
        .map(|mention| GroupId::from_proto(mention.group_id))
        .collect::<HashSet<_>>()
    {
        group_members.insert(group_id, db.get_group_member_ids(group_id).await?);
    }
    Ok(expand_group_mentions_from_members(
        mentions,
        &group_members,
    )?)
}

fn expand_group_mentions_from_members(
    mentions: &[proto::ChatMention],
    group_members: &HashMap<GroupId, Vec<UserId>>,
) -> anyhow::Result<Vec<proto::ChatMention>> {
    let mut expanded = Vec::new();
    for mention in mentions {
        if mention.group_id == 0 {
            expanded.push(mention.clone());
            continue;
        }
        let user_ids = group_members
            .get(&GroupId::from_proto(mention.group_id))
            .with_context(|| format!("group {} not found", mention.group_id))?;
        for user_id in user_ids {
            expanded.push(proto::ChatMention {
                range: mention.range.clone(),
                user_id: user_id.to_proto(),
                group_id: mention.group_id,
            });
        }
    }
    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn expand_group_mentions_preserves_individual_mentions_and_ranges() {
        let mentions = vec![
            proto::ChatMention {
                range: Some(proto::Range { start: 0, end: 4 }),
                user_id: 10,
                group_id: 0,
            },
            proto::ChatMention {
                range: Some(proto::Range { start: 5, end: 9 }),
                user_id: 0,
                group_id: 7,
            },
        ];
        let members = [(
            GroupId::from_proto(7),
            vec![UserId::from_proto(20), UserId::from_proto(30)],
        )]
        .into_iter()
        .collect();

        let expanded = expand_group_mentions_from_members(&mentions, &members).unwrap();

        assert_eq!(
            expanded,
            vec![
                mentions[0].clone(),
                proto::ChatMention {
                    range: mentions[1].range.clone(),
                    user_id: 20,
                    group_id: 7,
                },
                proto::ChatMention {
                    range: mentions[1].range.clone(),
                    user_id: 30,
                    group_id: 7,
                },
            ]
        );
    }

    #[test]
    fn expand_group_mentions_rejects_missing_groups() {
        let mentions = [proto::ChatMention {
            range: Some(proto::Range { start: 0, end: 4 }),
            user_id: 0,
            group_id: 99,
        }];

        assert!(expand_group_mentions_from_members(&mentions, &HashMap::default()).is_err());
    }

    proptest! {
        #[test]
        fn status_text_validation_matches_the_character_boundary(text in any::<String>()) {
            let text = text.chars().take(200).collect::<String>();
            let result = validate_status_request(&text, None, None);

            prop_assert_eq!(result.is_ok(), !text.is_empty() && text.chars().count() <= 100);
        }

        #[test]
        fn expand_group_mentions_preserves_group_membership_mapping(
            member_ids in prop::collection::vec(1_u64..10_000, 1..20),
        ) {
            let mut member_ids = member_ids
                .into_iter()
                .map(UserId::from_proto)
                .collect::<Vec<_>>();
            member_ids.sort_unstable();
            member_ids.dedup();
            let mentions = [proto::ChatMention {
                range: Some(proto::Range { start: 2, end: 8 }),
                user_id: 0,
                group_id: 7,
            }];
            let members = [(GroupId::from_proto(7), member_ids.clone())]
                .into_iter()
                .collect();

            let expanded = expand_group_mentions_from_members(&mentions, &members).unwrap();

            prop_assert!(expanded.iter().all(|mention| mention.group_id == 7));
            prop_assert_eq!(
                expanded.iter().map(|mention| mention.user_id).collect::<Vec<_>>(),
                member_ids.iter().map(|id| id.to_proto()).collect::<Vec<_>>(),
            );
        }
    }
}

fn channel_message_priority_from_proto(priority: Option<i32>) -> Result<i16> {
    match priority.unwrap_or_default() {
        0 => Ok(0),
        1 => Ok(1),
        2 => Ok(2),
        _ => Err(anyhow!("invalid channel message priority").into()),
    }
}

async fn schedule_channel_message(
    request: proto::ScheduleChannelMessage,
    response: Response<proto::ScheduleChannelMessage>,
    session: MessageContext,
) -> Result<()> {
    let scheduled_at = timestamp_millis_to_primitive_datetime(request.scheduled_at)?;
    let nonce = request.nonce.unwrap_or_else(|| {
        tracing::warn!(
            channel_id = request.channel_id,
            user_id = %session.user_id(),
            "missing scheduled message nonce; generating server-side fallback"
        );
        rand::rng().random::<u128>().into()
    });
    let store = ScheduledMessageStore::new(session.app_state.db.clone());
    let scheduled_message_id = store
        .create(NewScheduledMessage {
            channel_id: ChannelId::from_proto(request.channel_id),
            sender_id: session.user_id(),
            body: request.body,
            scheduled_at,
            nonce,
            mentions: request.mentions,
        })
        .await?;

    response.send(proto::ScheduleChannelMessageResponse {
        scheduled_message_id: scheduled_message_id.to_proto(),
    })
}

async fn cancel_scheduled_message(
    request: proto::CancelScheduledMessage,
    response: Response<proto::CancelScheduledMessage>,
    session: MessageContext,
) -> Result<()> {
    let store = ScheduledMessageStore::new(session.app_state.db.clone());
    store
        .cancel(
            db::ScheduledMessageId::from_proto(request.scheduled_message_id),
            ChannelId::from_proto(request.channel_id),
            session.user_id(),
        )
        .await?;
    response.send(proto::Ack {})
}

async fn update_scheduled_message(
    request: proto::UpdateScheduledMessage,
    response: Response<proto::UpdateScheduledMessage>,
    session: MessageContext,
) -> Result<()> {
    let scheduled_at = request
        .scheduled_at
        .map(timestamp_millis_to_primitive_datetime)
        .transpose()?;
    let store = ScheduledMessageStore::new(session.app_state.db.clone());
    store
        .update(ScheduledMessageUpdate {
            scheduled_message_id: db::ScheduledMessageId::from_proto(request.scheduled_message_id),
            channel_id: ChannelId::from_proto(request.channel_id),
            sender_id: session.user_id(),
            body: request.body,
            scheduled_at,
            mentions: Some(request.mentions),
        })
        .await?;
    response.send(proto::Ack {})
}

async fn get_scheduled_messages(
    request: proto::GetScheduledMessages,
    response: Response<proto::GetScheduledMessages>,
    session: MessageContext,
) -> Result<()> {
    let store = ScheduledMessageStore::new(session.app_state.db.clone());
    let messages = store
        .list_for_user(session.user_id(), ChannelId::from_proto(request.channel_id))
        .await?
        .into_iter()
        .map(|message| message.to_proto())
        .collect();
    response.send(proto::GetScheduledMessagesResponse { messages })
}

async fn get_bookmarks(
    request: proto::GetBookmarks,
    response: Response<proto::GetBookmarks>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    ensure_can_read_bookmarks(&session, channel_id).await?;
    let store = BookmarkStore::new(session.app_state.db.clone());
    let bookmarks = store
        .get_bookmarks(channel_id)
        .await?
        .into_iter()
        .map(db::bookmark_store::Bookmark::to_proto)
        .collect();
    response.send(proto::GetBookmarksResponse { bookmarks })
}

async fn get_pending_join_requests(
    request: proto::GetPendingJoinRequests,
    response: Response<proto::GetPendingJoinRequests>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let user_id = session.user_id();
    let db = session.app_state.db.clone();
    db.transaction(|tx| {
        let db = db.clone();
        async move {
            let channel = db.get_channel_internal(channel_id, &tx).await?;
            db.check_user_is_channel_admin(&channel, user_id, &tx).await
        }
    })
    .await?;

    let requests = JoinRequestStore::new(db)
        .get_pending_requests(channel_id)
        .await?
        .into_iter()
        .map(|request| proto::PendingJoinRequest {
            user_id: request.user_id.to_proto(),
            reason: request.reason,
            created_at: request.created_at.assume_utc().unix_timestamp() as u64,
        })
        .collect();
    response.send(proto::GetPendingJoinRequestsResponse { requests })
}

async fn request_join_channel(
    request: proto::RequestJoinChannel,
    response: Response<proto::RequestJoinChannel>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let requester_id = session.user_id();
    if request
        .reason
        .as_ref()
        .is_some_and(|reason| reason.chars().count() > JOIN_REQUEST_REASON_MAX_CHARS)
    {
        return Err(anyhow!(
            "join request reasons must be at most {JOIN_REQUEST_REASON_MAX_CHARS} characters"
        )
        .into());
    }
    let db = session.app_state.db.clone();
    let channel = db
        .transaction(|tx| {
            let db = db.clone();
            async move {
                let channel = db.get_channel_internal(channel_id, &tx).await?;
                if channel.visibility != db::ChannelVisibility::Members {
                    return Err(anyhow!("public channels can be joined directly").into());
                }
                match db
                    .channel_role_for_user(&channel, requester_id, &tx)
                    .await?
                {
                    Some(ChannelRole::Banned) | None => Ok(channel),
                    Some(_) => Err(anyhow!("user is already a channel member").into()),
                }
            }
        })
        .await?;

    check_join_request_rate_limit(&session.app_state, requester_id)?;

    let join_request = JoinRequestStore::new(db.clone())
        .request_join(channel_id, requester_id, request.reason.clone())
        .await?;

    let root_channel_id = channel.root_id();
    let (notifications, admin_user_ids) = db
        .transaction(|tx| {
            let db = db.clone();
            let channel_name = channel.name.clone();
            let reason = request.reason.clone();
            async move {
                let admins = db::channel_member::Entity::find()
                    .filter(db::channel_member::Column::ChannelId.eq(root_channel_id))
                    .filter(db::channel_member::Column::Accepted.eq(true))
                    .filter(db::channel_member::Column::Role.eq(ChannelRole::Admin))
                    .all(&*tx)
                    .await?;
                let mut notifications = Vec::new();
                let mut admin_user_ids = Vec::new();
                for admin in admins {
                    admin_user_ids.push(admin.user_id);
                    if let Some(notification) = db
                        .create_notification(
                            admin.user_id,
                            rpc::Notification::JoinRequest {
                                channel_id: channel_id.to_proto(),
                                channel_name: channel_name.clone(),
                                requesting_user_id: requester_id.to_proto(),
                                requesting_user_name: requester_id.to_string(),
                                reason: reason.clone(),
                            },
                            true,
                            &tx,
                        )
                        .await?
                    {
                        notifications.push(notification);
                    }
                }
                Ok((notifications, admin_user_ids))
            }
        })
        .await?;

    let pending_request_count = JoinRequestStore::new(db.clone())
        .count_pending_requests(channel_id)
        .await?;
    let connection_pool = session.connection_pool().await;
    send_notifications(&connection_pool, &session.peer, notifications);
    send_pending_join_request_count_update(
        &session.peer,
        &connection_pool,
        root_channel_id,
        channel_id,
        pending_request_count,
    )?;
    for admin_user_id in admin_user_ids {
        for connection_id in connection_pool.user_connection_ids(admin_user_id) {
            session.peer.send(
                connection_id,
                proto::JoinRequestAdded {
                    channel_id: channel_id.to_proto(),
                    requesting_user_id: requester_id.to_proto(),
                    reason: request.reason.clone(),
                    created_at: join_request.created_at.assume_utc().unix_timestamp() as u64,
                },
            )?;
        }
    }

    response.send(proto::RequestJoinChannelResponse { success: true })
}

fn check_join_request_rate_limit(app_state: &AppState, user_id: UserId) -> Result<()> {
    let now = Instant::now();
    let mut attempts_by_user = app_state.join_request_attempts.lock();
    let attempts = attempts_by_user.entry(user_id).or_default();
    attempts.retain(|attempt| now.duration_since(*attempt) < JOIN_REQUEST_RATE_LIMIT_WINDOW);
    if attempts.len() >= JOIN_REQUEST_RATE_LIMIT {
        return Err(anyhow!("too many join requests; try again in a minute").into());
    }
    attempts.push(now);
    Ok(())
}

async fn get_file_upload_url(
    request: proto::GetFileUploadUrl,
    response: Response<proto::GetFileUploadUrl>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    ensure_can_upload_files(&session, channel_id).await?;
    let store = file_store(&session);
    let upload_url = store
        .generate_upload_url(NewFileUpload {
            channel_id,
            filename: request.filename,
            file_size: request.file_size,
            mime_type: request.mime_type,
            uploader_id: session.user_id(),
            image_width: None,
            image_height: None,
            duration_ms: None,
        })
        .await
        .map_err(file_store_rpc_error)?;

    response.send(proto::GetFileUploadUrlResponse {
        url: upload_url.url,
        file_id: upload_url.file_id.to_string(),
        headers: upload_url.headers,
    })
}

async fn confirm_file_upload(
    request: proto::ConfirmFileUpload,
    response: Response<proto::ConfirmFileUpload>,
    session: MessageContext,
) -> Result<()> {
    let file_id = uuid::Uuid::parse_str(&request.file_id).context("invalid file id")?;
    let store = file_store(&session);
    let attachment = store
        .confirm_upload(file_id, session.user_id())
        .await
        .map_err(file_store_rpc_error)?;

    response.send(proto::ConfirmFileUploadResponse {
        attachment: Some(attachment.to_proto()),
    })
}

async fn get_file_download_url(
    request: proto::GetFileDownloadUrl,
    response: Response<proto::GetFileDownloadUrl>,
    session: MessageContext,
) -> Result<()> {
    let file_id = uuid::Uuid::parse_str(&request.file_id).context("invalid file id")?;
    let store = file_store(&session);
    let channel_id = store
        .file_channel_id(file_id)
        .await
        .map_err(file_store_rpc_error)?;
    ensure_can_read_bookmarks(&session, channel_id).await?;
    let download = store
        .get_file_download_url(file_id)
        .await
        .map_err(file_store_rpc_error)?;

    response.send(proto::GetFileDownloadUrlResponse {
        url: download.url,
        download_count: download.download_count,
    })
}

async fn add_bookmark(
    request: proto::AddBookmark,
    response: Response<proto::AddBookmark>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    ensure_can_edit_bookmarks(&session, channel_id).await?;
    let bookmark_type =
        proto::BookmarkType::from_i32(request.r#type).context("invalid bookmark type")?;
    let store = BookmarkStore::new(session.app_state.db.clone());
    let bookmark = store
        .create(NewBookmark {
            channel_id,
            label: request.label,
            description: request.description,
            bookmark_type,
            url: request.url,
            file_id: request.file_id,
            message_id: request.message_id.map(MessageId::from_proto),
            created_by: session.user_id(),
        })
        .await?;

    let bookmarks = store.get_bookmarks(channel_id).await?;
    post_bookmark_system_message(
        &session,
        channel_id,
        format!(
            "Pinned a {} bookmark: {}",
            bookmark_type_label(bookmark.bookmark_type),
            bookmark.label
        ),
    )
    .await?;
    response.send(proto::Ack {})?;
    broadcast_channel_bookmarks_update(&session, channel_id, bookmarks, Vec::new()).await
}

async fn remove_bookmark(
    request: proto::RemoveBookmark,
    response: Response<proto::RemoveBookmark>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let bookmark_id = db::BookmarkId::from_proto(request.bookmark_id);
    ensure_can_edit_bookmarks(&session, channel_id).await?;
    let store = BookmarkStore::new(session.app_state.db.clone());
    let bookmark_label = store
        .get_bookmarks(channel_id)
        .await?
        .into_iter()
        .find(|bookmark| bookmark.id == bookmark_id)
        .map(|bookmark| bookmark.label);
    let deleted = store.delete(channel_id, bookmark_id).await?;

    let bookmarks = store.get_bookmarks(channel_id).await?;
    if deleted {
        post_bookmark_system_message(
            &session,
            channel_id,
            format!(
                "Removed bookmark: {}",
                bookmark_label.unwrap_or_else(|| "Untitled bookmark".to_string())
            ),
        )
        .await?;
    }
    response.send(proto::Ack {})?;
    broadcast_channel_bookmarks_update(&session, channel_id, bookmarks, vec![bookmark_id]).await
}

async fn update_bookmark(
    request: proto::UpdateBookmark,
    response: Response<proto::UpdateBookmark>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    ensure_can_edit_bookmarks(&session, channel_id).await?;
    let store = BookmarkStore::new(session.app_state.db.clone());
    let bookmark = store
        .update(BookmarkUpdate {
            channel_id,
            bookmark_id: db::BookmarkId::from_proto(request.bookmark_id),
            label: request.label,
            description: request.description,
        })
        .await?;

    let bookmarks = store.get_bookmarks(channel_id).await?;
    post_bookmark_system_message(
        &session,
        channel_id,
        format!("Updated bookmark: {}", bookmark.label),
    )
    .await?;
    response.send(proto::Ack {})?;
    broadcast_channel_bookmarks_update(&session, channel_id, bookmarks, Vec::new()).await
}

async fn reorder_bookmarks(
    request: proto::ReorderBookmarks,
    response: Response<proto::ReorderBookmarks>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    ensure_can_edit_bookmarks(&session, channel_id).await?;
    let store = BookmarkStore::new(session.app_state.db.clone());
    store
        .reorder(
            channel_id,
            request
                .bookmark_ids
                .into_iter()
                .map(db::BookmarkId::from_proto)
                .collect(),
        )
        .await?;

    response.send(proto::Ack {})?;
    schedule_channel_bookmarks_reorder_broadcast(&session, channel_id);
    Ok(())
}

fn file_store(session: &MessageContext) -> FileStore {
    #[cfg(feature = "test-support")]
    if session.app_state.blob_store_client.is_none() {
        return FileStore::new_for_tests(
            session.app_state.db.clone(),
            FileStoreConfig::new(
                Some("test-bucket".to_string()),
                session.app_state.config.file_upload_storage_prefix.clone(),
                session
                    .app_state
                    .config
                    .file_upload_max_file_size
                    .unwrap_or(DEFAULT_FILE_UPLOAD_MAX_FILE_SIZE),
                allowed_file_upload_mime_types(
                    session
                        .app_state
                        .config
                        .file_upload_allowed_mime_types
                        .as_deref(),
                ),
            ),
            "http://file-store.test",
        );
    }

    FileStore::new(
        session.app_state.db.clone(),
        session.app_state.blob_store_client.clone(),
        FileStoreConfig::new(
            session.app_state.config.blob_store_bucket.clone(),
            session.app_state.config.file_upload_storage_prefix.clone(),
            session
                .app_state
                .config
                .file_upload_max_file_size
                .unwrap_or(DEFAULT_FILE_UPLOAD_MAX_FILE_SIZE),
            allowed_file_upload_mime_types(
                session
                    .app_state
                    .config
                    .file_upload_allowed_mime_types
                    .as_deref(),
            ),
        ),
    )
}

async fn populate_channel_message_files(
    session: &MessageContext,
    messages: &mut [proto::ChannelMessage],
) -> Result<()> {
    let message_ids = messages
        .iter()
        .map(|message| MessageId::from_proto(message.id))
        .collect::<Vec<_>>();
    let mut files_by_message_id = file_store(session)
        .get_message_files(message_ids)
        .await
        .map_err(file_store_rpc_error)?;

    for message in messages {
        message.files = files_by_message_id
            .remove(&MessageId::from_proto(message.id))
            .unwrap_or_default()
            .into_iter()
            .map(db::file_store::FileAttachment::to_proto)
            .collect();
    }

    Ok(())
}

fn allowed_file_upload_mime_types(allowed_mime_types: Option<&str>) -> Vec<String> {
    allowed_mime_types
        .into_iter()
        .flat_map(|allowed_mime_types| allowed_mime_types.split(','))
        .map(str::trim)
        .filter(|allowed_mime_type| !allowed_mime_type.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn file_store_rpc_error(error: Error) -> Error {
    let Error::Internal(error) = error else {
        return error;
    };
    let Some(file_store_error) = error.downcast_ref::<FileStoreError>() else {
        return Error::Internal(error);
    };
    let message = file_store_error.to_string();
    let rpc_error = match file_store_error {
        FileStoreError::FileTooLarge { max_file_size } => ErrorCode::FileTooLarge
            .with_tag("max_file_size", &max_file_size.to_string())
            .message(message),
        FileStoreError::EmptyFilename => ErrorCode::InvalidFileName.message(message),
        FileStoreError::UnsupportedFileType => ErrorCode::UnsupportedFileType.message(message),
        FileStoreError::StorageUnavailable
        | FileStoreError::PresignFailed(_)
        | FileStoreError::DeleteFailed(_) => ErrorCode::FileStorageUnavailable.message(message),
    };
    Error::from(rpc_error.anyhow())
}

async fn ensure_can_upload_files(session: &MessageContext, channel_id: ChannelId) -> Result<()> {
    ensure_can_edit_bookmarks(session, channel_id).await
}

async fn post_bookmark_system_message(
    session: &MessageContext,
    channel_id: ChannelId,
    body: String,
) -> Result<()> {
    let nonce = rand::rng().random::<u128>().into();
    let message = session
        .db()
        .await
        .create_channel_message(NewChannelMessage {
            channel_id,
            sender_id: session.user_id(),
            body,
            nonce,
            mentions: Vec::new(),
            reply_to_message_id: None,
            scheduled_at: None,
            priority: 0,
        })
        .await?;
    broadcast_channel_message_sent(session, channel_id, message).await
}

fn bookmark_type_label(bookmark_type: proto::BookmarkType) -> &'static str {
    match bookmark_type {
        proto::BookmarkType::BookmarkLink => "link",
        proto::BookmarkType::BookmarkFile => "file",
        proto::BookmarkType::BookmarkMessage => "message",
    }
}

async fn ensure_can_edit_bookmarks(session: &MessageContext, channel_id: ChannelId) -> Result<()> {
    let db = session.app_state.db.clone();
    let user_id = session.user_id();
    db.transaction(|tx| {
        let db = db.clone();

        async move {
            let channel = db.get_channel_internal(channel_id, &tx).await?;
            let role = db.channel_role_for_user(&channel, user_id, &tx).await?;
            match role {
                Some(ChannelRole::Admin | ChannelRole::Member) => Ok(()),
                Some(ChannelRole::Guest | ChannelRole::Talker | ChannelRole::Banned) | None => {
                    Err(ErrorCode::Forbidden.anyhow())?
                }
            }
        }
    })
    .await
}

async fn ensure_can_read_bookmarks(session: &MessageContext, channel_id: ChannelId) -> Result<()> {
    let db = session.app_state.db.clone();
    let user_id = session.user_id();
    db.transaction(|tx| {
        let db = db.clone();

        async move {
            let channel = db.get_channel_internal(channel_id, &tx).await?;
            let role = db.channel_role_for_user(&channel, user_id, &tx).await?;
            match role {
                Some(
                    ChannelRole::Admin
                    | ChannelRole::Member
                    | ChannelRole::Guest
                    | ChannelRole::Talker,
                ) => Ok(()),
                Some(ChannelRole::Banned) | None => Err(ErrorCode::Forbidden.anyhow())?,
            }
        }
    })
    .await
}

/// Delete a channel message
async fn remove_channel_message(
    request: proto::RemoveChannelMessage,
    response: Response<proto::RemoveChannelMessage>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let message_id = MessageId::from_proto(request.message_id);
    let message = session
        .db()
        .await
        .delete_channel_message(channel_id, message_id, session.user_id())
        .await?;

    file_store(&session)
        .delete_message_files(channel_id, message_id)
        .await
        .trace_err();
    response.send(proto::Ack {})?;
    broadcast_channel_message_update(&session, channel_id, message).await?;
    broadcast_channel_message_reactions_update(&session, channel_id, message_id, Vec::new()).await
}

async fn update_channel_message(
    request: proto::UpdateChannelMessage,
    response: Response<proto::UpdateChannelMessage>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let message = session
        .db()
        .await
        .update_channel_message(ChannelMessageUpdate {
            channel_id,
            message_id: MessageId::from_proto(request.message_id),
            editor_id: session.user_id(),
            body: request.body,
            nonce: request.nonce.context("missing channel message nonce")?,
            mentions: request.mentions,
        })
        .await?;

    response.send(proto::Ack {})?;
    broadcast_channel_message_update(&session, channel_id, message).await
}

async fn add_reaction(
    request: proto::AddReaction,
    response: Response<proto::AddReaction>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let message_id = MessageId::from_proto(request.message_id);
    let reactions = session
        .db()
        .await
        .insert_channel_message_reaction(
            channel_id,
            message_id,
            session.user_id(),
            request.emoji_name,
        )
        .await?;

    response.send(proto::UpdateMessageReactionsResponse {
        reactions: reactions.clone(),
    })?;
    broadcast_channel_message_reactions_update(&session, channel_id, message_id, reactions).await
}

async fn remove_reaction(
    request: proto::RemoveReaction,
    response: Response<proto::RemoveReaction>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let message_id = MessageId::from_proto(request.message_id);
    let reactions = session
        .db()
        .await
        .delete_channel_message_reaction(
            channel_id,
            message_id,
            session.user_id(),
            request.emoji_name,
        )
        .await?;

    response.send(proto::UpdateMessageReactionsResponse {
        reactions: reactions.clone(),
    })?;
    broadcast_channel_message_reactions_update(&session, channel_id, message_id, reactions).await
}

/// Mark a channel message as read
async fn acknowledge_channel_message(
    request: proto::AckChannelMessage,
    session: MessageContext,
) -> Result<()> {
    session
        .db()
        .await
        .acknowledge_channel_message(
            ChannelId::from_proto(request.channel_id),
            session.user_id(),
            MessageId::from_proto(request.message_id),
        )
        .await
}

/// Mark a channel thread as read through a specific reply
async fn acknowledge_channel_thread(
    request: proto::AckChannelThread,
    session: MessageContext,
) -> Result<()> {
    session
        .db()
        .await
        .acknowledge_channel_thread(
            ChannelId::from_proto(request.channel_id),
            session.user_id(),
            MessageId::from_proto(request.root_message_id),
            MessageId::from_proto(request.message_id),
        )
        .await
}

/// Mark a buffer version as synced
async fn acknowledge_buffer_version(
    request: proto::AckBufferOperation,
    session: MessageContext,
) -> Result<()> {
    let buffer_id = BufferId::from_proto(request.buffer_id);
    session
        .db()
        .await
        .observe_buffer_version(
            buffer_id,
            session.user_id(),
            request.epoch as i32,
            &request.version,
        )
        .await?;
    Ok(())
}

/// Start receiving chat updates for a channel
async fn join_channel_chat(
    request: proto::JoinChannelChat,
    response: Response<proto::JoinChannelChat>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    let (mut messages, role) = session
        .db()
        .await
        .join_channel_chat(channel_id, session.user_id(), session.connection_id)
        .await?;
    populate_channel_message_files(&session, &mut messages).await?;
    session
        .connection_pool()
        .await
        .subscribe_to_channel(session.user_id(), channel_id, role);
    response.send(proto::JoinChannelChatResponse {
        messages,
        done: true,
    })
}

/// Stop receiving chat updates for a channel
async fn leave_channel_chat(
    request: proto::LeaveChannelChat,
    session: MessageContext,
) -> Result<()> {
    let channel_id = ChannelId::from_proto(request.channel_id);
    session
        .db()
        .await
        .leave_channel_chat(channel_id, session.user_id(), session.connection_id)
        .await
}

/// Retrieve the chat history for a channel
async fn get_channel_messages(
    request: proto::GetChannelMessages,
    response: Response<proto::GetChannelMessages>,
    session: MessageContext,
) -> Result<()> {
    const CHANNEL_MESSAGE_PAGE_SIZE: usize = 50;

    let before_message_id = if request.before_message_id == 0 {
        None
    } else {
        Some(MessageId::from_proto(request.before_message_id))
    };
    let mut messages = session
        .db()
        .await
        .get_channel_messages(
            ChannelId::from_proto(request.channel_id),
            session.user_id(),
            before_message_id,
            CHANNEL_MESSAGE_PAGE_SIZE,
        )
        .await?;
    populate_channel_message_files(&session, &mut messages).await?;
    let done = messages.len() < CHANNEL_MESSAGE_PAGE_SIZE;
    response.send(proto::GetChannelMessagesResponse { messages, done })
}

/// Retrieve specific chat messages
async fn get_channel_messages_by_id(
    request: proto::GetChannelMessagesById,
    response: Response<proto::GetChannelMessagesById>,
    session: MessageContext,
) -> Result<()> {
    let mut messages = session
        .db()
        .await
        .get_channel_messages_by_id(
            request
                .message_ids
                .into_iter()
                .map(MessageId::from_proto)
                .collect(),
            session.user_id(),
        )
        .await?;
    populate_channel_message_files(&session, &mut messages).await?;
    response.send(proto::GetChannelMessagesResponse {
        messages,
        done: true,
    })
}

async fn search_channel_messages(
    request: proto::SearchChannelMessages,
    response: Response<proto::SearchChannelMessages>,
    session: MessageContext,
) -> Result<()> {
    let channel_id = (request.channel_id != 0).then(|| ChannelId::from_proto(request.channel_id));
    let filter_channel_id = if let Some(filter_channel) = request.filter_channel.as_deref() {
        let channels = session
            .db()
            .await
            .get_channels_for_user(session.user_id())
            .await?;
        Some(
            channels
                .channels
                .into_iter()
                .find(|channel| channel.name == filter_channel)
                .with_context(|| format!("unknown channel {filter_channel}"))?
                .id,
        )
    } else {
        None
    };
    let filter_sender_id = if let Some(filter_user) = request.filter_user.as_deref() {
        Some(
            session
                .app_state
                .user_service
                .get_user_by_github_login(filter_user)
                .await?
                .with_context(|| format!("unknown user {filter_user}"))?
                .id,
        )
    } else {
        None
    };

    let before_message_id = request.before_message_id.map(MessageId::from_proto);
    let filter_after = request
        .filter_after
        .map(timestamp_to_primitive_datetime)
        .transpose()?;
    let filter_before = request
        .filter_before
        .map(timestamp_to_primitive_datetime)
        .transpose()?;

    let (results, done) = session
        .db()
        .await
        .search_channel_messages(
            session.user_id(),
            SearchChannelMessagesParams {
                channel_id,
                query: request.query,
                before_message_id,
                limit: request.limit as usize,
                filter_channel_id,
                filter_sender_id,
                filter_after,
                filter_before,
            },
        )
        .await?;

    let users = session
        .app_state
        .user_service
        .get_users_by_ids(results.iter().map(|result| result.sender_id).collect())
        .await?
        .into_iter()
        .map(|user| (user.id, user.github_login))
        .collect::<HashMap<_, _>>();

    response.send(proto::SearchChannelMessagesResponse {
        results: results
            .into_iter()
            .map(|result| proto::SearchResult {
                sender_name: users
                    .get(&result.sender_id)
                    .cloned()
                    .unwrap_or_else(|| result.sender_id.to_string()),
                channel_id: result.channel_id.to_proto(),
                channel_name: result.channel_name,
                message: Some(result.message),
                match_positions: result.match_positions,
            })
            .collect(),
        done,
    })
}

async fn get_thread(
    request: proto::GetThread,
    response: Response<proto::GetThread>,
    session: MessageContext,
) -> Result<()> {
    let before_message_id =
        (request.before_message_id != 0).then(|| MessageId::from_proto(request.before_message_id));
    let limit = (request.limit as usize).clamp(1, 100);
    let (root_message, replies, done) = session
        .db()
        .await
        .get_channel_thread(
            ChannelId::from_proto(request.channel_id),
            session.user_id(),
            MessageId::from_proto(request.message_id),
            before_message_id,
            limit,
        )
        .await?;
    response.send(proto::GetThreadResponse {
        root_message: Some(root_message),
        replies,
        done,
    })
}

async fn get_threads(
    request: proto::GetThreads,
    response: Response<proto::GetThreads>,
    session: MessageContext,
) -> Result<()> {
    let threads = session
        .db()
        .await
        .get_channel_threads(ChannelId::from_proto(request.channel_id), session.user_id())
        .await?;
    response.send(proto::GetThreadsResponse { threads })
}

async fn run_scheduled_message_loop(
    app_state: Arc<AppState>,
    peer: Arc<Peer>,
    connection_pool: Arc<parking_lot::Mutex<ConnectionPool>>,
) {
    let store = ScheduledMessageStore::new(app_state.db.clone());
    store
        .reset_stale_processing(stale_scheduled_message_processing_cutoff())
        .await
        .trace_err();

    loop {
        app_state
            .executor
            .sleep(SCHEDULED_MESSAGE_POLL_INTERVAL)
            .await;
        deliver_due_scheduled_messages(&app_state, &peer, &connection_pool)
            .await
            .trace_err();
    }
}

const JOIN_REQUEST_EXPIRY_INTERVAL: Duration = Duration::from_secs(60 * 60);

async fn run_join_request_expiry_loop(
    app_state: Arc<AppState>,
    peer: Arc<Peer>,
    connection_pool: Arc<parking_lot::Mutex<ConnectionPool>>,
) {
    loop {
        match crate::jobs::expire_join_requests(app_state.db.clone()).await {
            Ok(expired_requests) => {
                let mut channel_ids = expired_requests
                    .into_iter()
                    .map(|request| request.channel_id)
                    .collect::<Vec<_>>();
                channel_ids.sort_unstable();
                channel_ids.dedup();

                for channel_id in channel_ids {
                    let result = app_state
                        .db
                        .transaction(|tx| {
                            let db = app_state.db.clone();
                            async move {
                                let channel = db.get_channel_internal(channel_id, &tx).await?;
                                Ok(channel.root_id())
                            }
                        })
                        .await;
                    match result {
                        Ok(root_channel_id) => {
                            let result = JoinRequestStore::new(app_state.db.clone())
                                .count_pending_requests(channel_id)
                                .await;
                            match result {
                                Ok(count) => {
                                    send_pending_join_request_count_update(
                                        &peer,
                                        &connection_pool.lock(),
                                        root_channel_id,
                                        channel_id,
                                        count,
                                    )
                                    .trace_err();
                                }
                                Err(error) => {
                                    log::error!("counting expired join requests: {error}")
                                }
                            }
                        }
                        Err(error) => log::error!("finding expired join request channel: {error}"),
                    }
                }
            }
            Err(error) => log::error!("expiring join requests: {error}"),
        }
        app_state.executor.sleep(JOIN_REQUEST_EXPIRY_INTERVAL).await;
    }
}

fn send_pending_join_request_count_update(
    peer: &Peer,
    connection_pool: &ConnectionPool,
    root_channel_id: ChannelId,
    channel_id: ChannelId,
    count: u64,
) -> Result<()> {
    let update = proto::UpdateChannels {
        pending_request_counts: vec![proto::PendingRequestCount {
            channel_id: channel_id.to_proto(),
            count: count as u32,
        }],
        ..Default::default()
    };
    for (connection_id, role) in connection_pool.channel_connection_ids(root_channel_id) {
        if role == ChannelRole::Admin {
            peer.send(connection_id, update.clone())?;
        }
    }
    Ok(())
}

async fn deliver_due_scheduled_messages(
    app_state: &AppState,
    peer: &Peer,
    connection_pool: &parking_lot::Mutex<ConnectionPool>,
) -> Result<()> {
    let store = ScheduledMessageStore::new(app_state.db.clone());
    let messages = store.pop_due().await?;

    for scheduled in messages {
        let scheduled_message_id = scheduled.id;
        let channel_id = scheduled.channel_id;
        let sender_id = scheduled.sender_id;
        let scheduled_at = scheduled.scheduled_at;
        let message = app_state
            .db
            .create_channel_message(NewChannelMessage {
                channel_id,
                sender_id,
                body: scheduled.body,
                nonce: scheduled.nonce,
                mentions: scheduled.mentions,
                reply_to_message_id: None,
                scheduled_at: Some(scheduled_at),
                priority: 0,
            })
            .await;

        match message {
            Ok(message) => {
                broadcast_channel_message_sent_to_channel(
                    &app_state.db,
                    peer,
                    channel_id,
                    message.clone(),
                )
                .await
                .trace_err();
                notify_scheduled_message_sent(
                    connection_pool,
                    peer,
                    sender_id,
                    channel_id,
                    message,
                );
                store
                    .delete_delivered(scheduled_message_id)
                    .await
                    .trace_err();
            }
            Err(error) => {
                let reason = error.to_string();
                store
                    .mark_failed(scheduled_message_id, reason.clone())
                    .await
                    .trace_err();
                notify_scheduled_message_failed(
                    connection_pool,
                    peer,
                    sender_id,
                    scheduled_message_id,
                    channel_id,
                    reason,
                );
            }
        }
    }

    Ok(())
}

fn notify_scheduled_message_sent(
    connection_pool: &parking_lot::Mutex<ConnectionPool>,
    peer: &Peer,
    sender_id: UserId,
    channel_id: ChannelId,
    message: proto::ChannelMessage,
) {
    for connection_id in connection_pool.lock().user_connection_ids(sender_id) {
        peer.send(
            connection_id,
            proto::ScheduledMessageSent {
                channel_id: channel_id.to_proto(),
                message: Some(message.clone()),
            },
        )
        .trace_err();
    }
}

fn notify_scheduled_message_failed(
    connection_pool: &parking_lot::Mutex<ConnectionPool>,
    peer: &Peer,
    sender_id: UserId,
    scheduled_message_id: db::ScheduledMessageId,
    channel_id: ChannelId,
    reason: String,
) {
    for connection_id in connection_pool.lock().user_connection_ids(sender_id) {
        peer.send(
            connection_id,
            proto::ScheduledMessageFailed {
                scheduled_message_id: scheduled_message_id.to_proto(),
                channel_id: channel_id.to_proto(),
                reason: reason.clone(),
            },
        )
        .trace_err();
    }
}

fn stale_scheduled_message_processing_cutoff() -> PrimitiveDateTime {
    let now = time::OffsetDateTime::now_utc()
        - time::Duration::seconds(SCHEDULED_MESSAGE_STALE_PROCESSING_GRACE.as_secs() as i64);
    PrimitiveDateTime::new(now.date(), now.time())
}

fn timestamp_to_primitive_datetime(timestamp: u64) -> Result<PrimitiveDateTime> {
    let timestamp = timestamp
        .try_into()
        .context("search timestamp is out of range")?;
    let timestamp = time::OffsetDateTime::from_unix_timestamp(timestamp)
        .context("search timestamp is invalid")?;
    Ok(PrimitiveDateTime::new(timestamp.date(), timestamp.time()))
}

fn timestamp_millis_to_primitive_datetime(timestamp: u64) -> Result<PrimitiveDateTime> {
    let timestamp = i128::from(timestamp)
        .checked_mul(1_000_000)
        .context("scheduled timestamp is out of range")?;
    let timestamp = time::OffsetDateTime::from_unix_timestamp_nanos(timestamp)
        .context("scheduled timestamp is invalid")?;
    Ok(PrimitiveDateTime::new(timestamp.date(), timestamp.time()))
}

async fn broadcast_channel_message_sent(
    session: &MessageContext,
    channel_id: ChannelId,
    message: proto::ChannelMessage,
) -> Result<()> {
    let db = session.db().await;
    broadcast_channel_message_sent_to_channel(&db, &session.peer, channel_id, message).await
}

async fn broadcast_channel_message_sent_to_channel(
    db: &Database,
    peer: &Peer,
    channel_id: ChannelId,
    message: proto::ChannelMessage,
) -> Result<()> {
    let connection_ids = db
        .channel_chat_participant_connection_ids(channel_id)
        .await?;
    for connection_id in connection_ids {
        peer.send(
            connection_id,
            proto::ChannelMessageSent {
                channel_id: channel_id.to_proto(),
                message: Some(message.clone()),
            },
        )?;
    }
    Ok(())
}

async fn broadcast_channel_message_update(
    session: &MessageContext,
    channel_id: ChannelId,
    message: proto::ChannelMessage,
) -> Result<()> {
    let connection_ids = session
        .db()
        .await
        .channel_chat_participant_connection_ids(channel_id)
        .await?;
    for connection_id in connection_ids {
        session.peer.send(
            connection_id,
            proto::ChannelMessageUpdate {
                channel_id: channel_id.to_proto(),
                message: Some(message.clone()),
            },
        )?;
    }
    Ok(())
}

async fn broadcast_channel_message_reactions_update(
    session: &MessageContext,
    channel_id: ChannelId,
    message_id: MessageId,
    reactions: Vec<proto::ReactionSummary>,
) -> Result<()> {
    let connection_ids = session
        .db()
        .await
        .channel_chat_participant_connection_ids(channel_id)
        .await?;
    for connection_id in connection_ids {
        session.peer.send(
            connection_id,
            proto::UpdateMessageReactions {
                channel_id: channel_id.to_proto(),
                message_id: message_id.to_proto(),
                reactions: reactions.clone(),
            },
        )?;
    }
    Ok(())
}

async fn broadcast_channel_bookmarks_update(
    session: &MessageContext,
    channel_id: ChannelId,
    bookmarks: Vec<db::bookmark_store::Bookmark>,
    removed_bookmark_ids: Vec<db::BookmarkId>,
) -> Result<()> {
    send_channel_bookmarks_update(
        &session.app_state.db,
        &session.peer,
        channel_id,
        bookmarks,
        removed_bookmark_ids,
    )
    .await
}

fn schedule_channel_bookmarks_reorder_broadcast(session: &MessageContext, channel_id: ChannelId) {
    let generation = {
        let mut pending = session.app_state.pending_bookmark_reorder_broadcasts.lock();
        let generation = pending.entry(channel_id).or_insert(0);
        *generation += 1;
        *generation
    };
    let app_state = session.app_state.clone();
    let peer = session.peer.clone();
    let executor = app_state.executor.clone();
    executor.clone().spawn_detached(async move {
        executor.sleep(BOOKMARK_REORDER_BROADCAST_DEBOUNCE).await;
        let should_broadcast = {
            let mut pending = app_state.pending_bookmark_reorder_broadcasts.lock();
            if pending.get(&channel_id).copied() == Some(generation) {
                pending.remove(&channel_id);
                true
            } else {
                false
            }
        };
        if !should_broadcast {
            return;
        }

        let store = BookmarkStore::new(app_state.db.clone());
        let Some(bookmarks) = store.get_bookmarks(channel_id).await.trace_err() else {
            return;
        };
        send_channel_bookmarks_update(&app_state.db, &peer, channel_id, bookmarks, Vec::new())
            .await
            .trace_err();
    });
}

async fn send_channel_bookmarks_update(
    db: &Database,
    peer: &Peer,
    channel_id: ChannelId,
    bookmarks: Vec<db::bookmark_store::Bookmark>,
    removed_bookmark_ids: Vec<db::BookmarkId>,
) -> Result<()> {
    let bookmarks = bookmarks
        .into_iter()
        .map(db::bookmark_store::Bookmark::to_proto)
        .collect::<Vec<_>>();
    let removed_bookmark_ids = removed_bookmark_ids
        .into_iter()
        .map(db::BookmarkId::to_proto)
        .collect::<Vec<_>>();
    let connection_ids = db
        .channel_chat_participant_connection_ids(channel_id)
        .await?;
    for connection_id in connection_ids {
        peer.send(
            connection_id,
            proto::UpdateChannelBookmarks {
                channel_id: channel_id.to_proto(),
                bookmarks: bookmarks.clone(),
                removed_bookmark_ids: removed_bookmark_ids.clone(),
            },
        )?;
    }
    Ok(())
}

/// Retrieve the current users notifications
async fn get_notifications(
    request: proto::GetNotifications,
    response: Response<proto::GetNotifications>,
    session: MessageContext,
) -> Result<()> {
    let notifications = session
        .db()
        .await
        .get_notifications(
            session.user_id(),
            NOTIFICATION_COUNT_PER_PAGE,
            request.before_id.map(db::NotificationId::from_proto),
        )
        .await?;
    response.send(proto::GetNotificationsResponse {
        done: notifications.len() < NOTIFICATION_COUNT_PER_PAGE,
        notifications,
    })?;
    Ok(())
}

/// Mark notifications as read
async fn mark_notification_as_read(
    request: proto::MarkNotificationRead,
    response: Response<proto::MarkNotificationRead>,
    session: MessageContext,
) -> Result<()> {
    let database = &session.db().await;
    let notifications = database
        .mark_notification_as_read_by_id(
            session.user_id(),
            NotificationId::from_proto(request.notification_id),
        )
        .await?;
    send_notifications(
        &*session.connection_pool().await,
        &session.peer,
        notifications,
    );
    response.send(proto::Ack {})?;
    Ok(())
}

fn to_axum_message(message: TungsteniteMessage) -> anyhow::Result<AxumMessage> {
    let message = match message {
        TungsteniteMessage::Text(payload) => AxumMessage::Text(payload.as_str().to_string()),
        TungsteniteMessage::Binary(payload) => AxumMessage::Binary(payload.into()),
        TungsteniteMessage::Ping(payload) => AxumMessage::Ping(payload.into()),
        TungsteniteMessage::Pong(payload) => AxumMessage::Pong(payload.into()),
        TungsteniteMessage::Close(frame) => AxumMessage::Close(frame.map(|frame| AxumCloseFrame {
            code: frame.code.into(),
            reason: frame.reason.as_str().to_owned().into(),
        })),
        // We should never receive a frame while reading the message, according
        // to the `tungstenite` maintainers:
        //
        // > It cannot occur when you read messages from the WebSocket, but it
        // > can be used when you want to send the raw frames (e.g. you want to
        // > send the frames to the WebSocket without composing the full message first).
        // >
        // > — https://github.com/snapview/tungstenite-rs/issues/268
        TungsteniteMessage::Frame(_) => {
            bail!("received an unexpected frame while reading the message")
        }
    };

    Ok(message)
}

fn to_tungstenite_message(message: AxumMessage) -> TungsteniteMessage {
    match message {
        AxumMessage::Text(payload) => TungsteniteMessage::Text(payload.into()),
        AxumMessage::Binary(payload) => TungsteniteMessage::Binary(payload.into()),
        AxumMessage::Ping(payload) => TungsteniteMessage::Ping(payload.into()),
        AxumMessage::Pong(payload) => TungsteniteMessage::Pong(payload.into()),
        AxumMessage::Close(frame) => {
            TungsteniteMessage::Close(frame.map(|frame| TungsteniteCloseFrame {
                code: frame.code.into(),
                reason: frame.reason.as_ref().into(),
            }))
        }
    }
}

fn notify_membership_updated(
    connection_pool: &mut ConnectionPool,
    result: MembershipUpdated,
    user_id: UserId,
    peer: &Peer,
) {
    for membership in &result.new_channels.channel_memberships {
        connection_pool.subscribe_to_channel(user_id, membership.channel_id, membership.role)
    }
    for channel_id in &result.removed_channels {
        connection_pool.unsubscribe_from_channel(&user_id, channel_id)
    }

    let user_channels_update = proto::UpdateUserChannels {
        channel_memberships: result
            .new_channels
            .channel_memberships
            .iter()
            .map(|cm| proto::ChannelMembership {
                channel_id: cm.channel_id.to_proto(),
                role: cm.role.into(),
            })
            .collect(),
        ..Default::default()
    };

    let mut update = build_channels_update(result.new_channels);
    update.delete_channels = result
        .removed_channels
        .into_iter()
        .map(|id| id.to_proto())
        .collect();
    update.remove_channel_invitations = vec![result.channel_id.to_proto()];

    for connection_id in connection_pool.user_connection_ids(user_id) {
        peer.send(connection_id, user_channels_update.clone())
            .trace_err();
        peer.send(connection_id, update.clone()).trace_err();
    }
}

fn build_update_user_channels(channels: &ChannelsForUser) -> proto::UpdateUserChannels {
    proto::UpdateUserChannels {
        channel_memberships: channels
            .channel_memberships
            .iter()
            .map(|m| proto::ChannelMembership {
                channel_id: m.channel_id.to_proto(),
                role: m.role.into(),
            })
            .collect(),
        observed_channel_buffer_version: channels.observed_buffer_versions.clone(),
    }
}

fn build_channels_update(channels: ChannelsForUser) -> proto::UpdateChannels {
    let mut update = proto::UpdateChannels::default();

    for channel in channels.channels {
        update.channels.push(channel.to_proto());
    }

    update.latest_channel_buffer_versions = channels.latest_buffer_versions;

    for (channel_id, participants) in channels.channel_participants {
        update
            .channel_participants
            .push(proto::ChannelParticipants {
                channel_id: channel_id.to_proto(),
                participant_user_ids: participants.into_iter().map(|id| id.to_proto()).collect(),
            });
    }

    for channel in channels.invited_channels {
        update.channel_invitations.push(channel.to_proto());
    }

    update
}

fn build_initial_contacts_update(
    contacts: Vec<db::Contact>,
    pool: &ConnectionPool,
) -> proto::UpdateContacts {
    let mut update = proto::UpdateContacts::default();

    for contact in contacts {
        match contact {
            db::Contact::Accepted { user_id, busy } => {
                update.contacts.push(contact_for_user(user_id, busy, pool));
            }
            db::Contact::Outgoing { user_id } => update.outgoing_requests.push(user_id.to_proto()),
            db::Contact::Incoming { user_id } => {
                update
                    .incoming_requests
                    .push(proto::IncomingContactRequest {
                        requester_id: user_id.to_proto(),
                    })
            }
        }
    }

    update
}

fn contact_for_user(user_id: UserId, busy: bool, pool: &ConnectionPool) -> proto::Contact {
    proto::Contact {
        user_id: user_id.to_proto(),
        online: pool.is_user_online(user_id),
        busy,
    }
}

fn room_updated(room: &proto::Room, peer: &Peer) {
    broadcast(
        None,
        room.participants
            .iter()
            .filter_map(|participant| Some(participant.peer_id?.into())),
        |peer_id| {
            peer.send(
                peer_id,
                proto::RoomUpdated {
                    room: Some(room.clone()),
                },
            )
        },
    );
}

fn channel_updated(
    channel: &db::channel::Model,
    room: &proto::Room,
    peer: &Peer,
    pool: &ConnectionPool,
) {
    let participants = room
        .participants
        .iter()
        .map(|p| p.user_id)
        .collect::<Vec<_>>();

    broadcast(
        None,
        pool.channel_connection_ids(channel.root_id())
            .filter_map(|(channel_id, role)| {
                role.can_see_channel(channel.visibility)
                    .then_some(channel_id)
            }),
        |peer_id| {
            peer.send(
                peer_id,
                proto::UpdateChannels {
                    channel_participants: vec![proto::ChannelParticipants {
                        channel_id: channel.id.to_proto(),
                        participant_user_ids: participants.clone(),
                    }],
                    ..Default::default()
                },
            )
        },
    );
}

async fn update_user_contacts(user_id: UserId, session: &Session) -> Result<()> {
    let db = session.db().await;

    let contacts = db.get_contacts(user_id).await?;
    let busy = db.is_user_busy(user_id).await?;

    let pool = session.connection_pool().await;
    let updated_contact = contact_for_user(user_id, busy, &pool);
    for contact in contacts {
        if let db::Contact::Accepted {
            user_id: contact_user_id,
            ..
        } = contact
        {
            for contact_conn_id in pool.user_connection_ids(contact_user_id) {
                session
                    .peer
                    .send(
                        contact_conn_id,
                        proto::UpdateContacts {
                            contacts: vec![updated_contact.clone()],
                            remove_contacts: Default::default(),
                            incoming_requests: Default::default(),
                            remove_incoming_requests: Default::default(),
                            outgoing_requests: Default::default(),
                            remove_outgoing_requests: Default::default(),
                        },
                    )
                    .trace_err();
            }
        }
    }
    Ok(())
}

async fn leave_room_for_session(session: &Session, connection_id: ConnectionId) -> Result<()> {
    let mut contacts_to_update = HashSet::default();

    let room_id;
    let canceled_calls_to_user_ids;
    let livekit_room;
    let delete_livekit_room;
    let room;
    let channel;

    if let Some(mut left_room) = session.db().await.leave_room(connection_id).await? {
        contacts_to_update.insert(session.user_id());

        for project in left_room.left_projects.values() {
            project_left(project, session);
        }

        room_id = RoomId::from_proto(left_room.room.id);
        canceled_calls_to_user_ids = mem::take(&mut left_room.canceled_calls_to_user_ids);
        livekit_room = mem::take(&mut left_room.room.livekit_room);
        delete_livekit_room = left_room.deleted;
        room = mem::take(&mut left_room.room);
        channel = mem::take(&mut left_room.channel);

        room_updated(&room, &session.peer);
    } else {
        return Ok(());
    }

    if let Some(channel) = channel {
        channel_updated(
            &channel,
            &room,
            &session.peer,
            &*session.connection_pool().await,
        );
    }

    {
        let pool = session.connection_pool().await;
        for canceled_user_id in canceled_calls_to_user_ids {
            for connection_id in pool.user_connection_ids(canceled_user_id) {
                session
                    .peer
                    .send(
                        connection_id,
                        proto::CallCanceled {
                            room_id: room_id.to_proto(),
                        },
                    )
                    .trace_err();
            }
            contacts_to_update.insert(canceled_user_id);
        }
    }

    for contact_user_id in contacts_to_update {
        update_user_contacts(contact_user_id, session).await?;
    }

    if let Some(live_kit) = session.app_state.livekit_client.as_ref() {
        live_kit
            .remove_participant(livekit_room.clone(), session.user_id().to_string())
            .await
            .trace_err();

        if delete_livekit_room {
            live_kit.delete_room(livekit_room).await.trace_err();
        }
    }

    Ok(())
}

async fn leave_channel_buffers_for_session(session: &Session) -> Result<()> {
    let left_channel_buffers = session
        .db()
        .await
        .leave_channel_buffers(session.connection_id)
        .await?;

    for left_buffer in left_channel_buffers {
        channel_buffer_updated(
            session.connection_id,
            left_buffer.connections,
            &proto::UpdateChannelBufferCollaborators {
                channel_id: left_buffer.channel_id.to_proto(),
                collaborators: left_buffer.collaborators,
            },
            &session.peer,
        );
    }

    Ok(())
}

fn project_left(project: &db::LeftProject, session: &Session) {
    for connection_id in &project.connection_ids {
        if project.should_unshare {
            session
                .peer
                .send(
                    *connection_id,
                    proto::UnshareProject {
                        project_id: project.id.to_proto(),
                    },
                )
                .trace_err();
        } else {
            session
                .peer
                .send(
                    *connection_id,
                    proto::RemoveProjectCollaborator {
                        project_id: project.id.to_proto(),
                        peer_id: Some(session.connection_id.into()),
                    },
                )
                .trace_err();
        }
    }
}

async fn share_agent_thread(
    request: proto::ShareAgentThread,
    response: Response<proto::ShareAgentThread>,
    session: MessageContext,
) -> Result<()> {
    let user_id = session.user_id();

    let share_id = SharedThreadId::from_proto(request.session_id.clone())
        .ok_or_else(|| anyhow!("Invalid session ID format"))?;

    session
        .db()
        .await
        .upsert_shared_thread(share_id, user_id, &request.title, request.thread_data)
        .await?;

    response.send(proto::Ack {})?;

    Ok(())
}

async fn get_shared_agent_thread(
    request: proto::GetSharedAgentThread,
    response: Response<proto::GetSharedAgentThread>,
    session: MessageContext,
) -> Result<()> {
    let share_id = SharedThreadId::from_proto(request.session_id)
        .ok_or_else(|| anyhow!("Invalid session ID format"))?;

    let result = session.db().await.get_shared_thread(share_id).await?;

    match result {
        Some((thread, username)) => {
            response.send(proto::GetSharedAgentThreadResponse {
                title: thread.title,
                thread_data: thread.data,
                sharer_username: username,
                created_at: thread.created_at.and_utc().to_rfc3339(),
            })?;
        }
        None => {
            return Err(anyhow!("Shared thread not found").into());
        }
    }

    Ok(())
}

pub trait ResultExt {
    type Ok;

    fn trace_err(self) -> Option<Self::Ok>;
}

impl<T, E> ResultExt for Result<T, E>
where
    E: std::fmt::Debug,
{
    type Ok = T;

    #[track_caller]
    fn trace_err(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::error!("{:?}", error);
                None
            }
        }
    }
}

impl From<User> for proto::User {
    fn from(user: User) -> Self {
        Self {
            id: user.id.to_proto(),
            avatar_url: user.avatar_url,
            github_login: user.github_login,
            name: user.name,
        }
    }
}
