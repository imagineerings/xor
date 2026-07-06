use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use audio::{
    ActiveCaptureConfig, CaptureConfig, CaptureSampleFormat, CapturedAudio, MicrophoneCapture,
};

use crate::{
    AudioFormat, DictationConfig, DictationError, DictationProvider, WhisperLocalProvider,
};

/// Orchestrates the microphone capture → provider → text pipeline.
///
/// Holds a `DictationProvider` and optional `CaptureConfig` to coordinate
/// audio capture from the microphone and speech-to-text transcription.
pub struct DictationService {
    provider: Box<dyn DictationProvider>,
    capture_config: CaptureConfig,
}

impl DictationService {
    /// Creates a new `DictationService` with the given provider.
    ///
    /// The microphone capture uses default settings (device-preferred sample
    /// rate, channels, and format). Call [`with_capture_config`](Self::with_capture_config)
    /// to customize capture parameters.
    pub fn new(provider: Box<dyn DictationProvider>) -> Self {
        Self {
            provider,
            capture_config: CaptureConfig::default(),
        }
    }

    /// Creates a `DictationService` from a [`DictationConfig`].
    ///
    /// Selects the provider based on the configuration:
    /// - `Whisper`: uses the built-in [`WhisperLocalProvider`].
    /// - `Cloud(name)`: uses a [`CloudDictationProvider`] if the `cloud_provider`
    ///    field is set in the config.
    /// - `Custom(_)`: reserved for future custom provider registration.
    pub fn from_config(config: &DictationConfig, capture_config: CaptureConfig) -> Result<Self> {
        let provider = provider_from_config(config)?;
        Ok(Self {
            provider,
            capture_config,
        })
    }

    /// Overrides the microphone capture configuration.
    pub fn with_capture_config(mut self, config: CaptureConfig) -> Self {
        self.capture_config = config;
        self
    }

    /// Returns a reference to the underlying provider.
    pub fn provider(&self) -> &dyn DictationProvider {
        self.provider.as_ref()
    }

    /// Transcribes pre-recorded audio using the configured provider.
    ///
    /// The audio data must be in a format supported by the provider (see
    /// [`DictationProvider::supported_formats`]).
    pub async fn transcribe_audio(&self, audio: &[u8], format: AudioFormat) -> Result<String> {
        self.provider.transcribe(audio, format).await
    }

    /// Captures audio from the default microphone for the given duration and
    /// transcribes it.
    ///
    /// Returns `Ok(text)` on success, or a [`DictationError`] if the capture
    /// or transcription fails.
    pub async fn transcribe_from_mic(&self, duration: Duration) -> Result<String> {
        let audio = Self::capture(duration, &self.capture_config)?;
        let (bytes, format) = captured_audio_to_raw_pcm(&audio)?;
        self.provider.transcribe(&bytes, format).await
    }

    /// Captures audio from the default microphone with automatic silence
    /// detection.
    ///
    /// Recording stops when:
    /// - The audio level stays below `silence_threshold` for `silence_duration`.
    /// - The `max_duration` is reached.
    ///
    /// The `silence_threshold` is an RMS amplitude value (0.0 = silence,
    /// 1.0 = maximum). A reasonable default for typical microphone input is
    /// around 0.01–0.02.
    pub async fn transcribe_from_mic_with_auto_stop(
        &self,
        max_duration: Duration,
        silence_threshold: f32,
        silence_duration: Duration,
    ) -> Result<String> {
        let audio = Self::capture_with_auto_stop(
            max_duration,
            silence_threshold,
            silence_duration,
            &self.capture_config,
        )?;
        let (bytes, format) = captured_audio_to_raw_pcm(&audio)?;
        self.provider.transcribe(&bytes, format).await
    }

    /// Captures audio from the default microphone.
    fn capture(duration: Duration, capture_config: &CaptureConfig) -> Result<CapturedAudio> {
        let stream = MicrophoneCapture::start_capture(None, capture_config.clone())
            .context("failed to start microphone capture")?;
        std::thread::sleep(duration);
        stream
            .stop_capture()
            .context("failed to stop microphone capture")
    }

    /// Captures audio with silence-based auto-stop.
    fn capture_with_auto_stop(
        max_duration: Duration,
        silence_threshold: f32,
        silence_duration: Duration,
        capture_config: &CaptureConfig,
    ) -> Result<CapturedAudio> {
        let stream = MicrophoneCapture::start_capture(None, capture_config.clone())
            .context("failed to start microphone capture")?;
        let start = Instant::now();
        let silence_start = Instant::now();
        let mut all_samples: Vec<i16> = Vec::new();
        let mut last_silence_start = silence_start;

        loop {
            let elapsed = start.elapsed();
            if elapsed >= max_duration {
                break;
            }

            let chunk = stream
                .read_available()
                .context("failed to read audio from microphone")?;
            let rms = rms_i16(&chunk.samples);
            all_samples.extend(chunk.samples);

            if rms < silence_threshold {
                if last_silence_start.elapsed() >= silence_duration {
                    break;
                }
            } else {
                last_silence_start = Instant::now();
            }
        }

        drop(stream);

        let active_config = ActiveCaptureConfig {
            sample_rate: capture_config
                .sample_rate
                .unwrap_or(DEFAULT_CAPTURE_CONFIG.sample_rate),
            channels: capture_config
                .channels
                .unwrap_or(DEFAULT_CAPTURE_CONFIG.channels),
            sample_format: capture_config
                .sample_format
                .unwrap_or(CaptureSampleFormat::I16),
        };

        Ok(CapturedAudio {
            samples: all_samples,
            config: active_config,
        })
    }
}

