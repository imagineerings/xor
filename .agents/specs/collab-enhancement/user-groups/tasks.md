# Implementation Plan: User Groups

## Overview

Add named user groups to Sim so workspace members can @mention a group (e.g., `@eng`, `@design`) to notify all members at once. This touches every layer: protobuf definitions, database schema and migrations, server-side GroupStore and RPC handlers, client-side GroupStore caching, autocomplete integration, distinct mention rendering, a group management UI, and notification dispatch.

The work is organized into 15 incremental tasks, each producing a buildable artifact with tests where appropriate. Each task references the relevant requirements (R9.x) and correctness properties (P5.x) from the spec.

---

## Tasks

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

- [ ] 4. Create SeaORM entity definitions
  - Create `user_group.rs` entity with `Model`, `ActiveModelBehavior`, `Relation` (Members, Admin), and `Related` impl for `user_group_member`.
  - Create `user_group_member.rs` entity with `Model`, `ActiveModelBehavior`, `Relation` (Group, User).
  - Register both modules in `crates/collab/src/db/tables.rs`.
  - _Requirements: 9.1_
  - _writes: crates/collab/src/db/tables/user_group.rs_
  - _writes: crates/collab/src/db/tables/user_group_member.rs_
  - _edits: crates/collab/src/db/tables.rs_

- [ ] 5. Implement GroupStore database queries
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

- [ ] 6. Register and implement server RPC handlers
  - Register six new request handlers in `rpc.rs`: `create_group`, `update_group`, `delete_group`, `get_groups`, `update_group_members`, `leave_group`.
  - Each handler follows the pattern: validate → call DB → broadcast `UpdateGroups` push to all connections → return response.
  - Add error handling: `ALREADY_EXISTS` for duplicate name, `INVALID_ARGUMENT` for max size, `NOT_FOUND` for missing group/user, `PERMISSION_DENIED` for non-admin membership changes.
  - _Requirements: 9.1, 9.3, P5.1, P5.2, P5.4, P5.5, P5.6, P5.8_
  - _edits: crates/collab/src/rpc.rs_

- [ ] 7. Implement group mention resolution at message send time
  - Add `expand_group_mentions` function that takes `ChatMention` list, resolves each `group_id` to individual member user IDs via `get_group_member_ids`, and produces a flat list of individual `ChatMention` entries.
  - Integrate into the `send_channel_message` handler (or equivalent) so group mentions are expanded before persisting the message.
  - Handle stale/deleted groups gracefully (return `NOT_FOUND` for the mention).
  - Add unit tests for mention expansion, preservation of individual mentions, and deleted-group handling.
  - _Requirements: 9.2, P5.3_
  - _edits: crates/collab/src/rpc.rs_

- [ ] 8. Create the client-side GroupStore entity
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

- [ ] 9. Integrate groups into @-autocomplete
  - In `crates/collab_ui/src/composer.rs`, extend the autocomplete query to also call `GroupStore::search_groups`.
  - Render group results in the autocomplete dropdown with a group icon (e.g., `IconName::Group`) and distinct background color.
  - Format group items as `@group-name` (Group display name).
  - When user selects a group, insert a `ChatMention` with `group_id` set (and `user_id` = 0).
  - _Requirements: 9.2, P5.10_
  - _edits: crates/collab_ui/src/composer.rs_

- [ ] 10. Implement distinct group mention rendering
  - In `crates/collab_ui/src/message_bubble.rs` (or equivalent render path), detect mentions where `group_id != 0`.
  - Look up the group in `GroupStore` and render with distinct styling (e.g., purple/pink background vs. blue for users, or a people icon).
  - Fall back gracefully if the group is not found in the local cache.
  - Add a test verifying that group mentions render with distinct style.
  - _Requirements: 9.2, P5.11_
  - _edits: crates/collab_ui/src/message_bubble.rs_

- [ ] 11. Create the Group Management UI
  - Create `crates/collab_ui/src/group_management.rs` with `GroupManagement` struct and `Render` impl.
  - Implement layout: group list sidebar (name + member count), group detail panel (member list + add/remove controls for admins), create-group dialog (name, display_name, initial member picker).
  - Implement member picker reusing existing user search component.
  - Implement "Leave Group" button for non-admin members.
  - Wire up RPC calls (CreateGroup, UpdateGroupMembers, LeaveGroup, DeleteGroup) with loading states and error display.
  - Handle permission toggling: hide add/remove controls for non-admin users.
  - _Requirements: 9.1, 9.3, P5.5, P5.7, P5.8_
  - _writes: crates/collab_ui/src/group_management.rs_

- [ ] 12. Connect Group Management UI to the app shell
  - Add an entry point to open the group management panel (e.g., from the channel sidebar or a top-level menu action).
  - Register the `GroupManagement` entity and ensure it receives `GroupStore` events so the group list stays up to date.
  - _Requirements: 9.1, 9.3_
  - _edits: crates/collab_ui/src/lib.rs_ (or relevant shell)
  - _edits: crates/collab_ui/src/group_management.rs_

- [ ] 13. Implement notification dispatch for group @mentions
  - After `expand_group_mentions` produces individual mentions, ensure the existing notification pipeline creates a notification for each online member of the group.
  - Exclude the sender from receiving their own notification.
  - Verify that users who have left the group do not receive notifications (resolved at send time by `get_group_member_ids`).
  - _Requirements: 9.2, P5.3_
  - _edits: crates/collab/src/rpc.rs_ (notification integration)

- [ ] 14. Write integration tests
  - **Full group lifecycle**: Create group → GetGroups → add members → verify member list → remove members → delete group → verify removed.
  - **Group @mention in message**: Create group with 3 members → send message with group mention → verify all 3 receive `ChannelMessageSent` + notification.
  - **Mixed mentions**: Send message with both individual user mentions and group mentions → verify both expand correctly.
  - **Group leave stops notifications**: User leaves group → send message with group mention → verify user does NOT receive notification.
  - **Concurrent membership add/remove**: Two admins simultaneously add different users → verify both succeed and final member list contains both.
  - _Requirements: 9.1, 9.2, 9.3, P5.3, P5.7_
  - _writes: crates/collab/tests/group_lifecycle.rs_ (or similar)

- [ ] 15. Write property-based tests
  - **Idempotent add**: Adding an already-present member N times yields the same state as adding once.
  - **Idempotent remove**: Removing an absent member N times yields the same state as removing once (no-op).
  - **Add-then-remove**: Adding a user then removing them yields the same state as if neither operation occurred.
  - **Remove-then-add**: Removing a user then adding them back yields the same state as a single add.
  - **Mention expansion round-trip**: Expanding a group mention into N individual mentions preserves the original group_id-to-user mapping.
  - **Autocomplete prefix closure**: Querying the first character of a group's name or display_name yields at least that group in results.
  - _Requirements: P5.6, P5.3, P5.10_
  - _writes: crates/collab/tests/group_properties.rs_ (or alongside query tests)
