# Requirements: Channel Bookmarks

## Introduction

Baymax channels lack a mechanism to permanently pin important links or references. Mattermost supports channel bookmarks (`bookmarks.yaml` API) — a dedicated section in the channel header where users can save important links, files, and messages for easy access by all members. This is a simple but high-value enhancement for team channels.

## Glossary

- **Bookmark**: A permanently saved link, file reference, or message reference in a channel
- **Bookmark Bar**: The UI section displaying bookmarks (typically in the channel header area)

## Requirements

### Requirement 6.1: Create and Manage Bookmarks

**User Story:** As a channel participant (with appropriate permissions), I want to add bookmarks to a channel, so that important resources are always accessible to members.

#### Acceptance Criteria

1. WHEN a user with edit permission opens a channel THEN THE system SHALL display a bookmarks section in the channel header
2. WHEN the user clicks a "Add bookmark" button THEN THE system SHALL prompt for a URL, label, and optional description
3. WHEN the user submits the bookmark form THEN THE system SHALL persist the bookmark to the server and display it in the bookmarks section
4. THE system SHALL support bookmark types: link (URL), file (file attachment reference), and message (link to specific message)
5. WHEN the user hovers over a bookmark THEN THE system SHALL show options to edit the label, reorder, or delete the bookmark
6. Only users with `Admin` or `Member` role in the channel SHALL be able to add/edit/delete bookmarks

### Requirement 6.2: Bookmarks Display

**User Story:** As a channel member, I want to see channel bookmarks prominently, so that I can quickly find pinned resources.

#### Acceptance Criteria

1. THE bookmarks section SHALL be visible at the top of the channel, above the message list
2. Each bookmark SHALL display: an icon (based on type), label, and a brief description if provided
3. WHEN the user clicks a bookmark link THEN THE system SHALL open the URL in the default browser (external) or the relevant file/message in Baymax
4. WHEN a channel has more than 5 bookmarks THEN THE system SHALL show a "Show all" expand/collapse toggle
5. THE bookmarks SHALL persist across sessions for all channel members

### Requirement 6.3: Bookmark Reordering

**User Story:** As a channel admin, I want to reorder bookmarks, so that the most important ones appear first.

#### Acceptance Criteria

1. WHEN editing bookmarks THEN THE admin SHALL be able to drag-and-drop bookmarks to reorder them
2. THE bookmark order SHALL be persisted and synced to all channel members
3. WHEN the user reorders a bookmark THEN THE system SHALL update the server and all clients in real-time

### Requirement 6.4: Bookmark Notifications and Updates

**User Story:** As a channel member, I want to know when bookmarks are added or updated, so that I stay informed of important resources.

#### Acceptance Criteria

1. WHEN a bookmark is added THEN THE system SHALL post an informational message in the channel (e.g., "Alice pinned a link: Deployment Guide")
2. WHEN a bookmark is deleted THEN THE system SHALL post an informational message
3. WHEN a bookmark is updated (label changed) THEN THE system SHALL post an informational message
4. Informational bookmark messages SHALL be visually distinct from regular messages
