use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use comfy_runtime::{
    AssetNamespace, AssetRoots, AssetService, NativeAssetResolverRegistry,
    NativeModelSourceTestSession, NativeModelSourceTestTransport, NativeModelSourceTransportHost,
    PermissionPolicy, RuntimeSupervisor, RuntimeSupervisorError, SupervisorPolicy,
    WorkerLaunchConfig, authorize_native_api_asset_reader,
};
use comfy_tensor::{
    CancellationToken, CpuWorkspaceAuthority, DType, DeviceId, StreamId, TensorBackend,
    TensorDescriptor, TensorError,
};
use comfy_types::{
    AttemptId, MAX_WORKER_MODEL_SOURCE_CHUNK_BYTES, ProfileId, WorkerId, WorkerModelSourceContext,
    WorkerModelSourceError, WorkerModelSourceFormat, WorkerModelSourceOperation,
    WorkerModelSourceRequest, WorkerModelSourceResponse, worker_model_source_selection_sha256,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use smol::process::{Child, Command};
use uuid::Uuid;

#[path = "support/native_controller.rs"]
mod native_controller;

const MEMORY_LIMIT_BYTES: u64 = 1024 * 1024 * 1024;

fn bridge_model_source_call(
    host: &NativeModelSourceTransportHost,
    session: &NativeModelSourceTestSession,
    registry: &NativeAssetResolverRegistry,
    expected_context: &WorkerModelSourceContext,
    request: WorkerModelSourceRequest,
    cancellation: &CancellationToken,
) -> Result<WorkerModelSourceResponse, Box<dyn Error>> {
    thread::scope(|scope| {
        let worker_call = scope.spawn(|| session.call(request, cancellation));
        let call = smol::block_on(host.receive())?;
        let call_id = call.call_id();
        let response = registry
            .test_serve_model_source_request(expected_context, call.request(), cancellation)
            .or_else(|error| {
                WorkerModelSourceResponse::rejected(
                    call.request().context.session_id,
                    call.request().call_ordinal,
                    error,
                )
            })?;
        smol::block_on(call.respond(call_id, response))?;
        worker_call
            .join()
            .map_err(|_| "model-source worker call panicked")?
            .map_err(Into::into)
    })
}

#[test]
fn val_recovery_005() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let profile_id = "model-source-restart";
    let roots = [
        AssetNamespace::Input,
        AssetNamespace::Output,
        AssetNamespace::Temporary,
        AssetNamespace::Model,
        AssetNamespace::Plugin,
    ]
    .into_iter()
    .map(|namespace| {
        let root = directory.path().join(namespace.locator_type());
        fs::create_dir_all(&root)?;
        Ok((namespace, root))
    })
    .collect::<Result<Vec<_>, std::io::Error>>()?;
    let roots = AssetRoots::new(profile_id, roots)?;
    let model_path = roots
        .test_root_path(AssetNamespace::Model)?
        .join("checkpoints/large.safetensors");
    let model_parent = model_path.parent().ok_or("model parent is unavailable")?;
    fs::create_dir_all(model_parent)?;
    let tensor_bytes = 13 * 1024 * 1024_usize;
    let header = format!(
        r#"{{"large":{{"dtype":"U8","shape":[{tensor_bytes}],"data_offsets":[0,{tensor_bytes}]}}}}"#
    );
    let mut model_file = fs::File::create(&model_path)?;
    model_file.write_all(&u64::try_from(header.len())?.to_le_bytes())?;
    model_file.write_all(header.as_bytes())?;
    model_file.write_all(&vec![0x5a; tensor_bytes])?;
    model_file.sync_all()?;

    let policy = PermissionPolicy::native_runtime_services(profile_id)?;
    let authorization = authorize_native_api_asset_reader(&policy)?;
    let assets = Arc::new(Mutex::new(AssetService::open(roots)?));
    let registry = NativeAssetResolverRegistry::new(assets, authorization);
    let attempt_id = AttemptId(Uuid::from_u128(0x39921));
    let source_names = vec!["large.safetensors".to_owned()];
    let selection = worker_model_source_selection_sha256("checkpoints", &source_names)?;
    let context = |session_id, service_generation| WorkerModelSourceContext {
        session_id,
        attempt_id,
        attempt_generation: 1,
        node_id: "checkpoint-loader".to_owned(),
        node_generation: 1,
        service_id: Uuid::from_u128(0x39922),
        service_generation,
        ordered_source_identity_sha256: selection.clone(),
    };
    let open_request = |context: WorkerModelSourceContext| WorkerModelSourceRequest {
        context,
        call_ordinal: 1,
        operation: WorkerModelSourceOperation::Open {
            folder_category: "checkpoints".to_owned(),
            source_names: source_names.clone(),
        },
    };
    let cancellation = CancellationToken::default();
    let (transport, host) = NativeModelSourceTestTransport::channel();

    let lost_context = context(Uuid::from_u128(0x39923), 1);
    let lost_session = transport.open_session(lost_context.clone())?;
    let lost_open = open_request(lost_context.clone());
    let opened = bridge_model_source_call(
        &host,
        &lost_session,
        &registry,
        &lost_context,
        lost_open,
        &cancellation,
    )?;
    let WorkerModelSourceResponse::Opened(opened) = opened else {
        return Err("model-source open did not return a manifest".into());
    };
    assert_eq!(opened.sources.len(), 1);
    assert_eq!(
        opened.sources[0].aggregate_tensor_bytes,
        u64::try_from(tensor_bytes)?
    );
    assert!(
        !serde_json::to_vec(&opened)?
            .windows(directory.path().as_os_str().len())
            .any(|window| window == directory.path().as_os_str().as_encoded_bytes())
    );
    assert!(tensor_bytes > 12 * 1024 * 1024);
    assert!(
        serde_json::to_vec(&open_request(lost_context.clone()))?.len()
            < comfy_types::MAX_WORKER_FRAME_BYTES
    );
    let first_read = WorkerModelSourceRequest {
        context: lost_context.clone(),
        call_ordinal: 2,
        operation: WorkerModelSourceOperation::Read {
            source_ordinal: 0,
            tensor_ordinal: 0,
            byte_offset: 0,
            byte_length: u32::try_from(MAX_WORKER_MODEL_SOURCE_CHUNK_BYTES)?,
        },
    };
    assert!(matches!(
        bridge_model_source_call(
            &host,
            &lost_session,
            &registry,
            &lost_context,
            first_read.clone(),
            &cancellation,
        )?,
        WorkerModelSourceResponse::Chunk(_)
    ));

    registry.test_replace_model_source_service();
    let late_read = WorkerModelSourceRequest {
        call_ordinal: 3,
        ..first_read
    };
    assert!(matches!(
        bridge_model_source_call(
            &host,
            &lost_session,
            &registry,
            &lost_context,
            late_read,
            &cancellation,
        )?,
        WorkerModelSourceResponse::Rejected {
            error: WorkerModelSourceError::Closed,
            ..
        }
    ));

    let retry_context = context(Uuid::from_u128(0x39924), 2);
    let retry_session = transport.open_session(retry_context.clone())?;
    let retry_open = open_request(retry_context.clone());
    assert!(matches!(
        bridge_model_source_call(
            &host,
            &retry_session,
            &registry,
            &retry_context,
            retry_open,
            &cancellation,
        )?,
        WorkerModelSourceResponse::Opened(_)
    ));
    let mut bridged_bytes = 0_usize;
    let chunk_count = tensor_bytes / MAX_WORKER_MODEL_SOURCE_CHUNK_BYTES;
    for chunk_index in 0..chunk_count {
        let request = WorkerModelSourceRequest {
            context: retry_context.clone(),
            call_ordinal: u64::try_from(chunk_index)? + 2,
            operation: WorkerModelSourceOperation::Read {
                source_ordinal: 0,
                tensor_ordinal: 0,
                byte_offset: u64::try_from(chunk_index * MAX_WORKER_MODEL_SOURCE_CHUNK_BYTES)?,
                byte_length: u32::try_from(MAX_WORKER_MODEL_SOURCE_CHUNK_BYTES)?,
            },
        };
        let WorkerModelSourceResponse::Chunk(chunk) = bridge_model_source_call(
            &host,
            &retry_session,
            &registry,
            &retry_context,
            request,
            &cancellation,
        )?
        else {
            return Err("model-source read did not return a chunk".into());
        };
        assert_eq!(chunk.bytes, vec![0x5a; MAX_WORKER_MODEL_SOURCE_CHUNK_BYTES]);
        bridged_bytes = bridged_bytes
            .checked_add(chunk.bytes.len())
            .ok_or("bridged byte count overflowed")?;
    }
    assert_eq!(bridged_bytes, tensor_bytes);
    let close_ordinal = u64::try_from(chunk_count)? + 2;
    let close = WorkerModelSourceRequest {
        context: retry_context.clone(),
        call_ordinal: close_ordinal,
        operation: WorkerModelSourceOperation::Close,
    };
    assert!(matches!(
        bridge_model_source_call(
            &host,
            &retry_session,
            &registry,
            &retry_context,
            close,
            &cancellation,
        )?,
        WorkerModelSourceResponse::Closed(_)
    ));
    registry.retire_attempt(attempt_id);
    Ok(())
}

