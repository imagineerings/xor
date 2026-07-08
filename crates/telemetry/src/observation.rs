use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{
    LangfuseBackend, LangfuseExportError, LangfuseExporter, LangfuseSpan, OtlpBackend,
    OtlpExportError, OtlpExporter, OtlpSpan,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    AgentTurn,
    ToolCall,
    LlmRequest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub id: u64,
    pub trace_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub kind: ObservationKind,
    pub timestamp_ms: u128,
    pub duration_ms: Option<u128>,
    pub success: Option<bool>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub metadata: Map<String, Value>,
}

impl Observation {
    fn new(
        id: u64,
        trace_id: impl Into<String>,
        name: impl Into<String>,
        kind: ObservationKind,
    ) -> Self {
        Self {
            id,
            trace_id: trace_id.into(),
            parent_id: None,
            name: name.into(),
            kind,
            timestamp_ms: now_ms(),
            duration_ms: None,
            success: None,
            input_tokens: None,
            output_tokens: None,
            metadata: Map::new(),
        }
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = Some(duration.as_millis());
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = Some(success);
        self
    }

    pub fn with_tokens(mut self, input_tokens: u64, output_tokens: u64) -> Self {
        self.input_tokens = Some(input_tokens);
        self.output_tokens = Some(output_tokens);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(value) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), value);
        }
        self
    }
}

#[derive(Clone, Debug)]
pub struct ObservationLayer {
    max_observations: usize,
    next_id: u64,
    observations: VecDeque<Observation>,
}

impl ObservationLayer {
    pub fn new(max_observations: usize) -> Self {
        Self {
            max_observations: max_observations.max(1),
            next_id: 1,
            observations: VecDeque::new(),
        }
    }

    pub fn record_agent_turn(
        &mut self,
        trace_id: impl Into<String>,
        message_count: usize,
        duration: Duration,
    ) -> u64 {
        let observation = self
            .next_observation(trace_id, "agent.turn", ObservationKind::AgentTurn)
            .with_duration(duration)
            .with_metadata("message_count", message_count);
        self.push(observation)
    }

    pub fn record_tool_call(
        &mut self,
        trace_id: impl Into<String>,
        tool_name: impl Into<String>,
        duration: Duration,
        success: bool,
    ) -> u64 {
        let tool_name = tool_name.into();
        let observation = self
            .next_observation(
                trace_id,
                format!("tool.call {tool_name}"),
                ObservationKind::ToolCall,
            )
            .with_duration(duration)
            .with_success(success)
            .with_metadata("tool_name", tool_name);
        self.push(observation)
    }

    pub fn record_llm_request(
        &mut self,
        trace_id: impl Into<String>,
        model: impl Into<String>,
        duration: Duration,
        input_tokens: u64,
        output_tokens: u64,
    ) -> u64 {
        let model = model.into();
        let observation = self
            .next_observation(
                trace_id,
                format!("llm.request {model}"),
                ObservationKind::LlmRequest,
            )
            .with_duration(duration)
            .with_tokens(input_tokens, output_tokens)
            .with_metadata("model", model);
        self.push(observation)
    }

    pub fn observations(&self) -> impl ExactSizeIterator<Item = &Observation> {
        self.observations.iter()
    }

    pub fn export_to_langfuse<E>(
        &self,
        backend: &LangfuseBackend<E>,
    ) -> Result<(), LangfuseExportError>
    where
        E: LangfuseExporter,
    {
        for observation in &self.observations {
            backend.record_span(observation.to_langfuse_span())?;
        }
        Ok(())
    }

    pub fn export_to_otlp<E>(&self, backend: &OtlpBackend<E>) -> Result<(), OtlpExportError>
    where
        E: OtlpExporter,
    {
        for observation in &self.observations {
            backend.record_span(observation.to_otlp_span())?;
        }
        Ok(())
    }

    fn next_observation(
        &mut self,
        trace_id: impl Into<String>,
        name: impl Into<String>,
        kind: ObservationKind,
    ) -> Observation {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        Observation::new(id, trace_id, name, kind)
    }

    fn push(&mut self, observation: Observation) -> u64 {
        let id = observation.id;
        if self.observations.len() == self.max_observations {
            self.observations.pop_front();
        }
        self.observations.push_back(observation);
        id
    }
}

