use std::{collections::HashSet, path::PathBuf};

use gpui::{Context, Entity, FontWeight, InteractiveElement, Render, SharedString, WeakEntity};
use project::{ProjectGroupKey, WorktreeId};
use ui::{Window, prelude::*};
use workspace::{
    MultiWorkspace,
    collaborative_navigation::{CollaborativeNavigationError, CollaborativeNavigationTarget},
};

use crate::collaborative_navigation::{CollaborativeNavigationRow, CollaborativeNavigationRowId};

#[derive(Clone, Debug)]
struct CollaborativeProjectSource {
    key: ProjectGroupKey,
    label: SharedString,
    repositories: Vec<(PathBuf, SharedString)>,
    worktrees: Vec<(WorktreeId, PathBuf, SharedString)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CollaborativeProjectGroupProjection {
    project: CollaborativeNavigationRow,
    repositories: Vec<CollaborativeNavigationRow>,
    worktrees: Vec<CollaborativeNavigationRow>,
}

fn project_hierarchy(
    sources: impl IntoIterator<Item = CollaborativeProjectSource>,
) -> Result<Vec<CollaborativeProjectGroupProjection>, SharedString> {
    let mut row_ids = HashSet::<CollaborativeNavigationRowId>::new();
    let mut groups = Vec::new();
    for mut source in sources {
        source
            .repositories
            .sort_by(|left, right| left.0.cmp(&right.0));
        source.worktrees.sort_by(|left, right| left.1.cmp(&right.1));

        let project = CollaborativeNavigationRow::from_project_group(
            source.key.clone(),
            source.label,
            Vec::new(),
        );
        if !row_ids.insert(project.id().clone()) {
            return Err("duplicate project group in collaborative hierarchy".into());
        }

        let mut repositories = Vec::new();
        for (work_directory, label) in source.repositories {
            let row = CollaborativeNavigationRow::from_repository(
                source.key.clone(),
                work_directory,
                label,
                Vec::new(),
            );
            if row_ids.insert(row.id().clone()) {
                repositories.push(row);
            }
        }

        let mut worktrees = Vec::new();
        for (worktree_id, path, label) in source.worktrees {
            let row = CollaborativeNavigationRow::from_worktree(
                source.key.clone(),
                worktree_id,
                path,
                label,
                Vec::new(),
            );
            if row_ids.insert(row.id().clone()) {
                worktrees.push(row);
            }
        }
        groups.push(CollaborativeProjectGroupProjection {
            project,
            repositories,
            worktrees,
        });
    }
    Ok(groups)
}

pub(crate) struct CollaborativeProjects {
    multi_workspace: WeakEntity<MultiWorkspace>,
    activation_error: Option<SharedString>,
}

impl CollaborativeProjects {
    pub(crate) fn new(multi_workspace: WeakEntity<MultiWorkspace>, cx: &mut Context<Self>) -> Self {
        if let Some(multi_workspace) = multi_workspace.upgrade() {
            cx.observe(&multi_workspace, |_, _, cx| cx.notify())
                .detach();
        }
        Self {
            multi_workspace,
            activation_error: None,
        }
    }

    fn sources(&self, cx: &gpui::App) -> Option<Vec<CollaborativeProjectSource>> {
        let multi_workspace = self.multi_workspace.upgrade()?;
        Some(
            multi_workspace
                .read(cx)
                .project_groups(cx)
                .into_iter()
                .map(|group| {
                    let mut repositories = Vec::new();
                    let mut worktrees = Vec::new();
                    for workspace in group.workspaces {
                        let project = workspace.read(cx).project().clone();
                        let project = project.read(cx);
                        repositories.extend(project.repositories(cx).values().map(|repository| {
                            let repository = repository.read(cx);
                            let path = repository.work_directory_abs_path.as_ref().to_path_buf();
                            let label = path
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.display().to_string());
                            (path, label.into())
                        }));
                        worktrees.extend(project.visible_worktrees(cx).map(|worktree| {
                            let worktree = worktree.read(cx);
                            (
                                worktree.id(),
                                worktree.abs_path().as_ref().to_path_buf(),
                                SharedString::from(worktree.root_name_str().to_owned()),
                            )
                        }));
                    }
                    CollaborativeProjectSource {
                        label: group.key.display_name(&Default::default()),
                        key: group.key,
                        repositories,
                        worktrees,
                    }
                })
                .collect(),
        )
    }

