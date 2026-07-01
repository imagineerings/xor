use std::sync::Arc;

use anyhow::Result;
use credentials_provider::CredentialsProvider;
use futures::FutureExt;
use gpui::{AnyView, App, AppContext, Context, Entity, Render, SharedString, Task, Window};
use http_client::HttpClient;
use language_model::{
    AuthenticateError, IconOrSvg, LanguageModel, LanguageModelCompletionError,
    LanguageModelCompletionEvent, LanguageModelId, LanguageModelName, LanguageModelProvider,
    LanguageModelProviderId, LanguageModelProviderName, LanguageModelProviderState,
    LanguageModelRequest, RateLimiter,
};
use settings::SettingsStore;

// ── Auth methods for ACP-based providers ────────────────────────────────────

/// How an ACP-based provider authenticates with its remote service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcpAuthMethod {
    /// Read the API key from the given environment variable.
    ApiKey { env_var: SharedString },
    /// OAuth 2.0 with a client ID and scopes.
    OAuth {
        client_id: SharedString,
        scopes: Vec<SharedString>,
    },
    /// OAuth 2.0 device-flow (code + poll).
    DeviceFlow { client_id: SharedString },
    /// No authentication needed (local binary, no remote service).
    None,
}

// ── ACP subprocess provider ────────────────────────────────────────────────

/// A [`LanguageModelProvider`] that spawns a binary subprocess and
/// communicates via the Agent-Client Protocol (ACP) over stdio.
///
/// This is used for providers like Claude Code, ChatGPT/Codex, and Gemini
/// CLI that expose an ACP interface when run as a subprocess.
///
/// The provider discovers the binary via `$PATH`, spawns it on demand, and
/// wraps the ACP session as a [`LanguageModel`] implementation.
pub struct AcpSubprocessProvider {
    id: LanguageModelProviderId,
    name: LanguageModelProviderName,
    binary_name: SharedString,
    auth_method: AcpAuthMethod,
    state: Entity<State>,
}

#[allow(dead_code)]
pub struct State {
    _credentials_provider: Arc<dyn CredentialsProvider>,
}

#[allow(dead_code)]
impl State {
    fn is_authenticated(&self) -> bool {
        true
    }

    fn authenticate(&mut self, _cx: &mut Context<Self>) -> Task<Result<(), AuthenticateError>> {
        Task::ready(Ok(()))
    }
}

impl AcpSubprocessProvider {
    pub fn new(
        id: LanguageModelProviderId,
        name: LanguageModelProviderName,
        binary_name: impl Into<SharedString>,
        auth_method: AcpAuthMethod,
        _http_client: Arc<dyn HttpClient>,
        credentials_provider: Arc<dyn CredentialsProvider>,
        cx: &mut App,
    ) -> Self {
        let state = cx.new(|cx| {
            cx.observe_global::<SettingsStore>(|_this: &mut State, _cx| {})
                .detach();
            State {
                _credentials_provider: credentials_provider,
            }
        });

        Self {
            id,
            name,
            binary_name: binary_name.into(),
            auth_method,
            state,
        }
    }

    /// Check whether the binary is available on `$PATH`.
    fn resolve_binary(&self) -> Option<String> {
        let name = self.binary_name.as_ref();
        std::env::var_os("PATH").and_then(|path| {
            std::env::split_paths(&path).find_map(|dir| {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if candidate.metadata().ok()?.permissions().mode() & 0o111 != 0 {
                            return Some(candidate.to_string_lossy().to_string());
                        }
                        return None;
                    }
                    #[cfg(not(unix))]
                    {
                        Some(candidate.to_string_lossy().to_string())
                    }
                } else {
                    None
                }
            })
        })
    }

    fn create_language_model(&self) -> Option<Arc<dyn LanguageModel>> {
        let binary_path = self.resolve_binary()?;
        Some(Arc::new(AcpSubprocessLanguageModel {
            id: LanguageModelId::from(self.id.to_string()),
            name: LanguageModelName::from(self.name.to_string()),
            provider_id: self.id.clone(),
            provider_name: self.name.clone(),
            binary_path: binary_path.into(),
            auth_method: self.auth_method.clone(),
            request_limiter: RateLimiter::new(4),
        }))
    }
}

