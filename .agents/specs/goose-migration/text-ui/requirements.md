# Requirements: Text UI / Terminal UI (GPUI Equivalents)

## Introduction

Migrate goose's text-based terminal UI (TUI) features into sim's existing CLI and GPUI terminal infrastructure. Goose's `text/` is a separate React-based TUI application. In sim, equivalent functionality should be built into the existing CLI (`crates/cli/`) and, where appropriate, exposed through GPUI terminal views or a new GPUI-based TUI mode.

Sim already provides: a CLI (`crates/cli/`) with various subcommands, a terminal view (`crates/terminal_view/`), a terminal emulator (`crates/terminal/`), and an inline assistant. Goose's text UI adds: an interactive TUI session, markdown rendering in the terminal, configuration wizard flow, extension management, onboarding, and slash commands.

## Glossary

- **CLI**: Sim's existing command-line interface (`crates/cli/`)
- **TUI**: Terminal User Interface — an interactive text-based UI distinct from batch CLI commands
- **GPUI Terminal**: Sim's embedded terminal emulator (`crates/terminal/`, `crates/terminal_view/`)
- **Inline Assistant**: Sim's existing inline agent within the editor/terminal (`crates/agent_ui/src/inline_assistant.rs`)
- **Markdown Rendering**: Sim already has `crates/markdown/` for markdown parsing and rendering

## Requirements

### Requirement 1: Interactive TUI Session

**User Story:** As a sim user, I want an interactive terminal UI for agent conversations, so that I can have a rich chat experience without the desktop GUI.

#### Acceptance Criteria

1. THE CLI SHALL support an interactive TUI mode
2. THE TUI mode SHALL display conversation history in a scrollable view
3. THE TUI mode SHALL support text input with history (up/down arrows)
4. THE TUI mode SHALL support multiline input
5. THE TUI mode SHALL preserve session state across invocations

**Mapping:** New interactive mode in `crates/cli/` using sim's existing `crates/terminal/` and `gpui` for rendering, or a simpler ratatui/crossterm-based approach if a full GPUI context is too heavy.

### Requirement 2: Configuration Wizard

**User Story:** As a sim user, I want to configure the agent interactively from the terminal, so that I can set up providers and preferences without a GUI.

#### Acceptance Criteria

1. THE CLI SHALL provide an interactive configuration wizard
2. THE wizard SHALL guide the user through provider setup
3. THE wizard SHALL guide the user through extension/MCP configuration
4. THE wizard SHALL persist configuration changes to sim's settings system

**Mapping:** New interactive subcommand in `crates/cli/`, leveraging sim's existing `crates/agent_settings/` and `crates/settings/`.

### Requirement 3: Extensions Management in Terminal

**User Story:** As a sim user, I want to list, add, and remove extensions from the terminal, so that I can manage the agent's capabilities without a GUI.

#### Acceptance Criteria

1. THE CLI SHALL list installed extensions with their status
2. THE CLI SHALL support adding new extensions
3. THE CLI SHALL support removing extensions
4. THE CLI SHALL display extension connection status

**Mapping:** Extend `crates/cli/` with extension management subcommands, leveraging `crates/context_server/` and `crates/extension/`.

### Requirement 4: Markdown Rendering in Terminal

**User Story:** As a sim user, I want agent responses rendered with markdown formatting in the terminal, so that code blocks, lists, and emphasis are visually clear.

#### Acceptance Criteria

1. THE TUI mode SHALL render markdown with ANSI colors and formatting
2. THE TUI mode SHALL support code blocks with syntax highlighting
3. THE TUI mode SHALL support clickable links (if terminal supports it)
4. THE TUI mode SHALL handle tables and other markdown elements

**Mapping:** Use or extend `crates/markdown/` for parsing, with terminal output formatting (ANSI escape codes). Already partially handled in existing agent output.

### Requirement 5: Terminal Onboarding

**User Story:** As a new sim user, I want an onboarding flow in the terminal, so that I can get started without a GUI.

#### Acceptance Criteria

1. THE first launch of the TUI mode SHALL show an onboarding sequence
2. THE onboarding SHALL guide through provider setup
3. THE onboarding SHALL offer to run a tutorial interaction

**Mapping:** New onboarding flow in `crates/cli/`, possibly sharing content with `crates/onboarding/` but rendered for terminal.

### Requirement 6: Slash Commands in TUI

**User Story:** As a sim user, I want slash commands available in the TUI, so that I can quickly invoke recipes, skills, and actions.

#### Acceptance Criteria

1. THE TUI mode SHALL support slash commands (e.g., `/help`, `/recipe`)
2. THE TUI mode SHALL provide command autocomplete or suggestions
3. THE TUI mode SHALL display available commands on `/help`

**Mapping:** Integrate goose's slash command patterns into `crates/cli/` and/or the agent prompt processing.

### Requirement 7: Tool Call Display

**User Story:** As a sim user, I want to see tool calls and results in the TUI, so that I can monitor what the agent is doing.

#### Acceptance Criteria

1. THE TUI mode SHALL display tool calls with their arguments
2. THE TUI mode SHALL display tool results when available
3. THE TUI mode SHALL indicate when tools are executing (spinner/progress)

**Mapping:** Extend sim's existing tool execution display patterns (already present in agent_ui conversation view) to the terminal output.

## Design Approach

```
goose text/ React Feature              →  sim CLI/TUI Equivalent
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
- Existing sim: `crates/cli/` — CLI framework
- Existing sim: `crates/terminal/`, `crates/terminal_view/` — terminal emulator
- Existing sim: `crates/markdown/` — markdown parsing and rendering
- Existing sim: `crates/agent_ui/src/inline_assistant.rs` — inline agent
- Existing sim: `crates/agent_ui/src/terminal_inline_assistant.rs` — terminal inline agent
- Existing sim: `crates/onboarding/` — onboarding views
- Existing sim: `crates/settings/`, `crates/agent_settings/` — settings system
