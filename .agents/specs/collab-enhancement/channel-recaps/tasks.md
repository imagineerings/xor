# Implementation Plan: Channel Recaps / Digests

## Overview

Channel Recaps are automated daily summaries of channel activity over 24-hour periods. Unlike chat-centric recaps, Baymax's recaps are document-first: they summarize CRDT buffer contributions (edits, active editors, sections with most change, line diffs). The implementation spans protobuf definitions, database schema, server-side generation (scheduler + generator), client-side UI widgets (RecapEntry, RecapPanel), a client-side recap store, notification integration, and preference management.

The feature is built in five phases, each producing a working increment:

| Phase | Focus | Delivers |
|-------|-------|----------|
| 1 | Proto + DB + Data models | Definitions, schema, SeaORM models, Rust data types |
| 2 | Server generation | RecapStore queries, RecapScheduler loop, RecapGenerator logic |
| 3 | Client store + RPC | ClientRecapStore, RPC handlers, serialization |
| 4 | Client UI | RecapEntry, RecapPanel, ChannelView integration |
| 5 | Notifications + Preferences | Notification variant, RecapSettings UI, opt-out |

## Task Conventions

- **Requirement references** use `R.X.Y` format (Requirement 13.X.Y from requirements.md)
- **Property references** use `P.X.Y` format (Property X.Y from design.md)
- **Files written** are listed under each task as `_writes: path/to/file.rs_`

## Tasks

### Phase 1: Protobuf, Database Schema, and Data Models

- [ ] 1. Define protobuf messages and RPCs
  - Add `GetRecap`, `GetRecapResponse`, `GetRecaps`, `GetRecapsResponse`, `UpdateRecapPreferences`, `UpdateRecapPreferencesResponse` RPC messages to the proto definitions.
  - Add `Recap`, `EditorSummary`, `SectionSummary`, `DocumentSnapshotSummary` data messages.
  - Register `GetRecap`, `GetRecaps`, `UpdateRecapPreferences` as new RPC methods in the service definition.
  - _Requirements: R.13.1, R.13.2, R.13.3_
  - _writes: `crates/proto/proto/recaps.proto`_ (new) _(or extend existing `crates/proto/proto/channels.proto`)_

- [ ] 2. Create database migration for recap tables
  - Author SQL migration file creating four tables: `channel_recaps`, `channel_recap_reads`, `channel_recap_preferences`, `channel_recap_opt_outs`.
  - Add indexes on `channel_recaps(channel_id, recap_date DESC)` and `channel_recaps(period_end)`.
  - Register migration in the migrations list.
  - _Requirements: R.13.1, R.13.3, R.13.4_
  - _writes: `crates/migrations/20260625000000_create_channel_recaps.sql`_ (new)
  - _writes: edit `crates/migrations/src/lib.rs`_ (register migration)

- [ ] 3. Create SeaORM entity models for recap tables
  - Define `Model` structs in `crates/collab/src/db/tables/` for `channel_recaps`, `channel_recap_reads`, `channel_recap_preferences`, `channel_recap_opt_outs`.
  - Define `RecapId` newtype (aliased to i64) and use in model primary keys.
  - Derive `DeriveEntityModel`, define `Column`, `PrimaryKey` enums, and `Relation` traits for each.
  - _writes: `crates/collab/src/db/tables/recap.rs`_ (new)
  - _writes: edit `crates/collab/src/db/tables/mod.rs`_ (register module)

- [ ] 4. Create client-side Rust types for recap data
  - Define `Recap`, `RecapData`, `EditorSummary`, `SectionSummary`, `DocumentSnapshotSummary` structs with `Serialize`/`Deserialize`.
  - These mirror the proto message fields but are native Rust types used by the GPUI client.
  - _writes: `crates/channel/src/recap_types.rs`_ (new)

### Phase 2: Server-side Recap Generation

