# Implementation Plan: Rust Workspace Tool Window

## Approach

Implement the typed Cargo model behind the narrow `project/cargo-workspace` feature and the always-available generic tree host first because they define both the runtime and compile-time boundaries. Keep protobuf messages inert across variants, wire the local/remote store only in feature-enabled project hosts, and make `cargo_ui` an optional dependency selected by `sim/rust-tools`. Register settings, actions, menus, and context bindings only from that optional UI. Finish with deterministic model, GPUI, remote/privacy, enabled/disabled dependency checks, release feature parity, and repository validation.

The plan uses existing `language_tools` for the Cargo-agnostic host, `cargo_ui` for the optional user-facing provider/panel, `project::cargo_workspace` for the model, and conditionally compiled `project::CargoWorkspaceStore` for authoritative host-side execution. `rust-tools` is the distribution capability; no `metal_cargo`, broad `metal_*` paths, umbrella crate, or public provider registry is added.

## Dependency waves

- Wave 1: Tasks 1, 2, and 5 establish the feature-gated typed Cargo model, inert typed protocol, and always-available generic tree host. They are parallel-safe; Task 5 extends the existing `language_tools` crate rather than creating a duplicate crate.
- Wave 2: Tasks 3 and 4 add project-host local/remote behavior; Task 6 adds the Cargo projection and panel after the model/store/host contracts exist.
- Wave 3: Tasks 7 and 8 register feature-local settings/context bindings, menus, panel loading, and application dependencies.
- Wave 4: Tasks 9, 10, and 11 add model/store, generic-host, and Cargo-panel coverage in parallel; Task 12 follows Task 9 because both register modules in the project integration-test harness.
- Wave 5: Task 13 adds the dependency/release/CI feature-boundary checks after integration paths exist.
- Wave 6: Task 14 runs the complete enabled/disabled validation and resolves cross-crate issues.

## Tasks

- [x] 1. Add the typed Cargo workspace model and metadata conversion
  - _id: rust-workspace-tool-window-model_
  - Add UI-independent workspace, package, target, feature, direct-dependency, candidate-status, source-kind, and navigation-path types in `project::cargo_workspace` behind a new `cargo-workspace` project feature.
  - Parse `cargo_metadata::Metadata` format version 1 fallibly, treat unknown enum-like values safely, match resolve nodes for enabled features/resolved packages, and omit unsupported workspace-inheritance claims.
  - Convert absolute metadata paths only through validated visible-worktree containment and emit stable relative structural keys without serializing absolute paths.
  - Preserve the private task-target metadata path in `crates/languages/src/rust.rs`; make the workspace `cargo_metadata` dependency optional in `project` and selected only by `cargo-workspace`, without coupling to task-private types.
  - _Requirements: 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 5.3, 7.2, 7.5, 7.6, 8.1, 8.2, 8.6, 9.3_
  - _Depends on: none_
  - _Reads: Cargo.toml, crates/project/src/project.rs, crates/project/src/worktree_store.rs, crates/languages/src/rust.rs, crates/project/Cargo.toml_
  - _Writes: crates/project/src/cargo?workspace.rs, crates/project/src/project.rs (module export), crates/project/Cargo.toml, Cargo.lock (project package dependency update)_
  - _Validation: Run `cargo check -p project --features cargo-workspace`, `cargo check -p project --no-default-features`, and `cargo test -p languages test_target_info_from_metadata`; confirm the new model is absent from the disabled API and existing task-target behavior remains unchanged._
  - Outcome: `project::cargo_workspace` exposes a feature-gated, UI-independent, path-safe Cargo model converted fallibly from format-version-1 metadata.
  - Design: D1, D6, D7, D16
  - Done when: Enabled and disabled project checks pass, model fixtures convert without panics or absolute-path leakage, and the existing Rust task-target regression passes.
  - _Evidence: `cargo test -p project --features test-support,cargo-workspace cargo_workspace -- --nocapture` passed 12 focused tests; `cargo check -p project --no-default-features` and `cargo test -p languages test_target_info_from_metadata` passed._

