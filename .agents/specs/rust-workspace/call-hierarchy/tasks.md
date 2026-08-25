# Tasks: Generic LSP call hierarchy

## Milestone 1: LSP model and routing

- [ ] 1. Add bounded call-hierarchy requests
  - [ ] 1.1. Implement prepare/incoming/outgoing project routing
    - _Requirements: 1.1, 1.2, 1.5, 1.6, 1.7_
    - _Depends on: none_
    - _Reads: crates/project/src/lsp_store.rs, crates/project/src/lsp_command.rs, crates/project/src/lsp_store/document_symbols.rs, crates/proto/proto/lsp.proto_
    - _Writes: crates/project/src/lsp_command.rs, crates/project/src/lsp_store.rs, crates/proto/proto/lsp.proto_
    - _Validation: cargo test -p project --features test-support call_hierarchy_lsp_
    - Design: D1, D4, D5
    - Outcome: Local and remote project paths expose bounded standard call-hierarchy requests.
    - Done when: Fake-server tests cover capabilities, locations, malformed data, cancellation and peer visibility.

## Milestone 2: Generic hierarchy view

- [ ] 2. Add editor and language-tools UI
  - [ ] 2.1. Implement lazy hierarchy projection and editor action
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.7_
    - _Depends on: none_
    - _Reads: crates/language_tools/src/language_tool_tree.rs, crates/editor/src/editor.rs, crates/editor/src/hover_links.rs, crates/workspace/src/workspace.rs_
    - _Writes: crates/language_tools/src/call_hierarchy.rs, crates/language_tools/src/language_tools.rs, crates/editor/src/editor.rs_
    - _Validation: cargo test -p language_tools call_hierarchy; cargo test -p editor --features test-support call_hierarchy_
    - Design: D2, D3, D5
    - Outcome: Users can inspect and navigate a bounded accessible callers/callees tree.
    - Done when: Cycles do not recurse, direction switching cancels old work, and keyboard/navigation/state tests pass.

## Milestone 3: Cross-language acceptance

- [ ] 3. Validate Rust and non-Rust behavior
  - [ ] 3.1. Add cross-language remote and scale coverage
    - _Requirements: 1.5, 1.6, 1.8_
    - _Depends on: 1.1, 2.1_
    - _Reads: crates/project/src/lsp_store.rs, crates/language_tools/src/call_hierarchy.rs, crates/remote_server/src/headless_project.rs_
    - _Writes: crates/project/tests/integration/call_hierarchy.rs, crates/project/tests/integration/project_tests.rs_
    - _Validation: cargo test -p project --features test-support call_hierarchy -- --nocapture; cargo test -p language_tools call_hierarchy_large_
    - Design: D4, D5
    - Outcome: The generic feature is proven independently of Rust and through remote routing.
    - Done when: Rust/non-Rust fake servers and a bounded large cyclic hierarchy pass with GPUI timers.

## Mandatory manual task-decomposition audit

- Project routing, UI and cross-language acceptance are independent leaves.
- No task introduces Rust/Cargo types or a second semantic index.
- Shared project files are sequenced through explicit dependencies where writes overlap.
- Every criterion maps to D1–D5 and at least one leaf.
