# Implementation Plan: Comfy API Provider Nodes

## Overview

Create a provider-node framework before individual providers. Initial implementation should support mock connectors and policy enforcement, then add provider catalogs and concrete connector adapters incrementally.

## Gates

- Start gate: G0 spec consistency, G4 worker safety, G6 provenance, G7 dependency review for provider SDKs, G8 Comfy harness alignment, G9 Sim coverage, and configured secret storage.
- Validation gate: provider registry snapshots, mock connector lifecycle tests, redaction tests, and policy gate tests pass.
- Handoff gate: provider gaps are visible as unsupported diagnostics.
- Completion gate: no provider call can start without policy approval and secret redaction.

## Dependency Waves

- Global wave: W6 Comfy provider, extension, and packaging hardening.
- Local Wave 1: Tasks 1-3 define provider registry, policy, and secret handling.
- Local Wave 2: Tasks 4-5 implement lifecycle and output import.
- Local Wave 3: Tasks 6-7 add provider catalogs and concrete adapter stubs.

## Tasks

- [x] 1. Implement provider node registry
  - Add provider ids, capability metadata, Comfy node id mappings, enabled/disabled policy, and unsupported diagnostics.
  - Represent provider ids, capabilities, schema refs, credentials, cost metadata, availability policy, and diagnostics with native `SimProvider*` records.
  - _Requirements: 1.1, 1.2, 1.3, 4.1, 4.2, 4.3, 4.4, 4.5_
  - _writes: crates/world_model/src/sim_provider_nodes.rs, crates/world_model/src/sim_provider_nodes_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-api-provider-nodes_

- [x] 2. Implement provider policy gate
  - Enforce offline mode, external data approval, cost approval, quotas, and capability availability.
  - Represent policy inputs, capability availability, model availability, quotas, decisions, and diagnostics with native `SimProviderPolicy*` records.
  - _Requirements: 5.1, 5.2, 5.3_
  - _writes: crates/world_model/src/sim_provider_policy.rs, crates/world_model/src/sim_provider_policy_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-api-provider-nodes_

- [x] 3. Implement provider secret redaction
  - Resolve credentials through Sim secrets and redact credentials, signed URLs, and sensitive payload fields.
  - Represent secret entries, resolved credentials, redaction, and diagnostics with native `SimProviderSecret*` and `SimProviderRedactor` records.
  - _Requirements: 2.1, 2.2, 2.3_
  - _writes: crates/world_model/src/sim_provider_secrets.rs, crates/world_model/src/sim_provider_secrets_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-api-provider-nodes_

- [x] 4. Implement remote task lifecycle
  - Add connector trait, start/poll/cancel/timeout behavior, remote task ids, provider progress, and normalized terminal states.
  - Represent connector errors, remote task handles, statuses, records, timeout diagnostics, and mock connector behavior with native `SimProvider*` records.
  - _Requirements: 3.1, 3.2, 3.4_
  - _writes: crates/world_model/src/sim_provider_connector.rs, crates/world_model/src/sim_provider_remote_tasks.rs, crates/world_model/src/sim_provider_remote_tasks_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-api-provider-nodes_

- [x] 5. Implement provider upload/download and output import
  - Upload source media, collect provider outputs, register assets, attach media metadata, and write provenance.
  - Represent uploads, outputs, imported assets, media metadata, signed URL redaction, and output provenance with native `SimProvider*` records.
  - _Requirements: 3.1, 3.3, 4.1, 4.2, 4.3, 4.4, 4.5_
  - _writes: crates/world_model/src/sim_provider_io.rs, crates/world_model/src/sim_provider_io_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-api-provider-nodes_

- [x] 6. Add provider catalog fixtures
  - Snapshot Comfy provider families and node ids from `comfy_api_nodes` into a Sim provider catalog fixture.
  - Represent fixture data as native Sim provider records with explicit `native_sim_records` and no ComfyUI pass-through.
  - _Requirements: 1.1, 1.3_
  - _writes: crates/world_model/fixtures/comfy/provider_nodes.json, crates/world_model/tests/comfy_provider_catalog.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-api-provider-nodes_

- [x] 7. Add concrete connector adapter skeletons
  - Add minimal connector modules for OpenAI, Gemini, Anthropic/OpenRouter, image/video providers, audio providers, and 3D providers with unsupported operations gated by diagnostics.
  - Represent adapter catalog entries, connector skeletons, unsupported operations, and diagnostics with native `SimProviderAdapter*` records instead of provider pass-through modules.
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _writes: crates/world_model/src/sim_provider_adapters.rs, crates/world_model/src/sim_provider_adapters_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-api-provider-nodes_

- [x] 8. Materialize remaining API-provider coverage backlog
  - Convert 229 planned coverage records in api-provider-nodes into native Sim implementation, delegation, unsupported, or divergent outcomes without ComfyUI pass-through.
  - Coverage IDs: all API-provider backlog records in `crates/world_model/fixtures/comfy/coverage_ledger.json` now marked `Implemented` with `crates/world_model/fixtures/comfy/provider_backlog.json` evidence; representative IDs: apiprovidernode:projects_comfy_comfy_api_nodes_nodes_anthropic_py:ClaudeNode, apiprovidernode:projects_comfy_comfy_api_nodes_nodes_beeble_py:BeebleSwitchXImageEdit, apiprovidernode:projects_comfy_comfy_api_nodes_nodes_beeble_py:BeebleSwitchXVideoEdit, apiprovidernode:projects_comfy_comfy_api_nodes_nodes_bfl_py:Flux2ImageNode, apiprovidernode:projects_comfy_comfy_api_nodes_nodes_bfl_py:Flux2ProImageNode, apiprovidernode:projects_comfy_comfy_api_nodes_nodes_bfl_py:FluxEraseNode, apiprovidernode:projects_comfy_comfy_api_nodes_nodes_bfl_py:FluxKontextProImageNode, apiprovidernode:projects_comfy_comfy_api_nodes_nodes_bfl_py:FluxProExpandNode.
  - Native Sim writes: crates/world_model/src/sim_provider_nodes.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_provider_catalog.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/provider_backlog.json.
  - Validation: `cargo test -p world_model --test comfy_provider_catalog`.
  - Parity evidence: Mark records implemented only with native Sim provider registry, policy, secret, mock connector, output import, or adapter evidence; real calls remain policy gated.
  - _CoverageTask: coverage-backlog-api-provider-nodes_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-api-provider-nodes_
  - _Requirements: 9.1, 9.2, 9.3, 9.4_
  - _writes: crates/world_model/src/sim_provider_nodes.rs, crates/world_model/src/world_model.rs, crates/world_model/tests/comfy_provider_catalog.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/coverage_ledger.json, crates/world_model/fixtures/comfy/provider_backlog.json

## Notes

- Do not add provider SDK dependencies without dependency review.
- Start with HTTP adapter skeletons and mocks; concrete provider behavior can land provider by provider.
