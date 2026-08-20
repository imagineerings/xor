use crate::{
    ItemHandle, MultiWorkspace, Pane, SidebarSide, ToggleWorkspaceSidebar,
    sidebar_side_context_menu,
};
#[cfg(feature = "multiplayer-tools")]
use crate::{
    collaborative_participants::{
        CollaborativeExecutionPhase, CollaborativeParticipantProviderState,
    },
    collaborative_status::{
        CollaborativeStatus, CollaborativeStatusProjection, CollaborativeTaskPhase,
    },
};
use gpui::{
    Anchor, AnyView, App, Context, Decorations, Entity, IntoElement, ParentElement, Render,
    SharedString, Styled, Subscription, WeakEntity, Window,
};
#[cfg(feature = "multiplayer-tools")]
use gpui::{FocusHandle, Role};
use project::Project;
use settings::{SettingsContent, update_settings_file};
use std::{any::TypeId, sync::Arc};
use theme::CLIENT_SIDE_DECORATION_ROUNDING;
use ui::{ContextMenu, Divider, IconPosition, Indicator, Tooltip, prelude::*, right_click_menu};

/// Describes how a status-bar item can be hidden by the user.
///
/// Every [`StatusItemView`] must either provide this (so that the user gets a
/// "Hide Button" entry in the right-click menu) or explicitly return `None`
/// to opt out. Returning `None` should be reserved for items that are
/// already conditional on some other setting exposed elsewhere (e.g., the
/// activity indicator, which disappears on its own once there's no work to
/// display).
#[derive(Clone)]
pub struct HideStatusItem {
    hide: Arc<dyn Fn(&mut SettingsContent) + Send + Sync>,
}

impl HideStatusItem {
    pub fn new(hide: impl Fn(&mut SettingsContent) + Send + Sync + 'static) -> Self {
        Self {
            hide: Arc::new(hide),
        }
    }

    /// Persists the hide by updating the user settings file.
    pub fn apply(&self, cx: &App) {
        let hide = self.hide.clone();
        let fs = <dyn fs::Fs>::global(cx);
        update_settings_file(fs, cx, move |settings, _cx| (hide)(settings));
    }
}

pub trait StatusItemView: Render {
    /// Event callback that is triggered when the active pane item changes.
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn crate::ItemHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    );

    /// Returns metadata describing how this item can be hidden from the
    /// status bar by writing to the user settings file.
    ///
    /// Implementors that return `None` must be inherently conditional on
    /// another user-exposed setting; otherwise, they should return `Some` so
    /// that the status bar can show a "Hide Button" entry in its
    /// right-click menu.
    fn hide_setting(&self, cx: &App) -> Option<HideStatusItem>;
}

trait StatusItemViewHandle: Send {
    fn to_any(&self) -> AnyView;
    fn set_active_pane_item(
        &self,
        active_pane_item: Option<&dyn ItemHandle>,
        window: &mut Window,
        cx: &mut App,
    );
    fn item_type(&self) -> TypeId;
    fn hide_setting(&self, cx: &App) -> Option<HideStatusItem>;
}

#[derive(Default)]
struct SidebarStatus {
    open: bool,
    side: SidebarSide,
    has_notifications: bool,
    show_toggle: bool,
}

impl SidebarStatus {
    fn query(multi_workspace: &Option<WeakEntity<MultiWorkspace>>, cx: &App) -> Self {
        multi_workspace
            .as_ref()
            .and_then(|mw| mw.upgrade())
            .map(|mw| {
                let mw = mw.read(cx);
                let enabled = mw.multi_workspace_enabled(cx);
                Self {
                    open: mw.sidebar_open() && enabled,
                    side: mw.sidebar_side(cx),
                    has_notifications: mw.sidebar_has_notifications(cx),
                    show_toggle: enabled,
                }
            })
            .unwrap_or_default()
    }
}

