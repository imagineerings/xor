use std::{collections::BTreeMap, fs, path::PathBuf, process::Stdio, time::Duration};

#[cfg(feature = "cuda")]
use comfy_runtime::NativeCudaPackageSettings;
#[cfg(feature = "directml")]
use comfy_runtime::NativeDirectMlPackageSettings;
#[cfg(feature = "mlu")]
use comfy_runtime::NativeMluPackageSettings;
#[cfg(feature = "npu")]
use comfy_runtime::NativeNpuPackageSettings;
#[cfg(any(
    feature = "cuda",
    feature = "directml",
    feature = "mlu",
    feature = "npu"
))]
use comfy_runtime::RuntimeSupervisorError;
use comfy_runtime::{
    AssetNamespace, AssetRoots, AssetService, NativeImageExecutor, NativeImageOutputProposal,
    NativeImageWorkerEvent, NativeImageWorkerPlan, NativeImageWorkerProgress,
    NativeImageWorkerProgressKind, ProcessOwnership, PromptCompiler, RuntimeSupervisor,
    SupervisorPolicy, WorkerHealth, WorkerLaunchConfig, authorize_native_input_reader,
    native_image_registry_projection,
};
#[cfg(any(
    feature = "cuda",
    feature = "directml",
    feature = "mlu",
    feature = "npu"
))]
use comfy_types::DeviceKind;
use comfy_types::{
    ApiPrompt, AttemptId, NodeId, ProfileId, PromptId, PromptNode, PromptSubmission, RequestId,
    WorkerEnvelope, WorkerId, WorkerMessage, WorkerOutputProposal,
};
use comfy_worker::{FrameError, read_frame};
use serde_json::json;
use tempfile::TempDir;

fn worker_config() -> WorkerLaunchConfig {
    worker_config_with_memory_limit(1024 * 1024 * 1024)
}

fn worker_config_with_memory_limit(memory_limit_bytes: u64) -> WorkerLaunchConfig {
    WorkerLaunchConfig::new(
        PathBuf::from(env!("CARGO_BIN_EXE_comfy-worker")),
        ProfileId(Default::default()),
        WorkerId(Default::default()),
        "registry-v1",
        memory_limit_bytes,
    )
}

fn fixture_roots() -> Result<(TempDir, AssetRoots), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let profile_id = uuid::Uuid::from_u128(0x5349_4d00_0000_0000_0000_0000_0000_1902);
    let mut typed = Vec::new();
    for (namespace, name) in [
        (AssetNamespace::Input, "input"),
        (AssetNamespace::Output, "output"),
        (AssetNamespace::Temporary, "temporary"),
        (AssetNamespace::Model, "model"),
        (AssetNamespace::Plugin, "plugin"),
    ] {
        let path = temporary.path().join(name);
        fs::create_dir(&path)?;
        typed.push((namespace, path));
    }
    Ok((temporary, AssetRoots::new(profile_id.to_string(), typed)?))
}

fn write_fixture_png(roots: &AssetRoots) -> Result<(), Box<dyn std::error::Error>> {
    let encoded = comfy_media::encode_png_frame(
        &[0.0, 0.25, 0.5, 0.75, 1.0, 0.125],
        1,
        1,
        2,
        3,
        0,
        &BTreeMap::new(),
        comfy_media::PngLimits::default(),
    )?;
    fs::write(
        roots
            .test_root_path(AssetNamespace::Input)?
            .join("fixture.png"),
        encoded,
    )?;
    Ok(())
}

fn encoded_worker_plan(
    plan: comfy_runtime::CompiledPlan,
    roots: &AssetRoots,
    delay_millis: u64,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    encoded_worker_plan_with_memory_policy(
        plan,
        roots,
        delay_millis,
        comfy_runtime::MemoryPolicy::Balanced,
    )
}

fn encoded_worker_plan_with_memory_policy(
    plan: comfy_runtime::CompiledPlan,
    roots: &AssetRoots,
    delay_millis: u64,
    memory_policy: comfy_runtime::MemoryPolicy,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let input = fs::read(
        roots
            .test_root_path(AssetNamespace::Input)?
            .join("fixture.png"),
    )?;
    let worker_plan = NativeImageWorkerPlan::new_with_memory_policy(
        plan,
        BTreeMap::from([("fixture.png".to_owned(), input)]),
        memory_policy,
        true,
        delay_millis,
    )?;
    Ok(serde_json::to_vec(&worker_plan)?)
}

