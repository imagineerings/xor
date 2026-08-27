//! Baseline interface of Tasks in Zed: all tasks in Zed are intended to use those for implementing their own logic.

mod adapter_schema;
mod debug_format;
mod serde_helpers;
pub mod static_source;
mod task_template;
mod vscode_debug_format;
mod vscode_format;

use anyhow::Context as _;
use collections::{HashMap, HashSet, hash_map};
use gpui::{App, SharedString, Window};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::str::FromStr;
use std::sync::Arc;

pub use adapter_schema::{AdapterSchema, AdapterSchemas};
pub use debug_format::{
    AttachRequest, BuildTaskDefinition, DebugRequest, DebugScenario, DebugTaskFile, LaunchRequest,
    Request, TcpArgumentsTemplate, ZedDebugConfig,
};
pub use task_template::{
    DebugArgsRequest, HideStrategy, RevealStrategy, SaveStrategy, TaskHook, TaskTemplate,
    TaskTemplates, substitute_variables_in_map, substitute_variables_in_str,
};
pub use util::shell::{Shell, ShellKind};
pub use util::shell_builder::ShellBuilder;
pub use vscode_debug_format::VsCodeDebugTaskFile;
pub use vscode_format::VsCodeTaskFile;
pub use zed_actions::RevealTarget;

pub const MAX_TASK_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_TASK_ARTIFACT_PATH_BYTES: usize = 1024;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskArtifactKind {
    Svg,
    Html,
    /// A machine-readable artifact consumed by a feature after task completion.
    Data,
    #[default]
    File,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskArtifact {
    pub path: String,
    #[serde(default)]
    pub kind: TaskArtifactKind,
    #[serde(default = "default_task_artifact_max_bytes")]
    pub max_bytes: u64,
}

fn default_task_artifact_max_bytes() -> u64 {
    MAX_TASK_ARTIFACT_BYTES
}

impl TaskArtifact {
    pub fn with_resolved_path(&self, path: String) -> anyhow::Result<Self> {
        use std::path::Component;

        if path.is_empty() || path.len() > MAX_TASK_ARTIFACT_PATH_BYTES {
            anyhow::bail!("task artifact path is empty or exceeds the supported length");
        }
        if path.contains("://")
            || path.starts_with('/')
            || path.starts_with('\\')
            || path.as_bytes().get(1) == Some(&b':')
            || PathBuf::from(&path).components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            anyhow::bail!("task artifact must be a project-relative path");
        }
        if self.max_bytes == 0 || self.max_bytes > MAX_TASK_ARTIFACT_BYTES {
            anyhow::bail!(
                "task artifact byte limit must be between 1 and {MAX_TASK_ARTIFACT_BYTES}"
            );
        }
        let extension = PathBuf::from(&path)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        match self.kind {
            TaskArtifactKind::Svg if extension.as_deref() != Some("svg") => {
                anyhow::bail!("SVG task artifacts must use the .svg extension")
            }
            TaskArtifactKind::Html if !matches!(extension.as_deref(), Some("html" | "htm")) => {
                anyhow::bail!("HTML task artifacts must use the .html or .htm extension")
            }
            TaskArtifactKind::Data if extension.as_deref() != Some("json") => {
                anyhow::bail!("data task artifacts must use the .json extension")
            }
            TaskArtifactKind::Svg
            | TaskArtifactKind::Html
            | TaskArtifactKind::Data
            | TaskArtifactKind::File => {}
        }
        Ok(Self {
            path,
            kind: self.kind,
            max_bytes: self.max_bytes,
        })
    }
}

/// Task identifier, unique within the application.
/// Based on it, task reruns and terminal tabs are managed.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructuredTerminalId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuredTaskState {
    Queued,
    Running {
        terminal_id: Option<StructuredTerminalId>,
    },
    Completed {
        terminal_id: Option<StructuredTerminalId>,
        exit_code: Option<i32>,
        success: bool,
    },
    SpawnError {
        message: String,
    },
    Cancelled {
        terminal_id: Option<StructuredTerminalId>,
        termination_confirmed: bool,
    },
}

