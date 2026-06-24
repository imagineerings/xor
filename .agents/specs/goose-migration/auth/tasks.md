# Implementation Plan: Authentication Subsystem

## Overview

Implement the OIDC proxy (Cloudflare Worker), OAuth token persistence via keyring, and OAuth device flow support, extending baymax's existing `crates/oauth_callback_server/`.

## Tasks

- [ ] 1. Implement OAuth token persistence
  - Create `TokenStore` trait with `store`, `load`, `delete` methods
  - Implement `KeyringTokenStore` using the `keyring` crate (already workspace dependency)
  - Implement `EncryptedFileTokenStore` as fallback for headless environments
  - Encrypt tokens at rest using platform keychain or AES-GCM
  - _Requirements: 2_
  - _writes: crates/oauth_callback_server/src/token_store.rs_

- [ ] 2. Implement OAuth device flow
  - Create `DeviceFlowHandler` with start/poll flow
  - Display device code and verification URL to user
  - Poll token endpoint until user completes authorization
  - Store resulting tokens via TokenStore
  - _Requirements: 3_
  - _writes: crates/oauth_callback_server/src/device_flow.rs_

- [ ] 3. Implement OIDC proxy Cloudflare Worker
  - Worker routes: `/authorize`, `/callback`, `/token`, `/.well-known/openid-configuration`
  - Anthropic-specific OIDC configuration
  - Secure token exchange and forwarding
  - Wrangler deployment configuration
  - _Requirements: 1_
  - _writes: oidc-proxy/src/index.ts, oidc-proxy/wrangler.toml_

- [ ] 4. Integrate OAuth persistence with providers
  - Update ACP-based providers to use TokenStore for OAuth tokens
  - Auto-refresh expired tokens using refresh tokens
  - _Requirements: 2_
  - _writes: crates/language_models/src/provider/oauth_integration.rs_

- [ ] 5. Write tests
  - TokenStore read/write/delete with keyring mock
  - Device flow state machine with mock OAuth server
  - Worker tests with Miniflare or similar
  - _Requirements: 1-3_

## Notes

- The OIDC proxy is a separate deployment (Cloudflare Worker), not compiled into baymax
- OAuth device flow is the primary auth method for CLI/TUI users
- Token persistence ensures users don't re-authenticate on every app restart
