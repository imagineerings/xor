use anyhow::{Context, Result};
use futures::io::AsyncReadExt;
use gpui::SharedString;
use http_client::{AsyncBody, HttpClient};
use language_model::{EmbeddingProvider, LanguageModelProviderId, LanguageModelProviderName};
use std::sync::Arc;

// ── Provider IDs ────────────────────────────────────────────────────────────

pub const OPENAI_EMBEDDING_PROVIDER_ID: LanguageModelProviderId =
    LanguageModelProviderId::new("openai_embedding");
pub const OPENAI_EMBEDDING_PROVIDER_NAME: LanguageModelProviderName =
    LanguageModelProviderName::new("OpenAI Embeddings");

pub const LOCAL_EMBEDDING_PROVIDER_ID: LanguageModelProviderId =
    LanguageModelProviderId::new("local_embedding");
pub const LOCAL_EMBEDDING_PROVIDER_NAME: LanguageModelProviderName =
    LanguageModelProviderName::new("Local Embeddings");

// ── OpenAI Embedding Provider ───────────────────────────────────────────────

/// An embedding provider that uses OpenAI's embedding API.
pub struct OpenAiEmbeddingProvider {
    http_client: Arc<dyn HttpClient>,
    api_key: String,
    model: String,
    dimension: usize,
}

impl OpenAiEmbeddingProvider {
    pub fn new(http_client: Arc<dyn HttpClient>, api_key: String, model: Option<String>) -> Self {
        Self {
            http_client,
            api_key,
            model: model.unwrap_or_else(|| "text-embedding-3-small".to_string()),
            dimension: 1536,
        }
    }
}

impl EmbeddingProvider for OpenAiEmbeddingProvider {
    fn id(&self) -> LanguageModelProviderId {
        OPENAI_EMBEDDING_PROVIDER_ID
    }

    fn name(&self) -> LanguageModelProviderName {
        OPENAI_EMBEDDING_PROVIDER_NAME
    }

    fn embed(
        &self,
        input: Vec<String>,
        _cx: &SharedString,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Vec<f32>>>> + Send + 'static>>
    {
        let client = self.http_client.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();

        Box::pin(async move {
            let body = serde_json::json!({
                "input": input,
                "model": model,
            });

            let request = http::Request::post("https://api.openai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(AsyncBody::from(serde_json::to_string(&body)?))
                .context("failed to build embedding request")?;

            let response = client.send(request).await?;
            let status = response.status();
            let mut response_body = String::new();
            response
                .into_body()
                .read_to_string(&mut response_body)
                .await
                .context("failed to read embedding response body")?;

            if !status.is_success() {
                let snippet = &response_body[..response_body.len().min(256)];
                anyhow::bail!("OpenAI embedding API error ({}): {}", status, snippet);
            }

            let data: serde_json::Value = serde_json::from_str(&response_body)
                .context("failed to parse embedding response")?;

            let embeddings = data["data"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("missing 'data' in embedding response"))?
                .iter()
                .map(|item| {
                    item["embedding"]
                        .as_array()
                        .ok_or_else(|| anyhow::anyhow!("missing 'embedding' in data item"))
                        .map(|arr| {
                            arr.iter()
                                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                                .collect::<Vec<f32>>()
                        })
                })
                .collect::<Result<Vec<Vec<f32>>>>()?;

            Ok(embeddings)
        })
    }

    fn max_batch_size(&self) -> usize {
        2048
    }

    fn embedding_dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

// ── Local Embedding Provider (stub) ─────────────────────────────────────────

/// A local embedding provider using candle-based models.
///
/// This is a stub for now. The full implementation requires the candle ML
/// framework dependencies, which is tracked as a separate effort.
pub struct LocalEmbeddingProvider;

impl LocalEmbeddingProvider {
    pub fn new() -> Self {
        Self
    }
}

impl EmbeddingProvider for LocalEmbeddingProvider {
    fn id(&self) -> LanguageModelProviderId {
        LOCAL_EMBEDDING_PROVIDER_ID
    }

    fn name(&self) -> LanguageModelProviderName {
        LOCAL_EMBEDDING_PROVIDER_NAME
    }

    fn embed(
        &self,
        _input: Vec<String>,
        _cx: &SharedString,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Vec<f32>>>> + Send + 'static>>
    {
        Box::pin(async {
            anyhow::bail!(
                "Local embedding inference requires the candle ML framework, \
                 which is not yet wired into the language_models crate."
            )
        })
    }

    fn max_batch_size(&self) -> usize {
        1
    }

    fn embedding_dimension(&self) -> usize {
        384
    }

    fn model_name(&self) -> &str {
        "local (stub)"
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_embedding_provider_has_correct_id() {
        let client = http_client::FakeHttpClient::with_404_response();
        let provider = OpenAiEmbeddingProvider::new(client, "sk-test".into(), None);
        assert_eq!(provider.id(), OPENAI_EMBEDDING_PROVIDER_ID);
        assert_eq!(provider.name(), OPENAI_EMBEDDING_PROVIDER_NAME);
        assert_eq!(provider.model_name(), "text-embedding-3-small");
        assert_eq!(provider.embedding_dimension(), 1536);
        assert_eq!(provider.max_batch_size(), 2048);
    }

    #[test]
    fn test_local_embedding_provider_has_correct_id() {
        let provider = LocalEmbeddingProvider::new();
        assert_eq!(provider.id(), LOCAL_EMBEDDING_PROVIDER_ID);
        assert_eq!(provider.name(), LOCAL_EMBEDDING_PROVIDER_NAME);
        assert_eq!(provider.embedding_dimension(), 384);
    }

    #[test]
    fn test_openai_embedding_provider_custom_model() {
        let client = http_client::FakeHttpClient::with_404_response();
        let provider = OpenAiEmbeddingProvider::new(
            client,
            "sk-test".into(),
            Some("text-embedding-3-large".into()),
        );
        assert_eq!(provider.model_name(), "text-embedding-3-large");
        assert_eq!(provider.embedding_dimension(), 1536);
    }
}
