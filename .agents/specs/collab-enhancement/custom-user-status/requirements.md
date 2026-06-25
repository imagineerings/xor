# Requirements: Custom User Status

## Introduction

Baymax currently shows online/busy presence but lacks custom user status messages — short text labels like "In a meeting", "AFK", "Out sick", or custom text. Mattermost supports custom status with emoji. Adding custom status will let users communicate their availability more precisely.

## Glossary

- **Custom Status**: A short user-set text label and optional emoji displayed on the user's profile
- **Status Preset**: A predefined status option (e.g., "In a meeting", "Out sick", "Working remotely")
- **Clear After**: A timer that automatically clears the custom status after a set duration

## Requirements

### Requirement 8.1: Set Custom Status

**User Story:** As a user, I want to set a custom status with an optional emoji, so that others know my current availability.

#### Acceptance Criteria

1. WHEN a user clicks their avatar or status area THEN THE system SHALL show a "Set a status" option in the menu
2. WHEN the user selects "Set a status" THEN THE system SHALL open a modal or panel with: emoji picker, text input (max 100 chars), "Clear after" dropdown (never, 30min, 1hr, 4hr, today, this week)
3. WHEN the user submits the status THEN THE system SHALL display the emoji + text next to their name in the channel sidebar, message headers, and member lists
4. THE system SHALL provide a set of preset statuses: "In a meeting", "Out sick", "Working remotely", "On vacation", "In a call", "Away", "Busy"
5. WHEN a status is set THEN THE system SHALL persist it on the server and sync to all clients

### Requirement 8.2: Clear/Expire Status

**User Story:** As a user, I want my custom status to automatically clear after a duration, so that I don't have to remember to clear it.

#### Acceptance Criteria

1. WHEN the "Clear after" timer expires THEN THE system SHALL automatically clear the user's status
2. WHEN the user manually clears their status THEN THE system SHALL remove it immediately
3. WHEN the status is cleared (auto or manual) THEN THE system SHALL revert to the default online/offline presence indicator

### Requirement 8.3: Status Display Throughout UI

**User Story:** As a user, I want to see other users' custom status, so that I know what they're doing.

#### Acceptance Criteria

1. THE system SHALL display the custom status emoji + text in: channel member lists, direct message headers, mentions/autocomplete popover, user profile popover
2. WHEN a user has a custom status set THEN THE system SHALL show the status text in a muted/secondary color below their name
3. WHEN the user changes their status THEN THE system SHALL broadcast the change to all connected clients in real-time
