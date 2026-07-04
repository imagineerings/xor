use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// Telegram's maximum message length in characters.
const TELEGRAM_MAX_LENGTH: usize = 4096;

/// Formats agent responses for Telegram using HTML parse mode.
///
/// Converts markdown to Telegram-compatible HTML and splits long
/// messages into multiple chunks that fit within Telegram's limits.
pub struct TelegramFormatter;

impl TelegramFormatter {
    /// Convert a markdown string to Telegram-compatible HTML.
    ///
    /// Supported formatting:
    /// - `**bold**` / `__bold__` → `<b>...</b>`
    /// - `*italic*` / `_italic_` → `<i>...</i>`
    /// - `` `inline code` `` → `<code>...</code>`
    /// - ``` ```language ... ``` ``` → `<pre><code class="language-...">...</code></pre>`
    /// - `[text](url)` → `<a href="url">text</a>`
    /// - Headings → `<b>...</b>` (Telegram doesn't support headings)
    /// - Lists → preserved with line breaks
    /// - Paragraphs → separated by newlines
    pub fn format_to_html(markdown: &str) -> String {
        let mut html = String::new();
        let mut in_code_block = false;

        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);

        let parser = Parser::new_ext(markdown, options);

        for event in parser {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Paragraph => {}
                    Tag::Heading { .. } => html.push_str("<b>"),
                    Tag::BlockQuote(_) => html.push_str("<i>"),
                    Tag::CodeBlock(kind) => {
                        in_code_block = true;
                        match kind {
                            CodeBlockKind::Fenced(ref lang) => {
                                if lang.is_empty() {
                                    html.push_str("<pre><code>");
                                } else {
                                    let escaped = escape_html(lang);
                                    html.push_str("<pre><code class=\"language-");
                                    html.push_str(&escaped);
                                    html.push_str("\">");
                                }
                            }
                            CodeBlockKind::Indented => {
                                html.push_str("<pre><code>");
                            }
                        }
                    }
                    Tag::List(_) | Tag::Item => {}
                    Tag::Emphasis => html.push_str("<i>"),
                    Tag::Strong => html.push_str("<b>"),
                    Tag::Strikethrough => html.push_str("<s>"),
                    Tag::Link { dest_url, .. } => {
                        html.push_str("<a href=\"");
                        html.push_str(&escape_html(&dest_url));
                        html.push_str("\">");
                    }
                    _ => {}
                },
                Event::End(tag) => match tag {
                    TagEnd::Paragraph => html.push('\n'),
                    TagEnd::Heading { .. } => html.push_str("</b>\n"),
                    TagEnd::BlockQuote(_) => html.push_str("</i>\n"),
                    TagEnd::CodeBlock => {
                        html.push_str("</code></pre>\n");
                        in_code_block = false;
                    }
                    TagEnd::List(_) => html.push('\n'),
                    TagEnd::Item => html.push('\n'),
                    TagEnd::Emphasis => html.push_str("</i>"),
                    TagEnd::Strong => html.push_str("</b>"),
                    TagEnd::Strikethrough => html.push_str("</s>"),
                    TagEnd::Link => html.push_str("</a>"),
                    _ => {}
                },
                Event::Text(text) => {
                    if in_code_block {
                        html.push_str(&text);
                    } else {
                        html.push_str(&escape_html(&text));
                    }
                }
                Event::Code(text) => {
                    html.push_str("<code>");
                    html.push_str(&escape_html(&text));
                    html.push_str("</code>");
                }
                Event::SoftBreak => html.push('\n'),
                Event::HardBreak => html.push('\n'),
                _ => {}
            }
        }

        html
    }

    /// Split a message into chunks that fit within Telegram's length limit.
    ///
    /// Splits on paragraph boundaries (double newlines) when possible.
    /// If a single paragraph exceeds the limit, it is split at the
    /// character boundary.
    pub fn split_message(text: &str) -> Vec<String> {
        if text.len() <= TELEGRAM_MAX_LENGTH {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut start = 0;
        let bytes = text.as_bytes();

        while start < bytes.len() {
            // Determine the end of this chunk
            let end = if start + TELEGRAM_MAX_LENGTH >= bytes.len() {
                bytes.len()
            } else {
                // Try to find a paragraph boundary (double newline) before the limit
                let search_end = start + TELEGRAM_MAX_LENGTH;
                if let Some(boundary) = find_last_double_newline(bytes, start, search_end) {
                    boundary
                } else {
                    // Try to find a single newline boundary
                    if let Some(boundary) = find_last_newline(bytes, start, search_end) {
                        boundary
                    } else {
                        search_end
                    }
                }
            };

            chunks.push(text[start..end].to_string());
            start = end;

            // Skip leading newlines in the next chunk
            while start < bytes.len() && (bytes[start] == b'\n' || bytes[start] == b'\r') {
                start += 1;
            }
        }

        chunks
    }

    /// Format and split a markdown message for Telegram.
    ///
    /// Returns a list of HTML-formatted strings, each within Telegram's
    /// message length limit.
    pub fn format_and_split(markdown: &str) -> Vec<String> {
        let html = Self::format_to_html(markdown);
        Self::split_message(&html)
    }
}

