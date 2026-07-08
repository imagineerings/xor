use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const DEFAULT_HOST: &str = "https://app.posthog.com";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostHogConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
    pub host: String,
    pub distinct_id: String,
    pub default_properties: Map<String, Value>,
}

impl PostHogConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            api_key: None,
            host: DEFAULT_HOST.to_string(),
            distinct_id: "anonymous".to_string(),
            default_properties: Map::new(),
        }
    }

    pub fn new(api_key: impl Into<String>, distinct_id: impl Into<String>) -> Self {
        Self {
            enabled: true,
            api_key: Some(api_key.into()),
            host: DEFAULT_HOST.to_string(),
            distinct_id: distinct_id.into(),
            default_properties: Map::new(),
        }
    }

    pub fn from_env() -> Self {
        let enabled = std::env::var("SIM_POSTHOG_ENABLED")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);

        Self {
            enabled,
            api_key: std::env::var("SIM_POSTHOG_API_KEY").ok(),
            host: std::env::var("SIM_POSTHOG_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string()),
            distinct_id: std::env::var("SIM_POSTHOG_DISTINCT_ID")
                .unwrap_or_else(|_| "anonymous".to_string()),
            default_properties: Map::new(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.enabled
            && self
                .api_key
                .as_ref()
                .is_some_and(|api_key| !api_key.is_empty())
    }

    pub fn capture_endpoint(&self) -> String {
        format!("{}/capture/", self.host.trim_end_matches('/'))
    }

    pub fn identify_endpoint(&self) -> String {
        format!("{}/identify/", self.host.trim_end_matches('/'))
    }

    pub fn with_default_property(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(value) = serde_json::to_value(value) {
            self.default_properties.insert(key.into(), value);
        }
        self
    }
}

impl Default for PostHogConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PostHogUserAction {
    AgentTurnStarted,
    AgentTurnCompleted,
    ToolInvoked,
    SettingsOpened,
    Custom(String),
}

impl PostHogUserAction {
    pub fn event_name(&self) -> String {
        match self {
            Self::AgentTurnStarted => "agent_turn_started".to_string(),
            Self::AgentTurnCompleted => "agent_turn_completed".to_string(),
            Self::ToolInvoked => "tool_invoked".to_string(),
            Self::SettingsOpened => "settings_opened".to_string(),
            Self::Custom(event) => event.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PostHogCaptureRequest {
    pub api_key: String,
    pub event: String,
    pub properties: Map<String, Value>,
    pub timestamp_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PostHogIdentifyRequest {
    pub api_key: String,
    pub distinct_id: String,
    pub properties: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PostHogCaptureOutcome {
    Disabled,
    Sent,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PostHogError {
    #[error("PostHog is enabled but missing an API key")]
    MissingApiKey,
    #[error("PostHog sender failed: {0}")]
    Sender(String),
}

pub trait PostHogSender: Send + Sync {
    fn capture(&self, endpoint: &str, request: PostHogCaptureRequest) -> Result<(), PostHogError>;

    fn identify(&self, endpoint: &str, request: PostHogIdentifyRequest)
    -> Result<(), PostHogError>;

    fn flush(&self) -> Result<(), PostHogError> {
        Ok(())
    }
}

#[derive(Default)]
pub struct NoopPostHogSender;

impl PostHogSender for NoopPostHogSender {
    fn capture(
        &self,
        _endpoint: &str,
        _request: PostHogCaptureRequest,
    ) -> Result<(), PostHogError> {
        Ok(())
    }

    fn identify(
        &self,
        _endpoint: &str,
        _request: PostHogIdentifyRequest,
    ) -> Result<(), PostHogError> {
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct InMemoryPostHogSender {
    captures: Arc<Mutex<Vec<(String, PostHogCaptureRequest)>>>,
    identifies: Arc<Mutex<Vec<(String, PostHogIdentifyRequest)>>>,
    flush_count: Arc<Mutex<usize>>,
}

impl InMemoryPostHogSender {
    pub fn captures(&self) -> Vec<(String, PostHogCaptureRequest)> {
        self.captures
            .lock()
            .map(|captures| captures.clone())
            .unwrap_or_default()
    }

    pub fn identifies(&self) -> Vec<(String, PostHogIdentifyRequest)> {
        self.identifies
            .lock()
            .map(|identifies| identifies.clone())
            .unwrap_or_default()
    }

    pub fn flush_count(&self) -> usize {
        self.flush_count
            .lock()
            .map(|flush_count| *flush_count)
            .unwrap_or_default()
    }
}

impl PostHogSender for InMemoryPostHogSender {
    fn capture(&self, endpoint: &str, request: PostHogCaptureRequest) -> Result<(), PostHogError> {
        self.captures
            .lock()
            .map_err(|error| PostHogError::Sender(error.to_string()))?
            .push((endpoint.to_string(), request));
        Ok(())
    }

    fn identify(
        &self,
        endpoint: &str,
        request: PostHogIdentifyRequest,
    ) -> Result<(), PostHogError> {
        self.identifies
            .lock()
            .map_err(|error| PostHogError::Sender(error.to_string()))?
            .push((endpoint.to_string(), request));
        Ok(())
    }

    fn flush(&self) -> Result<(), PostHogError> {
        *self
            .flush_count
            .lock()
            .map_err(|error| PostHogError::Sender(error.to_string()))? += 1;
        Ok(())
    }
}

#[derive(Clone)]
pub struct PostHogClient {
    config: PostHogConfig,
    sender: Arc<dyn PostHogSender>,
}

impl PostHogClient {
    pub fn new(config: PostHogConfig) -> Self {
        Self::with_sender(config, NoopPostHogSender)
    }

    pub fn with_sender(config: PostHogConfig, sender: impl PostHogSender + 'static) -> Self {
        Self {
            config,
            sender: Arc::new(sender),
        }
    }

    pub fn config(&self) -> &PostHogConfig {
        &self.config
    }

    pub fn capture(
        &self,
        event: &str,
        properties: Map<String, Value>,
    ) -> Result<PostHogCaptureOutcome, PostHogError> {
        let Some(api_key) = self.api_key()? else {
            return Ok(PostHogCaptureOutcome::Disabled);
        };

        let mut properties = sanitized_properties(properties);
        for (key, value) in sanitized_properties(self.config.default_properties.clone()) {
            properties.entry(key).or_insert(value);
        }
        properties.insert(
            "distinct_id".to_string(),
            Value::String(self.config.distinct_id.clone()),
        );

        self.sender.capture(
            &self.config.capture_endpoint(),
            PostHogCaptureRequest {
                api_key,
                event: event.to_string(),
                properties,
                timestamp_ms: now_ms(),
            },
        )?;
        Ok(PostHogCaptureOutcome::Sent)
    }

    pub fn capture_user_action(
        &self,
        action: PostHogUserAction,
        properties: Map<String, Value>,
    ) -> Result<PostHogCaptureOutcome, PostHogError> {
        self.capture(&action.event_name(), properties)
    }

    pub fn identify(
        &self,
        properties: Map<String, Value>,
    ) -> Result<PostHogCaptureOutcome, PostHogError> {
        let Some(api_key) = self.api_key()? else {
            return Ok(PostHogCaptureOutcome::Disabled);
        };

        self.sender.identify(
            &self.config.identify_endpoint(),
            PostHogIdentifyRequest {
                api_key,
                distinct_id: self.config.distinct_id.clone(),
                properties: sanitized_properties(properties),
            },
        )?;
        Ok(PostHogCaptureOutcome::Sent)
    }

    pub fn flush(&self) -> Result<(), PostHogError> {
        self.sender.flush()
    }

    fn api_key(&self) -> Result<Option<String>, PostHogError> {
        if !self.config.enabled {
            return Ok(None);
        }

        match self.config.api_key.as_ref() {
            Some(api_key) if !api_key.is_empty() => Ok(Some(api_key.clone())),
            _ => Err(PostHogError::MissingApiKey),
        }
    }
}

pub fn sanitized_properties(properties: Map<String, Value>) -> Map<String, Value> {
    properties
        .into_iter()
        .filter_map(|(key, value)| sanitize_property(key, value))
        .collect()
}

fn sanitize_property(key: String, value: Value) -> Option<(String, Value)> {
    if is_sensitive_key(&key) {
        return None;
    }

    Some((key.clone(), sanitize_value(&key, value)))
}

fn sanitize_value(key: &str, value: Value) -> Value {
    match value {
        Value::String(value) if is_safe_string_key(key) => Value::String(value),
        Value::String(_) => Value::String("[redacted]".to_string()),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| sanitize_value(key, value))
                .collect(),
        ),
        Value::Object(object) => Value::Object(sanitized_properties(object)),
        value @ (Value::Null | Value::Bool(_) | Value::Number(_)) => value,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "email", "name", "username", "phone", "address", "ip", "token", "api_key", "secret",
        "password", "prompt", "message", "input", "output", "content", "path", "file", "cwd",
        "url",
    ]
    .iter()
    .any(|sensitive| key.contains(sensitive))
}

fn is_safe_string_key(key: &str) -> bool {
    matches!(
        key,
        "action"
            | "agent"
            | "error_code"
            | "feature"
            | "model"
            | "provider"
            | "release_channel"
            | "source"
            | "status"
            | "surface"
            | "tool"
            | "version"
    )
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
    use serde_json::json;

    fn properties(pairs: &[(&str, Value)]) -> Map<String, Value> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn disabled_client_does_not_send() {
        let sender = InMemoryPostHogSender::default();
        let client = PostHogClient::with_sender(PostHogConfig::disabled(), sender.clone());

        let outcome = client
            .capture("agent_turn_started", Map::new())
            .expect("disabled client should not fail capture");

        assert_eq!(outcome, PostHogCaptureOutcome::Disabled);
        assert!(sender.captures().is_empty());
    }

    #[test]
    fn capture_formats_posthog_request() {
        let sender = InMemoryPostHogSender::default();
        let config = PostHogConfig::new("test-key", "install-1")
            .with_default_property("release_channel", "preview");
        let client = PostHogClient::with_sender(config, sender.clone());

        let outcome = client
            .capture_user_action(
                PostHogUserAction::ToolInvoked,
                properties(&[
                    ("tool", json!("shell")),
                    ("success", json!(true)),
                    ("duration_ms", json!(42)),
                ]),
            )
            .expect("capture should send");

        assert_eq!(outcome, PostHogCaptureOutcome::Sent);
        let captures = sender.captures();
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].0, "https://app.posthog.com/capture/");
        assert_eq!(captures[0].1.api_key, "test-key");
        assert_eq!(captures[0].1.event, "tool_invoked");
        assert_eq!(
            captures[0].1.properties.get("distinct_id"),
            Some(&json!("install-1"))
        );
        assert_eq!(captures[0].1.properties.get("tool"), Some(&json!("shell")));
        assert_eq!(
            captures[0].1.properties.get("release_channel"),
            Some(&json!("preview"))
        );
    }

    #[test]
    fn capture_requires_api_key_when_enabled() {
        let sender = InMemoryPostHogSender::default();
        let mut config = PostHogConfig::disabled();
        config.enabled = true;
        let client = PostHogClient::with_sender(config, sender);

        assert_eq!(
            client.capture("event", Map::new()),
            Err(PostHogError::MissingApiKey)
        );
    }

    #[test]
    fn properties_are_sanitized_before_capture() {
        let sender = InMemoryPostHogSender::default();
        let client =
            PostHogClient::with_sender(PostHogConfig::new("key", "install"), sender.clone());

        client
            .capture(
                "agent_turn_completed",
                properties(&[
                    ("model", json!("gpt-test")),
                    ("prompt", json!("secret prompt")),
                    ("freeform", json!("possibly identifying")),
                    (
                        "metadata",
                        json!({"email": "person@example.com", "status": "ok"}),
                    ),
                ]),
            )
            .expect("capture should send");

        let captures = sender.captures();
        let properties = &captures[0].1.properties;
        assert_eq!(properties.get("model"), Some(&json!("gpt-test")));
        assert!(!properties.contains_key("prompt"));
        assert_eq!(properties.get("freeform"), Some(&json!("[redacted]")));
        assert_eq!(properties.get("metadata"), Some(&json!({"status": "ok"})));
    }

    #[test]
    fn identify_uses_sanitized_properties() {
        let sender = InMemoryPostHogSender::default();
        let mut config = PostHogConfig::new("key", "install");
        config.host = "https://posthog.example/".to_string();
        let client = PostHogClient::with_sender(config, sender.clone());

        client
            .identify(properties(&[
                ("provider", json!("openai")),
                ("email", json!("person@example.com")),
            ]))
            .expect("identify should send");

        let identifies = sender.identifies();
        assert_eq!(identifies.len(), 1);
        assert_eq!(identifies[0].0, "https://posthog.example/identify/");
        assert_eq!(identifies[0].1.distinct_id, "install");
        assert_eq!(
            identifies[0].1.properties,
            properties(&[("provider", json!("openai"))])
        );
    }

    #[test]
    fn flush_delegates_to_sender() {
        let sender = InMemoryPostHogSender::default();
        let client = PostHogClient::with_sender(PostHogConfig::disabled(), sender.clone());

        client.flush().expect("flush should delegate to sender");

        assert_eq!(sender.flush_count(), 1);
    }
}
