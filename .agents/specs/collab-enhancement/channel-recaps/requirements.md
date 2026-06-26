# Requirements: Channel Recaps/Digests

## Introduction

Baymax channels provide no summary or digest of activity. Mattermost supports recaps (`recaps.yaml` API, `recap.go`) — automated daily summaries of channel activity that help users catch up on missed messages. This is especially valuable for channels with high message volume.

## Glossary

- **Recap**: An automated summary of channel activity over a period (typically daily)
- **Digest**: A collection of recaps from multiple channels delivered as a single notification
- **Recap Period**: The time window covered by a recap (e.g., "last 24 hours")

## Requirements

### Requirement 13.1: Automated Daily Recaps

**User Story:** As a channel participant, I want daily recaps of channel activity, so that I can catch up on what I missed without scrolling through all messages.

#### Acceptance Criteria

1. WHEN a channel has activity in a 24-hour period THEN THE system SHALL generate a recap of that period
2. THE recap SHALL include: total message count, list of active participants, top threads (by reply count), and any flagged/pinned messages
3. THE recap SHALL be generated and available in the channel at a configurable time (default: 8 AM user's local time)
4. THE system SHALL only generate recaps for channels with activity above a configurable threshold (minimum messages)

### Requirement 13.2: Recap Display and Navigation

**User Story:** As a user, I want to easily navigate to recaps, so that I can quickly jump to the content I'm interested in.

#### Acceptance Criteria

1. WHEN a recap is available THEN THE system SHALL display a recap entry in the channel, visually distinct from regular messages
2. WHEN the user opens a recap THEN THE system SHALL expand it to show: message count, participant list, top threads (with links to the first unread message in each thread)
3. WHEN the user clicks a thread link in the recap THEN THE system SHALL navigate to that thread at the position of the first unread message
4. THE system SHALL show a "View recap" button in the channel header or jump-to menu

### Requirement 13.3: Recap Delivery (Optional Notification)

**User Story:** As a user, I want to receive recaps as notifications, so that I'm reminded to catch up.

#### Acceptance Criteria

1. THE system SHALL support optional daily recap notifications (email or in-app)
2. WHEN a user enables recap notifications THEN THE system SHALL deliver a daily summary at the configured time
3. THE notification SHALL include: channel name, message count, and a button/link to open the recap
4. THE system SHALL allow users to opt out of recap notifications per-channel or globally

### Requirement 13.4: Recap Generation Service

**User Story:** As a system administrator, I want recap generation to be efficient, so that it doesn't impact server performance.

#### Acceptance Criteria

1. THE recap generation SHALL run as a scheduled background job on the server (similar to existing job infrastructure)
2. THE job SHALL be configurable: schedule time, timezone, minimum activity threshold
3. THE system SHALL avoid generating recaps for channels with no activity in the period
4. THE system SHALL store recaps as special message types in the database
