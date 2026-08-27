# Requirements: Rust Workspace Tool Window

## Problem

Rust developers in Zed can browse files and use Rust language-server features, but they cannot inspect the Cargo structure of an open project as a workspace model. Discovering workspace membership, targets, defined and enabled features, and dependency declarations currently requires opening manifests or running Cargo manually. Zed needs a dockable Cargo view that makes this structure navigable while establishing only the reusable tree-panel behavior that later language ecosystems demonstrably need.

## Scope

### In scope

- A user-facing **Cargo** panel integrated with Zed's existing workspace docks, panel actions, settings, and menus.
- Cargo workspace and standalone-package discovery across all visible project worktrees.
- A finite, navigable tree of workspace members, targets, features, and direct dependencies.
- Cargo metadata collection in the project host environment for local, remote-server, and multiplayer projects.
- Automatic and manual refresh, partial results, stale-data handling, trust checks, cancellation, and actionable error states.
- A small Cargo-agnostic tree host and provider boundary used by the Cargo implementation.
- A compile-time `rust-tools` capability that includes the Cargo panel in Rust-oriented builds and excludes its UI, project-store implementation, execution path, and Cargo-only dependencies from core-editor builds.
- Deterministic model, UI, concurrency, accessibility, remote, and large-workspace verification.

### Out of scope

- Editing `Cargo.toml` through the tree, including adding or removing dependencies or changing feature definitions.
- Enabling or disabling Cargo features from the panel.
- Running build, check, test, run, bench, clean, update, or other Cargo commands.
- A recursively expandable or graphical transitive dependency graph.
- Vulnerability, license, update, or supply-chain auditing.
- Java, C#, Gradle, Maven, or .NET providers.
- A public extension or plugin API for build-system providers.
- A new `metal_rust` umbrella crate; that name remains available if a later distribution architecture demonstrates a need for it.
- Making Zed's existing Rust language registration, rust-analyzer integration, Rust grammar, task context provider, or existing task-target discovery optional; that broader separation is a follow-up feature.
- Search, filtering, drag-and-drop, multi-selection, or exact RustRover visual and toolbar parity.
- Persisting row selection or expansion state across application restarts; preservation is limited to refreshes during the current panel lifetime.

## Glossary

- **Cargo workspace root**: The directory reported by `cargo metadata` as `workspace_root`, including a virtual workspace with no root package.
- **Workspace member**: A package whose Cargo package ID appears in `workspace_members` for a discovered workspace.
- **Defined feature**: A feature name declared by a package in Cargo metadata.
- **Enabled feature**: A feature name reported for a package by the resolved graph from the panel's default-feature metadata invocation.
- **Direct dependency**: A dependency declaration attached directly to a workspace member's package metadata; it is not a recursively displayed edge from the resolved graph.
- **Project host**: The process with authoritative filesystem, environment, trust, and command-execution access for the project. It is the desktop process for a local project, the remote server for SSH/WSL-style projects, and the sharing host for a multiplayer project.
- **Tree host**: The Cargo-agnostic UI state and rendering component that presents a provider-supplied tree projection.
- **Provider**: The Cargo-specific adapter that discovers models, requests refreshes, projects Cargo data into tree nodes, and handles node activation.
- **`rust-tools`**: The compile-time capability used by Zed and the remote server to include this specification's Cargo tooling. For this MVP it does not imply that all pre-existing Rust language support is feature-gated.
- **Metal Rust**: Permitted product or distribution branding for a build that includes Rust tooling; it is not the Cargo panel title, Cargo module prefix, or the existing Apple Metal GPU feature.

## Requirements

### Requirement 1: Cargo panel lifecycle and discovery

**User story:** As a Rust developer, I want a dedicated Cargo panel that discovers every Cargo model in my open project, so that I can inspect project structure without searching manifests manually.

#### Acceptance criteria

1. **1.1** WHERE the `rust-tools` capability is included, WHEN the user invokes the Cargo panel action or selects Cargo from the View menu, THEN THE workspace SHALL open or focus a panel titled `Cargo` in an existing side dock.
2. **1.2** THE Cargo panel SHALL be available through the dock button and workspace layout persistence, SHALL support the left and right docks, and SHALL start closed for a workspace that has no previously persisted open Cargo panel.
3. **1.3** WHEN the panel first becomes active, THEN THE system SHALL discover Cargo manifests across every visible, non-single-file worktree without requiring a Rust source file or a running rust-analyzer instance.
4. **1.4** WHEN visible worktrees contain a virtual workspace, a package-root workspace, a standalone package, multiple independent Cargo workspaces, or any combination across worktrees, THEN THE panel SHALL represent every successfully discovered Cargo workspace exactly once.
5. **1.5** WHEN two discovered workspace roots or packages have the same display name, THEN THE panel SHALL show enough worktree-relative path context to distinguish them.
6. **1.6** IF no Cargo manifest is discoverable in any visible worktree, THEN THE panel SHALL show a non-error empty state that explains that no Cargo projects were found.
7. **1.7** WHEN visible worktrees are added, removed, reordered, connected, or disconnected, THEN THE panel SHALL invalidate its discovery state and reconcile its workspace roots with the current visible-worktree set.

