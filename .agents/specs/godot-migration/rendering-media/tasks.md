# Implementation Plan: Rendering and Media

## Overview

Route generated outputs through existing Sim preview/media systems first; Godot media classification is deferred W7 compatibility unless it unlocks the target product. Generated media diagnostics start in W2, while generated asset previews finish in W4.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, and applicable G3 shared world-model foundations are satisfied.
- Validation gate: media classification, unsupported-preview, generated-output routing, and provenance tests pass.
- Handoff gate: unsupported render/audio/text server features and missing previews produce explicit diagnostics.
- Completion gate: generated media import waits for G4 worker safety and G6 provenance, new codecs/native media/shader dependencies require G7 dependency review, and Comfy media node behavior references G8 Comfy harness alignment.

## Dependency Waves

- W2 Value-first world-model serving substrate: generated media diagnostics and routing depend on G3 and G4.
- W4 Generation outputs and asset pipelines: generated-asset previews and artifact import depend on G6 provenance.
- W7 Deferred Godot-origin compatibility: Godot media classification starts only when it directly supports import or preview of target-product assets.

## Tasks

- [ ] 1. Add Godot media and generated-output preview routing
  - Classify media files, preserve unsupported reasons, and route generated media with provenance.
  - _Requirements: 1.1, 2.1, 2.2, 3.1_
  - _writes: crates/sim_game/src/media.rs, crates/world_model/src/media_artifacts.rs, crates/sim_game/src/media_tests.rs_
