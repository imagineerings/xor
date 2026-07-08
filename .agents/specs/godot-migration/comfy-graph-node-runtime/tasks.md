# Implementation Plan: Comfy Graph and Node Runtime

## Overview

Implement runtime graph compatibility in layers: registry/schema first, validation and replacement next, then execution planning, caching, and artifact output. UI tasks stay in `diffusion-graph-editor/`.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations, G5 graph safety, and G8 Comfy harness alignment are satisfied.
- Validation gate: graph validation tests, cache policy tests, replacement tests, and object-info compatibility tests pass.
- Handoff gate: unsupported node classes have import diagnostics and are not silently exposed.
- Completion gate: invalid graphs cannot enqueue and valid graph plans are deterministic.

## Dependency Waves

- Global wave: W3 Comfy execution core.
- Local Wave 1: Tasks 1-2 define registry and schemas.
- Local Wave 2: Tasks 3-5 add validation, replacement, and planning.
- Local Wave 3: Tasks 6-7 add executor integration and compatibility fixtures.

## Tasks

- [x] 1. Implement Comfy node registry
  - Store node definitions, display metadata, categories, API-node markers, and disabled-node policy as native Sim registry records rather than forwarding object-info lookup to Comfy.
  - _Requirements: 1.1, 1.2, 1.3_
  - _writes: crates/world_model/src/comfy_nodes.rs, crates/world_model/src/comfy_nodes_tests.rs_

- [x] 2. Implement node schema adapter
  - Normalize Comfy required, optional, hidden, primitive, combo, list, and lazy input declarations into native Sim graph schema data rather than forwarding schema conversion to Comfy.
  - _Requirements: 1.1, 1.2_
  - _writes: crates/world_model/src/comfy_schema.rs, crates/world_model/src/comfy_schema_tests.rs_

- [x] 3. Implement node replacement engine
  - Apply old-to-new node mappings, input mappings, metadata key rewrites, and output link rewrites as native Sim graph transformations before validation rather than passing replacement handling through to Comfy.
  - _Requirements: 2.3_
  - _writes: crates/world_model/src/comfy_node_replacement.rs, crates/world_model/src/comfy_node_replacement_tests.rs_

- [ ] 4. Implement prompt graph validator
  - Validate node existence, required inputs, link indexes, type compatibility, cycles, and partial execution targets.
  - _Requirements: 2.1, 2.2, 5.3_
  - _writes: crates/world_model/src/comfy_graph_validation.rs, crates/world_model/src/comfy_graph_validation_tests.rs_

- [ ] 5. Implement execution planning and cache policy
  - Add dependency closure planning, dirty-node detection, RAM-pressure/classic/LRU/none cache policy models, and cache-key tests.
  - _Requirements: 3.1, 3.2, 3.3_
  - _writes: crates/world_model/src/comfy_execution_plan.rs, crates/world_model/src/comfy_cache.rs, crates/world_model/src/comfy_cache_tests.rs_

- [ ] 6. Implement node executor adapter
  - Support sync, async, list-mapped, interrupted, blocked, cached, and failed execution states with UI output preservation.
  - Dispatch sampler, scheduler, conditioning, VAE, latent, model patch, diffusion, and world-model execution nodes to `comfy-diffusion-world-model-runtime/`.
  - _Requirements: 3.4, 4.1, 4.2, 4.3, 5.2, 5.4_
  - _writes: crates/world_model/src/comfy_executor.rs, crates/world_model/src/comfy_executor_tests.rs_

- [ ] 7. Add core node compatibility fixtures
  - Add fixture prompts and object-info snapshots for core node categories.
  - _Requirements: 1.1, 1.2, 2.1, 3.4_
  - _writes: crates/world_model/fixtures/comfy/core_nodes.json, crates/world_model/tests/comfy_core_nodes.rs_

## Notes

- Graph canvas behavior belongs to `diffusion-graph-editor/`.
- Sampler, scheduler, conditioning, VAE, latent, model patch, diffusion, and world-model execution semantics belong to `comfy-diffusion-world-model-runtime/`.
- Python-backed node execution must wait for packaging and worker diagnostics.