    fn target(row: &CollaborativeNavigationRow) -> Option<CollaborativeNavigationTarget> {
        match row.source_id() {
            crate::collaborative_navigation::CollaborativeNavigationSourceId::Project(project) => {
                Some(CollaborativeNavigationTarget::project(project))
            }
            crate::collaborative_navigation::CollaborativeNavigationSourceId::Repository {
                project,
                work_directory,
            } => Some(CollaborativeNavigationTarget::repository(
                project,
                work_directory.clone(),
            )),
            crate::collaborative_navigation::CollaborativeNavigationSourceId::Worktree {
                project,
                worktree_id,
                path,
            } => Some(CollaborativeNavigationTarget::worktree(
                project,
                worktree_id.to_proto(),
                path.clone(),
            )),
            _ => None,
        }
    }

    fn activate(
        &mut self,
        target: CollaborativeNavigationTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(multi_workspace) = self.multi_workspace.upgrade() else {
            self.activation_error = Some("Navigation is unavailable".into());
            cx.notify();
            return;
        };
        self.activation_error = activate_project_target(&multi_workspace, target, window, cx).err();
        cx.notify();
    }
}

pub(super) fn activate_project_target(
    multi_workspace: &Entity<MultiWorkspace>,
    target: CollaborativeNavigationTarget,
    window: &mut Window,
    cx: &mut gpui::App,
) -> Result<(), SharedString> {
    let selected_workspace = multi_workspace
        .read(cx)
        .project_groups(cx)
        .into_iter()
        .find_map(|group| {
            group.workspaces.into_iter().find(|workspace| {
                let project = workspace.read(cx).project().read(cx);
                match &target {
                    CollaborativeNavigationTarget::Project {
                        project: target_project,
                    } => target_project.matches(&group.key),
                    CollaborativeNavigationTarget::Repository {
                        project: target_project,
                        work_directory,
                    } => {
                        target_project.matches(&group.key)
                            && project.repositories(cx).values().any(|repository| {
                                repository.read(cx).work_directory_abs_path.as_ref()
                                    == work_directory
                            })
                    }
                    CollaborativeNavigationTarget::Worktree {
                        project: target_project,
                        worktree_id,
                        path,
                    } => {
                        target_project.matches(&group.key)
                            && project.visible_worktrees(cx).any(|worktree| {
                                let worktree = worktree.read(cx);
                                worktree.id().to_proto() == *worktree_id
                                    && worktree.abs_path().as_ref() == path
                            })
                    }
                    _ => false,
                }
            })
        })
        .ok_or_else(|| SharedString::from("The selected project item is no longer available"))?;

    multi_workspace.update(cx, |multi_workspace, cx| {
        multi_workspace.activate(selected_workspace.clone(), None, window, cx);
    });
    selected_workspace
        .update(cx, |workspace, cx| {
            workspace.navigate_collaborative_to(target, |_| true, window, cx)
        })
        .map_err(navigation_error_message)?;
    Ok(())
}

fn navigation_error_message(error: CollaborativeNavigationError) -> SharedString {
    match error {
        CollaborativeNavigationError::MissingTarget(_) => "The selected item is unavailable".into(),
        _ => error.to_string().into(),
    }
}

impl Render for CollaborativeProjects {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let contents = match self.sources(cx) {
            None => unavailable_projects("Projects are unavailable"),
            Some(sources) => match project_hierarchy(sources) {
                Ok(groups) if groups.is_empty() => v_flex()
                    .debug_selector(|| "COLLABORATIVE-PROJECTS-EMPTY".to_owned())
                    .child(
                        Label::new("No communities or projects")
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .into_any_element(),
                Ok(groups) => v_flex()
                    .gap_1()
                    .when_some(self.activation_error.clone(), |this, error| {
                        this.child(
                            Label::new(error)
                                .size(LabelSize::XSmall)
                                .color(Color::Error),
                        )
                    })
                    .children(
                        groups
                            .into_iter()
                            .map(|group| render_project_group(group, cx)),
                    )
                    .into_any_element(),
                Err(error) => unavailable_projects(error),
            },
        };
        v_flex()
            .debug_selector(|| "COLLABORATIVE-PROJECTS".to_owned())
            .child(contents)
    }
}

fn unavailable_projects(message: impl Into<SharedString>) -> AnyElement {
    v_flex()
        .debug_selector(|| "COLLABORATIVE-PROJECTS-UNAVAILABLE".to_owned())
        .child(
            Label::new(message.into())
                .size(LabelSize::Small)
                .color(Color::Muted),
        )
        .into_any_element()
}

