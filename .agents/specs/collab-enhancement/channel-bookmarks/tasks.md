# Implementation Plan: Channel Bookmarks

## Overview

Add a channel bookmarks feature: a dedicated section in the channel header where users with Admin/Member role can pin links, files, and messages. Bookmarks are persisted in the database, synced in real-time via WebSocket, and support drag-and-drop reordering. The feature spans protobuf definitions, a database migration, server-side CRUD operations, and client UI components (BookmarkBar, BookmarkForm, drag-reorder).

**Incremental build strategy:**

1. Proto + DB — the foundation, no behavior yet.
2. Server BookmarkStore + RPC handlers — back-end complete, testable via API.
3. Client BookmarkBar + BookmarkForm — UI for viewing and creating bookmarks.
4. Client bookmark management (edit, delete, context menu) — full CRUD from the UI.
5. Drag-and-drop reorder — reordering with real-time sync.
6. Real-time push notifications + channel system messages — final sync layer.
7. Polish, testing, and edge cases.

---

## Tasks

- [x] 1. Define protobuf messages
  - Add `Bookmark` message, `BookmarkType` enum, `AddBookmark`, `RemoveBookmark`, `UpdateBookmark`, and `ReorderBookmarks` RPC request messages.
  - Add `UpdateChannelBookmarks` push message for real-time broadcast.
  - Register new push type in the RPC dispatch table.
  - _Requirements: 6.1_
  - _writes: proto/sim.proto_
  - _Completed: Added channel bookmark proto model, mutation request messages, bookmark update push message, envelope tags, request mappings, and channel entity routing._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `git diff --check`._

- [x] 2. Create database migration
  - Add `channel_bookmarks` table with columns: `id`, `channel_id`, `label`, `description`, `bookmark_type`, `url`, `file_id`, `message_id`, `created_by`, `created_at`, `updated_at`, `sort_order`.
  - Add composite index on `(channel_id, sort_order)`.
  - Add `ON DELETE CASCADE` foreign key to `channels(id)`.
  - _Requirements: 6.1, 6.3_
  - _writes: migrations/XXXXXXXXXXXX_create_channel_bookmarks.up.sql_
  - _Completed: Added the `channel_bookmarks` migration with channel, creator, and optional message references plus the channel/order index._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `git diff --check`._

- [x] 3. Generate Rust protobuf types
  - Run the protobuf code generation step to produce Rust types for all new messages.
  - Ensure new types are re-exported from the `proto` crate.
  - _Requirements: 6.1_
  - _writes: proto/src/gen/sim.rs_
  - _Completed: Verified the repo's `prost` build generates and re-exports the new bookmark types from `OUT_DIR` during the proto crate build._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `git diff --check`._

- [x] 4. Implement server `BookmarkStore`
  - [x] 4.1 Implement `CreateBookmark` — insert row, return created bookmark with sort_order.
    - _Requirements: 6.1_
    - _writes: collab/src/db/bookmark_store.rs_
  - [x] 4.2 Implement `GetBookmarks` — select bookmarks for a channel ordered by `sort_order`.
    - _Requirements: 6.2_
    - _writes: collab/src/db/bookmark_store.rs_
  - [x] 4.3 Implement `UpdateBookmark` — update label/description for a bookmark.
    - _Requirements: 6.1_
    - _writes: collab/src/db/bookmark_store.rs_
  - [x] 4.4 Implement `DeleteBookmark` — delete a single bookmark by channel_id + bookmark_id.
    - _Requirements: 6.1_
    - _writes: collab/src/db/bookmark_store.rs_
  - [x] 4.5 Implement `ReorderBookmarks` — update sort_order for each bookmark based on position in an ordered ID list.
    - _Requirements: 6.3_
    - _writes: collab/src/db/bookmark_store.rs_
  - [x] 4.6 Implement `DeleteChannelBookmarks` — delete all bookmarks for a channel (used on channel deletion).
    - _Requirements: 6.1_
    - _writes: collab/src/db/bookmark_store.rs_
  - [x] 4.7 Write unit tests for all `BookmarkStore` methods: CRUD, reorder consistency, concurrent reorder, empty result sets.
    - _Requirements: 6.1, 6.3_
    - _writes: collab/src/db/bookmark_store.rs_
  - _Completed: Added `BookmarkStore`, `BookmarkId`, SeaORM `channel_bookmark` entity, SQLite test schema support, proto conversion, CRUD/list/reorder/delete-channel methods, and integration coverage for empty results, CRUD, repeated/concurrent reorder consistency, cross-channel reorder rejection, and channel cleanup._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab --features collab/test-support`; exact SQLite bookmark store tests passed. Full `test_bookmark_store` both-DB run failed only because local Postgres refused connections._

- [x] 5. Implement server-side permission checks
  - Add a helper that validates the requesting user has `Admin` or `Member` role in the target channel.
  - Wire the helper into each bookmark RPC handler before any store operation.
  - Guest role (and unauthenticated) requests receive `403 FORBIDDEN`.
  - _Requirements: 6.1 (AC 6)_
  - _writes: collab/src/rpc/bookmark_permissions.rs_
  - _Completed: Added a bookmark edit permission helper in the collab RPC layer and wired it before every bookmark mutation handler; Guest/Talker/Banned/non-members receive structured `Forbidden` errors._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p client -p collab --features collab/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_bookmark_rpc_flow_and_permissions --features collab/test-support -- --nocapture`._

