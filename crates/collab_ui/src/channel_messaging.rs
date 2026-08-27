use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context as _, Result, anyhow};
use channel::Channel;
use chrono::{DateTime, Utc};
use client::{Client, Status, Subscription, User};
use collaboration_domain::{
    AggregateId, CommunityId, OperationId, channel_id_for_legacy_channel,
    community_id_for_legacy_root_channel,
};
use editor::Editor;
use futures::StreamExt as _;
use gpui::{
    App, AppContext as _, Context, Entity, Focusable, Global, Render, Task, WeakEntity, Window,
};
use menu::Confirm;
use multi_buffer::MultiBufferOffset;
use nostr_compat::SignedEvent;
use project::Project;
use rpc::TypedEnvelope;
use rpc::proto;
use ui::{ButtonStyle, Color, IconButton, IconName, Label, LabelSize, prelude::*};
use util::ResultExt as _;
use workspace::collaborative_composer::{
    CollaborativeComposerActionError, CollaborativeComposerProvider,
};
use workspace::{Workspace, collaborative_navigation::CollaborativeNavigationTarget};
use zed_credentials_provider::channel_signing::{
    ChannelSigningIdentity, load_or_create_channel_signing_identity,
};

use crate::message_timeline::{
    MessageTimeline, MessageTimelineAuthor, MessageTimelineAuthorKind, MessageTimelineContext,
    MessageTimelineEntry, MessageTimelinePage, MessageTimelineReaction, OptimisticMessage,
};

const CONTRACT_VERSION: u32 = 1;
const PAGE_SIZE: u32 = 100;
const MESSAGE_KIND: u16 = 40_002;

struct GlobalChannelMessaging(Entity<ChannelMessagingTransport>);

impl Global for GlobalChannelMessaging {}

#[derive(Clone)]
pub struct ChannelMessagingViews {
    pub timeline: Entity<MessageTimeline>,
    pub composer: CollaborativeComposerProvider,
}

#[derive(Clone)]
struct ActiveChannel {
    generation: u64,
    community_id: CommunityId,
    channel_id: AggregateId,
    user: Arc<User>,
    timeline: Entity<MessageTimeline>,
    composer: Entity<ChannelMessageComposer>,
    signing_identity: Option<Arc<ChannelSigningIdentity>>,
    principal_id: Option<Vec<u8>>,
    next_cursor: Option<proto::CollaborativeMessageCursor>,
    next_cursor_token: Option<String>,
    authoritative_outbox_cursor: u64,
    latest_target: Option<MessageTarget>,
    latest_own_target: Option<MessageTarget>,
    reply_target: Option<MessageTarget>,
}

#[derive(Clone)]
struct PendingOperation {
    request: proto::ApplyCollaborativeMessageOperation,
    optimistic: bool,
}

#[derive(Clone)]
struct MessageTarget {
    message_id: Vec<u8>,
    source_event_id: Vec<u8>,
    version: u64,
    reaction_version: u64,
}

pub struct ChannelMessagingTransport {
    client: Arc<Client>,
    active: Option<ActiveChannel>,
    pending: BTreeMap<String, PendingOperation>,
    active_submit: Option<(String, Task<()>)>,
    status: String,
    _message_subscription: Subscription,
    _connection_task: Task<()>,
}

