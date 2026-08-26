use anyhow::{Result, anyhow, bail};
use gpui::{Context, Window, actions};
use project::{
    ProjectPath, TaskSourceKind, WorktreeId,
    rust_coverage_provider::{MAX_RUST_COVERAGE_ARTIFACT_BYTES, RUST_COVERAGE_PROVIDER_ID},
    source_coverage::{SourceCoverageProviderId, SourceCoverageStatus},
};
use task::{DebugScenario, SharedTaskContext, TaskArtifactKind, TaskContext, TaskTemplate};
use workspace::{
    Toast, Workspace,
    notifications::NotificationId,
    tasks::{ScheduledTaskResult, resolve_task_artifact},
};

use crate::{
    CARGO_COVERAGE_FAILURE_GUIDANCE, CargoCompileContext, CargoPreset, CargoPresetScope,
    CargoSubcommand, CargoTargetSelector, compile_coverage_plan, compile_debug_scenario,
    compile_preset,
};

actions!(
    cargo_actions,
    [
        BuildSelected,
        CheckSelected,
        RunSelected,
        RunWithCoverageSelected,
        TestSelected,
        BenchSelected,
        DebugSelected,
        DocSelected,
        ClippySelected,
        FmtSelected,
        CleanSelected,
        TreeSelected
    ]
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoAction {
    Build,
    Check,
    Run,
    RunWithCoverage,
    Test,
    Bench,
    Debug,
    Doc,
    Clippy,
    Fmt,
    Clean,
    Tree,
}

impl CargoAction {
    pub const ALL: [Self; 12] = [
        Self::Build,
        Self::Check,
        Self::Run,
        Self::RunWithCoverage,
        Self::Test,
        Self::Bench,
        Self::Debug,
        Self::Doc,
        Self::Clippy,
        Self::Fmt,
        Self::Clean,
        Self::Tree,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Build => "Build",
            Self::Check => "Check",
            Self::Run => "Run",
            Self::RunWithCoverage => "Run with Coverage",
            Self::Test => "Test",
            Self::Bench => "Bench",
            Self::Debug => "Debug",
            Self::Doc => "Doc",
            Self::Clippy => "Clippy",
            Self::Fmt => "Fmt",
            Self::Clean => "Clean",
            Self::Tree => "Tree",
        }
    }

    fn subcommand(self, selection: &CargoActionSelection) -> Result<CargoSubcommand> {
        match self {
            Self::Build => Ok(CargoSubcommand::Build),
            Self::Check => Ok(CargoSubcommand::Check),
            Self::Run | Self::RunWithCoverage => Ok(CargoSubcommand::Run),
            Self::Test => Ok(CargoSubcommand::Test),
            Self::Bench => Ok(CargoSubcommand::Bench),
            Self::Doc => Ok(CargoSubcommand::Doc),
            Self::Clippy => Ok(CargoSubcommand::Clippy),
            Self::Fmt => Ok(CargoSubcommand::Fmt),
            Self::Clean => Ok(CargoSubcommand::Clean),
            Self::Tree => Ok(CargoSubcommand::Tree),
            Self::Debug => match selection.node_kind {
                CargoActionNodeKind::Target(CargoActionTargetKind::Binary)
                | CargoActionNodeKind::Target(CargoActionTargetKind::Example) => {
                    Ok(CargoSubcommand::Run)
                }
                CargoActionNodeKind::Target(CargoActionTargetKind::Test) => {
                    Ok(CargoSubcommand::Test)
                }
                CargoActionNodeKind::Target(CargoActionTargetKind::Bench) => {
                    Ok(CargoSubcommand::Bench)
                }
                _ => bail!("the selected Cargo node cannot be debugged"),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoActionTargetKind {
    Library,
    Binary,
    Example,
    Test,
    Bench,
    BuildScript,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoActionNodeKind {
    Workspace,
    Package,
    Target(CargoActionTargetKind),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoActionSelection {
    pub node_kind: CargoActionNodeKind,
    pub worktree_id: WorktreeId,
    pub workspace_name: String,
    pub workspace_manifest: ProjectPath,
    pub package_name: Option<String>,
    pub package_manifest: Option<ProjectPath>,
    pub target: Option<CargoTargetSelector>,
    pub has_bench_targets: bool,
}

impl CargoActionSelection {
    pub fn scope(&self) -> CargoPresetScope {
        match self.node_kind {
            CargoActionNodeKind::Workspace => CargoPresetScope::Workspace,
            CargoActionNodeKind::Package | CargoActionNodeKind::Target(_) => {
                CargoPresetScope::Package
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CargoActionRuntime {
    pub trusted: bool,
    pub connected: bool,
    pub writable: bool,
    pub cargo_available: bool,
    pub host_capable: bool,
}

impl Default for CargoActionRuntime {
    fn default() -> Self {
        Self {
            trusted: true,
            connected: true,
            writable: true,
            cargo_available: true,
            host_capable: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoActionAvailability {
    pub action: CargoAction,
    pub enabled: bool,
    pub reason: Option<String>,
    pub accessibility_label: String,
}

pub fn cargo_action_availability(
    selection: &CargoActionSelection,
    runtime: CargoActionRuntime,
) -> Vec<CargoActionAvailability> {
    CargoAction::ALL
        .into_iter()
        .map(|action| {
            let reason = action_inapplicable_reason(action, selection)
                .map(str::to_string)
                .or_else(|| runtime_denial_reason(runtime).map(str::to_string));
            let accessibility_label = reason
                .as_ref()
                .map(|reason| format!("{} unavailable: {reason}", action.label()))
                .unwrap_or_else(|| format!("{} selected Cargo item", action.label()));
            CargoActionAvailability {
                action,
                enabled: reason.is_none(),
                reason,
                accessibility_label,
            }
        })
        .collect()
}

fn runtime_denial_reason(runtime: CargoActionRuntime) -> Option<&'static str> {
    if !runtime.trusted {
        Some("trust this worktree before running Cargo commands")
    } else if !runtime.connected {
        Some("the authoritative project host is disconnected")
    } else if !runtime.writable {
        Some("multiplayer guests with read-only access cannot run Cargo commands")
    } else if !runtime.cargo_available {
        Some("Cargo is not available on the authoritative project host")
    } else if !runtime.host_capable {
        Some("the authoritative project host does not support Tasks and DAP")
    } else {
        None
    }
}

fn action_inapplicable_reason(
    action: CargoAction,
    selection: &CargoActionSelection,
) -> Option<&'static str> {
    use CargoActionNodeKind::{Package, Target, Workspace};
    use CargoActionTargetKind::{Bench, Binary, BuildScript, Example, Library, Other, Test};

    let applicable = match (selection.node_kind, action) {
        (
            Workspace | Package,
            CargoAction::Build
            | CargoAction::Check
            | CargoAction::Test
            | CargoAction::Doc
            | CargoAction::Clippy
            | CargoAction::Fmt
            | CargoAction::Clean
            | CargoAction::Tree,
        ) => true,
        (Workspace | Package, CargoAction::Bench) => selection.has_bench_targets,
        (
            Workspace | Package,
            CargoAction::Run | CargoAction::RunWithCoverage | CargoAction::Debug,
        ) => false,
        (
            Target(Library),
            CargoAction::Build
            | CargoAction::Check
            | CargoAction::Test
            | CargoAction::Doc
            | CargoAction::Clippy,
        ) => true,
        (
            Target(Binary | Example),
            CargoAction::Build | CargoAction::Check | CargoAction::Doc | CargoAction::Clippy,
        ) => true,
        (
            Target(Binary | Example),
            CargoAction::Run
            | CargoAction::RunWithCoverage
            | CargoAction::Test
            | CargoAction::Debug,
        ) => true,
        (
            Target(Test),
            CargoAction::Build | CargoAction::Check | CargoAction::Test | CargoAction::Clippy,
        ) => true,
        (Target(Test), CargoAction::Debug) => true,
        (
            Target(Bench),
            CargoAction::Build | CargoAction::Check | CargoAction::Bench | CargoAction::Clippy,
        ) => true,
        (Target(Bench), CargoAction::Debug) => true,
        (Target(BuildScript | Other), CargoAction::Build | CargoAction::Check) => true,
        _ => false,
    };
    (!applicable).then_some(match action {
        CargoAction::Run | CargoAction::RunWithCoverage => {
            "Run requires a binary or example target"
        }
        CargoAction::Bench => "Bench requires a bench target",
        CargoAction::Debug => "Debug requires a binary, example, test, or bench target",
        CargoAction::Test => "Test is not applicable to this target kind",
        CargoAction::Doc => "Doc requires a workspace, package, library, binary, or example",
        CargoAction::Clippy => "Clippy is not applicable to this target kind",
        CargoAction::Fmt | CargoAction::Clean | CargoAction::Tree => {
            "the action requires a workspace or package"
        }
        CargoAction::Build | CargoAction::Check => "the action is not applicable to this node",
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CargoActionPlan {
    Task {
        source: TaskSourceKind,
        template: TaskTemplate,
        context: TaskContext,
        worktree_id: WorktreeId,
        pre_launch_task: Option<String>,
        omit_history: bool,
    },
    Debug {
        scenario: DebugScenario,
        context: TaskContext,
        worktree_id: WorktreeId,
        pre_launch_task: Option<String>,
    },
}

impl CargoActionPlan {
    fn pre_launch(&self) -> Option<(&str, &TaskContext, WorktreeId)> {
        match self {
            Self::Task {
                context,
                worktree_id,
                pre_launch_task,
                ..
            }
            | Self::Debug {
                context,
                worktree_id,
                pre_launch_task,
                ..
            } => pre_launch_task
                .as_deref()
                .map(|reference| (reference, context, *worktree_id)),
        }
    }

    fn clear_pre_launch(&mut self) {
        match self {
            Self::Task {
                pre_launch_task, ..
            }
            | Self::Debug {
                pre_launch_task, ..
            } => *pre_launch_task = None,
        }
    }
}

pub fn plan_cargo_action(
    action: CargoAction,
    selection: &CargoActionSelection,
    active_preset: Option<&CargoPreset>,
    base_context: &TaskContext,
) -> Result<CargoActionPlan> {
    plan_cargo_action_with_confirmation(action, selection, active_preset, base_context, false)
}

pub fn plan_cargo_action_with_confirmation(
    action: CargoAction,
    selection: &CargoActionSelection,
    active_preset: Option<&CargoPreset>,
    base_context: &TaskContext,
    clean_confirmed: bool,
) -> Result<CargoActionPlan> {
    if let Some(reason) = action_inapplicable_reason(action, selection) {
        bail!("{} is unavailable: {reason}", action.label());
    }
    if action == CargoAction::Clean && !clean_confirmed {
        bail!("Clean requires explicit confirmation before scheduling");
    }
    let subcommand = action.subcommand(selection)?;
    let mut preset = active_preset
        .cloned()
        .unwrap_or_else(|| CargoPreset::ephemeral_default(subcommand));
    preset.scope = selection.scope();
    preset.package = selection.package_name.clone();
    preset.target = selection.target.clone();

    let workspace_cwd = manifest_directory_template(&selection.workspace_manifest);
    let package_cwd = selection
        .package_manifest
        .as_ref()
        .map(manifest_directory_template);
    let mut compiled = compile_preset(
        &preset,
        &CargoCompileContext {
            workspace_name: Some(selection.workspace_name.clone()),
            workspace_cwd: Some(workspace_cwd),
            package_name: selection.package_name.clone(),
            package_cwd,
        },
        Some(subcommand),
    )?;
    normalize_extended_action_args(action, &mut compiled.task_template.args);
    if action == CargoAction::RunWithCoverage {
        compiled = compile_coverage_plan(compiled)?;
    }
    let context = selection_task_context(base_context, &selection.workspace_manifest)?;
    if action == CargoAction::Debug {
        return Ok(CargoActionPlan::Debug {
            scenario: compile_debug_scenario(&compiled, None)?,
            context,
            worktree_id: selection.worktree_id,
            pre_launch_task: compiled.pre_launch_task,
        });
    }
    Ok(CargoActionPlan::Task {
        source: TaskSourceKind::Language {
            name: "Cargo".into(),
        },
        template: compiled.task_template,
        context,
        worktree_id: selection.worktree_id,
        pre_launch_task: compiled.pre_launch_task,
        omit_history: false,
    })
}

fn normalize_extended_action_args(action: CargoAction, args: &mut Vec<String>) {
    let subcommand_index = usize::from(
        args.first()
            .is_some_and(|argument| argument.starts_with('+')),
    );
    let scope_index = subcommand_index + 1;
    match action {
        CargoAction::Fmt
            if args
                .get(scope_index)
                .is_some_and(|argument| argument == "--workspace") =>
        {
            args[scope_index] = "--all".to_string();
        }
        CargoAction::Clean
            if args
                .get(scope_index)
                .is_some_and(|argument| argument == "--workspace") =>
        {
            args.remove(scope_index);
        }
        CargoAction::Tree => {
            let delimiter = args
                .iter()
                .position(|argument| argument == "--")
                .unwrap_or(args.len());
            args.splice(
                delimiter..delimiter,
                ["--locked".to_string(), "--offline".to_string()],
            );
        }
        _ => {}
    }
}

fn manifest_directory_template(manifest: &ProjectPath) -> String {
    let root = task::VariableName::WorktreeRoot.template_value();
    manifest
        .path
        .parent()
        .filter(|parent| !parent.is_empty())
        .map(|parent| format!("{root}/{}", parent.as_unix_str()))
        .unwrap_or(root)
}

fn selection_task_context(
    base_context: &TaskContext,
    workspace_manifest: &ProjectPath,
) -> Result<TaskContext> {
    let mut context = base_context.clone();
    let worktree_root = base_context
        .cwd
        .as_ref()
        .ok_or_else(|| anyhow!("the selected worktree has no task working directory"))?;
    let workspace_directory = workspace_manifest
        .path
        .parent()
        .map(|parent| worktree_root.join(parent.as_std_path()))
        .unwrap_or_else(|| worktree_root.clone());
    context.cwd = Some(workspace_directory);
    Ok(context)
}

pub trait CargoActionDispatcher {
    fn schedule_task(
        &mut self,
        source: TaskSourceKind,
        template: TaskTemplate,
        context: TaskContext,
        omit_history: bool,
    );

    fn start_debug_session(
        &mut self,
        scenario: DebugScenario,
        context: TaskContext,
        worktree_id: WorktreeId,
    );

    fn run_pre_launch_task(
        &mut self,
        reference: String,
        context: TaskContext,
        worktree_id: WorktreeId,
        continuation: CargoActionPlan,
    );
}

pub fn dispatch_cargo_action(
    mut plan: CargoActionPlan,
    dispatcher: &mut impl CargoActionDispatcher,
) {
    let pre_launch = plan.pre_launch().map(|(reference, context, worktree_id)| {
        (reference.to_string(), context.clone(), worktree_id)
    });
    if let Some((reference, context, worktree_id)) = pre_launch {
        plan.clear_pre_launch();
        dispatcher.run_pre_launch_task(reference, context, worktree_id, plan);
        return;
    }
    match plan {
        CargoActionPlan::Task {
            source,
            template,
            context,
            worktree_id: _,
            pre_launch_task: _,
            omit_history,
        } => dispatcher.schedule_task(source, template, context, omit_history),
        CargoActionPlan::Debug {
            scenario,
            context,
            worktree_id,
            pre_launch_task: _,
        } => dispatcher.start_debug_session(scenario, context, worktree_id),
    }
}

pub struct WorkspaceCargoActionDispatcher<'a, 'cx> {
    workspace: &'a mut Workspace,
    window: &'a mut Window,
    cx: &'a mut Context<'cx, Workspace>,
}

impl<'a, 'cx> WorkspaceCargoActionDispatcher<'a, 'cx> {
    pub fn new(
        workspace: &'a mut Workspace,
        window: &'a mut Window,
        cx: &'a mut Context<'cx, Workspace>,
    ) -> Self {
        Self {
            workspace,
            window,
            cx,
        }
    }
}

impl CargoActionDispatcher for WorkspaceCargoActionDispatcher<'_, '_> {
    fn schedule_task(
        &mut self,
        source: TaskSourceKind,
        template: TaskTemplate,
        context: TaskContext,
        omit_history: bool,
    ) {
        if template
            .artifact
            .as_ref()
            .is_some_and(|artifact| artifact.kind == TaskArtifactKind::Data)
        {
            self.schedule_coverage_task(source, template, context, omit_history);
            return;
        }
        self.workspace.schedule_task(
            source,
            &template,
            &context,
            omit_history,
            self.window,
            self.cx,
        );
    }

    fn start_debug_session(
        &mut self,
        scenario: DebugScenario,
        context: TaskContext,
        worktree_id: WorktreeId,
    ) {
        self.workspace.start_debug_session(
            scenario,
            SharedTaskContext::from(context),
            None,
            Some(worktree_id),
            self.window,
            self.cx,
        );
    }

    fn run_pre_launch_task(
        &mut self,
        reference: String,
        context: TaskContext,
        worktree_id: WorktreeId,
        continuation: CargoActionPlan,
    ) {
        let workspace = self.cx.entity().downgrade();
        self.workspace.schedule_task_reference_with_completion(
            reference,
            worktree_id,
            context,
            false,
            move |result, cx| {
                if result != ScheduledTaskResult::Success {
                    return;
                }
                if let Err(error) = workspace.update_in(cx, |workspace, window, cx| {
                    let mut dispatcher = WorkspaceCargoActionDispatcher::new(workspace, window, cx);
                    dispatch_cargo_action(continuation, &mut dispatcher);
                }) {
                    log::debug!("Cargo action was dropped after its pre-launch task: {error:#}");
                }
            },
            self.window,
            self.cx,
        );
    }
}

struct CargoCoverageNotification;

impl WorkspaceCargoActionDispatcher<'_, '_> {
    fn schedule_coverage_task(
        &mut self,
        source: TaskSourceKind,
        template: TaskTemplate,
        context: TaskContext,
        omit_history: bool,
    ) {
        let Some(resolved) = template.resolve_task(&source.to_id_base(), &context) else {
            self.workspace.show_toast(
                Toast::new(
                    NotificationId::unique::<CargoCoverageNotification>(),
                    "Run with Coverage could not resolve its task context",
                ),
                self.cx,
            );
            return;
        };
        let Some(artifact) = resolved.resolved_artifact().cloned() else {
            self.workspace.show_toast(
                Toast::new(
                    NotificationId::unique::<CargoCoverageNotification>(),
                    "Run with Coverage has no declared report artifact",
                ),
                self.cx,
            );
            return;
        };
        let task_working_directory = resolved.resolved.cwd.clone();
        let project = self.workspace.project().clone();
        let source_coverage_store = project.read(self.cx).source_coverage_store().clone();
        if let Err(error) = source_coverage_store.update(self.cx, |store, cx| {
            store.mark_provider_status(
                SourceCoverageProviderId(RUST_COVERAGE_PROVIDER_ID.to_string()),
                SourceCoverageStatus::Loading,
                None,
                cx,
            )
        }) {
            log::error!("Coverage loading state could not be published: {error:#}");
        }
        let workspace = self.cx.entity().downgrade();
        self.workspace.schedule_resolved_task_with_completion(
            source,
            resolved,
            omit_history,
            move |result, cx| match result {
                ScheduledTaskResult::Success => {
                    let interpretation: Result<_> = (|| {
                        let artifact_path = project.read_with(cx, |project, cx| {
                            resolve_task_artifact(
                                project,
                                &artifact,
                                task_working_directory.as_deref(),
                                cx,
                            )
                        })?;
                        let provider = project.read_with(cx, |project, _| {
                            project.rust_coverage_provider_store().clone()
                        });
                        let max_bytes = usize::try_from(artifact.max_bytes)
                            .unwrap_or(usize::MAX)
                            .min(MAX_RUST_COVERAGE_ARTIFACT_BYTES);
                        Ok(provider.update(cx, |provider, cx| {
                            provider.interpret_artifact(artifact_path, max_bytes, cx)
                        }))
                    })();
                    match interpretation {
                        Ok(interpretation) => {
                            cx.spawn(async move |cx| {
                                let message = match interpretation.await {
                                    Ok(snapshot) => format!(
                                        "Coverage loaded for {} file(s)",
                                        snapshot.files.len()
                                    ),
                                    Err(error) => {
                                        if let Err(status_error) = source_coverage_store.update(
                                            cx,
                                            |store, cx| {
                                                store.mark_provider_status(
                                                    SourceCoverageProviderId(
                                                        RUST_COVERAGE_PROVIDER_ID.to_string(),
                                                    ),
                                                    SourceCoverageStatus::Error,
                                                    Some(error.to_string()),
                                                    cx,
                                                )
                                            },
                                        ) {
                                            log::error!(
                                                "Coverage error state could not be published: {status_error:#}"
                                            );
                                        }
                                        format!("Coverage report could not be interpreted: {error}")
                                    }
                                };
                                workspace
                                    .update(cx, |workspace, cx| {
                                        workspace.show_toast(
                                            Toast::new(
                                                NotificationId::unique::<
                                                    CargoCoverageNotification,
                                                >(),
                                                message,
                                            ),
                                            cx,
                                        )
                                    })
                                    .ok();
                            })
                            .detach();
                        }
                        Err(error) => {
                            log::error!("Coverage artifact could not be scheduled: {error:#}");
                            if let Err(status_error) = source_coverage_store.update(
                                cx,
                                |store, cx| {
                                    store.mark_provider_status(
                                        SourceCoverageProviderId(
                                            RUST_COVERAGE_PROVIDER_ID.to_string(),
                                        ),
                                        SourceCoverageStatus::Error,
                                        Some("Coverage artifact validation failed".to_string()),
                                        cx,
                                    )
                                },
                            ) {
                                log::error!(
                                    "Coverage error state could not be published: {status_error:#}"
                                );
                            }
                            workspace
                                .update(cx, |workspace, cx| {
                                    workspace.show_toast(
                                        Toast::new(
                                            NotificationId::unique::<CargoCoverageNotification>(),
                                            "Coverage report could not be validated on the project host",
                                        ),
                                        cx,
                                    )
                                })
                                .ok();
                        }
                    }
                }
                ScheduledTaskResult::Failure | ScheduledTaskResult::SpawnFailed => {
                    if let Err(error) = source_coverage_store.update(cx, |store, cx| {
                        store.mark_provider_status(
                            SourceCoverageProviderId(RUST_COVERAGE_PROVIDER_ID.to_string()),
                            SourceCoverageStatus::Error,
                            Some(CARGO_COVERAGE_FAILURE_GUIDANCE.to_string()),
                            cx,
                        )
                    }) {
                        log::error!("Coverage failure state could not be published: {error:#}");
                    }
                    workspace
                        .update(cx, |workspace, cx| {
                            workspace.show_toast(
                                Toast::new(
                                    NotificationId::unique::<CargoCoverageNotification>(),
                                    CARGO_COVERAGE_FAILURE_GUIDANCE,
                                ),
                                cx,
                            )
                        })
                        .ok();
                }
                ScheduledTaskResult::Cancelled => {
                    if let Err(error) = source_coverage_store.update(cx, |store, cx| {
                        store.mark_provider_status(
                            SourceCoverageProviderId(RUST_COVERAGE_PROVIDER_ID.to_string()),
                            SourceCoverageStatus::Stale,
                            Some("Coverage run was cancelled".to_string()),
                            cx,
                        )
                    }) {
                        log::error!("Coverage cancellation state could not be published: {error:#}");
                    }
                }
            },
            self.window,
            self.cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use collections::HashMap;
    use project::WorktreeId;
    use task::{BuildTaskDefinition, RevealStrategy, RevealTarget, SaveStrategy};
    use util::rel_path::RelPath;

    use super::*;

    fn path(worktree_id: WorktreeId, path: &'static str) -> ProjectPath {
        ProjectPath {
            worktree_id,
            path: RelPath::from_unix_str(path)
                .expect("fixture path should be relative")
                .into(),
        }
    }

    fn selection(kind: CargoActionNodeKind) -> CargoActionSelection {
        let worktree_id = WorktreeId::from_proto(7);
        CargoActionSelection {
            node_kind: kind,
            worktree_id,
            workspace_name: "workspace".to_string(),
            workspace_manifest: path(worktree_id, "nested/Cargo.toml"),
            package_name: Some("member with spaces".to_string()),
            package_manifest: Some(path(worktree_id, "nested/member/Cargo.toml")),
            target: None,
            has_bench_targets: false,
        }
    }

    #[test]
    fn cargo_actions_eligibility_table_and_runtime_denials_are_accessible() {
        use CargoActionNodeKind::{Package, Target, Workspace};
        use CargoActionTargetKind::{Bench, Binary, BuildScript, Library, Test};

        let cases = [
            (
                Workspace,
                vec![
                    CargoAction::Build,
                    CargoAction::Check,
                    CargoAction::Test,
                    CargoAction::Doc,
                    CargoAction::Clippy,
                    CargoAction::Fmt,
                    CargoAction::Clean,
                    CargoAction::Tree,
                ],
            ),
            (
                Package,
                vec![
                    CargoAction::Build,
                    CargoAction::Check,
                    CargoAction::Test,
                    CargoAction::Doc,
                    CargoAction::Clippy,
                    CargoAction::Fmt,
                    CargoAction::Clean,
                    CargoAction::Tree,
                ],
            ),
            (
                Target(Library),
                vec![
                    CargoAction::Build,
                    CargoAction::Check,
                    CargoAction::Test,
                    CargoAction::Doc,
                    CargoAction::Clippy,
                ],
            ),
            (
                Target(Binary),
                vec![
                    CargoAction::Build,
                    CargoAction::Check,
                    CargoAction::Run,
                    CargoAction::RunWithCoverage,
                    CargoAction::Test,
                    CargoAction::Debug,
                    CargoAction::Doc,
                    CargoAction::Clippy,
                ],
            ),
            (
                Target(Test),
                vec![
                    CargoAction::Build,
                    CargoAction::Check,
                    CargoAction::Test,
                    CargoAction::Debug,
                    CargoAction::Clippy,
                ],
            ),
            (
                Target(Bench),
                vec![
                    CargoAction::Build,
                    CargoAction::Check,
                    CargoAction::Bench,
                    CargoAction::Debug,
                    CargoAction::Clippy,
                ],
            ),
            (
                Target(BuildScript),
                vec![CargoAction::Build, CargoAction::Check],
            ),
        ];
        for (kind, expected) in cases {
            let enabled = cargo_action_availability(&selection(kind), Default::default())
                .into_iter()
                .filter(|availability| availability.enabled)
                .map(|availability| availability.action)
                .collect::<Vec<_>>();
            assert_eq!(enabled, expected, "unexpected eligibility for {kind:?}");
        }

        let denial_cases = [
            (
                CargoActionRuntime {
                    trusted: false,
                    ..Default::default()
                },
                "trust this worktree",
            ),
            (
                CargoActionRuntime {
                    connected: false,
                    ..Default::default()
                },
                "disconnected",
            ),
            (
                CargoActionRuntime {
                    writable: false,
                    ..Default::default()
                },
                "read-only",
            ),
            (
                CargoActionRuntime {
                    cargo_available: false,
                    ..Default::default()
                },
                "Cargo is not available",
            ),
            (
                CargoActionRuntime {
                    host_capable: false,
                    ..Default::default()
                },
                "does not support Tasks and DAP",
            ),
        ];
        for (runtime, expected_reason) in denial_cases {
            let availability = cargo_action_availability(&selection(Target(Binary)), runtime);
            assert!(availability.iter().all(|entry| !entry.enabled));
            assert!(
                availability
                    .iter()
                    .find(|entry| entry.action == CargoAction::Build)
                    .is_some_and(|entry| entry.accessibility_label.contains(expected_reason))
            );
        }
    }

    #[derive(Default)]
    struct FakeDispatcher {
        plans: Vec<CargoActionPlan>,
        pre_launch_plans: Vec<(String, TaskContext, WorktreeId, CargoActionPlan)>,
    }

    impl CargoActionDispatcher for FakeDispatcher {
        fn schedule_task(
            &mut self,
            source: TaskSourceKind,
            template: TaskTemplate,
            context: TaskContext,
            omit_history: bool,
        ) {
            self.plans.push(CargoActionPlan::Task {
                source,
                template,
                context,
                worktree_id: WorktreeId::from_proto(0),
                pre_launch_task: None,
                omit_history,
            });
        }

        fn start_debug_session(
            &mut self,
            scenario: DebugScenario,
            context: TaskContext,
            worktree_id: WorktreeId,
        ) {
            self.plans.push(CargoActionPlan::Debug {
                scenario,
                context,
                worktree_id,
                pre_launch_task: None,
            });
        }

        fn run_pre_launch_task(
            &mut self,
            reference: String,
            context: TaskContext,
            worktree_id: WorktreeId,
            continuation: CargoActionPlan,
        ) {
            self.pre_launch_plans
                .push((reference, context, worktree_id, continuation));
        }
    }

    #[test]
    fn cargo_actions_route_exact_templates_and_debug_scenarios_to_fake_dispatcher() {
        let mut binary = selection(CargoActionNodeKind::Target(CargoActionTargetKind::Binary));
        binary.target = Some(CargoTargetSelector::Binary("bin;not-shell".to_string()));
        let mut preset = CargoPreset::ephemeral_default(CargoSubcommand::Run);
        preset.environment =
            HashMap::from_iter([("TOKEN".to_string(), "value with spaces".to_string())]);
        preset.args = vec!["--config".to_string(), "x='$(not-shell)'".to_string()];
        preset.presentation.save = SaveStrategy::All;
        preset.presentation.reveal = RevealStrategy::NoFocus;
        preset.presentation.reveal_target = RevealTarget::Center;
        let base_context = TaskContext {
            cwd: Some(PathBuf::from("/remote/worktree")),
            ..TaskContext::default()
        };

        let task_plan = plan_cargo_action(CargoAction::Run, &binary, Some(&preset), &base_context)
            .expect("binary run should compile");
        let mut dispatcher = FakeDispatcher::default();
        dispatch_cargo_action(task_plan, &mut dispatcher);
        let CargoActionPlan::Task {
            source,
            template,
            context,
            omit_history,
            ..
        } = &dispatcher.plans[0]
        else {
            panic!("run should route to the task scheduler")
        };
        assert!(matches!(source, TaskSourceKind::Language { name } if name == "Cargo"));
        assert_eq!(
            template.args,
            vec![
                "run",
                "--package",
                "member with spaces",
                "--bin",
                "bin;not-shell",
                "--config",
                "x='$(not-shell)'",
            ]
        );
        assert_eq!(template.env["TOKEN"], "value with spaces");
        assert_eq!(template.save, SaveStrategy::All);
        assert_eq!(template.reveal, RevealStrategy::NoFocus);
        assert_eq!(template.reveal_target, RevealTarget::Center);
        assert_eq!(
            context.cwd.as_deref(),
            Some(PathBuf::from("/remote/worktree/nested").as_path())
        );
        assert!(!omit_history);

        let debug_plan =
            plan_cargo_action(CargoAction::Debug, &binary, Some(&preset), &base_context)
                .expect("binary debug should compile");
        dispatch_cargo_action(debug_plan, &mut dispatcher);
        let CargoActionPlan::Debug {
            scenario,
            context,
            worktree_id,
            ..
        } = &dispatcher.plans[1]
        else {
            panic!("debug should route to DAP")
        };
        assert_eq!(*worktree_id, binary.worktree_id);
        assert_eq!(context.cwd, Some(PathBuf::from("/remote/worktree/nested")));
        let Some(BuildTaskDefinition::Template {
            task_template,
            locator_name,
        }) = scenario.build.as_ref()
        else {
            panic!("debug should contain a Cargo build task")
        };
        assert_eq!(
            task_template.args.first().map(String::as_str),
            Some("build")
        );
        assert_eq!(locator_name.as_deref(), Some("rust-cargo-locator"));
    }

    #[test]
    fn rust_workspace_comprehensive_fixture_compiles_the_selected_preset_without_tools() {
        let fixture: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../project/test_data/cargo_workspace/comprehensive-v1.json"
        ))
        .expect("comprehensive Cargo fixture should parse");
        let package = fixture["roots"][0]["metadata"]["packages"][0]["name"]
            .as_str()
            .expect("fixture package should have a name");
        let binary = fixture["roots"][0]["metadata"]["packages"][0]["targets"]
            .as_array()
            .and_then(|targets| {
                targets.iter().find(|target| {
                    target["kind"]
                        .as_array()
                        .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
                })
            })
            .and_then(|target| target["name"].as_str())
            .expect("fixture package should have a binary");
        let mut selection = selection(CargoActionNodeKind::Target(CargoActionTargetKind::Binary));
        selection.package_name = Some(package.to_string());
        selection.target = Some(CargoTargetSelector::Binary(binary.to_string()));
        let mut preset = CargoPreset::ephemeral_default(CargoSubcommand::Run);
        preset.toolchain = Some("stable".to_string());
        preset.features = vec!["extra".to_string()];
        let context = TaskContext {
            cwd: Some(PathBuf::from("/fixture")),
            ..TaskContext::default()
        };
        let CargoActionPlan::Task { template, .. } = plan_cargo_action(
            CargoAction::RunWithCoverage,
            &selection,
            Some(&preset),
            &context,
        )
        .expect("fixture selection should compile") else {
            panic!("coverage selection should compile to an ordinary task")
        };
        assert_eq!(
            template.args,
            [
                "+stable",
                "llvm-cov",
                "--json",
                "--output-path",
                crate::CARGO_COVERAGE_ARTIFACT_PATH,
                "run",
                "--package",
                package,
                "--bin",
                binary,
                "--features",
                "extra",
            ]
        );
        assert_eq!(
            template.artifact.as_ref().map(|artifact| artifact.kind),
            Some(TaskArtifactKind::Data)
        );
    }

    #[test]
    fn cargo_pre_launch_task_gates_the_main_action_without_embedding_a_command() {
        let binary = selection(CargoActionNodeKind::Target(CargoActionTargetKind::Binary));
        let mut preset = CargoPreset::ephemeral_default(CargoSubcommand::Run);
        preset.pre_launch_task = Some("Generate bindings".to_string());
        let base_context = TaskContext {
            cwd: Some(PathBuf::from("/remote/worktree")),
            ..TaskContext::default()
        };
        let plan = plan_cargo_action(CargoAction::Run, &binary, Some(&preset), &base_context)
            .expect("pre-launch preset should compile");
        let mut dispatcher = FakeDispatcher::default();
        dispatch_cargo_action(plan, &mut dispatcher);

        assert!(dispatcher.plans.is_empty());
        assert_eq!(dispatcher.pre_launch_plans.len(), 1);
        let (reference, context, worktree_id, continuation) = dispatcher
            .pre_launch_plans
            .pop()
            .expect("gate should be recorded");
        assert_eq!(reference, "Generate bindings");
        assert_eq!(context.cwd, Some(PathBuf::from("/remote/worktree/nested")));
        assert_eq!(worktree_id, binary.worktree_id);
        assert!(continuation.pre_launch().is_none());

        dispatch_cargo_action(continuation, &mut dispatcher);
        assert_eq!(dispatcher.plans.len(), 1);
    }

    fn task_args(plan: CargoActionPlan) -> Vec<String> {
        let CargoActionPlan::Task { template, .. } = plan else {
            panic!("extended Cargo actions should route through Tasks")
        };
        assert_eq!(template.command, "cargo");
        assert!(!template.allow_concurrent_runs);
        template.args
    }

    #[test]
    fn cargo_extended_action_matrix_uses_exact_safe_argv() {
        let workspace = selection(CargoActionNodeKind::Workspace);
        let package = selection(CargoActionNodeKind::Package);
        let binary = selection(CargoActionNodeKind::Target(CargoActionTargetKind::Binary));
        let base_context = TaskContext {
            cwd: Some(PathBuf::from("/remote/worktree")),
            ..TaskContext::default()
        };

        let cases = [
            (CargoAction::Doc, &workspace, vec!["doc", "--workspace"]),
            (
                CargoAction::Clippy,
                &package,
                vec!["clippy", "--package", "member with spaces"],
            ),
            (CargoAction::Fmt, &workspace, vec!["fmt", "--all"]),
            (
                CargoAction::Fmt,
                &package,
                vec!["fmt", "--package", "member with spaces"],
            ),
            (CargoAction::Clean, &workspace, vec!["clean"]),
            (
                CargoAction::Clean,
                &package,
                vec!["clean", "--package", "member with spaces"],
            ),
            (
                CargoAction::Tree,
                &workspace,
                vec!["tree", "--workspace", "--locked", "--offline"],
            ),
            (
                CargoAction::Tree,
                &package,
                vec![
                    "tree",
                    "--package",
                    "member with spaces",
                    "--locked",
                    "--offline",
                ],
            ),
        ];
        for (action, selection, expected) in cases {
            let plan = plan_cargo_action_with_confirmation(
                action,
                selection,
                None,
                &base_context,
                action == CargoAction::Clean,
            )
            .expect("approved Cargo action should plan");
            assert_eq!(task_args(plan), expected, "unexpected argv for {action:?}");
        }

        for action in [CargoAction::Fmt, CargoAction::Clean, CargoAction::Tree] {
            let availability = cargo_action_availability(&binary, Default::default())
                .into_iter()
                .find(|availability| availability.action == action)
                .expect("every action should have an availability row");
            assert!(!availability.enabled);
            assert!(
                availability
                    .accessibility_label
                    .contains("requires a workspace or package")
            );
        }

        let source = include_str!("cargo_actions.rs");
        assert!(!source.contains(concat!("CargoAction::", "Update")));
        assert!(!source.contains(concat!("Self::", "Update")));
    }

    #[test]
    fn cargo_clean_confirmation_is_required_before_a_plan_can_be_scheduled() {
        let workspace = selection(CargoActionNodeKind::Workspace);
        let base_context = TaskContext {
            cwd: Some(PathBuf::from("/remote/worktree")),
            ..TaskContext::default()
        };
        let error = plan_cargo_action(CargoAction::Clean, &workspace, None, &base_context)
            .expect_err("unconfirmed Clean must not produce a task plan");
        assert!(error.to_string().contains("explicit confirmation"));

        let plan = plan_cargo_action_with_confirmation(
            CargoAction::Clean,
            &workspace,
            None,
            &base_context,
            true,
        )
        .expect("confirmed Clean should produce a task plan");
        assert_eq!(task_args(plan), vec!["clean"]);
    }

    #[test]
    fn cargo_actions_have_no_direct_process_execution_path() {
        let source = include_str!("cargo_actions.rs");
        assert!(!source.contains(concat!("std::process::", "Command")));
        assert!(!source.contains(concat!("Command", "::new")));
        assert!(source.contains("schedule_task"));
        assert!(source.contains("start_debug_session"));
    }
}
