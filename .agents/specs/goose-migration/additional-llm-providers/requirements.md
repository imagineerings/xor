# Requirements: Additional LLM Providers

## Introduction

Migrate the LLM provider integrations from goose that do not yet have equivalents in zed. Goose includes 20+ provider implementations that extend zed's current provider ecosystem (which already covers Anthropic, OpenAI, Google, Bedrock, Ollama, OpenRouter, DeepSeek, Mistral, xAI, LM Studio, Copilot Chat, and OpenAI-compatible).

## Glossary

- **Provider**: An LLM API service implementation that conforms to zed's `language_model` trait
- **ACP Provider**: A provider that communicates via the Agent-Client Protocol
- **Declarative Provider**: A provider defined via configuration rather than code
- **Embedding Provider**: A provider that generates embeddings rather than chat completions
- **Local Inference**: Running LLM inference locally on the user's machine
- **Provider Registry**: Central registry that maps provider names to their implementations

## Requirements

### Requirement 1: Cloud API Providers

**User Story:** As a zed user, I want to use Azure OpenAI, GCP Vertex AI, HuggingFace, LiteLLM, Snowflake, and Sagemaker TGI as LLM providers, so that I can leverage enterprise cloud AI services.

#### Acceptance Criteria

1. **1.1** WHEN a user configures Azure OpenAI as their provider THEN the system SHALL send requests using Azure's API format and authentication
2. **1.2** WHEN a user configures GCP Vertex AI as their provider THEN the system SHALL authenticate via GCP credentials and use Vertex AI's API
3. **1.3** WHEN a user configures HuggingFace as their provider THEN the system SHALL use the HuggingFace Inference API
4. **1.4** WHEN a user configures LiteLLM as their provider THEN the system SHALL route requests through LiteLLM's proxy interface
5. **1.5** WHEN a user configures Snowflake as their provider THEN the system SHALL use Snowflake's Cortex AI API
6. **1.6** WHEN a user configures Sagemaker TGI as their provider THEN the system SHALL use AWS Sagemaker's Text Generation Inference endpoint
7. **1.7** IF any cloud provider returns an authentication error THEN the system SHALL surface a clear error message indicating the credential issue

### Requirement 2: Consumer API Providers

**User Story:** As a zed user, I want to use NanoGPT, Tetrate, Avian, and KimiCode as LLM providers, so that I can access additional LLM API services.

#### Acceptance Criteria

1. **2.1** WHEN a user configures NanoGPT as their provider THEN the system SHALL use NanoGPT's API
2. **2.2** WHEN a user configures Tetrate as their provider THEN the system SHALL use Tetrate's API
3. **2.3** WHEN a user configures Avian as their provider THEN the system SHALL use Avian's API
4. **2.4** WHEN a user configures KimiCode as their provider THEN the system SHALL use KimiCode's API
5. **2.5** THE provider implementation SHALL support streaming responses where the upstream API supports it

### Requirement 3: ACP-Based Providers

**User Story:** As a zed user, I want to use Claude ACP, Claude Code, ChatGPT/Codex (Codex CLI), Cursor Agent, and Gemini CLI as providers, so that I can leverage my existing subscriptions.

#### Acceptance Criteria

1. **3.1** WHEN a user configures Claude ACP as their provider THEN the system SHALL communicate via the Agent-Client Protocol
2. **3.2** WHEN a user configures Claude Code as their provider THEN the system SHALL spawn and communicate with the Claude Code CLI
3. **3.3** WHEN a user configures ChatGPT/Codex as their provider THEN the system SHALL use the Codex CLI interface
4. **3.4** WHEN a user configures Cursor Agent as their provider THEN the system SHALL use Cursor's agent protocol
5. **3.5** WHEN a user configures Gemini CLI as their provider THEN the system SHALL authenticate via OAuth and spawn the Gemini CLI
6. **3.6** IF the ACP provider binary is not found THEN the system SHALL show an actionable error message

### Requirement 4: Databricks Provider

**User Story:** As a zed user, I want to use Databricks as an LLM provider with both v1 and v2 API support, so that I can use Databricks-hosted models.

#### Acceptance Criteria

1. **4.1** WHEN a user configures Databricks v1 as their provider THEN the system SHALL use the legacy Databricks API
2. **4.2** WHEN a user configures Databricks v2 as their provider THEN the system SHALL use the new Databricks API
3. **4.3** THE Databricks provider SHALL support Databricks-hosted model serving endpoints

### Requirement 5: Local Inference

**User Story:** As a zed user, I want to run LLM inference locally without a cloud provider, so that I can work offline and keep data private.

#### Acceptance Criteria

1. **5.1** WHEN a user enables local inference via an OpenAI-compatible local server THEN the system SHALL allow configuring the provider with a local `/v1` endpoint such as `http://localhost:11434/v1`
2. **5.2** WHEN a local Ollama or llama.cpp server does not require authentication THEN the system SHALL allow saving the provider without a user-entered API key
3. **5.3** IF a local inference provider is selected while its local server is not running THEN the system SHALL display a clear error that instructs the user to start Ollama or llama.cpp and configure the OpenAI-compatible endpoint
4. **5.4** WHERE Zed-owned in-process inference is approved, THE system SHALL extend the existing `llama_cpp` integration and explicitly decide whether audited Goose MLX behavior is supported; it SHALL NOT introduce Candle solely from the obsolete plan.
5. **5.5** IF local inference hardware is insufficient THEN the system SHALL display a clear error
6. **5.6** WHILE local inference is running THE system SHALL handle resource constraints gracefully

