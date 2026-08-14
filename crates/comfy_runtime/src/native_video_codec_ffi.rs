use crate::{
    CertifiedVideoCodecDependencyClosure, VIDEO_CODEC_FFI_UNSAFE_OWNER,
    native_video_codec_abi as abi,
};
use comfy_tensor::{CpuBackend, CpuWorkspaceLease, CpuWorkspaceVec, ExecutionContext, TensorError};
use comfy_types::{CancellationError, CancellationToken};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::c_void,
    marker::PhantomData,
    path::PathBuf,
    ptr::NonNull,
    sync::Arc,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NativeVideoCodecLoadError {
    #[error("native video codec loading was cancelled")]
    Cancelled,
    #[error("native video codec loading is unsupported for this target")]
    UnsupportedTarget,
    #[error("the certified native video codec closure is incomplete")]
    InvalidClosure,
    #[error("native video codec loader handle reservation failed")]
    ResourceExhausted,
    #[error("certified native video codec library {identity} could not be loaded: {reason}")]
    LibraryLoad { identity: String, reason: String },
    #[error("the loaded native video codec namespace failed binding proof: {0}")]
    BindingProof(String),
}

impl From<CancellationError> for NativeVideoCodecLoadError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

pub struct NativeVideoCodecLoad {
    loaded: LoadedVideoCodecLibraries,
    closure: CertifiedVideoCodecDependencyClosure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeVideoCodecRuntimeVersions {
    avcodec: u32,
    avformat: u32,
    avutil: u32,
    swresample: u32,
    swscale: u32,
}

impl NativeVideoCodecRuntimeVersions {
    pub fn avcodec(&self) -> u32 {
        self.avcodec
    }

    pub fn avformat(&self) -> u32 {
        self.avformat
    }

    pub fn avutil(&self) -> u32 {
        self.avutil
    }

    pub fn swresample(&self) -> u32 {
        self.swresample
    }

    pub fn swscale(&self) -> u32 {
        self.swscale
    }
}

pub struct NativeVideoCodecBinding {
    _symbols: NativeVideoCodecSymbols,
    versions: NativeVideoCodecRuntimeVersions,
    load: NativeVideoCodecLoad,
}

impl NativeVideoCodecBinding {
    pub fn target(&self) -> &str {
        self.load.target()
    }

    pub fn primary_catalog_sha256(&self) -> &str {
        self.load.primary_catalog_sha256()
    }

    pub fn dependency_first_order(&self) -> &[String] {
        self.load.dependency_first_order()
    }

    pub fn loaded_library_count(&self) -> usize {
        self.load.loaded_library_count()
    }

    pub fn runtime_versions(&self) -> NativeVideoCodecRuntimeVersions {
        self.versions
    }

    #[allow(
        dead_code,
        reason = "consumed by the following bounded codec operation"
    )]
    pub(crate) fn open_bounded_avio_input<'binding>(
        &'binding self,
        bytes: Arc<[u8]>,
        maximum_input_bytes: usize,
        buffer_bytes: usize,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeVideoCodecMemoryInput<'binding>, NativeVideoCodecIoError> {
        let state = NativeVideoCodecInputState {
            bytes,
            position: 0,
            maximum_position: maximum_input_bytes,
            cancellation: context.cancellation.clone(),
            failure: None,
            #[cfg(test)]
            panic_on_next_callback: false,
        };
        if state.bytes.len() > maximum_input_bytes {
            return Err(NativeVideoCodecIoError::InvalidBounds);
        }
        let functions = NativeVideoCodecAvioFunctions::from_binding(self);
        let avio = allocate_native_video_codec_avio(
            self,
            state,
            functions,
            buffer_bytes,
            false,
            Some(native_video_codec_input_read),
            None,
            Some(native_video_codec_input_seek),
            backend,
            context,
            &mut || context.cancellation.check(),
        )?;
        Ok(NativeVideoCodecMemoryInput { avio })
    }

    #[allow(
        dead_code,
        reason = "consumed by the following bounded codec operation"
    )]
    pub(crate) fn open_bounded_avio_output<'binding>(
        &'binding self,
        maximum_output_bytes: usize,
        buffer_bytes: usize,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeVideoCodecMemoryOutput<'binding>, NativeVideoCodecIoError> {
        if maximum_output_bytes == 0 {
            return Err(NativeVideoCodecIoError::InvalidBounds);
        }
        let bytes = backend.workspace_vec::<u8>(context, maximum_output_bytes)?;
        let state = NativeVideoCodecOutputState {
            bytes,
            position: 0,
            maximum_bytes: maximum_output_bytes,
            cancellation: context.cancellation.clone(),
            failure: None,
            #[cfg(test)]
            panic_on_next_callback: false,
        };
        let functions = NativeVideoCodecAvioFunctions::from_binding(self);
        let avio = allocate_native_video_codec_avio(
            self,
            state,
            functions,
            buffer_bytes,
            true,
            None,
            Some(native_video_codec_output_write),
            Some(native_video_codec_output_seek),
            backend,
            context,
            &mut || context.cancellation.check(),
        )?;
        Ok(NativeVideoCodecMemoryOutput { avio })
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
const NATIVE_VIDEO_CODEC_AVIO_CONTEXT_HEADROOM_BYTES: u64 = 4_096;

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeVideoCodecIoCallbackFailure {
    Cancelled,
    InvalidArgument,
    OutputLimit,
    ResourceExhausted,
    Panicked,
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
#[derive(Debug, Error)]
pub(crate) enum NativeVideoCodecIoError {
    #[error("native video codec memory I/O was cancelled")]
    Cancelled,
    #[error("native video codec memory I/O bounds are invalid")]
    InvalidBounds,
    #[error("native video codec memory I/O allocation failed")]
    NativeAllocationFailed,
    #[error("native video codec memory I/O callback rejected an argument")]
    InvalidCallbackArgument,
    #[error("native video codec memory I/O output limit was exceeded")]
    OutputLimitExceeded,
    #[error("native video codec memory I/O callback exhausted its authorized storage")]
    CallbackResourceExhausted,
    #[error("native video codec memory I/O callback panicked")]
    CallbackPanicked,
    #[error(transparent)]
    Tensor(#[from] TensorError),
}

impl From<CancellationError> for NativeVideoCodecIoError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
#[derive(Clone, Copy)]
struct NativeVideoCodecAvioFunctions {
    av_malloc: abi::AvMalloc,
    av_free: abi::AvFree,
    avio_alloc_context: abi::AvioAllocContext,
    avio_context_free: abi::AvioContextFree,
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
impl NativeVideoCodecAvioFunctions {
    fn from_binding(binding: &NativeVideoCodecBinding) -> Self {
        Self {
            av_malloc: binding._symbols.avutil.av_malloc,
            av_free: binding._symbols.avutil.av_free,
            avio_alloc_context: binding._symbols.avformat.avio_alloc_context,
            avio_context_free: binding._symbols.avformat.avio_context_free,
        }
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
struct NativeVideoCodecInputState {
    bytes: Arc<[u8]>,
    position: usize,
    maximum_position: usize,
    cancellation: CancellationToken,
    failure: Option<NativeVideoCodecIoCallbackFailure>,
    #[cfg(test)]
    panic_on_next_callback: bool,
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
struct NativeVideoCodecOutputState {
    bytes: CpuWorkspaceVec<u8>,
    position: usize,
    maximum_bytes: usize,
    cancellation: CancellationToken,
    failure: Option<NativeVideoCodecIoCallbackFailure>,
    #[cfg(test)]
    panic_on_next_callback: bool,
}

struct NativeVideoCodecAvio<'binding, State> {
    context: NonNull<abi::AvIoContext>,
    state: Box<[State]>,
    functions: NativeVideoCodecAvioFunctions,
    _workspace: CpuWorkspaceLease,
    _binding: PhantomData<&'binding NativeVideoCodecBinding>,
    _thread_bound: PhantomData<std::rc::Rc<()>>,
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
pub(crate) struct NativeVideoCodecMemoryInput<'binding> {
    avio: NativeVideoCodecAvio<'binding, NativeVideoCodecInputState>,
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
pub(crate) struct NativeVideoCodecMemoryOutput<'binding> {
    avio: NativeVideoCodecAvio<'binding, NativeVideoCodecOutputState>,
}

impl NativeVideoCodecMemoryInput<'_> {
    #[allow(
        dead_code,
        reason = "consumed by the following bounded codec operation"
    )]
    pub(crate) fn context_ptr(&self) -> *mut abi::AvIoContext {
        self.avio.context.as_ptr()
    }

    #[allow(
        dead_code,
        reason = "consumed by the following bounded codec operation"
    )]
    pub(crate) fn check_callback_status(&self) -> Result<(), NativeVideoCodecIoError> {
        callback_status(&self.avio.state[0].cancellation, self.avio.state[0].failure)
    }
}

