use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SummarizeToolInput {
    pub text: String,
    pub max_sentences: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeToolOutput {
    pub summary: String,
}

impl From<SummarizeToolOutput> for LanguageModelToolResultContent {
    fn from(output: SummarizeToolOutput) -> Self {
        output.summary.into()
    }
}

pub struct SummarizeTool;

impl AgentTool for SummarizeTool {
    type Input = SummarizeToolInput;
    type Output = SummarizeToolOutput;

    const NAME: &'static str = "summarize";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Other
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Summarizing content".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| match input.recv().await {
            Ok(input) => {
                let output = SummarizeToolOutput {
                    summary: summarize_text(&input.text, input.max_sentences.unwrap_or(3)),
                };
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().content(vec![output.summary.clone().into()]),
                );
                Ok(output)
            }
            Err(error) => Err(SummarizeToolOutput {
                summary: format!("failed to summarize content: {error}"),
            }),
        })
    }
}

fn summarize_text(text: &str, max_sentences: usize) -> String {
    let mut summary = String::new();
    let mut sentence_count = 0;

    for character in text.chars() {
        summary.push(character);
        if matches!(character, '.' | '!' | '?') {
            sentence_count += 1;
            if sentence_count >= max_sentences {
                break;
            }
        }
    }

    if summary.trim().is_empty() {
        text.chars().take(400).collect()
    } else {
        summary.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_text_respects_sentence_limit() {
        let summary = summarize_text("One. Two! Three?", 2);

        assert_eq!(summary, "One. Two!");
    }

    #[test]
    fn summarize_text_falls_back_for_text_without_sentence_end() {
        let summary = summarize_text("plain text without punctuation", 2);

        assert_eq!(summary, "plain text without punctuation");
    }
}
