use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LangfuseConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub public_key: Option<String>,
    pub secret_key: Option<String>,
    pub environment: Option<String>,
    pub release: Option<String>,
}

impl LangfuseConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            endpoint: "https://cloud.langfuse.com".to_string(),
            public_key: None,
            secret_key: None,
            environment: None,
            release: None,
        }
    }

    pub fn is_configured(&self) -> bool {
        self.enabled && self.public_key.is_some() && self.secret_key.is_some()
    }

    pub fn from_env() -> Self {
        let enabled = std::env::var("SIM_LANGFUSE_ENABLED")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);
        Self {
            enabled,
            endpoint: std::env::var("SIM_LANGFUSE_ENDPOINT")
                .unwrap_or_else(|_| "https://cloud.langfuse.com".to_string()),
            public_key: std::env::var("SIM_LANGFUSE_PUBLIC_KEY").ok(),
            secret_key: std::env::var("SIM_LANGFUSE_SECRET_KEY").ok(),
            environment: std::env::var("SIM_LANGFUSE_ENVIRONMENT").ok(),
            release: std::env::var("SIM_LANGFUSE_RELEASE").ok(),
        }
    }
}

impl Default for LangfuseConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LangfuseSpanKind {
    LlmCall,
    ToolCall,
    AgentTurn,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LangfuseSpan {
    pub id: String,
    pub trace_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub kind: LangfuseSpanKind,
    pub started_at_ms: u128,
    pub ended_at_ms: Option<u128>,
    pub attributes: Map<String, Value>,
}

impl LangfuseSpan {
    pub fn new(
        trace_id: impl Into<String>,
        id: impl Into<String>,
        name: impl Into<String>,
        kind: LangfuseSpanKind,
    ) -> Self {
        Self {
            id: id.into(),
            trace_id: trace_id.into(),
            parent_id: None,
            name: name.into(),
            kind,
            started_at_ms: now_ms(),
            ended_at_ms: None,
            attributes: Map::new(),
        }
    }

    pub fn llm_call(
        trace_id: impl Into<String>,
        id: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let model = model.into();
        Self::new(
            trace_id,
            id,
            format!("llm:{model}"),
            LangfuseSpanKind::LlmCall,
        )
        .with_attribute("model", model)
    }

    pub fn tool_call(
        trace_id: impl Into<String>,
        id: impl Into<String>,
        tool_name: impl Into<String>,
    ) -> Self {
        let tool_name = tool_name.into();
        Self::new(
            trace_id,
            id,
            format!("tool:{tool_name}"),
            LangfuseSpanKind::ToolCall,
        )
        .with_attribute("tool_name", tool_name)
    }

