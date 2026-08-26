use std::path::Path;

use anyhow::{Context as _, Result, bail};
use gpui::{AppContext as _, AsyncApp, Context, Entity, Task};
use rpc::{AnyProtoClient, TypedEnvelope, proto};
use serde_json::Value;

use crate::{
    ProjectPath,
    source_coverage::{
        MAX_SOURCE_COVERAGE_FILES, MAX_SOURCE_COVERAGE_RANGES, MAX_SOURCE_COVERAGE_RANGES_PER_FILE,
        SourceCoverageFile, SourceCoveragePoint, SourceCoverageProviderId, SourceCoverageRange,
        SourceCoverageSnapshot, SourceCoverageStatus, SourceCoverageStore, snapshot_from_proto,
        snapshot_to_proto,
    },
    worktree_store::WorktreeStore,
};

pub const RUST_COVERAGE_PROVIDER_ID: &str = "cargo-llvm-cov";
pub const MAX_RUST_COVERAGE_ARTIFACT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustCoverageRunToken {
    project_generation: u64,
    generation: u64,
}

pub struct RustCoverageArtifactProvider {
    project_generation: u64,
    generation: u64,
}

impl RustCoverageArtifactProvider {
    pub fn new(project_generation: u64) -> Self {
        Self {
            project_generation,
            generation: 0,
        }
    }

    pub fn begin_run(&mut self) -> RustCoverageRunToken {
        self.generation = self.generation.wrapping_add(1);
        RustCoverageRunToken {
            project_generation: self.project_generation,
            generation: self.generation,
        }
    }

    pub fn cancel(&mut self, token: RustCoverageRunToken) {
        if token.project_generation == self.project_generation
            && token.generation == self.generation
        {
            self.generation = self.generation.wrapping_add(1);
        }
    }

