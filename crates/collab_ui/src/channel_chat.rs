use crate::draft_store::DraftStore;
use anyhow::Result;
use channel::{Channel, ChannelStore};
use client::{
    ChannelId, Client, UserStore,
    channel_chat::SendChannelMessage,
    proto::{self, ChannelVisibility},
};
use db::kvp::KeyValueStore;
use editor::{Editor, EditorEvent};
use gpui::{
    App, AsyncApp, Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    KeyBinding, PromptLevel, Render, SharedString, StatefulInteractiveElement,
    Subscription as GpuiSubscription, Task, VisualContext as _, WeakEntity, Window, actions,
    prelude::*,
};
use menu::Confirm;
use rpc::{ErrorExt as _, TypedEnvelope};
use std::{
    any::TypeId,
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use ui::{Tooltip, prelude::*};
use util::ResultExt;
use workspace::{
    Workspace,
    item::{Item, TabContentParams},
};

#[path = "channel_chat/compose_area.rs"]
mod compose_area;
#[path = "channel_chat/formatting_toolbar.rs"]
mod formatting_toolbar;
#[path = "channel_chat/markdown_style.rs"]
mod markdown_style;
#[path = "channel_chat/message_bubble.rs"]
mod message_bubble;
#[path = "channel_chat/sanitize.rs"]
mod sanitize;

const RECENT_EMOJI_NAMESPACE: &str = "channel_chat_recent_emojis";
const RECENT_EMOJI_KEY: &str = "recent";
const MAX_RECENT_EMOJIS: usize = 12;
const REACTION_UPDATE_ATTEMPTS: usize = 3;
const REACTION_RETRY_DELAYS: [Duration; REACTION_UPDATE_ATTEMPTS - 1] =
    [Duration::from_millis(250), Duration::from_millis(750)];
const DRAFT_SAVE_DEBOUNCE: Duration = Duration::from_millis(500);

actions!(
    channel_chat,
    [
        /// Discards the current channel chat draft.
        DiscardDraft,
        /// Applies bold Markdown formatting to the channel chat draft.
        ToggleBold,
        /// Applies italic Markdown formatting to the channel chat draft.
        ToggleItalic,
        /// Applies inline code Markdown formatting to the channel chat draft.
        ToggleCode,
        /// Applies link Markdown formatting to the channel chat draft.
        ToggleLink,
        /// Applies blockquote Markdown formatting to the channel chat draft.
        ToggleBlockquote,
        /// Toggles the channel chat draft between source and preview modes.
        TogglePreview
    ]
);

pub fn init(cx: &mut App) {
    cx.bind_keys(channel_chat_key_bindings());
}

fn channel_chat_key_bindings() -> [KeyBinding; 5] {
    [
        KeyBinding::new("ctrl-b", ToggleBold, Some("ChannelChat")),
        KeyBinding::new("ctrl-i", ToggleItalic, Some("ChannelChat")),
        KeyBinding::new("ctrl-`", ToggleCode, Some("ChannelChat")),
        KeyBinding::new("ctrl-shift-k", ToggleLink, Some("ChannelChat")),
        KeyBinding::new("ctrl-shift-p", TogglePreview, Some("ChannelChat")),
    ]
}

pub struct ChannelChat {
    channel_id: ChannelId,
    client: Arc<Client>,
    user_store: Entity<UserStore>,
    channel_store: Entity<ChannelStore>,
    workspace: WeakEntity<Workspace>,
    composer: Entity<Editor>,
    compose_mode: compose_area::ComposeMode,
    compose_preview: Option<compose_area::PreviewBody>,
    emoji_search: Entity<Editor>,
    messages: Vec<proto::ChannelMessage>,
    message_bodies: HashMap<u64, message_bubble::MessageBody>,
    emoji_picker: Option<EmojiPickerState>,
    recent_emoji_names: Vec<String>,
    send_state: SendState,
    pending_draft_save: Option<Task<()>>,
    _rpc_subscriptions: Vec<client::Subscription>,
    _composer_subscription: GpuiSubscription,
    _emoji_search_subscription: GpuiSubscription,
}

#[derive(Clone, PartialEq, Eq)]
enum SendState {
    Idle,
    Sending,
    Failed(SharedString),
}

struct EmojiPickerState {
    message_id: u64,
}

struct EmojiDefinition {
    name: &'static str,
    character: &'static str,
    keywords: &'static [&'static str],
}

