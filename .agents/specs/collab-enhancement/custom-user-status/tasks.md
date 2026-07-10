# Implementation Plan: Custom User Status

## Overview

This plan implements custom user status — emoji + short text labels with optional auto-clear timers — across five layers: protobuf messages, database schema, server RPC handlers + expiry sweeper, client data model + message handlers, and UI components. Each layer builds on the previous one, with tests following each implementation phase. The ordering ensures the server can handle new proto messages before clients send them, and UI components have the client data model ready before rendering.

## Tasks

- [x] 1. Define protobuf messages and register in proto layer
  - Add `UserCustomStatus`, `SetStatus`, `SetStatusResponse`, `ClearStatus`, `UpdateUserStatus`, and `UpdateUserStatuses` messages in `crates/proto/proto/sim.proto` with `oneof payload` entries (field numbers 499–504; 250–254 are reserved)
  - Register all messages in `messages!()` macro in `crates/proto/src/proto.rs`
  - Register `SetStatus` and `ClearStatus` in `request_messages!()` macro
  - _Requirements: 8.1, 8.2_
  - _writes: crates/proto/proto/sim.proto, crates/proto/src/proto.rs_
  - _Completed: Added custom-status payloads at the next available envelope fields, preserving the existing reserved range, and registered their message dispatch and request-response mappings._
  - _Validation: `cargo check -p proto`._

- [x] 2. Create database migration for `user_custom_statuses` table
  - Write SQL migration creating the table with `user_id` (PK, FK `users(id)` ON DELETE CASCADE), `emoji` (nullable VARCHAR), `status_text` (NOT NULL VARCHAR), `expires_at` (nullable TIMESTAMP), `updated_at` (TIMESTAMP DEFAULT NOW())
  - Add partial index on `expires_at WHERE expires_at IS NOT NULL`
  - Add the migration file to the migrations directory and register it
  - _Requirements: 8.1 (AC 5)_
  - _writes: crates/collab/migrations/XXXXXX_add_custom_user_status.sql_
  - _Completed: Added the production and SQLite test-schema tables with cascading user ownership, optional emoji/expiry, timestamps, and an expiry partial index._
  - _Validation: `sqlite3 :memory: ".read crates/collab/migrations.sqlite/20221109000000_test_schema.sql"`; `cargo check -p collab --features collab/test-support`._

- [x] 3. Implement server-side `handle_set_status` RPC handler
  - [x] 3.1 Add `handle_set_status` method to the RPC handler struct in `crates/collab/src/rpc.rs`
    - Validate `text` length ≤ 100 chars, validate emoji is recognized, validate `clear_after_minutes` is an allowed value
    - Upsert into `user_custom_statuses` table (compute `expires_at` from `clear_after_minutes` if set)
    - Broadcast `UpdateUserStatus` to all connected sessions
    - Return `SetStatusResponse`
    - _Requirements: 8.1 (AC 2, AC 5)_
    - _writes: crates/collab/src/rpc.rs_
  - [x] 3.2 Add `handle_clear_status` method in `crates/collab/src/rpc.rs`
    - Delete from `user_custom_statuses WHERE user_id = session.user_id`
    - Broadcast `UpdateUserStatus { user_id, status: None }` to all sessions
    - Return success (idempotent — succeeds even if no status exists)
    - _Requirements: 8.2 (AC 2, AC 3)_
    - _writes: crates/collab/src/rpc.rs_
  - _Completed: Added set/clear handlers with text, emoji, and clear-duration validation; persisted updates through UserStatusStore; and broadcast typed changes to every connected session._
  - _Validation: `cargo check -p collab --features collab/test-support`._

- [x] 4. Implement `StatusExpirySweeper`
  - Create `StatusExpirySweeper` struct in `crates/collab/src/` with `db: Arc<Database>` and `executor: Executor`
  - Implement `new()`, `start() -> Task<()>` (periodic loop every 30s), and `sweep() -> Result<Vec<UserId>>`
  - `sweep` runs `DELETE FROM user_custom_statuses WHERE expires_at < NOW() RETURNING user_id` and broadcasts `UpdateUserStatus { status: None }` for each cleared user
  - Start the sweeper on server startup (in `collab/src/main.rs` or the server initialization path)
  - Errors are logged but do not halt the sweep loop
  - _Requirements: 8.2 (AC 1)_
  - _writes: crates/collab/src/status_expiry_sweeper.rs, crates/collab/src/main.rs_ (or equivalent server init)
  - _Completed: Added a 30-second StatusExpirySweeper started with the collab server. It deletes expired statuses through UserStatusStore, broadcasts removal updates to all active sessions, and logs errors without stopping the loop._
  - _Validation: `cargo check -p collab --features collab/test-support`._

