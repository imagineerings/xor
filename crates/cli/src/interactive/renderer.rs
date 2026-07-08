use super::markdown_renderer::MarkdownRenderer;
use super::session::{ConversationMessage, ConversationRole};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalDimensions {
    pub width: u16,
    pub height: u16,
}

impl Default for TerminalDimensions {
    fn default() -> Self {
        Self {
            width: 80,
            height: 24,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallState {
    Running,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallView {
    pub name: String,
    pub arguments: Vec<(String, String)>,
    pub result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TerminalRenderer {
    dimensions: TerminalDimensions,
    markdown_renderer: MarkdownRenderer,
}

impl TerminalRenderer {
    pub fn new(dimensions: TerminalDimensions) -> Self {
        Self {
            dimensions,
            markdown_renderer: MarkdownRenderer::default(),
        }
    }

    pub fn with_markdown_renderer(
        dimensions: TerminalDimensions,
        markdown_renderer: MarkdownRenderer,
    ) -> Self {
        Self {
            dimensions,
            markdown_renderer,
        }
    }

    pub fn dimensions(&self) -> TerminalDimensions {
        self.dimensions
    }

    pub fn render_message(&self, message: &ConversationMessage) -> Vec<String> {
        let label = match message.role {
            ConversationRole::User => "You",
            ConversationRole::Assistant => "Assistant",
            ConversationRole::System => "System",
            ConversationRole::Tool => "Tool",
        };
        let mut rendered = vec![format!("{}{}{}", ansi::BOLD, label, ansi::RESET)];
        let body = match message.role {
            ConversationRole::Assistant => self.markdown_renderer.render(&message.content),
            ConversationRole::Tool => message
                .content
                .lines()
                .map(|line| format!("{}{}{}", ansi::DIM, line, ansi::RESET))
                .collect(),
            ConversationRole::User | ConversationRole::System => {
                message.content.lines().map(str::to_string).collect()
            }
        };

        for line in body {
            rendered.extend(self.wrap_line(&line, "  "));
        }
        rendered
    }

    pub fn render_conversation(&self, messages: &[ConversationMessage]) -> Vec<String> {
        let mut rendered = Vec::new();
        for (index, message) in messages.iter().enumerate() {
            if index > 0 {
                rendered.push(String::new());
            }
            rendered.extend(self.render_message(message));
        }
        rendered
    }

    pub fn render_markdown(&self, markdown: &str) -> Vec<String> {
        self.markdown_renderer.render(markdown)
    }

    pub fn render_spinner(&self, label: &str, tick: usize) -> String {
        const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
        let frame = FRAMES[tick % FRAMES.len()];
        format!("{frame} {label}")
    }

    pub fn render_tool_call(&self, tool_call: &ToolCallView, state: &ToolCallState) -> Vec<String> {
        let state_label = match state {
            ToolCallState::Running => format!("{}running{}", ansi::YELLOW, ansi::RESET),
            ToolCallState::Completed => format!("{}completed{}", ansi::GREEN, ansi::RESET),
            ToolCallState::Failed(error) => {
                format!("{}failed{}: {error}", ansi::RED, ansi::RESET)
            }
        };
        let mut rendered = vec![format!(
            "{}tool{} {} {}",
            ansi::BOLD,
            ansi::RESET,
            tool_call.name,
            state_label
        )];

        for (name, value) in &tool_call.arguments {
            rendered.extend(self.wrap_line(&format!("{name}: {value}"), "  "));
        }

        if let Some(result) = &tool_call.result {
            rendered.push(format!("{}result{}", ansi::DIM, ansi::RESET));
            for line in result.lines() {
                rendered.extend(self.wrap_line(line, "  "));
            }
        }

        rendered
    }

    fn wrap_line(&self, line: &str, prefix: &str) -> Vec<String> {
        let available_width = usize::from(self.dimensions.width)
            .saturating_sub(plain_text_len(prefix))
            .max(20);
        let mut wrapped = Vec::new();
        let mut current = String::new();
        let mut current_len = 0usize;

        for word in line.split_whitespace() {
            let word_len = plain_text_len(word);
            let separator_len = usize::from(!current.is_empty());
            if current_len + separator_len + word_len > available_width && !current.is_empty() {
                wrapped.push(format!("{prefix}{current}"));
                current.clear();
                current_len = 0;
            }
            if !current.is_empty() {
                current.push(' ');
                current_len += 1;
            }
            current.push_str(word);
            current_len += word_len;
        }

        if current.is_empty() {
            wrapped.push(prefix.to_string());
        } else {
            wrapped.push(format!("{prefix}{current}"));
        }

        wrapped
    }
}

fn plain_text_len(text: &str) -> usize {
    let mut length = 0;
    let mut in_escape = false;
    for character in text.chars() {
        if in_escape {
            if character == 'm' || character == '\\' {
                in_escape = false;
            }
            continue;
        }
        if character == '\x1b' {
            in_escape = true;
            continue;
        }
        length += 1;
    }
    length
}

mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const GREEN: &str = "\x1b[32m";
    pub const RED: &str = "\x1b[31m";
    pub const YELLOW: &str = "\x1b[33m";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_assistant_markdown_messages() {
        let renderer = TerminalRenderer::new(TerminalDimensions {
            width: 80,
            height: 24,
        });
        let message = ConversationMessage {
            role: ConversationRole::Assistant,
            content: "## Done\n\n- item".to_string(),
        };

        let rendered = renderer.render_message(&message);

        assert_eq!(
            rendered.first().map(String::as_str),
            Some("\x1b[1mAssistant\x1b[0m")
        );
        assert!(rendered.iter().any(|line| line.contains("Done")));
        assert!(rendered.iter().any(|line| line.contains("- item")));
    }

    #[test]
    fn renders_spinner_frames_deterministically() {
        let renderer = TerminalRenderer::new(TerminalDimensions::default());

        assert_eq!(renderer.render_spinner("thinking", 0), "| thinking");
        assert_eq!(renderer.render_spinner("thinking", 3), "\\ thinking");
        assert_eq!(renderer.render_spinner("thinking", 4), "| thinking");
    }

    #[test]
    fn renders_tool_calls_with_arguments_and_result() {
        let renderer = TerminalRenderer::new(TerminalDimensions {
            width: 36,
            height: 24,
        });
        let tool_call = ToolCallView {
            name: "shell".to_string(),
            arguments: vec![("command".to_string(), "cargo test -p cli".to_string())],
            result: Some("ok".to_string()),
        };

        let rendered = renderer.render_tool_call(&tool_call, &ToolCallState::Completed);

        assert!(
            rendered
                .iter()
                .any(|line| line.contains("shell") && line.contains("completed"))
        );
        assert!(rendered.iter().any(|line| line.contains("command:")));
        assert!(rendered.iter().any(|line| line.contains("ok")));
    }
}
