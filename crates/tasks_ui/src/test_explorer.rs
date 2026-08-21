use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[cfg(feature = "rust-test-actions")]
use anyhow::Context as _;
use db::kvp::KeyValueStore;
use gpui::{
    Action, Anchor, App, AsyncWindowContext, Context, DismissEvent, Entity, EventEmitter,
    FocusHandle, Focusable, IntoElement, Pixels, Point, Render, ScrollStrategy, Subscription,
    TaskExt as _, UpdateGlobal as _, WeakEntity, Window, actions, anchored, deferred,
};
use language_tools::language_tool_tree::{
    self, LanguageToolNode, LanguageToolNodeId, LanguageToolProviderStatus, LanguageToolSnapshot,
    LanguageToolTreeHost, LanguageToolTreeStatus, language_tool_tree, status_message,
};
use project::{
    Project, ProjectPath,
    structured_execution::{
        DiscoveryGeneration, StructuredExecutionEvent, StructuredExecutionStoreEvent,
        StructuredExecutionSummary, StructuredNode, StructuredNodeId, StructuredNodeKind,
        StructuredNodeState, StructuredProviderId, StructuredProviderSnapshot,
        StructuredProviderStatus, StructuredRun, StructuredRunId,
    },
};
#[cfg(feature = "rust-test-actions")]
use project::{
    TaskSourceKind,
    rust_test_provider::{
        MAX_RUST_TEST_RERUNS, RUST_TEST_PROVIDER_ID, RustTestAction, RustTestActionPlan,
    },
};
use serde::{Deserialize, Serialize};
use settings::{DockSide, Settings, SettingsStore, TestsPanelSettingsContent};
use ui::{ContextMenu, IconName, Tooltip, prelude::*};
use ui_input::{ErasedEditorEvent, InputField};
#[cfg(feature = "rust-test-actions")]
use workspace::notifications::NotifyResultExt as _;
use workspace::{
    Panel, Workspace,
    dock::{DockPosition, PanelEvent},
    workspace_scoped_state_key,
};

actions!(
    test_explorer,
    [
        ToggleTestsPanel,
        RunSelectedTests,
        DebugSelectedTests,
        CancelTestRun,
        RerunFailedTests,
        RevealTestTerminal,
    ]
);

const TESTS_PANEL_KEY: &str = "TestsPanel";
const TESTS_PANEL_STATE_KEY: &str = "tests-panel-state-v1";
const TESTS_PANEL_STATE_VERSION: u32 = 1;
const MAX_FILTER_BYTES: usize = 256;
const MAX_TREE_DEPTH: usize = 128;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum TestStatusFilter {
    #[default]
    All,
    Failed,
    Passed,
    Skipped,
    Running,
    Cancelled,
}

