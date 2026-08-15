use crate::{
    CertifiedVideoCodecDependencyClosure, VIDEO_CODEC_FFI_UNSAFE_OWNER,
    native_video_codec_abi as abi,
};
use comfy_media::NativeVideoCrf;
use comfy_tensor::{
    CpuBackend, CpuWorkspaceLease, CpuWorkspaceVec, DType, DeviceId, ExecutionContext, ImageTensor,
    Layout, Rgb8ImageTensor, TensorDescriptor, TensorError, ViewAccess,
};
use comfy_types::{CancellationError, CancellationToken};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{CString, c_void},
    io::{self, Write},
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
    #[cfg(test)]
    pub(crate) const fn from_components(
        avcodec: u32,
        avformat: u32,
        avutil: u32,
        swresample: u32,
        swscale: u32,
    ) -> Self {
        Self {
            avcodec,
            avformat,
            avutil,
            swresample,
            swscale,
        }
    }

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

pub struct NativeLtxvH264Codec {
    binding: NativeVideoCodecBinding,
    #[allow(dead_code, reason = "consumed by the bounded LTXV H.264 session")]
    encoder: NonNull<abi::AvCodec>,
    #[allow(dead_code, reason = "consumed by the bounded LTXV H.264 session")]
    decoder: NonNull<abi::AvCodec>,
}

pub(crate) struct NativeVideoCodecSuite {
    ltxv_h264: NativeLtxvH264Codec,
    #[allow(dead_code, reason = "consumed by the bounded AAC MP4 session")]
    aac_encoder: NonNull<abi::AvCodec>,
    #[allow(dead_code, reason = "consumed by the bounded AV1 WebM session")]
    svt_av1_encoder: NonNull<abi::AvCodec>,
    #[allow(dead_code, reason = "consumed by the bounded VP9 WebM session")]
    vpx_vp9_encoder: NonNull<abi::AvCodec>,
    #[allow(dead_code, reason = "consumed by the bounded AAC decode session")]
    aac_decoder: NonNull<abi::AvCodec>,
    #[allow(dead_code, reason = "consumed by the bounded VP9 decode session")]
    vp9_decoder: NonNull<abi::AvCodec>,
    #[allow(dead_code, reason = "consumed by the bounded AV1 decode session")]
    av1_decoder: NonNull<abi::AvCodec>,
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeVp9WebmEncodeLimits {
    maximum_output_bytes: usize,
    avio_buffer_bytes: usize,
    maximum_native_session_bytes: u64,
    maximum_packet_iterations: usize,
}

#[allow(
    dead_code,
    reason = "consumed by the following owned codec-thread byte bridge"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeVp9WebmBatchLimits {
    session: NativeVp9WebmEncodeLimits,
    maximum_frames: usize,
    maximum_pixels_per_frame: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    dead_code,
    reason = "constructed by the following SaveWEBM prepared-effect adapter"
)]
pub(crate) struct NativeVideoContainerMetadataLimits {
    maximum_entries: usize,
    maximum_key_bytes: usize,
    maximum_value_bytes: usize,
    maximum_aggregate_bytes: usize,
}

