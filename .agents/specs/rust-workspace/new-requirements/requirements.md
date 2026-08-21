# Zed Rust Development Environment — Requirements

**Status:** research/specification gate  
**Date:** 2026-08-13

## 1. Ranked user problems

1. **Cargo project state is executable but not sufficiently visible.** Rust developers can invoke runnables/tasks, but package/target/profile/features/toolchain/metadata health are not presented as one coherent worktree-scoped model.
2. **Run/test/debug configuration is fragmented.** Tasks and debugger configurations are powerful but require separate JSON/manual setup; Rust developers repeatedly encode the same package/target/features/profile/env/args state.
3. **Test execution lacks a verified first-class structured result workflow.** Runnables launch work, but developers need suites/cases/status/duration/output/rerun-failed/navigation without replacing Zed's task runner.
4. **Useful generic LSP affordances remain underexposed.** Call hierarchy is a known generic LSP gap; grouped usages/refactor preview should be solved once for multiple languages.
5. **Coverage is external rather than source-attached.** Rust has mature collectors, but Zed needs a generic overlay/result model before a Rust-specific collector is worthwhile.

## 2. Roadmap classification

### Now

- R1 — Worktree-scoped Rust project model and Cargo dashboard.
- R2 — Unified Cargo execution/configuration presets.
- R3 — Structured Rust test explorer backed by generic execution-result infrastructure.

### Next

- Generic LSP call hierarchy + richer usages.
- Generic analysis overlays + Rust coverage integration.
- Cargo dependency and feature provenance insight.
- Rust-aware debug launch/attach discoverability.
- Command-first project/crate creation.

### Later

- Native profiler/flamegraph/call tree.
- Advanced refactoring UI beyond server-provided actions.
- Macro expansion history/macro-authoring enhancements.
- Framework-specific semantic tooling.

### External

- Platform profilers for MVP.
- audit/deny/outdated/unused dependency tools.
- evcxr scratch/REPL.

### Reject

- General database/HTTP/enterprise IDE breadth in the Rust roadmap.
- Duplicate Rust semantic indexing.
- Hosted-service dependency.

## 3. Global non-functional requirements

### Performance

- No Cargo process may be launched on every keystroke.
- Expensive work must be asynchronous, cancellable and off the GPUI/UI thread.
- Metadata refresh must coalesce file-system changes and use a bounded debounce.
- Reuse rust-analyzer/project information where authoritative; do not run duplicate indexing.
- Large-workspace validation must include CPU, memory, process count, first-load time and incremental-refresh latency.

### Security

- Any command capable of executing project-controlled code requires worktree trust.
- This includes Cargo commands, build scripts, proc macros, tests, benchmarks, coverage and profiling tools.
- Project-shared presets must never silently include user-local secrets.
- Dependency network access/mutation requires explicit user action.

### Remote model

- Cargo metadata, tasks, tests, debug builds and tool collectors execute where the project executes: local host, SSH server, WSL or dev container.
- UI/result presentation remains local.
- State exchanged over the remote protocol must be serializable and bounded.
- Windows remote-server degradation follows Zed's current remote support; Windows local client/WSL remain supported.

### Platform

- macOS, Linux and Windows local operation required.
- Tool-dependent features must expose capability/availability rather than fail silently.
- No platform profiler is bundled in the initial roadmap.

### Local-first

- All features operate without a hosted service.
- Optional registry/version information is fetched only on explicit action unless Zed already has a user-approved cached source.

## 4. R1 — Rust Project Model + Cargo Dashboard

### Problem statement

Zed has strong Rust semantics and flexible tasks, but developers lack one authoritative, discoverable worktree representation of Cargo members, targets, profiles, active features/target/toolchain and metadata errors. The result is repeated terminal usage and settings editing for common project-level operations.

### Goals

- Represent Cargo workspace/package/target structure once per worktree.
- Surface metadata health and active Rust configuration.
- Provide quick actions that resolve through existing task infrastructure.
- Refresh incrementally on relevant manifest/configuration changes.
- Work identically through local/remote project abstractions.

### Non-goals

- Replace rust-analyzer's project model or semantic database.
- Build a Cargo package manager UI.
- Run Cargo continuously.
- Automatically modify dependencies/toolchains/features.
- Copy RustRover's Cargo tool-window layout.

### Functional requirements

- R1.1 Discover workspace root(s), packages and members.
- R1.2 Represent lib/bin/example/test/bench/build-script targets.
- R1.3 Represent current toolchain, target triple and Cargo profiles when discoverable.
- R1.4 Represent declared feature graph and effective selected feature set.
- R1.5 Show Cargo metadata/load error with actionable retry/open-config actions.
- R1.6 Expose check/build/run/test/bench/doc/clippy/fmt/clean/update/tree actions based on context.
- R1.7 Resolve actions through Zed Tasks; no second process runner.
- R1.8 Invalidate on Cargo.toml/Cargo.lock/.cargo/config relevant changes; coalesce updates.
- R1.9 Preserve last-good model while refresh is in flight or fails, clearly marking it stale.
- R1.10 Provide keyboard-accessible command-palette actions independent of the visual dashboard.

