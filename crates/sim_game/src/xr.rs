use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameXrFeatureKind {
    OpenXrRuntime,
    WebXrRuntime,
    VrRuntime,
    SpatialMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameXrFeature {
    pub name: String,
    pub kind: SimGameXrFeatureKind,
}

impl SimGameXrFeature {
    pub fn new(name: impl Into<String>, kind: SimGameXrFeatureKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameXrBoundaryDecision {
    NativeSpatialMetadata,
    Excluded { reason: String },
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameXrBoundary;

impl SimGameXrBoundary {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(&self, feature: &SimGameXrFeature) -> SimGameXrBoundaryDecision {
        match feature.kind {
            SimGameXrFeatureKind::SpatialMetadata => {
                SimGameXrBoundaryDecision::NativeSpatialMetadata
            }
            SimGameXrFeatureKind::OpenXrRuntime
            | SimGameXrFeatureKind::WebXrRuntime
            | SimGameXrFeatureKind::VrRuntime => SimGameXrBoundaryDecision::Excluded {
                reason: format!(
                    "{} requires XR runtime migration and is not embedded in Sim",
                    feature.name
                ),
            },
        }
    }
}
