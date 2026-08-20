# Implementation Plan: Rust Development Workspace

## Planning rules

This plan implements only audited gaps. The Cargo tree host, Cargo metadata model/store, panel, protocol, navigation, direct-dependency projection, remote metadata flow, and initial `rust-tools` boundary are accepted as completed baseline. They are not re-created as tasks. Where a leaf touches one of those components, its delta is stated explicitly.

All executable work is a top-level checkbox leaf. Milestone and epic headings are parent planning units, not executable tasks. Durable `_id` values survive renumbering. Numeric `_Depends on` values support the repository feature-spec validator; `_blocked_by` carries the same graph by durable ID for execution tooling.

## Dependency waves

| Wave | Leaves | Exit condition |
| --- | --- | --- |
| 1 | 1 | Cargo configuration facts exist behind the current store boundary. |
| 2 | 2 | The existing Cargo panel presents facts and preserves its baseline. |
| 3 | 3 | Presets merge, persist, and compile purely. |
| 4 | 4 | Contextual actions route through Tasks/DAP. |
| 5 | 5 | Generic structured result state/protocol exists. |
| 6 | 6 | Existing terminal Tasks expose bounded lifecycle/cancellation handles. |
| 7 | 7 | Generic Tests panel renders fake-provider results. |
| 8 | 8 | Rust discovery protocol passes the required fixture gate. |
| 9 | 9 | Rust discovery works on authoritative local/remote hosts. |
| 10 | 10 | Rust run/debug/result actions work through Tasks/DAP. |
| 11 | 11 | Feature, trust, privacy, remote, and mismatch boundaries are enforced. |
| 12 | 12 | Scale, GPUI, hermetic, docs, and complete validation pass. |

## Milestone Now A: Cargo dashboard evolution

### Epic A1 parent: Extend model discovery with configuration facts

This epic adds profiles and bounded toolchain/target facts to the existing Cargo snapshot. It does not add user command execution to `CargoWorkspaceStore`.

- [x] 1. Extend Cargo snapshots with bounded profile and toolchain facts
  - _id: rust-workspace-cargo-configuration-model_
  - Add implicit and custom profile descriptors, declared toolchain descriptors, host compiler facts, completeness, and bounded diagnostics to `project::cargo_workspace` and `cargo.proto` using backward-compatible fields.
  - Add visible-worktree parsers for `[profile.*]`, `rust-toolchain.toml`, and legacy `rust-toolchain`, plus an injectable configuration probe whose production implementation runs only bounded `rustc -vV` through the existing trusted `ProjectEnvironment` on the authoritative host.
  - Extend the current store's invalidation/fingerprint lifecycle for relevant manifests, lockfiles, toolchain declarations, and visible Cargo config files. Keep Cargo builds/tests/runs outside the store and label Cargo-config-derived target state unresolved rather than reimplementing Cargo configuration.
  - Add deterministic fixtures for custom/malformed profiles, toolchain formats, missing/restricted/failed probes, path containment, stale retention, and local/remote snapshot conversion.
  - _Requirements: 1.3, 1.4, 2.1, 2.2, 2.3, 2.7, 2.8, 6.1, 6.2, 6.6, 7.1, 7.2, 7.3, 7.5, 7.6, 7.7, 7.8, 9.1, 9.2, 9.6, 9.7_
  - _Depends on: none_
  - _Reads: crates/project/src/cargo_workspace.rs, crates/project/src/cargo_workspace_store.rs, crates/project/src/environment.rs, crates/project/src/worktree_store.rs, crates/project/src/trusted_worktrees.rs, crates/proto/proto/cargo.proto, crates/languages/src/rust.rs, crates/project/test_data/cargo_workspace_
  - _Writes: crates/project/src/cargo_workspace.rs, crates/project/src/cargo_workspace_store.rs, crates/project/tests/integration/cargo_workspace.rs, crates/project/test_data/cargo_workspace, crates/proto/proto/cargo.proto, crates/proto/src/proto.rs, crates/cargo_ui/src/cargo_panel.rs (fixture compatibility)_
  - _Validation: `cargo test -p project --features test-support,cargo-workspace cargo_workspace -- --nocapture`; `cargo test -p proto cargo_workspace`; `cargo check -p project --no-default-features`_
  - Outcome: A generation-tagged Cargo snapshot reports profile/toolchain/host-target facts and partial failures without expanding the store into a general command runner.
  - Design: D1, D2, D3, D10, D11, D12, D15
  - Done when: Injected-runner tests prove deterministic local/remote facts, restricted mode makes zero probe calls, malformed roots are isolated, stale snapshots retain safe facts, wire data is bounded/path-safe, and disabled project builds still compile without Cargo support.
  - Evidence: `cargo test -p project --features test-support,cargo-workspace cargo_workspace -- --nocapture` (22 passed); `cargo test -p proto cargo_workspace` (1 passed); `cargo check -p project --no-default-features` (passed); `cargo check -p project --tests --features test-support,cargo-workspace` (passed).

### Epic A2 parent: Present active configuration without regressing the dashboard

This epic changes the existing Cargo projection and panel only. It does not add another dashboard or alter the direct-only dependency decision.

