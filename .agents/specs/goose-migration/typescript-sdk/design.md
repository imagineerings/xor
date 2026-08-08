# Design: TypeScript ACP SDK

## Overview

The package composes the upstream ACP TypeScript client, an optional ACP HTTP stream, generated custom-method bindings, MCP Apps metadata types, and a narrow native-binary resolver. It is generated from the same canonical definitions used by the approved Sim ACP server.

## Existing context

Goose's SDK implementation lives in `projects/goose/ui/sdk/src`. Its `GooseClient` wraps `@agentclientprotocol/sdk`; it does not issue REST resource requests. Sim currently has ACP Rust/client code but no audited public TypeScript package.

## Design decisions

### D-TS-ACP-CLIENT

- Responsibility: typed standard ACP calls and lifecycle.
- Integration: compose the upstream ACP client package.
- Rationale: avoids protocol drift and duplicate framing/state logic.

### D-TS-HTTP-STREAM

- Responsibility: connection/session ACP stream routing over the approved HTTP transport.
- Integration: match the server transport contract and expose it as an ACP `Stream`.
- Rationale: transport stays separate from client methods.

### D-TS-GENERATION

- Responsibility: generate custom types, Zod validators, method client, and dispatchers from canonical definitions.
- Integration: the server and SDK share the same compatibility inventory.
- Rationale: stale hand-written bindings are detectable.

### D-TS-MCP-APPS

- Responsibility: type host capabilities and MCP app tool/resource metadata.
- Integration: rendering/security remains with the host application.
- Rationale: the SDK transports metadata but does not execute untrusted content.

### D-TS-BINARY

- Responsibility: resolve an explicitly configured or installed platform package.
- Integration: Node-only helper, separate from browser-safe exports where necessary.
- Rationale: no implicit downloads/execution.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2, 1.3, 1.4 | D-TS-ACP-CLIENT | Standard-method, unsupported-method, close, cancel, and error tests |
| 2.1, 2.2, 2.3, 2.4 | D-TS-HTTP-STREAM | Stream routing, disconnect, abort, malformed data, and cleanup tests |
| 3.1, 3.2, 3.3, 3.4 | D-TS-GENERATION | Generated-output freshness and server compatibility tests |
| 4.1, 4.2, 4.3 | D-TS-MCP-APPS | Capability negotiation and unknown-metadata tests |
| 5.1, 5.2, 5.3 | D-TS-ACP-CLIENT, D-TS-GENERATION | Version/metadata compatibility matrix tests |
| 6.1, 6.2, 6.3 | D-TS-BINARY | OS/architecture/override/missing-package/security tests |

## Testing strategy

- Run the SDK against ACP fixture streams and, if approved, the Sim stdio/HTTP server.
- Verify every generated custom method in both directions.
- Inject out-of-order/malformed messages, aborts, dropped streams, unknown methods, and newer metadata.
- Test package exports in supported Node/browser/Electron environments.
- Test binary resolution without downloading or executing binaries.
