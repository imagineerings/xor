# Zed Rust Development Environment — Design

**Status:** implementation-ready design subject to exact type-name trace at implementation SHA
**Zed architecture reference:** `main` at `0307288d903afd5673c361c548bd448fc8a684df`, 2026-08-13

## 1. Architectural decision

Adopt the four shared foundations proposed by the research prompt, with one refinement: treat the Rust project model as an **orchestration/read model**, not a second semantic model. rust-analyzer remains authoritative for Rust semantics; Cargo metadata/settings remain authoritative for package/target structure; Zed owns the cached presentation/execution coordination state.

Foundations:

1. `RustProjectModel` concept — worktree-scoped cached Cargo structure and metadata health.
2. `CargoExecutionSpec` concept — reusable typed execution/configuration description that compiles into existing tasks/debug scenarios.
3. `StructuredExecutionResult` concept — generic suite/case event/result model with Rust test provider first.
4. `AnalysisOverlay` concept — generic source annotations reserved for the Next-phase coverage work.

Names above are conceptual. Before coding, trace current source and choose names consistent with Zed conventions.

## 2. Existing architecture to reuse

Current Zed workspace dependencies show `cargo_metadata`, `cargo_toml`, DAP/debugger crates and dev-container infrastructure already exist. Current documentation confirms:

- tasks can be global/worktree/one-shot/language-provided;
- tasks execute on the remote host for remote projects;
- Rust tasks already receive `ZED_CUSTOM_RUST_PACKAGE`;
- Rust debugging uses CodeLLDB/GDB and Cargo build inference;
- remote source/language servers/tasks/terminal live on the remote server;
- dev-container tasks/language servers/terminal live in the container;
- worktrees start restricted and trust controls execution-sensitive project configuration.

### Source areas to trace before first PR

- `crates/project` — worktree/project/language-server coordination.
- `crates/task` plus current task UI crate(s) — task model/resolution/spawn and task inventory.
- `crates/dap`, `crates/dap_adapters`, `crates/debugger_tools`, `crates/debugger_ui` — debug scenario model and adapter capabilities.
- `crates/editor` — gutter controls, source annotations, navigation.
- current workspace/project panel crates — panel integration.
- current worktree trust implementation.
- remote protocol/protobuf declarations used by project/task state.

The first architecture task must record exact file/type/action names at the implementation SHA. This avoids baking historical type names into the plan.

## 3. Foundation A — Worktree-scoped Rust project model

### Responsibilities

- Identify Cargo workspace roots associated with a Zed worktree.
- Cache packages, members, targets, declared features, profile names, manifest paths, selected toolchain/target state and metadata health.
- Coordinate refresh/invalidation.
- Provide read-only queries to UI, task generation and configuration validation.

### Non-responsibilities

- symbol resolution, type inference, references or refactoring;
- proc-macro expansion;
- continuous `cargo check`;
- package mutation/network update.

### Data model

```text
RustWorktreeState
  generation: u64
  status: Loading | Ready | Stale | Error
  workspaces: Vec<CargoWorkspaceSummary>
  active_config: ActiveRustConfig
  last_refresh: Instant/Time
  error: Option<ProjectLoadError>

CargoWorkspaceSummary
  root_manifest
  packages: Vec<CargoPackageSummary>
  workspace_default_members
  target_directory? (only if needed)

CargoPackageSummary
  package_id
  name
  manifest_path
  edition
  targets: Vec<CargoTargetSummary>
  declared_features

CargoTargetSummary
  name
  kinds: lib/bin/example/test/bench/custom-build/...
  source_path?
  required_features

ActiveRustConfig
  toolchain?
  target_triple?
  profile?
  feature_selection
```

Use stable IDs derived from Cargo package IDs + target identity, never list indices.

### Refresh strategy

Relevant changes include `Cargo.toml`, `Cargo.lock`, `.cargo/config*`, toolchain files and project Rust settings. A file event marks state stale and schedules a debounced refresh. Multiple events collapse into one generation. If a newer generation starts, older results are discarded/cancelled.

