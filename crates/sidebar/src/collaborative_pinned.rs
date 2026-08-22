use std::{cmp::Reverse, collections::HashMap};

use agent_ui::thread_metadata_store::{ThreadMetadata, ThreadMetadataStore};
use chrono::{DateTime, Utc};
use gpui::{Context, InteractiveElement, Render, SharedString, Task, WeakEntity};
use ui::{Window, prelude::*};
use workspace::{
    MultiWorkspace, RecentWorkspace, WorkspaceDb,
    collaborative_navigation::CollaborativeNavigationTarget,
};

use crate::collaborative_navigation::{
    CollaborativeNavigationProjection, CollaborativeNavigationRow, CollaborativeNavigationSourceId,
};
use crate::{
    collaborative_awareness::{observe_collaborative_awareness, render_collaborative_awareness},
    collaborative_projects::activate_project_target,
    collaborative_tasks::activate_thread_target,
};

const MAX_RECENT_WORK_ROWS: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
enum CollaborativePinnedState {
    Loading,
    Ready,
    Unavailable(SharedString),
}

#[derive(Clone)]
struct RecentNavigationCandidate {
    updated_at: DateTime<Utc>,
    canonical_row: CollaborativeNavigationRow,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CollaborativePinnedProjection {
    navigation: CollaborativeNavigationProjection,
    missing_targets: Vec<CollaborativeNavigationSourceId>,
}

impl CollaborativePinnedProjection {
    fn from_sources(
        pinned_targets: &[CollaborativeNavigationSourceId],
        recent_projects: &[RecentWorkspace],
        recent_threads: impl IntoIterator<Item = ThreadMetadata>,
    ) -> Result<Self, SharedString> {
        let mut recent_candidates = Vec::new();
        for workspace in recent_projects {
            let project_row = CollaborativeNavigationRow::from_project_group(
                workspace.project_group_key(),
                workspace.project_group_key().display_name(&HashMap::new()),
                Vec::new(),
            );
            recent_candidates.push(RecentNavigationCandidate {
                updated_at: workspace.timestamp,
                canonical_row: project_row,
            });
        }
        for thread in recent_threads {
            if !thread.archived {
                recent_candidates.push(RecentNavigationCandidate {
                    updated_at: thread.updated_at,
                    canonical_row: CollaborativeNavigationRow::from_thread(&thread, Vec::new()),
                });
            }
        }
        recent_candidates.sort_by_key(|candidate| Reverse(candidate.updated_at));

        let mut candidates_by_source = HashMap::new();
        for candidate in &recent_candidates {
            let source_id = candidate.canonical_row.source_id().clone();
            if candidates_by_source
                .insert(source_id.clone(), candidate.canonical_row.clone())
                .is_some()
            {
                return Err(format!("duplicate recent source: {source_id:?}").into());
            }
        }
        let mut rows = Vec::new();
        let mut missing_targets = Vec::new();
        let mut included_sources = std::collections::HashSet::new();
        for target in pinned_targets {
            if !included_sources.insert(target.clone()) {
                return Err(format!("duplicate pinned target: {target:?}").into());
            }
            if let Some(row) = candidates_by_source.get(target) {
                rows.push(CollaborativeNavigationRow::pinned(row, Vec::new()));
            } else {
                missing_targets.push(target.clone());
            }
        }

        let mut recent_row_count = 0;
        for candidate in recent_candidates {
            if recent_row_count >= MAX_RECENT_WORK_ROWS {
                break;
            }
            let source_id = candidate.canonical_row.source_id().clone();
            if included_sources.insert(source_id) {
                rows.push(CollaborativeNavigationRow::pinned(
                    &candidate.canonical_row,
                    Vec::new(),
                ));
                recent_row_count += 1;
            }
        }

        let navigation = CollaborativeNavigationProjection::try_from_rows(rows)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            navigation,
            missing_targets,
        })
    }
}

pub(crate) struct CollaborativePinned {
    multi_workspace: Option<WeakEntity<MultiWorkspace>>,
    state: CollaborativePinnedState,
    activation_error: Option<SharedString>,
    recent_projects: Vec<RecentWorkspace>,
    pinned_targets: Vec<CollaborativeNavigationSourceId>,
    _load_task: Task<()>,
}

