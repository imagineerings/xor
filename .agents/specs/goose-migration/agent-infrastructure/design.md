# Design Document: Agent Infrastructure

## 1. Overview

Migrate ~17 foundational agent infrastructure features from goose into zed's existing agent and supporting crates. These are varied subsystems that augment the core agent loop, configuration, and system management.

### Key Architectural Decisions

- **Distribute across existing crates**: Rather than one monolithic crate, each feature goes where it fits architecturally:
  - Context management → `crates/agent/`
  - Plugins → `crates/extension/` or new `crates/plugins/`
  - Hooks → `crates/agent/` (hook points in the agent loop)
  - Subagents → `crates/agent/` (enhance existing spawn_agent_tool)
  - Platform extensions → `crates/agent/` (tools registered with the agent)
  - Doctor → extend existing diagnostics and provider-registry health paths
  - Download behavior → reuse `http_client` and the selected provider/cache owner; extract a shared manager only if multiple existing consumers require it
  - Config migrations → `crates/settings/`
  - Zed mode → `crates/agent_settings/`
  - Misc (instance_id, subprocess, prompt_template, etc.) → `crates/util/` or small modules

## 2. Architecture

```mermaid
graph TD
    subgraph "crates/agent/"
        ContextMgmt[ContextManager]
        HookSystem[HookSystem]
        SubagentExec[SubagentExecution]
        PlatformExt[PlatformExtensions]
        LargeResp[LargeResponseHandler]
        FinalOutput[FinalOutputTool]
        Snapshot[SnapshotManager]
    end

    subgraph "New / Extended Crates"
        Plugins[crates/plugins/]
        Doctor[Existing diagnostics + provider health]
        DL[HTTP client + provider cache]
    end

    subgraph "crates/settings/ + crates/agent_settings/"
        ConfigMigrate[ConfigMigrations]
        SimMode[SimMode]
    end

    subgraph "crates/util/ + small modules"
        InstanceID[InstanceID]
        Subproc[SubprocessManager]
        PromptTmpl[PromptTemplate]
        ActionMgr[ActionRequiredManager]
        MalwareCheck[ExtensionMalwareCheck]
        BuiltinExt[BuiltinExtensions]
    end

    AgentCore[Agent Core] --> ContextMgmt
    AgentCore --> HookSystem
    AgentCore --> SubagentExec
    AgentCore --> PlatformExt
    AgentCore --> LargeResp
    AgentCore --> FinalOutput
    AgentCore --> Snapshot
    AgentCore --> Plugins
    AgentCore --> ActionMgr
    AgentCore --> BuiltinExt
    AgentCore --> MalwareCheck

    Doctor -->|checks| AgentCore
    Doctor -->|checks| Config[Configuration]
```

## 3. Components and Interfaces

### Component: Context Manager

```rust
pub struct ContextManager {
    max_tokens: usize,
    strategy: CompactionStrategy,
}

impl ContextManager {
    pub fn monitor_usage(&self, messages: &[Message]) -> ContextUsage;
    pub fn compact(&self, messages: &mut Vec<Message>) -> Result<CompactionResult>;
    pub fn should_compact(&self, usage: &ContextUsage) -> bool;
}

pub enum CompactionStrategy {
    Summarize { preserve_last_n: usize },
    Trim { max_messages: usize },
    DropLeastRelevant { threshold: f32 },
}
```

### Component: Hook System

```rust
pub struct HookSystem {
    hooks: HashMap<HookPoint, Vec<Box<dyn Hook>>>,
}

pub enum HookPoint {
    BeforeToolExecution { tool_name: String },
    AfterToolExecution { tool_name: String },
    BeforeLlmCall,
    AfterLlmCall,
    OnSessionStart,
    OnSessionEnd,
    OnError,
}

#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, context: HookContext) -> Result<HookAction>;
}

pub enum HookAction {
    Continue,
    Abort(String),
    Modify(HookContext),
}
```

### Component: Subagent Execution

