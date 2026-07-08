# Implementation Plan: World Model Runtime

## Overview

Model world generation requests, controls, sessions, and artifacts as the W2 value-first substrate for later workers, graphs, authoring UI, and generated output imports.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations, and G8 Comfy harness alignment for prompt, graph, sampler, scheduler, conditioning, model, asset, or runner decisions are satisfied.
- Validation gate: request, control, session, artifact, and provenance tests pass.
- Handoff gate: invalid controls, unavailable workers, and unsupported model profiles produce stable diagnostics.
- Completion gate: real Python/GPU execution waits for G4 worker safety, and importing generated outputs waits for G6 provenance.

## Dependency Waves

- W2 Value-first world-model serving substrate: request/control/session/artifact models depend on W1 world-model foundations and feed W3-W5 integrations.

## Tasks

- [ ] 1. Add world-model runtime request, control, session, and artifact types
  - Model LingBot/Wan request fields, WASD/IJKL controls, persistent sessions, and generated artifact provenance.
  - _Requirements: 1.1, 2.1, 3.1, 4.1_
  - _writes: crates/world_model/src/request.rs, crates/world_model/src/controls.rs, crates/world_model/src/session.rs, crates/world_model/src/artifact.rs_