impl StructuredTaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::SpawnError { .. } | Self::Cancelled { .. }
        )
    }

    pub fn terminal_id(&self) -> Option<StructuredTerminalId> {
        match self {
            Self::Running { terminal_id }
            | Self::Completed { terminal_id, .. }
            | Self::Cancelled { terminal_id, .. } => *terminal_id,
            Self::Queued | Self::SpawnError { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredTaskLifecycleEvent {
    pub task_id: TaskId,
    pub state: StructuredTaskState,
}

type StructuredTaskSubscriber = Arc<dyn Fn(&StructuredTaskLifecycleEvent, &mut App) + Send + Sync>;
type StructuredTaskCancel = Arc<dyn Fn(&mut App) -> bool + Send + Sync>;
type StructuredTaskReveal = Arc<dyn Fn(&mut Window, &mut App) -> bool + Send + Sync>;

struct StructuredTaskHandleInner {
    task_id: TaskId,
    state: StructuredTaskState,
    subscribers: Vec<StructuredTaskSubscriber>,
    cancel: Option<StructuredTaskCancel>,
    reveal: Option<StructuredTaskReveal>,
}

#[derive(Clone)]
pub struct StructuredTaskHandle {
    inner: Arc<parking_lot::Mutex<StructuredTaskHandleInner>>,
}

impl std::fmt::Debug for StructuredTaskHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StructuredTaskHandle")
            .field("task_id", &self.task_id())
            .field("state", &self.state())
            .finish_non_exhaustive()
    }
}

impl StructuredTaskHandle {
    pub fn new(task_id: TaskId) -> Self {
        Self {
            inner: Arc::new(parking_lot::Mutex::new(StructuredTaskHandleInner {
                task_id,
                state: StructuredTaskState::Queued,
                subscribers: Vec::new(),
                cancel: None,
                reveal: None,
            })),
        }
    }

    pub fn task_id(&self) -> TaskId {
        self.inner.lock().task_id.clone()
    }

    pub fn state(&self) -> StructuredTaskState {
        self.inner.lock().state.clone()
    }

    pub fn subscribe(
        &self,
        cx: &mut App,
        subscriber: impl Fn(&StructuredTaskLifecycleEvent, &mut App) + Send + Sync + 'static,
    ) {
        let subscriber: StructuredTaskSubscriber = Arc::new(subscriber);
        let event = self.event();
        subscriber(&event, cx);
        self.inner.lock().subscribers.push(subscriber);
    }

    pub fn mark_running(&self, terminal_id: Option<StructuredTerminalId>, cx: &mut App) -> bool {
        self.transition(StructuredTaskState::Running { terminal_id }, cx)
    }

    pub fn bind_terminal(
        &self,
        terminal_id: StructuredTerminalId,
        cancel: impl Fn(&mut App) -> bool + Send + Sync + 'static,
        cx: &mut App,
    ) -> bool {
        let cancel: StructuredTaskCancel = Arc::new(cancel);
        let was_cancelled = {
            let mut inner = self.inner.lock();
            inner.cancel = Some(cancel.clone());
            matches!(inner.state, StructuredTaskState::Cancelled { .. })
        };
        if was_cancelled {
            cancel(cx);
            self.mark_cancelled(Some(terminal_id), false, cx);
            false
        } else {
            self.mark_running(Some(terminal_id), cx)
        }
    }

    pub fn set_reveal(
        &self,
        reveal: impl Fn(&mut Window, &mut App) -> bool + Send + Sync + 'static,
    ) {
        self.inner.lock().reveal = Some(Arc::new(reveal));
    }

    pub fn reveal_terminal(&self, window: &mut Window, cx: &mut App) -> bool {
        let reveal = self.inner.lock().reveal.clone();
        reveal.is_some_and(|reveal| reveal(window, cx))
    }

    pub fn cancel(&self, cx: &mut App) -> bool {
        let (state, cancel) = {
            let inner = self.inner.lock();
            (inner.state.clone(), inner.cancel.clone())
        };
        if state.is_terminal() {
            return false;
        }
        let terminal_id = state.terminal_id();
        let termination_confirmed = if let Some(cancel) = cancel {
            cancel(cx);
            false
        } else {
            matches!(state, StructuredTaskState::Queued)
        };
        self.mark_cancelled(terminal_id, termination_confirmed, cx)
    }

    pub fn mark_completed(&self, exit_status: ExitStatus, cx: &mut App) -> bool {
        self.transition(
            StructuredTaskState::Completed {
                terminal_id: self.state().terminal_id(),
                exit_code: exit_status.code(),
                success: exit_status.success(),
            },
            cx,
        )
    }

    pub fn mark_spawn_error(&self, message: impl Into<String>, cx: &mut App) -> bool {
        const MAX_ERROR_BYTES: usize = 4 * 1024;
        let mut message = message.into();
        if message.len() > MAX_ERROR_BYTES {
            let mut end = MAX_ERROR_BYTES;
            while end > 0 && !message.is_char_boundary(end) {
                end -= 1;
            }
            message.truncate(end);
        }
        self.transition(StructuredTaskState::SpawnError { message }, cx)
    }

    pub fn mark_cancelled(
        &self,
        terminal_id: Option<StructuredTerminalId>,
        termination_confirmed: bool,
        cx: &mut App,
    ) -> bool {
        self.transition(
            StructuredTaskState::Cancelled {
                terminal_id,
                termination_confirmed,
            },
            cx,
        )
    }

    fn event(&self) -> StructuredTaskLifecycleEvent {
        let inner = self.inner.lock();
        StructuredTaskLifecycleEvent {
            task_id: inner.task_id.clone(),
            state: inner.state.clone(),
        }
    }

    fn transition(&self, state: StructuredTaskState, cx: &mut App) -> bool {
        let subscribers = {
            let mut inner = self.inner.lock();
            if inner.state == state {
                return false;
            }
            let cancellation_update = matches!(
                (&inner.state, &state),
                (
                    StructuredTaskState::Cancelled {
                        terminal_id: old_terminal_id,
                        termination_confirmed: old_confirmed,
                    },
                    StructuredTaskState::Cancelled {
                        terminal_id: new_terminal_id,
                        termination_confirmed: new_confirmed,
                    },
                ) if (!old_confirmed && *new_confirmed)
                    || (old_terminal_id.is_none() && new_terminal_id.is_some())
            );
            if inner.state.is_terminal() && !cancellation_update {
                return false;
            }
            let valid = matches!(
                (&inner.state, &state),
                (
                    StructuredTaskState::Queued,
                    StructuredTaskState::Running { .. }
                        | StructuredTaskState::SpawnError { .. }
                        | StructuredTaskState::Cancelled { .. }
                ) | (
                    StructuredTaskState::Running { .. },
                    StructuredTaskState::Running { .. }
                        | StructuredTaskState::Completed { .. }
                        | StructuredTaskState::SpawnError { .. }
                        | StructuredTaskState::Cancelled { .. }
                )
            ) || cancellation_update;
            if !valid {
                return false;
            }
            inner.state = state;
            inner.subscribers.clone()
        };
        let event = self.event();
        for subscriber in subscribers {
            subscriber(&event, cx);
        }
        true
    }
}

/// Contains all information needed by Zed to spawn a new terminal tab for the given task.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SpawnInTerminal {
    /// Id of the task to use when determining task tab affinity.
    pub id: TaskId,
    /// Full unshortened form of `label` field.
    pub full_label: String,
    /// Human readable name of the terminal tab.
    pub label: String,
    /// Executable command to spawn.
    pub command: Option<String>,
    /// Arguments to the command, potentially unsubstituted,
    /// to let the shell that spawns the command to do the substitution, if needed.
    pub args: Vec<String>,
    /// A human-readable label, containing command and all of its arguments, joined and substituted.
    pub command_label: String,
    /// Current working directory to spawn the command into.
    pub cwd: Option<PathBuf>,
    /// Env overrides for the command, will be appended to the terminal's environment from the settings.
    pub env: HashMap<String, String>,
    /// Whether to use a new terminal tab or reuse the existing one to spawn the process.
    pub use_new_terminal: bool,
    /// Whether to allow multiple instances of the same task to be run, or rather wait for the existing ones to finish.
    pub allow_concurrent_runs: bool,
    /// What to do with the terminal pane and tab, after the command was started.
    pub reveal: RevealStrategy,
    /// Where to show tasks' terminal output.
    pub reveal_target: RevealTarget,
    /// What to do with the terminal pane and tab, after the command had finished.
    pub hide: HideStrategy,
    /// Which shell to use when spawning the task.
    pub shell: Shell,
    /// Whether to show the task summary line in the task output (success/failure).
    pub show_summary: bool,
    /// Whether to show the command line in the task output.
    pub show_command: bool,
    /// Whether to show the rerun button in the terminal tab.
    pub show_rerun: bool,
    /// Which edited buffers to save before running the task.
    pub save: SaveStrategy,
}