impl LanguageModelProviderState for AcpSubprocessProvider {
    type ObservableEntity = State;

    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        Some(self.state.clone())
    }
}

impl LanguageModelProvider for AcpSubprocessProvider {
    fn id(&self) -> LanguageModelProviderId {
        self.id.clone()
    }

    fn name(&self) -> LanguageModelProviderName {
        self.name.clone()
    }

    fn icon(&self) -> IconOrSvg {
        IconOrSvg::default()
    }

    fn default_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        self.create_language_model()
    }

    fn default_fast_model(&self, _cx: &App) -> Option<Arc<dyn LanguageModel>> {
        None
    }

    fn provided_models(&self, _cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        self.create_language_model().into_iter().collect()
    }

    fn is_authenticated(&self, _cx: &App) -> bool {
        self.resolve_binary().is_some()
    }

    fn authenticate(&self, _cx: &mut App) -> Task<Result<(), AuthenticateError>> {
        let binary_found = self.resolve_binary().is_some();
        Task::ready(if binary_found {
            Ok(())
        } else {
            Err(AuthenticateError::CredentialsNotFound)
        })
    }

    fn configuration_view(
        &self,
        _target_agent: language_model::ConfigurationViewTargetAgent,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        // ACP subprocess providers don't have a configuration view yet.
        // The binary must be installed and available on $PATH.
        cx.new(|_| AcpSubprocessConfigurationView).into()
    }

    fn reset_credentials(&self, _cx: &mut App) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    fn fast_mode_confirmation(&self, _cx: &App) -> Option<language_model::FastModeConfirmation> {
        None
    }
}

// ── ACP Subprocess Language Model ──────────────────────────────────────────

/// A [`LanguageModel`] backed by an ACP subprocess.
///
/// When `stream_completion` is called, this spawns the binary subprocess,
/// sends an ACP completion request over stdio, and translates the ACP
/// response stream into [`LanguageModelCompletionEvent`]s.
#[allow(dead_code)]
pub struct AcpSubprocessLanguageModel {
    id: LanguageModelId,
    name: LanguageModelName,
    provider_id: LanguageModelProviderId,
    provider_name: LanguageModelProviderName,
    binary_path: SharedString,
    auth_method: AcpAuthMethod,
    request_limiter: RateLimiter,
}

impl LanguageModel for AcpSubprocessLanguageModel {
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
        format!("acp-subprocess-{}", self.provider_id)
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_images(&self) -> bool {
        false
    }

    fn supports_tool_choice(&self, _choice: language_model::LanguageModelToolChoice) -> bool {
        true
    }

    fn supports_streaming_tools(&self) -> bool {
        true
    }

    fn max_token_count(&self) -> u64 {
        128_000
    }

    fn max_output_tokens(&self) -> Option<u64> {
        Some(16_384)
    }

    fn stream_completion(
        &self,
        _request: LanguageModelRequest,
        _cx: &gpui::AsyncApp,
    ) -> futures::future::BoxFuture<
        'static,
        Result<
            futures::stream::BoxStream<
                'static,
                Result<LanguageModelCompletionEvent, LanguageModelCompletionError>,
            >,
            LanguageModelCompletionError,
        >,
    > {
        // ACP subprocess streaming is not yet implemented at the
        // LanguageModel level.  The provider can be registered and
        // discovered but model creation / streaming will return an
        // "unavailable" error until the ACP-over-stdio transport is
        // wired through.
        let err = LanguageModelCompletionError::Other(anyhow::anyhow!(
            "ACP subprocess model stream_completion not yet implemented"
        ));
        async move { Err(err) }.boxed()
    }
}

/// Placeholder configuration view for ACP subprocess providers.
///
/// TODO: Show binary status, auth method, and a connect/disconnect button.
struct AcpSubprocessConfigurationView;

impl Render for AcpSubprocessConfigurationView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div()
    }
}
