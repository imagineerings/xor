# Requirements: Agent Infrastructure

## Introduction

Migrate several foundational agent infrastructure features from goose: context management, plugins, hooks, subagent execution, platform extensions, large response handling, final output tool, agent snapshots, extension malware checking, action required manager, doctor, download manager, instance ID, prompt templates, subprocess management, built-in extensions, config migrations, and Zed mode.

## Glossary

- **Context Management**: Strategically managing the agent's context window (trimming, summarizing, compacting)
- **Plugin**: Dynamically loaded extension that adds functionality to the agent
- **Hook**: Extension point within the agent lifecycle for custom behavior
- **Subagent**: A child agent spawned by the main agent for parallel or delegated tasks
- **Platform Extension**: Built-in tools/features for platform capabilities (apps, code execution, orchestration, etc.)
- **Large Response Handler**: Logic for processing and truncating overly large LLM responses
- **Final Output Tool**: Special tool that delivers the agent's final answer
- **Snapshot**: Capture of agent state at a point in time for rollback or replay
- **Extension Malware Check**: Security scan of extensions before loading
- **Action Required Manager**: Tracks actions that need user attention
- **Doctor**: Diagnostic utility for troubleshooting system health
- **Goose Mode**: Operational modes that change agent behavior (e.g., "focus", "creative")
- **Config Migration**: Automatic migration of configuration files between versions

## Requirements

### Requirement 1: Context Management

**User Story:** As a zed user, I want the agent to manage its context window efficiently, so that it can handle long conversations without running out of context.

#### Acceptance Criteria

1. **1.1** THE context manager SHALL monitor context window usage
2. **1.2** WHEN the context window approaches capacity THEN the system SHALL apply configured strategies (summarize, trim, or compact)
3. **1.3** THE context management SHALL preserve critical information during compaction
4. **1.4** THE context management strategies SHALL be configurable by the user

### Requirement 2: Plugin System

**User Story:** As a zed developer, I want to create plugins that extend the agent's capabilities, so that third-party functionality can be added without modifying core code.

#### Acceptance Criteria

1. **2.1** THE plugin system SHALL support discovering plugins from configured directories
2. **2.2** THE plugin system SHALL support loading plugins dynamically
3. **2.3** THE plugin system SHALL support different plugin formats
4. **2.4** IF a plugin fails to load THEN the system SHALL report the error without crashing

### Requirement 3: Hook System

**User Story:** As a zed developer, I want lifecycle hooks in the agent, so that I can inject custom behavior at key points (before tool execution, after response, etc.).

#### Acceptance Criteria

1. **3.1** THE hook system SHALL provide lifecycle points where hooks can be registered
2. **3.2** THE hook system SHALL support multiple hooks at each lifecycle point
3. **3.3** WHEN a lifecycle point is reached THEN all registered hooks SHALL execute in order
4. **3.4** IF a hook fails THEN the system SHALL log the error and continue with remaining hooks
5. **3.5** THE hook system SHALL load command hooks from enabled plugin `hooks/hooks.json` files and ignore unknown events or action types compatibly.
6. **3.6** THE hook system SHALL support the audited Goose events for session, prompt, tool, file, shell, and stop lifecycle points.
7. **3.7** WHEN a hook runs, THE system SHALL pass the documented JSON context on stdin, apply matcher rules, substitute the plugin root, and enforce the configured/default timeout.
8. **3.8** IF a pre-tool hook denies execution, THEN THE agent SHALL not execute the tool and SHALL surface the plugin-provided reason.

### Requirement 4: Subagent Execution

**User Story:** As a zed user, I want the agent to spawn sub-agents for complex tasks, so that work can be parallelized and delegated.

#### Acceptance Criteria

1. **4.1** THE subagent system SHALL support spawning a child agent with a specific task
2. **4.2** THE subagent system SHALL support providing task configuration (model, instructions, tools)
3. **4.3** WHEN a subagent completes its task THEN it SHALL report results to the parent agent
4. **4.4** THE subagent system SHALL support notification events between parent and child agents
5. **4.5** IF a subagent fails THEN the parent agent SHALL be notified of the failure

### Requirement 5: Platform Extensions

**User Story:** As a zed user, I want the agent to have built-in platform capabilities (code execution, app management, orchestration, summarization, etc.), so that it can perform a wide range of tasks without external extensions.

#### Acceptance Criteria

