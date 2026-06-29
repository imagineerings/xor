use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OrchestratorToolInput {
    pub goal: String,
    #[serde(default)]
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorToolOutput {
    pub goal: String,
    pub steps: Vec<OrchestratorStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorStep {
    pub index: usize,
    pub description: String,
    pub status: String,
}

impl From<OrchestratorToolOutput> for LanguageModelToolResultContent {
    fn from(output: OrchestratorToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| format!("failed to serialize orchestrator output: {error}"))
            .into()
    }
}

pub struct OrchestratorTool;

impl AgentTool for OrchestratorTool {
    type Input = OrchestratorToolInput;
    type Output = OrchestratorToolOutput;

    const NAME: &'static str = "orchestrator";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Other
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Coordinating workflow".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| match input.recv().await {
            Ok(input) => {
                let output = orchestrate(input);
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new()
                        .content(vec![format!("Planned {} steps", output.steps.len()).into()]),
                );
                Ok(output)
            }
            Err(error) => Err(OrchestratorToolOutput {
                goal: format!("failed to read input: {error}"),
                steps: Vec::new(),
            }),
        })
    }
}

fn orchestrate(input: OrchestratorToolInput) -> OrchestratorToolOutput {
    let steps = if input.steps.is_empty() {
        vec![format!("Complete goal: {}", input.goal)]
    } else {
        input.steps
    };

    OrchestratorToolOutput {
        goal: input.goal,
        steps: steps
            .into_iter()
            .enumerate()
            .map(|(index, description)| OrchestratorStep {
                index: index + 1,
                description,
                status: if index == 0 { "in_progress" } else { "pending" }.to_string(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orchestrator_marks_first_step_in_progress() {
        let output = orchestrate(OrchestratorToolInput {
            goal: "ship feature".to_string(),
            steps: vec!["design".to_string(), "test".to_string()],
        });

        assert_eq!(output.goal, "ship feature");
        assert_eq!(output.steps.len(), 2);
        assert_eq!(output.steps[0].index, 1);
        assert_eq!(output.steps[0].status, "in_progress");
        assert_eq!(output.steps[1].status, "pending");
    }

    #[test]
    fn orchestrator_creates_default_step_for_empty_plan() {
        let output = orchestrate(OrchestratorToolInput {
            goal: "finish task".to_string(),
            steps: Vec::new(),
        });

        assert_eq!(output.steps.len(), 1);
        assert_eq!(output.steps[0].description, "Complete goal: finish task");
    }
}
