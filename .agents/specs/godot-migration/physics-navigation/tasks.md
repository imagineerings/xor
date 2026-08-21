# Implementation Plan: Physics and Navigation

## Overview

Represent physics and navigation as W7 Zed-native metadata and docs only when they directly support the target generative game engine. Executable behavior remains excluded or decision-blocked until a Zed-owned runtime is approved.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Zed game metadata are satisfied.
- Validation gate: runtime exclusion, metadata extraction, docs lookup, native-owner, and no-Godot tests pass.
- Handoff gate: unsupported physics/navigation execution produces excluded or decision-required diagnostics.
- Completion gate: metadata does not claim executable support and no task launches, wraps, or delegates to Godot physics/navigation servers.

## Dependency Waves

- W7 Deferred Godot-origin compatibility: metadata/docs hooks wait for G1, G2, and an explicit product-enabling dependency; runtime tasks additionally wait for an approved native Zed architecture.

## Tasks

- [ ] 1. Add physics/navigation boundary metadata
  - Encode runtime exclusions, extract native Zed metadata, and provide docs/diagnostic hooks without runtime placeholders or fallback launchers.
  - _Requirements: 1.1, 1.2, 2.1, 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Writes: crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/task/src/task.rs, crates/diagnostics/src/diagnostics.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/physics-navigation/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/physics-navigation/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/physics-navigation; run metadata/exclusion scenarios without Godot and prove no task, process, library, server, or runtime dependency_

- [ ] 2. Prove native simulation ownership or explicit exclusion
  - Add hermetic metadata, unsupported-runtime, failure, cancellation, process, loader, dependency, and deterministic lifecycle checks; executable fixtures require an approved native owner.
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 1_
  - _Reads: crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/task/src/task.rs, crates/diagnostics/src/diagnostics.rs_
  - _Writes: crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/diagnostics/src/diagnostics.rs_
  - _Validation: execute supported metadata or approved native simulation on a machine without Godot; assert no Godot process, library, server, CLI, hidden instance, or runtime dependency_
