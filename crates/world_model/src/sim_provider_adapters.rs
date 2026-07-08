use serde::{Deserialize, Serialize};

use crate::{
    SIM_PROVIDER_CONNECTOR_CANCEL_UNSUPPORTED_CODE, SIM_PROVIDER_CONNECTOR_START_FAILED_CODE,
    SimProviderCapability, SimProviderConnector, SimProviderConnectorError, SimProviderId,
    SimProviderPolicyRequest, SimProviderRemoteTaskHandle, SimProviderRemoteTaskStatus,
};

pub const SIM_PROVIDER_ADAPTER_UNSUPPORTED_OPERATION_CODE: &str =
    "world_model.provider_adapter.unsupported_operation";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimProviderAdapterFamily {
    OpenAi,
    Gemini,
    AnthropicOpenRouter,
    ImageVideo,
    Audio,
    ThreeD,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderAdapterUnsupportedOperation {
    pub comfy_node_id: String,
    pub capability: SimProviderCapability,
    pub reason: String,
}

impl SimProviderAdapterUnsupportedOperation {
    pub fn new(
        comfy_node_id: impl Into<String>,
        capability: SimProviderCapability,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            comfy_node_id: comfy_node_id.into(),
            capability,
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderAdapterDefinition {
    pub provider_id: SimProviderId,
    pub family: SimProviderAdapterFamily,
    pub native_handler_prefix: String,
    pub comfy_node_ids: Vec<String>,
    pub capabilities: Vec<SimProviderCapability>,
    pub credential_keys: Vec<String>,
    pub unsupported_operations: Vec<SimProviderAdapterUnsupportedOperation>,
}

impl SimProviderAdapterDefinition {
    pub fn new(provider_id: impl Into<String>, family: SimProviderAdapterFamily) -> Self {
        let provider_id = SimProviderId::new(provider_id);
        Self {
            native_handler_prefix: format!("sim.provider.{}", provider_id.as_str()),
            provider_id,
            family,
            comfy_node_ids: Vec::new(),
            capabilities: Vec::new(),
            credential_keys: Vec::new(),
            unsupported_operations: Vec::new(),
        }
    }

    pub fn with_node(mut self, comfy_node_id: impl Into<String>) -> Self {
        self.comfy_node_ids.push(comfy_node_id.into());
        self
    }

    pub fn with_capability(mut self, capability: SimProviderCapability) -> Self {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
        self
    }

    pub fn with_credential_key(mut self, credential_key: impl Into<String>) -> Self {
        self.credential_keys.push(credential_key.into());
        self
    }

    pub fn with_unsupported_operation(
        mut self,
        operation: SimProviderAdapterUnsupportedOperation,
    ) -> Self {
        self.unsupported_operations.push(operation);
        self
    }

    pub fn native_handler_for(&self, comfy_node_id: &str) -> String {
        format!("{}.{}", self.native_handler_prefix, comfy_node_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderAdapterDiagnostic {
    pub code: String,
    pub provider_id: SimProviderId,
    pub comfy_node_id: String,
    pub capability: SimProviderCapability,
    pub message: String,
}

impl SimProviderAdapterDiagnostic {
    fn unsupported(
        provider_id: SimProviderId,
        operation: &SimProviderAdapterUnsupportedOperation,
    ) -> Self {
        Self {
            code: SIM_PROVIDER_ADAPTER_UNSUPPORTED_OPERATION_CODE.to_string(),
            provider_id,
            comfy_node_id: operation.comfy_node_id.clone(),
            capability: operation.capability,
            message: operation.reason.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderAdapterCatalog {
    definitions: Vec<SimProviderAdapterDefinition>,
}

impl Default for SimProviderAdapterCatalog {
    fn default() -> Self {
        Self::new(default_adapter_definitions())
    }
}

impl SimProviderAdapterCatalog {
    pub fn new(definitions: Vec<SimProviderAdapterDefinition>) -> Self {
        Self { definitions }
    }

    pub fn definitions(&self) -> &[SimProviderAdapterDefinition] {
        &self.definitions
    }

    pub fn definition(&self, provider_id: &SimProviderId) -> Option<&SimProviderAdapterDefinition> {
        self.definitions
            .iter()
            .find(|definition| &definition.provider_id == provider_id)
    }

    pub fn connector(&self, provider_id: &SimProviderId) -> Option<SimProviderAdapterSkeleton> {
        self.definition(provider_id)
            .cloned()
            .map(SimProviderAdapterSkeleton::new)
    }

    pub fn unsupported_diagnostics(&self) -> Vec<SimProviderAdapterDiagnostic> {
        self.definitions
            .iter()
            .flat_map(|definition| {
                definition
                    .unsupported_operations
                    .iter()
                    .map(|operation| {
                        SimProviderAdapterDiagnostic::unsupported(
                            definition.provider_id.clone(),
                            operation,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimProviderAdapterSkeleton {
    definition: SimProviderAdapterDefinition,
    next_task_index: u64,
}

impl SimProviderAdapterSkeleton {
    pub fn new(definition: SimProviderAdapterDefinition) -> Self {
        Self {
            definition,
            next_task_index: 0,
        }
    }

    pub fn definition(&self) -> &SimProviderAdapterDefinition {
        &self.definition
    }

    fn unsupported_operation(
        &self,
        request: &SimProviderPolicyRequest,
    ) -> Option<&SimProviderAdapterUnsupportedOperation> {
        self.definition
            .unsupported_operations
            .iter()
            .find(|operation| operation.comfy_node_id == request.comfy_node_id)
    }
}

impl SimProviderConnector for SimProviderAdapterSkeleton {
    fn provider_id(&self) -> &SimProviderId {
        &self.definition.provider_id
    }

    fn capabilities(&self) -> &[SimProviderCapability] {
        &self.definition.capabilities
    }

    fn start(
        &mut self,
        request: SimProviderPolicyRequest,
    ) -> Result<SimProviderRemoteTaskHandle, SimProviderConnectorError> {
        if request.provider_id != self.definition.provider_id {
            return Err(SimProviderConnectorError::new(
                SIM_PROVIDER_CONNECTOR_START_FAILED_CODE,
                self.definition.provider_id.clone(),
                "request provider does not match native Sim adapter skeleton",
            ));
        }

        if let Some(operation) = self.unsupported_operation(&request) {
            return Err(SimProviderConnectorError::new(
                SIM_PROVIDER_ADAPTER_UNSUPPORTED_OPERATION_CODE,
                self.definition.provider_id.clone(),
                operation.reason.clone(),
            ));
        }

        if !self
            .definition
            .comfy_node_ids
            .contains(&request.comfy_node_id)
        {
            return Err(SimProviderConnectorError::new(
                SIM_PROVIDER_CONNECTOR_START_FAILED_CODE,
                self.definition.provider_id.clone(),
                "native Sim adapter skeleton does not expose requested node id",
            ));
        }

        if !self.definition.capabilities.contains(&request.capability) {
            return Err(SimProviderConnectorError::new(
                SIM_PROVIDER_CONNECTOR_START_FAILED_CODE,
                self.definition.provider_id.clone(),
                "native Sim adapter skeleton does not support requested capability",
            ));
        }

        self.next_task_index += 1;
        Ok(SimProviderRemoteTaskHandle::new(
            request.provider_id,
            format!(
                "sim-provider-adapter-{}-{}",
                self.definition.provider_id.as_str(),
                self.next_task_index
            ),
            request.comfy_node_id,
            request.native_handler,
        ))
    }

    fn poll(
        &mut self,
        _handle: &SimProviderRemoteTaskHandle,
    ) -> Result<SimProviderRemoteTaskStatus, SimProviderConnectorError> {
        Ok(SimProviderRemoteTaskStatus::Running {
            progress: None,
            message: Some(
                "native Sim provider adapter skeleton is awaiting provider implementation"
                    .to_string(),
            ),
        })
    }

    fn cancel(
        &mut self,
        _handle: &SimProviderRemoteTaskHandle,
    ) -> Result<SimProviderRemoteTaskStatus, SimProviderConnectorError> {
        Err(SimProviderConnectorError::new(
            SIM_PROVIDER_CONNECTOR_CANCEL_UNSUPPORTED_CODE,
            self.definition.provider_id.clone(),
            "native Sim provider adapter skeleton has no remote task to cancel yet",
        ))
    }
}

fn default_adapter_definitions() -> Vec<SimProviderAdapterDefinition> {
    vec![
        SimProviderAdapterDefinition::new("openai", SimProviderAdapterFamily::OpenAi)
            .with_node("OpenAIImageGenerate")
            .with_node("OpenAILLM")
            .with_capability(SimProviderCapability::TextToImage)
            .with_capability(SimProviderCapability::Llm)
            .with_credential_key("openai.api_key")
            .with_unsupported_operation(SimProviderAdapterUnsupportedOperation::new(
                "OpenAITextToVideo",
                SimProviderCapability::TextToVideo,
                "OpenAI video generation is not implemented in the native Sim adapter yet",
            )),
        SimProviderAdapterDefinition::new("gemini", SimProviderAdapterFamily::Gemini)
            .with_node("GeminiPromptEnhance")
            .with_capability(SimProviderCapability::PromptEnhancement)
            .with_credential_key("gemini.api_key")
            .with_unsupported_operation(SimProviderAdapterUnsupportedOperation::new(
                "GeminiImageGenerate",
                SimProviderCapability::TextToImage,
                "Gemini image generation is not implemented in the native Sim adapter yet",
            )),
        SimProviderAdapterDefinition::new(
            "anthropic_openrouter",
            SimProviderAdapterFamily::AnthropicOpenRouter,
        )
        .with_node("AnthropicLLM")
        .with_node("OpenRouterLLM")
        .with_capability(SimProviderCapability::Llm)
        .with_credential_key("anthropic.api_key")
        .with_credential_key("openrouter.api_key")
        .with_unsupported_operation(SimProviderAdapterUnsupportedOperation::new(
            "AnthropicVision",
            SimProviderCapability::ImageEdit,
            "Anthropic/OpenRouter vision operations are not implemented in the native Sim adapter yet",
        )),
        SimProviderAdapterDefinition::new("runway", SimProviderAdapterFamily::ImageVideo)
            .with_node("RunwayTextToVideo")
            .with_capability(SimProviderCapability::TextToVideo)
            .with_credential_key("runway.api_key")
            .with_unsupported_operation(SimProviderAdapterUnsupportedOperation::new(
                "LumaImageToVideo",
                SimProviderCapability::ImageToVideo,
                "Luma image-to-video is not implemented in the native Sim adapter yet",
            ))
            .with_unsupported_operation(SimProviderAdapterUnsupportedOperation::new(
                "KlingVideoExtend",
                SimProviderCapability::VideoExtend,
                "Kling video extension is not implemented in the native Sim adapter yet",
            )),
        SimProviderAdapterDefinition::new("elevenlabs", SimProviderAdapterFamily::Audio)
            .with_node("ElevenLabsTextToSpeech")
            .with_capability(SimProviderCapability::TextToSpeech)
            .with_credential_key("elevenlabs.api_key")
            .with_unsupported_operation(SimProviderAdapterUnsupportedOperation::new(
                "ElevenLabsSpeechToSpeech",
                SimProviderCapability::SpeechToSpeech,
                "ElevenLabs speech-to-speech is not implemented in the native Sim adapter yet",
            )),
        SimProviderAdapterDefinition::new("meshy", SimProviderAdapterFamily::ThreeD)
            .with_node("MeshyTextTo3D")
            .with_capability(SimProviderCapability::TextToThreeD)
            .with_credential_key("meshy.api_key")
            .with_unsupported_operation(SimProviderAdapterUnsupportedOperation::new(
                "TripoImageTo3D",
                SimProviderCapability::ImageToThreeD,
                "Tripo image-to-3D is not implemented in the native Sim adapter yet",
            ))
            .with_unsupported_operation(SimProviderAdapterUnsupportedOperation::new(
                "RodinMultiviewTo3D",
                SimProviderCapability::MultiviewToThreeD,
                "Rodin multiview-to-3D is not implemented in the native Sim adapter yet",
            )),
    ]
}