    pub fn parse_artifact(
        &self,
        token: RustCoverageRunToken,
        contents: &[u8],
        mut resolve_path: impl FnMut(&Path) -> Option<ProjectPath>,
    ) -> Result<SourceCoverageSnapshot> {
        if token.project_generation != self.project_generation
            || token.generation != self.generation
        {
            bail!("Rust coverage artifact belongs to a stale or cancelled run");
        }
        if contents.len() > MAX_RUST_COVERAGE_ARTIFACT_BYTES {
            bail!("Rust coverage artifact exceeds the supported byte limit");
        }
        let root: Value = serde_json::from_slice(contents).context("malformed coverage JSON")?;
        if root.get("type").and_then(Value::as_str) != Some("llvm.coverage.json.export") {
            bail!("unsupported Rust coverage artifact type");
        }
        let version = root
            .get("version")
            .and_then(Value::as_str)
            .context("coverage artifact is missing its schema version")?;
        if version.split('.').next() != Some("2") {
            bail!("unsupported LLVM coverage export schema version {version}");
        }
        let collector_version = root
            .get("cargo_llvm_cov")
            .and_then(Value::as_object)
            .and_then(|metadata| metadata.get("version"))
            .and_then(Value::as_str)
            .filter(|version| !version.is_empty() && version.len() <= 64)
            .context("coverage artifact was not produced by a supported cargo-llvm-cov")?;
        let _ = collector_version;

        let data = root
            .get("data")
            .and_then(Value::as_array)
            .context("coverage artifact is missing data")?;
        let mut files = Vec::new();
        let mut truncated = false;
        let mut partial_count = 0usize;
        let mut remaining_ranges = MAX_SOURCE_COVERAGE_RANGES;
        for export in data.iter().take(16) {
            let Some(export_files) = export.get("files").and_then(Value::as_array) else {
                partial_count += 1;
                continue;
            };
            for file in export_files {
                if files.len() >= MAX_SOURCE_COVERAGE_FILES || remaining_ranges == 0 {
                    truncated = true;
                    break;
                }
                let Some(filename) = file.get("filename").and_then(Value::as_str) else {
                    partial_count += 1;
                    continue;
                };
                let Some(path) = resolve_path(Path::new(filename)) else {
                    partial_count += 1;
                    continue;
                };
                let segments = file
                    .get("segments")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let range_limit = remaining_ranges.min(MAX_SOURCE_COVERAGE_RANGES_PER_FILE);
                let mut ranges = Vec::new();
                for (index, segment) in segments.iter().enumerate() {
                    if ranges.len() >= range_limit {
                        truncated = true;
                        break;
                    }
                    let Some(segment) = segment.as_array() else {
                        partial_count += 1;
                        continue;
                    };
                    let Some(line) = segment.first().and_then(Value::as_u64) else {
                        partial_count += 1;
                        continue;
                    };
                    let Some(column) = segment.get(1).and_then(Value::as_u64) else {
                        partial_count += 1;
                        continue;
                    };
                    let Some(hit_count) = segment.get(2).and_then(Value::as_u64) else {
                        partial_count += 1;
                        continue;
                    };
                    if segment.get(3).and_then(Value::as_bool) != Some(true) {
                        continue;
                    }
                    let Ok(line) = u32::try_from(line.saturating_sub(1)) else {
                        partial_count += 1;
                        continue;
                    };
                    let Ok(column) = u32::try_from(column.saturating_sub(1)) else {
                        partial_count += 1;
                        continue;
                    };
                    let next = segments.get(index + 1).and_then(Value::as_array);
                    let end_line = next
                        .and_then(|next| next.first())
                        .and_then(Value::as_u64)
                        .and_then(|line| u32::try_from(line.saturating_sub(1)).ok())
                        .unwrap_or(line);
                    let end_column = next
                        .and_then(|next| next.get(1))
                        .and_then(Value::as_u64)
                        .and_then(|column| u32::try_from(column.saturating_sub(1)).ok())
                        .unwrap_or_else(|| column.saturating_add(1));
                    ranges.push(SourceCoverageRange {
                        start: SourceCoveragePoint { line, column },
                        end: SourceCoveragePoint {
                            line: end_line,
                            column: end_column,
                        },
                        hit_count,
                    });
                }
                remaining_ranges = remaining_ranges.saturating_sub(ranges.len());
                let lines = file.get("summary").and_then(|summary| summary.get("lines"));
                let total = lines
                    .and_then(|lines| lines.get("count"))
                    .and_then(Value::as_u64)
                    .and_then(|count| u32::try_from(count).ok())
                    .unwrap_or_default();
                let covered = lines
                    .and_then(|lines| lines.get("covered"))
                    .and_then(Value::as_u64)
                    .and_then(|count| u32::try_from(count).ok())
                    .unwrap_or_default();
                files.push(SourceCoverageFile {
                    path,
                    ranges,
                    covered_lines: covered.min(total),
                    uncovered_lines: total.saturating_sub(covered),
                    truncated,
                });
            }
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(SourceCoverageSnapshot {
            project_generation: self.project_generation,
            provider_id: SourceCoverageProviderId(RUST_COVERAGE_PROVIDER_ID.to_string()),
            generation: token.generation,
            status: if partial_count > 0 || truncated {
                SourceCoverageStatus::Partial
            } else if files.is_empty() {
                SourceCoverageStatus::Empty
            } else {
                SourceCoverageStatus::Current
            },
            files,
            truncated,
            diagnostic: (partial_count > 0).then(|| {
                format!("Ignored {partial_count} malformed or non-visible coverage item(s)")
            }),
        })
    }
}

pub struct RustCoverageProviderStore {
    mode: RustCoverageProviderStoreMode,
    worktree_store: Entity<WorktreeStore>,
    source_coverage_store: Entity<SourceCoverageStore>,
    provider: RustCoverageArtifactProvider,
}

enum RustCoverageProviderStoreMode {
    Local,
    Remote {
        project_id: u64,
        client: AnyProtoClient,
    },
}

impl RustCoverageProviderStore {
    pub fn init(client: &AnyProtoClient) {
        client.add_entity_request_handler(Self::handle_interpret_artifact);
    }

    pub fn local(
        worktree_store: Entity<WorktreeStore>,
        source_coverage_store: Entity<SourceCoverageStore>,
        project_generation: u64,
    ) -> Self {
        Self {
            mode: RustCoverageProviderStoreMode::Local,
            worktree_store,
            source_coverage_store,
            provider: RustCoverageArtifactProvider::new(project_generation),
        }
    }

