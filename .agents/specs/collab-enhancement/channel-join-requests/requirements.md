# Requirements: Channel Join Requests

## Introduction

Baymax private channels currently require an explicit invitation from an existing member. Mattermost supports join requests — users can request access to a private channel, and channel admins/members can approve or deny the request. This reduces friction for team onboarding.

## Glossary

- **Join Request**: A request from a user to join a private channel
- **Private Channel**: A channel with `Members` visibility (only members can see it)
- **Public Channel**: A channel with `Public` visibility (anyone can join directly)

## Requirements

### Requirement 10.1: Request to Join Private Channels

**User Story:** As a channel participant who can see a private channel exists, I want to request to join it, so that I don't need to find a member to invite me.

#### Acceptance Criteria

1. WHEN a user navigates to a private channel they are not a member of THEN THE system SHALL show a "Request to Join" button instead of the channel content
2. WHEN the user clicks "Request to Join" THEN THE system SHALL send a join request notification to channel admins
3. WHEN a request is sent THEN THE system SHALL display a confirmation to the requesting user: "Join request sent. You'll be notified when a channel admin responds."
4. THE system SHALL prevent duplicate pending requests from the same user for the same channel

### Requirement 10.2: Approve or Deny Join Requests

**User Story:** As a channel admin, I want to review and respond to join requests, so that I control who joins my channels.

#### Acceptance Criteria

1. WHEN a join request arrives THEN THE system SHALL notify channel admins (and optionally members with Manage permission) via the notification system
2. WHEN an admin opens the join request THEN THE system SHALL show: the requesting user's profile, any join reason provided, and Approve/Deny buttons
3. WHEN the admin approves the request THEN THE system SHALL add the user to the channel and notify the user
4. WHEN the admin denies the request THEN THE system SHALL notify the user with an optional denial reason
5. THE join request list SHALL be accessible from the channel member management interface

### Requirement 10.3: Join Request Notifications

**User Story:** As a requesting user, I want to be notified when my join request is approved or denied, so that I know the outcome.

#### Acceptance Criteria

1. WHEN a join request is approved THEN THE requesting user SHALL receive a notification with a link to join the channel
2. WHEN a join request is denied THEN THE requesting user SHALL receive a notification (without showing denial reason unless provided)
3. WHEN the user is approved AND clicks the notification link THEN THE system SHALL navigate them to the channel

### Requirement 10.4: Pending Requests Visibility

**User Story:** As a channel admin, I want to see all pending join requests, so that I don't miss any.

#### Acceptance Criteria

1. THE system SHALL show a badge on the channel management UI indicating the count of pending join requests
2. THE system SHALL list all pending requests with timestamps in the channel member management view
3. THE system SHALL auto-expire pending requests after a configurable period (default 7 days)
