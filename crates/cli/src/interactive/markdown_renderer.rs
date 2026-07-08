#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownRendererOptions {
    pub clickable_links: bool,
    pub syntax_highlighting: bool,
}

impl Default for MarkdownRendererOptions {
    fn default() -> Self {
        Self {
            clickable_links: true,
            syntax_highlighting: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MarkdownRenderer {
    options: MarkdownRendererOptions,
}

impl MarkdownRenderer {
    pub fn new(options: MarkdownRendererOptions) -> Self {
        Self { options }
    }

    pub fn render(&self, markdown: &str) -> Vec<String> {
        let mut rendered = Vec::new();
        let mut paragraph = Vec::new();
        let mut code_block_language: Option<String> = None;
        let mut code_block_lines = Vec::new();

        for line in markdown.lines() {
            if let Some(language) = line.trim_start().strip_prefix("```") {
                if let Some(language) = code_block_language.take() {
                    self.flush_paragraph(&mut paragraph, &mut rendered);
                    rendered.extend(
                        self.render_code_block(&code_block_lines.join("\n"), language.trim()),
                    );
                    code_block_lines.clear();
                } else {
                    self.flush_paragraph(&mut paragraph, &mut rendered);
                    code_block_language = Some(language.trim().to_string());
                }
                continue;
            }

            if code_block_language.is_some() {
                code_block_lines.push(line.to_string());
                continue;
            }

            if line.trim().is_empty() {
                self.flush_paragraph(&mut paragraph, &mut rendered);
                rendered.push(String::new());
                continue;
            }

            if let Some(rendered_line) = self.render_block_line(line) {
                self.flush_paragraph(&mut paragraph, &mut rendered);
                rendered.push(rendered_line);
            } else {
                paragraph.push(line.trim().to_string());
            }
        }

        self.flush_paragraph(&mut paragraph, &mut rendered);

        if let Some(language) = code_block_language {
            rendered.extend(self.render_code_block(&code_block_lines.join("\n"), language.trim()));
        }

        rendered
    }

    pub fn render_code_block(&self, code: &str, language: &str) -> Vec<String> {
        let mut rendered = Vec::new();
        let header = if language.is_empty() {
            "code".to_string()
        } else {
            format!("code ({language})")
        };
        rendered.push(format!("{}{}{}", ansi::DIM, header, ansi::RESET));

        for line in code.lines() {
            let line = if self.options.syntax_highlighting {
                highlight_code_line(line, language)
            } else {
                line.to_string()
            };
            rendered.push(format!("{}  {}{}", ansi::CYAN, line, ansi::RESET));
        }

        rendered
    }

    fn render_block_line(&self, line: &str) -> Option<String> {
        let trimmed = line.trim_start();

        if let Some((level, heading)) = parse_heading(trimmed) {
            let color = if level <= 2 { ansi::BOLD } else { ansi::DIM };
            return Some(format!(
                "{color}{}{}",
                self.render_inline(heading.trim()),
                ansi::RESET
            ));
        }

        if let Some(item) = parse_unordered_list_item(trimmed) {
            return Some(format!("  - {}", self.render_inline(item.trim())));
        }

        if let Some(item) = parse_ordered_list_item(trimmed) {
            return Some(format!(
                "  {}. {}",
                item.number,
                self.render_inline(item.text.trim())
            ));
        }

        if let Some(quote) = trimmed.strip_prefix("> ") {
            return Some(format!(
                "{}> {}{}",
                ansi::DIM,
                self.render_inline(quote.trim()),
                ansi::RESET
            ));
        }

        if is_table_line(trimmed) {
            return Some(format!("{}{}{}", ansi::CYAN, trimmed, ansi::RESET));
        }

        None
    }

    fn flush_paragraph(&self, paragraph: &mut Vec<String>, rendered: &mut Vec<String>) {
        if paragraph.is_empty() {
            return;
        }
        rendered.push(self.render_inline(&paragraph.join(" ")));
        paragraph.clear();
    }

    fn render_inline(&self, text: &str) -> String {
        let mut rendered = String::new();
        let mut remaining = text;

        while !remaining.is_empty() {
            if let Some((before, link_text, url, after)) = parse_link(remaining) {
                rendered.push_str(&render_inline_styles(before));
                rendered.push_str(&self.render_link(link_text, url));
                remaining = after;
                continue;
            }

            rendered.push_str(&render_inline_styles(remaining));
            break;
        }

        rendered
    }

    fn render_link(&self, text: &str, url: &str) -> String {
        if self.options.clickable_links {
            format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\")
        } else {
            format!("{text} ({url})")
        }
    }
}

struct OrderedListItem<'a> {
    number: &'a str,
    text: &'a str,
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let marker_count = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if marker_count == 0 || marker_count > 6 {
        return None;
    }
    let heading = line.get(marker_count..)?;
    if !heading.starts_with(' ') {
        return None;
    }
    Some((marker_count, heading))
}

fn parse_unordered_list_item(line: &str) -> Option<&str> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(item) = line.strip_prefix(marker) {
            return Some(item);
        }
    }
    None
}

fn parse_ordered_list_item(line: &str) -> Option<OrderedListItem<'_>> {
    let dot_index = line.find('.')?;
    let (number, rest) = line.split_at(dot_index);
    if number.is_empty() || !number.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let text = rest.strip_prefix(". ")?;
    Some(OrderedListItem { number, text })
}

