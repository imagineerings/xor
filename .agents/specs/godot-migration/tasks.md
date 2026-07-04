# Implementation Plan: Godot Migration Umbrella

## Gates

- Start gate: G0 spec consistency passes for the umbrella and every grouped spec, and the selected task's dependency wave prerequisites are satisfied.
- Validation gate: requirement references resolve, design properties validate existing requirements, `_writes:` manifests are present, and any task-specific tests named by the grouped spec pass.
- Handoff gate: the next implementer receives the task's wave, required gates, expected writes, known write conflicts, and unsupported/delegated behavior notes.
- Completion gate: no task is marked complete until the full consistency pass reconciles `requirements.md`, `design.md`, `tasks.md`, and `master-migration-plan.md`.

## Execution Gate Catalog

- **G0 Spec consistency**: all spec folders have requirements, design, and tasks files; every task declares requirement and write manifests; every task reference and design property target resolves to an existing acceptance criterion.
- **G1 Boundary policy**: excluded Godot runtimes are encoded and tested before runtime-adjacent work.
- **G2 Shared Godot metadata**: project descriptors, diagnostics, source references, and fixture attribution exist before Godot integrations.
- **G3 Shared world-model foundations**: request/control/worker/graph/mesh/artifact/provenance models exist before world-model integrations.
- **G4 Worker safety**: Python/model/GPU/remote diagnostics exist before starting real model workers.
- **G5 Graph safety**: graph validator exists before executing graph nodes.
- **G6 Provenance**: generated artifact provenance exists before importing generated media/meshes/exports.
- **G7 Dependency review**: heavy/native/vendored dependencies are reviewed before implementation.
- **G8 Comfy harness alignment**: world-model harness tasks involving prompts, graphs, samplers, schedulers, conditioning, diffusion/world-model execution, models, assets, media nodes, providers, or extensions reference the applicable Comfy spec or an explicit safety/security/dependency divergence.

## Dependency Waves

| Wave | Tasks |
|---|---|
| W0 Planning validation | Spec documents only; no code task starts until G0 passes |
| W1 Shared foundations | Umbrella tasks 1 -> 8 serially for `Cargo.toml`; after task 1, tasks 2, 3, 5, 6, 13, and 14, with 2 -> 13 -> 14 serial for inventory/gatekeeper writes; after task 8, tasks 9, 10, 11, and 12 |
| W2 Godot compatibility substrate | Umbrella tasks 4 and 7 after G1/G2; grouped `engine-core-runtime`, `language-scripting`, `game-formats-assets`, and `build-test-docs` metadata/docs work |
| W3 World-model and Comfy serving substrate | grouped `world-model-runtime`, `model-serving-packaging`, `comfy-model-memory-runtime`, W3 portions of `comfy-packaging-quality`, and generated-media diagnostics/routing |
| W4 Authoring, graph UX, and Comfy workflows | grouped `diffusion-graph-editor`, `unified-authoring-app`, `comfy-runtime-control-plane`, `comfy-graph-node-runtime`, `comfy-diffusion-world-model-runtime`, W4 portions of `comfy-workflows-blueprints`, editor affordances, and graph agent tools |
| W5 Generation outputs and asset pipelines | grouped `mesh-generation-pipeline`, `comfy-asset-library`, `comfy-media-node-pipelines`, W5 portions of `comfy-workflows-blueprints`, generated mesh assets, generated workflow metadata, and generation agent tools |
| W6 External execution hardening | grouped `platform-export`, `networking-collaboration`, `xr-spatial`, `physics-navigation`, `comfy-api-provider-nodes`, `comfy-extension-ecosystem`, W6 portions of `comfy-packaging-quality`, and real worker/export/debug hardening |

## Tasks

- [ ] 1. Create the shared Godot support crate
  - Add metadata, source-reference, boundary-decision, and project descriptor types.
  - _Requirements: 2.1, 2.2, 3.1_
  - _writes: Cargo.toml, crates/godot/Cargo.toml, crates/godot/src/godot.rs, crates/godot/src/migration.rs, crates/godot/src/tests.rs_

- [ ] 2. Implement migration inventory validation
  - Add validation for top-level source areas and grouped spec coverage.
  - _Requirements: 1.1, 1.2, 1.3, 12.4_
  - _writes: crates/godot/src/inventory.rs, crates/godot/src/inventory_tests.rs_

- [ ] 3. Implement the runtime boundary policy
  - Encode metadata-only, Baymax-adapter, external-command, and excluded scopes.
  - _Requirements: 2.1, 2.2, 2.3_
  - _writes: crates/godot/src/boundary.rs, crates/godot/src/boundary_tests.rs_

