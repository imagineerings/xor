# Implementation Plan: Agentic Game Tools

## Overview

Register agent tools over existing graph, world-model, and mesh primitives after the W2-W4 harness can execute real product flows. Graph editing consumes W3 validation, while generation and import tools move in W5 after worker diagnostics and provenance exist.

## Gates

- Start gate: G0 spec consistency and G3 shared world-model foundations are satisfied.
- Validation gate: graph edit diff tests, typed generation request tests, and provenance import tests pass.
- Handoff gate: unsupported graph edits, unavailable workers, and missing provenance paths produce stable diagnostics.
- Completion gate: graph edits cannot apply without G5 graph validation, generation cannot execute without G4 worker safety, generated artifacts cannot import without G6 provenance, and Comfy-backed graph or generation behavior references G8 Comfy harness alignment.

## Dependency Waves

- W5 Product authoring and agentic tools: graph editing tools can start after G3 and G5 are satisfied.
- W5 Product authoring and agentic tools: world and mesh generation tools depend on W2 worker diagnostics, W4 artifact pipelines, and G6 provenance.

## Tasks

- [ ] 1. Add graph, world generation, and mesh generation agent tools
  - Register validated tools for graph edits and typed generation requests.
  - _Requirements: 1.1, 2.1, 2.2_
  - _writes: crates/agent/src/tools/game_graph_tool.rs, crates/agent/src/tools/world_generation_tool.rs, crates/agent/src/tools/mesh_generation_tool.rs_
