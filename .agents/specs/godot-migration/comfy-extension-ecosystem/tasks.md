# Implementation Plan: Comfy Extension Ecosystem

## Overview

Build extension support with policy and diagnostics first. Only after disabled and unsafe behaviors are controlled should node registration, web assets, translations, templates, and manager actions be wired in.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, G7 dependency review for install/update behavior, and G8 Comfy harness alignment, G9 Sim coverage are satisfied.
- Validation gate: discovery, policy, loader, asset path, translation, and manager-policy tests pass.
- Handoff gate: every skipped, blocked, or failed extension has a visible diagnostic.
- Completion gate: disabled extensions cannot execute or expose assets.

## Dependency Waves

- Global wave: W6 Comfy provider, extension, and packaging hardening.
- Local Wave 1: Tasks 1-2 implement discovery and policy.
- Local Wave 2: Tasks 3-5 implement loading, node registration, and assets.
- Local Wave 3: Tasks 6-7 implement translations, templates, subgraphs, and manager boundaries.

## Tasks

- [x] 1. Implement extension discovery
  - Find Python files and directories, skip disabled packs, apply whitelist filtering, and preserve deterministic load order.
  - Represent discovered packs, source kinds, load order, and skip diagnostics with native `SimExtension*` records without importing or executing ComfyUI extension code.
  - _Requirements: 1.1, 1.2_
  - _writes: crates/world_model/src/comfy_extensions.rs, crates/world_model/src/comfy_extensions_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-extension-ecosystem_

- [x] 2. Implement extension policy and diagnostics
  - Add enable/disable/block/whitelist decisions, script permission, web asset permission, network/install permission, and diagnostic records.
  - Represent policy decisions, permission reports, install review gates, and diagnostics with native `SimExtensionPolicy*` records.
  - _Requirements: 1.2, 1.3, 1.4, 3.1, 5.3_
  - _writes: crates/world_model/src/comfy_extension_policy.rs, crates/world_model/src/comfy_extension_diagnostics.rs, crates/world_model/src/comfy_extension_policy_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-extension-ecosystem_

- [x] 3. Implement controlled extension loader
  - Load allowed packs, run permitted prestartup scripts, restore protected hooks, and isolate import failures.
  - Represent load metadata, loaded/skipped packs, restored hooks, missing dependencies, and import diagnostics with native `SimExtensionLoad*` records without arbitrary ComfyUI execution.
  - _Requirements: 1.4, 3.1, 3.2, 3.3_
  - _writes: crates/world_model/src/comfy_extension_loader.rs, crates/world_model/src/comfy_extension_loader_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-extension-ecosystem_

- [x] 4. Implement custom node registration bridge
  - Support V1 mappings and modern extension entrypoints, display names, module metadata, and unsupported registration diagnostics.
  - Represent custom node declarations, module metadata, registration records, and diagnostics with native `SimCustomNode*` records while registering Sim-owned node definitions.
  - _Requirements: 2.1, 2.2, 2.4_
  - _writes: crates/world_model/src/comfy_custom_node_bridge.rs, crates/world_model/src/comfy_custom_node_bridge_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-extension-ecosystem_

- [x] 5. Implement extension web asset service
  - Serve registered web directories and templates with path confinement, cache policy, and safe content types.
  - Represent extension web/template roots, responses, and diagnostics with native `SimExtensionAsset*` records and Sim-owned routes rather than ComfyUI asset pass-throughs.
  - _Requirements: 2.3, 4.2_
  - _writes: crates/world_model/src/comfy_extension_assets.rs, crates/world_model/src/comfy_extension_assets_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-extension-ecosystem_

- [x] 6. Implement translations, templates, and subgraph indexing
  - Merge locale bundles, expose template names/assets, and feed custom node subgraphs into workflow subgraph index.
  - Represent locale bundles, template declarations, subgraph declarations, and index reports with native `SimExtensionLocale*` and `SimExtensionTemplate*` records.
  - _Requirements: 4.1, 4.2, 4.3_
  - _writes: crates/world_model/src/comfy_extension_i18n.rs, crates/world_model/src/comfy_extension_templates.rs, crates/world_model/src/comfy_extension_i18n_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-extension-ecosystem_

- [x] 7. Implement manager compatibility boundary
  - Add manager status/policy metadata and approval gates for install, update, disable, and background operations.
  - Represent manager actions, status, approvals, evaluations, and diagnostics with native `SimManager*` records that enforce Sim policy and dependency review instead of calling ComfyUI-Manager directly.
  - _Requirements: 5.1, 5.2, 5.3_
  - _writes: crates/world_model/src/comfy_manager.rs, crates/world_model/src/comfy_manager_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-extension-ecosystem_

- [x] 8. Materialize remaining extension-ecosystem coverage backlog
  - Convert 149 planned coverage records in extension-ecosystem into native Sim implementation, delegation, unsupported, or divergent outcomes without ComfyUI pass-through.
  - Coverage IDs: all former records in `crates/world_model/fixtures/comfy/coverage_ledger.json` with `backlog_task.task_id = coverage-backlog-extension-ecosystem` are now marked `Implemented` with `crates/world_model/fixtures/comfy/extension_backlog.json` evidence; representative IDs: extensionhook:projects_comfy_app_assets_api_routes_py:routes, extensionhook:projects_comfy_comfy_api_nodes_nodes_anthropic_py:AnthropicExtension, extensionhook:projects_comfy_comfy_api_nodes_nodes_beeble_py:BeebleExtension, extensionhook:projects_comfy_comfy_api_nodes_nodes_bfl_py:BFLExtension, extensionhook:projects_comfy_comfy_api_nodes_nodes_bria_py:BriaExtension, extensionhook:projects_comfy_comfy_api_nodes_nodes_bytedance_py:ByteDanceExtension, extensionhook:projects_comfy_comfy_api_nodes_nodes_bytedance_llm_py:ByteDanceLLMExtension, extensionhook:projects_comfy_comfy_api_nodes_nodes_elevenlabs_py:ElevenLabsExtension.
  - Native Sim writes: crates/world_model/src/comfy_extensions.rs, crates/world_model/src/comfy_extensions_tests.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/extension_backlog.json.
  - Validation: `cargo test -p world_model comfy_extension`.
  - Parity evidence: Records are implemented only with native Sim extension discovery, policy, loader, asset, i18n, template, or manager-boundary evidence; the backlog fixture is metadata-only and does not execute ComfyUI extension code.
  - _CoverageTask: coverage-backlog-extension-ecosystem_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-extension-ecosystem_
  - _Requirements: 9.1, 9.2, 9.3, 9.4_
  - _writes: crates/world_model/src/comfy_extensions.rs, crates/world_model/src/comfy_extensions_tests.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/extension_backlog.json

## Notes

- This spec does not grant arbitrary Python execution by default.
- Package installation and downloads require explicit approval and dependency review.