- [ ] 4. Add Godot workspace integration registration
  - Connect Godot detection, tasks, language support, docs, and previews to existing Baymax registries.
  - _Requirements: 3.1, 3.2, 3.3, 4.1_
  - _writes: crates/godot/src/integration.rs, crates/baymax/src/baymax.rs, crates/godot/src/integration_tests.rs_

- [ ] 5. Add shared parser and diagnostics primitives
  - Define diagnostics, source ranges, parse status, and recoverable-error helpers.
  - _Requirements: 3.2, 10.1_
  - _writes: crates/godot/src/diagnostics.rs, crates/godot/src/parser.rs, crates/godot/src/parser_tests.rs_

- [ ] 6. Add fixture attribution support
  - Implement fixture metadata records for copied or converted Godot and world-model fixtures.
  - _Requirements: 11.2_
  - _writes: crates/godot/src/fixtures.rs, crates/godot/src/fixtures_tests.rs_

- [ ] 7. Add umbrella smoke tests
  - Exercise detection, metadata routing, boundary decisions, and registration together.
  - _Requirements: 3.1, 3.2, 3.3, 10.3_
  - _writes: crates/godot/src/smoke_tests.rs_

- [ ] 8. Create the shared world-model support crate
  - Add request, control, worker, graph, mesh, artifact, and provenance types.
  - _Requirements: 4.2, 5.1, 5.4, 7.2_
  - _writes: Cargo.toml, crates/world_model/Cargo.toml, crates/world_model/src/world_model.rs, crates/world_model/src/request.rs, crates/world_model/src/provenance.rs, crates/world_model/src/tests.rs_

- [ ] 9. Add world-model action/control compatibility
  - Port WASD/IJKL action-string parsing semantics from `projects/world-model`.
  - _Requirements: 5.1, 5.2_
  - _writes: crates/world_model/src/controls.rs, crates/world_model/src/controls_tests.rs_

- [ ] 10. Add diffusion graph primitives
  - Define typed graph nodes, edges, validation errors, artifact outputs, and execution IDs.
  - _Requirements: 6.1, 6.2, 6.3_
  - _writes: crates/world_model/src/graph.rs, crates/world_model/src/graph_validation.rs, crates/world_model/src/graph_tests.rs_

- [ ] 11. Add mesh generation primitives
  - Define mesh generation request, backend, preview, export target, and generated asset metadata.
  - _Requirements: 7.1, 7.2, 7.3_
  - _writes: crates/world_model/src/mesh.rs, crates/world_model/src/mesh_tests.rs_

- [ ] 12. Add model serving diagnostics
  - Define Python environment, model weight, GPU, local worker, and remote worker diagnostics.
  - _Requirements: 5.3, 9.1, 9.2, 9.3_
  - _writes: crates/world_model/src/serving.rs, crates/world_model/src/serving_diagnostics.rs, crates/world_model/src/serving_tests.rs_

- [ ] 13. Add migration gatekeeper validation
  - Validate spec completeness, task manifests, gate decisions, and wave progression.
  - _Requirements: 10.1, 10.2, 10.3, 12.1, 12.2, 12.3, 12.4_
  - _writes: crates/godot/src/spec_gatekeeper.rs, crates/godot/src/spec_gatekeeper_tests.rs_

- [ ] 14. Extend migration inventory validation for Comfy specs
  - Validate the ten Comfy harness specs are present, non-overlapping, and treated as core world-model harness owners for prompt, graph, sampler, scheduler, conditioning, diffusion/world-model execution, model, asset, media-node, provider, extension, and packaging behavior.
  - Fail validation when a world-model harness task adds prompt, graph, sampler, scheduler, conditioning, diffusion/world-model execution, model, asset, media-node, provider, or extension behavior without an applicable Comfy spec reference or explicit safety/security/dependency divergence.
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 13.1, 13.2, 13.3, 13.4, 13.5, 13.6_
  - _writes: crates/godot/src/spec_gatekeeper.rs, crates/godot/src/spec_gatekeeper_tests.rs, crates/godot/src/inventory_tests.rs_

## Notes

- Do not implement tasks that require Godot runtime execution before the external-command task provider exists.
- Do not implement tasks that require world-model execution before the worker protocol and serving diagnostics exist.
- Do not execute a task from a later dependency wave until the applicable earlier gates are satisfied.
- Do not run tasks with overlapping `_writes:` manifests in parallel unless the grouped spec explicitly marks the overlap as an extension of an already-completed foundation task.
- Do not add vendored Godot third-party code without updating the relevant design and passing dependency review.
- Do not treat Comfy as optional UI compatibility for the world-model harness; use the applicable Comfy spec as the functional starting point for prompt, graph, sampler, scheduler, conditioning, diffusion/world-model execution, model, asset, media-node, provider, and extension behavior.
- Do not add a parallel Comfy subsystem when an existing Baymax or migration spec already owns the underlying runtime behavior; add a harness adapter and delegation instead.
