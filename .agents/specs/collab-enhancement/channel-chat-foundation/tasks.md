# Implementation Plan: Channel Chat Foundation

## Overview

Restore the channel-chat storage, RPC, client, and desktop UI foundation that the rest of the collaboration enhancement suite assumes. Complete this before starting rich text, reactions, threading, file sharing, search, scheduled messages, priorities, or user-group message mentions.

## Tasks

- [x] 1. Add channel message persistence
  - [x] 1.1 Create migrations for channel messages, mentions, and read acknowledgements.
  - [x] 1.2 Add SeaORM table definitions and id conversions.
  - [x] 1.3 Register the new entities with the collab DB module.
  - _Requirements: 0.1_
  - _writes: `crates/collab/migrations/*channel_messages*.sql`_
  - _writes: `crates/collab/migrations.sqlite/20221109000000_test_schema.sql`_
  - _writes: `crates/collab/src/db/tables/channel_message.rs`_
  - _writes: `crates/collab/src/db/tables/channel_message_mention.rs`_
  - _writes: `crates/collab/src/db/tables/channel_message_read.rs`_
  - _validated: `cargo fmt -p collab`; `cargo check -p collab`_

- [x] 2. Implement channel message store queries
  - [x] 2.1 Implement create, update, delete/tombstone, paginated history, lookup-by-id, and acknowledgement queries.
  - [x] 2.2 Enforce channel membership before returning or mutating messages.
  - [x] 2.3 Convert DB rows to existing `proto::ChannelMessage` values.
  - _Requirements: 0.1, 0.2, 0.4_
  - _writes: `crates/collab/src/db/queries/channel_messages.rs`_
  - _validated: `cargo fmt -p collab`; `cargo check -p collab`_

- [x] 3. Restore server channel-chat RPC handlers
  - [x] 3.1 Replace removed-chat errors in `send_channel_message`, `join_channel_chat`, `leave_channel_chat`, `get_channel_messages`, `get_channel_messages_by_id`, `update_channel_message`, `remove_channel_message`, and `acknowledge_channel_message`.
  - [x] 3.2 Track active participants with `channel_chat_participants`.
  - [x] 3.3 Broadcast `ChannelMessageSent` and `ChannelMessageUpdate` to active participants.
  - _Requirements: 0.2, 0.4_
  - _writes: `crates/collab/src/rpc.rs`_
  - _writes: `crates/collab/src/db/queries/channel_messages.rs`_
  - _validated: `cargo fmt -p collab`; `cargo check -p collab`_

- [x] 4. Add client channel-chat API
  - [x] 4.1 Add focused methods for join, leave, send, history, edit, delete, and ack.
  - [x] 4.2 Add live event handling for `ChannelMessageSent` and `ChannelMessageUpdate`.
  - [x] 4.3 Surface errors to callers instead of swallowing them.
  - _Requirements: 0.2, 0.3_
  - _writes: `crates/client/src/channel_chat.rs`_
  - _writes: `crates/client/src/client.rs`_
  - _validated: `cargo fmt -p client`; `cargo check -p client`_

- [ ] 5. Build the desktop channel-chat view
  - [ ] 5.1 Render a scrollable message list with sender, timestamp, and body text.
  - [ ] 5.2 Add a composer that sends through the client channel-chat API.
  - [ ] 5.3 Preserve drafts and show user-visible errors on send failure.
  - [ ] 5.4 Apply live `ChannelMessageSent` and `ChannelMessageUpdate` events to the open view.
  - _Requirements: 0.3_
  - _writes: `crates/collab_ui/src/channel_chat.rs`_
  - _writes: `crates/collab_ui/src/collab_ui.rs`_

- [ ] 6. Wire channel chat into navigation
  - [ ] 6.1 Add an entry point from channel navigation/panel surfaces.
  - [ ] 6.2 Ensure leaving the view calls `LeaveChannelChat` and cleans subscriptions.
  - _Requirements: 0.3_
  - _writes: `crates/collab_ui/src/collab_panel.rs`_

- [ ] 7. Add server integration tests
  - [ ] 7.1 Test join, send, history, edit, delete, and ack flows.
  - [ ] 7.2 Test private channel access rejection.
  - [ ] 7.3 Test simultaneous sends preserve stable ordering.
  - _Requirements: 0.1, 0.2, 0.4_
  - _writes: `crates/collab/tests/integration/channel_chat_tests.rs`_

- [ ] 8. Add client and GPUI tests
  - [ ] 8.1 Test client request/response conversions and live event application.
  - [ ] 8.2 Test desktop message rendering, send success, send failure, and live insertion.
  - _Requirements: 0.2, 0.3_
  - _writes: `crates/client/src/channel_chat.rs`_ (tests)
  - _writes: `crates/collab_ui/src/channel_chat.rs`_ (tests)
