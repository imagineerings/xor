# Zed Rust Development Environment — Implementation Tasks

**Gate:** This file sequences implementation after research approval. No product implementation was performed during research.

Each milestone must be independently reviewable and include tests/benchmarks with the code that introduces behaviour.

## Milestone 0 — Pin baseline and close evidence gaps

### M0.1 Pin implementation baseline

- Record current Zed commit SHA, stable version, OS/toolchains and rust-analyzer version used by the developer environment.
- Re-run source/issue/PR search for all proposals.
- Diff architecture against research SHA `0307288d...`.

**Exit:** `research.md` version table updated; no architecture reference is stale/guessed.

### M0.2 Trace exact Zed types and actions

Search current checkout for project/worktree state, task inventory/resolution/spawn, DAP scenario/configuration, worktree trust, remote serialization and editor panel/gutter APIs.

Suggested commands (adapt to current repo tooling):

```bash
rg -n "cargo_metadata|cargo_toml" crates
rg -n "TaskTemplate|ResolvedTask|TaskInventory|spawn.*task" crates
rg -n "DebugScenario|debug.json|DAP|DebugAdapter" crates
rg -n "trust.*worktree|Restricted Mode|is_trusted" crates
rg -n "remote.*project|dev_container" crates/project crates/remote* crates/dev_container
rg -n "gutter|annotation|diagnostic" crates/editor crates/ui
```

Write exact findings into `design.md` before implementation.

**Exit:** every proposed new type has an identified owner crate and existing integration point.

### M0.3 Build evaluation fixture

Create a disposable fixture satisfying `research.md` section 3. Add a generator script only if repository conventions favor it.

Validation:

```bash
cargo metadata --format-version 1 --no-deps
cargo check --workspace
cargo test --workspace
cargo test --workspace --doc
cargo bench --no-run
```

One test/diagnostic is expected to fail in controlled commands; document expected invocation.

**Exit:** fixture exercises all target kinds/features/config transitions needed by Now features.

### M0.4 Structured-test protocol spike — blocker for Milestone 4

On current stable Rust plus the minimum supported toolchain and current cargo-nextest:

- enumerate available machine-readable discovery/result modes;
- record schema/version stability, source-location quality and cancellation behaviour;
- test unit/integration/doctest/ignored/generated names;
- test large output and non-UTF8/error cases;
- decide provider contracts and fallback.

**Do not** ship a human-output parser based only on a single observed version.

**Exit:** short ADR selects supported provider(s) or explicitly limits v1 structured coverage.

**PR boundary:** research-only fixture/ADR if repository accepts permanent test fixture; otherwise no production PR.

---

## Milestone 1 — Worktree Rust project model (no UI)

### M1.1 Add typed project summaries

Implement minimal data types for workspace/package/target/features/config/status using current naming conventions. Stable IDs; serde/remote support only where required.

Tests:

- package/target identity stability;
- target kinds including custom-build/proc-macro/bench/example;
- feature graph representation;
- malformed/unknown future Cargo fields degrade safely.

**Exit:** model can represent fixture metadata without UI.

### M1.2 Add project-side metadata loader

Use existing command/process abstraction. Prefer current project/rust-analyzer metadata source if equivalent; otherwise invoke Cargo metadata with explicit cancellation and bounded stderr capture.

Tests:

- success, Cargo missing, invalid manifest, cancelled process;
- no UI-thread blocking;
- process executes in expected worktree cwd.

**Exit:** one explicit load returns typed state/error.

### M1.3 Integrate trust gate

- No trust-sensitive Cargo spawn in restricted worktree.
- Expose restricted/degraded state to caller.
- Verify local + remote trust host semantics.

Tests should use existing trust test harness.

**Exit:** malicious/untrusted fixture cannot cause Cargo/project code execution through model load.

### M1.4 Add cache/generation/invalidation

Watch only relevant manifests/config/toolchain files. Debounce/coalesce; generation-check results; preserve last-good state as stale.

