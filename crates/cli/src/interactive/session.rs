use anyhow::Result;

use super::input::{InputEditor, InputEvent, InputOutcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationMessage {
    pub role: ConversationRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    InputUpdated,
    UserMessageSubmitted(String),
    AssistantMessageReceived(String),
    Canceled,
    NoChange,
}

pub trait TerminalMode {
    fn enter_raw_mode(&mut self) -> Result<()>;
    fn leave_raw_mode(&mut self) -> Result<()>;
}

pub struct TerminalModeGuard<'a, T: TerminalMode> {
    terminal_mode: &'a mut T,
    active: bool,
}

impl<'a, T: TerminalMode> TerminalModeGuard<'a, T> {
    pub fn enter(terminal_mode: &'a mut T) -> Result<Self> {
        terminal_mode.enter_raw_mode()?;
        Ok(Self {
            terminal_mode,
            active: true,
        })
    }

    pub fn leave(mut self) -> Result<()> {
        if self.active {
            self.active = false;
            self.terminal_mode.leave_raw_mode()?;
        }
        Ok(())
    }
}

impl<T: TerminalMode> Drop for TerminalModeGuard<'_, T> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.terminal_mode.leave_raw_mode();
            self.active = false;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InteractiveSession {
    conversation: Vec<ConversationMessage>,
    input: InputEditor,
    awaiting_agent_output: bool,
}

impl InteractiveSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_history(history: impl IntoIterator<Item = String>) -> Self {
        let mut session = Self::new();
        for entry in history {
            session.input.push_history(entry);
        }
        session
    }

    pub fn conversation(&self) -> &[ConversationMessage] {
        &self.conversation
    }

    pub fn input(&self) -> &InputEditor {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut InputEditor {
        &mut self.input
    }

    pub fn is_awaiting_agent_output(&self) -> bool {
        self.awaiting_agent_output
    }

    pub fn handle_input_event(&mut self, event: InputEvent) -> SessionEvent {
        match self.input.handle_event(event) {
            InputOutcome::Updated => SessionEvent::InputUpdated,
            InputOutcome::Submitted(input) => {
                self.submit_user_message(input.clone());
                SessionEvent::UserMessageSubmitted(input)
            }
            InputOutcome::Canceled => SessionEvent::Canceled,
            InputOutcome::Unchanged => SessionEvent::NoChange,
        }
    }

    pub fn submit_user_message(&mut self, content: String) {
        self.conversation.push(ConversationMessage {
            role: ConversationRole::User,
            content,
        });
        self.awaiting_agent_output = true;
    }

    pub fn receive_agent_output(&mut self, content: String) -> SessionEvent {
        self.conversation.push(ConversationMessage {
            role: ConversationRole::Assistant,
            content: content.clone(),
        });
        self.awaiting_agent_output = false;
        SessionEvent::AssistantMessageReceived(content)
    }

    pub fn add_system_message(&mut self, content: String) {
        self.conversation.push(ConversationMessage {
            role: ConversationRole::System,
            content,
        });
    }

    pub fn add_tool_message(&mut self, content: String) {
        self.conversation.push(ConversationMessage {
            role: ConversationRole::Tool,
            content,
        });
    }

    pub fn clear_conversation(&mut self) {
        self.conversation.clear();
        self.awaiting_agent_output = false;
    }

    pub fn render_conversation_plain_text(&self) -> String {
        self.conversation
            .iter()
            .map(|message| {
                let role = match message.role {
                    ConversationRole::User => "user",
                    ConversationRole::Assistant => "assistant",
                    ConversationRole::System => "system",
                    ConversationRole::Tool => "tool",
                };
                format!("{role}: {}", message.content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use anyhow::anyhow;

    use super::*;

    #[test]
    fn records_user_and_agent_messages() {
        let mut session = InteractiveSession::new();

        assert_eq!(
            session.handle_input_event(InputEvent::Insert('h')),
            SessionEvent::InputUpdated
        );
        assert_eq!(
            session.handle_input_event(InputEvent::Insert('i')),
            SessionEvent::InputUpdated
        );
        assert_eq!(
            session.handle_input_event(InputEvent::Submit),
            SessionEvent::UserMessageSubmitted("hi".to_string())
        );
        assert!(session.is_awaiting_agent_output());

        assert_eq!(
            session.receive_agent_output("hello".to_string()),
            SessionEvent::AssistantMessageReceived("hello".to_string())
        );
        assert!(!session.is_awaiting_agent_output());
        assert_eq!(
            session.render_conversation_plain_text(),
            "user: hi\nassistant: hello"
        );
    }

    #[test]
    fn restores_session_history() {
        let mut session = InteractiveSession::with_history(["old prompt".to_string()]);

        assert_eq!(
            session.handle_input_event(InputEvent::PreviousHistory),
            SessionEvent::InputUpdated
        );
        assert_eq!(session.input().buffer(), "old prompt");
    }

    #[derive(Clone, Default)]
    struct TestTerminalMode {
        events: Rc<RefCell<Vec<&'static str>>>,
        fail_leave: bool,
    }

    impl TerminalMode for TestTerminalMode {
        fn enter_raw_mode(&mut self) -> Result<()> {
            self.events.borrow_mut().push("enter");
            Ok(())
        }

        fn leave_raw_mode(&mut self) -> Result<()> {
            self.events.borrow_mut().push("leave");
            if self.fail_leave {
                return Err(anyhow!("leave failed"));
            }
            Ok(())
        }
    }

    #[test]
    fn terminal_mode_guard_leaves_raw_mode_on_drop() -> Result<()> {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut terminal_mode = TestTerminalMode {
            events: events.clone(),
            fail_leave: false,
        };

        {
            let _guard = TerminalModeGuard::enter(&mut terminal_mode)?;
        }

        assert_eq!(&*events.borrow(), &["enter", "leave"]);
        Ok(())
    }

    #[test]
    fn terminal_mode_guard_reports_explicit_leave_errors() -> Result<()> {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut terminal_mode = TestTerminalMode {
            events: events.clone(),
            fail_leave: true,
        };
        let guard = TerminalModeGuard::enter(&mut terminal_mode)?;

        let error = guard
            .leave()
            .expect_err("leave should report terminal errors");

        assert_eq!(error.to_string(), "leave failed");
        assert_eq!(&*events.borrow(), &["enter", "leave"]);
        Ok(())
    }
}
