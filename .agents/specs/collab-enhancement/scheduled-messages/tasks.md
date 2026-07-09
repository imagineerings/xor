# Implementation Plan: Scheduled Messages

## Overview

Add the ability for channel participants to schedule messages for future delivery (1 minute to 30 days ahead). This spans protobuf definitions, a new DB table, a server-side `ScheduledMessageStore` + polling scheduler loop, and client UI for composing (SchedulePicker), managing (ScheduledMessagesPanel), and surfacing scheduled messages (sidebar badge, notifications). Timezone handling is client-side only; the server stores and compares UTC timestamps.

**Root crates involved:** `proto`, `db` / `migrations`, `collab`, `client`, `collab_ui`, `rpc`.

---

## Tasks

- [x] 1. Define protobuf messages and register in the message framework
    - Add `ScheduleChannelMessage`, `ScheduleChannelMessageResponse`, `CancelScheduledMessage`, `UpdateScheduledMessage`, `GetScheduledMessages`, `GetScheduledMessagesResponse`, `ScheduledMessage`, `ScheduledMessageSent`, `ScheduledMessageFailed` to `channel.proto`.
    - Add `scheduled_at` (optional uint64) to the existing `ChannelMessage` proto.
    - Register all new request messages in the `messages!` macro (background-priority) and push messages in `entity_messages!` (`ScheduledMessageSent`, `ScheduledMessageFailed`).
    - _Requirements: 11.1, 11.2, 11.3_
    - _writes: proto/src/channel.proto, proto/src/proto.rs_
    - _Completed: Added scheduled-message proto request/response/push messages, registered request mappings, and added `ChannelMessage.scheduled_at` with DB-to-proto hydration._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `git diff --check`._

