# Requirements: Rust Development Workspace

## Purpose

This specification consolidates Zed's implemented Cargo workspace tool window with the next Rust workspace capabilities: richer project configuration, contextual Cargo actions, reusable execution presets, structured execution results, and a Rust test explorer. It translates the source material in `new-requirements/` from historical Zed terminology into the current Zed architecture and treats `tool-window/` as an implemented baseline rather than a greenfield plan.

The specification was audited against the repository at commit `58c21883` on 2026-08-14. The three documents in this directory are self-contained; the source folders remain research and provenance only.

## Status vocabulary

- **Verified baseline** means behavior present in the audited checkout and protected by existing code or tests. Later work may extend it but must not reimplement it.
- **Required change** means a verified gap in the audited checkout and is planned in `tasks.md`.
- **Compatibility requirement** constrains extensions so existing local, remote, multiplayer, trust, task, DAP, and feature-boundary behavior remains valid.

## Verified baseline

The checkout already contains the following behavior:

- `language_tools::language_tool_tree` is a Cargo-agnostic, virtualized tree host with opaque node identities, selection, expansion, focus, keyboard navigation, refresh generations, stale/error states, accessibility roles, and large-tree tests.
- `project::cargo_workspace` and `CargoWorkspaceStore`, behind `project/cargo-workspace`, discover visible Cargo manifests and collect format-version-1 Cargo metadata on the authoritative project host. The typed snapshot covers workspaces, packages, targets, defined and resolved features, direct dependencies, navigation paths, candidate failures, revisions, and completeness.
- `cargo_ui` projects that snapshot into the dedicated dockable **Cargo** panel, with lazy activation, refresh/expand/collapse controls, navigation, stable tree state, read-only context menus, settings, and partial/stale states.
- `zed/rust-tools` optionally selects `cargo_ui`; `remote_server/rust-tools` selects host-side Cargo workspace support. Disabled graphs exclude `cargo_ui` and `cargo_metadata`, and Cargo registration and execution paths are conditionally compiled.
- The Cargo metadata store uses the project environment, trust state, remote project host, bounded typed protocol, cancellation, and privacy filtering. It is not a general Cargo command runner.
- Rust task templates and rust-analyzer runnables already resolve through `TaskTemplate`, `TaskContext`, `TaskStore`, `Workspace::schedule_task`, terminals, and task history. Cargo debug builds already resolve through `DebugScenario`, the Cargo debugger locator, and DAP.

The checkout does **not** yet provide Cargo profiles, declared/effective toolchain and target configuration in the panel; reusable Cargo execution presets; contextual Cargo run/build/test/debug actions in the panel; a generic structured execution-result store; or a Rust test explorer/provider.

## Scope

### Now: first implementation milestone

- Preserve and regression-test the existing Cargo model/dashboard behavior while extending it.
- Add bounded Cargo configuration facts and an explicit active configuration to the Cargo panel.
- Add Cargo execution presets and contextual actions that compile into existing Tasks and DAP flows.
- Add an internal, language-neutral structured execution-result contract and a dockable Tests experience.
- Add a Rust test provider without duplicating rust-analyzer semantic indexing.
- Support the same authoritative-host, trust, privacy, remote, multiplayer, cancellation, and `rust-tools` boundaries as the Cargo baseline.

### Next

- Call hierarchy using rust-analyzer capabilities.
- Generic coverage overlays with a Rust `cargo llvm-cov` adapter.
- Dependency provenance and source inspection beyond the current direct-dependency tree.
- Improved debug-adapter acquisition and project creation templates.

### Later

- Profiler/flamegraph integration, advanced refactorings, macro-expansion history, framework-specific semantic views, and an optional `cargo-nextest` adapter after protocol and installation policy validation.

### External integrations

- System profilers and ecosystem tools such as `cargo-audit`, `cargo-deny`, `cargo-outdated`, `cargo-unused-features`, and `evcxr` remain external tools until separate specifications define installation, trust, protocol, and UX contracts.

### Rejected approaches

- A second Rust semantic/indexing engine.
- A universal build-system/package-manager model or a public provider/plugin API.
- A Cargo-specific shape in `language_tools` or the generic result model.
- A second terminal, Tasks, `tasks.json`, `debug.json`, or DAP implementation.
- Turning `CargoWorkspaceStore` into a general build/test/run process runner.
- `metal_cargo` or broad `metal_*` folder renaming. **Metal Rust** may be distribution branding; implementation names remain descriptive.