```rust
pub struct SubagentConfig {
    pub model: Option<ModelId>,
    pub instructions: String,
    pub tools: Vec<String>,
    pub max_turns: usize,
    pub timeout: Duration,
}

pub struct SubagentHandle {
    pub task: Task<Result<SubagentResult>>,
    pub events: Receiver<SubagentEvent>,
}

pub enum SubagentEvent {
    TurnCompleted { turn: usize, summary: String },
    ToolCalled { tool: String, duration: Duration },
    Completed { result: String },
    Failed { error: String },
}
```

### Component: Doctor

```rust
pub struct Doctor {
    checks: Vec<Box<dyn HealthCheck>>,
}

#[async_trait]
pub trait HealthCheck: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn run(&self) -> HealthCheckResult;
}

pub struct HealthCheckResult {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    pub remediation: Option<String>,
}

pub enum HealthStatus {
    Pass,
    Warning(String),
    Fail(String),
}
```

### Component: Platform Extensions

```rust
// Each platform extension is registered as an agent tool
pub struct CodeExecutionTool;  // Run code in sandbox
impl AgentTool for CodeExecutionTool { ... }

pub struct OrchestratorTool;  // Coordinate multi-step workflows
impl AgentTool for OrchestratorTool { ... }

pub struct SummarizeTool;  // Summarize content
impl AgentTool for SummarizeTool { ... }

// ... similar for Todo, Tom, Apps, Chatrecall, Summon, Analyze, Developer
```

### Component: Configuration Migration

```rust
pub struct ConfigMigrator {
    current_version: u32,
    migrations: Vec<Box<dyn Migration>>,
}

#[async_trait]
pub trait Migration: Send + Sync {
    fn from_version(&self) -> u32;
    fn to_version(&self) -> u32;
    fn description(&self) -> &str;
    async fn migrate(&self, config: &mut Value) -> Result<()>;
    async fn rollback(&self, config: &mut Value) -> Result<()>;
}
```

### Component: Zed Mode

```rust
pub enum SimMode {
    Balanced,
    Focus,
    Creative,
    Custom { prompt_override: String, temperature: Option<f32> },
}

impl SimMode {
    pub fn system_prompt_modifier(&self) -> &str;
    pub fn temperature(&self) -> Option<f32>;
}
```

## 4. Data Models

```rust
pub struct CompactionResult {
    pub original_tokens: usize,
    pub compacted_tokens: usize,
    pub strategy_used: CompactionStrategy,
    pub messages_removed: usize,
}

pub struct SubagentResult {
    pub id: String,
    pub output: String,
    pub tool_calls_made: usize,
    pub total_duration: Duration,
    pub token_usage: TokenUsage,
}

pub struct Snapshot {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub conversation: Vec<Message>,
    pub tool_states: Value,
}
```

## 5. Correctness Properties

### Property 1: Context Bounds

_For any_ conversation [processed by the context manager], AFTER compaction, THE total token count SHALL be within the configured limit.

**Validates: Requirement 1.3**

### Property 2: Hook Execution Order

_For any_ registered hooks [at the same hook point], THE hooks SHALL execute in registration order.

**Validates: Requirement 3.3**

### Property 3: Subagent Isolation

_For any_ subagent execution, [when the parent agent errors or is cancelled], THE subagent SHALL also be cancelled.

**Validates: Requirement 4.5**

### Property 4: Migration Atomicity

_For any_ configuration migration [that fails], THE system SHALL fully roll back to the previous configuration version.

**Validates: Requirement 16.3**

### Property 5: Doctor Completeness

_For any_ doctor run, ALL registered checks SHALL execute and produce a result.

**Validates: Requirement 11.5**

## 6. Error Handling

| Error Scenario | Handling |
|---|---|
| Context compaction fails | Keep existing context, log error, continue |
| Hook panics | Catch, log, continue with remaining hooks |
| Subagent timeout | Kill subagent, return timeout error to parent |
| Migration incompatible | Halt startup, prompt user to restore backup |
| Doctor check hangs | Timeout after 10s, mark as failed |

## 7. Testing Strategy

- **Unit tests**: Each component independently (context manager, hooks, migrations)
- **Integration tests**: Subagent spawning, doctor running, migration applying
- **Edge cases**: Empty context, all hooks failing, version skip migrations

## Audit corrections

