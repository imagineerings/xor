use anyhow::Result;
use credentials_provider::CredentialsProvider;
use futures::io::BufReader;
use futures::{
    AsyncBufReadExt, AsyncReadExt, FutureExt, StreamExt, future::BoxFuture, stream::BoxStream,
};
use gpui::{AnyView, App, AppContext, AsyncApp, Context, Entity, SharedString, Task, Window};
use http_client::{AsyncBody, HttpClient, Method, Request as HttpRequest};
use language_model::{
    ApiKeyState, AuthenticateError, EnvVar, IconOrSvg, LanguageModel, LanguageModelCompletionError,
    LanguageModelCompletionEvent, LanguageModelId, LanguageModelName, LanguageModelProvider,
    LanguageModelProviderId, LanguageModelProviderName, LanguageModelProviderState,
    LanguageModelRequest, RateLimiter, env_var,
};
use open_ai::{
    ResponseStreamEvent,
    completion::{OpenAiEventMapper, into_open_ai},
};
use settings::{Settings, SettingsStore};
use std::sync::{Arc, LazyLock};

// ── AzureDeployment (re-exported from settings for use in tests) ───────
pub use settings::AzureDeployment;

// ── Constants ──────────────────────────────────────────────────────────────

const PROVIDER_ID: LanguageModelProviderId = LanguageModelProviderId::new("azure");
const PROVIDER_NAME: LanguageModelProviderName = LanguageModelProviderName::new("Azure OpenAI");

const API_KEY_ENV_VAR_NAME: &str = "AZURE_OPENAI_API_KEY";
static API_KEY_ENV_VAR: LazyLock<EnvVar> = env_var!(API_KEY_ENV_VAR_NAME);

const DEFAULT_API_VERSION: &str = "2024-06-01";

// ── Settings ───────────────────────────────────────────────────────────────

#[derive(Default, Clone, Debug, PartialEq)]
pub struct AzureSettings {
    pub resource_name: String,
    pub deployments: Vec<AzureDeployment>,
    pub api_version: Option<String>,
    pub endpoint: Option<String>,
    pub use_ad_token: bool,
}

// ── Provider ───────────────────────────────────────────────────────────────

pub struct AzureLanguageModelProvider {
    http_client: Arc<dyn HttpClient>,
    state: Entity<State>,
}

pub struct State {
    api_key_state: ApiKeyState,
    credentials_provider: Arc<dyn CredentialsProvider>,
}

impl State {
    fn is_authenticated(&self) -> bool {
        self.api_key_state.has_key()
    }

    fn set_api_key(&mut self, api_key: Option<String>, cx: &mut Context<Self>) -> Task<Result<()>> {
        let credentials_provider = self.credentials_provider.clone();
        let api_url = AzureLanguageModelProvider::api_url(cx);
        self.api_key_state.store(
            api_url,
            api_key,
            |this| &mut this.api_key_state,
            credentials_provider,
            cx,
        )
    }

    fn authenticate(&mut self, cx: &mut Context<Self>) -> Task<Result<(), AuthenticateError>> {
        let credentials_provider = self.credentials_provider.clone();
        let api_url = AzureLanguageModelProvider::api_url(cx);
        self.api_key_state.load_if_needed(
            api_url,
            |this| &mut this.api_key_state,
            credentials_provider,
            cx,
        )
    }
}

impl AzureLanguageModelProvider {
    pub fn new(
        http_client: Arc<dyn HttpClient>,
        credentials_provider: Arc<dyn CredentialsProvider>,
        cx: &mut App,
    ) -> Self {
        let state = cx.new(|cx| {
            cx.observe_global::<SettingsStore>(|this: &mut State, cx| {
                let credentials_provider = this.credentials_provider.clone();
                let api_url = AzureLanguageModelProvider::api_url(cx);
                this.api_key_state.handle_url_change(
                    api_url,
                    |this| &mut this.api_key_state,
                    credentials_provider,
                    cx,
                );
                cx.notify();
            })
            .detach();
            State {
                api_key_state: ApiKeyState::new(Self::api_url(cx), (*API_KEY_ENV_VAR).clone()),
                credentials_provider,
            }
        });

        Self { http_client, state }
    }

    fn settings(cx: &App) -> AzureSettings {
        crate::AllLanguageModelSettings::get_global(cx)
            .azure
            .clone()
    }