fn fixture_plan(
    delay_millis: u64,
) -> Result<comfy_runtime::CompiledPlan, Box<dyn std::error::Error>> {
    let link = |node: &str, output: usize| json!([node, output]);
    let submission = PromptSubmission {
        prompt: ApiPrompt(BTreeMap::from([
            (
                NodeId("1".to_owned()),
                PromptNode {
                    class_type: "LoadImage".to_owned(),
                    inputs: BTreeMap::from([("image".to_owned(), json!("fixture.png"))]),
                    unknown: BTreeMap::new(),
                },
            ),
            (
                NodeId("2".to_owned()),
                PromptNode {
                    class_type: "ImageScale".to_owned(),
                    inputs: BTreeMap::from([
                        ("image".to_owned(), link("1", 0)),
                        ("upscale_method".to_owned(), json!("nearest-exact")),
                        ("width".to_owned(), json!(4)),
                        ("height".to_owned(), json!(0)),
                        ("crop".to_owned(), json!("disabled")),
                    ]),
                    unknown: BTreeMap::new(),
                },
            ),
            (
                NodeId("3".to_owned()),
                PromptNode {
                    class_type: "ImageInvert".to_owned(),
                    inputs: BTreeMap::from([("image".to_owned(), link("2", 0))]),
                    unknown: BTreeMap::new(),
                },
            ),
            (
                NodeId("4".to_owned()),
                PromptNode {
                    class_type: "PreviewImage".to_owned(),
                    inputs: BTreeMap::from([("images".to_owned(), link("3", 0))]),
                    unknown: BTreeMap::new(),
                },
            ),
            (
                NodeId("5".to_owned()),
                PromptNode {
                    class_type: "SaveImage".to_owned(),
                    inputs: BTreeMap::from([
                        ("images".to_owned(), link("3", 0)),
                        ("filename_prefix".to_owned(), json!("worker-native-image")),
                    ]),
                    unknown: BTreeMap::new(),
                },
            ),
        ])),
        prompt_id: Some(PromptId(uuid::Uuid::from_u128(1902))),
        client_id: Some("native-worker-test".to_owned()),
        number: Some(1.0),
        extra_data: BTreeMap::from([("sim_native_delay_millis".to_owned(), json!(delay_millis))]),
        unknown: BTreeMap::new(),
    };
    let registry = native_image_registry_projection()?;
    Ok(PromptCompiler::new(&registry).compile(submission)?)
}

#[test]
fn packaged_worker_handshake_heartbeat_cancel_and_stop() {
    smol::block_on(async {
        let (temporary, roots) = fixture_roots().expect("fixture roots");
        write_fixture_png(&roots).expect("fixture PNG");
        let plan = fixture_plan(10_000).expect("native image plan");
        let mut config = worker_config();
        config.profile_id = ProfileId(
            uuid::Uuid::parse_str(&roots.profile_id).expect("fixture profile identifier"),
        );
        config.policy = SupervisorPolicy {
            heartbeat_interval: Duration::from_secs(3),
            missed_heartbeat_limit: 1,
            ..SupervisorPolicy::default()
        };
        let mut supervisor = RuntimeSupervisor::start(config)
            .await
            .expect("worker becomes ready");
        assert_eq!(
            supervisor.accepted_backend().map(|matrix| matrix.device()),
            Some(comfy_tensor::DeviceId::CPU)
        );
        assert_eq!(supervisor.snapshot().health, WorkerHealth::BackendReady);
        assert_eq!(
            supervisor.snapshot().launch.ownership,
            if cfg!(windows) {
                ProcessOwnership::WindowsJobObject
            } else {
                ProcessOwnership::ProcessGroup
            }
        );

        test_delay(Duration::from_millis(3_200)).await;
        assert_eq!(supervisor.snapshot().health, WorkerHealth::BackendReady);
        assert_eq!(supervisor.snapshot().missed_heartbeats, 0);

        let prompt_id = plan.prompt_id;
        let attempt_id = AttemptId(Default::default());
        let encoded_plan =
            encoded_worker_plan(plan, &roots, 10_000).expect("encode native image worker plan");
        supervisor
            .execute(prompt_id, attempt_id, encoded_plan)
            .await
            .expect("execution accepted");
        let started = supervisor
            .next_event(Duration::from_secs(1))
            .await
            .expect("execution event");
        assert!(matches!(
            started.message,
            WorkerMessage::Lifecycle {
                event: comfy_types::WorkerLifecycleEvent::ExecutionStarted
            }
        ));
        supervisor
            .cancel(prompt_id, attempt_id, "integration test")
            .await
            .expect("cancellation sent");
        let cancellation_requested = supervisor
            .next_event(Duration::from_secs(1))
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "cancellation requested event failed: {error}; logs: {:?}",
                    supervisor.logs()
                )
            });
        assert!(matches!(
            cancellation_requested.message,
            WorkerMessage::Lifecycle {
                event: comfy_types::WorkerLifecycleEvent::CancellationRequested { .. }
            }
        ));
        let cancelled = supervisor
            .next_event(Duration::from_secs(2))
            .await
            .expect("cancellation completion event");
        let WorkerMessage::Event { event } = cancelled.message else {
            panic!("expected cancellation completion event");
        };
        assert!(matches!(
            postcard::from_bytes::<NativeImageWorkerEvent>(&event),
            Ok(NativeImageWorkerEvent::Failed {
                cancelled: true,
                ..
            })
        ));
        assert_eq!(supervisor.snapshot().health, WorkerHealth::BackendReady);

        let status = supervisor.shutdown().await.expect("graceful shutdown");
        assert!(status.success());
        assert!(matches!(
            supervisor.snapshot().health,
            WorkerHealth::Exited { success: true, .. }
        ));
        assert!(temporary.path().exists());
    });
}