- `agents/snapshots/*.snap` are prompt golden files; the proposed runtime `SnapshotManager` is not Goose parity work and must not be implemented from this specification.
- Goose's hook behavior is plugin-owned command execution from `hooks/hooks.json`, with regex matching, JSON stdin, timeouts, plugin-root substitution, and pre-tool denial. A generic in-process trait-only hook design is insufficient.
- Large-response handling applies to oversized text tool results and preserves full content in a temporary file; it is not model-response truncation.
- Structured final output is recipe-scoped JSON Schema validation with corrective retry behavior.
- Plugins, Doctor, downloads, and container execution must first extend existing Zed agent-server, diagnostics, HTTP/cache, dev-container, and process-lifecycle integration points. The earlier diagrams' proposed crates are illustrative, not approved ownership.

### D-HOOKS

Integrate command hooks at existing agent lifecycle points. Parse enabled plugin manifests once per relevant project/session lifecycle, preserve declared order, enforce timeouts, and convert pre-tool denial into the existing tool error/permission path.

### D-LARGE-RESULT

Process tool results at the central ingestion boundary. Use a restricted temporary file for above-threshold text, surface write failures visibly, and define cleanup/retention before implementation.

### D-PROMPT-GOLDENS

Use prompt regression fixtures only as tests. Do not add runtime state capture/restore.

### D-CONTAINER-EXTENSIONS

Reuse Zed's development-container and agent-server process abstractions. The container option changes extension process placement, not the agent/session authority or permission model.

## References

