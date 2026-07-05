# Implementation Plan: Networking and Collaboration

## Overview

Keep Godot networking as boundary metadata and optional debug integration. Runtime networking remains excluded unless routed through explicit external task/debug integration.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Baymax game metadata are satisfied.
- Validation gate: networking boundary, debug metadata, and non-migration tests pass.
- Handoff gate: unsupported runtime networking features produce explicit boundary diagnostics.
- Completion gate: no Godot-specific network runtime or protocol adapter is added without G7 dependency review and an explicit external-command decision.

## Dependency Waves

- W6 External execution hardening: debug metadata and external task/debug hooks wait for boundary policy and metadata gates.

## Tasks

- [ ] 1. Add networking boundary and debug metadata support
  - Encode non-migration decisions and model optional debug metadata for task/debug workflows.
  - _Requirements: 1.1, 2.1_
  - _writes: crates/baymax_game/src/networking.rs, crates/baymax_game/src/debug_metadata.rs, crates/baymax_game/src/networking_tests.rs_
