# Design Document: Desktop UI (GPUI Equivalents)

## 1. Overview

Migrate goose's Electron/React desktop app features by building GPUI-based equivalents within sim's existing desktop application. Sim already provides a full GPUI desktop (`crates/sim/`), agent panel (`crates/agent_ui/`), settings UI (`crates/settings_ui/`), workspace system (`crates/workspace/`), notifications (`crates/notifications/`), updates (`crates/auto_update_ui/`), and onboarding (`crates/onboarding/`). Goose adds recipe execution UI, scheduling UI, startup diagnostics, shared sessions, ACP connection management, and enhanced agent settings.

### Key Architectural Decisions

- **No Electron/React port**: All UI is built as native GPUI views. Goose's React components inform the design but are not directly ported.
- **Integrate into existing agent panel**: Recipe browser, diagnostics, and connection status are added as sub-views within or alongside the existing `AgentPanel` in `crates/agent_ui/`.
- **Settings extensions go into `crates/settings_ui/`**: Scheduling and extension management UIs follow the existing settings panel patterns.
- **Shared sessions via deeplinks**: Use sim's existing `parse_sim_link` deeplink mechanism for sharing sessions.

## 2. Architecture

```mermaid
graph TD
    subgraph "Existing Sim Desktop (crates/sim/)"
        GPUI[GPUI Framework]
        Workspace[Workspace System]
        AgentPanel[AgentPanel - crates/agent_ui/]
        SettingsUI[SettingsUI - crates/settings_ui/]
        Notifications[Notifications]
        AutoUpdateUI[AutoUpdate UI]
        Onboarding[Onboarding Views]
    end

    subgraph "New GPUI Components"
        RecipeUI[Recipe Browser Panel]
        ScheduleUI[Scheduling Settings]
        DiagView[Diagnostics View]
        SharedSesh[Shared Session Panel]
        ConnStatus[ACP Connection Status]
        ExtSettings[Extension Settings]
    end

    subgraph "Backend Integration"
        Agent[crates/agent/]
        Service[crates/recipe/ Recipe Engine]
        Sched[crates/scheduler/]
        Doctor[Existing diagnostics + provider health]
        ACP[crates/acp_thread/]
    end

    RecipeUI --> Service
    RecipeUI --> AgentPanel
    ScheduleUI --> Sched
    ScheduleUI --> SettingsUI
    DiagView --> Doctor
    DiagView --> Agent
    SharedSesh --> Agent
    ConnStatus --> ACP
    ConnStatus --> AgentPanel
    ExtSettings --> Agent
    ExtSettings --> SettingsUI

    Workspace --> AgentPanel
    Workspace --> RecipeUI
    Notifications --> Agent
    AutoUpdateUI -->|update check sim| Agent
```

## 3. Components and Interfaces

### Component: Recipe Browser Panel

```rust
pub struct RecipeBrowser {
    recipes: Vec<RecipeManifest>,
    search_query: SharedString,
    selected_recipe: Option<usize>,
    is_running: bool,
}

impl Render for RecipeBrowser {
    // GPUI element tree:
    // - Search bar
    // - Scrollable recipe list with cards
    // - Detail view for selected recipe
    // - "Run" button
}

impl RecipeBrowser {
    pub fn new(cx: &mut Context<Self>) -> Self;
    pub fn load_recipes(&mut self, cx: &mut Context<Self>);
    pub fn run_selected_recipe(&mut self, cx: &mut Context<Self>);
    pub fn filter_recipes(&mut self, query: &str);
}
```

### Component: Scheduling Settings View

```rust
pub struct SchedulingSettings {
    schedules: Vec<Schedule>,
    new_schedule_form: Option<ScheduleForm>,
}

impl SchedulingSettings {
    pub fn create_schedule(&mut self, config: ScheduleConfig, cx: &mut Context<Self>);
    pub fn delete_schedule(&mut self, id: ScheduleId, cx: &mut Context<Self>);
    pub fn toggle_schedule(&mut self, id: ScheduleId, cx: &mut Context<Self>);
}

impl Render for SchedulingSettings {
    // Renders within settings_ui panel
    // - List of schedules with enable/disable toggle
    // - Add new schedule form (cron input, task selection)
    // - Delete button per schedule
}
```

