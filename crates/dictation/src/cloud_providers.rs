use anyhow::{Result, bail};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::{AudioFormat, CloudDictationConfig, DictationError, DictationProvider};

/// A generic HTTP-based cloud dictation provider.
///
/// Sends audio to a cloud STT API via HTTP POST and extracts the transcription
/// text from the JSON response. Supports multiple authentication schemes and
/// response formats to work with common providers (Deepgram, Google Cloud
/// Speech-to-Text, Azure Speech, etc.).
#[derive(Debug)]
pub struct CloudDictationProvider {
    name: String,
    api_url: String,
    api_key: Option<String>,
    /// The HTTP header used to send the API key (e.g. "Authorization").
    api_key_header: String,
    /// An optional prefix for the header value (e.g. "Bearer", "Token").
    /// When `Some`, the final header value is `"{prefix} {api_key}"`.
    /// When `None`, the raw `api_key` is used as the header value.
    api_key_scheme: Option<String>,
    client: Client,
    supported_formats: Vec<AudioFormat>,
    language: Option<String>,
    /// Optional JSON field path for extracting transcription text from the
    /// response body. Uses dot-separated keys and array indices, for example
    /// `"results.channels.0.alternatives.0.transcript"`.
    response_field: Option<String>,
}

impl CloudDictationProvider {
    const DEFAULT_RESPONSE_FIELD: &'static str = "results.channels.0.alternatives.0.transcript";

    /// Creates a new `CloudDictationProvider` with the given configuration.
    ///
    /// A `reqwest::Client` is created automatically with default settings.
    pub fn new(config: CloudDictationConfig) -> Self {
        Self::with_client(config, Client::new())
    }

    /// Creates a new `CloudDictationProvider` using an existing HTTP client.
    pub fn with_client(config: CloudDictationConfig, client: Client) -> Self {
        let api_url = config
            .api_url
            .unwrap_or_else(|| "https://api.deepgram.com/v1/listen".to_string());

        Self {
            name: config.provider.clone(),
            api_url,
            api_key: config.api_key,
            api_key_header: config
                .api_key_header
                .unwrap_or_else(|| "Authorization".into()),
            api_key_scheme: config.api_key_scheme,
            client,
            supported_formats: vec![
                AudioFormat::Wav,
                AudioFormat::Flac,
                AudioFormat::RawPcm {
                    sample_rate: 16_000,
                    channels: 1,
                    bits_per_sample: 16,
                },
            ],
            language: config.language,
            response_field: config
                .response_field
                .or(Some(Self::DEFAULT_RESPONSE_FIELD.to_string())),
        }
    }

    /// Sets the supported audio formats for this provider.
    pub fn with_supported_formats(mut self, formats: Vec<AudioFormat>) -> Self {
        self.supported_formats = formats;
        self
    }

    /// Sets the JSON response field path for transcription extraction.
    pub fn with_response_field(mut self, field: impl Into<String>) -> Self {
        self.response_field = Some(field.into());
        self
    }

    /// Resolves the Content-Type header value for a given audio format.
    fn content_type(format: &AudioFormat) -> &'static str {
        match format {
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::OggOpus => "audio/ogg",
            AudioFormat::Flac => "audio/flac",
            AudioFormat::RawPcm { .. } => "audio/l16",
        }
    }

    /// Extracts the transcription text from a JSON response body using the
    /// configured field path.
    fn extract_transcription(&self, body: &Value) -> Option<String> {
        let path = self.response_field.as_deref()?;
        let mut current = body;

        for segment in path.split('.') {
            if let Ok(index) = segment.parse::<usize>() {
                current = current.get(index)?;
            } else {
                current = current.get(segment)?;
            }
        }

        current.as_str().map(|s| s.to_string())
    }
}

#[async_trait]
impl DictationProvider for CloudDictationProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn transcribe(&self, audio: &[u8], format: AudioFormat) -> Result<String> {
        if !self.supported_formats.contains(&format) {
            return Err(DictationError::UnsupportedFormat {
                provider: self.name.clone(),
                format,
            }
            .into());
        }

        let content_type = Self::content_type(&format);
        let mut request = self
            .client
            .post(&self.api_url)
            .header("Content-Type", content_type);

        if let Some(ref api_key) = self.api_key {
            let header_value = match self.api_key_scheme {
                Some(ref scheme) => format!("{} {}", scheme, api_key),
                None => api_key.clone(),
            };
            request = request.header(&self.api_key_header, header_value);
        }

        if let Some(ref language) = self.language {
            request = request.query(&[("language", language)]);
        }

        let response = request.body(audio.to_vec()).send().await.map_err(|e| {
            DictationError::ProviderUnavailable {
                provider: self.name.clone(),
                message: format!("HTTP request failed: {}", e),
            }
        })?;

