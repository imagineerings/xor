# Design: ACP Server Transport

## Overview

If the product decision approves a standalone server, a thin ACP adapter will expose Sim's existing agent/session services over stdio and/or authenticated HTTP. It will not create REST resources, a second session database, a second provider registry, or a second permission system.

## Existing context

- Goose's current server entry points are `Command::Acp` and `Command::Serve`, backed by `acp/server.rs` and `acp/transport/*`.
- Sim already implements ACP client/session abstractions in `crates/acp_thread`, external agent lifecycle in `crates/agent_servers`, and native sessions in `crates/agent`.
- Domain services planned elsewhere own recipes, schedules, dictation, providers, extensions, prompts, sources, and MCP Apps.

## Design decisions

### D-ACP-SERVER-BOUNDARY

- Responsibility: expose only approved ACP behavior and declare unsupported custom methods explicitly.
- Integration: adapt `agent::Thread`/thread storage and existing registries.
- Rationale: protocol serving is an adapter, not a new application core.

### D-ACP-STDIO

- Responsibility: framed ACP on stdin/stdout, clean process lifecycle, and stderr/file logging.
- Integration: CLI entry point creates the same session service used in-process.
- Rationale: preserves protocol integrity and avoids a parallel headless engine.

### D-ACP-HTTP

- Responsibility: connection/session streams, generated secret authentication, optional TLS, limits, and idle cleanup.
- Integration: reuse Sim HTTP/TLS primitives and ACP message types.
- Rationale: matches current Goose observable transport behavior without inventing REST routes.

### D-ACP-CUSTOM-METHODS

- Responsibility: generated, versioned dispatch to canonical domain owners.
- Integration: each method delegates to provider, extension, recipe, schedule, dictation, session, prompt, source, diagnostic, or app services.
- Rationale: prevents protocol handlers from becoming duplicate business logic.

### D-ACP-SERVER-SECURITY

- Responsibility: enforce authentication, filesystem/sandbox/permission policy, redaction, isolation, and safe errors.
- Integration: existing Sim credential, sandbox, tool-permission, logging, and session boundaries.
- Rationale: remote transport must not be a policy bypass.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2, 1.3 | D-ACP-SERVER-BOUNDARY | Source-inventory review and negative route test |
| 2.1, 2.2, 2.3, 2.4 | D-ACP-STDIO | ACP conformance, stdout-integrity, cancellation, and failure tests |
| 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7 | D-ACP-HTTP | Bind/auth/TLS/limit/disconnect error-path tests |
| 4.1, 4.2, 4.3, 4.4, 4.5, 4.6 | D-ACP-CUSTOM-METHODS | Goose SDK compatibility and unsupported-method tests |
| 5.1, 5.2, 5.3, 5.4, 5.5, 5.6 | D-ACP-SERVER-SECURITY | Policy-bypass, redaction, malformed input, isolation, and audit tests |

## Testing strategy

- Run protocol fixtures against both stdio and HTTP transports.
- Differentially exercise approved standard/custom methods with the Goose TypeScript client.
- Inject disconnects, cancellation, malformed frames, invalid secrets/certificates, idle expiry, and connection saturation.
- Verify thread persistence and deletion through the existing database.
- Verify no secrets or protocol frames leak to the wrong output/log channel.

## Unresolved decisions

Implementation must not start until the product chooses the supported transport(s) and custom-method compatibility set. REST/OpenAPI remains outside this parity design.
