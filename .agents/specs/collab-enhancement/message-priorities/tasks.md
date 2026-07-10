# Implementation Plan: Message Priorities

## Overview

Add message priority levels (Normal, Important, Urgent) to channel messages in Sim. This spans protobuf definitions, a database migration, server-side priority handling with DND-aware urgent notification dispatch, client-side priority UI (selector, badge, toast), and settings integration. The implementation follows an incremental bottom-up order: proto → DB → server → client types → client UI → settings → integration wiring → tests.

## Tasks

### Phase 1: Proto & Data Layer

- [x] 1. Define `ChannelMessagePriority` protobuf enum
  - Add the `ChannelMessagePriority` enum (`ChannelMessagePriorityNormal = 0`, `ChannelMessagePriorityImportant = 1`, `ChannelMessagePriorityUrgent = 2`) to the proto definitions. The prefixes avoid the package-scoped enum-value collision with existing debugger protocol values.
  - _Requirements: 12.1_
  - _writes: crates/proto/proto/channel.proto_

- [x] 2. Extend `SendChannelMessage` proto with priority field
  - Add optional `ChannelMessagePriority priority = 7` to the `SendChannelMessage` message (`file_ids` already uses field 6).
  - _Requirements: 12.1_
  - _writes: crates/proto/proto/definitions.proto_

- [x] 3. Extend `ChannelMessage` proto with priority field
  - Add `ChannelMessagePriority priority = 12` to the `ChannelMessage` message (defaults to Normal; fields 9–11 are already assigned).
  - _Requirements: 12.1, 12.3_
  - _writes: crates/proto/proto/definitions.proto_

- [x] 4. Define `UrgentMessageNotification` proto message
  - New push message with `channel_id`, `message_id`, `sender_id`, `message_preview` fields.
  - _Requirements: 12.2_
  - _writes: crates/proto/proto/definitions.proto_

