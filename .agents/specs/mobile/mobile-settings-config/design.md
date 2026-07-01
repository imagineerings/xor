# Design: Settings & Configuration

## 1. Overview

The settings system extends the existing `SettingsView.swift` / `SettingsScreen.kt` with organized sections for server, display, notifications, security, and about. The architecture uses a settings store (UserDefaults/DataStore) with platform-appropriate UI components.

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Settings organization | Grouped table (iOS: Form/List sections, Android: PreferenceScreen) | Platform-standard settings UX |
| Server config | Reuse existing AgentStorage/CredentialManager | Already implemented Group 1 |
| Destructive actions | Confirmation alerts required | Safety for data loss |

## 2. Tasks

- [ ] 1. Server configuration section (extend existing)
- [ ] 2. Display settings section (theme, font, clock, timezone, CRT)
- [ ] 3. Notification settings section (toggles, system settings link)
- [ ] 4. Security settings section (biometric lock, clear data)
- [ ] 5. About section (version, license, report problem)
- [ ] 6. Reset and data management actions