- [x] 2. Add Cargo configuration rows and baseline regression coverage
  - _id: rust-workspace-cargo-configuration-ui_
  - Extend `CargoTreeProvider`/`CargoPanel` with a compact configuration summary and detail rows for profiles, declared toolchain, host compiler target, explicit target override, and unresolved Cargo default. Preserve the existing title, dock integration, lazy activation, toolbar, navigation, read-only dependency tree, stable IDs, partial/stale states, and accessible interaction.
  - Add a default ephemeral active configuration view; it must label Cargo defaults and show environment key names only. Subscribe through the current provider invalidation/debounce path rather than adding panel-owned process work.
  - Add focused GPUI and projection regressions for all verified baseline behaviors affected by the new rows, including virtual/multiple roots, every target/dependency kind, finite direct dependencies, safe navigation, selection/expansion preservation, loading/error states, and dormant activation.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.4, 2.5, 2.6, 2.7, 2.8, 7.4, 7.6, 7.9, 9.1, 9.2, 9.5_
  - _Depends on: 1_
  - _blocked_by: rust-workspace-cargo-configuration-model_
  - _wave: 2_
  - _Reads: crates/cargo_ui/src/cargo_panel.rs, crates/cargo_ui/src/cargo_panel_settings.rs, crates/cargo_ui/src/cargo_ui.rs, crates/language_tools/src/language_tool_tree.rs, crates/project/src/cargo_workspace.rs, crates/workspace/src/dock.rs, crates/panel/src/panel.rs_
  - _Writes: crates/cargo_ui/src/cargo_panel.rs_
  - _Validation: `cargo test -p cargo_ui -- --nocapture`; `cargo test -p language_tools language_tool_tree`; `cargo test -p sim --features test-support,rust-tools cargo_panel`_
  - Outcome: The existing Cargo panel exposes truthful active/configuration facts while retaining its implemented finite, navigable, lazy, accessible dashboard behavior.
  - Design: D1, D3, D4, D12, D15
  - Done when: Fixture projections distinguish explicit/host/unresolved targets, all new rows have stable IDs/accessibility labels, dormant panels trigger no probe, and baseline Cargo panel regressions pass.
  - Evidence: `cargo test -p cargo_ui -- --nocapture` (6 passed); `cargo test -p language_tools language_tool_tree` (8 passed); `CARGO_INCREMENTAL=0 cargo test -p sim --features test-support,rust-tools cargo_panel` (1 passed).

## Milestone Now B: Cargo presets and contextual actions

### Epic B1 parent: Define and persist Cargo execution presets

Presets are Cargo-specific configuration that compile into Tasks/DAP. They are not new task or debug file formats.

- [x] 3. Add Cargo preset schema, precedence, persistence, and pure compilation
  - _id: rust-workspace-cargo-presets_
  - Add versioned Cargo preset settings content behind `rust-tools`, with typed subcommand/scope/target/profile/features/default-feature/target-triple/argument/environment/working-directory and task-presentation fields. Use user and trusted project settings with project-by-ID precedence and per-entry validation isolation.
  - Add pure merge and compiler modules in `cargo_ui` that produce deterministic `TaskTemplate`, `TaskContext` inputs, and `DebugScenario` build templates using structured argv/env fields. Share a Cargo argument builder with existing Rust task creation only where equivalence tests demonstrate identical semantics; do not import task-private metadata types.
  - Persist active preset ID and safe selection overrides through existing workspace persistence/panel serialization. Add an additive migration/fallback for missing, renamed, invalid, or deleted IDs; do not persist result history or environment values in workspace state.
  - _Requirements: 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 3.7, 3.9, 7.4, 7.7, 8.1, 8.2, 9.1, 9.2, 9.3_
  - _Depends on: 2_
  - _blocked_by: rust-workspace-cargo-configuration-ui_
  - _wave: 3_
  - _Reads: crates/settings_content/src/settings_content.rs, crates/settings_content/src/workspace.rs, crates/settings/src/settings_store.rs, crates/cargo_ui/src/cargo_panel_settings.rs, crates/cargo_ui/src/cargo_panel.rs, crates/task/src/task_template.rs, crates/task/src/debug_format.rs, crates/languages/src/rust.rs, crates/workspace/src/persistence.rs_
  - _Writes: crates/settings_content/src/settings_content.rs, crates/settings_content/src/workspace.rs, crates/settings/src/settings.rs, crates/settings/src/vscode_import.rs, crates/cargo_ui/src/cargo_preset.rs, crates/cargo_ui/src/cargo_panel.rs, crates/cargo_ui/src/cargo_ui.rs, crates/cargo_ui/Cargo.toml, crates/workspace/src/persistence.rs, crates/workspace/src/workspace.rs_
  - _Validation: `cargo test -p cargo_ui cargo_preset -- --nocapture`; `cargo test -p settings cargo`; `cargo test -p workspace cargo_preset_persistence`; `cargo check -p settings --no-default-features`_
  - Outcome: Valid user/project/ephemeral presets merge predictably, restore safely, and compile to exact existing Task/DAP data structures without shell concatenation.
  - Design: D4, D5, D11, D14, D15
  - Done when: Schema and adversarial argv/env tests pass, project precedence and invalid-entry isolation are deterministic, active-ID migration falls back visibly, and settings/workspace summaries contain no environment values.
  - Evidence: `CARGO_INCREMENTAL=0 cargo test -p cargo_ui cargo_preset -- --nocapture` (3 passed); `CARGO_INCREMENTAL=0 cargo test -p settings cargo` (disabled boundary, 1 passed); `CARGO_INCREMENTAL=0 cargo test -p settings --features rust-tools cargo` (enabled keyed merge, 1 passed); `CARGO_INCREMENTAL=0 cargo test -p workspace cargo_preset_persistence` (1 passed); `CARGO_INCREMENTAL=0 cargo check -p settings --no-default-features` (passed).

### Epic B2 parent: Route contextual Cargo actions through existing execution systems

This epic adds explicit actions to `cargo_ui`; `CargoWorkspaceStore` remains metadata-only.