fn render_project_group(
    group: CollaborativeProjectGroupProjection,
    cx: &mut Context<CollaborativeProjects>,
) -> AnyElement {
    let Some(project_target) = CollaborativeProjects::target(&group.project) else {
        return unavailable_projects("Project target is unavailable");
    };
    v_flex()
        .gap_0p5()
        .child(
            h_flex()
                .id(SharedString::from(format!(
                    "collaborative-project-target-{project_target:?}"
                )))
                .h_6()
                .min_w_0()
                .child(
                    Label::new(group.project.label().clone())
                        .size(LabelSize::Small)
                        .weight(FontWeight::MEDIUM)
                        .truncate(),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.activate(project_target.clone(), window, cx);
                })),
        )
        .children(
            group
                .repositories
                .into_iter()
                .map(|row| render_project_child("Repository", row, cx)),
        )
        .children(
            group
                .worktrees
                .into_iter()
                .map(|row| render_project_child("Worktree", row, cx)),
        )
        .into_any_element()
}

fn render_project_child(
    kind: &'static str,
    row: CollaborativeNavigationRow,
    cx: &mut Context<CollaborativeProjects>,
) -> AnyElement {
    let Some(target) = CollaborativeProjects::target(&row) else {
        return unavailable_projects(format!("{kind} target is unavailable"));
    };
    h_flex()
        .id(SharedString::from(format!(
            "collaborative-{}-target-{target:?}",
            kind.to_lowercase()
        )))
        .h_6()
        .pl_2()
        .min_w_0()
        .child(
            Label::new(format!("{kind} · {}", row.label()))
                .size(LabelSize::XSmall)
                .truncate(),
        )
        .on_click(cx.listener(move |this, _, window, cx| {
            this.activate(target.clone(), window, cx);
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use util::path_list::PathList;

    use super::*;

    fn source(worktrees: &[(&str, usize)], repositories: &[&str]) -> CollaborativeProjectSource {
        let key = ProjectGroupKey::new(None, PathList::new(&[PathBuf::from("/project")]));
        CollaborativeProjectSource {
            key,
            label: "project".into(),
            repositories: repositories
                .iter()
                .map(|path| (PathBuf::from(path), SharedString::from(*path)))
                .collect(),
            worktrees: worktrees
                .iter()
                .map(|(path, id)| {
                    (
                        WorktreeId::from_usize(*id),
                        PathBuf::from(path),
                        SharedString::from(*path),
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn collaborative_projects_projects_multiple_repositories_and_worktrees() {
        let groups = project_hierarchy([source(
            &[("/project", 1), ("/worktrees/feature", 1)],
            &["/project/nested", "/project"],
        )])
        .expect("valid hierarchy should project");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].repositories.len(), 2);
        assert_eq!(groups[0].repositories[0].label().as_ref(), "/project");
        assert_eq!(groups[0].worktrees.len(), 2);
        assert_eq!(groups[0].worktrees[0].label().as_ref(), "/project");
        assert_eq!(
            groups[0].worktrees[1].label().as_ref(),
            "/worktrees/feature"
        );
        assert_ne!(groups[0].worktrees[0].id(), groups[0].worktrees[1].id());
    }

    #[test]
    fn collaborative_projects_removes_deleted_worktrees() {
        let before =
            project_hierarchy([source(&[("/project", 1), ("/worktrees/feature", 2)], &[])])
                .expect("initial hierarchy should project");
        let after = project_hierarchy([source(&[("/project", 1)], &[])])
            .expect("updated hierarchy should project");

        assert_eq!(before[0].worktrees.len(), 2);
        assert_eq!(after[0].worktrees.len(), 1);
        assert!(
            after[0]
                .worktrees
                .iter()
                .all(|row| row.label().as_ref() != "/worktrees/feature")
        );
    }

    #[test]
    fn collaborative_projects_rejects_duplicate_groups() {
        let source = source(&[], &[]);
        let error = project_hierarchy([source.clone(), source])
            .expect_err("duplicate project groups should fail explicitly");
        assert!(error.contains("duplicate project group"));
    }

    #[test]
    fn collaborative_navigation_activation_projects_hierarchy_targets() {
        let groups = project_hierarchy([source(&[("/project", 1)], &["/project/repository"])])
            .expect("valid hierarchy should project");
        let group = &groups[0];
        let project_key = ProjectGroupKey::new(None, PathList::new(&[PathBuf::from("/project")]));

        assert_eq!(
            CollaborativeProjects::target(&group.project),
            Some(CollaborativeNavigationTarget::project(&project_key))
        );
        assert_eq!(
            CollaborativeProjects::target(&group.repositories[0]),
            Some(CollaborativeNavigationTarget::repository(
                &project_key,
                PathBuf::from("/project/repository")
            ))
        );
        assert_eq!(
            CollaborativeProjects::target(&group.worktrees[0]),
            Some(CollaborativeNavigationTarget::worktree(
                &project_key,
                1,
                PathBuf::from("/project")
            ))
        );
    }
}