- [x] 2. Define the typed Cargo workspace project protocol
  - _id: rust-workspace-tool-window-protocol_
  - Add request, cancellation, response, workspace/member/target/feature/dependency/status, source-kind, and `ProjectPath`-based navigation messages.
  - Register message priorities, request/response associations, and the `sim.proto` envelope fields using unused stable field numbers and existing proto conventions.
  - Keep request IDs peer-scoped and ensure the wire schema contains no environment fields, raw Cargo output, or absolute host-path fields; intentionally keep these inert definitions compiled in both build variants.
  - _Requirements: 4.5, 5.2, 5.3, 6.2, 6.5, 7.2, 8.4, 8.6, 9.9_
  - _Depends on: none_
  - _Reads: crates/proto/build.rs, crates/proto/proto/sim.proto, crates/proto/proto/toolchain.proto, crates/proto/proto/task.proto, crates/proto/src/proto.rs, crates/project/src/toolchain_store.rs, crates/project/src/task_store.rs_
  - _Writes: crates/proto/proto/cargo.proto, crates/proto/proto/sim.proto, crates/proto/src/proto.rs_
  - _Validation: Run `cargo test -p proto` and a proto round-trip test that rejects/omits absolute-path and unbounded-diagnostic fields._
  - Outcome: The shared protocol carries bounded Cargo workspace snapshots and peer-scoped refresh/cancellation messages without host-only data.
  - Design: D6, D10, D11, D16
  - Done when: Protocol generation and round-trip tests pass in the always-compiled proto configuration.
  - _Evidence: `cargo test -p proto` passed three tests, including `cargo_workspace_response_round_trips_project_paths_only`._

- [x] 3. Implement Cargo manifest discovery and the local/remote Cargo workspace store
  - _id: rust-workspace-tool-window-store_
  - _blocked_by: rust-workspace-tool-window-model, rust-workspace-tool-window-protocol_
  - Add local and remote store modes patterned after `TaskStore` and `ToolchainStore`; guard the module, project field/accessor, construction, sharing, subscriptions, and request handlers with `project/cargo-workspace` so disabled project builds contain no store path.
  - Discover sorted visible `Cargo.toml` candidates from `WorktreeStore`, wait for initial scans, skip single-file/private candidates, run shallowest uncovered candidates, deduplicate successful workspace roots/members, and keep candidate-scoped failures.
  - Add an injectable `CargoMetadataRunner`; production shall use `util::command::new_command("cargo")`, the owning worktree's `ProjectEnvironment`, explicit format version and manifest path, captured output, and `kill_on_drop(true)`.
  - Gate each host command through `TrustedWorktrees`, sanitize/bound failures, filter trust/connectivity/relevant-worktree events into Cargo-input invalidations, and attach input fingerprints to snapshots so command-induced lockfile events can be suppressed after a covering result.
  - Own local process and peer-scoped remote-request lifetimes, cancellation, and stale process-result rejection; leave dormant/dirty state, debounce, UI generations, partial/stale presentation, and tree reconciliation to the generic host/provider layer.
  - Implement typed remote snapshot and peer-scoped cancellation request handling; one peer must not cancel another peer's generation.
  - _Requirements: 1.3, 1.4, 1.6, 1.7, 2.9, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.8, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 7.2, 7.6, 8.4, 8.6, 9.3, 9.4_
  - _Depends on: 1, 2_
  - _Reads: crates/project/src/project.rs, crates/project/src/task_store.rs, crates/project/src/toolchain_store.rs, crates/project/src/environment.rs, crates/project/src/trusted_worktrees.rs, crates/project/src/worktree_store.rs, crates/worktree/src/worktree.rs, crates/util/src/command.rs_
  - _Writes: crates/project/src/cargo?workspace?store.rs, crates/project/src/project.rs (store ownership and lifecycle)_
  - _Validation: Run `cargo check -p project --features cargo-workspace,test-support` and `cargo check -p project --no-default-features`; verify a disabled source/API compile probe cannot construct or import `CargoWorkspaceStore` before Task 9 adds runtime coverage._
  - Outcome: Feature-enabled projects own a cancellable, trusted, host-authoritative Cargo workspace store while disabled projects contain no store path or Cargo metadata dependency.
  - Design: D3, D4, D5, D9, D10, D11, D16
  - Done when: Both project configurations compile and focused fake-runner checks prove discovery, trust, environment, cancellation, and bounded-error behavior.
  - _Evidence: The focused project suite passed eight store tests and four fixture/integration tests with injected runners; the enabled and disabled project checks passed, and the feature-boundary script proved the disabled graph omits Cargo workspace APIs and dependencies._

- [x] 4. Attach the Cargo workspace store to the headless remote project
  - _id: rust-workspace-tool-window-headless_
  - _blocked_by: rust-workspace-tool-window-store_
  - Add `remote_server/rust-tools = ["project/cargo-workspace"]`; construct local Cargo store mode in `HeadlessProject` with its authoritative worktree store and project environment only under that feature.
  - Under the same feature, retain the entity, subscribe it under `REMOTE_SERVER_PROJECT_ID`, and register typed snapshot/cancel handlers on the remote session; compile none of those paths when disabled.
  - Ensure SSH/WSL requests execute through the headless host runner and use existing remote trust synchronization rather than desktop-local paths.
  - _Requirements: 1.7, 4.5, 6.1, 6.2, 6.3, 6.4, 6.6, 8.4, 8.6, 9.4, 9.6, 9.7, 9.9_
  - _Depends on: 3_
  - _Reads: crates/remote_server/src/headless_project.rs, crates/project/src/cargo_workspace_store.rs, crates/project/src/environment.rs, crates/project/src/toolchain_store.rs_
  - _Writes: crates/remote?server/src/headless?project.rs, crates/remote?server/Cargo.toml_
  - _Validation: Run `cargo check -p remote_server --features rust-tools` and `cargo check -p remote_server --no-default-features`; verify only the enabled build contains the headless Cargo field/handler cfg blocks._
  - Outcome: Rust-tools-capable remote servers execute and serve Cargo metadata on the authoritative host, while disabled servers register no Cargo store or handler.
  - Design: D3, D11, D16
  - Done when: Enabled and disabled remote-server checks pass and handler ownership is confined to the enabled headless host.
  - _Evidence: `cargo test -p remote_server --features test-support,rust-tools cargo_workspace` and `cargo test -p remote_server --no-default-features cargo_workspace_disabled` passed; enabled/disabled remote-server checks also passed._

