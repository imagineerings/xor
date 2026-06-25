# Requirements: User Groups

## Introduction

Baymax currently supports @mentioning individual users but has no concept of user groups. Mattermost supports user groups — named collections of users (@eng, @design, @team-leads) that can be @mentioned to notify all members at once. Adding groups will streamline team communication.

## Glossary

- **User Group**: A named collection of users within a workspace
- **Group Member**: A user who belongs to a group
- **@Mention**: Mentioning a group sends notifications to all group members
- **Group Admin**: A user who can manage group membership

## Requirements

### Requirement 9.1: Create and Manage Groups

**User Story:** As a user, I want to create named groups of users, so that I can @mention everyone in the group at once.

#### Acceptance Criteria

1. WHEN a user opens group management THEN THE system SHALL show a "Create Group" button
2. WHEN the user creates a group THEN THE system SHALL prompt for: group name (alphanumeric with hyphens, e.g., "eng-team"), display name (human-readable), and initial member list
3. WHEN the group is created THEN THE system SHALL persist it and allow members and group admins to manage the group
4. THE system SHALL enforce uniqueness of group names within a workspace
5. THE system SHALL support a maximum group size (configurable, default 100 members)

### Requirement 9.2: @Mention Groups

**User Story:** As a channel participant, I want to @mention a group, so that all group members receive a notification.

#### Acceptance Criteria

1. WHEN a user types "@" followed by the beginning of a group name in the message compose area THEN THE system SHALL show matching groups in the autocomplete dropdown alongside individual users
2. WHEN the user selects a group from autocomplete THEN THE system SHALL insert the group mention with distinct visual styling (e.g., different color from individual @mentions)
3. WHEN the message is sent THEN THE system SHALL send notifications to all online members of the mentioned group
4. The system SHALL allow users to leave a group if they no longer wish to receive group @mentions

### Requirement 9.3: Group Membership Management

**User Story:** As a group admin, I want to add and remove group members, so that the group stays current.

#### Acceptance Criteria

1. WHEN viewing a group THEN THE system SHALL show a member list with add/remove controls for group admins
2. WHEN adding members THEN THE system SHALL show a searchable user picker
3. WHEN a user is added to a group THEN THE system SHALL notify the user (optional notification)
4. WHEN a user is removed from a group THEN THE system SHALL notify the user
5. WHEN a group has no members remaining THEN THE system SHALL flag the group as empty (but not auto-delete)
