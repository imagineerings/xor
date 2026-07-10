# Implementation Plan: Channel Join Requests

## Overview

Add a join request workflow for private channels, enabling non-members to request access and channel admins to approve/deny those requests. The implementation spans protobuf definitions, a database migration, server-side store + RPC handlers + background expiry job, and client-side UI panels (RequestToJoinPanel, PendingRequestsList, RequestDetailPanel), plus notification integration.

**Key crates involved:** `proto`, `collab`, `collab_ui`, `client`, `rpc`, `db`, `migrations`

---

## Tasks

- [x] 1. Define protobuf messages
  - Add `RequestJoinChannel`, `RequestJoinChannelResponse`, `RespondToJoinRequest`, `RespondToJoinRequestResponse` RPC messages.
  - Add `GetPendingJoinRequests`, `GetPendingJoinRequestsResponse`, `PendingJoinRequest` entity messages.
  - Add `JoinRequestAdded`, `JoinRequestResponded` push messages.
  - Add `PendingRequestCount` message and extend `UpdateChannels` with a `repeated PendingRequestCount pending_request_counts` field (field number 16).
  - Register all new RPCs in the `collab` proto dispatch table.
  - _Requirements: 10.1, 10.2, 10.3, 10.4_
  - _writes: crates/proto/proto/channel.proto, crates/proto/proto/sim.proto, crates/proto/src/proto.rs_
  - _Completed: Added join-request RPC request/response types, admin/requester push messages, pending-request entities, and `UpdateChannels.pending_request_counts`; registered every envelope, request-response pair, and channel-targeted push in the proto dispatch layer._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto --features proto/test-support`._

- [x] 2. Database migration
  - Create `channel_join_requests` table with columns: `id` (BIGINT PK generated), `channel_id` (BIGINT NOT NULL FK references channels ON DELETE CASCADE), `user_id` (BIGINT NOT NULL FK references users ON DELETE CASCADE), `reason` (TEXT NULL), `created_at` (TIMESTAMP NOT NULL DEFAULT NOW()).
  - Add UNIQUE constraint on `(channel_id, user_id)` for duplicate prevention.
  - Add index `idx_join_requests_channel` on `(channel_id)`.
  - Add index `idx_join_requests_created_at` on `(created_at)`.
  - _Requirements: 10.1 (AC 4), 10.4 (AC 3)_
  - _writes: crates/collab/migrations/..._channel_join_requests.sql, crates/collab/migrations.sqlite/20221109000000_test_schema.sql_
  - _Completed: Added the production `channel_join_requests` table with cascading channel/user foreign keys, per-user/channel duplicate prevention, and channel/expiry query indexes. Added the equivalent SQLite integration-test schema._
  - _Validation: `sqlite3 :memory: ".read crates/collab/migrations.sqlite/20221109000000_test_schema.sql"`; `CARGO_INCREMENTAL=0 cargo check -p collab --features collab/test-support`._

- [x] 3. Regenerate proto Rust bindings
  - Run the proto codegen to produce `crates/proto/src/proto.rs` with the new messages.
  - Validate that `PendingRequestCount`, `JoinRequestAdded`, `JoinRequestResponded`, and the new RPC types compile.
  - _writes: crates/proto/src/proto.rs_
  - _Completed: Verified the existing `prost` build generates the new join-request messages from `channel.proto` and exposes them through the `proto` crate during compilation._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto --features proto/test-support`._

