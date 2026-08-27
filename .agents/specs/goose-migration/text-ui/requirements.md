# Requirements: Text UI / Terminal UI (GPUI Equivalents)

## Introduction

Migrate goose's text-based terminal UI (TUI) features into zed's existing CLI and GPUI terminal infrastructure. Goose's `text/` is a separate React-based TUI application. In zed, equivalent functionality should be built into the existing CLI (`crates/cli/`) and, where appropriate, exposed through GPUI terminal views or a new GPUI-based TUI mode.

Zed already provides: a CLI (`crates/cli/`) with various subcommands, a terminal view (`crates/terminal_view/`), a terminal emulator (`crates/terminal/`), and an inline assistant. Goose's text UI adds: an interactive TUI session, markdown rendering in the terminal, configuration wizard flow, extension management, onboarding, and slash commands.

## Glossary

- **CLI**: Zed's existing command-line interface (`crates/cli/`)
- **TUI**: Terminal User Interface — an interactive text-based UI distinct from batch CLI commands
- **GPUI Terminal**: Zed's embedded terminal emulator (`crates/terminal/`, `crates/terminal_view/`)
- **Inline Assistant**: Zed's existing inline agent within the editor/terminal (`crates/agent_ui/src/inline_assistant.rs`)
- **Markdown Rendering**: Zed already has `crates/markdown/` for markdown parsing and rendering

## Requirements

### Requirement 1: Interactive TUI Session

**User Story:** As a zed user, I want an interactive terminal UI for agent conversations, so that I can have a rich chat experience without the desktop GUI.

#### Acceptance Criteria

1. **1.1** THE CLI SHALL support an interactive TUI mode
2. **1.2** THE TUI mode SHALL display conversation history in a scrollable view
3. **1.3** THE TUI mode SHALL support text input with history (up/down arrows)
4. **1.4** THE TUI mode SHALL support multiline input
5. **1.5** THE TUI mode SHALL preserve session state across invocations

**Mapping:** New interactive mode in `crates/cli/` using zed's existing `crates/terminal/` and `gpui` for rendering, or a simpler ratatui/crossterm-based approach if a full GPUI context is too heavy.

### Requirement 2: Configuration Wizard

**User Story:** As a zed user, I want to configure the agent interactively from the terminal, so that I can set up providers and preferences without a GUI.

#### Acceptance Criteria

1. **2.1** THE CLI SHALL provide an interactive configuration wizard
2. **2.2** THE wizard SHALL guide the user through provider setup
3. **2.3** THE wizard SHALL guide the user through extension/MCP configuration
4. **2.4** THE wizard SHALL persist configuration changes to zed's settings system

**Mapping:** New interactive subcommand in `crates/cli/`, leveraging zed's existing `crates/agent_settings/` and `crates/settings/`.

### Requirement 3: Extensions Management in Terminal

**User Story:** As a zed user, I want to list, add, and remove extensions from the terminal, so that I can manage the agent's capabilities without a GUI.

#### Acceptance Criteria

1. **3.1** THE CLI SHALL list installed extensions with their status
2. **3.2** THE CLI SHALL support adding new extensions
3. **3.3** THE CLI SHALL support removing extensions
4. **3.4** THE CLI SHALL display extension connection status

**Mapping:** Extend `crates/cli/` with extension management subcommands, leveraging `crates/context_server/` and `crates/extension/`.

### Requirement 4: Markdown Rendering in Terminal

**User Story:** As a zed user, I want agent responses rendered with markdown formatting in the terminal, so that code blocks, lists, and emphasis are visually clear.

#### Acceptance Criteria

1. **4.1** THE TUI mode SHALL render markdown with ANSI colors and formatting
2. **4.2** THE TUI mode SHALL support code blocks with syntax highlighting
3. **4.3** THE TUI mode SHALL support clickable links (if terminal supports it)
4. **4.4** THE TUI mode SHALL handle tables and other markdown elements

**Mapping:** Use or extend `crates/markdown/` for parsing, with terminal output formatting (ANSI escape codes). Already partially handled in existing agent output.

### Requirement 5: Terminal Onboarding

**User Story:** As a new zed user, I want an onboarding flow in the terminal, so that I can get started without a GUI.

#### Acceptance Criteria

1. **5.1** THE first launch of the TUI mode SHALL show an onboarding sequence
2. **5.2** THE onboarding SHALL guide through provider setup
3. **5.3** THE onboarding SHALL offer to run a tutorial interaction

**Mapping:** New onboarding flow in `crates/cli/`, possibly sharing content with `crates/onboarding/` but rendered for terminal.

### Requirement 6: Slash Commands in TUI

**User Story:** As a zed user, I want slash commands available in the TUI, so that I can quickly invoke recipes, skills, and actions.

#### Acceptance Criteria

1. **6.1** THE TUI mode SHALL support slash commands (e.g., `/help`, `/recipe`)
2. **6.2** THE TUI mode SHALL provide command autocomplete or suggestions
3. **6.3** THE TUI mode SHALL display available commands on `/help`

**Mapping:** Integrate goose's slash command patterns into `crates/cli/` and/or the agent prompt processing.