pub struct StatusBar {
    left_items: Vec<Box<dyn StatusItemViewHandle>>,
    right_items: Vec<Box<dyn StatusItemViewHandle>>,
    active_pane: Entity<Pane>,
    #[cfg(feature = "multiplayer-tools")]
    project: Entity<Project>,
    multi_workspace: Option<WeakEntity<MultiWorkspace>>,
    #[cfg(feature = "multiplayer-tools")]
    collaborative_participants: Option<CollaborativeParticipantProviderState>,
    #[cfg(feature = "multiplayer-tools")]
    collaborative_status_focus_handle: FocusHandle,
    _observe_active_pane: Subscription,
    #[cfg(feature = "multiplayer-tools")]
    _observe_project: Subscription,
    _observe_git_store: Subscription,
}

impl Render for StatusBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = SidebarStatus::query(&self.multi_workspace, cx);

        h_flex()
            .w_full()
            .justify_between()
            .gap(DynamicSpacing::Base08.rems(cx))
            .p(DynamicSpacing::Base04.rems(cx))
            .bg(cx.theme().colors().status_bar_background)
            .map(|el| match window.window_decorations() {
                Decorations::Server => el,
                Decorations::Client { tiling, .. } => el
                    .when(
                        !(tiling.bottom || tiling.right)
                            && !(sidebar.open && sidebar.side == SidebarSide::Right),
                        |el| el.rounded_br(CLIENT_SIDE_DECORATION_ROUNDING),
                    )
                    .when(
                        !(tiling.bottom || tiling.left)
                            && !(sidebar.open && sidebar.side == SidebarSide::Left),
                        |el| el.rounded_bl(CLIENT_SIDE_DECORATION_ROUNDING),
                    )
                    // This border is to avoid a transparent gap in the rounded corners
                    .mb(px(-1.))
                    .mt({
                        #[cfg(target_os = "linux")]
                        let needs_gap_fix = {
                            // Running on Wayland and using some scaling levels other than 100% causes a
                            // 1px gap above the status bar; adding a margin avoids this.
                            gpui::guess_compositor() == "Wayland" && window.scale_factor() != 1.0
                        };
                        #[cfg(not(target_os = "linux"))]
                        let needs_gap_fix = false;
                        if needs_gap_fix { px(-1.) } else { px(0.) }
                    })
                    .border_b(px(1.0))
                    .border_color(cx.theme().colors().status_bar_background),
            })
            .child(self.render_left_tools(&sidebar, cx))
            .child(self.render_right_tools(&sidebar, cx))
    }
}

impl StatusBar {
    fn render_left_tools(
        &self,
        sidebar: &SidebarStatus,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tools = h_flex().gap_1().min_w_0().overflow_x_hidden().when(
            sidebar.show_toggle && !sidebar.open && sidebar.side == SidebarSide::Left,
            |this| this.child(self.render_sidebar_toggle(sidebar, cx)),
        );
        #[cfg(feature = "multiplayer-tools")]
        let tools = tools.when(self.collaborative_participants.is_some(), |this| {
            let task = self.collaborative_participants.as_ref().and_then(|state| {
                let CollaborativeParticipantProviderState::Ready(view_data) = state else {
                    return None;
                };
                view_data.execution.as_ref().and_then(|execution| {
                    CollaborativeTaskPhase::from_execution_phase(execution.phase)
                })
            });
            this.child(
                div()
                    .id("collaborative-status-landmark")
                    .track_focus(&self.collaborative_status_focus_handle)
                    .tab_index(0)
                    .role(Role::Status)
                    .aria_label(crate::collaborative_accessibility::STATUS_LABEL)
                    .child(CollaborativeStatus::new(
                        CollaborativeStatusProjection::from_project(&self.project, None, task, cx),
                    )),
            )
        });
        tools.children(
            self.left_items.iter().enumerate().map(|(index, item)| {
                render_hideable_item("status-bar-left", index, item.as_ref(), cx)
            }),
        )
    }

