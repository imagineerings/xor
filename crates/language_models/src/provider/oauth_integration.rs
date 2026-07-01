//! OAuth token lifecycle management for language model providers.
//!
//! [`TokenManager`] wraps a [`TokenStore`] to provide persistent storage,
//! loading, and automatic refresh of OAuth tokens.  It is the integration
//! layer between the low-level [`oauth_callback_server`] crate and the
//! provider implementations in this crate.
//!
//! # Usage
//!
//! ```ignore
//! let manager = TokenManager::new(store, http_client);
//!
//! // Attempt to load a previously stored token (auto-refreshes if needed).
//! if let Some(tokens) = manager.get_token("anthropic", None, None).await? {
//!     // Use tokens.access_token …
//! }
//!
//! // Store a freshly obtained token.
//! manager.set_token("anthropic", &new_tokens).await?;
//!
//! // Delete (e.g. on sign-out or unrecoverable auth error).
//! manager.delete_token("anthropic").await?;
//! ```

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use futures::io::AsyncReadExt;
use http_client::{AsyncBody, HttpClient};
use oauth_callback_server::token_store::{OAuthTokens, TokenStore};
use std::sync::Arc;
use std::time::Duration;

// ── TokenManager ─────────────────────────────────────────────────────────

/// High-level manager for OAuth token lifecycle.
///
/// Each provider is identified by a string key (e.g. `"anthropic"` or
/// `"openai:codex"`).  All tokens are persisted through the configured
/// [`TokenStore`].
pub struct TokenManager {
    store: Arc<dyn TokenStore>,
    http_client: Arc<dyn HttpClient>,
}

impl TokenManager {
    /// Create a new token manager.
    pub fn new(store: Arc<dyn TokenStore>, http_client: Arc<dyn HttpClient>) -> Self {
        Self { store, http_client }
    }

    /// Return a reference to the underlying [`TokenStore`].
    pub fn store(&self) -> &Arc<dyn TokenStore> {
        &self.store
    }

    /// Return a reference to the underlying HTTP client.
    pub fn http_client(&self) -> &Arc<dyn HttpClient> {
        &self.http_client
    }

    // ── Public API ──────────────────────────────────────────────────────

    /// Load tokens for `provider_key`.
    ///
    /// If the stored token is expired and carries a `refresh_token`, this
    /// method automatically attempts to refresh it using the given
    /// `refresh_url` and `client_id`.  Refreshed tokens are persisted before
    /// being returned.
    ///
    /// Returns `Ok(None)` when no token exists for the key.
    pub async fn get_token(
        &self,
        provider_key: &str,
        refresh_url: Option<&str>,
        client_id: Option<&str>,
    ) -> Result<Option<OAuthTokens>> {
        let Some(mut tokens) = self.store.load(provider_key)? else {
            return Ok(None);
        };

        // Auto-refresh if the token is expired and we have the necessary
        // configuration to perform a refresh.
        if Self::token_needs_refresh(&tokens) {
            if let (Some(url), Some(cid)) = (refresh_url, client_id) {
                if let Some(refresh_token) = &tokens.refresh_token {
                    match self.do_refresh(url, cid, refresh_token).await {
                        Ok(fresh) => {
                            self.store
                                .store(provider_key, &fresh)
                                .context("failed to persist refreshed token")?;
                            tokens = fresh;
                        }
                        Err(e) => {
                            // Refresh failed – return the expired token and
                            // let the caller decide.  Common reasons include
                            // network errors and revoked refresh tokens.
                            log::warn!("failed to refresh token for {provider_key}: {e}");
                        }
                    }
                }
            }
        }

        Ok(Some(tokens))
    }

    /// Persist `tokens` for `provider_key`.
    pub async fn set_token(&self, provider_key: &str, tokens: &OAuthTokens) -> Result<()> {
        self.store
            .store(provider_key, tokens)
            .with_context(|| format!("failed to store token for {provider_key}"))
    }

    /// Delete any stored tokens for `provider_key`.
    pub async fn delete_token(&self, provider_key: &str) -> Result<()> {
        self.store
            .delete(provider_key)
            .with_context(|| format!("failed to delete token for {provider_key}"))
    }

