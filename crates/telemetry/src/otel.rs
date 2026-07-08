use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtlpConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub authorization: Option<String>,
    pub service_name: String,
    pub resource_attributes: HashMap<String, String>,
}

impl OtlpConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            endpoint: "http://127.0.0.1:4318/v1/traces".to_string(),
            authorization: None,
            service_name: "sim".to_string(),
            resource_attributes: HashMap::new(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.enabled && !self.endpoint.trim().is_empty()
    }

    pub fn from_env() -> Self {
        let enabled = std::env::var("SIM_OTLP_ENABLED")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);
        Self {
            enabled,
            endpoint: std::env::var("SIM_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://127.0.0.1:4318/v1/traces".to_string()),
            authorization: std::env::var("SIM_OTLP_AUTHORIZATION").ok(),
            service_name: std::env::var("SIM_OTLP_SERVICE_NAME")
                .unwrap_or_else(|_| "sim".to_string()),
            resource_attributes: HashMap::new(),
        }
    }
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OtlpSpanKind {
    Internal,
    Client,
    Server,
    Producer,
    Consumer,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OtlpSpanEvent {
    pub name: String,
    pub timestamp_ms: u128,
    pub attributes: Map<String, Value>,
}

impl OtlpSpanEvent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timestamp_ms: now_ms(),
            attributes: Map::new(),
        }
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(value) = serde_json::to_value(value) {
            self.attributes.insert(key.into(), value);
        }
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OtlpSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub kind: OtlpSpanKind,
    pub started_at_ms: u128,
    pub ended_at_ms: Option<u128>,
    pub attributes: Map<String, Value>,
    pub events: Vec<OtlpSpanEvent>,
}

impl OtlpSpan {
    pub fn new(
        trace_id: impl Into<String>,
        span_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            parent_span_id: None,
            name: name.into(),
            kind: OtlpSpanKind::Internal,
            started_at_ms: now_ms(),
            ended_at_ms: None,
            attributes: Map::new(),
            events: Vec::new(),
        }
    }

    pub fn llm_request(
        trace_id: impl Into<String>,
        span_id: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let model = model.into();
        Self::new(trace_id, span_id, format!("llm.request {model}"))
            .with_kind(OtlpSpanKind::Client)
            .with_attribute("llm.model", model)
    }

    pub fn tool_call(
        trace_id: impl Into<String>,
        span_id: impl Into<String>,
        tool_name: impl Into<String>,
    ) -> Self {
        let tool_name = tool_name.into();
        Self::new(trace_id, span_id, format!("tool.call {tool_name}"))
            .with_attribute("tool.name", tool_name)
    }

    pub fn agent_turn(trace_id: impl Into<String>, span_id: impl Into<String>) -> Self {
        Self::new(trace_id, span_id, "agent.turn")
    }

    pub fn with_kind(mut self, kind: OtlpSpanKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_parent(mut self, parent_span_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_span_id.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(value) = serde_json::to_value(value) {
            self.attributes.insert(key.into(), value);
        }
        self
    }

    pub fn with_event(mut self, event: OtlpSpanEvent) -> Self {
        self.events.push(event);
        self
    }

    pub fn finish(mut self) -> Self {
        self.ended_at_ms = Some(now_ms());
        self
    }

    pub fn finish_with_duration(mut self, duration: Duration) -> Self {
        self.ended_at_ms = Some(self.started_at_ms + duration.as_millis());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OtlpTraceExport {
    pub service_name: String,
    pub resource_attributes: HashMap<String, String>,
    pub spans: Vec<OtlpSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtlpExportError {
    NotConfigured,
    Exporter(String),
}

impl fmt::Display for OtlpExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => write!(formatter, "OTLP backend is not configured"),
            Self::Exporter(message) => write!(formatter, "OTLP exporter failed: {message}"),
        }
    }
}

impl std::error::Error for OtlpExportError {}

pub trait OtlpExporter: Send + Sync {
    fn export(&self, config: &OtlpConfig, trace: OtlpTraceExport) -> Result<(), OtlpExportError>;
}

#[derive(Default)]
pub struct NoopOtlpExporter;

