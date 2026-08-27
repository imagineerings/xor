use std::{
    collections::{HashSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context as _, Result, anyhow};
use async_trait::async_trait;
use collections::HashMap;
use futures::{
    FutureExt as _,
    future::{AbortHandle, Abortable, Either, Shared},
};
use gpui::{
    App, AppContext as _, AsyncApp, BackgroundExecutor, Context, Entity, EventEmitter,
    Subscription, Task, TaskExt as _,
};
use rpc::{AnyProtoClient, TypedEnvelope, proto};
use util::{paths::PathStyle, rel_path::RelPath};

use crate::{
    ProjectEnvironment, ProjectPath,
    cargo_workspace::{
        CargoCandidateFailure, CargoConfigurationCompleteness,
        CargoConfigurationDiagnosticCategory, CargoHostCompilerModel, CargoHostCompilerStatus,
        CargoSnapshotCompleteness, CargoWorkspaceErrorCategory, CargoWorkspaceModel,
        CargoWorkspaceSnapshot, MAX_RUSTC_VERBOSE_VERSION_BYTES, deduplicate_workspaces,
        enrich_dependency_provenance, parse_cargo_profiles, parse_metadata, parse_rust_toolchain,
        parse_rustc_verbose_version, workspace_from_metadata,
    },
    trusted_worktrees::TrustedWorktrees,
    worktree_store::{WorktreeStore, WorktreeStoreEvent},
};

const MAX_ERROR_BYTES: usize = 4 * 1024;
const CARGO_CONFIGURATION_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoWorkspaceRemoteErrorKind {
    UnsupportedHost,
    Disconnected,
}

#[derive(Debug)]
pub struct CargoWorkspaceRemoteError {
    pub kind: CargoWorkspaceRemoteErrorKind,
}

impl std::fmt::Display for CargoWorkspaceRemoteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            CargoWorkspaceRemoteErrorKind::UnsupportedHost => formatter.write_str(
                "This project host does not include Rust tooling. Install or select a rust-tools-capable remote server.",
            ),
            CargoWorkspaceRemoteErrorKind::Disconnected => {
                formatter.write_str("The project host is disconnected. Reconnect and refresh Cargo.")
            }
        }
    }
}

impl std::error::Error for CargoWorkspaceRemoteError {}

#[derive(Clone, Debug)]
pub struct CargoMetadataRequest {
    pub manifest_path: PathBuf,
    pub working_directory: PathBuf,
    pub environment: Option<HashMap<String, String>>,
}

#[async_trait]
pub trait CargoMetadataRunner: Send + Sync {
    async fn run(&self, request: CargoMetadataRequest) -> Result<Vec<u8>>;
}

#[derive(Clone, Debug)]
pub struct CargoConfigurationProbeRequest {
    pub working_directory: PathBuf,
    pub environment: Option<HashMap<String, String>>,
}

#[async_trait]
pub trait CargoConfigurationProbe: Send + Sync {
    async fn run(&self, request: CargoConfigurationProbeRequest) -> Result<Vec<u8>>;
}

struct ProcessCargoMetadataRunner;

struct ProcessCargoConfigurationProbe {
    executor: BackgroundExecutor,
}

#[cfg(any(test, feature = "test-support"))]
struct UnavailableCargoConfigurationProbe;

#[async_trait]
impl CargoMetadataRunner for ProcessCargoMetadataRunner {
    async fn run(&self, request: CargoMetadataRequest) -> Result<Vec<u8>> {
        let mut command = util::command::new_command("cargo");
        command
            .args(["metadata", "--format-version", "1", "--manifest-path"])
            .arg(&request.manifest_path)
            .current_dir(&request.working_directory)
            .kill_on_drop(true);
        if let Some(environment) = request.environment {
            command.envs(environment);
        }
        let output = command.output().await.context("failed to start Cargo")?;
        if !output.status.success() {
            let message = bounded_error(&output.stderr);
            return Err(anyhow!(
                "Cargo metadata exited with {}: {message}",
                output.status
            ));
        }
        Ok(output.stdout)
    }
}

#[async_trait]
impl CargoConfigurationProbe for ProcessCargoConfigurationProbe {
    async fn run(&self, request: CargoConfigurationProbeRequest) -> Result<Vec<u8>> {
        let mut command = util::command::new_command("rustc");
        command
            .arg("-vV")
            .current_dir(&request.working_directory)
            .kill_on_drop(true);
        if let Some(environment) = request.environment {
            command.envs(environment);
        }
        let output = command.output().boxed();
        let timeout = self
            .executor
            .timer(CARGO_CONFIGURATION_PROBE_TIMEOUT)
            .boxed();
        let output = match futures::future::select(output, timeout).await {
            Either::Left((output, _)) => output.context("failed to start rustc")?,
            Either::Right(_) => return Err(anyhow!("rustc -vV timed out")),
        };
        if !output.status.success() {
            return Err(anyhow!(
                "rustc -vV exited with {}: {}",
                output.status,
                bounded_error(&output.stderr)
            ));
        }
        if output.stdout.len() > MAX_RUSTC_VERBOSE_VERSION_BYTES {
            return Err(anyhow!("rustc -vV output exceeded the supported limit"));
        }
        Ok(output.stdout)
    }
}

#[async_trait]
#[cfg(any(test, feature = "test-support"))]
impl CargoConfigurationProbe for UnavailableCargoConfigurationProbe {
    async fn run(&self, _request: CargoConfigurationProbeRequest) -> Result<Vec<u8>> {
        Err(anyhow!("rustc probe is unavailable in this test store"))
    }
}

#[derive(Clone, Debug)]
pub enum CargoWorkspaceStoreEvent {
    Invalidated,
}

impl EventEmitter<CargoWorkspaceStoreEvent> for CargoWorkspaceStore {}

pub struct CargoWorkspaceStore {
    mode: CargoWorkspaceStoreMode,
    worktree_store: Entity<WorktreeStore>,
    revision: u64,
    active_remote_request_id: Option<u64>,
    cancelled_remote_requests: HashSet<(proto::PeerId, u64)>,
    active_remote_requests: HashMap<(proto::PeerId, u64), AbortHandle>,
    _subscriptions: Vec<Subscription>,
}

enum CargoWorkspaceStoreMode {
    Local {
        environment: Entity<ProjectEnvironment>,
        runner: Arc<dyn CargoMetadataRunner>,
        configuration_probe: Arc<dyn CargoConfigurationProbe>,
        shared: Option<(u64, AnyProtoClient)>,
    },
    Remote {
        project_id: u64,
        client: AnyProtoClient,
    },
}

struct Candidate {
    project_path: ProjectPath,
    absolute_path: PathBuf,
    worktree_root: PathBuf,
    environment: Shared<Task<Option<HashMap<String, String>>>>,
    manifest_text: Task<Result<String>>,
    lock_text: Option<(ProjectPath, Task<Result<String>>)>,
    toolchain_text: Option<(ProjectPath, Task<Result<String>>)>,
    trusted: bool,
    private_paths: Arc<[Arc<RelPath>]>,
}

impl CargoWorkspaceStore {
    pub fn init(client: &AnyProtoClient) {
        client.add_entity_request_handler(Self::handle_get_workspace);
        client.add_entity_request_handler(Self::handle_cancel_workspace);
    }