- [ ] 5. Implement RecapStore database queries
  - Add methods on `impl Database` in `crates/collab/src/db/queries/recaps.rs`:
    - `create_recap` — JSON-serialize recap data and insert into `channel_recaps` with idempotency check (`ON CONFLICT DO NOTHING` on period uniqueness)
    - `get_latest_recap` — fetch most recent recap for a channel by `recap_date DESC`
    - `get_recap_by_id` — single recap lookup
    - `get_recaps` — batch fetch for multiple channel IDs with per-channel limit
    - `channels_due_for_recap` — query channels whose `period_end` has elapsed AND where at least one user has their delivery time matching current wall-clock time (accounting for timezone)
    - `mark_recap_read` — upsert into `channel_recap_reads`
    - `get_recap_preferences` / `update_recap_preferences` — CRUD on `channel_recap_preferences`
    - `is_recap_opted_out` — check `channel_recap_opt_outs` for user+channel pair
  - Add `count_buffer_operations(channel_id, period_start, period_end)` to count edits in a window (needed for activity threshold check).
  - _Requirements: R.13.1, R.13.3, R.13.4_
  - _Properties: P.5.1, P.5.2, P.5.3, P.5.5_
  - _writes: `crates/collab/src/db/queries/recaps.rs`_ (new)
  - _writes: edit `crates/collab/src/db/queries/mod.rs`_ (register module)

- [ ] 6. Implement RecapGenerator core logic
  - Add to `crates/collab/src/db/queries/recaps.rs` (same module as step 5): internal helper functions for computing recap statistics from buffer operations:
    - `aggregate_editors` — group operations by `replica_id`, map to `user_id`, compute edit count + lines added/deleted per editor
    - `compute_active_sections` — map operation byte ranges to markdown heading sections, identify top sections by edit count
    - `diff_snapshots` — compare buffer snapshots before/after period to compute total lines added/deleted
    - `compute_document_summary` — extract `total_bytes`, `section_count`, `changed_headings`, `most_edited_excerpt` from period-end snapshot
  - Wire these into `Database::create_recap` which orchestrates: fetch buffer → query ops → compute stats → serialize → insert.
  - _Requirements: R.13.1, R.13.4_
  - _Properties: P.5.8, P.5.9_
  - _writes: edit `crates/collab/src/db/queries/recaps.rs`_

- [ ] 7. Implement RecapScheduler polling loop
  - Create `crates/collab/src/recaps.rs` with:
    - `RecapPreferences` struct (re-export or re-define cleanly from the DB model)
    - `start_recap_scheduler(app_state: Arc<AppState>)` — spawns a detached background task that loops every 60 seconds
    - `generate_due_recaps(app_state)` — calls `channels_due_for_recap`, applies activity threshold filter (effective minimum across users), calls `create_recap`, then creates notifications for opted-in users
    - `compute_recap_period` — given a channel and current time, compute the aligned `(period_start, period_end)` 24-hour window based on user delivery time preferences
    - `get_effective_threshold` — returns the minimum `min_activity_threshold` across all opted-in users for a channel
  - Call `start_recap_scheduler` during server startup (in `main.rs` or the server initialization sequence).
  - Add error handling: skip on DB failure (log + retry next tick), 30s timeout on generation per channel.
  - _Requirements: R.13.1, R.13.3, R.13.4_
  - _Properties: P.5.1, P.5.2, P.5.3, P.5.5, P.5.7, P.5.8_
  - _writes: `crates/collab/src/recaps.rs`_ (new)
  - _writes: edit `crates/collab/src/lib.rs`_ (register module)
  - _writes: edit `crates/collab/src/main.rs`_ (invoke `start_recap_scheduler` on startup)

### Phase 3: Client-Side Store and RPC

- [ ] 8. Implement server RPC handlers for recap endpoints
  - Register handlers for `GetRecap`, `GetRecaps`, `UpdateRecapPreferences` in the RPC dispatch.
  - `GetRecap`: validate user is channel member, call `get_latest_recap` or `get_recap_by_id`, serialize to proto response, mark as read via `mark_recap_read`.
  - `GetRecaps`: batch fetch, filter to channels user can access, return recaps.
  - `UpdateRecapPreferences`: validate input (delivery_time_minutes ≤ 1440, valid timezone string), persist via DB.
  - _writes: edit `crates/collab/src/rpc.rs`_ (add handlers)
  - _Requirements: R.13.1, R.13.2, R.13.3_

