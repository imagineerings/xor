use crate::{
    CertifiedVideoCodecDependencyClosure, NativeLtxvH264Codec, NativeLtxvH264PreprocessLimits,
    NativeVideoCodecBindingError, NativeVideoCodecLoadError, NativeVideoCodecLtxvAdmissionError,
    NativeVideoCodecLtxvPreprocessError, NativeVideoCodecRuntimeVersions,
    bind_certified_video_codec_abi, load_certified_video_codec_closure,
};
use comfy_nodes::{
    NativeLtxvPreprocessService, NativeLtxvPreprocessServiceError,
    NativeLtxvPreprocessServiceIdentity,
};
use comfy_tensor::{
    CpuBackend, ExecutionContext, ImageTensor, ScratchReservation, StreamId, TensorError,
};
use comfy_types::CancellationToken;
use futures::channel::oneshot;
use futures::future::{BoxFuture, FutureExt};
use sha2::{Digest, Sha256};
use std::{
    fmt, io,
    sync::{Arc, Mutex, TryLockError, mpsc},
    thread::{self, JoinHandle},
};
use thiserror::Error;

const LTXV_CODEC_THREAD_NAME: &str = "comfy-ltxv-codec";
const LTXV_CODEC_THREAD_IDENTITY_VERSION: &str = "sim.comfy.ltxv-codec-thread.v1";

#[allow(
    dead_code,
    reason = "constructed by the following native LTXVPreprocess node-service adapter"
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeLtxvCodecThreadIdentity {
    target: String,
    primary_catalog_sha256: String,
    configuration_sha256: String,
    runtime_versions: NativeVideoCodecRuntimeVersions,
}

#[allow(
    dead_code,
    reason = "consumed by the following native LTXVPreprocess node-service adapter"
)]
impl NativeLtxvCodecThreadIdentity {
    pub(crate) fn target(&self) -> &str {
        &self.target
    }

    pub(crate) fn primary_catalog_sha256(&self) -> &str {
        &self.primary_catalog_sha256
    }

    pub(crate) fn configuration_sha256(&self) -> &str {
        &self.configuration_sha256
    }

    pub(crate) fn runtime_versions(&self) -> NativeVideoCodecRuntimeVersions {
        self.runtime_versions
    }
}

