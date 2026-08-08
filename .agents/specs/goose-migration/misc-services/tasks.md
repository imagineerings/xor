# Implementation Plan: Miscellaneous Services

## Overview

Implement the remaining smaller goose components: session import formats, Nostr sharing, Ask AI bot, examples, dev/CI scripts, and provider error proxy.

## Tasks

- [ ] 0. Resolve and, if approved, specify the Ask AI service boundary
  - Confirm whether Sim needs a separately deployed documentation Q&A service or whether existing in-product help and documentation search are the intended owner
  - If approved, define source ingestion, freshness, citation, authentication, privacy, deployment, and failure behavior before implementation
  - Do not create a standalone service until that product and operational boundary is approved

  - _Requirements: 1.1, 1.2, 1.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/misc-services/requirements.md, .agents/specs/goose-migration/misc-services/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/services/ask-ai-bot/, crates/assistant/, docs/_
  - _Writes: .agents/specs/goose-migration/misc-services/requirements.md, .agents/specs/goose-migration/misc-services/design.md_
  - _Validation: Record the approved ownership and deployment decision, then validate source freshness, citation, authentication, privacy, and unavailable-service scenarios in the resulting design_

- [ ] 1. Implement session import formats
  - Extend the existing thread import pipeline with explicit Claude Code, Codex, and Pi JSONL adapters
  - Convert supported messages, roles, tools/results, timestamps, metadata, and attachments without executing imported content
  - Validate size/depth/path limits, partial records, duplicate IDs, unsupported events, tool pairing, and atomic failure

  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/misc-services/requirements.md, .agents/specs/goose-migration/misc-services/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/session/import_formats/, crates/agent_ui/src/thread_import.rs, crates/agent/src/db.rs_
  - _Writes: crates/agent_ui/src/thread_import.rs, selected thread import adapters_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Resolve and, if approved, implement optional Nostr session sharing
  - Create Nostr client for publishing/retrieving session events
  - Session serialization/deserialization for Nostr events
  - Configurable relay list

  - _Requirements: 3.1, 3.2, 3.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/misc-services/requirements.md, .agents/specs/goose-migration/misc-services/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/session/nostr_share.rs, existing shared-session serialization_
  - _Writes: selected shared-session transport adapter only if approved_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Migrate examples
  - MCP wiki integration example
  - Plugin usage example
  - Frontend tools example
  - Ensure examples are documented and runnable

  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/misc-services/requirements.md, .agents/specs/goose-migration/misc-services/design.md, .agents/specs/goose-migration/coverage-catalog.md, examples/_
  - _Writes: examples/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Reconcile development and CI workflows with existing Sim tooling
  - Windows build script
  - Exclude the obsolete REST/OpenAPI validation premise; validate ACP/generated SDK contracts in their owning specs
  - Diagnostics viewer
  - Database helper script
  - MCP testing scripts
  - Sub-agent and sub-recipe testing scripts
  - Pre-release script
  - Compaction testing script

  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/misc-services/requirements.md, .agents/specs/goose-migration/misc-services/design.md, .agents/specs/goose-migration/coverage-catalog.md, scripts/_
  - _Writes: scripts/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Decide whether a standalone provider error proxy remains necessary
  - HTTP proxy that intercepts provider API calls
  - Redacts secrets and user content while preserving bounded status/timing/retry/stream diagnostics
  - Forwards to actual provider
  - Useful for debugging provider integration issues

  - _Requirements: 6.1, 6.2, 6.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/misc-services/requirements.md, .agents/specs/goose-migration/misc-services/design.md, .agents/specs/goose-migration/coverage-catalog.md, scripts/provider-error-proxy/_
  - _Writes: scripts/provider-error-proxy/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Write tests
  - Import format detection and parsing
  - Nostr event serialization/round-trip
  - Script validation (CI integration)
  - Example verification

  - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 6.1, 6.2, 6.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/misc-services/requirements.md, .agents/specs/goose-migration/misc-services/design.md, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: none_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

## Notes

- Nostr sharing is optional — behind a Cargo feature flag
- Examples are standalone files, not part of the compiled application
- Scripts are documented in `scripts/README.md`
- Provider error proxy is a development tool, not shipped to users
