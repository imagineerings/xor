//! OAuth 2.0 Device Authorization Grant (Device Flow) handler.
//!
//! Implements [RFC 8628](https://datatracker.ietf.org/doc/html/rfc8628)
//! for devices with limited input capabilities such as terminals, SSH
//! sessions, and containers.
//!
//! # Runtime requirements
//!
//! Both [`start_device_flow`] and [`poll_for_token`] / [`poll_once`] require
//! a Tokio runtime.  In the sim application this is always available via
//! `crates/reqwest_client`, which initialises a global Tokio runtime on
//! startup.

use crate::token_store::{OAuthTokens, TokenStore};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;

// ── DeviceFlowSession ────────────────────────────────────────────────────

/// Represents a device flow authorisation session returned by the
/// device-authorisation endpoint.
#[derive(Clone, Debug)]
pub struct DeviceFlowSession {
    /// The device verification code (used when polling).
    pub device_code: String,
    /// The end-user verification code (short, human-readable).
    pub user_code: String,
    /// The verification URI the user must visit in their browser.
    pub verification_uri: String,
    /// Complete verification URI that already includes the user code.
    pub verification_uri_complete: Option<String>,
    /// Minimum interval the client **should** wait between poll requests.
    pub interval: Duration,
    /// Absolute time at which the device code expires.
    pub expires_at: DateTime<Utc>,
}

// ── DeviceFlowError ──────────────────────────────────────────────────────

/// Errors specific to device-flow token polling.
#[derive(Clone, Debug)]
pub enum DeviceFlowError {
    /// The user hasn't completed authorisation yet – keep polling.
    AuthorizationPending,
    /// The server asks the client to increase the polling interval.
    SlowDown,
    /// The device code expired before the user authorised.
    ExpiredToken,
    /// The user denied the authorisation request.
    AuthorizationDeclined,
    /// An unexpected error occurred.
    Other(String),
}

