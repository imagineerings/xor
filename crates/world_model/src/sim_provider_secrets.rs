use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{SimProviderId, SimProviderNodeDefinition};

pub const SIM_PROVIDER_SECRET_MISSING_CODE: &str = "world_model.provider_secrets.missing";
pub const SIM_PROVIDER_SECRET_EMPTY_KEY_CODE: &str = "world_model.provider_secrets.empty_key";
pub const SIM_PROVIDER_REDACTION_PLACEHOLDER: &str = "[REDACTED]";
pub const SIM_PROVIDER_SIGNED_URL_PLACEHOLDER: &str = "[REDACTED_SIGNED_URL]";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderSecretEntry {
    pub key: String,
    pub provider_id: SimProviderId,
    pub secret_ref: String,
}

impl SimProviderSecretEntry {
    pub fn new(
        key: impl Into<String>,
        provider_id: SimProviderId,
        secret_ref: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            provider_id,
            secret_ref: secret_ref.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderResolvedCredential {
    pub key: String,
    pub provider_id: SimProviderId,
    pub secret_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderSecretDiagnostic {
    pub code: String,
    pub provider_id: SimProviderId,
    pub comfy_node_id: String,
    pub credential_key: String,
    pub message: String,
}

impl SimProviderSecretDiagnostic {
    fn new(
        code: impl Into<String>,
        provider_id: SimProviderId,
        comfy_node_id: impl Into<String>,
        credential_key: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            provider_id,
            comfy_node_id: comfy_node_id.into(),
            credential_key: credential_key.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderCredentialReport {
    pub credentials: Vec<SimProviderResolvedCredential>,
    pub diagnostics: Vec<SimProviderSecretDiagnostic>,
}

impl SimProviderCredentialReport {
    pub fn is_complete(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderSecretStore {
    entries: BTreeMap<String, SimProviderSecretEntry>,
    #[serde(skip)]
    secret_values: BTreeMap<String, String>,
}

impl fmt::Debug for SimProviderSecretStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimProviderSecretStore")
            .field("entries", &self.entries)
            .field("secret_values", &"<redacted>")
            .finish()
    }
}

impl SimProviderSecretStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_secret(
        mut self,
        key: impl Into<String>,
        provider_id: SimProviderId,
        secret_ref: impl Into<String>,
        secret_value: impl Into<String>,
    ) -> Self {
        let key = key.into();
        self.entries.insert(
            key.clone(),
            SimProviderSecretEntry::new(key.clone(), provider_id, secret_ref),
        );
        self.secret_values.insert(key, secret_value.into());
        self
    }

    pub fn entry(&self, key: &str) -> Option<&SimProviderSecretEntry> {
        self.entries.get(key)
    }

    pub fn resolve_required_credentials(
        &self,
        node: &SimProviderNodeDefinition,
    ) -> SimProviderCredentialReport {
        let mut credentials = Vec::new();
        let mut diagnostics = Vec::new();

        for credential_key in &node.required_credentials {
            if credential_key.trim().is_empty() {
                diagnostics.push(SimProviderSecretDiagnostic::new(
                    SIM_PROVIDER_SECRET_EMPTY_KEY_CODE,
                    node.provider_id.clone(),
                    node.comfy_node_id.clone(),
                    credential_key.clone(),
                    "provider credential key must not be empty",
                ));
                continue;
            }

            match self.entry(credential_key) {
                Some(entry) if entry.provider_id == node.provider_id => {
                    credentials.push(SimProviderResolvedCredential {
                        key: entry.key.clone(),
                        provider_id: entry.provider_id.clone(),
                        secret_ref: entry.secret_ref.clone(),
                    });
                }
                Some(_) | None => diagnostics.push(SimProviderSecretDiagnostic::new(
                    SIM_PROVIDER_SECRET_MISSING_CODE,
                    node.provider_id.clone(),
                    node.comfy_node_id.clone(),
                    credential_key.clone(),
                    "provider credential must be configured in Sim secrets",
                )),
            }
        }

        SimProviderCredentialReport {
            credentials,
            diagnostics,
        }
    }

    fn secret_values(&self) -> impl Iterator<Item = &str> {
        self.secret_values.values().map(String::as_str)
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderRedactor {
    sensitive_keys: BTreeSet<String>,
    signed_url_markers: BTreeSet<String>,
    secret_values: BTreeSet<String>,
}

impl fmt::Debug for SimProviderRedactor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimProviderRedactor")
            .field("sensitive_keys", &self.sensitive_keys)
            .field("signed_url_markers", &self.signed_url_markers)
            .field("secret_values", &"<redacted>")
            .finish()
    }
}

impl Default for SimProviderRedactor {
    fn default() -> Self {
        Self {
            sensitive_keys: [
                "api_key",
                "apikey",
                "authorization",
                "credential",
                "password",
                "secret",
                "signed_url",
                "token",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            signed_url_markers: [
                "x-amz-signature",
                "x-goog-signature",
                "signature=",
                "sig=",
                "signed=",
                "token=",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            secret_values: BTreeSet::new(),
        }
    }
}

impl SimProviderRedactor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sensitive_key(mut self, key: impl Into<String>) -> Self {
        self.sensitive_keys.insert(key.into().to_ascii_lowercase());
        self
    }

    pub fn with_secret_value(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        if !value.is_empty() {
            self.secret_values.insert(value);
        }
        self
    }

    pub fn with_secret_store(mut self, store: &SimProviderSecretStore) -> Self {
        for secret_value in store.secret_values() {
            self = self.with_secret_value(secret_value);
        }
        self
    }

    pub fn redact_json(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let redacted = map
                    .iter()
                    .map(|(key, value)| {
                        let value = if self.is_sensitive_key(key) {
                            serde_json::Value::String(
                                SIM_PROVIDER_REDACTION_PLACEHOLDER.to_string(),
                            )
                        } else {
                            self.redact_json(value)
                        };
                        (key.clone(), value)
                    })
                    .collect();
                serde_json::Value::Object(redacted)
            }
            serde_json::Value::Array(values) => serde_json::Value::Array(
                values.iter().map(|value| self.redact_json(value)).collect(),
            ),
            serde_json::Value::String(value) => {
                serde_json::Value::String(self.redact_string(value))
            }
            _ => value.clone(),
        }
    }

    pub fn redact_string(&self, value: &str) -> String {
        if self.is_signed_url(value) {
            return SIM_PROVIDER_SIGNED_URL_PLACEHOLDER.to_string();
        }

        let mut redacted = value.to_string();
        for secret_value in &self.secret_values {
            if redacted.contains(secret_value) {
                redacted = redacted.replace(secret_value, SIM_PROVIDER_REDACTION_PLACEHOLDER);
            }
        }
        redacted
    }

    fn is_sensitive_key(&self, key: &str) -> bool {
        let key = key.to_ascii_lowercase();
        self.sensitive_keys
            .iter()
            .any(|sensitive_key| key.contains(sensitive_key))
    }

    fn is_signed_url(&self, value: &str) -> bool {
        let value = value.to_ascii_lowercase();
        value.starts_with("http")
            && value.contains('?')
            && self
                .signed_url_markers
                .iter()
                .any(|marker| value.contains(marker))
    }
}
