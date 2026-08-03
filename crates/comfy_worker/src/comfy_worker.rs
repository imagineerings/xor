use std::{
    collections::BTreeMap,
    future::pending,
    io::{self, Read, Write},
    sync::Arc,
    time::{Duration, Instant},
};

use comfy_runtime::{
    AttemptEvent, AttemptEventKind, AttemptState, ExecutionEventBus, NativeDiffusionProvider,
    NativeImageExecutor, NativeImageRuntimeError, NativeImageWorkerEvent, NativeImageWorkerPlan,
    NativeImageWorkerProgress, NativeImageWorkerProgressKind, PluginAuthorizationVerifier,
    WorkerBackendSelection,
};
use comfy_tensor::{CancellationToken, CpuBackend, DeviceId};
use comfy_types::{
    BackendUnavailable, DeviceKind, MAX_ENCODED_PREVIEW_BYTES, MAX_WORKER_FRAME_BYTES,
    WorkerEnvelope, WorkerMessage, WorkerPluginExecutionOutcome, WorkerProtocolError,
    WorkerRegistryDeploymentRejectionReason, decode_worker_frame, encode_worker_frame,
};
use thiserror::Error;

pub mod memory_modes;
pub mod memory_planner;
mod plugin_runtime;
pub mod supervisor;

pub use memory_modes::*;
pub use memory_planner::*;
pub use supervisor::*;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);

enum NextWorkerInput {
    Frame(Result<Result<WorkerEnvelope, FrameError>, async_channel::RecvError>),
    Heartbeat,
    NativeJob(
        Result<
            Result<comfy_runtime::NativeImageExecutionResult, NativeImageRuntimeError>,
            async_channel::RecvError,
        >,
    ),
    PluginJob(Result<WorkerPluginExecutionOutcome, async_channel::RecvError>),
    PluginCapability(
        Result<plugin_runtime::WorkerCapabilityBridgeRequest, async_channel::RecvError>,
    ),
    JobEvent(Result<AttemptEvent, async_channel::RecvError>),
}

struct ActiveExecution {
    cancellation: CancellationToken,
    memory: Option<AttemptMemoryController>,
    native_result: Option<
        async_channel::Receiver<
            Result<comfy_runtime::NativeImageExecutionResult, NativeImageRuntimeError>,
        >,
    >,
    plugin_result: Option<async_channel::Receiver<WorkerPluginExecutionOutcome>>,
    plugin_capabilities:
        Option<async_channel::Receiver<plugin_runtime::WorkerCapabilityBridgeRequest>>,
    events: Option<async_channel::Receiver<AttemptEvent>>,
}

struct CommittedPluginRegistry {
    source: Arc<AssembledWorkerRegistry>,
    compiled: Arc<plugin_runtime::WorkerPluginRegistry>,
}

