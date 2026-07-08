# Implementation Plan: Comfy API Provider Nodes

## Overview

Create a provider-node framework before individual providers. Initial implementation should support mock connectors and policy enforcement, then add provider catalogs and concrete connector adapters incrementally.

## Gates

- Start gate: G0 spec consistency, G4 worker safety, G6 provenance, G7 dependency review for provider SDKs, G8 Comfy harness alignment, and configured secret storage.
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

- [ ] 2. Implement provider policy gate
  - Enforce offline mode, external data approval, cost approval, quotas, and capability availability.
  - _Requirements: 5.1, 5.2, 5.3_
  - _writes: crates/world_model/src/comfy_provider_policy.rs, crates/world_model/src/comfy_provider_policy_tests.rs_

- [ ] 3. Implement provider secret redaction
  - Resolve credentials through Sim secrets and redact credentials, signed URLs, and sensitive payload fields.
  - _Requirements: 2.1, 2.2, 2.3_
  - _writes: crates/world_model/src/comfy_provider_secrets.rs, crates/world_model/src/comfy_provider_redaction_tests.rs_

- [ ] 4. Implement remote task lifecycle
  - Add connector trait, start/poll/cancel/timeout behavior, remote task ids, provider progress, and normalized terminal states.
  - _Requirements: 3.1, 3.2, 3.4_
  - _writes: crates/world_model/src/comfy_provider_connector.rs, crates/world_model/src/comfy_remote_tasks.rs, crates/world_model/src/comfy_remote_tasks_tests.rs_

- [ ] 5. Implement provider upload/download and output import
  - Upload source media, collect provider outputs, register assets, attach media metadata, and write provenance.
  - _Requirements: 3.1, 3.3, 4.1, 4.2, 4.3, 4.4, 4.5_
  - _writes: crates/world_model/src/comfy_provider_io.rs, crates/world_model/src/comfy_provider_io_tests.rs_

- [ ] 6. Add provider catalog fixtures
  - Snapshot Comfy provider families and node ids from `comfy_api_nodes` into a Sim provider catalog fixture.
  - _Requirements: 1.1, 1.3_
  - _writes: crates/world_model/fixtures/comfy/provider_nodes.json, crates/world_model/tests/comfy_provider_catalog.rs_

- [ ] 7. Add concrete connector adapter skeletons
  - Add minimal connector modules for OpenAI, Gemini, Anthropic/OpenRouter, image/video providers, audio providers, and 3D providers with unsupported operations gated by diagnostics.
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _writes: crates/world_model/src/providers/openai.rs, crates/world_model/src/providers/gemini.rs, crates/world_model/src/providers/anthropic.rs, crates/world_model/src/providers/media.rs, crates/world_model/src/providers/audio.rs, crates/world_model/src/providers/three_d.rs_

## Notes

- Do not add provider SDK dependencies without dependency review.
- Start with HTTP adapter skeletons and mocks; concrete provider behavior can land provider by provider.