fn is_table_line(line: &str) -> bool {
    let pipe_count = line.chars().filter(|character| *character == '|').count();
    pipe_count >= 2
}

fn parse_link(text: &str) -> Option<(&str, &str, &str, &str)> {
    let link_start = text.find('[')?;
    let link_text_end = text[link_start..].find(']')? + link_start;
    let url_start = link_text_end + 1;
    let url = text.get(url_start..)?.strip_prefix('(')?;
    let url_end = url.find(')')?;
    let before = &text[..link_start];
    let link_text = &text[link_start + 1..link_text_end];
    let url = &url[..url_end];
    let after_start = url_start + url_end + 2;
    let after = text.get(after_start..)?;
    Some((before, link_text, url, after))
}

fn render_inline_styles(text: &str) -> String {
    let text = replace_delimited(text, "`", ansi::YELLOW, ansi::RESET);
    let text = replace_delimited(&text, "**", ansi::BOLD, ansi::RESET);
    replace_delimited(&text, "*", ansi::ITALIC, ansi::RESET)
}

fn replace_delimited(text: &str, delimiter: &str, prefix: &str, suffix: &str) -> String {
    let mut rendered = String::new();
    let mut remaining = text;

    loop {
        let Some(start) = remaining.find(delimiter) else {
            rendered.push_str(remaining);
            break;
        };
        let content_start = start + delimiter.len();
        let Some(end) = remaining[content_start..].find(delimiter) else {
            rendered.push_str(remaining);
            break;
        };
        rendered.push_str(&remaining[..start]);
        rendered.push_str(prefix);
        rendered.push_str(&remaining[content_start..content_start + end]);
        rendered.push_str(suffix);
        remaining = &remaining[content_start + end + delimiter.len()..];
    }

    rendered
}

fn highlight_code_line(line: &str, language: &str) -> String {
    let keyword_set: &[&str] = match language {
        "rs" | "rust" => &[
            "async", "await", "enum", "fn", "impl", "let", "match", "pub", "struct",
        ][..],
        "js" | "javascript" | "ts" | "typescript" => &[
            "async", "await", "const", "export", "function", "import", "let", "return", "type",
        ][..],
        "sh" | "bash" | "zsh" => &["case", "do", "done", "else", "fi", "for", "if", "then"][..],
        _ => &[][..],
    };

    if keyword_set.is_empty() {
        return line.to_string();
    }

    let mut highlighted = String::new();
    let mut token = String::new();

    for character in line.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            token.push(character);
        } else {
            push_highlighted_token(&mut highlighted, &token, keyword_set);
            token.clear();
            highlighted.push(character);
        }
    }
    push_highlighted_token(&mut highlighted, &token, keyword_set);
    highlighted
}

fn push_highlighted_token(rendered: &mut String, token: &str, keyword_set: &[&str]) {
    if token.is_empty() {
        return;
    }

    if keyword_set.contains(&token) {
        rendered.push_str(ansi::MAGENTA);
        rendered.push_str(token);
        rendered.push_str(ansi::RESET);
    } else {
        rendered.push_str(token);
    }
}

mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const ITALIC: &str = "\x1b[3m";
    pub const CYAN: &str = "\x1b[36m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const YELLOW: &str = "\x1b[33m";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown_blocks_and_inline_styles() {
        let renderer = MarkdownRenderer::new(MarkdownRendererOptions {
            clickable_links: false,
            syntax_highlighting: true,
        });

        let rendered = renderer.render(
            "# Title\n\nA **bold** word and `code`.\n\n- item\n1. numbered\n> quote\n| a | b |",
        );

        assert!(rendered.iter().any(|line| line.contains("\x1b[1mTitle")));
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("\x1b[1mbold\x1b[0m"))
        );
        assert!(
            rendered
                .iter()
                .any(|line| line.contains("\x1b[33mcode\x1b[0m"))
        );
        assert!(rendered.iter().any(|line| line.contains("  - item")));
        assert!(rendered.iter().any(|line| line.contains("  1. numbered")));
        assert!(rendered.iter().any(|line| line.contains("> quote")));
        assert!(rendered.iter().any(|line| line.contains("| a | b |")));
    }

    #[test]
    fn renders_code_block_with_language_header_and_keyword_highlighting() {
        let renderer = MarkdownRenderer::default();

        let rendered = renderer.render("```rust\npub fn main() {}\n```");

        assert_eq!(
            rendered.first().map(String::as_str),
            Some("\x1b[2mcode (rust)\x1b[0m")
        );
        assert!(
            rendered
                .get(1)
                .is_some_and(|line| line.contains("\x1b[35mpub\x1b[0m"))
        );
        assert!(
            rendered
                .get(1)
                .is_some_and(|line| line.contains("\x1b[35mfn\x1b[0m"))
        );
    }

    #[test]
    fn renders_clickable_links_when_enabled() {
        let renderer = MarkdownRenderer::default();

        let rendered = renderer.render("[Sim](https://sim.dev)");

        assert_eq!(
            rendered,
            vec!["\x1b]8;;https://sim.dev\x1b\\Sim\x1b]8;;\x1b\\".to_string()]
        );
    }
}
