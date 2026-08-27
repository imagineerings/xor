use gpui::{
    Action, Context, Entity, InteractiveElement, Render, Role, ScrollHandle, SharedString,
    WeakEntity,
};
use ui::{Divider, Window, prelude::*};
use workspace::MultiWorkspace;

use crate::collaborative_pinned::CollaborativePinned;
use crate::collaborative_projects::CollaborativeProjects;
use crate::collaborative_tasks::CollaborativeTasks;

const SECTION_LABELS: [&str; 3] = ["Pinned", "Communities and Projects", "Tasks and Threads"];
#[cfg(test)]
const EMPTY_LABELS: [&str; 3] = [
    "No pinned or recent work",
    "No communities or projects",
    "No tasks or threads",
];

pub(crate) struct CollaborativeRail {
    multi_workspace: WeakEntity<MultiWorkspace>,
    pinned: Entity<CollaborativePinned>,
    projects: Entity<CollaborativeProjects>,
    tasks: Entity<CollaborativeTasks>,
    pinned_scroll_handle: ScrollHandle,
    projects_scroll_handle: ScrollHandle,
    tasks_scroll_handle: ScrollHandle,
}

impl CollaborativeRail {
    pub(crate) fn new(multi_workspace: WeakEntity<MultiWorkspace>, cx: &mut Context<Self>) -> Self {
        Self {
            multi_workspace: multi_workspace.clone(),
            pinned: cx.new(|cx| CollaborativePinned::new(multi_workspace.clone(), cx)),
            projects: cx.new(|cx| CollaborativeProjects::new(multi_workspace.clone(), cx)),
            tasks: cx.new(|cx| CollaborativeTasks::new(multi_workspace, cx)),
            pinned_scroll_handle: ScrollHandle::new(),
            projects_scroll_handle: ScrollHandle::new(),
            tasks_scroll_handle: ScrollHandle::new(),
        }
    }

    fn render_account_surface(&self, cx: &gpui::App) -> impl IntoElement {
        let user = self.multi_workspace.upgrade().and_then(|multi_workspace| {
            let workspace = multi_workspace.read(cx).workspace().clone();
            workspace
                .read(cx)
                .app_state()
                .user_store
                .read(cx)
                .current_user()
        });
        let account_label = user
            .as_ref()
            .and_then(|user| user.name.as_deref())
            .filter(|name| !name.trim().is_empty())
            .map(SharedString::from)
            .or_else(|| user.as_ref().map(|user| user.username.clone()))
            .unwrap_or_else(|| "Sign in".into());
        let product_label = release_channel::ReleaseChannel::try_global(cx)
            .map(|channel| channel.display_name())
            .unwrap_or("Zed");

        h_flex()
            .id("collaborative-account-surface")
            .debug_selector(|| "COLLABORATIVE-ACCOUNT-SURFACE".to_owned())
            .h_10()
            .flex_none()
            .px_2()
            .gap_2()
            .border_t_1()
            .border_color(cx.theme().colors().border)
            .tab_index(0)
            .role(Role::Button)
            .aria_label(format!("Account: {account_label}, {product_label}"))
            .child(
                h_flex()
                    .size_6()
                    .rounded_full()
                    .justify_center()
                    .bg(cx.theme().colors().element_background)
                    .child(
                        Icon::new(IconName::Person)
                            .size(IconSize::Small)
                            .color(Color::Muted),
                    ),
            )
            .child(
                v_flex()
                    .min_w_0()
                    .flex_1()
                    .child(Label::new(account_label).size(LabelSize::Small).truncate())
                    .child(
                        Label::new(product_label)
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    ),
            )
            .child(
                Icon::new(IconName::ChevronDown)
                    .size(IconSize::XSmall)
                    .color(Color::Muted),
            )
            .on_click(|_, window, cx| {
                window.dispatch_action(zed_actions::OpenAccountSettings.boxed_clone(), cx);
            })
    }

    fn render_section(
        label: &'static str,
        selector: &'static str,
        scroll_handle: &ScrollHandle,
        fills_remaining_space: bool,
        contents: impl IntoElement,
    ) -> impl IntoElement {
        v_flex()
            .debug_selector(move || selector.to_owned())
            .min_h_0()
            .when(fills_remaining_space, |this| this.flex_1())
            .when(!fills_remaining_space, |this| {
                this.flex_none().max_h(rems(18.))
            })
            .child(
                h_flex().h_7().flex_none().px_2().child(
                    Label::new(label.to_ascii_uppercase())
                        .size(LabelSize::XSmall)
                        .color(Color::Muted),
                ),
            )
            .child(
                v_flex()
                    .id(selector)
                    .min_h_0()
                    .when(fills_remaining_space, |this| this.flex_1())
                    .when(!fills_remaining_space, |this| this.max_h(rems(16.)))
                    .overflow_y_scroll()
                    .track_scroll(scroll_handle)
                    .px_2()
                    .pb_2()
                    .child(contents),
            )
    }

    #[cfg(test)]
    pub(crate) fn test_scroll_handles(&self) -> [ScrollHandle; 3] {
        [
            self.pinned_scroll_handle.clone(),
            self.projects_scroll_handle.clone(),
            self.tasks_scroll_handle.clone(),
        ]
    }

    #[cfg(test)]
    pub(crate) fn test_labels(&self) -> ([&'static str; 3], [&'static str; 3]) {
        (SECTION_LABELS, EMPTY_LABELS)
    }
}

impl Render for CollaborativeRail {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("collaborative-rail")
            .debug_selector(|| "COLLABORATIVE-RAIL".to_owned())
            .size_full()
            .role(Role::Navigation)
            .aria_label(workspace::collaborative_accessibility::NAVIGATION_LABEL)
            .child(Self::render_section(
                SECTION_LABELS[0],
                "COLLABORATIVE-RAIL-PINNED",
                &self.pinned_scroll_handle,
                false,
                self.pinned.clone(),
            ))
            .child(Divider::horizontal())
            .child(Self::render_section(
                SECTION_LABELS[1],
                "COLLABORATIVE-RAIL-PROJECTS",
                &self.projects_scroll_handle,
                false,
                self.projects.clone(),
            ))
            .child(Divider::horizontal())
            .child(Self::render_section(
                SECTION_LABELS[2],
                "COLLABORATIVE-RAIL-TASKS",
                &self.tasks_scroll_handle,
                true,
                self.tasks.clone(),
            ))
            .child(self.render_account_surface(cx))
    }
}
