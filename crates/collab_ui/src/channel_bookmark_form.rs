use client::{AddBookmark, Bookmark, BookmarkId, ChannelId, UpdateBookmark};
use editor::Editor;
use gpui::{AppContext as _, Context, Entity, SharedString, Window};
use rpc::proto;

pub(crate) struct BookmarkForm {
    pub(crate) url_editor: Entity<Editor>,
    pub(crate) label_editor: Entity<Editor>,
    pub(crate) description_editor: Entity<Editor>,
    pub(crate) bookmark_type: proto::BookmarkType,
    pub(crate) mode: BookmarkFormMode,
    pub(crate) state: BookmarkFormState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BookmarkFormMode {
    Create,
    Edit { bookmark_id: BookmarkId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BookmarkFormState {
    Idle,
    Submitting,
    Failed(SharedString),
}

impl BookmarkForm {
    pub(crate) fn new_create(window: &mut Window, cx: &mut Context<impl Sized>) -> Self {
        Self::new(window, None, cx)
    }

    pub(crate) fn new_edit(
        bookmark: &Bookmark,
        window: &mut Window,
        cx: &mut Context<impl Sized>,
    ) -> Self {
        Self::new(window, Some(bookmark), cx)
    }

    fn new(
        window: &mut Window,
        bookmark: Option<&Bookmark>,
        cx: &mut Context<impl Sized>,
    ) -> Self {
        let bookmark_type = bookmark
            .map(|bookmark| bookmark.bookmark_type)
            .unwrap_or(proto::BookmarkType::BookmarkLink);
        let url_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text(bookmark_target_placeholder(bookmark_type), window, cx);
            if let Some(bookmark) = bookmark {
                editor.set_text(bookmark_target_value(bookmark), window, cx);
            }
            editor
        });
        let label_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Label", window, cx);
            if let Some(bookmark) = bookmark {
                editor.set_text(bookmark.label.to_string(), window, cx);
            }
            editor
        });
        let description_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Description", window, cx);
            if let Some(description) = bookmark.and_then(|bookmark| bookmark.description.as_ref()) {
                editor.set_text(description.to_string(), window, cx);
            }
            editor
        });
        let mode = match bookmark {
            Some(bookmark) => BookmarkFormMode::Edit {
                bookmark_id: bookmark.id,
            },
            None => BookmarkFormMode::Create,
        };

        Self {
            url_editor,
            label_editor,
            description_editor,
            bookmark_type,
            mode,
            state: BookmarkFormState::Idle,
        }
    }

    pub(crate) fn add_bookmark(
        &self,
        channel_id: ChannelId,
        cx: &mut Context<impl Sized>,
    ) -> Result<AddBookmark, SharedString> {
        let draft = BookmarkFormDraft {
            bookmark_type: self.bookmark_type,
            target: self.url_editor.read(cx).text(cx).trim().to_string(),
            label: self.label_editor.read(cx).text(cx).trim().to_string(),
            description: self
                .description_editor
                .read(cx)
                .text(cx)
                .trim()
                .to_string(),
        };
        draft.into_add_bookmark(channel_id)
    }

    pub(crate) fn update_bookmark(
        &self,
        channel_id: ChannelId,
        cx: &mut Context<impl Sized>,
    ) -> Result<UpdateBookmark, SharedString> {
        let BookmarkFormMode::Edit { bookmark_id } = self.mode else {
            return Err(SharedString::from("Select a bookmark to edit."));
        };
        let draft = BookmarkFormDraft {
            bookmark_type: self.bookmark_type,
            target: self.url_editor.read(cx).text(cx).trim().to_string(),
            label: self.label_editor.read(cx).text(cx).trim().to_string(),
            description: self
                .description_editor
                .read(cx)
                .text(cx)
                .trim()
                .to_string(),
        };
        let (label, description) = draft.label_and_description()?;
        Ok(UpdateBookmark {
            channel_id,
            bookmark_id,
            label,
            description,
        })
    }

    pub(crate) fn set_submitting(&mut self) {
        self.state = BookmarkFormState::Submitting;
    }

    pub(crate) fn set_error(&mut self, error: impl Into<SharedString>) {
        self.state = BookmarkFormState::Failed(error.into());
    }

    pub(crate) fn is_submitting(&self) -> bool {
        self.state == BookmarkFormState::Submitting
    }

    pub(crate) fn is_editing(&self) -> bool {
        matches!(self.mode, BookmarkFormMode::Edit { .. })
    }

    pub(crate) fn set_bookmark_type(
        &mut self,
        bookmark_type: proto::BookmarkType,
        window: &mut Window,
        cx: &mut Context<impl Sized>,
    ) {
        if self.is_editing() {
            return;
        }

        self.bookmark_type = bookmark_type;
        self.url_editor.update(cx, |editor, cx| {
            editor.set_placeholder_text(bookmark_target_placeholder(bookmark_type), window, cx);
        });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BookmarkFormDraft {
    bookmark_type: proto::BookmarkType,
    target: String,
    label: String,
    description: String,
}

impl BookmarkFormDraft {
    fn into_add_bookmark(self, channel_id: ChannelId) -> Result<AddBookmark, SharedString> {
        let (label, description) = self.label_and_description()?;
        let (url, file_id, message_id) = match self.bookmark_type {
            proto::BookmarkType::BookmarkLink => {
                if self.target.is_empty() {
                    return Err(SharedString::from("Enter a URL."));
                }
                if !is_supported_url(&self.target) {
                    return Err(SharedString::from("Use an http:// or https:// URL."));
                }
                (self.target, None, None)
            }
            proto::BookmarkType::BookmarkFile => {
                if self.target.is_empty() {
                    return Err(SharedString::from("Enter a file ID."));
                }
                (String::new(), Some(self.target), None)
            }
            proto::BookmarkType::BookmarkMessage => {
                if self.target.is_empty() {
                    return Err(SharedString::from("Enter a message ID."));
                }
                let message_id = self
                    .target
                    .parse()
                    .map_err(|_| SharedString::from("Use a numeric message ID."))?;
                (String::new(), None, Some(message_id))
            }
        };

        Ok(AddBookmark {
            channel_id,
            label,
            bookmark_type: self.bookmark_type,
            url,
            file_id,
            message_id,
            description,
        })
    }

    fn label_and_description(&self) -> Result<(String, Option<String>), SharedString> {
        if self.label.is_empty() {
            return Err(SharedString::from("Enter a label."));
        }

        Ok((
            self.label.clone(),
            (!self.description.is_empty()).then_some(self.description.clone()),
        ))
    }
}

fn is_supported_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

fn bookmark_target_placeholder(bookmark_type: proto::BookmarkType) -> &'static str {
    match bookmark_type {
        proto::BookmarkType::BookmarkLink => "URL",
        proto::BookmarkType::BookmarkFile => "File ID",
        proto::BookmarkType::BookmarkMessage => "Message ID",
    }
}

