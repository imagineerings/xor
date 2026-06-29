# Implementation Plan: Settings & Configuration

- [ ] 1. Extend server configuration section
  - URL/secret fields, connection status with provenance, test button, server version
  - Saved agents list with add/edit/switch/delete
  - QR code scanning
  - _Requirements: 1.1–1.7_
  - _writes: iOS: `Views/SettingsView.swift` (extend); Android: `ui/screens/SettingsScreen.kt` (extend)_

- [ ] 2. Implement display settings section
  - Theme (Light/Dark/System), Font Size, Clock Format, Timezone, CRT Mode
  - _Requirements: 2.1–2.3_
  - _writes: iOS: `Views/DisplaySettingsView.swift`; Android: `ui/screens/DisplaySettingsScreen.kt`_

- [ ] 3. Implement notification settings section
  - Toggles per category, system settings link
  - _Requirements: 3.1, 3.2_
  - _writes: iOS: `Views/NotificationSettingsView.swift`; Android: `ui/screens/NotificationSettingsScreen.kt`_

- [ ] 4. Implement security settings section
  - Biometric lock toggle (hidden if unavailable)
  - Clear all agents option
  - _Requirements: 4.1–4.4_
  - _writes: iOS: `Views/SecuritySettingsView.swift`; Android: `ui/screens/SecuritySettingsScreen.kt`_

- [ ] 5. Implement about section
  - Version, build, license, report problem link
  - _Requirements: 5.1–5.3_
  - _writes: iOS: `Views/AboutView.swift`; Android: `ui/screens/AboutScreen.kt`_

- [ ] 6. Implement reset and data management actions
  - Reset to Trial Mode, Clear All Data — both with confirmation
  - _Requirements: 6.1–6.3_
  - _writes: iOS: `Views/DataManagementView.swift`; Android: `ui/screens/DataManagementScreen.kt`_