- [x] 6. Implement server RPC handlers
  - [x] 6.1 Handle `AddBookmark` — validate permissions, call `BookmarkStore.CreateBookmark`, broadcast `UpdateChannelBookmarks`.
    - _Requirements: 6.1, 6.4_
    - _writes: collab/src/rpc/bookmark_rpc.rs_
  - [x] 6.2 Handle `RemoveBookmark` — validate permissions, call `BookmarkStore.DeleteBookmark`, broadcast `UpdateChannelBookmarks`.
    - _Requirements: 6.1, 6.4_
    - _writes: collab/src/rpc/bookmark_rpc.rs_
  - [x] 6.3 Handle `UpdateBookmark` — validate permissions, call `BookmarkStore.UpdateBookmark`, broadcast `UpdateChannelBookmarks`.
    - _Requirements: 6.1, 6.4_
    - _writes: collab/src/rpc/bookmark_rpc.rs_
  - [x] 6.4 Handle `ReorderBookmarks` — validate permissions, call `BookmarkStore.ReorderBookmarks`, broadcast `UpdateChannelBookmarks`.
    - _Requirements: 6.3, 6.4_
    - _writes: collab/src/rpc/bookmark_rpc.rs_
  - [x] 6.5 Write integration tests: full RPC flow for create → read → update → delete → reorder, with permission denial verification.
    - _Requirements: 6.1, 6.3_
    - _writes: collab/src/rpc/bookmark_rpc.rs_
  - _Completed: Registered and implemented add/remove/update/reorder bookmark RPC handlers, added `UpdateChannelBookmarks` push dispatch to channel participants, added a client-side bookmark update handler hook, and covered the flow through integration tests using push state as the readback path._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p client -p collab --features collab/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_bookmark_rpc_flow_and_permissions --features collab/test-support -- --nocapture`._

- [x] 7. Implement `UpdateChannelBookmarks` broadcast
  - After any successful bookmark mutation, build the `UpdateChannelBookmarks` message with current bookmarks and (for deletes) removed IDs.
  - Push to all connected channel members via the existing WebSocket broadcast mechanism.
  - Include a 200ms debounce to coalesce rapid reorder operations.
  - _Requirements: 6.4_
  - _writes: collab/src/rpc/bookmark_rpc.rs_
  - _Completed: Refactored bookmark push dispatch into a reusable latest-snapshot sender and added a per-channel 200ms reorder debounce that coalesces rapid reorder RPCs before broadcasting._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p client -p collab --features collab/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_bookmark_rpc_flow_and_permissions --features collab/test-support -- --nocapture`._

