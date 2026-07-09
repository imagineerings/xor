use client::{AddBookmark, Bookmark, BookmarkId, ChannelId, UpdateBookmark};
use editor::Editor;
use gpui::{AppContext as _, Context, Entity, SharedString, Window};
use rpc::proto;

pub(crate) struct BookmarkForm {
    pub(crate) url_editor: Entity<Editor>,
    pub(crate) label_editor: Entity<Editor>,
    pub(crate) description_editor: Entity<Editor>,
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
        let url_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("URL", window, cx);
            if let Some(bookmark) = bookmark {
                editor.set_text(bookmark.url.to_string(), window, cx);
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
            url: self.url_editor.read(cx).text(cx).trim().to_string(),
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
            url: self.url_editor.read(cx).text(cx).trim().to_string(),
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BookmarkFormDraft {
    url: String,
    label: String,
    description: String,
}

impl BookmarkFormDraft {
    fn into_add_bookmark(self, channel_id: ChannelId) -> Result<AddBookmark, SharedString> {
        let (label, description) = self.label_and_description()?;
        if self.url.is_empty() {
            return Err(SharedString::from("Enter a URL."));
        }
        if !is_supported_url(&self.url) {
            return Err(SharedString::from("Use an http:// or https:// URL."));
        }

        Ok(AddBookmark {
            channel_id,
            label,
            bookmark_type: proto::BookmarkType::BookmarkLink,
            url: self.url,
            file_id: None,
            message_id: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_builds_link_bookmark() {
        let bookmark = BookmarkFormDraft {
            url: "https://sim.dev/runbook".to_string(),
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
    fn draft_rejects_invalid_url() {
        let error = BookmarkFormDraft {
            url: "sim.dev/runbook".to_string(),
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
            url: "https://sim.dev/runbook".to_string(),
            label: String::new(),
            description: String::new(),
        }
        .into_add_bookmark(ChannelId(7))
        .unwrap_err();

        assert_eq!(error, SharedString::from("Enter a label."));
    }

    #[test]
    fn draft_builds_bookmark_update() {
        let draft = BookmarkFormDraft {
            url: "https://sim.dev/runbook".to_string(),
            label: "Updated runbook".to_string(),
            description: String::new(),
        };
        let (label, description) = draft.label_and_description().unwrap();

        assert_eq!(label, "Updated runbook");
        assert_eq!(description, None);
    }
}
