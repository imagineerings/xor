# Implementation Plan: Godot Migration Umbrella

## Execution Gates

- **G0 Spec consistency**: all spec folders have requirements, design, and tasks files; every task declares requirement and write manifests.
- **G1 Boundary policy**: excluded Godot runtimes are encoded and tested before runtime-adjacent work.
- **G2 Shared Godot metadata**: project descriptors, diagnostics, source references, and fixture attribution exist before Godot integrations.
- **G3 Shared world-model foundations**: request/control/worker/graph/mesh/artifact/provenance models exist before world-model integrations.
- **G4 Worker safety**: Python/model/GPU/remote diagnostics exist before starting real model workers.
- **G5 Graph safety**: graph validator exists before executing graph nodes.
- **G6 Provenance**: generated artifact provenance exists before importing generated media/meshes/exports.
- **G7 Dependency review**: heavy/native/vendored dependencies are reviewed before implementation.

## Dependency Waves

| Wave | Tasks |
|---|---|
| W0 Planning validation | 2, 13 |
| W1 Shared foundations | 1, 3, 5, 6, 8, 9, 10, 11, 12 |
| W2 Godot compatibility substrate | grouped `engine-core-runtime`, `language-scripting`, `game-formats-assets`, `build-test-docs` metadata/docs tasks |
| W3 World-model serving substrate | grouped `world-model-runtime`, `model-serving-packaging`, generated-media routing |
| W4 Authoring and graph UX | grouped `diffusion-graph-editor`, `unified-authoring-app`, editor affordances, graph agent tools |
| W5 Generation outputs and asset pipelines | grouped `mesh-generation-pipeline`, generated mesh assets, generation agent tools |
| W6 External execution hardening | grouped `platform-export`, `networking-collaboration`, `xr-spatial`, `physics-navigation`, worker hardening |

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

## Notes

- Do not implement tasks that require Godot runtime execution before the external-command task provider exists.
- Do not implement tasks that require world-model execution before the worker protocol and serving diagnostics exist.
- Do not execute a task from a later dependency wave until the applicable earlier gates are satisfied.
- Do not add vendored Godot third-party code without updating the relevant design and passing dependency review.
