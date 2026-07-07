# Requirements: Gateway System

## Introduction

Migrate the goose gateway system — a multi-channel communication layer that allows the agent to operate through different messaging platforms. The primary implementation is a Telegram bot that enables users to interact with the agent through Telegram.

## Glossary

- **Gateway**: A communication channel that connects the agent to an external messaging platform
- **Gateway Handler**: Processes messages from a platform, routes them to the agent, and sends responses back
- **Gateway Manager**: Manages lifecycle, configuration, and routing for all gateway instances
- **Pairing**: Process of linking an external platform user to a goose user identity
- **Telegram Bot**: A bot running on the Telegram messaging platform

## Requirements

### Requirement 1: Gateway Manager

**User Story:** As a sim user, I want a central gateway manager, so that I can manage multiple communication channels for the agent.

#### Acceptance Criteria

1. THE gateway manager SHALL support registering and unregistering gateway handlers
2. THE gateway manager SHALL route incoming messages from gateways to the agent
3. THE gateway manager SHALL route outgoing responses from the agent back to the appropriate gateway
4. WHEN a gateway handler errors THEN the manager SHALL log the error and continue operating

### Requirement 2: Telegram Gateway

**User Story:** As a sim user, I want to interact with the agent through Telegram, so that I can use goose from my phone or any device with Telegram.

#### Acceptance Criteria

1. WHEN a message is received from Telegram THEN the system SHALL process it through the agent
2. WHEN the agent produces a response THEN the system SHALL send it back to the Telegram chat
3. THE Telegram gateway SHALL support text messages and media types supported by Telegram
4. WHEN the Telegram bot starts THEN the system SHALL authenticate with the Telegram API

### Requirement 3: Gateway Pairing

**User Story:** As a Telegram user, I want to pair my Telegram account with my goose identity, so that the agent knows who I am.

#### Acceptance Criteria

1. THE gateway pairing system SHALL support linking an external platform user to a goose user
2. WHEN a user sends a pairing command THEN the system SHALL associate the external ID with the goose user
3. THE pairing SHALL be persistent across sessions

### Requirement 4: Telegram Message Formatting

**User Story:** As a Telegram user, I want agent responses to be properly formatted for Telegram, so that markdown, code blocks, and rich content display correctly.

#### Acceptance Criteria

1. THE Telegram formatter SHALL convert agent responses to Telegram-compatible formatting
2. THE formatter SHALL handle markdown, code blocks, and links appropriately
3. IF the response exceeds Telegram's message length limit THEN the system SHALL split it into multiple messages

## References

- Source: `projects/goose/crates/goose/src/gateway/` — handler.rs, manager.rs, pairing.rs, telegram.rs, telegram_format.rs