    fn api_url(cx: &App) -> SharedString {
        let settings = Self::settings(cx);
        if let Some(endpoint) = settings.endpoint.as_ref() {
            if !endpoint.is_empty() {
                return SharedString::new(endpoint.clone());
            }
        }
        let resource = &settings.resource_name;
        if resource.is_empty() {
            return SharedString::from("https://.openai.azure.com");
        }
        SharedString::new(format!("https://{resource}.openai.azure.com"))
    }

    fn create_language_model(&self, deployment: &AzureDeployment) -> Arc<dyn LanguageModel> {
        Arc::new(AzureLanguageModel {
            id: LanguageModelId::from(deployment.model.clone()),
            name: LanguageModelName::from(
                deployment
                    .name
                    .clone()
                    .unwrap_or_else(|| deployment.id.clone()),
            ),
            provider_id: PROVIDER_ID,
            provider_name: PROVIDER_NAME,
            deployment_id: deployment.id.clone().into(),
            model: deployment.model.clone().into(),
            max_tokens: deployment.max_tokens,
            supports_tools: deployment.supports_tools,
            supports_images: deployment.supports_images,
            state: self.state.clone(),
            http_client: self.http_client.clone(),
            request_limiter: RateLimiter::new(4),
        })
    }

    fn deployments(cx: &App) -> Vec<AzureDeployment> {
        Self::settings(cx).deployments
    }
}

impl LanguageModelProviderState for AzureLanguageModelProvider {
    type ObservableEntity = State;

    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        Some(self.state.clone())
    }
}

impl LanguageModelProvider for AzureLanguageModelProvider {
    fn id(&self) -> LanguageModelProviderId {
        PROVIDER_ID
    }

    fn name(&self) -> LanguageModelProviderName {
        PROVIDER_NAME
    }

    fn icon(&self) -> IconOrSvg {
        IconOrSvg::default()
    }

    fn default_model(&self, cx: &App) -> Option<Arc<dyn LanguageModel>> {
        let deployments = Self::deployments(cx);
        deployments.first().map(|d| self.create_language_model(d))
    }

    fn default_fast_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        None
    }

    fn provided_models(&self, cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        Self::deployments(cx)
            .iter()
            .map(|d| self.create_language_model(d))
            .collect()
    }

    fn is_authenticated(&self, cx: &App) -> bool {
        self.state.read(cx).is_authenticated()
    }

    fn authenticate(&self, cx: &mut App) -> Task<Result<(), AuthenticateError>> {
        self.state.update(cx, |state, cx| state.authenticate(cx))
    }

    fn configuration_view(
        &self,
        _target_agent: language_model::ConfigurationViewTargetAgent,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        cx.new(|_| AzureConfigurationView).into()
    }

    fn reset_credentials(&self, cx: &mut App) -> Task<Result<()>> {
        self.state
            .update(cx, |state, cx| state.set_api_key(None, cx))
    }
}

// ── Language Model ─────────────────────────────────────────────────────────

struct AzureLanguageModel {
    id: LanguageModelId,
    name: LanguageModelName,
    provider_id: LanguageModelProviderId,
    provider_name: LanguageModelProviderName,
    deployment_id: SharedString,
    model: SharedString,
    max_tokens: Option<u64>,
    supports_tools: bool,
    supports_images: bool,
    state: Entity<State>,
    http_client: Arc<dyn HttpClient>,
    request_limiter: RateLimiter,
}

impl LanguageModel for AzureLanguageModel {
    fn id(&self) -> LanguageModelId {
        self.id.clone()
    }

    fn name(&self) -> LanguageModelName {
        self.name.clone()
    }

    fn provider_id(&self) -> LanguageModelProviderId {
        self.provider_id.clone()
    }

    fn provider_name(&self) -> LanguageModelProviderName {
        self.provider_name.clone()
    }

    fn telemetry_id(&self) -> String {
        format!("azure-{}", self.deployment_id)
    }

    fn supports_tools(&self) -> bool {
        self.supports_tools
    }

    fn supports_images(&self) -> bool {
        self.supports_images
    }

    fn supports_tool_choice(&self, _choice: language_model::LanguageModelToolChoice) -> bool {
        self.supports_tools
    }

    fn max_token_count(&self) -> u64 {
        self.max_tokens.unwrap_or(128_000)
    }

