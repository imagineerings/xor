# Requirements: Cargo dashboard

## Purpose and status

This pack owns Zed's Cargo metadata/configuration model and the dockable user-facing `Cargo` panel. The model, store, generic tree host integration, panel, navigation, configuration projection, remote protocol, and focused scale tests are verified baseline. Required work is limited to a comprehensive evaluation fixture and formal performance budgets.

Canonical IDs are `cargo-dashboard/<criterion>`.

### Requirement 1: Preserve the Cargo model and panel baseline [Verified baseline]

#### Acceptance criteria

1. **1.1** WHERE `rust-tools` is enabled and visible Cargo manifests exist, THE system SHALL expose a dockable panel titled `Cargo` through Zed's existing panel, settings, menu, persistence, and focus systems.
2. **1.2** WHEN the panel is first activated, THE system SHALL lazily request the authoritative Cargo snapshot and SHALL NOT run metadata merely because a workspace window opened.
3. **1.3** WHEN visible worktrees contain virtual workspaces, ordinary workspaces, standalone packages, nested manifests, or independent roots, THE host SHALL discover them deterministically, deduplicate covered members, and retain candidate-scoped failures.
4. **1.4** WHEN metadata is converted, THE model SHALL represent packages, library, binary, example, test, bench, build-script and unknown targets, defined and resolved-enabled features, and bounded direct dependency declarations.
5. **1.5** THE panel SHALL show only direct dependencies in this baseline and SHALL NOT recursively render the resolved transitive graph or cycles.
6. **1.6** WHEN a user activates a package or target, THE panel SHALL navigate to the visible manifest or source; feature and dependency activation SHALL navigate only to a safely resolved visible local manifest.
7. **1.7** WHEN refresh replaces the tree, THE host SHALL preserve expansion and selection for surviving opaque IDs, choose a deterministic nearest fallback, and reject obsolete generations.
8. **1.8** THE panel SHALL preserve keyboard/focus/scroll behavior, accessible tree labels and roles, read-only context menus, Refresh, Expand All, Collapse All, and distinct loading, empty, partial, stale, unsupported, missing-Cargo and metadata-error states.
9. **1.9** THE implementation SHALL reuse `language_tools::language_tool_tree`, `project::cargo_workspace`, `CargoWorkspaceStore`, and `cargo_ui` and SHALL NOT add a parallel tree host or Cargo model.

### Requirement 2: Preserve bounded Cargo configuration facts [Verified baseline]

#### Acceptance criteria

1. **2.1** WHEN a snapshot refreshes, THE model SHALL expose implicit `dev` and `release` profiles and valid custom profile names while isolating malformed declarations as bounded partial diagnostics.
2. **2.2** WHEN a visible `rust-toolchain.toml` or `rust-toolchain` applies, THE model SHALL expose its declared channel, components, and targets without invoking rustup or installing anything.
3. **2.3** WHEN a trusted authoritative project environment can run the bounded compiler probe, THE model SHALL expose the host compiler release and host target; otherwise it SHALL report unknown, restricted, missing, or failed.
4. **2.4** THE dashboard SHALL distinguish host compiler target, an active preset's explicit target, and the unresolved Cargo-config default; it SHALL NOT claim full layered Cargo-config evaluation.
5. **2.5** WHEN a workspace/package is selected, THE panel SHALL summarize active scope, profile, feature set/default policy, target override/selector, and environment key names.
6. **2.6** WHEN no named preset is active, THE panel SHALL display a non-persisted Cargo-default configuration and label implicit values as defaults.
7. **2.7** WHEN relevant manifests, lockfiles, toolchain declarations, configuration or presets change, THE view SHALL use debounced generation refresh and retain the last safe snapshot as stale on failure.
8. **2.8** THE configuration model and protocol SHALL omit secret values, outside-visible absolute paths, raw process output, and unbounded diagnostics.

### Requirement 3: Close dashboard validation gaps [Required change]

<!-- impl: crates/project/tests/integration/cargo_workspace.rs#cargo_workspace_comprehensive_fixture -->
<!-- impl: crates/cargo_ui/src/cargo_panel.rs#cargo_dashboard_foreground_budget -->

#### Acceptance criteria

1. **3.1** THE repository SHALL provide one standalone deterministic evaluation fixture that combines multiple Cargo roots, a virtual workspace, a standalone package, every supported target/dependency form, profiles, toolchain declarations, duplicate names, malformed input, and partial failure without using host Cargo or network access.
2. **3.2** THE repository SHALL define repeatable time and memory budgets for metadata conversion, tree projection, and refresh of at least 1,000 packages and 10,000 visible rows, and SHALL fail a benchmark gate when an accepted budget regresses.
3. **3.3** THE benchmark SHALL separate background parsing/model conversion from GPUI foreground reconciliation and SHALL verify that command collection/parsing never blocks the foreground thread.

## Compatibility and non-goals

The host-authority, trust, privacy, feature-gating, and physical environment matrix are owned by `rust-tools-platform`. Cargo action execution is owned by `cargo-execution`. Transitive provenance is owned by `cargo-dependency-insight` and is not added to this direct-only panel baseline.

Out of scope: manifest/dependency/feature mutation, arbitrary Cargo execution in `CargoWorkspaceStore`, a universal package model, exact RustRover parity, public provider APIs, and full Cargo configuration reimplementation.

## Open questions

None. Dashboard placement, naming, ownership, direct-only scope, and configuration limits are resolved by the current implementation.
