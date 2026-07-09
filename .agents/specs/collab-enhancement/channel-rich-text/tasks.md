# Implementation Plan: Channel Rich Text

## Overview

Render `ChannelMessage.body` as Markdown using the existing `markdown` crate, and enhance the message composition area with a formatting toolbar and source/preview toggle. All changes are client-side — the `ChannelMessage` proto, server handlers, and database remain untouched.

The work is ordered to build incrementally: first the rendering pipeline (Markdown output in the message list), then compose-area enhancements (FormattingToolbar, Source/Preview toggle), then safety hardening (sanitization, error states), and finally cross-cutting tests.

**Scope**: `collab_ui` (channel chat rendering, ComposeArea, FormattingToolbar, PreviewPane), `markdown` (safety helpers if missing).

---

## Tasks

- [x] 1. Create the channel message rendering module
  - [x] 1.1 Define a `ChannelMessageBubble` component (or function) that takes a `ChannelMessage` and returns rendered GPUI elements via `MarkdownElement`. Use `MarkdownStyle::themed(MarkdownFont::Editor, window, cx)` for a chat-appropriate style.
  - [x] 1.2 Hook `ChannelMessageBubble` into the channel message list so each message body is rendered through the `markdown` crate instead of as a plain `SharedString`.
  - [x] 1.3 Verify plain text (no Markdown syntax) renders identically to before — the `markdown` crate passes through plain text unchanged.
  - _Requirements: 1.1, 1.5_
  - _writes: `collab_ui/src/channel_chat/message_bubble.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab_ui resolves_only_http_remote_images --features test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_chat_view_live_insert_and_send_states --features test-support`_

- [x] 2. Bind `ChannelMessage` `MarkdownElement` events (links, images)
  - [x] 2.1 Wire `on_url_click` on the `MarkdownElement` to open external links in the OS browser via `open::that()` (or similar shell command).
  - [x] 2.2 Set `render_links: true` and `render_images: true` in the render options. Wire `image_resolver` to show external images with a click-to-load guard.
  - [x] 2.3 Test that clicking a link in a rendered message opens the external browser.
  - _Requirements: 1.1, 1.4_
  - _writes: `collab_ui/src/channel_chat/message_bubble.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_chat_view_live_insert_and_send_states --features test-support`_

- [x] 3. Extract a shared Markdown style for channel chat
  - [x] 3.1 Add a `MarkdownFont::Chat` variant (or reuse `MarkdownFont::Editor`) and create a `channel_chat_markdown_style(window, cx) -> MarkdownStyle` helper that applies chat-appropriate base text size, link colors, and code block styling.
  - [x] 3.2 Consume the helper in both the message bubble renderer and the preview pane (added later).
  - _Requirements: 1.1_
  - _writes: `collab_ui/src/channel_chat/markdown_style.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_chat_view_live_insert_and_send_states --features test-support`_

- [x] 4. Build the `FormattingToolbar` component
  - [x] 4.1 Define `FormatKind` enum (`Bold`, `Italic`, `Code`, `Strikethrough`, `Blockquote`, `Link`, `CodeBlock`, `BulletList`, `NumberedList`) with the Markdown syntax each maps to.
  - [x] 4.2 Define `FormatFlags` bitmask for tracking active formatting at cursor position.
  - [x] 4.3 Implement `FormattingToolbar` as a GPUI component with styled icon buttons for bold, italic, inline code, link, and blockquote.
  - [x] 4.4 Implement `apply_format(format_kind, editor, window, cx)` — inserts/wraps the appropriate Markdown markers around the current selection (or at cursor with placeholder text between markers when nothing is selected).
  - [x] 4.5 Render the toolbar as a floating row above the compose editor, visible only when the editor is focused.
  - _Requirements: 1.2_
  - _writes: `collab_ui/src/channel_chat/formatting_toolbar.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab_ui formatting_toolbar --features test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_chat_view_live_insert_and_send_states --features test-support`_