Do not refresh for arbitrary `.rs` edits unless a known project-structure signal requires it.

### Data acquisition

Preferred order:

1. Reuse existing Zed/rust-analyzer-known workspace information if the current API exposes equivalent Cargo metadata with clear freshness semantics.
2. Use `cargo metadata --format-version ...` through Zed's project-side command/task execution primitive when extra structure is needed.
3. Parse manifest-only information with existing `cargo_toml` only for cheap display/validation that does not claim resolved dependency truth.

Never independently maintain a semantic package graph that conflicts with Cargo metadata.

### Remote boundary

Model acquisition occurs where the project lives. The local UI receives a compact serialized summary. Avoid sending full Cargo metadata JSON repeatedly if a typed delta or bounded summary is sufficient.

### Trust

Reading tracked manifest text is safe; spawning Cargo can execute/configure project-controlled behaviour depending on Cargo/build environment. Treat refresh that spawns Cargo as trust-sensitive. Restricted mode may show a manifest-only degraded view and an explicit “Trust to load Cargo metadata” action rather than executing silently.

### Failure behaviour

- Keep last-good data as `Stale`.
- Surface concise error + “View details”, “Retry”, “Open manifest/config”.
- If Cargo/toolchain missing, show setup state; do not install silently.
- Cancellation is normal, not an error toast.

## 4. Feature 1 UI — Cargo dashboard

Use progressive disclosure. Recommended product shape: a compact Rust/Cargo section associated with the Project panel or a small dedicated panel only if panel affordances prove necessary.

Default view:

```text
Rust
  workspace-name          Ready
  Toolchain  stable       Target host
  Features   default + 2

  app
    bin app               ▶  Debug
    example demo          ▶
    tests (3 targets)
  core
    lib
    benches (1)
```

Keyboard/command equivalents exist for every primary action. Context actions resolve to existing tasks.

Do not permanently display every Cargo command. “More actions…” exposes check/build/doc/clippy/fmt/clean/tree/update.

## 5. Foundation B — Unified Cargo execution/configuration model

### Conceptual data model

```text
CargoExecutionSpec
  id?: stable preset id
  label
  scope: Workspace | Package(package_id)
  target?: TargetRef
  cargo_action: Check | Build | Run | Test | Bench | Doc | Clippy | Fmt | Clean | Custom
  target_triple?: String
  profile: Default | Release | Named(String)
  features: Default | NoDefault | All | Selected(Set<String>)
  cargo_args: Vec<String>
  program_args: Vec<String>
  env: Map<String, EnvValue>
  cwd?: ProjectPath
  toolchain?: ToolchainSelector
  pre_launch_task?: TaskRef
```

`EnvValue` must distinguish literal shareable value from user/environment reference. Project serialization should not encourage literals for secrets.

### Resolution pipeline

```text
CargoExecutionSpec
  -> validate against RustProjectModel
  -> resolve host/path/toolchain
  -> produce ResolvedCargoInvocation
      -> existing Zed Task for Run/Test/Bench/etc.
      -> existing debugger build/launch scenario for Debug
      -> future coverage/profile collector wrapper
```

The task/debug systems remain owners of process lifecycle, cancellation, terminal output and DAP sessions.

### Storage

Support three lifetimes:

- **Ephemeral:** generated from gutter/runnable/current target; session only.
- **User:** local configuration, never committed.
- **Project:** explicit shared file/schema under `.zed/` after product decision.

Migration is additive. Existing `.zed/tasks.json` and `.zed/debug.json` remain valid and are not auto-rewritten.

### UI

The editor/runnable menu can show `Run with Preset…` / `Debug with Preset…`; a compact editor/modal edits structured fields. Advanced/raw Cargo args remain available. Always show a resolved-command preview on request.

### Remote and trust

Resolution occurs against remote/project-side model. The generated task/debug build is executed on project host through existing abstractions. Project presets are inert data until the user invokes an execution; invocation obeys trust.

## 6. Foundation C — Generic structured execution results

### Why generic

