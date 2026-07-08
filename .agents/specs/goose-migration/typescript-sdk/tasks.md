# Implementation Plan: TypeScript SDK

## Overview

Implement a TypeScript SDK as an npm package that provides programmatic access to the sim agent. The SDK supports both HTTP connections (to a remote sim-server) and local ACP connections.

## Tasks

- [x] 1. Set up npm package structure
  - Create package.json with dependencies
  - Set up TypeScript configuration
  - Set up build pipeline (tsc or esbuild)
  - Generate TypeScript types from OpenAPI schema
  - _Requirements: 6_
  - _writes: ui/sdk/package.json, ui/sdk/tsconfig.json, ui/sdk/src/generated/types.ts_

- [x] 2. Implement HTTP transport
  - HTTP client with configurable base URL and auth
  - Request/response handling for all API endpoints
  - Error handling with typed errors
  - _Requirements: 1_
  - _writes: ui/sdk/src/http-transport.ts_

- [x] 3. Implement streaming support
  - SSE client for streaming responses
  - AsyncGenerator-based streaming API
  - Reconnection with backoff
  - _Requirements: 2_
  - _writes: ui/sdk/src/stream.ts_

- [x] 4. Implement GooseClient
  - Session management (create, list, get, delete)
  - Message sending (batch + streaming)
  - Agent status
  - Recipe listing and execution
  - Connection lifecycle (connect, disconnect, state change events)
  - _Requirements: 1_
  - _writes: ui/sdk/src/goose-client.ts, ui/sdk/src/index.ts_

- [x] 5. Implement MCP apps client
  - Tool registration
  - Tool invocation
  - Lifecycle management
  - _Requirements: 3_
  - _writes: ui/sdk/src/mcp-apps.ts_

- [x] 6. Implement client capabilities
  - Declare client capabilities on connection initialization
  - Feature detection and advertisement
  - _Requirements: 4_
  - _writes: ui/sdk/src/client-capabilities.ts_

- [x] 7. Implement binary resolver
  - Find sim binary for current platform
  - Support custom binary paths
  - Version detection
  - _Requirements: 5_
  - _writes: ui/sdk/src/resolve-binary.ts_

- [x] 8. Implement ACP transport (stdio)
  - Spawn sim binary as subprocess
  - ACP protocol communication over stdio
  - Process lifecycle management
  - _Requirements: 1_
  - _writes: ui/sdk/src/acp-transport.ts_

- [x] 9. Write tests
  - Unit tests with mock HTTP server
  - Streaming tests with mock SSE
  - Binary resolver platform tests
  - Type validation tests
  - _Requirements: 1-6_

## Notes

- Published as `@sim/sdk` npm package
- HTTP mode is for remote sim-server connections
- ACP mode is for local processes (spawns sim binary)
- Generated types stay in sync with the OpenAPI schema from sim-server
