use super::{markdown_style::channel_chat_markdown_style, message_bubble::resolve_remote_image};
use gpui::{AnyElement, App, Entity, SharedString, Window, prelude::*};
use markdown::{Markdown, MarkdownElement};
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
    markdown: Entity<Markdown>,
}

impl PreviewBody {
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
                .on_url_click(|url, _, cx| cx.open_url(&url))
                .image_resolver(resolve_remote_image),
            )
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