### Requirement 2: Finite Cargo workspace tree

**User story:** As a Rust developer, I want Cargo concepts presented in a predictable hierarchy, so that I can understand a workspace without interpreting raw metadata.

#### Acceptance criteria

1. **2.1** WHEN a Cargo workspace loads successfully, THEN THE panel SHALL show a workspace root containing its workspace members, and each member SHALL expose `Targets`, `Features`, and `Dependencies` sections when the corresponding section is non-empty.
2. **2.2** WHEN a member declares library, binary, example, integration-test, benchmark, or custom-build targets, THEN THE `Targets` section SHALL show every target with its name, target kind, source path context, and required features when present.
3. **2.3** WHEN a member has feature information, THEN THE `Features` section SHALL show the union of defined and resolved-enabled feature names and SHALL distinguish `enabled`, `disabled`, and `enabled but not declared in the package feature map` states without presenting an unavailable resolved state as disabled.
4. **2.4** THE enabled-feature state SHALL reflect Cargo's default feature selection for the metadata invocation and SHALL NOT imply that every feature, target, or user-specific build configuration is enabled.
5. **2.5** WHEN a member declares dependencies, THEN THE `Dependencies` section SHALL group direct declarations as normal, development, and build dependencies and SHALL NOT recursively expand dependency packages.
6. **2.6** WHEN Cargo metadata exposes the applicable fields, THEN EACH dependency row SHALL distinguish its manifest name or rename, version requirement, resolved package version, optional state, default-feature use, requested dependency features, target condition, path/registry/Git source kind, and whether it resolves to another visible workspace member.
7. **2.7** IF Cargo metadata does not expose whether a declaration was inherited from `[workspace.dependencies]`, THEN THE panel SHALL omit that annotation rather than infer or mislabel it.
8. **2.8** THE panel SHALL sort workspace roots, members, sections, targets, features, and dependencies deterministically and SHALL assign stable opaque node identities that do not contain host-only absolute paths.
9. **2.9** IF one workspace cannot be loaded while another succeeds, THEN THE tree SHALL retain the successful workspace and place the failed workspace's status at the corresponding candidate root instead of replacing the entire panel with one global failure.

### Requirement 3: Navigation, interaction, and accessibility

**User story:** As a keyboard or pointer user, I want the Cargo tree to behave like an accessible Zed panel, so that I can inspect and navigate project structure efficiently.

#### Acceptance criteria

1. **3.1** THE Cargo panel SHALL provide toolbar controls for `Refresh`, `Expand All`, and `Collapse All`, with tooltips, accessible names, and disabled states when an action cannot have an effect.
2. **3.2** WHILE focus is in the Cargo tree, THE user SHALL be able to move to the previous or next visible row, jump to the first or last row, expand or collapse a branch, move to a parent or first child, and activate a navigable row using the existing platform keymap conventions.
3. **3.3** WHEN the user clicks a row, THEN THE panel SHALL select it; WHEN the user activates a disclosure control, THEN THE panel SHALL only toggle that branch; WHEN the user double-clicks or presses the activation key on a navigable row, THEN THE panel SHALL perform the row's navigation action.
4. **3.4** WHEN the user activates a workspace member, THEN THE workspace SHALL open that member's `Cargo.toml` using its project-relative path.
5. **3.5** WHEN the user activates a target, THEN THE workspace SHALL open the target's root source file using its project-relative path.
6. **3.6** WHEN the user activates a feature or dependency with a safely resolved local declaration target, THEN THE workspace SHALL open the owning or resolved local `Cargo.toml`; THE MVP SHALL NOT require exact TOML line positioning.
7. **3.7** IF a registry, Git, unshared, outside-worktree, or otherwise non-navigable dependency has no safe local project path, THEN activation SHALL perform no invalid filesystem operation and SHALL expose the reason through disabled affordance text or a tooltip.
8. **3.8** WHEN the user opens a row's context menu, THEN THE menu SHALL contain only applicable read-only navigation or copy actions and SHALL NOT contain manifest mutation or Cargo execution actions.
9. **3.9** THE rendered tree SHALL expose tree and tree-item accessibility roles, row labels, hierarchy levels, selection, and expansion state, and SHALL virtualize its flattened visible rows so that assistive and pointer interaction do not require rendering the complete expanded model at once.

