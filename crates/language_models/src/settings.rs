use std::sync::Arc;

use collections::HashMap;
use settings::RegisterSetting;

use crate::provider::{
    anthropic, anthropic::AnthropicSettings, avian::AvianSettings, azure::AzureSettings, bedrock,
    bedrock::AmazonBedrockSettings, cloud::SimDotDevSettings, databricks::DatabricksV1Settings,
    databricks_v2::DatabricksSettings, deepseek::DeepSeekSettings,
    gcp_vertex_ai::GcpVertexAiSettings, google::GoogleSettings, huggingface::HuggingFaceSettings,
    kimicode::KimiCodeSettings, litellm::LiteLlmSettings, lmstudio::LmStudioSettings, mistral,
    mistral::MistralSettings, nanogpt::NanoGptSettings, ollama::OllamaSettings,
    open_ai::OpenAiSettings, open_ai_compatible::OpenAiCompatibleSettings, open_router,
    open_router::OpenRouterSettings, opencode, opencode::OpenCodeSettings, resolve_custom_headers,
    sagemaker_tgi::SageMakerTgiSettings, snowflake::SnowflakeSettings, tetrate::TetrateSettings,
    vercel_ai_gateway::VercelAiGatewaySettings, x_ai::XAiSettings,
};

#[derive(Debug, RegisterSetting)]
pub struct AllLanguageModelSettings {
    pub anthropic: AnthropicSettings,
    pub bedrock: AmazonBedrockSettings,
    pub deepseek: DeepSeekSettings,
    pub google: GoogleSettings,
    pub lmstudio: LmStudioSettings,
    pub mistral: MistralSettings,
    pub ollama: OllamaSettings,
    pub opencode: OpenCodeSettings,
    pub open_router: OpenRouterSettings,
    pub openai: OpenAiSettings,
    pub openai_compatible: HashMap<Arc<str>, OpenAiCompatibleSettings>,
    pub vercel_ai_gateway: VercelAiGatewaySettings,
    pub x_ai: XAiSettings,
    pub azure: AzureSettings,
    pub sim_dot_dev: SimDotDevSettings,
    pub gcp_vertex_ai: GcpVertexAiSettings,
    pub huggingface: HuggingFaceSettings,
    pub litellm: LiteLlmSettings,
    pub nanogpt: NanoGptSettings,
    pub sagemaker_tgi: SageMakerTgiSettings,
    pub snowflake: SnowflakeSettings,
    pub databricks: DatabricksSettings,
    pub databricks_v1: DatabricksV1Settings,
    pub tetrate: TetrateSettings,
    pub avian: AvianSettings,
    pub kimicode: KimiCodeSettings,
}

fn custom_headers_from(
    provider_name: &str,
    raw: Option<HashMap<String, String>>,
    reserved: &[&str],
) -> http_client::CustomHeaders {
    raw.as_ref()
        .filter(|map| !map.is_empty())
        .map(|map| resolve_custom_headers(provider_name, map, reserved))
        .unwrap_or_default()
}

