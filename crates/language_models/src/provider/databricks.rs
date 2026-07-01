use anyhow::Result;
use collections::BTreeMap;
use credentials_provider::CredentialsProvider;
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::BoxStream};
use gpui::{AnyView, App, AppContext, AsyncApp, Context, Entity, SharedString, Task, Window};
use http_client::{CustomHeaders, HttpClient};
use language_model::{
    ApiKeyState, AuthenticateError, EnvVar, IconOrSvg, LanguageModel, LanguageModelCompletionError,
    LanguageModelCompletionEvent, LanguageModelEffortLevel, LanguageModelId, LanguageModelName,
    LanguageModelProvider, LanguageModelProviderId, LanguageModelProviderName,
    LanguageModelProviderState, LanguageModelRequest, LanguageModelToolChoice,
    LanguageModelToolSchemaFormat, RateLimiter, env_var,
};
use open_ai::{ResponseStreamEvent, stream_completion};
use settings::{Settings, SettingsStore};
use std::sync::{Arc, LazyLock};

const PROVIDER_ID: LanguageModelProviderId = LanguageModelProviderId::new("databricks_v1");
const PROVIDER_NAME: LanguageModelProviderName = LanguageModelProviderName::new("Databricks v1");

const API_KEY_ENV_VAR_NAME: &str = "DATABRICKS_API_KEY";
static API_KEY_ENV_VAR: LazyLock<EnvVar> = env_var!(API_KEY_ENV_VAR_NAME);

#[derive(Default, Clone, Debug, PartialEq)]
pub struct DatabricksV1Settings {
    pub api_url: String,
    pub custom_headers: CustomHeaders,
}

/// Databricks Legacy Serving Endpoint model.
#[derive(Clone, Debug, PartialEq)]
pub enum DatabricksV1Model {
    Default,
}

impl DatabricksV1Model {
    pub fn id(&self) -> &str {
        match self {
            Self::Default => "default",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Default => "Default",
        }
    }

    pub fn max_token_count(&self) -> u64 {
        match self {
            Self::Default => 128_000,
        }
    }

    pub fn supports_tools(&self) -> bool {
        match self {
            Self::Default => false,
        }
    }

    pub fn supports_images(&self) -> bool {
        match self {
            Self::Default => false,
        }
    }
}

impl Default for DatabricksV1Model {
    fn default() -> Self {
        Self::Default
    }
}

// ── State ──────────────────────────────────────────────────────────────────

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
        let api_url = DatabricksV1LanguageModelProvider::api_url(cx);
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
        let api_url = DatabricksV1LanguageModelProvider::api_url(cx);
        self.api_key_state.load_if_needed(
            api_url,
            |this| &mut this.api_key_state,
            credentials_provider,
            cx,
        )
    }
}

// ── Provider ───────────────────────────────────────────────────────────────

pub struct DatabricksV1LanguageModelProvider {
    http_client: Arc<dyn HttpClient>,
    state: Entity<State>,
}

impl DatabricksV1LanguageModelProvider {
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
                api_key_state: ApiKeyState::new(Self::api_url(cx), (*API_KEY_ENV_VAR).clone()),
                credentials_provider,
            }
        });

        Self { http_client, state }
    }

    fn create_language_model(&self, model: DatabricksV1Model) -> Arc<dyn LanguageModel> {
        Arc::new(DatabricksV1LanguageModel {
            id: LanguageModelId::from(model.id().to_string()),
            model,
            state: self.state.clone(),
            http_client: self.http_client.clone(),
            request_limiter: RateLimiter::new(4),
        })
    }

    fn settings(cx: &App) -> &DatabricksV1Settings {
        &crate::AllLanguageModelSettings::get_global(cx).databricks_v1
    }

    fn api_url(cx: &App) -> SharedString {
        let api_url = &Self::settings(cx).api_url;
        if api_url.is_empty() {
            SharedString::new("")
        } else {
            SharedString::new(api_url.as_str())
        }
    }
}

impl LanguageModelProviderState for DatabricksV1LanguageModelProvider {
    type ObservableEntity = State;

    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        Some(self.state.clone())
    }
}

impl LanguageModelProvider for DatabricksV1LanguageModelProvider {
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
        Some(self.create_language_model(DatabricksV1Model::default()))
    }

    fn default_fast_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        Some(self.create_language_model(DatabricksV1Model::default()))
    }

    fn provided_models(&self, _cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        let mut models = BTreeMap::<String, DatabricksV1Model>::default();

        models.insert(
            DatabricksV1Model::Default.id().to_string(),
            DatabricksV1Model::Default,
        );

        models
            .into_values()
            .map(|model| self.create_language_model(model))
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
        cx.new(|_| DatabricksV1ConfigurationView).into()
    }

    fn reset_credentials(&self, cx: &mut App) -> Task<Result<()>> {
        self.state
            .update(cx, |state, cx| state.set_api_key(None, cx))
    }
}

// ── Language Model ─────────────────────────────────────────────────────────

pub struct DatabricksV1LanguageModel {
    id: LanguageModelId,
    model: DatabricksV1Model,
    state: Entity<State>,
    http_client: Arc<dyn HttpClient>,
    request_limiter: RateLimiter,
}

impl DatabricksV1LanguageModel {
    fn stream_completion(
        &self,
        request: open_ai::Request,
        cx: &AsyncApp,
    ) -> BoxFuture<
        'static,
        Result<BoxStream<'static, Result<ResponseStreamEvent>>, LanguageModelCompletionError>,
    > {
        let http_client = self.http_client.clone();

        let (api_key, api_url, extra_headers) = self.state.read_with(cx, |state, cx| {
            let api_url = DatabricksV1LanguageModelProvider::api_url(cx);
            let extra_headers = DatabricksV1LanguageModelProvider::settings(cx)
                .custom_headers
                .clone();
            (state.api_key_state.key(&api_url), api_url, extra_headers)
        });

        let future = self.request_limiter.stream(async move {
            let provider = PROVIDER_NAME;
            let Some(api_key) = api_key else {
                return Err(LanguageModelCompletionError::NoApiKey { provider });
            };
            let request = stream_completion(
                http_client.as_ref(),
                provider.0.as_str(),
                &api_url,
                &api_key,
                request,
                &extra_headers,
            );
            let response = request.await?;
            Ok(response)
        });

        async move { Ok(future.await?.boxed()) }.boxed()
    }
}

impl LanguageModel for DatabricksV1LanguageModel {
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
        false
    }

    fn supports_images(&self) -> bool {
        false
    }

    fn supports_thinking(&self) -> bool {
        false
    }

    fn supported_effort_levels(&self) -> Vec<LanguageModelEffortLevel> {
        Vec::new()
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
        format!("databricks_v1/{}", self.model.id())
    }

    fn max_token_count(&self) -> u64 {
        self.model.max_token_count()
    }

    fn max_output_tokens(&self) -> Option<u64> {
        None
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
        let request = crate::provider::open_ai::into_open_ai(
            request,
            self.model.id(),
            self.model.supports_tools(),
            false, // supports_prompt_cache_key
            self.max_output_tokens(),
            None,  // reasoning_effort
            false, // interleaved_reasoning
        );
        let completions = self.stream_completion(request, cx);
        async move {
            let mapper = crate::provider::open_ai::OpenAiEventMapper::new();
            Ok(mapper.map_stream(completions.await?).boxed())
        }
        .boxed()
    }
}

// ── Configuration View (stub) ──────────────────────────────────────────────

struct DatabricksV1ConfigurationView;

impl gpui::Render for DatabricksV1ConfigurationView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div()
    }
}
