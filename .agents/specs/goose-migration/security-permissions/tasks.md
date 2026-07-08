# Implementation Plan: Security and Permissions

## Overview

Implement the security inspection system (`crates/security/`) and permission system (`crates/permission/`) as new crates that layer on top of sim's existing `crates/sandbox/` and `crates/agent/src/tool_permissions.rs`.

## Tasks

- [x] 1. Create security pattern registry
  - Define Pattern, PatternCategory, PatternAction types
  - Load patterns from configuration files
  - Compile regex/matcher patterns at load time
  - _Requirements: 5_
  - _writes: crates/security/src/patterns.rs_

- [x] 2. Implement adversary inspector
  - Detect prompt injection, jailbreak attempts, indirect injection
  - Configurable sensitivity levels
  - Use pattern registry for detection
  - _Requirements: 1_
  - _writes: crates/security/src/adversary_inspector.rs_

- [x] 3. Implement egress inspector
  - Detect API keys, secrets, PII in outgoing content
  - Configurable redaction strategies (block vs. redact)
  - _Requirements: 2_
  - _writes: crates/security/src/egress_inspector.rs_

- [x] 4. Implement classification client
  - Client for content moderation/safety APIs
  - Configurable thresholds and actions
  - Error handling for API unavailability
  - _Requirements: 3_
  - _writes: crates/security/src/classification_client.rs_

- [ ] 5. Implement security scanner
  - Unified scanner that orchestrates all inspectors
  - Aggregated result reporting
  - Configurable fail-open vs. fail-closed mode
  - _Requirements: 4_
  - _writes: crates/security/src/scanner.rs, crates/security/src/lib.rs_

- [ ] 6. Implement permission store
  - Persistent storage for permission decisions (SQLite/db)
  - Store tool name, args pattern, decision type, expiration
  - CRUD operations for stored decisions
  - _Requirements: 9_
  - _writes: crates/permission/src/store.rs_

- [ ] 7. Implement permission inspector and judge
  - Examine tool calls against stored decisions
  - Classify risk level (low/medium/high)
  - Auto-allow low risk, auto-block high risk, prompt for medium
  - _Requirements: 7, 8_
  - _writes: crates/permission/src/inspector.rs, crates/permission/src/judge.rs_

- [ ] 8. Implement permission confirmation UI (GPUI)
  - Confirmation dialog showing tool name, arguments, risk level
  - Allow/Deny/Always-Allow/Always-Deny actions
  - _Requirements: 6_
  - _writes: crates/permission/src/confirmation.rs_

- [ ] 9. Integrate security scanner and permission system into agent
  - Hook security scanner into agent input/output processing
  - Hook permission inspector into tool execution pipeline (alongside existing tool_permissions.rs)
  - _Requirements: 1-9_
  - _writes: crates/agent/src/security_integration.rs, crates/agent/src/permission_integration.rs_

- [ ] 10. Write tests
  - Pattern matching accuracy tests
  - Inspector tests with known-good and known-bad inputs
  - Permission store persistence and expiry tests
  - Integration tests with mock agent
  - _Requirements: 1-9_

## Notes

- The security systems are optional — disabled by default, enabled via configuration
- Permission system extends (does not replace) existing `tool_permissions.rs`