    pub fn remote(
        project_id: u64,
        client: AnyProtoClient,
        worktree_store: Entity<WorktreeStore>,
        source_coverage_store: Entity<SourceCoverageStore>,
        project_generation: u64,
    ) -> Self {
        Self {
            mode: RustCoverageProviderStoreMode::Remote { project_id, client },
            worktree_store,
            source_coverage_store,
            provider: RustCoverageArtifactProvider::new(project_generation),
        }
    }

    pub fn interpret_artifact(
        &mut self,
        artifact_path: ProjectPath,
        max_bytes: usize,
        cx: &mut Context<Self>,
    ) -> Task<Result<SourceCoverageSnapshot>> {
        match &self.mode {
            RustCoverageProviderStoreMode::Local => {
                self.interpret_local(artifact_path, max_bytes, cx)
            }
            RustCoverageProviderStoreMode::Remote { project_id, client } => {
                let project_id = *project_id;
                let client = client.clone();
                let source_coverage_store = self.source_coverage_store.clone();
                cx.spawn(async move |_this, cx| {
                    let response = client
                        .request(proto::InterpretRustCoverageArtifact {
                            project_id,
                            artifact_path: Some(artifact_path.to_proto()),
                            max_bytes: u64::try_from(max_bytes).unwrap_or(u64::MAX),
                        })
                        .await
                        .map_err(map_remote_coverage_error)?;
                    let snapshot = snapshot_from_proto(response)?;
                    source_coverage_store
                        .update(cx, |store, cx| store.replace_remote(snapshot.clone(), cx))?;
                    Ok(snapshot)
                })
            }
        }
    }

    fn interpret_local(
        &mut self,
        artifact_path: ProjectPath,
        max_bytes: usize,
        cx: &mut Context<Self>,
    ) -> Task<Result<SourceCoverageSnapshot>> {
        let max_bytes = max_bytes.min(MAX_RUST_COVERAGE_ARTIFACT_BYTES);
        let Some(worktree) = self
            .worktree_store
            .read(cx)
            .worktree_for_id(artifact_path.worktree_id, cx)
        else {
            return Task::ready(Err(anyhow::anyhow!(
                "coverage artifact worktree is not visible"
            )));
        };
        let (entry_size, roots) = worktree.read_with(cx, |worktree, cx| {
            let snapshot = worktree.snapshot();
            let entry_size = snapshot
                .entry_for_path(artifact_path.path.as_ref())
                .filter(|entry| !entry.is_private && entry.is_file())
                .map(|entry| entry.size);
            let roots = self
                .worktree_store
                .read(cx)
                .visible_worktrees(cx)
                .map(|worktree| {
                    worktree.read_with(cx, |worktree, _| {
                        (worktree.id(), worktree.abs_path().to_path_buf())
                    })
                })
                .collect::<Vec<_>>();
            (entry_size, roots)
        });
        let Some(entry_size) = entry_size else {
            return Task::ready(Err(anyhow::anyhow!(
                "coverage artifact is missing, private, or not a file"
            )));
        };
        if usize::try_from(entry_size).unwrap_or(usize::MAX) > max_bytes {
            return Task::ready(Err(anyhow::anyhow!(
                "coverage artifact exceeds the declared byte limit"
            )));
        }
        let load = worktree.update(cx, |worktree, cx| {
            worktree.load_file(artifact_path.path.as_ref(), cx)
        });
        let token = self.provider.begin_run();
        let project_generation = token.project_generation;
        let generation = token.generation;
        let source_coverage_store = self.source_coverage_store.clone();
        cx.spawn(async move |this, cx| {
            let loaded = load.await?;
            if loaded.text.len() > max_bytes {
                bail!("coverage artifact exceeds the declared byte limit");
            }
            let parse = cx.background_spawn(async move {
                let provider = RustCoverageArtifactProvider {
                    project_generation,
                    generation,
                };
                provider.parse_artifact(token, loaded.text.as_bytes(), |absolute_path| {
                    roots.iter().find_map(|(worktree_id, root)| {
                        let relative = absolute_path.strip_prefix(root).ok()?;
                        let path =
                            util::rel_path::RelPath::new(relative, util::paths::PathStyle::local())
                                .ok()?;
                        Some(ProjectPath {
                            worktree_id: *worktree_id,
                            path: path.as_ref().into(),
                        })
                    })
                })
            });
            let snapshot = parse.await?;
            this.read_with(cx, |store, _| {
                if store.provider.generation != token.generation {
                    bail!("Rust coverage artifact completed after cancellation");
                }
                Ok(())
            })??;
            source_coverage_store.update(cx, |store, cx| store.publish(snapshot.clone(), cx))?;
            Ok(snapshot)
        })
    }

