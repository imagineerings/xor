# Requirements: Message Priorities

## Introduction

Sim channel messages have no urgency indicators. Mattermost supports message priorities (`post_priority.go`) — users can mark messages as "Urgent" or "Important" with visual indicators and special notification handling. This helps critical communications stand out in busy channels.

## Glossary

- **Message Priority**: A flag on a channel message indicating its urgency level
- **Urgent Message**: A message requiring immediate attention (triggers persistent notification)
- **Important Message**: A message marked as noteworthy (visual indicator only)

## Requirements

### Requirement 12.1: Set Message Priority

**User Story:** As a channel participant, I want to mark important or urgent messages, so that recipients understand the message's significance.

#### Acceptance Criteria

1. WHEN composing a message THEN THE system SHALL provide priority selection: Normal (default), Important, or Urgent
2. WHEN the priority is set to Important THEN THE system SHALL display a visual indicator (e.g., yellow/orange icon) next to the message
3. WHEN the priority is set to Urgent THEN THE system SHALL display a prominent visual indicator (e.g., red icon) AND trigger special notification handling
4. THE priority selection SHALL be visible in the compose area as a dropdown or button group

### Requirement 12.2: Urgent Message Notifications

**User Story:** As a channel participant, I want urgent messages to get my attention, so that I don't miss time-sensitive information.

#### Acceptance Criteria

1. WHEN a channel message is sent with Urgent priority THEN THE system SHALL send a persistent notification to all channel members
2. WHEN the recipient has Do Not Disturb mode enabled THEN THE system SHALL respect DND and not override it for urgent messages (configurable)
3. WHEN the recipient reads the urgent message THEN THE persistent notification SHALL be dismissed
4. THE system SHALL allow users to configure notification behavior for urgent messages in their notification preferences

### Requirement 12.3: Priority Display in Channel

**User Story:** As a channel participant, I want to see message priorities at a glance, so that I can scan for important messages.

#### Acceptance Criteria

1. WHEN a message has a priority set THEN THE system SHALL display a colored priority indicator (badge/border/icon) before the message timestamp
2. Important messages SHALL use a yellow/amber indicator; Urgent messages SHALL use a red indicator
3. THE priority indicator SHALL be visible in both the main channel and thread views
4. THE priority indicator SHALL be included in search results for easy scanning

### Requirement 12.4: Priority in Threads

**User Story:** As a channel participant, I want message priorities in thread replies, so that urgency is preserved in nested conversations.

#### Acceptance Criteria

1. WHEN a reply in a thread has a priority set THEN THE system SHALL display the priority indicator on that reply
2. WHEN the root message of a thread has a priority THEN THE system SHALL include the priority indicator in the thread summary/reply count badge
3. Thread replies with Urgent priority SHALL trigger the same notification behavior as top-level urgent messages
