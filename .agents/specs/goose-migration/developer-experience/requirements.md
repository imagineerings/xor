# Requirements: Developer Experience

## Introduction

Migrate several goose developer-experience features: slash commands, hints system, goose apps (embedded mini-apps), source roots/sources management, and the execution manager. These features enhance the user's interaction with the agent.

## Glossary

- **Slash Command**: A text command starting with `/` that the agent can interpret (e.g., `/help`, `/recipe`)
- **Hint**: Contextual information loaded into the agent's prompt based on the current project or task
- **Sim App**: An embedded mini-application within sim (chat app, clock app, etc.)
- **Source Root**: A directory root for source code or content that the agent can reference
- **Source**: A named source of information for the agent (files, directories, snippets)
- **Execution Manager**: Coordinates task execution and manages running processes

## Requirements

### Requirement 1: Slash Command System

**User Story:** As a sim user, I want to use slash commands to quickly invoke common actions, so that I can interact with the agent efficiently.

#### Acceptance Criteria

1. WHEN a user types a slash command THEN the system SHALL parse and execute the corresponding handler
2. THE slash command system SHALL support recipe commands to run recipes
3. THE slash command system SHALL support skill commands to load skills
4. THE slash command system SHALL support extensible custom slash commands
5. IF an unknown slash command is entered THEN the system SHALL show available commands

### Requirement 2: Hints System

**User Story:** As a sim user, I want the agent to automatically load relevant context about my project, so that it has better context without manual setup.

#### Acceptance Criteria

1. THE hints system SHALL load hints from configured sources (files, directories)
2. WHEN a project is opened THEN the system SHALL load project-specific hints
3. WHEN a hint file matches the current context THEN the system SHALL include it in the agent's prompt
4. THE hints SHALL support importing content from referenced files

### Requirement 3: Sim Apps

**User Story:** As a sim user, I want embedded mini-applications within the agent (chat, clock, etc.), so that I can access utility functions without leaving the agent interface.

#### Acceptance Criteria

1. THE app system SHALL support registering and launching embedded apps
2. THE app system SHALL include a chat app for messaging-like interactions
3. THE app system SHALL include a clock app for time-related functions
4. THE app system SHALL support resource management for app data
5. THE app system SHALL support caching for app state

### Requirement 4: Source Roots and Sources

**User Story:** As a sim user, I want the agent to know about project source roots and named sources, so that it can reference the right code and content.

#### Acceptance Criteria

1. THE system SHALL support defining source roots (directory roots for code/ content)
2. THE system SHALL support defining named sources that reference files or directories
3. WHEN the agent references a source THEN the system SHALL resolve it to the actual path

### Requirement 5: Execution Manager

**User Story:** As a sim user, I want the system to manage concurrent task execution, so that multiple agent operations can be coordinated.

#### Acceptance Criteria

1. THE execution manager SHALL track running tasks and their state
2. THE execution manager SHALL support starting, monitoring, and stopping tasks
3. WHEN a task completes THEN the execution manager SHALL report the result
4. IF a task fails THEN the execution manager SHALL report the error

## References

- Source: `projects/goose/crates/goose/src/slash_commands/` — mod.rs, slash_command.rs, recipe_slash_command.rs, skill_slash_command.rs, types.rs, util.rs
- Source: `projects/goose/crates/goose/src/hints/` — mod.rs, import_files.rs, load_hints.rs
- Source: `projects/goose/crates/goose/src/goose_apps/` — mod.rs, app.rs, cache.rs, chat.html, clock.html, resource.rs
- Source: `projects/goose/crates/goose/src/source_roots.rs`, `sources.rs`
- Source: `projects/goose/crates/goose/src/execution/` — mod.rs, manager.rs
