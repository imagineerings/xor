use gpui::SharedString;

pub(super) const MAX_CHANNEL_MARKDOWN_SOURCE_LEN: usize = 10_000;

pub(super) struct SanitizedMarkdown {
    pub(super) source: String,
    pub(super) truncated: bool,
}

pub(super) fn sanitize_channel_markdown(input: &str) -> SanitizedMarkdown {
    let (input, truncated) = truncate_at_char_boundary(input, MAX_CHANNEL_MARKDOWN_SOURCE_LEN);
    SanitizedMarkdown {
        source: sanitize_markdown_html(input),
        truncated,
    }
}

pub(super) fn sanitize_markdown_html(input: &str) -> String {
    escape_html_tags(&strip_unsafe_inline_links(input))
}

pub(super) fn is_trusted_channel_url(destination_url: &str) -> bool {
    !has_untrusted_protocol(destination_url)
}

pub(super) fn trusted_channel_url(destination_url: SharedString) -> Option<SharedString> {
    is_trusted_channel_url(&destination_url).then_some(destination_url)
}

fn truncate_at_char_boundary(input: &str, max_len: usize) -> (&str, bool) {
    if input.len() <= max_len {
        return (input, false);
    }

    let mut end = max_len;
    while !input.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    (&input[..end], true)
}

fn strip_unsafe_inline_links(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while index < input.len() {
        let remaining = &input[index..];
        let image = remaining.starts_with("![");
        let label_open_len = if image {
            2
        } else if remaining.starts_with('[') {
            1
        } else {
            output.push(remaining.chars().next().unwrap_or_default());
            index += remaining
                .chars()
                .next()
                .map_or(1, |character| character.len_utf8());
            continue;
        };

        let label_start = index + label_open_len;
        let Some(label_end) = find_unescaped_byte(input, label_start, b']') else {
            output.push_str(&input[index..label_start]);
            index = label_start;
            continue;
        };

        if input.as_bytes().get(label_end + 1) != Some(&b'(') {
            output.push_str(&input[index..=label_end]);
            index = label_end + 1;
            continue;
        }

        let destination_start = label_end + 2;
        let Some(destination_end) = find_link_destination_end(input, destination_start) else {
            output.push_str(&input[index..destination_start]);
            index = destination_start;
            continue;
        };

        let destination = &input[destination_start..destination_end];
        if has_untrusted_protocol(destination) {
            output.push_str(&input[label_start..label_end]);
        } else {
            output.push_str(&input[index..=destination_end]);
        }
        index = destination_end + 1;
    }

    output
}

fn find_unescaped_byte(input: &str, mut index: usize, needle: u8) -> Option<usize> {
    let bytes = input.as_bytes();
    while index < bytes.len() {
        if bytes[index] == needle && !is_escaped(bytes, index) {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn find_link_destination_end(input: &str, mut index: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut nested_parentheses = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'(' if !is_escaped(bytes, index) => {
                nested_parentheses = nested_parentheses.saturating_add(1);
            }
            b')' if !is_escaped(bytes, index) => {
                if nested_parentheses == 0 {
                    return Some(index);
                }
                nested_parentheses = nested_parentheses.saturating_sub(1);
            }
            _ => {}
        }
        index += 1;
    }

    None
}

fn is_escaped(bytes: &[u8], index: usize) -> bool {
    let mut slash_count = 0;
    let mut cursor = index;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        slash_count += 1;
        cursor -= 1;
    }
    slash_count % 2 == 1
}

fn escape_html_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_tag = false;

    while let Some(character) = chars.next() {
        if character == '<' && chars.peek().is_some_and(|next| starts_html_tag(*next)) {
            output.push_str("&lt;");
            in_tag = true;
        } else if character == '>' && in_tag {
            output.push_str("&gt;");
            in_tag = false;
        } else {
            output.push(character);
        }
    }

    output
}

fn starts_html_tag(character: char) -> bool {
    character.is_ascii_alphabetic() || matches!(character, '/' | '!' | '?')
}

fn has_untrusted_protocol(destination_url: &str) -> bool {
    let normalized = destination_url
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .chars()
        .filter(|character| !character.is_ascii_whitespace() && !character.is_control())
        .take("javascript:".len())
        .collect::<String>()
        .to_ascii_lowercase();

    normalized.starts_with("javascript:")
        || normalized.starts_with("data:")
        || normalized.starts_with("file:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_raw_html_tags() {
        assert_eq!(
            sanitize_markdown_html("before <script>alert(1)</script> after"),
            "before &lt;script&gt;alert(1)&lt;/script&gt; after"
        );
    }

    #[test]
    fn removes_unsafe_inline_links() {
        assert_eq!(
            sanitize_markdown_html("[click](javascript:alert(1))"),
            "click"
        );
        assert_eq!(
            sanitize_markdown_html("![alt](data:image/png;base64,AAAA)"),
            "alt"
        );
        assert_eq!(sanitize_markdown_html("[file](FILE:///tmp/a)"), "file");
    }

    #[test]
    fn preserves_safe_inline_links() {
        assert_eq!(
            sanitize_markdown_html("[site](https://example.com)"),
            "[site](https://example.com)"
        );
    }

    #[test]
    fn truncates_at_char_boundary() {
        let input = format!("{}é", "a".repeat(MAX_CHANNEL_MARKDOWN_SOURCE_LEN - 1));
        let sanitized = sanitize_channel_markdown(&input);

        assert!(sanitized.truncated);
        assert_eq!(sanitized.source.len(), MAX_CHANNEL_MARKDOWN_SOURCE_LEN - 1);
    }

    #[test]
    fn rejects_untrusted_protocols_in_click_handler() {
        assert!(!is_trusted_channel_url("javascript:alert(1)"));
        assert!(!is_trusted_channel_url("java\nscript:alert(1)"));
        assert!(!is_trusted_channel_url("data:text/html,<script></script>"));
        assert!(!is_trusted_channel_url("file:///tmp/file"));
        assert!(is_trusted_channel_url("https://example.com"));
    }
}
