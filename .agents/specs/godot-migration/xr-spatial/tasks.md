# Implementation Plan: XR and Spatial Tooling

## Dependency Gates

- **Primary wave**: W6 External execution hardening
- **Prerequisite gates**: G0 Spec consistency, G1 Boundary policy, G2 Shared Godot metadata
- **External execution gate**: XR task fallback work must wait for worker/task diagnostics appropriate to the execution target

## Tasks

- [ ] 1. Add XR boundary and spatial metadata support
  - Encode XR runtime exclusions and expose spatial asset metadata/docs hooks.
  - _Requirements: 1.1, 2.1_
  - _writes: crates/godot/src/xr.rs, crates/godot/src/spatial.rs, crates/godot/src/xr_tests.rs_
