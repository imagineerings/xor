use client::{Client, Group, GroupStore, GroupStoreEvent, User, UserStore};
use collections::HashSet;
use editor::{Editor, EditorEvent};
use gpui::{
    App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, Render, SharedString,
    Subscription, Window, prelude::*,
};
use std::sync::Arc;
use ui::{
    Avatar, Button, ButtonStyle, Label, LabelSize, ListItem, ListItemSpacing, Tooltip, prelude::*,
};
use workspace::ModalView;

pub struct GroupManagement {
    client: Arc<Client>,
    group_store: Entity<GroupStore>,
    user_store: Entity<UserStore>,
    focus_handle: FocusHandle,
    name_editor: Entity<Editor>,
    display_name_editor: Entity<Editor>,
    member_search_editor: Entity<Editor>,
    selected_group_id: Option<u64>,
    selected_member_ids: HashSet<u64>,
    matching_users: Arc<[Arc<User>]>,
    saving: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl GroupManagement {
    pub fn new(
        client: Arc<Client>,
        group_store: Entity<GroupStore>,
        user_store: Entity<UserStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name_editor = Self::new_editor("Group name, e.g. eng-team", window, cx);
        let display_name_editor = Self::new_editor("Display name", window, cx);
        let member_search_editor = Self::new_editor("Search members", window, cx);
        let subscriptions = vec![
            cx.subscribe(&group_store, |_this, _, _: &GroupStoreEvent, cx| {
                cx.notify()
            }),
            cx.subscribe(&name_editor, |_this, _, event, cx| {
                if matches!(event, EditorEvent::BufferEdited) {
                    cx.notify();
                }
            }),
            cx.subscribe(&display_name_editor, |_this, _, event, cx| {
                if matches!(event, EditorEvent::BufferEdited) {
                    cx.notify();
                }
            }),
        ];

        Self {
            client,
            group_store,
            user_store,
            focus_handle: cx.focus_handle(),
            name_editor,
            display_name_editor,
            member_search_editor,
            selected_group_id: None,
            selected_member_ids: HashSet::default(),
            matching_users: Arc::from([]),
            saving: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn new_editor(
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<Editor> {
        cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text(placeholder, window, cx);
            editor
        })
    }

    fn select_group(&mut self, group_id: u64, cx: &mut Context<Self>) {
        self.selected_group_id = Some(group_id);
        self.selected_member_ids.clear();
        self.error = None;
        cx.notify();
    }

    fn search_members(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.member_search_editor.read(cx).text(cx);
        let search_users = self
            .user_store
            .update(cx, |store, cx| store.fuzzy_search_users(query, cx));
        self.error = None;
        cx.spawn_in(window, async move |this, cx| {
            let users = search_users.await;
            this.update_in(cx, |this, _window, cx| match users {
                Ok(users) => {
                    this.matching_users = users.into();
                    cx.notify();
                }
                Err(error) => {
                    this.error = Some(error.to_string().into());
                    cx.notify();
                }
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn toggle_initial_member(&mut self, user_id: u64, cx: &mut Context<Self>) {
        if !self.selected_member_ids.insert(user_id) {
            self.selected_member_ids.remove(&user_id);
        }
        cx.notify();
    }

    fn create_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let name = self.name_editor.read(cx).text(cx);
        let display_name = self.display_name_editor.read(cx).text(cx);
        if name.trim().is_empty() || display_name.trim().is_empty() {
            self.error = Some("Enter a group name and display name".into());
            cx.notify();
            return;
        }
        self.saving = true;
        self.error = None;
        let client = self.client.clone();
        let member_ids = self.selected_member_ids.iter().copied().collect();
        cx.spawn_in(window, async move |this, cx| {
            let result = client.create_group(name, display_name, member_ids).await;
            this.update_in(cx, |this, window, cx| match result {
                Ok(group) => {
                    this.selected_group_id = Some(group.id);
                    this.selected_member_ids.clear();
                    this.name_editor
                        .update(cx, |editor, cx| editor.set_text("", window, cx));
                    this.display_name_editor
                        .update(cx, |editor, cx| editor.set_text("", window, cx));
                    this.saving = false;
                    cx.notify();
                }
                Err(error) => {
                    this.saving = false;
                    this.error = Some(error.to_string().into());
                    cx.notify();
                }
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn update_members(
        &mut self,
        group_id: u64,
        add_user_ids: Vec<u64>,
        remove_user_ids: Vec<u64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving {
            return;
        }
        self.saving = true;
        self.error = None;
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = client
                .update_group_members(group_id, add_user_ids, remove_user_ids)
                .await;
            this.update_in(cx, |this, _window, cx| match result {
                Ok(_) => {
                    this.saving = false;
                    cx.notify();
                }
                Err(error) => {
                    this.saving = false;
                    this.error = Some(error.to_string().into());
                    cx.notify();
                }
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn leave_group(&mut self, group_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        self.saving = true;
        self.error = None;
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = client.leave_group(group_id).await;
            this.update_in(cx, |this, _window, cx| match result {
                Ok(()) => {
                    this.selected_group_id = None;
                    this.saving = false;
                    cx.notify();
                }
                Err(error) => {
                    this.saving = false;
                    this.error = Some(error.to_string().into());
                    cx.notify();
                }
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn delete_group(&mut self, group_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        self.saving = true;
        self.error = None;
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = client.delete_group(group_id).await;
            this.update_in(cx, |this, _window, cx| match result {
                Ok(()) => {
                    this.selected_group_id = None;
                    this.saving = false;
                    cx.notify();
                }
                Err(error) => {
                    this.saving = false;
                    this.error = Some(error.to_string().into());
                    cx.notify();
                }
            })?;
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn dismiss(&mut self, _: &menu::Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }

    fn current_user_id(&self, cx: &App) -> Option<u64> {
        self.user_store
            .read(cx)
            .current_user()
            .map(|user| user.legacy_id)
    }

    fn render_member(&self, user_id: u64, group: &Group, cx: &mut Context<Self>) -> ListItem {
        let user = self.user_store.read(cx).get_cached_user(user_id);
        let label = user
            .as_ref()
            .map(|user| user.github_login.clone())
            .unwrap_or_else(|| format!("User {user_id}").into());
        let is_admin = self.current_user_id(cx) == Some(group.admin_id);
        let group_id = group.id;
        ListItem::new(format!("group-member-{group_id}-{user_id}"))
            .inset(true)
            .spacing(ListItemSpacing::Sparse)
            .when_some(user, |this, user| {
                this.start_slot(Avatar::new(user.avatar_uri.clone()))
            })
            .child(Label::new(label))
            .when(is_admin && user_id != group.admin_id, |this| {
                this.end_slot(
                    Button::new(
                        format!("remove-group-member-{group_id}-{user_id}"),
                        "Remove",
                    )
                    .style(ButtonStyle::Subtle)
                    .disabled(self.saving)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.update_members(group_id, Vec::new(), vec![user_id], window, cx);
                    })),
                )
            })
    }
}

impl EventEmitter<DismissEvent> for GroupManagement {}
impl ModalView for GroupManagement {}

impl Focusable for GroupManagement {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GroupManagement {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let groups = self.group_store.read(cx).all_groups();
        let selected_group = self
            .selected_group_id
            .and_then(|group_id| self.group_store.read(cx).group(group_id));
        let current_user_id = self.current_user_id(cx);
        let matching_users = self.matching_users.clone();
        let saving = self.saving;

        h_flex()
            .key_context("GroupManagement")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::dismiss))
            .elevation_3(cx)
            .w(rems(60.))
            .h(rems(38.))
            .child(
                v_flex()
                    .w(rems(18.))
                    .h_full()
                    .overflow_hidden()
                    .p_3()
                    .gap_1()
                    .child(Label::new("Groups").size(LabelSize::Large))
                    .children(groups.into_iter().map(|group| {
                        let group_id = group.id;
                        Button::new(("select-group", group.id), group.display_name.clone())
                            .full_width()
                            .style(ButtonStyle::Subtle)
                            .selected_style(ButtonStyle::Filled)
                            .toggle_state(self.selected_group_id == Some(group.id))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_group(group_id, cx);
                            }))
                            .tooltip(Tooltip::text(format!(
                                "@{} ({} members)",
                                group.name,
                                group.member_ids.len()
                            )))
                    })),
            )
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .overflow_hidden()
                    .p_4()
                    .gap_3()
                    .when_some(self.error.clone(), |this, error| {
                        this.child(Label::new(error).color(Color::Error))
                    })
                    .when_some(selected_group.clone(), |this, group| {
                        let group_id = group.id;
                        let group_member_ids = group.member_ids.clone();
                        let matching_users = matching_users.clone();
                        let is_admin = current_user_id == Some(group.admin_id);
                        let is_member = current_user_id
                            .is_some_and(|user_id| group.member_ids.contains(&user_id));
                        this.child(Label::new(group.display_name.clone()).size(LabelSize::Large))
                            .child(
                                Label::new(format!(
                                    "@{} · {} members",
                                    group.name,
                                    group.member_ids.len()
                                ))
                                .color(Color::Muted),
                            )
                            .child(Label::new("Members").size(LabelSize::Small))
                            .children(
                                group
                                    .member_ids
                                    .iter()
                                    .copied()
                                    .map(|user_id| self.render_member(user_id, &group, cx)),
                            )
                            .when(is_admin, |this| {
                                this.child(Label::new("Add members").size(LabelSize::Small))
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(self.member_search_editor.clone())
                                            .child(
                                                Button::new("search-group-members", "Search")
                                                    .style(ButtonStyle::Subtle)
                                                    .disabled(self.saving)
                                                    .on_click(cx.listener(
                                                        |this, _, window, cx| {
                                                            this.search_members(window, cx)
                                                        },
                                                    )),
                                            ),
                                    )
                                    .children(
                                        matching_users
                                            .iter()
                                            .filter(move |user| {
                                                !group_member_ids.contains(&user.legacy_id)
                                            })
                                            .cloned()
                                            .map(|user| {
                                                let user_id = user.legacy_id;
                                                ListItem::new(format!(
                                                    "add-group-member-{group_id}-{user_id}"
                                                ))
                                                .inset(true)
                                                .spacing(ListItemSpacing::Sparse)
                                                .start_slot(Avatar::new(user.avatar_uri.clone()))
                                                .child(Label::new(user.github_login.clone()))
                                                .end_slot(
                                                    Button::new(
                                                        format!("add-member-{group_id}-{user_id}"),
                                                        "Add",
                                                    )
                                                    .style(ButtonStyle::Subtle)
                                                    .disabled(saving)
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            this.update_members(
                                                                group_id,
                                                                vec![user_id],
                                                                Vec::new(),
                                                                window,
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                                )
                                            }),
                                    )
                                    .child(
                                        Button::new(("delete-group", group_id), "Delete group")
                                            .style(ButtonStyle::Subtle)
                                            .disabled(saving)
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.delete_group(group_id, window, cx)
                                            })),
                                    )
                            })
                            .when(!is_admin && is_member, |this| {
                                this.child(
                                    Button::new(("leave-group", group_id), "Leave group")
                                        .style(ButtonStyle::Subtle)
                                        .disabled(saving)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.leave_group(group_id, window, cx)
                                        })),
                                )
                            })
                    })
                    .when(selected_group.is_none(), |this| {
                        this.child(Label::new("Create a group").size(LabelSize::Large))
                            .child(self.name_editor.clone())
                            .child(self.display_name_editor.clone())
                            .child(Label::new("Initial members").size(LabelSize::Small))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(self.member_search_editor.clone())
                                    .child(
                                        Button::new("search-initial-members", "Search")
                                            .style(ButtonStyle::Subtle)
                                            .disabled(saving)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.search_members(window, cx)
                                            })),
                                    ),
                            )
                            .children(matching_users.iter().map(|user| {
                                let user_id = user.legacy_id;
                                Button::new(
                                    ("initial-group-member", user.legacy_id),
                                    user.github_login.clone(),
                                )
                                .style(ButtonStyle::Subtle)
                                .selected_style(ButtonStyle::Filled)
                                .toggle_state(self.selected_member_ids.contains(&user_id))
                                .on_click(cx.listener(
                                    move |this, _, _, cx| this.toggle_initial_member(user_id, cx),
                                ))
                            }))
                            .child(
                                Button::new("create-group", "Create group")
                                    .disabled(saving)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.create_group(window, cx)
                                    })),
                            )
                    }),
            )
    }
}