        let status = response.status();
        let body_bytes =
            response
                .bytes()
                .await
                .map_err(|e| DictationError::ProviderUnavailable {
                    provider: self.name.clone(),
                    message: format!("failed to read response body: {}", e),
                })?;
        let body: Value =
            serde_json::from_slice(&body_bytes).map_err(|e| DictationError::ProviderFailed {
                provider: self.name.clone(),
                message: format!("failed to parse response as JSON: {}", e),
            })?;

        if !status.is_success() {
            let error_message = body
                .get("error")
                .or_else(|| body.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(DictationError::ProviderFailed {
                provider: self.name.clone(),
                message: format!("API returned {}: {}", status.as_u16(), error_message),
            }
            .into());
        }

        let transcription =
            self.extract_transcription(&body)
                .ok_or_else(|| DictationError::ProviderFailed {
                    provider: self.name.clone(),
                    message: format!(
                        "no transcription text found in response at field `{}`",
                        self.response_field.as_deref().unwrap_or("(none)")
                    ),
                })?;

        if transcription.trim().is_empty() {
            bail!(DictationError::ProviderFailed {
                provider: self.name.clone(),
                message: "received empty transcription".to_string(),
            });
        }

        Ok(transcription)
    }

    fn supported_formats(&self) -> Vec<AudioFormat> {
        self.supported_formats.clone()
    }

    fn requires_network(&self) -> bool {
        true
    }
}

/// Pre-built configuration for Deepgram's dictation API.
pub mod deepgram {
    use crate::CloudDictationConfig;

    /// Creates a `CloudDictationConfig` for Deepgram's `/v1/listen` endpoint.
    ///
    /// Deepgram uses `Authorization: Token <api_key>` for authentication.
    pub fn config(api_key: impl Into<String>) -> CloudDictationConfig {
        CloudDictationConfig {
            provider: "deepgram".into(),
            api_url: Some("https://api.deepgram.com/v1/listen".into()),
            api_key: Some(api_key.into()),
            api_key_header: Some("Authorization".into()),
            api_key_scheme: Some("Token".into()),
            language: None,
            response_field: Some("results.channels.0.alternatives.0.transcript".into()),
        }
    }
}

/// Pre-built configuration for Google Cloud Speech-to-Text.
pub mod google {
    use crate::CloudDictationConfig;

    /// Creates a `CloudDictationConfig` for Google Cloud Speech-to-Text.
    ///
    /// Google uses the API key as a query parameter (`?key=<api_key>`).
    /// The response field is `results[0].alternatives[0].transcript`.
    pub fn config(api_key: impl Into<String>) -> CloudDictationConfig {
        CloudDictationConfig {
            provider: "google".into(),
            api_url: Some("https://speech.googleapis.com/v1/speech:recognize".into()),
            api_key: Some(api_key.into()),
            api_key_header: Some("X-Goog-Api-Key".into()),
            api_key_scheme: None,
            language: None,
            response_field: Some("results.0.alternatives.0.transcript".into()),
        }
    }
}

/// Pre-built configuration for Azure Speech-to-Text.
pub mod azure {
    use crate::CloudDictationConfig;

