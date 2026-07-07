# Implementation Plan: XR and Spatial Tooling

## Overview

Keep XR runtime support out of Sim and expose only spatial metadata, docs, preview routing, and explicit external fallback hooks.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: XR runtime exclusion, spatial metadata, docs hook, and preview routing tests pass.
- Handoff gate: unsupported XR runtime paths produce excluded or external-command diagnostics.
- Completion gate: XR fallback work waits for W6 task diagnostics and does not embed OpenXR, WebXR, or VR runtime stacks.

## Dependency Waves

- W6 External execution hardening: spatial metadata and external fallback hooks wait for G1 and G2.

## Tasks

- [ ] 1. Add XR boundary and spatial metadata support
  - Encode XR runtime exclusions and expose spatial asset metadata/docs hooks.
  - _Requirements: 1.1, 2.1_
  - _writes: crates/sim_game/src/xr.rs, crates/sim_game/src/spatial.rs, crates/sim_game/src/xr_tests.rs_
