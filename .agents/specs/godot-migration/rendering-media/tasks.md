# Implementation Plan: Rendering and Media

## Overview

Route Godot media and generated outputs through existing Baymax preview/media systems. Generated media diagnostics start in W3, while generated asset previews finish in W5.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and applicable G3 shared world-model foundations are satisfied.
- Validation gate: media classification, unsupported-preview, generated-output routing, and provenance tests pass.
- Handoff gate: unsupported render/audio/text server features and missing previews produce explicit diagnostics.
- Completion gate: generated media import waits for G4 worker safety and G6 provenance, new codecs/native media/shader dependencies require G7 dependency review, and Comfy media node behavior references G8 Comfy harness alignment.

## Dependency Waves

- W3 World-model and Comfy serving substrate: generated media diagnostics and routing depend on G3 and G4.
- W5 Generation outputs and asset pipelines: generated-asset previews and artifact import depend on G6 provenance.

## Tasks

- [ ] 1. Add Godot media and generated-output preview routing
  - Classify media files, preserve unsupported reasons, and route generated media with provenance.
  - _Requirements: 1.1, 2.1, 2.2, 3.1_
  - _writes: crates/baymax_game/src/media.rs, crates/world_model/src/media_artifacts.rs, crates/baymax_game/src/media_tests.rs_
