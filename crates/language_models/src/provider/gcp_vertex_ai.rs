use anyhow::{Context as _, Result};
use collections::BTreeMap;
use credentials_provider::CredentialsProvider;
use futures::{
    AsyncBufReadExt, AsyncReadExt, FutureExt, StreamExt, future::BoxFuture, stream::BoxStream,
};
use google_ai::GenerateContentResponse;
pub use google_ai::completion::{GoogleEventMapper, into_google};
use gpui::{AnyView, App, AppContext, AsyncApp, Context, Entity, SharedString, Task, Window};
use http_client::{AsyncBody, HttpClient, Method, Request as HttpRequest};
use language_model::{ApiKeyState, EnvVar, env_var};
use language_model::{
    AuthenticateError, ConfigurationViewTargetAgent, IconOrSvg, LanguageModel,
    LanguageModelCompletionError, LanguageModelCompletionEvent, LanguageModelEffortLevel,
    LanguageModelId, LanguageModelName, LanguageModelProvider, LanguageModelProviderId,
    LanguageModelProviderName, LanguageModelProviderState, LanguageModelRequest,
    LanguageModelToolChoice, LanguageModelToolSchemaFormat, RateLimiter,
};
use settings::{Settings, SettingsStore};
use std::sync::{Arc, LazyLock};
use strum::IntoEnumIterator;

// ── Provider Identifiers ────────────────────────────────────────────────────

const PROVIDER_ID: LanguageModelProviderId = LanguageModelProviderId::new("gcp-vertex-ai");
const PROVIDER_NAME: LanguageModelProviderName = LanguageModelProviderName::new("GCP Vertex AI");

const ACCESS_TOKEN_ENV_VAR_NAME: &str = "GCP_VERTEX_AI_ACCESS_TOKEN";
static ACCESS_TOKEN_ENV_VAR: LazyLock<EnvVar> = env_var!(ACCESS_TOKEN_ENV_VAR_NAME);

/// Default GCP region for Vertex AI.
const DEFAULT_REGION: &str = "us-central1";

// ── Settings ───────────────────────────────────────────────────────────────

#[derive(Default, Clone, Debug, PartialEq)]
pub struct GcpVertexAiSettings {
    /// Base API URL. When empty, constructed from `project_id` and `region`.
    pub api_url: String,
    /// GCP project ID.
    pub project_id: String,
    /// GCP region (e.g. us-central1, europe-west4).
    pub region: String,
    /// Model definitions, overriding or extending the built-in list.
    pub available_models: Vec<AvailableModel>,
    /// Custom HTTP headers sent with every request.
    pub custom_headers: http_client::CustomHeaders,
}

pub use settings::GoogleAvailableModel as AvailableModel;

// ── State ───────────────────────────────────────────────────────────────────

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
        let api_url = GcpVertexAiLanguageModelProvider::api_url(cx);
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
        let api_url = GcpVertexAiLanguageModelProvider::api_url(cx);
        self.api_key_state.load_if_needed(
            api_url,
            |this| &mut this.api_key_state,
            credentials_provider,
            cx,
        )
    }
}

// ── Provider ────────────────────────────────────────────────────────────────

pub struct GcpVertexAiLanguageModelProvider {
    http_client: Arc<dyn HttpClient>,
    state: Entity<State>,
}