- [x] 5. Implement the Cargo-agnostic language-tool tree host
  - _id: rust-workspace-tool-window-tree-host_
  - Extend the existing `language_tools` crate with an internal `language_tool_tree` module and provider contract carrying opaque IDs, presentation metadata, hierarchy, provider statuses, activation capability, invalidation, and refresh tasks.
  - Implement focus, selection, expanded-ID state, parent/child indexes, deterministic visible-row flattening, dormant/dirty state, GPUI-timer debounce, refresh generations, loading/refreshing/stale/error/empty states, selection fallback, and in-session state preservation.
  - Render arbitrary-depth accessible rows using `Role::Tree`, `Role::TreeItem`, `ListItem`, `Disclosure`, icons, tooltips, context menus, indent guides, and `uniform_list`; do not modify `TreeViewItem` solely for this feature.
  - Implement generic previous/next/first/last/parent/child, activate, expand/collapse, Expand All, Collapse All, and Refresh actions while leaving node meaning and activation to the provider.
  - Keep the API crate-internal to Sim and free of Cargo domain types, features, dependencies, dynamic registries, and plugin protocols; do not make `rust-tools` a `language_tools` feature.
  - _Requirements: 3.1, 3.2, 3.3, 3.9, 4.1, 4.3, 4.4, 4.5, 4.6, 4.7, 6.7, 7.1, 7.3, 7.4, 7.5, 8.3, 8.5, 9.5_
  - _Depends on: none_
  - _Reads: crates/language_tools/Cargo.toml, crates/language_tools/src/language_tools.rs, crates/language_tools/src/highlights_tree_view.rs, crates/workspace/src/dock.rs, crates/project_panel/src/project_panel.rs, crates/outline_panel/src/outline_panel.rs, crates/ui/src/components/tree_view_item.rs, crates/ui/src/components/list/list_item.rs, crates/ui/src/components/disclosure.rs, crates/panel/src/panel.rs_
  - _Writes: crates/language?tools/src/language?tool?tree.rs, crates/language?tools/src/language?tools.rs (module export)_
  - _Validation: Run `cargo check -p language_tools` and `cargo tree --locked -p language_tools -e normal --prefix none`; verify the tree contains neither `cargo_ui` nor `cargo_metadata` and compile the non-Cargo fake-provider seam for Task 10._
  - Outcome: Existing `language_tools` provides a reusable virtualized tree host with generic state, refresh, interaction, and accessibility behavior and no Cargo dependency.
  - Design: D1, D2, D9, D14, D16
  - Done when: The crate compiles, its dependency tree is Cargo-free, and the fake non-Cargo provider seam exercises arbitrary-depth state.
  - _Evidence: `cargo test -p language_tools` passed 13 tests, and `./script/check-rust-tools-feature-boundary` verified the host dependency graph contains neither `cargo_ui` nor `cargo_metadata`._

