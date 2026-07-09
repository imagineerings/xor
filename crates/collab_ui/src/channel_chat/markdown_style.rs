use gpui::{App, Window};
use markdown::{MarkdownFont, MarkdownStyle};

pub(super) fn channel_chat_markdown_style(window: &Window, cx: &App) -> MarkdownStyle {
    MarkdownStyle::themed(MarkdownFont::Editor, window, cx)
}
