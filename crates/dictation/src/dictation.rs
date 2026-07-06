use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod whisper;
pub use whisper::{
    WhisperConfig, WhisperLocalProvider, WhisperModel, WhisperModelDownload, WhisperModelManager,
};

#[async_trait]
pub trait DictationProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn transcribe(&self, audio: &[u8], format: AudioFormat) -> Result<String>;
    fn supported_formats(&self) -> Vec<AudioFormat>;
    fn requires_network(&self) -> bool;
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DictationConfig {
    pub provider: DictationProviderName,
    pub model: Option<String>,
    pub language: Option<String>,
    pub silence_threshold: Option<f32>,
    pub auto_stop_silence: Option<Duration>,
    pub cloud_provider: Option<CloudDictationConfig>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum DictationProviderName {
    #[default]
    Whisper,
    Cloud(String),
    Custom(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CloudDictationConfig {
    pub provider: String,
    pub api_url: Option<String>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum AudioFormat {
    Wav,
    Mp3,
    OggOpus,
    Flac,
    RawPcm {
        sample_rate: u32,
        channels: u16,
        bits_per_sample: u8,
    },
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum DictationError {
    #[error("dictation provider `{provider}` does not support audio format {format:?}")]
    UnsupportedFormat {
        provider: String,
        format: AudioFormat,
    },
    #[error("dictation provider `{provider}` is unavailable: {message}")]
    ProviderUnavailable { provider: String, message: String },
    #[error("dictation provider `{provider}` failed: {message}")]
    ProviderFailed { provider: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictation_config_defaults_to_whisper_provider() {
        let config = DictationConfig::default();

        assert_eq!(config.provider, DictationProviderName::Whisper);
        assert_eq!(config.model, None);
        assert_eq!(config.cloud_provider, None);
    }

    #[test]
    fn raw_pcm_audio_format_carries_stream_parameters() {
        let format = AudioFormat::RawPcm {
            sample_rate: 16_000,
            channels: 1,
            bits_per_sample: 16,
        };

        assert_eq!(
            format,
            AudioFormat::RawPcm {
                sample_rate: 16_000,
                channels: 1,
                bits_per_sample: 16,
            }
        );
    }

    #[test]
    fn dictation_errors_include_provider_context() {
        let error = DictationError::ProviderUnavailable {
            provider: "cloud".to_string(),
            message: "missing API key".to_string(),
        };

        assert_eq!(
            error.to_string(),
            "dictation provider `cloud` is unavailable: missing API key"
        );
    }
}
