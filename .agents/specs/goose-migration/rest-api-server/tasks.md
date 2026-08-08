# Implementation Plan: ACP Server Transport

## Approach

These tasks are conditional on approval of a standalone ACP server. They add adapters around existing Sim owners; they do not create the previously proposed REST/OpenAPI server.

## Tasks

- [ ] 1. Add the standalone ACP server integration boundary
  - Bind approved ACP capabilities to the existing agent, thread database, provider registry, extension registry, and permission policy.
  - Return explicit unsupported errors for unapproved domain methods.
  - _Requirements: 1.1, 1.2, 1.3, 4.1, 4.2, 4.4, 4.5_
  - _Depends on: none_
  - _Reads: projects/goose/crates/goose/src/acp/server.rs, crates/agent/src/agent.rs, crates/agent/src/db.rs, crates/acp_thread/src/connection.rs, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: existing ACP/agent server integration files selected after architecture review_
  - _Validation: focused session creation, load, unsupported-method, permission, and persistence tests_

- [ ] 2. Add the ACP stdio entry point
  - Keep protocol stdout clean and propagate initialization, cancellation, EOF, and process errors.
  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Depends on: 1_
  - _Reads: projects/goose/crates/goose-cli/src/cli.rs, projects/goose/crates/goose/src/acp/server_factory.rs, crates/cli/src/main.rs_
  - _Writes: crates/cli/src/main.rs, existing ACP server integration files_
  - _Validation: ACP stdio conformance fixture plus stdout-integrity and shutdown tests_

- [ ] 3. Add authenticated ACP HTTP transport
  - Implement connection/session streams, loopback default, generated/configured secret, capacity/idle limits, and disconnect cleanup.
  - _Requirements: 3.1, 3.2, 3.3, 3.5, 3.6, 3.7, 5.5_
  - _Depends on: 1_
  - _Reads: projects/goose/crates/goose/src/acp/transport/mod.rs, projects/goose/ui/sdk/src/http-stream.ts, existing Sim HTTP server primitives_
  - _Writes: existing ACP server transport files selected after architecture review_
  - _Validation: connection/session stream compatibility, invalid-secret, timeout, capacity, disconnect, and backpressure tests_

- [ ] 4. Integrate TLS and transport security policy
  - Reuse Sim TLS/credential/logging facilities and enforce redaction and safe startup failure.
  - _Requirements: 3.4, 3.7, 5.1, 5.2, 5.3, 5.6_
  - _Depends on: 3_
  - _Reads: projects/goose/crates/goose/src/acp/transport/auth.rs, projects/goose/crates/goose/src/acp/transport/tls.rs, crates/credentials_provider, crates/http_client_tls_
  - _Writes: existing ACP server transport and configuration files_
  - _Validation: invalid certificate/key, authentication bypass, secret redaction, filesystem-root, and permission-policy tests_

- [ ] 5. Generate and dispatch the approved custom-method set
  - Generate request/response types from canonical definitions and delegate every method to its domain owner.
  - _Requirements: 4.3, 4.4, 4.5, 4.6, 5.3, 5.4_
  - _Depends on: 1_
  - _Reads: projects/goose/crates/goose/src/acp/server/custom_dispatch.rs, projects/goose/crates/goose-sdk-types/src, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: ACP schema/code-generation and server-dispatch files selected after method-scope approval_
  - _Validation: generated-schema freshness check and Goose TypeScript SDK compatibility tests for every approved method_

- [ ] 6. Add end-to-end lifecycle and isolation coverage
  - Exercise stdio/HTTP startup, session persistence, cancellation, malformed input, concurrent connections, and clean shutdown.
  - _Requirements: 2.3, 2.4, 3.5, 3.6, 3.7, 4.1, 4.2, 4.6, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
  - _Depends on: 2, 3, 4, 5_
  - _Reads: all ACP server integration and transport files, projects/goose/crates/goose/tests/acp_fixtures_
  - _Writes: focused ACP server integration tests and fixtures_
  - _Validation: run focused ACP server tests and ./script/clippy for affected Rust crates_
