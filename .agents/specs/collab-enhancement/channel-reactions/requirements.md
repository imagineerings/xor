# Requirements: Emoji Reactions on Channel Messages

## Introduction

Baymax channel messages currently lack emoji reactions — a core affordance of modern chat that allows lightweight, non-verbal responses without adding reply noise. Mattermost supports emoji reactions on posts (via the `reactions.yaml` API). Adding reactions will reduce message clutter and enable quick sentiment/acknowledgement in channels.

## Glossary

- **Reaction**: An emoji attached to a channel message by a user
- **Reaction Picker**: UI component for selecting an emoji to react with
- **Reaction Bar**: The row of emoji+count displays below a message

## Requirements

### Requirement 2.1: Add/Remove Emoji Reactions

**User Story:** As a channel participant, I want to react to a message with an emoji, so that I can quickly acknowledge or respond without sending a separate message.

#### Acceptance Criteria

1. WHEN a user hovers over a channel message THEN THE system SHALL display a "+" reaction button near the message
2. WHEN the user clicks the reaction button THEN THE system SHALL open an emoji picker
3. WHEN the user selects an emoji THEN THE system SHALL add that reaction to the message on behalf of the user
4. WHEN the user clicks an existing reaction they already added THEN THE system SHALL remove their reaction
5. WHEN the user clicks an existing reaction they did NOT add THEN THE system SHALL add their reaction (joining it)
6. THE system SHALL support adding reactions to any channel message

### Requirement 2.2: Reaction Display

**User Story:** As a channel participant, I want to see which reactions a message has received and who reacted, so that I can understand the sentiment at a glance.

#### Acceptance Criteria

1. WHEN a message has reactions THEN THE system SHALL display a reaction bar showing each unique emoji with a count of how many users reacted
2. WHEN the user hovers over a reaction in the reaction bar THEN THE system SHALL show a tooltip listing the names of users who reacted with that emoji
3. WHEN a user adds or removes a reaction THEN THE system SHALL update the reaction bar in real-time for all participants (via WebSocket)

### Requirement 2.3: Emoji Picker

**User Story:** As a channel participant, I want a searchable emoji picker, so that I can quickly find the emoji I want.

#### Acceptance Criteria

1. WHEN the reaction picker opens THEN THE system SHALL display a grid of frequently used and recently used emojis at the top
2. THE system SHALL support searching emojis by name and keyword
3. THE system SHALL support skin tone variations for applicable emojis
4. THE system SHALL display custom server emojis if available

### Requirement 2.4: Reactions Persistence and Sync

**User Story:** As a user, I want my reactions to persist across sessions and sync to all my devices.

#### Acceptance Criteria

1. WHEN a user adds a reaction THEN THE system SHALL persist it to the server
2. WHEN a user reconnects or opens a channel THEN THE system SHALL load all existing reactions for visible messages
3. WHEN a message is deleted THEN THE system SHALL delete all associated reactions
