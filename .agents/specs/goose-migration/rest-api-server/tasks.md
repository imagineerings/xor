# Implementation Plan: REST API Server

## Overview

Implement an HTTP REST API server as a new `crates/baymax-server/` crate using Axum, providing endpoints for agent interaction, session management, recipes, scheduling, configuration, and system monitoring. Include OpenAPI documentation and optional tunnel support.

## Tasks

- [x] 1. Create server crate with core infrastructure
  - Set up Axum router with shared state
  - Implement TLS configuration
  - Implement authentication middleware (API key, Bearer token)
  - Implement CORS middleware
  - Error response types and error handling
  - _Requirements: 1, 13_
  - _writes: crates/baymax-server/src/lib.rs, crates/baymax-server/src/auth.rs, crates/baymax-server/src/error.rs, crates/baymax-server/src/configuration.rs_

- [x] 2. Implement agent routes
  - POST `/agent/message` — send message, receive response
  - POST `/agent/stream` — send message, stream response via SSE
  - GET `/agent/status` — agent status
  - _Requirements: 2_
  - _writes: crates/baymax-server/src/routes/agent.rs_

- [x] 3. Implement session routes
  - GET `/sessions` — list sessions with pagination
  - POST `/sessions` — create session
  - GET `/sessions/:id` — get session details
  - DELETE `/sessions/:id` — delete session
  - _Requirements: 3_
  - _writes: crates/baymax-server/src/routes/sessions.rs_

- [x] 4. Implement session event bus
  - SSE event streaming for session events
  - Publisher/subscriber pattern with session scoping
  - Reconnection support with last event ID
  - _Requirements: 4_
  - _writes: crates/baymax-server/src/event_bus.rs, crates/baymax-server/src/routes/session_events.rs_

- [x] 5. Implement recipe routes
  - GET `/recipes` — list available recipes
  - GET `/recipes/:name` — recipe details
  - POST `/recipes/:name/run` — execute recipe
  - _Requirements: 5_
  - _writes: crates/baymax-server/src/routes/recipes.rs_

- [x] 6. Implement configuration routes
  - GET `/config` — full configuration
  - PUT `/config` — update configuration
  - GET `/config/:key` — specific config value
  - _Requirements: 6_
  - _writes: crates/baymax-server/src/routes/config.rs_

- [x] 7. Implement schedule routes
  - GET `/schedules` — list schedules
  - POST `/schedules` — create schedule
  - DELETE `/schedules/:id` — delete schedule
  - _Requirements: 7_
  - _writes: crates/baymax-server/src/routes/schedules.rs_

- [x] 8. Implement system routes
  - GET `/health` — health check
  - GET `/status` — detailed status
  - GET `/telemetry` — telemetry data
  - POST `/setup` — initial setup
  - _Requirements: 9, 14_
  - _writes: crates/baymax-server/src/routes/system.rs, crates/baymax-server/src/routes/setup.rs_

- [x] 9. Implement remaining routes
  - [x] 9.1 Dictation route — POST `/dictation`
    - _Requirements: 8_
    - _writes: crates/baymax-server/src/routes/dictation.rs_
  - [x] 9.2 Gateway routes — gateway management
    - _Requirements: 10_
    - _writes: crates/baymax-server/src/routes/gateways.rs_

- [x] 10. Implement tunnel
  - Secure tunnel to relay server for external access
  - Auto-reconnect on disconnect
  - _Requirements: 11_
  - _writes: crates/baymax-server/src/tunnel.rs_

- [x] 11. Implement OpenAPI documentation
  - Define OpenAPI schema using utoipa
  - Serve `/openapi.json` and Swagger UI at `/docs`
  - _Requirements: 12_
  - _writes: crates/baymax-server/src/openapi.rs_

- [x] 12. Write tests
  - Route handler unit tests with mock state
  - Integration tests with HTTP client
  - SSE event streaming tests
  - Auth middleware tests
  - OpenAPI spec validation
  - _Requirements: 1-14_

## Notes

- The server crate is optional — compiled behind `server` feature flag
- Default port is 8443 with configurable host/port
- Auto-generated OpenAPI spec from utoipa annotations