#[test]
fn packaged_worker_executes_native_image_plan_and_only_proposes_outputs() {
    smol::block_on(async {
        let (temporary, roots) = fixture_roots().expect("fixture roots");
        write_fixture_png(&roots).expect("fixture PNG");
        let plan = fixture_plan(0).expect("native image plan");
        let prompt_id = plan.prompt_id;
        let attempt_id = AttemptId(uuid::Uuid::from_u128(1902));
        let mut config = worker_config();
        config.profile_id = ProfileId(
            uuid::Uuid::parse_str(&roots.profile_id).expect("fixture profile identifier"),
        );
        let mut supervisor = RuntimeSupervisor::start(config)
            .await
            .expect("worker becomes ready");
        supervisor
            .execute(
                prompt_id,
                attempt_id,
                encoded_worker_plan_with_memory_policy(
                    plan,
                    &roots,
                    0,
                    comfy_runtime::MemoryPolicy::Conservative,
                )
                .expect("encode worker plan"),
            )
            .await
            .expect("execution accepted");
        let started = supervisor
            .next_event(Duration::from_secs(1))
            .await
            .expect("worker start event");
        assert!(matches!(
            started.message,
            WorkerMessage::Lifecycle {
                event: comfy_types::WorkerLifecycleEvent::ExecutionStarted
            }
        ));
        let (terminal, progress, proposals) =
            await_native_terminal_with_progress(&supervisor, Duration::from_secs(5)).await;
        let result = match terminal {
            NativeImageWorkerEvent::Completed { result } => result,
            NativeImageWorkerEvent::Failed { message, .. } => {
                panic!("native worker failed: {message}")
            }
            NativeImageWorkerEvent::BackendUnavailable { unavailable } => {
                panic!("native worker backend unavailable: {unavailable}")
            }
            NativeImageWorkerEvent::Progress { .. } => {
                unreachable!("terminal helper filters progress")
            }
        };
        assert_eq!(result.report.state, comfy_runtime::AttemptState::Succeeded);
        assert_eq!(result.output_proposal_ids.len(), 2);
        assert_eq!(proposals.len(), 2);
        assert_eq!(
            result.output_proposal_ids,
            proposals
                .iter()
                .map(WorkerOutputProposal::proposal_id)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            progress
                .iter()
                .filter_map(|progress| match progress.kind {
                    NativeImageWorkerProgressKind::Progress { completed, .. } => Some(completed),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
        assert_eq!(
            progress
                .iter()
                .filter(|progress| {
                    matches!(
                        progress.kind,
                        NativeImageWorkerProgressKind::OutputPrepared { .. }
                    )
                })
                .filter_map(|progress| progress.node_id.as_ref().map(|node| node.0.as_str()))
                .collect::<Vec<_>>(),
            vec!["4", "5"]
        );
        assert!(
            fs::read_dir(
                roots
                    .test_root_path(AssetNamespace::Output)
                    .expect("output root"),
            )
            .expect("read output root")
            .next()
            .is_none(),
            "the worker must not publish a final output"
        );
        supervisor.shutdown().await.expect("worker shutdown");
        assert!(temporary.path().exists());
    });
}

#[test]
fn packaged_worker_reports_preflight_oom_without_dispatch_or_restart() {
    smol::block_on(async {
        let (temporary, roots) = fixture_roots().expect("fixture roots");
        write_fixture_png(&roots).expect("fixture PNG");
        let plan = fixture_plan(0).expect("native image plan");
        let prompt_id = plan.prompt_id;
        let attempt_id = AttemptId(uuid::Uuid::from_u128(0x1902_0002));
        let mut config = worker_config_with_memory_limit(64 * 1024 * 1024);
        config.profile_id = ProfileId(
            uuid::Uuid::parse_str(&roots.profile_id).expect("fixture profile identifier"),
        );
        let mut supervisor = RuntimeSupervisor::start(config)
            .await
            .expect("worker becomes ready");
        supervisor
            .execute(
                prompt_id,
                attempt_id,
                encoded_worker_plan_with_memory_policy(
                    plan,
                    &roots,
                    0,
                    comfy_runtime::MemoryPolicy::Conservative,
                )
                .expect("encode worker plan"),
            )
            .await
            .expect("execution accepted");
        let started = supervisor
            .next_event(Duration::from_secs(1))
            .await
            .expect("worker start event");
        assert!(matches!(
            started.message,
            WorkerMessage::Lifecycle {
                event: comfy_types::WorkerLifecycleEvent::ExecutionStarted
            }
        ));
        let (terminal, progress, proposals) =
            await_native_terminal_with_progress(&supervisor, Duration::from_secs(5)).await;
        let NativeImageWorkerEvent::Failed { message, cancelled } = terminal else {
            panic!("expected native memory preflight failure");
        };
        assert!(!cancelled);
        assert!(message.contains("native memory preflight failed without dispatch"));
        assert!(message.contains("capacity"));
        assert!(message.contains("residency=novram"));
        assert!(progress.is_empty());
        assert!(proposals.is_empty());
        assert!(matches!(
            supervisor.snapshot().health,
            WorkerHealth::BackendReady
        ));
        let status = supervisor.shutdown().await.expect("graceful shutdown");
        assert!(status.success());
        assert!(matches!(
            supervisor.snapshot().health,
            WorkerHealth::Exited { success: true, .. }
        ));
        assert!(temporary.path().exists());
    });
}

#[test]
fn packaged_worker_kill_interrupts_then_bounded_recovery_accepts_explicit_retry() {
    smol::block_on(async {
        let (temporary, roots) = fixture_roots().expect("fixture roots");
        write_fixture_png(&roots).expect("fixture PNG");
        let plan = fixture_plan(10_000).expect("native image plan");
        let prompt_id = plan.prompt_id;
        let interrupted_attempt_id = AttemptId(uuid::Uuid::from_u128(0x1902_0001));
        let mut config = worker_config();
        config.profile_id = ProfileId(
            uuid::Uuid::parse_str(&roots.profile_id).expect("fixture profile identifier"),
        );
        let mut supervisor = RuntimeSupervisor::start(config)
            .await
            .expect("worker becomes ready");
        supervisor
            .execute(
                prompt_id,
                interrupted_attempt_id,
                encoded_worker_plan(plan.clone(), &roots, 10_000)
                    .expect("encode delayed worker plan"),
            )
            .await
            .expect("delayed execution accepted");
        let started = supervisor
            .next_event(Duration::from_secs(1))
            .await
            .expect("worker start event");
        assert!(matches!(
            started.message,
            WorkerMessage::Lifecycle {
                event: comfy_types::WorkerLifecycleEvent::ExecutionStarted
            }
        ));

        let status = supervisor.terminate().await.expect("worker terminated");
        assert!(!status.success());
        assert!(
            fs::read_dir(
                roots
                    .test_root_path(AssetNamespace::Output)
                    .expect("output root"),
            )
            .expect("read output root")
            .next()
            .is_none(),
            "interrupted execution must not expose a partial output"
        );

        let mut supervisor = supervisor
            .recover()
            .await
            .expect("bounded replacement worker starts");
        assert_eq!(supervisor.snapshot().health, WorkerHealth::BackendReady);
        let retry_attempt_id = AttemptId(uuid::Uuid::from_u128(0x1902_0002));
        assert_ne!(retry_attempt_id, interrupted_attempt_id);
        supervisor
            .execute(
                prompt_id,
                retry_attempt_id,
                encoded_worker_plan(plan, &roots, 0).expect("encode retry worker plan"),
            )
            .await
            .expect("explicit retry accepted");
        let retry_started = supervisor
            .next_event(Duration::from_secs(1))
            .await
            .expect("retry start event");
        assert!(matches!(
            retry_started.message,
            WorkerMessage::Lifecycle {
                event: comfy_types::WorkerLifecycleEvent::ExecutionStarted
            }
        ));
        let (terminal, _, proposals) =
            await_native_terminal_with_progress(&supervisor, Duration::from_secs(5)).await;
        let NativeImageWorkerEvent::Completed { result } = terminal else {
            panic!("expected successful retry completion");
        };
        assert_eq!(result.report.attempt_id, retry_attempt_id);
        assert_eq!(result.report.state, comfy_runtime::AttemptState::Succeeded);
        assert_eq!(result.output_proposal_ids.len(), 2);
        assert_eq!(proposals.len(), 2);
        supervisor.shutdown().await.expect("replacement shutdown");
        assert!(temporary.path().exists());
    });
}

async fn await_native_terminal_with_progress(
    supervisor: &RuntimeSupervisor,
    timeout: Duration,
) -> (
    NativeImageWorkerEvent,
    Vec<NativeImageWorkerProgress>,
    Vec<WorkerOutputProposal>,
) {
    let deadline = std::time::Instant::now() + timeout;
    let mut progress_events = Vec::new();
    let mut output_proposals = Vec::new();
    let mut last_progress_sequence = None;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        assert!(
            !remaining.is_zero(),
            "native worker terminal event timed out"
        );
        let envelope = supervisor
            .next_event(remaining)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "worker terminal event failed: {error}; logs: {:?}",
                    supervisor.logs()
                )
            });
        let envelope_profile_id = envelope.profile_id;
        let envelope_prompt_id = envelope.prompt_id;
        let envelope_attempt_id = envelope.attempt_id;
        let event = match envelope.message {
            WorkerMessage::OutputProposal { proposal } => {
                output_proposals.push(proposal);
                continue;
            }
            WorkerMessage::Event { event } => event,
            _ => continue,
        };
        match postcard::from_bytes::<NativeImageWorkerEvent>(&event) {
            Ok(NativeImageWorkerEvent::Completed { result }) => {
                return (
                    NativeImageWorkerEvent::Completed { result },
                    progress_events,
                    output_proposals,
                );
            }
            Ok(NativeImageWorkerEvent::Failed { message, cancelled }) => {
                return (
                    NativeImageWorkerEvent::Failed { message, cancelled },
                    progress_events,
                    output_proposals,
                );
            }
            Ok(NativeImageWorkerEvent::BackendUnavailable { unavailable }) => {
                return (
                    NativeImageWorkerEvent::BackendUnavailable { unavailable },
                    progress_events,
                    output_proposals,
                );
            }
            Ok(NativeImageWorkerEvent::Progress { progress }) => {
                // The reader may already have accepted a later terminal frame and updated the
                // snapshot while earlier ordered frames remain in the bounded delivery channel.
                assert_eq!(progress.profile_id, envelope_profile_id);
                assert_eq!(Some(progress.prompt_id), envelope_prompt_id);
                assert_eq!(Some(progress.attempt_id), envelope_attempt_id);
                if let Some(previous) = last_progress_sequence {
                    assert!(progress.sequence > previous);
                }
                last_progress_sequence = Some(progress.sequence);
                progress_events.push(progress);
            }
            Err(_) => {}
        }
    }
}

