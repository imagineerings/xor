# Requirements: Desktop UI (GPUI Equivalents)

## Introduction

Migrate the user-facing features of goose's Electron desktop app by building GPUI-based equivalents within sim's existing desktop application. Rather than porting React/Electron code, we map each goose UI feature to the appropriate GPUI component or build new GPUI views where gaps exist.

Sim already provides: GPUI rendering framework, workspace/panel system, agent panel, conversation view, inline assistant, configuration UI, theme system, notifications, settings UI, and onboarding. Goose's desktop app adds: recipe execution UI, scheduling UI, startup diagnostics, shared sessions, mesh networking, dedicated session manager UI, and ACP client connection management.

## Glossary

- **GPUI**: Sim's native Rust UI framework
- **Agent Panel**: Existing GPUI panel for interacting with the agent (`crates/agent_ui/src/agent_panel.rs`)
- **Conversation View**: Existing GPUI conversation view (`crates/agent_ui/src/conversation_view.rs`)
- **Workspace**: Sim's window/panel management system (`crates/workspace/`)
- **ACP Client**: Agent-Client Protocol connection integrated in `crates/acp_thread/`
- **Mesh**: Peer-to-peer networking (if applicable to sim's architecture)

## Requirements

### Requirement 1: Agent Session Management UI

**User Story:** As a sim user, I want to manage agent sessions (list, create, delete, rename) through the GPUI interface, so that I can organize my agent conversations.

#### Acceptance Criteria

1. THE agent panel SHALL display a list of available agent sessions
2. THE user SHALL be able to create a new agent session
3. THE user SHALL be able to delete an existing session
4. THE user SHALL be able to rename a session
5. THE session list SHALL persist across application restarts

**Mapping:** Enhance existing `agent_ui` session management; goose session manager (`sessions.ts`) logic informs the GPUI `ThreadStore` integration.

### Requirement 2: Recipe Execution UI

**User Story:** As a sim user, I want to discover, view, and run recipes from the GPUI interface, so that I can execute multi-step workflows without the CLI.

#### Acceptance Criteria

1. THE agent panel SHALL provide a recipe browser showing available recipes
2. THE recipe browser SHALL support searching and filtering recipes
3. THE user SHALL be able to view recipe details before running
4. THE user SHALL be able to execute a recipe from the UI
5. THE recipe execution progress SHALL be displayed in the conversation view

**Mapping:** New GPUI component in `agent_ui` or as a standalone panel, drawing from goose's `recipe/` UI code but rendered in GPUI.

### Requirement 3: Scheduling UI

**User Story:** As a sim user, I want to schedule agent tasks from the GPUI interface, so that I can set up recurring operations.

#### Acceptance Criteria

1. THE settings SHALL include a scheduling section
2. THE user SHALL be able to view existing schedules
3. THE user SHALL be able to create, pause, and delete schedules
4. THE scheduling UI SHALL integrate with the existing `crates/scheduler/`

**Mapping:** New GPUI settings view or panel, integrating with sim's existing `crates/scheduler/` crate.

### Requirement 4: Startup Diagnostics

**User Story:** As a sim user, I want the application to check for configuration and connectivity issues on startup, so that I can resolve problems early.

#### Acceptance Criteria

1. ON startup THE system SHALL run diagnostics (provider connectivity, extension validity, system requirements)
2. IF issues are found THEN the system SHALL display them in a diagnostics view
3. THE diagnostics SHALL provide actionable remediation steps
4. THE diagnostics SHALL be logged for troubleshooting

**Mapping:** New GPUI diagnostics view, leveraging existing `crates/agent/src/agent.rs` health checks and the `doctor` functionality from goose.

### Requirement 5: Shared Sessions

**User Story:** As a sim user, I want to share agent sessions with other users via links, so that I can collaborate on agent interactions.

#### Acceptance Criteria

1. THE user SHALL be able to export a session as a shareable link
2. THE user SHALL be able to import a shared session from a link
3. THE shared session data SHALL be serializable and deserializable

**Mapping:** GPUI integration using sim's existing deeplink mechanism (`parse_sim_link`) and session serialization.

### Requirement 6: ACP Connection Management

**User Story:** As a sim user, I want visibility into the ACP connection state, so that I know when the agent is connected, disconnected, or reconnecting.

#### Acceptance Criteria

1. THE agent panel SHALL display the current ACP connection status
2. THE connection status SHALL update in real-time
3. THE user SHALL be able to manually reconnect if disconnected

**Mapping:** Extend existing `agent_connection_store.rs` in `agent_ui` with connection status indicators derived from `crates/acp_thread/`.

### Requirement 7: Agent Configuration UI Enhancements

**User Story:** As a sim user, I want to configure provider extensions and agent settings through the GPUI settings UI, so that I can manage all configuration visually.

#### Acceptance Criteria

1. THE settings UI SHALL allow configuring MCP/context server extensions
2. THE settings UI SHALL allow configuring agent modes and behavior
3. THE settings UI SHALL integrate with the existing `crates/agent_settings/` and `crates/settings_ui/`

**Mapping:** Extend existing `crates/settings_ui/` and `crates/agent_ui/src/agent_configuration.rs` to cover goose's extension/settings patterns.

### Requirement 8: Notification Enhancements

**User Story:** As a sim user, I want agent-related notifications (task complete, action required, errors) via the GPUI notification system, so that I stay informed.

#### Acceptance Criteria

1. THE agent SHALL use sim's existing notification system (`crates/notifications/`) for agent events
2. WHEN a scheduled task completes THEN the system SHALL show a notification
3. WHEN an action requires user input THEN the system SHALL show a notification

**Mapping:** Integrate agent events with sim's `Toast` and `NotificationId` system from `crates/workspace/`.

### Requirement 9: Internationalization (i18n)

**User Story:** As a sim user, I want the UI in my language, so that I can use the application comfortably.

#### Acceptance Criteria

1. THE GPUI application SHALL support i18n following sim's approach (if any exists) or introduce a lightweight i18n system
2. THE language SHALL be configurable in settings
3. THE i18n system SHALL support dynamic loading of translation files

**Mapping:** Evaluate if i18n is already partially present in GPUI; if not, introduce based on goose's i18n patterns adapted to Rust/GPUI.

### Requirement 10: Auto-Update UI

**User Story:** As a sim user, I want to be notified of and manage application updates from the UI, so that I stay up to date.

#### Acceptance Criteria

1. THE system SHALL check for updates on startup using existing `crates/auto_update/`
2. WHEN an update is available THEN the UI SHALL show an update notification
3. THE user SHALL be able to view update details and trigger installation

**Mapping:** Leverage existing `crates/auto_update/` and `crates/auto_update_ui/`, exposing update status in the GPUI interface.

## Design Approach

All goose desktop UI features will be implemented as **GPUI components/views** within sim's existing architecture:

```
goose Electron/React Feature          →  sim GPUI Equivalent
──────────────────────────────────────────────────────────────
sessions.ts / session management      →  ThreadStore + agent_ui session list
recipe/ UI components                 →  New agent_ui recipe panel
schedule.ts                           →  Settings view + crate/scheduler integration
startupDiagnostics.ts                 →  New diagnostics view in agent_ui
sharedSessions.ts / sessionLinks.ts   →  Deeplink + serialization in agent
updates.ts                            →  auto_update_ui integration
config.ts / goosed.ts                 →  Existing settings_ui + agent_configuration
i18n/                                 →  New GPUI i18n crate or pattern
mesh.ts                               →  Evaluate; may not be needed in sim
```

## References

- Source (goose): `projects/goose/ui/desktop/` — Full Electron/React app
- Existing sim: `crates/sim/` — main desktop binary
- Existing sim: `crates/agent_ui/` — agent panel, conversation, configuration
- Existing sim: `crates/gpui/` — GPUI framework
- Existing sim: `crates/ui/` — UI components
- Existing sim: `crates/settings_ui/` — settings UI
- Existing sim: `crates/workspace/` — workspace/panel management
- Existing sim: `crates/notifications/` — notification system
- Existing sim: `crates/auto_update/`, `crates/auto_update_ui/` — update system
- Existing sim: `crates/onboarding/` — onboarding views
- Existing sim: `crates/theme/` — theming system
