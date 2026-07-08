use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const BLUEPRINT_COUNT_MISMATCH_CODE: &str = "world_model.comfy_blueprints.count_mismatch";
pub const DUPLICATE_BLUEPRINT_CODE: &str = "world_model.comfy_blueprints.duplicate";
pub const MISSING_BLUEPRINT_DEPENDENCY_CODE: &str =
    "world_model.comfy_blueprints.missing_dependency";
pub const UNSUPPORTED_BLUEPRINT_NODE_CODE: &str = "world_model.comfy_blueprints.unsupported_node";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ComfyBlueprintCategory {
    Image,
    Video,
    Audio,
    ThreeD,
    Depth,
    Segmentation,
    Pose,
    Text,
    Utility,
}

impl ComfyBlueprintCategory {
    pub fn parse(value: &str) -> Self {
        match value {
            "image" => Self::Image,
            "video" => Self::Video,
            "audio" => Self::Audio,
            "three_d" => Self::ThreeD,
            "depth" => Self::Depth,
            "segmentation" => Self::Segmentation,
            "pose" => Self::Pose,
            "text" => Self::Text,
            _ => Self::Utility,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ComfyBlueprintDependencyKind {
    Glsl,
    Helper,
    Asset,
}

impl ComfyBlueprintDependencyKind {
    pub fn parse(value: &str) -> Self {
        match value {
            "glsl" => Self::Glsl,
            "helper" => Self::Helper,
            _ => Self::Asset,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ComfyBlueprintDependency {
    pub kind: ComfyBlueprintDependencyKind,
    pub source_path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyBlueprintRecord {
    pub name: String,
    pub source_path: String,
    pub category: ComfyBlueprintCategory,
    pub graph_json: serde_json::Value,
    pub node_count: usize,
    pub link_count: usize,
    pub node_types: BTreeSet<String>,
    pub dependencies: Vec<ComfyBlueprintDependency>,
    pub attribution: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyBlueprintDiagnostic {
    pub code: String,
    pub blueprint_name: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComfyBlueprintCatalog {
    records: BTreeMap<String, ComfyBlueprintRecord>,
    diagnostics: Vec<ComfyBlueprintDiagnostic>,
}

impl ComfyBlueprintCatalog {
    pub fn from_manifest(
        manifest: &str,
        available_dependencies: impl IntoIterator<Item = String>,
        supported_node_types: impl IntoIterator<Item = String>,
    ) -> Result<Self, serde_json::Error> {
        let manifest: BlueprintManifest = serde_json::from_str(manifest)?;
        let available_dependencies = available_dependencies.into_iter().collect::<BTreeSet<_>>();
        let supported_node_types = supported_node_types.into_iter().collect::<BTreeSet<_>>();

        let mut records = BTreeMap::new();
        let mut diagnostics = Vec::new();

        if manifest.blueprints.len() != manifest.expected_blueprint_count {
            diagnostics.push(ComfyBlueprintDiagnostic {
                code: BLUEPRINT_COUNT_MISMATCH_CODE.to_string(),
                blueprint_name: None,
                message: format!(
                    "manifest expected {} blueprints but contained {}",
                    manifest.expected_blueprint_count,
                    manifest.blueprints.len()
                ),
            });
        }

        for fixture in manifest.blueprints {
            if records.contains_key(&fixture.name) {
                diagnostics.push(ComfyBlueprintDiagnostic {
                    code: DUPLICATE_BLUEPRINT_CODE.to_string(),
                    blueprint_name: Some(fixture.name.clone()),
                    message: format!("duplicate blueprint `{}`", fixture.name),
                });
                continue;
            }

            let dependencies = fixture
                .dependencies
                .into_iter()
                .map(|dependency| ComfyBlueprintDependency {
                    kind: ComfyBlueprintDependencyKind::parse(&dependency.kind),
                    source_path: dependency.source_path,
                })
                .collect::<Vec<_>>();

            for dependency in &dependencies {
                if !available_dependencies.contains(&dependency.source_path) {
                    diagnostics.push(ComfyBlueprintDiagnostic {
                        code: MISSING_BLUEPRINT_DEPENDENCY_CODE.to_string(),
                        blueprint_name: Some(fixture.name.clone()),
                        message: format!(
                            "blueprint `{}` references missing dependency `{}`",
                            fixture.name, dependency.source_path
                        ),
                    });
                }
            }

            let node_types = fixture.node_types.into_iter().collect::<BTreeSet<_>>();
            for node_type in &node_types {
                if !supported_node_types.contains(node_type) {
                    diagnostics.push(ComfyBlueprintDiagnostic {
                        code: UNSUPPORTED_BLUEPRINT_NODE_CODE.to_string(),
                        blueprint_name: Some(fixture.name.clone()),
                        message: format!(
                            "blueprint `{}` references unsupported node `{node_type}`",
                            fixture.name
                        ),
                    });
                }
            }

            records.insert(
                fixture.name.clone(),
                ComfyBlueprintRecord {
                    name: fixture.name,
                    source_path: fixture.source_path,
                    category: ComfyBlueprintCategory::parse(&fixture.category),
                    graph_json: fixture.graph_json,
                    node_count: fixture.node_count,
                    link_count: fixture.link_count,
                    node_types,
                    dependencies,
                    attribution: fixture.attribution,
                },
            );
        }

        Ok(Self {
            records,
            diagnostics,
        })
    }

    pub fn records(&self) -> impl Iterator<Item = &ComfyBlueprintRecord> {
        self.records.values()
    }

    pub fn record(&self, name: &str) -> Option<&ComfyBlueprintRecord> {
        self.records.get(name)
    }

    pub fn diagnostics(&self) -> &[ComfyBlueprintDiagnostic] {
        &self.diagnostics
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn categories(&self) -> BTreeSet<ComfyBlueprintCategory> {
        self.records
            .values()
            .map(|record| record.category.clone())
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct BlueprintManifest {
    expected_blueprint_count: usize,
    blueprints: Vec<BlueprintFixture>,
}

#[derive(Debug, Deserialize)]
struct BlueprintFixture {
    name: String,
    source_path: String,
    category: String,
    attribution: String,
    graph_json: serde_json::Value,
    node_count: usize,
    link_count: usize,
    node_types: Vec<String>,
    dependencies: Vec<BlueprintDependencyFixture>,
}

#[derive(Debug, Deserialize)]
struct BlueprintDependencyFixture {
    kind: String,
    source_path: String,
}