/// Escape characters that have special meaning in Telegram HTML.
///
/// Telegram's HTML parser only supports `&`, `<`, and `>` as entities.
fn escape_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            other => result.push(other),
        }
    }
    result
}

/// Find the last occurrence of `\n\n` in `bytes[start..end]`.
/// Returns the position _after_ the double newline (i.e., the start of the
/// next paragraph). Returns `None` if no double newline is found.
fn find_last_double_newline(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
    let search_end = end.min(bytes.len());
    let mut found = None;
    let mut i = start;

    while i + 1 < search_end {
        if bytes[i] == b'\n' && bytes[i + 1] == b'\n' {
            found = Some(i + 2);
        }
        i += 1;
    }

    found
}

/// Find the last newline in `bytes[start..end]`.
/// Returns the position _after_ the newline.
fn find_last_newline(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
    let search_end = end.min(bytes.len());
    let mut found = None;
    let mut i = start;

    while i < search_end {
        if bytes[i] == b'\n' {
            found = Some(i + 1);
        }
        i += 1;
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold_italic() {
        let html = TelegramFormatter::format_to_html("**bold** and *italic*");
        assert!(html.contains("<b>bold</b>"));
        assert!(html.contains("<i>italic</i>"));
    }

    #[test]
    fn test_inline_code() {
        let html = TelegramFormatter::format_to_html("Use `code` here");
        assert!(html.contains("<code>code</code>"));
    }

    #[test]
    fn test_code_block() {
        let markdown = "```rust\nfn main() {}\n```";
        let html = TelegramFormatter::format_to_html(markdown);
        assert!(html.contains("<pre><code class=\"language-rust\">"));
        assert!(html.contains("fn main() {}"));
        assert!(html.contains("</code></pre>"));
    }

    #[test]
    fn test_link() {
        let html = TelegramFormatter::format_to_html("[click](https://example.com)");
        assert!(html.contains("<a href=\"https://example.com\">"));
        assert!(html.contains("click"));
        assert!(html.contains("</a>"));
    }

    #[test]
    fn test_strikethrough() {
        let html = TelegramFormatter::format_to_html("~~strike~~");
        assert!(html.contains("<s>strike</s>"));
    }

    #[test]
    fn test_html_escaping() {
        let html = TelegramFormatter::format_to_html("x < 5 & 7 > 3");
        assert!(html.contains("&lt;"));
        assert!(html.contains("&amp;"));
        assert!(html.contains("&gt;"));
    }

    #[test]
    fn test_no_split_for_short_message() {
        let chunks = TelegramFormatter::split_message("Hello, world!");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Hello, world!");
    }

    #[test]
    fn test_split_at_paragraph() {
        let long_text = "A".repeat(3000) + "\n\n" + &"B".repeat(2000);
        let chunks = TelegramFormatter::split_message(&long_text);
        assert!(
            chunks.len() >= 2,
            "expected at least 2 chunks, got {}",
            chunks.len()
        );
        // First chunk should end at the paragraph boundary (after the double newline)
        assert!(
            chunks[0].ends_with("\n\n"),
            "first chunk should end with double newline, ends with: {:?}",
            chunks[0].chars().rev().take(4).collect::<String>()
        );
    }

    #[test]
    fn test_each_chunk_within_limit() {
        let text = "word ".repeat(5000);
        let text = text.as_str();
        let chunks = TelegramFormatter::split_message(text);
        for chunk in &chunks {
            assert!(
                chunk.len() <= TELEGRAM_MAX_LENGTH,
                "chunk too long: {}",
                chunk.len()
            );
        }
    }

    #[test]
    fn test_format_and_split_produces_valid_chunks() {
        let markdown = "# Title\n\nSome **bold** and *italic* text.\n\n```rust\nlet x = 1;\n```\n\nMore text.\n\n" .repeat(200);
        let chunks = TelegramFormatter::format_and_split(&markdown);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(
                chunk.len() <= TELEGRAM_MAX_LENGTH,
                "chunk too long: {}",
                chunk.len()
            );
        }
    }
}
