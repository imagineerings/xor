use crate::{
    channel_bookmark_bar::ChannelBookmarkBar,
    channel_bookmark_form::{BookmarkForm, BookmarkFormState},
    channel_bookmark_store::ChannelBookmarkStore,
    channel_file_upload::{UploadManager, UploadProgress, UploadStatus},
    draft_store::DraftStore,
};
use anyhow::Result;
use channel::{Channel, ChannelStore};
use chrono::{
    DateTime, Datelike, Duration as ChronoDuration, Local, LocalResult, NaiveDate, NaiveTime,
    TimeZone as _, Timelike as _, Utc,
};
use client::{
    AddBookmark, Bookmark, BookmarkId, ChannelId, Client, UpdateBookmark, UserStore,
    channel_chat::{
        DEFAULT_THREAD_REPLY_LIMIT, ScheduleChannelMessage, SearchChannelMessages,
        SendChannelMessage, ThreadSummary, UpdateScheduledMessage,
    },
    proto::{self, ChannelVisibility},
    scheduled_message::{ScheduledMessage, ScheduledMessageId},
};
use db::kvp::KeyValueStore;
use editor::{Editor, EditorEvent};
use gpui::{
    App, AsyncApp, ClickEvent, Context, Entity, EventEmitter, ExternalPaths, FocusHandle,
    Focusable, InteractiveElement, KeyBinding, PathPromptOptions, PromptLevel, Render,
    SharedString, StatefulInteractiveElement, Subscription as GpuiSubscription, Task,
    VisualContext as _, WeakEntity, Window, actions, prelude::*,
};
use menu::{Confirm, SelectNext, SelectPrevious};
use rpc::{ErrorExt as _, TypedEnvelope};
use smallvec::SmallVec;
use std::{
    any::TypeId,
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use ui::{Avatar, Facepile, ProgressBar, TintColor, Tooltip, prelude::*};
use util::ResultExt;
use workspace::{
    Toast, Workspace,
    item::{Item, TabContentParams},
    notifications::NotificationId,
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
#[path = "channel_chat/search.rs"]
mod search;

const RECENT_EMOJI_NAMESPACE: &str = "channel_chat_recent_emojis";
const RECENT_EMOJI_KEY: &str = "recent";
const MAX_RECENT_EMOJIS: usize = 12;
const REACTION_UPDATE_ATTEMPTS: usize = 3;
const REACTION_RETRY_DELAYS: [Duration; REACTION_UPDATE_ATTEMPTS - 1] =
    [Duration::from_millis(250), Duration::from_millis(750)];
const THREAD_LOAD_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
];
const DRAFT_SAVE_DEBOUNCE: Duration = Duration::from_millis(500);
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(300);
const SEARCH_PAGE_SIZE: u32 = 20;

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
        TogglePreview,
        /// Toggles channel message search.
        ToggleSearch,
        /// Closes the open channel message thread.
        CloseThread
    ]
);

pub fn init(cx: &mut App) {
    cx.bind_keys(channel_chat_key_bindings());
}

fn channel_chat_key_bindings() -> [KeyBinding; 10] {
    [
        KeyBinding::new("ctrl-b", ToggleBold, Some("ChannelChat")),
        KeyBinding::new("ctrl-i", ToggleItalic, Some("ChannelChat")),
        KeyBinding::new("ctrl-`", ToggleCode, Some("ChannelChat")),
        KeyBinding::new("ctrl-shift-k", ToggleLink, Some("ChannelChat")),
        KeyBinding::new("ctrl-shift-p", TogglePreview, Some("ChannelChat")),
        KeyBinding::new("cmd-f", ToggleSearch, Some("ChannelChat")),
        KeyBinding::new("escape", CloseThread, Some("ChannelChat")),
        KeyBinding::new("up", SelectPrevious, Some("ChannelMessageSearch")),
        KeyBinding::new("down", SelectNext, Some("ChannelMessageSearch")),
        KeyBinding::new("enter", Confirm, Some("ChannelMessageSearch")),
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
    upload_manager: Entity<UploadManager>,
    search_editor: Entity<Editor>,
    search_state: search::SearchState,
    pending_search: Option<Task<()>>,
    highlighted_search_message_id: Option<u64>,
    emoji_search: Entity<Editor>,
    bookmark_store: Entity<ChannelBookmarkStore>,
    bookmarks_expanded: bool,
    bookmark_form: Option<BookmarkForm>,
    bookmark_action_error: Option<SharedString>,
    messages: Vec<proto::ChannelMessage>,
    message_bodies: HashMap<u64, message_bubble::MessageBody>,
    thread_summaries: HashMap<u64, ThreadSummary>,
    thread_panel: Option<ThreadPanel>,
    scheduled_messages_panel: Option<ScheduledMessagesPanel>,
    pending_scheduled_count: usize,
    emoji_picker: Option<EmojiPickerState>,
    recent_emoji_names: Vec<String>,
    send_state: SendState,
    schedule_picker: SchedulePicker,
    pending_draft_save: Option<Task<()>>,
    _rpc_subscriptions: Vec<client::Subscription>,
    _composer_subscription: GpuiSubscription,
    _search_subscription: GpuiSubscription,
    _emoji_search_subscription: GpuiSubscription,
    _bookmark_store_subscription: GpuiSubscription,
    _upload_manager_subscription: GpuiSubscription,
}

#[derive(Clone, PartialEq, Eq)]
enum SendState {
    Idle,
    Sending,
    Failed(SharedString),
}

enum BookmarkFormRequest {
    Add(AddBookmark),
    Update(UpdateBookmark),
}

struct ThreadPanel {
    channel_id: ChannelId,
    root_message_id: u64,
    root_message: Option<proto::ChannelMessage>,
    replies: Vec<proto::ChannelMessage>,
    replies_done: bool,
    loading_earlier_replies: bool,
    pending_reply_nonce: Option<u128>,
    compose_editor: Entity<Editor>,
    load_state: ThreadLoadState,
    send_state: SendState,
}

struct ScheduledMessagesPanel {
    channel_id: ChannelId,
    messages: Vec<ScheduledMessage>,
    load_state: ScheduledMessagesLoadState,
    editing_message_id: Option<ScheduledMessageId>,
    edit_body: Entity<Editor>,
    edit_schedule_picker: SchedulePicker,
    saving_message_id: Option<ScheduledMessageId>,
    cancelling_message_id: Option<ScheduledMessageId>,
    action_error: Option<SharedString>,
}

#[derive(Clone, PartialEq, Eq)]
enum ScheduledMessagesLoadState {
    Loading,
    Loaded,
    Failed(SharedString),
}

#[derive(Clone)]
struct SchedulePicker {
    selected_date: NaiveDate,
    selected_time: NaiveTime,
    visible_month: NaiveDate,
    timezone: String,
    popover_visible: bool,
    active: bool,
    validation_error: Option<SharedString>,
}

impl SchedulePicker {
    fn new() -> Self {
        let now = Local::now() + ChronoDuration::hours(1);
        let selected_date = now.date_naive();
        Self {
            selected_date,
            selected_time: NaiveTime::from_hms_opt(now.hour(), now.minute(), 0)
                .unwrap_or(NaiveTime::MIN),
            visible_month: month_start(selected_date),
            timezone: now.offset().to_string(),
            popover_visible: false,
            active: false,
            validation_error: None,
        }
    }

    fn from_scheduled_at(scheduled_at: DateTime<Utc>) -> Self {
        let mut picker = Self::new();
        picker.set_scheduled_at(scheduled_at);
        picker
    }

    fn set_scheduled_at(&mut self, scheduled_at: DateTime<Utc>) {
        let scheduled_at = scheduled_at.with_timezone(&Local);
        self.selected_date = scheduled_at.date_naive();
        self.selected_time = NaiveTime::from_hms_opt(scheduled_at.hour(), scheduled_at.minute(), 0)
            .unwrap_or(self.selected_time);
        self.visible_month = month_start(self.selected_date);
        self.timezone = scheduled_at.offset().to_string();
        self.active = true;
        self.validation_error = None;
    }

    fn scheduled_at_utc(&self) -> Option<DateTime<Utc>> {
        if !self.active {
            return None;
        }

        let local = self.selected_date.and_time(self.selected_time);
        match Local.from_local_datetime(&local) {
            LocalResult::Single(timestamp) => Some(timestamp.with_timezone(&Utc)),
            LocalResult::Ambiguous(earliest, _) => Some(earliest.with_timezone(&Utc)),
            LocalResult::None => None,
        }
    }

    fn validate(&self) -> Result<DateTime<Utc>, SharedString> {
        let scheduled_at = self
            .scheduled_at_utc()
            .ok_or_else(|| SharedString::from("Choose a valid local time"))?;
        let earliest = Utc::now() + ChronoDuration::minutes(1);
        if scheduled_at < earliest {
            return Err(SharedString::from(
                "Scheduled messages need at least 1 minute lead time",
            ));
        }
        let latest = Utc::now() + ChronoDuration::days(30);
        if scheduled_at > latest {
            return Err(SharedString::from(
                "Scheduled messages can be at most 30 days away",
            ));
        }
        Ok(scheduled_at)
    }

    fn select_date(&mut self, date: NaiveDate) {
        self.selected_date = date;
        self.visible_month = month_start(date);
        self.active = true;
        self.validation_error = None;
    }

    fn set_hour(&mut self, hour: u32) {
        self.selected_time = NaiveTime::from_hms_opt(hour, self.selected_time.minute(), 0)
            .unwrap_or(self.selected_time);
        self.active = true;
        self.validation_error = None;
    }

    fn set_minute(&mut self, minute: u32) {
        self.selected_time = NaiveTime::from_hms_opt(self.selected_time.hour(), minute, 0)
            .unwrap_or(self.selected_time);
        self.active = true;
        self.validation_error = None;
    }

    fn adjust_month(&mut self, delta: i32) {
        self.visible_month = add_months(self.visible_month, delta);
    }

    fn clear(&mut self) {
        self.active = false;
        self.popover_visible = false;
        self.validation_error = None;
    }
}

struct ThreadIndicator {
    message_id: u64,
    reply_count: u32,
    has_unread: bool,
    participants: Vec<Arc<client::User>>,
}

impl ThreadIndicator {
    fn render(self, cx: &mut Context<ChannelChat>) -> gpui::AnyElement {
        const FACEPILE_LIMIT: usize = 3;

        let reply_label = if self.reply_count == 1 {
            "1 reply".to_string()
        } else {
            format!("{} replies", self.reply_count)
        };
        let extra_count = self.participants.len().saturating_sub(FACEPILE_LIMIT);
        let faces = self
            .participants
            .iter()
            .take(FACEPILE_LIMIT)
            .map(|user| {
                Avatar::new(user.avatar_uri.clone())
                    .size(px(18.))
                    .into_any_element()
            })
            .collect::<SmallVec<[_; 2]>>();
        let message_id = self.message_id;

        h_flex()
            .id(("channel-thread-indicator", message_id))
            .gap_2()
            .items_center()
            .child(
                Button::new(
                    format!("channel-thread-indicator-button-{message_id}"),
                    reply_label,
                )
                .label_size(LabelSize::XSmall)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_thread(message_id, window, cx);
                })),
            )
            .when(!faces.is_empty(), |this| this.child(Facepile::new(faces)))
            .when(extra_count > 0, |this| {
                this.child(
                    Label::new(format!("+{extra_count}"))
                        .size(LabelSize::XSmall)
                        .color(Color::Muted),
                )
            })
            .when(self.has_unread, |this| {
                this.child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .bg(cx.theme().colors().text_accent),
                )
            })
            .into_any_element()
    }
}

