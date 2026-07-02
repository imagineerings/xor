use crate::BaymaxApp;
use gpui::{AnyElement, App, SharedString, Window};
use ui::prelude::*;

/// A simple chat-like app for sending and displaying messages.
pub struct ChatApp {
    messages: Vec<ChatMessage>,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub sender: SharedString,
    pub text: SharedString,
}

impl ChatApp {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, sender: impl Into<SharedString>, text: impl Into<SharedString>) {
        self.messages.push(ChatMessage {
            sender: sender.into(),
            text: text.into(),
        });
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }
}

impl BaymaxApp for ChatApp {
    fn id(&self) -> &str {
        "chat"
    }

    fn name(&self) -> SharedString {
        "Chat".into()
    }

    fn render(&self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        let messages = self.messages.clone();

        v_flex()
            .id("chat-app")
            .size_full()
            .gap_2()
            .p_4()
            .child(
                Headline::new("Chat")
                    .size(HeadlineSize::Small)
                    .color(Color::Default),
            )
            .child(
                div()
                    .id("messages")
                    .flex_1()
                    .overflow_y_scroll()
                    .v_flex()
                    .gap_1()
                    .children(messages.into_iter().map(|msg| {
                        h_flex()
                            .gap_1()
                            .child(
                                Label::new(msg.sender)
                                    .size(LabelSize::Small)
                                    .color(Color::Accent),
                            )
                            .child(Label::new(msg.text).size(LabelSize::Small))
                    })),
            )
            .into_any_element()
    }

    fn handle_action(&mut self, _action: &dyn gpui::Action, _cx: &mut App) {}
}