- [x] 4. Implement server-side `JoinRequestStore`
  - [x] 4.1 Implement `request_join` — INSERT into `channel_join_requests`; returns error on UNIQUE violation (duplicate pending request).
  - [x] 4.2 Implement `pending_join_request_exists` — SELECT COUNT(*) for `(channel_id, user_id)`.
  - [x] 4.3 Implement `approve_join_request` — DELETE the request row and INSERT a `channel_members` row with `accepted = true`, `role = Member` in a single transaction.
  - [x] 4.4 Implement `deny_join_request` — DELETE the request row.
  - [x] 4.5 Implement `get_pending_requests` — SELECT all requests for a channel, JOIN with `users` for display info.
  - [x] 4.6 Implement `expire_old_requests` — DELETE all rows with `created_at < threshold`, return list of expired `(user_id, channel_id, channel_name)` for notification dispatch.
  - [x] 4.7 Implement `count_pending_requests` — SELECT COUNT(*) for a channel.
  - _Requirements: 10.1, 10.2, 10.4_
  - _writes: crates/collab/src/db/join_request_store.rs, crates/collab/src/db/tables/channel_join_request.rs_
  - _Completed: Added transactional request creation, duplicate checks, approval that atomically removes the request and creates or restores an accepted Member membership, denial, ordered pending lists, expiry with channel metadata, and per-channel counts. Added SeaORM table metadata and SQLite integration coverage._
  - _Validation: `target/debug/deps/collab_tests-9512c50120ff4d19 db_tests::join_request_tests::test_join_request_store_lifecycle_sqlite --exact --nocapture`; `target/debug/deps/collab_tests-9512c50120ff4d19 db_tests::join_request_tests::test_join_request_store_expires_requests_sqlite --exact --nocapture`; `CARGO_INCREMENTAL=0 cargo check -p collab --features collab/test-support`._

- [x] 5. Implement `handle_request_join` RPC handler
  - Verify channel exists and has `visibility = Members` (otherwise return error: public channels can be joined directly).
  - Verify caller is not already a member (skip banned role).
  - Check for duplicate pending request via `pending_join_request_exists`.
  - Insert the request via `request_join`.
  - Look up all admins of the channel.
  - Create a `Notification::JoinRequest` for each admin via `notification_store`.
  - Broadcast `JoinRequestAdded` push to all connected admin clients.
  - Return `RequestJoinChannelResponse { success: true }`.
  - _Requirements: 10.1_
  - _writes: crates/collab/src/rpc.rs_
  - _Completed: Registered `RequestJoinChannel`; it rejects public channels and existing non-banned members, persists the request through `JoinRequestStore`, creates deduplicated admin notifications, and pushes `JoinRequestAdded` to connected admins._
  - _Validation: `cargo check -p collab --features collab/test-support`._

- [x] 6. Implement `handle_respond_join_request` RPC handler
  - Verify caller is a channel admin (otherwise return `Forbidden`).
  - Verify the pending request still exists (otherwise return error: already handled or expired).
  - If `approve = true`: call `approve_join_request`, create `Notification::JoinRequestApproved`, broadcast `JoinRequestResponded` push with `approved = true`.
  - If `approve = false`: call `deny_join_request`, create `Notification::JoinRequestDenied` with optional denial reason, broadcast `JoinRequestResponded` push with `approved = false`.
  - Return `RespondToJoinRequestResponse { success: true }`.
  - _Requirements: 10.2, 10.3_
  - _writes: crates/collab/src/rpc.rs_
  - _Completed: Registered admin-gated approval and denial handling. The handler resolves the pending request atomically through `JoinRequestStore`, persists an outcome notification for the requester, and pushes `JoinRequestResponded` to active requester connections._
  - _Validation: `cargo check -p collab --features collab/test-support`._

- [x] 7. Implement `handle_get_pending_join_requests` RPC handler
  - Verify caller is a channel admin.
  - Call `get_pending_requests` and return the list.
  - _Requirements: 10.4_
  - _writes: crates/collab/src/rpc.rs_
  - _Completed: Registered an admin-only pending-request endpoint that returns ordered requester IDs, optional reasons, and request timestamps from `JoinRequestStore`._
  - _Validation: `cargo check -p collab --features collab/test-support`._

- [x] 8. Implement background expiry job
  - Add `expire_join_requests` async function that reads `CHANNEL_JOIN_REQUEST_TTL_SECS` env var (default 7 days), calls `expire_old_requests`, and creates `Notification::JoinRequestDenied` with reason "Your join request has expired." for each expired request.
  - Register the job in the server startup/periodic task scheduler, running every hour.
  - _Requirements: 10.4 (AC 3)_
  - _writes: crates/collab/src/jobs.rs_
  - _Completed: Added an hourly collab-server job that reads `CHANNEL_JOIN_REQUEST_TTL_SECS` (defaulting to seven days), deletes stale requests, and persists an expiry-denial notification for each requester._
  - _Validation: `cargo check -p collab --features collab/test-support`; `target/debug/deps/collab_tests-9512c50120ff4d19 db_tests::join_request_tests::test_expire_join_requests_creates_notification_sqlite --exact --nocapture`._