    fn stream_completion(
        &self,
        request: LanguageModelRequest,
        cx: &AsyncApp,
    ) -> BoxFuture<
        'static,
        Result<
            BoxStream<'static, Result<LanguageModelCompletionEvent, LanguageModelCompletionError>>,
            LanguageModelCompletionError,
        >,
    > {
        let http_client = self.http_client.clone();
        let deployment_id = self.deployment_id.clone();
        let api_key_state = self.state.clone();
        let model = self.model.clone();
        let request_limiter = self.request_limiter.clone();
        let supports_tools = self.supports_tools;
        let max_output_tokens = self.max_output_tokens();

        let (api_key, api_url) = api_key_state.read_with(cx, |state, cx| {
            let api_url = AzureLanguageModelProvider::api_url(cx);
            (state.api_key_state.key(&api_url), api_url)
        });

        let future = request_limiter.stream(async move {
            let Some(api_key) = api_key else {
                return Err(LanguageModelCompletionError::NoApiKey {
                    provider: PROVIDER_NAME,
                });
            };

            let url = format!(
                "{api_url}/openai/deployments/{deployment_id}/chat/completions?api-version={}",
                DEFAULT_API_VERSION,
            );

            let open_ai_request = into_open_ai(
                request,
                &model,
                supports_tools,
                true,
                max_output_tokens,
                None,
                false,
            );

            let body = serde_json::to_string(&open_ai_request).map_err(|e| {
                LanguageModelCompletionError::SerializeRequest {
                    provider: PROVIDER_NAME,
                    error: e,
                }
            })?;

            let http_request = HttpRequest::builder()
                .method(Method::POST)
                .uri(&url)
                .header("Content-Type", "application/json")
                .header("api-key", &*api_key)
                .body(AsyncBody::from(body))
                .map_err(|e| LanguageModelCompletionError::BuildRequestBody {
                    provider: PROVIDER_NAME,
                    error: e,
                })?;

            let response = http_client.send(http_request).await.map_err(|e| {
                LanguageModelCompletionError::HttpSend {
                    provider: PROVIDER_NAME,
                    error: e,
                }
            })?;

            let status = response.status();
            if !status.is_success() {
                let mut body = String::new();
                let _ = futures::io::BufReader::new(response.into_body())
                    .read_to_string(&mut body)
                    .await;
                return Err(LanguageModelCompletionError::from_http_status(
                    PROVIDER_NAME,
                    status,
                    body,
                    None,
                ));
            }

            let reader = BufReader::new(response.into_body());
            let stream = reader
                .lines()
                .filter_map(move |line| async move {
                    let line = match line {
                        Ok(line) => line,
                        Err(_) => return None,
                    };
                    let line = line
                        .strip_prefix("data: ")
                        .or_else(|| line.strip_prefix("data:"))?;
                    if line == "[DONE]" {
                        return None;
                    }
                    let event: Result<ResponseStreamEvent> =
                        serde_json::from_str(line).map_err(|e| anyhow::anyhow!("{e}"));
                    Some(event)
                })
                .boxed();

            let mapper = OpenAiEventMapper::new();
            let mapped = mapper.map_stream(Box::pin(stream));
            Ok(mapped.boxed())
        });

        async move { Ok(future.await?.boxed()) }.boxed()
    }
}

// ── Configuration View ─────────────────────────────────────────────────────

struct AzureConfigurationView;

impl gpui::Render for AzureConfigurationView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_deserialization() {
        let json = serde_json::json!({
            "id": "my-gpt4",
            "model": "gpt-4o",
            "name": "GPT-4o (Azure)",
            "max_tokens": 128000,
            "supports_tools": true,
            "supports_images": true
        });
        let deployment: AzureDeployment = serde_json::from_value(json).unwrap();
        assert_eq!(deployment.id, "my-gpt4");
        assert_eq!(deployment.model, "gpt-4o");
        assert_eq!(deployment.name.unwrap(), "GPT-4o (Azure)");
        assert_eq!(deployment.max_tokens, Some(128000));
        assert!(deployment.supports_tools);
        assert!(deployment.supports_images);
    }

    #[test]
    fn test_deployment_defaults() {
        let json = serde_json::json!({
            "id": "my-deployment",
            "model": "gpt-35-turbo"
        });
        let deployment: AzureDeployment = serde_json::from_value(json).unwrap();
        assert_eq!(deployment.id, "my-deployment");
        assert_eq!(deployment.model, "gpt-35-turbo");
        assert!(deployment.supports_tools); // default_true
        assert!(!deployment.supports_images); // default false
        assert!(deployment.name.is_none());
        assert!(deployment.max_tokens.is_none());
    }
}