- Source: All `projects/goose/crates/goose/src/` files listed in requirements
- Zed: `crates/agent/`, `crates/settings/`, `crates/agent_settings/`, `crates/util/`

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | Context Management audit design | Observable scenario and failure-path test for 1.1 |
| 1.2 | Context Management audit design | Observable scenario and failure-path test for 1.2 |
| 1.3 | Context Management audit design | Observable scenario and failure-path test for 1.3 |
| 1.4 | Context Management audit design | Observable scenario and failure-path test for 1.4 |
| 2.1 | Plugin System audit design | Observable scenario and failure-path test for 2.1 |
| 2.2 | Plugin System audit design | Observable scenario and failure-path test for 2.2 |
| 2.3 | Plugin System audit design | Observable scenario and failure-path test for 2.3 |
| 2.4 | Plugin System audit design | Observable scenario and failure-path test for 2.4 |
| 3.1 | Hook System audit design | Observable scenario and failure-path test for 3.1 |
| 3.2 | Hook System audit design | Observable scenario and failure-path test for 3.2 |
| 3.3 | Hook System audit design | Observable scenario and failure-path test for 3.3 |
| 3.4 | Hook System audit design | Observable scenario and failure-path test for 3.4 |
| 3.5, 3.6, 3.7, 3.8 | D-HOOKS | Manifest, event, matcher, stdin, timeout, ordering, and denial tests |
| 4.1 | Subagent Execution audit design | Observable scenario and failure-path test for 4.1 |
| 4.2 | Subagent Execution audit design | Observable scenario and failure-path test for 4.2 |
| 4.3 | Subagent Execution audit design | Observable scenario and failure-path test for 4.3 |
| 4.4 | Subagent Execution audit design | Observable scenario and failure-path test for 4.4 |
| 4.5 | Subagent Execution audit design | Observable scenario and failure-path test for 4.5 |
| 5.1 | Platform Extensions audit design | Observable scenario and failure-path test for 5.1 |
| 5.2 | Platform Extensions audit design | Observable scenario and failure-path test for 5.2 |
| 5.3 | Platform Extensions audit design | Observable scenario and failure-path test for 5.3 |
| 5.4 | Platform Extensions audit design | Observable scenario and failure-path test for 5.4 |
| 5.5 | Platform Extensions audit design | Observable scenario and failure-path test for 5.5 |
| 5.6 | Platform Extensions audit design | Observable scenario and failure-path test for 5.6 |
| 5.7 | Platform Extensions audit design | Observable scenario and failure-path test for 5.7 |
| 5.8 | Platform Extensions audit design | Observable scenario and failure-path test for 5.8 |
| 5.9 | Platform Extensions audit design | Observable scenario and failure-path test for 5.9 |
| 5.10 | Platform Extensions audit design | Observable scenario and failure-path test for 5.10 |
| 5.11 | Platform Extensions audit design | Observable scenario and failure-path test for 5.11 |
| 6.1 | Large Response Handler audit design | Observable scenario and failure-path test for 6.1 |
| 6.2 | Large Response Handler audit design | Observable scenario and failure-path test for 6.2 |
| 6.3 | Large Response Handler audit design | Observable scenario and failure-path test for 6.3 |
| 6.4 | D-LARGE-RESULT | Retention, cleanup, permissions, and disclosure test |
| 7.1 | Final Output Tool audit design | Observable scenario and failure-path test for 7.1 |
| 7.2 | Final Output Tool audit design | Observable scenario and failure-path test for 7.2 |
| 7.3 | D-FINAL-OUTPUT | Invalid-schema, invalid-payload, and retry-limit tests |
| 8.1, 8.2 | D-PROMPT-GOLDENS | Source audit and prompt-regression test review |
| 9.1 | Extension Malware Check audit design | Observable scenario and failure-path test for 9.1 |
| 9.2 | Extension Malware Check audit design | Observable scenario and failure-path test for 9.2 |
| 9.3 | Extension Malware Check audit design | Observable scenario and failure-path test for 9.3 |
| 10.1 | Action Required Manager audit design | Observable scenario and failure-path test for 10.1 |
| 10.2 | Action Required Manager audit design | Observable scenario and failure-path test for 10.2 |
| 10.3 | Action Required Manager audit design | Observable scenario and failure-path test for 10.3 |
| 11.1 | Doctor / Troubleshooting audit design | Observable scenario and failure-path test for 11.1 |
| 11.2 | Doctor / Troubleshooting audit design | Observable scenario and failure-path test for 11.2 |
| 11.3 | Doctor / Troubleshooting audit design | Observable scenario and failure-path test for 11.3 |
| 11.4 | Doctor / Troubleshooting audit design | Observable scenario and failure-path test for 11.4 |
| 11.5 | Doctor / Troubleshooting audit design | Observable scenario and failure-path test for 11.5 |
| 11.6, 11.7 | Doctor integration | Redaction/cap and provider-change policy tests |
| 12.1 | Download Manager audit design | Observable scenario and failure-path test for 12.1 |
| 12.2 | Download Manager audit design | Observable scenario and failure-path test for 12.2 |
| 12.3 | Download Manager audit design | Observable scenario and failure-path test for 12.3 |
| 12.4, 12.5, 12.6 | Download integration | Auth, sharding, cancellation, partial cleanup, integrity, disk, and permission tests |
| 13.1 | Instance ID audit design | Observable scenario and failure-path test for 13.1 |
| 13.2 | Instance ID audit design | Observable scenario and failure-path test for 13.2 |
| 13.3 | Instance ID audit design | Observable scenario and failure-path test for 13.3 |
| 14.1 | Prompt Templates audit design | Observable scenario and failure-path test for 14.1 |
| 14.2 | Prompt Templates audit design | Observable scenario and failure-path test for 14.2 |
| 14.3 | Prompt Templates audit design | Observable scenario and failure-path test for 14.3 |
| 15.1 | Subprocess Management audit design | Observable scenario and failure-path test for 15.1 |
| 15.2 | Subprocess Management audit design | Observable scenario and failure-path test for 15.2 |
| 15.3 | Subprocess Management audit design | Observable scenario and failure-path test for 15.3 |
| 16.1 | Configuration Migration audit design | Observable scenario and failure-path test for 16.1 |
| 16.2 | Configuration Migration audit design | Observable scenario and failure-path test for 16.2 |
| 16.3 | Configuration Migration audit design | Observable scenario and failure-path test for 16.3 |
| 17.1 | Goose Mode audit design | Observable scenario and failure-path test for 17.1 |
| 17.2 | Goose Mode audit design | Observable scenario and failure-path test for 17.2 |
| 17.3 | Goose Mode audit design | Observable scenario and failure-path test for 17.3 |
| 18.1, 18.2, 18.3, 18.4 | D-CONTAINER-EXTENSIONS | Container validation, routing, failure cleanup, and policy tests |
