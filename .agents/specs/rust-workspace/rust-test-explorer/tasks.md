# Tasks: Rust test explorer

## Milestone 0: Verified Rust provider baseline

- [x] 1. Verify discovery, action and result integration
  - [x] 1.1. Record protocol/provider/action evidence
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 1.11_
    - _Depends on: none_
    - _Reads: crates/project/src/rust_test_provider.rs, crates/project/test_data/rust_test_provider/cargo_messages.jsonl, crates/proto/proto/structured_execution.proto, crates/tasks_ui/src/test_explorer.rs, docs/src/languages/rust.md_
    - _Writes: none_
    - _Validation: cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_protocol -- --nocapture; cargo test -p project --features test-support,cargo-workspace,structured-execution,rust-tests rust_test_protocol_fixture_matrix -- --ignored --nocapture; cargo test -p tasks_ui --features rust-test-actions test_explorer_
    - Design: D1, D2, D3, D4, D5
    - Outcome: Built-in Rust discovery and structured actions are accepted as baseline.
    - Done when: Existing fixtures/tests prove every harness kind, partial parsing, exact Tasks/DAP, cancellation, rerun and privacy.
    - Evidence: The configured stable fixture matrix covers unit/integration/bin/example/bench/ignored/doctest; project and Tests UI tests cover lifecycle and remote request identity.

## Milestone 1: Physical environment certification

- [ ] 2. Validate supported local and remote project modes
  - [ ] 2.1. Build the physical Rust test matrix harness
    - Exercise production project transports/environments with a hermetic offline fixture and emit supported/unsupported discovery, run, cancel, reconnect and debug outcomes per platform cell.
    - _Requirements: 2.1, 2.2, 2.3_
    - _Depends on: none_
    - _Reads: crates/remote_server/src/headless_project.rs, crates/project/src/rust_test_provider.rs, crates/tasks_ui/src/test_explorer.rs_
    - _Writes: crates/project/test_data/rust_test_provider/physical-workspace/Cargo.toml, script/test-rust-tools-environments_
    - _Validation: ./script/test-rust-tools-environments --matrix --offline; cargo test -p remote_server --features test-support,rust-tools rust_test_provider_
    - Design: D5, D6
    - Cross-pack dependency: `rust-tools-platform/2.3` consumes the completed matrix after Task 2.2.
    - Outcome: Release evidence distinguishes real supported and unsupported environment cells.
    - Done when: The harness runs local and actual SSH/headless cells, probes available WSL/dev-container cells, observes host execution, and rejects stale reconnect results.
    - Current evidence: `physical-workspace` and `script/test-rust-tools-environments` implement the offline local probe and configurable SSH/WSL/container/multiplayer rows. Actual production-transport certification remains in `physical-matrix-evidence.md`; run `ZED_RUST_TOOLS_REQUIRE_PHYSICAL=1 ./script/test-rust-tools-environments --matrix --offline` in the required physical environments.
  - [ ] 2.2. Add required physical cells to CI and release documentation
    - Select required/optional cells from the harness, fail required regressions, and document explicit unsupported results.
    - _Requirements: 2.1, 2.2, 2.3_
    - _Depends on: 2.1_
    - _Reads: script/test-rust-tools-environments, .github/workflows/run_tests.yml, docs/src/remote-development.md_
    - _Writes: .github/workflows/run_tests.yml, docs/src/languages/rust.md_
    - _Validation: cargo xtask workflows; ./script/test-rust-tools-environments --matrix --offline_
    - Design: D5, D6
    - Cross-pack dependency: `rust-tools-platform/2.3` requires this leaf complete.
    - Outcome: Physical compatibility results are maintained rather than inferred from source-shape tests.
    - Done when: Required cells run in CI/release gates and optional/unsupported cells have visible reasons.
    - Current evidence: Generated CI runs the hermetic local matrix and documentation exposes optional/unavailable reasons. Required remote physical cells cannot be selected until the dated production-transport checklist in `physical-matrix-evidence.md` is complete.

## Mandatory manual task-decomposition audit

- Completed provider implementation is one evidence leaf, not duplicated work.
- Physical certification is isolated from unit fixtures and does not add runner variants.
- No task modifies generic structured result ownership or Cargo metadata execution.
- Every criterion maps to D1–D6 and at least one leaf.