### Out of scope for Now

- Editing manifests, dependencies, profiles, or features through the tree.
- Automatic dependency changes, tool installation, registry access, or other network activity.
- Coverage, profiling, dependency/license/vulnerability auditing, call hierarchy, or graphical dependency graphs.
- Making rust-analyzer, Rust grammars, all existing Rust tasks, or every Rust language module optional under `rust-tools`.
- Java, C#, Gradle, Maven, or .NET implementations.

## Requirements

### Requirement 1: Preserve the Cargo project dashboard baseline [Verified baseline and compatibility]

**User story:** As a Rust developer, I want the implemented Cargo dashboard to remain reliable while new workspace capabilities are added.

#### Acceptance criteria

1. **1.1** WHERE `rust-tools` is enabled and a supported project contains visible Cargo manifests, THE system SHALL expose a dockable panel titled `Cargo` through Zed's existing panel, settings, menu, persistence, and focus systems.
2. **1.2** WHEN the Cargo panel is first activated, THE system SHALL lazily request the authoritative Cargo snapshot and SHALL NOT collect Cargo metadata merely because an unopened workspace window exists.
3. **1.3** WHEN visible worktrees contain virtual workspaces, ordinary workspaces, standalone packages, nested manifests, or multiple independent roots, THE authoritative host SHALL discover them deterministically, deduplicate covered members, and retain candidate-scoped failures as partial results.
4. **1.4** WHEN Cargo metadata is converted, THE model SHALL represent packages, libraries, binaries, examples, tests, benches, build scripts, defined features, resolved enabled features, and finite direct dependency declarations with the source and dependency distinctions already supported by `cargo_workspace`.
5. **1.5** THE Cargo dependency tree SHALL remain direct-only in Now; THE system SHALL NOT recursively project the resolved transitive graph or create dependency cycles.
6. **1.6** WHEN a user activates a package or target, THE panel SHALL navigate to its visible `Cargo.toml` or source file; WHEN a feature or direct dependency has a safely resolved local declaration, THE panel SHALL navigate to the most relevant visible manifest location.
7. **1.7** WHEN a snapshot refresh replaces the tree, THE tree host SHALL preserve expansion and selection for surviving opaque node identities, choose the documented nearest fallback for removed nodes, and reject obsolete generations.
8. **1.8** THE Cargo panel SHALL preserve existing keyboard navigation, accessibility labels and roles, selection, scrolling, read-only context menus, Refresh, Expand All, Collapse All, loading, empty, partial, stale, unsupported, missing-Cargo, and metadata-error behavior.
9. **1.9** THE extension work SHALL reuse `language_tools::language_tool_tree`, `project::cargo_workspace`, `CargoWorkspaceStore`, and `cargo_ui`; it SHALL NOT introduce a parallel dashboard, Cargo model, or tree host.

### Requirement 2: Present Cargo workspace configuration [Required change]

**User story:** As a Rust developer, I want to understand the configuration under which Cargo actions will run before I invoke them.

#### Acceptance criteria

1. **2.1** WHEN a workspace snapshot is refreshed, THE Cargo model SHALL expose the standard `dev` and `release` profiles plus valid custom profile names declared by visible workspace manifests, while reporting malformed or unreadable profile declarations as bounded partial diagnostics.
2. **2.2** WHEN a visible `rust-toolchain.toml` or `rust-toolchain` applies to a Cargo root, THE model SHALL expose its declared channel and supported components/targets without executing rustup or installing a toolchain.
3. **2.3** WHEN the authoritative project environment can execute a bounded toolchain probe in a trusted worktree, THE model SHALL expose the host Rust compiler release and host target triple; WHEN it cannot, THE dashboard SHALL display an explicit unknown, restricted, missing-tool, or failed state rather than inventing an effective value.
4. **2.4** THE dashboard SHALL distinguish the host compiler triple, an active preset's explicit `--target`, and an unresolved Cargo-config default; it SHALL NOT claim to have fully evaluated Cargo's layered configuration when it has not.
5. **2.5** WHEN a user selects a Cargo workspace or package, THE panel SHALL display an active configuration summary containing scope, profile, selected features, default-feature policy, target triple override, target selector, and environment-key names.
6. **2.6** WHEN no saved preset is active, THE panel SHALL use a non-persisted default configuration that means Cargo defaults for the selected scope and SHALL label implicit values as defaults.
7. **2.7** WHEN metadata, a toolchain declaration, Cargo lock state, or a configuration preset changes, THE relevant configuration view SHALL refresh through the existing debounced generation lifecycle and SHALL retain the last good snapshot as stale when a new probe fails.
8. **2.8** THE configuration model SHALL contain no secret values, absolute paths outside visible worktrees, raw process output, or unbounded diagnostics.

