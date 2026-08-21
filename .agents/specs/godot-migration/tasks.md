# Implementation Plan: Zed Game Development Surface

## Gates

- Start gate: G0 spec consistency passes for the umbrella and every grouped spec, and the selected task's dependency wave prerequisites are satisfied.
- Validation gate: requirement references resolve, design properties validate existing requirements, `_Writes:` manifests are present, and any task-specific tests named by the grouped spec pass.
- Handoff gate: the next implementer receives the task's wave, required gates, expected writes, known write conflicts, and unsupported/delegated behavior notes.
- Completion gate: no task is marked complete until the full consistency pass reconciles `requirements.md`, `design.md`, `tasks.md`, and `master-migration-plan.md`.

## Execution Gate Catalog

- **G0 Spec consistency**: all spec folders have requirements, design, and tasks files; every task declares requirement and write manifests; every task reference and design property target resolves to an existing acceptance criterion.
- **G1 Boundary policy**: excluded Godot runtimes are encoded and tested before runtime-adjacent work.
- **G2 Shared Zed game metadata**: project descriptors, diagnostics, source references, and fixture attribution exist before game integrations.
- **G3 Shared world-model foundations**: request/control/worker/graph/mesh/artifact/provenance models exist before world-model integrations.
- **G4 Worker safety**: Python/model/GPU/remote diagnostics exist before starting real model workers.
- **G5 Graph safety**: graph validator exists before executing graph nodes.
- **G6 Provenance**: generated artifact provenance exists before importing generated media/meshes/exports.
- **G7 Dependency review**: heavy/native/vendored dependencies are reviewed before implementation.
- **G8 Comfy harness alignment**: world-model harness tasks involving prompts, graphs, samplers, schedulers, conditioning, diffusion/world-model execution, models, assets, media nodes, providers, or extensions reference the applicable Comfy spec or an explicit safety/security/dependency divergence.
- **G9 Native Comfy recreation**: every supported Comfy-derived feature is implemented as native Zed functionality with Zed records, services, workers, artifacts, provenance, and diagnostics rather than a compatibility label or ComfyUI pass-through.
- **G11 Native Godot independence**: every supported or fully specified Godot-origin capability names existing/proposed Zed owners and passes hermetic no-Godot process, package, loader, dependency, storage, execution, UI, and lifecycle validation.

## Dependency Waves

| Wave | Tasks |
|---|---|
| W0 Planning validation | Spec documents only; no code task starts until G0 passes |
| W1 Shared foundations | Umbrella tasks 1 -> 14 are complete and remain prerequisites for new grouped work |
| W2 Value-first world-model serving substrate | grouped `world-model-runtime`, `model-serving-packaging`, `comfy-model-memory-runtime`, W2 portions of `comfy-packaging-quality`, and generated-media diagnostics/routing |
| W3 Comfy execution core | grouped `comfy-runtime-control-plane`, `comfy-graph-node-runtime`, `comfy-diffusion-world-model-runtime`, W3 portions of `comfy-workflows-blueprints`, and `diffusion-graph-editor` execution/validation work |
| W4 Generation outputs and asset pipelines | grouped `mesh-generation-pipeline`, `comfy-asset-library`, `comfy-media-node-pipelines`, W4 portions of `comfy-workflows-blueprints`, generated mesh/media routing, and generated asset provenance |
| W5 Product authoring and agentic tools | grouped `agentic-game-tools`, `unified-authoring-app`, native SimScript/natural-language authoring work, and editor affordances that consume W2-W4 capabilities |
| W6 Comfy provider, extension, and packaging hardening | grouped `comfy-api-provider-nodes`, `comfy-extension-ecosystem`, W6 portions of `comfy-packaging-quality`, remote/persistent worker hardening, and provider policy gates |
| W7 Deferred Godot-origin compatibility | grouped `engine-core-runtime`, legacy parts of `language-scripting`, legacy `game-formats-assets`, `platform-export`, `networking-collaboration`, `xr-spatial`, `physics-navigation`, and Godot run/debug/export/editor tasks; start only when they unblock W2-W6 product work |

## Selection Policy

- After W1, select W2-W4 Comfy and world-model harness tasks before Godot-format/runtime/editor/export tasks.
- Select W5 product authoring and agentic tools once they can consume W2-W4 substrate instead of building placeholder UI around missing execution paths.
- Select W6 provider, extension, and packaging hardening after local Comfy/world-model execution paths have their safety and provenance gates, unless a W6 policy gate blocks W2-W4 work.
- Select W7 deferred Godot-origin compatibility only when the task directly unlocks the native Zed target product, such as importing a required source project, exposing generated assets, or supporting SimScript/natural-language authoring.
- Treat legacy `.gd` files and Godot-format assets as migration/import sources; native authoring uses natural language as the interface and SimScript as the executable language.

