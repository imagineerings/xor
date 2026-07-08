use crate::AnyAgentTool;
use anyhow::{Context as _, Result};
use language_model::LanguageModelToolSchemaFormat;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolInspection {
    pub name: String,
    pub description: String,
    pub kind: String,
    pub supports_input_streaming: bool,
    pub input_schema: Value,
}

#[derive(Clone, Debug)]
pub struct ToolInspector {
    schema_format: LanguageModelToolSchemaFormat,
}

impl ToolInspector {
    pub fn new(schema_format: LanguageModelToolSchemaFormat) -> Self {
        Self { schema_format }
    }

    pub fn inspect_tools<'a>(
        &self,
        tools: impl IntoIterator<Item = &'a Arc<dyn AnyAgentTool>>,
    ) -> Result<Vec<ToolInspection>> {
        tools
            .into_iter()
            .map(|tool| self.inspect_tool(tool))
            .collect()
    }

    pub fn inspect_tool(&self, tool: &Arc<dyn AnyAgentTool>) -> Result<ToolInspection> {
        let name = tool.name().to_string();
        Ok(ToolInspection {
            name: name.clone(),
            description: tool.description().to_string(),
            kind: format!("{:?}", tool.kind()),
            supports_input_streaming: tool.supports_input_streaming(),
            input_schema: tool
                .input_schema(self.schema_format)
                .with_context(|| format!("failed to inspect input schema for tool `{name}`"))?,
        })
    }
}

impl Default for ToolInspector {
    fn default() -> Self {
        Self::new(LanguageModelToolSchemaFormat::JsonSchema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentToolOutput, ToolCallEventStream, ToolInput, thread::AnyAgentTool};
    use agent_client_protocol::schema as acp;
    use anyhow::anyhow;
    use gpui::{App, SharedString, Task};
    use language_model::LanguageModelProviderId;
    use serde_json::json;

    struct FakeTool {
        name: &'static str,
        streaming: bool,
    }

    impl AnyAgentTool for FakeTool {
        fn name(&self) -> SharedString {
            self.name.into()
        }

        fn description(&self) -> SharedString {
            "Fake tool description".into()
        }

        fn kind(&self) -> acp::ToolKind {
            acp::ToolKind::Read
        }

        fn initial_title(&self, _input: Value, _cx: &mut App) -> SharedString {
            "Fake".into()
        }

        fn input_schema(&self, _format: LanguageModelToolSchemaFormat) -> Result<Value> {
            Ok(json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                }
            }))
        }

        fn supports_input_streaming(&self) -> bool {
            self.streaming
        }

        fn supports_provider(&self, _provider: &LanguageModelProviderId) -> bool {
            true
        }

        fn run(
            self: Arc<Self>,
            _input: ToolInput<Value>,
            _event_stream: ToolCallEventStream,
            _cx: &mut App,
        ) -> Task<Result<AgentToolOutput, AgentToolOutput>> {
            Task::ready(Err(anyhow!("fake tool is not executable").into()))
        }

        fn replay(
            &self,
            _input: Value,
            _output: Value,
            _event_stream: ToolCallEventStream,
            _cx: &mut App,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn inspects_registered_tools() {
        let inspector = ToolInspector::default();
        let tools: Vec<Arc<dyn AnyAgentTool>> = vec![Arc::new(FakeTool {
            name: "fake_tool",
            streaming: true,
        })];

        let inspected = inspector
            .inspect_tools(&tools)
            .expect("tool inspection should succeed");

        assert_eq!(inspected.len(), 1);
        assert_eq!(inspected[0].name, "fake_tool");
        assert_eq!(inspected[0].description, "Fake tool description");
        assert!(inspected[0].supports_input_streaming);
        assert_eq!(inspected[0].input_schema["type"], json!("object"));
    }
}
