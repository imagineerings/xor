# Implementation Plan: Diffusion Graph Editor

## Dependency Gates

- **Primary wave**: W4 Authoring and graph UX
- **Prerequisite gates**: G0 Spec consistency, G3 Shared world-model foundations
- **Graph gates**: G5 Graph safety before node execution; G6 Provenance before artifact import

## Tasks

- [ ] 1. Add graph model, validation, editor state, and execution runner
  - Define graph primitives, validation, editor-facing state, and execution plan outputs.
  - _Requirements: 1.1, 2.1, 3.1_
  - _writes: crates/world_model/src/graph.rs, crates/world_model/src/graph_validation.rs, crates/baymax_apps/src/diffusion_graph.rs_