## Tasks

- [ ] 1. Extend existing Zed owners for game project metadata
  - Add only the required project metadata, source-reference, diagnostics, and project descriptor behavior at existing project, worktree, language, and diagnostics integration points.
  - _Requirements: 2.1, 2.2, 2.3, 3.1, 15.1, 15.2, 15.3, 15.4, 15.7, 15.9_
  - _Writes: crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/language/src/language_registry.rs, crates/diagnostics/src/diagnostics.rs_
  - _Depends on: none_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 2. Maintain migration inventory validation in the specification tooling
  - Add validation for top-level source areas and grouped spec coverage.
  - _Requirements: 1.1, 1.2, 1.3, 12.4_
  - _Writes: .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv, .agents/specs/godot-migration/godot-full-port-coverage/validate_audit.py_
  - _Depends on: 1_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 3. Implement the runtime boundary policy
  - Encode native-owner, unresolved, intentionally-excluded, and upstream-only scopes in the coverage catalog; do not introduce a product runtime-policy registry.
  - _Requirements: 2.1, 2.2, 2.3, 15.1, 15.2, 15.4, 15.7, 15.9, 15.10_
  - _Writes: .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv, .agents/specs/godot-migration/godot-full-port-coverage/decisions.md, .agents/specs/godot-migration/godot-full-port-coverage/findings.md_
  - _Depends on: 2_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 4. Add game workspace integration registration
  - Wire game project detection, Zed-owned task actions, SimScript language registration data, docs routes, and preview routing directly to existing Zed registries.
  - _Requirements: 3.1, 3.2, 3.3, 4.1, 15.1, 15.2, 15.3, 15.4, 15.7, 15.9_
  - _Writes: crates/project/src/project.rs, crates/workspace/src/workspace.rs, crates/languages/src/lib.rs, crates/task/src/task.rs, crates/image_viewer/src/image_viewer.rs_
  - _Depends on: 3_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: cargo test -p zed_
- [ ] 5. Add shared parser and diagnostics primitives
  - Extend existing project/worktree/diagnostics types with required source ranges, parse status, and recoverable errors rather than adding a parallel parser framework.
  - _Requirements: 3.2, 10.1, 15.1, 15.2, 15.3, 15.4, 15.5, 15.7, 15.9_
  - _Writes: crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/diagnostics/src/diagnostics.rs_
  - _Depends on: 4_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 6. Add fixture attribution support
  - Extend existing compliance/test tooling for approved copied or converted Godot/world-model fixtures; keep every unapproved Godot copy blocked.
  - _Requirements: 11.1, 11.2, 15.2, 15.8, 15.10_
  - _Writes: tooling/compliance/src/lib.rs, .agents/specs/godot-migration/godot-full-port-coverage/decisions.md, .agents/specs/godot-migration/godot-full-port-coverage/findings.md_
  - _Depends on: 5_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 7. Add umbrella smoke tests
  - Exercise detection, metadata routing, native-owner decisions, registration, unsupported states, and no-Godot independence together.
  - _Requirements: 3.1, 3.2, 3.3, 10.3, 15.1, 15.2, 15.3, 15.4, 15.5, 15.6, 15.7, 15.9_
  - _Writes: crates/project/tests/integration/project_tests.rs, crates/workspace/src/workspace.rs, crates/task/src/task.rs_
  - _Depends on: 6_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 8. Create the shared world-model support crate
  - Add request, control, worker, graph, mesh, artifact, and provenance types.
  - _Requirements: 4.2, 5.1, 5.4, 7.2_
  - _Writes: Cargo.toml, crates/world_model/Cargo.toml, crates/world_model/src/world_model.rs, crates/world_model/src/request.rs, crates/world_model/src/provenance.rs, crates/world_model/src/tests.rs_
  - _Depends on: 7_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 9. Add world-model action/control compatibility
  - Port WASD/IJKL action-string parsing semantics from `projects/world-model`.
  - _Requirements: 5.1, 5.2_
  - _Writes: crates/world_model/src/controls.rs, crates/world_model/src/controls_tests.rs_
  - _Depends on: 8_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 10. Add diffusion graph primitives
  - Define typed graph nodes, edges, validation errors, artifact outputs, and execution IDs.
  - _Requirements: 6.1, 6.2, 6.3_
  - _Writes: crates/world_model/src/graph.rs, crates/world_model/src/graph_validation.rs, crates/world_model/src/graph_tests.rs_
  - _Depends on: 9_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 11. Add mesh generation primitives
  - Define mesh generation request, backend, preview, export target, and generated asset metadata.
  - _Requirements: 7.1, 7.2, 7.3_
  - _Writes: crates/world_model/src/mesh.rs, crates/world_model/src/mesh_tests.rs_
  - _Depends on: 10_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 12. Add model serving diagnostics
  - Define Python environment, model weight, GPU, local worker, and remote worker diagnostics.
  - _Requirements: 5.3, 9.1, 9.2, 9.3_
  - _Writes: crates/world_model/src/serving.rs, crates/world_model/src/serving_diagnostics.rs, crates/world_model/src/serving_tests.rs_
  - _Depends on: 11_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 13. Add migration gatekeeper validation
  - Validate spec completeness, task manifests, gate decisions, and wave progression.
  - _Requirements: 10.1, 10.2, 10.3, 12.1, 12.2, 12.3, 12.4_
  - _Writes: .agents/specs/godot-migration/godot-full-port-coverage/validate_audit.py, .agents/specs/godot-migration/godot-full-port-coverage/validation-results.md_
  - _Depends on: 12_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_
