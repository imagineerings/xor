use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlatformExtensionToolInput {
    /// Operation requested from the platform extension.
    pub operation: String,
    /// Optional structured payload for the operation.
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformExtensionToolOutput {
    pub extension: String,
    pub operation: String,
    pub status: String,
    pub payload: serde_json::Value,
}

impl From<PlatformExtensionToolOutput> for LanguageModelToolResultContent {
    fn from(output: PlatformExtensionToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| {
                format!("failed to serialize platform extension output: {error}")
            })
            .into()
    }
}

macro_rules! platform_extension_tool {
    ($tool:ident, $name:literal, $title:literal) => {
        pub struct $tool;

        impl AgentTool for $tool {
            type Input = PlatformExtensionToolInput;
            type Output = PlatformExtensionToolOutput;

            const NAME: &'static str = $name;

            fn kind() -> acp::ToolKind {
                acp::ToolKind::Other
            }

            fn initial_title(
                &self,
                _input: Result<Self::Input, serde_json::Value>,
                _cx: &mut App,
            ) -> SharedString {
                $title.into()
            }

            fn run(
                self: Arc<Self>,
                input: ToolInput<Self::Input>,
                event_stream: ToolCallEventStream,
                cx: &mut App,
            ) -> Task<Result<Self::Output, Self::Output>> {
                cx.spawn(async move |_cx| match input.recv().await {
                    Ok(input) => {
                        let output = PlatformExtensionToolOutput {
                            extension: $name.to_string(),
                            operation: input.operation,
                            status: "accepted".to_string(),
                            payload: input.payload,
                        };
                        event_stream.update_fields(
                            acp::ToolCallUpdateFields::new().content(vec![format!(
                                "{} operation accepted: {}",
                                $name, output.operation
                            )
                            .into()]),
                        );
                        Ok(output)
                    }
                    Err(error) => Err(PlatformExtensionToolOutput {
                        extension: $name.to_string(),
                        operation: "read_input".to_string(),
                        status: format!("failed: {error}"),
                        payload: serde_json::Value::Null,
                    }),
                })
            }
        }
    };
}

platform_extension_tool!(AppsTool, "apps", "Using app connector");
platform_extension_tool!(
    ChatrecallTool,
    "chatrecall",
    "Retrieving chat recall context"
);
platform_extension_tool!(SummonTool, "summon", "Summoning agent workflow");
platform_extension_tool!(TomTool, "tom", "Using TOM integration");
platform_extension_tool!(AnalyzeTool, "analyze", "Analyzing structured data");
platform_extension_tool!(DeveloperTool, "developer", "Using developer extension");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_extension_tool_names_are_stable() {
        assert_eq!(AppsTool::NAME, "apps");
        assert_eq!(ChatrecallTool::NAME, "chatrecall");
        assert_eq!(SummonTool::NAME, "summon");
        assert_eq!(TomTool::NAME, "tom");
        assert_eq!(AnalyzeTool::NAME, "analyze");
        assert_eq!(DeveloperTool::NAME, "developer");
    }
}