### Requirement 3: Compile Cargo presets and contextual actions into Tasks and DAP [Required change]

**User story:** As a Rust developer, I want repeatable Cargo actions from workspace context without learning a second execution system.

#### Acceptance criteria

1. **3.1** THE system SHALL support ephemeral presets and named user- and project-scoped Cargo presets containing a Cargo subcommand, workspace/package scope, target selector, profile, features, default-feature policy, target triple, additional argument arrays, working-directory policy, and environment-key/value overrides.
2. **3.2** WHEN user and project presets have the same stable identifier, THE project preset SHALL override the user preset for that project; invalid presets SHALL be isolated and surfaced without preventing valid presets from loading.
3. **3.3** THE system SHALL persist user and project presets through Zed's existing settings scope and trust behavior and SHALL persist only an active preset identifier and non-secret selection state in workspace persistence.
4. **3.4** WHEN a preset is executed, THE Cargo adapter SHALL compile it into an ordinary `TaskTemplate` plus `TaskContext` and call the existing workspace task scheduler, so save policy, shell/environment resolution, remote execution, terminal presentation, concurrency, cancellation, history, and rerun behavior remain owned by Tasks and terminals.
5. **3.5** WHEN a preset is debugged and the selected target is debuggable, THE Cargo adapter SHALL compile it into an existing `DebugScenario`/Cargo build task and start it through the existing debugger provider and DAP locator; it SHALL NOT launch or speak DAP itself.
6. **3.6** WHEN the selection represents an applicable package, binary, example, test, bench, or workspace scope, THE Cargo panel SHALL offer only contextually valid Build, Check, Run, Test, Bench, and Debug actions, with unavailable actions disabled or omitted and an accessible reason.
7. **3.7** WHEN a contextual action can reuse an equivalent Rust task template or rust-analyzer runnable, THE adapter SHALL reuse its command semantics or common Cargo argument builder rather than maintain conflicting private command construction.
8. **3.8** THE Cargo panel SHALL keep metadata navigation and refresh actions separate from execution actions, SHALL require an explicit user invocation for execution, and SHALL NOT offer dependency mutation, feature mutation, manifest editing, or automatic network commands.
9. **3.9** WHEN arguments, feature names, target names, environment entries, or paths are compiled, THE adapter SHALL preserve them as structured command/argument/environment fields and SHALL NOT construct a shell command by concatenating untrusted strings.
10. **3.10** WHEN a project is restricted, disconnected, read-only as a multiplayer guest, missing Cargo, or connected to a host without the required capability, THE panel SHALL prevent execution and present the reason without falling back to client-local execution.

### Requirement 4: Provide generic structured execution results [Required change]

**User story:** As a developer, I want test and later tool results to be queryable and navigable without forcing every language to model itself as Cargo.

#### Acceptance criteria

1. **4.1** THE project layer SHALL provide an internal language-neutral result model with stable run and node identities, parent/child relationships, provider-owned opaque keys, display labels, generic node kinds, queued/running/passed/failed/skipped/cancelled/error/stale states, duration, bounded messages, and optional visible `ProjectPath` navigation.
2. **4.2** THE generic result contract SHALL describe execution trees and events, not packages, targets, features, Cargo commands, or a universal build-system model.
3. **4.3** WHEN a provider publishes discovery or run events, THE result store SHALL apply them monotonically to the matching provider/run generation, ignore duplicate events idempotently, reject stale or cross-project events, and preserve the last complete run separately from an in-progress run.
4. **4.4** WHEN retained result limits are exceeded, THE store SHALL evict oldest completed runs and bounded output details deterministically while retaining the current run, its summary counts, and actionable failure locations.
5. **4.5** WHEN a structured execution task is scheduled, THE generic task bridge SHALL expose lifecycle completion and cancellation to the result adapter while preserving the ordinary terminal task, task history, and user-visible output.
6. **4.6** THE generic results UI SHALL provide a dockable panel titled `Tests` with provider/suite/test hierarchy, filtering by status or text, keyboard and accessible tree interaction, run/cancel/rerun-failed controls, summaries, failure navigation, and links to the owning task terminal.
7. **4.7** WHEN no structured provider is available, data is loading, results are empty, discovery is partial/stale, execution fails, or a host is incompatible, THE Tests panel SHALL show a distinct actionable state and SHALL NOT infer success from absent data.
8. **4.8** THE structured result types, protocol, store, task bridge, and tree projection SHALL be usable by a future in-tree non-Rust provider without depending on `cargo_ui`, `cargo_metadata`, or Cargo/Rust domain types, and SHALL remain an internal Zed contract rather than a public extension API.