    pub fn agent_turn(trace_id: impl Into<String>, id: impl Into<String>) -> Self {
        Self::new(trace_id, id, "agent_turn", LangfuseSpanKind::AgentTurn)
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(value) = serde_json::to_value(value) {
            self.attributes.insert(key.into(), value);
        }
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
pub struct LangfuseTraceExport {
    pub trace_id: String,
    pub spans: Vec<LangfuseSpan>,
    pub environment: Option<String>,
    pub release: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LangfuseExportError {
    NotConfigured,
    Exporter(String),
}

impl fmt::Display for LangfuseExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => write!(formatter, "Langfuse backend is not configured"),
            Self::Exporter(message) => write!(formatter, "Langfuse exporter failed: {message}"),
        }
    }
}

impl std::error::Error for LangfuseExportError {}

pub trait LangfuseExporter: Send + Sync {
    fn export(
        &self,
        config: &LangfuseConfig,
        trace: LangfuseTraceExport,
    ) -> Result<(), LangfuseExportError>;
}

#[derive(Default)]
pub struct NoopLangfuseExporter;

impl LangfuseExporter for NoopLangfuseExporter {
    fn export(
        &self,
        _config: &LangfuseConfig,
        _trace: LangfuseTraceExport,
    ) -> Result<(), LangfuseExportError> {
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct InMemoryLangfuseExporter {
    exports: Arc<Mutex<Vec<LangfuseTraceExport>>>,
}

impl InMemoryLangfuseExporter {
    pub fn exports(&self) -> Vec<LangfuseTraceExport> {
        self.exports
            .lock()
            .map(|exports| exports.clone())
            .unwrap_or_default()
    }
}

impl LangfuseExporter for InMemoryLangfuseExporter {
    fn export(
        &self,
        _config: &LangfuseConfig,
        trace: LangfuseTraceExport,
    ) -> Result<(), LangfuseExportError> {
        self.exports
            .lock()
            .map_err(|error| LangfuseExportError::Exporter(error.to_string()))?
            .push(trace);
        Ok(())
    }
}

pub struct LangfuseBackend<E> {
    config: LangfuseConfig,
    exporter: E,
}

impl<E> LangfuseBackend<E>
where
    E: LangfuseExporter,
{
    pub fn new(config: LangfuseConfig, exporter: E) -> Self {
        Self { config, exporter }
    }

    pub fn config(&self) -> &LangfuseConfig {
        &self.config
    }

    pub fn record_span(&self, span: LangfuseSpan) -> Result<(), LangfuseExportError> {
        if !self.config.enabled {
            return Ok(());
        }
        if !self.config.is_configured() {
            return Err(LangfuseExportError::NotConfigured);
        }
        let trace = LangfuseTraceExport {
            trace_id: span.trace_id.clone(),
            spans: vec![span],
            environment: self.config.environment.clone(),
            release: self.config.release.clone(),
        };
        self.exporter.export(&self.config, trace)
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

    fn enabled_config() -> LangfuseConfig {
        LangfuseConfig {
            enabled: true,
            endpoint: "https://langfuse.test".into(),
            public_key: Some("public".into()),
            secret_key: Some("secret".into()),
            environment: Some("test".into()),
            release: Some("abc123".into()),
        }
    }

    #[test]
    fn disabled_backend_does_not_export() {
        let exporter = InMemoryLangfuseExporter::default();
        let backend = LangfuseBackend::new(LangfuseConfig::disabled(), exporter.clone());

        backend
            .record_span(LangfuseSpan::agent_turn("trace", "span"))
            .unwrap();

        assert!(exporter.exports().is_empty());
    }

    #[test]
    fn enabled_backend_requires_keys() {
        let backend = LangfuseBackend::new(
            LangfuseConfig {
                enabled: true,
                ..LangfuseConfig::disabled()
            },
            NoopLangfuseExporter,
        );

        assert_eq!(
            backend
                .record_span(LangfuseSpan::agent_turn("trace", "span"))
                .unwrap_err(),
            LangfuseExportError::NotConfigured
        );
    }

    #[test]
    fn records_llm_tool_and_agent_spans() {
        let exporter = InMemoryLangfuseExporter::default();
        let backend = LangfuseBackend::new(enabled_config(), exporter.clone());

        backend
            .record_span(
                LangfuseSpan::llm_call("trace", "llm", "gpt-4")
                    .with_parent("turn")
                    .with_attribute("input_tokens", 42_u64)
                    .finish_with_duration(Duration::from_millis(15)),
            )
            .unwrap();
        backend
            .record_span(LangfuseSpan::tool_call("trace", "tool", "read_file").finish())
            .unwrap();
        backend
            .record_span(LangfuseSpan::agent_turn("trace", "turn"))
            .unwrap();

        let exports = exporter.exports();
        assert_eq!(exports.len(), 3);
        assert_eq!(exports[0].trace_id, "trace");
        assert_eq!(exports[0].environment.as_deref(), Some("test"));
        assert_eq!(exports[0].spans[0].kind, LangfuseSpanKind::LlmCall);
        assert_eq!(exports[0].spans[0].parent_id.as_deref(), Some("turn"));
        assert_eq!(
            exports[0].spans[0].attributes["model"],
            Value::String("gpt-4".into())
        );
        assert_eq!(exports[1].spans[0].kind, LangfuseSpanKind::ToolCall);
        assert_eq!(exports[2].spans[0].kind, LangfuseSpanKind::AgentTurn);
    }
}