impl GcpVertexAiLanguageModelProvider {
    pub fn new(
        http_client: Arc<dyn HttpClient>,
        credentials_provider: Arc<dyn CredentialsProvider>,
        cx: &mut App,
    ) -> Self {
        let state = cx.new(|cx| {
            cx.observe_global::<SettingsStore>(|this: &mut State, cx| {
                let credentials_provider = this.credentials_provider.clone();
                let api_url = Self::api_url(cx);
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
                api_key_state: ApiKeyState::new(Self::api_url(cx), (*ACCESS_TOKEN_ENV_VAR).clone()),
                credentials_provider,
            }
        });

        Self { http_client, state }
    }

    fn settings(cx: &App) -> &GcpVertexAiSettings {
        &crate::AllLanguageModelSettings::get_global(cx).gcp_vertex_ai
    }

    fn api_url(cx: &App) -> SharedString {
        let settings = Self::settings(cx);
        if !settings.api_url.is_empty() {
            return SharedString::new(settings.api_url.clone());
        }
        let region = if settings.region.is_empty() {
            DEFAULT_REGION
        } else {
            &settings.region
        };
        let project = &settings.project_id;
        if project.is_empty() {
            return SharedString::from("https://us-central1-aiplatform.googleapis.com");
        }
        SharedString::new(format!(
            "https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}"
        ))
    }

    fn create_language_model(&self, model: google_ai::Model) -> Arc<dyn LanguageModel> {
        Arc::new(GcpVertexAiLanguageModel {
            id: LanguageModelId::from(model.id().to_string()),
            model,
            state: self.state.clone(),
            http_client: self.http_client.clone(),
            request_limiter: RateLimiter::new(4),
        })
    }
}

impl LanguageModelProviderState for GcpVertexAiLanguageModelProvider {
    type ObservableEntity = State;

    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        Some(self.state.clone())
    }
}

impl LanguageModelProvider for GcpVertexAiLanguageModelProvider {
    fn id(&self) -> LanguageModelProviderId {
        PROVIDER_ID
    }

    fn name(&self) -> LanguageModelProviderName {
        PROVIDER_NAME
    }

    fn icon(&self) -> IconOrSvg {
        IconOrSvg::default()
    }

    fn default_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        Some(self.create_language_model(google_ai::Model::default()))
    }

    fn default_fast_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        Some(self.create_language_model(google_ai::Model::default_fast()))
    }

    fn provided_models(&self, cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        let mut models = BTreeMap::<String, google_ai::Model>::default();

        for model in google_ai::Model::iter() {
            if !matches!(model, google_ai::Model::Custom { .. }) {
                models.insert(model.id().to_string(), model);
            }
        }

        for model in &Self::settings(cx).available_models {
            models.insert(
                model.name.clone(),
                google_ai::Model::Custom {
                    name: model.name.clone(),
                    display_name: model.display_name.clone(),
                    max_tokens: model.max_tokens,
                    mode: model.mode.unwrap_or_default(),
                },
            );
        }

        models
            .into_values()
            .map(|model| {
                Arc::new(GcpVertexAiLanguageModel {
                    id: LanguageModelId::from(model.id().to_string()),
                    model,
                    state: self.state.clone(),
                    http_client: self.http_client.clone(),
                    request_limiter: RateLimiter::new(4),
                }) as Arc<dyn LanguageModel>
            })
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
        _target_agent: ConfigurationViewTargetAgent,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        cx.new(|_| GcpVertexAiConfigurationView).into()
    }

    fn reset_credentials(&self, cx: &mut App) -> Task<Result<()>> {
        self.state
            .update(cx, |state, cx| state.set_api_key(None, cx))
    }
}

// ── Language Model ──────────────────────────────────────────────────────────

pub struct GcpVertexAiLanguageModel {
    id: LanguageModelId,
    model: google_ai::Model,
    state: Entity<State>,
    http_client: Arc<dyn HttpClient>,
    request_limiter: RateLimiter,
}

impl GcpVertexAiLanguageModel {
    fn stream_completion(
        &self,
        request: google_ai::GenerateContentRequest,
        cx: &AsyncApp,
    ) -> BoxFuture<'static, Result<BoxStream<'static, Result<GenerateContentResponse>>>> {
        let http_client = self.http_client.clone();

        let (access_token, api_url) = self.state.read_with(cx, |state, cx| {
            let api_url = GcpVertexAiLanguageModelProvider::api_url(cx);
            (state.api_key_state.key(&api_url), api_url)
        });

        async move {
            let access_token = access_token.context("Missing GCP Vertex AI access token")?;
            let request = stream_generate_content_vertex(
                http_client.as_ref(),
                &api_url,
                &access_token,
                request,
            );
            request.await.context("failed to stream completion")
        }
        .boxed()
    }
}

impl LanguageModel for GcpVertexAiLanguageModel {
    fn id(&self) -> LanguageModelId {
        self.id.clone()
    }

