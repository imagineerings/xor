# Requirements: Message Threading in Channels

## Introduction

Sim channel messages currently exist as a flat list. There is `reply_to_message_id` in `SendChannelMessage` proto, but no dedicated thread view. Mattermost's threading model allows replies to be viewed in a dedicated thread panel (RHS), keeping the main channel clean while preserving context. Adding proper threading will reduce channel noise and organize conversations.

## Glossary

- **Thread**: A collection of replies to a root message
- **Root Message**: The original message that starts a thread
- **Reply**: A message sent in response to a root message, tracked via `reply_to_message_id`
- **Thread Panel**: A dedicated view showing all replies in a thread (typically in the right sidebar)

## Requirements

### Requirement 3.1: Reply to Messages with Threading

**User Story:** As a channel participant, I want to reply to a specific message and see replies grouped together, so that conversations stay organized.

#### Acceptance Criteria

1. WHEN a user sends a message with `reply_to_message_id` set THEN THE system SHALL classify it as a reply to the root message
2. WHEN a message has one or more replies THEN THE system SHALL display a reply count indicator below the root message (e.g., "3 replies")
3. WHEN the user clicks the reply count indicator THEN THE system SHALL open a thread panel showing all replies
4. THE system SHALL display replies chronologically within the thread

### Requirement 3.2: Thread Panel

**User Story:** As a channel participant, I want a dedicated thread view, so that I can follow a conversation without scrolling through the main channel.

#### Acceptance Criteria

1. WHEN a thread panel is open THEN THE system SHALL display the root message at the top, followed by all replies in chronological order
2. WHEN the user sends a reply from within the thread panel THEN THE message SHALL be sent with the correct `reply_to_message_id` and appear immediately in the thread
3. WHEN a new reply arrives via WebSocket WHILE the thread panel is open THEN THE system SHALL append it to the thread in real-time
4. THE thread panel SHALL have a text input at the bottom for composing replies
5. THE thread panel SHALL show the channel name as context

### Requirement 3.3: Thread Indicators in Channel

**User Story:** As a channel participant, I want clear visual indicators when messages have threads, so that I know which conversations have activity.

#### Acceptance Criteria

1. WHEN a root message has unread replies THEN THE system SHALL display an unread indicator (e.g., blue dot) on the reply count
2. WHEN the user has participated in a thread THEN THE system SHALL display a participant indicator (e.g., user avatar overlay on the reply count)
3. WHEN all replies in a thread have been read THEN THE system SHALL clear the unread indicator

### Requirement 3.4: Thread Navigation

**User Story:** As a user, I want to navigate between threads, so that I can catch up on multiple conversations.

#### Acceptance Criteria

1. WHEN the user clicks a different message's reply count WHILE a thread panel is open THEN THE system SHALL switch the thread panel to show the new thread
2. THE system SHALL support keyboard shortcuts for closing the thread panel (Escape)
3. WHEN the thread panel is closed AND the user clicks another reply count THEN THE system SHALL reopen the thread panel with the new thread
