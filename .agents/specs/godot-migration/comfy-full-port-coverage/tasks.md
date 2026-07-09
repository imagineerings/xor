# Implementation Plan: Comfy Full Port Coverage

## Overview

This plan adds a coverage ledger and gatekeeper extension around the existing Comfy migration specs, then turns every missing coverage record into an owner-specific implementation task. The coverage work does not port a second Comfy runtime and does not replace the existing ten Comfy implementation specs. Instead, it gives implementation tasks a complete source inventory, one-owner mapping, parity evidence requirements, and concrete port-backlog tasks for missing functionality.

The dependency waves start with committed inventory data, then add ledger validation, then wire task gates and fixture checks. Tasks that refresh inventory from an external Comfy checkout are separated from normal CI so day-to-day validation does not depend on `/Users/ahmad.vegah/repos/projects/sim/projects/comfy`.

## Gates

- Start gate: the selected task must identify whether it edits inventory fixtures, coverage ledger fixtures, gatekeeper code, owner spec manifests, native Sim implementation modules, or Comfy parity fixtures; tasks touching Comfy-derived implementation must reference coverage IDs and the coverage owner.
- Validation gate: run `cargo test -p sim_game -p world_model comfy` or a narrower package-specific test named by the task; fixture-only changes must pass the coverage fixture validation tests, and owner-specific port tasks must pass their owner fixture or module tests.
- Handoff gate: hand off changed source inventory counts, coverage owner changes, unsupported/divergent records, and any newly discovered source paths.
- Completion gate: no task is complete until every new inventory item has one owner, implemented items have evidence, unsupported/divergent items have reasons, missing records have owner-specific implementation tasks, and no fixture marks ComfyUI pass-through as implemented behavior.

## Dependency Waves

- Wave 1: Tasks 1-2 create the source inventory schema and committed fixture snapshot.
- Wave 2: Tasks 3-5 add ownership ledger types, owner suggestions, and validation.
- Wave 3: Tasks 6-7 wire task gates and parity fixture validation.
- Wave 4: Tasks 8-9 reconcile existing Comfy specs and add coverage regression tests.
- Wave 5: Task 10 materializes the missing-functionality backlog into owner specs.
- Wave 6: Tasks 11-17 port missing local execution, model, asset, workflow, and media functionality through native Sim owners.
- Wave 7: Tasks 18-20 port missing provider, extension, and packaging functionality when gates allow.
- Wave 8: Task 21 closes coverage after owner tasks land; Task 22 adds the optional refresh tool for future source-tree updates.

## Tasks

- [x] 1. Add Sim source inventory schema and fixture model
  - Define typed source-kind, extraction-status, inventory-summary, source-item, and extraction-diagnostic records.
  - Add serde round-trip tests for representative route, node, model, blueprint, asset, extension, provider, CLI, OpenAPI, and packaging items.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 6.1_
  - _writes: crates/world_model/src/sim_source_inventory.rs, crates/world_model/src/world_model.rs, crates/world_model/src/sim_source_inventory_tests.rs_

- [x] 2. Commit the initial Sim source inventory fixture
  - Add a metadata-only inventory snapshot based on the current `projects/comfy` tree: route/API surfaces, `786` unique node IDs, `210` API-provider nodes, `121` extra node modules, `36` API-node modules, `89` blueprint JSONs, model-family classes, folder categories, CLI flags, OpenAPI operations, tests, and packaging surfaces.
  - Preserve source paths and mark unparsed or ambiguous areas as unclassified with diagnostics rather than dropping them.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 5.1, 5.2_
  - _writes: crates/world_model/fixtures/comfy/source_inventory.json, crates/world_model/tests/sim_source_inventory.rs_

- [x] 3. Add Sim coverage ledger data model
  - Define coverage owner, status, boundary decision, evidence reference, dependency gate, and diagnostic models.
  - Add validation for missing owners, duplicate owners, implemented-without-evidence, unsupported-without-reason, and invalid owner paths.
  - _Requirements: 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 5.1, 5.4_
  - _writes: crates/sim_game/src/sim_coverage.rs, crates/sim_game/src/sim_game.rs, crates/sim_game/src/sim_coverage_tests.rs_

- [x] 4. Implement owner suggestion rules for existing Comfy specs
  - Add deterministic mapping rules for runtime control plane, graph/node runtime, model/memory runtime, diffusion/world-model runtime, assets, workflows/blueprints, media node pipelines, provider nodes, extension ecosystem, and packaging quality.
  - Test representative source items for every requirement 4 owner rule.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 4.10_
  - _writes: crates/sim_game/src/sim_coverage.rs, crates/sim_game/src/sim_coverage_tests.rs_