    fn render_right_tools(
        &self,
        sidebar: &SidebarStatus,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tools = h_flex()
            .flex_shrink_0()
            .gap_1()
            .overflow_x_hidden()
            .children(
                self.right_items
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(index, item)| {
                        render_hideable_item("status-bar-right", index, item.as_ref(), cx)
                    }),
            );
        #[cfg(feature = "multiplayer-tools")]
        let tools = tools.when_some(
            self.collaborative_participants.clone(),
            |this, participant_state| {
                this.child(CollaborativeParticipantStatus::new(participant_state))
            },
        );
        tools.when(
            sidebar.show_toggle && !sidebar.open && sidebar.side == SidebarSide::Right,
            |this| this.child(self.render_sidebar_toggle(sidebar, cx)),
        )
    }

    fn render_sidebar_toggle(
        &self,
        sidebar: &SidebarStatus,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let on_right = sidebar.side == SidebarSide::Right;
        let has_notifications = sidebar.has_notifications;
        let indicator_border = cx.theme().colors().status_bar_background;

        let toggle = sidebar_side_context_menu("sidebar-status-toggle-menu", cx)
            .anchor(if on_right {
                Anchor::BottomRight
            } else {
                Anchor::BottomLeft
            })
            .attach(if on_right {
                Anchor::TopRight
            } else {
                Anchor::TopLeft
            })
            .trigger(move |_is_active, _window, _cx| {
                IconButton::new(
                    "toggle-workspace-sidebar",
                    if on_right {
                        IconName::ThreadsSidebarRightClosed
                    } else {
                        IconName::ThreadsSidebarLeftClosed
                    },
                )
                .icon_size(IconSize::Small)
                .when(has_notifications, |this| {
                    this.indicator(Indicator::dot().color(Color::Accent))
                        .indicator_border_color(Some(indicator_border))
                })
                .tooltip(move |_, cx| {
                    Tooltip::for_action("Open Threads Sidebar", &ToggleWorkspaceSidebar, cx)
                })
                .on_click(move |_, window, cx| {
                    if let Some(multi_workspace) = window.root::<MultiWorkspace>().flatten() {
                        multi_workspace.update(cx, |multi_workspace, cx| {
                            multi_workspace.toggle_sidebar(window, cx);
                        });
                    }
                })
            });

        h_flex()
            .gap_0p5()
            .when(on_right, |this| {
                this.child(Divider::vertical().color(ui::DividerColor::Border))
            })
            .child(toggle)
            .when(!on_right, |this| {
                this.child(Divider::vertical().color(ui::DividerColor::Border))
            })
    }
}

fn render_hideable_item(
    side: &'static str,
    index: usize,
    item: &dyn StatusItemViewHandle,
    cx: &App,
) -> impl IntoElement {
    let view = item.to_any();
    let Some(hide) = item.hide_setting(cx) else {
        return view.into_any_element();
    };

    let menu_id: SharedString = format!("{side}-item-menu-{index}").into();
    right_click_menu(menu_id)
        .trigger(move |_is_active, _window, _cx| view)
        .menu(move |window, cx| {
            let hide = hide.clone();
            ContextMenu::build(window, cx, move |menu, _window, _cx| {
                add_hide_button_entry(menu, hide)
            })
        })
        .into_any_element()
}

/// Appends a "Hide Button" entry aligned with surrounding toggleable entries.
pub fn add_hide_button_entry(menu: ContextMenu, hide: HideStatusItem) -> ContextMenu {
    menu.toggleable_entry(
        "Hide Button",
        false,
        IconPosition::Start,
        None,
        move |_window, cx| hide.apply(cx),
    )
}

impl StatusBar {
    #[cfg(feature = "multiplayer-tools")]
    pub(crate) fn collaborative_focus_handle(&self) -> FocusHandle {
        self.collaborative_status_focus_handle.clone()
    }

