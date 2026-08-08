# Design Document: Developer Experience

## 1. Overview

Migrate goose's developer experience features: slash commands, hints system, sim apps (embedded mini-apps), source roots/sources management, and the execution manager. These enhance day-to-day interaction with the agent.

### Key Architectural Decisions

- **Slash commands in agent input processing**: Rather than a separate system, slash commands are parsed at the agent input boundary and routed to handlers — similar to how `crates/agent/` already processes messages.
- **Hints as skill-like files**: Hints are essentially auto-loaded skills without explicit user invocation. Use the existing `crates/agent_skills/` discovery mechanism.
- **One MCP Apps path**: If embedded MCP Apps are approved, extend the existing context-server and agent UI boundary. Do not introduce a separate `sim_apps` registry or duplicate chat/clock applications.
- **Execution lifecycle in existing agent state**: Goose's manager coalesces agent initialization and restores evicted sessions; extend `agent::Agent`/`ThreadStore`, not a generic task manager or new crate.

## 2. Architecture

```mermaid
graph TD
    subgraph "Slash Commands"
        Input[Agent Input]
        Parser[SlashParser]
        Router[SlashRouter]
        RecipeCmd[RecipeSlashCommand]
        SkillCmd[SkillSlashCommand]
        HelpCmd[HelpSlashCommand]
    end

    subgraph "Hints System"
        HintLoader[HintLoader]
        HintFiles[.simhints files]
                ProjectHint[Project Hints]
                GlobalHint[Global Hints]
    end

    subgraph "MCP Apps (conditional)"
        ContextServer[Existing context server]
        AppRenderer[Isolated app renderer]
        Bridge[Restricted MCP bridge]
        Cache[Scoped resource cache]
    end

    subgraph "Infrastructure"
        Sources[SourceRoots / Sources]
        ExecMgr[ExecutionManager]
    end

    Input --> Parser
    Parser --> Router
    Router --> RecipeCmd
    Router --> SkillCmd
    Router --> HelpCmd

    HintLoader --> HintFiles
    HintLoader --> ProjectHint
    HintLoader --> GlobalHint
    HintLoader -->|feeds| Agent[Agent Context]

    ContextServer --> AppRenderer
    AppRenderer --> Bridge
    ContextServer --> Cache

    Sources -->|resolves paths| Agent
    ExecMgr -->|tracks tasks| Agent
```

## 3. Components and Interfaces

### Component: Slash Command System

```rust
pub struct SlashCommandParser;

impl SlashCommandParser {
    pub fn parse(input: &str) -> Option<ParsedSlashCommand>;
}

pub struct SlashCommandRouter {
    commands: HashMap<String, Box<dyn SlashCommandHandler>>,
}

#[async_trait]
pub trait SlashCommandHandler: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, args: &str, cx: &mut App) -> Result<SlashCommandResult>;
}

pub struct SlashCommandResult {
    pub response: String,
    pub insert_into_conversation: bool,
}
```

### Component: Hints System

```rust
pub struct HintLoader {
    global_hints_path: PathBuf,
    project_hints_path: Option<PathBuf>,
}

impl HintLoader {
    pub fn load_global_hints(&self) -> Result<Vec<Hint>>;
    pub fn load_project_hints(&self, project_root: &Path) -> Result<Vec<Hint>>;
    pub fn load_all(&self) -> Result<Vec<Hint>>;
}

pub struct Hint {
    pub source: HintSource,
    pub content: String,
    pub priority: u8,
}
```

### Component: MCP Apps Boundary (conditional)

`context_server` remains the resource owner. A single agent-UI renderer resolves resources by server and session, applies CSP/origin/navigation policy, and uses a capability-scoped bridge for allowed MCP operations. Resource cache keys include server identity and version, and are retired when the server or session closes.

Native GPUI views are not treated as MCP Apps and need no HTML compatibility layer.

### Component: Source Roots

```rust
pub struct SourceRoots {
    roots: Vec<SourceRoot>,
}

pub struct SourceRoot {
    pub name: String,
    pub path: PathBuf,
    pub priority: u8,
}

impl SourceRoots {
    pub fn resolve(&self, relative: &str) -> Option<PathBuf>;
    pub fn add_root(&mut self, name: &str, path: &Path);
}
```

### Component: Execution Manager

