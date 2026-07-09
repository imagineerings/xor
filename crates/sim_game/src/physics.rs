use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameSimulationFeatureKind {
    PhysicsServerRuntime,
    NavigationServerRuntime,
    MetadataInspection,
    ExternalSimulationFallback,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameSimulationFeature {
    pub name: String,
    pub kind: SimGameSimulationFeatureKind,
}

impl SimGameSimulationFeature {
    pub fn new(name: impl Into<String>, kind: SimGameSimulationFeatureKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameSimulationBoundaryDecision {
    NativeMetadata,
    ExternalFallback,
    Excluded { reason: String },
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameSimulationBoundary;

impl SimGameSimulationBoundary {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(
        &self,
        feature: &SimGameSimulationFeature,
    ) -> SimGameSimulationBoundaryDecision {
        match feature.kind {
            SimGameSimulationFeatureKind::MetadataInspection => {
                SimGameSimulationBoundaryDecision::NativeMetadata
            }
            SimGameSimulationFeatureKind::ExternalSimulationFallback => {
                SimGameSimulationBoundaryDecision::ExternalFallback
            }
            SimGameSimulationFeatureKind::PhysicsServerRuntime
            | SimGameSimulationFeatureKind::NavigationServerRuntime => {
                SimGameSimulationBoundaryDecision::Excluded {
                    reason: format!(
                        "{} requires Godot server execution and is not embedded in Sim",
                        feature.name
                    ),
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGamePhysicsMetadata {
    pub source_path: PathBuf,
    pub bodies: Vec<String>,
    pub collision_shapes: Vec<String>,
    pub docs_symbols: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGamePhysicsMetadataExtractor;

impl SimGamePhysicsMetadataExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract(&self, source_path: impl Into<PathBuf>, source: &str) -> SimGamePhysicsMetadata {
        let mut metadata = SimGamePhysicsMetadata {
            source_path: source_path.into(),
            ..Default::default()
        };

        for line in source.lines() {
            let line = line.trim();
            if line.contains("RigidBody3D") {
                metadata.bodies.push("RigidBody3D".to_string());
                metadata.docs_symbols.push("RigidBody3D".to_string());
            }
            if line.contains("StaticBody3D") {
                metadata.bodies.push("StaticBody3D".to_string());
                metadata.docs_symbols.push("StaticBody3D".to_string());
            }
            if line.contains("CollisionShape3D") {
                metadata
                    .collision_shapes
                    .push("CollisionShape3D".to_string());
                metadata.docs_symbols.push("CollisionShape3D".to_string());
            }
        }

        metadata.docs_symbols.sort();
        metadata.docs_symbols.dedup();
        metadata
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameSimulationFallbackTask {
    pub id: String,
    pub label: String,
    pub command_template: Option<String>,
    pub diagnostics: Vec<String>,
}

impl SimGameSimulationFallbackTask {
    pub fn external(command_template: impl Into<String>) -> Self {
        Self {
            id: "sim_game.simulation.external".to_string(),
            label: "Run external game simulation".to_string(),
            command_template: Some(command_template.into()),
            diagnostics: Vec::new(),
        }
    }

    pub fn missing_configuration() -> Self {
        Self {
            id: "sim_game.simulation.external".to_string(),
            label: "Run external game simulation".to_string(),
            command_template: None,
            diagnostics: vec![
                "configure an external simulation task before running physics/navigation fallback"
                    .to_string(),
            ],
        }
    }
}
