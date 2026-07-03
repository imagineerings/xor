# Implementation Plan: Editor Experience

## Dependency Gates

- **Primary wave**: W4 Authoring and graph UX; W6 External execution hardening for run/debug tasks
- **Prerequisite gates**: G0 Spec consistency, G1 Boundary policy, G2 Shared Godot metadata
- **External execution gate**: run/debug task work must wait for the Godot external-command path in W6

## Tasks

- [ ] 1. Add Godot-aware editor affordances
  - Register commands, project-panel labels, import links, and external run/debug templates.
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 3.1, 3.2_
  - _writes: crates/godot/src/editor.rs, crates/godot/src/tasks.rs, crates/godot/src/editor_tests.rs_
