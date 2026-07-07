# Implementation Plan: Agent Infrastructure

## Overview

Implement ~17 agent infrastructure features across existing and new crates. Most features extend `crates/agent/`; some get their own crate (doctor, download_manager, plugins).

## Repo Reconciliation

- Existing context/token-window support lives in `crates/agent/src/thread.rs` and `crates/agent_settings/src/agent_settings.rs`: auto compaction, manual `/compact`, token usage updates, compaction telemetry, and threshold settings already exist.
- Existing subagent support lives in `crates/agent/src/tools/spawn_agent_tool.rs`, `crates/agent/src/thread.rs`, `crates/agent/src/agent.rs`, and `crates/acp_thread/src/acp_thread.rs`: spawning, resuming, depth limiting, parent cancellation, subagent session metadata, and tests already exist.
- Existing prompt template support lives in `crates/agent/src/templates.rs` and `crates/prompt_store/src/prompts.rs`; do not add a generic `crates/util/src/prompt_template.rs` until a concrete Goose-only gap is identified.

## Tasks

- [x] 1. Reconcile and extend existing context management
  - Audit existing auto/manual compaction before adding new files
  - Identify Goose-only gaps beyond current summary compaction
  - Add only missing strategies (for example trim or drop-least-relevant) if still required
  - Preserve existing `Thread` compaction events and telemetry
  - _Requirements: 1_
  - _writes: crates/agent/src/thread.rs, crates/agent_settings/src/agent_settings.rs_

- [x] 2. Implement hook system
  - Define hook points (before/after tool execution, LLM call, session start/end)
  - Hook registration and ordered execution
  - Hook return value handling (continue, abort, modify context)
  - _Requirements: 3_
  - _writes: crates/agent/src/hooks.rs, crates/agent/src/thread.rs_

- [x] 3. Create plugins crate
  - Plugin discovery from configured directories
  - Plugin format support and loading
  - Error handling for failed plugin loads
  - _Requirements: 2_
  - _writes: crates/plugins/src/lib.rs_

- [x] 4. Reconcile and extend existing subagent execution
  - Compare Goose task configuration against existing `SpawnAgentToolInput` and `SubagentContext`
  - Add only missing fields or behavior (for example task-specific instructions, tool scoping, or timeout policy)
  - Preserve existing parent-child events, resume support, depth limiting, cancellation, and tests
  - _Requirements: 4_
  - _writes: crates/agent/src/tools/spawn_agent_tool.rs, crates/agent/src/thread.rs, crates/agent/src/agent.rs, crates/agent/src/tests/mod.rs_

- [x] 5. Implement platform extensions
  - [x] 5.1 Code execution extension — sandboxed code running
    - _Requirements: 5.1_
    - _writes: crates/agent/src/tools/code_execution.rs_
  - [x] 5.2 Orchestrator extension — multi-step workflow coordination
    - _Requirements: 5.3_
    - _writes: crates/agent/src/tools/orchestrator.rs_
  - [x] 5.3 Summarize extension — content summarization
    - _Requirements: 5.5_
    - _writes: crates/agent/src/tools/summarize.rs_
  - [x] 5.4 Todo extension — task management
    - _Requirements: 5.7_
    - _writes: crates/agent/src/tools/todo.rs_
  - [x] 5.5 Apps, Chatrecall, Summon, Tom, Analyze, Developer extensions
    - _Requirements: 5.2, 5.4, 5.6, 5.8, 5.9, 5.10_
    - _writes: crates/agent/src/platform_extensions/_

- [x] 6. Implement large response handler
  - Detect and truncate/split oversized LLM responses
  - Preserve semantic completeness where possible
  - Notify agent when truncation occurs
  - _Requirements: 6_
  - _writes: crates/agent/src/large_response_handler.rs, crates/agent/src/thread.rs_

- [x] 7. Implement final output tool
  - Structured formatting of agent's final response
  - Include relevant context and results
  - _Requirements: 7_
  - _writes: crates/agent/src/tools/final_output_tool.rs, crates/agent/src/tools.rs, crates/agent/src/thread.rs_

- [x] 8. Implement agent snapshot
  - Capture agent state (conversation, tool state)
  - Restore from snapshot
  - _Requirements: 8_
  - _writes: crates/agent/src/snapshot.rs_

- [x] 9. Implement extension malware check
  - Scan extension contents on load
  - Configurable heuristics and patterns
  - Block extensions that match malware patterns
  - _Requirements: 9_
  - _writes: crates/agent/src/extension_malware_check.rs_

- [x] 10. Create doctor crate
  - Implement individual health checks (provider connectivity, extensions, system deps)
  - Aggregated report with pass/warning/fail per check
  - Actionable remediation steps
  - _Requirements: 11_
  - _writes: crates/doctor/src/lib.rs, crates/doctor/src/checks.rs_

- [x] 11. Implement download manager
  - URL download with progress reporting
  - Resume support for interrupted downloads
  - _Requirements: 12_
  - _writes: crates/download_manager/src/lib.rs_

- [x] 12. Implement small infrastructure features
  - [x] 12.1 Instance ID — generate and persist unique instance identifier
    - _Requirements: 13_
    - _writes: crates/util/src/instance_id.rs_
  - [x] 12.2 Prompt template — audit existing Handlebars/prompt-store templates and add only missing Goose behavior
    - _Requirements: 14_
    - _writes: crates/agent/src/templates.rs_
  - [x] 12.3 Subprocess manager — process lifecycle and cleanup
    - _Requirements: 15_
    - _writes: crates/util/src/subprocess.rs_
  - [x] 12.4 Action required manager — track pending user actions
    - _Requirements: 10_
    - _writes: crates/agent/src/action_required_manager.rs_
  - [x] 12.5 Built-in extensions registry
    - _Requirements: implicit_
    - _writes: crates/agent/src/builtin_extensions.rs_
  - [x] 12.6 Configuration migration and Sim mode
    - [x] 12.6.1 Config migrator — version detection, migration steps, rollback
      - _Requirements: 16_
      - _writes: crates/settings/src/migrations.rs_
    - [x] 12.6.2 Sim mode — Focus, Creative, Balanced modes
      - _Requirements: 17_
      - _writes: crates/agent_settings/src/sim_mode.rs_

- [x] 13. Write tests
  - Context manager compaction accuracy
  - Hook execution order and error handling
  - Subagent spawning and communication
  - Platform extension tool registration
  - Doctor check accuracy
  - Config migration apply and rollback
  - _Requirements: 1-17_

## Notes

- Many of these features are small enough to implement in a single session
- Platform extensions build on the existing tool registration pattern in `crates/agent/src/tools/`
- Config migration runs automatically on settings load
- Sim mode settings are consumed by the agent's prompt builder
