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

- [x] 3. Implement baymax apps as GPUI panels
  - Created `AppRegistry` for registering and launching embedded apps
  - Implemented `ChatApp` (plain struct with `BaymaxApp` trait impl, chat-like interface)
  - Implemented `ClockApp` (plain struct with `BaymaxApp` trait impl, clock/time display)
  - Implemented `ResourceManager` for app data (key-value with JSON values)
  - Implemented `CacheManager` for app cache (key-value with optional TTL)
  - Created `AppsPanel` GPUI Entity implementing `Panel`, `Focusable`, `EventEmitter<PanelEvent>`, `Render`
  - `AppsPanel` owns `AppRegistry`, registers ChatApp + ClockApp by default, launches ChatApp on startup
  - Panel registered in workspace initialization (`initialize_panels` in `crates/baymax/src/baymax.rs`)
  - ToggleFocus action registered for keyboard shortcut
  - AppsPanel docked to right side by default, 320px width
  - Removed dead `ChatApp.input` field, removed GPUI Entity pattern from ChatApp/ClockApp
  - _Requirements: 3_
  - _writes: crates/baymax_apps/src/baymax_apps.rs, crates/baymax_apps/src/chat_app.rs, crates/baymax_apps/src/clock_app.rs, crates/baymax_apps/src/app_registry.rs, crates/baymax_apps/src/resource_manager.rs, crates/baymax_apps/src/cache_manager.rs, crates/baymax/src/baymax/apps_panel.rs, crates/baymax/src/baymax.rs, crates/baymax/Cargo.toml_

- [x] 4. Implement source roots and sources
  - Defined `SourceRoot` and `SourceRoots` types with `add_root`, `get`, `resolve`, `remove`, `roots`, `into_roots`
  - Defined `Source` type with `resolve()` against a `SourceRoots` collection
  - Defined `Sources` collection with `add`, `get`, `resolve`, `names`
  - Path resolution from `"root_name/relative/path"` format
  - Priority-based ordering with `roots()` returning highest-priority first
  - Wired into `crates/agent` as `pub mod source_roots`
  - 12 unit tests covering add/get/missing/resolve/priority/remove/sources collection
  - _Requirements: 4_
  - _writes: crates/agent/src/source_roots.rs_

- [x] 5. Implement execution manager
  - Track running tasks with metadata
  - Support spawn, cancel, status, and list operations
  - Integration with agent task spawning
  - _Requirements: 5_
  - _writes: crates/agent/src/execution_manager.rs_

- [x] 6. Write tests
  - Slash command parsing and routing (14 tests: parse, unqualified, MCP, skill scope, edge cases)
  - Hint discovery and loading (10 tests: project hints, multiple roots, empty content, imports, load_all)
  - App registration and lifecycle (11 tests: register, launch, close, list, active app)
  - ChatApp and ClockApp methods (9 tests: new, add_message, tick, set_label)
  - Source root resolution (12 tests, pre-existing)
  - Execution manager task lifecycle (7 tests, pre-existing)
  - _Requirements: 1-5_
  - _writes: crates/agent/src/agent.rs, crates/agent/src/hints/loader.rs, crates/baymax_apps/src/app_registry.rs, crates/baymax_apps/src/chat_app.rs, crates/baymax_apps/src/clock_app.rs_

## Notes

- Slash commands are parsed before the input reaches the LLM, giving them priority over model-generated commands
- Hints integrate with the prompt builder to inject context
- Baymax apps follow the same GPUI Entity pattern as existing workspace panels
