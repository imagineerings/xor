# Implementation Plan: Security and Permissions

## Overview

Extend Zed's existing tool-permission, ACP confirmation, sandbox, HTTP, settings, and agent UI boundaries. Add a separate component only if implementation review proves the existing owner cannot maintain the security boundary.

## Tasks

- [ ] 1. Create security pattern registry
  - Define Pattern, PatternCategory, PatternAction types
  - Load patterns from configuration files
  - Compile regex/matcher patterns at load time

  - _Requirements: 5.1, 5.2, 5.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/security-permissions/requirements.md, .agents/specs/goose-migration/security-permissions/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/security/patterns.rs, crates/agent/, crates/settings/, crates/sandbox/_
  - _Writes: selected existing agent/security-policy owner_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Implement adversary inspector
  - Detect prompt injection, jailbreak attempts, indirect injection
  - Configurable sensitivity levels
  - Use pattern registry for detection

  - _Requirements: 1.1, 1.2, 1.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/security-permissions/requirements.md, .agents/specs/goose-migration/security-permissions/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/security/adversary_inspector.rs, crates/agent/, crates/sandbox/_
  - _Writes: selected existing agent input/security-policy owner_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Implement egress inspector
  - Detect API keys, secrets, PII in outgoing content
  - Configurable redaction strategies (block vs. redact)

  - _Requirements: 2.1, 2.2, 2.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/security-permissions/requirements.md, .agents/specs/goose-migration/security-permissions/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/security/egress_inspector.rs, crates/agent/, crates/sandbox/, existing secret-redaction owner_
  - _Writes: selected existing egress/security-policy owner_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Implement classification client
  - Client for content moderation/safety APIs
  - Configurable thresholds and actions
  - Error handling for API unavailability

  - _Requirements: 3.1, 3.2, 3.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/security-permissions/requirements.md, .agents/specs/goose-migration/security-permissions/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/security/classification_client.rs, crates/http_client/, crates/credentials_provider/_
  - _Writes: selected existing security-policy and HTTP integration owner_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Implement security scanner
  - Unified scanner that orchestrates all inspectors
  - Aggregated result reporting
  - Configurable fail-open vs. fail-closed mode

  - _Requirements: 4.1, 4.2, 4.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/security-permissions/requirements.md, .agents/specs/goose-migration/security-permissions/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/security/scanner.rs, projects/goose/crates/goose/src/security/security_inspector.rs, crates/agent/, crates/sandbox/_
  - _Writes: selected existing agent/security-policy owner_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Extend the existing permission store and pattern owner
  - Persist normalized tool/argument patterns, decision, scope, and expiration through the existing permission owner
  - Minimize/redact readable context, use private permissions and atomic writes, recover visibly from corruption, and support clearing

  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/security-permissions/requirements.md, .agents/specs/goose-migration/security-permissions/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/permission/permission_store.rs, crates/agent/src/tool_permissions.rs, crates/acp_thread/src/connection.rs_
  - _Writes: crates/agent/src/tool_permissions.rs, selected existing permission persistence owner_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 7. Reconcile deterministic permission inspection and optional read-only judgment
  - Examine tool calls against stored decisions
  - Apply annotations, normalized patterns, session mode, and stored decisions deterministically first
  - If separately approved, send minimal labeled untrusted request data to a model and accept only validated submitted IDs classified strictly read-only
  - On error, ambiguity, unknown ID, timeout, or cancellation, approve nothing through the judge and return to normal confirmation/denial policy

  - _Requirements: 7.1, 7.2, 7.3, 8.1, 8.2, 8.3, 8.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/security-permissions/requirements.md, .agents/specs/goose-migration/security-permissions/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/permission/permission_inspector.rs, projects/goose/crates/goose/src/permission/permission_judge.rs, crates/agent/src/tool_permissions.rs, crates/acp_thread/src/connection.rs_
  - _Writes: crates/agent/src/tool_permissions.rs, crates/acp_thread/src/connection.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 8. Implement permission confirmation UI (GPUI)
  - Confirmation dialog showing tool name, arguments, risk level
  - Allow/Deny/Always-Allow/Always-Deny actions

  - _Requirements: 6.1, 6.2, 6.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/security-permissions/requirements.md, .agents/specs/goose-migration/security-permissions/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/permission/permission_confirmation.rs, crates/acp_thread/src/connection.rs, crates/agent_ui/src/conversation_view/_
  - _Writes: crates/acp_thread/src/connection.rs, crates/agent_ui/src/conversation_view/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 9. Integrate security scanner and permission system into agent
  - Hook security scanner into agent input/output processing
  - Hook permission inspector into tool execution pipeline (alongside existing tool_permissions.rs)

  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 4.1, 4.2, 4.3, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3, 8.4, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/security-permissions/requirements.md, .agents/specs/goose-migration/security-permissions/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/security_integration.rs, crates/agent/src/permission_integration.rs_
  - _Writes: crates/agent/src/security_integration.rs, crates/agent/src/permission_integration.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 10. Write tests
  - Pattern matching accuracy tests
  - Inspector tests with known-good and known-bad inputs
  - Permission store persistence and expiry tests
  - Integration tests with mock agent

  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 4.1, 4.2, 4.3, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3, 8.4, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/security-permissions/requirements.md, .agents/specs/goose-migration/security-permissions/design.md, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: none_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

## Notes

- The security systems are optional — disabled by default, enabled via configuration
- Permission system extends (does not replace) existing `tool_permissions.rs`
