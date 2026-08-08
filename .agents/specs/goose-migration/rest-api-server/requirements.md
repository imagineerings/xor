# Requirements: ACP Server Transport

## Problem

Current Goose does not contain the REST/OpenAPI server described by the previous version of this specification. It exposes the agent through ACP over stdio and an authenticated HTTP transport, with standard ACP session operations and versioned Goose custom methods. The migration plan must not treat invented REST routes as parity work.

## Scope

### In scope

- Source-backed ACP stdio and authenticated HTTP transport behavior, if Sim approves a standalone agent-server product surface.
- Reuse of Sim's agent sessions, thread database, permissions, provider registry, and domain services.
- Standard ACP operations and a versioned adapter for approved Goose custom methods.

### Out of scope

- Resource-oriented REST routes, SSE application endpoints, OpenAPI, Swagger UI, tunnels, and setup routes unless separately approved as Sim product requirements.

## Requirements

### Requirement 1: Parity boundary

**User story:** As a migration reviewer, I want server work tied to executable Goose behavior, so that the plan does not create an unrelated API product.

#### Acceptance criteria

1. **1.1** THE migration SHALL treat ACP stdio and ACP HTTP transport as the only current Goose server parity surfaces.
2. **1.2** THE migration SHALL NOT implement REST/OpenAPI routes without a separately approved product requirement.
3. **1.3** THE server adapter SHALL reuse Sim's existing agent, session, provider, permission, and settings owners.

### Requirement 2: ACP stdio server

**User story:** As an ACP client developer, I want to launch Sim as an ACP agent over stdio, so that I can embed it in an ACP-compatible host.

#### Acceptance criteria

1. **2.1** WHERE standalone ACP serving is approved, THE CLI SHALL start an ACP agent server over stdio without protocol-corrupting stdout logs.
2. **2.2** WHEN an ACP client initializes a stdio connection, THE server SHALL negotiate only capabilities it implements.
3. **2.3** WHEN stdin closes or the client cancels, THE server SHALL cancel owned work and release subprocess, terminal, and session resources.
4. **2.4** IF initialization or session creation fails, THEN THE server SHALL return an ACP error and a non-success process result without panicking.

### Requirement 3: Authenticated ACP HTTP transport

**User story:** As an ACP host, I want a remote transport with explicit security and lifecycle limits, so that the agent is not exposed accidentally.

#### Acceptance criteria

1. **3.1** WHERE remote ACP serving is approved, THE server SHALL bind to an explicitly configured host and port and default to loopback.
2. **3.2** THE HTTP transport SHALL carry ACP messages through the upstream-compatible connection and session streams.
3. **3.3** THE server SHALL require the configured/generated secret for non-public protocol requests and reject invalid credentials without revealing the secret.
4. **3.4** WHERE TLS is enabled, THE server SHALL load the configured certificate and private key and fail startup on invalid material.
5. **3.5** THE server SHALL enforce configured connection and idle limits and cleanly close expired connections.
6. **3.6** IF a stream disconnects, THEN THE server SHALL preserve only the session state promised by ACP and release transport-owned resources.
7. **3.7** THE server SHALL surface bind, authentication, TLS, framing, timeout, and capacity errors through actionable logs and protocol responses.

### Requirement 4: ACP sessions and custom methods

**User story:** As a Goose-compatible ACP client, I want standard session operations and approved custom methods, so that supported workflows behave consistently.

#### Acceptance criteria

1. **4.1** THE server SHALL support the approved standard ACP initialization, authentication, session creation, load, list, prompt, cancel, fork, resume, close, mode, model, and configuration operations.
2. **4.2** THE server SHALL use Sim's thread database for persistence, deletion cascades, parent-child relationships, and project working directories.
3. **4.3** THE server SHALL expose a versioned custom-method inventory generated from canonical request/response definitions.
4. **4.4** WHERE a domain capability is unavailable, THE related custom method SHALL return an explicit unsupported error rather than a successful placeholder.
5. **4.5** THE server SHALL route provider, extension, recipe, schedule, dictation, app/resource, prompt, source, diagnostic, and permission methods to their canonical domain owners.
6. **4.6** THE server SHALL preserve notification ordering and correlate session, tool, permission, usage, and error updates with the originating request.

### Requirement 5: Security, permissions, and compatibility

**User story:** As a Sim user, I want remote and embedded clients constrained by the same safety policy as the desktop UI, so that protocol access cannot bypass protections.

#### Acceptance criteria

1. **5.1** THE server SHALL enforce Sim's filesystem roots, sandbox, tool permission, credential, and secret-redaction policies for every transport.
2. **5.2** THE server SHALL NOT expose provider tokens, extension environment secrets, sensitive logs, or unrestricted local resources through custom methods.
3. **5.3** THE server SHALL reject unsupported or malformed methods and fields with stable protocol errors.
4. **5.4** THE server SHALL support compatibility tests against the audited Goose ACP schema and TypeScript client.
5. **5.5** THE server SHALL isolate one connection's cancellation, malformed input, and backpressure from unrelated sessions.
6. **5.6** THE server SHALL record security-relevant lifecycle events without logging message bodies or secrets by default.

## Open questions

- Should Sim ship a standalone ACP server at all, and if so over stdio, HTTP, or both?
- Which Goose custom methods are compatibility commitments versus intentionally unsupported extensions?
- Is a separate REST/OpenAPI product desired independently of Goose migration? This specification does not approve it.

## Evidence

- `projects/goose/crates/goose-cli/src/cli.rs` — `Command::Acp`, `Command::Serve`.
- `projects/goose/crates/goose/src/acp/server.rs`, `server_factory.rs`, `server/*`.
- `projects/goose/crates/goose/src/acp/transport/{mod,auth,tls}.rs`.
- `projects/goose/crates/goose/src/acp/server/custom_dispatch.rs`.
- `projects/goose/crates/goose-sdk-types/src/custom_requests/*`.
- Sim reuse points: `crates/acp_thread`, `crates/agent`, `crates/agent_servers`, `crates/settings`, `crates/credentials_provider`.