    pub fn new(
        active_pane: &Entity<Pane>,
        project: Entity<Project>,
        multi_workspace: Option<WeakEntity<MultiWorkspace>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let git_store = project.read(cx).git_store().clone();
        let mut this = Self {
            left_items: Default::default(),
            right_items: Default::default(),
            active_pane: active_pane.clone(),
            #[cfg(feature = "multiplayer-tools")]
            project: project.clone(),
            multi_workspace,
            #[cfg(feature = "multiplayer-tools")]
            collaborative_participants: None,
            #[cfg(feature = "multiplayer-tools")]
            collaborative_status_focus_handle: cx.focus_handle(),
            _observe_active_pane: cx.observe_in(active_pane, window, |this, _, window, cx| {
                this.update_active_pane_item(window, cx)
            }),
            #[cfg(feature = "multiplayer-tools")]
            _observe_project: cx.observe(&project, |_, _, cx| cx.notify()),
            _observe_git_store: cx.observe(&git_store, |_, _, cx| cx.notify()),
        };
        this.update_active_pane_item(window, cx);
        this
    }

    pub fn set_multi_workspace(
        &mut self,
        multi_workspace: WeakEntity<MultiWorkspace>,
        cx: &mut Context<Self>,
    ) {
        self.multi_workspace = Some(multi_workspace);
        cx.notify();
    }

    #[cfg(feature = "multiplayer-tools")]
    pub(crate) fn set_collaborative_participants(
        &mut self,
        state: Option<CollaborativeParticipantProviderState>,
        cx: &mut Context<Self>,
    ) {
        if self.collaborative_participants != state {
            self.collaborative_participants = state;
            cx.notify();
        }
    }

    pub fn add_left_item<T>(&mut self, item: Entity<T>, window: &mut Window, cx: &mut Context<Self>)
    where
        T: 'static + StatusItemView,
    {
        let active_pane_item = self.active_pane.read(cx).active_item();
        item.set_active_pane_item(active_pane_item.as_deref(), window, cx);

        self.left_items.push(Box::new(item));
        cx.notify();
    }

    pub fn item_of_type<T: StatusItemView>(&self) -> Option<Entity<T>> {
        self.left_items
            .iter()
            .chain(self.right_items.iter())
            .find_map(|item| item.to_any().downcast().ok())
    }

    pub fn position_of_item<T>(&self) -> Option<usize>
    where
        T: StatusItemView,
    {
        for (index, item) in self.left_items.iter().enumerate() {
            if item.item_type() == TypeId::of::<T>() {
                return Some(index);
            }
        }
        for (index, item) in self.right_items.iter().enumerate() {
            if item.item_type() == TypeId::of::<T>() {
                return Some(index + self.left_items.len());
            }
        }
        None
    }