impl ChannelMessagingTransport {
    pub fn open(
        channel: Arc<Channel>,
        user: Arc<User>,
        client: Arc<Client>,
        project: Entity<Project>,
        workspace: Entity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<ChannelMessagingViews> {
        let transport = if let Some(global) = cx.try_global::<GlobalChannelMessaging>() {
            global.0.clone()
        } else {
            let transport = cx.new(|cx| {
                let message_subscription =
                    client.add_message_handler(cx.weak_entity(), Self::handle_stream_update);
                let mut connection_status = client.status();
                let connection_task = cx.spawn(async move |this, cx| {
                    while let Some(status) = connection_status.next().await {
                        let Some(this) = this.upgrade() else {
                            break;
                        };
                        match status {
                            Status::Connected { .. } => {
                                this.update(cx, |this, cx| this.reconnect(cx));
                            }
                            Status::ConnectionLost | Status::Reconnecting => {
                                this.update(cx, |this, cx| {
                                    this.set_status("Offline — reconnecting from server cursor", cx)
                                });
                            }
                            Status::SignedOut | Status::UpgradeRequired => {
                                this.update(cx, |this, cx| {
                                    this.set_status("Channel messaging requires authentication", cx)
                                });
                            }
                            _ => {}
                        }
                    }
                });
                Self {
                    client: client.clone(),
                    active: None,
                    pending: BTreeMap::new(),
                    active_submit: None,
                    status: String::new(),
                    _message_subscription: message_subscription,
                    _connection_task: connection_task,
                }
            });
            cx.set_global(GlobalChannelMessaging(transport.clone()));
            transport
        };

        if !Arc::ptr_eq(&transport.read(cx).client, &client) {
            return Err(anyhow!(
                "collaboration client changed while the workspace is active"
            ));
        }

        let timeline = cx.new(MessageTimeline::new);
        let editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Message this channel…", window, cx);
            editor
        });
        let composer = cx.new(|cx| ChannelMessageComposer::new(editor, transport.downgrade(), cx));
        let generation = transport
            .read(cx)
            .active
            .as_ref()
            .map_or(1, |active| active.generation.saturating_add(1));
        let community_id = community_id_for_legacy_root_channel(channel.root_id().0);
        let channel_id = channel_id_for_legacy_channel(channel.id.0);
        transport.update(cx, |this, cx| {
            this.close_active(cx);
            this.pending.clear();
            this.active_submit = None;
            this.active = Some(ActiveChannel {
                generation,
                community_id,
                channel_id,
                user,
                timeline: timeline.clone(),
                composer: composer.clone(),
                signing_identity: None,
                principal_id: None,
                next_cursor: None,
                next_cursor_token: None,
                authoritative_outbox_cursor: 0,
                latest_target: None,
                latest_own_target: None,
                reply_target: None,
            });
            this.set_status("Connecting to channel…", cx);
            this.connect(generation, cx);
            cx.observe(&workspace, move |this, workspace, cx| {
                let selected_channel_id =
                    match workspace.read(cx).collaborative_navigation().current() {
                        Some(CollaborativeNavigationTarget::Channel { channel_id }) => {
                            channel_id.parse::<u64>().ok()
                        }
                        _ => None,
                    };
                if selected_channel_id != Some(channel.id.0)
                    && this
                        .active
                        .as_ref()
                        .is_some_and(|active| active.generation == generation)
                {
                    this.close_active(cx);
                    this.pending.clear();
                    this.active_submit = None;
                }
            })
            .detach();
            cx.observe_release(&workspace, move |this, _, cx| {
                if this
                    .active
                    .as_ref()
                    .is_some_and(|active| active.generation == generation)
                {
                    this.close_active(cx);
                    this.pending.clear();
                    this.active_submit = None;
                }
            })
            .detach();
        });