impl std::fmt::Display for DeviceFlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthorizationPending => write!(f, "authorisation pending"),
            Self::SlowDown => write!(f, "slow down polling interval"),
            Self::ExpiredToken => write!(f, "device code expired"),
            Self::AuthorizationDeclined => write!(f, "authorisation declined"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for DeviceFlowError {}

// ── DeviceFlowHandler ────────────────────────────────────────────────────

/// Handler for OAuth 2.0 Device Authorisation Grant flows.
///
/// # Example
///
/// ```ignore
/// let handler = DeviceFlowHandler::new(
///     "my-client-id",
///     vec!["openid".into(), "profile".into()],
///     "https://provider.example.com/token",
///     "https://provider.example.com/auth/device",
/// );
///
/// let session = handler.start_device_flow().await?;
/// println!("{}", handler.display_instructions(&session));
///
/// let tokens = handler.poll_for_token(&session).await?;
/// // Use tokens.access_token ...
/// ```
pub struct DeviceFlowHandler {
    client_id: String,
    scopes: Vec<String>,
    token_url: String,
    device_auth_url: String,
    store: Option<Arc<dyn TokenStore>>,
    http_client: reqwest::Client,
}

impl DeviceFlowHandler {
    /// Create a new device flow handler.
    pub fn new(
        client_id: impl Into<String>,
        scopes: Vec<String>,
        token_url: impl Into<String>,
        device_auth_url: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            scopes,
            token_url: token_url.into(),
            device_auth_url: device_auth_url.into(),
            store: None,
            http_client: reqwest::Client::new(),
        }
    }

    /// Attach a [`TokenStore`] so that tokens are automatically persisted
    /// once the poll succeeds.
    pub fn with_token_store(mut self, store: Arc<dyn TokenStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Inject a custom HTTP client (e.g. for testing with a mock server).
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = client;
        self
    }

    /// Start a device authorisation flow.
    ///
    /// Makes a POST request to the configured device-authorisation endpoint
    /// and returns a [`DeviceFlowSession`] containing the user-facing code
    /// and verification URI.
    pub async fn start_device_flow(&self) -> Result<DeviceFlowSession> {
        let scope = self.scopes.join(" ");

        let params = [("client_id", self.client_id.as_str()), ("scope", &scope)];

        let resp = self
            .http_client
            .post(&self.device_auth_url)
            .form(&params)
            .send()
            .await
            .with_context(|| {
                format!(
                    "failed to send device auth request to {}",
                    self.device_auth_url
                )
            })?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .with_context(|| format!("failed to parse device auth response (status {status})"))?;

        if !status.is_success() {
            let error = body["error"].as_str().unwrap_or("unknown");
            let desc = body["error_description"].as_str().unwrap_or("");
            return Err(anyhow!("device auth failed: {error} {desc}"));
        }

        let device_code = body["device_code"]
            .as_str()
            .ok_or_else(|| anyhow!("missing 'device_code' in response"))?
            .to_string();
        let user_code = body["user_code"]
            .as_str()
            .ok_or_else(|| anyhow!("missing 'user_code' in response"))?
            .to_string();
        let verification_uri = body["verification_uri"]
            .as_str()
            .ok_or_else(|| anyhow!("missing 'verification_uri' in response"))?
            .to_string();
        let verification_uri_complete =
            body["verification_uri_complete"].as_str().map(String::from);
        let expires_in = body["expires_in"].as_u64().unwrap_or(1800);
        let interval = Duration::from_secs(body["interval"].as_u64().unwrap_or(5));

        let delta = chrono::TimeDelta::from_std(Duration::from_secs(expires_in))
            .context("expires_in value out of range")?;
        let expires_at = Utc::now() + delta;

        Ok(DeviceFlowSession {
            device_code,
            user_code,
            verification_uri,
            verification_uri_complete,
            interval,
            expires_at,
        })
    }

    /// Poll the token endpoint until the user completes authorisation.
    ///
    /// Returns [`OAuthTokens`] on success or a [`DeviceFlowError`] if the
    /// flow expires, is denied, or encounters an unrecoverable error.
    ///
    /// The method respects the server's polling interval and automatically
    /// adjusts it when the server sends a `slow_down` response.  Polling
    /// stops once the session's `expires_at` is reached.
    pub async fn poll_for_token(
        &self,
        session: &DeviceFlowSession,
    ) -> std::result::Result<OAuthTokens, DeviceFlowError> {
        let mut interval = session.interval;

        loop {
            if Utc::now() > session.expires_at {
                return Err(DeviceFlowError::ExpiredToken);
            }

            match self.poll_once_inner(session).await {
                Ok(Some(tokens)) => {
                    return Ok(tokens);
                }
                Ok(None) => {
                    // Still pending – sleep and retry.
                    tokio::time::sleep(interval).await;
                }
                Err(DeviceFlowError::SlowDown) => {
                    interval += Duration::from_secs(5);
                    tokio::time::sleep(interval).await;
                }
                Err(e @ DeviceFlowError::ExpiredToken)
                | Err(e @ DeviceFlowError::AuthorizationDeclined) => {
                    return Err(e);
                }
                Err(DeviceFlowError::AuthorizationPending) => {
                    tokio::time::sleep(interval).await;
                }
                Err(DeviceFlowError::Other(msg)) => {
                    return Err(DeviceFlowError::Other(msg));
                }
            }
        }
    }

    /// Make a single poll attempt against the token endpoint.
    ///
    /// - `Ok(Some(tokens))` – authorisation completed.
    /// - `Ok(None)` – still pending (caller should sleep and retry).
    /// - `Err(DeviceFlowError)` – terminal error or `SlowDown`.
    pub async fn poll_once(
        &self,
        session: &DeviceFlowSession,
    ) -> std::result::Result<Option<OAuthTokens>, DeviceFlowError> {
        self.poll_once_inner(session).await
    }

    /// Shared poll logic used by both [`poll_for_token`] and [`poll_once`].
    async fn poll_once_inner(
        &self,
        session: &DeviceFlowSession,
    ) -> std::result::Result<Option<OAuthTokens>, DeviceFlowError> {
        if Utc::now() > session.expires_at {
            return Err(DeviceFlowError::ExpiredToken);
        }

        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", &session.device_code),
            ("client_id", &self.client_id),
        ];

        let resp = self
            .http_client
            .post(&self.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| DeviceFlowError::Other(format!("HTTP request failed: {e}")))?;

        if resp.status().is_success() {
            let tokens = parse_token_response(resp)
                .await
                .map_err(|e| DeviceFlowError::Other(format!("{e}")))?;

            // Auto-store if a token store is configured.
            if let Some(store) = &self.store {
                let key = format!("device_flow:{}", self.client_id);
                let _ = store.store(&key, &tokens);
            }

            return Ok(Some(tokens));
        }

        let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        let error = body["error"].as_str().unwrap_or("unknown_error");

        match error {
            "authorization_pending" => Err(DeviceFlowError::AuthorizationPending),
            "slow_down" => Err(DeviceFlowError::SlowDown),
            "expired_token" => Err(DeviceFlowError::ExpiredToken),
            "access_denied" | "authorization_declined" => {
                Err(DeviceFlowError::AuthorizationDeclined)
            }
            other => Err(DeviceFlowError::Other(format!(
                "unexpected error from token endpoint: {other}"
            ))),
        }
    }

    /// Format user-facing instructions for the device flow.
    ///
    /// Returns a multi-line string telling the user to visit the verification
    /// URI and enter the displayed code.
    pub fn display_instructions(&self, session: &DeviceFlowSession) -> String {
        match &session.verification_uri_complete {
            Some(complete) => {
                format!(
                    "To authorise, visit:\n\n  {complete}\n\n\
                     Or manually go to:\n  {}\nand enter the code: {}",
                    session.verification_uri, session.user_code,
                )
            }
            None => {
                format!(
                    "To authorise, visit:\n\n  {}\n\nand enter the code: {}",
                    session.verification_uri, session.user_code,
                )
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Parse an OAuth token response (JSON) into [`OAuthTokens`].
async fn parse_token_response(resp: reqwest::Response) -> Result<OAuthTokens> {
    let body: serde_json::Value = resp
        .json()
        .await
        .context("failed to parse token response")?;
    tokens_from_json(body)
}

/// Extract [`OAuthTokens`] from a JSON value (shared by production and test code).
fn tokens_from_json(body: serde_json::Value) -> Result<OAuthTokens> {
    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| anyhow!("missing 'access_token' in token response"))?
        .to_string();

    let refresh_token = body["refresh_token"].as_str().map(String::from);
    let expires_in = body["expires_in"].as_u64().map(Duration::from_secs);
    let token_type = body["token_type"].as_str().unwrap_or("Bearer").to_string();
    let scope = body["scope"].as_str().map(String::from);

    Ok(OAuthTokens {
        access_token,
        refresh_token,
        expires_in,
        token_type,
        scope,
        obtained_at: Some(Utc::now()),
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    // ── Mock OAuth server ────────────────────────────────────────────────

    /// A lightweight mock OAuth server for device-flow testing.
    ///
    /// The server runs on a background thread and accepts requests on a
    /// random loopback port.  Response behaviours are configured upfront
    /// via [`MockOAuthServerConfig`].
    struct MockOAuthServerConfig {
        /// JSON returned by `POST /auth/device`.
        device_auth_response: serde_json::Value,
        /// HTTP status for device auth response.
        device_auth_status: u16,
        /// Sequence of JSON responses returned by `POST /token`.
        token_responses: VecDeque<serde_json::Value>,
        /// HTTP status for token responses.
        token_status: u16,
    }

    struct MockOAuthServer {
        base_url: String,
        _shutdown: std::sync::mpsc::Sender<()>,
    }

    impl MockOAuthServer {
        /// Start a mock server on a random port and return a handle.
        fn start(config: MockOAuthServerConfig) -> Self {
            let server =
                tiny_http::Server::http("127.0.0.1:0").expect("failed to bind mock server");
            let port = server.server_addr().to_ip().unwrap().port();
            let base_url = format!("http://127.0.0.1:{port}");

            let config = Arc::new(Mutex::new(config));
            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                loop {
                    // Check for shutdown signal (non-blocking).
                    if rx.try_recv().is_ok() {
                        break;
                    }

                    // Accept with a short timeout so we can also check shutdown.
                    let req = match server.recv_timeout(std::time::Duration::from_millis(100)) {
                        Ok(Some(r)) => r,
                        Ok(None) | Err(_) => continue,
                    };

                    let url = req.url().to_string();
                    let mut cfg = config.lock().unwrap();

                    let (status, body) = if url.contains("/auth/device") {
                        (
                            cfg.device_auth_status,
                            serde_json::to_vec(&cfg.device_auth_response).unwrap(),
                        )
                    } else if url.contains("/token") {
                        let resp = cfg
                            .token_responses
                            .pop_front()
                            .unwrap_or(serde_json::json!({"error": "no_more_responses"}));
                        (cfg.token_status, serde_json::to_vec(&resp).unwrap())
                    } else {
                        (404, b"not found".to_vec())
                    };

                    let response = tiny_http::Response::from_data(body)
                        .with_status_code(status)
                        .with_header(
                            "Content-Type: application/json"
                                .parse::<tiny_http::Header>()
                                .unwrap(),
                        );
                    let _ = req.respond(response);
                }
            });

            Self {
                base_url,
                _shutdown: tx,
            }
        }

        fn url(&self) -> &str {
            &self.base_url
        }
    }

    // ── Token store mock ─────────────────────────────────────────────────

    /// In-memory token store for testing.
    #[derive(Clone, Default)]
    struct MockTokenStore {
        inner: Arc<Mutex<std::collections::HashMap<String, OAuthTokens>>>,
    }

    impl TokenStore for MockTokenStore {
        fn store(&self, key: &str, tokens: &OAuthTokens) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .insert(key.to_string(), tokens.clone());
            Ok(())
        }

        fn load(&self, key: &str) -> Result<Option<OAuthTokens>> {
            Ok(self.inner.lock().unwrap().get(key).cloned())
        }

        fn delete(&self, key: &str) -> Result<()> {
            self.inner.lock().unwrap().remove(key);
            Ok(())
        }
    }

    #[test]
    fn test_device_flow_error_display() {
        assert_eq!(
            DeviceFlowError::AuthorizationPending.to_string(),
            "authorisation pending"
        );
        assert_eq!(
            DeviceFlowError::SlowDown.to_string(),
            "slow down polling interval"
        );
        assert_eq!(
            DeviceFlowError::ExpiredToken.to_string(),
            "device code expired"
        );
        assert_eq!(
            DeviceFlowError::AuthorizationDeclined.to_string(),
            "authorisation declined"
        );
        assert_eq!(
            DeviceFlowError::Other("something broke".into()).to_string(),
            "something broke"
        );
    }

    #[test]
    fn test_device_flow_error_is_error() {
        // Verify it implements std::error::Error
        let err: &dyn std::error::Error = &DeviceFlowError::ExpiredToken;
        assert_eq!(err.to_string(), "device code expired");
    }

    #[test]
    fn test_display_instructions_with_complete_uri() {
        let handler = DeviceFlowHandler::new(
            "test-client",
            vec!["openid".into()],
            "https://example.com/token",
            "https://example.com/auth/device",
        );

        let session = DeviceFlowSession {
            device_code: "dev-123".into(),
            user_code: "ABCD-1234".into(),
            verification_uri: "https://example.com/device".into(),
            verification_uri_complete: Some(
                "https://example.com/device?user_code=ABCD-1234".into(),
            ),
            interval: Duration::from_secs(5),
            expires_at: Utc::now() + chrono::TimeDelta::hours(1),
        };

        let msg = handler.display_instructions(&session);
        assert!(msg.contains("ABCD-1234"));
        assert!(msg.contains("https://example.com/device?user_code=ABCD-1234"));
        // The full URI is shown first, so it should appear before the manual URI.
        let complete_pos = msg.find("example.com/device?user_code");
        let manual_pos = msg.find("Or manually");
        assert!(complete_pos.is_some());
        assert!(manual_pos.is_some());
        assert!(complete_pos.unwrap() < manual_pos.unwrap());
    }

    #[test]
    fn test_display_instructions_without_complete_uri() {
        let handler = DeviceFlowHandler::new(
            "test-client",
            vec![],
            "https://example.com/token",
            "https://example.com/auth/device",
        );

        let session = DeviceFlowSession {
            device_code: "dev-456".into(),
            user_code: "WXYZ-5678".into(),
            verification_uri: "https://example.com/device".into(),
            verification_uri_complete: None,
            interval: Duration::from_secs(5),
            expires_at: Utc::now() + chrono::TimeDelta::hours(1),
        };

        let msg = handler.display_instructions(&session);
        assert!(msg.contains("WXYZ-5678"));
        assert!(msg.contains("https://example.com/device"));
        // Without the complete URI, there should be no mention of it.
        assert!(!msg.contains("user_code="));
    }

    #[test]
    fn test_device_flow_session_expires_in_future() {
        let now = Utc::now();
        let session = DeviceFlowSession {
            device_code: "test".into(),
            user_code: "TEST-CODE".into(),
            verification_uri: "https://example.com/device".into(),
            verification_uri_complete: None,
            interval: Duration::from_secs(5),
            expires_at: now + chrono::TimeDelta::hours(1),
        };

        // Session should still be valid
        assert!(session.expires_at > Utc::now());
    }

    #[test]
    fn test_tokens_from_json_full() {
        let body = serde_json::json!({
            "access_token": "ya29.token123",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "refresh123",
            "scope": "openid profile"
        });

        let tokens = tokens_from_json(body).unwrap();
        assert_eq!(tokens.access_token, "ya29.token123");
        assert_eq!(tokens.token_type, "Bearer");
        assert_eq!(tokens.expires_in, Some(Duration::from_secs(3600)));
        assert_eq!(tokens.refresh_token, Some("refresh123".into()));
        assert_eq!(tokens.scope, Some("openid profile".into()));
    }

    #[test]
    fn test_tokens_from_json_missing_access_token() {
        let body = serde_json::json!({
            "error": "invalid_request"
        });

        let result = tokens_from_json(body);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("access_token"));
    }

    #[test]
    fn test_tokens_from_json_minimal() {
        let body = serde_json::json!({
            "access_token": "abc123"
        });

        let tokens = tokens_from_json(body).unwrap();
        assert_eq!(tokens.access_token, "abc123");
        assert_eq!(tokens.token_type, "Bearer"); // default
        assert!(tokens.refresh_token.is_none());
        assert!(tokens.expires_in.is_none());
        assert!(tokens.scope.is_none());
    }

    // ── Integration tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_start_device_flow_success() {
        let mock = MockOAuthServer::start(MockOAuthServerConfig {
            device_auth_response: serde_json::json!({
                "device_code": "dev-123",
                "user_code": "ABCD-1234",
                "verification_uri": "https://example.com/device",
                "verification_uri_complete": "https://example.com/device?user_code=ABCD-1234",
                "expires_in": 1800,
                "interval": 5,
            }),
            device_auth_status: 200,
            token_responses: VecDeque::new(),
            token_status: 200,
        });

        let handler = DeviceFlowHandler::new(
            "test-client",
            vec!["openid".into()],
            &format!("{}/token", mock.url()),
            &format!("{}/auth/device", mock.url()),
        );

        let session = handler.start_device_flow().await.unwrap();
        assert_eq!(session.device_code, "dev-123");
        assert_eq!(session.user_code, "ABCD-1234");
        assert_eq!(session.interval, Duration::from_secs(5));
        assert!(session.expires_at > Utc::now());
    }

    #[tokio::test]
    async fn test_start_device_flow_server_error() {
        let mock = MockOAuthServer::start(MockOAuthServerConfig {
            device_auth_response: serde_json::json!({
                "error": "invalid_client",
                "error_description": "Unknown client"
            }),
            device_auth_status: 400,
            token_responses: VecDeque::new(),
            token_status: 200,
        });

        let handler = DeviceFlowHandler::new(
            "bad-client",
            vec![],
            &format!("{}/token", mock.url()),
            &format!("{}/auth/device", mock.url()),
        );

        let err = handler.start_device_flow().await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid_client"), "got: {msg}");
    }

    #[tokio::test]
    async fn test_poll_once_authorization_pending() {
        let mock = MockOAuthServer::start(MockOAuthServerConfig {
            device_auth_response: serde_json::json!({}),
            device_auth_status: 200,
            token_responses: VecDeque::from(vec![serde_json::json!({
                "error": "authorization_pending"
            })]),
            token_status: 400,
        });

        let session = DeviceFlowSession {
            device_code: "dev-123".into(),
            user_code: "ABCD-1234".into(),
            verification_uri: "https://example.com/device".into(),
            verification_uri_complete: None,
            interval: Duration::from_secs(1),
            expires_at: Utc::now() + chrono::TimeDelta::hours(1),
        };

        let handler = DeviceFlowHandler::new(
            "test-client",
            vec![],
            &format!("{}/token", mock.url()),
            &format!("{}/auth/device", mock.url()),
        );

        let result = handler.poll_once(&session).await;
        match result {
            Err(DeviceFlowError::AuthorizationPending) => {} // expected
            other => panic!("expected AuthorizationPending, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_poll_once_success() {
        let mock = MockOAuthServer::start(MockOAuthServerConfig {
            device_auth_response: serde_json::json!({}),
            device_auth_status: 200,
            token_responses: VecDeque::from(vec![serde_json::json!({
                "access_token": "ya29.newtoken",
                "token_type": "Bearer",
                "expires_in": 3600,
                "scope": "openid"
            })]),
            token_status: 200,
        });

        let session = DeviceFlowSession {
            device_code: "dev-123".into(),
            user_code: "ABCD-1234".into(),
            verification_uri: "https://example.com/device".into(),
            verification_uri_complete: None,
            interval: Duration::from_secs(1),
            expires_at: Utc::now() + chrono::TimeDelta::hours(1),
        };

        let handler = DeviceFlowHandler::new(
            "test-client",
            vec![],
            &format!("{}/token", mock.url()),
            &format!("{}/auth/device", mock.url()),
        );

        let result = handler.poll_once(&session).await;
        match result {
            Ok(Some(tokens)) => {
                assert_eq!(tokens.access_token, "ya29.newtoken");
                assert!(tokens.obtained_at.is_some());
            }
            other => panic!("expected Ok(Some(...)), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_poll_once_authorization_declined() {
        let mock = MockOAuthServer::start(MockOAuthServerConfig {
            device_auth_response: serde_json::json!({}),
            device_auth_status: 200,
            token_responses: VecDeque::from(vec![serde_json::json!({
                "error": "access_denied"
            })]),
            token_status: 400,
        });

        let session = DeviceFlowSession {
            device_code: "dev-123".into(),
            user_code: "ABCD-1234".into(),
            verification_uri: "https://example.com/device".into(),
            verification_uri_complete: None,
            interval: Duration::from_secs(1),
            expires_at: Utc::now() + chrono::TimeDelta::hours(1),
        };

        let handler = DeviceFlowHandler::new(
            "test-client",
            vec![],
            &format!("{}/token", mock.url()),
            &format!("{}/auth/device", mock.url()),
        );

        let result = handler.poll_once(&session).await;
        match result {
            Err(DeviceFlowError::AuthorizationDeclined) => {} // expected
            other => panic!("expected AuthorizationDeclined, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_poll_once_slow_down() {
        let mock = MockOAuthServer::start(MockOAuthServerConfig {
            device_auth_response: serde_json::json!({}),
            device_auth_status: 200,
            token_responses: VecDeque::from(vec![serde_json::json!({
                "error": "slow_down"
            })]),
            token_status: 400,
        });

        let session = DeviceFlowSession {
            device_code: "dev-123".into(),
            user_code: "ABCD-1234".into(),
            verification_uri: "https://example.com/device".into(),
            verification_uri_complete: None,
            interval: Duration::from_secs(1),
            expires_at: Utc::now() + chrono::TimeDelta::hours(1),
        };

        let handler = DeviceFlowHandler::new(
            "test-client",
            vec![],
            &format!("{}/token", mock.url()),
            &format!("{}/auth/device", mock.url()),
        );

        let result = handler.poll_once(&session).await;
        match result {
            Err(DeviceFlowError::SlowDown) => {} // expected
            other => panic!("expected SlowDown, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_device_flow_with_token_store() {
        let mock = MockOAuthServer::start(MockOAuthServerConfig {
            device_auth_response: serde_json::json!({
                "device_code": "dev-w-store",
                "user_code": "STORE-CODE",
                "verification_uri": "https://example.com/device",
                "expires_in": 1800,
                "interval": 5,
            }),
            device_auth_status: 200,
            token_responses: VecDeque::from(vec![serde_json::json!({
                "access_token": "stored-token",
                "token_type": "Bearer",
                "expires_in": 3600,
                "refresh_token": "stored-refresh",
                "scope": "openid"
            })]),
            token_status: 200,
        });

        let store = Arc::new(MockTokenStore::default());
        let handler = DeviceFlowHandler::new(
            "test-client-w-store",
            vec!["openid".into()],
            &format!("{}/token", mock.url()),
            &format!("{}/auth/device", mock.url()),
        )
        .with_token_store(store.clone());

        // Start the device flow
        let session = handler.start_device_flow().await.unwrap();

        // Poll once — should get tokens and auto-store them
        let tokens = handler.poll_once(&session).await.unwrap().unwrap();
        assert_eq!(tokens.access_token, "stored-token");

        // Verify the token store has the tokens
        let stored = store.load("device_flow:test-client-w-store").unwrap();
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().access_token, "stored-token");
    }

    #[tokio::test]
    async fn test_poll_once_expired_token_error() {
        let mock = MockOAuthServer::start(MockOAuthServerConfig {
            device_auth_response: serde_json::json!({}),
            device_auth_status: 200,
            token_responses: VecDeque::from(vec![serde_json::json!({
                "error": "expired_token"
            })]),
            token_status: 400,
        });

        let session = DeviceFlowSession {
            device_code: "dev-expired".into(),
            user_code: "EXP-CODE".into(),
            verification_uri: "https://example.com/device".into(),
            verification_uri_complete: None,
            interval: Duration::from_secs(1),
            expires_at: Utc::now() + chrono::TimeDelta::hours(1),
        };

        let handler = DeviceFlowHandler::new(
            "test-client",
            vec![],
            &format!("{}/token", mock.url()),
            &format!("{}/auth/device", mock.url()),
        );

        let result = handler.poll_once(&session).await;
        match result {
            Err(DeviceFlowError::ExpiredToken) => {} // expected
            other => panic!("expected ExpiredToken, got {other:?}"),
        }
    }
}
