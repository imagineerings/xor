# Requirements: REST API Server

## Introduction

Migrate goose's HTTP REST API server (`goose-server`), which provides a full HTTP API for interacting with the agent, managing sessions, recipes, schedules, and more. This enables embedding goose functionality into other applications and services.

## Glossary

- **REST API**: HTTP-based API following REST principles
- **Session Event Bus**: Pub/sub mechanism for streaming session events via SSE (Server-Sent Events)
- **Tunnel**: Secure tunnel for exposing the server to external networks
- **OpenAPI**: Standard format for API documentation (OpenAPI/Swagger)
- **SSE**: Server-Sent Events, for streaming real-time events over HTTP
- **TLS**: Transport Layer Security for HTTPS

## Requirements

### Requirement 1: HTTP API Server

**User Story:** As a baymax developer, I want an HTTP API server, so that I can integrate baymax into other applications and services.

#### Acceptance Criteria

1. THE server SHALL start an HTTP listener on a configurable host and port
2. THE server SHALL support HTTPS via TLS configuration
3. THE server SHALL support authentication for API endpoints
4. THE server SHALL support CORS for cross-origin requests
5. WHEN the server starts THEN it SHALL log the listening address and configuration

### Requirement 2: Agent Routes

**User Story:** As an API consumer, I want to interact with the agent via REST endpoints, so that I can send messages and receive responses.

#### Acceptance Criteria

1. POST `/agent/message` SHALL accept a message and return the agent's response
2. POST `/agent/stream` SHALL accept a message and stream the response via SSE
3. GET `/agent/status` SHALL return the agent's current status
4. IF the agent is busy THEN the server SHALL queue or reject the request as configured

### Requirement 3: Session Routes

**User Story:** As an API consumer, I want to manage agent sessions via REST endpoints, so that I can create, list, and manage sessions remotely.

#### Acceptance Criteria

1. GET `/sessions` SHALL list all sessions
2. POST `/sessions` SHALL create a new session
3. GET `/sessions/:id` SHALL return session details
4. DELETE `/sessions/:id` SHALL delete a session
5. THE session routes SHALL support query parameters for filtering and pagination
6. WHEN a session is deleted THEN its associated data SHALL be cleaned up

### Requirement 4: Session Events

**User Story:** As an API consumer, I want to subscribe to session events in real-time, so that I can react to agent state changes.

#### Acceptance Criteria

1. GET `/sessions/:id/events` SHALL stream session events via SSE
2. THE event stream SHALL include message turn events, tool call events, and status changes
3. THE event stream SHALL support reconnection with last event ID

### Requirement 5: Recipe Routes

**User Story:** As an API consumer, I want to manage and run recipes via REST endpoints, so that I can integrate recipes into workflows.

#### Acceptance Criteria

1. GET `/recipes` SHALL list available recipes
2. GET `/recipes/:name` SHALL return recipe details
3. POST `/recipes/:name/run` SHALL execute a recipe
4. IF a recipe is not found THEN the API SHALL return 404

### Requirement 6: Configuration Routes

**User Story:** As an API consumer, I want to read and update configuration via REST, so that I can manage the agent remotely.

#### Acceptance Criteria

1. GET `/config` SHALL return the current configuration
2. PUT `/config` SHALL update the configuration
3. GET `/config/:key` SHALL return a specific configuration value
4. IF invalid configuration is provided THEN the API SHALL return validation errors

### Requirement 7: Schedule Routes

**User Story:** As an API consumer, I want to manage scheduled agent tasks via REST, so that I can automate recurring operations.

#### Acceptance Criteria

1. GET `/schedules` SHALL list all schedules
2. POST `/schedules` SHALL create a new schedule
3. DELETE `/schedules/:id` SHALL delete a schedule
4. THE schedule SHALL support cron expressions for timing

### Requirement 8: Dictation Routes

**User Story:** As an API consumer, I want to send audio for transcription via REST, so that I can use speech-to-text remotely.

#### Acceptance Criteria

1. POST `/dictation` SHALL accept audio and return transcribed text
2. THE dictation endpoint SHALL support common audio formats

### Requirement 9: Status and Telemetry Routes

**User Story:** As an API consumer, I want to check server health and get telemetry data, so that I can monitor the agent remotely.

#### Acceptance Criteria

1. GET `/health` SHALL return server health status
2. GET `/status` SHALL return detailed system status (provider connectivity, active sessions, etc.)
3. GET `/telemetry` SHALL return telemetry data (usage statistics, performance metrics)

### Requirement 10: Gateway Routes

**User Story:** As an API consumer, I want to manage gateway connections via REST, so that I can configure and monitor gateway channels.

#### Acceptance Criteria

1. GET `/gateways` SHALL list configured gateways
2. POST `/gateways` SHALL configure a new gateway
3. DELETE `/gateways/:id` SHALL remove a gateway

### Requirement 11: Tunnel

**User Story:** As a baymax user, I want to expose the API server securely through a tunnel, so that I can access it from outside my local network.

#### Acceptance Criteria

1. THE tunnel SHALL establish a secure connection to a relay server
2. THE tunnel SHALL provide a public URL that forwards to the local server
3. IF the tunnel connection drops THEN it SHALL automatically reconnect

### Requirement 12: OpenAPI Documentation

**User Story:** As an API consumer, I want OpenAPI documentation for the API, so that I can explore and test endpoints.

#### Acceptance Criteria

1. GET `/openapi.json` SHALL return the OpenAPI specification
2. GET `/docs` SHALL serve an interactive API documentation UI (Swagger UI or similar)
3. THE OpenAPI spec SHALL accurately describe all routes, parameters, and response schemas

### Requirement 13: Authentication and Authorization

**User Story:** As a baymax operator, I want the API server to require authentication, so that unauthorized users cannot access it.

#### Acceptance Criteria

1. THE server SHALL support API key authentication
2. THE server SHALL support bearer token authentication
3. IF a request lacks valid authentication THEN the server SHALL return 401 Unauthorized
4. IF a request is authenticated but lacks permissions THEN the server SHALL return 403 Forbidden

### Requirement 14: Setup Route

**User Story:** As a baymax user, I want a first-time setup endpoint, so that I can configure the server from a client application.

#### Acceptance Criteria

1. GET `/setup` SHALL return whether setup is required
2. POST `/setup` SHALL accept initial configuration (provider keys, etc.)
3. THE setup endpoint SHALL be unavailable after initial configuration

## References

- Source: `projects/goose/crates/goose-server/` — main.rs, lib.rs, configuration.rs, auth.rs, error.rs, logging.rs, openapi.rs, state.rs, tls.rs, session_event_bus.rs
- Source: `projects/goose/crates/goose-server/src/routes/` — agent.rs, session.rs, session_events.rs, recipe.rs, config_management.rs, schedule.rs, dictation.rs, gateway.rs, status.rs, telemetry.rs, setup.rs, action_required.rs, features.rs, local_inference.rs, mcp_app_proxy.rs, mcp_ui_proxy.rs, prompts.rs, reply.rs, sampling.rs, tunnel.rs, utils.rs, errors.rs
- Source: `projects/goose/crates/goose-server/src/commands/` — agent.rs
