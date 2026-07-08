use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{MeshArtifactMetadata, MeshBackend, MeshFormat};

pub const SIM_THREE_D_INVALID_GEOMETRY_CODE: &str = "world_model.three_d_nodes.invalid_geometry";
pub const SIM_THREE_D_DEPENDENCY_REVIEW_REQUIRED_CODE: &str =
    "world_model.three_d_nodes.dependency_review_required";
pub const SIM_THREE_D_UNSUPPORTED_FORMAT_CODE: &str =
    "world_model.three_d_nodes.unsupported_format";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimThreeDArtifactKind {
    Mesh,
    PointCloud,
    GaussianSplat,
    DepthMap,
    NormalMap,
    Camera,
    PointMap,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimThreeDMetadata {
    pub format: String,
    pub preview_reference: Option<String>,
    pub provenance_id: Option<String>,
    pub source_assets: Vec<String>,
    pub vertex_count: Option<u64>,
    pub triangle_count: Option<u64>,
    pub point_count: Option<u64>,
    pub fields: BTreeMap<String, String>,
}

impl SimThreeDMetadata {
    pub fn new(format: impl Into<String>) -> Self {
        Self {
            format: format.into(),
            preview_reference: None,
            provenance_id: None,
            source_assets: Vec::new(),
            vertex_count: None,
            triangle_count: None,
            point_count: None,
            fields: BTreeMap::new(),
        }
    }

    pub fn with_preview_reference(mut self, preview_reference: impl Into<String>) -> Self {
        self.preview_reference = Some(preview_reference.into());
        self
    }

    pub fn with_provenance(mut self, provenance_id: impl Into<String>) -> Self {
        self.provenance_id = Some(provenance_id.into());
        self
    }

    pub fn with_source_asset(mut self, source_asset: impl Into<String>) -> Self {
        self.source_assets.push(source_asset.into());
        self
    }

    pub fn with_vertex_count(mut self, vertex_count: u64) -> Self {
        self.vertex_count = Some(vertex_count);
        self
    }

    pub fn with_triangle_count(mut self, triangle_count: u64) -> Self {
        self.triangle_count = Some(triangle_count);
        self
    }

    pub fn with_point_count(mut self, point_count: u64) -> Self {
        self.point_count = Some(point_count);
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimThreeDArtifact {
    pub reference: String,
    pub kind: SimThreeDArtifactKind,
    pub metadata: SimThreeDMetadata,
}

impl SimThreeDArtifact {
    pub fn new(
        reference: impl Into<String>,
        kind: SimThreeDArtifactKind,
        metadata: SimThreeDMetadata,
    ) -> Self {
        Self {
            reference: reference.into(),
            kind,
            metadata,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimMeshPipelineDelegation {
    pub artifact: SimThreeDArtifact,
    pub mesh_metadata: MeshArtifactMetadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimThreeDOperation {
    Load,
    Preview,
    Transform,
    Render,
    Merge,
    Save,
    Convert,
    RegisterGeometry,
    GaussianSplatPreview,
    TexturedMeshExport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimThreeDBackendStatus {
    Native,
    DependencyReviewRequired,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimThreeDNodeDiagnostic {
    pub code: String,
    pub operation: Option<SimThreeDOperation>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimThreeDNodeAdapter;

impl SimThreeDNodeAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn load_mesh(&self, mesh_metadata: MeshArtifactMetadata) -> SimThreeDArtifact {
        SimThreeDArtifact::new(
            mesh_metadata.mesh_path.clone(),
            SimThreeDArtifactKind::Mesh,
            metadata_from_mesh(&mesh_metadata).with_field("sim.operation", "load"),
        )
    }

    pub fn preview(
        &self,
        artifact: &SimThreeDArtifact,
        preview_reference: impl Into<String>,
    ) -> SimThreeDArtifact {
        let mut artifact = artifact.clone();
        artifact.metadata.preview_reference = Some(preview_reference.into());
        artifact
            .metadata
            .fields
            .insert("sim.operation".to_string(), "preview".to_string());
        artifact
    }

    pub fn save_mesh(
        &self,
        artifact: &SimThreeDArtifact,
        reference: impl Into<String>,
        format: MeshFormat,
    ) -> Result<SimThreeDArtifact, SimThreeDNodeDiagnostic> {
        ensure_mesh(artifact, SimThreeDOperation::Save)?;
        let mut artifact = artifact.clone();
        artifact.reference = reference.into();
        artifact.metadata.format = format.extension().to_string();
        artifact
            .metadata
            .fields
            .insert("sim.operation".to_string(), "save".to_string());
        Ok(artifact)
    }

    pub fn register_geometry(
        &self,
        reference: impl Into<String>,
        kind: SimThreeDArtifactKind,
        metadata: SimThreeDMetadata,
    ) -> Result<SimThreeDArtifact, SimThreeDNodeDiagnostic> {
        if matches!(kind, SimThreeDArtifactKind::Mesh)
            && (metadata.vertex_count.unwrap_or(0) == 0
                || metadata.triangle_count.unwrap_or(0) == 0)
        {
            return Err(diagnostic(
                SIM_THREE_D_INVALID_GEOMETRY_CODE,
                SimThreeDOperation::RegisterGeometry,
                "mesh geometry requires vertex and triangle counts",
            ));
        }
        if matches!(
            kind,
            SimThreeDArtifactKind::PointCloud | SimThreeDArtifactKind::GaussianSplat
        ) && metadata.point_count.unwrap_or(0) == 0
        {
            return Err(diagnostic(
                SIM_THREE_D_INVALID_GEOMETRY_CODE,
                SimThreeDOperation::RegisterGeometry,
                "point cloud and Gaussian splat outputs require point counts",
            ));
        }

        Ok(SimThreeDArtifact::new(
            reference,
            kind,
            metadata.with_field("sim.operation", "register_geometry"),
        ))
    }

    pub fn gaussian_splat_preview(
        &self,
        artifact: &SimThreeDArtifact,
        preview_reference: impl Into<String>,
    ) -> Result<SimThreeDArtifact, SimThreeDNodeDiagnostic> {
        if artifact.kind != SimThreeDArtifactKind::GaussianSplat {
            return Err(diagnostic(
                SIM_THREE_D_INVALID_GEOMETRY_CODE,
                SimThreeDOperation::GaussianSplatPreview,
                "Gaussian splat preview requires a Gaussian splat artifact",
            ));
        }

        Ok(self.preview(artifact, preview_reference))
    }

    pub fn delegate_textured_mesh_export(
        &self,
        mesh_metadata: MeshArtifactMetadata,
    ) -> Result<SimMeshPipelineDelegation, SimThreeDNodeDiagnostic> {
        if !mesh_metadata.has_textures {
            return Err(diagnostic(
                SIM_THREE_D_INVALID_GEOMETRY_CODE,
                SimThreeDOperation::TexturedMeshExport,
                "textured mesh export must use mesh metadata with textures enabled",
            ));
        }

        let artifact = SimThreeDArtifact::new(
            mesh_metadata.mesh_path.clone(),
            SimThreeDArtifactKind::Mesh,
            metadata_from_mesh(&mesh_metadata).with_field("sim.operation", "textured_mesh_export"),
        );
        Ok(SimMeshPipelineDelegation {
            artifact,
            mesh_metadata,
        })
    }

    pub fn backend_diagnostic(
        &self,
        operation: SimThreeDOperation,
        status: SimThreeDBackendStatus,
        reason: impl Into<String>,
    ) -> Option<SimThreeDNodeDiagnostic> {
        let reason = reason.into();
        match status {
            SimThreeDBackendStatus::Native => None,
            SimThreeDBackendStatus::DependencyReviewRequired => Some(SimThreeDNodeDiagnostic {
                code: SIM_THREE_D_DEPENDENCY_REVIEW_REQUIRED_CODE.to_string(),
                operation: Some(operation),
                message: format!("{reason} requires dependency review before native execution"),
            }),
            SimThreeDBackendStatus::Unsupported => Some(SimThreeDNodeDiagnostic {
                code: SIM_THREE_D_UNSUPPORTED_FORMAT_CODE.to_string(),
                operation: Some(operation),
                message: format!("{reason} does not have a native Sim 3D backend yet"),
            }),
        }
    }

    pub fn format_diagnostic(
        &self,
        operation: SimThreeDOperation,
        format: MeshFormat,
    ) -> Option<SimThreeDNodeDiagnostic> {
        format
            .requires_dependency_review()
            .then(|| SimThreeDNodeDiagnostic {
                code: SIM_THREE_D_DEPENDENCY_REVIEW_REQUIRED_CODE.to_string(),
                operation: Some(operation),
                message: format!(
                    "{} export requires dependency review before native execution",
                    format.label()
                ),
            })
    }

    pub fn backend_dependency_diagnostic(
        &self,
        operation: SimThreeDOperation,
        backend: MeshBackend,
    ) -> Option<SimThreeDNodeDiagnostic> {
        backend
            .requires_dependency_review()
            .then(|| SimThreeDNodeDiagnostic {
                code: SIM_THREE_D_DEPENDENCY_REVIEW_REQUIRED_CODE.to_string(),
                operation: Some(operation),
                message: format!(
                    "{} mesh backend requires dependency review before native execution",
                    backend.label()
                ),
            })
    }
}

fn metadata_from_mesh(mesh_metadata: &MeshArtifactMetadata) -> SimThreeDMetadata {
    let mut metadata = SimThreeDMetadata::new(mesh_metadata.format.extension())
        .with_field("sim.mesh_path", mesh_metadata.mesh_path.clone());
    if let Some(preview_path) = &mesh_metadata.preview_path {
        metadata.preview_reference = Some(preview_path.clone());
    }
    if let Some(provenance_id) = &mesh_metadata.provenance_id {
        metadata.provenance_id = Some(provenance_id.clone());
    }
    metadata.source_assets = mesh_metadata.source_assets.clone();
    metadata.vertex_count = mesh_metadata.vertex_count;
    metadata.triangle_count = mesh_metadata.triangle_count;
    metadata.fields.insert(
        "sim.has_textures".to_string(),
        mesh_metadata.has_textures.to_string(),
    );
    if let Some(texture_resolution) = mesh_metadata.texture_resolution {
        metadata.fields.insert(
            "sim.texture_resolution".to_string(),
            texture_resolution.to_string(),
        );
    }
    metadata
}

fn ensure_mesh(
    artifact: &SimThreeDArtifact,
    operation: SimThreeDOperation,
) -> Result<(), SimThreeDNodeDiagnostic> {
    if artifact.kind == SimThreeDArtifactKind::Mesh {
        Ok(())
    } else {
        Err(diagnostic(
            SIM_THREE_D_INVALID_GEOMETRY_CODE,
            operation,
            "operation requires a mesh artifact",
        ))
    }
}

fn diagnostic(
    code: &str,
    operation: SimThreeDOperation,
    message: impl Into<String>,
) -> SimThreeDNodeDiagnostic {
    SimThreeDNodeDiagnostic {
        code: code.to_string(),
        operation: Some(operation),
        message: message.into(),
    }
}
