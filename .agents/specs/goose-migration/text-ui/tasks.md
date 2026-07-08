# Implementation Plan: Text UI / Terminal UI

## Overview

Implement an interactive terminal UI mode for sim's CLI using crossterm/ratatui, with a configuration wizard, extension management, markdown rendering, onboarding, and slash commands. This extends `crates/cli/` with a new `goose interactive` subcommand.

## Repo Reconciliation

- Agent slash-command dispatch already exists for desktop/ACP flows. The TUI should call into shared command behavior where possible instead of creating a divergent slash-command implementation.

## Tasks

- [x] 1. Implement interactive session core
  - Create `InteractiveSession` with conversation state management
  - Implement multiline input with history (up/down navigation)
  - Terminal raw mode setup/teardown
  - _Requirements: 1_
  - _writes: crates/cli/src/interactive/session.rs, crates/cli/src/interactive/input.rs_

- [x] 2. Implement terminal renderer
  - Markdown to ANSI rendering (headings, lists, bold, italic, code blocks)
  - Syntax highlighting for code blocks
  - Progress spinner for loading states
  - Tool call display with state (running/completed/failed)
  - _Requirements: 4, 7_
  - _writes: crates/cli/src/interactive/renderer.rs, crates/cli/src/interactive/markdown_renderer.rs_

- [x] 3. Wire interactive mode to shared slash command behavior
  - Add TUI-only commands such as `/help`, `/clear`, `/save`, `/load`, and `/model`
  - Reuse agent command paths for `/recipe`, `/skill`, and `/compact` where possible
  - Add autocomplete/suggestions backed by the shared command catalog
  - _Requirements: 6_
  - _writes: crates/cli/src/interactive/slash_commands.rs_

- [ ] 4. Implement configuration wizard
  - Step-by-step wizard for provider setup
  - Input types: text (with secret masking), confirm, select, file
  - Validation per step
  - Persist to sim settings
  - _Requirements: 2_
  - _writes: crates/cli/src/commands/configure.rs_

- [ ] 5. Implement extension management commands
  - `goose extension list` — list installed extensions with status
  - `goose extension add <path>` — add extension
  - `goose extension remove <name>` — remove extension
  - `goose extension status <name>` — extension connection status
  - _Requirements: 3_
  - _writes: crates/cli/src/commands/extension.rs_

- [ ] 6. Implement CLI onboarding flow
  - First-run detection
  - Welcome message with introduction
  - Provider setup prompt
  - First interaction tutorial
  - _Requirements: 5_
  - _writes: crates/cli/src/commands/onboarding.rs_

- [ ] 7. Wire interactive mode into CLI
  - Add `goose interactive` subcommand
  - Handle CLI arguments and configuration
  - Graceful shutdown (Ctrl+C, Ctrl+D)
  - _Requirements: 1_
  - _writes: crates/cli/src/main.rs (modifications), crates/cli/src/commands/interactive.rs_

- [ ] 8. Write tests
  - Markdown rendering with known input/output pairs
  - Slash command parsing and routing
  - Configuration wizard flow (all paths)
  - Extension command output
  - Interactive session input processing
  - _Requirements: 1-7_

## Notes

- Interactive mode uses `crossterm` for terminal control and `ratatui` for rendering
- Exiting interactive mode returns to the normal CLI shell
- Session state is saved automatically and can be resumed
