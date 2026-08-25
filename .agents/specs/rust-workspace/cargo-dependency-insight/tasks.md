# Tasks: Cargo dependency insight

## Milestone 1: Bounded provenance model

- [ ] 1. Extend Cargo metadata with validated provenance
  - [ ] 1.1. Add manifest/lock/resolution provenance conversion
    - _Requirements: 1.1, 1.3, 1.5, 1.6, 1.7_
    - _Depends on: none_
    - _Reads: crates/project/src/cargo_workspace.rs, crates/project/src/cargo_workspace_store.rs, crates/project/test_data/cargo_workspace/workspace-v1.json, crates/proto/proto/cargo.proto_
    - _Writes: crates/project/src/cargo_workspace.rs, crates/project/src/cargo_workspace_store.rs, crates/proto/proto/cargo.proto_
    - _Validation: cargo test -p project --features test-support,cargo-workspace cargo_dependency_provenance; cargo test -p proto cargo_dependency_provenance_
    - Design: D1, D2, D4, D5
    - Cross-pack dependency: completed `cargo-dashboard/1.1` baseline.
    - Outcome: The authoritative host exposes bounded, origin-labelled direct dependency provenance.
    - Done when: Every declaration/source kind, ambiguity, missing lock, cycle, trust and path filter passes without network.

## Milestone 2: Finite Cargo UI detail

- [ ] 2. Present selected dependency provenance
  - [ ] 2.1. Add the package-centric detail projection and navigation
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_
    - _Depends on: 1.1_
    - _Reads: crates/cargo_ui/src/cargo_panel.rs, crates/project/src/cargo_workspace.rs, crates/language_tools/src/language_tool_tree.rs_
    - _Writes: crates/cargo_ui/src/cargo_panel.rs, crates/cargo_ui/src/dependency_insight.rs_
    - _Validation: cargo test -p cargo_ui cargo_dependency_insight_
    - Design: D1, D3, D4
    - Outcome: A selected direct dependency has accessible finite provenance and safe local navigation.
    - Done when: Multiple versions, unknown causality, external sources, cycles and stale state render without recursive children.

## Milestone 3: Remote and scale acceptance

- [ ] 3. Validate privacy and large graphs
  - [ ] 3.1. Add deterministic provenance acceptance fixtures
    - _Requirements: 1.5, 1.6, 1.7, 1.8_
    - _Depends on: 1.1, 2.1_
    - _Reads: crates/project/src/cargo_workspace_store.rs, crates/cargo_ui/src/dependency_insight.rs, crates/remote_server/src/headless_project.rs_
    - _Writes: crates/project/test_data/cargo_workspace/dependency-provenance-v1.json, crates/project/tests/integration/cargo_workspace.rs_
    - _Validation: cargo test -p project --features test-support,cargo-workspace cargo_dependency_provenance_acceptance -- --nocapture; cargo test -p cargo_ui cargo_dependency_insight_large_
    - Design: D4, D5
    - Outcome: Provenance remains deterministic, private and bounded across local/remote scale.
    - Done when: Peer filtering, stale rejection, no-network runner assertions and the accepted large graph cap pass.

## Mandatory manual task-decomposition audit

- Model conversion, UI detail and acceptance fixtures are separate leaves.
- No task creates recursive dependency UI, mutation actions or external audits.
- Shared model/UI writes are sequenced; cross-pack dependency targets a completed baseline.
- Every criterion maps to D1–D5 and at least one leaf.
