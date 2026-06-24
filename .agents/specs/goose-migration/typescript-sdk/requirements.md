# Requirements: TypeScript SDK

## Introduction

Migrate goose's TypeScript SDK, which provides a programmatic client for interacting with the goose agent from TypeScript/JavaScript applications. This enables embedding agent functionality into web apps, Node.js services, and other JavaScript environments.

## Glossary

- **SDK**: Software Development Kit — a library for programmatic access to goose
- **HTTP Streaming**: Receiving streaming responses over HTTP (SSE or chunked transfer)
- **MCP Apps**: Model Context Protocol apps integration
- **Client Capabilities**: Declared capabilities that the client supports
- **Binary Resolution**: Locating the correct platform-specific goose binary

## Requirements

### Requirement 1: Goose Client

**User Story:** As a TypeScript developer, I want a programmatic client for goose, so that I can integrate agent functionality into my applications.

#### Acceptance Criteria

1. THE TypeScript SDK SHALL provide a GooseClient class
2. THE GooseClient SHALL support sending messages to the agent
3. THE GooseClient SHALL support receiving responses, both batch and streaming
4. THE GooseClient SHALL support managing sessions (create, list, delete)
5. THE GooseClient SHALL handle connection errors gracefully

### Requirement 2: HTTP Streaming

**User Story:** As a TypeScript developer, I want to receive streaming responses from the agent, so that I can show real-time progress to users.

#### Acceptance Criteria

1. THE SDK SHALL support HTTP streaming (SSE)
2. WHEN a streaming request is made THEN the client SHALL emit events as data arrives
3. THE SDK SHALL handle reconnection for dropped streams

### Requirement 3: MCP Apps Integration

**User Story:** As a TypeScript developer, I want to integrate with MCP apps via the SDK, so that I can build extensions that interact with the agent.

#### Acceptance Criteria

1. THE SDK SHALL support MCP apps integration
2. THE SDK SHALL expose methods for tool registration and invocation

### Requirement 4: Client Capabilities

**User Story:** As an SDK consumer, I want the client to declare its capabilities, so that the server can adapt its behavior accordingly.

#### Acceptance Criteria

1. THE SDK SHALL support declaring client capabilities
2. THE capabilities SHALL be communicated during connection initialization

### Requirement 5: Binary Resolution

**User Story:** As the SDK, I want to resolve the correct goose binary for the current platform, so that I can launch the agent process correctly.

#### Acceptance Criteria

1. THE SDK SHALL locate the goose binary for the current OS and architecture
2. THE SDK SHALL support custom binary paths via configuration
3. IF the binary is not found THEN the SDK SHALL return a clear error

### Requirement 6: Generated Client Bindings

**User Story:** As a TypeScript developer, I want generated client bindings matching the API, so that I have type-safe access to all endpoints.

#### Acceptance Criteria

1. THE SDK SHALL include generated TypeScript types for all API requests and responses
2. THE generated types SHALL be derived from the OpenAPI schema or canonical definitions

## References

- Source: `goose/ui/sdk/` — TypeScript SDK implementation
- Key files: goose-client.ts, http-stream.ts, mcp-apps.ts, resolve-binary.ts, client-capabilities.ts, index.ts
- Source: `goose/ui/sdk/src/generated/` — generated schema types
- Source: `goose/crates/goose-sdk/` — Rust SDK bindings
- Source: `goose/crates/goose-sdk-types/` — SDK type definitions
