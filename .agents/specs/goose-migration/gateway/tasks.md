# Implementation Plan: Gateway System

## Overview

Implement the multi-channel gateway system as a new `crates/gateway/` crate, starting with the Telegram gateway. The gateway manager routes messages between external platforms and the agent.

## Tasks

- [x] 1. Create gateway crate with core types and traits
  - Define GatewayHandler trait, IncomingMessage, OutgoingMessage types
  - Define GatewayManager as GPUI Entity
  - _Requirements: 1_
  - _writes: crates/gateway/src/lib.rs, crates/gateway/src/types.rs_

- [x] 2. Implement gateway manager
  - Handler registration/unregistration
  - Message routing (incoming → agent, agent → outgoing)
  - Error handling and logging
  - _Requirements: 1_
  - _writes: crates/gateway/src/manager.rs_

- [x] 3. Implement Telegram gateway
  - Telegram Bot API client (getUpdates polling or webhook)
  - Message receiving and sending
  - Handle Telegram-specific message types (text, media, documents)
  - _Requirements: 2_
  - _writes: crates/gateway/src/telegram.rs_

- [x] 4. Implement message formatter
  - Convert agent markdown to Telegram-compatible format (HTML/MarkdownV2)
  - Split long messages per Telegram's length limits
  - Format code blocks, links, and rich content
  - _Requirements: 4_
  - _writes: crates/gateway/src/telegram_format.rs_

- [x] 5. Implement pairing service
  - Link external platform user IDs to sim user identities
  - Persistent storage for pairings
  - Unlink support
  - _Requirements: 3_
  - _writes: crates/gateway/src/pairing.rs_

- [x] 6. Integrate gateway into sim application
  - Gateway manager initialization during app startup
  - CLI command for gateway configuration
  - Configuration via settings files
  - _Requirements: 1_
  - _writes: crates/cli/src/commands/gateway.rs_

- [x] 7. Write tests
  - Unit tests: message formatting, pairing logic, manager routing
  - Integration tests: mock Telegram API server
  - E2E tests: full message round-trip with mock agent
  - _Requirements: 1-4_

## Notes

- Telegram gateway is the first implementation; the trait system allows adding more platforms
- Gateway is optional — compiled behind a Cargo feature flag
- Telegram bot token is configured via settings or environment variable