Test hierarchy/status/duration/output/navigation are not Rust concepts. Implementing them in a Rust panel would force Python/JS/Go/etc. to reinvent the same model. Rust should provide the first adapter.

### Event model

```text
ExecutionRun
  run_id
  provider_id
  configuration_ref
  started_at
  state
  roots: Vec<ResultNodeId>

ResultNode
  id
  parent_id?
  kind: Suite | Case | Benchmark | Other
  label
  location?: SourceLocation
  state: Discovered | Queued | Running | Passed | Failed | Skipped | Cancelled | Unknown
  duration?
  failure?: FailureSummary
  output_ref?: OutputRef
  children

ExecutionEvent
  NodeDiscovered
  NodeStarted
  OutputChunk
  NodeFinished
  RunFinished
```

Providers must tolerate out-of-order/late events and unknown nodes where tool output is incomplete.

### Output strategy

Do not duplicate unlimited terminal output in UI state. Store bounded snippets + references/ranges to task terminal or artifact. A failed test can show the relevant failure excerpt while “Open full output” jumps to the terminal/result artifact.

### Rust provider discovery

Do not lock the design to a parser yet. Conduct the protocol spike described in `tasks.md` across current stable Rust and cargo-nextest. Candidate sources include language-server runnable metadata, test listing modes and nextest machine-readable interfaces. Prefer stable identifiers/source locations. If no stable structured protocol exists for a mode, degrade to task execution rather than pretend to have exact per-case results.

### Execution

Provider creates an R2 `CargoExecutionSpec` or an equivalent existing task. It subscribes to structured tool events if available and forwards lifecycle/cancel to the task runner. Debug uses the same selected test config but routes through existing DAP.

## 7. Feature 3 UI — Structured test explorer

Recommended shape: generic Tests panel/result view, not “Rust Tests”.

```text
Tests                          Run All
  app
    integration auth
      ✓ login_success      34 ms
      ✗ expired_token      12 ms
  core
    unit
      ✓ parse_header        2 ms

Failure
  expired_token
  expected 401, got 200
  [Open source] [Open full output] [Debug]
```

Primary controls: Run, Debug, Rerun, Rerun Failed, filter, Cancel. Result navigation works by keyboard; status is conveyed with icon + accessible text, not colour alone.

### Staleness

A completed run is marked stale if execution-affecting configuration changes. Ordinary edits may mark source-location accuracy stale depending on anchoring; do not erase results immediately.

## 8. Foundation D — Generic analysis overlays (Next)

Do not implement in Now unless Feature 3 needs a reusable line-decoration primitive already missing. For coverage, define a generic source annotation model:

```text
AnalysisRun -> FileAnalysis -> LineAnnotation/BranchAnnotation + summaries
```

Rust `cargo llvm-cov` becomes a collector adapter, not the owner of editor rendering. Collectors execute remotely/project-side; compact parsed results render locally. Provide stale-generation tracking and explicit installation guidance.

## 9. Debugger integration design

R2 should generate Cargo-aware debugger scenarios but should not grow new debugger protocol handling. Existing DAP crates own adapter capability discovery, launch/attach and session state.

Any later Rust presets (break on panic, pretty-printer status, disassembly/memory) require:

1. capability check against selected adapter;
2. platform-specific documented behaviour;
3. clear fallback when unsupported;
4. generic debugger UI improvements upstream when language-neutral.

## 10. Call hierarchy design (Next)

Zed issue #14203 indicates a generic LSP `prepareCallHierarchy` gap. Implement standard LSP call hierarchy in generic language/project/editor UI, then Rust receives it through rust-analyzer automatically. Do not add a Rust call-graph engine.

## 11. Coverage design (Next)

MVP:

- explicit `Run with Coverage` for current test/package/workspace via R2;
- collector adapter prefers `cargo llvm-cov` after installation/version check;
- parse a standard report format into generic `AnalysisOverlay`;
- editor gutter/line indicators + summary + uncovered navigation;
- run sets are named/time-stamped and stale-aware;
- no silent install;
- remote collector and report parsing may occur remote or local depending transfer size; transfer normalized compact result.

Merged run support comes after single-run correctness.