- [ ] 14. Extend migration inventory validation for Comfy specs
  - Validate the ten Comfy harness specs are present, non-overlapping, and treated as core world-model harness owners for prompt, graph, sampler, scheduler, conditioning, diffusion/world-model execution, model, asset, media-node, provider, extension, and packaging behavior.
  - Fail validation when a world-model harness task adds prompt, graph, sampler, scheduler, conditioning, diffusion/world-model execution, model, asset, media-node, provider, or extension behavior without an applicable Comfy spec reference or explicit safety/security/dependency divergence.
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 13.1, 13.2, 13.3, 13.4, 13.5, 13.6, 4.3, 6.4, 8.1, 8.2, 8.3, 11.1, 11.3, 12.5, 13.7, 13.8, 14.1, 14.2, 14.3, 14.4_
  - _Writes: .agents/specs/godot-migration/comfy-full-port-coverage/catalogs/authoritative-ownership.csv, .agents/specs/godot-migration/comfy-full-port-coverage/tasks.md, .agents/specs/godot-migration/godot-full-port-coverage/findings.md_
  - _Depends on: 13_
  - _Reads: /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/requirements.md, /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration/design.md, Cargo.toml, projects/godot_
  - _Validation: python3 .agents/skills/feature-spec/scripts/validate_spec.py /Users/ahmad.vegah/repos/zed/.agents/specs/godot-migration_

- [ ] 15. Enforce native Zed ownership for every Godot-origin capability
  - Audit catalog rows and owner specs for Godot embedding, invocation, linkage, wrapping, hidden instances, source copying, duplicate owners, placeholder-only support, external delegation, and missing no-Godot evidence; keep material decisions unresolved.
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 15.5, 15.6, 15.7, 15.8, 15.9, 15.10_
  - _Depends on: 14_
  - _Reads: .agents/specs/godot-migration/**, Cargo.toml, Cargo.lock, deny.toml, projects/godot/COPYRIGHT.txt, projects/godot/thirdparty/README.md_
  - _Writes: .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv, .agents/specs/godot-migration/godot-full-port-coverage/findings.md, .agents/specs/godot-migration/godot-full-port-coverage/decisions.md, .agents/specs/godot-migration/godot-full-port-coverage/validation-results.md_
  - _Validation: validate all feature-spec packs and the audit; assert zero checked tasks; run supported scenarios and exported artifacts on a clean machine without Godot; inspect packages, processes, loaders, dependencies, network delegation, storage, UI, and lifecycle_

## Notes

- Do not implement or classify as supported any task that requires Godot runtime execution; keep it unresolved, intentionally excluded, or architecture-decision blocked until Zed owns execution.
- Do not implement tasks that require world-model execution before the worker protocol and serving diagnostics exist.
- Do not execute a task from a later dependency wave until the applicable earlier gates are satisfied.
- Do not select W7 deferred Godot-origin compatibility while W2-W6 Comfy/world-model product work is available unless the W7 task has an explicit product-enabling dependency note.
- Do not run tasks with overlapping `_Writes:` manifests in parallel unless the grouped spec explicitly marks the overlap as an extension of an already-completed foundation task.
- Do not copy or vendor Godot source, generated code, third-party code, fixtures, docs, or assets without separate licensing and architecture approval for the exact material.
- Do not treat Comfy as optional UI compatibility for the world-model harness; use the applicable Comfy spec as the functional starting point for prompt, graph, sampler, scheduler, conditioning, diffusion/world-model execution, model, asset, media-node, provider, and extension behavior.
- Do not add a parallel Comfy subsystem when an existing Zed or migration spec already owns the underlying runtime behavior; add a harness adapter and delegation instead.
- Do not create an intermediate abstraction layer (registrar trait, parallel type hierarchy) between game features and Zed registries. Wire directly into Zed's native registries.
