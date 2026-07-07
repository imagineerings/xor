use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SimGameSourceReference {
    pub path: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl SimGameSourceReference {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            line: None,
            column: None,
        }
    }

    pub fn with_position(mut self, line: u32, column: u32) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum SimGameProjectFormat {
    GodotCompatible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum SimGameFeatureArea {
    ProjectMetadata,
    SceneResourceMetadata,
    ScriptMetadata,
    ShaderMetadata,
    AssetMetadata,
    RuntimeExecution,
    RenderingRuntime,
    PlatformRuntime,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum RuntimeBoundaryDecision {
    NativeSimFeature,
    SimAdapter { owner: String },
    ExternalCommand { command: String },
    Excluded { reason: String },
}

impl RuntimeBoundaryDecision {
    pub fn is_executable_inside_sim(&self) -> bool {
        matches!(self, Self::NativeSimFeature | Self::SimAdapter { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SimGameMetadata {
    pub feature_area: SimGameFeatureArea,
    pub source: SimGameSourceReference,
    pub boundary: RuntimeBoundaryDecision,
}

impl SimGameMetadata {
    pub fn new(
        feature_area: SimGameFeatureArea,
        source: SimGameSourceReference,
        boundary: RuntimeBoundaryDecision,
    ) -> Self {
        Self {
            feature_area,
            source,
            boundary,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SimGameProjectDescriptor {
    pub root_path: PathBuf,
    pub manifest_path: PathBuf,
    pub format: SimGameProjectFormat,
}

impl SimGameProjectDescriptor {
    pub fn from_godot_compatible_manifest_path(manifest_path: impl Into<PathBuf>) -> Option<Self> {
        let manifest_path = manifest_path.into();
        if !is_godot_compatible_manifest(&manifest_path) {
            return None;
        }

        let root_path = manifest_path.parent()?.to_path_buf();
        Some(Self {
            root_path,
            manifest_path,
            format: SimGameProjectFormat::GodotCompatible,
        })
    }
}

pub fn is_godot_compatible_manifest(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|file_name| file_name == "project.godot")
}
