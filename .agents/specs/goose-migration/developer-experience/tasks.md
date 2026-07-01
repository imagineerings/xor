# Implementation Plan: Developer Experience

## Overview

Implement slash commands, hints system, goose apps (GPUI panels), source roots, and execution manager by extending existing baymax crates.

## Repo Reconciliation

- A slash-command path already exists for `/compact`, MCP prompts, and skills in `crates/agent/src/agent.rs`, with UI autocomplete support in `crates/agent_ui/src/conversation_view/thread_view.rs`.
- Treat Goose slash-command work as incremental command coverage, not a new parser/system.

## Tasks

- [x] 1. Extend existing slash command handling for Goose-specific commands
  - Audit existing `/compact`, MCP prompt, and skill invocation paths
  - Add missing Goose commands such as `/recipe`, `/help`, and `/clear`
  - Reuse existing autocomplete and prompt dispatch flow
  - Add arguments parsing only where existing command parsing is insufficient
  - Added `HELP_COMMAND_NAME`, `CLEAR_COMMAND_NAME`, `RECIPE_COMMAND_NAME` constants
  - Registered all three in `build_available_commands_for_project()` with `Native` category
  - Added dispatch in `NativeAgentConnection::prompt()` for each command
  - `send_help_command`: Queries available commands from ACP thread, formats markdown, injects via channel-based event stream
  - `send_clear_command`: Clears ACP thread entries via `AcpThread::clear_entries()`, shows confirmation
  - `send_recipe_command`: Placeholder listing recipes — full engine integration deferred to later task
  - Added `AcpThread::clear_entries()` to `crates/acp_thread/src/acp_thread.rs`
  - Added `recipe` dependency to `crates/agent/Cargo.toml`
  - Uses existing `leading_native_command()`/`send_command_queueing_remainder()` flow — no new parser needed
  - All existing `acp_thread` tests pass (70/70)
  - _Requirements: 1_
  - _writes: crates/agent/src/agent.rs, crates/agent_ui/src/conversation_view/thread_view.rs_

- [x] 2. Implement hints system
  - Created `HintLoader` in `crates/agent/src/hints/loader.rs`
  - Supports global hints (`~/.config/baymax/hints/`) and project hints (`.baymaxhints` in worktree roots)
  - Hint content injected into agent context via `ProjectContext.hints_content`
  - `@import` file resolution for hints that reference other files
  - Hints rendered in `## Project Hints` section of system prompt
  - _Requirements: 2_
  - _writes: crates/agent/src/hints/mod.rs, crates/agent/src/hints/loader.rs_

- [ ] 3. Implement goose apps as GPUI panels
  - Create `AppRegistry` for registering and launching embedded apps
  - Implement ChatApp GPUI panel (chat-like interface)
  - Implement ClockApp GPUI panel (clock/time tools)
  - Implement Resource and Cache managers for app state
  - _Requirements: 3_
  - _writes: crates/goose_apps/src/lib.rs, crates/goose_apps/src/chat.rs, crates/goose_apps/src/clock.rs_

- [ ] 4. Implement source roots and sources
  - Define SourceRoot and Source types
  - Path resolution from source name
  - Priority-based source ordering
  - _Requirements: 4_
  - _writes: crates/agent/src/source_roots.rs, crates/agent/src/sources.rs_

- [ ] 5. Implement execution manager
  - Track running tasks with metadata
  - Support spawn, cancel, status, and list operations
  - Integration with agent task spawning
  - _Requirements: 5_
  - _writes: crates/agent/src/execution_manager.rs_

- [ ] 6. Write tests
  - Slash command parsing and routing
  - Hint discovery and loading
  - App rendering and action handling
  - Source root resolution
  - Execution manager task lifecycle
  - _Requirements: 1-5_

## Notes

- Slash commands are parsed before the input reaches the LLM, giving them priority over model-generated commands
- Hints integrate with the prompt builder to inject context
- Goose apps follow the same panel pattern as existing GPUI workspace panels
