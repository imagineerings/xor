# Implementation Plan: Notifications

- [ ] 1. Implement push notification registration and token management
  - Request permission on first launch, register token with push proxy
  - Unregister on disconnect, re-register on reconnect
  - _Requirements: 1.5, 1.6_
  - _writes: iOS: `Services/PushNotificationManager.swift`; Android: `data/repository/PushNotificationManager.kt`_

- [ ] 2. Implement remote push handling
  - Parse push payload (agent response, channel message, call)
  - On tap: deep link to session, channel, or call UI
  - _Requirements: 1.1–1.4_
  - _writes: iOS: `BaymaxApp.swift` (modify for UNUserNotificationCenter); Android: `BaymaxApplication.kt` (modify for FCM)_

- [ ] 3. Implement in-app notification toasts and overlays
  - Toast for channel messages, overlay for incoming calls
  - Auto-dismiss (5s) for toasts, persistent for calls
  - _Requirements: 2.1–2.4_
  - _writes: iOS: `Components/InAppNotification.swift`; Android: `ui/components/InAppNotification.kt`_

- [ ] 4. Implement notification preferences settings screen
  - Toggles per category, per-channel override, quiet hours
  - _Requirements: 3.1–3.3_
  - _writes: iOS: `Views/NotificationSettingsView.swift`; Android: `ui/screens/NotificationSettingsScreen.kt`_

- [ ] 5. Implement badge management
  - Update badge count from unread notifications
  - Decrement on read or app open
  - _Requirements: 4.1, 4.2_
  - _writes: iOS: `Services/BadgeManager.swift`; Android: `data/repository/BadgeManager.kt`_