impl NativeVideoCodecMemoryOutput<'_> {
    #[allow(
        dead_code,
        reason = "consumed by the following bounded codec operation"
    )]
    pub(crate) fn context_ptr(&self) -> *mut abi::AvIoContext {
        self.avio.context.as_ptr()
    }

    #[allow(
        dead_code,
        reason = "consumed by the following bounded codec operation"
    )]
    pub(crate) fn check_callback_status(&self) -> Result<(), NativeVideoCodecIoError> {
        callback_status(&self.avio.state[0].cancellation, self.avio.state[0].failure)
    }

    #[allow(
        dead_code,
        reason = "consumed by the following bounded codec operation"
    )]
    pub(crate) fn staged_bytes(&self) -> Result<&[u8], NativeVideoCodecIoError> {
        self.check_callback_status()?;
        Ok(&self.avio.state[0].bytes)
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
fn callback_status(
    cancellation: &CancellationToken,
    failure: Option<NativeVideoCodecIoCallbackFailure>,
) -> Result<(), NativeVideoCodecIoError> {
    cancellation.check()?;
    match failure {
        None => Ok(()),
        Some(NativeVideoCodecIoCallbackFailure::Cancelled) => {
            Err(NativeVideoCodecIoError::Cancelled)
        }
        Some(NativeVideoCodecIoCallbackFailure::InvalidArgument) => {
            Err(NativeVideoCodecIoError::InvalidCallbackArgument)
        }
        Some(NativeVideoCodecIoCallbackFailure::OutputLimit) => {
            Err(NativeVideoCodecIoError::OutputLimitExceeded)
        }
        Some(NativeVideoCodecIoCallbackFailure::ResourceExhausted) => {
            Err(NativeVideoCodecIoError::CallbackResourceExhausted)
        }
        Some(NativeVideoCodecIoCallbackFailure::Panicked) => {
            Err(NativeVideoCodecIoError::CallbackPanicked)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn allocate_native_video_codec_avio<'binding, State>(
    _binding: &'binding NativeVideoCodecBinding,
    state: State,
    functions: NativeVideoCodecAvioFunctions,
    buffer_bytes: usize,
    write: bool,
    read_packet: Option<abi::AvIoReadPacket>,
    write_packet: Option<abi::AvIoWritePacket>,
    seek: Option<abi::AvIoSeek>,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<NativeVideoCodecAvio<'binding, State>, NativeVideoCodecIoError> {
    allocate_native_video_codec_avio_inner(
        state,
        functions,
        buffer_bytes,
        write,
        read_packet,
        write_packet,
        seek,
        backend,
        context,
        check_cancellation,
    )
}

#[allow(clippy::too_many_arguments)]
fn allocate_native_video_codec_avio_inner<'binding, State>(
    state: State,
    functions: NativeVideoCodecAvioFunctions,
    buffer_bytes: usize,
    write: bool,
    read_packet: Option<abi::AvIoReadPacket>,
    write_packet: Option<abi::AvIoWritePacket>,
    seek: Option<abi::AvIoSeek>,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<NativeVideoCodecAvio<'binding, State>, NativeVideoCodecIoError> {
    check_cancellation()?;
    let buffer_size = i32::try_from(buffer_bytes)
        .ok()
        .filter(|size| *size > 0)
        .ok_or(NativeVideoCodecIoError::InvalidBounds)?;
    let requested_workspace = u64::try_from(buffer_bytes)
        .ok()
        .and_then(|bytes| bytes.checked_add(NATIVE_VIDEO_CODEC_AVIO_CONTEXT_HEADROOM_BYTES))
        .ok_or(NativeVideoCodecIoError::InvalidBounds)?;
    let workspace = backend.reserve_workspace(context, requested_workspace)?;
    let mut states = Vec::new();
    states
        .try_reserve_exact(1)
        .map_err(|_| NativeVideoCodecIoError::NativeAllocationFailed)?;
    states.push(state);
    let state = states.into_boxed_slice();
    check_cancellation()?;
    let buffer = unsafe { (functions.av_malloc)(buffer_bytes) }.cast::<u8>();
    if buffer.is_null() {
        return Err(NativeVideoCodecIoError::NativeAllocationFailed);
    }
    if let Err(error) = check_cancellation() {
        unsafe { (functions.av_free)(buffer.cast()) };
        return Err(error.into());
    }
    let opaque = state.as_ptr().cast_mut().cast::<c_void>();
    let context_ptr = unsafe {
        (functions.avio_alloc_context)(
            buffer,
            buffer_size,
            i32::from(write),
            opaque,
            read_packet,
            write_packet,
            seek,
        )
    };
    let Some(context_ptr) = NonNull::new(context_ptr) else {
        unsafe { (functions.av_free)(buffer.cast()) };
        return Err(NativeVideoCodecIoError::NativeAllocationFailed);
    };
    let avio = NativeVideoCodecAvio {
        context: context_ptr,
        state,
        functions,
        _workspace: workspace,
        _binding: PhantomData,
        _thread_bound: PhantomData,
    };
    if let Err(error) = check_cancellation() {
        drop(avio);
        return Err(error.into());
    }
    Ok(avio)
}

impl<State> Drop for NativeVideoCodecAvio<'_, State> {
    fn drop(&mut self) {
        let mut context = self.context.as_ptr();
        let buffer = unsafe { (*context).buffer };
        unsafe {
            (*context).buffer = std::ptr::null_mut();
            if !buffer.is_null() {
                (self.functions.av_free)(buffer.cast());
            }
            (self.functions.avio_context_free)(std::ptr::addr_of_mut!(context));
        }
        if !context.is_null() {
            eprintln!("native video codec AVIO context cleanup did not clear its pointer");
        }
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
unsafe extern "C" fn native_video_codec_input_read(
    opaque: *mut c_void,
    destination: *mut u8,
    requested: i32,
) -> i32 {
    if opaque.is_null() {
        return abi::AV_ERROR_INVALID_ARGUMENT;
    }
    let state = unsafe { &mut *opaque.cast::<NativeVideoCodecInputState>() };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        #[cfg(test)]
        if std::mem::take(&mut state.panic_on_next_callback) {
            panic!("injected AVIO input callback panic");
        }
        if state.cancellation.is_cancelled() {
            return Err(NativeVideoCodecIoCallbackFailure::Cancelled);
        }
        if let Some(failure) = state.failure {
            return Err(failure);
        }
        if destination.is_null() || requested <= 0 {
            return Err(NativeVideoCodecIoCallbackFailure::InvalidArgument);
        }
        if state.position >= state.bytes.len() {
            return Ok(abi::AV_ERROR_END_OF_FILE);
        }
        let requested = usize::try_from(requested)
            .map_err(|_| NativeVideoCodecIoCallbackFailure::InvalidArgument)?;
        let count = requested.min(state.bytes.len() - state.position);
        unsafe {
            std::ptr::copy_nonoverlapping(
                state.bytes.as_ptr().add(state.position),
                destination,
                count,
            );
        }
        state.position += count;
        i32::try_from(count).map_err(|_| NativeVideoCodecIoCallbackFailure::InvalidArgument)
    })) {
        Ok(Ok(result)) => result,
        Ok(Err(failure)) => record_callback_failure(&mut state.failure, failure),
        Err(_) => record_callback_failure(
            &mut state.failure,
            NativeVideoCodecIoCallbackFailure::Panicked,
        ),
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
unsafe extern "C" fn native_video_codec_output_write(
    opaque: *mut c_void,
    source: *const u8,
    requested: i32,
) -> i32 {
    if opaque.is_null() {
        return abi::AV_ERROR_INVALID_ARGUMENT;
    }
    let state = unsafe { &mut *opaque.cast::<NativeVideoCodecOutputState>() };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        #[cfg(test)]
        if std::mem::take(&mut state.panic_on_next_callback) {
            panic!("injected AVIO output callback panic");
        }
        if state.cancellation.is_cancelled() {
            return Err(NativeVideoCodecIoCallbackFailure::Cancelled);
        }
        if let Some(failure) = state.failure {
            return Err(failure);
        }
        if source.is_null() || requested <= 0 {
            return Err(NativeVideoCodecIoCallbackFailure::InvalidArgument);
        }
        let count = usize::try_from(requested)
            .map_err(|_| NativeVideoCodecIoCallbackFailure::InvalidArgument)?;
        let end = state
            .position
            .checked_add(count)
            .ok_or(NativeVideoCodecIoCallbackFailure::OutputLimit)?;
        if end > state.maximum_bytes {
            return Err(NativeVideoCodecIoCallbackFailure::OutputLimit);
        }
        while state.bytes.len() < end {
            state
                .bytes
                .try_push(0)
                .map_err(|_| NativeVideoCodecIoCallbackFailure::ResourceExhausted)?;
        }
        let source = unsafe { std::slice::from_raw_parts(source, count) };
        state.bytes[state.position..end].copy_from_slice(source);
        state.position = end;
        Ok(requested)
    })) {
        Ok(Ok(result)) => result,
        Ok(Err(failure)) => record_callback_failure(&mut state.failure, failure),
        Err(_) => record_callback_failure(
            &mut state.failure,
            NativeVideoCodecIoCallbackFailure::Panicked,
        ),
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
unsafe extern "C" fn native_video_codec_input_seek(
    opaque: *mut c_void,
    offset: i64,
    whence: i32,
) -> i64 {
    if opaque.is_null() {
        return i64::from(abi::AV_ERROR_INVALID_ARGUMENT);
    }
    let state = unsafe { &mut *opaque.cast::<NativeVideoCodecInputState>() };
    seek_callback(
        &state.cancellation,
        &mut state.failure,
        &mut state.position,
        state.bytes.len(),
        state.maximum_position,
        offset,
        whence,
    )
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
unsafe extern "C" fn native_video_codec_output_seek(
    opaque: *mut c_void,
    offset: i64,
    whence: i32,
) -> i64 {
    if opaque.is_null() {
        return i64::from(abi::AV_ERROR_INVALID_ARGUMENT);
    }
    let state = unsafe { &mut *opaque.cast::<NativeVideoCodecOutputState>() };
    seek_callback(
        &state.cancellation,
        &mut state.failure,
        &mut state.position,
        state.bytes.len(),
        state.maximum_bytes,
        offset,
        whence,
    )
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
fn seek_callback(
    cancellation: &CancellationToken,
    failure: &mut Option<NativeVideoCodecIoCallbackFailure>,
    position: &mut usize,
    logical_length: usize,
    maximum_position: usize,
    offset: i64,
    whence: i32,
) -> i64 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if cancellation.is_cancelled() {
            return Err(NativeVideoCodecIoCallbackFailure::Cancelled);
        }
        if let Some(failure) = *failure {
            return Err(failure);
        }
        let operation = whence & !abi::AV_SEEK_FORCE;
        if operation == abi::AV_SEEK_SIZE {
            return i64::try_from(logical_length)
                .map_err(|_| NativeVideoCodecIoCallbackFailure::InvalidArgument);
        }
        let base = match operation {
            value if value == libc::SEEK_SET => 0_i64,
            value if value == libc::SEEK_CUR => i64::try_from(*position)
                .map_err(|_| NativeVideoCodecIoCallbackFailure::InvalidArgument)?,
            value if value == libc::SEEK_END => i64::try_from(logical_length)
                .map_err(|_| NativeVideoCodecIoCallbackFailure::InvalidArgument)?,
            _ => return Err(NativeVideoCodecIoCallbackFailure::InvalidArgument),
        };
        let next = base
            .checked_add(offset)
            .ok_or(NativeVideoCodecIoCallbackFailure::InvalidArgument)?;
        let next = usize::try_from(next)
            .map_err(|_| NativeVideoCodecIoCallbackFailure::InvalidArgument)?;
        if next > maximum_position {
            return Err(NativeVideoCodecIoCallbackFailure::InvalidArgument);
        }
        *position = next;
        i64::try_from(next).map_err(|_| NativeVideoCodecIoCallbackFailure::InvalidArgument)
    })) {
        Ok(Ok(result)) => result,
        Ok(Err(callback_failure)) => record_callback_failure(failure, callback_failure).into(),
        Err(_) => {
            record_callback_failure(failure, NativeVideoCodecIoCallbackFailure::Panicked).into()
        }
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
fn record_callback_failure(
    failure: &mut Option<NativeVideoCodecIoCallbackFailure>,
    callback_failure: NativeVideoCodecIoCallbackFailure,
) -> i32 {
    if callback_failure == NativeVideoCodecIoCallbackFailure::Cancelled {
        failure.get_or_insert(callback_failure);
        return abi::AV_ERROR_EXIT;
    }
    let first_failure = *failure.get_or_insert(callback_failure);
    match first_failure {
        NativeVideoCodecIoCallbackFailure::Cancelled
        | NativeVideoCodecIoCallbackFailure::Panicked => abi::AV_ERROR_EXIT,
        NativeVideoCodecIoCallbackFailure::InvalidArgument => abi::AV_ERROR_INVALID_ARGUMENT,
        NativeVideoCodecIoCallbackFailure::OutputLimit => abi::AV_ERROR_NO_SPACE,
        NativeVideoCodecIoCallbackFailure::ResourceExhausted => abi::AV_ERROR_OUT_OF_MEMORY,
    }
}

#[derive(Debug, Error)]
pub enum NativeVideoCodecBindingError {
    #[error("native video codec ABI binding was cancelled")]
    Cancelled,
    #[error("native video codec ABI binding is unsupported for this target")]
    UnsupportedTarget,
    #[error("the loaded native video codec contract is incomplete")]
    InvalidLoadedContract,
    #[error("the loaded native video codec source archive does not match the reviewed ABI")]
    SourceArchiveMismatch,
    #[error("native video codec certificate mismatch for {library}")]
    CertificateMismatch { library: &'static str },
    #[error("native video codec symbol {symbol} in {library} could not be resolved")]
    SymbolResolution {
        library: &'static str,
        symbol: &'static str,
    },
    #[error("native video codec symbol {symbol} in {library} resolved from the wrong provider")]
    SymbolProviderMismatch {
        library: &'static str,
        symbol: &'static str,
    },
    #[error("native video codec function-pointer representation is unsupported for {symbol}")]
    InvalidFunctionPointerLayout { symbol: &'static str },
    #[error(
        "native video codec runtime version mismatch for {library}: expected {expected:#x}, got {actual:#x}"
    )]
    RuntimeVersionMismatch {
        library: &'static str,
        expected: u32,
        actual: u32,
    },
}

impl From<CancellationError> for NativeVideoCodecBindingError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

pub fn bind_certified_video_codec_abi(
    load: NativeVideoCodecLoad,
    cancellation: &CancellationToken,
) -> Result<NativeVideoCodecBinding, NativeVideoCodecBindingError> {
    cancellation.check()?;
    if !cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        return Err(NativeVideoCodecBindingError::UnsupportedTarget);
    }
    let projection = VideoCodecBindingProjection::from_load(&load)?;
    let (symbols, versions) =
        bind_video_codec_projection_with_check(&load.loaded, &projection, || cancellation.check())?;
    cancellation.check()?;
    Ok(NativeVideoCodecBinding {
        _symbols: symbols,
        versions,
        load,
    })
}

