use client::Bookmark;
use gpui::{App, IntoElement, ParentElement as _, RenderOnce, SharedString, Window, div};
use rpc::proto;
use std::rc::Rc;
use ui::{Button, ButtonSize, ButtonStyle, Color, Icon, IconName, Label, Tooltip, prelude::*};

const COLLAPSED_BOOKMARK_LIMIT: usize = 5;
type EditBookmarkHandler = Rc<dyn Fn(Bookmark, &mut Window, &mut App)>;
type DeleteBookmarkHandler = Rc<dyn Fn(Bookmark, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct ChannelBookmarkBar {
    bookmarks: Vec<Bookmark>,
    expanded: bool,
    on_edit: Option<EditBookmarkHandler>,
    on_delete: Option<DeleteBookmarkHandler>,
}

impl ChannelBookmarkBar {
    pub fn new(bookmarks: Vec<Bookmark>, expanded: bool) -> Self {
        Self {
            bookmarks,
            expanded,
            on_edit: None,
            on_delete: None,
        }
    }

    pub fn on_edit(mut self, on_edit: impl Fn(Bookmark, &mut Window, &mut App) + 'static) -> Self {
        self.on_edit = Some(Rc::new(on_edit));
        self
    }

    pub fn on_delete(
        mut self,
        on_delete: impl Fn(Bookmark, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_delete = Some(Rc::new(on_delete));
        self
    }

    pub fn has_overflow(&self) -> bool {
        self.bookmarks.len() > COLLAPSED_BOOKMARK_LIMIT
    }
}

impl RenderOnce for ChannelBookmarkBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.bookmarks.len();
        let visible_count = if self.expanded {
            count
        } else {
            count.min(COLLAPSED_BOOKMARK_LIMIT)
        };
        let hidden_count = count.saturating_sub(visible_count);

        div().when(count > 0, |this| {
            this.border_b_1()
                .border_color(cx.theme().colors().border)
                .px_3()
                .py_2()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            Label::new(format!("Bookmarks ({count})"))
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        )
                        .when(hidden_count > 0, |this| {
                            this.child(
                                Label::new(format!("+{hidden_count}"))
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted),
                            )
                        }),
                )
                .child(
                    h_flex().gap_2().flex_wrap().children(
                        self.bookmarks
                            .into_iter()
                            .take(visible_count)
                            .map(|bookmark| {
                                render_bookmark(
                                    bookmark,
                                    self.on_edit.clone(),
                                    self.on_delete.clone(),
                                )
                            }),
                    ),
                )
        })
    }
}

fn render_bookmark(
    bookmark: Bookmark,
    on_edit: Option<EditBookmarkHandler>,
    on_delete: Option<DeleteBookmarkHandler>,
) -> impl IntoElement {
    let id = bookmark.id.to_proto();
    let label = bookmark.label.clone();
    let description = bookmark.description.clone();
    let bookmark_type = bookmark.bookmark_type;
    let url = bookmark.url.to_string();
    let bookmark_for_edit = bookmark.clone();
    let bookmark_for_delete = bookmark.clone();

    h_flex()
        .gap_1()
        .items_center()
        .child(
            Button::new(("channel-bookmark", id), label.clone())
                .style(ButtonStyle::Subtle)
                .size(ButtonSize::Compact)
                .start_icon(Icon::new(bookmark_icon(bookmark_type)))
                .truncate(true)
                .tooltip(Tooltip::text(bookmark_tooltip(
                    label,
                    description,
                    bookmark_type,
                )))
                .on_click(move |_, _, cx| {
                    if bookmark_type == proto::BookmarkType::BookmarkLink {
                        cx.open_url(&url);
                    }
                }),
        )
        .when_some(on_edit, |this, on_edit| {
            this.child(
                IconButton::new(("edit-channel-bookmark", id), IconName::Pencil)
                    .icon_size(IconSize::XSmall)
                    .icon_color(Color::Muted)
                    .tooltip(Tooltip::text("Edit bookmark"))
                    .on_click(move |_, window, cx| {
                        on_edit(bookmark_for_edit.clone(), window, cx);
                    }),
            )
        })
        .when_some(on_delete, |this, on_delete| {
            this.child(
                IconButton::new(("delete-channel-bookmark", id), IconName::Trash)
                    .icon_size(IconSize::XSmall)
                    .icon_color(Color::Muted)
                    .tooltip(Tooltip::text("Delete bookmark"))
                    .on_click(move |_, window, cx| {
                        on_delete(bookmark_for_delete.clone(), window, cx);
                    }),
            )
        })
}

fn bookmark_icon(bookmark_type: proto::BookmarkType) -> IconName {
    match bookmark_type {
        proto::BookmarkType::BookmarkLink => IconName::Link,
        proto::BookmarkType::BookmarkFile => IconName::File,
        proto::BookmarkType::BookmarkMessage => IconName::Chat,
    }
}

fn bookmark_tooltip(
    label: SharedString,
    description: Option<SharedString>,
    bookmark_type: proto::BookmarkType,
) -> SharedString {
    let bookmark_type = match bookmark_type {
        proto::BookmarkType::BookmarkLink => "Link",
        proto::BookmarkType::BookmarkFile => "File",
        proto::BookmarkType::BookmarkMessage => "Message",
    };
    match description {
        Some(description) if !description.is_empty() => {
            SharedString::from(format!("{bookmark_type}: {label}\n{description}"))
        }
        _ => SharedString::from(format!("{bookmark_type}: {label}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone as _, Utc};
    use client::{BookmarkId, ChannelId};

    #[test]
    fn collapsed_bar_reports_overflow_after_five_bookmarks() {
        let bookmarks = (0..6)
            .map(|index| Bookmark {
                id: BookmarkId(index),
                channel_id: ChannelId(1),
                label: SharedString::from(format!("Bookmark {index}")),
                description: None,
                bookmark_type: proto::BookmarkType::BookmarkLink,
                url: SharedString::from("https://sim.dev"),
                file_id: None,
                message_id: None,
                created_by: 1,
                created_at: Utc.timestamp_millis_opt(0).unwrap(),
                sort_order: index as u32,
            })
            .collect();

        assert!(ChannelBookmarkBar::new(bookmarks, false).has_overflow());
    }
}
