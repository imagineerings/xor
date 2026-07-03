# Implementation Plan: Engine Core and Runtime

## Dependency Gates

- **Primary wave**: W2 Godot compatibility substrate
- **Prerequisite gates**: G0 Spec consistency, G1 Boundary policy
- **Gate produced/extended**: G2 Shared Godot metadata

## Tasks

- [ ] 1. Implement Godot project and resource metadata
  - Add project descriptors, resource indexing, diagnostics, and runtime boundary tests.
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 3.1, 3.2_
  - _writes: crates/godot/src/project.rs, crates/godot/src/resource_index.rs, crates/godot/src/boundary_tests.rs_