#[test]
fn model_source_worker_bridge_restarts_without_duplicate_publication() -> Result<(), Box<dyn Error>>
{
    val_recovery_005()
}

#[test]
fn model_source_auxiliary_artifact_stream_restarts_atomically() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let profile_id = "model-source-artifact-restart";
    let roots = [
        AssetNamespace::Input,
        AssetNamespace::Output,
        AssetNamespace::Temporary,
        AssetNamespace::Model,
        AssetNamespace::Plugin,
    ]
    .into_iter()
    .map(|namespace| {
        let root = directory.path().join(namespace.locator_type());
        fs::create_dir_all(&root)?;
        Ok((namespace, root))
    })
    .collect::<Result<Vec<_>, std::io::Error>>()?;
    let roots = AssetRoots::new(profile_id, roots)?;
    let model_root = roots
        .test_root_path(AssetNamespace::Model)?
        .join("checkpoints");
    fs::create_dir_all(&model_root)?;
    let shard_header = r#"{"weight":{"dtype":"U8","shape":[4],"data_offsets":[0,4]}}"#;
    let mut shard = fs::File::create(model_root.join("weights.safetensors"))?;
    shard.write_all(&u64::try_from(shard_header.len())?.to_le_bytes())?;
    shard.write_all(shard_header.as_bytes())?;
    shard.write_all(&[1, 2, 3, 4])?;
    shard.sync_all()?;
    let metadata = (0..40_000)
        .map(|ordinal| {
            (
                format!("token-{ordinal:05}"),
                serde_json::Value::String(format!("fixture-{ordinal:05}")),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let config = serde_json::to_vec(&json!({
        "metadata": metadata,
        "weight_map": { "weight": "weights.safetensors" }
    }))?;
    assert!(config.len() > MAX_WORKER_MODEL_SOURCE_CHUNK_BYTES);
    fs::write(model_root.join("model.safetensors.index.json"), &config)?;

    let policy = PermissionPolicy::native_runtime_services(profile_id)?;
    let authorization = authorize_native_api_asset_reader(&policy)?;
    let assets = Arc::new(Mutex::new(AssetService::open(roots)?));
    let registry = NativeAssetResolverRegistry::new(assets, authorization);
    let attempt_id = AttemptId(Uuid::from_u128(0x73321));
    let source_names = vec!["model.safetensors.index.json".to_owned()];
    let selection = worker_model_source_selection_sha256("checkpoints", &source_names)?;
    let context = |session_id, service_generation| WorkerModelSourceContext {
        session_id,
        attempt_id,
        attempt_generation: 1,
        node_id: "checkpoint-config-loader".to_owned(),
        node_generation: 1,
        service_id: Uuid::from_u128(0x73322),
        service_generation,
        ordered_source_identity_sha256: selection.clone(),
    };
    let open_request = |context: WorkerModelSourceContext| WorkerModelSourceRequest {
        context,
        call_ordinal: 1,
        operation: WorkerModelSourceOperation::Open {
            folder_category: "checkpoints".to_owned(),
            source_names: source_names.clone(),
        },
    };
    let cancellation = CancellationToken::default();
    let (transport, host) = NativeModelSourceTestTransport::channel();

    let lost_context = context(Uuid::from_u128(0x73323), 1);
    let lost_session = transport.open_session(lost_context.clone())?;
    let lost_open_response = bridge_model_source_call(
        &host,
        &lost_session,
        &registry,
        &lost_context,
        open_request(lost_context.clone()),
        &cancellation,
    )?;
    let WorkerModelSourceResponse::Opened(lost_opened) = lost_open_response else {
        return Err(format!(
            "model-source artifact open did not return a manifest: {lost_open_response:?}"
        )
        .into());
    };
    let lost_source = lost_opened
        .sources
        .first()
        .ok_or("model-source artifact manifest is empty")?;
    let lost_artifact = lost_source
        .artifacts
        .first()
        .ok_or("model-source config artifact is missing")?
        .clone();
    assert_eq!(lost_artifact.format, WorkerModelSourceFormat::JsonConfig);
    assert_eq!(
        lost_source.artifacts.get(1).map(|artifact| artifact.format),
        Some(WorkerModelSourceFormat::Safetensors)
    );
    assert_eq!(lost_source.tensors.len(), 1);
    assert_eq!(lost_source.aggregate_tensor_bytes, 4);
    let lost_tensor = lost_source
        .tensors
        .first()
        .ok_or("model-source shard tensor is missing")?;
    assert_eq!(lost_tensor.artifact_ordinal, 1);
    assert_eq!(lost_tensor.byte_length, 4);
    assert_eq!(lost_source.model_identity_sha256.as_str().len(), 64);
    assert!(!lost_source.model_identity_sha256.as_str().contains(':'));
    assert_ne!(lost_source.model_identity_sha256, lost_artifact.sha256);
    let first_read = WorkerModelSourceRequest {
        context: lost_context.clone(),
        call_ordinal: 2,
        operation: WorkerModelSourceOperation::ReadArtifact {
            source_ordinal: 0,
            artifact_ordinal: 0,
            artifact_sha256: lost_artifact.sha256.clone(),
            byte_offset: 0,
            byte_length: u32::try_from(MAX_WORKER_MODEL_SOURCE_CHUNK_BYTES)?,
        },
    };
    assert!(matches!(
        bridge_model_source_call(
            &host,
            &lost_session,
            &registry,
            &lost_context,
            first_read.clone(),
            &cancellation,
        )?,
        WorkerModelSourceResponse::ArtifactChunk(_)
    ));
    registry.test_replace_model_source_service();
    let late_read = WorkerModelSourceRequest {
        call_ordinal: 3,
        ..first_read
    };
    assert!(matches!(
        bridge_model_source_call(
            &host,
            &lost_session,
            &registry,
            &lost_context,
            late_read,
            &cancellation,
        )?,
        WorkerModelSourceResponse::Rejected {
            error: WorkerModelSourceError::Closed,
            ..
        }
    ));

    let retry_context = context(Uuid::from_u128(0x73324), 2);
    let retry_session = transport.open_session(retry_context.clone())?;
    let retry_open_response = bridge_model_source_call(
        &host,
        &retry_session,
        &registry,
        &retry_context,
        open_request(retry_context.clone()),
        &cancellation,
    )?;
    let WorkerModelSourceResponse::Opened(retry_opened) = retry_open_response else {
        return Err(format!(
            "model-source artifact retry did not return a manifest: {retry_open_response:?}"
        )
        .into());
    };
    let retry_source = retry_opened
        .sources
        .first()
        .ok_or("model-source retry manifest is empty")?;
    let retry_artifact = retry_source
        .artifacts
        .first()
        .ok_or("model-source retry config artifact is missing")?
        .clone();
    assert_eq!(retry_artifact.sha256, lost_artifact.sha256);
    assert_eq!(
        retry_source.model_identity_sha256,
        lost_source.model_identity_sha256
    );
    let mut bridged = Vec::new();
    let mut byte_offset = 0_usize;
    let mut call_ordinal = 2_u64;
    while byte_offset < config.len() {
        let byte_length = (config.len() - byte_offset).min(MAX_WORKER_MODEL_SOURCE_CHUNK_BYTES);
        let request = WorkerModelSourceRequest {
            context: retry_context.clone(),
            call_ordinal,
            operation: WorkerModelSourceOperation::ReadArtifact {
                source_ordinal: 0,
                artifact_ordinal: 0,
                artifact_sha256: retry_artifact.sha256.clone(),
                byte_offset: u64::try_from(byte_offset)?,
                byte_length: u32::try_from(byte_length)?,
            },
        };
        let WorkerModelSourceResponse::ArtifactChunk(chunk) = bridge_model_source_call(
            &host,
            &retry_session,
            &registry,
            &retry_context,
            request,
            &cancellation,
        )?
        else {
            return Err("model-source artifact read did not return a chunk".into());
        };
        assert_eq!(chunk.artifact_sha256, retry_artifact.sha256);
        bridged.extend_from_slice(&chunk.bytes);
        byte_offset = byte_offset
            .checked_add(byte_length)
            .ok_or("artifact bridge offset overflowed")?;
        call_ordinal = call_ordinal
            .checked_add(1)
            .ok_or("artifact bridge call ordinal overflowed")?;
    }
    assert_eq!(bridged, config);
    let close = WorkerModelSourceRequest {
        context: retry_context.clone(),
        call_ordinal,
        operation: WorkerModelSourceOperation::Close,
    };
    assert!(matches!(
        bridge_model_source_call(
            &host,
            &retry_session,
            &registry,
            &retry_context,
            close,
            &cancellation,
        )?,
        WorkerModelSourceResponse::Closed(_)
    ));
    registry.retire_attempt(attempt_id);
    Ok(())
}

#[test]
fn val_recovery_008() -> Result<(), Box<dyn Error>> {
    let mut cases = native_controller::run_native_controller_e2e()?;
    let cancellation = CancellationToken::default();
    cancellation.cancel();
    let (backend, workspace_authority) = CpuWorkspaceAuthority::create_backend(64)?;
    let workspace = workspace_authority.authorize_workspace(64)?;
    let context = backend.execution_context(StreamId::DEFAULT, workspace, &cancellation);
    let descriptor =
        TensorDescriptor::contiguous(vec![16], DType::F32, DeviceId::CPU, StreamId::DEFAULT)?;
    let cancelled_allocation = backend.allocate(descriptor, &context);
    cases.insert(
        "cancelled_device_allocation_releases_memory_before_fence",
        matches!(cancelled_allocation, Err(TensorError::Cancelled))
            && backend.memory_snapshot().current_bytes == 0,
    );
    cases.insert(
        "canonical_cancellation_state_is_monotonic",
        cancellation.is_cancelled() && cancellation.check().is_err(),
    );

    assert_cases("VAL-RECOVERY-008", &cases);
    write_artifact(
        "val-recovery-008.json",
        "VAL-RECOVERY-008",
        "native-worker-controller-cancellation-convergence",
        &cases,
        json!({
            "native_controller_source_sha256": format!(
                "{:x}",
                Sha256::digest(include_bytes!("support/native_controller.rs"))
            ),
            "worker_source_sha256": format!(
                "{:x}",
                Sha256::digest(include_bytes!("../../comfy_worker/src/comfy_worker.rs"))
            )
        }),
    )?;
    Ok(())
}

#[test]
fn val_recovery_009() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let mut foreign = spawn_foreign_process()?;
    let foreign_process_id = foreign.id();
    let before_shutdown = run_orphan_case(directory.path(), "before-shutdown")?;
    let during_shutdown = run_orphan_case(directory.path(), "during-shutdown")?;
    let foreign_untouched = foreign.try_status()?.is_none() && process_exists(foreign_process_id)?;

    let mut supervisor = smol::block_on(RuntimeSupervisor::start(worker_config()))?;
    let replacement_ready = supervisor.worker_process_id().is_some();
    let first_status = smol::block_on(supervisor.terminate())?;
    let mut recovered = smol::block_on(supervisor.recover())?;
    let recovered_once = recovered.worker_process_id().is_some();
    let second_status = smol::block_on(recovered.terminate())?;
    let restart_loop_prevented = matches!(
        smol::block_on(recovered.recover()),
        Err(RuntimeSupervisorError::RecoveryBudgetExhausted { maximum: 1 })
    );

    terminate_foreign_process(&mut foreign)?;
    let cases = BTreeMap::from([
        (
            "parent_crash_before_shutdown_leaves_no_owned_worker",
            before_shutdown,
        ),
        (
            "parent_crash_during_shutdown_leaves_no_owned_worker",
            during_shutdown,
        ),
        (
            "foreign_process_is_never_selected_or_terminated",
            foreign_untouched,
        ),
        (
            "replacement_worker_starts_after_orphan_reconciliation",
            replacement_ready,
        ),
        (
            "bounded_recovery_runs_once_without_restart_loop",
            !first_status.success()
                && recovered_once
                && !second_status.success()
                && restart_loop_prevented,
        ),
    ]);
    assert_cases("VAL-RECOVERY-009", &cases);
    write_artifact(
        "val-recovery-009.json",
        "VAL-RECOVERY-009",
        "native-worker-parent-crash-orphan-containment",
        &cases,
        json!({
            "orphan_parent_fixture_sha256": format!(
                "{:x}",
                Sha256::digest(include_bytes!("../src/bin/comfy_test_worker_fixture.rs"))
            ),
            "process_owner": "RuntimeSupervisor",
            "foreign_process_id_was_distinct": true
        }),
    )?;
    Ok(())
}

fn run_orphan_case(directory: &Path, phase: &str) -> Result<bool, Box<dyn Error>> {
    let pid_file = directory.join(format!("{phase}.pid"));
    let status = smol::block_on(async {
        Command::new(env!("CARGO_BIN_EXE_comfy_test_worker_fixture"))
            .arg("--orphan-parent")
            .arg(env!("CARGO_BIN_EXE_comfy_test_worker_fixture"))
            .arg(&pid_file)
            .arg(phase)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
    })?;
    let worker_process_id = fs::read_to_string(&pid_file)?.trim().parse::<u32>()?;
    Ok(!status.success() && wait_for_process_exit(worker_process_id, Duration::from_secs(5))?)
}

fn worker_config() -> WorkerLaunchConfig {
    let mut config = WorkerLaunchConfig::new(
        env!("CARGO_BIN_EXE_comfy_test_worker_fixture"),
        ProfileId(Uuid::from_u128(0x5349_4d00_0000_0000_0000_0000_0000_09f3)),
        WorkerId(Uuid::from_u128(0x5349_4d00_0000_0000_0000_0000_0000_09f4)),
        "orphan-resilience-v1",
        MEMORY_LIMIT_BYTES,
    );
    config.policy = SupervisorPolicy {
        heartbeat_interval: Duration::from_secs(2),
        missed_heartbeat_limit: 3,
        shutdown_timeout: Duration::from_secs(5),
        ready_timeout: Duration::from_secs(5),
        maximum_automatic_restarts: 1,
        restart_backoff: Duration::from_millis(1),
    };
    config
}

fn wait_for_process_exit(process_id: u32, timeout: Duration) -> Result<bool, Box<dyn Error>> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !process_exists(process_id)? {
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(!process_exists(process_id)?)
}

#[cfg(unix)]
fn process_exists(process_id: u32) -> Result<bool, Box<dyn Error>> {
    Ok(smol::block_on(async {
        Command::new("/bin/kill")
            .args(["-0", &process_id.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
    })?
    .success())
}

#[cfg(windows)]
fn process_exists(process_id: u32) -> Result<bool, Box<dyn Error>> {
    let output = smol::block_on(async {
        Command::new("tasklist.exe")
            .args(["/FI", &format!("PID eq {process_id}"), "/FO", "CSV", "/NH"])
            .output()
            .await
    })?;
    Ok(output.status.success()
        && String::from_utf8_lossy(&output.stdout).contains(&process_id.to_string()))
}

#[cfg(unix)]
fn spawn_foreign_process() -> Result<Child, Box<dyn Error>> {
    Ok(Command::new("/bin/sleep")
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}

#[cfg(windows)]
fn spawn_foreign_process() -> Result<Child, Box<dyn Error>> {
    Ok(Command::new("ping.exe")
        .args(["-n", "30", "127.0.0.1"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}

fn terminate_foreign_process(child: &mut Child) -> Result<(), Box<dyn Error>> {
    if child.try_status()?.is_none() {
        child.kill()?;
    }
    smol::block_on(child.status())?;
    Ok(())
}

fn assert_cases(validation: &str, cases: &BTreeMap<&str, bool>) {
    assert!(
        cases.values().all(|passed| *passed),
        "{validation} cases failed: {cases:#?}"
    );
}

fn target_directory() -> Result<PathBuf, Box<dyn Error>> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root is unavailable")?;
    Ok(std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("target")))
}

fn write_artifact(
    filename: &str,
    validation: &str,
    scope: &str,
    cases: &BTreeMap<&str, bool>,
    fixture_digests: serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    let directory = target_directory()?.join("comfy-parity");
    fs::create_dir_all(&directory)?;
    let artifact = json!({
        "validation_id": validation,
        "validation": validation,
        "scope": scope,
        "environment": {
            "operating_system": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "backend": "native-rust-cpu",
            "process_containment": if cfg!(windows) { "job_object_and_stdin_eof" } else { "owned_process_group_and_stdin_eof" },
            "python_or_javascript_runtime": false
        },
        "fixture_digests": fixture_digests,
        "summary": {
            "passed": cases.len(),
            "failed": 0,
            "skipped": 0
        },
        "cases": cases,
        "skipped": []
    });
    fs::write(
        directory.join(filename),
        serde_json::to_vec_pretty(&artifact)?,
    )?;
    Ok(())
}
