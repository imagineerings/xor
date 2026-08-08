# Tasks: Authentication and CI OIDC Proxy

- [ ] 1. Reconcile browser and MCP OAuth with the existing callback server
  - Route loopback ownership through `oauth_callback_server`
  - Implement state, timeout, cancellation, exchange, error propagation, and safe success/error pages
  - Reuse persisted credentials and refresh before starting a new browser flow
  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _Depends on: none_
  - _Reads: requirements.md, design.md, coverage-catalog.md, projects/goose/crates/goose/src/oauth/mod.rs, crates/oauth_callback_server/, crates/http_client/_
  - _Writes: crates/oauth_callback_server/, selected provider/MCP auth integration_
  - _Validation: Run callback state, port conflict, browser failure, timeout, cancellation, provider rejection, success, and no-token-leak tests_

- [ ] 2. Extend the canonical credential lifecycle
  - Store typed provider/server identity, scopes, expiry, and refresh metadata through `credentials_provider`
  - Make replacement/refresh/revocation atomic and serialize concurrent refresh
  - Surface unavailable/corrupt storage and redact every diagnostic/export path
  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Depends on: none_
  - _Reads: requirements.md, design.md, projects/goose/crates/goose/src/oauth/persist.rs, crates/credentials_provider/, crates/language_models/_
  - _Writes: crates/credentials_provider/, selected provider auth integrations_
  - _Validation: Run restart, atomic replacement, concurrent refresh, invalid-grant, revocation, unavailable/corrupt store, and log/telemetry/export redaction tests_

- [ ] 3. Add device flow only to approved provider capabilities
  - Share polling mechanics where exact contracts agree; keep provider-specific endpoints, encodings, and refresh behavior in provider integrations
  - Honor interval, slow-down, denial, expiry, retry, cancellation, and timeout
  - Persist success through Task 2 and hide device/access/refresh tokens
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Depends on: 2_
  - _Reads: requirements.md, design.md, projects/goose/crates/goose/src/providers/oauth_device_flow.rs, projects/goose/crates/goose/src/providers/githubcopilot.rs, projects/goose/crates/goose/src/providers/kimicode.rs, projects/goose/crates/goose/src/providers/xai_oauth.rs, crates/language_models/_
  - _Writes: selected provider auth integrations_
  - _Validation: Run provider-capability, response-validation, interval, slow-down, pending, denial, expiry, cancellation, timeout, refresh, persistence, and secret-redaction tests_

- [ ] 4. Decide whether to operate the GitHub Actions OIDC upstream proxy
  - Record deployment owner, approved upstreams, allowed repositories/refs/audiences, rate/budget policy, monitoring, abuse response, and secret rotation
  - Do not deploy or implement the service until this operational decision is approved
  - _Requirements: 4.4, 4.5_
  - _Depends on: none_
  - _Reads: requirements.md, design.md, projects/goose/oidc-proxy/README.md, projects/goose/oidc-proxy/wrangler.toml_
  - _Writes: auth design/deployment decision record_
  - _Validation: Security and operations review confirms ownership, threat model, upstream/CORS allowlists, quotas, monitoring, rotation, and incident procedure_

- [ ] 5. If approved, port and harden the CI OIDC proxy behavior
  - Verify JWT structure/signature/algorithm/expiry/age/issuer/audience/repository/ref and JWKS rotation
  - Enforce atomic per-token budget/rate limits before forwarding
  - Strip caller auth, inject the Worker secret, preserve requests, normalize response headers, and expose stable failures
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_
  - _Depends on: 4_
  - _Reads: requirements.md, design.md, projects/goose/oidc-proxy/src/index.js, projects/goose/oidc-proxy/test/index.test.js, projects/goose/oidc-proxy/wrangler.toml_
  - _Writes: approved CI service location_
  - _Validation: Run malformed/expired/old/wrong-claim/key-rotation JWT tests, concurrent quota/rate tests, forwarding/header/CORS/compression tests, secret-leak tests, and negative tests proving no login/callback/token routes_

- [ ] 6. Run end-to-end authentication compatibility tests
  - Cover desktop and terminal callback/device surfaces, persistence across restart, refresh races, revocation, cancellation, and redaction
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 4.5_
  - _Depends on: 1, 2, 3, 5_
  - _Reads: requirements.md, design.md, affected provider/MCP auth integrations, affected CI service tests_
  - _Writes: affected authentication integration tests_
  - _Validation: Run focused authentication tests and `./script/clippy` for affected Rust crates; run approved CI service tests and type/lint checks_
