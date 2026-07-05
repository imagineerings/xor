use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct BaymaxGameSourceReference {
    pub path: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl BaymaxGameSourceReference {
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
pub enum BaymaxGameProjectFormat {
    GodotCompatible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BaymaxGameFeatureArea {
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
    NativeBaymaxFeature,
    BaymaxAdapter { owner: String },
    ExternalCommand { command: String },
    Excluded { reason: String },
}

impl RuntimeBoundaryDecision {
    pub fn is_executable_inside_baymax(&self) -> bool {
        matches!(self, Self::NativeBaymaxFeature | Self::BaymaxAdapter { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BaymaxGameMetadata {
    pub feature_area: BaymaxGameFeatureArea,
    pub source: BaymaxGameSourceReference,
    pub boundary: RuntimeBoundaryDecision,
}

impl BaymaxGameMetadata {
    pub fn new(
        feature_area: BaymaxGameFeatureArea,
        source: BaymaxGameSourceReference,
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
pub struct BaymaxGameProjectDescriptor {
    pub root_path: PathBuf,
    pub manifest_path: PathBuf,
    pub format: BaymaxGameProjectFormat,
}

impl BaymaxGameProjectDescriptor {
    pub fn from_godot_compatible_manifest_path(manifest_path: impl Into<PathBuf>) -> Option<Self> {
        let manifest_path = manifest_path.into();
        if !is_godot_compatible_manifest(&manifest_path) {
            return None;
        }

        let root_path = manifest_path.parent()?.to_path_buf();
        Some(Self {
            root_path,
            manifest_path,
            format: BaymaxGameProjectFormat::GodotCompatible,
        })
    }
}

pub fn is_godot_compatible_manifest(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|file_name| file_name == "project.godot")
}