impl SpawnInTerminal {
    pub fn to_proto(&self) -> proto::SpawnInTerminal {
        proto::SpawnInTerminal {
            label: self.label.clone(),
            command: self.command.clone(),
            args: self.args.clone(),
            env: self
                .env
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            cwd: self
                .cwd
                .clone()
                .map(|cwd| cwd.to_string_lossy().into_owned()),
        }
    }

    pub fn from_proto(proto: proto::SpawnInTerminal) -> Self {
        Self {
            label: proto.label.clone(),
            command: proto.command.clone(),
            args: proto.args.clone(),
            env: proto.env.into_iter().collect(),
            cwd: proto.cwd.map(PathBuf::from),
            ..Default::default()
        }
    }
}

/// A final form of the [`TaskTemplate`], that got resolved with a particular [`TaskContext`] and now is ready to spawn the actual task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTask {
    /// A way to distinguish tasks produced by the same template, but different contexts.
    /// NOTE: Resolved tasks may have the same labels, commands and do the same things,
    /// but still may have different ids if the context was different during the resolution.
    /// Since the template has `env` field, for a generic task that may be a bash command,
    /// so it's impossible to determine the id equality without more context in a generic case.
    pub id: TaskId,
    /// A template the task got resolved from.
    original_task: TaskTemplate,
    resolved_artifact: Option<TaskArtifact>,
    /// Full, unshortened label of the task after all resolutions are made.
    pub resolved_label: String,
    /// Variables that were substituted during the task template resolution.
    substituted_variables: HashSet<VariableName>,
    /// Further actions that need to take place after the resolved task is spawned,
    /// with all task variables resolved.
    pub resolved: SpawnInTerminal,
}