```rust
pub struct ExecutionManager {
    running_tasks: HashMap<TaskId, TrackedTask>,
}

impl ExecutionManager {
    pub fn spawn_task(&mut self, task: Task<Result<()>>, metadata: TaskMetadata) -> TaskId;
    pub fn cancel(&mut self, id: TaskId) -> Result<()>;
    pub fn status(&self, id: TaskId) -> Option<TaskStatus>;
    pub fn list_active(&self) -> Vec<TaskInfo>;
}
```

## 4. Data Models

```rust
pub enum HintSource {
    File(PathBuf),
    Inline { name: String },
    Project { path: PathBuf },
}

pub struct TaskMetadata {
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

pub enum TaskStatus {
    Running,
    Completed(Result<()>),
    Cancelled,
}

pub struct TaskInfo {
    pub id: TaskId,
    pub metadata: TaskMetadata,
    pub status: TaskStatus,
    pub duration: Option<Duration>,
}
```

## 5. Correctness Properties

### Property 1: Slash Command Uniqueness

_For any_ registered slash command, [its name], THE system SHALL have at most one handler registered.

**Validates: Requirement 1.4**

### Property 2: Hint Scoping

_For any_ project directory, [when hints are loaded], THE system SHALL load global hints merged with project-specific hints, with project hints taking priority.

**Validates: Requirement 2.2**

### Property 3: Task Tracking

_For any_ task registered with the execution manager, [after it completes], THE manager SHALL record the final status for at least 5 seconds before cleanup.

**Validates: Requirement 5.3**

## 6. Error Handling

| Error Scenario | Handling |
|---|---|
| Unknown slash command | Show available commands with descriptions |
| Hint file unreadable | Skip file with warning, continue loading others |
| Source root path doesn't exist | Log warning, return None on resolve |
| Task panics during execution | Catch panic, mark as failed with error |

## 7. Testing Strategy

- **Unit tests**: Slash command parsing and routing
- **Hint tests**: Loading from various sources, priority merging
- **App tests**: Render and action handling
- **Execution manager tests**: Spawn, cancel, status, completion

## References

- Source: `projects/goose/crates/goose/src/slash_commands/`
- Source: `projects/goose/crates/goose/src/hints/`
- Source: `projects/goose/crates/goose/src/goose_apps/`
- Source: `projects/goose/crates/goose/src/source_roots.rs`, `sources.rs`
- Source: `projects/goose/crates/goose/src/execution/`

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | Slash Command System audit design | Observable scenario and failure-path test for 1.1 |
| 1.2 | Slash Command System audit design | Observable scenario and failure-path test for 1.2 |
| 1.3 | Slash Command System audit design | Observable scenario and failure-path test for 1.3 |
| 1.4 | Slash Command System audit design | Observable scenario and failure-path test for 1.4 |
| 1.5 | Slash Command System audit design | Observable scenario and failure-path test for 1.5 |
| 2.1 | Hints System audit design | Observable scenario and failure-path test for 2.1 |
| 2.2 | Hints System audit design | Observable scenario and failure-path test for 2.2 |
| 2.3 | Hints System audit design | Observable scenario and failure-path test for 2.3 |
| 2.4 | Hints System audit design | Observable scenario and failure-path test for 2.4 |
| 3.1 | Sim Apps audit design | Observable scenario and failure-path test for 3.1 |
| 3.2 | Sim Apps audit design | Observable scenario and failure-path test for 3.2 |
| 3.3 | Sim Apps audit design | Observable scenario and failure-path test for 3.3 |
| 3.4 | Sim Apps audit design | Observable scenario and failure-path test for 3.4 |
| 3.5 | Sim Apps audit design | Observable scenario and failure-path test for 3.5 |
| 4.1 | Source Roots and Sources audit design | Observable scenario and failure-path test for 4.1 |
| 4.2 | Source Roots and Sources audit design | Observable scenario and failure-path test for 4.2 |
| 4.3 | Source Roots and Sources audit design | Observable scenario and failure-path test for 4.3 |
| 5.1 | Execution Manager audit design | Observable scenario and failure-path test for 5.1 |
| 5.2 | Execution Manager audit design | Observable scenario and failure-path test for 5.2 |
| 5.3 | Execution Manager audit design | Observable scenario and failure-path test for 5.3 |
| 5.4 | Execution Manager audit design | Observable scenario and failure-path test for 5.4 |