### Requirement 4: Refresh, concurrency, and state preservation

**User story:** As a developer editing Cargo manifests, I want the panel to refresh without blocking or jumping unnecessarily, so that it remains trustworthy during active work.

#### Acceptance criteria

1. **4.1** WHEN a Cargo panel with no snapshot becomes active, THEN THE panel SHALL show a loading state immediately and SHALL collect and parse Cargo metadata without blocking the GPUI foreground thread.
2. **4.2** WHEN a visible worktree reports a relevant `Cargo.toml` or `Cargo.lock` addition, update, removal, or rename, THEN THE panel SHALL schedule one debounced automatic refresh for the resulting burst of changes.
3. **4.3** WHEN the user invokes `Refresh`, THEN THE panel SHALL start a refresh immediately, supersede any pending debounce, and make the refresh progress observable.
4. **4.4** WHILE a refresh is running and a previous snapshot exists, THE panel SHALL keep that tree usable and mark it stale or refreshing; WHILE no previous snapshot exists, THE panel SHALL keep the loading state visible.
5. **4.5** WHEN a newer refresh supersedes an older refresh or the panel/store is dropped, THEN THE system SHALL cancel or terminate obsolete Cargo processes where supported and SHALL reject every late result whose generation is no longer current.
6. **4.6** WHEN a current refresh succeeds, THEN THE panel SHALL replace affected workspace models atomically, clear stale/error status for those workspaces, and preserve expansion and selection for stable node identities that still exist.
7. **4.7** IF a selected node disappears after refresh, THEN THE panel SHALL select the nearest surviving ancestor or nearest visible row, and SHALL leave the tree unselected only when it has no rows.
8. **4.8** WHEN Cargo's normal metadata resolution writes `Cargo.lock` or updates Cargo caches, THEN THE resulting worktree event SHALL NOT create an unbounded refresh loop, and THE panel SHALL never write a manifest directly.

### Requirement 5: Empty, partial, stale, and failure states

**User story:** As a developer, I want failures scoped and explained, so that one broken workspace or environment does not make the whole Cargo panel useless.

#### Acceptance criteria

1. **5.1** IF a Cargo manifest is found but the project host cannot locate the `cargo` executable, THEN THE corresponding workspace candidate SHALL show an actionable `Cargo not found` state that identifies the project environment or toolchain as the place to fix it.
2. **5.2** IF Cargo exits unsuccessfully, THEN THE corresponding candidate SHALL show a concise metadata error with its worktree-relative manifest context and a safely bounded diagnostic derived from Cargo's standard error output.
3. **5.3** IF metadata JSON is malformed or missing required format-version-1 structure, THEN THE provider SHALL fail that candidate safely, preserve other successful candidates, and identify the response as unsupported or invalid without panicking; WHEN a non-structural target or source value is unknown, THEN THE provider SHALL retain it with a safe generic representation rather than fail the complete candidate.
4. **5.4** IF refresh of a previously successful workspace fails, THEN THE panel SHALL retain its last successful model as stale, attach the new error to that workspace, and allow explicit retry.
5. **5.5** WHEN the user retries after correcting Cargo, network, lockfile, manifest, trust, or remote-connection state, THEN THE current error SHALL clear only after a successful refresh for that candidate.
6. **5.6** WHEN all previously represented Cargo manifests are removed, THEN THE panel SHALL discard their stale models after reconciliation and return to the non-error empty state.
7. **5.7** IF dependency resolution requires unavailable network or registry data, THEN THE panel SHALL surface Cargo's failure without automatic rapid retries, silent online/offline mode changes, or fallback data presented as a successful resolved graph.

### Requirement 6: Local, remote, multiplayer, trust, and privacy behavior

**User story:** As a developer working locally, over a remote server, or with collaborators, I want the Cargo model to come from the authoritative project host without bypassing Zed's security boundaries.

#### Acceptance criteria