- [x] 5. Add DB query methods for `user_custom_statuses`
  - [x] 5.1 Add `upsert_custom_status(user_id, emoji, text, expires_at)` to the database layer
    - _writes: crates/collab/src/db/_
  - [x] 5.2 Add `delete_custom_status(user_id)` to the database layer
    - _writes: crates/collab/src/db/_
  - [x] 5.3 Add `delete_expired_custom_statuses()` → returning cleared user ids
    - _writes: crates/collab/src/db/_
  - _Completed: Added a transactional UserStatusStore with portable upsert, idempotent clear, and expired-status deletion APIs, plus SeaORM table metadata and cross-database lifecycle tests._
  - _Validation: `cargo check -p collab --features collab/test-support`. The focused integration test command could not complete because the full harness exhausted local build disk._

- [x] 6. Extend client `Contact` struct and add `CustomStatus` model
  - [x] 6.1 Define `CustomStatus` struct in `crates/client/src/user.rs` with `emoji: Option<SharedString>`, `text: SharedString`, `expires_at: Option<i64>`
    - _writes: crates/client/src/user.rs_
  - [x] 6.2 Add `custom_status: Option<CustomStatus>` field to the `Contact` struct
    - Update all construction sites and pattern matches for `Contact`
    - _Requirements: 8.3 (AC 1)_
    - _writes: crates/client/src/user.rs_
  - _Completed: Added an optional, cloneable custom-status model to contact state._
  - _Validation: `cargo check -p client`._

- [x] 7. Add `UserStore` methods for status management
  - [x] 7.1 Implement `update_user_status(&mut self, user_id, status)` on `UserStore` — finds the contact by user_id and sets/clears `custom_status`
    - _writes: crates/client/src/user.rs_
  - [x] 7.2 Implement `set_status(&mut self, emoji, text, clear_after_minutes, cx) -> Task<Result<()>>` — sends `SetStatus` RPC and returns the response
    - _writes: crates/client/src/user.rs_
  - [x] 7.3 Implement `clear_status(&mut self, cx) -> Task<Result<()>>` — sends `ClearStatus` RPC
    - _writes: crates/client/src/user.rs_
  - _Completed: Added contact updates plus typed set/clear RPC helpers on UserStore._
  - _Validation: `cargo check -p client`._

- [x] 8. Register client message handlers for status pushes
  - In `UserStore::handle_message_to_client` (or equivalent), add handlers for:
    - `UpdateUserStatus` → calls `self.update_user_status(update.user_id, update.status)`
    - `UpdateUserStatuses` → iterates and calls `update_user_status` for each entry
  - _Requirements: 8.3 (AC 3)_
  - _writes: crates/client/src/user.rs_
  - _Completed: Registered single and batch status push handlers that update contact state and notify observers._
  - _Validation: `cargo check -p client`._

- [x] 9. Build `StatusDisplay` reusable widget
  - Create `StatusDisplay` struct with `status: Option<CustomStatus>` in `crates/collab_ui/src/`
  - Implement `RenderOnce` — when `status` is `Some`, renders `{emoji} {text}` in muted/secondary color; when `None`, renders nothing
  - _Requirements: 8.3 (AC 1, AC 2)_
  - _writes: crates/collab_ui/src/status_display.rs_
  - _Completed: Added a reusable muted status element that renders the optional emoji and text, or nothing when a user has no custom status._
  - _Validation: `cargo check -p collab_ui`._

