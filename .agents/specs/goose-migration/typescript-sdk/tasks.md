# Implementation Plan: TypeScript ACP SDK

> Cross-cutting contract: every production write in this plan inherits the [`agentic` feature boundary](../feature-boundary.md). Completion evidence must classify actual writes and include the required enabled/disabled validation.

## Approach

Publish this package only after the SDK product decision and ACP server compatibility set are approved. Compose the upstream ACP client and generate extension bindings from canonical definitions.

## Tasks

- [ ] 1. Define package exports and compatibility metadata
  - Separate browser-safe protocol exports from Node-only binary helpers.
  - _Requirements: 5.3, 6.1, 6.2, 6.3_
  - _Depends on: none_
  - _Reads: projects/goose/ui/sdk/package.json, projects/goose/ui/sdk/src/index.ts, projects/goose/ui/sdk/src/resolve-binary.ts_
  - _Writes: approved TypeScript SDK package manifest, exports, and compatibility documentation_
  - _Validation: package build/typecheck plus Node/browser import and unsupported-platform tests_

- [ ] 2. Wrap the upstream ACP client
  - Expose only negotiated standard methods and deterministic connection close/cancellation state.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 5.1, 5.2_
  - _Depends on: 1_
  - _Reads: projects/goose/ui/sdk/src/goose-client.ts, upstream ACP TypeScript client API_
  - _Writes: TypeScript SDK ACP client and lifecycle tests_
  - _Validation: standard-method fixture tests plus unsupported method, close, cancel, and transport-error tests_

- [ ] 3. Implement the approved ACP HTTP stream adapter
  - Route connection/session messages and clean up pending work on abort/disconnect/malformed input.
  - _Requirements: 2.2, 2.3, 2.4_
  - _Depends on: 2_
  - _Reads: projects/goose/ui/sdk/src/http-stream.ts, approved Zed ACP HTTP transport design_
  - _Writes: TypeScript SDK HTTP stream adapter and transport tests_
  - _Validation: server compatibility, stream correlation, abort, disconnect, malformed-message, and cleanup tests_

- [ ] 4. Generate custom-method types, validators, client, and dispatchers
  - Use the approved server method inventory as the single source of truth.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 5.1, 5.2_
  - _Depends on: 2_
  - _Reads: projects/goose/ui/sdk/src/generated, projects/goose/crates/goose-sdk-types/src, approved Zed ACP schema definitions_
  - _Writes: SDK generation script and committed generated TypeScript outputs_
  - _Validation: generation freshness check and request/response/notification compatibility tests for every approved method_

- [ ] 5. Add MCP Apps capability and metadata types
  - Keep execution/rendering outside the SDK and omit unsupported capabilities.
  - _Requirements: 4.1, 4.2, 4.3, 5.1, 5.2_
  - _Depends on: 2, 4_
  - _Reads: projects/goose/ui/sdk/src/mcp-apps.ts, MCP Apps extension definitions_
  - _Writes: SDK MCP Apps capability/metadata modules and tests_
  - _Validation: capability negotiation, unknown metadata, unsupported capability, and type/runtime validation tests_

- [ ] 6. Add safe platform binary resolution
  - Resolve explicit overrides and installed optional packages without implicit downloads.
  - _Requirements: 6.1, 6.2, 6.3_
  - _Depends on: 1_
  - _Reads: projects/goose/ui/sdk/src/resolve-binary.ts, approved Zed native package matrix_
  - _Writes: Node-only binary resolver and platform fixture tests_
  - _Validation: override, supported package, unsupported OS/architecture, missing package, and no-download tests_

- [ ] 7. Add end-to-end SDK/server compatibility coverage
  - Exercise standard ACP, approved custom methods, streams, capabilities, and failure behavior.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3_
  - _Depends on: 2, 3, 4, 5, 6_
  - _Reads: all SDK source, approved Zed ACP server fixtures, Goose SDK compatibility fixtures_
  - _Writes: TypeScript SDK end-to-end fixtures and compatibility matrix_
  - _Validation: package tests/typecheck and end-to-end compatibility suite against each approved transport_