#[allow(
    dead_code,
    reason = "returned through the following native LTXVPreprocess node-service adapter"
)]
#[derive(Debug, Error)]
pub(crate) enum NativeLtxvCodecThreadError {
    #[error("native LTXV codec thread startup or request was cancelled")]
    Cancelled,
    #[error("native LTXV codec thread could not be spawned: {0}")]
    ThreadSpawn(#[source] io::Error),
    #[error("native LTXV codec thread stopped before completing the operation")]
    ThreadStopped,
    #[error("native LTXV codec thread panicked")]
    ThreadPanicked,
    #[error("native LTXV codec thread state was poisoned")]
    StatePoisoned,
    #[error("native LTXV codec request queue is full")]
    Busy,
    #[error("native LTXV codec request carried scratch from the wrong backend: {0}")]
    InvalidScratch(#[source] Box<TensorError>),
    #[error("native LTXV codec request exhausted its reviewed resources")]
    ResourceExhausted,
    #[error("native LTXV codec loading failed: {0}")]
    Load(#[source] Box<NativeVideoCodecLoadError>),
    #[error("native LTXV codec ABI binding failed: {0}")]
    Binding(#[source] Box<NativeVideoCodecBindingError>),
    #[error("native LTXV codec admission failed: {0}")]
    Admission(#[source] Box<NativeVideoCodecLtxvAdmissionError>),
    #[error("native LTXV preprocessing failed: {0}")]
    Preprocess(#[source] Box<NativeVideoCodecLtxvPreprocessError>),
}

struct NativeLtxvCodecThreadRequest {
    invocation: NativeLtxvCodecThreadInvocation,
    response: oneshot::Sender<Result<ImageTensor, NativeLtxvCodecThreadError>>,
}

struct NativeLtxvCodecThreadInvocation {
    image: ImageTensor,
    compression: u8,
    stream: StreamId,
    scratch: ScratchReservation,
    cancellation: CancellationToken,
}

struct NativeLtxvCodecThreadInner {
    identity: NativeLtxvCodecThreadIdentity,
    node_service_identity: NativeLtxvPreprocessServiceIdentity,
    sender: Mutex<Option<mpsc::SyncSender<NativeLtxvCodecThreadRequest>>>,
    runner: Mutex<Option<JoinHandle<()>>>,
}

#[allow(
    dead_code,
    reason = "consumed by the following native LTXVPreprocess node-service adapter"
)]
pub(crate) struct NativeLtxvCodecThreadService {
    inner: Arc<NativeLtxvCodecThreadInner>,
}

#[allow(
    dead_code,
    reason = "consumed by the following native LTXVPreprocess node-service adapter"
)]
#[derive(Clone)]
pub(crate) struct NativeLtxvCodecRequestProxy {
    inner: Arc<NativeLtxvCodecThreadInner>,
}

impl fmt::Debug for NativeLtxvCodecRequestProxy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeLtxvCodecRequestProxy")
            .field("identity", &self.inner.node_service_identity)
            .finish_non_exhaustive()
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following native LTXVPreprocess node-service adapter"
)]
impl NativeLtxvCodecThreadService {
    pub(crate) fn start(
        closure: CertifiedVideoCodecDependencyClosure,
        backend: Arc<CpuBackend>,
        limits: NativeLtxvH264PreprocessLimits,
        startup_cancellation: &CancellationToken,
    ) -> Result<Self, NativeLtxvCodecThreadError> {
        startup_cancellation
            .check()
            .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
        let base_configuration_sha256 = video_codec_thread_base_identity(&closure, limits);
        let startup_cancellation = startup_cancellation.clone();
        start_ltxv_codec_thread(move || {
            startup_cancellation
                .check()
                .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
            let load = load_certified_video_codec_closure(closure, &startup_cancellation)
                .map_err(|error| NativeLtxvCodecThreadError::Load(Box::new(error)))?;
            let binding = bind_certified_video_codec_abi(load, &startup_cancellation)
                .map_err(|error| NativeLtxvCodecThreadError::Binding(Box::new(error)))?;
            let codec = binding
                .admit_ltxv_h264(&startup_cancellation)
                .map_err(|error| NativeLtxvCodecThreadError::Admission(Box::new(error)))?;
            startup_cancellation
                .check()
                .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
            let identity = NativeLtxvCodecThreadIdentity {
                target: codec.target().to_owned(),
                primary_catalog_sha256: codec.primary_catalog_sha256().to_owned(),
                configuration_sha256: finalize_video_codec_thread_identity(
                    &base_configuration_sha256,
                    codec.runtime_versions(),
                ),
                runtime_versions: codec.runtime_versions(),
            };
            let processor = move |request: NativeLtxvCodecThreadInvocation| {
                process_ltxv_codec_request(&codec, &backend, limits, request)
            };
            Ok((identity, processor))
        })
    }

    pub(crate) fn proxy(&self) -> NativeLtxvCodecRequestProxy {
        NativeLtxvCodecRequestProxy {
            inner: self.inner.clone(),
        }
    }

    pub(crate) fn shutdown(self) -> Result<(), NativeLtxvCodecThreadError> {
        self.inner.close()
    }
}

impl Drop for NativeLtxvCodecThreadService {
    fn drop(&mut self) {
        if let Err(error) = self.inner.close() {
            eprintln!("native LTXV codec service cleanup failed: {error}");
        }
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following native LTXVPreprocess node-service adapter"
)]
impl NativeLtxvCodecRequestProxy {
    pub(crate) fn identity(&self) -> &NativeLtxvCodecThreadIdentity {
        &self.inner.identity
    }