#[derive(Clone, PartialEq, Eq)]
enum ThreadLoadState {
    Loading,
    Loaded,
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
        let search_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text(
                "Search messages... (in:, from:, before:, after:)",
                window,
                cx,
            );
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
        let _search_subscription =
            cx.subscribe(&search_editor, |this, _, event: &EditorEvent, cx| {
                if matches!(
                    event,
                    EditorEvent::Edited { .. } | EditorEvent::BufferEdited
                ) {
                    this.schedule_search(cx);
                }
            });
        let _emoji_search_subscription = cx.observe(&emoji_search, |_, _, cx| cx.notify());
        let bookmark_store = cx.new(|cx| ChannelBookmarkStore::new(client.clone(), cx));
        let _bookmark_store_subscription = cx.observe(&bookmark_store, |_, _, cx| cx.notify());
        let upload_manager = UploadManager::global(cx);
        let _upload_manager_subscription = cx.observe(&upload_manager, |_, _, cx| cx.notify());
        let weak_self = cx.weak_entity();
        let _rpc_subscriptions = vec![
            client.add_channel_message_sent_handler(weak_self.clone(), Self::handle_message_sent),
            client
                .add_channel_message_update_handler(weak_self.clone(), Self::handle_message_update),
            client.add_channel_message_reactions_update_handler(
                weak_self.clone(),
                Self::handle_message_reactions_update,
            ),
            client.add_scheduled_message_sent_handler(
                weak_self.clone(),
                Self::handle_scheduled_message_sent,
            ),
            client.add_scheduled_message_failed_handler(
                weak_self,
                Self::handle_scheduled_message_failed,
            ),
        ];
        let load_thread_summaries = cx.spawn({
            let client = client.clone();
            let channel_id = channel_id.0;
            async move |this, cx| {
                let summaries = client.get_threads(channel_id).await?;
                this.update(cx, |this, cx| {
                    this.set_thread_summaries(summaries);
                    cx.notify();
                })?;
                anyhow::Ok(())
            }
        });
        load_thread_summaries.detach_and_log_err(cx);
        let load_scheduled_count = cx.spawn({
            let client = client.clone();
            let channel_id = channel_id.0;
            async move |this, cx| {
                let messages = client.get_scheduled_messages(channel_id).await?;
                this.update(cx, |this, cx| {
                    this.pending_scheduled_count = messages.len();
                    cx.notify();
                })?;
                anyhow::Ok(())
            }
        });
        load_scheduled_count.detach_and_log_err(cx);
        let load_bookmarks = cx.spawn({
            let client = client.clone();
            let channel_id = channel_id;
            let bookmark_store = bookmark_store.clone();
            async move |_, cx| {
                let bookmarks = client.get_bookmarks(channel_id).await?;
                bookmark_store.update(cx, |bookmark_store, cx| {
                    bookmark_store.set_bookmarks(channel_id, bookmarks, cx);
                });
                anyhow::Ok(())
            }
        });
        load_bookmarks.detach_and_log_err(cx);
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
            upload_manager,
            search_editor,
            search_state: search::SearchState::default(),
            pending_search: None,
            highlighted_search_message_id: None,
            emoji_search,
            bookmark_store,
            bookmarks_expanded: false,
            bookmark_form: None,
            bookmark_action_error: None,
            messages,
            message_bodies: HashMap::default(),
            thread_summaries: HashMap::default(),
            thread_panel: None,
            scheduled_messages_panel: None,
            pending_scheduled_count: 0,
            emoji_picker: None,
            recent_emoji_names,
            send_state: SendState::Idle,
            schedule_picker: SchedulePicker::new(),
            pending_draft_save: None,
            _rpc_subscriptions,
            _composer_subscription,
            _search_subscription,
            _emoji_search_subscription,
            _bookmark_store_subscription,
            _upload_manager_subscription,
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

    async fn handle_scheduled_message_sent(
        this: Entity<Self>,
        message: TypedEnvelope<proto::ScheduledMessageSent>,
        mut cx: AsyncApp,
    ) -> Result<()> {
        this.update(&mut cx, |this, cx| {
            if message.payload.channel_id != this.channel_id.0 {
                return;
            }

            let channel_name = this
                .channel(cx)
                .map(|channel| channel.name.clone())
                .unwrap_or_else(|| SharedString::from("channel"));
            if let Some(message) = message.payload.message {
                if let Some(scheduled_at) = message.scheduled_at {
                    this.remove_matching_scheduled_message(scheduled_at, &message.body);
                }
                this.upsert_message(message.clone(), cx);
                this.workspace
                    .update(cx, |workspace, cx| {
                        workspace.show_toast(
                            Toast::new(
                                NotificationId::named(
                                    format!("scheduled-message-sent-{}", message.id).into(),
                                ),
                                format!("Your scheduled message was sent to #{channel_name}"),
                            )
                            .autohide(),
                            cx,
                        );
                    })
                    .log_err();
            }
            cx.notify();
        });
        Ok(())
    }

