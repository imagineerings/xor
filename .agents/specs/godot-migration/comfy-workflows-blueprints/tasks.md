# Implementation Plan: Comfy Workflows and Blueprints

## Overview

Import shipped blueprints first, then add workflow persistence and API export, then subgraphs, replacements, and embedded metadata. UI tasks are intentionally delegated.

## Gates

- Start gate: G0 spec consistency, G5 graph safety for validation hooks, G6 provenance for generated metadata links, and G8 Comfy harness alignment are satisfied.
- Validation gate: blueprint import snapshots, workflow round-trip tests, API export tests, and subgraph id tests pass.
- Handoff gate: unsupported blueprint capabilities are visible as diagnostics.
- Completion gate: all 89 shipped blueprint names are represented in the catalog fixture.

## Dependency Waves

- Global waves: W3 Comfy execution core for Tasks 1-5; W4 Generation outputs and asset pipelines for Tasks 6-7.
- Local Wave 1: Tasks 1-2 build catalog and workflow store.
- Local Wave 2: Tasks 3-5 add subgraphs, replacements, and replacement-compatible metadata.
- Local Wave 3: Tasks 6-7 wire embedded workflow metadata and app-mode metadata into generated-output and authoring integrations.

## Tasks

- [x] 1. Import shipped Comfy blueprints
  - Add importer fixtures for all blueprint JSON files and associated GLSL/helper dependencies.
  - Preserve shipped names, source paths, graph JSON, categories, attribution, node inventory, dependency diagnostics, and unsupported-node diagnostics as native Sim catalog records without ComfyUI pass-through.
  - _Requirements: 1.1, 1.2, 1.3_
  - _writes: crates/world_model/src/comfy_blueprints.rs, crates/world_model/fixtures/comfy/blueprints_manifest.json, crates/world_model/src/comfy_blueprints_tests.rs_

- [x] 2. Implement workflow store and API export
  - Load/save workflow documents, preserve UI metadata, and emit API prompt graphs for execution.
  - Preserve workflow source, versions, default view, provenance links, graph JSON, and export diagnostics as native Sim records without ComfyUI pass-through.
  - _Requirements: 2.1, 2.2, 2.3_
  - _writes: crates/world_model/src/comfy_workflows.rs, crates/world_model/src/comfy_workflow_export.rs, crates/world_model/src/comfy_workflows_tests.rs_

- [x] 3. Implement global subgraph index
  - Index blueprint and custom-node subgraphs with stable ids, sanitized listings, and full graph data retrieval.
  - Preserve blueprint/custom-node source metadata, node-pack metadata, graph JSON, and listing diagnostics as native Sim records without ComfyUI registry pass-through.
  - _Requirements: 3.1, 3.2_
  - _writes: crates/world_model/src/comfy_subgraphs.rs, crates/world_model/src/comfy_subgraphs_tests.rs_

- [x] 4. Implement workflow template adapter
  - Expose custom node workflow templates and static template assets through Sim template services.
  - Preserve template names, node-pack metadata, safe static asset refs, graph JSON, sanitized listings, and diagnostics as native Sim template records without ComfyUI directory pass-through.
  - _Requirements: 3.3_
  - _writes: crates/world_model/src/comfy_workflow_templates.rs, crates/world_model/src/comfy_workflow_templates_tests.rs_

- [ ] 5. Implement node replacement catalog
  - Store deduped replacement mappings and expose them to graph validation and workflow import.
  - _Requirements: 4.1, 4.2, 4.3_
  - _writes: crates/world_model/src/comfy_replacements.rs, crates/world_model/src/comfy_replacements_tests.rs_

- [ ] 6. Implement embedded workflow metadata extraction
  - Extract supported prompt/workflow metadata from generated outputs and link recovered workflows to asset provenance.
  - _Requirements: 5.1, 5.2, 5.3_
  - _writes: crates/world_model/src/comfy_embedded_workflow.rs, crates/world_model/src/comfy_embedded_workflow_tests.rs_

- [ ] 7. Add app-mode metadata bridge
  - Preserve app-mode control metadata and expose it to the unified authoring app without implementing UI here.
  - _Requirements: 6.1, 6.2, 6.3_
  - _writes: crates/world_model/src/comfy_app_mode.rs, crates/world_model/src/comfy_app_mode_tests.rs_

## Notes

- Do not duplicate graph editor UI here.
- Blueprint validation diagnostics should not prevent catalog visibility.
