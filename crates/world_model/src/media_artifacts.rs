use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{ArtifactType, GeneratedWorldArtifact};

pub const GENERATED_MEDIA_MISSING_PREVIEW_CODE: &str = "world_model.media.missing_preview";
pub const GENERATED_MEDIA_UNSUPPORTED_PREVIEW_CODE: &str = "world_model.media.unsupported_preview";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GeneratedMediaPreviewKind {
    Image,
    Video,
    Audio,
    Mesh,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratedMediaPreviewRoute {
    pub artifact_path: PathBuf,
    pub preview_path: PathBuf,
    pub kind: GeneratedMediaPreviewKind,
    pub provenance_backend: Option<String>,
    pub provenance_workflow: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeneratedMediaPreviewDiagnostic {
    pub code: String,
    pub artifact_path: PathBuf,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeneratedMediaPreviewRouter;

impl GeneratedMediaPreviewRouter {
    pub fn new() -> Self {
        Self
    }

    pub fn route(
        &self,
        generated: &GeneratedWorldArtifact,
    ) -> Result<GeneratedMediaPreviewRoute, GeneratedMediaPreviewDiagnostic> {
        let artifact = &generated.artifact;
        if !artifact.preview_supported {
            return Err(preview_diagnostic(
                GENERATED_MEDIA_UNSUPPORTED_PREVIEW_CODE,
                artifact.relative_path.clone(),
                "generated media artifact does not support preview routing",
            ));
        }
        let Some(preview_path) = artifact.preview_path.clone() else {
            return Err(preview_diagnostic(
                GENERATED_MEDIA_MISSING_PREVIEW_CODE,
                artifact.relative_path.clone(),
                "generated media artifact is missing preview metadata",
            ));
        };
        let kind = preview_kind(artifact.artifact_type);
        if kind == GeneratedMediaPreviewKind::Unsupported {
            return Err(preview_diagnostic(
                GENERATED_MEDIA_UNSUPPORTED_PREVIEW_CODE,
                artifact.relative_path.clone(),
                "generated media artifact type is not supported by native Sim preview routing",
            ));
        }

        Ok(GeneratedMediaPreviewRoute {
            artifact_path: artifact.relative_path.clone(),
            preview_path,
            kind,
            provenance_backend: generated.provenance.backend_name.clone(),
            provenance_workflow: generated.provenance.workflow_name.clone(),
        })
    }
}

fn preview_kind(artifact_type: ArtifactType) -> GeneratedMediaPreviewKind {
    match artifact_type {
        ArtifactType::Image | ArtifactType::Texture => GeneratedMediaPreviewKind::Image,
        ArtifactType::Video => GeneratedMediaPreviewKind::Video,
        ArtifactType::Audio => GeneratedMediaPreviewKind::Audio,
        ArtifactType::Mesh => GeneratedMediaPreviewKind::Mesh,
        ArtifactType::Control | ArtifactType::Other => GeneratedMediaPreviewKind::Unsupported,
    }
}

fn preview_diagnostic(
    code: impl Into<String>,
    artifact_path: PathBuf,
    message: impl Into<String>,
) -> GeneratedMediaPreviewDiagnostic {
    GeneratedMediaPreviewDiagnostic {
        code: code.into(),
        artifact_path,
        message: message.into(),
    }
}