1. **6.1** WHEN Cargo metadata is requested for a local project, remote-server project, or shared project, THEN THE `cargo` process SHALL run only on the project host that owns the relevant files and project environment.
2. **6.2** WHEN the panel is attached to a remote-server or multiplayer client, THEN THE client SHALL request the same typed Cargo snapshot through the existing project RPC architecture and SHALL NOT execute Cargo against client-local absolute paths.
3. **6.3** BEFORE the project host executes Cargo for a worktree governed by Zed's worktree trust mechanism, THE system SHALL use the existing trust authority; WHILE the worktree is restricted, THE panel SHALL show a restricted state and SHALL NOT spawn Cargo for it.
4. **6.4** WHEN a previously restricted worktree becomes trusted, THEN THE provider SHALL invalidate that worktree and allow the normal debounced or explicit refresh path to load it.
5. **6.5** WHEN serving a multiplayer client, THE host SHALL return only workspace members, navigation paths, diagnostics, and dependency details permitted by that client's shared-worktree visibility, and SHALL NOT expose host-only absolute paths, environment variables, credentials, or private-file contents.
6. **6.6** WHILE a remote project is disconnected, THE panel SHALL retain any last successful tree as read-only stale data, disable refresh and navigation that require the connection, and SHALL refresh after reconnection rather than presenting the stale tree as current.
7. **6.7** THE panel's focus, selection, expansion, scroll position, and toolbar interactions SHALL remain participant-local UI state and SHALL NOT be synchronized to other collaborators.

### Requirement 7: Bounded reusable architecture and naming

**User story:** As a Zed maintainer, I want the Cargo panel to establish a small reusable tree boundary without encoding Cargo as the model for every language ecosystem.

#### Acceptance criteria

1. **7.1** THE implementation SHALL separate a Cargo-agnostic tree host, which owns focus, flattened-row rendering, selection, expansion, loading/error presentation, refresh coordination, and generic activation dispatch, from the Cargo provider and Cargo project model.
2. **7.2** THE Cargo-specific implementation SHALL own manifest discovery, project-host invocation, metadata parsing, Cargo domain types, Cargo labels and icons, stable Cargo node projection, and Cargo navigation decisions.
3. **7.3** THE reusable boundary SHALL exchange opaque node identities, presentation metadata, hierarchy, status, and activation capabilities and SHALL NOT require future providers to model their data as Cargo workspaces, packages, targets, features, or dependencies.
4. **7.4** THE MVP SHALL NOT expose a public third-party provider registry, dynamic plugin protocol, generalized package-manager API, or implementations for ecosystems other than Cargo.
5. **7.5** THE generic infrastructure SHALL remain in `language_tools`, Cargo discovery/model/store code SHALL use `cargo_workspace`, and the panel SHALL use `cargo_ui`; implementation SHALL NOT use `metal_cargo` or rename Rust folders to `metal_*`, while product/distribution text MAY use `Metal Rust` and a future umbrella crate MAY use `metal_rust` only when it remains explicitly distinguishable from Apple Metal GPU code.
6. **7.6** THE implementation SHALL preserve existing Rust task-target discovery behavior in `crates/languages/src/rust.rs`; shared Cargo parsing or command helpers MAY be extracted only when they remain UI-independent and do not make the Cargo panel depend on task-private types.

### Requirement 8: Verification and performance confidence

**User story:** As a Zed maintainer, I want deterministic coverage of the Cargo model and panel lifecycle, so that the feature can evolve without relying on a developer machine's Cargo installation or network.

#### Acceptance criteria

1. **8.1** THE Cargo model tests SHALL use deterministic format-version-1 metadata fixtures covering a virtual workspace, standalone package, multiple roots, malformed data, duplicate display names, and partial success.
2. **8.2** THE test suite SHALL cover all in-scope target kinds and normal, development, build, optional, renamed, target-specific, path, registry, Git, workspace-member, requested-feature, default-feature, and resolved-version dependency presentations.
3. **8.3** THE GPUI tests SHALL verify toolbar actions, pointer disclosure behavior, keyboard traversal, activation, context menus, accessibility state, loading, empty, error, stale, selection fallback, and expansion preservation using fake providers.
4. **8.4** THE project-store, provider, and generic-host tests SHALL collectively verify manifest invalidation, debounce coalescing, manual-refresh supersession, process cancellation, stale-result rejection, trust transitions, local/remote request parity, multiplayer filtering, disconnect/reconnect, and bounded errors at their owning layers.
5. **8.5** THE implementation SHALL include a large synthetic workspace test that exercises at least 10,000 projected rows and verifies deterministic flattening, stable identities, and visible-range rendering without a foreground-thread metadata parse.
6. **8.6** THE automated tests SHALL inject metadata results and command outcomes and SHALL NOT require an installed host `cargo`, network access, registry credentials, or mutation of the repository's real Cargo workspace.

### Requirement 9: Rust tooling build variants