fn apply_compiled_registry_commit(
    current: &mut Option<CommittedPluginRegistry>,
    responses: &[WorkerEnvelope],
    source: Option<Arc<AssembledWorkerRegistry>>,
    compiled: Option<Arc<plugin_runtime::WorkerPluginRegistry>>,
) -> anyhow::Result<()> {
    let acknowledged = matches!(
        responses,
        [WorkerEnvelope {
            message: WorkerMessage::RegistryDeploymentAck { .. },
            ..
        }]
    );
    if acknowledged {
        *current = Some(CommittedPluginRegistry {
            source: source.ok_or_else(|| {
                anyhow::anyhow!("worker registry commit produced no source registry")
            })?,
            compiled: compiled.ok_or_else(|| {
                anyhow::anyhow!("worker registry commit produced no compiled registry")
            })?,
        });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("worker frame exceeds {MAX_WORKER_FRAME_BYTES} bytes")]
    TooLarge,
    #[error("encoded worker event exceeds {MAX_ENCODED_PREVIEW_BYTES} bytes")]
    PreviewTooLarge,
    #[error("worker frame allocation failed: {0}")]
    Allocation(String),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Protocol(#[from] WorkerProtocolError),
}

pub fn validate_worker_payload(envelope: &WorkerEnvelope) -> Result<(), FrameError> {
    if let WorkerMessage::Event { event } = &envelope.message
        && event.len() > MAX_ENCODED_PREVIEW_BYTES
    {
        return Err(FrameError::PreviewTooLarge);
    }
    Ok(())
}

pub fn apply_worker_control_cancellation(
    message: &WorkerMessage,
    cancellation: &CancellationToken,
) -> Option<bool> {
    match message {
        WorkerMessage::Cancel { .. } | WorkerMessage::Shutdown => Some(cancellation.cancel()),
        _ => None,
    }
}

pub fn write_frame(mut writer: impl Write, envelope: &WorkerEnvelope) -> Result<(), FrameError> {
    validate_worker_payload(envelope)?;
    let frame = encode_worker_frame(envelope)?;
    writer.write_all(&frame)?;
    writer.flush()?;
    Ok(())
}

pub fn read_frame(mut reader: impl Read) -> Result<WorkerEnvelope, FrameError> {
    let mut prefix = [0_u8; 4];
    reader.read_exact(&mut prefix)?;
    let length = u32::from_le_bytes(prefix) as usize;
    if length > MAX_WORKER_FRAME_BYTES {
        return Err(FrameError::TooLarge);
    }

    let mut frame = Vec::new();
    frame
        .try_reserve_exact(4_usize.saturating_add(length))
        .map_err(|error| FrameError::Allocation(error.to_string()))?;
    frame.extend_from_slice(&prefix);
    frame.resize(4 + length, 0);
    let payload = frame
        .get_mut(4..)
        .ok_or_else(|| FrameError::Allocation("invalid frame allocation length".to_owned()))?;
    reader.read_exact(payload)?;
    let envelope = decode_worker_frame(&frame)?;
    validate_worker_payload(&envelope)?;
    Ok(envelope)
}

pub async fn run_worker_process(memory_limit_bytes: u64) -> anyhow::Result<()> {
    run_worker_process_with_authorization_verifier(memory_limit_bytes, None).await
}

pub async fn run_worker_process_with_authorization_verifier(
    memory_limit_bytes: u64,
    plugin_authorization_verifier: Option<PluginAuthorizationVerifier>,
) -> anyhow::Result<()> {
    run_worker_process_with_backend_selection(
        memory_limit_bytes,
        WorkerBackendSelection::Cpu,
        plugin_authorization_verifier,
    )
    .await
}

pub async fn run_worker_process_with_backend_selection(
    memory_limit_bytes: u64,
    backend_selection: WorkerBackendSelection,
    plugin_authorization_verifier: Option<PluginAuthorizationVerifier>,
) -> anyhow::Result<()> {
    run_worker_process_with_configuration(
        memory_limit_bytes,
        backend_selection,
        None,
        plugin_authorization_verifier,
    )
    .await
}

pub async fn run_worker_process_with_diffusion_provider(
    memory_limit_bytes: u64,
    diffusion_provider: Option<Arc<dyn NativeDiffusionProvider>>,
) -> anyhow::Result<()> {
    run_worker_process_with_configuration(
        memory_limit_bytes,
        WorkerBackendSelection::Cpu,
        diffusion_provider,
        None,
    )
    .await
}

fn initialize_worker_backend(
    selection: WorkerBackendSelection,
    memory_limit_bytes: u64,
) -> (
    Result<WorkerBackendSession, BackendUnavailable>,
    Option<Arc<CpuBackend>>,
) {
    match selection {
        WorkerBackendSelection::Cpu => match WorkerBackendSession::cpu(memory_limit_bytes) {
            Ok((session, backend)) => (Ok(session), Some(backend)),
            Err(error) => (
                Err(BackendUnavailable::new(DeviceKind::Cpu, error.to_string())),
                None,
            ),
        },
        WorkerBackendSelection::DirectMl { package } => {
            #[cfg(feature = "directml")]
            {
                let cancellation = CancellationToken::default();
                let initialized =
                    comfy_runtime::initialize_certified_directml_runtime(&package, &cancellation)
                        .and_then(|session| {
                            comfy_tensor::generated_backend_directml_comfy_model_0018::DirectMlTensorBackend::from_certified_session(
                                session,
                                memory_limit_bytes,
                                &cancellation,
                            )
                            .map_err(|_| {
                                BackendUnavailable::new(
                                    DeviceKind::DirectMl,
                                    "certified DirectML tensor backend initialization failed",
                                )
                            })
                        })
                        .and_then(|(backend, authority)| {
                            WorkerBackendSession::new(Arc::new(backend), authority).map_err(|_| {
                                BackendUnavailable::new(
                                    DeviceKind::DirectMl,
                                    "workspace, allocation, transfer, operator, event, or accounting readiness probe failed",
                                )
                            })
                        });
                (initialized, None)
            }
            #[cfg(not(feature = "directml"))]
            {
                let _ = (package, memory_limit_bytes);
                (
                    Err(BackendUnavailable::new(
                        DeviceKind::DirectMl,
                        "the packaged worker was built without the DirectML integration feature",
                    )),
                    None,
                )
            }
        }
        WorkerBackendSelection::Rocm {
            package,
            device_ordinal,
        } => {
            #[cfg(feature = "rocm")]
            {
                let cancellation = CancellationToken::default();
                let initialized =
                    comfy_runtime::initialize_certified_rocm_runtime(&package, &cancellation)
                        .and_then(|certified| {
                            comfy_tensor::RocmTensorBackend::from_certified_runtime(
                                certified.into_runtime(),
                                device_ordinal,
                                memory_limit_bytes,
                                &cancellation,
                            )
                            .map_err(|_| {
                                BackendUnavailable::new(
                                    DeviceKind::Rocm,
                                    "driver or device initialization failed",
                                )
                            })
                        })
                        .and_then(|(backend, authority)| {
                            WorkerBackendSession::new(Arc::new(backend), authority).map_err(|_| {
                                BackendUnavailable::new(
                                    DeviceKind::Rocm,
                                    "allocation, transfer, or event readiness probe failed",
                                )
                            })
                        });
                (initialized, None)
            }
            #[cfg(not(feature = "rocm"))]
            {
                let _ = (package, device_ordinal, memory_limit_bytes);
                (
                    Err(BackendUnavailable::new(
                        DeviceKind::Rocm,
                        "the packaged worker was built without the ROCm integration feature",
                    )),
                    None,
                )
            }
        }
        WorkerBackendSelection::Metal { package } => {
            #[cfg(feature = "metal")]
            {
                let cancellation = CancellationToken::default();
                let initialized =
                    comfy_runtime::initialize_certified_metal_runtime(&package, &cancellation)
                        .and_then(|certified| {
                            let host_physical_memory_bytes =
                                certified.host_physical_memory_bytes();
                            let runtime = certified.into_runtime();
                            comfy_tensor::MetalTensorBackend::from_certified_runtime(
                                runtime,
                                host_physical_memory_bytes,
                                memory_limit_bytes,
                                &cancellation,
                            )
                            .map_err(|_| {
                                BackendUnavailable::new(
                                    DeviceKind::Metal,
                                    "certified Metal tensor backend initialization failed",
                                )
                            })
                        })
                        .and_then(|(backend, authority)| {
                            WorkerBackendSession::new(Arc::new(backend), authority).map_err(|_| {
                                BackendUnavailable::new(
                                    DeviceKind::Metal,
                                    "workspace, buffer, transfer, command-buffer, or event readiness probe failed",
                                )
                            })
                        });
                (initialized, None)
            }
            #[cfg(not(feature = "metal"))]
            {
                let _ = (package, memory_limit_bytes);
                (
                    Err(BackendUnavailable::new(
                        DeviceKind::Metal,
                        "the packaged worker was built without the Metal integration feature",
                    )),
                    None,
                )
            }
        }
        WorkerBackendSelection::Mlu {
            package,
            device_ordinal,
        } => {
            #[cfg(feature = "mlu")]
            {
                let cancellation = CancellationToken::default();
                let initialized =
                    comfy_runtime::initialize_certified_mlu_runtime(&package, &cancellation)
                        .and_then(|runtime| {
                            comfy_tensor::MluTensorBackend::from_certified_runtime(
                                runtime,
                                device_ordinal,
                                memory_limit_bytes,
                                &cancellation,
                            )
                            .map_err(|_| {
                                BackendUnavailable::new(
                                    DeviceKind::Mlu,
                                    "certified MLU tensor backend initialization failed",
                                )
                            })
                        })
                        .and_then(|(backend, authority)| {
                            WorkerBackendSession::new(Arc::new(backend), authority).map_err(|_| {
                                BackendUnavailable::new(
                                    DeviceKind::Mlu,
                                    "workspace, allocation, transfer, kernel, event, or accounting readiness probe failed",
                                )
                            })
                        });
                (initialized, None)
            }
            #[cfg(not(feature = "mlu"))]
            {
                let _ = (package, device_ordinal, memory_limit_bytes);
                (
                    Err(BackendUnavailable::new(
                        DeviceKind::Mlu,
                        "the packaged worker was built without the MLU integration feature",
                    )),
                    None,
                )
            }
        }
        WorkerBackendSelection::Npu {
            package,
            device_ordinal,
        } => {
            #[cfg(feature = "npu")]
            {
                let cancellation = CancellationToken::default();
                let initialized = comfy_runtime::initialize_certified_npu_runtime(
                    &package,
                    device_ordinal,
                    &cancellation,
                )
                        .and_then(|runtime| {
                            comfy_tensor::NpuTensorBackend::from_certified_runtime(
                                runtime,
                                device_ordinal,
                                memory_limit_bytes,
                                &cancellation,
                            )
                            .map_err(|_| {
                                BackendUnavailable::new(
                                    DeviceKind::Npu,
                                    "certified NPU tensor backend initialization failed",
                                )
                            })
                        })
                        .and_then(|(backend, authority)| {
                            WorkerBackendSession::new(Arc::new(backend), authority).map_err(|_| {
                                BackendUnavailable::new(
                                    DeviceKind::Npu,
                                    "workspace, allocation, transfer, kernel, event, or accounting readiness probe failed",
                                )
                            })
                        });
                (initialized, None)
            }
            #[cfg(not(feature = "npu"))]
            {
                let _ = (package, device_ordinal, memory_limit_bytes);
                (
                    Err(BackendUnavailable::new(
                        DeviceKind::Npu,
                        "the packaged worker was built without the NPU integration feature",
                    )),
                    None,
                )
            }
        }
        WorkerBackendSelection::Cuda {
            package,
            device_ordinal,
        } => {
            #[cfg(feature = "cuda")]
            {
                let cancellation = CancellationToken::default();
                let initialized = usize::try_from(device_ordinal)
                    .map_err(|_| {
                        BackendUnavailable::new(
                            DeviceKind::Cuda,
                            "CUDA device ordinal is not representable on this worker",
                        )
                    })
                    .and_then(|device_ordinal| {
                        comfy_runtime::initialize_certified_cuda_runtime(
                            &package,
                            device_ordinal,
                            &cancellation,
                        )
                    })
                    .and_then(|session| {
                        comfy_tensor::CudaTensorBackend::from_certified_session(
                            session,
                            memory_limit_bytes,
                            &cancellation,
                        )
                        .map_err(|_| {
                            BackendUnavailable::new(
                                DeviceKind::Cuda,
                                "certified CUDA tensor backend initialization failed",
                            )
                        })
                    })
                    .and_then(|(backend, authority)| {
                        WorkerBackendSession::new(Arc::new(backend), authority).map_err(|_| {
                            BackendUnavailable::new(
                                DeviceKind::Cuda,
                                "workspace, allocation, transfer, kernel, event, or accounting readiness probe failed",
                            )
                        })
                    });
                (initialized, None)
            }
            #[cfg(not(feature = "cuda"))]
            {
                let _ = (package, device_ordinal, memory_limit_bytes);
                (
                    Err(BackendUnavailable::new(
                        DeviceKind::Cuda,
                        "the packaged worker was built without the CUDA integration feature",
                    )),
                    None,
                )
            }
        }
        WorkerBackendSelection::Xpu {
            package,
            device_ordinal,
        } => {
            #[cfg(feature = "xpu")]
            {
                let cancellation = CancellationToken::default();
                let initialized = usize::try_from(device_ordinal)
                    .map_err(|_| {
                        BackendUnavailable::new(
                            DeviceKind::Xpu,
                            "XPU device ordinal is not representable on this worker",
                        )
                    })
                    .and_then(|device_ordinal| {
                        comfy_runtime::initialize_certified_xpu_runtime(
                            &package,
                            device_ordinal,
                            &cancellation,
                        )
                    })
                    .and_then(|session| {
                    comfy_tensor::XpuTensorBackend::from_certified_session(
                        session,
                        memory_limit_bytes,
                        &cancellation,
                    )
                    .map_err(|_| {
                        BackendUnavailable::new(
                            DeviceKind::Xpu,
                            "certified XPU tensor backend initialization failed",
                        )
                    })
                })
                .and_then(|(backend, authority)| {
                    WorkerBackendSession::new(Arc::new(backend), authority).map_err(|_| {
                        BackendUnavailable::new(
                            DeviceKind::Xpu,
                            "workspace, allocation, transfer, kernel, event, or accounting readiness probe failed",
                        )
                    })
                });
                (initialized, None)
            }
            #[cfg(not(feature = "xpu"))]
            {
                let _ = (package, device_ordinal, memory_limit_bytes);
                (
                    Err(BackendUnavailable::new(
                        DeviceKind::Xpu,
                        "the packaged worker was built without the XPU integration feature",
                    )),
                    None,
                )
            }
        }
    }
}

async fn run_worker_process_with_configuration(
    memory_limit_bytes: u64,
    backend_selection: WorkerBackendSelection,
    diffusion_provider: Option<Arc<dyn NativeDiffusionProvider>>,
    plugin_authorization_verifier: Option<PluginAuthorizationVerifier>,
) -> anyhow::Result<()> {
    let (backend_session, cpu_executor_backend) =
        initialize_worker_backend(backend_selection, memory_limit_bytes);
    let mut session = WorkerSession::with_backend_result(backend_session);
    let (input_sender, input_receiver) = async_channel::bounded(8);
    let reader_thread = std::thread::Builder::new()
        .name("comfy-worker-stdin".to_owned())
        .spawn(move || {
            let stdin = io::stdin();
            loop {
                let frame = read_frame(stdin.lock());
                let terminal = frame.is_err();
                if input_sender.send_blocking(frame).is_err() || terminal {
                    break;
                }
            }
        })?;

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut next_heartbeat = Instant::now() + HEARTBEAT_INTERVAL;
    let mut active_execution: Option<ActiveExecution> = None;
    let mut native_image_executor: Option<NativeImageExecutor> = None;
    let mut plugin_registry: Option<CommittedPluginRegistry> = None;
    let mut pending_plugin_capabilities: BTreeMap<u64, async_channel::Sender<Vec<u8>>> =
        BTreeMap::new();
    'worker: loop {
        let delay = next_heartbeat.saturating_duration_since(Instant::now());
        let next = smol::future::race(
            async { NextWorkerInput::Frame(input_receiver.recv().await) },
            smol::future::race(
                async {
                    if session.heartbeat_enabled() {
                        worker_delay(delay).await;
                        NextWorkerInput::Heartbeat
                    } else {
                        pending().await
                    }
                },
                smol::future::race(
                    async {
                        if let Some(active) = &active_execution {
                            if let Some(result) = active.native_result.as_ref() {
                                NextWorkerInput::NativeJob(result.recv().await)
                            } else {
                                pending().await
                            }
                        } else {
                            pending().await
                        }
                    },
                    smol::future::race(
                        async {
                            if let Some(result) = active_execution
                                .as_ref()
                                .and_then(|active| active.plugin_result.as_ref())
                            {
                                NextWorkerInput::PluginJob(result.recv().await)
                            } else {
                                pending().await
                            }
                        },
                        smol::future::race(
                            async {
                                if let Some(requests) = active_execution
                                    .as_ref()
                                    .and_then(|active| active.plugin_capabilities.as_ref())
                                {
                                    NextWorkerInput::PluginCapability(requests.recv().await)
                                } else {
                                    pending().await
                                }
                            },
                            async {
                                if let Some(events) = active_execution
                                    .as_ref()
                                    .and_then(|active| active.events.as_ref())
                                {
                                    NextWorkerInput::JobEvent(events.recv().await)
                                } else {
                                    pending().await
                                }
                            },
                        ),
                    ),
                ),
            ),
        )
        .await;

        match next {
            NextWorkerInput::Heartbeat => {
                if let Some(heartbeat) = session.heartbeat()? {
                    write_frame(&mut stdout, &heartbeat)?;
                }
                next_heartbeat = Instant::now() + HEARTBEAT_INTERVAL;
            }
            NextWorkerInput::Frame(Ok(Ok(envelope))) => {
                let handled = if matches!(
                    &envelope.message,
                    comfy_types::WorkerMessage::RegistryDeploymentCommit { .. }
                ) {
                    let authorization_verifier = plugin_authorization_verifier.as_ref();
                    let mut compiled_candidate = None;
                    let responses =
                        session.handle_verified_registry_commit(envelope.clone(), |source| {
                            let candidate = plugin_runtime::WorkerPluginRegistry::from_assembled(
                                envelope.profile_id,
                                source,
                                comfy_plugin_host::ComponentLimits::default(),
                                authorization_verifier.ok_or_else(|| {
                                    WorkerRegistryDeploymentRejectionReason::VerificationUnavailable
                                })?,
                            )
                            .map_err(|error| error.deployment_rejection_reason())?;
                            compiled_candidate = Some(Arc::new(candidate));
                            Ok(())
                        });
                    if let Ok(responses) = &responses {
                        apply_compiled_registry_commit(
                            &mut plugin_registry,
                            responses,
                            session.registry().cloned().map(Arc::new),
                            compiled_candidate,
                        )?;
                    }
                    responses
                } else {
                    session.handle(envelope.clone())
                };
                match handled {
                    Ok(responses) => {
                        for response in responses {
                            write_frame(&mut stdout, &response)?;
                        }
                        match &envelope.message {
                            comfy_types::WorkerMessage::Execute { plan } => {
                                let worker_plan: NativeImageWorkerPlan =
                                    serde_json::from_slice(plan).map_err(|error| {
                                        anyhow::anyhow!("invalid native image worker plan: {error}")
                                    })?;
                                worker_plan.validate()?;
                                if let Some(unavailable) =
                                    backend_neutral_executor_unavailable(session.backend_device())
                                {
                                    let event =
                                        NativeImageWorkerEvent::BackendUnavailable { unavailable };
                                    let encoded = postcard::to_stdvec(&event)?;
                                    let response = session.complete_execution(encoded)?;
                                    write_frame(&mut stdout, &response)?;
                                    continue 'worker;
                                }
                                let (mut memory, memory_configuration) =
                                    match prepare_native_image_memory(&session, &worker_plan) {
                                        Ok(memory) => memory,
                                        Err(error) => {
                                            let event = NativeImageWorkerEvent::Failed {
                                                message: format!(
                                                    "native memory preflight failed without dispatch: {error}"
                                                ),
                                                cancelled: false,
                                            };
                                            let encoded = postcard::to_stdvec(&event)?;
                                            let response = session.complete_execution(encoded)?;
                                            write_frame(&mut stdout, &response)?;
                                            continue 'worker;
                                        }
                                    };
                                let requires_diffusion =
                                    worker_plan.plan.nodes.values().any(|node| {
                                        matches!(
                                            node.class_type.as_str(),
                                            "CheckpointLoaderSimple"
                                                | "CLIPTextEncode"
                                                | "EmptyLatentImage"
                                                | "KSampler"
                                                | "VAEDecode"
                                        )
                                    });
                                let reuse_executor =
                                    native_image_executor.as_ref().is_some_and(|executor| {
                                        executor.profile_id() == envelope.profile_id
                                            && executor.metadata_enabled()
                                                == worker_plan.metadata_enabled
                                            && executor.diffusion_enabled() == requires_diffusion
                                    });
                                let executor_update = if reuse_executor {
                                    native_image_executor
                                        .as_ref()
                                        .ok_or_else(|| {
                                            NativeImageRuntimeError::Execution(
                                                "native executor vanished".to_owned(),
                                            )
                                        })
                                        .and_then(|executor| {
                                            executor.replace_input_assets(
                                                worker_plan.input_assets.clone(),
                                            )
                                        })
                                } else {
                                    let created = if requires_diffusion {
                                        diffusion_provider
                                        .clone()
                                        .ok_or_else(|| {
                                            NativeImageRuntimeError::Execution(
                                                "native diffusion plan has no admitted model provider"
                                                    .to_owned(),
                                            )
                                        })
                                        .and_then(|provider| {
                                            NativeImageExecutor::new_with_diffusion_provider(
                                                envelope.profile_id,
                                                worker_plan.input_assets.clone(),
                                                worker_plan.metadata_enabled,
                                                cpu_executor_backend.clone().ok_or_else(|| {
                                                    NativeImageRuntimeError::Execution(
                                                        "CPU executor adapter is unavailable for the selected backend"
                                                            .to_owned(),
                                                    )
                                                })?,
                                                provider,
                                            )
                                        })
                                    } else {
                                        NativeImageExecutor::new_with_cpu_backend(
                                            envelope.profile_id,
                                            worker_plan.input_assets.clone(),
                                            worker_plan.metadata_enabled,
                                            cpu_executor_backend.clone().ok_or_else(|| {
                                                NativeImageRuntimeError::Execution(
                                                    "CPU executor adapter is unavailable for the selected backend"
                                                        .to_owned(),
                                                )
                                            })?,
                                        )
                                    };
                                    created.map(|executor| {
                                        native_image_executor = Some(executor);
                                    })
                                };
                                if let Err(error) = executor_update {
                                    let event = NativeImageWorkerEvent::Failed {
                                        message: format!(
                                            "native executor setup failed without dispatch: {error}"
                                        ),
                                        cancelled: false,
                                    };
                                    let encoded = postcard::to_stdvec(&event)?;
                                    let response = session.complete_execution(encoded)?;
                                    write_frame(&mut stdout, &response)?;
                                    continue 'worker;
                                }
                                let planned_workspace = memory.issue_workspace_authorization()?;
                                let workspace_authorization =
                                    session.authorize_planned_workspace(planned_workspace)?;
                                memory.begin()?;
                                let current = native_image_executor
                                    .as_ref()
                                    .ok_or_else(|| anyhow::anyhow!("native executor unavailable"))?
                                    .clone();
                                let attempt_id = envelope.attempt_id.ok_or_else(|| {
                                    anyhow::anyhow!("execute omitted attempt identity")
                                })?;
                                let cancellation = CancellationToken::default();
                                let event_bus = ExecutionEventBus::new(32)?;
                                let events = event_bus.subscribe();
                                let (result_sender, result) = async_channel::bounded(1);
                                let cancellation_for_job = cancellation.clone();
                                smol::spawn(async move {
                                let result = smol::unblock(move || {
                                    current.execute_blocking_with_event_bus_and_configuration(
                                        &worker_plan.plan,
                                        attempt_id,
                                        cancellation_for_job,
                                        worker_plan.injected_delay_millis,
                                        Some(event_bus),
                                        workspace_authorization,
                                        &memory_configuration,
                                    )
                                })
                                .await;
                                if let Err(error) = result_sender.send(result).await {
                                    eprintln!(
                                        "comfy-worker: execution result receiver closed: {error}"
                                    );
                                }
                            })
                            .detach();
                                active_execution = Some(ActiveExecution {
                                    cancellation,
                                    memory: Some(memory),
                                    native_result: Some(result),
                                    plugin_result: None,
                                    plugin_capabilities: None,
                                    events: Some(events),
                                });
                            }
                            comfy_types::WorkerMessage::ExecutePlugin { invocation } => {
                                let invocation =
                                    comfy_plugin_host::WorkerPluginInvocation::from_bytes(
                                        invocation,
                                    )?;
                                let component_limits = invocation.component_limits().clone();
                                let reuse_registry =
                                    plugin_registry.as_ref().is_some_and(|registry| {
                                        registry.compiled.uses_component_limits(&component_limits)
                                    });
                                if !reuse_registry {
                                    let registry = plugin_registry.as_mut().ok_or_else(|| {
                                        anyhow::anyhow!("worker plugin registry is unavailable")
                                    })?;
                                    let candidate = plugin_runtime::WorkerPluginRegistry::from_assembled(
                                        envelope.profile_id,
                                        &registry.source,
                                        component_limits,
                                        plugin_authorization_verifier.as_ref().ok_or_else(|| {
                                            anyhow::anyhow!(
                                                "worker plugin authorization verifier is unavailable"
                                            )
                                        })?,
                                    )?;
                                    registry.compiled = Arc::new(candidate);
                                }
                                let registry = plugin_registry
                                    .as_ref()
                                    .ok_or_else(|| {
                                        anyhow::anyhow!("worker plugin registry is unavailable")
                                    })?
                                    .compiled
                                    .clone();
                                let cancellation = CancellationToken::default();
                                let (capability_sender, plugin_capabilities) =
                                    async_channel::bounded(8);
                                let bridge = Arc::new(plugin_runtime::WorkerCapabilityBridge::new(
                                    capability_sender,
                                ));
                                let (result_sender, plugin_result) = async_channel::bounded(1);
                                let cancellation_for_job = cancellation.clone();
                                smol::spawn(async move {
                                    let result = smol::unblock(move || {
                                        registry.execute(invocation, bridge, cancellation_for_job)
                                    })
                                    .await;
                                    let outcome = plugin_runtime::encode_plugin_outcome(result);
                                    if let Err(error) = result_sender.send(outcome).await {
                                        eprintln!(
                                            "comfy-worker: plugin result receiver closed: {error}"
                                        );
                                    }
                                })
                                .detach();
                                active_execution = Some(ActiveExecution {
                                    cancellation,
                                    memory: None,
                                    native_result: None,
                                    plugin_result: Some(plugin_result),
                                    plugin_capabilities: Some(plugin_capabilities),
                                    events: None,
                                });
                            }
                            comfy_types::WorkerMessage::PluginCapabilityResponse {
                                call_id,
                                response,
                            } => {
                                let response_sender = pending_plugin_capabilities
                                    .remove(call_id)
                                    .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "worker received an unknown plugin capability response"
                                    )
                                })?;
                                response_sender.send(response.clone()).await.map_err(|_| {
                                    anyhow::anyhow!(
                                        "worker plugin capability response receiver was closed"
                                    )
                                })?;
                            }
                            comfy_types::WorkerMessage::Cancel { .. } => {
                                let active = active_execution.as_mut().ok_or_else(|| {
                                    anyhow::anyhow!("cancellation has no active execution")
                                })?;
                                if let Some(memory) = active.memory.as_mut() {
                                    memory.cancel()?;
                                }
                                if apply_worker_control_cancellation(
                                    &envelope.message,
                                    &active.cancellation,
                                )
                                .is_none()
                                {
                                    return Err(anyhow::anyhow!(
                                        "worker cancellation branch received a non-control message"
                                    ));
                                }
                            }
                            comfy_types::WorkerMessage::Shutdown => {
                                if let Some(active) = &mut active_execution {
                                    if let Some(memory) = active.memory.as_mut()
                                        && matches!(
                                            memory.state(),
                                            AttemptMemoryState::Planned
                                                | AttemptMemoryState::Running
                                        )
                                    {
                                        memory.cancel()?;
                                    }
                                    if apply_worker_control_cancellation(
                                        &envelope.message,
                                        &active.cancellation,
                                    )
                                    .is_none()
                                    {
                                        return Err(anyhow::anyhow!(
                                            "worker shutdown branch received a non-control message"
                                        ));
                                    }
                                }
                            }
                            _ => {}
                        }
                        if session.is_terminal() {
                            break;
                        }
                    }
                    Err(error) => {
                        eprintln!("comfy-worker: {error}");
                        let fatal = session.fatal_for(&envelope, &error)?;
                        write_frame(&mut stdout, &fatal)?;
                        return Err(error.into());
                    }
                }
            }
            NextWorkerInput::Frame(Ok(Err(error))) => {
                eprintln!("comfy-worker: {error}");
                return Err(error.into());
            }
            NextWorkerInput::Frame(Err(error)) => {
                return Err(anyhow::anyhow!("worker input channel closed: {error}"));
            }
            NextWorkerInput::NativeJob(Ok(result)) => {
                if let Some(active) = &active_execution
                    && let Some(events) = &active.events
                {
                    while let Ok(event) = events.try_recv() {
                        write_execution_event(&mut session, &mut stdout, event)?;
                    }
                }
                let policy_cancelled = active_execution
                    .as_ref()
                    .and_then(|active| active.memory.as_ref())
                    .is_some_and(|memory| !memory.accepts_value());
                let cancelled = policy_cancelled
                    || matches!(result, Err(NativeImageRuntimeError::Cancelled))
                    || matches!(
                        result.as_ref().map(|value| value.report.state),
                        Ok(AttemptState::Cancelled | AttemptState::Interrupted)
                    );
                if let Some(memory) = active_execution
                    .as_mut()
                    .and_then(|active| active.memory.as_mut())
                {
                    if cancelled {
                        if matches!(
                            memory.state(),
                            AttemptMemoryState::Planned | AttemptMemoryState::Running
                        ) {
                            memory.cancel()?;
                        }
                    } else {
                        match result.as_ref().map(|value| value.report.state) {
                            Ok(AttemptState::Succeeded) => memory.complete()?,
                            Ok(AttemptState::Failed)
                            | Ok(
                                AttemptState::Queued
                                | AttemptState::Running
                                | AttemptState::Cancelling,
                            )
                            | Err(_) => memory.fail()?,
                            Ok(AttemptState::Cancelled | AttemptState::Interrupted) => {
                                memory.cancel()?;
                            }
                        }
                    }
                }
                let event = match result {
                    Ok(_) if policy_cancelled => NativeImageWorkerEvent::Failed {
                        message: "native image execution was cancelled; late values were discarded"
                            .to_owned(),
                        cancelled: true,
                    },
                    Ok(mut result) => {
                        let mut output_proposal_ids =
                            Vec::with_capacity(result.output_proposals.len());
                        for proposal in result.output_proposals.drain(..) {
                            output_proposal_ids.push(proposal.proposal_id());
                            let proposal = proposal.to_worker_proposal()?;
                            let response = session.output_proposal(proposal)?;
                            write_frame(&mut stdout, &response)?;
                        }
                        result.report.outputs.clear();
                        result.report.events.clear();
                        NativeImageWorkerEvent::Completed {
                            result: comfy_runtime::NativeImageWorkerResult::from_execution_report(
                                result.report,
                                output_proposal_ids,
                                result.executed_node_count,
                            )?,
                        }
                    }
                    Err(error) => NativeImageWorkerEvent::Failed {
                        message: error.to_string(),
                        cancelled,
                    },
                };
                let encoded = postcard::to_stdvec(&event)?;
                let response = session.complete_execution(encoded)?;
                write_frame(&mut stdout, &response)?;
                active_execution = None;
            }
            NextWorkerInput::NativeJob(Err(error)) => {
                return Err(anyhow::anyhow!("worker execution channel closed: {error}"));
            }
            NextWorkerInput::PluginJob(Ok(outcome)) => {
                if matches!(outcome, WorkerPluginExecutionOutcome::Succeeded(_))
                    && !pending_plugin_capabilities.is_empty()
                {
                    return Err(anyhow::anyhow!(
                        "plugin execution finished with capability calls still pending"
                    ));
                }
                pending_plugin_capabilities.clear();
                let response = session.complete_plugin_execution(outcome)?;
                write_frame(&mut stdout, &response)?;
                active_execution = None;
            }
            NextWorkerInput::PluginJob(Err(error)) => {
                return Err(anyhow::anyhow!(
                    "worker plugin execution channel closed: {error}"
                ));
            }
            NextWorkerInput::PluginCapability(Ok(request)) => {
                if pending_plugin_capabilities
                    .insert(request.call_id, request.response_sender)
                    .is_some()
                {
                    return Err(anyhow::anyhow!(
                        "worker plugin repeated a capability call identifier"
                    ));
                }
                let response =
                    session.plugin_capability_request(request.call_id, request.request)?;
                write_frame(&mut stdout, &response)?;
            }
            NextWorkerInput::PluginCapability(Err(_)) => {
                if active_execution
                    .as_ref()
                    .and_then(|active| active.plugin_result.as_ref())
                    .is_none()
                {
                    return Err(anyhow::anyhow!(
                        "worker plugin capability channel closed before completion"
                    ));
                }
                if let Some(active) = &mut active_execution {
                    active.plugin_capabilities = None;
                }
            }
            NextWorkerInput::JobEvent(Ok(event)) => {
                write_execution_event(&mut session, &mut stdout, event)?;
            }
            NextWorkerInput::JobEvent(Err(_)) => {
                if let Some(active) = &mut active_execution {
                    active.events = None;
                }
            }
        }
    }

    // The reader owns a blocking stdin lock; process exit closes it after the
    // supervisor receives the shutdown acknowledgement.
    drop(reader_thread);
    Ok(())
}