### Requirement 5: Provide a Rust test provider and explorer [Required change]

**User story:** As a Rust developer, I want to discover, run, debug, and inspect tests at workspace, package, target, module, and test-case scopes.

#### Acceptance criteria

1. **5.1** WHEN a trusted Cargo project is supported, THE Rust test provider SHALL project workspace, package, test-bearing target, module/group, and test-case nodes with stable identities and visible source navigation where safely available.
2. **5.2** THE provider SHALL reuse Cargo workspace target data and existing rust-analyzer runnable/semantic information where available and SHALL NOT parse Rust source to create a second semantic index.
3. **5.3** WHEN complete test-case discovery requires tool execution, THE authoritative host SHALL use a separately injectable, bounded Rust test-discovery runner; it SHALL NOT add build/test/run methods to `CargoWorkspaceStore`.
4. **5.4** BEFORE the production discovery adapter is selected, THE implementation SHALL validate its protocol against unit, integration, binary, example-harness, benchmark, ignored, and doctest fixtures on supported stable toolchains; unknown records SHALL yield partial discovery instead of panics or fabricated tests.
5. **5.5** WHEN the user runs a single test, THE provider SHALL schedule an exact existing Cargo/Rust task and derive the test result from the observed task lifecycle; it SHALL NOT scrape ANSI-rendered terminal text for pass/fail status.
6. **5.6** WHEN the user runs a suite whose individual outcomes cannot be obtained from the validated structured protocol, THE explorer SHALL report the suite aggregate and leave child outcomes unknown/stale rather than assigning the aggregate outcome to every child.
7. **5.7** WHEN the user debugs a supported Rust test, THE provider SHALL create an existing Cargo `DebugScenario` and use the current Cargo debugger locator and DAP provider; unsupported doctest or harness debug cases SHALL be disabled with an explanation.
8. **5.8** WHEN the user cancels a test run or starts a superseding run for the same scope, THE provider SHALL cancel the owned task/discovery work where possible, mark the run accurately, and reject late results.
9. **5.9** WHEN the user invokes rerun-failed, THE provider SHALL schedule only failed/error test nodes that still resolve in the current discovery generation, with a bounded concurrency policy and an explicit summary for removed tests.
10. **5.10** THE provider SHALL preserve terminal output in the existing terminal experience, store only bounded structured summaries and failure messages in the result model, and never serialize process environment values.
11. **5.11** THE Now implementation SHALL require no downloaded runner or network access; an optional `cargo-nextest` adapter remains Later unless a separate decision defines detection, installation, protocol stability, and fallback behavior.

### Requirement 6: Preserve authoritative-host behavior across project modes [Compatibility and required change]

**User story:** As a local, remote, container, WSL, or multiplayer user, I want the Rust workspace to act on the project host and fail safely when capabilities differ.

#### Acceptance criteria

