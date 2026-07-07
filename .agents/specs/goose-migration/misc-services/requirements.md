# Requirements: Miscellaneous Services

## Introduction

Migrate the remaining goose services, scripts, and examples: the Ask AI bot service, development/CI scripts, Nostr session sharing, session import formats, examples, and the provider error proxy.

## Glossary

- **Ask AI Bot**: A service bot for answering questions about goose
- **Nostr**: Open protocol for decentralized social networking — used for sharing sessions
- **Provider Error Proxy**: A proxy server for intercepting and debugging provider API errors
- **Session Import Format**: Format for importing sessions from other tools or formats

## Requirements

### Requirement 1: Ask AI Bot Service

**User Story:** As a sim user, I want a Q&A bot that can answer questions about using the agent, so that I can get help without reading all documentation.

#### Acceptance Criteria

1. THE Ask AI bot SHALL answer questions about the agent's features and usage
2. THE Ask AI bot SHALL be configurable with documentation sources
3. THE Ask AI bot SHALL provide responses based on the configured knowledge base

### Requirement 2: Session Import Formats

**User Story:** As a sim user, I want to import sessions from other formats, so that I can migrate conversations from other tools.

#### Acceptance Criteria

1. THE system SHALL support importing sessions from defined import formats
2. WHEN a session is imported THEN it SHALL be converted to the native format
3. IF the import format is not recognized THEN the system SHALL return an error
4. THE import system SHALL validate imported data

### Requirement 3: Nostr Session Sharing

**User Story:** As a sim user, I want to share agent sessions via Nostr, so that I can publish and discover sessions on a decentralized network.

#### Acceptance Criteria

1. THE Nostr sharing module SHALL publish session data to Nostr relays
2. THE Nostr sharing module SHALL support importing sessions from Nostr
3. THE Nostr sharing SHALL be opt-in

### Requirement 4: Examples

**User Story:** As a sim developer, I want example integrations, so that I can see how to use and extend the agent.

#### Acceptance Criteria

1. THE examples SHALL include MCP integration examples
2. THE examples SHALL include plugin examples
3. THE examples SHALL include frontend tool examples
4. THE examples SHALL be documented and runnable

### Requirement 5: Development and CI Scripts

**User Story:** As a sim developer, I want scripts for common development tasks, so that development workflows are automated.

#### Acceptance Criteria

1. THE scripts SHALL include a Windows build script
2. THE scripts SHALL include an OpenAPI schema validation script
3. THE scripts SHALL include a diagnostics viewer
4. THE scripts SHALL include a database helper script
5. THE scripts SHALL include MCP testing scripts
6. THE scripts SHALL include sub-agent and sub-recipe testing scripts
7. THE scripts SHALL include a pre-release script
8. THE scripts SHALL include compaction testing scripts

### Requirement 6: Provider Error Proxy

**User Story:** As a sim developer, I want a proxy for intercepting and debugging provider API errors, so that I can diagnose provider integration issues.

#### Acceptance Criteria

1. THE provider error proxy SHALL intercept requests to LLM providers
2. THE provider error proxy SHALL log request and response details
3. THE provider error proxy SHALL forward requests to the actual provider

## References

- Source: `projects/goose/services/ask-ai-bot/`
- Source: `projects/goose/crates/goose/src/session/import_formats/`
- Source: `projects/goose/crates/goose/src/session/nostr_share.rs`
- Source: `projects/goose/examples/`
- Source: `projects/goose/scripts/`
- Source: `projects/goose/scripts/provider-error-proxy/`
