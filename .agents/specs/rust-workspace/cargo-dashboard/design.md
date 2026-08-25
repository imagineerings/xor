# Design: Cargo dashboard

## Current implementation baseline

`project::cargo_workspace` is a feature-gated, UI-independent Cargo model. `CargoWorkspaceStore` discovers visible manifest candidates and invokes injected or production metadata/configuration runners on the authoritative project host. `cargo_ui::CargoTreeProvider` projects bounded snapshots into `language_tools::language_tool_tree`; `CargoPanel` is a thin dockable wrapper titled `Cargo`.

Evidence includes `crates/project/test_data/cargo_workspace/`, `crates/project/tests/integration/cargo_workspace.rs`, inline store tests, Cargo panel projection/GPUI tests, the 1,000-package panel case, and the generic host's 10,000-row case.

## Design decisions

### D1: Retain the established dependency direction

`language_tools` knows only opaque generic tree nodes and lifecycle state. `project` owns the Cargo model/store and typed bounded protocol conversion. `cargo_ui` depends on both and owns Cargo labels, icons, actions and navigation. No lower layer depends on Cargo UI.

### D2: Keep discovery authoritative, lazy and cancellable

The owning project host discovers visible manifests and uses its existing environment/trust boundary. The panel starts dormant, requests on first activation, debounces relevant invalidations, cancels obsolete work, fingerprints inputs, and rejects old generations. Remote/multiplayer clients receive visible project-relative snapshots only.

### D3: Preserve a finite direct tree

Workspace roots contain members; members contain Targets, Features and direct Dependencies. Dependencies are annotated with exposed kind/source/rename/optional/target/resolution facts but have no recursive children. Stable IDs derive from visible structural keys, never host absolute paths.

### D4: Model only configuration facts that can be stated truthfully

The store parses visible profile/toolchain declarations and performs one bounded `rustc -vV`-style host probe through the project environment. It distinguishes host, explicit preset and unresolved Cargo-default target facts. It does not reproduce Cargo's layered configuration resolution or invoke rustup.

### D5: Add a fixture and benchmark without changing runtime architecture

The remaining fixture composes existing deterministic captures under `crates/project/test_data`; it must not run real Cargo. A repeatable benchmark seam shall invoke pure metadata conversion, store reconciliation and tree projection separately, report time/peak retained allocations using repository-standard tooling, and define accepted budgets after a checked-in baseline is reviewed. GPUI portions use executor timers and visible-range assertions.

### D6: Preserve failure, privacy and persistence behavior

Candidate failures remain scoped; last-good safe data can become stale; malformed roots do not remove valid roots. Panel layout/selection/preset ID persistence remains additive and excludes secrets. Absolute host paths, raw output and environment values remain host-only.

## Cross-pack dependencies

- `rust-tools-platform` owns compile-time selection, host parity, privacy certification and physical environment coverage.
- `cargo-execution` consumes stable Cargo node identities/configuration but does not add execution methods to the store.
- `cargo-dependency-insight` may later consume additional Cargo evidence without changing the direct baseline hierarchy.

## Test seams and limits

- `CargoMetadataRunner` and configuration probes remain injectable.
- Metadata conversion and projection are pure and fixture-driven.
- Current hard limits include bounded diagnostics/configuration fields and 10,000 generic tree rows; the new benchmark establishes budgets rather than increasing these limits.
- No test depends on developer Cargo, rustc, rustup, registry credentials, network, or the repository's real workspace.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1, 1.2, 1.7, 1.8 | D1, D2, D6 | Existing Cargo panel/GPUI/persistence tests |
| 1.3, 1.4, 1.5 | D2, D3 | Existing model/store fixtures and projection tests |
| 1.6 | D3, D6 | Existing safe-navigation tests |
| 1.9 | D1 | Feature-boundary dependency audit |
| 2.1, 2.2, 2.3, 2.4 | D4 | Existing configuration parser/probe fixtures |
| 2.5, 2.6, 2.7 | D2, D4, D6 | Existing panel/preset/stale tests |
| 2.8 | D2, D6 | Protocol/privacy tests |
| 3.1 | D5 | New comprehensive deterministic fixture test |
| 3.2, 3.3 | D5 | New repeatable benchmark and foreground separation gate |

## Remaining delta

Only D5 is unimplemented. No dashboard production behavior is otherwise reopened by this pack.