impl NativeVideoCodecLoad {
    pub fn target(&self) -> &str {
        self.closure.target()
    }

    pub fn primary_catalog_sha256(&self) -> &str {
        self.closure.primary_catalog_sha256()
    }

    pub fn dependency_first_order(&self) -> &[String] {
        self.closure.dependency_first_order()
    }

    pub fn loaded_library_count(&self) -> usize {
        self.loaded.libraries.len()
    }
}

pub fn load_certified_video_codec_closure(
    closure: CertifiedVideoCodecDependencyClosure,
    cancellation: &CancellationToken,
) -> Result<NativeVideoCodecLoad, NativeVideoCodecLoadError> {
    cancellation.check()?;
    if closure.target() != "x86_64-unknown-linux-gnu"
        || !cfg!(all(
            target_os = "linux",
            target_arch = "x86_64",
            target_env = "gnu"
        ))
    {
        return Err(NativeVideoCodecLoadError::UnsupportedTarget);
    }
    let projection = VideoCodecLoadProjection::from_closure(&closure)?;
    let loaded = load_video_codec_projection(&projection, cancellation)?;
    cancellation.check()?;
    Ok(NativeVideoCodecLoad { loaded, closure })
}

#[derive(Clone)]
struct VideoCodecBindingLibraryProjection {
    symbols: BTreeMap<String, u64>,
}

struct VideoCodecBindingProjection {
    libraries: BTreeMap<String, VideoCodecBindingLibraryProjection>,
}

impl VideoCodecBindingProjection {
    fn from_load(load: &NativeVideoCodecLoad) -> Result<Self, NativeVideoCodecBindingError> {
        if load.closure.source_archive_sha256() != abi::FFMPEG_7_1_SOURCE_ARCHIVE_SHA256 {
            return Err(NativeVideoCodecBindingError::SourceArchiveMismatch);
        }
        if load.closure.primary_libraries().len() != 5
            || load.closure.primary_elf_libraries().len() != 5
            || load.closure.primary_certificates().len() != 5
            || load
                .closure
                .dependency_certificates()
                .values()
                .any(|certificate| !certificate.required_symbols().is_empty())
        {
            return Err(NativeVideoCodecBindingError::InvalidLoadedContract);
        }

        let mut libraries = BTreeMap::new();
        for (identity, _major, expected_symbols) in abi::video_codec_library_contracts() {
            let library = load
                .closure
                .primary_libraries()
                .get(identity)
                .ok_or(NativeVideoCodecBindingError::InvalidLoadedContract)?;
            let elf = load
                .closure
                .primary_elf_libraries()
                .get(identity)
                .ok_or(NativeVideoCodecBindingError::InvalidLoadedContract)?;
            let certificate = load
                .closure
                .primary_certificates()
                .get(identity)
                .ok_or(NativeVideoCodecBindingError::InvalidLoadedContract)?;
            let expected_symbols = expected_symbols
                .iter()
                .map(|symbol| (*symbol).to_owned())
                .collect::<BTreeSet<_>>();
            if certificate.library_id() != identity
                || certificate.digest_sha256() != library.digest_sha256()
                || certificate.abi_version()
                    != abi::video_codec_abi_version(identity)
                        .ok_or(NativeVideoCodecBindingError::InvalidLoadedContract)?
                || certificate.required_symbols() != &expected_symbols
                || certificate.unsafe_owner() != VIDEO_CODEC_FFI_UNSAFE_OWNER
                || elf
                    .callable_symbols()
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    != expected_symbols
                || elf.callable_symbols().values().any(|symbol| {
                    abi::video_codec_symbol_version_namespace(identity)
                        != Some(symbol.version_namespace())
                })
                || !load
                    .loaded
                    .libraries
                    .iter()
                    .any(|loaded| loaded.identity == identity)
            {
                return Err(NativeVideoCodecBindingError::CertificateMismatch {
                    library: identity,
                });
            }
            libraries.insert(
                identity.to_owned(),
                VideoCodecBindingLibraryProjection {
                    symbols: elf
                        .callable_symbols()
                        .iter()
                        .map(|(name, symbol)| (name.clone(), symbol.value()))
                        .collect(),
                },
            );
        }
        Ok(Self { libraries })
    }
}

#[allow(
    dead_code,
    reason = "the reviewed table is consumed by subsequent codec leaves"
)]
struct NativeVideoCodecSymbols {
    avcodec: NativeAvcodecSymbols,
    avformat: NativeAvformatSymbols,
    avutil: NativeAvutilSymbols,
    swresample: NativeSwresampleSymbols,
    swscale: NativeSwscaleSymbols,
}

#[allow(
    dead_code,
    reason = "the reviewed table is consumed by subsequent codec leaves"
)]
struct NativeAvcodecSymbols {
    av_packet_alloc: abi::AvPacketAlloc,
    av_packet_free: abi::AvPacketFree,
    av_packet_unref: abi::AvPacketUnref,
    avcodec_alloc_context3: abi::AvcodecAllocContext3,
    avcodec_find_decoder: abi::AvcodecFindDecoder,
    avcodec_find_encoder_by_name: abi::AvcodecFindEncoderByName,
    avcodec_free_context: abi::AvcodecFreeContext,
    avcodec_open2: abi::AvcodecOpen2,
    avcodec_parameters_from_context: abi::AvcodecParametersFromContext,
    avcodec_parameters_to_context: abi::AvcodecParametersToContext,
    avcodec_receive_frame: abi::AvcodecReceiveFrame,
    avcodec_receive_packet: abi::AvcodecReceivePacket,
    avcodec_send_frame: abi::AvcodecSendFrame,
    avcodec_send_packet: abi::AvcodecSendPacket,
    avcodec_version: abi::AvcodecVersion,
}

#[allow(
    dead_code,
    reason = "the reviewed table is consumed by subsequent codec leaves"
)]
struct NativeAvformatSymbols {
    av_find_best_stream: abi::AvFindBestStream,
    av_interleaved_write_frame: abi::AvInterleavedWriteFrame,
    av_read_frame: abi::AvReadFrame,
    av_write_trailer: abi::AvWriteTrailer,
    avformat_alloc_context: abi::AvformatAllocContext,
    avformat_alloc_output_context2: abi::AvformatAllocOutputContext2,
    avformat_close_input: abi::AvformatCloseInput,
    avformat_find_stream_info: abi::AvformatFindStreamInfo,
    avformat_free_context: abi::AvformatFreeContext,
    avformat_new_stream: abi::AvformatNewStream,
    avformat_open_input: abi::AvformatOpenInput,
    avformat_version: abi::AvformatVersion,
    avformat_write_header: abi::AvformatWriteHeader,
    avio_alloc_context: abi::AvioAllocContext,
    avio_context_free: abi::AvioContextFree,
}

#[allow(
    dead_code,
    reason = "the reviewed table is consumed by subsequent codec leaves"
)]
struct NativeAvutilSymbols {
    av_channel_layout_default: abi::AvChannelLayoutDefault,
    av_channel_layout_uninit: abi::AvChannelLayoutUninit,
    av_dict_free: abi::AvDictFree,
    av_dict_set: abi::AvDictSet,
    av_frame_alloc: abi::AvFrameAlloc,
    av_frame_free: abi::AvFrameFree,
    av_frame_get_buffer: abi::AvFrameGetBuffer,
    av_frame_make_writable: abi::AvFrameMakeWritable,
    av_free: abi::AvFree,
    av_malloc: abi::AvMalloc,
    av_opt_set: abi::AvOptSet,
    av_opt_set_int: abi::AvOptSetInt,
    av_rescale_q: abi::AvRescaleQ,
    avutil_version: abi::AvutilVersion,
}

#[allow(
    dead_code,
    reason = "the reviewed table is consumed by subsequent codec leaves"
)]
struct NativeSwresampleSymbols {
    swr_alloc: abi::SwrAlloc,
    swr_alloc_set_opts2: abi::SwrAllocSetOpts2,
    swr_convert: abi::SwrConvert,
    swr_free: abi::SwrFree,
    swr_init: abi::SwrInit,
    swresample_version: abi::SwresampleVersion,
}

#[allow(
    dead_code,
    reason = "the reviewed table is consumed by subsequent codec leaves"
)]
struct NativeSwscaleSymbols {
    sws_free_context: abi::SwsFreeContext,
    sws_get_context: abi::SwsGetContext,
    sws_scale: abi::SwsScale,
    swscale_version: abi::SwscaleVersion,
}