- [ ] 9. Implement ClientRecapStore
  - Create `ClientRecapStore` entity (GPUI Entity) that holds a `HashMap<ChannelId, Vec<Recap>>` cache and an `Arc<Client>`.
  - Implement methods:
    - `get_recap(channel_id, date)` — check cache first, fall back to `GetRecap` RPC, emit event on result
    - `get_recent_recaps(channel_ids)` — batch RPC call for digest view
    - `mark_read(recap_id)` — fire-and-forget RPC to server
  - Register as a global model (attach to `App`).
  - _writes: `crates/channel/src/recap_store.rs`_ (new)
  - _Requirements: R.13.1, R.13.2_

- [ ] 10. Wire RPC client-request methods for recap endpoints
  - Add `request_recap`, `request_recaps`, `update_recap_preferences` methods on `Client` (in `crates/client`) that send the proto requests and deserialize responses.
  - _writes: edit `crates/client/src/client.rs`_ (or relevant RPC module)

### Phase 4: Client UI

- [ ] 11. Implement RecapEntry inline widget
  - Create `RecapEntry` struct with fields: `channel_id`, `recap_summary: Option<RecapSummary>`, `expanded`, `loading_task`, `recap_store: Entity<ClientRecapStore>`.
  - Implement rendering: a visually distinct card (subtle blue/tinted background, rounded, calendar icon emoji) showing summary stats (editor count, edit count, lines added/deleted, recap date).
  - Collapsed state: show only the summary line.
  - Expanded state: reveal full RecapPanel below the summary.
  - Implement click-to-toggle expansion via `on_click` + `cx.listener`.
  - _writes: `crates/channel/src/recap_entry.rs`_ (new)
  - _Requirements: R.13.2_
  - _Properties: P.5.6_

- [ ] 12. Implement RecapPanel expanded view
  - Create `RecapPanel` struct with fields: `channel_id`, `recap: Option<Recap>`, `loading`, `error`.
  - Implement `open(channel_id, recap_date)` — fetches full recap via `ClientRecapStore::get_recap`.
  - Render full layout sections:
    - Summary bar: edit operations, editor count, +lines/-lines
    - Active editors list with avatar, github login, edit count, line delta (use existing `Avatar` component)
    - Top sections list with section preview, edit count, activity level badge. Click navigates to buffer position via `window.dispatch_action(NavigateToOffset { offset })`.
    - Document snapshot summary with total bytes, section count, changed headings list, most-edited excerpt.
  - Render a close/dismiss button.
  - _writes: `crates/channel/src/recap_panel.rs`_ (new)
  - _Requirements: R.13.2_
  - _Properties: P.5.6_

- [ ] 13. Integrate recap entry into Channel View
  - Add `recap_entry: Option<RecapEntry>` field to `ChannelView`.
  - On channel load / buffer change, query `ClientRecapStore` for the latest recap. If exists, create a `RecapEntry` and render it above the buffer (in the channel chrome, not part of the document text).
  - Add "View Recap" button to channel header that opens the recap panel.
  - Add recap entry to the jump-to menu for quick navigation.
  - _writes: edit `crates/channel/src/channel.rs`_ (ChannelView modifications)
  - _Requirements: R.13.2_

- [ ] 14. Implement unread indicator for recaps
  - In `RecapEntry`, check `is_read` on the recap. If false, show a blue dot / bold styling.
  - When the user expands the recap panel, call `ClientRecapStore::mark_read` (debounced).
  - When `mark_read` completes, update the `is_read` field and call `cx.notify()` to re-render.
  - _Requirements: R.13.2_
  - _Properties: P.5.6_

### Phase 5: Notifications and Preferences

- [ ] 15. Add ChannelRecap notification variant
  - Add `ChannelRecap { channel_id: u64, recap_date: String, edit_count: u32 }` to the `Notification` enum in `crates/rpc/src/notification.rs`.
  - Implement notification display in the in-app notification panel: show channel name, edit count, "View recap" action button. Clicking navigates to channel and opens recap.
  - _Requirements: R.13.3_
  - _writes: edit `crates/rpc/src/notification.rs`_

