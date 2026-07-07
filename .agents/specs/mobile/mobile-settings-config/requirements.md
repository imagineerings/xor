# Requirements: Settings & Configuration

## Introduction

The Sim mobile client needs a comprehensive settings system covering app configuration, server management, and security. The iOS app already has a basic `SettingsView.swift` with server URL/secret configuration. Android has `SettingsScreen.kt` and `SettingsViewModel.kt`. This spec extends them with advanced settings from `mobile-dev` including display customization, notification management, about/licensing, and security controls.

## Glossary

| Term | Definition |
|------|------------|
| **Server Configuration** | The URL and secret key for connecting to a Sim agent, plus any collab server settings. |
| **Multiple Servers** | The ability to save, switch, and manage multiple agent configurations. |
| **Biometric Lock** | App-level security using Face ID / Fingerprint to prevent unauthorized access. |
| **Display Settings** | Visual customization: theme, font size, time format, CRT mode. |
| **Notification Settings** | Control over which push and in-app notifications are delivered. |

## Requirements

### Requirement 1: Server Configuration

**User Story:** As a mobile user, I want to configure and manage my server connections easily.

1.1 THE app SHALL provide a settings screen with fields for: Server URL, Secret Key.

1.2 THE app SHALL show connection status (connected/disconnected with provenance) and a "Test Connection" button.

1.3 THE app SHALL show the server version string when connected.

1.4 THE app SHALL support saving the current configuration as a named agent for later reuse.

1.5 THE app SHALL display the list of saved agents with the current one highlighted.

1.6 WHEN the user switches agents THEN THE app SHALL change the active connection and navigate to the session list.

1.7 THE app SHALL support scanning a QR code to configure the server.

### Requirement 2: Display Settings

**User Story:** As a mobile user, I want to customize the app's appearance to my preference.

2.1 THE app SHALL provide a display settings section with: Theme (Light/Dark/System), Font Size (Small/Medium/Large).

2.2 THE app SHALL support Clock Format (12h/24h) and Timezone selection.

2.3 THE app SHALL support CRT Mode toggle (scanline effect on code blocks).

### Requirement 3: Notification Settings

**User Story:** As a mobile user, I want to control what notifications I receive.

3.1 THE app SHALL provide a notification settings section with toggles for: Agent Responses, Channel Messages, Calls, Sounds.

3.2 THE app SHALL show the current push notification permission status and a link to system settings to change it.

### Requirement 4: Security Settings

**User Story:** As a mobile user, I want to protect the app with biometric authentication.

4.1 THE app SHALL provide a "Require Face ID / Fingerprint" toggle in settings.

4.2 THE toggle SHALL only appear if biometric hardware is available on the device.

4.3 WHEN enabled, the app SHALL lock after 5 minutes in the background and require biometrics to unlock.

4.4 THE app SHALL provide a "Clear all saved agents" option for secure device handoff.

### Requirement 5: About & Legal

**User Story:** As a mobile user, I want to see app version information and legal notices.

5.1 THE app SHALL show an "About" section with: App version, build number, and license information.

5.2 THE app SHALL show a "Report a Problem" option (opens support URL or email).

5.3 THE app SHALL link to the open source repository and contribution guidelines.

### Requirement 6: Reset & Data Management

**User Story:** As a mobile user, I want to reset the app to its initial state.

6.1 THE app SHALL provide a "Reset to Trial Mode" option that clears all saved agents and connects to the demo server.

6.2 THE app SHALL provide a "Clear All Data" option that removes all saved agents, credentials, and cached sessions.

6.3 All destructive actions SHALL require confirmation before executing.

## Existing Assets

- iOS: `SettingsView.swift`, `ConfigurationHandler.swift`, `AgentStorage`, `TrialMode.swift`
- Android: `SettingsScreen.kt`, `SettingsViewModel.kt`, `SettingsRepository.kt`, `TrialModeManager.kt`, `TrialModeInstructionsScreen.kt`
- mobile-dev: `app/screens/settings/` (display, notification, advanced, about sub-screens), `app/screens/edit_server/`, `app/constants/about_links.ts`