- [x] 4. Add contextual Build, Check, Run, Test, Bench, and Debug actions
  - _id: rust-workspace-cargo-context-actions_
  - Add a table-driven eligibility resolver for workspace/package/target node kinds and accessible disabled reasons. Add panel/context-menu/action registrations only under `rust-tools`.
  - For task actions, combine selection plus active preset, resolve an ordinary task context through existing project APIs, and call `Workspace::schedule_task`. For Debug, create `DebugScenario` with the compiled Cargo build template and call `Workspace::start_debug_session`, preserving the Cargo debugger locator and DAP provider.
  - Preserve navigation/refresh menus separately, require explicit invocation, and add no mutation/install/fetch command. Gate execution on trust, remote connection, guest policy, Cargo availability, target applicability, and host capability; never fall back to client-local execution.
  - Add fake scheduler/debugger tests that assert exact command/argv/env/cwd, action availability, save/history/reveal routing, inaccessible states, and no direct process call from `cargo_ui`.
  - _Requirements: 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.3, 7.4, 7.9, 9.3, 9.7_
  - _Depends on: 3_
  - _blocked_by: rust-workspace-cargo-presets_
  - _wave: 4_
  - _Reads: crates/cargo_ui/src/cargo_panel.rs, crates/cargo_ui/src/cargo_preset.rs, crates/cargo_ui/src/cargo_ui.rs, crates/workspace/src/tasks.rs, crates/workspace/src/workspace.rs, crates/project/src/task_store.rs, crates/project/src/task_inventory.rs, crates/project/src/debugger/locators/cargo.rs, crates/tasks_ui/src/tasks_ui.rs, crates/task/src/debug_format.rs_
  - _Writes: crates/cargo_ui/src/cargo_actions.rs, crates/cargo_ui/src/cargo_panel.rs, crates/cargo_ui/src/cargo_ui.rs, crates/cargo_ui/Cargo.toml, Cargo.lock, crates/project/src/debugger/locators/cargo.rs, crates/sim/src/sim.rs_
  - _Validation: `cargo test -p cargo_ui cargo_actions -- --nocapture`; `cargo test -p workspace tasks`; `cargo test -p project cargo_locator`; `cargo test -p sim --features test-support,rust-tools cargo_actions`_
  - Outcome: Valid Cargo selections expose explicit contextual actions whose only execution paths are Sim Tasks or DAP.
  - Design: D2, D5, D10, D11, D15
  - Done when: Every eligibility row and denial state is tested, exact TaskTemplate/DebugScenario snapshots pass, fake runners prove `cargo_ui` never spawns directly, and current navigation/refresh behavior remains available.
  - Evidence: `CARGO_INCREMENTAL=0 cargo test -p cargo_ui cargo_actions -- --nocapture` (3 passed); `CARGO_INCREMENTAL=0 cargo test -p cargo_ui -- --nocapture` (12 passed, including navigation/refresh regressions); `CARGO_INCREMENTAL=0 cargo test -p workspace tasks` (7 passed); `CARGO_INCREMENTAL=0 cargo test -p project cargo_locator` (1 passed); `CARGO_INCREMENTAL=0 cargo test -p sim --features test-support,rust-tools cargo_actions` (1 passed).

## Milestone Now C: Generic structured results

### Epic C1 parent: Add a language-neutral result model and store

The result contract is internal and execution-tree shaped. It contains no Cargo packages, targets, features, commands, or public plugin API.

- [x] 5. Implement the feature-gated structured execution model, protocol, and store
  - _id: rust-workspace-structured-execution-store_
  - Add `project/structured-execution` with provider/discovery/run/node/event identities, generic node kinds and states, optional visible `ProjectPath`, summaries, monotonic sequences, last-complete/current run separation, deterministic retention, truncation, and bounded errors.
  - Add typed paged/chunked protocol messages and handlers patterned after existing project stores. Keep protobuf definitions inert across variants if needed, but conditionally compile store construction/registration. Exclude commands, env, terminal bytes, raw metadata, and absolute paths.
  - Add a fake non-Rust provider and deterministic state-machine/property tests for duplicate events, gaps, late/cross-project generations, partial discovery, retention, path filtering, malformed enums, and limits.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.8, 6.5, 6.6, 6.7, 6.8, 7.4, 7.6, 7.7, 7.8, 8.4, 8.6, 9.1, 9.4, 9.6, 9.7_
  - _Depends on: 4_
  - _blocked_by: rust-workspace-cargo-context-actions_
  - _wave: 5_
  - _Reads: crates/project/src/project.rs, crates/project/src/task_store.rs, crates/project/src/toolchain_store.rs, crates/project/src/cargo_workspace_store.rs, crates/proto/proto/sim.proto, crates/proto/src/proto.rs, crates/remote_server/src/headless_project.rs_
  - _Writes: crates/project/src/structured_execution.rs, crates/project/src/project.rs, crates/project/Cargo.toml, crates/proto/proto/structured_execution.proto, crates/proto/proto/sim.proto, crates/proto/src/proto.rs_
  - _Validation: `cargo test -p project --features test-support,structured-execution structured_execution -- --nocapture`; `cargo test -p proto structured_execution`; `cargo check -p project --no-default-features`; `cargo tree --locked -p project --no-default-features -e normal --prefix none`_
  - Outcome: A bounded, remote-capable, language-neutral result store accepts deterministic fake-provider discovery and run events without any Cargo dependency.
  - Design: D2, D6, D10, D11, D12, D13, D15
  - Done when: State-machine and privacy/retention tests pass, disabled project builds contain no store, protocol remains path-safe/bounded, and source/dependency audits find no Cargo-shaped generic type.
  - Evidence: `CARGO_INCREMENTAL=0 cargo test -p project --features test-support,structured-execution structured_execution -- --nocapture` (4 passed); `CARGO_INCREMENTAL=0 cargo test -p proto structured_execution` (1 passed); `CARGO_INCREMENTAL=0 cargo check -p project --no-default-features` (passed); `cargo tree --locked -p project --no-default-features -e normal --prefix none` (passed; disabled graph contains no Cargo UI or metadata dependency).