    /// Creates a `CloudDictationConfig` for Azure Speech Service.
    ///
    /// Azure uses `Ocp-Apim-Subscription-Key: <api_key>` for authentication.
    /// The region should be provided as part of the `api_url`, for example
    /// `https://<region>.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1`.
    pub fn config(api_key: impl Into<String>, region: impl Into<String>) -> CloudDictationConfig {
        CloudDictationConfig {
            provider: "azure".into(),
            api_url: Some(format!(
                "https://{}.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1",
                region.into()
            )),
            api_key: Some(api_key.into()),
            api_key_header: Some("Ocp-Apim-Subscription-Key".into()),
            api_key_scheme: None,
            language: None,
            response_field: Some("DisplayText".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn provider_name_matches_config() {
        let config = CloudDictationConfig {
            provider: "my-cloud-stt".into(),
            api_url: Some("https://example.com/stt".into()),
            api_key: Some("test-key".into()),
            api_key_header: None,
            api_key_scheme: None,
            language: None,
            response_field: None,
        };
        let provider = CloudDictationProvider::new(config);

        assert_eq!(provider.name(), "my-cloud-stt");
        assert!(provider.requires_network());
    }

    #[test]
    fn provider_supports_standard_formats() {
        let config = CloudDictationConfig {
            provider: "test".into(),
            api_url: None,
            api_key: None,
            api_key_header: None,
            api_key_scheme: None,
            language: None,
            response_field: None,
        };
        let provider = CloudDictationProvider::new(config);

        let formats = provider.supported_formats();
        assert!(formats.contains(&AudioFormat::Wav));
        assert!(formats.contains(&AudioFormat::Flac));
        assert!(formats.contains(&AudioFormat::RawPcm {
            sample_rate: 16_000,
            channels: 1,
            bits_per_sample: 16,
        }));
    }

    #[test]
    fn provider_rejects_unsupported_format() {
        let config = CloudDictationConfig {
            provider: "test".into(),
            api_url: None,
            api_key: None,
            api_key_header: None,
            api_key_scheme: None,
            language: None,
            response_field: None,
        };
        let provider = CloudDictationProvider::with_supported_formats(
            CloudDictationProvider::new(config),
            vec![AudioFormat::Wav],
        );

        assert!(provider.supported_formats().len() == 1);
        assert!(!provider.supported_formats().contains(&AudioFormat::Mp3));
    }

    #[test]
    fn content_type_resolves_correctly() {
        assert_eq!(
            CloudDictationProvider::content_type(&AudioFormat::Wav),
            "audio/wav"
        );
        assert_eq!(
            CloudDictationProvider::content_type(&AudioFormat::Mp3),
            "audio/mpeg"
        );
        assert_eq!(
            CloudDictationProvider::content_type(&AudioFormat::OggOpus),
            "audio/ogg"
        );
        assert_eq!(
            CloudDictationProvider::content_type(&AudioFormat::Flac),
            "audio/flac"
        );
        assert_eq!(
            CloudDictationProvider::content_type(&AudioFormat::RawPcm {
                sample_rate: 16_000,
                channels: 1,
                bits_per_sample: 16,
            }),
            "audio/l16"
        );
    }

    #[test]
    fn extract_transcription_deepgram_format() {
        let config = CloudDictationConfig {
            provider: "deepgram".into(),
            api_url: None,
            api_key: None,
            api_key_header: None,
            api_key_scheme: None,
            language: None,
            response_field: Some("results.channels.0.alternatives.0.transcript".into()),
        };
        let provider = CloudDictationProvider::new(config);

        let body: Value = serde_json::from_str(
            r#"{
                "results": {
                    "channels": [{
                        "alternatives": [{
                            "transcript": "hello world"
                        }]
                    }]
                }
            }"#,
        )
        .unwrap();

        assert_eq!(
            provider.extract_transcription(&body),
            Some("hello world".to_string())
        );
    }

    #[test]
    fn extract_transcription_google_format() {
        let config = CloudDictationConfig {
            provider: "google".into(),
            api_url: None,
            api_key: None,
            api_key_header: None,
            api_key_scheme: None,
            language: None,
            response_field: Some("results.0.alternatives.0.transcript".into()),
        };
        let provider = CloudDictationProvider::new(config);

        let body: Value = serde_json::from_str(
            r#"{
                "results": [{
                    "alternatives": [{
                        "transcript": "how are you today"
                    }]
                }]
            }"#,
        )
        .unwrap();

        assert_eq!(
            provider.extract_transcription(&body),
            Some("how are you today".to_string())
        );
    }

    #[test]
    fn extract_transcription_azure_format() {
        let config = CloudDictationConfig {
            provider: "azure".into(),
            api_url: None,
            api_key: None,
            api_key_header: None,
            api_key_scheme: None,
            language: None,
            response_field: Some("DisplayText".into()),
        };
        let provider = CloudDictationProvider::new(config);

        let body: Value = serde_json::from_str(r#"{"DisplayText": "good morning"}"#).unwrap();

        assert_eq!(
            provider.extract_transcription(&body),
            Some("good morning".to_string())
        );
    }

    #[test]
    fn extract_transcription_returns_none_for_missing_field() {
        let config = CloudDictationConfig {
            provider: "test".into(),
            api_url: None,
            api_key: None,
            api_key_header: None,
            api_key_scheme: None,
            language: None,
            response_field: Some("nonexistent.field".into()),
        };
        let provider = CloudDictationProvider::new(config);

        let body: Value = serde_json::from_str(r#"{"status": "ok"}"#).unwrap();

        assert_eq!(provider.extract_transcription(&body), None);
    }

    #[test]
    fn deepgram_prebuilt_config() {
        let config = deepgram::config("dg-api-key");

        assert_eq!(config.provider, "deepgram");
        assert_eq!(
            config.api_url.unwrap(),
            "https://api.deepgram.com/v1/listen"
        );
        assert_eq!(config.api_key.unwrap(), "dg-api-key");
        assert_eq!(config.api_key_header.unwrap(), "Authorization");
        assert_eq!(config.api_key_scheme.unwrap(), "Token");
        assert_eq!(
            config.response_field.unwrap(),
            "results.channels.0.alternatives.0.transcript"
        );
    }

    #[test]
    fn google_prebuilt_config() {
        let config = google::config("google-api-key");

        assert_eq!(config.provider, "google");
        assert_eq!(
            config.api_url.unwrap(),
            "https://speech.googleapis.com/v1/speech:recognize"
        );
        assert_eq!(config.api_key.unwrap(), "google-api-key");
        assert_eq!(config.api_key_header.unwrap(), "X-Goog-Api-Key");
        assert!(config.api_key_scheme.is_none());
        assert_eq!(
            config.response_field.unwrap(),
            "results.0.alternatives.0.transcript"
        );
    }

    #[test]
    fn azure_prebuilt_config() {
        let config = azure::config("az-api-key", "eastus");

        assert_eq!(config.provider, "azure");
        assert_eq!(
            config.api_url.unwrap(),
            "https://eastus.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1"
        );
        assert_eq!(config.api_key.unwrap(), "az-api-key");
        assert_eq!(config.api_key_header.unwrap(), "Ocp-Apim-Subscription-Key");
        assert!(config.api_key_scheme.is_none());
        assert_eq!(config.response_field.unwrap(), "DisplayText");
    }

    #[test]
    fn unsupported_format_returns_error() {
        let config = CloudDictationConfig {
            provider: "test".into(),
            api_url: None,
            api_key: None,
            api_key_header: None,
            api_key_scheme: None,
            language: None,
            response_field: None,
        };
        let provider = CloudDictationProvider::with_supported_formats(
            CloudDictationProvider::new(config),
            vec![AudioFormat::Wav],
        );

        let format = AudioFormat::Mp3;
        assert!(!provider.supported_formats().contains(&format));
    }

    #[test]
    fn transcribe_posts_audio_and_extracts_text() {
        let (url, request_rx) = spawn_test_server(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"results\":{\"channels\":[{\"alternatives\":[{\"transcript\":\"test transcript\"}]}]}}",
        );
        let provider = CloudDictationProvider::new(CloudDictationConfig {
            provider: "test".into(),
            api_url: Some(url),
            api_key: Some("secret".into()),
            api_key_header: Some("Authorization".into()),
            api_key_scheme: Some("Token".into()),
            language: Some("en".into()),
            response_field: None,
        });

        let transcript = block_on_tokio(provider.transcribe(&[1, 2, 3], AudioFormat::Wav))
            .expect("cloud provider should transcribe mocked response");
        let request = request_rx
            .recv()
            .expect("test server should receive one request");

        assert_eq!(transcript, "test transcript");
        assert!(request.starts_with("POST /?language=en HTTP/1.1"));
        assert!(request.contains("content-type: audio/wav"));
        assert!(request.contains("authorization: Token secret"));
        assert!(request.ends_with("\r\n\r\n\u{1}\u{2}\u{3}"));
    }

    #[test]
    fn transcribe_returns_provider_error_for_api_failure() {
        let (url, _request_rx) = spawn_test_server(
            "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"error\":\"rate limited\"}",
        );
        let provider = CloudDictationProvider::new(CloudDictationConfig {
            provider: "test".into(),
            api_url: Some(url),
            api_key: None,
            api_key_header: None,
            api_key_scheme: None,
            language: None,
            response_field: None,
        });

        let error = block_on_tokio(provider.transcribe(&[1, 2, 3], AudioFormat::Wav))
            .expect_err("non-success response should fail");

        assert_eq!(
            error.to_string(),
            "dictation provider `test` failed: API returned 429: rate limited"
        );
    }

    #[test]
    fn transcribe_rejects_unsupported_format_before_http_request() {
        let provider = CloudDictationProvider::new(CloudDictationConfig {
            provider: "test".into(),
            api_url: Some("http://127.0.0.1:1".into()),
            api_key: None,
            api_key_header: None,
            api_key_scheme: None,
            language: None,
            response_field: Some("text".into()),
        })
        .with_supported_formats(vec![AudioFormat::Wav]);

        let error = smol::block_on(provider.transcribe(&[1, 2, 3], AudioFormat::Mp3))
            .expect_err("unsupported format should fail");

        assert_eq!(
            error.to_string(),
            "dictation provider `test` does not support audio format Mp3"
        );
    }

    fn spawn_test_server(response: &'static str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("read test server address");
        let (request_tx, request_rx) = mpsc::channel();

        thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buffer = [0; 4096];
            let Ok(read) = stream.read(&mut buffer) else {
                return;
            };
            let request = String::from_utf8_lossy(&buffer[..read]).into_owned();
            request_tx.send(request).expect("send captured request");
            stream
                .write_all(response.as_bytes())
                .expect("write test response");
        });

        (format!("http://{address}"), request_rx)
    }

    fn block_on_tokio<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .expect("build tokio test runtime")
            .block_on(future)
    }
}
