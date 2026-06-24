# Implementation Plan: Additional LLM Providers

## Overview

Implement the 18+ provider integrations from goose that don't yet exist in baymax, following the patterns established in `crates/language_models/src/provider/`. Work is grouped: first the shared infrastructure (registry, declarative providers, ACP adapter), then individual providers by category.

## Tasks

- [ ] 1. Implement provider registry and declarative providers
  - Create provider registry (`ProviderRegistry`) that maps provider names to factory functions
  - Implement declarative provider config loading from settings files
  - Add embedding provider trait alongside existing LanguageModel trait
  - _Requirements: 6, 7, 8_
  - _writes: crates/language_models/src/provider/registry.rs, crates/language_models/src/provider/declarative.rs_

- [ ] 2. Implement ACP-based provider adapter
  - Create `AcpSubprocessProvider` that spawns a binary and communicates via ACP
  - Handle binary discovery, process lifecycle, and auth methods
  - _Requirements: 3_
  - _writes: crates/language_models/src/provider/acp_subprocess.rs_

- [ ] 3. Implement cloud API providers: Azure, GCP Vertex AI
  - [ ] 3.1 Azure provider — Azure OpenAI API format, auth, model list
    - _Requirements: 1.1_
    - _writes: crates/language_models/src/provider/azure.rs_ (or new crate)
  - [ ] 3.2 GCP Vertex AI — GCP auth, Vertex AI endpoint, model support
    - _Requirements: 1.2_
    - _writes: crates/language_models/src/provider/gcp_vertex_ai.rs_

- [ ] 4. Implement cloud API providers: HuggingFace, LiteLLM, Snowflake, Sagemaker TGI
  - [ ] 4.1 HuggingFace provider — Inference API, model resolution
    - _Requirements: 1.3_
    - _writes: crates/language_models/src/provider/huggingface.rs_
  - [ ] 4.2 LiteLLM provider — proxy interface, model routing
    - _Requirements: 1.4_
    - _writes: crates/language_models/src/provider/litellm.rs_
  - [ ] 4.3 Snowflake provider — Cortex AI API
    - _Requirements: 1.5_
    - _writes: crates/language_models/src/provider/snowflake.rs_
  - [ ] 4.4 Sagemaker TGI provider — endpoint configuration, model serving
    - _Requirements: 1.6_
    - _writes: crates/language_models/src/provider/sagemaker_tgi.rs_

- [ ] 5. Implement consumer API providers: NanoGPT, Tetrate, Avian, KimiCode
  - [ ] 5.1 NanoGPT provider
    - _Requirements: 2.1_
    - _writes: crates/language_models/src/provider/nanogpt.rs_
  - [ ] 5.2 Tetrate provider
    - _Requirements: 2.2_
    - _writes: crates/language_models/src/provider/tetrate.rs_
  - [ ] 5.3 Avian provider
    - _Requirements: 2.3_
    - _writes: crates/language_models/src/provider/avian.rs_
  - [ ] 5.4 KimiCode provider
    - _Requirements: 2.4_
    - _writes: crates/language_models/src/provider/kimicode.rs_

- [ ] 6. Implement Databricks providers (v1 + v2) and Local Inference
  - [ ] 6.1 Databricks v1 — legacy API
    - _Requirements: 4.1_
    - _writes: crates/language_models/src/provider/databricks.rs_
  - [ ] 6.2 Databricks v2 — new API
    - _Requirements: 4.2_
    - _writes: crates/language_models/src/provider/databricks_v2.rs_
  - [ ] 6.3 Local inference provider — candle-based model loading
    - _Requirements: 5_
    - _writes: crates/language_models/src/provider/local_inference.rs_

- [ ] 7. Implement ACP/CLI-based providers: Claude ACP, Claude Code, ChatGPT/Codex, Cursor Agent, Gemini CLI
  - [ ] 7.1 Claude ACP provider
    - _Requirements: 3.1_
    - _writes: crates/language_models/src/provider/claude_acp.rs_
  - [ ] 7.2 Claude Code provider
    - _Requirements: 3.2_
    - _writes: crates/language_models/src/provider/claude_code.rs_
  - [ ] 7.3 ChatGPT/Codex provider
    - _Requirements: 3.3_
    - _writes: crates/language_models/src/provider/codex.rs_
  - [ ] 7.4 Cursor Agent provider
    - _Requirements: 3.4_
    - _writes: crates/language_models/src/provider/cursor_agent.rs_
  - [ ] 7.5 Gemini CLI provider (OAuth + subprocess)
    - _Requirements: 3.5_
    - _writes: crates/language_models/src/provider/gemini_cli.rs_

- [ ] 8. Implement embedding providers
  - Define embedding provider trait
  - Implement embedding providers for supported backends
  - _Requirements: 8_
  - _writes: crates/language_models/src/provider/embedding.rs_

- [ ] 9. Write tests for all new providers
  - Unit tests for request formatting and response parsing per provider
  - Registry tests (registration, lookup, duplicates, missing)
  - Declarative provider config validation tests
  - _Requirements: 1-8_
  - _writes: crates/language_models/src/provider/tests/_

## Notes

- Each provider implements the `LanguageModelProvider` trait from `crates/language_model_core/`
- ACP-based providers reuse the ACP connection logic from `crates/acp_thread/`
- Feature-gate large SDK dependencies behind Cargo features
