use crate::SimApp;
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

impl SimApp for ChatApp {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_chat_app() {
        let app = ChatApp::new();
        assert_eq!(app.id(), "chat");
        assert_eq!(app.name(), SharedString::from("Chat"));
        assert!(app.messages().is_empty());
    }

    #[test]
    fn test_add_message() {
        let mut app = ChatApp::new();
        app.add_message("Alice", "Hello");
        assert_eq!(app.messages().len(), 1);
        assert_eq!(app.messages()[0].sender, SharedString::from("Alice"));
        assert_eq!(app.messages()[0].text, SharedString::from("Hello"));
    }

    #[test]
    fn test_multiple_messages() {
        let mut app = ChatApp::new();
        app.add_message("Alice", "First");
        app.add_message("Bob", "Second");
        assert_eq!(app.messages().len(), 2);
        assert_eq!(app.messages()[0].sender, SharedString::from("Alice"));
        assert_eq!(app.messages()[1].sender, SharedString::from("Bob"));
    }

    #[test]
    fn test_chat_app_trait_methods() {
        let app = ChatApp::new();
        assert_eq!(app.id(), "chat");
        assert_eq!(app.name(), SharedString::from("Chat"));
    }
}