### Epic C2 parent: Observe ordinary task lifecycle without parsing terminal output

The existing scheduler/terminal remains authoritative. This epic adds an opt-in handle and keeps the ordinary API compatible.

- [x] 6. Add cancellable structured task lifecycle handles
  - _id: rust-workspace-structured-task-handle_
  - Extend the `TerminalProvider`/workspace scheduling seam with an additive opt-in handle that exposes task/terminal identity, queued/running/completed/spawn-error/cancelled lifecycle, eventual exit status, terminal navigation, and cancellation delegation.
  - Keep `schedule_task` and ordinary terminals/history/rerun source-compatible. Do not expose terminal emulator bytes or add a second process runner. Route lifecycle events into `StructuredExecutionStore` only when an observer is supplied.
  - Cover completion-before-subscribe, cancellation races, terminal close, spawn failure, remote/disconnected state, superseding run, late event rejection, and dropped handle behavior with fake providers and GPUI executor timing.
  - _Requirements: 3.4, 4.3, 4.5, 5.5, 5.8, 5.10, 6.2, 6.7, 7.2, 7.6, 7.8, 7.9, 9.3, 9.4, 9.7_
  - _Depends on: 5_
  - _blocked_by: rust-workspace-structured-execution-store_
  - _wave: 6_
  - _Reads: crates/workspace/src/workspace.rs, crates/workspace/src/tasks.rs, crates/terminal_view/src/terminal_panel.rs, crates/terminal/src/terminal.rs, crates/task/src/task.rs, crates/project/src/terminals.rs, crates/project/src/structured_execution.rs_
  - _Writes: crates/workspace/src/workspace.rs, crates/workspace/src/tasks.rs, crates/terminal_view/src/terminal_panel.rs, crates/terminal/src/terminal.rs, crates/task/src/task.rs, crates/project/src/terminals.rs, crates/project/src/structured_execution.rs_
  - _Validation: `cargo test -p workspace structured_task -- --nocapture`; `cargo test -p terminal structured_task`; `cargo test -p terminal_view structured_task`; `cargo test -p project --features test-support,structured-execution structured_task_bridge`_
  - Outcome: Structured adapters can observe and cancel an ordinary task while the terminal remains the single owner of process output and completion.
  - Design: D5, D6, D7, D10, D12, D15
  - Done when: Race tests report exact lifecycle states, cancellation uses existing terminal behavior, ordinary task regressions pass unchanged, and no API exposes or parses rendered terminal content.
  - Evidence: `CARGO_INCREMENTAL=0 cargo test -p workspace structured_task -- --nocapture` (5 passed, including retained completion, cancellation races, dropped handles, spawn errors, and disconnected-host policy); `CARGO_INCREMENTAL=0 cargo test -p terminal structured_task` (1 passed); `CARGO_INCREMENTAL=0 cargo test -p terminal_view structured_task` (1 passed); `CARGO_INCREMENTAL=0 cargo test -p project --features test-support,structured-execution structured_task_bridge` (1 passed); `CARGO_INCREMENTAL=0 cargo test -p workspace tasks -- --nocapture` (12 passed, including unchanged ordinary scheduling/save behavior).

### Epic C3 parent: Render generic results in a Tests panel

This epic reuses `language_tool_tree`; it does not add a Rust-specific tree host.

- [x] 7. Implement the generic dockable Tests panel
  - _id: rust-workspace-test-explorer-ui_
  - Add `tasks_ui/test-explorer`, selectable by a `tasks_ui` feature, that projects `StructuredExecutionStore` providers/suites/groups/cases through `language_tool_tree` with stable opaque IDs.
  - Implement title `Tests`, docking/persistence, text and status filtering, summary counts, run/cancel/rerun-failed dispatch hooks, terminal links, failure navigation, keyboard/focus/accessibility behavior, and distinct loading/empty/partial/stale/error/restricted/disconnected/mismatch states.
  - Register no Rust/Cargo labels or provider when only the generic feature is compiled. Use a fake non-Rust provider for UI tests and retain result history in memory only.
  - _Requirements: 1.8, 4.2, 4.6, 4.7, 4.8, 7.6, 7.8, 8.4, 9.1, 9.4, 9.5, 9.7_
  - _Depends on: 6_
  - _blocked_by: rust-workspace-structured-task-handle_
  - _wave: 7_
  - _Reads: crates/tasks_ui/src/tasks_ui.rs, crates/tasks_ui/Cargo.toml, crates/language_tools/src/language_tool_tree.rs, crates/cargo_ui/src/cargo_panel.rs, crates/panel/src/panel.rs, crates/workspace/src/dock.rs, crates/project/src/structured_execution.rs_
  - _Writes: crates/tasks_ui/src/test_explorer.rs, crates/tasks_ui/src/tasks_ui.rs, crates/tasks_ui/Cargo.toml, crates/language_tools/src/language_tool_tree.rs, crates/settings_content/src/workspace.rs, crates/settings_content/src/settings_content.rs, crates/settings_content/Cargo.toml, crates/settings/src/vscode_import.rs, crates/settings/Cargo.toml, Cargo.lock_
  - _Validation: `cargo test -p tasks_ui --features test-explorer test_explorer -- --nocapture`; `cargo test -p language_tools language_tool_tree`; `cargo check -p tasks_ui --no-default-features`; `cargo tree --locked -p language_tools -e normal --prefix none`_
  - Outcome: A Cargo-free Tests panel renders and controls bounded structured results from an internal fake provider using existing panel/tree conventions.
  - Design: D1, D2, D6, D8, D12, D13, D15
  - Done when: GPUI tests cover every state and interaction, persistence excludes payloads/secrets, fake non-Rust results navigate/filter correctly, and generic dependency graphs contain no Cargo tooling.
  - Evidence: `CARGO_INCREMENTAL=0 cargo test -p tasks_ui --features test-explorer test_explorer -- --nocapture` (4 passed, including fake non-Rust projection/filtering, every provider state, privacy-safe persistence, and GPUI tree behavior); `CARGO_INCREMENTAL=0 cargo test -p language_tools language_tool_tree` (8 passed); `CARGO_INCREMENTAL=0 cargo check -p tasks_ui --no-default-features` (passed); `cargo tree --locked -p language_tools -e normal --prefix none` (passed; generic tree graph contains no `cargo_ui` or `cargo_metadata`).

