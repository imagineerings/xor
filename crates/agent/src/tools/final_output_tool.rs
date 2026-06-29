use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Format the final answer for the user after completing a task.
///
/// Use this tool when you have completed the requested work and want to assemble
/// a structured final response from the relevant results, validation, and
/// follow-up items. After this tool returns, send the formatted answer to the
/// user in your next assistant message.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FinalOutputToolInput {
    /// A concise summary of the completed work or answer.
    pub summary: String,
    /// Optional result details to include below the summary.
    #[serde(default)]
    pub details: Vec<FinalOutputSection>,
    /// Optional validation commands or checks that were run.
    #[serde(default)]
    pub validation: Vec<String>,
    /// Optional follow-up items that remain relevant.
    #[serde(default)]
    pub next_steps: Vec<String>,
    /// Optional file paths, URLs, or other references that support the answer.
    #[serde(default)]
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FinalOutputSection {
    /// Short section label.
    pub label: String,
    /// Section body. Use concise markdown when helpful.
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalOutputToolOutput {
    pub markdown: String,
}

impl From<FinalOutputToolOutput> for LanguageModelToolResultContent {
    fn from(output: FinalOutputToolOutput) -> Self {
        output.markdown.into()
    }
}

pub struct FinalOutputTool;

impl AgentTool for FinalOutputTool {
    type Input = FinalOutputToolInput;
    type Output = FinalOutputToolOutput;

    const NAME: &'static str = "final_output";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Other
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Formatting final output".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| match input.recv().await {
            Ok(input) => {
                let markdown = render_final_output(&input);
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().content(vec![markdown.clone().into()]),
                );
                Ok(FinalOutputToolOutput { markdown })
            }
            Err(error) => Err(FinalOutputToolOutput {
                markdown: format!("Failed to format final output: {error}"),
            }),
        })
    }
}

fn render_final_output(input: &FinalOutputToolInput) -> String {
    let mut markdown = String::new();
    markdown.push_str(input.summary.trim());
    markdown.push('\n');

    for section in &input.details {
        push_section(&mut markdown, &section.label, &[section.content.as_str()]);
    }

    push_section(
        &mut markdown,
        "Validation",
        &input
            .validation
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    push_section(
        &mut markdown,
        "Next Steps",
        &input
            .next_steps
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    push_section(
        &mut markdown,
        "References",
        &input
            .references
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );

    markdown
}

fn push_section(markdown: &mut String, title: &str, items: &[&str]) {
    let items = items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if items.is_empty() {
        return;
    }

    markdown.push('\n');
    markdown.push_str("**");
    markdown.push_str(title);
    markdown.push_str("**\n");
    for item in items {
        markdown.push_str("- ");
        markdown.push_str(item);
        markdown.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_final_output_includes_structured_sections() {
        let output = render_final_output(&FinalOutputToolInput {
            summary: "Implemented the change.".to_string(),
            details: vec![FinalOutputSection {
                label: "Changed".to_string(),
                content: "Added a formatter.".to_string(),
            }],
            validation: vec!["cargo check -p agent".to_string()],
            next_steps: vec!["Run broader tests.".to_string()],
            references: vec!["crates/agent/src/tools/final_output_tool.rs".to_string()],
        });

        assert_eq!(
            output,
            "Implemented the change.\n\
             \n\
             **Changed**\n\
             - Added a formatter.\n\
             \n\
             **Validation**\n\
             - cargo check -p agent\n\
             \n\
             **Next Steps**\n\
             - Run broader tests.\n\
             \n\
             **References**\n\
             - crates/agent/src/tools/final_output_tool.rs\n"
        );
    }

    #[test]
    fn test_render_final_output_skips_empty_sections() {
        let output = render_final_output(&FinalOutputToolInput {
            summary: "Done.".to_string(),
            details: vec![FinalOutputSection {
                label: "Empty".to_string(),
                content: " ".to_string(),
            }],
            validation: Vec::new(),
            next_steps: Vec::new(),
            references: Vec::new(),
        });

        assert_eq!(output, "Done.\n");
    }
}