- [x] 6. Implement the Cargo tree provider and user-facing Cargo panel
  - _id: rust-workspace-tool-window-panel_
  - _blocked_by: rust-workspace-tool-window-model, rust-workspace-tool-window-store, rust-workspace-tool-window-tree-host_
  - Create `cargo_ui` with a normal dependency on `project` that enables `project/cargo-workspace`, and implement `CargoTreeProvider` over `CargoWorkspaceStore`, including deterministic workspace/member/section/target/feature/dependency projection, opaque stable node IDs, duplicate-name path context, and default-resolution labels.
  - Implement direct-only dependency groups and every in-scope target/feature/dependency annotation, including safe unknown states and partial/stale candidate rows.
  - Implement `CargoPanel` as a thin `Panel` wrapper around the generic host with title `Cargo`, lazy first activation, Cargo icon, toolbar, focus, side-dock support, read-only context menus, and error/empty/restricted/disconnected states.
  - Implement provider-owned package/target/feature/dependency activation through `ProjectPath`; disable unsafe external/private/outside-worktree navigation and surface the reason.
  - Add the `CargoPanelSettings` runtime type for button, width, dock, and starts-open behavior using Cargo-specific names, but leave its feature-gated shared content/default installation and explicit registration to Task 7.
  - _Requirements: 1.1, 1.2, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 3.1, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 4.1, 4.4, 4.6, 4.7, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 6.3, 6.6, 6.7, 7.1, 7.2, 7.3, 7.4, 7.5, 9.1, 9.5_
  - _Depends on: 1, 3, 5_
  - _Reads: crates/language_tools/src/language_tool_tree.rs, crates/project/src/cargo_workspace.rs, crates/project/src/cargo_workspace_store.rs, crates/project_panel/src/project_panel_settings.rs, crates/outline_panel/src/outline_panel.rs, crates/icons/src/icons.rs_
  - _Writes: Cargo.toml (Cargo UI workspace entry), Cargo.lock (Cargo UI package), crates/cargo?ui/Cargo.toml, crates/cargo?ui/src/cargo?ui.rs, crates/cargo?ui/src/cargo?panel.rs, crates/cargo?ui/src/cargo?panel?settings.rs_
  - _Validation: Run `cargo check -p cargo_ui`; inspect a debug projection from a typed fake snapshot before Task 11 adds the focused assertions._
  - Outcome: Optional `cargo_ui` projects direct Cargo workspace data into a dockable, read-only panel titled `Cargo` with safe navigation and actionable states.
  - Design: D7, D8, D10, D12, D13, D16
  - Done when: `cargo_ui` compiles and a typed fake snapshot produces the finite stable hierarchy without transitive dependency children or unsafe navigation.
  - _Evidence: `cargo test -p cargo_ui` passed projection, direct-only dependency, stale refresh, dormant lifecycle, and settings tests; targeted `./script/clippy -p language_tools -p cargo_ui -p project` passed._

- [x] 7. Register Cargo panel settings and contextual key bindings
  - _id: rust-workspace-tool-window-settings_
  - _blocked_by: rust-workspace-tool-window-panel_
  - Add forwarding `rust-tools` features to `settings_content` and `settings`; under that feature only, add the data-only `CargoPanelSettingsContent` and `SettingsContent::cargo_panel` field without introducing a `cargo_ui` or Cargo-model dependency.
  - In `cargo_ui::init`, install typed defaults (`button: true`, `default_width: 280`, `dock: right`, `starts_open: false`) through `SettingsStore::update_default_settings` before explicitly registering the Task 6 `CargoPanelSettings`; do not use unconditional inventory registration and do not add Cargo defaults to the shared `assets/settings/default.json`.
  - Install Cargo-panel context bindings programmatically from `cargo_ui::init` for traversal, activation, expand/collapse, expand/collapse all, and refresh using established conventions; do not place references to optional Cargo actions in always-loaded default keymap assets and do not claim a new global shortcut.
  - Verify enabled settings merging/hide-button behavior and verify the disabled Sim build registers no Cargo settings inventory type or context bindings.
  - _Requirements: 1.2, 3.1, 3.2, 3.9, 7.5, 8.3, 9.1, 9.2_
  - _Depends on: 6_
  - _Reads: crates/settings_content/Cargo.toml, crates/settings_content/src/settings_content.rs, crates/settings_content/src/workspace.rs, crates/settings/Cargo.toml, crates/settings/src/settings_store.rs, crates/settings/src/vscode_import.rs, assets/settings/default.json, crates/project_panel/src/project_panel_settings.rs, crates/cargo_ui/src/cargo_panel_settings.rs_
  - _Writes: crates/settings?content/Cargo.toml, crates/settings?content/src/settings?content.rs, crates/settings?content/src/workspace.rs, crates/settings/Cargo.toml, crates/settings/src/vscode?import.rs (feature-gated struct fields), crates/cargo?ui/Cargo.toml (settings feature forwarding), crates/cargo?ui/src/cargo?ui.rs (defaults, explicit registration, and context bindings)_
  - _Validation: Run `cargo test -p settings_content --features rust-tools`, `cargo test -p settings --features rust-tools`, their `--no-default-features` counterparts, and `cargo test -p cargo_ui cargo_panel_settings`; verify `assets/settings/default.json` remains Cargo-free and Task 12's disabled build observes no Cargo setting or binding._
  - Outcome: Cargo settings and context bindings exist only in rust-tools builds, with typed defaults installed before explicit setting registration.
  - Design: D13, D16
  - Done when: Enabled/disabled settings tests pass, the shared default asset remains Cargo-free, and disabled Sim registers no Cargo setting or binding.
  - _Evidence: Enabled and `--no-default-features` tests for `settings_content` and `settings` passed, `cargo test -p cargo_ui cargo_panel_settings` passed, and the boundary script confirmed shared settings/default assets remain Cargo-free when disabled._

