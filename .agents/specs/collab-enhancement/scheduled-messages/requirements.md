# Requirements: Scheduled Messages

## Introduction

Sim channel messages are sent immediately upon composition. Mattermost supports scheduled messages (`scheduled_post.yaml` API) — messages that are composed now and delivered at a specified future time. This enables users to communicate at appropriate times without staying online.

## Glossary

- **Scheduled Message**: A message that is composed now but delivered at a future time
- **Schedule Time**: The UTC timestamp when the message should be sent
- **Pending Schedule**: A scheduled message that has not yet been sent

## Requirements

### Requirement 11.1: Schedule a Message for Future Delivery

**User Story:** As a channel participant, I want to compose a message now and schedule it for later, so that it arrives at an appropriate time.

#### Acceptance Criteria

1. WHEN composing a message THEN THE system SHALL provide a schedule dropdown (calendar/time picker) next to the send button
2. WHEN the user selects a future time AND clicks send THEN THE system SHALL schedule the message instead of sending it immediately
3. THE system SHALL support scheduling at least 1 minute in advance and at most 30 days in advance
4. WHEN the schedule time arrives THEN THE system SHALL send the message as if the user sent it at that moment
5. THE system SHALL support scheduling messages in both public and private channels

### Requirement 11.2: Manage Scheduled Messages

**User Story:** As a user, I want to view, edit, and cancel my scheduled messages, so that I have full control over future communications.

#### Acceptance Criteria

1. THE system SHALL provide a "Scheduled Messages" view listing all pending schedules ordered by send time
2. WHEN a user opens a pending schedule THEN THE system SHALL allow editing the message content and/or schedule time
3. WHEN a user cancels a scheduled message THEN THE system SHALL delete it and not send it
4. WHEN a scheduled message is sent (or cancelled) THEN THE system SHALL remove it from the pending list

### Requirement 11.3: Scheduled Message Notifications

**User Story:** As a user, I want visibility into my scheduled messages, so that I don't forget about them.

#### Acceptance Criteria

1. THE system SHALL show a badge or indicator in the channel sidebar when a user has pending scheduled messages
2. WHEN a scheduled message fails to send (e.g., user loses channel access) THEN THE system SHALL notify the user
3. THE system SHALL show "Scheduled" as a label on the send button when a future time is selected

### Requirement 11.4: Timezone Handling

**User Story:** As a user in a different timezone, I want scheduled messages to respect timezone settings, so that the schedule time is intuitive.

#### Acceptance Criteria

1. THE time picker SHALL use the user's local timezone by default
2. THE system SHALL display scheduled message times in the sender's local timezone
3. THE system SHALL store all schedule times in UTC on the server
