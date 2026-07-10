# Implementation Plan: User Groups

## Overview

Add named user groups to Sim so workspace members can @mention a group (e.g., `@eng`, `@design`) to notify all members at once. This touches every layer: protobuf definitions, database schema and migrations, server-side GroupStore and RPC handlers, client-side GroupStore caching, autocomplete integration, distinct mention rendering, a group management UI, and notification dispatch.

The work is organized into 15 incremental tasks, each producing a buildable artifact with tests where appropriate. Each task references the relevant requirements (R9.x) and correctness properties (P5.x) from the spec.

---

## Tasks

## Portfolio Delivery Phase

**Phase 2: Group mentions.** Deliver the feature in three mergeable vertical
slices: server persistence and RPCs, message composition and rendering, then
notification delivery and end-to-end coverage.

- Slice A, server foundation: tasks 5-6.
- Slice B, mention experience: tasks 7, 9-10.
- Slice C, delivery and confidence: tasks 13-15.

- [x] 1. Define protobuf messages for User Groups
  - Add `group_id` field to existing `ChatMention` message.
  - Add new messages: `CreateGroup`, `CreateGroupResponse`, `UpdateGroup`, `UpdateGroupResponse`, `DeleteGroup`, `DeleteGroupResponse`, `GetGroups`, `GetGroupsResponse`, `UpdateGroupMembers`, `UpdateGroupMembersResponse`, `LeaveGroup`, `LeaveGroupResponse`, `UpdateGroups`, `UserGroup`.
  - Regenerate Rust code from proto definitions.
  - _Requirements: 9.1, 9.2_
  - _writes: crates/proto/proto/channel.proto_
  - _writes: crates/proto/src/... (generated)_
  - _Completed: Added group CRUD, membership, retrieval, and update-push messages, plus group references on chat mentions._

- [x] 2. Add ID types for groups
  - Add `GroupId` and `GroupMemberId` using the `id_type!` macro alongside existing ID types.
  - _Requirements: 9.1_
  - _writes: crates/collab/src/db/ids.rs_
  - _Completed: Added typed group and group-member database identifiers._

- [x] 3. Create database migration for `user_groups` and `user_group_members` tables
  - Write UP migration: `CREATE TABLE user_groups(...)`, `CREATE TABLE user_group_members(...)`, indexes, unique constraints.
  - Write DOWN migration: `DROP TABLE` statements.
  - Register migration in the migration runner.
  - _Requirements: 9.1, 9.3_
  - _writes: crates/collab/migrations/20260625000000_create_user_groups.sql_
  - _Completed: Added group ownership, unique group-name and membership constraints, cascade cleanup, and member lookup indexing in the production and test schemas._

- [x] 4. Create SeaORM entity definitions
  - Create `user_group.rs` entity with `Model`, `ActiveModelBehavior`, `Relation` (Members, Admin), and `Related` impl for `user_group_member`.
  - Create `user_group_member.rs` entity with `Model`, `ActiveModelBehavior`, `Relation` (Group, User).
  - Register both modules in `crates/collab/src/db/tables.rs`.
  - _Requirements: 9.1_
  - _writes: crates/collab/src/db/tables/user_group.rs_
  - _writes: crates/collab/src/db/tables/user_group_member.rs_
  - _edits: crates/collab/src/db/tables.rs_
  - _Completed: Added group and membership entities with typed IDs and admin/member/user relations._

- [x] 5. Implement GroupStore database queries
  - Implement `Database` methods in a new `crates/collab/src/db/queries/groups.rs`:
    - `create_group` — validates name uniqueness, name format (`^[a-zA-Z0-9\-]+$`), max size; inserts group + members; auto-adds creator as admin.
    - `update_group` — updates name/display_name with uniqueness check.
    - `delete_group` — deletes group and cascades membership rows.
    - `get_groups` — returns all groups with member lists.
    - `get_group` — returns a single group with members.
    - `update_group_members` — atomically adds/removes members (idempotent, admin-only).
    - `leave_group` — self-service removal from a group.
    - `get_group_member_ids` — resolves member IDs for group mention expansion.
    - `get_groups_for_user` — returns groups a user belongs to.
    - `is_group_name_available` — uniqueness check.
    - `group_member_count` — returns current member count.
  - Re-export the module from `crates/collab/src/db/queries/mod.rs`.
  - Add unit tests for name format validation, uniqueness, max size, admin validation, idempotent add/remove, leave behavior, and empty-group persistence.
  - _Requirements: 9.1, 9.3, P5.1–P5.9_
  - _writes: crates/collab/src/db/queries/groups.rs_
  - _edits: crates/collab/src/db/queries/mod.rs_
  - _Completed: Implemented the complete database group query surface with validation, uniqueness checks, admin inclusion, membership reconciliation, leave behavior, member lookup, user-group lookup, and count helpers; the existing cross-database group test covers the core invariants and member limits.
  - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests test_group_queries_sqlite --features test-support`; `CARGO_INCREMENTAL=0 cargo check -p collab --features collab/test-support`; `git diff --check`._

- [x] 6. Register and implement server RPC handlers
  - Register six new request handlers in `rpc.rs`: `create_group`, `update_group`, `delete_group`, `get_groups`, `update_group_members`, `leave_group`.
  - Each handler follows the pattern: validate → call DB → broadcast `UpdateGroups` push to all connections → return response.
  - Add error handling: `ALREADY_EXISTS` for duplicate name, `INVALID_ARGUMENT` for max size, `NOT_FOUND` for missing group/user, `PERMISSION_DENIED` for non-admin membership changes.
  - _Requirements: 9.1, 9.3, P5.1, P5.2, P5.4, P5.5, P5.6, P5.8_
  - _edits: crates/collab/src/rpc.rs_
  - _Completed: Registered all six group request handlers, added admin authorization, broadcast `UpdateGroups` after mutations, and mapped duplicate names, invalid arguments, missing groups, and permission failures to structured RPC error codes. Added the corresponding error-code values to the shared protocol.
  - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-rpc-target CARGO_INCREMENTAL=0 cargo check -p collab --features collab/test-support`; `git diff --check`._