- [x] 5. Commit the initial coverage ledger fixture
  - Map every source inventory item to one owner or an existing-Sim subsystem delegation.
  - Mark unsupported/divergent items with user-visible reasons and dependency-gate references where needed.
  - Validate that all currently implemented Comfy fixtures use `native_sim_records: true` and `comfyui_passthrough: false`.
  - _Requirements: 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 3.4, 6.3, 6.4_
  - _writes: crates/world_model/fixtures/comfy/coverage_ledger.json, crates/sim_game/src/sim_coverage_tests.rs_

- [x] 6. Extend the migration gatekeeper with Sim coverage checks
  - Make G8/G9 validation load the committed inventory and coverage ledger.
  - Fail on missing coverage, duplicate owners, unsupported records without reasons, implemented records without evidence, and task/spec owner mismatches.
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 7.1, 7.2, 7.3, 7.4_
  - _writes: crates/sim_game/src/spec_gatekeeper.rs, crates/sim_game/src/spec_gatekeeper_tests.rs, crates/sim_game/src/inventory.rs_

- [x] 7. Add parity fixture safety validation
  - Validate source attribution, `native_sim_records`, `comfyui_passthrough`, dependency-review metadata, and unsupported/divergent diagnostics across Comfy fixtures.
  - Add regression tests for provider/API-key fixtures, model-download fixtures, media-codec fixtures, and mock-runner fixtures.
  - _Requirements: 3.1, 3.2, 3.3, 6.1, 6.2, 6.3, 6.4_
  - _writes: crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/tests/sim_source_inventory.rs, crates/world_model/fixtures/comfy/*.json, crates/world_model/fixtures/comfy/README.md_

- [x] 8. Reconcile existing Comfy specs with coverage owners
  - Add owner references or coverage IDs to existing Comfy task manifests where implementation tasks touch source areas represented in the ledger.
  - Ensure each existing spec delegates overlapping behavior rather than claiming duplicate ownership.
  - _Requirements: 2.1, 2.2, 2.3, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 4.10, 7.1, 7.2_
  - _writes: .agents/specs/godot-migration/comfy-*/tasks.md, .agents/specs/godot-migration/master-migration-plan.md, crates/sim_game/src/spec_gatekeeper.rs, crates/sim_game/src/spec_gatekeeper_tests.rs_

- [x] 9. Add coverage regression tests for product sequencing
  - Verify uncovered local execution and authoring parity gaps rank ahead of provider, extension, packaging, and legacy hardening gaps unless a gate reason exists.
  - Verify Comfy/Godot/world-model overlaps choose the least-duplicative native Sim owner.
  - _Requirements: 8.1, 8.2, 8.3_
  - _writes: crates/sim_game/src/sim_coverage.rs, crates/sim_game/src/sim_coverage_tests.rs, crates/sim_game/src/spec_gatekeeper_tests.rs_

- [x] 10. Materialize owner-specific missing functionality tasks
  - Read the coverage ledger and generate implementation backlog entries for every source item with `Planned`, product-approved `Unsupported`, `Divergent`, or missing-evidence status.
  - Update the owning Comfy spec task manifest with coverage IDs, expected native Sim writes, validation commands, fixture requirements, and prerequisite foundation tasks where needed.
  - _Requirements: 5.1, 5.4, 7.1, 7.2, 8.1, 9.1, 9.2, 9.3_
  - _writes: crates/sim_game/src/sim_coverage.rs, crates/sim_game/src/sim_game.rs, crates/sim_game/src/sim_coverage_tests.rs, .agents/specs/godot-migration/comfy-*/tasks.md, crates/world_model/fixtures/comfy/coverage_ledger.json_