1. **5.1** THE system SHALL provide a code execution extension for running code in sandboxed environments
2. **5.2** THE system SHALL provide an app extension for managing applications and windows
3. **5.3** THE system SHALL provide an orchestration extension for coordinating multi-step workflows
4. **5.4** THE system SHALL provide a chatrecall extension for searching chat history
5. **5.5** THE system SHALL provide a summarize extension for condensing content
6. **5.6** THE system SHALL provide a summon extension for bringing content into the conversation
7. **5.7** THE system SHALL provide a todo extension for task management
8. **5.8** THE system SHALL provide a tom extension (task-oriented management)
9. **5.9** THE system SHALL provide an analyze extension for code analysis
10. **5.10** THE system SHALL provide a developer extension for development workflows
11. **5.11** THE system SHALL provide an extension manager for managing extensions

### Requirement 6: Large Response Handler

**User Story:** As a zed user, I want the agent to handle large text tool results gracefully, so that a tool cannot overwhelm the model context.

#### Acceptance Criteria

1. **6.1** WHEN a text tool result exceeds the configured threshold, THE handler SHALL write the complete content to a permission-restricted temporary file and replace the result with its size and path.
2. **6.2** THE handler SHALL pass non-text and below-threshold content through unchanged.
3. **6.3** IF the file cannot be written, THEN THE handler SHALL return the original content with a visible warning rather than silently discard it.
4. **6.4** THE implementation SHALL define file lifetime, cleanup, and disclosure behavior before shipping.

### Requirement 7: Final Output Tool

**User Story:** As a zed user, I want the agent to produce structured final output, so that results are clearly communicated.

#### Acceptance Criteria

1. **7.1** WHERE a recipe declares a response JSON schema, THE agent SHALL register a transient final-output tool using that schema.
2. **7.2** WHEN the tool receives valid JSON, THE system SHALL store and return a deterministic serialized final output.
3. **7.3** IF the schema or submitted JSON is invalid, THEN THE system SHALL return specific validation errors and allow the model to correct the output within the configured retry limit.

### Requirement 8: Prompt Golden-Test Reconciliation

**User Story:** As a migration reviewer, I want upstream test artifacts classified accurately, so that golden files are not mistaken for product capabilities.

#### Acceptance Criteria

1. **8.1** THE migration SHALL classify `agents/snapshots/*.snap` as prompt regression fixtures and SHALL NOT plan runtime state capture/restore from them.
2. **8.2** WHERE prompt golden coverage is useful, THE implementation SHALL use Zed's repository-appropriate test pattern without adding a runtime snapshot manager.

### Requirement 9: Extension Malware Check

**User Story:** As a zed user, I want extensions to be checked for malicious content before loading, so that my system is protected from harmful extensions.

#### Acceptance Criteria

1. **9.1** WHEN an extension is loaded THEN the malware check SHALL scan its contents
2. **9.2** IF potential malware is detected THEN the extension SHALL be blocked
3. **9.3** THE malware check SHALL use configurable heuristics and patterns

### Requirement 10: Action Required Manager

**User Story:** As a zed user, I want the agent to track actions that need my attention, so that I don't miss important pending tasks.

#### Acceptance Criteria

1. **10.1** THE action required manager SHALL track actions requiring user intervention
2. **10.2** WHEN an action is completed THEN it SHALL be removed from the pending list
3. **10.3** THE pending actions SHALL be surfaced in the user interface

### Requirement 11: Doctor / Troubleshooting

**User Story:** As a zed user, I want a diagnostic tool that checks system health, so that I can troubleshoot configuration, connectivity, and dependency issues.

#### Acceptance Criteria

1. **11.1** THE doctor SHALL check provider connectivity
2. **11.2** THE doctor SHALL check extension/configuration validity
3. **11.3** THE doctor SHALL check system requirements and dependencies
4. **11.4** WHEN a check fails THEN the doctor SHALL provide actionable remediation steps
5. **11.5** THE doctor SHALL provide a summary of all checks with pass/fail status
6. **11.6** THE doctor SHALL cap and redact diagnostic/log content before it is included in an agent prompt or report.
7. **11.7** THE doctor SHALL NOT switch the configured provider or model without an approved policy and a user-visible record of the change.

### Requirement 12: Download Manager

**User Story:** As a zed system, I want a download manager for fetching assets, models, and updates, so that downloads are reliable and resumable.

#### Acceptance Criteria