    pub(crate) fn preprocess_image(
        &self,
        image: &ImageTensor,
        compression: u8,
        context: &ExecutionContext<'_>,
    ) -> BoxFuture<'static, Result<ImageTensor, NativeLtxvCodecThreadError>> {
        if context.cancellation.check().is_err() {
            return async { Err(NativeLtxvCodecThreadError::Cancelled) }.boxed();
        }
        let sender = match self.inner.sender.try_lock() {
            Ok(sender) => match sender.as_ref() {
                Some(sender) => sender.clone(),
                None => {
                    return async { Err(NativeLtxvCodecThreadError::ThreadStopped) }.boxed();
                }
            },
            Err(TryLockError::WouldBlock) => {
                return async { Err(NativeLtxvCodecThreadError::Busy) }.boxed();
            }
            Err(TryLockError::Poisoned(_)) => {
                return async { Err(NativeLtxvCodecThreadError::StatePoisoned) }.boxed();
            }
        };
        let cancellation = context.cancellation.clone();
        let (response, receiver) = oneshot::channel();
        let request = NativeLtxvCodecThreadRequest {
            invocation: NativeLtxvCodecThreadInvocation {
                image: image.clone(),
                compression,
                stream: context.stream,
                scratch: context.scratch.clone(),
                cancellation: cancellation.clone(),
            },
            response,
        };
        match sender.try_send(request) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(_)) => {
                return async { Err(NativeLtxvCodecThreadError::Busy) }.boxed();
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                return async { Err(NativeLtxvCodecThreadError::ThreadStopped) }.boxed();
            }
        }
        async move {
            let result = receiver
                .await
                .map_err(|_| NativeLtxvCodecThreadError::ThreadStopped)?;
            cancellation
                .check()
                .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
            result
        }
        .boxed()
    }
}

impl NativeLtxvPreprocessService for NativeLtxvCodecRequestProxy {
    fn identity(&self) -> &NativeLtxvPreprocessServiceIdentity {
        &self.inner.node_service_identity
    }

    fn preprocess_image(
        &self,
        image: &ImageTensor,
        compression: u8,
        context: &ExecutionContext<'_>,
    ) -> BoxFuture<'static, Result<ImageTensor, NativeLtxvPreprocessServiceError>> {
        NativeLtxvCodecRequestProxy::preprocess_image(self, image, compression, context)
            .map(|result| result.map_err(map_ltxv_node_service_error))
            .boxed()
    }
}

impl NativeLtxvCodecThreadInner {
    fn close(&self) -> Result<(), NativeLtxvCodecThreadError> {
        let sender = self
            .sender
            .lock()
            .map_err(|_| NativeLtxvCodecThreadError::StatePoisoned)?
            .take();
        drop(sender);
        let runner = self
            .runner
            .lock()
            .map_err(|_| NativeLtxvCodecThreadError::StatePoisoned)?
            .take();
        if let Some(runner) = runner {
            runner
                .join()
                .map_err(|_| NativeLtxvCodecThreadError::ThreadPanicked)?;
        }
        Ok(())
    }
}

impl Drop for NativeLtxvCodecThreadInner {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("native LTXV codec thread cleanup failed: {error}");
        }
    }
}

fn start_ltxv_codec_thread<Factory, Processor>(
    factory: Factory,
) -> Result<NativeLtxvCodecThreadService, NativeLtxvCodecThreadError>
where
    Factory: FnOnce() -> Result<(NativeLtxvCodecThreadIdentity, Processor), NativeLtxvCodecThreadError>
        + Send
        + 'static,
    Processor: FnMut(NativeLtxvCodecThreadInvocation) -> Result<ImageTensor, NativeLtxvCodecThreadError>
        + 'static,
{
    let (sender, receiver) = mpsc::sync_channel(1);
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let runner = thread::Builder::new()
        .name(LTXV_CODEC_THREAD_NAME.to_owned())
        .spawn(move || match factory() {
            Ok((identity, mut processor)) => {
                if ready_sender.send(Ok(identity)).is_err() {
                    eprintln!("native LTXV codec thread readiness receiver was dropped");
                    return;
                }
                while let Ok(request) = receiver.recv() {
                    let NativeLtxvCodecThreadRequest {
                        invocation,
                        response,
                    } = request;
                    let result = processor(invocation);
                    if response.send(result).is_err() {
                        eprintln!("native LTXV codec request receiver was dropped");
                    }
                }
            }
            Err(error) => {
                if ready_sender.send(Err(error)).is_err() {
                    eprintln!("native LTXV codec thread startup error receiver was dropped");
                }
            }
        })
        .map_err(NativeLtxvCodecThreadError::ThreadSpawn)?;
    let identity = match ready_receiver.recv() {
        Ok(Ok(identity)) => identity,
        Ok(Err(error)) => {
            if runner.join().is_err() {
                eprintln!("native LTXV codec thread panicked after reporting startup failure");
            }
            return Err(error);
        }
        Err(_) => {
            return match runner.join() {
                Ok(()) => Err(NativeLtxvCodecThreadError::ThreadStopped),
                Err(_) => Err(NativeLtxvCodecThreadError::ThreadPanicked),
            };
        }
    };
    Ok(NativeLtxvCodecThreadService {
        inner: Arc::new(NativeLtxvCodecThreadInner {
            node_service_identity: NativeLtxvPreprocessServiceIdentity::checked(
                identity.configuration_sha256().to_owned(),
            )
            .map_err(|_| NativeLtxvCodecThreadError::StatePoisoned)?,
            identity,
            sender: Mutex::new(Some(sender)),
            runner: Mutex::new(Some(runner)),
        }),
    })
}

