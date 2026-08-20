use std::{error::Error, fmt};

use gpui::{App, Entity, EntityId};
use project::Project;
use workspace::{
    Workspace,
    collaborative_review::{
        CollaborativeReviewRegistration, CollaborativeReviewRegistrationError,
        CollaborativeReviewSlot,
    },
};

use crate::project_diff::ProjectDiff;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollaborativeProjectReviewError {
    ProjectDiffUnavailable,
    StaleProject,
    StaleGitStore,
    Registration(CollaborativeReviewRegistrationError),
}

impl fmt::Display for CollaborativeProjectReviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectDiffUnavailable => formatter.write_str("the project diff is unavailable"),
            Self::StaleProject => formatter.write_str("the review belongs to another project"),
            Self::StaleGitStore => formatter.write_str("the review Git store is no longer current"),
            Self::Registration(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl Error for CollaborativeProjectReviewError {}

pub struct CollaborativeProjectReviewAdapter {
    project: Entity<Project>,
    git_store_id: EntityId,
    project_diff: Entity<ProjectDiff>,
}

impl CollaborativeProjectReviewAdapter {
    pub fn from_workspace(
        workspace: &Entity<Workspace>,
        cx: &App,
    ) -> Result<Self, CollaborativeProjectReviewError> {
        let workspace = workspace.read(cx);
        let project = workspace.project().clone();
        let git_store_id = project.read(cx).git_store().entity_id();
        let project_diff = workspace
            .item_of_type::<ProjectDiff>(cx)
            .ok_or(CollaborativeProjectReviewError::ProjectDiffUnavailable)?;
        Ok(Self {
            project,
            git_store_id,
            project_diff,
        })
    }

    pub fn project_diff(&self) -> &Entity<ProjectDiff> {
        &self.project_diff
    }

    pub fn register_in_workspace(
        &self,
        workspace: &mut Workspace,
        cx: &mut gpui::Context<Workspace>,
    ) -> Result<CollaborativeReviewRegistration, CollaborativeProjectReviewError> {
        if self.project.entity_id() != workspace.project().entity_id() {
            return Err(CollaborativeProjectReviewError::StaleProject);
        }
        if self.project.read(cx).git_store().entity_id() != self.git_store_id {
            return Err(CollaborativeProjectReviewError::StaleGitStore);
        }

        workspace
            .register_collaborative_review_provider(
                CollaborativeReviewSlot::ProjectChanges,
                &self.project,
                self.project_diff.clone().into(),
                cx,
            )
            .map_err(CollaborativeProjectReviewError::Registration)
    }
}

#[cfg(test)]
mod tests {
    use fs::FakeFs;
    use gpui::{BorrowAppContext as _, TestAppContext};
    use project::Project;
    use settings::{DiffViewStyle, SettingsStore};
    use workspace::MultiWorkspace;

    use super::*;

    #[gpui::test]
    async fn collaborative_project_review_adapter(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = SettingsStore::test(cx);
            cx.set_global(store);
            cx.update_global::<SettingsStore, _>(|store, cx| {
                store.update_user_settings(cx, |settings| {
                    settings.editor.diff_view_style = Some(DiffViewStyle::Unified);
                });
            });
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            editor::init(cx);
            crate::init(cx);
        });

        let fs = FakeFs::new(cx.executor());
        let project = Project::test(fs.clone(), [], cx).await;
        let other_project = Project::test(fs, [], cx).await;
        let (multi_workspace, cx) =
            cx.add_window_view(|window, cx| MultiWorkspace::test_new(project.clone(), window, cx));
        let workspace =
            multi_workspace.read_with(cx, |multi_workspace, _| multi_workspace.workspace().clone());

        let unavailable = cx
            .update(|_, cx| CollaborativeProjectReviewAdapter::from_workspace(&workspace, cx))
            .err()
            .expect("an unopened project diff should remain unavailable");
        assert_eq!(
            unavailable,
            CollaborativeProjectReviewError::ProjectDiffUnavailable
        );

        workspace.update_in(cx, |workspace, window, cx| {
            ProjectDiff::deploy_at(workspace, None, window, cx);
        });
        let adapter = cx
            .update(|_, cx| CollaborativeProjectReviewAdapter::from_workspace(&workspace, cx))
            .expect("the existing native project diff should adapt");
        let project_diff_id = adapter.project_diff().entity_id();
        workspace
            .update(cx, |workspace, cx| {
                adapter.register_in_workspace(workspace, cx)
            })
            .expect("the project diff should register in its canonical workspace");
        workspace.read_with(cx, |workspace, _| {
            assert_eq!(
                workspace
                    .collaborative_review()
                    .selected_view()
                    .expect("project review should be selected")
                    .entity_id(),
                project_diff_id
            );
        });

        let (other_multi_workspace, cx) =
            cx.add_window_view(|window, cx| MultiWorkspace::test_new(other_project, window, cx));
        let other_workspace = other_multi_workspace
            .read_with(cx, |multi_workspace, _| multi_workspace.workspace().clone());
        let stale = other_workspace
            .update(cx, |workspace, cx| {
                adapter.register_in_workspace(workspace, cx)
            })
            .expect_err("cross-project reuse should fail closed");
        assert_eq!(stale, CollaborativeProjectReviewError::StaleProject);
    }
}