impl CollaborativePinned {
    pub(crate) fn new(multi_workspace: WeakEntity<MultiWorkspace>, cx: &mut Context<Self>) -> Self {
        observe_collaborative_awareness(cx);
        if let Some(thread_store) = ThreadMetadataStore::try_global(cx) {
            cx.observe(&thread_store, |_, _, cx| cx.notify()).detach();
        }

        let multi_workspace = multi_workspace.upgrade();
        let load_task = if let Some(multi_workspace) = multi_workspace.as_ref() {
            let fs = multi_workspace
                .read(cx)
                .workspace()
                .read(cx)
                .app_state()
                .fs
                .clone();
            let database = WorkspaceDb::global(cx);
            cx.spawn(async move |this, cx| {
                let result = database.recent_project_workspaces(fs.as_ref()).await;
                if let Err(error) = this.update(cx, |this, cx| match result {
                    Ok(recent_projects) => {
                        this.recent_projects = recent_projects;
                        this.state = CollaborativePinnedState::Ready;
                        cx.notify();
                    }
                    Err(error) => {
                        this.state = CollaborativePinnedState::Unavailable(
                            format!("Recent work is unavailable: {error}").into(),
                        );
                        cx.notify();
                    }
                }) {
                    log::error!("failed to update collaborative recent work: {error:#}");
                }
            })
        } else {
            Task::ready(())
        };

        let has_multi_workspace = multi_workspace.is_some();
        let multi_workspace = multi_workspace
            .as_ref()
            .map(|multi_workspace| multi_workspace.downgrade());
        Self {
            multi_workspace,
            state: if has_multi_workspace {
                CollaborativePinnedState::Loading
            } else {
                CollaborativePinnedState::Unavailable(
                    "Recent work is unavailable without an active workspace".into(),
                )
            },
            activation_error: None,
            recent_projects: Vec::new(),
            pinned_targets: Vec::new(),
            _load_task: load_task,
        }
    }

