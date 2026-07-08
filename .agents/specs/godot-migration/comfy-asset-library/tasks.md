# Implementation Plan: Comfy Asset Library

## Overview

Build the asset library as shared storage primitives plus Comfy-compatible route adapters. Scanning and enrichment come after CRUD so generated outputs can register through the same path.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations, G6 provenance for generated output links, and G8 Comfy harness alignment are satisfied.
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
  - _writes: crates/world_model/src/comfy_assets.rs, crates/world_model/src/comfy_assets_tests.rs_

- [ ] 2. Implement asset query validation
  - Add hash validation, cursor pagination, metadata filter parsing, sort/order validation, tag normalization, and owner scoping helpers.
  - _Requirements: 2.1, 3.2, 3.4, 5.1_
  - _writes: crates/world_model/src/comfy_asset_query.rs, crates/world_model/src/comfy_asset_query_tests.rs_

- [ ] 3. Implement asset CRUD and upload APIs
  - Add list, detail, create-from-hash, multipart upload, update, delete, and hash existence behavior.
  - _Requirements: 2.1, 2.2, 2.3, 2.5_
  - _writes: crates/world_model/src/comfy_asset_api.rs, crates/world_model/src/comfy_asset_upload.rs, crates/world_model/src/comfy_asset_api_tests.rs_

- [ ] 4. Implement asset download and preview resolution
  - Stream content, force safe content types, resolve preview ids, and hand media preview routing to Sim media.
  - _Requirements: 2.4, 6.1_
  - _writes: crates/world_model/src/comfy_asset_download.rs, crates/world_model/src/comfy_asset_download_tests.rs_

- [ ] 5. Implement tags and user data
  - Add tag add/remove/list/refine and Comfy-compatible user settings and user data file operations.
  - _Requirements: 3.1, 3.2, 3.3, 5.1, 5.2, 5.3, 5.4_
  - _writes: crates/world_model/src/comfy_asset_tags.rs, crates/world_model/src/comfy_user_data.rs, crates/world_model/src/comfy_user_data_tests.rs_

- [ ] 6. Implement asset seeding and pruning
  - Scan models/input/output roots, report progress, support cancellation, and mark missing references outside known roots.
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _writes: crates/world_model/src/comfy_asset_seeder.rs, crates/world_model/src/comfy_asset_scanner.rs, crates/world_model/src/comfy_asset_scanner_tests.rs_

- [ ] 7. Implement output registration and enrichment
  - Register generated output files, attach job ids, compute optional hashes, extract image/model metadata, and enqueue enrichment after execution.
  - _Requirements: 4.5, 6.2, 6.3_
  - _writes: crates/world_model/src/comfy_asset_enrichment.rs, crates/world_model/src/comfy_asset_enrichment_tests.rs_

## Notes

- Media rendering stays in `rendering-media/`.
- Model folder definitions stay in `comfy-model-memory-runtime/`.