    async fn handle_interpret_artifact(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::InterpretRustCoverageArtifact>,
        mut cx: AsyncApp,
    ) -> Result<proto::SourceCoverageResponse> {
        let request = envelope.payload;
        let artifact_path = ProjectPath::from_proto(
            request
                .artifact_path
                .context("missing Rust coverage artifact path")?,
        )
        .context("invalid Rust coverage artifact path")?;
        let max_bytes = usize::try_from(request.max_bytes)
            .unwrap_or(usize::MAX)
            .min(MAX_RUST_COVERAGE_ARTIFACT_BYTES);
        let snapshot = this
            .update(&mut cx, |store, cx| {
                store.interpret_local(artifact_path, max_bytes, cx)
            })
            .await?;
        Ok(snapshot_to_proto(&snapshot, None))
    }
}

fn map_remote_coverage_error(error: anyhow::Error) -> anyhow::Error {
    let unsupported = error
        .downcast_ref::<proto::RpcError>()
        .is_some_and(|error| error.raw_message().contains("was not handled"));
    if unsupported {
        anyhow::anyhow!("The project host does not support Rust coverage")
    } else {
        anyhow::anyhow!("The Rust coverage host rejected the request: {error}")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use settings::WorktreeId;
    use util::rel_path::RelPath;

    use super::*;

    fn resolve(path: &Path) -> Option<ProjectPath> {
        let relative = path.strip_prefix("/workspace").ok()?;
        Some(ProjectPath {
            worktree_id: WorktreeId::from_usize(1),
            path: Arc::from(
                RelPath::new(relative, util::paths::PathStyle::Unix)
                    .ok()?
                    .as_ref(),
            ),
        })
    }

    #[test]
    fn rust_coverage_provider_validates_supported_artifacts_paths_and_cancellation() {
        let mut provider = RustCoverageArtifactProvider::new(5);
        let token = provider.begin_run();
        let artifact = br#"{
          "type":"llvm.coverage.json.export","version":"2.0.1",
          "cargo_llvm_cov":{"version":"0.9.0","manifest_path":"/workspace/Cargo.toml"},
          "data":[{"files":[
            {"filename":"/workspace/src/lib.rs","segments":[[1,1,3,true,true,false],[2,1,0,true,true,false]],"summary":{"lines":{"count":2,"covered":1}}},
            {"filename":"/private/cache.rs","segments":[],"summary":{"lines":{"count":1,"covered":0}}},
            {"segments":[]}
          ]}]
        }"#;
        let snapshot = provider
            .parse_artifact(token, artifact, resolve)
            .expect("supported artifact should parse");
        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.status, SourceCoverageStatus::Partial);
        assert_eq!(snapshot.files[0].ranges.len(), 2);
        provider.cancel(token);
        assert!(provider.parse_artifact(token, artifact, resolve).is_err());
    }

    #[test]
    fn rust_coverage_provider_rejects_malformed_unsupported_and_oversized_artifacts() {
        let mut provider = RustCoverageArtifactProvider::new(1);
        let token = provider.begin_run();
        assert!(provider.parse_artifact(token, b"{}", resolve).is_err());
        assert!(
            provider
                .parse_artifact(
                    token,
                    &vec![b' '; MAX_RUST_COVERAGE_ARTIFACT_BYTES + 1],
                    resolve
                )
                .is_err()
        );
    }
}