- [x] 8. Add client-side `Bookmark` model and RPC dispatch
  - [x] 8.1 Add `Bookmark` struct in the `client` crate mirroring the protobuf type, with conversions.
    - _Requirements: 6.1_
    - _writes: client/src/bookmark.rs_
  - [x] 8.2 Add methods to the RPC client: `add_bookmark`, `remove_bookmark`, `update_bookmark`, `reorder_bookmarks`, each constructing the proto request and calling the appropriate RPC.
    - _Requirements: 6.1_
    - _writes: client/src/rpc.rs_
  - [x] 8.3 Handle incoming `UpdateChannelBookmarks` push — update the local bookmarks state for the affected channel.
    - _Requirements: 6.4_
    - _writes: client/src/rpc.rs_
  - [x] 8.4 Add `BookmarkStore` (client-side) — reactive state holder for bookmarks per channel, observable by UI components.
    - _Requirements: 6.2_
    - _writes: collab_ui/src/channel_bookmark_store.rs_
  - _Completed: Added typed client bookmark IDs/model/conversions, bookmark mutation RPC helper methods, and a `ChannelBookmarkStore` that subscribes to `UpdateChannelBookmarks` pushes and exposes observable per-channel bookmark state._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p client -p collab_ui --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p client bookmark --features client/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui store_replaces_bookmarks_for_channel --features collab_ui/test-support`._

- [ ] 9. Implement `BookmarkBar` component
  - [x] 9.1 Render the bookmark bar at the top of the channel view (between channel header and message list).
    - _Requirements: 6.2 (AC 1)_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - [x] 9.2 Display bookmark entries: type icon, label, optional description. Truncate long labels.
    - _Requirements: 6.2 (AC 2)_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - [x] 9.3 Show bookmark count header with "Show all" expand/collapse toggle when >5 bookmarks.
    - _Requirements: 6.2 (AC 4)_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - [ ] 9.4 Handle click on link-type bookmarks — open URL in default browser. Handle file/message types — open in-app.
    - _Requirements: 6.2 (AC 3)_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - [x] 9.5 Wire `BookmarkBar` into the channel view, observe `ChannelBookmarkStore` for updates.
    - _Requirements: 6.2_
    - _writes: collab_ui/src/channel_view.rs_
  - [x] 9.6 Write UI tests: rendering with 0, 3, 8 bookmarks; expand/collapse behavior; click handling.
    - _Requirements: 6.2_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - _Completed: Added a compact bookmark bar component, mounted it above channel chat messages, observed `ChannelBookmarkStore` for live updates, and added collapsed/expanded rendering with a Show all/Show less toggle. Link bookmarks open through the platform URL handler; file/message click-through remains open in 9.4._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_bar --features collab_ui/test-support`._
  - _Partial: Added pure coverage for 0, 3, and 8 bookmark visible/hidden counts plus link/file/message click action selection. Message bookmarks now highlight loaded messages in the channel chat; file bookmark opening remains open until the file-sharing target surface exists._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_bar --features collab_ui/test-support`._
  - _Completed: The bookmark bar coverage exercises empty, small, collapsed, and expanded counts plus link/file/message click action routing._

- [x] 10. Implement `BookmarkForm` modal
  - [x] 10.1 Build create-mode form with fields: label (required), URL, type selector (Link/File/Message), description (optional).
    - _Requirements: 6.1 (AC 2, 4)_
    - _writes: collab_ui/src/channel_bookmark_form.rs_
  - [x] 10.2 Build edit-mode form pre-filled with existing bookmark data.
    - _Requirements: 6.1 (AC 5)_
    - _writes: collab_ui/src/channel_bookmark_form.rs_
  - [x] 10.3 Implement client-side validation: label is required, URL is required for link type, show inline error messages.
    - _Requirements: 6.1_
    - _writes: collab_ui/src/channel_bookmark_form.rs_
  - [x] 10.4 Wire form submission to the appropriate RPC call (create vs update).
    - _Requirements: 6.1 (AC 3)_
    - _writes: collab_ui/src/channel_bookmark_form.rs_
  - [x] 10.5 Write UI tests: form validation errors, successful create, successful edit, field population.
    - _Requirements: 6.1_
    - _writes: collab_ui/src/channel_bookmark_form.rs_
  - _Partial: Added an inline create form for link bookmarks with URL, label, and optional description fields; client-side link validation; submit/cancel states; and pure validation tests. Type selection, file/message create modes, edit mode, and end-to-end UI submission coverage remain open._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_form --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_bar --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_store --features collab_ui/test-support`._
  - _Completed: Added edit-mode form prefill, label/description update submission via `update_bookmark`, and shared validation for create/update paths._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_form --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_bar --features collab_ui/test-support`._
  - _Completed: Added create-mode type selection for Link/File/Message bookmarks, target placeholders for URL/File ID/Message ID, typed add-bookmark request generation, and validation for file/message targets._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_form --features collab_ui/test-support -- --nocapture`._
  - _Completed: Added editor-backed GPUI coverage for create validation, successful link/message create request generation, edit-mode field prefill, and update request generation._
  - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_form --features collab_ui/test-support -- --nocapture`._

- [x] 11. Implement bookmark context menu
  - [x] 11.1 Show context menu on hover/bookmark click with options: Edit, Delete.
    - _Requirements: 6.1 (AC 5)_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - [x] 11.2 Wire "Edit" to open `BookmarkForm` in edit mode.
    - _Requirements: 6.1_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - [x] 11.3 Wire "Delete" to call `remove_bookmark` RPC with confirmation dialog.
    - _Requirements: 6.1_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - _Partial: Added a compact delete action to each rendered bookmark, confirmation prompt, `remove_bookmark` RPC wiring, and inline delete failure feedback. Full hover/context-menu presentation and edit action remain open._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_bar --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_form --features collab_ui/test-support`._
  - _Completed: Added a compact edit action to each rendered bookmark and wired it to open `BookmarkForm` in edit mode._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_form --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_bar --features collab_ui/test-support`._
  - _Completed: Replaced the always-visible bookmark action buttons with an ellipsis dropdown context menu containing Edit and Delete actions._
  - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_bar --features collab_ui/test-support -- --nocapture`._

