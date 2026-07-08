use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimGameMediaKind {
    Texture,
    Shader,
    Video,
    Audio,
    Scene,
    RenderBackend,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameMediaClassification {
    pub extension: String,
    pub kind: SimGameMediaKind,
    pub preview_supported: bool,
    pub unsupported_reason: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGameMediaClassifier;

impl SimGameMediaClassifier {
    pub fn new() -> Self {
        Self
    }

    pub fn classify_path(&self, path: impl AsRef<Path>) -> SimGameMediaClassification {
        let extension = path
            .as_ref()
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        self.classify_extension(extension)
    }

    pub fn classify_extension(&self, extension: impl Into<String>) -> SimGameMediaClassification {
        let extension = extension
            .into()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        let (kind, preview_supported, unsupported_reason) = match extension.as_str() {
            "png" | "jpg" | "jpeg" | "webp" | "ktx" | "ktx2" => {
                (SimGameMediaKind::Texture, true, None)
            }
            "gdshader" | "shader" | "wgsl" => (SimGameMediaKind::Shader, true, None),
            "mp4" | "webm" | "mov" => (SimGameMediaKind::Video, true, None),
            "wav" | "ogg" | "mp3" => (SimGameMediaKind::Audio, true, None),
            "tscn" | "scn" => (SimGameMediaKind::Scene, true, None),
            "vulkan" | "d3d12" | "metal" | "gles" | "render_server" | "audio_server"
            | "text_server" => (
                SimGameMediaKind::RenderBackend,
                false,
                Some(
                    "render, audio, and text server backends are excluded from Sim media preview routing"
                        .to_string(),
                ),
            ),
            "res" | "import" => (
                SimGameMediaKind::Unknown,
                false,
                Some("binary or imported resources require engine inspection".to_string()),
            ),
            _ => (
                SimGameMediaKind::Unknown,
                false,
                Some("no native Sim preview route is registered for this media type".to_string()),
            ),
        };

        SimGameMediaClassification {
            extension,
            kind,
            preview_supported,
            unsupported_reason,
        }
    }
}