    async fn handle_scheduled_message_failed(
        this: Entity<Self>,
        message: TypedEnvelope<proto::ScheduledMessageFailed>,
        mut cx: AsyncApp,
    ) -> Result<()> {
        this.update(&mut cx, |this, cx| {
            if message.payload.channel_id != this.channel_id.0 {
                return;
            }

            let scheduled_message_id =
                ScheduledMessageId::from_proto(message.payload.scheduled_message_id);
            this.remove_scheduled_message(scheduled_message_id);
            let reason = message.payload.reason.clone();
            let channel_name = this
                .channel(cx)
                .map(|channel| channel.name.clone())
                .unwrap_or_else(|| SharedString::from("channel"));
            let chat = cx.weak_entity();
            let toast = Toast::new(
                NotificationId::named(
                    format!("scheduled-message-failed-{}", scheduled_message_id.0).into(),
                ),
                format!("Scheduled message failed in #{channel_name}: {reason}"),
            )
            .on_click("Review", move |window, cx| {
                chat.update(cx, |this, cx| {
                    this.show_scheduled_messages_panel(window, cx);
                })
                .log_err();
            });
            this.workspace
                .update(cx, |workspace, cx| workspace.show_toast(toast, cx))
                .log_err();
            cx.notify();
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
        let reply_to_message_id = message.reply_to_message_id;
        let thread_reply = message.clone();
        if let Some(existing) = self
            .messages
            .iter_mut()
            .find(|existing| existing.id == message.id)
        {
            *existing = message;
        } else {
            if let Some(root_message_id) = message.reply_to_message_id {
                self.apply_thread_reply_to_summary(root_message_id, &message);
            }
            self.messages.push(message);
            self.messages
                .sort_by_key(|message| (message.timestamp, message.id));
        }

        if let Some(root_message_id) = reply_to_message_id {
            self.upsert_open_thread_reply(root_message_id, thread_reply);
        }

        if let Some(latest_message_id) = self.messages.last().map(|message| message.id) {
            self.client
                .acknowledge_channel_message(self.channel_id.0, latest_message_id)
                .log_err();
        }

        cx.notify();
    }

    fn set_thread_summaries(&mut self, summaries: Vec<ThreadSummary>) {
        self.thread_summaries = summaries
            .into_iter()
            .map(|summary| (summary.root_message_id, summary))
            .collect();
    }

    fn apply_thread_reply_to_summary(
        &mut self,
        root_message_id: u64,
        reply: &proto::ChannelMessage,
    ) {
        let summary = self
            .thread_summaries
            .entry(root_message_id)
            .or_insert_with(|| ThreadSummary {
                root_message_id,
                reply_count: 0,
                latest_reply_at: reply.timestamp,
                participant_user_ids: Vec::new(),
                has_unread: false,
            });
        if summary.latest_reply_at < reply.timestamp {
            summary.latest_reply_at = reply.timestamp;
        }
        if !summary.participant_user_ids.contains(&reply.sender_id) {
            summary.participant_user_ids.push(reply.sender_id);
        }
        summary.reply_count = summary.reply_count.saturating_add(1);
    }

    fn upsert_open_thread_reply(&mut self, root_message_id: u64, reply: proto::ChannelMessage) {
        let Some(thread_panel) = self.thread_panel.as_mut() else {
            return;
        };
        if thread_panel.root_message_id != root_message_id {
            return;
        }

        if let Some(existing) = thread_panel
            .replies
            .iter_mut()
            .find(|existing| existing.id == reply.id)
        {
            *existing = reply;
        } else if let Some(existing) = thread_panel.replies.iter_mut().find(|existing| {
            thread_panel
                .pending_reply_nonce
                .is_some_and(|pending_reply_nonce| {
                    message_nonce(existing) == Some(pending_reply_nonce)
                })
                && existing.nonce == reply.nonce
        }) {
            *existing = reply;
            thread_panel.pending_reply_nonce = None;
        } else {
            thread_panel.replies.push(reply);
            thread_panel
                .replies
                .sort_by_key(|reply| (reply.timestamp, reply.id));
        }
    }

    fn send(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_selected_search_result(window, cx) {
            return;
        }

        if self.send_state == SendState::Sending {
            return;
        }

        let body = self.composer.read(cx).text(cx).trim().to_string();
        if body.is_empty() {
            return;
        }

        self.send_state = SendState::Sending;
        cx.notify();

        let scheduled_at = if self.schedule_picker.active {
            match self.schedule_picker.validate() {
                Ok(scheduled_at) => Some(scheduled_at),
                Err(message) => {
                    self.schedule_picker.validation_error = Some(message);
                    cx.notify();
                    return;
                }
            }
        } else {
            None
        };
        let client = self.client.clone();
        let channel_id = self.channel_id;
        cx.spawn_in(window, async move |this, cx| {
            let nonce = next_nonce(channel_id);
            let send_result = if let Some(scheduled_at) = scheduled_at {
                client
                    .schedule_channel_message(ScheduleChannelMessage {
                        channel_id: channel_id.0,
                        body,
                        scheduled_at,
                        nonce,
                        mentions: Vec::new(),
                    })
                    .await
                    .map(|_| None)
            } else {
                client
                    .send_channel_message(SendChannelMessage {
                        channel_id: channel_id.0,
                        body,
                        nonce,
                        mentions: Vec::new(),
                        reply_to_message_id: None,
                    })
                    .await
                    .map(Some)
            };

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
                    if scheduled_at.is_some() {
                        this.pending_scheduled_count =
                            this.pending_scheduled_count.saturating_add(1);
                    }
                    this.schedule_picker.clear();
                    if let Some(message) = message {
                        this.upsert_message(message, cx);
                    }
                }
                Err(error) => {
                    let message = SharedString::from(error.to_string());
                    this.send_state = SendState::Failed(message.clone());
                    let action = if scheduled_at.is_some() {
                        "schedule"
                    } else {
                        "send"
                    };
                    this.workspace
                        .update(cx, |workspace, cx| {
                            workspace
                                .show_error(format!("Failed to {action} message: {message}"), cx);
                        })
                        .log_err();
                    cx.notify();
                }
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn open_file_picker(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let paths_receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(paths_result) = paths_receiver.await else {
                return anyhow::Ok(());
            };
            let Some(paths) = paths_result? else {
                return anyhow::Ok(());
            };
            this.update(cx, |this, cx| {
                this.upload_paths(paths, cx);
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn upload_external_paths(
        &mut self,
        paths: &ExternalPaths,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.upload_paths(paths.paths().to_vec(), cx);
        cx.stop_propagation();
    }

    fn upload_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let file_paths = paths
            .into_iter()
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        if file_paths.is_empty() {
            self.show_upload_error("No files to upload", cx);
            return;
        }

        for file_path in file_paths {
            self.start_file_upload(file_path, cx);
        }
        cx.notify();
    }

    fn start_file_upload(&mut self, file_path: PathBuf, cx: &mut Context<Self>) {
        let filename = file_path
            .file_name()
            .and_then(|filename| filename.to_str())
            .unwrap_or("file")
            .to_string();
        let upload_task = self.upload_manager.update(cx, |upload_manager, cx| {
            upload_manager.upload_file(self.channel_id, file_path, cx)
        });

        cx.spawn(async move |this, cx| {
            if let Err(error) = upload_task.await {
                if error.to_string() == "file upload cancelled" {
                    return anyhow::Ok(());
                }
                this.update(cx, |this, cx| {
                    this.show_upload_error(format!("Failed to upload {filename}: {error}"), cx);
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn cancel_upload(&mut self, file_id: String, cx: &mut Context<Self>) {
        self.upload_manager.update(cx, |upload_manager, cx| {
            upload_manager.cancel_upload(&file_id, cx);
        });
    }

    fn retry_upload(&mut self, file_id: String, cx: &mut Context<Self>) {
        let file_path = self.upload_manager.update(cx, |upload_manager, cx| {
            let file_path = upload_manager
                .uploads_for_channel(self.channel_id)
                .into_iter()
                .find(|upload| upload.file_id == file_id)
                .map(|upload| upload.file_path);
            upload_manager.remove_upload(&file_id, cx);
            file_path
        });
        if let Some(file_path) = file_path {
            self.start_file_upload(file_path, cx);
        }
    }

    fn remove_upload(&mut self, file_id: String, cx: &mut Context<Self>) {
        self.upload_manager.update(cx, |upload_manager, cx| {
            upload_manager.remove_upload(&file_id, cx);
        });
    }

    fn show_upload_error(&self, message: impl Into<String>, cx: &mut App) {
        let message = message.into();
        self.workspace
            .update(cx, |workspace, cx| {
                workspace.show_error(message, cx);
            })
            .log_err();
    }

    fn toggle_schedule_picker(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.schedule_picker.popover_visible = !self.schedule_picker.popover_visible;
        cx.notify();
    }

    fn clear_schedule(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.schedule_picker.clear();
        cx.notify();
    }

    fn select_schedule_date(
        &mut self,
        date: NaiveDate,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.schedule_picker.select_date(date);
        cx.notify();
    }

    fn select_schedule_hour(
        &mut self,
        hour: u32,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.schedule_picker.set_hour(hour);
        cx.notify();
    }

    fn select_schedule_minute(
        &mut self,
        minute: u32,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.schedule_picker.set_minute(minute);
        cx.notify();
    }

    fn show_previous_schedule_month(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.schedule_picker.adjust_month(-1);
        cx.notify();
    }

    fn show_next_schedule_month(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.schedule_picker.adjust_month(1);
        cx.notify();
    }

    fn render_schedule_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_label = selected_schedule_label(&self.schedule_picker);

        v_flex()
            .gap_2()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().editor_background)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Icon::new(IconName::Clock).size(IconSize::Small))
                    .child(
                        Label::new(selected_label.unwrap_or_else(|| "Send now".to_string()))
                            .size(LabelSize::Small),
                    )
                    .child(
                        Label::new(format!("Local time ({})", self.schedule_picker.timezone))
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    ),
            )
            .child(self.render_schedule_calendar(cx))
            .child(self.render_schedule_time_picker(cx))
            .when_some(
                self.schedule_picker.validation_error.clone(),
                |this, message| {
                    this.child(
                        Label::new(message)
                            .size(LabelSize::XSmall)
                            .color(Color::Error),
                    )
                },
            )
            .when(self.schedule_picker.active, |this| {
                this.child(
                    Button::new("clear-schedule", "Clear schedule")
                        .style(ButtonStyle::Subtle)
                        .on_click(cx.listener(Self::clear_schedule)),
                )
            })
    }

    fn render_schedule_calendar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let month = self.schedule_picker.visible_month;
        let first_weekday = month.weekday().num_days_from_monday() as usize;
        let days_in_month = days_in_month(month);
        let mut day_cells = Vec::with_capacity(42);
        for index in 0..42 {
            let day_number = index as i32 - first_weekday as i32 + 1;
            let date = if day_number >= 1 && day_number <= days_in_month as i32 {
                month.with_day(day_number as u32)
            } else {
                None
            };
            day_cells.push(date);
        }

        v_flex()
            .gap_1()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        IconButton::new("schedule-previous-month", IconName::ChevronLeft)
                            .icon_size(IconSize::Small)
                            .on_click(cx.listener(Self::show_previous_schedule_month))
                            .tooltip(Tooltip::text("Previous month")),
                    )
                    .child(
                        Label::new(month.format("%B %Y").to_string())
                            .size(LabelSize::Small)
                            .color(Color::Default),
                    )
                    .child(
                        IconButton::new("schedule-next-month", IconName::ChevronRight)
                            .icon_size(IconSize::Small)
                            .on_click(cx.listener(Self::show_next_schedule_month))
                            .tooltip(Tooltip::text("Next month")),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(7)
                    .gap_1()
                    .children(
                        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map(|weekday| {
                            Label::new(weekday)
                                .size(LabelSize::XSmall)
                                .color(Color::Muted)
                                .into_any_element()
                        }),
                    )
                    .children(day_cells.into_iter().map(|date| {
                        if let Some(date) = date {
                            let selected = date == self.schedule_picker.selected_date;
                            Button::new(
                                format!("schedule-day-{}", date.format("%Y-%m-%d")),
                                date.day().to_string(),
                            )
                            .label_size(LabelSize::XSmall)
                            .style(if selected {
                                ButtonStyle::Tinted(TintColor::Accent)
                            } else {
                                ButtonStyle::Subtle
                            })
                            .on_click(cx.listener(move |this, event, window, cx| {
                                this.select_schedule_date(date, event, window, cx);
                            }))
                            .into_any_element()
                        } else {
                            div().h(px(26.)).into_any_element()
                        }
                    })),
            )
    }

    fn render_schedule_time_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_hour = self.schedule_picker.selected_time.hour();
        let selected_minute = self.schedule_picker.selected_time.minute();
        v_flex()
            .gap_1()
            .child(
                Label::new("Time")
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(h_flex().gap_1().flex_wrap().children((0..24).map(|hour| {
                        Button::new(("schedule-hour", hour), format!("{hour:02}"))
                            .label_size(LabelSize::XSmall)
                            .style(if selected_hour == hour {
                                ButtonStyle::Tinted(TintColor::Accent)
                            } else {
                                ButtonStyle::Subtle
                            })
                            .on_click(cx.listener(move |this, event, window, cx| {
                                this.select_schedule_hour(hour, event, window, cx);
                            }))
                            .into_any_element()
                    })))
                    .child(div().w(px(1.)).h(px(48.)).bg(cx.theme().colors().border))
                    .child(
                        h_flex()
                            .gap_1()
                            .flex_wrap()
                            .children((0..60).step_by(5).map(|minute| {
                                Button::new(("schedule-minute", minute), format!("{minute:02}"))
                                    .label_size(LabelSize::XSmall)
                                    .style(if selected_minute == minute {
                                        ButtonStyle::Tinted(TintColor::Accent)
                                    } else {
                                        ButtonStyle::Subtle
                                    })
                                    .on_click(cx.listener(move |this, event, window, cx| {
                                        this.select_schedule_minute(minute, event, window, cx);
                                    }))
                            })),
                    ),
            )
    }

    fn open_scheduled_messages_panel(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.scheduled_messages_panel.is_some() {
            self.scheduled_messages_panel = None;
            cx.notify();
            return;
        }

        self.show_scheduled_messages_panel(window, cx);
    }

    fn show_scheduled_messages_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let edit_body = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Message body", window, cx);
            editor
        });
        self.thread_panel = None;
        if self.scheduled_messages_panel.is_none() {
            self.scheduled_messages_panel = Some(ScheduledMessagesPanel {
                channel_id: self.channel_id,
                messages: Vec::new(),
                load_state: ScheduledMessagesLoadState::Loading,
                editing_message_id: None,
                edit_body,
                edit_schedule_picker: SchedulePicker::new(),
                saving_message_id: None,
                cancelling_message_id: None,
                action_error: None,
            });
        }
        self.refresh_scheduled_messages(window, cx);
    }

    fn close_scheduled_messages_panel(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.scheduled_messages_panel = None;
        cx.notify();
    }

    fn refresh_scheduled_messages(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.scheduled_messages_panel.as_mut() else {
            return;
        };
        panel.load_state = ScheduledMessagesLoadState::Loading;
        panel.action_error = None;
        let channel_id = panel.channel_id;
        cx.notify();

        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let messages = client.get_scheduled_messages(channel_id.0).await;
            this.update(cx, |this, cx| {
                let Some(panel) = this.scheduled_messages_panel.as_mut() else {
                    return;
                };
                if panel.channel_id != channel_id {
                    return;
                }

                match messages {
                    Ok(mut messages) => {
                        sort_scheduled_messages_for_display(&mut messages);
                        this.pending_scheduled_count = messages.len();
                        panel.messages = messages;
                        panel.load_state = ScheduledMessagesLoadState::Loaded;
                    }
                    Err(error) => {
                        panel.load_state = ScheduledMessagesLoadState::Failed(
                            format!("Failed to load scheduled messages: {error}").into(),
                        );
                    }
                }
                cx.notify();
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn start_edit_scheduled_message(
        &mut self,
        scheduled_message_id: ScheduledMessageId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(panel) = self.scheduled_messages_panel.as_mut() else {
            return;
        };
        let Some(message) = panel
            .messages
            .iter()
            .find(|message| message.id == scheduled_message_id)
            .cloned()
        else {
            return;
        };

        panel.editing_message_id = Some(scheduled_message_id);
        panel.edit_schedule_picker = SchedulePicker::from_scheduled_at(message.scheduled_at);
        panel.action_error = None;
        panel
            .edit_body
            .update(cx, |editor, cx| editor.set_text(message.body, window, cx));
        cx.notify();
    }

    fn cancel_edit_scheduled_message(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(panel) = self.scheduled_messages_panel.as_mut() {
            panel.editing_message_id = None;
            panel.action_error = None;
            cx.notify();
        }
    }

    fn select_edit_schedule_date(
        &mut self,
        date: NaiveDate,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(panel) = self.scheduled_messages_panel.as_mut() {
            panel.edit_schedule_picker.select_date(date);
            cx.notify();
        }
    }

    fn select_edit_schedule_hour(
        &mut self,
        hour: u32,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(panel) = self.scheduled_messages_panel.as_mut() {
            panel.edit_schedule_picker.set_hour(hour);
            cx.notify();
        }
    }

    fn select_edit_schedule_minute(
        &mut self,
        minute: u32,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(panel) = self.scheduled_messages_panel.as_mut() {
            panel.edit_schedule_picker.set_minute(minute);
            cx.notify();
        }
    }

    fn show_previous_edit_schedule_month(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(panel) = self.scheduled_messages_panel.as_mut() {
            panel.edit_schedule_picker.adjust_month(-1);
            cx.notify();
        }
    }

    fn show_next_edit_schedule_month(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(panel) = self.scheduled_messages_panel.as_mut() {
            panel.edit_schedule_picker.adjust_month(1);
            cx.notify();
        }
    }

    fn save_scheduled_message_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.scheduled_messages_panel.as_mut() else {
            return;
        };
        let Some(scheduled_message_id) = panel.editing_message_id else {
            return;
        };
        if panel.saving_message_id.is_some() {
            return;
        }

        let body = panel.edit_body.read(cx).text(cx).trim().to_string();
        if body.is_empty() {
            panel.action_error = Some(SharedString::from("Message body cannot be empty"));
            cx.notify();
            return;
        }
        let scheduled_at = match panel.edit_schedule_picker.validate() {
            Ok(scheduled_at) => scheduled_at,
            Err(message) => {
                panel.edit_schedule_picker.validation_error = Some(message);
                cx.notify();
                return;
            }
        };
        let Some(message) = panel
            .messages
            .iter()
            .find(|message| message.id == scheduled_message_id)
            .cloned()
        else {
            return;
        };

        panel.saving_message_id = Some(scheduled_message_id);
        panel.action_error = None;
        let channel_id = panel.channel_id;
        let client = self.client.clone();
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let result = client
                .update_scheduled_message(UpdateScheduledMessage {
                    scheduled_message_id,
                    channel_id: channel_id.0,
                    body: Some(body),
                    scheduled_at: Some(scheduled_at),
                    mentions: message.mentions,
                })
                .await;

            this.update_in(cx, |this, window, cx| {
                let Some(panel) = this.scheduled_messages_panel.as_mut() else {
                    return;
                };
                if panel.channel_id != channel_id {
                    return;
                }

                panel.saving_message_id = None;
                match result {
                    Ok(()) => {
                        panel.editing_message_id = None;
                        this.refresh_scheduled_messages(window, cx);
                    }
                    Err(error) => {
                        panel.action_error =
                            Some(format!("Failed to update scheduled message: {error}").into());
                        cx.notify();
                    }
                }
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn confirm_cancel_scheduled_message(
        &mut self,
        scheduled_message_id: ScheduledMessageId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(panel) = self.scheduled_messages_panel.as_ref() else {
            return;
        };
        let channel_id = panel.channel_id;
        let answer = window.prompt(
            PromptLevel::Warning,
            "Cancel scheduled message?",
            Some("This removes the pending message before it is sent."),
            &["Cancel message", "Keep"],
            cx,
        );
        let client = self.client.clone();

        cx.spawn_in(window, async move |this, cx| {
            if answer.await? != 0 {
                return anyhow::Ok(());
            }

            this.update(cx, |this, cx| {
                if let Some(panel) = this.scheduled_messages_panel.as_mut()
                    && panel.channel_id == channel_id
                {
                    panel.cancelling_message_id = Some(scheduled_message_id);
                    panel.action_error = None;
                    cx.notify();
                }
            })?;

            let result = client
                .cancel_scheduled_message(channel_id.0, scheduled_message_id)
                .await;

            this.update(cx, |this, cx| {
                let should_remove = {
                    let Some(panel) = this.scheduled_messages_panel.as_mut() else {
                        return;
                    };
                    if panel.channel_id != channel_id {
                        return;
                    }

                    panel.cancelling_message_id = None;
                    match result {
                        Ok(()) => true,
                        Err(error) => {
                            panel.action_error =
                                Some(format!("Failed to cancel scheduled message: {error}").into());
                            false
                        }
                    }
                };
                if should_remove {
                    this.remove_scheduled_message(scheduled_message_id);
                }
                cx.notify();
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn remove_scheduled_message(&mut self, scheduled_message_id: ScheduledMessageId) {
        if let Some(panel) = self.scheduled_messages_panel.as_mut() {
            let previous_len = panel.messages.len();
            panel
                .messages
                .retain(|message| message.id != scheduled_message_id);
            if panel.messages.len() != previous_len {
                self.pending_scheduled_count = self.pending_scheduled_count.saturating_sub(1);
            }
            if panel.editing_message_id == Some(scheduled_message_id) {
                panel.editing_message_id = None;
            }
        } else {
            self.pending_scheduled_count = self.pending_scheduled_count.saturating_sub(1);
        }
    }

    fn remove_matching_scheduled_message(&mut self, scheduled_at: u64, body: &str) {
        if let Some(panel) = self.scheduled_messages_panel.as_mut() {
            let previous_len = panel.messages.len();
            panel.messages.retain(|message| {
                let message_scheduled_at = u64::try_from(message.scheduled_at.timestamp_millis());
                !matches!(message_scheduled_at, Ok(message_scheduled_at) if message_scheduled_at == scheduled_at && message.body == body)
            });
            if panel.messages.len() != previous_len {
                self.pending_scheduled_count = self.pending_scheduled_count.saturating_sub(1);
            }
        } else {
            self.pending_scheduled_count = self.pending_scheduled_count.saturating_sub(1);
        }
    }

    fn open_thread(&mut self, root_message_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.scheduled_messages_panel = None;
        let compose_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Reply in thread", window, cx);
            editor
        });
        let existing_root = self
            .messages
            .iter()
            .find(|message| message.id == root_message_id)
            .cloned();
        self.thread_panel = Some(ThreadPanel {
            channel_id: self.channel_id,
            root_message_id,
            root_message: existing_root,
            replies: Vec::new(),
            replies_done: true,
            loading_earlier_replies: false,
            pending_reply_nonce: None,
            compose_editor,
            load_state: ThreadLoadState::Loading,
            send_state: SendState::Idle,
        });
        cx.notify();

        let client = self.client.clone();
        let channel_id = self.channel_id;
        cx.spawn_in(window, async move |this, cx| {
            let mut retries = 0;
            let thread_result = loop {
                match client.get_thread(channel_id.0, root_message_id).await {
                    Ok(thread) => break Ok(thread),
                    Err(_) if retries < THREAD_LOAD_RETRY_DELAYS.len() => {
                        let delay = THREAD_LOAD_RETRY_DELAYS[retries];
                        retries += 1;
                        cx.background_executor().timer(delay).await;
                    }
                    Err(error) => break Err(error),
                }
            };
            this.update(cx, |this, cx| {
                let Some(thread_panel) = this.thread_panel.as_mut() else {
                    return;
                };
                if thread_panel.root_message_id != root_message_id {
                    return;
                }

                match thread_result {
                    Ok(thread) => {
                        let pending_reply = thread_panel.pending_reply_nonce.and_then(|nonce| {
                            thread_panel
                                .replies
                                .iter()
                                .find(|reply| message_nonce(reply) == Some(nonce))
                                .cloned()
                        });
                        thread_panel.root_message = Some(thread.root_message);
                        thread_panel.replies = thread.replies;
                        if let Some(pending_reply) = pending_reply {
                            if thread_panel
                                .replies
                                .iter()
                                .any(|reply| reply.nonce == pending_reply.nonce)
                            {
                                thread_panel.pending_reply_nonce = None;
                            } else {
                                thread_panel.replies.push(pending_reply);
                                thread_panel
                                    .replies
                                    .sort_by_key(|reply| (reply.timestamp, reply.id));
                            }
                        } else {
                            thread_panel.pending_reply_nonce = None;
                        }
                        thread_panel.load_state = ThreadLoadState::Loaded;
                        thread_panel.replies_done = thread.done;
                        this.mark_thread_read(root_message_id);
                    }
                    Err(error) => {
                        thread_panel.load_state = ThreadLoadState::Failed(
                            format!(
                                "Failed to load thread after {} attempts: {error}",
                                THREAD_LOAD_RETRY_DELAYS.len() + 1
                            )
                            .into(),
                        );
                    }
                }
                cx.notify();
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn close_thread(&mut self, _: &CloseThread, _: &mut Window, cx: &mut Context<Self>) {
        if self.close_search(cx) {
            return;
        }
        if self.scheduled_messages_panel.take().is_some() {
            cx.notify();
            return;
        }
        if self.thread_panel.take().is_some() {
            cx.notify();
        }
    }

    fn mark_thread_read(&mut self, root_message_id: u64) {
        let Some(thread_panel) = self.thread_panel.as_ref() else {
            return;
        };
        if thread_panel.root_message_id != root_message_id {
            return;
        }
        let Some(latest_reply_id) = thread_panel.replies.last().map(|reply| reply.id) else {
            return;
        };

        if let Some(summary) = self.thread_summaries.get_mut(&root_message_id) {
            summary.has_unread = false;
        }
        self.client
            .acknowledge_channel_thread(self.channel_id.0, root_message_id, latest_reply_id)
            .log_err();
    }

    fn load_earlier_thread_replies(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(thread_panel) = self.thread_panel.as_mut() else {
            return;
        };
        if thread_panel.replies_done || thread_panel.loading_earlier_replies {
            return;
        }
        let Some(before_message_id) = thread_panel.replies.first().map(|reply| reply.id) else {
            return;
        };

        thread_panel.loading_earlier_replies = true;
        let channel_id = thread_panel.channel_id;
        let root_message_id = thread_panel.root_message_id;
        cx.notify();

        let client = self.client.clone();
        let workspace = self.workspace.clone();
        cx.spawn_in(window, async move |this, cx| {
            let thread_result = client
                .get_thread_page(
                    channel_id.0,
                    root_message_id,
                    Some(before_message_id),
                    DEFAULT_THREAD_REPLY_LIMIT,
                )
                .await;

            this.update(cx, |this, cx| {
                let Some(thread_panel) = this.thread_panel.as_mut() else {
                    return;
                };
                if thread_panel.root_message_id != root_message_id {
                    return;
                }

                thread_panel.loading_earlier_replies = false;
                match thread_result {
                    Ok(thread) => {
                        thread_panel.root_message = Some(thread.root_message);
                        let existing_reply_ids = thread_panel
                            .replies
                            .iter()
                            .map(|reply| reply.id)
                            .collect::<std::collections::HashSet<_>>();
                        let earlier_replies = thread
                            .replies
                            .into_iter()
                            .filter(|reply| !existing_reply_ids.contains(&reply.id));
                        thread_panel.replies.splice(0..0, earlier_replies);
                        thread_panel.replies_done = thread.done;
                    }
                    Err(error) => {
                        workspace
                            .update(cx, |workspace, cx| {
                                workspace.show_error(
                                    format!("Failed to load earlier thread replies: {error}"),
                                    cx,
                                );
                            })
                            .log_err();
                    }
                }
                cx.notify();
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn send_thread_reply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(thread_panel) = self.thread_panel.as_mut() else {
            return;
        };
        if thread_panel.send_state == SendState::Sending {
            return;
        }

        let body = thread_panel
            .compose_editor
            .read(cx)
            .text(cx)
            .trim()
            .to_string();
        if body.is_empty() {
            return;
        }

        let root_message_id = thread_panel.root_message_id;
        let channel_id = thread_panel.channel_id;
        let nonce = next_nonce(channel_id);
        let Some(sender_id) = self
            .user_store
            .read(cx)
            .current_user()
            .map(|user| user.legacy_id)
        else {
            thread_panel.send_state =
                SendState::Failed(SharedString::from("Current user unavailable"));
            cx.notify();
            return;
        };

        let optimistic_reply = proto::ChannelMessage {
            id: 0,
            body: body.clone(),
            timestamp: current_unix_timestamp(),
            sender_id,
            nonce: Some(nonce.into()),
            mentions: Vec::new(),
            reply_to_message_id: Some(root_message_id),
            edited_at: None,
            reaction_summaries: Vec::new(),
            scheduled_at: None,
            files: Vec::new(),
        };

        thread_panel.send_state = SendState::Sending;
        thread_panel.pending_reply_nonce = Some(nonce);
        thread_panel.replies.push(optimistic_reply);
        let compose_editor = thread_panel.compose_editor.clone();
        compose_editor.update(cx, |editor, cx| editor.clear(window, cx));
        cx.notify();

        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let send_result = client
                .send_channel_message(SendChannelMessage {
                    channel_id: channel_id.0,
                    body: body.clone(),
                    nonce,
                    mentions: Vec::new(),
                    reply_to_message_id: Some(root_message_id),
                })
                .await;

            this.update_in(cx, |this, window, cx| match send_result {
                Ok(message) => {
                    this.upsert_message(message.clone(), cx);
                    if let Some(thread_panel) = this.thread_panel.as_mut()
                        && thread_panel.root_message_id == root_message_id
                    {
                        thread_panel.send_state = SendState::Idle;
                    }
                    cx.notify();
                }
                Err(error) => {
                    let message = SharedString::from(error.to_string());
                    if let Some(thread_panel) = this.thread_panel.as_mut()
                        && thread_panel.root_message_id == root_message_id
                    {
                        thread_panel.send_state = SendState::Failed(message.clone());
                        thread_panel
                            .replies
                            .retain(|reply| message_nonce(reply) != Some(nonce));
                        thread_panel.pending_reply_nonce = None;
                        compose_editor.update(cx, |editor, cx| editor.set_text(body, window, cx));
                    }
                    this.workspace
                        .update(cx, |workspace, cx| {
                            workspace
                                .show_error(format!("Failed to send thread reply: {message}"), cx);
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
        if is_bookmark_system_message(&message.body) {
            return h_flex()
                .gap_2()
                .items_center()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(cx.theme().colors().border_variant)
                .child(Icon::new(IconName::Pin).size(IconSize::XSmall))
                .child(
                    Label::new(message.body.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Muted)
                        .italic(),
                )
                .into_any_element();
        }

        let sender = self.user_display_name(message.sender_id, cx);
        let timestamp = format_timestamp(message.timestamp);
        let scheduled_label = message
            .scheduled_at
            .and_then(format_scheduled_message_label);
        let edited = message.edited_at.is_some();

        v_flex()
            .gap_1()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().colors().border_variant)
            .when(
                self.highlighted_search_message_id == Some(message.id),
                |this| this.bg(cx.theme().colors().element_selected),
            )
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
                    .when_some(scheduled_label, |this, scheduled_label| {
                        this.child(
                            h_flex()
                                .id(format!("scheduled-message-label-{}", message.id))
                                .gap_1()
                                .items_center()
                                .child(Icon::new(IconName::Clock).size(IconSize::XSmall))
                                .child(
                                    Label::new(scheduled_label)
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted),
                                )
                                .tooltip(Tooltip::text("Scheduled message")),
                        )
                    })
                    .when(edited, |this| {
                        this.child(
                            Label::new("edited")
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        )
                    }),
            )
            .child(self.rendered_message_body(message, cx).render(window, cx))
            .child(
                h_flex().child(
                    Button::new(format!("channel-open-thread-{}", message.id), "Reply")
                        .label_size(LabelSize::XSmall)
                        .on_click(cx.listener({
                            let message_id = message.id;
                            move |this, _, window, cx| {
                                this.open_thread(message_id, window, cx);
                            }
                        })),
                ),
            )
            .child(self.render_thread_indicator(message.id, cx))
            .child(self.render_reactions(message, cx))
            .into_any_element()
    }

    fn render_thread_indicator(&self, message_id: u64, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(summary) = self.thread_summaries.get(&message_id) else {
            return div().into_any_element();
        };
        if summary.reply_count == 0 {
            return div().into_any_element();
        }

        let participants = self.user_store.update(cx, |user_store, cx| {
            summary
                .participant_user_ids
                .iter()
                .filter_map(|user_id| user_store.get_user_optimistic(*user_id, cx))
                .collect()
        });
        ThreadIndicator {
            message_id,
            reply_count: summary.reply_count,
            has_unread: summary.has_unread,
            participants,
        }
        .render(cx)
    }

    fn render_thread_message(
        &mut self,
        message: &proto::ChannelMessage,
        is_pending: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let sender = self.user_display_name(message.sender_id, cx);
        let timestamp = format_timestamp(message.timestamp);
        let edited = message.edited_at.is_some();

        v_flex()
            .gap_1()
            .p_2()
            .rounded_md()
            .bg(cx.theme().colors().editor_background)
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
                    })
                    .when(is_pending, |this| {
                        this.child(
                            Label::new("Sending...")
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        )
                    }),
            )
            .child(self.rendered_message_body(message, cx).render(window, cx))
            .into_any_element()
    }

    fn render_scheduled_messages_panel(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(panel) = self.scheduled_messages_panel.as_ref() else {
            return div().into_any_element();
        };

        let channel_name = self
            .channel(cx)
            .map(|channel| channel.name.clone())
            .unwrap_or_else(|| SharedString::from("Channel"));
        let load_state = panel.load_state.clone();
        let messages = panel.messages.clone();
        let editing_message_id = panel.editing_message_id;
        let edit_body = panel.edit_body.clone();
        let saving_message_id = panel.saving_message_id;
        let cancelling_message_id = panel.cancelling_message_id;
        let action_error = panel.action_error.clone();

        v_flex()
            .id("scheduled-messages-panel")
            .w(px(380.))
            .min_w(px(300.))
            .h_full()
            .border_l_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().editor_background)
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .p_3()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(Icon::new(IconName::Clock).size(IconSize::Small))
                                    .child(
                                        Label::new("Scheduled")
                                            .size(LabelSize::Small)
                                            .weight(gpui::FontWeight::MEDIUM),
                                    ),
                            )
                            .child(
                                Label::new(channel_name)
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("refresh-scheduled-messages", "Refresh")
                                    .label_size(LabelSize::XSmall)
                                    .disabled(matches!(
                                        load_state,
                                        ScheduledMessagesLoadState::Loading
                                    ))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.refresh_scheduled_messages(window, cx);
                                    })),
                            )
                            .child(
                                IconButton::new("close-scheduled-messages", IconName::Close)
                                    .icon_size(IconSize::Small)
                                    .tooltip(Tooltip::text("Close scheduled messages"))
                                    .on_click(cx.listener(Self::close_scheduled_messages_panel)),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .id("scheduled-messages-list")
                    .gap_3()
                    .p_3()
                    .overflow_y_scroll()
                    .when(matches!(load_state, ScheduledMessagesLoadState::Loading), |this| {
                        this.child(LoadingLabel::new("Loading scheduled messages").color(Color::Muted))
                    })
                    .when_some(
                        match &load_state {
                            ScheduledMessagesLoadState::Failed(message) => Some(message.clone()),
                            ScheduledMessagesLoadState::Loading
                            | ScheduledMessagesLoadState::Loaded => None,
                        },
                        |this, message| {
                            this.child(
                                Label::new(message)
                                    .size(LabelSize::Small)
                                    .color(Color::Error),
                            )
                        },
                    )
                    .when_some(action_error, |this, message| {
                        this.child(
                            Label::new(message)
                                .size(LabelSize::XSmall)
                                .color(Color::Error),
                        )
                    })
                    .when(
                        matches!(load_state, ScheduledMessagesLoadState::Loaded)
                            && messages.is_empty(),
                        |this| {
                            this.child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        Label::new("No scheduled messages")
                                            .size(LabelSize::Small)
                                            .color(Color::Muted),
                                    )
                                    .child(
                                        Label::new("Messages you schedule from the composer will appear here.")
                                            .size(LabelSize::XSmall)
                                            .color(Color::Muted),
                                    ),
                            )
                        },
                    )
                    .children(self.render_scheduled_message_rows(
                        messages,
                        editing_message_id,
                        edit_body,
                        saving_message_id,
                        cancelling_message_id,
                        cx,
                    )),
            )
            .into_any_element()
    }

    fn render_scheduled_message_rows(
        &mut self,
        messages: Vec<ScheduledMessage>,
        editing_message_id: Option<ScheduledMessageId>,
        edit_body: Entity<Editor>,
        saving_message_id: Option<ScheduledMessageId>,
        cancelling_message_id: Option<ScheduledMessageId>,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let mut rows = Vec::new();
        let mut last_date = None;
        for message in messages {
            let message_date = message.display_time.date_naive();
            if last_date != Some(message_date) {
                last_date = Some(message_date);
                rows.push(
                    Label::new(message_date.format("%A, %b %-d").to_string())
                        .size(LabelSize::XSmall)
                        .color(Color::Muted)
                        .into_any_element(),
                );
            }

            let is_editing = editing_message_id == Some(message.id);
            rows.push(
                v_flex()
                    .gap_2()
                    .p_3()
                    .border_1()
                    .border_color(cx.theme().colors().border)
                    .rounded_md()
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_2()
                            .child(
                                Label::new(scheduled_message_time_label(&message))
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .when(!is_editing, |this| {
                                        let message_id = message.id;
                                        this.child(
                                            Button::new(
                                                format!("edit-scheduled-message-{}", message_id.0),
                                                "Edit",
                                            )
                                            .label_size(LabelSize::XSmall)
                                            .on_click(
                                                cx.listener(move |this, _, window, cx| {
                                                    this.start_edit_scheduled_message(
                                                        message_id, window, cx,
                                                    );
                                                }),
                                            ),
                                        )
                                    })
                                    .child(
                                        Button::new(
                                            format!("cancel-scheduled-message-{}", message.id.0),
                                            "Cancel",
                                        )
                                        .label_size(LabelSize::XSmall)
                                        .disabled(cancelling_message_id == Some(message.id))
                                        .loading(cancelling_message_id == Some(message.id))
                                        .on_click(
                                            cx.listener({
                                                let message_id = message.id;
                                                move |this, _, window, cx| {
                                                    this.confirm_cancel_scheduled_message(
                                                        message_id, window, cx,
                                                    );
                                                }
                                            }),
                                        ),
                                    ),
                            ),
                    )
                    .when(!is_editing, |this| {
                        this.child(
                            Label::new(message.body.clone())
                                .size(LabelSize::Small)
                                .color(Color::Default),
                        )
                    })
                    .when(is_editing, |this| {
                        this.child(
                            v_flex()
                                .gap_2()
                                .child(edit_body.clone())
                                .child(self.render_edit_schedule_calendar(cx))
                                .child(self.render_edit_schedule_time_picker(cx))
                                .when_some(
                                    self.scheduled_messages_panel.as_ref().and_then(|panel| {
                                        panel.edit_schedule_picker.validation_error.clone()
                                    }),
                                    |this, message| {
                                        this.child(
                                            Label::new(message)
                                                .size(LabelSize::XSmall)
                                                .color(Color::Error),
                                        )
                                    },
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .justify_end()
                                        .child(
                                            Button::new("cancel-edit-scheduled-message", "Done")
                                                .style(ButtonStyle::Subtle)
                                                .label_size(LabelSize::XSmall)
                                                .on_click(
                                                    cx.listener(
                                                        Self::cancel_edit_scheduled_message,
                                                    ),
                                                ),
                                        )
                                        .child(
                                            Button::new("save-scheduled-message", "Save")
                                                .label_size(LabelSize::XSmall)
                                                .disabled(saving_message_id == Some(message.id))
                                                .loading(saving_message_id == Some(message.id))
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.save_scheduled_message_edit(window, cx);
                                                })),
                                        ),
                                ),
                        )
                    })
                    .into_any_element(),
            );
        }
        rows
    }

    fn render_edit_schedule_calendar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(panel) = self.scheduled_messages_panel.as_ref() else {
            return v_flex().into_any_element();
        };
        let month = panel.edit_schedule_picker.visible_month;
        let selected_date = panel.edit_schedule_picker.selected_date;
        let first_weekday = month.weekday().num_days_from_monday() as usize;
        let days_in_month = days_in_month(month);
        let mut day_cells = Vec::with_capacity(42);
        for index in 0..42 {
            let day_number = index as i32 - first_weekday as i32 + 1;
            let date = if day_number >= 1 && day_number <= days_in_month as i32 {
                month.with_day(day_number as u32)
            } else {
                None
            };
            day_cells.push(date);
        }

        v_flex()
            .gap_1()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        IconButton::new("edit-schedule-previous-month", IconName::ChevronLeft)
                            .icon_size(IconSize::Small)
                            .on_click(cx.listener(Self::show_previous_edit_schedule_month))
                            .tooltip(Tooltip::text("Previous month")),
                    )
                    .child(
                        Label::new(month.format("%B %Y").to_string())
                            .size(LabelSize::Small)
                            .color(Color::Default),
                    )
                    .child(
                        IconButton::new("edit-schedule-next-month", IconName::ChevronRight)
                            .icon_size(IconSize::Small)
                            .on_click(cx.listener(Self::show_next_edit_schedule_month))
                            .tooltip(Tooltip::text("Next month")),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(7)
                    .gap_1()
                    .children(
                        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map(|weekday| {
                            Label::new(weekday)
                                .size(LabelSize::XSmall)
                                .color(Color::Muted)
                                .into_any_element()
                        }),
                    )
                    .children(day_cells.into_iter().map(|date| {
                        if let Some(date) = date {
                            Button::new(
                                format!("edit-schedule-day-{}", date.format("%Y-%m-%d")),
                                date.day().to_string(),
                            )
                            .label_size(LabelSize::XSmall)
                            .style(if date == selected_date {
                                ButtonStyle::Tinted(TintColor::Accent)
                            } else {
                                ButtonStyle::Subtle
                            })
                            .on_click(cx.listener(move |this, event, window, cx| {
                                this.select_edit_schedule_date(date, event, window, cx);
                            }))
                            .into_any_element()
                        } else {
                            div().h(px(26.)).into_any_element()
                        }
                    })),
            )
            .into_any_element()
    }

    fn render_edit_schedule_time_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(panel) = self.scheduled_messages_panel.as_ref() else {
            return v_flex().into_any_element();
        };
        let selected_hour = panel.edit_schedule_picker.selected_time.hour();
        let selected_minute = panel.edit_schedule_picker.selected_time.minute();
        v_flex()
            .gap_1()
            .child(
                Label::new("Time")
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(h_flex().gap_1().flex_wrap().children((0..24).map(|hour| {
                        Button::new(("edit-schedule-hour", hour), format!("{hour:02}"))
                            .label_size(LabelSize::XSmall)
                            .style(if selected_hour == hour {
                                ButtonStyle::Tinted(TintColor::Accent)
                            } else {
                                ButtonStyle::Subtle
                            })
                            .on_click(cx.listener(move |this, event, window, cx| {
                                this.select_edit_schedule_hour(hour, event, window, cx);
                            }))
                            .into_any_element()
                    })))
                    .child(div().w(px(1.)).h(px(48.)).bg(cx.theme().colors().border))
                    .child(
                        h_flex()
                            .gap_1()
                            .flex_wrap()
                            .children((0..60).step_by(5).map(|minute| {
                                Button::new(
                                    ("edit-schedule-minute", minute),
                                    format!("{minute:02}"),
                                )
                                .label_size(LabelSize::XSmall)
                                .style(if selected_minute == minute {
                                    ButtonStyle::Tinted(TintColor::Accent)
                                } else {
                                    ButtonStyle::Subtle
                                })
                                .on_click(cx.listener(
                                    move |this, event, window, cx| {
                                        this.select_edit_schedule_minute(minute, event, window, cx);
                                    },
                                ))
                            })),
                    ),
            )
            .into_any_element()
    }

    fn render_thread_panel(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(thread_panel) = self.thread_panel.as_ref() else {
            return div().into_any_element();
        };

        let root_message_id = thread_panel.root_message_id;
        let root_message = thread_panel.root_message.clone();
        let root_message_is_missing = thread_panel.root_message.is_none();
        let root_message_is_deleted = root_message
            .as_ref()
            .is_some_and(|root_message| root_message.body.is_empty());
        let replies = thread_panel.replies.clone();
        let replies_done = thread_panel.replies_done;
        let loading_earlier_replies = thread_panel.loading_earlier_replies;
        let pending_reply_nonce = thread_panel.pending_reply_nonce;
        let compose_editor = thread_panel.compose_editor.clone();
        let load_state = thread_panel.load_state.clone();
        let send_state = thread_panel.send_state.clone();
        let channel_name = self
            .channel(cx)
            .map(|channel| channel.name.clone())
            .unwrap_or_else(|| SharedString::from("Channel"));

        v_flex()
            .id("channel-thread-panel")
            .w(px(360.))
            .min_w(px(280.))
            .h_full()
            .border_l_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().editor_background)
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .p_3()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(
                                Label::new("Thread")
                                    .size(LabelSize::Small)
                                    .weight(gpui::FontWeight::MEDIUM),
                            )
                            .child(
                                Label::new(channel_name)
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted),
                            ),
                    )
                    .child(
                        IconButton::new("close-channel-thread", IconName::Close)
                            .icon_size(IconSize::Small)
                            .tooltip(Tooltip::text("Close thread"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_thread(&CloseThread, window, cx);
                            })),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .id("channel-thread-replies")
                    .gap_3()
                    .p_3()
                    .overflow_y_scroll()
                    .when_some(
                        root_message.filter(|root_message| !root_message.body.is_empty()),
                        |this, root_message| {
                            this.child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        Label::new("Original message")
                                            .size(LabelSize::XSmall)
                                            .color(Color::Muted),
                                    )
                                    .child(self.render_thread_message(
                                        &root_message,
                                        false,
                                        window,
                                        cx,
                                    )),
                            )
                        },
                    )
                    .when(
                        matches!(load_state, ThreadLoadState::Loaded)
                            && (root_message_is_missing || root_message_is_deleted),
                        |this| {
                            this.child(
                                Label::new("This message has been deleted")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            )
                        },
                    )
                    .when(matches!(load_state, ThreadLoadState::Loading), |this| {
                        this.child(LoadingLabel::new("Loading replies").color(Color::Muted))
                    })
                    .when_some(
                        match &load_state {
                            ThreadLoadState::Failed(message) => Some(message.clone()),
                            ThreadLoadState::Loading | ThreadLoadState::Loaded => None,
                        },
                        |this, message| {
                            this.child(
                                Label::new(message)
                                    .size(LabelSize::Small)
                                    .color(Color::Error),
                            )
                        },
                    )
                    .when(
                        matches!(load_state, ThreadLoadState::Loaded) && replies.is_empty(),
                        |this| {
                            this.child(
                                Label::new("No replies yet")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            )
                        },
                    )
                    .when(
                        matches!(load_state, ThreadLoadState::Loaded)
                            && !replies_done
                            && !replies.is_empty(),
                        |this| {
                            this.child(
                                Button::new("load-earlier-thread-replies", "Load earlier replies")
                                    .label_size(LabelSize::XSmall)
                                    .disabled(loading_earlier_replies)
                                    .loading(loading_earlier_replies)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.load_earlier_thread_replies(window, cx);
                                    })),
                            )
                        },
                    )
                    .children(replies.into_iter().map(|reply| {
                        let is_pending = pending_reply_nonce.is_some_and(|pending_reply_nonce| {
                            message_nonce(&reply) == Some(pending_reply_nonce)
                        });
                        self.render_thread_message(&reply, is_pending, window, cx)
                    })),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .border_t_1()
                    .border_color(cx.theme().colors().border)
                    .child(compose_editor.clone())
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                Label::new(format!("Replying to #{root_message_id}"))
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted),
                            )
                            .child(
                                Button::new("send-thread-reply", "Send")
                                    .label_size(LabelSize::XSmall)
                                    .disabled(matches!(send_state, SendState::Sending))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.send_thread_reply(window, cx);
                                    })),
                            ),
                    )
                    .when_some(
                        match &send_state {
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

    fn render_uploads(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let uploads = self
            .upload_manager
            .read(cx)
            .uploads_for_channel(self.channel_id);
        v_flex()
            .gap_1()
            .children(
                uploads
                    .into_iter()
                    .map(|upload| self.render_upload(upload, cx)),
            )
            .into_any_element()
    }

    fn render_upload(&self, upload: UploadProgress, cx: &mut Context<Self>) -> gpui::AnyElement {
        let progress = upload_progress_value(&upload.status, upload.progress);
        let status_text = upload_status_text(&upload.status, progress);
        let file_id = upload.file_id.clone();
        let upload_element_id = format!("channel-file-upload-{}", upload.file_id);
        let progress_element_id = format!("channel-file-upload-progress-{}", upload.file_id);
        let progress_color = match &upload.status {
            UploadStatus::Failed(_) => cx.theme().status().error,
            UploadStatus::Cancelled => cx.theme().colors().text_muted,
            UploadStatus::Completed => cx.theme().status().success,
            UploadStatus::Uploading | UploadStatus::Confirming => cx.theme().status().info,
        };

        h_flex()
            .id(upload_element_id)
            .gap_2()
            .items_center()
            .px_2()
            .py_1()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border)
            .child(
                Icon::new(IconName::File)
                    .size(IconSize::Small)
                    .color(Color::Muted),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_between()
                            .child(Label::new(upload.filename.clone()).truncate())
                            .child(
                                Label::new(status_text)
                                    .size(LabelSize::XSmall)
                                    .color(upload_status_color(&upload.status)),
                            ),
                    )
                    .child(
                        ProgressBar::new(progress_element_id, progress, 1.0, cx)
                            .fg_color(progress_color),
                    )
                    .when_some(upload_error_text(&upload.status), |this, error| {
                        this.child(
                            Label::new(error)
                                .size(LabelSize::XSmall)
                                .color(Color::Error)
                                .truncate(),
                        )
                    }),
            )
            .when(
                matches!(
                    upload.status,
                    UploadStatus::Uploading | UploadStatus::Confirming
                ),
                |this| {
                    this.child(
                        IconButton::new(
                            format!("cancel-channel-file-upload-{file_id}"),
                            IconName::Close,
                        )
                        .icon_size(IconSize::XSmall)
                        .icon_color(Color::Muted)
                        .on_click(cx.listener({
                            let file_id = file_id.clone();
                            move |this, _, _, cx| this.cancel_upload(file_id.clone(), cx)
                        }))
                        .tooltip(Tooltip::text("Cancel upload")),
                    )
                },
            )
            .when(matches!(upload.status, UploadStatus::Failed(_)), |this| {
                this.child(
                    IconButton::new(
                        format!("retry-channel-file-upload-{file_id}"),
                        IconName::RotateCcw,
                    )
                    .icon_size(IconSize::XSmall)
                    .icon_color(Color::Muted)
                    .on_click(cx.listener({
                        let file_id = file_id.clone();
                        move |this, _, _, cx| this.retry_upload(file_id.clone(), cx)
                    }))
                    .tooltip(Tooltip::text("Retry upload")),
                )
            })
            .when(
                matches!(
                    upload.status,
                    UploadStatus::Completed | UploadStatus::Failed(_) | UploadStatus::Cancelled
                ),
                |this| {
                    this.child(
                        IconButton::new(
                            format!("remove-channel-file-upload-{file_id}"),
                            IconName::Close,
                        )
                        .icon_size(IconSize::XSmall)
                        .icon_color(Color::Muted)
                        .on_click(cx.listener({
                            let file_id = file_id.clone();
                            move |this, _, _, cx| this.remove_upload(file_id.clone(), cx)
                        }))
                        .tooltip(Tooltip::text("Remove upload")),
                    )
                },
            )
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
    pub fn open_thread_for_test(
        &mut self,
        root_message_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_thread(root_message_id, window, cx);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn thread_reply_bodies_for_test(&self) -> Vec<String> {
        self.thread_panel
            .as_ref()
            .map(|thread_panel| {
                thread_panel
                    .replies
                    .iter()
                    .map(|reply| reply.body.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn thread_reply_count_for_test(&self, root_message_id: u64) -> Option<u32> {
        self.thread_summaries
            .get(&root_message_id)
            .map(|summary| summary.reply_count)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn thread_has_unread_for_test(&self, root_message_id: u64) -> Option<bool> {
        self.thread_summaries
            .get(&root_message_id)
            .map(|summary| summary.has_unread)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn thread_deleted_placeholder_visible_for_test(&self) -> bool {
        self.thread_panel.as_ref().is_some_and(|thread_panel| {
            matches!(thread_panel.load_state, ThreadLoadState::Loaded)
                && thread_panel
                    .root_message
                    .as_ref()
                    .is_none_or(|root_message| root_message.body.is_empty())
        })
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn thread_load_error_for_test(&self) -> Option<SharedString> {
        self.thread_panel
            .as_ref()
            .and_then(|thread_panel| match &thread_panel.load_state {
                ThreadLoadState::Failed(message) => Some(message.clone()),
                ThreadLoadState::Loading | ThreadLoadState::Loaded => None,
            })
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn thread_draft_for_test(&self, cx: &App) -> String {
        self.thread_panel
            .as_ref()
            .map(|thread_panel| thread_panel.compose_editor.read(cx).text(cx))
            .unwrap_or_default()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_thread_draft_for_test(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(thread_panel) = self.thread_panel.as_ref() {
            thread_panel
                .compose_editor
                .update(cx, |composer, cx| composer.set_text(text, window, cx));
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn send_thread_reply_for_test(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.send_thread_reply(window, cx);
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

    #[cfg(any(test, feature = "test-support"))]
    pub fn set_search_query_for_test(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.search_state.active = true;
        self.search_editor
            .update(cx, |editor, cx| editor.set_text(text, window, cx));
        self.schedule_search(cx);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn search_result_bodies_for_test(&self) -> Vec<String> {
        self.search_state
            .results
            .iter()
            .filter_map(|result| result.message.as_ref())
            .map(|message| message.body.clone())
            .collect()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn search_error_for_test(&self) -> Option<SharedString> {
        self.search_state.error.clone()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn search_done_for_test(&self) -> bool {
        self.search_state.done
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn search_load_more_visible_for_test(&self) -> bool {
        !self.search_state.done && !self.search_state.results.is_empty()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn load_more_search_results_for_test(&mut self, cx: &mut Context<Self>) {
        self.load_more_search_results(cx);
    }

    fn render_bookmark_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let bookmarks = self
            .bookmark_store
            .read(cx)
            .bookmarks(self.channel_id)
            .to_vec();
        let has_overflow = bookmarks.len() > 5;
        let show_bookmark_section = !bookmarks.is_empty() || self.bookmark_form.is_some();

        v_flex()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .child(
                        Label::new(format!("Bookmarks ({})", bookmarks.len()))
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    )
                    .child(
                        Button::new("add-channel-bookmark", "Add")
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Compact)
                            .start_icon(Icon::new(IconName::Plus))
                            .disabled(
                                self.bookmark_form
                                    .as_ref()
                                    .is_some_and(BookmarkForm::is_submitting),
                            )
                            .on_click(cx.listener(Self::open_bookmark_form)),
                    ),
            )
            .when(show_bookmark_section, |this| {
                this.when(!bookmarks.is_empty(), |this| {
                    let weak_self = cx.weak_entity();
                    this.child(
                        ChannelBookmarkBar::new(bookmarks, self.bookmarks_expanded)
                            .on_edit({
                                let weak_self = weak_self.clone();
                                move |bookmark, window, cx| {
                                    weak_self
                                        .update(cx, |this, cx| {
                                            this.open_edit_bookmark_form(
                                                bookmark.clone(),
                                                window,
                                                cx,
                                            );
                                        })
                                        .log_err();
                                }
                            })
                            .on_delete(move |bookmark, window, cx| {
                                weak_self
                                    .update(cx, |this, cx| {
                                        this.confirm_remove_bookmark(bookmark.clone(), window, cx);
                                    })
                                    .log_err();
                            })
                            .on_reorder({
                                let weak_self = cx.weak_entity();
                                move |dragged_id, target_id, _, cx| {
                                    weak_self
                                        .update(cx, |this, cx| {
                                            this.reorder_bookmark(dragged_id, target_id, cx);
                                        })
                                        .log_err();
                                }
                            })
                            .on_open_message({
                                let weak_self = cx.weak_entity();
                                move |message_id, _, cx| {
                                    weak_self
                                        .update(cx, |this, cx| {
                                            this.highlight_message_bookmark(message_id, cx);
                                        })
                                        .log_err();
                                }
                            }),
                    )
                })
                .when_some(self.bookmark_form.as_ref(), |this, form| {
                    this.child(self.render_bookmark_form(form, cx))
                })
                .when_some(self.bookmark_action_error.clone(), |this, error| {
                    this.child(
                        div().px_3().pb_2().child(
                            Label::new(error)
                                .size(LabelSize::XSmall)
                                .color(Color::Error),
                        ),
                    )
                })
                .when(has_overflow, |this| {
                    this.child(
                        h_flex()
                            .justify_end()
                            .px_3()
                            .pb_2()
                            .border_b_1()
                            .border_color(cx.theme().colors().border)
                            .child(
                                Button::new(
                                    "toggle-channel-bookmarks",
                                    if self.bookmarks_expanded {
                                        "Show less"
                                    } else {
                                        "Show all"
                                    },
                                )
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.bookmarks_expanded = !this.bookmarks_expanded;
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                })
            })
    }

    fn render_bookmark_form(
        &self,
        form: &BookmarkForm,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let submitting = form.is_submitting();

        v_flex()
            .gap_2()
            .px_3()
            .pb_2()
            .when(!form.is_editing(), |this| {
                this.child(
                    v_flex()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_1()
                                .child(self.render_bookmark_type_button(
                                    proto::BookmarkType::BookmarkLink,
                                    "Link",
                                    form,
                                    cx,
                                ))
                                .child(self.render_bookmark_type_button(
                                    proto::BookmarkType::BookmarkFile,
                                    "File",
                                    form,
                                    cx,
                                ))
                                .child(self.render_bookmark_type_button(
                                    proto::BookmarkType::BookmarkMessage,
                                    "Message",
                                    form,
                                    cx,
                                )),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(div().flex_1().child(form.url_editor.clone()))
                                .child(div().flex_1().child(form.label_editor.clone())),
                        ),
                )
            })
            .when(form.is_editing(), |this| {
                this.child(form.label_editor.clone())
            })
            .child(form.description_editor.clone())
            .when_some(
                match &form.state {
                    BookmarkFormState::Failed(error) => Some(error.clone()),
                    BookmarkFormState::Idle | BookmarkFormState::Submitting => None,
                },
                |this, error| {
                    this.child(
                        Label::new(error)
                            .size(LabelSize::XSmall)
                            .color(Color::Error),
                    )
                },
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-channel-bookmark", "Cancel")
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Compact)
                            .disabled(submitting)
                            .on_click(cx.listener(Self::close_bookmark_form)),
                    )
                    .child(
                        Button::new(
                            "submit-channel-bookmark",
                            if form.is_editing() {
                                "Save changes"
                            } else {
                                "Save"
                            },
                        )
                        .style(ButtonStyle::Filled)
                        .size(ButtonSize::Compact)
                        .disabled(submitting)
                        .on_click(cx.listener(Self::submit_bookmark_form)),
                    ),
            )
    }

    fn render_bookmark_type_button(
        &self,
        bookmark_type: proto::BookmarkType,
        label: &'static str,
        form: &BookmarkForm,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(("channel-bookmark-type", bookmark_type as u32), label)
            .style(ButtonStyle::Subtle)
            .selected_style(ButtonStyle::Filled)
            .toggle_state(form.bookmark_type == bookmark_type)
            .size(ButtonSize::Compact)
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Some(form) = this.bookmark_form.as_mut() {
                    form.set_bookmark_type(bookmark_type, window, cx);
                }
                cx.notify();
            }))
    }

    fn open_bookmark_form(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.bookmark_form.is_none() {
            self.bookmark_form = Some(BookmarkForm::new_create(window, cx));
        }
        cx.notify();
    }

    fn open_edit_bookmark_form(
        &mut self,
        bookmark: Bookmark,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.bookmark_form = Some(BookmarkForm::new_edit(&bookmark, window, cx));
        self.bookmark_action_error = None;
        cx.notify();
    }

    fn highlight_message_bookmark(&mut self, message_id: u64, cx: &mut Context<Self>) {
        if self.messages.iter().any(|message| message.id == message_id) {
            self.highlighted_search_message_id = Some(message_id);
        } else {
            self.bookmark_action_error = Some("Bookmarked message is not loaded.".into());
        }
        cx.notify();
    }

    fn reorder_bookmark(
        &mut self,
        dragged_id: BookmarkId,
        target_id: BookmarkId,
        cx: &mut Context<Self>,
    ) {
        let bookmarks = self
            .bookmark_store
            .read(cx)
            .bookmarks(self.channel_id)
            .to_vec();
        let Some(ordered_ids) =
            crate::channel_bookmark_bar::reordered_bookmark_ids(&bookmarks, dragged_id, target_id)
        else {
            return;
        };
        let Some(previous_bookmarks) = self.bookmark_store.update(cx, |bookmark_store, cx| {
            bookmark_store.reorder_bookmarks(self.channel_id, &ordered_ids, cx)
        }) else {
            return;
        };

        self.bookmark_action_error = None;
        cx.notify();

        cx.spawn({
            let client = self.client.clone();
            let channel_id = self.channel_id;
            let bookmark_store = self.bookmark_store.clone();
            async move |this, cx| {
                let result = client.reorder_bookmarks(channel_id, ordered_ids).await;
                if let Err(error) = result {
                    bookmark_store.update(cx, |bookmark_store, cx| {
                        bookmark_store.set_bookmarks(channel_id, previous_bookmarks, cx);
                    });
                    this.update(cx, |this, cx| {
                        this.bookmark_action_error =
                            Some(format!("Failed to reorder bookmarks: {error:#}").into());
                        cx.notify();
                    })?;
                }
                anyhow::Ok(())
            }
        })
        .detach_and_log_err(cx);
    }

    fn close_bookmark_form(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.bookmark_form = None;
        cx.notify();
    }

    fn submit_bookmark_form(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.bookmark_form.as_mut() else {
            return;
        };
        if form.is_submitting() {
            return;
        }

        let request = match if form.is_editing() {
            form.update_bookmark(self.channel_id, cx)
                .map(BookmarkFormRequest::Update)
        } else {
            form.add_bookmark(self.channel_id, cx)
                .map(BookmarkFormRequest::Add)
        } {
            Ok(request) => request,
            Err(error) => {
                form.set_error(error);
                cx.notify();
                return;
            }
        };

        form.set_submitting();
        cx.notify();

        cx.spawn({
            let client = self.client.clone();
            async move |this, cx| {
                let result = match request {
                    BookmarkFormRequest::Add(bookmark) => client.add_bookmark(bookmark).await,
                    BookmarkFormRequest::Update(bookmark) => client.update_bookmark(bookmark).await,
                };
                this.update(cx, |this, cx| {
                    match result {
                        Ok(()) => {
                            this.bookmark_form = None;
                        }
                        Err(error) => {
                            if let Some(form) = this.bookmark_form.as_mut() {
                                form.set_error(SharedString::from(format!(
                                    "Failed to save bookmark: {error:#}"
                                )));
                            }
                        }
                    }
                    cx.notify();
                })?;
                anyhow::Ok(())
            }
        })
        .detach_and_log_err(cx);
    }

    fn confirm_remove_bookmark(
        &mut self,
        bookmark: Bookmark,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let answer = window.prompt(
            PromptLevel::Warning,
            "Delete bookmark?",
            Some(&format!(
                "This removes \"{}\" from the channel.",
                bookmark.label
            )),
            &["Delete", "Cancel"],
            cx,
        );
        let client = self.client.clone();
        let channel_id = self.channel_id;
        let bookmark_id = bookmark.id;

        cx.spawn_in(window, async move |this, cx| {
            if answer.await? != 0 {
                return anyhow::Ok(());
            }

            this.update(cx, |this, cx| {
                this.bookmark_action_error = None;
                cx.notify();
            })?;

            let result = client.remove_bookmark(channel_id, bookmark_id).await;
            this.update(cx, |this, cx| {
                if let Err(error) = result {
                    this.bookmark_action_error =
                        Some(format!("Failed to delete bookmark: {error:#}").into());
                }
                cx.notify();
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn selected_search_result_index_for_test(&self) -> Option<usize> {
        self.search_state.selected_result_index
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn select_next_search_result_for_test(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_next_search_result(&SelectNext, window, cx);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn select_previous_search_result_for_test(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_previous_search_result(&SelectPrevious, window, cx);
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
        h_flex()
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
            .on_action(cx.listener(Self::toggle_search))
            .on_action(cx.listener(Self::select_next_search_result))
            .on_action(cx.listener(Self::select_previous_search_result))
            .on_action(cx.listener(Self::close_thread))
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .child(self.render_search_header(window, cx))
                    .when(self.search_state.active, |this| {
                        this.child(self.render_search_results_panel(window, cx))
                    })
                    .child(self.render_bookmark_bar(cx))
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
                            .drag_over::<ExternalPaths>(|style, _, _, cx| {
                                style.bg(cx.theme().colors().drop_target_background)
                            })
                            .on_drop(cx.listener(Self::upload_external_paths))
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
                            .when(
                                !self
                                    .upload_manager
                                    .read(cx)
                                    .uploads_for_channel(self.channel_id)
                                    .is_empty(),
                                |this| this.child(self.render_uploads(cx)),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        IconButton::new("attach-channel-file", IconName::Paperclip)
                                            .icon_size(IconSize::Small)
                                            .icon_color(Color::Muted)
                                            .on_click(cx.listener(Self::open_file_picker))
                                            .tooltip(Tooltip::text("Attach file")),
                                    )
                                    .child(
                                        IconButton::new("toggle-schedule-picker", IconName::Clock)
                                            .icon_size(IconSize::Small)
                                            .icon_color(if self.schedule_picker.active {
                                                Color::Accent
                                            } else {
                                                Color::Muted
                                            })
                                            .on_click(cx.listener(Self::toggle_schedule_picker))
                                            .tooltip(Tooltip::text("Schedule message")),
                                    )
                                    .when_some(
                                        selected_schedule_label(&self.schedule_picker),
                                        |this, label| {
                                            this.child(
                                                Label::new(label)
                                                    .size(LabelSize::XSmall)
                                                    .color(Color::Muted),
                                            )
                                        },
                                    ),
                            )
                            .when(self.schedule_picker.popover_visible, |this| {
                                this.child(self.render_schedule_picker(cx))
                            })
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
                    ),
            )
            .when(self.thread_panel.is_some(), |this| {
                this.child(self.render_thread_panel(window, cx))
            })
            .when(self.scheduled_messages_panel.is_some(), |this| {
                this.child(self.render_scheduled_messages_panel(cx))
            })
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

fn format_scheduled_message_label(timestamp: u64) -> Option<String> {
    let timestamp = timestamp.try_into().ok()?;
    Some(format!(
        "scheduled {}",
        DateTime::<Utc>::from_timestamp_millis(timestamp)?
            .with_timezone(&Local)
            .format("%b %-d, %-I:%M %p")
    ))
}

fn schedule_label(timestamp: DateTime<Utc>) -> String {
    timestamp
        .with_timezone(&Local)
        .format("Scheduled for %b %-d, %-I:%M %p")
        .to_string()
}

fn selected_schedule_label(schedule_picker: &SchedulePicker) -> Option<String> {
    schedule_picker.scheduled_at_utc().map(schedule_label)
}

fn pending_scheduled_badge_label(pending_scheduled_count: usize) -> Option<String> {
    if pending_scheduled_count == 0 {
        None
    } else {
        Some(pending_scheduled_count.to_string())
    }
}

fn is_bookmark_system_message(body: &str) -> bool {
    (body.starts_with("Pinned a ") && body.contains(" bookmark: "))
        || body.starts_with("Updated bookmark: ")
        || body.starts_with("Removed bookmark: ")
}

fn scheduled_message_time_label(message: &ScheduledMessage) -> String {
    message.display_time.format("%-I:%M %p").to_string()
}

fn sort_scheduled_messages_for_display(messages: &mut [ScheduledMessage]) {
    messages.sort_by_key(|message| (message.scheduled_at, message.id));
}

fn month_start(date: NaiveDate) -> NaiveDate {
    date.with_day(1).unwrap_or(date)
}

fn add_months(date: NaiveDate, delta: i32) -> NaiveDate {
    let mut year = date.year();
    let mut month = date.month() as i32 + delta;
    while month < 1 {
        year -= 1;
        month += 12;
    }
    while month > 12 {
        year += 1;
        month -= 12;
    }
    NaiveDate::from_ymd_opt(year, month as u32, 1).unwrap_or(date)
}

fn days_in_month(month: NaiveDate) -> u32 {
    let next_month = add_months(month_start(month), 1);
    (next_month - ChronoDuration::days(1)).day()
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
    current_timestamp_nanos() ^ u128::from(channel_id.0)
}

fn current_unix_timestamp() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

fn current_timestamp_nanos() -> u128 {
    let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    };
    nanos
}

fn upload_status_text(status: &UploadStatus, progress: f32) -> SharedString {
    match status {
        UploadStatus::Uploading => format!("{:.0}%", progress * 100.0),
        UploadStatus::Confirming => "Finishing".to_string(),
        UploadStatus::Completed => "Uploaded".to_string(),
        UploadStatus::Failed(_) => "Failed".to_string(),
        UploadStatus::Cancelled => "Cancelled".to_string(),
    }
    .into()
}

fn upload_status_color(status: &UploadStatus) -> Color {
    match status {
        UploadStatus::Uploading | UploadStatus::Confirming => Color::Info,
        UploadStatus::Completed => Color::Success,
        UploadStatus::Failed(_) => Color::Error,
        UploadStatus::Cancelled => Color::Muted,
    }
}

fn upload_error_text(status: &UploadStatus) -> Option<SharedString> {
    match status {
        UploadStatus::Failed(error) => Some(error.clone().into()),
        UploadStatus::Uploading
        | UploadStatus::Confirming
        | UploadStatus::Completed
        | UploadStatus::Cancelled => None,
    }
}

fn upload_progress_value(status: &UploadStatus, progress: f32) -> f32 {
    match status {
        UploadStatus::Completed | UploadStatus::Confirming => 1.0,
        UploadStatus::Failed(_) | UploadStatus::Cancelled | UploadStatus::Uploading => progress,
    }
    .clamp(0.0, 1.0)
}

fn message_nonce(message: &proto::ChannelMessage) -> Option<u128> {
    message.nonce.clone().map(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_chat_key_bindings_parse() {
        let bindings = super::channel_chat_key_bindings();

        assert_eq!(bindings.len(), 10);
        assert!(bindings.iter().any(|binding| {
            binding.action().name().ends_with("TogglePreview")
                && binding
                    .keystrokes()
                    .first()
                    .is_some_and(|keystroke| keystroke.unparse() == "ctrl-shift-p")
        }));
        assert!(bindings.iter().any(|binding| {
            binding.action().name().ends_with("CloseThread")
                && binding
                    .keystrokes()
                    .first()
                    .is_some_and(|keystroke| keystroke.unparse() == "escape")
        }));
    }

    #[test]
    fn schedule_picker_round_trips_selected_utc_time() {
        let scheduled_at = DateTime::<Utc>::from_timestamp(1_893_456_000, 0).unwrap();
        let mut picker = SchedulePicker::new();

        picker.set_scheduled_at(scheduled_at);

        assert!(picker.active);
        assert_eq!(picker.scheduled_at_utc(), Some(scheduled_at));
        assert_eq!(picker.visible_month, month_start(picker.selected_date));
        assert!(picker.validation_error.is_none());
    }

    #[test]
    fn schedule_picker_validation_enforces_time_bounds() {
        let mut picker = SchedulePicker::new();
        assert_eq!(picker.validate().unwrap_err(), "Choose a valid local time");

        picker.set_scheduled_at(Utc::now() - ChronoDuration::minutes(5));
        assert_eq!(
            picker.validate().unwrap_err(),
            "Scheduled messages need at least 1 minute lead time"
        );

        picker.set_scheduled_at(Utc::now() + ChronoDuration::days(31));
        assert_eq!(
            picker.validate().unwrap_err(),
            "Scheduled messages can be at most 30 days away"
        );

        picker.set_scheduled_at(Utc::now() + ChronoDuration::minutes(5));
        assert!(picker.validate().is_ok());
    }

    #[test]
    fn scheduled_labels_render_in_local_timezone() {
        let scheduled_at = DateTime::<Utc>::from_timestamp(1_893_456_000, 0).unwrap();
        let timestamp = scheduled_at.timestamp_millis() as u64;

        assert_eq!(
            format_scheduled_message_label(timestamp),
            Some(format!(
                "scheduled {}",
                scheduled_at
                    .with_timezone(&Local)
                    .format("%b %-d, %-I:%M %p")
            ))
        );
        assert_eq!(
            schedule_label(scheduled_at),
            scheduled_at
                .with_timezone(&Local)
                .format("Scheduled for %b %-d, %-I:%M %p")
                .to_string()
        );
    }

    #[test]
    fn selected_schedule_label_tracks_picker_state() {
        let scheduled_at = DateTime::<Utc>::from_timestamp(1_893_456_000, 0).unwrap();
        let mut picker = SchedulePicker::new();

        assert!(selected_schedule_label(&picker).is_none());

        picker.set_scheduled_at(scheduled_at);

        assert_eq!(
            selected_schedule_label(&picker),
            Some(schedule_label(scheduled_at))
        );
    }

    #[test]
    fn pending_scheduled_badge_label_hides_zero_count() {
        assert_eq!(pending_scheduled_badge_label(0), None);
        assert_eq!(pending_scheduled_badge_label(3), Some("3".to_string()));
    }

    #[test]
    fn bookmark_system_message_detection_is_narrow() {
        assert!(is_bookmark_system_message(
            "Pinned a link bookmark: Deploy Guide"
        ));
        assert!(is_bookmark_system_message(
            "Pinned a message bookmark: Design thread"
        ));
        assert!(is_bookmark_system_message(
            "Updated bookmark: Deploy Guide v2"
        ));
        assert!(is_bookmark_system_message(
            "Removed bookmark: Deploy Guide v2"
        ));

        assert!(!is_bookmark_system_message(
            "Pinned a link outside the bookmark bar"
        ));
        assert!(!is_bookmark_system_message(
            "Updated my bookmark workflow notes"
        ));
        assert!(!is_bookmark_system_message(""));
    }

    #[test]
    fn scheduled_messages_sort_by_time_then_id_for_panel_display() {
        let earlier = DateTime::<Utc>::from_timestamp(1_893_456_000, 0).unwrap();
        let later = earlier + ChronoDuration::hours(1);
        let mut messages = vec![
            scheduled_message_for_test(3, later, "later"),
            scheduled_message_for_test(2, earlier, "second"),
            scheduled_message_for_test(1, earlier, "first"),
        ];

        sort_scheduled_messages_for_display(&mut messages);

        assert_eq!(
            messages
                .iter()
                .map(|message| (message.id.0, message.body.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "first"), (2, "second"), (3, "later")]
        );
    }

    fn scheduled_message_for_test(
        id: u64,
        scheduled_at: DateTime<Utc>,
        body: &str,
    ) -> ScheduledMessage {
        ScheduledMessage {
            id: ScheduledMessageId(id),
            channel_id: 1,
            sender_id: 1,
            body: body.to_string(),
            scheduled_at,
            created_at: scheduled_at,
            nonce: None,
            mentions: Vec::new(),
            display_time: scheduled_at.with_timezone(&Local),
        }
    }
}
