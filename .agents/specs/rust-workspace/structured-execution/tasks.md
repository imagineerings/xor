# Tasks: Structured execution results

## Milestone 0: Verified generic results baseline

- [x] 1. Verify the generic store, task bridge and Tests panel
  - [x] 1.1. Record language-neutral implementation evidence
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 2.2_
    - _Depends on: none_
    - _Reads: crates/project/src/structured_execution.rs, crates/proto/proto/structured_execution.proto, crates/task/src/task.rs, crates/tasks_ui/src/test_explorer.rs, crates/language_tools/src/language_tool_tree.rs_
    - _Writes: none_
    - _Validation: cargo test -p project --features test-support,structured-execution structured_execution; cargo test -p task structured; cargo test -p tasks_ui --features test-explorer test_explorer_
    - Design: D1, D2, D3, D4, D5
    - Outcome: The 10,000-node generic result platform is accepted as baseline.
    - Done when: Existing tests prove bounds, idempotency, lifecycle, generic UI and non-Rust projection without Cargo dependencies.
    - Evidence: `structured_execution.rs` includes bounded 10,000-node, malformed-page, stale-generation and privacy tests; Tests UI includes non-Rust and GPUI interaction tests.

## Milestone 1: Performance certification

- [x] 2. Establish formal structured-result budgets
  - [x] 2.1. Benchmark store reduction and paging at the supported limit
    - Add a deterministic synthetic-provider benchmark for discovery application, event reduction and pagination.
    - _Requirements: 2.1, 2.2_
    - _Depends on: none_
    - _Reads: crates/project/src/structured_execution.rs, crates/project/Cargo.toml_
    - _Writes: crates/project/benches/structured_execution.rs, crates/project/Cargo.toml_
    - _Validation: cargo bench -p project --features structured-execution --bench structured_execution_
    - Design: D6
    - Cross-pack dependency: `rust-tools-platform/2.2` consumes the accepted budget in release certification.
    - Outcome: The current limit has reviewed, repeatable time/memory regression gates.
    - Done when: 10,000-node discovery, events and paging meet recorded time/memory budgets.
    - _Evidence: `cargo bench -p project --features structured-execution --bench structured_execution` passed on macOS arm64 after exercising every protocol page: 2 ms discovery, 3 ms event reduction, 65 ms pagination and approximately 5.4 MiB conservatively modeled retained state against 2 s/2 s/100 ms/64 MiB ceilings. Criterion recorded 2.04–2.26 ms discovery, 3.58–4.03 ms reduction and 66.82–68.74 ms pagination._
  - [x] 2.2. Certify Tests-panel foreground projection
    - Add a GPUI executor test separating background reduction from foreground tree reconciliation and visible-range rendering.
    - _Requirements: 2.1, 2.3_
    - _Depends on: 2.1_
    - _Reads: crates/tasks_ui/src/test_explorer.rs, crates/language_tools/src/language_tool_tree.rs, docs/src/tasks.md_
    - _Writes: crates/tasks_ui/src/test_explorer.rs, docs/src/tasks.md_
    - _Validation: cargo test -p tasks_ui --features test-explorer structured_execution_foreground_budget -- --nocapture_
    - Design: D6
    - Cross-pack dependency: `rust-tools-platform/2.2` consumes this accepted budget.
    - Outcome: Generic Tests projection has a reviewed foreground budget.
    - Done when: 10,000 rows remain bounded/virtualized and every timed transition uses GPUI executor timers.
    - _Evidence: `cargo test -p tasks_ui --features test-explorer structured_execution_foreground_budget -- --nocapture` passed at 37 ms against the 250 ms foreground ceiling for exactly 10,000 rows. Provider projection runs on the background executor, the test yields through `background_executor.timer`, and foreground validation accesses only a 25-row visible range._

## Mandatory manual task-decomposition audit

- Verified model, bridge and UI work remains one evidence leaf and is not reissued.
- Store benchmarking and GPUI projection certification are separate leaves with distinct owners and writes.
- No task adds provider-specific discovery or changes Cargo/Rust ownership.
- Every criterion maps to D1–D6 and at least one leaf.
