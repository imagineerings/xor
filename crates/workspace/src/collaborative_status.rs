use gpui::{App, Entity, IntoElement, RenderOnce, Role, SharedString};
use project::Project;
use ui::{Divider, prelude::*};

use crate::collaborative_accessibility::project_status_label;
use crate::collaborative_participants::CollaborativeExecutionPhase;
use crate::collaborative_review_summary::CollaborativeReviewSummary;

const NO_PROJECT_LABEL: &str = "No project";
const NO_REPOSITORY_LABEL: &str = "No repository";
const DETACHED_HEAD_LABEL: &str = "Detached HEAD";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaborativeTaskPhase {
    Running,
    WaitingForUser,
    Failed,
    Completed,
}

impl CollaborativeTaskPhase {
    pub fn from_execution_phase(phase: CollaborativeExecutionPhase) -> Option<Self> {
        match phase {
            CollaborativeExecutionPhase::Running => Some(Self::Running),
            CollaborativeExecutionPhase::WaitingForUser => Some(Self::WaitingForUser),
            CollaborativeExecutionPhase::Failed => Some(Self::Failed),
            CollaborativeExecutionPhase::Completed => Some(Self::Completed),
            CollaborativeExecutionPhase::Idle | CollaborativeExecutionPhase::Unknown => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "Task running",
            Self::WaitingForUser => "Waiting for user",
            Self::Failed => "Task failed",
            Self::Completed => "Task completed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollaborativeRepositoryStatus {
    pub label: SharedString,
    pub branch: Option<SharedString>,
    pub dirty: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollaborativeStatusProjection {
    pub project: SharedString,
    pub worktree: Option<SharedString>,
    pub repository: Option<CollaborativeRepositoryStatus>,
    pub changed_files: u32,
    pub additions: u32,
    pub deletions: u32,
    pub task: Option<CollaborativeTaskPhase>,
}

impl CollaborativeStatusProjection {
    pub fn from_project(
        project: &Entity<Project>,
        review_summary: Option<&CollaborativeReviewSummary>,
        task: Option<CollaborativeTaskPhase>,
        cx: &App,
    ) -> Self {
        let project = project.read(cx);
        let worktree = project.visible_worktrees(cx).next().map(|worktree| {
            let worktree = worktree.read(cx);
            SharedString::from(worktree.root_name_str().to_owned())
        });
        let project_label = worktree
            .clone()
            .filter(|label| !label.trim().is_empty())
            .unwrap_or_else(|| NO_PROJECT_LABEL.into());

        let repository = project.active_repository(cx).map(|repository| {
            let repository = repository.read(cx);
            let label = repository
                .work_directory_abs_path
                .file_name()
                .map(|label| label.to_string_lossy().into_owned())
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| repository.work_directory_abs_path.to_string_lossy().into());
            CollaborativeRepositoryStatus {
                label: label.into(),
                branch: repository
                    .branch
                    .as_ref()
                    .map(|branch| SharedString::from(branch.name().to_owned())),
                dirty: repository.status().next().is_some(),
            }
        });

        let (changed_files, additions, deletions) = review_summary
            .map(|summary| {
                (
                    u32::try_from(summary.files().len()).unwrap_or(u32::MAX),
                    summary.additions(),
                    summary.deletions(),
                )
            })
            .or_else(|| {
                project.active_repository(cx).map(|repository| {
                    repository.read(cx).status().fold(
                        (0_u32, 0_u32, 0_u32),
                        |(files, additions, deletions), status| {
                            let diff_stat = status.diff_stat.unwrap_or_default();
                            (
                                files.saturating_add(1),
                                additions.saturating_add(diff_stat.added),
                                deletions.saturating_add(diff_stat.deleted),
                            )
                        },
                    )
                })
            })
            .unwrap_or_default();

        Self {
            project: project_label,
            worktree,
            repository,
            changed_files,
            additions,
            deletions,
            task,
        }
    }

    pub fn repository_label(&self) -> SharedString {
        self.repository
            .as_ref()
            .map(|repository| repository.label.clone())
            .unwrap_or_else(|| NO_REPOSITORY_LABEL.into())
    }

    pub fn branch_label(&self) -> SharedString {
        self.repository
            .as_ref()
            .and_then(|repository| repository.branch.clone())
            .unwrap_or_else(|| DETACHED_HEAD_LABEL.into())
    }
}

#[derive(IntoElement)]
pub(crate) struct CollaborativeStatus {
    projection: CollaborativeStatusProjection,
}

impl CollaborativeStatus {
    pub(crate) fn new(projection: CollaborativeStatusProjection) -> Self {
        Self { projection }
    }
}

impl RenderOnce for CollaborativeStatus {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let repository_available = self.projection.repository.is_some();
        let repository_dirty = self
            .projection
            .repository
            .as_ref()
            .is_some_and(|repository| repository.dirty);
        let repository_label = self.projection.repository_label();
        let branch_label = self.projection.branch_label();
        let accessibility_label = project_status_label(&self.projection);
        let accessibility_role = if self.projection.task == Some(CollaborativeTaskPhase::Failed) {
            Role::Alert
        } else {
            Role::Status
        };

        h_flex()
            .id("collaborative-project-status")
            .debug_selector(|| "COLLABORATIVE-PROJECT-STATUS".to_owned())
            .role(accessibility_role)
            .aria_label(accessibility_label)
            .min_w_0()
            .gap_1()
            .child(Icon::new(IconName::Folder).size(IconSize::XSmall))
            .child(
                Label::new(self.projection.project)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .child(Divider::vertical())
            .child(
                Label::new(repository_label)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .when(repository_available, |this| {
                this.child(Icon::new(IconName::GitBranch).size(IconSize::XSmall))
                    .child(
                        Label::new(branch_label)
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    )
                    .when(repository_dirty, |this| {
                        this.child(
                            Label::new("Modified")
                                .size(LabelSize::XSmall)
                                .color(Color::Warning),
                        )
                    })
            })
            .child(Divider::vertical())
            .child(
                Label::new(format!(
                    "{} files  +{} −{}",
                    self.projection.changed_files,
                    self.projection.additions,
                    self.projection.deletions
                ))
                .size(LabelSize::XSmall)
                .color(Color::Muted),
            )
            .when_some(self.projection.task, |this, task| {
                this.child(Divider::vertical()).child(
                    Label::new(task.label())
                        .size(LabelSize::XSmall)
                        .color(match task {
                            CollaborativeTaskPhase::Failed => Color::Error,
                            CollaborativeTaskPhase::WaitingForUser => Color::Warning,
                            CollaborativeTaskPhase::Running | CollaborativeTaskPhase::Completed => {
                                Color::Muted
                            }
                        }),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use fs::FakeFs;
    use gpui::{AppContext as _, TestAppContext};
    use serde_json::json;
    use settings::SettingsStore;
    use util::{path, rel_path::RelPath};

    use crate::{
        collaborative_review::CollaborativeReviewSlot,
        collaborative_review_summary::{
            CollaborativeReviewFileSummary, CollaborativeReviewSummarySource,
        },
    };

    use super::*;

    #[test]
    fn collaborative_status() {
        let missing_repository = CollaborativeStatusProjection {
            project: "sandbox".into(),
            worktree: Some("sandbox".into()),
            repository: None,
            changed_files: 0,
            additions: 0,
            deletions: 0,
            task: None,
        };
        assert_eq!(
            missing_repository.repository_label().as_ref(),
            NO_REPOSITORY_LABEL
        );

        let dirty_branch = CollaborativeStatusProjection {
            project: "zed".into(),
            worktree: Some("zed".into()),
            repository: Some(CollaborativeRepositoryStatus {
                label: "zed".into(),
                branch: Some("feature/collaborative".into()),
                dirty: true,
            }),
            changed_files: 3,
            additions: 21,
            deletions: 8,
            task: Some(CollaborativeTaskPhase::Running),
        };
        assert_eq!(
            dirty_branch.branch_label().as_ref(),
            "feature/collaborative"
        );
        assert!(
            dirty_branch
                .repository
                .as_ref()
                .is_some_and(|repository| repository.dirty)
        );
        assert_eq!(
            dirty_branch.task.map(CollaborativeTaskPhase::label),
            Some("Task running")
        );

        let waiting = CollaborativeStatusProjection {
            task: Some(CollaborativeTaskPhase::WaitingForUser),
            ..dirty_branch
        };
        assert_eq!(
            waiting.task.map(CollaborativeTaskPhase::label),
            Some("Waiting for user")
        );
        assert_eq!(
            (waiting.changed_files, waiting.additions, waiting.deletions),
            (3, 21, 8)
        );
    }

    #[gpui::test]
    async fn collaborative_status_from_project(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let settings_store = SettingsStore::test(cx);
            cx.set_global(settings_store);
            cx.set_global(db::AppDatabase::test_new());
            theme_settings::init(theme::LoadThemes::JustBase, cx);
        });

        let file_system = FakeFs::new(cx.executor());
        file_system
            .insert_tree(
                path!("/project"),
                json!({
                    ".git": {},
                    "src": { "lib.rs": "pub fn value() -> u32 { 2 }\n" },
                }),
            )
            .await;
        let dot_git = Path::new(path!("/project/.git"));
        file_system.set_head_and_index_for_repo(
            dot_git,
            &[("src/lib.rs", "pub fn value() -> u32 { 1 }\n".into())],
        );
        file_system.set_branch_name(dot_git, Some("feature/collaborative"));
        let project = Project::test(file_system.clone(), [Path::new(path!("/project"))], cx).await;
        project
            .update(cx, |project, cx| project.git_scans_complete(cx))
            .await;
        cx.run_until_parked();

        let (worktree_id, provider_id) = cx.update(|cx| {
            let worktree_id = project
                .read(cx)
                .visible_worktrees(cx)
                .next()
                .expect("repository fixture should have a visible worktree")
                .read(cx)
                .id();
            (worktree_id, cx.new(|_| ()).entity_id())
        });
        let review_summary = CollaborativeReviewSummary::new(
            CollaborativeReviewSummarySource::new(
                CollaborativeReviewSlot::ProjectChanges,
                provider_id,
                1,
            ),
            vec![
                CollaborativeReviewFileSummary::new(
                    "lib-file",
                    project::ProjectPath {
                        worktree_id,
                        path: RelPath::from_unix_str("src/lib.rs")
                            .expect("fixture path should be relative")
                            .into(),
                    },
                )
                .expect("fixture file should have a stable identity"),
            ],
            Some("lib-file".into()),
            7,
            2,
        )
        .expect("fixture review summary should be valid");
        let running = cx.update(|cx| {
            CollaborativeStatusProjection::from_project(
                &project,
                Some(&review_summary),
                Some(CollaborativeTaskPhase::Running),
                cx,
            )
        });
        assert!(
            running
                .repository
                .as_ref()
                .is_some_and(|repository| repository.dirty)
        );
        assert_ne!(running.branch_label().as_ref(), DETACHED_HEAD_LABEL);
        assert_eq!(
            (running.changed_files, running.additions, running.deletions),
            (1, 7, 2)
        );
        assert_eq!(
            running.task.map(CollaborativeTaskPhase::label),
            Some("Task running")
        );

        let waiting = cx.update(|cx| {
            CollaborativeStatusProjection::from_project(
                &project,
                None,
                Some(CollaborativeTaskPhase::WaitingForUser),
                cx,
            )
        });
        assert_eq!(
            waiting.task.map(CollaborativeTaskPhase::label),
            Some("Waiting for user")
        );

        let no_repository = Project::test(file_system, [], cx).await;
        let missing = cx.update(|cx| {
            CollaborativeStatusProjection::from_project(&no_repository, None, None, cx)
        });
        assert!(missing.repository.is_none());
        assert_eq!(missing.repository_label().as_ref(), NO_REPOSITORY_LABEL);
    }
}
