use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameNetworkFeatureKind {
    MultiplayerRuntime,
    EnetProtocol,
    UpnpDiscovery,
    PacketPeerProtocol,
    DebugMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameNetworkFeature {
    pub name: String,
    pub kind: SimGameNetworkFeatureKind,
}

impl SimGameNetworkFeature {
    pub fn new(name: impl Into<String>, kind: SimGameNetworkFeatureKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameNetworkBoundaryDecision {
    NativeDebugMetadata,
    Excluded { reason: String },
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameNetworkBoundary;

impl SimGameNetworkBoundary {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(&self, feature: &SimGameNetworkFeature) -> SimGameNetworkBoundaryDecision {
        match feature.kind {
            SimGameNetworkFeatureKind::DebugMetadata => {
                SimGameNetworkBoundaryDecision::NativeDebugMetadata
            }
            SimGameNetworkFeatureKind::MultiplayerRuntime
            | SimGameNetworkFeatureKind::EnetProtocol
            | SimGameNetworkFeatureKind::UpnpDiscovery
            | SimGameNetworkFeatureKind::PacketPeerProtocol => {
                SimGameNetworkBoundaryDecision::Excluded {
                    reason: format!(
                        "{} duplicates Sim collaboration/networking infrastructure and is not ported",
                        feature.name
                    ),
                }
            }
        }
    }
}
