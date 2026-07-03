# Implementation Plan: Physics and Navigation

## Dependency Gates

- **Primary wave**: W6 External execution hardening
- **Prerequisite gates**: G0 Spec consistency, G1 Boundary policy, G2 Shared Godot metadata
- **External execution gate**: simulation task fallback work must wait for explicit external-command diagnostics

## Tasks

- [ ] 1. Add physics/navigation boundary metadata
  - Encode runtime exclusions, extract metadata, and provide docs/task fallback hooks.
  - _Requirements: 1.1, 2.1_
  - _writes: crates/godot/src/physics.rs, crates/godot/src/navigation.rs, crates/godot/src/physics_tests.rs_
