# Requirements: Rust test explorer

## Purpose and status

This pack owns Rust-specific test discovery, stable Rust test identities/action plans, and their adapter into the generic structured execution/Tests experience. Discovery is separate from `CargoWorkspaceStore` and runs only on the authoritative project host. The built-in stable protocol adapter and actions are verified baseline; physical environment certification remains.

Canonical IDs are `rust-test-explorer/<criterion>`.

### Requirement 1: Preserve Rust test discovery and actions [Verified baseline]

#### Acceptance criteria

1. **1.1** WHEN a trusted supported Cargo project is refreshed, THE provider SHALL project workspace, package, test-bearing target, group/module and case nodes with stable opaque IDs and safe source navigation.
2. **1.2** THE provider SHALL reuse Cargo target facts and available rust-analyzer runnable/source hints and SHALL NOT parse Rust source into a second semantic index.
3. **1.3** WHEN tool execution is needed, THE authoritative host SHALL use a separately injectable, bounded Rust discovery runner and SHALL NOT add run/build/test methods to `CargoWorkspaceStore`.
4. **1.4** THE built-in adapter SHALL validate Cargo JSON plus harness listing against unit, integration, binary, example, benchmark, ignored and doctest fixtures; unknown/malformed records SHALL yield bounded partial discovery rather than panics or fabricated tests.
5. **1.5** WHEN a single test is run, THE provider SHALL compile an exact existing Cargo task and derive status from typed task lifecycle, never ANSI terminal parsing.
6. **1.6** WHEN per-child suite outcomes are unavailable, THE provider SHALL report only the suite aggregate and leave child results unknown/stale.
7. **1.7** WHEN a supported case is debugged, THE provider SHALL compile the existing Cargo DAP scenario/locator; unsupported doctest or harness debug SHALL be disabled with a reason.
8. **1.8** WHEN a run is cancelled or superseded, THE provider SHALL cancel owned discovery/task work where possible, mark lifecycle accurately and reject late generations.
9. **1.9** WHEN rerun-failed is invoked, THE provider SHALL schedule only failed/error nodes still present in current discovery, with bounded concurrency and an explicit removed-test summary.
10. **1.10** THE provider SHALL retain terminal output in the existing terminal, store only bounded structured summaries/messages, filter visible worktrees per peer, and never serialize environment values.
11. **1.11** THE built-in provider SHALL require no downloaded runner or network; `cargo-nextest` remains optional Later and SHALL NOT be automatically installed.

### Requirement 2: Certify physical project environments [Required change]

#### Acceptance criteria

1. **2.1** CI or a documented release matrix SHALL exercise Rust test discovery, run, cancellation and supported debug on local macOS/Linux/Windows clients where supported, an actual SSH/headless Linux host, and an available WSL or dev-container project path without relying only on source-shape tests.
2. **2.2** FOR every physical environment, THE test SHALL prove discovery and execution occur on the authoritative host, disconnect/reconnect rejects stale results, and no client-local Cargo fallback occurs.
3. **2.3** WHEN an environment or DAP adapter is unsupported, THE matrix SHALL record an explicit capability result rather than marking the whole Rust provider complete or silently skipping it.

## Compatibility and non-goals

The generic result model/UI belongs to `structured-execution`; Cargo presets belong to `cargo-execution`; feature mismatch/privacy policy belongs to `rust-tools-platform`. This pack does not own generic test UI, Cargo metadata, terminal parsing, automatic nextest setup, coverage, profiling, or a Rust semantic index.

## Open questions

None. The production adapter, session-bounded history, doctest debug limitation and no-nextest-install policy are implemented decisions. Physical environments are tested where the underlying Zed mode is supported and otherwise recorded as unsupported.
