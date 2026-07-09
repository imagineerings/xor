# Implementation Plan: Message Threading in Channels

## Overview

Add threaded replies to channel messages in Sim. This feature reuses the existing `reply_to_message_id` proto field and introduces a thread panel (right sidebar) for viewing and composing replies, along with server endpoints for fetching threads and real-time reply delivery via WebSocket.

**Scope**: `proto` (new RPC messages), `collab` (server endpoints + ThreadStore queries), `collab_ui` (thread panel, thread indicator, compose input), `client` (thread types + RPC dispatch), `rpc` (WebSocket event handling).

---

## Tasks

- [x] 1. Define new protobuf messages for thread RPCs
  - Add `GetThread`, `GetThreadResponse`, `GetThreads`, `GetThreadsResponse`, `ThreadSummary` messages to `experiments.proto` (or channel-related proto file).
  - Add `GetThread`/`GetThreads` to the RPC dispatch enum and server handler trait.
  - _Requirements: 3.1, 3.4_
  - _writes: `proto/src/experiments.proto`_

- [x] 2. Implement ThreadStore on the server
  - [x] 2.1 Write SQL queries for `GetThread` (fetch replies by `reply_to_message_id` ordered chronologically), `GetThreads` (group-by with reply count and latest timestamp), and `GetReplyCount`.
  - [x] 2.2 Implement `ThreadStore` struct wrapping `sqlx::PgPool` with methods `get_thread`, `get_threads`, `get_reply_count`.
  - [x] 2.3 Add validation: return error if `reply_to_message_id` references a non-existent message.
  - _Requirements: 3.1, 3.4_
  - _writes: `collab/src/db/thread_store.rs`_
  - _implemented in existing `crates/collab/src/db/queries/channel_messages.rs` `Database` query layer_
  - _validated: `CARGO_INCREMENTAL=0 cargo check -p collab`; `CARGO_INCREMENTAL=0 cargo test -p collab --features test-support --test collab_tests channel_chat_tests`; `git diff --check`_

- [ ] 3. Register thread RPC handlers on the server
  - [ ] 3.1 Handle `GetThread` — parse request, call `ThreadStore::get_thread`, build `GetThreadResponse`.
  - [ ] 3.2 Handle `GetThreads` — parse request, call `ThreadStore::get_threads`, build `GetThreadsResponse`.
  - [ ] 3.3 Wire handlers into the existing `handle_rpc` dispatch in the channel server.
  - _Requirements: 3.1, 3.4_
  - _writes: `collab/src/rpc/channel_rpc.rs`_

- [ ] 4. Add client-side thread types and RPC dispatch
  - [ ] 4.1 Define `ThreadSummary` struct in the client crate matching the proto message.
  - [ ] 4.2 Add `get_thread` and `get_threads` methods to the `Client` struct that send the corresponding RPCs and deserialize responses.
  - _Requirements: 3.1, 3.4_
  - _writes: `client/src/channel_thread.rs`_

- [ ] 5. Build the ThreadPanel UI component
  - [ ] 5.1 Create `ThreadPanel` struct with fields: `channel_id`, `root_message`, `replies: Vec<ChannelMessage>`, `compose_editor: Entity<Editor>`, loading state, and error state.
  - [ ] 5.2 Implement `Render` for `ThreadPanel` — layout: header (channel name + close button), pinned root message, scrollable reply list, compose input at bottom.
  - [ ] 5.3 Implement `open` method that fetches the thread via `Client::get_thread` and populates the panel.
  - [ ] 5.4 Implement reply sending from the compose input — build `SendChannelMessage` with `reply_to_message_id` set to the root message ID.
  - [ ] 5.5 Register `ThreadPanel` as a dockable panel in the workspace (right sidebar), with a toggle action and keyboard shortcut (Escape to close).
  - _Requirements: 3.2_
  - _writes: `collab_ui/src/channel_thread/thread_panel.rs`_

- [ ] 6. Build the ThreadIndicator component
  - [ ] 6.1 Create `ThreadIndicator` struct with fields: `message_id`, `reply_count`, `has_unread`, `participants`.
  - [ ] 6.2 Implement `Render` — show "N replies" link when `reply_count > 0`, with participant avatar overlays and blue unread dot.
  - [ ] 6.3 Wire `ThreadIndicator` into the channel message rendering pipeline: display below each message that has replies.
  - [ ] 6.4 Handle click on the indicator — open/open the thread panel for that root message.
  - _Requirements: 3.1, 3.3_
  - _writes: `collab_ui/src/channel_thread/thread_indicator.rs`_

- [ ] 7. Implement thread unread tracking
  - [ ] 7.1 Reuse existing channel message read state infrastructure to track per-thread read timestamps.
  - [ ] 7.2 Compute `has_unread` for thread summaries: replies exist with `created_at > last_read_timestamp` for the current user.
  - [ ] 7.3 Mark all replies in a thread as read when the thread panel is opened.
  - [ ] 7.4 Update `ThreadIndicator` reactivity when `has_unread` state changes via `cx.notify()`.
  - _Requirements: 3.3_
  - _writes: `collab_ui/src/channel_thread/unread_tracker.rs`_

- [ ] 8. Wire real-time reply updates to the ThreadPanel
  - [ ] 8.1 In the `ChannelMessageSent` WebSocket handler, check if the received message has `reply_to_message_id` set and belongs to the currently open thread.
  - [ ] 8.2 If so, append the new reply to the `ThreadPanel::replies` list and call `cx.notify()`.
  - [ ] 8.3 If the thread panel is not open, update the relevant `ThreadIndicator`'s reply count and unread state.
  - _Requirements: 3.2_
  - _writes: `collab_ui/src/channel_thread/thread_realtime.rs`_

- [ ] 9. Add error handling and edge-case states
  - [ ] 9.1 Show a "This message has been deleted" placeholder if the root message is unavailable.
  - [ ] 9.2 Add retry logic with exponential backoff for thread loading failures (max 3 retries), showing a loading spinner during retry and an error state on exhaustion.
  - [ ] 9.3 Implement "Load earlier replies" button when a thread contains 50+ replies, fetching the next page.
  - [ ] 9.4 Handle optimistic reply sending — append locally, reconcile on server ack or revert on error.
  - _Requirements: 3.2_
  - _writes: `collab_ui/src/channel_thread/thread_panel.rs`_

- [ ] 10. Write tests
  - [ ] 10.1 **Unit tests**: `ThreadStore::get_thread`, `ThreadStore::get_threads`, `ThreadStore::get_reply_count` with known message IDs (empty, single reply, multiple replies, non-existent root).
  - [ ] 10.2 **Unit tests**: Validation that `reply_to_message_id` referencing a non-existent message returns an error.
  - [ ] 10.3 **Integration tests**: Full flow — send root message → send reply → call `GetThread` → verify reply is returned → verify `ThreadIndicator` shows correct count.
  - [ ] 10.4 **UI tests**: `ThreadPanel` rendering with varying reply counts, unread/read state transitions, compose input interaction and reply submission.
  - [ ] 10.5 **Real-time tests**: Two simulated clients both viewing the same thread; client A sends a reply; verify client B's panel appends the reply.
  - [ ] 10.6 **Edge-case tests**: Root message deleted before thread opens, network failure retry exhaustion, deep thread pagination.
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _writes: `collab/src/db/thread_store.rs`_ (tests), `collab_ui/src/channel_thread/thread_panel.rs` (tests), `collab_ui/src/channel_thread/thread_indicator.rs` (tests)