fn backend_neutral_executor_unavailable(device: Option<DeviceId>) -> Option<BackendUnavailable> {
    let device = device?;
    if device == DeviceId::CPU {
        return None;
    }
    Some(BackendUnavailable::new(
        device.kind(),
        "the selected instance has no backend-neutral native image/diffusion executor binding for this graph; CPU fallback is forbidden",
    ))
}

fn prepare_native_image_memory(
    session: &WorkerSession,
    worker_plan: &NativeImageWorkerPlan,
) -> anyhow::Result<(AttemptMemoryController, String)> {
    let input_asset_bytes = worker_plan
        .input_assets
        .values()
        .try_fold(0_u64, |total, bytes| {
            let bytes = u64::try_from(bytes.len())
                .map_err(|_| anyhow::anyhow!("native image input byte accounting overflowed"))?;
            total
                .checked_add(bytes)
                .ok_or_else(|| anyhow::anyhow!("native image input byte accounting overflowed"))
        })?;
    let node_count = u64::try_from(worker_plan.plan.nodes.len())
        .map_err(|_| anyhow::anyhow!("native image node accounting overflowed"))?;
    let memory_request =
        native_image_memory_request(input_asset_bytes, node_count, worker_plan.metadata_enabled)?;
    let memory_snapshot = session.memory_snapshot()?;
    let backend = session
        .accepted_backend()
        .ok_or_else(|| anyhow::anyhow!("native memory preflight has no accepted backend"))?;
    let effective_mode = EffectiveMemoryMode::resolve(
        MemoryModeRequest::from_runtime_policy(worker_plan.memory_policy),
        MemoryModeCapabilities::from_backend(backend),
    )?;
    let memory_configuration = effective_mode.configuration_token();
    let controller = AttemptMemoryController::new(
        memory_snapshot.limit_bytes,
        memory_snapshot.current_bytes,
        memory_request,
    )
    .map_err(|error| {
        anyhow::anyhow!("{error}; effective native memory configuration is {memory_configuration}")
    })?;
    Ok((controller, memory_configuration))
}

