use editor::{Editor, SelectionEffects};
use gpui::{App, Context, Entity, Focusable, IntoElement, Window, prelude::*};
use multi_buffer::MultiBufferOffset;
use ui::{IconButton, IconButtonShape, IconName, IconSize, Tooltip, prelude::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatKind {
    Bold,
    Italic,
    Code,
    Strikethrough,
    Blockquote,
    Link,
    CodeBlock,
    BulletList,
    NumberedList,
}

impl FormatKind {
    fn syntax(self) -> FormatSyntax {
        match self {
            Self::Bold => FormatSyntax::inline("**", "**", "text"),
            Self::Italic => FormatSyntax::inline("_", "_", "text"),
            Self::Code => FormatSyntax::inline("`", "`", "code"),
            Self::Strikethrough => FormatSyntax::inline("~~", "~~", "text"),
            Self::Blockquote => FormatSyntax::inline("> ", "", "quote"),
            Self::Link => FormatSyntax::inline("[", "](https://example.com)", "link text"),
            Self::CodeBlock => FormatSyntax::inline("```\n", "\n```", "code"),
            Self::BulletList => FormatSyntax::inline("- ", "", "list item"),
            Self::NumberedList => FormatSyntax::inline("1. ", "", "list item"),
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::Code => "Inline Code",
            Self::Strikethrough => "Strikethrough",
            Self::Blockquote => "Blockquote",
            Self::Link => "Link",
            Self::CodeBlock => "Code Block",
            Self::BulletList => "Bullet List",
            Self::NumberedList => "Numbered List",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Bold => IconName::FontWeight,
            Self::Italic => IconName::Font,
            Self::Code | Self::CodeBlock => IconName::Code,
            Self::Strikethrough => IconName::Dash,
            Self::Blockquote => IconName::Quote,
            Self::Link => IconName::Link,
            Self::BulletList | Self::NumberedList => IconName::ListTodo,
        }
    }

    fn id(self) -> usize {
        match self {
            Self::Bold => 0,
            Self::Italic => 1,
            Self::Code => 2,
            Self::Strikethrough => 3,
            Self::Blockquote => 4,
            Self::Link => 5,
            Self::CodeBlock => 6,
            Self::BulletList => 7,
            Self::NumberedList => 8,
        }
    }

    fn flag(self) -> FormatFlags {
        match self {
            Self::Bold => FormatFlags::BOLD,
            Self::Italic => FormatFlags::ITALIC,
            Self::Code => FormatFlags::CODE,
            Self::Strikethrough => FormatFlags::STRIKETHROUGH,
            Self::Blockquote => FormatFlags::BLOCKQUOTE,
            Self::Link => FormatFlags::LINK,
            Self::CodeBlock => FormatFlags::CODE_BLOCK,
            Self::BulletList => FormatFlags::BULLET_LIST,
            Self::NumberedList => FormatFlags::NUMBERED_LIST,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormatFlags(u16);

impl FormatFlags {
    pub const BOLD: Self = Self(1 << 0);
    pub const ITALIC: Self = Self(1 << 1);
    pub const CODE: Self = Self(1 << 2);
    pub const STRIKETHROUGH: Self = Self(1 << 3);
    pub const BLOCKQUOTE: Self = Self(1 << 4);
    pub const LINK: Self = Self(1 << 5);
    pub const CODE_BLOCK: Self = Self(1 << 6);
    pub const BULLET_LIST: Self = Self(1 << 7);
    pub const NUMBERED_LIST: Self = Self(1 << 8);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

pub struct FormattingToolbar {
    editor: Entity<Editor>,
    active_formats: FormatFlags,
}

impl FormattingToolbar {
    pub fn new(editor: Entity<Editor>) -> Self {
        Self {
            editor,
            active_formats: FormatFlags::default(),
        }
    }

    pub fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .id("channel-formatting-toolbar")
            .gap_1()
            .px_1()
            .py_0p5()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().editor_background)
            .children(FormatKind::ALL.map(|format_kind| self.button(format_kind)))
    }

    fn button(&self, format_kind: FormatKind) -> impl IntoElement {
        let editor = self.editor.clone();
        IconButton::new(("channel-format", format_kind.id()), format_kind.icon())
            .shape(IconButtonShape::Square)
            .icon_size(IconSize::Small)
            .icon_color(Color::Muted)
            .toggle_state(self.active_formats.contains(format_kind.flag()))
            .on_click(move |_, window, cx| {
                editor.update(cx, |editor, cx| {
                    apply_format(format_kind, editor, window, cx);
                });
                window.focus(&editor.focus_handle(cx), cx);
            })
            .tooltip(Tooltip::text(format_kind.tooltip()))
    }
}

impl FormatKind {
    const ALL: [Self; 9] = [
        Self::Bold,
        Self::Italic,
        Self::Code,
        Self::Strikethrough,
        Self::Blockquote,
        Self::Link,
        Self::CodeBlock,
        Self::BulletList,
        Self::NumberedList,
    ];
}

pub fn apply_format(
    format_kind: FormatKind,
    editor: &mut Editor,
    window: &mut Window,
    cx: &mut Context<Editor>,
) {
    if editor.read_only(cx) {
        return;
    }

    let buffer = editor.buffer().read(cx);
    let snapshot = buffer.snapshot(cx);
    let selections = editor
        .selections
        .all::<MultiBufferOffset>(&editor.display_snapshot(cx));

    let mut replacements = Vec::new();
    let mut next_selection_ranges = Vec::new();
    let mut offset_delta: isize = 0;

    for selection in selections {
        let range = selection.start..selection.end;
        let selected_text = snapshot.text_for_range(range.clone()).collect::<String>();
        let formatted = format_selection(format_kind, &selected_text);
        let adjusted_start = offset_with_delta(range.start, offset_delta);
        let adjusted_inner_start = MultiBufferOffset(adjusted_start.0 + formatted.selection.start);
        let adjusted_inner_end = MultiBufferOffset(adjusted_start.0 + formatted.selection.end);
        offset_delta += formatted.replacement.len() as isize - selected_text.len() as isize;
        replacements.push((range, formatted.replacement));
        next_selection_ranges.push(adjusted_inner_start..adjusted_inner_end);
    }

    editor.transact(window, cx, |editor, window, cx| {
        editor.edit(replacements, cx);
        editor.change_selections(SelectionEffects::no_scroll(), window, cx, |selections| {
            selections.select_ranges(next_selection_ranges);
        });
    });
}

fn offset_with_delta(offset: MultiBufferOffset, delta: isize) -> MultiBufferOffset {
    if delta.is_negative() {
        MultiBufferOffset(offset.0.saturating_sub(delta.unsigned_abs()))
    } else {
        MultiBufferOffset(offset.0.saturating_add(delta as usize))
    }
}

struct FormatSyntax {
    prefix: &'static str,
    suffix: &'static str,
    placeholder: &'static str,
}

impl FormatSyntax {
    fn inline(prefix: &'static str, suffix: &'static str, placeholder: &'static str) -> Self {
        Self {
            prefix,
            suffix,
            placeholder,
        }
    }
}

struct FormattedSelection {
    replacement: String,
    selection: std::ops::Range<usize>,
}

fn format_selection(format_kind: FormatKind, selected_text: &str) -> FormattedSelection {
    let syntax = format_kind.syntax();
    let inner_text = if selected_text.is_empty() {
        syntax.placeholder
    } else {
        selected_text
    };
    let selection_start = syntax.prefix.len();
    let selection_end = selection_start + inner_text.len();
    FormattedSelection {
        replacement: format!("{}{}{}", syntax.prefix, inner_text, syntax.suffix),
        selection: selection_start..selection_end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor::test::{assert_text_with_selections, select_ranges};
    use gpui::TestAppContext;
    use multi_buffer::MultiBuffer;
    use settings::SettingsStore;

    #[test]
    fn wraps_selected_text_with_markdown_markers() {
        let formatted = format_selection(FormatKind::Bold, "hello");

        assert_eq!(formatted.replacement, "**hello**");
        assert_eq!(&formatted.replacement[formatted.selection], "hello");
    }

    #[test]
    fn inserts_placeholder_when_selection_is_empty() {
        let formatted = format_selection(FormatKind::Link, "");

        assert_eq!(formatted.replacement, "[link text](https://example.com)");
        assert_eq!(&formatted.replacement[formatted.selection], "link text");
    }

    #[test]
    fn maps_all_format_kinds_to_markdown_syntax() {
        let cases = [
            (FormatKind::Bold, "**text**"),
            (FormatKind::Italic, "_text_"),
            (FormatKind::Code, "`code`"),
            (FormatKind::Strikethrough, "~~text~~"),
            (FormatKind::Blockquote, "> quote"),
            (FormatKind::Link, "[link text](https://example.com)"),
            (FormatKind::CodeBlock, "```\ncode\n```"),
            (FormatKind::BulletList, "- list item"),
            (FormatKind::NumberedList, "1. list item"),
        ];

        for (format_kind, expected) in cases {
            assert_eq!(format_selection(format_kind, "").replacement, expected);
        }
    }

    #[gpui::test]
    fn apply_format_wraps_each_editor_selection(cx: &mut TestAppContext) {
        init_test(cx);

        for format_kind in FormatKind::ALL {
            let editor = cx.add_window(|window, cx| {
                let buffer = MultiBuffer::build_simple("hello world", cx);
                Editor::for_multibuffer(buffer, None, window, cx)
            });

            editor
                .update(cx, |editor, window, cx| {
                    select_ranges(editor, "«ˇhello» world", window, cx);
                    apply_format(format_kind, editor, window, cx);
                    assert_text_with_selections(
                        editor,
                        &format!("{} world", marked_replacement(format_kind, "hello")),
                        cx,
                    );
                })
                .expect("window should be alive");
        }
    }

    #[gpui::test]
    fn apply_format_selects_placeholder_for_empty_selection(cx: &mut TestAppContext) {
        init_test(cx);

        for format_kind in FormatKind::ALL {
            let editor = cx.add_window(|window, cx| {
                let buffer = MultiBuffer::build_simple("", cx);
                Editor::for_multibuffer(buffer, None, window, cx)
            });

            editor
                .update(cx, |editor, window, cx| {
                    select_ranges(editor, "«ˇ»", window, cx);
                    apply_format(format_kind, editor, window, cx);
                    assert_text_with_selections(editor, &marked_replacement(format_kind, ""), cx);
                })
                .expect("window should be alive");
        }
    }

    #[test]
    fn formatting_shortcuts_dispatch_expected_actions() {
        let cases = [
            ("ctrl-b", "ToggleBold"),
            ("ctrl-i", "ToggleItalic"),
            ("ctrl-`", "ToggleCode"),
            ("ctrl-shift-k", "ToggleLink"),
        ];
        let bindings = super::super::channel_chat_key_bindings();

        for (keystroke, action_name) in cases {
            assert!(
                bindings.iter().any(|binding| {
                    binding.action().name().ends_with(action_name)
                        && binding
                            .keystrokes()
                            .first()
                            .is_some_and(|binding_keystroke| {
                                binding_keystroke.unparse() == keystroke
                            })
                }),
                "missing {keystroke} binding for {action_name}"
            );
        }
    }

    fn marked_replacement(format_kind: FormatKind, selected_text: &str) -> String {
        let formatted = format_selection(format_kind, selected_text);
        format!(
            "{}«{}ˇ»{}",
            &formatted.replacement[..formatted.selection.start],
            &formatted.replacement[formatted.selection.clone()],
            &formatted.replacement[formatted.selection.end..],
        )
    }

    fn init_test(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let settings_store = SettingsStore::test(cx);
            cx.set_global(settings_store);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            editor::init(cx);
        });
    }
}
