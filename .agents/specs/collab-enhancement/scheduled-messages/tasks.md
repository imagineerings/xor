# Implementation Plan: Scheduled Messages

## Overview

Add the ability for channel participants to schedule messages for future delivery (1 minute to 30 days ahead). This spans protobuf definitions, a new DB table, a server-side `ScheduledMessageStore` + polling scheduler loop, and client UI for composing (SchedulePicker), managing (ScheduledMessagesPanel), and surfacing scheduled messages (sidebar badge, notifications). Timezone handling is client-side only; the server stores and compares UTC timestamps.

**Root crates involved:** `proto`, `db` / `migrations`, `collab`, `client`, `collab_ui`, `rpc`.

---

## Tasks

- [ ] 1. Define protobuf messages and register in the message framework
    - Add `ScheduleChannelMessage`, `ScheduleChannelMessageResponse`, `CancelScheduledMessage`, `UpdateScheduledMessage`, `GetScheduledMessages`, `GetScheduledMessagesResponse`, `ScheduledMessage`, `ScheduledMessageSent`, `ScheduledMessageFailed` to `channel.proto`.
    - Add `scheduled_at` (optional uint64) to the existing `ChannelMessage` proto.
    - Register all new request messages in the `messages!` macro (background-priority) and push messages in `entity_messages!` (`ScheduledMessageSent`, `ScheduledMessageFailed`).
    - _Requirements: 11.1, 11.2, 11.3_
    - _writes: proto/src/channel.proto, proto/src/proto.rs_

