# Implementation Plan: Physics and Navigation

## Overview

Represent physics and navigation as metadata, docs, and external simulation hooks. Godot physics or navigation server execution remains outside Sim runtime.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: runtime exclusion, metadata extraction, docs lookup, and task fallback tests pass.
- Handoff gate: unsupported physics/navigation execution produces external-command or excluded diagnostics.
- Completion gate: simulation task fallback work waits for W6 external-command diagnostics and does not embed Godot physics/navigation servers.

## Dependency Waves

- W6 External execution hardening: metadata-backed fallback hooks wait for G1 and G2.

## Tasks

- [ ] 1. Add physics/navigation boundary metadata
  - Encode runtime exclusions, extract metadata, and provide docs/task fallback hooks.
  - _Requirements: 1.1, 2.1_
  - _writes: crates/sim_game/src/physics.rs, crates/sim_game/src/navigation.rs, crates/sim_game/src/physics_tests.rs_
