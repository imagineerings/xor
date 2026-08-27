# Implementation Plan: Additional LLM Providers

> Cross-cutting contract: every production write in this plan inherits the [`agentic` feature boundary](../feature-boundary.md). Completion evidence must classify actual writes and include the required enabled/disabled validation.

## Overview

Implement the 18+ provider integrations from goose that don't yet exist in zed, following the patterns established in `crates/language_models/src/provider/`. Work is grouped: first the shared infrastructure (registry, declarative providers, ACP adapter), then individual providers by category.

## Tasks

- [ ] 1. Implement provider registry and declarative providers
  - [ ] 1.1. Create `ProviderRegistry` struct with factory registration
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 1.2. Create `DeclarativeProviderConfig` types and validation
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 1.3. Reconcile canonical model metadata and provider contract normalization with `language_model_core`
  - Extend the existing provider registry rather than centralizing registration in a second registry
  - Define declarative configuration only for source-confirmed catalog fields and validate unknown or conflicting fields
  - Reuse `language_model_core` request, response, error, usage, tool, image, and thinking contracts
  - Add canonical metadata only where an approved provider needs fields Zed cannot currently express

  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_models/src/provider/registry.rs, crates/language_models/src/provider/declarative.rs, crates/language_model_core/src/provider.rs, crates/language_models/src/language_models.rs, crates/language_models/src/provider.rs_
  - _Writes: crates/language_models/src/provider/registry.rs, crates/language_models/src/provider/declarative.rs, crates/language_model_core/src/provider.rs, crates/language_models/src/language_models.rs, crates/language_models/src/provider.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Implement ACP-based provider adapter
  - [ ] 2.1. Create `AcpSubprocessProvider` struct with binary discovery and `LanguageModelProvider` trait impl
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 2.2. Define `AcpAuthMethod` enum (ApiKey, OAuth, DeviceFlow, None)
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 2.3. Connect ACP subprocess sessions through `agent_servers` and the existing ACP thread boundary
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 2.4. Add a functional configuration view only for approved ACP presets
  - Reuse `agent_servers` binary discovery, launch, authentication, and ACP-over-stdio transport
  - Do not register a stub provider or expose a model whose completion path always fails

  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_models/src/provider/acp_subprocess.rs_
  - _Writes: crates/language_models/src/provider/acp_subprocess.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Implement cloud API providers: Azure, GCP Vertex AI
  - [ ] 3.1. Azure provider — Azure OpenAI API format, auth, model list
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 3.2. GCP Vertex AI — GCP auth, Vertex AI endpoint, model support

  - _Requirements: 1.2_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_models/src/provider/gcp_vertex_ai.rs_
  - _Writes: crates/language_models/src/provider/gcp_vertex_ai.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Implement cloud API providers: HuggingFace, LiteLLM, Snowflake, Sagemaker TGI
  - [ ] 4.1. HuggingFace provider — Inference API, model resolution
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 4.2. LiteLLM provider — proxy interface, model routing
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 4.3. Snowflake provider — Cortex AI API
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 4.4. Sagemaker TGI provider — endpoint configuration, model serving

  - _Requirements: 1.6_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_models/src/provider/sagemaker_tgi.rs_
  - _Writes: crates/language_models/src/provider/sagemaker_tgi.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Implement consumer API providers: NanoGPT, Tetrate, Avian, KimiCode
  - [ ] 5.1. NanoGPT provider
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 5.2. Tetrate provider
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 5.3. Avian provider
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 5.4. KimiCode provider

  - _Requirements: 2.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_models/src/provider/kimicode.rs_
  - _Writes: crates/language_models/src/provider/kimicode.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Implement Databricks providers (v1 + v2) and Local Inference
  - [ ] 6.1. Databricks v1 — legacy API
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 6.2. Databricks v2 — new API
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 6.3. Local inference provider — stub with Ollama/llama.cpp models, guidance to use OpenAI-compatible provider

  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_models/src/provider/local_inference.rs_
  - _Writes: crates/language_models/src/provider/local_inference.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 7. Implement the initially approved ACP/CLI presets: Claude ACP, Claude Code, ChatGPT/Codex, Cursor Agent, Gemini CLI
  - [ ] 7.1. Claude ACP provider
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 7.2. Claude Code provider
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 7.3. ChatGPT/Codex provider
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 7.4. Cursor Agent provider
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
    - _Depends on: none_
    - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_
    - _Writes: crates/language_
    - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_
  - [ ] 7.5. Gemini CLI provider (OAuth + subprocess)

  - _Requirements: 3.5_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_models/src/provider/gemini_cli.rs_
  - _Writes: crates/language_models/src/provider/gemini_cli.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 8. Reconcile provider normalization, canonical metadata, and estimated usage
  - Reuse `language_model_core` request, response, error, image, tool, thinking, retry, and usage types
  - Add only missing canonical metadata and clearly labeled usage estimation behavior
  - Add metadata identity, normalization, fallback-estimate labeling, and unsupported-tokenizer tests

  - _Requirements: 8.1, 8.2, 8.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/providers/usage_estimator.rs, projects/goose/crates/goose-provider-types/src/canonical/, crates/language_model_core/src/language_model.rs, crates/language_models/src/language_models.rs_
  - _Writes: existing language_model_core and language_models provider metadata/usage files selected after reconciliation_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 9. Write tests for all new providers
  - Cover optional provider settings without panics or silent invalid defaults
  - Cover registry initialization through `SettingsStore::test` and repository test runtime conventions
  - Add per-provider streaming, tool, image/thinking, usage, cancellation, auth, and normalized error tests

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.1, 4.2, 4.3, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3, 9.1, 9.2, 9.3, 9.4, 9.5, 10.1, 10.2, 10.3, 10.4, 11.1, 11.2, 11.3, 11.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_models/src/provider/tests/_
  - _Writes: crates/language_models/src/provider/tests/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 10. Add local OpenAI-compatible provider configuration UI
  - Add a local inference preset to the LLM provider configuration UI for Ollama and llama.cpp.
  - Prefill `http://localhost:11434/v1` and allow saving local endpoints without a user-entered API key.
  - Keep local inference on the existing OpenAI-compatible provider path so configured models are registered by `LanguageModelRegistry`.

  - _Requirements: 5.1, 5.2, 6.3, 6.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/additional-llm-providers/requirements.md, .agents/specs/goose-migration/additional-llm-providers/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent_ui/src/agent_configuration.rs, crates/agent_ui/src/agent_configuration/add_llm_provider_modal.rs_
  - _Writes: crates/agent_ui/src/agent_configuration.rs, crates/agent_ui/src/agent_configuration/add_llm_provider_modal.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 11. Add approved local model discovery and management behavior
  - Reuse existing llama.cpp, credentials, HTTP, cache, and model-configuration owners
  - Cover search/compatibility, authenticated multi-file downloads, progress, cancellation/resume, integrity, eviction, and destructive deletion
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Depends on: 6_
  - _Reads: projects/goose/crates/goose-local-inference/src, crates/llama_cpp, crates/credentials_provider, crates/http_client, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: existing local inference/model configuration/cache integration files selected after architecture review_
  - _Validation: focused search, compatibility, auth redaction, download/cancel/resume, integrity, resource, eviction, and deletion tests_