Tests with paused/fake clock where possible:

- burst changes -> one refresh;
- stale old result cannot overwrite newer generation;
- Cargo.toml refreshes, ordinary `.rs` edit does not trigger metadata process;
- cancellation has no error toast/state corruption.

**Exit:** bounded refresh behaviour proven by tests.

### M1.5 Remote serialization/ownership

Ensure acquisition occurs on project host and only compact state crosses remote boundary. Add protocol changes using Zed's normal compatibility conventions.

Tests:

- local/remote state equivalence;
- reconnect/resync;
- old peer compatibility according to repo policy.

**Exit:** model works without local-path leakage in SSH/dev-container sessions.

### M1.6 Benchmark large workspace

Benchmark initial parse/load, refresh and memory on generated large fixture/Zed repo.

**Exit:** documented budgets accepted; no main-thread blocking; one metadata refresh process per worktree generation.

**Proposed PR:** `rust: add worktree-scoped Cargo project model`.

---

## Milestone 2 — Cargo dashboard + task actions

### M2.1 Read-only dashboard tree

Use existing panel/tree components. Show workspace/package/targets and loading/stale/error states. Keep default presentation compact.

UI tests:

- loading/ready/stale/error/restricted states;
- keyboard focus/expand/collapse;
- no colour-only status.

**Exit:** fixture structure is navigable without executing a command.

### M2.2 Active configuration summary

Display toolchain/target/profile/features when confidently known. Unknown values must be labelled unknown rather than inferred.

**Exit:** changing supported config invalidates/reloads once and updates summary.

### M2.3 Generate contextual Cargo actions through Tasks

Map selected target/package/workspace to task templates for check/build/run/test/bench/doc/clippy/fmt/clean/tree/update. Use task subsystem for process/terminal/cancel.

Tests:

- correct args/package/target flags;
- path quoting;
- unavailable action hidden/disabled appropriately;
- task cancellation remains owned by task runner.

**Exit:** no Cargo process spawning code exists in dashboard UI.

### M2.4 Error/recovery UX

Show compact metadata failure, details, retry and open relevant manifest/config.

**Exit:** failed refresh keeps stale tree usable and never presents stale data as current.

**Proposed PR:** `rust: add Cargo project dashboard backed by project model`.

---

## Milestone 3 — Unified Cargo execution/configuration presets

### M3.1 Implement typed `CargoExecutionSpec` equivalent

Fields: scope/package/target/action/target triple/profile/features/Cargo args/program args/env/cwd/toolchain/pre-launch task.

Tests for serialization, stable IDs and future/unknown field handling.

**Exit:** fixture run/test/debug states fit without raw shell-string encoding.

### M3.2 Resolve spec to existing tasks

Generate task command/args/env/cwd using argv arrays (not shell concatenation). Integrate remote host paths through existing task variables/project path APIs.

Tests:

- spaces/special characters;
- feature combinations;
- custom profile/target triple;
- program args after `--`;
- remote path handling.

**Exit:** equivalent manually authored task and generated task produce equivalent resolved invocation.

### M3.3 Resolve spec to existing debugger scenario

Use existing Cargo build inference/DAP scenario path. Do not implement adapter protocol in Rust code.

Tests:

- CodeLLDB and GDB supported configurations;
- unsupported adapter capability produces clear validation/fallback;
- build cancellation stops launch.

**Exit:** same preset can Run and Debug.

### M3.4 Add ephemeral preset editor

Create small modal/popover using existing components; prefill from selected Cargo target/rust-analyzer runnable. Include resolved command preview.

UI/accessibility tests.

**Exit:** common run-with-features/args flow needs no JSON editing.

### M3.5 Add user/project persistence

After product chooses schema/location:

- explicit Save for User / Save for Project;
- secret-aware environment value policy;
- tolerant schema versioning;
- project config inert until execution and trust approval.

Tests for no-secret-by-default behaviour.

