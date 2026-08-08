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

1. **1.1** WHEN a user types a slash command THEN the system SHALL parse and execute the corresponding handler
2. **1.2** THE slash command system SHALL support recipe commands to run recipes
3. **1.3** THE slash command system SHALL support skill commands to load skills
4. **1.4** THE slash command system SHALL support extensible custom slash commands
5. **1.5** IF an unknown slash command is entered THEN the system SHALL show available commands

### Requirement 2: Hints System

**User Story:** As a sim user, I want the agent to automatically load relevant context about my project, so that it has better context without manual setup.

#### Acceptance Criteria

1. **2.1** THE hints system SHALL load hints from configured sources (files, directories)
2. **2.2** WHEN a project is opened THEN the system SHALL load project-specific hints
3. **2.3** WHEN a hint file matches the current context THEN the system SHALL include it in the agent's prompt
4. **2.4** THE hints SHALL support importing content from referenced files

### Requirement 3: MCP Apps and Embedded App Resources

**User Story:** As a sim user, I want approved MCP-provided interactive resources to render inside the agent, so that I can use extension interfaces without leaving the conversation.

#### Acceptance Criteria

1. **3.1** WHERE embedded MCP Apps are approved, THE existing context-server and agent UI integration SHALL discover and render MCP app resources without creating a second extension registry
2. **3.2** THE renderer SHALL isolate untrusted HTML, scripts, origins, navigation, downloads, clipboard access, and tool calls according to an explicit security policy
3. **3.3** THE bridge SHALL expose only the MCP Apps protocol operations and tools authorized for the owning server and session
4. **3.4** THE system SHALL cache and retire app resources with explicit server, session, version, and lifetime boundaries
5. **3.5** IF a resource, renderer, bridge, or server fails, THEN THE conversation SHALL remain usable and SHALL show a diagnostic without executing partial privileged actions

### Requirement 4: Source Roots and Sources

**User Story:** As a sim user, I want the agent to know about project source roots and named sources, so that it can reference the right code and content.

#### Acceptance Criteria

1. **4.1** THE system SHALL support defining source roots (directory roots for code/ content)
2. **4.2** THE system SHALL support defining named sources that reference files or directories
3. **4.3** WHEN the agent references a source THEN the system SHALL resolve it to the actual path

### Requirement 5: Execution Manager

**User Story:** As a sim user, I want concurrent requests for an agent session to share initialization and lifecycle state, so that provider and extension startup is consistent and cancellable.

#### Acceptance Criteria

1. **5.1** THE existing agent and thread store SHALL coalesce concurrent initialization for the same session rather than creating duplicate providers or extensions
2. **5.2** THE lifecycle SHALL restore evicted session, provider, and extension state before accepting work and SHALL support cancellation during restoration
3. **5.3** WHEN initialization or restoration completes THEN all waiting callers SHALL observe the same resulting session state
4. **5.4** IF initialization, restoration, or cancellation fails THEN every waiting caller SHALL receive a consistent error and incomplete resources SHALL be cleaned up

## References

- Source: `projects/goose/crates/goose/src/slash_commands/` — mod.rs, slash_command.rs, recipe_slash_command.rs, skill_slash_command.rs, types.rs, util.rs
- Source: `projects/goose/crates/goose/src/hints/` — mod.rs, import_files.rs, load_hints.rs
- Source: `projects/goose/crates/goose/src/goose_apps/` — mod.rs, app.rs, cache.rs, chat.html, clock.html, resource.rs
- Source: `projects/goose/crates/goose/src/source_roots.rs`, `sources.rs`
- Source: `projects/goose/crates/goose/src/execution/` — mod.rs, manager.rs