### Requirement 7: Tool Call Display

**User Story:** As a zed user, I want to see tool calls and results in the TUI, so that I can monitor what the agent is doing.

#### Acceptance Criteria

1. **7.1** THE TUI mode SHALL display tool calls with their arguments
2. **7.2** THE TUI mode SHALL display tool results when available
3. **7.3** THE TUI mode SHALL indicate when tools are executing (spinner/progress)

**Mapping:** Extend zed's existing tool execution display patterns (already present in agent_ui conversation view) to the terminal output.

### Requirement 8: Machine-Readable Agent Output

**User Story:** As an automation author, I want stable noninteractive output, so that scripts can consume agent runs without parsing presentation text.

#### Acceptance Criteria

1. **8.1** WHERE a headless agent CLI is approved, THE CLI SHALL support text, single JSON, and newline-delimited streaming JSON output
2. **8.2** IN JSON modes, stdout SHALL contain only the versioned machine-readable contract and diagnostics SHALL be written to stderr
3. **8.3** THE stream SHALL represent tool calls, tool results, messages, errors, cancellation, and exactly one terminal completion state
4. **8.4** THE CLI SHALL handle broken pipes, invalid Unicode, partial provider failure, and cancellation with documented exit codes

### Requirement 9: Completions and Manual Pages

**User Story:** As a terminal user, I want generated completions and manual pages, so that the approved CLI remains discoverable and consistent.

#### Acceptance Criteria

1. **9.1** THE approved CLI command tree SHALL generate completions for Bash, Zsh, Fish, PowerShell, and Nushell
2. **9.2** THE release workflow SHALL generate manual pages from the same command definitions
3. **9.3** TESTS SHALL detect drift between command definitions, completions, manual pages, and packaged artifacts

### Requirement 10: Review Command

**User Story:** As a developer, I want an optional headless diff-review command, so that I can request structured review without opening the desktop UI.

#### Acceptance Criteria

1. **10.1** WHERE the review command is approved, IT SHALL compose Zed's existing git, project, agent, model, and permission services rather than implement a parallel review engine
2. **10.2** THE command SHALL support an explicit diff range, prompt, model, check selection, severity filter, output format, and dry-run behavior
3. **10.3** THE command SHALL preserve repository state and return documented errors for invalid ranges, dirty-state conflicts, denied tools, failed checks, and provider failures

### Requirement 11: Shell Session Integration

**User Story:** As a terminal user, I want optional shell-session association, so that agent sessions can receive disclosed command context without unsafe prompt-hook behavior.

#### Acceptance Criteria

1. **11.1** WHERE shell integration is approved, THE CLI SHALL generate shell-specific initialization that can associate commands with an explicit agent session
2. **11.2** THE integration SHALL quote arguments safely, avoid recording secrets by default, prevent duplicate nested hooks, and support opt-out
3. **11.3** UNSUPPORTED shells or platforms SHALL fail with a clear diagnostic and SHALL NOT modify shell startup files implicitly

### Requirement 12: CLI Domain Ownership

**User Story:** As a maintainer, I want one command adapter over domain services, so that CLI parity does not duplicate product logic.

#### Acceptance Criteria

1. **12.1** THE migration SHALL inventory Goose session, provider, extension, plugin, skill, recipe, schedule, gateway, local-model, ACP, MCP, update, and validation commands and mark each reusable, extended, excluded, or unresolved
2. **12.2** EACH approved command SHALL call the owning Zed domain service and SHALL NOT introduce a second persistence, provider, extension, scheduler, or update implementation
3. **12.3** DEPRECATED or intentionally excluded Goose commands SHALL return a documented compatibility message where command-name compatibility is required

## Design Approach

```
goose text/ React Feature              →  zed CLI/TUI Equivalent
─────────────────────────────────────────────
tui.tsx (interactive session)          →  New interactive CLI mode in crates/cli/
configure.tsx (wizard)                  →  New CLI configure subcommand
extensions.tsx (ext mgmt)              →  Extend CLI with extension commands
markdown.tsx (rendering)               →  Use crates/markdown + ANSI formatting
onboarding.tsx                         →  New CLI onboarding flow
slashCommands.tsx                      →  Integrate into CLI input processing
toolcall.tsx                           →  Enhanced tool output in CLI
```

## References

- Source (goose): `projects/goose/ui/text/` — React TUI application
- Source (goose): `projects/goose/crates/goose-cli/src/cli.rs`, `session/mod.rs`, `session/output.rs`, `commands/review/`, `commands/term.rs`, `bin/generate_manpages.rs`
- Existing zed: `crates/cli/` — CLI framework
- Existing zed: `crates/terminal/`, `crates/terminal_view/` — terminal emulator
- Existing zed: `crates/markdown/` — markdown parsing and rendering
- Existing zed: `crates/agent_ui/src/inline_assistant.rs` — inline agent
- Existing zed: `crates/agent_ui/src/terminal_inline_assistant.rs` — terminal inline agent
- Existing zed: `crates/onboarding/` — onboarding views
- Existing zed: `crates/settings/`, `crates/agent_settings/` — settings system
