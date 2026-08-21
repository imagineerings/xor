use std::collections::HashSet;

use gpui::{AnyElement, Context, ListAlignment, ListState, Render, Role, Window, list, px};
use ui::{Button, ButtonStyle, LabelSize, prelude::*};
use workspace::collaborative_accessibility::TIMELINE_LABEL;

use crate::{
    activity_projection::{
        ActivityDetailHandle, ActivityItem, ActivityItemId, ActivityLifecycle,
        ActivitySemanticClass,
    },
    activity_reducer::{ActivityReducer, ActivityReduction, ActivityReductionError},
};

pub struct CollaborativeTimeline {
    reducer: ActivityReducer,
    list_state: ListState,
    expanded_details: HashSet<ActivityItemId>,
}

impl CollaborativeTimeline {
    pub fn new() -> Self {
        let list_state = ListState::new(0, ListAlignment::Top, px(1024.0));
        list_state.set_follow_mode(gpui::FollowMode::Tail);
        Self {
            reducer: ActivityReducer::new(),
            list_state,
            expanded_details: HashSet::new(),
        }
    }

    pub fn items(&self) -> &[ActivityItem] {
        self.reducer.items()
    }

    pub fn list_state(&self) -> &ListState {
        &self.list_state
    }

    pub fn update_item(
        &mut self,
        item: ActivityItem,
        cx: &mut Context<Self>,
    ) -> Result<ActivityReduction, ActivityReductionError> {
        let reduction = self.apply_item(item)?;
        cx.notify();
        Ok(reduction)
    }

    pub fn is_detail_expanded(&self, id: &ActivityItemId) -> bool {
        self.expanded_details.contains(id)
    }

    fn apply_item(
        &mut self,
        item: ActivityItem,
    ) -> Result<ActivityReduction, ActivityReductionError> {
        let reduction = self.reducer.reduce(item)?;
        match reduction {
            ActivityReduction::Inserted { index } => self.list_state.splice(index..index, 1),
            ActivityReduction::Updated { index } => {
                self.list_state.remeasure_items(index..index + 1)
            }
            ActivityReduction::Duplicate { .. } | ActivityReduction::IgnoredStale { .. } => {}
        }
        Ok(reduction)
    }

    fn toggle_details(&mut self, id: ActivityItemId, cx: &mut Context<Self>) {
        let Some(index) = self.set_detail_expanded(&id, !self.is_detail_expanded(&id)) else {
            return;
        };
        self.list_state.remeasure_items(index..index + 1);
        cx.notify();
    }

    fn set_detail_expanded(&mut self, id: &ActivityItemId, expanded: bool) -> Option<usize> {
        let index = self.items().iter().position(|item| &item.id == id)?;
        if expanded {
            self.expanded_details.insert(id.clone());
        } else {
            self.expanded_details.remove(id);
        }
        Some(index)
    }

    fn render_item(
        &self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(item) = self.items().get(index) else {
            return div().into_any_element();
        };
        let item_id = item.id.clone();
        let is_expanded = self.is_detail_expanded(&item_id);
        let has_details = item.details.is_some()
            || matches!(
                item.class,
                ActivitySemanticClass::Generic | ActivitySemanticClass::Raw
            );
        let summary = semantic_summary(item);
        let lifecycle = lifecycle_label(item.lifecycle);
        let outcome = item.outcome.summary.clone();
        let accessibility_label = activity_accessibility_label(item);
        let detail = is_expanded.then(|| detail_summary(item));

        v_flex()
            .id(("collaborative-activity-row", index))
            .role(Role::ListItem)
            .aria_label(accessibility_label)
            .w_full()
            .px_4()
            .py_2()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().colors().border_variant)
            .child(
                h_flex()
                    .w_full()
                    .items_start()
                    .justify_between()
                    .gap_3()
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap_0p5()
                            .child(div().text_ui(cx).child(summary))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().colors().text_muted)
                                    .child(format!("{lifecycle} · {}", class_label(item.class))),
                            ),
                    )
                    .when(has_details, |this| {
                        this.child(
                            Button::new(
                                ("collaborative-activity-details", index),
                                if is_expanded {
                                    "Hide details"
                                } else {
                                    "Show details"
                                },
                            )
                            .style(ButtonStyle::Subtle)
                            .label_size(LabelSize::Small)
                            .aria_expanded(is_expanded)
                            .on_click(cx.listener(
                                move |this, _, _window, cx| {
                                    this.toggle_details(item_id.clone(), cx);
                                },
                            )),
                        )
                    }),
            )
            .when_some(outcome, |this, outcome| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().colors().text_muted)
                        .child(outcome),
                )
            })
            .when_some(detail, |this, detail| {
                this.child(
                    div()
                        .mt_1()
                        .p_2()
                        .rounded_sm()
                        .bg(cx.theme().colors().editor_background)
                        .border_1()
                        .border_color(cx.theme().colors().border)
                        .text_xs()
                        .text_color(cx.theme().colors().text_muted)
                        .child(detail),
                )
            })
            .into_any_element()
    }
}

