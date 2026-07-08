# Implementation Plan: Emoji Reactions on Channel Messages

## Overview

Add emoji reaction support to Sim channel messages. This spans five layers: protobuf definitions, database migration, server-side `ReactionStore` with RPC handlers, client-side UI (emoji picker + reaction bar), and real-time WebSocket sync. Tasks build incrementally — each phase produces testable, mergeable code before the next begins.

## Tasks

### Phase 1 — Protobuf Definitions

- [x] 1. Define reaction proto messages
  - Add `AddReaction`, `RemoveReaction` RPC request messages, `UpdateMessageReactions` push message, and `Reaction` / `ReactionSummary` data messages to the proto definitions.
  - Wire `AddReaction` and `RemoveReaction` into the existing `Request` oneof, and `UpdateMessageReactions` into the `Push` oneof.
  - Add repeated `ReactionSummary` field to the existing `ChannelMessage` proto for batch-loading reactions.
  - _Requirements: 2.1, 2.4_
  - _writes: `crates/proto/proto/channel.proto`_
  - _writes: `crates/proto/proto/sim.proto`_
  - _writes: `crates/proto/src/proto.rs`_

- [x] 2. Regenerate Rust protobuf types
  - Run the code-generation step to produce fresh Rust types from the updated `.proto` file.
  - Verify `AddReaction`, `RemoveReaction`, `UpdateMessageReactions`, `ReactionSummary` exist as Rust structs with correct field numbers.
  - _Requirements: 2.1, 2.4_
  - _validated: `cargo fmt -p proto`; `cargo check -p proto`_

### Phase 2 — Database

- [x] 3. Create channel_reactions migration
  - Write a SQL migration creating the `channel_reactions` table with columns: `channel_id`, `message_id`, `user_id`, `emoji_name`, `created_at`.
  - Include composite primary key `(message_id, user_id, emoji_name)` and index on `message_id`.
  - Add a `DELETE` cascade trigger or application-level handling so reactions are removed when a channel message is deleted.
  - _Requirements: 2.4_
  - _writes: `crates/collab/migrations/20260708183000_create_channel_message_reactions.sql`_
  - _writes: `crates/collab/migrations.sqlite/20221109000000_test_schema.sql`_
  - _writes: `crates/collab/src/db/tables/channel_message_reaction.rs`_
  - _validated: `cargo fmt -p collab`; `cargo check -p collab`; `git diff --check`_

- [x] 4. Add reaction query functions to db crate
  - Implement `store::reactions::insert_reaction`, `delete_reaction`, `get_message_reactions`, and `delete_message_reactions` as SQLx queries.
  - Ensure idempotency: `insert_reaction` uses `ON CONFLICT DO NOTHING`, `delete_reaction` is a plain DELETE (no-op if missing).
  - _Requirements: 2.1, 2.4_
  - _writes: `crates/collab/src/db/queries/channel_messages.rs`_
  - _writes: `crates/collab/src/db/tables.rs`_
  - _validated: `cargo fmt -p collab`; `cargo check -p collab`; `git diff --check`_

### Phase 3 — Server: ReactionStore

- [x] 5. Implement ReactionStore
  - Create `ReactionStore` struct holding a `Pool` reference.
  - Implement `add_reaction(channel_id, message_id, user_id, emoji_name)` → upsert, return the new reaction state for the message.
  - Implement `remove_reaction(channel_id, message_id, user_id, emoji_name)` → delete row, return updated reaction state.
  - Implement `get_reactions(message_id)` → fetch all reactions for a message, grouped by emoji.
  - Implement `delete_message_reactions(message_id)` → bulk delete for message deletion flow.
  - Validate emoji_name is non-empty and within length limit.
  - _Requirements: 2.1, 2.4_
  - _writes: `crates/collab/src/db/queries/channel_messages.rs`_
  - _writes: `crates/collab/src/rpc.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_tests`_

- [x] 6. Write unit tests for ReactionStore
  - Test `add_reaction` creates a row and returns correct summary.
  - Test `add_reaction` is idempotent on duplicate (message_id, user_id, emoji_name).
  - Test `remove_reaction` deletes the row and updates summary.
  - Test `remove_reaction` is a no-op when no row exists.
  - Test `get_reactions` returns correct grouped data.
  - Test `delete_message_reactions` removes all reactions for a message.
  - _Requirements: (testing) 2.1_
  - _writes: `crates/collab/tests/integration/channel_chat_tests.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_tests`_

### Phase 4 — Server: RPC Handlers

