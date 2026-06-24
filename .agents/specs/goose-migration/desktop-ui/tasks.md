# Implementation Plan: Desktop UI (GPUI Equivalents)

## Overview

Build GPUI-native views for recipe browsing, scheduling, diagnostics, shared sessions, and ACP connection status within baymax's existing desktop UI architecture. No Electron/React code is ported — all UI is native GPUI.

## Tasks

- [ ] 1. Implement ACP connection status indicator
  - GPUI component showing connection state (connected/reconnecting/disconnected)
  - Real-time state updates via subscription to acp_thread connection events
  - Click to show details or reconnect
  - _Requirements: 6_
  - _writes: crates/agent_ui/src/acp_connection_indicator.rs_

- [ ] 2. Implement recipe browser panel
  - Recipe search bar with filtering
  - Scrollable recipe list with cards (name, description, tags)
  - Recipe detail view (steps, variables, metadata)
  - "Run" button with confirmation
  - Integration with `crates/recipe/` engine
  - _Requirements: 2_
  - _writes: crates/agent_ui/src/recipe_browser.rs_

- [ ] 3. Implement scheduling settings view
  - List existing schedules with enable/disable toggle
  - Create schedule form (name, cron expression, task selection)
  - Delete schedule with confirmation
  - Integration with `crates/scheduler/`
  - _Requirements: 3_
  - _writes: crates/settings_ui/src/scheduling_settings.rs_

- [ ] 4. Implement diagnostics view
  - Run health checks with visual status (pass/warning/fail)
  - Expandable detail per check with remediation steps
  - Auto-run on first open; manual re-run button
  - Integration with `crates/doctor/`
  - _Requirements: 4_
  - _writes: crates/agent_ui/src/diagnostics_view.rs_

- [ ] 5. Implement shared session support
  - Export session as shareable data (serialized JSON)
  - Import session from deeplink or file
  - Deeplink handler integration with `parse_baymax_link`
  - _Requirements: 5_
  - _writes: crates/agent/src/shared_session.rs, crates/baymax/src/baymax/shared_session_handler.rs_

- [ ] 6. Integrate desktop UI features into the agent panel
  - Add recipe browser button/tab to agent panel header
  - Add connection status indicator to agent panel header
  - Add diagnostics to settings or help menu
  - Ensure all new components follow existing agent_ui patterns
  - _Requirements: 2, 4, 6_
  - _writes: crates/agent_ui/src/agent_panel.rs (modifications)_

- [ ] 7. Implement i18n support (if not already present)
  - Evaluate baymax's existing i18n state
  - If absent, introduce lightweight i18n system
  - Mark user-facing strings for translation
  - _Requirements: 9_
  - _writes: crates/i18n/src/lib.rs (if new)_

- [ ] 8. Enhance auto-update UI
  - Check for updates on startup (using existing `crates/auto_update/`)
  - Show update notification in UI
  - Allow triggering update install from UI
  - _Requirements: 10_
  - _writes: crates/auto_update_ui/src/update_notification.rs_

- [ ] 9. Write tests
  - Visual tests for each new GPUI component
  - Component state transition tests
  - Integration tests with mock backends
  - Accessibility tests (keyboard navigation, screen reader)
  - _Requirements: 1-10_

## Notes

- All new components use GPUI rendering, not HTML/React
- Follow existing patterns in `crates/agent_ui/` for consistency
- Components are added to the agent panel or settings panels, not as standalone windows
