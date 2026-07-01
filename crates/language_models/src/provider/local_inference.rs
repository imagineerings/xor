use anyhow::Result;
use futures::{FutureExt, future::BoxFuture, stream::BoxStream};
use gpui::{AnyView, App, AppContext, AsyncApp, Context, Entity, Task, Window};
use language_model::{
    AuthenticateError, IconOrSvg, LanguageModel, LanguageModelCompletionError,
    LanguageModelCompletionEvent, LanguageModelEffortLevel, LanguageModelId, LanguageModelName,
    LanguageModelProvider, LanguageModelProviderId, LanguageModelProviderName,
    LanguageModelProviderState, LanguageModelRequest, LanguageModelToolChoice,
    LanguageModelToolSchemaFormat,
};
use std::sync::Arc;

const PROVIDER_ID: LanguageModelProviderId = LanguageModelProviderId::new("local_inference");
const PROVIDER_NAME: LanguageModelProviderName = LanguageModelProviderName::new("Local Inference");

// ── Models ──────────────────────────────────────────────────────────────────

/// Models available for local inference.
#[derive(Clone, Debug, PartialEq)]
pub enum LocalInferenceModel {
    LlamaCpp,
    Ollama,
    Custom { name: String },
}

impl LocalInferenceModel {
    pub fn id(&self) -> &str {
        match self {
            Self::LlamaCpp => "llama.cpp",
            Self::Ollama => "ollama",
            Self::Custom { name } => name.as_str(),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::LlamaCpp => "llama.cpp (local)",
            Self::Ollama => "Ollama (local)",
            Self::Custom { name } => name.as_str(),
        }
    }

    pub fn max_token_count(&self) -> u64 {
        128_000
    }

    pub fn supports_tools(&self) -> bool {
        true
    }

    pub fn supports_images(&self) -> bool {
        match self {
            Self::LlamaCpp => true,
            Self::Ollama => true,
            Self::Custom { .. } => true,
        }
    }
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct State;

// ── Provider ────────────────────────────────────────────────────────────────

pub struct LocalInferenceLanguageModelProvider;

impl LocalInferenceLanguageModelProvider {
    pub fn new(_cx: &mut App) -> Self {
        Self
    }

    fn create_language_model(&self, model: LocalInferenceModel) -> Arc<dyn LanguageModel> {
        Arc::new(LocalInferenceLanguageModel {
            id: LanguageModelId::from(model.id().to_string()),
            model,
        })
    }
}

impl LanguageModelProviderState for LocalInferenceLanguageModelProvider {
    type ObservableEntity = State;

    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        None
    }
}

impl LanguageModelProvider for LocalInferenceLanguageModelProvider {
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
        Some(self.create_language_model(LocalInferenceModel::Ollama))
    }

    fn default_fast_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        Some(self.create_language_model(LocalInferenceModel::LlamaCpp))
    }

    fn provided_models(&self, _cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        vec![
            self.create_language_model(LocalInferenceModel::LlamaCpp),
            self.create_language_model(LocalInferenceModel::Ollama),
        ]
    }

    fn is_authenticated(&self, _cx: &App) -> bool {
        true
    }

    fn authenticate(&self, _cx: &mut App) -> Task<Result<(), AuthenticateError>> {
        Task::ready(Ok(()))
    }

    fn configuration_view(
        &self,
        _target_agent: language_model::ConfigurationViewTargetAgent,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        cx.new(|_| LocalInferenceConfigurationView).into()
    }

    fn reset_credentials(&self, _cx: &mut App) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }
}

// ── Language Model ──────────────────────────────────────────────────────────

pub struct LocalInferenceLanguageModel {
    id: LanguageModelId,
    model: LocalInferenceModel,
}

impl LanguageModel for LocalInferenceLanguageModel {
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
        format!("local_inference/{}", self.model.id())
    }

    fn max_token_count(&self) -> u64 {
        self.model.max_token_count()
    }

    fn max_output_tokens(&self) -> Option<u64> {
        None
    }

    fn stream_completion(
        &self,
        _request: LanguageModelRequest,
        _cx: &AsyncApp,
    ) -> BoxFuture<
        'static,
        Result<
            BoxStream<'static, Result<LanguageModelCompletionEvent, LanguageModelCompletionError>>,
            LanguageModelCompletionError,
        >,
    > {
        // Local inference via candle requires additional dependencies not yet
        // wired into the language_models crate. Users should configure an Ollama
        // or llama.cpp endpoint via the OpenAI-compatible provider instead.
        let msg = "Local inference requires a running Ollama or llama.cpp server. \
                   Configure it as an OpenAI-compatible provider pointing at your \
                   local endpoint (e.g. http://localhost:11434/v1)."
            .to_string();

        async move { Err(LanguageModelCompletionError::Other(anyhow::anyhow!(msg))) }.boxed()
    }
}

// ── Configuration View ──────────────────────────────────────────────────────

struct LocalInferenceConfigurationView;

impl gpui::Render for LocalInferenceConfigurationView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div()
    }
}