- [ ] 2. Add database migration for the `scheduled_messages` table
    - Create a new migration file with the `CREATE TABLE scheduled_messages (...)` DDL including all columns: `id`, `channel_id` (FK → `channels(id)` ON DELETE CASCADE), `sender_id`, `body`, `scheduled_at`, `created_at`, `state` (SMALLINT), `nonce`, `mentions` (JSONB), `delivered_message_id`, `failure_reason`, `updated_at`.
    - Add partial indexes: `(state, scheduled_at) WHERE state = 0` and `(sender_id, channel_id) WHERE state = 0`.
    - Add a unique index on `(channel_id, sender_id, nonce)` for idempotent scheduling.
    - _Requirements: 11.1, 11.2_
    - _writes: db/migrations/*_scheduled_messages.sql_

- [ ] 3. Implement `ScheduledMessageStore` (server-side data access layer)
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

- [ ] 4. Implement server RPC handlers
    - Register handlers in `Server::new()`: `schedule_channel_message`, `cancel_scheduled_message`, `update_scheduled_message`, `get_scheduled_messages`.
    - `schedule_channel_message`: extract params, validate time bounds (≥1 min, ≤30 days), check channel membership/permissions, delegate to `ScheduledMessageStore::create`, return the new ID.
    - `cancel_scheduled_message`: verify sender owns the message via `store.cancel`, return ack.
    - `update_scheduled_message`: delegate to `store.update`; re-validate time bounds if `scheduled_at` is being changed.
    - `get_scheduled_messages`: delegate to `store.list_for_user`.
    - _Requirements: 11.1, 11.2, 11.4_
    - _writes: collab/src/rpc/scheduled_messages.rs_ (new file or inline in existing handler module)

- [ ] 5. Implement the `SchedulerLoop` background task
    - In `Server::start()`, spawn a detached task that runs every 10 seconds.
    - On tick: call `store.pop_due()`, then for each due message execute `deliver_scheduled_message`.
    - Delivery function:
        - [ ] 5.1 Re-validate sender's channel membership and `can_send_message` permission. If either fails, mark as failed and push `ScheduledMessageFailed` to the sender.
        - [ ] 5.2 Insert the message as a regular `channel_messages` row via the existing `insert_channel_message` DB function.
        - [ ] 5.3 Build a `ChannelMessage` proto with the `scheduled_at` field populated.
        - [ ] 5.4 Broadcast `ChannelMessageSent` to all connected channel members.
        - [ ] 5.5 Send `ScheduledMessageSent` push specifically to the sender's connection(s).
        - [ ] 5.6 Call `store.delete_delivered()` on success.
    - On start-up, call `store.reset_stale_processing()` to recover from any crash mid-delivery.
    - _Requirements: 11.1.4, 11.2.4, 11.3.2_
    - _writes: collab/src/scheduler_loop.rs_ (new file)

- [ ] 6. Add client-side Rust data models
    - Define `ScheduledMessageId(u64)` newtype with proto conversion helpers.
    - Define `ScheduledMessage` struct: `id`, `channel_id`, `sender_id`, `body`, `scheduled_at` (UTC), `created_at`, `mentions`, `display_time` (computed local time).
    - Implement `TryFrom<proto::ScheduledMessage>` and `Into<proto::ScheduledMessage>` conversions.
    - Add a `scheduled_at: Option<DateTime<Utc>>` field to the client's `ChannelMessage` model.
    - _Requirements: 11.1, 11.2, 11.4_
    - _writes: client/src/scheduled_message.rs_

- [ ] 7. Build `SchedulePicker` compose-area widget
    - Implement `SchedulePicker` struct with `selected_date`, `selected_time`, `scheduled_at` (local time), `timezone`, `popover_visible`.
    - Implement `scheduled_at_utc()` — converts local selection to UTC.
    - Implement `render()` — small button next to the send button that toggles a popover.
    - Implement `render_calendar()` — a month-grid calendar for date selection.
    - Implement `render_time_picker()` — hour:minute selection (dropdowns or sliders).
    - Implement `validate()` — ensures selected time is ≥1 minute from now.
    - Wire the picker into the existing compose area: when a time is selected, the send button label changes to "Schedule (time)" with a clock icon; clicking sends a `ScheduleChannelMessage` RPC instead of an immediate message.
    - _Requirements: 11.1.1, 11.1.2, 11.3.3, 11.4.1_
    - _writes: collab_ui/src/schedule_picker.rs_

- [ ] 8. Build `ScheduledMessagesPanel` management view
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
    - _writes: collab_ui/src/scheduled_messages_panel.rs_

- [ ] 9. Implement sidebar badge for pending scheduled messages
    - On app startup and after relevant RPCs, fetch pending count via `GetScheduledMessages` (or a lightweight count endpoint).
    - Store a `pending_scheduled_count: usize` in the relevant app state.
    - Show a badge (clock icon + count) next to the "Scheduled" entry in the channel sidebar.
    - Update the badge when messages are scheduled, delivered, or cancelled.
    - _Requirements: 11.3.1_
    - _writes: collab_ui/src/sidebar.rs, collab_ui/src/app_state.rs_

- [ ] 10. Wire client-side notifications for `ScheduledMessageSent` and `ScheduledMessageFailed`
    - Register handlers for the two push messages in the WebSocket message dispatcher.
    - On `ScheduledMessageSent`: show a toast notification ("Your scheduled message was sent to #channel"), remove the message from the `ScheduledMessagesPanel` list if open, update sidebar badge count.
    - On `ScheduledMessageFailed`: show a toast notification with the failure reason and an action to review/edit the message if still pending.
    - _Requirements: 11.3.2_
    - _writes: collab_ui/src/notifications.rs_ (or integrate into existing notification handler)

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

- [ ] 13. Implement error handling edge cases (server-side)
    - Validation in `schedule_channel_message`: return descriptive errors for out-of-bounds `scheduled_at`, missing channel membership, insufficient permissions.
    - Server restart recovery: in scheduler startup, reset `processing` → `pending` for rows with `updated_at` older than a configurable grace period (e.g., 60 seconds).
    - Concurrent cancel + delivery: the atomic `UPDATE ... WHERE state = pending` ensures only one wins; handle the case where cancel finds no rows (idempotent).
    - Edit-after-delivery: `store.update` checks `state != pending` and rejects with "message already delivered".
    - Nonce fallback: if `nonce` is absent in the request, generate one server-side with a warning log.
    - Register all new error variants for proper client-side display.
    - _Requirements: 11.1, 11.2, 11.3_
    - _writes: collab/src/rpc/scheduled_messages.rs_ (amendments)

- [ ] 14. Update `ChannelMessage` model and rendering for the `scheduled_at` label
    - Parse the `scheduled_at` field from `ChannelMessage` proto on the client.
    - When rendering a channel message, if `scheduled_at` is present, show a small "Scheduled" label/tooltip next to the timestamp.
    - _Requirements: 11.1.4_
    - _writes: client/src/channel_message.rs, collab_ui/src/message_element.rs_