    fn name(&self) -> LanguageModelName {
        LanguageModelName::from(self.model.display_name().to_string())
    }

    fn provider_id(&self) -> LanguageModelProviderId {
        PROVIDER_ID
    }

    fn provider_name(&self) -> LanguageModelProviderName {
        PROVIDER_NAME
    }

    fn supports_tools(&self) -> bool {
        self.model.supports_tools()
    }

    fn supports_images(&self) -> bool {
        self.model.supports_images()
    }

    fn supports_thinking(&self) -> bool {
        self.model.supports_thinking()
    }

    fn supported_effort_levels(&self) -> Vec<LanguageModelEffortLevel> {
        let default_level = self.model.default_thinking_level();
        self.model
            .supported_thinking_levels()
            .iter()
            .map(|level| LanguageModelEffortLevel {
                name: level.name().into(),
                value: level.value().into(),
                is_default: Some(*level) == default_level,
            })
            .collect()
    }

    fn supports_tool_choice(&self, choice: LanguageModelToolChoice) -> bool {
        matches!(
            choice,
            LanguageModelToolChoice::Auto
                | LanguageModelToolChoice::Any
                | LanguageModelToolChoice::None
        )
    }

    fn tool_input_format(&self) -> LanguageModelToolSchemaFormat {
        LanguageModelToolSchemaFormat::JsonSchemaSubset
    }

    fn telemetry_id(&self) -> String {
        format!("gcp-vertex-ai/{}", self.model.request_id())
    }

    fn max_token_count(&self) -> u64 {
        self.model.max_token_count()
    }

    fn max_output_tokens(&self) -> Option<u64> {
        self.model.max_output_tokens()
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
        let request = into_google(
            request,
            self.model.request_id().to_string(),
            self.model.mode(),
        );
        let request = self.stream_completion(request, cx);
        let future = self.request_limiter.stream(async move {
            let response = request.await.map_err(LanguageModelCompletionError::from)?;
            Ok(GoogleEventMapper::new().map_stream(response))
        });
        async move { Ok(future.await?.boxed()) }.boxed()
    }
}

// ── HTTP Helper ─────────────────────────────────────────────────────────────

async fn stream_generate_content_vertex(
    client: &dyn HttpClient,
    api_url: &str,
    access_token: &str,
    request: google_ai::GenerateContentRequest,
) -> Result<BoxStream<'static, Result<google_ai::GenerateContentResponse>>> {
    let mut request = request;
    let model_id = std::mem::take(&mut request.model.model_id);

    let uri =
        format!("{api_url}/publishers/google/models/{model_id}:streamGenerateContent?alt=sse");

    let http_request = HttpRequest::builder()
        .method(Method::POST)
        .uri(&uri)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", access_token.trim()))
        .body(AsyncBody::from(serde_json::to_string(&request)?))?;

    let response = client.send(http_request).await?;
    let status = response.status();
    if status.is_success() {
        let reader = futures::io::BufReader::new(response.into_body());
        Ok(reader
            .lines()
            .filter_map(|line| async move {
                match line {
                    Ok(line) => {
                        if let Some(line) = line.strip_prefix("data: ") {
                            match serde_json::from_str(line) {
                                Ok(response) => Some(Ok(response)),
                                Err(error) => Some(Err(anyhow::anyhow!(
                                    "Error parsing JSON: {error:?}\n{line:?}"
                                ))),
                            }
                        } else {
                            None
                        }
                    }
                    Err(error) => Some(Err(anyhow::anyhow!(error))),
                }
            })
            .boxed())
    } else {
        let mut text = String::new();
        futures::io::BufReader::new(response.into_body())
            .read_to_string(&mut text)
            .await?;
        Err(anyhow::anyhow!(
            "error during streamGenerateContent, status code: {:?}, body: {}",
            status,
            text
        ))
    }
}

// ── Configuration View (stub) ───────────────────────────────────────────────

struct GcpVertexAiConfigurationView;

impl gpui::Render for GcpVertexAiConfigurationView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div()
    }
}