fn map_ltxv_node_service_error(
    error: NativeLtxvCodecThreadError,
) -> NativeLtxvPreprocessServiceError {
    match error {
        NativeLtxvCodecThreadError::Cancelled => NativeLtxvPreprocessServiceError::Cancelled,
        NativeLtxvCodecThreadError::Busy => NativeLtxvPreprocessServiceError::Busy,
        NativeLtxvCodecThreadError::InvalidScratch(_) => {
            NativeLtxvPreprocessServiceError::InvalidRequest
        }
        NativeLtxvCodecThreadError::ResourceExhausted => {
            NativeLtxvPreprocessServiceError::ResourceExhausted
        }
        NativeLtxvCodecThreadError::ThreadStopped
        | NativeLtxvCodecThreadError::ThreadPanicked
        | NativeLtxvCodecThreadError::StatePoisoned => {
            NativeLtxvPreprocessServiceError::Unavailable
        }
        error @ (NativeLtxvCodecThreadError::ThreadSpawn(_)
        | NativeLtxvCodecThreadError::Load(_)
        | NativeLtxvCodecThreadError::Binding(_)
        | NativeLtxvCodecThreadError::Admission(_)
        | NativeLtxvCodecThreadError::Preprocess(_)) => {
            NativeLtxvPreprocessServiceError::Execution(error.to_string())
        }
    }
}

fn process_ltxv_codec_request(
    codec: &NativeLtxvH264Codec,
    backend: &CpuBackend,
    limits: NativeLtxvH264PreprocessLimits,
    request: NativeLtxvCodecThreadInvocation,
) -> Result<ImageTensor, NativeLtxvCodecThreadError> {
    request
        .cancellation
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    backend
        .validate_scratch_reservation(&request.scratch)
        .map_err(|error| NativeLtxvCodecThreadError::InvalidScratch(Box::new(error)))?;
    let context = ExecutionContext {
        stream: request.stream,
        scratch: request.scratch,
        rng_phase: None,
        cancellation: &request.cancellation,
    };
    let output = codec
        .preprocess_image(
            &request.image,
            request.compression,
            limits,
            backend,
            &context,
        )
        .map_err(map_ltxv_thread_preprocess_error)?;
    context
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    Ok(output)
}

fn map_ltxv_thread_preprocess_error(
    error: NativeVideoCodecLtxvPreprocessError,
) -> NativeLtxvCodecThreadError {
    match error {
        NativeVideoCodecLtxvPreprocessError::Cancelled => NativeLtxvCodecThreadError::Cancelled,
        NativeVideoCodecLtxvPreprocessError::ResourceExhausted => {
            NativeLtxvCodecThreadError::ResourceExhausted
        }
        error => NativeLtxvCodecThreadError::Preprocess(Box::new(error)),
    }
}