impl Default for CollaborativeTimeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for CollaborativeTimeline {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.items().is_empty() {
            return div()
                .id("collaborative-activity-empty")
                .role(Role::Status)
                .aria_label("Collaborative activity timeline is empty")
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(cx.theme().colors().text_muted)
                .child("No activity yet")
                .into_any_element();
        }

        let view = cx.entity();
        div()
            .id("collaborative-activity-timeline")
            .role(Role::Document)
            .aria_label(TIMELINE_LABEL)
            .size_full()
            .child(
                list(self.list_state.clone(), move |index, window, cx| {
                    view.update(cx, |this, cx| this.render_item(index, window, cx))
                })
                .size_full(),
            )
            .into_any_element()
    }
}

fn semantic_summary(item: &ActivityItem) -> String {
    match item.class {
        ActivitySemanticClass::Generic | ActivitySemanticClass::Raw => format!(
            "{} reported an unsupported activity event",
            item.actor.label
        ),
        ActivitySemanticClass::Suppressed => "Activity hidden by policy".into(),
        _ => format!("{} {} {}", item.actor.label, item.verb, item.object.label),
    }
}

fn class_label(class: ActivitySemanticClass) -> &'static str {
    match class {
        ActivitySemanticClass::Message => "Message",
        ActivitySemanticClass::PlatformOperation => "Operation",
        ActivitySemanticClass::FileEdit => "File edit",
        ActivitySemanticClass::ShellCommand => "Command",
        ActivitySemanticClass::Lifecycle => "Session",
        ActivitySemanticClass::Thought => "Thought summary",
        ActivitySemanticClass::Plan => "Plan",
        ActivitySemanticClass::Permission => "Permission",
        ActivitySemanticClass::Error => "Error",
        ActivitySemanticClass::Generic => "Unsupported event",
        ActivitySemanticClass::Raw => "Raw event",
        ActivitySemanticClass::Suppressed => "Suppressed",
    }
}

fn lifecycle_label(lifecycle: ActivityLifecycle) -> &'static str {
    match lifecycle {
        ActivityLifecycle::Pending => "Pending",
        ActivityLifecycle::Running => "Running",
        ActivityLifecycle::WaitingForUser => "Waiting for you",
        ActivityLifecycle::Idle => "Idle",
        ActivityLifecycle::Succeeded => "Completed",
        ActivityLifecycle::Failed => "Failed",
        ActivityLifecycle::Cancelled => "Cancelled",
        ActivityLifecycle::TimedOut => "Timed out",
        ActivityLifecycle::Disconnected => "Disconnected",
        ActivityLifecycle::Suppressed => "Suppressed",
    }
}

fn activity_accessibility_label(item: &ActivityItem) -> String {
    let mut label = format!(
        "{}. {}. {}",
        semantic_summary(item),
        class_label(item.class),
        lifecycle_label(item.lifecycle)
    );
    if let Some(outcome) = &item.outcome.summary {
        label.push_str(". ");
        label.push_str(outcome);
    }
    label
}