**Exit:** shared preset contains no accidental user secret and works cross-platform when its fields are portable.

### M3.6 Compatibility tests

Existing `.zed/tasks.json` and `.zed/debug.json` unchanged and continue working.

**Proposed PRs:**
1. `rust: add reusable Cargo execution spec and task resolution`
2. `debugger: resolve Cargo execution specs into DAP launches`
3. `rust: add Cargo preset editor and persistence`

---

## Milestone 4 — Generic structured execution results

**Blocked by M0.4 protocol ADR.**

### M4.1 Generic result/event model

Add suite/case nodes, state, duration, source location, failure and bounded output references. No Rust types in generic model.

Tests for event ordering, duplicate events, parent-after-child, cancellation and very large suites.

**Exit:** synthetic provider can drive 100k-case run within accepted memory/performance budget.

### M4.2 Provider/execution bridge

Connect generic run lifecycle to existing task cancellation/output without taking process ownership away from Tasks.

**Exit:** synthetic provider task can start/cancel/finish with deterministic state.

### M4.3 Generic Tests UI

Tree/filter/failure details/run/rerun/rerun-failed/cancel/navigation. Terminal/full output remains accessible.

UI/accessibility tests with synthetic provider.

**Exit:** language-neutral test UI ships behind feature flag.

**Proposed PR:** `test results: add generic structured execution model and UI`.

---

## Milestone 5 — Rust test provider

### M5.1 Implement discovery adapter selected by ADR

Map stable provider data into workspace/package/target/module/test/doctest hierarchy where available. Never fabricate source locations.

Tests across fixture and supported toolchain matrix.

**Exit:** discovered identities stable across unchanged reruns.

### M5.2 Run tests through Cargo presets/tasks

Current/package/target/case filters resolve through Milestone 3; ingest structured events.

**Exit:** no second process runner.

### M5.3 Debug selected test

Reuse same selection/config with DAP. Validate adapter/build failures.

**Exit:** Debug from test node launches the exact intended test or clearly states provider limitation.

### M5.4 Rerun failed, ignored, doctest and nextest behaviour

Implement only capabilities validated by provider contract. Unsupported modes fall back to task/terminal with a visible explanation.

**Exit:** no silent misclassification.

### M5.5 Remote/dev-container test matrix

Run fixture locally, SSH and container. Verify output/result serialization, cancellation and source navigation.

**Proposed PR:** `rust: add structured test provider`.

---

## Milestone 6 — Harden Now features and graduate flags

- Windows/macOS/Linux local matrix.
- SSH Linux/macOS host matrix; document unsupported Windows remote server behaviour consistent with Zed platform support.
- dev-container matrix.
- restricted/trusted transition tests.
- large Zed workspace performance benchmark.
- cancellation stress tests.
- accessibility keyboard/screen-reader pass.
- docs/update release notes.

**Exit:** all Now acceptance criteria met; no uncontrolled Cargo spawn; flags removed only with maintainer/product approval.

**Proposed PR:** focused hardening/docs PRs, not a single mega-PR.

---

## Next-phase task seeds (not part of Now implementation)

### Generic LSP call hierarchy

- Reconfirm issue #14203/related PR status.
- Implement standard `prepareCallHierarchy`, incoming/outgoing requests in generic LSP core/UI.
- Validate Rust through rust-analyzer plus at least one non-Rust language.

### Coverage

- Add generic analysis-overlay model/UI.
- Spike `cargo llvm-cov` supported output/version behaviour.
- Implement explicit collector adapter through Cargo execution spec.
- Add gutter/summary/uncovered navigation, stale runs and later merging.

### Cargo dependency insight

- Reuse project model resolved graph.
- Add declared/resolved/locked distinction and feature provenance first.
- Keep latest-version network fetch and mutation explicit.

### Profiling

- First add task action + artifact opening for external SVG/HTML outputs.
- Revisit native view only after measured demand.