impl ResolvedTask {
    /// A task template before the resolution.
    pub fn original_task(&self) -> &TaskTemplate {
        &self.original_task
    }

    /// Variables that were substituted during the task template resolution.
    pub fn substituted_variables(&self) -> &HashSet<VariableName> {
        &self.substituted_variables
    }

    pub fn resolved_artifact(&self) -> Option<&TaskArtifact> {
        self.resolved_artifact.as_ref()
    }

    /// A human-readable label to display in the UI.
    pub fn display_label(&self) -> &str {
        self.resolved.label.as_str()
    }
}

/// Variables, available for use in [`TaskContext`] when a Zed's [`TaskTemplate`] gets resolved into a [`ResolvedTask`].
/// Name of the variable must be a valid shell variable identifier, which generally means that it is
/// a word  consisting only  of alphanumeric characters and underscores,
/// and beginning with an alphabetic character or an  underscore.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum VariableName {
    /// An absolute path of the currently opened file.
    File,
    /// A path of the currently opened file (relative to worktree root).
    RelativeFile,
    /// A path of the currently opened file's directory (relative to worktree root).
    RelativeDir,
    /// The currently opened filename.
    Filename,
    /// The path to a parent directory of a currently opened file.
    Dirname,
    /// Stem (filename without extension) of the currently opened file.
    Stem,
    /// An absolute path of the currently opened worktree, that contains the file.
    WorktreeRoot,
    /// A symbol text, that contains latest cursor/selection position.
    Symbol,
    /// A row with the latest cursor/selection position.
    Row,
    /// A column with the latest cursor/selection position.
    Column,
    /// Text from the latest selection.
    SelectedText,
    /// The language of the currently opened buffer (e.g., "Rust", "Python").
    Language,
    /// The symbol selected by the symbol tagging system, specifically the @run capture in a runnables.scm
    RunnableSymbol,
    /// Open a Picker to select a process ID to use in place
    /// Can only be used to debug configurations
    PickProcessId,
    /// An absolute path of the main (original) git worktree for the current repository.
    /// For normal checkouts, this equals the worktree root. For linked worktrees,
    /// this is the original repo's working directory.
    MainGitWorktree,
    /// Full SHA for the Git commit associated with the task context.
    GitSha,
    /// Short SHA for the Git commit associated with the task context.
    GitShaShort,
    /// Name of the Git repository associated with the task context.
    GitRepositoryName,
    /// Absolute path of the Git repository associated with the task context.
    GitRepositoryPath,
    /// Name of the Git ref (branch, remote ref, or tag) associated with the task context.
    GitRef,
    /// Custom variable, provided by the plugin or other external source.
    /// Will be printed with `CUSTOM_` prefix to avoid potential conflicts with other variables.
    Custom(Cow<'static, str>),
}