- [x] 8. Integrate the Cargo panel into Sim startup, menus, and standard panel loading
  - _id: rust-workspace-tool-window-sim-integration_
  - _blocked_by: rust-workspace-tool-window-headless, rust-workspace-tool-window-panel, rust-workspace-tool-window-settings_
  - Add `sim/rust-tools = ["dep:cargo_ui"]`, declare `cargo_ui` optional, and guard every import, initialization call, panel loader, visual-test hook, and test reference with `#[cfg(feature = "rust-tools")]`.
  - Load `CargoPanel` through `initialize_panels` and add the View-menu action only in enabled builds, without starting metadata until activation; disabled builds expose neither action nor menu entry.
  - Ensure enabled local, remote-server, shared, and collaboration workspaces obtain the feature-present project store through the established project lifecycle; do not add a fallback store or runner to disabled Sim code.
  - _Requirements: 1.1, 1.2, 1.3, 1.7, 4.1, 6.1, 6.2, 6.6, 7.5, 9.1, 9.2, 9.4, 9.8_
  - _Depends on: 4, 6, 7_
  - _Reads: crates/sim/src/main.rs, crates/sim/src/sim.rs, crates/sim/src/sim/app_menus.rs, crates/sim/src/sim/visual_tests.rs, crates/sim/src/visual_test_runner.rs, crates/sim/Cargo.toml, Cargo.toml_
  - _Writes: crates/sim/src/main.rs, crates/sim/src/sim.rs (gated Cargo panel loading), crates/sim/src/sim/app?menus.rs, crates/sim/src/sim/visual?tests.rs (gated Cargo initialization), crates/sim/src/visual?test?runner.rs, crates/sim/Cargo.toml_
  - _Validation: Run `cargo check -p sim --features rust-tools`, `cargo check -p sim --no-default-features`, and inspect action/menu inventories in both configurations; manually exercise the enabled visual-test seam to confirm panel construction remains dormant before Task 12 adds integration assertions._
  - Outcome: Sim exposes the Cargo panel, action, menu, and initialization only when optional `cargo_ui` is selected by `rust-tools`.
  - Design: D3, D13, D16
  - Done when: Both Sim build variants compile, enabled registration is present and lazy, and disabled inventories contain no Cargo UI surface.
  - _Evidence: `CARGO_INCREMENTAL=0 cargo test -p sim --features test-support,rust-tools cargo_panel` and `CARGO_INCREMENTAL=0 cargo test -p sim --no-default-features cargo_panel_disabled` passed; the dormant-panel test observed no metadata request._

- [x] 9. Add deterministic Cargo model and store regression coverage
  - _id: rust-workspace-tool-window-model-tests_
  - _blocked_by: rust-workspace-tool-window-model, rust-workspace-tool-window-store_
  - Add one comprehensive virtual-workspace format-version-1 fixture and one standalone-package fixture; generate duplicate-name, malformed, unsupported, multi-root, partial-failure, and 10,000-row variants in tests.
  - Assert every target kind, feature tri-state, direct dependency kind/annotation, deterministic ordering/identity, workspace deduplication, and safe relative navigation conversion.
  - Exercise fake-runner command arguments, project environment, trust transitions, relevant-file invalidation filtering, input fingerprints, lockfile follow-up suppression, process cancellation, late process results, bounded UTF-8 errors, and no-network/no-host-Cargo guarantees; host debounce and UI-generation supersession remain Task 10 coverage.
  - _Requirements: 1.3, 1.4, 1.5, 1.6, 1.7, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 6.1, 6.3, 6.4, 7.6, 8.1, 8.2, 8.4, 8.5, 8.6, 9.3_
  - _Depends on: 1, 3_
  - _Reads: crates/project/src/cargo_workspace.rs, crates/project/src/cargo_workspace_store.rs, crates/project/tests/integration/project_tests.rs, crates/project/test_data/_
  - _Writes: crates/project/test?data/cargo?workspace/workspace-v1.json, crates/project/test?data/cargo?workspace/standalone-v1.json, crates/project/tests/integration/cargo?workspace.rs, crates/project/tests/integration/project?tests.rs (model/store module registration)_
  - _Validation: Run `cargo test -p project --features test-support,cargo-workspace cargo_workspace -- --nocapture` and `cargo check -p project --no-default-features`; verify enabled tests use only the fake runner and disabled compilation exposes no Cargo store API._
  - Outcome: Deterministic fixtures and fake runners cover Cargo model semantics, multi-root discovery, partial failures, privacy, and store concurrency without real Cargo or network access.
  - Design: D4, D5, D6, D7, D8, D9, D10, D11, D15, D16
  - Done when: The focused enabled project suite and disabled compile check pass with every in-scope target/dependency form and runner edge case asserted.
  - _Evidence: `cargo test -p project --features test-support,cargo-workspace cargo_workspace -- --nocapture` passed deterministic virtual-workspace, standalone, malformed/unsupported, multi-root, partial-failure, privacy, environment, peer-cancellation, and bounded-error coverage without invoking host Cargo._