    pub fn insert_item_after<T>(
        &mut self,
        position: usize,
        item: Entity<T>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        T: 'static + StatusItemView,
    {
        let active_pane_item = self.active_pane.read(cx).active_item();
        item.set_active_pane_item(active_pane_item.as_deref(), window, cx);

        if position < self.left_items.len() {
            self.left_items.insert(position + 1, Box::new(item))
        } else {
            self.right_items
                .insert(position + 1 - self.left_items.len(), Box::new(item))
        }
        cx.notify()
    }

    pub fn remove_item_at(&mut self, position: usize, cx: &mut Context<Self>) {
        if position < self.left_items.len() {
            self.left_items.remove(position);
        } else {
            self.right_items.remove(position - self.left_items.len());
        }
        cx.notify();
    }

    pub fn add_right_item<T>(
        &mut self,
        item: Entity<T>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        T: 'static + StatusItemView,
    {
        let active_pane_item = self.active_pane.read(cx).active_item();
        item.set_active_pane_item(active_pane_item.as_deref(), window, cx);

        self.right_items.push(Box::new(item));
        cx.notify();
    }

    pub fn set_active_pane(
        &mut self,
        active_pane: &Entity<Pane>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_pane = active_pane.clone();
        self._observe_active_pane = cx.observe_in(active_pane, window, |this, _, window, cx| {
            this.update_active_pane_item(window, cx)
        });
        self.update_active_pane_item(window, cx);
    }

    fn update_active_pane_item(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active_pane_item = self.active_pane.read(cx).active_item();
        for item in self.left_items.iter().chain(&self.right_items) {
            item.set_active_pane_item(active_pane_item.as_deref(), window, cx);
        }
    }
}

#[cfg(feature = "multiplayer-tools")]
#[derive(IntoElement)]
struct CollaborativeParticipantStatus {
    state: CollaborativeParticipantProviderState,
}

#[cfg(feature = "multiplayer-tools")]
impl CollaborativeParticipantStatus {
    fn new(state: CollaborativeParticipantProviderState) -> Self {
        Self { state }
    }
}

#[cfg(feature = "multiplayer-tools")]
impl RenderOnce for CollaborativeParticipantStatus {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        h_flex()
            .id("collaborative-participant-status")
            .debug_selector(|| "COLLABORATIVE-PARTICIPANT-STATUS".to_owned())
            .min_w_0()
            .gap_1()
            .px_1()
            .child(Icon::new(IconName::SimAgent).size(IconSize::XSmall))
            .child(h_flex().gap_1().map(|this| {
                match self.state {
                    CollaborativeParticipantProviderState::Ready(view_data) => {
                        let execution_available = view_data.execution.is_some();
                        let execution = view_data.execution;
                        this.debug_selector(|| "COLLABORATIVE-PARTICIPANT-STATUS-READY".to_owned())
                            .when_some(execution, |this, execution| {
                                let phase = match execution.phase {
                                    CollaborativeExecutionPhase::Idle => "Idle",
                                    CollaborativeExecutionPhase::Running => "Running",
                                    CollaborativeExecutionPhase::WaitingForUser => "Waiting",
                                    CollaborativeExecutionPhase::Failed => "Failed",
                                    CollaborativeExecutionPhase::Completed => "Completed",
                                    CollaborativeExecutionPhase::Unknown => "Unknown",
                                };
                                this.child(
                                    Label::new(phase)
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted),
                                )
                                .child(Divider::vertical())
                                .child(
                                    Label::new(execution.model_label())
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted),
                                )
                                .child(
                                    Label::new(execution.runtime_label())
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted),
                                )
                                .child(
                                    Label::new(execution.location_label())
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted),
                                )
                            })
                            .when(!execution_available, |this| {
                                this.child(
                                    Label::new("No active execution")
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted),
                                )
                            })
                    }
                    CollaborativeParticipantProviderState::Failed(message) => this
                        .debug_selector(|| "COLLABORATIVE-PARTICIPANT-STATUS-FAILED".to_owned())
                        .child(
                            Label::new(message)
                                .size(LabelSize::XSmall)
                                .color(Color::Error),
                        ),
                    CollaborativeParticipantProviderState::Unavailable => this
                        .debug_selector(|| {
                            "COLLABORATIVE-PARTICIPANT-STATUS-UNAVAILABLE".to_owned()
                        })
                        .child(
                            Label::new("Agent unavailable")
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        ),
                }
            }))
    }
}

impl<T: StatusItemView> StatusItemViewHandle for Entity<T> {
    fn to_any(&self) -> AnyView {
        self.clone().into()
    }

    fn set_active_pane_item(
        &self,
        active_pane_item: Option<&dyn ItemHandle>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.update(cx, |this, cx| {
            this.set_active_pane_item(active_pane_item, window, cx)
        });
    }

    fn item_type(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn hide_setting(&self, cx: &App) -> Option<HideStatusItem> {
        self.read(cx).hide_setting(cx)
    }
}

impl From<&dyn StatusItemViewHandle> for AnyView {
    fn from(val: &dyn StatusItemViewHandle) -> Self {
        val.to_any()
    }
}
