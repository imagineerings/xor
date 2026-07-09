use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use gpui::{App, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use world_model::{
    BackendOptions, MeshArtifactMetadata, MeshBackend, MeshFormat, MeshGenerationRequest,
    TextureOptions,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimMeshGenerationToolInput {
    pub prompt: String,
    #[serde(default)]
    pub reference_image: Option<String>,
    #[serde(default)]
    pub target_format: SimMeshFormatInput,
    #[serde(default)]
    pub backend: SimMeshBackendInput,
    #[serde(default)]
    pub textures: SimTextureOptionsInput,
    #[serde(default)]
    pub backend_model: Option<String>,
    #[serde(default)]
    pub backend_params: HashMap<String, String>,
    #[serde(default)]
    pub output_mesh_path: Option<String>,
    #[serde(default)]
    pub preview_path: Option<String>,
    #[serde(default)]
    pub provenance_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimMeshFormatInput {
    #[default]
    Obj,
    Glb,
    Gltf,
    Fbx,
    Ply,
    Usd,
    Abc,
    Stl,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimMeshBackendInput {
    Python,
    RemoteApi,
    Native,
    #[default]
    Automatic,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SimTextureOptionsInput {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub resolution: Option<u32>,
    #[serde(default)]
    pub bake_ao: bool,
    #[serde(default)]
    pub normal_map: bool,
    #[serde(default)]
    pub pbr_maps: bool,
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimMeshGenerationToolOutput {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<MeshGenerationRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_metadata: Option<MeshArtifactMetadata>,
    pub diagnostics: Vec<String>,
    pub message: String,
}

impl From<SimMeshGenerationToolOutput> for LanguageModelToolResultContent {
    fn from(output: SimMeshGenerationToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| {
                format!("failed to serialize Sim mesh generation tool output: {error}")
            })
            .into()
    }
}

pub struct SimMeshGenerationTool;

impl AgentTool for SimMeshGenerationTool {
    type Input = SimMeshGenerationToolInput;
    type Output = SimMeshGenerationToolOutput;

    const NAME: &'static str = "sim_mesh_generation";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Other
    }

    fn initial_title(
        &self,
        _input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        "Preparing Sim mesh generation".into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |_cx| match input.recv().await {
            Ok(input) => {
                let output = run_sim_mesh_generation_tool(input);
                event_stream.update_fields(
                    acp::ToolCallUpdateFields::new().content(vec![output.message.clone().into()]),
                );
                if output.success {
                    Ok(output)
                } else {
                    Err(output)
                }
            }
            Err(error) => Err(SimMeshGenerationToolOutput {
                success: false,
                request: None,
                artifact_metadata: None,
                diagnostics: vec![format!("Failed to read Sim mesh generation input: {error}")],
                message: "Failed to prepare Sim mesh generation request".to_string(),
            }),
        })
    }
}

pub fn run_sim_mesh_generation_tool(
    input: SimMeshGenerationToolInput,
) -> SimMeshGenerationToolOutput {
    let request = build_mesh_request(&input);
    let mut diagnostics = Vec::new();
    if request.prompt.trim().is_empty() {
        diagnostics.push("mesh generation prompt cannot be empty".to_string());
    }
    if request.backend.requires_dependency_review() {
        diagnostics.push("native mesh generation backend requires dependency review".to_string());
    }
    if request.target_format.requires_dependency_review() {
        diagnostics.push(format!(
            "{} mesh export requires dependency review",
            request.target_format.extension()
        ));
    }
    if !diagnostics.is_empty() {
        return SimMeshGenerationToolOutput {
            success: false,
            request: Some(request),
            artifact_metadata: None,
            diagnostics,
            message: "Sim mesh generation request failed validation".to_string(),
        };
    }

    let artifact_metadata = input.output_mesh_path.map(|mesh_path| {
        let provenance_id = input.provenance_id.unwrap_or_else(|| {
            format!(
                "sim-agent:mesh:{}",
                mesh_path.replace(|character: char| !character.is_ascii_alphanumeric(), "-")
            )
        });
        let mut metadata = MeshArtifactMetadata::new(mesh_path, request.target_format)
            .with_provenance(provenance_id);
        if let Some(preview_path) = input.preview_path {
            metadata = metadata.with_preview(preview_path);
        }
        metadata
    });

    SimMeshGenerationToolOutput {
        success: true,
        request: Some(request),
        artifact_metadata,
        diagnostics,
        message: "Prepared typed Sim mesh generation request".to_string(),
    }
}

fn build_mesh_request(input: &SimMeshGenerationToolInput) -> MeshGenerationRequest {
    let mut request = MeshGenerationRequest::new(input.prompt.clone())
        .with_target_format(MeshFormat::from(input.target_format))
        .with_backend(MeshBackend::from(input.backend))
        .with_textures(TextureOptions::from(input.textures.clone()))
        .with_backend_options(BackendOptions {
            model: input.backend_model.clone(),
            params: input.backend_params.clone(),
        });
    if let Some(reference_image) = input.reference_image.clone() {
        request = request.with_reference_image(reference_image);
    }
    request
}

impl From<SimMeshFormatInput> for MeshFormat {
    fn from(input: SimMeshFormatInput) -> Self {
        match input {
            SimMeshFormatInput::Obj => Self::Obj,
            SimMeshFormatInput::Glb => Self::Glb,
            SimMeshFormatInput::Gltf => Self::Gltf,
            SimMeshFormatInput::Fbx => Self::Fbx,
            SimMeshFormatInput::Ply => Self::Ply,
            SimMeshFormatInput::Usd => Self::Usd,
            SimMeshFormatInput::Abc => Self::Abc,
            SimMeshFormatInput::Stl => Self::Stl,
        }
    }
}

impl From<SimMeshBackendInput> for MeshBackend {
    fn from(input: SimMeshBackendInput) -> Self {
        match input {
            SimMeshBackendInput::Python => Self::Python,
            SimMeshBackendInput::RemoteApi => Self::RemoteApi,
            SimMeshBackendInput::Native => Self::Native,
            SimMeshBackendInput::Automatic => Self::Automatic,
        }
    }
}

impl From<SimTextureOptionsInput> for TextureOptions {
    fn from(input: SimTextureOptionsInput) -> Self {
        Self {
            enabled: input.enabled,
            resolution: input.resolution,
            bake_ao: input.bake_ao,
            normal_map: input.normal_map,
            pbr_maps: input.pbr_maps,
            extra: input.extra,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_mesh_generation_tool_builds_typed_request_and_artifact_metadata() {
        let output = run_sim_mesh_generation_tool(SimMeshGenerationToolInput {
            prompt: "a carved stone arch".to_string(),
            reference_image: Some("inputs/arch.png".to_string()),
            target_format: SimMeshFormatInput::Glb,
            backend: SimMeshBackendInput::Automatic,
            textures: SimTextureOptionsInput {
                enabled: true,
                resolution: Some(2048),
                bake_ao: true,
                normal_map: true,
                pbr_maps: true,
                extra: HashMap::new(),
            },
            backend_model: Some("sim-mesh".to_string()),
            backend_params: HashMap::new(),
            output_mesh_path: Some("outputs/arch.glb".to_string()),
            preview_path: Some("previews/arch.png".to_string()),
            provenance_id: Some("prov-arch".to_string()),
        });

        assert!(output.success);
        let request = output.request.expect("request");
        assert_eq!(request.target_format, MeshFormat::Glb);
        assert!(request.textures.enabled);
        let metadata = output.artifact_metadata.expect("metadata");
        assert_eq!(metadata.provenance_id.as_deref(), Some("prov-arch"));
        assert_eq!(metadata.preview_path.as_deref(), Some("previews/arch.png"));
    }

    #[test]
    fn sim_mesh_generation_tool_rejects_unreviewed_native_dependency() {
        let output = run_sim_mesh_generation_tool(SimMeshGenerationToolInput {
            prompt: "a statue".to_string(),
            reference_image: None,
            target_format: SimMeshFormatInput::Obj,
            backend: SimMeshBackendInput::Native,
            textures: SimTextureOptionsInput::default(),
            backend_model: None,
            backend_params: HashMap::new(),
            output_mesh_path: None,
            preview_path: None,
            provenance_id: None,
        });

        assert!(!output.success);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("dependency review"))
        );
    }
}