- [x] 10. Add generic tree-host GPUI and large-model coverage
  - _id: rust-workspace-tool-window-tree-tests_
  - _blocked_by: rust-workspace-tool-window-tree-host_
  - Test fake-provider loading/current/refreshing/stale/error/empty/disconnected transitions, generation rejection, debounce/manual supersession, state preservation, ancestor/nearest-row fallback, and participant-local state.
  - Test pointer selection versus disclosure/activation, all keyboard traversal/actions, toolbar disabled states/tooltips, read-only context actions, arbitrary ARIA levels/states, and visible-range rendering.
  - Generate at least 10,000 rows and assert stable deterministic flattening plus a rendered-element count bounded by the requested `uniform_list` range, using GPUI executor timers for every timed transition.
  - Assert the crate and fake provider compile with no Rust-tools feature and no Cargo-specific imports or node concepts.
  - _Requirements: 3.1, 3.2, 3.3, 3.9, 4.1, 4.3, 4.4, 4.5, 4.6, 4.7, 6.7, 7.1, 7.3, 7.4, 8.3, 8.5, 8.6, 9.5_
  - _Depends on: 5_
  - _Reads: crates/language_tools/src/language_tool_tree.rs, crates/ui/src/components/list/list_item.rs, crates/outline_panel/src/outline_panel.rs_
  - _Writes: crates/language?tools/src/language?tool?tree.rs (inline generic-host tests)_
  - _Validation: Run `cargo test -p language_tools`; reproduce timed failures with reported GPUI scheduler seeds rather than adding `smol::Timer` waits._
  - Outcome: GPUI tests prove generic interaction, accessibility, refresh reconciliation, debounce, and bounded 10,000-row rendering independently of Cargo.
  - Design: D2, D9, D14, D15, D16
  - Done when: `cargo test -p language_tools` passes using GPUI executor timers and the large-model assertions remain deterministic.
  - _Evidence: `cargo test -p language_tools` passed the generic host's arbitrary-depth navigation, reconciliation, stale generation, GPUI-timer refresh supersession, action availability, deterministic 10,000-row tests, and a render-level assertion that populated rows receive a non-zero viewport._

- [x] 11. Add Cargo panel GPUI, navigation, settings, and persistence coverage
  - _id: rust-workspace-tool-window-panel-tests_
  - _blocked_by: rust-workspace-tool-window-panel, rust-workspace-tool-window-settings, rust-workspace-tool-window-sim-integration_
  - Feed fake typed snapshots through `CargoTreeProvider` and verify exact hierarchy, default-resolution feature labels, direct-only dependency leaves, duplicate-name labels, partial/stale/error/empty/restricted/disconnected presentation, and stable IDs.
  - Verify toolbar, keymap actions, context menus, accessibility state, package/target/feature/dependency navigation, disabled unsafe navigation, selection/expansion preservation, settings defaults, hide-button behavior, side-dock movement, starts-closed behavior, and dock layout restoration.
  - Assert no context or toolbar action mutates manifests or invokes build/test/run commands.
  - _Requirements: 1.1, 1.2, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 4.4, 4.6, 4.7, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 6.3, 6.6, 6.7, 7.2, 7.5, 8.3, 8.6, 9.1_
  - _Depends on: 6, 7, 8_
  - _Reads: crates/cargo_ui/src/cargo_panel.rs, crates/cargo_ui/src/cargo_panel_settings.rs, crates/language_tools/src/language_tool_tree.rs, crates/workspace/src/dock.rs, assets/settings/default.json_
  - _Writes: crates/cargo?ui/src/cargo?panel.rs (inline projection/lifecycle tests), crates/cargo?ui/src/cargo?panel?settings.rs (inline settings tests)_
  - _Validation: Run `cargo test -p cargo_ui`; run `cargo test -p workspace test_panel_size_state_persistence` twice against its temporary workspace database._
  - Outcome: Cargo-panel tests cover projection, read-only navigation, toolbar/context interactions, settings, accessibility, and dock persistence.
  - Design: D7, D8, D9, D10, D12, D13, D15
  - Done when: The Cargo UI suite and repeated persistence test pass with no manifest mutation or build/run/test action.
  - _Evidence: `cargo test -p cargo_ui` passed four panel/settings tests, and `cargo test -p workspace test_panel_size_state_persistence` passed twice against isolated temporary databases._