## 12. Cross-platform matrix

| Area | macOS | Linux | Windows local | SSH remote | Dev container |
|---|---|---|---|---|---|
| Rust project model | required | required | required | required on supported remote hosts | required |
| Cargo presets/tasks | required | required | required | required | required |
| Structured tests | required | required | required | required | required |
| CodeLLDB/GDB specifics | adapter-dependent | adapter-dependent | adapter-dependent | adapter/host-dependent | adapter/container-dependent |
| Coverage Next | `cargo llvm-cov` if installed | same | same | collector on remote | collector in container |
| Profiling Later | platform tool | platform tool | platform tool | explicit limited support | explicit limited support |

Zed documentation currently says Windows is not supported as an SSH **remote server**, though Windows can be the local machine and WSL is supported. Do not make a new Rust feature promise broader than the host platform supports.

## 13. Performance measurement plan

Fixture + Zed repository measurements:

- Zed startup delta with Rust feature enabled but dashboard closed/open.
- time to first project model ready.
- refresh latency after single manifest edit and burst of 20 edits.
- maximum simultaneous Cargo/metadata processes: target 1 per worktree refresh generation.
- task execution overhead relative to baseline task.
- resident memory per 100 packages / 1,000 targets.
- generic test result memory for 1k, 10k, 100k cases with bounded output.
- remote serialized bytes for initial model, refresh and large test run.
- GPUI frame/main-thread blocking: no synchronous metadata parsing/process waits on UI thread.

## 14. Accessibility and keyboard

- Every dashboard/test action available from command palette/keybinding.
- Tree controls implement focus, expand/collapse and selection semantics consistent with existing Zed trees.
- Status never depends only on colour.
- Failure/output text is selectable and screen-reader labelled.
- Focus should return predictably after run/cancel/navigation.

## 15. Settings/configuration

Keep global settings small. Prefer project-derived defaults and presets. Candidate settings only after validation:

- Cargo model auto-refresh on/off (default on for trusted projects, debounced).
- test provider preference (`auto`, Cargo/default, nextest if installed).
- result output retention cap.

Do not expose low-level implementation tuning until telemetry/local profiling demonstrates need.

## 16. Telemetry

No new telemetry by default. Use existing local diagnostics/performance logging during development. If product later needs adoption/performance telemetry, make a separate privacy/product decision.

## 17. Dependencies and licensing

- Reuse existing `cargo_metadata` and `cargo_toml` dependencies where suitable.
- rust-analyzer remains external/managed under existing Zed integration.
- Optional cargo-nextest and cargo-llvm-cov are user-installed external tools; verify licenses/versions at implementation time and do not bundle by default.
- Avoid platform profiler bundling.

## 18. Migration/backwards compatibility

- Existing task/debug configuration remains authoritative and functional.
- New presets are additive and can coexist with manual task/debug entries.
- No automatic rewrite of user JSON.
- If preset schema evolves, version it and provide tolerant reads/migrations.

## 19. Rollout

1. Land internal Rust project model behind a feature flag with tests, no UI.
2. Add read-only dashboard + existing-task actions.
3. Land Cargo execution spec/resolution behind flag.
4. Add ephemeral preset UI, then persistence.
5. Land generic execution-result model + test provider protocol spike.
6. Add test explorer behind flag; dogfood on Zed workspace.
7. Remove flags only after cross-platform/remote/security exit criteria.

## 20. Alternatives rejected

- **Rust-only process runner:** duplicates tasks, cancellation, terminal and remote execution.
- **Rust-only debugger launcher:** duplicates DAP/debugger scenario infrastructure.
- **Parse every Cargo/rustc/test human output:** brittle and localization/version sensitive; use structured protocols where verified.
- **Continuous Cargo discovery on edit:** unacceptable process/performance model.
- **IntelliJ-style all-purpose Rust tool window clone:** conflicts with Zed's progressive-disclosure design and proprietary-copy boundary.
- **Extension-only assumption:** UI-heavy capabilities may exceed current extension API; verify, but architecture should not contort around an unavailable surface.