### Component: Diagnostics View

```rust
pub struct DiagnosticsView {
    results: Vec<HealthCheckResult>,
    running: bool,
}

impl DiagnosticsView {
    pub fn run_checks(&mut self, cx: &mut Context<Self>);
    pub fn get_result(&self, name: &str) -> Option<&HealthCheckResult>;
    pub fn is_all_passing(&self) -> bool;
}

impl Render for DiagnosticsView {
    // - List of health checks with status icons (pass/warning/fail)
    // - Expandable detail per check
    // - Remediation steps for failed checks
    // - "Re-run" button
    // - Auto-runs on first open
}
```

### Component: ACP Connection Status

```rust
pub struct AcpConnectionIndicator {
    state: AcpConnectionState,
}

impl Render for AcpConnectionIndicator {
    // Small indicator shown in agent panel header:
    // - Green dot: connected
    // - Yellow dot: reconnecting
    // - Red dot: disconnected
    // - Click to show details/reconnect
}

pub enum AcpConnectionState {
    Connected { since: DateTime<Utc>, agent_name: String },
    Reconnecting { attempt: u32, max_attempts: u32 },
    Disconnected { error: Option<String> },
    Disabled,
}
```

### Component: Shared Session Integration

```rust
// Enhance existing session serialization in crates/agent/
impl Session {
    pub fn export_shareable(&self) -> Result<SharedSessionData>;
    pub fn import_shareable(data: &SharedSessionData) -> Result<Self>;
}

pub struct SharedSessionData {
    pub version: u32,
    pub session: Session,
    pub metadata: ShareMetadata,
}

// Deeplink handler in crates/sim/
fn handle_shared_session_link(url: &Url, cx: &mut App) {
    // Parses sim://session/<encoded-data>
    // Imports session
    // Opens in agent panel
}
```

### Component: Enhanced Extension Settings

```rust
// Extend existing crates/agent_ui/src/agent_configuration.rs
impl AgentConfiguration {
    // Add extension management:
    // - List installed extensions
    // - Add extension (path picker / URL input)
    // - Remove extension
    // - Toggle enable/disable
    // - View extension details (tools, resources, prompts)
}
```

## 4. Data Models

```rust
pub struct ScheduleConfig {
    pub name: String,
    pub cron_expression: String,
    pub task_type: ScheduledTask,
    pub enabled: bool,
}

pub enum ScheduledTask {
    RunRecipe { recipe_name: String, variables: HashMap<String, String> },
    SendMessage { prompt: String },
    RunDiagnostics,
}
```

## 5. Correctness Properties

### Property 1: Recipe UI Consistency

_For any_ recipe [visible in the recipe browser], THE displayed information (name, description, version, variables) SHALL match the recipe manifest.

**Validates: Requirements 2.2, 2.3**

### Property 2: Connection Status Accuracy

_For any_ ACP connection state change, THE connection indicator SHALL reflect the new state within 500ms.

**Validates: Requirement 6.2**

### Property 3: Schedule Reliability

_For any_ schedule [created via the scheduling UI], [at the configured cron time], THE task SHALL execute.

**Validates: Requirement 3.2**

## 6. Error Handling

| Error Scenario | Handling |
|---|---|
| Recipe load fails | Show error state in browser with retry |
| Schedule creation fails | Show validation error in form |
| Diagnostics check hangs | Timeout after 10s, mark as failed |
| Shared session data corrupted | Show import error with detail |
| Extension load fails | Show error status in extension list |

## 7. Testing Strategy

- **Unit tests**: Each GPUI component's state transitions and event handling
- **Visual tests**: Use sim's visual test framework for component screenshots
- **Integration tests**: Recipe browser with mock recipe engine
- **Accessibility tests**: Keyboard navigation, screen reader labels

## References

