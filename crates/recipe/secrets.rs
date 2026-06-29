use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::Recipe;

pub trait SecretProvider {
    fn get_secret(&self, name: &str) -> Option<String>;
}

pub struct EnvironmentSecretProvider;

impl SecretProvider for EnvironmentSecretProvider {
    fn get_secret(&self, name: &str) -> Option<String> {
        std::env::var(name).ok().filter(|value| !value.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRequirement {
    pub key: String,
    pub env_var: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretStatus {
    pub requirement: SecretRequirement,
    pub configured: bool,
}

pub fn discover_required_secrets(recipe: &Recipe) -> Vec<SecretRequirement> {
    let mut requirements = BTreeMap::new();

    for parameter in &recipe.parameters {
        if looks_like_secret(&parameter.key) || looks_like_secret(&parameter.description) {
            requirements.insert(
                parameter.key.clone(),
                SecretRequirement {
                    key: parameter.key.clone(),
                    env_var: env_var_name(&parameter.key),
                    description: parameter.description.clone(),
                },
            );
        }
    }

    if let Some(value) = recipe.metadata.get("secrets") {
        for key in metadata_secret_keys(value) {
            requirements
                .entry(key.clone())
                .or_insert(SecretRequirement {
                    env_var: env_var_name(&key),
                    description: format!("Secret required by recipe metadata: {key}"),
                    key,
                });
        }
    }

    requirements.into_values().collect()
}

pub fn check_configured_secrets(
    recipe: &Recipe,
    provider: &impl SecretProvider,
) -> Vec<SecretStatus> {
    discover_required_secrets(recipe)
        .into_iter()
        .map(|requirement| {
            let configured = provider.get_secret(&requirement.env_var).is_some();
            SecretStatus {
                requirement,
                configured,
            }
        })
        .collect()
}

fn looks_like_secret(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "token",
        "secret",
        "password",
        "credential",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

fn env_var_name(key: &str) -> String {
    key.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn metadata_secret_keys(value: &serde_json::Value) -> Vec<String> {
    let mut keys = BTreeSet::new();

    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                if let Some(key) = value.as_str() {
                    keys.insert(key.to_string());
                }
            }
        }
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if value.as_bool().unwrap_or(true) {
                    keys.insert(key.clone());
                }
            }
        }
        serde_json::Value::String(key) => {
            keys.insert(key.clone());
        }
        _ => {}
    }

    keys.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::Recipe;

    struct MapSecretProvider(HashMap<String, String>);

    impl SecretProvider for MapSecretProvider {
        fn get_secret(&self, name: &str) -> Option<String> {
            self.0.get(name).cloned()
        }
    }

    #[test]
    fn discovers_secret_like_parameters() {
        let recipe = Recipe::from_yaml(
            r#"
title: Secret Recipe
description: Needs credentials
prompt: Run
parameters:
  - key: github_token
    input_type: string
    requirement: required
    description: GitHub token
"#,
        )
        .unwrap();

        let secrets = discover_required_secrets(&recipe);

        assert_eq!(secrets[0].key, "github_token");
        assert_eq!(secrets[0].env_var, "GITHUB_TOKEN");
    }

    #[test]
    fn discovers_metadata_secrets() {
        let recipe = Recipe::from_yaml(
            r#"
title: Secret Recipe
description: Needs credentials
prompt: Run
metadata:
  secrets:
    - openai_api_key
"#,
        )
        .unwrap();

        let secrets = discover_required_secrets(&recipe);

        assert_eq!(secrets[0].env_var, "OPENAI_API_KEY");
    }

    #[test]
    fn checks_configured_secrets() {
        let recipe = Recipe::from_yaml(
            r#"
title: Secret Recipe
description: Needs credentials
prompt: Run
parameters:
  - key: github_token
    input_type: string
    requirement: required
    description: GitHub token
"#,
        )
        .unwrap();

        let statuses = check_configured_secrets(
            &recipe,
            &MapSecretProvider(HashMap::from([(
                "GITHUB_TOKEN".to_string(),
                "configured".to_string(),
            )])),
        );

        assert!(statuses[0].configured);
    }
}
