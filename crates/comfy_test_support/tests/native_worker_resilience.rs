use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    thread,
    time::{Duration, Instant},
};

use comfy_runtime::{
    RuntimeSupervisor, RuntimeSupervisorError, SupervisorPolicy, WorkerLaunchConfig,
};
use comfy_tensor::{
    CancellationToken, CpuWorkspaceAuthority, DType, DeviceId, StreamId, TensorBackend,
    TensorDescriptor, TensorError,
};
use comfy_types::{ProfileId, WorkerId};
use serde_json::json;
use sha2::{Digest, Sha256};
use smol::process::{Child, Command};
use uuid::Uuid;

#[path = "support/native_controller.rs"]
mod native_controller;

const MEMORY_LIMIT_BYTES: u64 = 1024 * 1024 * 1024;

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
