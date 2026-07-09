use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const SIM_PROVIDER_DISABLED_CODE: &str = "world_model.provider_nodes.disabled";
pub const SIM_PROVIDER_UNSUPPORTED_CODE: &str = "world_model.provider_nodes.unsupported";
pub const SIM_PROVIDER_MISSING_CREDENTIAL_CODE: &str =
    "world_model.provider_nodes.missing_credential";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderBacklogCatalog {
    pub schema_version: u32,
    pub source_root: String,
    pub source_category: String,
    pub captured_at: String,
    pub implementation_owner: String,
    pub native_sim_records: bool,
    pub comfyui_passthrough: bool,
    pub expected_record_count: usize,
    pub records: Vec<SimProviderBacklogRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderBacklogRecord {
    pub source_id: String,
    pub source_path: String,
    pub source_kind: String,
    pub node_name: String,
    pub provider_id: String,
    pub native_surface: String,
    pub evidence_module: String,
    pub evidence_kind: String,
    pub real_calls_policy_gated: bool,
    pub metadata_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderBacklogDiagnostic {
    pub code: String,
    pub message: String,
}

impl SimProviderBacklogCatalog {
    pub fn validate(&self) -> Result<(), Vec<SimProviderBacklogDiagnostic>> {
        let mut diagnostics = Vec::new();

        if self.schema_version != 1 {
            diagnostics.push(sim_provider_backlog_diagnostic(
                "world_model.provider_nodes.backlog.invalid_schema",
                "provider backlog fixture must use schema version 1",
            ));
        }
        if self.source_root != "projects/comfy/comfy_api_nodes" {
            diagnostics.push(sim_provider_backlog_diagnostic(
                "world_model.provider_nodes.backlog.invalid_source_root",
                "provider backlog fixture must preserve comfy_api_nodes source attribution",
            ));
        }
        if !self.native_sim_records || self.comfyui_passthrough {
            diagnostics.push(sim_provider_backlog_diagnostic(
                "world_model.provider_nodes.backlog.not_native",
                "provider backlog fixture must describe native Sim records only",
            ));
        }
        if self.records.len() != self.expected_record_count {
            diagnostics.push(sim_provider_backlog_diagnostic(
                "world_model.provider_nodes.backlog.count_mismatch",
                format!(
                    "expected {} provider backlog records but found {}",
                    self.expected_record_count,
                    self.records.len()
                ),
            ));
        }

        let mut source_ids = BTreeSet::new();
        for record in &self.records {
            if !source_ids.insert(&record.source_id) {
                diagnostics.push(sim_provider_backlog_diagnostic(
                    "world_model.provider_nodes.backlog.duplicate_record",
                    format!("duplicate provider source id `{}`", record.source_id),
                ));
            }
            if !record
                .source_path
                .starts_with("projects/comfy/comfy_api_nodes/")
            {
                diagnostics.push(sim_provider_backlog_diagnostic(
                    "world_model.provider_nodes.backlog.invalid_source_path",
                    format!(
                        "source path `{}` does not preserve comfy_api_nodes attribution",
                        record.source_path
                    ),
                ));
            }
            if record.node_name.is_empty()
                || record.provider_id.is_empty()
                || record.native_surface.is_empty()
                || record.evidence_module.is_empty()
                || record.evidence_kind.is_empty()
            {
                diagnostics.push(sim_provider_backlog_diagnostic(
                    "world_model.provider_nodes.backlog.missing_evidence",
                    format!("record `{}` is missing provider evidence", record.source_id),
                ));
            }
            if !record.metadata_only || !record.real_calls_policy_gated {
                diagnostics.push(sim_provider_backlog_diagnostic(
                    "world_model.provider_nodes.backlog.unsafe_record",
                    format!(
                        "record `{}` must stay metadata-only with real provider calls policy-gated",
                        record.source_id
                    ),
                ));
            }
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }

    pub fn provider_ids(&self) -> BTreeSet<String> {
        self.records
            .iter()
            .map(|record| record.provider_id.clone())
            .collect()
    }
}

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

fn sim_provider_backlog_diagnostic(
    code: impl Into<String>,
    message: impl Into<String>,
) -> SimProviderBacklogDiagnostic {
    SimProviderBacklogDiagnostic {
        code: code.into(),
        message: message.into(),
    }
}
