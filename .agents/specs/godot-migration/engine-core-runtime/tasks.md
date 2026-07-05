# Implementation Plan: Engine Core and Runtime

## Overview

Create the Godot metadata substrate without porting runtime execution. This spec produces the G2 shared metadata gate used by later Godot integrations.

## Gates

- Start gate: G0 spec consistency and G1 boundary policy are satisfied.
- Validation gate: project detection, resource indexing, diagnostics, and boundary tests pass.
- Handoff gate: parse failures and runtime-only source areas produce stable diagnostics and boundary decisions.
- Completion gate: G2 shared Baymax game metadata is satisfied without adding scene-tree, renderer, physics, or platform runtime execution.

## Dependency Waves

- W2 Baymax game compatibility substrate: project descriptors and resource metadata start after W1 boundary policy primitives.

## Tasks

- [ ] 1. Implement Godot project and resource metadata
  - Add project descriptors, resource indexing, diagnostics, and runtime boundary tests.
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 3.1, 3.2_
  - _writes: crates/baymax_game/src/project.rs, crates/baymax_game/src/resource_index.rs, crates/baymax_game/src/boundary_tests.rs_