#[allow(clippy::disallowed_methods)]
async fn test_delay(duration: Duration) {
    smol::Timer::after(duration).await;
}

#[test]
fn explicit_termination_kills_the_owned_worker_tree() {
    smol::block_on(async {
        let mut supervisor = RuntimeSupervisor::start(worker_config())
            .await
            .expect("worker becomes ready");
        let status = supervisor.terminate().await.expect("worker terminated");
        assert!(!status.success());
        assert!(matches!(
            supervisor.snapshot().health,
            WorkerHealth::Exited { success: false, .. }
        ));
    });
}

#[test]
fn worker_fault_is_typed_and_does_not_abort_the_caller() {
    smol::block_on(async {
        let mut supervisor = RuntimeSupervisor::start(worker_config())
            .await
            .expect("worker becomes ready");
        supervisor
            .cancel(
                PromptId(Default::default()),
                AttemptId(Default::default()),
                "no active attempt",
            )
            .await
            .expect("invalid command reaches worker");
        let fatal = supervisor
            .next_event(Duration::from_secs(1))
            .await
            .expect("fatal frame remains a domain event");
        assert!(matches!(fatal.message, WorkerMessage::Fatal { .. }));
        assert!(matches!(
            supervisor.snapshot().health,
            WorkerHealth::Degraded { .. }
        ));
        let mut supervisor = supervisor
            .recover()
            .await
            .expect("bounded replacement worker starts");
        assert_eq!(supervisor.snapshot().health, WorkerHealth::BackendReady);
        supervisor
            .cancel(
                PromptId(Default::default()),
                AttemptId(Default::default()),
                "second invalid attempt",
            )
            .await
            .expect("second fault reaches replacement");
        let second_fatal = supervisor
            .next_event(Duration::from_secs(1))
            .await
            .expect("replacement reports its fault");
        assert!(matches!(second_fatal.message, WorkerMessage::Fatal { .. }));
        assert!(matches!(
            supervisor.recover().await,
            Err(comfy_runtime::RuntimeSupervisorError::RecoveryBudgetExhausted { maximum: 1 })
        ));
    });
}

