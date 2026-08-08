# Design: Authentication and CI OIDC Proxy

## Ownership

- **D-AUTH-CALLBACK:** `oauth_callback_server` remains the only loopback listener. Provider and MCP auth code supplies state/exchange logic and receives a typed result.
- **D-AUTH-CREDENTIALS:** `credentials_provider` remains the only credential persistence authority. Provider implementations may cache non-secret derived state but cannot persist a second token copy.
- **D-AUTH-DEVICE:** A shared device-flow helper may live with provider authentication only when at least two approved providers share the exact polling contract. Provider capability metadata decides availability.
- **D-CI-OIDC-PROXY:** The Cloudflare Worker is an independently deployed CI service. It verifies GitHub Actions identity and injects an upstream key; it is not a desktop/CLI login component.

## Lifecycle and failures

Browser OAuth binds one loopback listener, generates and verifies state, opens the authorization URL when possible, waits with a bounded timeout, exchanges the code, and atomically persists credentials. Cancellation closes the listener and does not leave partial credentials.

Credential refresh is keyed by provider/server identity. Concurrent callers share one refresh attempt. A failed refresh does not erase still-valid credentials; invalid-grant or explicit revocation clears them and prompts reauthentication through the initiating UI.

Device flow validates response fields, displays only the user-facing code/URL, polls at the provider interval, increases delay on `slow_down`, and stops on success, denial, expiry, cancellation, or timeout. Tokens are never clipboard-copied or logged without an explicit safe UX decision.

The CI proxy verifies claims and signature before quota admission or forwarding. The per-token Durable Object serializes rate/budget state. The Worker deletes incoming authorization headers, injects the operator secret, and never returns it. JWKS refresh handles key rotation, and operational deployment remains outside this implementation plan until approved.

## Security and compatibility

- Treat callback state, device codes, access tokens, refresh tokens, upstream keys, and JWTs as sensitive.
- Use Sim's HTTP/TLS/proxy configuration for application auth flows.
- Do not migrate Goose credential files verbatim; import only through an explicit, redacted migration with rollback.
- Return typed errors to desktop/CLI surfaces and keep logs useful without secret material.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2, 1.3, 1.4 | D-AUTH-CALLBACK | State, timeout, port, cancellation, browser, provider-error, and token-leak tests |
| 2.1, 2.2, 2.3, 2.4 | D-AUTH-CREDENTIALS | Restart, atomic update, concurrent refresh, corruption, unavailable-store, and redaction tests |
| 3.1, 3.2, 3.3, 3.4 | D-AUTH-DEVICE | Capability, interval, slow-down, denial, expiry, timeout, cancellation, refresh, and persistence tests |
| 4.1, 4.2, 4.3, 4.4, 4.5 | D-CI-OIDC-PROXY | JWT/claim/JWKS, quota, forwarding, header, CORS, compression, secret, and route-negative tests |

## Open decisions

1. Whether Sim will operate the CI OIDC proxy at all, and which team owns deployment, abuse response, monitoring, and credential rotation.
2. Which provider device flows are in the initial supported set.
3. Whether a legacy Goose credential import is desirable; this requires a separate migration threat model.