## Milestone Now D: Rust test provider

### Epic D1 parent: Prove a stable, bounded discovery protocol

Production provider work is gated on evidence; terminal text scraping is not an acceptable fallback.

- [x] 8. Validate and implement the Rust test-discovery protocol adapter
  - _id: rust-workspace-rust-test-protocol_
  - Add an injectable protocol adapter and deterministic captured fixtures for Cargo JSON harness location plus bounded stable test listing across unit, integration, binary, example-harness, benchmark, ignored, and doctest cases on Sim's supported stable toolchain matrix.
  - Define fallible record parsing, stable identity inputs, source/runnable enrichment, unknown-record partial diagnostics, timeout/output/node limits, and capability reporting. Do not parse ANSI terminal output and do not require or install `cargo-nextest`.
  - Keep the adapter separate from `CargoWorkspaceStore`. If the fixture gate cannot satisfy the design on supported stable toolchains, record the failed evidence, leave production provider/panel registration disabled, and apply OQ2 instead of weakening the protocol.
  - _Requirements: 5.2, 5.3, 5.4, 5.6, 5.10, 5.11, 7.3, 7.7, 7.8, 9.1, 9.2, 9.7_
  - _Depends on: 7_
  - _blocked_by: rust-workspace-test-explorer-ui_
  - _wave: 8_
  - _Reads: crates/project/src/debugger/locators/cargo.rs, crates/project/src/cargo_workspace.rs, crates/project/src/cargo_workspace_store.rs, crates/project/src/lsp_store/rust_analyzer_ext.rs, crates/editor/src/runnables.rs, crates/languages/src/rust.rs, rust-toolchain.toml_
  - _Writes: crates/project/src/rust_test_provider.rs, crates/project/src/project.rs, crates/project/test_data/rust_test_provider, crates/project/Cargo.toml_
  - _Validation: `cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_protocol -- --nocapture`; `cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_protocol_fixture_matrix -- --ignored --nocapture`_
  - Outcome: A fixture-backed adapter can enumerate supported Rust test forms into bounded records, or produces explicit evidence that prevents premature Tests-panel release.
  - Design: D2, D9, D11, D12, D15, D16
  - Done when: The supported-toolchain fixture matrix passes with no nextest/network/machine dependency, unknown records are partial rather than fatal, identities are deterministic, and CargoWorkspaceStore exposes no test/build/run API.
  - Evidence: `CARGO_INCREMENTAL=0 cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_protocol -- --nocapture` (1 passed, 1 fixture gate ignored; deterministic identities, source/runnable enrichment, injected hermetic runner, malformed/future-record partial handling, and byte/line/case/field/diagnostic limits); `CARGO_INCREMENTAL=0 cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_protocol_fixture_matrix -- --ignored --nocapture` (1 passed for the configured stable 1.95.0 fixture matrix covering unit, integration, binary, example-harness, benchmark, ignored, and doctest cases); source/dependency audit found no `cargo-nextest`, ANSI terminal parser, network, or `CargoWorkspaceStore` execution API.

### Epic D2 parent: Discover Rust tests on the authoritative host

This epic adds the Rust-specific provider/store and remote proxy, not another semantic index.

- [x] 9. Implement local, remote, and multiplayer Rust test discovery
  - _id: rust-workspace-rust-test-provider_
  - Complete `project::rust_test_provider` behind `project/rust-tests`, consuming Cargo workspace suites, existing rust-analyzer runnable/source hints, and the validated injected discovery adapter. Publish generic nodes to `StructuredExecutionStore` with stable provider-owned keys and safe `ProjectPath` locations.
  - Add local and remote modes, project/headless ownership, typed request/cancel/capability handlers, peer/project generations, worktree visibility filtering, trust gating, relevant source/manifest invalidation, debounce, kill-on-drop, partial root isolation, and stale retention.
  - Use `ProjectEnvironment` only on the authoritative host; support existing SSH/remote-server and host-backed WSL/dev-container modes without client-local paths. Make unsupported/missing/mismatch states typed and non-retrying.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.8, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 7.1, 7.2, 7.3, 7.5, 7.6, 7.7, 7.8, 9.1, 9.2, 9.4, 9.6, 9.7_
  - _Depends on: 8_
  - _blocked_by: rust-workspace-rust-test-protocol_
  - _wave: 9_
  - _Reads: crates/project/src/project.rs, crates/project/src/cargo_workspace_store.rs, crates/project/src/structured_execution.rs, crates/project/src/rust_test_provider.rs, crates/project/src/environment.rs, crates/project/src/trusted_worktrees.rs, crates/remote_server/src/headless_project.rs, crates/proto/proto/structured_execution.proto_
  - _Writes: crates/project/src/rust_test_provider.rs, crates/project/src/project.rs, crates/project/Cargo.toml, crates/remote_server/src/headless_project.rs, crates/remote_server/Cargo.toml, crates/proto/proto/structured_execution.proto, crates/proto/proto/sim.proto_
  - _Validation: `cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_provider -- --nocapture`; `cargo test -p remote_server --features test-support,rust-tools rust_test_provider`; `cargo check -p remote_server --no-default-features`_
  - Outcome: Rust test discovery is authoritative-host, cancellable, privacy-filtered, partial-failure tolerant, and projected through the generic store in local and remote projects.
  - Design: D2, D6, D9, D10, D11, D12, D13, D15
  - Done when: Injected local/headless tests prove no client runner call, peer visibility and trust are enforced, reconnect/late generations are rejected, unsupported modes stabilize, and no source-indexing implementation is added.
  - Evidence: `CARGO_INCREMENTAL=0 cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_provider -- --nocapture` (3 passed, 1 stable-toolchain fixture gate ignored; stable hierarchy/source enrichment, scoped worktree filtering, partial-root isolation, and stale retention); `CARGO_INCREMENTAL=0 cargo test -p remote_server --features test-support,rust-tools rust_test_provider` (1 passed; rust-tools headless host compiles and exposes the typed Rust-test/structured-result capability); `CARGO_INCREMENTAL=0 cargo check -p remote_server --no-default-features` (passed; Cargo/test stores and handlers remain absent from the disabled headless build); protocol/store review confirms bounded worktree scopes and cancellation state, authoritative-host `ProjectEnvironment` use, remote-only typed request mode without a runner, generation rejection, kill-on-drop commands, and no new semantic index or client-local execution path.