#[cfg(feature = "mlu")]
#[test]
fn packaged_worker_reports_unavailable_mlu_before_ready_without_cpu_fallback() {
    smol::block_on(async {
        let package = NativeMluPackageSettings::from_public_authority(
            "/missing/reviewed-mlu-package",
            "mlu.release",
            &"55".repeat(32),
        )
        .expect("bounded public MLU authority");
        for launch in 0..2 {
            let config = WorkerLaunchConfig::for_mlu(
                PathBuf::from(env!("CARGO_BIN_EXE_comfy-worker")),
                ProfileId(Default::default()),
                WorkerId(Default::default()),
                "registry-v1",
                package.clone(),
                0,
                4096,
            )
            .expect("MLU launch configuration");
            let error = match RuntimeSupervisor::start(config).await {
                Ok(mut supervisor) => {
                    let termination = supervisor.terminate().await;
                    panic!("untrusted MLU worker launch {launch} became ready: {termination:?}");
                }
                Err(error) => error,
            };
            assert!(matches!(
                error,
                RuntimeSupervisorError::BackendUnavailable(unavailable)
                    if unavailable.device() == DeviceKind::Mlu
            ));
        }
    });
}

#[cfg(feature = "npu")]
#[test]
fn packaged_worker_reports_unavailable_npu_before_ready_without_cpu_fallback() {
    smol::block_on(async {
        let package = NativeNpuPackageSettings::from_public_authority(
            "/missing/reviewed-npu-package",
            "npu.release",
            &"57".repeat(32),
        )
        .expect("bounded public NPU authority");
        for launch in 0..2 {
            let config = WorkerLaunchConfig::for_npu(
                PathBuf::from(env!("CARGO_BIN_EXE_comfy-worker")),
                ProfileId(Default::default()),
                WorkerId(Default::default()),
                "registry-v1",
                package.clone(),
                0,
                4096,
            )
            .expect("NPU launch configuration");
            let error = match RuntimeSupervisor::start(config).await {
                Ok(mut supervisor) => {
                    let termination = supervisor.terminate().await;
                    panic!("untrusted NPU worker launch {launch} became ready: {termination:?}");
                }
                Err(error) => error,
            };
            assert!(matches!(
                error,
                RuntimeSupervisorError::BackendUnavailable(unavailable)
                    if unavailable.device() == DeviceKind::Npu
            ));
        }
    });
}