    fn projection(&self, cx: &gpui::App) -> Result<CollaborativePinnedProjection, SharedString> {
        let recent_threads = ThreadMetadataStore::try_global(cx)
            .map(|store| store.read(cx).entries().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        CollaborativePinnedProjection::from_sources(
            &self.pinned_targets,
            &self.recent_projects,
            recent_threads,
        )
    }

    fn target(row: &CollaborativeNavigationRow) -> Option<CollaborativeNavigationTarget> {
        match row.source_id() {
            CollaborativeNavigationSourceId::Project(project) => {
                Some(CollaborativeNavigationTarget::project(project))
            }
            CollaborativeNavigationSourceId::Thread(thread_id) => Some(
                CollaborativeNavigationTarget::thread(thread_id.to_key_string()),
            ),
            _ => None,
        }
    }

    fn activate(
        &mut self,
        target: CollaborativeNavigationTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(multi_workspace) = self.multi_workspace.as_ref().and_then(WeakEntity::upgrade)
        else {
            self.activation_error = Some("Navigation is unavailable".into());
            cx.notify();
            return;
        };
        let result = match &target {
            CollaborativeNavigationTarget::Project { .. }
            | CollaborativeNavigationTarget::Repository { .. }
            | CollaborativeNavigationTarget::Worktree { .. } => {
                activate_project_target(&multi_workspace, target, window, cx)
            }
            CollaborativeNavigationTarget::Thread { thread_id } => {
                let metadata = ThreadMetadataStore::try_global(cx).and_then(|store| {
                    store
                        .read(cx)
                        .entries()
                        .find(|metadata| metadata.thread_id.to_key_string() == *thread_id)
                        .cloned()
                });
                metadata.map_or_else(
                    || {
                        Err(SharedString::from(
                            "The selected thread is no longer available",
                        ))
                    },
                    |metadata| activate_thread_target(&multi_workspace, metadata, window, cx),
                )
            }
            _ => Err("The selected item is unsupported".into()),
        };
        self.activation_error = result.err();
        cx.notify();
    }
}

impl Render for CollaborativePinned {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let contents = match &self.state {
            CollaborativePinnedState::Loading => v_flex()
                .debug_selector(|| "COLLABORATIVE-PINNED-LOADING".to_owned())
                .child(
                    Label::new("Loading recent work…")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
            CollaborativePinnedState::Unavailable(message) => v_flex()
                .debug_selector(|| "COLLABORATIVE-PINNED-UNAVAILABLE".to_owned())
                .child(
                    Label::new(message.clone())
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
            CollaborativePinnedState::Ready => match self.projection(cx) {
                Ok(projection)
                    if projection.navigation.rows().is_empty()
                        && projection.missing_targets.is_empty() =>
                {
                    v_flex()
                        .debug_selector(|| "COLLABORATIVE-PINNED-EMPTY".to_owned())
                        .child(
                            Label::new("No pinned or recent work")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .into_any_element()
                }
                Ok(projection) => {
                    let missing_target_count = projection.missing_targets.len();
                    v_flex()
                        .gap_0p5()
                        .when_some(self.activation_error.clone(), |this, error| {
                            this.child(
                                Label::new(error)
                                    .size(LabelSize::XSmall)
                                    .color(Color::Error),
                            )
                        })
                        .when(missing_target_count > 0, |this| {
                            this.child(
                                Label::new(format!(
                                    "{missing_target_count} pinned item unavailable"
                                ))
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                            )
                        })
                        .children(projection.navigation.rows().iter().map(|row| {
                            let awareness = render_collaborative_awareness(row, cx);
                            let Some(target) = Self::target(row) else {
                                return Label::new("Pinned target is unavailable")
                                    .size(LabelSize::Small)
                                    .color(Color::Error)
                                    .into_any_element();
                            };
                            h_flex()
                                .id(SharedString::from(format!(
                                    "collaborative-pinned-target-{target:?}"
                                )))
                                .h_6()
                                .min_w_0()
                                .child(
                                    Label::new(row.label().clone())
                                        .size(LabelSize::Small)
                                        .truncate(),
                                )
                                .when_some(awareness, |this, awareness| this.child(awareness))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.activate(target.clone(), window, cx);
                                }))
                                .into_any_element()
                        }))
                        .into_any_element()
                }
                Err(error) => v_flex()
                    .debug_selector(|| "COLLABORATIVE-PINNED-UNAVAILABLE".to_owned())
                    .child(Label::new(error).size(LabelSize::Small).color(Color::Muted))
                    .into_any_element(),
            },
        };

        v_flex()
            .debug_selector(|| "COLLABORATIVE-PINNED".to_owned())
            .child(contents)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use agent_ui::ThreadId;
    use agent_ui::thread_metadata_store::WorktreePaths;
    use chrono::TimeZone as _;
    use project::{AgentId, ProjectGroupKey};
    use util::path_list::PathList;
    use workspace::{SerializedWorkspaceLocation, WorkspaceId};

    use super::*;

    fn recent_project(path: &str, timestamp: DateTime<Utc>) -> RecentWorkspace {
        let paths = PathList::new(&[PathBuf::from(path)]);
        RecentWorkspace {
            workspace_id: WorkspaceId::from_i64(timestamp.timestamp()),
            location: SerializedWorkspaceLocation::Local,
            paths: paths.clone(),
            identity_paths: paths,
            timestamp,
        }
    }

    fn recent_thread(title: &str, timestamp: DateTime<Utc>, archived: bool) -> ThreadMetadata {
        ThreadMetadata {
            thread_id: ThreadId::new(),
            session_id: None,
            agent_id: AgentId("test-agent".into()),
            title: Some(title.into()),
            title_override: None,
            updated_at: timestamp,
            created_at: None,
            interacted_at: None,
            worktree_paths: WorktreePaths::default(),
            remote_connection: None,
            archived,
        }
    }