fn bookmark_target_value(bookmark: &Bookmark) -> String {
    match bookmark.bookmark_type {
        proto::BookmarkType::BookmarkLink => bookmark.url.to_string(),
        proto::BookmarkType::BookmarkFile => bookmark.file_id.clone().unwrap_or_default(),
        proto::BookmarkType::BookmarkMessage => bookmark
            .message_id
            .map(|message_id| message_id.to_string())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_builds_link_bookmark() {
        let bookmark = BookmarkFormDraft {
            bookmark_type: proto::BookmarkType::BookmarkLink,
            target: "https://sim.dev/runbook".to_string(),
            label: "Runbook".to_string(),
            description: "Deploy steps".to_string(),
        }
        .into_add_bookmark(ChannelId(7))
        .unwrap();

        assert_eq!(bookmark.channel_id, ChannelId(7));
        assert_eq!(bookmark.label, "Runbook");
        assert_eq!(bookmark.bookmark_type, proto::BookmarkType::BookmarkLink);
        assert_eq!(bookmark.url, "https://sim.dev/runbook");
        assert_eq!(bookmark.description.as_deref(), Some("Deploy steps"));
    }

    #[test]
    fn draft_builds_file_bookmark() {
        let bookmark = BookmarkFormDraft {
            bookmark_type: proto::BookmarkType::BookmarkFile,
            target: "file-123".to_string(),
            label: "Spec".to_string(),
            description: String::new(),
        }
        .into_add_bookmark(ChannelId(7))
        .unwrap();

        assert_eq!(bookmark.bookmark_type, proto::BookmarkType::BookmarkFile);
        assert_eq!(bookmark.url, "");
        assert_eq!(bookmark.file_id.as_deref(), Some("file-123"));
        assert_eq!(bookmark.message_id, None);
    }

    #[test]
    fn draft_builds_message_bookmark() {
        let bookmark = BookmarkFormDraft {
            bookmark_type: proto::BookmarkType::BookmarkMessage,
            target: "42".to_string(),
            label: "Decision".to_string(),
            description: String::new(),
        }
        .into_add_bookmark(ChannelId(7))
        .unwrap();

        assert_eq!(bookmark.bookmark_type, proto::BookmarkType::BookmarkMessage);
        assert_eq!(bookmark.url, "");
        assert_eq!(bookmark.file_id, None);
        assert_eq!(bookmark.message_id, Some(42));
    }

    #[test]
    fn draft_rejects_invalid_url() {
        let error = BookmarkFormDraft {
            bookmark_type: proto::BookmarkType::BookmarkLink,
            target: "sim.dev/runbook".to_string(),
            label: "Runbook".to_string(),
            description: String::new(),
        }
        .into_add_bookmark(ChannelId(7))
        .unwrap_err();

        assert_eq!(error, SharedString::from("Use an http:// or https:// URL."));
    }

    #[test]
    fn draft_rejects_empty_label() {
        let error = BookmarkFormDraft {
            bookmark_type: proto::BookmarkType::BookmarkLink,
            target: "https://sim.dev/runbook".to_string(),
            label: String::new(),
            description: String::new(),
        }
        .into_add_bookmark(ChannelId(7))
        .unwrap_err();

        assert_eq!(error, SharedString::from("Enter a label."));
    }

    #[test]
    fn draft_rejects_invalid_message_id() {
        let error = BookmarkFormDraft {
            bookmark_type: proto::BookmarkType::BookmarkMessage,
            target: "abc".to_string(),
            label: "Decision".to_string(),
            description: String::new(),
        }
        .into_add_bookmark(ChannelId(7))
        .unwrap_err();

        assert_eq!(error, SharedString::from("Use a numeric message ID."));
    }

    #[test]
    fn draft_builds_bookmark_update() {
        let draft = BookmarkFormDraft {
            bookmark_type: proto::BookmarkType::BookmarkLink,
            target: "https://sim.dev/runbook".to_string(),
            label: "Updated runbook".to_string(),
            description: String::new(),
        };
        let (label, description) = draft.label_and_description().unwrap();

        assert_eq!(label, "Updated runbook");
        assert_eq!(description, None);
    }
}
