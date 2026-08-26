# Tasks: Cargo execution presets and actions

## Milestone 0: Verified execution baseline

- [x] 1. Verify existing preset and contextual action behavior
  - [x] 1.1. Record preset, Tasks and DAP evidence
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 3.2, 3.3_
    - _Depends on: none_
    - _Reads: crates/cargo_ui/src/cargo_preset.rs, crates/cargo_ui/src/cargo_actions.rs, crates/cargo_ui/src/cargo_panel.rs, crates/workspace/src/tasks.rs, docs/src/tasks.md, docs/src/debugger.md_
    - _Writes: none_
    - _Validation: cargo test -p cargo_ui; cargo test -p zed --features test-support,rust-tools cargo_panel_
    - Design: D1, D2, D6
    - Outcome: Current six actions and version-1 preset compilation are accepted as canonical baseline.
    - Done when: Existing tests prove structured argv/env, applicability, Tasks/DAP dispatch, safe persistence and stale selection handling.
    - Evidence: `cargo_preset_compiler_preserves_argv_env_and_dap_shape`, action eligibility/dispatcher tests, Cargo panel feature tests, and task/debug documentation.

## Milestone 1: Configuration and authoring gaps

- [x] 2. Extend presets without adding an execution subsystem
  - [x] 2.1. Add toolchain and pre-launch task fields
    - Version and migrate the schema, compile an explicit toolchain without installation, and resolve pre-launch only by existing stable Tasks references and lifecycle.
    - _Requirements: 2.1, 2.2, 3.1, 3.2, 3.3_
    - _Depends on: none_
    - _Reads: crates/cargo_ui/src/cargo_preset.rs, crates/settings_content/src/workspace.rs, crates/task/src/task.rs, crates/workspace/src/tasks.rs_
    - _Writes: crates/cargo_ui/src/cargo_preset.rs, crates/settings_content/src/workspace.rs, crates/workspace/src/tasks.rs_
    - _Validation: cargo test -p cargo_ui cargo_preset_v2; cargo test -p workspace cargo_pre_launch_task_
    - Design: D1, D2, D3, D6
    - Outcome: Missing fields compile through existing host Tasks/DAP with bounded migration and no installer.
    - Done when: Version-1 settings migrate, invalid references isolate, pre-launch failure prevents main launch, and remote context remains unchanged.
    - _Evidence: `cargo test -p cargo_ui cargo_preset_v2 -- --nocapture` and `cargo test -p workspace cargo_pre_launch_task -- --nocapture` passed. Version 1 migrates to version 2, toolchains remain structured argv, and exact unique Tasks references gate the main Task/DAP continuation on successful lifecycle completion._
  - [x] 2.2. Add the ephemeral editor, redacted preview and explicit save workflow
    - Build a draft editor in the Cargo UI, redacted token preview, Run/Debug-without-save, and explicit existing-settings writes for User/Project.
    - _Requirements: 2.3, 2.4, 3.1, 3.2, 3.3_
    - _Depends on: 2.1_
    - _Reads: crates/cargo_ui/src/cargo_panel.rs, crates/cargo_ui/src/cargo_preset.rs, crates/settings/src/settings_store.rs, crates/ui/src/components_
    - _Writes: crates/cargo_ui/src/cargo_panel.rs, crates/cargo_ui/src/cargo_preset.rs, crates/cargo_ui/src/cargo_preset_editor.rs_
    - _Validation: cargo test -p cargo_ui cargo_preset_editor; cargo test -p cargo_ui cargo_preset_save_scope_
    - Design: D3, D4, D6
    - Outcome: Users can inspect, run and explicitly save presets without leaking secrets or replacing settings infrastructure.
    - Done when: GPUI tests cover validation, keyboard/focus, redaction, user/project confirmation, trust denial and restart recovery.
    - _Evidence: `cargo test -p cargo_ui cargo_preset_editor -- --nocapture` passed 3 focused tests and `cargo test -p cargo_ui cargo_preset_save_scope -- --nocapture` passed 1 focused test. The modal covers keyboard focus, ephemeral Run, validation, redacted previews, trust/confirmation, environment stripping and restart recovery._

## Milestone 2: Compatibility and optional action extensions

- [x] 3. Close compatibility coverage and separately approve extra actions
  - [x] 3.1. Add dedicated tasks.json and debug.json coexistence tests
    - Load representative global/project definitions, run user and Cargo actions, and assert no definition, priority, history or DAP scenario is replaced.
    - _Requirements: 1.10, 2.7, 3.1_
    - _Depends on: none_
    - _Reads: crates/task/src/task.rs, crates/project/src/task_store.rs, crates/workspace/src/tasks.rs, crates/debugger_tools, crates/cargo_ui/src/cargo_actions.rs_
    - _Writes: crates/cargo_ui/tests/cargo_configuration_compatibility.rs_
    - _Validation: cargo test -p cargo_ui --test cargo_configuration_compatibility_
    - Design: D2, D6
    - Outcome: Configuration coexistence has explicit regression coverage.
    - Done when: Both JSON scopes and generated Cargo plans remain independently selectable and byte-equivalent after dispatch.
    - _Evidence: `cargo test -p cargo_ui --test cargo_configuration_compatibility -- --nocapture` passed. The fixture loads global/project Tasks and debug scenarios, records user and Cargo histories, preserves project-over-global priority, and compares all authored definitions byte-for-byte._
  - [x] 3.2. Add approved Doc, Clippy, Fmt, Clean and Tree task plans
    - Extend the action table only after product approval; require Clean confirmation, locked/offline Tree, and no Update action.
    - _Requirements: 2.5, 2.6, 3.1_
    - _Depends on: 2.2, 3.1_
    - _Reads: crates/cargo_ui/src/cargo_actions.rs, crates/cargo_ui/src/cargo_panel.rs, crates/task/src/task.rs_
    - _Writes: crates/cargo_ui/src/cargo_actions.rs, crates/cargo_ui/src/cargo_panel.rs_
    - _Validation: cargo test -p cargo_ui cargo_extended_action_matrix; cargo test -p cargo_ui cargo_clean_confirmation_
    - Design: D5, D6
    - Outcome: Approved actions remain explicit Tasks with truthful safety/capability gating.
    - Done when: Exact argv and unavailable reasons are asserted, Clean cannot run without confirmation, and source contains no Update plan.
    - _Evidence: `cargo test -p cargo_ui cargo_extended_action_matrix -- --nocapture` and `cargo test -p cargo_ui cargo_clean_confirmation -- --nocapture` each passed. Tests assert exact Doc/Clippy/Fmt/Clean/Tree argv, locked/offline Tree, accessible scope denials, default Clean rejection, confirmed Clean planning and absence of an Update variant or plan._

## Mandatory manual task-decomposition audit

- Baseline verification does not recreate implemented compilers or dispatchers.
- Schema/lifecycle, authoring UI, compatibility fixtures and optional actions are independently reviewable leaves.
- No leaf writes CargoWorkspaceStore or generic structured-result code.
- Overlapping Cargo UI writes are sequenced; every criterion maps to D1–D6 and at least one leaf.