fn video_codec_thread_base_identity(
    closure: &CertifiedVideoCodecDependencyClosure,
    limits: NativeLtxvH264PreprocessLimits,
) -> String {
    let mut digest = Sha256::new();
    hash_identity_field(&mut digest, LTXV_CODEC_THREAD_IDENTITY_VERSION.as_bytes());
    hash_identity_field(&mut digest, closure.target().as_bytes());
    hash_identity_field(&mut digest, closure.primary_catalog_sha256().as_bytes());
    hash_identity_field(&mut digest, closure.source_archive_sha256().as_bytes());
    for (identity, library) in closure.primary_libraries() {
        hash_identity_field(&mut digest, identity.as_bytes());
        hash_identity_field(&mut digest, library.filename().as_bytes());
        hash_identity_field(&mut digest, library.digest_sha256().as_bytes());
        hash_identity_field(&mut digest, &library.abi_major().to_le_bytes());
    }
    for (identity, dependency) in closure.dependencies() {
        hash_identity_field(&mut digest, identity.as_bytes());
        hash_identity_field(&mut digest, dependency.filename().as_bytes());
        hash_identity_field(&mut digest, dependency.digest_sha256().as_bytes());
        hash_identity_field(&mut digest, dependency.abi_version().as_bytes());
        hash_identity_field(&mut digest, dependency.certificate_sponsor().as_bytes());
    }
    for edge in closure.edges() {
        hash_identity_field(&mut digest, edge.consumer().as_bytes());
        hash_identity_field(&mut digest, edge.dependency().as_bytes());
    }
    for (encoder, provider) in closure.encoder_providers() {
        hash_identity_field(&mut digest, encoder.as_bytes());
        hash_identity_field(&mut digest, provider.as_bytes());
    }
    for identity in closure.dependency_first_order() {
        hash_identity_field(&mut digest, identity.as_bytes());
    }
    hash_identity_field(
        &mut digest,
        &closure.retained_dependency_bytes().to_le_bytes(),
    );
    for value in limits.configuration_values() {
        hash_identity_field(&mut digest, &value.to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn finalize_video_codec_thread_identity(
    base_configuration_sha256: &str,
    versions: NativeVideoCodecRuntimeVersions,
) -> String {
    let mut digest = Sha256::new();
    hash_identity_field(&mut digest, base_configuration_sha256.as_bytes());
    for version in [
        versions.avcodec(),
        versions.avformat(),
        versions.avutil(),
        versions.swresample(),
        versions.swscale(),
    ] {
        hash_identity_field(&mut digest, &version.to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn hash_identity_field(digest: &mut Sha256, value: &[u8]) {
    digest.update(u64::try_from(value.len()).unwrap_or(u64::MAX).to_le_bytes());
    digest.update(value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use comfy_tensor::{BackendWorkspaceAuthority, DType, DeviceId, Layout, TensorDescriptor};
    use futures::executor::block_on;
    use std::{
        rc::Rc,
        sync::{
            Condvar,
            atomic::{AtomicUsize, Ordering},
        },
    };

    fn test_identity(seed: &str) -> NativeLtxvCodecThreadIdentity {
        let versions = NativeVideoCodecRuntimeVersions::from_components(
            0x3d1364, 0x3d0764, 0x3b2764, 0x050364, 0x080364,
        );
        NativeLtxvCodecThreadIdentity {
            target: "x86_64-unknown-linux-gnu".to_owned(),
            primary_catalog_sha256: "11".repeat(32),
            configuration_sha256: finalize_video_codec_thread_identity(seed, versions),
            runtime_versions: versions,
        }
    }

    fn test_image_and_context(
        cancellation: &CancellationToken,
    ) -> Result<(Arc<CpuBackend>, ImageTensor, ScratchReservation), TensorError> {
        let (backend, authority) = BackendWorkspaceAuthority::create_backend(1024 * 1024)?;
        let backend = Arc::new(backend);
        let scratch = authority.authorize_workspace(1024 * 1024)?;
        let context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: scratch.clone(),
            rng_phase: None,
            cancellation,
        };
        let image = ImageTensor::from_f32(&backend, &context, 1, 1, 1, 3, &[0.0, 0.5, 1.0])?;
        Ok((backend, image, scratch))
    }

    fn request_context<'a>(
        scratch: ScratchReservation,
        cancellation: &'a CancellationToken,
    ) -> ExecutionContext<'a> {
        ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch,
            rng_phase: None,
            cancellation,
        }
    }

    #[test]
    fn retained_ltxv_codec_thread_is_send_sync_serial_and_thread_affine()
    -> Result<(), Box<dyn std::error::Error>> {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NativeLtxvCodecRequestProxy>();

        let cancellation = CancellationToken::default();
        let (_backend, image, scratch) = test_image_and_context(&cancellation)?;
        let events = Arc::new(Mutex::new(Vec::new()));
        let drop_events = events.clone();
        let service = start_ltxv_codec_thread({
            let events = events.clone();
            move || {
                let thread_id = thread::current().id();
                events
                    .lock()
                    .map_err(|_| NativeLtxvCodecThreadError::StatePoisoned)?
                    .push(("start", thread_id));
                let thread_bound = Rc::new(ThreadBoundDrop {
                    events: drop_events,
                });
                Ok((
                    test_identity("serial"),
                    move |request: NativeLtxvCodecThreadInvocation| {
                        let _thread_bound = &thread_bound;
                        events
                            .lock()
                            .map_err(|_| NativeLtxvCodecThreadError::StatePoisoned)?
                            .push(("request", thread::current().id()));
                        Ok(request.image)
                    },
                ))
            }
        })?;
        let proxy = service.proxy();
        assert_eq!(proxy.identity().target(), "x86_64-unknown-linux-gnu");
        assert_eq!(
            NativeLtxvPreprocessService::identity(&proxy).configuration_sha256(),
            proxy.identity().configuration_sha256()
        );
        let context = request_context(scratch, &cancellation);
        let first = block_on(proxy.preprocess_image(&image, 0, &context))?;
        let second = block_on(proxy.preprocess_image(&image, 0, &context))?;
        assert_eq!(first.dimensions()?, (1, 1, 1, 3));
        assert_eq!(second.dimensions()?, (1, 1, 1, 3));
        service.shutdown()?;

        let events = events
            .lock()
            .map_err(|_| "thread event mutex was poisoned")?;
        let actor_thread = events.first().ok_or("missing actor start event")?.1;
        assert_ne!(actor_thread, thread::current().id());
        assert_eq!(
            events.iter().filter(|event| event.0 == "request").count(),
            2
        );
        assert!(events.iter().all(|event| event.1 == actor_thread));
        assert_eq!(events.last().map(|event| event.0), Some("drop"));
        Ok(())
    }

    #[test]
    fn retained_ltxv_codec_thread_bounds_queue_cancellation_failure_and_retry()
    -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::default();
        let (_backend, image, scratch) = test_image_and_context(&cancellation)?;
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let started = Arc::new((Mutex::new(false), Condvar::new()));
        let calls = Arc::new(AtomicUsize::new(0));
        let service = start_ltxv_codec_thread({
            let gate = gate.clone();
            let started = started.clone();
            let calls = calls.clone();
            move || {
                Ok((
                    test_identity("bounded"),
                    move |request: NativeLtxvCodecThreadInvocation| {
                        calls.fetch_add(1, Ordering::AcqRel);
                        if request.compression == 99 {
                            return Err(NativeLtxvCodecThreadError::ResourceExhausted);
                        }
                        if request.compression == 98 {
                            request.cancellation.cancel();
                            return Ok(request.image);
                        }
                        let (started_lock, started_condition) = &*started;
                        let mut is_started = started_lock
                            .lock()
                            .map_err(|_| NativeLtxvCodecThreadError::StatePoisoned)?;
                        *is_started = true;
                        started_condition.notify_all();
                        drop(is_started);
                        let (gate_lock, gate_condition) = &*gate;
                        let mut released = gate_lock
                            .lock()
                            .map_err(|_| NativeLtxvCodecThreadError::StatePoisoned)?;
                        while !*released {
                            released = gate_condition
                                .wait(released)
                                .map_err(|_| NativeLtxvCodecThreadError::StatePoisoned)?;
                        }
                        request
                            .cancellation
                            .check()
                            .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
                        Ok(request.image)
                    },
                ))
            }
        })?;
        let proxy = service.proxy();
        let context = request_context(scratch.clone(), &cancellation);
        let first = proxy.preprocess_image(&image, 0, &context);
        {
            let (started_lock, started_condition) = &*started;
            let mut is_started = started_lock
                .lock()
                .map_err(|_| "started mutex was poisoned")?;
            while !*is_started {
                is_started = started_condition
                    .wait(is_started)
                    .map_err(|_| "started mutex was poisoned")?;
            }
        }
        let second = proxy.preprocess_image(&image, 0, &context);
        assert!(matches!(
            block_on(proxy.preprocess_image(&image, 0, &context)),
            Err(NativeLtxvCodecThreadError::Busy)
        ));
        {
            let (gate_lock, gate_condition) = &*gate;
            *gate_lock.lock().map_err(|_| "gate mutex was poisoned")? = true;
            gate_condition.notify_all();
        }
        block_on(first)?;
        block_on(second)?;

        assert!(matches!(
            block_on(proxy.preprocess_image(&image, 99, &context)),
            Err(NativeLtxvCodecThreadError::ResourceExhausted)
        ));
        block_on(proxy.preprocess_image(&image, 0, &context))?;
        let late_cancellation = CancellationToken::default();
        let late_context = request_context(scratch.clone(), &late_cancellation);
        assert!(matches!(
            block_on(proxy.preprocess_image(&image, 98, &late_context)),
            Err(NativeLtxvCodecThreadError::Cancelled)
        ));
        let cancelled = CancellationToken::default();
        cancelled.cancel();
        let cancelled_context = request_context(scratch, &cancelled);
        assert!(matches!(
            block_on(proxy.preprocess_image(&image, 0, &cancelled_context)),
            Err(NativeLtxvCodecThreadError::Cancelled)
        ));
        assert_eq!(calls.load(Ordering::Acquire), 5);
        service.shutdown()?;
        assert!(matches!(
            block_on(proxy.preprocess_image(&image, 0, &context)),
            Err(NativeLtxvCodecThreadError::ThreadStopped)
        ));
        Ok(())
    }

    #[test]
    fn retained_ltxv_codec_thread_startup_and_identity_fail_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(matches!(
            start_ltxv_codec_thread::<_, fn(NativeLtxvCodecThreadInvocation) -> _>(|| {
                Err(NativeLtxvCodecThreadError::Cancelled)
            }),
            Err(NativeLtxvCodecThreadError::Cancelled)
        ));
        let versions = test_identity("first").runtime_versions();
        assert_ne!(
            finalize_video_codec_thread_identity("first", versions),
            finalize_video_codec_thread_identity("second", versions)
        );
        Ok(())
    }

    #[test]
    fn retained_ltxv_codec_thread_node_service_maps_typed_failures() {
        for (error, expected) in [
            (
                NativeLtxvCodecThreadError::Cancelled,
                NativeLtxvPreprocessServiceError::Cancelled,
            ),
            (
                NativeLtxvCodecThreadError::Busy,
                NativeLtxvPreprocessServiceError::Busy,
            ),
            (
                NativeLtxvCodecThreadError::ResourceExhausted,
                NativeLtxvPreprocessServiceError::ResourceExhausted,
            ),
            (
                NativeLtxvCodecThreadError::ThreadStopped,
                NativeLtxvPreprocessServiceError::Unavailable,
            ),
            (
                NativeLtxvCodecThreadError::ThreadPanicked,
                NativeLtxvPreprocessServiceError::Unavailable,
            ),
            (
                NativeLtxvCodecThreadError::StatePoisoned,
                NativeLtxvPreprocessServiceError::Unavailable,
            ),
        ] {
            assert_eq!(
                std::mem::discriminant(&map_ltxv_node_service_error(error)),
                std::mem::discriminant(&expected)
            );
        }
        let execution = map_ltxv_node_service_error(NativeLtxvCodecThreadError::ThreadSpawn(
            io::Error::other("synthetic spawn failure"),
        ));
        assert!(matches!(
            execution,
            NativeLtxvPreprocessServiceError::Execution(message)
                if message.contains("synthetic spawn failure")
        ));
    }

    struct ThreadBoundDrop {
        events: Arc<Mutex<Vec<(&'static str, thread::ThreadId)>>>,
    }

    impl Drop for ThreadBoundDrop {
        fn drop(&mut self) {
            if let Ok(mut events) = self.events.lock() {
                events.push(("drop", thread::current().id()));
            } else {
                eprintln!("thread-bound codec test drop event mutex was poisoned");
            }
        }
    }

    #[test]
    fn retained_ltxv_codec_thread_test_tensor_contract_is_cpu_f32() -> Result<(), TensorError> {
        let cancellation = CancellationToken::default();
        let (_backend, image, _scratch) = test_image_and_context(&cancellation)?;
        let descriptor: &TensorDescriptor = image.tensor().descriptor();
        assert_eq!(descriptor.dtype(), DType::F32);
        assert_eq!(descriptor.device(), DeviceId::CPU);
        assert_eq!(descriptor.layout(), Layout::Contiguous);
        Ok(())
    }
}