impl NativeVideoContainerMetadataLimits {
    #[allow(
        dead_code,
        reason = "constructed by the following SaveWEBM prepared-effect adapter"
    )]
    pub(crate) fn checked(
        maximum_entries: usize,
        maximum_key_bytes: usize,
        maximum_value_bytes: usize,
        maximum_aggregate_bytes: usize,
    ) -> Result<Self, NativeVideoContainerMetadataError> {
        if maximum_entries == 0
            || maximum_key_bytes == 0
            || maximum_value_bytes == 0
            || maximum_aggregate_bytes == 0
        {
            return Err(NativeVideoContainerMetadataError::InvalidLimits);
        }
        Ok(Self {
            maximum_entries,
            maximum_key_bytes,
            maximum_value_bytes,
            maximum_aggregate_bytes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeVideoContainerMetadata {
    entries: Vec<(CString, CString)>,
}

impl NativeVideoContainerMetadata {
    #[allow(
        dead_code,
        reason = "constructed by the following SaveWEBM prepared-effect adapter"
    )]
    pub(crate) fn checked(
        entries: Vec<(String, String)>,
        limits: NativeVideoContainerMetadataLimits,
    ) -> Result<Self, NativeVideoContainerMetadataError> {
        if entries.len() > limits.maximum_entries {
            return Err(NativeVideoContainerMetadataError::LimitExceeded);
        }
        let mut aggregate_bytes = 0_usize;
        let mut checked_entries = Vec::with_capacity(entries.len());
        for (key, value) in entries {
            let key_bytes = key.as_bytes();
            let value_bytes = value.as_bytes();
            if key_bytes.is_empty() {
                return Err(NativeVideoContainerMetadataError::EmptyKey);
            }
            if key_bytes.contains(&0) || value_bytes.contains(&0) {
                return Err(NativeVideoContainerMetadataError::EmbeddedNul);
            }
            if key_bytes.len() > limits.maximum_key_bytes
                || value_bytes.len() > limits.maximum_value_bytes
            {
                return Err(NativeVideoContainerMetadataError::LimitExceeded);
            }
            aggregate_bytes = aggregate_bytes
                .checked_add(key_bytes.len())
                .and_then(|bytes| bytes.checked_add(1))
                .and_then(|bytes| bytes.checked_add(value_bytes.len()))
                .and_then(|bytes| bytes.checked_add(1))
                .filter(|bytes| *bytes <= limits.maximum_aggregate_bytes)
                .ok_or(NativeVideoContainerMetadataError::LimitExceeded)?;
            let key =
                CString::new(key).map_err(|_| NativeVideoContainerMetadataError::EmbeddedNul)?;
            let value =
                CString::new(value).map_err(|_| NativeVideoContainerMetadataError::EmbeddedNul)?;
            checked_entries.push((key, value));
        }
        Ok(Self {
            entries: checked_entries,
        })
    }

    pub(crate) fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub(crate) fn entries(&self) -> &[(CString, CString)] {
        &self.entries
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[allow(
    dead_code,
    reason = "returned to the following SaveWEBM prepared-effect adapter"
)]
pub(crate) enum NativeVideoContainerMetadataError {
    #[error("native video container metadata limits are invalid")]
    InvalidLimits,
    #[error("native video container metadata keys cannot be empty")]
    EmptyKey,
    #[error("native video container metadata contains an embedded NUL byte")]
    EmbeddedNul,
    #[error("native video container metadata exceeds its checked limits")]
    LimitExceeded,
}

#[allow(
    dead_code,
    reason = "consumed by the following owned codec-thread byte bridge"
)]
impl NativeVp9WebmBatchLimits {
    pub(crate) fn checked(
        session: NativeVp9WebmEncodeLimits,
        maximum_frames: usize,
        maximum_pixels_per_frame: u64,
    ) -> Result<Self, NativeVideoCodecVp9EncodeError> {
        if maximum_frames == 0 || maximum_pixels_per_frame == 0 {
            return Err(NativeVideoCodecVp9EncodeError::InvalidLimits);
        }
        Ok(Self {
            session,
            maximum_frames,
            maximum_pixels_per_frame,
        })
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
impl NativeVp9WebmEncodeLimits {
    pub(crate) fn checked(
        maximum_output_bytes: usize,
        avio_buffer_bytes: usize,
        maximum_native_session_bytes: u64,
        maximum_packet_iterations: usize,
    ) -> Result<Self, NativeVideoCodecVp9EncodeError> {
        if maximum_output_bytes == 0
            || avio_buffer_bytes == 0
            || maximum_native_session_bytes == 0
            || maximum_packet_iterations == 0
        {
            return Err(NativeVideoCodecVp9EncodeError::InvalidLimits);
        }
        Ok(Self {
            maximum_output_bytes,
            avio_buffer_bytes,
            maximum_native_session_bytes,
            maximum_packet_iterations,
        })
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
pub(crate) struct NativeVp9Webm<'suite> {
    _suite: PhantomData<&'suite NativeVideoCodecSuite>,
    output: NativeVideoCodecMemoryOutput<'suite>,
    width: i32,
    height: i32,
    frame_rate: abi::AvRational,
    frame_count: usize,
    has_alpha: bool,
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
impl NativeVp9Webm<'_> {
    pub(crate) fn encoded_bytes(&self) -> Result<&[u8], NativeVideoCodecVp9EncodeError> {
        let bytes = self.output.staged_bytes()?;
        if bytes.is_empty() {
            return Err(NativeVideoCodecVp9EncodeError::EmptyOutput);
        }
        Ok(bytes)
    }

    pub(crate) fn dimensions(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    pub(crate) fn frame_rate(&self) -> (i32, i32) {
        (self.frame_rate.numerator, self.frame_rate.denominator)
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
    reason = "consumed by the following codec-thread batch bridge"
)]
#[derive(Debug, Error)]
pub(crate) enum NativeVideoCodecVp9EncodeError {
    #[error("native VP9 WebM encoding was cancelled")]
    Cancelled,
    #[error("native VP9 WebM encoding received an invalid RGB8 frame")]
    InvalidInput,
    #[error("native VP9 WebM encoding received an invalid IMAGE batch")]
    InvalidBatch,
    #[error("native VP9 WebM encoding received invalid resource limits")]
    InvalidLimits,
    #[error("native VP9 WebM encoding requires CRF in the source range 0 through 63")]
    InvalidCrf,
    #[error("native VP9 WebM encoding received an invalid frame rate")]
    InvalidFrameRate,
    #[error("native VP9 WebM allocation failed during {phase}")]
    NativeAllocation { phase: &'static str },
    #[error("native VP9 WebM resources were exhausted during {phase}")]
    ResourceExhausted { phase: &'static str },
    #[error("native VP9 WebM operation {phase} failed with status {status}")]
    NativeCall { phase: &'static str, status: i32 },
    #[error("native VP9 WebM codec options were not fully consumed")]
    UnconsumedCodecOptions,
    #[error("native VP9 WebM packet draining exceeded its checked iteration limit")]
    PacketIterationLimit,
    #[error("native VP9 WebM encoding produced no bytes")]
    EmptyOutput,
    #[error(transparent)]
    Io(#[from] NativeVideoCodecIoError),
    #[error("native VP9 WebM tensor operation failed: {0}")]
    Tensor(#[source] TensorError),
}

impl From<CancellationError> for NativeVideoCodecVp9EncodeError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

#[derive(Debug, Error)]
pub(crate) enum NativeVideoCodecSuiteAdmissionError {
    #[error("native video codec-suite admission was cancelled")]
    Cancelled,
    #[error("native video codec-suite admission is unsupported for this target")]
    UnsupportedTarget,
    #[error("the retained native video codec-suite dependency contract is incomplete")]
    InvalidDependencyContract,
    #[error("the retained FFmpeg registry has no `{encoder}` encoder")]
    MissingEncoder { encoder: &'static str },
    #[error("the retained FFmpeg registry has no `{decoder}` decoder")]
    MissingDecoder { decoder: &'static str },
    #[error("the `{codec}` codec descriptor came from the wrong loaded image")]
    DescriptorProviderMismatch { codec: &'static str },
}

impl From<CancellationError> for NativeVideoCodecSuiteAdmissionError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeLtxvH264EncodeLimits {
    maximum_output_bytes: usize,
    avio_buffer_bytes: usize,
    maximum_native_session_bytes: u64,
    maximum_packet_iterations: usize,
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
impl NativeLtxvH264EncodeLimits {
    pub(crate) fn checked(
        maximum_output_bytes: usize,
        avio_buffer_bytes: usize,
        maximum_native_session_bytes: u64,
        maximum_packet_iterations: usize,
    ) -> Result<Self, NativeVideoCodecLtxvEncodeError> {
        if maximum_output_bytes == 0
            || avio_buffer_bytes == 0
            || maximum_native_session_bytes == 0
            || maximum_packet_iterations == 0
        {
            return Err(NativeVideoCodecLtxvEncodeError::InvalidLimits);
        }
        Ok(Self {
            maximum_output_bytes,
            avio_buffer_bytes,
            maximum_native_session_bytes,
            maximum_packet_iterations,
        })
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
pub(crate) struct NativeLtxvH264Mp4<'codec> {
    codec: &'codec NativeLtxvH264Codec,
    output: NativeVideoCodecMemoryOutput<'codec>,
    width: i32,
    height: i32,
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
impl NativeLtxvH264Mp4<'_> {
    pub(crate) fn encoded_bytes(&self) -> Result<&[u8], NativeVideoCodecLtxvEncodeError> {
        let bytes = self.output.staged_bytes()?;
        if bytes.is_empty() {
            return Err(NativeVideoCodecLtxvEncodeError::EmptyOutput);
        }
        Ok(bytes)
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeLtxvH264DemuxLimits {
    maximum_input_bytes: usize,
    avio_buffer_bytes: usize,
    maximum_native_session_bytes: u64,
    maximum_streams: usize,
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
impl NativeLtxvH264DemuxLimits {
    pub(crate) fn checked(
        maximum_input_bytes: usize,
        avio_buffer_bytes: usize,
        maximum_native_session_bytes: u64,
        maximum_streams: usize,
    ) -> Result<Self, NativeVideoCodecLtxvDemuxError> {
        if maximum_input_bytes == 0
            || avio_buffer_bytes == 0
            || maximum_native_session_bytes == 0
            || maximum_streams == 0
        {
            return Err(NativeVideoCodecLtxvDemuxError::InvalidLimits);
        }
        Ok(Self {
            maximum_input_bytes,
            avio_buffer_bytes,
            maximum_native_session_bytes,
            maximum_streams,
        })
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
pub(crate) struct NativeLtxvH264Demux<'codec, 'bytes> {
    format: NativeLtxvInputFormatContext,
    input: NativeVideoCodecMemoryInput<'codec, 'bytes>,
    _native_session_workspace: CpuWorkspaceLease,
    video_stream_index: i32,
    expected_width: i32,
    expected_height: i32,
    codec: &'codec NativeLtxvH264Codec,
    _thread_bound: PhantomData<std::rc::Rc<()>>,
}

#[allow(
    dead_code,
    reason = "constructed by the following source-compatible LTXV tensor adapter"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeLtxvH264DecodeLimits {
    maximum_packet_iterations: usize,
    maximum_receive_iterations: usize,
    maximum_width: u64,
    maximum_height: u64,
    maximum_pixels: u64,
    maximum_output_bytes: usize,
    maximum_native_session_bytes: u64,
}

impl NativeLtxvH264DecodeLimits {
    #[allow(
        dead_code,
        reason = "constructed by the following source-compatible LTXV tensor adapter"
    )]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn checked(
        maximum_packet_iterations: usize,
        maximum_receive_iterations: usize,
        maximum_width: u64,
        maximum_height: u64,
        maximum_pixels: u64,
        maximum_output_bytes: usize,
        maximum_native_session_bytes: u64,
    ) -> Result<Self, NativeVideoCodecLtxvDecodeError> {
        if maximum_packet_iterations == 0
            || maximum_receive_iterations == 0
            || maximum_width == 0
            || maximum_height == 0
            || maximum_pixels == 0
            || maximum_output_bytes == 0
            || maximum_native_session_bytes == 0
        {
            return Err(NativeVideoCodecLtxvDecodeError::InvalidLimits);
        }
        Ok(Self {
            maximum_packet_iterations,
            maximum_receive_iterations,
            maximum_width,
            maximum_height,
            maximum_pixels,
            maximum_output_bytes,
            maximum_native_session_bytes,
        })
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following native LTXVPreprocess node adapter"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeLtxvH264PreprocessLimits {
    maximum_batch: u64,
    maximum_output_elements: usize,
    encode: NativeLtxvH264EncodeLimits,
    demux: NativeLtxvH264DemuxLimits,
    decode: NativeLtxvH264DecodeLimits,
}

impl NativeLtxvH264PreprocessLimits {
    #[allow(
        dead_code,
        reason = "constructed by the following native LTXVPreprocess node adapter"
    )]
    pub(crate) fn checked(
        maximum_batch: u64,
        maximum_output_elements: usize,
        encode: NativeLtxvH264EncodeLimits,
        demux: NativeLtxvH264DemuxLimits,
        decode: NativeLtxvH264DecodeLimits,
    ) -> Result<Self, NativeVideoCodecLtxvPreprocessError> {
        if maximum_batch == 0 || maximum_output_elements == 0 {
            return Err(NativeVideoCodecLtxvPreprocessError::InvalidLimits);
        }
        Ok(Self {
            maximum_batch,
            maximum_output_elements,
            encode,
            demux,
            decode,
        })
    }

    pub(crate) fn configuration_values(&self) -> [u64; 17] {
        [
            self.maximum_batch,
            u64::try_from(self.maximum_output_elements).unwrap_or(u64::MAX),
            u64::try_from(self.encode.maximum_output_bytes).unwrap_or(u64::MAX),
            u64::try_from(self.encode.avio_buffer_bytes).unwrap_or(u64::MAX),
            self.encode.maximum_native_session_bytes,
            u64::try_from(self.encode.maximum_packet_iterations).unwrap_or(u64::MAX),
            u64::try_from(self.demux.maximum_input_bytes).unwrap_or(u64::MAX),
            u64::try_from(self.demux.avio_buffer_bytes).unwrap_or(u64::MAX),
            self.demux.maximum_native_session_bytes,
            u64::try_from(self.demux.maximum_streams).unwrap_or(u64::MAX),
            u64::try_from(self.decode.maximum_packet_iterations).unwrap_or(u64::MAX),
            u64::try_from(self.decode.maximum_receive_iterations).unwrap_or(u64::MAX),
            self.decode.maximum_width,
            self.decode.maximum_height,
            self.decode.maximum_pixels,
            u64::try_from(self.decode.maximum_output_bytes).unwrap_or(u64::MAX),
            self.decode.maximum_native_session_bytes,
        ]
    }
}

#[allow(
    dead_code,
    reason = "returned through the following native LTXVPreprocess node adapter"
)]
#[derive(Debug, Error)]
pub(crate) enum NativeVideoCodecLtxvPreprocessError {
    #[error("native LTXV preprocessing was cancelled")]
    Cancelled,
    #[error("native LTXV preprocessing received invalid resource limits")]
    InvalidLimits,
    #[error("native LTXV preprocessing compression must be in the inclusive range 0 through 100")]
    InvalidCompression,
    #[error(
        "native LTXV preprocessing requires a nonempty CPU F32 BHWC image and RGB channels for compression"
    )]
    InvalidInput,
    #[error("native LTXV preprocessing exceeded its reviewed output bounds")]
    ResourceExhausted,
    #[error("native LTXV preprocessing tensor operation failed: {0}")]
    Tensor(#[source] TensorError),
    #[error("native LTXV preprocessing encode failed: {0}")]
    Encode(#[source] NativeVideoCodecLtxvEncodeError),
    #[error("native LTXV preprocessing demux failed: {0}")]
    Demux(#[source] NativeVideoCodecLtxvDemuxError),
    #[error("native LTXV preprocessing decode failed: {0}")]
    Decode(#[source] NativeVideoCodecLtxvDecodeError),
}

impl From<CancellationError> for NativeVideoCodecLtxvPreprocessError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

impl From<TensorError> for NativeVideoCodecLtxvPreprocessError {
    fn from(error: TensorError) -> Self {
        map_ltxv_preprocess_tensor_error(error)
    }
}

#[allow(
    dead_code,
    reason = "returned through the following source-compatible LTXV tensor adapter"
)]
#[derive(Debug, Error)]
pub(crate) enum NativeVideoCodecLtxvDecodeError {
    #[error("native LTXV H.264 first-frame decode was cancelled")]
    Cancelled,
    #[error("native LTXV H.264 first-frame decode received invalid resource limits")]
    InvalidLimits,
    #[error("native LTXV H.264 first-frame decode exhausted resources during {phase}")]
    ResourceExhausted { phase: &'static str },
    #[error("native LTXV H.264 first-frame decode allocation failed during {phase}")]
    NativeAllocation { phase: &'static str },
    #[error("native LTXV H.264 first-frame decode failed during {phase} with status {status}")]
    NativeCall { phase: &'static str, status: i32 },
    #[error("native LTXV H.264 first-frame decode exceeded its packet iteration limit")]
    PacketIterationLimit,
    #[error("native LTXV H.264 first-frame decode exceeded its receive iteration limit")]
    ReceiveIterationLimit,
    #[error("native LTXV H.264 input yielded no decoded frame")]
    MissingFrame,
    #[error("native LTXV H.264 decoded frame exceeded its reviewed bounds")]
    InvalidFrame,
    #[error("native LTXV H.264 decoder made no protocol progress")]
    ProtocolStalled,
    #[error(transparent)]
    Io(#[from] NativeVideoCodecIoError),
    #[error("native LTXV H.264 tensor operation failed: {0}")]
    Tensor(#[source] TensorError),
}

impl From<CancellationError> for NativeVideoCodecLtxvDecodeError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
#[derive(Debug, Error)]
pub(crate) enum NativeVideoCodecLtxvDemuxError {
    #[error("native LTXV H.264 MP4 demux was cancelled")]
    Cancelled,
    #[error("native LTXV H.264 MP4 demux received invalid resource limits")]
    InvalidLimits,
    #[error("native LTXV H.264 MP4 input was empty")]
    EmptyInput,
    #[error("native LTXV H.264 MP4 format allocation failed")]
    NativeAllocation,
    #[error("native LTXV H.264 MP4 resources were exhausted during {phase}")]
    ResourceExhausted { phase: &'static str },
    #[error("native LTXV H.264 MP4 open failed with status {status}")]
    OpenFailed { status: i32 },
    #[error("native LTXV H.264 MP4 declared too many streams")]
    StreamLimitExceeded,
    #[error("native LTXV H.264 MP4 contained no video stream")]
    MissingVideoStream,
    #[error("the first native LTXV video stream was not H.264")]
    UnexpectedVideoCodec,
    #[error("native LTXV H.264 MP4 open replaced the retained memory input")]
    InputContextMismatch,
    #[error(transparent)]
    Io(#[from] NativeVideoCodecIoError),
    #[error("native LTXV H.264 demux tensor operation failed: {0}")]
    Tensor(#[source] TensorError),
}

impl From<CancellationError> for NativeVideoCodecLtxvDemuxError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

impl NativeLtxvH264Codec {
    pub fn target(&self) -> &str {
        self.binding.target()
    }

    pub fn primary_catalog_sha256(&self) -> &str {
        self.binding.primary_catalog_sha256()
    }

    pub fn runtime_versions(&self) -> NativeVideoCodecRuntimeVersions {
        self.binding.runtime_versions()
    }

    pub(crate) fn admit_video_suite(
        self,
        cancellation: &CancellationToken,
    ) -> Result<NativeVideoCodecSuite, NativeVideoCodecSuiteAdmissionError> {
        if !cfg!(all(
            target_os = "linux",
            target_arch = "x86_64",
            target_env = "gnu"
        )) {
            return Err(NativeVideoCodecSuiteAdmissionError::UnsupportedTarget);
        }
        if !has_exact_video_codec_suite_dependency_contract(&self.binding) {
            return Err(NativeVideoCodecSuiteAdmissionError::InvalidDependencyContract);
        }
        let descriptors = admit_video_suite_with_check(
            &self.binding._symbols,
            &self.binding.load.loaded,
            || cancellation.check(),
        )?;
        cancellation.check()?;
        Ok(NativeVideoCodecSuite {
            ltxv_h264: self,
            aac_encoder: descriptors.aac_encoder,
            svt_av1_encoder: descriptors.svt_av1_encoder,
            vpx_vp9_encoder: descriptors.vpx_vp9_encoder,
            aac_decoder: descriptors.aac_decoder,
            vp9_decoder: descriptors.vp9_decoder,
            av1_decoder: descriptors.av1_decoder,
        })
    }

    #[allow(
        dead_code,
        reason = "consumed by the following retained H.264 decode leaf"
    )]
    pub(crate) fn encode_rgb8_frame<'codec>(
        &'codec self,
        frame: &Rgb8ImageTensor,
        crf: u8,
        limits: NativeLtxvH264EncodeLimits,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeLtxvH264Mp4<'codec>, NativeVideoCodecLtxvEncodeError> {
        let (input, width, height) = validate_ltxv_h264_encode_input(frame, crf)?;
        context.check().map_err(map_ltxv_tensor_error)?;
        let _native_session_workspace = backend
            .reserve_workspace(context, limits.maximum_native_session_bytes)
            .map_err(map_ltxv_tensor_error)?;
        let mut output = self.binding.open_bounded_avio_output(
            limits.maximum_output_bytes,
            limits.avio_buffer_bytes,
            backend,
            context,
        )?;
        let functions = NativeLtxvH264EncodeFunctions::from_codec(self);
        encode_ltxv_h264_rgb8_with_check(
            self.encoder,
            input,
            width,
            height,
            crf,
            limits.maximum_packet_iterations,
            &functions,
            &mut output,
            &mut || context.cancellation.check(),
        )?;
        context.check().map_err(map_ltxv_tensor_error)?;
        if output.staged_bytes()?.is_empty() {
            return Err(NativeVideoCodecLtxvEncodeError::EmptyOutput);
        }
        Ok(NativeLtxvH264Mp4 {
            codec: self,
            output,
            width,
            height,
        })
    }

    #[allow(
        dead_code,
        reason = "consumed by the following native LTXVPreprocess node adapter"
    )]
    pub(crate) fn preprocess_image(
        &self,
        image: &ImageTensor,
        compression: u8,
        limits: NativeLtxvH264PreprocessLimits,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<ImageTensor, NativeVideoCodecLtxvPreprocessError> {
        preprocess_ltxv_image_with_round_trip(
            image,
            compression,
            limits,
            backend,
            context,
            &mut |frame, backend, context| {
                let encoded = self
                    .encode_rgb8_frame(frame, compression, limits.encode, backend, context)
                    .map_err(NativeVideoCodecLtxvPreprocessError::Encode)?;
                let demux = encoded
                    .open_first_h264_video_stream(limits.demux, backend, context)
                    .map_err(NativeVideoCodecLtxvPreprocessError::Demux)?;
                demux
                    .decode_first_rgb8_frame(limits.decode, backend, context)
                    .map_err(NativeVideoCodecLtxvPreprocessError::Decode)
            },
        )
    }
}

impl NativeVideoCodecSuite {
    pub(crate) fn target(&self) -> &str {
        self.ltxv_h264.target()
    }

    pub(crate) fn primary_catalog_sha256(&self) -> &str {
        self.ltxv_h264.primary_catalog_sha256()
    }

    pub(crate) fn runtime_versions(&self) -> NativeVideoCodecRuntimeVersions {
        self.ltxv_h264.runtime_versions()
    }

    pub(crate) fn preprocess_image(
        &self,
        image: &ImageTensor,
        compression: u8,
        limits: NativeLtxvH264PreprocessLimits,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<ImageTensor, NativeVideoCodecLtxvPreprocessError> {
        self.ltxv_h264
            .preprocess_image(image, compression, limits, backend, context)
    }

    #[allow(
        dead_code,
        reason = "consumed by the following codec-thread batch bridge"
    )]
    pub(crate) fn encode_vp9_rgb8_frame<'suite>(
        &'suite self,
        frame: &Rgb8ImageTensor,
        frame_rate: (u64, u64),
        crf: NativeVideoCrf,
        limits: NativeVp9WebmEncodeLimits,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeVp9Webm<'suite>, NativeVideoCodecVp9EncodeError> {
        let (input, width, height) = validate_vp9_rgb8_encode_input(frame)?;
        let frame_rate = checked_vp9_frame_rate(frame_rate)?;
        context.check().map_err(map_vp9_tensor_error)?;
        let _native_session_workspace = backend
            .reserve_workspace(context, limits.maximum_native_session_bytes)
            .map_err(map_vp9_tensor_error)?;
        let mut output = self.ltxv_h264.binding.open_bounded_avio_output(
            limits.maximum_output_bytes,
            limits.avio_buffer_bytes,
            backend,
            context,
        )?;
        let functions = NativeLtxvH264EncodeFunctions::from_codec(&self.ltxv_h264);
        encode_rgb8_frame_with_check(
            NativeRgb8EncodeProfile::vp9_webm(frame_rate, crf),
            self.vpx_vp9_encoder,
            input,
            width,
            height,
            limits.maximum_packet_iterations,
            &functions,
            &mut output,
            &mut || context.cancellation.check(),
        )
        .map_err(map_vp9_session_error)?;
        context.check().map_err(map_vp9_tensor_error)?;
        if output.staged_bytes()?.is_empty() {
            return Err(NativeVideoCodecVp9EncodeError::EmptyOutput);
        }
        Ok(NativeVp9Webm {
            _suite: PhantomData,
            output,
            width,
            height,
            frame_rate,
            frame_count: 1,
            has_alpha: false,
        })
    }

    #[allow(
        dead_code,
        reason = "consumed by the following owned codec-thread byte bridge"
    )]
    pub(crate) fn encode_vp9_webm_batch<'suite>(
        &'suite self,
        images: &ImageTensor,
        frame_rate: (u64, u64),
        crf: NativeVideoCrf,
        limits: NativeVp9WebmBatchLimits,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeVp9Webm<'suite>, NativeVideoCodecVp9EncodeError> {
        self.encode_vp9_webm_batch_with_metadata(
            images,
            frame_rate,
            crf,
            limits,
            &NativeVideoContainerMetadata::empty(),
            backend,
            context,
        )
    }

    #[allow(
        dead_code,
        reason = "consumed by the retained codec-thread metadata bridge"
    )]
    pub(crate) fn encode_vp9_webm_batch_with_metadata<'suite>(
        &'suite self,
        images: &ImageTensor,
        frame_rate: (u64, u64),
        crf: NativeVideoCrf,
        limits: NativeVp9WebmBatchLimits,
        metadata: &NativeVideoContainerMetadata,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeVp9Webm<'suite>, NativeVideoCodecVp9EncodeError> {
        let (frame_count, width, height, channels) =
            validate_vp9_image_batch(images, limits, context)?;
        let frame_rate = checked_vp9_frame_rate(frame_rate)?;
        context.check().map_err(map_vp9_tensor_error)?;
        let _native_session_workspace = backend
            .reserve_workspace(context, limits.session.maximum_native_session_bytes)
            .map_err(map_vp9_tensor_error)?;
        let mut output = self.ltxv_h264.binding.open_bounded_avio_output(
            limits.session.maximum_output_bytes,
            limits.session.avio_buffer_bytes,
            backend,
            context,
        )?;
        let functions = NativeLtxvH264EncodeFunctions::from_codec(&self.ltxv_h264);
        let mut provide_frame = |frame_index: usize,
                                 consume: &mut dyn FnMut(
            &[u8],
        ) -> Result<
            (),
            NativeVideoCodecLtxvEncodeError,
        >| {
            if channels == 3 {
                let frame =
                    source_compatible_vp9_rgb8_frame(images, frame_index, backend, context)?;
                consume(frame.as_u8_slice().map_err(map_vp9_staging_tensor_error)?)
            } else {
                let frame =
                    source_compatible_vp9_rgba8_frame(images, frame_index, backend, context)?;
                consume(&frame)
            }
        };
        let profile = if channels == 4 {
            NativeRgb8EncodeProfile::vp9_webm_alpha(frame_rate, crf)
        } else {
            NativeRgb8EncodeProfile::vp9_webm(frame_rate, crf)
        };
        encode_rgb8_frames_with_metadata_check(
            profile,
            self.vpx_vp9_encoder,
            frame_count,
            width,
            height,
            limits.session.maximum_packet_iterations,
            &functions,
            &mut output,
            metadata,
            &mut provide_frame,
            &mut || context.cancellation.check(),
        )
        .map_err(map_vp9_session_error)?;
        context.check().map_err(map_vp9_tensor_error)?;
        if output.staged_bytes()?.is_empty() {
            return Err(NativeVideoCodecVp9EncodeError::EmptyOutput);
        }
        Ok(NativeVp9Webm {
            _suite: PhantomData,
            output,
            width,
            height,
            frame_rate,
            frame_count,
            has_alpha: channels == 4,
        })
    }
}

impl<'codec> NativeLtxvH264Mp4<'codec> {
    #[allow(
        dead_code,
        reason = "consumed by the following retained H.264 decode leaf"
    )]
    pub(crate) fn open_first_h264_video_stream<'bytes>(
        &'bytes self,
        limits: NativeLtxvH264DemuxLimits,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeLtxvH264Demux<'codec, 'bytes>, NativeVideoCodecLtxvDemuxError> {
        let bytes = self.output.staged_bytes()?;
        if bytes.is_empty() {
            return Err(NativeVideoCodecLtxvDemuxError::EmptyInput);
        }
        context.check().map_err(map_ltxv_demux_tensor_error)?;
        let native_session_workspace = backend
            .reserve_workspace(context, limits.maximum_native_session_bytes)
            .map_err(map_ltxv_demux_tensor_error)?;
        let input = self.codec.binding.open_bounded_avio_borrowed_input(
            bytes,
            limits.maximum_input_bytes,
            limits.avio_buffer_bytes,
            backend,
            context,
        )?;
        let functions = NativeLtxvH264DemuxFunctions::from_codec(self.codec);
        let (format, video_stream_index) = open_first_ltxv_h264_stream_with_check(
            &input,
            self.codec.decoder,
            limits.maximum_streams,
            &functions,
            &mut || context.cancellation.check(),
        )?;
        context.check().map_err(map_ltxv_demux_tensor_error)?;
        Ok(NativeLtxvH264Demux {
            format,
            input,
            _native_session_workspace: native_session_workspace,
            video_stream_index,
            expected_width: self.width,
            expected_height: self.height,
            codec: self.codec,
            _thread_bound: PhantomData,
        })
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following source-compatible LTXV tensor adapter"
)]
impl NativeLtxvH264Demux<'_, '_> {
    pub(crate) fn decode_first_rgb8_frame(
        self,
        limits: NativeLtxvH264DecodeLimits,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<Rgb8ImageTensor, NativeVideoCodecLtxvDecodeError> {
        context.check().map_err(map_ltxv_decode_tensor_error)?;
        let _native_decode_workspace = backend
            .reserve_workspace(context, limits.maximum_native_session_bytes)
            .map_err(map_ltxv_decode_tensor_error)?;
        let functions = NativeLtxvH264DecodeFunctions::from_codec(self.codec);
        let format = self.format.pointer().map_err(|_| {
            NativeVideoCodecLtxvDecodeError::NativeAllocation {
                phase: "access retained MP4 input",
            }
        })?;
        let rgb = decode_first_ltxv_h264_frame_with_check(
            format,
            self.video_stream_index,
            self.codec.decoder,
            self.expected_width,
            self.expected_height,
            limits,
            &functions,
            &self.input,
            backend,
            context,
            &mut || context.cancellation.check(),
        )?;
        context.check().map_err(map_ltxv_decode_tensor_error)?;
        Ok(rgb)
    }
}

fn preprocess_ltxv_image_with_round_trip(
    image: &ImageTensor,
    compression: u8,
    limits: NativeLtxvH264PreprocessLimits,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    round_trip: &mut impl FnMut(
        &Rgb8ImageTensor,
        &CpuBackend,
        &ExecutionContext<'_>,
    ) -> Result<Rgb8ImageTensor, NativeVideoCodecLtxvPreprocessError>,
) -> Result<ImageTensor, NativeVideoCodecLtxvPreprocessError> {
    context.check()?;
    if compression > 100 {
        return Err(NativeVideoCodecLtxvPreprocessError::InvalidCompression);
    }
    let (batch, input_height, input_width, channels) = image
        .dimensions()
        .map_err(map_ltxv_preprocess_tensor_error)?;
    if batch == 0 || batch > limits.maximum_batch || (compression != 0 && channels != 3) {
        return Err(NativeVideoCodecLtxvPreprocessError::InvalidInput);
    }
    let (output_height, output_width) = if compression == 0 {
        (input_height, input_width)
    } else {
        (input_height / 2 * 2, input_width / 2 * 2)
    };
    if compression != 0 && (output_height == 0 || output_width == 0) {
        return Err(NativeVideoCodecLtxvPreprocessError::InvalidInput);
    }
    let output_elements = batch
        .checked_mul(output_height)
        .and_then(|value| value.checked_mul(output_width))
        .and_then(|value| value.checked_mul(channels))
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value <= limits.maximum_output_elements)
        .ok_or(NativeVideoCodecLtxvPreprocessError::ResourceExhausted)?;
    let mut output = backend
        .workspace_vec::<f32>(context, output_elements)
        .map_err(map_ltxv_preprocess_tensor_error)?;

    if compression == 0 {
        let input = image
            .as_f32_slice()
            .map_err(map_ltxv_preprocess_tensor_error)?;
        if input.len() != output_elements {
            return Err(NativeVideoCodecLtxvPreprocessError::InvalidInput);
        }
        for (index, value) in input.iter().copied().enumerate() {
            if index & 0xffff == 0 {
                context.check()?;
            }
            output
                .try_push(value)
                .map_err(map_ltxv_preprocess_tensor_error)?;
        }
    } else {
        for batch_index in 0..batch {
            context.check()?;
            let frame = source_compatible_ltxv_rgb8_frame(
                image,
                batch_index,
                output_height,
                output_width,
                backend,
                context,
            )?;
            let decoded = round_trip(&frame, backend, context)?;
            if decoded
                .dimensions()
                .map_err(map_ltxv_preprocess_tensor_error)?
                != (output_height, output_width)
            {
                return Err(NativeVideoCodecLtxvPreprocessError::InvalidInput);
            }
            let decoded = decoded
                .as_u8_slice()
                .map_err(map_ltxv_preprocess_tensor_error)?;
            let expected_frame_elements = usize::try_from(
                output_height
                    .checked_mul(output_width)
                    .and_then(|value| value.checked_mul(3))
                    .ok_or(NativeVideoCodecLtxvPreprocessError::ResourceExhausted)?,
            )
            .map_err(|_| NativeVideoCodecLtxvPreprocessError::ResourceExhausted)?;
            if decoded.len() != expected_frame_elements {
                return Err(NativeVideoCodecLtxvPreprocessError::InvalidInput);
            }
            for (index, value) in decoded.iter().copied().enumerate() {
                if index & 0xffff == 0 {
                    context.check()?;
                }
                output
                    .try_push(f32::from(value) / 255.0)
                    .map_err(map_ltxv_preprocess_tensor_error)?;
            }
        }
    }

    context.check()?;
    let image = ImageTensor::from_f32(
        backend,
        context,
        batch,
        output_height,
        output_width,
        channels,
        &output,
    )
    .map_err(map_ltxv_preprocess_tensor_error)?;
    context.check()?;
    Ok(image)
}

fn source_compatible_ltxv_rgb8_frame(
    image: &ImageTensor,
    batch_index: u64,
    height: u64,
    width: u64,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<Rgb8ImageTensor, NativeVideoCodecLtxvPreprocessError> {
    let descriptor = image.tensor().descriptor();
    let [batch, input_height, input_width, channels] = descriptor.shape() else {
        return Err(NativeVideoCodecLtxvPreprocessError::InvalidInput);
    };
    let [batch_stride, height_stride, width_stride, channel_stride] = descriptor.strides() else {
        return Err(NativeVideoCodecLtxvPreprocessError::InvalidInput);
    };
    if batch_index >= *batch
        || *channels != 3
        || height == 0
        || width == 0
        || height > *input_height
        || width > *input_width
        || *batch_stride < 0
    {
        return Err(NativeVideoCodecLtxvPreprocessError::InvalidInput);
    }
    let batch_offset = u64::try_from(*batch_stride)
        .ok()
        .and_then(|stride| stride.checked_mul(batch_index))
        .and_then(|offset| descriptor.offset_elements().checked_add(offset))
        .ok_or(NativeVideoCodecLtxvPreprocessError::ResourceExhausted)?;
    let frame_descriptor = TensorDescriptor::new_strided(
        vec![3, height, width],
        vec![*channel_stride, *height_stride, *width_stride],
        batch_offset,
        DType::F32,
        Layout::Strided,
        DeviceId::CPU,
        descriptor.stream(),
    )
    .map_err(map_ltxv_preprocess_tensor_error)?;
    let frame = image
        .tensor()
        .view(frame_descriptor, ViewAccess::ReadOnly)
        .map_err(map_ltxv_preprocess_tensor_error)?;
    Rgb8ImageTensor::from_logical_chw(backend, context, &frame)
        .map_err(map_ltxv_preprocess_tensor_error)
}

fn map_ltxv_preprocess_tensor_error(error: TensorError) -> NativeVideoCodecLtxvPreprocessError {
    match error {
        TensorError::Cancelled => NativeVideoCodecLtxvPreprocessError::Cancelled,
        TensorError::AllocationFailed { .. }
        | TensorError::ResourceLimitExceeded { .. }
        | TensorError::WorkspaceAuthorizationExceeded { .. } => {
            NativeVideoCodecLtxvPreprocessError::ResourceExhausted
        }
        error => NativeVideoCodecLtxvPreprocessError::Tensor(error),
    }
}

fn map_ltxv_decode_tensor_error(error: TensorError) -> NativeVideoCodecLtxvDecodeError {
    match error {
        TensorError::Cancelled => NativeVideoCodecLtxvDecodeError::Cancelled,
        TensorError::AllocationFailed { .. }
        | TensorError::ResourceLimitExceeded { .. }
        | TensorError::WorkspaceAuthorizationExceeded { .. } => {
            NativeVideoCodecLtxvDecodeError::ResourceExhausted {
                phase: "authorize or allocate decode output",
            }
        }
        error => NativeVideoCodecLtxvDecodeError::Tensor(error),
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
fn map_ltxv_demux_tensor_error(error: TensorError) -> NativeVideoCodecLtxvDemuxError {
    match error {
        TensorError::Cancelled => NativeVideoCodecLtxvDemuxError::Cancelled,
        TensorError::AllocationFailed { .. }
        | TensorError::ResourceLimitExceeded { .. }
        | TensorError::WorkspaceAuthorizationExceeded { .. } => {
            NativeVideoCodecLtxvDemuxError::ResourceExhausted {
                phase: "authorize demux workspace",
            }
        }
        error => NativeVideoCodecLtxvDemuxError::Tensor(error),
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
fn validate_ltxv_h264_encode_input(
    frame: &Rgb8ImageTensor,
    crf: u8,
) -> Result<(&[u8], i32, i32), NativeVideoCodecLtxvEncodeError> {
    let (height, width) = frame.dimensions().map_err(map_ltxv_tensor_error)?;
    let width = i32::try_from(width)
        .ok()
        .filter(|width| *width > 0 && *width % 2 == 0)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let height = i32::try_from(height)
        .ok()
        .filter(|height| *height > 0 && *height % 2 == 0)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    if !(1..=100).contains(&crf) {
        return Err(NativeVideoCodecLtxvEncodeError::InvalidCrf);
    }
    let expected_bytes = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(usize::try_from(height).ok()?))
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let input = frame.as_u8_slice().map_err(map_ltxv_tensor_error)?;
    if input.len() != expected_bytes {
        return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
    }
    Ok((input, width, height))
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
fn validate_vp9_rgb8_encode_input(
    frame: &Rgb8ImageTensor,
) -> Result<(&[u8], i32, i32), NativeVideoCodecVp9EncodeError> {
    let (height, width) = frame.dimensions().map_err(map_vp9_tensor_error)?;
    let width = i32::try_from(width)
        .ok()
        .filter(|width| *width > 0)
        .ok_or(NativeVideoCodecVp9EncodeError::InvalidInput)?;
    let height = i32::try_from(height)
        .ok()
        .filter(|height| *height > 0)
        .ok_or(NativeVideoCodecVp9EncodeError::InvalidInput)?;
    let expected_bytes = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(usize::try_from(height).ok()?))
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or(NativeVideoCodecVp9EncodeError::InvalidInput)?;
    let input = frame.as_u8_slice().map_err(map_vp9_tensor_error)?;
    if input.len() != expected_bytes {
        return Err(NativeVideoCodecVp9EncodeError::InvalidInput);
    }
    Ok((input, width, height))
}

fn validate_vp9_image_batch(
    images: &ImageTensor,
    limits: NativeVp9WebmBatchLimits,
    context: &ExecutionContext<'_>,
) -> Result<(usize, i32, i32, i32), NativeVideoCodecVp9EncodeError> {
    let descriptor = images.tensor().descriptor();
    if descriptor.stream() != context.stream {
        return Err(NativeVideoCodecVp9EncodeError::InvalidBatch);
    }
    let [frame_count, height, width, channels] = descriptor.shape() else {
        return Err(NativeVideoCodecVp9EncodeError::InvalidBatch);
    };
    if !matches!(*channels, 3 | 4) {
        return Err(NativeVideoCodecVp9EncodeError::InvalidBatch);
    }
    let frame_count = usize::try_from(*frame_count)
        .ok()
        .filter(|count| {
            *count > 0 && *count <= limits.maximum_frames && i64::try_from(*count).is_ok()
        })
        .ok_or(NativeVideoCodecVp9EncodeError::InvalidBatch)?;
    let _pixels = height
        .checked_mul(*width)
        .filter(|pixels| *pixels > 0 && *pixels <= limits.maximum_pixels_per_frame)
        .ok_or(NativeVideoCodecVp9EncodeError::InvalidBatch)?;
    let width = i32::try_from(*width)
        .ok()
        .filter(|value| *value > 0)
        .ok_or(NativeVideoCodecVp9EncodeError::InvalidBatch)?;
    let height = i32::try_from(*height)
        .ok()
        .filter(|value| *value > 0)
        .ok_or(NativeVideoCodecVp9EncodeError::InvalidBatch)?;
    let channels = i32::try_from(*channels)
        .ok()
        .filter(|channels| matches!(*channels, 3 | 4))
        .ok_or(NativeVideoCodecVp9EncodeError::InvalidBatch)?;
    Ok((frame_count, width, height, channels))
}

fn source_compatible_vp9_rgb8_frame(
    images: &ImageTensor,
    frame_index: usize,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<Rgb8ImageTensor, NativeVideoCodecLtxvEncodeError> {
    context.check().map_err(map_vp9_staging_tensor_error)?;
    let [frame_count, height, width, 3] = images.tensor().descriptor().shape() else {
        return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
    };
    let frame_count =
        usize::try_from(*frame_count).map_err(|_| NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    if frame_index >= frame_count {
        return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
    }
    let frame_elements = height
        .checked_mul(*width)
        .and_then(|pixels| pixels.checked_mul(3))
        .and_then(|elements| usize::try_from(elements).ok())
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let start = frame_index
        .checked_mul(frame_elements)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let end = start
        .checked_add(frame_elements)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let input = images
        .as_f32_slice()
        .map_err(map_vp9_staging_tensor_error)?
        .get(start..end)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let mut bytes = backend
        .workspace_vec(context, frame_elements)
        .map_err(map_vp9_staging_tensor_error)?;
    for (index, value) in input.iter().copied().enumerate() {
        if index & 0xffff == 0 {
            context.check().map_err(map_vp9_staging_tensor_error)?;
        }
        bytes
            .try_push((value * 255.0).clamp(0.0, 255.0) as u8)
            .map_err(map_vp9_staging_tensor_error)?;
    }
    context.check().map_err(map_vp9_staging_tensor_error)?;
    let descriptor = TensorDescriptor::contiguous(
        vec![*height, *width, 3],
        DType::U8,
        DeviceId::CPU,
        context.stream,
    )
    .map_err(map_vp9_staging_tensor_error)?;
    let (tensor, _) = backend
        .upload_bytes(descriptor, &bytes, context)
        .map_err(map_vp9_staging_tensor_error)?;
    Rgb8ImageTensor::from_tensor(tensor).map_err(map_vp9_staging_tensor_error)
}

fn source_compatible_vp9_rgba8_frame(
    images: &ImageTensor,
    frame_index: usize,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<CpuWorkspaceVec<u8>, NativeVideoCodecLtxvEncodeError> {
    context.check().map_err(map_vp9_staging_tensor_error)?;
    let [frame_count, height, width, 4] = images.tensor().descriptor().shape() else {
        return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
    };
    let frame_count =
        usize::try_from(*frame_count).map_err(|_| NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    if frame_index >= frame_count {
        return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
    }
    let frame_elements = height
        .checked_mul(*width)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|elements| usize::try_from(elements).ok())
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let start = frame_index
        .checked_mul(frame_elements)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let end = start
        .checked_add(frame_elements)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let input = images
        .as_f32_slice()
        .map_err(map_vp9_staging_tensor_error)?
        .get(start..end)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let mut bytes = backend
        .workspace_vec(context, frame_elements)
        .map_err(map_vp9_staging_tensor_error)?;
    for (index, value) in input.iter().copied().enumerate() {
        if index & 0xffff == 0 {
            context.check().map_err(map_vp9_staging_tensor_error)?;
        }
        bytes
            .try_push((value * 255.0).clamp(0.0, 255.0) as u8)
            .map_err(map_vp9_staging_tensor_error)?;
    }
    context.check().map_err(map_vp9_staging_tensor_error)?;
    Ok(bytes)
}

fn map_vp9_staging_tensor_error(error: TensorError) -> NativeVideoCodecLtxvEncodeError {
    match error {
        TensorError::Cancelled => NativeVideoCodecLtxvEncodeError::Cancelled,
        TensorError::AllocationFailed { .. }
        | TensorError::ResourceLimitExceeded { .. }
        | TensorError::WorkspaceAuthorizationExceeded { .. } => {
            NativeVideoCodecLtxvEncodeError::ResourceExhausted {
                phase: "stage VP9 RGB frame",
            }
        }
        error => NativeVideoCodecLtxvEncodeError::Tensor(error),
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
fn checked_vp9_frame_rate(
    frame_rate: (u64, u64),
) -> Result<abi::AvRational, NativeVideoCodecVp9EncodeError> {
    let numerator = i32::try_from(frame_rate.0)
        .ok()
        .filter(|value| *value > 0)
        .ok_or(NativeVideoCodecVp9EncodeError::InvalidFrameRate)?;
    let denominator = i32::try_from(frame_rate.1)
        .ok()
        .filter(|value| *value > 0)
        .ok_or(NativeVideoCodecVp9EncodeError::InvalidFrameRate)?;
    if greatest_common_divisor(frame_rate.0, frame_rate.1) != 1 {
        return Err(NativeVideoCodecVp9EncodeError::InvalidFrameRate);
    }
    Ok(abi::AvRational {
        numerator,
        denominator,
    })
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
fn greatest_common_divisor(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
fn map_vp9_tensor_error(error: TensorError) -> NativeVideoCodecVp9EncodeError {
    match error {
        TensorError::Cancelled => NativeVideoCodecVp9EncodeError::Cancelled,
        TensorError::AllocationFailed { .. }
        | TensorError::ResourceLimitExceeded { .. }
        | TensorError::WorkspaceAuthorizationExceeded { .. } => {
            NativeVideoCodecVp9EncodeError::ResourceExhausted {
                phase: "authorize VP9 WebM workspace",
            }
        }
        error => NativeVideoCodecVp9EncodeError::Tensor(error),
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
fn map_vp9_session_error(error: NativeVideoCodecLtxvEncodeError) -> NativeVideoCodecVp9EncodeError {
    match error {
        NativeVideoCodecLtxvEncodeError::Cancelled => NativeVideoCodecVp9EncodeError::Cancelled,
        NativeVideoCodecLtxvEncodeError::InvalidInput => {
            NativeVideoCodecVp9EncodeError::InvalidInput
        }
        NativeVideoCodecLtxvEncodeError::InvalidLimits => {
            NativeVideoCodecVp9EncodeError::InvalidLimits
        }
        NativeVideoCodecLtxvEncodeError::InvalidCrf => NativeVideoCodecVp9EncodeError::InvalidCrf,
        NativeVideoCodecLtxvEncodeError::NativeAllocation { phase } => {
            NativeVideoCodecVp9EncodeError::NativeAllocation { phase }
        }
        NativeVideoCodecLtxvEncodeError::ResourceExhausted { phase } => {
            NativeVideoCodecVp9EncodeError::ResourceExhausted { phase }
        }
        NativeVideoCodecLtxvEncodeError::NativeCall { phase, status } => {
            NativeVideoCodecVp9EncodeError::NativeCall { phase, status }
        }
        NativeVideoCodecLtxvEncodeError::UnconsumedCodecOptions => {
            NativeVideoCodecVp9EncodeError::UnconsumedCodecOptions
        }
        NativeVideoCodecLtxvEncodeError::PacketIterationLimit => {
            NativeVideoCodecVp9EncodeError::PacketIterationLimit
        }
        NativeVideoCodecLtxvEncodeError::EmptyOutput => NativeVideoCodecVp9EncodeError::EmptyOutput,
        NativeVideoCodecLtxvEncodeError::Io(error) => NativeVideoCodecVp9EncodeError::Io(error),
        NativeVideoCodecLtxvEncodeError::Tensor(error) => map_vp9_tensor_error(error),
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
#[derive(Debug, Error)]
pub(crate) enum NativeVideoCodecLtxvEncodeError {
    #[error("native LTXV H.264 encoding was cancelled")]
    Cancelled,
    #[error("native LTXV H.264 encoding received an invalid RGB8 frame")]
    InvalidInput,
    #[error("native LTXV H.264 encoding received invalid resource limits")]
    InvalidLimits,
    #[error("native LTXV H.264 encoding requires CRF in the source range 1 through 100")]
    InvalidCrf,
    #[error("native LTXV H.264 allocation failed during {phase}")]
    NativeAllocation { phase: &'static str },
    #[error("native LTXV H.264 resources were exhausted during {phase}")]
    ResourceExhausted { phase: &'static str },
    #[error("native LTXV H.264 operation {phase} failed with status {status}")]
    NativeCall { phase: &'static str, status: i32 },
    #[error("native LTXV H.264 codec options were not fully consumed")]
    UnconsumedCodecOptions,
    #[error("native LTXV H.264 packet draining exceeded its checked iteration limit")]
    PacketIterationLimit,
    #[error("native LTXV H.264 encoding produced no MP4 bytes")]
    EmptyOutput,
    #[error(transparent)]
    Io(#[from] NativeVideoCodecIoError),
    #[error("native LTXV H.264 tensor operation failed: {0}")]
    Tensor(#[source] TensorError),
}

impl From<CancellationError> for NativeVideoCodecLtxvEncodeError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
fn map_ltxv_tensor_error(error: TensorError) -> NativeVideoCodecLtxvEncodeError {
    match error {
        TensorError::Cancelled => NativeVideoCodecLtxvEncodeError::Cancelled,
        error => NativeVideoCodecLtxvEncodeError::Tensor(error),
    }
}

#[derive(Debug, Error)]
pub enum NativeVideoCodecLtxvAdmissionError {
    #[error("native LTXV H.264 codec admission was cancelled")]
    Cancelled,
    #[error("native LTXV H.264 codec admission is unsupported for this target")]
    UnsupportedTarget,
    #[error("the retained LTXV libx264 dependency contract is incomplete")]
    InvalidDependencyContract,
    #[error("the retained FFmpeg registry has no libx264 encoder")]
    MissingLibx264Encoder,
    #[error("the retained FFmpeg registry has no H.264 decoder")]
    MissingH264Decoder,
    #[error("the libx264 encoder descriptor came from the wrong loaded image")]
    EncoderProviderMismatch,
    #[error("the H.264 decoder descriptor came from the wrong loaded image")]
    DecoderProviderMismatch,
}

impl From<CancellationError> for NativeVideoCodecLtxvAdmissionError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
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

    pub fn admit_ltxv_h264(
        self,
        cancellation: &CancellationToken,
    ) -> Result<NativeLtxvH264Codec, NativeVideoCodecLtxvAdmissionError> {
        if !cfg!(all(
            target_os = "linux",
            target_arch = "x86_64",
            target_env = "gnu"
        )) {
            return Err(NativeVideoCodecLtxvAdmissionError::UnsupportedTarget);
        }
        if !has_exact_ltxv_h264_dependency_contract(&self) {
            return Err(NativeVideoCodecLtxvAdmissionError::InvalidDependencyContract);
        }
        let (encoder, decoder) =
            admit_ltxv_h264_with_check(&self._symbols, &self.load.loaded, || cancellation.check())?;
        cancellation.check()?;
        Ok(NativeLtxvH264Codec {
            binding: self,
            encoder,
            decoder,
        })
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
    ) -> Result<NativeVideoCodecMemoryInput<'binding, 'static>, NativeVideoCodecIoError> {
        let state = NativeVideoCodecInputState {
            bytes: NonNull::new(bytes.as_ptr().cast_mut())
                .ok_or(NativeVideoCodecIoError::InvalidBounds)?,
            byte_length: bytes.len(),
            position: 0,
            maximum_position: maximum_input_bytes,
            cancellation: context.cancellation.clone(),
            failure: None,
            #[cfg(test)]
            panic_on_next_callback: false,
        };
        if state.byte_length > maximum_input_bytes {
            return Err(NativeVideoCodecIoError::InvalidBounds);
        }
        let owner = NativeVideoCodecInputOwner::Owned(bytes);
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
        Ok(NativeVideoCodecMemoryInput {
            avio,
            _owner: owner,
        })
    }

    #[allow(
        dead_code,
        reason = "consumed by the following retained H.264 decode leaf"
    )]
    pub(crate) fn open_bounded_avio_borrowed_input<'binding, 'bytes>(
        &'binding self,
        bytes: &'bytes [u8],
        maximum_input_bytes: usize,
        buffer_bytes: usize,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeVideoCodecMemoryInput<'binding, 'bytes>, NativeVideoCodecIoError> {
        let state = NativeVideoCodecInputState {
            bytes: NonNull::new(bytes.as_ptr().cast_mut())
                .ok_or(NativeVideoCodecIoError::InvalidBounds)?,
            byte_length: bytes.len(),
            position: 0,
            maximum_position: maximum_input_bytes,
            cancellation: context.cancellation.clone(),
            failure: None,
            #[cfg(test)]
            panic_on_next_callback: false,
        };
        if state.byte_length > maximum_input_bytes {
            return Err(NativeVideoCodecIoError::InvalidBounds);
        }
        let owner = NativeVideoCodecInputOwner::Borrowed(bytes);
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
        Ok(NativeVideoCodecMemoryInput {
            avio,
            _owner: owner,
        })
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
enum NativeVideoCodecInputOwner<'bytes> {
    Owned(Arc<[u8]>),
    Borrowed(&'bytes [u8]),
}

struct NativeVideoCodecInputState {
    bytes: NonNull<u8>,
    byte_length: usize,
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
pub(crate) struct NativeVideoCodecMemoryInput<'binding, 'bytes> {
    avio: NativeVideoCodecAvio<'binding, NativeVideoCodecInputState>,
    _owner: NativeVideoCodecInputOwner<'bytes>,
}

#[allow(
    dead_code,
    reason = "consumed by the following bounded codec operation"
)]
pub(crate) struct NativeVideoCodecMemoryOutput<'binding> {
    avio: NativeVideoCodecAvio<'binding, NativeVideoCodecOutputState>,
}

impl NativeVideoCodecMemoryInput<'_, '_> {
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
        if state.position >= state.byte_length {
            return Ok(abi::AV_ERROR_END_OF_FILE);
        }
        let requested = usize::try_from(requested)
            .map_err(|_| NativeVideoCodecIoCallbackFailure::InvalidArgument)?;
        let count = requested.min(state.byte_length - state.position);
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
        state.byte_length,
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

#[derive(Clone, Copy)]
struct NativeLtxvH264DemuxFunctions {
    av_find_best_stream: abi::AvFindBestStream,
    avformat_alloc_context: abi::AvformatAllocContext,
    avformat_close_input: abi::AvformatCloseInput,
    avformat_free_context: abi::AvformatFreeContext,
    avformat_open_input: abi::AvformatOpenInput,
}

impl NativeLtxvH264DemuxFunctions {
    #[allow(
        dead_code,
        reason = "consumed by the following retained H.264 decode leaf"
    )]
    fn from_codec(codec: &NativeLtxvH264Codec) -> Self {
        let symbols = &codec.binding._symbols.avformat;
        Self {
            av_find_best_stream: symbols.av_find_best_stream,
            avformat_alloc_context: symbols.avformat_alloc_context,
            avformat_close_input: symbols.avformat_close_input,
            avformat_free_context: symbols.avformat_free_context,
            avformat_open_input: symbols.avformat_open_input,
        }
    }
}

enum NativeLtxvInputFormatCleanup {
    Allocated(abi::AvformatFreeContext),
    Opened(abi::AvformatCloseInput),
}

struct NativeLtxvInputFormatContext {
    pointer: Option<NonNull<abi::AvFormatContext>>,
    cleanup: NativeLtxvInputFormatCleanup,
}

impl NativeLtxvInputFormatContext {
    fn pointer(&self) -> Result<NonNull<abi::AvFormatContext>, NativeVideoCodecLtxvDemuxError> {
        self.pointer
            .ok_or(NativeVideoCodecLtxvDemuxError::NativeAllocation)
    }
}

impl Drop for NativeLtxvInputFormatContext {
    fn drop(&mut self) {
        let Some(pointer) = self.pointer.take() else {
            return;
        };
        match self.cleanup {
            NativeLtxvInputFormatCleanup::Allocated(free) => unsafe { free(pointer.as_ptr()) },
            NativeLtxvInputFormatCleanup::Opened(close) => {
                let mut pointer = pointer.as_ptr();
                unsafe { close(std::ptr::addr_of_mut!(pointer)) };
                if !pointer.is_null() {
                    eprintln!("native LTXV MP4 input cleanup did not clear its format pointer");
                }
            }
        }
    }
}

fn open_first_ltxv_h264_stream_with_check<'binding, 'bytes>(
    input: &NativeVideoCodecMemoryInput<'binding, 'bytes>,
    expected_decoder: NonNull<abi::AvCodec>,
    maximum_streams: usize,
    functions: &NativeLtxvH264DemuxFunctions,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(NativeLtxvInputFormatContext, i32), NativeVideoCodecLtxvDemuxError> {
    check_cancellation()?;
    let format_pointer = unsafe { (functions.avformat_alloc_context)() };
    let mut format = NativeLtxvInputFormatContext {
        pointer: Some(
            NonNull::new(format_pointer).ok_or(NativeVideoCodecLtxvDemuxError::NativeAllocation)?,
        ),
        cleanup: NativeLtxvInputFormatCleanup::Allocated(functions.avformat_free_context),
    };
    check_cancellation()?;
    let mut format_pointer = format.pointer()?;
    unsafe { format_pointer.as_mut().io_context = input.context_ptr() };
    let mut raw_format = format_pointer.as_ptr();
    check_cancellation()?;
    let open_status = unsafe {
        (functions.avformat_open_input)(
            std::ptr::addr_of_mut!(raw_format),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    format.pointer = NonNull::new(raw_format);
    if open_status >= 0 && format.pointer.is_some() {
        format.cleanup = NativeLtxvInputFormatCleanup::Opened(functions.avformat_close_input);
    }
    let post_open_cancellation = check_cancellation();
    let callback_status = input.check_callback_status();
    if format.pointer.is_none() {
        if let Err(error) = post_open_cancellation {
            return Err(error.into());
        }
        callback_status?;
        return Err(map_ltxv_demux_open_status(open_status));
    }
    post_open_cancellation?;
    callback_status?;
    if open_status < 0 {
        return Err(map_ltxv_demux_open_status(open_status));
    }

    let format_pointer = format.pointer()?;
    if unsafe { format_pointer.as_ref().io_context } != input.context_ptr() {
        return Err(NativeVideoCodecLtxvDemuxError::InputContextMismatch);
    }
    let stream_count = usize::try_from(unsafe { format_pointer.as_ref().stream_count })
        .map_err(|_| NativeVideoCodecLtxvDemuxError::StreamLimitExceeded)?;
    if stream_count > maximum_streams {
        return Err(NativeVideoCodecLtxvDemuxError::StreamLimitExceeded);
    }
    if stream_count != 1 {
        return Err(NativeVideoCodecLtxvDemuxError::MissingVideoStream);
    }
    let streams = unsafe { format_pointer.as_ref().streams };
    if streams.is_null() {
        return Err(NativeVideoCodecLtxvDemuxError::MissingVideoStream);
    }
    let Some(stream) = NonNull::new(unsafe { *streams }) else {
        return Err(NativeVideoCodecLtxvDemuxError::MissingVideoStream);
    };
    if unsafe { stream.as_ref().codec_parameters }.is_null() {
        return Err(NativeVideoCodecLtxvDemuxError::MissingVideoStream);
    }
    let stream_index = unsafe { stream.as_ref().index };
    if stream_index < 0 {
        return Err(NativeVideoCodecLtxvDemuxError::MissingVideoStream);
    }
    check_cancellation()?;
    let mut decoder = std::ptr::null();
    let selected_stream = unsafe {
        (functions.av_find_best_stream)(
            format_pointer.as_ptr(),
            abi::AV_MEDIA_TYPE_VIDEO,
            -1,
            -1,
            std::ptr::addr_of_mut!(decoder),
            0,
        )
    };
    check_cancellation()?;
    if selected_stream != stream_index || decoder != expected_decoder.as_ptr() {
        return Err(NativeVideoCodecLtxvDemuxError::UnexpectedVideoCodec);
    }
    Ok((format, stream_index))
}

fn map_ltxv_demux_open_status(status: i32) -> NativeVideoCodecLtxvDemuxError {
    if status == abi::AV_ERROR_OUT_OF_MEMORY {
        NativeVideoCodecLtxvDemuxError::ResourceExhausted {
            phase: "open MP4 input",
        }
    } else {
        NativeVideoCodecLtxvDemuxError::OpenFailed { status }
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
#[derive(Clone, Copy)]
struct NativeLtxvH264EncodeFunctions {
    av_packet_alloc: abi::AvPacketAlloc,
    av_packet_free: abi::AvPacketFree,
    av_packet_unref: abi::AvPacketUnref,
    avcodec_alloc_context3: abi::AvcodecAllocContext3,
    avcodec_free_context: abi::AvcodecFreeContext,
    avcodec_open2: abi::AvcodecOpen2,
    avcodec_parameters_from_context: abi::AvcodecParametersFromContext,
    avcodec_receive_packet: abi::AvcodecReceivePacket,
    avcodec_send_frame: abi::AvcodecSendFrame,
    avformat_alloc_output_context2: abi::AvformatAllocOutputContext2,
    avformat_free_context: abi::AvformatFreeContext,
    avformat_new_stream: abi::AvformatNewStream,
    avformat_write_header: abi::AvformatWriteHeader,
    av_interleaved_write_frame: abi::AvInterleavedWriteFrame,
    av_write_trailer: abi::AvWriteTrailer,
    av_dict_free: abi::AvDictFree,
    av_dict_set: abi::AvDictSet,
    av_frame_alloc: abi::AvFrameAlloc,
    av_frame_free: abi::AvFrameFree,
    av_frame_get_buffer: abi::AvFrameGetBuffer,
    av_frame_make_writable: abi::AvFrameMakeWritable,
    av_opt_set: abi::AvOptSet,
    av_opt_set_int: abi::AvOptSetInt,
    av_rescale_q: abi::AvRescaleQ,
    sws_free_context: abi::SwsFreeContext,
    sws_get_context: abi::SwsGetContext,
    sws_scale: abi::SwsScale,
}

#[derive(Clone, Copy)]
struct NativeLtxvH264DecodeFunctions {
    av_packet_alloc: abi::AvPacketAlloc,
    av_packet_free: abi::AvPacketFree,
    av_packet_unref: abi::AvPacketUnref,
    avcodec_alloc_context3: abi::AvcodecAllocContext3,
    avcodec_free_context: abi::AvcodecFreeContext,
    avcodec_open2: abi::AvcodecOpen2,
    avcodec_parameters_to_context: abi::AvcodecParametersToContext,
    avcodec_receive_frame: abi::AvcodecReceiveFrame,
    avcodec_send_packet: abi::AvcodecSendPacket,
    av_read_frame: abi::AvReadFrame,
    av_frame_alloc: abi::AvFrameAlloc,
    av_frame_free: abi::AvFrameFree,
    av_opt_set_int: abi::AvOptSetInt,
    sws_free_context: abi::SwsFreeContext,
    sws_get_context: abi::SwsGetContext,
    sws_scale: abi::SwsScale,
}

impl NativeLtxvH264DecodeFunctions {
    #[allow(
        dead_code,
        reason = "consumed by the following source-compatible LTXV tensor adapter"
    )]
    fn from_codec(codec: &NativeLtxvH264Codec) -> Self {
        let symbols = &codec.binding._symbols;
        Self {
            av_packet_alloc: symbols.avcodec.av_packet_alloc,
            av_packet_free: symbols.avcodec.av_packet_free,
            av_packet_unref: symbols.avcodec.av_packet_unref,
            avcodec_alloc_context3: symbols.avcodec.avcodec_alloc_context3,
            avcodec_free_context: symbols.avcodec.avcodec_free_context,
            avcodec_open2: symbols.avcodec.avcodec_open2,
            avcodec_parameters_to_context: symbols.avcodec.avcodec_parameters_to_context,
            avcodec_receive_frame: symbols.avcodec.avcodec_receive_frame,
            avcodec_send_packet: symbols.avcodec.avcodec_send_packet,
            av_read_frame: symbols.avformat.av_read_frame,
            av_frame_alloc: symbols.avutil.av_frame_alloc,
            av_frame_free: symbols.avutil.av_frame_free,
            av_opt_set_int: symbols.avutil.av_opt_set_int,
            sws_free_context: symbols.swscale.sws_free_context,
            sws_get_context: symbols.swscale.sws_get_context,
            sws_scale: symbols.swscale.sws_scale,
        }
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
impl NativeLtxvH264EncodeFunctions {
    fn from_codec(codec: &NativeLtxvH264Codec) -> Self {
        let symbols = &codec.binding._symbols;
        Self {
            av_packet_alloc: symbols.avcodec.av_packet_alloc,
            av_packet_free: symbols.avcodec.av_packet_free,
            av_packet_unref: symbols.avcodec.av_packet_unref,
            avcodec_alloc_context3: symbols.avcodec.avcodec_alloc_context3,
            avcodec_free_context: symbols.avcodec.avcodec_free_context,
            avcodec_open2: symbols.avcodec.avcodec_open2,
            avcodec_parameters_from_context: symbols.avcodec.avcodec_parameters_from_context,
            avcodec_receive_packet: symbols.avcodec.avcodec_receive_packet,
            avcodec_send_frame: symbols.avcodec.avcodec_send_frame,
            avformat_alloc_output_context2: symbols.avformat.avformat_alloc_output_context2,
            avformat_free_context: symbols.avformat.avformat_free_context,
            avformat_new_stream: symbols.avformat.avformat_new_stream,
            avformat_write_header: symbols.avformat.avformat_write_header,
            av_interleaved_write_frame: symbols.avformat.av_interleaved_write_frame,
            av_write_trailer: symbols.avformat.av_write_trailer,
            av_dict_free: symbols.avutil.av_dict_free,
            av_dict_set: symbols.avutil.av_dict_set,
            av_frame_alloc: symbols.avutil.av_frame_alloc,
            av_frame_free: symbols.avutil.av_frame_free,
            av_frame_get_buffer: symbols.avutil.av_frame_get_buffer,
            av_frame_make_writable: symbols.avutil.av_frame_make_writable,
            av_opt_set: symbols.avutil.av_opt_set,
            av_opt_set_int: symbols.avutil.av_opt_set_int,
            av_rescale_q: symbols.avutil.av_rescale_q,
            sws_free_context: symbols.swscale.sws_free_context,
            sws_get_context: symbols.swscale.sws_get_context,
            sws_scale: symbols.swscale.sws_scale,
        }
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
struct NativeLtxvFormatContext {
    pointer: NonNull<abi::AvFormatContext>,
    free: abi::AvformatFreeContext,
}

impl Drop for NativeLtxvFormatContext {
    fn drop(&mut self) {
        unsafe {
            self.pointer.as_mut().io_context = std::ptr::null_mut();
            (self.free)(self.pointer.as_ptr());
        }
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
struct NativeLtxvCodecContext {
    pointer: NonNull<abi::AvCodecContext>,
    free: abi::AvcodecFreeContext,
}

impl Drop for NativeLtxvCodecContext {
    fn drop(&mut self) {
        let mut pointer = self.pointer.as_ptr();
        unsafe { (self.free)(std::ptr::addr_of_mut!(pointer)) };
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
struct NativeLtxvFrame {
    pointer: NonNull<abi::AvFrame>,
    free: abi::AvFrameFree,
}

impl Drop for NativeLtxvFrame {
    fn drop(&mut self) {
        let mut pointer = self.pointer.as_ptr();
        unsafe { (self.free)(std::ptr::addr_of_mut!(pointer)) };
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
struct NativeLtxvPacket {
    pointer: NonNull<abi::AvPacket>,
    free: abi::AvPacketFree,
}

impl Drop for NativeLtxvPacket {
    fn drop(&mut self) {
        let mut pointer = self.pointer.as_ptr();
        unsafe { (self.free)(std::ptr::addr_of_mut!(pointer)) };
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
struct NativeLtxvSwsContext {
    pointer: NonNull<abi::SwsContext>,
    free: abi::SwsFreeContext,
}

impl Drop for NativeLtxvSwsContext {
    fn drop(&mut self) {
        unsafe { (self.free)(self.pointer.as_ptr()) };
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
struct NativeLtxvDictionary {
    pointer: *mut abi::AvDictionary,
    free: abi::AvDictFree,
}

impl Drop for NativeLtxvDictionary {
    fn drop(&mut self) {
        unsafe { (self.free)(std::ptr::addr_of_mut!(self.pointer)) };
    }
}

struct NativeLtxvPacketContent {
    packet: NonNull<abi::AvPacket>,
    unref: abi::AvPacketUnref,
    active: bool,
}

impl NativeLtxvPacketContent {
    fn new(packet: NonNull<abi::AvPacket>, unref: abi::AvPacketUnref) -> Self {
        Self {
            packet,
            unref,
            active: true,
        }
    }

    fn clear(&mut self) {
        if self.active {
            unsafe { (self.unref)(self.packet.as_ptr()) };
            self.active = false;
        }
    }
}

impl Drop for NativeLtxvPacketContent {
    fn drop(&mut self) {
        self.clear();
    }
}

#[allow(clippy::too_many_arguments)]
fn decode_first_ltxv_h264_frame_with_check(
    format: NonNull<abi::AvFormatContext>,
    video_stream_index: i32,
    decoder: NonNull<abi::AvCodec>,
    expected_width: i32,
    expected_height: i32,
    limits: NativeLtxvH264DecodeLimits,
    functions: &NativeLtxvH264DecodeFunctions,
    input: &NativeVideoCodecMemoryInput<'_, '_>,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<Rgb8ImageTensor, NativeVideoCodecLtxvDecodeError> {
    macro_rules! checked_native_call {
        ($expression:expr) => {{
            check_cancellation()?;
            let result = unsafe { $expression };
            check_cancellation()?;
            result
        }};
    }

    check_cancellation()?;
    let stream_count = unsafe { format.as_ref().stream_count };
    let stream_index = usize::try_from(video_stream_index)
        .ok()
        .filter(|index| *index < usize::try_from(stream_count).unwrap_or(0))
        .ok_or(NativeVideoCodecLtxvDecodeError::InvalidFrame)?;
    let streams = unsafe { format.as_ref().streams };
    if streams.is_null() {
        return Err(NativeVideoCodecLtxvDecodeError::InvalidFrame);
    }
    let stream_pointer = unsafe { *streams.add(stream_index) };
    let stream =
        NonNull::new(stream_pointer).ok_or(NativeVideoCodecLtxvDecodeError::InvalidFrame)?;
    let codec_parameters = unsafe { stream.as_ref().codec_parameters };
    if codec_parameters.is_null() {
        return Err(NativeVideoCodecLtxvDecodeError::InvalidFrame);
    }

    let codec_pointer = checked_native_call!((functions.avcodec_alloc_context3)(decoder.as_ptr()));
    let codec = NativeLtxvCodecContext {
        pointer: NonNull::new(codec_pointer).ok_or(
            NativeVideoCodecLtxvDecodeError::NativeAllocation {
                phase: "allocate H.264 decoder context",
            },
        )?,
        free: functions.avcodec_free_context,
    };
    check_ltxv_decode_status(
        "copy H.264 decoder parameters",
        checked_native_call!((functions.avcodec_parameters_to_context)(
            codec.pointer.as_ptr(),
            codec_parameters,
        )),
    )?;
    check_ltxv_decode_status(
        "set H.264 decoder threads",
        checked_native_call!((functions.av_opt_set_int)(
            codec.pointer.as_ptr().cast(),
            c"threads".as_ptr(),
            1,
            0,
        )),
    )?;
    check_ltxv_decode_status(
        "open H.264 decoder",
        checked_native_call!((functions.avcodec_open2)(
            codec.pointer.as_ptr(),
            decoder.as_ptr(),
            std::ptr::null_mut(),
        )),
    )?;

    let packet_pointer = checked_native_call!((functions.av_packet_alloc)());
    let packet = NativeLtxvPacket {
        pointer: NonNull::new(packet_pointer).ok_or(
            NativeVideoCodecLtxvDecodeError::NativeAllocation {
                phase: "allocate H.264 input packet",
            },
        )?,
        free: functions.av_packet_free,
    };
    let frame_pointer = checked_native_call!((functions.av_frame_alloc)());
    let frame = NativeLtxvFrame {
        pointer: NonNull::new(frame_pointer).ok_or(
            NativeVideoCodecLtxvDecodeError::NativeAllocation {
                phase: "allocate H.264 output frame",
            },
        )?,
        free: functions.av_frame_free,
    };

    let mut packet_iterations = limits.maximum_packet_iterations;
    let mut receive_iterations = limits.maximum_receive_iterations;
    let mut flushing = false;
    'decode: loop {
        consume_decode_iteration(&mut receive_iterations, false)?;
        let receive_status = checked_native_call!((functions.avcodec_receive_frame)(
            codec.pointer.as_ptr(),
            frame.pointer.as_ptr(),
        ));
        match receive_status {
            0 => break 'decode,
            abi::AV_ERROR_END_OF_FILE => {
                return Err(NativeVideoCodecLtxvDecodeError::MissingFrame);
            }
            abi::AV_ERROR_TRY_AGAIN if flushing => {
                return Err(NativeVideoCodecLtxvDecodeError::ProtocolStalled);
            }
            abi::AV_ERROR_TRY_AGAIN => {}
            status => check_ltxv_decode_status("receive H.264 frame", status)?,
        }

        consume_decode_iteration(&mut packet_iterations, true)?;
        check_cancellation()?;
        let read_status =
            unsafe { (functions.av_read_frame)(format.as_ptr(), packet.pointer.as_ptr()) };
        let mut packet_content = (read_status == 0)
            .then(|| NativeLtxvPacketContent::new(packet.pointer, functions.av_packet_unref));
        let post_read_cancellation = check_cancellation();
        let callback_status = input.check_callback_status();
        post_read_cancellation?;
        callback_status.map_err(map_ltxv_decode_io_error)?;
        match read_status {
            0 => {
                if unsafe { packet.pointer.as_ref().stream_index } != video_stream_index {
                    continue;
                }
                consume_decode_iteration(&mut receive_iterations, false)?;
                check_cancellation()?;
                let send_status = unsafe {
                    (functions.avcodec_send_packet)(codec.pointer.as_ptr(), packet.pointer.as_ptr())
                };
                let post_send_cancellation = check_cancellation();
                if send_status == 0 {
                    if let Some(content) = packet_content.as_mut() {
                        content.clear();
                    }
                    post_send_cancellation?;
                    continue;
                }
                post_send_cancellation?;
                if send_status != abi::AV_ERROR_TRY_AGAIN {
                    check_ltxv_decode_status("send H.264 packet", send_status)?;
                }
                consume_decode_iteration(&mut receive_iterations, false)?;
                let pending_receive_status = checked_native_call!((functions
                    .avcodec_receive_frame)(
                    codec.pointer.as_ptr(),
                    frame.pointer.as_ptr(),
                ));
                match pending_receive_status {
                    0 => break 'decode,
                    abi::AV_ERROR_END_OF_FILE => {
                        return Err(NativeVideoCodecLtxvDecodeError::MissingFrame);
                    }
                    abi::AV_ERROR_TRY_AGAIN => {
                        return Err(NativeVideoCodecLtxvDecodeError::ProtocolStalled);
                    }
                    status => check_ltxv_decode_status("receive pending H.264 frame", status)?,
                }
            }
            abi::AV_ERROR_TRY_AGAIN => continue,
            abi::AV_ERROR_END_OF_FILE => {
                flushing = true;
                consume_decode_iteration(&mut receive_iterations, false)?;
                let flush_status = checked_native_call!((functions.avcodec_send_packet)(
                    codec.pointer.as_ptr(),
                    std::ptr::null(),
                ));
                match flush_status {
                    0 => continue,
                    abi::AV_ERROR_END_OF_FILE => {
                        return Err(NativeVideoCodecLtxvDecodeError::MissingFrame);
                    }
                    abi::AV_ERROR_TRY_AGAIN => {
                        return Err(NativeVideoCodecLtxvDecodeError::ProtocolStalled);
                    }
                    status => check_ltxv_decode_status("flush H.264 decoder", status)?,
                }
            }
            status => check_ltxv_decode_status("read H.264 packet", status)?,
        }
    }

    validate_ltxv_decoded_frame(frame.pointer, expected_width, expected_height, limits)?;
    let width = unsafe { frame.pointer.as_ref().width };
    let height = unsafe { frame.pointer.as_ref().height };
    let output_bytes = decoded_rgb_byte_count(width, height, limits)?;
    let mut rgb = backend
        .workspace_vec::<u8>(context, output_bytes)
        .map_err(map_ltxv_decode_tensor_error)?;
    for index in 0..output_bytes {
        if index & 0xffff == 0 {
            check_cancellation()?;
        }
        rgb.try_push(0).map_err(map_ltxv_decode_tensor_error)?;
    }
    let sws_pointer = checked_native_call!((functions.sws_get_context)(
        width,
        height,
        abi::AV_PIXEL_FORMAT_YUV420P,
        width,
        height,
        abi::AV_PIXEL_FORMAT_RGB24,
        abi::SWS_BILINEAR,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null(),
    ));
    let sws = NativeLtxvSwsContext {
        pointer: NonNull::new(sws_pointer).ok_or(
            NativeVideoCodecLtxvDecodeError::NativeAllocation {
                phase: "allocate decoded RGB conversion",
            },
        )?,
        free: functions.sws_free_context,
    };
    let mut destination_data = [std::ptr::null_mut(); abi::AV_NUM_DATA_POINTERS];
    destination_data[0] = rgb.as_mut_ptr();
    let mut destination_line_size = [0_i32; abi::AV_NUM_DATA_POINTERS];
    destination_line_size[0] = width
        .checked_mul(3)
        .ok_or(NativeVideoCodecLtxvDecodeError::InvalidFrame)?;
    let converted_height = checked_native_call!((functions.sws_scale)(
        sws.pointer.as_ptr(),
        frame.pointer.as_ref().data.as_ptr().cast(),
        frame.pointer.as_ref().line_size.as_ptr(),
        0,
        height,
        destination_data.as_mut_ptr(),
        destination_line_size.as_ptr(),
    ));
    if converted_height != height {
        return Err(NativeVideoCodecLtxvDecodeError::NativeCall {
            phase: "convert decoded H.264 frame to RGB",
            status: converted_height,
        });
    }
    check_cancellation()?;
    let descriptor = TensorDescriptor::contiguous(
        vec![
            u64::try_from(height).map_err(|_| NativeVideoCodecLtxvDecodeError::InvalidFrame)?,
            u64::try_from(width).map_err(|_| NativeVideoCodecLtxvDecodeError::InvalidFrame)?,
            3,
        ],
        DType::U8,
        DeviceId::CPU,
        context.stream,
    )
    .map_err(map_ltxv_decode_tensor_error)?;
    let (tensor, _) = backend
        .upload_bytes(descriptor, &rgb, context)
        .map_err(map_ltxv_decode_tensor_error)?;
    let image = Rgb8ImageTensor::from_tensor(tensor).map_err(map_ltxv_decode_tensor_error)?;
    check_cancellation()?;
    Ok(image)
}

fn consume_decode_iteration(
    remaining: &mut usize,
    packet: bool,
) -> Result<(), NativeVideoCodecLtxvDecodeError> {
    if *remaining == 0 {
        return if packet {
            Err(NativeVideoCodecLtxvDecodeError::PacketIterationLimit)
        } else {
            Err(NativeVideoCodecLtxvDecodeError::ReceiveIterationLimit)
        };
    }
    *remaining -= 1;
    Ok(())
}

fn validate_ltxv_decoded_frame(
    frame: NonNull<abi::AvFrame>,
    expected_width: i32,
    expected_height: i32,
    limits: NativeLtxvH264DecodeLimits,
) -> Result<(), NativeVideoCodecLtxvDecodeError> {
    let frame = unsafe { frame.as_ref() };
    if frame.width <= 0
        || frame.height <= 0
        || frame.width % 2 != 0
        || frame.height % 2 != 0
        || frame.width != expected_width
        || frame.height != expected_height
        || frame.format != abi::AV_PIXEL_FORMAT_YUV420P
        || u64::try_from(frame.width)
            .ok()
            .is_none_or(|width| width > limits.maximum_width)
        || u64::try_from(frame.height)
            .ok()
            .is_none_or(|height| height > limits.maximum_height)
        || frame.data[..3].iter().any(|plane| plane.is_null())
        || frame.line_size[0] < frame.width
        || frame.line_size[1] < frame.width / 2
        || frame.line_size[2] < frame.width / 2
    {
        return Err(NativeVideoCodecLtxvDecodeError::InvalidFrame);
    }
    decoded_rgb_byte_count(frame.width, frame.height, limits)?;
    Ok(())
}

fn decoded_rgb_byte_count(
    width: i32,
    height: i32,
    limits: NativeLtxvH264DecodeLimits,
) -> Result<usize, NativeVideoCodecLtxvDecodeError> {
    let width = u64::try_from(width).map_err(|_| NativeVideoCodecLtxvDecodeError::InvalidFrame)?;
    let height =
        u64::try_from(height).map_err(|_| NativeVideoCodecLtxvDecodeError::InvalidFrame)?;
    let pixels = width
        .checked_mul(height)
        .filter(|pixels| *pixels <= limits.maximum_pixels)
        .ok_or(NativeVideoCodecLtxvDecodeError::InvalidFrame)?;
    usize::try_from(
        pixels
            .checked_mul(3)
            .ok_or(NativeVideoCodecLtxvDecodeError::InvalidFrame)?,
    )
    .ok()
    .filter(|bytes| *bytes <= limits.maximum_output_bytes)
    .ok_or(NativeVideoCodecLtxvDecodeError::InvalidFrame)
}

fn map_ltxv_decode_io_error(error: NativeVideoCodecIoError) -> NativeVideoCodecLtxvDecodeError {
    match error {
        NativeVideoCodecIoError::Cancelled => NativeVideoCodecLtxvDecodeError::Cancelled,
        NativeVideoCodecIoError::CallbackResourceExhausted
        | NativeVideoCodecIoError::OutputLimitExceeded
        | NativeVideoCodecIoError::NativeAllocationFailed => {
            NativeVideoCodecLtxvDecodeError::ResourceExhausted {
                phase: "read bounded MP4 input",
            }
        }
        error => NativeVideoCodecLtxvDecodeError::Io(error),
    }
}

fn check_ltxv_decode_status(
    phase: &'static str,
    status: i32,
) -> Result<(), NativeVideoCodecLtxvDecodeError> {
    if status >= 0 {
        return Ok(());
    }
    if status == abi::AV_ERROR_OUT_OF_MEMORY {
        return Err(NativeVideoCodecLtxvDecodeError::ResourceExhausted { phase });
    }
    Err(NativeVideoCodecLtxvDecodeError::NativeCall { phase, status })
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
#[derive(Clone, Copy)]
enum NativeRgb8CodecKind {
    LtxvH264,
    Vp9,
}

#[derive(Clone, Copy)]
enum NativeRgb8Crf {
    Integer(u8),
    SourceFloat(NativeVideoCrf),
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
#[derive(Clone, Copy)]
struct NativeRgb8EncodeProfile {
    kind: NativeRgb8CodecKind,
    container: &'static std::ffi::CStr,
    frame_time_base: abi::AvRational,
    frame_rate: Option<abi::AvRational>,
    crf: NativeRgb8Crf,
    source_pixel_format: i32,
    destination_pixel_format: i32,
    destination_pixel_format_name: &'static std::ffi::CStr,
    source_channels: i32,
}

#[allow(
    dead_code,
    reason = "consumed by the following codec-thread batch bridge"
)]
impl NativeRgb8EncodeProfile {
    fn ltxv_h264(crf: u8) -> Self {
        Self {
            kind: NativeRgb8CodecKind::LtxvH264,
            container: c"mp4",
            frame_time_base: ltxv_frame_time_base(),
            frame_rate: None,
            crf: NativeRgb8Crf::Integer(crf),
            source_pixel_format: abi::AV_PIXEL_FORMAT_RGB24,
            destination_pixel_format: abi::AV_PIXEL_FORMAT_YUV420P,
            destination_pixel_format_name: c"yuv420p",
            source_channels: 3,
        }
    }

    fn vp9_webm(frame_rate: abi::AvRational, crf: NativeVideoCrf) -> Self {
        Self {
            kind: NativeRgb8CodecKind::Vp9,
            container: c"webm",
            frame_time_base: abi::AvRational {
                numerator: frame_rate.denominator,
                denominator: frame_rate.numerator,
            },
            frame_rate: Some(frame_rate),
            crf: NativeRgb8Crf::SourceFloat(crf),
            source_pixel_format: abi::AV_PIXEL_FORMAT_RGB24,
            destination_pixel_format: abi::AV_PIXEL_FORMAT_YUV420P,
            destination_pixel_format_name: c"yuv420p",
            source_channels: 3,
        }
    }

    fn vp9_webm_alpha(frame_rate: abi::AvRational, crf: NativeVideoCrf) -> Self {
        Self {
            kind: NativeRgb8CodecKind::Vp9,
            container: c"webm",
            frame_time_base: abi::AvRational {
                numerator: frame_rate.denominator,
                denominator: frame_rate.numerator,
            },
            frame_rate: Some(frame_rate),
            crf: NativeRgb8Crf::SourceFloat(crf),
            source_pixel_format: abi::AV_PIXEL_FORMAT_RGBA,
            destination_pixel_format: abi::AV_PIXEL_FORMAT_YUVA420P,
            destination_pixel_format_name: c"yuva420p",
            source_channels: 4,
        }
    }

    fn is_vp9(self) -> bool {
        matches!(self.kind, NativeRgb8CodecKind::Vp9)
    }

    fn phase(self, h264: &'static str, vp9: &'static str) -> &'static str {
        if self.is_vp9() { vp9 } else { h264 }
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_ltxv_h264_rgb8_with_check(
    encoder: NonNull<abi::AvCodec>,
    input: &[u8],
    width: i32,
    height: i32,
    crf: u8,
    maximum_packet_iterations: usize,
    functions: &NativeLtxvH264EncodeFunctions,
    output: &mut NativeVideoCodecMemoryOutput<'_>,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    encode_rgb8_frame_with_check(
        NativeRgb8EncodeProfile::ltxv_h264(crf),
        encoder,
        input,
        width,
        height,
        maximum_packet_iterations,
        functions,
        output,
        check_cancellation,
    )
}

#[allow(clippy::too_many_arguments)]
fn encode_rgb8_frame_with_check(
    profile: NativeRgb8EncodeProfile,
    encoder: NonNull<abi::AvCodec>,
    input: &[u8],
    width: i32,
    height: i32,
    maximum_packet_iterations: usize,
    functions: &NativeLtxvH264EncodeFunctions,
    output: &mut NativeVideoCodecMemoryOutput<'_>,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    let mut provide_frame =
        |_frame_index: usize,
         consume: &mut dyn FnMut(&[u8]) -> Result<(), NativeVideoCodecLtxvEncodeError>| {
            consume(input)
        };
    encode_rgb8_frames_with_check(
        profile,
        encoder,
        1,
        width,
        height,
        maximum_packet_iterations,
        functions,
        output,
        &mut provide_frame,
        check_cancellation,
    )
}

#[allow(clippy::too_many_arguments)]
fn encode_rgb8_frames_with_check(
    profile: NativeRgb8EncodeProfile,
    encoder: NonNull<abi::AvCodec>,
    frame_count: usize,
    width: i32,
    height: i32,
    maximum_packet_iterations: usize,
    functions: &NativeLtxvH264EncodeFunctions,
    output: &mut NativeVideoCodecMemoryOutput<'_>,
    provide_frame: &mut impl FnMut(
        usize,
        &mut dyn FnMut(&[u8]) -> Result<(), NativeVideoCodecLtxvEncodeError>,
    ) -> Result<(), NativeVideoCodecLtxvEncodeError>,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    encode_rgb8_frames_with_metadata_check(
        profile,
        encoder,
        frame_count,
        width,
        height,
        maximum_packet_iterations,
        functions,
        output,
        &NativeVideoContainerMetadata::empty(),
        provide_frame,
        check_cancellation,
    )
}

#[allow(clippy::too_many_arguments)]
fn encode_rgb8_frames_with_metadata_check(
    profile: NativeRgb8EncodeProfile,
    encoder: NonNull<abi::AvCodec>,
    frame_count: usize,
    width: i32,
    height: i32,
    maximum_packet_iterations: usize,
    functions: &NativeLtxvH264EncodeFunctions,
    output: &mut NativeVideoCodecMemoryOutput<'_>,
    metadata: &NativeVideoContainerMetadata,
    provide_frame: &mut impl FnMut(
        usize,
        &mut dyn FnMut(&[u8]) -> Result<(), NativeVideoCodecLtxvEncodeError>,
    ) -> Result<(), NativeVideoCodecLtxvEncodeError>,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    if frame_count == 0 {
        return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
    }
    macro_rules! checked_native_call {
        ($expression:expr) => {{
            check_cancellation()?;
            let result = unsafe { $expression };
            check_cancellation()?;
            result
        }};
    }

    check_cancellation()?;
    let mut format_pointer = std::ptr::null_mut();
    let format_status = checked_native_call!((functions.avformat_alloc_output_context2)(
        std::ptr::addr_of_mut!(format_pointer),
        std::ptr::null(),
        profile.container.as_ptr(),
        std::ptr::null(),
    ));
    let allocate_format_phase = profile.phase("allocate MP4 format", "allocate WebM format");
    check_ltxv_native_status(allocate_format_phase, format_status)?;
    let mut format = NativeLtxvFormatContext {
        pointer: NonNull::new(format_pointer).ok_or(
            NativeVideoCodecLtxvEncodeError::NativeAllocation {
                phase: allocate_format_phase,
            },
        )?,
        free: functions.avformat_free_context,
    };
    unsafe { format.pointer.as_mut().io_context = output.context_ptr() };

    if !profile.is_vp9() && !metadata.entries().is_empty() {
        return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
    }
    for (key, value) in metadata.entries() {
        let metadata_pointer = unsafe {
            let projection = format
                .pointer
                .as_ptr()
                .cast::<abi::AvFormatContextMetadataProjection>();
            std::ptr::addr_of_mut!((*projection).metadata)
        };
        check_ltxv_native_status(
            "set WebM container metadata",
            checked_native_call!((functions.av_dict_set)(
                metadata_pointer,
                key.as_ptr(),
                value.as_ptr(),
                0,
            )),
        )?;
    }

    let stream_pointer = checked_native_call!((functions.avformat_new_stream)(
        format.pointer.as_ptr(),
        encoder.as_ptr(),
    ));
    let allocate_stream_phase = profile.phase("allocate MP4 stream", "allocate WebM stream");
    let mut stream =
        NonNull::new(stream_pointer).ok_or(NativeVideoCodecLtxvEncodeError::NativeAllocation {
            phase: allocate_stream_phase,
        })?;
    if unsafe { stream.as_ref().codec_parameters }.is_null() {
        return Err(NativeVideoCodecLtxvEncodeError::NativeAllocation {
            phase: profile.phase(
                "allocate MP4 stream parameters",
                "allocate WebM stream parameters",
            ),
        });
    }

    let codec_pointer = checked_native_call!((functions.avcodec_alloc_context3)(encoder.as_ptr()));
    let codec = NativeLtxvCodecContext {
        pointer: NonNull::new(codec_pointer).ok_or(
            NativeVideoCodecLtxvEncodeError::NativeAllocation {
                phase: profile.phase("allocate libx264 context", "allocate libvpx-vp9 context"),
            },
        )?,
        free: functions.avcodec_free_context,
    };

    let mut video_size = [0_u8; 24];
    write_video_size(width, height, &mut video_size)?;
    check_ltxv_native_status(
        "set video size",
        checked_native_call!((functions.av_opt_set)(
            codec.pointer.as_ptr().cast(),
            c"video_size".as_ptr(),
            video_size.as_ptr().cast(),
            0,
        )),
    )?;
    check_ltxv_native_status(
        "set pixel format",
        checked_native_call!((functions.av_opt_set)(
            codec.pointer.as_ptr().cast(),
            c"pixel_format".as_ptr(),
            profile.destination_pixel_format_name.as_ptr(),
            0,
        )),
    )?;
    let mut time_base_bytes = [0_u8; 24];
    write_rational(profile.frame_time_base, &mut time_base_bytes)?;
    check_ltxv_native_status(
        "set time base",
        checked_native_call!((functions.av_opt_set)(
            codec.pointer.as_ptr().cast(),
            c"time_base".as_ptr(),
            time_base_bytes.as_ptr().cast(),
            0,
        )),
    )?;
    if let Some(frame_rate) = profile.frame_rate {
        let mut frame_rate_bytes = [0_u8; 24];
        write_rational(frame_rate, &mut frame_rate_bytes)?;
        check_ltxv_native_status(
            "set frame rate",
            checked_native_call!((functions.av_opt_set)(
                codec.pointer.as_ptr().cast(),
                c"framerate".as_ptr(),
                frame_rate_bytes.as_ptr().cast(),
                0,
            )),
        )?;
    }
    check_ltxv_native_status(
        "set encoder threads",
        checked_native_call!((functions.av_opt_set_int)(
            codec.pointer.as_ptr().cast(),
            c"threads".as_ptr(),
            1,
            0,
        )),
    )?;
    if profile.is_vp9() {
        check_ltxv_native_status(
            "set zero bit rate",
            checked_native_call!((functions.av_opt_set_int)(
                codec.pointer.as_ptr().cast(),
                c"b".as_ptr(),
                0,
                0,
            )),
        )?;
    } else {
        check_ltxv_native_status(
            "set global header",
            checked_native_call!((functions.av_opt_set_int)(
                codec.pointer.as_ptr().cast(),
                c"flags".as_ptr(),
                i64::from(abi::AV_CODEC_FLAG_GLOBAL_HEADER),
                0,
            )),
        )?;
    }

    let mut dictionary = NativeLtxvDictionary {
        pointer: std::ptr::null_mut(),
        free: functions.av_dict_free,
    };
    let mut crf_bytes = [0_u8; 32];
    match profile.crf {
        NativeRgb8Crf::Integer(value) => {
            let mut cursor = 0;
            write_decimal(u32::from(value), &mut crf_bytes, &mut cursor)?;
        }
        NativeRgb8Crf::SourceFloat(value) => {
            write_python_float(value.value(), &mut crf_bytes)?;
        }
    }
    check_ltxv_native_status(
        "set CRF option",
        checked_native_call!((functions.av_dict_set)(
            std::ptr::addr_of_mut!(dictionary.pointer),
            c"crf".as_ptr(),
            crf_bytes.as_ptr().cast(),
            0,
        )),
    )?;
    if !profile.is_vp9() {
        check_ltxv_native_status(
            "set preset option",
            checked_native_call!((functions.av_dict_set)(
                std::ptr::addr_of_mut!(dictionary.pointer),
                c"preset".as_ptr(),
                c"veryfast".as_ptr(),
                0,
            )),
        )?;
    }
    check_ltxv_native_status(
        profile.phase("open libx264 encoder", "open libvpx-vp9 encoder"),
        checked_native_call!((functions.avcodec_open2)(
            codec.pointer.as_ptr(),
            encoder.as_ptr(),
            std::ptr::addr_of_mut!(dictionary.pointer),
        )),
    )?;
    if !dictionary.pointer.is_null() {
        return Err(NativeVideoCodecLtxvEncodeError::UnconsumedCodecOptions);
    }

    unsafe { stream.as_mut().time_base = profile.frame_time_base };
    check_ltxv_native_status(
        "copy encoder parameters",
        checked_native_call!((functions.avcodec_parameters_from_context)(
            stream.as_ref().codec_parameters,
            codec.pointer.as_ptr(),
        )),
    )?;
    let header_status = checked_native_call!((functions.avformat_write_header)(
        format.pointer.as_ptr(),
        std::ptr::null_mut(),
    ));
    output.check_callback_status()?;
    check_ltxv_native_status(
        profile.phase("write MP4 header", "write WebM header"),
        header_status,
    )?;

    let frame_pointer = checked_native_call!((functions.av_frame_alloc)());
    let mut frame = NativeLtxvFrame {
        pointer: NonNull::new(frame_pointer).ok_or(
            NativeVideoCodecLtxvEncodeError::NativeAllocation {
                phase: "allocate YUV frame",
            },
        )?,
        free: functions.av_frame_free,
    };
    unsafe {
        frame.pointer.as_mut().width = width;
        frame.pointer.as_mut().height = height;
        frame.pointer.as_mut().format = profile.destination_pixel_format;
        frame.pointer.as_mut().presentation_timestamp = 0;
    }
    check_ltxv_native_status(
        "allocate YUV frame buffer",
        checked_native_call!((functions.av_frame_get_buffer)(frame.pointer.as_ptr(), 32)),
    )?;

    let packet_pointer = checked_native_call!((functions.av_packet_alloc)());
    let packet = NativeLtxvPacket {
        pointer: NonNull::new(packet_pointer).ok_or(
            NativeVideoCodecLtxvEncodeError::NativeAllocation {
                phase: "allocate encoded packet",
            },
        )?,
        free: functions.av_packet_free,
    };

    let sws_pointer = checked_native_call!((functions.sws_get_context)(
        width,
        height,
        profile.source_pixel_format,
        width,
        height,
        profile.destination_pixel_format,
        abi::SWS_BILINEAR,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null(),
    ));
    let _sws = NativeLtxvSwsContext {
        pointer: NonNull::new(sws_pointer).ok_or(
            NativeVideoCodecLtxvEncodeError::NativeAllocation {
                phase: "allocate RGB conversion",
            },
        )?,
        free: functions.sws_free_context,
    };
    let source_stride = width
        .checked_mul(profile.source_channels)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let mut source_line_size = [0_i32; abi::AV_NUM_DATA_POINTERS];
    source_line_size[0] = source_stride;
    let mut packet_iterations = maximum_packet_iterations;
    for frame_index in 0..frame_count {
        check_cancellation()?;
        let frame_timestamp = i64::try_from(frame_index)
            .map_err(|_| NativeVideoCodecLtxvEncodeError::InvalidInput)?;
        let mut consumed = false;
        let mut consume_frame = |input: &[u8]| {
            if consumed {
                return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
            }
            consumed = true;
            let expected_bytes = usize::try_from(width)
                .ok()
                .and_then(|width| width.checked_mul(usize::try_from(height).ok()?))
                .and_then(|pixels| {
                    pixels.checked_mul(usize::try_from(profile.source_channels).ok()?)
                })
                .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
            if input.len() != expected_bytes {
                return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
            }
            check_ltxv_native_status(
                "make YUV frame writable",
                checked_native_call!((functions.av_frame_make_writable)(frame.pointer.as_ptr())),
            )?;
            unsafe { frame.pointer.as_mut().presentation_timestamp = frame_timestamp };
            let mut source_data = [std::ptr::null(); abi::AV_NUM_DATA_POINTERS];
            source_data[0] = input.as_ptr();
            let converted_height = checked_native_call!((functions.sws_scale)(
                sws_pointer,
                source_data.as_ptr(),
                source_line_size.as_ptr(),
                0,
                height,
                frame.pointer.as_ref().data.as_ptr(),
                frame.pointer.as_ref().line_size.as_ptr(),
            ));
            if converted_height != height {
                return Err(NativeVideoCodecLtxvEncodeError::NativeCall {
                    phase: "convert RGB frame",
                    status: converted_height,
                });
            }
            check_ltxv_native_status(
                "send RGB frame",
                checked_native_call!((functions.avcodec_send_frame)(
                    codec.pointer.as_ptr(),
                    frame.pointer.as_ptr(),
                )),
            )?;
            drain_rgb8_encode_packets(
                false,
                profile,
                codec.pointer,
                packet.pointer,
                stream,
                format.pointer,
                functions,
                output,
                &mut packet_iterations,
                check_cancellation,
            )
        };
        provide_frame(frame_index, &mut consume_frame)?;
        if !consumed {
            return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
        }
    }
    check_ltxv_native_status(
        profile.phase("flush libx264 encoder", "flush libvpx-vp9 encoder"),
        checked_native_call!((functions.avcodec_send_frame)(
            codec.pointer.as_ptr(),
            std::ptr::null(),
        )),
    )?;
    drain_rgb8_encode_packets(
        true,
        profile,
        codec.pointer,
        packet.pointer,
        stream,
        format.pointer,
        functions,
        output,
        &mut packet_iterations,
        check_cancellation,
    )?;
    let trailer_status =
        checked_native_call!((functions.av_write_trailer)(format.pointer.as_ptr()));
    output.check_callback_status()?;
    check_ltxv_native_status(
        profile.phase("write MP4 trailer", "write WebM trailer"),
        trailer_status,
    )?;
    check_cancellation()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
fn drain_rgb8_encode_packets(
    flushing: bool,
    profile: NativeRgb8EncodeProfile,
    codec: NonNull<abi::AvCodecContext>,
    packet: NonNull<abi::AvPacket>,
    stream: NonNull<abi::AvStream>,
    format: NonNull<abi::AvFormatContext>,
    functions: &NativeLtxvH264EncodeFunctions,
    output: &NativeVideoCodecMemoryOutput<'_>,
    remaining_iterations: &mut usize,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    if matches!(profile.kind, NativeRgb8CodecKind::LtxvH264) {
        return drain_ltxv_h264_packets(
            flushing,
            codec,
            packet,
            stream,
            format,
            functions,
            output,
            remaining_iterations,
            check_cancellation,
        );
    }
    drain_rgb8_encode_packets_inner(
        flushing,
        profile,
        codec,
        packet,
        stream,
        format,
        functions,
        output,
        remaining_iterations,
        check_cancellation,
    )
}

#[allow(clippy::too_many_arguments)]
fn drain_ltxv_h264_packets(
    flushing: bool,
    codec: NonNull<abi::AvCodecContext>,
    packet: NonNull<abi::AvPacket>,
    stream: NonNull<abi::AvStream>,
    format: NonNull<abi::AvFormatContext>,
    functions: &NativeLtxvH264EncodeFunctions,
    output: &NativeVideoCodecMemoryOutput<'_>,
    remaining_iterations: &mut usize,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    drain_rgb8_encode_packets_inner(
        flushing,
        NativeRgb8EncodeProfile::ltxv_h264(0),
        codec,
        packet,
        stream,
        format,
        functions,
        output,
        remaining_iterations,
        check_cancellation,
    )
}

#[allow(clippy::too_many_arguments)]
fn drain_rgb8_encode_packets_inner(
    flushing: bool,
    profile: NativeRgb8EncodeProfile,
    codec: NonNull<abi::AvCodecContext>,
    packet: NonNull<abi::AvPacket>,
    stream: NonNull<abi::AvStream>,
    format: NonNull<abi::AvFormatContext>,
    functions: &NativeLtxvH264EncodeFunctions,
    output: &NativeVideoCodecMemoryOutput<'_>,
    remaining_iterations: &mut usize,
    check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    loop {
        if *remaining_iterations == 0 {
            return Err(NativeVideoCodecLtxvEncodeError::PacketIterationLimit);
        }
        *remaining_iterations -= 1;
        check_cancellation()?;
        let status = unsafe { (functions.avcodec_receive_packet)(codec.as_ptr(), packet.as_ptr()) };
        check_cancellation()?;
        match status {
            0 => {
                unsafe {
                    let packet = &mut *packet.as_ptr();
                    let source_time_base = profile.frame_time_base;
                    let target_time_base = stream.as_ref().time_base;
                    if packet.presentation_timestamp != abi::AV_NO_PRESENTATION_TIMESTAMP {
                        packet.presentation_timestamp = (functions.av_rescale_q)(
                            packet.presentation_timestamp,
                            source_time_base,
                            target_time_base,
                        );
                    }
                    if packet.decoding_timestamp != abi::AV_NO_PRESENTATION_TIMESTAMP {
                        packet.decoding_timestamp = (functions.av_rescale_q)(
                            packet.decoding_timestamp,
                            source_time_base,
                            target_time_base,
                        );
                    }
                    if packet.duration > 0 {
                        packet.duration = (functions.av_rescale_q)(
                            packet.duration,
                            source_time_base,
                            target_time_base,
                        );
                    }
                    packet.stream_index = stream.as_ref().index;
                }
                check_cancellation()?;
                let mux_status = unsafe {
                    (functions.av_interleaved_write_frame)(format.as_ptr(), packet.as_ptr())
                };
                unsafe { (functions.av_packet_unref)(packet.as_ptr()) };
                check_cancellation()?;
                output.check_callback_status()?;
                check_ltxv_native_status(
                    profile.phase("mux H.264 packet", "mux VP9 packet"),
                    mux_status,
                )?;
            }
            abi::AV_ERROR_TRY_AGAIN if !flushing => return Ok(()),
            abi::AV_ERROR_END_OF_FILE => return Ok(()),
            abi::AV_ERROR_TRY_AGAIN => {
                return Err(NativeVideoCodecLtxvEncodeError::NativeCall {
                    phase: profile
                        .phase("drain flushed H.264 packets", "drain flushed VP9 packets"),
                    status,
                });
            }
            status => {
                return check_ltxv_native_status(
                    profile.phase("receive H.264 packet", "receive VP9 packet"),
                    status,
                );
            }
        }
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
fn check_ltxv_native_status(
    phase: &'static str,
    status: i32,
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    if status >= 0 {
        return Ok(());
    }
    if status == abi::AV_ERROR_OUT_OF_MEMORY {
        return Err(NativeVideoCodecLtxvEncodeError::ResourceExhausted { phase });
    }
    Err(NativeVideoCodecLtxvEncodeError::NativeCall { phase, status })
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
fn ltxv_frame_time_base() -> abi::AvRational {
    abi::AvRational {
        numerator: 1,
        denominator: 1,
    }
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
fn write_video_size(
    width: i32,
    height: i32,
    buffer: &mut [u8; 24],
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    let mut cursor = 0;
    write_decimal(
        u32::try_from(width).map_err(|_| NativeVideoCodecLtxvEncodeError::InvalidInput)?,
        buffer,
        &mut cursor,
    )?;
    let separator = buffer
        .get_mut(cursor)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    *separator = b'x';
    cursor += 1;
    write_decimal(
        u32::try_from(height).map_err(|_| NativeVideoCodecLtxvEncodeError::InvalidInput)?,
        buffer,
        &mut cursor,
    )?;
    Ok(())
}

fn write_rational(
    value: abi::AvRational,
    buffer: &mut [u8; 24],
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    let mut cursor = 0;
    write_decimal(
        u32::try_from(value.numerator)
            .ok()
            .filter(|value| *value > 0)
            .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?,
        buffer,
        &mut cursor,
    )?;
    let separator = buffer
        .get_mut(cursor)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    *separator = b'/';
    cursor += 1;
    write_decimal(
        u32::try_from(value.denominator)
            .ok()
            .filter(|value| *value > 0)
            .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?,
        buffer,
        &mut cursor,
    )?;
    Ok(())
}

#[allow(
    dead_code,
    reason = "consumed by the following retained H.264 decode leaf"
)]
fn write_decimal<const N: usize>(
    mut value: u32,
    buffer: &mut [u8; N],
    cursor: &mut usize,
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    let mut reversed = [0_u8; 10];
    let mut digits = 0;
    loop {
        reversed[digits] = b'0' + u8::try_from(value % 10).unwrap_or(0);
        digits += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for index in (0..digits).rev() {
        let target = buffer
            .get_mut(*cursor)
            .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
        *target = reversed[index];
        *cursor += 1;
    }
    let terminator = buffer
        .get_mut(*cursor)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    *terminator = 0;
    Ok(())
}

struct FixedByteWriter<'buffer> {
    buffer: &'buffer mut [u8],
    length: usize,
}

impl Write for FixedByteWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self
            .length
            .checked_add(bytes.len())
            .filter(|end| *end <= self.buffer.len())
            .ok_or_else(|| io::Error::other("floating-point option exceeded its fixed buffer"))?;
        self.buffer[self.length..end].copy_from_slice(bytes);
        self.length = end;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_python_float(
    value: f64,
    buffer: &mut [u8; 32],
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    let mut serialized = [0_u8; 32];
    let mut writer = FixedByteWriter {
        buffer: &mut serialized,
        length: 0,
    };
    serde_json::to_writer(&mut writer, &value)
        .map_err(|_| NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let serialized_length = writer.length;
    let serialized = serialized
        .get(..serialized_length)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    let mut length = serialized_length;
    buffer
        .get_mut(..length)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?
        .copy_from_slice(serialized);

    let sign_bytes = usize::from(serialized.first() == Some(&b'-'));
    let fractional_digits = serialized.get(sign_bytes + 2..).filter(|_| {
        serialized.get(sign_bytes) == Some(&b'0') && serialized.get(sign_bytes + 1) == Some(&b'.')
    });
    if !serialized.contains(&b'e')
        && let Some(fractional_digits) = fractional_digits
        && let Some(first_nonzero) = fractional_digits.iter().position(|digit| *digit != b'0')
        && first_nonzero >= 4
    {
        let significant = fractional_digits
            .get(first_nonzero..)
            .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
        let mut cursor = 0;
        if sign_bytes == 1 {
            write_option_byte(buffer, &mut cursor, b'-')?;
        }
        let first = *significant
            .first()
            .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
        write_option_byte(buffer, &mut cursor, first)?;
        if let Some(remaining) = significant
            .get(1..)
            .filter(|remaining| !remaining.is_empty())
        {
            write_option_byte(buffer, &mut cursor, b'.')?;
            write_option_bytes(buffer, &mut cursor, remaining)?;
        }
        write_option_bytes(buffer, &mut cursor, b"e-")?;
        let exponent = u32::try_from(first_nonzero + 1)
            .map_err(|_| NativeVideoCodecLtxvEncodeError::InvalidInput)?;
        if exponent < 10 {
            write_option_byte(buffer, &mut cursor, b'0')?;
        }
        write_decimal(exponent, buffer, &mut cursor)?;
        length = cursor;
    }

    if let Some(exponent) = buffer[..length].iter().position(|byte| *byte == b'e') {
        let digit_start = exponent
            .checked_add(2)
            .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
        if buffer
            .get(exponent + 1)
            .is_some_and(|byte| matches!(byte, b'+' | b'-'))
            && length.saturating_sub(digit_start) == 1
        {
            if length + 1 >= buffer.len() {
                return Err(NativeVideoCodecLtxvEncodeError::InvalidInput);
            }
            buffer.copy_within(digit_start..length, digit_start + 1);
            buffer[digit_start] = b'0';
            length += 1;
        }
    }
    let terminator = buffer
        .get_mut(length)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    *terminator = 0;
    Ok(())
}

fn write_option_byte<const N: usize>(
    buffer: &mut [u8; N],
    cursor: &mut usize,
    byte: u8,
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    let slot = buffer
        .get_mut(*cursor)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    *slot = byte;
    *cursor = cursor
        .checked_add(1)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    Ok(())
}

fn write_option_bytes<const N: usize>(
    buffer: &mut [u8; N],
    cursor: &mut usize,
    bytes: &[u8],
) -> Result<(), NativeVideoCodecLtxvEncodeError> {
    let end = cursor
        .checked_add(bytes.len())
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?;
    buffer
        .get_mut(*cursor..end)
        .ok_or(NativeVideoCodecLtxvEncodeError::InvalidInput)?
        .copy_from_slice(bytes);
    *cursor = end;
    Ok(())
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

fn has_exact_ltxv_h264_dependency_contract(binding: &NativeVideoCodecBinding) -> bool {
    let closure = &binding.load.closure;
    closure
        .encoder_providers()
        .get("libx264")
        .is_some_and(|provider| provider == "x264")
        && closure.dependencies().contains_key("x264")
        && closure.dependency_certificates().contains_key("x264")
        && closure
            .edges()
            .iter()
            .any(|edge| edge.consumer() == "avcodec" && edge.dependency() == "x264")
        && binding
            .load
            .loaded
            .libraries
            .iter()
            .any(|library| library.identity == "x264")
}

fn has_exact_video_codec_suite_dependency_contract(binding: &NativeVideoCodecBinding) -> bool {
    let closure = &binding.load.closure;
    let expected_providers = BTreeMap::from([
        ("aac".to_owned(), "avcodec".to_owned()),
        ("libsvtav1".to_owned(), "svtav1".to_owned()),
        ("libvpx-vp9".to_owned(), "vpx".to_owned()),
        ("libx264".to_owned(), "x264".to_owned()),
    ]);
    if closure.encoder_providers() != &expected_providers {
        return false;
    }
    for dependency in ["svtav1", "vpx", "x264"] {
        if !closure.dependencies().contains_key(dependency)
            || !closure.dependency_certificates().contains_key(dependency)
            || !closure
                .edges()
                .iter()
                .any(|edge| edge.consumer() == "avcodec" && edge.dependency() == dependency)
            || !binding
                .load
                .loaded
                .libraries
                .iter()
                .any(|library| library.identity == dependency)
        {
            return false;
        }
    }
    binding
        .load
        .loaded
        .libraries
        .iter()
        .any(|library| library.identity == "avcodec")
}

struct NativeVideoCodecSuiteDescriptors {
    aac_encoder: NonNull<abi::AvCodec>,
    svt_av1_encoder: NonNull<abi::AvCodec>,
    vpx_vp9_encoder: NonNull<abi::AvCodec>,
    aac_decoder: NonNull<abi::AvCodec>,
    vp9_decoder: NonNull<abi::AvCodec>,
    av1_decoder: NonNull<abi::AvCodec>,
}

fn admit_video_suite_with_check(
    symbols: &NativeVideoCodecSymbols,
    loaded: &LoadedVideoCodecLibraries,
    mut check_cancellation: impl FnMut() -> Result<(), CancellationError>,
) -> Result<NativeVideoCodecSuiteDescriptors, NativeVideoCodecSuiteAdmissionError> {
    fn encoder(
        symbols: &NativeVideoCodecSymbols,
        loaded: &LoadedVideoCodecLibraries,
        name: &'static std::ffi::CStr,
        identity: &'static str,
        check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
    ) -> Result<NonNull<abi::AvCodec>, NativeVideoCodecSuiteAdmissionError> {
        check_cancellation()?;
        let descriptor = unsafe { (symbols.avcodec.avcodec_find_encoder_by_name)(name.as_ptr()) };
        check_cancellation()?;
        let descriptor = NonNull::new(descriptor.cast_mut())
            .ok_or(NativeVideoCodecSuiteAdmissionError::MissingEncoder { encoder: identity })?;
        prove_codec_descriptor_provider(loaded, descriptor).map_err(|_| {
            NativeVideoCodecSuiteAdmissionError::DescriptorProviderMismatch { codec: identity }
        })?;
        check_cancellation()?;
        Ok(descriptor)
    }

    fn decoder(
        symbols: &NativeVideoCodecSymbols,
        loaded: &LoadedVideoCodecLibraries,
        codec_id: std::ffi::c_int,
        identity: &'static str,
        check_cancellation: &mut impl FnMut() -> Result<(), CancellationError>,
    ) -> Result<NonNull<abi::AvCodec>, NativeVideoCodecSuiteAdmissionError> {
        check_cancellation()?;
        let descriptor = unsafe { (symbols.avcodec.avcodec_find_decoder)(codec_id) };
        check_cancellation()?;
        let descriptor = NonNull::new(descriptor.cast_mut())
            .ok_or(NativeVideoCodecSuiteAdmissionError::MissingDecoder { decoder: identity })?;
        prove_codec_descriptor_provider(loaded, descriptor).map_err(|_| {
            NativeVideoCodecSuiteAdmissionError::DescriptorProviderMismatch { codec: identity }
        })?;
        check_cancellation()?;
        Ok(descriptor)
    }

    let aac_encoder = encoder(symbols, loaded, c"aac", "aac", &mut check_cancellation)?;
    let svt_av1_encoder = encoder(
        symbols,
        loaded,
        c"libsvtav1",
        "libsvtav1",
        &mut check_cancellation,
    )?;
    let vpx_vp9_encoder = encoder(
        symbols,
        loaded,
        c"libvpx-vp9",
        "libvpx-vp9",
        &mut check_cancellation,
    )?;
    let aac_decoder = decoder(
        symbols,
        loaded,
        abi::AV_CODEC_ID_AAC,
        "aac",
        &mut check_cancellation,
    )?;
    let vp9_decoder = decoder(
        symbols,
        loaded,
        abi::AV_CODEC_ID_VP9,
        "vp9",
        &mut check_cancellation,
    )?;
    let av1_decoder = decoder(
        symbols,
        loaded,
        abi::AV_CODEC_ID_AV1,
        "av1",
        &mut check_cancellation,
    )?;
    check_cancellation()?;
    Ok(NativeVideoCodecSuiteDescriptors {
        aac_encoder,
        svt_av1_encoder,
        vpx_vp9_encoder,
        aac_decoder,
        vp9_decoder,
        av1_decoder,
    })
}

fn admit_ltxv_h264_with_check(
    symbols: &NativeVideoCodecSymbols,
    loaded: &LoadedVideoCodecLibraries,
    mut check_cancellation: impl FnMut() -> Result<(), CancellationError>,
) -> Result<(NonNull<abi::AvCodec>, NonNull<abi::AvCodec>), NativeVideoCodecLtxvAdmissionError> {
    check_cancellation()?;
    let encoder = unsafe { (symbols.avcodec.avcodec_find_encoder_by_name)(c"libx264".as_ptr()) };
    check_cancellation()?;
    let encoder = NonNull::new(encoder.cast_mut())
        .ok_or(NativeVideoCodecLtxvAdmissionError::MissingLibx264Encoder)?;
    check_cancellation()?;
    prove_codec_descriptor_provider(loaded, encoder)
        .map_err(|_| NativeVideoCodecLtxvAdmissionError::EncoderProviderMismatch)?;
    check_cancellation()?;

    check_cancellation()?;
    let decoder = unsafe { (symbols.avcodec.avcodec_find_decoder)(abi::AV_CODEC_ID_H264) };
    check_cancellation()?;
    let decoder = NonNull::new(decoder.cast_mut())
        .ok_or(NativeVideoCodecLtxvAdmissionError::MissingH264Decoder)?;
    check_cancellation()?;
    prove_codec_descriptor_provider(loaded, decoder)
        .map_err(|_| NativeVideoCodecLtxvAdmissionError::DecoderProviderMismatch)?;
    check_cancellation()?;
    check_cancellation()?;
    Ok((encoder, decoder))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn prove_codec_descriptor_provider(
    loaded: &LoadedVideoCodecLibraries,
    descriptor: NonNull<abi::AvCodec>,
) -> Result<(), NativeVideoCodecLoadError> {
    use std::ffi::CStr;

    let library = loaded
        .libraries
        .iter()
        .find(|library| library.identity == "avcodec")
        .ok_or(NativeVideoCodecLoadError::InvalidClosure)?;
    let link_map = loaded_link_map(library)?;
    let mut information = unsafe { std::mem::zeroed::<libc::Dl_info>() };
    let mut provider: *mut c_void = std::ptr::null_mut();
    let status = unsafe {
        libc::dladdr1(
            descriptor.as_ptr().cast(),
            std::ptr::addr_of_mut!(information),
            std::ptr::addr_of_mut!(provider),
            2,
        )
    };
    if status == 0
        || provider.cast::<LoaderLinkMap>() != link_map
        || information.dli_fname.is_null()
        || information.dli_fbase as usize != unsafe { (*link_map).address }
        || unsafe { CStr::from_ptr(information.dli_fname) }.to_bytes()
            != library.path.as_os_str().as_encoded_bytes()
    {
        return Err(NativeVideoCodecLoadError::BindingProof(
            "codec registry descriptor did not originate in retained avcodec".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
fn prove_codec_descriptor_provider(
    loaded: &LoadedVideoCodecLibraries,
    descriptor: NonNull<abi::AvCodec>,
) -> Result<(), NativeVideoCodecLoadError> {
    let _ = loaded;
    let _ = descriptor;
    Err(NativeVideoCodecLoadError::UnsupportedTarget)
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
    use comfy_tensor::{CpuWorkspaceAuthority, DType, DeviceId, StreamId, TensorDescriptor};
    use std::{
        fs,
        process::Command,
        sync::{
            Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
    };

    static AVIO_EVENTS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    static AVIO_MALLOC_RETURNS_NULL: AtomicBool = AtomicBool::new(false);
    static AVIO_CONTEXT_RETURNS_NULL: AtomicBool = AtomicBool::new(false);

    #[repr(C)]
    struct MockAvIoContext {
        prefix: abi::AvIoContext,
        opaque: *mut c_void,
        read_packet: Option<abi::AvIoReadPacket>,
        write_packet: Option<abi::AvIoWritePacket>,
    }

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
        opaque: *mut c_void,
        read_packet: Option<abi::AvIoReadPacket>,
        write_packet: Option<abi::AvIoWritePacket>,
        _seek: Option<abi::AvIoSeek>,
    ) -> *mut abi::AvIoContext {
        record_avio_event("alloc_context");
        if AVIO_CONTEXT_RETURNS_NULL.swap(false, Ordering::AcqRel) {
            return std::ptr::null_mut();
        }
        let context = Box::new(MockAvIoContext {
            prefix: abi::AvIoContext {
                class: std::ptr::null(),
                buffer,
            },
            opaque,
            read_packet,
            write_packet,
        });
        Box::into_raw(context).cast()
    }

    unsafe extern "C" fn mock_avio_context_free(context: *mut *mut abi::AvIoContext) {
        record_avio_event("context_free");
        if context.is_null() {
            return;
        }
        let pointer = unsafe { *context };
        if !pointer.is_null() {
            drop(unsafe { Box::from_raw(pointer.cast::<MockAvIoContext>()) });
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

    #[derive(Clone, Copy)]
    enum MissingCodecDescriptor {
        None,
        LtxvEncoder,
        H264Decoder,
        SuiteEncoder(&'static str),
        SuiteDecoder(std::ffi::c_int),
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "the Linux-only binding test synchronously compiles tiny reviewed-symbol ELF fixtures before dlmopen"
    )]
    fn binding_fixture(
        changed_version: Option<(&str, u32)>,
        missing_codec_descriptor: MissingCodecDescriptor,
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
            if identity == "avcodec" {
                source.push_str(
                    "struct AVCodec { unsigned int witness; };\n\
                     static int same(const char *left, const char *right) {\n\
                         if (left == 0 || right == 0) return 0;\n\
                         while (*left != '\\0' && *left == *right) { ++left; ++right; }\n\
                         return *left == *right;\n\
                     }\n\
                     static const struct AVCodec ltxv_encoder = { 1u };\n\
                     static const struct AVCodec h264_decoder = { 2u };\n\
                     static const struct AVCodec aac_encoder = { 3u };\n\
                     static const struct AVCodec svt_av1_encoder = { 4u };\n\
                     static const struct AVCodec vpx_vp9_encoder = { 5u };\n\
                     static const struct AVCodec aac_decoder = { 6u };\n\
                     static const struct AVCodec vp9_decoder = { 7u };\n\
                     static const struct AVCodec av1_decoder = { 8u };\n",
                );
            }
            for symbol in symbols {
                if *symbol == version_symbol {
                    writeln!(
                        source,
                        "unsigned int {symbol}(void) {{ return {actual_version}u; }}"
                    )?;
                } else if *symbol == "avcodec_find_encoder_by_name" {
                    let missing = match missing_codec_descriptor {
                        MissingCodecDescriptor::LtxvEncoder => Some("libx264"),
                        MissingCodecDescriptor::SuiteEncoder(name) => Some(name),
                        MissingCodecDescriptor::None
                        | MissingCodecDescriptor::H264Decoder
                        | MissingCodecDescriptor::SuiteDecoder(_) => None,
                    };
                    let missing = missing.unwrap_or("");
                    writeln!(
                        source,
                        "const struct AVCodec *{symbol}(const char *name) {{\n\
                         if (name == 0 || same(name, \"{missing}\")) return 0;\n\
                         if (same(name, \"libx264\")) return &ltxv_encoder;\n\
                         if (same(name, \"aac\")) return &aac_encoder;\n\
                         if (same(name, \"libsvtav1\")) return &svt_av1_encoder;\n\
                         if (same(name, \"libvpx-vp9\")) return &vpx_vp9_encoder;\n\
                         return 0;\n\
                         }}"
                    )?;
                } else if *symbol == "avcodec_find_decoder" {
                    let missing = match missing_codec_descriptor {
                        MissingCodecDescriptor::H264Decoder => Some(abi::AV_CODEC_ID_H264),
                        MissingCodecDescriptor::SuiteDecoder(codec_id) => Some(codec_id),
                        MissingCodecDescriptor::None
                        | MissingCodecDescriptor::LtxvEncoder
                        | MissingCodecDescriptor::SuiteEncoder(_) => None,
                    };
                    let missing = missing.unwrap_or(-1);
                    writeln!(
                        source,
                        "const struct AVCodec *{symbol}(int codec_id) {{\n\
                         if (codec_id == {missing}) return 0;\n\
                         if (codec_id == 27) return &h264_decoder;\n\
                         if (codec_id == 86018) return &aac_decoder;\n\
                         if (codec_id == 167) return &vp9_decoder;\n\
                         if (codec_id == 225) return &av1_decoder;\n\
                         return 0;\n\
                         }}"
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
        let fixture = binding_fixture(None, MissingCodecDescriptor::None)?;
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
        let fixture = binding_fixture(Some(("avformat", 0x3d0765)), MissingCodecDescriptor::None)?;
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
        let fixture = binding_fixture(None, MissingCodecDescriptor::None)?;
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
        let fixture = binding_fixture(None, MissingCodecDescriptor::None)?;
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
    fn retained_ltxv_h264_admission_uses_exact_registered_codec_pair()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = binding_fixture(None, MissingCodecDescriptor::None)?;
        let loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        let (symbols, _) = bind_video_codec_projection_with_check(
            &loaded,
            &fixture.binding_projection,
            || Ok(()),
        )?;
        let (encoder, decoder) = admit_ltxv_h264_with_check(&symbols, &loaded, || Ok(()))?;
        assert_ne!(encoder, decoder);
        drop(symbols);
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );
        Ok(())
    }

    #[test]
    fn retained_ltxv_h264_admission_rejects_missing_registry_entries()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        for (missing, expected_encoder_error) in [
            (MissingCodecDescriptor::LtxvEncoder, true),
            (MissingCodecDescriptor::H264Decoder, false),
        ] {
            reset_close_order()?;
            let fixture = binding_fixture(None, missing)?;
            let loaded = load_video_codec_projection(
                &fixture.load_projection,
                &CancellationToken::default(),
            )?;
            let (symbols, _) = bind_video_codec_projection_with_check(
                &loaded,
                &fixture.binding_projection,
                || Ok(()),
            )?;
            let result = admit_ltxv_h264_with_check(&symbols, &loaded, || Ok(()));
            if expected_encoder_error {
                assert!(matches!(
                    result,
                    Err(NativeVideoCodecLtxvAdmissionError::MissingLibx264Encoder)
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(NativeVideoCodecLtxvAdmissionError::MissingH264Decoder)
                ));
            }
            drop(symbols);
            drop(loaded);
            assert_eq!(
                close_order()?,
                ["swscale", "swresample", "avutil", "avformat", "avcodec"]
            );
        }
        Ok(())
    }

    #[test]
    fn retained_ltxv_h264_admission_rejects_wrong_descriptor_provider()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = binding_fixture(None, MissingCodecDescriptor::None)?;
        let mut loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        let (symbols, _) = bind_video_codec_projection_with_check(
            &loaded,
            &fixture.binding_projection,
            || Ok(()),
        )?;
        loaded
            .libraries
            .iter_mut()
            .find(|library| library.identity == "avcodec")
            .ok_or("fixture retained avcodec image is missing")?
            .path = PathBuf::from("/wrong/retained/avcodec");
        assert!(matches!(
            admit_ltxv_h264_with_check(&symbols, &loaded, || Ok(())),
            Err(NativeVideoCodecLtxvAdmissionError::EncoderProviderMismatch)
        ));
        drop(symbols);
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );
        Ok(())
    }

    #[test]
    fn retained_ltxv_h264_admission_cancellation_is_atomic_and_retryable()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        let fixture = binding_fixture(None, MissingCodecDescriptor::None)?;
        for cancellation_check in 1..=9 {
            reset_close_order()?;
            let loaded = load_video_codec_projection(
                &fixture.load_projection,
                &CancellationToken::default(),
            )?;
            let (symbols, _) = bind_video_codec_projection_with_check(
                &loaded,
                &fixture.binding_projection,
                || Ok(()),
            )?;
            let cancellation = CancellationToken::default();
            let mut checks = 0;
            assert!(matches!(
                admit_ltxv_h264_with_check(&symbols, &loaded, || {
                    checks += 1;
                    if checks == cancellation_check {
                        cancellation.cancel();
                    }
                    cancellation.check()
                }),
                Err(NativeVideoCodecLtxvAdmissionError::Cancelled)
            ));
            drop(symbols);
            drop(loaded);
            assert_eq!(
                close_order()?,
                ["swscale", "swresample", "avutil", "avformat", "avcodec"]
            );
        }

        reset_close_order()?;
        let loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        let (symbols, _) = bind_video_codec_projection_with_check(
            &loaded,
            &fixture.binding_projection,
            || Ok(()),
        )?;
        admit_ltxv_h264_with_check(&symbols, &loaded, || Ok(()))?;
        drop(symbols);
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );
        Ok(())
    }

    #[test]
    fn retained_video_codec_suite_admission_uses_exact_registered_codec_set()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = binding_fixture(None, MissingCodecDescriptor::None)?;
        let loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        let (symbols, _) = bind_video_codec_projection_with_check(
            &loaded,
            &fixture.binding_projection,
            || Ok(()),
        )?;
        let descriptors = admit_video_suite_with_check(&symbols, &loaded, || Ok(()))?;
        let pointers = [
            descriptors.aac_encoder,
            descriptors.svt_av1_encoder,
            descriptors.vpx_vp9_encoder,
            descriptors.aac_decoder,
            descriptors.vp9_decoder,
            descriptors.av1_decoder,
        ]
        .map(NonNull::as_ptr)
        .into_iter()
        .collect::<BTreeSet<_>>();
        assert_eq!(pointers.len(), 6);
        drop(symbols);
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );
        Ok(())
    }

    #[test]
    fn retained_video_codec_suite_admission_rejects_each_missing_descriptor()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        for (missing, missing_identity, encoder) in [
            (MissingCodecDescriptor::SuiteEncoder("aac"), "aac", true),
            (
                MissingCodecDescriptor::SuiteEncoder("libsvtav1"),
                "libsvtav1",
                true,
            ),
            (
                MissingCodecDescriptor::SuiteEncoder("libvpx-vp9"),
                "libvpx-vp9",
                true,
            ),
            (
                MissingCodecDescriptor::SuiteDecoder(abi::AV_CODEC_ID_AAC),
                "aac",
                false,
            ),
            (
                MissingCodecDescriptor::SuiteDecoder(abi::AV_CODEC_ID_VP9),
                "vp9",
                false,
            ),
            (
                MissingCodecDescriptor::SuiteDecoder(abi::AV_CODEC_ID_AV1),
                "av1",
                false,
            ),
        ] {
            reset_close_order()?;
            let fixture = binding_fixture(None, missing)?;
            let loaded = load_video_codec_projection(
                &fixture.load_projection,
                &CancellationToken::default(),
            )?;
            let (symbols, _) = bind_video_codec_projection_with_check(
                &loaded,
                &fixture.binding_projection,
                || Ok(()),
            )?;
            let result = admit_video_suite_with_check(&symbols, &loaded, || Ok(()));
            if encoder {
                assert!(matches!(
                    result,
                    Err(NativeVideoCodecSuiteAdmissionError::MissingEncoder { encoder })
                        if encoder == missing_identity
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(NativeVideoCodecSuiteAdmissionError::MissingDecoder { decoder })
                        if decoder == missing_identity
                ));
            }
            drop(symbols);
            drop(loaded);
            assert_eq!(
                close_order()?,
                ["swscale", "swresample", "avutil", "avformat", "avcodec"]
            );
        }
        Ok(())
    }

    #[test]
    fn retained_video_codec_suite_admission_rejects_wrong_descriptor_provider()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        reset_close_order()?;
        let fixture = binding_fixture(None, MissingCodecDescriptor::None)?;
        let mut loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        let (symbols, _) = bind_video_codec_projection_with_check(
            &loaded,
            &fixture.binding_projection,
            || Ok(()),
        )?;
        loaded
            .libraries
            .iter_mut()
            .find(|library| library.identity == "avcodec")
            .ok_or("fixture retained avcodec image is missing")?
            .path = PathBuf::from("/wrong/retained/avcodec");
        assert!(matches!(
            admit_video_suite_with_check(&symbols, &loaded, || Ok(())),
            Err(NativeVideoCodecSuiteAdmissionError::DescriptorProviderMismatch { codec: "aac" })
        ));
        drop(symbols);
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );
        Ok(())
    }

    #[test]
    fn retained_video_codec_suite_admission_cancellation_is_atomic_and_retryable()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec loader test mutex was poisoned")?;
        let fixture = binding_fixture(None, MissingCodecDescriptor::None)?;
        for cancellation_check in 1..=19 {
            reset_close_order()?;
            let loaded = load_video_codec_projection(
                &fixture.load_projection,
                &CancellationToken::default(),
            )?;
            let (symbols, _) = bind_video_codec_projection_with_check(
                &loaded,
                &fixture.binding_projection,
                || Ok(()),
            )?;
            let cancellation = CancellationToken::default();
            let mut checks = 0;
            assert!(matches!(
                admit_video_suite_with_check(&symbols, &loaded, || {
                    checks += 1;
                    if checks == cancellation_check {
                        cancellation.cancel();
                    }
                    cancellation.check()
                }),
                Err(NativeVideoCodecSuiteAdmissionError::Cancelled)
            ));
            drop(symbols);
            drop(loaded);
            assert_eq!(
                close_order()?,
                ["swscale", "swresample", "avutil", "avformat", "avcodec"]
            );
        }

        reset_close_order()?;
        let loaded =
            load_video_codec_projection(&fixture.load_projection, &CancellationToken::default())?;
        let (symbols, _) = bind_video_codec_projection_with_check(
            &loaded,
            &fixture.binding_projection,
            || Ok(()),
        )?;
        admit_video_suite_with_check(&symbols, &loaded, || Ok(()))?;
        drop(symbols);
        drop(loaded);
        assert_eq!(
            close_order()?,
            ["swscale", "swresample", "avutil", "avformat", "avcodec"]
        );
        Ok(())
    }

    static DEMUX_EVENTS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    static DEMUX_MODE: AtomicUsize = AtomicUsize::new(0);
    static DEMUX_READ_BYTES: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    static DEMUX_DECODER: std::sync::atomic::AtomicPtr<abi::AvCodec> =
        std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

    #[allow(
        clippy::vec_box,
        reason = "the mock exposes stable stream and codec-parameter addresses through FFI"
    )]
    #[repr(C)]
    struct MockDemuxFormat {
        prefix: abi::AvFormatContext,
        parameters: Vec<Box<u8>>,
        streams: Vec<Box<abi::AvStream>>,
        stream_pointers: Vec<*mut abi::AvStream>,
    }

    fn record_demux_event(event: &'static str) {
        match DEMUX_EVENTS.lock() {
            Ok(mut events) => events.push(event),
            Err(_) => std::process::abort(),
        }
    }

    fn reset_demux_fixture(mode: usize) -> Result<(), Box<dyn std::error::Error>> {
        DEMUX_MODE.store(mode, Ordering::Release);
        DEMUX_EVENTS
            .lock()
            .map_err(|_| "demux event mutex was poisoned")?
            .clear();
        DEMUX_READ_BYTES
            .lock()
            .map_err(|_| "demux read mutex was poisoned")?
            .clear();
        Ok(())
    }

    unsafe extern "C" fn mock_demux_format_alloc() -> *mut abi::AvFormatContext {
        record_demux_event("format_alloc");
        let allocation = Box::new(MockDemuxFormat {
            prefix: abi::AvFormatContext {
                class: std::ptr::null(),
                input_format: std::ptr::null(),
                output_format: std::ptr::null(),
                private_data: std::ptr::null_mut(),
                io_context: std::ptr::null_mut(),
                context_flags: 0,
                stream_count: 0,
                streams: std::ptr::null_mut(),
            },
            parameters: Vec::new(),
            streams: Vec::new(),
            stream_pointers: Vec::new(),
        });
        Box::into_raw(allocation).cast()
    }

    unsafe extern "C" fn mock_demux_format_open(
        format: *mut *mut abi::AvFormatContext,
        _url: *const std::ffi::c_char,
        _input_format: *const abi::AvInputFormat,
        _options: *mut *mut abi::AvDictionary,
    ) -> i32 {
        record_demux_event("format_open");
        if format.is_null() || unsafe { (*format).is_null() } {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        let format_pointer = unsafe { *format };
        let io_context = unsafe { (*format_pointer).io_context }.cast::<MockAvIoContext>();
        if io_context.is_null() {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        let mut bytes = [0_u8; 3];
        let read = unsafe { (*io_context).read_packet };
        let Some(read) = read else {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        };
        let read_count = unsafe { read((*io_context).opaque, bytes.as_mut_ptr(), 3) };
        if read_count != 3 {
            return read_count.min(abi::AV_ERROR_INVALID_ARGUMENT);
        }
        match DEMUX_READ_BYTES.lock() {
            Ok(mut observed) => observed.extend_from_slice(&bytes),
            Err(_) => std::process::abort(),
        }

        if DEMUX_MODE.load(Ordering::Acquire) == 3 {
            unsafe { mock_demux_format_close(format) };
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        let allocation = unsafe { &mut *format_pointer.cast::<MockDemuxFormat>() };
        let mode = DEMUX_MODE.load(Ordering::Acquire);
        let stream_count = if mode == 1 {
            0
        } else if mode == 4 {
            2
        } else {
            1
        };
        for index in 0..stream_count {
            let mut parameters = Box::new(0_u8);
            let parameters_pointer = std::ptr::addr_of_mut!(*parameters).cast();
            let mut stream = Box::new(abi::AvStream {
                class: std::ptr::null(),
                index,
                identifier: 0,
                codec_parameters: parameters_pointer,
                private_data: std::ptr::null_mut(),
                time_base: ltxv_frame_time_base(),
            });
            allocation
                .stream_pointers
                .push(std::ptr::addr_of_mut!(*stream));
            allocation.parameters.push(parameters);
            allocation.streams.push(stream);
        }
        allocation.prefix.stream_count =
            u32::try_from(allocation.stream_pointers.len()).unwrap_or(u32::MAX);
        allocation.prefix.streams = allocation.stream_pointers.as_mut_ptr();
        0
    }

    unsafe extern "C" fn mock_demux_format_close(format: *mut *mut abi::AvFormatContext) {
        record_demux_event("format_close");
        if format.is_null() {
            return;
        }
        let pointer = unsafe { *format };
        if !pointer.is_null() {
            drop(unsafe { Box::from_raw(pointer.cast::<MockDemuxFormat>()) });
            unsafe { *format = std::ptr::null_mut() };
        }
    }

    unsafe extern "C" fn mock_demux_format_free(format: *mut abi::AvFormatContext) {
        record_demux_event("format_free");
        if !format.is_null() {
            drop(unsafe { Box::from_raw(format.cast::<MockDemuxFormat>()) });
        }
    }

    unsafe extern "C" fn mock_demux_find_best_stream(
        _format: *mut abi::AvFormatContext,
        media_type: i32,
        _wanted_stream: i32,
        _related_stream: i32,
        decoder: *mut *const abi::AvCodec,
        _flags: i32,
    ) -> i32 {
        record_demux_event("find_stream");
        if media_type != abi::AV_MEDIA_TYPE_VIDEO || decoder.is_null() {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        let selected = if DEMUX_MODE.load(Ordering::Acquire) == 2 {
            NonNull::<u16>::dangling().as_ptr().cast()
        } else {
            DEMUX_DECODER.load(Ordering::Acquire)
        };
        unsafe { *decoder = selected };
        0
    }

    fn mock_demux_functions() -> NativeLtxvH264DemuxFunctions {
        NativeLtxvH264DemuxFunctions {
            av_find_best_stream: mock_demux_find_best_stream,
            avformat_alloc_context: mock_demux_format_alloc,
            avformat_close_input: mock_demux_format_close,
            avformat_free_context: mock_demux_format_free,
            avformat_open_input: mock_demux_format_open,
        }
    }

    fn mock_borrowed_input<'bytes, 'context>(
        bytes: &'bytes [u8],
        backend: &CpuBackend,
        context: &'context ExecutionContext<'_>,
    ) -> Result<NativeVideoCodecMemoryInput<'context, 'bytes>, NativeVideoCodecIoError> {
        let state = NativeVideoCodecInputState {
            bytes: NonNull::new(bytes.as_ptr().cast_mut())
                .ok_or(NativeVideoCodecIoError::InvalidBounds)?,
            byte_length: bytes.len(),
            position: 0,
            maximum_position: bytes.len(),
            cancellation: context.cancellation.clone(),
            failure: None,
            panic_on_next_callback: false,
        };
        let avio = allocate_native_video_codec_avio_inner(
            state,
            mock_avio_functions(),
            64,
            false,
            Some(native_video_codec_input_read),
            None,
            Some(native_video_codec_input_seek),
            backend,
            context,
            &mut || context.cancellation.check(),
        )?;
        Ok(NativeVideoCodecMemoryInput {
            avio,
            _owner: NativeVideoCodecInputOwner::Borrowed(bytes),
        })
    }

    #[test]
    fn retained_ltxv_mp4_demux_borrows_bytes_and_selects_first_h264_video_stream()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        reset_demux_fixture(0)?;
        take_avio_events()?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 64 * 1024)?;
        let decoder = NonNull::<abi::AvCodec>::dangling();
        DEMUX_DECODER.store(decoder.as_ptr(), Ordering::Release);
        let input = mock_borrowed_input(b"MP4", &backend, &context)?;
        let (format, stream_index) = open_first_ltxv_h264_stream_with_check(
            &input,
            decoder,
            4,
            &mock_demux_functions(),
            &mut || cancellation.check(),
        )?;
        assert_eq!(stream_index, 0);
        assert_eq!(
            *DEMUX_READ_BYTES
                .lock()
                .map_err(|_| "demux read mutex was poisoned")?,
            b"MP4"
        );
        drop(format);
        drop(input);
        assert_eq!(context.scratch.in_use_bytes(), 0);
        assert_eq!(
            *DEMUX_EVENTS
                .lock()
                .map_err(|_| "demux event mutex was poisoned")?,
            ["format_alloc", "format_open", "find_stream", "format_close"]
        );
        Ok(())
    }

    #[test]
    fn retained_ltxv_mp4_demux_rejects_stream_drift_and_cancellation_atomically()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 64 * 1024)?;
        let decoder = NonNull::<abi::AvCodec>::dangling();
        DEMUX_DECODER.store(decoder.as_ptr(), Ordering::Release);
        for (mode, expected) in [(1, "missing"), (2, "codec"), (3, "open")] {
            reset_demux_fixture(mode)?;
            let input = mock_borrowed_input(b"MP4", &backend, &context)?;
            let result = open_first_ltxv_h264_stream_with_check(
                &input,
                decoder,
                4,
                &mock_demux_functions(),
                &mut || cancellation.check(),
            );
            match expected {
                "missing" => assert!(matches!(
                    result,
                    Err(NativeVideoCodecLtxvDemuxError::MissingVideoStream)
                )),
                "codec" => assert!(matches!(
                    result,
                    Err(NativeVideoCodecLtxvDemuxError::UnexpectedVideoCodec)
                )),
                _ => assert!(matches!(
                    result,
                    Err(NativeVideoCodecLtxvDemuxError::OpenFailed { .. })
                )),
            }
            drop(input);
            assert_eq!(context.scratch.in_use_bytes(), 0);
        }

        reset_demux_fixture(4)?;
        let input = mock_borrowed_input(b"MP4", &backend, &context)?;
        assert!(matches!(
            open_first_ltxv_h264_stream_with_check(
                &input,
                decoder,
                1,
                &mock_demux_functions(),
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecLtxvDemuxError::StreamLimitExceeded)
        ));
        drop(input);

        let successful_checks = {
            reset_demux_fixture(0)?;
            let input = mock_borrowed_input(b"MP4", &backend, &context)?;
            let mut checks = 0;
            let (format, _) = open_first_ltxv_h264_stream_with_check(
                &input,
                decoder,
                4,
                &mock_demux_functions(),
                &mut || {
                    checks += 1;
                    Ok(())
                },
            )?;
            drop(format);
            drop(input);
            checks
        };
        for cancelled_check in 1..=successful_checks {
            reset_demux_fixture(0)?;
            let input = mock_borrowed_input(b"MP4", &backend, &context)?;
            let mut checks = 0;
            assert!(matches!(
                open_first_ltxv_h264_stream_with_check(
                    &input,
                    decoder,
                    4,
                    &mock_demux_functions(),
                    &mut || {
                        checks += 1;
                        if checks == cancelled_check {
                            Err(CancellationError)
                        } else {
                            Ok(())
                        }
                    },
                ),
                Err(NativeVideoCodecLtxvDemuxError::Cancelled)
            ));
            drop(input);
            assert_eq!(context.scratch.in_use_bytes(), 0);
        }
        Ok(())
    }

    static ENCODE_EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ENCODE_FAILURE_PHASE: Mutex<Option<&'static str>> = Mutex::new(None);
    static ENCODE_RECEIVE_STATE: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);
    static ENCODE_WRITABLE_CALLS: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);

    #[repr(C)]
    struct MockEncodeFormat {
        prefix: abi::AvFormatContext,
        opaque_stream_groups_through_data_codec_id: [u8; 136],
        metadata: *mut abi::AvDictionary,
        stream: *mut abi::AvStream,
        parameters: *mut u8,
    }

    #[repr(C)]
    struct MockEncodeFrame {
        prefix: abi::AvFrame,
        plane: Vec<u8>,
    }

    fn record_encode_event(event: impl Into<String>) {
        match ENCODE_EVENTS.lock() {
            Ok(mut events) => events.push(event.into()),
            Err(_) => std::process::abort(),
        }
    }

    fn reset_encode_fixture() -> Result<(), Box<dyn std::error::Error>> {
        ENCODE_EVENTS
            .lock()
            .map_err(|_| "encode event mutex was poisoned")?
            .clear();
        *ENCODE_FAILURE_PHASE
            .lock()
            .map_err(|_| "encode failure mutex was poisoned")? = None;
        ENCODE_RECEIVE_STATE.store(0, Ordering::Release);
        ENCODE_WRITABLE_CALLS.store(0, Ordering::Release);
        Ok(())
    }

    fn take_encode_events() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut events = ENCODE_EVENTS
            .lock()
            .map_err(|_| "encode event mutex was poisoned")?;
        Ok(std::mem::take(&mut *events))
    }

    fn fail_encode_phase(phase: &'static str) -> bool {
        match ENCODE_FAILURE_PHASE.lock() {
            Ok(mut failure) if *failure == Some(phase) => {
                *failure = None;
                true
            }
            Ok(_) => false,
            Err(_) => std::process::abort(),
        }
    }

    unsafe extern "C" fn mock_encode_format_alloc(
        destination: *mut *mut abi::AvFormatContext,
        _format: *const abi::AvOutputFormat,
        name: *const std::ffi::c_char,
        _filename: *const std::ffi::c_char,
    ) -> i32 {
        if destination.is_null() || name.is_null() || fail_encode_phase("format_alloc") {
            return abi::AV_ERROR_OUT_OF_MEMORY;
        }
        let name = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
        record_encode_event(format!("format_alloc:{name}"));
        let allocation = Box::new(MockEncodeFormat {
            prefix: abi::AvFormatContext {
                class: std::ptr::null(),
                input_format: std::ptr::null(),
                output_format: std::ptr::null(),
                private_data: std::ptr::null_mut(),
                io_context: std::ptr::null_mut(),
                context_flags: 0,
                stream_count: 0,
                streams: std::ptr::null_mut(),
            },
            opaque_stream_groups_through_data_codec_id: [0; 136],
            metadata: std::ptr::null_mut(),
            stream: std::ptr::null_mut(),
            parameters: std::ptr::null_mut(),
        });
        unsafe { *destination = Box::into_raw(allocation).cast() };
        0
    }

    unsafe extern "C" fn mock_encode_format_free(format: *mut abi::AvFormatContext) {
        if format.is_null() {
            return;
        }
        let allocation = unsafe { Box::from_raw(format.cast::<MockEncodeFormat>()) };
        if !allocation.metadata.is_null() {
            record_encode_event("format_metadata_free");
            drop(unsafe { Box::from_raw(allocation.metadata.cast::<u8>()) });
        }
        if !allocation.stream.is_null() {
            drop(unsafe { Box::from_raw(allocation.stream) });
        }
        if !allocation.parameters.is_null() {
            drop(unsafe { Box::from_raw(allocation.parameters) });
        }
        record_encode_event("format_free");
    }

    unsafe extern "C" fn mock_encode_new_stream(
        format: *mut abi::AvFormatContext,
        _encoder: *const abi::AvCodec,
    ) -> *mut abi::AvStream {
        record_encode_event("stream_alloc");
        if format.is_null() || fail_encode_phase("stream_alloc") {
            return std::ptr::null_mut();
        }
        let allocation = unsafe { &mut *format.cast::<MockEncodeFormat>() };
        allocation.parameters = Box::into_raw(Box::new(0_u8));
        allocation.stream = Box::into_raw(Box::new(abi::AvStream {
            class: std::ptr::null(),
            index: 0,
            identifier: 0,
            codec_parameters: allocation.parameters.cast(),
            private_data: std::ptr::null_mut(),
            time_base: ltxv_frame_time_base(),
        }));
        allocation.prefix.stream_count = 1;
        allocation.stream
    }

    unsafe extern "C" fn mock_encode_codec_alloc(
        _encoder: *const abi::AvCodec,
    ) -> *mut abi::AvCodecContext {
        record_encode_event("codec_alloc");
        if fail_encode_phase("codec_alloc") {
            return std::ptr::null_mut();
        }
        Box::into_raw(Box::new(0_u8)).cast()
    }

    unsafe extern "C" fn mock_encode_codec_free(pointer: *mut *mut abi::AvCodecContext) {
        record_encode_event("codec_free");
        if pointer.is_null() {
            return;
        }
        let value = unsafe { *pointer };
        if !value.is_null() {
            drop(unsafe { Box::from_raw(value.cast::<u8>()) });
            unsafe { *pointer = std::ptr::null_mut() };
        }
    }

    unsafe extern "C" fn mock_encode_option_set(
        _object: *mut c_void,
        name: *const std::ffi::c_char,
        value: *const std::ffi::c_char,
        _flags: i32,
    ) -> i32 {
        let name = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
        let value = unsafe { std::ffi::CStr::from_ptr(value) }.to_string_lossy();
        record_encode_event(format!("option:{name}={value}"));
        if fail_encode_phase("option") {
            abi::AV_ERROR_INVALID_ARGUMENT
        } else {
            0
        }
    }

    unsafe extern "C" fn mock_encode_option_set_int(
        _object: *mut c_void,
        name: *const std::ffi::c_char,
        value: i64,
        _flags: i32,
    ) -> i32 {
        let name = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
        record_encode_event(format!("option:{name}={value}"));
        0
    }

    unsafe extern "C" fn mock_encode_dict_set(
        dictionary: *mut *mut abi::AvDictionary,
        name: *const std::ffi::c_char,
        value: *const std::ffi::c_char,
        flags: i32,
    ) -> i32 {
        let name = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
        let value = unsafe { std::ffi::CStr::from_ptr(value) }.to_string_lossy();
        if name == "crf" || name == "preset" {
            record_encode_event(format!("dict:{name}={value}"));
        } else {
            record_encode_event(format!("metadata:{name}={value}:flags={flags}"));
        }
        if dictionary.is_null() {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        if name == "fail_oom" {
            return abi::AV_ERROR_OUT_OF_MEMORY;
        }
        if name == "fail_invalid" {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        if unsafe { (*dictionary).is_null() } {
            unsafe { *dictionary = Box::into_raw(Box::new(0_u8)).cast() };
        }
        0
    }

    unsafe extern "C" fn mock_encode_dict_free(dictionary: *mut *mut abi::AvDictionary) {
        record_encode_event("dict_free");
        if dictionary.is_null() {
            return;
        }
        let pointer = unsafe { *dictionary };
        if !pointer.is_null() {
            drop(unsafe { Box::from_raw(pointer.cast::<u8>()) });
            unsafe { *dictionary = std::ptr::null_mut() };
        }
    }

    unsafe extern "C" fn mock_encode_codec_open(
        _context: *mut abi::AvCodecContext,
        _codec: *const abi::AvCodec,
        options: *mut *mut abi::AvDictionary,
    ) -> i32 {
        record_encode_event("codec_open");
        if fail_encode_phase("codec_open") {
            return abi::AV_ERROR_OUT_OF_MEMORY;
        }
        unsafe { mock_encode_dict_free(options) };
        0
    }

    unsafe extern "C" fn mock_encode_parameters(
        _parameters: *mut abi::AvCodecParameters,
        _context: *const abi::AvCodecContext,
    ) -> i32 {
        record_encode_event("parameters");
        0
    }

    unsafe fn mock_encode_write(format: *mut abi::AvFormatContext, bytes: &[u8]) -> i32 {
        if format.is_null() {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        let avio = unsafe { (*format).io_context }.cast::<MockAvIoContext>();
        if avio.is_null() {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        match unsafe { (*avio).write_packet } {
            Some(write) => unsafe {
                write(
                    (*avio).opaque,
                    bytes.as_ptr(),
                    i32::try_from(bytes.len()).unwrap_or(i32::MAX),
                )
            },
            None => abi::AV_ERROR_INVALID_ARGUMENT,
        }
    }

    unsafe extern "C" fn mock_encode_header(
        format: *mut abi::AvFormatContext,
        _options: *mut *mut abi::AvDictionary,
    ) -> i32 {
        record_encode_event("header");
        unsafe { mock_encode_write(format, b"H") }.min(0)
    }

    unsafe extern "C" fn mock_encode_frame_alloc() -> *mut abi::AvFrame {
        record_encode_event("frame_alloc");
        if fail_encode_phase("frame_alloc") {
            return std::ptr::null_mut();
        }
        let allocation = Box::new(MockEncodeFrame {
            prefix: abi::AvFrame {
                data: [std::ptr::null_mut(); abi::AV_NUM_DATA_POINTERS],
                line_size: [0; abi::AV_NUM_DATA_POINTERS],
                extended_data: std::ptr::null_mut(),
                width: 0,
                height: 0,
                sample_count: 0,
                format: -1,
                key_frame: 0,
                picture_type: 0,
                sample_aspect_ratio: abi::AvRational {
                    numerator: 0,
                    denominator: 1,
                },
                presentation_timestamp: abi::AV_NO_PRESENTATION_TIMESTAMP,
            },
            plane: Vec::new(),
        });
        Box::into_raw(allocation).cast()
    }

    unsafe extern "C" fn mock_encode_frame_free(pointer: *mut *mut abi::AvFrame) {
        record_encode_event("frame_free");
        if pointer.is_null() {
            return;
        }
        let frame = unsafe { *pointer };
        if !frame.is_null() {
            drop(unsafe { Box::from_raw(frame.cast::<MockEncodeFrame>()) });
            unsafe { *pointer = std::ptr::null_mut() };
        }
    }

    unsafe extern "C" fn mock_encode_frame_buffer(frame: *mut abi::AvFrame, _align: i32) -> i32 {
        record_encode_event("frame_buffer");
        if frame.is_null() || fail_encode_phase("frame_buffer") {
            return abi::AV_ERROR_OUT_OF_MEMORY;
        }
        let allocation = unsafe { &mut *frame.cast::<MockEncodeFrame>() };
        let byte_count = usize::try_from(allocation.prefix.width)
            .ok()
            .and_then(|width| width.checked_mul(usize::try_from(allocation.prefix.height).ok()?))
            .and_then(|pixels| pixels.checked_mul(2))
            .unwrap_or(0);
        allocation.plane.resize(byte_count, 0);
        allocation.prefix.data[0] = allocation.plane.as_mut_ptr();
        allocation.prefix.line_size[0] = allocation.prefix.width;
        0
    }

    unsafe extern "C" fn mock_encode_frame_make_writable(frame: *mut abi::AvFrame) -> i32 {
        record_encode_event("frame_writable");
        let call = ENCODE_WRITABLE_CALLS.fetch_add(1, Ordering::AcqRel) + 1;
        if frame.is_null()
            || fail_encode_phase("frame_writable")
            || (call == 2 && fail_encode_phase("second_frame_writable"))
        {
            return abi::AV_ERROR_OUT_OF_MEMORY;
        }
        0
    }

    unsafe extern "C" fn mock_encode_packet_alloc() -> *mut abi::AvPacket {
        record_encode_event("packet_alloc");
        Box::into_raw(Box::new(abi::AvPacket {
            buffer: std::ptr::null_mut(),
            presentation_timestamp: abi::AV_NO_PRESENTATION_TIMESTAMP,
            decoding_timestamp: abi::AV_NO_PRESENTATION_TIMESTAMP,
            data: std::ptr::null_mut(),
            size: 0,
            stream_index: 0,
            flags: 0,
            side_data: std::ptr::null_mut(),
            side_data_count: 0,
            duration: 0,
        }))
    }

    unsafe extern "C" fn mock_encode_packet_unref(_packet: *mut abi::AvPacket) {
        record_encode_event("packet_unref");
    }

    unsafe extern "C" fn mock_encode_packet_free(pointer: *mut *mut abi::AvPacket) {
        record_encode_event("packet_free");
        if pointer.is_null() {
            return;
        }
        let packet = unsafe { *pointer };
        if !packet.is_null() {
            drop(unsafe { Box::from_raw(packet) });
            unsafe { *pointer = std::ptr::null_mut() };
        }
    }

    unsafe extern "C" fn mock_encode_sws_get(
        _source_width: i32,
        _source_height: i32,
        source_format: i32,
        _destination_width: i32,
        _destination_height: i32,
        destination_format: i32,
        _flags: i32,
        _source_filter: *mut abi::SwsFilter,
        _destination_filter: *mut abi::SwsFilter,
        _parameters: *const f64,
    ) -> *mut abi::SwsContext {
        record_encode_event(format!("sws_alloc:{source_format}->{destination_format}"));
        Box::into_raw(Box::new(0_u8)).cast()
    }

    unsafe extern "C" fn mock_encode_sws_free(pointer: *mut abi::SwsContext) {
        record_encode_event("sws_free");
        if !pointer.is_null() {
            drop(unsafe { Box::from_raw(pointer.cast::<u8>()) });
        }
    }

    unsafe extern "C" fn mock_encode_sws_scale(
        _context: *mut abi::SwsContext,
        source: *const *const u8,
        source_stride: *const i32,
        _source_slice_y: i32,
        source_slice_height: i32,
        destination: *const *mut u8,
        _destination_stride: *const i32,
    ) -> i32 {
        record_encode_event("sws_scale");
        if source.is_null() || destination.is_null() {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        if !source_stride.is_null() {
            let stride = unsafe { *source_stride };
            if stride > 0 && source_slice_height > 0 {
                let byte_count = usize::try_from(stride)
                    .ok()
                    .and_then(|stride| {
                        stride.checked_mul(usize::try_from(source_slice_height).ok()?)
                    })
                    .unwrap_or(0);
                let data = unsafe { *source };
                if !data.is_null() && byte_count > 0 {
                    record_encode_event(format!("rgb:{:?}", unsafe {
                        std::slice::from_raw_parts(data, byte_count)
                    }));
                }
            }
        }
        source_slice_height
    }

    unsafe extern "C" fn mock_encode_send_frame(
        _context: *mut abi::AvCodecContext,
        frame: *const abi::AvFrame,
    ) -> i32 {
        if frame.is_null() {
            record_encode_event("send_flush");
            ENCODE_RECEIVE_STATE.store(3, Ordering::Release);
        } else {
            record_encode_event("send_frame");
            record_encode_event(format!("frame_pts:{}", unsafe {
                (*frame).presentation_timestamp
            }));
            ENCODE_RECEIVE_STATE.store(1, Ordering::Release);
        }
        0
    }

    unsafe extern "C" fn mock_encode_receive_packet(
        _context: *mut abi::AvCodecContext,
        packet: *mut abi::AvPacket,
    ) -> i32 {
        record_encode_event("receive_packet");
        match ENCODE_RECEIVE_STATE.load(Ordering::Acquire) {
            1 => {
                ENCODE_RECEIVE_STATE.store(2, Ordering::Release);
                if !packet.is_null() {
                    unsafe {
                        (*packet).presentation_timestamp = 0;
                        (*packet).decoding_timestamp = 0;
                        (*packet).duration = 1;
                        (*packet).size = 1;
                    }
                }
                0
            }
            2 => abi::AV_ERROR_TRY_AGAIN,
            3 => abi::AV_ERROR_END_OF_FILE,
            _ => abi::AV_ERROR_INVALID_ARGUMENT,
        }
    }

    unsafe extern "C" fn mock_encode_rescale(
        value: i64,
        source: abi::AvRational,
        destination: abi::AvRational,
    ) -> i64 {
        record_encode_event(format!(
            "rescale:{}/{}->{}/{}",
            source.numerator, source.denominator, destination.numerator, destination.denominator
        ));
        value
    }

    unsafe extern "C" fn mock_encode_mux(
        format: *mut abi::AvFormatContext,
        _packet: *mut abi::AvPacket,
    ) -> i32 {
        record_encode_event("mux");
        unsafe { mock_encode_write(format, b"P") }.min(0)
    }

    unsafe extern "C" fn mock_encode_trailer(format: *mut abi::AvFormatContext) -> i32 {
        record_encode_event("trailer");
        unsafe { mock_encode_write(format, b"T") }.min(0)
    }

    fn mock_encode_functions() -> NativeLtxvH264EncodeFunctions {
        NativeLtxvH264EncodeFunctions {
            av_packet_alloc: mock_encode_packet_alloc,
            av_packet_free: mock_encode_packet_free,
            av_packet_unref: mock_encode_packet_unref,
            avcodec_alloc_context3: mock_encode_codec_alloc,
            avcodec_free_context: mock_encode_codec_free,
            avcodec_open2: mock_encode_codec_open,
            avcodec_parameters_from_context: mock_encode_parameters,
            avcodec_receive_packet: mock_encode_receive_packet,
            avcodec_send_frame: mock_encode_send_frame,
            avformat_alloc_output_context2: mock_encode_format_alloc,
            avformat_free_context: mock_encode_format_free,
            avformat_new_stream: mock_encode_new_stream,
            avformat_write_header: mock_encode_header,
            av_interleaved_write_frame: mock_encode_mux,
            av_write_trailer: mock_encode_trailer,
            av_dict_free: mock_encode_dict_free,
            av_dict_set: mock_encode_dict_set,
            av_frame_alloc: mock_encode_frame_alloc,
            av_frame_free: mock_encode_frame_free,
            av_frame_get_buffer: mock_encode_frame_buffer,
            av_frame_make_writable: mock_encode_frame_make_writable,
            av_opt_set: mock_encode_option_set,
            av_opt_set_int: mock_encode_option_set_int,
            av_rescale_q: mock_encode_rescale,
            sws_free_context: mock_encode_sws_free,
            sws_get_context: mock_encode_sws_get,
            sws_scale: mock_encode_sws_scale,
        }
    }

    fn mock_encode_output<'context>(
        backend: &CpuBackend,
        context: &'context ExecutionContext<'_>,
        maximum_bytes: usize,
    ) -> Result<NativeVideoCodecMemoryOutput<'context>, NativeVideoCodecIoError> {
        let state = NativeVideoCodecOutputState {
            bytes: backend.workspace_vec(context, maximum_bytes)?,
            position: 0,
            maximum_bytes,
            cancellation: context.cancellation.clone(),
            failure: None,
            panic_on_next_callback: false,
        };
        let avio = allocate_native_video_codec_avio_inner(
            state,
            mock_avio_functions(),
            64,
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

    #[test]
    fn retained_ltxv_h264_mp4_encode_uses_exact_options_packets_and_cleanup()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        reset_encode_fixture()?;
        take_avio_events()?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 64 * 1024)?;
        let mut output = mock_encode_output(&backend, &context, 64)?;
        let encoder = NonNull::new(Box::into_raw(Box::new(0_u8)).cast::<abi::AvCodec>())
            .ok_or("mock encoder allocation failed")?;
        encode_ltxv_h264_rgb8_with_check(
            encoder,
            &[1_u8; 12],
            2,
            2,
            35,
            8,
            &mock_encode_functions(),
            &mut output,
            &mut || cancellation.check(),
        )?;
        assert_eq!(output.staged_bytes()?, b"HPT");
        drop(output);
        drop(unsafe { Box::from_raw(encoder.as_ptr().cast::<u8>()) });
        assert_eq!(context.scratch.in_use_bytes(), 0);
        let events = take_encode_events()?;
        assert!(events.windows(3).any(|events| {
            events
                == [
                    "option:video_size=2x2",
                    "option:pixel_format=yuv420p",
                    "option:time_base=1/1",
                ]
        }));
        assert!(
            events
                .windows(2)
                .any(|events| { events == ["dict:crf=35", "dict:preset=veryfast"] })
        );
        assert!(events.windows(11).any(|events| {
            events
                == [
                    "send_frame",
                    "frame_pts:0",
                    "receive_packet",
                    "rescale:1/1->1/1",
                    "rescale:1/1->1/1",
                    "rescale:1/1->1/1",
                    "mux",
                    "packet_unref",
                    "receive_packet",
                    "send_flush",
                    "receive_packet",
                ]
        }));
        assert!(events.ends_with(&[
            "trailer".to_owned(),
            "sws_free".to_owned(),
            "packet_free".to_owned(),
            "frame_free".to_owned(),
            "dict_free".to_owned(),
            "codec_free".to_owned(),
            "format_free".to_owned(),
        ]));
        Ok(())
    }

    #[test]
    fn retained_ltxv_h264_mp4_encode_failure_and_cancellation_are_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 64 * 1024)?;
        reset_encode_fixture()?;
        *ENCODE_FAILURE_PHASE
            .lock()
            .map_err(|_| "encode failure mutex was poisoned")? = Some("codec_open");
        let mut output = mock_encode_output(&backend, &context, 64)?;
        let encoder = NonNull::new(Box::into_raw(Box::new(0_u8)).cast::<abi::AvCodec>())
            .ok_or("mock encoder allocation failed")?;
        assert!(matches!(
            encode_ltxv_h264_rgb8_with_check(
                encoder,
                &[1_u8; 12],
                2,
                2,
                35,
                8,
                &mock_encode_functions(),
                &mut output,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecLtxvEncodeError::ResourceExhausted {
                phase: "open libx264 encoder"
            })
        ));
        drop(output);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        for invalid_limits in [(0, 64, 1, 1), (64, 0, 1, 1), (64, 64, 0, 1), (64, 64, 1, 0)] {
            assert!(matches!(
                NativeLtxvH264EncodeLimits::checked(
                    invalid_limits.0,
                    invalid_limits.1,
                    invalid_limits.2,
                    invalid_limits.3,
                ),
                Err(NativeVideoCodecLtxvEncodeError::InvalidLimits)
            ));
        }

        let descriptor = TensorDescriptor::contiguous(
            vec![2, 2, 3],
            DType::U8,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let (tensor, _) = backend.upload_bytes(descriptor, &[1_u8; 12], &context)?;
        let image = Rgb8ImageTensor::from_tensor(tensor)?;
        let (bytes, width, height) = validate_ltxv_h264_encode_input(&image, 35)?;
        assert_eq!((bytes.len(), width, height), (12, 2, 2));
        assert!(matches!(
            validate_ltxv_h264_encode_input(&image, 0),
            Err(NativeVideoCodecLtxvEncodeError::InvalidCrf)
        ));
        assert!(matches!(
            validate_ltxv_h264_encode_input(&image, 101),
            Err(NativeVideoCodecLtxvEncodeError::InvalidCrf)
        ));
        let odd_descriptor = TensorDescriptor::contiguous(
            vec![3, 2, 3],
            DType::U8,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let (odd_tensor, _) = backend.upload_bytes(odd_descriptor, &[1_u8; 18], &context)?;
        let odd_image = Rgb8ImageTensor::from_tensor(odd_tensor)?;
        assert!(matches!(
            validate_ltxv_h264_encode_input(&odd_image, 35),
            Err(NativeVideoCodecLtxvEncodeError::InvalidInput)
        ));

        reset_encode_fixture()?;
        let mut limited_output = mock_encode_output(&backend, &context, 2)?;
        assert!(matches!(
            encode_ltxv_h264_rgb8_with_check(
                encoder,
                &[1_u8; 12],
                2,
                2,
                35,
                8,
                &mock_encode_functions(),
                &mut limited_output,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecLtxvEncodeError::Io(
                NativeVideoCodecIoError::OutputLimitExceeded
            ))
        ));
        drop(limited_output);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        reset_encode_fixture()?;
        let mut iteration_limited_output = mock_encode_output(&backend, &context, 64)?;
        assert!(matches!(
            encode_ltxv_h264_rgb8_with_check(
                encoder,
                &[1_u8; 12],
                2,
                2,
                35,
                1,
                &mock_encode_functions(),
                &mut iteration_limited_output,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecLtxvEncodeError::PacketIterationLimit)
        ));
        drop(iteration_limited_output);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        reset_encode_fixture()?;
        let mut successful_output = mock_encode_output(&backend, &context, 64)?;
        let mut successful_checks = 0;
        encode_ltxv_h264_rgb8_with_check(
            encoder,
            &[1_u8; 12],
            2,
            2,
            35,
            8,
            &mock_encode_functions(),
            &mut successful_output,
            &mut || {
                successful_checks += 1;
                Ok(())
            },
        )?;
        drop(successful_output);

        for cancellation_check in 1..=successful_checks {
            reset_encode_fixture()?;
            let mut output = mock_encode_output(&backend, &context, 64)?;
            let mut checks = 0;
            assert!(matches!(
                encode_ltxv_h264_rgb8_with_check(
                    encoder,
                    &[1_u8; 12],
                    2,
                    2,
                    35,
                    8,
                    &mock_encode_functions(),
                    &mut output,
                    &mut || {
                        checks += 1;
                        if checks == cancellation_check {
                            Err(CancellationError)
                        } else {
                            Ok(())
                        }
                    },
                ),
                Err(NativeVideoCodecLtxvEncodeError::Cancelled)
            ));
            drop(output);
            assert_eq!(context.scratch.in_use_bytes(), 0);
        }
        drop(unsafe { Box::from_raw(encoder.as_ptr().cast::<u8>()) });
        Ok(())
    }

    #[test]
    fn retained_vp9_webm_container_metadata_is_bounded_ordered_and_preheader()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        let limits = NativeVideoContainerMetadataLimits::checked(3, 16, 32, 64)?;
        let metadata = NativeVideoContainerMetadata::checked(
            vec![
                ("prompt".to_owned(), "first".to_owned()),
                ("workflow".to_owned(), "{}".to_owned()),
                ("prompt".to_owned(), "last".to_owned()),
            ],
            limits,
        )?;
        assert_eq!(metadata.entries().len(), 3);
        assert!(matches!(
            NativeVideoContainerMetadataLimits::checked(0, 1, 1, 1),
            Err(NativeVideoContainerMetadataError::InvalidLimits)
        ));
        for (entries, expected) in [
            (
                vec![(String::new(), String::new())],
                NativeVideoContainerMetadataError::EmptyKey,
            ),
            (
                vec![("embedded\0key".to_owned(), String::new())],
                NativeVideoContainerMetadataError::EmbeddedNul,
            ),
            (
                vec![("key".to_owned(), "embedded\0value".to_owned())],
                NativeVideoContainerMetadataError::EmbeddedNul,
            ),
            (
                vec![("key-that-is-too-long".to_owned(), String::new())],
                NativeVideoContainerMetadataError::LimitExceeded,
            ),
            (
                vec![(
                    "key".to_owned(),
                    "value-that-is-far-too-long-for-the-limit".to_owned(),
                )],
                NativeVideoContainerMetadataError::LimitExceeded,
            ),
        ] {
            assert_eq!(
                NativeVideoContainerMetadata::checked(entries, limits),
                Err(expected)
            );
        }
        assert!(matches!(
            NativeVideoContainerMetadata::checked(
                vec![
                    ("a".to_owned(), String::new()),
                    ("b".to_owned(), String::new()),
                    ("c".to_owned(), String::new()),
                    ("d".to_owned(), String::new()),
                ],
                limits,
            ),
            Err(NativeVideoContainerMetadataError::LimitExceeded)
        ));
        assert!(matches!(
            NativeVideoContainerMetadata::checked(
                vec![("prompt".to_owned(), "first".to_owned())],
                NativeVideoContainerMetadataLimits::checked(1, 16, 32, 4)?,
            ),
            Err(NativeVideoContainerMetadataError::LimitExceeded)
        ));

        reset_encode_fixture()?;
        take_avio_events()?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 64 * 1024)?;
        let mut output = mock_encode_output(&backend, &context, 64)?;
        let encoder = NonNull::new(Box::into_raw(Box::new(0_u8)).cast::<abi::AvCodec>())
            .ok_or("mock encoder allocation failed")?;
        let mut provide_frame = |_frame_index: usize,
                                 consume: &mut dyn FnMut(
            &[u8],
        ) -> Result<
            (),
            NativeVideoCodecLtxvEncodeError,
        >| { consume(&[1_u8; 12]) };
        encode_rgb8_frames_with_metadata_check(
            NativeRgb8EncodeProfile::vp9_webm(
                checked_vp9_frame_rate((2_997, 125))?,
                NativeVideoCrf::checked(31.5)?,
            ),
            encoder,
            1,
            2,
            2,
            8,
            &mock_encode_functions(),
            &mut output,
            &metadata,
            &mut provide_frame,
            &mut || cancellation.check(),
        )?;
        assert_eq!(output.staged_bytes()?, b"HPT");
        drop(output);
        drop(unsafe { Box::from_raw(encoder.as_ptr().cast::<u8>()) });
        assert_eq!(context.scratch.in_use_bytes(), 0);
        let events = take_encode_events()?;
        let metadata_events = events
            .iter()
            .filter(|event| event.starts_with("metadata:"))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            metadata_events,
            [
                "metadata:prompt=first:flags=0",
                "metadata:workflow={}:flags=0",
                "metadata:prompt=last:flags=0",
            ]
        );
        let last_metadata = events
            .iter()
            .rposition(|event| event.starts_with("metadata:"))
            .ok_or("metadata event missing")?;
        let stream = events
            .iter()
            .position(|event| event == "stream_alloc")
            .ok_or("stream allocation event missing")?;
        let header = events
            .iter()
            .position(|event| event == "header")
            .ok_or("header event missing")?;
        assert!(last_metadata < stream && stream < header);
        assert_eq!(
            events
                .iter()
                .filter(|event| event.as_str() == "format_metadata_free")
                .count(),
            1
        );
        assert_eq!(events.last().map(String::as_str), Some("format_free"));
        Ok(())
    }

    #[test]
    fn retained_vp9_webm_container_metadata_failure_cancellation_and_retry_are_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        let limits = NativeVideoContainerMetadataLimits::checked(3, 32, 32, 96)?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 64 * 1024)?;
        let encoder = NonNull::new(Box::into_raw(Box::new(0_u8)).cast::<abi::AvCodec>())
            .ok_or("mock encoder allocation failed")?;
        let profile = NativeRgb8EncodeProfile::vp9_webm(
            checked_vp9_frame_rate((2_997, 125))?,
            NativeVideoCrf::checked(31.5)?,
        );

        for key in ["fail_oom", "fail_invalid"] {
            reset_encode_fixture()?;
            let metadata = NativeVideoContainerMetadata::checked(
                vec![
                    ("prompt".to_owned(), "first".to_owned()),
                    (key.to_owned(), "value".to_owned()),
                ],
                limits,
            )?;
            let mut output = mock_encode_output(&backend, &context, 64)?;
            let mut provide_frame = |_frame_index: usize,
                                     consume: &mut dyn FnMut(
                &[u8],
            ) -> Result<
                (),
                NativeVideoCodecLtxvEncodeError,
            >| { consume(&[1_u8; 12]) };
            let result = encode_rgb8_frames_with_metadata_check(
                profile,
                encoder,
                1,
                2,
                2,
                8,
                &mock_encode_functions(),
                &mut output,
                &metadata,
                &mut provide_frame,
                &mut || cancellation.check(),
            );
            if key == "fail_oom" {
                assert!(matches!(
                    result,
                    Err(NativeVideoCodecLtxvEncodeError::ResourceExhausted {
                        phase: "set WebM container metadata"
                    })
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(NativeVideoCodecLtxvEncodeError::NativeCall {
                        phase: "set WebM container metadata",
                        status: abi::AV_ERROR_INVALID_ARGUMENT
                    })
                ));
            }
            drop(output);
            assert_eq!(context.scratch.in_use_bytes(), 0);
            let events = take_encode_events()?;
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.as_str() == "format_metadata_free")
                    .count(),
                1
            );
            assert_eq!(events.last().map(String::as_str), Some("format_free"));
        }

        let metadata = NativeVideoContainerMetadata::checked(
            vec![("prompt".to_owned(), "value".to_owned())],
            limits,
        )?;
        reset_encode_fixture()?;
        let mut successful_output = mock_encode_output(&backend, &context, 64)?;
        let mut successful_checks = 0;
        let mut provide_frame = |_frame_index: usize,
                                 consume: &mut dyn FnMut(
            &[u8],
        ) -> Result<
            (),
            NativeVideoCodecLtxvEncodeError,
        >| { consume(&[1_u8; 12]) };
        encode_rgb8_frames_with_metadata_check(
            profile,
            encoder,
            1,
            2,
            2,
            8,
            &mock_encode_functions(),
            &mut successful_output,
            &metadata,
            &mut provide_frame,
            &mut || {
                successful_checks += 1;
                Ok(())
            },
        )?;
        drop(successful_output);

        for cancelled_check in 1..=successful_checks {
            reset_encode_fixture()?;
            let mut output = mock_encode_output(&backend, &context, 64)?;
            let mut checks = 0;
            let mut provide_frame = |_frame_index: usize,
                                     consume: &mut dyn FnMut(
                &[u8],
            ) -> Result<
                (),
                NativeVideoCodecLtxvEncodeError,
            >| { consume(&[1_u8; 12]) };
            assert!(matches!(
                encode_rgb8_frames_with_metadata_check(
                    profile,
                    encoder,
                    1,
                    2,
                    2,
                    8,
                    &mock_encode_functions(),
                    &mut output,
                    &metadata,
                    &mut provide_frame,
                    &mut || {
                        checks += 1;
                        if checks == cancelled_check {
                            Err(CancellationError)
                        } else {
                            Ok(())
                        }
                    },
                ),
                Err(NativeVideoCodecLtxvEncodeError::Cancelled)
            ));
            drop(output);
            assert_eq!(context.scratch.in_use_bytes(), 0);
        }

        reset_encode_fixture()?;
        let mut retry_output = mock_encode_output(&backend, &context, 64)?;
        let mut provide_frame = |_frame_index: usize,
                                 consume: &mut dyn FnMut(
            &[u8],
        ) -> Result<
            (),
            NativeVideoCodecLtxvEncodeError,
        >| { consume(&[1_u8; 12]) };
        encode_rgb8_frames_with_metadata_check(
            profile,
            encoder,
            1,
            2,
            2,
            8,
            &mock_encode_functions(),
            &mut retry_output,
            &metadata,
            &mut provide_frame,
            &mut || cancellation.check(),
        )?;
        assert_eq!(retry_output.staged_bytes()?, b"HPT");
        drop(retry_output);
        drop(unsafe { Box::from_raw(encoder.as_ptr().cast::<u8>()) });
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn retained_vp9_webm_crf_formats_source_float_without_integer_narrowing()
    -> Result<(), Box<dyn std::error::Error>> {
        for (value, expected) in [
            (-0.0, "-0.0"),
            (0.0, "0.0"),
            (31.5, "31.5"),
            (32.0, "32.0"),
            (0.000_01, "1e-05"),
            (0.000_000_1, "1e-07"),
            (63.0, "63.0"),
        ] {
            let crf = NativeVideoCrf::checked(value)?;
            assert_eq!(crf.bits(), value.to_bits());
            let mut buffer = [0_u8; 32];
            write_python_float(crf.value(), &mut buffer)?;
            let rendered = std::ffi::CStr::from_bytes_until_nul(&buffer)?.to_str()?;
            assert_eq!(rendered, expected);
        }
        Ok(())
    }

    #[test]
    fn retained_vp9_webm_encode_uses_exact_profile_rate_packets_and_cleanup()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        reset_encode_fixture()?;
        take_avio_events()?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 64 * 1024)?;
        let mut output = mock_encode_output(&backend, &context, 64)?;
        let encoder = NonNull::new(Box::into_raw(Box::new(0_u8)).cast::<abi::AvCodec>())
            .ok_or("mock encoder allocation failed")?;
        let frame_rate = checked_vp9_frame_rate((2_997, 125))?;
        encode_rgb8_frame_with_check(
            NativeRgb8EncodeProfile::vp9_webm(frame_rate, NativeVideoCrf::checked(31.5)?),
            encoder,
            &[1_u8; 12],
            2,
            2,
            8,
            &mock_encode_functions(),
            &mut output,
            &mut || cancellation.check(),
        )?;
        assert_eq!(output.staged_bytes()?, b"HPT");
        drop(output);
        drop(unsafe { Box::from_raw(encoder.as_ptr().cast::<u8>()) });
        assert_eq!(context.scratch.in_use_bytes(), 0);
        let events = take_encode_events()?;
        assert!(events.iter().any(|event| event == "format_alloc:webm"));
        assert!(events.windows(5).any(|events| {
            events
                == [
                    "option:pixel_format=yuv420p",
                    "option:time_base=125/2997",
                    "option:framerate=2997/125",
                    "option:threads=1",
                    "option:b=0",
                ]
        }));
        assert!(events.iter().any(|event| event == "dict:crf=31.5"));
        assert!(!events.iter().any(|event| event.starts_with("dict:preset=")));
        assert!(
            !events
                .iter()
                .any(|event| event.starts_with("option:flags="))
        );
        assert!(
            events
                .iter()
                .any(|event| event == "rescale:125/2997->125/2997")
        );
        assert!(events.windows(8).any(|events| {
            events
                == [
                    "send_frame",
                    "frame_pts:0",
                    "receive_packet",
                    "rescale:125/2997->125/2997",
                    "rescale:125/2997->125/2997",
                    "rescale:125/2997->125/2997",
                    "mux",
                    "packet_unref",
                ]
        }));
        assert!(events.ends_with(&[
            "trailer".to_owned(),
            "sws_free".to_owned(),
            "packet_free".to_owned(),
            "frame_free".to_owned(),
            "dict_free".to_owned(),
            "codec_free".to_owned(),
            "format_free".to_owned(),
        ]));
        Ok(())
    }

    #[test]
    fn retained_vp9_webm_encode_validates_bounds_cancellation_and_retry()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 64 * 1024)?;
        for invalid_limits in [(0, 64, 1, 1), (64, 0, 1, 1), (64, 64, 0, 1), (64, 64, 1, 0)] {
            assert!(matches!(
                NativeVp9WebmEncodeLimits::checked(
                    invalid_limits.0,
                    invalid_limits.1,
                    invalid_limits.2,
                    invalid_limits.3,
                ),
                Err(NativeVideoCodecVp9EncodeError::InvalidLimits)
            ));
        }
        assert!(matches!(
            checked_vp9_frame_rate((0, 1)),
            Err(NativeVideoCodecVp9EncodeError::InvalidFrameRate)
        ));
        assert!(matches!(
            checked_vp9_frame_rate((2, 2)),
            Err(NativeVideoCodecVp9EncodeError::InvalidFrameRate)
        ));
        assert!(matches!(
            checked_vp9_frame_rate((u64::MAX, 1)),
            Err(NativeVideoCodecVp9EncodeError::InvalidFrameRate)
        ));

        let descriptor = TensorDescriptor::contiguous(
            vec![3, 5, 3],
            DType::U8,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let (tensor, _) = backend.upload_bytes(descriptor, &[1_u8; 45], &context)?;
        let image = Rgb8ImageTensor::from_tensor(tensor)?;
        let (bytes, width, height) = validate_vp9_rgb8_encode_input(&image)?;
        assert_eq!((bytes.len(), width, height), (45, 5, 3));
        let encoder = NonNull::new(Box::into_raw(Box::new(0_u8)).cast::<abi::AvCodec>())
            .ok_or("mock encoder allocation failed")?;
        let profile = NativeRgb8EncodeProfile::vp9_webm(
            checked_vp9_frame_rate((2_997, 125))?,
            NativeVideoCrf::checked(63.0)?,
        );
        reset_encode_fixture()?;
        *ENCODE_FAILURE_PHASE
            .lock()
            .map_err(|_| "encode failure mutex was poisoned")? = Some("codec_open");
        let mut failed_output = mock_encode_output(&backend, &context, 64)?;
        assert!(matches!(
            encode_rgb8_frame_with_check(
                profile,
                encoder,
                bytes,
                width,
                height,
                8,
                &mock_encode_functions(),
                &mut failed_output,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecLtxvEncodeError::ResourceExhausted {
                phase: "open libvpx-vp9 encoder"
            })
        ));
        drop(failed_output);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        reset_encode_fixture()?;
        let mut limited_output = mock_encode_output(&backend, &context, 2)?;
        assert!(matches!(
            encode_rgb8_frame_with_check(
                profile,
                encoder,
                bytes,
                width,
                height,
                8,
                &mock_encode_functions(),
                &mut limited_output,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecLtxvEncodeError::Io(
                NativeVideoCodecIoError::OutputLimitExceeded
            ))
        ));
        drop(limited_output);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        reset_encode_fixture()?;
        let mut iteration_limited_output = mock_encode_output(&backend, &context, 64)?;
        assert!(matches!(
            encode_rgb8_frame_with_check(
                profile,
                encoder,
                bytes,
                width,
                height,
                1,
                &mock_encode_functions(),
                &mut iteration_limited_output,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecLtxvEncodeError::PacketIterationLimit)
        ));
        drop(iteration_limited_output);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        reset_encode_fixture()?;
        let mut successful_output = mock_encode_output(&backend, &context, 64)?;
        let mut successful_checks = 0;
        encode_rgb8_frame_with_check(
            profile,
            encoder,
            bytes,
            width,
            height,
            8,
            &mock_encode_functions(),
            &mut successful_output,
            &mut || {
                successful_checks += 1;
                Ok(())
            },
        )?;
        drop(successful_output);
        for cancelled_check in 1..=successful_checks {
            reset_encode_fixture()?;
            let mut output = mock_encode_output(&backend, &context, 64)?;
            let mut checks = 0;
            assert!(matches!(
                encode_rgb8_frame_with_check(
                    profile,
                    encoder,
                    bytes,
                    width,
                    height,
                    8,
                    &mock_encode_functions(),
                    &mut output,
                    &mut || {
                        checks += 1;
                        if checks == cancelled_check {
                            Err(CancellationError)
                        } else {
                            Ok(())
                        }
                    },
                ),
                Err(NativeVideoCodecLtxvEncodeError::Cancelled)
            ));
            drop(output);
            assert_eq!(context.scratch.in_use_bytes(), 0);
        }
        reset_encode_fixture()?;
        let mut retry_output = mock_encode_output(&backend, &context, 64)?;
        encode_rgb8_frame_with_check(
            profile,
            encoder,
            bytes,
            width,
            height,
            8,
            &mock_encode_functions(),
            &mut retry_output,
            &mut || cancellation.check(),
        )?;
        assert_eq!(retry_output.staged_bytes()?, b"HPT");
        drop(retry_output);
        drop(unsafe { Box::from_raw(encoder.as_ptr().cast::<u8>()) });
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn retained_vp9_webm_batch_encode_reuses_one_session_and_preserves_order()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        reset_encode_fixture()?;
        take_avio_events()?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 256 * 1024)?;
        let mut values = Vec::new();
        for _pixel in 0..4 {
            values.extend_from_slice(&[-1.0, 0.5, 2.0]);
        }
        for _pixel in 0..4 {
            values.extend_from_slice(&[0.25, 0.75, 1.0]);
        }
        for _pixel in 0..4 {
            values.extend_from_slice(&[1.0, 0.0, 0.1]);
        }
        let images = ImageTensor::from_f32(&backend, &context, 3, 2, 2, 3, &values)?;
        let session = NativeVp9WebmEncodeLimits::checked(64, 64, 1, 16)?;
        let limits = NativeVp9WebmBatchLimits::checked(session, 3, 4)?;
        let (frame_count, width, height, channels) =
            validate_vp9_image_batch(&images, limits, &context)?;
        assert_eq!(channels, 3);
        let mut output = mock_encode_output(&backend, &context, 64)?;
        let encoder = NonNull::new(Box::into_raw(Box::new(0_u8)).cast::<abi::AvCodec>())
            .ok_or("mock encoder allocation failed")?;
        let mut provide_frame = |frame_index: usize,
                                 consume: &mut dyn FnMut(
            &[u8],
        ) -> Result<
            (),
            NativeVideoCodecLtxvEncodeError,
        >| {
            let frame = source_compatible_vp9_rgb8_frame(&images, frame_index, &backend, &context)?;
            consume(frame.as_u8_slice().map_err(map_vp9_staging_tensor_error)?)
        };
        encode_rgb8_frames_with_check(
            NativeRgb8EncodeProfile::vp9_webm(
                checked_vp9_frame_rate((2_997, 125))?,
                NativeVideoCrf::checked(32.0)?,
            ),
            encoder,
            frame_count,
            width,
            height,
            session.maximum_packet_iterations,
            &mock_encode_functions(),
            &mut output,
            &mut provide_frame,
            &mut || cancellation.check(),
        )?;
        assert_eq!(output.staged_bytes()?, b"HPPPT");
        drop(output);
        drop(unsafe { Box::from_raw(encoder.as_ptr().cast::<u8>()) });
        assert_eq!(context.scratch.in_use_bytes(), 0);
        let events = take_encode_events()?;
        assert_eq!(
            events
                .iter()
                .filter(|event| event.as_str() == "format_alloc:webm")
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.as_str() == "frame_writable")
                .count(),
            3
        );
        for timestamp in 0..3 {
            assert!(
                events
                    .iter()
                    .any(|event| event == &format!("frame_pts:{timestamp}"))
            );
        }
        for expected in [
            "rgb:[0, 127, 255, 0, 127, 255, 0, 127, 255, 0, 127, 255]",
            "rgb:[63, 191, 255, 63, 191, 255, 63, 191, 255, 63, 191, 255]",
            "rgb:[255, 0, 25, 255, 0, 25, 255, 0, 25, 255, 0, 25]",
        ] {
            assert!(events.iter().any(|event| event == expected));
        }
        assert_eq!(
            events
                .iter()
                .filter(|event| event.as_str() == "send_flush")
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.as_str() == "trailer")
                .count(),
            1
        );
        Ok(())
    }

    #[test]
    fn retained_vp9_webm_alpha_batch_preserves_rgba_profile_and_order()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        reset_encode_fixture()?;
        take_avio_events()?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 256 * 1024)?;
        let mut values = Vec::new();
        for _pixel in 0..4 {
            values.extend_from_slice(&[-1.0, 0.0, 0.5, 2.0]);
        }
        for _pixel in 0..4 {
            values.extend_from_slice(&[1.0, 0.25, 0.75, 0.0]);
        }
        let images = ImageTensor::from_f32(&backend, &context, 2, 2, 2, 4, &values)?;
        let session = NativeVp9WebmEncodeLimits::checked(64, 64, 1, 16)?;
        let limits = NativeVp9WebmBatchLimits::checked(session, 2, 4)?;
        let (frame_count, width, height, channels) =
            validate_vp9_image_batch(&images, limits, &context)?;
        assert_eq!(channels, 4);
        let metadata = NativeVideoContainerMetadata::checked(
            vec![("prompt".to_owned(), "alpha".to_owned())],
            NativeVideoContainerMetadataLimits::checked(1, 16, 16, 32)?,
        )?;
        let mut output = mock_encode_output(&backend, &context, 64)?;
        let encoder = NonNull::new(Box::into_raw(Box::new(0_u8)).cast::<abi::AvCodec>())
            .ok_or("mock encoder allocation failed")?;
        let mut provide_frame = |frame_index: usize,
                                 consume: &mut dyn FnMut(
            &[u8],
        ) -> Result<
            (),
            NativeVideoCodecLtxvEncodeError,
        >| {
            let frame =
                source_compatible_vp9_rgba8_frame(&images, frame_index, &backend, &context)?;
            consume(&frame)
        };
        encode_rgb8_frames_with_metadata_check(
            NativeRgb8EncodeProfile::vp9_webm_alpha(
                checked_vp9_frame_rate((2_997, 125))?,
                NativeVideoCrf::checked(31.5)?,
            ),
            encoder,
            frame_count,
            width,
            height,
            session.maximum_packet_iterations,
            &mock_encode_functions(),
            &mut output,
            &metadata,
            &mut provide_frame,
            &mut || cancellation.check(),
        )?;
        assert_eq!(output.staged_bytes()?, b"HPPT");
        drop(output);
        drop(unsafe { Box::from_raw(encoder.as_ptr().cast::<u8>()) });
        assert_eq!(context.scratch.in_use_bytes(), 0);
        let events = take_encode_events()?;
        assert!(
            events
                .iter()
                .any(|event| event == "option:pixel_format=yuva420p")
        );
        assert!(events.iter().any(|event| event == "sws_alloc:26->33"));
        assert!(events.iter().any(|event| {
            event == "rgb:[0, 0, 127, 255, 0, 0, 127, 255, 0, 0, 127, 255, 0, 0, 127, 255]"
        }));
        assert!(events.iter().any(|event| {
            event == "rgb:[255, 63, 191, 0, 255, 63, 191, 0, 255, 63, 191, 0, 255, 63, 191, 0]"
        }));
        let metadata_index = events
            .iter()
            .position(|event| event == "metadata:prompt=alpha:flags=0")
            .ok_or("alpha metadata was not attached")?;
        let stream_index = events
            .iter()
            .position(|event| event == "stream_alloc")
            .ok_or("alpha stream was not allocated")?;
        assert!(metadata_index < stream_index);
        assert_eq!(
            events
                .iter()
                .filter(|event| event.as_str() == "frame_writable")
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.as_str() == "send_flush")
                .count(),
            1
        );
        Ok(())
    }

    #[test]
    fn retained_vp9_webm_alpha_staging_cancellation_and_retry_are_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::default();
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(4096)?;
        let construction_context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority.authorize_workspace(1024)?,
            rng_phase: None,
            cancellation: &cancellation,
        };
        let images =
            ImageTensor::from_f32(&backend, &construction_context, 1, 2, 2, 4, &[0.5; 16])?;
        assert_eq!(construction_context.scratch.in_use_bytes(), 0);

        let constrained_context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority.authorize_workspace(8)?,
            rng_phase: None,
            cancellation: &cancellation,
        };
        assert!(matches!(
            source_compatible_vp9_rgba8_frame(&images, 0, &backend, &constrained_context,),
            Err(NativeVideoCodecLtxvEncodeError::ResourceExhausted { .. })
        ));
        assert_eq!(constrained_context.scratch.in_use_bytes(), 0);

        let cancelled = CancellationToken::default();
        cancelled.cancel();
        let cancelled_context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority.authorize_workspace(64)?,
            rng_phase: None,
            cancellation: &cancelled,
        };
        assert!(matches!(
            source_compatible_vp9_rgba8_frame(&images, 0, &backend, &cancelled_context),
            Err(NativeVideoCodecLtxvEncodeError::Cancelled)
        ));
        assert_eq!(cancelled_context.scratch.in_use_bytes(), 0);

        let retry_context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority.authorize_workspace(64)?,
            rng_phase: None,
            cancellation: &cancellation,
        };
        let staged = source_compatible_vp9_rgba8_frame(&images, 0, &backend, &retry_context)?;
        assert_eq!(&*staged, &[127; 16]);
        drop(staged);
        assert_eq!(retry_context.scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn retained_vp9_webm_batch_encode_validates_bounds_and_global_protocol()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 256 * 1024)?;
        let session = NativeVp9WebmEncodeLimits::checked(64, 64, 1, 6)?;
        assert!(matches!(
            NativeVp9WebmBatchLimits::checked(session, 0, 4),
            Err(NativeVideoCodecVp9EncodeError::InvalidLimits)
        ));
        assert!(matches!(
            NativeVp9WebmBatchLimits::checked(session, 3, 0),
            Err(NativeVideoCodecVp9EncodeError::InvalidLimits)
        ));
        let limits = NativeVp9WebmBatchLimits::checked(session, 3, 4)?;
        let images = ImageTensor::from_f32(&backend, &context, 3, 2, 2, 3, &[0.5; 36])?;
        assert!(matches!(
            validate_vp9_image_batch(
                &images,
                NativeVp9WebmBatchLimits::checked(session, 2, 4)?,
                &context,
            ),
            Err(NativeVideoCodecVp9EncodeError::InvalidBatch)
        ));
        let four_channel = ImageTensor::from_f32(&backend, &context, 1, 2, 2, 4, &[0.5; 16])?;
        assert_eq!(
            validate_vp9_image_batch(&four_channel, limits, &context)?,
            (1, 2, 2, 4)
        );
        let one_channel = ImageTensor::from_f32(&backend, &context, 1, 2, 2, 1, &[0.5; 4])?;
        assert!(matches!(
            validate_vp9_image_batch(&one_channel, limits, &context),
            Err(NativeVideoCodecVp9EncodeError::InvalidBatch)
        ));

        reset_encode_fixture()?;
        let mut output = mock_encode_output(&backend, &context, 64)?;
        let encoder = NonNull::new(Box::into_raw(Box::new(0_u8)).cast::<abi::AvCodec>())
            .ok_or("mock encoder allocation failed")?;
        let mut provide_frame = |frame_index: usize,
                                 consume: &mut dyn FnMut(
            &[u8],
        ) -> Result<
            (),
            NativeVideoCodecLtxvEncodeError,
        >| {
            let frame = source_compatible_vp9_rgb8_frame(&images, frame_index, &backend, &context)?;
            consume(frame.as_u8_slice().map_err(map_vp9_staging_tensor_error)?)
        };
        assert!(matches!(
            encode_rgb8_frames_with_check(
                NativeRgb8EncodeProfile::vp9_webm(
                    checked_vp9_frame_rate((2_997, 125))?,
                    NativeVideoCrf::checked(32.0)?,
                ),
                encoder,
                3,
                2,
                2,
                session.maximum_packet_iterations,
                &mock_encode_functions(),
                &mut output,
                &mut provide_frame,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecLtxvEncodeError::PacketIterationLimit)
        ));
        drop(output);
        drop(unsafe { Box::from_raw(encoder.as_ptr().cast::<u8>()) });
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn retained_vp9_webm_batch_encode_failure_cancellation_and_retry_are_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 256 * 1024)?;
        let images = ImageTensor::from_f32(&backend, &context, 3, 2, 2, 3, &[0.5; 36])?;
        let encoder = NonNull::new(Box::into_raw(Box::new(0_u8)).cast::<abi::AvCodec>())
            .ok_or("mock encoder allocation failed")?;
        let profile = NativeRgb8EncodeProfile::vp9_webm(
            checked_vp9_frame_rate((2_997, 125))?,
            NativeVideoCrf::checked(32.0)?,
        );

        reset_encode_fixture()?;
        *ENCODE_FAILURE_PHASE
            .lock()
            .map_err(|_| "encode failure mutex was poisoned")? = Some("second_frame_writable");
        let mut failed_output = mock_encode_output(&backend, &context, 64)?;
        let mut provide_frame = |frame_index: usize,
                                 consume: &mut dyn FnMut(
            &[u8],
        ) -> Result<
            (),
            NativeVideoCodecLtxvEncodeError,
        >| {
            let frame = source_compatible_vp9_rgb8_frame(&images, frame_index, &backend, &context)?;
            consume(frame.as_u8_slice().map_err(map_vp9_staging_tensor_error)?)
        };
        assert!(matches!(
            encode_rgb8_frames_with_check(
                profile,
                encoder,
                3,
                2,
                2,
                16,
                &mock_encode_functions(),
                &mut failed_output,
                &mut provide_frame,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecLtxvEncodeError::ResourceExhausted {
                phase: "make YUV frame writable"
            })
        ));
        drop(failed_output);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        reset_encode_fixture()?;
        let mut cancelled_output = mock_encode_output(&backend, &context, 64)?;
        let mut cancel_on_second = |frame_index: usize,
                                    consume: &mut dyn FnMut(
            &[u8],
        ) -> Result<
            (),
            NativeVideoCodecLtxvEncodeError,
        >| {
            if frame_index == 1 {
                return Err(NativeVideoCodecLtxvEncodeError::Cancelled);
            }
            let frame = source_compatible_vp9_rgb8_frame(&images, frame_index, &backend, &context)?;
            consume(frame.as_u8_slice().map_err(map_vp9_staging_tensor_error)?)
        };
        assert!(matches!(
            encode_rgb8_frames_with_check(
                profile,
                encoder,
                3,
                2,
                2,
                16,
                &mock_encode_functions(),
                &mut cancelled_output,
                &mut cancel_on_second,
                &mut || cancellation.check(),
            ),
            Err(NativeVideoCodecLtxvEncodeError::Cancelled)
        ));
        drop(cancelled_output);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        reset_encode_fixture()?;
        let mut retry_output = mock_encode_output(&backend, &context, 64)?;
        let mut retry_frames = |frame_index: usize,
                                consume: &mut dyn FnMut(
            &[u8],
        ) -> Result<
            (),
            NativeVideoCodecLtxvEncodeError,
        >| {
            let frame = source_compatible_vp9_rgb8_frame(&images, frame_index, &backend, &context)?;
            consume(frame.as_u8_slice().map_err(map_vp9_staging_tensor_error)?)
        };
        encode_rgb8_frames_with_check(
            profile,
            encoder,
            3,
            2,
            2,
            16,
            &mock_encode_functions(),
            &mut retry_output,
            &mut retry_frames,
            &mut || cancellation.check(),
        )?;
        assert_eq!(retry_output.staged_bytes()?, b"HPPPT");
        drop(retry_output);
        drop(unsafe { Box::from_raw(encoder.as_ptr().cast::<u8>()) });
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn bounded_video_codec_avio_callbacks_enforce_read_write_seek_and_limits()
    -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::default();
        let input_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
        let mut input = NativeVideoCodecInputState {
            bytes: NonNull::new(input_bytes.as_ptr().cast_mut())
                .ok_or("input bytes were unexpectedly empty")?,
            byte_length: input_bytes.len(),
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
        let input_bytes: Arc<[u8]> = Arc::from([1_u8]);
        let mut input = NativeVideoCodecInputState {
            bytes: NonNull::new(input_bytes.as_ptr().cast_mut())
                .ok_or("input bytes were unexpectedly empty")?,
            byte_length: input_bytes.len(),
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
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
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

    static DECODE_EVENTS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    static DECODE_MODE: AtomicUsize = AtomicUsize::new(0);
    static DECODE_RECEIVE_CALLS: AtomicUsize = AtomicUsize::new(0);
    static DECODE_READ_CALLS: AtomicUsize = AtomicUsize::new(0);
    static DECODE_Y_PLANE: [u8; 4] = [16, 32, 48, 64];
    static DECODE_U_PLANE: [u8; 1] = [128];
    static DECODE_V_PLANE: [u8; 1] = [128];

    fn record_decode_event(event: &'static str) {
        match DECODE_EVENTS.lock() {
            Ok(mut events) => events.push(event),
            Err(_) => std::process::abort(),
        }
    }

    fn reset_decode_fixture(mode: usize) -> Result<(), Box<dyn std::error::Error>> {
        DECODE_MODE.store(mode, Ordering::Release);
        DECODE_RECEIVE_CALLS.store(0, Ordering::Release);
        DECODE_READ_CALLS.store(0, Ordering::Release);
        DECODE_EVENTS
            .lock()
            .map_err(|_| "decode event mutex was poisoned")?
            .clear();
        Ok(())
    }

    unsafe extern "C" fn mock_decode_codec_alloc(
        _codec: *const abi::AvCodec,
    ) -> *mut abi::AvCodecContext {
        record_decode_event("codec_alloc");
        if DECODE_MODE.load(Ordering::Acquire) == 3 {
            return std::ptr::null_mut();
        }
        Box::into_raw(Box::new(0_u8)).cast()
    }

    unsafe extern "C" fn mock_decode_codec_free(context: *mut *mut abi::AvCodecContext) {
        record_decode_event("codec_free");
        if !context.is_null() {
            let pointer = unsafe { *context };
            if !pointer.is_null() {
                drop(unsafe { Box::from_raw(pointer.cast::<u8>()) });
                unsafe { *context = std::ptr::null_mut() };
            }
        }
    }

    unsafe extern "C" fn mock_decode_parameters(
        _context: *mut abi::AvCodecContext,
        _parameters: *const abi::AvCodecParameters,
    ) -> i32 {
        record_decode_event("parameters");
        0
    }

    unsafe extern "C" fn mock_decode_opt_int(
        _object: *mut c_void,
        _name: *const std::ffi::c_char,
        _value: i64,
        _flags: i32,
    ) -> i32 {
        record_decode_event("threads");
        0
    }

    unsafe extern "C" fn mock_decode_open(
        _context: *mut abi::AvCodecContext,
        _codec: *const abi::AvCodec,
        _options: *mut *mut abi::AvDictionary,
    ) -> i32 {
        record_decode_event("codec_open");
        0
    }

    unsafe extern "C" fn mock_decode_packet_alloc() -> *mut abi::AvPacket {
        record_decode_event("packet_alloc");
        Box::into_raw(Box::new(abi::AvPacket {
            buffer: std::ptr::null_mut(),
            presentation_timestamp: abi::AV_NO_PRESENTATION_TIMESTAMP,
            decoding_timestamp: abi::AV_NO_PRESENTATION_TIMESTAMP,
            data: std::ptr::null_mut(),
            size: 0,
            stream_index: 0,
            flags: 0,
            side_data: std::ptr::null_mut(),
            side_data_count: 0,
            duration: 0,
        }))
    }

    unsafe extern "C" fn mock_decode_packet_unref(packet: *mut abi::AvPacket) {
        record_decode_event("packet_unref");
        if !packet.is_null() {
            unsafe {
                (*packet).data = std::ptr::null_mut();
                (*packet).size = 0;
            }
        }
    }

    unsafe extern "C" fn mock_decode_packet_free(packet: *mut *mut abi::AvPacket) {
        record_decode_event("packet_free");
        if !packet.is_null() {
            let pointer = unsafe { *packet };
            if !pointer.is_null() {
                drop(unsafe { Box::from_raw(pointer) });
                unsafe { *packet = std::ptr::null_mut() };
            }
        }
    }

    unsafe extern "C" fn mock_decode_frame_alloc() -> *mut abi::AvFrame {
        record_decode_event("frame_alloc");
        Box::into_raw(Box::new(abi::AvFrame {
            data: [std::ptr::null_mut(); abi::AV_NUM_DATA_POINTERS],
            line_size: [0; abi::AV_NUM_DATA_POINTERS],
            extended_data: std::ptr::null_mut(),
            width: 0,
            height: 0,
            sample_count: 0,
            format: -1,
            key_frame: 0,
            picture_type: 0,
            sample_aspect_ratio: abi::AvRational {
                numerator: 0,
                denominator: 1,
            },
            presentation_timestamp: abi::AV_NO_PRESENTATION_TIMESTAMP,
        }))
    }

    unsafe extern "C" fn mock_decode_frame_free(frame: *mut *mut abi::AvFrame) {
        record_decode_event("frame_free");
        if !frame.is_null() {
            let pointer = unsafe { *frame };
            if !pointer.is_null() {
                drop(unsafe { Box::from_raw(pointer) });
                unsafe { *frame = std::ptr::null_mut() };
            }
        }
    }

    unsafe extern "C" fn mock_decode_read_frame(
        _format: *mut abi::AvFormatContext,
        packet: *mut abi::AvPacket,
    ) -> i32 {
        record_decode_event("read");
        DECODE_READ_CALLS.fetch_add(1, Ordering::AcqRel);
        match DECODE_MODE.load(Ordering::Acquire) {
            1 => abi::AV_ERROR_END_OF_FILE,
            2 => abi::AV_ERROR_TRY_AGAIN,
            _ if packet.is_null() => abi::AV_ERROR_INVALID_ARGUMENT,
            _ => {
                unsafe {
                    (*packet).data = NonNull::<u8>::dangling().as_ptr();
                    (*packet).size = 1;
                    (*packet).stream_index = 0;
                }
                0
            }
        }
    }

    unsafe extern "C" fn mock_decode_send_packet(
        _context: *mut abi::AvCodecContext,
        packet: *const abi::AvPacket,
    ) -> i32 {
        record_decode_event(if packet.is_null() { "flush" } else { "send" });
        0
    }

    unsafe extern "C" fn mock_decode_receive_frame(
        _context: *mut abi::AvCodecContext,
        frame: *mut abi::AvFrame,
    ) -> i32 {
        record_decode_event("receive");
        let call = DECODE_RECEIVE_CALLS.fetch_add(1, Ordering::AcqRel);
        if call == 0 || DECODE_MODE.load(Ordering::Acquire) == 2 {
            return abi::AV_ERROR_TRY_AGAIN;
        }
        if frame.is_null() {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        unsafe {
            (*frame).width = if DECODE_MODE.load(Ordering::Acquire) == 4 {
                4
            } else {
                2
            };
            (*frame).height = 2;
            (*frame).format = abi::AV_PIXEL_FORMAT_YUV420P;
            (*frame).data[0] = DECODE_Y_PLANE.as_ptr().cast_mut();
            (*frame).data[1] = DECODE_U_PLANE.as_ptr().cast_mut();
            (*frame).data[2] = DECODE_V_PLANE.as_ptr().cast_mut();
            (*frame).line_size[0] = 2;
            (*frame).line_size[1] = 1;
            (*frame).line_size[2] = 1;
        }
        0
    }

    unsafe extern "C" fn mock_decode_sws_get(
        _source_width: i32,
        _source_height: i32,
        _source_format: i32,
        _destination_width: i32,
        _destination_height: i32,
        _destination_format: i32,
        _flags: i32,
        _source_filter: *mut abi::SwsFilter,
        _destination_filter: *mut abi::SwsFilter,
        _parameters: *const f64,
    ) -> *mut abi::SwsContext {
        record_decode_event("sws_get");
        NonNull::<u8>::dangling().as_ptr().cast()
    }

    unsafe extern "C" fn mock_decode_sws_scale(
        _context: *mut abi::SwsContext,
        _source_data: *const *const u8,
        _source_line_size: *const i32,
        _source_y: i32,
        source_height: i32,
        destination_data: *const *mut u8,
        _destination_line_size: *const i32,
    ) -> i32 {
        record_decode_event("sws_scale");
        if destination_data.is_null() || unsafe { (*destination_data).is_null() } {
            return abi::AV_ERROR_INVALID_ARGUMENT;
        }
        for index in 0..12_usize {
            unsafe { *(*destination_data).add(index) = u8::try_from(index).unwrap_or(0) };
        }
        source_height
    }

    unsafe extern "C" fn mock_decode_sws_free(_context: *mut abi::SwsContext) {
        record_decode_event("sws_free");
    }

    fn mock_decode_functions() -> NativeLtxvH264DecodeFunctions {
        NativeLtxvH264DecodeFunctions {
            av_packet_alloc: mock_decode_packet_alloc,
            av_packet_free: mock_decode_packet_free,
            av_packet_unref: mock_decode_packet_unref,
            avcodec_alloc_context3: mock_decode_codec_alloc,
            avcodec_free_context: mock_decode_codec_free,
            avcodec_open2: mock_decode_open,
            avcodec_parameters_to_context: mock_decode_parameters,
            avcodec_receive_frame: mock_decode_receive_frame,
            avcodec_send_packet: mock_decode_send_packet,
            av_read_frame: mock_decode_read_frame,
            av_frame_alloc: mock_decode_frame_alloc,
            av_frame_free: mock_decode_frame_free,
            av_opt_set_int: mock_decode_opt_int,
            sws_free_context: mock_decode_sws_free,
            sws_get_context: mock_decode_sws_get,
            sws_scale: mock_decode_sws_scale,
        }
    }

    fn decode_test_format() -> Box<MockDemuxFormat> {
        let mut parameters = Box::new(0_u8);
        let parameters_pointer = std::ptr::addr_of_mut!(*parameters).cast();
        let mut stream = Box::new(abi::AvStream {
            class: std::ptr::null(),
            index: 0,
            identifier: 0,
            codec_parameters: parameters_pointer,
            private_data: std::ptr::null_mut(),
            time_base: ltxv_frame_time_base(),
        });
        let stream_pointer = std::ptr::addr_of_mut!(*stream);
        let mut format = Box::new(MockDemuxFormat {
            prefix: abi::AvFormatContext {
                class: std::ptr::null(),
                input_format: std::ptr::null(),
                output_format: std::ptr::null(),
                private_data: std::ptr::null_mut(),
                io_context: std::ptr::null_mut(),
                context_flags: 0,
                stream_count: 1,
                streams: std::ptr::null_mut(),
            },
            parameters: vec![parameters],
            streams: vec![stream],
            stream_pointers: vec![stream_pointer],
        });
        format.prefix.streams = format.stream_pointers.as_mut_ptr();
        format
    }

    fn decode_limits() -> Result<NativeLtxvH264DecodeLimits, NativeVideoCodecLtxvDecodeError> {
        NativeLtxvH264DecodeLimits::checked(8, 8, 2, 2, 4, 12, 1_024)
    }

    fn preprocess_limits(
        maximum_batch: u64,
        maximum_output_elements: usize,
    ) -> Result<NativeLtxvH264PreprocessLimits, Box<dyn std::error::Error>> {
        Ok(NativeLtxvH264PreprocessLimits::checked(
            maximum_batch,
            maximum_output_elements,
            NativeLtxvH264EncodeLimits::checked(1_024, 64, 1_024, 8)?,
            NativeLtxvH264DemuxLimits::checked(1_024, 64, 1_024, 1)?,
            decode_limits()?,
        )?)
    }

    fn preprocess_test_image(
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<ImageTensor, TensorError> {
        let mut values = vec![0.0_f32; 2 * 3 * 3 * 3];
        let first = [
            0.0, 0.1, 0.5, 1.0, 1.1, -0.1, 9.0, 9.0, 9.0, 0.25, 0.5, 0.75, 0.9, 0.999, 1.01, 9.0,
            9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0,
        ];
        values[..first.len()].copy_from_slice(&first);
        for (index, value) in values[first.len()..].iter_mut().enumerate() {
            *value = f32::from(u8::try_from(index + 11).unwrap_or(0)) / 255.0;
        }
        ImageTensor::from_f32(backend, context, 2, 3, 3, 3, &values)
    }

    #[test]
    fn retained_ltxv_preprocess_bypasses_or_quantizes_crops_and_stacks_in_order()
    -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 1024 * 1024)?;
        let image = preprocess_test_image(&backend, &context)?;
        let source_id = image.tensor().tensor_id();
        let source_values = image.as_f32_slice()?.to_vec();
        let mut bypass_calls = 0;
        let bypass = preprocess_ltxv_image_with_round_trip(
            &image,
            0,
            preprocess_limits(2, 54)?,
            &backend,
            &context,
            &mut |_, _, _| {
                bypass_calls += 1;
                Err(NativeVideoCodecLtxvPreprocessError::InvalidInput)
            },
        )?;
        assert_eq!(bypass_calls, 0);
        assert_ne!(bypass.tensor().tensor_id(), source_id);
        assert_eq!(bypass.as_f32_slice()?, source_values);

        let rgba_values = [-0.25, 0.0, 1.0, 1.25, 0.1, 0.2, 0.3, 0.4];
        let rgba = ImageTensor::from_f32(&backend, &context, 1, 1, 2, 4, &rgba_values)?;
        let rgba_bypass = preprocess_ltxv_image_with_round_trip(
            &rgba,
            0,
            preprocess_limits(1, rgba_values.len())?,
            &backend,
            &context,
            &mut |_, _, _| Err(NativeVideoCodecLtxvPreprocessError::InvalidInput),
        )?;
        assert_eq!(rgba_bypass.dimensions()?, (1, 1, 2, 4));
        assert_eq!(rgba_bypass.as_f32_slice()?, rgba_values);

        let zero_spatial = ImageTensor::from_f32(&backend, &context, 2, 0, 3, 4, &[])?;
        let zero_spatial_bypass = preprocess_ltxv_image_with_round_trip(
            &zero_spatial,
            0,
            preprocess_limits(2, 1)?,
            &backend,
            &context,
            &mut |_, _, _| Err(NativeVideoCodecLtxvPreprocessError::InvalidInput),
        )?;
        assert_eq!(zero_spatial_bypass.dimensions()?, (2, 0, 3, 4));
        assert!(zero_spatial_bypass.as_f32_slice()?.is_empty());

        let mut encoded_frames = Vec::new();
        let compressed = preprocess_ltxv_image_with_round_trip(
            &image,
            35,
            preprocess_limits(2, 24)?,
            &backend,
            &context,
            &mut |frame, _, _| {
                encoded_frames.push(frame.as_u8_slice()?.to_vec());
                Ok(frame.clone())
            },
        )?;
        assert_eq!(compressed.dimensions()?, (2, 2, 2, 3));
        assert_eq!(
            encoded_frames.first().map(Vec::as_slice),
            Some([0, 25, 127, 255, 24, 231, 63, 127, 191, 229, 254, 1].as_slice())
        );
        assert_eq!(
            encoded_frames.get(1).map(Vec::as_slice),
            Some([11, 12, 13, 14, 15, 16, 20, 21, 22, 23, 24, 25].as_slice())
        );
        let expected = encoded_frames
            .iter()
            .flatten()
            .map(|value| f32::from(*value) / 255.0)
            .collect::<Vec<_>>();
        assert_eq!(compressed.as_f32_slice()?, expected);
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn retained_ltxv_preprocess_failure_cancellation_and_retry_are_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 1024 * 1024)?;
        let image = preprocess_test_image(&backend, &context)?;
        let mut calls = 0;
        assert!(matches!(
            preprocess_ltxv_image_with_round_trip(
                &image,
                101,
                preprocess_limits(2, 24)?,
                &backend,
                &context,
                &mut |frame, _, _| {
                    calls += 1;
                    Ok(frame.clone())
                },
            ),
            Err(NativeVideoCodecLtxvPreprocessError::InvalidCompression)
        ));
        assert_eq!(calls, 0);
        assert!(matches!(
            preprocess_ltxv_image_with_round_trip(
                &image,
                35,
                preprocess_limits(2, 23)?,
                &backend,
                &context,
                &mut |frame, _, _| Ok(frame.clone()),
            ),
            Err(NativeVideoCodecLtxvPreprocessError::ResourceExhausted)
        ));

        calls = 0;
        assert!(matches!(
            preprocess_ltxv_image_with_round_trip(
                &image,
                35,
                preprocess_limits(2, 24)?,
                &backend,
                &context,
                &mut |frame, _, _| {
                    calls += 1;
                    if calls == 2 {
                        Err(NativeVideoCodecLtxvPreprocessError::InvalidInput)
                    } else {
                        Ok(frame.clone())
                    }
                },
            ),
            Err(NativeVideoCodecLtxvPreprocessError::InvalidInput)
        ));
        assert_eq!(calls, 2);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        let cancelled = CancellationToken::default();
        let (cancel_backend, cancel_context) = avio_context(&cancelled, 1024 * 1024)?;
        let cancel_image = preprocess_test_image(&cancel_backend, &cancel_context)?;
        assert!(matches!(
            preprocess_ltxv_image_with_round_trip(
                &cancel_image,
                35,
                preprocess_limits(2, 24)?,
                &cancel_backend,
                &cancel_context,
                &mut |frame, _, _| {
                    cancelled.cancel();
                    Ok(frame.clone())
                },
            ),
            Err(NativeVideoCodecLtxvPreprocessError::Cancelled)
        ));
        assert_eq!(cancel_context.scratch.in_use_bytes(), 0);

        let retry = preprocess_ltxv_image_with_round_trip(
            &image,
            35,
            preprocess_limits(2, 24)?,
            &backend,
            &context,
            &mut |frame, _, _| Ok(frame.clone()),
        )?;
        assert_eq!(retry.dimensions()?, (2, 2, 2, 3));
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn retained_ltxv_h264_decode_returns_first_rgb8_frame_and_drains_packets()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        for mode in [0, 1] {
            reset_decode_fixture(mode)?;
            let cancellation = CancellationToken::default();
            let (backend, context) = avio_context(&cancellation, 128 * 1024)?;
            let input = mock_borrowed_input(b"MP4", &backend, &context)?;
            let mut format = decode_test_format();
            let format_pointer = NonNull::new(std::ptr::addr_of_mut!(format.prefix))
                .ok_or("decode format pointer was null")?;
            let image = decode_first_ltxv_h264_frame_with_check(
                format_pointer,
                0,
                NonNull::dangling(),
                2,
                2,
                decode_limits()?,
                &mock_decode_functions(),
                &input,
                &backend,
                &context,
                &mut || cancellation.check(),
            )?;
            assert_eq!(image.dimensions()?, (2, 2));
            assert_eq!(image.as_u8_slice()?, (0_u8..12).collect::<Vec<_>>());
            let events = DECODE_EVENTS
                .lock()
                .map_err(|_| "decode event mutex was poisoned")?;
            assert!(events.starts_with(&[
                "codec_alloc",
                "parameters",
                "threads",
                "codec_open",
                "packet_alloc",
                "frame_alloc",
                "receive",
            ]));
            assert!(events.contains(&"packet_unref") || mode == 1);
            assert!(events.ends_with(&["sws_free", "frame_free", "packet_free", "codec_free"]));
            drop(events);
            drop(input);
            assert_eq!(context.scratch.in_use_bytes(), 0);
        }
        Ok(())
    }

    #[test]
    fn retained_ltxv_h264_decode_failure_cancellation_and_retry_are_atomic()
    -> Result<(), Box<dyn std::error::Error>> {
        let _test_serial_guard = TEST_SERIAL
            .lock()
            .map_err(|_| "video codec test mutex was poisoned")?;
        assert!(matches!(
            NativeLtxvH264DecodeLimits::checked(0, 1, 2, 2, 4, 12, 1_024),
            Err(NativeVideoCodecLtxvDecodeError::InvalidLimits)
        ));
        for (mode, expected) in [(2, "iteration"), (3, "allocation"), (4, "frame")] {
            reset_decode_fixture(mode)?;
            let cancellation = CancellationToken::default();
            let (backend, context) = avio_context(&cancellation, 128 * 1024)?;
            let input = mock_borrowed_input(b"MP4", &backend, &context)?;
            let mut format = decode_test_format();
            let format_pointer = NonNull::new(std::ptr::addr_of_mut!(format.prefix))
                .ok_or("decode format pointer was null")?;
            let result = decode_first_ltxv_h264_frame_with_check(
                format_pointer,
                0,
                NonNull::dangling(),
                2,
                2,
                decode_limits()?,
                &mock_decode_functions(),
                &input,
                &backend,
                &context,
                &mut || cancellation.check(),
            );
            assert!(result.is_err(), "{expected} failure unexpectedly succeeded");
            drop(input);
            assert_eq!(context.scratch.in_use_bytes(), 0);
        }

        reset_decode_fixture(0)?;
        let cancellation = CancellationToken::default();
        let (backend, context) = avio_context(&cancellation, 128 * 1024)?;
        let input = mock_borrowed_input(b"MP4", &backend, &context)?;
        let mut format = decode_test_format();
        let format_pointer = NonNull::new(std::ptr::addr_of_mut!(format.prefix))
            .ok_or("decode format pointer was null")?;
        let mut checks = 0;
        assert!(matches!(
            decode_first_ltxv_h264_frame_with_check(
                format_pointer,
                0,
                NonNull::dangling(),
                2,
                2,
                decode_limits()?,
                &mock_decode_functions(),
                &input,
                &backend,
                &context,
                &mut || {
                    checks += 1;
                    if checks == 14 {
                        Err(CancellationError)
                    } else {
                        Ok(())
                    }
                },
            ),
            Err(NativeVideoCodecLtxvDecodeError::Cancelled)
        ));
        drop(input);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        reset_decode_fixture(0)?;
        let input = mock_borrowed_input(b"MP4", &backend, &context)?;
        let mut retry_format = decode_test_format();
        let retry_pointer = NonNull::new(std::ptr::addr_of_mut!(retry_format.prefix))
            .ok_or("retry decode format pointer was null")?;
        decode_first_ltxv_h264_frame_with_check(
            retry_pointer,
            0,
            NonNull::dangling(),
            2,
            2,
            decode_limits()?,
            &mock_decode_functions(),
            &input,
            &backend,
            &context,
            &mut || Ok(()),
        )?;
        drop(input);
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }
}