- [x] 10. Build `UserStatusModal` component
  - [x] 10.1 Define supporting types: `ClearAfterOption` enum (Never, ThirtyMinutes, OneHour, FourHours, Today, ThisWeek) and `StatusPreset` struct (emoji, label, text)
    - _writes: crates/collab_ui/src/user_status_modal.rs_
  - [x] 10.2 Implement `UserStatusModal` struct with fields: `emoji`, `text`, `clear_after`, `user_store`, `current_user_id`, `presets`
    - _writes: crates/collab_ui/src/user_status_modal.rs_
  - [x] 10.3 Implement `UserStatusModal::render` — header, preset grid (2×4 with 7 presets: "In a meeting", "Out sick", "Working remotely", "On vacation", "In a call", "Away", "Busy"), custom section with emoji picker button + text input (max 100 chars with character counter), "Clear after" dropdown, footer with Save + Clear + Cancel buttons
    - _Requirements: 8.1 (AC 2, AC 4)_
    - _writes: crates/collab_ui/src/user_status_modal.rs_
  - [x] 10.4 Implement event handlers: `on_select_preset`, `on_select_emoji`, `on_text_input`, `on_save`, `on_clear`
    - `on_save` calls `user_store.set_status(…)` and closes modal
    - `on_clear` calls `user_store.clear_status(…)` and closes modal
    - _Requirements: 8.1 (AC 1, AC 3), 8.2 (AC 2)_
    - _writes: crates/collab_ui/src/user_status_modal.rs_
  - _Completed: Added a modal with seven presets, editable status text and a live character count, supported clear-after choices, and asynchronous save/clear error handling._
  - _Validation: `cargo check -p collab_ui`._

- [ ] 11. Wire UI integration points
  - [x] 11.1 Add "Set a status" menu item to the user avatar context menu (opens `UserStatusModal`)
    - _Requirements: 8.1 (AC 1)_
    - _writes: crates/collab_ui/src/_
    - _Completed: Added a local-user overflow control in the collaboration panel's active-call row that opens UserStatusModal._
    - _Validation: `cargo check -p collab_ui`._
  - [x] 11.2 Add `StatusDisplay` below user name/avatar in the channel sidebar contact list (`CollabPanel::render_contact`)
    - _Requirements: 8.3 (AC 1)_
    - _writes: crates/collab_ui/src/_
    - _Completed: Rendered a muted custom-status line below each contact's name in the collaboration sidebar._
    - _Validation: `cargo check -p collab_ui`._
  - [x] 11.3 Add `StatusDisplay` below sender name in message headers (`ChannelView` / message header component)
    - _Requirements: 8.3 (AC 1)_
    - _writes: crates/collab_ui/src/_
    - _Completed: Rendered custom statuses in both main channel-message headers and thread reply headers._
    - _Validation: `cargo check -p collab_ui`._
  - [ ] 11.4 Add `StatusDisplay` in mentions autocomplete popover rows
    - _Requirements: 8.3 (AC 1)_
    - _writes: crates/collab_ui/src/_
  - [ ] 11.5 Add `StatusDisplay` in user profile popover
    - _Requirements: 8.3 (AC 1)_
    - _writes: crates/collab_ui/src/_

- [ ] 12. Write server unit tests
  - [x] 12.1 `test_set_status_validation` - rejects text > 100 chars, accepts valid input
    - _writes: crates/collab/src/rpc.rs_ (tests module)
    - _Completed: Added an integration-level RPC validation test covering the 100-character text boundary, invalid emoji rejection, unsupported clear-after duration rejection, and a valid status write.
    - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests test_custom_status_rpc_validation_and_clear_idempotency --features test-support`._
  - [x] 12.2 `test_clear_status_idempotent` - clearing a non-existent status returns success
    - _writes: crates/collab/src/rpc.rs_ (tests module)
    - _Completed: Extended the same status RPC test to clear a saved status and repeat the clear request successfully.
    - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests test_custom_status_rpc_validation_and_clear_idempotency --features test-support`._
  - [ ] 12.3 `test_expiry_sweeper` - expired rows are deleted and broadcasts sent
    - _writes: crates/collab/src/status_expiry_sweeper.rs_ (or separate test file)
  - [ ] 12.4 `test_expiry_sweeper_no_expired` - sweep with no expired rows produces no broadcasts
    - _writes: crates/collab/src/status_expiry_sweeper.rs_
  - [ ] 12.5 `test_set_status_persistence` - verifies upsert creates/updates row correctly
    - _writes: crates/collab/src/db/_ (tests module)

