# Implementation Plan: Platform and Export

## Overview

Integrate Godot-origin executable settings and export presets as native Sim game task templates only when they directly support the target generative game engine. Platform packaging remains outside Sim unless a reviewed Sim dependency is explicitly accepted.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and G2 shared Sim game metadata are satisfied.
- Validation gate: executable settings, export preset parsing, task template, and missing-setup diagnostic tests pass.
- Handoff gate: missing executables, invalid presets, and unsupported target platforms produce actionable diagnostics.
- Completion gate: export work uses explicit external task integration, and platform packaging dependencies beyond task invocation require G7 dependency review.

## Dependency Waves

- W7 Deferred Godot-origin compatibility: executable and export task integration waits for G1 boundary, G2 metadata, and an explicit product-enabling dependency.

## Tasks

- [x] 1. Implement Godot executable settings and export task templates
  - Parse `export_presets.cfg`, resolve executable settings, and create native Sim export task records.
  - _Requirements: 1.1, 1.2, 2.1, 2.2_
  - _writes: crates/sim_game/src/export.rs, crates/sim_game/src/executable.rs, crates/sim_game/src/export_tests.rs_