1. **6.1** WHEN the project is local, all Cargo model probes, test discovery, and user-invoked tasks SHALL use the existing local project environment and worktree/trust boundaries.
2. **6.2** WHEN the project is hosted by `remote_server`, SSH, or another supported remote mode, Cargo model probes and test discovery SHALL execute on that authoritative host and user actions SHALL use existing remote Tasks/DAP; THE client SHALL NOT execute against a mirrored local path.
3. **6.3** WHERE WSL or dev-container projects are represented by Zed's existing remote/project-environment mechanisms, THE Rust workspace SHALL use those mechanisms without a provider-specific local-filesystem execution path; unsupported modes SHALL be labeled explicitly.
4. **6.4** WHEN a multiplayer guest views a shared project, THE host SHALL filter snapshots and structured results to worktrees visible to that peer and SHALL reject guest execution where existing Tasks/DAP policy disallows it.
5. **6.5** WHEN client and host capabilities or protocol versions differ, THE UI SHALL show an actionable feature-mismatch state, avoid retry loops, preserve unrelated editor functionality, and SHALL NOT downgrade to unsafe local execution.
6. **6.6** THE remote protocol SHALL carry stable IDs, bounded status/configuration/result fields, and visible `ProjectPath` values only; it SHALL NOT carry absolute host paths, environment values, raw Cargo metadata, terminal streams, or unbounded diagnostics.
7. **6.7** WHEN a remote connection disconnects, reconnects, or changes host generation, THE stores SHALL cancel or invalidate in-flight work, retain the last safe snapshot as stale where appropriate, and accept results only from the current peer/project generation.
8. **6.8** THE desktop and headless host SHALL register Cargo configuration, Rust test discovery, and structured-result request handlers only for capabilities compiled into that build.

### Requirement 7: Enforce trust, privacy, lifecycle, and failure boundaries [Compatibility and required change]

**User story:** As a user, I want project-defined Rust tooling to remain explicit, cancellable, bounded, and safe.

#### Acceptance criteria

1. **7.1** WHILE any involved worktree is untrusted, THE system SHALL perform no Cargo metadata/configuration probe, Rust test-discovery command, task, or debug launch for that worktree and SHALL show how to enable trust.
2. **7.2** WHEN trust is revoked, THE system SHALL cancel owned discovery/probe work, invalidate executable presets, retain only privacy-safe stale data, and reject late results from the former trust generation.
3. **7.3** THE system SHALL never initiate dependency fetching, toolchain installation, runner installation, or registry/network activity merely by opening or refreshing either panel.
4. **7.4** THE project model, protocol, UI summaries, telemetry, and logs SHALL omit environment values and secrets; user-facing summaries SHALL show environment key names only, and errors SHALL use existing bounded sanitization. Explicit values may remain in the user-authored settings source and the in-memory `TaskTemplate` passed to the existing task environment path.
5. **7.5** WHEN a manifest, lockfile, toolchain declaration, relevant Cargo config file, preset, source test declaration, or provider capability changes, THE owning store SHALL debounce the minimum relevant invalidation and cancel or supersede obsolete work.
6. **7.6** WHEN a refresh or run fails after a good result, THE UI SHALL retain the last privacy-safe data as stale with the new error; WHEN no good result exists, THE UI SHALL show a non-stale error or empty state as appropriate.
7. **7.7** WHEN malformed metadata, manifests, preset settings, protocol events, paths, tool output, or unknown enum values are received, THE system SHALL fail fallibly, isolate the affected root/provider/run, and SHALL NOT panic or discard all unrelated valid data.
8. **7.8** THE system SHALL use bounded command timeouts/output, retained-run limits, diagnostic lengths, node counts, and protocol payload sizes, with observable truncation or partial-state indicators.
9. **7.9** THE system SHALL preserve existing Cargo panel and task behavior when structured results or Rust test discovery are unavailable or disabled.

### Requirement 8: Preserve the `rust-tools` build boundary [Compatibility and required change]

**User story:** As a Zed distributor, I want Rust workspace tooling to remain optional without claiming all existing Rust language support is optional.

#### Acceptance criteria

1. **8.1** WHERE `zed/rust-tools` is enabled, THE build SHALL include and initialize the Cargo panel, Cargo presets/actions, structured Tests panel, Rust test provider, and their settings/menu registrations.
2. **8.2** WHERE `zed/rust-tools` is disabled, THE build SHALL register none of those Cargo/Rust workspace UI elements, settings, actions, context keys, menus, providers, stores, or request handlers and SHALL execute no Cargo workspace/configuration or Rust test-discovery command on their behalf.
3. **8.3** WHERE the corresponding project features are disabled, THE selected normal dependency graph SHALL exclude `cargo_metadata`, `cargo_ui`, and every dependency introduced solely for Cargo workspace or Rust test tooling.
4. **8.4** THE generic `language_tools` tree host and generic structured-result contract SHALL have no dependency on `cargo_ui`, `cargo_metadata`, `project::cargo_workspace`, or Rust test provider types.
5. **8.5** WHERE `remote_server/rust-tools` is enabled or disabled, THE headless build SHALL respectively include or exclude the Cargo configuration/test provider stores and request handlers in parity with the desktop capability.
6. **8.6** Inert protobuf definitions MAY remain compiled in disabled builds when removing them would add disproportionate build complexity, but disabled builds SHALL instantiate no associated store and register no associated request handler.
7. **8.7** THE existing Rust language initialization, Rust grammars, rust-analyzer integration, and pre-existing Rust task-target discovery SHALL remain outside this Cargo/Rust-workspace feature boundary unless a separate follow-up specification makes them optional.
8. **8.8** THE feature-boundary and release validation SHALL cover enabled and disabled desktop and remote-server builds, dependency leakage, feature forwarding, bundle plans, and multiplayer capability mismatch.