- [x] 12. Add remote-server, multiplayer privacy, and application integration coverage
  - _id: rust-workspace-tool-window-remote-tests_
  - _blocked_by: rust-workspace-tool-window-protocol, rust-workspace-tool-window-store, rust-workspace-tool-window-headless, rust-workspace-tool-window-sim-integration, rust-workspace-tool-window-model-tests_
  - Verify local desktop, SSH/WSL remote-server, sharing-host, and collaboration-guest modes return equivalent typed visible models while only the project host runner executes Cargo.
  - Verify request/cancel peer ownership, private candidate/member/target filtering, outside-worktree path removal, diagnostic bounding, proto path safety, disconnect stale mode, reconnect refresh, and participant-local UI state.
  - Verify enabled startup/menu/action/settings/panel registration in Sim and enabled HeadlessProject handler registration, and verify adding the dormant panel does not eagerly execute Cargo.
  - Add feature-mismatch cases: enabled client/disabled host renders `UnsupportedHost` once with no client-local fallback or retry; disabled client sends no Cargo workspace request to an enabled fake host; disabled headless lifecycle registers no Cargo workspace handler and records no invocation of the feature's metadata runner.
  - _Requirements: 1.1, 1.3, 1.7, 4.1, 4.5, 5.2, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 8.4, 8.6, 9.1, 9.2, 9.4, 9.6, 9.7, 9.8, 9.9_
  - _Depends on: 2, 3, 4, 8, 9_
  - _Reads: crates/remote_server/src/headless_project.rs, crates/remote_server/src/server.rs, crates/project/src/cargo_workspace_store.rs, crates/proto/proto/cargo.proto, crates/sim/src/sim.rs, crates/sim/src/sim/app_menus.rs_
  - _Writes: crates/remote?server/src/headless?project.rs (inline feature tests), crates/project/src/cargo?workspace?store.rs (inline remote/privacy tests), crates/sim/src/sim.rs (inline Cargo panel feature tests)_
  - _Validation: Run `cargo test -p remote_server --features test-support,rust-tools cargo_workspace`, `cargo test -p remote_server --no-default-features cargo_workspace_disabled`, `cargo test -p project --features test-support,cargo-workspace cargo_workspace_remote`, `cargo test -p sim --features test-support,rust-tools cargo_panel`, and `cargo test -p sim --no-default-features cargo_panel_disabled` with mock transports and fake runners._
  - Outcome: Integration coverage proves authoritative host execution, peer privacy, feature mismatch behavior, and enabled/disabled registration across local, remote, and multiplayer projects.
  - Design: D3, D9, D10, D11, D13, D15, D16
  - Done when: All five focused integration commands pass with fake transports/runners and no client-local fallback.
  - _Evidence: Both remote-server variants, the project remote/privacy suite, and both Sim panel feature variants passed with injected runners and typed protocol snapshots._

- [x] 13. Add the Rust-tools dependency, release, and CI boundary
  - _id: rust-workspace-tool-window-build-boundary_
  - _blocked_by: rust-workspace-tool-window-model, rust-workspace-tool-window-protocol, rust-workspace-tool-window-store, rust-workspace-tool-window-headless, rust-workspace-tool-window-tree-host, rust-workspace-tool-window-settings, rust-workspace-tool-window-sim-integration, rust-workspace-tool-window-remote-tests_
  - Add `script/check-rust-tools-feature-boundary`, modeled on the Comfy boundary check, to parse Sim, Cargo UI, project, remote-server, settings, and settings-content manifests; assert feature forwarding/optional dependencies/cfg integration; and inspect locked normal dependency trees for disabled Sim, remote-server, and `language_tools` builds.
  - Assert disabled graphs contain neither `cargo_ui` nor `cargo_metadata`, `language_tools` contains no Cargo-specific dependency in either build, Cargo protobuf remains buildable, `default = []` remains unchanged, and `rust-tools` stays independent from Comfy and Apple Metal features.
  - Add `--rust-tools`/`-RustTools` release-bundle selections and dry-run output that forwards `sim/rust-tools` to every Sim build/bundle invocation (including macOS `cargo bundle`) and `rust-tools` to the separately built remote server; update Rust-oriented bundling/release workflows to request matching features while leaving an explicit core mode available without them.
  - Add CI checks for `cargo check -p sim --features rust-tools`, `cargo check -p sim --no-default-features`, `cargo check -p remote_server --features rust-tools`, and `cargo check -p remote_server --no-default-features`, plus the boundary script and enabled/disabled registration tests.
  - Document in the script assertions that current `languages::init` still registers Rust in both variants, so the boundary is not broadened silently to rust-analyzer, grammars, or existing tasks.
  - _Requirements: 7.5, 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9, 9.10_
  - _Depends on: 1, 2, 3, 4, 5, 7, 8, 12_
  - _Reads: script/check-comfy-feature-boundary, script/bundle-mac, script/bundle-linux, script/bundle-freebsd, script/bundle-windows.ps1, script/flatpak/bundle-flatpak, script/install-linux, .github/workflows/run_tests.yml, .github/workflows/run_bundling.yml, .github/workflows/release.yml, .github/workflows/release_nightly.yml, crates/sim/Cargo.toml, crates/cargo_ui/Cargo.toml, crates/project/Cargo.toml, crates/remote_server/Cargo.toml, crates/settings/Cargo.toml, crates/settings_content/Cargo.toml, crates/languages/src/lib.rs, crates/proto/build.rs, assets/settings/default.json_
  - _Writes: script/check-rust-tools-feature-boundary, script/bundle-mac, script/bundle-linux, script/bundle-freebsd, script/bundle-windows.ps1, script/flatpak/bundle-flatpak, script/install-linux, tooling/xtask/src/tasks/workflows/run?tests.rs, tooling/xtask/src/tasks/workflows/run?bundling.rs, .github/workflows/run_tests.yml, .github/workflows/run_bundling.yml, .github/workflows/release.yml, .github/workflows/release_nightly.yml_
  - _Validation: Run `./script/check-rust-tools-feature-boundary`; run each bundle script's dry-run in core and Rust-tools modes and verify Sim/remote feature parity; run all four enabled/disabled `cargo check` commands named above._
  - Outcome: Manifests, bundle scripts, and CI enforce matching rust-tools variants and prove Cargo-only dependencies and initialization are absent from core builds.
  - Design: D1, D11, D13, D15, D16
  - Done when: The boundary script, all dry-run plans, and enabled/disabled Sim and remote-server checks pass in CI-compatible form.
  - _Evidence: `./script/check-rust-tools-feature-boundary` passed dependency-tree, cfg, defaults, and release dry-run assertions; generated CI workflows contain enabled/disabled Sim and remote-server checks and registration tests._

