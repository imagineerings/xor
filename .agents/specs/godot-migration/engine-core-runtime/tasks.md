# Implementation Plan: Engine Core and Runtime

## Overview

Create the Godot metadata substrate without porting runtime execution only when a target-product import or preview path requires it. This is W7 deferred compatibility because W1 umbrella metadata already satisfies the shared foundations for Comfy/world-model work.

## Gates

- Start gate: G0 spec consistency and G1 boundary policy are satisfied.
- Validation gate: project detection, resource indexing, diagnostics, and boundary tests pass.
- Handoff gate: parse failures and runtime-only source areas produce stable diagnostics and boundary decisions.
- Completion gate: G2 shared Sim game metadata is satisfied without adding scene-tree, renderer, physics, or platform runtime execution.

## Dependency Waves

- W7 Deferred Godot-origin compatibility: project descriptors and resource metadata start only with an explicit product-enabling import or preview dependency.

## Tasks

- [ ] 1. Implement Godot project and resource metadata
  - Add project descriptors, resource indexing, diagnostics, and runtime boundary tests.
  - Preserve owner-native metadata required for indexing, preview, and tooling; unsupported runtime execution remains unresolved or excluded and is never delegated to Godot.
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 3.1, 3.2, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/language/src/language_registry.rs, crates/project/tests/integration/project_tests.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/engine-core-runtime/requirements.md, /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/engine-core-runtime/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/engine-core-runtime; run project/resource scenarios with Godot absent from PATH and loader paths; inspect process tree and runtime dependencies_

- [ ] 2. Prove native project and resource ownership without Godot
  - Add hermetic success, failure, cancellation, persistence, recovery, process-tree, package, and linkage checks proving Godot-format inputs produce only Sim-owned state.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/language/src/language_registry.rs, Cargo.toml, Cargo.lock_
  - _Writes: crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Validation: run the native project/resource fixture on a machine image without Godot and assert no Godot process, library, server, command, or package dependency_
