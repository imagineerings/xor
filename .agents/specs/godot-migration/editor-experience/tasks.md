# Implementation Plan: Editor Experience

## Overview

Add native game-development commands and project-panel affordances in W4, while run and debug templates remain W6 work because they require the external-command boundary.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: command registration, project-panel classification, import-link, and run/debug template tests pass.
- Handoff gate: missing Godot executable and unsupported runtime actions surface setup diagnostics.
- Completion gate: no run/debug workflow bypasses the W6 external-command path or the G1 runtime boundary policy.

## Dependency Waves

- W4 Authoring, graph UX, and Comfy workflows: commands, labels, and import links can start after G2.
- W6 External execution hardening: run/debug templates wait for external-command diagnostics and task integration.

## Tasks

- [ ] 1. Add native game editor affordances
  - Register commands, project-panel labels, import links, and external run/debug templates.
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 3.1, 3.2_
  - _writes: crates/sim_game/src/editor.rs, crates/sim_game/src/tasks.rs, crates/sim_game/src/editor_tests.rs_
