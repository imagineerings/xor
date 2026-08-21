# Implementation Plan: Gateway System

## Overview

Implement the multi-channel gateway system as a new `crates/gateway/` crate, starting with the Telegram gateway. The gateway manager routes messages between external platforms and the agent.

## Tasks

- [ ] 1. Create gateway crate with core types and traits
  - Define GatewayHandler trait, IncomingMessage, OutgoingMessage types
  - Define GatewayManager as GPUI Entity

  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/gateway/requirements.md, .agents/specs/goose-migration/gateway/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/gateway/src/lib.rs, crates/gateway/src/types.rs_
  - _Writes: crates/gateway/src/lib.rs, crates/gateway/src/types.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Implement gateway manager
  - Handler registration/unregistration
  - Message routing (incoming → agent, agent → outgoing)
  - Error handling and logging

  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/gateway/requirements.md, .agents/specs/goose-migration/gateway/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/gateway/src/manager.rs_
  - _Writes: crates/gateway/src/manager.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Implement Telegram gateway
  - Telegram Bot API client (getUpdates polling or webhook)
  - Message receiving and sending
  - Handle Telegram-specific message types (text, media, documents)

  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/gateway/requirements.md, .agents/specs/goose-migration/gateway/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/gateway/src/telegram.rs_
  - _Writes: crates/gateway/src/telegram.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Implement message formatter
  - Convert agent markdown to Telegram-compatible format (HTML/MarkdownV2)
  - Split long messages per Telegram's length limits
  - Format code blocks, links, and rich content

  - _Requirements: 4.1, 4.2, 4.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/gateway/requirements.md, .agents/specs/goose-migration/gateway/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/gateway/src/telegram_format.rs_
  - _Writes: crates/gateway/src/telegram_format.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Implement pairing service
  - Link external platform user IDs to zed user identities
  - Persistent storage for pairings
  - Unlink support

  - _Requirements: 3.1, 3.2, 3.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/gateway/requirements.md, .agents/specs/goose-migration/gateway/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/gateway/src/pairing.rs_
  - _Writes: crates/gateway/src/pairing.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Integrate gateway into zed application
  - Gateway manager initialization during app startup
  - CLI command for gateway configuration
  - Configuration via settings files

  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/gateway/requirements.md, .agents/specs/goose-migration/gateway/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/commands/gateway.rs_
  - _Writes: crates/cli/src/commands/gateway.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 7. Write tests
  - Unit tests: message formatting, pairing logic, manager routing
  - Integration tests: mock Telegram API server
  - E2E tests: full message round-trip with mock agent

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 4.1, 4.2, 4.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/gateway/requirements.md, .agents/specs/goose-migration/gateway/design.md, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: none_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

## Notes

- Telegram gateway is the first implementation; the trait system allows adding more platforms
- Gateway is optional — compiled behind a Cargo feature flag
- Telegram bot token is configured via settings or environment variable