    /// Returns `true` if the token is expired (or has no obtainment
    /// timestamp and therefore should be treated as stale).
    pub fn token_needs_refresh(tokens: &OAuthTokens) -> bool {
        let Some(expires_in) = tokens.expires_in else {
            // No expiry information – assume still valid.
            return false;
        };
        let Some(obtained_at) = tokens.obtained_at else {
            // No obtainment timestamp – treat as stale.
            return true;
        };
        Utc::now() >= obtained_at + expires_in
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Exchange a refresh token for fresh tokens.
    async fn do_refresh(
        &self,
        token_url: &str,
        client_id: &str,
        refresh_token: &str,
    ) -> Result<OAuthTokens> {
        let body = format!(
            "grant_type=refresh_token&refresh_token={}&client_id={}",
            urlencode(refresh_token),
            urlencode(client_id),
        );

        let request = http::Request::post(token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(AsyncBody::from(body))
            .context("failed to build refresh request")?;

        let response = self
            .http_client
            .send(request)
            .await
            .context("refresh request failed")?;

        let status = response.status();
        let mut response_body = String::new();
        response
            .into_body()
            .read_to_string(&mut response_body)
            .await
            .context("failed to read refresh response body")?;

        if !status.is_success() {
            let snippet = &response_body[..response_body.len().min(256)];
            return Err(anyhow!("token refresh returned {status}: {snippet}"));
        }

        let json: serde_json::Value =
            serde_json::from_str(&response_body).context("failed to parse refresh response")?;

        tokens_from_refresh_json(json)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Parse a token-refresh JSON response into [`OAuthTokens`].
fn tokens_from_refresh_json(body: serde_json::Value) -> Result<OAuthTokens> {
    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| anyhow!("missing 'access_token' in refresh response"))?
        .to_string();

    // The refresh *may* return a new refresh_token; keep the old one if not.
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

/// URL-encode a string for form bodies.
fn urlencode(s: &str) -> String {
    // A simple percent-encoding for values used in OAuth forms.
    // This handles most characters; we use it for refresh_token and
    // client_id which are typically alphanumeric anyway.
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push_str("%20"),
            _ => {
                out.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    out
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_needs_refresh_no_expiry() {
        let tokens = OAuthTokens {
            access_token: "tok".into(),
            refresh_token: None,
            expires_in: None,
            token_type: "Bearer".into(),
            scope: None,
            obtained_at: None,
        };
        assert!(!TokenManager::token_needs_refresh(&tokens));
    }

    #[test]
    fn test_token_needs_refresh_no_obtained_at() {
        let tokens = OAuthTokens {
            access_token: "tok".into(),
            refresh_token: None,
            expires_in: Some(Duration::from_secs(3600)),
            token_type: "Bearer".into(),
            scope: None,
            obtained_at: None,
        };
        // No obtainment timestamp → treat as stale.
        assert!(TokenManager::token_needs_refresh(&tokens));
    }

    #[test]
    fn test_token_needs_refresh_still_valid() {
        let tokens = OAuthTokens {
            access_token: "tok".into(),
            refresh_token: None,
            expires_in: Some(Duration::from_secs(3600)),
            token_type: "Bearer".into(),
            scope: None,
            obtained_at: Some(Utc::now()),
        };
        assert!(!TokenManager::token_needs_refresh(&tokens));
    }

    #[test]
    fn test_token_needs_refresh_expired() {
        let tokens = OAuthTokens {
            access_token: "tok".into(),
            refresh_token: None,
            expires_in: Some(Duration::from_secs(1)),
            token_type: "Bearer".into(),
            scope: None,
            // 2 seconds ago – beyond the 1-second expiry.
            obtained_at: Some(Utc::now() - chrono::TimeDelta::seconds(2)),
        };
        assert!(TokenManager::token_needs_refresh(&tokens));
    }

    #[test]
    fn test_tokens_from_refresh_json_full() {
        let json = serde_json::json!({
            "access_token": "new-token",
            "refresh_token": "new-refresh",
            "expires_in": 7200,
            "token_type": "Bearer",
            "scope": "openid profile"
        });
        let tokens = tokens_from_refresh_json(json).unwrap();
        assert_eq!(tokens.access_token, "new-token");
        assert_eq!(tokens.refresh_token, Some("new-refresh".into()));
        assert_eq!(tokens.expires_in, Some(Duration::from_secs(7200)));
        assert_eq!(tokens.token_type, "Bearer");
        assert_eq!(tokens.scope, Some("openid profile".into()));
        assert!(tokens.obtained_at.is_some());
    }

    #[test]
    fn test_tokens_from_refresh_json_minimal() {
        let json = serde_json::json!({
            "access_token": "just-a-token"
        });
        let tokens = tokens_from_refresh_json(json).unwrap();
        assert_eq!(tokens.access_token, "just-a-token");
        assert!(tokens.refresh_token.is_none());
        assert!(tokens.expires_in.is_none());
        assert_eq!(tokens.token_type, "Bearer");
    }

    #[test]
    fn test_tokens_from_refresh_json_missing_access_token() {
        let json = serde_json::json!({ "error": "invalid_grant" });
        let result = tokens_from_refresh_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_urlencode_normal() {
        assert_eq!(urlencode("abc123"), "abc123");
        assert_eq!(urlencode("ABC-DEF.ghi_jkl~"), "ABC-DEF.ghi_jkl~");
    }

    #[test]
    fn test_urlencode_special_chars() {
        assert_eq!(urlencode("a b"), "a%20b");
        assert_eq!(urlencode("foo@bar"), "foo%40bar");
    }
}
