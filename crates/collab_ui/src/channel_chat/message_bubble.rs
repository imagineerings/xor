use gpui::{AnyElement, App, Entity, SharedString, Window};
use markdown::{Markdown, MarkdownElement, MarkdownFont, MarkdownStyle};
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
            .child(MarkdownElement::new(
                self.markdown.clone(),
                MarkdownStyle::themed(MarkdownFont::Editor, window, cx),
            ))
            .into_any_element()
    }
}
