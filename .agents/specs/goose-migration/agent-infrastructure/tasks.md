# Implementation Plan: Agent Infrastructure

> Cross-cutting contract: every production write in this plan inherits the [`agentic` feature boundary](../feature-boundary.md). Completion evidence must classify actual writes and include the required enabled/disabled validation.

## Overview

Implement ~17 agent infrastructure features across existing and new crates. Most features extend `crates/agent/`; some get their own crate (doctor, download_manager, plugins).

## Repo Reconciliation

- Existing context/token-window support lives in `crates/agent/src/thread.rs` and `crates/agent_settings/src/agent_settings.rs`: auto compaction, manual `/compact`, token usage updates, compaction telemetry, and threshold settings already exist.
- Existing subagent support lives in `crates/agent/src/tools/spawn_agent_tool.rs`, `crates/agent/src/thread.rs`, `crates/agent/src/agent.rs`, and `crates/acp_thread/src/acp_thread.rs`: spawning, resuming, depth limiting, parent cancellation, subagent session metadata, and tests already exist.
- Existing prompt template support lives in `crates/agent/src/templates.rs` and `crates/prompt_store/src/prompts.rs`; do not add a generic `crates/util/src/prompt_template.rs` until a concrete Goose-only gap is identified.

## Tasks

- [ ] 1. Reconcile and extend existing context management
  - Audit existing auto/manual compaction before adding new files
  - Identify Goose-only gaps beyond current summary compaction
  - Add only missing strategies (for example trim or drop-least-relevant) if still required
  - Preserve existing `Thread` compaction events and telemetry

  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/thread.rs, crates/agent_settings/src/agent_settings.rs_
  - _Writes: crates/agent/src/thread.rs, crates/agent_settings/src/agent_settings.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Implement hook system
  - Define hook points (before/after tool execution, LLM call, session start/end)
  - Hook registration and ordered execution
  - Hook return value handling (continue, abort, modify context)

  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/hooks.rs, crates/agent/src/thread.rs_
  - _Writes: crates/agent/src/hooks.rs, crates/agent/src/thread.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Extend the existing agent-server and extension registry for Goose plugin discovery
  - Plugin discovery from configured directories
  - Plugin format support and loading
  - Error handling for failed plugin loads

  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/plugins/src/lib.rs_
  - _Writes: crates/plugins/src/lib.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Reconcile and extend existing subagent execution
  - Compare Goose task configuration against existing `SpawnAgentToolInput` and `SubagentContext`
  - Add only missing fields or behavior (for example task-specific instructions, tool scoping, or timeout policy)
  - Preserve existing parent-child events, resume support, depth limiting, cancellation, and tests

  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/tools/spawn_agent_tool.rs, crates/agent/src/thread.rs, crates/agent/src/agent.rs, crates/agent/src/tests/mod.rs_
  - _Writes: crates/agent/src/tools/spawn_agent_tool.rs, crates/agent/src/thread.rs, crates/agent/src/agent.rs, crates/agent/src/tests/mod.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Implement platform extensions
  - [ ] 5.1. Code execution extension — sandboxed code running
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 5.11_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/platform_
    - _Writes: crates/agent/src/platform_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 5.2. Orchestrator extension — multi-step workflow coordination
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 5.11_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/platform_
    - _Writes: crates/agent/src/platform_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 5.3. Summarize extension — content summarization
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 5.11_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/platform_
    - _Writes: crates/agent/src/platform_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 5.4. Todo extension — task management
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 5.11_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/platform_
    - _Writes: crates/agent/src/platform_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 5.5. Apps, Chatrecall, Summon, Tom, Analyze, Developer extensions

  - _Requirements: 5.2, 5.4, 5.6, 5.8, 5.9, 5.10_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/platform_extensions/_
  - _Writes: crates/agent/src/platform_extensions/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Handle oversized text tool results at the central ingestion boundary
  - Persist complete above-threshold text to a restricted temporary file and replace it with a reference
  - Pass other content through unchanged and surface write failures without data loss
  - Define cleanup and retention behavior

  - _Requirements: 6.1, 6.2, 6.3, 6.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/large_response_handler.rs, crates/agent/src/thread.rs_
  - _Writes: crates/agent/src/large_response_handler.rs, crates/agent/src/thread.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 7. Implement final output tool
  - Structured formatting of agent's final response
  - Include relevant context and results

  - _Requirements: 7.1, 7.2_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/tools/final_output_tool.rs, crates/agent/src/tools.rs, crates/agent/src/thread.rs_
  - _Writes: crates/agent/src/tools/final_output_tool.rs, crates/agent/src/tools.rs, crates/agent/src/thread.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 8. Reconcile prompt golden-test coverage without adding runtime snapshots
  - Confirm upstream `.snap` files are prompt fixtures
  - Add repository-native prompt regression coverage only where it detects a real compatibility regression

  - _Requirements: 8.1, 8.2_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/snapshot.rs_
  - _Writes: existing prompt/template tests selected after coverage review_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 9. Implement extension malware check
  - Scan extension contents on load
  - Configurable heuristics and patterns
  - Block extensions that match malware patterns

  - _Requirements: 9.1, 9.2, 9.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/extension_malware_check.rs_
  - _Writes: crates/agent/src/extension_malware_check.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 10. Extend existing diagnostics with Goose Doctor provider and extension checks
  - Implement individual health checks (provider connectivity, extensions, system deps)
  - Aggregated report with pass/warning/fail per check
  - Actionable remediation steps

  - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/doctor.rs, crates/agent_ui/src/diagnostics.rs, crates/diagnostics/, crates/language_models/_
  - _Writes: crates/agent_ui/src/diagnostics.rs, crates/diagnostics/, selected provider-health integration_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 11. Reconcile model downloads with existing Zed HTTP and cache owners
  - URL download with progress reporting
  - Resume support for interrupted downloads

  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose-download-manager/src/lib.rs, crates/http_client/, crates/auto_update/, crates/llama_cpp/_
  - _Writes: selected existing provider/cache owner and shared HTTP integration_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 12. Implement small infrastructure features
  - [ ] 12.1. Instance ID — generate and persist unique instance identifier
    - _Requirements: 13.1, 13.2, 13.3_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/instance_id.rs, crates/telemetry/, crates/settings/_
    - _Writes: selected existing telemetry/settings identity owner_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 12.2. Prompt template — audit existing Handlebars/prompt-store templates and add only missing Goose behavior
    - _Requirements: 14.1, 14.2, 14.3_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/prompt_template.rs, crates/agent/src/templates.rs, crates/prompt_store/_
    - _Writes: crates/agent/src/templates.rs, crates/prompt_store/_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 12.3. Subprocess manager — process lifecycle and cleanup
    - _Requirements: 15.1, 15.2, 15.3_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/subprocess_manager.rs, crates/agent_servers/, crates/context_server/_
    - _Writes: selected existing agent-server/context-server process owner_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 12.4. Action required manager — track pending user actions
    - _Requirements: 10.1, 10.2, 10.3_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/agents/action_required_manager.rs, crates/acp_thread/src/acp_thread.rs, crates/agent_ui/src/conversation_view/elicitation.rs_
    - _Writes: crates/acp_thread/src/acp_thread.rs, crates/agent_ui/src/conversation_view/elicitation.rs_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 12.5. Built-in extensions registry
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 5.11_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/agents/platform_extensions/, crates/agent/src/tools.rs, crates/agent/src/tools/_
    - _Writes: crates/agent/src/tools.rs, selected existing tool/skill owners_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 12.6. Configuration migration and Zed mode
    - Config migrator — version detection, migration steps, rollback
    - Zed mode — Focus, Creative, Balanced modes

  - _Requirements: 16.1, 16.2, 16.3, 17.1, 17.2, 17.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent_settings/src/zed_mode.rs_
  - _Writes: crates/agent_settings/src/zed_mode.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 13. Write tests
  - Context manager compaction accuracy
  - Hook execution order and error handling
  - Subagent spawning and communication
  - Platform extension tool registration
  - Doctor check accuracy
  - Config migration apply and rollback

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 5.11, 6.1, 6.2, 6.3, 6.4, 7.1, 7.2, 7.3, 8.1, 8.2, 9.1, 9.2, 9.3, 10.1, 10.2, 10.3, 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7, 12.1, 12.2, 12.3, 12.4, 12.5, 12.6, 13.1, 13.2, 13.3, 14.1, 14.2, 14.3, 15.1, 15.2, 15.3, 16.1, 16.2, 16.3, 17.1, 17.2, 17.3, 18.1, 18.2, 18.3, 18.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/agent-infrastructure/requirements.md, .agents/specs/goose-migration/agent-infrastructure/design.md, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: none_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 14. Add approved containerized extension execution
  - Route only selected extension processes through an existing development container
  - Validate container/workdir/binary/mount policy and propagate lifecycle failures
  - Preserve Zed permissions, secrets, filesystem, network, and cleanup policy
  - _Requirements: 18.1, 18.2, 18.3, 18.4_
  - _Depends on: 3_
  - _Reads: projects/goose/crates/goose/src/agents/container.rs, projects/goose/crates/goose-cli/src/cli.rs, crates/dev_container, crates/agent_servers_
  - _Writes: existing dev-container and agent-server integration files selected after product approval_
  - _Validation: focused container routing, missing container/binary, exit cleanup, permission, secret, filesystem, and network policy tests_

## Notes

- Many of these features are small enough to implement in a single session
- Platform extensions build on the existing tool registration pattern in `crates/agent/src/tools/`
- Config migration runs automatically on settings load
- Zed mode settings are consumed by the agent's prompt builder
