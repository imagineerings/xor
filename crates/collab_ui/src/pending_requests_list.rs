use crate::channel_join_requests::PendingJoinRequest;
use anyhow::Result;
use channel::ChannelStore;
use client::{ChannelId, Subscription, UserStore};
use gpui::{AsyncApp, Context, Entity, EventEmitter, Render, Window, prelude::*};
use rpc::{TypedEnvelope, proto};
use time::OffsetDateTime;
use ui::{Label, LabelSize, prelude::*};

pub struct PendingRequestsList {
    channel_id: ChannelId,
    requests: Vec<PendingJoinRequest>,
    loading: bool,
    channel_store: Entity<ChannelStore>,
    user_store: Entity<UserStore>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone, Debug)]
pub enum PendingRequestsListEvent {
    RequestSelected(PendingJoinRequest),
}

impl EventEmitter<PendingRequestsListEvent> for PendingRequestsList {}

impl PendingRequestsList {
    pub fn new(
        channel_id: ChannelId,
        channel_store: Entity<ChannelStore>,
        user_store: Entity<UserStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        let client = channel_store.read(cx).client();
        let subscriptions =
            vec![client.add_message_handler(cx.weak_entity(), Self::handle_join_request_added)];
        let mut this = Self {
            channel_id,
            requests: Vec::new(),
            loading: false,
            channel_store,
            user_store,
            _subscriptions: subscriptions,
        };
        this.load_requests(cx);
        this
    }

    pub fn load_requests(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }

        self.loading = true;
        let channel_id = self.channel_id;
        let client = self.channel_store.read(cx).client();
        let user_store = self.user_store.clone();
        cx.spawn(async move |this, cx| {
            let response = client
                .request(proto::GetPendingJoinRequests {
                    channel_id: channel_id.0,
                })
                .await?;
            let user_ids = response
                .requests
                .iter()
                .map(|request| request.user_id)
                .collect::<Vec<_>>();
            let users = user_store
                .update(cx, |store, cx| store.get_users(user_ids, cx))
                .await?;
            let requests = response
                .requests
                .into_iter()
                .zip(users)
                .filter_map(|(request, user)| {
                    OffsetDateTime::from_unix_timestamp(request.created_at as i64)
                        .ok()
                        .map(|created_at| PendingJoinRequest {
                            user_id: request.user_id,
                            user,
                            reason: request.reason.map(Into::into),
                            created_at,
                        })
                })
                .collect();
            this.update(cx, |this, cx| {
                this.requests = requests;
                this.loading = false;
                cx.notify();
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    async fn handle_join_request_added(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::JoinRequestAdded>,
        mut cx: AsyncApp,
    ) -> Result<()> {
        let channel_id = ChannelId(envelope.payload.channel_id);
        let should_refresh = this.read_with(&cx, |this, cx| {
            this.channel_id == channel_id
                && this.channel_store.read(cx).is_channel_admin(channel_id)
        });
        if should_refresh {
            this.update(&mut cx, |this, cx| this.load_requests(cx));
        }
        Ok(())
    }
}

impl Render for PendingRequestsList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(
                Label::new(format!("Pending Requests ({})", self.requests.len()))
                    .size(LabelSize::Large),
            )
            .when(self.loading, |this| {
                this.child(Label::new("Loading requests...").color(Color::Muted))
            })
            .when(!self.loading && self.requests.is_empty(), |this| {
                this.child(Label::new("No pending requests").color(Color::Muted))
            })
            .children(self.requests.iter().cloned().map(|request| {
                let requester_name = request.user.github_login.clone();
                let detail = request
                    .reason
                    .clone()
                    .unwrap_or_else(|| "No reason provided".into());
                let timestamp = request.created_at.to_string();
                v_flex()
                    .id(("pending-request", request.user_id))
                    .p_2()
                    .gap_1()
                    .cursor_pointer()
                    .border_1()
                    .border_color(cx.theme().colors().border)
                    .child(Label::new(requester_name))
                    .child(
                        Label::new(detail)
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(
                        Label::new(timestamp)
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    )
                    .on_click(cx.listener(move |_this, _, _, cx| {
                        cx.emit(PendingRequestsListEvent::RequestSelected(request.clone()));
                    }))
            }))
    }
}