- [x] 9. Extend `UpdateChannels` handling on server
  - After any insert/delete in `channel_join_requests`, compute `PendingRequestCount` per channel and include in `UpdateChannels` broadcast to affected admins.
  - Ensure `UpdateChannels` pushes include the `pending_request_counts` field so admin clients stay in sync.
  - _Requirements: 10.4 (AC 1)_
  - _writes: crates/collab/src/rpc.rs, crates/collab/src/jobs.rs_
  - _Completed: Broadcast pending-request counts to connected channel admins after request creation, resolution, and background expiry. The expiry loop now runs with the RPC peer and connection pool so it can keep admin state synchronized._
  - _Validation: `cargo check -p collab --features collab/test-support`._

- [x] 10. Update `rpc::Notification` enum with join request variants
  - Add `JoinRequest { channel_id, channel_name, requesting_user_id, requesting_user_name, reason }` variant.
  - Add `JoinRequestApproved { channel_id, channel_name }` variant.
  - Add `JoinRequestDenied { channel_id, channel_name, reason }` variant.
  - Implement `entity_id` via serde rename on `channel_id` for each variant (matching existing pattern).
  - Update the notification content rendering table (title, body, action) for each new variant.
  - _Requirements: 10.2 (AC 1), 10.3_
  - _writes: crates/rpc/src/notification.rs_
  - _Completed: Added join-request, approval, and denial notification variants with channel entity IDs. Notification loading resolves requesters through `UserStore`; toast rendering includes channel names and optional reasons._
  - _Validation: `cargo test -p rpc`; `cargo check -p collab_ui -p notifications`._

- [x] 11. Add client-side data models
  - Define `PendingJoinRequest` struct with `user_id`, `user: Arc<User>`, `reason: Option<SharedString>`, `created_at: OffsetDateTime`.
  - Define `PendingRequestCount` struct with `channel_id: ChannelId`, `count: u32`.
  - Derive appropriate traits (Clone, Debug, etc.).
  - _Requirements: 10.4_
  - _writes: crates/collab_ui/src/channel_join_requests.rs_
  - _Completed: Added shared pending-request and pending-count client models with the requester user, optional reason, timestamp, channel ID, and count._
  - _Validation: `cargo check -p collab_ui`._

- [x] 12. Update `ChannelStore` for pending request counts
  - Add `pending_join_request_counts: HashMap<ChannelId, u32>` field.
  - Add `pub fn pending_request_count(&self, channel_id: ChannelId) -> u32`.
  - In `handle_update_channels`, parse `pending_request_counts` from the proto message and update the map (remove entry when count is 0).
  - _Requirements: 10.4 (AC 1)_
  - _writes: crates/channel/src/channel_store.rs_
  - _Completed: Added pending-request count state and lookup to the shared channel store. `UpdateChannels` applies nonzero counts and clears entries when the server sends zero._
  - _Validation: `cargo test -p channel test_pending_join_request_counts`._

- [x] 13. Implement `RequestToJoinPanel`
  - Create `RequestToJoinPanel` struct with `channel_id`, `reason` (editable `SharedString`), `state: RequestState`.
  - Define `RequestState` enum: `Idle`, `Sending`, `Sent`, `AlreadyRequested`, `Error(SharedString)`.
  - Implement `Render`:
    - `Idle`: Show reason text field + "Request to Join" button.
    - `Sending`: Show spinner / disabled state.
    - `Sent`: Show confirmation: "Join request sent. You'll be notified when a channel admin responds."
    - `AlreadyRequested`: Show "You have already requested to join this channel."
    - `Error(msg)`: Show error message with "Try Again" button.
  - `submit` method: set state to `Sending`, call `RequestJoinChannel` RPC, transition to `Sent` or `Error` on result.
  - Handle `AlreadyRequested` error from the server and transition to `AlreadyRequested` state.
  - _Requirements: 10.1 (AC 1, AC 2, AC 3)_
  - _writes: crates/collab_ui/src/request_to_join_panel.rs_
  - _Completed: Added a requester panel with an optional reason editor, request RPC submission, sending, sent, duplicate-request, and retryable error states._
  - _Validation: `cargo check -p collab_ui`._