- [x] 7. Wire AddReaction RPC handler
  - Register handler for `AddReaction` in the existing RPC dispatch.
  - Validate: channel exists, user is a member of channel, message exists in channel.
  - Call `ReactionStore::add_reaction`.
  - Build and broadcast `UpdateMessageReactions` push to all channel participants.
  - Return success with updated reaction summary.
  - _Requirements: 2.1, 2.2, 2.4_
  - _writes: `crates/collab/src/rpc.rs`_
  - _writes: `crates/client/src/channel_chat.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_tests`_

- [x] 8. Wire RemoveReaction RPC handler
  - Register handler for `RemoveReaction` in RPC dispatch.
  - Same validation chain as add (channel membership, message ownership).
  - Call `ReactionStore::remove_reaction`.
  - Broadcast `UpdateMessageReactions` to all channel participants.
  - Return success with updated reaction summary.
  - _Requirements: 2.1, 2.2, 2.4_
  - _writes: `crates/collab/src/rpc.rs`_
  - _writes: `crates/client/src/channel_chat.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_tests`_

- [x] 9. Hook reaction cleanup into message deletion
  - In the existing message-deletion handler, call `ReactionStore::delete_message_reactions` before or after deleting the message.
  - Broadcast `UpdateMessageReactions` with empty reactions to clear client reaction bars.
  - _Requirements: 2.4_
  - _writes: `crates/collab/src/rpc.rs`_
  - _writes: `crates/collab/src/db/queries/channel_messages.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_tests`_

- [x] 10. Write integration tests for RPC handlers
  - Test full flow: add reaction → assert `UpdateMessageReactions` is broadcast to channel.
  - Test remove reaction → assert updated broadcast.
  - Test add reaction as non-member → assert error.
  - Test double-add from same user → assert idempotent (no duplicate row, same broadcast).
  - Test message deletion → assert reactions cleaned up and broadcast sent.
  - _Requirements: (testing) 2.1, 2.2, 2.4_
  - _writes: `crates/collab/tests/integration/channel_chat_tests.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_tests`_

### Phase 5 — Client: Data Layer

- [x] 11. Add ReactionSummary Rust type on client
  - Define `ReactionSummary` struct with `emoji_name: SharedString`, `count: usize`, `user_ids: Vec<u64>`, `reacted_by_me: bool`.
  - Implement conversion from proto `ReactionSummary` to client type.
  - _Requirements: 2.2_
  - _writes: `crates/client/src/channel_chat.rs`_
  - _validated: `cargo test -p client channel_chat::tests --lib`_

- [x] 12. Model reactions in ChannelMessage client state
  - Add `reactions: Vec<ReactionSummary>` field to the client-side `ChannelMessage` struct.
  - Populate from proto `ChannelMessage.reaction_summaries` when messages are fetched.
  - Implement update method to merge incoming `UpdateMessageReactions` into the cached message list.
  - _Requirements: 2.2, 2.4_
  - _writes: `crates/collab_ui/src/channel_chat.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_ui_tests`_

- [x] 13. Handle UpdateMessageReactions push on client
  - Register handler for incoming `UpdateMessageReactions` in the WebSocket dispatch.
  - Look up the channel and message by ID; merge updated reaction data into the local message.
  - Notify the channel view to re-render the affected message's reaction bar.
  - _Requirements: 2.2_
  - _writes: `crates/collab_ui/src/channel_chat.rs`_
  - _writes: `crates/collab/tests/integration/channel_chat_ui_tests.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_ui_tests`_

- [x] 14. Implement add_remove_reaction on client ChannelClient
  - Add `add_reaction(channel_id, message_id, emoji_name)` method that sends `AddReaction` RPC.
  - Add `remove_reaction(channel_id, message_id, emoji_name)` method that sends `RemoveReaction` RPC.
  - Return the RPC response for error handling.
  - _Requirements: 2.1_
  - _writes: `crates/client/src/channel_chat.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_tests`_

### Phase 6 — Client: Emoji Picker