### Acceptance criteria

- Opening the fixture shows all members and target kinds correctly.
- A manifest edit triggers one coalesced refresh, not repeated Cargo storms.
- Failed metadata preserves prior data as stale and exposes the error.
- Selecting a target and invoking Run produces the same existing task execution path as an equivalent runnable/task.
- Local, SSH and dev-container fixture runs resolve paths/processes on the project host.
- Untrusted worktree does not execute project-controlled Cargo work without trust.

## 5. R2 — Unified Cargo Execution/Configuration Presets

### Problem statement

A Rust launch state (package, target, profile, features, args, env, cwd, toolchain) is duplicated across tasks and debug JSON. Coverage/profile/test will multiply that duplication unless Zed has a reusable Cargo configuration model.

### Goals

- One typed Rust execution description reusable by Run, Test, Debug and later Coverage/Profile.
- Resolve into existing task and debugger systems.
- Support ephemeral, user-local and project-shared presets.
- Separate shareable fields from secrets.

### Non-goals

- Replace `tasks.json` or `debug.json`.
- Introduce a general IDE run-configuration framework before a proven need.
- Hide generated command lines; users must be able to inspect them.

### Functional requirements

- R2.1 Fields: scope/package, target kind/name, target triple, profile, feature mode/set, Cargo args, program/test args, env, cwd, toolchain, pre-launch task.
- R2.2 Validate fields against R1 project model where available.
- R2.3 Generate a preview of resolved Cargo command and execution host.
- R2.4 Resolve Run/Test into tasks and Debug into DAP scenario/build flow.
- R2.5 Provide temporary presets generated from rust-analyzer runnables.
- R2.6 Allow save-as-user and save-as-project with explicit choice.
- R2.7 Project files reject/flag secret-like environment values and support references to user env instead.
- R2.8 Preserve backwards compatibility with existing task/debug configs.
- R2.9 Unknown/custom Cargo flags remain representable via raw additional args.

### Acceptance criteria

- One preset can be run and debugged without duplicating package/features/profile values.
- Generated task/debug behaviour matches equivalent manually written existing configuration.
- User secret environment values are not serialized into shared project config by default.
- Remote presets execute entirely on the remote/container host except UI.
- Editing a preset does not start Cargo until an explicit run/discovery operation requires it.

## 6. R3 — Structured Rust Test Explorer + Generic Results

### Problem statement

The terminal is excellent for raw output but weak for repeated test triage. Rust developers need discoverable test entities, statuses, durations, failures, rerun-failed and source navigation while preserving Zed's task/debug execution architecture.

### Goals

- A generic execution-result model for suite/case/result events.
- Rust provider for unit/integration/doctest and optional nextest workflows.
- Run/debug/rerun/rerun-failed/filter/cancel.
- Structured output without uncontrolled background compilation.

### Non-goals

- Invent a brittle parser for human test output without a stability study.
- Continuously compile to discover tests.
- Replace Terminal output; raw command/output remains accessible.
- Require cargo-nextest.

### Functional requirements

- R3.1 Generic result entity: id, parent, label, source location, status, duration, stdout/stderr references, failure, timestamps.
- R3.2 Lifecycle events: discovered, started, output, passed, failed, skipped/ignored, cancelled, finished.
- R3.3 Rust hierarchy: workspace/package/target/module/test/doctest where evidence supports stable discovery.
- R3.4 Provider abstraction separates discovery from execution/result ingestion.
- R3.5 Execution resolves through Tasks/R2 preset; debug resolves through DAP.
- R3.6 Rerun failed creates an explicit test filter/configuration rather than silently mutating global state.
- R3.7 Bounded output retention with access to full terminal/artifact where applicable.
- R3.8 Cancel propagates to underlying task/process.
- R3.9 Result sets are marked stale when source/configuration relevant to them changes.
- R3.10 Optional cargo-nextest provider only after a machine-readable/stable protocol is validated.
- R3.11 No test discovery process on each edit.

### Acceptance criteria

- Fixture unit/integration/doctests map to correct source locations where the selected provider can supply them.
- Failing test exposes failure, duration, output and source navigation.
- Rerun-failed executes only failed eligible tests.
- Cancellation stops task and updates unresolved cases to cancelled/unknown appropriately.
- Raw terminal command/output remains reachable.
- A provider-version incompatibility degrades to task/terminal execution rather than misreporting results.

## 7. Human product decisions required before implementation

- Whether the Cargo dashboard is a dedicated lightweight panel, a Project-panel mode, or command/modal-first UI. Architecture should support all three, but product should choose one.
- Which preset fields are project-shareable by default and what schema filename/location is preferred.
- Whether test result state persists across Zed restarts or remains session-only for v1.
- Whether the first coverage UI should be gutter-only + summary or include a dedicated generic analysis panel.
