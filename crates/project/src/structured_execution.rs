use std::collections::{BTreeMap, HashSet, VecDeque};

use anyhow::{Context as _, Result, bail};
use gpui::{AsyncApp, Context, Entity, EventEmitter, Task};
use rpc::{AnyProtoClient, TypedEnvelope, proto};
use task::{StructuredTaskHandle, StructuredTaskLifecycleEvent, StructuredTaskState};

use crate::{ProjectPath, WorktreeId, worktree_store::WorktreeStore};

pub const STRUCTURED_EXECUTION_PROTOCOL_VERSION: u32 = 1;
pub const MAX_STRUCTURED_NODES: usize = 10_000;
pub const MAX_STRUCTURED_EVENTS: usize = 50_000;
pub const MAX_STRUCTURED_MESSAGE_BYTES: usize = 64 * 1024;
pub const MAX_STRUCTURED_COMPLETED_RUNS: usize = 20;
pub const MAX_STRUCTURED_PAGE_SIZE: usize = 256;
pub const MAX_STRUCTURED_EVENT_CHUNK_SIZE: usize = 512;
const MAX_IDENTIFIER_BYTES: usize = 256;
const MAX_LABEL_BYTES: usize = 4 * 1024;
const MAX_DIAGNOSTIC_BYTES: usize = 4 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct StructuredProviderId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct StructuredNodeId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct StructuredRunId(pub String);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct DiscoveryGeneration(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuredNodeKind {
    Provider,
    Suite,
    Group,
    Case,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuredProviderStatus {
    Loading,
    Current,
    Empty,
    Partial,
    Stale,
    Error,
    Restricted,
    Disconnected,
    Mismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredNode {
    pub id: StructuredNodeId,
    pub parent_id: Option<StructuredNodeId>,
    pub label: String,
    pub kind: StructuredNodeKind,
    pub path: Option<ProjectPath>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuredNodeState {
    Queued,
    Running,
    Passed,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuredRunPhase {
    Queued,
    Running,
    Completed,
    Cancelled,
    SpawnError,
}

impl StructuredRunPhase {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::SpawnError)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredExecutionEvent {
    pub sequence: u64,
    pub node_id: StructuredNodeId,
    pub state: StructuredNodeState,
    pub duration_millis: Option<u64>,
    pub message: Option<String>,
    pub location: Option<ProjectPath>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StructuredExecutionSummary {
    pub total: u32,
    pub queued: u32,
    pub running: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub cancelled: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredRun {
    pub id: StructuredRunId,
    pub discovery_generation: DiscoveryGeneration,
    pub phase: StructuredRunPhase,
    pub scope_node_ids: Vec<StructuredNodeId>,
    pub summary: StructuredExecutionSummary,
    pub events: Vec<StructuredExecutionEvent>,
    pub next_sequence: u64,
    pub truncated: bool,
    pub diagnostic: Option<String>,
    message_bytes: usize,
}

impl StructuredRun {
    pub fn new(
        id: StructuredRunId,
        discovery_generation: DiscoveryGeneration,
        scope_node_ids: Vec<StructuredNodeId>,
    ) -> Self {
        Self {
            id,
            discovery_generation,
            phase: StructuredRunPhase::Queued,
            scope_node_ids,
            summary: StructuredExecutionSummary::default(),
            events: Vec::new(),
            next_sequence: 0,
            truncated: false,
            diagnostic: None,
            message_bytes: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredProviderSnapshot {
    pub provider_id: StructuredProviderId,
    pub discovery_generation: DiscoveryGeneration,
    pub status: StructuredProviderStatus,
    pub nodes: Vec<StructuredNode>,
    pub partial: bool,
    pub diagnostic: Option<String>,
    pub current_run: Option<StructuredRun>,
    pub last_complete_run: Option<StructuredRun>,
    pub completed_runs: VecDeque<StructuredRun>,
}

impl StructuredProviderSnapshot {
    pub fn discovery(
        provider_id: StructuredProviderId,
        discovery_generation: DiscoveryGeneration,
        status: StructuredProviderStatus,
        nodes: Vec<StructuredNode>,
    ) -> Self {
        Self {
            provider_id,
            discovery_generation,
            status,
            nodes,
            partial: false,
            diagnostic: None,
            current_run: None,
            last_complete_run: None,
            completed_runs: VecDeque::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuredExecutionLimits {
    pub nodes: usize,
    pub events_per_run: usize,
    pub message_bytes_per_run: usize,
    pub completed_runs: usize,
}

impl Default for StructuredExecutionLimits {
    fn default() -> Self {
        Self {
            nodes: MAX_STRUCTURED_NODES,
            events_per_run: MAX_STRUCTURED_EVENTS,
            message_bytes_per_run: MAX_STRUCTURED_MESSAGE_BYTES,
            completed_runs: MAX_STRUCTURED_COMPLETED_RUNS,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuredApplyOutcome {
    Applied,
    Duplicate,
    Truncated,
}

#[derive(Clone, Debug)]
pub struct StructuredExecutionState {
    project_generation: u64,
    limits: StructuredExecutionLimits,
    providers: BTreeMap<StructuredProviderId, StructuredProviderSnapshot>,
}

impl StructuredExecutionState {
    pub fn new(project_generation: u64) -> Self {
        Self::with_limits(project_generation, StructuredExecutionLimits::default())
    }

    pub fn with_limits(project_generation: u64, limits: StructuredExecutionLimits) -> Self {
        Self {
            project_generation,
            limits,
            providers: BTreeMap::new(),
        }
    }

    pub fn project_generation(&self) -> u64 {
        self.project_generation
    }

    pub fn replace_project_generation(&mut self, project_generation: u64) {
        if self.project_generation != project_generation {
            self.project_generation = project_generation;
            self.providers.clear();
        }
    }

    pub fn providers(&self) -> impl Iterator<Item = &StructuredProviderSnapshot> {
        self.providers.values()
    }

    pub fn provider(
        &self,
        provider_id: &StructuredProviderId,
    ) -> Option<&StructuredProviderSnapshot> {
        self.providers.get(provider_id)
    }

    pub fn apply_discovery(
        &mut self,
        project_generation: u64,
        mut snapshot: StructuredProviderSnapshot,
        visible_worktrees: Option<&HashSet<WorktreeId>>,
    ) -> Result<StructuredApplyOutcome> {
        self.require_project_generation(project_generation)?;
        validate_identifier("provider ID", &snapshot.provider_id.0)?;
        let (nodes, was_filtered, diagnostic) = bounded_nodes(
            std::mem::take(&mut snapshot.nodes),
            visible_worktrees,
            self.limits.nodes,
        )?;
        snapshot.nodes = nodes;
        snapshot.partial |= was_filtered;
        if snapshot.partial && snapshot.status == StructuredProviderStatus::Current {
            snapshot.status = StructuredProviderStatus::Partial;
        }
        if snapshot.diagnostic.is_none() {
            snapshot.diagnostic = diagnostic;
        } else {
            snapshot.diagnostic = snapshot
                .diagnostic
                .map(|value| bounded_text(&value, MAX_DIAGNOSTIC_BYTES));
        }
        if let Some(existing) = self.providers.get(&snapshot.provider_id) {
            if snapshot.discovery_generation < existing.discovery_generation {
                bail!("stale structured discovery generation");
            }
            if snapshot.discovery_generation == existing.discovery_generation {
                if existing.status == snapshot.status
                    && existing.nodes == snapshot.nodes
                    && existing.partial == snapshot.partial
                    && existing.diagnostic == snapshot.diagnostic
                {
                    return Ok(StructuredApplyOutcome::Duplicate);
                }
                bail!("conflicting structured discovery generation");
            }
            snapshot.last_complete_run = existing.last_complete_run.clone();
            snapshot.completed_runs = existing.completed_runs.clone();
        }
        snapshot.current_run = None;
        let outcome = if was_filtered {
            StructuredApplyOutcome::Truncated
        } else {
            StructuredApplyOutcome::Applied
        };
        self.providers
            .insert(snapshot.provider_id.clone(), snapshot);
        Ok(outcome)
    }

    pub fn begin_run(
        &mut self,
        project_generation: u64,
        provider_id: &StructuredProviderId,
        mut run: StructuredRun,
    ) -> Result<StructuredApplyOutcome> {
        self.require_project_generation(project_generation)?;
        validate_identifier("run ID", &run.id.0)?;
        let limits = self.limits;
        let provider = self
            .providers
            .get_mut(provider_id)
            .context("unknown structured execution provider")?;
        if run.discovery_generation != provider.discovery_generation {
            bail!("run uses a stale structured discovery generation");
        }
        let known_nodes = provider
            .nodes
            .iter()
            .map(|node| &node.id)
            .collect::<HashSet<_>>();
        run.scope_node_ids.retain(|id| known_nodes.contains(id));
        run.scope_node_ids.sort();
        run.scope_node_ids.dedup();
        if run.scope_node_ids.is_empty() {
            bail!("structured run has no visible scope nodes");
        }
        if let Some(current) = provider.current_run.as_ref() {
            if current.id == run.id {
                return Ok(StructuredApplyOutcome::Duplicate);
            }
        }
        if let Some(mut superseded) = provider.current_run.take()
            && !superseded.phase.is_terminal()
        {
            superseded.phase = StructuredRunPhase::Cancelled;
            superseded.diagnostic = Some("Superseded by a newer structured run".to_string());
            retain_completed_run(provider, superseded, limits.completed_runs);
        }
        run.events.clear();
        run.next_sequence = 0;
        run.summary = StructuredExecutionSummary::default();
        run.message_bytes = 0;
        provider.current_run = Some(run);
        Ok(StructuredApplyOutcome::Applied)
    }

    pub fn set_run_phase(
        &mut self,
        project_generation: u64,
        provider_id: &StructuredProviderId,
        run_id: &StructuredRunId,
        phase: StructuredRunPhase,
        diagnostic: Option<&str>,
    ) -> Result<StructuredApplyOutcome> {
        self.require_project_generation(project_generation)?;
        let limits = self.limits;
        let provider = self
            .providers
            .get_mut(provider_id)
            .context("unknown structured execution provider")?;
        let run = provider
            .current_run
            .as_mut()
            .filter(|run| &run.id == run_id)
            .context("stale or unknown structured run")?;
        if run.phase == phase {
            return Ok(StructuredApplyOutcome::Duplicate);
        }
        if run.phase.is_terminal() {
            bail!("structured run is already terminal");
        }
        run.phase = phase;
        run.diagnostic = diagnostic.map(|value| bounded_text(value, MAX_DIAGNOSTIC_BYTES));
        if phase.is_terminal() {
            let completed = run.clone();
            if phase == StructuredRunPhase::Completed {
                provider.last_complete_run = Some(completed.clone());
            }
            retain_completed_run(provider, completed, limits.completed_runs);
        }
        Ok(StructuredApplyOutcome::Applied)
    }

    pub fn apply_event(
        &mut self,
        project_generation: u64,
        provider_id: &StructuredProviderId,
        run_id: &StructuredRunId,
        mut event: StructuredExecutionEvent,
        visible_worktrees: Option<&HashSet<WorktreeId>>,
    ) -> Result<StructuredApplyOutcome> {
        self.require_project_generation(project_generation)?;
        validate_identifier("node ID", &event.node_id.0)?;
        let limits = self.limits;
        let provider = self
            .providers
            .get_mut(provider_id)
            .context("unknown structured execution provider")?;
        if !provider.nodes.iter().any(|node| node.id == event.node_id) {
            bail!("structured event references an unknown node");
        }
        let run = provider
            .current_run
            .as_mut()
            .filter(|run| &run.id == run_id)
            .context("stale or unknown structured run")?;
        if run.discovery_generation != provider.discovery_generation {
            bail!("structured event uses a stale discovery generation");
        }
        if event.sequence < run.next_sequence {
            return if run.events.iter().any(|existing| existing == &event) {
                Ok(StructuredApplyOutcome::Duplicate)
            } else {
                bail!("conflicting duplicate structured event")
            };
        }
        if event.sequence > run.next_sequence {
            bail!("structured event sequence gap");
        }
        run.next_sequence = run.next_sequence.saturating_add(1);
        if event
            .location
            .as_ref()
            .is_some_and(|path| !path_is_visible(path, visible_worktrees))
        {
            run.truncated = true;
            return Ok(StructuredApplyOutcome::Truncated);
        }
        if let Some(message) = event.message.take() {
            let original_message_bytes = message.len();
            let remaining = limits
                .message_bytes_per_run
                .saturating_sub(run.message_bytes);
            if remaining == 0 {
                run.truncated = true;
            } else {
                let message = bounded_text(&message, remaining.min(MAX_DIAGNOSTIC_BYTES));
                run.message_bytes = run.message_bytes.saturating_add(message.len());
                if message.len() < original_message_bytes
                    || run.message_bytes >= limits.message_bytes_per_run
                {
                    run.truncated = true;
                }
                event.message = Some(message);
            }
        }
        if run.events.len() >= limits.events_per_run {
            run.truncated = true;
            return Ok(StructuredApplyOutcome::Truncated);
        }
        run.events.push(event);
        run.summary = summarize_events(&run.events);
        Ok(if run.truncated {
            StructuredApplyOutcome::Truncated
        } else {
            StructuredApplyOutcome::Applied
        })
    }

    fn require_project_generation(&self, project_generation: u64) -> Result<()> {
        if project_generation != self.project_generation {
            bail!("stale or cross-project structured execution generation");
        }
        Ok(())
    }

    fn replace_remote_snapshot(
        &mut self,
        project_generation: u64,
        snapshot: StructuredProviderSnapshot,
    ) {
        self.replace_project_generation(project_generation);
        self.providers
            .insert(snapshot.provider_id.clone(), snapshot);
    }
}

fn retain_completed_run(
    provider: &mut StructuredProviderSnapshot,
    run: StructuredRun,
    retention: usize,
) {
    provider.completed_runs.push_back(run);
    while provider.completed_runs.len() > retention {
        provider.completed_runs.pop_front();
    }
}

fn summarize_events(events: &[StructuredExecutionEvent]) -> StructuredExecutionSummary {
    let mut latest = BTreeMap::<&StructuredNodeId, StructuredNodeState>::new();
    for event in events {
        latest.insert(&event.node_id, event.state);
    }
    let mut summary = StructuredExecutionSummary {
        total: latest.len().try_into().unwrap_or(u32::MAX),
        ..StructuredExecutionSummary::default()
    };
    for state in latest.into_values() {
        match state {
            StructuredNodeState::Queued => summary.queued = summary.queued.saturating_add(1),
            StructuredNodeState::Running => summary.running = summary.running.saturating_add(1),
            StructuredNodeState::Passed => summary.passed = summary.passed.saturating_add(1),
            StructuredNodeState::Failed => summary.failed = summary.failed.saturating_add(1),
            StructuredNodeState::Skipped => summary.skipped = summary.skipped.saturating_add(1),
            StructuredNodeState::Cancelled => {
                summary.cancelled = summary.cancelled.saturating_add(1)
            }
        }
    }
    summary
}

fn bounded_nodes(
    nodes: Vec<StructuredNode>,
    visible_worktrees: Option<&HashSet<WorktreeId>>,
    limit: usize,
) -> Result<(Vec<StructuredNode>, bool, Option<String>)> {
    let mut result = Vec::new();
    let mut retained_ids = HashSet::new();
    let mut seen_ids = HashSet::new();
    let mut partial = false;
    let mut diagnostic = None;
    for mut node in nodes {
        validate_identifier("node ID", &node.id.0)?;
        if !seen_ids.insert(node.id.clone()) {
            bail!("duplicate structured node ID");
        }
        node.label = bounded_text(&node.label, MAX_LABEL_BYTES);
        let parent_visible = node
            .parent_id
            .as_ref()
            .is_none_or(|parent| retained_ids.contains(parent));
        let path_visible = node
            .path
            .as_ref()
            .is_none_or(|path| path_is_visible(path, visible_worktrees));
        if !parent_visible || !path_visible {
            partial = true;
            diagnostic.get_or_insert_with(|| {
                "Some structured result nodes were filtered or had missing parents".to_string()
            });
            continue;
        }
        if result.len() >= limit {
            partial = true;
            diagnostic.get_or_insert_with(|| {
                format!("Structured result nodes were truncated at {limit} entries")
            });
            continue;
        }
        retained_ids.insert(node.id.clone());
        result.push(node);
    }
    Ok((result, partial, diagnostic))
}

fn filter_nodes(
    nodes: &[StructuredNode],
    visible_worktrees: Option<&HashSet<WorktreeId>>,
) -> Result<Vec<StructuredNode>> {
    bounded_nodes(nodes.to_vec(), visible_worktrees, MAX_STRUCTURED_NODES)
        .map(|(nodes, _, _)| nodes)
}

fn path_is_visible(path: &ProjectPath, visible_worktrees: Option<&HashSet<WorktreeId>>) -> bool {
    visible_worktrees.is_none_or(|worktrees| worktrees.contains(&path.worktree_id))
}

fn validate_identifier(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > MAX_IDENTIFIER_BYTES {
        bail!("{label} is empty or exceeds {MAX_IDENTIFIER_BYTES} bytes");
    }
    Ok(())
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes.min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

#[derive(Clone, Debug)]
pub enum StructuredExecutionStoreEvent {
    Changed(StructuredProviderId),
    ProjectGenerationChanged(u64),
}

impl EventEmitter<StructuredExecutionStoreEvent> for StructuredExecutionStore {}

enum StructuredExecutionStoreMode {
    Local {
        shared: Option<(u64, AnyProtoClient)>,
    },
    Remote {
        project_id: u64,
        client: AnyProtoClient,
    },
}

pub struct StructuredExecutionStore {
    mode: StructuredExecutionStoreMode,
    worktree_store: Entity<WorktreeStore>,
    state: StructuredExecutionState,
}

impl StructuredExecutionStore {
    pub fn init(client: &AnyProtoClient) {
        client.add_entity_request_handler(Self::handle_get_snapshot);
        client.add_entity_request_handler(Self::handle_get_events);
    }

    pub fn local(worktree_store: Entity<WorktreeStore>, project_generation: u64) -> Self {
        Self {
            mode: StructuredExecutionStoreMode::Local { shared: None },
            worktree_store,
            state: StructuredExecutionState::new(project_generation),
        }
    }

    pub fn remote(
        project_id: u64,
        client: AnyProtoClient,
        worktree_store: Entity<WorktreeStore>,
    ) -> Self {
        Self {
            mode: StructuredExecutionStoreMode::Remote { project_id, client },
            worktree_store,
            state: StructuredExecutionState::new(0),
        }
    }

    pub fn state(&self) -> &StructuredExecutionState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut StructuredExecutionState {
        &mut self.state
    }

    pub fn shared(&mut self, project_id: u64, client: AnyProtoClient) {
        if let StructuredExecutionStoreMode::Local { shared } = &mut self.mode {
            *shared = Some((project_id, client));
        }
    }

    pub fn unshared(&mut self) {
        if let StructuredExecutionStoreMode::Local { shared } = &mut self.mode {
            *shared = None;
        }
    }

    pub fn observe_task_handle(
        &mut self,
        provider_id: StructuredProviderId,
        run_id: StructuredRunId,
        handle: &StructuredTaskHandle,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if !matches!(self.mode, StructuredExecutionStoreMode::Local { .. }) {
            bail!("structured task lifecycle must be observed by the authoritative project host");
        }
        let project_generation = self.state.project_generation();
        let store = cx.entity().downgrade();
        handle.subscribe(cx, move |event, cx| {
            let update = store.update(cx, |store, cx| {
                match apply_task_lifecycle(
                    &mut store.state,
                    project_generation,
                    &provider_id,
                    &run_id,
                    event,
                ) {
                    Ok(_) => cx.emit(StructuredExecutionStoreEvent::Changed(provider_id.clone())),
                    Err(error) => log::warn!("Rejected structured task lifecycle event: {error:#}"),
                }
            });
            if let Err(error) = update {
                log::debug!("Structured execution store was dropped: {error:#}");
            }
        });
        Ok(())
    }

    pub fn apply_task_lifecycle(
        &mut self,
        provider_id: &StructuredProviderId,
        run_id: &StructuredRunId,
        event: &StructuredTaskLifecycleEvent,
        cx: &mut Context<Self>,
    ) -> Result<StructuredApplyOutcome> {
        if !matches!(self.mode, StructuredExecutionStoreMode::Local { .. }) {
            bail!("structured task lifecycle must be applied by the authoritative project host");
        }
        let project_generation = self.state.project_generation();
        let outcome = apply_task_lifecycle(
            &mut self.state,
            project_generation,
            provider_id,
            run_id,
            event,
        )?;
        cx.emit(StructuredExecutionStoreEvent::Changed(provider_id.clone()));
        Ok(outcome)
    }

    pub fn refresh_provider(
        &mut self,
        provider_id: StructuredProviderId,
        cx: &mut Context<Self>,
    ) -> Task<Result<StructuredProviderSnapshot>> {
        match &self.mode {
            StructuredExecutionStoreMode::Local { .. } => Task::ready(
                self.state
                    .provider(&provider_id)
                    .cloned()
                    .context("unknown structured execution provider"),
            ),
            StructuredExecutionStoreMode::Remote { project_id, client } => {
                let project_id = *project_id;
                let client = client.clone();
                let worktree_ids = self
                    .worktree_store
                    .read(cx)
                    .visible_worktrees(cx)
                    .map(|worktree| worktree.read(cx).id().to_proto())
                    .collect::<Vec<_>>();
                cx.spawn(async move |this, cx| {
                    let (project_generation, snapshot) =
                        fetch_remote_provider(project_id, client, provider_id, worktree_ids)
                            .await?;
                    this.update(cx, |store, cx| {
                        store
                            .state
                            .replace_remote_snapshot(project_generation, snapshot.clone());
                        cx.emit(StructuredExecutionStoreEvent::Changed(
                            snapshot.provider_id.clone(),
                        ));
                    })?;
                    Ok(snapshot)
                })
            }
        }
    }

    async fn handle_get_snapshot(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::GetStructuredExecutionSnapshot>,
        cx: AsyncApp,
    ) -> Result<proto::StructuredExecutionSnapshotPage> {
        let request = envelope.payload;
        let allowed_worktrees = request
            .worktree_ids
            .into_iter()
            .map(WorktreeId::from_proto)
            .collect::<HashSet<_>>();
        this.read_with(&cx, |store, _| {
            snapshot_page_to_proto(
                &store.state,
                &StructuredProviderId(request.provider_id),
                DiscoveryGeneration(request.discovery_generation),
                request.page_start as usize,
                request.page_size as usize,
                Some(&allowed_worktrees),
            )
        })
    }

    async fn handle_get_events(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::GetStructuredExecutionEvents>,
        cx: AsyncApp,
    ) -> Result<proto::StructuredExecutionEventChunk> {
        let request = envelope.payload;
        let allowed_worktrees = request
            .worktree_ids
            .into_iter()
            .map(WorktreeId::from_proto)
            .collect::<HashSet<_>>();
        this.read_with(&cx, |store, _| {
            event_chunk_to_proto(
                &store.state,
                &StructuredProviderId(request.provider_id),
                &StructuredRunId(request.run_id),
                request.start_sequence,
                request.chunk_size as usize,
                Some(&allowed_worktrees),
            )
        })
    }
}

fn apply_task_lifecycle(
    state: &mut StructuredExecutionState,
    project_generation: u64,
    provider_id: &StructuredProviderId,
    run_id: &StructuredRunId,
    event: &StructuredTaskLifecycleEvent,
) -> Result<StructuredApplyOutcome> {
    let single_case = state.provider(provider_id).and_then(|provider| {
        let run = provider
            .current_run
            .as_ref()
            .filter(|run| &run.id == run_id)?;
        let node_id = run
            .scope_node_ids
            .first()
            .filter(|_| run.scope_node_ids.len() == 1)?;
        provider
            .nodes
            .iter()
            .find(|node| &node.id == node_id && node.kind == StructuredNodeKind::Case)
            .map(|node| (node.id.clone(), run.next_sequence))
    });
    let (phase, diagnostic) = match &event.state {
        StructuredTaskState::Queued => (StructuredRunPhase::Queued, None),
        StructuredTaskState::Running { .. } => (StructuredRunPhase::Running, None),
        StructuredTaskState::Completed {
            exit_code, success, ..
        } => (
            StructuredRunPhase::Completed,
            (!success).then(|| match exit_code {
                Some(exit_code) => format!("Task exited with status {exit_code}"),
                None => "Task exited unsuccessfully without a status code".to_string(),
            }),
        ),
        StructuredTaskState::SpawnError { message } => {
            (StructuredRunPhase::SpawnError, Some(message.clone()))
        }
        StructuredTaskState::Cancelled {
            termination_confirmed,
            ..
        } => (
            StructuredRunPhase::Cancelled,
            (!termination_confirmed)
                .then(|| "Cancellation requested; host termination was not confirmed".to_string()),
        ),
    };
    let single_case_event = single_case.map(|(node_id, sequence)| {
        let node_state = match &event.state {
            StructuredTaskState::Queued => StructuredNodeState::Queued,
            StructuredTaskState::Running { .. } => StructuredNodeState::Running,
            StructuredTaskState::Completed { success: true, .. } => StructuredNodeState::Passed,
            StructuredTaskState::Completed { success: false, .. }
            | StructuredTaskState::SpawnError { .. } => StructuredNodeState::Failed,
            StructuredTaskState::Cancelled { .. } => StructuredNodeState::Cancelled,
        };
        StructuredExecutionEvent {
            sequence,
            node_id,
            state: node_state,
            duration_millis: None,
            message: diagnostic.clone(),
            location: None,
        }
    });
    if phase.is_terminal() {
        let current_phase = state
            .provider(provider_id)
            .and_then(|provider| provider.current_run.as_ref())
            .filter(|run| &run.id == run_id)
            .map(|run| run.phase)
            .context("stale or unknown structured run")?;
        if current_phase.is_terminal() {
            bail!("structured run is already terminal");
        }
        if let Some(single_case_event) = single_case_event {
            state.apply_event(
                project_generation,
                provider_id,
                run_id,
                single_case_event,
                None,
            )?;
        }
        return state.set_run_phase(
            project_generation,
            provider_id,
            run_id,
            phase,
            diagnostic.as_deref(),
        );
    }
    let outcome = state.set_run_phase(
        project_generation,
        provider_id,
        run_id,
        phase,
        diagnostic.as_deref(),
    )?;
    if let Some(single_case_event) = single_case_event {
        state.apply_event(
            project_generation,
            provider_id,
            run_id,
            single_case_event,
            None,
        )?;
    }
    Ok(outcome)
}

fn snapshot_page_to_proto(
    state: &StructuredExecutionState,
    provider_id: &StructuredProviderId,
    requested_generation: DiscoveryGeneration,
    page_start: usize,
    requested_page_size: usize,
    visible_worktrees: Option<&HashSet<WorktreeId>>,
) -> Result<proto::StructuredExecutionSnapshotPage> {
    let provider = state
        .provider(provider_id)
        .context("unknown structured execution provider")?;
    if requested_generation.0 != 0 && requested_generation != provider.discovery_generation {
        bail!("stale structured discovery generation");
    }
    let nodes = filter_nodes(&provider.nodes, visible_worktrees)?;
    let page_size = requested_page_size.clamp(1, MAX_STRUCTURED_PAGE_SIZE);
    let page_end = page_start.saturating_add(page_size).min(nodes.len());
    let page = nodes
        .get(page_start..page_end)
        .unwrap_or_default()
        .iter()
        .map(node_to_proto)
        .collect();
    Ok(proto::StructuredExecutionSnapshotPage {
        protocol_version: STRUCTURED_EXECUTION_PROTOCOL_VERSION,
        project_generation: state.project_generation(),
        provider_id: provider.provider_id.0.clone(),
        discovery_generation: provider.discovery_generation.0,
        status: provider_status_to_proto(provider.status),
        nodes: page,
        next_page_start: if page_end < nodes.len() {
            page_end.try_into().unwrap_or(u32::MAX)
        } else {
            0
        },
        partial: provider.partial || nodes.len() != provider.nodes.len(),
        diagnostic: provider
            .diagnostic
            .as_deref()
            .map(|value| bounded_text(value, MAX_DIAGNOSTIC_BYTES)),
        current_run: provider
            .current_run
            .as_ref()
            .map(|run| run_to_proto(run, &nodes)),
        last_complete_run: provider
            .last_complete_run
            .as_ref()
            .map(|run| run_to_proto(run, &nodes)),
    })
}

fn event_chunk_to_proto(
    state: &StructuredExecutionState,
    provider_id: &StructuredProviderId,
    run_id: &StructuredRunId,
    start_sequence: u64,
    requested_chunk_size: usize,
    visible_worktrees: Option<&HashSet<WorktreeId>>,
) -> Result<proto::StructuredExecutionEventChunk> {
    let provider = state
        .provider(provider_id)
        .context("unknown structured execution provider")?;
    let run = provider
        .current_run
        .iter()
        .chain(provider.last_complete_run.iter())
        .chain(provider.completed_runs.iter())
        .find(|run| &run.id == run_id)
        .context("unknown structured run")?;
    let visible_nodes = filter_nodes(&provider.nodes, visible_worktrees)?
        .into_iter()
        .map(|node| node.id)
        .collect::<HashSet<_>>();
    let chunk_size = requested_chunk_size.clamp(1, MAX_STRUCTURED_EVENT_CHUNK_SIZE);
    let mut events = Vec::new();
    let mut next_sequence = start_sequence;
    for event in run
        .events
        .iter()
        .filter(|event| event.sequence >= start_sequence)
    {
        next_sequence = event.sequence.saturating_add(1);
        if visible_nodes.contains(&event.node_id)
            && event
                .location
                .as_ref()
                .is_none_or(|path| path_is_visible(path, visible_worktrees))
        {
            events.push(event_to_proto(event));
            if events.len() >= chunk_size {
                break;
            }
        }
    }
    let complete = next_sequence >= run.next_sequence;
    Ok(proto::StructuredExecutionEventChunk {
        protocol_version: STRUCTURED_EXECUTION_PROTOCOL_VERSION,
        project_generation: state.project_generation(),
        provider_id: provider.provider_id.0.clone(),
        run_id: run.id.0.clone(),
        discovery_generation: run.discovery_generation.0,
        events,
        next_sequence,
        complete,
        truncated: run.truncated,
    })
}

async fn fetch_remote_provider(
    project_id: u64,
    client: AnyProtoClient,
    provider_id: StructuredProviderId,
    worktree_ids: Vec<u64>,
) -> Result<(u64, StructuredProviderSnapshot)> {
    let mut page_start = 0;
    let mut nodes = Vec::new();
    let mut first_page = None;
    loop {
        let page = client
            .request(proto::GetStructuredExecutionSnapshot {
                project_id,
                provider_id: provider_id.0.clone(),
                discovery_generation: 0,
                page_start,
                page_size: MAX_STRUCTURED_PAGE_SIZE as u32,
                worktree_ids: worktree_ids.clone(),
            })
            .await?;
        validate_protocol(page.protocol_version)?;
        if page.provider_id != provider_id.0 {
            bail!("structured provider response identity mismatch");
        }
        if let Some(first) = first_page.as_ref() {
            let first: &proto::StructuredExecutionSnapshotPage = first;
            if first.project_generation != page.project_generation
                || first.discovery_generation != page.discovery_generation
            {
                bail!("structured provider page generation mismatch");
            }
        } else {
            first_page = Some(page.clone());
        }
        nodes.extend(
            page.nodes
                .into_iter()
                .map(node_from_proto)
                .collect::<Result<Vec<_>>>()?,
        );
        if page.next_page_start == 0 {
            break;
        }
        if page.next_page_start <= page_start {
            bail!("structured provider page did not advance");
        }
        page_start = page.next_page_start;
    }
    let first = first_page.context("structured provider returned no page")?;
    if nodes.len() > MAX_STRUCTURED_NODES {
        bail!("structured provider exceeded the node limit");
    }
    let mut snapshot = StructuredProviderSnapshot {
        provider_id,
        discovery_generation: DiscoveryGeneration(first.discovery_generation),
        status: provider_status_from_proto(first.status)?,
        nodes,
        partial: first.partial,
        diagnostic: first
            .diagnostic
            .map(|value| bounded_text(&value, MAX_DIAGNOSTIC_BYTES)),
        current_run: first.current_run.map(run_from_proto).transpose()?,
        last_complete_run: first.last_complete_run.map(run_from_proto).transpose()?,
        completed_runs: VecDeque::new(),
    };
    if let Some(run) = snapshot.current_run.as_mut() {
        run.events = fetch_remote_events(
            project_id,
            &client,
            &snapshot.provider_id,
            run,
            &worktree_ids,
        )
        .await?;
        run.next_sequence = run
            .events
            .last()
            .map_or(0, |event| event.sequence.saturating_add(1));
        run.summary = summarize_events(&run.events);
    }
    if let Some(run) = snapshot.last_complete_run.as_mut() {
        run.events = fetch_remote_events(
            project_id,
            &client,
            &snapshot.provider_id,
            run,
            &worktree_ids,
        )
        .await?;
        run.next_sequence = run
            .events
            .last()
            .map_or(0, |event| event.sequence.saturating_add(1));
        run.summary = summarize_events(&run.events);
    }
    Ok((first.project_generation, snapshot))
}

async fn fetch_remote_events(
    project_id: u64,
    client: &AnyProtoClient,
    provider_id: &StructuredProviderId,
    run: &StructuredRun,
    worktree_ids: &[u64],
) -> Result<Vec<StructuredExecutionEvent>> {
    let mut start_sequence = 0;
    let mut events = Vec::new();
    loop {
        let chunk = client
            .request(proto::GetStructuredExecutionEvents {
                project_id,
                provider_id: provider_id.0.clone(),
                run_id: run.id.0.clone(),
                start_sequence,
                chunk_size: MAX_STRUCTURED_EVENT_CHUNK_SIZE as u32,
                worktree_ids: worktree_ids.to_vec(),
            })
            .await?;
        validate_protocol(chunk.protocol_version)?;
        if chunk.provider_id != provider_id.0 || chunk.run_id != run.id.0 {
            bail!("structured event response identity mismatch");
        }
        events.extend(
            chunk
                .events
                .into_iter()
                .map(event_from_proto)
                .collect::<Result<Vec<_>>>()?,
        );
        if events.len() > MAX_STRUCTURED_EVENTS {
            bail!("structured event response exceeded the event limit");
        }
        if chunk.complete {
            break;
        }
        if chunk.next_sequence <= start_sequence {
            bail!("structured event chunk did not advance");
        }
        start_sequence = chunk.next_sequence;
    }
    Ok(events)
}

fn validate_protocol(version: u32) -> Result<()> {
    if version != STRUCTURED_EXECUTION_PROTOCOL_VERSION {
        bail!("unsupported structured execution protocol version {version}");
    }
    Ok(())
}

fn node_to_proto(node: &StructuredNode) -> proto::StructuredExecutionNode {
    proto::StructuredExecutionNode {
        node_id: node.id.0.clone(),
        parent_id: node.parent_id.as_ref().map(|id| id.0.clone()),
        label: bounded_text(&node.label, MAX_LABEL_BYTES),
        kind: node_kind_to_proto(node.kind),
        path: node.path.as_ref().map(ProjectPath::to_proto),
    }
}

fn node_from_proto(node: proto::StructuredExecutionNode) -> Result<StructuredNode> {
    validate_identifier("node ID", &node.node_id)?;
    let path = node
        .path
        .map(|path| ProjectPath::from_proto(path).context("invalid structured node path"))
        .transpose()?;
    Ok(StructuredNode {
        id: StructuredNodeId(node.node_id),
        parent_id: node.parent_id.map(StructuredNodeId),
        label: bounded_text(&node.label, MAX_LABEL_BYTES),
        kind: node_kind_from_proto(node.kind)?,
        path,
    })
}

fn run_to_proto(
    run: &StructuredRun,
    visible_nodes: &[StructuredNode],
) -> proto::StructuredExecutionRun {
    let visible_ids = visible_nodes
        .iter()
        .map(|node| &node.id)
        .collect::<HashSet<_>>();
    let visible_events = run
        .events
        .iter()
        .filter(|event| visible_ids.contains(&event.node_id))
        .cloned()
        .collect::<Vec<_>>();
    proto::StructuredExecutionRun {
        run_id: run.id.0.clone(),
        discovery_generation: run.discovery_generation.0,
        phase: run_phase_to_proto(run.phase),
        scope_node_ids: run
            .scope_node_ids
            .iter()
            .filter(|id| visible_ids.contains(id))
            .map(|id| id.0.clone())
            .collect(),
        summary: Some(summarize_events(&visible_events).into()),
        truncated: run.truncated || visible_events.len() != run.events.len(),
        diagnostic: run
            .diagnostic
            .as_deref()
            .map(|value| bounded_text(value, MAX_DIAGNOSTIC_BYTES)),
    }
}

fn run_from_proto(run: proto::StructuredExecutionRun) -> Result<StructuredRun> {
    validate_identifier("run ID", &run.run_id)?;
    for node_id in &run.scope_node_ids {
        validate_identifier("scope node ID", node_id)?;
    }
    let mut result = StructuredRun::new(
        StructuredRunId(run.run_id),
        DiscoveryGeneration(run.discovery_generation),
        run.scope_node_ids
            .into_iter()
            .map(StructuredNodeId)
            .collect(),
    );
    result.phase = run_phase_from_proto(run.phase)?;
    result.summary = run.summary.map(Into::into).unwrap_or_default();
    result.truncated = run.truncated;
    result.diagnostic = run
        .diagnostic
        .map(|value| bounded_text(&value, MAX_DIAGNOSTIC_BYTES));
    Ok(result)
}

fn event_to_proto(event: &StructuredExecutionEvent) -> proto::StructuredExecutionEvent {
    proto::StructuredExecutionEvent {
        sequence: event.sequence,
        node_id: event.node_id.0.clone(),
        state: node_state_to_proto(event.state),
        duration_millis: event.duration_millis,
        message: event
            .message
            .as_deref()
            .map(|value| bounded_text(value, MAX_DIAGNOSTIC_BYTES)),
        location: event.location.as_ref().map(ProjectPath::to_proto),
    }
}

fn event_from_proto(event: proto::StructuredExecutionEvent) -> Result<StructuredExecutionEvent> {
    validate_identifier("node ID", &event.node_id)?;
    let location = event
        .location
        .map(|path| ProjectPath::from_proto(path).context("invalid structured event path"))
        .transpose()?;
    Ok(StructuredExecutionEvent {
        sequence: event.sequence,
        node_id: StructuredNodeId(event.node_id),
        state: node_state_from_proto(event.state)?,
        duration_millis: event.duration_millis,
        message: event
            .message
            .map(|value| bounded_text(&value, MAX_DIAGNOSTIC_BYTES)),
        location,
    })
}

impl From<StructuredExecutionSummary> for proto::StructuredExecutionSummary {
    fn from(summary: StructuredExecutionSummary) -> Self {
        Self {
            total: summary.total,
            queued: summary.queued,
            running: summary.running,
            passed: summary.passed,
            failed: summary.failed,
            skipped: summary.skipped,
            cancelled: summary.cancelled,
        }
    }
}

impl From<proto::StructuredExecutionSummary> for StructuredExecutionSummary {
    fn from(summary: proto::StructuredExecutionSummary) -> Self {
        Self {
            total: summary.total,
            queued: summary.queued,
            running: summary.running,
            passed: summary.passed,
            failed: summary.failed,
            skipped: summary.skipped,
            cancelled: summary.cancelled,
        }
    }
}

fn node_kind_to_proto(kind: StructuredNodeKind) -> i32 {
    match kind {
        StructuredNodeKind::Provider => 1,
        StructuredNodeKind::Suite => 2,
        StructuredNodeKind::Group => 3,
        StructuredNodeKind::Case => 4,
    }
}

fn node_kind_from_proto(kind: i32) -> Result<StructuredNodeKind> {
    match kind {
        1 => Ok(StructuredNodeKind::Provider),
        2 => Ok(StructuredNodeKind::Suite),
        3 => Ok(StructuredNodeKind::Group),
        4 => Ok(StructuredNodeKind::Case),
        _ => bail!("invalid structured node kind {kind}"),
    }
}

fn provider_status_to_proto(status: StructuredProviderStatus) -> i32 {
    match status {
        StructuredProviderStatus::Loading => 1,
        StructuredProviderStatus::Current => 2,
        StructuredProviderStatus::Empty => 3,
        StructuredProviderStatus::Partial => 4,
        StructuredProviderStatus::Stale => 5,
        StructuredProviderStatus::Error => 6,
        StructuredProviderStatus::Restricted => 7,
        StructuredProviderStatus::Disconnected => 8,
        StructuredProviderStatus::Mismatch => 9,
    }
}

fn provider_status_from_proto(status: i32) -> Result<StructuredProviderStatus> {
    match status {
        1 => Ok(StructuredProviderStatus::Loading),
        2 => Ok(StructuredProviderStatus::Current),
        3 => Ok(StructuredProviderStatus::Empty),
        4 => Ok(StructuredProviderStatus::Partial),
        5 => Ok(StructuredProviderStatus::Stale),
        6 => Ok(StructuredProviderStatus::Error),
        7 => Ok(StructuredProviderStatus::Restricted),
        8 => Ok(StructuredProviderStatus::Disconnected),
        9 => Ok(StructuredProviderStatus::Mismatch),
        _ => bail!("invalid structured provider status {status}"),
    }
}

fn run_phase_to_proto(phase: StructuredRunPhase) -> i32 {
    match phase {
        StructuredRunPhase::Queued => 1,
        StructuredRunPhase::Running => 2,
        StructuredRunPhase::Completed => 3,
        StructuredRunPhase::Cancelled => 4,
        StructuredRunPhase::SpawnError => 5,
    }
}

fn run_phase_from_proto(phase: i32) -> Result<StructuredRunPhase> {
    match phase {
        1 => Ok(StructuredRunPhase::Queued),
        2 => Ok(StructuredRunPhase::Running),
        3 => Ok(StructuredRunPhase::Completed),
        4 => Ok(StructuredRunPhase::Cancelled),
        5 => Ok(StructuredRunPhase::SpawnError),
        _ => bail!("invalid structured run phase {phase}"),
    }
}

fn node_state_to_proto(state: StructuredNodeState) -> i32 {
    match state {
        StructuredNodeState::Queued => 1,
        StructuredNodeState::Running => 2,
        StructuredNodeState::Passed => 3,
        StructuredNodeState::Failed => 4,
        StructuredNodeState::Skipped => 5,
        StructuredNodeState::Cancelled => 6,
    }
}

fn node_state_from_proto(state: i32) -> Result<StructuredNodeState> {
    match state {
        1 => Ok(StructuredNodeState::Queued),
        2 => Ok(StructuredNodeState::Running),
        3 => Ok(StructuredNodeState::Passed),
        4 => Ok(StructuredNodeState::Failed),
        5 => Ok(StructuredNodeState::Skipped),
        6 => Ok(StructuredNodeState::Cancelled),
        _ => bail!("invalid structured node state {state}"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use util::rel_path::RelPath;

    use super::*;

    fn project_path(worktree: u64, path: &'static str) -> ProjectPath {
        ProjectPath {
            worktree_id: WorktreeId::from_proto(worktree),
            path: Arc::from(RelPath::from_unix_str(path).expect("fixture path should be relative")),
        }
    }

    fn fake_provider(generation: u64) -> StructuredProviderSnapshot {
        let provider = StructuredNodeId("js-provider".to_string());
        let suite = StructuredNodeId("web-suite".to_string());
        StructuredProviderSnapshot::discovery(
            StructuredProviderId("web-tests".to_string()),
            DiscoveryGeneration(generation),
            StructuredProviderStatus::Current,
            vec![
                StructuredNode {
                    id: provider.clone(),
                    parent_id: None,
                    label: "Web tests".to_string(),
                    kind: StructuredNodeKind::Provider,
                    path: None,
                },
                StructuredNode {
                    id: suite.clone(),
                    parent_id: Some(provider),
                    label: "Browser suite".to_string(),
                    kind: StructuredNodeKind::Suite,
                    path: Some(project_path(1, "web/tests.js")),
                },
                StructuredNode {
                    id: StructuredNodeId("case-a".to_string()),
                    parent_id: Some(suite.clone()),
                    label: "renders a page".to_string(),
                    kind: StructuredNodeKind::Case,
                    path: Some(project_path(1, "web/tests.js")),
                },
                StructuredNode {
                    id: StructuredNodeId("private-case".to_string()),
                    parent_id: Some(suite),
                    label: "private case".to_string(),
                    kind: StructuredNodeKind::Case,
                    path: Some(project_path(2, "private/tests.js")),
                },
            ],
        )
    }

    #[test]
    fn structured_execution_state_rejects_duplicates_gaps_and_stale_generations() {
        let mut state = StructuredExecutionState::new(11);
        let provider = fake_provider(4);
        let provider_id = provider.provider_id.clone();
        assert_eq!(
            state
                .apply_discovery(11, provider.clone(), None)
                .expect("discovery should apply"),
            StructuredApplyOutcome::Applied
        );
        assert_eq!(
            state
                .apply_discovery(11, provider, None)
                .expect("identical generation should be idempotent"),
            StructuredApplyOutcome::Duplicate
        );
        let run_id = StructuredRunId("run-1".to_string());
        state
            .begin_run(
                11,
                &provider_id,
                StructuredRun::new(
                    run_id.clone(),
                    DiscoveryGeneration(4),
                    vec![StructuredNodeId("case-a".to_string())],
                ),
            )
            .expect("run should begin");
        let event = StructuredExecutionEvent {
            sequence: 0,
            node_id: StructuredNodeId("case-a".to_string()),
            state: StructuredNodeState::Passed,
            duration_millis: Some(8),
            message: Some("safe failure summary".to_string()),
            location: Some(project_path(1, "web/tests.js")),
        };
        assert_eq!(
            state
                .apply_event(11, &provider_id, &run_id, event.clone(), None)
                .expect("event should apply"),
            StructuredApplyOutcome::Applied
        );
        assert_eq!(
            state
                .apply_event(11, &provider_id, &run_id, event, None)
                .expect("duplicate should be idempotent"),
            StructuredApplyOutcome::Duplicate
        );
        let gap = StructuredExecutionEvent {
            sequence: 2,
            node_id: StructuredNodeId("case-a".to_string()),
            state: StructuredNodeState::Failed,
            duration_millis: None,
            message: None,
            location: None,
        };
        assert!(
            state
                .apply_event(11, &provider_id, &run_id, gap, None)
                .expect_err("sequence gaps must fail")
                .to_string()
                .contains("gap")
        );
        assert!(
            state
                .apply_discovery(10, fake_provider(5), None)
                .expect_err("cross-project generation must fail")
                .to_string()
                .contains("cross-project")
        );
        assert!(
            state
                .apply_discovery(11, fake_provider(3), None)
                .expect_err("stale discovery must fail")
                .to_string()
                .contains("stale")
        );
        let newer_run_id = StructuredRunId("run-2".to_string());
        state
            .begin_run(
                11,
                &provider_id,
                StructuredRun::new(
                    newer_run_id,
                    DiscoveryGeneration(4),
                    vec![StructuredNodeId("case-a".to_string())],
                ),
            )
            .expect("newer run should supersede the active run");
        let late_event = StructuredExecutionEvent {
            sequence: 1,
            node_id: StructuredNodeId("case-a".to_string()),
            state: StructuredNodeState::Passed,
            duration_millis: None,
            message: None,
            location: None,
        };
        assert!(
            state
                .apply_event(11, &provider_id, &run_id, late_event, None)
                .expect_err("events from superseded runs must fail")
                .to_string()
                .contains("stale")
        );
    }

    #[test]
    fn structured_execution_contract_remains_ecosystem_neutral() {
        let source = include_str!("structured_execution.rs")
            .split_once("#[cfg(test)]")
            .map(|(production_source, _)| production_source)
            .expect("structured execution tests should follow production code");
        for forbidden in [
            concat!("cargo_", "metadata"),
            concat!("Cargo", "Workspace"),
            concat!("terminal_", "bytes"),
            concat!("project_", "env"),
        ] {
            assert!(
                !source.contains(forbidden),
                "generic structured execution source contains forbidden term {forbidden}"
            );
        }
    }

    #[test]
    fn structured_task_bridge_maps_lifecycle_and_rejects_late_events() {
        let mut state = StructuredExecutionState::new(21);
        let provider = fake_provider(8);
        let provider_id = provider.provider_id.clone();
        state
            .apply_discovery(21, provider, None)
            .expect("provider discovery should apply");
        let run_id = StructuredRunId("task-run".to_string());
        state
            .begin_run(
                21,
                &provider_id,
                StructuredRun::new(
                    run_id.clone(),
                    DiscoveryGeneration(8),
                    vec![StructuredNodeId("case-a".to_string())],
                ),
            )
            .expect("run should begin");
        let running = StructuredTaskLifecycleEvent {
            task_id: task::TaskId("task-a".to_string()),
            state: StructuredTaskState::Running {
                terminal_id: Some(task::StructuredTerminalId(9)),
            },
        };
        apply_task_lifecycle(&mut state, 21, &provider_id, &run_id, &running)
            .expect("running lifecycle should apply");
        let completed = StructuredTaskLifecycleEvent {
            task_id: running.task_id.clone(),
            state: StructuredTaskState::Completed {
                terminal_id: Some(task::StructuredTerminalId(9)),
                exit_code: Some(7),
                success: false,
            },
        };
        apply_task_lifecycle(&mut state, 21, &provider_id, &run_id, &completed)
            .expect("completion should apply");
        let snapshot = state
            .provider(&provider_id)
            .expect("provider should remain");
        assert_eq!(
            snapshot
                .last_complete_run
                .as_ref()
                .and_then(|run| run.diagnostic.as_deref()),
            Some("Task exited with status 7")
        );
        let completed_run = snapshot
            .last_complete_run
            .as_ref()
            .expect("completed task should be retained");
        assert_eq!(completed_run.summary.failed, 1);
        assert_eq!(completed_run.events.len(), 2);
        assert_eq!(
            completed_run.events.last().map(|event| event.state),
            Some(StructuredNodeState::Failed)
        );

        let late_cancel = StructuredTaskLifecycleEvent {
            task_id: running.task_id,
            state: StructuredTaskState::Cancelled {
                terminal_id: Some(task::StructuredTerminalId(9)),
                termination_confirmed: true,
            },
        };
        assert!(
            apply_task_lifecycle(&mut state, 21, &provider_id, &run_id, &late_cancel)
                .expect_err("terminal runs must reject late lifecycle events")
                .to_string()
                .contains("terminal")
        );
    }

    #[test]
    fn structured_execution_filters_paths_and_enforces_retention_and_bounds() {
        let limits = StructuredExecutionLimits {
            nodes: 3,
            events_per_run: 2,
            message_bytes_per_run: 8,
            completed_runs: 1,
        };
        let mut state = StructuredExecutionState::with_limits(1, limits);
        let visible = HashSet::from_iter([WorktreeId::from_proto(1)]);
        let provider = fake_provider(1);
        let provider_id = provider.provider_id.clone();
        assert_eq!(
            state
                .apply_discovery(1, provider, Some(&visible))
                .expect("filtered discovery should apply"),
            StructuredApplyOutcome::Truncated
        );
        let snapshot = state.provider(&provider_id).expect("provider should exist");
        assert_eq!(snapshot.nodes.len(), 3);
        assert!(snapshot.partial);
        assert!(snapshot.nodes.iter().all(|node| {
            node.path
                .as_ref()
                .is_none_or(|path| path.worktree_id == WorktreeId::from_proto(1))
        }));

        for index in 0..2 {
            let run_id = StructuredRunId(format!("run-{index}"));
            state
                .begin_run(
                    1,
                    &provider_id,
                    StructuredRun::new(
                        run_id.clone(),
                        DiscoveryGeneration(1),
                        vec![StructuredNodeId("case-a".to_string())],
                    ),
                )
                .expect("run should begin");
            state
                .apply_event(
                    1,
                    &provider_id,
                    &run_id,
                    StructuredExecutionEvent {
                        sequence: 0,
                        node_id: StructuredNodeId("case-a".to_string()),
                        state: StructuredNodeState::Passed,
                        duration_millis: None,
                        message: Some("secret-value-is-too-long".to_string()),
                        location: None,
                    },
                    Some(&visible),
                )
                .expect("event should be bounded");
            state
                .set_run_phase(
                    1,
                    &provider_id,
                    &run_id,
                    StructuredRunPhase::Completed,
                    None,
                )
                .expect("run should complete");
        }
        let snapshot = state
            .provider(&provider_id)
            .expect("provider should remain");
        assert_eq!(snapshot.completed_runs.len(), 1);
        let retained = snapshot
            .last_complete_run
            .as_ref()
            .expect("last complete run should be separate");
        assert!(retained.truncated);
        assert_eq!(retained.events[0].message.as_deref(), Some("secret-v"));
    }

    #[test]
    fn rust_workspace_bounds_ten_thousand_structured_test_nodes_deterministically() {
        let provider_id = StructuredProviderId("scale-tests".to_string());
        let provider_node_id = StructuredNodeId("scale-provider".to_string());
        let nodes = std::iter::once(StructuredNode {
            id: provider_node_id.clone(),
            parent_id: None,
            label: "Scale tests".to_string(),
            kind: StructuredNodeKind::Provider,
            path: None,
        })
        .chain((0..10_000).map(|index| StructuredNode {
            id: StructuredNodeId(format!("scale-case-{index:05}")),
            parent_id: Some(provider_node_id.clone()),
            label: format!("case {index:05}"),
            kind: StructuredNodeKind::Case,
            path: Some(project_path(1, "tests/scale.rs")),
        }))
        .collect::<Vec<_>>();
        let snapshot = StructuredProviderSnapshot::discovery(
            provider_id.clone(),
            DiscoveryGeneration(1),
            StructuredProviderStatus::Current,
            nodes,
        );

        let mut first = StructuredExecutionState::new(1);
        let mut second = StructuredExecutionState::new(1);
        assert_eq!(
            first
                .apply_discovery(1, snapshot.clone(), None)
                .expect("large discovery should be bounded"),
            StructuredApplyOutcome::Truncated
        );
        assert_eq!(
            second
                .apply_discovery(1, snapshot, None)
                .expect("repeated large discovery should be bounded"),
            StructuredApplyOutcome::Truncated
        );
        let first_provider = first
            .provider(&provider_id)
            .expect("provider should be retained");
        let second_provider = second
            .provider(&provider_id)
            .expect("provider should be retained");
        assert_eq!(first_provider.nodes.len(), MAX_STRUCTURED_NODES);
        assert_eq!(first_provider.nodes, second_provider.nodes);
        assert_eq!(first_provider.status, StructuredProviderStatus::Partial);
        assert!(first_provider.partial);
        assert!(
            first_provider
                .diagnostic
                .as_deref()
                .is_some_and(|message| message.contains("10000"))
        );
    }

    #[test]
    fn structured_execution_proto_rejects_malformed_enums_and_bounds_pages() {
        assert!(
            node_from_proto(proto::StructuredExecutionNode {
                node_id: "node".to_string(),
                parent_id: None,
                label: "label".to_string(),
                kind: 99,
                path: None,
            })
            .expect_err("unknown node kind must fail")
            .to_string()
            .contains("invalid")
        );
        let mut state = StructuredExecutionState::new(4);
        let provider = fake_provider(2);
        let provider_id = provider.provider_id.clone();
        state
            .apply_discovery(4, provider, None)
            .expect("fixture should apply");
        let page = snapshot_page_to_proto(
            &state,
            &provider_id,
            DiscoveryGeneration(2),
            0,
            usize::MAX,
            None,
        )
        .expect("page should serialize");
        assert!(page.nodes.len() <= MAX_STRUCTURED_PAGE_SIZE);
        let encoded = format!("{page:?}");
        assert!(!encoded.contains("project_env"));
        assert!(!encoded.contains("/Users/"));
        assert!(!encoded.contains("terminal_bytes"));
    }
}
