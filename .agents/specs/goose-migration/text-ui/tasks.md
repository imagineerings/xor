# Implementation Plan: Text UI / Terminal UI

## Overview

Implement an interactive terminal UI mode for zed's CLI using crossterm/ratatui, with a configuration wizard, extension management, markdown rendering, onboarding, and slash commands. This extends `crates/cli/` with a new `goose interactive` subcommand.

## Repo Reconciliation

- Agent slash-command dispatch already exists for desktop/ACP flows. The TUI should call into shared command behavior where possible instead of creating a divergent slash-command implementation.

## Tasks

- [ ] 1. Implement interactive session core
  - Create `InteractiveSession` with conversation state management
  - Implement multiline input with history (up/down navigation)
  - Terminal raw mode setup/teardown

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/interactive/session.rs, crates/cli/src/interactive/input.rs_
  - _Writes: crates/cli/src/interactive/session.rs, crates/cli/src/interactive/input.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Implement terminal renderer
  - Markdown to ANSI rendering (headings, lists, bold, italic, code blocks)
  - Syntax highlighting for code blocks
  - Progress spinner for loading states
  - Tool call display with state (running/completed/failed)

  - _Requirements: 4.1, 4.2, 4.3, 4.4, 7.1, 7.2, 7.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/interactive/renderer.rs, crates/cli/src/interactive/markdown_renderer.rs_
  - _Writes: crates/cli/src/interactive/renderer.rs, crates/cli/src/interactive/markdown_renderer.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Wire interactive mode to shared slash command behavior
  - Add TUI-only commands such as `/help`, `/clear`, `/save`, `/load`, and `/model`
  - Reuse agent command paths for `/recipe`, `/skill`, and `/compact` where possible
  - Add autocomplete/suggestions backed by the shared command catalog

  - _Requirements: 6.1, 6.2, 6.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/interactive/slash_commands.rs_
  - _Writes: crates/cli/src/interactive/slash_commands.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Implement configuration wizard
  - Step-by-step wizard for provider setup
  - Input types: text (with secret masking), confirm, select, file
  - Validation per step
  - Persist to zed settings

  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/commands/configure.rs_
  - _Writes: crates/cli/src/commands/configure.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Implement extension management commands
  - `goose extension list` — list installed extensions with status
  - `goose extension add <path>` — add extension
  - `goose extension remove <name>` — remove extension
  - `goose extension status <name>` — extension connection status

  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/commands/extension.rs_
  - _Writes: crates/cli/src/commands/extension.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Implement CLI onboarding flow
  - First-run detection
  - Welcome message with introduction
  - Provider setup prompt
  - First interaction tutorial

  - _Requirements: 5.1, 5.2, 5.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/commands/onboarding.rs_
  - _Writes: crates/cli/src/commands/onboarding.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 7. Wire interactive mode into CLI
  - Add `goose interactive` subcommand
  - Handle CLI arguments and configuration
  - Graceful shutdown (Ctrl+C, Ctrl+D)

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/cli/src/main.rs (modifications), crates/cli/src/commands/interactive.rs_
  - _Writes: crates/cli/src/main.rs (modifications), crates/cli/src/commands/interactive.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 8. Write tests
  - Markdown rendering with known input/output pairs
  - Slash command parsing and routing
  - Configuration wizard flow (all paths)
  - Extension command output
  - Interactive session input processing

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3, 7.1, 7.2, 7.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: none_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 9. Define and, if approved, implement machine-readable agent output
  - Specify a versioned JSON and streaming-event contract before implementation
  - Keep stdout machine-readable and route logs/diagnostics to stderr
  - Cover terminal events, tool activity, partial failures, cancellation, broken pipes, Unicode, and exit codes

  - _Requirements: 8.1, 8.2, 8.3, 8.4_
  - _Depends on: 1, 7_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, projects/goose/crates/goose-cli/src/session/mod.rs, projects/goose/crates/goose-cli/src/session/output.rs, crates/cli/_
  - _Writes: crates/cli/_
  - _Validation: Run JSON-schema snapshots, stdout/stderr isolation, broken-pipe, Unicode, cancellation, partial-failure, and exit-code tests_

- [ ] 10. Generate completions and manual pages for the approved command tree
  - Generate Bash, Zsh, Fish, PowerShell, and Nushell completions from canonical command definitions
  - Generate manual pages from the same definitions and package them through the existing release owner

  - _Requirements: 9.1, 9.2, 9.3_
  - _Depends on: 12_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, projects/goose/crates/goose-cli/src/bin/generate_manpages.rs, crates/cli/, script/, .github/workflows/_
  - _Writes: crates/cli/, script/, .github/workflows/_
  - _Validation: Regenerate completions and manual pages in CI and fail on command-tree or packaged-artifact drift_

- [ ] 11. Resolve optional review and shell-integration product scope
  - If review is approved, compose existing git/project/agent services and preserve repository state in dry-run and failure paths
  - If shell integration is approved, generate opt-in shell hooks with safe quoting, privacy defaults, nested-hook prevention, and no implicit startup-file edits

  - _Requirements: 10.1, 10.2, 10.3, 11.1, 11.2, 11.3_
  - _Depends on: 9, 12_
  - _Reads: .agents/specs/goose-migration/text-ui/requirements.md, .agents/specs/goose-migration/text-ui/design.md, projects/goose/crates/goose-cli/src/commands/review/, projects/goose/crates/goose-cli/src/commands/term.rs, crates/git/, crates/project/, crates/agent/, crates/terminal/_
  - _Writes: crates/cli/_
  - _Validation: Review dry-run/state-preservation tests plus shell quoting, redaction, nested-hook, opt-out, and unsupported-platform tests_

- [ ] 12. Reconcile every Goose command with a Zed domain owner
  - Inventory session, provider, extension, plugin, skill, recipe, schedule, gateway, local-model, ACP, MCP, update, and validation commands
  - Record reuse, extension, exclusion, or unresolved status and route approved commands to their existing domain services
  - Add compatibility diagnostics for approved deprecated command names without duplicating domain logic

  - _Requirements: 12.1, 12.2, 12.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose-cli/src/cli.rs, crates/cli/, crates/agent/, crates/agent_servers/, crates/context_server/, crates/auto_update/_
  - _Writes: .agents/specs/goose-migration/text-ui/design.md, crates/cli/_
  - _Validation: Review the command ownership matrix and run dispatch tests proving approved commands call the canonical domain owner_

## Notes

- Interactive mode uses `crossterm` for terminal control and `ratatui` for rendering
- Exiting interactive mode returns to the normal CLI shell
- Session state is saved automatically and can be resumed