- [x] 5. Regenerate Rust proto bindings
  - Run the proto code generator to produce updated Rust types for all changed/added messages and the new enum.
  - _writes: crates/proto/src/** (auto-generated)_
  - _Completed: Added priority protocol fields at unused wire IDs, an urgent-message push, and the associated envelope and entity-message registrations._
  - _Validation: `cargo check -p proto`._

- [ ] 6. Add database migration for priority column
  - `ALTER TABLE channel_messages ADD COLUMN priority SMALLINT NOT NULL DEFAULT 0;` plus index `idx_channel_messages_priority`.
  - _Requirements: 12.1_
  - _writes: crates/db/migrations/XXXXXXXX_add_message_priority.sql_

- [ ] 7. Define `MessagePriority` Rust enum in the client crate
  - Mirror the proto enum: `Normal`, `Important`, `Urgent` with `color()`, `icon()`, `label()` methods. Implement `From<proto::ChannelMessagePriority>`.
  - _Requirements: 12.3_
  - _writes: crates/client/src/message_priority.rs_

- [ ] 8. Add `priority` field to client-side `ChannelMessage` struct
  - Extend the model with `priority: MessagePriority`, deserialized from proto `ChannelMessage`.
  - _Requirements: 12.1, 12.3_
  - _writes: crates/client/src/channel_message.rs_

### Phase 2: Server-Side Logic

- [ ] 9. Add `priority` column read/write to `ChannelMessagesStore`
  - Update `InsertMessage` to persist the priority field. Ensure the column defaults to `0` (Normal) for existing rows.
  - _Requirements: 12.1_
  - _writes: crates/collab/src/db/channel_messages.rs_

- [ ] 10. Validate priority on incoming `SendChannelMessage`
  - Reject invalid priority values with `INVALID_ARGUMENT`. Treat missing/unset priority as Normal (backward compat).
  - _Requirements: 12.1_
  - _writes: crates/collab/src/rpc/channel_handler.rs_

- [ ] 11. Enforce priority immutability on `UpdateChannelMessage`
  - Ensure the `priority` field is never changed on edit — always preserve the originally stored value.
  - _Requirements: 12.1_
  - _writes: crates/collab/src/rpc/channel_handler.rs_

- [ ] 12. Add notification preferences store (DND bypass for urgent)
  - Add `notification_preferences` storage (extend `user_settings` or new store method) for `bypass_dnd_for_urgent` boolean. Default to `false` (respect DND).
  - _Requirements: 12.2_
  - _writes: crates/collab/src/db/user_settings.rs_

- [ ] 13. Implement `dispatchUrgentNotifications` server function
  - Iterate channel members (excluding sender), check DND preference, create notification row (`kind = "urgent_message"`), push `AddNotification` + `UrgentMessageNotification` to connected clients.
  - _Requirements: 12.2_
  - _writes: crates/collab/src/rpc/urgent_notifications.rs_

- [ ] 14. Wire urgent dispatch into `SendChannelMessage` handler
  - After successful message insert and broadcast, call `dispatchUrgentNotifications` if priority is Urgent.
  - _Requirements: 12.2_
  - _writes: crates/collab/src/rpc/channel_handler.rs_

### Phase 3: Client — Priority Composing UI

- [ ] 15. Build `PrioritySelector` component
  - Compact button group (Normal / Important / Urgent) in the compose area. Highlights the selected priority with color cues (amber for Important, red for Urgent).
  - _Requirements: 12.1_
  - _writes: crates/collab_ui/src/priority_selector.rs_

- [ ] 16. Integrate `PrioritySelector` into `ComposeArea`
  - Hold `PrioritySelector` state in `ComposeArea`. Pass `selected_priority` into the `SendChannelMessage` RPC call. Render the selector below the message input.
  - _Requirements: 12.1_
  - _writes: crates/collab_ui/src/compose_area.rs_

### Phase 4: Client — Priority Display

- [ ] 17. Build `PriorityBadge` component
  - Render a colored icon + label before the message timestamp. Normal → nothing. Important → amber `AlertTriangle` icon + "Important". Urgent → red `AlertOctagon` icon + "Urgent".
  - _Requirements: 12.3_
  - _writes: crates/collab_ui/src/priority_badge.rs_

- [ ] 18. Render `PriorityBadge` in the main channel message list
  - Insert `PriorityBadge::render` into each channel message element, before the timestamp. Only messages with non-Normal priority produce a visible badge.
  - _Requirements: 12.3_
  - _writes: crates/collab_ui/src/channel_view.rs_

- [ ] 19. Render `PriorityBadge` in thread replies
  - Include the badge in each thread reply message, same position as the main channel.
  - _Requirements: 12.4_
  - _writes: crates/collab_ui/src/thread_view.rs_

- [ ] 20. Show root-message priority in thread summary badge
  - When a thread's root message has Important/Urgent priority, display the priority indicator next to the reply count in the main channel thread summary.
  - _Requirements: 12.4_
  - _writes: crates/collab_ui/src/thread_summary.rs_

- [ ] 21. Render `PriorityBadge` in search results
  - Include the badge in channel message search result items.
  - _Requirements: 12.3_
  - _writes: crates/collab_ui/src/search_results.rs_

### Phase 5: Client — Urgent Notifications

- [ ] 22. Build `UrgentNotificationToast` component
  - Persistent toast with red background, `AlertOctagon` icon, sender name, message preview, and "Dismiss" button. Clicking navigates to the channel/message; "Dismiss" marks notification read and removes the toast.
  - _Requirements: 12.2_
  - _writes: crates/collab_ui/src/urgent_notification_toast.rs_

- [ ] 23. Register `UrgentMessageNotification` handler on the client
  - In the workspace or collab panel, add a `client.add_message_handler` for `UrgentMessageNotification`. On receipt, create and display an `UrgentNotificationToast`.
  - _Requirements: 12.2_
  - _writes: crates/collab_ui/src/workspace.rs_

- [ ] 24. Wire toast dismissal to message-read events
  - When the user navigates to the channel containing the urgent message (via toast click or directly), dismiss the corresponding toast and mark the notification read on the server.
  - _Requirements: 12.2_
  - _writes: crates/collab_ui/src/urgent_notification_toast.rs_

### Phase 6: Client — Notification Settings

- [ ] 25. Define `NotificationSettingsContent` settings structs
  - JSON-deserializable structs for `notifications.urgent_messages.bypass_dnd` setting.
  - _Requirements: 12.2_
  - _writes: crates/settings/src/notification_settings.rs_

- [ ] 26. Build `UrgentNotificationSettings` UI component
  - Toggle under a "Urgent Messages" heading: "Allow urgent notifications during Do Not Disturb". Persists to user settings on toggle.
  - _Requirements: 12.2_
  - _writes: crates/collab_ui/src/notification_settings.rs_

- [ ] 27. Integrate urgent settings into the preferences panel
  - Add the `UrgentNotificationSettings` section into the notification preferences page.
  - _Requirements: 12.2_
  - _writes: crates/collab_ui/src/preferences.rs_

### Phase 7: Tests

- [ ] 28. Server unit tests: priority storage and immutability
  - Test that priority is correctly stored on insert. Test that `UpdateChannelMessage` does not alter the stored priority.
  - _Requirements: 12.1_
  - _writes: crates/collab/src/rpc/channel_handler.rs (tests module)_

- [ ] 29. Server unit tests: urgent notification dispatch logic
  - Test `dispatchUrgentNotifications`: correct recipients, DND suppression, bypass_dnd toggle, sender exclusion, notification row creation.
  - _Requirements: 12.2_
  - _writes: crates/collab/src/rpc/urgent_notifications.rs (tests module)_

- [ ] 30. Client unit tests: `MessagePriority` enum methods
  - Verify `color()`, `icon()`, `label()` return correct values for each variant. Verify `From<proto>` conversion.
  - _Requirements: 12.3_
  - _writes: crates/client/src/message_priority.rs (tests module)_

- [ ] 31. Client unit tests: `PriorityBadge` rendering
  - GPUI unit test: render each priority level and verify icon presence, text content, and color class.
  - _Requirements: 12.3_
  - _writes: crates/collab_ui/src/priority_badge.rs (tests module)_

- [ ] 32. Client unit tests: `PrioritySelector` state transitions
  - GPUI unit test: verify initial state is Normal, clicking buttons changes `selected_priority`, and the active button is visually highlighted.
  - _Requirements: 12.1_
  - _writes: crates/collab_ui/src/priority_selector.rs (tests module)_

- [ ] 33. Integration test: full send flow with each priority level
  - Send messages with Normal, Important, and Urgent priority. Verify `ChannelMessageSent` contains the correct priority. Verify `UrgentMessageNotification` is pushed only for Urgent.
  - _Requirements: 12.1, 12.2_
  - _writes: crates/collab/tests/message_priority_tests.rs_

- [ ] 34. Integration test: backward compatibility with old clients
  - Send a `SendChannelMessage` without the priority field set; verify it is stored and returned as Normal.
  - _Requirements: 12.1_
  - _writes: crates/collab/tests/message_priority_tests.rs_

- [ ] 35. Integration test: DND notification suppression
  - Create a user with DND enabled → send urgent message → verify no notification. Enable `bypass_dnd_for_urgent` → send urgent message → verify notification arrives.
  - _Requirements: 12.2_
  - _writes: crates/collab/tests/message_priority_tests.rs_

- [ ] 36. Concurrency test: simultaneous urgent messages
  - Send two urgent messages concurrently; verify both notifications are delivered independently. Verify no duplicate notifications for the same message.
  - _Requirements: 12.2_
  - _writes: crates/collab/tests/message_priority_tests.rs_

- [ ] 37. UI test: priority badge display in channel, thread, and search
  - GPUI integration test: verify `PriorityBadge` renders correctly in all three surfaces for each priority level.
  - _Requirements: 12.3, 12.4_
  - _writes: crates/collab_ui/src/priority_badge.rs (tests module)_

- [ ] 38. UI test: `UrgentNotificationToast` rendering and dismissal
  - GPUI test: render the toast, verify styling and content, simulate dismiss click, verify toast is removed.
  - _Requirements: 12.2_
  - _writes: crates/collab_ui/src/urgent_notification_toast.rs (tests module)_
