use serde_json::json;

use crate::{
    SIM_PROVIDER_REDACTION_PLACEHOLDER, SIM_PROVIDER_SECRET_MISSING_CODE,
    SIM_PROVIDER_SIGNED_URL_PLACEHOLDER, SimProviderCapability, SimProviderId,
    SimProviderNodeDefinition, SimProviderRedactor, SimProviderSecretStore,
};

#[test]
fn provider_secret_store_resolves_required_credentials_to_sim_secret_refs() {
    let node = SimProviderNodeDefinition::new(
        "openai",
        "OpenAI",
        "OpenAIImageGenerate",
        SimProviderCapability::TextToImage,
    )
    .with_credential("openai.api_key");
    let store = SimProviderSecretStore::new().with_secret(
        "openai.api_key",
        SimProviderId::new("openai"),
        "sim://secrets/provider/openai/api_key",
        "sk-test-secret",
    );

    let report = store.resolve_required_credentials(&node);

    assert!(report.is_complete());
    assert_eq!(report.credentials.len(), 1);
    assert_eq!(report.credentials[0].key, "openai.api_key");
    assert_eq!(
        report.credentials[0].secret_ref,
        "sim://secrets/provider/openai/api_key"
    );
}

#[test]
fn provider_secret_store_reports_missing_credentials_without_plaintext() {
    let node = SimProviderNodeDefinition::new(
        "runway",
        "Runway",
        "RunwayTextToVideo",
        SimProviderCapability::TextToVideo,
    )
    .with_credential("runway.api_key");

    let report = SimProviderSecretStore::new().resolve_required_credentials(&node);

    assert!(!report.is_complete());
    assert_eq!(report.credentials, Vec::new());
    assert_eq!(report.diagnostics[0].code, SIM_PROVIDER_SECRET_MISSING_CODE);
    assert_eq!(report.diagnostics[0].credential_key, "runway.api_key");
    assert!(!report.diagnostics[0].message.contains("runway-secret"));
}

#[test]
fn provider_redactor_removes_sensitive_fields_and_nested_secret_values() {
    let store = SimProviderSecretStore::new().with_secret(
        "openai.api_key",
        SimProviderId::new("openai"),
        "sim://secrets/provider/openai/api_key",
        "sk-test-secret",
    );
    let redactor = SimProviderRedactor::new().with_secret_store(&store);
    let payload = json!({
        "prompt": "draw a city",
        "api_key": "sk-test-secret",
        "headers": {
            "Authorization": "Bearer sk-test-secret",
            "safe": "value"
        },
        "events": [
            { "message": "request used sk-test-secret" }
        ]
    });

    let redacted = redactor.redact_json(&payload);

    assert_eq!(redacted["prompt"], "draw a city");
    assert_eq!(redacted["api_key"], SIM_PROVIDER_REDACTION_PLACEHOLDER);
    assert_eq!(
        redacted["headers"]["Authorization"],
        SIM_PROVIDER_REDACTION_PLACEHOLDER
    );
    assert_eq!(redacted["headers"]["safe"], "value");
    assert_eq!(
        redacted["events"][0]["message"],
        format!("request used {SIM_PROVIDER_REDACTION_PLACEHOLDER}")
    );
}

#[test]
fn provider_redactor_replaces_signed_urls() {
    let redactor = SimProviderRedactor::new();
    let payload = json!({
        "outputs": [
            {
                "download_url": "https://cdn.example.com/file.png?X-Amz-Signature=abc123&Expires=1"
            }
        ]
    });

    let redacted = redactor.redact_json(&payload);

    assert_eq!(
        redacted["outputs"][0]["download_url"],
        SIM_PROVIDER_SIGNED_URL_PLACEHOLDER
    );
}

#[test]
fn provider_redactor_keeps_plain_public_urls() {
    let redactor = SimProviderRedactor::new();

    assert_eq!(
        redactor.redact_string("https://example.com/public/image.png"),
        "https://example.com/public/image.png"
    );
}
