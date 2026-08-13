#![recursion_limit = "256"]

use comfy_api::SafeVirtualPath;
use comfy_plugin_host::AssetPluginCapabilityServices;
use comfy_plugin_sdk::{CapabilityKind, CapabilityQuota, CapabilityRequest};
use comfy_runtime::{
    AssetIdentity, AssetNamespace, AssetOperation, Capability, ExternalNavigationPolicy,
    authorize_native_output_committer, authorize_native_output_ui,
    authorize_native_plugin_asset_broker, open_native_profile_asset_service,
};
use comfy_tensor::{
    BackendCapabilityMatrix, CpuWorkspaceAuthority, DType, DeviceId, ExecutionContext, StreamId,
    TensorDescriptor, TensorError,
};
use comfy_types::DeviceKind;
use comfy_worker::{
    AttemptMemoryController, MemoryPlanRequest, MemoryReservationKind, WorkerSession,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

fn repository_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("comfy_test_support has no repository root")?
        .to_path_buf())
}

fn validation_target_directory(root: &Path) -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR") {
        Some(directory) => {
            let directory = PathBuf::from(directory);
            if directory.is_absolute() {
                directory
            } else {
                root.join(directory)
            }
        }
        None => root.join("target"),
    }
}

fn rust_sources(root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    fn visit(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
        let mut entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if comfy_test_support::is_apple_double_metadata(&path) {
                continue;
            }
            if path.is_dir() {
                if !matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("target" | ".git" | ".agents" | "projects" | "node_modules")
                ) {
                    visit(&path, files)?;
                }
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                files.push(path);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    visit(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn source_occurrences(sources: &[(PathBuf, String)], needle: &str) -> Vec<String> {
    let mut matches = Vec::new();
    for (path, source) in sources {
        if path.file_name().and_then(|name| name.to_str()) == Some("ownership_consolidation.rs") {
            continue;
        }
        for (line_index, line) in source.lines().enumerate() {
            if line.contains(needle) {
                matches.push(format!("{}:{}", path.display(), line_index + 1));
            }
        }
    }
    matches
}

fn is_test_only_source(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "tests")
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("_test.rs") || name.ends_with("_tests.rs"))
}

fn production_source_occurrences(sources: &[(PathBuf, String)], needle: &str) -> Vec<String> {
    let mut matches = Vec::new();
    for (path, source) in sources {
        if is_test_only_source(path) {
            continue;
        }
        let production = source
            .split_once("#[cfg(test)]\nmod tests")
            .map_or(source.as_str(), |(production, _)| production);
        for (line_index, line) in production.lines().enumerate() {
            if line.contains(needle) {
                matches.push(format!("{}:{}", path.display(), line_index + 1));
            }
        }
    }
    matches
}

fn production_identifier_occurrences(
    sources: &[(PathBuf, String)],
    identifier: &str,
) -> Vec<String> {
    let mut matches = Vec::new();
    for (path, source) in sources {
        if is_test_only_source(path) {
            continue;
        }
        let production = source
            .split_once("#[cfg(test)]\nmod tests")
            .map_or(source.as_str(), |(production, _)| production);
        for (line_index, line) in production.lines().enumerate() {
            for (column, _) in line.match_indices(identifier) {
                let preceding = line[..column].chars().next_back();
                let following = line[column + identifier.len()..].chars().next();
                let is_identifier_character =
                    |character: char| character.is_ascii_alphanumeric() || character == '_';
                if !preceding.is_some_and(is_identifier_character)
                    && !following.is_some_and(is_identifier_character)
                {
                    matches.push(format!(
                        "{}:{}:{}",
                        path.display(),
                        line_index + 1,
                        column + 1
                    ));
                }
            }
        }
    }
    matches
}

fn occurrence_files(
    root: &Path,
    locations: &[String],
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    locations
        .iter()
        .map(|location| {
            let source_end = location
                .find(".rs:")
                .ok_or_else(|| format!("source occurrence has no Rust path: {location}"))?
                + 3;
            let source_path = Path::new(&location[..source_end]);
            Ok(source_path
                .strip_prefix(root)?
                .to_string_lossy()
                .into_owned())
        })
        .collect()
}

fn exact_occurrence_files(
    root: &Path,
    locations: &[String],
    expected: &[&str],
) -> Result<bool, Box<dyn std::error::Error>> {
    let expected = expected
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    Ok(occurrence_files(root, locations)? == expected)
}

fn declaration_derives_trait(source: &str, declaration: &str, trait_name: &str) -> bool {
    let Some(declaration_offset) = source.find(declaration) else {
        return false;
    };
    let prefix = &source[..declaration_offset];
    let Some(derive_offset) = prefix.rfind("#[derive(") else {
        return false;
    };
    let derive = &prefix[derive_offset..];
    let Some(derive_end) = derive.find(")]") else {
        return false;
    };
    if derive[derive_end + 2..]
        .chars()
        .any(|character| matches!(character, '{' | '}' | ';'))
    {
        return false;
    }
    derive["#[derive(".len()..derive_end]
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|candidate| candidate == trait_name)
}

fn file_sha256(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn task_completion_statuses(tasks: &str) -> Result<BTreeMap<String, bool>, String> {
    let mut statuses = BTreeMap::new();
    let mut current_complete = None;
    for (line_index, line) in tasks.lines().enumerate() {
        let task_status = if line.starts_with("- [x] ") || line.starts_with("- [X] ") {
            Some(true)
        } else if line.starts_with("- [ ] ")
            || line.starts_with("- [~] ")
            || line.starts_with("- [-] ")
        {
            Some(false)
        } else {
            None
        };
        if let Some(task_status) = task_status {
            if current_complete.is_some() {
                return Err(format!(
                    "task heading at line {} follows a task with no _id",
                    line_index + 1
                ));
            }
            current_complete = Some(task_status);
            continue;
        }
        if let Some(identifier) = line.trim().strip_prefix("- _id: ") {
            let complete = current_complete.take().ok_or_else(|| {
                format!("orphan task _id {identifier:?} at line {}", line_index + 1)
            })?;
            if identifier.is_empty() {
                return Err(format!("empty task _id at line {}", line_index + 1));
            }
            if statuses.insert(identifier.to_owned(), complete).is_some() {
                return Err(format!("duplicate task _id {identifier:?}"));
            }
        }
    }
    if current_complete.is_some() {
        return Err("final task heading has no _id".to_owned());
    }
    Ok(statuses)
}

fn parse_csv_records(csv: &str) -> Result<Vec<Vec<String>>, String> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut characters = csv.chars().peekable();
    let mut in_quotes = false;
    let mut after_closing_quote = false;

    while let Some(character) = characters.next() {
        if in_quotes {
            if character == '"' {
                if characters.peek() == Some(&'"') {
                    characters.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                    after_closing_quote = true;
                }
            } else {
                field.push(character);
            }
            continue;
        }

        if after_closing_quote {
            match character {
                ',' => {
                    record.push(std::mem::take(&mut field));
                    after_closing_quote = false;
                }
                '\n' => {
                    record.push(std::mem::take(&mut field));
                    records.push(std::mem::take(&mut record));
                    after_closing_quote = false;
                }
                '\r' if characters.peek() == Some(&'\n') => {
                    characters.next();
                    record.push(std::mem::take(&mut field));
                    records.push(std::mem::take(&mut record));
                    after_closing_quote = false;
                }
                _ => return Err("unexpected character after closing CSV quote".to_owned()),
            }
            continue;
        }

        match character {
            '"' if field.is_empty() => in_quotes = true,
            '"' => return Err("unexpected CSV quote inside an unquoted field".to_owned()),
            ',' => record.push(std::mem::take(&mut field)),
            '\n' => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            '\r' if characters.peek() == Some(&'\n') => {
                characters.next();
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            '\r' => return Err("bare carriage return in CSV input".to_owned()),
            _ => field.push(character),
        }
    }

    if in_quotes {
        return Err("unterminated quoted CSV field".to_owned());
    }
    if after_closing_quote || !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    Ok(records)
}

fn accounted_pending_ownership_rows(
    ownership_catalog: &str,
    policy_concerns: &[serde_json::Value],
    task_statuses: &BTreeMap<String, bool>,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut records = parse_csv_records(ownership_catalog)?.into_iter();
    let header = records
        .next()
        .ok_or_else(|| "ownership catalog has no header".to_owned())?;
    let mut header_positions = BTreeMap::new();
    for (index, column) in header.iter().enumerate() {
        if header_positions.insert(column.as_str(), index).is_some() {
            return Err(format!("ownership catalog has duplicate column {column:?}"));
        }
    }
    let concern_index = *header_positions
        .get("concern")
        .ok_or_else(|| "ownership catalog has no concern column".to_owned())?;
    let status_index = *header_positions
        .get("current_status")
        .ok_or_else(|| "ownership catalog has no current_status column".to_owned())?;

    let mut accounted = BTreeMap::new();
    let mut seen_concerns = std::collections::BTreeSet::<String>::new();
    for (row_index, row) in records.enumerate() {
        if row.len() != header.len() {
            return Err(format!(
                "ownership catalog row {} has {} fields, expected {}",
                row_index + 2,
                row.len(),
                header.len()
            ));
        }
        let concern = row
            .get(concern_index)
            .filter(|concern| !concern.is_empty())
            .ok_or_else(|| format!("ownership catalog row {} has no concern", row_index + 2))?;
        if !seen_concerns.insert(concern.clone()) {
            return Err(format!(
                "ownership catalog has duplicate concern {concern:?}"
            ));
        }
        let status = row
            .get(status_index)
            .filter(|status| !status.is_empty())
            .ok_or_else(|| {
                format!(
                    "ownership catalog row {} has no current_status",
                    row_index + 2
                )
            })?;
        if status == "authoritative_owner_confirmed" {
            continue;
        }
        let policy = policy_concerns
            .iter()
            .find(|entry| {
                entry.get("concern").and_then(serde_json::Value::as_str) == Some(concern.as_str())
            })
            .ok_or_else(|| format!("unresolved ownership row {concern} has no policy entry"))?;
        let mapped_tasks = policy
            .get("consolidation_tasks")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                format!("unresolved ownership row {concern} has no mapped closure tasks")
            })?;
        let mut pending = Vec::new();
        for task in mapped_tasks {
            let identifier = task.as_str().ok_or_else(|| {
                format!("unresolved ownership row {concern} has a non-string closure task")
            })?;
            let complete = task_statuses.get(identifier).ok_or_else(|| {
                format!("unresolved ownership row {concern} maps missing task {identifier}")
            })?;
            if !complete {
                pending.push(identifier.to_owned());
            }
        }
        if pending.is_empty() {
            return Err(format!(
                "unresolved ownership row {concern} has no existing incomplete closure task"
            ));
        }
        accounted.insert(concern.to_owned(), pending);
    }
    Ok(accounted)
}

fn workspace_accounting_chain_uses_authoritative_owners() -> Result<bool, Box<dyn std::error::Error>>
{
    const WORKSPACE_BYTES: u64 = 128;
    let mut controller = AttemptMemoryController::new(
        512 * 1024 * 1024,
        0,
        MemoryPlanRequest {
            workspace_bytes: WORKSPACE_BYTES,
            ..MemoryPlanRequest::default()
        },
    )?;
    let planner_retains_the_exact_ceiling = controller.workspace_authorization_bytes()
        == WORKSPACE_BYTES
        && controller
            .current_plan()
            .reservations
            .iter()
            .any(|reservation| {
                reservation.kind == MemoryReservationKind::Workspace
                    && reservation.bytes == WORKSPACE_BYTES
            });

    let session = WorkerSession::new(512)?;
    let backend = session.backend().ok_or("worker backend is unavailable")?;
    let planned_authorization = controller.issue_workspace_authorization()?;
    let planner_issued_exactly_one_opaque_ceiling = planned_authorization.bytes()
        == WORKSPACE_BYTES
        && controller.issue_workspace_authorization().is_err();
    let authorization = session.authorize_planned_workspace(planned_authorization)?;
    let cancellation = comfy_types::CancellationToken::default();
    let context = ExecutionContext {
        stream: StreamId::DEFAULT,
        scratch: authorization.clone(),
        rng_phase: None,
        cancellation: &cancellation,
    };
    let descriptor =
        TensorDescriptor::contiguous(vec![16], DType::F32, DeviceId::CPU, StreamId::DEFAULT)?;
    let (tensor, event) = backend.allocate(descriptor, &context)?;
    backend.wait_event(event, &context)?;
    let allocation_uses_the_tensor_capacity_owner = authorization.in_use_bytes() == 0
        && authorization.peak_bytes() == 0
        && session.memory_snapshot()?.current_bytes == 64;
    drop(tensor);
    let tensor_drop_releases_backend_memory = session.memory_snapshot()?.current_bytes == 0;

    let workspace = backend.reserve_workspace(&context, 64)?;
    let workspace_uses_both_authoritative_owners = authorization.in_use_bytes() == 64
        && authorization.peak_bytes() == 64
        && session.memory_snapshot()?.current_bytes == 64;
    drop(workspace);
    let workspace_drop_releases_both_owners =
        authorization.in_use_bytes() == 0 && session.memory_snapshot()?.current_bytes == 0;
    let over_ceiling_is_typed = matches!(
        backend.reserve_workspace(&context, WORKSPACE_BYTES + 1),
        Err(TensorError::WorkspaceAuthorizationExceeded { .. })
    ) && authorization.in_use_bytes() == 0
        && session.memory_snapshot()?.current_bytes == 0;

    let (other_backend, _other_workspace_authority) = CpuWorkspaceAuthority::create_backend(512)?;
    let other_context = ExecutionContext {
        stream: StreamId::DEFAULT,
        scratch: authorization.clone(),
        rng_phase: None,
        cancellation: &cancellation,
    };
    let cross_backend_authorization_is_typed = matches!(
        comfy_tensor::TensorBackend::reserve_workspace(&other_backend, &other_context, 1),
        Err(TensorError::WorkspaceAuthorizationMismatch { .. })
    ) && authorization.in_use_bytes() == 0
        && other_backend.memory_snapshot().current_bytes == 0;

    let cancelled = comfy_types::CancellationToken::default();
    cancelled.cancel();
    let cancelled_context = ExecutionContext {
        stream: StreamId::DEFAULT,
        scratch: authorization.clone(),
        rng_phase: None,
        cancellation: &cancelled,
    };
    let one = TensorDescriptor::contiguous(vec![1], DType::F32, DeviceId::CPU, StreamId::DEFAULT)?;
    let cancellation_precedes_workspace_mutation = matches!(
        backend.allocate(one, &cancelled_context),
        Err(TensorError::Cancelled)
    ) && authorization.in_use_bytes() == 0
        && session.memory_snapshot()?.current_bytes == 0;

    let passed = planner_retains_the_exact_ceiling
        && planner_issued_exactly_one_opaque_ceiling
        && allocation_uses_the_tensor_capacity_owner
        && tensor_drop_releases_backend_memory
        && workspace_uses_both_authoritative_owners
        && workspace_drop_releases_both_owners
        && over_ceiling_is_typed
        && cross_backend_authorization_is_typed
        && cancellation_precedes_workspace_mutation;
    if !passed {
        eprintln!(
            "workspace accounting chain: planner_ceiling={planner_retains_the_exact_ceiling}, \
             single_authorization={planner_issued_exactly_one_opaque_ceiling}, \
             tensor_capacity={allocation_uses_the_tensor_capacity_owner}, \
             tensor_drop={tensor_drop_releases_backend_memory}, \
             workspace_dual_accounting={workspace_uses_both_authoritative_owners}, \
             workspace_drop={workspace_drop_releases_both_owners}, \
             typed_ceiling={over_ceiling_is_typed}, \
             cross_backend_rejection={cross_backend_authorization_is_typed}, \
             cancellation_order={cancellation_precedes_workspace_mutation}"
        );
    }
    Ok(passed)
}

fn run_ownership_validation(
    validation: &str,
    scope: &str,
    artifact_filename: &str,
    exact_test_name: &str,
    case_prefix: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;

    let cancellation_definitions =
        source_occurrences(&sources, &["pub struct ", "CancellationToken"].concat());
    let permission_policy_definitions = source_occurrences(
        &sources,
        &["pub struct ", "PermissionPolicy", " {"].concat(),
    );
    let plugin_trust_definitions = source_occurrences(
        &sources,
        &["pub struct ", "PluginTrustPolicy", " {"].concat(),
    );
    let backend_matrix_definitions = source_occurrences(
        &sources,
        &["pub struct ", "BackendCapabilityMatrix"].concat(),
    );
    let backend_readiness_definitions =
        source_occurrences(&sources, &["pub fn ", "for_native_device", "("].concat());
    let backend_binding_definitions =
        source_occurrences(&sources, &["pub trait ", "NativeBackendBinding"].concat());
    let provider_policy_definitions =
        source_occurrences(&sources, &["pub struct ", "ProviderPolicy", " {"].concat());
    let external_navigation_policy_definitions = source_occurrences(
        &sources,
        &["pub struct ", "ExternalNavigationPolicy", " {"].concat(),
    );
    let execution_queue_definitions =
        source_occurrences(&sources, &["pub struct ", "ExecutionQueue", " {"].concat());
    let execution_owner_definitions = source_occurrences(
        &sources,
        &["pub struct ", "ExecutionPresentationOwner", " {"].concat(),
    );
    let native_queue_definitions =
        source_occurrences(&sources, &["pub struct ", "NativeQueue", " {"].concat());
    let artifact_root_definitions =
        source_occurrences(&sources, &["pub struct ", "ArtifactRoot", " {"].concat());
    let artifact_root_recursive_enumeration_definitions =
        production_source_occurrences(&sources, "pub fn list_contained_regular_files_recursive(");
    let native_package_capture_definitions =
        production_source_occurrences(&sources, "pub(crate) fn capture_native_package(");
    let native_package_coverage_definitions =
        production_source_occurrences(&sources, "pub(crate) fn validate_native_package_coverage(");
    let native_package_capture_sites =
        production_identifier_occurrences(&sources, "capture_native_package");
    let native_package_coverage_sites =
        production_identifier_occurrences(&sources, "validate_native_package_coverage");
    let artifact_index_definitions =
        source_occurrences(&sources, &["pub struct ", "ArtifactIndex", " {"].concat());
    let asset_service_definitions =
        source_occurrences(&sources, &["pub struct ", "AssetService", " {"].concat());
    let model_store_definitions =
        source_occurrences(&sources, &["pub struct ", "ModelStore", " {"].concat());
    let prompt_compiler_definitions =
        production_source_occurrences(&sources, "pub struct PromptCompiler<'a> {");
    let native_cache_definitions =
        production_source_occurrences(&sources, "pub struct NativeCache {");
    let cache_key_definitions = production_source_occurrences(&sources, "pub struct CacheKey {");
    let recovery_journal_definitions =
        production_source_occurrences(&sources, "pub struct RecoveryJournal {");
    let recovery_output_receipt_definitions =
        production_source_occurrences(&sources, "pub struct RecoveryOutputReceipt {");
    let output_commit_receipt_definitions =
        production_source_occurrences(&sources, "pub struct OutputCommitReceipt {");
    let model_archive_entry_definitions =
        source_occurrences(&sources, &["pub struct ", "ArchiveEntry", " {"].concat());
    let output_committer_definitions =
        source_occurrences(&sources, &["pub struct ", "OutputCommitter", " {"].concat());
    let runtime_database_definitions =
        source_occurrences(&sources, &["pub struct ", "ComfyRuntimeDb", "("].concat());
    let graph_context_binding_definitions =
        source_occurrences(&sources, "pub(crate) struct GraphContextActionBinding");
    let graph_context_dispatch_definitions =
        source_occurrences(&sources, "pub(crate) fn dispatch_context_action(");
    let graph_context_dispatch_sites =
        production_identifier_occurrences(&sources, "dispatch_context_action");
    let attempt_memory_controller_definitions = production_source_occurrences(
        &sources,
        &["pub struct ", "AttemptMemoryController"].concat(),
    );
    let memory_planner_definitions =
        production_source_occurrences(&sources, &["pub struct ", "MemoryPlanner"].concat());
    let scratch_reservation_definitions =
        production_source_occurrences(&sources, &["pub struct ", "ScratchReservation"].concat());
    let backend_workspace_authority_definitions = production_source_occurrences(
        &sources,
        &["pub struct ", "BackendWorkspaceAuthority"].concat(),
    );
    let cpu_workspace_authority_aliases = production_source_occurrences(
        &sources,
        "pub type CpuWorkspaceAuthority = BackendWorkspaceAuthority;",
    );
    let planned_workspace_authorization_definitions = production_source_occurrences(
        &sources,
        &["pub struct ", "PlannedWorkspaceAuthorization"].concat(),
    );
    let workspace_authorizer_definitions =
        production_source_occurrences(&sources, "fn authorize_workspace(");
    let planned_workspace_authorizer_definitions =
        production_source_occurrences(&sources, "fn authorize_planned_workspace(");
    let scratch_binding_sites =
        production_source_occurrences(&sources, "ScratchReservation::bound(");
    let unpaired_cpu_backend_constructors =
        production_source_occurrences(&sources, "CpuBackend::new(");
    let zero_scratch_sites = production_source_occurrences(&sources, "ScratchReservation::none()");
    let legacy_symmetric_eigen_decomposition_sites =
        production_source_occurrences(&sources, "pub fn symmetric_eigen_decomposition(");
    let legacy_workspace_context_sites = [
        "legacy_context",
        "legacy_execution_context",
        "transitional_context",
        "tracked_workspace",
        "LegacyCompatibility",
    ]
    .into_iter()
    .flat_map(|needle| production_source_occurrences(&sources, needle))
    .filter(|location| {
        location.contains("crates/comfy_tensor/") || location.contains("crates/comfy_model/")
    })
    .collect::<Vec<_>>();
    let backend_workspace_lease_definitions =
        production_source_occurrences(&sources, &["pub struct ", "BackendWorkspaceLease"].concat());
    let cpu_workspace_vector_definitions =
        production_source_occurrences(&sources, &["pub struct ", "CpuWorkspaceVec"].concat());
    let backend_memory_tracker_definitions =
        production_source_occurrences(&sources, &["struct ", "BackendMemoryTracker"].concat());
    let normative_memory_artifact_writer_sites =
        source_occurrences(&sources, "\"val-memory-001.json\"");
    let normative_vae_artifact_writer_sites = source_occurrences(&sources, "\"val-vae-001.json\"");

    let cpu_matrix = BackendCapabilityMatrix::for_native_device(DeviceId::CPU)?;
    let all_device_readiness_is_typed = !cpu_matrix.supported().is_empty()
        && DeviceKind::ALL
            .into_iter()
            .filter(|device| *device != DeviceKind::Cpu)
            .all(|device| {
                BackendCapabilityMatrix::for_native_device(DeviceId::new(device, 0))
                    .is_err_and(|error| error.device() == device && !error.reason().is_empty())
            })
        && BackendCapabilityMatrix::for_native_device(DeviceId::new(DeviceKind::Cpu, 1))
            .is_err_and(|error| error.device() == DeviceKind::Cpu && !error.reason().is_empty());

    let backend_adapters = [
        "crates/comfy_backend_corex/src/comfy_backend_corex.rs",
        "crates/comfy_backend_cuda/src/comfy_backend_cuda.rs",
        "crates/comfy_backend_directml/src/comfy_backend_directml.rs",
        "crates/comfy_backend_metal/src/comfy_backend_metal.rs",
        "crates/comfy_backend_mlu/src/comfy_backend_mlu.rs",
        "crates/comfy_backend_npu/src/comfy_backend_npu.rs",
        "crates/comfy_backend_rocm/src/comfy_backend_rocm.rs",
        "crates/comfy_backend_xpu/src/comfy_backend_xpu.rs",
    ];
    let backend_adapter_sources = backend_adapters
        .iter()
        .map(|relative| fs::read_to_string(root.join(relative)))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let backend_adapters_are_binding_only = backend_adapter_sources.iter().all(|source| {
        let production = source
            .split_once("#[cfg(test)]\nmod tests")
            .map_or(source.as_str(), |(production, _)| production);
        production.contains("impl NativeBackendBinding")
            && production.contains("NativeBackendBindingStatus::unbound")
            && !production.contains("BackendCapabilityMatrix")
            && !production.contains("BackendUnavailable")
    });
    if !backend_adapters_are_binding_only {
        for (path, source) in backend_adapters.iter().zip(&backend_adapter_sources) {
            let production = source
                .split_once("#[cfg(test)]\nmod tests")
                .map_or(source.as_str(), |(production, _)| production);
            eprintln!(
                "backend binding-only audit failed for {path}: binding={}, unbound={}, capability={}, unavailable={}",
                production.contains("impl NativeBackendBinding"),
                production.contains("NativeBackendBindingStatus::unbound"),
                production.contains("BackendCapabilityMatrix"),
                production.contains("BackendUnavailable"),
            );
        }
    }
    let plugin_host_capabilities =
        fs::read_to_string(root.join("crates/comfy_plugin_host/src/capabilities.rs"))?;
    let plugin_host_production_capabilities = plugin_host_capabilities
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(plugin_host_capabilities.as_str(), |(production, _)| {
            production
        });
    let runtime_assets = fs::read_to_string(root.join("crates/comfy_runtime/src/assets.rs"))?;
    let model_vae_image = fs::read_to_string(root.join("crates/comfy_model/src/vae_image.rs"))?;
    let model_crate_root = fs::read_to_string(root.join("crates/comfy_model/src/comfy_model.rs"))?;
    let tensor_activation = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/activation_normalization_functional_01.rs"),
    )?;
    let tensor_functional = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/neural_network_functional_01.rs"),
    )?;
    let tensor_module =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/neural_network_module_02.rs"))?;
    let model_artifact_index =
        fs::read_to_string(root.join("crates/comfy_model/src/artifact_index.rs"))?;
    let model_artifact_index_production = model_artifact_index
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(model_artifact_index.as_str(), |(production, _)| production);
    let model_formats = fs::read_to_string(root.join("crates/comfy_model/src/formats.rs"))?;
    let model_restricted_pickle =
        fs::read_to_string(root.join("crates/comfy_model/src/restricted_pickle.rs"))?;
    let runtime_trust = fs::read_to_string(root.join("crates/comfy_runtime/src/trust.rs"))?;
    let runtime_trust_production = runtime_trust
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_trust.as_str(), |(production, _)| production);
    let runtime_settings = fs::read_to_string(root.join("crates/comfy_runtime/src/settings.rs"))?;
    let runtime_rocm_ffi =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_rocm.rs"))?;
    let runtime_rocm_ffi_production = runtime_rocm_ffi
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_rocm_ffi.as_str(), |(production, _)| production);
    let runtime_metal_ffi =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_metal.rs"))?;
    let runtime_metal_ffi_production = runtime_metal_ffi
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_metal_ffi.as_str(), |(production, _)| production);
    let runtime_mlu_ffi =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_mlu.rs"))?;
    let runtime_mlu_ffi_production = runtime_mlu_ffi
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_mlu_ffi.as_str(), |(production, _)| production);
    let runtime_npu_ffi =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_npu.rs"))?;
    let runtime_npu_ffi_production = runtime_npu_ffi
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_npu_ffi.as_str(), |(production, _)| production);
    let runtime_xpu_ffi =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_xpu.rs"))?;
    let runtime_xpu_ffi_production = runtime_xpu_ffi
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_xpu_ffi.as_str(), |(production, _)| production);
    let runtime_cuda_ffi =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_cuda.rs"))?;
    let runtime_cuda_ffi_production = runtime_cuda_ffi
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_cuda_ffi.as_str(), |(production, _)| production);
    let runtime_directml_ffi =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_directml.rs"))?;
    let runtime_directml_ffi_production = runtime_directml_ffi
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_directml_ffi.as_str(), |(production, _)| production);
    let gpui_windows_directx_renderer =
        fs::read_to_string(root.join("crates/gpui_windows/src/directx_renderer.rs"))?;
    let gpui_windows_directx_devices =
        fs::read_to_string(root.join("crates/gpui_windows/src/directx_devices.rs"))?;
    let backend_rocm_loader =
        fs::read_to_string(root.join("crates/comfy_backend_rocm/src/loader.rs"))?;
    let backend_rocm_packager = fs::read_to_string(root.join("script/package-comfy-backend-rocm"))?;
    let backend_rocm_package_policy =
        fs::read_to_string(root.join("nix/comfy-backends/rocm/package-policy.json"))?;
    let backend_metal_abi = fs::read_to_string(root.join("crates/comfy_backend_metal/src/abi.rs"))?;
    let backend_metal_loader =
        fs::read_to_string(root.join("crates/comfy_backend_metal/src/loader.rs"))?;
    let backend_metal_adapter =
        fs::read_to_string(root.join("crates/comfy_backend_metal/src/comfy_backend_metal.rs"))?;
    let backend_metal_build = fs::read_to_string(root.join("crates/comfy_backend_metal/build.rs"))?;
    let backend_metal_packager =
        fs::read_to_string(root.join("script/package-comfy-backend-metal"))?;
    let backend_metal_package_policy =
        fs::read_to_string(root.join("nix/comfy-backends/metal/package-policy.json"))?;
    let backend_metal_contract_schema =
        fs::read_to_string(root.join("nix/comfy-backends/metal/ffi-contracts-v1.schema.json"))?;
    let backend_mlu_abi = fs::read_to_string(root.join("crates/comfy_backend_mlu/src/abi.rs"))?;
    let backend_mlu_loader =
        fs::read_to_string(root.join("crates/comfy_backend_mlu/src/loader.rs"))?;
    let backend_mlu_execution =
        fs::read_to_string(root.join("crates/comfy_backend_mlu/src/execution.rs"))?;
    let backend_mlu_execution_production = backend_mlu_execution
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(backend_mlu_execution.as_str(), |(production, _)| production);
    let backend_mlu_adapter =
        fs::read_to_string(root.join("crates/comfy_backend_mlu/src/comfy_backend_mlu.rs"))?;
    let backend_mlu_packager = fs::read_to_string(root.join("script/package-comfy-backend-mlu"))?;
    let backend_mlu_package_policy =
        fs::read_to_string(root.join("nix/comfy-backends/mlu/package-policy.json"))?;
    let backend_directml_abi =
        fs::read_to_string(root.join("crates/comfy_backend_directml/src/abi.rs"))?;
    let backend_directml_loader =
        fs::read_to_string(root.join("crates/comfy_backend_directml/src/loader.rs"))?;
    let backend_directml_loader_production = backend_directml_loader
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(backend_directml_loader.as_str(), |(production, _)| {
            production
        });
    let backend_directml_execution =
        fs::read_to_string(root.join("crates/comfy_backend_directml/src/execution.rs"))?;
    let backend_directml_execution_production = backend_directml_execution
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(backend_directml_execution.as_str(), |(production, _)| {
            production
        });
    let backend_directml_adapter = fs::read_to_string(
        root.join("crates/comfy_backend_directml/src/comfy_backend_directml.rs"),
    )?;
    let backend_directml_adapter_production = backend_directml_adapter
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(backend_directml_adapter.as_str(), |(production, _)| {
            production
        });
    let backend_directml_packager =
        fs::read_to_string(root.join("script/package-comfy-backend-directml"))?;
    let backend_directml_package_policy =
        fs::read_to_string(root.join("nix/comfy-backends/directml/package-policy.json"))?;
    let backend_directml_contract_schema =
        fs::read_to_string(root.join("nix/comfy-backends/directml/ffi-contracts-v1.schema.json"))?;
    let backend_npu_abi = fs::read_to_string(root.join("crates/comfy_backend_npu/src/abi.rs"))?;
    let backend_npu_loader =
        fs::read_to_string(root.join("crates/comfy_backend_npu/src/loader.rs"))?;
    let backend_npu_execution =
        fs::read_to_string(root.join("crates/comfy_backend_npu/src/execution.rs"))?;
    let backend_npu_execution_production = backend_npu_execution
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(backend_npu_execution.as_str(), |(production, _)| production);
    let backend_npu_adapter =
        fs::read_to_string(root.join("crates/comfy_backend_npu/src/comfy_backend_npu.rs"))?;
    let backend_npu_adapter_production = backend_npu_adapter
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(backend_npu_adapter.as_str(), |(production, _)| production);
    let tensor_npu_adapter = fs::read_to_string(
        root.join("crates/comfy_tensor/src/backends/huawei_ascend_npu_comfy_model_0019.rs"),
    )?;
    let backend_npu_packager = fs::read_to_string(root.join("script/package-comfy-backend-npu"))?;
    let backend_npu_package_policy =
        fs::read_to_string(root.join("nix/comfy-backends/npu/package-policy.json"))?;
    let backend_npu_contract_schema =
        fs::read_to_string(root.join("nix/comfy-backends/npu/ffi-contracts-v1.schema.json"))?;
    let backend_corex_abi = fs::read_to_string(root.join("crates/comfy_backend_corex/src/abi.rs"))?;
    let backend_corex_loader =
        fs::read_to_string(root.join("crates/comfy_backend_corex/src/loader.rs"))?;
    let backend_corex_adapter =
        fs::read_to_string(root.join("crates/comfy_backend_corex/src/comfy_backend_corex.rs"))?;
    let backend_corex_adapter_production = backend_corex_adapter
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(backend_corex_adapter.as_str(), |(production, _)| production);
    let backend_corex_packager =
        fs::read_to_string(root.join("script/package-comfy-backend-corex"))?;
    let backend_corex_package_policy =
        fs::read_to_string(root.join("nix/comfy-backends/corex/package-policy.json"))?;
    let backend_xpu_abi = fs::read_to_string(root.join("crates/comfy_backend_xpu/src/abi.rs"))?;
    let backend_xpu_loader =
        fs::read_to_string(root.join("crates/comfy_backend_xpu/src/loader.rs"))?;
    let backend_xpu_execution =
        fs::read_to_string(root.join("crates/comfy_backend_xpu/src/execution.rs"))?;
    let backend_xpu_reviewed_execution = fs::read_to_string(
        root.join("crates/comfy_backend_xpu/abi/reviewed-execution-bindings-v1.txt"),
    )?;
    let backend_xpu_execution_verifier =
        fs::read_to_string(root.join("crates/comfy_backend_xpu/abi/verify-execution-bindings.c"))?;
    let backend_xpu_adapter =
        fs::read_to_string(root.join("crates/comfy_backend_xpu/src/comfy_backend_xpu.rs"))?;
    let backend_xpu_adapter_production = backend_xpu_adapter
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(backend_xpu_adapter.as_str(), |(production, _)| production);
    let backend_xpu_packager = fs::read_to_string(root.join("script/package-comfy-backend-xpu"))?;
    let backend_xpu_package_policy =
        fs::read_to_string(root.join("nix/comfy-backends/xpu/package-policy.json"))?;
    let backend_xpu_contract_schema =
        fs::read_to_string(root.join("nix/comfy-backends/xpu/ffi-contracts-v1.schema.json"))?;
    let tensor_xpu_adapter = fs::read_to_string(
        root.join("crates/comfy_tensor/src/backends/intel_xpu_comfy_model_0021.rs"),
    )?;
    let backend_cuda_abi = fs::read_to_string(root.join("crates/comfy_backend_cuda/src/abi.rs"))?;
    let backend_cuda_loader =
        fs::read_to_string(root.join("crates/comfy_backend_cuda/src/loader.rs"))?;
    let backend_cuda_execution =
        fs::read_to_string(root.join("crates/comfy_backend_cuda/src/execution.rs"))?;
    let backend_cuda_adapter =
        fs::read_to_string(root.join("crates/comfy_backend_cuda/src/comfy_backend_cuda.rs"))?;
    let backend_cuda_adapter_production = backend_cuda_adapter
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(backend_cuda_adapter.as_str(), |(production, _)| production);
    let backend_cuda_packager = fs::read_to_string(root.join("script/package-comfy-backend-cuda"))?;
    let backend_cuda_package_policy =
        fs::read_to_string(root.join("nix/comfy-backends/cuda/package-policy.json"))?;
    let backend_cuda_contract_schema =
        fs::read_to_string(root.join("nix/comfy-backends/cuda/ffi-contracts-v1.schema.json"))?;
    let tensor_cuda_adapter = fs::read_to_string(
        root.join("crates/comfy_tensor/src/backends/nvidia_cuda_comfy_model_0022.rs"),
    )?;
    let backend_metal_execution =
        fs::read_to_string(root.join("crates/comfy_backend_metal/src/execution.rs"))?;
    let backend_metal_execution_abi =
        fs::read_to_string(root.join("crates/comfy_backend_metal/src/execution_abi.rs"))?;
    let backend_metal_execution_verifier = fs::read_to_string(
        root.join("crates/comfy_backend_metal/abi/verify-execution-bindings.m"),
    )?;
    let backend_metal_execution_packager =
        fs::read_to_string(root.join("script/package-comfy-backend-metal-execution"))?;
    let backend_metal_execution_package_policy =
        fs::read_to_string(root.join("nix/comfy-backends/metal/execution-policy.json"))?;
    let backend_metal_execution_catalog = fs::read_to_string(
        root.join(".agents/specs/comfy-parity/catalogs/native-backend-abi/metal-execution.json"),
    )?;
    let gpui_metal_renderer =
        fs::read_to_string(root.join("crates/gpui_macos/src/metal_renderer.rs"))?;
    let media_owner = fs::read_to_string(root.join("crates/media/src/media.rs"))?;
    let runtime_controller =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_execution_controller.rs"))?;
    let runtime_controller_production = runtime_controller
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_controller.as_str(), |(production, _)| production);
    let runtime_prompt_compiler =
        fs::read_to_string(root.join("crates/comfy_runtime/src/prompt_compiler.rs"))?;
    let runtime_prompt_compiler_production = runtime_prompt_compiler
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_prompt_compiler.as_str(), |(production, _)| {
            production
        });
    let runtime_cache = fs::read_to_string(root.join("crates/comfy_runtime/src/cache.rs"))?;
    let runtime_cache_production = runtime_cache
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_cache.as_str(), |(production, _)| production);
    let execution_presentation =
        fs::read_to_string(root.join("crates/comfy_runtime/src/execution_presentation.rs"))?;
    let recovery_source = fs::read_to_string(root.join("crates/comfy_runtime/src/recovery.rs"))?;
    let recovery_production = recovery_source
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(recovery_source.as_str(), |(production, _)| production);
    let output_committer_source =
        fs::read_to_string(root.join("crates/comfy_runtime/src/output_committer.rs"))?;
    let output_committer_production = output_committer_source
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(output_committer_source.as_str(), |(production, _)| {
            production
        });
    let runtime_persistence =
        fs::read_to_string(root.join("crates/comfy_runtime/src/persistence.rs"))?;
    let runtime_persistence_production = runtime_persistence
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_persistence.as_str(), |(production, _)| production);
    let subgraph_blueprints =
        fs::read_to_string(root.join("crates/comfy_runtime/src/subgraph_blueprints.rs"))?;
    let subgraph_blueprints_production = subgraph_blueprints
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(subgraph_blueprints.as_str(), |(production, _)| production);
    let runtime_graph = fs::read_to_string(root.join("crates/comfy_runtime/src/graph.rs"))?;
    let runtime_graph_production = runtime_graph
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_graph.as_str(), |(production, _)| production);
    let execution_ui = fs::read_to_string(root.join("crates/comfy_ui/src/execution_model.rs"))?;
    let execution_ui_production = execution_ui
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(execution_ui.as_str(), |(production, _)| production);
    let execution_panel = fs::read_to_string(root.join("crates/comfy_ui/src/execution_panel.rs"))?;
    let execution_panel_production = execution_panel
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(execution_panel.as_str(), |(production, _)| production);
    let api_services = fs::read_to_string(root.join("crates/comfy_api/src/services.rs"))?;
    let api_host = fs::read_to_string(root.join("crates/comfy_api/src/comfy_api.rs"))?;
    let api_host_production = api_host
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(api_host.as_str(), |(production, _)| production);
    let api_headless = fs::read_to_string(root.join("crates/comfy_api/src/headless.rs"))?;
    let api_http = fs::read_to_string(root.join("crates/comfy_api/src/http.rs"))?;
    let api_security = fs::read_to_string(root.join("crates/comfy_api/src/security.rs"))?;
    let api_security_production = api_security
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(api_security.as_str(), |(production, _)| production);
    let api_transport = fs::read_to_string(root.join("crates/comfy_api/src/transport.rs"))?;
    let api_websocket = fs::read_to_string(root.join("crates/comfy_api/src/websocket.rs"))?;
    let runtime_permissions =
        fs::read_to_string(root.join("crates/comfy_runtime/src/permissions.rs"))?;
    let runtime_supervisor =
        fs::read_to_string(root.join("crates/comfy_runtime/src/runtime_supervisor.rs"))?;
    let runtime_plugin_services =
        fs::read_to_string(root.join("crates/comfy_runtime/src/plugin_services.rs"))?;
    let runtime_plugin_services_production = runtime_plugin_services
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_plugin_services.as_str(), |(production, _)| {
            production
        });
    let runtime_executor = fs::read_to_string(root.join("crates/comfy_runtime/src/executor.rs"))?;
    let runtime_executor_production = runtime_executor
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(runtime_executor.as_str(), |(production, _)| production);
    let plugin_host =
        fs::read_to_string(root.join("crates/comfy_plugin_host/src/comfy_plugin_host.rs"))?;
    let plugin_component_host =
        fs::read_to_string(root.join("crates/comfy_plugin_host/src/component_host.rs"))?;
    let plugin_private_worker =
        fs::read_to_string(root.join("crates/comfy_plugin_host/src/private_worker.rs"))?;
    let plugin_registry_adapter =
        fs::read_to_string(root.join("crates/comfy_plugin_host/src/registry_adapter.rs"))?;
    let plugin_registry_adapter_production = plugin_registry_adapter
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(plugin_registry_adapter.as_str(), |(production, _)| {
            production
        });
    let worker_plugin_runtime =
        fs::read_to_string(root.join("crates/comfy_worker/src/plugin_runtime.rs"))?;
    let worker_plugin_runtime_production = worker_plugin_runtime
        .split_once("#[cfg(test)]")
        .map_or(worker_plugin_runtime.as_str(), |(production, _)| production);
    let worker_memory_modes =
        fs::read_to_string(root.join("crates/comfy_worker/src/memory_modes.rs"))?;
    let worker_memory_tests =
        fs::read_to_string(root.join("crates/comfy_worker/tests/memory_conformance.rs"))?;
    let worker_process = fs::read_to_string(root.join("crates/comfy_worker/src/comfy_worker.rs"))?;
    let worker_protocol =
        fs::read_to_string(root.join("crates/comfy_types/src/worker_protocol.rs"))?;
    let native_asset_services = fs::read_to_string(root.join("crates/comfy_ui/src/comfy_ui.rs"))?;
    let context_menu = fs::read_to_string(root.join("crates/comfy_ui/src/context_menu.rs"))?;
    let context_menu_tests =
        fs::read_to_string(root.join("crates/comfy_ui/src/context_menu_tests.rs"))?;
    let workflow_item = fs::read_to_string(root.join("crates/comfy_ui/src/workflow_item.rs"))?;
    let extension_capability_granter =
        fs::read_to_string(root.join("crates/extension_host/src/capability_granter.rs"))?;
    let extension_host =
        fs::read_to_string(root.join("crates/extension_host/src/extension_host.rs"))?;
    let extension_component_runtime =
        fs::read_to_string(root.join("crates/extension_host/src/wasm_host.rs"))?;
    let sim_bootstrap = fs::read_to_string(root.join("crates/sim/src/sim.rs"))?;
    let sim_plugin_services =
        fs::read_to_string(root.join("crates/sim/src/comfy_plugin_services.rs"))?;
    let sim_cli = fs::read_to_string(root.join("crates/sim/src/comfy_cli.rs"))?;
    let ownership_policy_source =
        fs::read_to_string(root.join(".agents/specs/comfy-parity/ownership-policy.json"))?;
    let ownership_policy: serde_json::Value = serde_json::from_str(&ownership_policy_source)?;
    let ownership_generator =
        fs::read_to_string(root.join(".agents/specs/comfy-parity/generate_ownership_catalog.py"))?;
    let ownership_catalog = fs::read_to_string(
        root.join(".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv"),
    )?;
    let task_source = fs::read_to_string(root.join(".agents/specs/comfy-parity/tasks.md"))?;
    let native_spec_mapping: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/specs/comfy-parity/catalogs/native-spec-mapping.json"),
    )?)?;
    let corex_future_task_source =
        fs::read_to_string(root.join(".agents/specs/comfy-corex-enablement/tasks.md"))?;
    let tensor_external_kernel_part_one =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/external_tensor_kernel_01.rs"))?;
    let tensor_external_kernel_part_two =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/external_tensor_kernel_02.rs"))?;
    let tensor_external_kernel_part_three =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/external_tensor_kernel_03.rs"))?;
    let tensor_image_ops = fs::read_to_string(root.join("crates/comfy_tensor/src/image_ops.rs"))?;
    let tensor_domain = fs::read_to_string(root.join("crates/comfy_tensor/src/comfy_tensor.rs"))?;
    let tensor_autograd = fs::read_to_string(root.join("crates/comfy_tensor/src/autograd.rs"))?;
    let tensor_autograd_breadth =
        fs::read_to_string(root.join("crates/comfy_tensor/src/autograd/breadth.rs"))?;
    let model_quantization =
        fs::read_to_string(root.join("crates/comfy_model/src/quantization.rs"))?;
    let model_quantized_autograd =
        fs::read_to_string(root.join("crates/comfy_model/src/quantized_autograd.rs"))?;
    let model_patch_graph = fs::read_to_string(root.join("crates/comfy_model/src/patch_graph.rs"))?;
    let model_patches = fs::read_to_string(root.join("crates/comfy_model/src/patches.rs"))?;
    let model_weight_adapter =
        fs::read_to_string(root.join("crates/comfy_model/src/weight_adapter.rs"))?;
    let model_clip = fs::read_to_string(root.join("crates/comfy_model/src/clip.rs"))?;
    let model_clip_text = fs::read_to_string(root.join("crates/comfy_model/src/clip_text.rs"))?;
    let model_clip_text_encoder_t5 =
        fs::read_to_string(root.join("crates/comfy_model/src/clip_text_encoder_t5.rs"))?;
    let model_clip_text_encoder_decoder =
        fs::read_to_string(root.join("crates/comfy_model/src/clip_text_encoder_decoder.rs"))?;
    let model_clip_text_encoder_multimodal =
        fs::read_to_string(root.join("crates/comfy_model/src/clip_text_encoder_multimodal.rs"))?;
    let model_native_node_payload =
        fs::read_to_string(root.join("crates/comfy_model/src/native_node_payload.rs"))?;
    let nodes_stored_payload =
        fs::read_to_string(root.join("crates/comfy_nodes/src/stored_payload.rs"))?;
    let model_clip_vision = fs::read_to_string(root.join("crates/comfy_model/src/clip_vision.rs"))?;
    let model_clip_vision_production = model_clip_vision
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(model_clip_vision.as_str(), |(production, _)| production);
    let model_clip_tokenizer =
        fs::read_to_string(root.join("crates/comfy_model/src/clip_tokenizer.rs"))?;
    let model_store = fs::read_to_string(root.join("crates/comfy_model/src/model_store.rs"))?;
    let model_native_diffusion =
        fs::read_to_string(root.join("crates/comfy_model/src/slices/native_diffusion.rs"))?;
    let native_diffusion_fixture =
        fs::read_to_string(root.join("crates/comfy_test_support/src/native_diffusion_fixture.rs"))?;
    let model_clip_tokenizer_tests =
        fs::read_to_string(root.join("crates/comfy_model/tests/clip_tokenizer.rs"))?;
    let model_clip_text_tests =
        fs::read_to_string(root.join("crates/comfy_model/tests/clip_text.rs"))?;
    let model_clip_text_encoder_t5_tests =
        fs::read_to_string(root.join("crates/comfy_model/tests/clip_text_encoder_t5.rs"))?;
    let model_clip_text_encoder_decoder_tests =
        fs::read_to_string(root.join("crates/comfy_model/tests/clip_text_encoder_decoder.rs"))?;
    let model_clip_text_encoder_multimodal_tests =
        fs::read_to_string(root.join("crates/comfy_model/tests/clip_text_encoder_multimodal.rs"))?;
    let model_clip_vision_tests =
        fs::read_to_string(root.join("crates/comfy_model/tests/clip_vision.rs"))?;
    let model_patch_adapter_tests =
        fs::read_to_string(root.join("crates/comfy_model/tests/patch_adapters.rs"))?;
    let model_quantized_autograd_tests =
        fs::read_to_string(root.join("crates/comfy_model/tests/quantized_autograd.rs"))?;
    let tensor_autograd_state_tests =
        fs::read_to_string(root.join("crates/comfy_tensor/tests/autograd_state_consolidation.rs"))?;
    let tensor_dtypes = fs::read_to_string(root.join("crates/comfy_tensor/src/dtypes.rs"))?;
    let tensor_cpu_backend =
        fs::read_to_string(root.join("crates/comfy_tensor/src/cpu_backend.rs"))?;
    let tensor_operation = fs::read_to_string(root.join("crates/comfy_tensor/src/operation.rs"))?;
    let tensor_rocm_backend = fs::read_to_string(
        root.join("crates/comfy_tensor/src/backends/amd_rocm_comfy_model_0014.rs"),
    )?;
    let tensor_metal_backend = fs::read_to_string(
        root.join("crates/comfy_tensor/src/backends/apple_metal_mps_comfy_model_0015.rs"),
    )?;
    let tensor_rocm_backend_production = tensor_rocm_backend
        .split_once("#[cfg(test)]\nstruct TestRuntime")
        .map_or(tensor_rocm_backend.as_str(), |(production, _)| production);
    let tensor_cpu_backend_production = tensor_cpu_backend
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(tensor_cpu_backend.as_str(), |(production, _)| production);
    let tensor_operation_part_three = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_03.rs"),
    )?;
    let tensor_operation_part_four = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_04.rs"),
    )?;
    let tensor_operation_part_five = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_05.rs"),
    )?;
    let tensor_operation_part_six = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_06.rs"),
    )?;
    let tensor_operation_part_seven = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_07.rs"),
    )?;
    let tensor_operation_part_eight = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_08.rs"),
    )?;
    let tensor_operation_part_nine = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_09.rs"),
    )?;
    let tensor_operation_part_fifteen = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_15.rs"),
    )?;
    let tensor_operation_part_ten = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_10.rs"),
    )?;
    let tensor_operation_part_eleven = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_11.rs"),
    )?;
    let tensor_operation_part_twelve = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_12.rs"),
    )?;
    let tensor_operation_part_thirteen = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_13.rs"),
    )?;
    let tensor_operation_part_fourteen = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_14.rs"),
    )?;
    let model_attention = fs::read_to_string(root.join("crates/comfy_model/src/attention.rs"))?;
    let tensor_operation_part_sixteen = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_16.rs"),
    )?;
    let tensor_operation_part_seventeen = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_17.rs"),
    )?;
    let tensor_operation_part_seventeen_resolution = fs::read_to_string(root.join(
        "crates/comfy_tensor/src/operation_resolutions/elementwise_or_runtime_operation_17.rs",
    ))?;
    let tensor_operation_part_eighteen = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_18.rs"),
    )?;
    let tensor_operation_part_eighteen_resolution = fs::read_to_string(root.join(
        "crates/comfy_tensor/src/operation_resolutions/elementwise_or_runtime_operation_18.rs",
    ))?;
    let tensor_operation_part_eighteen_tests = fs::read_to_string(
        root.join("crates/comfy_tensor/tests/ops/elementwise_or_runtime_operation_18.rs"),
    )?;
    let tensor_operation_part_nineteen = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_19.rs"),
    )?;
    let tensor_operation_part_nineteen_resolution = fs::read_to_string(root.join(
        "crates/comfy_tensor/src/operation_resolutions/elementwise_or_runtime_operation_19.rs",
    ))?;
    let tensor_operation_part_nineteen_tests = fs::read_to_string(
        root.join("crates/comfy_tensor/tests/ops/elementwise_or_runtime_operation_19.rs"),
    )?;
    let tensor_operation_part_twenty = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_20.rs"),
    )?;
    let tensor_operation_part_twenty_resolution = fs::read_to_string(root.join(
        "crates/comfy_tensor/src/operation_resolutions/elementwise_or_runtime_operation_20.rs",
    ))?;
    let tensor_operation_part_twenty_tests = fs::read_to_string(
        root.join("crates/comfy_tensor/tests/ops/elementwise_or_runtime_operation_20.rs"),
    )?;
    let tensor_activation_normalization = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/activation_normalization_functional_01.rs"),
    )?;
    let tensor_indexing_masking_part_one =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/indexing_masking_01.rs"))?;
    let tensor_indexing_masking_part_two =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/indexing_masking_02.rs"))?;
    let tensor_linear_algebra_part_one =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/linear_algebra_01.rs"))?;
    let tensor_linear_algebra_part_two =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/linear_algebra_02.rs"))?;
    let tensor_neural_network_functional_part_one = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/neural_network_functional_01.rs"),
    )?;
    let tensor_neural_network_functional_part_one_tests = fs::read_to_string(
        root.join("crates/comfy_tensor/tests/ops/neural_network_functional_01.rs"),
    )?;
    let tensor_neural_network_module_part_one =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/neural_network_module_01.rs"))?;
    let tensor_neural_network_module_part_one_tests =
        fs::read_to_string(root.join("crates/comfy_tensor/tests/ops/neural_network_module_01.rs"))?;
    let tensor_neural_network_module_part_two =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/neural_network_module_02.rs"))?;
    let tensor_neural_network_module_part_three =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/neural_network_module_03.rs"))?;
    let tensor_neural_network_module_part_three_tests =
        fs::read_to_string(root.join("crates/comfy_tensor/tests/ops/neural_network_module_03.rs"))?;
    let tensor_neural_network_module_part_four =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/neural_network_module_04.rs"))?;
    let tensor_neural_network_module_part_four_tests =
        fs::read_to_string(root.join("crates/comfy_tensor/tests/ops/neural_network_module_04.rs"))?;
    let tensor_spatial_functional_kernel = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs"),
    )?;
    let tensor_spatial_functional_kernel_tests = fs::read_to_string(
        root.join("crates/comfy_tensor/tests/ops/spatial_functional_kernel_01.rs"),
    )?;
    let tensor_spectral_transform =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/spectral_transform_01.rs"))?;
    let tensor_spectral_transform_tests =
        fs::read_to_string(root.join("crates/comfy_tensor/tests/ops/spectral_transform_01.rs"))?;
    let tensor_storage_dtype_device =
        fs::read_to_string(root.join("crates/comfy_tensor/src/ops/storage_dtype_device_01.rs"))?;
    let tensor_storage_dtype_device_tests =
        fs::read_to_string(root.join("crates/comfy_tensor/tests/ops/storage_dtype_device_01.rs"))?;
    let tensor_rng = fs::read_to_string(root.join("crates/comfy_tensor/src/rng.rs"))?;
    let tensor_random_number_generation_part_one = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/random_number_generation_01.rs"),
    )?;
    let tensor_random_number_generation_part_one_resolution = fs::read_to_string(
        root.join("crates/comfy_tensor/src/operation_resolutions/random_number_generation_01.rs"),
    )?;
    let tensor_random_number_generation_part_one_tests = fs::read_to_string(
        root.join("crates/comfy_tensor/tests/ops/random_number_generation_01.rs"),
    )?;
    let tensor_random_number_generation_part_two = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/random_number_generation_02.rs"),
    )?;
    let tensor_random_number_generation_part_two_resolution = fs::read_to_string(
        root.join("crates/comfy_tensor/src/operation_resolutions/random_number_generation_02.rs"),
    )?;
    let tensor_random_number_generation_part_two_tests = fs::read_to_string(
        root.join("crates/comfy_tensor/tests/ops/random_number_generation_02.rs"),
    )?;
    let sampler_noise = fs::read_to_string(root.join("crates/comfy_sampler/src/noise.rs"))?;
    let tensor_operator_indirection = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/comfy_operator_indirection_01.rs"),
    )?;
    let tensor_operation_part_twenty_one = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_21.rs"),
    )?;
    let tensor_operation_part_twenty_one_resolution = fs::read_to_string(root.join(
        "crates/comfy_tensor/src/operation_resolutions/elementwise_or_runtime_operation_21.rs",
    ))?;
    let tensor_operation_part_twenty_two = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_22.rs"),
    )?;
    let tensor_operation_part_twenty_two_resolution = fs::read_to_string(root.join(
        "crates/comfy_tensor/src/operation_resolutions/elementwise_or_runtime_operation_22.rs",
    ))?;
    let tensor_operation_part_twenty_two_tests = fs::read_to_string(
        root.join("crates/comfy_tensor/tests/ops/elementwise_or_runtime_operation_22.rs"),
    )?;
    let tensor_operation_part_twenty_three = fs::read_to_string(
        root.join("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_23.rs"),
    )?;
    let tensor_operation_part_twenty_three_resolution = fs::read_to_string(root.join(
        "crates/comfy_tensor/src/operation_resolutions/elementwise_or_runtime_operation_23.rs",
    ))?;
    let tensor_operation_part_twenty_three_tests = fs::read_to_string(
        root.join("crates/comfy_tensor/tests/ops/elementwise_or_runtime_operation_23.rs"),
    )?;
    let tensor_operation_part_twenty_one_tests = fs::read_to_string(
        root.join("crates/comfy_tensor/tests/ops/elementwise_or_runtime_operation_21.rs"),
    )?;
    let model_native_ops = fs::read_to_string(root.join("crates/comfy_model/src/native_ops.rs"))?;
    let model_alias_free =
        fs::read_to_string(root.join("crates/comfy_model/src/alias_free_activation.rs"))?;
    let model_operation_part_twenty_tests =
        fs::read_to_string(root.join("crates/comfy_model/tests/elementwise_runtime_part20.rs"))?;
    let model_vision = fs::read_to_string(root.join("crates/comfy_model/src/vision_models.rs"))?;
    let task_20_concerns = [
        "cancellation",
        "backend_capability",
        "permission_capability_domain",
        "plugin_signature_verification",
        "provider_request_authorization",
        "external_navigation_authorization",
        "asset_domain_adapter",
        "execution_queue",
    ];
    let native_api_concerns = [
        "api_security",
        "api_idempotency",
        "native_api_transport",
        "api_http_routing",
        "api_websocket_sessions",
        "api_request_target",
    ];
    let policy_concerns = ownership_policy
        .get("concerns")
        .and_then(serde_json::Value::as_array)
        .ok_or("ownership policy has no concern array")?;
    let task_101_policy_mappings: [(&str, &str, &[&str]); 5] = [
        (
            "autograd_checkpoint_execution",
            "comfy_tensor::generated_elementwise_or_runtime_operation_06::CheckpointExecution",
            &[
                "task101-operation-checkpoint-delegates-canonical-record",
                "task101-ownership-oracle-proves-state-owner-reuse",
            ],
        ),
        (
            "autograd_gradient_publication",
            "comfy_tensor::GradientStore",
            &[
                "task101-gradient-publication-and-zeroing-use-gradient-store",
                "task101-ownership-oracle-proves-state-owner-reuse",
            ],
        ),
        (
            "autograd_tape_and_reverse_traversal",
            "comfy_tensor::AutogradTape",
            &[
                "task101-leaf-binding-uses-logical-tensor-identity",
                "task101-retained-backward-uses-canonical-tape",
                "task101-ownership-oracle-proves-state-owner-reuse",
            ],
        ),
        (
            "tensor_logical_identity",
            "comfy_tensor::Tensor",
            &[
                "task101-tensor-owns-logical-identity",
                "task101-saved-tensor-validates-logical-identity-and-lineage",
                "task101-ownership-oracle-proves-state-owner-reuse",
            ],
        ),
        (
            "tensor_mutation_lineage",
            "comfy_tensor::Tensor",
            &[
                "task101-tensor-write-bumps-shared-mutation-lineage",
                "task101-saved-tensor-validates-logical-identity-and-lineage",
                "task101-ownership-oracle-proves-state-owner-reuse",
            ],
        ),
    ];
    let task_101_policy_trace = task_101_policy_mappings.iter().all(
        |(concern_name, canonical_owner, required_mapping_names)| {
            policy_concerns
                .iter()
                .find(|entry| {
                    entry.get("concern").and_then(serde_json::Value::as_str) == Some(*concern_name)
                })
                .is_some_and(|entry| {
                    entry
                        .get("canonical_owner")
                        .and_then(serde_json::Value::as_str)
                        == Some(*canonical_owner)
                        && entry
                            .get("known_open_reasons")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(Vec::is_empty)
                        && entry
                            .get("consolidation_tasks")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|tasks| {
                                tasks.iter().any(|task| {
                                    task.as_str()
                                        == Some(
                                            "comfy-parity-autograd-state-ownership-consolidation",
                                        )
                                })
                            })
                        && entry
                            .get("validation")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|validations| {
                                ["VAL-AUTOGRAD-001", "VAL-TENSOR-001", "VAL-OWNERSHIP-001"]
                                    .iter()
                                    .all(|required| {
                                        validations.iter().any(|validation| {
                                            validation.as_str() == Some(*required)
                                        })
                                    })
                            })
                        && entry
                            .get("required_mappings")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|mappings| {
                                required_mapping_names.iter().all(|required| {
                                    mappings.iter().any(|mapping| {
                                        mapping.get("name").and_then(serde_json::Value::as_str)
                                            == Some(*required)
                                    })
                                })
                            })
                })
        },
    );
    let task_101_catalog_trace =
        task_101_policy_mappings
            .iter()
            .all(|(concern_name, canonical_owner, _)| {
                ownership_catalog
                    .lines()
                    .find(|line| line.starts_with(&format!("{concern_name},")))
                    .is_some_and(|line| {
                        line.contains(canonical_owner)
                            && line.contains("comfy-parity-autograd-state-ownership-consolidation")
                            && line.contains("VAL-AUTOGRAD-001")
                            && line.contains("VAL-TENSOR-001")
                            && line.contains("VAL-OWNERSHIP-001")
                            && line.contains("authoritative_owner_confirmed")
                    })
            });
    let tensor_id_definitions = source_occurrences(&sources, "struct TensorId");
    let mutation_lineage_definitions = source_occurrences(&sources, "struct MutationLineage");
    let autograd_tape_definitions = source_occurrences(&sources, "pub struct AutogradTape");
    let gradient_store_definitions = source_occurrences(&sources, "pub struct GradientStore");
    let checkpoint_record_definitions = source_occurrences(&sources, "struct CheckpointRecord");
    let checkpoint_execution_definitions =
        source_occurrences(&sources, "struct CheckpointExecution");
    let function_context_definitions = source_occurrences(&sources, "struct FunctionContext");
    let higher_order_context_definitions =
        source_occurrences(&sources, "struct HigherOrderContext");
    let gradient_mode_stack_definitions = source_occurrences(&sources, "struct GradientModeStack");
    let task_101_tensor_identity_and_mutation_lineage_have_one_owner = tensor_id_definitions.len()
        == 1
        && tensor_id_definitions[0].contains("crates/comfy_tensor/src/comfy_tensor.rs")
        && mutation_lineage_definitions.len() == 1
        && mutation_lineage_definitions[0].contains("crates/comfy_tensor/src/comfy_tensor.rs")
        && tensor_domain.contains("pub struct TensorId")
        && tensor_domain.contains("struct MutationLineage")
        && tensor_domain.contains("pub struct Tensor")
        && tensor_domain.contains("TensorId")
        && tensor_domain.contains("MutationLineage")
        && tensor_domain.contains("pub struct TensorWrite")
        && !tensor_domain.contains("id: StorageId,\n    version: AtomicU64")
        && tensor_autograd.contains("pub struct SavedTensor")
        && tensor_autograd.contains("MutationWitness")
        && tensor_autograd.contains("tensor.mutation_witness()")
        && !tensor_autograd.contains("tensor.storage_id()")
        && !tensor_autograd.contains("tensor.storage_version()")
        && tensor_autograd_state_tests
            .contains("cow_mutation_preserves_logical_identity_and_invalidates_saved_witnesses")
        && tensor_autograd_state_tests.contains("detach_data_and_views_share_mutation_lineage")
        && tensor_autograd_state_tests.contains("factory_requires_grad_binds_logical_identity");
    let task_101_tape_gradient_store_and_checkpoint_have_one_owner =
        autograd_tape_definitions.len() == 1
            && autograd_tape_definitions[0].contains("crates/comfy_tensor/src/autograd.rs")
            && gradient_store_definitions.len() == 1
            && gradient_store_definitions[0].contains("crates/comfy_tensor/src/autograd.rs")
            && checkpoint_record_definitions.len() == 1
            && checkpoint_record_definitions[0].contains("crates/comfy_tensor/src/autograd.rs")
            && checkpoint_execution_definitions.len() == 1
            && checkpoint_execution_definitions[0]
                .contains("crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_06.rs")
            && tensor_autograd.contains("leaf_bindings: HashMap<TensorId, LeafId>")
            && !tensor_autograd.contains("HashMap<StorageId, LeafId>")
            && tensor_autograd.contains("pub struct GradientStore")
            && tensor_autograd.contains("struct CheckpointRecord")
            && !tensor_autograd_breadth.contains("pub struct GradientStore")
            && tensor_operation_part_six.contains("CheckpointRecord")
            && tensor_operation_part_six.contains("pub struct CheckpointExecution")
            && tensor_operation_part_six.contains("outputs: Vec<Tensor>")
            && !tensor_operation_part_six.contains("output_descriptors:")
            && !tensor_operation_part_six.contains("saved_inputs:")
            && !tensor_operation_part_six.contains("saved_versions:")
            && !tensor_operation_part_six.contains("storage_version()")
            && tensor_autograd_breadth.contains("execution: CheckpointExecution")
            && tensor_autograd_breadth.contains("checkpoint_execution_from_outputs_exact_native")
            && !tensor_autograd_breadth.contains("CheckpointRecord")
            && !tensor_autograd.contains("pub fn output_descriptors")
            && ownership_generator.contains("TASK_101_CONCERNS")
            && ownership_generator.contains("TASK_101_VALIDATIONS");
    let task_101_adapters_preserve_canonical_autograd_semantics = tensor_operation_part_six
        .contains("CheckpointRecord")
        && tensor_autograd_breadth.contains("CheckpointExecution")
        && tensor_operation_part_eight.contains("tape.backward")
        && tensor_operation_part_seventeen.contains("tape.set_requires_grad")
        && tensor_operation_part_twenty_one.contains("tape.reverse_and_publish")
        && tensor_autograd_state_tests
            .contains("gradient_publication_and_zeroing_use_canonical_store")
        && tensor_autograd_state_tests
            .contains("checkpoint_adapters_share_tape_record_and_reverse_path")
        && tensor_autograd_state_tests
            .contains("retained_backward_cancellation_and_terminal_release_are_atomic");
    let task_103_higher_order_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("autograd_tape_and_reverse_traversal")
        })
        .is_some_and(|entry| {
            entry
                .get("definitions")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|definitions| {
                    definitions.iter().any(|definition| {
                        definition.get("symbol").and_then(serde_json::Value::as_str)
                            == Some("HigherOrderContext")
                            && definition.get("role").and_then(serde_json::Value::as_str)
                                == Some("canonical_interface")
                    })
                })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "task103-higher-order-context-delegates-recording-to-the-canonical-tape",
                            "task103-analytical-custom-functions-compose-recorded-canonical-operations",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task_103_higher_order_catalog_trace = ownership_catalog
        .lines()
        .find(|line| line.starts_with("autograd_tape_and_reverse_traversal,"))
        .is_some_and(|line| {
            line.contains("comfy_tensor::AutogradTape")
                && line.contains("comfy-parity-native-autograd-breadth")
                && line.contains("VAL-AUTOGRAD-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task_103_custom_functions_modes_and_kernels_have_one_owner = function_context_definitions
        .len()
        == 1
        && function_context_definitions[0].contains("crates/comfy_tensor/src/autograd/breadth.rs")
        && higher_order_context_definitions.len() == 1
        && higher_order_context_definitions[0].contains("crates/comfy_tensor/src/autograd.rs")
        && task_103_higher_order_policy_trace
        && task_103_higher_order_catalog_trace
        && gradient_mode_stack_definitions.is_empty()
        && tensor_autograd.contains("pub fn with_mode<T>(")
        && tensor_autograd.contains("self.mode = previous")
        && tensor_autograd.contains("pub struct HigherOrderContext<'a, 'execution>")
        && tensor_autograd.contains("tape: &'a mut AutogradTape")
        && tensor_autograd.contains("self.tape\n            .record_operation(")
        && tensor_autograd_breadth.contains("context: FunctionContext")
        && model_quantized_autograd.contains("FunctionContext")
        && tensor_autograd_breadth.contains("execution: CheckpointExecution")
        && tensor_autograd_breadth.contains("AutogradConstructOwner::QuantizedModelAdapter")
        && !tensor_autograd_breadth.contains("struct Matrix")
        && !tensor_autograd_breadth.contains("matmul_values")
        && !tensor_autograd_breadth.contains("tucker_rebuild")
        && !tensor_autograd_breadth.contains("GradientModeStack")
        && tensor_autograd_breadth.contains("mm_with_context_exact_native")
        && tensor_autograd_breadth.contains("einsum_with_context_exact_native")
        && tensor_autograd_breadth.contains("index_select_with_context_exact_native")
        && tensor_autograd_breadth.contains("checkpoint_execution_from_outputs_exact_native")
        && tensor_autograd_breadth.contains("recorded_index_select_vjp")
        && tensor_autograd_breadth.contains("recorded_mm_vjp")
        && tensor_autograd_breadth.contains("recorded_einsum_vjp")
        && !tensor_operation_part_six
            .contains("#[derive(Clone, Debug)]\npub struct CheckpointExecution")
        && !tensor_autograd.contains("#[derive(Clone, Debug)]\npub struct CheckpointRecord")
        && ownership_generator.contains("TASK_103_CONCERNS")
        && ownership_generator.contains("TASK_103_VALIDATIONS");
    let task_103_adapters_preserve_canonical_autograd_semantics = tensor_autograd_state_tests
        .contains("requires_grad_rejects_integral_tensor_without_mutating_leaf_state")
        && tensor_autograd_state_tests
            .contains("retained_backward_cancellation_and_terminal_release_are_atomic")
        && tensor_autograd_breadth.contains("canonical_error(")
        && tensor_autograd_breadth.contains("execution.cancellation.is_cancelled()")
        && model_quantized_autograd_tests.contains("input_arity(), 6");
    let task_27_prompt_compiler_and_cache_have_one_owner = prompt_compiler_definitions.len() == 1
        && prompt_compiler_definitions[0].contains("crates/comfy_runtime/src/prompt_compiler.rs")
        && native_cache_definitions.len() == 1
        && native_cache_definitions[0].contains("crates/comfy_runtime/src/cache.rs")
        && cache_key_definitions.len() == 1
        && cache_key_definitions[0].contains("crates/comfy_runtime/src/cache.rs")
        && runtime_prompt_compiler_production.contains("pub struct PromptCompiler<'a> {")
        && runtime_prompt_compiler_production
            .contains("pub fn compile(\n        &self,\n        submission: PromptSubmission,")
        && runtime_cache_production.contains("pub struct CacheKey {")
        && runtime_cache_production.contains("pub struct NativeCache {")
        && runtime_cache_production.contains("entries: BTreeMap<CacheKey, CacheRecord>")
        && runtime_cache_production.contains("least_recently_used: VecDeque<CacheKey>");
    let task_27_controller_and_engine_delegate_compilation_and_cache = runtime_controller_production
        .matches("crate::PromptCompiler::new(&registry).compile(submission)?")
        .count() == 3
        && runtime_controller_production.contains("cache: Arc<Mutex<NativeCache>>")
        && runtime_controller_production.contains("self.cache.clone(),")
        && runtime_executor_production.contains("cache: Arc<Mutex<NativeCache>>")
        && runtime_executor_production
            .contains("let cache_key = CacheKey::from_inputs_with_dependencies(")
        && runtime_executor_production
            .contains("self.cache.lock().get_with_handle_lease(&cache_key)")
        && runtime_executor_production.contains("fn publish_cache_batch(")
        && runtime_executor_production.contains("cache.insert_batch_with_handle_leases(entries)")
        && runtime_executor_production
            .contains("self.publish_cache_batch(leased_cache_entries, &state.cancellation)")
        && runtime_executor_production.contains("*cache = prior_cache")
        && !runtime_controller_production.contains("pub struct PromptCompiler")
        && !runtime_controller_production.contains("pub struct CacheKey")
        && !runtime_controller_production.contains("pub struct NativeCache")
        && !runtime_executor_production.contains("pub struct PromptCompiler")
        && !runtime_executor_production.contains("pub struct CacheKey")
        && !runtime_executor_production.contains("pub struct NativeCache");
    let runtime_restart_surfaces = [
        runtime_persistence_production,
        recovery_production,
        runtime_executor_production,
        runtime_controller_production,
    ];
    let task_27_autograd_checkpoints_are_ephemeral_and_not_restartable =
        !declaration_derives_trait(&tensor_autograd, "pub struct CheckpointRecord", "Serialize")
            && !declaration_derives_trait(
                &tensor_autograd,
                "pub struct CheckpointRecord",
                "Deserialize",
            )
            && !declaration_derives_trait(
                &tensor_operation_part_six,
                "pub struct CheckpointExecution",
                "Serialize",
            )
            && !declaration_derives_trait(
                &tensor_operation_part_six,
                "pub struct CheckpointExecution",
                "Deserialize",
            )
            && tensor_autograd.contains("pub struct CheckpointRecord")
            && tensor_autograd.contains("saved: Vec<SavedTensor>")
            && tensor_operation_part_six.contains("pub struct CheckpointExecution")
            && tensor_operation_part_six.contains("impl Drop for CheckpointExecution")
            && runtime_restart_surfaces.iter().all(|source| {
                [
                    "CheckpointRecord",
                    "CheckpointExecution",
                    "AutogradTape",
                    "SavedTensor",
                    "autograd_checkpoint",
                ]
                .into_iter()
                .all(|identifier| !source.contains(identifier))
            })
            && runtime_persistence_production.contains("pub struct PersistedExecutionAttempt {")
            && runtime_persistence_production.contains("pub record: AttemptRecord,")
            && runtime_persistence_production.contains("pub plan: Option<CompiledPlan>,")
            && runtime_persistence_production.contains("pub source: ExecutionDataSource,")
            && runtime_persistence_production
                .contains("pub queue: Option<PersistedQueueMetadata>,")
            && runtime_persistence_production
                .contains("unknown_fields: BTreeMap<String, serde_json::Value>")
            && runtime_persistence_production.contains("unknown_fields: BTreeMap::new()")
            && !runtime_persistence_production.contains("autograd")
            && !recovery_production.contains("autograd");
    let task_27_recovery_journal_only_records_checked_immutable_output_receipts =
        recovery_journal_definitions.len() == 1
            && recovery_journal_definitions[0].contains("crates/comfy_runtime/src/recovery.rs")
            && recovery_output_receipt_definitions.len() == 1
            && recovery_output_receipt_definitions[0]
                .contains("crates/comfy_runtime/src/recovery.rs")
            && output_commit_receipt_definitions.len() == 1
            && output_commit_receipt_definitions[0]
                .contains("crates/comfy_runtime/src/output_committer.rs")
            && recovery_production.contains("fn from_commit_receipt(")
            && recovery_production.contains("receipt: &OutputCommitReceipt,")
            && recovery_production.contains("pub fn record_output_receipt(")
            && recovery_production.contains(".ok_or(RecoveryError::UnscopedOutputReceipt(")
            && recovery_production.contains("scope.profile_id != profile_id")
            && recovery_production.contains("|| scope.prompt_id != prompt_id")
            && recovery_production.contains("|| scope.attempt_id != attempt_id")
            && recovery_production.contains("RecoveryOutputReceipt::from_commit_receipt(")
            && recovery_production.contains("receipts: Vec<RecoveryOutputReceipt>")
            && !recovery_production.contains("pub receipts:")
            && !recovery_production.contains("pub fn receipts_mut(")
            && !recovery_production.contains("&mut RecoveryOutputReceipt")
            && !recovery_production.contains("fn prepare(")
            && !recovery_production.contains("fn commit(")
            && !recovery_production.contains("OutputOperation")
            && !recovery_production.contains("OutputCommitter")
            && output_committer_production.contains("pub struct OutputCommitReceipt {")
            && !output_committer_production.contains("pub proposal_id: Uuid")
            && !output_committer_production.contains("pub operation: OutputOperation")
            && !declaration_derives_trait(
                output_committer_production,
                "pub struct OutputCommitReceipt",
                "Serialize",
            )
            && !declaration_derives_trait(
                output_committer_production,
                "pub struct OutputCommitReceipt",
                "Deserialize",
            );
    let task_102_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("model_quantization_contracts")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::quantization")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str() == Some("comfy-parity-quantized-autograd-adapter")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "task102-quantlinear-adapter-delegates-layout-and-scale-equations",
                            "task102-native-module-maps-canonical-quantized-storage",
                            "task102-quantized-content-identity-binds-source-and-encoding",
                            "task102-materialization-retains-caller-workspace",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task_102_catalog_trace = ownership_catalog
        .lines()
        .find(|line| line.starts_with("model_quantization_contracts,"))
        .is_some_and(|line| {
            line.contains("comfy_model::quantization")
                && line.contains("comfy-parity-quantized-autograd-adapter")
                && line.contains("VAL-AUTOGRAD-001")
                && line.contains("VAL-TENSOR-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let quant_linear_layout_definitions = source_occurrences(&sources, "enum QuantLinearLayout");
    let quant_linear_scale_definitions = source_occurrences(&sources, "enum QuantLinearScale");
    let quantized_linear_matrix_definitions =
        source_occurrences(&sources, "struct QuantizedLinearMatrix");
    let quantize_linear_function_definitions =
        source_occurrences(&sources, "pub fn quantize_linear_matrix");
    let quantized_source_identity_definitions =
        source_occurrences(&sources, "struct QuantizedSourceIdentity");
    let quantized_content_identity_definitions =
        source_occurrences(&sources, "struct QuantizedContentIdentity");
    let quantized_materialization_definitions =
        source_occurrences(&sources, "struct QuantizedMaterialization");
    let task_102_quantization_has_one_owner = quant_linear_layout_definitions.len() == 1
        && quant_linear_layout_definitions[0].contains("crates/comfy_model/src/quantization.rs")
        && quant_linear_scale_definitions.len() == 1
        && quant_linear_scale_definitions[0].contains("crates/comfy_model/src/quantization.rs")
        && quantized_linear_matrix_definitions.len() == 1
        && quantized_linear_matrix_definitions[0]
            .contains("crates/comfy_model/src/quantization.rs")
        && quantize_linear_function_definitions.len() == 1
        && quantize_linear_function_definitions[0]
            .contains("crates/comfy_model/src/quantization.rs")
        && quantized_source_identity_definitions.len() == 1
        && quantized_source_identity_definitions[0]
            .contains("crates/comfy_model/src/quantization.rs")
        && quantized_content_identity_definitions.len() == 1
        && quantized_content_identity_definitions[0]
            .contains("crates/comfy_model/src/quantization.rs")
        && quantized_materialization_definitions.len() == 1
        && quantized_materialization_definitions[0]
            .contains("crates/comfy_model/src/quantization.rs")
        && model_quantization.contains("fn resolve_fp8_scale(")
        && model_quantization.contains("fn quantize_fp8_tensorwise(")
        && model_quantization.contains("fn quantize_mxfp8(")
        && model_quantization.contains("fn quantize_nvfp4(")
        && !model_quantized_autograd.contains("encode_float8")
        && !model_quantized_autograd.contains("decode_float8")
        && !model_quantized_autograd.contains("E4M3_MAX")
        && !model_quantized_autograd.contains("MXFP8_GROUP_SIZE")
        && !tensor_autograd_breadth.contains("QuantLinearExecution")
        && !tensor_autograd_breadth.contains("QuantLinearLayout")
        && model_quantized_autograd.contains("quantize_linear_matrix")
        && model_quantized_autograd.contains("linear_with_context_exact_native")
        && model_quantized_autograd.contains("linear_vjp_with_context_exact_native")
        && model_quantized_autograd.contains("FunctionContext")
        && model_quantized_autograd.contains("backend: &dyn TensorBackend")
        && model_quantized_autograd.contains("matrix.materialize(backend, context)")
        && !model_quantized_autograd.contains("identity: Tensor")
        && !model_quantized_autograd.contains(".dequantize(")
        && model_native_ops.contains("matrix.content_identity().as_bytes()")
        && model_native_ops.contains("weight.materialize(backend, context)")
        && !model_native_ops.contains("weight.dequantize(")
        && !model_patch_graph.contains("matrix.materialize(")
        && !model_patch_graph.contains("matrix.dequantize(")
        && model_patches.contains("matrix.materialize(backend, context)")
        && model_patches.contains("quantize_matrix(")
        && model_patches.contains("quantize_linear_matrix(")
        && !model_patches.contains("matrix.dequantize(")
        && ownership_generator.contains("TASK_102_CONCERNS")
        && ownership_generator.contains("TASK_102_VALIDATIONS")
        && model_quantized_autograd_tests.contains("input_arity(), 6")
        && model_quantized_autograd_tests
            .contains("saved_tensor_versions_follow_the_source_fp8_cache_policy")
        && model_quantized_autograd_tests
            .contains("native_module_adapter_reuses_canonical_quantized_parameter_storage")
        && model_quantized_autograd_tests
            .contains("canonical_identity_and_scoped_materialization_bind_source_and_workspace");
    let task_511_policy_trace = [
        (
            "model_quantization_contracts",
            "task511-patch-adapter-delegates-quantized-materialization-and-codecs",
        ),
        (
            "model_weight_adapter_contracts",
            "task511-patch-loader-delegates-source-family-selection-once",
        ),
        (
            "workspace_tensor_z_h_patch_graph_domain",
            "task511-patch-adapter-delegates-ordered-graph-lifecycle",
        ),
    ]
    .iter()
    .all(|(concern_name, required_mapping)| {
        policy_concerns
            .iter()
            .find(|entry| {
                entry.get("concern").and_then(serde_json::Value::as_str) == Some(*concern_name)
            })
            .is_some_and(|entry| {
                entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some("comfy-parity-patch-loading-merge-quantized-adapter")
                        })
                    })
                    && entry
                        .get("required_mappings")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|mappings| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required_mapping)
                            })
                        })
                    && entry
                        .get("validation")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|validations| {
                            validations.iter().any(|validation| {
                                validation.as_str() == Some("VAL-PATCH-ADAPTER-001")
                            })
                        })
            })
    });
    let task_511_catalog_trace = [
        "model_quantization_contracts",
        "model_weight_adapter_contracts",
        "workspace_tensor_z_h_patch_graph_domain",
    ]
    .iter()
    .all(|concern| {
        ownership_catalog
            .lines()
            .find(|line| line.starts_with(&format!("{concern},")))
            .is_some_and(|line| {
                line.contains("comfy-parity-patch-loading-merge-quantized-adapter")
                    && line.contains("VAL-PATCH-ADAPTER-001")
                    && (line.contains("implementation_pending")
                        || line.contains("authoritative_owner_confirmed"))
            })
    });
    let task_511_adapters_delegate_canonical_owners = model_patches
        .contains("WeightAdapterRegistry.load_unique(&request)?")
        && model_patches.contains("PatchGraph::checked_semantic(")
        && model_patches.contains("graph.append_semantic(")
        && model_patches.contains("graph.apply_single_tensor(")
        && model_patches.contains("matrix.materialize(backend, context)")
        && model_patches.contains("quantize_matrix(")
        && model_patches.contains("quantize_linear_matrix(")
        && !model_patches.contains(concat!("struct PatchPayload", "Parser"))
        && !model_patches.contains("fn load_family(")
        && !model_patches.contains("encode_float8")
        && !model_patches.contains("decode_float8")
        && model_weight_adapter.contains("pub fn load_unique(")
        && model_weight_adapter.contains("pub fn to_patch_payload(")
        && model_patch_graph.contains("pub fn append_semantic(")
        && model_patch_graph.contains("pub fn apply_single_tensor(")
        && model_quantization.contains("pub fn quantize_matrix(")
        && model_quantization.contains("pub fn quantize_linear_matrix(")
        && model_patch_adapter_tests
            .contains("val_patch_adapter_001_no_duplicate_family_parser_or_owner")
        && ownership_generator.contains("TASK_511_CONCERNS")
        && ownership_generator.contains("TASK_511_VALIDATIONS");
    let sd1_tokenizer_definitions =
        production_source_occurrences(&sources, "pub struct Sd1Tokenizer");
    let sd1_token_sequence_definitions =
        production_source_occurrences(&sources, "pub struct TokenSequence");
    let native_prompt_tokenizer_definitions =
        production_source_occurrences(&sources, "pub struct NativePromptTokenizer");
    let sentencepiece_tokenizer_definitions =
        production_source_occurrences(&sources, "pub struct SentencePieceTokenizer");
    let sentencepiece_parser_definitions =
        production_source_occurrences(&sources, "fn parse_sentencepiece_model(");
    let verified_sentencepiece_definitions =
        production_source_occurrences(&sources, "pub struct VerifiedSentencePieceVocabulary");
    let verified_embedding_archive_definitions =
        production_source_occurrences(&sources, "pub struct VerifiedEmbeddingArchivePayload");
    let task_512_has_one_canonical_tokenizer_and_artifact_owner = sd1_tokenizer_definitions.len()
        == 1
        && sd1_tokenizer_definitions[0].contains("crates/comfy_model/src/clip.rs")
        && sd1_token_sequence_definitions.len() == 1
        && sd1_token_sequence_definitions[0].contains("crates/comfy_model/src/clip.rs")
        && native_prompt_tokenizer_definitions.len() == 1
        && native_prompt_tokenizer_definitions[0]
            .contains("crates/comfy_model/src/clip_tokenizer.rs")
        && sentencepiece_tokenizer_definitions.len() == 1
        && sentencepiece_tokenizer_definitions[0]
            .contains("crates/comfy_model/src/clip_tokenizer.rs")
        && sentencepiece_parser_definitions.len() == 1
        && sentencepiece_parser_definitions[0].contains("crates/comfy_model/src/formats.rs")
        && verified_sentencepiece_definitions.len() == 1
        && verified_sentencepiece_definitions[0].contains("crates/comfy_model/src/model_store.rs")
        && verified_embedding_archive_definitions.len() == 1
        && verified_embedding_archive_definitions[0]
            .contains("crates/comfy_model/src/model_store.rs")
        && model_clip.contains("pub const SD1_VOCABULARY_SIZE: usize = 49_408;")
        && model_clip.contains("pub const SD1_MERGE_COUNT: usize = 48_894;")
        && model_clip.contains("merge_lines.next() != Some(\"#version: 0.2\")")
        && model_clip.contains("SD1 vocabulary token IDs are not the contiguous canonical domain")
        && model_clip.matches("merge_ranks:").count() == 1
        && model_clip.matches("fn byte_encoder(").count() == 1
        && model_clip_tokenizer.matches("fn pack(").count() == 1;
    let task_512_adapters_delegate_without_unverified_bypasses = model_clip_tokenizer
        .contains("NativePromptTokenizer::empty_token_ids(")
        && model_clip_tokenizer.contains("tokenizer.encode_content(text, cancellation)")
        && model_clip_tokenizer.contains("VerifiedSentencePieceVocabulary")
        && model_clip_tokenizer.contains("VerifiedEmbeddingArchivePayload")
        && model_clip_tokenizer.contains("payload.has_nested_string_to_param()")
        && model_store.contains("pub fn verified_sentencepiece_vocabulary(")
        && model_store.contains("pub fn verified_embedding_archive(")
        && model_formats.contains("pub(crate) fn parse_verified_embedding_archive_file(")
        && model_formats.contains("pub(crate) fn has_nested_string_to_param(")
        && [
            "UnverifiedEmbeddingTensorRows",
            "project_verified_embedding_candidates",
            "concatenate_unverified_bundled_embedding_rows",
            "select_unverified_named_embedding_rows",
            "pub fn from_vocabulary(",
            "pub fn parse_embedding_archive(",
            "pub struct EmbeddingArchiveEntry",
        ]
        .into_iter()
        .all(|forbidden| production_source_occurrences(&sources, forbidden).is_empty())
        && model_clip_tokenizer_tests
            .contains("sentencepiece_accepts_only_current_model_store_verified_vocabulary")
        && model_clip_tokenizer_tests
            .contains("textual_inversion_selection_priority_and_bundle_order_match_source")
        && model_clip_tokenizer_tests
            .contains("groups_larger_than_empty_section_capacity_make_bounded_progress")
        && model_clip_tokenizer_tests.contains("embedding:missing,")
        && model_clip_tokenizer_tests.contains(
            "canonical_sd1_artifact_admission_rejects_noncanonical_id_domains_and_merge_cardinality",
        );
    let provider_identity_surface = runtime_controller_production
        .split_once("pub trait NativeDiffusionProvider: Send + Sync {")
        .and_then(|(_, remainder)| remainder.split_once("    fn load("))
        .map(|(identity_surface, _)| identity_surface);
    let fixture_provider_identity_surface = native_diffusion_fixture
        .split_once("impl NativeDiffusionProvider for NativeDiffusionFixture {")
        .and_then(|(_, remainder)| remainder.split_once("    fn load("))
        .map(|(identity_surface, _)| identity_surface);
    let task_512_native_diffusion_binds_canonical_tokenizer_identity = model_native_diffusion
        .contains("pub const SD15_VOCAB_SIZE: usize = crate::clip::SD1_VOCABULARY_SIZE;")
        && model_native_diffusion
            .contains("pub const SD15_TOKEN_COUNT: usize = crate::clip::SD1_CONTEXT_LENGTH;")
        && model_native_diffusion.contains(".encode_fixed_token_ids(text, cancellation)")
        && !model_native_diffusion.contains("merge_ranks:")
        && !model_native_diffusion.contains("fn byte_encoder(")
        && runtime_cache_production.contains("    tokenizer_digest: String,")
        && runtime_controller_production
            .contains("cache_identities: CanonicalNativeDiffusionCacheIdentities")
        && runtime_cache_production.contains("pub struct CanonicalNativeDiffusionCacheIdentities")
        && provider_identity_surface.is_some_and(|surface| {
            surface.contains("fn cache_identities(")
                && surface.contains("&CancellationToken")
                && surface.contains("CanonicalNativeDiffusionCacheIdentities")
                && !surface.contains("fn model_digest(")
                && !surface.contains("fn tokenizer_digest(")
                && !surface.contains("fn clip_cache_identities(")
                && !surface.contains("fn vae_cache_identities(")
                && !surface.contains("fn conditioning_cache_identities(")
        })
        && runtime_controller_production
            .contains("self.admitted_identities.tokenizer_digest() != bundle.tokenizer_digest()")
        && runtime_cache_production.contains(
            "digests.insert(\"tokenizer.sd1\".to_owned(), self.tokenizer_digest.clone());",
        )
        && runtime_controller_production.contains(
            "NativeModelPayload::sd1_clip(bundle.tokenizer.clone(), bundle.clip.clone())",
        )
        && runtime_controller_production.contains("NativeStoredPayload::Model(Arc::new(payload))")
        && runtime_controller_production
            .contains(".map_err(|error| invalid_diffusion_input(&error.to_string()))?")
        && !runtime_controller_production.contains("NativeDiffusionHandle")
        && fixture_provider_identity_surface.is_some_and(|surface| {
            surface.contains("fn cache_identities(")
                && surface.contains("checkpoint_identity_snapshot(cancellation)")
                && surface.contains("tokenizer.identity()")
                && surface.contains(".digest()")
                && !surface.contains("fn model_digest(")
                && !surface.contains("fn tokenizer_digest(")
                && !surface.contains("fn clip_cache_identities(")
                && !surface.contains("fn vae_cache_identities(")
                && !surface.contains("fn conditioning_cache_identities(")
        });
    let task_512_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_diffusion_language_tokenization_and_embedding_artifacts")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str() == Some("comfy-parity-sd1-tokenizer-owner-consolidation")
                    })
                })
                && entry
                    .get("requirements")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|requirements| {
                        requirements
                            .iter()
                            .any(|requirement| requirement.as_str() == Some("Requirement 41"))
                    })
                && entry
                    .get("validation")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|validations| {
                        ["VAL-CLIP-001", "VAL-MODEL-FORMAT-001", "VAL-OWNERSHIP-001"]
                            .iter()
                            .all(|required| {
                                validations
                                    .iter()
                                    .any(|validation| validation.as_str() == Some(*required))
                            })
                    })
        });
    let task_512_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_diffusion_language_tokenization_and_embedding_artifacts,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-sd1-tokenizer-owner-consolidation")
                && line.contains("VAL-MODEL-FORMAT-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let qwen2_tokenizer_definitions =
        production_source_occurrences(&sources, "pub struct Qwen2BpeTokenizer");
    let task383_qwen2_has_one_native_prompt_tokenizer_family = qwen2_tokenizer_definitions.len()
        == 1
        && qwen2_tokenizer_definitions[0].contains("crates/comfy_model/src/clip_tokenizer.rs")
        && model_clip_tokenizer.contains("Qwen2ByteBpe(Qwen2BpeTokenizer)")
        && model_clip_tokenizer.contains("NativeTokenizerFamily::Qwen2ByteBpe(tokenizer)")
        && !model_clip_tokenizer.contains("struct QwenTokenizer");
    let task383_qwen2_admission_identity_and_residency_are_canonical = model_clip_tokenizer
        .contains("pub fn from_artifacts(")
        && model_clip_tokenizer.contains("validate_qwen2_configuration(")
        && model_clip_tokenizer.contains("text.nfc().collect()")
        && model_clip_tokenizer.contains("fn apply_merges(")
        && model_clip_tokenizer.contains("String::from_utf8_lossy(bytes)")
        && model_clip_tokenizer.contains("sim.comfy.qwen2-byte-bpe.v1")
        && model_clip_tokenizer.contains("NativeTokenizerFamily::Qwen2ByteBpe(tokenizer) =>")
        && !model_clip_tokenizer.contains("RngStreamAddress")
        && !model_clip_tokenizer.contains("NativeCache")
        && !model_clip_tokenizer.contains("OutputTransaction");
    let task383_qwen2_source_fixtures_and_failures_are_executable = model_clip_tokenizer_tests
        .contains("qwen2_real_artifacts_preserve_text_added_tokens_and_minimum_padding")
        && model_clip_tokenizer_tests
            .contains("qwen2_fixture_manifest_pins_source_artifacts_and_provenance")
        && model_clip_tokenizer_tests
            .contains("qwen2_artifact_admission_and_cancellation_are_typed")
        && model_clip_tokenizer_tests.contains("\"code-inferred\"");
    let task383_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_diffusion_language_tokenization_and_embedding_artifacts")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some("comfy-parity-native-qwen2-tokenizer-foundation")
                    })
                })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "qwen2-byte-bpe-is-one-checked-native-prompt-tokenizer-family",
                            "qwen2-artifacts-bind-semantic-identity-and-owned-residency",
                            "qwen2-source-fingerprinted-tests-cover-specials-nfc-profiles-and-failures",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task383_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_diffusion_language_tokenization_and_embedding_artifacts,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-qwen2-tokenizer-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let gemma_tokenizer_definitions =
        production_source_occurrences(&sources, "pub struct GemmaTokenizer");
    let task391_gemma_has_one_native_prompt_tokenizer_family = gemma_tokenizer_definitions.len()
        == 1
        && gemma_tokenizer_definitions[0].contains("crates/comfy_model/src/clip_tokenizer.rs")
        && model_clip_tokenizer.contains("Gemma(GemmaTokenizer)")
        && model_clip_tokenizer.contains("NativeTokenizerFamily::Gemma(tokenizer)")
        && !model_clip_tokenizer.contains("struct GemmaPromptTokenizer");
    let task391_gemma_admission_identity_residency_and_cleanup_are_canonical = model_clip_tokenizer
        .contains("pub fn gemma3(")
        && model_clip_tokenizer.contains("pub fn gemma4_from_tokenizer_json(")
        && model_clip_tokenizer.contains("Gemma3SentencePiece")
        && model_clip_tokenizer.contains("Gemma4TokenizerJson")
        && model_clip_tokenizer.contains("GEMMA3_IMAGE_TOKEN")
        && model_clip_tokenizer.contains("GEMMA4_IMAGE_TOKEN")
        && model_clip_tokenizer.contains("GEMMA4_AUDIO_TOKEN")
        && model_clip_tokenizer.contains("GEMMA4_VIDEO_TOKEN")
        && model_clip_tokenizer.contains("pub fn decode_generated(")
        && model_clip_tokenizer.contains("sim.comfy.gemma3-sentencepiece-tokenizer.v1")
        && !model_clip_tokenizer.contains("RngStreamAddress")
        && !model_clip_tokenizer.contains("NativeCache")
        && !model_clip_tokenizer.contains("OutputTransaction");
    let task391_gemma_source_fixtures_and_failures_are_executable = model_clip_tokenizer_tests
        .contains("gemma_tokenizers_are_profile_checked_left_padded_and_cleanup_exact")
        && model_clip_tokenizer_tests.contains("gemma4-tokenizer.json")
        && model_clip_tokenizer_tests.contains("licensed_checkpoint_included")
        && model_clip_tokenizer_tests.contains("UnsupportedSpecialTokenDecode");
    let task391_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_diffusion_language_tokenization_and_embedding_artifacts")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str() == Some("comfy-parity-native-gemma-tokenizer-foundation")
                    })
                })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "gemma-tokenizers-extend-the-sole-native-prompt-tokenizer-family",
                            "gemma3-admission-binds-verified-sentencepiece-external-specials-and-left-padding",
                            "gemma4-tokenizer-json-binds-unigram-specials-cleanup-identity-and-residency",
                            "gemma-source-fingerprinted-tests-cover-admission-left-padding-cleanup-and-failures",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task391_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_diffusion_language_tokenization_and_embedding_artifacts,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-gemma-tokenizer-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let native_clip_text_definitions =
        production_source_occurrences(&sources, "pub struct NativeClipText");
    let clip_text_configuration_definitions =
        production_source_occurrences(&sources, "pub struct ClipTextConfiguration");
    let sd1_clip_text_encoder_definitions =
        production_source_occurrences(&sources, "pub struct Sd1ClipTextEncoder");
    let task_339_clip_text_has_one_architecture_owner = native_clip_text_definitions.len() == 1
        && native_clip_text_definitions[0].contains("crates/comfy_model/src/clip_text.rs")
        && clip_text_configuration_definitions.len() == 1
        && clip_text_configuration_definitions[0].contains("crates/comfy_model/src/clip_text.rs")
        && sd1_clip_text_encoder_definitions.len() == 1
        && sd1_clip_text_encoder_definitions[0].contains("crates/comfy_model/src/clip.rs")
        && model_clip.contains("transformer: NativeClipText")
        && !model_clip_vision.contains("NativeClipText");
    let task_339_clip_text_delegates_canonical_mechanics = model_clip_text
        .contains("layer_norm_1: NativeModule")
        && model_clip_text.contains("token_embedding: NativeModule")
        && model_clip_text.contains("scaled_dot_product_attention_with_context(")
        && model_clip_text.contains(".admit_backend_target(")
        && model_clip_text.contains("for (layer_index, layer) in self.layers.iter().enumerate()")
        && model_clip_text.contains("pool_final_hidden(")
        && !model_clip_text.contains("ArtifactIndex")
        && !model_clip_text.contains("ModelStore")
        && !model_clip_text.contains("CpuWorkspaceAuthority")
        && !model_clip_text.contains("pub struct CancellationToken")
        && model_clip.contains("self.transformer.forward(");
    let task_339_clip_text_adapter_semantics_are_executable = model_clip_text_tests
        .contains("val_clip_001_text_rows_execute_and_extend_cumulative_ledger")
        && model_clip_text_tests
            .contains("hidden_capture_continues_remaining_layers_and_final_pooling")
        && model_clip_text_tests
            .contains("causal_padding_projection_and_layer_list_semantics_are_exact")
        && model_clip_text_tests.contains("embedding_input_num_tokens_and_all_activations_execute")
        && model_clip_text_tests
            .contains("invalid_configuration_inputs_masks_projection_and_layers_fail_typed")
        && model_clip_text_tests
            .contains("cancellation_and_workspace_oom_publish_nothing_and_converge")
        && model_clip.contains(
            "a hidden-layer conditioning selection must still pool the final transformer state",
        );
    let task_339_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_architecture_execution")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text::NativeClipText")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str() == Some("comfy-parity-clip-text-transformer-foundation")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "clip-text-assembles-canonical-native-modules",
                            "clip-text-admission-and-attention-delegate-canonical-owners",
                            "clip-text-capture-continues-to-final-pooling",
                            "sd1-text-encoder-is-a-focused-transformer-adapter",
                            "clip-text-root-exports-one-canonical-module",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task_339_catalog_trace = ownership_catalog
        .lines()
        .find(|line| line.starts_with("native_vision_text_transformer_architecture_execution,"))
        .is_some_and(|line| {
            line.contains("comfy_model::clip_text::NativeClipText")
                && line.contains("comfy-parity-clip-text-transformer-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-OWNERSHIP-001")
                && (line.contains("consolidation_required")
                    || line.contains("authoritative_owner_confirmed"))
        });
    let native_t5_text_encoder_definitions =
        production_source_occurrences(&sources, "pub struct NativeT5TextEncoder");
    let bidirectional_text_configuration_definitions =
        production_source_occurrences(&sources, "pub struct BidirectionalTextConfiguration");
    let relative_position_bucket_definitions =
        production_source_occurrences(&sources, "pub fn relative_position_bucket(");
    let task_342_t5_bidirectional_has_one_architecture_owner =
        native_t5_text_encoder_definitions.len() == 1
            && native_t5_text_encoder_definitions[0]
                .contains("crates/comfy_model/src/clip_text_encoder_t5.rs")
            && bidirectional_text_configuration_definitions.len() == 1
            && bidirectional_text_configuration_definitions[0]
                .contains("crates/comfy_model/src/clip_text_encoder_t5.rs")
            && relative_position_bucket_definitions.len() == 1
            && relative_position_bucket_definitions[0]
                .contains("crates/comfy_model/src/clip_text_encoder_t5.rs");
    let task_342_t5_bidirectional_delegates_canonical_mechanics = model_clip_text_encoder_t5
        .contains("token_embedding: NativeModule")
        && model_clip_text_encoder_t5.contains("query: NativeModule")
        && model_clip_text_encoder_t5.contains("rms_norm_with_context_exact_native(")
        && model_clip_text_encoder_t5.contains("scaled_dot_product_attention_with_context(")
        && model_clip_text_encoder_t5.contains(".admit_backend_target(")
        && model_clip_text_encoder_t5.contains("tokenizer")
        && model_clip_text_encoder_t5.contains(".tokenize(text, cancellation)")
        && !model_clip_text_encoder_t5.contains("ArtifactIndex")
        && !model_clip_text_encoder_t5.contains("ModelStore")
        && !model_clip_text_encoder_t5.contains("CpuWorkspaceAuthority")
        && !model_clip_text_encoder_t5.contains("pub struct CancellationToken")
        && !model_clip_text_encoder_t5.contains("pub struct BackendCapabilityMatrix")
        && !model_clip_text_encoder_t5.contains("pub struct OutputTransaction");
    let task_342_t5_bidirectional_adapter_semantics_are_executable =
        model_clip_text_encoder_t5_tests
            .contains("val_clip_001_t5_bidirectional_rows_execute_and_extend_cumulative_ledger")
            && model_clip_text_encoder_t5_tests
                .contains("t5_relative_attention_gating_capture_and_pooling_execute")
            && model_clip_text_encoder_t5_tests
                .contains("bert_embeddings_masks_post_norm_and_embedding_input_execute")
            && model_clip_text_encoder_t5_tests
                .contains("tokenizer_input_delegates_to_verified_canonical_sentencepiece_owner")
            && model_clip_text_encoder_t5_tests
                .contains("target_shape_mask_layer_projection_cancellation_and_oom_fail_typed");
    let task_342_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_t5_bidirectional_execution")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text_encoder_t5::NativeT5TextEncoder")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str() == Some("comfy-parity-clip-text-encoder-t5-foundation")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "t5-bidirectional-assembles-canonical-native-modules",
                            "t5-bidirectional-delegates-rms-attention-and-admission",
                            "t5-bidirectional-delegates-token-input",
                            "t5-bidirectional-owns-no-foundational-authority",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task_342_catalog_trace = ownership_catalog
        .lines()
        .find(|line| line.starts_with("native_vision_text_transformer_t5_bidirectional_execution,"))
        .is_some_and(|line| {
            line.contains("comfy_model::clip_text_encoder_t5::NativeT5TextEncoder")
                && line.contains("comfy-parity-clip-text-encoder-t5-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-OWNERSHIP-001")
                && (line.contains("consolidation_required")
                    || line.contains("authoritative_owner_confirmed"))
        });
    let native_decoder_text_encoder_definitions =
        production_source_occurrences(&sources, "pub struct NativeDecoderTextEncoder");
    let decoder_text_configuration_definitions =
        production_source_occurrences(&sources, "pub struct DecoderTextConfiguration");
    let decoder_kv_state_definitions =
        production_source_occurrences(&sources, "pub struct DecoderKvState");
    let task343_decoder_llm_graph_and_transient_cache_have_one_owner =
        native_decoder_text_encoder_definitions.len() == 1
            && native_decoder_text_encoder_definitions[0]
                .contains("crates/comfy_model/src/clip_text_encoder_decoder.rs")
            && decoder_text_configuration_definitions.len() == 1
            && decoder_text_configuration_definitions[0]
                .contains("crates/comfy_model/src/clip_text_encoder_decoder.rs")
            && decoder_kv_state_definitions.len() == 1
            && decoder_kv_state_definitions[0]
                .contains("crates/comfy_model/src/clip_text_encoder_decoder.rs");
    let task343_decoder_delegates_every_foundational_mechanic = model_clip_text_encoder_decoder
        .contains("token_embedding: NativeModule")
        && model_clip_text_encoder_decoder.contains("query: NativeModule")
        && model_clip_text_encoder_decoder.contains("rms_norm_with_context_exact_native(")
        && model_clip_text_encoder_decoder.contains("scaled_dot_product_attention_with_context(")
        && model_clip_text_encoder_decoder.contains(".admit_backend_target(")
        && model_clip_text_encoder_decoder.contains("transaction: &RngTransaction")
        && model_clip_text_encoder_decoder
            .contains("let mut staged_transaction = transaction.clone()")
        && model_clip_text_encoder_decoder.contains("pub fn tokenize_decoder_prompt(")
        && model_clip_text_encoder_decoder.contains(".tokenize(text, cancellation)")
        && model_clip_text_encoder_decoder.contains("pub struct DecoderProfileFact")
        && model_clip_text_encoder_decoder.contains("pub const DECODER_PROFILE_FACTS")
        && model_clip_text_encoder_decoder.contains("pub fn decoder_symbol_behavior(")
        && model_clip_text_encoder_decoder.contains("keys: Tensor")
        && model_clip_text_encoder_decoder.contains("convolution_state: Tensor")
        && !model_clip_text_encoder_decoder.contains("RngStream::")
        && !model_clip_text_encoder_decoder.contains("RngStreamAddress")
        && !model_clip_text_encoder_decoder.contains("ArtifactIndex")
        && !model_clip_text_encoder_decoder.contains("ModelStore")
        && !model_clip_text_encoder_decoder.contains("NativeCache")
        && !model_clip_text_encoder_decoder.contains("CpuWorkspaceAuthority")
        && !model_clip_text_encoder_decoder.contains("pub struct CancellationToken")
        && !model_clip_text_encoder_decoder.contains("pub struct BackendCapabilityMatrix")
        && !model_clip_text_encoder_decoder.contains("pub struct OutputTransaction");
    let task343_decoder_adapters_and_failure_atomicity_are_executable =
        model_clip_text_encoder_decoder_tests
            .contains("val_clip_001_decoder_rows_execute_and_extend_cumulative_ledger")
            && model_clip_text_encoder_decoder_tests
                .contains("decoder_graph_executes_causal_gqa_sliding_cache_and_batch_safe_append")
            && model_clip_text_encoder_decoder_tests
                .contains("gpt_oss_sinks_yarn_router_and_experts_execute")
            && model_clip_text_encoder_decoder_tests
                .contains("qwen35_linear_recurrent_convolution_and_hybrid_graph_execute")
            && model_clip_text_encoder_decoder_tests
                .contains("caller_addressed_generation_is_deterministic_and_does_not_mutate_input_transaction")
            && model_clip_text_encoder_decoder_tests
                .contains("target_shape_cache_generation_cancellation_and_oom_fail_typed_and_atomic")
            && model_clip_text_encoder_decoder_tests
                .contains("source_profile_facts_cover_every_static_and_factory_decoder_profile_exactly")
            && model_clip_text_encoder_decoder_tests
                .contains("decoder_symbol_behavior(symbol).is_some()");
    let prepared_text_request_definitions =
        production_source_occurrences(&sources, "pub struct DecoderPreparedTextRequest");
    let prepared_generation_prompt_definitions =
        production_source_occurrences(&sources, "pub struct DecoderPreparedGenerationPrompt");
    let task380_prepared_decoder_has_one_borrowed_invocation_local_owner =
        prepared_text_request_definitions.len() == 1
            && prepared_text_request_definitions[0]
                .contains("crates/comfy_model/src/clip_text_encoder_decoder.rs")
            && prepared_generation_prompt_definitions.len() == 1
            && prepared_generation_prompt_definitions[0]
                .contains("crates/comfy_model/src/clip_text_encoder_decoder.rs")
            && model_clip_text_encoder_decoder.contains("embeddings: &'a Tensor")
            && model_clip_text_encoder_decoder.contains("sampling_history: &'a [i64]")
            && !model_clip_text_encoder_decoder.contains("Serialize for DecoderPrepared")
            && !model_clip_text_encoder_decoder.contains("Deserialize for DecoderPrepared");
    let task380_prepared_decoder_delegates_one_graph_cache_rng_and_rope_owner =
        model_clip_text_encoder_decoder.contains("pub fn forward_prepared(")
            && model_clip_text_encoder_decoder.contains("self.forward_hidden(")
            && model_clip_text_encoder_decoder.contains("pub fn generate_prepared(")
            && model_clip_text_encoder_decoder.contains("self.generate_with_prefill(")
            && model_clip_text_encoder_decoder.contains("precompute_multidimensional_rope(")
            && model_clip_text_encoder_decoder.contains("transaction: &RngTransaction");
    let task380_prepared_decoder_boundaries_are_executable = model_clip_text_encoder_decoder_tests
        .contains("prepared_prefill_shares_generation_rng_cache_and_multidimensional_rope")
        && model_clip_text_encoder_decoder_tests
            .contains("prepared_prefill_rejects_shape_cache_and_cancellation_without_rng_mutation");
    let task382_deepstack_has_one_borrowed_decoder_owner =
        production_source_occurrences(&sources, "pub struct DecoderPreparedDeepstack").len() == 1
            && model_clip_text_encoder_decoder.contains("visual_position_mask: &'a [bool]")
            && model_clip_text_encoder_decoder.contains("layers: &'a [Tensor]")
            && !model_clip_text_encoder_decoder.contains("Serialize for DecoderPreparedDeepstack")
            && !model_clip_text_encoder_decoder
                .contains("Deserialize for DecoderPreparedDeepstack");
    let task382_deepstack_delegates_canonical_prefill_cache_rng_and_indexing =
        model_clip_text_encoder_decoder.contains("fn validate_prepared_deepstack(")
            && model_clip_text_encoder_decoder
                .contains("prepared deepstack is valid only for uncached prefill")
            && model_clip_text_encoder_decoder
                .contains("index_add_in_place_with_context_exact_native(")
            && model_clip_text_encoder_decoder.contains("if capture == Some(layer_index)")
            && model_clip_text_encoder_decoder.contains("prompt.deepstack")
            && model_clip_text_encoder_decoder.contains("transaction: &RngTransaction");
    let task382_deepstack_boundaries_are_executable = model_clip_text_encoder_decoder_tests
        .contains("prepared_deepstack_is_exact_post_layer_prefill_only_and_transactional")
        && model_clip_text_encoder_decoder_tests.contains("visual_position_mask: &no_visual")
        && model_clip_text_encoder_decoder_tests.contains("too_many_layers")
        && model_clip_text_encoder_decoder_tests.contains("transaction.checkpoint()")
        && model_clip_text_encoder_decoder_tests.contains("scratch.in_use_bytes()");
    let task382_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_unidirectional_decoder_execution")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some("comfy-parity-native-prepared-decoder-deepstack-foundation")
                    })
                })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "prepared-deepstack-is-borrowed-validated-and-prefill-only",
                            "prepared-deepstack-adds-after-layer-before-capture-through-canonical-indexing",
                            "prepared-deepstack-tests-exact-prefill-only-rollback",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task382_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_unidirectional_decoder_execution,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-prepared-decoder-deepstack-foundation")
                && line.contains("VAL-TENSOR-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task384_qwen3_query_key_norm_has_one_decoder_owner = model_clip_text_encoder_decoder
        .matches("fn normalize_attention_heads(")
        .count()
        == 1
        && model_clip_text_encoder_decoder.contains("query_norm_weight: Option<Tensor>")
        && model_clip_text_encoder_decoder.contains("key_norm_weight: Option<Tensor>")
        && !model_clip_text_encoder_multimodal.contains("fn normalize_attention_heads(");
    let task384_qwen3_delegates_rms_rope_attention_cache_and_residency =
        model_clip_text_encoder_decoder.contains("normalize_attention_heads(")
            && model_clip_text_encoder_decoder.contains("rms_norm_with_context_exact_native(")
            && model_clip_text_encoder_decoder.contains("let query = apply_decoder_rope(")
            && model_clip_text_encoder_decoder.contains("stage_attention_cache(")
            && model_clip_text_encoder_decoder.contains("expand_grouped_query(")
            && model_clip_text_encoder_decoder.contains("query_norm_weight")
            && model_clip_text_encoder_decoder.contains("normalization_tensors(&self)");
    let task384_qwen3_fixture_is_exact_and_failure_atomic = model_clip_text_encoder_decoder_tests
        .contains("qwen3_query_key_norm_is_per_head_pre_rope_checkpoint_backed_and_cache_exact")
        && model_clip_text_encoder_decoder_tests
            .contains("e2f7a9dc822b118de4e2b20f5db96609c9d4bb0ab1d7c557fef3ba3b76f3d0f1")
        && model_clip_text_encoder_decoder_tests
            .contains("query/key normalization weights must exactly match the decoder profile")
        && model_clip_text_encoder_decoder_tests.contains("scratch.in_use_bytes()");
    let task384_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_unidirectional_decoder_execution")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some("comfy-parity-native-qwen3-decoder-exactness-foundation")
                    })
                })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "qwen3-query-key-norm-is-checkpoint-backed-per-head-and-pre-rope",
                            "qwen3-query-key-norm-participates-in-identity-and-residency",
                            "qwen3-query-key-norm-fixture-proves-cache-equivalence-admission-and-rollback",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task384_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_unidirectional_decoder_execution,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-qwen3-decoder-exactness-foundation")
                && line.contains("VAL-MODEL-FAMILY-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task392_gemma3_profiles_have_one_canonical_decoder_owner =
        production_source_occurrences(&sources, "pub enum Gemma3DecoderProfile {").len() == 1
            && production_source_occurrences(&sources, "pub struct Gemma3DecoderConfiguration {")
                .len()
                == 1
            && production_source_occurrences(&sources, "fn rope_for_layer(").len() == 1
            && !model_clip_text_encoder_multimodal.contains("struct Gemma3DecoderConfiguration");
    let task392_gemma3_delegates_rope_norm_attention_cache_and_rng = model_clip_text_encoder_decoder
        .contains("pub fn gemma3_decoder_configuration(")
        && model_clip_text_encoder_decoder.contains("gemma3.local_rope")
        && model_clip_text_encoder_decoder
            .contains("let layer_rope = configuration.rope_for_layer(self.kind)")
        && model_clip_text_encoder_decoder.contains("normalize_attention_heads(")
        && model_clip_text_encoder_decoder.contains("rms_norm_with_context_exact_native(")
        && model_clip_text_encoder_decoder.contains("stage_attention_cache(")
        && model_clip_text_encoder_decoder.contains("transaction: &RngTransaction")
        && model_clip_text_encoder_decoder
            .contains("let mut staged_transaction = transaction.clone()");
    let task392_gemma3_fixture_is_exact_and_failure_atomic = model_clip_text_encoder_decoder_tests
        .contains("gemma3_alternating_rope_norm_scaling_cache_and_generation_are_exact")
        && model_clip_text_encoder_decoder_tests
            .contains("30447eeb298bdaa2edcf48e1b2407903f2da625da3e2bdf0313acc0bc1b046a8")
        && model_clip_text_encoder_decoder_tests.contains(
            "Gemma layers require post-attention and post-feed-forward normalization weights",
        )
        && model_clip_text_encoder_decoder_tests.contains("transaction.checkpoint()")
        && model_clip_text_encoder_decoder_tests.contains("scratch.in_use_bytes()");
    let task392_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_unidirectional_decoder_execution")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some("comfy-parity-native-gemma3-decoder-exactness-foundation")
                    })
                })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "gemma3-source-profiles-are-closed-executable-decoder-configurations",
                            "gemma3-selects-local-and-global-rope-inside-one-canonical-decoder",
                            "gemma3-executes-four-stage-rms-residual-and-checkpoint-backed-query-key-normalization",
                            "gemma3-fixture-proves-profile-rope-cache-generation-and-failure-atomicity",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task392_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_unidirectional_decoder_execution,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-gemma3-decoder-exactness-foundation")
                && line.contains("VAL-RNG-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task393_gemma4_profiles_have_one_canonical_decoder_owner =
        production_source_occurrences(&sources, "pub enum Gemma4DecoderProfile {").len() == 1
            && production_source_occurrences(&sources, "pub struct Gemma4DecoderConfiguration {")
                .len()
                == 1
            && production_source_occurrences(&sources, "pub struct Gemma4PerLayerWeights {").len()
                == 1
            && production_source_occurrences(&sources, "pub struct Gemma4LayerInputWeights {")
                .len()
                == 1
            && !model_clip_text_encoder_multimodal.contains("struct Gemma4DecoderConfiguration");
    let task393_gemma4_delegates_head_rope_shared_kv_layer_input_cache_and_rng =
        model_clip_text_encoder_decoder.contains("pub fn gemma4_decoder_configuration(")
            && model_clip_text_encoder_decoder.contains("fn head_dimension_for_layer(")
            && model_clip_text_encoder_decoder.contains("fn feed_forward_size_for_layer(")
            && model_clip_text_encoder_decoder.contains("fn apply_decoder_layer_rope(")
            && model_clip_text_encoder_decoder.contains("let mut shared_sliding = None")
            && model_clip_text_encoder_decoder.contains("let mut shared_global = None")
            && model_clip_text_encoder_decoder.contains("fn prepare_gemma4_layer_inputs(")
            && model_clip_text_encoder_decoder.contains("initial_input_ids: Option<&'a [i64]>")
            && model_clip_text_encoder_decoder.contains("stage_attention_cache(")
            && model_clip_text_encoder_decoder.contains("transaction: &RngTransaction")
            && model_clip_text_encoder_decoder
                .contains("let mut staged_transaction = transaction.clone()");
    let task393_gemma4_fixture_is_exact_and_failure_atomic = model_clip_text_encoder_decoder_tests
        .contains("gemma4_global_shared_per_layer_cache_and_generation_are_exact")
        && model_clip_text_encoder_decoder_tests
            .contains("10080fb49e529b5b7f341c47c112dba564b067c6f908d7604c200c3b78d02e16")
        && model_clip_text_encoder_decoder_tests.contains("source_profile = Some")
        && model_clip_text_encoder_decoder_tests.contains("transaction.checkpoint()")
        && model_clip_text_encoder_decoder_tests.contains("scratch.in_use_bytes()");
    let task393_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_unidirectional_decoder_execution")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some("comfy-parity-native-gemma4-decoder-exactness-foundation")
                    })
                })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "gemma4-source-profiles-are-closed-executable-decoder-configurations",
                            "gemma4-executes-global-partial-rope-shared-kv-and-double-wide-mlp-in-one-decoder",
                            "gemma4-expanded-initial-ids-drive-checkpoint-backed-per-layer-inputs",
                            "gemma4-fixture-proves-profile-cache-generation-identity-and-failure-atomicity",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task393_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_unidirectional_decoder_execution,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-gemma4-decoder-exactness-foundation")
                && line.contains("VAL-RNG-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task394_gemma3_vision_has_one_retained_projection_owner =
        production_source_occurrences(&sources, "pub struct NativeGemma3VisionProjector {").len()
            == 1
            && production_source_occurrences(&sources, "fn gemma3_pool_and_normalize(").len() == 1
            && model_clip_text_encoder_multimodal.contains("vision: Arc<NativeClipVision>")
            && model_clip_text_encoder_multimodal.contains("input_projection_weight: Tensor");
    let task394_gemma3_vision_delegates_preparation_clip_projection_and_residency =
        model_clip_text_encoder_multimodal.contains("pub fn prepare_gemma3_image(")
            && model_clip_text_encoder_multimodal.contains(".preprocess(")
            && model_clip_text_encoder_multimodal.contains("session.forward(")
            && model_clip_text_encoder_multimodal.contains("gemma3_pool_and_normalize(")
            && model_clip_text_encoder_multimodal.contains("matmul_with_context_exact_native(")
            && model_clip_text_encoder_multimodal.contains("resident_tensor_allocations(")
            && !model_clip_text_encoder_multimodal.contains("struct NativeGemma3Decoder")
            && !model_clip_text_encoder_multimodal.contains("NativeCache");
    let task394_gemma3_vision_fixture_proves_exactness_aliasing_and_rollback =
        model_clip_text_encoder_multimodal_tests
            .contains("gemma3_retained_vision_projector_is_exact_alias_aware_and_transactional")
            && model_clip_text_encoder_multimodal_tests.contains("gemma3_vision/manifest.json")
            && model_clip_text_encoder_multimodal_tests.contains("shared_norm.storage_id()")
            && model_clip_text_encoder_multimodal_tests.contains("scratch.in_use_bytes()");
    let task394_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_text_media_projection_gemma3")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some(
                    "comfy_model::clip_text_encoder_multimodal::NativeGemma3VisionProjector",
                )
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some(
                                    "comfy-parity-native-gemma3-vision-projection-foundation",
                                )
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "gemma3-vision-retains-exact-siglip-projector-graph",
                            "gemma3-vision-delegates-preprocess-transformer-pooling-norm-and-projection",
                            "gemma3-vision-binds-source-semantic-state-and-storage-residency",
                            "gemma3-vision-tests-exact-projection-aliasing-and-failure-atomicity",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task394_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_text_media_projection_gemma3,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-gemma3-vision-projection-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task395_gemma4_vision_has_one_retained_projection_owner =
        production_source_occurrences(&sources, "pub struct NativeGemma4VisionEncoder {").len()
            == 1
            && production_source_occurrences(&sources, "fn gemma4_vision_attention(").len() == 1
            && model_clip_text_encoder_multimodal.contains("patch_projection: NativeModule")
            && model_clip_text_encoder_multimodal.contains("blocks: Vec<NativeGemma4VisionBlock>")
            && model_clip_text_encoder_multimodal.contains("projector: NativeModule");
    let task395_gemma4_vision_delegates_preparation_projection_and_residency =
        model_clip_text_encoder_multimodal.contains("pub fn prepare_gemma4_visuals(")
            && model_clip_text_encoder_multimodal.contains("gemma4_patchify(")
            && model_clip_text_encoder_multimodal.contains("gemma4_add_positions(")
            && model_clip_text_encoder_multimodal.contains("gemma4_vision_attention(")
            && model_clip_text_encoder_multimodal.contains("gemma4_pool(")
            && model_clip_text_encoder_multimodal.contains("resident_tensor_allocations(")
            && model_clip_text_encoder_multimodal.contains("insert_gemma4_resident_allocation(")
            && !model_clip_text_encoder_multimodal.contains("struct NativeGemma4Decoder")
            && !model_clip_text_encoder_multimodal.contains("NativeCache");
    let task395_gemma4_vision_fixture_proves_exactness_aliasing_and_rollback =
        model_clip_text_encoder_multimodal_tests
            .contains("gemma4_retained_vision_projector_is_exact_alias_aware_and_transactional")
            && model_clip_text_encoder_multimodal_tests.contains("gemma4_vision/manifest.json")
            && model_clip_text_encoder_multimodal_tests.contains("storage_ids")
            && model_clip_text_encoder_multimodal_tests.contains("scratch.in_use_bytes()");
    let task395_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_text_media_projection_gemma4")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some(
                    "comfy_model::clip_text_encoder_multimodal::NativeGemma4VisionEncoder",
                )
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some(
                                    "comfy-parity-native-gemma4-vision-projection-foundation",
                                )
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "gemma4-vision-retains-complete-closed-three-profile-graph",
                            "gemma4-vision-executes-clipped-rope-attention-pooling-and-projector-order",
                            "gemma4-vision-consumes-canonical-image-video-soft-budgets",
                            "gemma4-vision-binds-source-semantic-state-and-size-checked-storage-residency",
                            "gemma4-vision-tests-reduced-exactness-aliasing-and-failure-atomicity",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task395_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_text_media_projection_gemma4,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-gemma4-vision-projection-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task396_gemma4_audio_has_one_retained_execution_owner =
        production_source_occurrences(&sources, "pub struct NativeGemma4AudioEncoder {").len() == 1
            && production_source_occurrences(&sources, "fn gemma4_audio_attention(").len() == 1
            && production_source_occurrences(&sources, "fn gemma4_audio_convolution(").len() == 1
            && model_clip_text_encoder_multimodal.contains("blocks: Vec<NativeGemma4AudioBlock>")
            && model_clip_text_encoder_multimodal.contains("encoder_output: NativeModule")
            && model_clip_text_encoder_multimodal.contains("projector: NativeModule");
    let task396_gemma4_audio_delegates_preparation_graph_and_residency =
        model_clip_text_encoder_multimodal.contains("pub fn prepare_gemma4_audio(")
            && model_clip_text_encoder_multimodal.contains("gemma4_audio_conv2d_layer(")
            && model_clip_text_encoder_multimodal.contains("gemma4_audio_attention(")
            && model_clip_text_encoder_multimodal.contains("gemma4_audio_convolution(")
            && model_clip_text_encoder_multimodal.contains("prepared.marker_tokens()")
            && model_clip_text_encoder_multimodal.contains("insert_gemma4_resident_allocation(")
            && !model_clip_text_encoder_multimodal.contains("struct NativeGemma4AudioDecoder")
            && !model_clip_text_encoder_multimodal.contains("NativeCache");
    let task396_gemma4_audio_fixture_proves_exactness_capability_and_rollback =
        model_clip_text_encoder_multimodal_tests
            .contains("gemma4_retained_audio_encoder_is_exact_alias_aware_and_transactional")
            && model_clip_text_encoder_multimodal_tests.contains("gemma4_audio/manifest.json")
            && model_clip_text_encoder_multimodal_tests.contains("Gemma4AudioProfile::ThirtyOneB")
            && model_clip_text_encoder_multimodal_tests.contains("scratch.in_use_bytes()");
    let task396_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_text_media_projection_gemma4_audio")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text_encoder_multimodal::NativeGemma4AudioEncoder")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some("comfy-parity-native-gemma4-audio-execution-foundation")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "gemma4-audio-retains-complete-closed-e2b-e4b-graph-and-rejects-31b",
                            "gemma4-audio-executes-subsampling-relative-attention-causal-convolution-and-projector-order",
                            "gemma4-audio-consumes-canonical-log-mel-mask-and-marker-plan",
                            "gemma4-audio-binds-source-semantic-state-and-size-checked-storage-residency",
                            "gemma4-audio-tests-reduced-exactness-capability-aliasing-and-failure-atomicity",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task396_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_text_media_projection_gemma4_audio,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-gemma4-audio-execution-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task385_qwen35_hybrid_has_one_checkpoint_backed_decoder_owner =
        production_source_occurrences(&sources, "pub struct Qwen35LinearWeights {").len() == 1
            && production_source_occurrences(&sources, "fn forward_linear_attention(").len() == 1
            && model_clip_text_encoder_decoder.contains("pub enum DecoderAttentionWeights")
            && model_clip_text_encoder_decoder.contains("Qwen35Linear(Qwen35LinearWeights)")
            && !model_clip_text_encoder_multimodal.contains("struct Qwen35LinearWeights");
    let task385_qwen35_delegates_full_gate_delta_cache_and_residency =
        model_clip_text_encoder_decoder.contains("split_qwen35_query_gate(")
            && model_clip_text_encoder_decoder.contains("*value *= sigmoid(gate)")
            && model_clip_text_encoder_decoder.contains("qwen35_causal_conv1d_update_exact(")
            && model_clip_text_encoder_decoder.contains("qwen35_chunk_gated_delta_rule_exact(")
            && model_clip_text_encoder_decoder.contains("qwen35_gated_rms_norm(")
            && model_clip_text_encoder_decoder.contains("pub step_index: usize")
            && model_clip_text_encoder_decoder.contains("normalization_tensors(&self)")
            && !model_clip_text_encoder_decoder.contains("let log_decay = vec![0.0; gate_count]")
            && !model_clip_text_encoder_decoder.contains("let beta = vec![1.0; gate_count]");
    let task385_qwen35_fixture_proves_hybrid_equivalence_and_admission =
        model_clip_text_encoder_decoder_tests
            .contains("qwen35_linear_recurrent_convolution_and_hybrid_graph_execute")
            && model_clip_text_encoder_decoder_tests
                .contains("02f4e8afd51de3d86100655bdb69f8c9b673fe42dfc242d47840b74f408ad53c")
            && model_clip_text_encoder_decoder_tests.contains("linear_cache.step_index, 3")
            && model_clip_text_encoder_decoder_tests
                .contains("parameter shape does not match the decoder profile");
    let task385_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_unidirectional_decoder_execution")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some("comfy-parity-native-qwen35-decoder-exactness-foundation")
                    })
                })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "qwen35-hybrid-attention-is-closed-and-checkpoint-backed",
                            "qwen35-full-gating-and-linear-state-use-the-canonical-decoder",
                            "qwen35-fixture-proves-hybrid-cache-equivalence-and-weight-admission",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task385_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_unidirectional_decoder_execution,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-qwen35-decoder-exactness-foundation")
                && line.contains("VAL-MODEL-FAMILY-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task386_qwen_vision_has_one_retained_checkpoint_owner =
        production_source_occurrences(&sources, "pub struct NativeQwenVisionEncoder {").len() == 1
            && production_source_occurrences(&sources, "fn qwen_vision_attention(").len() == 1
            && model_clip_text_encoder_multimodal.contains("pub struct QwenVisionWeights")
            && model_clip_text_encoder_multimodal.contains("pub struct QwenVisionProjection")
            && !model_clip_text_encoder_multimodal.contains("NativeClipVision::new");
    let task386_qwen_vision_delegates_preparation_modules_attention_and_residency =
        model_clip_text_encoder_multimodal.contains("pub fn prepare_qwen_images(")
            && model_clip_text_encoder_multimodal.contains("pub fn plan_qwen_markers(")
            && model_clip_text_encoder_multimodal
                .contains("scaled_dot_product_attention_with_context(")
            && model_clip_text_encoder_multimodal.contains("GeluApproximation::Tanh")
            && model_clip_text_encoder_multimodal.contains("GeluApproximation::None")
            && model_clip_text_encoder_multimodal.contains("semantic_state_digest(")
            && model_clip_text_encoder_multimodal.contains("resident_tensor_allocations(")
            && !model_clip_text_encoder_multimodal.contains("RngStreamAddress")
            && !model_clip_text_encoder_multimodal.contains("NativeCache");
    let task386_qwen_vision_fixture_proves_family_exactness_and_rollback =
        model_clip_text_encoder_multimodal_tests
            .contains("retained_qwen_vision_executes_closed_family_graphs_and_rolls_back")
            && model_clip_text_encoder_multimodal_tests.contains("QWEN35_IMAGE_PAD_TOKEN")
            && model_clip_text_encoder_multimodal_tests.contains("QWEN35_IMAGE_MEAN")
            && model_clip_text_encoder_multimodal_tests
                .contains("16a13ffa41fb0a5f742f0e8c3b2364fa4020dd2cb9a71e1309e62d6e143cb8ed")
            && model_clip_text_encoder_multimodal_tests.contains("deepstack.len(), 3")
            && model_clip_text_encoder_multimodal_tests.contains("scratch.in_use_bytes()");
    let task386_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_text_media_projection_qwen")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text_encoder_multimodal::NativeQwenVisionEncoder")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some("comfy-parity-native-qwen-vision-projection-foundation")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "qwen-vision-retains-complete-checked-module-graph",
                            "qwen-vision-projects-positions-attention-and-exact-mergers",
                            "qwen-vision-delegates-canonical-attention-and-module-kernels",
                            "qwen-vision-binds-semantic-state-and-storage-residency",
                            "qwen-vision-tests-family-exactness-and-failure-atomicity",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task386_catalog_trace = ownership_catalog
        .lines()
        .find(|line| line.starts_with("native_vision_text_transformer_text_media_projection_qwen,"))
        .is_some_and(|line| {
            line.contains("comfy-parity-native-qwen-vision-projection-foundation")
                && line.contains("VAL-MODEL-FAMILY-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task387_qwen_resource_has_one_retained_composite =
        production_source_occurrences(&sources, "pub struct NativeQwenMultimodal {").len() == 1
            && model_clip_text_encoder_multimodal.contains("tokenizer: Arc<NativePromptTokenizer>")
            && model_clip_text_encoder_multimodal
                .contains("decoder: Arc<NativeDecoderTextEncoder>")
            && model_clip_text_encoder_multimodal.contains("vision: Arc<NativeQwenVisionEncoder>")
            && model_native_node_payload.contains("QwenMultimodalClip")
            && model_native_node_payload.contains("qwen_multimodal_resource");
    let task387_qwen_resource_closes_identity_residency_and_storage =
        model_clip_text_encoder_multimodal.contains("QWEN25_TOKENIZER_ARTIFACT_DIGEST")
            && model_clip_text_encoder_multimodal.contains("QWEN35_TOKENIZER_ARTIFACT_DIGEST")
            && model_clip_text_encoder_multimodal.contains("qwen_multimodal_decoder_configuration")
            && model_clip_text_encoder_multimodal.contains("sim.comfy.qwen-multimodal-resource.v2")
            && model_clip_text_encoder_multimodal
                .contains("shared Qwen tensor storage changed resident size")
            && nodes_stored_payload.contains("resource.qwen_multimodal_resource().is_some()")
            && !model_clip_text_encoder_multimodal.contains("RngStreamAddress")
            && !model_clip_text_encoder_multimodal.contains("NativeCache");
    let task387_qwen_resource_fixture_proves_admission_identity_and_residency =
        model_clip_text_encoder_multimodal_tests
            .contains("qwen_multimodal_resource_closes_admission_identity_and_residency")
            && model_clip_text_encoder_multimodal_tests
                .contains("NativeQwenMultimodal::reduced_fixture")
            && model_clip_text_encoder_multimodal_tests
                .contains("NativeModelPayload::qwen_multimodal_clip")
            && model_clip_text_encoder_multimodal_tests.contains("resident_tensor_allocations")
            && model_clip_text_encoder_multimodal_tests.contains("semantic_state_digest");
    let task387_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_text_media_resource_qwen")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text_encoder_multimodal::NativeQwenMultimodal")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some("comfy-parity-native-qwen-multimodal-resource-foundation")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "qwen-multimodal-resource-retains-one-tokenizer-decoder-and-vision",
                            "qwen-multimodal-resource-cross-admission-is-closed",
                            "qwen-multimodal-resource-binds-source-and-component-identities",
                            "qwen-multimodal-resource-unions-storage-residency",
                            "qwen-multimodal-resource-is-sealed-clip-payload",
                            "qwen-multimodal-resource-stored-adapter-preserves-specialization",
                            "qwen-multimodal-resource-tests-identity-residency-and-reduced-rejection",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task387_catalog_trace = ownership_catalog
        .lines()
        .find(|line| line.starts_with("native_vision_text_transformer_text_media_resource_qwen,"))
        .is_some_and(|line| {
            line.contains("comfy-parity-native-qwen-multimodal-resource-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task397_gemma_resource_has_one_retained_composite =
        production_source_occurrences(&sources, "pub struct NativeGemmaMultimodal {").len() == 1
            && model_clip_text_encoder_multimodal.contains("tokenizer: Arc<NativePromptTokenizer>")
            && model_clip_text_encoder_multimodal
                .contains("decoder: Arc<NativeDecoderTextEncoder>")
            && model_clip_text_encoder_multimodal.contains("vision: NativeGemmaVisionResource")
            && model_clip_text_encoder_multimodal
                .contains("audio: Option<Arc<NativeGemma4AudioEncoder>>")
            && model_native_node_payload.contains("GemmaMultimodalClip")
            && model_native_node_payload.contains("gemma_multimodal_resource");
    let task397_gemma_resource_closes_family_identity_residency_and_storage =
        model_clip_text_encoder_multimodal.contains("GemmaMultimodalFamily")
            && model_clip_text_encoder_multimodal.contains("pub const fn supports_audio")
            && model_clip_text_encoder_multimodal
                .contains("sim.comfy.gemma-multimodal-resource.v1")
            && model_clip_text_encoder_multimodal
                .contains("Gemma4 aliased tensor storage has inconsistent residency")
            && nodes_stored_payload.contains("resource.gemma_multimodal_resource().is_some()")
            && !model_clip_text_encoder_multimodal.contains("RngStreamAddress")
            && !model_clip_text_encoder_multimodal.contains("NativeCache");
    let task397_gemma_resource_fixture_proves_family_identity_residency_and_reduced_rejection =
        model_clip_text_encoder_multimodal_tests.contains(
            "gemma_multimodal_resource_closes_family_identity_residency_and_payload_admission",
        ) && model_clip_text_encoder_multimodal_tests
            .contains("NativeGemmaMultimodal::reduced_gemma4_fixture")
            && model_clip_text_encoder_multimodal_tests
                .contains("NativeModelPayload::gemma_multimodal_clip")
            && model_clip_text_encoder_multimodal_tests.contains("resident_tensor_allocations")
            && model_clip_text_encoder_multimodal_tests.contains("semantic_state_digest");
    let task397_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_text_media_resource_specialized_gemma")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text_encoder_multimodal::NativeGemmaMultimodal")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some("comfy-parity-native-gemma-multimodal-resource-foundation")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "gemma-multimodal-resource-retains-tokenizer-decoder-vision-and-optional-audio",
                            "gemma-multimodal-resource-cross-admission-is-closed",
                            "gemma-multimodal-resource-binds-source-and-component-identities",
                            "gemma-multimodal-resource-unions-size-consistent-storage-residency",
                            "gemma-multimodal-resource-is-sealed-clip-payload",
                            "gemma-multimodal-resource-stored-adapter-preserves-specialization",
                            "gemma-multimodal-resource-tests-family-identity-residency-and-reduced-rejection",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task397_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with(
                "native_vision_text_transformer_text_media_resource_specialized_gemma,",
            )
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-gemma-multimodal-resource-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task388_qwen_generation_has_one_model_domain_adapter =
        production_source_occurrences(&sources, "pub struct QwenMultimodalGenerationRequest<'a> {")
            .len()
            == 1
            && model_clip_text_encoder_multimodal.contains("pub fn generate(")
            && model_clip_text_encoder_multimodal.contains("plan_qwen_markers(")
            && model_clip_text_encoder_multimodal.contains("join_multimodal_embeddings(")
            && model_clip_text_encoder_multimodal.contains("generate_prepared(")
            && model_clip_text_encoder_multimodal.contains("finish_prepared_generation(")
            && !model_clip_text_encoder_multimodal.contains("RngStreamAddress")
            && !model_clip_text_encoder_multimodal.contains("NativeCache");
    let task388_qwen_generation_preserves_source_routes_and_atomicity =
        model_clip_text_encoder_multimodal.contains("sampling_history: &[]")
            && model_clip_text_encoder_multimodal.contains("attention_mask: None")
            && model_clip_text_encoder_multimodal
                .contains("Qwen3.5 generation cannot admit deepstack inputs")
            && model_clip_text_encoder_multimodal.contains("qwen2vl_mrope_position_ids(")
            && model_clip_text_encoder_multimodal_tests.contains(
                "qwen_multimodal_generation_replaces_markers_and_delegates_transactionally",
            )
            && model_clip_text_encoder_multimodal_tests
                .contains("qwen_multimodal/generation/manifest.json")
            && model_clip_text_encoder_multimodal_tests.contains("transaction.checkpoint()")
            && model_clip_text_encoder_multimodal_tests.contains("scratch.in_use_bytes()");
    let task388_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_text_media_generation_qwen")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text_encoder_multimodal::NativeQwenMultimodal::generate")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some("comfy-parity-native-qwen-multimodal-generation-foundation")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "qwen-generation-validates-and-plans-before-projection",
                            "qwen-generation-replaces-every-marker-span-with-real-projections",
                            "qwen-generation-scopes-mrope-and-deepstack-to-source-route",
                            "qwen-generation-delegates-empty-history-and-borrowed-transaction",
                            "qwen-generation-tests-marker-replacement-and-failure-atomicity",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task388_catalog_trace = ownership_catalog
        .lines()
        .find(|line| line.starts_with("native_vision_text_transformer_text_media_generation_qwen,"))
        .is_some_and(|line| {
            line.contains("comfy-parity-native-qwen-multimodal-generation-foundation")
                && line.contains("VAL-RNG-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task381p_qwen_preparation_has_one_attempt_local_owner =
        production_source_occurrences(&sources, "pub struct Qwen3VlPreparedImage {").len() == 1
            && production_source_occurrences(&sources, "pub struct Qwen3VlMarkerPlan {").len() == 1
            && production_source_occurrences(&sources, "pub fn prepare_qwen3vl_images(").len() == 1
            && production_source_occurrences(&sources, "pub fn plan_qwen3vl_markers(").len() == 1
            && model_clip_text_encoder_multimodal.contains("backend.workspace_vec(context")
            && model_clip_text_encoder_multimodal.contains("ResizeMode::Bilinear")
            && !model_clip_text_encoder_multimodal.contains("RngStreamAddress")
            && !model_clip_text_encoder_multimodal.contains("NativeCache")
            && !model_clip_text_encoder_multimodal.contains("OutputTransaction");
    let task381p_qwen_preparation_is_exact_and_executable = model_clip_text_encoder_multimodal
        .contains("QWEN3VL_IMAGE_MINIMUM_PIXELS")
        && model_clip_text_encoder_multimodal.contains("round_ties_even")
        && model_clip_text_encoder_multimodal.contains("marker_count != images.len()")
        && model_clip_text_encoder_multimodal.contains("visual_position_mask")
        && model_clip_text_encoder_multimodal_tests
            .contains("qwen3vl_resize_patch_packing_and_batch_splitting_are_source_exact")
        && model_clip_text_encoder_multimodal_tests
            .contains("qwen3vl_marker_plan_expands_real_image_spans_and_fails_closed")
        && model_clip_text_encoder_multimodal_tests
            .contains("qwen3vl_preparation_cancellation_and_oom_leave_workspace_empty");
    let task381p_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_text_media_preparation_qwen")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text_encoder_multimodal")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some("comfy-parity-native-qwen-image-preparation-foundation")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "qwen-preparation-uses-canonical-image-resize-and-workspace",
                            "qwen-preparation-packs-source-temporal-spatial-merge-order",
                            "qwen-preparation-requires-exact-marker-media-cardinality",
                            "qwen-preparation-produces-checked-three-axis-position-inputs",
                            "qwen-preparation-tests-exact-patches-spans-and-rollback",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task381p_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_text_media_preparation_qwen,")
        })
        .is_some_and(|line| {
            line.contains("comfy_model::clip_text_encoder_multimodal")
                && line.contains("comfy-parity-native-qwen-image-preparation-foundation")
                && line.contains("VAL-CANCEL-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task389p_gemma_preparation_has_one_attempt_local_owner =
        production_source_occurrences(&sources, "pub struct GemmaPreparedVisual {").len() == 1
            && production_source_occurrences(&sources, "pub enum GemmaPreparedVisualKind {").len()
                == 1
            && production_source_occurrences(&sources, "pub fn prepare_gemma3_image(").len() == 1
            && production_source_occurrences(&sources, "pub fn prepare_gemma4_visuals(").len() == 1
            && !model_clip_text_encoder_multimodal.contains("RngStreamAddress")
            && !model_clip_text_encoder_multimodal.contains("NativeCache")
            && !model_clip_text_encoder_multimodal.contains("OutputTransaction");
    let task389p_gemma_preparation_is_exact_bounded_and_executable =
        model_clip_text_encoder_multimodal.contains("GEMMA3_IMAGE_AREA_PIXELS")
            && model_clip_text_encoder_multimodal.contains("ResizeMode::Area")
            && model_clip_text_encoder_multimodal.contains("GEMMA4_VIDEO_SOURCE_FPS")
            && model_clip_text_encoder_multimodal.contains("InterpolateMode::Bicubic")
            && model_clip_text_encoder_multimodal.contains("antialias: true")
            && model_clip_text_encoder_multimodal_tests.contains(
                "gemma_image_video_preparation_is_source_exact_bounded_and_transactional",
            )
            && model_clip_text_encoder_multimodal_tests.contains("Gemma4VideoFrame")
            && model_clip_text_encoder_multimodal_tests.contains("scratch.in_use_bytes()");
    let task389p_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_text_media_preparation_source_gemma")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text_encoder_multimodal::GemmaPreparedVisual")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some(
                                    "comfy-parity-native-gemma-image-video-preparation-foundation",
                                )
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "gemma-preparation-projects-gemma3-source-area-with-canonical-resize",
                            "gemma-preparation-preserves-video-precedence-and-source-frame-timestamps",
                            "gemma-preparation-delegates-source-quantized-bicubic-antialias-resize",
                            "gemma-preparation-is-bounded-cancellable-and-attempt-local",
                            "gemma-preparation-tests-source-fixtures-precedence-and-rollback",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task389p_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_text_media_preparation_source_gemma,")
        })
        .is_some_and(|line| {
            line.contains("comfy_model::clip_text_encoder_multimodal::GemmaPreparedVisual")
                && line.contains("comfy-parity-native-gemma-image-video-preparation-foundation")
                && line.contains("VAL-TENSOR-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task390p_gemma_audio_has_one_attempt_local_owner =
        production_source_occurrences(&sources, "pub struct GemmaPreparedAudio {").len() == 1
            && production_source_occurrences(&sources, "pub fn prepare_gemma4_audio(").len() == 1
            && production_source_occurrences(&sources, "pub fn gemma4_audio_marker_tokens(").len()
                == 1
            && !model_clip_text_encoder_multimodal.contains("NativeAudioEncoder")
            && !model_clip_text_encoder_multimodal.contains("RngStreamAddress")
            && !model_clip_text_encoder_multimodal.contains("NativeCache")
            && !model_clip_text_encoder_multimodal.contains("OutputTransaction");
    let task390p_gemma_audio_is_exact_bounded_and_executable = model_clip_text_encoder_multimodal
        .contains("GEMMA4_AUDIO_SAMPLE_RATE")
        && model_clip_text_encoder_multimodal.contains("GEMMA4_AUDIO_KAISER_BETA")
        && model_clip_text_encoder_multimodal.contains("GEMMA4_AUDIO_FRAME_LENGTH")
        && model_clip_text_encoder_multimodal.contains("fftn_with_context_exact_native")
        && model_clip_text_encoder_multimodal.contains("gemma4_mel_filterbank")
        && model_clip_text_encoder_multimodal_tests
            .contains("gemma_audio_preparation_is_source_exact_bounded_and_transactional")
        && model_clip_text_encoder_multimodal_tests.contains("resampled_44k1_sine")
        && model_clip_text_encoder_multimodal_tests.contains("scratch.in_use_bytes()");
    let task390p_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some(
                    "native_vision_text_transformer_text_media_preparation_source_gemma_audio",
                )
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text_encoder_multimodal::GemmaPreparedAudio")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some(
                                    "comfy-parity-native-gemma-audio-preparation-foundation",
                                )
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "gemma-audio-preparation-mixes-and-resamples-source-exactly",
                            "gemma-audio-preparation-owns-semicausal-log-mel-and-mask-projection",
                            "gemma-audio-preparation-bounds-post-subsample-marker-count",
                            "gemma-audio-preparation-is-attempt-local-cancellable-and-memory-authorized",
                            "gemma-audio-preparation-tests-source-fixtures-and-rollback",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task390p_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with(
                "native_vision_text_transformer_text_media_preparation_source_gemma_audio,",
            )
        })
        .is_some_and(|line| {
            line.contains("comfy_model::clip_text_encoder_multimodal::GemmaPreparedAudio")
                && line.contains("comfy-parity-native-gemma-audio-preparation-foundation")
                && line.contains("VAL-TENSOR-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task343_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_unidirectional_decoder_execution")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_text_encoder_decoder::NativeDecoderTextEncoder")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some("comfy-parity-clip-text-encoder-decoder-foundation")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "decoder-llm-assembles-canonical-native-modules",
                            "decoder-llm-delegates-backend-admission",
                            "decoder-llm-delegates-rms-and-attention-execution",
                            "decoder-llm-consumes-caller-rng-transaction-without-opening-one",
                            "decoder-llm-delegates-token-input-to-canonical-tokenizer",
                            "decoder-llm-records-exact-profiles-and-total-symbol-behaviors",
                            "decoder-kv-state-is-invocation-local-staged-domain-state",
                            "decoder-cache-storage-delegates-canonical-tensor-memory-owner",
                            "task343-ownership-oracle-proves-decoder-foundation-reuse",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task343_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_unidirectional_decoder_execution,")
        })
        .is_some_and(|line| {
            line.contains("comfy_model::clip_text_encoder_decoder::NativeDecoderTextEncoder")
                && line.contains("comfy-parity-clip-text-encoder-decoder-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-RNG-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let task380_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_text_transformer_unidirectional_decoder_execution")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some("comfy-parity-native-prepared-decoder-generation-foundation")
                    })
                })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "prepared-decoder-prefill-delegates-one-generation-loop-and-kv-owner",
                            "prepared-decoder-prefill-executes-checked-multidimensional-rope",
                            "prepared-decoder-state-remains-invocation-local-and-borrowed",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task380_catalog_trace = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("native_vision_text_transformer_unidirectional_decoder_execution,")
        })
        .is_some_and(|line| {
            line.contains("comfy-parity-native-prepared-decoder-generation-foundation")
                && line.contains("VAL-RNG-001")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let native_clip_vision_definitions =
        production_source_occurrences(&sources, "pub struct NativeClipVision {");
    let clip_preprocess_definitions =
        production_source_occurrences(&sources, "pub fn clip_preprocess_with_context(");
    let siglip2_preprocess_definitions =
        production_source_occurrences(&sources, "pub fn siglip2_preprocess_with_context(");
    let siglip2_flex_resolution_definitions =
        production_source_occurrences(&sources, "pub fn siglip2_flex_resolution(");
    let task_340_clip_vision_has_one_architecture_and_preprocess_owner =
        native_clip_vision_definitions.len() == 1
            && native_clip_vision_definitions[0].contains("crates/comfy_model/src/clip_vision.rs")
            && clip_preprocess_definitions.len() == 1
            && clip_preprocess_definitions[0].contains("crates/comfy_model/src/clip_vision.rs")
            && siglip2_preprocess_definitions.len() == 1
            && siglip2_preprocess_definitions[0].contains("crates/comfy_model/src/clip_vision.rs")
            && siglip2_flex_resolution_definitions.len() == 1
            && siglip2_flex_resolution_definitions[0]
                .contains("crates/comfy_model/src/clip_vision.rs")
            && !model_vision.contains("NativeClipVision")
            && !model_clip_vision_production.contains("NativeEfficientNetV2S")
            && !model_clip_vision_production.contains("NativeRaftLarge");
    let task_340_clip_vision_delegates_canonical_mechanics = model_clip_vision_production
        .contains("attention: NativeModule")
        && model_clip_vision_production.contains("patch_embedding: NativeModule")
        && model_clip_vision_production.contains("NativeModule::multihead_attention")
        && model_clip_vision_production.contains("resize_with_context_exact_native(")
        && model_clip_vision_production.contains("normalize_with_context_exact_native(")
        && model_clip_vision_production.contains(".admit_backend_target(")
        && model_clip_vision_production.contains("try_reserve_exact(self.layers.len())")
        && model_clip_vision_production
            .contains("input exceeds the configured maximum patch count")
        && !model_clip_vision_production.contains("ArtifactIndex")
        && !model_clip_vision_production.contains("ModelStore")
        && !model_clip_vision_production.contains("CpuWorkspaceAuthority")
        && !model_clip_vision_production.contains("pub struct CancellationToken");
    let task_340_clip_vision_adapter_semantics_are_executable = model_clip_vision_tests
        .contains("val_clip_001_vision_rows_execute_and_extend_cumulative_ledger")
        && model_clip_vision_tests
            .contains("standard_preprocess_truncates_alpha_quantizes_and_normalizes")
        && model_clip_vision_tests
            .contains("siglip2_position_resize_adds_nonzero_source_ordered_embeddings")
        && model_clip_vision_tests.contains(
            "clip_embeddings_pool_projection_intermediates_and_llava_match_source_shapes",
        )
        && model_clip_vision_tests
            .contains("unsupported_dtypes_and_devices_fail_typed_without_relabel_or_substitution")
        && model_clip_vision_tests
            .contains("cancellation_and_workspace_oom_publish_nothing_and_converge");
    let task_340_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_vision_transformer_architecture_execution")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_model::clip_vision::NativeClipVision")
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str() == Some("comfy-parity-clip-vision-foundation")
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            "clip-vision-assembles-canonical-native-modules",
                            "clip-vision-preprocess-delegates-tensor-owners",
                            "clip-vision-admission-uses-actual-backend",
                            "clip-vision-bounds-patches-and-intermediate-capture",
                        ]
                        .iter()
                        .all(|required| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*required)
                            })
                        })
                    })
        });
    let task_340_catalog_trace = ownership_catalog
        .lines()
        .find(|line| line.starts_with("native_vision_transformer_architecture_execution,"))
        .is_some_and(|line| {
            line.contains("comfy_model::clip_vision::NativeClipVision")
                && line.contains("comfy-parity-clip-vision-foundation")
                && line.contains("VAL-CLIP-001")
                && line.contains("VAL-OWNERSHIP-001")
                && (line.contains("consolidation_required")
                    || line.contains("authoritative_owner_confirmed"))
        });
    let task_39_policy_mappings: [(&str, &[&str]); 3] = [
        (
            "native_attempt_memory_policy",
            &[
                "task39-attempt-controller-exposes-planned-workspace-ceiling",
                "task39-worker-binds-planned-ceiling-to-backend-authorization",
                "task39-retry-test-preserves-the-planner-owned-ceiling",
            ],
        ),
        (
            "tensor_backend_allocation_and_cache",
            &[
                "task39-backend-neutral-helper-charges-workspace-to-both-authoritative-counters",
                "task39-workspace-vector-retains-the-backend-lease",
                "task39-ownership-test-executes-the-workspace-chain",
            ],
        ),
        (
            "tensor_execution_context",
            &[
                "task39-execution-context-binds-the-sealed-workspace-authorization",
                "task39-scratch-authorization-bounds-live-workspace-bytes",
            ],
        ),
    ];
    let task_39_policy_trace =
        task_39_policy_mappings
            .iter()
            .all(|(concern_name, required_mapping_names)| {
                policy_concerns
                    .iter()
                    .find(|entry| {
                        entry.get("concern").and_then(serde_json::Value::as_str)
                            == Some(*concern_name)
                    })
                    .is_some_and(|entry| {
                        let tasks = entry
                            .get("consolidation_tasks")
                            .and_then(serde_json::Value::as_array);
                        let validations = entry
                            .get("validation")
                            .and_then(serde_json::Value::as_array);
                        let mappings = entry
                            .get("required_mappings")
                            .and_then(serde_json::Value::as_array);
                        tasks.is_some_and(|tasks| {
                            tasks.iter().any(|task| {
                                task.as_str()
                                    == Some(
                                        "comfy-parity-tensor-workspace-accounting-consolidation",
                                    )
                            })
                        }) && ["VAL-MEMORY-001", "VAL-OWNERSHIP-001"]
                            .into_iter()
                            .all(|required| {
                                validations.is_some_and(|validations| {
                                    validations
                                        .iter()
                                        .any(|validation| validation.as_str() == Some(required))
                                })
                            })
                            && required_mapping_names.iter().all(|required| {
                                mappings.is_some_and(|mappings| {
                                    mappings.iter().any(|mapping| {
                                        mapping.get("name").and_then(serde_json::Value::as_str)
                                            == Some(*required)
                                    })
                                })
                            })
                    })
            });
    let task_statuses = task_completion_statuses(&task_source)?;
    let corex_future_task_statuses = task_completion_statuses(&corex_future_task_source)?;
    let accounted_pending_ownership_rows =
        accounted_pending_ownership_rows(&ownership_catalog, policy_concerns, &task_statuses);
    let ownership_catalog_has_only_accounted_pending_rows =
        accounted_pending_ownership_rows.is_ok();
    let task_67_mapping_names = [
        (
            "native_audio_dsp_foundations",
            "task67-audio-dsp-shares-recurrence-resample-and-spectral-owners",
        ),
        (
            "native_image_to_tensor_conversion",
            "task67-native-image-conversion-is-bounded-and-python-free",
        ),
        (
            "native_roi_align_traversal",
            "task67-roi-forward-vjp-jvp-share-one-plan",
        ),
        (
            "tensor_flat_morphology_engine",
            "task67-morphology-compositions-share-one-primitive",
        ),
        (
            "tensor_rearrangement_traversal",
            "task67-rearrange-plan-validates-one-to-one-mapping",
        ),
        (
            "tensor_resize_sampling",
            "task67-resize-adapter-delegates-canonical-backend",
        ),
    ];
    let task_67_policy_mappings_are_declared =
        task_67_mapping_names.iter().all(|(concern, mapping_name)| {
            policy_concerns
                .iter()
                .find(|entry| {
                    entry.get("concern").and_then(serde_json::Value::as_str) == Some(*concern)
                })
                .and_then(|entry| entry.get("required_mappings"))
                .and_then(serde_json::Value::as_array)
                .is_some_and(|mappings| {
                    mappings.iter().any(|mapping| {
                        mapping.get("name").and_then(serde_json::Value::as_str)
                            == Some(*mapping_name)
                    })
                })
        });
    let task_68_mapping_names = [
        (
            "tensor_checked_bilinear_sampling",
            "raft-grid-sampling-delegates-checked-bilinear-owner",
        ),
        (
            "tensor_deformable_sampling",
            "deform-forward-vjp-jvp-share-checked-bilinear-owner",
        ),
        (
            "native_rgb8_tensor_conversion",
            "rgb8-boundary-retains-canonical-tensor-storage",
        ),
        (
            "native_rgb8_tensor_conversion",
            "task68-topil-delegates-rgb8-owner",
        ),
        (
            "native_vision_model_architectures",
            "vision-model-state-publication-delegates-native-module",
        ),
        (
            "native_vision_model_architectures",
            "vision-model-normalization-delegates-functional-owner",
        ),
        (
            "native_vision_model_architectures",
            "raft-grid-sampling-delegates-tensor-bilinear-owner",
        ),
        (
            "native_color_conversion_traversal",
            "color-forward-and-analytical-maps-share-one-traversal",
        ),
    ];
    let task_68_policy_mappings_are_declared =
        task_68_mapping_names.iter().all(|(concern, mapping_name)| {
            policy_concerns
                .iter()
                .find(|entry| {
                    entry.get("concern").and_then(serde_json::Value::as_str) == Some(*concern)
                })
                .and_then(|entry| entry.get("required_mappings"))
                .and_then(serde_json::Value::as_array)
                .is_some_and(|mappings| {
                    mappings.iter().any(|mapping| {
                        mapping.get("name").and_then(serde_json::Value::as_str)
                            == Some(*mapping_name)
                    })
                })
        });
    let task_69_mapping_names = [
        (
            "native_audio_dsp_foundations",
            "task69-bass-and-melscale-delegate-canonical-audio-owners",
        ),
        (
            "tensor_flat_morphology_engine",
            "task69-bottom-hat-selects-canonical-morphology-owner",
        ),
        (
            "native_rgb8_tensor_conversion",
            "task69-totensor-borrows-canonical-rgb8-boundary",
        ),
        (
            "native_color_conversion_traversal",
            "task69-lab-inverse-delegates-canonical-color-traversal",
        ),
        (
            "native_box_coordinate_conversion",
            "task69-box-forward-and-analytical-maps-share-one-coordinate-owner",
        ),
        (
            "native_tensor_transform_composition",
            "task69-compose-orders-only-and-delegates-normalization",
        ),
    ];
    let task_69_policy_mappings_are_declared =
        task_69_mapping_names.iter().all(|(concern, mapping_name)| {
            policy_concerns
                .iter()
                .find(|entry| {
                    entry.get("concern").and_then(serde_json::Value::as_str) == Some(*concern)
                })
                .and_then(|entry| entry.get("required_mappings"))
                .and_then(serde_json::Value::as_array)
                .is_some_and(|mappings| {
                    mappings.iter().any(|mapping| {
                        mapping.get("name").and_then(serde_json::Value::as_str)
                            == Some(*mapping_name)
                    })
                })
        });
    let task_67_external_kernel_foundations_have_one_owner = [
        "pub fn rearrange_with_context_exact_native(",
        "pub fn native_morphology_with_context_exact(",
        "pub fn biquad_with_context_exact_native(",
        "pub fn resample_with_context_exact_native(",
        "fn mel_filter_bank(",
        "pub fn roi_align_with_context_exact_native(",
    ]
    .iter()
    .all(|symbol| production_source_occurrences(&sources, symbol).len() == 1)
        && tensor_external_kernel_part_one
            .matches("pub fn resize_with_context_exact_native(")
            .count()
            == 1
        && tensor_external_kernel_part_one
            .matches("pub fn normalize_with_context_exact_native(")
            .count()
            == 1
        && tensor_external_kernel_part_one
            .matches("pub fn to_tensor_with_context_exact_native(")
            .count()
            == 1
        && tensor_external_kernel_part_three.contains(
            "to_tensor_with_context_exact_native as image_bytes_to_tensor_with_context_exact_native",
        )
        && tensor_external_kernel_part_three
            .contains("image_bytes_to_tensor_with_context_exact_native(")
        && !tensor_external_kernel_part_one.contains("context.check()?")
        && tensor_external_kernel_part_one
            .matches("context.cancellation.check()?")
            .count()
            >= 23;
    let tensor_index_foundation_concerns = [
        "tensor_broadcast_geometry",
        "tensor_decoded_scalar_encoding",
        "tensor_narrow_view_geometry",
        "tensor_scalar_truth_predicate",
    ];
    let tensor_indexing_part_one_concerns = [
        "tensor_conditional_selection",
        "tensor_gather_scatter_index_plan",
        "tensor_nonzero_projection",
    ];
    let gather_scatter_plan_definitions = source_occurrences(&sources, "struct GatherScatterPlan");
    let conditional_selection_definitions =
        source_occurrences(&sources, "pub fn where_with_context_exact_native(");
    let nonzero_projection_definitions =
        source_occurrences(&sources, "pub fn nonzero_with_context_exact_native(");
    let masked_fill_method_adapter_definitions = source_occurrences(
        &sources,
        "pub fn masked_fill_method_with_context_exact_native(",
    );
    let nonzero_method_adapter_definitions =
        source_occurrences(&sources, "pub fn nonzero_method_with_context_exact_native(");
    let tensor_index_foundation_policy_is_declared =
        tensor_index_foundation_concerns.iter().all(|concern| {
            policy_concerns
                .iter()
                .find(|entry| {
                    entry.get("concern").and_then(serde_json::Value::as_str) == Some(*concern)
                })
                .is_some_and(|entry| {
                    entry
                        .get("consolidation_tasks")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|tasks| {
                            tasks.iter().any(|task| {
                                task.as_str()
                                    == Some("comfy-parity-tensor-index-ownership-consolidation")
                            })
                        })
                        && entry
                            .get("known_open_reasons")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(Vec::is_empty)
                })
        });
    let tensor_indexing_part_one_policy_is_declared =
        tensor_indexing_part_one_concerns.iter().all(|concern| {
            policy_concerns
                .iter()
                .find(|entry| {
                    entry.get("concern").and_then(serde_json::Value::as_str) == Some(*concern)
                })
                .is_some_and(|entry| {
                    entry
                        .get("consolidation_tasks")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|tasks| {
                            tasks.iter().any(|task| {
                                task.as_str() == Some(
                                    "comfy-parity-tensor-ops-indexing-masking-comfy-tensor-op-006e05c5daaf",
                                )
                            })
                        })
                })
        });
    let tensor_indexing_part_two_policy_is_declared =
        ["tensor_conditional_selection", "tensor_nonzero_projection"]
            .iter()
            .all(|concern| {
            policy_concerns
                .iter()
                .find(|entry| {
                    entry.get("concern").and_then(serde_json::Value::as_str) == Some(*concern)
                })
                .is_some_and(|entry| {
                    entry
                        .get("consolidation_tasks")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|tasks| {
                            tasks.iter().any(|task| {
                                task.as_str() == Some(
                                    "comfy-parity-tensor-ops-indexing-masking-comfy-tensor-op-e9a313720d5d",
                                )
                            })
                        })
                })
            });
    let tensor_index_foundation_catalog_is_declared =
        tensor_index_foundation_concerns.iter().all(|concern| {
            ownership_catalog
                .lines()
                .find(|line| line.starts_with(&format!("{concern},")))
                .is_some_and(|line| {
                    line.contains("comfy-parity-tensor-index-ownership-consolidation")
                        && line.contains("VAL-OWNERSHIP-001")
                })
        });
    let tensor_linear_algebra_policy_is_declared = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("workspace_tensor_linear_algebra_part_one_mechanics")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some(
                                "comfy-parity-tensor-ops-linear-algebra-comfy-tensor-op-061170cbb6f7",
                            )
                    })
                })
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
        });
    let tensor_linear_algebra_catalog_is_declared = ownership_catalog
        .lines()
        .find(|line| line.starts_with("workspace_tensor_linear_algebra_part_one_mechanics,"))
        .is_some_and(|line| {
            line.contains("comfy-parity-tensor-ops-linear-algebra-comfy-tensor-op-061170cbb6f7")
                && line.contains("VAL-OWNERSHIP-001")
                && line.contains("authoritative_owner_confirmed")
        });
    let tensor_linear_algebra_owner_symbols = [
        "struct RectangularMatrixBatch",
        "struct SquareMatrixBatch",
        "struct SolveGeometry",
        "struct EinsumPlan",
        "struct WorkspaceLuFactor",
        "pub fn einsum_with_context_exact_native(",
        "pub fn tensordot_with_context_exact_native(",
        "pub fn determinant_with_context_exact_native(",
        "pub fn inverse_with_context_exact_native(",
        "pub fn solve_with_context_exact_native(",
        "pub fn qr_with_context_exact_native(",
        "pub fn eigh_with_context_exact_native(",
        "pub fn matmul_with_context_exact_native(",
        "pub fn mm_with_context_exact_native(",
        "pub fn vector_norm_with_context_exact_native(",
        "pub fn transpose_last_two_with_context_exact_native(",
        "pub fn symmetric_eigen_decomposition_with_context(",
    ];
    let tensor_linear_algebra_has_one_owner = tensor_linear_algebra_owner_symbols
        .iter()
        .all(|symbol| production_source_occurrences(&sources, symbol).len() == 1)
        && tensor_linear_algebra_part_one
            .contains("cross_with_context_exact_native as canonical_cross_with_context")
        && tensor_linear_algebra_part_one
            .contains("cross_vjp_with_context_exact_native as canonical_cross_vjp_with_context")
        && tensor_linear_algebra_part_one
            .contains("cross_jvp_with_context_exact_native as canonical_cross_jvp_with_context")
        && tensor_linear_algebra_part_two.contains("symmetric_eigen_decomposition_with_context")
        && tensor_linear_algebra_part_two.contains("transpose_last_two_with_context_exact_native")
        && !tensor_linear_algebra_part_two.contains("struct EinsumPlan")
        && !tensor_linear_algebra_part_two.contains("struct WorkspaceLuFactor")
        && !tensor_linear_algebra_part_two.contains("fn symmetric_eigen_decomposition_into")
        && !tensor_linear_algebra_part_one.contains("context.check()?")
        && tensor_linear_algebra_part_one
            .matches("context.cancellation.check()?")
            .count()
            >= 31;
    let tensor_linear_algebra_part_two_policy_is_declared = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("workspace_tensor_linear_algebra_part_two_mechanics")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some(
                                "comfy-parity-tensor-ops-linear-algebra-comfy-tensor-op-a5d623c79a18",
                            )
                    })
                })
        });
    let tensor_linear_algebra_part_two_catalog_is_declared = ownership_catalog
        .lines()
        .find(|line| line.starts_with("workspace_tensor_linear_algebra_part_two_mechanics,"))
        .is_some_and(|line| {
            line.contains("comfy-parity-tensor-ops-linear-algebra-comfy-tensor-op-a5d623c79a18")
                && line.contains("authoritative_owner_confirmed")
        });
    let tensor_linear_algebra_part_two_has_one_owner = [
        "struct SvdGeometry",
        "struct WorkspaceMatrixSvd",
        "struct WorkspaceReducedSvdFactors",
        "pub fn bmm_with_context_exact_native(",
        "pub fn svd_with_context_exact_native(",
        "pub fn svd_jvp_with_context_exact_native(",
        "pub fn svd_vjp_with_context_exact_native(",
    ]
    .iter()
    .all(|symbol| production_source_occurrences(&sources, symbol).len() == 1)
        && tensor_linear_algebra_part_two.contains("LinearAlgebraOperation::BatchMatrixMultiply")
        && tensor_linear_algebra_part_two.contains("vector_norm_with_context_exact_native(")
        && tensor_linear_algebra_part_two.contains("transpose_last_two_with_context_exact_native(")
        && tensor_linear_algebra_part_two.contains("symmetric_eigen_decomposition_with_context(")
        && !tensor_linear_algebra_part_two.contains("context.check()?")
        && tensor_linear_algebra_part_two
            .matches("context.cancellation.check()?")
            .count()
            >= 10;
    let tensor_neural_network_functional_policy_is_declared = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("workspace_tensor_neural_network_functional_part_one_mechanics")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some(
                                "comfy-parity-tensor-ops-neural-network-functional-comfy-tensor-op-13df18f5f426",
                            )
                    })
                })
        });
    let tensor_neural_network_functional_catalog_is_declared = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("workspace_tensor_neural_network_functional_part_one_mechanics,")
        })
        .is_some_and(|line| {
            line.contains(
                "comfy-parity-tensor-ops-neural-network-functional-comfy-tensor-op-13df18f5f426",
            ) && line.contains("authoritative_owner_confirmed")
        });
    let tensor_neural_network_functional_has_one_owner = [
        "pub fn cosine_similarity_with_context_exact_native(",
        "pub fn embedding_with_context_exact_native(",
        "pub fn unfold_with_context_exact_native(",
        "pub fn fold_with_context_exact_native(",
        "pub fn glu_with_context_exact_native(",
        "pub fn one_hot_with_context_exact_native(",
        "pub fn softplus_with_context_exact_native(",
    ]
    .iter()
    .all(|symbol| production_source_occurrences(&sources, symbol).len() == 1)
        && tensor_neural_network_functional_part_one.contains("canonical_linear")
        && tensor_neural_network_functional_part_one.contains("canonical_attention_with_context")
        && tensor_neural_network_functional_part_one.contains("canonical_sigmoid_with_context")
        && tensor_neural_network_functional_part_one
            .contains("canonical_index_select_with_context")
        && tensor_neural_network_functional_part_one.contains("canonical_rearrange_with_context")
        && !tensor_neural_network_functional_part_one.contains("context.check()?")
        && tensor_neural_network_functional_part_one
            .matches("context.cancellation.check()?")
            .count()
            >= 29
        && tensor_neural_network_functional_part_one_tests
            .matches("assert_cancelled(")
            .count()
            == 34;
    let tensor_neural_network_module_policy_is_declared = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("workspace_tensor_neural_network_module_part_one_local_mechanics")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some(
                                "comfy-parity-tensor-ops-neural-network-module-comfy-tensor-op-0e602e58360a",
                            )
                    })
                })
        });
    let tensor_neural_network_module_catalog_is_declared = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with("workspace_tensor_neural_network_module_part_one_local_mechanics,")
        })
        .is_some_and(|line| {
            line.contains(
                "comfy-parity-tensor-ops-neural-network-module-comfy-tensor-op-0e602e58360a",
            ) && line.contains("authoritative_owner_confirmed")
        });
    let tensor_neural_network_module_has_one_owner =
        [
            "pub fn average_pool_2d_with_context_exact_native(",
            "pub fn prelu_with_context_exact_native(",
            "pub fn smooth_l1_loss_with_context_exact_native(",
        ]
        .iter()
        .all(|symbol| {
            let occurrences = production_source_occurrences(&sources, symbol);
            if *symbol == "pub fn average_pool_2d_with_context_exact_native(" {
                occurrences.len() == 2
                    && occurrences.iter().any(|path| {
                        path.contains("crates/comfy_tensor/src/ops/neural_network_module_01.rs")
                    })
                    && occurrences.iter().any(|path| {
                        path.contains("crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs")
                    })
            } else {
                occurrences.len() == 1
            }
        }) && production_source_occurrences(&sources, "pub fn resize_vjp(").len() == 1
            && tensor_neural_network_module_part_one.contains("canonical_convolution")
            && tensor_neural_network_module_part_one.contains("canonical_group_norm")
            && tensor_neural_network_module_part_one.contains("canonical_layer_norm")
            && tensor_neural_network_module_part_one.contains("canonical_silu")
            && tensor_neural_network_module_part_one.contains("canonical_softmax")
            && tensor_neural_network_module_part_one.contains("canonical_tanh")
            && tensor_neural_network_module_part_one.contains("canonical_resize")
            && tensor_neural_network_module_part_one.contains("backend.resize_vjp(")
            && !tensor_neural_network_module_part_one.contains("struct NativeModule")
            && model_native_ops.contains("let mut staged_children = self.children.clone()")
            && model_native_ops.contains("self.children = staged_children")
            && tensor_neural_network_module_part_one_tests
                .contains("all_twelve_resolutions_are_unique_and_runtime_hash_sealed")
            && tensor_neural_network_module_part_one_tests
                .contains("tanh_and_upsample_delegate_tensor_owners_including_aligned_coordinates")
            && tensor_neural_network_module_part_one_tests
                .matches("assert_cancelled(")
                .count()
                == 30;
    let tensor_neural_network_module_part_three_policy_is_declared = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("workspace_tensor_neural_network_module_part_two_max_selection_and_relu6")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some(
                                "comfy-parity-tensor-ops-neural-network-module-comfy-tensor-op-904c1e14bae4",
                            )
                    })
                })
        });
    let tensor_neural_network_module_part_three_catalog_is_declared = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with(
                "workspace_tensor_neural_network_module_part_two_max_selection_and_relu6,",
            )
        })
        .is_some_and(|line| {
            line.contains(
                "comfy-parity-tensor-ops-neural-network-module-comfy-tensor-op-904c1e14bae4",
            ) && line.contains("authoritative_owner_confirmed")
        });
    let tensor_neural_network_module_part_three_has_one_owner = [
        "pub fn max_pool_2d_with_context_exact_native(",
        "pub fn relu_6_with_context_exact_native(",
    ]
    .iter()
    .all(|symbol| {
        let occurrences = production_source_occurrences(&sources, symbol);
        if *symbol == "pub fn max_pool_2d_with_context_exact_native(" {
            occurrences.len() == 2
                && occurrences.iter().any(|path| {
                    path.contains("crates/comfy_tensor/src/ops/neural_network_module_03.rs")
                })
                && occurrences.iter().any(|path| {
                    path.contains("crates/comfy_tensor/src/ops/spatial_functional_kernel_01.rs")
                })
        } else {
            occurrences.len() == 1
        }
    })
        && tensor_neural_network_module_part_three.contains("canonical_convolution")
        && tensor_neural_network_module_part_three.contains("canonical_gelu")
        && tensor_neural_network_module_part_three.contains("canonical_relu")
        && tensor_neural_network_module_part_three.contains("canonical_smooth_l1")
        && tensor_neural_network_module_part_three.contains("canonical_requires_grad")
        && tensor_neural_network_module_part_three.contains("canonical_pixel_shuffle")
        && tensor_neural_network_module_part_three.contains("AveragePoolGeometry::new_extended")
        && tensor_neural_network_module_part_three.contains("Pad2dGeometry::new")
        && tensor_neural_network_module_part_two.contains("pub(crate) struct Pad2dGeometry")
        && !tensor_neural_network_module_part_three.contains("struct NativeModule")
        && !tensor_neural_network_module_part_three.contains("struct AutogradTape")
        && model_native_ops.contains("NativeModuleSpec::ModuleDict")
        && model_native_ops.contains("NativeModuleSpec::ModuleList")
        && model_native_ops.contains("pub fn child_at(&self, index: usize)")
        && tensor_neural_network_module_part_three_tests
            .contains("all_twelve_part_three_resolutions_are_unique_and_runtime_hash_sealed")
        && tensor_neural_network_module_part_three_tests
            .contains("parameter_uses_the_caller_owned_autograd_tape");
    let tensor_spatial_functional_kernel_policy_is_declared = [
        "linear_convolution_kernel_mechanics",
        "tensor_checked_bilinear_sampling",
        "tensor_resize_sampling",
        "workspace_tensor_neural_network_module_part_one_local_mechanics",
        "workspace_tensor_neural_network_module_part_two_max_selection_and_relu6",
    ]
    .iter()
    .all(|concern| {
        policy_concerns
            .iter()
            .find(|entry| entry.get("concern").and_then(serde_json::Value::as_str) == Some(concern))
            .and_then(|entry| entry.get("consolidation_tasks"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|tasks| {
                tasks.iter().any(|task| {
                    task.as_str()
                        == Some(
                            "comfy-parity-tensor-ops-spatial-functional-kernel-comfy-tensor-op-1f9d23f3b331",
                        )
                })
            })
    });
    let tensor_spatial_functional_kernel_catalog_is_declared = [
        "linear_convolution_kernel_mechanics",
        "tensor_checked_bilinear_sampling",
        "tensor_resize_sampling",
        "workspace_tensor_neural_network_module_part_one_local_mechanics",
        "workspace_tensor_neural_network_module_part_two_max_selection_and_relu6",
    ]
    .iter()
    .all(|concern| {
        ownership_catalog
            .lines()
            .find(|line| line.starts_with(&format!("{concern},")))
            .is_some_and(|line| {
                line.contains(
                    "comfy-parity-tensor-ops-spatial-functional-kernel-comfy-tensor-op-1f9d23f3b331",
                ) && line.contains("authoritative_owner_confirmed")
            })
    });
    let tensor_spatial_functional_kernel_only_adapts_authoritative_owners =
        tensor_spatial_functional_kernel.contains("canonical_convolution")
            && tensor_spatial_functional_kernel.contains("AveragePoolGeometry::new_extended")
            && tensor_spatial_functional_kernel.contains("canonical_max_pool_2d")
            && tensor_spatial_functional_kernel.contains("checked_bilinear_weights")
            && tensor_spatial_functional_kernel.contains("checked_linear_weights")
            && !tensor_spatial_functional_kernel.contains("struct ConvolutionGeometry")
            && !tensor_spatial_functional_kernel.contains("struct AveragePoolGeometry")
            && !tensor_spatial_functional_kernel.contains("fn checked_bilinear_weights")
            && !tensor_spatial_functional_kernel.contains("fn checked_linear_weights")
            && !tensor_spatial_functional_kernel.contains("fn max_pool_selection")
            && tensor_spatial_functional_kernel_tests
                .contains("all_convolution_dimensions_delegate_one_geometry_owner")
            && tensor_spatial_functional_kernel_tests
                .contains("all_twelve_contracts_are_build_sealed_against_runtime_fixtures");
    let tensor_spectral_transform_policy_is_declared = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("workspace_tensor_spectral_transform_fft_kernel")
        })
        .and_then(|entry| entry.get("consolidation_tasks"))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|tasks| {
            [
                "comfy-parity-tensor-ops-elementwise-or-runtime-operation-comfy-tensor-op-861ee6173859",
                "comfy-parity-tensor-ops-spectral-transform-comfy-tensor-op-2c39e32acd3c",
            ]
            .iter()
            .all(|expected| tasks.iter().any(|task| task.as_str() == Some(expected)))
        });
    let tensor_spectral_transform_catalog_is_declared = ownership_catalog
        .lines()
        .find(|line| line.starts_with("workspace_tensor_spectral_transform_fft_kernel,"))
        .is_some_and(|line| {
            line.contains("comfy-parity-tensor-ops-spectral-transform-comfy-tensor-op-2c39e32acd3c")
                && line.contains("authoritative_owner_confirmed")
        });
    let tensor_spectral_transform_only_adapts_the_task_55_fft_owner =
        production_source_occurrences(&sources, "fn complex_fft_in_place(").len() == 1
            && tensor_operation_part_twelve
                .matches("complex_fft_in_place(")
                .count()
                == 2
            && tensor_operation_part_twelve
                .contains("complex_fft_in_place(backend, &mut frame_values, false, context)")
            && tensor_spectral_transform
                .contains("generated_elementwise_or_runtime_operation_12::complex_fft_in_place")
            && tensor_spectral_transform
                .contains("complex_fft_in_place(backend, &mut line, inverse, context)")
            && !tensor_spectral_transform.contains("consts::TAU")
            && !tensor_spectral_transform.contains("DType::decode_scalar")
            && tensor_spectral_transform.contains(".decode_scalar(")
            && tensor_spectral_transform.contains(".encode_decoded_scalar(")
            && tensor_spectral_transform.contains("backend.workspace_vec(")
            && tensor_spectral_transform.contains("backend.upload_bytes(")
            && tensor_spectral_transform_tests.contains("task_55_is_the_only_fft_kernel_owner")
            && tensor_spectral_transform_tests
                .contains("all_four_resolutions_are_unique_and_runtime_hash_sealed");
    let tensor_storage_dtype_device_policy_is_declared = [
        "tensor_storage_descriptors_and_views",
        "tensor_dtype_contracts",
        "tensor_device_identity",
        "tensor_cast_contracts",
        "tensor_backend_allocation_and_cache",
        "tensor_execution_context",
    ]
    .iter()
    .all(|concern| {
        policy_concerns
            .iter()
            .find(|entry| {
                entry.get("concern").and_then(serde_json::Value::as_str) == Some(concern)
            })
            .is_some_and(|entry| {
                entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some(
                                    "comfy-parity-tensor-ops-storage-dtype-device-comfy-tensor-op-00f639d6c8a7",
                                )
                        })
                    })
                    && entry
                        .get("required_mappings")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|mappings| {
                            mappings.iter().any(|mapping| {
                                mapping
                                    .get("name")
                                    .and_then(serde_json::Value::as_str)
                                    .is_some_and(|name| name.starts_with("task91-"))
                            })
                        })
            })
    });
    let tensor_storage_dtype_device_catalog_is_declared = [
        "tensor_storage_descriptors_and_views",
        "tensor_dtype_contracts",
        "tensor_device_identity",
        "tensor_cast_contracts",
        "tensor_backend_allocation_and_cache",
        "tensor_execution_context",
    ]
    .iter()
    .all(|concern| {
        ownership_catalog
            .lines()
            .find(|line| line.starts_with(&format!("{concern},")))
            .is_some_and(|line| {
                line.contains(
                    "comfy-parity-tensor-ops-storage-dtype-device-comfy-tensor-op-00f639d6c8a7",
                ) && line.contains("authoritative_owner_confirmed")
            })
    });
    let tensor_storage_dtype_device_only_adapts_authoritative_owners =
        source_occurrences(&sources, concat!("pub struct ", "Tensor {")).len() == 1
            && source_occurrences(&sources, concat!("struct ", "Storage {")).len() == 1
            && source_occurrences(&sources, concat!("pub enum ", "DType {")).len() == 1
            && source_occurrences(&sources, concat!("pub struct ", "DeviceId {")).len() == 1
            && source_occurrences(&sources, concat!("pub struct Cancellation", "Token")).len() == 1
            && source_occurrences(
                &sources,
                concat!("pub fn ", "cast_to_with_context_exact_native("),
            )
            .len()
                == 1
            && source_occurrences(&sources, concat!("pub struct ", "NativeArrayView")).len() == 1
            && tensor_storage_dtype_device.contains("cast_to_with_context_exact_native(")
            && tensor_storage_dtype_device.contains("descriptor_for_memory_format(")
            && tensor_storage_dtype_device.contains(".workspace_vec(")
            && tensor_storage_dtype_device.contains("destination.write()?")
            && tensor_storage_dtype_device.matches(".copy(").count() >= 4
            && tensor_storage_dtype_device.contains("host_storage_bytes()?")
            && !tensor_storage_dtype_device.contains(concat!("pub struct ", "Storage {"))
            && !tensor_storage_dtype_device.contains(concat!("pub enum ", "DType {"))
            && !tensor_storage_dtype_device.contains(concat!("pub struct ", "DeviceId {"))
            && !tensor_storage_dtype_device.contains(concat!("pub struct Cancellation", "Token"))
            && !tensor_storage_dtype_device.contains("Command::new")
            && tensor_storage_dtype_device_tests
                .contains("authoritative_owners_are_reused_without_competing_foundations")
            && tensor_storage_dtype_device_tests
                .contains("all_eleven_resolutions_are_unique_and_runtime_hash_sealed");
    let tensor_neural_network_module_part_four_policy_is_declared = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("workspace_tensor_neural_network_module_part_two_mse_and_dropout_mask")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    tasks.iter().any(|task| {
                        task.as_str()
                            == Some(
                                "comfy-parity-tensor-ops-neural-network-module-comfy-tensor-op-d60003ac2b14",
                            )
                    })
                })
        });
    let tensor_neural_network_module_part_four_catalog_is_declared = ownership_catalog
        .lines()
        .find(|line| {
            line.starts_with(
                "workspace_tensor_neural_network_module_part_two_mse_and_dropout_mask,",
            )
        })
        .is_some_and(|line| {
            line.contains(
                "comfy-parity-tensor-ops-neural-network-module-comfy-tensor-op-d60003ac2b14",
            ) && line.contains("authoritative_owner_confirmed")
        });
    let tensor_neural_network_module_part_four_has_one_owner = [
        "pub fn dropout_with_context_exact_native(",
        "pub fn mse_loss_with_context_exact_native(",
    ]
    .iter()
    .all(|symbol| production_source_occurrences(&sources, symbol).len() == 1)
        && source_occurrences(&sources, "pub struct RngStream {").len() == 1
        && source_occurrences(&sources, "pub struct RngTransaction {").len() == 1
        && source_occurrences(&sources, "pub struct NativeModule {").len() == 1
        && source_occurrences(&sources, "pub(crate) struct AveragePoolGeometry").len() == 1
        && tensor_neural_network_module_part_four.contains("AveragePoolGeometry::new(")
        && tensor_neural_network_module_part_four.contains("rng::{RngError, RngTransaction}")
        && tensor_neural_network_module_part_four.contains("canonical_elu")
        && tensor_neural_network_module_part_four.contains("canonical_sigmoid")
        && !tensor_neural_network_module_part_four.contains("struct NativeModule")
        && !tensor_neural_network_module_part_four.contains("struct RngStream")
        && !tensor_neural_network_module_part_four.contains("pub struct RngTransaction")
        && !tensor_neural_network_module_part_four.contains("struct AveragePoolGeometry")
        && model_native_ops.contains("NativeModuleSpec::Dropout")
        && model_native_ops.contains("pub fn forward_with_rng_with_context(")
        && model_native_ops.contains("transaction: result.transaction")
        && tensor_neural_network_module_part_four_tests
            .contains("dropout_replays_and_commits_only_the_canonical_rng_transaction")
        && tensor_neural_network_module_part_four_tests
            .contains("identity_and_sigmoid_preserve_canonical_tensor_semantics")
        && tensor_neural_network_module_part_four_tests
            .contains("operation_contracts_are_unique_and_evidence_is_exact");
    let tensor_random_number_generation_part_one_policy_is_declared = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_rng_stream_state")
        })
        .is_some_and(|entry| {
            entry
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| {
                    let has_part_one = tasks.iter().any(|task| {
                        task.as_str()
                            == Some(
                                "comfy-parity-tensor-ops-random-number-generation-comfy-tensor-op-095b3e192800",
                            )
                    });
                    let has_part_two = tasks.iter().any(|task| {
                        task.as_str()
                            == Some(
                                "comfy-parity-tensor-ops-random-number-generation-comfy-tensor-op-fd729b8a5363",
                            )
                    });
                    has_part_one && has_part_two
                })
        });
    let tensor_random_number_generation_part_one_catalog_is_declared = ownership_catalog
        .lines()
        .find(|line| line.starts_with("native_rng_stream_state,"))
        .is_some_and(|line| {
            line.contains(
                "comfy-parity-tensor-ops-random-number-generation-comfy-tensor-op-095b3e192800",
            ) && line.contains(
                "comfy-parity-tensor-ops-random-number-generation-comfy-tensor-op-fd729b8a5363",
            ) && line.contains("authoritative_owner_confirmed")
        });
    let tensor_random_number_generation_part_one_has_one_owner = [
        "pub struct RngStream {",
        "pub struct RngTransaction {",
        "pub struct SobolEngine {",
        "pub struct BrownianTree {",
        "pub fn next_standard_normal_pair(",
        "pub fn next_bounded_u64(",
    ]
    .iter()
    .all(|symbol| production_source_occurrences(&sources, symbol).len() == 1)
        && tensor_rng.contains("pub fn next_unit_f64(")
        && tensor_rng.contains("pub fn next_standard_normal_pair(")
        && tensor_rng.contains("pub fn next_bounded_u64(")
        && tensor_rng.contains("pub fn require_device(&self, expected: DeviceId)")
        && tensor_rng
            .split_once("#[cfg(test)]")
            .map_or(tensor_rng.as_str(), |(production, _)| production)
            .matches("RngError::DeviceMismatch")
            .count()
            == 1
        && tensor_rng.contains("pub struct SobolEngine {")
        && tensor_rng.contains("pub struct BrownianTree {")
        && tensor_rng.contains("let mut candidate = self.clone();")
        && tensor_random_number_generation_part_one.contains("transaction.next_unit_f64(")
        && tensor_random_number_generation_part_one
            .contains("transaction.next_standard_normal_pair(")
        && tensor_random_number_generation_part_one.contains("transaction.next_bounded_u64(")
        && tensor_random_number_generation_part_one.contains("SobolEngine::new(")
        && tensor_random_number_generation_part_one.contains("BrownianTree::new(")
        && tensor_random_number_generation_part_one.contains("transaction.require_device(")
        && !tensor_random_number_generation_part_one.contains("struct RngStream")
        && !tensor_random_number_generation_part_one.contains("struct RngTransaction")
        && !tensor_random_number_generation_part_one.contains("struct SobolEngine")
        && !tensor_random_number_generation_part_one.contains("struct BrownianTree")
        && sampler_noise.contains("transaction.next_standard_normal_pair(context.cancellation)")
        && sampler_noise.contains("transaction.require_device(DeviceId::CPU)")
        && !sampler_noise.contains("uniform_1.ln()")
        && tensor_operation_part_four.contains("rng.require_device(input.descriptor().device())")
        && tensor_neural_network_module_part_four.contains("transaction.require_device(device)")
        && tensor_random_number_generation_part_two
            .contains("standard_normal_tensor_with_context(")
        && tensor_random_number_generation_part_two.contains(".require_device(device)")
        && !tensor_random_number_generation_part_two.contains("next_standard_normal_pair(")
        && !tensor_random_number_generation_part_two.contains("struct RngStream")
        && !tensor_random_number_generation_part_two.contains("struct RngTransaction")
        && tensor_random_number_generation_part_two_resolution
            .contains("sim.native.rng.randn.cpu-floating-strided.v1")
        && tensor_random_number_generation_part_two_tests
            .contains("randn_replays_advances_and_reuses_the_canonical_normal_transform")
        && tensor_random_number_generation_part_two_tests
            .contains("randn_rejects_layout_dtype_and_rng_device_mismatches")
        && tensor_random_number_generation_part_two_tests
            .contains("randn_resolution_is_unique_source_profiled_and_hash_sealed")
        && tensor_random_number_generation_part_one_resolution
            .matches("source_observations")
            .count()
            == 0
        && tensor_random_number_generation_part_one_tests
            .contains("generator_facades_reseed_the_single_canonical_stream")
        && tensor_random_number_generation_part_one_tests
            .contains("initializers_are_copy_on_write_and_publish_only_after_success")
        && tensor_random_number_generation_part_one_tests
            .contains("sobol_and_brownian_state_are_deterministic_and_additive")
        && tensor_random_number_generation_part_one_tests
            .contains("cancellation_precedes_invalid_inputs_and_leaves_mutations_unchanged")
        && tensor_random_number_generation_part_one_tests
            .contains("operation_contracts_are_unique_and_evidence_is_exact");
    let reopened_mapping_names = [
        (
            "tensor_cartesian_product_traversal",
            "task66-cartesian-product-reuses-canonical-codec-access-and-publication",
        ),
        (
            "tensor_cumulative_scan_traversal",
            "task64-zero-aware-cumprod-derivatives-are-tested",
        ),
        (
            "tensor_execution_context",
            "task60-xpu-synchronize-delegates-context-and-backend-events",
        ),
        (
            "tensor_primitive_operation_semantics",
            "task57-unary-binary-and-concat-adapters-reuse-canonical-owners",
        ),
    ];
    let reopened_policy_mappings_are_declared =
        reopened_mapping_names
            .iter()
            .all(|(concern, mapping_name)| {
                policy_concerns
                    .iter()
                    .find(|entry| {
                        entry.get("concern").and_then(serde_json::Value::as_str) == Some(*concern)
                    })
                    .and_then(|entry| entry.get("required_mappings"))
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        mappings.iter().any(|mapping| {
                            mapping.get("name").and_then(serde_json::Value::as_str)
                                == Some(*mapping_name)
                        })
                    })
            });
    let task_20_policy_trace = task_20_concerns.iter().all(|concern| {
        policy_concerns
            .iter()
            .find(|entry| entry.get("concern").and_then(serde_json::Value::as_str) == Some(concern))
            .is_some_and(|entry| {
                entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str() == Some("comfy-parity-authoritative-domain-ownership")
                        })
                    })
                    && entry
                        .get("validation")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|validations| {
                            validations.iter().any(|validation| {
                                validation.as_str() == Some("VAL-OWNERSHIP-DOMAIN-001")
                            })
                        })
            })
    });
    let task_20_catalog_trace = task_20_concerns.iter().all(|concern| {
        ownership_catalog
            .lines()
            .find(|line| line.starts_with(&format!("{concern},")))
            .is_some_and(|line| {
                line.contains("comfy-parity-authoritative-domain-ownership")
                    && line.contains("VAL-OWNERSHIP-DOMAIN-001")
            })
    });
    let native_api_policy_trace = native_api_concerns.iter().all(|concern| {
        policy_concerns.iter().any(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str) == Some(concern)
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("validation")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|validations| {
                        validations
                            .iter()
                            .any(|validation| validation.as_str() == Some("VAL-OWNERSHIP-001"))
                    })
        })
    });
    let native_api_catalog_trace = native_api_concerns.iter().all(|concern| {
        ownership_catalog
            .lines()
            .find(|line| line.starts_with(&format!("{concern},")))
            .is_some_and(|line| {
                line.contains("authoritative_owner_confirmed") && line.contains("VAL-OWNERSHIP-001")
            })
    });
    let graph_context_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("graph_context_dispatch")
        })
        .is_some_and(|entry| {
            entry
                .get("owner_symbols")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|symbols| {
                    ["GraphContextActionBinding", "dispatch_context_action"]
                        .into_iter()
                        .all(|expected| {
                            symbols
                                .iter()
                                .any(|symbol| symbol.as_str() == Some(expected))
                        })
                })
                && entry
                    .get("definitions")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|definitions| {
                        definitions.iter().any(|definition| {
                            definition.get("symbol").and_then(serde_json::Value::as_str)
                                == Some("dispatch_context_action")
                                && definition.get("role").and_then(serde_json::Value::as_str)
                                    == Some("canonical")
                                && definition.get("pattern").is_some()
                        })
                    })
        });
    let graph_context_catalog_trace = ownership_catalog
        .lines()
        .find(|line| line.starts_with("graph_context_dispatch,"))
        .is_some_and(|line| {
            line.contains("GraphContextActionBinding | dispatch_context_action")
                && line.contains("canonical@crates/comfy_ui/src/context_menu.rs:")
                && line.contains(":dispatch_context_action")
                && line.contains("authoritative_owner_confirmed")
        });
    let native_ffi_activation_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_ffi_certification")
        })
        .is_some_and(|entry| {
            entry
                .get("known_open_reasons")
                .and_then(serde_json::Value::as_array)
                .is_some_and(Vec::is_empty)
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        mappings.iter().any(|mapping| {
                            mapping.get("name").and_then(serde_json::Value::as_str)
                                == Some("platform-adapter-consumes-certified-ffi")
                                && mapping
                                    .get("activation_task")
                                    .and_then(serde_json::Value::as_str)
                                    == Some("comfy-parity-security-platform")
                                && mapping
                                    .get("activation_reason")
                                    .and_then(serde_json::Value::as_str)
                                    .is_some_and(|reason| !reason.is_empty())
                        })
                    })
        })
        && ownership_catalog
            .lines()
            .find(|line| line.starts_with("native_ffi_certification,"))
            .is_some_and(|line| {
                line.contains("authoritative_owner_confirmed")
                    && line.contains("mapping obligations activate with their owning tasks")
                    && line.contains("comfy-parity-security-platform")
            })
        && ownership_generator.contains("def task_completion_states()")
        && ownership_generator.contains("def mapping_obligations(")
        && ownership_generator.contains("completed task did not activate its required mapping");
    let native_library_image_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_library_image_capture_and_sealing")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_runtime::trust native-library image capture and sealing")
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some(
                                    "comfy-parity-unix-native-library-image-owner-consolidation",
                                )
                        })
                    })
                && entry
                    .get("required_mappings")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|mappings| {
                        [
                            (
                                "canonical-native-library-owner-captures-digests-seals-and-retains",
                                "crates/comfy_runtime/src/trust.rs",
                            ),
                            (
                                "mlu-certification-reuses-canonical-native-library-images",
                                "crates/comfy_runtime/src/native_ffi_mlu.rs",
                            ),
                            (
                                "rocm-certification-reuses-canonical-native-library-images",
                                "crates/comfy_runtime/src/native_ffi_rocm.rs",
                            ),
                            (
                                "npu-certification-reuses-canonical-native-library-images",
                                "crates/comfy_runtime/src/native_ffi_npu.rs",
                            ),
                            (
                                "xpu-certification-reuses-canonical-native-library-images",
                                "crates/comfy_runtime/src/native_ffi_xpu.rs",
                            ),
                            (
                                "cuda-certification-reuses-canonical-native-library-images",
                                "crates/comfy_runtime/src/native_ffi_cuda.rs",
                            ),
                            (
                                "native-library-image-owner-oracle-rejects-duplicates",
                                "crates/comfy_test_support/tests/ownership_consolidation.rs",
                            ),
                        ]
                        .iter()
                        .all(|(expected_name, expected_path)| {
                            mappings.iter().any(|mapping| {
                                mapping.get("name").and_then(serde_json::Value::as_str)
                                    == Some(*expected_name)
                                    && mapping.get("path").and_then(serde_json::Value::as_str)
                                        == Some(*expected_path)
                                    && mapping
                                        .get("pattern")
                                        .and_then(serde_json::Value::as_str)
                                        .is_some_and(|pattern| !pattern.is_empty())
                            })
                        })
                    })
        });
    let external_native_library_capture_sites = [
        "capture_native_library_image",
        "capture_native_library_image_with_check",
    ]
    .into_iter()
    .flat_map(|identifier| production_identifier_occurrences(&sources, identifier))
    .filter(|location| !location.contains("/crates/comfy_runtime/src/trust.rs"))
    .collect::<Vec<_>>();
    let native_library_capture_consumers_are_exact = exact_occurrence_files(
        &root,
        &external_native_library_capture_sites,
        &[
            "crates/comfy_runtime/src/native_ffi_mlu.rs",
            "crates/comfy_runtime/src/native_ffi_npu.rs",
            "crates/comfy_runtime/src/native_ffi_rocm.rs",
            "crates/comfy_runtime/src/native_ffi_xpu.rs",
            "crates/comfy_runtime/src/native_ffi_cuda.rs",
        ],
    )?;
    let native_library_image_capture_and_sealing_have_one_owner = native_library_image_policy_trace
        && production_source_occurrences(&sources, "pub(crate) enum NativeLibraryImageError").len()
            == 1
        && production_source_occurrences(&sources, "pub(crate) struct CapturedNativeLibraryImage")
            .len()
            == 1
        && production_source_occurrences(&sources, "pub(crate) struct RetainedNativeLibraryImage")
            .len()
            == 1
        && production_source_occurrences(&sources, "pub(crate) fn capture_native_library_image(")
            .len()
            == 1
        && runtime_trust_production.matches("libc::O_NOFOLLOW").count() == 1
        && runtime_trust_production
            .matches("libc::memfd_create")
            .count()
            == 1
        && runtime_trust_production
            .matches("libc::F_ADD_SEALS")
            .count()
            == 1
        && runtime_trust_production.contains("const MAX_NATIVE_LIBRARY_IMAGE_BYTES: u64")
        && runtime_trust_production.contains("const NATIVE_LIBRARY_IMAGE_CHUNK_BYTES: usize")
        && runtime_trust_production.contains("before.dev() != after.dev()")
        && runtime_trust_production.contains("before.ino() != after.ino()")
        && runtime_trust_production.contains("digest_sha256: format!(\"{:x}\", hasher.finalize())")
        && runtime_trust_production.contains("actual_seals & required_seals")
        && runtime_trust_production.contains("_file: File")
        && runtime_mlu_ffi_production.contains("capture_native_library_image(path, cancellation)")
        && runtime_mlu_ffi_production.contains("captured.digest_sha256()")
        && runtime_mlu_ffi_production.contains("captured\n            .seal(")
        && runtime_mlu_ffi_production.contains("_sealed_images: Vec<RetainedNativeLibraryImage>")
        && runtime_npu_ffi_production.contains("capture_native_library_image(path, cancellation)")
        && runtime_npu_ffi_production.contains("captured.digest_sha256()")
        && runtime_npu_ffi_production
            .contains(".seal(&format!(\"npu-{library_id}\"), cancellation)")
        && runtime_npu_ffi_production.contains("_sealed_images: Vec<RetainedNativeLibraryImage>")
        && runtime_xpu_ffi_production.contains("capture_native_library_image(path, cancellation)")
        && runtime_xpu_ffi_production.contains("captured.digest_sha256()")
        && runtime_xpu_ffi_production
            .contains(".seal(&format!(\"xpu-{library_id}\"), cancellation)")
        && runtime_xpu_ffi_production.contains("_sealed_images: Vec<RetainedNativeLibraryImage>")
        && runtime_cuda_ffi_production.contains("capture_native_library_image(path, cancellation)")
        && runtime_cuda_ffi_production.contains("captured.digest_sha256()")
        && runtime_cuda_ffi_production
            .contains(".seal(&format!(\"cuda-{library_id}\"), cancellation)")
        && runtime_cuda_ffi_production
            .contains("_sealed_images: BTreeMap<String, RetainedNativeLibraryImage>")
        && runtime_rocm_ffi_production.contains("capture_native_library_image_with_check(")
        && runtime_rocm_ffi_production
            .contains("elf64_dynamic_contract(image.bytes(), cancellation)")
        && runtime_rocm_ffi_production.contains("candidate.image.digest_sha256()")
        && runtime_rocm_ffi_production.contains(".seal_with_check(")
        && runtime_rocm_ffi_production.contains("_snapshots: Vec<RetainedNativeLibraryImage>")
        && [
            runtime_mlu_ffi_production,
            runtime_npu_ffi_production,
            runtime_rocm_ffi_production,
            runtime_xpu_ffi_production,
            runtime_cuda_ffi_production,
        ]
        .iter()
        .all(|adapter| {
            !adapter.contains("libc::O_NOFOLLOW")
                && !adapter.contains("libc::memfd_create")
                && !adapter.contains("libc::F_ADD_SEALS")
                && !adapter.contains("fn read_regular_file_without_following(")
                && !adapter.contains("fn sealed_snapshot(")
                && !adapter.contains("fn sha256_hex_cancellable(")
        })
        && runtime_directml_ffi_production.contains("struct SealedDirectMlImages")
        && runtime_directml_ffi_production.contains("fn seal_directml_images(")
        && !runtime_directml_ffi_production.contains("capture_native_library_image")
        && !runtime_directml_ffi_production.contains("libc::memfd_create")
        && native_library_capture_consumers_are_exact;
    if !native_library_image_capture_and_sealing_have_one_owner {
        eprintln!(
            "native-library image ownership: policy={native_library_image_policy_trace}, \
             capture_consumers={external_native_library_capture_sites:#?}, \
             exact_consumers={native_library_capture_consumers_are_exact}"
        );
    }

    let rocm_ffi_certification_has_one_authority_and_a_checked_adapter =
        source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && runtime_rocm_ffi_production.contains("registry: &NativeFfiRegistry")
            && runtime_rocm_ffi_production.contains("let certificate = registry")
            && runtime_rocm_ffi_production.contains(".authorize(")
            && runtime_rocm_ffi_production.contains("cancellation: &CancellationToken")
            && runtime_rocm_ffi_production.contains("checked_candidate_path")
            && runtime_rocm_ffi_production
                .contains("elf64_dynamic_contract(image.bytes(), cancellation)")
            && runtime_rocm_ffi_production.contains("exactly one PT_DYNAMIC segment")
            && runtime_rocm_ffi_production.contains("TRUSTED_SYSTEM_ELF_DEPENDENCIES")
            && runtime_rocm_ffi_production.contains("CancellableChunks")
            && runtime_rocm_ffi_production.contains("UnaccountedDependency")
            && runtime_rocm_ffi_production.contains("capture_native_library_image_with_check(")
            && runtime_rocm_ffi_production.contains(".seal_with_check(")
            && runtime_rocm_ffi_production.contains("remap_to_retained_descriptors")
            && runtime_rocm_ffi_production.contains("BackendLoader::load")
            && runtime_rocm_ffi_production.contains("struct RocmCertificationRetention")
            && runtime_rocm_ffi_production.contains("_snapshots: Vec<RetainedNativeLibraryImage>")
            && runtime_rocm_ffi_production.contains("_certificates: Vec<CertifiedNativeFfi>")
            && runtime_rocm_ffi_production.contains("Arc<dyn Any + Send + Sync>")
            && runtime_rocm_ffi_production.contains("RocmRuntime::load_certified")
            && runtime_rocm_ffi_production.contains("_certificates: prepared.certificates")
            && runtime_rocm_ffi_production.contains("_snapshots: prepared.snapshots")
            && runtime_rocm_ffi_production
                .matches("NativeFfiContract::new")
                .count()
                == 1
            && backend_rocm_loader.contains("pub unsafe fn load_certified")
            && backend_rocm_loader.contains("remap_to_retained_descriptors")
            && backend_rocm_loader.contains("validate_sealed_memfd")
            && backend_rocm_loader.contains("F_GET_SEALS")
            && backend_rocm_loader.contains("signature_coverage")
            && backend_rocm_loader.contains("validate_signed_package")
            && backend_rocm_loader.contains("validate_tree_membership")
            && backend_rocm_loader.contains("validate_coverage")
            && backend_rocm_package_policy.contains("comfy_runtime-native-rust-ed25519")
            && backend_rocm_packager.contains("ffi-contracts-v1.json")
            && !backend_rocm_packager.contains("COMFY_ROCM_SIGNATURE_VERIFIER")
            && backend_rocm_packager.contains("package-coverage.sha256")
            && backend_rocm_packager.contains("runtime_root")
            && !backend_rocm_loader.contains("use comfy_runtime")
            && !backend_rocm_loader.contains("NativeFfiRegistry::")
            && !backend_rocm_loader.contains("NativeFfiContract::");

    let rocm_package_trust_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("rocm_package_trust_and_contract_mapping")
        })
        .is_some_and(|entry| {
            entry.get("canonical_owner").and_then(serde_json::Value::as_str)
                == Some("comfy_runtime::RocmPackageVerificationKey")
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some(
                                    "comfy-parity-provision-native-ffi-contracts-amd-rocm-comfy-model-0014",
                                )
                        })
                    })
        });
    let rocm_package_trust_has_one_authority_and_explicit_adapters = runtime_trust_production
        .matches("pub struct RocmPackageVerificationKey")
        .count()
        == 1
        && runtime_trust.contains("ROCM_PACKAGE_SIGNATURE_DOMAIN")
        && runtime_trust.contains("rocm_package_signing_payload")
        && runtime_trust.contains("canonical_receipt.push(b'\\n')")
        && runtime_trust.contains("rocm_package_signature_has_a_distinct_signer_bound_domain")
        && runtime_rocm_ffi_production.contains("NativeRocmPackageVerifier")
        && runtime_rocm_ffi_production
            .find("verify_signed_package_root")
            .zip(runtime_rocm_ffi_production.find("parse_rocm_ffi_contract_catalog"))
            .is_some_and(|(verification, catalog)| verification < catalog)
        && runtime_rocm_ffi_production
            .matches("NativeFfiContract::new")
            .count()
            == 1
        && runtime_rocm_ffi
            .contains("complete_signed_fixture_drives_exact_registry_certification_closure")
        && runtime_rocm_ffi.contains("prepare_certified_load(verified.registry()")
        && runtime_settings.contains("project_rocm_package_settings")
        && runtime_settings.contains("RocmPackageVerificationKey::new")
        && runtime_settings.contains("is_private_signing_setting")
        && backend_rocm_loader.contains("ffi_contracts_sha256")
        && backend_rocm_loader.contains("validate_tree_membership")
        && backend_rocm_loader.contains("validate_coverage")
        && backend_rocm_loader.contains("checked_sdk_root")
        && backend_rocm_packager.contains("validate_and_copy_contract_catalog")
        && backend_rocm_packager.contains("canonical-json-v1")
        && !backend_rocm_packager.contains("COMFY_ROCM_SIGNATURE_VERIFIER");

    let metal_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_ffi_metal_abi_and_package_foundation")
        })
        .is_some_and(|entry| {
            entry.get("canonical_owner").and_then(serde_json::Value::as_str)
                == Some("comfy_backend_metal")
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some(
                                    "comfy-parity-device-foundation-apple-metal-mps-comfy-model-0015",
                                )
                        })
                    })
        });
    let metal_package_trust_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("runtime_metal_package_trust_and_contract_mapping")
        })
        .is_some_and(|entry| {
            entry.get("canonical_owner").and_then(serde_json::Value::as_str)
                == Some("comfy_runtime::MetalPackageVerificationKey")
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.iter().any(|task| {
                            task.as_str()
                                == Some(
                                    "comfy-parity-provision-native-ffi-contracts-apple-metal-mps-comfy-model-0015",
                                )
                        })
                    })
        });
    let native_package_admission_helpers_have_only_expected_sites = native_package_capture_sites
        .iter()
        .chain(&native_package_coverage_sites)
        .all(|location| {
            location.contains("crates/comfy_runtime/src/trust.rs")
                || location.contains("crates/comfy_runtime/src/native_ffi_metal.rs")
                || location.contains("crates/comfy_runtime/src/native_ffi_mlu.rs")
                || location.contains("crates/comfy_runtime/src/native_ffi_npu.rs")
                || location.contains("crates/comfy_runtime/src/native_ffi_xpu.rs")
                || location.contains("crates/comfy_runtime/src/native_ffi_cuda.rs")
                || location.contains("crates/comfy_runtime/src/native_ffi_directml.rs")
        });
    let native_package_admission_uses_one_security_owner =
        artifact_root_recursive_enumeration_definitions.len() == 1
            && artifact_root_recursive_enumeration_definitions[0]
                .contains("crates/comfy_model/src/artifact_index.rs")
            && native_package_capture_definitions.len() == 1
            && native_package_capture_definitions[0].contains("crates/comfy_runtime/src/trust.rs")
            && native_package_coverage_definitions.len() == 1
            && native_package_coverage_definitions[0].contains("crates/comfy_runtime/src/trust.rs")
            && native_package_admission_helpers_have_only_expected_sites
            && model_artifact_index_production
                .contains("pub fn list_contained_regular_files_recursive(")
            && model_artifact_index_production.contains("validate_root_snapshot(self, &limits)")
            && model_artifact_index_production.contains("trusted_directory")
            && model_artifact_index_production.contains(".entries()")
            && model_artifact_index_production.contains("symlink_metadata(&file_name)")
            && model_artifact_index_production.contains("metadata.file_type().is_symlink()")
            && runtime_trust_production.contains("pub(crate) fn capture_native_package(")
            && runtime_trust_production.contains("pub(crate) fn validate_native_package_coverage(")
            && runtime_trust_production.contains("if observed_paths != expected_paths")
            && runtime_trust_production.contains("package tree changed while it was captured")
            && runtime_trust_production.contains("if total_bytes > maximum_total_bytes")
            && runtime_trust_production.contains("coverage paths must be exact, strictly sorted")
            && !runtime_trust_production.contains("fs::read_dir(")
            && runtime_metal_ffi_production.contains("capture_native_package(")
            && runtime_metal_ffi_production.contains("validate_native_package_coverage(")
            && runtime_mlu_ffi_production.contains("capture_native_package(")
            && runtime_mlu_ffi_production.contains("validate_native_package_coverage(")
            && runtime_npu_ffi_production.contains("capture_native_package(")
            && runtime_npu_ffi_production.contains("validate_native_package_coverage(")
            && runtime_xpu_ffi_production.contains("capture_native_package(")
            && runtime_xpu_ffi_production.contains("validate_native_package_coverage(")
            && runtime_cuda_ffi_production.contains("capture_native_package(")
            && runtime_cuda_ffi_production.contains("validate_native_package_coverage(")
            && runtime_directml_ffi_production.contains("capture_native_package(")
            && runtime_directml_ffi_production.contains("validate_native_package_coverage(")
            && [
                runtime_metal_ffi_production,
                runtime_mlu_ffi_production,
                runtime_npu_ffi_production,
                runtime_xpu_ffi_production,
                runtime_cuda_ffi_production,
                runtime_directml_ffi_production,
            ]
            .into_iter()
            .all(|adapter| {
                !adapter.contains("inspect_exact_package_tree")
                    && !adapter.contains("fn validate_coverage(")
                    && !adapter.contains("fs::read_dir(")
            })
            && [
                backend_metal_packager.as_str(),
                backend_mlu_packager.as_str(),
                backend_npu_packager.as_str(),
                backend_xpu_packager.as_str(),
                backend_cuda_packager.as_str(),
                backend_directml_packager.as_str(),
            ]
            .into_iter()
            .all(|packager| {
                !packager.contains("capture_native_package")
                    && !packager.contains("validate_native_package_coverage")
                    && !packager.contains("list_contained_regular_files_recursive")
            });
    if !native_package_admission_uses_one_security_owner {
        eprintln!(
            "native package admission ownership: recursive_enumeration={artifact_root_recursive_enumeration_definitions:#?}, \
             capture_definitions={native_package_capture_definitions:#?}, \
             coverage_definitions={native_package_coverage_definitions:#?}, \
             capture_sites={native_package_capture_sites:#?}, \
             coverage_sites={native_package_coverage_sites:#?}"
        );
    }
    let metal_package_trust_has_one_authority_and_explicit_adapters =
        production_source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && production_source_occurrences(&sources, "struct NativePackageVerificationAuthority")
                .len()
                == 1
            && runtime_trust_production
                .matches("pub struct MetalPackageVerificationKey")
                .count()
                == 1
            && metal_package_trust_policy_trace
            && runtime_trust.contains("ROCM_PACKAGE_SIGNATURE_DOMAIN")
            && runtime_trust.contains("METAL_PACKAGE_SIGNATURE_DOMAIN")
            && runtime_trust.contains("parse_strict_json_value")
            && runtime_trust.contains("rocm_package_signature_has_a_distinct_signer_bound_domain")
            && runtime_metal_ffi_production
                .find("verification_key.verify_package")
                .zip(runtime_metal_ffi_production.find("let catalog: MetalFfiContractCatalogDto"))
                .is_some_and(|(verification, catalog)| verification < catalog)
            && runtime_metal_ffi_production.contains("capture_native_package")
            && runtime_metal_ffi_production.contains("validate_native_package_coverage")
            && runtime_metal_ffi_production.contains("crate::trust::parse_strict_json_value")
            && runtime_metal_ffi_production
                .matches("NativeFfiRegistry::new")
                .count()
                == 1
            && runtime_metal_ffi_production.contains("readiness_metallib")
            && runtime_metal_ffi_production.contains("tensor_ops_metallib")
            && runtime_settings.contains("project_metal_package_settings")
            && runtime_settings.contains("NativeMetalPackageSettings::from_public_authority")
            && runtime_settings.contains("is_private_signing_setting")
            && backend_metal_contract_schema.contains("\"additionalProperties\": false")
            && backend_metal_contract_schema.contains("metal-tensor-ops-metallib")
            && backend_metal_package_policy.contains("ffi-contracts-v1.json")
            && backend_metal_package_policy.contains("comfy_runtime::MetalPackageVerificationKey")
            && backend_metal_packager.contains("validate_and_copy_contract_catalog")
            && backend_metal_packager.contains("canonical-json-v1")
            && !backend_metal_packager.contains("NativeFfiRegistry")
            && !backend_metal_packager.contains("verify_package(")
            && !backend_metal_loader.contains("use comfy_runtime")
            && !backend_metal_loader.contains("NativeFfiRegistry::");
    let metal_abi_foundation_is_observation_only =
        source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && metal_policy_trace
            && backend_metal_abi.contains("const FRAMEWORKS:")
            && backend_metal_abi.contains("const CLASSES:")
            && backend_metal_abi.contains("const LAYOUTS:")
            && backend_metal_abi.contains("const HEADERS:")
            && backend_metal_abi.contains("/System/Library/Frameworks/")
            && backend_metal_loader.contains("RTLD_FIRST")
            && backend_metal_loader.contains("dladdr")
            && backend_metal_loader.contains("class_getImageName")
            && backend_metal_loader.contains("method_getTypeEncoding")
            && backend_metal_loader.contains("MTLCreateSystemDefaultDevice")
            && backend_metal_loader.contains("MPSSupportsMTLDevice")
            && !backend_metal_loader.contains("use comfy_runtime")
            && !backend_metal_loader.contains("NativeFfiRegistry::")
            && !backend_metal_loader.contains("CertifiedNativeFfi")
            && !backend_metal_loader.contains("BackendCapabilityMatrix")
            && !backend_metal_loader.contains("TensorBackend")
            && !backend_metal_loader.contains("NativeBackendBindingStatus::bound")
            && backend_metal_adapter.contains("NativeBackendBindingStatus::unbound")
            && backend_metal_adapter.contains("NativeFfiRegistry")
            && !backend_metal_adapter.contains("NativeBackendBindingStatus::bound")
            && !backend_metal_build.contains("xcrun")
            && backend_metal_packager.contains("readiness.metal")
            && backend_metal_packager.contains("readiness.metallib")
            && backend_metal_packager.contains("package-coverage.sha256")
            && backend_metal_package_policy.contains("required_entitlements")
            && !backend_metal_packager.contains("objc_getClass")
            && !backend_metal_packager.contains("MTLCreateSystemDefaultDevice")
            && !backend_metal_packager.contains("cp /System/Library/Frameworks/")
            && !backend_metal_packager.contains("cp -R /System/Library/Frameworks/")
            && !backend_metal_packager.contains("NativeFfiRegistry")
            && !backend_metal_packager.contains("MetalPackageVerificationKey")
            && !gpui_metal_renderer.contains("comfy_backend_metal")
            && !media_owner.contains("comfy_backend_metal");

    let mlu_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_ffi_mlu_abi_and_package_foundation")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_backend_mlu")
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        [
                            "comfy-parity-device-foundation-cambricon-mlu-comfy-model-0017",
                            "comfy-parity-vendor-abi-wave39-ownership-consolidation",
                        ]
                        .iter()
                        .all(|required| tasks.iter().any(|task| task.as_str() == Some(required)))
                    })
        });
    let mlu_abi_foundation_requires_canonical_runtime_certification =
        production_source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && production_source_occurrences(&sources, "pub enum NativeBackendBindingStatus").len()
                == 1
            && mlu_policy_trace
            && backend_mlu_abi.contains("pub struct AbiManifest")
            && backend_mlu_abi.contains("const REVIEWED_ENUM_VALUES:")
            && backend_mlu_abi.contains("pub type CnnlOpTensor")
            && backend_mlu_loader.contains("pub(crate) struct CertifiedMluImages<'certificate>")
            && backend_mlu_loader.contains("pub struct RegistryCertifiedImage {")
            && backend_mlu_loader
                .contains("pub(crate) unsafe fn from_registry_certificates<W: ?Sized>(")
            && backend_mlu_loader.contains("pub struct MluRuntime<'certificate>")
            && backend_mlu_loader.contains("struct MluSymbols {")
            && backend_mlu_loader.contains("struct MluCallSurface<'authority>")
            && backend_mlu_loader.contains("certificate_lifetime: PhantomData")
            && backend_mlu_loader.contains("_retained_images: platform::RetainedHandles")
            && backend_mlu_loader.contains("/proc/self/fd/")
            && backend_mlu_loader.contains("fn verify_immutable_sealed_fd(path: &Path) -> bool")
            && backend_mlu_loader.contains("libc::F_GET_SEALS")
            && backend_mlu_loader.contains("pub(crate) fn add(")
            && !declaration_derives_trait(
                &backend_mlu_loader,
                "pub(crate) struct CertifiedMluImages<'certificate>",
                "Clone",
            )
            && !backend_mlu_loader.contains("impl Clone for CertifiedMluImages")
            && !declaration_derives_trait(
                &backend_mlu_loader,
                "pub struct MluRuntime<'certificate>",
                "Clone",
            )
            && !backend_mlu_loader.contains("impl Clone for MluRuntime")
            && !declaration_derives_trait(&backend_mlu_loader, "struct MluSymbols", "Clone")
            && !backend_mlu_loader.contains("impl Clone for MluSymbols")
            && !declaration_derives_trait(
                &backend_mlu_loader,
                "struct MluCallSurface<'authority>",
                "Clone",
            )
            && !backend_mlu_loader.contains("impl Clone for MluCallSurface")
            && !declaration_derives_trait(
                &backend_mlu_loader,
                "pub struct RegistryCertifiedImage",
                "Clone",
            )
            && !backend_mlu_loader.contains("impl Clone for RegistryCertifiedImage")
            && !backend_mlu_loader.contains("NativeFfiRegistry::")
            && !backend_mlu_loader.contains("NativeBackendBindingStatus::bound")
            && !backend_mlu_loader.contains("BackendCapabilityMatrix")
            && !backend_mlu_loader.contains("TensorBackend")
            && backend_mlu_adapter.contains("NativeBackendBindingStatus::unbound")
            && !backend_mlu_adapter.contains("NativeBackendBindingStatus::bound")
            && backend_mlu_packager.contains("redistributes_vendor_runtime")
            && backend_mlu_packager.contains("package-coverage.sha256")
            && backend_mlu_package_policy.contains("\"redistributes_vendor_runtime\": false")
            && !backend_mlu_packager.contains("NativeFfiRegistry::")
            && !backend_mlu_packager.contains("ed25519_dalek");

    let directml_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_ffi_directml_abi_and_package_foundation")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_backend_directml")
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        [
                            "comfy-parity-device-foundation-directml-comfy-model-0018",
                            "comfy-parity-vendor-abi-wave39-ownership-consolidation",
                            "comfy-parity-native-package-admission-ownership-consolidation",
                            "comfy-parity-provision-native-ffi-contracts-directml-comfy-model-0018",
                            "comfy-parity-integrate-device-directml-comfy-model-0018",
                        ]
                        .iter()
                        .all(|required| tasks.iter().any(|task| task.as_str() == Some(required)))
                    })
        });
    let directml_abi_foundation_requires_canonical_runtime_certification =
        source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && source_occurrences(&sources, "pub enum NativeBackendBindingStatus").len() == 1
            && directml_policy_trace
            && backend_directml_abi.contains("pub struct AbiManifest")
            && backend_directml_abi.contains("pub type DmlCreateDevice1Fn")
            && backend_directml_abi.contains("pub struct DmlDeviceVTable")
            && backend_directml_loader.contains("pub struct RegistryCertifiedDirectMlImage {")
            && backend_directml_loader.contains("pub struct RetainedDirectMlLibraryHandles {")
            && backend_directml_loader
                .contains("pub(crate) struct CertifiedDirectMlExecutionInputs {")
            && backend_directml_loader.contains("struct OwnedModule {")
            && backend_directml_loader.contains("_retention: Arc<dyn Any + Send + Sync>")
            && backend_directml_loader.contains("module: OwnedModule")
            && backend_directml_loader.contains("unsafe fn load_exact(")
            && backend_directml_loader.matches("LoadLibraryExW(").count() == 1
            && backend_directml_loader
                .contains("LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32")
            && backend_directml_loader.matches("FreeLibrary(").count() == 1
            && backend_directml_loader.contains("into_execution_inputs(")
            && backend_directml_loader.contains("_handles: RetainedDirectMlLibraryHandles")
            && backend_directml_loader.contains("GetProcAddress")
            && backend_directml_loader.contains("pub(crate) struct DirectMlDevice")
            && backend_directml_loader.contains("struct ComObject")
            && backend_directml_loader.contains("vtable.release")
            && !backend_directml_loader.contains("impl Clone for RegistryCertifiedDirectMlImage")
            && !backend_directml_loader.contains("impl Clone for RetainedDirectMlLibraryHandles")
            && !backend_directml_loader.contains("impl Clone for OwnedModule")
            && !backend_directml_loader.contains("LoadLibraryW(")
            && backend_directml_loader_production.contains("pub struct DirectMlDiscoveryPlan")
            && backend_directml_loader_production
                .contains("pub struct DirectMlCandidateObservation")
            && backend_directml_loader_production.contains("pub fn observe_directml_candidate")
            && backend_directml_loader_production.contains("GetSystemDirectoryW")
            && backend_directml_loader_production.contains("RtlGetVersion")
            && backend_directml_loader_production.contains("GetFileVersionInfoW")
            && backend_directml_loader_production.contains("WinVerifyTrust")
            && backend_directml_loader_production.contains("WTD_CACHE_ONLY_URL_RETRIEVAL")
            && backend_directml_loader_production.contains("WTD_REVOKE_NONE")
            && !backend_directml_loader_production.contains("pub authenticode_trusted")
            && !backend_directml_loader.contains("NativeFfiRegistry::")
            && !backend_directml_loader.contains("NativeBackendBindingStatus::bound")
            && !backend_directml_loader.contains("BackendCapabilityMatrix")
            && !backend_directml_loader.contains("TensorBackend")
            && backend_directml_adapter_production.contains("NativeBackendBindingStatus::unbound")
            && !backend_directml_adapter_production.contains("NativeBackendBindingStatus::bound")
            && backend_directml_packager.contains("runtime_authorization_from_structure")
            && backend_directml_packager.contains("package-coverage.sha256")
            && backend_directml_package_policy
                .contains("\"runtime_authorization_from_structure\": false")
            && backend_directml_package_policy.contains("\"structural_receipt_required\": true")
            && !backend_directml_packager.contains("NativeFfiRegistry::")
            && !backend_directml_packager.contains("ed25519_dalek");
    let directml_compute_and_gpui_rendering_raw_abi_owners_are_separate = backend_directml_abi
        .contains("pub type DmlCreateDevice1Fn")
        && backend_directml_loader_production.contains("LoadLibraryExW")
        && backend_directml_loader_production.contains("DirectML.dll")
        && backend_directml_loader_production.contains("D3D12CreateDevice")
        && backend_directml_loader_production.contains("CreateDXGIFactory2")
        && gpui_windows_directx_renderer.contains("IDXGISwapChain1")
        && gpui_windows_directx_renderer.contains("DXGI_USAGE_RENDER_TARGET_OUTPUT")
        && gpui_windows_directx_devices.contains("CreateDXGIFactory2")
        && gpui_windows_directx_devices.contains("IDXGIFactory6")
        && !gpui_windows_directx_renderer.contains("comfy_backend_directml")
        && !gpui_windows_directx_devices.contains("comfy_backend_directml")
        && !gpui_windows_directx_renderer.contains("DirectML.dll")
        && !gpui_windows_directx_devices.contains("DirectML.dll")
        && !gpui_windows_directx_renderer.contains("DmlCreateDevice1Fn")
        && !gpui_windows_directx_devices.contains("DmlCreateDevice1Fn")
        && !backend_directml_loader_production.contains("gpui_windows")
        && !runtime_directml_ffi_production.contains("gpui_windows");

    let directml_package_trust_has_one_authority_and_explicit_adapters =
        production_source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && production_source_occurrences(&sources, "struct NativePackageVerificationAuthority")
                .len()
                == 1
            && runtime_trust_production
                .matches("pub struct DirectMlPackageVerificationKey")
                .count()
                == 1
            && directml_policy_trace
            && runtime_trust.contains("ROCM_PACKAGE_SIGNATURE_DOMAIN")
            && runtime_trust
                .matches("const DIRECTML_PACKAGE_SIGNATURE_DOMAIN")
                .count()
                == 1
            && runtime_trust.contains("directml_package_signing_payload")
            && runtime_directml_ffi_production
                .find("verification_key.verify_package")
                .zip(
                    runtime_directml_ffi_production
                        .find("let catalog: DirectMlFfiContractCatalogDto"),
                )
                .is_some_and(|(verification, catalog)| verification < catalog)
            && runtime_directml_ffi_production.contains("capture_native_package")
            && runtime_directml_ffi_production.contains("validate_native_package_coverage")
            && runtime_directml_ffi_production.contains("crate::trust::parse_strict_json_value")
            && runtime_directml_ffi_production
                .matches("NativeFfiRegistry::new")
                .count()
                == 1
            && runtime_directml_ffi_production
                .matches("NativeFfiContract::new")
                .count()
                == 1
            && runtime_directml_ffi_production.contains("RetainedDirectMlLibraryHandles")
            && runtime_directml_ffi_production
                .contains("let retention = Arc::new(DirectMlCertificationRetention")
            && runtime_directml_ffi_production
                .contains("from_registry_certificates(retention, projected_images)")
            && runtime_directml_ffi_production
                .contains("DirectMlDiscoveryPlan::for_current_system")
            && runtime_directml_ffi_production.contains("observe_directml_candidate")
            && !runtime_directml_ffi_production.contains("authenticode_trusted: true")
            && !runtime_directml_ffi_production.contains("Box::leak")
            && runtime_settings.contains("project_directml_package_settings")
            && runtime_settings.contains("NativeDirectMlPackageSettings::from_public_authority")
            && runtime_settings.contains("is_private_signing_setting")
            && !runtime_settings.contains("DirectMlPackageVerificationKey::verify_package")
            && !runtime_settings.contains("NativeFfiRegistry::new")
            && backend_directml_contract_schema.contains("\"additionalProperties\": false")
            && backend_directml_contract_schema.contains("D3D12.dll")
            && backend_directml_contract_schema.contains("DirectML.dll")
            && backend_directml_contract_schema.contains("DXGI.dll")
            && backend_directml_package_policy.contains("ffi-contracts-v1.json")
            && backend_directml_package_policy
                .contains("comfy_runtime::DirectMlPackageVerificationKey")
            && backend_directml_packager.contains("validate_contract_catalog")
            && backend_directml_packager.contains("stable_regular_file")
            && backend_directml_packager.contains("separately reviewed FFI contract catalog")
            && !backend_directml_packager.contains("NativeFfiRegistry::new")
            && !backend_directml_packager.contains("verify_package(")
            && !backend_directml_loader.contains("use comfy_runtime")
            && !backend_directml_loader.contains("NativeFfiRegistry::");

    let mlu_directml_execution_policy_trace = [
        (
            "backend_capability",
            "comfy_tensor::BackendCapabilityMatrix",
        ),
        ("cancellation", "comfy_types::CancellationToken"),
        (
            "native_ffi_certification",
            "comfy_runtime::NativeFfiRegistry",
        ),
        (
            "native_ffi_mlu_execution_resources",
            "comfy_backend_mlu::MluExecutionRuntime and its opaque MluExecutionAllocation, MluExecutionStream, and MluExecutionEvent resources",
        ),
        (
            "native_ffi_directml_execution_resources",
            "comfy_backend_directml::DirectMlExecutionSession and its opaque DirectMlAllocation, DirectMlStream, and DirectMlEvent resources",
        ),
        (
            "selected_worker_backend_session",
            "comfy_worker::WorkerBackendSession",
        ),
        (
            "tensor_backend_adapter_resource_registries",
            "comfy_tensor::BackendStorage, Tensor::backend_storage, BackendResourceRegistry, and BackendEventTracker",
        ),
        (
            "tensor_backend_allocation_and_cache",
            "the certified comfy_tensor::TensorBackend implementation for an exact DeviceId",
        ),
    ]
    .into_iter()
    .all(|(concern, canonical_owner)| {
        policy_concerns
            .iter()
            .find(|entry| {
                entry.get("concern").and_then(serde_json::Value::as_str) == Some(concern)
            })
            .is_some_and(|entry| {
                entry
                    .get("canonical_owner")
                    .and_then(serde_json::Value::as_str)
                    == Some(canonical_owner)
                    && entry
                        .get("known_open_reasons")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(Vec::is_empty)
                    && entry
                        .get("consolidation_tasks")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|tasks| {
                            tasks.iter().any(|task| {
                                task.as_str()
                                    == Some(
                                        "comfy-parity-mlu-directml-execution-ownership-reconciliation",
                                    )
                            })
                        })
            })
    });
    let external_mlu_runtime_sites = production_identifier_occurrences(&sources, "MluRuntime")
        .into_iter()
        .filter(|location| !location.contains("/crates/comfy_backend_mlu/"))
        .collect::<Vec<_>>();
    let external_directml_runtime_sites =
        production_identifier_occurrences(&sources, "DirectMlRuntime")
            .into_iter()
            .filter(|location| !location.contains("/crates/comfy_backend_directml/"))
            .collect::<Vec<_>>();
    let external_mlu_execution_sites =
        production_identifier_occurrences(&sources, "MluExecutionRuntime")
            .into_iter()
            .filter(|location| !location.contains("/crates/comfy_backend_mlu/"))
            .collect::<Vec<_>>();
    let external_directml_execution_sites =
        production_identifier_occurrences(&sources, "DirectMlExecutionSession")
            .into_iter()
            .filter(|location| !location.contains("/crates/comfy_backend_directml/"))
            .collect::<Vec<_>>();
    let mlu_execution_consumers_are_exact = exact_occurrence_files(
        &root,
        &external_mlu_execution_sites,
        &[
            "crates/comfy_runtime/src/native_ffi_mlu.rs",
            "crates/comfy_tensor/src/backends/cambricon_mlu_comfy_model_0017.rs",
        ],
    )?;
    let directml_execution_consumers_are_exact = exact_occurrence_files(
        &root,
        &external_directml_execution_sites,
        &[
            "crates/comfy_runtime/src/native_ffi_directml.rs",
            "crates/comfy_tensor/src/backends/directml_comfy_model_0018.rs",
        ],
    )?;
    let certification_constructor_sites_are_exact = [
        (
            "MluExecutionRuntime::load_certified",
            "crates/comfy_runtime/src/native_ffi_mlu.rs",
        ),
        (
            "CertifiedMluImages::from_registry_certificates",
            "crates/comfy_backend_mlu/src/execution.rs",
        ),
        (
            "RegistryCertifiedDirectMlImage::load_from_registry_certificate",
            "crates/comfy_runtime/src/native_ffi_directml.rs",
        ),
        (
            "RetainedDirectMlLibraryHandles::from_registry_certificates",
            "crates/comfy_runtime/src/native_ffi_directml.rs",
        ),
        (
            "DirectMlExecutionSession::from_registry_certified_handles",
            "crates/comfy_runtime/src/native_ffi_directml.rs",
        ),
    ]
    .into_iter()
    .try_fold(true, |all_exact, (constructor, expected_path)| {
        let sites = production_identifier_occurrences(&sources, constructor);
        Ok::<_, Box<dyn std::error::Error>>(
            all_exact && exact_occurrence_files(&root, &sites, &[expected_path])?,
        )
    })?;
    let backend_production_source = |crate_path: &str| {
        let crate_path = root.join(crate_path);
        sources
            .iter()
            .filter(|(path, _)| path.starts_with(&crate_path) && !is_test_only_source(path))
            .map(|(_, source)| {
                source
                    .split_once("#[cfg(test)]\nmod tests")
                    .map_or(source.as_str(), |(production, _)| production)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let backend_mlu_production = backend_production_source("crates/comfy_backend_mlu");
    let backend_directml_production = backend_production_source("crates/comfy_backend_directml");
    let forbidden_backend_owner_symbols = [
        "pub struct NativeFfiRegistry",
        "NativeFfiRegistry::",
        "pub struct BackendCapabilityMatrix",
        "pub struct NativeDeviceProperties",
        "pub(crate) struct BackendMemoryTracker",
        "pub struct BackendWorkspaceAuthority",
        "pub(crate) struct BackendResourceRegistry",
        "pub(crate) struct BackendEventTracker",
        "pub struct WorkerBackendSession",
        "pub struct CancellationToken",
        "WorkspaceDb",
        "SerializableItem",
        "OutputCommitter",
        "AssetService",
        "PermissionPolicy",
        "ExecutionQueue",
    ];
    let backend_crates_do_not_own_canonical_services =
        forbidden_backend_owner_symbols.iter().all(|symbol| {
            !backend_mlu_production.contains(symbol)
                && !backend_directml_production.contains(symbol)
        });
    let mlu_directml_execution_resources_preserve_canonical_owners =
        mlu_directml_execution_policy_trace
            && source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && source_occurrences(&sources, "pub enum NativeBackendBindingStatus").len() == 1
            && source_occurrences(&sources, "pub struct BackendCapabilityMatrix").len() == 1
            && source_occurrences(&sources, "pub struct NativeDeviceProperties").len() == 1
            && production_source_occurrences(&sources, "pub(crate) struct BackendMemoryTracker")
                .len()
                == 1
            && production_source_occurrences(&sources, "pub struct BackendWorkspaceAuthority")
                .len()
                == 1
            && production_source_occurrences(
                &sources,
                "pub(crate) struct BackendResourceRegistry<",
            )
            .len()
                == 1
            && production_source_occurrences(&sources, "pub(crate) struct BackendEventTracker<")
                .len()
                == 1
            && production_source_occurrences(&sources, "pub struct WorkerBackendSession").len()
                == 1
            && source_occurrences(&sources, "pub struct CancellationToken").len() == 1
            && source_occurrences(&sources, "pub struct MluExecutionRuntime").len() == 1
            && source_occurrences(&sources, "pub struct MluExecutionAllocation").len() == 1
            && source_occurrences(&sources, "pub struct MluExecutionStream").len() == 1
            && source_occurrences(&sources, "pub struct MluExecutionEvent").len() == 1
            && source_occurrences(&sources, "pub struct MluAbiProbe").len() == 1
            && source_occurrences(&sources, "pub struct DirectMlExecutionSession").len() == 1
            && source_occurrences(&sources, "pub struct DirectMlAllocation").len() == 1
            && source_occurrences(&sources, "pub struct DirectMlStream").len() == 1
            && source_occurrences(&sources, "pub struct DirectMlEvent").len() == 1
            && source_occurrences(&sources, "unsafe impl Send for SerializedMluCore").len() == 1
            && backend_mlu_execution_production.contains("pub unsafe fn load_certified(")
            && backend_mlu_execution_production
                .contains("_certification: Arc<dyn Any + Send + Sync>")
            && backend_mlu_execution_production.contains("state: Mutex<RuntimeState>")
            && backend_mlu_execution_production.contains("probe: MluAbiProbe")
            && backend_mlu_execution_production.contains("pub fn probe(&self) -> &MluAbiProbe")
            && backend_mlu_execution_production.contains("runtime_id: u64")
            && backend_mlu_execution_production.contains("stream: MluExecutionStream")
            && backend_mlu_execution_production.contains("synchronized: Arc<AtomicBool>")
            && backend_mlu_execution_production
                .contains("self.with_state(|state| state.synchronize_stream(")
            && backend_mlu_execution_production
                .contains("event.synchronized.store(true, Ordering::Release)")
            && !backend_mlu_execution_production.contains("sequence: u64")
            && !backend_mlu_execution_production.contains("next_event_sequence")
            && backend_mlu_execution_production.contains("use comfy_types::CancellationToken")
            && !backend_mlu_execution_production.contains("pub struct CancellationToken")
            && !backend_mlu_execution_production.contains("NativeFfiRegistry::")
            && !backend_mlu_execution_production.contains("NativeBackendBindingStatus::bound")
            && !backend_mlu_execution_production.contains("BackendCapabilityMatrix")
            && !backend_mlu_execution_production.contains("BackendMemoryTracker")
            && !backend_mlu_execution_production.contains("BackendWorkspaceAuthority")
            && !backend_mlu_execution_production.contains("BackendResourceRegistry")
            && !backend_mlu_execution_production.contains("BackendEventTracker")
            && !backend_mlu_execution_production.contains("WorkerBackendSession")
            && !backend_mlu_execution_production.contains("WorkspaceDb")
            && !backend_mlu_execution_production.contains("SerializableItem")
            && !backend_mlu_execution_production.contains("OutputCommitter")
            && backend_directml_execution_production
                .contains("pub fn from_registry_certified_handles(")
            && backend_directml_execution_production
                .contains("handles: RetainedDirectMlLibraryHandles")
            && backend_directml_execution_production.contains("into_execution_inputs()")
            && backend_directml_execution_production
                .contains("certification: Arc<dyn std::any::Any + Send + Sync>")
            && backend_directml_loader.contains("struct DirectMlSymbols {")
            && !declaration_derives_trait(
                &backend_directml_loader,
                "struct DirectMlSymbols",
                "Clone",
            )
            && !declaration_derives_trait(
                &backend_directml_loader,
                "struct DirectMlSymbols",
                "Copy",
            )
            && !backend_directml_loader.contains("impl Clone for DirectMlSymbols")
            && !backend_directml_loader.contains("impl Copy for DirectMlSymbols")
            && !declaration_derives_trait(
                &backend_directml_loader,
                "pub(crate) struct CertifiedDirectMlExecutionInputs",
                "Clone",
            )
            && !backend_directml_loader.contains("impl Clone for CertifiedDirectMlExecutionInputs")
            && !backend_directml_execution_production.contains("pub struct CancellationToken")
            && !backend_directml_execution_production.contains("NativeFfiRegistry::")
            && !backend_directml_execution_production.contains("NativeBackendBindingStatus::bound")
            && !backend_directml_execution_production.contains("BackendCapabilityMatrix")
            && !backend_directml_execution_production.contains("NativeDeviceProperties")
            && !backend_directml_execution_production.contains("BackendMemoryTracker")
            && !backend_directml_execution_production.contains("BackendWorkspaceAuthority")
            && !backend_directml_execution_production.contains("BackendResourceRegistry")
            && !backend_directml_execution_production.contains("BackendEventTracker")
            && !backend_directml_execution_production.contains("WorkerBackendSession")
            && !backend_directml_execution_production.contains("WorkspaceDb")
            && !backend_directml_execution_production.contains("SerializableItem")
            && !backend_directml_execution_production.contains("OutputCommitter")
            && backend_mlu_adapter.contains("NativeBackendBindingStatus::unbound")
            && !backend_mlu_adapter.contains("NativeBackendBindingStatus::bound")
            && backend_directml_adapter_production.contains("NativeBackendBindingStatus::unbound")
            && !backend_directml_adapter_production.contains("NativeBackendBindingStatus::bound")
            && external_mlu_runtime_sites.is_empty()
            && external_directml_runtime_sites.is_empty()
            && mlu_execution_consumers_are_exact
            && directml_execution_consumers_are_exact
            && certification_constructor_sites_are_exact
            && backend_crates_do_not_own_canonical_services;
    if !mlu_directml_execution_resources_preserve_canonical_owners {
        eprintln!(
            "MLU/DirectML execution ownership: policy={mlu_directml_execution_policy_trace}, \
             legacy_mlu_runtime_sites={external_mlu_runtime_sites:#?}, \
             legacy_directml_runtime_sites={external_directml_runtime_sites:#?}, \
             mlu_execution_consumers={external_mlu_execution_sites:#?}, \
             directml_execution_consumers={external_directml_execution_sites:#?}, \
             exact_certification_sites={certification_constructor_sites_are_exact}, \
             backend_owner_closure={backend_crates_do_not_own_canonical_services}"
        );
    }

    let npu_foundation_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_ffi_npu_abi_and_package_foundation")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_backend_npu")
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        [
                            "comfy-parity-device-foundation-huawei-ascend-npu-comfy-model-0019",
                            "comfy-parity-provision-native-ffi-contracts-huawei-ascend-npu-comfy-model-0019",
                            "comfy-parity-vendor-abi-wave39-ownership-consolidation",
                            "comfy-parity-npu-execution-resource-ownership-consolidation",
                        ]
                        .iter()
                        .all(|required| tasks.iter().any(|task| task.as_str() == Some(required)))
                    })
        });
    let npu_execution_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_ffi_npu_execution_resources")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some(
                    "comfy_backend_npu::NpuExecutionSession and its opaque NpuAllocation, NpuStream, and NpuEvent resources",
                )
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        [
                            "comfy-parity-npu-execution-resource-ownership-consolidation",
                            "comfy-parity-wave39-vendor-execution-ownership-consolidation",
                        ]
                        .iter()
                        .all(|required| tasks.iter().any(|task| task.as_str() == Some(required)))
                    })
        });
    let external_npu_execution_sites =
        production_identifier_occurrences(&sources, "NpuExecutionSession")
            .into_iter()
            .filter(|location| !location.contains("/crates/comfy_backend_npu/"))
            .collect::<Vec<_>>();
    let npu_certification_projection_sites =
        production_source_occurrences(&sources, "pub unsafe fn from_registry_certified_handles(");
    let npu_backend_production = backend_production_source("crates/comfy_backend_npu");
    let npu_abi_foundation_requires_canonical_runtime_certification =
        source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && source_occurrences(&sources, "pub enum NativeBackendBindingStatus").len() == 1
            && npu_foundation_policy_trace
            && npu_execution_policy_trace
            && backend_npu_abi.contains("pub struct AbiManifest")
            && backend_npu_abi.contains("pub const REQUIRED_ASCENDCL_SYMBOLS")
            && backend_npu_loader.contains("pub struct SignedPackageRoot<'certificate>")
            && backend_npu_loader.contains("pub struct RegistryCertifiedNpuImages")
            && backend_npu_loader.contains("pub unsafe fn from_registry_certified_handles(")
            && backend_npu_loader.contains("_certification: Arc<dyn Any + Send + Sync>")
            && backend_npu_loader.contains("pub(crate) struct OwnedNpuCore")
            && backend_npu_loader.contains("struct NativeNpuSymbols {")
            && backend_npu_loader.contains("unsafe impl Send for OwnedNpuCore")
            && backend_npu_loader.contains("impl Drop for OwnedNpuCore")
            && !declaration_derives_trait(&backend_npu_loader, "struct NativeNpuSymbols", "Clone")
            && !backend_npu_loader.contains("impl Clone for NativeNpuSymbols")
            && !declaration_derives_trait(
                &backend_npu_loader,
                "pub(crate) struct OwnedNpuCore",
                "Clone",
            )
            && !backend_npu_loader.contains("impl Clone for OwnedNpuCore")
            && !backend_npu_loader.contains("impl Clone for SignedPackageRoot")
            && !backend_npu_loader.contains("impl Clone for RegistryCertifiedNpuImages")
            && !backend_npu_loader.contains("NativeFfiRegistry::")
            && !backend_npu_loader.contains("NativeBackendBindingStatus::bound")
            && !backend_npu_loader.contains("BackendCapabilityMatrix")
            && !backend_npu_loader.contains("TensorBackend")
            && backend_npu_execution_production.contains("pub struct NpuExecutionSession")
            && backend_npu_execution_production.contains("pub fn from_registry_certified_images(")
            && backend_npu_execution_production.contains("#[cfg(feature = \"test-support\")]")
            && backend_npu_execution_production.contains("pub fn for_test_harness(")
            && backend_npu_execution_production.contains("use comfy_types::CancellationToken")
            && backend_npu_execution_production.contains("session: Arc<Session>")
            && backend_npu_execution_production.contains("state: Mutex<RuntimeState>")
            && runtime_trust_production
                .matches("pub struct NpuPackageVerificationKey")
                .count()
                == 1
            && runtime_trust.contains("NPU_PACKAGE_SIGNATURE_DOMAIN")
            && runtime_trust_production.contains("pub fn new_dependency(")
            && runtime_trust_production.contains("pub fn authorize_dependency(")
            && runtime_npu_ffi_production
                .find("verification_key.verify_package")
                .zip(runtime_npu_ffi_production.find("let catalog: NpuFfiContractCatalogDto"))
                .is_some_and(|(verification, catalog)| verification < catalog)
            && runtime_npu_ffi_production.contains("NativeFfiRegistry::new")
            && runtime_npu_ffi_production.contains("NativeFfiContract::new_dependency")
            && runtime_npu_ffi_production.contains("authorize_dependency(")
            && runtime_npu_ffi_production.contains("capture_native_package(")
            && runtime_npu_ffi_production.contains("validate_native_package_coverage(")
            && runtime_npu_ffi_production.contains("capture_native_library_image(")
            && runtime_npu_ffi_production
                .contains("NpuExecutionSession::from_registry_certified_images")
            && runtime_npu_ffi_production.contains("struct NativeNpuLibraryHandles")
            && !runtime_npu_ffi_production.contains("libc::O_NOFOLLOW")
            && !runtime_npu_ffi_production.contains("libc::memfd_create")
            && !runtime_npu_ffi_production.contains("libc::F_ADD_SEALS")
            && !runtime_npu_ffi_production.contains("PluginVerificationKey")
            && production_source_occurrences(&sources, "pub struct NpuExecutionSession").len() == 1
            && production_source_occurrences(&sources, "pub struct NpuAllocation").len() == 1
            && production_source_occurrences(&sources, "pub struct NpuStream").len() == 1
            && production_source_occurrences(&sources, "pub struct NpuEvent").len() == 1
            && production_source_occurrences(&sources, "pub struct RegistryCertifiedNpuImages")
                .len()
                == 1
            && production_source_occurrences(&sources, "pub struct NpuDeviceProperties").len() == 1
            && production_source_occurrences(&sources, "pub enum NpuElementType").len() == 1
            && production_source_occurrences(&sources, "pub enum NpuExecutionError").len() == 1
            && production_source_occurrences(&sources, "pub(crate) struct OwnedNpuCore").len() == 1
            && !backend_npu_execution_production.contains("pub struct CancellationToken")
            && !backend_npu_execution_production.contains("*mut c_void")
            && !backend_npu_execution_production.contains("NativeFfiRegistry::")
            && !backend_npu_execution_production.contains("NativeBackendBindingStatus::bound")
            && !backend_npu_execution_production.contains("BackendCapabilityMatrix")
            && !backend_npu_execution_production.contains("BackendMemoryTracker")
            && !backend_npu_execution_production.contains("BackendWorkspaceAuthority")
            && !backend_npu_execution_production.contains("BackendResourceRegistry")
            && !backend_npu_execution_production.contains("BackendEventTracker")
            && !backend_npu_execution_production.contains("WorkerBackendSession")
            && !backend_npu_execution_production.contains("WorkspaceDb")
            && !backend_npu_execution_production.contains("SerializableItem")
            && !backend_npu_execution_production.contains("OutputCommitter")
            && !backend_npu_execution_production.contains("AssetService")
            && !backend_npu_execution_production.contains("PermissionPolicy")
            && !backend_npu_execution_production.contains("ExecutionQueue")
            && production_source_occurrences(&sources, "RetainedNpuLibraryHandles").is_empty()
            && production_source_occurrences(&sources, "AscendClSymbols").is_empty()
            && production_source_occurrences(&sources, "AscendClSession").is_empty()
            && production_source_occurrences(&sources, "pub struct NpuContext").is_empty()
            && production_source_occurrences(&sources, "NpuPendingCopy").is_empty()
            && exact_occurrence_files(
                &root,
                &external_npu_execution_sites,
                &[
                    "crates/comfy_runtime/src/native_ffi_npu.rs",
                    "crates/comfy_tensor/src/backends/huawei_ascend_npu_comfy_model_0019.rs",
                ],
            )?
            && tensor_npu_adapter.contains("pub struct NpuTensorBackend")
            && tensor_npu_adapter.contains("pub fn from_certified_runtime(")
            && !tensor_npu_adapter.contains("for_test_harness(")
            && exact_occurrence_files(
                &root,
                &npu_certification_projection_sites,
                &["crates/comfy_backend_npu/src/loader.rs"],
            )?
            && forbidden_backend_owner_symbols
                .iter()
                .all(|symbol| !npu_backend_production.contains(symbol))
            && backend_npu_adapter_production.contains("NativeBackendBindingStatus::unbound")
            && !backend_npu_adapter_production.contains("NativeBackendBindingStatus::bound")
            && backend_npu_packager.contains("vendor runtime payload was accepted")
            && backend_npu_packager.contains("separately reviewed bounded regular file")
            && backend_npu_packager.contains("package-coverage.sha256")
            && backend_npu_package_policy.contains("\"redistributes_vendor_runtime\": false")
            && backend_npu_contract_schema.contains("\"required_by\": { \"const\": \"ascendcl\" }")
            && !backend_npu_packager.contains("NativeFfiRegistry::")
            && !backend_npu_packager.contains("ed25519_dalek");

    let moved_corex_tasks = [
        "comfy-parity-device-foundation-iluvatar-corex-ixuca-comfy-model-0020",
        "comfy-parity-native-device-iluvatar-corex-ixuca-comfy-model-0020",
        "comfy-parity-provision-native-ffi-contracts-iluvatar-corex-ixuca-comfy-model-0020",
        "comfy-parity-integrate-device-iluvatar-corex-ixuca-comfy-model-0020",
        "comfy-parity-certify-device-iluvatar-corex-ixuca-comfy-model-0020",
    ];
    let corex_scope_transfer = native_spec_mapping
        .pointer("/scope_transfers/corex_enablement")
        .is_some_and(|transfer| {
            transfer.get("destination_spec").and_then(serde_json::Value::as_str)
                == Some(".agents/specs/comfy-corex-enablement")
                && transfer
                    .get("retained_task_id")
                    .and_then(serde_json::Value::as_str)
                    == Some("comfy-parity-corex-provenance-blocked-structural-foundation")
                && transfer
                    .get("retained_state")
                    .and_then(serde_json::Value::as_str)
                    == Some(
                        "compiled zero-symbol adapter; canonical typed Unbound; no runtime loader or certificate projection",
                    )
                && transfer
                    .get("moved_task_ids")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        tasks.len() == moved_corex_tasks.len()
                            && moved_corex_tasks.iter().all(|required| {
                                tasks.iter().any(|task| task.as_str() == Some(required))
                                    && !task_statuses.contains_key(*required)
                                    && corex_future_task_statuses.get(*required) == Some(&false)
                            })
                    })
        });
    let corex_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_ffi_corex_provenance_and_structural_package_foundation")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_backend_corex")
                && entry.get("known_open_reasons").is_none()
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        [
                            "comfy-parity-corex-provenance-blocked-structural-foundation",
                            "comfy-parity-vendor-abi-wave42-ownership-consolidation",
                        ]
                        .iter()
                        .all(|required| tasks.iter().any(|task| task.as_str() == Some(required)))
                    })
        });
    let corex_structural_foundation_preserves_provenance_blocker =
        source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && source_occurrences(&sources, "pub enum NativeBackendBindingStatus").len() == 1
            && corex_policy_trace
            && corex_scope_transfer
            && backend_corex_abi.contains("ReviewState::BlockedMissingVendorHeaders")
            && backend_corex_abi.contains("!actual.symbols.is_empty()")
            && backend_corex_abi.contains("!self.layouts.is_empty()")
            && backend_corex_abi.contains("missing-evidence ledger differs")
            && backend_corex_loader.contains("pub struct CertifiedCoreXImages<'certificate>")
            && backend_corex_loader.contains("Err(CoreXLoadError::MissingReviewedAbiEvidence")
            && !backend_corex_loader.contains("libc::dlopen")
            && !backend_corex_loader.contains("libc::dlsym")
            && !backend_corex_loader.contains("GetProcAddress")
            && !backend_corex_loader.contains("NativeFfiRegistry::")
            && !backend_corex_loader.contains("NativeBackendBindingStatus::bound")
            && !backend_corex_loader.contains("BackendCapabilityMatrix")
            && !backend_corex_loader.contains("TensorBackend")
            && backend_corex_adapter_production.contains("NativeBackendBindingStatus::unbound")
            && !backend_corex_adapter_production.contains("NativeBackendBindingStatus::bound")
            && backend_corex_packager.contains("blocked-missing-vendor-headers")
            && backend_corex_packager.contains("CoreX structural metadata cannot self-authorize")
            && backend_corex_packager.contains("package-coverage.sha256")
            && backend_corex_package_policy.contains("\"runtime_loading_enabled\": false")
            && backend_corex_packager.contains("\"package_receipt_is_authorization\": False")
            && !backend_corex_packager.contains("NativeFfiRegistry::")
            && !backend_corex_packager.contains("libixrt.so.");

    let xpu_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_ffi_xpu_abi_and_package_foundation")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_backend_xpu")
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        [
                            "comfy-parity-device-foundation-intel-xpu-comfy-model-0021",
                            "comfy-parity-provision-native-ffi-contracts-intel-xpu-comfy-model-0021",
                            "comfy-parity-vendor-abi-wave42-ownership-consolidation",
                            "comfy-parity-xpu-execution-resource-ownership-consolidation",
                        ]
                        .iter()
                        .all(|required| tasks.iter().any(|task| task.as_str() == Some(required)))
                    })
        });
    let xpu_abi_foundation_requires_canonical_runtime_certification =
        source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && source_occurrences(&sources, "pub enum NativeBackendBindingStatus").len() == 1
            && xpu_policy_trace
            && backend_xpu_abi.contains("pub struct AbiManifest")
            && backend_xpu_abi.contains("pub struct HeaderContract")
            && backend_xpu_abi.contains("pub struct ZeCommandQueueGroupProperties")
            && backend_xpu_abi.contains("pub struct ZeDeviceProperties")
            && backend_xpu_abi.contains("pub struct ZeDeviceMemoryProperties")
            && backend_xpu_abi.contains("pub type DnnlVersionFn")
            && backend_xpu_abi.contains("pub type DnnlBinaryPrimitiveDescCreate")
            && backend_xpu_abi.contains("pub type DnnlPrimitiveExecute")
            && backend_xpu_loader.contains("pub struct RegistryCertifiedXpuImages")
            && backend_xpu_loader.contains("pub(crate) struct OwnedXpuCore")
            && backend_xpu_loader.contains("_images: RegistryCertifiedXpuImages")
            && backend_xpu_loader.contains("struct LevelZeroSymbols")
            && backend_xpu_loader.contains("struct OneDnnSymbols")
            && backend_xpu_loader.contains("impl Drop for OwnedXpuCore")
            && backend_xpu_loader.contains("version.major != ONEDNN_MINIMUM_MAJOR")
            && backend_xpu_loader.contains("version.minor < ONEDNN_MINIMUM_MINOR")
            && !backend_xpu_loader.contains("pub struct LevelZeroSymbols")
            && !backend_xpu_loader.contains("pub struct OneDnnSymbols")
            && !backend_xpu_loader.contains("pub struct LevelZeroContext")
            && !backend_xpu_loader.contains("pub struct OneDnnStream")
            && !backend_xpu_loader.contains("NativeFfiRegistry::")
            && !backend_xpu_loader.contains("NativeBackendBindingStatus::bound")
            && !backend_xpu_loader.contains("BackendCapabilityMatrix")
            && !backend_xpu_loader.contains("TensorBackend")
            && backend_xpu_execution.contains("pub struct XpuExecutionSession")
            && backend_xpu_execution.contains("Mutex<RuntimeState>")
            && backend_xpu_execution.contains("use comfy_types::CancellationToken")
            && backend_xpu_execution.contains("cfg(feature = \"test-support\")")
            && backend_xpu_execution.contains("RuntimeState::Fake")
            && !backend_xpu_execution.contains("unsafe {")
            && !backend_xpu_execution.contains("NativeBackendBindingStatus::bound")
            && backend_xpu_adapter_production.contains("NativeBackendBindingStatus::unbound")
            && !backend_xpu_adapter_production.contains("NativeBackendBindingStatus::bound")
            && backend_xpu_packager.contains("reviewed header digest mismatch")
            && backend_xpu_packager.contains("reviewed XPU execution symbol excerpt differs")
            && backend_xpu_packager.contains("execution_verifier_sha256")
            && backend_xpu_packager.contains("package-coverage.sha256")
            && backend_xpu_reviewed_execution
                .contains("schema=sim-comfy-xpu-reviewed-execution-bindings-v1")
            && backend_xpu_reviewed_execution.contains("symbol=dnnl_primitive_execute")
            && backend_xpu_execution_verifier.contains("CHECK_FUNCTION(dnnl_primitive_execute")
            && backend_xpu_execution_verifier.contains("CHECK_LAYOUT(ze_device_properties_t")
            && runtime_trust_production
                .matches("pub struct XpuPackageVerificationKey")
                .count()
                == 1
            && runtime_trust.contains("XPU_PACKAGE_SIGNATURE_DOMAIN")
            && runtime_xpu_ffi_production
                .find("verification_key.verify_package")
                .zip(runtime_xpu_ffi_production.find("let catalog: XpuFfiContractCatalogDto"))
                .is_some_and(|(verification, catalog)| verification < catalog)
            && runtime_xpu_ffi_production.contains("NativeFfiRegistry::new")
            && runtime_xpu_ffi_production.contains("NativeFfiContract::new(")
            && runtime_xpu_ffi_production.contains("capture_native_package(")
            && runtime_xpu_ffi_production.contains("validate_native_package_coverage(")
            && runtime_xpu_ffi_production.contains("capture_native_library_image(")
            && runtime_xpu_ffi_production.contains("RetainedNativeLibraryImage")
            && runtime_xpu_ffi_production
                .contains("RegistryCertifiedXpuImages::from_registry_certified_images")
            && runtime_xpu_ffi_production
                .contains("XpuExecutionSession::from_registry_certified_images")
            && !runtime_xpu_ffi_production.contains("PluginVerificationKey")
            && production_source_occurrences(&sources, "pub struct XpuExecutionSession").len() == 1
            && production_source_occurrences(&sources, "pub struct RegistryCertifiedXpuImages")
                .len()
                == 1
            && production_source_occurrences(&sources, "pub struct XpuAllocation").len() == 1
            && production_source_occurrences(&sources, "pub struct XpuEvent").len() == 1
            && tensor_xpu_adapter.contains("pub struct XpuTensorBackend")
            && tensor_xpu_adapter.contains("pub fn from_certified_session(")
            && !tensor_xpu_adapter.contains("for_test_harness(")
            && backend_xpu_package_policy.contains("\"redistributes_vendor_runtime\": false")
            && backend_xpu_package_policy.contains("\"ffi-contracts-v1.json\"")
            && backend_xpu_package_policy.contains("\"reviewed_execution_bindings_sha256\"")
            && backend_xpu_package_policy
                .contains("\"structural_receipt_is_authorization\": false")
            && backend_xpu_contract_schema.contains("\"additionalProperties\": false")
            && backend_xpu_contract_schema.contains("comfy_backend_xpu::loader")
            && backend_xpu_packager.contains("separately reviewed XPU FFI contract catalog")
            && backend_xpu_packager.contains("ffi_contracts_sha256")
            && !backend_xpu_packager.contains("NativeFfiRegistry::");

    let cuda_policy_trace = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str)
                == Some("native_ffi_cuda_abi_and_package_foundation")
        })
        .is_some_and(|entry| {
            entry
                .get("canonical_owner")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_backend_cuda")
                && entry
                    .get("known_open_reasons")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(Vec::is_empty)
                && entry
                    .get("consolidation_tasks")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|tasks| {
                        [
                            "comfy-parity-device-foundation-nvidia-cuda-comfy-model-0022",
                            "comfy-parity-native-device-nvidia-cuda-comfy-model-0022",
                            "comfy-parity-provision-native-ffi-contracts-nvidia-cuda-comfy-model-0022",
                            "comfy-parity-vendor-abi-wave42-ownership-consolidation",
                        ]
                        .iter()
                        .all(|required| tasks.iter().any(|task| task.as_str() == Some(required)))
                    })
        });
    let cuda_abi_foundation_requires_canonical_runtime_certification =
        source_occurrences(&sources, "pub struct NativeFfiRegistry").len() == 1
            && source_occurrences(&sources, "pub enum NativeBackendBindingStatus").len() == 1
            && cuda_policy_trace
            && backend_cuda_abi.contains("pub struct AbiManifest")
            && backend_cuda_abi.contains("pub struct HeaderContract")
            && backend_cuda_abi.contains("pub type CuLaunchKernel")
            && backend_cuda_abi.contains("pub type CublasLtCreate")
            && backend_cuda_abi.contains("pub type CudnnSetStream")
            && backend_cuda_loader.contains("pub struct RegistryCertifiedCudaImages")
            && backend_cuda_loader.contains("pub(crate) struct OwnedCudaCore")
            && backend_cuda_loader.contains("_images: RegistryCertifiedCudaImages")
            && backend_cuda_loader.contains("struct CudaSymbols")
            && backend_cuda_loader.contains("impl Drop for OwnedCudaCore")
            && backend_cuda_loader.contains("unsafe impl Send for OwnedCudaCore")
            && !backend_cuda_loader.contains("pub struct CudaSymbols")
            && !backend_cuda_loader.contains("pub fn raw_stream")
            && !backend_cuda_loader.contains("pub fn device_pointer")
            && !backend_cuda_loader.contains("pub struct CudaRuntime")
            && !backend_cuda_loader.contains("NativeFfiRegistry::")
            && !backend_cuda_loader.contains("NativeBackendBindingStatus::bound")
            && !backend_cuda_loader.contains("BackendCapabilityMatrix")
            && !backend_cuda_loader.contains("TensorBackend")
            && backend_cuda_execution.contains("pub struct CudaExecutionSession")
            && backend_cuda_execution.contains("Mutex<RuntimeState>")
            && backend_cuda_execution.contains("use comfy_types::CancellationToken")
            && backend_cuda_execution.contains("cfg(feature = \"test-support\")")
            && backend_cuda_execution.contains("RuntimeState::Fake")
            && !backend_cuda_execution.contains("unsafe {")
            && !backend_cuda_execution.contains("NativeBackendBindingStatus::bound")
            && production_source_occurrences(&sources, "pub struct CudaExecutionSession").len()
                == 1
            && production_source_occurrences(&sources, "pub struct RegistryCertifiedCudaImages")
                .len()
                == 1
            && production_source_occurrences(&sources, "pub struct CudaAllocation").len() == 1
            && production_source_occurrences(&sources, "pub struct CudaEvent").len() == 1
            && tensor_cuda_adapter.contains("pub struct CudaTensorBackend")
            && tensor_cuda_adapter.contains("pub fn from_certified_session(")
            && tensor_cuda_adapter.contains("struct RuntimeAdapter(CudaExecutionSession)")
            && !tensor_cuda_adapter.contains("for_test_harness(")
            && runtime_trust_production
                .matches("pub struct CudaPackageVerificationKey")
                .count()
                == 1
            && runtime_trust.contains("CUDA_PACKAGE_SIGNATURE_DOMAIN")
            && runtime_cuda_ffi_production
                .find("verification_key.verify_package")
                .zip(runtime_cuda_ffi_production.find("let catalog: CudaFfiContractCatalogDto"))
                .is_some_and(|(verification, catalog)| verification < catalog)
            && runtime_cuda_ffi_production.contains("NativeFfiRegistry::new")
            && runtime_cuda_ffi_production.contains("NativeFfiContract::new(")
            && runtime_cuda_ffi_production.contains("capture_native_package(")
            && runtime_cuda_ffi_production.contains("validate_native_package_coverage(")
            && runtime_cuda_ffi_production.contains("capture_native_library_image(")
            && runtime_cuda_ffi_production.contains("RetainedNativeLibraryImage")
            && runtime_cuda_ffi_production
                .contains("RegistryCertifiedCudaImages::from_registry_certified_images")
            && runtime_cuda_ffi_production
                .contains("CudaExecutionSession::from_registry_certified_images")
            && !runtime_cuda_ffi_production.contains("PluginVerificationKey")
            && backend_cuda_adapter_production.contains("NativeBackendBindingStatus::unbound")
            && !backend_cuda_adapter_production.contains("NativeBackendBindingStatus::bound")
            && backend_cuda_packager.contains("reviewed raw header digest mismatch")
            && backend_cuda_packager.contains("reviewed header omits versioned symbol mapping")
            && backend_cuda_packager.contains("package-coverage.sha256")
            && backend_cuda_package_policy.contains("\"redistributes_driver\": false")
            && backend_cuda_package_policy.contains("\"ffi-contracts-v1.json\"")
            && backend_cuda_package_policy
                .contains("\"structural_receipt_is_authorization\": false")
            && backend_cuda_contract_schema.contains("\"additionalProperties\": false")
            && backend_cuda_contract_schema.contains("comfy_backend_cuda::loader")
            && backend_cuda_packager.contains("separately reviewed CUDA FFI contract catalog")
            && backend_cuda_packager.contains("ffi_contracts_sha256")
            && !backend_cuda_packager.contains("NativeFfiRegistry::");

    let tensor_resource_policy_trace = policy_concerns.iter().find(|entry| {
        entry.get("concern").and_then(serde_json::Value::as_str)
            == Some("tensor_backend_adapter_resource_registries")
    });
    let metal_execution_policy_trace = policy_concerns.iter().find(|entry| {
        entry.get("concern").and_then(serde_json::Value::as_str)
            == Some("native_ffi_metal_execution_resources")
    });
    let tensor_backend_resource_registries_are_authoritative =
        source_occurrences(&sources, "pub(crate) trait BackendStorage").len() == 1
            && source_occurrences(&sources, "pub(crate) struct BackendResourceRegistry<").len()
                == 1
            && source_occurrences(&sources, "struct BackendEventCursor").len() == 1
            && source_occurrences(&sources, "struct BackendEventState<Event>").len() == 1
            && source_occurrences(&sources, "pub(crate) struct BackendEventTracker<").len() == 1
            && tensor_resource_policy_trace.is_some_and(|entry| {
                entry.get("canonical_owner").and_then(serde_json::Value::as_str)
                    == Some(
                        "comfy_tensor::BackendStorage, Tensor::backend_storage, BackendResourceRegistry, and BackendEventTracker",
                    )
                    && entry
                        .get("known_open_reasons")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(Vec::is_empty)
            })
            && tensor_domain.contains("pub(crate) trait BackendStorage: Any")
            && tensor_domain.contains("pub(crate) fn backend_storage<StorageType: Any>")
            && tensor_domain.contains("self.storage.allocation.as_any().downcast_ref()")
            && tensor_operation.contains("pub(crate) struct BackendResourceRegistry<Resource>")
            && tensor_operation.contains(
                "struct BackendEventCursor {\n    slot: u16,\n    next: u64,\n    completed: u64,\n}",
            )
            && tensor_operation.contains(
                "struct BackendEventState<Event> {\n    pending: BTreeMap<u64, PendingBackendEvent<Event>>,\n    streams: BTreeMap<u64, BackendEventCursor>,\n}",
            )
            && tensor_operation.contains("pub(crate) struct BackendEventTracker<Event>")
            && tensor_operation.contains("state: Arc<Mutex<BackendEventState<Event>>>")
            && !tensor_operation.contains("pending: Mutex<BTreeMap<u64, PendingBackendEvent<Event>>>")
            && !tensor_operation.contains("completed: Mutex<BTreeMap<u64, u64>>")
            && !tensor_operation.contains("completed_owners")
            && tensor_operation.contains("const BACKEND_EVENT_COUNTER_BITS: u32 = 48")
            && tensor_operation.contains("const BACKEND_EVENT_COUNTER_MASK: u64")
            && tensor_operation.contains("fn encode_backend_event_sequence(")
            && tensor_operation.contains("fn decode_backend_event_sequence(")
            && tensor_operation.contains("if state.streams.len() >= self.limit")
            && tensor_operation
                .matches("self.validate_sequence(&state, stream, sequence)?")
                .count()
                == 3
            && tensor_operation.contains("cursor.slot == slot && counter <= cursor.next")
            && tensor_operation.contains(
                ".get(&sequence)\n                .is_some_and(|pending| pending.stream == stream)",
            )
            && tensor_operation.contains("events.complete(StreamId::new(1), first)")
            && tensor_operation.contains("events.complete(StreamId::DEFAULT, 999)")
            && tensor_operation.contains("events.event_for_wait(StreamId::new(1), first)")
            && tensor_operation.contains("events.event_for_wait(StreamId::DEFAULT, 0)")
            && tensor_operation.contains("events.complete(StreamId::DEFAULT, 0)")
            && tensor_operation.contains("events.cancel(StreamId::new(1), third)")
            && tensor_operation.contains(
                "backend_event_watermark_preserves_old_fences_with_bounded_stream_provenance",
            )
            && tensor_operation.contains("events.complete(StreamId::DEFAULT, first)?.is_empty()")
            && tensor_operation.contains("drop(retired)")
            && tensor_operation.contains("drops_while_locked")
            && tensor_operation.contains("backend_resource_registries_own_bounds_completion_and_drop_transitions")
            && tensor_rocm_backend_production.contains(".backend_storage::<RocmStorage>()")
            && tensor_rocm_backend_production
                .contains("streams: BackendResourceRegistry<StreamAdapter>")
            && tensor_rocm_backend_production
                .contains("events: BackendEventTracker<EventAdapter>")
            && tensor_rocm_backend_production.contains("let retired = self.events.complete")
            && tensor_rocm_backend_production.contains("drop(retired)")
            && !tensor_rocm_backend_production.contains("Weak<RocmStorageInner>")
            && !tensor_rocm_backend_production.contains("storages: Mutex")
            && !tensor_rocm_backend_production.contains("completed_events")
            && !tensor_rocm_backend_production.contains("BackendEventCursor")
            && !tensor_rocm_backend_production.contains("BACKEND_EVENT_COUNTER_BITS")
            && !tensor_rocm_backend_production.contains("streams: Mutex")
            && !tensor_rocm_backend_production.contains("events: Mutex")
            && tensor_metal_backend.contains("impl BackendStorage for MetalStorage")
            && tensor_metal_backend.contains(".backend_storage::<MetalStorage>()")
            && tensor_metal_backend.contains("streams: BackendResourceRegistry<MetalStream>")
            && tensor_metal_backend.contains("events: BackendEventTracker<MetalEvent>")
            && tensor_metal_backend.contains("self.events.record_with(context.stream, create)")
            && tensor_metal_backend.contains("drop(self.events.complete(event.stream, event.sequence)?)")
            && !tensor_metal_backend.contains("BTreeMap")
            && !tensor_metal_backend.contains("struct BackendEventCursor")
            && !tensor_metal_backend.contains("struct CancellationToken")
            && !tensor_metal_backend.contains("NativeBackendBindingStatus::bound")
            && !backend_metal_execution.contains("BackendEventState")
            && !backend_metal_execution.contains("completed_events")
            && !backend_metal_execution.contains("BackendEventCursor")
            && !backend_metal_execution.contains("BACKEND_EVENT_COUNTER_BITS")
            && !backend_metal_execution.contains("events: Mutex")
            && !backend_metal_execution.contains("streams: Mutex");

    let metal_execution_catalog: serde_json::Value =
        serde_json::from_str(&backend_metal_execution_catalog)?;
    let metal_execution_resources_have_one_opaque_owner =
        source_occurrences(&sources, "pub struct MetalRuntime").len() == 1
            && source_occurrences(&sources, "pub struct MetalAllocation").len() == 1
            && source_occurrences(&sources, "pub struct MetalStream").len() == 1
            && source_occurrences(&sources, "pub struct MetalEvent").len() == 1
            && metal_execution_policy_trace.is_some_and(|entry| {
                entry.get("canonical_owner").and_then(serde_json::Value::as_str)
                    == Some(
                        "comfy_backend_metal::MetalRuntime and its opaque MetalAllocation, MetalStream, and MetalEvent handles",
                    )
                    && entry
                        .get("known_open_reasons")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(Vec::is_empty)
            })
            && backend_metal_execution.contains("struct CertifiedInputs")
            && backend_metal_execution.contains("pub unsafe fn from_certified_metallibs")
            && backend_metal_execution.contains("readiness_metallib: Arc<[u8]>")
            && backend_metal_execution.contains("tensor_ops_metallib: Arc<[u8]>")
            && backend_metal_execution.contains("_certified: Arc<CertifiedInputs>")
            && backend_metal_execution.contains("pub fn for_test_harness(")
            && tensor_metal_backend.contains("pub fn from_certified_runtime(")
            && tensor_metal_backend.contains("runtime: MetalRuntime")
            && tensor_metal_backend.contains("BackendWorkspaceAuthority::new(effective_limit)")
            && tensor_metal_backend.contains("metal_capability_matrix(device, properties)")
            && tensor_metal_backend.contains("Vec::with_capacity(12)")
            && tensor_metal_backend.contains("OperationSupport::record_event()")
            && !tensor_metal_backend.contains("pub fn from_test")
            && !tensor_metal_backend.contains("NativeFfiRegistry")
            && !tensor_metal_backend.contains("NativeBackendBindingStatus::bound")
            && backend_metal_execution.contains("requires_explicit_synchronization")
            && backend_metal_execution.contains("opaque_identity_and_storage_synchronization_rules_are_platform_independent")
            && backend_metal_execution.contains("const MAXIMUM_DEVICE_NAME_BYTES: usize = 256")
            && backend_metal_execution.contains("const MAXIMUM_DIAGNOSTIC_BYTES: usize = 1_024")
            && backend_metal_execution.contains("pub struct MetalDiagnostic(String)")
            && !backend_metal_execution.contains("pub struct MetalDiagnostic(pub String)")
            && backend_metal_execution.contains("UnsupportedTarget { target: MetalDiagnostic }")
            && backend_metal_execution.contains("InvalidAbi { reason: MetalDiagnostic }")
            && backend_metal_execution.contains("reason: MetalDiagnostic::bounded(reason)")
            && backend_metal_execution.contains("name: bounded_device_name(probed.name)?")
            && backend_metal_execution
                .contains("fn device_names_and_public_diagnostics_are_bounded()")
            && backend_metal_execution_abi.contains("sim-comfy-metal-execution-v1")
            && backend_metal_execution_abi
                .contains(
                    "const RESOURCE_SELECTORS: [(&str, &str, &str, ReturnNullability); 29]",
                )
            && backend_metal_execution_abi
                .contains("const REVIEWED_HEADERS: [(&str, &str); 12]")
            && backend_metal_execution_abi
                .contains("pub reviewed_headers: Vec<ReviewedHeaderContract>")
            && backend_metal_execution_abi
                .contains("resource selector names, exact encodings, or return nullability differ")
            && backend_metal_execution_abi.contains("pub return_nullability: ReturnNullability")
            && backend_metal_execution_abi.contains("reviewed SDK header identities differ")
            && backend_metal_execution_abi.contains("fn is_sha256")
            && backend_metal_execution_verifier.contains("objc_getProtocol")
            && backend_metal_execution_verifier.contains("protocol_getMethodDescription")
            && backend_metal_execution_verifier.contains("method_getTypeEncoding")
            && backend_metal_execution_verifier
                .contains("strcmp(actual_encoding, expected_encoding.UTF8String)")
            && backend_metal_execution_verifier.contains("expected_return_nullability")
            && backend_metal_execution.contains("trait NullableMetalResourceCalls")
            && backend_metal_execution.contains("fn non_null_resource<Resource>")
            && backend_metal_execution
                .contains("nullable_sdk_resources_fail_before_foreign_type_construction")
            && !backend_metal_execution.contains("new_buffer(requested, options)")
            && !backend_metal_execution
                .contains("new_command_queue_with_max_command_buffer_count(")
            && backend_metal_execution_packager.contains("verify-execution-bindings.m")
            && backend_metal_execution_packager
                .contains("\"$verify_directory/verify-execution-bindings\" \"$contract\"")
            && !backend_metal_execution.contains("use comfy_runtime")
            && !backend_metal_execution.contains("NativeFfiRegistry::")
            && !backend_metal_execution.contains("pub use metal")
            && !backend_metal_adapter.contains("pub use metal")
            && backend_metal_execution_package_policy
                .contains("\"runtime_compilation_forbidden\": true")
            && backend_metal_execution_package_policy
                .contains("\"runtime_authorization_from_structure\": false")
            && backend_metal_execution_packager.contains("tensor_ops.metallib")
            && backend_metal_execution_packager.contains("package-coverage.sha256")
            && !backend_metal_execution_packager.contains("NativeFfiRegistry")
            && !backend_metal_execution_packager.contains("verify_signature")
            && metal_execution_catalog
                .get("planned_capability_rows")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|rows| rows.len() == 12)
            && metal_execution_catalog
                .pointer("/domain_owners/metal_execution_resources")
                .and_then(serde_json::Value::as_str)
                == Some("comfy_backend_metal::MetalRuntime")
            && !gpui_metal_renderer.contains("comfy_backend_metal")
            && !media_owner.contains("comfy_backend_metal");

    let raft_bilinear_adapter = model_vision
        .split_once("fn sample_bilinear(")
        .and_then(|(_, remainder)| remainder.split_once("fn index_correlation_pyramid("))
        .map(|(implementation, _)| implementation);
    let task_68_bilinear_sampling_has_one_owner =
        source_occurrences(&sources, "pub fn checked_bilinear_weights(")
            .as_slice()
            .first()
            .is_some_and(|location| {
                tensor_external_kernel_part_one
                    .matches("pub fn checked_bilinear_weights(")
                    .count()
                    == 1
                    && source_occurrences(&sources, "pub fn checked_bilinear_weights(").len() == 1
                    && location.contains("crates/comfy_tensor/src/ops/external_tensor_kernel_01.rs")
            })
            && raft_bilinear_adapter.is_some_and(|implementation| {
                implementation.contains("checked_bilinear_weights(")
                    && implementation.contains("NativeBilinearBoundary::ZeroPadding")
                    && !implementation.contains(".floor()")
                    && !implementation.contains("sample_y >=")
            });
    let task_68_color_traversal_has_one_owner = tensor_external_kernel_part_two
        .matches("for pixel in 0..pixels")
        .count()
        == 1
        && tensor_external_kernel_part_two.contains("fn map_color_inputs<const INPUTS: usize>(")
        && tensor_external_kernel_part_two.contains("fn map_color(")
        && tensor_external_kernel_part_two.contains("fn map_color_pair(")
        && tensor_external_kernel_part_two
            .split_once("pub fn rgb_to_lab_jvp_with_context_exact_native(")
            .and_then(|(_, remainder)| {
                remainder.split_once("pub fn rgb_to_lab_vjp_with_context_exact_native(")
            })
            .is_some_and(|(jvp, _)| jvp.contains("map_color_pair("))
        && tensor_external_kernel_part_two
            .split_once("pub fn rgb_to_lab_vjp_with_context_exact_native(")
            .and_then(|(_, remainder)| remainder.split_once("fn rgb_to_ycbcr_value("))
            .is_some_and(|(vjp, _)| vjp.contains("map_color_pair("));
    let task_68_external_kernel_contexts_map_canonical_cancellation =
        !tensor_external_kernel_part_two.contains("context.check()?")
            && tensor_external_kernel_part_two
                .matches("context.cancellation.check()?")
                .count()
                >= 26;
    let task_68_normalization_delegates_functional_owner = model_vision
        .split_once("fn batch_normalize(")
        .and_then(|(_, remainder)| remainder.split_once("fn instance_normalize("))
        .is_some_and(|(batch_normalization, _)| {
            batch_normalization.contains("batch_norm_with_context_exact_native(")
                && !batch_normalization.contains("variances.fill(")
        })
        && model_vision
            .split_once("fn instance_normalize(")
            .and_then(|(_, remainder)| remainder.split_once("fn adaptive_average_pool("))
            .is_some_and(|(instance_normalization, _)| {
                instance_normalization.contains("group_norm_with_context_exact_native(")
                    && !instance_normalization.contains(".sum::<f32>()")
            });
    let task_68_model_state_delegates_native_module = model_vision
        .contains("struct NativeModuleSlot {")
        && model_vision.contains("module: NativeModule,")
        && model_vision.contains("slot.module.load_dense_parameters(weight, bias)?;")
        && !model_vision.contains("zero_weight: bool")
        && !model_vision.contains("bias_values: Option<Vec<f32>>")
        && !model_vision.contains("zero_valued_weights:")
        && !model_vision.contains("final_feature_eval_values:")
        && !model_vision.contains("classifier_bias:")
        && !model_vision.contains("flow_delta_bias:")
        && model_native_ops.contains("pub fn forward_if_dense_weight_is_zero_with_context(")
        && model_vision.contains(".forward_if_dense_weight_is_zero_with_context(");
    let task_68_rgb8_boundary_is_a_focused_tensor_adapter =
        source_occurrences(&sources, "pub struct Rgb8ImageTensor {")
            .as_slice()
            .first()
            .is_some_and(|location| {
                source_occurrences(&sources, "pub struct Rgb8ImageTensor {").len() == 1
                    && location.contains("crates/comfy_tensor/src/image_ops.rs")
            })
            && tensor_image_ops.contains("pub struct Rgb8ImageTensor {\n    tensor: Tensor,")
            && tensor_image_ops.contains("pub fn from_logical_chw(")
            && tensor_image_ops.contains("Self::from_tensor(output)")
            && tensor_external_kernel_part_two
                .contains("pub fn to_pil_image_with_context_exact_native(")
            && tensor_external_kernel_part_two.contains("Rgb8ImageTensor::from_logical_chw(");
    let task_69_adapters_delegate_canonical_owners = tensor_external_kernel_part_three
        .contains("map_color(")
        && tensor_external_kernel_part_three.contains("map_color_pair(")
        && tensor_external_kernel_part_three.contains("NativeMorphologyOperation::BottomHat")
        && tensor_external_kernel_part_three.contains("validate_audio_parameters(")
        && tensor_external_kernel_part_three.contains("biquad_with_context_exact_native(")
        && tensor_external_kernel_part_three
            .contains("mel_scale_project_with_context_exact_native(")
        && tensor_external_kernel_part_three.contains("normalize_with_context_exact_native(")
        && tensor_external_kernel_part_three.contains("image: &Rgb8ImageTensor")
        && tensor_external_kernel_part_three.contains("image.as_u8_slice()?")
        && tensor_external_kernel_part_three
            .contains("image_bytes_to_tensor_with_context_exact_native(")
        && !tensor_external_kernel_part_three.contains("fn map_color_inputs<")
        && !tensor_external_kernel_part_three.contains("fn mel_filter_bank(")
        && !tensor_external_kernel_part_three.contains("fn frequency_to_mel(")
        && !tensor_external_kernel_part_three.contains("fn mel_to_frequency(")
        && tensor_external_kernel_part_one
            .matches("fn mel_filter_bank(")
            .count()
            == 1
        && tensor_external_kernel_part_one
            .matches("fn frequency_to_mel(")
            .count()
            == 1
        && tensor_external_kernel_part_one
            .matches("fn mel_to_frequency(")
            .count()
            == 1
        && source_occurrences(&sources, "pub fn box_convert_with_context_exact_native(").len() == 1
        && source_occurrences(&sources, "pub enum NativeTensorTransform").len() == 1;
    let task_69_external_kernel_contexts_map_canonical_cancellation =
        !tensor_external_kernel_part_three.contains("context.check()?")
            && tensor_external_kernel_part_three
                .matches("context.cancellation.check()?")
                .count()
                >= 21;
    let task_68_deform_sampling_has_one_owner =
        source_occurrences(&sources, "struct NativeDeformConv2dPlan {").len() == 1
            && source_occurrences(&sources, "pub fn deform_conv2d_with_context_exact_native(")
                .len()
                == 1
            && source_occurrences(
                &sources,
                "pub fn deform_conv2d_vjp_with_context_exact_native(",
            )
            .len()
                == 1
            && source_occurrences(
                &sources,
                "pub fn deform_conv2d_jvp_with_context_exact_native(",
            )
            .len()
                == 1
            && tensor_external_kernel_part_two
                .matches("checked_bilinear_weights(")
                .count()
                == 3
            && tensor_external_kernel_part_two
                .matches("NativeDeformConv2dPlan::checked(")
                .count()
                == 3;
    let cartesian_product_traversal_has_one_owner =
        source_occurrences(&sources, "pub fn cartesian_prod_with_context_exact_native(").len() == 1
            && tensor_operation_part_twenty_three
                .contains("pub fn cartesian_prod_with_context_exact_native(")
            && tensor_operation_part_twenty_three.contains("let mut suffix_products")
            && tensor_operation_part_twenty_three.contains("input.element_bytes(&[input_index])")
            && tensor_operation_part_twenty_three.contains("upload_bytes(");
    let rmsprop_state_machine_has_one_owner =
        source_occurrences(&sources, "pub struct NativeRmsprop {").len() == 1
            && tensor_operation_part_twenty_three.contains("pub struct NativeRmsprop {")
            && tensor_operation_part_twenty_three
                .contains("context.cancellation.check()?;\n        if parameters.len()")
            && tensor_operation_part_twenty_three
                .contains("parameters.iter_mut().zip(staged_parameters)")
            && tensor_operation_part_twenty_three
                .contains("square_average.commit_in_place(staged)?")
            && tensor_operation_part_twenty_three_resolution
                .contains("NativeRmsprop::new_with_context_exact_native")
            && !tensor_operation_part_twenty_three_resolution
                .contains("NativeRmsprop::new_exact_native")
            && tensor_operation_part_twenty_three_tests
                .contains("task_66_resolution_slice_seals_both_unique_contracts")
            && tensor_operation_part_twenty_three_tests
                .contains("task_66_invalid_inputs_and_cancellation_fail_closed");
    let cumulative_scan_traversal_has_one_owner =
        source_occurrences(&sources, "pub enum NativeCumulativeOperation").len() == 1
            && source_occurrences(&sources, "pub fn cumulative_with_context_exact_native(").len()
                == 1
            && tensor_operation_part_ten.contains("NativeCumulativeOperation::Sum")
            && tensor_operation_part_twenty_one.contains("canonical_cumulative_with_context(")
            && tensor_operation_part_twenty_one.contains("NativeCumulativeOperation::Product")
            && tensor_operation_part_twenty_one_tests.contains("let zero_input")
            && tensor_operation_part_twenty_one_tests.contains("&[1.0, 28.0, 0.0]");
    let execution_context_has_one_owner =
        source_occurrences(&sources, "pub struct ExecutionContext").len() == 1
            && source_occurrences(&sources, "pub struct NativeStreamRegistry").len() == 1
            && tensor_operation_part_seventeen.contains("xpu_synchronize_exact_native(")
            && tensor_operation_part_seventeen.contains("synchronize_device_exact_native(")
            && !tensor_operation_part_seventeen.contains("backend.record_event(execution)")
            && tensor_operation.contains("pub fn synchronize_device_exact_native(")
            && tensor_operation.contains("let event = backend.record_event(execution)?;")
            && tensor_operation.contains("backend.wait_event(event, execution)?;");
    let primitive_operation_semantics_have_one_owner =
        source_occurrences(&sources, "pub enum UnaryOperation").len() == 1
            && source_occurrences(&sources, "pub enum BinaryOperation").len() == 1
            && source_occurrences(&sources, "pub trait TensorBackend").len() == 1
            && tensor_operation_part_fourteen
                .contains("mul_with_context_exact_native as canonical_mul")
            && tensor_operation_part_fourteen
                .contains("abs_with_context_exact_native as canonical_abs")
            && tensor_operation_part_fourteen
                .contains("concatenate_with_context_exact_native as canonical_concatenate")
            && !tensor_operation_part_fourteen.contains("log1p_with_context_exact_native");
    let flat_unique_semantics_have_one_owner =
        source_occurrences(&sources, "pub struct UniqueResult").len() == 1
            && source_occurrences(&sources, "fn unique_flat_with_context_exact_native(").len() == 1
            && tensor_operation_part_six.contains("entries.sort_by(")
            && tensor_operation_part_ten.contains(
                "pub use crate::generated_elementwise_or_runtime_operation_06::UniqueResult",
            )
            && tensor_operation_part_ten
                .contains("return Ok(unique_flat_with_context_exact_native(")
            && tensor_operation_part_ten
                .find("return Ok(unique_flat_with_context_exact_native(")
                .zip(tensor_operation_part_ten.find("order.sort_by("))
                .is_some_and(|(adapter, dimension_sort)| adapter < dimension_sort);
    let task50_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, &["pub struct ", "NativeSgd"].concat()).len() == 1
            && source_occurrences(
                &sources,
                &["pub struct ", "TensorPrintOptions", " {"].concat(),
            )
            .len()
                == 1
            && source_occurrences(&sources, &["pub struct ", "NativeModule"].concat()).len() == 1
            && source_occurrences(&sources, &["pub struct ", "ExecutionContext"].concat()).len()
                == 1
            && source_occurrences(&sources, &["pub struct ", "CancellationToken"].concat()).len()
                == 1
            && tensor_operation_part_seven.contains("canonical_expm1_with_context(")
            && tensor_operation_part_seven.contains("canonical_tanh_with_context(")
            && tensor_operation_part_seven.contains("UnaryOperation::Log1p")
            && tensor_operation_part_seven.contains("UnaryOperation::ReciprocalSquareRoot")
            && tensor_operation_part_seven.contains("Ok(execution.stream)")
            && !tensor_operation_part_seven.contains("thread_local!")
            && !tensor_operation_part_seven.contains("OnceLock")
            && model_native_ops.contains("module.register_weight_norm_exact_native(");
    let task51_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, &["pub struct ", "AutogradTape"].concat()).len() == 1
            && source_occurrences(&sources, &["pub trait ", "GradientReducer"].concat()).len() == 1
            && source_occurrences(
                &sources,
                &["pub fn ", "autograd_grad_exact_native("].concat(),
            )
            .len()
                == 1
            && source_occurrences(
                &sources,
                &["pub fn ", "concatenate_with_context_exact_native("].concat(),
            )
            .len()
                == 1
            && source_occurrences(
                &sources,
                &["pub fn ", "index_select_with_context_exact_native("].concat(),
            )
            .len()
                == 1
            && source_occurrences(
                &sources,
                &["pub struct ", "MedianDimensionResult", " {"].concat(),
            )
            .len()
                == 1
            && source_occurrences(
                &sources,
                &["pub fn ", "rot90_with_context_exact_native("].concat(),
            )
            .len()
                == 1
            && source_occurrences(
                &sources,
                &["pub fn ", "square_with_context_exact_native("].concat(),
            )
            .len()
                == 1
            && tensor_operation_part_eight.contains("tape.backward(seeds, reducer, cancellation)")
            && tensor_operation_part_eight
                .contains("capability.device().kind() == DeviceKind::Metal")
            && tensor_operation_part_eight.contains("device.kind() != DeviceKind::Mlu")
            && !tensor_operation_part_eight.contains("thread_local!")
            && !tensor_operation_part_eight.contains("OnceLock")
            && tensor_operation_part_fourteen
                .contains("concatenate_with_context_exact_native as canonical_concatenate")
            && tensor_operation_part_sixteen
                .contains("square_with_context_exact_native as canonical_square_with_context");
    let task52_foundational_boundaries_have_one_owner = source_occurrences(
        &sources,
        &["pub enum ", "NativeBitwiseOperation", " {"].concat(),
    )
    .len()
        == 1
        && source_occurrences(
            &sources,
            &["pub fn ", "bitwise_binary_with_context_exact_native("].concat(),
        )
        .len()
            == 1
        && source_occurrences(
            &sources,
            &["pub trait ", "TorchArchiveLoader", " {"].concat(),
        )
        .len()
            == 1
        && source_occurrences(
            &sources,
            &["pub struct ", "TorchArchiveFileLoader"].concat(),
        )
        .len()
            == 1
        && source_occurrences(&sources, &["pub struct ", "NativeAdamW"].concat()).len() == 1
        && tensor_operation_part_nine.contains("BinaryOperation::Multiply")
        && tensor_operation_part_nine.contains("BinaryOperation::Power")
        && tensor_operation_part_nine.contains(
            "generated_elementwise_or_runtime_operation_03::sigmoid_with_context_exact_native",
        )
        && tensor_operation_part_nine.contains(
            "generated_elementwise_or_runtime_operation_02::adamw_with_context_exact_native",
        )
        && tensor_operation_part_nine.contains("device.kind() != DeviceKind::Npu")
        && model_formats.contains("impl TorchArchiveLoader for TorchArchiveFileLoader<'_>")
        && model_formats.contains("parse_model_file(path, limits, context.cancellation)")
        && model_formats.contains("validate_pytorch_rebuild")
        && model_restricted_pickle.contains("pub fn parse_restricted_pickle_cancellable(")
        && !tensor_operation_part_nine.contains("thread_local!")
        && !tensor_operation_part_nine.contains("OnceLock");
    let task53_foundational_boundaries_have_one_owner = source_occurrences(
        &sources,
        &["pub enum ", "NativeCumulativeOperation", " {"].concat(),
    )
    .len()
        == 1
        && source_occurrences(
            &sources,
            &["pub fn ", "cumulative_with_context_exact_native("].concat(),
        )
        .len()
            == 1
        && source_occurrences(&sources, &["pub struct ", "IntegerInfo", " {"].concat()).len() == 1
        && source_occurrences(&sources, &["pub struct ", "AutocastPolicy", " {"].concat()).len()
            == 1
        && source_occurrences(&sources, &["pub struct ", "MemoryTopology", " {"].concat()).len()
            == 1
        && source_occurrences(
            &sources,
            &["fn ", "source_device_count_exact_native("].concat(),
        )
        .len()
            == 1
        && source_occurrences(
            &sources,
            &["fn ", "device_empty_cache_exact_native("].concat(),
        )
        .len()
            == 1
        && tensor_operation_part_ten.contains("BinaryOperation::FloatingRemainder")
        && tensor_operation_part_ten.contains("UnaryOperation::LogarithmBaseTwo")
        && tensor_operation_part_ten.contains("Ok(dtype.integer_info()?)")
        && tensor_operation_part_ten.contains("policy.cache_enabled()")
        && tensor_operation_part_ten.contains("unique_flat_with_context_exact_native(")
        && worker_memory_modes.contains("mlu_device_count_exact_native(")
        && worker_memory_modes.contains("source_device_count_exact_native(")
        && worker_memory_modes.contains("xpu_empty_cache_exact_native(")
        && worker_memory_modes.contains("device_empty_cache_exact_native(")
        && !tensor_operation_part_ten.contains("thread_local!")
        && !tensor_operation_part_ten.contains("OnceLock");
    let task54_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, &["pub struct ", "NativeAdam", " {"].concat()).len() == 1
            && source_occurrences(&sources, &["pub enum ", "GradientMode", " {"].concat()).len()
                == 1
            && source_occurrences(
                &sources,
                &["pub struct ", "BackendCapabilityMatrix", " {"].concat(),
            )
            .len()
                == 1
            && source_occurrences(
                &sources,
                &["pub struct ", "MemoryPlacementInventory", " {"].concat(),
            )
            .len()
                == 1
            && source_occurrences(
                &sources,
                &["pub struct ", "MemoryAccountingSnapshot", " {"].concat(),
            )
            .len()
                == 1
            && source_occurrences(&sources, &["pub struct ", "CancellationToken"].concat()).len()
                == 1
            && tensor_operation_part_eleven.contains("canonical_zero_in_place_with_context")
            && tensor_operation_part_eleven.contains("round_method_with_context_exact_native")
            && tensor_operation_part_eleven.contains("clamp_with_context_exact_native")
            && tensor_operation_part_eleven.contains("BinaryOperation::Maximum")
            && tensor_operation_part_eleven.contains("UnaryOperation::Signum")
            && tensor_operation_part_eleven.contains("inner: NativeAdamW")
            && tensor_operation_part_eleven.contains("NativeAdamW::new_with_context_exact_native")
            && tensor_operation_part_eleven
                .matches("context.cancellation.check()?")
                .count()
                >= 16
            && tensor_operation_part_eleven.contains("cancellation.check()?")
            && worker_memory_modes.contains("xpu_memory_stats_exact_native(")
            && worker_memory_modes.contains("source_memory_stats_exact_native(")
            && worker_memory_modes.contains("npu_empty_cache_exact_native(")
            && worker_memory_modes.contains("device_empty_cache_exact_native(")
            && !tensor_operation_part_eleven.contains("thread_local!")
            && !tensor_operation_part_eleven.contains("OnceLock");
    let task55_foundational_boundaries_have_one_owner = source_occurrences(
        &sources,
        &["pub struct ", "TensorDescriptor", " {"].concat(),
    )
    .len()
        == 1
        && source_occurrences(&sources, &["pub enum ", "DType", " {"].concat()).len() == 1
        && source_occurrences(&sources, &["pub struct ", "AutocastPolicy", " {"].concat()).len()
            == 1
        && source_occurrences(
            &sources,
            &["pub struct ", "NativeStreamRegistry", " {"].concat(),
        )
        .len()
            == 1
        && source_occurrences(
            &sources,
            &["pub struct ", "BackendCapabilityMatrix", " {"].concat(),
        )
        .len()
            == 1
        && source_occurrences(&sources, &["pub struct ", "CancellationToken"].concat()).len() == 1
        && tensor_operation_part_twelve.contains("pow_with_context_exact_native(")
        && tensor_operation_part_twelve.contains("cumsum_with_context_exact_native(")
        && tensor_operation_part_twelve.contains("triangular_mask_with_context_exact_native(")
        && tensor_operation_part_twelve.contains("struct StftConfiguration")
        && tensor_operation_part_twelve.contains("pub struct TopKResult")
        && tensor_operation_part_twelve
            .matches("context.cancellation.check()?")
            .count()
            >= 15
        && model_attention.contains("pub fn enable_math_sdp_exact_native(")
        && model_attention.contains("MathSdpSelection::Enabled(AttentionKernelKind::ReferenceSdp)")
        && !tensor_operation_part_twelve.contains("thread_local!")
        && !tensor_operation_part_twelve.contains("OnceLock");
    let task56_foundational_boundaries_have_one_owner = source_occurrences(
        &sources,
        &["pub struct ", "DeterministicAlgorithmsPolicy", " {"].concat(),
    )
    .len()
        == 1
        && source_occurrences(
            &sources,
            &["pub struct ", "BackendCapabilityMatrix", " {"].concat(),
        )
        .len()
            == 1
        && source_occurrences(&sources, &["pub struct ", "MemoryTopology", " {"].concat()).len()
            == 1
        && source_occurrences(
            &sources,
            &["pub struct ", "MemoryAccountingSnapshot", " {"].concat(),
        )
        .len()
            == 1
        && source_occurrences(&sources, &["pub struct ", "CancellationToken"].concat()).len() == 1
        && tensor_operation_part_thirteen.contains("sign_with_context_exact_native(")
        && tensor_operation_part_thirteen.contains("isclose_with_context_exact_native(")
        && tensor_operation_part_thirteen.contains("linear_with_context_exact_native(")
        && tensor_operation_part_thirteen.contains("softmax_with_context_exact_native(")
        && tensor_operation_part_thirteen.contains("preserving_format_for(")
        && tensor_operation_part_thirteen.contains("DeterministicAlgorithmsPolicy::new(")
        && tensor_operation_part_thirteen
            .matches("context.cancellation.check()?")
            .count()
            >= 17
        && worker_memory_modes.contains("mlu_is_available_exact_native(")
        && worker_memory_modes.contains("mlu_device_count_exact_native(")
        && worker_memory_modes.contains("source_device_count_exact_native(")
        && worker_memory_modes.contains("mlu_mem_get_info_exact_native(")
        && worker_memory_modes.contains("source_mem_get_info_exact_native(")
        && !tensor_operation_part_thirteen.contains("struct DeterministicAlgorithmsPolicy")
        && !tensor_operation_part_thirteen.contains("thread_local!")
        && !tensor_operation_part_thirteen.contains("OnceLock");
    let task57_foundational_boundaries_have_one_owner = source_occurrences(
        &sources,
        &["pub struct ", "TensorDescriptor", " {"].concat(),
    )
    .len()
        == 1
        && source_occurrences(&sources, &["pub struct ", "AutocastPolicy", " {"].concat()).len()
            == 1
        && source_occurrences(
            &sources,
            &["pub struct ", "BackendCapabilityMatrix", " {"].concat(),
        )
        .len()
            == 1
        && source_occurrences(&sources, &["pub struct ", "MemoryTopology", " {"].concat()).len()
            == 1
        && source_occurrences(&sources, &["pub struct ", "CancellationToken"].concat()).len() == 1
        && tensor_operation_part_fourteen.contains("canonical_abs(")
        && tensor_operation_part_fourteen.contains("canonical_mul(")
        && tensor_operation_part_fourteen.contains("canonical_concatenate(")
        && tensor_operation_part_fourteen.contains("reinterpret_read_only(")
        && tensor_operation_part_fourteen
            .matches("context.cancellation.check()?")
            .count()
            >= 9
        && model_attention.contains("allow_fp16_bf16_reduction_math_sdp_exact_native(")
        && worker_memory_modes.contains("npu_device_count_exact_native(")
        && worker_memory_modes.contains("source_device_count_exact_native(")
        && !tensor_operation_part_fourteen.contains("struct AutocastPolicy")
        && !tensor_operation_part_fourteen.contains("struct BackendCapabilityMatrix")
        && !tensor_operation_part_fourteen.contains("thread_local!")
        && !tensor_operation_part_fourteen.contains("OnceLock");
    let task58_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, "pub enum DType").len() == 1
            && source_occurrences(&sources, "pub enum UnaryOperation").len() == 1
            && source_occurrences(&sources, "pub enum BinaryOperation").len() == 1
            && source_occurrences(&sources, "pub enum LinearAlgebraOperation").len() == 1
            && source_occurrences(&sources, &["pub struct ", "Tensor", " {"].concat()).len() == 1
            && source_occurrences(&sources, &["pub struct ", "MemoryTopology", " {"].concat())
                .len()
                == 1
            && source_occurrences(&sources, &["pub struct ", "CancellationToken"].concat()).len()
                == 1
            && tensor_operation_part_fifteen.contains("tensor_from_f32_with_context_exact_native(")
            && tensor_operation_part_fifteen.contains("cast_to_with_context_exact_native(")
            && tensor_operation_part_fifteen.contains("canonical_ceil_exact_native(")
            && tensor_operation_part_fifteen.contains("full_like_with_context_exact_native(")
            && tensor_operation_part_fifteen
                .contains("LinearAlgebraOperation::BatchMatrixMultiply")
            && tensor_operation_part_fifteen.contains("BinaryOperation::Atan2")
            && tensor_operation_part_fifteen.contains("UnaryOperation::Tangent")
            && tensor_operation_part_fifteen
                .matches("context.cancellation.check()?")
                .count()
                >= 21
            && model_attention.contains("enable_flash_sdp_exact_native(")
            && worker_memory_modes.contains("npu_is_available_exact_native(")
            && worker_memory_modes.contains("source_device_count_exact_native(")
            && !tensor_operation_part_fifteen.contains("enum DType")
            && !tensor_operation_part_fifteen.contains("enum UnaryOperation")
            && !tensor_operation_part_fifteen.contains("enum BinaryOperation")
            && !tensor_operation_part_fifteen.contains("enum LinearAlgebraOperation")
            && !tensor_operation_part_fifteen.contains("struct MemoryTopology")
            && !tensor_operation_part_fifteen.contains("thread_local!")
            && !tensor_operation_part_fifteen.contains("OnceLock");
    let task59_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, "pub enum DType").len() == 1
            && source_occurrences(&sources, "pub enum UnaryOperation").len() == 1
            && source_occurrences(&sources, "pub enum BinaryOperation").len() == 1
            && source_occurrences(&sources, "pub enum NativeBitwiseOperation").len() == 1
            && source_occurrences(&sources, &["pub struct ", "Tensor", " {"].concat()).len() == 1
            && source_occurrences(&sources, &["pub struct ", "CancellationToken"].concat()).len()
                == 1
            && tensor_operation_part_sixteen.contains("canonical_mul_with_context(")
            && tensor_operation_part_sixteen.contains("canonical_square_with_context(")
            && tensor_operation_part_sixteen.contains("canonical_bitwise_binary_with_context(")
            && tensor_operation_part_sixteen.contains("UnaryOperation::ArcTangent")
            && tensor_operation_part_sixteen.contains("BinaryOperation::LogAddExp")
            && tensor_operation_part_sixteen
                .matches("context.cancellation.check()?")
                .count()
                >= 20
            && worker_memory_modes.contains("mlu_empty_cache_exact_native(")
            && worker_memory_modes.contains("device_empty_cache_exact_native(")
            && runtime_trust.contains("pub struct NativeFfiRegistry")
            && runtime_trust.contains("pub fn cudart_exact_native")
            && model_restricted_pickle.contains("pub fn add_safe_globals_exact_native(")
            && !tensor_operation_part_sixteen.contains("enum DType")
            && !tensor_operation_part_sixteen.contains("enum UnaryOperation")
            && !tensor_operation_part_sixteen.contains("enum BinaryOperation")
            && !tensor_operation_part_sixteen.contains("enum NativeBitwiseOperation")
            && !tensor_operation_part_sixteen.contains("thread_local!")
            && !tensor_operation_part_sixteen.contains("OnceLock");
    let task60_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, "pub struct AutogradTape").len() == 1
            && source_occurrences(&sources, "pub enum UnaryOperation").len() == 1
            && source_occurrences(&sources, "pub struct MemoryTopology").len() == 1
            && source_occurrences(&sources, "pub struct MemoryAccountingSnapshot").len() == 1
            && source_occurrences(&sources, "pub struct MemoryPlacementInventory").len() == 1
            && source_occurrences(&sources, "pub struct BackendCapabilityMatrix").len() == 1
            && source_occurrences(&sources, "pub struct ExecutionContext").len() == 1
            && source_occurrences(&sources, "pub struct NativeModule").len() == 1
            && source_occurrences(&sources, concat!("pub struct ", "CancellationToken")).len() == 1
            && source_occurrences(&sources, "pub fn sdpa_kernel_exact_native(").len() == 1
            && source_occurrences(&sources, "pub fn spectral_norm_exact_native").len() == 1
            && source_occurrences(&sources, "pub fn xpu_is_available_exact_native(").len() == 1
            && source_occurrences(&sources, "pub fn cuda_mem_get_info_exact_native(").len() == 1
            && source_occurrences(&sources, "pub fn cuda_ipc_collect_exact_native(").len() == 1
            && tensor_operation_part_seventeen.contains("tape.set_requires_grad(")
            && tensor_operation_part_seventeen.contains("UnaryOperation::ArcHyperbolicTangent")
            && tensor_operation_part_seventeen.contains("clamp_with_context_exact_native(")
            && tensor_operation_part_seventeen.contains("input.narrow_read_only(")
            && tensor_operation_part_seventeen.contains("synchronize_device_exact_native(")
            && tensor_operation_part_seventeen
                .matches("context.cancellation.check()?")
                .count()
                >= 10
            && tensor_operation_part_seventeen.contains("execution.cancellation.check()?")
            && model_attention.contains("pub fn sdpa_kernel_exact_native(")
            && model_native_ops.contains("module.register_spectral_norm_exact_native(")
            && worker_memory_modes.contains("source_device_count_exact_native(")
            && worker_memory_modes.contains("source_mem_get_info_exact_native(")
            && worker_memory_modes.contains("inventory.stage_ipc_collection(")
            && !tensor_operation_part_seventeen_resolution.contains("COMFY-TENSOR-OP-BE67DCC5B9C6")
            && !tensor_operation_part_seventeen.contains("struct AutogradTape")
            && !tensor_operation_part_seventeen.contains("struct MemoryTopology")
            && !tensor_operation_part_seventeen.contains("struct MemoryAccountingSnapshot")
            && !tensor_operation_part_seventeen.contains("struct MemoryPlacementInventory")
            && !tensor_operation_part_seventeen.contains("struct BackendCapabilityMatrix")
            && !tensor_operation_part_seventeen.contains("struct ExecutionContext")
            && !tensor_operation_part_seventeen.contains("struct NativeModule")
            && !tensor_operation_part_seventeen.contains("thread_local!")
            && !tensor_operation_part_seventeen.contains("OnceLock");
    let task61_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, "pub enum DType").len() == 1
            && source_occurrences(&sources, "pub enum BinaryOperation").len() == 1
            && source_occurrences(&sources, "pub struct CustomKernelId").len() == 1
            && source_occurrences(&sources, "pub struct BackendCapabilityMatrix").len() == 1
            && source_occurrences(&sources, "pub struct ExecutionContext").len() == 1
            && source_occurrences(&sources, "pub struct NativeStreamRegistry").len() == 1
            && source_occurrences(&sources, "pub struct AutocastPolicy").len() == 1
            && source_occurrences(&sources, "pub struct NativeCompilePolicy").len() == 1
            && source_occurrences(&sources, "pub enum NativeCompilePhase").len() == 1
            && source_occurrences(&sources, concat!("pub struct ", "CancellationToken")).len() == 1
            && tensor_operation_part_eighteen
                .contains("tensor_constructor_with_context_exact_native(")
            && tensor_operation_part_eighteen.contains("dtype().byte_width()")
            && tensor_operation_part_eighteen.contains("canonical_numel(input, cancellation)")
            && tensor_operation_part_eighteen.contains("canonical_add_method_with_context(")
            && tensor_operation_part_eighteen.contains("BinaryOperation::Multiply")
            && tensor_operation_part_eighteen.contains("BinaryOperation::Add")
            && tensor_operation_part_eighteen.contains("canonical_log_softmax(")
            && tensor_operation_part_eighteen.contains("canonical_log_softmax_vjp(")
            && tensor_operation_part_eighteen.contains("canonical_log_softmax_jvp(")
            && tensor_operation_part_eighteen.contains("CustomKernelId::new(")
            && tensor_operation_part_eighteen.contains("registry.create(capabilities")
            && tensor_operation_part_eighteen
                .matches("context.cancellation.check()?")
                .count()
                >= 11
            && tensor_activation_normalization.contains("fn log_softmax_jvp_linearized(")
            && tensor_activation_normalization.contains("softmax_dot_tangent +=")
            && runtime_executor.contains("pub fn compiler_is_compiling_exact_native(")
            && runtime_executor.contains(".map_err(|_| NativeCompileError::Cancelled)?")
            && tensor_operation_part_eighteen_tests.contains(
                "task_61_every_public_tensor_adapter_observes_cancellation_before_validation",
            )
            && tensor_operation_part_eighteen_tests
                .contains("assert_ne!(canonical_gradient, canonical_tangent)")
            && tensor_operation_part_eighteen_resolution
                .matches("ResolvedOperationContract {")
                .count()
                == 12
            && !tensor_operation_part_eighteen.contains("enum DType")
            && !tensor_operation_part_eighteen.contains("enum BinaryOperation")
            && !tensor_operation_part_eighteen.contains("struct BackendCapabilityMatrix")
            && !tensor_operation_part_eighteen.contains("struct ExecutionContext")
            && !tensor_operation_part_eighteen.contains("struct NativeStreamRegistry")
            && !tensor_operation_part_eighteen.contains("struct AutocastPolicy")
            && !tensor_operation_part_eighteen.contains("struct NativeCompilePolicy")
            && !tensor_operation_part_eighteen.contains("thread_local!")
            && !tensor_operation_part_eighteen.contains("OnceLock");
    let task62_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, "pub struct TensorDescriptor").len() == 1
            && source_occurrences(&sources, "pub struct BackendCapabilityMatrix").len() == 1
            && source_occurrences(&sources, "pub struct NativeDeviceProperties").len() == 1
            && source_occurrences(&sources, "pub struct NativeStreamRegistry").len() == 1
            && source_occurrences(&sources, "pub trait CachedAllocationOwner").len() == 1
            && source_occurrences(&sources, concat!("pub struct ", "CancellationToken")).len() == 1
            && source_occurrences(&sources, "pub fn cos_with_context_exact_native(").len() == 1
            && source_occurrences(&sources, "pub fn div_with_context_exact_native(").len() == 1
            && source_occurrences(
                &sources,
                "pub fn flip_dimensions_with_context_exact_native(",
            )
            .len()
                == 1
            && source_occurrences(&sources, "pub fn argsort_with_context_exact_native(").len()
                == 1
            && source_occurrences(&sources, "fn device_empty_cache_exact_native(").len() == 1
            && tensor_operation_part_nineteen.contains("canonical_cos_with_context(")
            && tensor_operation_part_nineteen.contains("canonical_div_with_context(")
            && tensor_operation_part_nineteen.contains("canonical_flip_with_context(")
            && tensor_operation_part_nineteen.contains("canonical_argsort_with_context(")
            && tensor_operation_part_nineteen.contains("registry.create(capabilities")
            && tensor_operation_part_nineteen
                .matches("context.cancellation.check()?")
                .count()
                >= 12
            && worker_memory_modes.contains(
                "cuda_empty_cache_exact_native(\n    backend: &dyn CachedAllocationOwner,",
            )
            && worker_memory_modes.contains(
                "cancellation.check()?;\n    if !matches!(device.kind(), DeviceKind::Cuda | DeviceKind::Rocm)",
            )
            && worker_memory_modes.contains(
                "device_empty_cache_exact_native(backend, inventory, device, device.kind(), cancellation)",
            )
            && tensor_operation_part_nineteen_tests.contains(
                "task_62_every_public_tensor_adapter_observes_cancellation_before_validation",
            )
            && tensor_operation_part_nineteen_resolution
                .matches("ResolvedOperationContract {")
                .count()
                == 11
            && !tensor_operation_part_nineteen.contains("struct TensorDescriptor")
            && !tensor_operation_part_nineteen.contains("struct BackendCapabilityMatrix")
            && !tensor_operation_part_nineteen.contains("struct NativeDeviceProperties")
            && !tensor_operation_part_nineteen.contains("struct NativeStreamRegistry")
            && !tensor_operation_part_nineteen.contains("trait CachedAllocationOwner")
            && !tensor_operation_part_nineteen.contains("thread_local!")
            && !tensor_operation_part_nineteen.contains("OnceLock");
    let task63_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, "pub struct TensorDescriptor").len() == 1
            && source_occurrences(&sources, "pub struct BackendCapabilityMatrix").len() == 1
            && source_occurrences(&sources, "pub struct NativeStreamRegistry").len() == 1
            && source_occurrences(&sources, concat!("pub struct ", "CancellationToken")).len() == 1
            && source_occurrences(&sources, "pub struct NativeModule").len() == 1
            && source_occurrences(&sources, "pub enum MathSdpSelection").len() == 1
            && source_occurrences(&sources, "pub fn native_device_name_exact(").len() == 1
            && source_occurrences(&sources, "pub fn synchronize_device_exact_native(").len() == 1
            && source_occurrences(&sources, "pub fn cast_to_with_context_exact_native(").len() == 1
            && source_occurrences(
                &sources,
                "pub fn softmax_function_with_context_exact_native(",
            )
            .len()
                == 1
            && source_occurrences(
                &sources,
                "pub fn flip_dimensions_with_context_exact_native(",
            )
            .len()
                == 1
            && tensor_operation_part_twenty.contains("cast_to_with_context_exact_native(")
            && tensor_operation_part_twenty.contains("canonical_softmax_with_context(")
            && tensor_operation_part_twenty.contains("canonical_cuda_stream(")
            && tensor_operation_part_twenty.contains("synchronize_device_exact_native(")
            && tensor_operation_part_twenty.contains("canonical_flip_with_context(")
            && tensor_operation_part_twenty.contains("native_device_name_exact(")
            && tensor_operation_part_twenty
                .matches("input.view(descriptor, ViewAccess::ReadOnly)")
                .count()
                == 2
            && tensor_operation_part_twenty
                .matches("cancellation.check()?")
                .count()
                >= 19
            && model_native_ops.contains("NativeModule::container(\"torch.nn.Module\")")
            && model_alias_free.contains("base: NativeModule")
            && model_alias_free
                .contains("NativeModule::container(\"alias_free_torch.Activation1d\")")
            && model_attention.contains("pub fn enable_mem_efficient_sdp_exact_native(")
            && tensor_operation_part_twenty_tests.contains(
                "task_63_every_public_tensor_adapter_observes_cancellation_before_validation",
            )
            && model_operation_part_twenty_tests
                .contains("task_63_model_adapters_observe_cancellation_before_validation")
            && tensor_operation_part_twenty_resolution
                .matches("ResolvedOperationContract {")
                .count()
                == 12
            && !tensor_operation_part_twenty.contains("struct TensorDescriptor")
            && !tensor_operation_part_twenty.contains("struct BackendCapabilityMatrix")
            && !tensor_operation_part_twenty.contains("struct NativeStreamRegistry")
            && !tensor_operation_part_twenty.contains("struct NativeModule")
            && !tensor_operation_part_twenty.contains("thread_local!")
            && !tensor_operation_part_twenty.contains("OnceLock")
            && !model_alias_free.contains("thread_local!")
            && !model_alias_free.contains("OnceLock");
    let task64_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, "pub enum NativeBitwiseOperation").len() == 1
            && source_occurrences(&sources, "pub enum NativeCumulativeOperation").len() == 1
            && source_occurrences(&sources, "pub struct AutogradTape").len() == 1
            && source_occurrences(&sources, "pub trait GradientReducer").len() == 1
            && source_occurrences(&sources, "pub enum GradientMode").len() == 1
            && source_occurrences(&sources, "pub struct TensorDescriptor").len() == 1
            && source_occurrences(&sources, "pub struct MemoryAccountingSnapshot").len() == 1
            && source_occurrences(&sources, concat!("pub struct ", "CancellationToken")).len() == 1
            && source_occurrences(&sources, "pub fn mlu_memory_stats_exact_native(").len() == 1
            && tensor_operation_part_twenty_one.contains("tape.reverse_and_publish_with_context(")
            && tensor_operation_part_twenty_one.contains("canonical_bitwise_binary_with_context(")
            && tensor_operation_part_twenty_one.contains("NativeBitwiseOperation::Or")
            && tensor_operation_part_twenty_one.contains("canonical_cumulative_with_context(")
            && tensor_operation_part_twenty_one.contains("NativeCumulativeOperation::Product")
            && tensor_operation_part_twenty_one.contains("input.view(descriptor, ViewAccess::ReadOnly)")
            && tensor_operation_part_twenty_one
                .matches("cancellation.check()?")
                .count()
                >= 27
            && tensor_operation_part_twenty_one_tests.contains(
                "task_64_every_public_tensor_adapter_observes_cancellation_before_validation",
            )
            && tensor_operation_part_twenty_one_tests.contains("tape.state(), &TapeState::Active")
            && tensor_operation_part_twenty_one_tests.contains("unfolded.write().is_err()")
            && tensor_operation_part_twenty_one_resolution
                .matches("ResolvedOperationContract {")
                .count()
                == 12
            && tensor_operation_part_twenty_one_resolution.contains(
                "mlu_memory_stats_exact_native(capabilities: &BackendCapabilityMatrix, snapshot: &MemoryAccountingSnapshot, cancellation: &CancellationToken)",
            )
            && worker_memory_modes.contains(
                "mlu_memory_stats_exact_native(\n    capabilities: &BackendCapabilityMatrix,",
            )
            && worker_memory_modes.contains(
                "cancellation.check()?;\n    source_memory_stats_exact_native(capabilities, snapshot, &[DeviceKind::Mlu], cancellation)",
            )
            && worker_memory_tests
                .contains("task_64_mlu_memory_stats_reuse_canonical_worker_accounting")
            && !tensor_operation_part_twenty_one.contains("enum NativeBitwiseOperation")
            && !tensor_operation_part_twenty_one.contains("enum NativeCumulativeOperation")
            && !tensor_operation_part_twenty_one.contains("struct AutogradTape")
            && !tensor_operation_part_twenty_one.contains("enum GradientMode")
            && !tensor_operation_part_twenty_one.contains("struct TensorDescriptor")
            && !tensor_operation_part_twenty_one.contains("struct MemoryAccountingSnapshot")
            && !tensor_operation_part_twenty_one.contains("thread_local!")
            && !tensor_operation_part_twenty_one.contains("OnceLock");
    let task65_foundational_boundaries_have_one_owner =
        source_occurrences(&sources, "pub struct BackendCapabilityMatrix").len() == 1
            && source_occurrences(&sources, "pub struct AutocastPolicy").len() == 1
            && source_occurrences(&sources, "pub struct TensorDescriptor").len() == 1
            && source_occurrences(&sources, "pub struct MemoryTopology").len() == 1
            && source_occurrences(&sources, "pub struct MemoryAccountingSnapshot").len() == 1
            && source_occurrences(&sources, concat!("pub struct ", "CancellationToken")).len() == 1
            && source_occurrences(&sources, "pub fn cuda_memory_stats_exact_native(").len() == 1
            && source_occurrences(&sources, "pub fn npu_mem_get_info_exact_native(").len() == 1
            && tensor_operation_part_twenty_two.contains("canonical_argsort_with_context(")
            && tensor_operation_part_twenty_two.contains("canonical_floor_with_context(")
            && tensor_operation_part_twenty_two.contains("canonical_acos_with_context(")
            && tensor_operation_part_twenty_two.contains("canonical_exp_with_context(")
            && tensor_operation_part_twenty_two.contains("canonical_lerp_with_context(")
            && tensor_operation_part_twenty_two.contains("canonical_autocast(")
            && tensor_operation_part_twenty_two.contains("canonical_swapaxes(")
            && tensor_operation_part_twenty_two.contains("native_device_name_exact_for_kinds(")
            && tensor_operation_part_twenty_two.contains("native_select_device_exact(")
            && tensor_operation_part_twenty_two.contains("preserving_format_for(dtype, device)")
            && tensor_operation_part_twenty_two.contains("backend.fill(Scalar::Unsigned(0)")
            && tensor_operation_part_twenty_two.matches("pub fn ").count() == 20
            && tensor_operation_part_twenty_two
                .matches("cancellation.check()?")
                .count()
                == 21
            && tensor_operation_part_twenty_two_tests.contains(
                "task_65_every_public_tensor_adapter_observes_cancellation_before_validation",
            )
            && tensor_operation_part_twenty_two_tests.contains("out.storage_id()")
            && tensor_operation_part_twenty_two_tests.contains("scratch.peak_bytes(), 0")
            && tensor_operation_part_twenty_two_tests.contains("write().is_err()")
            && tensor_operation_part_twenty_two_resolution
                .matches("ResolvedOperationContract {")
                .count()
                == 12
            && tensor_operation_part_twenty_two_resolution.contains(
                "cuda_memory_stats_exact_native(capabilities: &BackendCapabilityMatrix, snapshot: &MemoryAccountingSnapshot, cancellation: &CancellationToken)",
            )
            && tensor_operation_part_twenty_two_resolution.contains(
                "npu_mem_get_info_exact_native(capabilities: &BackendCapabilityMatrix, topology: &MemoryTopology, snapshot: &MemoryAccountingSnapshot, cancellation: &CancellationToken)",
            )
            && worker_memory_modes.contains(
                "cuda_memory_stats_exact_native(\n    capabilities: &BackendCapabilityMatrix,",
            )
            && worker_memory_modes.contains(
                "npu_mem_get_info_exact_native(\n    capabilities: &BackendCapabilityMatrix,",
            )
            && worker_memory_modes.contains(
                "cancellation.check()?;\n    source_memory_stats_exact_native(\n        capabilities,\n        snapshot,\n        &[DeviceKind::Cuda, DeviceKind::Rocm],",
            )
            && worker_memory_modes.contains(
                "cancellation.check()?;\n    source_mem_get_info_exact_native(\n        capabilities,\n        topology,\n        snapshot,\n        &[DeviceKind::Npu],",
            )
            && worker_memory_tests
                .contains("task_65_cuda_stats_and_npu_info_reuse_canonical_worker_accounting")
            && !tensor_operation_part_twenty_two.contains("struct BackendCapabilityMatrix")
            && !tensor_operation_part_twenty_two.contains("struct AutocastPolicy")
            && !tensor_operation_part_twenty_two.contains("struct TensorDescriptor")
            && !tensor_operation_part_twenty_two.contains("struct MemoryTopology")
            && !tensor_operation_part_twenty_two.contains("struct MemoryAccountingSnapshot")
            && !tensor_operation_part_twenty_two.contains("thread_local!")
            && !tensor_operation_part_twenty_two.contains("OnceLock");

    let public_asset_index_escapes = ["pub fn artifact_index", "pub async fn artifact_index"]
        .into_iter()
        .flat_map(|needle| production_source_occurrences(&sources, needle))
        .filter(|location| location.contains("crates/comfy_runtime/src/assets.rs"))
        .collect::<Vec<_>>();
    let plugin_root_mapping_definitions =
        production_source_occurrences(&sources, "pub fn from_plugin_root(");
    let plugin_root_mapping_calls =
        production_source_occurrences(&sources, "AssetNamespace::from_plugin_root(");
    let plugin_capability_broker_definitions = source_occurrences(
        &sources,
        &["pub struct ", "PluginCapabilityBroker", " {"].concat(),
    );
    let execution_owner_impl = execution_presentation
        .split_once("impl ExecutionPresentationOwner {")
        .and_then(|(_, implementation)| {
            implementation
                .split_once("impl ExecutionPresentationService {")
                .map(|(owner, _)| owner)
        })
        .ok_or("execution presentation owner implementation is missing")?;
    let execution_owner_deref_impls =
        production_source_occurrences(&sources, "impl Deref for ExecutionPresentationOwner")
            .into_iter()
            .chain(production_source_occurrences(
                &sources,
                "impl std::ops::Deref for ExecutionPresentationOwner",
            ))
            .collect::<Vec<_>>();
    let private_reducer_call_sites = [
        ".stage_clone()",
        ".reduce_command(",
        ".apply_ack_unpersisted(",
        ".apply_event(",
        ".apply_output_operation(",
        ".reconcile_with_change(",
    ]
    .into_iter()
    .flat_map(|needle| production_source_occurrences(&sources, needle))
    .collect::<Vec<_>>();
    let public_per_attempt_persistence_apis = [
        "pub fn save_execution_attempt",
        "pub async fn save_execution_attempt",
        "pub fn replace_execution_attempts_for_profile",
        "pub async fn replace_execution_attempts_for_profile",
        "pub fn delete_execution_attempt",
        "pub async fn delete_execution_attempt",
    ]
    .into_iter()
    .flat_map(|needle| source_occurrences(&sources, needle))
    .collect::<Vec<_>>();
    let component_lifecycle_adapter_impls =
        production_source_occurrences(&sources, "impl ComponentLifecycleAdapter for ");
    let subgraph_library_definitions = source_occurrences(
        &sources,
        &["struct ", "SubgraphBlueprintLibrary", " {"].concat(),
    );
    let subgraph_catalog_definitions = source_occurrences(
        &sources,
        &["struct ", "SubgraphBlueprintCatalog", " {"].concat(),
    );
    let subgraph_catalog_type_sites =
        production_identifier_occurrences(&sources, "SubgraphBlueprintCatalog");
    let subgraph_catalog_refresh_sites =
        production_source_occurrences(&sources, "replace_native_subgraph_catalog(");
    let subgraph_catalog_owner_mutation_sites =
        production_source_occurrences(&sources, "global_mut::<GlobalNativeAssetServices>");
    let subgraph_catalog_entry_consumers =
        production_source_occurrences(&sources, "subgraph_catalog_node_library_message(");
    let subgraph_byte_preflight_position = subgraph_blueprints_production
        .find("if blueprint_byte_size > MAX_PUBLISHED_SUBGRAPH_BLUEPRINT_BYTES");
    let subgraph_asset_lock_position = subgraph_blueprints_production.find("let mut assets = self");
    let subgraph_reload_position = subgraph_blueprints_production
        .find("let mut catalog = self.reload_from_assets(&assets, cancellation)?;");
    let subgraph_preflight_position =
        subgraph_blueprints_production.find("validate_projected_catalog(&catalog");
    let subgraph_commit_position =
        subgraph_blueprints_production.find("let asset = assets.write_exact(");
    let subgraph_transaction_is_ordered = subgraph_byte_preflight_position
        .zip(subgraph_asset_lock_position)
        .zip(subgraph_reload_position)
        .zip(subgraph_preflight_position)
        .zip(subgraph_commit_position)
        .is_some_and(
            |((((byte_preflight, asset_lock), reload), aggregate_preflight), commit)| {
                byte_preflight < asset_lock
                    && asset_lock < reload
                    && reload < aggregate_preflight
                    && aggregate_preflight < commit
            },
        );
    let blueprint_decode_bound_position = runtime_graph_production
        .find("if workflow_bytes.len() > MAX_PUBLISHED_SUBGRAPH_BLUEPRINT_BYTES");
    let blueprint_decode_parse_position = runtime_graph_production
        .find("extract_published_subgraph_blueprint(filename, workflow_bytes)?;");
    let blueprint_decode_is_bounded_before_parse = blueprint_decode_bound_position
        .zip(blueprint_decode_parse_position)
        .is_some_and(|(bound, parse)| bound < parse);
    let subgraph_accounting_position =
        subgraph_blueprints_production.find("usize::try_from(asset.byte_size)");
    let subgraph_read_position =
        subgraph_blueprints_production.find("let bytes = match assets.read_verified(");
    let subgraph_accounting_precedes_reads = subgraph_accounting_position
        .zip(subgraph_read_position)
        .is_some_and(|(accounting, read)| accounting < read);
    let subgraph_projection_await_position = context_menu.find("let result = publication.await;");
    let subgraph_projection_replace_position =
        context_menu.find("replace_native_subgraph_catalog(publication.catalog, cx)");
    let subgraph_projection_send_position = context_menu.find("completion_sender.send(completion)");
    let subgraph_projection_is_detached_and_ordered = subgraph_projection_await_position
        .zip(subgraph_projection_replace_position)
        .zip(subgraph_projection_send_position)
        .is_some_and(|((publish, projection), send)| publish < projection && projection < send)
        && context_menu
            .contains("subgraph publication completed after its workspace item was released")
        && context_menu.contains("})\n        .detach();");
    let subgraph_drop_impl = workflow_item
        .split_once("impl Drop for GraphWorkspaceItem")
        .and_then(|(_, tail)| tail.split_once("impl EventEmitter<ItemEvent>"))
        .map_or("", |(drop_impl, _)| drop_impl);
    let subgraph_drop_cancellation_helper = workflow_item
        .split_once("fn cancel_subgraph_publication_for_drop(&mut self)")
        .and_then(|(_, tail)| tail.split_once("#[cfg(all(test, feature = \"test-support\"))]"))
        .map_or("", |(helper, _)| helper);
    let provider_execution_source = runtime_plugin_services_production
        .split_once("pub fn execute_provider_request(")
        .and_then(|(_, implementation)| implementation.split_once("pub fn credential_is_present("))
        .map_or("", |(implementation, _)| implementation);
    let provider_capability_position =
        provider_execution_source.find("self.require_capability(&Capability::ProviderNetwork");
    let provider_secret_capability_position =
        provider_execution_source.find("self.require_capability(&Capability::Secret");
    let provider_policy_position = provider_execution_source.find(".provider_policy");
    let credential_read_position = provider_execution_source.find(".credential_presence_actuator");
    let provider_actuator_position = provider_execution_source.find("provider_actuator.execute(");
    let provider_security_precedes_actuators = provider_capability_position
        .zip(provider_secret_capability_position)
        .zip(provider_policy_position)
        .zip(credential_read_position)
        .zip(provider_actuator_position)
        .is_some_and(
            |(
                (((provider_capability, secret_capability), provider_policy), credential),
                provider,
            )| {
                provider_capability < secret_capability
                    && secret_capability < provider_policy
                    && provider_policy < credential
                    && credential < provider
            },
        );
    let cancellation_policy = policy_concerns
        .iter()
        .find(|entry| {
            entry.get("concern").and_then(serde_json::Value::as_str) == Some("cancellation")
        })
        .ok_or("ownership policy has no cancellation concern")?;
    let worker_cancellation_is_allowed_observation = cancellation_policy
        .get("definitions")
        .and_then(serde_json::Value::as_array)
        .and_then(|definitions| {
            definitions.iter().find(|definition| {
                definition
                    .get("qualified")
                    .and_then(serde_json::Value::as_str)
                    == Some("comfy_runtime::NativeImageWorkerEvent::Failed.cancelled")
            })
        })
        .is_some_and(|definition| {
            definition.get("role").and_then(serde_json::Value::as_str) == Some("allowed_adapter")
        })
        && cancellation_policy
            .get("allowed_adapters")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|adapters| {
                adapters.iter().any(|adapter| {
                    adapter.as_str().is_some_and(|adapter| {
                        adapter.contains("untrusted wire observation")
                            && adapter.contains("canonical attempt termination intent and token")
                    })
                })
            })
        && cancellation_policy
            .get("required_mappings")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|mappings| {
                mappings.iter().any(|mapping| {
                    mapping.get("name").and_then(serde_json::Value::as_str)
                        == Some("worker-cancellation-observation-cannot-own-terminal-state")
                })
            });

    let adapter_profile = "ownership-adapter-profile";
    let adapter_directory = tempfile::tempdir()?;
    let shared_assets =
        open_native_profile_asset_service(adapter_profile, adapter_directory.path(), &[])?;
    let plugin_asset_authorization = authorize_native_plugin_asset_broker(adapter_profile)?;
    let plugin_asset_adapter =
        AssetPluginCapabilityServices::new(shared_assets.clone(), plugin_asset_authorization)?;
    let plugin_adapter_preserves_owner = Arc::ptr_eq(plugin_asset_adapter.assets(), &shared_assets);

    let safe_path = SafeVirtualPath::parse("nested/example.png")?;
    let mapped_identity = safe_path.into_asset_identity(adapter_profile, AssetNamespace::Input)?;
    let canonical_identity =
        AssetIdentity::new(adapter_profile, AssetNamespace::Input, "nested/example.png")?;
    let boundary_path_semantics = mapped_identity == canonical_identity
        && SafeVirtualPath::parse("../outside.png").is_err()
        && AssetIdentity::new(adapter_profile, AssetNamespace::Input, "../outside.png").is_err()
        && SafeVirtualPath::parse("/absolute.png").is_err()
        && AssetIdentity::new(adapter_profile, AssetNamespace::Input, "/absolute.png").is_err();
    let asset_reference = mapped_identity.to_reference()?;
    let reference_identity = shared_assets
        .lock()
        .map_err(|error| format!("asset service is unavailable: {error}"))?
        .roots()
        .identity_from_reference(&asset_reference)?;
    let asset_reference_semantics = reference_identity == mapped_identity
        && asset_reference == "sim-asset://input/nested/example.png"
        && shared_assets
            .lock()
            .map_err(|error| format!("asset service is unavailable: {error}"))?
            .roots()
            .identity_from_reference("sim-asset://input/../outside.png")
            .is_err();

    let external_navigation_policy = ExternalNavigationPolicy::https_user_gesture();
    let external_navigation_semantics = external_navigation_policy
        .authorize("https://docs.comfy.org/troubleshooting/overview", true)
        .is_ok()
        && external_navigation_policy
            .authorize("https://docs.comfy.org/troubleshooting/overview", false)
            .is_err()
        && external_navigation_policy
            .authorize("javascript:alert(1)", true)
            .is_err();
    let comfy_open_url_calls = source_occurrences(&sources, ".open_url(")
        .into_iter()
        .filter(|location| location.contains("/crates/comfy_"))
        .collect::<Vec<_>>();
    let sim_asset_reference_parsers =
        source_occurrences(&sources, "strip_prefix(\"sim-asset://\")");
    let sim_asset_reference_formatters =
        source_occurrences(&sources, "\"sim-asset://{}/{relative_path}\"");

    let output_committer = authorize_native_output_committer("ownership-profile")?;
    let output_ui = authorize_native_output_ui("ownership-profile")?;
    let output_write = Capability::Asset {
        namespace: "output".to_owned(),
        action: AssetOperation::Write,
    };
    let output_read = Capability::Asset {
        namespace: "output".to_owned(),
        action: AssetOperation::Read,
    };
    let output_delete = Capability::Asset {
        namespace: "output".to_owned(),
        action: AssetOperation::Delete,
    };

    let plugin_request = CapabilityRequest {
        kind: CapabilityKind::NetworkProvider,
        scope: "provider-a|https://provider-a.invalid/v1/generate".to_owned(),
        quota: CapabilityQuota {
            maximum_operations: 4,
            maximum_request_bytes: 4_096,
            maximum_response_bytes: 4_096,
            maximum_total_bytes: 16_384,
            maximum_handles: 2,
            timeout_milliseconds: 5_000,
        },
    };
    let plugin_capability = Capability::from_plugin_request(&plugin_request)?;
    let plugin_round_trip = plugin_capability
        .plugin_capability_key()
        .is_some_and(|(kind, scope)| kind == plugin_request.kind && scope == plugin_request.scope);

    let wire_capabilities = [
        output_write.clone(),
        output_read.clone(),
        output_delete.clone(),
        plugin_capability,
        Capability::Secret {
            secret_id: "secret.demo".to_owned(),
        },
        Capability::NativeRoute {
            route_id: "route.demo".to_owned(),
        },
    ];
    let wire_round_trip = wire_capabilities.iter().all(|capability| {
        Capability::parse_wire_identifier(&capability.wire_identifier()).as_ref() == Ok(capability)
    });

    let mut cases = BTreeMap::from([
        (
            "task39_workspace_accounting_chain_uses_authoritative_owners",
            workspace_accounting_chain_uses_authoritative_owners()?,
        ),
        (
            "task39_workspace_foundational_definitions_are_unique",
            attempt_memory_controller_definitions.len() == 1
                && attempt_memory_controller_definitions[0]
                    .contains("crates/comfy_worker/src/memory_modes.rs")
                && memory_planner_definitions.len() == 1
                && memory_planner_definitions[0]
                    .contains("crates/comfy_worker/src/memory_planner.rs")
                && scratch_reservation_definitions.len() == 1
                && scratch_reservation_definitions[0]
                    .contains("crates/comfy_tensor/src/operation.rs")
                && backend_workspace_authority_definitions.len() == 1
                && backend_workspace_authority_definitions[0]
                    .contains("crates/comfy_tensor/src/operation.rs")
                && cpu_workspace_authority_aliases.len() == 1
                && cpu_workspace_authority_aliases[0]
                    .contains("crates/comfy_tensor/src/cpu_backend.rs")
                && planned_workspace_authorization_definitions.len() == 1
                && planned_workspace_authorization_definitions[0]
                    .contains("crates/comfy_worker/src/memory_modes.rs")
                && worker_memory_modes
                    .contains("#[derive(Debug)]\npub struct PlannedWorkspaceAuthorization")
                && !worker_memory_modes.contains(
                    "#[derive(Clone, Debug)]\npub struct PlannedWorkspaceAuthorization"
                )
                && worker_memory_modes
                    .contains("#[derive(Debug)]\npub struct AttemptMemoryController")
                && !worker_memory_modes
                    .contains("#[derive(Clone, Debug)]\npub struct AttemptMemoryController")
                && workspace_authorizer_definitions.len() == 1
                && workspace_authorizer_definitions[0]
                    .contains("crates/comfy_tensor/src/operation.rs")
                && planned_workspace_authorizer_definitions.len() == 1
                && planned_workspace_authorizer_definitions[0]
                    .contains("crates/comfy_worker/src/supervisor.rs")
                && scratch_binding_sites.len() == 1
                && scratch_binding_sites[0]
                    .contains("crates/comfy_tensor/src/operation.rs")
                && zero_scratch_sites.is_empty()
                && legacy_workspace_context_sites.is_empty()
                && backend_workspace_lease_definitions.len() == 1
                && backend_workspace_lease_definitions[0]
                    .contains("crates/comfy_tensor/src/operation.rs")
                && cpu_workspace_vector_definitions.len() == 1
                && cpu_workspace_vector_definitions[0]
                    .contains("crates/comfy_tensor/src/cpu_backend.rs")
                && backend_memory_tracker_definitions.len() == 1
                && backend_memory_tracker_definitions[0]
                    .contains("crates/comfy_tensor/src/operation.rs"),
        ),
        (
            "task315_workspace_authority_is_unique_and_sealed",
            backend_workspace_authority_definitions.len() == 1
                && tensor_operation
                    .contains("#[derive(Debug)]\npub struct BackendWorkspaceAuthority")
                && !tensor_operation
                    .contains("#[derive(Clone, Debug)]\npub struct BackendWorkspaceAuthority")
                && cpu_workspace_authority_aliases.len() == 1
                && tensor_cpu_backend.contains(
                    "pub type CpuWorkspaceAuthority = BackendWorkspaceAuthority;"
                )
                && tensor_operation
                    .matches("pub fn authorize_workspace(")
                    .count()
                    == 1
                && tensor_operation.contains("if authorized_bytes > self.memory.limit()")
                && scratch_binding_sites.len() == 1
                && workspace_authorizer_definitions.len() == 1
                && workspace_authorizer_definitions[0]
                    .contains("crates/comfy_tensor/src/operation.rs")
                && planned_workspace_authorizer_definitions.len() == 1
                && planned_workspace_authorizer_definitions[0]
                    .contains("crates/comfy_worker/src/supervisor.rs")
                && worker_process
                    .matches("memory.issue_workspace_authorization()?")
                    .count()
                    == 1
                && worker_process
                    .matches("session.authorize_planned_workspace(planned_workspace)?")
                    .count()
                    == 1
                && !tensor_operation.contains("pub const fn none() -> Self")
                && unpaired_cpu_backend_constructors.is_empty()
                && !tensor_cpu_backend.contains("pub fn new(memory_limit_bytes")
                && runtime_controller_production.contains("fn projection_only_cpu_backend()")
                && runtime_controller_production
                    .contains("CpuWorkspaceAuthority::create_backend(")
                && runtime_controller_production.contains("drop(authority);")
                && tensor_operation.contains("authority: Arc<ScratchAuthorization>")
                && !tensor_operation.contains("authority: Option<Arc<ScratchAuthorization>>")
                && zero_scratch_sites.is_empty()
                && legacy_workspace_context_sites.is_empty()
                && legacy_symmetric_eigen_decomposition_sites.is_empty()
                && ownership_generator.contains("TASK_315_CONCERNS")
                && ownership_generator.contains("TASK_315_VALIDATIONS"),
        ),
        (
            "task104_rocm_reuses_canonical_backend_accounting_and_authority",
            tensor_rocm_backend.contains("pub struct RocmTensorBackend")
                && tensor_rocm_backend.contains("memory: Arc<BackendMemoryTracker>")
                && tensor_rocm_backend
                    .contains("BackendWorkspaceAuthority::new(memory_limit_bytes)")
                && tensor_rocm_backend.contains("reserve_backend_workspace(")
                && tensor_rocm_backend.contains("check_backend_context(self.backend_id, context)")
                && !tensor_rocm_backend.contains("struct RocmWorkspaceAuthority")
                && !tensor_rocm_backend.contains("struct RocmMemoryTracker")
                && tensor_rocm_backend.contains(
                    "foreign_scratch_is_rejected_before_allocation_and_accounting_converges",
                )
                && tensor_rocm_backend
                    .contains("assert_eq!(runtime.allocation_calls.load(Ordering::Acquire), 0)")
                && tensor_rocm_backend.contains(
                    "download_staging_obeys_scratch_before_copy_or_output_allocation",
                )
                && tensor_rocm_backend
                    .contains("assert_eq!(runtime.device_to_host_calls.load(Ordering::Acquire), 0)"),
        ),
        (
            "task104_rocm_properties_and_protocol_boundaries_preserve_canonical_semantics",
            backend_rocm_loader.contains("pub struct RocmDeviceProperties")
                && backend_rocm_loader.contains("pub fn name(&self) -> &str")
                && backend_rocm_loader.contains("pub const fn total_memory_bytes(&self) -> u64")
                && backend_rocm_loader.contains("pub const fn major(&self) -> u32")
                && backend_rocm_loader.contains("pub const fn minor(&self) -> u32")
                && backend_rocm_loader.contains("pub fn architecture(&self) -> Option<&str>")
                && backend_rocm_loader.contains("pub const fn has_fp16(&self) -> bool")
                && !backend_rocm_loader.contains("impl Validate for RocmDeviceProperties")
                && tensor_rocm_backend_production.contains("NativeDeviceProperties::new(")
                && tensor_rocm_backend_production.contains("properties.name()")
                && tensor_rocm_backend_production.contains("properties.total_memory_bytes()")
                && tensor_rocm_backend_production.contains("properties.major()")
                && tensor_rocm_backend_production.contains("properties.minor()")
                && tensor_rocm_backend_production
                    .contains("properties.architecture().map(str::to_owned)")
                && tensor_rocm_backend_production.contains("properties.has_fp16()")
                && worker_protocol.contains("pub const WORKER_PROTOCOL_VERSION: u16 = 7;")
                && worker_protocol
                    .contains("pub const LEGACY_WORKER_PROTOCOL_VERSION: u16 = 6;")
                && worker_protocol.contains("pub struct WorkerNativeDeviceProperties")
                && tensor_operation.contains("WorkerNativeDeviceProperties::new(")
                && tensor_operation.contains("NativeDeviceProperties::new(")
                && tensor_operation.contains("Self::new_with_properties(")
                && worker_protocol
                    .contains("pub const WORKER_OPERATION_SUPPORT_VERSION: u16 = 2;")
                && worker_protocol.contains(
                    "pub const LEGACY_WORKER_OPERATION_SUPPORT_VERSION: u16 = 1;",
                )
                && worker_protocol.contains("pub enum WorkerPrimitiveOperationV1")
                && worker_protocol.contains("pub enum WorkerPrimitiveOperationV2")
                && worker_protocol.contains("pub struct WorkerOperationSupportV1")
                && worker_protocol.contains("postcard::take_from_bytes::<u16>(payload)")
                && worker_protocol
                    .contains("legacy_protocol_is_rejected_before_changed_payload_decode")
                && worker_protocol
                    .contains("operation_support_schema_versions_reject_mixed_peers_before_use")
                && worker_protocol
                    .contains("worker_primitive_operation_postcard_discriminants_are_append_only")
                && tensor_operation.contains(
                    "LinearAlgebraOperation => WorkerLinearAlgebraOperationV1",
                )
                && tensor_operation
                    .contains("impl From<PrimitiveOperation> for WorkerPrimitiveOperationV2")
                && tensor_operation
                    .contains("impl From<WorkerPrimitiveOperationV2> for PrimitiveOperation"),
        ),
        (
            "task104_cpu_and_rocm_factories_pair_backends_with_one_external_authority",
            tensor_cpu_backend_production.contains(
                "Result<(CpuBackend, BackendWorkspaceAuthority), TensorError>",
            )
                && tensor_cpu_backend_production
                    .matches("Result<(CpuBackend, BackendWorkspaceAuthority), TensorError>")
                    .count()
                    == 1
                && tensor_rocm_backend_production.contains(
                    "Result<(RocmTensorBackend, BackendWorkspaceAuthority), TensorError>",
                )
                && tensor_rocm_backend_production
                    .matches("pub fn from_certified_runtime(")
                    .count()
                    == 1
                && !tensor_cpu_backend_production.contains("impl CpuBackend {\n    pub fn new(")
                && !tensor_rocm_backend_production
                    .contains("impl RocmTensorBackend {\n    pub fn new(")
                && tensor_cpu_backend_production
                    .split_once("pub struct CpuBackend")
                    .and_then(|(_, source)| source.split_once("impl CpuBackend"))
                    .is_some_and(|(fields, _)| !fields.contains("authority:"))
                && tensor_rocm_backend_production
                    .split_once("pub struct RocmTensorBackend")
                    .and_then(|(_, source)| source.split_once("impl RocmTensorBackend"))
                    .is_some_and(|(fields, _)| !fields.contains("authority:")),
        ),
        (
            "task39_workspace_ownership_policy_is_traced",
            task_39_policy_trace
                && ownership_generator.contains("TASK_39_CONCERNS")
                && ownership_generator
                    .contains("if not definition[\"role\"].startswith(\"canonical\")")
                && ownership_generator
                    .contains("validate_concerns(policy[\"concerns\"], task_states)"),
        ),
        (
            "val_memory_001_has_exactly_one_normative_artifact_writer",
            normative_memory_artifact_writer_sites.len() == 1
                && normative_memory_artifact_writer_sites[0]
                    .contains("crates/comfy_worker/tests/memory_conformance.rs"),
        ),
        (
            "val_vae_001_has_exactly_one_normative_artifact_writer",
            normative_vae_artifact_writer_sites.len() == 1
                && normative_vae_artifact_writer_sites[0]
                    .contains("crates/comfy_model/tests/vae_architecture.rs"),
        ),
        (
            "tensor_scalar_truth_has_one_dtype_owner",
            tensor_dtypes.matches("pub const fn is_nonzero(self) -> bool").count() == 1
                && tensor_operation_part_five.contains("if value.is_nonzero()")
                && tensor_operation_part_seven.contains(".is_nonzero()")
                && !tensor_operation_part_five.contains("fn decoded_nonzero(")
                && !tensor_operation_part_seven.contains("fn scalar_is_nonzero("),
        ),
        (
            "tensor_decoded_scalar_encoding_has_one_dtype_owner",
            tensor_dtypes
                .matches("pub fn encode_decoded_scalar(")
                .count()
                == 1
                && [
                    &tensor_operation_part_three,
                    &tensor_operation_part_four,
                    &tensor_operation_part_ten,
                    &tensor_operator_indirection,
                    &tensor_indexing_masking_part_one,
                ]
                .into_iter()
                .all(|source| source.contains("encode_decoded_scalar"))
                && !tensor_indexing_masking_part_one.contains("fn encode_decoded("),
        ),
        (
            "tensor_broadcast_geometry_has_one_validation_owner",
            tensor_cpu_backend
                .matches("pub fn binary_broadcast_shape(")
                .count()
                == 1
                && tensor_cpu_backend
                    .matches("pub(crate) fn broadcast_indices(")
                    .count()
                    == 1
                && tensor_operation_part_seven
                    .contains("cpu_backend::{binary_broadcast_shape, broadcast_indices}")
                && tensor_operation_part_nine.contains(
                    "cpu_backend::{binary_broadcast_shape, broadcast_indices as canonical_broadcast_indices}",
                )
                && tensor_operation_part_fifteen
                    .contains("cpu_backend::{binary_broadcast_shape, broadcast_indices}")
                && [
                    &tensor_operation_part_seven,
                    &tensor_operation_part_nine,
                    &tensor_operation_part_fifteen,
                ]
                .into_iter()
                .all(|source| {
                    !source.contains("\nfn broadcast_indices(")
                        && !source.contains("\nfn broadcast_shape(\n")
                }),
        ),
        (
            "tensor_narrow_geometry_and_aliasing_have_one_owner",
            tensor_domain.matches("pub fn narrowed_view(").count() == 1
                && tensor_domain.matches("pub fn narrow_read_only(").count() == 1
                && tensor_domain
                    .matches("pub(crate) fn normalize_narrow_range(")
                    .count()
                    == 1
                && tensor_cpu_backend.contains(".narrowed_view(dimension, *start, *length)")
                && tensor_cpu_backend.contains("normalize_narrow_range(size, *start, *length)")
                && !tensor_cpu_backend.contains("fn normalize_narrow_start(")
                && tensor_operation_part_seventeen
                    .contains("input.narrow_read_only(axis, start, length)")
                && tensor_operation_part_seventeen
                    .split_once("pub fn tensor_split_exact_native")
                    .is_some_and(|(_, tensor_split)| {
                        tensor_split
                            .find("cancellation.check()?")
                            .zip(tensor_split.find("require_cpu(input"))
                            .is_some_and(|(cancellation, validation)| cancellation < validation)
                    }),
        ),
        (
            "tensor_index_foundation_ownership_is_traced",
            tensor_index_foundation_policy_is_declared
                && tensor_index_foundation_catalog_is_declared,
        ),
        (
            "tensor_indexing_part_one_has_focused_authoritative_owners",
            tensor_indexing_part_one_policy_is_declared
                && gather_scatter_plan_definitions.len() == 1
                && conditional_selection_definitions.len() == 1
                && nonzero_projection_definitions.len() == 1
                && tensor_indexing_masking_part_one
                    .matches("struct GatherScatterPlan")
                    .count()
                    == 1
                && tensor_indexing_masking_part_one
                    .matches("pub fn nonzero_with_context_exact_native(")
                    .count()
                    == 1
                && tensor_indexing_masking_part_one.contains("argwhere_with_context_exact_native(")
                && tensor_indexing_masking_part_one.contains("broadcast_tensor_vjp_with_context_exact_native(")
                && tensor_indexing_masking_part_one.contains("input.narrow_read_only(")
                && !tensor_indexing_masking_part_one.contains("fn broadcast_shape(")
                && !tensor_indexing_masking_part_one.contains("fn scalar_is_nonzero(")
                && !tensor_indexing_masking_part_one.contains("context.check()?")
                && tensor_indexing_masking_part_one
                    .matches("context.cancellation.check()?")
                    .count()
                    >= 25
                && tensor_indexing_masking_part_one.contains(
                    "ElementwiseRuntimePartSevenError::Cancelled => IndexingMaskingPartOneError::Cancelled",
                )
                && tensor_indexing_masking_part_one.contains(
                    "ElementwiseRuntimePartTwentyError::Cancelled => IndexingMaskingPartOneError::Cancelled",
                ),
        ),
        (
            "tensor_indexing_part_two_only_adapts_authoritative_owners",
            tensor_indexing_part_two_policy_is_declared
                && masked_fill_method_adapter_definitions.len() == 1
                && nonzero_method_adapter_definitions.len() == 1
                && tensor_indexing_masking_part_two.contains("masked_fill_in_place_with_context_exact_native(")
                && tensor_indexing_masking_part_two.contains("masked_fill_vjp_with_context_exact_native(")
                && tensor_indexing_masking_part_two.contains("masked_fill_jvp_with_context_exact_native(")
                && tensor_indexing_masking_part_two.contains("nonzero_with_context_exact_native(")
                && !tensor_indexing_masking_part_two.contains("binary_broadcast_shape")
                && !tensor_indexing_masking_part_two.contains("broadcast_indices")
                && !tensor_indexing_masking_part_two.contains("decode_scalar")
                && !tensor_indexing_masking_part_two.contains("encode_scalar")
                && !tensor_indexing_masking_part_two.contains("TensorWrite")
                && !tensor_indexing_masking_part_two.contains("cancellation.check()?")
                && tensor_indexing_masking_part_two
                    .contains("Canonical(#[from] IndexingMaskingPartOneError)")
                && !tensor_indexing_masking_part_two.contains("for ")
                && !tensor_indexing_masking_part_two.contains("while "),
        ),
        (
            "tensor_linear_algebra_part_one_has_one_mechanics_owner",
            tensor_linear_algebra_policy_is_declared
                && tensor_linear_algebra_catalog_is_declared
                && tensor_linear_algebra_has_one_owner,
        ),
        (
            "tensor_linear_algebra_part_two_has_one_svd_and_adapter_owner",
            tensor_linear_algebra_part_two_policy_is_declared
                && tensor_linear_algebra_part_two_catalog_is_declared
                && tensor_linear_algebra_part_two_has_one_owner,
        ),
        (
            "tensor_neural_network_functional_part_one_has_one_mechanics_and_adapter_owner",
            tensor_neural_network_functional_policy_is_declared
                && tensor_neural_network_functional_catalog_is_declared
                && tensor_neural_network_functional_has_one_owner,
        ),
        (
            "tensor_neural_network_module_part_one_has_one_lifecycle_resize_and_local_mechanics_owner",
            tensor_neural_network_module_policy_is_declared
                && tensor_neural_network_module_catalog_is_declared
                && tensor_neural_network_module_has_one_owner,
        ),
        (
            "tensor_neural_network_module_part_three_has_one_lifecycle_pooling_padding_and_autograd_owner",
            tensor_neural_network_module_part_three_policy_is_declared
                && tensor_neural_network_module_part_three_catalog_is_declared
                && tensor_neural_network_module_part_three_has_one_owner,
        ),
        (
            "tensor_spatial_functional_kernel_only_adapts_authoritative_convolution_pool_and_sampling_owners",
            tensor_spatial_functional_kernel_policy_is_declared
                && tensor_spatial_functional_kernel_catalog_is_declared
                && tensor_spatial_functional_kernel_only_adapts_authoritative_owners,
        ),
        (
            "tensor_spectral_transform_only_adapts_the_task_55_fft_codec_workspace_and_publication_owners",
            tensor_spectral_transform_policy_is_declared
                && tensor_spectral_transform_catalog_is_declared
                && tensor_spectral_transform_only_adapts_the_task_55_fft_owner,
        ),
        (
            "tensor_storage_dtype_device_only_adapts_canonical_storage_dtype_device_cast_workspace_and_context_owners",
            tensor_storage_dtype_device_policy_is_declared
                && tensor_storage_dtype_device_catalog_is_declared
                && tensor_storage_dtype_device_only_adapts_authoritative_owners,
        ),
        (
            "tensor_neural_network_module_part_four_has_one_lifecycle_rng_pooling_activation_and_storage_owner",
            tensor_neural_network_module_part_four_policy_is_declared
                && tensor_neural_network_module_part_four_catalog_is_declared
                && tensor_neural_network_module_part_four_has_one_owner,
        ),
        (
            "tensor_random_number_generation_part_one_has_one_rng_distribution_sobol_and_brownian_owner",
            tensor_random_number_generation_part_one_policy_is_declared
                && tensor_random_number_generation_part_one_catalog_is_declared
                && tensor_random_number_generation_part_one_has_one_owner,
        ),
        (
            "asset_duplicate_types_removed",
            source_occurrences(&sources, "struct AssetCapabilities").is_empty()
                && source_occurrences(&sources, "struct AssetGrant").is_empty(),
        ),
        (
            "asset_and_model_foundations_have_one_owner",
            artifact_root_definitions.len() == 1
                && artifact_root_definitions[0]
                    .contains("crates/comfy_model/src/artifact_index.rs")
                && artifact_index_definitions.len() == 1
                && artifact_index_definitions[0]
                    .contains("crates/comfy_model/src/artifact_index.rs")
                && asset_service_definitions.len() == 1
                && asset_service_definitions[0].contains("crates/comfy_runtime/src/assets.rs")
                && model_store_definitions.len() == 1
                && model_store_definitions[0].contains("crates/comfy_model/src/model_store.rs")
                && runtime_assets.contains("self.artifact_root_for_identity(identity)?")
                && runtime_assets.contains(".resolve_existing(&key.relative_path)")
                && runtime_assets.contains("ArtifactIndex::from_snapshot("),
        ),
        (
            "asset_raw_paths_are_crate_private_or_test_support_only",
            !runtime_assets.contains("pub fn root(")
                && !runtime_assets.contains("pub fn resolve_existing(")
                && runtime_assets.contains("pub(crate) fn resolve_existing(")
                && runtime_assets.contains("pub fn test_root_path(")
                && runtime_assets.contains("pub fn test_resolve_existing(")
                && production_source_occurrences(&sources, ".test_root_path(").is_empty()
                && production_source_occurrences(&sources, ".test_resolve_existing(").is_empty(),
        ),
        (
            "model_store_consumes_the_shared_asset_service_index_in_production",
            runtime_plugin_services_production.contains("assets: SharedAssetService")
                && runtime_plugin_services_production.contains("model_store: Mutex<ModelStore>")
                && runtime_plugin_services_production.contains("let assets = self")
                && runtime_plugin_services_production
                    .contains(".broker\n            .inner\n            .assets")
                && runtime_plugin_services_production
                    .contains("let mut model_store = self.broker.inner.model_store.lock()")
                && runtime_plugin_services_production.contains(
                    ".load_model(\n                &identity,\n                &mut model_store,",
                )
                && runtime_assets.contains(
                    "configured_model_roots_share_one_index_and_stable_logical_identities",
                ),
        ),
        (
            "image_vae_admission_and_equations_have_one_authoritative_owner",
            runtime_assets.contains("pub fn load_image_vae_with_context(")
                && runtime_assets.contains("self.load_model(")
                && runtime_assets.contains(".artifact_index\n            .record(&key)")
                && runtime_assets.contains("VaeDescriptor::checked_selection(")
                && runtime_assets.contains("load_image_vae_from_model_store_with_context(")
                && model_vae_image.contains("source_state_manifest(")
                && model_vae_image.contains("canonical_vision_model_store_dtype(")
                && model_vae_image.contains("geometry\n        .checked_output_shape(")
                && model_vae_image.contains("softmax_tensor_with_context_exact_native(")
                && model_vae_image.contains("group_norm_tensor_with_context_exact_native(")
                && model_vae_image.contains("batch_norm_tensor_with_context_exact_native(")
                && model_vae_image
                    .contains("channel_layer_norm_tensor_with_context_exact_native(")
                && model_vae_image.contains("channel_standardize_tensor_with_context_exact_native(")
                && model_vae_image.contains("replication_pad_2d_tensor_with_context_exact_native(")
                && model_vae_image.contains("pixel_shuffle_tensor_with_context_exact_native(")
                && model_vae_image.contains("pixel_unshuffle_tensor_with_context_exact_native(")
                && !model_vae_image.contains("NativeImageVaeStateSpec")
                && !model_vae_image.contains("NativeImageVaeStateKind")
                && !model_vae_image.contains("fn rearrange_pixel_channels(")
                && !model_vae_image.contains("fn storage_dtype(")
                && !model_crate_root.contains("load_image_vae_from_model_store_with_context")
                && tensor_activation.contains("pub fn softmax_tensor_with_context_exact_native(")
                && tensor_activation.contains("pub fn group_norm_tensor_with_context_exact_native(")
                && tensor_activation.contains("pub fn batch_norm_tensor_with_context_exact_native(")
                && tensor_activation
                    .contains("pub fn channel_layer_norm_tensor_with_context_exact_native(")
                && tensor_activation
                    .contains("pub fn channel_standardize_tensor_with_context_exact_native(")
                && tensor_functional
                    .contains("pub fn pixel_shuffle_tensor_with_context_exact_native(")
                && tensor_functional
                    .contains("pub fn pixel_unshuffle_tensor_with_context_exact_native(")
                && tensor_module
                    .contains("pub fn replication_pad_2d_tensor_with_context_exact_native("),
        ),
        (
            "model_archive_validation_is_disjoint_from_extension_lifecycle_unpacking",
            model_archive_entry_definitions.len() == 1
                && model_archive_entry_definitions[0].contains("crates/comfy_model/src/formats.rs")
                && model_formats.contains("fn canonical_archive_path(")
                && model_formats.contains("ModelFormatError::UnsafeArchivePath")
                && model_formats.contains("ModelFormatError::ArchiveLink")
                && model_formats.contains("ModelFormatError::DuplicateArchivePath")
                && extension_host.contains("Archive::new(decompressed_bytes)")
                && extension_host.contains("archive.unpack(temp_dir.path()).await?")
                && extension_host.contains("fn validate_component_extension_id(")
                && extension_host.contains("const MAXIMUM_EXTENSION_ID_BYTES: usize = 64 * 1024")
                && extension_host.contains("!extension_id.contains(['/', '\\\\', ':'])")
                && extension_host.contains("Some(path::Component::Normal(_))")
                && extension_host.contains("validate_component_extension_id(extension_id)?")
                && extension_host.matches("checked_extension_dir(").count() == 5
                && !extension_host.contains("comfy_model::ArtifactKey")
                && !extension_host.contains("comfy_model::formats")
                && !extension_host.contains("ArchiveEntry"),
        ),
        (
            "asset_index_has_no_public_service_escape",
            public_asset_index_escapes.is_empty()
                && runtime_assets.contains("#[cfg(test)]\n    pub(crate) fn artifact_index(&self)")
                && runtime_assets.contains("artifact_index: ArtifactIndex")
                && !subgraph_blueprints.contains("ArtifactIndex")
                && !native_asset_services.contains("ArtifactIndex"),
        ),
        (
            "boundary_paths_delegate_to_canonical_identity",
            boundary_path_semantics
                && api_http.contains("AssetIdentity::new(\"http-wire\"")
                && api_http.contains("AssetIdentity::new(profile_id, namespace, self.0)")
                && plugin_host
                    .contains("AssetIdentity::new(\"plugin-wire\", namespace, identifier)")
                && plugin_host_production_capabilities
                    .contains("AssetNamespace::from_plugin_root(root)")
                && plugin_host_production_capabilities
                    .contains("AssetIdentity::new(profile_id, namespace, relative_path)")
                && plugin_host_production_capabilities
                    .contains("AssetError::PermissionDenied { .. } | AssetError::ProfileMismatch")
                && plugin_root_mapping_definitions.len() == 1
                && plugin_root_mapping_definitions[0]
                    .contains("crates/comfy_runtime/src/assets.rs")
                && plugin_root_mapping_calls.len() == 2
                && plugin_root_mapping_calls.iter().all(|location| {
                    location.contains("crates/comfy_runtime/src/permissions.rs")
                        || location.contains("crates/comfy_plugin_host/src/capabilities.rs")
                }),
        ),
        (
            "asset_references_have_one_checked_mapping_owner",
            asset_reference_semantics
                && sim_asset_reference_parsers.len() == 1
                && sim_asset_reference_parsers.first().is_some_and(|location| {
                    location.contains("crates/comfy_runtime/src/assets.rs")
                })
                && sim_asset_reference_formatters.len() == 1
                && sim_asset_reference_formatters
                    .first()
                    .is_some_and(|location| {
                        location.contains("crates/comfy_runtime/src/assets.rs")
                    })
                && runtime_controller_production.contains(".to_reference()")
                && !runtime_controller_production.contains("format!(\"sim-asset://")
                && !execution_ui_production.contains("strip_prefix(\"sim-asset://\")"),
        ),
        (
            "backend_matrix_is_single_owner",
            backend_matrix_definitions.len() == 1
                && backend_matrix_definitions[0].contains("crates/comfy_tensor/src/operation.rs")
                && backend_readiness_definitions.len() == 1
                && backend_readiness_definitions[0]
                    .contains("crates/comfy_tensor/src/operation.rs")
                && backend_binding_definitions.len() == 1
                && backend_binding_definitions[0].contains("crates/comfy_types/src/comfy_types.rs")
                && all_device_readiness_is_typed,
        ),
        (
            "backend_adapters_only_report_binding_status",
            backend_adapters_are_binding_only
                && source_occurrences(&sources, "pub trait NativeBackend {").is_empty()
                && source_occurrences(&sources, "fn availability(&self)")
                    .into_iter()
                    .all(|location| !location.contains("crates/comfy_"))
                && source_occurrences(&sources, "profile.device !=").is_empty()
                && production_source_occurrences(&sources, "BackendUnavailable::new(")
                    .into_iter()
                    .all(|location| {
                        [
                            "crates/comfy_types/src/comfy_types.rs",
                            "crates/comfy_tensor/src/operation.rs",
                            "crates/comfy_worker/src/comfy_worker.rs",
                            "crates/comfy_runtime/src/native_ffi_metal.rs",
                            "crates/comfy_runtime/src/native_ffi_mlu.rs",
                            "crates/comfy_runtime/src/native_ffi_npu.rs",
                            "crates/comfy_runtime/src/native_ffi_xpu.rs",
                            "crates/comfy_runtime/src/native_ffi_cuda.rs",
                            "crates/comfy_runtime/src/native_ffi_directml.rs",
                            "crates/comfy_runtime/src/native_ffi_rocm.rs",
                            "crates/comfy_runtime/src/runtime_supervisor.rs",
                        ]
                        .iter()
                        .any(|allowed| location.contains(allowed))
                    }),
        ),
        (
            "cancellation_is_single_owner",
            cancellation_definitions.len() == 1
                && cancellation_definitions[0].contains("crates/comfy_types/src/cancellation.rs"),
        ),
        (
            "worker_cancellation_is_a_bounded_canonical_observation",
            worker_cancellation_is_allowed_observation
                && runtime_controller_production
                    .contains("NativeImageWorkerEvent::Failed { message, cancelled }")
                && runtime_controller_production
                    .contains("let canonical_termination = canonical_termination_kind(")
                && runtime_controller_production.contains(".termination_intent(")
                && runtime_controller_production
                    .contains("let worker_observation_was_unowned = cancelled")
                && runtime_controller_production.contains(
                    "let kind = canonical_termination.unwrap_or_else(|| AttemptEventKind::Failed",
                )
                && runtime_controller_production.contains("worker_reported_cancelled\": true")
                && runtime_controller_production
                    .contains("None if active.cancellation.is_cancelled()")
                && !runtime_controller_production
                    .contains("if cancelled { AttemptEventKind::Cancelled")
                && !runtime_controller_production
                    .contains("cancelled.then(|| AttemptEventKind::Cancelled"),
        ),
        (
            "execution_queue_is_single_owner",
            execution_queue_definitions.len() == 1
                && execution_queue_definitions[0]
                    .contains("crates/comfy_runtime/src/queue_history.rs")
                && native_queue_definitions.is_empty()
                && source_occurrences(&sources, "struct PendingNativeExecution").is_empty()
                && source_occurrences(&sources, "struct NativeControllerRecoveryDocument")
                    .is_empty()
                && source_occurrences(&sources, "struct NativeControllerRecoveryStore").is_empty()
                && source_occurrences(&sources, "InMemoryExecutionController").is_empty(),
        ),
        (
            "execution_consumers_share_one_profile_service",
            execution_presentation.contains("pub type SharedExecutionPresentationService")
                && runtime_controller_production
                    .contains("pub presentation: SharedExecutionPresentationService")
                && execution_ui_production.contains("service: SharedExecutionPresentationService")
                && api_services.contains("presentation: SharedExecutionPresentationService")
                && api_headless.contains("presentation: SharedExecutionPresentationService")
                && sim_bootstrap.contains("shared_service()")
                && sim_cli.contains("native_presentation(")
                && execution_ui_production.contains("contains_canonical_event(&event)")
                && !api_host_production.contains(".apply_event(event.clone())"),
        ),
        (
            "execution_mutation_and_persistence_have_one_owner",
            execution_owner_definitions.len() == 1
                && execution_owner_definitions[0]
                    .contains("crates/comfy_runtime/src/execution_presentation.rs")
                && runtime_database_definitions.len() == 1
                && runtime_database_definitions[0]
                    .contains("crates/comfy_runtime/src/persistence.rs")
                && execution_presentation.contains("pub async fn dispatch_durable(")
                && execution_presentation.contains("pub async fn apply_ack_durable(")
                && execution_presentation.contains("pub async fn reconcile_durable(")
                && execution_presentation.contains("pub async fn restore_profile(")
                && execution_presentation.contains(".replace_execution_state(")
                && execution_presentation.contains(".load_execution_state(")
                && runtime_persistence.contains("replace_comfy_execution_profile_state")
                && execution_ui_production.contains(".dispatch_durable(")
                && execution_ui_production.contains(".initialize_profile_durable(")
                && execution_ui_production.contains(".set_snapshot_status_durable(")
                && execution_ui_production.contains(".apply_ack_durable(")
                && execution_ui_production.contains(".reconcile_durable(")
                && execution_ui_production.contains(".restore_profile(")
                && !execution_ui_production.contains("load_execution_attempts_for_profile")
                && sim_cli.contains("presentation.restore_profile(profile_id)")
                && sim_cli.contains(".set_snapshot_status_durable(")
                && !sim_cli.contains("load_execution_attempts_for_profile"),
        ),
        (
            "execution_owner_has_no_mutable_bypass",
            execution_owner_deref_impls.is_empty()
                && !execution_owner_impl.contains("pub fn lock(")
                && !execution_owner_impl.contains("pub(crate) fn lock(")
                && execution_owner_impl.contains("fn service(")
                && execution_owner_impl.contains("pub fn snapshot(")
                && execution_owner_impl.contains("pub fn termination_intent(")
                && private_reducer_call_sites.iter().all(|location| {
                    location.contains("crates/comfy_runtime/src/execution_presentation.rs")
                })
                && !execution_presentation.contains("ExecutionPresentationMutation")
                && execution_presentation
                    .contains("pub(crate) struct ExecutionActuatorBatchValidator")
                && execution_presentation.contains("staged: ExecutionPresentationService")
                && execution_presentation.contains("staged: service.stage_clone()")
                && execution_presentation
                    .contains("pub(crate) async fn apply_actuator_event_transaction_durable")
                && runtime_controller_production
                    .contains(".apply_actuator_event_transaction_durable(")
                && runtime_controller_production.contains("validator.validate(&event_inputs)"),
        ),
        (
            "runtime_database_has_no_per_attempt_write_api",
            public_per_attempt_persistence_apis.is_empty()
                && runtime_persistence.contains("pub async fn replace_execution_profile(")
                && runtime_persistence
                    .contains("validate_profile_projection(profile.profile_id, &attempts)?")
                && runtime_persistence
                    .contains("with_savepoint(\"replace_comfy_execution_profile_state\"")
                && runtime_persistence.contains("fn replace_execution_state(")
                && execution_owner_impl.contains(".replace_execution_state("),
        ),
        (
            "execution_output_operations_use_the_durable_owner",
            execution_presentation.contains("pub async fn apply_output_operation_durable(")
                && execution_presentation
                    .contains("pub async fn apply_output_operation_transaction_durable(")
                && execution_presentation.contains("staged.apply_output_operation(")
                && execution_presentation.contains("persist_staged_profile(profile_id, &staged)")
                && execution_ui_production.contains("pub(crate) fn handle_output_operation(")
                && execution_ui_production.contains(".output_operation_eligibility(")
                && execution_ui_production.contains(".apply_output_operation_durable(")
                && execution_ui_production.contains(".prepare_removal(")
                && execution_ui_production.contains(".commit_removal_and_register(")
                && execution_ui_production.contains(".rollback_removal(")
                && !execution_ui_production.contains(".delete(&identity")
                && !execution_ui_production.contains("enum ExecutionOutputOperationAction")
                && !execution_ui_production.contains("output_availability_overrides")
                && !runtime_assets.contains("confirmed: bool"),
        ),
        (
            "attempt_cancellation_token_reaches_every_controller_stage",
            execution_presentation.contains("cancellation_tokens:")
                && execution_presentation.contains("pub fn cancellation_token(")
                && runtime_controller_production
                    .contains("cancellation: lease.cancellation.clone()")
                && runtime_controller_production.contains("&lease.cancellation")
                && runtime_controller_production.contains("&active.cancellation")
                && runtime_controller_production.contains("fn canonical_termination_kind(")
                && runtime_controller_production.contains("active.cancellation.is_cancelled()")
                && !runtime_controller_production.contains("enum NativeTerminationIntent")
                && !runtime_controller_production
                    .contains("Some(NativeTerminationIntent::Cancel) | None"),
        ),
        (
            "capability_wire_round_trip",
            wire_round_trip
                && Capability::parse_wire_identifier("asset:execute:output").is_err()
                && Capability::parse_wire_identifier("provider_network:missing-endpoint").is_err(),
        ),
        (
            "host_grant_and_trust_duplicates_removed",
            source_occurrences(&sources, "struct CapabilityGrant {").is_empty()
                && source_occurrences(&sources, "struct TrustStore {").is_empty(),
        ),
        (
            "native_service_rights_are_least_privilege",
            output_committer.require(&output_write).is_ok()
                && output_committer.require(&output_read).is_err()
                && output_committer.require(&output_delete).is_err()
                && output_ui.require(&output_read).is_ok()
                && output_ui.require(&output_delete).is_ok()
                && output_ui.require(&output_write).is_err(),
        ),
        (
            "native_consumers_share_the_profile_asset_service",
            plugin_adapter_preserves_owner
                && runtime_controller.contains("pub assets: SharedAssetService")
                && runtime_controller.contains(".read_verified(")
                && !runtime_controller.contains("pub roots: AssetRoots")
                && sim_bootstrap.contains("open_native_profile_asset_service(")
                && sim_cli.contains("open_native_profile_asset_service(")
                && sim_bootstrap.contains("NativeExecutionControllerConfig::new(")
                && sim_cli.contains("NativeExecutionControllerConfig::new("),
        ),
        (
            "no_raw_signature_authority_boolean",
            source_occurrences(&sources, "signature_verified: bool").is_empty(),
        ),
        (
            "no_untyped_worker_capability_decision",
            source_occurrences(&sources, "capabilities: Vec<String>")
                .into_iter()
                .all(|location| !location.contains("crates/comfy_")),
        ),
        (
            "permission_policy_is_single_owner",
            permission_policy_definitions.len() == 1
                && permission_policy_definitions[0]
                    .contains("crates/comfy_runtime/src/permissions.rs"),
        ),
        (
            "native_api_ownership_catalog_is_closed",
            native_api_policy_trace && native_api_catalog_trace,
        ),
        (
            "ownership_catalog_has_only_explicit_incomplete_closure_tasks_for_unresolved_rows",
            validation != "VAL-OWNERSHIP-001"
                || ownership_catalog_has_only_accounted_pending_rows,
        ),
        (
            "task_67_semantic_owner_mappings_are_normative",
            task_67_policy_mappings_are_declared,
        ),
        (
            "task_67_external_kernel_foundations_have_one_owner",
            task_67_external_kernel_foundations_have_one_owner,
        ),
        (
            "task_68_semantic_owner_mappings_are_normative",
            task_68_policy_mappings_are_declared,
        ),
        (
            "task_69_semantic_owner_mappings_are_normative",
            task_69_policy_mappings_are_declared,
        ),
        (
            "task_68_bilinear_sampling_has_one_owner",
            task_68_bilinear_sampling_has_one_owner,
        ),
        (
            "task_68_color_traversal_has_one_owner",
            task_68_color_traversal_has_one_owner,
        ),
        (
            "task_68_external_kernel_contexts_map_canonical_cancellation",
            task_68_external_kernel_contexts_map_canonical_cancellation,
        ),
        (
            "task_68_normalization_delegates_functional_owner",
            task_68_normalization_delegates_functional_owner,
        ),
        (
            "task_68_model_state_delegates_native_module",
            task_68_model_state_delegates_native_module,
        ),
        (
            "task_68_rgb8_boundary_is_a_focused_tensor_adapter",
            task_68_rgb8_boundary_is_a_focused_tensor_adapter,
        ),
        (
            "task_69_adapters_delegate_canonical_owners",
            task_69_adapters_delegate_canonical_owners,
        ),
        (
            "task_69_external_kernel_contexts_map_canonical_cancellation",
            task_69_external_kernel_contexts_map_canonical_cancellation,
        ),
        (
            "task_68_deform_sampling_has_one_owner",
            task_68_deform_sampling_has_one_owner,
        ),
        (
            "reopened_policy_mappings_match_live_canonical_owners",
            reopened_policy_mappings_are_declared,
        ),
        (
            "cartesian_product_traversal_has_one_owner",
            cartesian_product_traversal_has_one_owner,
        ),
        (
            "rmsprop_state_machine_has_one_owner",
            rmsprop_state_machine_has_one_owner,
        ),
        (
            "cumulative_scan_traversal_has_one_owner",
            cumulative_scan_traversal_has_one_owner,
        ),
        (
            "execution_context_has_one_owner",
            execution_context_has_one_owner,
        ),
        (
            "primitive_operation_semantics_have_one_owner",
            primitive_operation_semantics_have_one_owner,
        ),
        (
            "flat_unique_semantics_have_one_owner",
            flat_unique_semantics_have_one_owner,
        ),
        (
            "task50_foundational_boundaries_have_one_owner",
            task50_foundational_boundaries_have_one_owner,
        ),
        (
            "task51_foundational_boundaries_have_one_owner",
            task51_foundational_boundaries_have_one_owner,
        ),
        (
            "task52_foundational_boundaries_have_one_owner",
            task52_foundational_boundaries_have_one_owner,
        ),
        (
            "task53_foundational_boundaries_have_one_owner",
            task53_foundational_boundaries_have_one_owner,
        ),
        (
            "task54_foundational_boundaries_have_one_owner",
            task54_foundational_boundaries_have_one_owner,
        ),
        (
            "task55_foundational_boundaries_have_one_owner",
            task55_foundational_boundaries_have_one_owner,
        ),
        (
            "task56_foundational_boundaries_have_one_owner",
            task56_foundational_boundaries_have_one_owner,
        ),
        (
            "task57_foundational_boundaries_have_one_owner",
            task57_foundational_boundaries_have_one_owner,
        ),
        (
            "task58_foundational_boundaries_have_one_owner",
            task58_foundational_boundaries_have_one_owner,
        ),
        (
            "task59_foundational_boundaries_have_one_owner",
            task59_foundational_boundaries_have_one_owner,
        ),
        (
            "task60_foundational_boundaries_have_one_owner",
            task60_foundational_boundaries_have_one_owner,
        ),
        (
            "task61_foundational_boundaries_have_one_owner",
            task61_foundational_boundaries_have_one_owner,
        ),
        (
            "task62_foundational_boundaries_have_one_owner",
            task62_foundational_boundaries_have_one_owner,
        ),
        (
            "task63_foundational_boundaries_have_one_owner",
            task63_foundational_boundaries_have_one_owner,
        ),
        (
            "task64_foundational_boundaries_have_one_owner",
            task64_foundational_boundaries_have_one_owner,
        ),
        (
            "task65_foundational_boundaries_have_one_owner",
            task65_foundational_boundaries_have_one_owner,
        ),
        (
            "future_native_ffi_adapter_mapping_activates_with_owning_task",
            native_ffi_activation_policy_trace,
        ),
        (
            "rocm_ffi_certification_has_one_authority_and_a_checked_adapter",
            rocm_ffi_certification_has_one_authority_and_a_checked_adapter,
        ),
        (
            "native_library_image_capture_and_sealing_have_one_owner",
            native_library_image_capture_and_sealing_have_one_owner,
        ),
        (
            "rocm_package_trust_policy_names_one_authority",
            rocm_package_trust_policy_trace,
        ),
        (
            "rocm_package_trust_has_one_authority_and_explicit_adapters",
            rocm_package_trust_has_one_authority_and_explicit_adapters,
        ),
        (
            "metal_abi_foundation_is_observation_only",
            metal_abi_foundation_is_observation_only,
        ),
        (
            "metal_package_trust_policy_names_one_authority",
            metal_package_trust_policy_trace,
        ),
        (
            "metal_package_trust_has_one_authority_and_explicit_adapters",
            metal_package_trust_has_one_authority_and_explicit_adapters,
        ),
        (
            "native_package_admission_uses_one_security_owner",
            validation != "VAL-OWNERSHIP-001"
                || native_package_admission_uses_one_security_owner,
        ),
        (
            "mlu_abi_foundation_requires_canonical_runtime_certification",
            mlu_abi_foundation_requires_canonical_runtime_certification,
        ),
        (
            "directml_abi_foundation_requires_canonical_runtime_certification",
            directml_abi_foundation_requires_canonical_runtime_certification,
        ),
        (
            "directml_compute_and_gpui_rendering_raw_abi_owners_are_separate",
            directml_compute_and_gpui_rendering_raw_abi_owners_are_separate,
        ),
        (
            "directml_package_trust_has_one_authority_and_explicit_adapters",
            directml_package_trust_has_one_authority_and_explicit_adapters,
        ),
        (
            "mlu_directml_execution_resources_preserve_canonical_owners",
            mlu_directml_execution_resources_preserve_canonical_owners,
        ),
        (
            "npu_abi_foundation_requires_canonical_runtime_certification",
            npu_abi_foundation_requires_canonical_runtime_certification,
        ),
        (
            "corex_structural_foundation_preserves_provenance_blocker",
            corex_structural_foundation_preserves_provenance_blocker,
        ),
        (
            "xpu_abi_foundation_requires_canonical_runtime_certification",
            xpu_abi_foundation_requires_canonical_runtime_certification,
        ),
        (
            "cuda_abi_foundation_requires_canonical_runtime_certification",
            cuda_abi_foundation_requires_canonical_runtime_certification,
        ),
        (
            "tensor_backend_resource_registries_are_authoritative",
            tensor_backend_resource_registries_are_authoritative,
        ),
        (
            "metal_execution_resources_have_one_opaque_owner",
            metal_execution_resources_have_one_opaque_owner,
        ),
        (
            "native_api_idempotency_has_one_durable_owner",
            source_occurrences(&sources, "pub struct IdempotencyLedger").len() == 1
                && api_security.contains("pub struct IdempotencyLedger")
                && api_security.contains("pub struct ArtifactIdempotencySnapshotStore")
                && api_security_production.contains(".write_private_file(&self.relative_path")
                && api_security_production.contains(".read_private_file(&self.relative_path")
                && !api_security_production.contains("fs::rename")
                && !api_security_production.contains("OpenOptions")
                && !api_http.contains("struct ReplayableResponse")
                && !api_http.contains("struct IdempotencyLedger")
                && api_services.contains(".command_receipt_state(")
                && sim_cli.contains("ArtifactIdempotencySnapshotStore::from_directory("),
        ),
        (
            "native_api_uses_injected_canonical_permissions",
            api_security.contains("permission_policy: Arc<PermissionPolicy>")
                && api_security.contains("authorize_plugin_routes(")
                && !api_security_production.contains("PermissionPolicy::new(")
                && api_host_production
                    .contains("permission_policy: Arc<comfy_runtime::PermissionPolicy>")
                && api_host_production
                    .contains("authorize_native_api_asset_reader(&permission_policy)")
                && api_services
                    .contains("asset_reader_authorization: Option<AuthorizedCapabilities>")
                && !api_services.contains("authorize_native_plugin_asset_broker")
                && runtime_permissions.contains("NATIVE_API_ASSET_READER_SUBJECT")
                && runtime_permissions.contains("pub fn authorize_native_api_asset_reader("),
        ),
        (
            "native_api_security_gate_owns_preflight",
            api_security.contains("pub fn authorize_preflight(")
                && api_host_production.contains("self.security.authorize_preflight(&preflight)")
                && !api_host_production
                    .contains("trusted_reverse_proxies\n                    .contains")
                && !api_host_production.contains("allowed_origins.contains(origin)"),
        ),
        (
            "native_api_websocket_session_has_one_principal_owner",
            api_websocket.contains("struct ClientSession {")
                && api_websocket.contains("principal: AuthenticatedPrincipal")
                && api_websocket.contains("connect_authenticated_with_session_id")
                && api_websocket.contains("pub fn authenticated_principal(")
                && !api_host.contains("websocket_principals")
                && api_host_production.contains(".connect_authenticated_with_session_id(")
                && api_host_production.contains(".authenticated_principal(client_id)"),
        ),
        (
            "native_api_target_decoding_is_single_pass",
            api_http.contains("pub(crate) fn decode_uri_component(")
                && api_transport.contains("http::decode_uri_component(path, false)")
                && api_transport.contains("Ok((path.to_owned(), parameters))")
                && !api_transport.contains("fn percent_decode(")
                && !api_transport.contains("let path = percent_decode("),
        ),
        (
            "native_api_transport_and_request_limits_are_distinct",
            api_transport.contains("pub maximum_connections: usize")
                && api_transport.contains("let maximum_connections = config.maximum_connections")
                && !api_transport
                    .contains("host.security_config().limits.maximum_concurrent_requests"),
        ),
        (
            "runtime_supervisor_does_not_own_api_lifecycle",
            !runtime_supervisor.contains("ApiHostReady")
                && !runtime_supervisor.contains("ModelIndexReady")
                && !runtime_supervisor.contains("mark_api_host_ready")
                && !runtime_supervisor.contains("mark_model_index_ready"),
        ),
        (
            "native_api_never_publishes_outputs_or_files",
            !api_services.contains("OutputCommitter")
                && !api_services.contains("fs::rename")
                && !api_host_production.contains("fs::rename")
                && !api_host_production.contains("write_contained_file"),
        ),
        (
            "graph_context_dispatch_is_single_enforced_owner",
            graph_context_binding_definitions.len() == 1
                && graph_context_binding_definitions[0]
                    .contains("crates/comfy_ui/src/context_menu.rs")
                && graph_context_dispatch_definitions.len() == 1
                && graph_context_dispatch_definitions[0]
                    .contains("crates/comfy_ui/src/context_menu.rs")
                && graph_context_dispatch_sites.len() == 3
                && graph_context_dispatch_sites
                    .iter()
                    .all(|location| location.contains("crates/comfy_ui/src/context_menu.rs"))
                && graph_context_policy_trace
                && graph_context_catalog_trace
                && ownership_generator.contains("if len(matching_hits) != 1:"),
        ),
        (
            "output_commit_is_single_transaction_owner",
            output_committer_definitions.len() == 1
                && output_committer_definitions[0]
                    .contains("crates/comfy_runtime/src/output_committer.rs")
                && output_committer_source.contains("commit_proposal_batch_and_register")
                && output_committer_source.contains(
                    "assets.register_committed_outputs(identities, capabilities, cancellation)?;",
                )
                && output_committer_source.contains("committer.persist_journal()?;")
                && output_committer_source.contains("projection_metadata")
                && output_committer_source.contains("committed_execution_scopes")
                && runtime_controller_production.contains("reconcile_committed_output_receipts")
                && runtime_controller_production
                    .contains(".apply_actuator_event_transaction_durable(")
                && runtime_controller_production
                    .contains(".commit_scoped_proposal_batch_and_register_with_precommit(")
                && plugin_host_production_capabilities
                    .contains("pub struct PluginOutputPublicationAdapter")
                && plugin_host_production_capabilities
                    .contains("commit_scoped_proposal_batch_and_register_now")
                && recovery_production.contains("record_output_receipt")
                && !recovery_production.contains("pub fn prepare_output(")
                && !recovery_production.contains("pub fn commit_output(")
                && !recovery_production.contains("enum RecoveryPhase")
                && !plugin_host_production_capabilities.contains("fs::rename")
                && !plugin_host_production_capabilities.contains("std::fs"),
        ),
        ("plugin_capability_round_trip", plugin_round_trip),
        (
            "plugin_capability_dto_mapping_is_exhaustive_and_host_delegated",
            runtime_permissions.contains("CapabilityKind::Filesystem =>")
                && runtime_permissions.contains("CapabilityKind::NetworkProvider =>")
                && runtime_permissions.contains("CapabilityKind::Secret =>")
                && runtime_permissions.contains("CapabilityKind::Clock =>")
                && runtime_permissions.contains("CapabilityKind::Randomness =>")
                && runtime_permissions.contains("CapabilityKind::Model =>")
                && runtime_permissions.contains("CapabilityKind::TransactionalOutput =>")
                && runtime_permissions.contains("CapabilityKind::SanitizedLog =>")
                && runtime_permissions.contains("CapabilityKind::DeclarativeUi =>")
                && runtime_permissions.contains("CapabilityKind::Route =>")
                && runtime_permissions.contains(
                    "fn every_plugin_capability_kind_preserves_its_exact_runtime_scope()",
                )
                && plugin_host_capabilities.contains("Capability::from_plugin_request(request)"),
        ),
        (
            "provider_policy_is_single_security_owner",
            provider_policy_definitions.len() == 1
                && provider_policy_definitions[0].contains("crates/comfy_runtime/src/trust.rs")
                && runtime_plugin_services_production
                    .contains("self.require_capability(&Capability::ProviderNetwork")
                && runtime_plugin_services_production
                    .contains(".provider_policy\n            .authorize(")
                && !plugin_host_production_capabilities.contains("provider_policy: ProviderPolicy")
                && !plugin_component_host.contains("ProviderPolicy")
                && !plugin_private_worker.contains("ProviderPolicy")
                && sim_plugin_services.contains(".uri(request.endpoint())")
                && !sim_plugin_services.contains("validated_provider_url")
                && !sim_plugin_services.contains("Url::parse")
                && !sim_plugin_services.contains("url::Host"),
        ),
        (
            "canonical_provider_and_secret_security_precedes_actuators",
            provider_security_precedes_actuators
                && runtime_plugin_services.contains(
                    "fn capability_denials_happen_before_provider_or_credential_actuators()",
                )
                && runtime_plugin_services.contains(
                    "fn provider_endpoint_policy_cannot_be_bypassed_by_a_broader_capability()",
                )
                && runtime_plugin_services
                    .contains("assert_eq!(provider.calls.load(Ordering::Acquire), 0);")
                && runtime_plugin_services
                    .contains("assert_eq!(credential.calls.load(Ordering::Acquire), 0);"),
        ),
        (
            "plugin_trust_policy_is_single_owner",
            plugin_trust_definitions.len() == 1
                && plugin_trust_definitions[0].contains("crates/comfy_runtime/src/trust.rs"),
        ),
        (
            "external_navigation_policy_is_the_only_comfy_authorizer",
            external_navigation_semantics
                && external_navigation_policy_definitions.len() == 1
                && external_navigation_policy_definitions
                    .first()
                    .is_some_and(|location| location.contains("crates/comfy_runtime/src/trust.rs"))
                && comfy_open_url_calls.len() == 1
                && comfy_open_url_calls.first().is_some_and(|location| {
                    location.contains("crates/comfy_ui/src/execution_panel.rs")
                })
                && execution_panel_production
                    .contains("self.external_navigation_policy.authorize(url, true)")
                && execution_panel_production.contains("cx.open_url(url);"),
        ),
        (
            "plugin_host_is_extension_lifecycle_adapter",
            component_lifecycle_adapter_impls.len() == 1
                && component_lifecycle_adapter_impls[0]
                    .contains("crates/comfy_plugin_host/src/component_host.rs")
                && plugin_component_host
                    .contains("use extension_host::{ComponentLifecycleAdapter, ComponentRuntime")
                && plugin_component_host
                    .contains("impl ComponentLifecycleAdapter for ComponentHostRouter")
                && !plugin_component_host
                    .contains("impl ComponentLifecycleAdapter for ComponentHost {")
                && extension_host.contains("pub trait ComponentLifecycleAdapter: Send + Sync")
                && sim_bootstrap.contains("ComponentHostRouter::with_initial_generation(")
                && sim_bootstrap
                    .contains("extension_host::register_component_lifecycle_adapter(Arc::new(")
                && sim_bootstrap.contains("comfy_plugin_services::private_worker_services(")
                && sim_plugin_services
                    .contains("PluginCapabilityBroker::new_with_provider_cost_acceptance(")
                && sim_plugin_services.contains("ComponentExecutionBoundary::private_worker(")
                && plugin_private_worker.contains("RuntimeSupervisor::start(")
                && plugin_private_worker.contains(".execute_plugin_retaining_capabilities(")
                && worker_plugin_runtime.contains("pub(crate) struct WorkerCapabilityBridge"),
        ),
        (
            "component_compilation_is_owned_by_extension_runtime",
            extension_component_runtime.contains("pub fn compile_component(&self, bytes: &[u8])")
                && extension_component_runtime.contains("Component::new(&self.engine, bytes)")
                && plugin_host.contains("self.runtime.compile_component(bytes)")
                && !plugin_host.contains("Component::new("),
        ),
        (
            "output_boundary_dtos_map_to_the_canonical_proposal",
            worker_protocol.contains("pub struct WorkerOutputProposal")
                && worker_protocol.contains("struct WorkerOutputProposalWire")
                && worker_protocol.contains("impl TryFrom<WorkerOutputProposalWire>")
                && worker_protocol.contains("fn output_proposals_are_bounded_on_construction_and_decode()")
                && runtime_controller.contains("pub struct NativeImageOutputProposal")
                && runtime_controller.contains("pub fn to_worker_proposal(&self)")
                && runtime_controller.contains("pub fn from_worker_proposal(")
                && runtime_controller.contains("let output = OutputProposal::new(")
                && runtime_controller.contains(
                    "fn native_output_worker_adapter_round_trips_the_canonical_proposal()",
                ),
        ),
        (
            "plugin_abi_values_use_sealed_native_payload_adapters",
            plugin_host_production_capabilities.contains("fn artifact_value_identity(")
                && plugin_host_production_capabilities.contains("value: &ArtifactValue")
                && plugin_host_production_capabilities
                    .contains("Result<AssetIdentity, InvocationError>")
                && plugin_host_production_capabilities.contains("fn model_store_handle_value(")
                && plugin_host_production_capabilities.contains("model: &PluginModelHandle")
                && plugin_host_production_capabilities
                    .contains("Result<ModelValue, InvocationError>")
                && plugin_registry_adapter_production
                    .contains("artifact_value_identity(profile_id, artifact)")
                && plugin_registry_adapter_production.contains("fn plugin_value_from_stored(")
                && plugin_registry_adapter_production
                    .contains("NativeStoredPayload::Tensor(stored)")
                && plugin_registry_adapter_production
                    .contains("NativeStoredPayload::Model(stored)")
                && plugin_registry_adapter_production
                    .contains("NativeStoredPayload::Conditioning(_)")
                && plugin_registry_adapter_production.contains("unmaterialized_plugin_input(")
                && plugin_registry_adapter_production
                    .contains("NativeStoredPayload::Control(stored)")
                && plugin_registry_adapter_production
                    .contains("NativeStoredPayload::Provider(stored)")
                && plugin_registry_adapter_production.contains("fn imported_runtime_value(")
                && plugin_registry_adapter_production
                    .contains("unmaterialized_plugin_output(&port.id)")
                && !plugin_registry_adapter_production.contains("NativeProviderPayload::checked(")
                && !plugin_registry_adapter_production.contains("NativeStoredObject")
                && !plugin_registry_adapter_production.contains("NativeStoredTensorObject")
                && !plugin_registry_adapter_production.contains("NativeStoredArtifactObject")
                && !plugin_registry_adapter_production.contains("NativeStoredModelObject")
                && !plugin_registry_adapter_production.contains(".downcast::<")
                && plugin_registry_adapter.contains("fn invocation_inputs(")
                && plugin_registry_adapter.contains("fn invocation_outputs("),
        ),
        (
            "private_worker_preserves_authoritative_component_limits_and_diagnostics",
            plugin_component_host.contains("component_limits: ComponentLimits")
                && plugin_component_host.contains("self.inner.plugin_host.limits().clone()")
                && worker_process
                    .contains("let component_limits = invocation.component_limits().clone()")
                && worker_process.contains("&registry.source,")
                && worker_process.contains("component_limits,")
                && worker_plugin_runtime_production.contains("component_limits: ComponentLimits")
                && worker_plugin_runtime_production.contains("component_limits.clone()")
                && !worker_plugin_runtime_production.contains("ComponentLimits::default()")
                && worker_protocol.contains("MAX_WORKER_PLUGIN_DIAGNOSTIC_CHARS")
                && worker_protocol.contains("Trap { diagnostic: String }")
                && plugin_private_worker
                    .contains("WorkerPluginExecutionFailure::Trap { diagnostic }"),
        ),
        (
            "secret_id_delegates_to_the_existing_credentials_provider",
            sim_plugin_services.contains("sim_credentials_provider::global(cx)")
                && sim_plugin_services.contains(".read_credentials(&command.secret_id, cx)")
                && sim_plugin_services.contains(".read(request.secret_id().as_str())?")
                && sim_plugin_services
                    .contains("impl CredentialPresenceActuator for SimCredentialActuator")
                && !sim_plugin_services.contains("fs::write")
                && !sim_plugin_services.contains("std::fs"),
        ),
        (
            "desktop_api_headless_and_worker_consume_component_registry",
            sim_bootstrap.contains("component_host.router.active_execution_registry_bundle()?")
                && sim_bootstrap.contains("worker.with_registry_deployment(")
                && api_host_production.contains("pub fn with_registry_bundle(")
                && api_host_production.contains("NativeRuntimeHttpServices::from_registry_bundle(")
                && api_services
                    .contains("for (class_type, runtime) in self.registry.descriptors()")
                && api_services.contains("project_component_node(")
                && api_headless.contains("active.runtime.host()")
                && plugin_private_worker.contains("command.deployment")
                && plugin_private_worker.contains("supervisor.deploy_registry("),
        ),
        (
            "task367_production_consumers_use_the_comprehensive_generated_registry",
            runtime_executor.contains("pub fn validate_comprehensive_bindings(")
                && execution_ui_production.contains("generated_native_frontend_contracts(None)")
                && execution_ui_production.contains("compile_generated_native_prompt(")
                && execution_ui_production.contains("graph_to_prompt(")
                && !execution_ui_production.contains("compile_native_image_workflow(")
                && sim_bootstrap
                    .contains("generated_native_node_registry_projection(None)?")
                && !sim_bootstrap.contains("comfy_runtime::native_image_registry_projection()?")
                && api_host_production.contains("pub fn with_registry(")
                && api_services
                    .contains("for (class_type, runtime) in self.registry.descriptors()")
                && worker_process.contains("RegistryDeploymentCommit")
                && worker_process.contains("apply_compiled_registry_commit(")
                && worker_process.contains("worker_plan.validate()?")
                && plugin_registry_adapter.contains("fn invocation_inputs(")
                && plugin_registry_adapter.contains("BTreeMap<String, NativeValue>")
                && plugin_registry_adapter.contains("fn invocation_outputs(")
                && plugin_registry_adapter.contains("context.handle_store()")
                && plugin_registry_adapter.contains("NativeValue::Primitive")
                && plugin_registry_adapter.contains("NativeValue::PreservedUnknown")
                && plugin_registry_adapter.contains("NativeValue::List")
                && plugin_registry_adapter.contains("NativeValue::Handle")
                && !plugin_registry_adapter.contains("serde_json::to_value")
                && !plugin_registry_adapter.contains("serde_json::from_value")
                && runtime_controller_production
                    .contains("pub fn generated_native_node_registry_projection(")
                && runtime_controller_production
                    .contains("pub fn generated_native_frontend_descriptors(")
                && runtime_controller_production
                    .contains("pub fn compile_generated_native_prompt(")
                && runtime_controller_production
                    .contains("pub fn native_image_registry_projection(")
                && runtime_controller_production
                    .contains("pub fn compile_native_image_workflow("),
        ),
        (
            "signed_component_presentation_has_one_checked_projection_owner",
            production_source_occurrences(&sources, "pub struct NativeNodePresentation").len()
                == 1
                && runtime_executor
                    .contains("presentations: BTreeMap<String, RuntimeNodePresentation>")
                && runtime_executor.contains("fn register_bound_batch_internal(")
                && plugin_registry_adapter
                    .contains("fn native_presentation(node: &PluginNode) -> NativeNodePresentation")
                && plugin_registry_adapter.contains("display_name: node.display_name.clone()")
                && plugin_registry_adapter.contains("category: node.category.clone()")
                && plugin_registry_adapter.contains(".map(|port| port.name.clone())")
                && plugin_registry_adapter
                    .contains("register_bound_batch_with_presentations(ordinary_bindings)")
                && plugin_registry_adapter.contains("let presentation = base")
                && plugin_registry_adapter.contains(".presentation(&node.id)")
                && api_services.contains("self.registry.presentation(class_type)")
                && api_services.contains("native_node_presentation_missing")
                && api_services.contains(
                    "let (outputs, output_names, output_tooltips) = if let Some(schema)",
                )
                && api_services.contains("runtime.source_schema.as_ref()")
                && !api_services.contains("let output_names = presentation")
                && !api_services.contains("\"category\": \"extensions\"")
                && !api_services.contains("format!(\"output_{index}\")"),
        ),
        (
            "subgraph_catalog_is_an_asset_service_projection",
            subgraph_library_definitions.len() == 1
                && subgraph_library_definitions[0]
                    .contains("crates/comfy_runtime/src/subgraph_blueprints.rs")
                && subgraph_catalog_definitions.len() == 1
                && subgraph_catalog_definitions[0]
                    .contains("crates/comfy_runtime/src/subgraph_blueprints.rs")
                && subgraph_catalog_type_sites.len() == 15
                && subgraph_catalog_type_sites
                    .iter()
                    .filter(|location| {
                        location.contains("crates/comfy_runtime/src/subgraph_blueprints.rs")
                    })
                    .count()
                    == 8
                && subgraph_catalog_type_sites
                    .iter()
                    .filter(|location| location.contains("crates/comfy_ui/src/comfy_ui.rs"))
                    .count()
                    == 7
                && subgraph_blueprints.contains("assets: SharedAssetService")
                && subgraph_blueprints.contains(".write_exact(")
                && subgraph_blueprints.contains("assets.list_authorized(")
                && subgraph_blueprints.contains("assets.read_verified(")
                && !subgraph_blueprints.contains("std::fs")
                && !subgraph_blueprints.contains("ArtifactIndex")
                && native_asset_services.contains("assets: SharedAssetService")
                && native_asset_services
                    .contains("subgraph_blueprints.reload(&CancellationToken::default())")
                && native_asset_services.contains("subgraph_catalog: SubgraphBlueprintCatalog")
                && native_asset_services
                    .contains("pub(crate) fn native_subgraph_catalog(cx: &App)")
                && native_asset_services.contains("cx.global_mut::<GlobalNativeAssetServices>()")
                && native_asset_services.contains("mod native_asset_projection")
                && native_asset_services.contains("struct GlobalNativeAssetServices")
                && !native_asset_services.contains("pub(crate) struct GlobalNativeAssetServices")
                && native_asset_services
                    .contains("pub(crate) struct GlobalNativeSubgraphCatalogRevision")
                && native_asset_services
                    .contains("cx.global_mut::<GlobalNativeSubgraphCatalogRevision>()")
                && !native_asset_services.contains("Arc<Mutex<SubgraphBlueprintCatalog>>")
                && !native_asset_services.contains("pub fn replace_native_subgraph_catalog")
                && !native_asset_services.contains("pub fn replace_subgraph_catalog")
                && !native_asset_services.contains("ArtifactIndex")
                && subgraph_catalog_owner_mutation_sites.len() == 1
                && subgraph_catalog_owner_mutation_sites[0]
                    .contains("crates/comfy_ui/src/comfy_ui.rs")
                && subgraph_catalog_refresh_sites.len() == 2
                && subgraph_catalog_refresh_sites.iter().all(|location| {
                    location.contains("crates/comfy_ui/src/comfy_ui.rs")
                        || location.contains("crates/comfy_ui/src/context_menu.rs")
                })
                && context_menu.contains("replace_native_subgraph_catalog(publication.catalog")
                && workflow_item.contains("native_subgraph_catalog(cx)")
                && subgraph_catalog_entry_consumers.len() == 3
                && subgraph_catalog_entry_consumers.iter().all(|location| {
                    location.contains("crates/comfy_ui/src/comfy_ui.rs")
                        || location.contains("crates/comfy_ui/src/workflow_item.rs")
                })
                && native_asset_services.contains("catalog.entries().len()")
                && native_asset_services.contains("entry.descriptor.display_name.as_str()")
                && workflow_item.contains("subgraph_catalog_node_library_message(catalog)")
                && workflow_item
                    .contains("observe_global::<crate::GlobalNativeSubgraphCatalogRevision>")
                && subgraph_blueprints.contains("let mut catalog = self.reload_from_assets")
                && subgraph_blueprints
                    .contains("if blueprint_byte_size > MAX_PUBLISHED_SUBGRAPH_BLUEPRINT_BYTES")
                && subgraph_blueprints.contains("validate_projected_catalog(&catalog")
                && subgraph_blueprints.contains("let asset = assets.write_exact(")
                && subgraph_transaction_is_ordered
                && blueprint_decode_is_bounded_before_parse
                && subgraph_accounting_precedes_reads
                && subgraph_projection_is_detached_and_ordered
                && runtime_graph_production
                    .contains("return Err(GraphError::BlueprintTooLarge(workflow_bytes.len()))")
                && subgraph_blueprints
                    .contains("oversized_blueprint_rejects_publication_before_asset_mutation")
                && subgraph_blueprints.contains("Err(GraphError::BlueprintTooLarge(actual))")
                && subgraph_blueprints
                    .contains("Err(SubgraphBlueprintLibraryError::BlueprintByteLimit")
                && subgraph_blueprints
                    .contains("MAX_SUBGRAPH_BLUEPRINT_CATALOG_ENTRIES: usize = 1_024")
                && subgraph_blueprints
                    .contains("MAX_SUBGRAPH_BLUEPRINT_CATALOG_BYTES: usize = 64 * 1024 * 1024")
                && subgraph_blueprints.contains("asset_identities: BTreeSet<AssetIdentity>")
                && subgraph_blueprints.contains("asset_byte_sizes: BTreeMap<AssetIdentity, usize>")
                && subgraph_blueprints.contains("catalog.asset_identities.len()")
                && subgraph_blueprints.contains("usize::try_from(asset.byte_size)")
                && subgraph_blueprints.contains(".checked_sub(")
                && subgraph_blueprints.contains(".asset_byte_sizes")
                && subgraph_blueprints
                    .contains("every_tagged_asset_byte_counts_before_publication_mutation")
                && subgraph_blueprints
                    .contains("replacement_subtracts_existing_bytes_at_catalog_boundary")
                && subgraph_blueprints
                    .contains("validate_projected_catalog(&catalog, &replacement_identity, 1)")
                && subgraph_blueprints
                    .contains("malformed_catalog_bytes_reject_publication_before_asset_mutation")
                && context_menu_tests.contains("observer_item.read_with")
                && context_menu_tests
                    .contains("announcement.contains(\"Native node library updated\")")
                && context_menu_tests.contains("item.subgraph_publish_task.is_some()")
                && context_menu_tests.contains("announcement.contains(\"Native Blend\")")
                && context_menu_tests
                    .contains("committed_subgraph_projection_survives_workspace_item_drop")
                && context_menu_tests.contains("projection_sender.send(()).is_ok()")
                && context_menu_tests
                    .contains("item.cancel_subgraph_publication_for_drop_for_test()")
                && native_asset_services.contains("cx.defer(|cx|")
                && workflow_item.contains("subgraph_publish_cancellation")
                && subgraph_drop_impl.contains("self.cancel_subgraph_publication_for_drop()")
                && subgraph_drop_cancellation_helper.contains("self.subgraph_publish_cancellation")
                && subgraph_drop_cancellation_helper.contains("cancellation.cancel()"),
        ),
        (
            "editor_and_comfy_capability_models_have_disjoint_authority",
            extension_capability_granter.contains("pub struct CapabilityGranter")
                && extension_capability_granter.contains("grant_exec(")
                && extension_capability_granter.contains("grant_download_file(")
                && extension_capability_granter.contains("grant_npm_install_package(")
                && !plugin_host.contains("CapabilityGranter")
                && plugin_host_production_capabilities.contains("pub struct CapabilityState")
                && plugin_host_production_capabilities
                    .contains("authorization: &PluginAuthorization")
                && !plugin_host_production_capabilities.contains("provider_policy: ProviderPolicy")
                && runtime_plugin_services_production.contains("pub struct PluginCapabilityBroker")
                && runtime_plugin_services_production.contains("provider_policy: ProviderPolicy")
                && plugin_capability_broker_definitions.len() == 1
                && plugin_capability_broker_definitions[0]
                    .contains("crates/comfy_runtime/src/plugin_services.rs"),
        ),
        (
            "workspace_persistence_uses_serializable_item_owner",
            workflow_item.contains("impl SerializableItem for GraphWorkspaceItem")
                && workflow_item
                    .contains("db::static_connection!(ComfyWorkflowDb, [WorkspaceDb]);")
                && !workflow_item.contains("struct RuntimeWorkspace"),
        ),
        (
            "ownership_policy_and_catalog_trace_task_20",
            task_20_policy_trace && task_20_catalog_trace,
        ),
        (
            "task27_prompt_compiler_and_cache_have_one_owner",
            task_27_prompt_compiler_and_cache_have_one_owner,
        ),
        (
            "task27_controller_and_engine_delegate_compilation_and_cache",
            task_27_controller_and_engine_delegate_compilation_and_cache,
        ),
        (
            "task27_autograd_checkpoints_are_ephemeral_and_not_restartable",
            task_27_autograd_checkpoints_are_ephemeral_and_not_restartable,
        ),
        (
            "task27_recovery_journal_only_records_checked_immutable_output_receipts",
            task_27_recovery_journal_only_records_checked_immutable_output_receipts,
        ),
        (
            "task101_tensor_identity_and_mutation_lineage_have_one_owner",
            task_101_policy_trace
                && task_101_catalog_trace
                && task_101_tensor_identity_and_mutation_lineage_have_one_owner,
        ),
        (
            "task101_tape_gradient_store_and_checkpoint_have_one_owner",
            task_101_policy_trace
                && task_101_catalog_trace
                && task_101_tape_gradient_store_and_checkpoint_have_one_owner,
        ),
        (
            "task101_adapters_preserve_canonical_autograd_semantics",
            task_101_adapters_preserve_canonical_autograd_semantics,
        ),
        ("task102_policy_declares_every_required_mapping", task_102_policy_trace),
        ("task102_catalog_traces_the_canonical_owner", task_102_catalog_trace),
        (
            "task102_quantized_adapters_delegate_the_single_quantization_owner",
            task_102_quantization_has_one_owner,
        ),
        (
            "task511_policy_declares_every_required_owner_mapping",
            task_511_policy_trace,
        ),
        (
            "task511_catalog_traces_every_canonical_owner",
            task_511_catalog_trace,
        ),
        (
            "task511_adapters_delegate_canonical_owners",
            task_511_adapters_delegate_canonical_owners,
        ),
        (
            "task512_policy_and_catalog_trace_canonical_tokenizer_owners",
            task_512_policy_trace && task_512_catalog_trace,
        ),
        (
            "task512_has_one_canonical_tokenizer_parser_and_artifact_owner",
            task_512_has_one_canonical_tokenizer_and_artifact_owner,
        ),
        (
            "task512_adapters_delegate_without_unverified_bypasses",
            task_512_adapters_delegate_without_unverified_bypasses,
        ),
        (
            "task512_native_diffusion_binds_canonical_tokenizer_identity",
            task_512_native_diffusion_binds_canonical_tokenizer_identity,
        ),
        (
            "task383_policy_and_catalog_trace_qwen2_tokenizer",
            task383_policy_trace && task383_catalog_trace,
        ),
        (
            "task383_qwen2_has_one_native_prompt_tokenizer_family",
            task383_qwen2_has_one_native_prompt_tokenizer_family,
        ),
        (
            "task383_qwen2_admission_identity_and_residency_are_canonical",
            task383_qwen2_admission_identity_and_residency_are_canonical,
        ),
        (
            "task383_qwen2_source_fixtures_and_failures_are_executable",
            task383_qwen2_source_fixtures_and_failures_are_executable,
        ),
        (
            "task391_policy_and_catalog_trace_gemma_tokenizer",
            task391_policy_trace && task391_catalog_trace,
        ),
        (
            "task391_gemma_has_one_native_prompt_tokenizer_family",
            task391_gemma_has_one_native_prompt_tokenizer_family,
        ),
        (
            "task391_gemma_admission_identity_residency_and_cleanup_are_canonical",
            task391_gemma_admission_identity_residency_and_cleanup_are_canonical,
        ),
        (
            "task391_gemma_source_fixtures_and_failures_are_executable",
            task391_gemma_source_fixtures_and_failures_are_executable,
        ),
        (
            "task339_policy_and_catalog_trace_the_canonical_clip_text_owner",
            task_339_policy_trace && task_339_catalog_trace,
        ),
        (
            "task339_clip_text_has_one_architecture_owner",
            task_339_clip_text_has_one_architecture_owner,
        ),
        (
            "task339_clip_text_delegates_canonical_mechanics",
            task_339_clip_text_delegates_canonical_mechanics,
        ),
        (
            "task339_clip_text_adapter_semantics_are_executable",
            task_339_clip_text_adapter_semantics_are_executable,
        ),
        (
            "task342_policy_and_catalog_trace_the_canonical_t5_bidirectional_owner",
            task_342_policy_trace && task_342_catalog_trace,
        ),
        (
            "task342_t5_bidirectional_has_one_architecture_owner",
            task_342_t5_bidirectional_has_one_architecture_owner,
        ),
        (
            "task342_t5_bidirectional_delegates_canonical_mechanics",
            task_342_t5_bidirectional_delegates_canonical_mechanics,
        ),
        (
            "task342_t5_bidirectional_adapter_semantics_are_executable",
            task_342_t5_bidirectional_adapter_semantics_are_executable,
        ),
        (
            "task343_policy_and_catalog_trace_the_canonical_decoder_owner",
            task343_policy_trace && task343_catalog_trace,
        ),
        (
            "task343_decoder_llm_graph_and_transient_cache_have_one_owner",
            task343_decoder_llm_graph_and_transient_cache_have_one_owner,
        ),
        (
            "task343_decoder_delegates_every_foundational_mechanic",
            task343_decoder_delegates_every_foundational_mechanic,
        ),
        (
            "task343_decoder_adapters_and_failure_atomicity_are_executable",
            task343_decoder_adapters_and_failure_atomicity_are_executable,
        ),
        (
            "task380_policy_and_catalog_trace_the_prepared_decoder_boundary",
            task380_policy_trace && task380_catalog_trace,
        ),
        (
            "task380_prepared_decoder_has_one_borrowed_invocation_local_owner",
            task380_prepared_decoder_has_one_borrowed_invocation_local_owner,
        ),
        (
            "task380_prepared_decoder_delegates_one_graph_cache_rng_and_rope_owner",
            task380_prepared_decoder_delegates_one_graph_cache_rng_and_rope_owner,
        ),
        (
            "task380_prepared_decoder_boundaries_are_executable",
            task380_prepared_decoder_boundaries_are_executable,
        ),
        (
            "task382_policy_and_catalog_trace_prepared_deepstack",
            task382_policy_trace && task382_catalog_trace,
        ),
        (
            "task382_deepstack_has_one_borrowed_decoder_owner",
            task382_deepstack_has_one_borrowed_decoder_owner,
        ),
        (
            "task382_deepstack_delegates_canonical_prefill_cache_rng_and_indexing",
            task382_deepstack_delegates_canonical_prefill_cache_rng_and_indexing,
        ),
        (
            "task382_deepstack_boundaries_are_executable",
            task382_deepstack_boundaries_are_executable,
        ),
        (
            "task384_policy_and_catalog_trace_qwen3_query_key_norm",
            task384_policy_trace && task384_catalog_trace,
        ),
        (
            "task384_qwen3_query_key_norm_has_one_decoder_owner",
            task384_qwen3_query_key_norm_has_one_decoder_owner,
        ),
        (
            "task384_qwen3_delegates_rms_rope_attention_cache_and_residency",
            task384_qwen3_delegates_rms_rope_attention_cache_and_residency,
        ),
        (
            "task384_qwen3_fixture_is_exact_and_failure_atomic",
            task384_qwen3_fixture_is_exact_and_failure_atomic,
        ),
        (
            "task392_policy_and_catalog_trace_gemma3_decoder",
            task392_policy_trace && task392_catalog_trace,
        ),
        (
            "task392_gemma3_profiles_have_one_canonical_decoder_owner",
            task392_gemma3_profiles_have_one_canonical_decoder_owner,
        ),
        (
            "task392_gemma3_delegates_rope_norm_attention_cache_and_rng",
            task392_gemma3_delegates_rope_norm_attention_cache_and_rng,
        ),
        (
            "task392_gemma3_fixture_is_exact_and_failure_atomic",
            task392_gemma3_fixture_is_exact_and_failure_atomic,
        ),
        (
            "task393_policy_and_catalog_trace_gemma4_decoder",
            task393_policy_trace && task393_catalog_trace,
        ),
        (
            "task393_gemma4_profiles_have_one_canonical_decoder_owner",
            task393_gemma4_profiles_have_one_canonical_decoder_owner,
        ),
        (
            "task393_gemma4_delegates_head_rope_shared_kv_layer_input_cache_and_rng",
            task393_gemma4_delegates_head_rope_shared_kv_layer_input_cache_and_rng,
        ),
        (
            "task393_gemma4_fixture_is_exact_and_failure_atomic",
            task393_gemma4_fixture_is_exact_and_failure_atomic,
        ),
        (
            "task394_policy_and_catalog_trace_gemma3_vision",
            task394_policy_trace && task394_catalog_trace,
        ),
        (
            "task394_gemma3_vision_has_one_retained_projection_owner",
            task394_gemma3_vision_has_one_retained_projection_owner,
        ),
        (
            "task394_gemma3_vision_delegates_preparation_clip_projection_and_residency",
            task394_gemma3_vision_delegates_preparation_clip_projection_and_residency,
        ),
        (
            "task394_gemma3_vision_fixture_proves_exactness_aliasing_and_rollback",
            task394_gemma3_vision_fixture_proves_exactness_aliasing_and_rollback,
        ),
        (
            "task395_policy_and_catalog_trace_gemma4_vision",
            task395_policy_trace && task395_catalog_trace,
        ),
        (
            "task395_gemma4_vision_has_one_retained_projection_owner",
            task395_gemma4_vision_has_one_retained_projection_owner,
        ),
        (
            "task395_gemma4_vision_delegates_preparation_projection_and_residency",
            task395_gemma4_vision_delegates_preparation_projection_and_residency,
        ),
        (
            "task395_gemma4_vision_fixture_proves_exactness_aliasing_and_rollback",
            task395_gemma4_vision_fixture_proves_exactness_aliasing_and_rollback,
        ),
        (
            "task396_policy_and_catalog_trace_gemma4_audio",
            task396_policy_trace && task396_catalog_trace,
        ),
        (
            "task396_gemma4_audio_has_one_retained_execution_owner",
            task396_gemma4_audio_has_one_retained_execution_owner,
        ),
        (
            "task396_gemma4_audio_delegates_preparation_graph_and_residency",
            task396_gemma4_audio_delegates_preparation_graph_and_residency,
        ),
        (
            "task396_gemma4_audio_fixture_proves_exactness_capability_and_rollback",
            task396_gemma4_audio_fixture_proves_exactness_capability_and_rollback,
        ),
        (
            "task385_policy_and_catalog_trace_qwen35_hybrid_decoder",
            task385_policy_trace && task385_catalog_trace,
        ),
        (
            "task385_qwen35_hybrid_has_one_checkpoint_backed_decoder_owner",
            task385_qwen35_hybrid_has_one_checkpoint_backed_decoder_owner,
        ),
        (
            "task385_qwen35_delegates_full_gate_delta_cache_and_residency",
            task385_qwen35_delegates_full_gate_delta_cache_and_residency,
        ),
        (
            "task385_qwen35_fixture_proves_hybrid_equivalence_and_admission",
            task385_qwen35_fixture_proves_hybrid_equivalence_and_admission,
        ),
        (
            "task386_policy_and_catalog_trace_qwen_vision",
            task386_policy_trace && task386_catalog_trace,
        ),
        (
            "task386_qwen_vision_has_one_retained_checkpoint_owner",
            task386_qwen_vision_has_one_retained_checkpoint_owner,
        ),
        (
            "task386_qwen_vision_delegates_preparation_modules_attention_and_residency",
            task386_qwen_vision_delegates_preparation_modules_attention_and_residency,
        ),
        (
            "task386_qwen_vision_fixture_proves_family_exactness_and_rollback",
            task386_qwen_vision_fixture_proves_family_exactness_and_rollback,
        ),
        (
            "task387_policy_and_catalog_trace_qwen_multimodal_resource",
            task387_policy_trace && task387_catalog_trace,
        ),
        (
            "task387_qwen_resource_has_one_retained_composite",
            task387_qwen_resource_has_one_retained_composite,
        ),
        (
            "task387_qwen_resource_closes_identity_residency_and_storage",
            task387_qwen_resource_closes_identity_residency_and_storage,
        ),
        (
            "task387_qwen_resource_fixture_proves_admission_identity_and_residency",
            task387_qwen_resource_fixture_proves_admission_identity_and_residency,
        ),
        (
            "task397_policy_and_catalog_trace_gemma_multimodal_resource",
            task397_policy_trace && task397_catalog_trace,
        ),
        (
            "task397_gemma_resource_has_one_retained_composite",
            task397_gemma_resource_has_one_retained_composite,
        ),
        (
            "task397_gemma_resource_closes_family_identity_residency_and_storage",
            task397_gemma_resource_closes_family_identity_residency_and_storage,
        ),
        (
            "task397_gemma_resource_fixture_proves_family_identity_residency_and_reduced_rejection",
            task397_gemma_resource_fixture_proves_family_identity_residency_and_reduced_rejection,
        ),
        (
            "task388_policy_and_catalog_trace_qwen_multimodal_generation",
            task388_policy_trace && task388_catalog_trace,
        ),
        (
            "task388_qwen_generation_has_one_model_domain_adapter",
            task388_qwen_generation_has_one_model_domain_adapter,
        ),
        (
            "task388_qwen_generation_preserves_source_routes_and_atomicity",
            task388_qwen_generation_preserves_source_routes_and_atomicity,
        ),
        (
            "task381p_policy_and_catalog_trace_qwen_preparation",
            task381p_policy_trace && task381p_catalog_trace,
        ),
        (
            "task381p_qwen_preparation_has_one_attempt_local_owner",
            task381p_qwen_preparation_has_one_attempt_local_owner,
        ),
        (
            "task381p_qwen_preparation_is_exact_and_executable",
            task381p_qwen_preparation_is_exact_and_executable,
        ),
        (
            "task389p_policy_and_catalog_trace_gemma_preparation",
            task389p_policy_trace && task389p_catalog_trace,
        ),
        (
            "task389p_gemma_preparation_has_one_attempt_local_owner",
            task389p_gemma_preparation_has_one_attempt_local_owner,
        ),
        (
            "task389p_gemma_preparation_is_exact_bounded_and_executable",
            task389p_gemma_preparation_is_exact_bounded_and_executable,
        ),
        (
            "task390p_policy_and_catalog_trace_gemma_audio_preparation",
            task390p_policy_trace && task390p_catalog_trace,
        ),
        (
            "task390p_gemma_audio_has_one_attempt_local_owner",
            task390p_gemma_audio_has_one_attempt_local_owner,
        ),
        (
            "task390p_gemma_audio_is_exact_bounded_and_executable",
            task390p_gemma_audio_is_exact_bounded_and_executable,
        ),
        (
            "task340_policy_and_catalog_trace_the_canonical_clip_vision_owner",
            task_340_policy_trace && task_340_catalog_trace,
        ),
        (
            "task340_clip_vision_has_one_architecture_and_preprocess_owner",
            task_340_clip_vision_has_one_architecture_and_preprocess_owner,
        ),
        (
            "task340_clip_vision_delegates_canonical_mechanics",
            task_340_clip_vision_delegates_canonical_mechanics,
        ),
        (
            "task340_clip_vision_adapter_semantics_are_executable",
            task_340_clip_vision_adapter_semantics_are_executable,
        ),
        (
            "task103_custom_functions_modes_and_kernels_have_one_owner",
            task_103_custom_functions_modes_and_kernels_have_one_owner,
        ),
        (
            "task103_adapters_preserve_canonical_autograd_semantics",
            task_103_adapters_preserve_canonical_autograd_semantics,
        ),
    ]);
    if validation == "VAL-OWNERSHIP-001" && case_prefix.is_none() {
        cases.insert(
            "native_stored_payload_boundary_is_closed",
            validate_native_stored_payload_boundary(&root, &sources)?,
        );
        cases.insert(
            "native_text_regex_has_one_bounded_owner",
            validate_native_text_regex_boundary(&root, &sources)?,
        );
        cases.insert(
            "native_structured_input_links_have_one_checked_boundary",
            validate_native_structured_input_boundary(&root, &sources)?,
        );
        cases.insert(
            "native_shader_execution_has_one_injected_owner",
            validate_native_shader_execution_boundary(&root, &sources)?,
        );
    }
    let cases = cases
        .into_iter()
        .filter(|(name, _)| case_prefix.is_none_or(|prefix| name.starts_with(prefix)))
        .collect::<BTreeMap<_, _>>();
    let failed_cases = cases
        .iter()
        .filter_map(|(name, passed)| (!passed).then_some(name))
        .collect::<Vec<_>>();
    assert!(
        failed_cases.is_empty(),
        "ownership-domain cases failed: {failed_cases:#?}"
    );

    let fixture_paths = [
        "crates/comfy_types/src/cancellation.rs",
        "crates/comfy_types/src/worker_protocol.rs",
        "crates/comfy_worker/src/comfy_worker.rs",
        "crates/comfy_worker/src/memory_modes.rs",
        "crates/comfy_worker/src/memory_planner.rs",
        "crates/comfy_worker/src/supervisor.rs",
        "crates/comfy_worker/tests/memory_conformance.rs",
        "crates/comfy_backend_corex/src/comfy_backend_corex.rs",
        "crates/comfy_backend_cuda/src/comfy_backend_cuda.rs",
        "crates/comfy_backend_directml/src/comfy_backend_directml.rs",
        "crates/comfy_backend_metal/src/comfy_backend_metal.rs",
        "crates/comfy_backend_metal/src/abi.rs",
        "crates/comfy_backend_metal/src/loader.rs",
        "crates/comfy_backend_metal/abi/symbols-v1.json",
        "crates/comfy_backend_metal/abi/reviewed-bindings-v1.txt",
        "crates/comfy_backend_metal/build.rs",
        "crates/comfy_backend_metal/kernels/readiness.metal",
        "crates/comfy_backend_metal/LICENSES",
        "script/package-comfy-backend-metal",
        "nix/comfy-backends/metal/package-policy.json",
        "nix/comfy-backends/metal/ffi-contracts-v1.schema.json",
        "nix/comfy-backends/metal/default.nix",
        "crates/comfy_runtime/src/native_ffi_metal.rs",
        ".agents/specs/comfy-parity/catalogs/native-backend-abi/metal.json",
        "crates/comfy_backend_mlu/src/comfy_backend_mlu.rs",
        "crates/comfy_backend_mlu/src/abi.rs",
        "crates/comfy_backend_mlu/src/loader.rs",
        "crates/comfy_backend_mlu/src/execution.rs",
        "crates/comfy_backend_mlu/abi/symbols-v1.json",
        "crates/comfy_backend_mlu/build.rs",
        "crates/comfy_backend_mlu/LICENSES",
        "script/package-comfy-backend-mlu",
        "nix/comfy-backends/mlu/package-policy.json",
        ".agents/specs/comfy-parity/catalogs/native-backend-abi/mlu.json",
        "crates/comfy_backend_directml/src/comfy_backend_directml.rs",
        "crates/comfy_backend_directml/src/abi.rs",
        "crates/comfy_backend_directml/src/loader.rs",
        "crates/comfy_backend_directml/src/execution.rs",
        "crates/comfy_backend_directml/abi/symbols-v1.json",
        "crates/comfy_backend_directml/build.rs",
        "crates/comfy_backend_directml/LICENSES",
        "crates/gpui_windows/src/directx_devices.rs",
        "crates/gpui_windows/src/directx_renderer.rs",
        "script/package-comfy-backend-directml",
        "nix/comfy-backends/directml/package-policy.json",
        ".agents/specs/comfy-parity/catalogs/native-backend-abi/directml.json",
        "crates/comfy_backend_npu/src/comfy_backend_npu.rs",
        "crates/comfy_backend_npu/src/abi.rs",
        "crates/comfy_backend_npu/src/loader.rs",
        "crates/comfy_backend_npu/src/execution.rs",
        "crates/comfy_backend_npu/abi/symbols-v1.json",
        "crates/comfy_backend_npu/abi/reviewed-bindings-v1.txt",
        "crates/comfy_backend_npu/abi/verify-execution-bindings.sh",
        "crates/comfy_backend_npu/build.rs",
        "crates/comfy_backend_npu/LICENSES",
        "script/package-comfy-backend-npu",
        "nix/comfy-backends/npu/package-policy.json",
        ".agents/specs/comfy-parity/catalogs/native-backend-abi/npu.json",
        "crates/comfy_backend_rocm/src/comfy_backend_rocm.rs",
        "crates/comfy_backend_xpu/src/comfy_backend_xpu.rs",
        "crates/comfy_backend_rocm/src/loader.rs",
        "crates/comfy_backend_rocm/abi/symbols-v1.json",
        "crates/comfy_backend_rocm/abi/reviewed-bindings-v1.txt",
        "crates/comfy_backend_rocm/abi/verify-completion-evidence.sh",
        "crates/comfy_backend_rocm/build.rs",
        "crates/comfy_runtime/src/native_ffi_rocm.rs",
        "script/package-comfy-backend-rocm",
        "nix/comfy-backends/rocm/package-policy.json",
        "crates/comfy_runtime/src/permissions.rs",
        "crates/comfy_runtime/src/trust.rs",
        "crates/comfy_runtime/src/assets.rs",
        "crates/comfy_runtime/src/cache.rs",
        "crates/comfy_runtime/src/execution_presentation.rs",
        "crates/comfy_runtime/src/executor.rs",
        "crates/comfy_runtime/src/native_execution_controller.rs",
        "crates/comfy_runtime/src/output_committer.rs",
        "crates/comfy_runtime/src/persistence.rs",
        "crates/comfy_runtime/src/prompt_compiler.rs",
        "crates/comfy_runtime/src/queue_history.rs",
        "crates/comfy_runtime/src/recovery.rs",
        "crates/comfy_runtime/src/graph.rs",
        "crates/comfy_runtime/src/subgraph_blueprints.rs",
        "crates/comfy_tensor/src/cpu_backend.rs",
        "crates/comfy_tensor/src/operation.rs",
        "crates/comfy_tensor/src/shader.rs",
        "crates/comfy_tensor/src/image_ops.rs",
        "crates/comfy_tensor/src/autograd.rs",
        "crates/comfy_tensor/src/autograd/breadth.rs",
        "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_06.rs",
        "crates/comfy_tensor/tests/autograd_state_consolidation.rs",
        "crates/comfy_model/src/quantization.rs",
        "crates/comfy_model/src/quantized_autograd.rs",
        "crates/comfy_model/tests/quantized_autograd.rs",
        ".agents/specs/comfy-parity/fixtures/quant-linear-source-oracle.json",
        "crates/comfy_test_support/fixtures/autograd/breadth-v1.json",
        "crates/comfy_test_support/tests/autograd_breadth.rs",
        "crates/comfy_tensor/src/ops/external_tensor_kernel_01.rs",
        "crates/comfy_tensor/src/ops/external_tensor_kernel_02.rs",
        "crates/comfy_tensor/src/ops/external_tensor_kernel_03.rs",
        "crates/comfy_tensor/tests/ops/external_tensor_kernel_03.rs",
        "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_10.rs",
        "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_14.rs",
        "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_17.rs",
        "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_21.rs",
        "crates/comfy_tensor/src/operation_resolutions/elementwise_or_runtime_operation_21.rs",
        "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_22.rs",
        "crates/comfy_tensor/src/operation_resolutions/elementwise_or_runtime_operation_22.rs",
        "crates/comfy_tensor/src/ops/elementwise_or_runtime_operation_23.rs",
        "crates/comfy_tensor/src/ops/indexing_masking_01.rs",
        "crates/comfy_tensor/src/ops/indexing_masking_02.rs",
        "crates/comfy_tensor/src/ops/linear_algebra_01.rs",
        "crates/comfy_tensor/src/ops/linear_algebra_02.rs",
        "crates/comfy_tensor/src/ops/storage_dtype_device_01.rs",
        "crates/comfy_tensor/src/operation_resolutions/storage_dtype_device_01.rs",
        "crates/comfy_tensor/tests/ops/storage_dtype_device_01.rs",
        "crates/comfy_tensor/tests/ops/linear_algebra_01.rs",
        "crates/comfy_tensor/tests/ops/linear_algebra_02.rs",
        "crates/comfy_tensor/src/ops/neural_network_functional_01.rs",
        "crates/comfy_tensor/tests/ops/neural_network_functional_01.rs",
        "crates/comfy_tensor/src/ops/neural_network_module_01.rs",
        "crates/comfy_tensor/tests/ops/neural_network_module_01.rs",
        "crates/comfy_tensor/src/ops/neural_network_module_04.rs",
        "crates/comfy_tensor/src/operation_resolutions/neural_network_module_04.rs",
        "crates/comfy_tensor/tests/ops/neural_network_module_04.rs",
        "crates/comfy_tensor/tests/ops/elementwise_or_runtime_operation_21.rs",
        "crates/comfy_tensor/tests/ops/elementwise_or_runtime_operation_22.rs",
        "crates/comfy_model/src/native_ops.rs",
        "crates/comfy_model/src/vision_models.rs",
        "crates/comfy_runtime/src/runtime_supervisor.rs",
        "crates/comfy_runtime/src/plugin_services.rs",
        "crates/comfy_plugin_sdk/src/comfy_plugin_sdk.rs",
        "crates/comfy_plugin_host/src/capabilities.rs",
        "crates/comfy_plugin_host/src/comfy_plugin_host.rs",
        "crates/comfy_plugin_host/src/component_host.rs",
        "crates/comfy_plugin_host/src/private_worker.rs",
        "crates/comfy_worker/src/plugin_runtime.rs",
        "crates/extension_host/src/extension_host.rs",
        "crates/comfy_nodes/Cargo.toml",
        "crates/comfy_nodes/src/text_regex.rs",
        "crates/comfy_backend_metal/src/execution.rs",
        "crates/comfy_backend_metal/src/execution_abi.rs",
        "crates/comfy_backend_metal/abi/execution-v1.json",
        "crates/comfy_backend_metal/abi/reviewed-execution-bindings-v1.txt",
        "crates/comfy_backend_metal/kernels/tensor_ops.metal",
        "nix/comfy-backends/metal/execution-policy.json",
        "script/package-comfy-backend-metal-execution",
        ".agents/specs/comfy-parity/catalogs/native-backend-abi/metal-execution.json",
        "crates/comfy_model/src/artifact_index.rs",
        "crates/comfy_model/src/clip.rs",
        "crates/comfy_model/src/clip_text_encoder_t5.rs",
        "crates/comfy_model/src/clip_vision.rs",
        "crates/comfy_model/src/clip_tokenizer.rs",
        "crates/comfy_model/src/formats.rs",
        "crates/comfy_model/src/model_store.rs",
        "crates/comfy_model/src/slices/native_diffusion.rs",
        "crates/comfy_runtime/src/native_execution_controller.rs",
        "crates/comfy_test_support/src/native_diffusion_fixture.rs",
        "crates/comfy_model/tests/clip_vision.rs",
        "crates/comfy_model/tests/clip_text_encoder_t5.rs",
        "crates/comfy_model/tests/clip_tokenizer.rs",
        "crates/comfy_api/src/http.rs",
        "crates/comfy_api/src/services.rs",
        "crates/comfy_api/src/comfy_api.rs",
        "crates/comfy_api/src/headless.rs",
        "crates/comfy_ui/src/execution_model.rs",
        "crates/comfy_ui/src/execution_panel.rs",
        "crates/comfy_ui/src/comfy_ui.rs",
        "crates/comfy_ui/src/context_menu.rs",
        "crates/comfy_ui/src/context_menu_tests.rs",
        "crates/comfy_ui/src/workflow_item.rs",
        "crates/comfy_test_support/tests/native_image_recovery.rs",
        "crates/comfy_api/src/security.rs",
        "crates/comfy_api/src/transport.rs",
        "crates/sim/src/sim.rs",
        "crates/sim/src/comfy_plugin_services.rs",
        ".agents/specs/comfy-parity/ownership-policy.json",
        ".agents/specs/comfy-parity/catalogs/authoritative-ownership.csv",
        ".agents/specs/comfy-parity/generate_ownership_catalog.py",
        ".agents/specs/comfy-parity/regenerate_native_planning.py",
        "crates/comfy_test_support/tests/ownership_consolidation.rs",
    ];
    let fixture_digests = fixture_paths
        .into_iter()
        .map(|relative| Ok((relative.to_owned(), file_sha256(&root.join(relative))?)))
        .collect::<Result<BTreeMap<_, _>, Box<dyn std::error::Error>>>()?;
    let artifact = json!({
        "validation": validation,
        "scope": scope,
        "environment": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "backend": "native-rust",
            "network_requests": 0,
            "external_processes": 0,
        },
        "execution": {
            "command": format!(
                "cargo test -p comfy_test_support {exact_test_name} -- --exact --nocapture"
            ),
            "exact_test_name": exact_test_name,
            "executed_tests": 1,
        },
        "fixture_digests": fixture_digests,
        "definition_counts": {
            "backend_capability_matrix": backend_matrix_definitions.len(),
            "backend_readiness": backend_readiness_definitions.len(),
            "backend_binding": backend_binding_definitions.len(),
            "cancellation_token": cancellation_definitions.len(),
            "permission_policy": permission_policy_definitions.len(),
            "plugin_trust_policy": plugin_trust_definitions.len(),
            "provider_policy": provider_policy_definitions.len(),
            "external_navigation_policy": external_navigation_policy_definitions.len(),
            "execution_queue": execution_queue_definitions.len(),
            "execution_presentation_owner": execution_owner_definitions.len(),
            "native_queue": native_queue_definitions.len(),
            "artifact_root": artifact_root_definitions.len(),
            "artifact_root_recursive_enumeration": artifact_root_recursive_enumeration_definitions.len(),
            "native_package_capture": native_package_capture_definitions.len(),
            "native_package_coverage": native_package_coverage_definitions.len(),
            "artifact_index": artifact_index_definitions.len(),
            "asset_service": asset_service_definitions.len(),
            "model_store": model_store_definitions.len(),
            "prompt_compiler": prompt_compiler_definitions.len(),
            "native_cache": native_cache_definitions.len(),
            "cache_key": cache_key_definitions.len(),
            "native_clip_vision": native_clip_vision_definitions.len(),
            "clip_preprocess": clip_preprocess_definitions.len(),
            "siglip2_preprocess": siglip2_preprocess_definitions.len(),
            "siglip2_flex_resolution": siglip2_flex_resolution_definitions.len(),
            "checkpoint_record": checkpoint_record_definitions.len(),
            "checkpoint_execution": checkpoint_execution_definitions.len(),
            "higher_order_context": higher_order_context_definitions.len(),
            "recovery_journal": recovery_journal_definitions.len(),
            "recovery_output_receipt": recovery_output_receipt_definitions.len(),
            "output_commit_receipt": output_commit_receipt_definitions.len(),
            "plugin_capability_broker": plugin_capability_broker_definitions.len(),
            "output_committer": output_committer_definitions.len(),
            "comfy_runtime_db": runtime_database_definitions.len(),
            "graph_context_action_binding": graph_context_binding_definitions.len(),
            "graph_context_dispatch": graph_context_dispatch_definitions.len(),
            "attempt_memory_controller": attempt_memory_controller_definitions.len(),
            "memory_planner": memory_planner_definitions.len(),
            "scratch_reservation": scratch_reservation_definitions.len(),
            "unpaired_cpu_backend_constructors": unpaired_cpu_backend_constructors.len(),
            "backend_workspace_authority": backend_workspace_authority_definitions.len(),
            "cpu_workspace_authority_alias": cpu_workspace_authority_aliases.len(),
            "backend_workspace_lease": backend_workspace_lease_definitions.len(),
            "cpu_workspace_vector": cpu_workspace_vector_definitions.len(),
            "backend_memory_tracker": backend_memory_tracker_definitions.len(),
            "normative_val_memory_artifact_writers": normative_memory_artifact_writer_sites.len(),
            "normative_val_vae_artifact_writers": normative_vae_artifact_writer_sites.len(),
            "component_lifecycle_adapter_production_impls": component_lifecycle_adapter_impls.len(),
            "subgraph_blueprint_library": subgraph_library_definitions.len(),
            "subgraph_blueprint_catalog": subgraph_catalog_definitions.len(),
            "subgraph_blueprint_catalog_production_type_sites": subgraph_catalog_type_sites.len(),
            "public_asset_index_escapes": public_asset_index_escapes.len(),
            "plugin_root_mapping_definitions": plugin_root_mapping_definitions.len(),
            "plugin_root_mapping_calls": plugin_root_mapping_calls.len(),
            "public_per_attempt_persistence_apis": public_per_attempt_persistence_apis.len(),
            "execution_owner_deref_impls": execution_owner_deref_impls.len(),
        },
        "call_path_inventory": {
            "execution_state_transition_calls": private_reducer_call_sites,
            "graph_context_dispatch_sites": graph_context_dispatch_sites,
            "component_lifecycle_adapter_impls": component_lifecycle_adapter_impls,
            "normative_val_memory_artifact_writer_sites": normative_memory_artifact_writer_sites,
            "normative_val_vae_artifact_writer_sites": normative_vae_artifact_writer_sites,
            "subgraph_catalog_refresh_sites": subgraph_catalog_refresh_sites,
            "subgraph_catalog_owner_mutation_sites": subgraph_catalog_owner_mutation_sites,
            "subgraph_catalog_entry_consumers": subgraph_catalog_entry_consumers,
            "subgraph_catalog_production_type_sites": subgraph_catalog_type_sites,
            "subgraph_byte_preflight_position": subgraph_byte_preflight_position,
            "subgraph_asset_lock_position": subgraph_asset_lock_position,
            "subgraph_transaction_is_ordered": subgraph_transaction_is_ordered,
            "blueprint_decode_bound_position": blueprint_decode_bound_position,
            "blueprint_decode_parse_position": blueprint_decode_parse_position,
            "blueprint_decode_is_bounded_before_parse": blueprint_decode_is_bounded_before_parse,
            "subgraph_accounting_position": subgraph_accounting_position,
            "subgraph_read_position": subgraph_read_position,
            "subgraph_accounting_precedes_reads": subgraph_accounting_precedes_reads,
            "subgraph_projection_await_position": subgraph_projection_await_position,
            "subgraph_projection_replace_position": subgraph_projection_replace_position,
            "subgraph_projection_send_position": subgraph_projection_send_position,
            "subgraph_projection_is_detached_and_ordered": subgraph_projection_is_detached_and_ordered,
            "asset_service_exact_write_sites": production_source_occurrences(
                &sources,
                ".write_exact(",
            ),
            "asset_service_authorized_list_sites": production_source_occurrences(
                &sources,
                ".list_authorized(",
            ),
            "full_profile_persistence_replace_sites": production_source_occurrences(
                &sources,
                "replace_execution_profile(",
            ),
            "actuator_transaction_sites": production_source_occurrences(
                &sources,
                "apply_actuator_event_transaction_durable(",
            ),
            "native_package_capture_sites": native_package_capture_sites,
            "native_package_coverage_sites": native_package_coverage_sites,
        },
        "progressive_ownership": {
            "accounted_pending_rows": accounted_pending_ownership_rows?,
        },
        "cases": cases,
        "summary": {
            "executed_tests": 1,
            "passed": cases.len(),
            "failed": 0,
            "skipped": 0,
        },
    });
    let mut bytes = serde_json::to_vec_pretty(&artifact)?;
    bytes.push(b'\n');
    let artifact_path = validation_target_directory(&root)
        .join("comfy-parity")
        .join(artifact_filename);
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(artifact_path, bytes)?;
    Ok(())
}

#[test]
fn val_ownership_domain_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-DOMAIN-001",
        "authorization-cancellation-backend-ownership",
        "val-ownership-domain-001.json",
        "val_ownership_domain_001",
        None,
    )
}

#[test]
fn val_ownership_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "authoritative-service-adapter-and-foundational-type-ownership",
        "val-ownership-001.json",
        "val_ownership_001",
        None,
    )
}

#[test]
fn val_ownership_task102_quantization_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task102-whole-repository-quantization-identity-materialization-and-adapter-ownership",
        "val-ownership-task102-quantization-001.json",
        "val_ownership_task102_quantization_001",
        Some("task102_"),
    )
}

#[test]
fn val_ownership_task511_patch_adapters_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task511-weight-adapter-patch-graph-and-quantization-ownership",
        "val-ownership-task511-patch-adapters-001.json",
        "val_ownership_task511_patch_adapters_001",
        Some("task511_"),
    )
}

#[test]
fn val_ownership_task512_tokenizer_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task512-sd1-prompt-sentencepiece-artifact-and-runtime-cache-ownership",
        "val-ownership-task512-tokenizer-001.json",
        "val_ownership_task512_tokenizer_001",
        Some("task512_"),
    )
}

#[test]
fn val_ownership_task340_clip_vision_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task340-clip-siglip-siglip2-visual-architecture-and-adapter-ownership",
        "val-ownership-task340-clip-vision-001.json",
        "val_ownership_task340_clip_vision_001",
        Some("task340_"),
    )
}

#[test]
fn val_ownership_task339_clip_text_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task339-clip-text-transformer-and-sd1-adapter-ownership",
        "val-ownership-task339-clip-text-001.json",
        "val_ownership_task339_clip_text_001",
        Some("task339_"),
    )
}

#[test]
fn val_ownership_task342_t5_bidirectional_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task342-t5-bert-sentencepiece-bidirectional-architecture-ownership",
        "val-ownership-task342-t5-bidirectional-001.json",
        "val_ownership_task342_t5_bidirectional_001",
        Some("task342_"),
    )
}

#[test]
fn val_ownership_task343_decoder_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task343-llama-gemma-gpt-oss-qwen35-decoder-and-transient-cache-ownership",
        "val-ownership-task343-decoder-001.json",
        "val_ownership_task343_decoder_001",
        Some("task343_"),
    )
}

#[test]
fn val_ownership_task380_prepared_decoder_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task380-prepared-decoder-prefill-and-shared-generation-ownership",
        "val-ownership-task380-prepared-decoder-001.json",
        "val_ownership_task380_prepared_decoder_001",
        Some("task380_"),
    )
}

#[test]
fn val_ownership_task382_prepared_deepstack_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task382-prepared-decoder-deepstack-and-shared-generation-ownership",
        "val-ownership-task382-prepared-deepstack-001.json",
        "val_ownership_task382_prepared_deepstack_001",
        Some("task382_"),
    )
}

#[test]
fn val_ownership_task383_qwen2_tokenizer_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task383-qwen2-byte-bpe-and-native-prompt-tokenizer-ownership",
        "val-ownership-task383-qwen2-tokenizer-001.json",
        "val_ownership_task383_qwen2_tokenizer_001",
        Some("task383_"),
    )
}

#[test]
fn val_ownership_task391_gemma_tokenizer_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task391-gemma-artifact-tokenization-and-native-prompt-tokenizer-ownership",
        "val-ownership-task391-gemma-tokenizer-001.json",
        "val_ownership_task391_gemma_tokenizer_001",
        Some("task391_"),
    )
}

#[test]
fn val_ownership_task392_gemma3_decoder_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task392-gemma3-alternating-rope-and-canonical-decoder-ownership",
        "val-ownership-task392-gemma3-decoder-001.json",
        "val_ownership_task392_gemma3_decoder_001",
        Some("task392_"),
    )
}

#[test]
fn val_ownership_task393_gemma4_decoder_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task393-gemma4-global-shared-per-layer-and-canonical-decoder-ownership",
        "val-ownership-task393-gemma4-decoder-001.json",
        "val_ownership_task393_gemma4_decoder_001",
        Some("task393_"),
    )
}

#[test]
fn val_ownership_task394_gemma3_vision_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task394-gemma3-retained-vision-projection-ownership",
        "val-ownership-task394-gemma3-vision-001.json",
        "val_ownership_task394_gemma3_vision_001",
        Some("task394_"),
    )
}

#[test]
fn val_ownership_task395_gemma4_vision_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task395-gemma4-retained-vision-projection-ownership",
        "val-ownership-task395-gemma4-vision-001.json",
        "val_ownership_task395_gemma4_vision_001",
        Some("task395_"),
    )
}

#[test]
fn val_ownership_task396_gemma4_audio_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task396-gemma4-retained-audio-execution-ownership",
        "val-ownership-task396-gemma4-audio-001.json",
        "val_ownership_task396_gemma4_audio_001",
        Some("task396_"),
    )
}

#[test]
fn val_ownership_task384_qwen3_decoder_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task384-qwen3-query-key-norm-and-canonical-decoder-ownership",
        "val-ownership-task384-qwen3-decoder-001.json",
        "val_ownership_task384_qwen3_decoder_001",
        Some("task384_"),
    )
}

#[test]
fn val_ownership_task385_qwen35_decoder_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task385-qwen35-hybrid-attention-and-canonical-decoder-ownership",
        "val-ownership-task385-qwen35-decoder-001.json",
        "val_ownership_task385_qwen35_decoder_001",
        Some("task385_"),
    )
}

#[test]
fn val_ownership_task386_qwen_vision_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task386-qwen-vision-and-retained-projection-ownership",
        "val-ownership-task386-qwen-vision-001.json",
        "val_ownership_task386_qwen_vision_001",
        Some("task386_"),
    )
}

#[test]
fn val_ownership_task387_qwen_multimodal_resource_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task387-qwen-multimodal-clip-resource-ownership",
        "val-ownership-task387-qwen-multimodal-resource-001.json",
        "val_ownership_task387_qwen_multimodal_resource_001",
        Some("task387_"),
    )
}

#[test]
fn val_ownership_task397_gemma_multimodal_resource_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task397-gemma-multimodal-clip-resource-ownership",
        "val-ownership-task397-gemma-multimodal-resource-001.json",
        "val_ownership_task397_gemma_multimodal_resource_001",
        Some("task397_"),
    )
}

#[test]
fn val_ownership_task388_qwen_multimodal_generation_001() -> Result<(), Box<dyn std::error::Error>>
{
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task388-qwen-multimodal-generation-ownership",
        "val-ownership-task388-qwen-multimodal-generation-001.json",
        "val_ownership_task388_qwen_multimodal_generation_001",
        Some("task388_"),
    )
}

#[test]
fn val_ownership_task381p_qwen_preparation_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task381p-qwen-image-preparation-and-attempt-local-state-ownership",
        "val-ownership-task381p-qwen-preparation-001.json",
        "val_ownership_task381p_qwen_preparation_001",
        Some("task381p_"),
    )
}

#[test]
fn val_ownership_task389p_gemma_preparation_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task389p-gemma-image-video-preparation-and-attempt-local-state-ownership",
        "val-ownership-task389p-gemma-preparation-001.json",
        "val_ownership_task389p_gemma_preparation_001",
        Some("task389p_"),
    )
}

#[test]
fn val_ownership_task390p_gemma_audio_preparation_001() -> Result<(), Box<dyn std::error::Error>> {
    run_ownership_validation(
        "VAL-OWNERSHIP-001",
        "task390p-gemma-audio-preparation-and-attempt-local-state-ownership",
        "val-ownership-task390p-gemma-audio-preparation-001.json",
        "val_ownership_task390p_gemma_audio_preparation_001",
        Some("task390p_"),
    )
}

#[test]
fn progressive_ownership_contract_rejects_unaccounted_or_falsely_completed_rows() {
    assert_eq!(
        task_completion_statuses(concat!(
            "- [x] 1. Complete\n  - _id: complete-task\n",
            "- [X] 1a. Complete uppercase\n  - _id: complete-uppercase-task\n",
            "- [-] 2. In progress\n  - _id: in-progress-task\n",
        )),
        Ok(BTreeMap::from([
            ("complete-task".to_owned(), true),
            ("complete-uppercase-task".to_owned(), true),
            ("in-progress-task".to_owned(), false),
        ]))
    );
    let catalog = concat!(
        "concern,notes,current_status\n",
        "confirmed_owner,ordinary,authoritative_owner_confirmed\n",
        "\"open,owner\",\"mentions authoritative_owner_confirmed, but remains open\",consolidation_required[known_integration_gap]\n",
        "second_open_owner,\"quoted\nmultiline note with an escaped \"\"quote\"\"\",consolidation_required[second_gap]\n",
    );
    let policy = vec![
        json!({
            "concern": "open,owner",
            "consolidation_tasks": ["pending-task"],
        }),
        json!({
            "concern": "second_open_owner",
            "consolidation_tasks": ["second-pending-task"],
        }),
    ];
    let pending = BTreeMap::from([
        ("pending-task".to_owned(), false),
        ("second-pending-task".to_owned(), false),
    ]);
    assert_eq!(
        accounted_pending_ownership_rows(catalog, &policy, &pending),
        Ok(BTreeMap::from([
            ("open,owner".to_owned(), vec!["pending-task".to_owned()],),
            (
                "second_open_owner".to_owned(),
                vec!["second-pending-task".to_owned()],
            ),
        ]))
    );

    let completed = BTreeMap::from([
        ("pending-task".to_owned(), true),
        ("second-pending-task".to_owned(), true),
    ]);
    assert!(accounted_pending_ownership_rows(catalog, &policy, &completed).is_err());
    assert!(accounted_pending_ownership_rows(catalog, &[], &pending).is_err());
    assert!(accounted_pending_ownership_rows(catalog, &policy, &BTreeMap::new()).is_err());

    assert!(task_completion_statuses("  - _id: orphan-task\n").is_err());
    assert!(
        task_completion_statuses(concat!(
            "- [ ] 1. First\n  - _id: duplicate-task\n",
            "- [x] 2. Second\n  - _id: duplicate-task\n",
        ))
        .is_err()
    );
    assert!(
        task_completion_statuses(concat!(
            "- [ ] 1. First\n  - _id: first-task\n",
            "  - _id: late-task\n",
        ))
        .is_err()
    );
    assert!(task_completion_statuses("- [ ] 1. Missing ID\n").is_err());

    for malformed_catalog in [
        "concern,concern,current_status\nowner,owner,open\n",
        "concern,notes\nowner,open\n",
        "concern,current_status\nowner\n",
        "concern,current_status\nowner,open\nowner,open\n",
        "concern,current_status\n\"unterminated,open\n",
    ] {
        assert!(
            accounted_pending_ownership_rows(malformed_catalog, &policy, &pending).is_err(),
            "malformed catalog unexpectedly passed: {malformed_catalog:?}"
        );
    }
}

#[test]
fn mlu_package_trust_adapter_preserves_authoritative_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let trust = fs::read_to_string(root.join("crates/comfy_runtime/src/trust.rs"))?;
    let runtime = fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_mlu.rs"))?;
    let loader = fs::read_to_string(root.join("crates/comfy_backend_mlu/src/loader.rs"))?;
    let packager = fs::read_to_string(root.join("script/package-comfy-backend-mlu"))?;
    let settings = fs::read_to_string(root.join("crates/comfy_runtime/src/settings.rs"))?;

    assert_eq!(
        production_source_occurrences(&sources, "pub struct NativeFfiRegistry").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct MluPackageVerificationKey").len(),
        1
    );
    assert!(trust.contains("struct NativePackageVerificationAuthority"));
    assert!(runtime.contains("registry: NativeFfiRegistry"));
    assert_eq!(
        runtime
            .split_once("#[cfg(test)]\nmod tests")
            .map_or(runtime.as_str(), |(production, _)| production)
            .matches("NativeFfiRegistry::new")
            .count(),
        1
    );
    assert!(runtime.contains("MluExecutionRuntime::load_certified"));
    assert!(settings.contains("NativeMluPackageSettings::from_public_authority"));
    assert!(!loader.contains("MluPackageVerificationKey"));
    assert!(!loader.contains("NativeFfiRegistry::"));
    assert!(!packager.contains("NativeFfiRegistry::new"));
    assert!(!packager.contains("verify_package("));
    Ok(())
}

#[test]
fn directml_package_trust_adapter_preserves_authoritative_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let trust = fs::read_to_string(root.join("crates/comfy_runtime/src/trust.rs"))?;
    let runtime = fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_directml.rs"))?;
    let loader = fs::read_to_string(root.join("crates/comfy_backend_directml/src/loader.rs"))?;
    let packager = fs::read_to_string(root.join("script/package-comfy-backend-directml"))?;
    let settings = fs::read_to_string(root.join("crates/comfy_runtime/src/settings.rs"))?;

    assert_eq!(
        production_source_occurrences(&sources, "pub struct NativeFfiRegistry").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct DirectMlPackageVerificationKey").len(),
        1
    );
    assert!(trust.contains("struct NativePackageVerificationAuthority"));
    assert_eq!(
        trust
            .matches("const DIRECTML_PACKAGE_SIGNATURE_DOMAIN")
            .count(),
        1
    );
    assert!(runtime.contains("capture_native_package"));
    assert!(runtime.contains("validate_native_package_coverage"));
    let verification = runtime
        .find("verification_key.verify_package")
        .ok_or("DirectML adapter omitted package verification")?;
    let mapping = runtime
        .find("let catalog: DirectMlFfiContractCatalogDto")
        .ok_or("DirectML adapter omitted strict catalog mapping")?;
    assert!(verification < mapping);
    assert_eq!(
        runtime
            .split_once("#[cfg(test)]\nmod tests")
            .map_or(runtime.as_str(), |(production, _)| production)
            .matches("NativeFfiRegistry::new")
            .count(),
        1
    );
    assert!(runtime.contains("RetainedDirectMlLibraryHandles"));
    let initializer = runtime
        .split_once("pub fn initialize_certified_directml_runtime")
        .map(|(_, initializer)| initializer)
        .ok_or("DirectML runtime initializer is missing")?;
    let package_verification = initializer
        .find("verify_directml_package_contracts")
        .ok_or("DirectML initializer omitted package verification")?;
    let host_observation = initializer
        .find("observe_directml_candidate")
        .ok_or("DirectML initializer omitted canonical host observation")?;
    let image_certification = initializer
        .find("certify_directml_library_images")
        .ok_or("DirectML initializer omitted registry image certification")?;
    assert!(package_verification < host_observation && host_observation < image_certification);
    assert!(loader.contains("pub fn for_current_system"));
    assert!(loader.contains("GetSystemDirectoryW"));
    assert!(loader.contains("RtlGetVersion"));
    assert!(loader.contains("GetFileVersionInfoW"));
    assert!(loader.contains("WinVerifyTrust"));
    assert!(loader.contains("WTD_CACHE_ONLY_URL_RETRIEVAL"));
    assert!(loader.contains("WTD_REVOKE_NONE"));
    assert!(!runtime.contains("authenticode_trusted: true"));
    assert!(!loader.contains("pub authenticode_trusted"));
    assert!(settings.contains("NativeDirectMlPackageSettings::from_public_authority"));
    assert!(!settings.contains("DirectMlPackageVerificationKey::verify_package"));
    assert!(!settings.contains("NativeFfiRegistry::new"));
    assert!(!loader.contains("DirectMlPackageVerificationKey"));
    assert!(!loader.contains("NativeFfiRegistry::"));
    assert!(!packager.contains("NativeFfiRegistry::new"));
    assert!(!packager.contains("verify_package("));
    Ok(())
}

#[test]
fn mlu_worker_selection_adapter_preserves_authoritative_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let runtime_supervisor =
        fs::read_to_string(root.join("crates/comfy_runtime/src/runtime_supervisor.rs"))?;
    let worker = fs::read_to_string(root.join("crates/comfy_worker/src/comfy_worker.rs"))?;
    let worker_session = fs::read_to_string(root.join("crates/comfy_worker/src/supervisor.rs"))?;
    let tensor = fs::read_to_string(
        root.join("crates/comfy_tensor/src/backends/cambricon_mlu_comfy_model_0017.rs"),
    )?;
    let runtime = fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_mlu.rs"))?;

    assert_eq!(
        production_source_occurrences(&sources, "pub enum WorkerBackendSelection").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct WorkerBackendSession").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct MluTensorBackend").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct CancellationToken").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct BackendWorkspaceAuthority").len(),
        1
    );
    assert!(runtime_supervisor.contains("WorkerBackendSelection::Mlu"));
    assert!(runtime_supervisor.contains("Self::for_mlu"));
    let verification = worker
        .find("initialize_certified_mlu_runtime")
        .ok_or("worker omitted MLU package verification")?;
    let tensor_construction = worker
        .find("MluTensorBackend::from_certified_runtime")
        .ok_or("worker omitted MLU tensor construction")?;
    let session_construction = worker[verification..]
        .find("WorkerBackendSession::new")
        .map(|position| verification + position)
        .ok_or("worker omitted canonical session construction")?;
    assert!(verification < tensor_construction && tensor_construction < session_construction);
    assert!(worker_session.contains("BinaryOperation::Add"));
    assert!(worker_session.contains("backend readiness probe retained device allocations"));
    assert!(!worker.contains("WorkerBackendSelection::Mlu => WorkerBackendSelection::Cpu"));
    assert!(!worker.contains("NativeFfiRegistry::"));
    assert!(!tensor.contains("NativeFfiRegistry::"));
    assert!(runtime.contains("NativeFfiRegistry::new"));
    assert!(runtime.contains(".required_symbols_for("));
    assert!(runtime.contains("verified.registry().authorize("));
    Ok(())
}

#[test]
fn npu_worker_selection_adapter_preserves_authoritative_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let runtime_supervisor =
        fs::read_to_string(root.join("crates/comfy_runtime/src/runtime_supervisor.rs"))?;
    let worker = fs::read_to_string(root.join("crates/comfy_worker/src/comfy_worker.rs"))?;
    let worker_session = fs::read_to_string(root.join("crates/comfy_worker/src/supervisor.rs"))?;
    let tensor = fs::read_to_string(
        root.join("crates/comfy_tensor/src/backends/huawei_ascend_npu_comfy_model_0019.rs"),
    )?;
    let runtime = fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_npu.rs"))?;

    assert_eq!(
        production_source_occurrences(&sources, "pub enum WorkerBackendSelection").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct WorkerBackendSession").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct NpuTensorBackend").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct CancellationToken").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct BackendWorkspaceAuthority").len(),
        1
    );
    assert!(runtime_supervisor.contains("WorkerBackendSelection::Npu"));
    assert!(runtime_supervisor.contains("Self::for_npu"));
    let verification = worker
        .find("initialize_certified_npu_runtime")
        .ok_or("worker omitted NPU package verification")?;
    let tensor_construction = worker
        .find("NpuTensorBackend::from_certified_runtime")
        .ok_or("worker omitted NPU tensor construction")?;
    let session_construction = worker[verification..]
        .find("WorkerBackendSession::new")
        .map(|position| verification + position)
        .ok_or("worker omitted canonical session construction")?;
    assert!(verification < tensor_construction && tensor_construction < session_construction);
    assert!(worker_session.contains("BinaryOperation::Add"));
    assert!(worker_session.contains("backend readiness probe retained device allocations"));
    assert!(!worker.contains("WorkerBackendSelection::Npu => WorkerBackendSelection::Cpu"));
    assert!(!worker.contains("NativeFfiRegistry::"));
    assert!(!tensor.contains("NativeFfiRegistry::"));
    assert!(runtime.contains("NativeFfiRegistry::new"));
    assert!(runtime.contains(".required_symbols_for("));
    assert!(runtime.contains("verified.registry().authorize("));
    assert!(runtime.contains("verified.registry().authorize_dependency("));
    assert!(runtime.contains("load_execution_runtime(device_ordinal)"));
    assert!(runtime.contains("from_registry_certified_images(images, device_ordinal)"));
    assert!(!runtime.contains("from_registry_certified_images(images, 0)"));
    Ok(())
}

#[test]
fn cuda_worker_selection_adapter_preserves_authoritative_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let runtime_supervisor =
        fs::read_to_string(root.join("crates/comfy_runtime/src/runtime_supervisor.rs"))?;
    let worker = fs::read_to_string(root.join("crates/comfy_worker/src/comfy_worker.rs"))?;
    let worker_session = fs::read_to_string(root.join("crates/comfy_worker/src/supervisor.rs"))?;
    let tensor = fs::read_to_string(
        root.join("crates/comfy_tensor/src/backends/nvidia_cuda_comfy_model_0022.rs"),
    )?;
    let runtime = fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_cuda.rs"))?;

    assert_eq!(
        production_source_occurrences(&sources, "pub enum WorkerBackendSelection").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct WorkerBackendSession").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct CudaTensorBackend").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct CancellationToken").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct BackendWorkspaceAuthority").len(),
        1
    );
    assert!(runtime_supervisor.contains("WorkerBackendSelection::Cuda"));
    assert!(runtime_supervisor.contains("Self::for_cuda"));
    let verification = worker
        .find("initialize_certified_cuda_runtime")
        .ok_or("worker omitted CUDA package verification")?;
    let tensor_construction = worker
        .find("CudaTensorBackend::from_certified_session")
        .ok_or("worker omitted CUDA tensor construction")?;
    let session_construction = worker[verification..]
        .find("WorkerBackendSession::new")
        .map(|position| verification + position)
        .ok_or("worker omitted canonical session construction")?;
    assert!(verification < tensor_construction && tensor_construction < session_construction);
    assert!(worker_session.contains("BinaryOperation::Add"));
    assert!(worker_session.contains("backend readiness probe retained device allocations"));
    assert!(!worker.contains("WorkerBackendSelection::Cuda => WorkerBackendSelection::Cpu"));
    assert!(!worker.contains("NativeFfiRegistry::"));
    assert!(!tensor.contains("NativeFfiRegistry::"));
    assert!(runtime.contains("NativeFfiRegistry::new"));
    assert!(runtime.contains(".required_symbols_for("));
    assert!(runtime.contains("verified.registry().authorize("));
    assert!(runtime.contains("load_execution_runtime(device_ordinal)"));
    assert!(runtime.contains("from_registry_certified_images(images, device_ordinal)"));
    assert!(!runtime.contains("from_registry_certified_images(images, 0)"));
    Ok(())
}

#[test]
fn xpu_worker_selection_adapter_preserves_authoritative_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let runtime_supervisor =
        fs::read_to_string(root.join("crates/comfy_runtime/src/runtime_supervisor.rs"))?;
    let worker = fs::read_to_string(root.join("crates/comfy_worker/src/comfy_worker.rs"))?;
    let worker_session = fs::read_to_string(root.join("crates/comfy_worker/src/supervisor.rs"))?;
    let tensor = fs::read_to_string(
        root.join("crates/comfy_tensor/src/backends/intel_xpu_comfy_model_0021.rs"),
    )?;
    let runtime = fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_xpu.rs"))?;

    assert_eq!(
        production_source_occurrences(&sources, "pub enum WorkerBackendSelection").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct WorkerBackendSession").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct XpuTensorBackend").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct CancellationToken").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct BackendWorkspaceAuthority").len(),
        1
    );
    assert!(runtime_supervisor.contains("WorkerBackendSelection::Xpu"));
    assert!(runtime_supervisor.contains("Self::for_xpu"));
    let verification = worker
        .find("initialize_certified_xpu_runtime")
        .ok_or("worker omitted XPU package verification")?;
    let tensor_construction = worker
        .find("XpuTensorBackend::from_certified_session")
        .ok_or("worker omitted XPU tensor construction")?;
    let session_construction = worker[verification..]
        .find("WorkerBackendSession::new")
        .map(|position| verification + position)
        .ok_or("worker omitted canonical session construction")?;
    assert!(verification < tensor_construction && tensor_construction < session_construction);
    assert!(worker_session.contains("BinaryOperation::Add"));
    assert!(worker_session.contains("backend readiness probe retained device allocations"));
    assert!(!worker.contains("WorkerBackendSelection::Xpu => WorkerBackendSelection::Cpu"));
    assert!(!worker.contains("NativeFfiRegistry::"));
    assert!(!tensor.contains("NativeFfiRegistry::"));
    assert!(runtime.contains("NativeFfiRegistry::new"));
    assert!(runtime.contains(".required_symbols_for("));
    assert!(runtime.contains("verified.registry().authorize("));
    assert!(runtime.contains("load_execution_runtime(device_ordinal)"));
    assert!(runtime.contains("from_registry_certified_images(images, device_ordinal)"));
    assert!(!runtime.contains("from_registry_certified_images(images, 0)"));
    Ok(())
}

#[test]
fn directml_worker_selection_adapter_preserves_authoritative_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let runtime_supervisor =
        fs::read_to_string(root.join("crates/comfy_runtime/src/runtime_supervisor.rs"))?;
    let worker = fs::read_to_string(root.join("crates/comfy_worker/src/comfy_worker.rs"))?;
    let worker_session = fs::read_to_string(root.join("crates/comfy_worker/src/supervisor.rs"))?;
    let tensor = fs::read_to_string(
        root.join("crates/comfy_tensor/src/backends/directml_comfy_model_0018.rs"),
    )?;
    let runtime = fs::read_to_string(root.join("crates/comfy_runtime/src/native_ffi_directml.rs"))?;
    let loader = fs::read_to_string(root.join("crates/comfy_backend_directml/src/loader.rs"))?;

    assert_eq!(
        production_source_occurrences(&sources, "pub enum WorkerBackendSelection").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct WorkerBackendSession").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct DirectMlTensorBackend").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct CancellationToken").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct BackendWorkspaceAuthority").len(),
        1
    );
    assert!(runtime_supervisor.contains("WorkerBackendSelection::DirectMl"));
    assert!(runtime_supervisor.contains("Self::for_directml"));
    let verification = worker
        .find("initialize_certified_directml_runtime")
        .ok_or("worker omitted DirectML package and host verification")?;
    let tensor_construction = worker
        .find("DirectMlTensorBackend::from_certified_session")
        .ok_or("worker omitted DirectML tensor construction")?;
    let session_construction = worker[verification..]
        .find("WorkerBackendSession::new")
        .map(|position| verification + position)
        .ok_or("worker omitted canonical session construction")?;
    assert!(verification < tensor_construction && tensor_construction < session_construction);
    assert!(worker_session.contains("BinaryOperation::Add"));
    assert!(worker_session.contains("backend readiness probe retained device allocations"));
    assert!(!worker.contains("WorkerBackendSelection::DirectMl => WorkerBackendSelection::Cpu"));
    assert!(!worker.contains("NativeFfiRegistry::"));
    assert!(!tensor.contains("NativeFfiRegistry::"));
    assert!(runtime.contains("NativeFfiRegistry::new"));
    assert!(runtime.contains("observe_directml_candidate"));
    assert!(loader.contains("WinVerifyTrust"));
    Ok(())
}

#[test]
fn cpu_vae_convolution_bridge_preserves_canonical_owners() -> Result<(), Box<dyn std::error::Error>>
{
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let cpu_backend = fs::read_to_string(root.join("crates/comfy_tensor/src/cpu_backend.rs"))?;
    let native_ops = fs::read_to_string(root.join("crates/comfy_model/src/native_ops.rs"))?;
    let vae = fs::read_to_string(root.join("crates/comfy_model/src/vae.rs"))?;
    let vae_image = fs::read_to_string(root.join("crates/comfy_model/src/vae_image.rs"))?;
    let assets = fs::read_to_string(root.join("crates/comfy_runtime/src/assets.rs"))?;
    let policy: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/specs/comfy-parity/ownership-policy.json"),
    )?)?;

    assert!(cpu_backend.contains("ConvolutionGeometry::new"));
    assert!(cpu_backend.contains("convolution_into_with_context_exact_native"));
    assert!(!cpu_backend.contains("fn map_padded_coordinate"));
    assert_eq!(
        production_source_occurrences(&sources, "fn validate_native_vae_backend_binding",).len(),
        1
    );
    assert!(vae.contains("pub fn validate_native_vae_backend_target"));
    assert!(vae.contains("dtype != DType::F32"));
    assert!(vae_image.contains("crate::vae::validate_native_vae_backend_binding"));
    assert_eq!(
        production_source_occurrences(&sources, "fn materialize_execution_state_with_context",)
            .len(),
        1
    );
    assert!(native_ops.contains("self.prepare_parameters("));
    assert!(vae_image.contains("module.materialize_execution_state_with_context"));
    let materialization = vae_image
        .find("let module = build_native_module")
        .ok_or("image VAE loader omitted pre-binding module materialization")?;
    let binding = vae_image[materialization..]
        .find("VaeModelBinding::checked")
        .map(|position| materialization + position)
        .ok_or("image VAE loader omitted canonical model binding")?;
    assert!(materialization < binding);
    let image_load = assets
        .find("pub fn load_image_vae_with_context")
        .ok_or("asset service omitted image VAE loader")?;
    let image_load = &assets[image_load..];
    let authorization = image_load
        .find("require_asset_authorization")
        .ok_or("asset service omitted canonical authorization")?;
    let admission = image_load
        .find("validate_native_vae_backend_target")
        .ok_or("asset service omitted canonical VAE target admission")?;
    let model_load = image_load[admission..]
        .find("self.load_model")
        .map(|position| admission + position)
        .ok_or("asset service omitted canonical model load")?;
    assert!(authorization < admission && admission < model_load);
    assert!(assets.contains("pub fn load_and_execute_image_vae_with_context"));

    let concerns = policy
        .get("concerns")
        .and_then(serde_json::Value::as_array)
        .ok_or("ownership policy has no concerns")?;
    for concern_name in [
        "linear_convolution_kernel_mechanics",
        "native_model_vae_domain",
    ] {
        let concern = concerns
            .iter()
            .find(|concern| {
                concern.get("concern").and_then(serde_json::Value::as_str) == Some(concern_name)
            })
            .ok_or("ownership policy omitted Task 514 concern")?;
        assert!(
            concern
                .get("consolidation_tasks")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|tasks| tasks.iter().any(|task| {
                    task.as_str() == Some("comfy-parity-vae-canonical-cpu-execution-bridge")
                }))
        );
    }
    Ok(())
}

#[test]
fn val_ownership_001_task353_vae_tiling_preserves_canonical_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    assert_eq!(
        production_source_occurrences(&sources, "pub struct VaeTilePlan").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub enum VaeTileAxisFormula").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "fn execute_tiled_scale<").len(),
        1
    );

    let tiling = fs::read_to_string(root.join("crates/comfy_model/src/vae_tiling.rs"))?;
    let production = tiling.split("#[cfg(test)]").next().unwrap_or(&tiling);
    assert!(production.contains("if pass.tile_count == 1"));
    assert!(production.contains("backend.replace_rectangular_slice("));
    assert!(production.contains("backend.reserve_workspace(context"));
    assert!(production.contains("context.check()?"));
    for forbidden in [
        "CpuBackend",
        "CpuWorkspaceAuthority",
        "authorize_workspace",
        "CancellationToken::default",
        "Command::new",
    ] {
        assert!(!production.contains(forbidden));
    }
    assert!(!production.to_ascii_lowercase().contains("retry"));

    let policy: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/specs/comfy-parity/ownership-policy.json"),
    )?)?;
    let concern = policy
        .get("concerns")
        .and_then(serde_json::Value::as_array)
        .and_then(|concerns| {
            concerns.iter().find(|concern| {
                concern.get("concern").and_then(serde_json::Value::as_str)
                    == Some("native_model_vae_domain")
            })
        })
        .ok_or("ownership policy omitted the canonical VAE concern")?;
    assert!(
        concern
            .get("consolidation_tasks")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|tasks| tasks
                .iter()
                .any(|task| { task.as_str() == Some("comfy-parity-vae-multidimensional-tiling") }))
    );
    Ok(())
}

#[test]
fn val_ownership_001_task354_image_vae_preserves_canonical_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    for symbol in [
        "pub struct NativeImageVaeArchitecture",
        "pub fn inspect_image_vae_architecture",
        "pub fn load_image_vae_from_model_store_with_context",
        "pub fn load_image_vae_with_context",
        "pub fn load_and_execute_image_vae_with_context",
    ] {
        assert_eq!(production_source_occurrences(&sources, symbol).len(), 1);
    }

    let image = fs::read_to_string(root.join("crates/comfy_model/src/vae_image.rs"))?;
    let image_production = image
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(image.as_str(), |(production, _)| production);
    for delegation in [
        "canonical_vision_model_store_dtype",
        "VaeModelBinding::checked",
        "NativeVae::checked_kernel",
        "validate_native_vae_backend_binding",
        "backend.convolution",
        "backend.resize",
        "context.check()?",
    ] {
        assert!(
            image_production.contains(delegation),
            "missing {delegation}"
        );
    }
    for forbidden in [
        "CancellationToken::default",
        "CpuWorkspaceAuthority",
        "authorize_workspace",
        "Command::new",
        "std::process",
        "OutputCommitter",
    ] {
        assert!(
            !image_production.contains(forbidden),
            "duplicate owner {forbidden}"
        );
    }
    assert!(!image_production.to_ascii_lowercase().contains("python"));
    assert!(!image_production.to_ascii_lowercase().contains("retry"));

    let artifact_writer = ["val-vae-001", ".json.tmp"].concat();
    assert_eq!(
        sources
            .iter()
            .filter(|(_, source)| source.contains(&artifact_writer))
            .count(),
        1
    );

    let policy: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/specs/comfy-parity/ownership-policy.json"),
    )?)?;
    let concern = policy
        .get("concerns")
        .and_then(serde_json::Value::as_array)
        .and_then(|concerns| {
            concerns.iter().find(|concern| {
                concern.get("concern").and_then(serde_json::Value::as_str)
                    == Some("native_model_vae_domain")
            })
        })
        .ok_or("ownership policy omitted the canonical VAE concern")?;
    let consolidation_tasks = concern
        .get("consolidation_tasks")
        .and_then(serde_json::Value::as_array)
        .ok_or("canonical VAE concern omitted consolidation tasks")?;
    for dependency in [
        "comfy-parity-vae-image-foundation-consolidation",
        "comfy-parity-vae-image-adapter-ownership-consolidation",
        "comfy-parity-vae-canonical-cpu-execution-bridge",
    ] {
        assert!(
            consolidation_tasks
                .iter()
                .any(|task| task.as_str() == Some(dependency))
        );
    }
    Ok(())
}

#[test]
fn val_ownership_001_task346_audio_vae_preserves_canonical_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    for symbol in [
        "pub struct NativeAudioVaeArchitecture",
        "pub fn inspect_audio_vae_architecture",
        "pub fn load_audio_vae_from_model_store_with_context",
        "pub fn load_audio_vae_with_context",
        "pub fn load_and_execute_audio_vae_with_context",
        "fn loaded_mel_spectrogram",
    ] {
        assert_eq!(
            production_source_occurrences(&sources, symbol).len(),
            1,
            "{symbol} must have exactly one production owner"
        );
    }

    let audio = fs::read_to_string(root.join("crates/comfy_model/src/vae_audio.rs"))?;
    let audio_production = audio
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(audio.as_str(), |(production, _)| production);
    for delegation in [
        "load_vision_state_from_model_store_with_context",
        "VaeModelBinding::checked",
        "NativeVae::checked_kernel",
        "validate_native_vae_backend_binding",
        "context.check()?",
    ] {
        assert!(
            audio_production.contains(delegation),
            "missing {delegation}"
        );
    }
    for forbidden in [
        "CancellationToken::default",
        "CpuWorkspaceAuthority",
        "authorize_workspace",
        "Command::new",
        "std::process",
        "OutputCommitter",
        "struct AudioModelStore",
        "struct AudioAssetService",
        "retry",
    ] {
        assert!(
            !audio_production.contains(forbidden),
            "duplicate owner {forbidden}"
        );
    }

    let assets = fs::read_to_string(root.join("crates/comfy_runtime/src/assets.rs"))?;
    let audio_load = assets
        .find("pub fn load_audio_vae_with_context")
        .ok_or("asset service omitted audio VAE loader")?;
    let audio_load = &assets[audio_load..];
    let authorization = audio_load
        .find("require_asset_authorization")
        .ok_or("asset service omitted canonical authorization")?;
    let admission = audio_load
        .find("validate_native_vae_backend_target")
        .ok_or("asset service omitted canonical VAE target admission")?;
    let model_load = audio_load[admission..]
        .find("self.load_model")
        .map(|position| admission + position)
        .ok_or("asset service omitted canonical model load")?;
    assert!(authorization < admission && admission < model_load);
    assert!(assets.contains("pub fn load_and_execute_audio_vae_with_context"));

    let policy: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/specs/comfy-parity/ownership-policy.json"),
    )?)?;
    let concern = policy
        .get("concerns")
        .and_then(serde_json::Value::as_array)
        .and_then(|concerns| {
            concerns.iter().find(|concern| {
                concern.get("concern").and_then(serde_json::Value::as_str)
                    == Some("native_model_vae_domain")
            })
        })
        .ok_or("ownership policy omitted the canonical VAE concern")?;
    assert!(
        concern
            .get("consolidation_tasks")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|tasks| tasks
                .iter()
                .any(|task| { task.as_str() == Some("comfy-parity-vae-audio-architectures") }))
    );
    Ok(())
}

#[test]
fn val_ownership_001_task347_structured_vae_preserves_canonical_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    for symbol in [
        "pub struct NativeStructuredVae {",
        "pub enum VaeStructuredDecodeRequest",
        "pub enum VaeStructuredResult",
        "pub fn load_structured_vae_from_model_store_with_context",
        "pub fn load_structured_vae_with_context",
        "pub fn load_and_decode_structured_vae_with_context",
    ] {
        assert_eq!(
            production_source_occurrences(&sources, symbol).len(),
            1,
            "{symbol} must have exactly one production owner"
        );
    }

    let structured = fs::read_to_string(root.join("crates/comfy_model/src/vae_structured.rs"))?;
    let structured_production = structured
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(structured.as_str(), |(production, _)| production);
    for delegation in [
        "load_vision_state_from_model_store_with_context",
        "VaeModelBinding::checked",
        "NativeStructuredVae::checked_kernel",
        "validate_native_vae_backend_binding",
        "begin_vae_rng(context)",
        "backend.workspace_vec",
        "context.check()?",
        "parameter_tensor(module, \"gs.points_offset_perturbation\")",
        "parameter_tensor(module, \"gs.base_offset_scale\")",
    ] {
        assert!(
            structured_production.contains(delegation),
            "missing {delegation}"
        );
    }
    for forbidden in [
        "CancellationToken::default",
        "CpuWorkspaceAuthority",
        "authorize_workspace",
        "Command::new",
        "std::process",
        "OutputCommitter",
        "struct StructuredModelStore",
        "struct StructuredAssetService",
        "struct Mesh",
        "struct PointCloud",
        "struct GaussianSplatDomain",
        "retry",
    ] {
        assert!(
            !structured_production.contains(forbidden),
            "duplicate owner {forbidden}"
        );
    }
    assert!(
        !structured_production
            .to_ascii_lowercase()
            .contains("python")
    );

    let vae = fs::read_to_string(root.join("crates/comfy_model/src/vae.rs"))?;
    assert!(vae.contains("validate_generic_vae_boundary(descriptor.boundary())?;"));
    assert!(vae.contains("Err(VaeError::StructuredDecodeRequired)"));
    assert!(vae.contains("process_latent_out(self.latent_definition"));

    let assets = fs::read_to_string(root.join("crates/comfy_runtime/src/assets.rs"))?;
    let structured_load = assets
        .find("pub fn load_structured_vae_with_context")
        .ok_or("asset service omitted structured VAE loader")?;
    let structured_load = &assets[structured_load..];
    let authorization = structured_load
        .find("require_asset_authorization")
        .ok_or("asset service omitted canonical authorization")?;
    let admission = structured_load
        .find("validate_native_vae_backend_target")
        .ok_or("asset service omitted canonical VAE target admission")?;
    let model_load = structured_load[admission..]
        .find("self.load_model")
        .map(|position| admission + position)
        .ok_or("asset service omitted canonical model load")?;
    assert!(authorization < admission && admission < model_load);

    let policy: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/specs/comfy-parity/ownership-policy.json"),
    )?)?;
    let concern = policy
        .get("concerns")
        .and_then(serde_json::Value::as_array)
        .and_then(|concerns| {
            concerns.iter().find(|concern| {
                concern.get("concern").and_then(serde_json::Value::as_str)
                    == Some("native_model_vae_domain")
            })
        })
        .ok_or("ownership policy omitted the canonical VAE concern")?;
    let symbols = concern
        .get("owner_symbols")
        .and_then(serde_json::Value::as_array)
        .ok_or("canonical VAE concern omitted owner symbols")?;
    for symbol in [
        "NativeStructuredVae",
        "VaeStructuredDecodeRequest",
        "VaeStructuredResult",
    ] {
        assert!(symbols.iter().any(|value| value.as_str() == Some(symbol)));
    }
    assert!(
        concern
            .get("consolidation_tasks")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|tasks| tasks.iter().any(|task| {
                task.as_str() == Some("comfy-parity-vae-structured-architectures")
            }))
    );
    Ok(())
}

#[test]
fn native_module_backend_admission_preserves_canonical_capability_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let native_ops = fs::read_to_string(root.join("crates/comfy_model/src/native_ops.rs"))?;
    let clip = fs::read_to_string(root.join("crates/comfy_model/src/clip.rs"))?;
    let clip_text = fs::read_to_string(root.join("crates/comfy_model/src/clip_text.rs"))?;
    let clip_vision = fs::read_to_string(root.join("crates/comfy_model/src/clip_vision.rs"))?;

    assert_eq!(
        production_source_occurrences(&sources, "pub struct BackendCapabilityMatrix").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct NativeModule {").len(),
        1
    );
    assert_eq!(
        production_source_occurrences(&sources, "pub struct NativeExecutionRequirements {").len(),
        1
    );
    assert!(
        native_ops
            .contains("capabilities.require(\"sim.comfy_model.native_module.execute\", support)?")
    );
    assert!(!native_ops.contains("pub struct NativeBackendCapability"));
    assert!(!native_ops.contains("pub struct NativeExecutionContext"));
    assert!(native_ops.contains("child.append_execution_requirements(dtype, requirements)"));
    assert!(native_ops.contains("let capabilities = backend.capabilities();"));
    assert!(native_ops.contains("if capabilities.device() != backend.device()"));
    assert!(native_ops.contains("OperationSupport::record_event()"));
    assert!(native_ops.contains("OperationSupport::wait_event()"));
    assert!(clip.matches(".admit_backend_target(").count() >= 2);
    assert_eq!(clip_text.matches(".admit_backend_target(").count(), 1);
    assert!(clip_vision.matches(".admit_backend_target(").count() >= 4);
    assert!(!clip.contains("require_supported(capabilities)"));
    assert!(!clip_text.contains("require_supported(capabilities)"));
    assert!(!clip_vision.contains("require_supported(backend.capabilities())"));
    assert!(!native_ops.contains("pub fn require_supported("));
    let execute = clip
        .find("impl NativeTextEncoder for Sd1ClipTextEncoder")
        .ok_or("SD1 CLIP executor implementation is missing")?;
    let execute = &clip[execute..];
    let admission = execute
        .find("self.admit_execution_target(plan, context)?")
        .ok_or("SD1 CLIP executor omitted target admission")?;
    let binding_validation = execute
        .find("self.validate_bindings(plan, batch, context)?")
        .ok_or("SD1 CLIP executor omitted binding validation")?;
    let canonical_admission = execute
        .find(".admit_execution_target(&self.backend, context)?")
        .ok_or("SD1 CLIP executor omitted canonical backend admission")?;
    let token_input = execute
        .find("sd1_token_input(")
        .ok_or("SD1 CLIP executor omitted token input construction")?;
    let first_tensor_read = execute
        .find("tensor_to_f32(")
        .ok_or("SD1 CLIP executor omitted tensor execution")?;
    assert!(
        admission < binding_validation
            && binding_validation < canonical_admission
            && canonical_admission < token_input
            && token_input < first_tensor_read
    );
    Ok(())
}

#[test]
fn val_ownership_001_task346_text_encoder_registry_preserves_canonical_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    let registry = fs::read_to_string(root.join("crates/comfy_model/src/clip_text_encoders.rs"))?;
    let module_root = fs::read_to_string(root.join("crates/comfy_model/src/comfy_model.rs"))?;
    let design = fs::read_to_string(root.join(".agents/specs/comfy-parity/design.md"))?;

    assert_eq!(
        production_source_occurrences(&sources, "pub struct TextEncoderArchitectureRegistry {")
            .len(),
        1
    );
    for owner in [
        "comfy_model::clip_text_encoder_t5",
        "comfy_model::clip_text_encoder_decoder",
        "comfy_model::clip_text_encoder_multimodal",
        "comfy_model::clip_text_encoder_composite",
    ] {
        assert!(registry.contains(owner));
    }
    assert!(registry.contains("TEXT_ENCODER_ARCHITECTURE_CONTRACT_COUNT: usize = 398"));
    assert!(registry.contains("TEXT_ENCODER_ARCHITECTURE_REGISTRY_VERSION"));
    assert!(registry.contains("pub fn identity_sha256"));
    assert!(module_root.contains("pub mod clip_text_encoders;"));
    assert!(module_root.contains("TextEncoderArchitectureRegistry"));
    for forbidden in [
        "CpuBackend",
        "NativeModule",
        "RngStream",
        "NativeCache",
        "ModelStore",
        "OutputTransaction",
        "pub fn forward(",
    ] {
        assert!(
            !registry.contains(forbidden),
            "registry contains {forbidden}"
        );
    }

    let policy: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/specs/comfy-parity/ownership-policy.json"),
    )?)?;
    let concern = policy
        .get("concerns")
        .and_then(serde_json::Value::as_array)
        .and_then(|concerns| {
            concerns.iter().find(|concern| {
                concern.get("concern").and_then(serde_json::Value::as_str)
                    == Some("native_text_encoder_architecture_registry")
            })
        })
        .ok_or("ownership policy omitted the text-encoder registry concern")?;
    assert_eq!(
        concern
            .get("canonical_owner")
            .and_then(serde_json::Value::as_str),
        Some("comfy_model::clip_text_encoders::TextEncoderArchitectureRegistry")
    );
    assert!(
        concern
            .get("consolidation_tasks")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|tasks| tasks
                .iter()
                .any(|task| { task.as_str() == Some("comfy-parity-clip-text-encoder-breadth") }))
    );
    assert!(
        design.contains("is the sole versioned routing and contract-ownership table"),
        "design.md must document the text-encoder registry as the sole versioned routing table"
    );
    Ok(())
}

fn validate_native_stored_payload_boundary(
    root: &Path,
    sources: &[(PathBuf, String)],
) -> Result<bool, Box<dyn std::error::Error>> {
    for forbidden in [
        "NativeStructuredComputePayload",
        "NativeStructuredComputeRole",
        "NativeStructuredComputeValue",
        "NativeStoredObject",
        "NativeStoredTensorObject",
        "NativeStoredModelObject",
        "NativeStoredArtifactObject",
        "StoredNativeTensor",
        "StoredNativeDiffusion",
        "StoredNativeModel",
        "StoredNativeArtifact",
    ] {
        let occurrences = production_identifier_occurrences(&sources, forbidden);
        assert!(
            occurrences.is_empty(),
            "legacy native payload owner {forbidden} remains at {occurrences:?}"
        );
    }

    let boundary_paths = [
        "crates/comfy_nodes/src/execution.rs",
        "crates/comfy_nodes/src/stored_payload.rs",
        "crates/comfy_runtime/src/executor.rs",
        "crates/comfy_plugin_host/src/registry_adapter.rs",
    ];
    for relative_path in boundary_paths {
        let source = fs::read_to_string(root.join(relative_path))?;
        let production = source
            .split_once("#[cfg(test)]\nmod tests")
            .map_or(source.as_str(), |(production, _)| production);
        for forbidden in [
            "Arc<dyn Any",
            "Arc < dyn Any",
            ".downcast::<",
            ".downcast_ref::<",
            ".downcast_mut::<",
            ".as_any()",
        ] {
            assert!(
                !production.contains(forbidden),
                "native payload boundary {relative_path} contains {forbidden}"
            );
        }
    }
    let execution = fs::read_to_string(root.join("crates/comfy_nodes/src/execution.rs"))?;
    let prepared_effect_declaration = execution
        .split_once("pub struct NativePreparedEffectRequest {")
        .and_then(|(prefix, declaration)| {
            prefix.rsplit_once("#[derive(").map(|(_, derive)| {
                format!("#[derive({derive}pub struct NativePreparedEffectRequest {{{declaration}")
            })
        })
        .and_then(|declaration| {
            declaration
                .split_once("impl NativePreparedEffectRequest")
                .map(|(declaration, _)| declaration.to_owned())
        })
        .ok_or("native prepared-effect ticket declaration is missing")?;
    assert!(!prepared_effect_declaration.contains("Serialize"));
    assert!(!prepared_effect_declaration.contains("Deserialize"));
    for (path, source) in sources {
        let path = path.to_string_lossy();
        if path.contains("crates/comfy_worker/")
            || path.contains("crates/comfy_api/")
            || path.ends_with("crates/comfy_types/src/worker_protocol.rs")
        {
            if path.contains("/tests/") {
                continue;
            }
            let production = source
                .split_once("#[cfg(test)]")
                .map_or(source.as_str(), |(production, _)| production);
            for capability in [
                "NativeAssetReference",
                "NativeAssetReadRequest",
                "NativeNodeComputeSession",
                "NativeNodeServices",
                "NativePreparedEffectRequest",
            ] {
                assert!(
                    !production.contains(capability),
                    "process-local native capability {capability} leaked into {path}"
                );
            }
        }
    }
    assert!(
        production_source_occurrences(&sources, "impl<T> NativeResolvedPayloadRetention")
            .is_empty()
    );
    let resolved_payload_constructors =
        production_source_occurrences(&sources, "NativeResolvedPayload::checked(");
    assert_eq!(resolved_payload_constructors.len(), 1);
    assert!(resolved_payload_constructors[0].contains("crates/comfy_runtime/src/executor.rs"));

    let owner_definitions = [
        (
            "pub struct NativeAssetReference {",
            "crates/comfy_nodes/src/execution.rs",
        ),
        (
            "pub trait NativeAssetResolver:",
            "crates/comfy_nodes/src/execution.rs",
        ),
        (
            "pub trait NativePreparedEffectService:",
            "crates/comfy_nodes/src/execution.rs",
        ),
        (
            "pub struct NativeNodeComputeSession {",
            "crates/comfy_nodes/src/execution.rs",
        ),
        (
            "pub struct NativeNodeServices {",
            "crates/comfy_nodes/src/execution.rs",
        ),
        (
            "pub struct NativePreparedEffectRequest {",
            "crates/comfy_nodes/src/execution.rs",
        ),
        (
            "pub struct NativeAssetResolverRegistry {",
            "crates/comfy_runtime/src/assets.rs",
        ),
        (
            "pub enum NativeStoredPayload {",
            "crates/comfy_nodes/src/stored_payload.rs",
        ),
        (
            "pub struct NativeStoredModelPayload {",
            "crates/comfy_nodes/src/stored_payload.rs",
        ),
        (
            "pub struct NativeTensorPayload {",
            "crates/comfy_tensor/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeModelPayload {",
            "crates/comfy_model/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeModelResidentAllocation {",
            "crates/comfy_model/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeModelTensorResidentAllocation {",
            "crates/comfy_model/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeModelResidentParts {",
            "crates/comfy_model/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeStructuredResidentParts {",
            "crates/comfy_model/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeRaftLarge {",
            "crates/comfy_model/src/vision_models.rs",
        ),
        (
            "pub struct NativeRaftTensorResidentAllocation {",
            "crates/comfy_model/src/vision_models.rs",
        ),
        (
            "pub struct NativeRaftResidentParts {",
            "crates/comfy_model/src/vision_models.rs",
        ),
        (
            "pub struct NativeClipVision {",
            "crates/comfy_model/src/clip_vision.rs",
        ),
        (
            "pub struct ClipVisionTensorResidentAllocation {",
            "crates/comfy_model/src/clip_vision.rs",
        ),
        (
            "pub struct ClipVisionResidentParts {",
            "crates/comfy_model/src/clip_vision.rs",
        ),
        (
            "pub struct ConditioningSet {",
            "crates/comfy_model/src/conditioning.rs",
        ),
        (
            "pub struct ConditioningTensorResidentAllocation {",
            "crates/comfy_model/src/conditioning.rs",
        ),
        (
            "pub struct ConditioningResidentParts {",
            "crates/comfy_model/src/conditioning.rs",
        ),
        (
            "pub struct ControlChainTensorResidentAllocation {",
            "crates/comfy_model/src/controlnet.rs",
        ),
        (
            "pub struct ControlChainResidentParts {",
            "crates/comfy_model/src/controlnet.rs",
        ),
        (
            "pub struct NativeControlPayload {",
            "crates/comfy_sampler/src/native_diffusion_payload.rs",
        ),
        (
            "pub struct NativeConditioningPayload {",
            "crates/comfy_sampler/src/native_diffusion_payload.rs",
        ),
        (
            "pub struct NativeDiffusionPayload {",
            "crates/comfy_sampler/src/native_diffusion_payload.rs",
        ),
        (
            "pub enum NativeDiffusionResidentAllocationId {",
            "crates/comfy_sampler/src/native_diffusion_payload.rs",
        ),
        (
            "pub struct NativeDiffusionResidentAllocation {",
            "crates/comfy_sampler/src/native_diffusion_payload.rs",
        ),
        (
            "pub struct NativeDiffusionResidentParts {",
            "crates/comfy_sampler/src/native_diffusion_payload.rs",
        ),
        (
            "pub struct NativeNoisePayload {",
            "crates/comfy_sampler/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeNoiseTensorResidentAllocation {",
            "crates/comfy_sampler/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeNoiseResidentParts {",
            "crates/comfy_sampler/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeGuiderPayload {",
            "crates/comfy_sampler/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeSamplerPayload {",
            "crates/comfy_sampler/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeBoundingBoxPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeFaceLandmarksPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativePoseKeypointPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeSam3TrackDataPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeTracksPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeAudioPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeVideoPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeArtifactPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeFile3DPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeCameraPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeSplatPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeMeshPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeVoxelPayload {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeMediaTensorResidentAllocation {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeMediaResidentParts {",
            "crates/comfy_media/src/native_node_payload.rs",
        ),
        (
            "pub struct AudioEncoderOutput {",
            "crates/comfy_model/src/native_node_payload.rs",
        ),
        (
            "pub struct ClipVisionOutput {",
            "crates/comfy_model/src/clip_vision.rs",
        ),
        (
            "pub struct IcLoraParameters {",
            "crates/comfy_model/src/native_node_payload.rs",
        ),
        (
            "pub struct LossMap {",
            "crates/comfy_model/src/native_node_payload.rs",
        ),
        (
            "pub struct NativeProviderPayload {",
            "crates/comfy_nodes/src/stored_payload.rs",
        ),
    ];
    for (definition, owner_path) in owner_definitions {
        let occurrences = production_source_occurrences(&sources, definition);
        assert_eq!(
            occurrences.len(),
            1,
            "{definition} must have exactly one production owner: {occurrences:?}"
        );
        assert!(
            occurrences[0].contains(owner_path),
            "{definition} is not owned by {owner_path}: {occurrences:?}"
        );
    }

    let source_type_mapper =
        production_source_occurrences(&sources, "pub fn native_source_type_projection(");
    assert_eq!(source_type_mapper.len(), 1);
    assert!(source_type_mapper[0].contains("crates/comfy_nodes/src/source_type.rs"));

    let stored_payload = fs::read_to_string(root.join("crates/comfy_nodes/src/stored_payload.rs"))?;
    let stored_payload_production = stored_payload
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(stored_payload.as_str(), |(production, _)| production);
    for closed_variant in [
        "Tensor(Arc<NativeTensorPayload>)",
        "Model(Arc<NativeStoredModelPayload>)",
        "Control(Arc<NativeControlPayload>)",
        "Conditioning(Arc<ConditioningSet>)",
        "Noise(Arc<NativeNoisePayload>)",
        "Guider(Arc<NativeGuiderPayload>)",
        "Sampler(Arc<NativeSamplerPayload>)",
        "BoundingBox(Arc<NativeBoundingBoxPayload>)",
        "FaceLandmarks(Arc<NativeFaceLandmarksPayload>)",
        "PoseKeypoint(Arc<NativePoseKeypointPayload>)",
        "Sam3TrackData(Arc<NativeSam3TrackDataPayload>)",
        "Tracks(Arc<NativeTracksPayload>)",
        "AudioEncoderOutput(Arc<AudioEncoderOutput>)",
        "ClipVisionOutput(Arc<ClipVisionOutput>)",
        "IcLoraParameters(Arc<IcLoraParameters>)",
        "LossMap(Arc<LossMap>)",
        "Audio(Arc<NativeAudioPayload>)",
        "Video(Arc<NativeVideoPayload>)",
        "Artifact(Arc<NativeArtifactPayload>)",
        "File3D(Arc<NativeFile3DPayload>)",
        "Camera(Arc<NativeCameraPayload>)",
        "Splat(Arc<NativeSplatPayload>)",
        "Mesh(Arc<NativeMeshPayload>)",
        "Voxel(Arc<NativeVoxelPayload>)",
        "Provider(Arc<NativeProviderPayload>)",
    ] {
        assert!(
            stored_payload_production.contains(closed_variant),
            "closed NativeStoredPayload omits {closed_variant}"
        );
    }
    assert!(stored_payload_production.contains("native_source_type_projection(source_type)?"));

    let stored_payload_variants = stored_payload_production
        .split_once("pub enum NativeStoredPayload {")
        .and_then(|(_, variants)| variants.split_once("\n}"))
        .map(|(variants, _)| variants)
        .ok_or("closed NativeStoredPayload declaration is missing")?;
    assert!(!stored_payload_variants.contains("Diffusion(Arc<NativeDiffusionPayload>)"));
    assert!(!stored_payload_variants.contains("OpticalFlow"));
    assert!(!stored_payload_variants.contains("ClipVision("));
    assert!(stored_payload_variants.contains("ClipVisionOutput(Arc<ClipVisionOutput>)"));
    assert!(stored_payload_production.contains("enum NativeStoredModelResource {"));
    assert!(!stored_payload_production.contains("pub enum NativeStoredModelResource"));
    assert!(
        stored_payload_production.contains("ModelResource(Arc<NativeModelPayload>)"),
        "OPTICAL_FLOW must route through the private model-resource branch"
    );
    assert!(stored_payload_production.contains("fn require_model_resource_role("));
    assert!(
        stored_payload_production
            .contains("NativeModelResourceRole::OpticalFlow | NativeModelResourceRole::ClipVision")
    );
    assert!(stored_payload_production.contains(
        "NativeModelResourceRole::OpticalFlow => resource.optical_flow_resource().is_some()"
    ));
    assert!(
        stored_payload_production
            .contains("Err(NativeStoredPayloadError::NonCanonicalModelResourceRole { role })")
    );
    assert!(stored_payload_production.contains("NativeModelResourceRole::ClipVision"));
    assert!(stored_payload_production.contains(
        "NativeModelResourceRole::ClipVision => resource.clip_vision_resource().is_some()"
    ));
    assert!(
        stored_payload_production
            .contains("Self::ClipVisionOutput(_) => ClipVisionOutput::SOURCE_TYPE_ID")
    );

    let residency_projection = stored_payload_production
        .split_once("pub fn residency(&self)")
        .and_then(|(_, projection)| projection.split_once("fn arc_address"))
        .map(|(projection, _)| projection)
        .ok_or("NativeStoredPayload residency projection is missing")?;
    for required_projection in [
        "NativeStoredModelResource::Diffusion(diffusion) => {\n                    stored_diffusion_residency(payload, diffusion)?",
        "Self::Control(payload) => control_residency(payload)?",
        "Self::Noise(payload) => noise_residency(payload)?",
        "Self::Sam3TrackData(payload) => media_residency(",
        "Self::Tracks(payload) => media_residency(",
        "Self::AudioEncoderOutput(payload) => structured_model_residency(",
        "Self::LossMap(payload) => structured_model_residency(",
        "Self::Audio(payload) => media_residency(",
        "Self::Video(payload) => media_residency(",
        "Self::Splat(payload) => media_residency(",
        "Self::Mesh(payload) => media_residency(",
        "Self::Voxel(payload) => media_residency(",
    ] {
        assert!(
            residency_projection.contains(required_projection),
            "native stored residency omits checked projection {required_projection}"
        );
    }
    for forbidden_projection in [
        "Self::Control(payload) => single_arc_residency",
        "Self::Noise(payload) => single_arc_residency",
        "Self::Sam3TrackData(payload) => single_arc_residency",
        "Self::Tracks(payload) => single_arc_residency",
        "Self::AudioEncoderOutput(payload) => single_arc_residency",
        "Self::LossMap(payload) => single_arc_residency",
        "Self::Audio(payload) => single_arc_residency",
        "Self::Video(payload) => single_arc_residency",
        "Self::Splat(payload) => single_arc_residency",
        "Self::Mesh(payload) => single_arc_residency",
        "Self::Voxel(payload) => single_arc_residency",
        "NativeStoredModelResource::Diffusion(diffusion) => single_arc_residency",
    ] {
        assert!(
            !residency_projection.contains(forbidden_projection),
            "tensor-backed payload uses coarse residency: {forbidden_projection}"
        );
    }
    for resident_parts_adapter in [
        "fn media_residency<T>(",
        "parts: NativeMediaResidentParts",
        "fn structured_model_residency<T>(",
        "parts: NativeStructuredResidentParts",
        "fn noise_residency(",
        "let parts = payload.resident_parts()?;",
        "fn control_residency(",
        "fn stored_diffusion_residency(",
        "fn diffusion_allocations(",
    ] {
        assert!(
            stored_payload_production.contains(resident_parts_adapter),
            "native stored payload omits resident-parts adapter {resident_parts_adapter}"
        );
    }
    let tensor_backed_adapter = stored_payload_production
        .split_once("fn tensor_backed_arc_residency<T>(")
        .and_then(|(_, adapter)| adapter.split_once("fn media_residency<T>("))
        .map(|(adapter, _)| adapter)
        .ok_or("tensor-backed Arc residency adapter is missing")?;
    for required in [
        "arc_allocation(",
        "NativeResidentAllocationId::TensorStorage { storage_id }",
        "resident_bytes: usize::try_from(resident_bytes)",
        "NativePayloadResidency::checked(0, allocations)",
    ] {
        assert!(
            tensor_backed_adapter.contains(required),
            "tensor-backed Arc residency omits {required}"
        );
    }
    let media_adapter = stored_payload_production
        .split_once("fn media_residency<T>(")
        .and_then(|(_, adapter)| adapter.split_once("fn structured_model_residency<T>("))
        .map(|(adapter, _)| adapter)
        .ok_or("media resident-parts adapter is missing")?;
    let structured_adapter = stored_payload_production
        .split_once("fn structured_model_residency<T>(")
        .and_then(|(_, adapter)| adapter.split_once("fn noise_residency("))
        .map(|(adapter, _)| adapter)
        .ok_or("structured-model resident-parts adapter is missing")?;
    for (name, adapter) in [
        ("media", media_adapter),
        ("structured model", structured_adapter),
    ] {
        for required in [
            "tensor_backed_arc_residency(",
            "parts.owned_bytes()",
            ".tensor_allocations()",
            "allocation.storage_id().get()",
            "allocation.resident_bytes()",
        ] {
            assert!(
                adapter.contains(required),
                "{name} resident-parts adapter omits {required}"
            );
        }
        assert!(!adapter.contains("single_arc_residency"));
    }
    let noise_adapter = stored_payload_production
        .split_once("fn noise_residency(")
        .and_then(|(_, adapter)| adapter.split_once("fn translate_diffusion_allocation("))
        .map(|(adapter, _)| adapter)
        .ok_or("noise resident-parts adapter is missing")?;
    for required in [
        "payload.resident_parts()?",
        "tensor_backed_arc_residency(",
        "parts.owned_bytes()",
        ".tensor_allocation()",
        "allocation.storage_id().get()",
        "allocation.resident_bytes()",
    ] {
        assert!(
            noise_adapter.contains(required),
            "noise resident-parts adapter omits {required}"
        );
    }
    assert!(!noise_adapter.contains("single_arc_residency"));

    let diffusion_payload =
        fs::read_to_string(root.join("crates/comfy_sampler/src/native_diffusion_payload.rs"))?;
    let diffusion_allocation_ids = diffusion_payload
        .split_once("pub enum NativeDiffusionResidentAllocationId {")
        .and_then(|(_, ids)| ids.split_once("\n}"))
        .map(|(ids, _)| ids)
        .ok_or("NativeDiffusionResidentAllocationId declaration is missing")?;
    let diffusion_translation = stored_payload_production
        .split_once("fn translate_diffusion_allocation(")
        .and_then(|(_, translation)| translation.split_once("fn diffusion_allocations("))
        .map(|(translation, _)| translation)
        .ok_or("native diffusion resident allocation translation is missing")?;
    let diffusion_allocation_variants = [
        "ModelPayloadArc",
        "ModelBacking",
        "ConditioningPayloadArc",
        "PatchGraphArc",
        "ControlExecutionArc",
        "ControlChainArc",
        "ControlExecutorArc",
        "TensorStorage",
    ];
    for variant in diffusion_allocation_variants {
        assert!(
            diffusion_allocation_ids.contains(&format!("{variant} {{")),
            "lower diffusion residency omits {variant}"
        );
        assert!(
            diffusion_translation
                .contains(&format!("NativeDiffusionResidentAllocationId::{variant}")),
            "stored payload omits lower diffusion allocation {variant}"
        );
        assert!(
            diffusion_translation.contains(&format!("NativeResidentAllocationId::{variant}")),
            "stored payload does not preserve diffusion allocation identity {variant}"
        );
    }
    assert_eq!(
        diffusion_translation
            .matches("NativeDiffusionResidentAllocationId::")
            .count(),
        diffusion_allocation_variants.len(),
        "diffusion resident allocation translation is not exhaustive"
    );
    let control_adapter = stored_payload_production
        .split_once("fn control_residency(")
        .and_then(|(_, adapter)| adapter.split_once("fn stored_diffusion_residency("))
        .map(|(adapter, _)| adapter)
        .ok_or("control resident-parts adapter is missing")?;
    for required in [
        "payload.resident_parts()?",
        "parts.owned_bytes()",
        "diffusion_allocations(&parts)?",
    ] {
        assert!(
            control_adapter.contains(required),
            "control resident-parts adapter omits {required}"
        );
    }
    assert!(!control_adapter.contains("single_arc_residency"));
    let stored_diffusion_adapter = stored_payload_production
        .split_once("fn stored_diffusion_residency(")
        .and_then(|(_, adapter)| adapter.split_once("fn model_allocations("))
        .map(|(adapter, _)| adapter)
        .ok_or("stored diffusion resident-parts adapter is missing")?;
    for required in [
        "diffusion.resident_parts()?",
        "NativeResidentAllocationId::DiffusionPayloadArc",
        "parts.owned_bytes()",
        "diffusion_allocations(&parts)?",
    ] {
        assert!(
            stored_diffusion_adapter.contains(required),
            "stored diffusion resident-parts adapter omits {required}"
        );
    }
    assert!(!stored_diffusion_adapter.contains("single_arc_residency"));

    let model_payload =
        fs::read_to_string(root.join("crates/comfy_model/src/native_node_payload.rs"))?;
    let model_payload_production = model_payload
        .split_once("#[cfg(test)]\nmod tests")
        .map_or(model_payload.as_str(), |(production, _)| production);
    assert!(model_payload_production.contains("enum NativeModelResource {"));
    assert!(!model_payload_production.contains("pub enum NativeModelResource {"));
    assert!(model_payload_production.contains("OpticalFlow {"));
    assert!(model_payload_production.contains(
        "pub fn optical_flow(raft: Arc<NativeRaftLarge>) -> Result<Self, NativeModelPayloadError>"
    ));
    assert!(model_payload_production.contains("Self::OpticalFlow => \"OPTICAL_FLOW\""));
    assert!(model_payload_production.contains("NativeModelBackingKind::OpticalFlow"));
    assert!(model_payload_production.contains("Arc::as_ptr(raft) as usize"));
    assert!(model_payload_production.contains("ClipVision {"));
    assert!(model_payload_production.contains("pub fn clip_vision("));
    assert!(model_payload_production.contains("clip_vision: Arc<NativeClipVision>"));
    assert!(model_payload_production.contains("Self::ClipVision => \"CLIP_VISION\""));
    assert!(model_payload_production.contains("NativeModelBackingKind::ClipVision"));
    assert!(model_payload_production.contains("Arc::as_ptr(clip_vision) as usize"));
    assert!(model_payload_production.contains("pub fn clip_vision_resource("));

    let source_types = fs::read_to_string(root.join("crates/comfy_nodes/src/source_type.rs"))?;
    assert!(source_types.contains(
        "\"OPTICAL_FLOW\" => handle!(\"OPTICAL_FLOW\", Compute, Model, \"optical_flow\")"
    ));
    assert!(
        source_types.contains(
            "\"CLIP_VISION\" => handle!(\"CLIP_VISION\", Compute, Clip, \"clip_vision\")"
        )
    );
    assert!(source_types.contains("\"CLIP_VISION_OUTPUT\" => handle!("));
    assert!(source_types.contains("StructuredCompute,\n            \"clip_vision_output\""));
    for forbidden in [
        "handle!(\"OPTICAL_FLOW\", Compute, Tensor",
        "handle!(\"OPTICAL_FLOW\", Provider",
        "NativeStoredPayload::OpticalFlow",
        "OpticalFlow(Arc<NativeProviderPayload>)",
        "OpticalFlow(Arc<NativeTensorPayload>)",
    ] {
        assert!(
            !stored_payload_production.contains(forbidden) && !source_types.contains(forbidden),
            "OPTICAL_FLOW has a non-canonical stored fallback: {forbidden}"
        );
    }
    for forbidden in [
        "handle!(\"CLIP_VISION\", Compute, Tensor",
        "handle!(\"CLIP_VISION\", Compute, StructuredCompute",
        "handle!(\"CLIP_VISION\", Provider",
        "NativeStoredPayload::ClipVision(",
        "ClipVision(Arc<NativeProviderPayload>)",
        "ClipVision(Arc<NativeTensorPayload>)",
        "ClipVision(Arc<NativeStructuredComputePayload>)",
        "ClipVisionOutput(Arc<NativeProviderPayload>)",
        "ClipVisionOutput(Arc<NativeTensorPayload>)",
        "ClipVisionOutput(Arc<NativeStructuredComputePayload>)",
    ] {
        assert!(
            !stored_payload_production.contains(forbidden) && !source_types.contains(forbidden),
            "CLIP_VISION has a non-canonical stored fallback: {forbidden}"
        );
    }

    let nodes_root = fs::read_to_string(root.join("crates/comfy_nodes/src/comfy_nodes.rs"))?;
    let runtime_root = fs::read_to_string(root.join("crates/comfy_runtime/src/comfy_runtime.rs"))?;
    assert!(nodes_root.contains("NativeStoredPayload"));
    assert!(runtime_root.contains("NativeStoredPayload"));
    for forbidden in [
        "NativeStructuredComputePayload",
        "NativeStructuredComputeRole",
        "NativeStructuredComputeValue",
    ] {
        assert!(!nodes_root.contains(forbidden));
        assert!(!runtime_root.contains(forbidden));
    }
    Ok(true)
}

fn validate_native_shader_execution_boundary(
    root: &Path,
    sources: &[(PathBuf, String)],
) -> Result<bool, Box<dyn std::error::Error>> {
    for (definition, owner) in [
        (
            "pub struct NativeShaderRequest {",
            "crates/comfy_tensor/src/shader.rs",
        ),
        (
            "pub struct NativeShaderResult {",
            "crates/comfy_tensor/src/shader.rs",
        ),
        (
            "pub enum NativeShaderError {",
            "crates/comfy_tensor/src/shader.rs",
        ),
        (
            "pub trait NativeShaderExecutor:",
            "crates/comfy_tensor/src/shader.rs",
        ),
        (
            "pub struct WgpuNativeShaderExecutor {",
            "crates/comfy_tensor/src/shader.rs",
        ),
        (
            "pub enum NativeShaderServiceError {",
            "crates/comfy_nodes/src/execution.rs",
        ),
        (
            "pub enum NativeShaderPreviewError {",
            "crates/comfy_nodes/src/execution.rs",
        ),
        (
            "pub struct NativePreparedShaderResult {",
            "crates/comfy_nodes/src/execution.rs",
        ),
    ] {
        let occurrences = production_source_occurrences(sources, definition);
        assert_eq!(
            occurrences.len(),
            1,
            "native shader boundary {definition} must have one production definition"
        );
        assert!(
            occurrences[0].contains(owner),
            "native shader boundary {definition} must be owned by {owner}: {occurrences:?}"
        );
    }

    for (path, source) in sources {
        let path = path.to_string_lossy();
        if path.contains("/crates/comfy_nodes/src/families/")
            || path.contains("/crates/comfy_runtime/src/")
        {
            let production = source
                .split_once("#[cfg(test)]")
                .map_or(source.as_str(), |(production, _)| production);
            assert!(
                !production.contains("create_shader_module(")
                    && !production.contains("wgpu::Device")
                    && !production.contains("naga::front"),
                "native shader backend ownership leaked into {path}"
            );
        }
    }

    let workspace_manifest = fs::read_to_string(root.join("Cargo.toml"))?;
    let tensor_manifest = fs::read_to_string(root.join("crates/comfy_tensor/Cargo.toml"))?;
    let tensor_root = fs::read_to_string(root.join("crates/comfy_tensor/src/comfy_tensor.rs"))?;
    let shader = fs::read_to_string(root.join("crates/comfy_tensor/src/shader.rs"))?;
    let png = fs::read_to_string(root.join("crates/comfy_media/src/png.rs"))?;
    let execution = fs::read_to_string(root.join("crates/comfy_nodes/src/execution.rs"))?;
    let executor = fs::read_to_string(root.join("crates/comfy_runtime/src/executor.rs"))?;
    let controller =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_execution_controller.rs"))?;
    let policy = fs::read_to_string(root.join(".agents/specs/comfy-parity/ownership-policy.json"))?;
    let tasks = fs::read_to_string(root.join(".agents/specs/comfy-parity/tasks.md"))?;

    for required in [
        "naga = { git = \"https://github.com/simtropolis/wgpu.git\"",
        "wgpu = { git = \"https://github.com/simtropolis/wgpu.git\"",
    ] {
        assert!(
            workspace_manifest.contains(required),
            "workspace shader dependency is not pinned: {required}"
        );
    }
    for required in [
        "if !matches!(channels, 3 | 4)",
        "ExtendedColorType::Rgba8",
        "native_png_encoding_preserves_rgba_preview_alpha",
    ] {
        assert!(
            png.contains(required),
            "native shader preview PNG projection lacks {required}"
        );
    }
    for required in [
        "naga.workspace = true",
        "pollster.workspace = true",
        "wgpu = { workspace = true, features = [\"glsl\"] }",
    ] {
        assert!(
            tensor_manifest.contains(required),
            "comfy_tensor shader dependency is missing: {required}"
        );
    }
    for required in ["pub mod shader;", "pub use shader::*;"] {
        assert!(
            tensor_root.contains(required),
            "comfy_tensor shader export is missing: {required}"
        );
    }
    for required in [
        "pub const MAX_SHADER_IMAGES: usize = 5;",
        "pub const MAX_SHADER_OUTPUTS: usize = 4;",
        "fn lower_es_300_source(",
        "glsl::Frontend::default()",
        "wgpu::TextureFormat::Rgba32Float",
        "#pragma passes ",
        "wait_for_submission(",
        "NativeShaderError::BackendUnavailable",
    ] {
        assert!(
            shader.contains(required),
            "canonical native shader owner lacks {required}"
        );
    }
    for required in [
        "shader: Option<Arc<dyn NativeShaderExecutor>>",
        "pub fn with_shader(",
        "pub fn execute_shader(",
        "pub fn execute_shader_with_previews(",
        "let compute = self.compute_session()?;",
        "let execution_context = compute.execution_context(self)?;",
        "json!({\"input_images\": input_images, \"images\": output_images})",
    ] {
        assert!(
            execution.contains(required),
            "portable shader service boundary lacks {required}"
        );
    }
    for required in [
        "shader_executor: Option<Arc<dyn NativeShaderExecutor>>",
        "pub fn with_shader_executor(",
        "services = services.with_shader(shader.clone())",
    ] {
        assert!(
            executor.contains(required),
            "runtime shader injection lacks {required}"
        );
    }
    for required in [
        "WgpuNativeShaderExecutor::new_or_unavailable()",
        "self.shader_executor.configuration_identity()",
        ".with_shader_executor(self.shader_executor.clone())",
    ] {
        assert!(
            controller.contains(required),
            "native controller shader lifecycle lacks {required}"
        );
    }
    assert!(policy.contains("comfy-parity-native-shader-execution-foundation"));
    assert!(policy.contains("native-shader-owner-to-attempt-compute-service"));
    assert!(policy.contains("native-shader-result-to-transactional-preview"));
    assert!(tasks.contains("comfy-parity-native-shader-execution-foundation"));
    Ok(true)
}

#[test]
fn val_ownership_001_native_shader_execution_has_one_injected_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    assert!(validate_native_shader_execution_boundary(&root, &sources)?);
    Ok(())
}

fn validate_native_structured_input_boundary(
    root: &Path,
    sources: &[(PathBuf, String)],
) -> Result<bool, Box<dyn std::error::Error>> {
    let definitions = [
        (
            "pub struct NativeStructuredValue {",
            "crates/comfy_nodes/src/execution.rs",
        ),
        (
            "fn structured_option_from_expression(",
            "crates/comfy_nodes/src/descriptor.rs",
        ),
        (
            "fn resolve_active_structured_input(",
            "crates/comfy_runtime/src/prompt_compiler.rs",
        ),
        (
            "fn assemble_structured_inputs(",
            "crates/comfy_runtime/src/executor.rs",
        ),
    ];
    for (needle, expected_path) in definitions {
        let occurrences = production_source_occurrences(sources, needle);
        assert_eq!(
            occurrences.len(),
            1,
            "structured-input owner {needle} must have one production definition"
        );
        assert!(
            occurrences[0].contains(expected_path),
            "structured-input owner {needle} is declared at {}",
            occurrences[0]
        );
    }

    for (path, source) in sources {
        let path = path.to_string_lossy();
        if path.contains("crates/comfy_nodes/src/families/") {
            let production = source
                .split_once("#[cfg(test)]")
                .map_or(source.as_str(), |(production, _)| production);
            assert!(
                !production.contains("decode_link")
                    && !production.contains("[source_node, output_index]")
                    && !production.contains("value.as_array()"),
                "native family {path} implements a private structured-link decoder"
            );
        }
    }

    let descriptor = fs::read_to_string(root.join("crates/comfy_nodes/src/descriptor.rs"))?;
    let execution = fs::read_to_string(root.join("crates/comfy_nodes/src/execution.rs"))?;
    let compiler = fs::read_to_string(root.join("crates/comfy_runtime/src/prompt_compiler.rs"))?;
    let executor = fs::read_to_string(root.join("crates/comfy_runtime/src/executor.rs"))?;
    let persistence = fs::read_to_string(root.join("crates/comfy_runtime/src/persistence.rs"))?;
    Ok(descriptor.contains("pub fn structured_options(")
        && descriptor.contains("DynamicCombo.Option")
        && descriptor.contains("MultiType.Input")
        && execution.contains("sim.native-structured-value@1")
        && execution.contains("NativeStructuredValue::from_native_value(value)")
        && compiler.contains("resolve_active_structured_input(")
        && compiler.contains("decode_link(value)")
        && compiler.contains("validate_compiled_structured_inputs(node)?")
        && executor.contains("assemble_structured_inputs(&node.descriptor, &inputs)?")
        && executor.contains("collect_native_value_handles(value, handles)")
        && persistence.contains("plan.validate_integrity()"))
}

#[test]
fn val_ownership_001_native_structured_input_links_have_one_checked_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    assert!(validate_native_structured_input_boundary(&root, &sources)?);
    Ok(())
}

#[test]
fn val_ownership_001_native_stored_payload_boundary_is_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    assert!(validate_native_stored_payload_boundary(&root, &sources)?);
    Ok(())
}

fn validate_native_text_regex_boundary(
    root: &Path,
    sources: &[(PathBuf, String)],
) -> Result<bool, Box<dyn std::error::Error>> {
    for definition in [
        "pub struct NativeTextRegex {",
        "pub struct NativeTextRegexCaptureRows {",
        "pub enum NativeTextRegexError {",
        "pub struct NativeTextRegexFlags {",
    ] {
        let occurrences = production_source_occurrences(sources, definition);
        assert_eq!(
            occurrences.len(),
            1,
            "{definition} must have exactly one production owner: {occurrences:?}"
        );
        assert!(
            occurrences[0].contains("crates/comfy_nodes/src/text_regex.rs"),
            "{definition} must be owned by comfy_nodes text_regex: {occurrences:?}"
        );
    }
    for definition in [
        "pub struct NativeTextFormatter;",
        "pub enum NativeTextFormatError {",
    ] {
        let occurrences = production_source_occurrences(sources, definition);
        assert_eq!(
            occurrences.len(),
            1,
            "{definition} must have exactly one production owner: {occurrences:?}"
        );
        assert!(
            occurrences[0].contains("crates/comfy_nodes/src/text_format.rs"),
            "{definition} must be owned by comfy_nodes text_format: {occurrences:?}"
        );
    }
    let regex_import = ["use fancy_", "regex"].concat();
    let regex_engine_uses = sources
        .iter()
        .filter(|(path, source)| {
            path.to_string_lossy().contains("/crates/comfy_") && source.contains(&regex_import)
        })
        .map(|(path, _)| path.display().to_string())
        .collect::<Vec<_>>();
    assert!(
        regex_engine_uses
            .iter()
            .all(|path| path.contains("crates/comfy_nodes/src/text_regex.rs")),
        "native Comfy code contains a second regex-engine owner: {regex_engine_uses:?}"
    );
    let source = fs::read_to_string(root.join("crates/comfy_nodes/src/text_regex.rs"))?;
    for required in [
        "RegexBuilder::new(pattern)",
        ".backtrack_limit(limits.backtrack_limit)",
        ".delegate_size_limit(NATIVE_TEXT_REGEX_DELEGATE_SIZE_LIMIT)",
        ".delegate_dfa_size_limit(NATIVE_TEXT_REGEX_DELEGATE_DFA_SIZE_LIMIT)",
        "maximum_input_bytes",
        "maximum_matches",
        "maximum_capture_bytes",
        "NativeTextRegexReplacement::checked(replacement, &self.regex)?",
        "append_bounded(",
        "self.check_cancellation(cancellation)?",
    ] {
        assert!(
            source.contains(required),
            "native text regex lacks {required}"
        );
    }
    let format_source = fs::read_to_string(root.join("crates/comfy_nodes/src/text_format.rs"))?;
    for required in [
        "pub struct NativeTextFormatter;",
        "pub fn format(",
        "resolve_field(parsed.field_name, values)?",
        "render_template(parsed.format_spec, values, cancellation, depth + 1)?",
        "NATIVE_TEXT_FORMAT_MAX_TEMPLATE_BYTES",
        "NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES",
        "check_cancellation(cancellation)?",
    ] {
        assert!(
            format_source.contains(required),
            "native text formatter lacks {required}"
        );
    }
    let manifest = fs::read_to_string(root.join("crates/comfy_nodes/Cargo.toml"))?;
    assert!(manifest.contains("fancy-regex.workspace = true"));
    let root_source = fs::read_to_string(root.join("crates/comfy_nodes/src/comfy_nodes.rs"))?;
    assert!(root_source.contains("pub mod text_regex;"));
    assert!(root_source.contains("pub mod text_format;"));
    assert!(root_source.contains("NativeTextRegexCaptureRows"));
    assert!(root_source.contains("NativeTextFormatter"));
    let execution = fs::read_to_string(root.join("crates/comfy_nodes/src/execution.rs"))?;
    assert!(
        execution
            .contains("Self::String(value) => validate_workflow_text(\"primitive string\", value)")
    );
    assert!(execution.contains("fn validate_workflow_text("));
    assert!(execution.contains("'\\n' | '\\r' | '\\t'"));
    assert!(execution.contains("fn validate_identifier("));
    let policy = fs::read_to_string(root.join(".agents/specs/comfy-parity/ownership-policy.json"))?;
    assert!(policy.contains("comfy-parity-native-text-transform-foundation"));
    assert!(policy.contains("canonical-text-transforms-to-generated-text-leaves"));
    assert!(policy.contains(r"regex\\s*\\.replace\\("));
    Ok(true)
}

fn validate_native_image_source_compatibility_boundary(
    root: &Path,
    sources: &[(PathBuf, String)],
) -> Result<bool, Box<dyn std::error::Error>> {
    for (definition, owner) in [
        (
            "pub struct NumpyRandomState {",
            "crates/comfy_tensor/src/rng.rs",
        ),
        (
            "pub enum NativeImageDither {",
            "crates/comfy_media/src/image_quantization.rs",
        ),
        (
            "pub enum NativeImageQuantizationError {",
            "crates/comfy_media/src/image_quantization.rs",
        ),
        (
            "pub struct NativePreparedImagePreview {",
            "crates/comfy_nodes/src/execution.rs",
        ),
        (
            "pub enum NativeImagePreviewError {",
            "crates/comfy_nodes/src/execution.rs",
        ),
    ] {
        let occurrences = production_source_occurrences(sources, definition);
        assert_eq!(
            occurrences.len(),
            1,
            "{definition} must have exactly one production owner: {occurrences:?}"
        );
        assert!(
            occurrences[0].contains(owner),
            "{definition} must be owned by {owner}: {occurrences:?}"
        );
    }
    let image_ops = fs::read_to_string(root.join("crates/comfy_tensor/src/image_ops.rs"))?;
    for required in [
        "pub fn source_compatible_u8_crop(",
        "(value * 255.0).trunc() as i64 as u8",
        "f32::from(quantized) / 255.0",
    ] {
        assert!(
            image_ops.contains(required),
            "source-compatible crop lacks {required}"
        );
    }
    let rng = fs::read_to_string(root.join("crates/comfy_tensor/src/rng.rs"))?;
    for required in [
        "pub struct NumpyRandomState {",
        "seed % u64::from(u32::MAX)",
        "pub fn randint(",
        "let mut mask = maximum;",
        "self.generator.next_u32() & mask",
    ] {
        assert!(rng.contains(required), "NumPy RandomState lacks {required}");
    }
    let quantization =
        fs::read_to_string(root.join("crates/comfy_media/src/image_quantization.rs"))?;
    for required in [
        "pub fn quantize_image_tensor(",
        "pub fn quantize_rgb8(",
        "fn adaptive_palette(",
        "NativeImageDither::FloydSteinberg",
        "NativeImageDither::Bayer16",
        "check_cancel(cancellation)?",
    ] {
        assert!(
            quantization.contains(required),
            "native image quantization lacks {required}"
        );
    }
    let execution = fs::read_to_string(root.join("crates/comfy_nodes/src/execution.rs"))?;
    for required in [
        "pub fn prepare_image_preview(",
        "encode_png_frame_with_policy_and_context(",
        "NativeOutputNamespace::Temporary",
        ".prepare_output(request, &self.cancellation)",
        "effects_service.rollback_prepared(effect)?",
        "json!({\"images\": ui_images, \"animated\": [false]})",
    ] {
        assert!(
            execution.contains(required),
            "portable image preview lacks {required}"
        );
    }
    let runtime =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_execution_controller.rs"))?;
    assert!(runtime.contains(".prepare_image_preview(&image, \"ComfyUI_temp\")"));
    assert!(runtime.contains("let (effects, ui) = preview.into_parts();"));
    let policy: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/specs/comfy-parity/ownership-policy.json"),
    )?)?;
    let concern = policy
        .get("concerns")
        .and_then(serde_json::Value::as_array)
        .and_then(|concerns| {
            concerns.iter().find(|concern| {
                concern.get("concern").and_then(serde_json::Value::as_str)
                    == Some("native_rgb8_workflow_source_compatibility")
            })
        })
        .ok_or("missing native image source compatibility ownership concern")?;
    let mappings = concern
        .get("required_mappings")
        .and_then(serde_json::Value::as_array)
        .ok_or("missing native image compatibility mappings")?;
    assert_eq!(mappings.len(), 5);
    Ok(true)
}

#[test]
fn val_ownership_001_native_image_source_compatibility_has_one_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    assert!(validate_native_image_source_compatibility_boundary(
        &root, &sources
    )?);
    Ok(())
}

#[test]
fn val_ownership_001_native_text_regex_has_one_bounded_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let sources = rust_sources(&root)?
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)?;
            Ok((path, source))
        })
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    assert!(validate_native_text_regex_boundary(&root, &sources)?);
    Ok(())
}

#[test]
fn val_ownership_001_native_decoder_text_generation_has_one_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root()?;
    let decoder =
        fs::read_to_string(root.join("crates/comfy_model/src/clip_text_encoder_decoder.rs"))?;
    for required in [
        "pub fn generate_text(",
        "tokenizer.encode_numeric(",
        ".get(prompt_length..)",
        "tokenizer.decode_numeric(",
        "let mut staged_transaction = transaction.clone();",
    ] {
        assert!(
            decoder.contains(required),
            "decoder boundary lacks {required}"
        );
    }
    let nodes = fs::read_to_string(root.join("crates/comfy_nodes/src/execution.rs"))?;
    for required in [
        "pub const NATIVE_TEXT_GENERATION_RNG_PHASE",
        "pub fn native_text_generation_transaction(",
        "RngAlgorithm::Philox4x32_10",
        "RetryRngPolicy::Replay",
    ] {
        assert!(
            nodes.contains(required),
            "node RNG boundary lacks {required}"
        );
    }
    let runtime =
        fs::read_to_string(root.join("crates/comfy_runtime/src/native_execution_controller.rs"))?;
    for required in [
        "pub struct NativeDecoderClipResource",
        "pub fn resolve_native_decoder_clip(",
        "pub fn execute_native_decoder_text_generation(",
        "payload.diffusion().is_some()",
        "native_text_generation_transaction(context, seed)",
        "decoder_clip_resource()",
        "_resolved_payload: stored",
    ] {
        assert!(
            runtime.contains(required),
            "runtime boundary lacks {required}"
        );
    }
    let policy: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/specs/comfy-parity/ownership-policy.json"),
    )?)?;
    let concern = policy
        .get("concerns")
        .and_then(serde_json::Value::as_array)
        .and_then(|concerns| {
            concerns.iter().find(|concern| {
                concern.get("concern").and_then(serde_json::Value::as_str)
                    == Some("native_vision_text_transformer_unidirectional_decoder_execution")
            })
        })
        .ok_or("missing decoder text-generation ownership concern")?;
    let mappings = concern
        .get("required_mappings")
        .and_then(serde_json::Value::as_array)
        .ok_or("missing decoder text-generation ownership mappings")?;
    for name in [
        "text-generation-opens-one-attempt-addressed-rng-transaction",
        "decoder-generation-decodes-only-the-generated-suffix",
        "decoder-clip-resource-is-concrete-derived-and-retained",
        "runtime-decoder-clip-resolver-rejects-diffusion-clip-and-retains-lease",
        "runtime-text-generation-stages-resolution-rng-and-generated-text",
    ] {
        assert!(mappings.iter().any(|mapping| {
            mapping.get("name").and_then(serde_json::Value::as_str) == Some(name)
        }));
    }
    Ok(())
}