impl settings::Settings for AllLanguageModelSettings {
    const PRESERVED_KEYS: Option<&'static [&'static str]> = Some(&["version"]);

    fn from_settings(content: &settings::SettingsContent) -> Self {
        let language_models = content.language_models.clone().unwrap();
        let anthropic = language_models.anthropic.unwrap();
        let bedrock = language_models.bedrock.unwrap();
        let deepseek = language_models.deepseek.unwrap();
        let google = language_models.google.unwrap();
        let lmstudio = language_models.lmstudio.unwrap();
        let mistral = language_models.mistral.unwrap();
        let ollama = language_models.ollama.unwrap();
        let opencode = language_models.opencode.unwrap();
        let open_router = language_models.open_router.unwrap();
        let openai = language_models.openai.unwrap();
        let openai_compatible = language_models.openai_compatible.unwrap();
        let vercel_ai_gateway = language_models.vercel_ai_gateway.unwrap();
        let x_ai = language_models.x_ai.unwrap();
        let azure = language_models.azure.unwrap_or_default();
        let sim_dot_dev = language_models.sim_dot_dev.unwrap_or_default();
        let gcp_vertex_ai = language_models.gcp_vertex_ai.unwrap_or_default();
        let huggingface = language_models.huggingface.unwrap_or_default();
        let litellm = language_models.litellm.unwrap_or_default();
        let nanogpt = language_models.nanogpt.unwrap_or_default();
        let tetrate = language_models.tetrate.unwrap_or_default();
        let avian = language_models.avian.unwrap_or_default();
        let sagemaker_tgi = language_models.sagemaker_tgi.unwrap_or_default();
        let snowflake = language_models.snowflake.unwrap_or_default();
        let databricks = language_models.databricks.unwrap_or_default();
        let databricks_v1 = language_models.databricks_v1.unwrap_or_default();
        let kimicode = language_models.kimicode.unwrap_or_default();
        Self {
            azure: AzureSettings {
                resource_name: azure.resource_name.unwrap_or_default(),
                deployments: azure.deployments.unwrap_or_default(),
                api_version: azure.api_version,
                endpoint: azure.endpoint,
                use_ad_token: azure.use_ad_token.unwrap_or(false),
            },
            anthropic: AnthropicSettings {
                api_url: anthropic.api_url.unwrap(),
                available_models: anthropic.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from(
                    "Anthropic",
                    anthropic.custom_headers,
                    anthropic::RESERVED_HEADER_NAMES,
                ),
            },
            bedrock: AmazonBedrockSettings {
                available_models: bedrock.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from(
                    "Amazon Bedrock",
                    bedrock.custom_headers,
                    bedrock::RESERVED_HEADER_NAMES,
                ),
                region: bedrock.region,
                endpoint: bedrock.endpoint_url, // todo(should be api_url)
                profile_name: bedrock.profile,
                role_arn: None, // todo(was never a setting for this...)
                authentication_method: bedrock.authentication_method.map(Into::into),
                allow_global: bedrock.allow_global,
                guardrail_identifier: bedrock.guardrail_identifier,
                guardrail_version: bedrock.guardrail_version,
            },
            deepseek: DeepSeekSettings {
                api_url: deepseek.api_url.unwrap(),
                available_models: deepseek.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from("DeepSeek", deepseek.custom_headers, &[]),
            },
            google: GoogleSettings {
                api_url: google.api_url.unwrap(),
                available_models: google.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from("Google AI", google.custom_headers, &[]),
            },
            lmstudio: LmStudioSettings {
                api_url: lmstudio.api_url.unwrap(),
                available_models: lmstudio.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from("LM Studio", lmstudio.custom_headers, &[]),
            },
            mistral: MistralSettings {
                api_url: mistral.api_url.unwrap(),
                available_models: mistral.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from(
                    "Mistral",
                    mistral.custom_headers,
                    mistral::RESERVED_HEADER_NAMES,
                ),
            },
            ollama: OllamaSettings {
                api_url: ollama.api_url.unwrap(),
                auto_discover: ollama.auto_discover.unwrap_or(true),
                available_models: ollama.available_models.unwrap_or_default(),
                context_window: ollama.context_window,
                custom_headers: custom_headers_from("Ollama", ollama.custom_headers, &[]),
            },
            opencode: OpenCodeSettings {
                api_url: opencode.api_url.unwrap(),
                available_models: opencode.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from(
                    "OpenCode",
                    opencode.custom_headers,
                    opencode::RESERVED_HEADER_NAMES,
                ),
                show_zen_models: opencode.show_zen_models.unwrap_or(true),
                show_go_models: opencode.show_go_models.unwrap_or(true),
                show_free_models: opencode.show_free_models.unwrap_or(true),
            },
            open_router: OpenRouterSettings {
                api_url: open_router.api_url.unwrap(),
                available_models: open_router.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from(
                    "OpenRouter",
                    open_router.custom_headers,
                    open_router::RESERVED_HEADER_NAMES,
                ),
            },
            openai: OpenAiSettings {
                api_url: openai.api_url.unwrap(),
                available_models: openai.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from("OpenAI", openai.custom_headers, &[]),
            },
            openai_compatible: openai_compatible
                .into_iter()
                .map(|(key, value)| {
                    let provider_label = format!("OpenAI Compatible ({key})");
                    (
                        key,
                        OpenAiCompatibleSettings {
                            api_url: value.api_url,
                            available_models: value.available_models,
                            custom_headers: custom_headers_from(
                                &provider_label,
                                value.custom_headers,
                                &[],
                            ),
                        },
                    )
                })
                .collect(),
            vercel_ai_gateway: VercelAiGatewaySettings {
                api_url: vercel_ai_gateway.api_url.unwrap(),
                available_models: vercel_ai_gateway.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from(
                    "Vercel AI Gateway",
                    vercel_ai_gateway.custom_headers,
                    &[],
                ),
            },
            x_ai: XAiSettings {
                api_url: x_ai.api_url.unwrap(),
                available_models: x_ai.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from("xAI", x_ai.custom_headers, &[]),
            },
            sim_dot_dev: SimDotDevSettings {
                available_models: sim_dot_dev.available_models.unwrap_or_default(),
            },
            gcp_vertex_ai: GcpVertexAiSettings {
                api_url: gcp_vertex_ai.api_url.unwrap_or_default(),
                project_id: gcp_vertex_ai.project_id.unwrap_or_default(),
                region: gcp_vertex_ai.region.unwrap_or_default(),
                available_models: gcp_vertex_ai.available_models.unwrap_or_default(),
                custom_headers: custom_headers_from(
                    "GCP Vertex AI",
                    gcp_vertex_ai.custom_headers,
                    &[],
                ),
            },
            huggingface: HuggingFaceSettings {
                api_url: huggingface.api_url.unwrap_or_default(),
                custom_headers: custom_headers_from("HuggingFace", huggingface.custom_headers, &[]),
            },
            litellm: LiteLlmSettings {
                api_url: litellm.api_url.unwrap_or_default(),
                custom_headers: custom_headers_from("LiteLLM", litellm.custom_headers, &[]),
            },
            nanogpt: NanoGptSettings {
                api_url: nanogpt.api_url.unwrap_or_default(),
                custom_headers: custom_headers_from("NanoGPT", nanogpt.custom_headers, &[]),
            },
            tetrate: TetrateSettings {
                api_url: tetrate.api_url.unwrap_or_default(),
                custom_headers: custom_headers_from("Tetrate", tetrate.custom_headers, &[]),
            },
            avian: AvianSettings {
                api_url: avian.api_url.unwrap_or_default(),
                custom_headers: custom_headers_from("Avian", avian.custom_headers, &[]),
            },
            sagemaker_tgi: SageMakerTgiSettings {
                api_url: sagemaker_tgi.api_url.unwrap_or_default(),
                custom_headers: custom_headers_from(
                    "SageMaker TGI",
                    sagemaker_tgi.custom_headers,
                    &[],
                ),
            },
            snowflake: SnowflakeSettings {
                api_url: snowflake.api_url.unwrap_or_default(),
                custom_headers: custom_headers_from("Snowflake", snowflake.custom_headers, &[]),
            },
            databricks: DatabricksSettings {
                api_url: databricks.api_url.unwrap_or_default(),
                custom_headers: custom_headers_from("Databricks", databricks.custom_headers, &[]),
            },
            databricks_v1: DatabricksV1Settings {
                api_url: databricks_v1.api_url.unwrap_or_default(),
                custom_headers: custom_headers_from(
                    "Databricks v1",
                    databricks_v1.custom_headers,
                    &[],
                ),
            },
            kimicode: KimiCodeSettings {
                api_url: kimicode.api_url.unwrap_or_default(),
                custom_headers: custom_headers_from("KimiCode", kimicode.custom_headers, &[]),
            },
        }
    }
}
