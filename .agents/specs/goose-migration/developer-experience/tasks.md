# Implementation Plan: Developer Experience

## Overview

Implement slash commands, hints system, goose apps (GPUI panels), source roots, and execution manager by extending existing baymax crates.

## Tasks

- [ ] 1. Implement slash command system in agent input processing
  - Create `SlashCommandParser` that detects `/command` patterns in agent input
  - Create `SlashCommandRouter` with handler registration
  - Implement built-in slash commands: `/help`, `/recipe`, `/skill`, `/clear`
  - _Requirements: 1_
  - _writes: crates/agent/src/slash_commands/mod.rs, crates/agent/src/slash_commands/parser.rs, crates/agent/src/slash_commands/builtin.rs_

- [ ] 2. Implement hints system
  - Create `HintLoader` that discovers `.goosehints` files
  - Support global hints (`~/.config/baymax/hints/`) and project hints (`.goosehints` in project root)
  - Hint content injection into agent context
  - File import resolution for hints that reference other files
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
