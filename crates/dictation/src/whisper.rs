use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Result, bail};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{AudioFormat, DictationError, DictationProvider};

pub const WHISPER_PROVIDER_NAME: &str = "whisper";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WhisperConfig {
    pub model: WhisperModel,
    pub language: Option<String>,
    pub cache_dir: PathBuf,
}

impl WhisperConfig {
    pub fn new(model: WhisperModel, cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            model,
            language: None,
            cache_dir: cache_dir.into(),
        }
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
}

impl FromStr for WhisperModel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "tiny" => Ok(Self::Tiny),
            "base" => Ok(Self::Base),
            "small" => Ok(Self::Small),
            "medium" => Ok(Self::Medium),
            "large" => Ok(Self::Large),
            other => bail!("unknown Whisper model: {other}"),
        }
    }
}

impl WhisperModel {
    pub fn name(self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Base => "base",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Tiny => "ggml-tiny.bin",
            Self::Base => "ggml-base.bin",
            Self::Small => "ggml-small.bin",
            Self::Medium => "ggml-medium.bin",
            Self::Large => "ggml-large.bin",
        }
    }

    pub fn download_url(self) -> &'static str {
        match self {
            Self::Tiny => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
            Self::Base => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
            Self::Small => {
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
            }
            Self::Medium => {
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"
            }
            Self::Large => {
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large.bin"
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhisperModelManager {
    cache_dir: PathBuf,
}

impl WhisperModelManager {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn model_path(&self, model: WhisperModel) -> PathBuf {
        self.cache_dir.join(model.file_name())
    }

    pub fn model_download(&self, model: WhisperModel) -> WhisperModelDownload {
        WhisperModelDownload {
            model,
            url: model.download_url().to_string(),
            path: self.model_path(model),
        }
    }

    pub fn is_model_cached(&self, model: WhisperModel) -> bool {
        self.model_path(model).is_file()
    }

    pub fn ensure_model_cached(&self, model: WhisperModel) -> Result<PathBuf> {
        let path = self.model_path(model);
        if path.is_file() {
            return Ok(path);
        }

        Err(DictationError::ProviderUnavailable {
            provider: WHISPER_PROVIDER_NAME.to_string(),
            message: format!(
                "Whisper model `{}` is not cached at {}. Download it from {} before transcribing offline.",
                model.name(),
                path.display(),
                model.download_url()
            ),
        }
        .into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhisperModelDownload {
    pub model: WhisperModel,
    pub url: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreprocessedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug)]
pub struct WhisperLocalProvider {
    config: WhisperConfig,
    model_manager: WhisperModelManager,
}

impl WhisperLocalProvider {
    pub fn new(config: WhisperConfig) -> Self {
        let model_manager = WhisperModelManager::new(config.cache_dir.clone());
        Self {
            config,
            model_manager,
        }
    }

    pub fn config(&self) -> &WhisperConfig {
        &self.config
    }

    pub fn model_manager(&self) -> &WhisperModelManager {
        &self.model_manager
    }

    pub fn preprocess_audio(
        &self,
        audio: &[u8],
        format: &AudioFormat,
    ) -> Result<PreprocessedAudio> {
        match format {
            AudioFormat::RawPcm {
                sample_rate,
                channels,
                bits_per_sample,
            } => preprocess_raw_pcm(audio, *sample_rate, *channels, *bits_per_sample),
            unsupported => Err(DictationError::UnsupportedFormat {
                provider: WHISPER_PROVIDER_NAME.to_string(),
                format: unsupported.clone(),
            }
            .into()),
        }
    }
}

#[async_trait]
impl DictationProvider for WhisperLocalProvider {
    fn name(&self) -> &str {
        WHISPER_PROVIDER_NAME
    }

    async fn transcribe(&self, audio: &[u8], format: AudioFormat) -> Result<String> {
        self.model_manager.ensure_model_cached(self.config.model)?;
        let _audio = self.preprocess_audio(audio, &format)?;

        Err(DictationError::ProviderUnavailable {
            provider: WHISPER_PROVIDER_NAME.to_string(),
            message: "Whisper inference runtime is not wired yet".to_string(),
        }
        .into())
    }

    fn supported_formats(&self) -> Vec<AudioFormat> {
        vec![AudioFormat::RawPcm {
            sample_rate: 16_000,
            channels: 1,
            bits_per_sample: 16,
        }]
    }

    fn requires_network(&self) -> bool {
        false
    }
}

pub fn preprocess_raw_pcm(
    audio: &[u8],
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u8,
) -> Result<PreprocessedAudio> {
    if channels == 0 {
        bail!("raw PCM audio must have at least one channel");
    }

    let samples = match bits_per_sample {
        16 => pcm_i16_le_to_mono_f32(audio, channels)?,
        unsupported => bail!("unsupported raw PCM bit depth: {unsupported}"),
    };

    let samples = if sample_rate == 16_000 {
        samples
    } else {
        resample_linear(&samples, sample_rate, 16_000)
    };

    Ok(PreprocessedAudio {
        samples,
        sample_rate: 16_000,
        channels: 1,
    })
}

fn pcm_i16_le_to_mono_f32(audio: &[u8], channels: u16) -> Result<Vec<f32>> {
    if !audio.len().is_multiple_of(2) {
        bail!("raw PCM i16 audio must contain an even number of bytes");
    }

    let channels = usize::from(channels);
    let mut mono_samples = Vec::with_capacity(audio.len() / 2 / channels);
    for frame in audio.chunks_exact(2 * channels) {
        let mut frame_sum = 0.0;
        for channel_sample in frame.chunks_exact(2) {
            let sample = i16::from_le_bytes([channel_sample[0], channel_sample[1]]);
            frame_sum += f32::from(sample) / f32::from(i16::MAX);
        }
        mono_samples.push(frame_sum / channels as f32);
    }

    Ok(mono_samples)
}

fn resample_linear(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate == target_rate {
        return samples.to_vec();
    }

    let target_len =
        (samples.len() as u64 * u64::from(target_rate)).div_ceil(u64::from(source_rate));
    let mut resampled = Vec::with_capacity(target_len as usize);
    let ratio = source_rate as f32 / target_rate as f32;

    for target_index in 0..target_len {
        let source_position = target_index as f32 * ratio;
        let source_index = source_position.floor() as usize;
        let next_index = (source_index + 1).min(samples.len() - 1);
        let fraction = source_position - source_index as f32;
        let sample = samples[source_index] * (1.0 - fraction) + samples[next_index] * fraction;
        resampled.push(sample);
    }

    resampled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_manager_builds_cache_path_and_download_url() {
        let manager = WhisperModelManager::new("/tmp/whisper");
        let download = manager.model_download(WhisperModel::Tiny);

        assert_eq!(download.path, PathBuf::from("/tmp/whisper/ggml-tiny.bin"));
        assert!(download.url.ends_with("/ggml-tiny.bin"));
    }

    #[test]
    fn preprocess_raw_pcm_converts_stereo_i16_to_mono_f32() {
        let audio = [
            0_i16.to_le_bytes(),
            i16::MAX.to_le_bytes(),
            i16::MIN.to_le_bytes(),
            0_i16.to_le_bytes(),
        ]
        .concat();

        let preprocessed = preprocess_raw_pcm(&audio, 16_000, 2, 16).unwrap();

        assert_eq!(preprocessed.sample_rate, 16_000);
        assert_eq!(preprocessed.channels, 1);
        assert_eq!(preprocessed.samples.len(), 2);
        assert!(preprocessed.samples[0] > 0.49 && preprocessed.samples[0] <= 0.5);
        assert!(preprocessed.samples[1] < -0.5 && preprocessed.samples[1] > -0.51);
    }

    #[test]
    fn preprocess_raw_pcm_resamples_to_sixteen_kilohertz() {
        let audio = [
            0_i16.to_le_bytes(),
            1000_i16.to_le_bytes(),
            2000_i16.to_le_bytes(),
            3000_i16.to_le_bytes(),
        ]
        .concat();

        let preprocessed = preprocess_raw_pcm(&audio, 8_000, 1, 16).unwrap();

        assert_eq!(preprocessed.sample_rate, 16_000);
        assert_eq!(preprocessed.channels, 1);
        assert_eq!(preprocessed.samples.len(), 8);
    }

    #[test]
    fn provider_is_offline_and_supports_raw_pcm() {
        let provider = WhisperLocalProvider::new(WhisperConfig::new(WhisperModel::Base, "/tmp"));

        assert_eq!(provider.name(), WHISPER_PROVIDER_NAME);
        assert!(!provider.requires_network());
        assert_eq!(
            provider.supported_formats(),
            vec![AudioFormat::RawPcm {
                sample_rate: 16_000,
                channels: 1,
                bits_per_sample: 16,
            }]
        );
    }
}
