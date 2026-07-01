use crate::{LanguageModelProviderId, LanguageModelProviderName};

pub const ANTHROPIC_PROVIDER_ID: LanguageModelProviderId =
    LanguageModelProviderId::new("anthropic");
pub const ANTHROPIC_PROVIDER_NAME: LanguageModelProviderName =
    LanguageModelProviderName::new("Anthropic");

pub const OPEN_AI_PROVIDER_ID: LanguageModelProviderId = LanguageModelProviderId::new("openai");
pub const OPEN_AI_PROVIDER_NAME: LanguageModelProviderName =
    LanguageModelProviderName::new("OpenAI");

pub const GOOGLE_PROVIDER_ID: LanguageModelProviderId = LanguageModelProviderId::new("google");
pub const GOOGLE_PROVIDER_NAME: LanguageModelProviderName =
    LanguageModelProviderName::new("Google AI");

pub const X_AI_PROVIDER_ID: LanguageModelProviderId = LanguageModelProviderId::new("x_ai");
pub const X_AI_PROVIDER_NAME: LanguageModelProviderName = LanguageModelProviderName::new("xAI");

pub const BAYMAX_CLOUD_PROVIDER_ID: LanguageModelProviderId =
    LanguageModelProviderId::new("baymax.dev");
pub const BAYMAX_CLOUD_PROVIDER_NAME: LanguageModelProviderName =
    LanguageModelProviderName::new("Baymax");

/// An embedding provider generates vector embeddings from text input.
///
/// This is a separate trait from [`LanguageModelProvider`] because embedding
/// models serve a different purpose (RAG, semantic search) and may come from
/// different providers than the chat/completion model.
pub trait EmbeddingProvider: Send + Sync {
    /// The unique identifier for this embedding provider.
    fn id(&self) -> LanguageModelProviderId;

    /// A human-readable name for this embedding provider.
    fn name(&self) -> LanguageModelProviderName;

    /// Generate embeddings for the given list of input strings.
    ///
    /// Returns a vector of embedding vectors, one per input string.  Each
    /// embedding vector is a fixed-size float vector whose dimensionality
    /// depends on the model.
    fn embed(
        &self,
        input: Vec<String>,
        cx: &gpui_shared_string::SharedString,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<Vec<Vec<f32>>>> + Send + 'static>,
    >;

    /// The maximum number of input strings the provider can accept in a single
    /// embed request.
    fn max_batch_size(&self) -> usize {
        16
    }

    /// The dimensionality of the embedding vectors produced by this provider.
    fn embedding_dimension(&self) -> usize;

    /// The name of the embedding model used by this provider.
    fn model_name(&self) -> &str;
}