**User story:** As a Zed distributor, I want the Cargo tool window behind a compile-time Rust tooling capability, so that a core editor build does not carry or initialize tooling it does not ship.

#### Acceptance criteria

1. **9.1** WHERE `rust-tools` is enabled for the Zed application, THE built application SHALL include `cargo_ui`, register and initialize the Cargo panel, its Cargo settings, actions, context key bindings, and View-menu entry, and expose the user-facing title `Cargo`.
2. **9.2** WHERE `rust-tools` is disabled for the Zed application, THE built application SHALL contain no Cargo panel registration, Cargo action or context-key initialization, Cargo settings registration, or Cargo View-menu entry.
3. **9.3** WHERE the Cargo workspace project capability is disabled, THE `project` build SHALL exclude `CargoWorkspaceStore` construction and request handling and SHALL not include `cargo_metadata` in its selected normal dependency graph.
4. **9.4** WHERE `rust-tools` is disabled, THE application and remote server SHALL perform no Cargo workspace discovery and SHALL spawn no Cargo metadata process on behalf of the Cargo panel/store capability, including after project activation, worktree changes, sharing, or remote connection events; separately triggered pre-existing Rust task-target discovery remains outside this guarantee.
5. **9.5** THE generic `language_tools` tree host SHALL compile and remain usable without `rust-tools` and SHALL have no dependency on `cargo_ui`, `cargo_metadata`, or Cargo workspace domain types.
6. **9.6** WHERE `rust-tools` is enabled for both a desktop client and its authoritative local, remote-server, or multiplayer project host, THE Cargo panel SHALL provide the same typed model and host-execution behavior specified in Requirements 1 through 8.
7. **9.7** WHERE a Cargo-capable client connects to a project host built without Cargo workspace support, THE panel SHALL show an actionable unsupported-host state, SHALL not retry rapidly, and SHALL not fall back to client-local Cargo execution.
8. **9.8** WHERE a client is built without `rust-tools`, THE client SHALL ignore the availability of Cargo workspace support on a host and SHALL send no Cargo workspace requests.
9. **9.9** THE protobuf build MAY compile inert Cargo request and response definitions in both variants, but disabled application and headless builds SHALL register no Cargo request handler and SHALL instantiate no Cargo store.
10. **9.10** THE continuous-integration and release-build validation SHALL compile Zed and `remote_server` with the capability enabled and disabled and SHALL verify that disabled selected dependency graphs exclude `cargo_ui`, `cargo_metadata`, and dependencies introduced solely for the Cargo panel.

## Constraints

- Follow the repository's Rust, GPUI, error-handling, async-task, timer, and build-validation rules. In GPUI tests, use GPUI executor timers rather than `smol::Timer`.
- Use `cargo metadata --format-version 1` semantics as the source of truth. Do not use `--no-deps`, because the resolved graph and enabled features are required; do not use `--all-features`, because the MVP reports Cargo's default feature selection.
- Do not force `--offline`, `--locked`, or `--frozen` independently of the project's Cargo configuration. Cargo's normal resolution may access configured registries, update caches, or create/update `Cargo.lock`; execution therefore remains trust-gated and its failures remain visible.
- Treat Cargo source identifiers as opaque. Do not parse undocumented source-ID representations to infer security or navigation behavior.
- Never serialize project-host absolute paths, environment variables, or raw unbounded Cargo output into UI node identities, logs, telemetry, or multiplayer responses.
- Use a virtualized visible-row list and background parsing/model conversion. Avoid wall-clock assertions that would be flaky across CI hosts.
- The panel is read-only with respect to user intent: it invokes only Cargo metadata and never directly edits manifests, dependency declarations, feature definitions, or source files.
- Keep existing Rust language initialization outside this feature boundary. `rust-tools` gates the new Cargo tool window only until a separate feature specifies how to isolate rust-analyzer integration, grammars, tasks, and all existing Rust language code.
- Interpret disabled Cargo-execution assertions at the new `CargoWorkspaceStore`/panel runner boundary. Existing task-specific `cargo metadata --no-deps` behavior in `crates/languages/src/rust.rs` is intentionally preserved and requires its own follow-up boundary if distributions must remove every Cargo invocation.
- Preserve `default = []` for current package feature behavior and make distribution feature selection explicit; a Rust-oriented or `Metal Rust` distribution enables `rust-tools` for both Zed and its matching remote-server artifact.
- Use `language_tools`, `cargo_workspace`, and `cargo_ui` for implementation. `Metal Rust` is branding only for this scope; do not introduce `metal_cargo` or broad `metal_*` renames.
