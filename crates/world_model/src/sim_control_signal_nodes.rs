use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::SimMediaPortType;

pub const SIM_CONTROL_SIGNAL_MISSING_METADATA_CODE: &str =
    "world_model.control_signal_nodes.missing_metadata";
pub const SIM_CONTROL_SIGNAL_TYPE_MISMATCH_CODE: &str =
    "world_model.control_signal_nodes.type_mismatch";
pub const SIM_CONTROL_SIGNAL_UNSUPPORTED_BACKEND_CODE: &str =
    "world_model.control_signal_nodes.unsupported_backend";
pub const SIM_CONTROL_SIGNAL_DEPENDENCY_REVIEW_REQUIRED_CODE: &str =
    "world_model.control_signal_nodes.dependency_review_required";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimControlSignalKind {
    Canny,
    Pose,
    Keypoints,
    BoundingBoxes,
    FaceLandmarks,
    Segmentation,
    Detection,
    Depth,
    Geometry,
    OpticalFlow,
    CameraTrajectory,
    Tracking,
}

impl SimControlSignalKind {
    pub fn port_type(self) -> SimMediaPortType {
        match self {
            Self::Canny | Self::OpticalFlow | Self::CameraTrajectory | Self::Tracking => {
                SimMediaPortType::ControlSignal
            }
            Self::Pose | Self::Keypoints | Self::FaceLandmarks => SimMediaPortType::Pose,
            Self::BoundingBoxes | Self::Detection => SimMediaPortType::BoundingBoxes,
            Self::Segmentation => SimMediaPortType::Segmentation,
            Self::Depth => SimMediaPortType::DepthMap,
            Self::Geometry => SimMediaPortType::PointCloud,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimControlSignalMetadata {
    pub source_reference: String,
    pub width: u32,
    pub height: u32,
    pub frames: Option<u32>,
    pub confidence_basis: Option<String>,
    pub fields: BTreeMap<String, String>,
}

impl SimControlSignalMetadata {
    pub fn new(source_reference: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            source_reference: source_reference.into(),
            width,
            height,
            frames: None,
            confidence_basis: None,
            fields: BTreeMap::new(),
        }
    }

    pub fn with_frames(mut self, frames: u32) -> Self {
        self.frames = Some(frames);
        self
    }

    pub fn with_confidence_basis(mut self, confidence_basis: impl Into<String>) -> Self {
        self.confidence_basis = Some(confidence_basis.into());
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimControlSignalArtifact {
    pub reference: String,
    pub kind: SimControlSignalKind,
    pub port_type: SimMediaPortType,
    pub metadata: SimControlSignalMetadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimControlTargetKind {
    ControlNet,
    Inpainting,
    PoseToImage,
    PoseToVideo,
    DepthToImage,
    DepthToVideo,
    SegmentationToImage,
    DetectionToImage,
    CameraToVideo,
}

impl SimControlTargetKind {
    pub fn accepts(self, kind: SimControlSignalKind) -> bool {
        match self {
            Self::ControlNet => matches!(
                kind,
                SimControlSignalKind::Canny
                    | SimControlSignalKind::Pose
                    | SimControlSignalKind::Depth
                    | SimControlSignalKind::Segmentation
            ),
            Self::Inpainting => matches!(
                kind,
                SimControlSignalKind::Segmentation | SimControlSignalKind::Detection
            ),
            Self::PoseToImage | Self::PoseToVideo => matches!(
                kind,
                SimControlSignalKind::Pose
                    | SimControlSignalKind::Keypoints
                    | SimControlSignalKind::FaceLandmarks
            ),
            Self::DepthToImage | Self::DepthToVideo => matches!(
                kind,
                SimControlSignalKind::Depth | SimControlSignalKind::Geometry
            ),
            Self::SegmentationToImage => kind == SimControlSignalKind::Segmentation,
            Self::DetectionToImage => matches!(
                kind,
                SimControlSignalKind::Detection | SimControlSignalKind::BoundingBoxes
            ),
            Self::CameraToVideo => kind == SimControlSignalKind::CameraTrajectory,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimControlSignalBackendStatus {
    Native,
    DependencyReviewRequired,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimControlSignalDiagnostic {
    pub code: String,
    pub signal_kind: Option<SimControlSignalKind>,
    pub target_kind: Option<SimControlTargetKind>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimControlSignalNodeAdapter;

impl SimControlSignalNodeAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(
        &self,
        reference: impl Into<String>,
        kind: SimControlSignalKind,
        metadata: SimControlSignalMetadata,
    ) -> Result<SimControlSignalArtifact, SimControlSignalDiagnostic> {
        validate_metadata(kind, &metadata)?;
        Ok(SimControlSignalArtifact {
            reference: reference.into(),
            kind,
            port_type: kind.port_type(),
            metadata: metadata.with_field("sim.operation", "analyze"),
        })
    }

    pub fn validate_compatibility(
        &self,
        signal: &SimControlSignalArtifact,
        target_kind: SimControlTargetKind,
    ) -> Result<(), SimControlSignalDiagnostic> {
        if target_kind.accepts(signal.kind) {
            Ok(())
        } else {
            Err(SimControlSignalDiagnostic {
                code: SIM_CONTROL_SIGNAL_TYPE_MISMATCH_CODE.to_string(),
                signal_kind: Some(signal.kind),
                target_kind: Some(target_kind),
                message: format!(
                    "{:?} output cannot feed {:?} generation control input",
                    signal.kind, target_kind
                ),
            })
        }
    }

    pub fn backend_diagnostic(
        &self,
        kind: SimControlSignalKind,
        status: SimControlSignalBackendStatus,
        reason: impl Into<String>,
    ) -> Option<SimControlSignalDiagnostic> {
        let reason = reason.into();
        match status {
            SimControlSignalBackendStatus::Native => None,
            SimControlSignalBackendStatus::DependencyReviewRequired => {
                Some(SimControlSignalDiagnostic {
                    code: SIM_CONTROL_SIGNAL_DEPENDENCY_REVIEW_REQUIRED_CODE.to_string(),
                    signal_kind: Some(kind),
                    target_kind: None,
                    message: format!("{reason} requires dependency review before native execution"),
                })
            }
            SimControlSignalBackendStatus::Unsupported => Some(SimControlSignalDiagnostic {
                code: SIM_CONTROL_SIGNAL_UNSUPPORTED_BACKEND_CODE.to_string(),
                signal_kind: Some(kind),
                target_kind: None,
                message: format!("{reason} does not have a native Sim analysis backend yet"),
            }),
        }
    }
}

fn validate_metadata(
    kind: SimControlSignalKind,
    metadata: &SimControlSignalMetadata,
) -> Result<(), SimControlSignalDiagnostic> {
    if metadata.source_reference.trim().is_empty() || metadata.width == 0 || metadata.height == 0 {
        Err(SimControlSignalDiagnostic {
            code: SIM_CONTROL_SIGNAL_MISSING_METADATA_CODE.to_string(),
            signal_kind: Some(kind),
            target_kind: None,
            message: "control signal outputs require source media reference, width, and height"
                .to_string(),
        })
    } else {
        Ok(())
    }
}
