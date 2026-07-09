use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameSpatialPreviewRoute {
    pub route_name: String,
    pub target: String,
}

impl SimGameSpatialPreviewRoute {
    pub fn scene(target: impl Into<String>) -> Self {
        Self {
            route_name: "sim_game.spatial.preview.scene".to_string(),
            target: target.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameSpatialAssetMetadata {
    pub source_path: PathBuf,
    pub spatial_classes: Vec<String>,
    pub docs_symbols: Vec<String>,
    pub preview_route: Option<SimGameSpatialPreviewRoute>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameSpatialMetadataExtractor;

impl SimGameSpatialMetadataExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract(
        &self,
        source_path: impl Into<PathBuf>,
        source: &str,
    ) -> SimGameSpatialAssetMetadata {
        let source_path = source_path.into();
        let mut metadata = SimGameSpatialAssetMetadata {
            source_path: source_path.clone(),
            ..Default::default()
        };

        for line in source.lines() {
            let line = line.trim();
            for symbol in [
                "XROrigin3D",
                "XRCamera3D",
                "XRController3D",
                "Camera3D",
                "Node3D",
            ] {
                if line_contains_symbol(line, symbol) {
                    metadata.spatial_classes.push(symbol.to_string());
                    metadata.docs_symbols.push(symbol.to_string());
                }
            }
        }

        metadata.spatial_classes.sort();
        metadata.spatial_classes.dedup();
        metadata.docs_symbols.sort();
        metadata.docs_symbols.dedup();

        if !metadata.spatial_classes.is_empty() {
            metadata.preview_route = Some(SimGameSpatialPreviewRoute::scene(
                source_path.display().to_string(),
            ));
        }

        metadata
    }
}

fn line_contains_symbol(line: &str, symbol: &str) -> bool {
    line.contains(&format!("type=\"{symbol}\""))
        || line.contains(&format!("\"{symbol}\""))
        || line == symbol
}