- [ ] 16. Implement server-side notification delivery for recaps
  - In `generate_due_recaps` (step 7), after storing a recap, create `Notification::ChannelRecap` entries for each channel member who is not opted out.
  - Push `AddNotification` via WebSocket to currently connected clients (reuse existing notification broadcast infrastructure).
  - Respect per-channel and global opt-out preferences.
  - _Requirements: R.13.3_
  - _Properties: P.5.5, P.5.7_

- [ ] 17. Implement RecapSettings UI (preference management)
  - Create a settings panel or modal with:
    - Toggle for global opt-out (`opt_out_all`)
    - Per-channel opt-out list (checkboxes for visible channels)
    - Delivery time picker (hour:minute selector, default 8:00 AM)
    - Timezone selector (pre-populated IANA timezone list, default to user's detected timezone)
    - Minimum activity threshold slider/input (default 5 edits)
  - On save, call `UpdateRecapPreferences` RPC and update local state.
  - _writes: `crates/channel/src/recap_settings.rs`_ (new)
  - _Requirements: R.13.3, R.13.4_
  - _Properties: P.5.5_

### Testing

- [ ] 18. Write unit tests for RecapStore queries
  - Test `create_recap` with mocked DB: verify correct SQL, verify idempotency (second call is no-op).
  - Test `get_latest_recap`: return the correct recap when multiple exist, return `None` for empty channel.
  - Test `channels_due_for_recap`: verify timezone-aware period alignment, correct filtering of opted-out users.
  - Test `count_buffer_operations`: verify counts correct for non-overlapping time windows.
  - Test preference CRUD: verify insert, update, read-back consistency.
  - _Requirements: R.13.1, R.13.4_
  - _Properties: P.5.1, P.5.2, P.5.3_

- [ ] 19. Write unit tests for RecapGenerator computation
  - Test `aggregate_editors` with known operation sets: verify correct edit counts, lines added/deleted per user, correct ordering by edit count descending.
  - Test `compute_active_sections` with known byte-offset edits: verify section boundaries, preview truncation (≤80 chars), activity level classification (`light`/`moderate`/`heavy`).
  - Test `diff_snapshots` line counting with before/after text.
  - Test `compute_document_summary`: verify heading extraction, excerpt generation.
  - _Requirements: R.13.1, R.13.4_

- [ ] 20. Write integration tests for full generation flow
  - Set up a test channel with buffer + known edits. Trigger `generate_due_recaps`. Assert: recap row created, correct stats match known edits, notifications created for members.
  - Test activity threshold boundary: edits below threshold → no recap; edits at threshold → recap generated.
  - Test opt-out flow: user with per-channel opt-out → no notification for that user but recap exists.
  - Test timezone alignment: users in `America/New_York` vs `Europe/London` with same delivery time → different UTC trigger times.
  - _Requirements: R.13.1, R.13.3, R.13.4_
  - _Properties: P.5.2, P.5.3, P.5.5_

- [ ] 21. Write RPC / API tests
  - `GetRecap`: test existing recap, non-existent recap, unauthorized channel access → correct responses.
  - `UpdateRecapPreferences`: update each field → verified persistence. Invalid `delivery_time_minutes > 1440` → `INVALID_ARGUMENT` error.
  - Recap notification delivery: generate recap while user connected → verify `AddNotification` received with correct fields.
  - _Requirements: R.13.1, R.13.2, R.13.3_

- [ ] 22. Write GPUI UI tests
  - Render `RecapEntry` with mock data: verify summary stats display, date, editor count. Verify collapsed vs expanded state toggling.
  - Render `RecapPanel` with full editors and sections: verify all sections render, empty state renders correctly (no editors, no sections → informative message).
  - Click a section link in recap panel: verify buffer navigates to correct offset via dispatched action.
  - Unread indicator: render with `is_read = false` → blue dot visible. Call `mark_read` → dot disappears after re-render.
  - _Requirements: R.13.2_
  - _Properties: P.5.6_

- [ ] 23. Write concurrency tests
  - Simultaneous generation: Two scheduler ticks fire concurrently for same channel-period → only one recap created (verify idempotency).
  - Read during generation: Start async generation, fire RPC during generation → either previous recap returned or `None` — never partial/inconsistent data.
  - Race on preference update: Two concurrent updates → last-write-wins, no crash.
  - _Requirements: R.13.4_
  - _Properties: P.5.1, P.5.9_
