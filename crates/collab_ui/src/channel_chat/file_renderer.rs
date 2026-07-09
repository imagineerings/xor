#![allow(dead_code)] // Wired into channel message rendering in the next file-sharing task.

use client::FileAttachment;
use gpui::{
    AnyElement, App, ImageSource, IntoElement, ParentElement, RenderOnce, Resource, SharedUri,
    Styled as _, Window, img, px,
};
use ui::{Button, ButtonStyle, Color, Icon, IconName, IconSize, Label, LabelSize, prelude::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FileKind {
    Image,
    Video,
    Audio,
    Pdf,
    Code,
    Other,
}

#[derive(IntoElement)]
pub(super) struct FileAttachmentRenderer {
    file: FileAttachment,
}

impl FileAttachmentRenderer {
    pub(super) fn new(file: FileAttachment) -> Self {
        Self { file }
    }

    pub(super) fn detect_file_kind(mime_type: &str, filename: &str) -> FileKind {
        let mime_type = mime_type.to_ascii_lowercase();
        let extension = filename
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase());

        if mime_type.starts_with("image/") {
            return FileKind::Image;
        }
        if mime_type.starts_with("video/") {
            return FileKind::Video;
        }
        if mime_type.starts_with("audio/") {
            return FileKind::Audio;
        }
        if mime_type == "application/pdf" || extension.as_deref() == Some("pdf") {
            return FileKind::Pdf;
        }
        if mime_type.starts_with("text/") || is_code_extension(extension.as_deref()) {
            return FileKind::Code;
        }

        FileKind::Other
    }

    fn render_image_preview(&self, cx: &mut App) -> AnyElement {
        v_flex()
            .gap_2()
            .child(
                img(ImageSource::Resource(Resource::Uri(SharedUri::from(
                    self.file.url.clone(),
                ))))
                .max_h(px(220.))
                .max_w(px(360.))
                .rounded_sm()
                .border_1()
                .border_color(cx.theme().colors().border),
            )
            .child(self.render_file_card(cx))
            .into_any_element()
    }

    fn render_file_card(&self, cx: &mut App) -> AnyElement {
        let file = self.file.clone();
        let icon = icon_for_file_kind(Self::detect_file_kind(&file.mime_type, &file.filename));
        h_flex()
            .gap_2()
            .items_center()
            .max_w(px(420.))
            .px_2()
            .py_2()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().editor_background)
            .child(Icon::new(icon).size(IconSize::Small).color(Color::Muted))
            .child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(Label::new(file.filename.clone()).truncate())
                    .child(
                        Label::new(file_metadata_label(&file))
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    ),
            )
            .child(
                Button::new(format!("download-channel-file-{}", file.id), "Download")
                    .style(ButtonStyle::Subtle)
                    .on_click(move |_, _, cx| cx.open_url(&file.url)),
            )
            .into_any_element()
    }
}

impl RenderOnce for FileAttachmentRenderer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        match Self::detect_file_kind(&self.file.mime_type, &self.file.filename) {
            FileKind::Image => self.render_image_preview(cx),
            FileKind::Video
            | FileKind::Audio
            | FileKind::Pdf
            | FileKind::Code
            | FileKind::Other => self.render_file_card(cx),
        }
    }
}

fn icon_for_file_kind(file_kind: FileKind) -> IconName {
    match file_kind {
        FileKind::Image => IconName::File,
        FileKind::Video => IconName::File,
        FileKind::Audio => IconName::AudioOn,
        FileKind::Pdf => IconName::FileDoc,
        FileKind::Code => IconName::FileCode,
        FileKind::Other => IconName::FileGeneric,
    }
}

fn file_metadata_label(file: &FileAttachment) -> String {
    let mut parts = vec![
        format_file_size(file.file_size),
        file.mime_type.clone(),
        format!("Uploader #{}", file.uploader_id),
    ];
    if let (Some(width), Some(height)) = (file.image_width, file.image_height) {
        parts.push(format!("{width}x{height}"));
    }
    if let Some(duration_ms) = file.duration_ms {
        parts.push(format_duration(duration_ms));
    }
    parts.join(" · ")
}

fn format_file_size(file_size: u64) -> String {
    const KIB: f64 = 1024.0;
    let file_size = file_size as f64;
    if file_size < KIB {
        format!("{} B", file_size as u64)
    } else if file_size < KIB * KIB {
        format!("{:.1} KB", file_size / KIB)
    } else if file_size < KIB * KIB * KIB {
        format!("{:.1} MB", file_size / KIB / KIB)
    } else {
        format!("{:.1} GB", file_size / KIB / KIB / KIB)
    }
}

fn format_duration(duration_ms: u64) -> String {
    let total_seconds = duration_ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes}:{seconds:02}")
}

fn is_code_extension(extension: Option<&str>) -> bool {
    matches!(
        extension,
        Some(
            "c" | "cc"
                | "cpp"
                | "cs"
                | "css"
                | "go"
                | "h"
                | "hpp"
                | "html"
                | "java"
                | "js"
                | "jsx"
                | "json"
                | "kt"
                | "md"
                | "php"
                | "py"
                | "rb"
                | "rs"
                | "sh"
                | "sql"
                | "swift"
                | "toml"
                | "ts"
                | "tsx"
                | "xml"
                | "yaml"
                | "yml"
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_file_kind_from_mime_type() {
        assert_eq!(
            FileAttachmentRenderer::detect_file_kind("image/png", "image"),
            FileKind::Image
        );
        assert_eq!(
            FileAttachmentRenderer::detect_file_kind("video/mp4", "movie"),
            FileKind::Video
        );
        assert_eq!(
            FileAttachmentRenderer::detect_file_kind("audio/mpeg", "song"),
            FileKind::Audio
        );
        assert_eq!(
            FileAttachmentRenderer::detect_file_kind("application/pdf", "paper"),
            FileKind::Pdf
        );
        assert_eq!(
            FileAttachmentRenderer::detect_file_kind("text/plain", "notes"),
            FileKind::Code
        );
    }

    #[test]
    fn detects_file_kind_from_extension_fallback() {
        assert_eq!(
            FileAttachmentRenderer::detect_file_kind("application/octet-stream", "main.rs"),
            FileKind::Code
        );
        assert_eq!(
            FileAttachmentRenderer::detect_file_kind("application/octet-stream", "report.pdf"),
            FileKind::Pdf
        );
        assert_eq!(
            FileAttachmentRenderer::detect_file_kind("application/octet-stream", "archive.zip"),
            FileKind::Other
        );
    }
}
