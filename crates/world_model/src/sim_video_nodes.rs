use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SIM_VIDEO_INVALID_RANGE_CODE: &str = "world_model.video_nodes.invalid_range";
pub const SIM_VIDEO_DEPENDENCY_REVIEW_REQUIRED_CODE: &str =
    "world_model.video_nodes.dependency_review_required";
pub const SIM_VIDEO_UNSUPPORTED_BACKEND_CODE: &str = "world_model.video_nodes.unsupported_backend";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimVideoMetadata {
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub frame_rate_num: u32,
    pub frame_rate_den: u32,
    pub mime_type: String,
    pub audio_reference: Option<String>,
    pub fields: BTreeMap<String, String>,
}

impl SimVideoMetadata {
    pub fn new(
        width: u32,
        height: u32,
        frames: u32,
        frame_rate_num: u32,
        frame_rate_den: u32,
    ) -> Self {
        Self {
            width,
            height,
            frames,
            frame_rate_num,
            frame_rate_den: frame_rate_den.max(1),
            mime_type: "video/mp4".to_string(),
            audio_reference: None,
            fields: BTreeMap::new(),
        }
    }

    pub fn with_audio_reference(mut self, audio_reference: impl Into<String>) -> Self {
        self.audio_reference = Some(audio_reference.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = mime_type.into();
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimVideoArtifact {
    pub reference: String,
    pub metadata: SimVideoMetadata,
}

impl SimVideoArtifact {
    pub fn new(reference: impl Into<String>, metadata: SimVideoMetadata) -> Self {
        Self {
            reference: reference.into(),
            metadata,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimVideoFrameRange {
    pub start: u32,
    pub end_exclusive: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimVideoFrameBatch {
    pub source_reference: String,
    pub range: SimVideoFrameRange,
    pub frame_count: u32,
    pub frame_rate_num: u32,
    pub frame_rate_den: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimVideoAdvancedOperation {
    FrameInterpolation,
    Stitch,
    Merge,
    Upscale,
    Inpaint,
    Caption,
    DepthEstimation,
    PoseExtraction,
    FaceDetection,
    Segmentation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimVideoBackendStatus {
    Native,
    DependencyReviewRequired,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimVideoNodeDiagnostic {
    pub code: String,
    pub operation: Option<SimVideoAdvancedOperation>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimVideoNodeAdapter;

impl SimVideoNodeAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn load(
        &self,
        reference: impl Into<String>,
        metadata: SimVideoMetadata,
    ) -> SimVideoArtifact {
        SimVideoArtifact::new(reference, metadata.with_field("sim.operation", "load"))
    }

    pub fn create(
        &self,
        reference: impl Into<String>,
        width: u32,
        height: u32,
        frames: u32,
        frame_rate_num: u32,
        frame_rate_den: u32,
    ) -> SimVideoArtifact {
        SimVideoArtifact::new(
            reference,
            SimVideoMetadata::new(width, height, frames, frame_rate_num, frame_rate_den)
                .with_field("sim.operation", "create"),
        )
    }

    pub fn save_as(
        &self,
        artifact: &SimVideoArtifact,
        reference: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> SimVideoArtifact {
        let mut artifact = artifact.clone();
        artifact.reference = reference.into();
        artifact.metadata.mime_type = mime_type.into();
        artifact
            .metadata
            .fields
            .insert("sim.operation".to_string(), "save".to_string());
        artifact
    }

    pub fn slice(
        &self,
        artifact: &SimVideoArtifact,
        range: SimVideoFrameRange,
    ) -> Result<SimVideoArtifact, SimVideoNodeDiagnostic> {
        validate_range(artifact.metadata.frames, range)?;
        let mut artifact = artifact.clone();
        artifact.metadata.frames = range.end_exclusive - range.start;
        artifact.metadata.fields.insert(
            "sim.frame_range".to_string(),
            format!("{}..{}", range.start, range.end_exclusive),
        );
        Ok(artifact)
    }

    pub fn decompose(
        &self,
        artifact: &SimVideoArtifact,
        range: SimVideoFrameRange,
    ) -> Result<SimVideoFrameBatch, SimVideoNodeDiagnostic> {
        validate_range(artifact.metadata.frames, range)?;
        Ok(SimVideoFrameBatch {
            source_reference: artifact.reference.clone(),
            range,
            frame_count: range.end_exclusive - range.start,
            frame_rate_num: artifact.metadata.frame_rate_num,
            frame_rate_den: artifact.metadata.frame_rate_den,
        })
    }

    pub fn backend_diagnostic(
        &self,
        operation: SimVideoAdvancedOperation,
        status: SimVideoBackendStatus,
    ) -> Option<SimVideoNodeDiagnostic> {
        match status {
            SimVideoBackendStatus::Native => None,
            SimVideoBackendStatus::DependencyReviewRequired => Some(SimVideoNodeDiagnostic {
                code: SIM_VIDEO_DEPENDENCY_REVIEW_REQUIRED_CODE.to_string(),
                operation: Some(operation),
                message: format!(
                    "{operation:?} requires dependency review before native execution"
                ),
            }),
            SimVideoBackendStatus::Unsupported => Some(SimVideoNodeDiagnostic {
                code: SIM_VIDEO_UNSUPPORTED_BACKEND_CODE.to_string(),
                operation: Some(operation),
                message: format!("{operation:?} does not have a native Sim backend yet"),
            }),
        }
    }
}

fn validate_range(frames: u32, range: SimVideoFrameRange) -> Result<(), SimVideoNodeDiagnostic> {
    if range.start >= range.end_exclusive || range.end_exclusive > frames {
        Err(SimVideoNodeDiagnostic {
            code: SIM_VIDEO_INVALID_RANGE_CODE.to_string(),
            operation: None,
            message: "video frame range must be non-empty and stay inside the source video"
                .to_string(),
        })
    } else {
        Ok(())
    }
}