### Epic D3 parent: Run and debug Rust tests through Tasks and DAP

This epic maps provider keys to existing execution infrastructure and structured lifecycle state.

- [x] 10. Integrate Rust test run, debug, cancel, and rerun-failed actions
  - _id: rust-workspace-rust-test-actions_
  - Map current discovery keys to exact existing Cargo/Rust `TaskTemplate` values and supported `DebugScenario` values. Single-case runs derive status from the structured task handle; suite runs publish only aggregate status unless the validated protocol supplies per-case events.
  - Wire Tests-panel run/debug/cancel/rerun-failed actions through existing workspace scheduler/debugger. Add bounded rerun concurrency, removed-test summaries, task-terminal links, failure navigation, and unsupported doctest/harness debug reasons.
  - Preserve terminal output/history/rerun, reject obsolete discovery/run generations, and store only bounded safe messages/durations/status. Add no direct process launch or terminal-content parser.
  - _Requirements: 3.4, 3.5, 4.3, 4.5, 4.6, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 6.2, 6.4, 6.5, 7.2, 7.4, 7.6, 7.8, 7.9, 9.3, 9.4, 9.5, 9.7_
  - _Depends on: 9_
  - _blocked_by: rust-workspace-rust-test-provider_
  - _wave: 10_
  - _Reads: crates/tasks_ui/src/test_explorer.rs, crates/project/src/rust_test_provider.rs, crates/project/src/structured_execution.rs, crates/workspace/src/tasks.rs, crates/workspace/src/workspace.rs, crates/project/src/debugger/locators/cargo.rs, crates/task/src/debug_format.rs, crates/languages/src/rust.rs_
  - _Writes: crates/tasks_ui/src/test_explorer.rs, crates/project/src/rust_test_provider.rs, crates/workspace/src/tasks.rs_
  - _Validation: `cargo test -p tasks_ui --features test-explorer rust_test_actions -- --nocapture`; `cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_actions`; `cargo test -p workspace structured_task`; `cargo test -p project cargo_locator`_
  - Outcome: The Tests panel runs and debugs current Rust test nodes through ordinary Tasks/DAP and reports accurate bounded structured lifecycle state.
  - Design: D5, D6, D7, D8, D9, D10, D11, D12, D15
  - Done when: Exact-case, aggregate-suite, debug eligibility, cancel races, rerun-failed concurrency, removed cases, navigation, and terminal-link tests pass with fake schedulers and no machine tools.
  - Evidence: `CARGO_INCREMENTAL=0 cargo test -p tasks_ui --features test-explorer rust_test_actions -- --nocapture` (1 passed; generic projection exposes stable opaque action selections without a Rust dependency); `CARGO_INCREMENTAL=0 cargo test -p tasks_ui --features rust-test-actions rust_test_actions -- --nocapture` (1 passed and the Rust Tasks/DAP delegate compiled); `CARGO_INCREMENTAL=0 cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_actions -- --nocapture` (2 passed; exact Cargo argv, ignored/group selectors, stable run IDs, task policy, DAP eligibility, doctest denial, and rerun bound); `CARGO_INCREMENTAL=0 cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests structured_task_bridge -- --nocapture` (1 passed; single-case task lifecycle is retained as bounded structured case state while suite runs remain aggregate); `CARGO_INCREMENTAL=0 cargo test -p workspace structured_task` (5 passed; scheduler lifecycle, cancellation races, retained terminal handles, spawn errors, and disconnected-host policy); `CARGO_INCREMENTAL=0 cargo test -p project cargo_locator` (1 passed); `CARGO_INCREMENTAL=0 cargo check -p tasks_ui --features rust-test-actions` (passed without warnings). Source review confirms all execution enters the existing workspace Task scheduler or `Workspace::start_debug_session`, terminal reveal uses the retained structured task handle, stale discovery keys are rejected before scheduling, partial reruns report removed/stale/over-limit selections, and no process launcher or terminal parser was added.

## Milestone Now E: Distribution, compatibility, and quality gates

### Epic E1 parent: Preserve optional builds and project-mode safety

This epic extends the already implemented `rust-tools` validation; it does not move all Rust language support behind the feature.