fn write_execution_event(
    session: &mut WorkerSession,
    stdout: &mut impl Write,
    event: AttemptEvent,
) -> anyhow::Result<()> {
    let kind = match event.kind {
        AttemptEventKind::Started => NativeImageWorkerProgressKind::Started,
        AttemptEventKind::Progress { completed, total } => {
            NativeImageWorkerProgressKind::Progress { completed, total }
        }
        AttemptEventKind::CacheHit => NativeImageWorkerProgressKind::CacheHit,
        AttemptEventKind::OutputPrepared { transaction_id } => {
            NativeImageWorkerProgressKind::OutputPrepared { transaction_id }
        }
        AttemptEventKind::Preview { .. }
        | AttemptEventKind::OutputAvailable { .. }
        | AttemptEventKind::CancelRequested { .. }
        | AttemptEventKind::Succeeded
        | AttemptEventKind::Failed { .. }
        | AttemptEventKind::Cancelled
        | AttemptEventKind::Interrupted { .. }
        | AttemptEventKind::RecoveryInterrupted { .. } => return Ok(()),
    };
    let progress = NativeImageWorkerProgress {
        profile_id: event.profile_id,
        prompt_id: event.prompt_id,
        attempt_id: event.attempt_id,
        sequence: event.sequence,
        node_id: event.node_id,
        kind,
    };
    let event = postcard::to_stdvec(&NativeImageWorkerEvent::Progress { progress })?;
    let response = session.execution_event(event)?;
    write_frame(stdout, &response)?;
    Ok(())
}