/// Default active capture configuration used when no explicit config is given.
const DEFAULT_CAPTURE_CONFIG: ActiveCaptureConfig = ActiveCaptureConfig {
    sample_rate: 48_000,
    channels: 1,
    sample_format: CaptureSampleFormat::I16,
};

/// Builds a `DictationProvider` from a `DictationConfig`.
fn provider_from_config(config: &DictationConfig) -> Result<Box<dyn DictationProvider>> {
    match &config.provider {
        crate::DictationProviderName::Whisper => {
            let whisper_config = crate::WhisperConfig {
                model: config
                    .model
                    .as_ref()
                    .and_then(|m| m.parse().ok())
                    .unwrap_or(crate::WhisperModel::Base),
                language: config.language.clone(),
                cache_dir: std::path::PathBuf::from(
                    std::env::var("WHISPER_CACHE_DIR")
                        .unwrap_or_else(|_| "~/.cache/whisper".to_string()),
                ),
            };
            Ok(Box::new(WhisperLocalProvider::new(whisper_config)))
        }
        crate::DictationProviderName::Cloud(provider_name)
        | crate::DictationProviderName::Custom(provider_name) => {
            let cloud_config = config.cloud_provider.as_ref().ok_or_else(|| {
                DictationError::ProviderUnavailable {
                    provider: provider_name.clone(),
                    message: "cloud_provider configuration is not set".to_string(),
                }
            })?;
            let provider_config = crate::CloudDictationConfig {
                provider: cloud_config.provider.clone(),
                api_url: cloud_config.api_url.clone(),
                api_key: cloud_config.api_key.clone(),
                api_key_header: cloud_config.api_key_header.clone(),
                api_key_scheme: cloud_config.api_key_scheme.clone(),
                language: config.language.clone(),
                response_field: cloud_config.response_field.clone(),
            };
            Ok(Box::new(crate::CloudDictationProvider::new(
                provider_config,
            )))
        }
    }
}

/// Converts `CapturedAudio` (i16 samples) to a `RawPcm` byte buffer.
fn captured_audio_to_raw_pcm(captured: &CapturedAudio) -> Result<(Vec<u8>, AudioFormat)> {
    let sample_rate = captured.config.sample_rate;
    let channels = captured.config.channels;
    let bits_per_sample = 16u8;

    let mut bytes = Vec::with_capacity(captured.samples.len() * 2);
    for sample in &captured.samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }

    let format = AudioFormat::RawPcm {
        sample_rate,
        channels,
        bits_per_sample,
    };

    Ok((bytes, format))
}

/// Computes the Root Mean Square (RMS) amplitude of i16 audio samples.
///
/// Returns a value in `[0.0, 1.0]` where 0.0 is silence and 1.0 is maximum
/// amplitude. This is used for silence detection.
fn rms_i16(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_squares: f64 = samples
        .iter()
        .map(|s| {
            let normalized = f64::from(*s) / f64::from(i16::MAX);
            normalized * normalized
        })
        .sum();

    (sum_squares / samples.len() as f64).sqrt() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_of_silence_is_zero() {
        let samples = vec![0_i16; 100];
        let rms = rms_i16(&samples);
        assert!(rms < 1e-6);
    }

    #[test]
    fn rms_of_max_amplitude_is_one() {
        let samples = vec![i16::MAX; 100];
        let rms = rms_i16(&samples);
        assert!((rms - 1.0).abs() < 0.01);
    }

    #[test]
    fn rms_of_mixed_samples() {
        let samples = vec![i16::MAX, 0, i16::MIN, 0];
        let rms = rms_i16(&samples);
        // RMS of normalized [1.0, 0.0, -1.0, 0.0] = sqrt(0.5) ≈ 0.707
        assert!((rms - 0.707).abs() < 0.01);
    }

    #[test]
    fn rms_of_empty_slice_is_zero() {
        assert_eq!(rms_i16(&[]), 0.0);
    }

    #[test]
    fn captured_audio_to_raw_pcm_produces_correct_format() {
        let captured = CapturedAudio {
            samples: vec![0_i16, 1000, -1000, i16::MAX, i16::MIN],
            config: ActiveCaptureConfig {
                sample_rate: 48_000,
                channels: 1,
                sample_format: CaptureSampleFormat::I16,
            },
        };

        let (bytes, format) = captured_audio_to_raw_pcm(&captured).unwrap();

        assert_eq!(bytes.len(), captured.samples.len() * 2);
        assert_eq!(
            format,
            AudioFormat::RawPcm {
                sample_rate: 48_000,
                channels: 1,
                bits_per_sample: 16,
            }
        );

        // Verify the first sample round-trips correctly
        let first = i16::from_le_bytes([bytes[0], bytes[1]]);
        assert_eq!(first, 0);
    }

    #[test]
    fn provider_from_config_whisper_default() {
        let config = DictationConfig::default();
        let provider = provider_from_config(&config);

        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().name(), "whisper");
    }

    #[test]
    fn provider_from_config_cloud_without_cloud_config_errors() {
        let config = DictationConfig {
            provider: crate::DictationProviderName::Cloud("deepgram".into()),
            cloud_provider: None,
            ..Default::default()
        };
        let provider = provider_from_config(&config);

        assert!(provider.is_err());
        let error = provider.unwrap_err().to_string();
        assert!(error.contains("cloud_provider configuration is not set"));
    }

    #[test]
    fn provider_from_config_custom_without_cloud_config_errors() {
        let config = DictationConfig {
            provider: crate::DictationProviderName::Custom("my-provider".into()),
            cloud_provider: None,
            ..Default::default()
        };
        let provider = provider_from_config(&config);

        assert!(provider.is_err());
        let error = provider.unwrap_err().to_string();
        assert!(error.contains("cloud_provider configuration is not set"));
    }
}