impl VariableName {
    /// Generates a `$VARIABLE`-like string value to be used in templates.
    pub fn template_value(&self) -> String {
        format!("${self}")
    }
    /// Generates a `"$VARIABLE"`-like string, to be used instead of `Self::template_value` when expanded value could contain spaces or special characters.
    pub fn template_value_with_whitespace(&self) -> String {
        format!("\"${self}\"")
    }
}

impl FromStr for VariableName {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let without_prefix = s.strip_prefix(ZED_VARIABLE_NAME_PREFIX).ok_or(())?;
        let value = match without_prefix {
            "FILE" => Self::File,
            "FILENAME" => Self::Filename,
            "RELATIVE_FILE" => Self::RelativeFile,
            "RELATIVE_DIR" => Self::RelativeDir,
            "DIRNAME" => Self::Dirname,
            "STEM" => Self::Stem,
            "WORKTREE_ROOT" => Self::WorktreeRoot,
            "SYMBOL" => Self::Symbol,
            "RUNNABLE_SYMBOL" => Self::RunnableSymbol,
            "SELECTED_TEXT" => Self::SelectedText,
            "LANGUAGE" => Self::Language,
            "ROW" => Self::Row,
            "COLUMN" => Self::Column,
            "MAIN_GIT_WORKTREE" => Self::MainGitWorktree,
            "GIT_SHA" => Self::GitSha,
            "GIT_SHA_SHORT" => Self::GitShaShort,
            "GIT_REPOSITORY_NAME" => Self::GitRepositoryName,
            "GIT_REPOSITORY_PATH" => Self::GitRepositoryPath,
            "GIT_REF" => Self::GitRef,
            _ => {
                if let Some(custom_name) =
                    without_prefix.strip_prefix(ZED_CUSTOM_VARIABLE_NAME_PREFIX)
                {
                    Self::Custom(Cow::Owned(custom_name.to_owned()))
                } else {
                    return Err(());
                }
            }
        };
        Ok(value)
    }
}

/// A prefix that all [`VariableName`] variants are prefixed with when used in environment variables and similar template contexts.
pub const ZED_VARIABLE_NAME_PREFIX: &str = "ZED_";
const ZED_CUSTOM_VARIABLE_NAME_PREFIX: &str = "CUSTOM_";

