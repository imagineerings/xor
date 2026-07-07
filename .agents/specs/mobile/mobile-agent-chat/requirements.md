# Requirements: Agent Chat Interface

## Introduction

The core experience of the Sim mobile app is chatting with the AI agent. Users need a rich, responsive chat interface that displays streaming responses with markdown, code blocks, tool calls, and supports message actions, threading, and search. This spec builds on the existing chat implementations (iOS: `ChatView.swift`, Android: `ChatScreen.kt`) and extends them with features from the `mobile-dev` messaging app.

## Glossary

| Term | Definition |
|------|------------|
| **Token** | An incremental chunk of text emitted by the agent during SSE streaming. Multiple tokens are concatenated to form the full response. |
| **Tool Call** | A structured request from the agent to invoke a tool (e.g., search files, read code). Displayed as a card in the chat. |
| **Tool Result** | The output returned after a tool executes. Replaces the tool call card's loading state. |
| **Slash Command** | A `/`-prefixed command entered in the input field that invokes an agent tool or built-in function. |
| **Thread** | A branched sub-conversation within a session, initiated from a specific message. |
| **Syntax Highlighting** | Color-coded display of code blocks based on programming language. |

## Requirements

### Requirement 1: Message Display & Streaming

**User Story:** As a mobile user, I want to see the agent's response stream in real-time, so I can read the response as it's being generated.

1.1 WHEN the user sends a message THEN THE app SHALL display it immediately as a "user" message bubble in the chat.

1.2 WHEN the SSE stream delivers a `token` event THEN THE app SHALL append the token content to the current assistant message, rendering it incrementally.

1.3 WHEN the assistant message contains markdown THEN THE app SHALL render it: bold, italic, lists, links, headings, blockquotes, inline code, horizontal rules.

1.4 WHEN the assistant message contains a code block (triple-backtick) THEN THE app SHALL render it with syntax highlighting and a language label.

1.5 WHEN the SSE stream delivers a `toolCall` event THEN THE app SHALL display a tool call card with the tool name, arguments (collapsible), and a loading indicator.

1.6 WHEN the SSE stream delivers a `toolResult` event THEN THE app SHALL update the corresponding tool call card with the result status (completed/failed) and output (collapsible).

1.7 WHEN the SSE stream delivers an `error` event THEN THE app SHALL display an inline error in the assistant message area.

1.8 WHEN the SSE stream delivers an `endStream` event THEN THE app SHALL stop the loading indicator and finalize the message.

1.9 IF the SSE stream is interrupted (network loss) THEN THE app SHALL show a "Reconnecting..." indicator on the last assistant message.

### Requirement 2: Message Input

**User Story:** As a mobile user, I want a capable text input with formatting and attachments, so I can communicate effectively with the agent.

2.1 THE app SHALL provide a multi-line text input field that auto-expands as the user types.

2.2 THE app SHALL support sending messages via the keyboard's return/send key or a dedicated send button.

2.3 THE app SHALL support pasting text and images into the input field.

2.4 THE app SHALL support **slash commands** — when the user types `/` in an empty input, show a command picker with available tools.

2.5 WHEN the user selects a slash command THEN THE app SHALL insert the command template into the input.

2.6 THE app SHALL support **voice input** via a microphone button (reuse existing `VoiceInputManager` / `VoiceManager`).

2.7 THE app SHALL show a typing indicator while the agent is generating a response.

### Requirement 3: Message Actions

**User Story:** As a mobile user, I want to interact with individual messages (copy, edit, etc.) to manage the conversation.

3.1 WHEN the user long-presses a message THEN THE app SHALL show a context menu with actions.

3.2 THE context menu SHALL include:
   - Copy text (with "Copy code" option for code blocks)
   - Select text (native selection)
   - Share (iOS Share Sheet / Android Share Intent)
   - (For user messages) Edit — re-send modified message
   - (For assistant messages) Copy entire response

3.3 THE app SHALL support **copying code blocks** with a dedicated "Copy" button on each code block.

### Requirement 4: Conversation Threads

**User Story:** As a mobile user, I want to branch off from a specific message into a thread, so I can ask follow-up questions without cluttering the main conversation.

4.1 WHEN the user selects "Thread" from a message's context menu THEN THE app SHALL open a thread view scoped to that message.

4.2 THE thread view SHALL show the parent message at the top and all replies below it.

4.3 WHEN the user sends a message in a thread THEN THE app SHALL send it in the context of the parent message.

4.4 THE thread view SHALL have a visual indicator showing it's a sub-conversation.

### Requirement 5: Search

**User Story:** As a mobile user, I want to search across all my conversations, so I can quickly find past information.

5.1 THE app SHALL provide a search entry point (search icon in navigation bar).

5.2 WHEN the user enters a search query THEN THE app SHALL search across all session titles and message content.

5.3 THE search results SHALL be grouped by session, showing matching message snippets with highlighted terms.

5.4 WHEN the user taps a search result THEN THE app SHALL open the session at the relevant message.

5.5 THE app SHALL debounce search input (300ms) to avoid excessive API calls.

## Existing Assets

- iOS: `ChatView.swift`, `ChatInputView.swift`, `UserMessageView.swift`, `AssistantMessageView.swift`, `MarkdownTableView.swift`, `SharedMessageComponents.swift`, `ToolViews.swift`, `StackedToolCallsView.swift`
- Android: `ChatScreen.kt`, `ChatViewModel.kt`, `ChatInputView.kt`, `MessageBubble.kt`, `MarkdownText.kt`, `ToolCallCard.kt`, `StackedToolCallsView.kt`, `SyntaxHighlighter.kt`
- mobile-dev: `app/components/markdown/`, `app/components/formatted_markdown_text.tsx`, `app/components/syntax_highlight/`, `app/components/post_draft/`, `app/screens/thread/`, `app/screens/search/`
