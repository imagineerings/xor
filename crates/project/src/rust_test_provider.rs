use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::PathBuf,
    process::Output,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context as _, Result, anyhow, bail};
use cargo_metadata::{Message, TargetKind};
use collections::HashMap;
use futures::{
    StreamExt as _,
    channel::mpsc,
    future::{AbortHandle, Abortable, BoxFuture, Shared},
};
use gpui::{
    App, AppContext as _, AsyncApp, BackgroundExecutor, Context, Entity, EventEmitter,
    Subscription, Task, TaskExt as _,
};
use rpc::{AnyProtoClient, TypedEnvelope, proto};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use task::{
    BuildTaskDefinition, DebugScenario, SaveStrategy, StructuredTaskHandle,
    StructuredTaskLifecycleEvent, StructuredTaskState, StructuredTerminalId, TaskId, TaskTemplate,
    VariableName,
};

use crate::{
    ProjectEnvironment, ProjectPath, WorktreeId,
    cargo_workspace::{
        CargoPackageModel, CargoSnapshotCompleteness, CargoTargetKind, CargoWorkspaceErrorCategory,
        CargoWorkspaceModel, CargoWorkspaceSnapshot,
    },
    cargo_workspace_store::CargoWorkspaceStore,
    structured_execution::{
        DiscoveryGeneration, StructuredExecutionStore, StructuredExecutionStoreEvent,
        StructuredNode, StructuredNodeId, StructuredNodeKind, StructuredProviderId,
        StructuredProviderSnapshot, StructuredProviderStatus, StructuredRunId,
    },
    trusted_worktrees::TrustedWorktrees,
    worktree_store::{WorktreeStore, WorktreeStoreEvent},
};