- [x] 14. Run the cross-crate acceptance and repository validation pass
  - _id: rust-workspace-tool-window-validation_
  - _blocked_by: rust-workspace-tool-window-model-tests, rust-workspace-tool-window-tree-tests, rust-workspace-tool-window-panel-tests, rust-workspace-tool-window-remote-tests, rust-workspace-tool-window-build-boundary_
  - Run every focused model, store, generic-host, Cargo-panel, proto, remote-server, settings, and Sim integration test and resolve any ordering, feature, platform, or test-isolation failures.
  - Run `./script/clippy` rather than `cargo clippy`, noting that it forces all features; separately retain the explicit disabled checks. Inspect the final diff for `metal_cargo`, broad `metal_*` renames, accidental Apple-Metal coupling, public provider APIs, manifest mutation actions, raw absolute-path/error serialization, or real-Cargo/network tests, and confirm existing Rust task-target tests still pass.
  - Confirm every acceptance criterion is represented by an automated check or an explicit review assertion and that no out-of-scope Cargo command or ecosystem provider was added.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9, 9.10_
  - _Depends on: 9, 10, 11, 12, 13_
  - _Reads: .agents/specs/rust-workspace-tool-window/requirements.md, .agents/specs/rust-workspace-tool-window/design.md, Cargo.toml, Cargo.lock, crates/project/, crates/language_tools/, crates/cargo_ui/, crates/proto/, crates/remote_server/, crates/sim/, assets/settings/, assets/keymaps/_
  - _Writes: none_
  - _Validation: Run `cargo test -p project --features test-support,cargo-workspace cargo_workspace`, `cargo test -p language_tools`, `cargo test -p cargo_ui`, `cargo test -p proto`, `cargo test -p remote_server --features test-support,rust-tools cargo_workspace`, `cargo test -p remote_server --no-default-features cargo_workspace_disabled`, `cargo test -p settings_content`, `cargo test -p settings`, `cargo test -p sim --features test-support,rust-tools cargo_panel`, `cargo test -p sim --no-default-features cargo_panel_disabled`, `./script/check-rust-tools-feature-boundary`, and `./script/clippy`; feature-scoped checks must pass without host Cargo metadata or network access, and any unrelated full-workspace lint prerequisite failure must be recorded with the scoped clippy result._
  - Outcome: Every acceptance criterion has implementation evidence and the complete enabled/disabled test, boundary, spec, and clippy suite is green.
  - Design: D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12, D13, D14, D15, D16
  - Done when: All feature-scoped commands pass, the living specification matches the implementation, no unchecked leaf task remains, and any unrelated repository-wide lint blocker is recorded.
  - _Evidence: All focused enabled/disabled tests, the boundary checker, format/diff checks, both spec validators, and targeted clippy passed. `./script/clippy` was also attempted and reached unrelated existing blockers: missing `projects/comfy/ComfyUI/comfy/weight_adapter/{oft,boft}.py` fixtures and a redundant clone in `crates/comfy_runtime/src/prompt_compiler.rs:1293`._
