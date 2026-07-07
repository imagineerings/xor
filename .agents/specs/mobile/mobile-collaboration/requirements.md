# Requirements: Collaboration Features

## Introduction

The Sim mobile client will connect to the Sim Collaboration Server (the same `collab` server used by the desktop editor) to provide channels, chat, contacts, shared documents, project sharing, and agent thread sharing. These features transform the app from a solo agent chat into a multi-user collaborative platform. This spec adapts the collaboration features from both `mobile-dev` (channels, channel chat, channel management) and the Sim `collab` crate (channels, channel buffers, contacts, project sharing).

## Glossary

| Term | Definition |
|------|------------|
| **Collab Server** | The Sim multi-user server providing channels, calls, presence, and project sharing. Connected via WebSocket/RPC. |
| **Channel** | A persistent collaboration space with chat messages and optionally a shared document (channel buffer). |
| **Channel Buffer** | A collaborative document within a channel that multiple users can edit in real-time. |
| **Channel Chat** | Asynchronous messaging within a channel. Distinct from the agent chat session. |
| **Contact** | Another Sim user in your contacts list. Supports presence (online/offline). |
| **Presence** | Real-time indicator of whether a contact is online, idle, or offline. |
| **Project Sharing** | Inviting collaborators to view/edit your current project remotely. |
| **Agent Thread Sharing** | Sharing an AI agent conversation thread with channel members. |

## Requirements

### Requirement 1: Channel Browser

**User Story:** As a mobile user, I want to browse and join collaboration channels, so I can participate in team discussions.

1.1 THE app SHALL display a channel list organized by categories: Favorites, Channels, Direct Messages.

1.2 WHEN the user opens the channel list THEN THE app SHALL fetch the channel list from the collab server.

1.3 THE channel list SHALL show: channel name, unread indicator (if any), and member count.

1.4 WHEN the user taps a channel THEN THE app SHALL open the channel view showing channel chat.

1.5 THE app SHALL support joining a channel and leaving a channel.

1.6 THE app SHALL support marking channels as favorites for quick access.

### Requirement 2: Channel Chat

**User Story:** As a mobile user, I want to send and receive messages in collaboration channels, so I can communicate with my team.

2.1 WHEN the user opens a channel THEN THE app SHALL load recent channel chat messages.

2.2 THE app SHALL receive new channel messages in real-time via the collab WebSocket.

2.3 WHEN the user sends a message in a channel THEN THE app SHALL post it to the collab server via the channel chat API.

2.4 THE channel chat SHALL support the same markdown rendering as the agent chat (bold, lists, code blocks, etc.).

2.5 THE channel chat SHALL display message timestamps and sender names/avatars.

### Requirement 3: Channel Management

**User Story:** As a mobile user, I want to create and manage channels, so my team can organize discussions.

3.1 THE app SHALL support creating new channels with a name and optional description.

3.2 THE app SHALL support viewing channel members.

3.3 THE app SHALL support inviting users to a channel.

3.4 THE app SHALL support channel notification preferences (all messages, mentions only, mute).

### Requirement 4: Contacts & Presence

**User Story:** As a mobile user, I want to see who's online and manage my contacts, so I can connect with teammates.

4.1 THE app SHALL display a contact list showing: user name, avatar, and presence indicator (online/offline).

4.2 THE app SHALL update presence in real-time via the collab WebSocket.

4.3 THE app SHALL support sending and accepting contact requests.

4.4 WHEN the user taps a contact THEN THE app SHALL show the contact's profile and offer to start a direct message or call.

### Requirement 5: Shared Documents

**User Story:** As a mobile user, I want to view shared channel documents, so I can follow collaborative editing.

5.1 WHERE a channel has an associated channel buffer (shared document) THEN THE app SHALL show a "Notes" tab alongside the chat tab.

5.2 THE app SHALL display the document content as rendered markdown (read-only on mobile).

5.3 THE app SHALL update the document view in real-time as collaborators make changes.

### Requirement 6: Project Sharing

**User Story:** As a mobile user, I want to receive and accept project sharing invitations, so I can view shared projects.

6.1 WHEN a collaborator shares a project with the user THEN THE app SHALL display a notification.

6.2 WHEN the user accepts a shared project invitation THEN THE app SHALL show the project's file tree and allow browsing.

6.3 THE app SHALL support viewing file contents from a shared project (read-only on mobile).

6.4 THE app SHALL show which files collaborators are currently viewing.

### Requirement 7: Agent Thread Sharing

**User Story:** As a mobile user, I want to share my agent conversation threads with channel members.

7.1 WHEN the user selects "Share to channel" on an agent session THEN THE app SHALL show a channel picker.

7.2 WHEN the user selects a channel THEN THE app SHALL post the agent thread summary to the channel chat.

7.3 WHEN another user taps a shared thread link THEN THE app SHALL open the shared thread view.

## Existing Assets

- mobile-dev: `app/products/channels/` (browse, create, members, settings), `app/screens/channel/`, `app/screens/find_channels/`, `app/screens/create_direct_message/`, `app/components/channel_list_row/`, `app/components/channel_item/`, `app/components/team_sidebar/`, `app/components/user_list/`
- Sim collab: `crates/collab_ui/src/collab_panel.rs` (channel browser UI), `crates/collab_ui/src/channel_view.rs` (channel buffer viewer), `crates/channel/src/channel_store.rs` (channel data model), `crates/collab_ui/src/notifications/project_shared_notification.rs`
