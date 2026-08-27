# Implementation Plan: Desktop UI (GPUI Equivalents)

> Cross-cutting contract: every production write in this plan inherits the [`agentic` feature boundary](../feature-boundary.md). Completion evidence must classify actual writes and include the required enabled/disabled validation.

## Overview

Build GPUI-native views for recipe browsing, scheduling, diagnostics, shared sessions, and ACP connection status within zed's existing desktop UI architecture. No Electron/React code is ported — all UI is native GPUI.

## Repo Reconciliation

- Agent connection state already exists through `crates/agent_ui/src/agent_connection_store.rs` and is consumed by agent UI configuration/panel code.
- Diagnostics collection for agent context already exists in `crates/agent_ui/src/diagnostics.rs`; this task should add Goose-style doctor/system diagnostics rather than rebuild editor diagnostics.
- Auto-update notification UI already exists in `crates/auto_update_ui/src/auto_update_ui.rs`.

## Tasks

- [ ] 1. Extend existing agent connection status UI for ACP details
  - Audit existing `AgentConnectionStatus` usage
  - Add Goose-specific ACP details on hover/click
  - Add manual reconnect only if not already covered by existing controls

  - _Requirements: 6.1, 6.2, 6.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/desktop-ui/requirements.md, .agents/specs/goose-migration/desktop-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent_ui/src/agent_connection_store.rs, crates/agent_ui/src/agent_panel.rs_
  - _Writes: crates/agent_ui/src/agent_connection_store.rs, crates/agent_ui/src/agent_panel.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Implement recipe browser panel
  - Recipe search bar with filtering
  - Scrollable recipe list with cards (name, description, tags)
  - Recipe detail view (steps, variables, metadata)
  - "Run" button with confirmation
  - Integration with `crates/recipe/` engine

  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/desktop-ui/requirements.md, .agents/specs/goose-migration/desktop-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent_ui/src/recipe_browser.rs_
  - _Writes: crates/agent_ui/src/recipe_browser.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Implement scheduling settings view
  - List existing schedules with enable/disable toggle
  - Create schedule form (name, cron expression, task selection)
  - Delete schedule with confirmation
  - Use the recipe/session scheduled-job service; reuse `crates/scheduler/` only for executor primitives selected by that service

  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/desktop-ui/requirements.md, .agents/specs/goose-migration/desktop-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/settings_ui/src/scheduling_settings.rs_
  - _Writes: crates/settings_ui/src/scheduling_settings.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Extend diagnostics UI with Goose doctor results
  - Reuse existing diagnostics collection where applicable
  - Run health checks with visual status (pass/warning/fail)
  - Expandable detail per check with remediation steps
  - Auto-run on first open; manual re-run button
  - Integration with existing diagnostics collection and provider-registry health checks

  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/desktop-ui/requirements.md, .agents/specs/goose-migration/desktop-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent_ui/src/diagnostics.rs, crates/agent_ui/src/agent_panel.rs_
  - _Writes: crates/agent_ui/src/diagnostics.rs, crates/agent_ui/src/agent_panel.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Implement shared session support
  - Export session as shareable data (serialized JSON)
  - Import session from deeplink or file
  - Deeplink handler integration with `parse_zed_link`

  - _Requirements: 5.1, 5.2, 5.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/desktop-ui/requirements.md, .agents/specs/goose-migration/desktop-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/shared_session.rs, crates/zed/src/zed/shared_session_handler.rs_
  - _Writes: crates/agent/src/shared_session.rs, crates/zed/src/zed/shared_session_handler.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Integrate desktop UI features into the agent panel
  - Add recipe browser button/tab to agent panel header
  - Add connection status indicator to agent panel header
  - Add diagnostics to settings or help menu
  - Ensure all new components follow existing agent_ui patterns

  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 4.1, 4.2, 4.3, 4.4, 6.1, 6.2, 6.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/desktop-ui/requirements.md, .agents/specs/goose-migration/desktop-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent_ui/src/agent_panel.rs (modifications)_
  - _Writes: crates/agent_ui/src/agent_panel.rs (modifications)_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 7. Resolve and, if approved, extend i18n support
  - Evaluate zed's existing i18n state
  - If absent, record the product and architecture decision rather than introducing a new subsystem in this migration
  - Mark user-facing strings for translation

  - _Requirements: 9.1, 9.2, 9.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/desktop-ui/requirements.md, .agents/specs/goose-migration/desktop-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, existing localization/settings owner_
  - _Writes: selected existing localization/settings owner_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 8. Reconcile and enhance existing auto-update UI
  - Audit existing update notifications and release notes handling
  - Add only missing Goose update states or progress details
  - Keep using existing `crates/auto_update/`

  - _Requirements: 10.1, 10.2, 10.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/desktop-ui/requirements.md, .agents/specs/goose-migration/desktop-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/auto_update_ui/src/auto_update_ui.rs_
  - _Writes: crates/auto_update_ui/src/auto_update_ui.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 9. Write tests
  - Visual tests for each new GPUI component
  - Component state transition tests
  - Integration tests with mock backends
  - Accessibility tests (keyboard navigation, screen reader)

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3, 9.1, 9.2, 9.3, 10.1, 10.2, 10.3, 11.1, 11.2, 11.3, 11.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/desktop-ui/requirements.md, .agents/specs/goose-migration/desktop-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: none_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 10. Define and, if approved, implement the shared MCP App renderer security boundary
  - Reuse the context-server resource owner and the developer-experience MCP Apps boundary
  - Enforce CSP, origin, navigation, download, clipboard, resource-size, cache-lifetime, permission, and audit policies
  - Fail closed on invalid content, unsafe URLs, renderer crashes, server disconnects, and expired sessions without breaking the conversation

  - _Requirements: 11.1, 11.2, 11.3, 11.4_
  - _Depends on: developer-experience/3, security-permissions/6, security-permissions/7, security-permissions/9_
  - _Reads: .agents/specs/goose-migration/desktop-ui/requirements.md, .agents/specs/goose-migration/desktop-ui/design.md, projects/goose/ui/desktop/src/components/McpApps/, projects/goose/ui/desktop/src/utils/csp.ts, projects/goose/ui/desktop/src/utils/htmlSecurity.ts, projects/goose/ui/desktop/src/utils/urlSecurity.ts, crates/context_server/, crates/agent_ui/src/conversation_view/_
  - _Writes: crates/context_server/, crates/agent_ui/src/conversation_view/_
  - _Validation: Run CSP/origin/navigation/download/clipboard, cache isolation/retirement, tool permission/audit, unsafe URL, malformed HTML, renderer crash, disconnect, and expired-session tests_

## Notes

- All new components use GPUI rendering, not HTML/React
- Follow existing patterns in `crates/agent_ui/` for consistency
- Components are added to the agent panel or settings panels, not as standalone windows
