use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ComfyLatentRuntime, LatentArtifact, LatentMediaKind, ModelFamilyExecutionProfile,
    ModelMediaCapability,
};

pub const SIM_AUDIO_INVALID_RANGE_CODE: &str = "world_model.audio_nodes.invalid_range";
pub const SIM_AUDIO_INVALID_CHANNEL_CODE: &str = "world_model.audio_nodes.invalid_channel";
pub const SIM_AUDIO_SHAPE_MISMATCH_CODE: &str = "world_model.audio_nodes.shape_mismatch";
pub const SIM_AUDIO_DEPENDENCY_REVIEW_REQUIRED_CODE: &str =
    "world_model.audio_nodes.dependency_review_required";
pub const SIM_AUDIO_UNSUPPORTED_CODEC_CODE: &str = "world_model.audio_nodes.unsupported_codec";
pub const SIM_AUDIO_LATENT_MISMATCH_CODE: &str = "world_model.audio_nodes.latent_mismatch";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAudioMetadata {
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_samples: u64,
    pub mime_type: String,
    pub fields: BTreeMap<String, String>,
}

impl SimAudioMetadata {
    pub fn new(sample_rate: u32, channels: u16, duration_samples: u64) -> Self {
        Self {
            sample_rate,
            channels,
            duration_samples,
            mime_type: "audio/wav".to_string(),
            fields: BTreeMap::new(),
        }
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
pub struct SimAudioArtifact {
    pub reference: String,
    pub metadata: SimAudioMetadata,
}

impl SimAudioArtifact {
    pub fn new(reference: impl Into<String>, metadata: SimAudioMetadata) -> Self {
        Self {
            reference: reference.into(),
            metadata,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAudioSampleRange {
    pub start: u64,
    pub end_exclusive: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimAudioEqualizationBand {
    pub center_hz: f32,
    pub gain_db: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimAudioOperation {
    Load,
    Save,
    Preview,
    Record,
    Trim,
    SplitChannels,
    JoinChannels,
    Concatenate,
    Mix,
    Volume,
    Equalize,
    Empty,
    EncodeLatent,
    DecodeLatent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimAudioCodecStatus {
    Native,
    DependencyReviewRequired,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimAudioNodeDiagnostic {
    pub code: String,
    pub operation: Option<SimAudioOperation>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimAudioNodeAdapter;

impl SimAudioNodeAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn load(
        &self,
        reference: impl Into<String>,
        metadata: SimAudioMetadata,
    ) -> SimAudioArtifact {
        SimAudioArtifact::new(reference, metadata.with_field("sim.operation", "load"))
    }

    pub fn empty(
        &self,
        reference: impl Into<String>,
        sample_rate: u32,
        channels: u16,
        duration_samples: u64,
    ) -> SimAudioArtifact {
        SimAudioArtifact::new(
            reference,
            SimAudioMetadata::new(sample_rate, channels, duration_samples)
                .with_field("sim.operation", "empty"),
        )
    }

    pub fn record(
        &self,
        reference: impl Into<String>,
        sample_rate: u32,
        channels: u16,
        duration_samples: u64,
    ) -> SimAudioArtifact {
        SimAudioArtifact::new(
            reference,
            SimAudioMetadata::new(sample_rate, channels, duration_samples)
                .with_field("sim.operation", "record"),
        )
    }

    pub fn save_as(
        &self,
        artifact: &SimAudioArtifact,
        reference: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> SimAudioArtifact {
        let mut artifact = artifact.clone();
        artifact.reference = reference.into();
        artifact.metadata.mime_type = mime_type.into();
        artifact
            .metadata
            .fields
            .insert("sim.operation".to_string(), "save".to_string());
        artifact
    }

    pub fn preview(&self, artifact: &SimAudioArtifact) -> SimAudioArtifact {
        let mut artifact = artifact.clone();
        artifact
            .metadata
            .fields
            .insert("sim.operation".to_string(), "preview".to_string());
        artifact
    }

    pub fn trim(
        &self,
        artifact: &SimAudioArtifact,
        range: SimAudioSampleRange,
    ) -> Result<SimAudioArtifact, SimAudioNodeDiagnostic> {
        validate_range(artifact.metadata.duration_samples, range)?;
        let mut artifact = artifact.clone();
        artifact.metadata.duration_samples = range.end_exclusive - range.start;
        artifact.metadata.fields.insert(
            "sim.sample_range".to_string(),
            format!("{}..{}", range.start, range.end_exclusive),
        );
        artifact
            .metadata
            .fields
            .insert("sim.operation".to_string(), "trim".to_string());
        Ok(artifact)
    }

    pub fn split_channel(
        &self,
        artifact: &SimAudioArtifact,
        channel_index: u16,
    ) -> Result<SimAudioArtifact, SimAudioNodeDiagnostic> {
        if channel_index >= artifact.metadata.channels {
            return Err(diagnostic(
                SIM_AUDIO_INVALID_CHANNEL_CODE,
                SimAudioOperation::SplitChannels,
                "audio channel index must stay inside the source channel count",
            ));
        }

        let mut artifact = artifact.clone();
        artifact.metadata.channels = 1;
        artifact
            .metadata
            .fields
            .insert("sim.channel".to_string(), channel_index.to_string());
        artifact
            .metadata
            .fields
            .insert("sim.operation".to_string(), "split_channels".to_string());
        Ok(artifact)
    }

    pub fn join_channels(
        &self,
        reference: impl Into<String>,
        channels: &[SimAudioArtifact],
    ) -> Result<SimAudioArtifact, SimAudioNodeDiagnostic> {
        let first = channels.first().ok_or_else(|| {
            diagnostic(
                SIM_AUDIO_SHAPE_MISMATCH_CODE,
                SimAudioOperation::JoinChannels,
                "joining audio requires at least one source channel",
            )
        })?;

        if channels.iter().any(|channel| {
            channel.metadata.channels != 1
                || channel.metadata.sample_rate != first.metadata.sample_rate
                || channel.metadata.duration_samples != first.metadata.duration_samples
        }) {
            return Err(diagnostic(
                SIM_AUDIO_SHAPE_MISMATCH_CODE,
                SimAudioOperation::JoinChannels,
                "joined channels must be mono sources with matching sample rate and duration",
            ));
        }

        Ok(SimAudioArtifact::new(
            reference,
            SimAudioMetadata::new(
                first.metadata.sample_rate,
                channels.len() as u16,
                first.metadata.duration_samples,
            )
            .with_mime_type(first.metadata.mime_type.clone())
            .with_field("sim.operation", "join_channels"),
        ))
    }

    pub fn concatenate(
        &self,
        reference: impl Into<String>,
        clips: &[SimAudioArtifact],
    ) -> Result<SimAudioArtifact, SimAudioNodeDiagnostic> {
        let first = clips.first().ok_or_else(|| {
            diagnostic(
                SIM_AUDIO_SHAPE_MISMATCH_CODE,
                SimAudioOperation::Concatenate,
                "concatenating audio requires at least one source clip",
            )
        })?;

        if clips.iter().any(|clip| {
            clip.metadata.channels != first.metadata.channels
                || clip.metadata.sample_rate != first.metadata.sample_rate
        }) {
            return Err(diagnostic(
                SIM_AUDIO_SHAPE_MISMATCH_CODE,
                SimAudioOperation::Concatenate,
                "concatenated clips must share sample rate and channel count",
            ));
        }

        Ok(SimAudioArtifact::new(
            reference,
            SimAudioMetadata::new(
                first.metadata.sample_rate,
                first.metadata.channels,
                clips
                    .iter()
                    .map(|clip| clip.metadata.duration_samples)
                    .sum(),
            )
            .with_mime_type(first.metadata.mime_type.clone())
            .with_field("sim.operation", "concatenate"),
        ))
    }

    pub fn mix(
        &self,
        reference: impl Into<String>,
        clips: &[SimAudioArtifact],
    ) -> Result<SimAudioArtifact, SimAudioNodeDiagnostic> {
        let first = clips.first().ok_or_else(|| {
            diagnostic(
                SIM_AUDIO_SHAPE_MISMATCH_CODE,
                SimAudioOperation::Mix,
                "mixing audio requires at least one source clip",
            )
        })?;

        if clips.iter().any(|clip| {
            clip.metadata.channels != first.metadata.channels
                || clip.metadata.sample_rate != first.metadata.sample_rate
        }) {
            return Err(diagnostic(
                SIM_AUDIO_SHAPE_MISMATCH_CODE,
                SimAudioOperation::Mix,
                "mixed clips must share sample rate and channel count",
            ));
        }

        let duration_samples = clips
            .iter()
            .map(|clip| clip.metadata.duration_samples)
            .max()
            .unwrap_or(first.metadata.duration_samples);

        Ok(SimAudioArtifact::new(
            reference,
            SimAudioMetadata::new(
                first.metadata.sample_rate,
                first.metadata.channels,
                duration_samples,
            )
            .with_mime_type(first.metadata.mime_type.clone())
            .with_field("sim.operation", "mix"),
        ))
    }

    pub fn adjust_volume(&self, artifact: &SimAudioArtifact, gain_db: f32) -> SimAudioArtifact {
        let mut artifact = artifact.clone();
        artifact
            .metadata
            .fields
            .insert("sim.operation".to_string(), "volume".to_string());
        artifact
            .metadata
            .fields
            .insert("sim.gain_db".to_string(), gain_db.to_string());
        artifact
    }

    pub fn equalize(
        &self,
        artifact: &SimAudioArtifact,
        bands: &[SimAudioEqualizationBand],
    ) -> SimAudioArtifact {
        let mut artifact = artifact.clone();
        artifact
            .metadata
            .fields
            .insert("sim.operation".to_string(), "equalize".to_string());
        artifact
            .metadata
            .fields
            .insert("sim.eq_bands".to_string(), bands.len().to_string());
        artifact
    }

    pub fn codec_diagnostic(
        &self,
        operation: SimAudioOperation,
        status: SimAudioCodecStatus,
        codec: impl Into<String>,
    ) -> Option<SimAudioNodeDiagnostic> {
        let codec = codec.into();
        match status {
            SimAudioCodecStatus::Native => None,
            SimAudioCodecStatus::DependencyReviewRequired => Some(SimAudioNodeDiagnostic {
                code: SIM_AUDIO_DEPENDENCY_REVIEW_REQUIRED_CODE.to_string(),
                operation: Some(operation),
                message: format!("{codec} requires dependency review before native execution"),
            }),
            SimAudioCodecStatus::Unsupported => Some(SimAudioNodeDiagnostic {
                code: SIM_AUDIO_UNSUPPORTED_CODEC_CODE.to_string(),
                operation: Some(operation),
                message: format!("{codec} does not have a native Sim codec backend yet"),
            }),
        }
    }

    pub fn validate_audio_latent(
        &self,
        latent: &LatentArtifact,
        family_profile: &ModelFamilyExecutionProfile,
    ) -> Result<(), Vec<SimAudioNodeDiagnostic>> {
        let mut diagnostics = Vec::new();

        if latent.media != LatentMediaKind::Audio {
            diagnostics.push(diagnostic(
                SIM_AUDIO_LATENT_MISMATCH_CODE,
                SimAudioOperation::EncodeLatent,
                "audio latent nodes require audio media latents",
            ));
        }
        if !family_profile.media.contains(&ModelMediaCapability::Audio) {
            diagnostics.push(diagnostic(
                SIM_AUDIO_LATENT_MISMATCH_CODE,
                SimAudioOperation::EncodeLatent,
                "model family must advertise native audio media capability",
            ));
        }

        if let Err(latent_diagnostics) = ComfyLatentRuntime::new().validate(latent, family_profile)
        {
            diagnostics.extend(latent_diagnostics.into_iter().map(|diagnostic| {
                SimAudioNodeDiagnostic {
                    code: diagnostic.code,
                    operation: Some(SimAudioOperation::EncodeLatent),
                    message: diagnostic.message,
                }
            }));
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }
}

fn validate_range(
    duration_samples: u64,
    range: SimAudioSampleRange,
) -> Result<(), SimAudioNodeDiagnostic> {
    if range.start >= range.end_exclusive || range.end_exclusive > duration_samples {
        Err(diagnostic(
            SIM_AUDIO_INVALID_RANGE_CODE,
            SimAudioOperation::Trim,
            "audio sample range must be non-empty and stay inside the source clip",
        ))
    } else {
        Ok(())
    }
}

fn diagnostic(
    code: &str,
    operation: SimAudioOperation,
    message: impl Into<String>,
) -> SimAudioNodeDiagnostic {
    SimAudioNodeDiagnostic {
        code: code.to_string(),
        operation: Some(operation),
        message: message.into(),
    }
}
