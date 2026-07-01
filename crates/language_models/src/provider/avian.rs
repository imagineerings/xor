use anyhow::Result;
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

const PROVIDER_ID: LanguageModelProviderId = LanguageModelProviderId::new("avian");
const PROVIDER_NAME: LanguageModelProviderName = LanguageModelProviderName::new("Avian");
const AVIAN_API_URL: &str = "https://api.avian.io/v1";

const API_KEY_ENV_VAR_NAME: &str = "AVIAN_API_KEY";
static API_KEY_ENV_VAR: LazyLock<EnvVar> = env_var!(API_KEY_ENV_VAR_NAME);

#[derive(Default, Clone, Debug, PartialEq)]
pub struct AvianSettings {
    pub api_url: String,
    pub custom_headers: CustomHeaders,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AvianModel {
    Default,
}
impl AvianModel {
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
        128_000
    }
    pub fn supports_tools(&self) -> bool {
        true
    }
    pub fn supports_images(&self) -> bool {
        true
    }
}
impl Default for AvianModel {
    fn default() -> Self {
        Self::Default
    }
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
        let api_url = AvianLanguageModelProvider::api_url(cx);
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
        let api_url = AvianLanguageModelProvider::api_url(cx);
        self.api_key_state.load_if_needed(
            api_url,
            |this| &mut this.api_key_state,
            credentials_provider,
            cx,
        )
    }
}

pub struct AvianLanguageModelProvider {
    http_client: Arc<dyn HttpClient>,
    state: Entity<State>,
}
impl AvianLanguageModelProvider {
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
    fn create_language_model(&self, model: AvianModel) -> Arc<dyn LanguageModel> {
        Arc::new(AvianLanguageModel {
            id: LanguageModelId::from(model.id().to_string()),
            model,
            state: self.state.clone(),
            http_client: self.http_client.clone(),
            request_limiter: RateLimiter::new(4),
        })
    }
    fn settings(cx: &App) -> &AvianSettings {
        &crate::AllLanguageModelSettings::get_global(cx).avian
    }
    fn api_url(cx: &App) -> SharedString {
        let api_url = &Self::settings(cx).api_url;
        if api_url.is_empty() {
            AVIAN_API_URL.into()
        } else {
            SharedString::new(api_url.as_str())
        }
    }
}

impl LanguageModelProviderState for AvianLanguageModelProvider {
    type ObservableEntity = State;
    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        Some(self.state.clone())
    }
}

impl LanguageModelProvider for AvianLanguageModelProvider {
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
        Some(self.create_language_model(AvianModel::default()))
    }
    fn default_fast_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        Some(self.create_language_model(AvianModel::default()))
    }
    fn provided_models(&self, _cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        vec![self.create_language_model(AvianModel::Default)]
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
        cx.new(|_| AvianConfigurationView).into()
    }
    fn reset_credentials(&self, cx: &mut App) -> Task<Result<()>> {
        self.state
            .update(cx, |state, cx| state.set_api_key(None, cx))
    }
}

pub struct AvianLanguageModel {
    id: LanguageModelId,
    model: AvianModel,
    state: Entity<State>,
    http_client: Arc<dyn HttpClient>,
    request_limiter: RateLimiter,
}
impl AvianLanguageModel {
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
            let api_url = AvianLanguageModelProvider::api_url(cx);
            let extra_headers = AvianLanguageModelProvider::settings(cx)
                .custom_headers
                .clone();
            (state.api_key_state.key(&api_url), api_url, extra_headers)
        });
        let future = self.request_limiter.stream(async move {
            let provider = PROVIDER_NAME;
            let Some(api_key) = api_key else {
                return Err(LanguageModelCompletionError::NoApiKey { provider });
            };
            stream_completion(
                http_client.as_ref(),
                provider.0.as_str(),
                &api_url,
                &api_key,
                request,
                &extra_headers,
            )
            .await
            .map_err(LanguageModelCompletionError::from)
        });
        async move { Ok(future.await?.boxed()) }.boxed()
    }
}

impl LanguageModel for AvianLanguageModel {
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
        true
    }
    fn supports_images(&self) -> bool {
        true
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
        format!("avian/{}", self.model.id())
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
            true,
            false,
            self.max_output_tokens(),
            None,
            false,
        );
        let completions = self.stream_completion(request, cx);
        async move {
            let mapper = crate::provider::open_ai::OpenAiEventMapper::new();
            Ok(mapper.map_stream(completions.await?).boxed())
        }
        .boxed()
    }
}

struct AvianConfigurationView;
impl gpui::Render for AvianConfigurationView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div()
    }
}
