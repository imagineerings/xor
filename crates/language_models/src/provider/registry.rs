use std::sync::Arc;

use collections::HashSet;
use credentials_provider::CredentialsProvider;
use gpui::Context;
use http_client::HttpClient;
use language_model::{LanguageModelProviderId, LanguageModelRegistry};

use crate::provider::{
    anthropic::AnthropicLanguageModelProvider, avian::AvianLanguageModelProvider,
    azure::AzureLanguageModelProvider, bedrock::BedrockLanguageModelProvider, claude_acp,
    claude_code, codex, copilot_chat::CopilotChatLanguageModelProvider, cursor_agent,
    databricks::DatabricksV1LanguageModelProvider, databricks_v2::DatabricksLanguageModelProvider,
    deepseek::DeepSeekLanguageModelProvider, gcp_vertex_ai::GcpVertexAiLanguageModelProvider,
    gemini_cli, google::GoogleLanguageModelProvider, huggingface::HuggingFaceLanguageModelProvider,
    kimicode::KimiCodeLanguageModelProvider, litellm::LiteLlmLanguageModelProvider,
    lmstudio::LmStudioLanguageModelProvider, local_inference::LocalInferenceLanguageModelProvider,
    mistral::MistralLanguageModelProvider, nanogpt::NanoGptLanguageModelProvider,
    ollama::OllamaLanguageModelProvider, open_ai::OpenAiLanguageModelProvider,
    open_ai_compatible::OpenAiCompatibleLanguageModelProvider,
    open_router::OpenRouterLanguageModelProvider, openai_subscribed::OpenAiSubscribedProvider,
    opencode::OpenCodeLanguageModelProvider, sagemaker_tgi::SageMakerTgiLanguageModelProvider,
    snowflake::SnowflakeLanguageModelProvider, tetrate::TetrateLanguageModelProvider,
    vercel_ai_gateway::VercelAiGatewayLanguageModelProvider, x_ai::XAiLanguageModelProvider,
};

/// A registry that knows how to create and register all language model providers.
///
/// This centralizes provider lifecycle management:
/// - Built-in providers are created from factory functions
/// - Declarative (OpenAI-compatible) providers are loaded from settings
/// - All providers are bulk-registered into [`LanguageModelRegistry`] via `register_all`
pub struct ProviderRegistry {
    http_client: Arc<dyn HttpClient>,
    credentials_provider: Arc<dyn CredentialsProvider>,
}

impl ProviderRegistry {
    pub fn new(
        http_client: Arc<dyn HttpClient>,
        credentials_provider: Arc<dyn CredentialsProvider>,
    ) -> Self {
        Self {
            http_client,
            credentials_provider,
        }
    }

    /// Register all built-in and declarative providers into the given
    /// [`LanguageModelRegistry`].  Called once at application startup.
    pub fn register_all(
        self,
        registry: &mut LanguageModelRegistry,
        cx: &mut Context<'_, LanguageModelRegistry>,
    ) {
        self.register_builtin_providers(registry, cx);
    }

    /// Register declarative (OpenAI-compatible) providers from current settings.
    ///
    /// Call this when settings change to sync newly-added or removed
    /// OpenAI-compatible provider entries.
    pub fn register_declarative_providers(
        http_client: Arc<dyn HttpClient>,
        credentials_provider: Arc<dyn CredentialsProvider>,
        old: &HashSet<Arc<str>>,
        new: &HashSet<Arc<str>>,
        registry: &mut LanguageModelRegistry,
        cx: &mut Context<'_, LanguageModelRegistry>,
    ) {
        for provider_id in old {
            if !new.contains(provider_id) {
                registry
                    .unregister_provider(LanguageModelProviderId::from(provider_id.clone()), cx);
            }
        }

        for provider_id in new {
            if !old.contains(provider_id) {
                registry.register_provider(
                    Arc::new(OpenAiCompatibleLanguageModelProvider::new(
                        provider_id.clone(),
                        http_client.clone(),
                        credentials_provider.clone(),
                        cx,
                    )),
                    cx,
                );
            }
        }
    }

    fn register_builtin_providers(
        &self,
        registry: &mut LanguageModelRegistry,
        cx: &mut Context<'_, LanguageModelRegistry>,
    ) {
        let client = ClientRef {
            http_client: self.http_client.clone(),
            credentials_provider: self.credentials_provider.clone(),
        };

        // ── Cloud (sim.dev) ──────────────────────────────────────────
        // (needs user_store, done separately in language_models::init)
        //
        // ── NanoGPT ────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(NanoGptLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Tetrate ─────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(TetrateLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Avian ───────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(AvianLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── KimiCode ────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(KimiCodeLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── SageMaker TGI ────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(SageMakerTgiLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Snowflake ────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(SnowflakeLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Databricks v2 (Foundation Model API) ────────────────────────────
        registry.register_provider(
            Arc::new(DatabricksLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Databricks v1 (Legacy Serving Endpoints) ────────────────────────
        registry.register_provider(
            Arc::new(DatabricksV1LanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Claude ACP ───────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(claude_acp::provider(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Claude Code ──────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(claude_code::provider(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Codex ────────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(codex::provider(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Cursor Agent ─────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(cursor_agent::provider(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Gemini CLI ───────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(gemini_cli::provider(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── LiteLLM ────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(LiteLlmLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── HuggingFace ────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(HuggingFaceLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── GCP Vertex AI ─────────────────────────────────────────────
        registry.register_provider(
            Arc::new(GcpVertexAiLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Azure OpenAI ────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(AzureLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Anthropic ────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(AnthropicLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── OpenAI ──────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(OpenAiLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Ollama ──────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(OllamaLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── LM Studio ───────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(LmStudioLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── DeepSeek ────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(DeepSeekLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Google AI ───────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(GoogleLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Mistral ─────────────────────────────────────────────────────
        registry.register_provider(
            MistralLanguageModelProvider::global(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            ),
            cx,
        );
        // ── Amazon Bedrock ──────────────────────────────────────────────
        registry.register_provider(
            Arc::new(BedrockLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── OpenRouter ──────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(OpenRouterLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Vercel AI Gateway ───────────────────────────────────────────
        registry.register_provider(
            Arc::new(VercelAiGatewayLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── xAI ─────────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(XAiLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── OpenCode ────────────────────────────────────────────────────
        registry.register_provider(
            Arc::new(OpenCodeLanguageModelProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Copilot Chat ────────────────────────────────────────────────
        registry.register_provider(Arc::new(CopilotChatLanguageModelProvider::new(cx)), cx);
        // ── OpenAI Subscribed ───────────────────────────────────────────
        registry.register_provider(
            Arc::new(OpenAiSubscribedProvider::new(
                client.http_client.clone(),
                client.credentials_provider.clone(),
                cx,
            )),
            cx,
        );
        // ── Local Inference ───────────────────────────────────────────────
        registry.register_provider(Arc::new(LocalInferenceLanguageModelProvider::new(cx)), cx);
    }
}

/// Small bundle of shared dependencies needed to construct most providers.
struct ClientRef {
    http_client: Arc<dyn HttpClient>,
    credentials_provider: Arc<dyn CredentialsProvider>,
}
