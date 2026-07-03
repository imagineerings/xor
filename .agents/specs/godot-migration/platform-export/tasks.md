# Implementation Plan: Platform and Export

## Dependency Gates

- **Primary wave**: W6 External execution hardening
- **Prerequisite gates**: G0 Spec consistency, G1 Boundary policy, G2 Shared Godot metadata
- **Dependency gate**: G7 Dependency review for any platform packaging dependency beyond task invocation

## Tasks

- [ ] 1. Implement Godot executable settings and export task templates
  - Parse `export_presets.cfg`, resolve executable settings, and create external export tasks.
  - _Requirements: 1.1, 2.1, 2.2_
  - _writes: crates/godot/src/export.rs, crates/godot/src/executable.rs, crates/godot/src/export_tests.rs_
