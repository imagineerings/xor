# Implementation Plan: Agent Chat Interface

## Overview

Tasks build the chat interface from the message model up through rendering components and input. The first tasks establish data models and state management (the foundation), then rendering components are built individually, and finally everything is wired together.

- [ ] 1. Implement Message and ToolCall data models
  - Define `Message`, `MessageRole`, `ToolCall`, `ToolCallStatus`
  - _Requirements: 1.1–1.8_
  - _writes: iOS: `Models/Message.swift`; Android: `data/model/Message.kt`_

- [ ] 2. Implement ChatViewModel with message state management
  - Message list management (append, update, replace, streaming state)
  - Send message flow → SSE → token append
  - _Requirements: 1.1–1.9, 2.7_
  - _writes: iOS: `ViewModels/ChatViewModel.swift`; Android: `ui/screens/ChatViewModel.kt` (extend)_

- [ ] 3. Implement markdown rendering component
  - Render bold, italic, lists, links, headings, blockquotes, code, inline code
  - _Requirements: 1.3, 1.4_
  - _writes: iOS: `Components/MarkdownRenderer.swift`; Android: `ui/components/MarkdownText.kt` (extend)_

- [ ] 4. Implement syntax highlighting for code blocks
  - Language label, highlighted code, copy button
  - _Requirements: 1.4_
  - _writes: iOS: `Components/CodeBlockView.swift`; Android: `ui/components/SyntaxHighlighter.kt` (extend)_

- [ ] 5. Implement ToolCallCard component
  - Three states: running, completed, failed — with collapsible sections
  - _Requirements: 1.5, 1.6_
  - _writes: iOS: `Components/ToolCallCard.swift`; Android: `ui/components/ToolCallCard.kt` (extend)_

- [ ] 6. Implement chat input bar
  - Auto-expanding text field, send button, attachment button, mic button
  - Slash command autocomplete on `/`
  - _Requirements: 2.1–2.6_
  - _writes: iOS: `Components/ChatInputBar.swift`; Android: `ui/components/ChatInputView.kt` (extend)_

- [ ] 7. Implement message actions (context menu)
  - Long-press: Copy, Share, Edit, Copy Response
  - Code block copy button
  - _Requirements: 3.1–3.3_
  - _writes: iOS: `Components/MessageContextMenu.swift`; Android: `ui/components/MessageActions.kt`_

- [ ] 8. Implement thread view
  - Branch from a message, show parent + replies, support sending thread messages
  - _Requirements: 4.1–4.4_
  - _writes: iOS: `Views/ThreadView.swift`; Android: `ui/screens/ThreadScreen.kt`_

- [ ] 9. Implement search
  - Search bar, debounced query, results grouped by session with highlighted snippets
  - Tap result → open session at message
  - _Requirements: 5.1–5.5_
  - _writes: iOS: `Views/SearchView.swift`; Android: `ui/screens/SearchScreen.kt`_

- [ ] 10. Wire everything together in ChatView
  - Connect all rendering components, input bar, streaming to ViewModel
  - Handle stream interruption and reconnect indicator
  - _Requirements: 1.1–1.9, 2.1–2.7_
  - _writes: iOS: `ChatView.swift` (modify); Android: `ui/screens/ChatScreen.kt` (modify)_
