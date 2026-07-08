use crate::{AnyAgentTool, ToolInspection, ToolInspector, ToolMonitor, ToolStats};
use anyhow::{Context as _, Result};
use language_model::{LanguageModelRequest, ModelTokenCounter, TokenCounter};
use posthog::{PostHogCaptureOutcome, PostHogClient, PostHogError, PostHogUserAction};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use std::time::Duration;
use telemetry::{Observation, ObservationLayer};

const DEFAULT_MAX_OBSERVATIONS: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmTokenEstimate {
    pub model: String,
    pub input_tokens: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTurnObservation {
    pub trace_id: String,
    pub message_count: usize,
    pub duration: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmRequestObservation {
    pub trace_id: String,
    pub model: String,
    pub duration: Duration,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInvocationObservation {
    pub trace_id: String,
    pub tool_name: String,
    pub duration: Duration,
    pub success: bool,
    pub error_kind: Option<String>,
}

#[derive(Clone)]
pub struct AgentObservability {
    observations: ObservationLayer,
    tool_monitor: ToolMonitor,
    tool_inspector: ToolInspector,
    posthog: Option<PostHogClient>,
}

impl AgentObservability {
    pub fn new() -> Self {
        Self::with_max_observations(DEFAULT_MAX_OBSERVATIONS)
    }

    pub fn with_max_observations(max_observations: usize) -> Self {
        Self {
            observations: ObservationLayer::new(max_observations),
            tool_monitor: ToolMonitor::default(),
            tool_inspector: ToolInspector::default(),
            posthog: None,
        }
    }

    pub fn with_posthog(mut self, posthog: PostHogClient) -> Self {
        self.posthog = Some(posthog);
        self
    }

    pub fn estimate_request_tokens(
        &self,
        model: impl Into<String>,
        request: &LanguageModelRequest,
    ) -> Result<LlmTokenEstimate> {
        let model = model.into();
        let counter = ModelTokenCounter::for_model(model.clone());
        let input_tokens = counter
            .count_tokens_in_messages(&request.messages)
            .with_context(|| format!("failed to count input tokens for model `{model}`"))?;

        Ok(LlmTokenEstimate {
            model,
            input_tokens: input_tokens as u64,
        })
    }

    pub fn record_agent_turn(
        &mut self,
        turn: AgentTurnObservation,
    ) -> Result<PostHogCaptureOutcome, PostHogError> {
        self.observations
            .record_agent_turn(turn.trace_id, turn.message_count, turn.duration);

        self.capture_posthog(
            PostHogUserAction::AgentTurnCompleted,
            properties([
                ("message_count", json!(turn.message_count)),
                ("duration_ms", json!(turn.duration.as_millis())),
            ]),
        )
    }

    pub fn record_llm_request(
        &mut self,
        request: LlmRequestObservation,
    ) -> Result<PostHogCaptureOutcome, PostHogError> {
        self.observations.record_llm_request(
            request.trace_id,
            request.model.clone(),
            request.duration,
            request.input_tokens,
            request.output_tokens,
        );

        self.capture_posthog(
            PostHogUserAction::Custom("llm_request_completed".to_string()),
            properties([
                ("model", json!(request.model)),
                ("duration_ms", json!(request.duration.as_millis())),
                ("input_tokens", json!(request.input_tokens)),
                ("output_tokens", json!(request.output_tokens)),
            ]),
        )
    }

    pub fn record_tool_invocation(
        &mut self,
        invocation: ToolInvocationObservation,
    ) -> Result<PostHogCaptureOutcome, PostHogError> {
        self.tool_monitor.record_invocation_at(
            &invocation.tool_name,
            now_ms(),
            invocation.duration,
            invocation.success,
            invocation.error_kind.clone(),
        );
        self.observations.record_tool_call(
            invocation.trace_id,
            invocation.tool_name.clone(),
            invocation.duration,
            invocation.success,
        );

        self.capture_posthog(
            PostHogUserAction::ToolInvoked,
            properties([
                ("tool", json!(invocation.tool_name)),
                ("success", json!(invocation.success)),
                ("duration_ms", json!(invocation.duration.as_millis())),
                ("error_code", json!(invocation.error_kind)),
            ]),
        )
    }

    pub fn inspect_tools<'a>(
        &self,
        tools: impl IntoIterator<Item = &'a Arc<dyn AnyAgentTool>>,
    ) -> Result<Vec<ToolInspection>> {
        self.tool_inspector.inspect_tools(tools)
    }

    pub fn observations(&self) -> Vec<Observation> {
        self.observations.observations().cloned().collect()
    }

    pub fn tool_stats(&self, tool_name: &str) -> Option<&ToolStats> {
        self.tool_monitor.get_stats(tool_name)
    }

    pub fn all_tool_stats(&self) -> collections::HashMap<String, ToolStats> {
        self.tool_monitor.get_all_stats()
    }

    pub fn reset_tool_monitor(&mut self) {
        self.tool_monitor.reset();
    }

    fn capture_posthog(
        &self,
        action: PostHogUserAction,
        properties: Map<String, Value>,
    ) -> Result<PostHogCaptureOutcome, PostHogError> {
        if let Some(posthog) = &self.posthog {
            posthog.capture_user_action(action, properties)
        } else {
            Ok(PostHogCaptureOutcome::Disabled)
        }
    }
}

impl Default for AgentObservability {
    fn default() -> Self {
        Self::new()
    }
}

fn properties(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Map<String, Value> {
    pairs
        .into_iter()
        .filter_map(|(key, value)| match value {
            Value::Null => None,
            value => Some((key.to_string(), value)),
        })
        .collect()
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use language_model::{
        CompletionIntent, LanguageModelRequestMessage, MessageContent, Role, Speed,
    };
    use posthog::{InMemoryPostHogSender, PostHogConfig};

    fn request_with_text(text: &str) -> LanguageModelRequest {
        LanguageModelRequest {
            thread_id: None,
            prompt_id: None,
            intent: Some(CompletionIntent::UserPrompt),
            messages: vec![LanguageModelRequestMessage {
                role: Role::User,
                content: vec![MessageContent::Text(text.to_string())],
                cache: false,
                reasoning_details: None,
            }],
            tools: Vec::new(),
            tool_choice: None,
            stop: Vec::new(),
            temperature: None,
            thinking_allowed: false,
            thinking_effort: None,
            speed: Some(Speed::Standard),
        }
    }

    #[test]
    fn estimates_request_tokens() {
        let observability = AgentObservability::new();
        let request = request_with_text("hello world");

        let estimate = observability
            .estimate_request_tokens("unknown-test-model", &request)
            .expect("token estimate should succeed");

        assert_eq!(estimate.model, "unknown-test-model");
        assert!(estimate.input_tokens > 0);
    }

    #[test]
    fn records_tool_observation_and_stats() {
        let mut observability = AgentObservability::new();

        observability
            .record_tool_invocation(ToolInvocationObservation {
                trace_id: "trace-1".to_string(),
                tool_name: "read_file".to_string(),
                duration: Duration::from_millis(25),
                success: false,
                error_kind: Some("not_found".to_string()),
            })
            .expect("recording tool invocation should not fail without PostHog");

        let stats = observability
            .tool_stats("read_file")
            .expect("tool stats should be recorded");
        assert_eq!(stats.invocation_count, 1);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.last_error_kind.as_deref(), Some("not_found"));
        assert_eq!(observability.observations().len(), 1);
    }

    #[test]
    fn captures_posthog_when_configured() {
        let sender = InMemoryPostHogSender::default();
        let posthog =
            PostHogClient::with_sender(PostHogConfig::new("key", "install"), sender.clone());
        let mut observability = AgentObservability::new().with_posthog(posthog);

        let outcome = observability
            .record_llm_request(LlmRequestObservation {
                trace_id: "trace".to_string(),
                model: "gpt-test".to_string(),
                duration: Duration::from_millis(7),
                input_tokens: 11,
                output_tokens: 3,
            })
            .expect("recording LLM request should capture PostHog");

        assert_eq!(outcome, PostHogCaptureOutcome::Sent);
        let captures = sender.captures();
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].1.event, "llm_request_completed");
        assert_eq!(
            captures[0].1.properties.get("model"),
            Some(&json!("gpt-test"))
        );
    }
}
