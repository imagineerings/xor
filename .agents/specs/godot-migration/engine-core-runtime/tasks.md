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

- [x] 1. Implement Godot project and resource metadata
  - Add project descriptors, resource indexing, diagnostics, and runtime boundary tests.
  - Preserve only native Sim generative game-engine metadata required for indexing, preview, and tooling; runtime execution remains external or excluded.
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 3.1, 3.2_
  - _writes: crates/sim_game/src/project.rs, crates/sim_game/src/resource_index.rs, crates/sim_game/src/boundary_tests.rs_