#[cfg(feature = "directml")]
#[test]
fn packaged_worker_reports_unavailable_directml_before_ready_without_cpu_fallback() {
    smol::block_on(async {
        let package = NativeDirectMlPackageSettings::from_public_authority(
            "/missing/reviewed-directml-package",
            "directml.release",
            &"66".repeat(32),
        )
        .expect("bounded public DirectML authority");
        for launch in 0..2 {
            let config = WorkerLaunchConfig::for_directml(
                PathBuf::from(env!("CARGO_BIN_EXE_comfy-worker")),
                ProfileId(Default::default()),
                WorkerId(Default::default()),
                "registry-v1",
                package.clone(),
                4096,
            )
            .expect("DirectML launch configuration");
            let error = match RuntimeSupervisor::start(config).await {
                Ok(mut supervisor) => {
                    let termination = supervisor.terminate().await;
                    panic!(
                        "untrusted DirectML worker launch {launch} became ready: {termination:?}"
                    );
                }
                Err(error) => error,
            };
            assert!(matches!(
                error,
                RuntimeSupervisorError::BackendUnavailable(unavailable)
                    if unavailable.device() == DeviceKind::DirectMl
            ));
        }
    });
}

#[cfg(feature = "cuda")]
#[test]
fn packaged_worker_reports_unavailable_cuda_before_ready_without_cpu_fallback() {
    smol::block_on(async {
        let package = NativeCudaPackageSettings::from_public_authority(
            "/missing/reviewed-cuda-package",
            "cuda.release",
            &"56".repeat(32),
        )
        .expect("bounded public CUDA authority");
        for launch in 0..2 {
            let config = WorkerLaunchConfig::for_cuda(
                PathBuf::from(env!("CARGO_BIN_EXE_comfy-worker")),
                ProfileId(Default::default()),
                WorkerId(Default::default()),
                "registry-v1",
                package.clone(),
                0,
                4096,
            )
            .expect("CUDA launch configuration");
            let error = match RuntimeSupervisor::start(config).await {
                Ok(mut supervisor) => {
                    let termination = supervisor.terminate().await;
                    panic!("untrusted CUDA worker launch {launch} became ready: {termination:?}");
                }
                Err(error) => error,
            };
            assert!(matches!(
                error,
                RuntimeSupervisorError::BackendUnavailable(unavailable)
                    if unavailable.device() == DeviceKind::Cuda
            ));
        }
    });
}

