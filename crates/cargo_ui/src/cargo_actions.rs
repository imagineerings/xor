use anyhow::{Result, anyhow, bail};
use gpui::{Context, Window, actions};
use project::{ProjectPath, TaskSourceKind, WorktreeId};
use task::{DebugScenario, SharedTaskContext, TaskContext, TaskTemplate};
use workspace::Workspace;

use crate::{
    CargoCompileContext, CargoPreset, CargoPresetScope, CargoSubcommand, CargoTargetSelector,
    compile_debug_scenario, compile_preset,
};

actions!(
    cargo_actions,
    [
        BuildSelected,
        CheckSelected,
        RunSelected,
        TestSelected,
        BenchSelected,
        DebugSelected
    ]
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoAction {
    Build,
    Check,
    Run,
    Test,
    Bench,
    Debug,
}

impl CargoAction {
    pub const ALL: [Self; 6] = [
        Self::Build,
        Self::Check,
        Self::Run,
        Self::Test,
        Self::Bench,
        Self::Debug,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Build => "Build",
            Self::Check => "Check",
            Self::Run => "Run",
            Self::Test => "Test",
            Self::Bench => "Bench",
            Self::Debug => "Debug",
        }
    }

    fn subcommand(self, selection: &CargoActionSelection) -> Result<CargoSubcommand> {
        match self {
            Self::Build => Ok(CargoSubcommand::Build),
            Self::Check => Ok(CargoSubcommand::Check),
            Self::Run => Ok(CargoSubcommand::Run),
            Self::Test => Ok(CargoSubcommand::Test),
            Self::Bench => Ok(CargoSubcommand::Bench),
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
        (Workspace | Package, CargoAction::Build | CargoAction::Check | CargoAction::Test) => true,
        (Workspace | Package, CargoAction::Bench) => selection.has_bench_targets,
        (Workspace | Package, CargoAction::Run | CargoAction::Debug) => false,
        (Target(Library), CargoAction::Build | CargoAction::Check | CargoAction::Test) => true,
        (Target(Binary | Example), CargoAction::Build | CargoAction::Check) => true,
        (Target(Binary | Example), CargoAction::Run | CargoAction::Test | CargoAction::Debug) => {
            true
        }
        (Target(Test), CargoAction::Build | CargoAction::Check | CargoAction::Test) => true,
        (Target(Test), CargoAction::Debug) => true,
        (Target(Bench), CargoAction::Build | CargoAction::Check | CargoAction::Bench) => true,
        (Target(Bench), CargoAction::Debug) => true,
        (Target(BuildScript | Other), CargoAction::Build | CargoAction::Check) => true,
        _ => false,
    };
    (!applicable).then_some(match action {
        CargoAction::Run => "Run requires a binary or example target",
        CargoAction::Bench => "Bench requires a bench target",
        CargoAction::Debug => "Debug requires a binary, example, test, or bench target",
        CargoAction::Test => "Test is not applicable to this target kind",
        CargoAction::Build | CargoAction::Check => "the action is not applicable to this node",
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CargoActionPlan {
    Task {
        source: TaskSourceKind,
        template: TaskTemplate,
        context: TaskContext,
        omit_history: bool,
    },
    Debug {
        scenario: DebugScenario,
        context: TaskContext,
        worktree_id: WorktreeId,
    },
}

pub fn plan_cargo_action(
    action: CargoAction,
    selection: &CargoActionSelection,
    active_preset: Option<&CargoPreset>,
    base_context: &TaskContext,
) -> Result<CargoActionPlan> {
    if let Some(reason) = action_inapplicable_reason(action, selection) {
        bail!("{} is unavailable: {reason}", action.label());
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
    let compiled = compile_preset(
        &preset,
        &CargoCompileContext {
            workspace_name: Some(selection.workspace_name.clone()),
            workspace_cwd: Some(workspace_cwd),
            package_name: selection.package_name.clone(),
            package_cwd,
        },
        Some(subcommand),
    )?;
    let context = selection_task_context(base_context, &selection.workspace_manifest)?;
    if action == CargoAction::Debug {
        return Ok(CargoActionPlan::Debug {
            scenario: compile_debug_scenario(&compiled, None)?,
            context,
            worktree_id: selection.worktree_id,
        });
    }
    Ok(CargoActionPlan::Task {
        source: TaskSourceKind::Language {
            name: "Cargo".into(),
        },
        template: compiled.task_template,
        context,
        omit_history: false,
    })
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
}

pub fn dispatch_cargo_action(plan: CargoActionPlan, dispatcher: &mut impl CargoActionDispatcher) {
    match plan {
        CargoActionPlan::Task {
            source,
            template,
            context,
            omit_history,
        } => dispatcher.schedule_task(source, template, context, omit_history),
        CargoActionPlan::Debug {
            scenario,
            context,
            worktree_id,
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
            path: RelPath::unix(path)
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
                vec![CargoAction::Build, CargoAction::Check, CargoAction::Test],
            ),
            (
                Package,
                vec![CargoAction::Build, CargoAction::Check, CargoAction::Test],
            ),
            (
                Target(Library),
                vec![CargoAction::Build, CargoAction::Check, CargoAction::Test],
            ),
            (
                Target(Binary),
                vec![
                    CargoAction::Build,
                    CargoAction::Check,
                    CargoAction::Run,
                    CargoAction::Test,
                    CargoAction::Debug,
                ],
            ),
            (
                Target(Test),
                vec![
                    CargoAction::Build,
                    CargoAction::Check,
                    CargoAction::Test,
                    CargoAction::Debug,
                ],
            ),
            (
                Target(Bench),
                vec![
                    CargoAction::Build,
                    CargoAction::Check,
                    CargoAction::Bench,
                    CargoAction::Debug,
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
            });
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
    fn cargo_actions_have_no_direct_process_execution_path() {
        let source = include_str!("cargo_actions.rs");
        assert!(!source.contains(concat!("std::process::", "Command")));
        assert!(!source.contains(concat!("Command", "::new")));
        assert!(source.contains("schedule_task"));
        assert!(source.contains("start_debug_session"));
    }
}
