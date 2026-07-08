# Implementation Plan: Diffusion Graph Editor

## Overview

Build editor state and execution wiring on top of the shared world-model graph primitives. Graph execution planning belongs to W3, while product UI consumption follows in W5 after graph safety and provenance gates are satisfied.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations, and applicable G8 Comfy harness alignment decisions are satisfied.
- Validation gate: graph model, graph validation, editor state, and execution-plan tests pass.
- Handoff gate: invalid graphs, unavailable backends, and unsupported Comfy-compatible nodes are visible as diagnostics.
- Completion gate: graph execution cannot enqueue without G5 graph safety, and artifact import cannot complete without G6 provenance.

## Dependency Waves

- W3 Comfy execution core: editor state and graph execution planning depend on W1 world-model graph primitives and G5 graph safety.

## Tasks

- [ ] 1. Add graph model, validation, editor state, and execution runner
  - Define graph primitives, validation, editor-facing state, and execution plan outputs.
  - _Requirements: 1.1, 2.1, 3.1_
  - _writes: crates/world_model/src/graph.rs, crates/world_model/src/graph_validation.rs, crates/sim_apps/src/diffusion_graph.rs_
