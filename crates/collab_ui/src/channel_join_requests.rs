use anyhow::Result;
use channel::ChannelStore;
use client::{ChannelId, Subscription, User};
use gpui::{App, AppContext as _, AsyncApp, Context, Entity, EventEmitter, Global, SharedString};
use rpc::{TypedEnvelope, proto};
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub struct PendingJoinRequest {
    pub user_id: u64,
    pub user: Arc<User>,
    pub reason: Option<SharedString>,
    pub created_at: OffsetDateTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingRequestCount {
    pub channel_id: ChannelId,
    pub count: u32,
}

#[derive(Clone, Debug)]
pub enum JoinRequestEvent {
    Added {
        channel_id: ChannelId,
    },
    Responded {
        channel_id: ChannelId,
        approved: bool,
        denial_reason: Option<SharedString>,
    },
}

pub struct JoinRequestPushStore {
    channel_store: Entity<ChannelStore>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<JoinRequestEvent> for JoinRequestPushStore {}

impl JoinRequestPushStore {
    pub fn init(cx: &mut App) {
        let channel_store = ChannelStore::global(cx);
        let store = cx.new(|cx| Self::new(channel_store, cx));
        cx.set_global(GlobalJoinRequestPushStore(store));
    }

    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalJoinRequestPushStore>().0.clone()
    }

    pub fn new(channel_store: Entity<ChannelStore>, cx: &mut Context<Self>) -> Self {
        let client = channel_store.read(cx).client();
        let subscriptions = vec![
            client.add_message_handler(cx.weak_entity(), Self::handle_join_request_added),
            client.add_message_handler(cx.weak_entity(), Self::handle_join_request_responded),
        ];

        Self {
            channel_store,
            _subscriptions: subscriptions,
        }
    }

    async fn handle_join_request_added(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::JoinRequestAdded>,
        mut cx: AsyncApp,
    ) -> Result<()> {
        let channel_id = ChannelId(envelope.payload.channel_id);
        let is_channel_admin = this.read_with(&cx, |this, cx| {
            this.channel_store.read(cx).is_channel_admin(channel_id)
        });

        if is_channel_admin {
            this.update(&mut cx, |_this, cx| {
                cx.emit(JoinRequestEvent::Added { channel_id });
            });
        }

        Ok(())
    }

    async fn handle_join_request_responded(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::JoinRequestResponded>,
        mut cx: AsyncApp,
    ) -> Result<()> {
        this.update(&mut cx, |_, cx| {
            cx.emit(JoinRequestEvent::Responded {
                channel_id: ChannelId(envelope.payload.channel_id),
                approved: envelope.payload.approved,
                denial_reason: envelope.payload.denial_reason.map(Into::into),
            });
        });

        Ok(())
    }
}

struct GlobalJoinRequestPushStore(Entity<JoinRequestPushStore>);

impl Global for GlobalJoinRequestPushStore {}