1. **12.1** THE download manager SHALL support downloading files from URLs
2. **12.2** THE download manager SHALL support resuming interrupted downloads
3. **12.3** THE download manager SHALL report progress during downloads
4. **12.4** THE download manager SHALL support authenticated and sharded downloads, cancellation, duplicate-request protection, and resumable `.part` files.
5. **12.5** WHEN a download fails or is cancelled, THE manager SHALL retain or remove partial data according to an explicit retry/cleanup policy and report the error.
6. **12.6** BEFORE finalizing a download, THE manager SHALL verify expected size or integrity data when the source provides it and SHALL surface disk-space and permission failures.

### Requirement 13: Instance ID

**User Story:** As a zed developer, I want a unique instance identifier, so that I can correlate telemetry and diagnostics from individual installations.

#### Acceptance Criteria

1. **13.1** THE system SHALL generate a unique instance ID on first run
2. **13.2** THE instance ID SHALL be persisted across restarts
3. **13.3** THE instance ID SHALL be available for telemetry and diagnostic purposes

### Requirement 14: Prompt Templates

**User Story:** As a zed developer, I want a prompt template system, so that prompts can be parameterized and rendered consistently.

#### Acceptance Criteria

1. **14.1** THE prompt template system SHALL support variable substitution
2. **14.2** THE prompt template system SHALL support template composition (templates within templates)
3. **14.3** THE prompt template system SHALL handle missing variables gracefully

### Requirement 15: Subprocess Management

**User Story:** As a zed system, I want to manage subprocesses reliably, so that spawned processes are tracked and cleaned up properly.

#### Acceptance Criteria

1. **15.1** THE subprocess manager SHALL support spawning processes with arguments and environment
2. **15.2** THE subprocess manager SHALL track running subprocesses
3. **15.3** WHEN the application exits THE subprocess manager SHALL clean up remaining processes

### Requirement 16: Configuration Migration

**User Story:** As a zed user, I want configuration files to be automatically migrated between versions, so that I don't lose settings after an update.

#### Acceptance Criteria

1. **16.1** THE config migrator SHALL detect the current configuration version
2. **16.2** WHEN the current version is older than the application version THEN the migrator SHALL apply migration steps
3. **16.3** IF a migration fails THEN the system SHALL roll back to the previous configuration

### Requirement 17: Goose Mode

**User Story:** As a zed user, I want different agent modes (focus, creative, balanced), so that the agent's behavior can match my current task.

#### Acceptance Criteria

1. **17.1** THE system SHALL support multiple agent modes that change behavior
2. **17.2** WHEN a mode is selected THEN the system SHALL apply mode-specific prompts and settings
3. **17.3** THE mode SHALL be changeable during a session

### Requirement 18: Containerized Extension Execution

**User Story:** As a CLI user, I want selected extensions to execute inside an existing development container, so that their tools run in the intended isolated environment.

#### Acceptance Criteria

1. **18.1** WHERE container execution is approved, THE session SHALL route selected stdio and built-in extension processes through the configured container without changing unrelated extensions.
2. **18.2** THE system SHALL validate the container, working-directory mapping, binary availability, environment, and mount policy before starting the extension.
3. **18.3** IF the container exits, is unavailable, or lacks the extension, THEN THE agent SHALL surface an actionable extension failure and clean up owned processes.
4. **18.4** CONTAINER execution SHALL remain subject to Zed's tool permission, secret, filesystem, network, and process-cleanup policies.

## References

- Source: `projects/goose/crates/goose/src/context_mgmt/`
- Source: `projects/goose/crates/goose/src/plugins/`
- Source: `projects/goose/crates/goose/src/hooks/`
- Source: `projects/goose/crates/goose/src/agents/subagent_execution_tool/`, `subagent_handler.rs`, `subagent_task_config.rs`
- Source: `projects/goose/crates/goose/src/agents/platform_extensions/`
- Source: `projects/goose/crates/goose/src/agents/large_response_handler.rs`
- Source: `projects/goose/crates/goose/src/agents/final_output_tool.rs`
- Source: `projects/goose/crates/goose/src/agents/snapshots/`
- Source: `projects/goose/crates/goose/src/agents/extension_malware_check.rs`
- Source: `projects/goose/crates/goose/src/action_required_manager.rs`
- Source: `projects/goose/crates/goose/src/doctor.rs`
- Source: `projects/goose/crates/goose/src/download_manager.rs`
- Source: `projects/goose/crates/goose/src/instance_id.rs`
- Source: `projects/goose/crates/goose/src/prompt_template.rs`
- Source: `projects/goose/crates/goose/src/subprocess.rs`
- Source: `projects/goose/crates/goose/src/config/migrations.rs`, `config/goose_mode.rs` (renamed to Zed mode in Zed)
