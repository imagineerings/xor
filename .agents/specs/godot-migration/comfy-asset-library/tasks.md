# Implementation Plan: Comfy Asset Library

## Overview

Build the asset library as native Sim storage primitives plus Comfy-compatible route adapters. Comfy compatibility preserves external API semantics only; implementation modules, records, and services use `SimAsset*` and `SimUserData*` names because the features are recreated in Sim rather than passed through to ComfyUI. Scanning and enrichment come after CRUD so generated outputs can register through the same path.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations, G6 provenance for generated output links, and G8 Comfy harness alignment, G9 Sim coverage are satisfied.
- Validation gate: repository tests, route tests, scanner tests, and user-data path confinement tests pass.
- Handoff gate: asset-disabled and database-unavailable diagnostics are stable.
- Completion gate: all asset mutations are owner-scoped and path-confined.

## Dependency Waves

- Global wave: W4 Generation outputs and asset pipelines.
- Local Wave 1: Tasks 1-2 implement data and validation foundations.
- Local Wave 2: Tasks 3-5 implement APIs and user data.
- Local Wave 3: Tasks 6-7 implement scanning, enrichment, and generated output registration.

## Tasks

- [x] 1. Implement asset repository models
  - Add content records, reference records, tag links, metadata entries, soft delete, owner fields, and cache state.
  - Preserve content hash dedupe, owner-scoped references, separate mutable reference metadata, provenance ids, soft-delete state, and cache state as native Sim repository records without ComfyUI asset database pass-through.
  - _Requirements: 1.1, 1.2, 1.3, 6.2_
  - _writes: crates/world_model/src/sim_assets.rs, crates/world_model/src/sim_assets_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-asset-library_

- [x] 2. Implement asset query validation
  - Add hash validation, cursor pagination, metadata filter parsing, sort/order validation, tag normalization, and owner scoping helpers.
  - Parse compatibility route parameters into native Sim query types and diagnostics without forwarding ComfyUI query strings or treating validation as a compatibility label.
  - _Requirements: 2.1, 3.2, 3.4, 5.1_
  - _writes: crates/world_model/src/sim_asset_query.rs, crates/world_model/src/sim_asset_query_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-asset-library_

- [x] 3. Implement asset CRUD and upload APIs
  - Add list, detail, create-from-hash, multipart upload, update, delete, and hash existence behavior.
  - Execute CRUD and upload behavior against native Sim repository records, query validators, and owner scopes without proxying mutations to ComfyUI asset routes.
  - _Requirements: 2.1, 2.2, 2.3, 2.5_
  - _writes: crates/world_model/src/sim_asset_api.rs, crates/world_model/src/sim_asset_upload.rs, crates/world_model/src/sim_asset_api_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-asset-library_

- [x] 4. Implement asset download and preview resolution
  - Stream content, force safe content types, resolve preview ids, and hand media preview routing to Sim media.
  - Resolve downloads and previews from native Sim asset records and Sim media preview routes without forwarding to ComfyUI preview or file handlers.
  - _Requirements: 2.4, 6.1_
  - _writes: crates/world_model/src/sim_asset_download.rs, crates/world_model/src/sim_asset_download_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-asset-library_

- [x] 5. Implement tags and user data
  - Add tag add/remove/list/refine and Comfy-compatible user settings and user data file operations.
  - Implement tags, user files, and settings as native Sim asset/user-storage services without calling ComfyUI tag, settings, or user-data handlers.
  - _Requirements: 3.1, 3.2, 3.3, 5.1, 5.2, 5.3, 5.4_
  - _writes: crates/world_model/src/sim_asset_tags.rs, crates/world_model/src/sim_user_data.rs, crates/world_model/src/sim_user_data_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-asset-library_

- [x] 6. Implement asset seeding and pruning
  - Scan models/input/output roots, report progress, support cancellation, and mark missing references outside known roots.
  - Register scans and prune missing references through native Sim asset APIs and cache state without invoking ComfyUI scanner or pruning code.
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _writes: crates/world_model/src/sim_asset_seeder.rs, crates/world_model/src/sim_asset_scanner.rs, crates/world_model/src/sim_asset_scanner_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-asset-library_

- [x] 7. Implement output registration and enrichment
  - Register generated output files, attach job ids, compute optional hashes, extract image/model metadata, and enqueue enrichment after execution.
  - Register generated outputs and enrichment jobs through native Sim asset records, provenance ids, and system metadata without calling ComfyUI output or metadata handlers.
  - _Requirements: 4.5, 6.2, 6.3_
  - _writes: crates/world_model/src/sim_asset_enrichment.rs, crates/world_model/src/sim_asset_enrichment_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-asset-library_

- [x] 8. Materialize remaining asset-library coverage backlog
  - Convert 1 planned coverage records in asset-library into native Sim implementation, delegation, unsupported, or divergent outcomes without ComfyUI pass-through.
  - Coverage IDs: the asset-library backlog record in `crates/world_model/fixtures/comfy/coverage_ledger.json` now marked `Implemented` with `crates/world_model/fixtures/comfy/asset_library_backlog.json` evidence; representative ID: extranode:projects_comfy_comfy_extras_nodes_logic_py:ComboOutputTestNode.
  - Native Sim writes: crates/world_model/src/sim_assets.rs, crates/world_model/src/sim_assets_tests.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/asset_library_backlog.json.
  - Validation: `cargo test -p world_model sim_asset`.
  - Parity evidence: Mark records implemented only with native Sim asset/storage/user-data tests or fixtures; avoid duplicate storage infrastructure.
  - _CoverageTask: coverage-backlog-asset-library_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-asset-library_
  - _Requirements: 9.1, 9.2, 9.3, 9.4_
  - _writes: crates/world_model/src/sim_assets.rs, crates/world_model/src/sim_assets_tests.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/asset_library_backlog.json

## Notes

- Media rendering stays in `rendering-media/`.
- Model folder definitions stay in `comfy-model-memory-runtime/`.