impl TestStatusFilter {
    fn matches(self, state: Option<StructuredNodeState>) -> bool {
        match self {
            Self::All => true,
            Self::Failed => state == Some(StructuredNodeState::Failed),
            Self::Passed => state == Some(StructuredNodeState::Passed),
            Self::Skipped => state == Some(StructuredNodeState::Skipped),
            Self::Running => matches!(
                state,
                Some(StructuredNodeState::Queued | StructuredNodeState::Running)
            ),
            Self::Cancelled => state == Some(StructuredNodeState::Cancelled),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Failed => "Failed",
            Self::Passed => "Passed",
            Self::Skipped => "Skipped",
            Self::Running => "Running",
            Self::Cancelled => "Cancelled",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TestExplorerFilter {
    pub query: String,
    pub status: TestStatusFilter,
}

impl TestExplorerFilter {
    fn normalized_query(&self) -> String {
        self.query.trim().to_lowercase()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestExplorerSelection {
    pub provider_id: StructuredProviderId,
    pub discovery_generation: DiscoveryGeneration,
    pub node_id: StructuredNodeId,
    pub run_id: Option<StructuredRunId>,
}

pub trait TestExplorerActionDelegate: Send + Sync {
    fn run(
        &self,
        selection: &TestExplorerSelection,
        workspace: &WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()>;

    fn cancel(
        &self,
        selection: &TestExplorerSelection,
        workspace: &WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()>;

    fn debug(
        &self,
        selection: &TestExplorerSelection,
        workspace: &WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()>;

    fn rerun_failed(
        &self,
        selections: &[TestExplorerSelection],
        workspace: &WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()>;

    fn reveal_terminal(
        &self,
        selection: &TestExplorerSelection,
        workspace: &WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()>;
}

#[cfg(not(feature = "rust-test-actions"))]
struct NoopTestExplorerDelegate;

#[cfg(not(feature = "rust-test-actions"))]
impl TestExplorerActionDelegate for NoopTestExplorerDelegate {
    fn run(
        &self,
        _: &TestExplorerSelection,
        _: &WeakEntity<Workspace>,
        _: &mut Window,
        _: &mut App,
    ) -> anyhow::Result<()> {
        anyhow::bail!("No structured test provider can run this selection")
    }

    fn cancel(
        &self,
        _: &TestExplorerSelection,
        _: &WeakEntity<Workspace>,
        _: &mut Window,
        _: &mut App,
    ) -> anyhow::Result<()> {
        anyhow::bail!("No structured test run is active for this selection")
    }

    fn debug(
        &self,
        _: &TestExplorerSelection,
        _: &WeakEntity<Workspace>,
        _: &mut Window,
        _: &mut App,
    ) -> anyhow::Result<()> {
        anyhow::bail!("No structured test provider can debug this selection")
    }

    fn rerun_failed(
        &self,
        _: &[TestExplorerSelection],
        _: &WeakEntity<Workspace>,
        _: &mut Window,
        _: &mut App,
    ) -> anyhow::Result<()> {
        anyhow::bail!("No failed structured tests are available to rerun")
    }

    fn reveal_terminal(
        &self,
        _: &TestExplorerSelection,
        _: &WeakEntity<Workspace>,
        _: &mut Window,
        _: &mut App,
    ) -> anyhow::Result<()> {
        anyhow::bail!("No task terminal is available for this selection")
    }
}

fn default_test_explorer_delegate() -> Arc<dyn TestExplorerActionDelegate> {
    #[cfg(feature = "rust-test-actions")]
    {
        Arc::new(RustTestExplorerDelegate)
    }
    #[cfg(not(feature = "rust-test-actions"))]
    {
        Arc::new(NoopTestExplorerDelegate)
    }
}

#[cfg(feature = "rust-test-actions")]
struct RustTestExplorerDelegate;

#[cfg(feature = "rust-test-actions")]
impl TestExplorerActionDelegate for RustTestExplorerDelegate {
    fn run(
        &self,
        selection: &TestExplorerSelection,
        workspace: &WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()> {
        dispatch_rust_test_action(selection, RustTestAction::Run, workspace, window, cx)
    }

    fn debug(
        &self,
        selection: &TestExplorerSelection,
        workspace: &WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()> {
        dispatch_rust_test_action(selection, RustTestAction::Debug, workspace, window, cx)
    }

    fn cancel(
        &self,
        selection: &TestExplorerSelection,
        workspace: &WeakEntity<Workspace>,
        _: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()> {
        require_rust_provider(selection)?;
        let run_id = selection
            .run_id
            .as_ref()
            .context("The selected Rust test has no active run")?;
        let workspace = workspace
            .upgrade()
            .context("The workspace is no longer available")?;
        let store = workspace
            .read(cx)
            .project()
            .read(cx)
            .rust_test_provider_store()
            .clone();
        store.update(cx, |store, cx| store.cancel_run(run_id, cx))
    }

    fn rerun_failed(
        &self,
        selections: &[TestExplorerSelection],
        workspace: &WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()> {
        let mut scheduled = 0usize;
        let mut skipped = 0usize;
        for selection in selections.iter().take(MAX_RUST_TEST_RERUNS) {
            match dispatch_rust_test_action(selection, RustTestAction::Run, workspace, window, cx) {
                Ok(()) => scheduled = scheduled.saturating_add(1),
                Err(_) => skipped = skipped.saturating_add(1),
            }
        }
        skipped = skipped.saturating_add(selections.len().saturating_sub(MAX_RUST_TEST_RERUNS));
        if scheduled == 0 {
            anyhow::bail!("No current failed Rust tests remain to rerun");
        }
        if skipped > 0 {
            anyhow::bail!(
                "Scheduled {scheduled} failed Rust tests; skipped {skipped} removed, stale, or over-limit selections"
            );
        }
        Ok(())
    }

    fn reveal_terminal(
        &self,
        selection: &TestExplorerSelection,
        workspace: &WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()> {
        require_rust_provider(selection)?;
        let run_id = selection
            .run_id
            .as_ref()
            .context("The selected Rust test has no retained run")?;
        let workspace = workspace
            .upgrade()
            .context("The workspace is no longer available")?;
        let store = workspace
            .read(cx)
            .project()
            .read(cx)
            .rust_test_provider_store()
            .clone();
        store.update(cx, |store, cx| {
            store.reveal_run_terminal(run_id, window, cx)
        })
    }
}

#[cfg(feature = "rust-test-actions")]
fn require_rust_provider(selection: &TestExplorerSelection) -> anyhow::Result<()> {
    anyhow::ensure!(
        selection.provider_id.0 == RUST_TEST_PROVIDER_ID,
        "The selected structured provider is not a Rust test provider"
    );
    Ok(())
}

#[cfg(feature = "rust-test-actions")]
fn dispatch_rust_test_action(
    selection: &TestExplorerSelection,
    action: RustTestAction,
    workspace: &WeakEntity<Workspace>,
    window: &mut Window,
    cx: &mut App,
) -> anyhow::Result<()> {
    use task::SharedTaskContext;

    require_rust_provider(selection)?;
    let workspace_entity = workspace
        .upgrade()
        .context("The workspace is no longer available")?;
    let project = workspace_entity.read(cx).project().clone();
    let provider_store = project.read(cx).rust_test_provider_store().clone();
    let plan = provider_store.update(cx, |store, cx| {
        store.plan_action(
            &selection.node_id,
            selection.discovery_generation,
            action,
            cx,
        )
    });
    let task_contexts = workspace_entity.update(cx, |workspace, cx| {
        crate::task_contexts(workspace, window, cx)
    });
    let workspace = workspace_entity.downgrade();
    window
        .spawn(cx, async move |mut cx| {
            let task_contexts = task_contexts.await;
            let Some(plan) = plan
                .await
                .notify_workspace_async_err(workspace.clone(), &mut cx)
            else {
                return anyhow::Ok(());
            };
            let result = match plan {
                RustTestActionPlan::Task {
                    run_id,
                    discovery_generation,
                    scope_node_ids,
                    worktree_id,
                    template,
                } => {
                    let task_context = task_contexts
                        .task_context_for_worktree_id(worktree_id)
                        .context("No task context exists for the selected Rust test worktree")?;
                    let source = TaskSourceKind::Language {
                        name: "Rust Tests".into(),
                    };
                    let resolved = template
                        .resolve_task(&source.to_id_base(), task_context)
                        .context("The Rust test task could not be resolved")?;
                    let handle = workspace.update_in(cx, |workspace, window, cx| {
                        workspace.schedule_resolved_task_with_structured_handle(
                            source, resolved, false, window, cx,
                        )
                    })?;
                    let registered = provider_store.update(cx, |store, cx| {
                        store.register_run_handle(
                            run_id,
                            discovery_generation,
                            scope_node_ids,
                            handle.clone(),
                            cx,
                        )
                    });
                    if let Err(error) = registered {
                        if let Err(cancel_error) = cx.update(|_, cx| handle.cancel(cx)) {
                            log::warn!(
                                "Failed to cancel an untracked Rust test task: {cancel_error:#}"
                            );
                        }
                        return Err(error);
                    }
                    anyhow::Ok(())
                }
                RustTestActionPlan::Debug {
                    worktree_id,
                    scenario,
                } => {
                    let task_context = task_contexts
                        .task_context_for_worktree_id(worktree_id)
                        .cloned()
                        .context("No task context exists for the selected Rust test worktree")?;
                    workspace.update_in(cx, |workspace, window, cx| {
                        workspace.start_debug_session(
                            scenario,
                            SharedTaskContext::from(task_context),
                            None,
                            Some(worktree_id),
                            window,
                            cx,
                        )
                    })?;
                    anyhow::Ok(())
                }
            };
            if let Err(error) = result {
                Err::<(), _>(error).notify_workspace_async_err(workspace, &mut cx);
            }
            anyhow::Ok(())
        })
        .detach();
    Ok(())
}

#[derive(Clone, Debug)]
pub struct TestExplorerProjection {
    pub snapshot: LanguageToolSnapshot,
    pub summary: StructuredExecutionSummary,
    navigation: HashMap<LanguageToolNodeId, ProjectPath>,
    selections: HashMap<LanguageToolNodeId, TestExplorerSelection>,
    failed: Vec<TestExplorerSelection>,
}

impl Default for TestExplorerProjection {
    fn default() -> Self {
        Self {
            snapshot: LanguageToolSnapshot {
                roots: Vec::new(),
                status: LanguageToolProviderStatus::Empty(
                    "No structured test providers are available".to_string(),
                ),
            },
            summary: StructuredExecutionSummary::default(),
            navigation: HashMap::new(),
            selections: HashMap::new(),
            failed: Vec::new(),
        }
    }
}

impl TestExplorerProjection {
    pub fn navigation(&self, id: &LanguageToolNodeId) -> Option<&ProjectPath> {
        self.navigation.get(id)
    }

    pub fn selection(&self, id: &LanguageToolNodeId) -> Option<&TestExplorerSelection> {
        self.selections.get(id)
    }

    pub fn failed(&self) -> &[TestExplorerSelection] {
        &self.failed
    }
}

pub fn project_test_explorer(
    providers: &[StructuredProviderSnapshot],
    filter: &TestExplorerFilter,
) -> TestExplorerProjection {
    if providers.is_empty() {
        return TestExplorerProjection::default();
    }
    let query = filter.normalized_query();
    let mut providers = providers.to_vec();
    providers.sort_by(|left, right| left.provider_id.cmp(&right.provider_id));
    let mut projection = TestExplorerProjection {
        snapshot: LanguageToolSnapshot {
            roots: Vec::new(),
            status: combined_provider_status(&providers),
        },
        ..TestExplorerProjection::default()
    };

    for provider in &providers {
        let run = provider
            .current_run
            .as_ref()
            .or(provider.last_complete_run.as_ref());
        if let Some(run) = run {
            add_summary(&mut projection.summary, &run.summary);
        }
        let events = run
            .map(|run| {
                run.events
                    .iter()
                    .map(|event| (event.node_id.clone(), event))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        let mut children = HashMap::<Option<StructuredNodeId>, Vec<StructuredNode>>::new();
        for node in &provider.nodes {
            children
                .entry(node.parent_id.clone())
                .or_default()
                .push(node.clone());
        }
        for nodes in children.values_mut() {
            nodes.sort_by(|left, right| left.id.cmp(&right.id));
        }
        let mut visiting = HashSet::new();
        let roots = children.get(&None).cloned().unwrap_or_default();
        for node in roots {
            if let Some(node) = project_node(
                provider,
                run,
                &node,
                &children,
                &events,
                filter,
                &query,
                false,
                0,
                &mut visiting,
                &mut projection,
            ) {
                projection.snapshot.roots.push(node);
            }
        }
    }

    if projection.snapshot.roots.is_empty()
        && matches!(
            projection.snapshot.status,
            LanguageToolProviderStatus::Current
        )
    {
        projection.snapshot.status = LanguageToolProviderStatus::Empty(
            if query.is_empty() && filter.status == TestStatusFilter::All {
                "No tests were discovered".to_string()
            } else {
                "No tests match the current filters".to_string()
            },
        );
    }
    projection
}

#[allow(clippy::too_many_arguments)]
fn project_node(
    provider: &StructuredProviderSnapshot,
    run: Option<&StructuredRun>,
    node: &StructuredNode,
    children: &HashMap<Option<StructuredNodeId>, Vec<StructuredNode>>,
    events: &HashMap<StructuredNodeId, &StructuredExecutionEvent>,
    filter: &TestExplorerFilter,
    query: &str,
    ancestor_query_match: bool,
    depth: usize,
    visiting: &mut HashSet<StructuredNodeId>,
    projection: &mut TestExplorerProjection,
) -> Option<LanguageToolNode> {
    if depth >= MAX_TREE_DEPTH || !visiting.insert(node.id.clone()) {
        return None;
    }
    let event = events.get(&node.id).copied();
    let text_matches =
        ancestor_query_match || query.is_empty() || node.label.to_lowercase().contains(query);
    let mut projected_children = Vec::new();
    if let Some(node_children) = children.get(&Some(node.id.clone())) {
        for child in node_children {
            if let Some(child) = project_node(
                provider,
                run,
                child,
                children,
                events,
                filter,
                query,
                text_matches,
                depth.saturating_add(1),
                visiting,
                projection,
            ) {
                projected_children.push(child);
            }
        }
    }
    visiting.remove(&node.id);

    let status_matches = filter.status.matches(event.map(|event| event.state));
    if (!text_matches || !status_matches) && projected_children.is_empty() {
        return None;
    }
    let tree_id = tree_id(&provider.provider_id, &node.id);
    let navigation = event
        .and_then(|event| event.location.clone())
        .or_else(|| node.path.clone());
    if let Some(path) = navigation {
        projection.navigation.insert(tree_id.clone(), path);
    }
    let navigable = projection.navigation.contains_key(&tree_id);
    let selection = TestExplorerSelection {
        provider_id: provider.provider_id.clone(),
        discovery_generation: provider.discovery_generation,
        node_id: node.id.clone(),
        run_id: run.map(|run| run.id.clone()),
    };
    if event.is_some_and(|event| event.state == StructuredNodeState::Failed) {
        projection.failed.push(selection.clone());
    }
    projection.selections.insert(tree_id.clone(), selection);

    Some(LanguageToolNode {
        id: tree_id,
        label: node.label.clone(),
        secondary_label: event.map(event_label),
        icon: Some(node_icon(node.kind, event.map(|event| event.state))),
        accessibility_label: accessibility_label(node, event),
        children: projected_children,
        enabled: navigable,
        activation_label: navigable.then(|| "Open test source or failure location".to_string()),
    })
}

fn combined_provider_status(
    providers: &[StructuredProviderSnapshot],
) -> LanguageToolProviderStatus {
    let selected = providers
        .iter()
        .min_by_key(|provider| provider_status_priority(provider.status));
    let Some(provider) = selected else {
        return LanguageToolProviderStatus::Empty(
            "No structured test providers are available".to_string(),
        );
    };
    let diagnostic = || {
        provider
            .diagnostic
            .clone()
            .unwrap_or_else(|| provider_status_default_message(provider.status).to_string())
    };
    match provider.status {
        StructuredProviderStatus::Loading => LanguageToolProviderStatus::Loading,
        StructuredProviderStatus::Current => LanguageToolProviderStatus::Current,
        StructuredProviderStatus::Empty => LanguageToolProviderStatus::Empty(diagnostic()),
        StructuredProviderStatus::Partial => LanguageToolProviderStatus::Partial(diagnostic()),
        StructuredProviderStatus::Stale => LanguageToolProviderStatus::Stale(diagnostic()),
        StructuredProviderStatus::Error => LanguageToolProviderStatus::Error(diagnostic()),
        StructuredProviderStatus::Restricted => {
            LanguageToolProviderStatus::Restricted(diagnostic())
        }
        StructuredProviderStatus::Disconnected => {
            LanguageToolProviderStatus::Disconnected(diagnostic())
        }
        StructuredProviderStatus::Mismatch => LanguageToolProviderStatus::Mismatch(diagnostic()),
    }
}

fn provider_status_priority(status: StructuredProviderStatus) -> u8 {
    match status {
        StructuredProviderStatus::Mismatch => 0,
        StructuredProviderStatus::Restricted => 1,
        StructuredProviderStatus::Disconnected => 2,
        StructuredProviderStatus::Error => 3,
        StructuredProviderStatus::Stale => 4,
        StructuredProviderStatus::Partial => 5,
        StructuredProviderStatus::Loading => 6,
        StructuredProviderStatus::Empty => 7,
        StructuredProviderStatus::Current => 8,
    }
}

fn provider_status_default_message(status: StructuredProviderStatus) -> &'static str {
    match status {
        StructuredProviderStatus::Loading => "Test discovery is loading",
        StructuredProviderStatus::Current => "Test discovery is current",
        StructuredProviderStatus::Empty => "No tests were discovered",
        StructuredProviderStatus::Partial => "Some tests could not be discovered",
        StructuredProviderStatus::Stale => "Test discovery is stale; refresh to retry",
        StructuredProviderStatus::Error => "Test discovery failed; refresh to retry",
        StructuredProviderStatus::Restricted => "Trust this project to discover and run tests",
        StructuredProviderStatus::Disconnected => "Reconnect to the project host to load tests",
        StructuredProviderStatus::Mismatch => {
            "The project host does not support this structured test protocol"
        }
    }
}

fn tree_id(provider_id: &StructuredProviderId, node_id: &StructuredNodeId) -> LanguageToolNodeId {
    LanguageToolNodeId(format!(
        "provider:{}:{}:node:{}:{}",
        provider_id.0.len(),
        provider_id.0,
        node_id.0.len(),
        node_id.0
    ))
}

fn node_icon(kind: StructuredNodeKind, state: Option<StructuredNodeState>) -> IconName {
    match state {
        Some(StructuredNodeState::Passed) => IconName::Check,
        Some(StructuredNodeState::Failed) => IconName::XCircle,
        Some(StructuredNodeState::Skipped) => IconName::Circle,
        Some(StructuredNodeState::Queued) => IconName::Clock,
        Some(StructuredNodeState::Running) => IconName::LoadCircle,
        Some(StructuredNodeState::Cancelled) => IconName::Stop,
        None => match kind {
            StructuredNodeKind::Provider => IconName::ListTree,
            StructuredNodeKind::Suite => IconName::Folder,
            StructuredNodeKind::Group => IconName::ListTree,
            StructuredNodeKind::Case => IconName::Circle,
        },
    }
}

fn event_label(event: &StructuredExecutionEvent) -> String {
    let status = match event.state {
        StructuredNodeState::Queued => "queued",
        StructuredNodeState::Running => "running",
        StructuredNodeState::Passed => "passed",
        StructuredNodeState::Failed => "failed",
        StructuredNodeState::Skipped => "skipped",
        StructuredNodeState::Cancelled => "cancelled",
    };
    event
        .duration_millis
        .map(|duration| format!("{status} · {duration} ms"))
        .unwrap_or_else(|| status.to_string())
}

fn accessibility_label(node: &StructuredNode, event: Option<&StructuredExecutionEvent>) -> String {
    event
        .map(|event| format!("{}, {}", node.label, event_label(event)))
        .unwrap_or_else(|| node.label.clone())
}

fn add_summary(target: &mut StructuredExecutionSummary, source: &StructuredExecutionSummary) {
    target.total = target.total.saturating_add(source.total);
    target.queued = target.queued.saturating_add(source.queued);
    target.running = target.running.saturating_add(source.running);
    target.passed = target.passed.saturating_add(source.passed);
    target.failed = target.failed.saturating_add(source.failed);
    target.skipped = target.skipped.saturating_add(source.skipped);
    target.cancelled = target.cancelled.saturating_add(source.cancelled);
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TestsPanelSettings {
    button: bool,
    default_width: Pixels,
    dock: DockSide,
    starts_open: bool,
}

impl Settings for TestsPanelSettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        let settings = content.tests_panel.as_ref();
        Self {
            button: settings
                .and_then(|settings| settings.button)
                .unwrap_or(true),
            default_width: gpui::px(
                settings
                    .and_then(|settings| settings.default_width)
                    .unwrap_or(300.),
            ),
            dock: settings
                .and_then(|settings| settings.dock)
                .unwrap_or(DockSide::Left),
            starts_open: settings
                .and_then(|settings| settings.starts_open)
                .unwrap_or(false),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
struct PersistedTestsPanelState {
    version: u32,
    query: String,
    status: TestStatusFilter,
    selected_provider: Option<String>,
    selected_node: Option<String>,
}

impl PersistedTestsPanelState {
    fn sanitized(mut self) -> Self {
        self.version = TESTS_PANEL_STATE_VERSION;
        self.query = bounded_filter(&self.query);
        self.selected_provider = self.selected_provider.map(|value| bounded_filter(&value));
        self.selected_node = self.selected_node.map(|value| bounded_filter(&value));
        self
    }
}

fn bounded_filter(value: &str) -> String {
    if value.len() <= MAX_FILTER_BYTES {
        return value.to_string();
    }
    let mut end = MAX_FILTER_BYTES;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn persist_state(key: String, state: PersistedTestsPanelState, cx: &App) -> gpui::Task<()> {
    let key_value_store = KeyValueStore::global(cx);
    cx.background_spawn(async move {
        let result = async {
            key_value_store
                .write_kvp(key, serde_json::to_string(&state.sanitized())?)
                .await?;
            anyhow::Ok(())
        }
        .await;
        if let Err(error) = result {
            log::warn!("failed to persist Tests panel state: {error:#}");
        }
    })
}

pub struct TestsPanel {
    project: Entity<Project>,
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    query_input: Entity<InputField>,
    active: bool,
    host: LanguageToolTreeHost,
    filter: TestExplorerFilter,
    projection: TestExplorerProjection,
    snapshots: Vec<StructuredProviderSnapshot>,
    provider_ids: Vec<StructuredProviderId>,
    restored_selection: Option<(StructuredProviderId, StructuredNodeId)>,
    delegate: Arc<dyn TestExplorerActionDelegate>,
    state_key: Option<String>,
    serialization_task: gpui::Task<()>,
    action_notice: Option<String>,
    context_menu: Option<(Entity<ContextMenu>, Point<Pixels>, Subscription)>,
    _subscriptions: Vec<Subscription>,
}

impl TestsPanel {
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> anyhow::Result<Entity<Self>> {
        let workspace_handle = workspace.clone();
        workspace.update_in(&mut cx, |workspace, window, cx| {
            let project = workspace.project().clone();
            let state_key = workspace_scoped_state_key(
                TESTS_PANEL_STATE_KEY,
                workspace.database_id(),
                workspace.session_id().as_deref(),
            );
            let restored = state_key
                .as_ref()
                .and_then(|key| match KeyValueStore::global(cx).read_kvp(key) {
                    Ok(value) => value,
                    Err(error) => {
                        log::warn!("failed to restore Tests panel state: {error:#}");
                        None
                    }
                })
                .and_then(
                    |value| match serde_json::from_str::<PersistedTestsPanelState>(&value) {
                        Ok(state) if state.version == TESTS_PANEL_STATE_VERSION => {
                            Some(state.sanitized())
                        }
                        Ok(_) => None,
                        Err(error) => {
                            log::warn!("failed to parse Tests panel state: {error:#}");
                            None
                        }
                    },
                )
                .unwrap_or_default();
            let query_input = cx.new(|cx| {
                InputField::new(window, cx, "Filter tests…")
                    .start_icon(IconName::MagnifyingGlass)
                    .tab_index(0)
            });
            let query_editor = query_input.read(cx).editor().clone();
            query_editor.set_text(&restored.query, window, cx);
            cx.new(|cx: &mut Context<TestsPanel>| {
                let store = project.read(cx).structured_execution_store().clone();
                let store_subscription = cx
                    .subscribe(&store, |panel, _, _: &StructuredExecutionStoreEvent, cx| {
                        panel.invalidate(cx)
                    });
                let panel = cx.weak_entity();
                let query_editor = query_input.read(cx).editor().clone();
                let query_subscription = query_editor.subscribe(
                    Box::new(move |event, _, cx| {
                        if event == ErasedEditorEvent::BufferEdited {
                            if let Err(error) = panel.update(cx, |panel, cx| {
                                panel.filter.query =
                                    bounded_filter(&panel.query_input.read(cx).text(cx));
                                panel.rebuild_projection(cx);
                                panel.persist(cx);
                            }) {
                                log::debug!("Tests panel was dropped: {error:#}");
                            }
                        }
                    }),
                    window,
                    cx,
                );
                let provider_ids = store
                    .read(cx)
                    .state()
                    .providers()
                    .map(|provider| provider.provider_id.clone())
                    .collect();
                TestsPanel {
                    project,
                    workspace: workspace_handle,
                    focus_handle: cx.focus_handle(),
                    query_input,
                    active: false,
                    host: LanguageToolTreeHost::default(),
                    filter: TestExplorerFilter {
                        query: restored.query,
                        status: restored.status,
                    },
                    projection: TestExplorerProjection::default(),
                    snapshots: Vec::new(),
                    provider_ids,
                    restored_selection: restored.selected_provider.zip(restored.selected_node).map(
                        |(provider, node)| (StructuredProviderId(provider), StructuredNodeId(node)),
                    ),
                    delegate: default_test_explorer_delegate(),
                    state_key,
                    serialization_task: gpui::Task::ready(()),
                    action_notice: None,
                    context_menu: None,
                    _subscriptions: vec![store_subscription, query_subscription],
                }
            })
        })
    }

    pub fn set_delegate(&mut self, delegate: Arc<dyn TestExplorerActionDelegate>) {
        self.delegate = delegate;
    }

    pub fn set_provider_ids(
        &mut self,
        provider_ids: impl IntoIterator<Item = StructuredProviderId>,
        cx: &mut Context<Self>,
    ) {
        self.provider_ids = provider_ids.into_iter().collect();
        self.provider_ids.sort();
        self.provider_ids.dedup();
        self.refresh(cx);
    }

    pub fn toggle_focus(
        workspace: &mut Workspace,
        _: &ToggleTestsPanel,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        workspace.toggle_panel_focus::<Self>(window, cx);
    }

    fn invalidate(&mut self, cx: &mut Context<Self>) {
        if !self.active
            || matches!(
                self.host.status(),
                LanguageToolTreeStatus::Loading | LanguageToolTreeStatus::Refreshing
            )
        {
            self.host.mark_dirty();
        } else {
            self.refresh(cx);
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let generation = self.host.start_refresh();
        let store = self.project.read(cx).structured_execution_store().clone();
        let mut provider_ids = self.provider_ids.clone();
        provider_ids.extend(
            store
                .read(cx)
                .state()
                .providers()
                .map(|provider| provider.provider_id.clone()),
        );
        provider_ids.sort();
        provider_ids.dedup();
        self.provider_ids = provider_ids.clone();
        if provider_ids.is_empty() {
            self.snapshots.clear();
            self.projection = TestExplorerProjection::default();
            self.host
                .apply_refresh(generation, Ok(self.projection.snapshot.clone()));
            cx.notify();
            return;
        }
        let refreshes = provider_ids
            .into_iter()
            .map(|provider_id| {
                store.update(cx, |store, cx| store.refresh_provider(provider_id, cx))
            })
            .collect::<Vec<_>>();
        let task = cx.spawn(async move |this, cx| {
            let results = futures::future::join_all(refreshes).await;
            this.update(cx, |panel, cx| {
                let mut snapshots = Vec::new();
                let mut first_error = None;
                for result in results {
                    match result {
                        Ok(snapshot) => snapshots.push(snapshot),
                        Err(error) if first_error.is_none() => first_error = Some(error),
                        Err(_) => {}
                    }
                }
                if snapshots.is_empty() {
                    panel.host.apply_refresh(
                        generation,
                        Err(first_error.unwrap_or_else(|| {
                            anyhow::anyhow!("No structured test provider returned a snapshot")
                        })),
                    );
                } else {
                    panel.snapshots = snapshots;
                    panel.rebuild_projection_for_generation(generation);
                }
                cx.notify();
            })
            .ok();
        });
        self.host.replace_refresh_task(task);
        cx.notify();
    }

    fn rebuild_projection(&mut self, cx: &mut Context<Self>) {
        self.projection = project_test_explorer(&self.snapshots, &self.filter);
        self.host.replace_snapshot(self.projection.snapshot.clone());
        self.restore_selection();
        cx.notify();
    }

    fn rebuild_projection_for_generation(&mut self, generation: u64) {
        self.projection = project_test_explorer(&self.snapshots, &self.filter);
        self.host
            .apply_refresh(generation, Ok(self.projection.snapshot.clone()));
        self.restore_selection();
    }

    fn restore_selection(&mut self) {
        let Some((provider_id, node_id)) = self.restored_selection.take() else {
            return;
        };
        self.host.select(tree_id(&provider_id, &node_id));
    }

    fn selected(&self) -> Option<&TestExplorerSelection> {
        self.host
            .selected()
            .and_then(|id| self.projection.selection(id))
    }

    fn set_status_filter(&mut self, status: TestStatusFilter, cx: &mut Context<Self>) {
        self.filter.status = status;
        self.rebuild_projection(cx);
        self.persist(cx);
    }

    fn activate_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.host.can_activate() {
            return;
        }
        let Some(path) = self
            .host
            .selected()
            .and_then(|id| self.projection.navigation(id))
            .cloned()
        else {
            return;
        };
        if let Some(workspace) = self.workspace.upgrade() {
            workspace
                .update(cx, |workspace, cx| {
                    workspace.open_path(path, None, true, window, cx)
                })
                .detach_and_log_err(cx);
        }
    }

    fn run_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selection) = self.selected().cloned() else {
            self.set_notice("Select a test, group, or suite to run", cx);
            return;
        };
        let delegate = self.delegate.clone();
        let workspace = self.workspace.clone();
        self.handle_action_result(delegate.run(&selection, &workspace, window, cx), cx);
    }

    fn debug_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selection) = self.selected().cloned() else {
            self.set_notice("Select an individual test case to debug", cx);
            return;
        };
        let delegate = self.delegate.clone();
        let workspace = self.workspace.clone();
        self.handle_action_result(delegate.debug(&selection, &workspace, window, cx), cx);
    }

    fn cancel_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selection) = self.selected().cloned().or_else(|| {
            self.projection
                .selections
                .values()
                .find(|selection| selection.run_id.is_some())
                .cloned()
        }) else {
            self.set_notice("No structured test run is active", cx);
            return;
        };
        let delegate = self.delegate.clone();
        let workspace = self.workspace.clone();
        self.handle_action_result(delegate.cancel(&selection, &workspace, window, cx), cx);
    }

    fn rerun_failed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.projection.failed.is_empty() {
            self.set_notice("No failed tests are available to rerun", cx);
            return;
        }
        let delegate = self.delegate.clone();
        let workspace = self.workspace.clone();
        let failed = self.projection.failed.clone();
        self.handle_action_result(delegate.rerun_failed(&failed, &workspace, window, cx), cx);
    }

    fn reveal_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selection) = self.selected().cloned() else {
            self.set_notice("Select a test result with an owning task terminal", cx);
            return;
        };
        let delegate = self.delegate.clone();
        let workspace = self.workspace.clone();
        self.handle_action_result(
            delegate.reveal_terminal(&selection, &workspace, window, cx),
            cx,
        );
    }

    fn handle_action_result(&mut self, result: anyhow::Result<()>, cx: &mut Context<Self>) {
        match result {
            Ok(()) => self.action_notice = None,
            Err(error) => self.action_notice = Some(error.to_string()),
        }
        cx.notify();
    }

    fn set_notice(&mut self, notice: impl Into<String>, cx: &mut Context<Self>) {
        self.action_notice = Some(notice.into());
        cx.notify();
    }

    fn persist(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.state_key.clone() else {
            return;
        };
        let selected = self.selected().cloned();
        self.serialization_task = persist_state(
            key,
            PersistedTestsPanelState {
                version: TESTS_PANEL_STATE_VERSION,
                query: self.filter.query.clone(),
                status: self.filter.status,
                selected_provider: selected
                    .as_ref()
                    .map(|selection| selection.provider_id.0.clone()),
                selected_node: selected.map(|selection| selection.node_id.0),
            },
            cx,
        );
    }

    fn deploy_context_menu(
        &mut self,
        position: Point<Pixels>,
        id: LanguageToolNodeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.host.select(id);
        let can_open = self
            .host
            .selected()
            .is_some_and(|id| self.projection.navigation(id).is_some());
        let has_selection = self.selected().is_some();
        let has_failed = !self.projection.failed.is_empty();
        let menu = ContextMenu::build(window, cx, |menu, _, _| {
            menu.context(self.focus_handle.clone())
                .action_disabled_when(!has_selection, "Run", Box::new(RunSelectedTests))
                .action_disabled_when(!has_selection, "Debug", Box::new(DebugSelectedTests))
                .action_disabled_when(!has_selection, "Cancel", Box::new(CancelTestRun))
                .action_disabled_when(!has_failed, "Rerun Failed", Box::new(RerunFailedTests))
                .action_disabled_when(
                    !has_selection,
                    "Show Task Terminal",
                    Box::new(RevealTestTerminal),
                )
                .separator()
                .when(can_open, |menu| {
                    menu.action(
                        "Open Location",
                        Box::new(language_tool_tree::ActivateSelected),
                    )
                })
                .action("Refresh", Box::new(language_tool_tree::Refresh))
                .separator()
                .action("Expand All", Box::new(language_tool_tree::ExpandAll))
                .action("Collapse All", Box::new(language_tool_tree::CollapseAll))
        });
        window.focus(&menu.focus_handle(cx), cx);
        let subscription = cx.subscribe(&menu, |panel, _, _: &DismissEvent, cx| {
            panel.context_menu.take();
            cx.notify();
        });
        self.context_menu = Some((menu, position, subscription));
        self.persist(cx);
        cx.notify();
    }
}

impl Panel for TestsPanel {
    fn persistent_name() -> &'static str {
        "Tests"
    }

    fn panel_key() -> &'static str {
        TESTS_PANEL_KEY
    }

    fn position(&self, _: &Window, cx: &App) -> DockPosition {
        match TestsPanelSettings::get_global(cx).dock {
            DockSide::Left => DockPosition::Left,
            DockSide::Right => DockPosition::Right,
        }
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }

    fn set_position(&mut self, position: DockPosition, _: &mut Window, cx: &mut Context<Self>) {
        settings::update_settings_file(<dyn fs::Fs>::global(cx), cx, move |settings, _| {
            settings.tests_panel.get_or_insert_default().dock = Some(match position {
                DockPosition::Right => DockSide::Right,
                DockPosition::Left | DockPosition::Bottom => DockSide::Left,
            });
        });
    }

    fn default_size(&self, _: &Window, cx: &App) -> Pixels {
        TestsPanelSettings::get_global(cx).default_width
    }

    fn icon(&self, _: &Window, cx: &App) -> Option<IconName> {
        TestsPanelSettings::get_global(cx)
            .button
            .then_some(IconName::ListTodo)
    }

    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
        Some("Tests")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleTestsPanel)
    }

    fn starts_open(&self, _: &Window, cx: &App) -> bool {
        TestsPanelSettings::get_global(cx).starts_open
    }

    fn set_active(&mut self, active: bool, _: &mut Window, cx: &mut Context<Self>) {
        let was_active = self.active;
        self.active = active;
        if active
            && !was_active
            && (matches!(self.host.status(), LanguageToolTreeStatus::Dormant)
                || self.host.take_dirty())
        {
            self.refresh(cx);
        }
    }

    fn activation_priority(&self) -> u32 {
        8
    }

    fn hide_button_setting(&self, _: &App) -> Option<workspace::HideStatusItem> {
        Some(workspace::HideStatusItem::new(|settings| {
            settings.tests_panel.get_or_insert_default().button = Some(false);
        }))
    }
}

impl Focusable for TestsPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for TestsPanel {}

impl Render for TestsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.host.visible_rows().to_vec();
        let selected = self.host.selected().cloned();
        let status = status_message(self.host.status());
        let can_expand_all = self.host.can_expand_all();
        let can_collapse_all = self.host.can_collapse_all();
        let can_refresh = self.host.can_refresh();
        let has_selection = self.selected().is_some();
        let has_failed = !self.projection.failed.is_empty();
        let has_run = self
            .projection
            .selections
            .values()
            .any(|selection| selection.run_id.is_some());
        let summary = &self.projection.summary;
        let summary_label = format!(
            "{} total · {} passed · {} failed · {} skipped",
            summary.total, summary.passed, summary.failed, summary.skipped
        );
        let click_panel = cx.weak_entity();
        let toggle_panel = cx.weak_entity();
        let context_panel = cx.weak_entity();
        v_flex()
            .id("tests-panel")
            .key_context("TestsPanel")
            .track_focus(&self.focus_handle)
            .size_full()
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::Refresh, _, cx| panel.refresh(cx)),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::ExpandAll, _, cx| {
                    panel.host.expand_all();
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::CollapseAll, _, cx| {
                    panel.host.collapse_all();
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectNext, _, cx| {
                    panel.host.select_next();
                    panel.host.reveal_selection(ScrollStrategy::Center);
                    panel.persist(cx);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectPrevious, _, cx| {
                    panel.host.select_previous();
                    panel.host.reveal_selection(ScrollStrategy::Center);
                    panel.persist(cx);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectFirst, _, cx| {
                    panel.host.select_first();
                    panel.host.reveal_selection(ScrollStrategy::Top);
                    panel.persist(cx);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectLast, _, cx| {
                    panel.host.select_last();
                    panel.host.reveal_selection(ScrollStrategy::Bottom);
                    panel.persist(cx);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectParent, _, cx| {
                    panel.host.select_parent();
                    panel.host.reveal_selection(ScrollStrategy::Center);
                    panel.persist(cx);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::SelectFirstChild, _, cx| {
                    panel.host.select_first_child();
                    panel.host.reveal_selection(ScrollStrategy::Center);
                    panel.persist(cx);
                    cx.notify();
                }),
            )
            .on_action(
                cx.listener(|panel, _: &language_tool_tree::ToggleExpanded, _, cx| {
                    panel.host.toggle_selected();
                    cx.notify();
                }),
            )
            .on_action(cx.listener(
                |panel, _: &language_tool_tree::ActivateSelected, window, cx| {
                    panel.activate_selected(window, cx)
                },
            ))
            .on_action(
                cx.listener(|panel, _: &RunSelectedTests, window, cx| {
                    panel.run_selected(window, cx)
                }),
            )
            .on_action(cx.listener(|panel, _: &DebugSelectedTests, window, cx| {
                panel.debug_selected(window, cx)
            }))
            .on_action(
                cx.listener(|panel, _: &CancelTestRun, window, cx| {
                    panel.cancel_selected(window, cx)
                }),
            )
            .on_action(
                cx.listener(|panel, _: &RerunFailedTests, window, cx| {
                    panel.rerun_failed(window, cx)
                }),
            )
            .on_action(cx.listener(|panel, _: &RevealTestTerminal, window, cx| {
                panel.reveal_terminal(window, cx)
            }))
            .child(
                div()
                    .h_9()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child("Tests")
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                IconButton::new("tests-run", IconName::PlayFilled)
                                    .aria_label("Run Selected Tests")
                                    .tooltip(Tooltip::text("Run Selected Tests"))
                                    .disabled(!has_selection)
                                    .on_click(cx.listener(|panel, _, window, cx| {
                                        panel.run_selected(window, cx)
                                    })),
                            )
                            .child(
                                IconButton::new("tests-debug", IconName::Debug)
                                    .aria_label("Debug Selected Test")
                                    .tooltip(Tooltip::text("Debug Selected Test"))
                                    .disabled(!has_selection)
                                    .on_click(cx.listener(|panel, _, window, cx| {
                                        panel.debug_selected(window, cx)
                                    })),
                            )
                            .child(
                                IconButton::new("tests-cancel", IconName::Stop)
                                    .aria_label("Cancel Test Run")
                                    .tooltip(Tooltip::text("Cancel Test Run"))
                                    .disabled(!has_run)
                                    .on_click(cx.listener(|panel, _, window, cx| {
                                        panel.cancel_selected(window, cx)
                                    })),
                            )
                            .child(
                                IconButton::new("tests-rerun-failed", IconName::Rerun)
                                    .aria_label("Rerun Failed Tests")
                                    .tooltip(Tooltip::text("Rerun Failed Tests"))
                                    .disabled(!has_failed)
                                    .on_click(cx.listener(|panel, _, window, cx| {
                                        panel.rerun_failed(window, cx)
                                    })),
                            )
                            .child(
                                IconButton::new("tests-terminal", IconName::TerminalAlt)
                                    .aria_label("Show Task Terminal")
                                    .tooltip(Tooltip::text("Show Task Terminal"))
                                    .disabled(!has_selection)
                                    .on_click(cx.listener(|panel, _, window, cx| {
                                        panel.reveal_terminal(window, cx)
                                    })),
                            )
                            .child(
                                IconButton::new("tests-expand-all", IconName::ExpandVertical)
                                    .aria_label("Expand All")
                                    .tooltip(Tooltip::text("Expand All"))
                                    .disabled(!can_expand_all)
                                    .on_click(cx.listener(|panel, _, _, cx| {
                                        panel.host.expand_all();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                IconButton::new("tests-collapse-all", IconName::ListCollapse)
                                    .aria_label("Collapse All")
                                    .tooltip(Tooltip::text("Collapse All"))
                                    .disabled(!can_collapse_all)
                                    .on_click(cx.listener(|panel, _, _, cx| {
                                        panel.host.collapse_all();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                IconButton::new("tests-refresh", IconName::RefreshTitle)
                                    .aria_label("Refresh Tests")
                                    .tooltip(Tooltip::text("Refresh Tests"))
                                    .disabled(!can_refresh)
                                    .on_click(cx.listener(|panel, _, _, cx| panel.refresh(cx))),
                            ),
                    ),
            )
            .child(div().px_2().pb_1().child(self.query_input.clone()))
            .child(
                h_flex().px_2().pb_1().gap_1().children(
                    [
                        TestStatusFilter::All,
                        TestStatusFilter::Failed,
                        TestStatusFilter::Passed,
                        TestStatusFilter::Skipped,
                        TestStatusFilter::Running,
                        TestStatusFilter::Cancelled,
                    ]
                    .into_iter()
                    .map(|status| {
                        Button::new(
                            format!("tests-filter-{}", status.label().to_lowercase()),
                            status.label(),
                        )
                        .toggle_state(self.filter.status == status)
                        .on_click(
                            cx.listener(move |panel, _, _, cx| panel.set_status_filter(status, cx)),
                        )
                    }),
                ),
            )
            .child(
                div()
                    .px_2()
                    .py_1()
                    .text_color(cx.theme().colors().text_muted)
                    .child(summary_label),
            )
            .when_some(self.action_notice.clone(), |panel, notice| {
                panel.child(
                    div()
                        .px_2()
                        .py_1()
                        .text_color(cx.theme().status().warning)
                        .child(notice),
                )
            })
            .child(language_tool_tree(
                rows,
                selected,
                status,
                self.host.scroll_handle().clone(),
                move |id, click_count, window, cx| {
                    if let Err(error) = click_panel.update(cx, |panel, cx| {
                        window.focus(&panel.focus_handle, cx);
                        panel.host.select(id.clone());
                        panel.persist(cx);
                        if click_count > 1 {
                            panel.activate_selected(window, cx);
                        }
                        cx.notify();
                    }) {
                        log::debug!("Tests panel was dropped: {error:#}");
                    }
                },
                move |id, _, cx| {
                    if let Err(error) = toggle_panel.update(cx, |panel, cx| {
                        panel.host.select(id.clone());
                        panel.host.toggle(&id);
                        cx.notify();
                    }) {
                        log::debug!("Tests panel was dropped: {error:#}");
                    }
                },
                move |id, position, window, cx| {
                    if let Err(error) = context_panel.update(cx, |panel, cx| {
                        panel.deploy_context_menu(position, id, window, cx)
                    }) {
                        log::debug!("Tests panel was dropped: {error:#}");
                    }
                },
            ))
            .children(self.context_menu.as_ref().map(|(menu, position, _)| {
                deferred(
                    anchored()
                        .position(*position)
                        .anchor(Anchor::TopLeft)
                        .child(menu.clone()),
                )
                .with_priority(1)
            }))
    }
}

pub fn init(cx: &mut App) {
    SettingsStore::update_global(cx, |store, cx| {
        store.update_default_settings(cx, |settings| {
            settings.tests_panel = Some(TestsPanelSettingsContent {
                button: Some(true),
                default_width: Some(300.),
                dock: Some(DockSide::Left),
                starts_open: Some(false),
            });
        });
        store.register_setting::<TestsPanelSettings>();
    });
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(TestsPanel::toggle_focus);
    })
    .detach();
    cx.bind_keys([
        gpui::KeyBinding::new("up", language_tool_tree::SelectPrevious, Some("TestsPanel")),
        gpui::KeyBinding::new("down", language_tool_tree::SelectNext, Some("TestsPanel")),
        gpui::KeyBinding::new("home", language_tool_tree::SelectFirst, Some("TestsPanel")),
        gpui::KeyBinding::new("end", language_tool_tree::SelectLast, Some("TestsPanel")),
        gpui::KeyBinding::new("left", language_tool_tree::SelectParent, Some("TestsPanel")),
        gpui::KeyBinding::new(
            "right",
            language_tool_tree::SelectFirstChild,
            Some("TestsPanel"),
        ),
        gpui::KeyBinding::new(
            "space",
            language_tool_tree::ToggleExpanded,
            Some("TestsPanel"),
        ),
        gpui::KeyBinding::new(
            "enter",
            language_tool_tree::ActivateSelected,
            Some("TestsPanel"),
        ),
        gpui::KeyBinding::new(
            "cmd-shift-e",
            language_tool_tree::ExpandAll,
            Some("TestsPanel"),
        ),
        gpui::KeyBinding::new(
            "cmd-shift-c",
            language_tool_tree::CollapseAll,
            Some("TestsPanel"),
        ),
        gpui::KeyBinding::new("cmd-r", language_tool_tree::Refresh, Some("TestsPanel")),
    ]);
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use gpui::TestAppContext;
    use settings::WorktreeId;
    use util::rel_path::RelPath;

    use super::*;

    fn path(value: &str) -> ProjectPath {
        ProjectPath {
            worktree_id: WorktreeId::from_usize(1),
            path: Arc::from(RelPath::unix(value).expect("fixture path should be relative")),
        }
    }

    fn fake_provider(status: StructuredProviderStatus) -> StructuredProviderSnapshot {
        let provider_id = StructuredProviderId("web-tests".to_string());
        let root_id = StructuredNodeId("web-root".to_string());
        let suite_id = StructuredNodeId("browser-suite".to_string());
        let passed_id = StructuredNodeId("renders-home".to_string());
        let failed_id = StructuredNodeId("renders-login".to_string());
        let mut run = StructuredRun::new(
            StructuredRunId("run-1".to_string()),
            DiscoveryGeneration(3),
            vec![suite_id.clone()],
        );
        run.events = vec![
            StructuredExecutionEvent {
                sequence: 0,
                node_id: passed_id.clone(),
                state: StructuredNodeState::Passed,
                duration_millis: Some(4),
                message: None,
                location: Some(path("web/home.test.js")),
            },
            StructuredExecutionEvent {
                sequence: 1,
                node_id: failed_id.clone(),
                state: StructuredNodeState::Failed,
                duration_millis: Some(9),
                message: Some("expected dashboard".to_string()),
                location: Some(path("web/login.test.js")),
            },
        ];
        run.summary = StructuredExecutionSummary {
            total: 2,
            passed: 1,
            failed: 1,
            ..Default::default()
        };
        StructuredProviderSnapshot {
            provider_id,
            discovery_generation: DiscoveryGeneration(3),
            status,
            nodes: vec![
                StructuredNode {
                    id: root_id.clone(),
                    parent_id: None,
                    label: "Web Tests".to_string(),
                    kind: StructuredNodeKind::Provider,
                    path: None,
                },
                StructuredNode {
                    id: suite_id.clone(),
                    parent_id: Some(root_id),
                    label: "Browser".to_string(),
                    kind: StructuredNodeKind::Suite,
                    path: Some(path("web")),
                },
                StructuredNode {
                    id: passed_id,
                    parent_id: Some(suite_id.clone()),
                    label: "renders home".to_string(),
                    kind: StructuredNodeKind::Case,
                    path: Some(path("web/home.test.js")),
                },
                StructuredNode {
                    id: failed_id,
                    parent_id: Some(suite_id),
                    label: "renders login".to_string(),
                    kind: StructuredNodeKind::Case,
                    path: Some(path("web/login.test.js")),
                },
            ],
            partial: status == StructuredProviderStatus::Partial,
            diagnostic: (status != StructuredProviderStatus::Current)
                .then(|| provider_status_default_message(status).to_string()),
            current_run: None,
            last_complete_run: Some(run),
            completed_runs: VecDeque::new(),
        }
    }

    #[test]
    fn test_explorer_projects_non_rust_results_and_filters_stably() {
        let provider = fake_provider(StructuredProviderStatus::Current);
        let all = project_test_explorer(
            std::slice::from_ref(&provider),
            &TestExplorerFilter::default(),
        );
        let again = project_test_explorer(
            std::slice::from_ref(&provider),
            &TestExplorerFilter::default(),
        );
        assert_eq!(all.snapshot.roots[0].id, again.snapshot.roots[0].id);
        assert_eq!(all.summary.total, 2);
        assert_eq!(all.failed.len(), 1);
        assert!(
            all.snapshot.roots[0]
                .accessibility_label
                .contains("Web Tests")
        );
        assert!(
            all.navigation
                .values()
                .any(|value| value == &path("web/login.test.js"))
        );

        let failed = project_test_explorer(
            std::slice::from_ref(&provider),
            &TestExplorerFilter {
                query: String::new(),
                status: TestStatusFilter::Failed,
            },
        );
        assert!(find_node(&failed.snapshot.roots, "renders login").is_some());
        assert!(find_node(&failed.snapshot.roots, "renders home").is_none());

        let query = project_test_explorer(
            &[provider],
            &TestExplorerFilter {
                query: "login".to_string(),
                status: TestStatusFilter::All,
            },
        );
        assert!(find_node(&query.snapshot.roots, "renders login").is_some());
        assert!(find_node(&query.snapshot.roots, "renders home").is_none());
        assert!(
            collect_ids(&query.snapshot.roots)
                .iter()
                .all(|id| !id.0.contains("Cargo") && !id.0.contains("Rust"))
        );
    }

    #[test]
    fn test_explorer_maps_every_provider_state_distinctly() {
        let cases = [
            (StructuredProviderStatus::Loading, "loading"),
            (StructuredProviderStatus::Empty, "No tests"),
            (StructuredProviderStatus::Partial, "Some tests"),
            (StructuredProviderStatus::Stale, "stale"),
            (StructuredProviderStatus::Error, "failed"),
            (StructuredProviderStatus::Restricted, "Trust"),
            (StructuredProviderStatus::Disconnected, "Reconnect"),
            (StructuredProviderStatus::Mismatch, "protocol"),
        ];
        for (status, expected) in cases {
            let projection =
                project_test_explorer(&[fake_provider(status)], &TestExplorerFilter::default());
            let message = match projection.snapshot.status {
                LanguageToolProviderStatus::Loading => "loading".to_string(),
                LanguageToolProviderStatus::Empty(message)
                | LanguageToolProviderStatus::Partial(message)
                | LanguageToolProviderStatus::Stale(message)
                | LanguageToolProviderStatus::Restricted(message)
                | LanguageToolProviderStatus::Unsupported(message)
                | LanguageToolProviderStatus::Mismatch(message)
                | LanguageToolProviderStatus::Error(message)
                | LanguageToolProviderStatus::Disconnected(message) => message,
                LanguageToolProviderStatus::Current => "current".to_string(),
            };
            assert!(
                message.to_lowercase().contains(&expected.to_lowercase()),
                "{status:?} produced {message:?}"
            );
        }
    }

    #[test]
    fn test_explorer_persistence_contains_only_filters_and_opaque_ids() {
        let state = PersistedTestsPanelState {
            version: TESTS_PANEL_STATE_VERSION,
            query: "login".to_string(),
            status: TestStatusFilter::Failed,
            selected_provider: Some("web-tests".to_string()),
            selected_node: Some("renders-login".to_string()),
        };
        let serialized = serde_json::to_string(&state).expect("state should serialize");
        assert!(!serialized.contains("expected dashboard"));
        assert!(!serialized.contains("/Users/"));
        assert!(!serialized.contains("project_env"));
        assert!(!serialized.contains("events"));
        assert_eq!(
            <TestsPanel as Panel>::persistent_name(),
            "Tests",
            "the panel title is a resolved product decision"
        );
    }

    #[test]
    fn rust_workspace_test_explorer_state_migration_is_bounded_and_private() {
        let state = PersistedTestsPanelState {
            version: 0,
            query: "q".repeat(MAX_FILTER_BYTES + 32),
            status: TestStatusFilter::Failed,
            selected_provider: Some("provider".repeat(MAX_FILTER_BYTES)),
            selected_node: Some("node".repeat(MAX_FILTER_BYTES)),
        }
        .sanitized();
        let serialized = serde_json::to_string(&state).expect("state should serialize");

        assert_eq!(state.version, TESTS_PANEL_STATE_VERSION);
        assert_eq!(state.query.len(), MAX_FILTER_BYTES);
        assert!(
            state
                .selected_provider
                .is_some_and(|value| value.len() <= MAX_FILTER_BYTES)
        );
        assert!(
            state
                .selected_node
                .is_some_and(|value| value.len() <= MAX_FILTER_BYTES)
        );
        assert!(!serialized.contains("/Users/"));
        assert!(!serialized.contains("environment"));
        assert!(!serialized.contains("terminal"));
    }

    #[test]
    fn rust_test_actions_expose_stable_delegate_selections() {
        let provider = fake_provider(StructuredProviderStatus::Current);
        let projection = project_test_explorer(&[provider], &TestExplorerFilter::default());
        let failed_node = projection
            .snapshot
            .roots
            .iter()
            .find_map(|node| find_node(std::slice::from_ref(node), "renders login"))
            .expect("failed case should be visible");
        let selection = projection
            .selection(&failed_node.id)
            .expect("an actionable node should expose an opaque provider selection");

        assert_eq!(selection.provider_id.0, "web-tests");
        assert_eq!(selection.discovery_generation, DiscoveryGeneration(3));
        assert_eq!(selection.node_id.0, "renders-login");
        assert_eq!(
            selection.run_id.as_ref().map(|run| run.0.as_str()),
            Some("run-1")
        );
        assert_eq!(projection.failed(), std::slice::from_ref(selection));
    }

    #[gpui::test]
    async fn test_explorer_tree_host_preserves_selection_and_keyboard_behavior(
        cx: &mut TestAppContext,
    ) {
        let projection = project_test_explorer(
            &[fake_provider(StructuredProviderStatus::Current)],
            &TestExplorerFilter::default(),
        );
        let mut host = LanguageToolTreeHost::default();
        host.replace_snapshot(projection.snapshot);
        host.expand_all();
        host.select_first();
        let first = host.selected().cloned();
        host.select_next();
        assert_ne!(host.selected(), first.as_ref());
        host.select_parent();
        host.toggle_selected();
        assert!(host.selected().is_some());
        cx.background_executor
            .timer(std::time::Duration::from_millis(1))
            .await;
    }

    fn find_node<'a>(nodes: &'a [LanguageToolNode], label: &str) -> Option<&'a LanguageToolNode> {
        for node in nodes {
            if node.label == label {
                return Some(node);
            }
            if let Some(found) = find_node(&node.children, label) {
                return Some(found);
            }
        }
        None
    }

    fn collect_ids(nodes: &[LanguageToolNode]) -> Vec<LanguageToolNodeId> {
        let mut ids = Vec::new();
        for node in nodes {
            ids.push(node.id.clone());
            ids.extend(collect_ids(&node.children));
        }
        ids
    }
}