- Source: `projects/goose/ui/desktop/` — Electron/React app (design reference only)
- Sim: `crates/sim/` — main desktop binary
- Sim: `crates/agent_ui/` — agent panel, conversation, configuration
- Sim: `crates/settings_ui/` — settings panel
- Sim: `crates/workspace/` — workspace/panel management
- Sim: `crates/notifications/` — notification system
- Sim: `crates/auto_update/`, `crates/auto_update_ui/` — update system
- Sim: `crates/onboarding/` — onboarding

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | Agent Session Management UI audit design | Observable scenario and failure-path test for 1.1 |
| 1.2 | Agent Session Management UI audit design | Observable scenario and failure-path test for 1.2 |
| 1.3 | Agent Session Management UI audit design | Observable scenario and failure-path test for 1.3 |
| 1.4 | Agent Session Management UI audit design | Observable scenario and failure-path test for 1.4 |
| 1.5 | Agent Session Management UI audit design | Observable scenario and failure-path test for 1.5 |
| 2.1 | Recipe Execution UI audit design | Observable scenario and failure-path test for 2.1 |
| 2.2 | Recipe Execution UI audit design | Observable scenario and failure-path test for 2.2 |
| 2.3 | Recipe Execution UI audit design | Observable scenario and failure-path test for 2.3 |
| 2.4 | Recipe Execution UI audit design | Observable scenario and failure-path test for 2.4 |
| 2.5 | Recipe Execution UI audit design | Observable scenario and failure-path test for 2.5 |
| 3.1 | Scheduling UI audit design | Observable scenario and failure-path test for 3.1 |
| 3.2 | Scheduling UI audit design | Observable scenario and failure-path test for 3.2 |
| 3.3 | Scheduling UI audit design | Observable scenario and failure-path test for 3.3 |
| 3.4 | Scheduling UI audit design | Observable scenario and failure-path test for 3.4 |
| 4.1 | Startup Diagnostics audit design | Observable scenario and failure-path test for 4.1 |
| 4.2 | Startup Diagnostics audit design | Observable scenario and failure-path test for 4.2 |
| 4.3 | Startup Diagnostics audit design | Observable scenario and failure-path test for 4.3 |
| 4.4 | Startup Diagnostics audit design | Observable scenario and failure-path test for 4.4 |
| 5.1 | Shared Sessions audit design | Observable scenario and failure-path test for 5.1 |
| 5.2 | Shared Sessions audit design | Observable scenario and failure-path test for 5.2 |
| 5.3 | Shared Sessions audit design | Observable scenario and failure-path test for 5.3 |
| 6.1 | ACP Connection Management audit design | Observable scenario and failure-path test for 6.1 |
| 6.2 | ACP Connection Management audit design | Observable scenario and failure-path test for 6.2 |
| 6.3 | ACP Connection Management audit design | Observable scenario and failure-path test for 6.3 |
| 7.1 | Agent Configuration UI Enhancements audit design | Observable scenario and failure-path test for 7.1 |
| 7.2 | Agent Configuration UI Enhancements audit design | Observable scenario and failure-path test for 7.2 |
| 7.3 | Agent Configuration UI Enhancements audit design | Observable scenario and failure-path test for 7.3 |
| 8.1 | Notification Enhancements audit design | Observable scenario and failure-path test for 8.1 |
| 8.2 | Notification Enhancements audit design | Observable scenario and failure-path test for 8.2 |
| 8.3 | Notification Enhancements audit design | Observable scenario and failure-path test for 8.3 |
| 9.1 | Internationalization (i18n) audit design | Observable scenario and failure-path test for 9.1 |
| 9.2 | Internationalization (i18n) audit design | Observable scenario and failure-path test for 9.2 |
| 9.3 | Internationalization (i18n) audit design | Observable scenario and failure-path test for 9.3 |
| 10.1 | Auto-Update UI audit design | Observable scenario and failure-path test for 10.1 |
| 10.2 | Auto-Update UI audit design | Observable scenario and failure-path test for 10.2 |
| 10.3 | Auto-Update UI audit design | Observable scenario and failure-path test for 10.3 |
| 11.1, 11.2, 11.3, 11.4 | D-MCP-APP-SECURITY | CSP/origin/navigation/download/clipboard, resource scope, permission, crash, disconnect, and fail-closed tests |

## Audit design corrections

- **D-SCHEDULE-UI:** The settings view is an adapter over the recipe/session scheduling service. It never persists jobs through a GPUI-only or executor-only store.
- **D-MCP-APP-SECURITY:** If approved, reuse one isolated renderer and one context-server resource cache. Native GPUI views need no HTML port. The app bridge has no ambient filesystem, network, shell, clipboard, or cross-server access.
