# Tasks: Rust coverage over generic source analysis

## Milestone 1: Generic coverage result and presentation

- [ ] 1. Add bounded language-neutral coverage annotations
  - [ ] 1.1. Implement generic coverage state and protocol
    - _Requirements: 1.1, 1.3, 1.4, 1.5, 2.4_
    - _Depends on: none_
    - _Reads: crates/project/src/structured_execution.rs, crates/project/src/project.rs, crates/proto/proto/structured_execution.proto, crates/project/src/lsp_store.rs_
    - _Writes: crates/project/src/source_coverage.rs, crates/project/src/project.rs, crates/proto/proto/source_coverage.proto, crates/proto/proto/zed.proto_
    - _Validation: cargo test -p project --features test-support source_coverage; cargo test -p proto source_coverage_
    - Design: D1, D4
    - Outcome: Project hosts can publish bounded language-neutral coverage facts.
    - Done when: Generation, limits, path filtering, malformed input and remote round trips pass without Cargo/Rust dependencies.
  - [ ] 1.2. Add generic gutter annotations and compact summary
    - _Requirements: 1.2, 1.3, 1.4, 1.5_
    - _Depends on: 1.1_
    - _Reads: crates/editor/src/editor.rs, crates/editor/src/element.rs, crates/language_tools/src/language_tools.rs, crates/ui/src/components_
    - _Writes: crates/editor/src/source_coverage.rs, crates/editor/src/editor.rs, crates/language_tools/src/source_coverage_summary.rs_
    - _Validation: cargo test -p editor --features test-support source_coverage; cargo test -p language_tools source_coverage_summary_
    - Design: D2, D5
    - Outcome: Any in-tree provider can display accessible bounded coverage without Rust rendering logic.
    - Done when: GPUI tests cover markers, focus/navigation, filters, partial/stale/truncated states and visible-range performance.

## Milestone 2: Rust collector adapter

- [ ] 2. Integrate an explicit supported collector through Tasks
  - [ ] 2.1. Add the authoritative Rust coverage artifact provider
    - Validate supported machine-readable artifacts on the project host, map visible paths, and publish bounded generic coverage facts after typed task completion.
    - _Requirements: 2.1, 2.2, 2.3, 2.4_
    - _Depends on: 1.1_
    - _Reads: crates/project/src/source_coverage.rs, crates/project/src/rust_test_provider.rs, crates/project/src/project.rs, crates/task/src/task.rs_
    - _Writes: crates/project/src/rust_coverage_provider.rs, crates/project/src/project.rs_
    - _Validation: cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-coverage rust_coverage_provider_
    - Design: D3, D4, D5
    - Cross-pack dependencies: completed `cargo-execution/1.1` and `rust-tools-platform/1.1` baselines.
    - Outcome: The host can interpret a completed coverage artifact without CargoWorkspaceStore or terminal parsing.
    - Done when: Supported/malformed artifacts, path mapping, bounds, stale cancellation and no-network assertions pass.
  - [ ] 2.2. Register the Rust coverage provider on enabled desktop/headless hosts
    - Add project/remote feature forwarding and request registration only under `rust-tools`, retaining inert protocol behavior when disabled.
    - _Requirements: 2.2, 2.3, 2.4_
    - _Depends on: 2.1_
    - _Reads: crates/project/Cargo.toml, crates/remote_server/Cargo.toml, crates/remote_server/src/headless_project.rs, script/check-rust-tools-feature-boundary_
    - _Writes: crates/project/Cargo.toml, crates/remote_server/Cargo.toml, crates/remote_server/src/headless_project.rs, script/check-rust-tools-feature-boundary_
    - _Validation: cargo check -p remote_server --features rust-tools; cargo check -p remote_server --no-default-features; ./script/check-rust-tools-feature-boundary_
    - Design: D3, D4, D5
    - Outcome: Only matching rust-tools hosts expose coverage artifact interpretation.
    - Done when: Enabled/disabled graphs, handlers, mismatch and no-client-fallback tests pass.
  - [ ] 2.3. Add the explicit Cargo Run with Coverage action
    - Compile the current selection/preset to an ordinary collector Task with a project-relative artifact declaration, then ask the host provider to interpret it after success.
    - _Requirements: 2.1, 2.2, 2.3_
    - _Depends on: 2.1, 2.2_
    - _Reads: crates/cargo_ui/src/cargo_preset.rs, crates/cargo_ui/src/cargo_actions.rs, crates/workspace/src/tasks.rs, crates/project/src/rust_coverage_provider.rs_
    - _Writes: crates/cargo_ui/src/cargo_coverage.rs, crates/cargo_ui/src/cargo_actions.rs_
    - _Validation: cargo test -p cargo_ui cargo_coverage_
    - Design: D3, D5
    - Outcome: Explicit coverage uses existing Tasks and a validated host artifact with no installer/fallback.
    - Done when: Exact argv/artifact path, missing-tool, trust, cancellation and redaction tests pass.

## Milestone 3: Coverage acceptance

- [ ] 3. Validate cross-provider, remote and scale behavior
  - [ ] 3.1. Add deterministic coverage acceptance fixtures
    - _Requirements: 1.4, 1.5, 2.4, 2.5_
    - _Depends on: 1.2, 2.3_
    - _Reads: crates/project/src/source_coverage.rs, crates/editor/src/source_coverage.rs, crates/cargo_ui/src/cargo_coverage.rs_
    - _Writes: crates/project/test_data/source_coverage/rust-report.json, crates/project/tests/integration/source_coverage.rs, crates/project/tests/integration/project_tests.rs_
    - _Validation: cargo test -p project --features test-support source_coverage_acceptance -- --nocapture; cargo test -p editor --features test-support source_coverage_large_
    - Design: D1, D2, D3, D4, D5
    - Outcome: Rust and fake non-Rust reports meet privacy, failure and performance requirements.
    - Done when: All required report forms, remote filtering, stale cancellation and bomb-size rejection pass hermetically.

## Mandatory manual task-decomposition audit

- Generic state/protocol, generic UI, Rust adapter and acceptance fixtures are separate leaves.
- No leaf adds execution to CargoWorkspaceStore or parses terminal output.
- Overlapping project/proto writes are sequenced; cross-pack dependencies point to completed baselines.
- Every criterion maps to D1–D5 and at least one leaf.
