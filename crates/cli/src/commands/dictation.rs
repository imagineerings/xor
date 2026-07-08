use std::{str::FromStr as _, time::Duration};

use anyhow::{Context as _, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use dictation::{
    CloudDictationConfig, CloudDictationProvider, DictationService, WhisperConfig,
    WhisperLocalProvider, WhisperModel,
};

#[derive(Parser, Debug)]
#[command(name = "dictation", about = "Transcribe microphone audio")]
struct DictationArgs {
    #[command(subcommand)]
    command: DictationCommand,
}

#[derive(Subcommand, Debug)]
enum DictationCommand {
    /// Capture microphone audio and print the transcription.
    Transcribe {
        /// Provider to use: `whisper` or `cloud:<name>`.
        #[arg(long, default_value = "whisper")]
        provider: String,
        /// Maximum microphone capture duration in seconds.
        #[arg(long, default_value_t = 5)]
        duration_seconds: u64,
        /// Whisper model name: tiny, base, small, medium, or large.
        #[arg(long)]
        model: Option<String>,
        /// Optional provider language code.
        #[arg(long)]
        language: Option<String>,
        /// Cloud provider API URL. Defaults to the provider's built-in endpoint when omitted.
        #[arg(long)]
        cloud_api_url: Option<String>,
        /// Environment variable containing the cloud provider API key.
        #[arg(long)]
        cloud_api_key_env: Option<String>,
        /// JSON response field path for extracting cloud transcription text.
        #[arg(long)]
        cloud_response_field: Option<String>,
        /// Stop early after this many seconds of silence.
        #[arg(long)]
        auto_stop_silence_seconds: Option<u64>,
        /// RMS threshold used with auto-stop silence detection.
        #[arg(long)]
        silence_threshold: Option<f32>,
    },
}

pub fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let args = DictationArgs::try_parse_from(args)?;

    match args.command {
        DictationCommand::Transcribe {
            provider,
            duration_seconds,
            model,
            language,
            cloud_api_url,
            cloud_api_key_env,
            cloud_response_field,
            auto_stop_silence_seconds,
            silence_threshold,
        } => {
            let request = TranscribeRequest {
                provider,
                duration_seconds,
                model,
                language,
                cloud_api_url,
                cloud_api_key_env,
                cloud_response_field,
                auto_stop_silence_seconds,
                silence_threshold,
            };
            let text = smol::block_on(transcribe(request))?;
            println!("{text}");
            Ok(())
        }
    }
}

struct TranscribeRequest {
    provider: String,
    duration_seconds: u64,
    model: Option<String>,
    language: Option<String>,
    cloud_api_url: Option<String>,
    cloud_api_key_env: Option<String>,
    cloud_response_field: Option<String>,
    auto_stop_silence_seconds: Option<u64>,
    silence_threshold: Option<f32>,
}

async fn transcribe(request: TranscribeRequest) -> Result<String> {
    let duration = Duration::from_secs(request.duration_seconds.max(1));
    let service = service_from_request(&request)?;
    let text = if let Some(silence_seconds) = request.auto_stop_silence_seconds {
        service
            .transcribe_from_mic_with_auto_stop(
                duration,
                request.silence_threshold.unwrap_or(0.015),
                Duration::from_secs(silence_seconds.max(1)),
            )
            .await?
    } else {
        service.transcribe_from_mic(duration).await?
    };

    if text.trim().is_empty() {
        bail!("dictation provider returned an empty transcription");
    }

    Ok(text)
}

fn service_from_request(request: &TranscribeRequest) -> Result<DictationService> {
    if request.provider == "whisper" {
        let model = request
            .model
            .as_deref()
            .map(WhisperModel::from_str)
            .transpose()?
            .unwrap_or(WhisperModel::Base);
        let mut config = WhisperConfig::new(model, paths::data_dir().join("whisper"));
        if let Some(language) = request.language.clone() {
            config = config.with_language(language);
        }
        return Ok(DictationService::new(Box::new(WhisperLocalProvider::new(
            config,
        ))));
    }

    let provider_name = request
        .provider
        .strip_prefix("cloud:")
        .ok_or_else(|| anyhow!("provider must be `whisper` or `cloud:<name>`"))?;
    let api_key = match request.cloud_api_key_env.as_deref() {
        Some(variable) => Some(
            std::env::var(variable)
                .with_context(|| format!("environment variable `{variable}` is not set"))?,
        ),
        None => None,
    };
    let provider = CloudDictationProvider::new(CloudDictationConfig {
        provider: provider_name.to_string(),
        api_url: request.cloud_api_url.clone(),
        api_key,
        api_key_header: None,
        api_key_scheme: None,
        language: request.language.clone(),
        response_field: request.cloud_response_field.clone(),
    });
    Ok(DictationService::new(Box::new(provider)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(provider: &str) -> TranscribeRequest {
        TranscribeRequest {
            provider: provider.to_string(),
            duration_seconds: 1,
            model: None,
            language: None,
            cloud_api_url: None,
            cloud_api_key_env: None,
            cloud_response_field: None,
            auto_stop_silence_seconds: None,
            silence_threshold: None,
        }
    }

    #[test]
    fn rejects_unknown_provider() {
        match service_from_request(&request("unknown")) {
            Ok(_) => unreachable!("expected unknown provider to be rejected"),
            Err(error) => {
                assert!(
                    error
                        .to_string()
                        .contains("provider must be `whisper` or `cloud:<name>`")
                );
            }
        }
    }

    #[test]
    fn builds_whisper_service() {
        let service = service_from_request(&request("whisper")).unwrap();
        assert_eq!(service.provider().name(), "whisper");
    }
}
