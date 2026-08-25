# Requirements: Cargo execution presets and actions

## Purpose and status

This pack owns Cargo-specific execution configuration, contextual action availability, and compilation into Zed's existing Tasks and DAP systems. The versioned preset schema, settings precedence, safe workspace state, structured argv/env compilation, six contextual actions, and DAP conversion are verified baseline. This pack does not own process execution.

Canonical IDs are `cargo-execution/<criterion>`.

### Requirement 1: Preserve existing Cargo preset and action behavior [Verified baseline]

#### Acceptance criteria

1. **1.1** THE system SHALL support non-persisted defaults and named user/project presets containing subcommand, workspace/package scope, target selector, profile, features/default policy, target triple, structured Cargo/program arguments, environment overrides, working-directory policy, and task presentation.
2. **1.2** WHEN user and trusted project presets share an ID, THE project entry SHALL override fields for that project; an invalid entry SHALL be isolated without disabling valid entries.
3. **1.3** THE system SHALL persist named presets in existing settings scopes and persist only the active preset ID and non-secret selection state in workspace persistence.
4. **1.4** WHEN run, THE adapter SHALL compile a preset into an ordinary `TaskTemplate` and `TaskContext` and invoke the workspace task scheduler, leaving environment resolution, remote execution, terminal presentation, cancellation, history and rerun to existing systems.
5. **1.5** WHEN debug is valid, THE adapter SHALL compile an existing `DebugScenario` with the Cargo build locator and SHALL NOT implement DAP transport.
6. **1.6** FOR workspace, package and supported target selections, THE Cargo panel SHALL offer only applicable Build, Check, Run, Test, Bench and Debug actions with accessible unavailable reasons.
7. **1.7** THE adapter SHALL preserve user values as structured command, argument and environment fields and SHALL NOT concatenate an untrusted shell command.
8. **1.8** Execution SHALL require explicit invocation and SHALL be denied for restricted, disconnected, read-only guest, missing-Cargo or incompatible-host states without client-local fallback.
9. **1.9** Metadata refresh/navigation SHALL remain distinct from execution; `CargoWorkspaceStore` SHALL expose no arbitrary build/run/test API.
10. **1.10** Cargo presets SHALL augment, not replace, language tasks, runnables, terminals, `tasks.json`, `debug.json`, or DAP scenarios.

### Requirement 2: Complete preset authoring and missing configuration [Next]

#### Acceptance criteria

1. **2.1** WHEN explicitly selected, A preset SHALL support a toolchain override and SHALL pass it as structured Cargo invocation state on the authoritative host without installing or downloading a toolchain.
2. **2.2** WHEN a preset names a pre-launch task, THE system SHALL resolve only an existing Tasks-owned stable task reference, run it through Tasks, stop on failure/cancellation, and then start the Cargo task or DAP scenario; it SHALL NOT embed a second command language.
3. **2.3** THE Cargo panel SHALL provide an ephemeral preset editor that validates scope/target/profile/features/arguments/environment-key names/working directory before execution and previews a redacted structured command without exposing environment values.
4. **2.4** WHEN the user explicitly chooses Save for User or Save for Project, THE editor SHALL write through the corresponding existing settings workflow, explain project trust/shareability, and never copy environment values into workspace panel persistence.
5. **2.5** THE first extension milestone MAY add explicit Doc, Clippy and Fmt actions through Tasks; Clean SHALL require a destructive-action confirmation, and Tree SHALL use a bounded locked/offline invocation. Each action SHALL be capability/applicability gated.
6. **2.6** THE system SHALL NOT offer `cargo update`, dependency mutation, automatic fetching, tool installation, or automatic network activity from the panel.
7. **2.7** Dedicated compatibility tests SHALL prove Cargo presets coexist with user/global/project `tasks.json` and `debug.json`, preserve their priority/history behavior, and do not alter user-authored definitions.

### Requirement 3: Preserve failure, privacy and boundedness [Compatibility]

#### Acceptance criteria

1. **3.1** WHEN preset schema versions, fields or task references are invalid, THE system SHALL isolate the affected preset, show a bounded actionable diagnostic, and leave other presets and existing tasks/debug scenarios usable.
2. **3.2** THE UI, workspace state, logs, protocol and command preview SHALL show environment key names only; values MAY exist solely in user-authored settings and the in-memory task environment path.
3. **3.3** WHEN selection/configuration becomes stale, THE adapter SHALL re-resolve against the current Cargo snapshot and refuse removed packages/targets rather than execute an obsolete plan.

## Roadmap boundaries and non-goals

Now baseline is complete. Dedicated compatibility tests are the highest-priority gap; authoring UX and missing fields are Next. Doc/Clippy/Fmt/Clean/Tree require separate product sequencing within this pack. `cargo update` is rejected because it mutates resolution and can use the network.

Out of scope: a second task scheduler, terminal parsing, automatic dependency changes, inline shell strings, replacing configuration JSON, coverage/profiling execution, or moving arbitrary commands into `CargoWorkspaceStore`.

## Open questions

1. **Preset editor surface.** Recommended default: a Cargo-panel modal/popover with redacted argv preview, not a general settings editor. Task 2.2 depends on this interaction choice.
2. **Project save confirmation.** Recommended default: one explicit confirmation explaining repository sharing/trust and omitting environment values. Task 2.2 depends on final copy.
