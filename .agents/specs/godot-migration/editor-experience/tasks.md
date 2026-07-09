# Implementation Plan: Editor Experience

## Overview

Add native game-development commands and project-panel affordances in W5 after the Comfy/world-model harness can power them, while Godot run and debug templates remain W7 work because they require the external-command boundary.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: command registration, project-panel classification, import-link, and run/debug template tests pass.
- Handoff gate: missing Godot executable and unsupported runtime actions surface setup diagnostics.
- Completion gate: no run/debug workflow bypasses the W7 external-command path or the G1 runtime boundary policy.

## Dependency Waves

- W5 Product authoring and agentic tools: commands, labels, and import links can start after G2 and should consume W2-W4 harness capabilities.
- W7 Deferred Godot-origin compatibility: run/debug templates wait for external-command diagnostics, task integration, and an explicit product-enabling dependency.

## Tasks

- [x] 1. Add native game editor affordances
  - Register commands, project-panel labels, import links, and external run/debug templates.
  - Recreate Comfy/world-model authoring affordances as native Sim game commands and metadata, not as thin compatibility labels or pass-through workflows.
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 3.1, 3.2_
  - _writes: crates/sim_game/src/editor.rs, crates/sim_game/src/editor_tests.rs_
