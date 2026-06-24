# Implementation Plan: Agent Infrastructure

## Overview

Implement ~17 agent infrastructure features across existing and new crates. Most features extend `crates/agent/`; some get their own crate (doctor, download_manager, plugins).

## Tasks

- [ ] 1. Implement context manager
  - Monitor context window token usage
  - Implement compaction strategies: summarize, trim, drop-least-relevant
  - Preserve critical information during compaction
  - _Requirements: 1_
  - _writes: crates/agent/src/context_manager.rs_

- [ ] 2. Implement hook system
  - Define hook points (before/after tool execution, LLM call, session start/end)
  - Hook registration and ordered execution
  - Hook return value handling (continue, abort, modify context)
  - _Requirements: 3_
  - _writes: crates/agent/src/hooks.rs_

- [ ] 3. Create plugins crate
  - Plugin discovery from configured directories
  - Plugin format support and loading
  - Error handling for failed plugin loads
  - _Requirements: 2_
  - _writes: crates/plugins/src/lib.rs_

- [ ] 4. Implement subagent execution
  - Subagent spawning with task configuration (model, instructions, tools)
  - Parent-child event/notification channel
  - Timeout and cancellation support
  - _Requirements: 4_
  - _writes: crates/agent/src/subagent.rs_

- [ ] 5. Implement platform extensions
  - [ ] 5.1 Code execution extension — sandboxed code running
    - _Requirements: 5.1_
    - _writes: crates/agent/src/tools/code_execution.rs_
  - [ ] 5.2 Orchestrator extension — multi-step workflow coordination
    - _Requirements: 5.3_
    - _writes: crates/agent/src/tools/orchestrator.rs_
  - [ ] 5.3 Summarize extension — content summarization
    - _Requirements: 5.5_
    - _writes: crates/agent/src/tools/summarize.rs_
  - [ ] 5.4 Todo extension — task management
    - _Requirements: 5.7_
    - _writes: crates/agent/src/tools/todo.rs_
  - [ ] 5.5 Apps, Chatrecall, Summon, Tom, Analyze, Developer extensions
    - _Requirements: 5.2, 5.4, 5.6, 5.8, 5.9, 5.10_
    - _writes: crates/agent/src/platform_extensions/_

- [ ] 6. Implement large response handler
  - Detect and truncate/split oversized LLM responses
  - Preserve semantic completeness where possible
  - Notify agent when truncation occurs
  - _Requirements: 6_
  - _writes: crates/agent/src/large_response_handler.rs_

- [ ] 7. Implement final output tool
  - Structured formatting of agent's final response
  - Include relevant context and results
  - _Requirements: 7_
  - _writes: crates/agent/src/tools/final_output.rs_

- [ ] 8. Implement agent snapshot
  - Capture agent state (conversation, tool state)
  - Restore from snapshot
  - _Requirements: 8_
  - _writes: crates/agent/src/snapshot.rs_

- [ ] 9. Implement extension malware check
  - Scan extension contents on load
  - Configurable heuristics and patterns
  - Block extensions that match malware patterns
  - _Requirements: 9_
  - _writes: crates/agent/src/extension_malware_check.rs_

- [ ] 10. Create doctor crate
  - Implement individual health checks (provider connectivity, extensions, system deps)
  - Aggregated report with pass/warning/fail per check
  - Actionable remediation steps
  - _Requirements: 11_
  - _writes: crates/doctor/src/lib.rs, crates/doctor/src/checks.rs_

- [ ] 11. Implement download manager
  - URL download with progress reporting
  - Resume support for interrupted downloads
  - _Requirements: 12_
  - _writes: crates/download_manager/src/lib.rs_

- [ ] 12. Implement small infrastructure features
  - [ ] 12.1 Instance ID — generate and persist unique instance identifier
    - _Requirements: 13_
    - _writes: crates/util/src/instance_id.rs_
  - [ ] 12.2 Prompt template — variable substitution and template composition
    - _Requirements: 14_
    - _writes: crates/util/src/prompt_template.rs_
  - [ ] 12.3 Subprocess manager — process lifecycle and cleanup
    - _Requirements: 15_
    - _writes: crates/util/src/subprocess.rs_
  - [ ] 12.4 Action required manager — track pending user actions
    - _Requirements: 10_
    - _writes: crates/agent/src/action_required_manager.rs_
  - [ ] 12.5 Built-in extensions registry
    - _Requirements: implicit_
    - _writes: crates/agent/src/builtin_extensions.rs_
  - [ ] 12.6 Configuration migration and goose mode
    - [ ] 12.6.1 Config migrator — version detection, migration steps, rollback
      - _Requirements: 16_
      - _writes: crates/settings/src/migrations.rs_
    - [ ] 12.6.2 Goose mode — Focus, Creative, Balanced modes
      - _Requirements: 17_
      - _writes: crates/agent_settings/src/goose_mode.rs_

- [ ] 13. Write tests
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
- Goose mode settings are consumed by the agent's prompt builder