impl std::fmt::Display for VariableName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "{ZED_VARIABLE_NAME_PREFIX}FILE"),
            Self::Filename => write!(f, "{ZED_VARIABLE_NAME_PREFIX}FILENAME"),
            Self::RelativeFile => write!(f, "{ZED_VARIABLE_NAME_PREFIX}RELATIVE_FILE"),
            Self::RelativeDir => write!(f, "{ZED_VARIABLE_NAME_PREFIX}RELATIVE_DIR"),
            Self::Dirname => write!(f, "{ZED_VARIABLE_NAME_PREFIX}DIRNAME"),
            Self::Stem => write!(f, "{ZED_VARIABLE_NAME_PREFIX}STEM"),
            Self::WorktreeRoot => write!(f, "{ZED_VARIABLE_NAME_PREFIX}WORKTREE_ROOT"),
            Self::Symbol => write!(f, "{ZED_VARIABLE_NAME_PREFIX}SYMBOL"),
            Self::Row => write!(f, "{ZED_VARIABLE_NAME_PREFIX}ROW"),
            Self::Column => write!(f, "{ZED_VARIABLE_NAME_PREFIX}COLUMN"),
            Self::SelectedText => write!(f, "{ZED_VARIABLE_NAME_PREFIX}SELECTED_TEXT"),
            Self::Language => write!(f, "{ZED_VARIABLE_NAME_PREFIX}LANGUAGE"),
            Self::RunnableSymbol => write!(f, "{ZED_VARIABLE_NAME_PREFIX}RUNNABLE_SYMBOL"),
            Self::PickProcessId => write!(f, "{ZED_VARIABLE_NAME_PREFIX}PICK_PID"),
            Self::MainGitWorktree => write!(f, "{ZED_VARIABLE_NAME_PREFIX}MAIN_GIT_WORKTREE"),
            Self::GitSha => write!(f, "{ZED_VARIABLE_NAME_PREFIX}GIT_SHA"),
            Self::GitShaShort => write!(f, "{ZED_VARIABLE_NAME_PREFIX}GIT_SHA_SHORT"),
            Self::GitRepositoryName => write!(f, "{ZED_VARIABLE_NAME_PREFIX}GIT_REPOSITORY_NAME"),
            Self::GitRepositoryPath => write!(f, "{ZED_VARIABLE_NAME_PREFIX}GIT_REPOSITORY_PATH"),
            Self::GitRef => write!(f, "{ZED_VARIABLE_NAME_PREFIX}GIT_REF"),
            Self::Custom(s) => write!(
                f,
                "{ZED_VARIABLE_NAME_PREFIX}{ZED_CUSTOM_VARIABLE_NAME_PREFIX}{s}"
            ),
        }
    }
}

/// Container for predefined environment variables that describe state of Zed at the time the task was spawned.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TaskVariables(HashMap<VariableName, String>);

impl TaskVariables {
    /// Inserts another variable into the container, overwriting the existing one if it already exists — in this case, the old value is returned.
    pub fn insert(&mut self, variable: VariableName, value: String) -> Option<String> {
        self.0.insert(variable, value)
    }

