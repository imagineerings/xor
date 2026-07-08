use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SIM_PROVIDER_DISABLED_CODE: &str = "world_model.provider_nodes.disabled";
pub const SIM_PROVIDER_UNSUPPORTED_CODE: &str = "world_model.provider_nodes.unsupported";
pub const SIM_PROVIDER_MISSING_CREDENTIAL_CODE: &str =
    "world_model.provider_nodes.missing_credential";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SimProviderId(String);

impl SimProviderId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimProviderCapability {
    TextToImage,
    ImageToImage,
    ImageEdit,
    Inpaint,
    Outpaint,
    BackgroundRemoval,
    Upscale,
    Relight,
    StyleTransfer,
    Vectorization,
    TextToVideo,
    ImageToVideo,
    VideoEdit,
    VideoExtend,
    LipSync,
    Avatar,
    VideoEnhancement,
    TextToAudio,
    SpeechToText,
    TextToSpeech,
    SpeechToSpeech,
    SoundEffects,
    Music,
    AudioIsolation,
    Llm,
    PromptEnhancement,
    TextToThreeD,
    ImageToThreeD,
    MultiviewToThreeD,
    Texture,
    Rig,
    Animate,
    Retarget,
    Convert,
    Topology,
    ModelImport,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderCostMetadata {
    pub may_incur_cost: bool,
    pub quota_key: Option<String>,
    pub rate_limit_hint: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimProviderNodeAvailability {
    Enabled,
    DisabledByPolicy { reason: String },
    Unsupported { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderNodeDefinition {
    pub provider_id: SimProviderId,
    pub provider_name: String,
    pub comfy_node_id: String,
    pub native_handler: String,
    pub capability: SimProviderCapability,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    pub required_credentials: Vec<String>,
    pub cost: SimProviderCostMetadata,
    pub availability: SimProviderNodeAvailability,
}

impl SimProviderNodeDefinition {
    pub fn new(
        provider_id: impl Into<String>,
        provider_name: impl Into<String>,
        comfy_node_id: impl Into<String>,
        capability: SimProviderCapability,
    ) -> Self {
        let provider_id = SimProviderId::new(provider_id);
        let comfy_node_id = comfy_node_id.into();
        Self {
            native_handler: format!("sim.provider.{}.{}", provider_id.as_str(), comfy_node_id),
            input_schema_ref: format!("#/provider_nodes/{comfy_node_id}/inputs"),
            output_schema_ref: format!("#/provider_nodes/{comfy_node_id}/outputs"),
            provider_id,
            provider_name: provider_name.into(),
            comfy_node_id,
            capability,
            required_credentials: Vec::new(),
            cost: SimProviderCostMetadata {
                may_incur_cost: false,
                quota_key: None,
                rate_limit_hint: None,
            },
            availability: SimProviderNodeAvailability::Enabled,
        }
    }

    pub fn with_credential(mut self, credential: impl Into<String>) -> Self {
        self.required_credentials.push(credential.into());
        self
    }

    pub fn with_cost(mut self, quota_key: impl Into<String>) -> Self {
        self.cost.may_incur_cost = true;
        self.cost.quota_key = Some(quota_key.into());
        self
    }

    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        self.availability = SimProviderNodeAvailability::DisabledByPolicy {
            reason: reason.into(),
        };
        self
    }

    pub fn unsupported(mut self, reason: impl Into<String>) -> Self {
        self.availability = SimProviderNodeAvailability::Unsupported {
            reason: reason.into(),
        };
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderNodeDiagnostic {
    pub code: String,
    pub provider_id: SimProviderId,
    pub comfy_node_id: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderNodeRegistry {
    api_nodes_enabled: bool,
    nodes: BTreeMap<String, SimProviderNodeDefinition>,
}

impl Default for SimProviderNodeRegistry {
    fn default() -> Self {
        Self::new(default_provider_nodes())
    }
}

impl SimProviderNodeRegistry {
    pub fn new(nodes: impl IntoIterator<Item = SimProviderNodeDefinition>) -> Self {
        Self {
            api_nodes_enabled: true,
            nodes: nodes
                .into_iter()
                .map(|node| (node.comfy_node_id.clone(), node))
                .collect(),
        }
    }

    pub fn with_api_nodes_enabled(mut self, enabled: bool) -> Self {
        self.api_nodes_enabled = enabled;
        self
    }

    pub fn node(&self, comfy_node_id: &str) -> Option<&SimProviderNodeDefinition> {
        self.nodes.get(comfy_node_id)
    }

    pub fn object_info_nodes(&self) -> Vec<&SimProviderNodeDefinition> {
        if !self.api_nodes_enabled {
            return Vec::new();
        }
        self.nodes
            .values()
            .filter(|node| node.availability == SimProviderNodeAvailability::Enabled)
            .collect()
    }

    pub fn diagnostics(&self) -> Vec<SimProviderNodeDiagnostic> {
        let mut diagnostics = Vec::new();
        for node in self.nodes.values() {
            if !self.api_nodes_enabled {
                diagnostics.push(SimProviderNodeDiagnostic {
                    code: SIM_PROVIDER_DISABLED_CODE.to_string(),
                    provider_id: node.provider_id.clone(),
                    comfy_node_id: node.comfy_node_id.clone(),
                    message: "API provider nodes are disabled by policy".to_string(),
                });
                continue;
            }
            match &node.availability {
                SimProviderNodeAvailability::Enabled => {
                    if node
                        .required_credentials
                        .iter()
                        .any(|credential| credential.is_empty())
                    {
                        diagnostics.push(SimProviderNodeDiagnostic {
                            code: SIM_PROVIDER_MISSING_CREDENTIAL_CODE.to_string(),
                            provider_id: node.provider_id.clone(),
                            comfy_node_id: node.comfy_node_id.clone(),
                            message: "provider node declares an empty credential key".to_string(),
                        });
                    }
                }
                SimProviderNodeAvailability::DisabledByPolicy { reason } => {
                    diagnostics.push(SimProviderNodeDiagnostic {
                        code: SIM_PROVIDER_DISABLED_CODE.to_string(),
                        provider_id: node.provider_id.clone(),
                        comfy_node_id: node.comfy_node_id.clone(),
                        message: reason.clone(),
                    });
                }
                SimProviderNodeAvailability::Unsupported { reason } => {
                    diagnostics.push(SimProviderNodeDiagnostic {
                        code: SIM_PROVIDER_UNSUPPORTED_CODE.to_string(),
                        provider_id: node.provider_id.clone(),
                        comfy_node_id: node.comfy_node_id.clone(),
                        message: reason.clone(),
                    });
                }
            }
        }
        diagnostics
    }
}

fn default_provider_nodes() -> Vec<SimProviderNodeDefinition> {
    vec![
        SimProviderNodeDefinition::new(
            "openai",
            "OpenAI",
            "OpenAIImageGenerate",
            SimProviderCapability::TextToImage,
        )
        .with_credential("openai.api_key")
        .with_cost("openai.images"),
        SimProviderNodeDefinition::new("openai", "OpenAI", "OpenAILLM", SimProviderCapability::Llm)
            .with_credential("openai.api_key")
            .with_cost("openai.responses"),
        SimProviderNodeDefinition::new(
            "gemini",
            "Gemini",
            "GeminiPromptEnhance",
            SimProviderCapability::PromptEnhancement,
        )
        .with_credential("gemini.api_key")
        .with_cost("gemini.text"),
        SimProviderNodeDefinition::new(
            "runway",
            "Runway",
            "RunwayTextToVideo",
            SimProviderCapability::TextToVideo,
        )
        .with_credential("runway.api_key")
        .with_cost("runway.video"),
        SimProviderNodeDefinition::new(
            "elevenlabs",
            "ElevenLabs",
            "ElevenLabsTextToSpeech",
            SimProviderCapability::TextToSpeech,
        )
        .with_credential("elevenlabs.api_key")
        .with_cost("elevenlabs.tts"),
        SimProviderNodeDefinition::new(
            "meshy",
            "Meshy",
            "MeshyTextTo3D",
            SimProviderCapability::TextToThreeD,
        )
        .with_credential("meshy.api_key")
        .with_cost("meshy.three_d"),
        SimProviderNodeDefinition::new(
            "sam3",
            "SAM3",
            "SAM3Segment",
            SimProviderCapability::ImageEdit,
        )
        .unsupported("SAM3 provider connector is not available in native Sim yet"),
    ]
}
