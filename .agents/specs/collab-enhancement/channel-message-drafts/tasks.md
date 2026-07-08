# Implementation Plan: Channel Message Drafts

## Overview

Add client-side draft persistence for channel message composition. When a user types in the compose area, content is auto-saved to the local `KeyValueStore` with a 500ms debounce. Drafts survive navigation and app restarts. Channels with drafts show a pencil icon indicator in the sidebar. Users can discard drafts with a confirmation dialog.

**Key scope:**
- Client-side only (no server changes) — uses existing `KeyValueStore` (SQLite KVP)
- One draft per channel (channel-scoped)
- New `DraftStore` entity in `collab_ui` crate
- Integration points: `ChannelView` (compose area), `CollabPanel` (channel list indicators)

## Tasks

### Phase 1: DraftStore core

- [x] 1. Create `DraftStore` data structures and in-memory API
  - Define `Draft` struct with `body: String` and `updated_at: DateTime<Utc>`, with `Serialize`/`Deserialize` derives.
  - Define `DraftStore` struct with `kvp: Entity<KeyValueStore>`, `drafts: HashMap<ChannelId, Draft>` in-memory cache, and `active_draft_channel: Option<ChannelId>`.
  - Implement `save_draft()`, `load_draft()`, `clear_draft()`, `has_draft()`, `channels_with_drafts()` — all operating on the in-memory cache initially.
  - _Requirements: 7.1_
  - _writes: `crates/collab_ui/src/draft_store.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab_ui draft_store --features test-support`_

- [x] 2. Add KVP persistence layer to `DraftStore`
  - Implement `persist_key(channel_id) -> String` — key format `channel_draft.{channel_id}`.
  - Implement private async `write_to_kvp()` and `read_from_kvp()` methods.
  - Make `save_draft()` persist synchronously to KVP after updating cache.
  - Make `load_draft()` fall back to KVP read on cache miss.
  - Make `clear_draft()` remove from both cache and KVP.
  - _Requirements: 7.1, 7.4_
  - _writes: `crates/collab_ui/src/draft_store.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab_ui draft_store --features test-support`_

- [x] 3. Register `DraftStore` as a global singleton
  - Add `DraftStore::global(cx: &mut App) -> Entity<Self>` using `Global` trait.
  - Add `DraftStore::init(cx)` that creates the entity, primes cache by reading all draft keys from KVP, and registers the global.
  - Wire `draft_store::init(cx)` into `collab_ui::init()`.
  - _Requirements: 7.1, 7.4_
  - _writes: `crates/collab_ui/src/draft_store.rs`, `crates/collab_ui/src/collab_ui.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p db test_scoped_kvp_read_all --features test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab_ui draft_store --features test-support`_

### Phase 2: Compose area integration (ChannelView)

- [ ] 4. Auto-save draft on content change with 500ms debounce
  - Add a `pending_draft_save: Option<Task<()>>` field to `ChannelView`.
  - Subscribe to `EditorEvent::BufferEdited` on the editor.
  - On edit, cancel any pending save task and spawn a new one that waits 500ms via `cx.background_executor().timer(Duration::from_millis(500)).await`, then calls `DraftStore::save_draft(channel_id, body)`.
  - Only save when content is non-empty.
  - _Requirements: 7.1_
  - _writes: `crates/collab_ui/src/channel_view.rs`_

- [ ] 5. Restore draft on channel navigation
  - In `ChannelView::new()` (or when the channel buffer loads), call `DraftStore::load_draft(channel_id)`.
  - If a draft is returned and its body is non-empty, set the editor's text to the draft content.
  - Do not overwrite a draft when the buffer already has content (e.g., a shared channel buffer with existing text). Only pre-fill when the editor is empty.
  - Track whether a draft was restored to avoid clobbering user input.
  - _Requirements: 7.1_
  - _writes: `crates/collab_ui/src/channel_view.rs`_

