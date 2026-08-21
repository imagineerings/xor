use gpui::{Context, Entity, Render, Role, ScrollHandle, WeakEntity};
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
            pinned: cx.new(|cx| CollaborativePinned::new(multi_workspace.clone(), cx)),
            projects: cx.new(|cx| CollaborativeProjects::new(multi_workspace.clone(), cx)),
            tasks: cx.new(|cx| CollaborativeTasks::new(multi_workspace, cx)),
            pinned_scroll_handle: ScrollHandle::new(),
            projects_scroll_handle: ScrollHandle::new(),
            tasks_scroll_handle: ScrollHandle::new(),
        }
    }

    fn render_section(
        label: &'static str,
        selector: &'static str,
        scroll_handle: &ScrollHandle,
        contents: impl IntoElement,
    ) -> impl IntoElement {
        v_flex()
            .debug_selector(move || selector.to_owned())
            .flex_1()
            .min_h_0()
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
                    .flex_1()
                    .min_h_0()
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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
                self.pinned.clone(),
            ))
            .child(Divider::horizontal())
            .child(Self::render_section(
                SECTION_LABELS[1],
                "COLLABORATIVE-RAIL-PROJECTS",
                &self.projects_scroll_handle,
                self.projects.clone(),
            ))
            .child(Divider::horizontal())
            .child(Self::render_section(
                SECTION_LABELS[2],
                "COLLABORATIVE-RAIL-TASKS",
                &self.tasks_scroll_handle,
                self.tasks.clone(),
            ))
    }
}