    /// Extends the container with another one, overwriting the existing variables on collision.
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }
    /// Get the value associated with given variable name, if there is one.
    pub fn get(&self, key: &VariableName) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }
    /// Clear out variables obtained from tree-sitter queries, which are prefixed with '_' character
    pub fn sweep(&mut self) {
        self.0.retain(|name, _| {
            if let VariableName::Custom(name) = name {
                !name.starts_with('_')
            } else {
                true
            }
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&VariableName, &String)> {
        self.0.iter()
    }
}

impl FromIterator<(VariableName, String)> for TaskVariables {
    fn from_iter<T: IntoIterator<Item = (VariableName, String)>>(iter: T) -> Self {
        Self(HashMap::from_iter(iter))
    }
}

impl IntoIterator for TaskVariables {
    type Item = (VariableName, String);

    type IntoIter = hash_map::IntoIter<VariableName, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Keeps track of the file associated with a task and context of tasks execution (i.e. current file or current function).
/// Keeps all Zed-related state inside, used to produce a resolved task out of its template.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskContext {
    /// A path to a directory in which the task should be executed.
    pub cwd: Option<PathBuf>,
    /// Additional environment variables associated with a given task.
    pub task_variables: TaskVariables,
    /// Environment variables obtained when loading the project into Zed.
    /// This is the environment one would get when `cd`ing in a terminal
    /// into the project's root directory.
    pub project_env: HashMap<String, String>,
}

/// A shared reference to a [`TaskContext`], used to avoid cloning the context multiple times.
#[derive(Clone, Debug, Default)]
pub struct SharedTaskContext(Arc<TaskContext>);

impl std::ops::Deref for SharedTaskContext {
    type Target = TaskContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TaskContext> for SharedTaskContext {
    fn from(context: TaskContext) -> Self {
        Self(Arc::new(context))
    }
}

/// This is a new type representing a 'tag' on a 'runnable symbol', typically a test of main() function, found via treesitter.
#[derive(Clone, Debug)]
pub struct RunnableTag(pub SharedString);

pub fn shell_from_proto(proto: proto::Shell) -> anyhow::Result<Shell> {
    let shell_type = proto.shell_type.context("invalid shell type")?;
    let shell = match shell_type {
        proto::shell::ShellType::System(_) => Shell::System,
        proto::shell::ShellType::Program(program) => Shell::Program(program),
        proto::shell::ShellType::WithArguments(program) => Shell::WithArguments {
            program: program.program,
            args: program.args,
            title_override: None,
        },
    };
    Ok(shell)
}

pub fn shell_to_proto(shell: Shell) -> proto::Shell {
    let shell_type = match shell {
        Shell::System => proto::shell::ShellType::System(proto::System {}),
        Shell::Program(program) => proto::shell::ShellType::Program(program),
        Shell::WithArguments {
            program,
            args,
            title_override: _,
        } => proto::shell::ShellType::WithArguments(proto::shell::WithArguments { program, args }),
    };
    proto::Shell {
        shell_type: Some(shell_type),
    }
}

type VsCodeEnvVariable = String;
type VsCodeCommand = String;
type ZedEnvVariable = String;

struct EnvVariableReplacer {
    variables: HashMap<VsCodeEnvVariable, ZedEnvVariable>,
    commands: HashMap<VsCodeCommand, ZedEnvVariable>,
}

impl EnvVariableReplacer {
    fn new(variables: HashMap<VsCodeEnvVariable, ZedEnvVariable>) -> Self {
        Self {
            variables,
            commands: HashMap::default(),
        }
    }

    fn with_commands(
        mut self,
        commands: impl IntoIterator<Item = (VsCodeCommand, ZedEnvVariable)>,
    ) -> Self {
        self.commands = commands.into_iter().collect();
        self
    }

    fn replace_value(&self, input: serde_json::Value) -> serde_json::Value {
        match input {
            serde_json::Value::String(s) => serde_json::Value::String(self.replace(&s)),
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(|v| self.replace_value(v)).collect())
            }
            serde_json::Value::Object(obj) => serde_json::Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (self.replace(&k), self.replace_value(v)))
                    .collect(),
            ),
            _ => input,
        }
    }
    // Replaces occurrences of VsCode-specific environment variables with Zed equivalents.
    fn replace(&self, input: &str) -> String {
        shellexpand::env_with_context_no_errors(&input, |var: &str| {
            // Colons denote a default value in case the variable is not set. We want to preserve that default, as otherwise shellexpand will substitute it for us.
            let colon_position = var.find(':').unwrap_or(var.len());
            let (left, right) = var.split_at(colon_position);
            if left == "env" && !right.is_empty() {
                let variable_name = &right[1..];
                return Some(format!("${{{variable_name}}}"));
            } else if left == "command" && !right.is_empty() {
                let command_name = &right[1..];
                if let Some(replacement_command) = self.commands.get(command_name) {
                    return Some(format!("${{{replacement_command}}}"));
                }
            }

            let (variable_name, default) = (left, right);
            let append_previous_default = |ret: &mut String| {
                if !default.is_empty() {
                    ret.push_str(default);
                }
            };
            if let Some(substitution) = self.variables.get(variable_name) {
                // Got a VSCode->Zed hit, perform a substitution
                let mut name = format!("${{{substitution}");
                append_previous_default(&mut name);
                name.push('}');
                return Some(name);
            }
            // This is an unknown variable.
            // We should not error out, as they may come from user environment (e.g. $PATH). That means that the variable substitution might not be perfect.
            // If there's a default, we need to return the string verbatim as otherwise shellexpand will apply that default for us.
            if !default.is_empty() {
                return Some(format!("${{{var}}}"));
            }
            // Else we can just return None and that variable will be left as is.
            None
        })
        .into_owned()
    }
}
