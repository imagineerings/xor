use serde::{Deserialize, Serialize};
use sim_game::SimGeneratedAssetRecord;
use world_model::{ArtifactType, GeneratedWorldArtifact, WorldGenerationRequest};

pub const SIM_AUTHORING_PREVIEW_WORKER_UNAVAILABLE_CODE: &str =
    "sim_apps.game_authoring.preview_worker_unavailable";
pub const SIM_AUTHORING_PREVIEW_MISSING_PROVENANCE_CODE: &str =
    "sim_apps.game_authoring.preview_missing_provenance";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimGameAuthoringApp {
    pub items: Vec<SimAuthoringItem>,
    pub generated_assets: Vec<SimGeneratedAssetRecord>,
}

impl SimGameAuthoringApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_item(&mut self, item: SimAuthoringItem) {
        self.items.push(item);
    }

    pub fn register_generated_asset(&mut self, asset: SimGeneratedAssetRecord) {
        self.items.push(SimAuthoringItem::generated_artifact(
            asset.asset_path.clone(),
            asset.provenance_id.clone(),
        ));
        self.generated_assets.push(asset);
    }

    pub fn route_item(&self, item: &SimAuthoringItem) -> SimAuthoringRoute {
        let route_kind = match item.kind {
            SimAuthoringItemKind::ProjectAsset => SimAuthoringRouteKind::Inspector,
            SimAuthoringItemKind::DiffusionGraph => SimAuthoringRouteKind::GraphEditor,
            SimAuthoringItemKind::WorldModelRequest => SimAuthoringRouteKind::Inspector,
            SimAuthoringItemKind::GeneratedArtifact => SimAuthoringRouteKind::Preview,
            SimAuthoringItemKind::RunExportTask => SimAuthoringRouteKind::TaskView,
        };
        SimAuthoringRoute {
            item_id: item.id.clone(),
            route_kind,
            diagnostics: Vec::new(),
        }
    }

    pub fn preview_generated_artifact(
        &self,
        artifact: &GeneratedWorldArtifact,
        worker_diagnostics_ready: bool,
    ) -> Result<SimAuthoringPreviewRoute, SimAuthoringDiagnostic> {
        if !worker_diagnostics_ready {
            return Err(SimAuthoringDiagnostic::new(
                SIM_AUTHORING_PREVIEW_WORKER_UNAVAILABLE_CODE,
                "world-model preview requires worker diagnostics before execution",
            ));
        }
        if !artifact.provenance.artifacts.contains(&artifact.artifact) {
            return Err(SimAuthoringDiagnostic::new(
                SIM_AUTHORING_PREVIEW_MISSING_PROVENANCE_CODE,
                "generated artifact preview requires matching provenance metadata",
            ));
        }

        Ok(SimAuthoringPreviewRoute {
            artifact_path: artifact.artifact.relative_path.display().to_string(),
            preview_kind: SimAuthoringPreviewKind::from(artifact.artifact.artifact_type),
            provenance_backend: artifact.provenance.backend_name.clone(),
            provenance_workflow: artifact.provenance.workflow_name.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimAuthoringItem {
    pub id: String,
    pub label: String,
    pub kind: SimAuthoringItemKind,
    pub target: SimAuthoringItemTarget,
}

impl SimAuthoringItem {
    pub fn project_asset(id: impl Into<String>, path: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            id: id.into(),
            label: path.clone(),
            kind: SimAuthoringItemKind::ProjectAsset,
            target: SimAuthoringItemTarget::Path(path),
        }
    }

    pub fn diffusion_graph(id: impl Into<String>, label: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            label: label.into(),
            kind: SimAuthoringItemKind::DiffusionGraph,
            target: SimAuthoringItemTarget::Graph(id),
        }
    }

    pub fn world_model_request(id: impl Into<String>, request: WorldGenerationRequest) -> Self {
        let id = id.into();
        Self {
            id,
            label: request.output_target.clone(),
            kind: SimAuthoringItemKind::WorldModelRequest,
            target: SimAuthoringItemTarget::WorldModelRequest(request),
        }
    }

    pub fn generated_artifact(path: impl Into<String>, provenance_id: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            id: format!("generated:{path}"),
            label: path.clone(),
            kind: SimAuthoringItemKind::GeneratedArtifact,
            target: SimAuthoringItemTarget::GeneratedArtifact {
                path,
                provenance_id: provenance_id.into(),
            },
        }
    }

    pub fn run_export_task(id: impl Into<String>, label: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            label: label.into(),
            kind: SimAuthoringItemKind::RunExportTask,
            target: SimAuthoringItemTarget::Task(id),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimAuthoringItemKind {
    ProjectAsset,
    DiffusionGraph,
    WorldModelRequest,
    GeneratedArtifact,
    RunExportTask,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SimAuthoringItemTarget {
    Path(String),
    Graph(String),
    WorldModelRequest(WorldGenerationRequest),
    GeneratedArtifact { path: String, provenance_id: String },
    Task(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAuthoringRoute {
    pub item_id: String,
    pub route_kind: SimAuthoringRouteKind,
    pub diagnostics: Vec<SimAuthoringDiagnostic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimAuthoringRouteKind {
    Inspector,
    GraphEditor,
    Preview,
    TaskView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAuthoringPreviewRoute {
    pub artifact_path: String,
    pub preview_kind: SimAuthoringPreviewKind,
    pub provenance_backend: Option<String>,
    pub provenance_workflow: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimAuthoringPreviewKind {
    Image,
    Video,
    Audio,
    Mesh,
    Other,
}

impl From<ArtifactType> for SimAuthoringPreviewKind {
    fn from(artifact_type: ArtifactType) -> Self {
        match artifact_type {
            ArtifactType::Image | ArtifactType::Texture => Self::Image,
            ArtifactType::Video => Self::Video,
            ArtifactType::Audio => Self::Audio,
            ArtifactType::Mesh => Self::Mesh,
            ArtifactType::Control | ArtifactType::Other => Self::Other,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAuthoringDiagnostic {
    pub code: String,
    pub message: String,
}

impl SimAuthoringDiagnostic {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
