use serde::{Deserialize, Serialize};
use world_model::{MeshArtifactMetadata, MeshFormat};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimGeneratedAssetRecord {
    pub asset_path: String,
    pub format: MeshFormat,
    pub preview_path: Option<String>,
    pub export_path: Option<String>,
    pub export_format: Option<MeshFormat>,
    pub provenance_id: String,
    pub source_assets: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGeneratedAssetDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimGeneratedAssetRegistry {
    assets: Vec<SimGeneratedAssetRecord>,
}

impl SimGeneratedAssetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_mesh(
        &mut self,
        metadata: MeshArtifactMetadata,
    ) -> Result<SimGeneratedAssetRecord, SimGeneratedAssetDiagnostic> {
        let Some(provenance_id) = metadata.provenance_id.clone() else {
            return Err(SimGeneratedAssetDiagnostic {
                code: "sim_game.generated_asset.missing_provenance".to_string(),
                message: "generated mesh asset registration requires provenance metadata"
                    .to_string(),
            });
        };
        if metadata.mesh_path.trim().is_empty() {
            return Err(SimGeneratedAssetDiagnostic {
                code: "sim_game.generated_asset.missing_asset_path".to_string(),
                message: "generated mesh asset registration requires an asset path".to_string(),
            });
        }

        let record = SimGeneratedAssetRecord {
            asset_path: metadata.mesh_path,
            format: metadata.format,
            preview_path: metadata.preview_path,
            export_path: metadata.export_path,
            export_format: metadata.export_format,
            provenance_id,
            source_assets: metadata.source_assets,
        };
        self.assets.push(record.clone());
        Ok(record)
    }

    pub fn assets(&self) -> &[SimGeneratedAssetRecord] {
        &self.assets
    }
}