### Requirement 6: Declarative Providers

**User Story:** As a zed user, I want to define custom providers via configuration files or UI, so that I can use any OpenAI-compatible API without writing code.

#### Acceptance Criteria

1. **6.1** WHEN a user creates a provider config file THEN the system SHALL register it as an available provider
2. **6.2** THE declarative provider SHALL support base URL, API key, model list, and custom headers configuration
3. **6.3** WHEN a user opens the LLM provider configuration UI THEN the system SHALL provide an OpenAI-compatible provider flow and a local inference preset for Ollama or llama.cpp
4. **6.4** WHEN a user configures a local inference preset THEN the system SHALL prefill a local OpenAI-compatible API URL and allow editing the model list
5. **6.5** IF the declarative provider config is invalid THEN the system SHALL show validation errors

### Requirement 7: Provider Registry

**User Story:** As a zed developer, I want a centralized provider registry, so that providers can be discovered and instantiated by name.

#### Acceptance Criteria

1. **7.1** THE system SHALL maintain a registry mapping provider names to their factory functions
2. **7.2** WHEN a user specifies a provider by name THEN the system SHALL look it up in the registry
3. **7.3** IF a provider name is not found in the registry THEN the system SHALL return a clear "unknown provider" error

### Requirement 8: Provider Contract and Canonical Metadata

**User Story:** As a zed user, I want provider behavior and model metadata normalized consistently, so that switching providers does not produce silent compatibility or usage errors.

#### Acceptance Criteria

1. **8.1** THE provider integration SHALL normalize streaming messages, tool requests/results, images, thinking content, token usage, and retryable errors into Zed's existing language-model core types.
2. **8.2** THE provider registry SHALL expose canonical model identity and capability metadata where the upstream catalog supplies it.
3. **8.3** WHEN a provider omits usage, THE system SHALL label any fallback estimate and SHALL NOT present estimated cost or tokens as exact.

### Requirement 9: Local Model Management

**User Story:** As a local-inference user, I want to find, download, manage, and remove compatible models, so that local inference is usable without manual cache manipulation.

#### Acceptance Criteria

1. **9.1** WHERE in-process local inference is approved, THE system SHALL search supported Hugging Face GGUF/MLX model variants and report compatibility metadata.
2. **9.2** THE system SHALL download one or more model files with authenticated access, progress, cancellation, resume, integrity/size checks, and disk/permission errors.
3. **9.3** THE system SHALL list downloaded and loaded models and SHALL support explicit eviction and deletion with destructive confirmation.
4. **9.4** THE system SHALL store Hugging Face tokens through Zed's credentials provider and SHALL redact them from settings, logs, errors, and telemetry.
5. **9.5** IF a model or platform is unsupported or memory is insufficient, THEN THE system SHALL explain the incompatibility without leaving a falsely usable provider entry.

### Requirement 10: ACP and CLI Provider Preset Inventory

**User Story:** As a subscription-backed agent user, I want the audited preset inventory represented accurately, so that supported external agents can be configured without duplicate provider implementations.

#### Acceptance Criteria

1. **10.1** THE migration SHALL assess Amp ACP, Claude ACP, Claude Code, ChatGPT/Codex, Codex ACP, Cursor Agent, Gemini CLI, Pi ACP, and Copilot ACP separately.
2. **10.2** WHERE a preset speaks ACP, THE implementation SHALL register it in Zed's existing agent-server registry unless a review proves language-model-provider ownership is required.
3. **10.3** EACH approved preset SHALL define binary discovery, authentication, capability negotiation, working-directory behavior, cancellation, upgrade compatibility, and actionable missing-binary errors.
4. **10.4** THE UI and CLI SHALL distinguish an external ACP agent from an in-process language-model provider.

### Requirement 11: Curated Declarative Provider Catalog

**User Story:** As a user of an OpenAI/Ollama-compatible service, I want an accurate preset when protocol compatibility is known, so that Zed does not maintain unnecessary provider code.

#### Acceptance Criteria

1. **11.1** THE migration SHALL inventory every audited definition in `goose-providers/src/declarative/definitions` and classify it as reusable Zed support, approved preset, unsupported deviation, or intentionally excluded.
2. **11.2** AN approved preset SHALL declare endpoint, authentication, model source, headers, streaming/tool/image/thinking capabilities, and canonical filtering behavior.
3. **11.3** INVALID or conflicting custom/declarative definitions SHALL be isolated and reported without preventing built-in providers from loading.
4. **11.4** THE provider catalog SHALL support safe refresh without exposing secrets or replacing an active provider with an invalid definition.

## References

- Source: `projects/goose/crates/goose/src/providers/`, `projects/goose/crates/goose-providers/src/`, and `projects/goose/crates/goose-local-inference/src/`
- Existing zed providers: `crates/language_models/src/provider/`
