# Requirements: Rust coverage over generic source analysis

## Purpose and status

This Next pack owns the implemented language-neutral coverage-result/annotation contract and the Rust adapter for an explicitly available `cargo-llvm-cov` collector. Coverage execution uses Tasks on the authoritative host and is not a `CargoWorkspaceStore` responsibility.

Canonical IDs are `rust-coverage/<criterion>`.

### Requirement 1: Provide generic bounded coverage presentation [Implemented / Next]

#### Acceptance criteria

1. **1.1** THE project/UI layers SHALL represent provider ID, run generation, visible project file/range, covered/uncovered/partial state, optional bounded hit count and aggregate summary without Cargo/Rust types.
2. **1.2** THE first UI SHALL show accessible gutter annotations and a compact summary/filter, navigate to visible source, and SHALL NOT require a Rust-specific editor renderer.
3. **1.3** WHEN results are loading, empty, partial, stale, truncated, cancelled, incompatible or failed, THE UI SHALL distinguish those states and SHALL NOT present old coverage as current.
4. **1.4** Coverage reports SHALL be node/file/byte bounded, generation-bound and cancellable; obsolete reports SHALL be rejected and large reports SHALL expose truncation.
5. **1.5** Generic coverage types/rendering SHALL remain internal, language-neutral and independent of `cargo_ui`, `cargo_metadata` and Rust provider types.

### Requirement 2: Add an explicit Rust collector adapter [Implemented / Next]

#### Acceptance criteria

1. **2.1** WHEN the user explicitly invokes Run with Coverage and a supported collector is already available, THE Rust adapter SHALL compile the selected Cargo scope/preset into existing Tasks and parse only a validated machine-readable report artifact.
2. **2.2** THE adapter SHALL execute on the authoritative project host, obey trust/guest/remote policy, use structured argv/env, and SHALL NOT parse terminal text or run Cargo from the UI/store.
3. **2.3** THE feature SHALL NOT install `cargo-llvm-cov`, fetch dependencies, access the network automatically, or mutate manifests; missing/unsupported tools SHALL produce setup guidance.
4. **2.4** Remote/multiplayer protocols SHALL transmit bounded project-relative coverage facts, never raw environment values, absolute host paths or unbounded report files, and SHALL have no client-local fallback.
5. **2.5** Tests SHALL validate unit/integration/ignored/partial reports, path remapping, stale cancellation, remote filtering, malformed/bomb-sized reports, generic non-Rust projection and supported-platform capability differences.

## Non-goals

No dedicated Cargo process runner, automatic tool installation, terminal parser, coverage suite merging, public provider API, profiling, or call-hierarchy coupling.

## Resolved product choices

1. **Initial presentation.** Use gutter annotations plus one compact generic summary/filter projection; defer a dedicated analysis panel and suite merging.
2. **Collector support floor.** Invoke an already installed `cargo llvm-cov`, accept LLVM coverage export schema major 2 with bounded `cargo_llvm_cov` metadata, and otherwise show setup guidance. Zed performs no installation or network fallback.
