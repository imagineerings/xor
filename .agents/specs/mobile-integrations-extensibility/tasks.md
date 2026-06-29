# Implementation Plan: Integrations & Extensibility

- [ ] 1. Implement tool browser
  - Fetch available tools from agent API (`GET /tools`)
  - Display tool name, description, parameters
  - Detail view on tap
  - _Requirements: 1.1–1.4_
  - _writes: iOS: `Views/ToolBrowserView.swift`, `Services/ToolService.swift`; Android: `ui/screens/ToolBrowserScreen.kt`, `data/repository/ToolService.kt`_

- [ ] 2. Implement slash command autocomplete
  - Listen for `/` in input → show filtered command list
  - On select → insert command template
  - Populate from tool list (cached)
  - _Requirements: 2.1–2.4_
  - _writes: iOS: `Components/SlashCommandPicker.swift`; Android: `ui/components/SlashCommandPicker.kt`_

- [ ] 3. Reuse ToolCallCard for inline tool visualization
  - Loading, completed, failed states with collapsible sections
  - Already implemented in mobile-agent-chat — wire into chat view
  - _Requirements: 3.1–3.4_
  - _writes: (no new files — integrate existing ToolCallCard into ChatView)_

- [ ] 4. Implement integration management screen
  - List connected integrations with status
  - Detail view and disconnect option
  - _Requirements: 4.1–4.3_
  - _writes: iOS: `Views/IntegrationListView.swift`; Android: `ui/screens/IntegrationListScreen.kt`_
