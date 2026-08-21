# Requirements: Authentication and CI OIDC Proxy

## Introduction

Reconcile Goose authentication behavior with Zed's existing OAuth callback server, credentials provider, HTTP client, and provider integrations. Goose's `oidc-proxy` is not an end-user login service: it validates GitHub Actions OIDC JWTs, rate-limits each token, and injects an upstream API key for CI requests. That deployment remains conditional on an explicit operational decision.

## Requirements

### Requirement 1: OAuth Callback and MCP Authorization

**User Story:** As a user, I want browser-based OAuth to complete safely and persist, so that approved providers and MCP servers can reconnect without exposing credentials.

#### Acceptance Criteria

1. **1.1** THE migration SHALL reuse `oauth_callback_server` for loopback callback ownership and SHALL validate state, redirect address, callback timeout, provider error, and cancellation before accepting credentials
2. **1.2** THE MCP OAuth path SHALL try valid persisted credentials, refresh when possible, and start a full authorization flow only when credentials are missing or invalid
3. **1.3** THE callback UI SHALL report success or a specific authorization failure without placing access or refresh tokens in URLs, logs, settings text, or user-visible diagnostics
4. **1.4** IF the callback port is unavailable, the browser cannot open, the callback times out, the state mismatches, or the provider rejects the exchange, THEN the initiating CLI or desktop surface SHALL receive a clear recoverable error

### Requirement 2: Canonical Credential Persistence and Lifecycle

**User Story:** As a user, I want provider credentials to survive restarts and refresh safely, so that I am not repeatedly prompted or left with inconsistent authentication state.

#### Acceptance Criteria

1. **2.1** THE system SHALL persist OAuth credentials through Zed's existing credentials-provider abstraction with provider/server identity, scopes, access expiry, and optional refresh metadata
2. **2.2** WHEN credentials are refreshed, revoked, replaced, or cleared, THE canonical store SHALL update atomically and all consumers SHALL observe the same state
3. **2.3** CONCURRENT refresh requests for the same credential SHALL be coalesced or serialized so that stale refresh results cannot overwrite newer credentials
4. **2.4** CREDENTIAL material SHALL be redacted from logs, telemetry, diagnostics, exported settings, errors, and crash reports; unavailable or corrupt credential storage SHALL produce a visible error rather than silently forgetting authentication

### Requirement 3: Provider Device-Code Flow

**User Story:** As a terminal or remote user, I want provider device-code authentication, so that I can authorize approved providers without a local callback browser.

#### Acceptance Criteria

1. **3.1** FOR an approved provider that advertises device authorization, THE system SHALL request a device code and display the verification URL, user code, and bounded expiry without displaying the device token
2. **3.2** THE poller SHALL honor the provider interval, `authorization_pending`, `slow_down`, denial, expiry, HTTP retry policy, user cancellation, and overall timeout
3. **3.3** WHEN authorization succeeds, THE system SHALL persist access, refresh, expiry, and scope metadata through Requirement 2 and SHALL support provider-defined refresh semantics
4. **3.4** DEVICE flow SHALL remain provider-capability driven; Zed SHALL NOT expose a generic device-flow option for a provider that does not support it

### Requirement 4: GitHub Actions OIDC Upstream Proxy

**User Story:** As a CI operator, I want short-lived GitHub Actions identity to access an approved provider API, so that workflows do not store the provider's long-lived API key.

#### Acceptance Criteria

1. **4.1** WHERE this separately deployed service is approved, THE Worker SHALL validate JWT structure, signature, algorithm, expiry, optional maximum token age, issuer, audience, repository, and ref against fetched OIDC metadata and JWKS
2. **4.2** THE Worker SHALL rate-limit and budget requests atomically per `jti`-derived identity and SHALL return stable `401`, `429`, `Retry-After`, and not-found responses without exposing secrets
3. **4.3** ONLY after validation and quota admission, THE Worker SHALL remove caller authentication, inject the configured upstream credential/header, forward method/path/query/body, and normalize decompression and CORS headers
4. **4.4** THE deployment SHALL keep the upstream key in Worker secrets and SHALL define JWKS cache refresh, Durable Object storage, CORS allowlist, abuse controls, monitoring, secret rotation, and incident ownership
5. **4.5** THIS service SHALL NOT be described or implemented as an Anthropic user OIDC handshake, authorization callback, token endpoint, or discovery server

## References

- Goose: `projects/goose/crates/goose/src/oauth/mod.rs` — `oauth_flow`, `wait_for_callback`
- Goose: `projects/goose/crates/goose/src/oauth/persist.rs` — `GooseCredentialStore`
- Goose: `projects/goose/crates/goose/src/providers/oauth_device_flow.rs` — `run_device_flow`, `poll_for_tokens`, `refresh_device_flow_token`
- Goose: `projects/goose/crates/goose/src/providers/{githubcopilot,kimicode,xai_oauth}.rs`
- Goose: `projects/goose/oidc-proxy/src/index.js` — `fetch`, `verifyOidcToken`, `checkTokenBucket`; `README.md`; tests and `wrangler.toml`
- Zed: `crates/oauth_callback_server/src/oauth_callback_server.rs`
- Zed: `crates/credentials_provider/`
- Zed: `crates/http_client/`
