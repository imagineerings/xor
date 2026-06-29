# Requirements: Integrations & Extensibility

## Introduction

The Baymax mobile client needs to support tools, slash commands, and integrations that extend the agent's capabilities. Users should be able to browse available tools, invoke them via slash commands, and manage tool configurations. This spec draws from the `mobile-dev` integrations framework (apps, slash commands, webhooks) and Baymax's agent tool system (`acp_tools`, `agent` crates).

## Glossary

| Term | Definition |
|------|------------|
| **Slash Command** | A `/`-prefixed shortcut in the chat input that invokes a specific tool or action (e.g., `/search`, `/run`). |
| **Tool** | A capability the agent can use, such as reading files, searching code, running commands, or accessing external APIs. |
| **ACP (Agent Client Protocol)** | The protocol for agent-to-tool communication. Tools are discovered via the agent's tool list API. |
| **Context Server** | An MCP (Model Context Protocol) server that provides additional context/tools to the agent. |
| **Integration** | An external service connected to Baymax (e.g., GitHub, Slack, Jira) via webhooks or OAuth. |

## Requirements

### Requirement 1: Tool Browser

**User Story:** As a mobile user, I want to see what tools my agent has available and understand what each one does.

1.1 THE app SHALL provide a tool browser accessible from settings or the chat input area.

1.2 WHEN the user opens the tool browser THEN THE app SHALL fetch the available tools from the agent API.

1.3 THE tool browser SHALL display: tool name, description, and required parameters.

1.4 WHEN the user taps a tool THEN THE app SHALL show detailed information and an option to invoke it directly.

### Requirement 2: Slash Commands

**User Story:** As a mobile user, I want to invoke agent tools quickly using slash commands.

2.1 WHEN the user types `/` in an empty chat input THEN THE app SHALL show an autocomplete list of available slash commands.

2.2 THE slash command list SHALL filter as the user types more characters after `/`.

2.3 WHEN the user selects a slash command THEN THE app SHALL insert the command template into the input (e.g., `/search `).

2.4 WHEN the user sends a slash command message THEN THE agent SHALL execute the corresponding tool.

### Requirement 3: Tool Call Visualization

**User Story:** As a mobile user, I want to see tool calls and their results inline in the conversation.

3.1 WHEN the agent invokes a tool during a conversation THEN THE app SHALL display a tool call card showing: tool name, input arguments (collapsible), status (running/completed/failed).

3.2 WHEN the tool completes successfully THEN THE app SHALL update the card to show the result output (collapsible).

3.3 WHEN the tool fails THEN THE app SHALL update the card to show the error in red.

3.4 THE app SHALL support expanding/collapsing tool call details (arguments and results).

### Requirement 4: Integration Management

**User Story:** As a mobile user, I want to see and manage connected integrations.

4.1 WHERE the agent supports external integrations (GitHub, Slack, etc.) THEN THE app SHALL show them in a settings section.

4.2 THE integrations list SHALL show: integration name, connection status (connected/disconnected), and last used timestamp.

4.3 WHEN the user taps an integration THEN THE app SHALL show details and an option to disconnect.

## Existing Assets

- iOS: `ToolViews.swift`, `StackedToolCallsView.swift`, `TaskDetailView.swift`
- Android: `ToolCallCard.kt`, `StackedToolCallsView.kt`, `ToolCallDetailScreen.kt`, `TaskDetailScreen.kt`
- mobile-dev: `app/managers/apps_manager.ts`, `app/managers/integrations_manager.ts`, `app/screens/integration_selector/`, `app/screens/interactive_dialog/`, `app/constants/integrations.ts`
- Baymax desktop: `crates/agent/` (agent + tools), `crates/acp_tools/` (tool protocol), `crates/context_server/` (MCP server)