impl Observation {
    fn to_langfuse_span(&self) -> LangfuseSpan {
        let span_id = self.id.to_string();
        let mut span = match self.kind {
            ObservationKind::AgentTurn => LangfuseSpan::agent_turn(&self.trace_id, span_id),
            ObservationKind::ToolCall => LangfuseSpan::tool_call(
                &self.trace_id,
                span_id,
                self.metadata
                    .get("tool_name")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.name),
            ),
            ObservationKind::LlmRequest => LangfuseSpan::llm_call(
                &self.trace_id,
                span_id,
                self.metadata
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.name),
            ),
        };
        if let Some(parent_id) = &self.parent_id {
            span = span.with_parent(parent_id.clone());
        }
        self.add_common_langfuse_attributes(span)
    }

    fn to_otlp_span(&self) -> OtlpSpan {
        let span_id = self.id.to_string();
        let mut span = match self.kind {
            ObservationKind::AgentTurn => OtlpSpan::agent_turn(&self.trace_id, span_id),
            ObservationKind::ToolCall => OtlpSpan::tool_call(
                &self.trace_id,
                span_id,
                self.metadata
                    .get("tool_name")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.name),
            ),
            ObservationKind::LlmRequest => OtlpSpan::llm_request(
                &self.trace_id,
                span_id,
                self.metadata
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.name),
            ),
        };
        if let Some(parent_id) = &self.parent_id {
            span = span.with_parent(parent_id.clone());
        }
        self.add_common_otlp_attributes(span)
    }

    fn add_common_langfuse_attributes(&self, mut span: LangfuseSpan) -> LangfuseSpan {
        span.started_at_ms = self.timestamp_ms;
        if let Some(duration_ms) = self.duration_ms {
            span.ended_at_ms = Some(self.timestamp_ms + duration_ms);
        }
        span = span.with_attribute("observation_id", self.id);
        span = span.with_attribute("observation_kind", format!("{:?}", self.kind));
        if let Some(success) = self.success {
            span = span.with_attribute("success", success);
        }
        if let Some(input_tokens) = self.input_tokens {
            span = span.with_attribute("input_tokens", input_tokens);
        }
        if let Some(output_tokens) = self.output_tokens {
            span = span.with_attribute("output_tokens", output_tokens);
        }
        for (key, value) in &self.metadata {
            span.attributes.insert(key.clone(), value.clone());
        }
        span
    }

    fn add_common_otlp_attributes(&self, mut span: OtlpSpan) -> OtlpSpan {
        span.started_at_ms = self.timestamp_ms;
        if let Some(duration_ms) = self.duration_ms {
            span.ended_at_ms = Some(self.timestamp_ms + duration_ms);
        }
        span = span.with_attribute("observation.id", self.id);
        span = span.with_attribute("observation.kind", format!("{:?}", self.kind));
        if let Some(success) = self.success {
            span = span.with_attribute("success", success);
        }
        if let Some(input_tokens) = self.input_tokens {
            span = span.with_attribute("llm.input_tokens", input_tokens);
        }
        if let Some(output_tokens) = self.output_tokens {
            span = span.with_attribute("llm.output_tokens", output_tokens);
        }
        for (key, value) in &self.metadata {
            span.attributes.insert(key.clone(), value.clone());
        }
        span
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{InMemoryLangfuseExporter, InMemoryOtlpExporter, LangfuseConfig, OtlpConfig};

    #[test]
    fn keeps_only_the_most_recent_observations() {
        let mut layer = ObservationLayer::new(2);

        layer.record_agent_turn("trace", 1, Duration::from_millis(1));
        let second = layer.record_tool_call("trace", "read_file", Duration::from_millis(2), true);
        let third = layer.record_llm_request("trace", "gpt-4", Duration::from_millis(3), 10, 5);

        let ids = layer
            .observations()
            .map(|observation| observation.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![second, third]);
    }

    #[test]
    fn records_observation_metadata() {
        let mut layer = ObservationLayer::new(10);

        layer.record_agent_turn("trace", 3, Duration::from_millis(9));
        layer.record_tool_call("trace", "terminal", Duration::from_millis(4), false);
        layer.record_llm_request("trace", "gpt-4", Duration::from_millis(5), 100, 25);

        let observations = layer.observations().collect::<Vec<_>>();
        assert_eq!(observations[0].metadata["message_count"], 3);
        assert_eq!(observations[1].success, Some(false));
        assert_eq!(observations[1].metadata["tool_name"], "terminal");
        assert_eq!(observations[2].input_tokens, Some(100));
        assert_eq!(observations[2].output_tokens, Some(25));
        assert_eq!(observations[2].metadata["model"], "gpt-4");
    }

    #[test]
    fn exports_observations_to_langfuse_and_otlp() {
        let mut layer = ObservationLayer::new(10);
        layer.record_agent_turn("trace", 2, Duration::from_millis(1));
        layer.record_tool_call("trace", "read_file", Duration::from_millis(2), true);
        layer.record_llm_request("trace", "gpt-4", Duration::from_millis(3), 10, 5);

        let langfuse_exporter = InMemoryLangfuseExporter::default();
        let langfuse = LangfuseBackend::new(
            LangfuseConfig {
                enabled: true,
                endpoint: "https://langfuse.test".into(),
                public_key: Some("public".into()),
                secret_key: Some("secret".into()),
                environment: None,
                release: None,
            },
            langfuse_exporter.clone(),
        );

        let otlp_exporter = InMemoryOtlpExporter::default();
        let otlp = OtlpBackend::new(
            OtlpConfig {
                enabled: true,
                endpoint: "http://collector.test/v1/traces".into(),
                authorization: None,
                service_name: "sim-test".into(),
                resource_attributes: HashMap::new(),
            },
            otlp_exporter.clone(),
        );

        layer.export_to_langfuse(&langfuse).unwrap();
        layer.export_to_otlp(&otlp).unwrap();

        assert_eq!(langfuse_exporter.exports().len(), 3);
        assert_eq!(otlp_exporter.exports().len(), 3);
        assert_eq!(
            langfuse_exporter.exports()[2].spans[0].attributes["model"],
            "gpt-4"
        );
        assert_eq!(
            otlp_exporter.exports()[2].spans[0].attributes["llm.input_tokens"],
            10
        );
    }
}
