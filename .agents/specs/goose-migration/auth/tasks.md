# Implementation Plan: Authentication Subsystem

## Overview

Implement the OIDC proxy (Cloudflare Worker), OAuth token persistence via keyring, and OAuth device flow support, extending baymax's existing `crates/oauth_callback_server/`.

## Tasks

- [x] 1. Implement OAuth token persistence
  - Created `TokenStore` trait with `store`, `load`, `delete` methods
  - Implemented `KeyringTokenStore` using the `keyring` crate
  - Implemented `EncryptedFileTokenStore` as AES-GCM encrypted file fallback
  - Added encryption helpers with roundtrip and wrong-key tests
  - _Requirements: 2_
  - _writes: crates/oauth_callback_server/src/token_store.rs_

- [x] 2. Implement OAuth device flow
  - Created `DeviceFlowHandler` with `start_device_flow`, `poll_for_token`, `poll_once`
  - Display device code and verification URL via `display_instructions()`
  - Poll token endpoint until user completes authorization (respects server interval, slow_down, expires_at)
  - Store resulting tokens via optional `TokenStore` attachment
  - Added `reqwest`, `tokio` (time feature), `chrono` dependencies
  - _Requirements: 3_
  - _writes: crates/oauth_callback_server/src/device_flow.rs_
  - _reads: .agents/specs/goose-migration/auth/design.md_

- [x] 3. Implement OIDC proxy Cloudflare Worker
  - Worker routes: `/authorize` (redirect), `/callback` (code exchange), `/token` (POST code/refresh exchange), `/.well-known/openid-configuration` (discovery document)
  - Anthropic OIDC provider defaults (configurable via env vars)
  - Secure token exchange using `client_secret` stored as worker secret
  - Wrangler deployment configuration with routes, secrets, and vars
  - TypeScript strict mode, `esbuild` build pipeline
  - Proxy never leaks `client_secret` to clients
  - _Requirements: 1_
  - _writes: oidc-proxy/src/index.ts, oidc-proxy/wrangler.toml, oidc-proxy/package.json, oidc-proxy/tsconfig.json_

- [x] 4. Integrate OAuth persistence with providers
  - Created `TokenManager` wrapping `TokenStore` + `HttpClient`
  - `get_token()` with automatic refresh-token exchange when expired
  - `set_token()` / `delete_token()` for lifecycle management
  - `token_needs_refresh()` checks expiry via `obtained_at` + `expires_in`
  - Added `obtained_at` field to `OAuthTokens` for expiry tracking
  - 9 unit tests covering expiry detection, URL encoding, refresh JSON parsing
  - _Requirements: 2_
  - _writes: crates/language_models/src/provider/oauth_integration.rs_
  - _reads: crates/language_models/src/provider.rs, crates/http_client/src/http_client.rs_

- [x] 5. Write tests
  - TokenStore read/write/delete with keyring mock
  - Device flow state machine with mock OAuth server (21 tests passing)
  - Store-after-success moved to `poll_once_inner` so both `poll_once` and `poll_for_token` auto-store
  - Fixed `let mut req` → `let req` in mock server (clippy warning)
  - _Requirements: 1-3_

## Notes

- The OIDC proxy is a separate deployment (Cloudflare Worker), not compiled into baymax
- OAuth device flow is the primary auth method for CLI/TUI users
- Token persistence ensures users don't re-authenticate on every app restart
