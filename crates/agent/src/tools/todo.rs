use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TodoToolInput {
    #[serde(default)]
    pub items: Vec<TodoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TodoItem {
    pub text: String,
    #[serde(default)]
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoToolOutput {
    pub items: Vec<TodoItem>,
    pub remaining: usize,
}

impl TodoToolOutput {
    fn from_items(items: Vec<TodoItem>) -> Self {
        let remaining = items.iter().filter(|item| !item.completed).count();
        Self { items, remaining }
    }
}

impl From<TodoToolOutput> for LanguageModelToolResultContent {
    fn from(output: TodoToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| format!("failed to serialize todo output: {error}"))
            .into()
    }
}

pub struct TodoTool;

impl AgentTool for TodoTool {
    type Input = TodoToolInput;
    type Output = TodoToolOutput;

    const NAME: &'static str = "todo";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Other
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Updating todo list".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| match input.recv().await {
            Ok(input) => {
                let output = TodoToolOutput::from_items(input.items);
                event_stream.update_fields(acp::ToolCallUpdateFields::new().content(vec![
                    format!("{} todo items remaining", output.remaining).into(),
                ]));
                Ok(output)
            }
            Err(error) => Err(TodoToolOutput {
                items: vec![TodoItem {
                    text: format!("failed to update todo list: {error}"),
                    completed: false,
                }],
                remaining: 1,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_output_counts_remaining_items() {
        let output = TodoToolOutput::from_items(vec![
            TodoItem {
                text: "done".to_string(),
                completed: true,
            },
            TodoItem {
                text: "remaining".to_string(),
                completed: false,
            },
        ]);

        assert_eq!(output.remaining, 1);
        assert_eq!(output.items.len(), 2);
    }
}
