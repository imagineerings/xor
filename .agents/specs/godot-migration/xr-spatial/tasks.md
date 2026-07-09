# Implementation Plan: XR and Spatial Tooling

## Overview

Keep XR runtime support out of Sim and expose W7 native Sim spatial metadata, docs, and preview routing only when they directly support the target generative game engine.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: XR runtime exclusion, spatial metadata, docs hook, and preview routing tests pass.
- Handoff gate: unsupported XR runtime paths produce excluded or external-command diagnostics.
- Completion gate: XR fallback work waits for W7 task diagnostics and does not embed OpenXR, WebXR, or VR runtime stacks.

## Dependency Waves

- W7 Deferred Godot-origin compatibility: spatial metadata and external fallback hooks wait for G1, G2, and an explicit product-enabling dependency.

## Tasks

- [x] 1. Add XR boundary and spatial metadata support
  - Encode XR runtime exclusions and expose native Sim spatial asset metadata/docs hooks.
  - _Requirements: 1.1, 1.2, 2.1_
  - _writes: crates/sim_game/src/xr.rs, crates/sim_game/src/spatial.rs, crates/sim_game/src/xr_tests.rs_
