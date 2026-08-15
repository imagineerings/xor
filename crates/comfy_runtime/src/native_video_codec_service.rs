use crate::{
    CertifiedVideoCodecDependencyClosure, NativeLtxvH264PreprocessLimits,
    NativeVideoCodecAv1EncodeError, NativeVideoCodecBindingError, NativeVideoCodecLoadError,
    NativeVideoCodecLtxvAdmissionError, NativeVideoCodecLtxvPreprocessError,
    NativeVideoCodecRuntimeVersions, NativeVideoCodecSuite, NativeVideoCodecSuiteAdmissionError,
    NativeVideoCodecVp9EncodeError, NativeVideoContainerMetadata, NativeVp9WebmBatchLimits,
    bind_certified_video_codec_abi, load_certified_video_codec_closure,
};
use comfy_media::{NativeVideoBitDepth, NativeVideoCrf, NativeVideoPixelFormat};
use comfy_nodes::{
    NativeLtxvPreprocessService, NativeLtxvPreprocessServiceError,
    NativeLtxvPreprocessServiceIdentity,
};
use comfy_tensor::{
    CpuBackend, DType, DeviceId, ExecutionContext, ImageTensor, ScratchReservation, StreamId,
    Tensor, TensorDescriptor, TensorError,
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

const VIDEO_CODEC_THREAD_NAME: &str = "comfy-video-codec";
const VIDEO_CODEC_THREAD_IDENTITY_VERSION: &str = "sim.comfy.video-codec-thread.v7";

#[allow(
    dead_code,
    reason = "constructed by the native LTXVPreprocess adapter and VP9 owned-byte bridge"
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
    reason = "consumed by the native LTXVPreprocess adapter and VP9 owned-byte bridge"
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
    reason = "returned through the native LTXVPreprocess adapter and VP9 owned-byte bridge"
)]
#[derive(Debug, Error)]
pub(crate) enum NativeLtxvCodecThreadError {
    #[error("native video codec thread startup or request was cancelled")]
    Cancelled,
    #[error("native video codec thread could not be spawned: {0}")]
    ThreadSpawn(#[source] io::Error),
    #[error("native video codec thread stopped before completing the operation")]
    ThreadStopped,
    #[error("native video codec thread panicked")]
    ThreadPanicked,
    #[error("native video codec thread state was poisoned")]
    StatePoisoned,
    #[error("native video codec request queue is full")]
    Busy,
    #[error("native video codec request carried scratch from the wrong backend: {0}")]
    InvalidScratch(#[source] Box<TensorError>),
    #[error("native video codec request exhausted its reviewed resources")]
    ResourceExhausted,
    #[error("native video codec loading failed: {0}")]
    Load(#[source] Box<NativeVideoCodecLoadError>),
    #[error("native video codec ABI binding failed: {0}")]
    Binding(#[source] Box<NativeVideoCodecBindingError>),
    #[error("native LTXV codec admission failed: {0}")]
    Admission(#[source] Box<NativeVideoCodecLtxvAdmissionError>),
    #[error("native video codec-suite admission failed: {0}")]
    SuiteAdmission(#[source] Box<NativeVideoCodecSuiteAdmissionError>),
    #[error("native LTXV preprocessing failed: {0}")]
    Preprocess(#[source] Box<NativeVideoCodecLtxvPreprocessError>),
    #[error("native VP9 WebM encoding failed: {0}")]
    Vp9Encode(#[source] Box<NativeVideoCodecVp9EncodeError>),
    #[error("native AV1 WebM encoding failed: {0}")]
    Av1Encode(#[source] Box<NativeVideoCodecAv1EncodeError>),
    #[error("native WebM owned-byte materialization failed: {0}")]
    EncodedOutput(#[source] Box<TensorError>),
}

struct NativeLtxvCodecThreadRequest {
    invocation: NativeLtxvCodecThreadInvocation,
    response: oneshot::Sender<Result<NativeVideoCodecThreadOutput, NativeLtxvCodecThreadError>>,
}

struct NativeLtxvCodecThreadInvocation {
    operation: NativeVideoCodecThreadOperation,
    stream: StreamId,
    scratch: ScratchReservation,
    cancellation: CancellationToken,
}

enum NativeVideoCodecThreadOperation {
    Preprocess {
        image: ImageTensor,
        compression: u8,
    },
    EncodeVp9Webm {
        images: ImageTensor,
        frame_rate: (u64, u64),
        crf: NativeVideoCrf,
        limits: NativeVp9WebmBatchLimits,
        metadata: NativeVideoContainerMetadata,
    },
    EncodeAv1Webm {
        images: ImageTensor,
        frame_rate: (u64, u64),
        crf: NativeVideoCrf,
        limits: NativeVp9WebmBatchLimits,
        metadata: NativeVideoContainerMetadata,
    },
}

enum NativeVideoCodecThreadOutput {
    Image(ImageTensor),
    Vp9Webm(NativeOwnedVp9Webm),
    Av1Webm(NativeOwnedAv1Webm),
}

#[allow(
    dead_code,
    reason = "consumed by the following SaveWEBM prepared-effect adapter"
)]
#[derive(Debug)]
pub(crate) struct NativeOwnedVp9Webm {
    bytes: Tensor,
    content_sha256: [u8; 32],
    width: i32,
    height: i32,
    frame_rate: (i32, i32),
    frame_count: usize,
    has_alpha: bool,
}

#[allow(
    dead_code,
    reason = "consumed by the following SaveWEBM prepared-effect adapter"
)]
impl NativeOwnedVp9Webm {
    pub(crate) fn encoded_bytes(&self) -> Result<&[u8], TensorError> {
        self.bytes.contiguous_bytes()
    }

    pub(crate) fn dimensions(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    pub(crate) fn content_sha256(&self) -> [u8; 32] {
        self.content_sha256
    }

    pub(crate) fn frame_rate(&self) -> (i32, i32) {
        self.frame_rate
    }

    pub(crate) fn frame_count(&self) -> usize {
        self.frame_count
    }

    pub(crate) fn has_alpha(&self) -> bool {
        self.has_alpha
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following SaveWEBM prepared-effect adapter"
)]
#[derive(Debug)]
pub(crate) struct NativeOwnedAv1Webm {
    bytes: Tensor,
    content_sha256: [u8; 32],
    width: i32,
    height: i32,
    frame_rate: (i32, i32),
    frame_count: usize,
}

#[allow(
    dead_code,
    reason = "consumed by the following SaveWEBM prepared-effect adapter"
)]
impl NativeOwnedAv1Webm {
    pub(crate) fn encoded_bytes(&self) -> Result<&[u8], TensorError> {
        self.bytes.contiguous_bytes()
    }

    pub(crate) fn dimensions(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    pub(crate) fn content_sha256(&self) -> [u8; 32] {
        self.content_sha256
    }

    pub(crate) fn frame_rate(&self) -> (i32, i32) {
        self.frame_rate
    }

    pub(crate) fn frame_count(&self) -> usize {
        self.frame_count
    }

    pub(crate) const fn has_alpha(&self) -> bool {
        false
    }

    pub(crate) const fn bit_depth(&self) -> NativeVideoBitDepth {
        NativeVideoBitDepth::Ten
    }

    pub(crate) const fn pixel_format(&self) -> NativeVideoPixelFormat {
        NativeVideoPixelFormat::Yuv420p10le
    }
}

struct NativeLtxvCodecThreadInner {
    identity: NativeLtxvCodecThreadIdentity,
    node_service_identity: NativeLtxvPreprocessServiceIdentity,
    sender: Mutex<Option<mpsc::SyncSender<NativeLtxvCodecThreadRequest>>>,
    runner: Mutex<Option<JoinHandle<()>>>,
}

#[allow(
    dead_code,
    reason = "consumed by the native LTXVPreprocess adapter and VP9 owned-byte bridge"
)]
pub(crate) struct NativeLtxvCodecThreadService {
    inner: Arc<NativeLtxvCodecThreadInner>,
}

#[allow(
    dead_code,
    reason = "consumed by the native LTXVPreprocess adapter and VP9 owned-byte bridge"
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
    reason = "consumed by the native LTXVPreprocess adapter and VP9 owned-byte bridge"
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
            let codec = codec
                .admit_video_suite(&startup_cancellation)
                .map_err(|error| NativeLtxvCodecThreadError::SuiteAdmission(Box::new(error)))?;
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
                process_video_codec_request(&codec, &backend, limits, request)
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
            eprintln!("native video codec service cleanup failed: {error}");
        }
    }
}

#[allow(
    dead_code,
    reason = "consumed by the native LTXVPreprocess adapter and VP9 owned-byte bridge"
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
        let result = self.submit(
            NativeVideoCodecThreadOperation::Preprocess {
                image: image.clone(),
                compression,
            },
            context,
        );
        async move {
            match result.await? {
                NativeVideoCodecThreadOutput::Image(image) => Ok(image),
                NativeVideoCodecThreadOutput::Vp9Webm(_)
                | NativeVideoCodecThreadOutput::Av1Webm(_) => {
                    Err(NativeLtxvCodecThreadError::StatePoisoned)
                }
            }
        }
        .boxed()
    }

    pub(crate) fn encode_vp9_webm_batch(
        &self,
        images: &ImageTensor,
        frame_rate: (u64, u64),
        crf: NativeVideoCrf,
        limits: NativeVp9WebmBatchLimits,
        context: &ExecutionContext<'_>,
    ) -> BoxFuture<'static, Result<NativeOwnedVp9Webm, NativeLtxvCodecThreadError>> {
        self.encode_vp9_webm_batch_with_metadata(
            images,
            frame_rate,
            crf,
            limits,
            NativeVideoContainerMetadata::empty(),
            context,
        )
    }

    pub(crate) fn encode_vp9_webm_batch_with_metadata(
        &self,
        images: &ImageTensor,
        frame_rate: (u64, u64),
        crf: NativeVideoCrf,
        limits: NativeVp9WebmBatchLimits,
        metadata: NativeVideoContainerMetadata,
        context: &ExecutionContext<'_>,
    ) -> BoxFuture<'static, Result<NativeOwnedVp9Webm, NativeLtxvCodecThreadError>> {
        let result = self.submit(
            NativeVideoCodecThreadOperation::EncodeVp9Webm {
                images: images.clone(),
                frame_rate,
                crf,
                limits,
                metadata,
            },
            context,
        );
        async move {
            match result.await? {
                NativeVideoCodecThreadOutput::Vp9Webm(encoded) => Ok(encoded),
                NativeVideoCodecThreadOutput::Image(_)
                | NativeVideoCodecThreadOutput::Av1Webm(_) => {
                    Err(NativeLtxvCodecThreadError::StatePoisoned)
                }
            }
        }
        .boxed()
    }

    pub(crate) fn encode_av1_webm_batch_with_metadata(
        &self,
        images: &ImageTensor,
        frame_rate: (u64, u64),
        crf: NativeVideoCrf,
        limits: NativeVp9WebmBatchLimits,
        metadata: NativeVideoContainerMetadata,
        context: &ExecutionContext<'_>,
    ) -> BoxFuture<'static, Result<NativeOwnedAv1Webm, NativeLtxvCodecThreadError>> {
        let result = self.submit(
            NativeVideoCodecThreadOperation::EncodeAv1Webm {
                images: images.clone(),
                frame_rate,
                crf,
                limits,
                metadata,
            },
            context,
        );
        async move {
            match result.await? {
                NativeVideoCodecThreadOutput::Av1Webm(encoded) => Ok(encoded),
                NativeVideoCodecThreadOutput::Image(_)
                | NativeVideoCodecThreadOutput::Vp9Webm(_) => {
                    Err(NativeLtxvCodecThreadError::StatePoisoned)
                }
            }
        }
        .boxed()
    }

    fn submit(
        &self,
        operation: NativeVideoCodecThreadOperation,
        context: &ExecutionContext<'_>,
    ) -> BoxFuture<'static, Result<NativeVideoCodecThreadOutput, NativeLtxvCodecThreadError>> {
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
                operation,
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
            eprintln!("native video codec thread cleanup failed: {error}");
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
    Processor: FnMut(
            NativeLtxvCodecThreadInvocation,
        ) -> Result<NativeVideoCodecThreadOutput, NativeLtxvCodecThreadError>
        + 'static,
{
    let (sender, receiver) = mpsc::sync_channel(1);
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let runner = thread::Builder::new()
        .name(VIDEO_CODEC_THREAD_NAME.to_owned())
        .spawn(move || match factory() {
            Ok((identity, mut processor)) => {
                if ready_sender.send(Ok(identity)).is_err() {
                    eprintln!("native video codec thread readiness receiver was dropped");
                    return;
                }
                while let Ok(request) = receiver.recv() {
                    let NativeLtxvCodecThreadRequest {
                        invocation,
                        response,
                    } = request;
                    let result = processor(invocation);
                    if response.send(result).is_err() {
                        eprintln!("native video codec request receiver was dropped");
                    }
                }
            }
            Err(error) => {
                if ready_sender.send(Err(error)).is_err() {
                    eprintln!("native video codec thread startup error receiver was dropped");
                }
            }
        })
        .map_err(NativeLtxvCodecThreadError::ThreadSpawn)?;
    let identity = match ready_receiver.recv() {
        Ok(Ok(identity)) => identity,
        Ok(Err(error)) => {
            if runner.join().is_err() {
                eprintln!("native video codec thread panicked after reporting startup failure");
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
        | NativeLtxvCodecThreadError::SuiteAdmission(_)
        | NativeLtxvCodecThreadError::Preprocess(_)
        | NativeLtxvCodecThreadError::Vp9Encode(_)
        | NativeLtxvCodecThreadError::Av1Encode(_)
        | NativeLtxvCodecThreadError::EncodedOutput(_)) => {
            NativeLtxvPreprocessServiceError::Execution(error.to_string())
        }
    }
}

fn process_video_codec_request(
    codec: &NativeVideoCodecSuite,
    backend: &CpuBackend,
    limits: NativeLtxvH264PreprocessLimits,
    request: NativeLtxvCodecThreadInvocation,
) -> Result<NativeVideoCodecThreadOutput, NativeLtxvCodecThreadError> {
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
    let output = match request.operation {
        NativeVideoCodecThreadOperation::Preprocess { image, compression } => {
            NativeVideoCodecThreadOutput::Image(
                codec
                    .preprocess_image(&image, compression, limits, backend, &context)
                    .map_err(map_ltxv_thread_preprocess_error)?,
            )
        }
        NativeVideoCodecThreadOperation::EncodeVp9Webm {
            images,
            frame_rate,
            crf,
            limits,
            metadata,
        } => NativeVideoCodecThreadOutput::Vp9Webm(process_vp9_webm_request(
            codec, backend, &context, &images, frame_rate, crf, limits, &metadata,
        )?),
        NativeVideoCodecThreadOperation::EncodeAv1Webm {
            images,
            frame_rate,
            crf,
            limits,
            metadata,
        } => NativeVideoCodecThreadOutput::Av1Webm(process_av1_webm_request(
            codec, backend, &context, &images, frame_rate, crf, limits, &metadata,
        )?),
    };
    context
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn process_vp9_webm_request(
    codec: &NativeVideoCodecSuite,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    images: &ImageTensor,
    frame_rate: (u64, u64),
    crf: NativeVideoCrf,
    limits: NativeVp9WebmBatchLimits,
    metadata: &NativeVideoContainerMetadata,
) -> Result<NativeOwnedVp9Webm, NativeLtxvCodecThreadError> {
    context
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    let encoded = codec
        .encode_vp9_webm_batch_with_metadata(
            images, frame_rate, crf, limits, metadata, backend, context,
        )
        .map_err(map_vp9_thread_encode_error)?;
    context
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    let (width, height) = encoded.dimensions();
    let frame_rate = encoded.frame_rate();
    let frame_count = encoded.frame_count();
    let has_alpha = encoded.has_alpha();
    let encoded_bytes = encoded
        .encoded_bytes()
        .map_err(map_vp9_thread_encode_error)?;
    let output = materialize_owned_vp9_webm(
        backend,
        context,
        encoded_bytes,
        width,
        height,
        frame_rate,
        frame_count,
        has_alpha,
    )?;
    drop(encoded);
    context
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn process_av1_webm_request(
    codec: &NativeVideoCodecSuite,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    images: &ImageTensor,
    frame_rate: (u64, u64),
    crf: NativeVideoCrf,
    limits: NativeVp9WebmBatchLimits,
    metadata: &NativeVideoContainerMetadata,
) -> Result<NativeOwnedAv1Webm, NativeLtxvCodecThreadError> {
    context
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    let encoded = codec
        .encode_av1_webm_batch_with_metadata(
            images, frame_rate, crf, limits, metadata, backend, context,
        )
        .map_err(map_av1_thread_encode_error)?;
    context
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    let (width, height) = encoded.dimensions();
    let frame_rate = encoded.frame_rate();
    let frame_count = encoded.frame_count();
    let encoded_bytes = encoded
        .encoded_bytes()
        .map_err(map_av1_thread_encode_error)?;
    let output = materialize_owned_av1_webm(
        backend,
        context,
        encoded_bytes,
        width,
        height,
        frame_rate,
        frame_count,
    )?;
    drop(encoded);
    context
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn materialize_owned_vp9_webm(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    encoded_bytes: &[u8],
    width: i32,
    height: i32,
    frame_rate: (i32, i32),
    frame_count: usize,
    has_alpha: bool,
) -> Result<NativeOwnedVp9Webm, NativeLtxvCodecThreadError> {
    let (bytes, content_sha256) = materialize_owned_webm_bytes(backend, context, encoded_bytes)?;
    Ok(NativeOwnedVp9Webm {
        bytes,
        content_sha256,
        width,
        height,
        frame_rate,
        frame_count,
        has_alpha,
    })
}

#[allow(clippy::too_many_arguments)]
fn materialize_owned_av1_webm(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    encoded_bytes: &[u8],
    width: i32,
    height: i32,
    frame_rate: (i32, i32),
    frame_count: usize,
) -> Result<NativeOwnedAv1Webm, NativeLtxvCodecThreadError> {
    let (bytes, content_sha256) = materialize_owned_webm_bytes(backend, context, encoded_bytes)?;
    Ok(NativeOwnedAv1Webm {
        bytes,
        content_sha256,
        width,
        height,
        frame_rate,
        frame_count,
    })
}

fn materialize_owned_webm_bytes(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    encoded_bytes: &[u8],
) -> Result<(Tensor, [u8; 32]), NativeLtxvCodecThreadError> {
    context
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    let byte_length = u64::try_from(encoded_bytes.len())
        .map_err(|_| NativeLtxvCodecThreadError::ResourceExhausted)?;
    let descriptor =
        TensorDescriptor::contiguous(vec![byte_length], DType::U8, DeviceId::CPU, context.stream)
            .map_err(map_vp9_output_tensor_error)?;
    let (bytes, _) = backend
        .upload_bytes(descriptor, encoded_bytes, context)
        .map_err(map_vp9_output_tensor_error)?;
    context
        .check()
        .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
    Ok((bytes, Sha256::digest(encoded_bytes).into()))
}

fn map_vp9_thread_encode_error(
    error: NativeVideoCodecVp9EncodeError,
) -> NativeLtxvCodecThreadError {
    match error {
        NativeVideoCodecVp9EncodeError::Cancelled => NativeLtxvCodecThreadError::Cancelled,
        NativeVideoCodecVp9EncodeError::ResourceExhausted { .. } => {
            NativeLtxvCodecThreadError::ResourceExhausted
        }
        error => NativeLtxvCodecThreadError::Vp9Encode(Box::new(error)),
    }
}

fn map_av1_thread_encode_error(
    error: NativeVideoCodecAv1EncodeError,
) -> NativeLtxvCodecThreadError {
    match error {
        NativeVideoCodecAv1EncodeError::Cancelled => NativeLtxvCodecThreadError::Cancelled,
        NativeVideoCodecAv1EncodeError::ResourceExhausted { .. } => {
            NativeLtxvCodecThreadError::ResourceExhausted
        }
        error => NativeLtxvCodecThreadError::Av1Encode(Box::new(error)),
    }
}

fn map_vp9_output_tensor_error(error: TensorError) -> NativeLtxvCodecThreadError {
    match error {
        TensorError::Cancelled => NativeLtxvCodecThreadError::Cancelled,
        TensorError::AllocationFailed { .. }
        | TensorError::ResourceLimitExceeded { .. }
        | TensorError::WorkspaceAuthorizationExceeded { .. } => {
            NativeLtxvCodecThreadError::ResourceExhausted
        }
        error => NativeLtxvCodecThreadError::EncodedOutput(Box::new(error)),
    }
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
    hash_identity_field(&mut digest, VIDEO_CODEC_THREAD_IDENTITY_VERSION.as_bytes());
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
    use crate::NativeVp9WebmEncodeLimits;
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
                        match request.operation {
                            NativeVideoCodecThreadOperation::Preprocess { image, .. } => {
                                Ok(NativeVideoCodecThreadOutput::Image(image))
                            }
                            NativeVideoCodecThreadOperation::EncodeVp9Webm { crf, .. } => {
                                if crf.bits() != 31.5_f64.to_bits() {
                                    return Err(NativeLtxvCodecThreadError::StatePoisoned);
                                }
                                Err(NativeLtxvCodecThreadError::StatePoisoned)
                            }
                            NativeVideoCodecThreadOperation::EncodeAv1Webm { crf, .. } => {
                                if crf.bits() != 31.5_f64.to_bits() {
                                    return Err(NativeLtxvCodecThreadError::StatePoisoned);
                                }
                                Err(NativeLtxvCodecThreadError::StatePoisoned)
                            }
                        }
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
                        let NativeLtxvCodecThreadInvocation {
                            operation,
                            cancellation,
                            ..
                        } = request;
                        let NativeVideoCodecThreadOperation::Preprocess { image, compression } =
                            operation
                        else {
                            return Err(NativeLtxvCodecThreadError::StatePoisoned);
                        };
                        if compression == 99 {
                            return Err(NativeLtxvCodecThreadError::ResourceExhausted);
                        }
                        if compression == 98 {
                            cancellation.cancel();
                            return Ok(NativeVideoCodecThreadOutput::Image(image));
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
                        cancellation
                            .check()
                            .map_err(|_| NativeLtxvCodecThreadError::Cancelled)?;
                        Ok(NativeVideoCodecThreadOutput::Image(image))
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

    #[test]
    fn retained_video_codec_thread_returns_owned_vp9_bytes_and_preserves_ltxv()
    -> Result<(), Box<dyn std::error::Error>> {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NativeLtxvCodecRequestProxy>();
        assert_send_sync::<NativeOwnedVp9Webm>();
        assert_send_sync::<NativeOwnedAv1Webm>();

        let cancellation = CancellationToken::default();
        let (backend, _image, scratch) = test_image_and_context(&cancellation)?;
        let image_context = request_context(scratch.clone(), &cancellation);
        let image = ImageTensor::from_f32(&backend, &image_context, 1, 2, 2, 4, &[0.5; 16])?;
        let events = Arc::new(Mutex::new(Vec::new()));
        let actor_backend = backend.clone();
        let service = start_ltxv_codec_thread({
            let events = events.clone();
            move || {
                events
                    .lock()
                    .map_err(|_| NativeLtxvCodecThreadError::StatePoisoned)?
                    .push(("start", thread::current().id()));
                Ok((
                    test_identity("vp9-owned-bridge"),
                    move |request: NativeLtxvCodecThreadInvocation| {
                        events
                            .lock()
                            .map_err(|_| NativeLtxvCodecThreadError::StatePoisoned)?
                            .push(("request", thread::current().id()));
                        let NativeLtxvCodecThreadInvocation {
                            operation,
                            stream,
                            scratch,
                            cancellation,
                        } = request;
                        let context = ExecutionContext {
                            stream,
                            scratch,
                            rng_phase: None,
                            cancellation: &cancellation,
                        };
                        match operation {
                            NativeVideoCodecThreadOperation::Preprocess { image, .. } => {
                                Ok(NativeVideoCodecThreadOutput::Image(image))
                            }
                            NativeVideoCodecThreadOperation::EncodeVp9Webm {
                                images,
                                metadata,
                                ..
                            } => {
                                if !matches!(images.dimensions(), Ok((_, _, _, 4))) {
                                    return Err(NativeLtxvCodecThreadError::StatePoisoned);
                                }
                                let entries = metadata.entries();
                                if entries.len() != 3
                                    || entries[0].0.as_bytes() != b"prompt"
                                    || entries[0].1.as_bytes() != b"first"
                                    || entries[1].0.as_bytes() != b"workflow"
                                    || entries[2].0.as_bytes() != b"prompt"
                                    || entries[2].1.as_bytes() != b"last"
                                {
                                    return Err(NativeLtxvCodecThreadError::StatePoisoned);
                                }
                                let encoded = materialize_owned_vp9_webm(
                                    &actor_backend,
                                    &context,
                                    b"HPPPT",
                                    2,
                                    2,
                                    (125, 2997),
                                    3,
                                    true,
                                )?;
                                Ok(NativeVideoCodecThreadOutput::Vp9Webm(encoded))
                            }
                            NativeVideoCodecThreadOperation::EncodeAv1Webm {
                                images,
                                metadata,
                                ..
                            } => {
                                if !matches!(images.dimensions(), Ok((_, _, _, 4))) {
                                    return Err(NativeLtxvCodecThreadError::StatePoisoned);
                                }
                                let entries = metadata.entries();
                                if entries.len() != 2
                                    || entries[0].0.as_bytes() != b"prompt"
                                    || entries[0].1.as_bytes() != b"av1"
                                    || entries[1].0.as_bytes() != b"workflow"
                                {
                                    return Err(NativeLtxvCodecThreadError::StatePoisoned);
                                }
                                let encoded = materialize_owned_av1_webm(
                                    &actor_backend,
                                    &context,
                                    b"HA1T",
                                    2,
                                    2,
                                    (125, 2997),
                                    3,
                                )?;
                                Ok(NativeVideoCodecThreadOutput::Av1Webm(encoded))
                            }
                        }
                    },
                ))
            }
        })?;
        let proxy = service.proxy();
        let context = request_context(scratch.clone(), &cancellation);
        let preprocessed = block_on(proxy.preprocess_image(&image, 0, &context))?;
        assert_eq!(
            preprocessed.tensor().storage_id(),
            image.tensor().storage_id()
        );

        let session_limits = NativeVp9WebmEncodeLimits::checked(1024, 256, 1024, 32)?;
        let batch_limits = NativeVp9WebmBatchLimits::checked(session_limits, 4, 1024)?;
        let metadata = NativeVideoContainerMetadata::checked(
            vec![
                ("prompt".to_owned(), "first".to_owned()),
                ("workflow".to_owned(), "{}".to_owned()),
                ("prompt".to_owned(), "last".to_owned()),
            ],
            crate::NativeVideoContainerMetadataLimits::checked(3, 16, 16, 64)?,
        )?;
        let encoded = block_on(proxy.encode_vp9_webm_batch_with_metadata(
            &image,
            (2997, 125),
            NativeVideoCrf::checked(31.5)?,
            batch_limits,
            metadata,
            &context,
        ))?;
        assert_eq!(encoded.encoded_bytes()?, b"HPPPT");
        assert_eq!(encoded.dimensions(), (2, 2));
        assert_eq!(encoded.frame_rate(), (125, 2997));
        assert_eq!(encoded.frame_count(), 3);
        assert!(encoded.has_alpha());
        assert_eq!(
            encoded.content_sha256(),
            <[u8; 32]>::from(Sha256::digest(b"HPPPT"))
        );
        assert_eq!(scratch.in_use_bytes(), 0);
        drop(encoded);

        let av1_metadata = NativeVideoContainerMetadata::checked(
            vec![
                ("prompt".to_owned(), "av1".to_owned()),
                ("workflow".to_owned(), "{}".to_owned()),
            ],
            crate::NativeVideoContainerMetadataLimits::checked(2, 16, 16, 64)?,
        )?;
        let av1 = block_on(proxy.encode_av1_webm_batch_with_metadata(
            &image,
            (2997, 125),
            NativeVideoCrf::checked(31.5)?,
            batch_limits,
            av1_metadata,
            &context,
        ))?;
        assert_eq!(av1.encoded_bytes()?, b"HA1T");
        assert_eq!(av1.dimensions(), (2, 2));
        assert_eq!(av1.frame_rate(), (125, 2997));
        assert_eq!(av1.frame_count(), 3);
        assert!(!av1.has_alpha());
        assert_eq!(av1.bit_depth(), NativeVideoBitDepth::Ten);
        assert_eq!(av1.pixel_format(), NativeVideoPixelFormat::Yuv420p10le);
        assert_eq!(
            av1.content_sha256(),
            <[u8; 32]>::from(Sha256::digest(b"HA1T"))
        );
        assert_eq!(scratch.in_use_bytes(), 0);
        drop(av1);
        service.shutdown()?;

        let events = events
            .lock()
            .map_err(|_| "thread event mutex was poisoned")?;
        let actor_thread = events.first().ok_or("missing actor start event")?.1;
        assert_ne!(actor_thread, thread::current().id());
        assert_eq!(
            events.iter().filter(|event| event.0 == "request").count(),
            3
        );
        assert!(events.iter().all(|event| event.1 == actor_thread));
        Ok(())
    }

    #[test]
    fn owned_vp9_output_materialization_is_accounted_atomic_and_retryable()
    -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::default();
        let (backend, authority) = BackendWorkspaceAuthority::create_backend(1024)?;
        let scratch = authority.authorize_workspace(1024)?;
        let context = request_context(scratch.clone(), &cancellation);
        let baseline = backend.memory_snapshot().current_bytes;

        let encoded =
            materialize_owned_vp9_webm(&backend, &context, b"HPPPT", 2, 2, (125, 2997), 3, false)?;
        assert_eq!(scratch.in_use_bytes(), 0);
        assert_eq!(backend.memory_snapshot().current_bytes, baseline + 16);
        drop(encoded);
        assert_eq!(backend.memory_snapshot().current_bytes, baseline);

        let cancelled = CancellationToken::default();
        cancelled.cancel();
        let cancelled_context = request_context(scratch.clone(), &cancelled);
        assert!(matches!(
            materialize_owned_vp9_webm(
                &backend,
                &cancelled_context,
                b"HPPPT",
                2,
                2,
                (125, 2997),
                3,
                false,
            ),
            Err(NativeLtxvCodecThreadError::Cancelled)
        ));
        assert_eq!(backend.memory_snapshot().current_bytes, baseline);
        assert_eq!(scratch.in_use_bytes(), 0);

        let (constrained_backend, constrained_authority) =
            BackendWorkspaceAuthority::create_backend(8)?;
        let constrained_scratch = constrained_authority.authorize_workspace(8)?;
        let constrained_context = request_context(constrained_scratch.clone(), &cancellation);
        assert!(matches!(
            materialize_owned_vp9_webm(
                &constrained_backend,
                &constrained_context,
                b"HPPPT",
                2,
                2,
                (125, 2997),
                3,
                false,
            ),
            Err(NativeLtxvCodecThreadError::ResourceExhausted)
        ));
        assert_eq!(constrained_backend.memory_snapshot().current_bytes, 0);
        assert_eq!(constrained_scratch.in_use_bytes(), 0);

        let retry =
            materialize_owned_vp9_webm(&backend, &context, b"HPPPT", 2, 2, (125, 2997), 3, false)?;
        assert_eq!(retry.encoded_bytes()?, b"HPPPT");
        assert_eq!(scratch.in_use_bytes(), 0);
        Ok(())
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