    #[test]
    fn collaborative_pinned_orders_pins_before_recent_work() {
        let older = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let newer = Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap();
        let project = recent_project("/workspace/project", older);
        let thread = recent_thread("newer thread", newer, false);
        let project_source = CollaborativeNavigationSourceId::Project(ProjectGroupKey::new(
            None,
            project.identity_paths.clone(),
        ));

        let projection = CollaborativePinnedProjection::from_sources(
            std::slice::from_ref(&project_source),
            std::slice::from_ref(&project),
            [thread],
        )
        .expect("valid recent work should project");
        assert_eq!(projection.navigation.rows().len(), 2);
        assert_eq!(projection.navigation.rows()[0].source_id(), &project_source);
        assert_eq!(
            projection.navigation.rows()[1].label().as_ref(),
            "newer thread"
        );
    }

    #[test]
    fn collaborative_pinned_removes_missing_and_archived_work() {
        let timestamp = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let project = recent_project("/workspace/project", timestamp);
        let project_source = CollaborativeNavigationSourceId::Project(ProjectGroupKey::new(
            None,
            project.identity_paths.clone(),
        ));
        let archived = recent_thread("archived", timestamp, true);

        let present = CollaborativePinnedProjection::from_sources(
            std::slice::from_ref(&project_source),
            std::slice::from_ref(&project),
            [archived.clone()],
        )
        .expect("present target should project");
        assert_eq!(present.navigation.rows().len(), 1);

        let removed = CollaborativePinnedProjection::from_sources(
            std::slice::from_ref(&project_source),
            &[],
            [archived],
        )
        .expect("missing targets should remain observable");
        assert!(removed.navigation.rows().is_empty());
        assert_eq!(removed.missing_targets, vec![project_source]);
    }

    #[test]
    fn collaborative_pinned_rejects_duplicate_target_records() {
        let timestamp = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let project = recent_project("/workspace/project", timestamp);
        let source = CollaborativeNavigationSourceId::Project(ProjectGroupKey::new(
            None,
            project.identity_paths.clone(),
        ));
        let error = CollaborativePinnedProjection::from_sources(
            &[source.clone(), source],
            std::slice::from_ref(&project),
            [],
        )
        .expect_err("duplicate target records should not silently collapse");
        assert!(error.contains("duplicate pinned target"));

        let duplicate_recent =
            CollaborativePinnedProjection::from_sources(&[], &[project.clone(), project], [])
                .expect_err("duplicate recent records should not silently overwrite one another");
        assert!(duplicate_recent.contains("duplicate recent source"));
    }

    #[test]
    fn collaborative_navigation_activation_projects_pinned_targets() {
        let timestamp = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let project = recent_project("/workspace/project", timestamp);
        let thread = recent_thread("thread", timestamp, false);
        let project_row = CollaborativeNavigationRow::from_project_group(
            project.project_group_key(),
            "project",
            Vec::new(),
        );
        let thread_row = CollaborativeNavigationRow::from_thread(&thread, Vec::new());

        assert_eq!(
            CollaborativePinned::target(&project_row),
            Some(CollaborativeNavigationTarget::project(
                &project.project_group_key()
            ))
        );
        assert_eq!(
            CollaborativePinned::target(&thread_row),
            Some(CollaborativeNavigationTarget::thread(
                thread.thread_id.to_key_string()
            ))
        );
    }

    #[gpui::test]
    async fn collaborative_pinned_renders_empty_and_unavailable_states(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            let settings_store = settings::SettingsStore::test(cx);
            cx.set_global(settings_store);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
        });
        let (view, cx) = cx.add_window_view(|_, _| CollaborativePinned {
            multi_workspace: None,
            state: CollaborativePinnedState::Ready,
            activation_error: None,
            recent_projects: Vec::new(),
            pinned_targets: Vec::new(),
            _load_task: Task::ready(()),
        });
        cx.run_until_parked();
        assert!(cx.debug_bounds("COLLABORATIVE-PINNED-EMPTY").is_some());

        view.update(cx, |view, cx| {
            view.state = CollaborativePinnedState::Unavailable("Recent work unavailable".into());
            cx.notify();
        });
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("COLLABORATIVE-PINNED-UNAVAILABLE")
                .is_some()
        );
        assert!(cx.debug_bounds("COLLABORATIVE-PINNED-EMPTY").is_none());
    }
}
