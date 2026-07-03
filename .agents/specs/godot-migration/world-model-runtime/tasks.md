# Implementation Plan: World Model Runtime

## Dependency Gates

- **Primary wave**: W3 World-model serving substrate
- **Prerequisite gates**: G0 Spec consistency, G3 Shared world-model foundations
- **Worker gates**: G4 Worker safety before real Python/GPU execution; G6 Provenance before importing generated outputs

## Tasks

- [ ] 1. Add world-model runtime request, control, session, and artifact types
  - Model LingBot/Wan request fields, WASD/IJKL controls, persistent sessions, and generated artifact provenance.
  - _Requirements: 1.1, 2.1, 3.1, 4.1_
  - _writes: crates/world_model/src/request.rs, crates/world_model/src/controls.rs, crates/world_model/src/session.rs, crates/world_model/src/artifact.rs_
