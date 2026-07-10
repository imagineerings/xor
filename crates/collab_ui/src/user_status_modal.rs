use client::UserStore;
use editor::{Editor, EditorEvent};
use gpui::{
    App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, Render, SharedString,
    Subscription, Window, prelude::*,
};
use ui::{Button, ButtonStyle, Label, LabelSize, prelude::*};
use workspace::ModalView;

const MAX_STATUS_CHARS: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearAfterOption {
    Never,
    ThirtyMinutes,
    OneHour,
    FourHours,
    Today,
    ThisWeek,
}

impl ClearAfterOption {
    fn minutes(self) -> Option<u32> {
        match self {
            Self::Never => None,
            Self::ThirtyMinutes => Some(30),
            Self::OneHour => Some(60),
            Self::FourHours => Some(240),
            Self::Today => Some(1_440),
            Self::ThisWeek => Some(10_080),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Never => "Never",
            Self::ThirtyMinutes => "30 min",
            Self::OneHour => "1 hour",
            Self::FourHours => "4 hours",
            Self::Today => "Today",
            Self::ThisWeek => "This week",
        }
    }
}

#[derive(Clone, Copy)]
pub struct StatusPreset {
    emoji: &'static str,
    label: &'static str,
    text: &'static str,
}

const STATUS_PRESETS: [StatusPreset; 7] = [
    StatusPreset { emoji: "📅", label: "In a meeting", text: "In a meeting" },
    StatusPreset { emoji: "🤒", label: "Out sick", text: "Out sick" },
    StatusPreset { emoji: "🏠", label: "Working remotely", text: "Working remotely" },
    StatusPreset { emoji: "🏖", label: "On vacation", text: "On vacation" },
    StatusPreset { emoji: "📞", label: "In a call", text: "In a call" },
    StatusPreset { emoji: "🌙", label: "Away", text: "Away" },
    StatusPreset { emoji: "⛔", label: "Busy", text: "Busy" },
];

pub struct UserStatusModal {
    user_store: Entity<UserStore>,
    focus_handle: FocusHandle,
    text_editor: Entity<Editor>,
    emoji: Option<SharedString>,
    clear_after: ClearAfterOption,
    saving: bool,
    error: Option<SharedString>,
    _text_subscription: Subscription,
}

impl UserStatusModal {
    pub fn new(user_store: Entity<UserStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("What's your status?", window, cx);
            editor
        });
        let text_subscription = cx.subscribe(&text_editor, |_this, _, event, cx| {
            if matches!(event, EditorEvent::BufferEdited) {
                cx.notify();
            }
        });
        Self {
            user_store,
            focus_handle: cx.focus_handle(),
            text_editor,
            emoji: None,
            clear_after: ClearAfterOption::Never,
            saving: false,
            error: None,
            _text_subscription: text_subscription,
        }
    }

    fn select_preset(&mut self, preset: StatusPreset, window: &mut Window, cx: &mut Context<Self>) {
        self.emoji = Some(preset.emoji.into());
        self.text_editor.update(cx, |editor, cx| {
            editor.set_text(preset.text, window, cx);
        });
        self.error = None;
        cx.notify();
    }

    fn select_clear_after(&mut self, clear_after: ClearAfterOption, cx: &mut Context<Self>) {
        self.clear_after = clear_after;
        cx.notify();
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let text = self.text_editor.read(cx).text(cx);
        let text = text.chars().take(MAX_STATUS_CHARS).collect::<String>();
        if text.trim().is_empty() {
            self.error = Some("Enter a status message".into());
            cx.notify();
            return;
        }
        self.saving = true;
        self.error = None;
        let task = self.user_store.update(cx, |store, cx| {
            store.set_status(self.emoji.clone(), text.into(), self.clear_after.minutes(), cx)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            this.update_in(cx, |this, _window, cx| match result {
                Ok(()) => cx.emit(DismissEvent),
                Err(error) => {
                    this.saving = false;
                    this.error = Some(error.to_string().into());
                    cx.notify();
                }
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        self.saving = true;
        self.error = None;
        let task = self.user_store.update(cx, |store, cx| store.clear_status(cx));
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            this.update_in(cx, |this, _window, cx| match result {
                Ok(()) => cx.emit(DismissEvent),
                Err(error) => {
                    this.saving = false;
                    this.error = Some(error.to_string().into());
                    cx.notify();
                }
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn dismiss(&mut self, _: &menu::Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }
}

impl EventEmitter<DismissEvent> for UserStatusModal {}
impl ModalView for UserStatusModal {}

impl Focusable for UserStatusModal {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for UserStatusModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text_length = self.text_editor.read(cx).text(cx).chars().count();
        v_flex()
            .key_context("UserStatusModal")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::dismiss))
            .elevation_3(cx)
            .w(rems(30.))
            .p_4()
            .gap_3()
            .child(Label::new("Set a status").size(LabelSize::Large))
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_1()
                    .children(STATUS_PRESETS.iter().enumerate().map(|(index, preset)| {
                        Button::new(format!("status-preset-{index}"), preset.label)
                            .style(ButtonStyle::Subtle)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.select_preset(*preset, window, cx);
                            }))
                    })),
            )
            .child(self.text_editor.clone())
            .child(
                Label::new(format!("{text_length}/{MAX_STATUS_CHARS}"))
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .when_some(self.error.clone(), |this, error| {
                this.child(Label::new(error).color(Color::Error))
            })
            .child(Label::new("Clear after").size(LabelSize::Small))
            .child(
                h_flex().flex_wrap().gap_1().children([
                    ClearAfterOption::Never,
                    ClearAfterOption::ThirtyMinutes,
                    ClearAfterOption::OneHour,
                    ClearAfterOption::FourHours,
                    ClearAfterOption::Today,
                    ClearAfterOption::ThisWeek,
                ]
                .into_iter()
                .map(|option| {
                    Button::new(format!("clear-after-{}", option.label()), option.label())
                        .style(ButtonStyle::Subtle)
                        .selected_style(if self.clear_after == option {
                            ButtonStyle::Filled
                        } else {
                            ButtonStyle::Subtle
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_clear_after(option, cx);
                        }))
                })),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("clear-status", "Clear")
                            .style(ButtonStyle::Subtle)
                            .disabled(self.saving)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.clear(window, cx);
                            })),
                    )
                    .child(
                        Button::new("save-status", "Save")
                            .disabled(self.saving)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save(window, cx);
                            })),
                    ),
            )
    }
}
