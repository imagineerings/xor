# Implementation Plan: Comfy Packaging, Configuration, and Quality

## Overview

Build cross-cutting migration controls that other Comfy specs depend on: launch profile parsing, feature flags, schema catalog, compatibility fixtures, dependency gates, packaging profiles, and diagnostics. Comfy-compatible inputs are parsed into native Sim records; Sim-owned implementation modules and types use `Sim*` names rather than `Comfy*` pass-through labels.

## Gates

- Start gate: G0 spec consistency, G7 dependency review policy, and G8 Comfy harness alignment, G9 Sim coverage are satisfied.
- Validation gate: config parser tests, feature flag tests, schema catalog tests, fixture suite checks, and dependency gate tests pass.
- Handoff gate: unsupported options, unsupported routes, and dependency-review requirements are documented in machine-readable catalogs.
- Completion gate: every Comfy migration spec has at least one fixture or snapshot path planned.

## Dependency Waves

- Global waves: W2 Value-first world-model serving substrate for configuration, schemas, fixtures, and diagnostics; W6 Comfy provider, extension, and packaging hardening for packaging profiles and dependency gates.
- Local Wave 1: Tasks 1-2 implement configuration and feature flags.
- Local Wave 2: Tasks 3-5 implement schemas, fixtures, and dependency gates.
- Local Wave 3: Tasks 6-7 add packaging profiles and diagnostics.

## Tasks

- [x] 1. Implement Comfy launch profile parser
  - Parse networking, TLS, CORS, upload limits, directories, auto-launch, logging, assets, database, API nodes, custom nodes, manager, compression, runtime policy, cache, and performance options.
  - Represent parsed launch configuration with native `SimLaunch*` records and diagnostics, while accepting Comfy-compatible option names at the adapter boundary.
  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _writes: crates/world_model/src/sim_launch_profile.rs, crates/world_model/src/sim_launch_profile_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-packaging-quality_

- [x] 2. Implement feature flag registry
  - Add typed flag coercion, core flag protection, server feature response, and connection-specific client flag storage.
  - Report missing or outdated frontend, workflow template, and embedded docs packages with actionable Sim diagnostics.
  - Represent the registry and negotiated flags with native `SimFeatureFlag*` records while compatibility adapters translate route/event payloads.
  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _writes: crates/world_model/src/sim_feature_flags.rs, crates/world_model/src/sim_feature_flags_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-packaging-quality_

- [x] 3. Implement API schema support catalog
  - Track supported, planned, cloud-only, external, and unsupported route statuses for Comfy/OpenAPI compatibility.
  - Derive implemented route coverage from Sim-owned route handlers, require schema refs for implemented routes, and require reasons for planned/cloud-only/external/unsupported routes.
  - Represent schema status with native `SimApiSchema*` records while compatibility adapters translate documented route shape.
  - _Requirements: 3.1, 3.3, 3.4_
  - _writes: crates/world_model/src/sim_api_schema.rs, crates/world_model/fixtures/comfy/api_routes.json, crates/world_model/src/sim_api_schema_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-packaging-quality_

- [x] 4. Build compatibility fixture suite
  - Add fixture harnesses for script examples, route snapshots, node schemas, blueprint manifest, provider catalog, asset API, and media capability groups.
  - Aggregate implemented fixture groups through native Sim records and document future provider/media fixture ownership without treating missing future groups as ComfyUI pass-through coverage.
  - _Requirements: 3.2, 4.1, 4.2, 4.3_
  - _writes: crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/README.md_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-packaging-quality_

- [x] 5. Implement dependency review gate
  - Require review metadata for native libraries, codecs, Python packages, provider SDKs, model dependencies, frontend packages, vendored code, network access, and large downloads.
  - Represent dependency proposals, review records, decisions, diagnostics, and audit approvals with native `SimDependencyReview*` types rather than Comfy-labeled pass-through records.
  - _Requirements: 5.1, 5.3_
  - _writes: crates/world_model/src/sim_dependency_review.rs, crates/world_model/src/sim_dependency_review_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-packaging-quality_

- [x] 6. Add packaging profile catalog
  - Define CPU-only, GPU-specific, API-disabled, custom-node-disabled, asset-enabled, portable-like, and remote-worker launch profiles without duplicating platform packaging.
  - Represent catalog entries with native `SimPackagingProfile*` records and keep installer/platform packaging delegated to existing Sim systems.
  - _Requirements: 5.2_
  - _writes: crates/world_model/src/sim_packaging_profiles.rs, crates/world_model/src/sim_packaging_profiles_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-packaging-quality_

- [x] 7. Implement logs and internal diagnostics adapter
  - Expose formatted/raw logs, terminal size metadata, approved folder paths, and recent input/output/temp files through Sim diagnostics.
  - Represent logs, approved roots, recent files, diagnostics, and endpoint stability with native `SimDiagnostics*` records rather than Comfy-labeled pass-through records.
  - _Requirements: 6.1, 6.2, 6.3_
  - _writes: crates/world_model/src/sim_diagnostics_adapter.rs, crates/world_model/src/sim_diagnostics_adapter_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-packaging-quality_

- [x] 8. Materialize remaining packaging and quality coverage backlog
  - Convert 683 planned coverage records in packaging-quality into native Sim implementation, delegation, unsupported, or divergent outcomes without ComfyUI pass-through.
  - Coverage IDs: all former records in `crates/world_model/fixtures/comfy/coverage_ledger.json` with `backlog_task.task_id = coverage-backlog-packaging-quality` are now marked `Implemented` with `crates/world_model/fixtures/comfy/packaging_quality_backlog.json` evidence; representative IDs: cliflag:projects_comfy_comfy_cli_args_py:--async-offload, cliflag:projects_comfy_comfy_cli_args_py:--auto-launch, cliflag:projects_comfy_comfy_cli_args_py:--base-directory, cliflag:projects_comfy_comfy_cli_args_py:--bf16-text-enc, cliflag:projects_comfy_comfy_cli_args_py:--bf16-unet, cliflag:projects_comfy_comfy_cli_args_py:--bf16-vae, cliflag:projects_comfy_comfy_cli_args_py:--cache-classic, cliflag:projects_comfy_comfy_cli_args_py:--cache-lru.
  - Native Sim writes: crates/world_model/src/sim_packaging_profiles.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/packaging_quality_backlog.json.
  - Validation: `cargo test -p world_model --test comfy_compat_suite`.
  - Parity evidence: Records are implemented only with native Sim launch, feature flag, schema, fixture, dependency review, packaging, or diagnostics evidence; package and release surfaces remain metadata-only and review-gated where required.
  - _CoverageTask: coverage-backlog-packaging-quality_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-packaging-quality_
  - _Requirements: 9.1, 9.2, 9.3, 9.4_
  - _writes: crates/world_model/src/sim_packaging_profiles.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/packaging_quality_backlog.json

## Notes

- This spec is a support layer; runtime behavior lives in the domain specs.
- Internal diagnostics should remain explicitly unstable.