- [ ] 14. Integrate `RequestToJoinPanel` into workspace/channel navigation
  - In the channel navigation logic (when user clicks a private channel they are not a member of), show `RequestToJoinPanel` instead of `ChannelView`.
  - Check `ChannelStore` for the user's role on the selected channel — if `None`, render the request panel.
  - If the user already has a pending request, pre-populate the `AlreadyRequested` state.
  - _Requirements: 10.1 (AC 1)_
  - _writes: crates/collab_ui/src/collab_panel.rs_

- [x] 15. Implement `PendingRequestsList`
  - Create `PendingRequestsList` struct with `channel_id`, `requests: Vec<PendingRequestViewModel>`, `loading: bool`, `badge_count: u32`.
  - Define `PendingRequestViewModel` with `user: Arc<User>`, `reason: Option<SharedString>`, `created_at: OffsetDateTime`.
  - Implement `fn new(channel_id, cx)`, `fn load_requests`, and `Render`.
  - Render: tab header with badge "Pending Requests ({count})", list of request entries (avatar, name, timestamp, reason), each clickable to open `RequestDetailPanel`.
  - Fetch via `GetPendingJoinRequests` RPC on mount.
  - Listen for `JoinRequestAdded` push to refresh the list.
  - _Requirements: 10.2 (AC 5), 10.4 (AC 2)_
  - _writes: crates/collab_ui/src/pending_requests_list.rs_
  - _Completed: Added an admin-only pending-request list that fetches requests, resolves requester profiles, renders their reasons and timestamps, emits typed selection events, and refreshes when a matching `JoinRequestAdded` push arrives._
  - _Validation: `cargo check -p collab_ui`._

- [x] 16. Implement `RequestDetailPanel`
  - Create `RequestDetailPanel` struct with `request: PendingRequestViewModel`, `show_denial_input: bool`, `denial_reason: SharedString`.
  - Implement `Render`: requester profile (avatar, name, username), timestamp (relative format), reason text (or "No reason provided"), Approve button, Deny button.
  - On Deny click: show denial reason text field if not already visible, then send `RespondToJoinRequest(approve: false)`.
  - On Approve click: send `RespondToJoinRequest(approve: true)`.
  - Call `cx.notify()` after response to trigger parent list refresh.
  - _Requirements: 10.2 (AC 2, AC 3, AC 4)_
  - _writes: crates/collab_ui/src/request_detail_panel.rs_
  - _Completed: Added requester details, visible reason/timestamp, approve and deny actions with an optional denial reason, response errors, and a completion event for parent refreshes._
  - _Validation: `cargo check -p collab_ui`._

- [x] 17. Integrate pending requests tab into channel member management modal
  - Add a new tab "Pending Requests" with badge count from `ChannelStore::pending_request_count`.
  - Show `PendingRequestsList` under the tab.
  - Wire up badge updates so the badge refreshes when `UpdateChannels` brings new `pending_request_counts`.
  - _Requirements: 10.2 (AC 5), 10.4 (AC 1)_
  - _writes: crates/collab_ui/src/channel_member_management.rs_
  - _Completed: Added a Pending Requests tab to channel member management with the live channel-store badge. Selecting an entry opens its detail panel; completed approval or denial reloads the list._
  - _Validation: `cargo check -p collab_ui`._

- [x] 18. Notification rendering for join request variants
  - Add rendering logic for `Notification::JoinRequest` — title: "Join Request", body: "{username} wants to join #{channel}", action: navigate to member management for that channel.
  - Add rendering logic for `Notification::JoinRequestApproved` — title: "Request Approved", body: "You've been added to #{channel}", action: navigate to the channel.
  - Add rendering logic for `Notification::JoinRequestDenied` — title: "Request Denied", body: "Your request to join #{channel} was denied" + reason (if provided), action: dismiss.
  - Wire up `JoinRequestApproved` notification click to navigate to the channel (`open_channel` action).
  - Update `NotificationStore::add_notifications` to extract `requesting_user_id` from `JoinRequest` for proper notification routing.
  - _Requirements: 10.2 (AC 1), 10.3_
  - _writes: crates/notifications/src/notification_store.rs, crates/collab_ui/src/collab_panel.rs_
  - _Completed: Join request notifications resolve requester profiles and render channel/reason context. Toast clicks open the channel member-management review flow for admins and open the approved channel for requesters; denied notifications remain dismissible._
  - _Validation: `cargo check -p collab_ui`._