const EMOJI_DEFINITIONS: &[EmojiDefinition] = &[
    EmojiDefinition {
        name: "thumbs_up",
        character: "👍",
        keywords: &["approve", "yes", "like", "plus"],
    },
    EmojiDefinition {
        name: "thumbs_up_light_skin_tone",
        character: "👍🏻",
        keywords: &["approve", "yes", "like", "plus", "skin", "tone"],
    },
    EmojiDefinition {
        name: "thumbs_up_medium_light_skin_tone",
        character: "👍🏼",
        keywords: &["approve", "yes", "like", "plus", "skin", "tone"],
    },
    EmojiDefinition {
        name: "thumbs_up_medium_skin_tone",
        character: "👍🏽",
        keywords: &["approve", "yes", "like", "plus", "skin", "tone"],
    },
    EmojiDefinition {
        name: "thumbs_up_medium_dark_skin_tone",
        character: "👍🏾",
        keywords: &["approve", "yes", "like", "plus", "skin", "tone"],
    },
    EmojiDefinition {
        name: "thumbs_up_dark_skin_tone",
        character: "👍🏿",
        keywords: &["approve", "yes", "like", "plus", "skin", "tone"],
    },
    EmojiDefinition {
        name: "heart",
        character: "❤️",
        keywords: &["love", "favorite", "like"],
    },
    EmojiDefinition {
        name: "laugh",
        character: "😄",
        keywords: &["funny", "smile", "happy"],
    },
    EmojiDefinition {
        name: "hooray",
        character: "🎉",
        keywords: &["celebrate", "party", "ship"],
    },
    EmojiDefinition {
        name: "eyes",
        character: "👀",
        keywords: &["look", "watch", "seen"],
    },
    EmojiDefinition {
        name: "rocket",
        character: "🚀",
        keywords: &["ship", "launch", "fast"],
    },
    EmojiDefinition {
        name: "fire",
        character: "🔥",
        keywords: &["hot", "great", "lit"],
    },
    EmojiDefinition {
        name: "check",
        character: "✅",
        keywords: &["done", "yes", "complete"],
    },
    EmojiDefinition {
        name: "thinking",
        character: "🤔",
        keywords: &["question", "consider", "hmm"],
    },
    EmojiDefinition {
        name: "pray",
        character: "🙏",
        keywords: &["thanks", "please", "appreciate"],
    },
    EmojiDefinition {
        name: "tada",
        character: "🎊",
        keywords: &["celebrate", "confetti", "party"],
    },
    EmojiDefinition {
        name: "mind_blown",
        character: "🤯",
        keywords: &["wow", "amazed", "surprise"],
    },
];

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
        let emoji_search = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Search emoji", window, cx);
            editor
        });
        let restored_draft = DraftStore::global(cx)
            .update(cx, |draft_store, _| draft_store.cached_draft(channel_id));
        if let Some(restored_draft) = restored_draft {
            composer.update(cx, |composer, cx| {
                if composer.is_empty(cx) {
                    composer.set_text(restored_draft, window, cx);
                }
            });
        }
        let _composer_subscription = cx.subscribe(&composer, |this, _, event: &EditorEvent, cx| {
            if matches!(event, EditorEvent::Edited { .. }) {
                this.schedule_draft_save(cx);
            }
            cx.notify();
        });
        let _emoji_search_subscription = cx.observe(&emoji_search, |_, _, cx| cx.notify());
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
        let recent_emoji_names = Self::load_recent_emoji_names(cx);

        Self {
            channel_id,
            client,
            user_store,
            channel_store,
            workspace,
            composer,
            compose_mode: compose_area::ComposeMode::Source,
            compose_preview: None,
            emoji_search,
            messages,
            message_bodies: HashMap::default(),
            emoji_picker: None,
            recent_emoji_names,
            send_state: SendState::Idle,
            pending_draft_save: None,
            _rpc_subscriptions,
            _composer_subscription,
            _emoji_search_subscription,
        }
    }

    fn load_recent_emoji_names(cx: &App) -> Vec<String> {
        KeyValueStore::global(cx)
            .scoped(RECENT_EMOJI_NAMESPACE)
            .read(RECENT_EMOJI_KEY)
            .log_err()
            .flatten()
            .and_then(|json| serde_json::from_str::<Vec<String>>(&json).log_err())
            .unwrap_or_default()
            .into_iter()
            .filter(|emoji_name| emoji_by_name(emoji_name).is_some())
            .take(MAX_RECENT_EMOJIS)
            .collect()
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
                    this.pending_draft_save.take();
                    let clear_draft = DraftStore::global(cx).update(cx, |draft_store, cx| {
                        draft_store.clear_draft_in_background(channel_id, cx)
                    });
                    clear_draft.detach_and_log_err(cx);
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

    fn discard_draft(&mut self, _: &DiscardDraft, window: &mut Window, cx: &mut Context<Self>) {
        if self.composer.read(cx).text(cx).is_empty() {
            return;
        }

        let answer = window.prompt(
            PromptLevel::Warning,
            "Discard draft?",
            Some("This will clear the unsent message for this channel."),
            &["Discard", "Cancel"],
            cx,
        );
        let channel_id = self.channel_id;

        cx.spawn_in(window, async move |this, cx| {
            if answer.await? != 0 {
                return anyhow::Ok(());
            }

            this.update_in(cx, |this, window, cx| {
                this.pending_draft_save.take();
                let clear_draft = DraftStore::global(cx).update(cx, |draft_store, cx| {
                    draft_store.clear_draft_in_background(channel_id, cx)
                });
                clear_draft.detach_and_log_err(cx);
                this.composer.update(cx, |composer, cx| {
                    composer.clear(window, cx);
                });
                cx.notify();
            })?;

            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn schedule_draft_save(&mut self, cx: &mut Context<Self>) {
        let channel_id = self.channel_id;
        let body = self.composer.read(cx).text(cx);
        let draft_store = DraftStore::global(cx);

        self.pending_draft_save = Some(cx.spawn(async move |_, cx| {
            cx.background_executor().timer(DRAFT_SAVE_DEBOUNCE).await;
            let save_task = draft_store.update(cx, |draft_store, cx| {
                draft_store.save_draft_in_background(channel_id, body, cx)
            });
            save_task.await.log_err();
        }));
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

    fn rendered_message_body(
        &mut self,
        message: &proto::ChannelMessage,
        cx: &mut Context<Self>,
    ) -> &message_bubble::MessageBody {
        match self.message_bodies.entry(message.id) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if entry.get().source() != message.body.as_str() {
                    entry.insert(message_bubble::MessageBody::new(message.body.clone(), cx));
                }
                entry.into_mut()
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(message_bubble::MessageBody::new(message.body.clone(), cx))
            }
        }
    }

    fn render_message(
        &mut self,
        message: &proto::ChannelMessage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let sender = self.user_display_name(message.sender_id, cx);
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
            .child(self.rendered_message_body(message, cx).render(window, cx))
            .child(self.render_reactions(message, cx))
            .into_any_element()
    }

    fn render_reactions(
        &self,
        message: &proto::ChannelMessage,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let current_user_id = self
            .user_store
            .read(cx)
            .current_user()
            .map(|user| user.legacy_id);

        v_flex()
            .gap_1()
            .child(
                h_flex()
                    .gap_1()
                    .children(message.reaction_summaries.iter().enumerate().map(
                        |(reaction_index, reaction)| {
                            let message_id = message.id;
                            let emoji_name = reaction.emoji_name.clone();
                            let reacted_by_me = current_user_id
                                .is_some_and(|user_id| reaction.user_ids.contains(&user_id));
                            let tooltip = self.reaction_tooltip(reaction, cx);
                            let label = format!(
                                "{} {}",
                                emoji_character(&reaction.emoji_name),
                                reaction.count
                            );

                            div()
                                .id((
                                    gpui::ElementId::from(("channel-reaction", message.id)),
                                    reaction_index.to_string(),
                                ))
                                .px_2()
                                .py_0p5()
                                .border_1()
                                .rounded_md()
                                .text_size(rems(0.75))
                                .line_height(rems(1.0))
                                .border_color(if reacted_by_me {
                                    cx.theme().colors().editor_foreground
                                } else {
                                    cx.theme().colors().border_variant
                                })
                                .bg(if reacted_by_me {
                                    cx.theme().colors().element_hover
                                } else {
                                    gpui::transparent_black()
                                })
                                .child(label)
                                .tooltip(Tooltip::text(tooltip))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.toggle_reaction(
                                        message_id,
                                        emoji_name.clone(),
                                        reacted_by_me,
                                        window,
                                        cx,
                                    );
                                }))
                        },
                    ))
                    .child(
                        IconButton::new(
                            (
                                gpui::ElementId::from(("channel-add-reaction", message.id)),
                                "button",
                            ),
                            IconName::Plus,
                        )
                        .icon_size(IconSize::XSmall)
                        .size(ButtonSize::None)
                        .tooltip(Tooltip::text("Add reaction"))
                        .on_click(cx.listener({
                            let message_id = message.id;
                            move |this, _, window, cx| {
                                this.open_emoji_picker(message_id, window, cx);
                            }
                        })),
                    ),
            )
            .when(
                self.emoji_picker
                    .as_ref()
                    .is_some_and(|picker| picker.message_id == message.id),
                |this| this.child(self.render_emoji_picker(message.id, cx)),
            )
            .into_any_element()
    }

    fn user_display_name(&self, user_id: u64, cx: &mut Context<Self>) -> String {
        self.user_store
            .update(cx, |user_store, cx| {
                user_store.get_user_optimistic(user_id, cx)
            })
            .map(|user| {
                user.name
                    .clone()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| user.github_login.to_string())
            })
            .unwrap_or_else(|| format!("User {user_id}"))
    }

    fn reaction_tooltip(
        &self,
        reaction: &proto::ReactionSummary,
        cx: &mut Context<Self>,
    ) -> String {
        reaction
            .user_ids
            .iter()
            .map(|user_id| self.user_display_name(*user_id, cx))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn render_emoji_picker(&self, message_id: u64, cx: &mut Context<Self>) -> gpui::AnyElement {
        let emoji_options = self.filtered_emoji_options(cx);

        v_flex()
            .id(("channel-emoji-picker", message_id))
            .gap_2()
            .mt_1()
            .p_2()
            .w(px(248.))
            .max_h(px(220.))
            .overflow_y_scroll()
            .border_1()
            .rounded_md()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().editor_background)
            .child(self.emoji_search.clone())
            .when(emoji_options.is_empty(), |this| {
                this.child(
                    Label::new("No emojis found — try a different search")
                        .size(LabelSize::XSmall)
                        .color(Color::Muted),
                )
            })
            .when(!emoji_options.is_empty(), |this| {
                this.child(
                    h_flex().gap_1().flex_wrap().children(
                        emoji_options
                            .into_iter()
                            .enumerate()
                            .map(|(emoji_index, emoji)| {
                                div()
                                    .id((
                                        gpui::ElementId::from(("channel-emoji-option", message_id)),
                                        emoji_index.to_string(),
                                    ))
                                    .size(px(28.))
                                    .rounded_md()
                                    .text_size(rems(1.0))
                                    .line_height(rems(1.5))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .hover(|style| style.bg(cx.theme().colors().element_hover))
                                    .child(emoji.character)
                                    .tooltip(Tooltip::text(emoji.name))
                                    .on_click(cx.listener({
                                        let emoji_name = emoji.name.to_string();
                                        move |this, _, window, cx| {
                                            this.select_emoji_for_message(
                                                message_id,
                                                emoji_name.clone(),
                                                window,
                                                cx,
                                            );
                                        }
                                    }))
                            }),
                    ),
                )
            })
            .into_any_element()
    }

    fn filtered_emoji_options(&self, cx: &App) -> Vec<&'static EmojiDefinition> {
        let query = self
            .emoji_search
            .read(cx)
            .text(cx)
            .trim()
            .to_ascii_lowercase();
        let mut emoji_options = EMOJI_DEFINITIONS
            .iter()
            .enumerate()
            .filter(|(_, emoji)| {
                query.is_empty()
                    || emoji.name.contains(&query)
                    || emoji
                        .keywords
                        .iter()
                        .any(|keyword| keyword.contains(&query))
            })
            .collect::<Vec<_>>();

        emoji_options.sort_by_key(|(emoji_index, emoji)| {
            let recent_rank = self
                .recent_emoji_names
                .iter()
                .position(|recent| recent == emoji.name)
                .unwrap_or(MAX_RECENT_EMOJIS);
            (recent_rank, *emoji_index)
        });
        emoji_options.into_iter().map(|(_, emoji)| emoji).collect()
    }

    fn open_emoji_picker(&mut self, message_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let is_current_picker = self
            .emoji_picker
            .as_ref()
            .is_some_and(|picker| picker.message_id == message_id);

        if is_current_picker {
            self.emoji_picker = None;
        } else {
            self.emoji_search
                .update(cx, |search, cx| search.clear(window, cx));
            self.emoji_picker = Some(EmojiPickerState { message_id });
        }

        cx.notify();
    }

    fn select_emoji_for_message(
        &mut self,
        message_id: u64,
        emoji_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if emoji_by_name(&emoji_name).is_none() {
            return;
        }

        self.emoji_picker = None;
        self.remember_recent_emoji(emoji_name.clone(), cx);
        self.toggle_reaction(message_id, emoji_name, false, window, cx);
        cx.notify();
    }

    fn remember_recent_emoji(&mut self, emoji_name: String, cx: &mut Context<Self>) {
        self.recent_emoji_names
            .retain(|recent_emoji_name| recent_emoji_name != &emoji_name);
        self.recent_emoji_names.insert(0, emoji_name);
        self.recent_emoji_names.truncate(MAX_RECENT_EMOJIS);

        let recent_emoji_names = self.recent_emoji_names.clone();
        let kvp = KeyValueStore::global(cx);
        cx.background_spawn(async move {
            let result: anyhow::Result<()> = async {
                let json = serde_json::to_string(&recent_emoji_names)?;
                kvp.scoped(RECENT_EMOJI_NAMESPACE)
                    .write(RECENT_EMOJI_KEY.to_string(), json)
                    .await?;
                Ok(())
            }
            .await;
            result.log_err();
        })
        .detach();
    }

    fn toggle_reaction(
        &mut self,
        message_id: u64,
        emoji_name: String,
        reacted_by_me: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if emoji_by_name(&emoji_name).is_none() {
            self.workspace
                .update(cx, |workspace, cx| {
                    workspace.show_error(format!("Unsupported reaction emoji: {emoji_name}"), cx);
                })
                .log_err();
            return;
        }

        let client = self.client.clone();
        let workspace = self.workspace.clone();
        let channel_id = self.channel_id;
        let this = cx.weak_entity();

        cx.spawn_in(window, async move |_, cx| {
            let mut last_error = None;
            for attempt in 0..REACTION_UPDATE_ATTEMPTS {
                let result = if reacted_by_me {
                    client
                        .remove_channel_message_reaction(
                            channel_id.0,
                            message_id,
                            emoji_name.clone(),
                        )
                        .await
                } else {
                    client
                        .add_channel_message_reaction(channel_id.0, message_id, emoji_name.clone())
                        .await
                };

                match result {
                    Ok(_) => return anyhow::Ok(()),
                    Err(error) if is_missing_channel_message_error(&error) => {
                        this.update(cx, |this, cx| {
                            this.remove_message(message_id, cx);
                        })
                        .log_err();
                        return anyhow::Ok(());
                    }
                    Err(error) => {
                        last_error = Some(error);
                        if let Some(delay) = REACTION_RETRY_DELAYS.get(attempt) {
                            cx.background_executor().timer(*delay).await;
                        }
                    }
                }
            }

            if let Some(error) = last_error {
                workspace
                    .update(cx, |workspace, cx| {
                        workspace.show_error(
                            format!(
                                "Failed to update reaction after {REACTION_UPDATE_ATTEMPTS} attempts: {error}"
                            ),
                            cx,
                        );
                    })
                    .log_err();
            }

            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn remove_message(&mut self, message_id: u64, cx: &mut Context<Self>) {
        let original_len = self.messages.len();
        self.messages.retain(|message| message.id != message_id);
        if self.messages.len() != original_len {
            cx.notify();
        }
    }

    fn format_composer(
        &mut self,
        format_kind: formatting_toolbar::FormatKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.compose_mode == compose_area::ComposeMode::Preview {
            self.compose_mode = compose_area::ComposeMode::Source;
        }
        self.composer.update(cx, |composer, cx| {
            formatting_toolbar::apply_format(format_kind, composer, window, cx);
        });
        window.focus(&self.composer.focus_handle(cx), cx);
    }

    fn toggle_bold(&mut self, _: &ToggleBold, window: &mut Window, cx: &mut Context<Self>) {
        self.format_composer(formatting_toolbar::FormatKind::Bold, window, cx);
    }

    fn toggle_italic(&mut self, _: &ToggleItalic, window: &mut Window, cx: &mut Context<Self>) {
        self.format_composer(formatting_toolbar::FormatKind::Italic, window, cx);
    }

    fn toggle_code(&mut self, _: &ToggleCode, window: &mut Window, cx: &mut Context<Self>) {
        self.format_composer(formatting_toolbar::FormatKind::Code, window, cx);
    }

    fn toggle_link(&mut self, _: &ToggleLink, window: &mut Window, cx: &mut Context<Self>) {
        self.format_composer(formatting_toolbar::FormatKind::Link, window, cx);
    }

    fn toggle_blockquote(
        &mut self,
        _: &ToggleBlockquote,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.format_composer(formatting_toolbar::FormatKind::Blockquote, window, cx);
    }

    fn toggle_preview(&mut self, _: &TogglePreview, window: &mut Window, cx: &mut Context<Self>) {
        self.compose_mode = self.compose_mode.toggle();
        if self.compose_mode == compose_area::ComposeMode::Source {
            window.focus(&self.composer.focus_handle(cx), cx);
        }
        cx.notify();
    }

    fn render_compose_preview(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let draft = self.composer.read(cx).text(cx);
        if self
            .compose_preview
            .as_ref()
            .is_none_or(|preview| preview.source() != draft.as_str())
        {
            self.compose_preview = Some(compose_area::PreviewBody::new(draft, cx));
        }

        match &self.compose_preview {
            Some(preview) => preview.render(window, cx),
            None => div().flex_1().into_any_element(),
        }
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
    pub fn reaction_chip_labels_for_test(&self) -> Vec<Vec<String>> {
        self.messages
            .iter()
            .map(|message| {
                message
                    .reaction_summaries
                    .iter()
                    .map(|reaction| {
                        format!(
                            "{} {}",
                            emoji_character(&reaction.emoji_name),
                            reaction.count
                        )
                    })
                    .collect()
            })
            .collect()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn reaction_tooltips_for_test(&self, cx: &mut Context<Self>) -> Vec<Vec<String>> {
        let mut tooltips = Vec::new();
        for message in &self.messages {
            let mut message_tooltips = Vec::new();
            for reaction in &message.reaction_summaries {
                message_tooltips.push(self.reaction_tooltip(reaction, cx));
            }
            tooltips.push(message_tooltips);
        }
        tooltips
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn emoji_picker_labels_for_test(&self, cx: &App) -> Vec<String> {
        self.filtered_emoji_options(cx)
            .into_iter()
            .map(|emoji| format!("{} {}", emoji.character, emoji.name))
            .collect()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn emoji_picker_empty_for_test(&self, cx: &App) -> bool {
        self.filtered_emoji_options(cx).is_empty()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn emoji_picker_open_for_test(&self) -> bool {
        self.emoji_picker.is_some()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn recent_emoji_names_for_test(&self) -> Vec<String> {
        self.recent_emoji_names.clone()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn open_emoji_picker_for_test(
        &mut self,
        message_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_emoji_picker(message_id, window, cx);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_emoji_search_for_test(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.emoji_search
            .update(cx, |search, cx| search.set_text(text, window, cx));
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn select_emoji_for_test(
        &mut self,
        message_id: u64,
        emoji_name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_emoji_for_message(message_id, emoji_name.to_string(), window, cx);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn draft_for_test(&self, cx: &App) -> String {
        self.composer.read(cx).text(cx)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn compose_mode_for_test(&self) -> &'static str {
        match self.compose_mode {
            compose_area::ComposeMode::Source => "source",
            compose_area::ComposeMode::Preview => "preview",
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn toggle_preview_for_test(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_preview(&TogglePreview, window, cx);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn focus_composer_for_test(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.composer.focus_handle(cx), cx);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn blur_for_test(&self, window: &mut Window) {
        window.blur();
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn formatting_toolbar_visible_for_test(&self, window: &Window, cx: &App) -> bool {
        self.compose_mode == compose_area::ComposeMode::Source
            && self.composer.read(cx).is_focused(window)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn rendered_message_texts_for_test(
        this: Entity<Self>,
        cx: &mut gpui::VisualTestContext,
    ) -> Vec<String> {
        let markdowns = cx.update(|_, cx| {
            this.update(cx, |this, cx| {
                let messages = this.messages.clone();
                messages
                    .iter()
                    .map(|message| this.rendered_message_body(message, cx).markdown_for_test())
                    .collect::<Vec<_>>()
            })
        });

        markdowns
            .into_iter()
            .map(|markdown| {
                markdown::MarkdownElement::rendered_text(
                    markdown,
                    cx,
                    markdown_style::channel_chat_markdown_style,
                )
            })
            .collect()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn rendered_compose_preview_for_test(
        this: Entity<Self>,
        cx: &mut gpui::VisualTestContext,
    ) -> Option<String> {
        let markdown = cx.update(|_, cx| {
            this.update(cx, |this, cx| {
                if this.compose_mode != compose_area::ComposeMode::Preview {
                    return None;
                }

                let draft = this.composer.read(cx).text(cx);
                if this
                    .compose_preview
                    .as_ref()
                    .is_none_or(|preview| preview.source() != draft.as_str())
                {
                    this.compose_preview = Some(compose_area::PreviewBody::new(draft, cx));
                }

                this.compose_preview
                    .as_ref()
                    .map(compose_area::PreviewBody::markdown_for_test)
            })
        });

        markdown.map(|markdown| {
            markdown::MarkdownElement::rendered_text(
                markdown,
                cx,
                markdown_style::channel_chat_markdown_style,
            )
        })
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("ChannelChat")
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .on_action(cx.listener(Self::send))
            .on_action(cx.listener(Self::discard_draft))
            .on_action(cx.listener(Self::toggle_bold))
            .on_action(cx.listener(Self::toggle_italic))
            .on_action(cx.listener(Self::toggle_code))
            .on_action(cx.listener(Self::toggle_link))
            .on_action(cx.listener(Self::toggle_blockquote))
            .on_action(cx.listener(Self::toggle_preview))
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
                            .clone()
                            .into_iter()
                            .map(|message| self.render_message(&message, window, cx)),
                    ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .border_t_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_between()
                            .child(div().flex_1().when(
                                self.compose_mode == compose_area::ComposeMode::Source
                                    && self.composer.read(cx).is_focused(window),
                                |this| {
                                    this.child(
                                        formatting_toolbar::FormattingToolbar::new(
                                            self.composer.clone(),
                                        )
                                        .render(window, cx),
                                    )
                                },
                            ))
                            .child(
                                IconButton::new(
                                    "toggle-compose-preview",
                                    self.compose_mode.toggle_icon(),
                                )
                                .icon_size(IconSize::Small)
                                .icon_color(Color::Muted)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.toggle_preview(&TogglePreview, window, cx);
                                }))
                                .tooltip(Tooltip::text(self.compose_mode.toggle_tooltip())),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(match self.compose_mode {
                                compose_area::ComposeMode::Source => div()
                                    .flex_1()
                                    .child(self.composer.clone())
                                    .into_any_element(),
                                compose_area::ComposeMode::Preview => {
                                    self.render_compose_preview(window, cx)
                                }
                            })
                            .when(!self.composer.read(cx).text(cx).is_empty(), |this| {
                                this.child(
                                    IconButton::new("discard-draft", IconName::Trash)
                                        .icon_size(IconSize::Small)
                                        .icon_color(Color::Muted)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.discard_draft(&DiscardDraft, window, cx);
                                        }))
                                        .tooltip(Tooltip::text("Discard draft")),
                                )
                            }),
                    )
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

fn emoji_character(emoji_name: &str) -> &str {
    emoji_by_name(emoji_name)
        .map(|emoji| emoji.character)
        .unwrap_or(emoji_name)
}

fn emoji_by_name(emoji_name: &str) -> Option<&'static EmojiDefinition> {
    EMOJI_DEFINITIONS
        .iter()
        .find(|emoji| emoji.name == emoji_name)
}

fn is_missing_channel_message_error(error: &anyhow::Error) -> bool {
    let error = error.to_proto();
    error.code == proto::ErrorCode::Internal as i32
        && error.message.contains("no channel message")
        && error.message.contains(" in channel ")
}

fn next_nonce(channel_id: ChannelId) -> u128 {
    let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    };
    nanos ^ u128::from(channel_id.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn channel_chat_key_bindings_parse() {
        let bindings = super::channel_chat_key_bindings();

        assert_eq!(bindings.len(), 5);
        assert!(bindings.iter().any(|binding| {
            binding.action().name().ends_with("TogglePreview")
                && binding
                    .keystrokes()
                    .first()
                    .is_some_and(|keystroke| keystroke.unparse() == "ctrl-shift-p")
        }));
    }
}
