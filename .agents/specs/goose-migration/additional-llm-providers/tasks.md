# Implementation Plan: Additional LLM Providers

## Overview

Implement the 18+ provider integrations from goose that don't yet exist in sim, following the patterns established in `crates/language_models/src/provider/`. Work is grouped: first the shared infrastructure (registry, declarative providers, ACP adapter), then individual providers by category.

## Tasks

- [x] 1. Implement provider registry and declarative providers
  - [x] 1.1 Create `ProviderRegistry` struct with factory registration
  - [x] 1.2 Create `DeclarativeProviderConfig` types and validation
  - [x] 1.3 Add `EmbeddingProvider` trait to `language_model_core`
  - Created `ProviderRegistry` in `crates/language_models/src/provider/registry.rs` that centralizes built-in provider registration
  - Created `DeclarativeProviderConfig` in `crates/language_models/src/provider/declarative.rs` with serde deserialization and validation
  - Added `EmbeddingProvider` trait to `crates/language_model_core/src/provider.rs`
  - Refactored `language_models.rs` to use `ProviderRegistry` instead of inline registration
  - _Requirements: 6, 7, 8_
  - _writes: crates/language_models/src/provider/registry.rs, crates/language_models/src/provider/declarative.rs, crates/language_model_core/src/provider.rs, crates/language_models/src/language_models.rs, crates/language_models/src/provider.rs_

- [x] 2. Implement ACP-based provider adapter
  - [x] 2.1 Create `AcpSubprocessProvider` struct with binary discovery and `LanguageModelProvider` trait impl
  - [x] 2.2 Define `AcpAuthMethod` enum (ApiKey, OAuth, DeviceFlow, None)
  - [x] 2.3 Create `AcpSubprocessLanguageModel` stub with `LanguageModel` trait
  - [x] 2.4 Add placeholder configuration view
  - Created `AcpSubprocessProvider` that discovers a binary on `$PATH` and wraps it as a provider
  - Defined `AcpAuthMethod` enum with four variants for different auth strategies
  - `stream_completion` returns `LanguageModelCompletionError::Other` until ACP-over-stdio transport is wired through
  - _Requirements: 3_
  - _writes: crates/language_models/src/provider/acp_subprocess.rs_

- [x] 3. Implement cloud API providers: Azure, GCP Vertex AI
  - [x] 3.1 Azure provider — Azure OpenAI API format, auth, model list
     - _Requirements: 1.1_
     - _writes: crates/language_models/src/provider/azure.rs_
  - [x] 3.2 GCP Vertex AI — GCP auth, Vertex AI endpoint, model support
    - _Requirements: 1.2_
    - _writes: crates/language_models/src/provider/gcp_vertex_ai.rs_

- [x] 4. Implement cloud API providers: HuggingFace, LiteLLM, Snowflake, Sagemaker TGI
  - [x] 4.1 HuggingFace provider — Inference API, model resolution
    - _Requirements: 1.3_
    - _writes: crates/language_models/src/provider/huggingface.rs_
  - [x] 4.2 LiteLLM provider — proxy interface, model routing
    - _Requirements: 1.4_
    - _writes: crates/language_models/src/provider/litellm.rs_
  - [x] 4.3 Snowflake provider — Cortex AI API
    - _Requirements: 1.5_
    - _writes: crates/language_models/src/provider/snowflake.rs_
  - [x] 4.4 Sagemaker TGI provider — endpoint configuration, model serving
    - _Requirements: 1.6_
    - _writes: crates/language_models/src/provider/sagemaker_tgi.rs_

- [x] 5. Implement consumer API providers: NanoGPT, Tetrate, Avian, KimiCode
  - [x] 5.1 NanoGPT provider
    - _Requirements: 2.1_
    - _writes: crates/language_models/src/provider/nanogpt.rs_
  - [x] 5.2 Tetrate provider
    - _Requirements: 2.2_
    - _writes: crates/language_models/src/provider/tetrate.rs_
  - [x] 5.3 Avian provider
    - _Requirements: 2.3_
    - _writes: crates/language_models/src/provider/avian.rs_
  - [x] 5.4 KimiCode provider
    - _Requirements: 2.4_
    - _writes: crates/language_models/src/provider/kimicode.rs_

- [x] 6. Implement Databricks providers (v1 + v2) and Local Inference
  - [x] 6.1 Databricks v1 — legacy API
    - _Requirements: 4.1_
    - _writes: crates/language_models/src/provider/databricks.rs_
  - [x] 6.2 Databricks v2 — new API
    - _Requirements: 4.2_
    - _writes: crates/language_models/src/provider/databricks_v2.rs_
  - [x] 6.3 Local inference provider — stub with Ollama/llama.cpp models, guidance to use OpenAI-compatible provider
    - _Requirements: 5_
    - _writes: crates/language_models/src/provider/local_inference.rs_

- [x] 7. Implement ACP/CLI-based providers: Claude ACP, Claude Code, ChatGPT/Codex, Cursor Agent, Gemini CLI
  - [x] 7.1 Claude ACP provider
    - _Requirements: 3.1_
    - _writes: crates/language_models/src/provider/claude_acp.rs_
  - [x] 7.2 Claude Code provider
    - _Requirements: 3.2_
    - _writes: crates/language_models/src/provider/claude_code.rs_
  - [x] 7.3 ChatGPT/Codex provider
    - _Requirements: 3.3_
    - _writes: crates/language_models/src/provider/codex.rs_
  - [x] 7.4 Cursor Agent provider
    - _Requirements: 3.4_
    - _writes: crates/language_models/src/provider/cursor_agent.rs_
  - [x] 7.5 Gemini CLI provider (OAuth + subprocess)
    - _Requirements: 3.5_
    - _writes: crates/language_models/src/provider/gemini_cli.rs_

- [x] 8. Implement embedding providers
  - EmbeddingProvider trait already existed in `language_model_core/src/provider.rs`
  - Created OpenAiEmbeddingProvider (text-embedding-3-small/large) in `crates/language_models/src/provider/embedding.rs`
  - Created LocalEmbeddingProvider stub for future candle-based local embeddings
  - 3 unit tests for id/name/dimension/construction
  - _Requirements: 8_
  - _writes: crates/language_models/src/provider/embedding.rs_

- [x] 9. Write tests for all new providers
  - Fixed `from_settings` to use `unwrap_or_default()` for optional providers
  - Fixed settings content structs to derive `Default`
  - Fixed registry tests to use `SettingsStore::test` with proper `GlobalTokio` init
  - All 51 tests passing
  - _Requirements: 1-8_
  - _writes: crates/language_models/src/provider/tests/_

- [x] 10. Add local OpenAI-compatible provider configuration UI
  - Add a local inference preset to the LLM provider configuration UI for Ollama and llama.cpp.
  - Prefill `http://localhost:11434/v1` and allow saving local endpoints without a user-entered API key.
  - Keep local inference on the existing OpenAI-compatible provider path so configured models are registered by `LanguageModelRegistry`.
  - _Requirements: 5.1, 5.2, 6.3, 6.4_
  - _writes: crates/agent_ui/src/agent_configuration.rs, crates/agent_ui/src/agent_configuration/add_llm_provider_modal.rs_

## Notes

- Each provider implements the `LanguageModelProvider` trait from `crates/language_model_core/`
- ACP-based providers reuse the ACP connection logic from `crates/acp_thread/`
- Feature-gate large SDK dependencies behind Cargo features
