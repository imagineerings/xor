use anyhow::Result;
use channel::{Channel, ChannelStore};
use client::{
    ChannelId, Client, UserStore,
    channel_chat::SendChannelMessage,
    proto::{self, ChannelVisibility},
};
use editor::Editor;
use gpui::{
    App, AsyncApp, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    Render, SharedString, StatefulInteractiveElement, Subscription as GpuiSubscription, Task,
    VisualContext as _, WeakEntity, Window, prelude::*,
};
use menu::Confirm;
use rpc::TypedEnvelope;
use std::{
    any::TypeId,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use ui::prelude::*;
use util::ResultExt;
use workspace::{
    Workspace,
    item::{Item, TabContentParams},
};

pub fn init(_cx: &mut App) {}

pub struct ChannelChat {
    channel_id: ChannelId,
    client: Arc<Client>,
    user_store: Entity<UserStore>,
    channel_store: Entity<ChannelStore>,
    workspace: WeakEntity<Workspace>,
    composer: Entity<Editor>,
    messages: Vec<proto::ChannelMessage>,
    send_state: SendState,
    _rpc_subscriptions: Vec<client::Subscription>,
    _composer_subscription: GpuiSubscription,
}

#[derive(Clone, PartialEq, Eq)]
enum SendState {
    Idle,
    Sending,
    Failed(SharedString),
}

impl ChannelChat {
    pub fn open(
        channel_id: ChannelId,
        workspace: Entity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Entity<Self>>> {
        let pane = workspace.read(cx).active_pane().clone();
        let chat = Self::load(channel_id, workspace, window, cx);
        window.spawn(cx, async move |cx| {
            let chat = chat.await?;
            pane.update_in(cx, |pane, window, cx| {
                if let Some(existing_chat) = pane
                    .items_of_type::<Self>()
                    .find(|chat| chat.read(cx).channel_id == channel_id)
                {
                    return existing_chat;
                }

                pane.add_item(Box::new(chat.clone()), true, true, None, window, cx);
                chat
            })
        })
    }

    pub fn load(
        channel_id: ChannelId,
        workspace: Entity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Entity<Self>>> {
        let client = workspace.read(cx).client().clone();
        let user_store = workspace.read(cx).user_store().clone();
        let channel_store = ChannelStore::global(cx);
        let weak_workspace = workspace.downgrade();

        window.spawn(cx, async move |cx| {
            let response = client.join_channel_chat(channel_id.0).await?;
            cx.new_window_entity(|window, cx| {
                Self::new(
                    channel_id,
                    client,
                    user_store,
                    channel_store,
                    weak_workspace,
                    response.messages,
                    window,
                    cx,
                )
            })
        })
    }

    fn new(
        channel_id: ChannelId,
        client: Arc<Client>,
        user_store: Entity<UserStore>,
        channel_store: Entity<ChannelStore>,
        workspace: WeakEntity<Workspace>,
        mut messages: Vec<proto::ChannelMessage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        messages.sort_by_key(|message| (message.timestamp, message.id));

        let composer = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Message channel", window, cx);
            editor
        });
        let _composer_subscription = cx.observe(&composer, |_, _, cx| cx.notify());
        let weak_self = cx.weak_entity();
        let _rpc_subscriptions = vec![
            client.add_channel_message_sent_handler(weak_self.clone(), Self::handle_message_sent),
            client
                .add_channel_message_update_handler(weak_self.clone(), Self::handle_message_update),
            client.add_channel_message_reactions_update_handler(
                weak_self,
                Self::handle_message_reactions_update,
            ),
        ];

        Self {
            channel_id,
            client,
            user_store,
            channel_store,
            workspace,
            composer,
            messages,
            send_state: SendState::Idle,
            _rpc_subscriptions,
            _composer_subscription,
        }
    }

    async fn handle_message_sent(
        this: Entity<Self>,
        message: TypedEnvelope<proto::ChannelMessageSent>,
        mut cx: AsyncApp,
    ) -> Result<()> {
        this.update(&mut cx, |this, cx| {
            if message.payload.channel_id == this.channel_id.0
                && let Some(message) = message.payload.message
            {
                this.upsert_message(message, cx);
            }
        });
        Ok(())
    }

    async fn handle_message_update(
        this: Entity<Self>,
        message: TypedEnvelope<proto::ChannelMessageUpdate>,
        mut cx: AsyncApp,
    ) -> Result<()> {
        this.update(&mut cx, |this, cx| {
            if message.payload.channel_id == this.channel_id.0
                && let Some(message) = message.payload.message
            {
                this.upsert_message(message, cx);
            }
        });
        Ok(())
    }

    async fn handle_message_reactions_update(
        this: Entity<Self>,
        update: TypedEnvelope<proto::UpdateMessageReactions>,
        mut cx: AsyncApp,
    ) -> Result<()> {
        this.update(&mut cx, |this, cx| {
            if update.payload.channel_id == this.channel_id.0
                && let Some(message) = this
                    .messages
                    .iter_mut()
                    .find(|message| message.id == update.payload.message_id)
            {
                message.reaction_summaries = update.payload.reactions;
                cx.notify();
            }
        });
        Ok(())
    }

    fn upsert_message(&mut self, message: proto::ChannelMessage, cx: &mut Context<Self>) {
        if let Some(existing) = self
            .messages
            .iter_mut()
            .find(|existing| existing.id == message.id)
        {
            *existing = message;
        } else {
            self.messages.push(message);
            self.messages
                .sort_by_key(|message| (message.timestamp, message.id));
        }

        if let Some(latest_message_id) = self.messages.last().map(|message| message.id) {
            self.client
                .acknowledge_channel_message(self.channel_id.0, latest_message_id)
                .log_err();
        }

        cx.notify();
    }

    fn send(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if self.send_state == SendState::Sending {
            return;
        }

        let body = self.composer.read(cx).text(cx).trim().to_string();
        if body.is_empty() {
            return;
        }

        self.send_state = SendState::Sending;
        cx.notify();

        let client = self.client.clone();
        let channel_id = self.channel_id;
        cx.spawn_in(window, async move |this, cx| {
            let nonce = next_nonce(channel_id);
            let send_result = client
                .send_channel_message(SendChannelMessage {
                    channel_id: channel_id.0,
                    body,
                    nonce,
                    mentions: Vec::new(),
                    reply_to_message_id: None,
                })
                .await;

            this.update_in(cx, |this, window, cx| match send_result {
                Ok(message) => {
                    this.composer.update(cx, |composer, cx| {
                        composer.clear(window, cx);
                    });
                    this.send_state = SendState::Idle;
                    this.upsert_message(message, cx);
                }
                Err(error) => {
                    let message = SharedString::from(error.to_string());
                    this.send_state = SendState::Failed(message.clone());
                    this.workspace
                        .update(cx, |workspace, cx| {
                            workspace.show_error(format!("Failed to send message: {message}"), cx);
                        })
                        .log_err();
                    cx.notify();
                }
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn channel(&self, cx: &App) -> Option<Arc<Channel>> {
        self.channel_store
            .read(cx)
            .channel_for_id(self.channel_id)
            .cloned()
    }

    fn tab_name(&self, cx: &App) -> SharedString {
        self.channel(cx)
            .map(|channel| format!("{} chat", channel.name).into())
            .unwrap_or_else(|| "Channel chat".into())
    }

    fn render_message(
        &self,
        message: &proto::ChannelMessage,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let sender = self
            .user_store
            .update(cx, |user_store, cx| {
                user_store.get_user_optimistic(message.sender_id, cx)
            })
            .map(|user| {
                user.name
                    .clone()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| user.github_login.to_string())
            })
            .unwrap_or_else(|| format!("User {}", message.sender_id));
        let timestamp = format_timestamp(message.timestamp);
        let edited = message.edited_at.is_some();

        v_flex()
            .gap_1()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().colors().border_variant)
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Label::new(sender)
                            .size(LabelSize::Small)
                            .weight(gpui::FontWeight::MEDIUM),
                    )
                    .child(
                        Label::new(timestamp)
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    )
                    .when(edited, |this| {
                        this.child(
                            Label::new("edited")
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        )
                    }),
            )
            .child(Label::new(message.body.clone()).size(LabelSize::Small))
            .into_any_element()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn message_bodies_for_test(&self) -> Vec<String> {
        self.messages
            .iter()
            .map(|message| message.body.clone())
            .collect()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn message_reactions_for_test(&self) -> Vec<Vec<proto::ReactionSummary>> {
        self.messages
            .iter()
            .map(|message| message.reaction_summaries.clone())
            .collect()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn draft_for_test(&self, cx: &App) -> String {
        self.composer.read(cx).text(cx)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn send_error_for_test(&self) -> Option<SharedString> {
        match &self.send_state {
            SendState::Failed(message) => Some(message.clone()),
            SendState::Idle | SendState::Sending => None,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_draft_for_test(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.composer
            .update(cx, |composer, cx| composer.set_text(text, window, cx));
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn send_for_test(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.send(&Confirm, window, cx);
    }
}

impl Drop for ChannelChat {
    fn drop(&mut self) {
        self.client.leave_channel_chat(self.channel_id.0).log_err();
    }
}

impl EventEmitter<()> for ChannelChat {}

impl Focusable for ChannelChat {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.composer.focus_handle(cx)
    }
}

impl Render for ChannelChat {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("ChannelChat")
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .on_action(cx.listener(Self::send))
            .child(
                v_flex()
                    .flex_1()
                    .id("channel-chat-messages")
                    .overflow_y_scroll()
                    .when(self.messages.is_empty(), |this| {
                        this.child(
                            div()
                                .p_4()
                                .child(Label::new("No messages yet").color(Color::Muted)),
                        )
                    })
                    .children(
                        self.messages
                            .iter()
                            .map(|message| self.render_message(message, cx)),
                    ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .border_t_1()
                    .border_color(cx.theme().colors().border)
                    .child(self.composer.clone())
                    .when_some(
                        match &self.send_state {
                            SendState::Failed(message) => Some(message.clone()),
                            SendState::Idle | SendState::Sending => None,
                        },
                        |this, message| {
                            this.child(
                                Label::new(message)
                                    .size(LabelSize::XSmall)
                                    .color(Color::Error),
                            )
                        },
                    ),
            )
    }
}

impl Item for ChannelChat {
    type Event = ();

    fn tab_icon(&self, _: &Window, cx: &App) -> Option<Icon> {
        let icon = match self.channel(cx).map(|channel| channel.visibility) {
            Some(ChannelVisibility::Public) => IconName::Public,
            Some(ChannelVisibility::Members) | None => IconName::Hash,
        };
        Some(Icon::new(icon))
    }

    fn tab_content_text(&self, _detail: usize, cx: &App) -> SharedString {
        self.tab_name(cx)
    }

    fn tab_content(&self, params: TabContentParams, _: &Window, cx: &App) -> gpui::AnyElement {
        Label::new(self.tab_name(cx))
            .color(params.text_color())
            .when(params.preview, |this| this.italic())
            .into_any_element()
    }

    fn act_as_type<'a>(
        &'a self,
        type_id: TypeId,
        self_handle: &'a Entity<Self>,
        _: &'a App,
    ) -> Option<gpui::AnyEntity> {
        if type_id == TypeId::of::<Self>() {
            Some(self_handle.clone().into())
        } else if type_id == TypeId::of::<Editor>() {
            Some(self.composer.clone().into())
        } else {
            None
        }
    }
}

fn format_timestamp(timestamp: u64) -> String {
    let seconds = timestamp / 1000;
    let seconds_in_day = seconds % 86_400;
    let hour = seconds_in_day / 3_600;
    let minute = (seconds_in_day % 3_600) / 60;
    format!("{hour:02}:{minute:02}")
}

fn next_nonce(channel_id: ChannelId) -> u128 {
    let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    };
    nanos ^ u128::from(channel_id.0)
}
