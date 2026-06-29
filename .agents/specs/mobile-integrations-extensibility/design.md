# Design: Integrations & Extensibility

## 1. Overview

Integrations extend the agent's capabilities with tools, slash commands, and external service connections. The architecture uses the agent's tool discovery API to populate available tools, and the existing SSE streaming to handle tool calls inline.

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Tool discovery | GET /tools from agent API | Dynamic — agent controls available tools |
| Slash commands | Local autocomplete list derived from tool list | Fast UX, no network request per keystroke |
| Tool visualization | Reuse ToolCallCard from Agent Chat | Consistent UX, shared component |

## 2. Tasks

- [ ] 1. Tool browser (fetch tools list, display with descriptions)
- [ ] 2. Slash command autocomplete (type `/` → filtered list → insert template)
- [ ] 3. Tool call visualization in chat (reuse ToolCallCard)
- [ ] 4. Integration management screen (view connected integrations)
