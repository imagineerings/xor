# Requirements: Additional LLM Providers

## Introduction

Migrate the LLM provider integrations from goose that do not yet have equivalents in sim. Goose includes 20+ provider implementations that extend sim's current provider ecosystem (which already covers Anthropic, OpenAI, Google, Bedrock, Ollama, OpenRouter, DeepSeek, Mistral, xAI, LM Studio, Copilot Chat, and OpenAI-compatible).

## Glossary

- **Provider**: An LLM API service implementation that conforms to sim's `language_model` trait
- **ACP Provider**: A provider that communicates via the Agent-Client Protocol
- **Declarative Provider**: A provider defined via configuration rather than code
- **Embedding Provider**: A provider that generates embeddings rather than chat completions
- **Local Inference**: Running LLM inference locally on the user's machine
- **Provider Registry**: Central registry that maps provider names to their implementations

## Requirements

### Requirement 1: Cloud API Providers

**User Story:** As a sim user, I want to use Azure OpenAI, GCP Vertex AI, HuggingFace, LiteLLM, Snowflake, and Sagemaker TGI as LLM providers, so that I can leverage enterprise cloud AI services.

#### Acceptance Criteria

1. WHEN a user configures Azure OpenAI as their provider THEN the system SHALL send requests using Azure's API format and authentication
2. WHEN a user configures GCP Vertex AI as their provider THEN the system SHALL authenticate via GCP credentials and use Vertex AI's API
3. WHEN a user configures HuggingFace as their provider THEN the system SHALL use the HuggingFace Inference API
4. WHEN a user configures LiteLLM as their provider THEN the system SHALL route requests through LiteLLM's proxy interface
5. WHEN a user configures Snowflake as their provider THEN the system SHALL use Snowflake's Cortex AI API
6. WHEN a user configures Sagemaker TGI as their provider THEN the system SHALL use AWS Sagemaker's Text Generation Inference endpoint
7. IF any cloud provider returns an authentication error THEN the system SHALL surface a clear error message indicating the credential issue

### Requirement 2: Consumer API Providers

**User Story:** As a sim user, I want to use NanoGPT, Tetrate, Avian, and KimiCode as LLM providers, so that I can access additional LLM API services.

#### Acceptance Criteria

1. WHEN a user configures NanoGPT as their provider THEN the system SHALL use NanoGPT's API
2. WHEN a user configures Tetrate as their provider THEN the system SHALL use Tetrate's API
3. WHEN a user configures Avian as their provider THEN the system SHALL use Avian's API
4. WHEN a user configures KimiCode as their provider THEN the system SHALL use KimiCode's API
5. THE provider implementation SHALL support streaming responses where the upstream API supports it

### Requirement 3: ACP-Based Providers

**User Story:** As a sim user, I want to use Claude ACP, Claude Code, ChatGPT/Codex (Codex CLI), Cursor Agent, and Gemini CLI as providers, so that I can leverage my existing subscriptions.

#### Acceptance Criteria

1. WHEN a user configures Claude ACP as their provider THEN the system SHALL communicate via the Agent-Client Protocol
2. WHEN a user configures Claude Code as their provider THEN the system SHALL spawn and communicate with the Claude Code CLI
3. WHEN a user configures ChatGPT/Codex as their provider THEN the system SHALL use the Codex CLI interface
4. WHEN a user configures Cursor Agent as their provider THEN the system SHALL use Cursor's agent protocol
5. WHEN a user configures Gemini CLI as their provider THEN the system SHALL authenticate via OAuth and spawn the Gemini CLI
6. IF the ACP provider binary is not found THEN the system SHALL show an actionable error message

### Requirement 4: Databricks Provider

**User Story:** As a sim user, I want to use Databricks as an LLM provider with both v1 and v2 API support, so that I can use Databricks-hosted models.

#### Acceptance Criteria

1. WHEN a user configures Databricks v1 as their provider THEN the system SHALL use the legacy Databricks API
2. WHEN a user configures Databricks v2 as their provider THEN the system SHALL use the new Databricks API
3. THE Databricks provider SHALL support Databricks-hosted model serving endpoints

### Requirement 5: Local Inference

**User Story:** As a sim user, I want to run LLM inference locally without a cloud provider, so that I can work offline and keep data private.

#### Acceptance Criteria

1. WHEN a user enables local inference via an OpenAI-compatible local server THEN the system SHALL allow configuring the provider with a local `/v1` endpoint such as `http://localhost:11434/v1`
2. WHEN a local Ollama or llama.cpp server does not require authentication THEN the system SHALL allow saving the provider without a user-entered API key
3. IF a local inference provider is selected while its local server is not running THEN the system SHALL display a clear error that instructs the user to start Ollama or llama.cpp and configure the OpenAI-compatible endpoint
4. THE local inference SHALL support loading models via candle (ML framework) when Sim-owned in-process local inference is implemented
5. IF local inference hardware is insufficient THEN the system SHALL display a clear error
6. WHILE local inference is running THE system SHALL handle resource constraints gracefully

### Requirement 6: Declarative Providers

**User Story:** As a sim user, I want to define custom providers via configuration files or UI, so that I can use any OpenAI-compatible API without writing code.

#### Acceptance Criteria

1. WHEN a user creates a provider config file THEN the system SHALL register it as an available provider
2. THE declarative provider SHALL support base URL, API key, model list, and custom headers configuration
3. WHEN a user opens the LLM provider configuration UI THEN the system SHALL provide an OpenAI-compatible provider flow and a local inference preset for Ollama or llama.cpp
4. WHEN a user configures a local inference preset THEN the system SHALL prefill a local OpenAI-compatible API URL and allow editing the model list
5. IF the declarative provider config is invalid THEN the system SHALL show validation errors

### Requirement 7: Provider Registry

**User Story:** As a sim developer, I want a centralized provider registry, so that providers can be discovered and instantiated by name.

#### Acceptance Criteria

1. THE system SHALL maintain a registry mapping provider names to their factory functions
2. WHEN a user specifies a provider by name THEN the system SHALL look it up in the registry
3. IF a provider name is not found in the registry THEN the system SHALL return a clear "unknown provider" error

### Requirement 8: Embedding Providers

**User Story:** As a sim user, I want embedding model support, so that I can use embeddings for retrieval-augmented generation and semantic search.

#### Acceptance Criteria

1. THE system SHALL support embedding providers that generate vector embeddings
2. WHEN an embedding provider is configured THEN the system SHALL make it available for embedding tasks
3. THE embedding provider SHALL conform to a standard embedding interface

## References

- Source: `projects/goose/crates/goose/src/providers/` — azure.rs, gcpvertexai.rs, huggingface.rs, litellm.rs, snowflake.rs, sagemaker_tgi.rs, nanogpt.rs, tetrate.rs, avian.rs, kimicode.rs, databricks.rs, databricks_v2.rs, local_inference.rs, claude_acp.rs, claude_code.rs, codex.rs, chatgpt_codex.rs, cursor_agent.rs, gemini_cli.rs, declarative/, provider_registry.rs, embedding.rs, init.rs
- Existing sim providers: `crates/language_models/src/provider/`
