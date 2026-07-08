use anyhow::Result;
use tiktoken_rs::{CoreBPE, bpe_for_model};

use crate::{LanguageModelRequestMessage, LanguageModelToolResultContent, MessageContent};

const MESSAGE_OVERHEAD_TOKENS: usize = 3;
const ASSISTANT_PRIMING_TOKENS: usize = 3;
const DEFAULT_CHARS_PER_TOKEN: usize = 4;

pub trait TokenCounter: Send + Sync {
    fn count_tokens(&self, text: &str) -> Result<usize>;

    fn count_tokens_in_messages(&self, messages: &[LanguageModelRequestMessage]) -> Result<usize> {
        let mut total = ASSISTANT_PRIMING_TOKENS;
        for message in messages {
            total += MESSAGE_OVERHEAD_TOKENS;
            total += self.count_tokens(&message.role.to_string())?;
            for content in &message.content {
                total += self.count_tokens(&message_content_text(content))?;
            }
        }
        Ok(total)
    }

    fn model_for_counter(&self) -> Option<String>;
}

pub struct TikTokenCounter {
    model: String,
    encoding: &'static CoreBPE,
}

impl TikTokenCounter {
    pub fn for_model(model: impl Into<String>) -> Result<Self> {
        let model = model.into();
        let encoding = bpe_for_model(&model)?;
        Ok(Self { model, encoding })
    }
}

impl TokenCounter for TikTokenCounter {
    fn count_tokens(&self, text: &str) -> Result<usize> {
        Ok(self.encoding.encode_with_special_tokens(text).len())
    }

    fn model_for_counter(&self) -> Option<String> {
        Some(self.model.clone())
    }
}

#[derive(Clone, Debug)]
pub struct CharacterTokenCounter {
    model: Option<String>,
    chars_per_token: usize,
}

impl CharacterTokenCounter {
    pub fn new(model: Option<String>) -> Self {
        Self {
            model,
            chars_per_token: DEFAULT_CHARS_PER_TOKEN,
        }
    }

    pub fn with_chars_per_token(model: Option<String>, chars_per_token: usize) -> Self {
        Self {
            model,
            chars_per_token: chars_per_token.max(1),
        }
    }
}

impl TokenCounter for CharacterTokenCounter {
    fn count_tokens(&self, text: &str) -> Result<usize> {
        let chars = text.chars().count();
        Ok(chars.div_ceil(self.chars_per_token))
    }

    fn model_for_counter(&self) -> Option<String> {
        self.model.clone()
    }
}

pub enum ModelTokenCounter {
    TikToken(TikTokenCounter),
    Character(CharacterTokenCounter),
}

impl ModelTokenCounter {
    pub fn for_model(model: impl Into<String>) -> Self {
        let model = model.into();
        match TikTokenCounter::for_model(model.clone()) {
            Ok(counter) => Self::TikToken(counter),
            Err(_) => Self::Character(CharacterTokenCounter::new(Some(model))),
        }
    }
}

impl TokenCounter for ModelTokenCounter {
    fn count_tokens(&self, text: &str) -> Result<usize> {
        match self {
            Self::TikToken(counter) => counter.count_tokens(text),
            Self::Character(counter) => counter.count_tokens(text),
        }
    }

    fn count_tokens_in_messages(&self, messages: &[LanguageModelRequestMessage]) -> Result<usize> {
        match self {
            Self::TikToken(counter) => counter.count_tokens_in_messages(messages),
            Self::Character(counter) => counter.count_tokens_in_messages(messages),
        }
    }

    fn model_for_counter(&self) -> Option<String> {
        match self {
            Self::TikToken(counter) => counter.model_for_counter(),
            Self::Character(counter) => counter.model_for_counter(),
        }
    }
}

fn message_content_text(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(text) => text.clone(),
        MessageContent::Thinking { text, .. } => text.clone(),
        MessageContent::RedactedThinking(text) => text.clone(),
        MessageContent::Image(image) => image.to_base64_url(),
        MessageContent::ToolUse(tool_use) => {
            if tool_use.raw_input.is_empty() {
                tool_use.name.to_string()
            } else {
                format!("{} {}", tool_use.name, tool_use.raw_input)
            }
        }
        MessageContent::ToolResult(tool_result) => tool_result
            .content
            .iter()
            .filter_map(|content| match content {
                LanguageModelToolResultContent::Text(text) => Some(text.as_ref()),
                LanguageModelToolResultContent::Image(_) => None,
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{LanguageModelRequestMessage, MessageContent, Role};

    #[test]
    fn character_counter_counts_unicode_by_character() {
        let counter = CharacterTokenCounter::with_chars_per_token(None, 4);
        assert_eq!(counter.count_tokens("").unwrap(), 0);
        assert_eq!(counter.count_tokens("abcd").unwrap(), 1);
        assert_eq!(counter.count_tokens("abcde").unwrap(), 2);
        assert_eq!(counter.count_tokens("🤖🤖🤖🤖🤖").unwrap(), 2);
    }

    #[test]
    fn tiktoken_counter_counts_known_model_text() {
        let counter = TikTokenCounter::for_model("gpt-4").unwrap();
        assert_eq!(counter.count_tokens("hello world").unwrap(), 2);
        assert_eq!(counter.model_for_counter(), Some("gpt-4".to_string()));
    }

    #[test]
    fn model_counter_falls_back_for_unknown_model() {
        let counter = ModelTokenCounter::for_model("unknown-model-family");
        assert!(matches!(counter, ModelTokenCounter::Character(_)));
        assert_eq!(
            counter.model_for_counter(),
            Some("unknown-model-family".to_string())
        );
    }

    #[test]
    fn counts_message_roles_and_contents() {
        let counter = CharacterTokenCounter::with_chars_per_token(None, 4);
        let messages = vec![
            LanguageModelRequestMessage {
                role: Role::User,
                content: vec![MessageContent::Text("hello".into())],
                cache: false,
                reasoning_details: None,
            },
            LanguageModelRequestMessage {
                role: Role::Assistant,
                content: vec![MessageContent::Thinking {
                    text: "thinking".into(),
                    signature: None,
                }],
                cache: false,
                reasoning_details: Some(Arc::new(serde_json::json!({"ok": true}))),
            },
        ];

        assert_eq!(counter.count_tokens_in_messages(&messages).unwrap(), 17);
    }
}
