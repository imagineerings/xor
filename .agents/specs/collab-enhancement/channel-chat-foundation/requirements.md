# Requirements: Channel Chat Foundation

## Introduction

The collaboration enhancement suite assumes channel chat messages are available through the existing `SendChannelMessage`, `JoinChannelChat`, and related protobuf types. The current collab server rejects those RPCs with "chat has been removed in the latest version of Sim", and the desktop UI has no channel message list or compose surface to enhance. Restoring a minimal, reliable channel-chat foundation is the prerequisite for rich text, reactions, threading, file sharing, search, scheduled messages, message priorities, user-group mentions, and mobile collaboration.

## Requirements

### Requirement 0.1: Channel Chat Storage

**User Story:** As a channel participant, I want messages to persist in the channel, so that conversation history is available across sessions and devices.

#### Acceptance Criteria

1. WHEN a user sends a channel message THEN THE system SHALL persist the message body, sender, channel, timestamp, nonce, mentions, optional parent message, and optional edit timestamp.
2. WHEN a channel message is deleted THEN THE system SHALL preserve message ordering and return an inert tombstone or remove the message consistently from future history responses.
3. WHEN a channel message is edited THEN THE system SHALL update the body, mentions, nonce, and edited timestamp without changing the original message id or sender.

### Requirement 0.2: Channel Chat RPCs

**User Story:** As a connected client, I want the channel chat RPCs to work, so that I can join chat, send messages, retrieve history, edit messages, delete messages, and acknowledge reads.

#### Acceptance Criteria

1. WHEN a member calls `JoinChannelChat` THEN THE system SHALL register the connection as an active chat participant and return the most recent channel messages.
2. WHEN a participant calls `LeaveChannelChat` THEN THE system SHALL stop sending live chat updates to that connection.
3. WHEN a participant calls `SendChannelMessage` THEN THE system SHALL return `SendChannelMessageResponse` and broadcast `ChannelMessageSent` to active participants in that channel.
4. WHEN a participant calls `GetChannelMessages` or `GetChannelMessagesById` THEN THE system SHALL return only messages for channels the caller can access.
5. WHEN a participant calls `AckChannelMessage` THEN THE system SHALL record the latest read message for that participant.

### Requirement 0.3: Channel Chat Desktop UI

**User Story:** As a desktop user, I want a channel chat view, so that I can read and send channel messages from Sim.

#### Acceptance Criteria

1. WHEN a user opens a channel chat THEN THE system SHALL show a scrollable message list with sender identity, timestamp, and message body.
2. WHEN a user types and submits a message THEN THE system SHALL send it with `SendChannelMessage` and clear the composer after success.
3. WHILE the connection is disconnected or the send fails THE system SHALL surface meaningful UI feedback and preserve unsent text.
4. WHEN live `ChannelMessageSent` or `ChannelMessageUpdate` events arrive THEN THE system SHALL update the open channel chat without requiring a reload.

### Requirement 0.4: Access Control and Compatibility

**User Story:** As a workspace administrator, I want channel chat access to follow channel membership, so that private channel conversations remain private.

#### Acceptance Criteria

1. IF a user is not allowed to view a channel THEN THE system SHALL reject chat RPCs for that channel.
2. WHEN existing clients send protobuf messages using the current channel chat schema THEN THE system SHALL remain wire-compatible with those messages.
3. WHEN future enhancement specs add fields or UI behavior THEN THE foundation SHALL expose clear server, client, and UI extension points rather than duplicating storage or transport.