- [x] 11. Wire and validate rust-tools, remote, trust, privacy, and mismatch boundaries
  - _id: rust-workspace-feature-remote-boundary_
  - Extend `sim/rust-tools` and `remote_server/rust-tools` forwarding for structured execution, test explorer, and Rust test provider. Gate settings, menus, actions, panel/provider/store initialization, request handlers, and runners. Keep `cargo_ui` and Cargo/test-only dependencies optional and absent from disabled normal graphs.
  - Extend capability/version negotiation and mismatch UI for enabled client/disabled host, disabled client/enabled host, disconnect/reconnect, guest execution denial, and visible-worktree filtering. Keep protobuf inert when conditional generation is disproportionate.
  - Extend `script/check-rust-tools-feature-boundary`, release bundle dry-runs, and CI enabled/disabled combinations. Assert that `language_tools` and generic structured modules contain no Cargo types/dependencies and that existing `languages::init`, grammars, rust-analyzer, and private Rust task discovery remain outside the new boundary.
  - Add privacy/trust assertions for zero runner calls in restricted/disabled builds, revocation cancellation, no client-local fallback, no secret/env values or absolute host paths on wire/log/model, bounded payloads, and no automatic network/install behavior.
  - _Requirements: 3.10, 4.8, 5.10, 5.11, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8, 7.9, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 9.6, 9.7_
  - _Depends on: 10_
  - _blocked_by: rust-workspace-rust-test-actions_
  - _wave: 11_
  - _Reads: crates/sim/Cargo.toml, crates/sim/src/main.rs, crates/sim/src/sim.rs, crates/sim/src/sim/app_menus.rs, crates/project/Cargo.toml, crates/remote_server/Cargo.toml, crates/remote_server/src/headless_project.rs, crates/tasks_ui/Cargo.toml, crates/settings/Cargo.toml, crates/settings_content/Cargo.toml, script/check-rust-tools-feature-boundary, script/bundle-mac, script/bundle-linux, script/bundle-freebsd, script/bundle-windows.ps1, .github/workflows/run_tests.yml_
  - _Writes: crates/sim/Cargo.toml, crates/sim/src/main.rs, crates/sim/src/sim.rs, crates/sim/src/sim/app_menus.rs, crates/project/Cargo.toml, crates/remote_server/Cargo.toml, crates/remote_server/src/headless_project.rs, crates/tasks_ui/Cargo.toml, crates/settings/Cargo.toml, crates/settings_content/Cargo.toml, script/check-rust-tools-feature-boundary, script/bundle-mac, script/bundle-linux, script/bundle-freebsd, script/bundle-windows.ps1, .github/workflows/run_tests.yml_
  - _Validation: `./script/check-rust-tools-feature-boundary`; `cargo check -p sim --features rust-tools`; `cargo check -p sim --no-default-features`; `cargo check -p remote_server --features rust-tools`; `cargo check -p remote_server --no-default-features`; `cargo tree --locked -p sim --no-default-features -e normal --prefix none`; `cargo tree --locked -p remote_server --no-default-features -e normal --prefix none`_
  - Outcome: Enabled desktop/headless builds expose the complete Rust workspace, disabled builds contain no Cargo/test workspace execution path or exclusive dependencies, and capability mismatch remains safe.
  - Design: D2, D6, D8, D10, D11, D12, D13, D14, D15
  - Done when: Boundary script, all four builds, graph assertions, bundle plans, mismatch/trust/privacy/multiplayer tests, and CI configuration pass without moving existing Rust language initialization under `rust-tools`.
  - Evidence: `./script/check-rust-tools-feature-boundary` (passed; disabled desktop/headless/project graphs exclude `cargo_ui` and `cargo_metadata`, generic trees remain Cargo-free, and enabled/disabled bundle plans agree); `CARGO_INCREMENTAL=0 cargo check -p sim --features rust-tools` and `CARGO_INCREMENTAL=0 cargo check -p sim --no-default-features` (passed); `CARGO_INCREMENTAL=0 cargo check -p remote_server --features rust-tools` and `CARGO_INCREMENTAL=0 cargo check -p remote_server --no-default-features` (passed); `CARGO_INCREMENTAL=0 cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_feature_remote_boundary -- --nocapture` (1 passed; bounded typed action plans and lifecycle events, secret rejection, mismatch mapping, and trust-restricted action suppression); `CARGO_INCREMENTAL=0 cargo test -p proto structured_execution` (1 passed); focused Tasks UI and Rust action regressions passed after the remote planning seam became asynchronous. The headless registration source audit and enabled check cover all four gated stores/handlers; CI now runs enabled Rust actions/provider tests plus disabled desktop/headless tests. Inert protobuf definitions remain compiled by design, while no Cargo discovery or execution store is initialized without `rust-tools`.

### Epic E2 parent: Prove end-to-end behavior, scale, and documentation

This epic closes test gaps and updates user/developer documentation; it does not add Next/Later functionality.