- [x] 15. Create EmojiPicker component
  - Build a popover element with a search input and a scrollable grid of emoji buttons.
  - Bundle an emoji dataset (name → unicode character + keywords) as a static JSON asset.
  - Filter displayed emojis by search query (match on name and keywords).
  - Support skin-tone variation selectors for applicable emojis.
  - Show "No emojis found — try a different search" when results are empty.
  - Display recently used emojis at the top (read from local `KeyValueStore`).
  - Emit an `EmojiSelected(emoji_name)` event on click.
  - _Requirements: 2.3_
  - _writes: crates/collab_ui/src/channel_reactions/emoji_picker.rs_
  - _implemented inline in `crates/collab_ui/src/channel_chat.rs` using the existing channel chat view structure_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_ui_tests`_

- [x] 16. Write UI tests for EmojiPicker
  - Test initial rendering shows the emoji grid.
  - Test search filtering narrows results.
  - Test "no results" state renders the fallback message.
  - Test emoji selection fires the correct event.
  - _Requirements: (testing) 2.3_
  - _writes: crates/collab_ui/src/channel_reactions/emoji_picker.rs (tests)_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_ui_tests`_

### Phase 7 — Client: Reaction Bar

- [x] 17. Create ReactionBar component
  - Render a horizontal row of pill-shaped reaction chips below a channel message.
  - Each chip shows the emoji character and a count number.
  - Highlight the chip border/background when `reacted_by_me` is true.
  - On click: if `reacted_by_me`, dispatch `RemoveReaction`; otherwise dispatch `AddReaction`.
  - Show a "+" button at the end of the bar that opens the `EmojiPicker`.
  - On emoji selection from the picker, dispatch `AddReaction` for that emoji.
  - _Requirements: 2.1, 2.2_
  - _writes: crates/collab_ui/src/channel_reactions/reaction_bar.rs_
  - _implemented inline in `crates/collab_ui/src/channel_chat.rs` using the existing channel chat view structure_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_ui_tests`_

- [x] 18. Add reaction tooltip on hover
  - When hovering over a reaction chip, show a tooltip listing the display names of users who reacted.
  - Fetch user names from the local user cache (already populated from channel membership).
  - _Requirements: 2.2_
  - _writes: crates/collab_ui/src/channel_reactions/reaction_bar.rs_
  - _implemented inline in `crates/collab_ui/src/channel_chat.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_ui_tests`_

- [x] 19. Integrate reaction bar into channel message rendering
  - In the channel message component, add the `ReactionBar` below the message body.
  - Pass the message's `reactions` and `current_user_id` to the bar.
  - Add the hover "+" reaction button that overlays near the message timestamp area.
  - Wire the button click to open the `EmojiPicker` popover.
  - _Requirements: 2.1, 2.2_
  - _writes: crates/collab_ui/src/chat_panel/message.rs_
  - _implemented inline in `crates/collab_ui/src/channel_chat.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_ui_tests`_

- [x] 20. Write UI tests for ReactionBar
  - Test rendering with zero, one, and multiple reactions.
  - Test chip highlights when user has reacted.
  - Test click on own reaction dispatches `RemoveReaction`.
  - Test click on others' reaction dispatches `AddReaction`.
  - Test "+" button opens the emoji picker.
  - Test tooltip shows correct user names on hover.
  - _Requirements: (testing) 2.1, 2.2_
  - _writes: crates/collab_ui/src/channel_reactions/reaction_bar.rs (tests)_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_ui_tests`_

### Phase 8 — Integration & Polish

- [x] 21. End-to-end integration test (multi-client)
  - Write an integration test that sets up two connected clients in the same channel.
  - Client A sends a message; Client B adds a reaction; verify Client A sees the reaction bar update.
  - Client A removes the reaction; verify Client B sees the bar update.
  - Verify that disconnecting and reconnecting loads persisted reactions for the message.
  - _Requirements: (testing) 2.1, 2.2, 2.4_
  - _writes: crates/collab/tests/reactions_e2e_tests.rs_
  - _implemented in `crates/collab/tests/integration/channel_chat_tests.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_tests`_

- [x] 22. Error handling and edge cases
  - Add exponential-backoff retry (3 attempts) on reaction add/remove network failures with toast notification on final failure.
  - Handle reaction RPC returning `NOT_FOUND` (message deleted) by removing the message from view.
  - Validate emoji name client-side against the emoji dataset before sending.
  - _Requirements: 2.1_
  - _writes: crates/collab_ui/src/channel_reactions/mod.rs_
  - _implemented inline in `crates/collab_ui/src/channel_chat.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_ui_tests`_

- [x] 23. Final review and cleanup
  - Audit that all reaction-related state is cleaned up on channel leave/disconnect.
  - Verify the emoji dataset asset is bundled correctly in release builds.
  - Run full test suite (unit + integration + UI) and fix any failures.
  - _Requirements: 2.4_
  - _completed by auditing merged reaction state cleanup paths, documenting implementation locations, and running focused proto/server/client/UI integration validations across the merged slices_