- [x] 12. Implement drag-and-drop reorder
  - [x] 12.1 Enable drag-and-drop on bookmark items in the `BookmarkBar` when user has edit permission.
    - _Requirements: 6.3 (AC 1)_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - [x] 12.2 On drop, build the reordered bookmark ID list and call `reorder_bookmarks` RPC.
    - _Requirements: 6.3 (AC 2)_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - [x] 12.3 Optimistically reorder the local bookmarks before the server confirms; roll back on error.
    - _Requirements: 6.3_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - [x] 12.4 Write UI tests: drag reorder triggers expected RPC call, optimistic update, rollback on failure.
    - _Requirements: 6.3_
    - _writes: collab_ui/src/channel_bookmark_bar.rs_
  - _Partial: Bookmark items are draggable and droppable onto other bookmark items; drops compute a reordered ID list, optimistically update `ChannelBookmarkStore`, call `reorder_bookmarks`, and restore the previous list with inline error feedback if the RPC fails. Full drag-event/RPC-failure UI coverage remains open in 12.4._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_bar --features collab_ui/test-support -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_store --features collab_ui/test-support -- --nocapture`._
  - _Completed: Added focused reorder coverage for the drop-derived RPC bookmark ID payload and for restoring the previous bookmark list after an optimistic reorder failure._
  - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_bar --features collab_ui/test-support -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_store --features collab_ui/test-support -- --nocapture`._

- [x] 13. Implement channel system messages for bookmark changes
  - [x] 13.1 Post an informational channel message when a bookmark is created (e.g., "Alice pinned a link: Deploy Guide").
    - _Requirements: 6.4 (AC 1)_
    - _writes: collab/src/rpc/bookmark_rpc.rs_
  - [x] 13.2 Post an informational channel message when a bookmark is deleted.
    - _Requirements: 6.4 (AC 2)_
    - _writes: collab/src/rpc/bookmark_rpc.rs_
  - [x] 13.3 Post an informational channel message when a bookmark is updated (label changed).
    - _Requirements: 6.4 (AC 3)_
    - _writes: collab/src/rpc/bookmark_rpc.rs_
  - [x] 13.4 Style informational bookmark messages distinctly from regular messages (e.g., italic, muted color, no avatar).
    - _Requirements: 6.4 (AC 4)_
    - _writes: collab_ui/src/channel_chat.rs_
  - _Partial: Bookmark create, update, and delete operations now post channel messages and broadcast them to channel participants. The messages currently use the existing channel message model/presentation, so distinct visual styling remains open in 13.4._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab --features collab/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_bookmark_rpc_flow_and_permissions --features collab/test-support -- --nocapture`._
  - _Completed: Bookmark activity messages now render as compact muted italic rows with a pin icon and no sender/reply chrome in the channel message list._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui bookmark_system_message_detection_is_narrow --features collab_ui/test-support -- --nocapture`._

- [x] 14. Load bookmarks on channel open
  - [x] 14.1 When a channel is opened, fetch existing bookmarks via `GetBookmarks` (or via the initial channel state payload).
    - _Requirements: 6.2 (AC 5)_
    - _writes: client/src/rpc.rs_
  - [x] 14.2 Populate `ChannelBookmarkStore` with the loaded bookmarks; `BookmarkBar` renders them on next frame.
    - _Requirements: 6.2_
    - _writes: collab_ui/src/channel_bookmark_store.rs_
  - _Completed: Added a `GetBookmarks` RPC, server read handler with channel-view permission checks, typed client helper, and channel-chat startup load into `ChannelBookmarkStore` so persisted bookmarks render on open._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab -p collab_ui --features collab/test-support,collab_ui/test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_bookmark_rpc_flow_and_permissions --features collab/test-support -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_bookmark_store --features collab_ui/test-support`._

- [x] 15. Add end-to-end tests
  - [x] 15.1 Full lifecycle test: create bookmark → verify it renders in another client → delete → verify removal in real-time.
    - _Requirements: 6.1, 6.4_
    - _writes: collab/tests/bookmark_e2e.rs_
  - [x] 15.2 Permission enforcement test: guest user attempts add/edit/delete → receives 403.
    - _Requirements: 6.1 (AC 6)_
    - _writes: collab/tests/bookmark_e2e.rs_
  - [x] 15.3 Real-time sync test: two users in same channel; one reorders → the other sees updated order.
    - _Requirements: 6.3_
    - _writes: collab/tests/bookmark_e2e.rs_
  - [x] 15.4 Concurrent reorder test: two admins reorder simultaneously → verify consistent final state.
    - _Requirements: 6.3_
    - _writes: collab/tests/bookmark_e2e.rs_
  - _Completed: Extended the channel chat integration bookmark flow to cover cross-client create/update/reorder/delete push behavior, guest add/update/reorder/delete denial, persisted readback, and concurrent two-client reorder consistency._
  - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_bookmark_rpc_flow_and_permissions --features collab/test-support -- --nocapture`._