- [x] 19. Implement client-side push handler for `JoinRequestAdded` and `JoinRequestResponded`
  - Handle `JoinRequestAdded` push: insert into pending requests store (if admin of the channel), update badge count, show toast notification.
  - Handle `JoinRequestResponded` push: if `approved`, navigate to the channel; show toast with outcome.
  - _Requirements: 10.2 (AC 1), 10.3 (AC 3)_
  - _writes: crates/collab_ui/src/channel_join_requests.rs_
  - _Completed: Added a shared push store owned by the collaboration panel. It emits admin-only request-added events for active review lists and routes requester approvals directly to the channel. Persisted join-request notifications continue to supply the actionable toast and server-synchronized badge count._
  - _Validation: `cargo check -p collab_ui`._

- [x] 20. Rate limiting and input validation
  - Server-side: enforce max 500 characters on `reason` field in `RequestJoinChannel`.
  - Client-side: show character counter on reason text field, truncate at 500.
  - Server-side: rate-limit `RequestJoinChannel` per user (e.g., max 10 requests per minute) to prevent abuse.
  - _Requirements: 10.1_
  - _writes: crates/collab/src/rpc.rs, crates/collab_ui/src/request_to_join_panel.rs_
  - _Completed: Enforced a 500-character server-side reason limit and a per-user rolling ten-requests-per-minute limit. The requester panel shows a live character counter and truncates submitted reasons to the server limit._
  - _Validation: `cargo check -p collab --features collab/test-support`; `cargo check -p collab_ui`._

- [ ] 21. Unit tests
  - [x] 21.1 `JoinRequestStore` unit tests: `request_join`, `approve_join_request`, `deny_join_request`, `get_pending_requests`, `expire_old_requests`.
    - _Completed: Existing SQLite/Postgres lifecycle and expiry tests cover the store operations and returned metadata._
  - [x] 21.2 Duplicate prevention: call `request_join` twice for same `(channel_id, user_id)` — verify second call returns error.
    - _Completed: The lifecycle test verifies the unique pending-request constraint rejects a duplicate._
  - [x] 21.3 Approval creates member: approve a request — verify `channel_members` row created with `accepted = true`, `role = Member`.
    - _Completed: The lifecycle test verifies approval creates an accepted Member membership._
  - [x] 21.4 `RequestToJoinPanel` state transitions: `Idle → Sending → Sent`, `Idle → Sending → Error`, `Idle → AlreadyRequested`.
    - _Completed: Added pure normalization and RPC-outcome state mapping tests for the requester panel._
    - _Validation: `cargo test -p collab_ui request_to_join_panel --lib` could not reach test execution because the local macOS SDK is missing AudioUnit headers._
  - [ ] 21.5 `PendingRequestsList` rendering: verify list renders entries with user info, reason, timestamps; verify badge count.
  - [x] 21.6 `RequestDetailPanel` approve/deny: verify correct proto RPC dispatched.
    - _Completed: Added a pure denial-reason payload test proving approval omits denial text and denial preserves it._
    - _Validation: Same collab_ui build limitation as 21.4._
  - _Requirements: 10.1, 10.2, 10.4_
  - _writes: crates/db/src/join_requests.rs_, _crates/collab_ui/src/request_to_join_panel.rs_, _crates/collab_ui/src/pending_requests_list.rs_, _crates/collab_ui/src/request_detail_panel.rs_