- [ ] 6. Clear draft on send
  - In the send-message path of `ChannelView`, after successful message submission, call `DraftStore::clear_draft(channel_id)`.
  - Clear the pending auto-save task if one exists.
  - _Requirements: 7.1_
  - _writes: `crates/collab_ui/src/channel_view.rs`_

### Phase 3: Channel sidebar indicators (CollabPanel)

- [ ] 7. Add draft indicator to channel entries
  - In `CollabPanel`, add a `draft_store: Entity<DraftStore>` field, initialized from `DraftStore::global()`.
  - Subscribe to `DraftStore` events (or observe it) to re-render when drafts change.
  - In `render_channel()`, read `draft_store.has_draft(channel_id)`.
  - When a draft exists: render a pencil icon (`IconName::FileEdit`) after the channel name, and/or italicize the channel label.
  - When no draft exists: render normally.
  - Update `render_channel_notes()` similarly if it should also show the indicator.
  - _Requirements: 7.2_
  - _writes: `crates/collab_ui/src/collab_panel.rs`_

### Phase 4: Discard draft functionality

- [ ] 8. Implement discard draft with confirmation dialog
  - Add a `DiscardDraft` action to `ChannelView`.
  - Bind the action to the `Escape` key (or add a "Discard" button in the compose area).
  - On trigger, show a confirmation dialog using an existing Sim pattern (e.g., a confirmation toast or modal).
  - On confirm: call `DraftStore::clear_draft(channel_id)` and clear the editor content.
  - On cancel: do nothing; keep draft intact.
  - _Requirements: 7.3_
  - _writes: `crates/collab_ui/src/channel_view.rs`_

### Phase 5: Limits, error handling, and polish

- [ ] 9. Enforce storage limit with eviction
  - Define `const MAX_DRAFTS: usize = 50`.
  - In `save_draft()`, after writing, if total cached drafts exceeds `MAX_DRAFTS`, evict the oldest draft(s) by `updated_at`.
  - Evict from both in-memory cache and KVP.
  - _Requirements: 7.4_
  - _writes: `crates/collab_ui/src/draft_store.rs`_

- [ ] 10. Add error handling and logging
  - Wrap KVP read/write calls and log warnings on failure (draft stays in memory only on write failure; compose area starts empty on read failure).
  - Handle corrupt draft JSON: discard the corrupt entry, log error, return `None`.
  - Handle concurrent channel switching: ensure per-channel serialization — if a save is in-flight for a channel when a new save is requested, cancel the prior task and start fresh.
  - _Requirements: 7.1, 7.4_
  - _writes: `crates/collab_ui/src/draft_store.rs`, `crates/collab_ui/src/channel_view.rs`_

### Phase 6: Tests

- [ ] 11. Write unit tests for `DraftStore`
  - Test `save_draft()` / `load_draft()` / `clear_draft()` / `has_draft()` round-trip with in-memory cache.
  - Test persistence round-trip through KVP (write draft, read back, verify body and updated_at).
  - Test `clear_draft()` removes from both cache and KVP.
  - Test `channels_with_drafts()` returns correct channel IDs.
  - _Requirements: 7.1, 7.2, 7.4_
  - _writes: `crates/collab_ui/src/draft_store.rs`_

- [ ] 12. Write integration tests for draft lifecycle
  - Test: type in channel A → switch to channel B → switch back to A → verify draft restored.
  - Test: type draft → send message → verify draft cleared (no indicator, empty compose on reopen).
  - Test: draft indicator appears when draft exists and disappears when cleared.
  - Use GPUI test framework with `TestAppContext` and `VisualTestContext`.
  - _Requirements: 7.1, 7.2_
  - _writes: `crates/collab_ui/src/channel_view.rs` (tests module)_

- [ ] 13. Write persistence and concurrency tests
  - Test: write draft to KVP → simulate app restart (create fresh `DraftStore` from same KVP) → verify draft restored.
  - Test: rapid channel switching while typing — verify no data loss and each channel's draft is independently preserved.
  - Test: exceed `MAX_DRAFTS` limit → verify oldest draft is evicted.
  - _Requirements: 7.4_
  - _writes: `crates/collab_ui/src/draft_store.rs`_
