# Implementation Plan: Editor Experience

## Overview

Add native game-development commands and project-panel affordances in W5 after the Comfy/world-model harness can power them. Run/debug remains W7 work only where a Sim-owned runtime and debugger path has been approved.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: command registration, project-panel classification, import-link, and run/debug template tests pass.
- Handoff gate: unsupported native runtime actions surface explicit unresolved/excluded diagnostics without Godot setup guidance.
- Completion gate: no run/debug workflow launches, embeds, wraps, or delegates execution to Godot.

## Dependency Waves

- W5 Product authoring and agentic tools: commands, labels, and import links can start after G2 and should consume W2-W4 harness capabilities.
- W7 Deferred Godot-origin compatibility: run/debug waits for a Sim-native runtime owner, native diagnostics/task integration, and an explicit product-enabling dependency.

## Tasks

- [ ] 1. Add native game editor affordances
  - Register commands, project-panel labels, import links, and Sim-owned run/debug actions where a native runtime owner exists.
  - Recreate Comfy/world-model authoring affordances as native Sim game commands and metadata, not as thin compatibility labels or pass-through workflows.
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 3.1, 3.2, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: crates/workspace/src/workspace.rs, crates/project_panel/src/project_panel.rs, crates/command_palette/src/command_palette.rs, crates/task/src/task.rs, crates/dap/src/dap.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/editor-experience/requirements.md, /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/editor-experience/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/editor-experience; run command and run/debug scenarios without Godot installed and inspect process/runtime dependencies_

- [ ] 2. Prove native editor execution without Godot
  - Add hermetic UI, command, run/debug, unsupported-state, cancellation, recovery, process-tree, and linkage validation for every supported editor capability.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: crates/workspace/src/workspace.rs, crates/project_panel/src/project_panel.rs, crates/task/src/task.rs, crates/dap/src/dap.rs, Cargo.toml, Cargo.lock_
  - _Writes: crates/workspace/src/workspace.rs, crates/project_panel/src/project_panel.rs, crates/task/src/task.rs, crates/dap/src/dap.rs_
  - _Validation: execute supported editor and run/debug scenarios on a machine image without Godot; assert no Godot process, library, server, CLI, hidden instance, or package dependency_
