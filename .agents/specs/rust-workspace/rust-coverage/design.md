# Design: Rust coverage over generic source analysis

## Current implementation baseline

No generic coverage model, source-analysis overlay, or Rust collector is present. Existing seams are editor gutter decorations/navigation, project-visible paths, structured task lifecycle, Cargo preset compilation, remote project ownership, and workspace panels/toolbar components.

## Design decisions

### D1: Add only a generic coverage annotation contract

Define bounded run/file/range annotations and aggregate summaries in a generic project-owned module. It models coverage state, not a universal analysis/provider plugin API. Paths are visible `ProjectPath` values; node/provider identities are opaque.

### D2: Render with generic editor annotations and a compact summary

The editor consumes annotations through a generic source-analysis seam and exposes accessible covered/uncovered markers. A small language-tools summary filters/navigates files. No Rust logic enters editor rendering and no dedicated analysis panel is required initially.

### D3: Split Cargo action planning from host artifact interpretation

`cargo_ui` resolves current Cargo scope/configuration through `cargo-execution` and creates an ordinary explicit Task for a detected supported collector with a known project-relative artifact path. A separate feature-gated `project::rust_coverage_provider` on the authoritative host validates/parses that artifact after typed task completion and publishes generic coverage facts. `CargoWorkspaceStore` remains untouched.

### D4: Validate artifacts under strict host/privacy bounds

The collector adapter accepts only supported schema/version, maps paths through visible worktrees, enforces bytes/files/ranges, and reports partial/truncated input. Remote peers receive converted facts, not raw report files. Environment values stay in Tasks only.

### D5: Degrade without installation or network

Missing collector, unsupported platform/version, restricted worktree, guest denial, disconnect and mismatch are explicit action/UI states. The product never installs the collector or fetches dependencies.

## Persistence and cancellation

Coverage is session-only in the first milestone. A new run supersedes the same provider/scope generation; cancellation stops the owned Task when possible and rejects late artifacts. Closing/reopening a project does not claim stale files remain covered.

## Cross-pack dependencies

- Requires completed `cargo-execution/1.1` baseline and any approved preset schema migration.
- May use `structured-execution` task lifecycle, but coverage annotations remain a distinct generic owner.
- `rust-tools-platform` gates the Cargo action, Rust artifact provider and authoritative remote/headless handlers.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.4, 1.5 | D1, D4 | Generic model bounds/dependency tests |
| 1.2 | D2 | Editor/summary GPUI and accessibility tests |
| 1.3 | D2, D5 | State rendering tests |
| 2.1, 2.2 | D3 | Pure task plan and fake lifecycle/artifact tests |
| 2.3 | D5 | Missing-tool/no-network source and runner tests |
| 2.4 | D4, D5 | Remote protocol/privacy/mismatch tests |
| 2.5 | D1, D2, D3, D4 | Deterministic cross-provider acceptance suite |

## Performance test seams

Synthetic reports cover maximum files/ranges and adversarial path/size input. Parsing occurs in the background; GPUI receives bounded deltas. Timed tests use GPUI executor timers.