- [x] 2. Add database migration for the `scheduled_messages` table
    - Create a new migration file with the `CREATE TABLE scheduled_messages (...)` DDL including all columns: `id`, `channel_id` (FK → `channels(id)` ON DELETE CASCADE), `sender_id`, `body`, `scheduled_at`, `created_at`, `state` (SMALLINT), `nonce`, `mentions` (JSONB), `delivered_message_id`, `failure_reason`, `updated_at`.
    - Add partial indexes: `(state, scheduled_at) WHERE state = 0` and `(sender_id, channel_id) WHERE state = 0`.
    - Add a unique index on `(channel_id, sender_id, nonce)` for idempotent scheduling.
    - _Requirements: 11.1, 11.2_
    - _writes: db/migrations/*_scheduled_messages.sql_
    - _Completed: Added Postgres and SQLite scheduled-message schema, indexes, idempotency constraint, and `channel_messages.scheduled_at` column._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `git diff --check`._

- [x] 3. Implement `ScheduledMessageStore` (server-side data access layer)
    - Create `ScheduledMessageStore` with `db: Arc<Database>`.
    - Implement `create` — validates channel membership server-side, inserts row, returns `ScheduledMessageId`.
    - Implement `cancel` — `DELETE WHERE id = ? AND sender_id = ?`; only the sender may cancel (returns `Result<Option<ScheduledMessage>>`).
    - Implement `update` — updates body and/or `scheduled_at` (re-validates time bounds); rejects if `state != pending`.
    - Implement `list_for_user` — `SELECT ... WHERE sender_id = ? AND channel_id = ? AND state = 0 ORDER BY scheduled_at`.
    - Implement `pop_due` — atomic `UPDATE ... SET state = 1 WHERE state = 0 AND scheduled_at <= NOW() RETURNING *` (advisory-lock style); returns `Vec<ScheduledMessage>`.
    - Implement `delete_delivered` — deletes row after successful send.
    - Implement `count_pending_for_user` — `SELECT COUNT(*) WHERE sender_id = ? AND state = 0`.
    - Implement `mark_failed` — sets `state = 3`, `failure_reason`.
    - Implement `reset_stale_processing` — resets `state = 0` for rows with `state = 1` and `updated_at` older than a grace period (for crash recovery).
    - _Requirements: 11.1, 11.2, 11.3, 11.4_
    - _writes: collab/src/db/scheduled_message_store.rs_
    - _Completed: Added `ScheduledMessageStore`, table entity, `ScheduledMessageId`, CRUD/list/count/failure/recovery methods, time-bound validation, nonce hydration, JSON mention persistence, and atomic due-message popping._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `git diff --check`._

- [x] 4. Implement server RPC handlers
    - Register handlers in `Server::new()`: `schedule_channel_message`, `cancel_scheduled_message`, `update_scheduled_message`, `get_scheduled_messages`.
    - `schedule_channel_message`: extract params, validate time bounds (≥1 min, ≤30 days), check channel membership/permissions, delegate to `ScheduledMessageStore::create`, return the new ID.
    - `cancel_scheduled_message`: verify sender owns the message via `store.cancel`, return ack.
    - `update_scheduled_message`: delegate to `store.update`; re-validate time bounds if `scheduled_at` is being changed.
    - `get_scheduled_messages`: delegate to `store.list_for_user`.
    - _Requirements: 11.1, 11.2, 11.4_
    - _writes: collab/src/rpc/scheduled_messages.rs_ (new file or inline in existing handler module)
    - _Completed: Registered inline RPC handlers for schedule/cancel/update/list, converted scheduled timestamps from Unix milliseconds, delegated validation and ownership checks to `ScheduledMessageStore`, and returned proto responses._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `git diff --check`._

- [x] 5. Implement the `SchedulerLoop` background task
    - In `Server::start()`, spawn a detached task that runs every 10 seconds.
    - On tick: call `store.pop_due()`, then for each due message execute `deliver_scheduled_message`.
    - Delivery function:
        - [x] 5.1 Re-validate sender's channel membership and `can_send_message` permission. If either fails, mark as failed and push `ScheduledMessageFailed` to the sender.
        - [x] 5.2 Insert the message as a regular `channel_messages` row via the existing `insert_channel_message` DB function.
        - [x] 5.3 Build a `ChannelMessage` proto with the `scheduled_at` field populated.
        - [x] 5.4 Broadcast `ChannelMessageSent` to all connected channel members.
        - [x] 5.5 Send `ScheduledMessageSent` push specifically to the sender's connection(s).
        - [x] 5.6 Call `store.delete_delivered()` on success.
    - On start-up, call `store.reset_stale_processing()` to recover from any crash mid-delivery.
    - _Requirements: 11.1.4, 11.2.4, 11.3.2_
    - _writes: collab/src/scheduler_loop.rs_ (new file)
    - _Completed: Added the scheduler loop inline with server startup, reset stale processing rows on start, popped due messages every 10 seconds, delivered via existing channel-message insertion with `scheduled_at`, broadcast channel sends, notified senders of success/failure, and deleted delivered rows._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `git diff --check`._

- [x] 6. Add client-side Rust data models
    - Define `ScheduledMessageId(u64)` newtype with proto conversion helpers.
    - Define `ScheduledMessage` struct: `id`, `channel_id`, `sender_id`, `body`, `scheduled_at` (UTC), `created_at`, `mentions`, `display_time` (computed local time).
    - Implement `TryFrom<proto::ScheduledMessage>` and `Into<proto::ScheduledMessage>` conversions.
    - Add a `scheduled_at: Option<DateTime<Utc>>` field to the client's `ChannelMessage` model.
    - _Requirements: 11.1, 11.2, 11.4_
    - _writes: client/src/scheduled_message.rs_
    - _Completed: Added scheduled-message and channel-message client models with UTC/local time conversion, scheduled-message ID helpers, proto conversions, and client RPC helpers for schedule/cancel/update/list._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `CARGO_INCREMENTAL=0 cargo test -p client test_channel_chat -- --nocapture`; `git diff --check`._

- [x] 7. Build `SchedulePicker` compose-area widget
    - Implement `SchedulePicker` struct with `selected_date`, `selected_time`, `scheduled_at` (local time), `timezone`, `popover_visible`.
    - Implement `scheduled_at_utc()` — converts local selection to UTC.
    - Implement `render()` — small button next to the send button that toggles a popover.
    - Implement `render_calendar()` — a month-grid calendar for date selection.
    - Implement `render_time_picker()` — hour:minute selection (dropdowns or sliders).
    - Implement `validate()` — ensures selected time is ≥1 minute from now.
    - Wire the picker into the existing compose area: when a time is selected, the send button label changes to "Schedule (time)" with a clock icon; clicking sends a `ScheduleChannelMessage` RPC instead of an immediate message.
    - _Requirements: 11.1.1, 11.1.2, 11.3.3, 11.4.1_
    - _writes: collab_ui/src/schedule_picker.rs_
    - _Completed: Added inline `SchedulePicker` state with local date/time selection, UTC conversion, month-grid calendar controls, hour/minute picker controls, lead-time validation, selected-time display, draft clearing, and scheduled-send RPC wiring in the compose area._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui`; `git diff --check`. Attempted `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_chat_key_bindings_parse -- --nocapture`, blocked by unrelated `remote_connection` test-support compile error for non-exhaustive `RemoteConnectionOptions::Mock(_)` handling._

- [x] 8. Build `ScheduledMessagesPanel` management view
    - Implement `ScheduledMessagesPanel` struct with `channel_id`, `messages`, `loading`, `editing_message_id`, `edit_body`, `edit_scheduled_at`.
    - Implement `refresh()` — calls `GetScheduledMessages` RPC and populates the list.
    - Implement `render()` — renders as a sliding panel or modal with a list of pending messages grouped by date, each showing scheduled time, body preview, and [Edit] [Cancel] buttons.
    - Implement `start_edit()` — populates edit fields from the selected message.
    - Implement `save_edit()` — calls `UpdateScheduledMessage` RPC and refreshes.
    - Implement `confirm_cancel()` — shows a confirmation dialog, then calls `CancelScheduledMessage` RPC.
    - Implement `on_message_sent()` — removes a message from the list when `ScheduledMessageSent` arrives.
    - Add a "Scheduled" entry in the channel sidebar or header that opens this panel.
    - Handle empty state (no scheduled messages) with a helpful message.
    - _Requirements: 11.2.1, 11.2.2, 11.2.3, 11.2.4_
    - _writes: collab_ui/src/channel_chat.rs, collab_ui/src/channel_chat/search.rs_
    - _Completed: Added a channel-header Scheduled drawer that loads pending scheduled messages, groups them by local date, renders loading/error/empty states, supports inline body and date/time edits through the existing scheduled-message RPCs, confirms cancellation, refreshes after saves, and removes cancelled messages from the panel._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui`; `git diff --check`._

- [x] 9. Implement sidebar badge for pending scheduled messages
    - On app startup and after relevant RPCs, fetch pending count via `GetScheduledMessages` (or a lightweight count endpoint).
    - Store a `pending_scheduled_count: usize` in the relevant app state.
    - Show a badge (clock icon + count) next to the "Scheduled" entry in the channel sidebar.
    - Update the badge when messages are scheduled, delivered, or cancelled.
    - _Requirements: 11.3.1_
    - _writes: collab_ui/src/channel_chat.rs, collab_ui/src/channel_chat/search.rs_
    - _Completed: Added `pending_scheduled_count` to channel chat state, fetch it on chat load via `GetScheduledMessages`, refresh it from scheduled-panel loads, update it after schedule/deliver/fail/cancel events, and render the count beside the channel-header Scheduled control._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui`; `git diff --check`._

- [x] 10. Wire client-side notifications for `ScheduledMessageSent` and `ScheduledMessageFailed`
    - Register handlers for the two push messages in the WebSocket message dispatcher.
    - On `ScheduledMessageSent`: show a toast notification ("Your scheduled message was sent to #channel"), remove the message from the `ScheduledMessagesPanel` list if open, update sidebar badge count.
    - On `ScheduledMessageFailed`: show a toast notification with the failure reason and an action to review/edit the message if still pending.
    - _Requirements: 11.3.2_
    - _writes: client/src/channel_chat.rs, collab_ui/src/channel_chat.rs_
    - _Completed: Added typed client subscription helpers for scheduled-message sent/failed pushes, registered channel chat handlers, show workspace toasts for success/failure, remove affected pending messages from the scheduled panel, upsert delivered scheduled messages, and provide a Review toast action that opens and refreshes the scheduled panel._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p client -p collab_ui`; `git diff --check`._

- [ ] 11. Add server-side unit and integration tests
    - [ ] 11.1 `ScheduledMessageStore` unit tests:
        - `create` — validates time bounds (rejects <1 min and >30 days).
        - `create` — nonce deduplication returns existing ID.
        - `cancel` — rejects cancel from non-owner; succeeds for owner.
        - `cancel` — idempotent on already-sent message.
        - `update` — rejects update on non-pending message.
        - `update` — re-validates time bounds on `scheduled_at` change.
        - `pop_due` — atomic state transition, no double-pop.
        - `count_pending_for_user` — correct per-user counts.
        - `reset_stale_processing` — resets stale `processing` rows to `pending`.
    - [ ] 11.2 Integration tests (using test server harness):
        - Schedule → confirm row in DB → advance clock → confirm message delivered → confirm row deleted.
        - Schedule → cancel → confirm row deleted and message never delivered.
        - Schedule → sender loses member role before delivery → confirm `ScheduledMessageFailed` push sent.
        - Multiple due messages at same timestamp → confirm all delivered in order.
        - Server restart with stale `processing` rows → confirm they're reset and re-delivered.
        - Concurrent cancel + delivery race → confirm at-most-once delivery.
    - _Requirements: 11.1, 11.2, 11.3, 11.4_
    - _writes: collab/src/db/scheduled_message_store.rs (tests module), collab/tests/scheduled_messages_integration.rs_

- [ ] 12. Add client-side UI tests
    - `SchedulePicker` rendering tests: calendar grid renders, time picker renders, validation error shows for invalid times.
    - Send button label changes from "Send" to "Schedule (…)" when time is selected.
    - `ScheduledMessagesPanel` tests: empty state, list with items, edit dialog opens and saves, cancel confirmation works.
    - Sidebar badge count matches pending messages.
    - Timezone display test: client stores UTC but renders in local timezone.
    - _Requirements: 11.1.1, 11.2.1, 11.3.1, 11.3.3, 11.4_
    - _writes: collab_ui/src/schedule_picker.rs (tests module), collab_ui/src/scheduled_messages_panel.rs (tests module)_

- [x] 13. Implement error handling edge cases (server-side)
    - Validation in `schedule_channel_message`: return descriptive errors for out-of-bounds `scheduled_at`, missing channel membership, insufficient permissions.
    - Server restart recovery: in scheduler startup, reset `processing` → `pending` for rows with `updated_at` older than a configurable grace period (e.g., 60 seconds).
    - Concurrent cancel + delivery: the atomic `UPDATE ... WHERE state = pending` ensures only one wins; handle the case where cancel finds no rows (idempotent).
    - Edit-after-delivery: `store.update` checks `state != pending` and rejects with "message already delivered".
    - Nonce fallback: if `nonce` is absent in the request, generate one server-side with a warning log.
    - Register all new error variants for proper client-side display.
    - _Requirements: 11.1, 11.2, 11.3_
    - _writes: collab/src/rpc.rs, collab/src/db/scheduled_message_store.rs_
    - _Completed: Confirmed existing time-bound validation, stale-processing recovery, atomic pop/cancel behavior, and pending-only update checks; added server-side fallback nonce generation with warning logging and clearer update errors for deleted/delivered, processing, and failed scheduled messages._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `git diff --check`._

- [x] 14. Update `ChannelMessage` model and rendering for the `scheduled_at` label
    - Parse the `scheduled_at` field from `ChannelMessage` proto on the client.
    - When rendering a channel message, if `scheduled_at` is present, show a small "Scheduled" label/tooltip next to the timestamp.
    - _Requirements: 11.1.4_
    - _writes: client/src/scheduled_message.rs, collab_ui/src/channel_chat.rs_
    - _Completed: `ChannelMessage` already parsed `scheduled_at`; added a muted clock label with local scheduled time and tooltip beside rendered channel-message timestamps._
    - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui`; `git diff --check`._
