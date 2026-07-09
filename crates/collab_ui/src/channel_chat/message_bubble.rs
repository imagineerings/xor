use super::markdown_style::channel_chat_markdown_style;
use gpui::{AnyElement, App, Entity, ImageSource, Resource, SharedString, SharedUri, Window};
use markdown::{Markdown, MarkdownElement};
use ui::prelude::*;

pub(super) struct MessageBody {
    source: String,
    markdown: Entity<Markdown>,
}

impl MessageBody {
    pub(super) fn new(source: String, cx: &mut App) -> Self {
        let body = SharedString::from(source.clone());
        let markdown = cx.new(|cx| Markdown::new(body, None, None, cx));
        Self { source, markdown }
    }

    pub(super) fn source(&self) -> &str {
        &self.source
    }

    pub(super) fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        div()
            .text_sm()
            .child(
                MarkdownElement::new(
                    self.markdown.clone(),
                    channel_chat_markdown_style(window, cx),
                )
                .on_url_click(|url, _, cx| cx.open_url(&url))
                .image_resolver(resolve_remote_image),
            )
            .into_any_element()
    }
}

pub(super) fn resolve_remote_image(destination_url: &str) -> Option<ImageSource> {
    if destination_url.starts_with("http://") || destination_url.starts_with("https://") {
        Some(ImageSource::Resource(Resource::Uri(SharedUri::from(
            destination_url.to_string(),
        ))))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_only_http_remote_images() {
        assert!(resolve_remote_image("https://example.com/image.png").is_some());
        assert!(resolve_remote_image("http://example.com/image.png").is_some());
        assert!(resolve_remote_image("data:image/png;base64,AAAA").is_none());
        assert!(resolve_remote_image("file:///tmp/image.png").is_none());
    }
}