- [x] 5. Wire keyboard shortcuts for formatting actions
  - [x] 5.1 Register `actions!` for `ToggleBold`, `ToggleItalic`, `ToggleCode`, `ToggleLink`, `ToggleBlockquote` in the compose area.
  - [x] 5.2 Bind Ctrl+B → bold, Ctrl+I → italic, Ctrl+` → code, Ctrl+Shift+K → link.
  - [x] 5.3 Each action calls `FormattingToolbar::apply_format` with the corresponding `FormatKind`.
  - _Requirements: 1.2_
  - _writes: `collab_ui/src/channel_chat/formatting_toolbar.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab_ui formatting --features test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_chat_view_live_insert_and_send_states --features test-support`_

- [x] 6. Add the Source/Preview toggle to the ComposeArea
  - [x] 6.1 Define `ComposeMode` enum: `Source` and `Preview`.
  - [x] 6.2 Add a toggle button (icon: eye / pencil) in the compose area header that switches between modes.
  - [x] 6.3 In `Preview` mode, hide the text editor and show a live-rendered `MarkdownElement` of the current draft text.
  - [x] 6.4 When switching back to `Source` mode, preserve the draft content unchanged.
  - [x] 6.5 Bind Ctrl+Shift+P as a keyboard shortcut for toggling modes.
  - _Requirements: 1.3_
  - _writes: `collab_ui/src/channel_chat/compose_area.rs`_
  - _validated: `CARGO_INCREMENTAL=0 cargo test -p collab_ui channel_chat --features test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab test_channel_chat_view_live_insert_and_send_states --features test-support`_

- [ ] 7. Harden Markdown rendering for safety
  - [ ] 7.1 Add a `sanitize_markdown_html(input: &str) -> String` function that strips or escapes raw HTML tags and `javascript:` / `data:` / `file:` protocol URLs before passing the string to the markdown renderer.
  - [ ] 7.2 Ensure `render_markdown` is never called with `parse_html: true` for user-generated channel content.
  - [ ] 7.3 Add a configurable max length check (e.g., 10K chars) — truncate before rendering and show a "message too long" indicator if exceeded.
  - [ ] 7.4 Verify that malformed Markdown (unclosed fences, stray punctuation) renders best-effort without panicking.
  - _Requirements: 1.4_
  - _writes: `collab_ui/src/channel_chat/sanitize.rs`_

- [ ] 8. Add unit tests for formatting toolbar
  - [ ] 8.1 Test each `FormatKind::apply_format` on a mock `Editor`: verify correct Markdown wrapping of selected text.
  - [ ] 8.2 Test empty-selection case: markers inserted with cursor placed between them.
  - [ ] 8.3 Test keyboard shortcut dispatch invokes the correct `FormatKind`.
  - _Requirements: 1.2_
  - _writes: `collab_ui/src/channel_chat/formatting_toolbar.rs`_ (tests module)

- [ ] 9. Add unit tests for Markdown rendering in channel messages
  - [ ] 9.1 Test `ChannelMessageBubble` rendering of each Markdown construct (bold, italic, code, blockquote, list, heading, link, image).
  - [ ] 9.2 Test plain text passthrough — messages with no Markdown syntax produce identical output to current plain-text rendering.
  - [ ] 9.3 Test malformed Markdown (unclosed `**`, stray `>`) renders without panic and with best-effort output.
  - [ ] 9.4 Test protocol-URL sanitization (`javascript:`, `data:`, `file:`) — links are stripped or rendered as inert text.
  - [ ] 9.5 Test max-length truncation: messages >10K chars are truncated before rendering.
  - _Requirements: 1.1, 1.4, 1.5_
  - _writes: `collab_ui/src/channel_chat/message_bubble.rs`_ (tests), `collab_ui/src/channel_chat/sanitize.rs`_ (tests)

- [ ] 10. Add integration tests for UI composition
  - [ ] 10.1 Create a test that opens a channel, types Markdown in the compose editor, switches to preview mode, and verifies the `MarkdownElement` is visible.
  - [ ] 10.2 Verify that toggling back to source mode preserves the raw Markdown text.
  - [ ] 10.3 Verify the formatting toolbar appears when the editor is focused and disappears when blurred.
  - [ ] 10.4 Verify a message with rich Markdown renders correctly in the message list after being sent (simulate via `ChannelMessageSent` event).
  - _Requirements: 1.2, 1.3_
  - _writes: `collab_ui/src/channel_chat/compose_area.rs`_ (tests)

- [ ] 11. Add property-based tests for rendering stability
  - [ ] 11.1 Use `proptest` (or a simple fuzz loop) to generate random Markdown-like strings and feed them through the rendering pipeline.
  - [ ] 11.2 Assert no panic occurs for any input — the renderer must produce valid GPUI elements or fall back gracefully.
  - [ ] 11.3 Assert sanitization invariants: no rendered output ever contains raw `<script>`, `javascript:` links, or unescaped HTML tags.
  - _Requirements: 1.4_
  - _writes: `collab_ui/src/channel_chat/sanitize.rs`_ (fuzz tests)
