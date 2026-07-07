use std::sync::Arc;

use collections::HashMap;
use serde::Deserialize;

/// Configuration for a declarative (OpenAI-compatible) provider that users
/// define in their sim settings without writing any Rust code.
///
/// Each entry creates an [`OpenAiCompatibleLanguageModelProvider`] instance
/// at startup (or on settings reload).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct DeclarativeProviderConfig {
    /// The base URL for the OpenAI-compatible API (e.g. `https://api.example.com/v1`).
    pub api_url: String,
    /// Optional environment variable name that holds the API key.
    pub api_key_env_var: Option<String>,
    /// List of model identifiers served by this endpoint.
    pub models: Vec<String>,
    /// Custom HTTP headers to attach to every request.
    pub custom_headers: HashMap<String, String>,
}

impl DeclarativeProviderConfig {
    /// Validate the configuration, returning a list of human-readable errors.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.api_url.is_empty() {
            errors.push("api_url must not be empty".into());
        }

        // Basic URL sanity check
        if !self.api_url.is_empty()
            && !self.api_url.starts_with("http://")
            && !self.api_url.starts_with("https://")
        {
            errors.push(format!(
                "api_url must start with http:// or https://, got: {}",
                self.api_url
            ));
        }

        if self.models.is_empty() {
            errors.push("at least one model must be specified".into());
        }

        errors
    }

    /// Returns `true` when the config is valid (no validation errors).
    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }
}

/// A loaded and validated declarative provider instance ready to be registered.
pub struct DeclarativeProviderInstance {
    /// The provider identifier (e.g. `"my-custom-llm"`).
    pub id: Arc<str>,
    /// The validated configuration.
    pub config: DeclarativeProviderConfig,
}

/// Load declarative provider configs from a settings section.
///
/// The expected shape is a map of provider-name → [`DeclarativeProviderConfig`].
pub fn load_declarative_providers(
    raw: &HashMap<String, serde_json::Value>,
) -> Vec<DeclarativeProviderInstance> {
    let mut providers = Vec::new();

    for (name, value) in raw {
        match serde_json::from_value::<DeclarativeProviderConfig>(value.clone()) {
            Ok(config) => {
                let errors = config.validate();
                if errors.is_empty() {
                    providers.push(DeclarativeProviderInstance {
                        id: Arc::from(name.as_str()),
                        config,
                    });
                } else {
                    log::warn!(
                        "declarative provider `{name}` has invalid configuration: {}",
                        errors.join("; ")
                    );
                }
            }
            Err(err) => {
                log::warn!("declarative provider `{name}` failed to parse: {err}");
            }
        }
    }

    providers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let config = DeclarativeProviderConfig {
            api_url: "https://api.example.com/v1".into(),
            api_key_env_var: Some("MY_API_KEY".into()),
            models: vec!["model-a".into(), "model-b".into()],
            custom_headers: HashMap::default(),
        };
        assert!(config.is_valid());
        assert!(config.validate().is_empty());
    }

    #[test]
    fn test_empty_api_url() {
        let config = DeclarativeProviderConfig {
            api_url: "".into(),
            ..Default::default()
        };
        assert!(!config.is_valid());
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("api_url")));
    }

    #[test]
    fn test_invalid_url_scheme() {
        let config = DeclarativeProviderConfig {
            api_url: "ftp://api.example.com".into(),
            models: vec!["m".into()],
            ..Default::default()
        };
        assert!(!config.is_valid());
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("api_url")));
    }

    #[test]
    fn test_empty_models() {
        let config = DeclarativeProviderConfig {
            api_url: "https://api.example.com/v1".into(),
            ..Default::default()
        };
        assert!(!config.is_valid());
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("model")));
    }

    #[test]
    fn test_load_declarative_providers_skips_invalid() {
        use serde_json::json;

        let mut raw: HashMap<String, serde_json::Value> = HashMap::default();
        raw.insert(
            "valid-provider".into(),
            json!({
                "api_url": "https://api.valid.com/v1",
                "models": ["m1"]
            }),
        );
        raw.insert(
            "invalid-provider".into(),
            json!({
                "api_url": "",
            }),
        );

        let providers = load_declarative_providers(&raw);
        assert_eq!(providers.len(), 1);
        assert_eq!(&*providers[0].id, "valid-provider");
    }
}
