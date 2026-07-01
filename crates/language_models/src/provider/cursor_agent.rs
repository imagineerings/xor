use credentials_provider::CredentialsProvider;
use gpui::App;
use http_client::HttpClient;
use language_model::{LanguageModelProviderId, LanguageModelProviderName};
use std::sync::Arc;

use super::acp_subprocess::{AcpAuthMethod, AcpSubprocessProvider};

// ── Cursor Agent ───────────────────────────────────────────────────────

pub fn provider(
    http_client: Arc<dyn HttpClient>,
    credentials_provider: Arc<dyn CredentialsProvider>,
    cx: &mut App,
) -> AcpSubprocessProvider {
    AcpSubprocessProvider::new(
        LanguageModelProviderId::new("cursor_agent"),
        LanguageModelProviderName::new("Cursor Agent"),
        "cursor",
        AcpAuthMethod::None,
        http_client,
        credentials_provider,
        cx,
    )
}