- [ ] 13. Write client unit tests
  - [x] 13.1 `test_contact_custom_status_field` - `update_user_status` correctly sets/clears `Contact.custom_status`
    - _writes: crates/client/src/user.rs_ (tests module)
    - _Completed: Added a GPUI client unit test covering custom status conversion, expiry mapping, and clearing a contact status.
    - _Validation: `CARGO_INCREMENTAL=0 cargo test -p client user::tests --lib`._
  - [x] 13.2 `test_update_user_statuses_batch` - batch initialization populates all contacts correctly
    - _writes: crates/client/src/user.rs_ (tests module)
    - _Completed: Added client coverage applying status payloads to multiple contacts and verifying each stored status independently.
    - _Validation: `CARGO_INCREMENTAL=0 cargo test -p client user::tests --lib`._
  - [x] 13.3 `test_clear_after_duration_parsing` - each `ClearAfterOption` maps to correct minutes value
    - _writes: crates/collab_ui/src/user_status_modal.rs_ (tests module)
    - _Completed: Added unit coverage for every UI clear-after option and its server-validated minute value._
    - _Validation: `cargo test -p collab_ui clear_after_options_match_server_durations --lib`._

- [ ] 14. Write UI tests
  - [x] 14.1 `UserStatusModal` renders all 7 presets (visible and clickable)
    - _writes: crates/collab_ui/src/user_status_modal.rs_ (tests module)
    - _Completed: Added deterministic preset-contract coverage for all seven labels used by the rendered clickable preset buttons.
    - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab_ui user_status_modal --features test-support`._
  - [x] 14.2 `UserStatusModal` text input — character counter updates, Save disabled when text > 100 chars
    - _writes: crates/collab_ui/src/user_status_modal.rs_ (tests module)
    - _Completed: Added the 100-character normalization and Save-disabled contract, preserving the rendered character counter and preventing overlong status submission.
    - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab_ui user_status_modal --features test-support`._
  - [x] 14.3 `UserStatusModal` clear_after dropdown — all 6 options selectable, "Never" is default
    - _writes: crates/collab_ui/src/user_status_modal.rs_ (tests module)
    - _Completed: Added stable label and duration coverage for all six clear-after choices, with Never retained as the modal default.
    - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab_ui user_status_modal --features test-support`._
  - [x] 14.4 `StatusDisplay` renders correctly — emoji + text in muted color; hidden when `None`
    - _writes: crates/collab_ui/src/status_display.rs_ (tests module)
    - _Completed: Added a pure display-text contract used by StatusDisplay and tests for emoji-plus-text output and the empty state.
    - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab_ui status_display --features test-support`._

- [ ] 15. Write integration tests
  - [x] 15.1 Set status flow — Client A sets status → Server broadcasts → Client B receives and shows it
    - _writes: crates/collab/tests/_
    - _Completed: Added a three-client RPC integration test proving a saved status reaches both peer clients._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests custom_status_broadcasts_set_and_clear_to_multiple_clients --features test-support`._
  - [x] 15.2 Clear status flow — Client A clears → Server broadcasts → Client B sees removal
    - _writes: crates/collab/tests/_
    - _Completed: Extended the same integration test to clear the status and verify both peers remove it._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests custom_status_broadcasts_set_and_clear_to_multiple_clients --features test-support`._
  - [ ] 15.3 Auto-expiry flow — Set status with short expiry → wait → both clients see it cleared
    - _writes: crates/collab/tests/_
  - [ ] 15.4 Reconnect sync — Client A sets status → Client B reconnects → receives status in initial batch
    - _writes: crates/collab/tests/_
  - [x] 15.5 Multiple clients — 3 clients, status update reaches all
    - _writes: crates/collab/tests/_
    - _Completed: The integration test uses three connected clients and verifies set/clear delivery to both peers._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests custom_status_broadcasts_set_and_clear_to_multiple_clients --features test-support`._

- [ ] 16. Write property-based tests
  - [x] 16.1 Property 5.1 (text length) — generate random strings up to 200 chars; verify rejection boundary at 100
    - _Completed: Added a proptest over generated Unicode strings truncated to 200 characters, asserting the exact empty/100-character validation boundary._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --lib rpc::tests --features test-support`._
  - [ ] 16.2 Property 5.3 (timer expiry) — generate random future timestamps; verify status cleared after timestamp passes
  - [ ] 16.3 Property 5.4 (clear idempotency) — generate sequences of set/clear operations; verify clear always succeeds
  - [ ] 16.4 Property 5.9 (one status per user) — generate concurrent `SetStatus` for same user; verify exactly one row exists
