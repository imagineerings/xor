use std::collections::{BTreeMap, HashSet};

use anyhow::{Context as _, Result, bail};
use gpui::{AsyncApp, Context, Entity, EventEmitter, Task};
use rpc::{AnyProtoClient, TypedEnvelope, proto};

use crate::{ProjectPath, WorktreeId, worktree_store::WorktreeStore};

pub const SOURCE_COVERAGE_PROTOCOL_VERSION: u32 = 1;
pub const MAX_SOURCE_COVERAGE_FILES: usize = 4_096;
pub const MAX_SOURCE_COVERAGE_RANGES: usize = 100_000;
pub const MAX_SOURCE_COVERAGE_RANGES_PER_FILE: usize = 10_000;
pub const MAX_SOURCE_COVERAGE_PROVIDER_BYTES: usize = 256;
pub const MAX_SOURCE_COVERAGE_DIAGNOSTIC_BYTES: usize = 4 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SourceCoverageProviderId(pub String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceCoverageStatus {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceCoveragePoint {
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceCoverageRange {
    pub start: SourceCoveragePoint,
    pub end: SourceCoveragePoint,
    pub hit_count: u64,
}

impl SourceCoverageRange {
    pub fn is_covered(&self) -> bool {
        self.hit_count > 0
    }

    fn is_valid(&self) -> bool {
        (self.start.line, self.start.column) <= (self.end.line, self.end.column)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceCoverageFile {
    pub path: ProjectPath,
    pub ranges: Vec<SourceCoverageRange>,
    pub covered_lines: u32,
    pub uncovered_lines: u32,
    pub truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceCoverageSnapshot {
    pub project_generation: u64,
    pub provider_id: SourceCoverageProviderId,
    pub generation: u64,
    pub status: SourceCoverageStatus,
    pub files: Vec<SourceCoverageFile>,
    pub truncated: bool,
    pub diagnostic: Option<String>,
}

impl SourceCoverageSnapshot {
    pub fn bounded(mut self) -> Result<Self> {
        if self.provider_id.0.trim().is_empty()
            || self.provider_id.0.len() > MAX_SOURCE_COVERAGE_PROVIDER_BYTES
        {
            bail!("source coverage provider identity is empty or too long");
        }
        if let Some(diagnostic) = &mut self.diagnostic {
            truncate_utf8(diagnostic, MAX_SOURCE_COVERAGE_DIAGNOSTIC_BYTES);
        }
        self.files.sort_by(|left, right| left.path.cmp(&right.path));
        self.files.dedup_by(|left, right| left.path == right.path);
        if self.files.len() > MAX_SOURCE_COVERAGE_FILES {
            self.files.truncate(MAX_SOURCE_COVERAGE_FILES);
            self.truncated = true;
        }
        let mut remaining_ranges = MAX_SOURCE_COVERAGE_RANGES;
        let mut malformed = 0usize;
        for file in &mut self.files {
            let original_len = file.ranges.len();
            file.ranges.retain(SourceCoverageRange::is_valid);
            malformed += original_len.saturating_sub(file.ranges.len());
            file.ranges.sort_by_key(|range| {
                (
                    range.start.line,
                    range.start.column,
                    range.end.line,
                    range.end.column,
                )
            });
            let limit = remaining_ranges.min(MAX_SOURCE_COVERAGE_RANGES_PER_FILE);
            if file.ranges.len() > limit {
                file.ranges.truncate(limit);
                file.truncated = true;
                self.truncated = true;
            }
            remaining_ranges = remaining_ranges.saturating_sub(file.ranges.len());
        }
        if malformed > 0 {
            self.status = SourceCoverageStatus::Partial;
            self.diagnostic = Some(format!(
                "Ignored {malformed} malformed source coverage range(s)"
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct SourceCoverageState {
    project_generation: u64,
    providers: BTreeMap<SourceCoverageProviderId, SourceCoverageSnapshot>,
}

impl SourceCoverageState {
    pub fn new(project_generation: u64) -> Self {
        Self {
            project_generation,
            providers: BTreeMap::new(),
        }
    }

    pub fn project_generation(&self) -> u64 {
        self.project_generation
    }

    pub fn provider(
        &self,
        provider_id: &SourceCoverageProviderId,
    ) -> Option<&SourceCoverageSnapshot> {
        self.providers.get(provider_id)
    }

    pub fn publish(&mut self, snapshot: SourceCoverageSnapshot) -> Result<()> {
        let snapshot = snapshot.bounded()?;
        if snapshot.project_generation != self.project_generation {
            bail!("source coverage project generation is stale");
        }
        if self
            .providers
            .get(&snapshot.provider_id)
            .is_some_and(|current| snapshot.generation < current.generation)
        {
            bail!("source coverage provider generation is stale");
        }
        self.providers
            .insert(snapshot.provider_id.clone(), snapshot);
        Ok(())
    }

    pub fn mark_provider_status(
        &mut self,
        provider_id: SourceCoverageProviderId,
        status: SourceCoverageStatus,
        diagnostic: Option<String>,
    ) -> Result<()> {
        let snapshot = self
            .provider(&provider_id)
            .cloned()
            .map(|mut snapshot| {
                snapshot.status = status;
                snapshot.diagnostic = diagnostic.clone();
                snapshot
            })
            .unwrap_or(SourceCoverageSnapshot {
                project_generation: self.project_generation(),
                provider_id,
                generation: 0,
                status,
                files: Vec::new(),
                truncated: false,
                diagnostic,
            });
        self.publish(snapshot)
    }
}

#[derive(Clone, Debug)]
pub enum SourceCoverageStoreEvent {
    Changed(SourceCoverageProviderId),
}

impl EventEmitter<SourceCoverageStoreEvent> for SourceCoverageStore {}

pub struct SourceCoverageStore {
    mode: SourceCoverageStoreMode,
    worktree_store: Entity<WorktreeStore>,
    state: SourceCoverageState,
}

enum SourceCoverageStoreMode {
    Local,
    Remote {
        project_id: u64,
        client: AnyProtoClient,
    },
}

impl SourceCoverageStore {
    pub fn init(client: &AnyProtoClient) {
        client.add_entity_request_handler(Self::handle_get_source_coverage);
    }

    pub fn local(worktree_store: Entity<WorktreeStore>, project_generation: u64) -> Self {
        Self {
            mode: SourceCoverageStoreMode::Local,
            worktree_store,
            state: SourceCoverageState::new(project_generation),
        }
    }

    pub fn remote(
        project_id: u64,
        client: AnyProtoClient,
        worktree_store: Entity<WorktreeStore>,
        project_generation: u64,
    ) -> Self {
        Self {
            mode: SourceCoverageStoreMode::Remote { project_id, client },
            worktree_store,
            state: SourceCoverageState::new(project_generation),
        }
    }

    pub fn publish(
        &mut self,
        snapshot: SourceCoverageSnapshot,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if !matches!(self.mode, SourceCoverageStoreMode::Local) {
            bail!("only the authoritative project host can publish source coverage");
        }
        let provider_id = snapshot.provider_id.clone();
        self.state.publish(snapshot)?;
        cx.emit(SourceCoverageStoreEvent::Changed(provider_id));
        Ok(())
    }

    pub fn mark_provider_status(
        &mut self,
        provider_id: SourceCoverageProviderId,
        status: SourceCoverageStatus,
        diagnostic: Option<String>,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        self.state
            .mark_provider_status(provider_id.clone(), status, diagnostic)?;
        cx.emit(SourceCoverageStoreEvent::Changed(provider_id));
        Ok(())
    }

    #[cfg(feature = "rust-coverage")]
    pub(crate) fn replace_remote(
        &mut self,
        snapshot: SourceCoverageSnapshot,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if !matches!(self.mode, SourceCoverageStoreMode::Remote { .. }) {
            bail!("remote source coverage replacement requires a remote store");
        }
        if self.state.project_generation != snapshot.project_generation {
            self.state = SourceCoverageState::new(snapshot.project_generation);
        }
        let provider_id = snapshot.provider_id.clone();
        self.state.publish(snapshot)?;
        cx.emit(SourceCoverageStoreEvent::Changed(provider_id));
        Ok(())
    }

    pub fn snapshot(
        &mut self,
        provider_id: SourceCoverageProviderId,
        cx: &mut Context<Self>,
    ) -> Task<Result<SourceCoverageSnapshot>> {
        match &self.mode {
            SourceCoverageStoreMode::Local => Task::ready(
                self.state
                    .provider(&provider_id)
                    .cloned()
                    .context("unknown source coverage provider"),
            ),
            SourceCoverageStoreMode::Remote { project_id, client } => {
                let project_id = *project_id;
                let client = client.clone();
                let worktree_ids = self
                    .worktree_store
                    .read(cx)
                    .visible_worktrees(cx)
                    .map(|worktree| worktree.read(cx).id().to_proto())
                    .collect();
                cx.spawn(async move |this, cx| {
                    let response = client
                        .request(proto::GetSourceCoverage {
                            project_id,
                            provider_id: provider_id.0,
                            worktree_ids,
                        })
                        .await?;
                    let snapshot = snapshot_from_proto(response)?;
                    this.update(cx, |store, cx| {
                        if store.state.project_generation != snapshot.project_generation {
                            store.state = SourceCoverageState::new(snapshot.project_generation);
                        }
                        store.state.publish(snapshot.clone())?;
                        cx.emit(SourceCoverageStoreEvent::Changed(
                            snapshot.provider_id.clone(),
                        ));
                        Ok::<(), anyhow::Error>(())
                    })??;
                    Ok(snapshot)
                })
            }
        }
    }

    async fn handle_get_source_coverage(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::GetSourceCoverage>,
        cx: AsyncApp,
    ) -> Result<proto::SourceCoverageResponse> {
        let request = envelope.payload;
        let allowed_worktrees = request
            .worktree_ids
            .into_iter()
            .map(WorktreeId::from_proto)
            .collect::<HashSet<_>>();
        this.read_with(&cx, |store, _| {
            let snapshot = store
                .state
                .provider(&SourceCoverageProviderId(request.provider_id))
                .context("unknown source coverage provider")?;
            Ok(snapshot_to_proto(snapshot, Some(&allowed_worktrees)))
        })
    }
}

pub fn snapshot_to_proto(
    snapshot: &SourceCoverageSnapshot,
    allowed_worktrees: Option<&HashSet<WorktreeId>>,
) -> proto::SourceCoverageResponse {
    let mut filtered = false;
    let files = snapshot
        .files
        .iter()
        .filter_map(|file| {
            if allowed_worktrees.is_some_and(|allowed| !allowed.contains(&file.path.worktree_id)) {
                filtered = true;
                return None;
            }
            Some(proto::SourceCoverageFile {
                path: Some(file.path.to_proto()),
                ranges: file
                    .ranges
                    .iter()
                    .map(|range| proto::SourceCoverageRange {
                        start_line: range.start.line,
                        start_column: range.start.column,
                        end_line: range.end.line,
                        end_column: range.end.column,
                        hit_count: range.hit_count,
                    })
                    .collect(),
                covered_lines: file.covered_lines,
                uncovered_lines: file.uncovered_lines,
                truncated: file.truncated,
            })
        })
        .collect();
    proto::SourceCoverageResponse {
        project_generation: snapshot.project_generation,
        provider_id: snapshot.provider_id.0.clone(),
        generation: snapshot.generation,
        status: status_to_proto(snapshot.status),
        files,
        truncated: snapshot.truncated || filtered,
        diagnostic: snapshot.diagnostic.clone(),
    }
}

pub fn snapshot_from_proto(
    response: proto::SourceCoverageResponse,
) -> Result<SourceCoverageSnapshot> {
    SourceCoverageSnapshot {
        project_generation: response.project_generation,
        provider_id: SourceCoverageProviderId(response.provider_id),
        generation: response.generation,
        status: status_from_proto(response.status),
        files: response
            .files
            .into_iter()
            .map(|file| {
                Ok(SourceCoverageFile {
                    path: ProjectPath::from_proto(
                        file.path.context("missing source coverage file path")?,
                    )
                    .context("invalid source coverage file path")?,
                    ranges: file
                        .ranges
                        .into_iter()
                        .map(|range| SourceCoverageRange {
                            start: SourceCoveragePoint {
                                line: range.start_line,
                                column: range.start_column,
                            },
                            end: SourceCoveragePoint {
                                line: range.end_line,
                                column: range.end_column,
                            },
                            hit_count: range.hit_count,
                        })
                        .collect(),
                    covered_lines: file.covered_lines,
                    uncovered_lines: file.uncovered_lines,
                    truncated: file.truncated,
                })
            })
            .collect::<Result<Vec<_>>>()?,
        truncated: response.truncated,
        diagnostic: response.diagnostic,
    }
    .bounded()
}

fn status_to_proto(status: SourceCoverageStatus) -> i32 {
    match status {
        SourceCoverageStatus::Loading => 0,
        SourceCoverageStatus::Current => 1,
        SourceCoverageStatus::Empty => 2,
        SourceCoverageStatus::Partial => 3,
        SourceCoverageStatus::Stale => 4,
        SourceCoverageStatus::Error => 5,
        SourceCoverageStatus::Restricted => 6,
        SourceCoverageStatus::Disconnected => 7,
        SourceCoverageStatus::Mismatch => 8,
    }
}

fn status_from_proto(status: i32) -> SourceCoverageStatus {
    match status {
        1 => SourceCoverageStatus::Current,
        2 => SourceCoverageStatus::Empty,
        3 => SourceCoverageStatus::Partial,
        4 => SourceCoverageStatus::Stale,
        5 => SourceCoverageStatus::Error,
        6 => SourceCoverageStatus::Restricted,
        7 => SourceCoverageStatus::Disconnected,
        8 => SourceCoverageStatus::Mismatch,
        _ => SourceCoverageStatus::Loading,
    }
}

fn truncate_utf8(value: &mut String, limit: usize) {
    while value.len() > limit {
        value.pop();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use settings::WorktreeId;
    use util::rel_path::RelPath;

    use super::*;

    fn path(value: &str, worktree: usize) -> ProjectPath {
        ProjectPath {
            worktree_id: WorktreeId::from_usize(worktree),
            path: Arc::from(RelPath::from_unix_str(value).expect("valid fixture path")),
        }
    }

    fn snapshot() -> SourceCoverageSnapshot {
        SourceCoverageSnapshot {
            project_generation: 7,
            provider_id: SourceCoverageProviderId("fake-language".to_string()),
            generation: 2,
            status: SourceCoverageStatus::Current,
            files: vec![SourceCoverageFile {
                path: path("src/lib.rs", 1),
                ranges: vec![
                    SourceCoverageRange {
                        start: SourceCoveragePoint { line: 1, column: 0 },
                        end: SourceCoveragePoint { line: 1, column: 8 },
                        hit_count: 4,
                    },
                    SourceCoverageRange {
                        start: SourceCoveragePoint { line: 4, column: 0 },
                        end: SourceCoveragePoint { line: 3, column: 0 },
                        hit_count: 0,
                    },
                ],
                covered_lines: 1,
                uncovered_lines: 1,
                truncated: false,
            }],
            truncated: false,
            diagnostic: None,
        }
    }

    #[test]
    fn source_coverage_bounds_malformed_input_and_stale_generations() {
        let mut state = SourceCoverageState::new(7);
        state.publish(snapshot()).expect("snapshot should publish");
        let current = state
            .provider(&SourceCoverageProviderId("fake-language".to_string()))
            .expect("provider should exist");
        assert_eq!(current.files[0].ranges.len(), 1);
        assert_eq!(current.status, SourceCoverageStatus::Partial);
        let mut stale = snapshot();
        stale.generation = 1;
        assert!(state.publish(stale).is_err());

        state
            .mark_provider_status(
                SourceCoverageProviderId("fake-language".to_string()),
                SourceCoverageStatus::Stale,
                Some("cancelled".to_string()),
            )
            .expect("lifecycle transition should retain bounded facts");
        let stale = state
            .provider(&SourceCoverageProviderId("fake-language".to_string()))
            .expect("provider should remain available");
        assert_eq!(stale.status, SourceCoverageStatus::Stale);
        assert_eq!(stale.files.len(), 1);
        assert_eq!(stale.diagnostic.as_deref(), Some("cancelled"));
    }

    #[test]
    fn source_coverage_remote_round_trip_filters_hidden_worktrees() {
        let mut snapshot = snapshot().bounded().expect("snapshot should bound");
        snapshot.files.push(SourceCoverageFile {
            path: path("private.rs", 2),
            ranges: Vec::new(),
            covered_lines: 0,
            uncovered_lines: 0,
            truncated: false,
        });
        let proto = snapshot_to_proto(&snapshot, Some(&HashSet::from([WorktreeId::from_usize(1)])));
        assert_eq!(proto.files.len(), 1);
        assert!(proto.truncated);
        let decoded = snapshot_from_proto(proto).expect("filtered snapshot should decode");
        assert_eq!(decoded.files[0].path, path("src/lib.rs", 1));
    }
}