fn detail_summary(item: &ActivityItem) -> String {
    match &item.details {
        Some(ActivityDetailHandle::AcpEntry {
            session_id,
            entry_id,
        }) => format!("ACP session {session_id}, entry {entry_id}"),
        Some(ActivityDetailHandle::NativeAction { action_id }) => {
            format!("Native action {action_id}")
        }
        Some(ActivityDetailHandle::ProtocolEvent { event_id }) => {
            format!("Protocol event {event_id}")
        }
        Some(ActivityDetailHandle::GitChange {
            repository_id,
            change_id,
        }) => format!("Repository {repository_id}, change {change_id}"),
        Some(ActivityDetailHandle::WorkflowRun { run_id, step_id }) => {
            step_id.as_ref().map_or_else(
                || format!("Workflow run {run_id}"),
                |step_id| format!("Workflow run {run_id}, step {step_id}"),
            )
        }
        Some(ActivityDetailHandle::RawSource { item_id }) => format!(
            "Raw {:?} event {}",
            item_id.source_kind(),
            item_id.source_id()
        ),
        None => format!(
            "Unsupported {:?} event {}",
            item.id.source_kind(),
            item.id.source_id()
        ),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone as _, Utc};

    use super::*;
    use crate::activity_projection::{
        ActivityActor, ActivityActorKind, ActivityContext, ActivityObject, ActivityObjectKind,
        ActivityOutcome, ActivityOutcomeStatus, ActivitySourceKind, ActivityVisibility,
    };

    fn item(source_id: &str, class: ActivitySemanticClass) -> ActivityItem {
        let timestamp = Utc
            .with_ymd_and_hms(2026, 8, 14, 12, 0, 0)
            .single()
            .expect("test timestamp should be valid");
        ActivityItem {
            id: ActivityItemId::new(ActivitySourceKind::Acp, source_id)
                .expect("test source id should be valid"),
            source_version: 1,
            class,
            actor: ActivityActor {
                kind: ActivityActorKind::Agent,
                id: "agent-1".into(),
                label: "Builder".into(),
            },
            verb: "edited".into(),
            object: ActivityObject {
                kind: ActivityObjectKind::File,
                id: Some("file-1".into()),
                label: "src/main.rs".into(),
            },
            outcome: ActivityOutcome {
                status: ActivityOutcomeStatus::Pending,
                summary: None,
            },
            lifecycle: ActivityLifecycle::Running,
            occurred_at: timestamp,
            projected_at: timestamp,
            context: ActivityContext::default(),
            visibility: ActivityVisibility::Private,
            details: Some(ActivityDetailHandle::AcpEntry {
                session_id: "session-1".into(),
                entry_id: source_id.into(),
            }),
            links: Vec::new(),
        }
    }

    #[test]
    fn collaborative_timeline_render_preserves_reducer_order() {
        let mut timeline = CollaborativeTimeline::new();
        timeline
            .apply_item(item("message-1", ActivitySemanticClass::Message))
            .expect("first item should insert");
        timeline
            .apply_item(item("edit-1", ActivitySemanticClass::FileEdit))
            .expect("second item should insert");

        assert_eq!(timeline.items()[0].id.source_id(), "message-1");
        assert_eq!(timeline.items()[1].id.source_id(), "edit-1");
    }

    #[test]
    fn collaborative_timeline_render_uses_virtualized_list_count() {
        let mut timeline = CollaborativeTimeline::new();
        for index in 0..1_000 {
            timeline
                .apply_item(item(
                    &format!("activity-{index}"),
                    ActivitySemanticClass::Message,
                ))
                .expect("unique activity should insert");
        }

        assert_eq!(timeline.items().len(), 1_000);
        assert_eq!(timeline.list_state().item_count(), 1_000);
    }

    #[test]
    fn collaborative_timeline_render_discloses_typed_details() {
        let item = item("edit-1", ActivitySemanticClass::FileEdit);
        let item_id = item.id.clone();
        let mut timeline = CollaborativeTimeline::new();
        timeline
            .apply_item(item.clone())
            .expect("detail-bearing item should insert");

        assert!(!timeline.is_detail_expanded(&item_id));
        assert_eq!(timeline.set_detail_expanded(&item_id, true), Some(0));
        assert!(timeline.is_detail_expanded(&item_id));
        assert_eq!(detail_summary(&item), "ACP session session-1, entry edit-1");
        assert_eq!(timeline.set_detail_expanded(&item_id, false), Some(0));
        assert!(!timeline.is_detail_expanded(&item_id));
    }

    #[test]
    fn collaborative_timeline_accessibility_labels_running_and_failed_activity() {
        let running = item("running-1", ActivitySemanticClass::ShellCommand);
        assert_eq!(
            activity_accessibility_label(&running),
            "Builder edited src/main.rs. Command. Running"
        );

        let mut failed = item("failed-1", ActivitySemanticClass::Error);
        failed.lifecycle = ActivityLifecycle::Failed;
        failed.outcome.summary = Some("Command exited with status 1".into());
        assert_eq!(
            activity_accessibility_label(&failed),
            "Builder edited src/main.rs. Error. Failed. Command exited with status 1"
        );
    }

    #[test]
    fn collaborative_timeline_render_labels_unknown_events_truthfully() {
        let mut unknown = item("future-1", ActivitySemanticClass::Generic);
        unknown.details = None;

        assert_eq!(
            semantic_summary(&unknown),
            "Builder reported an unsupported activity event"
        );
        assert_eq!(detail_summary(&unknown), "Unsupported Acp event future-1");
    }
}