- [ ] 12. Complete the approved ACP/CLI preset inventory in the agent-server registry
  - Assess Amp ACP, Codex ACP, Pi ACP, and Copilot ACP alongside the presets in Task 7
  - Keep ACP agents distinct from native language-model providers
  - _Requirements: 10.1, 10.2, 10.3, 10.4_
  - _Depends on: 2, 7_
  - _Reads: projects/goose/crates/goose/src/providers/{amp_acp,codex_acp,pi_acp,copilot_acp}.rs, crates/agent_servers, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: existing agent-server registry, configuration, and UI files selected per approved preset_
  - _Validation: per-preset binary discovery, auth, initialization/capability, working-directory, cancellation, upgrade, and missing-binary tests_

- [ ] 13. Curate declarative provider presets without duplicating compatible providers
  - Inventory every audited JSON definition and record supported, divergent, or excluded status
  - Add approved presets through existing OpenAI/Ollama-compatible configuration
  - _Requirements: 11.1, 11.2, 11.3, 11.4_
  - _Depends on: 1_
  - _Reads: projects/goose/crates/goose-providers/src/declarative/definitions, crates/language_models/src/provider/open_ai_compatible.rs, crates/ollama, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: existing compatible-provider preset/catalog and configuration files selected after support-matrix review_
  - _Validation: catalog inventory, request-contract, invalid-definition isolation, secret-redaction, refresh, and active-provider stability tests_

## Notes

- Each provider implements the `LanguageModelProvider` trait from `crates/language_model_core/`
- ACP-based providers reuse the ACP connection logic from `crates/acp_thread/`
- Feature-gate large SDK dependencies behind Cargo features
