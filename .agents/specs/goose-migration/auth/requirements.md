# Requirements: Authentication Subsystem

## Introduction

Migrate goose's authentication infrastructure: the OIDC proxy (Cloudflare Worker for OIDC-based auth) and additional OAuth features (persistent token storage and device flow authentication).

## Glossary

- **OIDC**: OpenID Connect, an identity layer on top of OAuth 2.0
- **OIDC Proxy**: A Cloudflare Worker that proxies OIDC authentication requests (specifically for Anthropic)
- **OAuth Persistence**: Storing OAuth tokens persistently so they survive application restarts
- **OAuth Device Flow**: OAuth 2.0 Device Authorization Grant, for devices with limited input capability
- **Cloudflare Worker**: Serverless JavaScript/TypeScript functions on Cloudflare's edge network

## Requirements

### Requirement 1: OIDC Proxy

**User Story:** As a sim user, I want to authenticate via OpenID Connect through a proxy, so that I can use OIDC-based provider authentication.

#### Acceptance Criteria

1. WHEN an OIDC authentication flow is initiated THEN the proxy SHALL handle the OIDC handshake
2. THE OIDC proxy SHALL support the Anthropic OIDC provider configuration
3. THE OIDC proxy SHALL securely transmit tokens between the provider and the application
4. IF the OIDC flow fails THEN the proxy SHALL return a clear error

### Requirement 2: OAuth Persistence

**User Story:** As a sim user, I want OAuth tokens to persist across application restarts, so that I don't need to re-authenticate every time.

#### Acceptance Criteria

1. THE system SHALL persist OAuth tokens to local storage
2. WHEN the application starts THE system SHALL load persisted tokens
3. WHEN a token is refreshed THE system SHALL persist the updated token
4. THE persisted tokens SHALL be encrypted at rest

### Requirement 3: OAuth Device Flow

**User Story:** As a sim user, I want to authenticate using OAuth device flow, so that I can authorize the agent on devices without a browser (terminals, SSH, etc.).

#### Acceptance Criteria

1. WHEN device flow is initiated THEN the system SHALL display a device code and verification URL
2. THE system SHALL poll the token endpoint until the user completes authorization
3. WHEN authorization completes THEN the system SHALL store the resulting tokens
4. IF the authorization expires or is denied THEN the system SHALL display the appropriate error

## References

- Source: `projects/goose/oidc-proxy/` — Cloudflare Worker implementation
- Source: `projects/goose/crates/goose/src/oauth/` — persist.rs, mod.rs
- Source: `projects/goose/crates/goose/src/providers/oauth_device_flow.rs`
- Existing sim: `crates/oauth_callback_server/`
