# Implementation Plan: Agentic Game Tools

## Dependency Gates

- **Primary wave**: W4 Authoring and graph UX; W5 Generation outputs and asset pipelines for generation tools
- **Prerequisite gates**: G0 Spec consistency, G3 Shared world-model foundations
- **Execution gates**: G5 Graph safety for graph edits; G6 Provenance for generated artifacts; G4 Worker safety before real model execution

## Tasks

- [ ] 1. Add graph, world generation, and mesh generation agent tools
  - Register validated tools for graph edits and typed generation requests.
  - _Requirements: 1.1, 2.1, 2.2_
  - _writes: crates/agent/src/tools/game_graph_tool.rs, crates/agent/src/tools/world_generation_tool.rs, crates/agent/src/tools/mesh_generation_tool.rs_
