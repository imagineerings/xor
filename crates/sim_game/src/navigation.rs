use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameNavigationMetadata {
    pub source_path: PathBuf,
    pub regions: Vec<String>,
    pub navigation_meshes: Vec<String>,
    pub docs_symbols: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameNavigationMetadataExtractor;

impl SimGameNavigationMetadataExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract(
        &self,
        source_path: impl Into<PathBuf>,
        source: &str,
    ) -> SimGameNavigationMetadata {
        let mut metadata = SimGameNavigationMetadata {
            source_path: source_path.into(),
            ..Default::default()
        };

        for line in source.lines() {
            let line = line.trim();
            if line.contains("NavigationRegion3D") {
                metadata.regions.push("NavigationRegion3D".to_string());
                metadata.docs_symbols.push("NavigationRegion3D".to_string());
            }
            if line.contains("NavigationMesh") {
                metadata
                    .navigation_meshes
                    .push("NavigationMesh".to_string());
                metadata.docs_symbols.push("NavigationMesh".to_string());
            }
        }

        metadata.docs_symbols.sort();
        metadata.docs_symbols.dedup();
        metadata
    }
}
