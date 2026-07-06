use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::WorldGenerationRequest;

// ---------------------------------------------------------------------------
// Artifact type
// ---------------------------------------------------------------------------

/// The kind of generated artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArtifactType {
    Video,
    Image,
    Mesh,
    Texture,
    Audio,
    Control,
    Other,
}

impl ArtifactType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Image => "image",
            Self::Mesh => "mesh",
            Self::Texture => "texture",
            Self::Audio => "audio",
            Self::Control => "control",
            Self::Other => "other",
        }
    }
}

// ---------------------------------------------------------------------------
// Artifact record
// ---------------------------------------------------------------------------

/// A generated artifact with metadata for preview, export, and provenance
/// tracking (Requirements 4.2, 7.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub relative_path: PathBuf,
    pub artifact_type: ArtifactType,
    pub label: Option<String>,
    pub preview_supported: bool,
    pub preview_path: Option<PathBuf>,
    pub export_path: Option<PathBuf>,
}

impl ArtifactRecord {
    pub fn new(relative_path: impl Into<PathBuf>, artifact_type: ArtifactType) -> Self {
        Self {
            relative_path: relative_path.into(),
            artifact_type,
            label: None,
            preview_supported: false,
            preview_path: None,
            export_path: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_preview(mut self, preview_path: impl Into<PathBuf>) -> Self {
        self.preview_supported = true;
        self.preview_path = Some(preview_path.into());
        self
    }

    pub fn with_export(mut self, export_path: impl Into<PathBuf>) -> Self {
        self.export_path = Some(export_path.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Generation provenance
// ---------------------------------------------------------------------------

/// Provenance metadata for a generated artifact, preserving the full
/// generation context (Requirement 5.4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenerationProvenance {
    pub request: WorldGenerationRequest,
    pub artifacts: Vec<ArtifactRecord>,
    pub graph_node: Option<String>,
    pub workflow_name: Option<String>,
    pub backend_name: Option<String>,
    pub model_family: Option<String>,
    pub generation_time_ms: Option<u64>,
    pub notes: Option<String>,
}

impl GenerationProvenance {
    pub fn new(request: WorldGenerationRequest) -> Self {
        Self {
            request,
            artifacts: Vec::new(),
            graph_node: None,
            workflow_name: None,
            backend_name: None,
            model_family: None,
            generation_time_ms: None,
            notes: None,
        }
    }

    pub fn with_artifact(mut self, artifact: ArtifactRecord) -> Self {
        self.artifacts.push(artifact);
        self
    }

    pub fn with_graph_node(mut self, node: impl Into<String>) -> Self {
        self.graph_node = Some(node.into());
        self
    }

    pub fn with_workflow(mut self, name: impl Into<String>) -> Self {
        self.workflow_name = Some(name.into());
        self
    }

    pub fn with_backend(mut self, name: impl Into<String>) -> Self {
        self.backend_name = Some(name.into());
        self
    }

    pub fn with_generation_time(mut self, ms: u64) -> Self {
        self.generation_time_ms = Some(ms);
        self
    }
}

// ---------------------------------------------------------------------------
// Provenance collection
// ---------------------------------------------------------------------------

/// A collection of generation provenance records for a session or project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceCollection {
    pub records: Vec<GenerationProvenance>,
}

impl ProvenanceCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, provenance: GenerationProvenance) {
        self.records.push(provenance);
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn find_by_prompt(&self, prompt_substring: &str) -> Vec<&GenerationProvenance> {
        self.records
            .iter()
            .filter(|r| r.request.prompt.contains(prompt_substring))
            .collect()
    }
}

impl IntoIterator for ProvenanceCollection {
    type Item = GenerationProvenance;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.into_iter()
    }
}
