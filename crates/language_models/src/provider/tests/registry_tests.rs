//! Tests for the provider registry.
//!
//! Covers registration, structural invariants of built-in providers (unique IDs,
//! non-empty names), and that the registry can be created and populated.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use credentials_provider::CredentialsProvider;
use gpui::{App, AppContext, AsyncApp, TestAppContext};
use http_client::FakeHttpClient;
use language_model::LanguageModelRegistry;
use parking_lot::Mutex;
use settings::SettingsStore;

use crate::provider::registry::ProviderRegistry;
use gpui_tokio;

// ── FakeCredentialsProvider ──────────────────────────────────────────────

/// A minimal in-memory credentials provider for use in tests.
struct FakeCredentialsProvider {
    storage: Mutex<Option<(String, Vec<u8>)>>,
}

impl FakeCredentialsProvider {
    fn new() -> Self {
        Self {
            storage: Mutex::new(None),
        }
    }
}

impl CredentialsProvider for FakeCredentialsProvider {
    fn read_credentials<'a>(
        &'a self,
        _url: &'a str,
        _cx: &'a AsyncApp,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Option<(String, Vec<u8>)>>> + 'a>> {
        Box::pin(async { Ok(self.storage.lock().clone()) })
    }

    fn write_credentials<'a>(
        &'a self,
        _url: &'a str,
        username: &'a str,
        password: &'a [u8],
        _cx: &'a AsyncApp,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>> {
        self.storage
            .lock()
            .replace((username.to_string(), password.to_vec()));
        Box::pin(async { Ok(()) })
    }

    fn delete_credentials<'a>(
        &'a self,
        _url: &'a str,
        _cx: &'a AsyncApp,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>> {
        *self.storage.lock() = None;
        Box::pin(async { Ok(()) })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Set up the minimal GPUI globals required for provider construction.
///
/// Built-in providers call `cx.observe_global::<SettingsStore>()` at
/// construction time and read settings via
/// `crate::AllLanguageModelSettings::get_global`.  The
/// [`SettingsStore::test`] constructor calls `load_settings_types()`, which
/// picks up every type annotated with `#[derive(RegisterSetting)]` through
/// the `inventory` crate — including `AllLanguageModelSettings`.
fn init_provider_context(cx: &mut App) {
    gpui_tokio::init(cx);
    let settings_store = SettingsStore::test(cx);
    cx.set_global(settings_store);
}

// ── Tests ────────────────────────────────────────────────────────────────

/// Smoke test: a [`ProviderRegistry`] can be constructed with mock
/// dependencies without panicking.
#[test]
fn test_provider_registry_can_be_created() {
    let http_client = FakeHttpClient::with_404_response();
    let credentials_provider = Arc::new(FakeCredentialsProvider::new());
    let _registry = ProviderRegistry::new(http_client, credentials_provider);
}

/// Registering all built-in providers should populate the
/// [`LanguageModelRegistry`] with the expected set of well-known providers.
#[gpui::test]
async fn test_register_all_populates_builtin_providers(cx: &mut TestAppContext) {
    let http_client = FakeHttpClient::with_404_response();
    let credentials_provider = Arc::new(FakeCredentialsProvider::new());

    cx.update(|app| {
        init_provider_context(app);

        let registry_entity = app.new(|_| LanguageModelRegistry::default());
        let provider_registry = ProviderRegistry::new(http_client, credentials_provider);

        registry_entity.update(app, |registry, cx| {
            provider_registry.register_all(registry, cx);
        });

        let providers = registry_entity.read(app).providers();
        let ids: Vec<String> = providers.iter().map(|p| p.id().0.to_string()).collect();
        assert!(
            !providers.is_empty(),
            "Expected at least one built-in provider"
        );

        assert!(
            ids.contains(&"anthropic".to_string()),
            "Expected 'anthropic' provider, got: {ids:?}"
        );
        assert!(
            ids.contains(&"openai".to_string()),
            "Expected 'openai' provider, got: {ids:?}"
        );
        assert!(
            ids.contains(&"ollama".to_string()),
            "Expected 'ollama' provider, got: {ids:?}"
        );
    });
}

/// Every built-in provider must have a unique [`LanguageModelProviderId`].
///
/// Duplicate IDs would cause silent overwrites in the registry's
/// `BTreeMap<LanguageModelProviderId, _>`.
#[gpui::test]
async fn test_builtin_provider_ids_are_unique(cx: &mut TestAppContext) {
    let http_client = FakeHttpClient::with_404_response();
    let credentials_provider = Arc::new(FakeCredentialsProvider::new());

    cx.update(|app| {
        init_provider_context(app);

        let registry_entity = app.new(|_| LanguageModelRegistry::default());
        let provider_registry = ProviderRegistry::new(http_client, credentials_provider);

        registry_entity.update(app, |registry, cx| {
            provider_registry.register_all(registry, cx);
        });

        let providers = registry_entity.read(app).providers();
        let mut ids: Vec<String> = providers.iter().map(|p| p.id().0.to_string()).collect();
        ids.sort();
        let deduped = {
            let mut copy = ids.clone();
            copy.dedup();
            copy
        };

        assert_eq!(
            ids.len(),
            deduped.len(),
            "Duplicate provider IDs found: {ids:?}"
        );
    });
}

/// Every built-in provider must have a non-empty display name.
#[gpui::test]
async fn test_builtin_provider_names_are_non_empty(cx: &mut TestAppContext) {
    let http_client = FakeHttpClient::with_404_response();
    let credentials_provider = Arc::new(FakeCredentialsProvider::new());

    cx.update(|app| {
        init_provider_context(app);

        let registry_entity = app.new(|_| LanguageModelRegistry::default());
        let provider_registry = ProviderRegistry::new(http_client, credentials_provider);

        registry_entity.update(app, |registry, cx| {
            provider_registry.register_all(registry, cx);
        });

        let providers = registry_entity.read(app).providers();
        for provider in &providers {
            let name = provider.name();
            assert!(
                !name.0.is_empty(),
                "Provider '{}' has an empty display name",
                provider.id(),
            );
        }
    });
}

/// Some built-in providers have models available without API calls (e.g. via
/// bundled model lists). Providers that depend on remote API calls or user
/// configuration (like Anthropic, Ollama) will have no models in a test
/// environment. This test checks that at least some providers register models.
#[gpui::test]
async fn test_builtin_providers_have_models(cx: &mut TestAppContext) {
    let http_client = FakeHttpClient::with_404_response();
    let credentials_provider = Arc::new(FakeCredentialsProvider::new());

    cx.update(|app| {
        init_provider_context(app);

        let registry_entity = app.new(|_| LanguageModelRegistry::default());
        let provider_registry = ProviderRegistry::new(http_client, credentials_provider);

        registry_entity.update(app, |registry, cx| {
            provider_registry.register_all(registry, cx);
        });

        let providers = registry_entity.read(app).providers();
        let total_models: usize = providers
            .iter()
            .map(|p| p.provided_models(app).len())
            .sum();
        assert!(
            total_models > 0,
            "Expected at least some built-in providers to expose models, but all {} providers returned empty model lists",
            providers.len(),
        );
    });
}