    pub fn local(
        worktree_store: Entity<WorktreeStore>,
        environment: Entity<ProjectEnvironment>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut subscriptions = vec![cx.subscribe(&worktree_store, Self::handle_worktree_event)];
        if let Some(trusted_worktrees) = TrustedWorktrees::try_get_global(cx) {
            subscriptions.push(cx.subscribe(&trusted_worktrees, |_, _, _, cx| {
                cx.emit(CargoWorkspaceStoreEvent::Invalidated);
            }));
        }
        Self {
            mode: CargoWorkspaceStoreMode::Local {
                environment,
                runner: Arc::new(ProcessCargoMetadataRunner),
                configuration_probe: Arc::new(ProcessCargoConfigurationProbe {
                    executor: cx.background_executor().clone(),
                }),
                shared: None,
            },
            worktree_store,
            revision: 0,
            active_remote_request_id: None,
            cancelled_remote_requests: HashSet::new(),
            active_remote_requests: HashMap::default(),
            _subscriptions: subscriptions,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn local_with_runner(
        worktree_store: Entity<WorktreeStore>,
        environment: Entity<ProjectEnvironment>,
        runner: Arc<dyn CargoMetadataRunner>,
    ) -> Self {
        Self {
            mode: CargoWorkspaceStoreMode::Local {
                environment,
                runner,
                configuration_probe: Arc::new(UnavailableCargoConfigurationProbe),
                shared: None,
            },
            worktree_store,
            revision: 0,
            active_remote_request_id: None,
            cancelled_remote_requests: HashSet::new(),
            active_remote_requests: HashMap::default(),
            _subscriptions: Vec::new(),
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn local_with_runners(
        worktree_store: Entity<WorktreeStore>,
        environment: Entity<ProjectEnvironment>,
        runner: Arc<dyn CargoMetadataRunner>,
        configuration_probe: Arc<dyn CargoConfigurationProbe>,
    ) -> Self {
        Self {
            mode: CargoWorkspaceStoreMode::Local {
                environment,
                runner,
                configuration_probe,
                shared: None,
            },
            worktree_store,
            revision: 0,
            active_remote_request_id: None,
            cancelled_remote_requests: HashSet::new(),
            active_remote_requests: HashMap::default(),
            _subscriptions: Vec::new(),
        }
    }

    pub fn remote(
        project_id: u64,
        worktree_store: Entity<WorktreeStore>,
        client: AnyProtoClient,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut subscriptions = vec![cx.subscribe(&worktree_store, Self::handle_worktree_event)];
        if let Some(trusted_worktrees) = TrustedWorktrees::try_get_global(cx) {
            subscriptions.push(cx.subscribe(&trusted_worktrees, |_, _, _, cx| {
                cx.emit(CargoWorkspaceStoreEvent::Invalidated);
            }));
        }
        Self {
            mode: CargoWorkspaceStoreMode::Remote { project_id, client },
            worktree_store,
            revision: 0,
            active_remote_request_id: None,
            cancelled_remote_requests: HashSet::new(),
            active_remote_requests: HashMap::default(),
            _subscriptions: subscriptions,
        }
    }

    fn handle_worktree_event(
        &mut self,
        _: Entity<WorktreeStore>,
        event: &WorktreeStoreEvent,
        cx: &mut Context<Self>,
    ) {
        let relevant = match event {
            WorktreeStoreEvent::WorktreeUpdatedEntries(_, entries) => entries
                .iter()
                .any(|(path, _, _)| is_cargo_workspace_input(path)),
            WorktreeStoreEvent::WorktreeUpdatedGitRepositories(_, _)
            | WorktreeStoreEvent::WorktreeUpdatedRootRepoCommonDir(_)
            | WorktreeStoreEvent::WorktreeUpdateSent(_) => false,
            WorktreeStoreEvent::WorktreeAdded(_)
            | WorktreeStoreEvent::WorktreeRemoved(_, _)
            | WorktreeStoreEvent::WorktreeReleased(_, _)
            | WorktreeStoreEvent::WorktreeOrderChanged
            | WorktreeStoreEvent::WorktreeDeletedEntry(_, _) => true,
        };
        if relevant {
            cx.emit(CargoWorkspaceStoreEvent::Invalidated);
        }
    }

    pub fn shared(&mut self, project_id: u64, client: AnyProtoClient, _cx: &mut Context<Self>) {
        if let CargoWorkspaceStoreMode::Local { shared, .. } = &mut self.mode {
            *shared = Some((project_id, client));
        }
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) -> Task<Result<CargoWorkspaceSnapshot>> {
        self.refresh_scoped(None, cx)
    }

    fn refresh_scoped(
        &mut self,
        allowed_worktree_ids: Option<HashSet<settings::WorktreeId>>,
        cx: &mut Context<Self>,
    ) -> Task<Result<CargoWorkspaceSnapshot>> {
        self.revision = self.revision.wrapping_add(1);
        let revision = self.revision;
        match &self.mode {
            CargoWorkspaceStoreMode::Remote { project_id, client } => {
                let project_id = *project_id;
                let client = client.clone();
                if let Some(request_id) = self.active_remote_request_id.replace(revision) {
                    cx.background_spawn({
                        let client = client.clone();
                        async move {
                            client
                                .request(proto::CancelCargoWorkspace {
                                    project_id,
                                    request_id,
                                })
                                .await?;
                            Ok::<(), anyhow::Error>(())
                        }
                    })
                    .detach_and_log_err(cx);
                }
                let worktree_ids = self
                    .worktree_store
                    .read(cx)
                    .visible_worktrees(cx)
                    .map(|worktree| worktree.read(cx).id().to_proto())
                    .collect();
                let request = cx.background_spawn(async move {
                    client
                        .request(proto::GetCargoWorkspace {
                            project_id,
                            request_id: revision,
                            worktree_ids,
                        })
                        .await
                });
                cx.spawn(async move |this, cx| {
                    let response = request.await.map_err(|error| {
                        let unsupported = error
                            .downcast_ref::<proto::RpcError>()
                            .is_some_and(|error| error.raw_message().contains("was not handled"));
                        anyhow!(CargoWorkspaceRemoteError {
                            kind: if unsupported {
                                CargoWorkspaceRemoteErrorKind::UnsupportedHost
                            } else {
                                CargoWorkspaceRemoteErrorKind::Disconnected
                            },
                        })
                    });
                    this.update(cx, |store, _| {
                        if store.active_remote_request_id == Some(revision) {
                            store.active_remote_request_id = None;
                        }
                    })
                    .ok();
                    snapshot_from_proto(response?)
                })
            }
            CargoWorkspaceStoreMode::Local {
                environment,
                runner,
                configuration_probe,
                ..
            } => {
                let wait_for_scan = self.worktree_store.read(cx).wait_for_initial_scan();
                let environment = environment.clone();
                let runner = runner.clone();
                let configuration_probe = configuration_probe.clone();
                let worktree_store = self.worktree_store.clone();
                cx.spawn(async move |_this, cx| {
                    wait_for_scan.await;
                    let (candidates, input_fingerprint) = cx.update(|cx| {
                        collect_candidates(
                            &worktree_store,
                            &environment,
                            allowed_worktree_ids.as_ref(),
                            cx,
                        )
                    });
                    collect_snapshot(
                        revision,
                        input_fingerprint,
                        candidates,
                        runner,
                        configuration_probe,
                    )
                    .await
                })
            }
        }
    }

    pub fn current_input_fingerprint(&self, cx: &App) -> u64 {
        cargo_input_fingerprint(&self.worktree_store, None, cx)
    }

    async fn handle_get_workspace(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::GetCargoWorkspace>,
        mut cx: AsyncApp,
    ) -> Result<proto::CargoWorkspaceResponse> {
        let request_id = envelope.payload.request_id;
        let request_key = (envelope.sender_id, request_id);
        let allowed_worktree_ids = envelope
            .payload
            .worktree_ids
            .iter()
            .copied()
            .map(settings::WorktreeId::from_proto)
            .collect();
        let refresh = this.update(&mut cx, |store, cx| {
            store.refresh_scoped(Some(allowed_worktree_ids), cx)
        });
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let proceed = this.update(&mut cx, |store, _| {
            if store.cancelled_remote_requests.remove(&request_key) {
                false
            } else {
                store
                    .active_remote_requests
                    .insert(request_key, abort_handle);
                true
            }
        });
        if !proceed {
            return Err(anyhow!("Cargo workspace request was cancelled"));
        }
        let snapshot = Abortable::new(refresh, abort_registration)
            .await
            .map_err(|_| anyhow!("Cargo workspace request was cancelled"))??;
        let was_cancelled = this.update(&mut cx, |store, _| {
            store.active_remote_requests.remove(&request_key);
            store.cancelled_remote_requests.remove(&request_key)
        });
        if was_cancelled {
            return Err(anyhow!("Cargo workspace request was cancelled"));
        }
        Ok(snapshot_to_proto(snapshot, request_id))
    }

    async fn handle_cancel_workspace(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::CancelCargoWorkspace>,
        mut cx: AsyncApp,
    ) -> Result<proto::Ack> {
        this.update(&mut cx, |store, _| {
            let request_key = (envelope.sender_id, envelope.payload.request_id);
            if let Some(handle) = store.active_remote_requests.remove(&request_key) {
                handle.abort();
            } else {
                store.cancelled_remote_requests.insert(request_key);
            }
        });
        Ok(proto::Ack {})
    }
}

fn collect_candidates(
    worktree_store: &Entity<WorktreeStore>,
    environment: &Entity<ProjectEnvironment>,
    allowed_worktree_ids: Option<&HashSet<settings::WorktreeId>>,
    cx: &mut App,
) -> (Vec<Candidate>, u64) {
    let mut candidates = Vec::new();
    let worktrees = worktree_store
        .read(cx)
        .visible_worktrees(cx)
        .collect::<Vec<_>>();
    for worktree in worktrees {
        let (worktree_id, worktree_root, is_single_file, snapshot) =
            worktree.read_with(cx, |tree, _| {
                (
                    tree.id(),
                    tree.abs_path().to_path_buf(),
                    tree.is_single_file(),
                    tree.snapshot(),
                )
            });
        if allowed_worktree_ids.is_some_and(|allowed| !allowed.contains(&worktree_id)) {
            continue;
        }
        if is_single_file {
            continue;
        }
        let trusted = TrustedWorktrees::try_get_global(cx)
            .map(|trusted| {
                trusted.update(cx, |trusted, cx| {
                    trusted.can_trust(worktree_store, worktree_id, cx)
                })
            })
            .unwrap_or(true);
        let worktree_environment = environment.update(cx, |environment, cx| {
            environment.worktree_environment(worktree.clone(), cx)
        });
        let private_paths: Arc<[Arc<RelPath>]> = snapshot
            .entries(false, 0)
            .filter(|entry| entry.is_private)
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>()
            .into();
        for entry in snapshot.files(false, 0) {
            if entry.path.file_name() != Some("Cargo.toml") || entry.is_private {
                continue;
            }
            let manifest_path = entry.path.clone();
            let manifest_text = load_visible_worktree_text(&worktree, manifest_path.clone(), cx);
            let toolchain_text =
                nearest_toolchain_path(&snapshot, manifest_path.as_ref()).map(|path| {
                    let project_path = ProjectPath {
                        worktree_id,
                        path: path.clone(),
                    };
                    let text = load_visible_worktree_text(&worktree, path, cx);
                    (project_path, text)
                });
            let lock_text =
                nearest_cargo_lock_path(&snapshot, manifest_path.as_ref()).map(|path| {
                    let project_path = ProjectPath {
                        worktree_id,
                        path: path.clone(),
                    };
                    let text = load_visible_worktree_text(&worktree, path, cx);
                    (project_path, text)
                });
            candidates.push(Candidate {
                project_path: ProjectPath {
                    worktree_id,
                    path: manifest_path.clone(),
                },
                absolute_path: worktree_root.join(manifest_path.as_unix_str()),
                worktree_root: worktree_root.clone(),
                environment: worktree_environment.clone(),
                manifest_text,
                lock_text,
                toolchain_text,
                trusted,
                private_paths: private_paths.clone(),
            });
        }
    }
    candidates.sort_by(|left, right| {
        left.project_path
            .worktree_id
            .cmp(&right.project_path.worktree_id)
            .then_with(|| {
                left.project_path
                    .path
                    .components()
                    .count()
                    .cmp(&right.project_path.path.components().count())
            })
            .then_with(|| left.project_path.path.cmp(&right.project_path.path))
    });
    let input_fingerprint = cargo_input_fingerprint(worktree_store, allowed_worktree_ids, cx);
    (candidates, input_fingerprint)
}

fn nearest_cargo_lock_path(
    snapshot: &worktree::Snapshot,
    manifest_path: &RelPath,
) -> Option<Arc<RelPath>> {
    let manifest_directory = manifest_path.parent().unwrap_or(RelPath::empty());
    let lock_name = RelPath::from_unix_str("Cargo.lock").ok()?;
    for directory in manifest_directory.ancestors() {
        let path = directory.join(lock_name);
        if snapshot
            .entry_for_path(path.as_ref())
            .is_some_and(|entry| !entry.is_private && entry.is_file())
        {
            return Some(path.into());
        }
    }
    None
}

fn load_visible_worktree_text(
    worktree: &Entity<worktree::Worktree>,
    path: Arc<RelPath>,
    cx: &mut App,
) -> Task<Result<String>> {
    worktree.update(cx, |worktree, cx| {
        let load = worktree.load_file(path.as_ref(), cx);
        cx.spawn(async move |_worktree, _cx| load.await.map(|loaded| loaded.text))
    })
}

fn nearest_toolchain_path(
    snapshot: &worktree::Snapshot,
    manifest_path: &RelPath,
) -> Option<Arc<RelPath>> {
    let manifest_directory = match manifest_path.parent() {
        Some(parent) => parent,
        None => RelPath::empty(),
    };
    let toml_name = RelPath::from_unix_str("rust-toolchain.toml").ok()?;
    let legacy_name = RelPath::from_unix_str("rust-toolchain").ok()?;
    for directory in manifest_directory.ancestors() {
        for name in [toml_name, legacy_name] {
            let path = directory.join(name);
            if snapshot
                .entry_for_path(path.as_ref())
                .is_some_and(|entry| !entry.is_private && entry.is_file())
            {
                return Some(path.into());
            }
        }
    }
    None
}

fn cargo_input_fingerprint(
    worktree_store: &Entity<WorktreeStore>,
    allowed_worktree_ids: Option<&HashSet<settings::WorktreeId>>,
    cx: &App,
) -> u64 {
    let mut inputs = Vec::new();
    for worktree in worktree_store.read(cx).visible_worktrees(cx) {
        let (worktree_id, snapshot) =
            worktree.read_with(cx, |worktree, _| (worktree.id(), worktree.snapshot()));
        if allowed_worktree_ids.is_some_and(|allowed| !allowed.contains(&worktree_id)) {
            continue;
        }
        for entry in snapshot.files(false, 0) {
            if !entry.is_private && is_cargo_workspace_input(entry.path.as_ref()) {
                inputs.push((
                    worktree_id.to_proto(),
                    entry.path.as_unix_str().to_string(),
                    entry.id.to_proto(),
                    entry.mtime,
                    entry.size,
                ));
            }
        }
    }
    inputs.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
    let mut hasher = DefaultHasher::new();
    inputs.hash(&mut hasher);
    hasher.finish()
}

fn is_cargo_workspace_input(path: &RelPath) -> bool {
    match path.file_name() {
        Some("Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml" | "rust-toolchain") => true,
        Some("config.toml" | "config") => path
            .parent()
            .is_some_and(|parent| parent.file_name() == Some(".cargo")),
        _ => false,
    }
}

async fn collect_snapshot(
    revision: u64,
    input_fingerprint: u64,
    candidates: Vec<Candidate>,
    runner: Arc<dyn CargoMetadataRunner>,
    configuration_probe: Arc<dyn CargoConfigurationProbe>,
) -> Result<CargoWorkspaceSnapshot> {
    let mut loaded_candidates = Vec::with_capacity(candidates.len());
    let mut manifest_contents = std::collections::BTreeMap::new();
    for candidate in candidates {
        let Candidate {
            project_path,
            absolute_path,
            worktree_root,
            environment,
            manifest_text,
            lock_text,
            toolchain_text,
            trusted,
            private_paths,
        } = candidate;
        let manifest_text = manifest_text.await;
        if let Ok(contents) = &manifest_text {
            manifest_contents.insert(project_path.clone(), contents.clone());
        }
        let lock_text = match lock_text {
            Some((path, text)) => Some((path, text.await)),
            None => None,
        };
        loaded_candidates.push((
            project_path,
            absolute_path,
            worktree_root,
            environment,
            manifest_text,
            lock_text,
            toolchain_text,
            trusted,
            private_paths,
        ));
    }
    let roots = loaded_candidates
        .iter()
        .map(|candidate| {
            (
                candidate.0.worktree_id,
                candidate.2.clone(),
                candidate.8.clone(),
            )
        })
        .collect::<Vec<_>>();
    let mut covered = HashSet::new();
    let mut workspaces = Vec::new();
    let mut failures = Vec::new();
    for candidate in loaded_candidates {
        let (
            project_path,
            absolute_path,
            worktree_root,
            environment,
            manifest_text,
            lock_text,
            toolchain_text,
            trusted,
            _,
        ) = candidate;
        if covered.contains(&absolute_path) {
            continue;
        }
        if !trusted {
            failures.push(failure(
                project_path,
                CargoWorkspaceErrorCategory::Restricted,
                "Cargo metadata is disabled for this restricted worktree. Trust the worktree and refresh.",
            ));
            continue;
        }
        let environment = environment.await;
        let working_directory = absolute_path
            .parent()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| worktree_root.clone());
        let request = CargoMetadataRequest {
            working_directory: working_directory.clone(),
            manifest_path: absolute_path.clone(),
            environment: environment.clone(),
        };
        match runner.run(request).await {
            Ok(output) => match parse_metadata(&output) {
                Ok(metadata) => {
                    for package in &metadata.packages {
                        if metadata.workspace_members.contains(&package.id) {
                            covered.insert(package.manifest_path.as_std_path().to_path_buf());
                        }
                    }
                    covered.insert(
                        metadata
                            .workspace_root
                            .join("Cargo.toml")
                            .into_std_path_buf(),
                    );
                    match workspace_from_metadata(&metadata, |path| {
                        project_path_for_absolute_path(path, &roots)
                    }) {
                        Ok(mut workspace) => {
                            enrich_dependency_provenance(
                                &mut workspace,
                                &manifest_contents,
                                lock_text
                                    .as_ref()
                                    .and_then(|(_, contents)| contents.as_ref().ok())
                                    .map(String::as_str),
                            );
                            enrich_workspace_configuration(
                                &mut workspace,
                                project_path.clone(),
                                manifest_text,
                                toolchain_text,
                                working_directory,
                                environment,
                                &absolute_path,
                                &worktree_root,
                                configuration_probe.as_ref(),
                            )
                            .await;
                            workspaces.push(workspace);
                        }
                        Err(error) => failures.push(failure(
                            project_path,
                            CargoWorkspaceErrorCategory::InvalidMetadata,
                            &error.to_string(),
                        )),
                    }
                }
                Err(error) => failures.push(failure(
                    project_path,
                    if error.to_string().contains("unsupported") {
                        CargoWorkspaceErrorCategory::UnsupportedMetadata
                    } else {
                        CargoWorkspaceErrorCategory::InvalidMetadata
                    },
                    &error.to_string(),
                )),
            },
            Err(error) => failures.push(failure(
                project_path.clone(),
                if error.to_string().contains("failed to start Cargo") {
                    CargoWorkspaceErrorCategory::CargoNotFound
                } else {
                    CargoWorkspaceErrorCategory::CargoFailed
                },
                &sanitize_candidate_error(
                    &error.to_string(),
                    &absolute_path,
                    &worktree_root,
                    &project_path,
                ),
            )),
        }
    }
    let completeness = if failures.is_empty()
        && workspaces.iter().all(|workspace| {
            workspace.configuration.completeness == CargoConfigurationCompleteness::Complete
        }) {
        CargoSnapshotCompleteness::Complete
    } else {
        CargoSnapshotCompleteness::Partial
    };
    Ok(CargoWorkspaceSnapshot {
        revision,
        input_fingerprint,
        workspaces: deduplicate_workspaces(workspaces),
        failures,
        completeness,
    })
}

#[allow(clippy::too_many_arguments)]
async fn enrich_workspace_configuration(
    workspace: &mut CargoWorkspaceModel,
    manifest_path: ProjectPath,
    manifest_text: Result<String>,
    toolchain_text: Option<(ProjectPath, Task<Result<String>>)>,
    working_directory: PathBuf,
    environment: Option<HashMap<String, String>>,
    absolute_manifest_path: &std::path::Path,
    worktree_root: &std::path::Path,
    configuration_probe: &dyn CargoConfigurationProbe,
) {
    match manifest_text {
        Ok(manifest_text) => match parse_cargo_profiles(&manifest_text) {
            Ok(profiles) => workspace.configuration.profiles = profiles,
            Err(error) => workspace.configuration.add_diagnostic(
                Some(manifest_path.clone()),
                CargoConfigurationDiagnosticCategory::Manifest,
                sanitize_candidate_error(
                    &error.to_string(),
                    absolute_manifest_path,
                    worktree_root,
                    &manifest_path,
                ),
            ),
        },
        Err(error) => workspace.configuration.add_diagnostic(
            Some(manifest_path.clone()),
            CargoConfigurationDiagnosticCategory::Manifest,
            sanitize_candidate_error(
                &error.to_string(),
                absolute_manifest_path,
                worktree_root,
                &manifest_path,
            ),
        ),
    }

    if let Some((toolchain_path, toolchain_text)) = toolchain_text {
        match toolchain_text.await {
            Ok(contents) => match parse_rust_toolchain(toolchain_path.clone(), &contents) {
                Ok(toolchain) => workspace.configuration.declared_toolchain = Some(toolchain),
                Err(error) => workspace.configuration.add_diagnostic(
                    Some(toolchain_path),
                    CargoConfigurationDiagnosticCategory::Toolchain,
                    error.to_string(),
                ),
            },
            Err(error) => workspace.configuration.add_diagnostic(
                Some(toolchain_path),
                CargoConfigurationDiagnosticCategory::Toolchain,
                sanitize_candidate_error(
                    &error.to_string(),
                    absolute_manifest_path,
                    worktree_root,
                    &manifest_path,
                ),
            ),
        }
    }

    match configuration_probe
        .run(CargoConfigurationProbeRequest {
            working_directory,
            environment,
        })
        .await
    {
        Ok(output) => match parse_rustc_verbose_version(&output) {
            Ok(host_compiler) => workspace.configuration.host_compiler = host_compiler,
            Err(error) => {
                workspace.configuration.host_compiler = CargoHostCompilerModel {
                    status: CargoHostCompilerStatus::Failed,
                    release: None,
                    host_target: None,
                    stale: false,
                };
                workspace.configuration.add_diagnostic(
                    None,
                    CargoConfigurationDiagnosticCategory::CompilerProbe,
                    error.to_string(),
                );
            }
        },
        Err(error) => {
            workspace.configuration.host_compiler = CargoHostCompilerModel {
                status: if error.to_string().contains("failed to start rustc") {
                    CargoHostCompilerStatus::Missing
                } else {
                    CargoHostCompilerStatus::Failed
                },
                release: None,
                host_target: None,
                stale: false,
            };
            workspace.configuration.add_diagnostic(
                None,
                CargoConfigurationDiagnosticCategory::CompilerProbe,
                sanitize_candidate_error(
                    &error.to_string(),
                    absolute_manifest_path,
                    worktree_root,
                    &manifest_path,
                ),
            );
        }
    }
}

fn project_path_for_absolute_path(
    path: &std::path::Path,
    roots: &[(settings::WorktreeId, PathBuf, Arc<[Arc<RelPath>]>)],
) -> Option<ProjectPath> {
    roots.iter().find_map(|(worktree_id, root, private_paths)| {
        let relative = path.strip_prefix(root).ok()?;
        let relative = RelPath::new(relative, PathStyle::local()).ok()?;
        if private_paths
            .iter()
            .any(|private_path| relative.starts_with(private_path))
        {
            return None;
        }
        Some(ProjectPath {
            worktree_id: *worktree_id,
            path: relative.into_owned().into(),
        })
    })
}

fn failure(
    manifest_path: ProjectPath,
    category: CargoWorkspaceErrorCategory,
    message: &str,
) -> CargoCandidateFailure {
    CargoCandidateFailure {
        manifest_path,
        category,
        message: bounded_error(message.as_bytes()),
        has_stale_model: false,
    }
}

fn bounded_error(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(MAX_ERROR_BYTES);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

fn bounded_configuration_value(value: &str) -> String {
    let mut boundary = value
        .len()
        .min(crate::cargo_workspace::MAX_CARGO_CONFIGURATION_FIELD_BYTES);
    while !value.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    value[..boundary].to_string()
}

fn sanitize_candidate_error(
    message: &str,
    manifest_path: &std::path::Path,
    worktree_root: &std::path::Path,
    project_path: &ProjectPath,
) -> String {
    let relative = project_path.path.as_unix_str();
    message
        .replace(&manifest_path.to_string_lossy().to_string(), relative)
        .replace(&worktree_root.to_string_lossy().to_string(), "<worktree>")
}

fn snapshot_to_proto(
    snapshot: CargoWorkspaceSnapshot,
    request_id: u64,
) -> proto::CargoWorkspaceResponse {
    proto::CargoWorkspaceResponse {
        request_id,
        revision: snapshot.revision,
        input_fingerprint: snapshot.input_fingerprint,
        workspaces: snapshot
            .workspaces
            .into_iter()
            .map(workspace_to_proto)
            .collect(),
        failures: snapshot
            .failures
            .into_iter()
            .map(|failure| proto::CargoCandidateFailure {
                manifest_path: Some(failure.manifest_path.to_proto()),
                category: error_category_to_proto(failure.category),
                message: bounded_error(failure.message.as_bytes()),
                has_stale_model: failure.has_stale_model,
            })
            .collect(),
        partial: snapshot.completeness == CargoSnapshotCompleteness::Partial,
    }
}

fn workspace_to_proto(workspace: CargoWorkspaceModel) -> proto::CargoWorkspace {
    proto::CargoWorkspace {
        root: Some(workspace.key.root.to_proto()),
        root_manifest: workspace.root_manifest.map(|path| path.to_proto()),
        display_name: workspace.display_name,
        is_virtual: workspace.is_virtual,
        members: workspace
            .members
            .into_iter()
            .map(|package| proto::CargoPackage {
                name: package.name,
                version: package.version,
                manifest_path: Some(package.manifest_path.to_proto()),
                is_default_member: package.is_default_member,
                targets: package.targets.into_iter().map(target_to_proto).collect(),
                features: package
                    .features
                    .into_iter()
                    .map(|feature| proto::CargoFeature {
                        name: feature.name,
                        defined: feature.defined,
                        enabled: match feature.enabled {
                            crate::cargo_workspace::CargoFeatureEnabled::Unknown => 0,
                            crate::cargo_workspace::CargoFeatureEnabled::Enabled => 1,
                            crate::cargo_workspace::CargoFeatureEnabled::Disabled => 2,
                        },
                        expands: feature.expands,
                    })
                    .collect(),
                dependencies: package
                    .dependencies
                    .into_iter()
                    .map(dependency_to_proto)
                    .collect(),
            })
            .collect(),
        configuration: Some(configuration_to_proto(workspace.configuration)),
    }
}

fn configuration_to_proto(
    configuration: crate::cargo_workspace::CargoWorkspaceConfiguration,
) -> proto::CargoWorkspaceConfiguration {
    use crate::cargo_workspace::{
        CargoConfigurationDiagnosticCategory, CargoHostCompilerStatus, CargoProfileOrigin,
        CargoTargetConfiguration, CargoToolchainFormat, MAX_CARGO_CONFIGURATION_DIAGNOSTICS,
        MAX_CARGO_CONFIGURATION_ITEMS,
    };
    proto::CargoWorkspaceConfiguration {
        profiles: configuration
            .profiles
            .into_iter()
            .take(MAX_CARGO_CONFIGURATION_ITEMS)
            .map(|profile| proto::CargoProfile {
                name: bounded_configuration_value(&profile.name),
                origin: match profile.origin {
                    CargoProfileOrigin::Implicit => 0,
                    CargoProfileOrigin::Declared => 1,
                },
            })
            .collect(),
        declared_toolchain: configuration.declared_toolchain.map(|toolchain| {
            proto::CargoToolchain {
                source_path: Some(toolchain.source_path.to_proto()),
                format: match toolchain.format {
                    CargoToolchainFormat::Toml => 0,
                    CargoToolchainFormat::Legacy => 1,
                },
                channel: toolchain
                    .channel
                    .map(|channel| bounded_configuration_value(&channel)),
                components: toolchain
                    .components
                    .into_iter()
                    .take(MAX_CARGO_CONFIGURATION_ITEMS)
                    .map(|component| bounded_configuration_value(&component))
                    .collect(),
                targets: toolchain
                    .targets
                    .into_iter()
                    .take(MAX_CARGO_CONFIGURATION_ITEMS)
                    .map(|target| bounded_configuration_value(&target))
                    .collect(),
            }
        }),
        host_compiler: Some(proto::CargoHostCompiler {
            status: match configuration.host_compiler.status {
                CargoHostCompilerStatus::Unknown => 0,
                CargoHostCompilerStatus::Available => 1,
                CargoHostCompilerStatus::Restricted => 2,
                CargoHostCompilerStatus::Missing => 3,
                CargoHostCompilerStatus::Failed => 4,
            },
            release: configuration
                .host_compiler
                .release
                .map(|release| bounded_configuration_value(&release)),
            host_target: configuration
                .host_compiler
                .host_target
                .map(|host| bounded_configuration_value(&host)),
            stale: configuration.host_compiler.stale,
        }),
        cargo_target: match configuration.cargo_target {
            CargoTargetConfiguration::UnresolvedCargoDefault => 0,
        },
        diagnostics: configuration
            .diagnostics
            .into_iter()
            .take(MAX_CARGO_CONFIGURATION_DIAGNOSTICS)
            .map(|diagnostic| proto::CargoConfigurationDiagnostic {
                source_path: diagnostic.source_path.map(|path| path.to_proto()),
                category: match diagnostic.category {
                    CargoConfigurationDiagnosticCategory::Manifest => 0,
                    CargoConfigurationDiagnosticCategory::Toolchain => 1,
                    CargoConfigurationDiagnosticCategory::CompilerProbe => 2,
                },
                message: bounded_configuration_value(&diagnostic.message),
            })
            .collect(),
        partial: configuration.completeness == CargoConfigurationCompleteness::Partial,
    }
}

fn target_to_proto(target: crate::cargo_workspace::CargoTargetModel) -> proto::CargoTarget {
    use crate::cargo_workspace::CargoTargetKind;
    let (kind, other_kind) = match target.kind {
        CargoTargetKind::Library => (1, String::new()),
        CargoTargetKind::Binary => (2, String::new()),
        CargoTargetKind::Example => (3, String::new()),
        CargoTargetKind::Test => (4, String::new()),
        CargoTargetKind::Bench => (5, String::new()),
        CargoTargetKind::BuildScript => (6, String::new()),
        CargoTargetKind::Other(value) => (0, value),
    };
    proto::CargoTarget {
        name: target.name,
        kind,
        other_kind,
        crate_types: target.crate_types,
        source_path: target.source_path.map(|path| path.to_proto()),
        source_display_path: target.source_display_path,
        required_features: target.required_features,
        edition: target.edition,
    }
}

fn dependency_to_proto(
    dependency: crate::cargo_workspace::CargoDependencyModel,
) -> proto::CargoDependency {
    use crate::cargo_workspace::{
        CargoDependencyDeclarationOrigin, CargoDependencyFeatureCausality, CargoDependencyKind,
        CargoDependencyLockStatus, CargoDependencySourceKind,
    };
    let source_kind_to_proto = |source_kind| match source_kind {
        CargoDependencySourceKind::Other => 0,
        CargoDependencySourceKind::Path => 1,
        CargoDependencySourceKind::Registry => 2,
        CargoDependencySourceKind::Git => 3,
    };
    proto::CargoDependency {
        declaration_name: dependency.declaration_name,
        rename: dependency.rename,
        kind: match dependency.kind {
            CargoDependencyKind::Unknown => 0,
            CargoDependencyKind::Normal => 1,
            CargoDependencyKind::Development => 2,
            CargoDependencyKind::Build => 3,
        },
        version_requirement: dependency.version_requirement,
        optional: dependency.optional,
        uses_default_features: dependency.uses_default_features,
        requested_features: dependency.requested_features,
        target: dependency.target,
        source_kind: source_kind_to_proto(dependency.source_kind),
        resolved_name: dependency.resolved_name,
        resolved_version: dependency.resolved_version,
        resolved_workspace_member: dependency
            .resolved_workspace_member
            .map(|path| path.to_proto()),
        local_manifest: dependency.local_manifest.map(|path| path.to_proto()),
        declaration_manifest: dependency.declaration_manifest.map(|path| path.to_proto()),
        declaration_origin: match dependency.declaration_origin {
            CargoDependencyDeclarationOrigin::Unknown => 0,
            CargoDependencyDeclarationOrigin::Direct => 1,
            CargoDependencyDeclarationOrigin::WorkspaceInherited => 2,
        },
        resolved_instances: dependency
            .resolved_instances
            .into_iter()
            .map(|resolved| proto::CargoResolvedDependency {
                name: resolved.name,
                version: resolved.version,
                source_kind: source_kind_to_proto(resolved.source_kind),
                enabled_features: resolved.enabled_features,
                lock_status: match resolved.lock_status {
                    CargoDependencyLockStatus::Unknown => 0,
                    CargoDependencyLockStatus::Locked => 1,
                    CargoDependencyLockStatus::NotLocked => 2,
                    CargoDependencyLockStatus::MissingLockfile => 3,
                },
                workspace_member: resolved.workspace_member.map(|path| path.to_proto()),
                local_manifest: resolved.local_manifest.map(|path| path.to_proto()),
            })
            .collect(),
        resolution_truncated: dependency.resolution_truncated,
        feature_causality: match dependency.feature_causality {
            CargoDependencyFeatureCausality::Unknown => 0,
            CargoDependencyFeatureCausality::Validated => 1,
            CargoDependencyFeatureCausality::Ambiguous => 2,
        },
        cycle_detected: dependency.cycle_detected,
    }
}

fn snapshot_from_proto(response: proto::CargoWorkspaceResponse) -> Result<CargoWorkspaceSnapshot> {
    let workspaces = response
        .workspaces
        .into_iter()
        .map(workspace_from_proto)
        .collect::<Result<Vec<_>>>()?;
    let failures = response
        .failures
        .into_iter()
        .map(|failure| {
            Ok(CargoCandidateFailure {
                manifest_path: ProjectPath::from_proto(
                    failure
                        .manifest_path
                        .context("missing Cargo failure path")?,
                )
                .context("invalid Cargo failure path")?,
                category: error_category_from_proto(failure.category),
                message: bounded_error(failure.message.as_bytes()),
                has_stale_model: failure.has_stale_model,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(CargoWorkspaceSnapshot {
        revision: response.revision,
        input_fingerprint: response.input_fingerprint,
        workspaces,
        failures,
        completeness: if response.partial {
            CargoSnapshotCompleteness::Partial
        } else {
            CargoSnapshotCompleteness::Complete
        },
    })
}

fn workspace_from_proto(workspace: proto::CargoWorkspace) -> Result<CargoWorkspaceModel> {
    let root = ProjectPath::from_proto(workspace.root.context("missing Cargo workspace root")?)
        .context("invalid Cargo workspace root")?;
    let root_manifest = workspace.root_manifest.and_then(ProjectPath::from_proto);
    let members = workspace
        .members
        .into_iter()
        .map(package_from_proto)
        .collect::<Result<Vec<_>>>()?;
    let configuration = workspace
        .configuration
        .map(configuration_from_proto)
        .transpose()?
        .unwrap_or_else(crate::cargo_workspace::CargoWorkspaceConfiguration::unresolved);
    Ok(CargoWorkspaceModel {
        key: crate::cargo_workspace::CargoWorkspaceKey { root },
        root_manifest,
        display_name: workspace.display_name,
        is_virtual: workspace.is_virtual,
        members,
        configuration,
    })
}

fn configuration_from_proto(
    configuration: proto::CargoWorkspaceConfiguration,
) -> Result<crate::cargo_workspace::CargoWorkspaceConfiguration> {
    use crate::cargo_workspace::*;
    let profiles = configuration
        .profiles
        .into_iter()
        .take(MAX_CARGO_CONFIGURATION_ITEMS)
        .map(|profile| CargoProfileModel {
            name: bounded_configuration_value(&profile.name),
            origin: if profile.origin == 1 {
                CargoProfileOrigin::Declared
            } else {
                CargoProfileOrigin::Implicit
            },
        })
        .collect();
    let declared_toolchain = configuration
        .declared_toolchain
        .map(|toolchain| {
            Ok::<CargoToolchainModel, anyhow::Error>(CargoToolchainModel {
                source_path: ProjectPath::from_proto(
                    toolchain
                        .source_path
                        .context("missing Cargo toolchain source path")?,
                )
                .context("invalid Cargo toolchain source path")?,
                format: if toolchain.format == 1 {
                    CargoToolchainFormat::Legacy
                } else {
                    CargoToolchainFormat::Toml
                },
                channel: toolchain
                    .channel
                    .map(|channel| bounded_configuration_value(&channel)),
                components: toolchain
                    .components
                    .into_iter()
                    .take(MAX_CARGO_CONFIGURATION_ITEMS)
                    .map(|component| bounded_configuration_value(&component))
                    .collect(),
                targets: toolchain
                    .targets
                    .into_iter()
                    .take(MAX_CARGO_CONFIGURATION_ITEMS)
                    .map(|target| bounded_configuration_value(&target))
                    .collect(),
            })
        })
        .transpose()?;
    let host_compiler = configuration
        .host_compiler
        .map(|host| CargoHostCompilerModel {
            status: match host.status {
                1 => CargoHostCompilerStatus::Available,
                2 => CargoHostCompilerStatus::Restricted,
                3 => CargoHostCompilerStatus::Missing,
                4 => CargoHostCompilerStatus::Failed,
                _ => CargoHostCompilerStatus::Unknown,
            },
            release: host
                .release
                .map(|release| bounded_configuration_value(&release)),
            host_target: host
                .host_target
                .map(|target| bounded_configuration_value(&target)),
            stale: host.stale,
        })
        .unwrap_or_else(CargoHostCompilerModel::unknown);
    let diagnostics = configuration
        .diagnostics
        .into_iter()
        .take(MAX_CARGO_CONFIGURATION_DIAGNOSTICS)
        .map(|diagnostic| CargoConfigurationDiagnostic {
            source_path: diagnostic.source_path.and_then(ProjectPath::from_proto),
            category: match diagnostic.category {
                1 => CargoConfigurationDiagnosticCategory::Toolchain,
                2 => CargoConfigurationDiagnosticCategory::CompilerProbe,
                _ => CargoConfigurationDiagnosticCategory::Manifest,
            },
            message: bounded_configuration_value(&diagnostic.message),
        })
        .collect();
    Ok(CargoWorkspaceConfiguration {
        profiles,
        declared_toolchain,
        host_compiler,
        cargo_target: CargoTargetConfiguration::UnresolvedCargoDefault,
        diagnostics,
        completeness: if configuration.partial {
            CargoConfigurationCompleteness::Partial
        } else {
            CargoConfigurationCompleteness::Complete
        },
    })
}

fn package_from_proto(
    package: proto::CargoPackage,
) -> Result<crate::cargo_workspace::CargoPackageModel> {
    use crate::cargo_workspace::*;
    let manifest_path = ProjectPath::from_proto(
        package
            .manifest_path
            .context("missing Cargo package manifest path")?,
    )
    .context("invalid Cargo package manifest path")?;
    let id = stable_package_id(&manifest_path, &package.name, &package.version);
    Ok(CargoPackageModel {
        id,
        name: package.name,
        version: package.version,
        manifest_path,
        is_default_member: package.is_default_member,
        targets: package
            .targets
            .into_iter()
            .map(|target| CargoTargetModel {
                name: target.name,
                kind: match target.kind {
                    1 => CargoTargetKind::Library,
                    2 => CargoTargetKind::Binary,
                    3 => CargoTargetKind::Example,
                    4 => CargoTargetKind::Test,
                    5 => CargoTargetKind::Bench,
                    6 => CargoTargetKind::BuildScript,
                    _ => CargoTargetKind::Other(target.other_kind),
                },
                crate_types: target.crate_types,
                source_path: target.source_path.and_then(ProjectPath::from_proto),
                source_display_path: target.source_display_path,
                required_features: target.required_features,
                edition: target.edition,
            })
            .collect(),
        features: package
            .features
            .into_iter()
            .map(|feature| CargoFeatureModel {
                name: feature.name,
                defined: feature.defined,
                enabled: match feature.enabled {
                    1 => CargoFeatureEnabled::Enabled,
                    2 => CargoFeatureEnabled::Disabled,
                    _ => CargoFeatureEnabled::Unknown,
                },
                expands: feature.expands,
            })
            .collect(),
        dependencies: package
            .dependencies
            .into_iter()
            .map(|dependency| {
                let resolved_instance_count = dependency.resolved_instances.len();
                CargoDependencyModel {
                    declaration_name: dependency.declaration_name,
                    rename: dependency.rename,
                    kind: match dependency.kind {
                        1 => CargoDependencyKind::Normal,
                        2 => CargoDependencyKind::Development,
                        3 => CargoDependencyKind::Build,
                        _ => CargoDependencyKind::Unknown,
                    },
                    version_requirement: dependency.version_requirement,
                    optional: dependency.optional,
                    uses_default_features: dependency.uses_default_features,
                    requested_features: dependency.requested_features,
                    target: dependency.target,
                    source_kind: match dependency.source_kind {
                        1 => CargoDependencySourceKind::Path,
                        2 => CargoDependencySourceKind::Registry,
                        3 => CargoDependencySourceKind::Git,
                        _ => CargoDependencySourceKind::Other,
                    },
                    resolved_name: dependency.resolved_name,
                    resolved_version: dependency.resolved_version,
                    resolved_workspace_member: dependency
                        .resolved_workspace_member
                        .and_then(ProjectPath::from_proto),
                    local_manifest: dependency.local_manifest.and_then(ProjectPath::from_proto),
                    declaration_manifest: dependency
                        .declaration_manifest
                        .and_then(ProjectPath::from_proto),
                    declaration_origin: match dependency.declaration_origin {
                        1 => CargoDependencyDeclarationOrigin::Direct,
                        2 => CargoDependencyDeclarationOrigin::WorkspaceInherited,
                        _ => CargoDependencyDeclarationOrigin::Unknown,
                    },
                    resolved_instances: dependency
                        .resolved_instances
                        .into_iter()
                        .take(MAX_CARGO_DEPENDENCY_INSTANCES)
                        .map(|resolved| CargoResolvedDependencyModel {
                            name: resolved.name,
                            version: resolved.version,
                            source_kind: match resolved.source_kind {
                                1 => CargoDependencySourceKind::Path,
                                2 => CargoDependencySourceKind::Registry,
                                3 => CargoDependencySourceKind::Git,
                                _ => CargoDependencySourceKind::Other,
                            },
                            enabled_features: resolved
                                .enabled_features
                                .into_iter()
                                .take(MAX_CARGO_DEPENDENCY_FEATURES)
                                .collect(),
                            lock_status: match resolved.lock_status {
                                1 => CargoDependencyLockStatus::Locked,
                                2 => CargoDependencyLockStatus::NotLocked,
                                3 => CargoDependencyLockStatus::MissingLockfile,
                                _ => CargoDependencyLockStatus::Unknown,
                            },
                            workspace_member: resolved
                                .workspace_member
                                .and_then(ProjectPath::from_proto),
                            local_manifest: resolved
                                .local_manifest
                                .and_then(ProjectPath::from_proto),
                        })
                        .collect(),
                    resolution_truncated: dependency.resolution_truncated
                        || resolved_instance_count > MAX_CARGO_DEPENDENCY_INSTANCES,
                    feature_causality: match dependency.feature_causality {
                        1 => CargoDependencyFeatureCausality::Validated,
                        2 => CargoDependencyFeatureCausality::Ambiguous,
                        _ => CargoDependencyFeatureCausality::Unknown,
                    },
                    cycle_detected: dependency.cycle_detected,
                }
            })
            .collect(),
    })
}

fn error_category_to_proto(category: CargoWorkspaceErrorCategory) -> i32 {
    match category {
        CargoWorkspaceErrorCategory::Internal => 0,
        CargoWorkspaceErrorCategory::Restricted => 1,
        CargoWorkspaceErrorCategory::CargoNotFound => 2,
        CargoWorkspaceErrorCategory::CargoFailed => 3,
        CargoWorkspaceErrorCategory::InvalidMetadata => 4,
        CargoWorkspaceErrorCategory::UnsupportedMetadata => 5,
        CargoWorkspaceErrorCategory::Disconnected => 6,
        CargoWorkspaceErrorCategory::Cancelled => 7,
    }
}

fn error_category_from_proto(category: i32) -> CargoWorkspaceErrorCategory {
    match category {
        1 => CargoWorkspaceErrorCategory::Restricted,
        2 => CargoWorkspaceErrorCategory::CargoNotFound,
        3 => CargoWorkspaceErrorCategory::CargoFailed,
        4 => CargoWorkspaceErrorCategory::InvalidMetadata,
        5 => CargoWorkspaceErrorCategory::UnsupportedMetadata,
        6 => CargoWorkspaceErrorCategory::Disconnected,
        7 => CargoWorkspaceErrorCategory::Cancelled,
        _ => CargoWorkspaceErrorCategory::Internal,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use futures::FutureExt as _;
    use gpui::TestAppContext;

    use super::*;

    struct FakeRunner {
        output: Vec<u8>,
        requests: Arc<Mutex<Vec<CargoMetadataRequest>>>,
    }

    struct RoutingRunner {
        requests: Arc<Mutex<Vec<PathBuf>>>,
    }

    struct FakeConfigurationProbe {
        output: Result<Vec<u8>, String>,
        requests: Arc<Mutex<Vec<CargoConfigurationProbeRequest>>>,
    }

    #[async_trait]
    impl CargoConfigurationProbe for FakeConfigurationProbe {
        async fn run(&self, request: CargoConfigurationProbeRequest) -> Result<Vec<u8>> {
            self.requests
                .lock()
                .expect("configuration probe request lock should be available")
                .push(request);
            self.output.clone().map_err(anyhow::Error::msg)
        }
    }

    fn successful_configuration_probe() -> Arc<dyn CargoConfigurationProbe> {
        successful_configuration_probe_with_requests(Arc::new(Mutex::new(Vec::new())))
    }

    fn successful_configuration_probe_with_requests(
        requests: Arc<Mutex<Vec<CargoConfigurationProbeRequest>>>,
    ) -> Arc<dyn CargoConfigurationProbe> {
        Arc::new(FakeConfigurationProbe {
            output: Ok(b"rustc 1.90.0\nbinary: rustc\ncommit-hash: unknown\ncommit-date: unknown\nhost: x86_64-unknown-linux-gnu\nrelease: 1.90.0\nLLVM version: 20.1.0\n".to_vec()),
            requests,
        })
    }

    #[async_trait]
    impl CargoMetadataRunner for RoutingRunner {
        async fn run(&self, request: CargoMetadataRequest) -> Result<Vec<u8>> {
            self.requests
                .lock()
                .expect("routing runner request lock should be available")
                .push(request.manifest_path.clone());
            if request.manifest_path.starts_with("/workspace") {
                Ok(include_bytes!("../test_data/cargo_workspace/workspace-v1.json").to_vec())
            } else if request.manifest_path.starts_with("/standalone") {
                Ok(include_bytes!("../test_data/cargo_workspace/standalone-v1.json").to_vec())
            } else {
                Err(anyhow!(
                    "metadata failed for {}",
                    request.manifest_path.display()
                ))
            }
        }
    }

    fn candidate(
        worktree_id: usize,
        worktree_root: &str,
        manifest_path: &str,
        trusted: bool,
    ) -> Candidate {
        let relative = std::path::Path::new(manifest_path)
            .strip_prefix(worktree_root)
            .expect("fixture manifest should be inside its worktree");
        let relative = relative
            .to_str()
            .expect("fixture manifest should be valid UTF-8");
        let relative = RelPath::new(std::path::Path::new(relative), PathStyle::local())
            .expect("fixture manifest should be project relative");
        let relative: Arc<RelPath> = Arc::from(relative.as_ref());
        Candidate {
            project_path: ProjectPath {
                worktree_id: settings::WorktreeId::from_usize(worktree_id),
                path: relative,
            },
            absolute_path: PathBuf::from(manifest_path),
            worktree_root: PathBuf::from(worktree_root),
            environment: Task::ready(None).shared(),
            manifest_text: Task::ready(Ok(
                "[workspace]\nresolver = \"2\"\n[profile.ship]\ninherits = \"release\"\n"
                    .to_string(),
            )),
            lock_text: None,
            toolchain_text: None,
            trusted,
            private_paths: Arc::from([]),
        }
    }

    #[async_trait]
    impl CargoMetadataRunner for FakeRunner {
        async fn run(&self, request: CargoMetadataRequest) -> Result<Vec<u8>> {
            self.requests
                .lock()
                .expect("fake runner request lock should be available")
                .push(request);
            Ok(self.output.clone())
        }
    }

    #[gpui::test]
    async fn cargo_workspace_fake_runner_uses_candidate_environment(_cx: &mut TestAppContext) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let runner = Arc::new(FakeRunner {
            output: include_bytes!("../test_data/cargo_workspace/workspace-v1.json").to_vec(),
            requests: requests.clone(),
        });
        let environment = collections::HashMap::from_iter([(
            "CARGO_HOME".to_string(),
            "/isolated/cargo".to_string(),
        )]);
        let candidate = Candidate {
            project_path: ProjectPath {
                worktree_id: settings::WorktreeId::from_usize(1),
                path: Arc::from(RelPath::from_unix_str("Cargo.toml").expect("valid fixture path")),
            },
            absolute_path: PathBuf::from("/workspace/Cargo.toml"),
            worktree_root: PathBuf::from("/workspace"),
            environment: Task::ready(Some(environment.clone())).shared(),
            manifest_text: Task::ready(Ok("[workspace]\nresolver = \"2\"\n".to_string())),
            lock_text: None,
            toolchain_text: None,
            trusted: true,
            private_paths: Arc::from([]),
        };
        let snapshot = collect_snapshot(
            5,
            17,
            vec![candidate],
            runner,
            successful_configuration_probe(),
        )
        .await
        .expect("fake metadata should convert");
        assert_eq!(snapshot.revision, 5);
        assert_eq!(snapshot.input_fingerprint, 17);
        assert_eq!(snapshot.workspaces.len(), 1);
        assert!(snapshot.failures.is_empty());
        let requests = requests
            .lock()
            .expect("fake runner request lock should be available");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].environment.as_ref(), Some(&environment));
        assert_eq!(
            requests[0].manifest_path,
            PathBuf::from("/workspace/Cargo.toml")
        );
    }

    #[gpui::test]
    async fn cargo_workspace_collects_multiple_roots_skips_covered_members_and_keeps_failures(
        _cx: &mut TestAppContext,
    ) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let snapshot = collect_snapshot(
            9,
            27,
            vec![
                candidate(1, "/workspace", "/workspace/Cargo.toml", true),
                candidate(1, "/workspace", "/workspace/member-one/Cargo.toml", true),
                candidate(2, "/standalone", "/standalone/Cargo.toml", true),
                candidate(3, "/broken", "/broken/Cargo.toml", true),
            ],
            Arc::new(RoutingRunner {
                requests: requests.clone(),
            }),
            successful_configuration_probe(),
        )
        .await
        .expect("partial fixture collection should succeed");

        assert_eq!(snapshot.revision, 9);
        assert_eq!(snapshot.input_fingerprint, 27);
        assert_eq!(snapshot.completeness, CargoSnapshotCompleteness::Partial);
        assert_eq!(snapshot.workspaces.len(), 2);
        assert_eq!(snapshot.failures.len(), 1);
        let requests = requests
            .lock()
            .expect("routing runner request lock should be available");
        assert_eq!(requests.len(), 3);
        assert!(!requests.contains(&PathBuf::from("/workspace/member-one/Cargo.toml")));
    }

    #[gpui::test]
    async fn cargo_workspace_restricted_candidate_never_runs_metadata(_cx: &mut TestAppContext) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let probe_requests = Arc::new(Mutex::new(Vec::new()));
        let snapshot = collect_snapshot(
            1,
            2,
            vec![candidate(1, "/workspace", "/workspace/Cargo.toml", false)],
            Arc::new(RoutingRunner {
                requests: requests.clone(),
            }),
            successful_configuration_probe_with_requests(probe_requests.clone()),
        )
        .await
        .expect("restricted collection should return a scoped failure");

        assert!(snapshot.workspaces.is_empty());
        assert_eq!(snapshot.failures.len(), 1);
        assert_eq!(
            snapshot.failures[0].category,
            CargoWorkspaceErrorCategory::Restricted
        );
        assert!(
            requests
                .lock()
                .expect("routing runner request lock should be available")
                .is_empty()
        );
        assert!(
            probe_requests
                .lock()
                .expect("configuration probe request lock should be available")
                .is_empty()
        );
    }

    #[gpui::test]
    async fn cargo_workspace_configuration_uses_visible_files_and_project_environment(
        _cx: &mut TestAppContext,
    ) {
        let probe_requests = Arc::new(Mutex::new(Vec::new()));
        let environment = collections::HashMap::from_iter([
            ("RUSTUP_TOOLCHAIN".to_string(), "stable".to_string()),
            ("SECRET_TOKEN".to_string(), "not-for-the-model".to_string()),
        ]);
        let mut candidate = candidate(1, "/workspace", "/workspace/Cargo.toml", true);
        candidate.environment = Task::ready(Some(environment.clone())).shared();
        candidate.manifest_text = Task::ready(Ok(include_str!(
            "../test_data/cargo_workspace/profiles-custom.toml"
        )
        .to_string()));
        candidate.toolchain_text = Some((
            ProjectPath {
                worktree_id: settings::WorktreeId::from_usize(1),
                path: Arc::from(
                    RelPath::from_unix_str("rust-toolchain.toml").expect("valid fixture path"),
                ),
            },
            Task::ready(Ok(include_str!(
                "../test_data/cargo_workspace/rust-toolchain.toml"
            )
            .to_string())),
        ));

        let snapshot = collect_snapshot(
            2,
            3,
            vec![candidate],
            Arc::new(RoutingRunner {
                requests: Arc::new(Mutex::new(Vec::new())),
            }),
            successful_configuration_probe_with_requests(probe_requests.clone()),
        )
        .await
        .expect("configuration fixture should collect");
        let configuration = &snapshot.workspaces[0].configuration;
        assert_eq!(
            configuration.completeness,
            CargoConfigurationCompleteness::Complete
        );
        assert_eq!(configuration.profiles.len(), 4);
        assert_eq!(
            configuration
                .declared_toolchain
                .as_ref()
                .and_then(|toolchain| toolchain.channel.as_deref()),
            Some("stable")
        );
        assert_eq!(
            configuration.host_compiler.host_target.as_deref(),
            Some("x86_64-unknown-linux-gnu")
        );
        let requests = probe_requests
            .lock()
            .expect("configuration probe request lock should be available");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].environment.as_ref(), Some(&environment));

        let encoded = format!("{:?}", snapshot_to_proto(snapshot, 4));
        assert!(!encoded.contains("SECRET_TOKEN"));
        assert!(!encoded.contains("not-for-the-model"));
        assert!(!encoded.contains("/workspace"));
    }

    #[gpui::test]
    async fn cargo_workspace_configuration_failures_are_partial_and_isolated(
        _cx: &mut TestAppContext,
    ) {
        let mut candidate = candidate(1, "/workspace", "/workspace/Cargo.toml", true);
        candidate.manifest_text = Task::ready(Ok(include_str!(
            "../test_data/cargo_workspace/profiles-malformed.toml"
        )
        .to_string()));
        let snapshot = collect_snapshot(
            4,
            5,
            vec![candidate],
            Arc::new(RoutingRunner {
                requests: Arc::new(Mutex::new(Vec::new())),
            }),
            Arc::new(FakeConfigurationProbe {
                output: Err("failed to start rustc: not found".to_string()),
                requests: Arc::new(Mutex::new(Vec::new())),
            }),
        )
        .await
        .expect("configuration failures should not discard metadata");
        assert_eq!(snapshot.workspaces.len(), 1);
        assert_eq!(snapshot.completeness, CargoSnapshotCompleteness::Partial);
        let configuration = &snapshot.workspaces[0].configuration;
        assert_eq!(
            configuration.profiles,
            crate::cargo_workspace::implicit_cargo_profiles()
        );
        assert_eq!(
            configuration.host_compiler.status,
            CargoHostCompilerStatus::Missing
        );
        assert_eq!(configuration.diagnostics.len(), 2);
    }

    #[test]
    fn cargo_workspace_path_projection_filters_private_and_outside_paths() {
        let roots = vec![(
            settings::WorktreeId::from_usize(1),
            PathBuf::from("/workspace"),
            Arc::from([Arc::from(
                RelPath::from_unix_str("private").expect("valid private fixture path"),
            )]),
        )];
        assert!(
            project_path_for_absolute_path(std::path::Path::new("/outside/Cargo.toml"), &roots)
                .is_none()
        );
        assert!(
            project_path_for_absolute_path(
                std::path::Path::new("/workspace/private/Cargo.toml"),
                &roots,
            )
            .is_none()
        );
        assert_eq!(
            project_path_for_absolute_path(
                std::path::Path::new("/workspace/public/Cargo.toml"),
                &roots,
            )
            .expect("public path should project")
            .path
            .as_unix_str(),
            "public/Cargo.toml"
        );
    }

    #[test]
    fn cargo_workspace_errors_are_bounded_and_lossy_utf8_safe() {
        let mut bytes = vec![b'x'; MAX_ERROR_BYTES + 32];
        bytes.extend_from_slice(&[0xf0, 0x28, 0x8c, 0x28]);
        let message = bounded_error(&bytes);
        assert!(message.len() <= MAX_ERROR_BYTES + 12);
        assert!(message.contains('\u{fffd}'));
    }

    #[test]
    fn cargo_workspace_errors_remove_host_paths() {
        let path = ProjectPath {
            worktree_id: settings::WorktreeId::from_usize(1),
            path: Arc::from(
                RelPath::from_unix_str("member/Cargo.toml").expect("valid fixture path"),
            ),
        };
        let sanitized = sanitize_candidate_error(
            "failed at /host/private/member/Cargo.toml under /host/private",
            std::path::Path::new("/host/private/member/Cargo.toml"),
            std::path::Path::new("/host/private"),
            &path,
        );
        assert!(!sanitized.contains("/host/private"));
        assert!(sanitized.contains("member/Cargo.toml"));
    }

    #[test]
    fn cargo_workspace_input_filter_includes_configuration_files_only() {
        for path in [
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "member/rust-toolchain",
            ".cargo/config.toml",
            "member/.cargo/config",
        ] {
            assert!(is_cargo_workspace_input(
                RelPath::from_unix_str(path).expect("fixture path should be valid")
            ));
        }
        for path in ["src/config.toml", ".cargo/notes.txt", "README.md"] {
            assert!(!is_cargo_workspace_input(
                RelPath::from_unix_str(path).expect("fixture path should be valid")
            ));
        }
    }

    #[test]
    fn cargo_workspace_remote_request_keys_are_peer_scoped() {
        let mut requests = HashSet::new();
        requests.insert(((1_u64, 2_u64), 9_u64));
        assert!(!requests.contains(&((1_u64, 3_u64), 9_u64)));
    }

    #[gpui::test]
    async fn cargo_workspace_remote_snapshot_round_trip_preserves_the_host_model(
        _cx: &mut TestAppContext,
    ) {
        let snapshot = collect_snapshot(
            12,
            34,
            vec![candidate(1, "/workspace", "/workspace/Cargo.toml", true)],
            Arc::new(RoutingRunner {
                requests: Arc::new(Mutex::new(Vec::new())),
            }),
            successful_configuration_probe(),
        )
        .await
        .expect("host fixture collection should succeed");

        let response = snapshot_to_proto(snapshot.clone(), 56);
        assert_eq!(response.request_id, 56);
        assert_eq!(
            snapshot_from_proto(response).expect("typed remote snapshot should decode"),
            snapshot
        );
    }
}