impl OtlpExporter for NoopOtlpExporter {
    fn export(&self, _config: &OtlpConfig, _trace: OtlpTraceExport) -> Result<(), OtlpExportError> {
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct InMemoryOtlpExporter {
    exports: Arc<Mutex<Vec<OtlpTraceExport>>>,
}

impl InMemoryOtlpExporter {
    pub fn exports(&self) -> Vec<OtlpTraceExport> {
        self.exports
            .lock()
            .map(|exports| exports.clone())
            .unwrap_or_default()
    }
}

impl OtlpExporter for InMemoryOtlpExporter {
    fn export(&self, _config: &OtlpConfig, trace: OtlpTraceExport) -> Result<(), OtlpExportError> {
        self.exports
            .lock()
            .map_err(|error| OtlpExportError::Exporter(error.to_string()))?
            .push(trace);
        Ok(())
    }
}

pub struct OtlpBackend<E> {
    config: OtlpConfig,
    exporter: E,
}

impl<E> OtlpBackend<E>
where
    E: OtlpExporter,
{
    pub fn new(config: OtlpConfig, exporter: E) -> Self {
        Self { config, exporter }
    }

    pub fn config(&self) -> &OtlpConfig {
        &self.config
    }

    pub fn record_span(&self, span: OtlpSpan) -> Result<(), OtlpExportError> {
        if !self.config.enabled {
            return Ok(());
        }
        if !self.config.is_configured() {
            return Err(OtlpExportError::NotConfigured);
        }
        self.exporter.export(
            &self.config,
            OtlpTraceExport {
                service_name: self.config.service_name.clone(),
                resource_attributes: self.config.resource_attributes.clone(),
                spans: vec![span],
            },
        )
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
    use super::*;

    fn enabled_config() -> OtlpConfig {
        OtlpConfig {
            enabled: true,
            endpoint: "http://collector.test/v1/traces".into(),
            authorization: Some("Bearer token".into()),
            service_name: "sim-test".into(),
            resource_attributes: HashMap::from([("deployment.environment".into(), "test".into())]),
        }
    }

    #[test]
    fn disabled_backend_does_not_export() {
        let exporter = InMemoryOtlpExporter::default();
        let backend = OtlpBackend::new(OtlpConfig::disabled(), exporter.clone());

        backend
            .record_span(OtlpSpan::agent_turn("trace", "span"))
            .unwrap();

        assert!(exporter.exports().is_empty());
    }

    #[test]
    fn enabled_backend_requires_endpoint() {
        let backend = OtlpBackend::new(
            OtlpConfig {
                enabled: true,
                endpoint: String::new(),
                ..OtlpConfig::disabled()
            },
            NoopOtlpExporter,
        );

        assert_eq!(
            backend
                .record_span(OtlpSpan::agent_turn("trace", "span"))
                .unwrap_err(),
            OtlpExportError::NotConfigured
        );
    }

    #[test]
    fn records_llm_tool_and_agent_spans() {
        let exporter = InMemoryOtlpExporter::default();
        let backend = OtlpBackend::new(enabled_config(), exporter.clone());

        backend
            .record_span(
                OtlpSpan::llm_request("trace", "llm", "gpt-4")
                    .with_parent("turn")
                    .with_attribute("llm.input_tokens", 42_u64)
                    .with_event(OtlpSpanEvent::new("stream.started"))
                    .finish_with_duration(Duration::from_millis(15)),
            )
            .unwrap();
        backend
            .record_span(OtlpSpan::tool_call("trace", "tool", "read_file").finish())
            .unwrap();
        backend
            .record_span(OtlpSpan::agent_turn("trace", "turn"))
            .unwrap();

        let exports = exporter.exports();
        assert_eq!(exports.len(), 3);
        assert_eq!(exports[0].service_name, "sim-test");
        assert_eq!(
            exports[0].resource_attributes["deployment.environment"],
            "test"
        );
        assert_eq!(exports[0].spans[0].kind, OtlpSpanKind::Client);
        assert_eq!(exports[0].spans[0].parent_span_id.as_deref(), Some("turn"));
        assert_eq!(
            exports[0].spans[0].attributes["llm.model"],
            Value::String("gpt-4".into())
        );
        assert_eq!(exports[0].spans[0].events[0].name, "stream.started");
        assert_eq!(exports[1].spans[0].name, "tool.call read_file");
        assert_eq!(exports[2].spans[0].name, "agent.turn");
    }
}
