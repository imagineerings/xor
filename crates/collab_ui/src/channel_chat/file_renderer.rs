use client::FileAttachment;
use gpui::{
    AnyElement, App, Context, DismissEvent, EventEmitter, FocusHandle, Focusable, ImageSource,
    IntoElement, ObjectFit, ParentElement, Render, RenderOnce, Resource, SharedUri, Styled as _,
    Window, img, px,
};
use std::rc::Rc;
use ui::{
    Button, ButtonStyle, Color, Icon, IconButton, IconName, IconSize, Label, LabelSize, Tooltip,
    prelude::*,
};
use workspace::ModalView;

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
    on_open_image: Option<ImageOpenHandler>,
}

type ImageOpenHandler = Rc<dyn Fn(FileAttachment, &mut Window, &mut App)>;

impl FileAttachmentRenderer {
    pub(super) fn with_image_open_handler(
        file: FileAttachment,
        on_open_image: ImageOpenHandler,
    ) -> Self {
        Self {
            file,
            on_open_image: Some(on_open_image),
        }
    }

    pub(super) fn detect_file_kind(mime_type: &str, filename: &str) -> FileKind {
        let mime_type = mime_type.to_ascii_lowercase();
        let extension = filename
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase());

        if mime_type.starts_with("image/") || is_image_extension(extension.as_deref()) {
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
        let file = self.file.clone();
        let on_open_image = self.on_open_image.clone();
        v_flex()
            .gap_2()
            .child(
                img(ImageSource::Resource(Resource::Uri(SharedUri::from(
                    file.url.clone(),
                ))))
                .id(format!("channel-image-preview-{}", file.id))
                .max_h(px(220.))
                .max_w(px(360.))
                .object_fit(ObjectFit::ScaleDown)
                .rounded_sm()
                .border_1()
                .border_color(cx.theme().colors().border)
                .when_some(on_open_image, |this, on_open_image| {
                    this.cursor_pointer().on_click(move |_, window, cx| {
                        on_open_image(file.clone(), window, cx);
                    })
                }),
            )
            .child(self.render_file_card(cx))
            .into_any_element()
    }

    fn render_pdf_thumbnail(&self, cx: &mut App) -> AnyElement {
        let file = self.file.clone();
        h_flex()
            .gap_3()
            .items_center()
            .max_w(px(420.))
            .px_3()
            .py_2()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().editor_background)
            .child(
                div()
                    .flex_none()
                    .size(px(40.))
                    .rounded_sm()
                    .border_1()
                    .border_color(cx.theme().colors().border_variant)
                    .bg(cx.theme().colors().element_background)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(IconName::FileDoc)
                            .size(IconSize::Medium)
                            .color(Color::Muted),
                    ),
            )
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
                Button::new(format!("view-channel-pdf-{}", file.id), "View PDF")
                    .style(ButtonStyle::Subtle)
                    .on_click(move |_, _, cx| cx.open_url(&file.url)),
            )
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

pub(super) struct ImagePreviewModal {
    file: FileAttachment,
    focus_handle: FocusHandle,
}

impl ImagePreviewModal {
    pub(super) fn new(file: FileAttachment, cx: &mut Context<Self>) -> Self {
        Self {
            file,
            focus_handle: cx.focus_handle(),
        }
    }

    fn dismiss(&mut self, _: &menu::Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }
}

impl EventEmitter<DismissEvent> for ImagePreviewModal {}
impl ModalView for ImagePreviewModal {
    fn fade_out_background(&self) -> bool {
        true
    }
}

impl Focusable for ImagePreviewModal {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ImagePreviewModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let file = self.file.clone();
        v_flex()
            .id("channel-image-preview-modal")
            .key_context("ChannelImagePreview")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::dismiss))
            .gap_3()
            .p_3()
            .w(px(900.))
            .max_w(px(900.))
            .rounded_md()
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().elevated_surface_background)
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .gap_3()
                    .child(Label::new(file.filename.clone()).truncate())
                    .child(
                        IconButton::new("close-channel-image-preview", IconName::Close)
                            .icon_size(IconSize::Small)
                            .tooltip(Tooltip::text("Close image preview"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.dismiss(&menu::Cancel, window, cx);
                            })),
                    ),
            )
            .child(
                img(ImageSource::Resource(Resource::Uri(SharedUri::from(
                    file.url,
                ))))
                .w_full()
                .max_h(px(640.))
                .object_fit(ObjectFit::ScaleDown)
                .rounded_sm(),
            )
    }
}

impl RenderOnce for FileAttachmentRenderer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        match Self::detect_file_kind(&self.file.mime_type, &self.file.filename) {
            FileKind::Image => self.render_image_preview(cx),
            FileKind::Pdf => self.render_pdf_thumbnail(cx),
            FileKind::Video | FileKind::Audio | FileKind::Code | FileKind::Other => {
                self.render_file_card(cx)
            }
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

fn is_image_extension(extension: Option<&str>) -> bool {
    matches!(
        extension,
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "svg")
    )
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
        for filename in [
            "image.png",
            "image.jpg",
            "image.jpeg",
            "image.gif",
            "image.webp",
            "image.svg",
        ] {
            assert_eq!(
                FileAttachmentRenderer::detect_file_kind("application/octet-stream", filename),
                FileKind::Image,
                "{filename} should render as an image"
            );
        }
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

    #[test]
    fn pdf_files_use_pdf_icon_and_metadata() {
        let file = file_attachment("runbook.pdf", "application/pdf");

        assert_eq!(
            icon_for_file_kind(FileAttachmentRenderer::detect_file_kind(
                &file.mime_type,
                &file.filename
            )),
            IconName::FileDoc
        );
        assert_eq!(
            file_metadata_label(&file),
            "4.0 KB · application/pdf · Uploader #7"
        );
    }

    #[test]
    fn metadata_label_includes_image_dimensions_and_duration() {
        let file = FileAttachment {
            image_width: Some(800),
            image_height: Some(600),
            duration_ms: Some(65_000),
            ..file_attachment("clip.mp4", "video/mp4")
        };

        assert_eq!(
            file_metadata_label(&file),
            "4.0 KB · video/mp4 · Uploader #7 · 800x600 · 1:05"
        );
    }

    fn file_attachment(filename: &str, mime_type: &str) -> FileAttachment {
        FileAttachment {
            id: "file-id".to_string(),
            filename: filename.to_string(),
            file_size: 4096,
            mime_type: mime_type.to_string(),
            url: "https://example.com/file".to_string(),
            uploader_id: 7,
            uploaded_at: None,
            image_width: None,
            image_height: None,
            duration_ms: None,
        }
    }
}
