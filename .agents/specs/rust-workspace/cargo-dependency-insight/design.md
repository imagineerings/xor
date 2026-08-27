# Design: Cargo dependency insight

## Current implementation baseline

`cargo_workspace` converts bounded direct dependency declarations and now enriches them with visible manifest origin, bounded resolved instances/features, lock status, and workspace-cycle observations. `CargoPanel` keeps the dashboard direct-only while rendering a finite selected-row detail projection. It does not retain or render a recursive transitive graph.

## Design decisions

### D1: Extend Cargo evidence, not the generic tree contract

Add Cargo-specific provenance types behind `project/cargo-workspace`. Keep the existing direct dependency row identity. A selected row may request/detail its bounded resolved instance(s); generic `language_tools` remains unchanged.

### D2: Combine only validated local/metadata/lock facts

Cargo metadata remains resolved authority. Visible manifests may be parsed only to identify declaration spans and `workspace = true`; Cargo.lock may be parsed for locked identity when present. Each fact carries origin/completeness, and ambiguous feature causality stays unknown. No registry query occurs.

### D3: Keep presentation package-centric and finite

`cargo_ui` shows a detail subtree/popover for one selected direct declaration: Declared, Resolved, Features and Source/Lock facts. Multiple resolved versions are siblings. There are no recursive dependency children; cycle/truncation markers are informational.

### D4: Preserve authoritative-host privacy and navigation

The host converts all paths to visible `ProjectPath`; registry/Git cache paths are discarded. Remote peers receive bounded facts filtered to visible worktrees. Trust revocation, invalidation and stale generations follow CargoWorkspaceStore. Refresh never executes an update/fetch.

### D5: Test deterministic provenance and scale

Fixtures combine metadata, visible manifests and lock captures for every declaration/source case, ambiguity, cycles, missing/stale input and multi-version resolution. Large synthetic graphs assert bounded conversion/projection without recursive rendering.

## Cross-pack dependencies

- Requires the completed `cargo-dashboard/1.1` model/store/panel baseline.
- Uses `rust-tools-platform` host/trust/protocol boundary.
- Does not depend on `structured-execution`, coverage or profiling.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.3 | D1, D2 | Provenance conversion fixtures |
| 1.2 | D3 | Finite projection/UI tests |
| 1.4 | D3, D4 | Safe navigation/privacy tests |
| 1.5, 1.6 | D2, D5 | Partial/ambiguity/bounds/cycle tests |
| 1.7 | D2, D4 | Fake runner remote/trust/no-network tests |
| 1.8 | D5 | Deterministic comprehensive graph suite |

## Performance and persistence

Details are derived from the current Cargo snapshot and are not persisted separately. Selected direct node identity may persist through existing panel state; provenance is invalidated with its generation. Parsing occurs off the GPUI foreground thread.
