use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{ArtifactRecord, GenerationProvenance};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GeneratedWorldArtifactError {
    MissingProvenanceArtifact,
}

impl fmt::Display for GeneratedWorldArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProvenanceArtifact => {
                write!(
                    formatter,
                    "generated world artifact requires provenance metadata"
                )
            }
        }
    }
}

impl std::error::Error for GeneratedWorldArtifactError {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratedWorldArtifact {
    pub artifact: ArtifactRecord,
    pub provenance: GenerationProvenance,
}

impl GeneratedWorldArtifact {
    pub fn new(
        artifact: ArtifactRecord,
        provenance: GenerationProvenance,
    ) -> Result<Self, GeneratedWorldArtifactError> {
        if provenance.artifacts.is_empty() {
            return Err(GeneratedWorldArtifactError::MissingProvenanceArtifact);
        }

        Ok(Self {
            artifact,
            provenance,
        })
    }
}
