# Design: Structured execution results

## Current implementation baseline

`project::structured_execution` owns the generic state machine, paging and bounded protocol conversion. The task layer exposes structured lifecycle handles while terminals remain canonical output. `tasks_ui::test_explorer` projects all providers through the generic language-tool tree and delegates provider-specific actions without importing Cargo types in its generic core.

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

A synthetic provider shall separately benchmark snapshot application, event reduction, pagination and tree flattening at 10,000 nodes. Accepted wall-time and retained-memory budgets are checked in only after review. GPUI tests drive timers through the executor and assert visible-range rendering.

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

Only D6 remains. Existing generic models, protocol, task bridge and Tests panel are not reopened.
