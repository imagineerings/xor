# Design: Rust test explorer

## Current implementation baseline

`project::rust_test_provider` owns bounded Cargo JSON/harness capture parsing, stable Rust node/action identities, authoritative local/remote store modes, request cancellation and typed action/lifecycle messages. It publishes generic snapshots/events into `StructuredExecutionStore`. `tasks_ui::test_explorer` delegates Rust Run/Debug/Cancel/Rerun/terminal/navigation actions when `rust-test-actions` is selected.

## Design decisions

### D1: Keep discovery separate from Cargo metadata and semantics

Cargo workspace data supplies roots/targets; optional rust-analyzer hints enrich source/runnable facts. The Rust provider owns test enumeration. `CargoWorkspaceStore` remains metadata/configuration-only, and no Rust source index is added.

### D2: Use a bounded validated host protocol

The production runner builds harnesses with offline structured Cargo messages, enumerates through a validated stable list form, and converts unknown lines to partial diagnostics. Time, bytes, lines, cases, fields and diagnostics are bounded. Captured fixtures cover all supported harness categories.

### D3: Compile actions into existing execution systems

Run creates a structured Cargo `TaskTemplate`; Debug creates a Cargo-locator `DebugScenario`. Typed task lifecycle updates generic results. Aggregate-only protocols never synthesize child outcomes. Doctest debug remains unavailable with an action-specific reason.

### D4: Preserve cancellation, stale state and privacy

Discovery and action requests carry protocol/discovery generations and visible worktree scope. Peer-scoped cancellation cannot affect another peer. Late generations, unauthorized runs and outside-scope paths fail. Raw output/environment/absolute host paths never cross the protocol.

### D5: Use the authoritative project environment in every mode

Local mode uses the existing project environment and trust. Remote/headless mode owns discovery and action planning; clients schedule via established remote Tasks/DAP and report bounded lifecycle. Multiplayer guests obey existing execution permissions. No local mirror path exists.

### D6: Add physical compatibility certification without provider forks

The remaining matrix uses the production transport/project environment on supported OS/SSH/WSL/dev-container paths, small hermetic Rust fixtures, preinstalled stable tools, offline execution and fake credentials. It records supported/unsupported capability per cell and tests disconnection and host identity. It adds no environment-specific runner.

## Cross-pack dependencies

- Completed baselines: `cargo-dashboard/1.1`, `cargo-execution/1.1`, `structured-execution/1.1`, and `rust-tools-platform/1.1`.
- The physical matrix leaf is consumed by the `rust-tools-platform` final certification leaf.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2, 1.3 | D1, D2 | Existing provider projection/injected-runner tests |
| 1.4 | D2 | Captured stable protocol fixture matrix |
| 1.5, 1.6, 1.7 | D3 | Existing action plan/lifecycle/DAP tests |
| 1.8, 1.9 | D3, D4 | Existing cancel/stale/rerun tests |
| 1.10 | D4, D5 | Existing protocol/privacy/visible-worktree tests |
| 1.11 | D2 | Dependency/source audit and offline fixture gate |
| 2.1, 2.2, 2.3 | D5, D6 | New physical compatibility matrix |

## Remaining delta

The hermetic physical-workspace fixture, matrix coordinator, local CI invocation and exact production-mode checklist now exist. D6 remains uncertified for actual SSH/headless, WSL, development-container and multiplayer transports. Checkout-only or fake-headless results remain useful but do not constitute that physical certification; the dated evidence requirements are recorded in `physical-matrix-evidence.md`.