#[test]
fn framing_rejects_truncation_and_oversized_lengths_before_decode() {
    assert!(matches!(
        read_frame([1_u8, 0, 0, 0].as_slice()),
        Err(FrameError::Io(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof
    ));
    let oversized = u32::try_from(comfy_types::MAX_WORKER_FRAME_BYTES + 1)
        .expect("limit fits u32")
        .to_le_bytes();
    assert!(matches!(
        read_frame(oversized.as_slice()),
        Err(FrameError::TooLarge)
    ));
}

#[test]
fn packaged_worker_rejects_protocol_skew_before_ready_and_logs_only_to_stderr() {
    smol::block_on(async {
        use smol::io::AsyncWriteExt as _;

        let mut command = smol::process::Command::new(env!("CARGO_BIN_EXE_comfy-worker"));
        command
            .args(["--memory-limit-bytes", "1024"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().expect("spawn worker");
        let skewed = WorkerEnvelope {
            version: comfy_types::WORKER_PROTOCOL_VERSION + 1,
            profile_id: ProfileId(Default::default()),
            worker_id: WorkerId(Default::default()),
            request_id: RequestId(Default::default()),
            prompt_id: None,
            attempt_id: None,
            sequence: 0,
            registry_version: "registry-v1".to_owned(),
            message: WorkerMessage::Hello {
                backend: comfy_tensor::CpuBackend::capability_matrix()
                    .to_worker_capabilities()
                    .expect("CPU capabilities project to worker protocol"),
            },
            extensions: BTreeMap::new(),
        };
        let payload = postcard::to_stdvec(&skewed).expect("serialize skew fixture");
        let payload_length = u32::try_from(payload.len()).expect("skew fixture length fits u32");
        let input = child.stdin.as_mut().expect("worker stdin");
        input
            .write_all(&payload_length.to_le_bytes())
            .await
            .expect("write length");
        input.write_all(&payload).await.expect("write payload");
        input.flush().await.expect("flush payload");
        drop(child.stdin.take());
        let output = child.output().await.expect("worker exits");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty(), "stdout must contain frames only");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unsupported worker protocol version"));
    });
}

#[test]
fn ipc_schema_contains_no_tensor_pointer_path_or_plugin_handle()
-> Result<(), Box<dyn std::error::Error>> {
    let protocol_source = include_str!("../../comfy_types/src/worker_protocol.rs");
    let production_source = protocol_source
        .split_once("#[cfg(test)]")
        .map_or(protocol_source, |(production, _)| production);
    for forbidden in [
        "comfy_tensor::Tensor",
        "TensorDescriptor",
        "PathBuf",
        "*mut",
        "PluginHandle",
        "PrepareOutput",
        "CommitOutput",
        "OutputCommitter",
    ] {
        assert!(
            !production_source.contains(forbidden),
            "private IPC contains forbidden boundary type {forbidden}"
        );
    }

    let (_temporary, roots) = fixture_roots()?;
    write_fixture_png(&roots)?;
    let plan = fixture_plan(0)?;
    let assets = std::sync::Arc::new(std::sync::Mutex::new(AssetService::open(roots.clone())?));
    let input_authorization = authorize_native_input_reader(&roots.profile_id)?;
    let worker_plan = NativeImageWorkerPlan::from_asset_service(
        plan.clone(),
        &assets,
        &input_authorization,
        &comfy_types::CancellationToken::default(),
        true,
        0,
    )?;
    let encoded_plan = serde_json::to_vec(&worker_plan)?;
    let plan_value = serde_json::to_value(&worker_plan)?;
    assert_eq!(
        plan_value.get("memory_policy"),
        Some(&serde_json::Value::String("balanced".to_owned()))
    );
    let mut legacy_plan_value = plan_value.clone();
    legacy_plan_value
        .as_object_mut()
        .ok_or("native worker plan must encode as an object")?
        .remove("memory_policy");
    let legacy_plan: NativeImageWorkerPlan = serde_json::from_value(legacy_plan_value)?;
    assert_eq!(
        legacy_plan.memory_policy,
        comfy_runtime::MemoryPolicy::Balanced
    );
    assert_path_free_json(&plan_value)?;
    assert_bytes_omit_host_root(&encoded_plan, &roots)?;

    let profile_id = ProfileId(uuid::Uuid::parse_str(&roots.profile_id)?);
    let (cpu_backend, workspace_authority) =
        comfy_tensor::CpuWorkspaceAuthority::create_backend(64 * 1024 * 1024)?;
    let executor = NativeImageExecutor::new_with_cpu_backend(
        profile_id,
        worker_plan.input_assets,
        true,
        std::sync::Arc::new(cpu_backend),
    )?;
    let result = executor.execute_blocking(
        &plan,
        AttemptId(uuid::Uuid::from_u128(0x1902)),
        comfy_types::CancellationToken::default(),
        0,
        workspace_authority.authorize_workspace(64 * 1024 * 1024)?,
    )?;
    for proposal in result.output_proposals {
        let wire = proposal.to_worker_proposal()?;
        let encoded = comfy_types::encode_worker_frame(&WorkerEnvelope {
            version: comfy_types::WORKER_PROTOCOL_VERSION,
            profile_id,
            worker_id: WorkerId(Default::default()),
            request_id: RequestId(Default::default()),
            prompt_id: Some(plan.prompt_id),
            attempt_id: Some(AttemptId(uuid::Uuid::from_u128(0x1902))),
            sequence: 0,
            registry_version: "registry-v1".to_owned(),
            message: WorkerMessage::OutputProposal {
                proposal: wire.clone(),
            },
            extensions: BTreeMap::new(),
        })?;
        assert_bytes_omit_host_root(&encoded, &roots)?;
        let canonical = NativeImageOutputProposal::from_worker_proposal(wire)?;
        assert!(matches!(
            canonical.output().namespace(),
            AssetNamespace::Output | AssetNamespace::Temporary
        ));
        assert!(!canonical.output().filename_prefix().starts_with('/'));
    }
    Ok(())
}

fn assert_path_free_json(value: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                assert_path_free_json(value)?;
            }
        }
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                for prohibited in [
                    "path",
                    "root",
                    "pointer",
                    "plugin_handle",
                    "commit_operation",
                ] {
                    if key.eq_ignore_ascii_case(prohibited) {
                        return Err(format!("worker payload contains prohibited key {key}").into());
                    }
                }
                assert_path_free_json(value)?;
            }
        }
        serde_json::Value::String(value) => {
            if value.starts_with("/") || value.contains(":\\") {
                return Err(format!("worker payload contains host path {value}").into());
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
    Ok(())
}

fn assert_bytes_omit_host_root(
    bytes: &[u8],
    roots: &AssetRoots,
) -> Result<(), Box<dyn std::error::Error>> {
    for namespace in [
        AssetNamespace::Input,
        AssetNamespace::Output,
        AssetNamespace::Temporary,
        AssetNamespace::Model,
        AssetNamespace::Plugin,
    ] {
        let root = roots.test_root_path(namespace)?.to_string_lossy();
        assert!(
            !bytes
                .windows(root.len())
                .any(|window| window == root.as_bytes()),
            "encoded worker payload contains host root {root}"
        );
    }
    Ok(())
}
