use super::{
    markdown_style::channel_chat_markdown_style,
    sanitize::{sanitize_channel_markdown, trusted_channel_url},
};
use gpui::{AnyElement, App, Entity, ImageSource, Resource, SharedString, SharedUri, Window};
use markdown::{Markdown, MarkdownElement, MarkdownOptions};
use ui::prelude::*;

pub(super) struct MessageBody {
    source: String,
    truncated: bool,
    markdown: Entity<Markdown>,
}

impl MessageBody {
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
                    Label::new("Message truncated before rendering")
                        .size(LabelSize::XSmall)
                        .color(Color::Muted),
                )
            })
            .into_any_element()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(super) fn markdown_for_test(&self) -> Entity<Markdown> {
        self.markdown.clone()
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
    use gpui::{Context, IntoElement, Render, TestAppContext, Window, div};
    use markdown::parser::{MarkdownEvent, MarkdownTag};
    use settings::SettingsStore;

    #[test]
    fn resolves_only_http_remote_images() {
        assert!(resolve_remote_image("https://example.com/image.png").is_some());
        assert!(resolve_remote_image("http://example.com/image.png").is_some());
        assert!(resolve_remote_image("data:image/png;base64,AAAA").is_none());
        assert!(resolve_remote_image("file:///tmp/image.png").is_none());
    }

    #[gpui::test]
    fn malformed_markdown_body_can_be_created(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let body = MessageBody::new(
                "**unclosed [bad](javascript:alert(1)) <script>".to_string(),
                cx,
            );

            assert_eq!(
                body.source(),
                "**unclosed [bad](javascript:alert(1)) <script>"
            );
        });
    }

    #[gpui::test]
    fn renders_markdown_construct_text(cx: &mut TestAppContext) {
        let rendered = render_message_body_text(
            "# Heading\n\
            **bold** _italic_ ~~strike~~ `code`\n\
            > quote\n\
            - item\n\
            1. numbered\n\
            [link](https://example.com)",
            cx,
        );

        for expected in [
            "Heading", "bold", "italic", "strike", "code", "quote", "item", "numbered", "link",
        ] {
            assert!(
                rendered.contains(expected),
                "rendered text {rendered:?} should contain {expected:?}"
            );
        }
    }

    #[gpui::test]
    fn parses_markdown_constructs_used_by_message_rendering(cx: &mut TestAppContext) {
        let events = parsed_events_for(
            "# Heading\n\
            **bold** _italic_ ~~strike~~ `code`\n\
            > quote\n\
            - item\n\
            1. numbered\n\
            [link](https://example.com)\n\
            ![alt](https://example.com/image.png)\n\
            ```rust\nlet value = 1;\n```",
            cx,
        );

        assert!(
            events.iter().any(|event| {
                matches!(event, MarkdownEvent::Start(MarkdownTag::Heading { .. }))
            })
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, MarkdownEvent::Start(MarkdownTag::Strong)))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, MarkdownEvent::Start(MarkdownTag::Emphasis)))
        );
        assert!(
            events
                .iter()
                .any(|event| { matches!(event, MarkdownEvent::Start(MarkdownTag::Strikethrough)) })
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, MarkdownEvent::Code))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, MarkdownEvent::Start(MarkdownTag::BlockQuote(_))))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, MarkdownEvent::Start(MarkdownTag::List(_))))
        );
        assert!(
            events.iter().any(|event| {
                matches!(event, MarkdownEvent::Start(MarkdownTag::CodeBlock { .. }))
            })
        );
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MarkdownEvent::Start(MarkdownTag::Link { dest_url, .. }) if dest_url.as_ref() == "https://example.com"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                MarkdownEvent::Start(MarkdownTag::Image { dest_url, .. }) if dest_url.as_ref() == "https://example.com/image.png"
            )
        }));
    }

    #[gpui::test]
    fn plain_text_renders_unchanged(cx: &mut TestAppContext) {
        let rendered = render_message_body_text("plain text with no markdown", cx);

        assert_eq!(rendered, "plain text with no markdown");
    }

    #[gpui::test]
    fn malformed_markdown_renders_best_effort(cx: &mut TestAppContext) {
        let rendered = render_message_body_text("**unclosed > quote [bad", cx);

        assert!(rendered.contains("unclosed"));
        assert!(rendered.contains("quote"));
        assert!(rendered.contains("[bad"));
    }

    #[gpui::test]
    fn unsafe_protocol_links_render_as_inert_text(cx: &mut TestAppContext) {
        let rendered = render_message_body_text(
            "[click](javascript:alert(1)) ![alt](data:image/png;base64,AAAA)",
            cx,
        );

        assert!(rendered.contains("click"));
        assert!(rendered.contains("alt"));
        assert!(!rendered.contains("javascript"));
        assert!(!rendered.contains("data:image"));
    }

    #[gpui::test]
    fn long_messages_are_truncated_before_rendering(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let body = MessageBody::new(
                "a".repeat(super::super::sanitize::MAX_CHANNEL_MARKDOWN_SOURCE_LEN + 1),
                cx,
            );

            assert!(body.truncated);
        });
    }

    fn render_message_body_text(source: &str, cx: &mut TestAppContext) -> String {
        struct TestWindow;

        impl Render for TestWindow {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
            }
        }

        init_test(cx);
        let body = cx.update(|cx| MessageBody::new(source.to_string(), cx));
        let (_, cx) = cx.add_window_view(|_, _| TestWindow);
        cx.run_until_parked();
        MarkdownElement::rendered_text(body.markdown, cx, channel_chat_markdown_style)
    }

    fn parsed_events_for(source: &str, cx: &mut TestAppContext) -> Vec<MarkdownEvent> {
        let body = cx.update(|cx| MessageBody::new(source.to_string(), cx));
        cx.run_until_parked();
        cx.update(|cx| {
            body.markdown
                .read(cx)
                .parsed_markdown()
                .events()
                .iter()
                .map(|(_, event)| event.clone())
                .collect()
        })
    }

    fn init_test(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let settings_store = SettingsStore::test(cx);
            cx.set_global(settings_store);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
        });
    }
}
