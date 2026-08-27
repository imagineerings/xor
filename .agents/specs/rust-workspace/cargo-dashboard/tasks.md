# Tasks: Cargo dashboard

## Milestone 0: Verified dashboard baseline

- [x] 1. Verify the implemented Cargo dashboard baseline
  - [x] 1.1. Record model, store, panel and configuration evidence
    - Confirm the current code satisfies the baseline without recreating implementation work.
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8_
    - _Depends on: none_
    - _Reads: crates/project/src/cargo_workspace.rs, crates/project/src/cargo_workspace_store.rs, crates/project/tests/integration/cargo_workspace.rs, crates/cargo_ui/src/cargo_panel.rs, crates/language_tools/src/language_tool_tree.rs_
    - _Writes: none_
    - _Validation: cargo test -p project --features test-support,cargo-workspace cargo_workspace -- --nocapture; cargo test -p cargo_ui; cargo test -p language_tools_
    - Design: D1, D2, D3, D4, D6
    - Outcome: The canonical pack accepts the implemented dashboard and bounded configuration behavior as baseline.
    - Done when: The cited source/tests still prove every covered criterion without a new implementation task.
    - Evidence: Model fixtures cover virtual/standalone roots, every target/dependency kind and configuration parsing; store/panel tests cover cancellation, stale state, navigation, 1,000 packages and 10,000 rows.

## Milestone 1: Dashboard validation gaps

- [x] 2. Complete deterministic and performance certification
  - [x] 2.1. Add the comprehensive standalone evaluation fixture
    - Compose existing Cargo captures into one deterministic fixture and assert multi-root, malformed and partial behavior without invoking installed tools.
    - _Requirements: 3.1_
    - _Depends on: none_
    - _Reads: crates/project/test_data/cargo_workspace/workspace-v1.json, crates/project/test_data/cargo_workspace/standalone-v1.json, crates/project/tests/integration/cargo_workspace.rs_
    - _Writes: crates/project/test_data/cargo_workspace/comprehensive-v1.json, crates/project/tests/integration/cargo_workspace.rs, crates/project/tests/integration/project_tests.rs_
    - _Validation: cargo test -p project --features test-support,cargo-workspace cargo_workspace_comprehensive_fixture -- --nocapture_
    - Design: D5
    - Cross-pack dependency: `rust-tools-platform/2.1` uses this fixture for the final matrix.
    - Outcome: One hermetic fixture exercises the historically requested evaluation surface.
    - Done when: The focused test proves all listed characteristics and records no real Cargo/network invocation.
    - _Evidence: `cargo test -p project --features test-support,cargo-workspace cargo_workspace_comprehensive_fixture -- --nocapture` passed (1 focused test); the test binary now explicitly includes the feature-gated Cargo integration module._
  - [x] 2.2. Establish pure model and projection budgets
    - Add a repeatable pure metadata/model/tree-projection benchmark and document reviewed time and retained-memory budgets without raising runtime limits.
    - _Requirements: 3.2_
    - _Depends on: 2.1_
    - _Reads: crates/project/src/cargo_workspace.rs, crates/project/test_data/cargo_workspace/comprehensive-v1.json, crates/project/Cargo.toml_
    - _Writes: crates/project/benches/cargo_workspace.rs, crates/project/Cargo.toml_
    - _Validation: cargo bench -p project --features cargo-workspace --bench cargo_workspace_
    - Design: D5
    - Cross-pack dependency: `rust-tools-platform/2.2` consumes the accepted budget.
    - Outcome: Dashboard scale has reproducible, accepted regression budgets.
    - Done when: The benchmark covers at least 1,000 packages, reports reviewed time/memory budgets, and is deterministic on the documented runner class.
    - _Evidence: `cargo bench -p project --features cargo-workspace --bench cargo_workspace` passed on macOS arm64; the deterministic 1,000-package gate reported 10 ms and 517,559 retained modeled bytes against 2 s/32 MiB ceilings, with Criterion at 4.2533–4.4942 ms._
  - [x] 2.3. Certify GPUI foreground reconciliation and 10,000-row projection
    - Add a visible-range/refresh budget test using the accepted model fixture and GPUI executor timing; document the foreground budget.
    - _Requirements: 3.2, 3.3_
    - _Depends on: 2.2_
    - _Reads: crates/cargo_ui/src/cargo_panel.rs, crates/language_tools/src/language_tool_tree.rs, docs/src/languages/rust.md_
    - _Writes: crates/cargo_ui/src/cargo_panel.rs, crates/cargo_ui/Cargo.toml, docs/src/languages/rust.md_
    - _Validation: cargo test -p cargo_ui cargo_dashboard_foreground_budget -- --nocapture; cargo test -p language_tools rust_workspace_large_model_
    - Design: D5
    - Cross-pack dependency: `rust-tools-platform/2.2` consumes the accepted foreground budget.
    - Outcome: UI reconciliation has a reviewed budget separate from background parsing.
    - Done when: 10,000 rows remain virtualized/deterministic and timed transitions use GPUI executor timers.
    - _Evidence: `cargo test -p cargo_ui cargo_dashboard_foreground_budget -- --nocapture` passed at 74 ms against the 250 ms foreground ceiling for exactly 10,000 rows; `cargo test -p language_tools rust_workspace_large_model` passed the generic 10,000-row visible-range check. The GPUI test projects on the background executor and yields through `background_executor.timer`._

## Mandatory manual task-decomposition audit

- Baseline evidence is one verification leaf, not a recreated implementation plan.
- The fixture and benchmark are separate leaves because their writes, review risks and validation differ.
- No leaf changes metadata ownership, introduces execution, or overlaps another active pack.
- Every criterion maps to D1–D6 and at least one leaf; cross-pack dependencies point to the completed platform baseline or a named real leaf.
