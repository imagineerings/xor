# Implementation Plan: Networking and Collaboration

## Overview

Keep Godot-origin networking as native Sim boundary metadata and optional debug metadata. Runtime networking remains excluded unless represented by native Sim task/debug metadata that directly supports the target generative game engine.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: networking boundary, debug metadata, and non-migration tests pass.
- Handoff gate: unsupported runtime networking features produce explicit boundary diagnostics.
- Completion gate: no Godot-specific network runtime or protocol adapter is added without G7 dependency review and an explicit external-command decision.

## Dependency Waves

- W7 Deferred Godot-origin compatibility: debug metadata and external task/debug hooks wait for boundary policy, metadata gates, and an explicit product-enabling dependency.

## Tasks

- [x] 1. Add networking boundary and debug metadata support
  - Encode non-migration decisions and model optional native Sim debug metadata for task/debug workflows.
  - _Requirements: 1.1, 1.2, 2.1_
  - _writes: crates/sim_game/src/networking.rs, crates/sim_game/src/debug_metadata.rs, crates/sim_game/src/networking_tests.rs_
