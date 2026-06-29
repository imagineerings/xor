# Implementation Plan: Collaboration Features

- [ ] 1. Implement channel data models and ChannelService
  - Channel, ChannelMessage, ChannelMember models
  - REST CRUD: list, join, leave, create, delete, invite, notification prefs
  - _Requirements: 1.1–1.6, 3.1–3.4_
  - _writes: iOS: `Models/Channel.swift`, `Services/ChannelService.swift`; Android: `data/model/Channel.kt`, `data/api/ChannelService.kt`_

- [ ] 2. Implement ChannelBrowser UI
  - Categorized list (Favorites, Channels, DMs) with unread indicators
  - Join/leave, favorite toggle
  - _Requirements: 1.1–1.6_
  - _writes: iOS: `Views/ChannelBrowser.swift`; Android: `ui/screens/ChannelBrowserScreen.kt`_

- [ ] 3. Implement ChannelChatView with real-time messages
  - Load message history, receive live messages via WebSocket
  - Reuse markdown rendering from Agent Chat
  - _Requirements: 2.1–2.5_
  - _writes: iOS: `Views/ChannelChatView.swift`; Android: `ui/screens/ChannelChatScreen.kt`_

- [ ] 4. Implement contact list with presence
  - Contact list with online/offline indicators
  - Contact requests (send, accept, decline)
  - Real-time presence via WebSocket
  - _Requirements: 4.1–4.4_
  - _writes: iOS: `Views/ContactListView.swift`, `Services/PresenceService.swift`; Android: `ui/screens/ContactListScreen.kt`, `data/repository/PresenceService.kt`_

- [ ] 5. Implement shared document viewer
  - Read-only document view with real-time updates
  - Notes tab alongside chat tab for channels with buffers
  - _Requirements: 5.1–5.3_
  - _writes: iOS: `Views/SharedDocumentView.swift`; Android: `ui/screens/SharedDocumentScreen.kt`_

- [ ] 6. Implement project sharing notification and accept flow
  - Notification display, accept → browse file tree, view files
  - _Requirements: 6.1–6.4_
  - _writes: iOS: `Views/ProjectShareHandler.swift`; Android: `ui/screens/ProjectShareScreen.kt`_

- [ ] 7. Implement agent thread sharing
  - Share to channel picker, post thread summary, open shared thread
  - _Requirements: 7.1–7.3_
  - _writes: iOS: `Services/ThreadShareService.swift`; Android: `data/repository/ThreadShareService.kt`_
