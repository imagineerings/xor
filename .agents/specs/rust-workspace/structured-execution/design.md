# Design: Structured execution results

## Current implementation baseline

`project::structured_execution` owns the generic state machine, paging and bounded protocol conversion. The task layer exposes structured lifecycle handles while terminals remain canonical output. `tasks_ui::test_explorer` projects all providers through the generic language-tool tree and delegates provider-specific actions without importing Cargo types in its generic core.

<!-- impl: crates/project/benches/structured_execution.rs#structured_execution_benchmark -->
<!-- impl: crates/tasks_ui/src/test_explorer.rs#structured_execution_foreground_budget -->

## Design decisions

### D1: Keep the contract ecosystem-neutral and internal

Nodes carry opaque provider IDs, hierarchy, generic kinds/state and optional visible paths. Providers interpret their identities and actions. No Cargo/Rust types or dynamic public registry are introduced.

### D2: Reduce events monotonically under explicit bounds

Every snapshot/event is keyed by project, provider, discovery generation and run. Duplicates are idempotent; late or mismatched events fail; current and last-complete runs are separate. Protocol pages, events, strings, diagnostics and nodes are bounded and visibly truncated.

### D3: Observe Tasks rather than parse terminals

The task scheduler produces typed lifecycle updates and retains terminal handles. Providers translate lifecycle into generic result events. Terminal content is neither serialized nor parsed for pass/fail.

### D4: Reuse the generic tree host for the Tests panel

The panel owns only generic filters, projection, selection/expansion, navigation and delegate actions. Provider states map one-to-one to explicit UI states. Workspace persistence stores filters and opaque IDs, not results or secrets.

### D5: Preserve host authority and privacy

The authoritative project store reduces provider results. Remote peers request bounded pages/events filtered to visible worktrees. Disconnect/generation changes invalidate in-flight work; no client-local execution exists. Protocol definitions may remain inert when the feature is disabled.

### D6: Add measured budgets at the existing limit

A deterministic synthetic provider separately benchmarks snapshot application, event reduction and the real bounded protocol-page conversion at 10,000 nodes. The gate allows two seconds for discovery, two seconds for event reduction, 100 ms for complete pagination and 64 MiB for conservatively modeled retained state. The accepted macOS arm64 result was 2 ms discovery, 3 ms event reduction, 65 ms pagination and approximately 5.4 MiB modeled retained state. Provider-node membership and latest result states are indexed so event reduction remains incremental rather than quadratic. Run metadata is serialized on the first snapshot page and continuation pages carry only their bounded node slices, matching the remote client assembly path without repeatedly cloning the same run scope.

The Tests-panel GPUI gate projects provider results on the background executor, yields with a GPUI executor timer, then requires foreground tree reconciliation and a 25-row visible-range projection for exactly 10,000 nodes to complete within 250 ms. The accepted local result was 37 ms.

## Cross-pack dependencies

- `rust-test-explorer` is the first provider and owns all Rust/Cargo interpretation.
- `rust-coverage` may reuse result summaries only where execution lifecycle is useful; source annotations remain a separate generic contract.
- `rust-tools-platform` owns optional registration and host capability parity.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2 | D1 | Existing model/source dependency tests |
| 1.3, 1.4 | D2 | Existing generation/idempotency/retention/paging tests |
| 1.5 | D3 | Existing structured task lifecycle tests |
| 1.6, 1.7 | D4 | Existing generic Tests panel projection/GPUI tests |
| 1.8 | D1, D4 | Existing non-Rust fake provider and dependency audit |
| 2.1, 2.2, 2.3 | D6 | New benchmark and GPUI foreground-budget gate |

## Remaining delta

D6 is implemented without changing the 10,000-node protocol limit. No structured-execution behavior remains open in this pack.
