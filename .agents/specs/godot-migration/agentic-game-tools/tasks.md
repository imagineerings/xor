# Implementation Plan: Agentic Game Tools

## Overview

Register agent tools over existing graph, world-model, and mesh primitives. Graph editing belongs to W4, while generation and import tools move in W5 after worker diagnostics and provenance exist.

## Gates

- Start gate: G0 spec consistency and G3 shared world-model foundations are satisfied.
- Validation gate: graph edit diff tests, typed generation request tests, and provenance import tests pass.
- Handoff gate: unsupported graph edits, unavailable workers, and missing provenance paths produce stable diagnostics.
- Completion gate: graph edits cannot apply without G5 graph validation, generation cannot execute without G4 worker safety, generated artifacts cannot import without G6 provenance, and Comfy-backed graph or generation behavior references G8 Comfy harness alignment.

## Dependency Waves

- W4 Authoring, graph UX, and Comfy workflows: graph editing tools can start after G3 and G5 are satisfied.
- W5 Generation outputs and asset pipelines: world and mesh generation tools depend on W3 worker diagnostics and G6 provenance.

## Tasks

- [ ] 1. Add graph, world generation, and mesh generation agent tools
  - Register validated tools for graph edits and typed generation requests.
  - _Requirements: 1.1, 2.1, 2.2_
  - _writes: crates/agent/src/tools/game_graph_tool.rs, crates/agent/src/tools/world_generation_tool.rs, crates/agent/src/tools/mesh_generation_tool.rs_
