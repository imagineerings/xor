# Implementation Plan: Developer Experience

## Overview

Implement slash commands, hints system, goose apps (GPUI panels), source roots, and execution manager by extending existing sim crates.

## Repo Reconciliation

- A slash-command path already exists for `/compact`, MCP prompts, and skills in `crates/agent/src/agent.rs`, with UI autocomplete support in `crates/agent_ui/src/conversation_view/thread_view.rs`.
- Treat Goose slash-command work as incremental command coverage, not a new parser/system.

## Tasks

- [ ] 1. Extend existing slash command handling for Goose-specific commands
  - Audit existing `/compact`, MCP prompt, and skill invocation paths
  - Add missing Goose commands such as `/recipe`, `/help`, and `/clear`
  - Reuse existing autocomplete and prompt dispatch flow
  - Add arguments parsing only where existing command parsing is insufficient
  - Add only source-confirmed commands to `build_available_commands_for_project()` and dispatch them through `NativeAgentConnection::prompt()`
  - Reuse `leading_native_command()` and `send_command_queueing_remainder()`; do not introduce a second parser
  - Implement `/clear` through existing thread mutation and `/recipe` through the recipe owner; do not ship placeholder success output
  - Verify help output, argument errors, cancellation, and ACP/native command consistency

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/agent.rs, crates/agent_ui/src/conversation_view/thread_view.rs_
  - _Writes: crates/agent/src/agent.rs, crates/agent_ui/src/conversation_view/thread_view.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Reconcile Goose hints with Sim instructions and skills
  - Extend the existing instruction/skill discovery owner only where Goose hint precedence or imports add observable behavior
  - Supports global hints (`~/.config/sim/hints/`) and project hints (`.simhints` in worktree roots)
  - Hint content injected into agent context via `ProjectContext.hints_content`
  - `@import` file resolution for hints that reference other files
  - Hints rendered in `## Project Hints` section of system prompt

  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/hints/mod.rs, crates/agent/src/hints/loader.rs_
  - _Writes: crates/agent/src/hints/mod.rs, crates/agent/src/hints/loader.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Define and, if approved, implement one secure MCP Apps boundary
  - Reuse `context_server` resource ownership and the existing agent conversation surface
  - Specify CSP, origin isolation, navigation, download, clipboard, tool authorization, resource caching, and retirement before enabling HTML execution
  - Keep native GPUI views separate; do not create a `sim_apps` registry or duplicate chat and clock applications
  - Show renderer/server failures without breaking the conversation or partially executing privileged operations

  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/agents/mcp_app_proxy.rs, projects/goose/ui/desktop/src/components/McpApps/, crates/context_server/, crates/agent_ui/src/conversation_view/_
  - _Writes: crates/context_server/, crates/agent_ui/src/conversation_view/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Implement source roots and sources
  - Define SourceRoot and Source types
  - Path resolution from source name
  - Priority-based source ordering
  - Add unit tests for path resolution, priority ordering, add/remove, and fallback resolution
  - Register modules and pub use in `crates/agent/src/agent.rs`

  - _Requirements: 4.1, 4.2, 4.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/source_roots.rs, crates/agent/src/sources.rs_
  - _Writes: crates/agent/src/source_roots.rs, crates/agent/src/sources.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Extend existing agent session initialization and restoration lifecycle
  - Coalesce concurrent initialization for the same session in `Agent`/`ThreadStore`
  - Restore evicted session, provider, and extension state before accepting work
  - Propagate one result or error to every waiter and support cancellation with cleanup
  - Add lifecycle, concurrent waiter, error, cancellation, and restoration tests

  - _Requirements: 5.1, 5.2, 5.3, 5.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/execution/manager.rs, crates/agent/src/agent.rs, crates/agent/src/thread_store.rs_
  - _Writes: crates/agent/src/agent.rs, crates/agent/src/thread_store.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Write tests
  - Slash command parsing and routing (pre-existing)
  - Hint discovery and loading (pre-existing)
  - MCP App isolation, bridge authorization, resource retirement, and failure containment if approved
  - Source root resolution — 7 tests in `source_roots.rs`
  - Sources content resolution — 6 tests in `sources.rs`
  - Shared initialization, restoration, cancellation, and waiter consistency

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.4, 3.5, 4.1, 4.2, 4.3, 5.1, 5.2, 5.3, 5.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: none_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

## Notes

- Slash commands are parsed before the input reaches the LLM, giving them priority over model-generated commands
- Hints integrate with the prompt builder to inject context
- Embedded MCP Apps remain conditional on an approved web-content security boundary
