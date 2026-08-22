use std::{cmp::Reverse, collections::HashMap};

use agent_client_protocol::schema::v1 as acp;
use agent_ui::thread_metadata_store::{ThreadMetadata, ThreadMetadataStore};
use agent_ui::{Agent, AgentPanel, AgentThreadSource};
use gpui::{Context, Entity, InteractiveElement, Render, SharedString, WeakEntity};
use ui::{AgentThreadStatus, Window, prelude::*};
use workspace::{
    MultiWorkspace, ProjectGroupKey,
    collaborative_navigation::{CollaborativeNavigationError, CollaborativeNavigationTarget},
};

use crate::{
    collaborative_awareness::{observe_collaborative_awareness, render_collaborative_awareness},
    collaborative_live_thread_statuses,
    collaborative_navigation::{CollaborativeNavigationBadge, CollaborativeNavigationRow},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollaborativeTaskState {
    Running,
    WaitingForUser,
    Failed,
    Draft,
    Completed,
    Archived,
}

impl CollaborativeTaskState {
    fn badge(self) -> Option<CollaborativeNavigationBadge> {
        match self {
            Self::Running => Some(CollaborativeNavigationBadge::Running),
            Self::WaitingForUser => Some(CollaborativeNavigationBadge::WaitingForUser),
            Self::Failed => Some(CollaborativeNavigationBadge::Failed),
            Self::Draft => None,
            Self::Completed => Some(CollaborativeNavigationBadge::Completed),
            Self::Archived => Some(CollaborativeNavigationBadge::Archived),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::WaitingForUser => "Waiting for user",
            Self::Failed => "Failed",
            Self::Draft => "Draft",
            Self::Completed => "Completed",
            Self::Archived => "Archived",
        }
    }

    fn priority(self) -> u8 {
        match self {
            Self::Running => 0,
            Self::WaitingForUser => 1,
            Self::Failed => 2,
            Self::Draft => 3,
            Self::Completed => 4,
            Self::Archived => 5,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct CollaborativeTaskRow {
    navigation: CollaborativeNavigationRow,
    metadata: ThreadMetadata,
    state: CollaborativeTaskState,
    updated_at: chrono::DateTime<chrono::Utc>,
}

fn task_rows(
    metadata: impl IntoIterator<Item = ThreadMetadata>,
    live_statuses: &HashMap<acp::SessionId, AgentThreadStatus>,
) -> Vec<CollaborativeTaskRow> {
    let mut rows = metadata
        .into_iter()
        .map(|thread| {
            let state = if thread.archived {
                CollaborativeTaskState::Archived
            } else if thread.is_draft() {
                CollaborativeTaskState::Draft
            } else {
                thread
                    .session_id
                    .as_ref()
                    .and_then(|session_id| live_statuses.get(session_id))
                    .map_or(CollaborativeTaskState::Completed, |status| match status {
                        AgentThreadStatus::Running => CollaborativeTaskState::Running,
                        AgentThreadStatus::WaitingForConfirmation => {
                            CollaborativeTaskState::WaitingForUser
                        }
                        AgentThreadStatus::Error => CollaborativeTaskState::Failed,
                        AgentThreadStatus::Completed => CollaborativeTaskState::Completed,
                    })
            };
            let badges = state.badge().into_iter().collect();
            let updated_at = thread.updated_at;
            CollaborativeTaskRow {
                navigation: CollaborativeNavigationRow::from_thread(&thread, badges),
                metadata: thread,
                state,
                updated_at,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| (row.state.priority(), Reverse(row.updated_at)));
    rows
}

fn task_target(metadata: &ThreadMetadata) -> CollaborativeNavigationTarget {
    CollaborativeNavigationTarget::thread(metadata.thread_id.to_key_string())
}

pub(crate) struct CollaborativeTasks {
    multi_workspace: WeakEntity<MultiWorkspace>,
    activation_error: Option<SharedString>,
}

impl CollaborativeTasks {
    pub(crate) fn new(multi_workspace: WeakEntity<MultiWorkspace>, cx: &mut Context<Self>) -> Self {
        observe_collaborative_awareness(cx);
        if let Some(thread_store) = ThreadMetadataStore::try_global(cx) {
            cx.observe(&thread_store, |_, _, cx| cx.notify()).detach();
        }
        if let Some(multi_workspace) = multi_workspace.upgrade() {
            cx.observe(&multi_workspace, |_, _, cx| cx.notify())
                .detach();
        }
        Self {
            multi_workspace,
            activation_error: None,
        }
    }

    fn rows(&self, cx: &gpui::App) -> Option<Vec<CollaborativeTaskRow>> {
        let multi_workspace = self.multi_workspace.upgrade()?;
        let metadata_store = ThreadMetadataStore::try_global(cx)?;
        let live_statuses = collaborative_live_thread_statuses(multi_workspace.read(cx), cx);
        Some(task_rows(
            metadata_store.read(cx).entries().cloned(),
            &live_statuses,
        ))
    }

    fn activate(&mut self, metadata: ThreadMetadata, window: &mut Window, cx: &mut Context<Self>) {
        let Some(multi_workspace) = self.multi_workspace.upgrade() else {
            self.activation_error = Some("Navigation is unavailable".into());
            cx.notify();
            return;
        };
        let is_available = ThreadMetadataStore::try_global(cx)
            .is_some_and(|store| store.read(cx).entry(metadata.thread_id).is_some());
        if !is_available {
            self.activation_error = Some("The selected thread is no longer available".into());
            cx.notify();
            return;
        }

        self.activation_error =
            activate_thread_target(&multi_workspace, metadata, window, cx).err();
        cx.notify();
    }
}

pub(super) fn activate_thread_target(
    multi_workspace: &Entity<MultiWorkspace>,
    metadata: ThreadMetadata,
    window: &mut Window,
    cx: &mut gpui::App,
) -> Result<(), SharedString> {
    let project_group = ProjectGroupKey::from_worktree_paths(
        &metadata.worktree_paths,
        metadata.remote_connection.clone(),
    );
    let workspace = if metadata.worktree_paths.is_empty() {
        Some(multi_workspace.read(cx).workspace().clone())
    } else {
        multi_workspace
            .read(cx)
            .last_active_workspace_for_group(&project_group, cx)
            .or_else(|| {
                multi_workspace
                    .read(cx)
                    .workspaces_for_project_group(&project_group, cx)
                    .into_iter()
                    .next()
            })
    }
    .ok_or_else(|| SharedString::from("The thread's project is no longer open"))?;
    let agent_panel = workspace
        .read(cx)
        .panel::<AgentPanel>(cx)
        .ok_or_else(|| SharedString::from("The thread surface is unavailable"))?;

    multi_workspace.update(cx, |multi_workspace, cx| {
        multi_workspace.activate(workspace.clone(), None, window, cx);
    });
    agent_panel.update(cx, |panel, cx| {
        panel.load_agent_thread(
            Agent::from(metadata.agent_id.clone()),
            metadata.thread_id,
            Some(metadata.folder_paths().clone()),
            metadata.title.clone(),
            true,
            AgentThreadSource::Sidebar,
            window,
            cx,
        );
    });
    workspace.update(cx, |workspace, cx| {
        workspace.focus_panel::<AgentPanel>(window, cx);
    });
    let target = task_target(&metadata);
    workspace
        .update(cx, |workspace, cx| {
            workspace.navigate_collaborative_to(target, |_| true, window, cx)
        })
        .map_err(navigation_error_message)?;
    Ok(())
}

fn navigation_error_message(error: CollaborativeNavigationError) -> SharedString {
    match error {
        CollaborativeNavigationError::MissingTarget(_) => {
            "The selected thread is unavailable".into()
        }
        _ => error.to_string().into(),
    }
}

impl Render for CollaborativeTasks {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let contents = match self.rows(cx) {
            None => v_flex()
                .debug_selector(|| "COLLABORATIVE-TASKS-UNAVAILABLE".to_owned())
                .child(
                    Label::new("Tasks and threads are unavailable")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
            Some(rows) if rows.is_empty() => v_flex()
                .debug_selector(|| "COLLABORATIVE-TASKS-EMPTY".to_owned())
                .child(
                    Label::new("No tasks or threads")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
            Some(rows) => v_flex()
                .gap_0p5()
                .when_some(self.activation_error.clone(), |this, error| {
                    this.child(
                        Label::new(error)
                            .size(LabelSize::XSmall)
                            .color(Color::Error),
                    )
                })
                .children(rows.into_iter().map(|row| {
                    let metadata = row.metadata.clone();
                    let awareness = render_collaborative_awareness(&row.navigation, cx);
                    h_flex()
                        .id(SharedString::from(format!(
                            "collaborative-thread-target-{}",
                            metadata.thread_id.to_key_string()
                        )))
                        .h_7()
                        .min_w_0()
                        .justify_between()
                        .gap_1()
                        .child(
                            Label::new(row.navigation.label().clone())
                                .size(LabelSize::Small)
                                .truncate(),
                        )
                        .child(
                            Label::new(SharedString::from(row.state.label()))
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        )
                        .when_some(awareness, |this, awareness| this.child(awareness))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.activate(metadata.clone(), window, cx);
                        }))
                }))
                .into_any_element(),
        };
        v_flex()
            .debug_selector(|| "COLLABORATIVE-TASKS".to_owned())
            .child(contents)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use agent_ui::{ThreadId, thread_metadata_store::WorktreePaths};
    use chrono::{TimeZone as _, Utc};
    use project::AgentId;

    use super::*;

    fn metadata(
        title: &str,
        session_id: Option<&str>,
        hour: u32,
        archived: bool,
    ) -> ThreadMetadata {
        ThreadMetadata {
            thread_id: ThreadId::new(),
            session_id: session_id.map(|id| acp::SessionId::new(Arc::<str>::from(id))),
            agent_id: AgentId("test-agent".into()),
            title: Some(title.into()),
            title_override: None,
            updated_at: Utc.with_ymd_and_hms(2026, 1, 1, hour, 0, 0).unwrap(),
            created_at: None,
            interacted_at: None,
            worktree_paths: WorktreePaths::default(),
            remote_connection: None,
            archived,
        }
    }

    #[test]
    fn collaborative_tasks_projects_state_transitions() {
        let thread = metadata("task", Some("session"), 1, false);
        let session_id = thread
            .session_id
            .clone()
            .expect("test session should exist");
        for (status, expected) in [
            (AgentThreadStatus::Running, CollaborativeTaskState::Running),
            (
                AgentThreadStatus::WaitingForConfirmation,
                CollaborativeTaskState::WaitingForUser,
            ),
            (AgentThreadStatus::Error, CollaborativeTaskState::Failed),
            (
                AgentThreadStatus::Completed,
                CollaborativeTaskState::Completed,
            ),
        ] {
            let rows = task_rows(
                [thread.clone()],
                &HashMap::from([(session_id.clone(), status)]),
            );
            assert_eq!(rows[0].state, expected);
            assert_eq!(rows[0].navigation.badges(), expected.badge().as_slice());
        }
    }

    #[test]
    fn collaborative_tasks_orders_active_history_and_archive() {
        let running = metadata("running", Some("running"), 1, false);
        let waiting = metadata("waiting", Some("waiting"), 2, false);
        let failed = metadata("failed", Some("failed"), 3, false);
        let newer_completed = metadata("newer completed", Some("newer"), 5, false);
        let older_completed = metadata("older completed", Some("older"), 4, false);
        let archived = metadata("archived", Some("archived"), 6, true);
        let statuses = HashMap::from([
            (
                running.session_id.clone().expect("running session"),
                AgentThreadStatus::Running,
            ),
            (
                waiting.session_id.clone().expect("waiting session"),
                AgentThreadStatus::WaitingForConfirmation,
            ),
            (
                failed.session_id.clone().expect("failed session"),
                AgentThreadStatus::Error,
            ),
        ]);
        let rows = task_rows(
            [
                archived,
                older_completed,
                newer_completed,
                failed,
                waiting,
                running,
            ],
            &statuses,
        );

        assert_eq!(
            rows.iter()
                .map(|row| row.navigation.label().as_ref())
                .collect::<Vec<_>>(),
            [
                "running",
                "waiting",
                "failed",
                "newer completed",
                "older completed",
                "archived",
            ]
        );
    }

    #[test]
    fn collaborative_tasks_keeps_drafts_out_of_completed_history() {
        let rows = task_rows([metadata("draft", None, 1, false)], &HashMap::new());
        assert_eq!(rows[0].state, CollaborativeTaskState::Draft);
        assert!(rows[0].navigation.badges().is_empty());
    }

    #[test]
    fn collaborative_navigation_activation_projects_task_thread_target() {
        let thread = metadata("task", Some("session"), 1, false);

        assert_eq!(
            task_target(&thread),
            CollaborativeNavigationTarget::thread(thread.thread_id.to_key_string())
        );
    }
}
