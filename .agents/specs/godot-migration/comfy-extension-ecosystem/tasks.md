# Implementation Plan: Comfy Extension Ecosystem

## Overview

Build extension support with policy and diagnostics first. Only after disabled and unsafe behaviors are controlled should node registration, web assets, translations, templates, and manager actions be wired in.

## Gates

- Start gate: G0 spec consistency, G1 boundary policy, G7 dependency review for install/update behavior, and G8 Comfy harness alignment are satisfied.
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

- [x] 2. Implement extension policy and diagnostics
  - Add enable/disable/block/whitelist decisions, script permission, web asset permission, network/install permission, and diagnostic records.
  - Represent policy decisions, permission reports, install review gates, and diagnostics with native `SimExtensionPolicy*` records.
  - _Requirements: 1.2, 1.3, 1.4, 3.1, 5.3_
  - _writes: crates/world_model/src/comfy_extension_policy.rs, crates/world_model/src/comfy_extension_diagnostics.rs, crates/world_model/src/comfy_extension_policy_tests.rs_

- [x] 3. Implement controlled extension loader
  - Load allowed packs, run permitted prestartup scripts, restore protected hooks, and isolate import failures.
  - Represent load metadata, loaded/skipped packs, restored hooks, missing dependencies, and import diagnostics with native `SimExtensionLoad*` records without arbitrary ComfyUI execution.
  - _Requirements: 1.4, 3.1, 3.2, 3.3_
  - _writes: crates/world_model/src/comfy_extension_loader.rs, crates/world_model/src/comfy_extension_loader_tests.rs_

- [x] 4. Implement custom node registration bridge
  - Support V1 mappings and modern extension entrypoints, display names, module metadata, and unsupported registration diagnostics.
  - Represent custom node declarations, module metadata, registration records, and diagnostics with native `SimCustomNode*` records while registering Sim-owned node definitions.
  - _Requirements: 2.1, 2.2, 2.4_
  - _writes: crates/world_model/src/comfy_custom_node_bridge.rs, crates/world_model/src/comfy_custom_node_bridge_tests.rs_

- [x] 5. Implement extension web asset service
  - Serve registered web directories and templates with path confinement, cache policy, and safe content types.
  - Represent extension web/template roots, responses, and diagnostics with native `SimExtensionAsset*` records and Sim-owned routes rather than ComfyUI asset pass-throughs.
  - _Requirements: 2.3, 4.2_
  - _writes: crates/world_model/src/comfy_extension_assets.rs, crates/world_model/src/comfy_extension_assets_tests.rs_

- [x] 6. Implement translations, templates, and subgraph indexing
  - Merge locale bundles, expose template names/assets, and feed custom node subgraphs into workflow subgraph index.
  - Represent locale bundles, template declarations, subgraph declarations, and index reports with native `SimExtensionLocale*` and `SimExtensionTemplate*` records.
  - _Requirements: 4.1, 4.2, 4.3_
  - _writes: crates/world_model/src/comfy_extension_i18n.rs, crates/world_model/src/comfy_extension_templates.rs, crates/world_model/src/comfy_extension_i18n_tests.rs_

- [ ] 7. Implement manager compatibility boundary
  - Add manager status/policy metadata and approval gates for install, update, disable, and background operations.
  - _Requirements: 5.1, 5.2, 5.3_
  - _writes: crates/world_model/src/comfy_manager.rs, crates/world_model/src/comfy_manager_tests.rs_

## Notes

- This spec does not grant arbitrary Python execution by default.
- Package installation and downloads require explicit approval and dependency review.
