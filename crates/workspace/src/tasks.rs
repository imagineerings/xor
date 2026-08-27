use std::process::ExitStatus;

use anyhow::{Context as _, Result, anyhow, bail};
use collections::HashSet;
use gpui::{AppContext, AsyncWindowContext, Context, Entity, Task, TaskExt, WeakEntity};
use language::Buffer;
use project::{TaskSourceKind, WorktreeId};
use remote::ConnectionState;
use task::{
    DebugScenario, ResolvedTask, SaveStrategy, SharedTaskContext, SpawnInTerminal,
    StructuredTaskHandle, TaskArtifact, TaskContext, TaskHook, TaskTemplate, TaskVariables,
    VariableName,
};
use ui::Window;
use util::TryFutureExt;

use crate::{SaveIntent, Toast, Workspace, notifications::NotificationId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduledTaskResult {
    Success,
    Failure,
    SpawnFailed,
    Cancelled,
}

type TaskCompletionHandler = Box<dyn FnOnce(ScheduledTaskResult, &mut AsyncWindowContext)>;

pub fn resolve_task_reference(
    tasks: Vec<(TaskSourceKind, TaskTemplate)>,
    label: &str,
    task_context: &TaskContext,
) -> Result<(TaskSourceKind, ResolvedTask)> {
    let mut matches = tasks
        .into_iter()
        .filter(|(_, template)| template.label == label);
    let (source, template) = matches
        .next()
        .with_context(|| format!("pre-launch task `{label}` was not found"))?;
    if matches.next().is_some() {
        bail!("pre-launch task reference `{label}` is ambiguous");
    }
    let resolved = template
        .resolve_task(&source.to_id_base(), task_context)
        .with_context(|| format!("pre-launch task `{label}` could not be resolved"))?;
    Ok((source, resolved))
}

impl Workspace {
    pub fn schedule_task_reference_with_completion(
        &mut self,
        label: String,
        worktree_id: WorktreeId,
        task_context: TaskContext,
        omit_history: bool,
        on_complete: impl FnOnce(ScheduledTaskResult, &mut AsyncWindowContext) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let inventory = self
            .project
            .read(cx)
            .task_store()
            .read(cx)
            .task_inventory()
            .cloned();
        let task_list = inventory.map(|inventory| {
            inventory
                .read(cx)
                .list_tasks(None, None, Some(worktree_id), cx)
        });
        let workspace = cx.entity().downgrade();
        let mut on_complete = Some(Box::new(on_complete) as TaskCompletionHandler);
        let task = cx.spawn_in(window, async move |_workspace, cx| {
            let resolution = match task_list {
                Some(task_list) => resolve_task_reference(task_list.await, &label, &task_context),
                None => Err(anyhow::anyhow!("task inventory is unavailable")),
            };
            match resolution {
                Ok((source, resolved)) => {
                    let Some(completion) = on_complete.take() else {
                        log::error!("Pre-launch completion handler is unavailable");
                        return;
                    };
                    if let Err(error) = workspace.update_in(cx, |workspace, window, cx| {
                        workspace.schedule_resolved_task_with_completion(
                            source,
                            resolved,
                            omit_history,
                            completion,
                            window,
                            cx,
                        );
                    }) {
                        log::debug!(
                            "Workspace closed before pre-launch task scheduling: {error:#}"
                        );
                    }
                }
                Err(error) => {
                    log::error!("Cargo pre-launch task could not start: {error:#}");
                    if let Err(update_error) = workspace.update(cx, |workspace, cx| {
                        let id = NotificationId::unique::<ResolvedTask>();
                        workspace.show_toast(
                            Toast::new(id, format!("Pre-launch task could not start: {error}")),
                            cx,
                        );
                    }) {
                        log::debug!(
                            "Workspace closed before pre-launch error display: {update_error:#}"
                        );
                    }
                    if let Some(on_complete) = on_complete.take() {
                        on_complete(ScheduledTaskResult::SpawnFailed, cx);
                    }
                }
            }
        });
        self.scheduled_tasks.push(task);
    }

    pub fn schedule_task(
        self: &mut Workspace,
        task_source_kind: TaskSourceKind,
        task_to_resolve: &TaskTemplate,
        task_cx: &TaskContext,
        omit_history: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.project.read(cx).remote_connection_state(cx) {
            None | Some(ConnectionState::Connected) => {}
            Some(
                ConnectionState::Connecting
                | ConnectionState::Disconnected
                | ConnectionState::HeartbeatMissed
                | ConnectionState::Reconnecting,
            ) => {
                log::warn!("Cannot schedule tasks when disconnected from a remote host");
                return;
            }
        }

        if let Some(spawn_in_terminal) =
            task_to_resolve.resolve_task(&task_source_kind.to_id_base(), task_cx)
        {
            self.schedule_resolved_task(
                task_source_kind,
                spawn_in_terminal,
                omit_history,
                window,
                cx,
            );
        }
    }

    pub fn schedule_resolved_task(
        self: &mut Workspace,
        task_source_kind: TaskSourceKind,
        resolved_task: ResolvedTask,
        omit_history: bool,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        self.schedule_resolved_task_internal(
            task_source_kind,
            resolved_task,
            omit_history,
            None,
            None,
            window,
            cx,
        );
    }

    pub fn schedule_resolved_task_with_structured_handle(
        &mut self,
        task_source_kind: TaskSourceKind,
        resolved_task: ResolvedTask,
        omit_history: bool,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> StructuredTaskHandle {
        let handle = StructuredTaskHandle::new(resolved_task.resolved.id.clone());
        if let Some(message) =
            structured_task_connection_error(self.project.read(cx).remote_connection_state(cx))
        {
            handle.mark_spawn_error(message, cx);
            return handle;
        }
        self.schedule_resolved_task_internal(
            task_source_kind,
            resolved_task,
            omit_history,
            Some(handle.clone()),
            None,
            window,
            cx,
        );
        handle
    }

    pub fn schedule_resolved_task_with_completion(
        self: &mut Workspace,
        task_source_kind: TaskSourceKind,
        resolved_task: ResolvedTask,
        omit_history: bool,
        on_complete: impl FnOnce(ScheduledTaskResult, &mut AsyncWindowContext) + 'static,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        self.schedule_resolved_task_internal(
            task_source_kind,
            resolved_task,
            omit_history,
            None,
            Some(Box::new(on_complete)),
            window,
            cx,
        );
    }

    fn schedule_resolved_task_internal(
        self: &mut Workspace,
        task_source_kind: TaskSourceKind,
        resolved_task: ResolvedTask,
        omit_history: bool,
        structured_handle: Option<StructuredTaskHandle>,
        on_complete: Option<TaskCompletionHandler>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let spawn_in_terminal = resolved_task.resolved.clone();
        let task_artifact = resolved_task.resolved_artifact().cloned();
        let task_working_directory = spawn_in_terminal.cwd.clone();
        if !omit_history {
            if let Some(debugger_provider) = self.debugger_provider.as_ref() {
                debugger_provider.task_scheduled(cx);
            }

            self.project().update(cx, |project, cx| {
                if let Some(task_inventory) =
                    project.task_store().read(cx).task_inventory().cloned()
                {
                    task_inventory.update(cx, |inventory, _| {
                        inventory.task_scheduled(task_source_kind, resolved_task);
                    })
                }
            });
        }

        if self.terminal_provider.is_some() {
            let task = cx.spawn_in(window, async move |workspace, cx| {
                Self::save_for_task(&workspace, spawn_in_terminal.save, cx).await;

                if structured_handle
                    .as_ref()
                    .is_some_and(|handle| handle.state().is_terminal())
                {
                    return;
                }

                let spawn_task = workspace.update_in(cx, |workspace, window, cx| {
                    workspace
                        .terminal_provider
                        .as_ref()
                        .map(|terminal_provider| {
                            if let Some(handle) = structured_handle.clone() {
                                terminal_provider.spawn_structured(
                                    spawn_in_terminal,
                                    handle,
                                    window,
                                    cx,
                                )
                            } else {
                                terminal_provider.spawn(spawn_in_terminal, window, cx)
                            }
                        })
                });
                if let Some(spawn_task) = spawn_task.ok().flatten() {
                    let res = cx.background_spawn(spawn_task).await;
                    let result = match res {
                        Some(Ok(status)) => {
                            if let Some(handle) = structured_handle.as_ref() {
                                if let Err(error) = cx.update(|_, cx| {
                                    handle.mark_completed(status, cx);
                                }) {
                                    log::debug!("Structured task window closed: {error:#}");
                                }
                            }
                            if status.success() {
                                log::debug!("Task spawn succeeded");
                                ScheduledTaskResult::Success
                            } else {
                                log::debug!("Task spawn failed, code: {:?}", status.code());
                                ScheduledTaskResult::Failure
                            }
                        }
                        Some(Err(e)) => {
                            if let Some(handle) = structured_handle.as_ref() {
                                if let Err(error) = cx.update(|_, cx| {
                                    handle.mark_spawn_error(e.to_string(), cx);
                                }) {
                                    log::debug!("Structured task window closed: {error:#}");
                                }
                            }
                            log::error!("Task spawn failed: {e:#}");
                            if let Err(error) = workspace.update(cx, |w, cx| {
                                let id = NotificationId::unique::<ResolvedTask>();
                                w.show_toast(Toast::new(id, format!("Task spawn failed: {e}")), cx);
                            }) {
                                log::debug!("Task error toast could not be shown: {error:#}");
                            }
                            ScheduledTaskResult::SpawnFailed
                        }
                        None => {
                            if let Some(handle) = structured_handle.as_ref()
                                && let Err(error) = cx.update(|_, cx| {
                                    handle.mark_cancelled(handle.state().terminal_id(), false, cx);
                                })
                            {
                                log::debug!("Structured task window closed: {error:#}");
                            }
                            log::debug!("Task spawn got cancelled");
                            ScheduledTaskResult::Cancelled
                        }
                    };
                    if result == ScheduledTaskResult::Success
                        && let Some(artifact) = task_artifact
                        && artifact.kind != task::TaskArtifactKind::Data
                        && let Err(error) = workspace.update_in(cx, |workspace, window, cx| {
                            workspace.open_task_artifact(
                                artifact,
                                task_working_directory.as_deref(),
                                window,
                                cx,
                            );
                        })
                    {
                        log::debug!(
                            "Task artifact could not be opened after workspace close: {error:#}"
                        );
                    }
                    if let Some(on_complete) = on_complete {
                        on_complete(result, cx);
                    }
                } else {
                    if let Some(handle) = structured_handle.as_ref()
                        && let Err(error) = cx.update(|_, cx| {
                            handle.mark_cancelled(handle.state().terminal_id(), false, cx);
                        })
                    {
                        log::debug!("Structured task window closed: {error:#}");
                    }
                    if let Some(on_complete) = on_complete {
                        on_complete(ScheduledTaskResult::Cancelled, cx);
                    }
                }
            });
            self.scheduled_tasks.push(task);
        } else if let Some(handle) = structured_handle {
            handle.mark_spawn_error("No terminal provider is available", cx);
        }
    }

    fn open_task_artifact(
        &mut self,
        artifact: TaskArtifact,
        task_working_directory: Option<&std::path::Path>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match resolve_task_artifact(self.project.read(cx), &artifact, task_working_directory, cx) {
            Ok(project_path) => self
                .open_path(project_path, None, true, window, cx)
                .detach_and_log_err(cx),
            Err(error) => self.show_toast(
                Toast::new(
                    NotificationId::unique::<TaskArtifact>(),
                    format!("Task artifact could not be opened: {error}"),
                ),
                cx,
            ),
        }
    }

    pub async fn save_for_task(
        workspace: &WeakEntity<Self>,
        save_strategy: SaveStrategy,
        cx: &mut AsyncWindowContext,
    ) {
        let save_action = match save_strategy {
            SaveStrategy::All => {
                let save_all = workspace.update_in(cx, |workspace, window, cx| {
                    let task = workspace.save_all_internal(SaveIntent::SaveAll, true, window, cx);
                    cx.background_spawn(async { task.await.map(|_| ()) })
                });
                save_all.ok()
            }
            SaveStrategy::Current => {
                let save_current = workspace.update_in(cx, |workspace, window, cx| {
                    workspace.save_active_item(SaveIntent::SaveAll, window, cx)
                });
                save_current.ok()
            }
            SaveStrategy::None => None,
        };
        if let Some(save_action) = save_action {
            save_action.log_err().await;
        }
    }

    pub fn start_debug_session(
        &mut self,
        scenario: DebugScenario,
        task_context: SharedTaskContext,
        active_buffer: Option<Entity<Buffer>>,
        worktree_id: Option<WorktreeId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(provider) = self.debugger_provider.as_mut() {
            provider.start_session(
                scenario,
                task_context,
                active_buffer,
                worktree_id,
                window,
                cx,
            )
        }
    }

    pub fn spawn_in_terminal(
        self: &mut Workspace,
        spawn_in_terminal: SpawnInTerminal,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Task<Option<Result<ExitStatus>>> {
        if let Some(terminal_provider) = self.terminal_provider.as_ref() {
            terminal_provider.spawn(spawn_in_terminal, window, cx)
        } else {
            Task::ready(None)
        }
    }

    pub fn run_create_worktree_tasks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let project = self.project().clone();
        let hooks = HashSet::from_iter([TaskHook::CreateWorktree]);

        let worktree_tasks: Vec<(WorktreeId, TaskContext, Vec<TaskTemplate>)> = {
            let project = project.read(cx);
            let task_store = project.task_store();
            let Some(inventory) = task_store.read(cx).task_inventory().cloned() else {
                return;
            };

            let git_store = project.git_store().read(cx);

            let mut worktree_tasks = Vec::new();
            for worktree in project.worktrees(cx) {
                let worktree = worktree.read(cx);
                let worktree_id = worktree.id();
                let worktree_abs_path = worktree.abs_path();

                let templates: Vec<TaskTemplate> = inventory
                    .read(cx)
                    .templates_with_hooks(&hooks, worktree_id)
                    .into_iter()
                    .map(|(_, template)| template)
                    .collect();

                if templates.is_empty() {
                    continue;
                }

                let mut task_variables = TaskVariables::default();
                task_variables.insert(
                    VariableName::WorktreeRoot,
                    worktree_abs_path.to_string_lossy().into_owned(),
                );

                if let Some(path) = git_store.original_repo_path_for_worktree(worktree_id, cx) {
                    task_variables.insert(
                        VariableName::MainGitWorktree,
                        path.to_string_lossy().into_owned(),
                    );
                }

                let task_context = TaskContext {
                    cwd: Some(worktree_abs_path.to_path_buf()),
                    task_variables,
                    project_env: Default::default(),
                };

                worktree_tasks.push((worktree_id, task_context, templates));
            }
            worktree_tasks
        };

        if worktree_tasks.is_empty() {
            return;
        }

        let task = cx.spawn_in(window, async move |workspace, cx| {
            let mut tasks = Vec::new();
            for (worktree_id, task_context, templates) in worktree_tasks {
                let id_base = format!("worktree_setup_{worktree_id}");

                tasks.push(cx.spawn({
                    let workspace = workspace.clone();
                    async move |cx| {
                        for task_template in templates {
                            let Some(resolved) =
                                task_template.resolve_task(&id_base, &task_context)
                            else {
                                continue;
                            };

                            let status = workspace.update_in(cx, |workspace, window, cx| {
                                workspace.spawn_in_terminal(resolved.resolved, window, cx)
                            })?;

                            if let Some(result) = status.await {
                                match result {
                                    Ok(exit_status) if !exit_status.success() => {
                                        log::error!(
                                            "Git worktree setup task failed with status: {:?}",
                                            exit_status.code()
                                        );
                                        break;
                                    }
                                    Err(error) => {
                                        log::error!("Git worktree setup task error: {error:#}");
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        anyhow::Ok(())
                    }
                }));
            }

            futures::future::join_all(tasks).await;
            anyhow::Ok(())
        });
        task.detach_and_log_err(cx);
    }
}

pub fn resolve_task_artifact(
    project: &project::Project,
    artifact: &TaskArtifact,
    task_working_directory: Option<&std::path::Path>,
    cx: &gpui::App,
) -> Result<project::ProjectPath> {
    let candidate = task_working_directory
        .map(|working_directory| working_directory.join(&artifact.path))
        .unwrap_or_else(|| artifact.path.clone().into());
    let project_path = project
        .find_project_path(&candidate, cx)
        .ok_or_else(|| anyhow!("{} is not visible in the project", artifact.path))?;
    let entry = project
        .entry_for_path(&project_path, cx)
        .ok_or_else(|| anyhow!("{} was not created", artifact.path))?;
    if !entry.is_file() {
        bail!("{} is not a file", artifact.path);
    }
    if entry.size > artifact.max_bytes {
        bail!(
            "{} exceeds the declared {}-byte limit",
            artifact.path,
            artifact.max_bytes
        );
    }
    Ok(project_path)
}

fn structured_task_connection_error(state: Option<ConnectionState>) -> Option<&'static str> {
    match state {
        None | Some(ConnectionState::Connected) => None,
        Some(ConnectionState::Connecting) => Some("The project host is still connecting"),
        Some(
            ConnectionState::Disconnected
            | ConnectionState::HeartbeatMissed
            | ConnectionState::Reconnecting,
        ) => Some("The project host is disconnected"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        TerminalProvider,
        item::test::{TestItem, TestProjectItem},
        register_serializable_item,
    };
    use gpui::{App, TestAppContext};
    use parking_lot::Mutex;
    use project::{FakeFs, Project, TaskSourceKind};
    use serde_json::json;
    use std::sync::Arc;
    use task::{TaskArtifactKind, TaskTemplate};

    struct Fixture {
        workspace: Entity<Workspace>,
        item: Entity<TestItem>,
        task: ResolvedTask,
        dirty_before_spawn: Arc<Mutex<Option<bool>>>,
    }

    #[gpui::test]
    async fn test_schedule_resolved_task_save_all(cx: &mut TestAppContext) {
        let (fixture, cx) = create_fixture(cx, SaveStrategy::All).await;
        fixture.workspace.update_in(cx, |workspace, window, cx| {
            workspace.schedule_resolved_task(
                TaskSourceKind::UserInput,
                fixture.task,
                false,
                window,
                cx,
            );
        });
        cx.executor().run_until_parked();

        assert_eq!(*fixture.dirty_before_spawn.lock(), Some(false));
        assert!(cx.read(|cx| !fixture.item.read(cx).is_dirty));
    }

    #[gpui::test]
    async fn test_schedule_resolved_task_save_current(cx: &mut TestAppContext) {
        let (fixture, cx) = create_fixture(cx, SaveStrategy::Current).await;
        // Add a second inactive dirty item
        let inactive = add_test_item(&fixture.workspace, "file2.txt", false, cx);
        fixture.workspace.update_in(cx, |workspace, window, cx| {
            workspace.schedule_resolved_task(
                TaskSourceKind::UserInput,
                fixture.task,
                false,
                window,
                cx,
            );
        });
        cx.executor().run_until_parked();

        // The active item (fixture.item) should be saved
        assert_eq!(*fixture.dirty_before_spawn.lock(), Some(false));
        assert!(cx.read(|cx| !fixture.item.read(cx).is_dirty));
        // The inactive item should not be saved
        assert!(cx.read(|cx| inactive.read(cx).is_dirty));
    }

    #[gpui::test]
    async fn test_schedule_resolved_task_save_none(cx: &mut TestAppContext) {
        let (fixture, cx) = create_fixture(cx, SaveStrategy::None).await;
        fixture.workspace.update_in(cx, |workspace, window, cx| {
            workspace.schedule_resolved_task(
                TaskSourceKind::UserInput,
                fixture.task,
                false,
                window,
                cx,
            );
        });
        cx.executor().run_until_parked();

        assert_eq!(*fixture.dirty_before_spawn.lock(), Some(true));
        assert!(cx.read(|cx| fixture.item.read(cx).is_dirty));
    }

    #[gpui::test]
    async fn test_schedule_resolved_task_with_completion_reports_success(cx: &mut TestAppContext) {
        let (fixture, cx) = create_fixture(cx, SaveStrategy::None).await;
        let task_result = Arc::new(Mutex::new(None));
        fixture.workspace.update_in(cx, |workspace, window, cx| {
            workspace.schedule_resolved_task_with_completion(
                TaskSourceKind::UserInput,
                fixture.task,
                false,
                {
                    let task_result = task_result.clone();
                    move |result, _| {
                        *task_result.lock() = Some(result);
                    }
                },
                window,
                cx,
            );
        });
        cx.executor().run_until_parked();

        assert_eq!(*task_result.lock(), Some(ScheduledTaskResult::Success));
    }

    #[gpui::test]
    async fn profile_artifact_opening_is_bounded_to_visible_project_files(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let settings_store = settings::SettingsStore::test(cx);
            cx.set_global(settings_store);
        });
        let fs = FakeFs::new(cx.executor());
        fs.insert_tree(
            "/root",
            json!({
                "target": {
                    "profile.svg": "<svg></svg>",
                    "oversized.svg": "0123456789abcdef",
                }
            }),
        )
        .await;
        let project = Project::test(fs, ["/root".as_ref()], cx).await;
        let artifact = TaskArtifact {
            path: "target/profile.svg".to_string(),
            kind: TaskArtifactKind::Svg,
            max_bytes: 1024,
        };
        let project_path = project
            .read_with(cx, |project, cx| {
                resolve_task_artifact(project, &artifact, Some("/root".as_ref()), cx)
            })
            .expect("visible bounded artifact should resolve");
        assert_eq!(project_path.path.as_unix_str(), "target/profile.svg");

        let missing = TaskArtifact {
            path: "target/missing.svg".to_string(),
            kind: TaskArtifactKind::Svg,
            max_bytes: 1024,
        };
        assert!(project.read_with(cx, |project, cx| {
            resolve_task_artifact(project, &missing, Some("/root".as_ref()), cx).is_err()
        }));
        let oversized = TaskArtifact {
            path: "target/oversized.svg".to_string(),
            max_bytes: 8,
            ..artifact
        };
        assert!(project.read_with(cx, |project, cx| {
            resolve_task_artifact(project, &oversized, Some("/root".as_ref()), cx).is_err()
        }));
    }

    #[test]
    fn cargo_pre_launch_task_reference_is_unique_and_uses_existing_task_resolution() {
        let context = TaskContext::default();
        let template = TaskTemplate {
            label: "Generate bindings".to_string(),
            command: "generator".to_string(),
            args: vec!["--checked".to_string()],
            ..TaskTemplate::default()
        };
        let (source, resolved) = resolve_task_reference(
            vec![(TaskSourceKind::UserInput, template.clone())],
            "Generate bindings",
            &context,
        )
        .expect("an exact existing Tasks label should resolve");
        assert_eq!(source, TaskSourceKind::UserInput);
        assert_eq!(resolved.resolved.command.as_deref(), Some("generator"));
        assert_eq!(resolved.resolved.args, vec!["--checked"]);

        assert!(
            resolve_task_reference(Vec::new(), "Generate bindings", &context)
                .expect_err("a missing reference must be isolated")
                .to_string()
                .contains("not found")
        );
        assert!(
            resolve_task_reference(
                vec![
                    (TaskSourceKind::UserInput, template.clone()),
                    (TaskSourceKind::UserInput, template),
                ],
                "Generate bindings",
                &context,
            )
            .expect_err("an ambiguous reference must not select arbitrarily")
            .to_string()
            .contains("ambiguous")
        );
    }

    async fn create_fixture(
        cx: &mut TestAppContext,
        save_strategy: SaveStrategy,
    ) -> (Fixture, &mut gpui::VisualTestContext) {
        cx.update(|cx| {
            let settings_store = settings::SettingsStore::test(cx);
            cx.set_global(settings_store);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            register_serializable_item::<TestItem>(cx);
        });
        let fs = FakeFs::new(cx.executor());
        fs.insert_tree("/root", json!({ "file.txt": "dirty" }))
            .await;
        let project = Project::test(fs.clone(), ["/root".as_ref()], cx).await;
        let (workspace, cx) =
            cx.add_window_view(|window, cx| Workspace::test_new(project.clone(), window, cx));

        // Add a dirty item to the workspace
        let item = add_test_item(&workspace, "file.txt", true, cx);

        let template = TaskTemplate {
            label: "test".to_string(),
            command: "echo".to_string(),
            save: save_strategy,
            ..Default::default()
        };
        let task = template
            .resolve_task("test", &task::TaskContext::default())
            .unwrap();
        let dirty_before_spawn: Arc<Mutex<Option<bool>>> = Arc::default();
        let terminal_provider = Box::new(TestTerminalProvider {
            item: item.clone(),
            dirty_before_spawn: dirty_before_spawn.clone(),
        });
        workspace.update(cx, |workspace, _| {
            workspace.terminal_provider = Some(terminal_provider);
        });
        let fixture = Fixture {
            workspace,
            item,
            task,
            dirty_before_spawn,
        };
        (fixture, cx)
    }

    fn add_test_item(
        workspace: &Entity<Workspace>,
        name: &str,
        active: bool,
        cx: &mut gpui::VisualTestContext,
    ) -> Entity<TestItem> {
        let item = cx.new(|cx| {
            TestItem::new(cx)
                .with_dirty(true)
                .with_project_items(&[TestProjectItem::new(1, name, cx)])
        });
        workspace.update_in(cx, |workspace, window, cx| {
            let pane = workspace.active_pane().clone();
            workspace.add_item(pane, Box::new(item.clone()), None, true, active, window, cx);
        });
        item
    }

    #[gpui::test]
    async fn test_save_for_task_all(cx: &mut TestAppContext) {
        let (fixture, cx) = create_fixture(cx, SaveStrategy::All).await;
        let workspace = fixture.workspace.downgrade();
        cx.run_until_parked();

        assert!(cx.read(|cx| fixture.item.read(cx).is_dirty));
        fixture.workspace.update_in(cx, |_workspace, window, cx| {
            cx.spawn_in(window, {
                let workspace = workspace.clone();
                async move |_this, cx| {
                    Workspace::save_for_task(&workspace, SaveStrategy::All, cx).await;
                }
            })
            .detach();
        });
        cx.run_until_parked();
        assert!(cx.read(|cx| !fixture.item.read(cx).is_dirty));
    }

    #[gpui::test]
    async fn test_save_for_task_none(cx: &mut TestAppContext) {
        let (fixture, cx) = create_fixture(cx, SaveStrategy::None).await;
        let workspace = fixture.workspace.downgrade();
        cx.run_until_parked();

        assert!(cx.read(|cx| fixture.item.read(cx).is_dirty));
        fixture.workspace.update_in(cx, |_workspace, window, cx| {
            cx.spawn_in(window, {
                let workspace = workspace.clone();
                async move |_this, cx| {
                    Workspace::save_for_task(&workspace, SaveStrategy::None, cx).await;
                }
            })
            .detach();
        });
        cx.run_until_parked();
        assert!(cx.read(|cx| fixture.item.read(cx).is_dirty));
    }

    #[gpui::test]
    async fn test_save_for_task_current(cx: &mut TestAppContext) {
        let (fixture, cx) = create_fixture(cx, SaveStrategy::Current).await;
        let inactive = add_test_item(&fixture.workspace, "file2.txt", false, cx);
        let workspace = fixture.workspace.downgrade();
        cx.run_until_parked();

        assert!(cx.read(|cx| fixture.item.read(cx).is_dirty));
        assert!(cx.read(|cx| inactive.read(cx).is_dirty));
        fixture.workspace.update_in(cx, |_workspace, window, cx| {
            cx.spawn_in(window, {
                let workspace = workspace.clone();
                async move |_this, cx| {
                    Workspace::save_for_task(&workspace, SaveStrategy::Current, cx).await;
                }
            })
            .detach();
        });
        cx.run_until_parked();
        assert!(cx.read(|cx| !fixture.item.read(cx).is_dirty));
        assert!(cx.read(|cx| inactive.read(cx).is_dirty));
    }

    struct TestTerminalProvider {
        item: Entity<TestItem>,
        dirty_before_spawn: Arc<Mutex<Option<bool>>>,
    }

    impl TerminalProvider for TestTerminalProvider {
        fn spawn(
            &self,
            _task: task::SpawnInTerminal,
            _window: &mut ui::Window,
            cx: &mut App,
        ) -> Task<Option<Result<ExitStatus>>> {
            *self.dirty_before_spawn.lock() = Some(cx.read_entity(&self.item, |e, _| e.is_dirty));
            Task::ready(Some(Ok(ExitStatus::default())))
        }
    }
}
