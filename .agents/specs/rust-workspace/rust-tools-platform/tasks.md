# Tasks: Rust tools platform boundary

## Milestone 0: Verified platform baseline

- [x] 1. Verify host, privacy and feature boundaries
  - [x] 1.1. Record authoritative-host and trust/privacy evidence
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9_
    - _Depends on: none_
    - _Reads: crates/project/src/cargo_workspace_store.rs, crates/project/src/structured_execution.rs, crates/project/src/rust_test_provider.rs, crates/remote_server/src/headless_project.rs, crates/proto/proto/cargo.proto, crates/proto/proto/structured_execution.proto_
    - _Writes: none_
    - _Validation: cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_workspace; cargo test -p remote_server --features test-support,rust-tools rust_test_provider_
    - Design: D1, D2, D5
    - Outcome: Host authority, privacy, cancellation and mismatch behavior are accepted as baseline.
    - Done when: Existing fake-runner/protocol/headless tests prove no local fallback and bounded visible data.
    - Evidence: Project store tests and HeadlessProject feature tests cover host registration, peer identity, redaction and stale generations.
  - [x] 1.2. Record enabled/disabled build and release evidence
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 4.5_
    - _Depends on: 1.1_
    - _Reads: crates/zed/Cargo.toml, crates/project/Cargo.toml, crates/remote_server/Cargo.toml, crates/zed/src/zed.rs, script/check-rust-tools-feature-boundary, .github/workflows/run_tests.yml_
    - _Writes: none_
    - _Validation: ./script/check-rust-tools-feature-boundary; cargo check -p zed --features rust-tools; cargo check -p zed --no-default-features; cargo check -p remote_server --features rust-tools; cargo check -p remote_server --no-default-features_
    - Design: D3, D4, D5
    - Outcome: `rust-tools` feature isolation and release parity are accepted as baseline.
    - Done when: Dependency graphs omit Cargo tooling when disabled and both desktop/headless configurations compile.
    - Evidence: Boundary script, generated CI steps and bundle dry-run assertions are present; inert protobuf is documented.

## Milestone 1: Integrated graduation evidence

- [ ] 2. Complete fixture, performance, environment and accessibility certification
  - [x] 2.1. Run the comprehensive fixture across the integrated feature
    - Wire the completed dashboard fixture through preset planning, structured results and Rust provider fake runners without real tools.
    - _Requirements: 4.1, 4.5_
    - _Depends on: none_
    - _Reads: crates/project/test_data/cargo_workspace/comprehensive-v1.json, crates/cargo_ui/src/cargo_preset.rs, crates/project/src/structured_execution.rs, crates/project/src/rust_test_provider.rs_
    - _Writes: crates/project/tests/integration/rust_workspace_comprehensive.rs, crates/project/tests/integration/project_tests.rs_
    - _Validation: cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_workspace_comprehensive -- --nocapture_
    - Design: D6
    - Cross-pack dependency: requires `cargo-dashboard/2.1` complete.
    - Outcome: One integrated hermetic fixture covers the Now stack.
    - Done when: The test records zero real tool/network calls and exercises partial/mismatch/stale transitions.
    - Evidence: The specified project command and the Cargo UI comprehensive-fixture planning test pass with zero real tool/network calls, partial-root isolation, typed result reduction and stale/mismatched generation rejection.
  - [x] 2.2. Enforce accepted dashboard and structured-result budgets
    - Add release/CI invocations for owner-pack benchmark budgets without duplicating their harnesses.
    - _Requirements: 4.2, 4.5_
    - _Depends on: 2.1_
    - _Reads: crates/project/benches/cargo_workspace.rs, crates/project/benches/structured_execution.rs, tooling/xtask/src/tasks/workflows/run_tests.rs_
    - _Writes: tooling/xtask/src/tasks/workflows/run_tests.rs, .github/workflows/run_tests.yml_
    - _Validation: cargo xtask workflows; ./script/check-rust-tools-feature-boundary_
    - Design: D5, D6
    - Cross-pack dependency: requires `cargo-dashboard/2.2`, `cargo-dashboard/2.3`, `structured-execution/2.1`, and `structured-execution/2.2` complete.
    - Outcome: Reviewed scale budgets become maintained release gates.
    - Done when: CI invokes both deterministic gates and does not depend on machine-specific Cargo projects.
    - Evidence: Both specified release benchmarks pass their enforced 1,000-package and 10,000-node budgets; `cargo xtask workflows` emits both gates and the offline local environment harness into `run_tests.yml`, and the feature-boundary check passes.
  - [ ] 2.3. Consume the physical project-mode matrix
    - Make supported local/SSH/WSL/dev-container/multiplayer cells visible in release evidence and fail required-cell regressions.
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.7, 4.3, 4.5_
    - _Depends on: 2.2_
    - _Reads: script/test-rust-tools-environments, .github/workflows/run_tests.yml, docs/src/remote-development.md_
    - _Writes: .github/workflows/run_tests.yml, docs/src/languages/rust.md_
    - _Validation: ./script/test-rust-tools-environments --matrix --offline; cargo xtask workflows_
    - Design: D1, D5, D6
    - Cross-pack dependency: requires `rust-test-explorer/2.2` complete.
    - Outcome: Zed distinguishes physically certified, unavailable and unsupported environment cells.
    - Done when: Required cells fail CI on routing/fallback regressions and unsupported cells are documented.
    - Current blocker: The local hermetic gate and matrix reporting are implemented, but actual SSH/WSL/development-container/multiplayer evidence is unavailable. Follow `../rust-test-explorer/physical-matrix-evidence.md`, then run `ZED_RUST_TOOLS_REQUIRE_PHYSICAL=1 ./script/test-rust-tools-environments --matrix --offline`.
  - [ ] 2.4. Complete screen-reader certification for Cargo and Tests panels
    - Execute and document role/name/state, focus, selection, expansion, filter/status and disabled-action announcements on supported desktop accessibility stacks.
    - _Requirements: 4.4_
    - _Depends on: 2.3_
    - _Reads: crates/language_tools/src/language_tool_tree.rs, crates/cargo_ui/src/cargo_panel.rs, crates/tasks_ui/src/test_explorer.rs, docs/src/languages/rust.md_
    - _Writes: docs/src/languages/rust.md, .agents/specs/rust-workspace/rust-tools-platform/accessibility-evidence.md_
    - _Validation: cargo test -p language_tools; cargo test -p cargo_ui; cargo test -p tasks_ui --features test-explorer test_explorer; manual VoiceOver/NVDA checklist in accessibility-evidence.md_
    - Design: D6
    - Outcome: Automated semantics are backed by explicit manual assistive-technology evidence.
    - Done when: Each supported stack has dated results and unresolved failures are tracked rather than marked complete.
    - Current blocker: Physical macOS VoiceOver and Windows NVDA sessions are unavailable. Run the automated commands and every dated checklist item in `accessibility-evidence.md` on both stacks.

## Mandatory manual task-decomposition audit

- Baseline host/privacy and build/release evidence are separate completed leaves.
- Fixture, budgets, physical matrix and accessibility are separate gaps with distinct owners and validation.
- Cross-pack dependencies point to real leaves; this pack consumes rather than duplicates their implementation.
- Every criterion maps to D1–D6 and at least one leaf.