### Requirement 9: Validate correctness and scale without machine-specific tools [Required change]

**User story:** As a maintainer, I want deterministic evidence that the consolidated Rust workspace remains correct, responsive, and portable.

#### Acceptance criteria

1. **9.1** THE tests SHALL use deterministic fixtures for Cargo configuration, preset conversion, test discovery, structured events, malformed/partial data, every supported target/dependency kind, and stable identity across refreshes.
2. **9.2** THE tests SHALL cover virtual workspaces, standalone packages, multiple roots, custom profiles, toolchain declarations, host/explicit target distinctions, active preset precedence, and missing tools.
3. **9.3** THE tests SHALL cover contextual action availability and exact `TaskTemplate`, `TaskContext`, and `DebugScenario` conversion without launching commands from unit tests.
4. **9.4** THE tests SHALL cover result-event idempotency, cancellation, late/stale rejection, retention limits, suite aggregate semantics, rerun-failed, and navigation.
5. **9.5** THE GPUI tests SHALL cover Cargo and Tests panel registration/persistence, loading/empty/partial/stale/error/mismatch rendering, keyboard/focus/expand-collapse/filter behavior, and SHALL use GPUI executor timers for debounce or timeout behavior.
6. **9.6** THE remote/multiplayer tests SHALL use injected runners and fake project environments and SHALL cover local/SSH-like/headless routing, peer filtering, trust transitions, disconnect/reconnect, bounded protocol data, and enabled/disabled host mismatch.
7. **9.7** THE automated tests SHALL not require the developer machine's Cargo, rustc, rustup, test runner, registry credentials, network, or mutation of the repository's real Rust workspace.
8. **9.8** THE performance suite SHALL exercise at least 10,000 dashboard/result rows and a synthetic workspace with at least 1,000 packages and 10,000 tests, verify deterministic bounded projection and visible-range rendering, and verify metadata/configuration/test parsing does not block GPUI's foreground thread.

## Open questions

### OQ1: Should project presets live in `.zed/settings.json` or a dedicated Cargo preset file?

**Recommended default:** Store `cargo.presets` in existing user and project settings (`settings.json` and `.zed/settings.json`) for Now, because Settings already provides precedence, remote synchronization, trust gating, schemas, and edit surfaces. Persist only the active preset ID in workspace persistence. A dedicated file would add another watcher, schema, migration, and trust surface without replacing Tasks.

**Work affected:** The preset schema/storage task and documentation. The runtime preset-to-Task/DAP contract is unaffected.

### OQ2: Should the Tests panel ship in the same release as contextual Cargo actions?

**Recommended default:** Yes, behind the same `rust-tools` capability, but gate release on the test-protocol fixture task. If supported stable toolchains cannot provide bounded discovery and exact per-test lifecycle outcomes without terminal scraping, ship contextual Cargo actions first and keep the Tests panel feature-internal until that gate passes.

**Work affected:** Tests panel registration, Rust test provider initialization, release notes, and CI feature expectations. The generic result model can land independently.

## Naming and branding

- User-facing names: **Cargo** panel and **Tests** panel.
- Compile-time capability: `rust-tools`.
- Existing generic host: `language_tools`.
- Cargo model/store: `cargo_workspace` and `CargoWorkspaceStore`.
- Cargo panel/preset adapter: `cargo_ui`.
- Recommended new descriptive modules: `structured_execution`, `test_explorer`, and `rust_test_provider`.
- **Metal Rust** is acceptable product/distribution branding. If an umbrella crate is ever justified, `metal_rust` is acceptable only with documentation distinguishing it from Zed's Apple Metal GPU features/modules. This specification does not add such a crate.