        let focus_handle = composer.read(cx).editor.read(cx).focus_handle(cx);
        let submit_transport = transport.downgrade();
        let cancel_transport = transport.downgrade();
        let provider = CollaborativeComposerProvider::new(
            project,
            composer.into(),
            move |cx| {
                submit_transport
                    .update(cx, |transport, cx| transport.submit(cx))
                    .map_err(|_| CollaborativeComposerActionError::ThreadUnavailable)?
            },
            move |cx| {
                cancel_transport
                    .update(cx, |transport, cx| transport.cancel(cx))
                    .map_err(|_| CollaborativeComposerActionError::ThreadUnavailable)?
            },
        )
        .with_focus_handle(focus_handle);
        Ok(ChannelMessagingViews {
            timeline,
            composer: provider,
        })
    }

    fn connect(&mut self, generation: u64, cx: &mut Context<Self>) {
        let Some(active) = self
            .active
            .as_ref()
            .filter(|active| active.generation == generation)
        else {
            return;
        };
        let provider = zed_credentials_provider::global(cx);
        let client = self.client.clone();
        let community_id = active.community_id;
        let channel_id = active.channel_id;
        let account_id = active.user.legacy_id;
        let username = active.user.username.to_string();
        let after_outbox_sequence = active.authoritative_outbox_cursor;
        cx.spawn(async move |this, cx| {
            let identity = Arc::new(
                load_or_create_channel_signing_identity(
                    provider.as_ref(),
                    community_id,
                    account_id,
                    cx,
                )
                .await
                .context("load protected channel signing identity")?,
            );
            let response = client
                .request(proto::OpenCollaborativeChannel {
                    contract_version: CONTRACT_VERSION,
                    community_id: community_id.as_uuid().as_bytes().to_vec(),
                    channel_id: channel_id.as_uuid().as_bytes().to_vec(),
                    page_size: PAGE_SIZE,
                    after_outbox_sequence,
                    signing_public_key: identity.public_key().to_vec(),
                })
                .await
                .context("open collaborative channel")?;
            this.update(cx, |this, cx| {
                this.apply_open_response(generation, identity, response, cx)
            })??;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
        self.set_status(&format!("Connecting as {username}…"), cx);
    }

    fn apply_open_response(
        &mut self,
        generation: u64,
        identity: Arc<ChannelSigningIdentity>,
        response: proto::OpenCollaborativeChannelResponse,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        ensure_success(response.error_code)?;
        let Some(active) = self
            .active
            .as_mut()
            .filter(|active| active.generation == generation)
        else {
            return Ok(());
        };
        active.signing_identity = Some(identity);
        active.principal_id = Some(response.principal_id);
        if let Some(page) = response.page {
            apply_page(active, None, page, cx)?;
        }
        self.set_status("Connected — history is server-backed", cx);
        Ok(())
    }

    fn reconnect(&mut self, cx: &mut Context<Self>) {
        if let Some(generation) = self.active.as_ref().map(|active| active.generation) {
            self.set_status("Reconnected — replaying missed channel activity", cx);
            self.connect(generation, cx);
        }
    }

    fn submit(
        &mut self,
        cx: &mut Context<Self>,
    ) -> std::result::Result<(), CollaborativeComposerActionError> {
        let active = self
            .active
            .as_ref()
            .ok_or(CollaborativeComposerActionError::ThreadUnavailable)?;
        if self.active_submit.is_some() {
            return Err(CollaborativeComposerActionError::ProviderFailure(
                "a channel message is already being submitted".into(),
            ));
        }
        let body = active.composer.read(cx).editor.read(cx).text(cx);
        let body = body.trim().to_owned();
        if body.is_empty() {
            return Err(CollaborativeComposerActionError::EmptyInput);
        }
        let identity = active.signing_identity.clone().ok_or_else(|| {
            CollaborativeComposerActionError::ProviderFailure(
                "channel signing identity is still loading".into(),
            )
        })?;
        let operation_id = OperationId::new();
        let message_id = AggregateId::new();
        let operation_id_string = operation_id.to_string();
        let reply_target = active.reply_target.clone();
        let mut tags = vec![vec!["h".into(), active.channel_id.to_string()]];
        if let Some(reply_target) = &reply_target {
            tags.push(vec![
                "e".into(),
                hex::encode(&reply_target.source_event_id),
                String::new(),
                "reply".into(),
            ]);
        }
        let signed_event = identity
            .sign(unix_time_seconds(), MESSAGE_KIND, tags, body.clone())
            .map_err(|error| {
                CollaborativeComposerActionError::ProviderFailure(error.to_string())
            })?;
        active
            .timeline
            .update(cx, |timeline, cx| {
                timeline.begin_optimistic(
                    OptimisticMessage {
                        operation_id: operation_id_string.clone(),
                        author: MessageTimelineAuthor {
                            kind: MessageTimelineAuthorKind::Human,
                            id: active.user.legacy_id.to_string(),
                            label: active.user.username.to_string(),
                        },
                        content: body.clone(),
                        reply_to: reply_target
                            .as_ref()
                            .map(|target| hex::encode(&target.source_event_id)),
                        occurred_at: Utc::now(),
                        context: timeline_context(active),
                    },
                    cx,
                )
            })
            .map_err(|error| {
                CollaborativeComposerActionError::ProviderFailure(error.to_string())
            })?;
        let request = proto::ApplyCollaborativeMessageOperation {
            contract_version: CONTRACT_VERSION,
            community_id: active.community_id.as_uuid().as_bytes().to_vec(),
            channel_id: active.channel_id.as_uuid().as_bytes().to_vec(),
            message_id: message_id.as_uuid().as_bytes().to_vec(),
            operation_id: operation_id_string.clone(),
            kind: proto::CollaborativeMessageOperationKind::CollaborativeMessageCreate.into(),
            expected_version: 0,
            body,
            reply_to_event_id: reply_target.map_or_else(Vec::new, |target| target.source_event_id),
            reaction: String::new(),
            related_reaction_event_id: Vec::new(),
            signed_event: Some(signed_event_to_proto(signed_event)),
            acknowledged_outbox_sequence: 0,
        };
        self.pending.insert(
            operation_id_string.clone(),
            PendingOperation {
                request: request.clone(),
                optimistic: true,
            },
        );
        self.start_operation(operation_id_string, request, cx);
        Ok(())
    }

    fn start_operation(
        &mut self,
        operation_id: String,
        request: proto::ApplyCollaborativeMessageOperation,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        let task_operation_id = operation_id.clone();
        let task = cx.spawn(async move |this, cx| {
            let response = client.request(request).await;
            this.update(cx, |this, cx| {
                this.finish_operation(&task_operation_id, response, cx)
            })
            .log_err();
        });
        self.active_submit = Some((operation_id, task));
        self.set_status("Sending channel message…", cx);
    }

    fn finish_operation(
        &mut self,
        operation_id: &str,
        response: Result<proto::ApplyCollaborativeMessageOperationResponse>,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        self.active_submit = None;
        let (timeline, composer) = self
            .active
            .as_ref()
            .map(|active| (active.timeline.clone(), active.composer.clone()))
            .context("channel is no longer active")?;
        let optimistic = self
            .pending
            .get(operation_id)
            .is_some_and(|operation| operation.optimistic);
        match response {
            Ok(response) if response.accepted => {
                let record = response
                    .message
                    .context("accepted channel operation omitted its message")?;
                let entry = record_to_entry(&record)?;
                if optimistic {
                    timeline.update(cx, |timeline, cx| {
                        timeline.accept_optimistic(operation_id, entry, cx)
                    })?;
                    composer.update(cx, |composer, cx| composer.clear(cx));
                    if let Some(active) = self.active.as_mut() {
                        active.reply_target = None;
                    }
                } else {
                    timeline.update(cx, |timeline, cx| timeline.upsert_live(entry, cx))?;
                }
                self.pending.remove(operation_id);
                self.set_status("Channel operation delivered", cx);
            }
            Ok(response) => {
                let reason = error_label(response.error_code).to_owned();
                if optimistic {
                    timeline.update(cx, |timeline, cx| {
                        timeline.reject_optimistic(operation_id, reason.clone(), cx)
                    })?;
                }
                self.set_status(&format!("{reason} — retry is available"), cx);
            }
            Err(error) => {
                if optimistic {
                    timeline.update(cx, |timeline, cx| {
                        timeline.reject_optimistic(
                            operation_id,
                            "Offline — message retained for retry",
                            cx,
                        )
                    })?;
                }
                self.set_status(
                    &format!("Offline — retrying requires confirmation: {error}"),
                    cx,
                );
            }
        }
        Ok(())
    }

    fn retry(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if self.active_submit.is_some() {
            return Ok(());
        }
        let (operation_id, pending) = self
            .pending
            .iter()
            .next_back()
            .map(|(operation_id, pending)| (operation_id.clone(), pending.clone()))
            .context("no failed channel message is available to retry")?;
        if pending.optimistic
            && let Some(active) = &self.active
        {
            active.timeline.update(cx, |timeline, cx| {
                timeline.retry_optimistic(&operation_id, cx)
            })?;
        }
        self.start_operation(operation_id, pending.request, cx);
        Ok(())
    }

    fn edit_latest(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let active = self
            .active
            .as_ref()
            .context("channel is no longer active")?;
        let target = active
            .latest_own_target
            .clone()
            .context("no authored message is available to edit")?;
        let body = active.composer.read(cx).editor.read(cx).text(cx);
        let body = body.trim().to_owned();
        if body.is_empty() {
            return Err(anyhow!(
                "type replacement text before editing the latest message"
            ));
        }
        self.start_target_operation(
            target,
            proto::CollaborativeMessageOperationKind::CollaborativeMessageEdit,
            40_003,
            body,
            String::new(),
            cx,
        )
    }

    fn delete_latest(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let target = self
            .active
            .as_ref()
            .and_then(|active| active.latest_own_target.clone())
            .context("no authored message is available to delete")?;
        self.start_target_operation(
            target,
            proto::CollaborativeMessageOperationKind::CollaborativeMessageDelete,
            5,
            String::new(),
            String::new(),
            cx,
        )
    }

    fn react_to_latest(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let target = self
            .active
            .as_ref()
            .and_then(|active| active.latest_target.clone())
            .context("no message is available to react to")?;
        self.start_target_operation(
            target,
            proto::CollaborativeMessageOperationKind::CollaborativeMessageReactionAdd,
            7,
            "👍".into(),
            "👍".into(),
            cx,
        )
    }

    fn toggle_reply_to_latest(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let active = self
            .active
            .as_mut()
            .context("channel is no longer active")?;
        if active.reply_target.is_some() {
            active.reply_target = None;
            self.set_status("Reply target cleared", cx);
        } else {
            active.reply_target = Some(
                active
                    .latest_target
                    .clone()
                    .context("no message is available to reply to")?,
            );
            self.set_status("Replying to the latest message", cx);
        }
        Ok(())
    }

    fn start_target_operation(
        &mut self,
        target: MessageTarget,
        kind: proto::CollaborativeMessageOperationKind,
        event_kind: u16,
        body: String,
        reaction: String,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if self.active_submit.is_some() {
            return Err(anyhow!("a channel operation is already in progress"));
        }
        let active = self
            .active
            .as_ref()
            .context("channel is no longer active")?;
        let identity = active
            .signing_identity
            .clone()
            .context("channel signing identity is still loading")?;
        let target_event_id = hex::encode(&target.source_event_id);
        let signed_event = identity.sign(
            unix_time_seconds(),
            event_kind,
            vec![vec!["e".into(), target_event_id]],
            body.clone(),
        )?;
        let operation_id = OperationId::new().to_string();
        let expected_version =
            if kind == proto::CollaborativeMessageOperationKind::CollaborativeMessageReactionAdd {
                target.reaction_version
            } else {
                target.version
            };
        let request = proto::ApplyCollaborativeMessageOperation {
            contract_version: CONTRACT_VERSION,
            community_id: active.community_id.as_uuid().as_bytes().to_vec(),
            channel_id: active.channel_id.as_uuid().as_bytes().to_vec(),
            message_id: target.message_id,
            operation_id: operation_id.clone(),
            kind: kind.into(),
            expected_version,
            body,
            reply_to_event_id: Vec::new(),
            reaction,
            related_reaction_event_id: Vec::new(),
            signed_event: Some(signed_event_to_proto(signed_event)),
            acknowledged_outbox_sequence: 0,
        };
        self.pending.insert(
            operation_id.clone(),
            PendingOperation {
                request: request.clone(),
                optimistic: false,
            },
        );
        self.start_operation(operation_id, request, cx);
        Ok(())
    }

    fn acknowledge(&self, sequence: u64, cx: &mut Context<Self>) {
        let Some(active) = self.active.as_ref() else {
            return;
        };
        let client = self.client.clone();
        let request = proto::ApplyCollaborativeMessageOperation {
            contract_version: CONTRACT_VERSION,
            community_id: active.community_id.as_uuid().as_bytes().to_vec(),
            channel_id: active.channel_id.as_uuid().as_bytes().to_vec(),
            message_id: AggregateId::new().as_uuid().as_bytes().to_vec(),
            operation_id: OperationId::new().to_string(),
            kind: proto::CollaborativeMessageOperationKind::CollaborativeMessageAcknowledge.into(),
            expected_version: 0,
            body: String::new(),
            reply_to_event_id: Vec::new(),
            reaction: String::new(),
            related_reaction_event_id: Vec::new(),
            signed_event: None,
            acknowledged_outbox_sequence: sequence,
        };
        cx.spawn(async move |_, _| {
            let response = client.request(request).await?;
            ensure_success(response.error_code)
        })
        .detach_and_log_err(cx);
    }

    fn cancel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> std::result::Result<(), CollaborativeComposerActionError> {
        if let Some((operation_id, task)) = self.active_submit.take() {
            drop(task);
            if let Some(active) = &self.active {
                active
                    .timeline
                    .update(cx, |timeline, cx| {
                        timeline.reject_optimistic(
                            &operation_id,
                            "Cancelled locally; a committed server result will still reconcile",
                            cx,
                        )
                    })
                    .map_err(|error| {
                        CollaborativeComposerActionError::ProviderFailure(error.to_string())
                    })?;
            }
            self.set_status("Message submission cancelled", cx);
            return Ok(());
        }
        if let Some(active) = &self.active {
            active
                .composer
                .update(cx, |composer, cx| composer.clear(cx));
            self.set_status("Draft cleared", cx);
            return Ok(());
        }
        Err(CollaborativeComposerActionError::ThreadUnavailable)
    }

    fn load_more(&mut self, cx: &mut Context<Self>) {
        let Some(active) = self.active.clone() else {
            return;
        };
        let Some(cursor) = active.next_cursor.clone() else {
            self.set_status("All channel history is loaded", cx);
            return;
        };
        let request_cursor_token = active.next_cursor_token.clone();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let response = client
                .request(proto::GetCollaborativeMessageWindow {
                    contract_version: CONTRACT_VERSION,
                    community_id: active.community_id.as_uuid().as_bytes().to_vec(),
                    channel_id: active.channel_id.as_uuid().as_bytes().to_vec(),
                    thread_root_event_id: Vec::new(),
                    page_size: PAGE_SIZE,
                    cursor: Some(cursor),
                })
                .await?;
            ensure_success(response.error_code)?;
            let page = response
                .page
                .context("channel history response omitted its page")?;
            this.update(cx, |this, cx| {
                let current = this
                    .active
                    .as_mut()
                    .filter(|current| current.generation == active.generation)
                    .context("channel changed while loading history")?;
                apply_page(current, request_cursor_token, page, cx)?;
                this.set_status("Loaded earlier channel messages", cx);
                anyhow::Ok(())
            })??;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
        self.set_status("Loading earlier channel messages…", cx);
    }

    async fn handle_stream_update(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::CollaborativeMessageStreamUpdate>,
        mut cx: gpui::AsyncApp,
    ) -> Result<()> {
        this.update(&mut cx, |this, cx| {
            this.apply_stream_update(envelope.payload, cx)
        })?;
        Ok(())
    }

    fn apply_stream_update(
        &mut self,
        update: proto::CollaborativeMessageStreamUpdate,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if update.contract_version != CONTRACT_VERSION {
            return Ok(());
        }
        let Some(active) = self.active.as_mut() else {
            return Ok(());
        };
        if update.community_id != active.community_id.as_uuid().as_bytes()
            || update.channel_id != active.channel_id.as_uuid().as_bytes()
        {
            return Ok(());
        }
        if update.outbox_sequence <= active.authoritative_outbox_cursor {
            return Ok(());
        }
        if update.operation_kind
            == proto::CollaborativeMessageOperationKind::CollaborativeMessageAcknowledge as i32
        {
            active.authoritative_outbox_cursor = update.outbox_sequence;
            self.set_status("Read state synchronized", cx);
            return Ok(());
        }
        let record = update
            .message
            .context("channel update omitted its message")?;
        let operation_id = (!record.accepted_operation_id.is_empty())
            .then(|| record.accepted_operation_id.clone());
        let entry = record_to_entry(&record)?;
        remember_record(active, &record, true);
        active
            .timeline
            .update(cx, |timeline, cx| timeline.upsert_live(entry, cx))?;
        active.authoritative_outbox_cursor = update.outbox_sequence;
        if let Some(operation_id) = operation_id {
            if self
                .pending
                .get(&operation_id)
                .is_some_and(|operation| operation.optimistic)
            {
                active
                    .composer
                    .update(cx, |composer, cx| composer.clear(cx));
                active.reply_target = None;
            }
        }
        self.acknowledge(update.outbox_sequence, cx);
        self.set_status("Channel updated in real time", cx);
        Ok(())
    }

    fn close_active(&mut self, cx: &mut Context<Self>) {
        let Some(active) = self.active.take() else {
            return;
        };
        let client = self.client.clone();
        cx.spawn(async move |_, _| {
            client
                .request(proto::CloseCollaborativeChannel {
                    contract_version: CONTRACT_VERSION,
                    community_id: active.community_id.as_uuid().as_bytes().to_vec(),
                    channel_id: active.channel_id.as_uuid().as_bytes().to_vec(),
                })
                .await?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn set_status(&mut self, status: &str, cx: &mut Context<Self>) {
        self.status.clear();
        self.status.push_str(status);
        if let Some(active) = &self.active {
            active.composer.update(cx, |composer, cx| {
                composer.status.clear();
                composer.status.push_str(status);
                cx.notify();
            });
        }
        cx.notify();
    }
}

pub struct ChannelMessageComposer {
    editor: Entity<Editor>,
    transport: WeakEntity<ChannelMessagingTransport>,
    status: String,
}

impl ChannelMessageComposer {
    fn new(
        editor: Entity<Editor>,
        transport: WeakEntity<ChannelMessagingTransport>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            editor,
            transport,
            status: "Connecting to channel…".into(),
        }
    }

    fn clear(&self, cx: &mut App) {
        let buffer = self.editor.read(cx).buffer().clone();
        buffer.update(cx, |buffer, cx| {
            let length = buffer.len(cx);
            buffer.edit([(MultiBufferOffset(0)..length, "")], None, cx);
        });
    }
}

impl Render for ChannelMessageComposer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let submit_transport = self.transport.clone();
        let cancel_transport = self.transport.clone();
        let retry_transport = self.transport.clone();
        let page_transport = self.transport.clone();
        let edit_transport = self.transport.clone();
        let delete_transport = self.transport.clone();
        let reaction_transport = self.transport.clone();
        let reply_transport = self.transport.clone();
        v_flex()
            .id("collaborative-channel-composer")
            .w_full()
            .gap_1()
            .p_2()
            .border_1()
            .border_color(cx.theme().colors().border)
            .on_action(move |_: &Confirm, _window, cx| {
                submit_transport
                    .update(cx, |transport, cx| transport.submit(cx))
                    .log_err();
            })
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .child(div().flex_1().child(self.editor.clone()))
                    .child(
                        IconButton::new("cancel-channel-message", IconName::Stop)
                            .style(ButtonStyle::Subtle)
                            .tooltip(ui::Tooltip::text("Cancel send or clear draft"))
                            .on_click(move |_, _, cx| {
                                cancel_transport
                                    .update(cx, |transport, cx| transport.cancel(cx))
                                    .log_err();
                            }),
                    )
                    .child(
                        IconButton::new(
                            "reply-to-latest-channel-message",
                            IconName::ReplyArrowRight,
                        )
                        .style(ButtonStyle::Subtle)
                        .tooltip(ui::Tooltip::text("Reply to the latest message"))
                        .on_click(move |_, _, cx| {
                            reply_transport
                                .update(cx, |transport, cx| transport.toggle_reply_to_latest(cx))
                                .log_err();
                        }),
                    )
                    .child(
                        IconButton::new("edit-latest-channel-message", IconName::Pencil)
                            .style(ButtonStyle::Subtle)
                            .tooltip(ui::Tooltip::text(
                                "Replace your latest message with the composer text",
                            ))
                            .on_click(move |_, _, cx| {
                                edit_transport
                                    .update(cx, |transport, cx| transport.edit_latest(cx))
                                    .log_err();
                            }),
                    )
                    .child(
                        IconButton::new("react-latest-channel-message", IconName::ThumbsUp)
                            .style(ButtonStyle::Subtle)
                            .tooltip(ui::Tooltip::text("React to the latest message"))
                            .on_click(move |_, _, cx| {
                                reaction_transport
                                    .update(cx, |transport, cx| transport.react_to_latest(cx))
                                    .log_err();
                            }),
                    )
                    .child(
                        IconButton::new("delete-latest-channel-message", IconName::Trash)
                            .style(ButtonStyle::Subtle)
                            .tooltip(ui::Tooltip::text("Delete your latest message"))
                            .on_click(move |_, _, cx| {
                                delete_transport
                                    .update(cx, |transport, cx| transport.delete_latest(cx))
                                    .log_err();
                            }),
                    )
                    .child(
                        IconButton::new("send-channel-message", IconName::Send)
                            .style(ButtonStyle::Filled)
                            .icon_color(Color::Accent)
                            .tooltip(ui::Tooltip::text("Send channel message"))
                            .on_click(move |_, _, cx| {
                                retry_transport
                                    .update(cx, |transport, cx| {
                                        if transport.pending.is_empty() {
                                            transport.submit(cx).map_err(anyhow::Error::msg)
                                        } else {
                                            transport.retry(cx)
                                        }
                                    })
                                    .log_err();
                            }),
                    )
                    .child(
                        IconButton::new("load-earlier-channel-messages", IconName::ArrowUp)
                            .style(ButtonStyle::Subtle)
                            .tooltip(ui::Tooltip::text("Load earlier messages"))
                            .on_click(move |_, _, cx| {
                                page_transport
                                    .update(cx, |transport, cx| transport.load_more(cx))
                                    .log_err();
                            }),
                    ),
            )
            .child(
                Label::new(self.status.clone())
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
    }
}

fn apply_page(
    active: &mut ActiveChannel,
    request_cursor: Option<String>,
    page: proto::CollaborativeMessagePage,
    cx: &mut App,
) -> Result<()> {
    let next_cursor_token = page
        .next_cursor
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let entries = page
        .messages
        .iter()
        .map(record_to_entry)
        .collect::<Result<Vec<_>>>()?;
    for (index, record) in page.messages.iter().enumerate() {
        remember_record(active, record, index == 0 && active.latest_target.is_none());
    }
    active.timeline.update(cx, |timeline, cx| {
        timeline.apply_history_page(
            MessageTimelinePage {
                request_cursor,
                next_cursor: next_cursor_token.clone(),
                entries,
            },
            cx,
        )
    })?;
    active.next_cursor = page.next_cursor;
    active.next_cursor_token = next_cursor_token;
    active.authoritative_outbox_cursor = active
        .authoritative_outbox_cursor
        .max(page.authoritative_outbox_cursor);
    Ok(())
}

fn remember_record(
    active: &mut ActiveChannel,
    record: &proto::CollaborativeMessageRecord,
    prefer_as_latest: bool,
) {
    let target = MessageTarget {
        message_id: record.message_id.clone(),
        source_event_id: record.source_event_id.clone(),
        version: record.version,
        reaction_version: record.reaction_version,
    };
    if prefer_as_latest {
        active.latest_target = Some(target.clone());
    }
    if active
        .principal_id
        .as_ref()
        .is_some_and(|principal_id| principal_id == &record.author_principal_id)
    {
        if prefer_as_latest || active.latest_own_target.is_none() {
            active.latest_own_target = Some(target);
        }
    }
}

fn record_to_entry(record: &proto::CollaborativeMessageRecord) -> Result<MessageTimelineEntry> {
    let occurred_at = DateTime::<Utc>::from_timestamp(record.created_at as i64, 0)
        .context("channel message timestamp is outside the supported range")?;
    let mut reactions = BTreeMap::<String, u32>::new();
    for reaction in &record.reactions {
        *reactions.entry(reaction.value.clone()).or_default() += 1;
    }
    Ok(MessageTimelineEntry {
        event_id: hex::encode(&record.source_event_id),
        operation_id: (!record.accepted_operation_id.is_empty())
            .then(|| record.accepted_operation_id.clone()),
        source_version: record.outbox_sequence.max(1),
        author: MessageTimelineAuthor {
            kind: MessageTimelineAuthorKind::Human,
            id: hex::encode(&record.author_principal_id),
            label: record.author_display_name.clone(),
        },
        content: record.body.clone(),
        reply_to: (!record.reply_to_event_id.is_empty())
            .then(|| hex::encode(&record.reply_to_event_id)),
        edited: record.edited,
        deleted: record.deleted,
        reactions: reactions
            .into_iter()
            .map(|(value, count)| MessageTimelineReaction { value, count })
            .collect(),
        occurred_at,
        projected_at: occurred_at,
        context: MessageTimelineContext {
            community_id: Some(hex::encode(&record.community_id)),
            project_id: Some(hex::encode(&record.channel_id)),
            thread_id: (!record.reply_to_event_id.is_empty())
                .then(|| hex::encode(&record.reply_to_event_id)),
        },
    })
}

fn timeline_context(active: &ActiveChannel) -> MessageTimelineContext {
    MessageTimelineContext {
        community_id: Some(active.community_id.to_string()),
        project_id: Some(active.channel_id.to_string()),
        thread_id: None,
    }
}

fn signed_event_to_proto(signed: SignedEvent) -> proto::CollaborativeSignedEvent {
    proto::CollaborativeSignedEvent {
        claimed_event_id: signed.claimed_id.as_bytes().to_vec(),
        public_key: signed.event.public_key.as_bytes().to_vec(),
        created_at: signed.event.created_at,
        kind: u32::from(signed.event.kind),
        tags: signed
            .event
            .tags
            .into_iter()
            .map(|values| proto::CollaborativeEventTag { values })
            .collect(),
        content: signed.event.content,
        signature: signed.signature.as_bytes().to_vec(),
    }
}

fn ensure_success(error_code: i32) -> Result<()> {
    if error_code == proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorNone as i32 {
        Ok(())
    } else {
        Err(anyhow!(error_label(error_code)))
    }
}

fn error_label(error_code: i32) -> &'static str {
    match error_code {
        value
            if value
                == proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorNone as i32 =>
        {
            "Ready"
        }
        value
            if value
                == proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorDenied as i32 =>
        {
            "Access to this channel was denied"
        }
        value
            if value
                == proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorOffline
                    as i32 =>
        {
            "Offline"
        }
        value
            if value
                == proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorRetrying
                    as i32 =>
        {
            "The server committed the operation and is retrying delivery"
        }
        value
            if value
                == proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorStaleVersion
                    as i32 =>
        {
            "The message changed on another client"
        }
        value
            if value
                == proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorUnavailable
                    as i32 =>
        {
            "Channel messaging is temporarily unavailable"
        }
        value
            if value
                == proto::CollaborativeMessageErrorCode::CollaborativeMessageErrorInvalidRequest
                    as i32 =>
        {
            "The server rejected the message operation"
        }
        _ => "The server does not support this messaging contract",
    }
}

fn unix_time_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
