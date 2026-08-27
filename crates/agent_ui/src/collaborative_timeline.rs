use std::{collections::HashSet, error::Error, fmt, sync::Arc};

use gpui::{
    AnyElement, App, Context, Entity, EntityId, EventEmitter, ListAlignment, ListState, Render,
    Role, Subscription, Window, list, px,
};
use ui::prelude::*;
use workspace::{
    Workspace,
    collaborative_accessibility::TIMELINE_LABEL,
    collaborative_timeline::{
        CollaborativeTimelineProvider, CollaborativeTimelineRegistration,
        CollaborativeTimelineRegistrationError,
    },
};

use crate::{
    AgentPanel,
    activity_projection::{ActivityItem, ActivityItemId},
    activity_reducer::{ActivityReducer, ActivityReduction, ActivityReductionError},
    collaborative_activity_cards::{ActivityCardIntervention, CollaborativeActivityCard},
    conversation_view::ThreadView,
};

#[derive(Clone, Debug)]
pub enum CollaborativeTimelineEvent {
    InterventionRequested(ActivityCardIntervention),
}

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
        let item = item.clone();
        let item_id = item.id.clone();
        let is_expanded = self.is_detail_expanded(&item_id);
        let timeline = cx.entity();
        let toggle_timeline = timeline.clone();
        CollaborativeActivityCard::new(index, item, is_expanded)
            .on_toggle(Arc::new(move |_window, cx| {
                toggle_timeline.update(cx, |this, cx| {
                    this.toggle_details(item_id.clone(), cx);
                });
            }))
            .on_intervention(Arc::new(move |intervention, _window, cx| {
                timeline.update(cx, |_this, cx| {
                    cx.emit(CollaborativeTimelineEvent::InterventionRequested(
                        intervention,
                    ));
                });
            }))
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

impl EventEmitter<CollaborativeTimelineEvent> for CollaborativeTimeline {}

pub struct CollaborativeTimelineAdapter {
    thread_view_id: EntityId,
    provider: CollaborativeTimelineProvider,
}

impl CollaborativeTimelineAdapter {
    pub fn from_agent_panel(
        agent_panel: &Entity<AgentPanel>,
        workspace: &Entity<Workspace>,
        cx: &mut App,
    ) -> Result<Self, CollaborativeTimelineAdapterError> {
        let thread_view = agent_panel
            .read(cx)
            .active_thread_view(cx)
            .ok_or(CollaborativeTimelineAdapterError::ThreadUnavailable)?;
        let project = thread_view.read(cx).project.upgrade();
        let project = project.ok_or(CollaborativeTimelineAdapterError::ProjectUnavailable)?;
        if project.entity_id() != workspace.read(cx).project().entity_id() {
            return Err(CollaborativeTimelineAdapterError::ProjectMismatch);
        }
        let timeline = cx.new(|cx| CollaborativeAcpTimeline::new(thread_view.clone(), cx));
        Ok(Self {
            thread_view_id: thread_view.entity_id(),
            provider: CollaborativeTimelineProvider::new(project, timeline.into()),
        })
    }

    pub fn thread_view_id(&self) -> EntityId {
        self.thread_view_id
    }

    pub fn register_in_workspace(
        self,
        workspace: &mut Workspace,
        cx: &mut gpui::Context<Workspace>,
    ) -> Result<CollaborativeTimelineRegistration, CollaborativeTimelineAdapterError> {
        workspace
            .register_collaborative_timeline_provider(self.provider, cx)
            .map_err(CollaborativeTimelineAdapterError::Registration)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaborativeTimelineAdapterError {
    ThreadUnavailable,
    ProjectUnavailable,
    ProjectMismatch,
    Registration(CollaborativeTimelineRegistrationError),
}

impl fmt::Display for CollaborativeTimelineAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ThreadUnavailable => formatter.write_str("active agent thread is unavailable"),
            Self::ProjectUnavailable => formatter.write_str("active agent project is unavailable"),
            Self::ProjectMismatch => {
                formatter.write_str("active agent thread belongs to a different project")
            }
            Self::Registration(error) => write!(formatter, "timeline registration failed: {error}"),
        }
    }
}

impl Error for CollaborativeTimelineAdapterError {}

struct CollaborativeAcpTimeline {
    thread_view: Entity<ThreadView>,
    _thread_view_observation: Subscription,
}

impl CollaborativeAcpTimeline {
    fn new(thread_view: Entity<ThreadView>, cx: &mut Context<Self>) -> Self {
        let _thread_view_observation = cx.observe(&thread_view, |_, _, cx| cx.notify());
        Self {
            thread_view,
            _thread_view_observation,
        }
    }
}

impl Render for CollaborativeAcpTimeline {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let thread_view = self.thread_view.clone();
        div()
            .id("collaborative-native-acp-timeline")
            .debug_selector(|| "COLLABORATIVE-NATIVE-ACP-TIMELINE".to_owned())
            .size_full()
            .child(thread_view.update(cx, |thread_view, cx| {
                thread_view.render_collaborative_entries(window, cx)
            }))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone as _, Utc};

    use super::*;
    use crate::{
        activity_projection::{
            ActivityActor, ActivityActorKind, ActivityContext, ActivityDetailHandle,
            ActivityLifecycle, ActivityObject, ActivityObjectKind, ActivityOutcome,
            ActivityOutcomeStatus, ActivitySemanticClass, ActivitySourceKind, ActivityVisibility,
        },
        collaborative_activity_cards::{ActivityCardPresentation, ActivityCardSource},
    };

    fn presentation(item: &ActivityItem) -> ActivityCardPresentation {
        ActivityCardPresentation::new(
            item,
            &[ActivityCardSource {
                id: item.id.clone(),
                provenance: None,
            }],
        )
    }

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
        assert_eq!(
            presentation(&item).detail.as_deref(),
            Some("ACP session session-1, entry edit-1")
        );
        assert_eq!(timeline.set_detail_expanded(&item_id, false), Some(0));
        assert!(!timeline.is_detail_expanded(&item_id));
    }

    #[test]
    fn collaborative_timeline_accessibility_labels_running_and_failed_activity() {
        let running = item("running-1", ActivitySemanticClass::ShellCommand);
        assert_eq!(
            presentation(&running).accessibility_label(),
            "Builder edited src/main.rs. Command. Running"
        );

        let mut failed = item("failed-1", ActivitySemanticClass::Error);
        failed.lifecycle = ActivityLifecycle::Failed;
        failed.outcome.summary = Some("Command exited with status 1".into());
        assert_eq!(
            presentation(&failed).accessibility_label(),
            "Builder edited src/main.rs. Error. Failed. Command exited with status 1"
        );
    }

    #[test]
    fn collaborative_timeline_render_labels_unknown_events_truthfully() {
        let mut unknown = item("future-1", ActivitySemanticClass::Generic);
        unknown.details = None;

        assert_eq!(
            presentation(&unknown).summary,
            "Builder reported an unsupported activity event"
        );
        assert_eq!(
            presentation(&unknown).detail.as_deref(),
            Some("Unsupported Acp event future-1")
        );
    }
}
