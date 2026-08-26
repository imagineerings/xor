# Tasks: Rust profiling workflow

## Milestone 0: External workflow baseline

- [x] 1. Verify explicit Tasks remain the profiling path
  - [x] 1.1. Record existing task/remote/artifact-opening evidence
    - _Requirements: 1.1, 1.4, 1.5_
    - _Depends on: none_
    - _Reads: crates/task/src/task.rs, crates/workspace/src/tasks.rs, crates/workspace/src/workspace.rs, docs/src/tasks.md, docs/src/remote-development.md_
    - _Writes: none_
    - _Validation: cargo test -p task; cargo test -p workspace test_open_url_or_file_routes_urls_
    - Design: D1, D3
    - Outcome: External explicit Tasks are accepted as current profiling support.
    - Done when: Existing systems demonstrably own command, terminal, cancellation, remote routing and safe opening.
    - Evidence: Task/Workspace APIs and documentation provide the path; no native profiler symbols or installer were found.

## Milestone 1: Optional external-tool convenience

- [x] 2. Add a declarative profile Task/artifact flow if approved
  - [x] 2.1. Compile an explicit Cargo profile preset and open its declared artifact
    - _Requirements: 1.2, 1.3, 1.4, 1.5_
    - _Depends on: none_
    - _Reads: crates/cargo_ui/src/cargo_preset.rs, crates/workspace/src/tasks.rs, crates/workspace/src/workspace.rs, crates/task/src/task.rs_
    - _Writes: crates/cargo_ui/src/cargo_profile.rs, crates/workspace/src/tasks.rs_
    - _Validation: cargo test -p cargo_ui cargo_profile_plan; cargo test -p workspace profile_artifact_opening_
    - Design: D2, D3
    - Cross-pack dependencies: completed `cargo-execution/1.1` and `rust-tools-platform/1.1` baselines.
    - Outcome: An approved shortcut remains an explicit Task with a declared safe artifact and no blessed installer.
    - Done when: Missing tool/artifact, remote visibility, size, trust and cancellation tests pass without terminal parsing.
    - Evidence: Pure Cargo planning preserves the compiled host context while requiring an explicit external command; Task resolution rejects unsafe/oversized artifact declarations; Workspace opens only successful, visible, bounded project files. Missing, failed and cancelled artifacts stay in the existing task failure path and no terminal parser or installer is present.

## Milestone 2: Native profiling decision and later implementation

- [ ] 3. Gate any native viewer on evidence
  - [ ] 3.1. Decide collector, artifact and platform support
    - Produce an ADR using deterministic captures and licensing/remote analysis; do not add product types in this leaf.
    - _Requirements: 2.1_
    - _Depends on: none_
    - _Reads: crates/task/src/task.rs, crates/workspace/src/workspace.rs, docs/src/remote-development.md, Cargo.toml_
    - _Writes: .agents/specs/rust-workspace/rust-profiling/collector-adr.md_
    - _Validation: manual ADR review confirms collector versions, licenses, platforms, bounds, privacy, cancellation and rejection fallback_
    - Design: D4
    - Outcome: Native profiling has an explicit go/no-go decision and bounded contract inputs.
    - Done when: The ADR is approved; otherwise Task 3.2 remains deferred.
  - [ ] 3.2. Implement the approved generic native profile result model
    - Only after an affirmative ADR, implement the minimal generic model, bounded background parser and wire conversion while keeping collection in Tasks.
    - _Requirements: 2.2, 2.3, 2.4_
    - _Depends on: 3.1_
    - _Reads: .agents/specs/rust-workspace/rust-profiling/collector-adr.md, crates/project/src/structured_execution.rs, crates/task/src/task.rs, crates/proto/proto/structured_execution.proto_
    - _Writes: crates/project/src/profile_results.rs, crates/proto/proto/profile_results.proto_
    - _Validation: cargo test -p project --features test-support profile_results; cargo test -p proto profile_results_
    - Design: D5
    - Outcome: If approved, a bounded language-neutral model consumes explicit Task artifacts.
    - Done when: Supported/unsupported platform, malformed/oversized, remote, cancellation and large-profile model fixtures pass.
  - [ ] 3.3. Implement the approved accessible native profile view
    - Project the approved generic profile model through a virtualized language-tools view with source navigation and explicit partial/stale states.
    - _Requirements: 2.2, 2.4_
    - _Depends on: 3.2_
    - _Reads: crates/project/src/profile_results.rs, crates/language_tools/src/language_tool_tree.rs, crates/workspace/src/workspace.rs_
    - _Writes: crates/language_tools/src/profile_view.rs, crates/language_tools/src/language_tools.rs_
    - _Validation: cargo test -p language_tools profile_view_
    - Design: D5
    - Outcome: If approved, users can inspect bounded profile results without Rust-specific rendering.
    - Done when: Accessibility, keyboard/navigation, partial/stale and large-profile virtualization tests pass with GPUI timers.

## Mandatory manual task-decomposition audit

- Existing external support is evidence-only; optional convenience, ADR, native model and native view are distinct leaves.
- The ADR is an explicit dependency gate; no native code is authorized before it.
- No task bundles profilers, parses terminals or adds execution to CargoWorkspaceStore.
- Every criterion maps to D1–D5 and at least one leaf.
