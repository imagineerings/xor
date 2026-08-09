# Implementation Plan: Platform and Export

## Overview

Import Godot-origin export presets into Sim-owned packaging requests only where an approved native platform owner exists. Platform packaging remains unresolved or excluded until reviewed dependencies and architecture are explicitly accepted.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: preset parsing, native packaging request, artifact, cancellation, unsupported-target, no-Godot, and package-dependency tests pass.
- Handoff gate: missing native owners, invalid presets, and unsupported targets produce actionable diagnostics without Godot setup guidance.
- Completion gate: export work is Sim-owned end to end, and every platform packaging dependency requires G7 dependency review.

## Dependency Waves

- W7 Deferred Godot-origin compatibility: native preset/import, packaging, and export integration waits for G1 boundary, G2 metadata, an approved platform owner, and an explicit product-enabling dependency.

## Tasks

- [ ] 1. Implement native preset import and export packaging requests
  - Parse `export_presets.cfg`, preserve compatibility metadata, select an approved native Sim platform owner, and create Sim-owned packaging/deployment requests or explicit unsupported diagnostics.
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: crates/project/src/project.rs, crates/settings/src/settings.rs, crates/task/src/task.rs, crates/gpui_platform/src/gpui_platform.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/platform-export/requirements.md, /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/platform-export/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/sim/.agents/specs/godot-migration/platform-export; package and execute supported exports without Godot; inspect spawned commands, package contents, process tree, and linked/runtime dependencies_

- [ ] 2. Prove exported projects are Godot-independent
  - Add hermetic packaging, signing, cancellation, cleanup, unsupported-target, package-content, linkage, and execution fixtures for each supported native platform tier.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: crates/project/src/project.rs, crates/settings/src/settings.rs, crates/task/src/task.rs, crates/gpui_platform/src/gpui_platform.rs, Cargo.toml, Cargo.lock_
  - _Writes: crates/task/src/task.rs, crates/project/src/project.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_
  - _Validation: run every supported exported artifact on a clean machine without Godot and assert no Godot process, file, dynamic library, server, CLI, hidden instance, or network delegation_
