# Requirements: Structured execution results

## Purpose and status

This pack owns Zed's internal language-neutral structured execution model/store/protocol, generic task-lifecycle bridge, and generic dockable `Tests` panel. The bounded 10,000-node implementation is verified baseline. It is not a build-system abstraction or public extension API.

Canonical IDs are `structured-execution/<criterion>`.

### Requirement 1: Preserve the generic structured execution baseline [Verified baseline]

#### Acceptance criteria

1. **1.1** THE project layer SHALL represent stable provider/run/node IDs, parent-child relationships, generic provider/suite/group/case kinds, queued/running/passed/failed/skipped/cancelled states, duration, bounded messages, summaries, and optional visible `ProjectPath` navigation.
2. **1.2** THE contract SHALL describe execution trees/events and SHALL NOT contain Cargo packages, targets, features, commands, Rust test types, or a universal build-system model.
3. **1.3** WHEN discovery or run events arrive, THE store SHALL apply them monotonically to the matching project/provider/run generation, ignore duplicates idempotently, reject stale/cross-project input, and preserve last-complete separately from current run.
4. **1.4** WHEN bounds are exceeded, THE store/protocol SHALL truncate or evict deterministically while retaining current run, summary counts and actionable failure locations, and SHALL expose partial/truncated status.
5. **1.5** WHEN a structured task is scheduled, THE bridge SHALL expose queued/running/completed/spawn-error/cancel lifecycle without scraping terminal text and SHALL preserve ordinary task terminal, history and output.
6. **1.6** THE generic `Tests` panel SHALL provide provider/suite/test hierarchy, status/text filters, keyboard/accessibility behavior, summaries, failure navigation, run/cancel/rerun delegation, terminal reveal, and stable participant-local selection.
7. **1.7** WHEN providers are unavailable/loading/empty/partial/stale/error/restricted/disconnected/incompatible, THE panel SHALL show a distinct actionable state and SHALL NOT infer success from absent data.
8. **1.8** A future in-tree non-Rust provider SHALL be able to use the model/store/task bridge/tree projection without `cargo_ui`, `cargo_metadata`, Cargo model or Rust provider dependencies.

### Requirement 2: Certify bounded performance [Required change]

<!-- impl: crates/project/benches/structured_execution.rs#structured_execution_benchmark -->
<!-- impl: crates/tasks_ui/src/test_explorer.rs#structured_execution_foreground_budget -->

#### Acceptance criteria

1. **2.1** THE repository SHALL define repeatable time and memory budgets for discovery application, event reduction, paging and visible-range projection at the supported 10,000-node limit and SHALL gate regressions against an accepted baseline.
2. **2.2** THE supported limit SHALL remain 10,000 nodes unless a separate measured decision changes protocol, memory and UI budgets; the historical unbudgeted 100,000-case target is superseded rather than implied.
3. **2.3** TIMED GPUI tests SHALL use GPUI executor timers and SHALL demonstrate that provider parsing/reduction work does not block the foreground render loop.

## Compatibility and non-goals

Provider-specific discovery/action planning belongs to provider packs. `rust-tools-platform` owns compile-time registration and remote privacy certification. Out of scope: terminal parsing, arbitrary output storage, a general package model, public extensions, persisted raw run history, and raising limits without evidence.

## Open questions

None. The internal-only contract, session-bounded history, 10,000-node limit and generic Tests panel are implemented decisions.