pub const RUST_TEST_PROTOCOL_VERSION: u32 = 1;
pub const MAX_RUST_TEST_CARGO_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_RUST_TEST_LISTING_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_RUST_TEST_LINES: usize = 50_000;
pub const MAX_RUST_TEST_HARNESSES: usize = 2_000;
pub const MAX_RUST_TEST_CASES: usize = 10_000;
pub const MAX_RUST_TEST_FIELD_BYTES: usize = 1_024;
pub const MAX_RUST_TEST_DIAGNOSTICS: usize = 64;
pub const RUST_TEST_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(20);
pub const RUST_TEST_PROVIDER_ID: &str = "rust-tests";
pub const RUST_TEST_DISCOVERY_PROTOCOL_VERSION: u32 = 1;
pub const MAX_RUST_TEST_REQUEST_WORKTREES: usize = 256;
pub const MAX_PENDING_RUST_TEST_CANCELLATIONS: usize = 256;
pub const MAX_RUST_TEST_RERUNS: usize = 4;
pub const MAX_RUST_TEST_ACTION_PLAN_BYTES: usize = 64 * 1024;
pub const MAX_RUST_TEST_ACTION_ARGUMENTS: usize = 256;
pub const MAX_PENDING_REMOTE_RUST_TEST_RUNS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustTestAction {
    Run,
    Debug,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RustTestActionPlan {
    Task {
        run_id: StructuredRunId,
        discovery_generation: DiscoveryGeneration,
        scope_node_ids: Vec<StructuredNodeId>,
        worktree_id: WorktreeId,
        template: TaskTemplate,
    },
    Debug {
        worktree_id: WorktreeId,
        scenario: DebugScenario,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum RustTestActionWirePlan {
    Task {
        run_id: String,
        discovery_generation: u64,
        scope_node_ids: Vec<String>,
        worktree_id: u64,
        template: TaskTemplate,
    },
    Debug {
        worktree_id: u64,
        build_template: TaskTemplate,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RustTestActionScope {
    Workspace,
    Package,
    Target,
    Group,
    Case,
}

#[derive(Clone, Debug)]
struct RustTestActionDescriptor {
    scope: RustTestActionScope,
    worktree_id: WorktreeId,
    manifest_path: ProjectPath,
    label: String,
    cargo_args: Vec<String>,
    executable_args: Vec<String>,
    target_kind: Option<RustTestTargetKind>,
}

#[derive(Clone, Debug)]
struct AuthorizedRemoteRun {
    discovery_generation: DiscoveryGeneration,
    scope_node_ids: Vec<StructuredNodeId>,
    worktree_id: WorktreeId,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustTestTargetKind {
    Unit,
    Integration,
    Binary,
    Example,
    Benchmark,
    Doctest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustTestCaseKind {
    Test,
    Benchmark,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustTestListingMode {
    All,
    Ignored,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RustTestHarnessLocator {
    Executable(PathBuf),
    CargoDoctest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustTestHarness {
    pub id: String,
    pub package_id: String,
    pub target_name: String,
    pub target_kind: RustTestTargetKind,
    pub locator: RustTestHarnessLocator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustTestRunnableSelector {
    pub cargo_args: Vec<String>,
    pub executable_args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustTestCaseRecord {
    pub id: String,
    pub harness_id: String,
    pub name: String,
    pub kind: RustTestCaseKind,
    pub ignored: bool,
    pub source: Option<ProjectPath>,
    pub runnable: RustTestRunnableSelector,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustTestProtocolDiagnostic {
    pub stage: RustTestProtocolStage,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustTestProtocolStage {
    CargoMessages,
    HarnessListing,
    Enrichment,
    Limits,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustTestProtocolCapability {
    Supported,
    Partial,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustTestProtocolSnapshot {
    pub protocol_version: u32,
    pub toolchain: String,
    pub capability: RustTestProtocolCapability,
    pub harnesses: Vec<RustTestHarness>,
    pub cases: Vec<RustTestCaseRecord>,
    pub diagnostics: Vec<RustTestProtocolDiagnostic>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RustTestListingCapture {
    pub package_id: String,
    pub target_name: String,
    pub target_kind: RustTestTargetKind,
    pub mode: RustTestListingMode,
    pub stdout: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustTestDiscoveryCapture {
    pub toolchain: String,
    pub cargo_messages: String,
    pub listings: Vec<RustTestListingCapture>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustTestSourceHint {
    pub package_id: String,
    pub target_name: String,
    pub target_kind: RustTestTargetKind,
    pub test_name: String,
    pub source: Option<ProjectPath>,
    pub runnable: Option<RustTestRunnableSelector>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustTestDiscoveryRequest {
    pub workspace_root: ProjectPath,
    pub generation: u64,
    pub limits: RustTestProtocolLimits,
    pub manifest_path: Option<PathBuf>,
    pub working_directory: Option<PathBuf>,
    pub environment: Option<HashMap<String, String>>,
    pub doctest_targets: Vec<(String, String)>,
}

pub trait RustTestDiscoveryRunner: Send + Sync {
    fn discover(
        &self,
        request: RustTestDiscoveryRequest,
    ) -> BoxFuture<'static, Result<RustTestDiscoveryCapture>>;
}

struct ProcessRustTestDiscoveryRunner {
    executor: BackgroundExecutor,
}

impl RustTestDiscoveryRunner for ProcessRustTestDiscoveryRunner {
    fn discover(
        &self,
        request: RustTestDiscoveryRequest,
    ) -> BoxFuture<'static, Result<RustTestDiscoveryCapture>> {
        let executor = self.executor.clone();
        Box::pin(async move {
            let manifest_path = request
                .manifest_path
                .context("Rust test discovery requires a host manifest path")?;
            let working_directory = request
                .working_directory
                .context("Rust test discovery requires a host working directory")?;
            let mut cargo = util::command::new_command("cargo");
            cargo
                .args([
                    "test",
                    "--workspace",
                    "--all-targets",
                    "--no-run",
                    "--offline",
                    "--message-format=json",
                    "--manifest-path",
                ])
                .arg(&manifest_path)
                .current_dir(&working_directory)
                .kill_on_drop(true);
            if let Some(environment) = request.environment.as_ref() {
                cargo.envs(environment);
            }
            let output = bounded_command_output(cargo, request.limits.timeout, &executor).await?;
            if !output.status.success() {
                bail!(
                    "Cargo test discovery exited with {}: {}",
                    output.status,
                    bounded_process_error(&output.stderr, request.limits.field_bytes)
                );
            }
            if output.stdout.len() > request.limits.cargo_bytes {
                bail!("Cargo test discovery exceeded the structured output limit");
            }
            let cargo_messages = String::from_utf8(output.stdout)
                .context("Cargo test discovery emitted non-UTF-8 JSON")?;
            let adapter = RustTestProtocolAdapter::new(request.limits);
            let harness_snapshot = adapter.adapt(
                RustTestDiscoveryCapture {
                    toolchain: String::new(),
                    cargo_messages: cargo_messages.clone(),
                    listings: Vec::new(),
                },
                &[],
            );
            let mut listings = Vec::new();
            for harness in harness_snapshot.harnesses {
                let RustTestHarnessLocator::Executable(executable) = harness.locator else {
                    continue;
                };
                for mode in [RustTestListingMode::All, RustTestListingMode::Ignored] {
                    let mut command = util::command::new_command(&executable);
                    command.args(["--list", "--format", "terse"]);
                    if mode == RustTestListingMode::Ignored {
                        command.arg("--ignored");
                    }
                    command.current_dir(&working_directory).kill_on_drop(true);
                    if let Some(environment) = request.environment.as_ref() {
                        command.envs(environment);
                    }
                    let output =
                        bounded_command_output(command, request.limits.timeout, &executor).await?;
                    if !output.status.success() {
                        bail!(
                            "Rust test harness listing exited with {}: {}",
                            output.status,
                            bounded_process_error(&output.stderr, request.limits.field_bytes)
                        );
                    }
                    if output.stdout.len() > request.limits.listing_bytes {
                        bail!("Rust test harness listing exceeded the output limit");
                    }
                    listings.push(RustTestListingCapture {
                        package_id: harness.package_id.clone(),
                        target_name: harness.target_name.clone(),
                        target_kind: harness.target_kind,
                        mode,
                        stdout: String::from_utf8(output.stdout)
                            .context("Rust test harness listing was not UTF-8")?,
                    });
                }
            }
            for (package_id, target_name) in request.doctest_targets {
                let mut command = util::command::new_command("cargo");
                command
                    .args(["test", "--offline", "--doc", "--package"])
                    .arg(&package_id)
                    .args(["--manifest-path"])
                    .arg(&manifest_path)
                    .args(["--", "--list", "--format", "terse"])
                    .current_dir(&working_directory)
                    .kill_on_drop(true);
                if let Some(environment) = request.environment.as_ref() {
                    command.envs(environment);
                }
                let output =
                    bounded_command_output(command, request.limits.timeout, &executor).await?;
                if output.status.success() {
                    if output.stdout.len() > request.limits.listing_bytes {
                        bail!("Rust doctest listing exceeded the output limit");
                    }
                    listings.push(RustTestListingCapture {
                        package_id,
                        target_name,
                        target_kind: RustTestTargetKind::Doctest,
                        mode: RustTestListingMode::All,
                        stdout: String::from_utf8(output.stdout)
                            .context("Rust doctest listing was not UTF-8")?,
                    });
                }
            }
            Ok(RustTestDiscoveryCapture {
                toolchain: "host".to_string(),
                cargo_messages,
                listings,
            })
        })
    }
}

async fn bounded_command_output(
    mut command: util::command::Command,
    timeout: Duration,
    executor: &BackgroundExecutor,
) -> Result<Output> {
    let output = futures::FutureExt::boxed(command.output());
    let timeout = futures::FutureExt::boxed(executor.timer(timeout));
    match futures::future::select(output, timeout).await {
        futures::future::Either::Left((output, _)) => {
            output.context("failed to start Rust test discovery command")
        }
        futures::future::Either::Right(_) => bail!("Rust test discovery timed out"),
    }
}

fn bounded_process_error(bytes: &[u8], limit: usize) -> String {
    bounded_field(&String::from_utf8_lossy(bytes), limit)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustTestProtocolLimits {
    pub timeout: Duration,
    pub cargo_bytes: usize,
    pub listing_bytes: usize,
    pub lines: usize,
    pub harnesses: usize,
    pub cases: usize,
    pub field_bytes: usize,
    pub diagnostics: usize,
}

impl Default for RustTestProtocolLimits {
    fn default() -> Self {
        Self {
            timeout: RUST_TEST_DISCOVERY_TIMEOUT,
            cargo_bytes: MAX_RUST_TEST_CARGO_BYTES,
            listing_bytes: MAX_RUST_TEST_LISTING_BYTES,
            lines: MAX_RUST_TEST_LINES,
            harnesses: MAX_RUST_TEST_HARNESSES,
            cases: MAX_RUST_TEST_CASES,
            field_bytes: MAX_RUST_TEST_FIELD_BYTES,
            diagnostics: MAX_RUST_TEST_DIAGNOSTICS,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RustTestProtocolAdapter {
    limits: RustTestProtocolLimits,
}

impl RustTestProtocolAdapter {
    pub fn new(limits: RustTestProtocolLimits) -> Self {
        Self { limits }
    }

    pub fn adapt(
        &self,
        capture: RustTestDiscoveryCapture,
        source_hints: &[RustTestSourceHint],
    ) -> RustTestProtocolSnapshot {
        let mut diagnostics = Vec::new();
        let mut truncated = false;
        let mut harnesses =
            self.parse_cargo_messages(&capture.cargo_messages, &mut diagnostics, &mut truncated);
        self.add_doctest_harnesses(&capture.listings, &mut harnesses, &mut diagnostics);
        let mut cases = self.parse_listings(
            &capture.listings,
            &harnesses,
            source_hints,
            &mut diagnostics,
            &mut truncated,
        );
        harnesses.sort_by(|left, right| left.id.cmp(&right.id));
        cases.sort_by(|left, right| left.id.cmp(&right.id));
        if capture.toolchain.len() > self.limits.field_bytes {
            truncated = true;
            push_diagnostic(
                &mut diagnostics,
                self.limits,
                RustTestProtocolStage::Limits,
                "Rust toolchain descriptor exceeded the field limit".to_string(),
            );
        }
        let toolchain = bounded_field(&capture.toolchain, self.limits.field_bytes);
        let capability = if harnesses.is_empty() && cases.is_empty() {
            RustTestProtocolCapability::Unsupported
        } else if diagnostics.is_empty() && !truncated {
            RustTestProtocolCapability::Supported
        } else {
            RustTestProtocolCapability::Partial
        };
        RustTestProtocolSnapshot {
            protocol_version: RUST_TEST_PROTOCOL_VERSION,
            toolchain,
            capability,
            harnesses,
            cases,
            diagnostics,
            truncated,
        }
    }

    fn parse_cargo_messages(
        &self,
        input: &str,
        diagnostics: &mut Vec<RustTestProtocolDiagnostic>,
        truncated: &mut bool,
    ) -> Vec<RustTestHarness> {
        if input.len() > self.limits.cargo_bytes {
            push_diagnostic(
                diagnostics,
                self.limits,
                RustTestProtocolStage::Limits,
                "Cargo JSON byte limit reached".to_string(),
            );
        }
        let input = bounded_input(input, self.limits.cargo_bytes, truncated);
        let mut harnesses = BTreeMap::new();
        for (line_index, line) in input.lines().take(self.limits.lines).enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let value = match serde_json::from_str::<serde_json::Value>(line) {
                Ok(value) => value,
                Err(error) => {
                    push_diagnostic(
                        diagnostics,
                        self.limits,
                        RustTestProtocolStage::CargoMessages,
                        format!("Cargo JSON line {} is malformed: {error}", line_index + 1),
                    );
                    continue;
                }
            };
            let reason = value.get("reason").and_then(serde_json::Value::as_str);
            if !matches!(
                reason,
                Some(
                    "compiler-artifact"
                        | "compiler-message"
                        | "build-script-executed"
                        | "build-finished"
                )
            ) {
                push_diagnostic(
                    diagnostics,
                    self.limits,
                    RustTestProtocolStage::CargoMessages,
                    format!(
                        "Cargo JSON line {} has an unknown record kind",
                        line_index + 1
                    ),
                );
                continue;
            }
            let message = match serde_json::from_value::<Message>(value) {
                Ok(message) => message,
                Err(error) => {
                    push_diagnostic(
                        diagnostics,
                        self.limits,
                        RustTestProtocolStage::CargoMessages,
                        format!("Cargo JSON line {} is incomplete: {error}", line_index + 1),
                    );
                    continue;
                }
            };
            match message {
                Message::CompilerArtifact(artifact) => {
                    if !artifact.profile.test {
                        continue;
                    }
                    let Some(executable) = artifact.executable else {
                        continue;
                    };
                    let Some(target_kind) = target_kind(&artifact.target.kind) else {
                        push_diagnostic(
                            diagnostics,
                            self.limits,
                            RustTestProtocolStage::CargoMessages,
                            format!(
                                "test artifact {} has an unsupported target kind",
                                artifact.target.name
                            ),
                        );
                        continue;
                    };
                    let package_id =
                        bounded_field(&artifact.package_id.repr, self.limits.field_bytes);
                    let target_name = bounded_field(&artifact.target.name, self.limits.field_bytes);
                    let id = harness_id(&package_id, &target_name, target_kind);
                    harnesses.insert(
                        id.clone(),
                        RustTestHarness {
                            id,
                            package_id,
                            target_name,
                            target_kind,
                            locator: RustTestHarnessLocator::Executable(
                                executable.into_std_path_buf(),
                            ),
                        },
                    );
                    if harnesses.len() >= self.limits.harnesses {
                        *truncated = true;
                        push_diagnostic(
                            diagnostics,
                            self.limits,
                            RustTestProtocolStage::Limits,
                            "Rust test harness limit reached".to_string(),
                        );
                        break;
                    }
                }
                Message::BuildFinished(result) if !result.success => push_diagnostic(
                    diagnostics,
                    self.limits,
                    RustTestProtocolStage::CargoMessages,
                    "Cargo reported an unsuccessful test-harness build".to_string(),
                ),
                Message::CompilerMessage(_)
                | Message::BuildScriptExecuted(_)
                | Message::BuildFinished(_) => {}
                _ => push_diagnostic(
                    diagnostics,
                    self.limits,
                    RustTestProtocolStage::CargoMessages,
                    "Cargo emitted a message unsupported by this protocol version".to_string(),
                ),
            }
        }
        if input.lines().count() > self.limits.lines {
            *truncated = true;
            push_diagnostic(
                diagnostics,
                self.limits,
                RustTestProtocolStage::Limits,
                "Cargo JSON line limit reached".to_string(),
            );
        }
        harnesses.into_values().collect()
    }

    fn add_doctest_harnesses(
        &self,
        listings: &[RustTestListingCapture],
        harnesses: &mut Vec<RustTestHarness>,
        diagnostics: &mut Vec<RustTestProtocolDiagnostic>,
    ) {
        let mut existing = harnesses
            .iter()
            .map(|harness| harness.id.clone())
            .collect::<BTreeSet<_>>();
        for listing in listings {
            if listing.target_kind != RustTestTargetKind::Doctest {
                continue;
            }
            let package_id = bounded_field(&listing.package_id, self.limits.field_bytes);
            let target_name = bounded_field(&listing.target_name, self.limits.field_bytes);
            let id = harness_id(&package_id, &target_name, RustTestTargetKind::Doctest);
            if existing.insert(id.clone()) {
                if harnesses.len() >= self.limits.harnesses {
                    push_diagnostic(
                        diagnostics,
                        self.limits,
                        RustTestProtocolStage::Limits,
                        "Rust test harness limit reached before doctest discovery".to_string(),
                    );
                    return;
                }
                harnesses.push(RustTestHarness {
                    id,
                    package_id,
                    target_name,
                    target_kind: RustTestTargetKind::Doctest,
                    locator: RustTestHarnessLocator::CargoDoctest,
                });
            }
        }
    }

    fn parse_listings(
        &self,
        listings: &[RustTestListingCapture],
        harnesses: &[RustTestHarness],
        source_hints: &[RustTestSourceHint],
        diagnostics: &mut Vec<RustTestProtocolDiagnostic>,
        truncated: &mut bool,
    ) -> Vec<RustTestCaseRecord> {
        let harnesses = harnesses
            .iter()
            .map(|harness| {
                (
                    (
                        harness.package_id.as_str(),
                        harness.target_name.as_str(),
                        harness.target_kind,
                    ),
                    harness,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let hints = source_hints
            .iter()
            .map(|hint| {
                (
                    (
                        hint.package_id.as_str(),
                        hint.target_name.as_str(),
                        hint.target_kind,
                        hint.test_name.as_str(),
                    ),
                    hint,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut cases = BTreeMap::new();
        let mut total_bytes = 0usize;
        let mut total_lines = 0usize;
        for listing in listings {
            total_bytes = total_bytes.saturating_add(listing.stdout.len());
            if total_bytes > self.limits.listing_bytes {
                *truncated = true;
                push_diagnostic(
                    diagnostics,
                    self.limits,
                    RustTestProtocolStage::Limits,
                    "Rust test listing byte limit reached".to_string(),
                );
                break;
            }
            let Some(harness) = harnesses.get(&(
                listing.package_id.as_str(),
                listing.target_name.as_str(),
                listing.target_kind,
            )) else {
                push_diagnostic(
                    diagnostics,
                    self.limits,
                    RustTestProtocolStage::HarnessListing,
                    format!(
                        "listing for {}:{} has no structured Cargo harness",
                        listing.package_id, listing.target_name
                    ),
                );
                continue;
            };
            for (line_index, line) in listing.stdout.lines().enumerate() {
                total_lines = total_lines.saturating_add(1);
                if total_lines > self.limits.lines || cases.len() >= self.limits.cases {
                    *truncated = true;
                    push_diagnostic(
                        diagnostics,
                        self.limits,
                        RustTestProtocolStage::Limits,
                        "Rust test case or line limit reached".to_string(),
                    );
                    break;
                }
                let line = line.trim();
                if line.is_empty() || is_listing_summary(line) {
                    continue;
                }
                let Some((name, kind)) = parse_listing_line(line) else {
                    push_diagnostic(
                        diagnostics,
                        self.limits,
                        RustTestProtocolStage::HarnessListing,
                        format!(
                            "{} listing line {} has an unknown record",
                            harness.target_name,
                            line_index + 1
                        ),
                    );
                    continue;
                };
                if name.len() > self.limits.field_bytes {
                    *truncated = true;
                    push_diagnostic(
                        diagnostics,
                        self.limits,
                        RustTestProtocolStage::Limits,
                        format!(
                            "{} emitted a test name over the field limit",
                            harness.target_name
                        ),
                    );
                }
                let name = bounded_field(name, self.limits.field_bytes);
                if name.is_empty() {
                    push_diagnostic(
                        diagnostics,
                        self.limits,
                        RustTestProtocolStage::HarnessListing,
                        format!("{} emitted an empty test name", harness.target_name),
                    );
                    continue;
                }
                let id = case_id(&harness.id, &name, kind);
                let hint = hints.get(&(
                    harness.package_id.as_str(),
                    harness.target_name.as_str(),
                    harness.target_kind,
                    name.as_str(),
                ));
                let source = hint.and_then(|hint| hint.source.clone());
                let runnable = hint
                    .and_then(|hint| hint.runnable.clone())
                    .unwrap_or_else(|| default_runnable(harness, &name));
                cases
                    .entry(id.clone())
                    .and_modify(|record: &mut RustTestCaseRecord| {
                        if listing.mode == RustTestListingMode::Ignored {
                            record.ignored = true;
                        }
                        if record.source.is_none() {
                            record.source = source.clone();
                        }
                    })
                    .or_insert(RustTestCaseRecord {
                        id,
                        harness_id: harness.id.clone(),
                        name,
                        kind,
                        ignored: listing.mode == RustTestListingMode::Ignored,
                        source,
                        runnable,
                    });
            }
            if *truncated {
                break;
            }
        }
        cases.into_values().collect()
    }
}

fn target_kind(kinds: &[TargetKind]) -> Option<RustTestTargetKind> {
    if kinds.contains(&TargetKind::Test) {
        Some(RustTestTargetKind::Integration)
    } else if kinds.contains(&TargetKind::Bench) {
        Some(RustTestTargetKind::Benchmark)
    } else if kinds.contains(&TargetKind::Example) {
        Some(RustTestTargetKind::Example)
    } else if kinds.contains(&TargetKind::Bin) {
        Some(RustTestTargetKind::Binary)
    } else if kinds.contains(&TargetKind::Lib) {
        Some(RustTestTargetKind::Unit)
    } else {
        None
    }
}

fn parse_listing_line(line: &str) -> Option<(&str, RustTestCaseKind)> {
    let (name, kind) = line.rsplit_once(": ")?;
    match kind {
        "test" => Some((name, RustTestCaseKind::Test)),
        "benchmark" => Some((name, RustTestCaseKind::Benchmark)),
        _ => None,
    }
}

fn is_listing_summary(line: &str) -> bool {
    line.split_whitespace().all(|part| {
        part.parse::<usize>().is_ok()
            || matches!(
                part.trim_end_matches(','),
                "test" | "tests" | "benchmark" | "benchmarks"
            )
    })
}

fn default_runnable(harness: &RustTestHarness, test_name: &str) -> RustTestRunnableSelector {
    let mut cargo_args = vec![
        "test".to_string(),
        "--package".to_string(),
        harness.package_id.clone(),
    ];
    match harness.target_kind {
        RustTestTargetKind::Unit => cargo_args.push("--lib".to_string()),
        RustTestTargetKind::Integration => {
            cargo_args.extend(["--test".to_string(), harness.target_name.clone()]);
        }
        RustTestTargetKind::Binary => {
            cargo_args.extend(["--bin".to_string(), harness.target_name.clone()]);
        }
        RustTestTargetKind::Example => {
            cargo_args.extend(["--example".to_string(), harness.target_name.clone()]);
        }
        RustTestTargetKind::Benchmark => {
            cargo_args[0] = "bench".to_string();
            cargo_args.extend(["--bench".to_string(), harness.target_name.clone()]);
        }
        RustTestTargetKind::Doctest => cargo_args.push("--doc".to_string()),
    }
    RustTestRunnableSelector {
        cargo_args,
        executable_args: vec![test_name.to_string(), "--exact".to_string()],
    }
}

fn harness_id(package_id: &str, target_name: &str, kind: RustTestTargetKind) -> String {
    format!(
        "rust-harness:v1:{}:{}:{}:{}:{kind:?}",
        package_id.len(),
        package_id,
        target_name.len(),
        target_name
    )
}

fn case_id(harness_id: &str, name: &str, kind: RustTestCaseKind) -> String {
    format!(
        "rust-case:v1:{}:{}:{}:{}:{kind:?}",
        harness_id.len(),
        harness_id,
        name.len(),
        name
    )
}

fn bounded_input<'a>(input: &'a str, limit: usize, truncated: &mut bool) -> &'a str {
    if input.len() <= limit {
        return input;
    }
    *truncated = true;
    let mut end = limit;
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    &input[..end]
}

fn bounded_field(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_string();
    }
    let mut end = limit;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn push_diagnostic(
    diagnostics: &mut Vec<RustTestProtocolDiagnostic>,
    limits: RustTestProtocolLimits,
    stage: RustTestProtocolStage,
    message: String,
) {
    if diagnostics.len() >= limits.diagnostics {
        return;
    }
    diagnostics.push(RustTestProtocolDiagnostic {
        stage,
        message: bounded_field(&message, limits.field_bytes),
    });
}

#[derive(Clone, Debug)]
pub enum RustTestProviderStoreEvent {
    Invalidated,
    Changed(DiscoveryGeneration),
}

impl EventEmitter<RustTestProviderStoreEvent> for RustTestProviderStore {}

enum RustTestProviderStoreMode {
    Local {
        runner: Arc<dyn RustTestDiscoveryRunner>,
        cargo_store: Entity<CargoWorkspaceStore>,
        structured_store: Entity<StructuredExecutionStore>,
        environment: Entity<ProjectEnvironment>,
    },
    Remote {
        project_id: u64,
        client: AnyProtoClient,
        structured_store: Entity<StructuredExecutionStore>,
    },
}

pub struct RustTestProviderStore {
    mode: RustTestProviderStoreMode,
    worktree_store: Entity<WorktreeStore>,
    generation: u64,
    next_run_id: u64,
    source_hints: Vec<RustTestSourceHint>,
    actions: HashMap<StructuredNodeId, RustTestActionDescriptor>,
    run_handles: HashMap<StructuredRunId, StructuredTaskHandle>,
    refresh_task: Task<()>,
    invalidation_task: Task<()>,
    active_remote_request_id: Option<u64>,
    active_remote_requests: HashMap<(proto::PeerId, u64), AbortHandle>,
    cancelled_remote_requests: HashSet<(proto::PeerId, u64)>,
    authorized_remote_runs: HashMap<(proto::PeerId, StructuredRunId), AuthorizedRemoteRun>,
    remote_lifecycle_tx: Option<mpsc::UnboundedSender<proto::UpdateRustTestRun>>,
    _remote_lifecycle_task: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl RustTestProviderStore {
    pub fn init(client: &AnyProtoClient) {
        client.add_entity_request_handler(Self::handle_get_discovery);
        client.add_entity_request_handler(Self::handle_cancel_discovery);
        client.add_entity_request_handler(Self::handle_resolve_action);
        client.add_entity_request_handler(Self::handle_update_run);
    }

    pub fn local(
        worktree_store: Entity<WorktreeStore>,
        environment: Entity<ProjectEnvironment>,
        cargo_store: Entity<CargoWorkspaceStore>,
        structured_store: Entity<StructuredExecutionStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::local_with_runner(
            worktree_store,
            environment,
            cargo_store,
            structured_store,
            Arc::new(ProcessRustTestDiscoveryRunner {
                executor: cx.background_executor().clone(),
            }),
            cx,
        )
    }

    pub fn local_with_runner(
        worktree_store: Entity<WorktreeStore>,
        environment: Entity<ProjectEnvironment>,
        cargo_store: Entity<CargoWorkspaceStore>,
        structured_store: Entity<StructuredExecutionStore>,
        runner: Arc<dyn RustTestDiscoveryRunner>,
        cx: &mut Context<Self>,
    ) -> Self {
        let cargo_subscription = cx.subscribe(&cargo_store, |store, _, _, cx| {
            store.schedule_invalidation(cx)
        });
        let worktree_subscription = cx.subscribe(&worktree_store, |store, _, event, cx| {
            if rust_test_input_changed(event) {
                store.schedule_invalidation(cx);
            }
        });
        let mut subscriptions = vec![cargo_subscription, worktree_subscription];
        if let Some(trusted_worktrees) = TrustedWorktrees::try_get_global(cx) {
            subscriptions.push(cx.subscribe(&trusted_worktrees, |store, _, _, cx| {
                store.schedule_invalidation(cx)
            }));
        }
        Self {
            mode: RustTestProviderStoreMode::Local {
                runner,
                cargo_store,
                structured_store,
                environment,
            },
            worktree_store,
            generation: 0,
            next_run_id: 0,
            source_hints: Vec::new(),
            actions: HashMap::default(),
            run_handles: HashMap::default(),
            refresh_task: Task::ready(()),
            invalidation_task: Task::ready(()),
            active_remote_request_id: None,
            active_remote_requests: HashMap::default(),
            cancelled_remote_requests: HashSet::new(),
            authorized_remote_runs: HashMap::default(),
            remote_lifecycle_tx: None,
            _remote_lifecycle_task: Task::ready(()),
            _subscriptions: subscriptions,
        }
    }

    pub fn remote(
        project_id: u64,
        client: AnyProtoClient,
        worktree_store: Entity<WorktreeStore>,
        structured_store: Entity<StructuredExecutionStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        let worktree_subscription = cx.subscribe(&worktree_store, |store, _, event, cx| {
            if rust_test_input_changed(event) {
                store.schedule_invalidation(cx);
            }
        });
        let (remote_lifecycle_tx, mut remote_lifecycle_rx) = mpsc::unbounded();
        let lifecycle_client = client.clone();
        let remote_lifecycle_task = cx.spawn(async move |_, _| {
            while let Some(request) = remote_lifecycle_rx.next().await {
                if let Err(error) = lifecycle_client.request(request).await {
                    log::warn!("Failed to publish remote Rust test lifecycle: {error:#}");
                }
            }
        });
        Self {
            mode: RustTestProviderStoreMode::Remote {
                project_id,
                client,
                structured_store,
            },
            worktree_store,
            generation: 0,
            next_run_id: 0,
            source_hints: Vec::new(),
            actions: HashMap::default(),
            run_handles: HashMap::default(),
            refresh_task: Task::ready(()),
            invalidation_task: Task::ready(()),
            active_remote_request_id: None,
            active_remote_requests: HashMap::default(),
            cancelled_remote_requests: HashSet::new(),
            authorized_remote_runs: HashMap::default(),
            remote_lifecycle_tx: Some(remote_lifecycle_tx),
            _remote_lifecycle_task: remote_lifecycle_task,
            _subscriptions: vec![worktree_subscription],
        }
    }

    pub fn provider_id() -> StructuredProviderId {
        StructuredProviderId(RUST_TEST_PROVIDER_ID.to_string())
    }

    pub fn generation(&self) -> DiscoveryGeneration {
        DiscoveryGeneration(self.generation)
    }

    pub fn plan_action(
        &mut self,
        node_id: &StructuredNodeId,
        discovery_generation: DiscoveryGeneration,
        action: RustTestAction,
        cx: &mut Context<Self>,
    ) -> Task<Result<RustTestActionPlan>> {
        match &self.mode {
            RustTestProviderStoreMode::Local { .. } => {
                Task::ready(self.plan_local_action(node_id, discovery_generation, action))
            }
            RustTestProviderStoreMode::Remote {
                project_id, client, ..
            } => {
                let request = proto::ResolveRustTestAction {
                    project_id: *project_id,
                    protocol_version: RUST_TEST_DISCOVERY_PROTOCOL_VERSION,
                    discovery_generation: discovery_generation.0,
                    node_id: node_id.0.clone(),
                    action: action_to_proto(action),
                    worktree_ids: visible_worktree_ids(&self.worktree_store, cx),
                };
                let client = client.clone();
                cx.background_spawn(async move {
                    let response = client
                        .request(request)
                        .await
                        .map_err(map_remote_action_error)?;
                    decode_action_plan(response)
                })
            }
        }
    }

    fn plan_local_action(
        &mut self,
        node_id: &StructuredNodeId,
        discovery_generation: DiscoveryGeneration,
        action: RustTestAction,
    ) -> Result<RustTestActionPlan> {
        if discovery_generation != self.generation() {
            bail!("The selected Rust test belongs to a stale discovery generation");
        }
        let descriptor = self
            .actions
            .get(node_id)
            .context("The selected Rust test is not executable on this project host")?
            .clone();
        self.next_run_id = self.next_run_id.wrapping_add(1);
        plan_rust_test_action(
            &descriptor,
            node_id,
            discovery_generation,
            self.next_run_id,
            action,
        )
    }

    pub fn register_run_handle(
        &mut self,
        run_id: StructuredRunId,
        discovery_generation: DiscoveryGeneration,
        scope_node_ids: Vec<StructuredNodeId>,
        handle: StructuredTaskHandle,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        match &self.mode {
            RustTestProviderStoreMode::Local {
                structured_store, ..
            } => {
                structured_store.update(cx, |store, cx| {
                    let project_generation = store.state().project_generation();
                    store.state_mut().begin_run(
                        project_generation,
                        &Self::provider_id(),
                        crate::structured_execution::StructuredRun::new(
                            run_id.clone(),
                            discovery_generation,
                            scope_node_ids.clone(),
                        ),
                    )?;
                    store.observe_task_handle(Self::provider_id(), run_id.clone(), &handle, cx)?;
                    cx.emit(StructuredExecutionStoreEvent::Changed(Self::provider_id()));
                    anyhow::Ok(())
                })?;
            }
            RustTestProviderStoreMode::Remote { project_id, .. } => {
                let lifecycle_tx = self
                    .remote_lifecycle_tx
                    .clone()
                    .context("The remote Rust test lifecycle channel is unavailable")?;
                let project_id = *project_id;
                let worktree_ids = visible_worktree_ids(&self.worktree_store, cx);
                let run_id = run_id.clone();
                handle.subscribe(cx, move |event, _| {
                    let request = lifecycle_to_proto(
                        project_id,
                        discovery_generation,
                        &run_id,
                        &scope_node_ids,
                        event,
                        worktree_ids.clone(),
                    );
                    if let Err(error) = lifecycle_tx.unbounded_send(request) {
                        log::warn!("Failed to queue remote Rust test lifecycle: {error}");
                    }
                });
            }
        }
        if self.run_handles.len() >= MAX_RUST_TEST_RERUNS * 2 {
            self.run_handles
                .retain(|_, handle| !handle.state().is_terminal());
        }
        self.run_handles.insert(run_id, handle);
        Ok(())
    }

    pub fn cancel_run(&self, run_id: &StructuredRunId, cx: &mut App) -> Result<()> {
        let handle = self
            .run_handles
            .get(run_id)
            .context("The selected Rust test run is no longer active")?;
        if !handle.cancel(cx) {
            bail!("The selected Rust test run is already complete");
        }
        Ok(())
    }

    pub fn reveal_run_terminal(
        &self,
        run_id: &StructuredRunId,
        window: &mut gpui::Window,
        cx: &mut App,
    ) -> Result<()> {
        let handle = self
            .run_handles
            .get(run_id)
            .context("The selected Rust test run has no retained task terminal")?;
        if !handle.reveal_terminal(window, cx) {
            bail!("The selected Rust test run has no available task terminal");
        }
        Ok(())
    }

    pub fn set_source_hints(
        &mut self,
        source_hints: Vec<RustTestSourceHint>,
        cx: &mut Context<Self>,
    ) {
        self.source_hints = source_hints;
        self.schedule_invalidation(cx);
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) -> Task<Result<StructuredProviderSnapshot>> {
        self.refresh_scoped(None, cx)
    }

    fn refresh_scoped(
        &mut self,
        allowed_worktrees: Option<HashSet<WorktreeId>>,
        cx: &mut Context<Self>,
    ) -> Task<Result<StructuredProviderSnapshot>> {
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        match &self.mode {
            RustTestProviderStoreMode::Remote {
                project_id,
                client,
                structured_store,
            } => {
                let project_id = *project_id;
                let client = client.clone();
                let structured_store = structured_store.clone();
                if let Some(request_id) = self.active_remote_request_id.replace(generation) {
                    cx.background_spawn({
                        let client = client.clone();
                        async move {
                            client
                                .request(proto::CancelRustTestDiscovery {
                                    project_id,
                                    request_id,
                                })
                                .await?;
                            anyhow::Ok(())
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
                cx.spawn(async move |this, cx| {
                    let response = client
                        .request(proto::GetRustTestDiscovery {
                            project_id,
                            request_id: generation,
                            protocol_version: RUST_TEST_DISCOVERY_PROTOCOL_VERSION,
                            worktree_ids,
                        })
                        .await
                        .map_err(map_remote_discovery_error)?;
                    if response.protocol_version != RUST_TEST_DISCOVERY_PROTOCOL_VERSION {
                        bail!("Rust test discovery protocol mismatch");
                    }
                    if response.request_id != generation {
                        bail!("Rust test discovery response identity mismatch");
                    }
                    if response.status == proto::StructuredProviderStatus::Mismatch as i32 {
                        bail!(
                            "{}",
                            response.diagnostic.as_deref().unwrap_or(
                                "The project host does not support this Rust test protocol"
                            )
                        );
                    }
                    this.update(cx, |store, _| {
                        if store.active_remote_request_id == Some(generation) {
                            store.active_remote_request_id = None;
                        }
                    })?;
                    structured_store
                        .update(cx, |store, cx| {
                            store.refresh_provider(Self::provider_id(), cx)
                        })
                        .await
                })
            }
            RustTestProviderStoreMode::Local {
                runner,
                cargo_store,
                structured_store,
                environment,
            } => {
                let runner = runner.clone();
                let cargo_refresh = cargo_store.update(cx, |store, cx| store.refresh(cx));
                let structured_store = structured_store.clone();
                let environment = environment.clone();
                let worktree_store = self.worktree_store.clone();
                let source_hints = self.source_hints.clone();
                cx.spawn(async move |this, cx| {
                    let cargo_snapshot =
                        scoped_cargo_snapshot(cargo_refresh.await?, allowed_worktrees.as_ref());
                    let prepared = cx.update(|cx| {
                        prepare_discovery_requests(
                            &cargo_snapshot,
                            &worktree_store,
                            &environment,
                            generation,
                            cx,
                        )
                    });
                    let mut captures = Vec::new();
                    let mut failures = Vec::new();
                    for (workspace, mut request, environment) in prepared {
                        request.environment = environment.await;
                        match runner.discover(request).await {
                            Ok(capture) => captures.push((workspace, capture)),
                            Err(error) => failures.push(error.to_string()),
                        }
                    }
                    let projection = project_provider_projection(
                        &cargo_snapshot,
                        captures,
                        &source_hints,
                        failures,
                        DiscoveryGeneration(generation),
                    );
                    let mut snapshot = projection.snapshot;
                    let previous = structured_store.read_with(cx, |store, _| {
                        store.state().provider(&Self::provider_id()).cloned()
                    });
                    retain_stale_discovery(&mut snapshot, previous.as_ref());
                    this.update(cx, |store, _| {
                        if store.generation != generation {
                            bail!("stale Rust test discovery generation");
                        }
                        Ok(())
                    })??;
                    structured_store.update(cx, |store, cx| {
                        let project_generation = store.state().project_generation();
                        store.state_mut().apply_discovery(
                            project_generation,
                            snapshot.clone(),
                            allowed_worktrees.as_ref(),
                        )?;
                        cx.emit(StructuredExecutionStoreEvent::Changed(Self::provider_id()));
                        anyhow::Ok(())
                    })?;
                    this.update(cx, |store, cx| {
                        store.actions = projection.actions;
                        store.authorized_remote_runs.retain(|_, authorization| {
                            authorization.discovery_generation == DiscoveryGeneration(generation)
                        });
                        cx.emit(RustTestProviderStoreEvent::Changed(DiscoveryGeneration(
                            generation,
                        )))
                    })?;
                    Ok(snapshot)
                })
            }
        }
    }

    fn schedule_invalidation(&mut self, cx: &mut Context<Self>) {
        cx.emit(RustTestProviderStoreEvent::Invalidated);
        self.invalidation_task = cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(150))
                .await;
            if let Err(error) = this.update(cx, |store, cx| {
                let refresh = store.refresh(cx);
                store.refresh_task = cx.spawn(async move |_, _| {
                    if let Err(error) = refresh.await {
                        log::warn!("Rust test discovery refresh failed: {error:#}");
                    }
                });
            }) {
                log::debug!("Rust test provider was dropped before refresh: {error:#}");
            }
        });
    }

    async fn handle_get_discovery(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::GetRustTestDiscovery>,
        mut cx: AsyncApp,
    ) -> Result<proto::RustTestDiscoveryResponse> {
        let request = envelope.payload;
        if request.protocol_version != RUST_TEST_DISCOVERY_PROTOCOL_VERSION {
            return Ok(proto::RustTestDiscoveryResponse {
                protocol_version: RUST_TEST_DISCOVERY_PROTOCOL_VERSION,
                request_id: request.request_id,
                discovery_generation: 0,
                status: proto::StructuredProviderStatus::Mismatch as i32,
                diagnostic: Some("Rust test discovery protocol mismatch".to_string()),
            });
        }
        if request.worktree_ids.len() > MAX_RUST_TEST_REQUEST_WORKTREES {
            bail!("Rust test discovery worktree scope exceeds the supported limit");
        }
        let key = (envelope.sender_id, request.request_id);
        let allowed = request
            .worktree_ids
            .into_iter()
            .map(WorktreeId::from_proto)
            .collect();
        let refresh = this.update(&mut cx, |store, cx| store.refresh_scoped(Some(allowed), cx));
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let proceed = this.update(&mut cx, |store, _| {
            if store.cancelled_remote_requests.remove(&key) {
                false
            } else {
                store.active_remote_requests.insert(key, abort_handle);
                true
            }
        });
        if !proceed {
            bail!("Rust test discovery request was cancelled");
        }
        let snapshot = Abortable::new(refresh, abort_registration)
            .await
            .map_err(|_| anyhow!("Rust test discovery request was cancelled"))??;
        this.update(&mut cx, |store, _| {
            store.active_remote_requests.remove(&key);
            store.cancelled_remote_requests.remove(&key);
        });
        Ok(proto::RustTestDiscoveryResponse {
            protocol_version: RUST_TEST_DISCOVERY_PROTOCOL_VERSION,
            request_id: request.request_id,
            discovery_generation: snapshot.discovery_generation.0,
            status: provider_status_to_proto(snapshot.status),
            diagnostic: snapshot.diagnostic,
        })
    }

    async fn handle_cancel_discovery(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::CancelRustTestDiscovery>,
        mut cx: AsyncApp,
    ) -> Result<proto::Ack> {
        this.update(&mut cx, |store, _| {
            let key = (envelope.sender_id, envelope.payload.request_id);
            if let Some(handle) = store.active_remote_requests.remove(&key) {
                handle.abort();
            } else if store.cancelled_remote_requests.len() < MAX_PENDING_RUST_TEST_CANCELLATIONS {
                store.cancelled_remote_requests.insert(key);
            } else {
                log::warn!("Ignored excess pending Rust test cancellation");
            }
        });
        Ok(proto::Ack {})
    }

    async fn handle_resolve_action(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::ResolveRustTestAction>,
        mut cx: AsyncApp,
    ) -> Result<proto::RustTestActionPlanResponse> {
        let request = envelope.payload;
        ensure_rust_test_protocol(request.protocol_version)?;
        validate_identifier_field("Rust test node ID", &request.node_id)?;
        if request.worktree_ids.len() > MAX_RUST_TEST_REQUEST_WORKTREES {
            bail!("Rust test action worktree scope exceeds the supported limit");
        }
        let allowed = request
            .worktree_ids
            .into_iter()
            .map(WorktreeId::from_proto)
            .collect::<HashSet<_>>();
        let node_id = StructuredNodeId(request.node_id);
        let action = action_from_proto(request.action)?;
        let sender_id = envelope.sender_id;
        let plan_json = this.update(&mut cx, |store, _| {
            if request.discovery_generation != store.generation {
                bail!("The selected Rust test belongs to a stale discovery generation");
            }
            let descriptor = store
                .actions
                .get(&node_id)
                .context("The selected Rust test is no longer executable")?;
            if !allowed.contains(&descriptor.worktree_id) {
                bail!("The selected Rust test is outside the visible worktree scope");
            }
            let plan = store.plan_local_action(
                &node_id,
                DiscoveryGeneration(request.discovery_generation),
                action,
            )?;
            if let RustTestActionPlan::Task {
                run_id,
                discovery_generation,
                scope_node_ids,
                worktree_id,
                ..
            } = &plan
            {
                if store.authorized_remote_runs.len() >= MAX_PENDING_REMOTE_RUST_TEST_RUNS {
                    bail!("Too many pending remote Rust test runs");
                }
                store.authorized_remote_runs.insert(
                    (sender_id, run_id.clone()),
                    AuthorizedRemoteRun {
                        discovery_generation: *discovery_generation,
                        scope_node_ids: scope_node_ids.clone(),
                        worktree_id: *worktree_id,
                    },
                );
            }
            encode_action_plan(&plan)
        })?;
        Ok(proto::RustTestActionPlanResponse {
            protocol_version: RUST_TEST_DISCOVERY_PROTOCOL_VERSION,
            plan_json,
        })
    }

    async fn handle_update_run(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::UpdateRustTestRun>,
        mut cx: AsyncApp,
    ) -> Result<proto::Ack> {
        let request = envelope.payload;
        ensure_rust_test_protocol(request.protocol_version)?;
        if request.worktree_ids.len() > MAX_RUST_TEST_REQUEST_WORKTREES {
            bail!("Rust test lifecycle worktree scope exceeds the supported limit");
        }
        let allowed = request
            .worktree_ids
            .iter()
            .copied()
            .map(WorktreeId::from_proto)
            .collect::<HashSet<_>>();
        let run_id = StructuredRunId(request.run_id.clone());
        let scope_node_ids = request
            .scope_node_ids
            .iter()
            .map(|node_id| StructuredNodeId(node_id.clone()))
            .collect::<Vec<_>>();
        let event = lifecycle_from_proto(&request)?;
        let key = (envelope.sender_id, run_id.clone());
        let structured_store = this.update(&mut cx, |store, _| {
            let authorization = store
                .authorized_remote_runs
                .get(&key)
                .context("Unknown or expired remote Rust test run")?;
            if authorization.discovery_generation.0 != request.discovery_generation
                || authorization.scope_node_ids != scope_node_ids
            {
                bail!("Remote Rust test lifecycle identity mismatch");
            }
            if !allowed.contains(&authorization.worktree_id) {
                bail!("Remote Rust test lifecycle is outside the visible worktree scope");
            }
            match &store.mode {
                RustTestProviderStoreMode::Local {
                    structured_store, ..
                } => Ok(structured_store.clone()),
                RustTestProviderStoreMode::Remote { .. } => {
                    bail!("Remote Rust test lifecycle reached a non-authoritative store")
                }
            }
        })?;
        structured_store.update(&mut cx, |store, cx| {
            let provider_id = Self::provider_id();
            let current_run = store
                .state()
                .provider(&provider_id)
                .and_then(|provider| provider.current_run.as_ref());
            if current_run.is_none_or(|run| run.id != run_id) {
                if !matches!(event.state, StructuredTaskState::Queued) {
                    bail!("Remote Rust test lifecycle did not begin with a queued state");
                }
                let project_generation = store.state().project_generation();
                store.state_mut().begin_run(
                    project_generation,
                    &provider_id,
                    crate::structured_execution::StructuredRun::new(
                        run_id.clone(),
                        DiscoveryGeneration(request.discovery_generation),
                        scope_node_ids.clone(),
                    ),
                )?;
            }
            store.apply_task_lifecycle(&provider_id, &run_id, &event, cx)?;
            anyhow::Ok(())
        })?;
        if event.state.is_terminal() {
            this.update(&mut cx, |store, _| {
                store.authorized_remote_runs.remove(&key);
            });
        }
        Ok(proto::Ack {})
    }
}

fn visible_worktree_ids(worktree_store: &Entity<WorktreeStore>, cx: &App) -> Vec<u64> {
    worktree_store
        .read(cx)
        .visible_worktrees(cx)
        .map(|worktree| worktree.read(cx).id().to_proto())
        .take(MAX_RUST_TEST_REQUEST_WORKTREES)
        .collect()
}

fn ensure_rust_test_protocol(protocol_version: u32) -> Result<()> {
    if protocol_version != RUST_TEST_DISCOVERY_PROTOCOL_VERSION {
        bail!("Rust test action protocol mismatch");
    }
    Ok(())
}

fn validate_identifier_field(name: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_RUST_TEST_FIELD_BYTES || value.contains('\0') {
        bail!("{name} is invalid or exceeds the supported limit");
    }
    Ok(())
}

fn action_to_proto(action: RustTestAction) -> i32 {
    (match action {
        RustTestAction::Run => proto::RustTestActionKind::Run,
        RustTestAction::Debug => proto::RustTestActionKind::Debug,
    }) as i32
}

fn action_from_proto(action: i32) -> Result<RustTestAction> {
    match proto::RustTestActionKind::from_i32(action) {
        Some(proto::RustTestActionKind::Run) => Ok(RustTestAction::Run),
        Some(proto::RustTestActionKind::Debug) => Ok(RustTestAction::Debug),
        _ => bail!("Unsupported Rust test action"),
    }
}

fn encode_action_plan(plan: &RustTestActionPlan) -> Result<String> {
    let wire_plan = match plan {
        RustTestActionPlan::Task {
            run_id,
            discovery_generation,
            scope_node_ids,
            worktree_id,
            template,
        } => RustTestActionWirePlan::Task {
            run_id: run_id.0.clone(),
            discovery_generation: discovery_generation.0,
            scope_node_ids: scope_node_ids
                .iter()
                .map(|node_id| node_id.0.clone())
                .collect(),
            worktree_id: worktree_id.to_proto(),
            template: template.clone(),
        },
        RustTestActionPlan::Debug {
            worktree_id,
            scenario,
        } => {
            let Some(BuildTaskDefinition::Template {
                task_template,
                locator_name,
            }) = &scenario.build
            else {
                bail!("Rust test debug plan has no Cargo build template");
            };
            if locator_name.as_deref() != Some("rust-cargo-locator") {
                bail!("Rust test debug plan has an unsupported locator");
            }
            RustTestActionWirePlan::Debug {
                worktree_id: worktree_id.to_proto(),
                build_template: task_template.clone(),
            }
        }
    };
    let serialized = serde_json::to_string(&wire_plan)?;
    if serialized.len() > MAX_RUST_TEST_ACTION_PLAN_BYTES {
        bail!("Rust test action plan exceeds the supported limit");
    }
    Ok(serialized)
}

fn decode_action_plan(response: proto::RustTestActionPlanResponse) -> Result<RustTestActionPlan> {
    ensure_rust_test_protocol(response.protocol_version)?;
    if response.plan_json.len() > MAX_RUST_TEST_ACTION_PLAN_BYTES {
        bail!("Remote Rust test action plan exceeds the supported limit");
    }
    let wire_plan = serde_json::from_str::<RustTestActionWirePlan>(&response.plan_json)
        .context("Remote Rust test action plan was malformed")?;
    match wire_plan {
        RustTestActionWirePlan::Task {
            run_id,
            discovery_generation,
            scope_node_ids,
            worktree_id,
            template,
        } => {
            validate_identifier_field("Rust test run ID", &run_id)?;
            if scope_node_ids.is_empty() || scope_node_ids.len() > MAX_RUST_TEST_RERUNS {
                bail!("Remote Rust test action scope exceeds the supported limit");
            }
            for node_id in &scope_node_ids {
                validate_identifier_field("Rust test node ID", node_id)?;
            }
            validate_action_template(&template)?;
            Ok(RustTestActionPlan::Task {
                run_id: StructuredRunId(run_id),
                discovery_generation: DiscoveryGeneration(discovery_generation),
                scope_node_ids: scope_node_ids.into_iter().map(StructuredNodeId).collect(),
                worktree_id: WorktreeId::from_proto(worktree_id),
                template,
            })
        }
        RustTestActionWirePlan::Debug {
            worktree_id,
            build_template,
        } => {
            validate_action_template(&build_template)?;
            Ok(RustTestActionPlan::Debug {
                worktree_id: WorktreeId::from_proto(worktree_id),
                scenario: rust_test_debug_scenario_from_build(build_template),
            })
        }
    }
}

fn validate_action_template(template: &TaskTemplate) -> Result<()> {
    if template.command != "cargo"
        || template.args.len() > MAX_RUST_TEST_ACTION_ARGUMENTS
        || !template.env.is_empty()
        || !template.hooks.is_empty()
    {
        bail!("Remote Rust test action plan contains unsupported task fields");
    }
    validate_identifier_field("Rust test task label", &template.label)?;
    let mut bytes = template.label.len() + template.command.len();
    for argument in &template.args {
        if argument.len() > MAX_RUST_TEST_FIELD_BYTES || argument.contains('\0') {
            bail!("Remote Rust test action argument exceeds the supported limit");
        }
        bytes = bytes.saturating_add(argument.len());
    }
    if let Some(cwd) = &template.cwd {
        if cwd.len() > MAX_RUST_TEST_FIELD_BYTES
            || cwd.contains('\0')
            || cwd.split('/').any(|component| component == "..")
        {
            bail!("Remote Rust test action working directory is unsafe");
        }
        bytes = bytes.saturating_add(cwd.len());
    }
    if bytes > MAX_RUST_TEST_ACTION_PLAN_BYTES {
        bail!("Remote Rust test action plan exceeds the supported limit");
    }
    Ok(())
}

fn lifecycle_to_proto(
    project_id: u64,
    discovery_generation: DiscoveryGeneration,
    run_id: &StructuredRunId,
    scope_node_ids: &[StructuredNodeId],
    event: &StructuredTaskLifecycleEvent,
    worktree_ids: Vec<u64>,
) -> proto::UpdateRustTestRun {
    let (state, terminal_id, exit_code, success, termination_confirmed, diagnostic) =
        match &event.state {
            StructuredTaskState::Queued => (
                proto::RustTestTaskState::Queued,
                None,
                None,
                false,
                false,
                None,
            ),
            StructuredTaskState::Running { terminal_id } => (
                proto::RustTestTaskState::Running,
                terminal_id.map(|terminal_id| terminal_id.0),
                None,
                false,
                false,
                None,
            ),
            StructuredTaskState::Completed {
                terminal_id,
                exit_code,
                success,
            } => (
                proto::RustTestTaskState::Completed,
                terminal_id.map(|terminal_id| terminal_id.0),
                *exit_code,
                *success,
                false,
                None,
            ),
            StructuredTaskState::SpawnError { message } => (
                proto::RustTestTaskState::SpawnError,
                None,
                None,
                false,
                false,
                Some(bounded_field(message, MAX_RUST_TEST_FIELD_BYTES)),
            ),
            StructuredTaskState::Cancelled {
                terminal_id,
                termination_confirmed,
            } => (
                proto::RustTestTaskState::Cancelled,
                terminal_id.map(|terminal_id| terminal_id.0),
                None,
                false,
                *termination_confirmed,
                None,
            ),
        };
    proto::UpdateRustTestRun {
        project_id,
        protocol_version: RUST_TEST_DISCOVERY_PROTOCOL_VERSION,
        discovery_generation: discovery_generation.0,
        run_id: run_id.0.clone(),
        scope_node_ids: scope_node_ids
            .iter()
            .map(|node_id| node_id.0.clone())
            .collect(),
        task_id: event.task_id.0.clone(),
        state: state as i32,
        terminal_id,
        exit_code,
        success,
        termination_confirmed,
        diagnostic,
        worktree_ids,
    }
}

fn lifecycle_from_proto(
    request: &proto::UpdateRustTestRun,
) -> Result<StructuredTaskLifecycleEvent> {
    validate_identifier_field("Rust test run ID", &request.run_id)?;
    validate_identifier_field("Rust test task ID", &request.task_id)?;
    if request.scope_node_ids.is_empty() || request.scope_node_ids.len() > MAX_RUST_TEST_RERUNS {
        bail!("Rust test lifecycle scope exceeds the supported limit");
    }
    for node_id in &request.scope_node_ids {
        validate_identifier_field("Rust test node ID", node_id)?;
    }
    let terminal_id = request.terminal_id.map(StructuredTerminalId);
    let state = match proto::RustTestTaskState::from_i32(request.state) {
        Some(proto::RustTestTaskState::Queued) => StructuredTaskState::Queued,
        Some(proto::RustTestTaskState::Running) => StructuredTaskState::Running { terminal_id },
        Some(proto::RustTestTaskState::Completed) => StructuredTaskState::Completed {
            terminal_id,
            exit_code: request.exit_code,
            success: request.success,
        },
        Some(proto::RustTestTaskState::SpawnError) => StructuredTaskState::SpawnError {
            message: bounded_field(
                request
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Remote Rust test task failed to start"),
                MAX_RUST_TEST_FIELD_BYTES,
            ),
        },
        Some(proto::RustTestTaskState::Cancelled) => StructuredTaskState::Cancelled {
            terminal_id,
            termination_confirmed: request.termination_confirmed,
        },
        _ => bail!("Unsupported remote Rust test lifecycle state"),
    };
    Ok(StructuredTaskLifecycleEvent {
        task_id: TaskId(request.task_id.clone()),
        state,
    })
}

fn map_remote_action_error(error: anyhow::Error) -> anyhow::Error {
    let unsupported = error
        .downcast_ref::<proto::RpcError>()
        .is_some_and(|error| error.raw_message().contains("was not handled"));
    if unsupported {
        anyhow!("The project host does not support Rust test actions")
    } else {
        anyhow!("The Rust test action host rejected the request: {error}")
    }
}

fn plan_rust_test_action(
    descriptor: &RustTestActionDescriptor,
    node_id: &StructuredNodeId,
    discovery_generation: DiscoveryGeneration,
    run_number: u64,
    action: RustTestAction,
) -> Result<RustTestActionPlan> {
    let template = rust_test_task_template(descriptor);
    match action {
        RustTestAction::Run => Ok(RustTestActionPlan::Task {
            run_id: StructuredRunId(format!(
                "rust-test-run-{}-{run_number}",
                discovery_generation.0
            )),
            discovery_generation,
            scope_node_ids: vec![node_id.clone()],
            worktree_id: descriptor.worktree_id,
            template,
        }),
        RustTestAction::Debug => {
            if descriptor.scope != RustTestActionScope::Case {
                bail!("Debug is available for individual Rust test cases only");
            }
            match descriptor.target_kind {
                Some(RustTestTargetKind::Doctest) => {
                    bail!("Cargo DAP does not support debugging doctests")
                }
                Some(RustTestTargetKind::Benchmark) => {
                    bail!("Cargo DAP does not support debugging an individual benchmark")
                }
                Some(_) => {}
                None => bail!("The selected Rust test has no debuggable harness"),
            }
            Ok(RustTestActionPlan::Debug {
                worktree_id: descriptor.worktree_id,
                scenario: rust_test_debug_scenario(template)?,
            })
        }
    }
}

fn scoped_cargo_snapshot(
    mut snapshot: CargoWorkspaceSnapshot,
    allowed_worktrees: Option<&HashSet<WorktreeId>>,
) -> CargoWorkspaceSnapshot {
    let Some(allowed_worktrees) = allowed_worktrees else {
        return snapshot;
    };
    snapshot
        .workspaces
        .retain(|workspace| allowed_worktrees.contains(&workspace.key.root.worktree_id));
    snapshot
        .failures
        .retain(|failure| allowed_worktrees.contains(&failure.manifest_path.worktree_id));
    snapshot
}

fn retain_stale_discovery(
    snapshot: &mut StructuredProviderSnapshot,
    previous: Option<&StructuredProviderSnapshot>,
) {
    let Some(previous) = previous else {
        return;
    };
    if snapshot.status != StructuredProviderStatus::Error || previous.nodes.is_empty() {
        return;
    }
    snapshot.nodes = previous.nodes.clone();
    snapshot.status = StructuredProviderStatus::Stale;
    snapshot.partial = true;
    if snapshot.diagnostic.is_none() {
        snapshot.diagnostic = Some("Rust test discovery failed; showing stale results".to_string());
    }
}

type PreparedDiscoveryRequest = (
    CargoWorkspaceModel,
    RustTestDiscoveryRequest,
    Shared<Task<Option<HashMap<String, String>>>>,
);

fn prepare_discovery_requests(
    snapshot: &CargoWorkspaceSnapshot,
    worktree_store: &Entity<WorktreeStore>,
    environment: &Entity<ProjectEnvironment>,
    generation: u64,
    cx: &mut App,
) -> Vec<PreparedDiscoveryRequest> {
    snapshot
        .workspaces
        .iter()
        .filter_map(|workspace| {
            let manifest = workspace.root_manifest.as_ref().or_else(|| {
                workspace
                    .members
                    .first()
                    .map(|package| &package.manifest_path)
            })?;
            let manifest_path = worktree_store.read(cx).absolutize(manifest, cx)?;
            let working_directory = manifest_path.parent()?.to_path_buf();
            let worktree = worktree_store
                .read(cx)
                .worktree_for_id(manifest.worktree_id, cx)?;
            let environment = environment.update(cx, |environment, cx| {
                environment.worktree_environment(worktree, cx)
            });
            let doctest_targets = workspace
                .members
                .iter()
                .flat_map(|package| {
                    package.targets.iter().filter_map(|target| {
                        (target.kind == CargoTargetKind::Library)
                            .then(|| (package.name.clone(), target.name.clone()))
                    })
                })
                .collect();
            Some((
                workspace.clone(),
                RustTestDiscoveryRequest {
                    workspace_root: workspace.key.root.clone(),
                    generation,
                    limits: RustTestProtocolLimits::default(),
                    manifest_path: Some(manifest_path),
                    working_directory: Some(working_directory),
                    environment: None,
                    doctest_targets,
                },
                environment,
            ))
        })
        .collect()
}

struct RustTestProviderProjection {
    snapshot: StructuredProviderSnapshot,
    actions: HashMap<StructuredNodeId, RustTestActionDescriptor>,
}

fn project_provider_projection(
    cargo_snapshot: &CargoWorkspaceSnapshot,
    captures: Vec<(CargoWorkspaceModel, RustTestDiscoveryCapture)>,
    source_hints: &[RustTestSourceHint],
    failures: Vec<String>,
    generation: DiscoveryGeneration,
) -> RustTestProviderProjection {
    let provider_id = RustTestProviderStore::provider_id();
    let root_id = stable_node_id("provider", &[RUST_TEST_PROVIDER_ID]);
    let mut nodes = vec![StructuredNode {
        id: root_id.clone(),
        parent_id: None,
        label: "Rust Tests".to_string(),
        kind: StructuredNodeKind::Provider,
        path: None,
    }];
    let mut actions = HashMap::default();
    let captures = captures
        .into_iter()
        .map(|(workspace, capture)| {
            let protocol = RustTestProtocolAdapter::default().adapt(capture, source_hints);
            (workspace.key, protocol)
        })
        .collect::<BTreeMap<_, _>>();
    let mut protocol_partial = false;
    let mut discovered_cases = 0usize;
    for workspace in &cargo_snapshot.workspaces {
        let workspace_id = stable_node_id(
            "workspace",
            &[
                &workspace.key.root.worktree_id.to_proto().to_string(),
                workspace.key.root.path.as_unix_str(),
            ],
        );
        nodes.push(StructuredNode {
            id: workspace_id.clone(),
            parent_id: Some(root_id.clone()),
            label: workspace.display_name.clone(),
            kind: StructuredNodeKind::Group,
            path: workspace.root_manifest.clone(),
        });
        if let Some(manifest_path) = workspace.root_manifest.clone().or_else(|| {
            workspace
                .members
                .first()
                .map(|package| package.manifest_path.clone())
        }) {
            actions.insert(
                workspace_id.clone(),
                RustTestActionDescriptor {
                    scope: RustTestActionScope::Workspace,
                    worktree_id: manifest_path.worktree_id,
                    manifest_path,
                    label: workspace.display_name.clone(),
                    cargo_args: vec!["test".to_string(), "--workspace".to_string()],
                    executable_args: Vec::new(),
                    target_kind: None,
                },
            );
        }
        let protocol = captures.get(&workspace.key);
        if let Some(protocol) = protocol {
            protocol_partial |= protocol.capability != RustTestProtocolCapability::Supported;
        }
        for package in &workspace.members {
            project_package_nodes(
                package,
                &workspace_id,
                protocol,
                &mut nodes,
                &mut actions,
                &mut discovered_cases,
            );
        }
    }
    let restricted = cargo_snapshot.workspaces.is_empty()
        && cargo_snapshot
            .failures
            .iter()
            .any(|failure| failure.category == CargoWorkspaceErrorCategory::Restricted);
    let cargo_failure = cargo_snapshot
        .failures
        .iter()
        .find(|failure| failure.category != CargoWorkspaceErrorCategory::Restricted);
    let partial = cargo_snapshot.completeness == CargoSnapshotCompleteness::Partial
        || !cargo_snapshot.failures.is_empty()
        || protocol_partial
        || !failures.is_empty();
    let status = if restricted {
        StructuredProviderStatus::Restricted
    } else if cargo_snapshot.workspaces.is_empty() && cargo_failure.is_some() {
        StructuredProviderStatus::Error
    } else if cargo_snapshot.workspaces.is_empty() {
        StructuredProviderStatus::Empty
    } else if !failures.is_empty() && discovered_cases == 0 {
        StructuredProviderStatus::Error
    } else if partial {
        StructuredProviderStatus::Partial
    } else if discovered_cases == 0 {
        StructuredProviderStatus::Empty
    } else {
        StructuredProviderStatus::Current
    };
    let diagnostic = if restricted {
        Some("Trust the project to discover Rust tests".to_string())
    } else if !failures.is_empty() {
        Some(bounded_field(
            &failures.join("; "),
            MAX_RUST_TEST_FIELD_BYTES,
        ))
    } else if let Some(failure) = cargo_failure {
        Some(bounded_field(&failure.message, MAX_RUST_TEST_FIELD_BYTES))
    } else if protocol_partial {
        Some("Some Rust test records were unsupported or truncated".to_string())
    } else {
        None
    };
    let mut snapshot =
        StructuredProviderSnapshot::discovery(provider_id, generation, status, nodes);
    snapshot.partial = partial;
    snapshot.diagnostic = diagnostic;
    RustTestProviderProjection { snapshot, actions }
}

#[cfg(any(test, feature = "test-support"))]
pub fn project_provider_snapshot_for_test(
    cargo_snapshot: &CargoWorkspaceSnapshot,
    captures: Vec<(CargoWorkspaceModel, RustTestDiscoveryCapture)>,
    source_hints: &[RustTestSourceHint],
    failures: Vec<String>,
    generation: DiscoveryGeneration,
) -> StructuredProviderSnapshot {
    project_provider_projection(cargo_snapshot, captures, source_hints, failures, generation)
        .snapshot
}

fn project_package_nodes(
    package: &CargoPackageModel,
    workspace_id: &StructuredNodeId,
    protocol: Option<&RustTestProtocolSnapshot>,
    nodes: &mut Vec<StructuredNode>,
    actions: &mut HashMap<StructuredNodeId, RustTestActionDescriptor>,
    discovered_cases: &mut usize,
) {
    let package_id = stable_node_id("package", &[&package.id]);
    nodes.push(StructuredNode {
        id: package_id.clone(),
        parent_id: Some(workspace_id.clone()),
        label: package.name.clone(),
        kind: StructuredNodeKind::Suite,
        path: Some(package.manifest_path.clone()),
    });
    actions.insert(
        package_id.clone(),
        RustTestActionDescriptor {
            scope: RustTestActionScope::Package,
            worktree_id: package.manifest_path.worktree_id,
            manifest_path: package.manifest_path.clone(),
            label: package.name.clone(),
            cargo_args: vec![
                "test".to_string(),
                "--package".to_string(),
                package.name.clone(),
            ],
            executable_args: Vec::new(),
            target_kind: None,
        },
    );
    for target in &package.targets {
        let Some(target_kind) = cargo_target_kind(&target.kind) else {
            continue;
        };
        let target_id = stable_node_id(
            "target",
            &[&package.id, &target.name, &format!("{target_kind:?}")],
        );
        nodes.push(StructuredNode {
            id: target_id.clone(),
            parent_id: Some(package_id.clone()),
            label: target.name.clone(),
            kind: StructuredNodeKind::Group,
            path: target.source_path.clone(),
        });
        actions.insert(
            target_id.clone(),
            RustTestActionDescriptor {
                scope: RustTestActionScope::Target,
                worktree_id: package.manifest_path.worktree_id,
                manifest_path: package.manifest_path.clone(),
                label: target.name.clone(),
                cargo_args: target_cargo_args(&package.name, &target.name, target_kind),
                executable_args: Vec::new(),
                target_kind: Some(target_kind),
            },
        );
        let Some(protocol) = protocol else {
            continue;
        };
        let harnesses = protocol
            .harnesses
            .iter()
            .filter(|harness| {
                (harness.package_id == package.id
                    || cargo_package_name(&harness.package_id) == package.name)
                    && harness.target_name == target.name
                    && (harness.target_kind == target_kind
                        || (target_kind == RustTestTargetKind::Unit
                            && harness.target_kind == RustTestTargetKind::Doctest))
            })
            .map(|harness| (&harness.id, harness.target_kind))
            .collect::<BTreeMap<_, _>>();
        let mut groups = BTreeMap::<String, StructuredNodeId>::new();
        for case in protocol
            .cases
            .iter()
            .filter(|case| harnesses.contains_key(&case.harness_id))
        {
            let mut parent_id = target_id.clone();
            let components = case.name.split("::").collect::<Vec<_>>();
            for depth in 0..components.len().saturating_sub(1) {
                let group_name = components[..=depth].join("::");
                let group_id = groups.entry(group_name.clone()).or_insert_with(|| {
                    let id = stable_node_id("group", &[&target_id.0, &group_name]);
                    nodes.push(StructuredNode {
                        id: id.clone(),
                        parent_id: Some(parent_id.clone()),
                        label: components[depth].to_string(),
                        kind: StructuredNodeKind::Group,
                        path: None,
                    });
                    id
                });
                actions.entry(group_id.clone()).or_insert_with(|| {
                    let harness_kind = harnesses
                        .get(&case.harness_id)
                        .copied()
                        .unwrap_or(target_kind);
                    let mut cargo_args = case.runnable.cargo_args.clone();
                    normalize_package_selector(&mut cargo_args, &package.name);
                    RustTestActionDescriptor {
                        scope: RustTestActionScope::Group,
                        worktree_id: package.manifest_path.worktree_id,
                        manifest_path: package.manifest_path.clone(),
                        label: group_name.clone(),
                        cargo_args,
                        executable_args: vec![group_name.clone(), "--nocapture".to_string()],
                        target_kind: Some(harness_kind),
                    }
                });
                parent_id = group_id.clone();
            }
            let case_id = stable_node_id("case", &[&target_id.0, &case.id]);
            nodes.push(StructuredNode {
                id: case_id.clone(),
                parent_id: Some(parent_id),
                label: components.last().copied().unwrap_or(&case.name).to_string(),
                kind: StructuredNodeKind::Case,
                path: case.source.clone(),
            });
            let mut cargo_args = case.runnable.cargo_args.clone();
            normalize_package_selector(&mut cargo_args, &package.name);
            let mut executable_args = case.runnable.executable_args.clone();
            if case.ignored
                && !executable_args
                    .iter()
                    .any(|argument| argument == "--include-ignored")
            {
                executable_args.push("--include-ignored".to_string());
            }
            if !executable_args
                .iter()
                .any(|argument| argument == "--nocapture")
            {
                executable_args.push("--nocapture".to_string());
            }
            actions.insert(
                case_id,
                RustTestActionDescriptor {
                    scope: RustTestActionScope::Case,
                    worktree_id: package.manifest_path.worktree_id,
                    manifest_path: package.manifest_path.clone(),
                    label: case.name.clone(),
                    cargo_args,
                    executable_args,
                    target_kind: harnesses.get(&case.harness_id).copied(),
                },
            );
            *discovered_cases = discovered_cases.saturating_add(1);
        }
    }
}

fn target_cargo_args(
    package_name: &str,
    target_name: &str,
    target_kind: RustTestTargetKind,
) -> Vec<String> {
    let mut arguments = vec![
        if target_kind == RustTestTargetKind::Benchmark {
            "bench".to_string()
        } else {
            "test".to_string()
        },
        "--package".to_string(),
        package_name.to_string(),
    ];
    match target_kind {
        RustTestTargetKind::Unit => arguments.push("--lib".to_string()),
        RustTestTargetKind::Integration => {
            arguments.extend(["--test".to_string(), target_name.to_string()])
        }
        RustTestTargetKind::Binary => {
            arguments.extend(["--bin".to_string(), target_name.to_string()])
        }
        RustTestTargetKind::Example => {
            arguments.extend(["--example".to_string(), target_name.to_string()])
        }
        RustTestTargetKind::Benchmark => {
            arguments.extend(["--bench".to_string(), target_name.to_string()])
        }
        RustTestTargetKind::Doctest => arguments.push("--doc".to_string()),
    }
    arguments
}

fn rust_test_task_template(descriptor: &RustTestActionDescriptor) -> TaskTemplate {
    let mut arguments = descriptor.cargo_args.clone();
    if !descriptor.executable_args.is_empty() {
        arguments.push("--".to_string());
        arguments.extend(descriptor.executable_args.clone());
    }
    let worktree_root = VariableName::WorktreeRoot.template_value();
    let cwd = descriptor
        .manifest_path
        .path
        .parent()
        .filter(|parent| !parent.is_empty())
        .map(|parent| format!("{worktree_root}/{}", parent.as_unix_str()))
        .unwrap_or(worktree_root);
    TaskTemplate {
        label: format!("Rust test {}", descriptor.label),
        command: "cargo".to_string(),
        args: arguments,
        cwd: Some(cwd),
        use_new_terminal: true,
        allow_concurrent_runs: false,
        tags: vec!["rust-test".to_string(), "structured-test".to_string()],
        save: SaveStrategy::All,
        ..TaskTemplate::default()
    }
}

fn rust_test_debug_scenario(mut template: TaskTemplate) -> Result<DebugScenario> {
    let delimiter = template
        .args
        .iter()
        .position(|argument| argument == "--")
        .unwrap_or(template.args.len());
    if !template.args[..delimiter]
        .iter()
        .any(|argument| argument == "--no-run")
    {
        template.args.insert(delimiter, "--no-run".to_string());
    }
    Ok(rust_test_debug_scenario_from_build(template))
}

fn rust_test_debug_scenario_from_build(template: TaskTemplate) -> DebugScenario {
    DebugScenario {
        adapter: "CodeLLDB".into(),
        label: format!("Debug {}", template.label).into(),
        build: Some(BuildTaskDefinition::Template {
            task_template: template,
            locator_name: Some("rust-cargo-locator".into()),
        }),
        config: serde_json::json!({ "sourceLanguages": ["rust"] }),
        tcp_connection: None,
    }
}

fn normalize_package_selector(arguments: &mut [String], package_name: &str) {
    let package_value = arguments
        .iter()
        .position(|argument| argument == "--package" || argument == "-p")
        .and_then(|index| index.checked_add(1));
    if let Some(package_value) = package_value.and_then(|index| arguments.get_mut(index)) {
        *package_value = package_name.to_string();
    }
}

fn cargo_target_kind(kind: &CargoTargetKind) -> Option<RustTestTargetKind> {
    match kind {
        CargoTargetKind::Library => Some(RustTestTargetKind::Unit),
        CargoTargetKind::Binary => Some(RustTestTargetKind::Binary),
        CargoTargetKind::Example => Some(RustTestTargetKind::Example),
        CargoTargetKind::Test => Some(RustTestTargetKind::Integration),
        CargoTargetKind::Bench => Some(RustTestTargetKind::Benchmark),
        CargoTargetKind::BuildScript | CargoTargetKind::Other(_) => None,
    }
}

fn cargo_package_name(package_id: &str) -> &str {
    package_id
        .rsplit_once('#')
        .map(|(_, package)| package)
        .unwrap_or(package_id)
        .split('@')
        .next()
        .unwrap_or(package_id)
}

fn stable_node_id(prefix: &str, components: &[&str]) -> StructuredNodeId {
    let mut digest = Sha256::new();
    digest.update(prefix.as_bytes());
    for component in components {
        digest.update(component.len().to_le_bytes());
        digest.update(component.as_bytes());
    }
    StructuredNodeId(format!("rust-test-{prefix}-{:x}", digest.finalize()))
}

fn rust_test_input_changed(event: &WorktreeStoreEvent) -> bool {
    match event {
        WorktreeStoreEvent::WorktreeUpdatedEntries(_, entries) => {
            entries.iter().any(|(path, _, _)| {
                matches!(path.extension(), Some("rs" | "toml"))
                    || matches!(path.file_name(), Some("Cargo.lock" | "rust-toolchain"))
            })
        }
        WorktreeStoreEvent::WorktreeAdded(_)
        | WorktreeStoreEvent::WorktreeRemoved(_, _)
        | WorktreeStoreEvent::WorktreeReleased(_, _)
        | WorktreeStoreEvent::WorktreeOrderChanged
        | WorktreeStoreEvent::WorktreeDeletedEntry(_, _) => true,
        WorktreeStoreEvent::WorktreeUpdatedGitRepositories(_, _)
        | WorktreeStoreEvent::WorktreeUpdatedRootRepoCommonDir(_)
        | WorktreeStoreEvent::WorktreeUpdateSent(_) => false,
    }
}

fn provider_status_to_proto(status: StructuredProviderStatus) -> i32 {
    (match status {
        StructuredProviderStatus::Loading => proto::StructuredProviderStatus::Loading,
        StructuredProviderStatus::Current => proto::StructuredProviderStatus::Current,
        StructuredProviderStatus::Empty => proto::StructuredProviderStatus::Empty,
        StructuredProviderStatus::Partial => proto::StructuredProviderStatus::Partial,
        StructuredProviderStatus::Stale => proto::StructuredProviderStatus::Stale,
        StructuredProviderStatus::Error => proto::StructuredProviderStatus::Error,
        StructuredProviderStatus::Restricted => proto::StructuredProviderStatus::Restricted,
        StructuredProviderStatus::Disconnected => proto::StructuredProviderStatus::Disconnected,
        StructuredProviderStatus::Mismatch => proto::StructuredProviderStatus::Mismatch,
    }) as i32
}

fn map_remote_discovery_error(error: anyhow::Error) -> anyhow::Error {
    let unsupported = error
        .downcast_ref::<proto::RpcError>()
        .is_some_and(|error| error.raw_message().contains("was not handled"));
    if unsupported {
        anyhow!("The project host does not support Rust test discovery")
    } else {
        anyhow!("The Rust test discovery host is disconnected: {error}")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        WorktreeId,
        cargo_workspace::{
            CargoCandidateFailure, CargoTargetModel, CargoWorkspaceConfiguration, CargoWorkspaceKey,
        },
    };
    use util::rel_path::RelPath;

    use super::*;

    struct FixtureRunner;

    impl RustTestDiscoveryRunner for FixtureRunner {
        fn discover(
            &self,
            _: RustTestDiscoveryRequest,
        ) -> BoxFuture<'static, Result<RustTestDiscoveryCapture>> {
            Box::pin(async { Ok(capture()) })
        }
    }

    const PACKAGE_ID: &str = "path+file:///fixture#rust-fixture@0.1.0";
    const CARGO_MESSAGES: &str =
        include_str!("../test_data/rust_test_provider/cargo_messages.jsonl");
    const LISTINGS: &str = include_str!("../test_data/rust_test_provider/listings.json");

    fn capture() -> RustTestDiscoveryCapture {
        RustTestDiscoveryCapture {
            toolchain: "1.95.0".to_string(),
            cargo_messages: CARGO_MESSAGES.to_string(),
            listings: serde_json::from_str(LISTINGS).expect("fixture listings should parse"),
        }
    }

    fn project_path(path: &str) -> ProjectPath {
        ProjectPath {
            worktree_id: WorktreeId::from_usize(1),
            path: Arc::from(RelPath::from_unix_str(path).expect("fixture path should be relative")),
        }
    }

    fn project_path_in(worktree_id: usize, path: &str) -> ProjectPath {
        ProjectPath {
            worktree_id: WorktreeId::from_usize(worktree_id),
            path: Arc::from(RelPath::from_unix_str(path).expect("fixture path should be relative")),
        }
    }

    fn cargo_target(name: &str, kind: CargoTargetKind, source: &str) -> CargoTargetModel {
        CargoTargetModel {
            name: name.to_string(),
            kind,
            crate_types: Vec::new(),
            source_path: Some(project_path(source)),
            source_display_path: Some(source.to_string()),
            required_features: Vec::new(),
            edition: "2024".to_string(),
        }
    }

    fn cargo_workspace(worktree_id: usize, display_name: &str) -> CargoWorkspaceModel {
        let manifest_path = project_path_in(worktree_id, "Cargo.toml");
        CargoWorkspaceModel {
            key: CargoWorkspaceKey {
                root: manifest_path.clone(),
            },
            root_manifest: Some(manifest_path.clone()),
            display_name: display_name.to_string(),
            is_virtual: false,
            members: vec![CargoPackageModel {
                id: format!("{worktree_id}:Cargo.toml:rust-fixture@0.1.0"),
                name: "rust-fixture".to_string(),
                version: "0.1.0".to_string(),
                manifest_path,
                is_default_member: true,
                targets: vec![
                    cargo_target("rust_fixture", CargoTargetKind::Library, "src/lib.rs"),
                    cargo_target("cli", CargoTargetKind::Binary, "src/bin/cli.rs"),
                    cargo_target("api", CargoTargetKind::Test, "tests/api.rs"),
                    cargo_target("demo", CargoTargetKind::Example, "examples/demo.rs"),
                    cargo_target(
                        "throughput",
                        CargoTargetKind::Bench,
                        "benches/throughput.rs",
                    ),
                ],
                features: Vec::new(),
                dependencies: Vec::new(),
            }],
            configuration: CargoWorkspaceConfiguration::unresolved(),
        }
    }

    fn cargo_snapshot(workspaces: Vec<CargoWorkspaceModel>) -> CargoWorkspaceSnapshot {
        CargoWorkspaceSnapshot {
            revision: 1,
            input_fingerprint: 1,
            workspaces,
            failures: Vec::new(),
            completeness: CargoSnapshotCompleteness::Complete,
        }
    }

    #[test]
    fn rust_test_protocol_is_bounded_stable_and_partial_for_unknown_records() {
        let hint = RustTestSourceHint {
            package_id: PACKAGE_ID.to_string(),
            target_name: "rust_fixture".to_string(),
            target_kind: RustTestTargetKind::Unit,
            test_name: "unit::works".to_string(),
            source: Some(project_path("src/lib.rs")),
            runnable: Some(RustTestRunnableSelector {
                cargo_args: vec!["test".to_string(), "--lib".to_string()],
                executable_args: vec!["unit::works".to_string(), "--exact".to_string()],
            }),
        };
        let adapter = RustTestProtocolAdapter::default();
        let first = adapter.adapt(capture(), std::slice::from_ref(&hint));
        let second = adapter.adapt(capture(), &[hint]);
        assert_eq!(first, second);
        assert_eq!(first.capability, RustTestProtocolCapability::Supported);
        assert_eq!(first.harnesses.len(), 6);
        assert_eq!(first.cases.len(), 7);
        let unit = first
            .cases
            .iter()
            .find(|case| case.name == "unit::works")
            .expect("unit fixture should be present");
        assert_eq!(unit.source, Some(project_path("src/lib.rs")));
        assert_eq!(unit.runnable.cargo_args, ["test", "--lib"]);

        let mut unknown = capture();
        unknown
            .cargo_messages
            .push_str("{\"reason\":\"future-cargo-record\",\"secret\":\"not retained\"}\n");
        unknown.listings[0]
            .stdout
            .push_str("future listing record\n");
        let partial = adapter.adapt(unknown, &[]);
        assert_eq!(partial.capability, RustTestProtocolCapability::Partial);
        assert_eq!(partial.cases.len(), 7);
        assert_eq!(partial.diagnostics.len(), 2);
        assert!(
            partial
                .diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.message.contains("not retained"))
        );

        let limited = RustTestProtocolAdapter::new(RustTestProtocolLimits {
            cases: 2,
            field_bytes: 64,
            diagnostics: 4,
            ..RustTestProtocolLimits::default()
        })
        .adapt(capture(), &[]);
        assert!(limited.truncated);
        assert_eq!(limited.cases.len(), 2);
        assert!(limited.diagnostics.len() <= 4);
        assert!(
            limited
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message.len() <= 64)
        );

        let request = RustTestDiscoveryRequest {
            workspace_root: project_path(""),
            generation: 4,
            limits: RustTestProtocolLimits::default(),
            manifest_path: None,
            working_directory: None,
            environment: None,
            doctest_targets: Vec::new(),
        };
        let injected = smol::block_on(FixtureRunner.discover(request))
            .expect("injected fixture runner should complete without Cargo or network");
        assert_eq!(injected.toolchain, "1.95.0");
    }

    #[test]
    fn rust_workspace_adapts_ten_thousand_tests_without_machine_tools() {
        let listing = (0..MAX_RUST_TEST_CASES)
            .map(|index| format!("scale::case_{index:05}: test\n"))
            .collect::<String>();
        let mut large_capture = capture();
        large_capture.listings = vec![RustTestListingCapture {
            package_id: PACKAGE_ID.to_string(),
            target_name: "rust_fixture".to_string(),
            target_kind: RustTestTargetKind::Unit,
            mode: RustTestListingMode::All,
            stdout: listing,
        }];

        let first = RustTestProtocolAdapter::default().adapt(large_capture.clone(), &[]);
        let second = RustTestProtocolAdapter::default().adapt(large_capture, &[]);
        assert_eq!(first.cases.len(), MAX_RUST_TEST_CASES);
        assert!(!first.truncated);
        assert_eq!(
            first.cases.iter().map(|case| &case.id).collect::<Vec<_>>(),
            second.cases.iter().map(|case| &case.id).collect::<Vec<_>>()
        );
        assert!(first.cases.windows(2).all(|pair| pair[0].id < pair[1].id));
    }

    #[test]
    fn rust_test_provider_projects_stable_scoped_hierarchy_and_source_hints() {
        let workspace = cargo_workspace(1, "fixture");
        let workspace_snapshot = cargo_snapshot(vec![workspace.clone()]);
        let hint = RustTestSourceHint {
            package_id: PACKAGE_ID.to_string(),
            target_name: "rust_fixture".to_string(),
            target_kind: RustTestTargetKind::Unit,
            test_name: "unit::works".to_string(),
            source: Some(project_path("src/lib.rs")),
            runnable: None,
        };
        let first = project_provider_snapshot_for_test(
            &workspace_snapshot,
            vec![(workspace.clone(), capture())],
            std::slice::from_ref(&hint),
            Vec::new(),
            DiscoveryGeneration(1),
        );
        let second = project_provider_snapshot_for_test(
            &workspace_snapshot,
            vec![(workspace, capture())],
            &[hint],
            Vec::new(),
            DiscoveryGeneration(2),
        );
        assert_eq!(first.status, StructuredProviderStatus::Current);
        assert_eq!(
            first.nodes.iter().map(|node| &node.id).collect::<Vec<_>>(),
            second.nodes.iter().map(|node| &node.id).collect::<Vec<_>>()
        );
        let cases = first
            .nodes
            .iter()
            .filter(|node| node.kind == StructuredNodeKind::Case)
            .collect::<Vec<_>>();
        assert_eq!(cases.len(), 7);
        assert!(cases.iter().any(|node| {
            node.label == "works" && node.path == Some(project_path("src/lib.rs"))
        }));

        let scoped = scoped_cargo_snapshot(
            cargo_snapshot(vec![
                cargo_workspace(1, "visible"),
                cargo_workspace(2, "hidden"),
            ]),
            Some(&HashSet::from([WorktreeId::from_usize(1)])),
        );
        assert_eq!(scoped.workspaces.len(), 1);
        assert_eq!(scoped.workspaces[0].display_name, "visible");
    }

    #[test]
    fn rust_test_provider_isolates_partial_roots_and_retains_stale_results() {
        let good_workspace = cargo_workspace(1, "good");
        let failed_workspace = cargo_workspace(2, "failed");
        let mut cargo_snapshot = cargo_snapshot(vec![good_workspace.clone(), failed_workspace]);
        cargo_snapshot.completeness = CargoSnapshotCompleteness::Partial;
        cargo_snapshot.failures.push(CargoCandidateFailure {
            manifest_path: project_path_in(2, "Cargo.toml"),
            category: CargoWorkspaceErrorCategory::CargoFailed,
            message: "fixture failure".to_string(),
            has_stale_model: false,
        });
        let partial = project_provider_snapshot_for_test(
            &cargo_snapshot,
            vec![(good_workspace, capture())],
            &[],
            vec!["failed root".to_string()],
            DiscoveryGeneration(2),
        );
        assert_eq!(partial.status, StructuredProviderStatus::Partial);
        assert!(
            partial
                .nodes
                .iter()
                .any(|node| { node.kind == StructuredNodeKind::Case && node.label == "works" })
        );

        let previous = StructuredProviderSnapshot::discovery(
            RustTestProviderStore::provider_id(),
            DiscoveryGeneration(2),
            StructuredProviderStatus::Current,
            partial.nodes,
        );
        let mut failed = StructuredProviderSnapshot::discovery(
            RustTestProviderStore::provider_id(),
            DiscoveryGeneration(3),
            StructuredProviderStatus::Error,
            Vec::new(),
        );
        failed.diagnostic = Some("runner failed".to_string());
        retain_stale_discovery(&mut failed, Some(&previous));
        assert_eq!(failed.status, StructuredProviderStatus::Stale);
        assert_eq!(failed.nodes, previous.nodes);
        assert!(failed.partial);
        assert_eq!(failed.diagnostic.as_deref(), Some("runner failed"));
    }

    #[test]
    fn rust_test_actions_compile_exact_tasks_and_supported_debug_scenarios() {
        let workspace = cargo_workspace(1, "fixture");
        let projection = project_provider_projection(
            &cargo_snapshot(vec![workspace.clone()]),
            vec![(workspace, capture())],
            &[],
            Vec::new(),
            DiscoveryGeneration(7),
        );
        let case_id = projection
            .snapshot
            .nodes
            .iter()
            .find(|node| node.kind == StructuredNodeKind::Case && node.label == "works")
            .map(|node| node.id.clone())
            .expect("unit test case should be projected");
        let descriptor = projection
            .actions
            .get(&case_id)
            .expect("unit test case should have an action descriptor");
        let RustTestActionPlan::Task {
            run_id,
            template,
            scope_node_ids,
            ..
        } = plan_rust_test_action(
            descriptor,
            &case_id,
            DiscoveryGeneration(7),
            3,
            RustTestAction::Run,
        )
        .expect("unit test should compile to a task")
        else {
            panic!("run should produce a task plan")
        };
        assert_eq!(run_id.0, "rust-test-run-7-3");
        assert_eq!(scope_node_ids, std::slice::from_ref(&case_id));
        assert_eq!(template.command, "cargo");
        assert_eq!(
            template.args,
            [
                "test",
                "--package",
                "rust-fixture",
                "--lib",
                "--",
                "unit::works",
                "--exact",
                "--nocapture",
            ]
        );
        assert_eq!(template.save, SaveStrategy::All);
        assert!(template.use_new_terminal);
        assert!(!template.allow_concurrent_runs);

        let RustTestActionPlan::Debug { scenario, .. } = plan_rust_test_action(
            descriptor,
            &case_id,
            DiscoveryGeneration(7),
            4,
            RustTestAction::Debug,
        )
        .expect("unit test should compile to Cargo DAP") else {
            panic!("debug should produce a debug plan")
        };
        let BuildTaskDefinition::Template {
            task_template,
            locator_name,
        } = scenario
            .build
            .expect("debug scenario should build with Cargo")
        else {
            panic!("debug scenario should use a task template")
        };
        assert_eq!(locator_name.as_deref(), Some("rust-cargo-locator"));
        let delimiter = task_template
            .args
            .iter()
            .position(|argument| argument == "--")
            .expect("test args should use a harness delimiter");
        assert!(task_template.args[..delimiter].contains(&"--no-run".to_string()));

        let doctest_id = StructuredNodeId("doctest-case".to_string());
        let doctest = RustTestActionDescriptor {
            scope: RustTestActionScope::Case,
            worktree_id: WorktreeId::from_usize(1),
            manifest_path: project_path("Cargo.toml"),
            label: "src/lib.rs - example".to_string(),
            cargo_args: vec!["test".to_string(), "--doc".to_string()],
            executable_args: vec!["src/lib.rs - example".to_string()],
            target_kind: Some(RustTestTargetKind::Doctest),
        };
        assert!(
            plan_rust_test_action(
                &doctest,
                &doctest_id,
                DiscoveryGeneration(7),
                5,
                RustTestAction::Debug,
            )
            .expect_err("doctest debug must be disabled")
            .to_string()
            .contains("doctests")
        );
    }

    #[test]
    fn rust_test_actions_bound_ignored_and_group_reruns() {
        let workspace = cargo_workspace(1, "fixture");
        let projection = project_provider_projection(
            &cargo_snapshot(vec![workspace.clone()]),
            vec![(workspace, capture())],
            &[],
            Vec::new(),
            DiscoveryGeneration(8),
        );
        let ignored = projection
            .snapshot
            .nodes
            .iter()
            .find(|node| node.kind == StructuredNodeKind::Case && node.label == "ignored")
            .expect("ignored case should be projected");
        let ignored_template = rust_test_task_template(
            projection
                .actions
                .get(&ignored.id)
                .expect("ignored case should remain executable"),
        );
        assert!(
            ignored_template
                .args
                .contains(&"--include-ignored".to_string())
        );

        let group = projection
            .snapshot
            .nodes
            .iter()
            .find(|node| {
                node.kind == StructuredNodeKind::Group
                    && node.label == "unit"
                    && projection.actions.contains_key(&node.id)
            })
            .expect("module group should be executable");
        let group_template = rust_test_task_template(
            projection
                .actions
                .get(&group.id)
                .expect("group should have an action descriptor"),
        );
        assert!(group_template.args.contains(&"unit".to_string()));
        assert!(!group_template.args.contains(&"--exact".to_string()));
        assert_eq!(MAX_RUST_TEST_RERUNS, 4);
    }

    #[test]
    fn rust_test_feature_remote_boundary_is_bounded_typed_and_restricted() {
        let workspace = cargo_workspace(1, "fixture");
        let projection = project_provider_projection(
            &cargo_snapshot(vec![workspace.clone()]),
            vec![(workspace, capture())],
            &[],
            Vec::new(),
            DiscoveryGeneration(12),
        );
        let case_id = projection
            .snapshot
            .nodes
            .iter()
            .find(|node| node.kind == StructuredNodeKind::Case && node.label == "works")
            .map(|node| node.id.clone())
            .expect("fixture case should be projected");
        let plan = plan_rust_test_action(
            projection
                .actions
                .get(&case_id)
                .expect("fixture case should be executable"),
            &case_id,
            DiscoveryGeneration(12),
            9,
            RustTestAction::Run,
        )
        .expect("fixture action should resolve");
        let plan_json = encode_action_plan(&plan).expect("action plan should encode");
        assert!(plan_json.len() <= MAX_RUST_TEST_ACTION_PLAN_BYTES);
        assert_eq!(
            decode_action_plan(proto::RustTestActionPlanResponse {
                protocol_version: RUST_TEST_DISCOVERY_PROTOCOL_VERSION,
                plan_json,
            })
            .expect("action plan should decode"),
            plan
        );

        let RustTestActionPlan::Task {
            run_id,
            discovery_generation,
            scope_node_ids,
            mut template,
            ..
        } = plan
        else {
            panic!("run action should produce a task")
        };
        template
            .env
            .insert("TOKEN".to_string(), "secret".to_string());
        let unsafe_plan = RustTestActionWirePlan::Task {
            run_id: run_id.0.clone(),
            discovery_generation: discovery_generation.0,
            scope_node_ids: scope_node_ids
                .iter()
                .map(|node_id| node_id.0.clone())
                .collect(),
            worktree_id: 1,
            template,
        };
        let unsafe_plan = serde_json::to_string(&unsafe_plan).expect("fixture should serialize");
        assert!(
            decode_action_plan(proto::RustTestActionPlanResponse {
                protocol_version: RUST_TEST_DISCOVERY_PROTOCOL_VERSION,
                plan_json: unsafe_plan,
            })
            .expect_err("remote task environments must be rejected")
            .to_string()
            .contains("unsupported task fields")
        );
        assert!(
            decode_action_plan(proto::RustTestActionPlanResponse {
                protocol_version: RUST_TEST_DISCOVERY_PROTOCOL_VERSION + 1,
                plan_json: "{}".to_string(),
            })
            .expect_err("feature mismatch must fail closed")
            .to_string()
            .contains("protocol mismatch")
        );

        let lifecycle = StructuredTaskLifecycleEvent {
            task_id: TaskId("task-remote".to_string()),
            state: StructuredTaskState::Completed {
                terminal_id: Some(StructuredTerminalId(17)),
                exit_code: Some(1),
                success: false,
            },
        };
        let lifecycle_proto = lifecycle_to_proto(
            42,
            discovery_generation,
            &run_id,
            &scope_node_ids,
            &lifecycle,
            vec![1],
        );
        assert_eq!(
            lifecycle_from_proto(&lifecycle_proto).expect("lifecycle should round-trip"),
            lifecycle
        );

        let mut restricted = cargo_snapshot(Vec::new());
        restricted.completeness = CargoSnapshotCompleteness::Partial;
        restricted.failures.push(CargoCandidateFailure {
            manifest_path: project_path("Cargo.toml"),
            category: CargoWorkspaceErrorCategory::Restricted,
            message: "sensitive host detail".to_string(),
            has_stale_model: false,
        });
        let restricted_projection = project_provider_projection(
            &restricted,
            Vec::new(),
            &[],
            Vec::new(),
            DiscoveryGeneration(13),
        );
        assert_eq!(
            restricted_projection.snapshot.status,
            StructuredProviderStatus::Restricted
        );
        assert!(restricted_projection.actions.is_empty());
        assert_eq!(
            restricted_projection.snapshot.diagnostic.as_deref(),
            Some("Trust the project to discover Rust tests")
        );
    }

    #[test]
    #[ignore = "stable-toolchain protocol gate; uses captured hermetic fixtures"]
    fn rust_test_protocol_fixture_matrix() {
        let snapshot = RustTestProtocolAdapter::default().adapt(capture(), &[]);
        assert_eq!(snapshot.toolchain, "1.95.0");
        assert_eq!(snapshot.capability, RustTestProtocolCapability::Supported);
        assert!(snapshot.diagnostics.is_empty());
        assert!(!snapshot.truncated);
        let target_kinds = snapshot
            .harnesses
            .iter()
            .map(|harness| harness.target_kind)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            target_kinds,
            BTreeSet::from([
                RustTestTargetKind::Unit,
                RustTestTargetKind::Integration,
                RustTestTargetKind::Binary,
                RustTestTargetKind::Example,
                RustTestTargetKind::Benchmark,
                RustTestTargetKind::Doctest,
            ])
        );
        assert!(snapshot.cases.iter().any(|case| case.ignored));
        assert!(snapshot.cases.iter().any(|case| {
            case.kind == RustTestCaseKind::Benchmark && case.harness_id.contains("Benchmark")
        }));
        assert!(snapshot.cases.iter().any(|case| {
            case.name.contains("src/lib.rs") && case.harness_id.contains("Doctest")
        }));
        assert!(snapshot.cases.iter().all(|case| {
            !case.runnable.cargo_args.is_empty()
                && case
                    .runnable
                    .executable_args
                    .ends_with(&["--exact".to_string()])
        }));
    }
}
