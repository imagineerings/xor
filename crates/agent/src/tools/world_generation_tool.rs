use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use world_model::{
    GenerationProvenance, WorldActionControl, WorldControl, WorldGenerationRequest,
    WorldModelProfile,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimWorldGenerationToolInput {
    pub prompt: String,
    pub model_profile_name: String,
    pub model_family: String,
    pub output_target: String,
    #[serde(default)]
    pub model_variant: Option<String>,
    #[serde(default)]
    pub source_image: Option<String>,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub controls: Vec<SimWorldControlInput>,
    #[serde(default = "default_world_generation_backend")]
    pub backend_name: String,
    #[serde(default = "default_world_generation_workflow")]
    pub workflow_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimWorldControlInput {
    pub frame_count: u64,
    #[serde(default)]
    pub actions: Vec<SimWorldActionControlInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimWorldActionControlInput {
    pub name: String,
    pub value: f32,
    pub frame: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimWorldGenerationToolOutput {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<WorldGenerationRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<GenerationProvenance>,
    pub diagnostics: Vec<String>,
    pub message: String,
}

impl From<SimWorldGenerationToolOutput> for LanguageModelToolResultContent {
    fn from(output: SimWorldGenerationToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| {
                format!("failed to serialize Sim world generation tool output: {error}")
            })
            .into()
    }
}

pub struct SimWorldGenerationTool;

impl AgentTool for SimWorldGenerationTool {
    type Input = SimWorldGenerationToolInput;
    type Output = SimWorldGenerationToolOutput;

    const NAME: &'static str = "sim_world_generation";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Other
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Preparing Sim world generation".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| match input.recv().await {
            Ok(input) => {
                let output = run_sim_world_generation_tool(input);
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().content(vec![output.message.clone().into()]),
                );
                if output.success {
                    Ok(output)
                } else {
                    Err(output)
                }
            }
            Err(error) => Err(SimWorldGenerationToolOutput {
                success: false,
                request: None,
                provenance: None,
                diagnostics: vec![format!(
                    "Failed to read Sim world generation input: {error}"
                )],
                message: "Failed to prepare Sim world generation request".to_string(),
            }),
        })
    }
}

pub fn run_sim_world_generation_tool(
    input: SimWorldGenerationToolInput,
) -> SimWorldGenerationToolOutput {
    let profile = match input.model_variant {
        Some(variant) => WorldModelProfile::new(input.model_profile_name, input.model_family)
            .with_variant(variant),
        None => WorldModelProfile::new(input.model_profile_name, input.model_family),
    };
    let mut request = WorldGenerationRequest::new(input.prompt, profile, input.output_target);
    if let Some(source_image) = input.source_image {
        request = request.with_source_image(source_image);
    }
    if let Some(seed) = input.seed {
        request = request.with_seed(seed);
    }
    request = request.with_controls(input.controls.into_iter().map(WorldControl::from).collect());
    let diagnostics = request.validate();
    if !diagnostics.is_empty() {
        return SimWorldGenerationToolOutput {
            success: false,
            request: Some(request),
            provenance: None,
            diagnostics,
            message: "Sim world generation request failed validation".to_string(),
        };
    }

    let provenance = GenerationProvenance::new(request.clone())
        .with_backend(input.backend_name)
        .with_workflow(input.workflow_name);
    SimWorldGenerationToolOutput {
        success: true,
        request: Some(request),
        provenance: Some(provenance),
        diagnostics,
        message: "Prepared typed Sim world generation request".to_string(),
    }
}

impl From<SimWorldControlInput> for WorldControl {
    fn from(input: SimWorldControlInput) -> Self {
        Self::new(
            input
                .actions
                .into_iter()
                .map(WorldActionControl::from)
                .collect(),
            input.frame_count,
        )
    }
}

impl From<SimWorldActionControlInput> for WorldActionControl {
    fn from(input: SimWorldActionControlInput) -> Self {
        Self::new(input.name, input.value, input.frame)
    }
}

fn default_world_generation_backend() -> String {
    "native-sim-agent".to_string()
}

fn default_world_generation_workflow() -> String {
    "sim-world-generation".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_world_generation_tool_builds_typed_request_with_provenance() {
        let output = run_sim_world_generation_tool(SimWorldGenerationToolInput {
            prompt: "walk through a neon plaza".to_string(),
            model_profile_name: "sim-video".to_string(),
            model_family: "lingbot".to_string(),
            output_target: "outputs/plaza.mp4".to_string(),
            model_variant: Some("fp16".to_string()),
            source_image: Some("inputs/plaza.png".to_string()),
            seed: Some(42),
            controls: vec![SimWorldControlInput {
                frame_count: 60,
                actions: vec![SimWorldActionControlInput {
                    name: "w".to_string(),
                    value: 1.0,
                    frame: 0,
                }],
            }],
            backend_name: "native-sim-worker".to_string(),
            workflow_name: "authoring-preview".to_string(),
        });

        assert!(output.success);
        let request = output.request.expect("request");
        assert_eq!(request.seed, Some(42));
        assert_eq!(request.model_profile.variant.as_deref(), Some("fp16"));
        let provenance = output.provenance.expect("provenance");
        assert_eq!(
            provenance.backend_name.as_deref(),
            Some("native-sim-worker")
        );
    }

    #[test]
    fn sim_world_generation_tool_rejects_invalid_request() {
        let output = run_sim_world_generation_tool(SimWorldGenerationToolInput {
            prompt: String::new(),
            model_profile_name: String::new(),
            model_family: String::new(),
            output_target: String::new(),
            model_variant: None,
            source_image: None,
            seed: None,
            controls: Vec::new(),
            backend_name: "native-sim-worker".to_string(),
            workflow_name: "authoring-preview".to_string(),
        });

        assert!(!output.success);
        assert!(output.provenance.is_none());
        assert!(!output.diagnostics.is_empty());
    }
}
