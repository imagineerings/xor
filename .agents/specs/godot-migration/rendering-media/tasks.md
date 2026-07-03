# Implementation Plan: Rendering and Media

## Dependency Gates

- **Primary wave**: W3 World-model serving substrate for generated media; W5 Generation outputs and asset pipelines for generated-asset previews
- **Prerequisite gates**: G0 Spec consistency, G1 Boundary policy
- **Generated media gates**: G3 Shared world-model foundations, G4 Worker safety, G6 Provenance
- **Dependency gate**: G7 Dependency review for any new codec, native media, or shader dependency

## Tasks

- [ ] 1. Add Godot media and generated-output preview routing
  - Classify media files, preserve unsupported reasons, and route generated media with provenance.
  - _Requirements: 1.1, 2.1, 2.2, 3.1_
  - _writes: crates/godot/src/media.rs, crates/world_model/src/media_artifacts.rs, crates/godot/src/media_tests.rs_