fn bind_video_codec_projection_with_check(
    loaded: &LoadedVideoCodecLibraries,
    projection: &VideoCodecBindingProjection,
    mut check_cancellation: impl FnMut() -> Result<(), CancellationError>,
) -> Result<(NativeVideoCodecSymbols, NativeVideoCodecRuntimeVersions), NativeVideoCodecBindingError>
{
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
    {
        let _ = loaded;
        let _ = projection;
        let _ = &mut check_cancellation;
        Err(NativeVideoCodecBindingError::UnsupportedTarget)
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    {
        check_cancellation()?;
        macro_rules! bind {
            ($library:literal, $symbol:literal, $name:literal, $kind:ty) => {{
                resolve_video_codec_symbol::<$kind>(
                    loaded,
                    projection,
                    $library,
                    $symbol,
                    $name,
                    &mut check_cancellation,
                )?
            }};
        }

        let avcodec = NativeAvcodecSymbols {
            av_packet_alloc: bind!(
                "avcodec",
                "av_packet_alloc",
                c"av_packet_alloc",
                abi::AvPacketAlloc
            ),
            av_packet_free: bind!(
                "avcodec",
                "av_packet_free",
                c"av_packet_free",
                abi::AvPacketFree
            ),
            av_packet_unref: bind!(
                "avcodec",
                "av_packet_unref",
                c"av_packet_unref",
                abi::AvPacketUnref
            ),
            avcodec_alloc_context3: bind!(
                "avcodec",
                "avcodec_alloc_context3",
                c"avcodec_alloc_context3",
                abi::AvcodecAllocContext3
            ),
            avcodec_find_decoder: bind!(
                "avcodec",
                "avcodec_find_decoder",
                c"avcodec_find_decoder",
                abi::AvcodecFindDecoder
            ),
            avcodec_find_encoder_by_name: bind!(
                "avcodec",
                "avcodec_find_encoder_by_name",
                c"avcodec_find_encoder_by_name",
                abi::AvcodecFindEncoderByName
            ),
            avcodec_free_context: bind!(
                "avcodec",
                "avcodec_free_context",
                c"avcodec_free_context",
                abi::AvcodecFreeContext
            ),
            avcodec_open2: bind!(
                "avcodec",
                "avcodec_open2",
                c"avcodec_open2",
                abi::AvcodecOpen2
            ),
            avcodec_parameters_from_context: bind!(
                "avcodec",
                "avcodec_parameters_from_context",
                c"avcodec_parameters_from_context",
                abi::AvcodecParametersFromContext
            ),
            avcodec_parameters_to_context: bind!(
                "avcodec",
                "avcodec_parameters_to_context",
                c"avcodec_parameters_to_context",
                abi::AvcodecParametersToContext
            ),
            avcodec_receive_frame: bind!(
                "avcodec",
                "avcodec_receive_frame",
                c"avcodec_receive_frame",
                abi::AvcodecReceiveFrame
            ),
            avcodec_receive_packet: bind!(
                "avcodec",
                "avcodec_receive_packet",
                c"avcodec_receive_packet",
                abi::AvcodecReceivePacket
            ),
            avcodec_send_frame: bind!(
                "avcodec",
                "avcodec_send_frame",
                c"avcodec_send_frame",
                abi::AvcodecSendFrame
            ),
            avcodec_send_packet: bind!(
                "avcodec",
                "avcodec_send_packet",
                c"avcodec_send_packet",
                abi::AvcodecSendPacket
            ),
            avcodec_version: bind!(
                "avcodec",
                "avcodec_version",
                c"avcodec_version",
                abi::AvcodecVersion
            ),
        };
        let avformat = NativeAvformatSymbols {
            av_find_best_stream: bind!(
                "avformat",
                "av_find_best_stream",
                c"av_find_best_stream",
                abi::AvFindBestStream
            ),
            av_interleaved_write_frame: bind!(
                "avformat",
                "av_interleaved_write_frame",
                c"av_interleaved_write_frame",
                abi::AvInterleavedWriteFrame
            ),
            av_read_frame: bind!(
                "avformat",
                "av_read_frame",
                c"av_read_frame",
                abi::AvReadFrame
            ),
            av_write_trailer: bind!(
                "avformat",
                "av_write_trailer",
                c"av_write_trailer",
                abi::AvWriteTrailer
            ),
            avformat_alloc_context: bind!(
                "avformat",
                "avformat_alloc_context",
                c"avformat_alloc_context",
                abi::AvformatAllocContext
            ),
            avformat_alloc_output_context2: bind!(
                "avformat",
                "avformat_alloc_output_context2",
                c"avformat_alloc_output_context2",
                abi::AvformatAllocOutputContext2
            ),
            avformat_close_input: bind!(
                "avformat",
                "avformat_close_input",
                c"avformat_close_input",
                abi::AvformatCloseInput
            ),
            avformat_find_stream_info: bind!(
                "avformat",
                "avformat_find_stream_info",
                c"avformat_find_stream_info",
                abi::AvformatFindStreamInfo
            ),
            avformat_free_context: bind!(
                "avformat",
                "avformat_free_context",
                c"avformat_free_context",
                abi::AvformatFreeContext
            ),
            avformat_new_stream: bind!(
                "avformat",
                "avformat_new_stream",
                c"avformat_new_stream",
                abi::AvformatNewStream
            ),
            avformat_open_input: bind!(
                "avformat",
                "avformat_open_input",
                c"avformat_open_input",
                abi::AvformatOpenInput
            ),
            avformat_version: bind!(
                "avformat",
                "avformat_version",
                c"avformat_version",
                abi::AvformatVersion
            ),
            avformat_write_header: bind!(
                "avformat",
                "avformat_write_header",
                c"avformat_write_header",
                abi::AvformatWriteHeader
            ),
            avio_alloc_context: bind!(
                "avformat",
                "avio_alloc_context",
                c"avio_alloc_context",
                abi::AvioAllocContext
            ),
            avio_context_free: bind!(
                "avformat",
                "avio_context_free",
                c"avio_context_free",
                abi::AvioContextFree
            ),
        };
        let avutil = NativeAvutilSymbols {
            av_channel_layout_default: bind!(
                "avutil",
                "av_channel_layout_default",
                c"av_channel_layout_default",
                abi::AvChannelLayoutDefault
            ),
            av_channel_layout_uninit: bind!(
                "avutil",
                "av_channel_layout_uninit",
                c"av_channel_layout_uninit",
                abi::AvChannelLayoutUninit
            ),
            av_dict_free: bind!("avutil", "av_dict_free", c"av_dict_free", abi::AvDictFree),
            av_dict_set: bind!("avutil", "av_dict_set", c"av_dict_set", abi::AvDictSet),
            av_frame_alloc: bind!(
                "avutil",
                "av_frame_alloc",
                c"av_frame_alloc",
                abi::AvFrameAlloc
            ),
            av_frame_free: bind!(
                "avutil",
                "av_frame_free",
                c"av_frame_free",
                abi::AvFrameFree
            ),
            av_frame_get_buffer: bind!(
                "avutil",
                "av_frame_get_buffer",
                c"av_frame_get_buffer",
                abi::AvFrameGetBuffer
            ),
            av_frame_make_writable: bind!(
                "avutil",
                "av_frame_make_writable",
                c"av_frame_make_writable",
                abi::AvFrameMakeWritable
            ),
            av_free: bind!("avutil", "av_free", c"av_free", abi::AvFree),
            av_malloc: bind!("avutil", "av_malloc", c"av_malloc", abi::AvMalloc),
            av_opt_set: bind!("avutil", "av_opt_set", c"av_opt_set", abi::AvOptSet),
            av_opt_set_int: bind!(
                "avutil",
                "av_opt_set_int",
                c"av_opt_set_int",
                abi::AvOptSetInt
            ),
            av_rescale_q: bind!("avutil", "av_rescale_q", c"av_rescale_q", abi::AvRescaleQ),
            avutil_version: bind!(
                "avutil",
                "avutil_version",
                c"avutil_version",
                abi::AvutilVersion
            ),
        };
        let swresample = NativeSwresampleSymbols {
            swr_alloc: bind!("swresample", "swr_alloc", c"swr_alloc", abi::SwrAlloc),
            swr_alloc_set_opts2: bind!(
                "swresample",
                "swr_alloc_set_opts2",
                c"swr_alloc_set_opts2",
                abi::SwrAllocSetOpts2
            ),
            swr_convert: bind!("swresample", "swr_convert", c"swr_convert", abi::SwrConvert),
            swr_free: bind!("swresample", "swr_free", c"swr_free", abi::SwrFree),
            swr_init: bind!("swresample", "swr_init", c"swr_init", abi::SwrInit),
            swresample_version: bind!(
                "swresample",
                "swresample_version",
                c"swresample_version",
                abi::SwresampleVersion
            ),
        };
        let swscale = NativeSwscaleSymbols {
            sws_free_context: bind!(
                "swscale",
                "sws_freeContext",
                c"sws_freeContext",
                abi::SwsFreeContext
            ),
            sws_get_context: bind!(
                "swscale",
                "sws_getContext",
                c"sws_getContext",
                abi::SwsGetContext
            ),
            sws_scale: bind!("swscale", "sws_scale", c"sws_scale", abi::SwsScale),
            swscale_version: bind!(
                "swscale",
                "swscale_version",
                c"swscale_version",
                abi::SwscaleVersion
            ),
        };
        let symbols = NativeVideoCodecSymbols {
            avcodec,
            avformat,
            avutil,
            swresample,
            swscale,
        };

        macro_rules! check_version {
            ($library:literal, $function:expr, $expected:expr) => {{
                check_cancellation()?;
                let actual = unsafe { $function() };
                check_cancellation()?;
                if actual != $expected {
                    return Err(NativeVideoCodecBindingError::RuntimeVersionMismatch {
                        library: $library,
                        expected: $expected,
                        actual,
                    });
                }
                actual
            }};
        }
        let versions = NativeVideoCodecRuntimeVersions {
            avcodec: check_version!(
                "avcodec",
                symbols.avcodec.avcodec_version,
                abi::FFMPEG_7_1_AVCODEC_VERSION
            ),
            avformat: check_version!(
                "avformat",
                symbols.avformat.avformat_version,
                abi::FFMPEG_7_1_AVFORMAT_VERSION
            ),
            avutil: check_version!(
                "avutil",
                symbols.avutil.avutil_version,
                abi::FFMPEG_7_1_AVUTIL_VERSION
            ),
            swresample: check_version!(
                "swresample",
                symbols.swresample.swresample_version,
                abi::FFMPEG_7_1_SWRESAMPLE_VERSION
            ),
            swscale: check_version!(
                "swscale",
                symbols.swscale.swscale_version,
                abi::FFMPEG_7_1_SWSCALE_VERSION
            ),
        };
        check_cancellation()?;
        Ok((symbols, versions))
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn resolve_video_codec_symbol<T: Copy>(
    loaded: &LoadedVideoCodecLibraries,
    projection: &VideoCodecBindingProjection,
    library_identity: &'static str,
    symbol_identity: &'static str,
    symbol_name: &'static std::ffi::CStr,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<T, NativeVideoCodecBindingError> {
    use std::ffi::CStr;

    check_cancellation()?;
    if std::mem::size_of::<T>() != std::mem::size_of::<*mut std::ffi::c_void>() {
        return Err(NativeVideoCodecBindingError::InvalidFunctionPointerLayout {
            symbol: symbol_identity,
        });
    }
    let library = loaded
        .libraries
        .iter()
        .find(|library| library.identity == library_identity)
        .ok_or(NativeVideoCodecBindingError::InvalidLoadedContract)?;
    let certified_symbol = projection
        .libraries
        .get(library_identity)
        .and_then(|library| library.symbols.get(symbol_identity))
        .ok_or(NativeVideoCodecBindingError::InvalidLoadedContract)?;
    let version_namespace = video_codec_version_namespace_cstr(library_identity)
        .ok_or(NativeVideoCodecBindingError::InvalidLoadedContract)?;

    unsafe { libc::dlerror() };
    let address = unsafe {
        libc::dlvsym(
            library.handle.as_ptr(),
            symbol_name.as_ptr(),
            version_namespace.as_ptr(),
        )
    };
    let lookup_error = unsafe { libc::dlerror() };
    if !lookup_error.is_null() || address.is_null() {
        return Err(NativeVideoCodecBindingError::SymbolResolution {
            library: library_identity,
            symbol: symbol_identity,
        });
    }
    check_cancellation()?;

    let link_map = loaded_link_map(library).map_err(|_| {
        NativeVideoCodecBindingError::SymbolProviderMismatch {
            library: library_identity,
            symbol: symbol_identity,
        }
    })?;
    let expected_address = unsafe { (*link_map).address }
        .checked_add(usize::try_from(*certified_symbol).map_err(|_| {
            NativeVideoCodecBindingError::SymbolProviderMismatch {
                library: library_identity,
                symbol: symbol_identity,
            }
        })?)
        .ok_or(NativeVideoCodecBindingError::SymbolProviderMismatch {
            library: library_identity,
            symbol: symbol_identity,
        })?;
    let mut information = unsafe { std::mem::zeroed::<libc::Dl_info>() };
    let mut provider: *mut std::ffi::c_void = std::ptr::null_mut();
    let status = unsafe {
        libc::dladdr1(
            address.cast_const(),
            std::ptr::addr_of_mut!(information),
            std::ptr::addr_of_mut!(provider),
            2,
        )
    };
    if status == 0
        || provider.cast::<LoaderLinkMap>() != link_map
        || information.dli_fname.is_null()
        || information.dli_sname.is_null()
        || information.dli_fbase as usize != unsafe { (*link_map).address }
        || information.dli_saddr != address
        || expected_address != address as usize
        || unsafe { CStr::from_ptr(information.dli_fname) }.to_bytes()
            != library.path.as_os_str().as_encoded_bytes()
        || unsafe { CStr::from_ptr(information.dli_sname) }.to_bytes() != symbol_name.to_bytes()
    {
        return Err(NativeVideoCodecBindingError::SymbolProviderMismatch {
            library: library_identity,
            symbol: symbol_identity,
        });
    }
    check_cancellation()?;
    let function = unsafe { std::mem::transmute_copy::<*mut std::ffi::c_void, T>(&address) };
    Ok(function)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn video_codec_version_namespace_cstr(identity: &str) -> Option<&'static std::ffi::CStr> {
    match identity {
        "avcodec" => Some(c"LIBAVCODEC_61"),
        "avformat" => Some(c"LIBAVFORMAT_61"),
        "avutil" => Some(c"LIBAVUTIL_59"),
        "swresample" => Some(c"LIBSWRESAMPLE_5"),
        "swscale" => Some(c"LIBSWSCALE_8"),
        _ => None,
    }
}

struct VideoCodecLoadProjection {
    paths: BTreeMap<String, PathBuf>,
    sonames: BTreeMap<String, String>,
    needed: BTreeMap<String, BTreeSet<String>>,
    system_libraries: BTreeSet<String>,
    dependency_first_order: Vec<String>,
}

impl VideoCodecLoadProjection {
    fn from_closure(
        closure: &CertifiedVideoCodecDependencyClosure,
    ) -> Result<Self, NativeVideoCodecLoadError> {
        let paths = closure
            .retained_loader_paths()
            .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
        let dependency_first_order = closure.dependency_first_order().to_vec();
        if paths.len() != dependency_first_order.len() {
            return Err(NativeVideoCodecLoadError::InvalidClosure);
        }
        let mut sonames = closure
            .primary_libraries()
            .iter()
            .map(|(identity, library)| (identity.clone(), library.filename().to_owned()))
            .collect::<BTreeMap<_, _>>();
        for (identity, dependency) in closure.dependencies() {
            if sonames
                .insert(identity.clone(), dependency.filename().to_owned())
                .is_some()
            {
                return Err(NativeVideoCodecLoadError::InvalidClosure);
            }
        }
        if sonames.len() != paths.len()
            || dependency_first_order
                .iter()
                .any(|identity| !paths.contains_key(identity) || !sonames.contains_key(identity))
        {
            return Err(NativeVideoCodecLoadError::InvalidClosure);
        }
        let mut needed = sonames
            .keys()
            .map(|identity| (identity.clone(), BTreeSet::new()))
            .collect::<BTreeMap<_, _>>();
        for edge in closure.edges() {
            let required = sonames
                .get(edge.dependency())
                .cloned()
                .unwrap_or_else(|| edge.dependency().to_owned());
            needed
                .get_mut(edge.consumer())
                .ok_or(NativeVideoCodecLoadError::InvalidClosure)?
                .insert(required);
        }
        Ok(Self {
            paths,
            sonames,
            needed,
            system_libraries: closure.reviewed_system_libraries().clone(),
            dependency_first_order,
        })
    }
}

struct LoadedVideoCodecLibrary {
    identity: String,
    path: PathBuf,
    handle: std::ptr::NonNull<std::ffi::c_void>,
    namespace: libc::c_long,
}

struct LoadedVideoCodecLibraries {
    libraries: Vec<LoadedVideoCodecLibrary>,
    _thread_bound: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl Drop for LoadedVideoCodecLibraries {
    fn drop(&mut self) {
        while let Some(library) = self.libraries.pop() {
            close_loaded_library(library);
        }
    }
}

fn load_video_codec_projection(
    projection: &VideoCodecLoadProjection,
    cancellation: &CancellationToken,
) -> Result<LoadedVideoCodecLibraries, NativeVideoCodecLoadError> {
    load_video_codec_projection_with_check(projection, || cancellation.check())
}

fn load_video_codec_projection_with_check(
    projection: &VideoCodecLoadProjection,
    mut check_cancellation: impl FnMut() -> Result<(), CancellationError>,
) -> Result<LoadedVideoCodecLibraries, NativeVideoCodecLoadError> {
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
    {
        let _ = projection;
        let _ = &mut check_cancellation;
        Err(NativeVideoCodecLoadError::UnsupportedTarget)
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    {
        check_cancellation()?;
        if projection.paths.len() != projection.dependency_first_order.len()
            || projection.sonames.len() != projection.dependency_first_order.len()
            || projection.needed.len() != projection.dependency_first_order.len()
        {
            return Err(NativeVideoCodecLoadError::InvalidClosure);
        }
        let mut libraries = Vec::new();
        libraries
            .try_reserve_exact(projection.dependency_first_order.len())
            .map_err(|_| NativeVideoCodecLoadError::ResourceExhausted)?;
        let mut loaded = LoadedVideoCodecLibraries {
            libraries,
            _thread_bound: std::marker::PhantomData,
        };
        let mut namespace = None;
        for identity in &projection.dependency_first_order {
            check_cancellation()?;
            let path = projection
                .paths
                .get(identity)
                .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
            let library = open_loaded_library(identity, path, namespace)?;
            namespace = Some(library.namespace);
            loaded.libraries.push(library);
            check_cancellation()?;
        }
        prove_exact_loaded_bindings(&loaded.libraries, projection, &mut check_cancellation)?;
        check_cancellation()?;
        Ok(loaded)
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn open_loaded_library(
    identity: &str,
    path: &std::path::Path,
    namespace: Option<libc::c_long>,
) -> Result<LoadedVideoCodecLibrary, NativeVideoCodecLoadError> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};

    let path_string = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        NativeVideoCodecLoadError::LibraryLoad {
            identity: identity.to_owned(),
            reason: "retained loader path contains an interior NUL".to_owned(),
        }
    })?;
    let handle = unsafe {
        libc::dlmopen(
            namespace.unwrap_or(libc::LM_ID_NEWLM),
            path_string.as_ptr(),
            libc::RTLD_NOW | libc::RTLD_LOCAL,
        )
    };
    let handle =
        std::ptr::NonNull::new(handle).ok_or_else(|| NativeVideoCodecLoadError::LibraryLoad {
            identity: identity.to_owned(),
            reason: dynamic_loader_error(),
        })?;
    let mut actual_namespace = 0;
    let namespace_status = unsafe {
        libc::dlinfo(
            handle.as_ptr(),
            libc::RTLD_DI_LMID,
            std::ptr::addr_of_mut!(actual_namespace).cast(),
        )
    };
    if namespace_status != 0
        || namespace.is_none() && actual_namespace == libc::LM_ID_BASE
        || namespace.is_some_and(|expected| expected != actual_namespace)
    {
        let status = unsafe { libc::dlclose(handle.as_ptr()) };
        if status != 0 {
            eprintln!("native video codec loader failed to close a rejected handle");
        }
        return Err(NativeVideoCodecLoadError::BindingProof(format!(
            "isolated loader namespace could not be proven for {identity}"
        )));
    }
    Ok(LoadedVideoCodecLibrary {
        identity: identity.to_owned(),
        path: path.to_owned(),
        handle,
        namespace: actual_namespace,
    })
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn close_loaded_library(library: LoadedVideoCodecLibrary) {
    let status = unsafe { libc::dlclose(library.handle.as_ptr()) };
    if status != 0 {
        eprintln!(
            "native video codec loader failed to close {}",
            library.identity
        );
    }
    #[cfg(test)]
    if let Ok(mut log) = TEST_CLOSE_ORDER.lock() {
        log.push(library.identity);
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
fn close_loaded_library(library: LoadedVideoCodecLibrary) {
    let _ = library;
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn dynamic_loader_error() -> String {
    use std::ffi::CStr;

    let error = unsafe { libc::dlerror() };
    if error.is_null() {
        "dynamic loader returned no diagnostic".to_owned()
    } else {
        unsafe { CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[repr(C)]
struct LoaderLinkMap {
    address: usize,
    name: *const libc::c_char,
    dynamic: *mut std::ffi::c_void,
    next: *mut LoaderLinkMap,
    previous: *mut LoaderLinkMap,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[repr(C)]
struct LoaderElfDynamic {
    tag: i64,
    value: u64,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn loaded_link_map(
    library: &LoadedVideoCodecLibrary,
) -> Result<*mut LoaderLinkMap, NativeVideoCodecLoadError> {
    let mut link_map: *mut LoaderLinkMap = std::ptr::null_mut();
    let status = unsafe {
        libc::dlinfo(
            library.handle.as_ptr(),
            libc::RTLD_DI_LINKMAP,
            std::ptr::addr_of_mut!(link_map).cast(),
        )
    };
    if status != 0 || link_map.is_null() {
        Err(NativeVideoCodecLoadError::BindingProof(format!(
            "loader returned no link map for {}",
            library.identity
        )))
    } else {
        Ok(link_map)
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
unsafe fn bounded_loaded_string(
    string_table: *const u8,
    offset: u64,
) -> Result<String, NativeVideoCodecLoadError> {
    let offset = usize::try_from(offset).map_err(|_| {
        NativeVideoCodecLoadError::BindingProof(
            "loaded dynamic string offset exceeds the address space".to_owned(),
        )
    })?;
    let start = unsafe { string_table.add(offset) };
    let mut length = 0;
    while length <= 255 {
        if unsafe { *start.add(length) } == 0 {
            let bytes = unsafe { std::slice::from_raw_parts(start, length) };
            return std::str::from_utf8(bytes).map(str::to_owned).map_err(|_| {
                NativeVideoCodecLoadError::BindingProof(
                    "loaded dynamic string is not UTF-8".to_owned(),
                )
            });
        }
        length += 1;
    }
    Err(NativeVideoCodecLoadError::BindingProof(
        "loaded dynamic string exceeds 255 bytes".to_owned(),
    ))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
unsafe fn loaded_dynamic_identity(
    link_map: *mut LoaderLinkMap,
) -> Result<(String, BTreeSet<String>), NativeVideoCodecLoadError> {
    let dynamic = unsafe { (*link_map).dynamic.cast::<LoaderElfDynamic>() };
    if dynamic.is_null() {
        return Err(NativeVideoCodecLoadError::BindingProof(
            "loaded object has no dynamic table".to_owned(),
        ));
    }
    let mut string_table = std::ptr::null();
    let mut soname_offset = None;
    let mut needed_offsets = Vec::new();
    let mut terminated = false;
    for index in 0..65_536 {
        let entry = unsafe { &*dynamic.add(index) };
        match entry.tag {
            0 => {
                terminated = true;
                break;
            }
            1 => needed_offsets.push(entry.value),
            5 => string_table = entry.value as *const u8,
            14 => soname_offset = Some(entry.value),
            _ => {}
        }
    }
    if !terminated || string_table.is_null() {
        return Err(NativeVideoCodecLoadError::BindingProof(
            "loaded object has an invalid dynamic string table".to_owned(),
        ));
    }
    let soname_offset = soname_offset.ok_or_else(|| {
        NativeVideoCodecLoadError::BindingProof("loaded object has no DT_SONAME".to_owned())
    })?;
    let soname = unsafe { bounded_loaded_string(string_table, soname_offset) }?;
    let mut needed = BTreeSet::new();
    for offset in needed_offsets {
        let dependency = unsafe { bounded_loaded_string(string_table, offset) }?;
        if !needed.insert(dependency) {
            return Err(NativeVideoCodecLoadError::BindingProof(format!(
                "{soname} repeats a DT_NEEDED entry"
            )));
        }
    }
    Ok((soname, needed))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn prove_exact_loaded_bindings(
    libraries: &[LoadedVideoCodecLibrary],
    projection: &VideoCodecLoadProjection,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLoadError> {
    use std::{ffi::CStr, os::unix::ffi::OsStrExt};

    let expected_paths = projection
        .paths
        .iter()
        .map(|(identity, path)| (path.as_os_str().as_bytes().to_vec(), identity.as_str()))
        .collect::<BTreeMap<_, _>>();
    let expected_by_soname = projection
        .sonames
        .iter()
        .map(|(identity, soname)| (soname.as_str(), identity.as_str()))
        .collect::<BTreeMap<_, _>>();
    if expected_by_soname.len() != projection.sonames.len() {
        return Err(NativeVideoCodecLoadError::InvalidClosure);
    }

    let mut maps_by_identity = BTreeMap::new();
    for library in libraries {
        check_cancellation()?;
        let expected_path = projection
            .paths
            .get(&library.identity)
            .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
        let link_map = loaded_link_map(library)?;
        let actual_name = unsafe { (*link_map).name };
        if actual_name.is_null() {
            return Err(NativeVideoCodecLoadError::BindingProof(format!(
                "loader returned no object path for {}",
                library.identity
            )));
        }
        let actual_path = unsafe { CStr::from_ptr(actual_name) }.to_bytes();
        if actual_path != expected_path.as_os_str().as_bytes()
            || library.path.as_os_str().as_bytes() != actual_path
        {
            return Err(NativeVideoCodecLoadError::BindingProof(format!(
                "{} did not resolve to its retained descriptor",
                library.identity
            )));
        }
        if maps_by_identity
            .insert(library.identity.as_str(), link_map)
            .is_some()
        {
            return Err(NativeVideoCodecLoadError::BindingProof(format!(
                "loader returned duplicate handle for {}",
                library.identity
            )));
        }
    }

    let mut head = *maps_by_identity
        .values()
        .next()
        .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
    let mut walked = BTreeSet::new();
    while !head.is_null() {
        check_cancellation()?;
        if !walked.insert(head as usize) || walked.len() > 4_096 {
            return Err(NativeVideoCodecLoadError::BindingProof(
                "loader namespace link map is cyclic or exceeds 4096 objects".to_owned(),
            ));
        }
        let previous = unsafe { (*head).previous };
        if previous.is_null() {
            break;
        }
        head = previous;
    }

    walked.clear();
    let mut observed_explicit = BTreeSet::new();
    let mut current = head;
    while !current.is_null() {
        check_cancellation()?;
        if !walked.insert(current as usize) || walked.len() > 4_096 {
            return Err(NativeVideoCodecLoadError::BindingProof(
                "loader namespace link map is cyclic or exceeds 4096 objects".to_owned(),
            ));
        }
        let name_pointer = unsafe { (*current).name };
        if name_pointer.is_null() {
            return Err(NativeVideoCodecLoadError::BindingProof(
                "loader namespace contains an unnamed object".to_owned(),
            ));
        }
        let loaded_path = unsafe { CStr::from_ptr(name_pointer) }.to_bytes();
        if let Some(identity) = expected_paths.get(loaded_path) {
            if !observed_explicit.insert(*identity) {
                return Err(NativeVideoCodecLoadError::BindingProof(format!(
                    "certified object {identity} appears more than once"
                )));
            }
        } else if !loaded_path.is_empty() {
            let basename = loaded_path
                .rsplit(|byte| *byte == b'/')
                .next()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .unwrap_or_default();
            if expected_by_soname.contains_key(basename) {
                return Err(NativeVideoCodecLoadError::BindingProof(format!(
                    "ambient object duplicates certified SONAME {basename}"
                )));
            }
            if !projection.system_libraries.contains(basename) {
                return Err(NativeVideoCodecLoadError::BindingProof(format!(
                    "loader namespace contains undeclared object {}",
                    String::from_utf8_lossy(loaded_path)
                )));
            }
        }
        current = unsafe { (*current).next };
    }
    if observed_explicit.len() != projection.paths.len() {
        return Err(NativeVideoCodecLoadError::BindingProof(
            "loader namespace omits a certified object".to_owned(),
        ));
    }

    for (identity, link_map) in maps_by_identity {
        check_cancellation()?;
        let (actual_soname, actual_needed) = unsafe { loaded_dynamic_identity(link_map) }?;
        let expected_soname = projection
            .sonames
            .get(identity)
            .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
        let expected_needed = projection
            .needed
            .get(identity)
            .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
        if &actual_soname != expected_soname || &actual_needed != expected_needed {
            return Err(NativeVideoCodecLoadError::BindingProof(format!(
                "loaded dynamic identity differs for {identity}"
            )));
        }
    }
    Ok(())
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
fn prove_exact_loaded_bindings(
    _libraries: &[LoadedVideoCodecLibrary],
    _projection: &VideoCodecLoadProjection,
    _check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLoadError> {
    Err(NativeVideoCodecLoadError::UnsupportedTarget)
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
static TEST_CLOSE_ORDER: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

#[cfg(all(test, target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
static TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(test, target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
mod tests {
    use super::*;
    use crate::{
        native_ffi_elf::inspect_elf64_dynamic_contract, trust::capture_native_library_image,
    };
    use comfy_tensor::{CpuWorkspaceAuthority, StreamId};
    use std::{
        fs,
        process::Command,
        sync::{
            Mutex,
            atomic::{AtomicBool, Ordering},
        },
    };

    static AVIO_EVENTS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    static AVIO_MALLOC_RETURNS_NULL: AtomicBool = AtomicBool::new(false);
    static AVIO_CONTEXT_RETURNS_NULL: AtomicBool = AtomicBool::new(false);

    fn record_avio_event(event: &'static str) {
        match AVIO_EVENTS.lock() {
            Ok(mut events) => events.push(event),
            Err(_) => std::process::abort(),
        }
    }

    fn take_avio_events() -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
        let mut events = AVIO_EVENTS
            .lock()
            .map_err(|_| "AVIO event log mutex was poisoned")?;
        Ok(std::mem::take(&mut *events))
    }

    unsafe extern "C" fn mock_av_malloc(bytes: usize) -> *mut c_void {
        record_avio_event("malloc");
        if AVIO_MALLOC_RETURNS_NULL.swap(false, Ordering::AcqRel) {
            return std::ptr::null_mut();
        }
        unsafe { libc::malloc(bytes) }
    }

    unsafe extern "C" fn mock_av_free(pointer: *mut c_void) {
        record_avio_event("free");
        unsafe { libc::free(pointer) };
    }

    unsafe extern "C" fn mock_avio_alloc_context(
        buffer: *mut u8,
        _buffer_size: i32,
        _write: i32,
        _opaque: *mut c_void,
        _read: Option<abi::AvIoReadPacket>,
        _write_packet: Option<abi::AvIoWritePacket>,
        _seek: Option<abi::AvIoSeek>,
    ) -> *mut abi::AvIoContext {
        record_avio_event("alloc_context");
        if AVIO_CONTEXT_RETURNS_NULL.swap(false, Ordering::AcqRel) {
            return std::ptr::null_mut();
        }
        Box::into_raw(Box::new(abi::AvIoContext {
            class: std::ptr::null(),
            buffer,
        }))
    }

    unsafe extern "C" fn mock_avio_context_free(context: *mut *mut abi::AvIoContext) {
        record_avio_event("context_free");
        if context.is_null() {
            return;
        }
        let pointer = unsafe { *context };
        if !pointer.is_null() {
            drop(unsafe { Box::from_raw(pointer) });
            unsafe { *context = std::ptr::null_mut() };
        }
    }

    fn mock_avio_functions() -> NativeVideoCodecAvioFunctions {
        NativeVideoCodecAvioFunctions {
            av_malloc: mock_av_malloc,
            av_free: mock_av_free,
            avio_alloc_context: mock_avio_alloc_context,
            avio_context_free: mock_avio_context_free,
        }
    }

    fn avio_context<'a>(
        cancellation: &'a CancellationToken,
        scratch_bytes: u64,
    ) -> Result<(CpuBackend, ExecutionContext<'a>), Box<dyn std::error::Error>> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(scratch_bytes)?;
        let context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority.authorize_workspace(scratch_bytes)?,
            rng_phase: None,
            cancellation,
        };
        Ok((backend, context))
    }

    struct LoaderFixture {
        _directory: tempfile::TempDir,
        _retained: Vec<crate::trust::RetainedNativeLibraryImage>,
        projection: VideoCodecLoadProjection,
    }

    struct BindingFixture {
        _directory: tempfile::TempDir,
        _retained: Vec<crate::trust::RetainedNativeLibraryImage>,
        load_projection: VideoCodecLoadProjection,
        binding_projection: VideoCodecBindingProjection,
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "the Linux-only binding test synchronously compiles tiny reviewed-symbol ELF fixtures before dlmopen"
    )]
    fn binding_fixture(
        changed_version: Option<(&str, u32)>,
    ) -> Result<BindingFixture, Box<dyn std::error::Error>> {
        use std::fmt::Write as _;

        let directory = tempfile::tempdir()?;
        let cancellation = CancellationToken::default();
        let mut retained = Vec::new();
        let mut paths = BTreeMap::new();
        let mut sonames = BTreeMap::new();
        let mut needed = BTreeMap::new();
        let mut binding_libraries = BTreeMap::new();
        for (identity, major, symbols) in abi::video_codec_library_contracts() {
            let namespace = abi::video_codec_symbol_version_namespace(identity)
                .ok_or("binding fixture namespace is missing")?;
            let version_symbol = match identity {
                "avcodec" => "avcodec_version",
                "avformat" => "avformat_version",
                "avutil" => "avutil_version",
                "swresample" => "swresample_version",
                "swscale" => "swscale_version",
                _ => return Err("binding fixture identity is unsupported".into()),
            };
            let expected_version = match identity {
                "avcodec" => abi::FFMPEG_7_1_AVCODEC_VERSION,
                "avformat" => abi::FFMPEG_7_1_AVFORMAT_VERSION,
                "avutil" => abi::FFMPEG_7_1_AVUTIL_VERSION,
                "swresample" => abi::FFMPEG_7_1_SWRESAMPLE_VERSION,
                "swscale" => abi::FFMPEG_7_1_SWSCALE_VERSION,
                _ => return Err("binding fixture identity is unsupported".into()),
            };
            let actual_version = changed_version
                .filter(|(changed_identity, _)| *changed_identity == identity)
                .map_or(expected_version, |(_, version)| version);
            let mut source = String::new();
            for symbol in symbols {
                if *symbol == version_symbol {
                    writeln!(
                        source,
                        "unsigned int {symbol}(void) {{ return {actual_version}u; }}"
                    )?;
                } else {
                    writeln!(source, "void {symbol}(void) {{}}")?;
                }
            }
            let mut version_script = format!("{namespace} {{\n  global:\n");
            for symbol in symbols {
                writeln!(version_script, "    {symbol};")?;
            }
            version_script.push_str("  local: *;\n};\n");

            let source_path = directory.path().join(format!("{identity}.c"));
            let script_path = directory.path().join(format!("{identity}.map"));
            fs::write(&source_path, source)?;
            fs::write(&script_path, version_script)?;
            let soname = format!("lib{identity}.so.{major}");
            let library_path = directory.path().join(&soname);
            let output = Command::new("cc")
                .arg("-shared")
                .arg("-fPIC")
                .arg(format!("-Wl,-soname,{soname}"))
                .arg(format!("-Wl,--version-script={}", script_path.display()))
                .arg(&source_path)
                .arg("-o")
                .arg(&library_path)
                .output()?;
            if !output.status.success() {
                return Err(format!(
                    "fixture {identity} compiler failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }

            let bytes = fs::read(&library_path)?;
            let dynamic = inspect_elf64_dynamic_contract(&bytes, 62, &cancellation)?;
            let symbol_values = symbols
                .iter()
                .map(|symbol| {
                    let identities = dynamic
                        .symbol_identities()
                        .get(*symbol)
                        .ok_or("fixture symbol identity is missing")?;
                    let [symbol_identity] = identities.as_slice() else {
                        return Err("fixture symbol identity is ambiguous");
                    };
                    Ok(((*symbol).to_owned(), symbol_identity.value))
                })
                .collect::<Result<BTreeMap<_, _>, &str>>()?;
            let captured = capture_native_library_image(&library_path, &cancellation)?;
            let image = captured.seal(&format!("video-binding-{identity}"), &cancellation)?;
            paths.insert(identity.to_owned(), image.loader_path().to_path_buf());
            sonames.insert(identity.to_owned(), soname);
            needed.insert(identity.to_owned(), dynamic.needed().clone());
            binding_libraries.insert(
                identity.to_owned(),
                VideoCodecBindingLibraryProjection {
                    symbols: symbol_values,
                },
            );
            retained.push(image);
        }
        let package_sonames = sonames.values().cloned().collect::<BTreeSet<_>>();
        let system_libraries = needed
            .values()
            .flat_map(BTreeSet::iter)
            .filter(|name| !package_sonames.contains(*name))
            .cloned()
            .collect();
        let dependency_first_order = abi::video_codec_library_contracts()
            .into_iter()
            .map(|(identity, _, _)| identity.to_owned())
            .collect();
        Ok(BindingFixture {
            _directory: directory,
            _retained: retained,
            load_projection: VideoCodecLoadProjection {
                paths,
                sonames,
                needed,
                system_libraries,
                dependency_first_order,
            },
            binding_projection: VideoCodecBindingProjection {
                libraries: binding_libraries,
            },
        })
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "the Linux-only retained-loader test synchronously compiles two tiny ELF fixtures before dlmopen"
    )]
    fn fixture() -> Result<LoaderFixture, Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let dependency_source = directory.path().join("dependency.c");
        fs::write(
            &dependency_source,
            "int video_codec_dependency(void) { return 7; }\n",
        )?;
        let dependency = directory.path().join("libvideo_dependency.so.1");
        let output = Command::new("cc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-Wl,-soname,libvideo_dependency.so.1")
            .arg(&dependency_source)
            .arg("-o")
            .arg(&dependency)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "fixture dependency compiler failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let consumer_source = directory.path().join("consumer.c");
        fs::write(
            &consumer_source,
            "extern int video_codec_dependency(void);\nint video_codec_consumer(void) { return video_codec_dependency(); }\n",
        )?;
        let consumer = directory.path().join("libvideo_consumer.so.1");
        let output = Command::new("cc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-Wl,-soname,libvideo_consumer.so.1")
            .arg("-Wl,-z,defs")
            .arg(&consumer_source)
            .arg("-L")
            .arg(directory.path())
            .arg("-l:libvideo_dependency.so.1")
            .arg("-o")
            .arg(&consumer)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "fixture consumer compiler failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let cancellation = CancellationToken::default();
        let mut retained = Vec::new();
        let mut paths = BTreeMap::new();
        let mut sonames = BTreeMap::new();
        let mut needed = BTreeMap::new();
        for (identity, path, soname) in [
            ("dependency", dependency, "libvideo_dependency.so.1"),
            ("consumer", consumer, "libvideo_consumer.so.1"),
        ] {
            let bytes = fs::read(&path)?;
            let dynamic = inspect_elf64_dynamic_contract(&bytes, 62, &cancellation)?;
            let captured = capture_native_library_image(&path, &cancellation)?;
            let image = captured.seal(&format!("video-loader-{identity}"), &cancellation)?;
            paths.insert(identity.to_owned(), image.loader_path().to_path_buf());
            sonames.insert(identity.to_owned(), soname.to_owned());
            needed.insert(identity.to_owned(), dynamic.needed().clone());
            retained.push(image);
        }
        let package_sonames = sonames.values().cloned().collect::<BTreeSet<_>>();
        let system_libraries = needed
            .values()
            .flat_map(BTreeSet::iter)
            .filter(|name| !package_sonames.contains(*name))
            .cloned()
            .collect();
        Ok(LoaderFixture {
            _directory: directory,
            _retained: retained,
            projection: VideoCodecLoadProjection {
                paths,
                sonames,
                needed,
                system_libraries,
                dependency_first_order: vec!["dependency".to_owned(), "consumer".to_owned()],
            },
        })
    }

    fn reset_close_order() -> Result<(), Box<dyn std::error::Error>> {
        TEST_CLOSE_ORDER
            .lock()
            .map_err(|_| "video codec close-order mutex was poisoned")?
            .clear();
        Ok(())
    }

    fn close_order() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(TEST_CLOSE_ORDER
            .lock()
            .map_err(|_| "video codec close-order mutex was poisoned")?
            .clone())
    }

    #[test]
    fn retained_video_codec_loader_uses_one_isolated_exact_namespace()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = fixture()?;
        let loaded =
            load_video_codec_projection(&fixture.projection, &CancellationToken::default())?;
        assert_eq!(loaded.libraries.len(), 2);
        assert_eq!(loaded.libraries[0].identity, "dependency");
        assert_eq!(loaded.libraries[1].identity, "consumer");
        assert_eq!(loaded.libraries[0].namespace, loaded.libraries[1].namespace);
        drop(loaded);
        assert_eq!(close_order()?, ["consumer", "dependency"]);
        Ok(())
    }

    #[test]
    fn retained_video_codec_loader_rolls_back_binding_failure_in_reverse_order()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let mut fixture = fixture()?;
        fixture
            .projection
            .needed
            .get_mut("consumer")
            .ok_or("fixture consumer dependency set is missing")?
            .clear();
        assert!(matches!(
            load_video_codec_projection(&fixture.projection, &CancellationToken::default(),),
            Err(NativeVideoCodecLoadError::BindingProof(_))
        ));
        assert_eq!(close_order()?, ["consumer", "dependency"]);
        Ok(())
    }

    #[test]
    fn retained_video_codec_loader_discards_late_cancellation_and_retries_cleanly()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = fixture()?;
        let cancellation = CancellationToken::default();
        let mut checks = 0;
        assert!(matches!(
            load_video_codec_projection_with_check(&fixture.projection, || {
                checks += 1;
                if checks == 5 {
                    cancellation.cancel();
                }
                cancellation.check()
            }),
            Err(NativeVideoCodecLoadError::Cancelled)
        ));
        assert_eq!(close_order()?, ["consumer", "dependency"]);

        reset_close_order()?;
        let loaded =
            load_video_codec_projection(&fixture.projection, &CancellationToken::default())?;
        drop(loaded);
        assert_eq!(close_order()?, ["consumer", "dependency"]);
        Ok(())
    }

    #[test]
    fn retained_video_codec_binding_resolves_exact_primary_symbols_and_versions()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = binding_fixture(None)?;
        let loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        let (symbols, versions) = bind_video_codec_projection_with_check(
            &loaded,
            &fixture.binding_projection,
            || Ok(()),
        )?;
        assert_eq!(versions.avcodec(), abi::FFMPEG_7_1_AVCODEC_VERSION);
        assert_eq!(versions.avformat(), abi::FFMPEG_7_1_AVFORMAT_VERSION);
        assert_eq!(versions.avutil(), abi::FFMPEG_7_1_AVUTIL_VERSION);
        assert_eq!(versions.swresample(), abi::FFMPEG_7_1_SWRESAMPLE_VERSION);
        assert_eq!(versions.swscale(), abi::FFMPEG_7_1_SWSCALE_VERSION);
        drop(symbols);
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );
        Ok(())
    }

    #[test]
    fn retained_video_codec_binding_rejects_version_and_provider_mismatch_atomically()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = binding_fixture(Some(("avformat", 0x3d0765)))?;
        let loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        assert!(matches!(
            bind_video_codec_projection_with_check(&loaded, &fixture.binding_projection, || Ok(()),),
            Err(NativeVideoCodecBindingError::RuntimeVersionMismatch {
                library: "avformat",
                ..
            })
        ));
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );

        reset_close_order()?;
        let fixture = binding_fixture(None)?;
        let loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        let mut wrong_projection = fixture.binding_projection;
        *wrong_projection
            .libraries
            .get_mut("avcodec")
            .and_then(|library| library.symbols.get_mut("avcodec_version"))
            .ok_or("fixture avcodec version identity is missing")? += 1;
        assert!(matches!(
            bind_video_codec_projection_with_check(&loaded, &wrong_projection, || Ok(())),
            Err(NativeVideoCodecBindingError::SymbolProviderMismatch {
                library: "avcodec",
                symbol: "avcodec_version",
            })
        ));
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );
        Ok(())
    }

    #[test]
    fn retained_video_codec_binding_discards_cancellation_and_retries_cleanly()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = binding_fixture(None)?;
        let loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        let cancellation = CancellationToken::default();
        let mut checks = 0;
        assert!(matches!(
            bind_video_codec_projection_with_check(&loaded, &fixture.binding_projection, || {
                checks += 1;
                if checks == 9 {
                    cancellation.cancel();
                }
                cancellation.check()
            },),
            Err(NativeVideoCodecBindingError::Cancelled)
        ));
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );

        reset_close_order()?;
        let loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        bind_video_codec_projection_with_check(&loaded, &fixture.binding_projection, || Ok(()))?;
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );
        Ok(())
    }

    #[test]
    fn bounded_video_codec_avio_callbacks_enforce_read_write_seek_and_limits()
    -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::default();
        let mut input = NativeVideoCodecInputState {
            bytes: Arc::from([1_u8, 2, 3, 4]),
            position: 0,
            maximum_position: 8,
            cancellation: cancellation.clone(),
            failure: None,
            panic_on_next_callback: false,
        };
        let mut destination = [0_u8; 3];
        assert_eq!(
            unsafe {
                native_video_codec_input_read(
                    std::ptr::addr_of_mut!(input).cast(),
                    destination.as_mut_ptr(),
                    3,
                )
            },
            3
        );
        assert_eq!(destination, [1, 2, 3]);
        assert_eq!(
            unsafe {
                native_video_codec_input_read(
                    std::ptr::addr_of_mut!(input).cast(),
                    destination.as_mut_ptr(),
                    3,
                )
            },
            1
        );
        assert_eq!(
            unsafe {
                native_video_codec_input_read(
                    std::ptr::addr_of_mut!(input).cast(),
                    destination.as_mut_ptr(),
                    3,
                )
            },
            abi::AV_ERROR_END_OF_FILE
        );
        assert_eq!(
            unsafe {
                native_video_codec_input_seek(
                    std::ptr::addr_of_mut!(input).cast(),
                    0,
                    abi::AV_SEEK_SIZE,
                )
            },
            4
        );
        assert_eq!(
            unsafe {
                native_video_codec_input_seek(
                    std::ptr::addr_of_mut!(input).cast(),
                    6,
                    libc::SEEK_SET | abi::AV_SEEK_FORCE,
                )
            },
            6
        );
        assert_eq!(
            unsafe {
                native_video_codec_input_read(
                    std::ptr::addr_of_mut!(input).cast(),
                    destination.as_mut_ptr(),
                    3,
                )
            },
            abi::AV_ERROR_END_OF_FILE
        );

        let (backend, context) = avio_context(&cancellation, 64 * 1024)?;
        let mut output = NativeVideoCodecOutputState {
            bytes: backend.workspace_vec(&context, 8)?,
            position: 0,
            maximum_bytes: 8,
            cancellation: cancellation.clone(),
            failure: None,
            panic_on_next_callback: false,
        };
        let first = [5_u8, 6];
        assert_eq!(
            unsafe {
                native_video_codec_output_write(
                    std::ptr::addr_of_mut!(output).cast(),
                    first.as_ptr(),
                    2,
                )
            },
            2
        );
        assert_eq!(
            unsafe {
                native_video_codec_output_seek(
                    std::ptr::addr_of_mut!(output).cast(),
                    4,
                    libc::SEEK_SET,
                )
            },
            4
        );
        let last = [9_u8];
        assert_eq!(
            unsafe {
                native_video_codec_output_write(
                    std::ptr::addr_of_mut!(output).cast(),
                    last.as_ptr(),
                    1,
                )
            },
            1
        );
        assert_eq!(&*output.bytes, &[5, 6, 0, 0, 9]);
        assert_eq!(
            unsafe {
                native_video_codec_output_seek(
                    std::ptr::addr_of_mut!(output).cast(),
                    8,
                    libc::SEEK_SET,
                )
            },
            8
        );
        assert_eq!(
            unsafe {
                native_video_codec_output_write(
                    std::ptr::addr_of_mut!(output).cast(),
                    last.as_ptr(),
                    1,
                )
            },
            abi::AV_ERROR_NO_SPACE
        );
        assert_eq!(&*output.bytes, &[5, 6, 0, 0, 9]);
        assert_eq!(
            unsafe {
                native_video_codec_output_write(
                    std::ptr::addr_of_mut!(output).cast(),
                    last.as_ptr(),
                    1,
                )
            },
            abi::AV_ERROR_NO_SPACE
        );
        assert!(matches!(
            callback_status(&output.cancellation, output.failure),
            Err(NativeVideoCodecIoError::OutputLimitExceeded)
        ));
        drop(output);
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn bounded_video_codec_avio_cancellation_and_panics_are_latched()
    -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::default();
        let mut input = NativeVideoCodecInputState {
            bytes: Arc::from([1_u8]),
            position: 0,
            maximum_position: 1,
            cancellation: cancellation.clone(),
            failure: None,
            panic_on_next_callback: true,
        };
        let mut destination = [0_u8; 1];
        assert_eq!(
            unsafe {
                native_video_codec_input_read(
                    std::ptr::addr_of_mut!(input).cast(),
                    destination.as_mut_ptr(),
                    1,
                )
            },
            abi::AV_ERROR_EXIT
        );
        assert!(matches!(
            callback_status(&input.cancellation, input.failure),
            Err(NativeVideoCodecIoError::CallbackPanicked)
        ));
        cancellation.cancel();
        assert_eq!(
            unsafe {
                native_video_codec_input_read(
                    std::ptr::addr_of_mut!(input).cast(),
                    destination.as_mut_ptr(),
                    1,
                )
            },
            abi::AV_ERROR_EXIT
        );
        assert!(matches!(
            callback_status(&input.cancellation, input.failure),
            Err(NativeVideoCodecIoError::Cancelled)
        ));
        Ok(())
    }

    #[test]
    fn retained_video_codec_avio_allocation_raii_and_retry_are_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        take_avio_events()?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 64 * 1024)?;
        let avio = allocate_native_video_codec_avio_inner(
            (),
            mock_avio_functions(),
            64,
            false,
            None,
            None,
            None,
            &backend,
            &context,
            &mut || cancellation.check(),
        )?;
        assert_eq!(take_avio_events()?, ["malloc", "alloc_context"]);
        let original_buffer = unsafe { (*avio.context.as_ptr()).buffer };
        unsafe { mock_av_free(original_buffer.cast()) };
        let replacement = unsafe { mock_av_malloc(64) }.cast::<u8>();
        if replacement.is_null() {
            return Err("replacement AVIO buffer allocation failed".into());
        }
        unsafe { (*avio.context.as_ptr()).buffer = replacement };
        take_avio_events()?;
        drop(avio);
        assert_eq!(take_avio_events()?, ["free", "context_free"]);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        AVIO_MALLOC_RETURNS_NULL.store(true, Ordering::Release);
        assert!(matches!(
            allocate_native_video_codec_avio_inner(
                (),
                mock_avio_functions(),
                64,
                false,
                None,
                None,
                None,
                &backend,
                &context,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecIoError::NativeAllocationFailed)
        ));
        assert_eq!(context.scratch.in_use_bytes(), 0);

        AVIO_CONTEXT_RETURNS_NULL.store(true, Ordering::Release);
        assert!(matches!(
            allocate_native_video_codec_avio_inner(
                (),
                mock_avio_functions(),
                64,
                false,
                None,
                None,
                None,
                &backend,
                &context,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecIoError::NativeAllocationFailed)
        ));
        assert_eq!(context.scratch.in_use_bytes(), 0);
        take_avio_events()?;

        let (constrained_backend, constrained_context) = avio_context(&cancellation, 4_159)?;
        assert!(matches!(
            allocate_native_video_codec_avio_inner(
                (),
                mock_avio_functions(),
                64,
                false,
                None,
                None,
                None,
                &constrained_backend,
                &constrained_context,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecIoError::Tensor(_))
        ));
        assert!(take_avio_events()?.is_empty());
        assert_eq!(constrained_context.scratch.in_use_bytes(), 0);

        let mut checks = 0;
        assert!(matches!(
            allocate_native_video_codec_avio_inner(
                (),
                mock_avio_functions(),
                64,
                false,
                None,
                None,
                None,
                &backend,
                &context,
                &mut || {
                    checks += 1;
                    if checks == 3 {
                        Err(CancellationError)
                    } else {
                        Ok(())
                    }
                },
            ),
            Err(NativeVideoCodecIoError::Cancelled)
        ));
        assert_eq!(take_avio_events()?, ["malloc", "free"]);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        let mut checks = 0;
        assert!(matches!(
            allocate_native_video_codec_avio_inner(
                (),
                mock_avio_functions(),
                64,
                false,
                None,
                None,
                None,
                &backend,
                &context,
                &mut || {
                    checks += 1;
                    if checks == 4 {
                        Err(CancellationError)
                    } else {
                        Ok(())
                    }
                },
            ),
            Err(NativeVideoCodecIoError::Cancelled)
        ));
        assert_eq!(
            take_avio_events()?,
            ["malloc", "alloc_context", "free", "context_free"]
        );
        assert_eq!(context.scratch.in_use_bytes(), 0);

        let retry = allocate_native_video_codec_avio_inner(
            (),
            mock_avio_functions(),
            64,
            false,
            None,
            None,
            None,
            &backend,
            &context,
            &mut || Ok(()),
        )?;
        drop(retry);
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }
}
