# Requirements: TypeScript ACP SDK

## Problem

Goose's TypeScript package is an ACP client plus generated Goose custom-method types and validators. The previous specification incorrectly treated it as a REST/OpenAPI SDK. Zed needs a source-compatible plan that follows the actual protocol boundary.

## Requirements

### Requirement 1: Standard ACP client

**User story:** As a TypeScript developer, I want a typed ACP client, so that I can control an approved Zed ACP server without hand-written protocol messages.

#### Acceptance criteria

1. **1.1** THE SDK SHALL wrap the upstream ACP client connection rather than define a competing protocol.
2. **1.2** THE client SHALL expose initialization, authentication, new/load/list/fork/resume/close session, prompt, cancel, mode, model, and configuration operations supported by the server.
3. **1.3** THE client SHALL surface protocol and transport failures as rejected typed operations and expose connection closure/cancellation state.
4. **1.4** THE client SHALL NOT claim support for a method the server marks unsupported.

### Requirement 2: Stream transports

**User story:** As a TypeScript developer, I want to supply an ACP stream or connect through the approved HTTP transport, so that the same client works locally and remotely.

#### Acceptance criteria

1. **2.1** THE client SHALL accept an upstream ACP `Stream` implementation.
2. **2.2** WHERE HTTP transport is supported, THE SDK SHALL provide the connection-scoped and session-scoped ACP HTTP stream behavior used by the server.
3. **2.3** THE HTTP stream SHALL correlate connection/session identifiers and route requests, notifications, and responses to the correct stream.
4. **2.4** IF a stream closes, aborts, reconnects, or receives malformed data, THEN THE client SHALL resolve/reject pending operations deterministically and release resources.

### Requirement 3: Generated custom-method client

**User story:** As a TypeScript developer, I want generated types and dispatch for approved Zed extensions, so that client and server stay compatible.

#### Acceptance criteria

1. **3.1** THE SDK SHALL generate TypeScript request, response, notification, and DTO types from canonical ACP/custom definitions.
2. **3.2** THE SDK SHALL generate runtime validators for external custom-method data.
3. **3.3** THE SDK SHALL generate a typed custom-method client and request/notification dispatchers.
4. **3.4** THE generation check SHALL fail when committed output is stale or a method lacks a stable compatibility decision.

### Requirement 4: MCP Apps metadata

**User story:** As an MCP Apps host, I want typed capability and resource metadata, so that app tools can be rendered safely by a compatible client.

#### Acceptance criteria

1. **4.1** WHERE MCP Apps is approved, THE SDK SHALL declare supported MIME types and host capabilities during initialization.
2. **4.2** THE SDK SHALL expose typed tool/resource/update metadata without executing untrusted app content itself.
3. **4.3** IF MCP Apps is unsupported, THEN THE SDK SHALL omit the capability and preserve unknown extension metadata safely.

### Requirement 5: Client capabilities and compatibility

**User story:** As a server implementer, I want explicit client capabilities and version compatibility, so that optional behavior is negotiated rather than assumed.

#### Acceptance criteria

1. **5.1** THE SDK SHALL communicate supported Goose/Zed extensions through ACP initialization metadata.
2. **5.2** THE SDK SHALL preserve unknown metadata fields and tolerate newer optional methods without crashing.
3. **5.3** THE package SHALL document the compatible ACP, server, Node.js, browser/Electron, OS, and architecture versions.

### Requirement 6: Binary resolution

**User story:** As a Node.js SDK user, I want deterministic native-binary resolution, so that I can launch an approved local server package.

#### Acceptance criteria

1. **6.1** WHERE native binary packages are published, THE resolver SHALL prefer an explicit environment/configuration override and otherwise select the matching OS/architecture package.
2. **6.2** IF the platform is unsupported or the optional binary package is absent, THEN THE resolver SHALL return an actionable error.
3. **6.3** THE resolver SHALL NOT download or execute an unverified binary implicitly.

## Open questions

- Will Zed publish this SDK and native binary packages, or only document the protocol?
- Which custom methods are stable compatibility commitments?
- Which environments (Node, browser, Electron) must be supported?

## Evidence

- `projects/goose/ui/sdk/src/goose-client.ts` — `GooseClient`.
- `projects/goose/ui/sdk/src/http-stream.ts` — `createHttpStream`.
- `projects/goose/ui/sdk/src/generated/*` — generated types, validators, client, and method inventory.
- `projects/goose/ui/sdk/src/mcp-apps.ts`, `client-capabilities.ts`, `resolve-binary.ts`.