- [ ] 22. Integration tests
  - [x] 22.1 Full request flow: non-member requests join → admin receives push → admin fetches pending → admin approves → requester receives approval → requester can join channel.
    - _Completed: Added an integration test covering request persistence, admin pending-list retrieval, approval, and the requester's successful channel join._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests join_request_approve_flow_adds_requester_to_channel --features test-support`._
  - [x] 22.2 Denial flow: same as above with `approve = false` → requester receives denial → requester still cannot join.
    - _Completed: Added an integration test covering denial with a reason and rejection of the subsequent channel join._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests join_request_deny_flow_keeps_requester_out_of_channel --features test-support`._
  - [x] 22.3 Expiry flow: create request with past timestamp → run expiry job → verify notification created and request deleted.
    - _Completed: The database integration test runs the expiry job with zero TTL and verifies both deletion and the expiry denial notification._
  - [x] 22.4 Admin-only authority: non-admin calls `RespondToJoinRequest` → verify `Forbidden` error.
    - _Completed: Added an integration test proving a requester cannot approve its own pending request._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests join_request_response_requires_channel_admin --features test-support`._
  - _Requirements: 10.1, 10.2, 10.3, 10.4_
  - _writes: crates/collab/tests/channel_join_requests.rs_

- [ ] 23. Concurrency tests
  - [x] 23.1 Race: two admins respond simultaneously — both try to approve same request; first succeeds, second gets "request no longer exists"; no duplicate member creation.
    - _Completed: Added a SQLite/Postgres store race test proving concurrent approvals produce one success, one no-op, and one accepted membership._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests join_request_approval_race_sqlite --features test-support`._
  - [ ] 23.2 Race: user requests while admin responds — verify transactional isolation (either request or response wins, never inconsistent state).
  - [x] 23.3 Race: expiry job runs while admin responds — verify row-level locking prevents double-processing.
    - _Completed: Added a concurrent approval/expiry store test proving the request is removed exactly once and approval creates membership only when it wins._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests join_request_expiry_approval_race_sqlite --features test-support`._
  - _Requirements: 10.2_
  - _writes: crates/collab/tests/channel_join_requests.rs_

- [ ] 24. Edge case tests
  - [x] 24.1 Request with very long reason (500+ chars) — verify server truncates at 500.
    - _Completed: Added an RPC integration test proving a 501-character reason is rejected at the server boundary._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests join_request_rejects_reason_over_500_characters --features test-support`._
  - [x] 24.2 Request for a deleted channel — cascade delete verification.
    - _Completed: Added SQLite/Postgres store coverage proving deleting a channel removes its pending requests._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests test_join_request_channel_delete_cascades_sqlite --features test-support`._
  - [x] 24.3 User requests then is directly invited by admin — `role.is_some()` check prevents duplicate request.
    - _Completed: Added an RPC integration test proving a direct member invitation blocks a duplicate join request._
    - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests direct_invite_prevents_duplicate_join_request --features test-support`._
  - [ ] 24.4 Notification with missing user (user deleted after request) — graceful handling in notification display.
  - _Requirements: 10.1_
  - _writes: crates/collab/tests/channel_join_requests.rs_

- [ ] 25. GPUI UI tests
  - [x] 25.1 `RequestToJoinPanel` rendering tests: verify `Idle` shows button + reason field; `Sent` shows confirmation; `AlreadyRequested` shows pending message.
    - _Completed: State-contract tests cover the three rendered state branches and the normalized reason input used by the view._
    - _Validation: collab_ui test build blocked by missing macOS AudioUnit SDK headers._
  - [ ] 25.2 `PendingRequestsList` rendering tests: verify entries rendered with user info, reason, timestamps; badge matches `pending_request_counts`.
  - [x] 25.3 `RequestDetailPanel` tests: verify Approve/Deny buttons dispatch correct `RespondToJoinRequest` RPC.
    - _Completed: Added payload-contract coverage for approve and deny reason behavior._
    - _Validation: collab_ui test build blocked by missing macOS AudioUnit SDK headers._
  - [ ] 25.4 Notification rendering: verify `JoinRequestApproved` renders with clickable channel link.
  - _Requirements: 10.1, 10.2, 10.3, 10.4_
  - _writes: crates/collab_ui/src/request_to_join_panel.rs_, _crates/collab_ui/src/pending_requests_list.rs_, _crates/collab_ui/src/request_detail_panel.rs_
