use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CodeExecutionToolInput {
    /// The sandbox language. Supported values: "math" and "text".
    pub language: String,
    /// Code or text to execute inside the sandbox.
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionToolOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl From<CodeExecutionToolOutput> for LanguageModelToolResultContent {
    fn from(output: CodeExecutionToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| format!("failed to serialize code execution output: {error}"))
            .into()
    }
}

pub struct CodeExecutionTool;

impl AgentTool for CodeExecutionTool {
    type Input = CodeExecutionToolInput;
    type Output = CodeExecutionToolOutput;

    const NAME: &'static str = "code_execution";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Other
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Running sandboxed code".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| match input.recv().await {
            Ok(input) => {
                let output = run_sandboxed(&input);
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().content(vec![output.stdout.clone().into()]),
                );
                if output.success {
                    Ok(output)
                } else {
                    Err(output)
                }
            }
            Err(error) => Err(CodeExecutionToolOutput {
                success: false,
                stdout: String::new(),
                stderr: error.to_string(),
            }),
        })
    }
}

fn run_sandboxed(input: &CodeExecutionToolInput) -> CodeExecutionToolOutput {
    match input.language.as_str() {
        "text" => CodeExecutionToolOutput {
            success: true,
            stdout: input.code.clone(),
            stderr: String::new(),
        },
        "math" => match evaluate_math_expression(&input.code) {
            Some(value) => CodeExecutionToolOutput {
                success: true,
                stdout: value.to_string(),
                stderr: String::new(),
            },
            None => CodeExecutionToolOutput {
                success: false,
                stdout: String::new(),
                stderr: "unsupported math expression".to_string(),
            },
        },
        language => CodeExecutionToolOutput {
            success: false,
            stdout: String::new(),
            stderr: format!("unsupported sandbox language: {language}"),
        },
    }
}

fn evaluate_math_expression(expression: &str) -> Option<i64> {
    let mut tokens = expression.split_whitespace();
    let mut value = tokens.next()?.parse::<i64>().ok()?;

    while let Some(operator) = tokens.next() {
        let right = tokens.next()?.parse::<i64>().ok()?;
        match operator {
            "+" => value += right,
            "-" => value -= right,
            "*" => value *= right,
            "/" if right != 0 => value /= right,
            _ => return None,
        }
    }

    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_simple_math_expression() {
        let output = run_sandboxed(&CodeExecutionToolInput {
            language: "math".to_string(),
            code: "2 + 3 * 4".to_string(),
        });

        assert_eq!(output.stdout, "20");
    }
}
