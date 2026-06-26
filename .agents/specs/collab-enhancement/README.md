# Collaboration Enhancement Suite

Migration of Mattermost collaboration features into Baymax.

## Overview

This spec suite defines the migration of select Mattermost features into Baymax's existing collaboration system. Baymax already has a strong foundation: channels, calls (LiveKit), real-time collaborative editing (CRDT), contacts, notifications, rooms, project sharing, and basic roles. These specs enhance that foundation with rich messaging, file sharing, and team collaboration features that Mattermost pioneered.

## Priority Matrix

| Priority | Feature | Spec Status | Effort | Impact | Depends On |
|---|---|---|---|---|---|
| **P0** | [Channel Rich Text](channel-rich-text/) | ✅ Req + Design + Tasks | Medium | High | — |
| **P0** | [Channel Reactions](channel-reactions/) | ✅ Req + Design + Tasks | Small | High | — |
| **P0** | [Message Threading](channel-threads/) | ✅ Req + Design + Tasks | Large | High | — |
| **P1** | [File Sharing](channel-file-sharing/) | ✅ Req + Design + Tasks | Large | High | Rich Text (for file messages) |
| **P1** | [Message Search](channel-message-search/) | ✅ Req + Design + Tasks | Large | High | — |
| **P1** | [Channel Bookmarks](channel-bookmarks/) | ✅ Req + Design + Tasks | Small | Medium | — |
| **P1** | [Message Drafts](channel-message-drafts/) | ✅ Req + Design + Tasks | Small | Medium | — |
| **P2** | [Custom User Status](custom-user-status/) | ✅ Req + Design + Tasks | Small | Medium | — |
| **P2** | [User Groups](user-groups/) | ✅ Req + Design + Tasks | Medium | Medium | Mention System |
| **P2** | [Channel Join Requests](channel-join-requests/) | ✅ Req + Design + Tasks | Small | Medium | Channel Permissions |
| **P3** | [Scheduled Messages](scheduled-messages/) | ✅ Req + Design + Tasks | Medium | Low | — |
| **P3** | [Message Priorities](message-priorities/) | ✅ Req + Design + Tasks | Small | Low | — |
| **P3** | [Channel Recaps](channel-recaps/) | ✅ Req + Design + Tasks | Large | Low | Background Job System |

## Architecture Impact Areas

These features touch the following Baymax components:

| Component | Features Affected |
|---|---|
| `channel` crate | Rich Text (message body format), Reactions, Threads, Bookmarks, File Sharing, Scheduled Messages, Priorities |
| `collab` crate (server) | All features — new API endpoints, message types, background jobs |
| `collab_ui` crate | UI components for all features — thread panel, reaction picker, bookmark bar, file preview, search UI |
| `proto` crate | New protobuf message types for reactions, threads, file attachments, bookmarks, scheduled messages |
| `client` crate | New request/response types for file uploads, search, scheduling |
| `rpc` crate | New message handlers for real-time features (reactions, typing indicators, thread updates) |
| `editor` crate | Rich text rendering (Markdown parser), file preview integration |
| `db` / `migrations` | New tables for reactions, file metadata, bookmarks, scheduled posts, recaps |

## Execution Order (Recommended)

1. **Phase 1** (P0): Rich Text + Reactions + Drafts — foundational messaging UX
2. **Phase 2** (P1): File Sharing + Bookmarks — content sharing
3. **Phase 3** (P1): Message Search — information retrieval
4. **Phase 4** (P0): Message Threading — conversation organization (depends on rich text)
5. **Phase 5** (P2): Custom Status + User Groups — team awareness
6. **Phase 6** (P2): Channel Join Requests — access management
7. **Phase 7** (P3): Scheduled Messages + Priorities + Recaps — advanced features