- [x] 12. Complete hermetic integration, GPUI, scale, migration, and documentation validation
  - _id: rust-workspace-quality-gate_
  - Add an end-to-end fake workspace suite covering verified Cargo baseline plus profiles/toolchain/targets, preset precedence/compilation, contextual Tasks/DAP, structured event semantics, Rust discovery, local/headless/multiplayer routing, trust changes, partial/stale/error/mismatch states, panel persistence, and enabled/disabled variants.
  - Add synthetic fixtures with at least 1,000 packages, 10,000 tests, and 10,000 rendered/projection rows. Assert deterministic stable IDs, retention/truncation limits, background parsing, visible-range rendering, bounded foreground updates, cancellation, and GPUI executor-timer debounce behavior.
  - Run tests with injected runners and unavailable Cargo/rustc/rustup/nextest/network. Add schema/persistence migration fixtures and verify no repository workspace mutation.
  - Update Rust/Cargo, Tasks, debugging, settings, remote-development, and feature-boundary documentation for the shipped Now behavior and explicit Next/Later/External/Rejected/out-of-scope limits. Do not document gated Tests behavior as shipped if Task 8 invoked OQ2.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 5.11, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8, 7.9, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8_
  - _Depends on: 11_
  - _blocked_by: rust-workspace-feature-remote-boundary_
  - _wave: 12_
  - _Reads: crates/cargo_ui, crates/language_tools/src/language_tool_tree.rs, crates/project/src/cargo_workspace.rs, crates/project/src/structured_execution.rs, crates/project/src/rust_test_provider.rs, crates/tasks_ui/src/test_explorer.rs, crates/remote_server/src/headless_project.rs, crates/sim/src/sim.rs, docs/src/languages/rust.md, docs/src/tasks.md, docs/src/debugger.md, docs/src/remote-development.md, script/check-rust-tools-feature-boundary_
  - _Writes: crates/cargo_ui/src/cargo_panel.rs, crates/language_tools/src/language_tool_tree.rs, crates/project/test_data/cargo_workspace, crates/project/test_data/rust_test_provider, crates/project/src/structured_execution.rs, crates/project/src/rust_test_provider.rs, crates/tasks_ui/src/test_explorer.rs, crates/remote_server/src/headless_project.rs, crates/sim/src/sim.rs, docs/src/languages/rust.md, docs/src/tasks.md, docs/src/debugger.md, docs/src/remote-development.md_
  - _Validation: `cargo test -p cargo_ui -- --nocapture`; `cargo test -p language_tools language_tool_tree`; `cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_workspace -- --nocapture`; `cargo test -p tasks_ui --features test-explorer test_explorer -- --nocapture`; `cargo test -p remote_server --features test-support,rust-tools rust_workspace`; `cargo test -p sim --features test-support,rust-tools rust_workspace`; `cargo test -p sim --no-default-features rust_workspace_disabled`; `./script/check-rust-tools-feature-boundary`; `./script/clippy -p cargo_ui -p language_tools -p project -p tasks_ui -p workspace -p terminal -p remote_server -p sim`; `python3 .agents/skills/feature-spec/scripts/validate_spec.py .agents/specs/rust-workspace`; `python3 .agents/skills/coding/scripts/validate_spec.py --require-complete .agents/specs/rust-workspace`_
  - Outcome: The complete Now milestone is hermetically validated at scale, documented accurately, and preserves every accepted Cargo baseline and distribution boundary.
  - Design: D1, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12, D13, D14, D15, D16
  - Done when: All exact commands pass; synthetic scale and foreground-thread assertions meet their bounds; migrations restore safely; source folders remain unchanged; and documentation clearly labels shipped, gated, Next, Later, External, Rejected, and out-of-scope behavior.
  - Evidence: `CARGO_INCREMENTAL=0 cargo test -p cargo_ui -- --nocapture` (13 passed, including a deterministic 1,000-package projection); `CARGO_INCREMENTAL=0 cargo test -p language_tools language_tool_tree` (8 passed, including deterministic 10,000-row visible-range projection); `CARGO_INCREMENTAL=0 cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_workspace -- --nocapture` (2 passed, covering bounded 10,000-node structured results and 10,000 hermetic Rust tests); `CARGO_INCREMENTAL=0 cargo test -p tasks_ui --features test-explorer test_explorer -- --nocapture` (6 passed, including bounded privacy-safe state migration); `CARGO_INCREMENTAL=0 cargo test -p remote_server --features test-support,rust-tools rust_workspace` (1 passed, authoritative headless routing); `CARGO_INCREMENTAL=0 cargo test -p sim --features test-support,rust-tools rust_workspace` and `CARGO_INCREMENTAL=0 cargo test -p sim --no-default-features rust_workspace_disabled` (1 passed each, enabled registration and disabled absence); `CARGO_INCREMENTAL=0 ./script/clippy -p cargo_ui -p language_tools -p project -p tasks_ui -p workspace -p terminal -p remote_server -p sim` (passed for release all-targets/all-features with warnings denied); `cargo fmt --all -- --check`, `git diff --check`, and `./script/check-rust-tools-feature-boundary` (passed); both feature-spec and coding `--require-complete` validators passed for all 79 criteria and 12 completed tasks (the feature-spec validator emitted only its expected repeated-write sequencing review warnings). Documentation now states shipped Cargo/Tests behavior, Tasks/DAP delegation, authoritative remote execution, feature mismatch, and Now/Next/Later/External/Rejected/out-of-scope limits. All tests use injected or synthetic data rather than the developer Cargo installation or network, and the source specification folders remain unchanged.

## Mandatory manual task-decomposition audit

Performed after drafting the requirements, design, and task graph:

1. **Baseline duplication:** No leaf recreates the Cargo model/store/panel/tree host. Tasks 1-2 and 12 alter or regression-test precise extension seams only.
2. **Epic size:** Each parent epic is split into executable leaves. Configuration model and UI, preset schema and actions, result store and task handle and UI, test protocol and provider and actions, and boundary and quality work are independently reviewable.
3. **Cross-layer separation:** No leaf owns both Cargo metadata discovery and user task execution. Test discovery is separate from CargoWorkspaceStore; execution remains Tasks/DAP.
4. **Test placement:** Unit/fixture tests land with their owner; remote and GPUI integration land with the relevant leaf; Task 12 supplies only cross-component/scale closure rather than postponing all testing.
5. **Dependency graph:** Every `_blocked_by` points to an earlier wave. Sequential waves are intentional because `project`, `workspace`, feature manifests, and panel registration are integration hot spots; no same-wave write/read conflict exists.
6. **Concrete scope:** Every leaf names repository-relative Reads, Writes, exact validation commands, observable outcome, and completion evidence. No broad capability epic is presented as one executable task.
7. **Traceability:** Every acceptance criterion appears in the design Traceability table and at least one executable leaf; Task 12 deliberately re-covers the full set as the end-to-end release gate.
8. **Roadmap containment:** No task implements call hierarchy, coverage, profiling, auditing, dependency mutation/provenance, public plugins, Java/C# providers, nextest installation, or full Rust-language optionality.
