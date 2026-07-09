use super::{
    markdown_style::channel_chat_markdown_style,
    message_bubble::resolve_remote_image,
    sanitize::{sanitize_channel_markdown, trusted_channel_url},
};
use gpui::{AnyElement, App, Entity, SharedString, Window, prelude::*};
use markdown::{Markdown, MarkdownElement, MarkdownOptions};
use ui::{IconName, prelude::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ComposeMode {
    Source,
    Preview,
}

impl ComposeMode {
    pub(super) fn toggle(self) -> Self {
        match self {
            Self::Source => Self::Preview,
            Self::Preview => Self::Source,
        }
    }

    pub(super) fn toggle_icon(self) -> IconName {
        match self {
            Self::Source => IconName::Eye,
            Self::Preview => IconName::Pencil,
        }
    }

    pub(super) fn toggle_tooltip(self) -> &'static str {
        match self {
            Self::Source => "Show preview",
            Self::Preview => "Edit source",
        }
    }
}

pub(super) struct PreviewBody {
    source: String,
    truncated: bool,
    markdown: Entity<Markdown>,
}

impl PreviewBody {
    pub(super) fn new(source: String, cx: &mut App) -> Self {
        let sanitized = sanitize_channel_markdown(&source);
        let body = SharedString::from(sanitized.source);
        let markdown = cx.new(|cx| {
            Markdown::new_with_options(
                body,
                None,
                None,
                MarkdownOptions {
                    parse_html: false,
                    ..Default::default()
                },
                cx,
            )
        });
        Self {
            source,
            truncated: sanitized.truncated,
            markdown,
        }
    }

    pub(super) fn source(&self) -> &str {
        &self.source
    }

    pub(super) fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        div()
            .id("channel-compose-preview")
            .flex_1()
            .min_h(px(32.))
            .max_h(px(180.))
            .overflow_y_scroll()
            .text_sm()
            .child(
                MarkdownElement::new(
                    self.markdown.clone(),
                    channel_chat_markdown_style(window, cx),
                )
                .on_url_click(|url, _, cx| {
                    if let Some(url) = trusted_channel_url(url) {
                        cx.open_url(&url);
                    }
                })
                .image_resolver(resolve_remote_image),
            )
            .when(self.truncated, |this| {
                this.child(
                    Label::new("Preview truncated before rendering")
                        .size(LabelSize::XSmall)
                        .color(Color::Muted),
                )
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_mode_toggles_between_source_and_preview() {
        assert_eq!(ComposeMode::Source.toggle(), ComposeMode::Preview);
        assert_eq!(ComposeMode::Preview.toggle(), ComposeMode::Source);
    }
}
