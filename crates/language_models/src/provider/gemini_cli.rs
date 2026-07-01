use credentials_provider::CredentialsProvider;
use gpui::{App, SharedString};
use http_client::HttpClient;
use language_model::{LanguageModelProviderId, LanguageModelProviderName};
use std::sync::Arc;

use super::acp_subprocess::{AcpAuthMethod, AcpSubprocessProvider};

// ── Gemini CLI ─────────────────────────────────────────────────────────

pub fn provider(
    http_client: Arc<dyn HttpClient>,
    credentials_provider: Arc<dyn CredentialsProvider>,
    cx: &mut App,
) -> AcpSubprocessProvider {
    AcpSubprocessProvider::new(
        LanguageModelProviderId::new("gemini_cli"),
        LanguageModelProviderName::new("Gemini CLI"),
        "google-gemini",
        AcpAuthMethod::ApiKey {
            env_var: SharedString::from("GEMINI_API_KEY"),
        },
        http_client,
        credentials_provider,
        cx,
    )
}