// The separately packaged worker has no GPUI context, so its protocol clock
// has one explicit async timer owner.
#[allow(clippy::disallowed_methods)]
async fn worker_delay(duration: Duration) {
    smol::Timer::after(duration).await;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use comfy_types::{
        ProfileId, RequestId, WORKER_PROTOCOL_VERSION, WorkerId, WorkerMessage,
        WorkerRegistryDeploymentRejection, WorkerRegistryDeploymentRejectionReason,
        WorkerRegistryGeneration, WorkerSha256Digest,
    };

    use super::*;

    fn envelope(message: WorkerMessage) -> WorkerEnvelope {
        WorkerEnvelope {
            version: WORKER_PROTOCOL_VERSION,
            profile_id: ProfileId(Default::default()),
            worker_id: WorkerId(Default::default()),
            request_id: RequestId(Default::default()),
            prompt_id: None,
            attempt_id: None,
            sequence: 0,
            registry_version: "registry-v1".to_owned(),
            message,
            extensions: BTreeMap::new(),
        }
    }

    #[test]
    fn canonical_frame_round_trip_uses_little_endian_length() {
        let original = envelope(WorkerMessage::Ready);
        let mut frame = Vec::new();
        write_frame(&mut frame, &original).expect("encode frame");
        let payload_length = frame.len() - 4;
        assert_eq!(
            frame.get(..4),
            Some((payload_length as u32).to_le_bytes().as_slice())
        );
        assert_eq!(
            read_frame(frame.as_slice()).expect("decode frame"),
            original
        );
    }

    #[test]
    fn oversized_prefix_is_rejected_before_payload_allocation() {
        let prefix = u32::try_from(MAX_WORKER_FRAME_BYTES + 1)
            .expect("frame limit fits u32")
            .to_le_bytes();
        assert!(matches!(
            read_frame(prefix.as_slice()),
            Err(FrameError::TooLarge)
        ));
    }

    #[test]
    fn encoded_event_limit_is_independent_from_frame_limit() {
        let oversized = envelope(WorkerMessage::Event {
            event: vec![0; MAX_ENCODED_PREVIEW_BYTES + 1],
        });
        assert!(matches!(
            write_frame(Vec::new(), &oversized),
            Err(FrameError::PreviewTooLarge)
        ));
    }

    #[test]
    fn val_cancel_001_worker_control_uses_canonical_state() {
        let cancellation = CancellationToken::default();
        let clone = cancellation.clone();
        let cancel = WorkerMessage::Cancel {
            reason: "operator".to_owned(),
        };

        assert_eq!(
            apply_worker_control_cancellation(&cancel, &cancellation),
            Some(true)
        );
        assert!(clone.is_cancelled());
        assert_eq!(
            apply_worker_control_cancellation(&WorkerMessage::Shutdown, &clone),
            Some(false)
        );
        assert_eq!(
            apply_worker_control_cancellation(&WorkerMessage::Heartbeat, &clone),
            None
        );
    }

    #[test]
    fn selected_accelerators_fail_before_cpu_executor_or_memory_preflight() {
        assert!(backend_neutral_executor_unavailable(Some(DeviceId::CPU)).is_none());
        for device in [
            DeviceId::new(DeviceKind::Metal, 0),
            DeviceId::new(DeviceKind::Rocm, 2),
            DeviceId::new(DeviceKind::Mlu, 0),
            DeviceId::new(DeviceKind::Npu, 0),
            DeviceId::new(DeviceKind::Cuda, 0),
            DeviceId::new(DeviceKind::Xpu, 0),
            DeviceId::new(DeviceKind::DirectMl, 0),
        ] {
            let unavailable = backend_neutral_executor_unavailable(Some(device))
                .expect("accelerator graph must fail closed");
            assert_eq!(unavailable.device(), device.kind());
            assert!(unavailable.reason().contains("CPU fallback is forbidden"));
        }
        assert!(backend_neutral_executor_unavailable(None).is_none());

        let source = include_str!("comfy_worker.rs");
        let rejection = source
            .find("backend_neutral_executor_unavailable(session.backend_device())")
            .expect("selected backend rejection is present");
        let memory = source
            .find("prepare_native_image_memory(&session, &worker_plan)")
            .expect("memory preflight is present");
        let executor = source
            .find("NativeImageExecutor::new_with_diffusion_provider")
            .expect("CPU executor construction is present");
        assert!(rejection < memory);
        assert!(rejection < executor);
    }

    #[cfg(feature = "mlu")]
    #[test]
    fn unavailable_mlu_selection_never_constructs_a_cpu_fallback() {
        let package = comfy_runtime::NativeMluPackageSettings::from_public_authority(
            "/missing/reviewed-mlu-package",
            "mlu.release",
            &"44".repeat(32),
        )
        .expect("bounded public MLU authority");
        let (session, cpu_backend) = initialize_worker_backend(
            WorkerBackendSelection::Mlu {
                package,
                device_ordinal: 0,
            },
            4096,
        );
        let unavailable = match session {
            Ok(_) => panic!("missing MLU trust package must fail closed"),
            Err(unavailable) => unavailable,
        };
        assert_eq!(unavailable.device(), DeviceKind::Mlu);
        assert!(cpu_backend.is_none());
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn unavailable_cuda_selection_never_constructs_a_cpu_fallback() {
        let package = comfy_runtime::NativeCudaPackageSettings::from_public_authority(
            "/missing/reviewed-cuda-package",
            "cuda.release",
            &"56".repeat(32),
        )
        .expect("bounded public CUDA authority");
        let (session, cpu_backend) = initialize_worker_backend(
            WorkerBackendSelection::Cuda {
                package,
                device_ordinal: 0,
            },
            4096,
        );
        let unavailable = match session {
            Ok(_) => panic!("missing CUDA trust package must fail closed"),
            Err(unavailable) => unavailable,
        };
        assert_eq!(unavailable.device(), DeviceKind::Cuda);
        assert!(cpu_backend.is_none());
    }

    #[cfg(feature = "npu")]
    #[test]
    fn unavailable_npu_selection_never_constructs_a_cpu_fallback() {
        let package = comfy_runtime::NativeNpuPackageSettings::from_public_authority(
            "/missing/reviewed-npu-package",
            "npu.release",
            &"45".repeat(32),
        )
        .expect("bounded public NPU authority");
        let (session, cpu_backend) = initialize_worker_backend(
            WorkerBackendSelection::Npu {
                package,
                device_ordinal: 0,
            },
            4096,
        );
        let unavailable = match session {
            Ok(_) => panic!("missing NPU trust package must fail closed"),
            Err(unavailable) => unavailable,
        };
        assert_eq!(unavailable.device(), DeviceKind::Npu);
        assert!(cpu_backend.is_none());
    }

    #[cfg(feature = "xpu")]
    #[test]
    fn unavailable_xpu_selection_never_constructs_a_cpu_fallback() {
        let package = comfy_runtime::NativeXpuPackageSettings::from_public_authority(
            "/missing/reviewed-xpu-package",
            "xpu.release",
            &"46".repeat(32),
        )
        .expect("bounded public XPU authority");
        let (session, cpu_backend) = initialize_worker_backend(
            WorkerBackendSelection::Xpu {
                package,
                device_ordinal: 0,
            },
            4096,
        );
        let unavailable = match session {
            Ok(_) => panic!("missing XPU trust package must fail closed"),
            Err(unavailable) => unavailable,
        };
        assert_eq!(unavailable.device(), DeviceKind::Xpu);
        assert!(cpu_backend.is_none());
    }

    #[cfg(feature = "directml")]
    #[test]
    fn unavailable_directml_selection_never_constructs_a_cpu_fallback() {
        let package = comfy_runtime::NativeDirectMlPackageSettings::from_public_authority(
            "/missing/reviewed-directml-package",
            "directml.release",
            &"44".repeat(32),
        )
        .expect("bounded public DirectML authority");
        let (session, cpu_backend) =
            initialize_worker_backend(WorkerBackendSelection::DirectMl { package }, 4096);
        let unavailable = match session {
            Ok(_) => panic!("missing DirectML trust package must fail closed"),
            Err(unavailable) => unavailable,
        };
        assert_eq!(unavailable.device(), DeviceKind::DirectMl);
        assert!(cpu_backend.is_none());
    }

    #[test]
    fn rejected_process_registry_keeps_the_compiled_generation_executable() {
        let profile_id = ProfileId(Default::default());
        let initial_generation = WorkerRegistryGeneration::new(1).expect("nonzero generation");
        let initial_digest = WorkerSha256Digest::new("a".repeat(64)).expect("valid digest");
        let initial_source = Arc::new(AssembledWorkerRegistry::empty_for_test(
            initial_generation,
            initial_digest.clone(),
        ));
        let initial_compiled = Arc::new(
            plugin_runtime::WorkerPluginRegistry::empty_for_test(
                profile_id,
                initial_generation,
                initial_digest.clone(),
            )
            .expect("empty compiled registry"),
        );
        let mut current = Some(CommittedPluginRegistry {
            source: initial_source,
            compiled: initial_compiled,
        });

        let replacement_generation = WorkerRegistryGeneration::new(2).expect("nonzero generation");
        let replacement_digest = WorkerSha256Digest::new("b".repeat(64)).expect("valid digest");
        let replacement_source = Arc::new(AssembledWorkerRegistry::empty_for_test(
            replacement_generation,
            replacement_digest.clone(),
        ));
        let rejection = envelope(WorkerMessage::RegistryDeploymentRejected {
            rejection: WorkerRegistryDeploymentRejection::new(
                replacement_generation,
                replacement_digest,
                WorkerRegistryDeploymentRejectionReason::ComponentCompilationFailed,
            ),
        });
        apply_compiled_registry_commit(&mut current, &[rejection], Some(replacement_source), None)
            .expect("rejection is a non-mutating process transition");

        let current = current.expect("previous registry remains committed");
        assert_eq!(current.source.generation(), initial_generation);
        let invocation = comfy_plugin_host::WorkerPluginInvocation::from_bytes(
            &serde_json::to_vec(&serde_json::json!({
                "registry_generation": initial_generation.get(),
                "registry_digest_sha256": initial_digest.as_str(),
                "extension_id": "missing.extension",
                "component_digest_sha256": "c".repeat(64),
                "authorization_generation": "d".repeat(64),
                "node_id": "missing-node",
                "inputs": { "values": {} },
                "timeout_milliseconds": 1_000,
                "maximum_response_bytes": 1_024,
                "component_limits": comfy_plugin_host::ComponentLimits::default(),
            }))
            .expect("old-generation invocation serializes"),
        )
        .expect("old-generation invocation is valid");
        let (capability_sender, _capability_receiver) = async_channel::bounded(1);
        let result = current.compiled.execute(
            invocation,
            Arc::new(plugin_runtime::WorkerCapabilityBridge::new(
                capability_sender,
            )),
            CancellationToken::default(),
        );
        assert!(matches!(
            result,
            Err(plugin_runtime::WorkerPluginRuntimeError::MissingComponent)
        ));
    }
}