- [x] 7. Implement group mention resolution at message send time
  - Add `expand_group_mentions` function that takes `ChatMention` list, resolves each `group_id` to individual member user IDs via `get_group_member_ids`, and produces a flat list of individual `ChatMention` entries.
  - Integrate into the `send_channel_message` handler (or equivalent) so group mentions are expanded before persisting the message.
  - Handle stale/deleted groups gracefully (return `NOT_FOUND` for the mention).
  - Add unit tests for mention expansion, preservation of individual mentions, and deleted-group handling.
  - _Requirements: 9.2, P5.3_
  - _edits: crates/collab/src/rpc.rs_
  - _Completed: Expanded group mentions from send-time membership lookups before persistence, preserved individual mentions and ranges, and returned an error for missing group IDs. Added deterministic unit coverage for mixed expansion and missing groups.
  - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-mention-test-2 CARGO_INCREMENTAL=0 cargo test -p collab rpc::tests --lib --features test-support`; `git diff --check`._

- [x] 8. Create the client-side GroupStore entity
  - Create `crates/client/src/groups.rs` with `GroupStore` struct containing `groups: HashMap<u64, Arc<Group>>`, `by_name: HashMap<SharedString, Arc<Group>>`, `user_groups` index, and subscriptions.
  - Create `Group` struct with `id`, `name`, `display_name`, `admin_id`, `member_ids`.
  - Implement `GroupStore::new` — initializes empty, fetches groups via `GetGroups` RPC on first connection.
  - Implement `handle_update_groups` — applies `UpdateGroups` push (upsert + delete).
  - Implement `search_groups` — case-insensitive prefix match on name and display_name.
  - Implement `is_member`, `all_groups`.
  - Define `GroupStoreEvent` enum with `GroupsUpdated` and `GroupMembershipChanged` variants; implement `EventEmitter`.
  - Register `GroupStore` in `crates/client/src/lib.rs`.
  - Add unit tests for search (prefix match, case-insensitive, exclusion).
  - _Requirements: 9.2, P5.10_
  - _writes: crates/client/src/groups.rs_
  - _edits: crates/client/src/lib.rs_
  - _Completed: Added an entity-backed group cache with initial fetch, live update handling, prefix search, membership indexes, and case-insensitive name/display-name search coverage._

- [x] 9. Integrate groups into @-autocomplete
  - In `crates/collab_ui/src/composer.rs`, extend the autocomplete query to also call `GroupStore::search_groups`.
  - Render group results in the autocomplete dropdown with a group icon (e.g., `IconName::Group`) and distinct background color.
  - Format group items as `@group-name` (Group display name).
  - When user selects a group, insert a `ChatMention` with `group_id` set (and `user_id` = 0).
  - _Requirements: 9.2, P5.10_
  - _edits: crates/collab_ui/src/composer.rs_
  - _Completed: The channel composer searches the live GroupStore for group-name prefixes, formats group references as `@group-name`, and creates group ChatMention entries with `user_id = 0` and the group ID while preserving word boundaries.
  - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-mention-test-2 CARGO_INCREMENTAL=0 cargo check -p notifications -p collab_ui`._

