# Implementation Plan: Miscellaneous Services

## Overview

Implement the remaining smaller goose components: session import formats, Nostr sharing, Ask AI bot, examples, dev/CI scripts, and provider error proxy.

## Tasks

- [x] 1. Implement session import formats
  - Define ImportFormat trait
  - Implement import from common formats (JSON, Markdown, Goose legacy)
  - Auto-detection of format from content or file extension
  - Validation of imported data
  - _Requirements: 2_
  - _writes: crates/session/src/import/mod.rs, crates/session/src/import/formats/_

- [x] 2. Implement Nostr session sharing
  - Create Nostr client for publishing/retrieving session events
  - Session serialization/deserialization for Nostr events
  - Configurable relay list
  - _Requirements: 3_
  - _writes: crates/nostr_sharing/src/lib.rs_

- [x] 3. Migrate examples
  - MCP wiki integration example
  - Plugin usage example
  - Frontend tools example
  - Ensure examples are documented and runnable
  - _Requirements: 4_
  - _writes: examples/_

- [x] 4. Migrate development/CI scripts
  - Windows build script
  - OpenAPI schema validation script
  - Diagnostics viewer
  - Database helper script
  - MCP testing scripts
  - Sub-agent and sub-recipe testing scripts
  - Pre-release script
  - Compaction testing script
  - _Requirements: 5_
  - _writes: scripts/_

- [x] 5. Implement provider error proxy
  - HTTP proxy that intercepts provider API calls
  - Logs request and response details
  - Forwards to actual provider
  - Useful for debugging provider integration issues
  - _Requirements: 6_
  - _writes: scripts/provider-error-proxy/_

- [ ] 6. Write tests
  - Import format detection and parsing
  - Nostr event serialization/round-trip
  - Script validation (CI integration)
  - Example verification
  - _Requirements: 2-6_

## Notes

- Nostr sharing is optional — behind a Cargo feature flag
- Examples are standalone files, not part of the compiled application
- Scripts are documented in `scripts/README.md`
- Provider error proxy is a development tool, not shipped to users