- [x] 11. Port missing runtime control-plane functionality
  - For coverage records assigned to `comfy-runtime-control-plane`, implement missing prompt, queue, history, jobs, features, upload/view, preview, route, WebSocket, event, and safety behavior through native Sim protocol, task, media, and artifact modules.
  - Add or update compatibility fixtures and mark each completed coverage record implemented, delegated, unsupported, or divergent with evidence.
  - _Requirements: 4.1, 5.4, 7.1, 7.3, 9.1, 9.2, 9.4_
  - _writes: .agents/specs/godot-migration/comfy-runtime-control-plane/tasks.md, crates/world_model/src/comfy_control.rs, crates/world_model/src/comfy_routes.rs, crates/world_model/src/world_model.rs, crates/world_model/src/sim_api_schema.rs, crates/world_model/src/comfy_control_tests.rs, crates/world_model/src/comfy_routes_tests.rs, crates/world_model/tests/comfy_api_compat.rs, crates/world_model/fixtures/comfy/*.json, crates/world_model/fixtures/comfy/coverage_ledger.json_

- [x] 12. Port missing graph and node runtime functionality
  - For coverage records assigned to `comfy-graph-node-runtime`, implement missing object-info, node schema, node replacement, graph validation, execution planning, caching, async/list execution, core node, and executor-dispatch behavior through native Sim graph modules.
  - Add registry/object-info snapshots and graph fixtures proving the missing nodes validate, plan, cache, and dispatch without ComfyUI pass-through.
  - _Requirements: 4.2, 5.4, 7.1, 7.3, 9.1, 9.2, 9.4_
  - _writes: .agents/specs/godot-migration/comfy-graph-node-runtime/tasks.md, crates/world_model/src/comfy_nodes.rs, crates/world_model/src/comfy_executor.rs, crates/world_model/src/comfy_nodes_tests.rs, crates/world_model/fixtures/comfy/*.json, crates/world_model/tests/comfy_core_nodes.rs, crates/world_model/fixtures/comfy/coverage_ledger.json_

- [x] 13. Port missing model and memory runtime functionality
  - For coverage records assigned to `comfy-model-memory-runtime`, implement missing model folder, catalog, preview, safetensors metadata, family detection, precision, quantization, device, memory, and resource-release behavior through native Sim model records and diagnostics.
  - Add fixtures for newly covered model families, folder categories, and memory-policy decisions without downloading model weights.
  - _Requirements: 4.3, 5.4, 6.2, 7.1, 7.3, 9.1, 9.2, 9.4_
  - _writes: .agents/specs/godot-migration/comfy-model-memory-runtime/tasks.md, crates/world_model/src/comfy_model_folders.rs, crates/world_model/src/comfy_model_family.rs, crates/world_model/src/world_model.rs, crates/world_model/src/comfy_model_family_tests.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/*.json, crates/world_model/fixtures/comfy/coverage_ledger.json_

- [x] 14. Port missing diffusion and world-model runtime functionality
  - For coverage records assigned to `comfy-diffusion-world-model-runtime`, implement missing sampler, scheduler, guider, conditioning, latent, VAE, model component, patch, LoRA/hypernetwork, diffusion runner, world-model runner, and worker-dispatch behavior through native Sim execution records.
  - Added metadata-only and mock-runner coverage for heavy model paths; production downloads and real workers remain dependency-review gated while backlog records now point to native Sim validation surfaces.
  - _Requirements: 4.4, 5.4, 6.2, 7.1, 7.3, 8.1, 9.1, 9.2, 9.4_
  - _writes: .agents/specs/godot-migration/comfy-diffusion-world-model-runtime/tasks.md, crates/world_model/src/comfy_execution_registry.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_model_execution.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/diffusion_world_model_backlog.json_

- [x] 15. Port missing asset-library functionality
  - For coverage records assigned to `comfy-asset-library`, implement missing asset CRUD, upload/download, tag, metadata filter, user-data, setting, scan, prune, output-registration, and enrichment behavior through native Sim asset, media, storage, and artifact modules.
  - Added asset backlog fixture coverage for the remaining output-registration fixture node and tied it to native Sim asset enrichment records without duplicating storage infrastructure.
  - _Requirements: 4.5, 5.4, 6.1, 7.1, 7.3, 9.1, 9.2, 9.4_
  - _writes: .agents/specs/godot-migration/comfy-asset-library/tasks.md, crates/world_model/src/sim_assets.rs, crates/world_model/src/sim_assets_tests.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/asset_library_backlog.json_

- [x] 16. Port missing workflow and blueprint functionality
  - For coverage records assigned to `comfy-workflows-blueprints`, implement missing blueprint import, workflow save/load/export, embedded metadata, app-mode metadata, global subgraph, template, and node-replacement catalog behavior through native Sim graph and authoring records.
  - Added workflow/blueprint backlog fixture coverage for every shipped blueprint and tied it to the native blueprint catalog, workflow, subgraph, template, replacement, embedded metadata, and app-mode records.
  - _Requirements: 4.6, 5.4, 6.1, 7.1, 7.3, 9.1, 9.2, 9.4_
  - _writes: .agents/specs/godot-migration/comfy-workflows-blueprints/tasks.md, crates/world_model/src/comfy_blueprints.rs, crates/world_model/src/comfy_blueprints_tests.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/workflows_blueprints_backlog.json_

- [x] 17. Port missing media-node functionality
  - For coverage records assigned to `comfy-media-node-pipelines`, implement missing image, mask, video, audio, 3D, Gaussian splat, geometry, detection, segmentation, control, utility, dataset, shader, and post-processing node behavior through native Sim media and artifact modules.
  - Added media-node backlog fixture coverage across native Sim image/mask, video, audio, 3D/geometry, analysis/control, and utility groups with metadata-only evidence for dependency-gated backends.
  - _Requirements: 4.7, 5.4, 6.2, 7.1, 7.3, 8.1, 9.1, 9.2, 9.4_
  - _writes: .agents/specs/godot-migration/comfy-media-node-pipelines/tasks.md, crates/world_model/src/sim_media_capabilities.rs, crates/world_model/src/sim_media_capabilities_tests.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/media_node_backlog.json_

- [x] 18. Port missing API-provider node functionality
  - For coverage records assigned to `comfy-api-provider-nodes`, implement missing provider catalogs, secret resolution, policy gates, remote task lifecycle, upload/download, output import, quota, cost, and connector-adapter behavior through native Sim provider modules.
  - Added provider backlog fixture coverage for all remaining API-provider nodes with real provider calls kept behind native Sim policy, secrets, mocks, and dependency review gates.
  - _Requirements: 4.8, 5.4, 6.2, 7.1, 7.3, 8.2, 9.1, 9.2, 9.4_
  - _writes: .agents/specs/godot-migration/comfy-api-provider-nodes/tasks.md, crates/world_model/src/sim_provider_nodes.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_provider_catalog.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/provider_backlog.json_

- [x] 19. Port missing extension-ecosystem functionality
  - For coverage records assigned to `comfy-extension-ecosystem`, implement missing custom-node discovery, V1/V3 registration, extension policy, loader diagnostics, web asset service, translations, templates, subgraphs, startup scripts, and manager-boundary behavior through native Sim extension controls.
  - Added `extension_backlog.json` coverage for all remaining extension hooks with native Sim discovery, policy, loader, asset, i18n, template, and manager-boundary evidence; fixtures remain metadata-only and do not execute unreviewed Python or serve arbitrary extension assets.
  - _Requirements: 4.9, 5.4, 6.2, 7.1, 7.3, 8.2, 9.1, 9.2, 9.4_
  - _writes: .agents/specs/godot-migration/comfy-extension-ecosystem/tasks.md, crates/world_model/src/comfy_extensions.rs, crates/world_model/src/comfy_extensions_tests.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/extension_backlog.json_

- [x] 20. Port missing packaging, configuration, and quality functionality
  - For coverage records assigned to `comfy-packaging-quality`, implement missing CLI launch flag, feature flag, OpenAPI/schema, API example, frontend package diagnostic, compatibility fixture, dependency-review, logging, CI, and packaging-profile behavior through native Sim config and diagnostics modules.
  - Added `packaging_quality_backlog.json` coverage for all remaining CLI flag, OpenAPI, packaging, test, dependency, and diagnostics surfaces with native Sim fixture evidence and review-gated metadata where required.
  - _Requirements: 4.10, 5.4, 6.1, 6.2, 7.1, 7.3, 8.2, 9.1, 9.2, 9.4_
  - _writes: .agents/specs/godot-migration/comfy-packaging-quality/tasks.md, crates/world_model/src/sim_packaging_profiles.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/packaging_quality_backlog.json_

- [x] 21. Close coverage after owner port tasks land
  - Re-run coverage validation after owner-specific port tasks complete and update the ledger so every source item is implemented, delegated, unsupported, or divergent with evidence.
  - Closed the ledger with 1,835 implemented records, zero remaining backlog tasks, and closure-aware Sim coverage tests that still validate owner backlog manifests when future gaps exist.
  - _Requirements: 2.1, 2.3, 5.1, 5.4, 7.3, 7.4, 9.1, 9.4_
  - _writes: crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/tests/sim_source_inventory.rs, crates/sim_game/src/sim_coverage_tests.rs, .agents/specs/godot-migration/comfy-full-port-coverage/tasks.md_

- [x] 22. Add an optional Comfy inventory refresh tool
  - Provide a local-only command that reads an external Comfy checkout and regenerates the source inventory fixture without importing torch or invoking provider APIs.
  - Added `cargo xtask comfy-inventory --comfy-root <path>` with a no-write `--check` mode and review summary for total, kind counts, added, removed, and changed source IDs.
  - Keep normal CI on committed fixtures only.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 5.2, 5.3, 6.1, 6.2, 9.1_
  - _writes: tooling/xtask/src/comfy_inventory.rs, tooling/xtask/src/main.rs, crates/world_model/fixtures/comfy/source_inventory.json_

## Notes

- This spec coordinates coverage; implementation ownership remains with the existing Comfy specs unless the coverage ledger names an existing Sim subsystem.
- Do not add a ComfyUI runtime process, proxy, or pass-through handler to satisfy coverage.
- Do not mark source behavior implemented without evidence.
- Do not duplicate existing Sim task, media, storage, secret, project, UI, artifact, or agent infrastructure.
- Inventory refreshes should be reviewed like dependency or fixture updates because they can create new implementation obligations.