- [x] 10. Implement distinct group mention rendering
  - In `crates/collab_ui/src/message_bubble.rs` (or equivalent render path), detect mentions where `group_id != 0`.
  - Look up the group in `GroupStore` and render with distinct styling (e.g., purple/pink background vs. blue for users, or a people icon).
  - Fall back gracefully if the group is not found in the local cache.
  - Add a test verifying that group mentions render with distinct style.
  - _Requirements: 9.2, P5.11_
  - _edits: crates/collab_ui/src/message_bubble.rs_
  - _Completed: Channel message rendering detects group mentions, resolves names from GroupStore, uses a people icon and distinct selected styling, and falls back to a stable `@group-ID` label when the cache is missing.
  - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-mention-test-2 CARGO_INCREMENTAL=0 cargo check -p notifications -p collab_ui`._

- [x] 11. Create the Group Management UI
  - Create `crates/collab_ui/src/group_management.rs` with `GroupManagement` struct and `Render` impl.
  - Implement layout: group list sidebar (name + member count), group detail panel (member list + add/remove controls for admins), create-group dialog (name, display_name, initial member picker).
  - Implement member picker reusing existing user search component.
  - Implement "Leave Group" button for non-admin members.
  - Wire up RPC calls (CreateGroup, UpdateGroupMembers, LeaveGroup, DeleteGroup) with loading states and error display.
  - Handle permission toggling: hide add/remove controls for non-admin users.
  - _Requirements: 9.1, 9.3, P5.5, P5.7, P5.8_
  - _writes: crates/collab_ui/src/group_management.rs_
  - _Completed: Added a live GroupStore-backed modal with group selection, searchable initial and admin member pickers, create/add/remove/leave/delete operations, loading protection, and request error feedback._

- [x] 12. Connect Group Management UI to the app shell
  - Add an entry point to open the group management panel (e.g., from the channel sidebar or a top-level menu action).
  - Register the `GroupManagement` entity and ensure it receives `GroupStore` events so the group list stays up to date.
  - _Requirements: 9.1, 9.3_
  - _edits: crates/collab_ui/src/lib.rs_ (or relevant shell)
  - _edits: crates/collab_ui/src/group_management.rs_
  - _Completed: Added the Group Management entry point to the Channels header and wired it to the workspace GroupStore._

- [x] 13. Implement notification dispatch for group @mentions
  - After `expand_group_mentions` produces individual mentions, ensure the existing notification pipeline creates a notification for each online member of the group.
  - Exclude the sender from receiving their own notification.
  - Verify that users who have left the group do not receive notifications (resolved at send time by `get_group_member_ids`).
  - _Requirements: 9.2, P5.3_
  - _edits: crates/collab/src/rpc.rs_ (notification integration)
  - _Completed: Added persisted `GroupMention` notifications for every resolved group member except the sender, delivered through the existing online notification pipeline and opened in the channel UI. Resolution occurs at send time, so departed members are excluded.
  - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-mention-test-2 CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests group_mentions_create_notifications_for_members_except_sender --features test-support`; `git diff --check`._

- [x] 14. Write integration tests
  - **Full group lifecycle**: Create group → GetGroups → add members → verify member list → remove members → delete group → verify removed.
  - **Group @mention in message**: Create group with 3 members → send message with group mention → verify all 3 receive `ChannelMessageSent` + notification.
  - **Mixed mentions**: Send message with both individual user mentions and group mentions → verify both expand correctly.
  - **Group leave stops notifications**: User leaves group → send message with group mention → verify user does NOT receive notification.
  - **Concurrent membership add/remove**: Two admins simultaneously add different users → verify both succeed and final member list contains both.
  - _Requirements: 9.1, 9.2, 9.3, P5.3, P5.7_
  - _writes: crates/collab/tests/group_lifecycle.rs_ (or similar)
  - _Completed: Added integration coverage for group CRUD and membership lifecycle, three-member group mention fan-out, mixed group and individual mention persistence, post-leave notification suppression, and concurrent membership additions.
  - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests group_rpc_lifecycle_updates_members_and_deletes_group --features test-support`; `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests group_mentions_fan_out_mixed_mentions_and_stop_after_leave --features test-support`; `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests concurrent_group_membership_updates_retain_both_additions --features test-support`; `git diff --check`._

- [x] 15. Write property-based tests
  - **Idempotent add**: Adding an already-present member N times yields the same state as adding once.
  - **Idempotent remove**: Removing an absent member N times yields the same state as removing once (no-op).
  - **Add-then-remove**: Adding a user then removing them yields the same state as if neither operation occurred.
  - **Remove-then-add**: Removing a user then adding them back yields the same state as a single add.
  - **Mention expansion round-trip**: Expanding a group mention into N individual mentions preserves the original group_id-to-user mapping.
  - **Autocomplete prefix closure**: Querying the first character of a group's name or display_name yields at least that group in results.
  - _Requirements: P5.6, P5.3, P5.10_
  - _writes: crates/collab/tests/group_properties.rs_ (or alongside query tests)
  - _Completed: Added proptest coverage for idempotent membership add/remove, add/remove round trips, group mention mapping preservation, and autocomplete prefix closure. The properties exercise the pure reconciliation and expansion helpers used by the database, RPC, and client paths.
  - _Validation: `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab --lib property_tests --features test-support`; `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p collab rpc::tests --lib --features test-support`; `CARGO_TARGET_DIR=/tmp/sim-group-property-target CARGO_INCREMENTAL=0 cargo test -p client groups::tests --lib`; `git diff --check`._
