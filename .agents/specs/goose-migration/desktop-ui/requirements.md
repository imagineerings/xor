# Requirements: Desktop UI (GPUI Equivalents)

## Introduction

Migrate the user-facing features of goose's Electron desktop app by building GPUI-based equivalents within zed's existing desktop application. Rather than porting React/Electron code, we map each goose UI feature to the appropriate GPUI component or build new GPUI views where gaps exist.

Zed already provides: GPUI rendering framework, workspace/panel system, agent panel, conversation view, inline assistant, configuration UI, theme system, notifications, settings UI, and onboarding. Goose's desktop app adds: recipe execution UI, scheduling UI, startup diagnostics, shared sessions, mesh networking, dedicated session manager UI, and ACP client connection management.

## Glossary

- **GPUI**: Zed's native Rust UI framework
- **Agent Panel**: Existing GPUI panel for interacting with the agent (`crates/agent_ui/src/agent_panel.rs`)
- **Conversation View**: Existing GPUI conversation view (`crates/agent_ui/src/conversation_view.rs`)
- **Workspace**: Zed's window/panel management system (`crates/workspace/`)
- **ACP Client**: Agent-Client Protocol connection integrated in `crates/acp_thread/`
- **Mesh**: Peer-to-peer networking (if applicable to zed's architecture)

## Requirements

### Requirement 1: Agent Session Management UI

**User Story:** As a zed user, I want to manage agent sessions (list, create, delete, rename) through the GPUI interface, so that I can organize my agent conversations.

#### Acceptance Criteria

1. **1.1** THE agent panel SHALL display a list of available agent sessions
2. **1.2** THE user SHALL be able to create a new agent session
3. **1.3** THE user SHALL be able to delete an existing session
4. **1.4** THE user SHALL be able to rename a session
5. **1.5** THE session list SHALL persist across application restarts

**Mapping:** Enhance existing `agent_ui` session management; goose session manager (`sessions.ts`) logic informs the GPUI `ThreadStore` integration.

### Requirement 2: Recipe Execution UI

**User Story:** As a zed user, I want to discover, view, and run recipes from the GPUI interface, so that I can execute multi-step workflows without the CLI.

#### Acceptance Criteria

1. **2.1** THE agent panel SHALL provide a recipe browser showing available recipes
2. **2.2** THE recipe browser SHALL support searching and filtering recipes
3. **2.3** THE user SHALL be able to view recipe details before running
4. **2.4** THE user SHALL be able to execute a recipe from the UI
5. **2.5** THE recipe execution progress SHALL be displayed in the conversation view

**Mapping:** New GPUI component in `agent_ui` or as a standalone panel, drawing from goose's `recipe/` UI code but rendered in GPUI.

### Requirement 3: Scheduling UI

**User Story:** As a zed user, I want to schedule agent tasks from the GPUI interface, so that I can set up recurring operations.

#### Acceptance Criteria

1. **3.1** THE settings SHALL include a scheduling section
2. **3.2** THE user SHALL be able to view existing schedules
3. **3.3** THE user SHALL be able to create, pause, and delete schedules
4. **3.4** THE scheduling UI SHALL call the single scheduled-recipe service defined by recipe-system Requirement 14; `crates/scheduler/` MAY supply executor primitives but SHALL NOT become a second persistence owner

**Mapping:** New GPUI settings view or panel, integrating with zed's existing `crates/scheduler/` crate.

### Requirement 4: Startup Diagnostics

**User Story:** As a zed user, I want the application to check for configuration and connectivity issues on startup, so that I can resolve problems early.

#### Acceptance Criteria

1. **4.1** ON startup THE system SHALL run diagnostics (provider connectivity, extension validity, system requirements)
2. **4.2** IF issues are found THEN the system SHALL display them in a diagnostics view
3. **4.3** THE diagnostics SHALL provide actionable remediation steps
4. **4.4** THE diagnostics SHALL be logged for troubleshooting

**Mapping:** New GPUI diagnostics view, leveraging existing `crates/agent/src/agent.rs` health checks and the `doctor` functionality from goose.

### Requirement 5: Shared Sessions

**User Story:** As a zed user, I want to share agent sessions with other users via links, so that I can collaborate on agent interactions.

#### Acceptance Criteria

1. **5.1** THE user SHALL be able to export a session as a shareable link
2. **5.2** THE user SHALL be able to import a shared session from a link
3. **5.3** THE shared session data SHALL be serializable and deserializable

**Mapping:** GPUI integration using zed's existing deeplink mechanism (`parse_zed_link`) and session serialization.

### Requirement 6: ACP Connection Management

**User Story:** As a zed user, I want visibility into the ACP connection state, so that I know when the agent is connected, disconnected, or reconnecting.

#### Acceptance Criteria

1. **6.1** THE agent panel SHALL display the current ACP connection status
2. **6.2** THE connection status SHALL update in real-time
3. **6.3** THE user SHALL be able to manually reconnect if disconnected

**Mapping:** Extend existing `agent_connection_store.rs` in `agent_ui` with connection status indicators derived from `crates/acp_thread/`.

### Requirement 7: Agent Configuration UI Enhancements

**User Story:** As a zed user, I want to configure provider extensions and agent settings through the GPUI settings UI, so that I can manage all configuration visually.

#### Acceptance Criteria

1. **7.1** THE settings UI SHALL allow configuring MCP/context server extensions
2. **7.2** THE settings UI SHALL allow configuring agent modes and behavior
3. **7.3** THE settings UI SHALL integrate with the existing `crates/agent_settings/` and `crates/settings_ui/`

**Mapping:** Extend existing `crates/settings_ui/` and `crates/agent_ui/src/agent_configuration.rs` to cover goose's extension/settings patterns.

### Requirement 8: Notification Enhancements

**User Story:** As a zed user, I want agent-related notifications (task complete, action required, errors) via the GPUI notification system, so that I stay informed.

#### Acceptance Criteria

1. **8.1** THE agent SHALL use zed's existing notification system (`crates/notifications/`) for agent events
2. **8.2** WHEN a scheduled task completes THEN the system SHALL show a notification
3. **8.3** WHEN an action requires user input THEN the system SHALL show a notification

**Mapping:** Integrate agent events with zed's `Toast` and `NotificationId` system from `crates/workspace/`.

### Requirement 9: Internationalization (i18n)

**User Story:** As a zed user, I want the UI in my language, so that I can use the application comfortably.

#### Acceptance Criteria

1. **9.1** WHERE Goose locale parity is approved, THE GPUI application SHALL extend Zed's existing localization approach; a new i18n subsystem SHALL require separate architecture approval
2. **9.2** THE language SHALL be configurable in settings
3. **9.3** THE i18n system SHALL support dynamic loading of translation files

**Mapping:** Evaluate if i18n is already partially present in GPUI; if not, introduce based on goose's i18n patterns adapted to Rust/GPUI.

### Requirement 10: Auto-Update UI

**User Story:** As a zed user, I want to be notified of and manage application updates from the UI, so that I stay up to date.

#### Acceptance Criteria

1. **10.1** THE system SHALL check for updates on startup using existing `crates/auto_update/`
2. **10.2** WHEN an update is available THEN the UI SHALL show an update notification
3. **10.3** THE user SHALL be able to view update details and trigger installation

**Mapping:** Leverage existing `crates/auto_update/` and `crates/auto_update_ui/`, exposing update status in the GPUI interface.

### Requirement 11: Embedded MCP App Security

**User Story:** As a zed user, I want embedded extension interfaces isolated from my application and machine, so that viewing an MCP App does not grant ambient privileges.

#### Acceptance Criteria

1. **11.1** WHERE MCP Apps are approved, THE desktop SHALL use the single context-server and renderer boundary defined by developer-experience Requirement 3
2. **11.2** THE renderer SHALL enforce CSP, opaque or allowlisted origins, navigation and download blocking, clipboard policy, resource size limits, and server/session-scoped caches
3. **11.3** MCP tool calls from the app SHALL be capability-scoped to the owning server and SHALL pass through the same permission confirmation and audit path as conversation tool calls
4. **11.4** INVALID HTML, unsafe URL schemes, blocked resources, crashed renderers, disconnected servers, and expired sessions SHALL fail closed with a visible diagnostic while leaving the conversation usable

## Design Approach

All goose desktop UI features will be implemented as **GPUI components/views** within zed's existing architecture:

```
goose Electron/React Feature          →  zed GPUI Equivalent
──────────────────────────────────────────────────────────────
sessions.ts / session management      →  ThreadStore + agent_ui session list
recipe/ UI components                 →  New agent_ui recipe panel
schedule.ts                           →  Settings view + crate/scheduler integration
startupDiagnostics.ts                 →  New diagnostics view in agent_ui
sharedSessions.ts / sessionLinks.ts   →  Deeplink + serialization in agent
updates.ts                            →  auto_update_ui integration
config.ts / goosed.ts                 →  Existing settings_ui + agent_configuration
i18n/                                 →  New GPUI i18n crate or pattern
mesh.ts                               →  Evaluate; may not be needed in zed
```

## References

- Source (goose): `projects/goose/ui/desktop/` — Full Electron/React app
- Existing zed: `crates/zed/` — main desktop binary
- Existing zed: `crates/agent_ui/` — agent panel, conversation, configuration
- Existing zed: `crates/gpui/` — GPUI framework
- Existing zed: `crates/ui/` — UI components
- Existing zed: `crates/settings_ui/` — settings UI
- Existing zed: `crates/workspace/` — workspace/panel management
- Existing zed: `crates/notifications/` — notification system
- Existing zed: `crates/auto_update/`, `crates/auto_update_ui/` — update system
- Existing zed: `crates/onboarding/` — onboarding views
- Existing zed: `crates/theme/` — theming system
- Source (goose): `projects/goose/ui/desktop/src/components/McpApps/`, `utils/csp.ts`, `utils/htmlSecurity.ts`, `utils/urlSecurity.ts`
