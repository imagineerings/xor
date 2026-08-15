#![allow(dead_code)]

use std::{
    ffi::{c_char, c_double, c_int, c_uint, c_void},
    marker::{PhantomData, PhantomPinned},
};

pub(crate) const FFMPEG_7_1_SOURCE_ARCHIVE_SHA256: &str =
    "40973d44970dbc83ef302b0609f2e74982be2d85916dd2ee7472d30678a7abe6";
pub(crate) const FFMPEG_7_1_RELEASE_SIGNING_KEY_FINGERPRINT: &str =
    "FCF986EA15E6E293A5644F10B4322F04D67658D8";
pub(crate) const FFMPEG_7_1_RELEASE_SIGNATURE_SHA256: &str =
    "9bd1689dce76b109034dcc4765a406e84e8799a2fd857b000c0a4d9744b70617";

pub(crate) const FFMPEG_7_1_AVCODEC_VERSION: c_uint = version(61, 19, 100);
pub(crate) const FFMPEG_7_1_AVFORMAT_VERSION: c_uint = version(61, 7, 100);
pub(crate) const FFMPEG_7_1_AVUTIL_VERSION: c_uint = version(59, 39, 100);
pub(crate) const FFMPEG_7_1_SWRESAMPLE_VERSION: c_uint = version(5, 3, 100);
pub(crate) const FFMPEG_7_1_SWSCALE_VERSION: c_uint = version(8, 3, 100);

const fn version(major: c_uint, minor: c_uint, micro: c_uint) -> c_uint {
    (major << 16) | (minor << 8) | micro
}

macro_rules! opaque_ffi_type {
    ($name:ident) => {
        #[repr(C)]
        pub(crate) struct $name {
            _private: [u8; 0],
            _thread_bound: PhantomData<(*mut u8, PhantomPinned)>,
        }
    };
}

opaque_ffi_type!(AvCodec);
opaque_ffi_type!(AvCodecContext);
opaque_ffi_type!(AvCodecParameters);
opaque_ffi_type!(AvBufferRef);
opaque_ffi_type!(AvClass);
opaque_ffi_type!(AvDictionary);
opaque_ffi_type!(AvInputFormat);
opaque_ffi_type!(AvOutputFormat);
opaque_ffi_type!(AvPacketSideData);
opaque_ffi_type!(SwrContext);
opaque_ffi_type!(SwsContext);
opaque_ffi_type!(SwsFilter);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub(crate) struct AvRational {
    pub(crate) numerator: c_int,
    pub(crate) denominator: c_int,
}

#[repr(C)]
pub(crate) struct AvChannelCustom {
    pub(crate) identifier: c_int,
    pub(crate) name: [c_char; 16],
    pub(crate) opaque: *mut c_void,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) union AvChannelLayoutData {
    pub(crate) mask: u64,
    pub(crate) map: *mut AvChannelCustom,
}

#[repr(C)]
pub(crate) struct AvChannelLayout {
    pub(crate) order: c_int,
    pub(crate) channel_count: c_int,
    pub(crate) data: AvChannelLayoutData,
    pub(crate) opaque: *mut c_void,
}

pub(crate) const AV_NUM_DATA_POINTERS: usize = 8;
pub(crate) const AV_MEDIA_TYPE_VIDEO: c_int = 0;
pub(crate) const AV_CODEC_ID_H264: c_int = 27;
pub(crate) const AV_CODEC_ID_VP9: c_int = 167;
pub(crate) const AV_CODEC_ID_AV1: c_int = 225;
pub(crate) const AV_CODEC_ID_AAC: c_int = 86_018;
pub(crate) const AV_PIXEL_FORMAT_YUV420P: c_int = 0;
pub(crate) const AV_PIXEL_FORMAT_RGB24: c_int = 2;
pub(crate) const AV_NO_PRESENTATION_TIMESTAMP: i64 = i64::MIN;
pub(crate) const AV_SEEK_SIZE: c_int = 0x1_0000;
pub(crate) const AV_SEEK_FORCE: c_int = 0x2_0000;
pub(crate) const AV_FORMAT_FLAG_CUSTOM_IO: c_int = 0x0080;
pub(crate) const AV_CODEC_FLAG_GLOBAL_HEADER: c_int = 1 << 22;
pub(crate) const AV_OPTION_SEARCH_CHILDREN: c_int = 1;
pub(crate) const SWS_BILINEAR: c_int = 2;
pub(crate) const AV_ERROR_TRY_AGAIN: c_int = -11;
pub(crate) const AV_ERROR_OUT_OF_MEMORY: c_int = -12;
pub(crate) const AV_ERROR_INVALID_ARGUMENT: c_int = -22;
pub(crate) const AV_ERROR_NO_SPACE: c_int = -28;
pub(crate) const AV_ERROR_END_OF_FILE: c_int = -541_478_725;
pub(crate) const AV_ERROR_EXIT: c_int = -1_414_092_869;

// These reviewed prefixes are only for accessing objects allocated by FFmpeg;
// their Rust size is intentionally not the complete public C struct size.
#[repr(C)]
pub(crate) struct AvFrame {
    pub(crate) data: [*mut u8; AV_NUM_DATA_POINTERS],
    pub(crate) line_size: [c_int; AV_NUM_DATA_POINTERS],
    pub(crate) extended_data: *mut *mut u8,
    pub(crate) width: c_int,
    pub(crate) height: c_int,
    pub(crate) sample_count: c_int,
    pub(crate) format: c_int,
    pub(crate) key_frame: c_int,
    pub(crate) picture_type: c_int,
    pub(crate) sample_aspect_ratio: AvRational,
    pub(crate) presentation_timestamp: i64,
}

#[repr(C)]
pub(crate) struct AvPacket {
    pub(crate) buffer: *mut AvBufferRef,
    pub(crate) presentation_timestamp: i64,
    pub(crate) decoding_timestamp: i64,
    pub(crate) data: *mut u8,
    pub(crate) size: c_int,
    pub(crate) stream_index: c_int,
    pub(crate) flags: c_int,
    pub(crate) side_data: *mut AvPacketSideData,
    pub(crate) side_data_count: c_int,
    pub(crate) duration: i64,
}

#[repr(C)]
pub(crate) struct AvStream {
    pub(crate) class: *const AvClass,
    pub(crate) index: c_int,
    pub(crate) identifier: c_int,
    pub(crate) codec_parameters: *mut AvCodecParameters,
    pub(crate) private_data: *mut c_void,
    pub(crate) time_base: AvRational,
}

#[repr(C)]
pub(crate) struct AvFormatContext {
    pub(crate) class: *const AvClass,
    pub(crate) input_format: *const AvInputFormat,
    pub(crate) output_format: *const AvOutputFormat,
    pub(crate) private_data: *mut c_void,
    pub(crate) io_context: *mut AvIoContext,
    pub(crate) context_flags: c_int,
    pub(crate) stream_count: c_uint,
    pub(crate) streams: *mut *mut AvStream,
}

#[repr(C)]
pub(crate) struct AvFormatContextMetadataProjection {
    pub(crate) prefix: AvFormatContext,
    pub(crate) opaque_stream_groups_through_data_codec_id: [u8; 136],
    pub(crate) metadata: *mut AvDictionary,
}

#[repr(C)]
pub(crate) struct AvIoContext {
    pub(crate) class: *const AvClass,
    pub(crate) buffer: *mut u8,
}

pub(crate) type AvIoReadPacket = unsafe extern "C" fn(*mut c_void, *mut u8, c_int) -> c_int;
pub(crate) type AvIoWritePacket = unsafe extern "C" fn(*mut c_void, *const u8, c_int) -> c_int;
pub(crate) type AvIoSeek = unsafe extern "C" fn(*mut c_void, i64, c_int) -> i64;

pub(crate) type AvPacketAlloc = unsafe extern "C" fn() -> *mut AvPacket;
pub(crate) type AvPacketFree = unsafe extern "C" fn(*mut *mut AvPacket);
pub(crate) type AvPacketUnref = unsafe extern "C" fn(*mut AvPacket);
pub(crate) type AvcodecAllocContext3 = unsafe extern "C" fn(*const AvCodec) -> *mut AvCodecContext;
pub(crate) type AvcodecFindDecoder = unsafe extern "C" fn(c_int) -> *const AvCodec;
pub(crate) type AvcodecFindEncoderByName = unsafe extern "C" fn(*const c_char) -> *const AvCodec;
pub(crate) type AvcodecFreeContext = unsafe extern "C" fn(*mut *mut AvCodecContext);
pub(crate) type AvcodecOpen2 =
    unsafe extern "C" fn(*mut AvCodecContext, *const AvCodec, *mut *mut AvDictionary) -> c_int;
pub(crate) type AvcodecParametersFromContext =
    unsafe extern "C" fn(*mut AvCodecParameters, *const AvCodecContext) -> c_int;
pub(crate) type AvcodecParametersToContext =
    unsafe extern "C" fn(*mut AvCodecContext, *const AvCodecParameters) -> c_int;
pub(crate) type AvcodecReceiveFrame =
    unsafe extern "C" fn(*mut AvCodecContext, *mut AvFrame) -> c_int;
pub(crate) type AvcodecReceivePacket =
    unsafe extern "C" fn(*mut AvCodecContext, *mut AvPacket) -> c_int;
pub(crate) type AvcodecSendFrame =
    unsafe extern "C" fn(*mut AvCodecContext, *const AvFrame) -> c_int;
pub(crate) type AvcodecSendPacket =
    unsafe extern "C" fn(*mut AvCodecContext, *const AvPacket) -> c_int;
pub(crate) type AvcodecVersion = unsafe extern "C" fn() -> c_uint;

pub(crate) type AvFindBestStream = unsafe extern "C" fn(
    *mut AvFormatContext,
    c_int,
    c_int,
    c_int,
    *mut *const AvCodec,
    c_int,
) -> c_int;
pub(crate) type AvInterleavedWriteFrame =
    unsafe extern "C" fn(*mut AvFormatContext, *mut AvPacket) -> c_int;
pub(crate) type AvReadFrame = unsafe extern "C" fn(*mut AvFormatContext, *mut AvPacket) -> c_int;
pub(crate) type AvWriteTrailer = unsafe extern "C" fn(*mut AvFormatContext) -> c_int;
pub(crate) type AvformatAllocContext = unsafe extern "C" fn() -> *mut AvFormatContext;
pub(crate) type AvformatAllocOutputContext2 = unsafe extern "C" fn(
    *mut *mut AvFormatContext,
    *const AvOutputFormat,
    *const c_char,
    *const c_char,
) -> c_int;
pub(crate) type AvformatCloseInput = unsafe extern "C" fn(*mut *mut AvFormatContext);
pub(crate) type AvformatFindStreamInfo =
    unsafe extern "C" fn(*mut AvFormatContext, *mut *mut AvDictionary) -> c_int;
pub(crate) type AvformatFreeContext = unsafe extern "C" fn(*mut AvFormatContext);
pub(crate) type AvformatNewStream =
    unsafe extern "C" fn(*mut AvFormatContext, *const AvCodec) -> *mut AvStream;
pub(crate) type AvformatOpenInput = unsafe extern "C" fn(
    *mut *mut AvFormatContext,
    *const c_char,
    *const AvInputFormat,
    *mut *mut AvDictionary,
) -> c_int;
pub(crate) type AvformatWriteHeader =
    unsafe extern "C" fn(*mut AvFormatContext, *mut *mut AvDictionary) -> c_int;
pub(crate) type AvformatVersion = unsafe extern "C" fn() -> c_uint;
pub(crate) type AvioAllocContext = unsafe extern "C" fn(
    *mut u8,
    c_int,
    c_int,
    *mut c_void,
    Option<AvIoReadPacket>,
    Option<AvIoWritePacket>,
    Option<AvIoSeek>,
) -> *mut AvIoContext;
pub(crate) type AvioContextFree = unsafe extern "C" fn(*mut *mut AvIoContext);

pub(crate) type AvChannelLayoutDefault = unsafe extern "C" fn(*mut AvChannelLayout, c_int);
pub(crate) type AvChannelLayoutUninit = unsafe extern "C" fn(*mut AvChannelLayout);
pub(crate) type AvDictFree = unsafe extern "C" fn(*mut *mut AvDictionary);
pub(crate) type AvDictSet =
    unsafe extern "C" fn(*mut *mut AvDictionary, *const c_char, *const c_char, c_int) -> c_int;
pub(crate) type AvFrameAlloc = unsafe extern "C" fn() -> *mut AvFrame;
pub(crate) type AvFrameFree = unsafe extern "C" fn(*mut *mut AvFrame);
pub(crate) type AvFrameGetBuffer = unsafe extern "C" fn(*mut AvFrame, c_int) -> c_int;
pub(crate) type AvFrameMakeWritable = unsafe extern "C" fn(*mut AvFrame) -> c_int;
pub(crate) type AvFree = unsafe extern "C" fn(*mut c_void);
pub(crate) type AvMalloc = unsafe extern "C" fn(usize) -> *mut c_void;
pub(crate) type AvOptSet =
    unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char, c_int) -> c_int;
pub(crate) type AvOptSetInt = unsafe extern "C" fn(*mut c_void, *const c_char, i64, c_int) -> c_int;
pub(crate) type AvRescaleQ = unsafe extern "C" fn(i64, AvRational, AvRational) -> i64;
pub(crate) type AvutilVersion = unsafe extern "C" fn() -> c_uint;

pub(crate) type SwrAlloc = unsafe extern "C" fn() -> *mut SwrContext;
pub(crate) type SwrAllocSetOpts2 = unsafe extern "C" fn(
    *mut *mut SwrContext,
    *const AvChannelLayout,
    c_int,
    c_int,
    *const AvChannelLayout,
    c_int,
    c_int,
    c_int,
    *mut c_void,
) -> c_int;
pub(crate) type SwrConvert =
    unsafe extern "C" fn(*mut SwrContext, *mut *mut u8, c_int, *const *const u8, c_int) -> c_int;
pub(crate) type SwrFree = unsafe extern "C" fn(*mut *mut SwrContext);
pub(crate) type SwrInit = unsafe extern "C" fn(*mut SwrContext) -> c_int;
pub(crate) type SwresampleVersion = unsafe extern "C" fn() -> c_uint;

pub(crate) type SwsFreeContext = unsafe extern "C" fn(*mut SwsContext);
pub(crate) type SwsGetContext = unsafe extern "C" fn(
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    *mut SwsFilter,
    *mut SwsFilter,
    *const c_double,
) -> *mut SwsContext;
pub(crate) type SwsScale = unsafe extern "C" fn(
    *mut SwsContext,
    *const *const u8,
    *const c_int,
    c_int,
    c_int,
    *const *mut u8,
    *const c_int,
) -> c_int;
pub(crate) type SwscaleVersion = unsafe extern "C" fn() -> c_uint;

pub(crate) const VIDEO_CODEC_AVCODEC_SYMBOLS: [&str; 15] = [
    "av_packet_alloc",
    "av_packet_free",
    "av_packet_unref",
    "avcodec_alloc_context3",
    "avcodec_find_decoder",
    "avcodec_find_encoder_by_name",
    "avcodec_free_context",
    "avcodec_open2",
    "avcodec_parameters_from_context",
    "avcodec_parameters_to_context",
    "avcodec_receive_frame",
    "avcodec_receive_packet",
    "avcodec_send_frame",
    "avcodec_send_packet",
    "avcodec_version",
];
pub(crate) const VIDEO_CODEC_AVFORMAT_SYMBOLS: [&str; 15] = [
    "av_find_best_stream",
    "av_interleaved_write_frame",
    "av_read_frame",
    "av_write_trailer",
    "avformat_alloc_context",
    "avformat_alloc_output_context2",
    "avformat_close_input",
    "avformat_find_stream_info",
    "avformat_free_context",
    "avformat_new_stream",
    "avformat_open_input",
    "avformat_version",
    "avformat_write_header",
    "avio_alloc_context",
    "avio_context_free",
];
pub(crate) const VIDEO_CODEC_AVUTIL_SYMBOLS: [&str; 14] = [
    "av_channel_layout_default",
    "av_channel_layout_uninit",
    "av_dict_free",
    "av_dict_set",
    "av_frame_alloc",
    "av_frame_free",
    "av_frame_get_buffer",
    "av_frame_make_writable",
    "av_free",
    "av_malloc",
    "av_opt_set",
    "av_opt_set_int",
    "av_rescale_q",
    "avutil_version",
];
pub(crate) const VIDEO_CODEC_SWRESAMPLE_SYMBOLS: [&str; 6] = [
    "swr_alloc",
    "swr_alloc_set_opts2",
    "swr_convert",
    "swr_free",
    "swr_init",
    "swresample_version",
];
pub(crate) const VIDEO_CODEC_SWSCALE_SYMBOLS: [&str; 4] = [
    "sws_freeContext",
    "sws_getContext",
    "sws_scale",
    "swscale_version",
];

pub(crate) fn video_codec_library_contracts() -> [(&'static str, u16, &'static [&'static str]); 5] {
    [
        ("avcodec", 61, &VIDEO_CODEC_AVCODEC_SYMBOLS),
        ("avformat", 61, &VIDEO_CODEC_AVFORMAT_SYMBOLS),
        ("avutil", 59, &VIDEO_CODEC_AVUTIL_SYMBOLS),
        ("swresample", 5, &VIDEO_CODEC_SWRESAMPLE_SYMBOLS),
        ("swscale", 8, &VIDEO_CODEC_SWSCALE_SYMBOLS),
    ]
}

pub(crate) fn video_codec_symbol_version_namespace(identity: &str) -> Option<&'static str> {
    match identity {
        "avcodec" => Some("LIBAVCODEC_61"),
        "avformat" => Some("LIBAVFORMAT_61"),
        "avutil" => Some("LIBAVUTIL_59"),
        "swresample" => Some("LIBSWRESAMPLE_5"),
        "swscale" => Some("LIBSWSCALE_8"),
        _ => None,
    }
}

pub(crate) fn video_codec_abi_version(identity: &str) -> Option<&'static str> {
    match identity {
        "avcodec" => Some("ffmpeg-7.1:61"),
        "avformat" => Some("ffmpeg-7.1:61"),
        "avutil" => Some("ffmpeg-7.1:59"),
        "swresample" => Some("ffmpeg-7.1:5"),
        "swscale" => Some("ffmpeg-7.1:8"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeSet, mem};

    #[test]
    fn ffmpeg_7_1_reviewed_abi_contract_is_exact() {
        assert_eq!(mem::size_of::<AvRational>(), 8);
        assert_eq!(mem::align_of::<AvRational>(), 4);
        assert_eq!(mem::offset_of!(AvRational, numerator), 0);
        assert_eq!(mem::offset_of!(AvRational, denominator), 4);
        assert_eq!(mem::size_of::<AvChannelLayout>(), 24);
        assert_eq!(mem::align_of::<AvChannelLayout>(), 8);
        assert_eq!(mem::offset_of!(AvChannelLayout, order), 0);
        assert_eq!(mem::offset_of!(AvChannelLayout, channel_count), 4);
        assert_eq!(mem::offset_of!(AvChannelLayout, data), 8);
        assert_eq!(mem::offset_of!(AvChannelLayout, opaque), 16);
        assert_eq!(
            mem::size_of::<Option<AvIoReadPacket>>(),
            mem::size_of::<usize>()
        );
        assert_eq!(mem::size_of::<AvcodecVersion>(), mem::size_of::<usize>());

        let contracts = video_codec_library_contracts();
        assert_eq!(contracts.len(), 5);
        let symbols = contracts
            .iter()
            .flat_map(|(_, _, symbols)| symbols.iter().copied())
            .collect::<BTreeSet<_>>();
        assert_eq!(symbols.len(), 54);
        assert_eq!(contracts[0].2.len(), 15);
        assert_eq!(contracts[1].2.len(), 15);
        assert_eq!(contracts[2].2.len(), 14);
        assert_eq!(contracts[3].2.len(), 6);
        assert_eq!(contracts[4].2.len(), 4);

        assert_eq!(FFMPEG_7_1_AVCODEC_VERSION, 0x3d1364);
        assert_eq!(FFMPEG_7_1_AVFORMAT_VERSION, 0x3d0764);
        assert_eq!(FFMPEG_7_1_AVUTIL_VERSION, 0x3b2764);
        assert_eq!(FFMPEG_7_1_SWRESAMPLE_VERSION, 0x050364);
        assert_eq!(FFMPEG_7_1_SWSCALE_VERSION, 0x080364);
        assert_eq!(FFMPEG_7_1_SOURCE_ARCHIVE_SHA256.len(), 64);
        assert_eq!(FFMPEG_7_1_RELEASE_SIGNING_KEY_FINGERPRINT.len(), 40);
        assert_eq!(FFMPEG_7_1_RELEASE_SIGNATURE_SHA256.len(), 64);
    }

    #[test]
    fn reviewed_manifest_matches_compiled_abi_contract() {
        let manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../abi/video-codec/ffmpeg-7.1-x86_64-gnu-v1.json"
        ))
        .expect("reviewed ABI manifest must be valid JSON");
        assert_eq!(
            manifest["source"]["archive_sha256"],
            FFMPEG_7_1_SOURCE_ARCHIVE_SHA256
        );
        assert_eq!(
            manifest["source"]["signing_key_fingerprint"],
            FFMPEG_7_1_RELEASE_SIGNING_KEY_FINGERPRINT
        );
        assert_eq!(manifest["contract"]["function_count"], 54);
        for (identity, abi_major, symbols) in video_codec_library_contracts() {
            assert_eq!(manifest["libraries"][identity]["abi_major"], abi_major);
            assert_eq!(
                manifest["libraries"][identity]["symbol_version_namespace"],
                video_codec_symbol_version_namespace(identity)
                    .expect("reviewed library must have a symbol-version namespace")
            );
            let manifest_symbols = manifest["libraries"][identity]["symbols"]
                .as_array()
                .expect("library symbols must be an array")
                .iter()
                .map(|value| value.as_str().expect("symbol must be a string"))
                .collect::<Vec<_>>();
            assert_eq!(manifest_symbols, symbols);
        }
    }

    #[test]
    fn ffmpeg_7_1_data_plane_prefixes_and_constants_are_exact() {
        assert_eq!(mem::size_of::<AvFrame>(), 144);
        assert_eq!(mem::align_of::<AvFrame>(), 8);
        assert_eq!(mem::offset_of!(AvFrame, data), 0);
        assert_eq!(mem::offset_of!(AvFrame, line_size), 64);
        assert_eq!(mem::offset_of!(AvFrame, extended_data), 96);
        assert_eq!(mem::offset_of!(AvFrame, width), 104);
        assert_eq!(mem::offset_of!(AvFrame, height), 108);
        assert_eq!(mem::offset_of!(AvFrame, sample_count), 112);
        assert_eq!(mem::offset_of!(AvFrame, format), 116);
        assert_eq!(mem::offset_of!(AvFrame, key_frame), 120);
        assert_eq!(mem::offset_of!(AvFrame, picture_type), 124);
        assert_eq!(mem::offset_of!(AvFrame, sample_aspect_ratio), 128);
        assert_eq!(mem::offset_of!(AvFrame, presentation_timestamp), 136);

        assert_eq!(mem::size_of::<AvPacket>(), 72);
        assert_eq!(mem::align_of::<AvPacket>(), 8);
        assert_eq!(mem::offset_of!(AvPacket, buffer), 0);
        assert_eq!(mem::offset_of!(AvPacket, presentation_timestamp), 8);
        assert_eq!(mem::offset_of!(AvPacket, decoding_timestamp), 16);
        assert_eq!(mem::offset_of!(AvPacket, data), 24);
        assert_eq!(mem::offset_of!(AvPacket, size), 32);
        assert_eq!(mem::offset_of!(AvPacket, stream_index), 36);
        assert_eq!(mem::offset_of!(AvPacket, flags), 40);
        assert_eq!(mem::offset_of!(AvPacket, side_data), 48);
        assert_eq!(mem::offset_of!(AvPacket, side_data_count), 56);
        assert_eq!(mem::offset_of!(AvPacket, duration), 64);

        assert_eq!(mem::size_of::<AvStream>(), 40);
        assert_eq!(mem::align_of::<AvStream>(), 8);
        assert_eq!(mem::offset_of!(AvStream, class), 0);
        assert_eq!(mem::offset_of!(AvStream, index), 8);
        assert_eq!(mem::offset_of!(AvStream, identifier), 12);
        assert_eq!(mem::offset_of!(AvStream, codec_parameters), 16);
        assert_eq!(mem::offset_of!(AvStream, private_data), 24);
        assert_eq!(mem::offset_of!(AvStream, time_base), 32);

        assert_eq!(mem::size_of::<AvFormatContext>(), 56);
        assert_eq!(mem::align_of::<AvFormatContext>(), 8);
        assert_eq!(mem::offset_of!(AvFormatContext, class), 0);
        assert_eq!(mem::offset_of!(AvFormatContext, input_format), 8);
        assert_eq!(mem::offset_of!(AvFormatContext, output_format), 16);
        assert_eq!(mem::offset_of!(AvFormatContext, private_data), 24);
        assert_eq!(mem::offset_of!(AvFormatContext, io_context), 32);
        assert_eq!(mem::offset_of!(AvFormatContext, context_flags), 40);
        assert_eq!(mem::offset_of!(AvFormatContext, stream_count), 44);
        assert_eq!(mem::offset_of!(AvFormatContext, streams), 48);

        assert_eq!(mem::size_of::<AvFormatContextMetadataProjection>(), 200);
        assert_eq!(mem::align_of::<AvFormatContextMetadataProjection>(), 8);
        assert_eq!(
            mem::offset_of!(AvFormatContextMetadataProjection, prefix),
            0
        );
        assert_eq!(
            mem::offset_of!(AvFormatContextMetadataProjection, metadata),
            192
        );

        assert_eq!(mem::size_of::<AvIoContext>(), 16);
        assert_eq!(mem::align_of::<AvIoContext>(), 8);
        assert_eq!(mem::offset_of!(AvIoContext, class), 0);
        assert_eq!(mem::offset_of!(AvIoContext, buffer), 8);

        assert_eq!(AV_NUM_DATA_POINTERS, 8);
        assert_eq!(AV_MEDIA_TYPE_VIDEO, 0);
        assert_eq!(AV_CODEC_ID_H264, 27);
        assert_eq!(AV_CODEC_ID_VP9, 167);
        assert_eq!(AV_CODEC_ID_AV1, 225);
        assert_eq!(AV_CODEC_ID_AAC, 86_018);
        assert_eq!(AV_PIXEL_FORMAT_YUV420P, 0);
        assert_eq!(AV_PIXEL_FORMAT_RGB24, 2);
        assert_eq!(AV_NO_PRESENTATION_TIMESTAMP, i64::MIN);
        assert_eq!(AV_SEEK_SIZE, 0x1_0000);
        assert_eq!(AV_SEEK_FORCE, 0x2_0000);
        assert_eq!(AV_FORMAT_FLAG_CUSTOM_IO, 0x0080);
        assert_eq!(AV_CODEC_FLAG_GLOBAL_HEADER, 1 << 22);
        assert_eq!(AV_OPTION_SEARCH_CHILDREN, 1);
        assert_eq!(SWS_BILINEAR, 2);
        assert_eq!(AV_ERROR_TRY_AGAIN, -11);
        assert_eq!(AV_ERROR_OUT_OF_MEMORY, -12);
        assert_eq!(AV_ERROR_INVALID_ARGUMENT, -22);
        assert_eq!(AV_ERROR_NO_SPACE, -28);
        assert_eq!(AV_ERROR_END_OF_FILE, -541_478_725);
        assert_eq!(AV_ERROR_EXIT, -1_414_092_869);
        assert_eq!(
            video_codec_library_contracts()
                .iter()
                .map(|(_, _, symbols)| symbols.len())
                .sum::<usize>(),
            54
        );
    }

    #[test]
    fn data_plane_manifest_matches_compiled_prefix_contract() {
        let manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../abi/video-codec/ffmpeg-7.1-x86_64-gnu-data-plane-v1.json"
        ))
        .expect("reviewed data-plane ABI manifest must be valid JSON");
        assert_eq!(
            manifest["source"]["archive_sha256"],
            FFMPEG_7_1_SOURCE_ARCHIVE_SHA256
        );
        assert_eq!(manifest["target"], "x86_64-unknown-linux-gnu");
        assert_eq!(
            manifest["source"]["signature_sha256"],
            "9bd1689dce76b109034dcc4765a406e84e8799a2fd857b000c0a4d9744b70617"
        );
        assert_eq!(
            manifest["source"]["headers"],
            serde_json::json!({
                "libavcodec/avcodec.h": {"bytes": 114986, "sha256": "d6dbc9694974237888592f71020092c4594511e762d609b0076a90e9696ad1b1"},
                "libavcodec/codec.h": {"bytes": 13314, "sha256": "681a5a4551b370e3e6b98ea0101aaf12e73419e96f1096a7377633a8d2ec1340"},
                "libavcodec/codec_id.h": {"bytes": 18054, "sha256": "de6a2924c58f83da84058fabc715b8838321518cbabc8458b15cb83d202fe14a"},
                "libavcodec/packet.h": {"bytes": 30025, "sha256": "219679e1ffe55fd22bc81b151fce8315372c87028fea87780be686f2b38f305f"},
                "libavformat/avformat.h": {"bytes": 119096, "sha256": "6171ac10e35a67fe04aa8ddfbf84263def2d7ae9159019dd71e5597be5dd3944"},
                "libavformat/avio.h": {"bytes": 31142, "sha256": "b1a25f1465b87b62bf2797870c39fbbf81d2e1b4513ac34a66d57963e8facb6f"},
                "libavutil/error.h": {"bytes": 5555, "sha256": "bcf4f7e69c7e0d658ad6e81611810f7cf1f0b8334ebe948d27e518c459e4104c"},
                "libavutil/frame.h": {"bytes": 41581, "sha256": "8218f0295206a6543e7d3974a6fcb1f22100a273b09234d0cb84c60cd1638e75"},
                "libavutil/mathematics.h": {"bytes": 9563, "sha256": "64fac2eb3a42fd3788f5585ac8e65c7d5cd82711730d1f030042ba0a62fe1a62"},
                "libavutil/opt.h": {"bytes": 47084, "sha256": "c6aec0aade9cb55696bd10a47294e43a38900208ba060d9f37e32348cf5f1fbf"},
                "libavutil/pixfmt.h": {"bytes": 41991, "sha256": "ace2ebcf84a382269c21ad7a050d69ab293b0dbceab350ac7f728a1b37dc0336"},
                "libavutil/version_major.h": {"bytes": 999, "sha256": "de46b2654c135c2d87631e955419a19f5ea315af9788d7aee08344215f198a50"},
                "libswscale/swscale.h": {"bytes": 16928, "sha256": "42ab58ed743efc74ba2152a049b521d41a0fae31595a9b9c6858742391570ca4"}
            })
        );
        assert_eq!(manifest["contract"]["symbol_count"], 54);
        assert_eq!(manifest["prefixes"]["AVFrame"]["size"], 144);
        assert_eq!(manifest["prefixes"]["AVFrame"]["alignment"], 8);
        assert_eq!(manifest["prefixes"]["AVPacket"]["size"], 72);
        assert_eq!(manifest["prefixes"]["AVPacket"]["alignment"], 8);
        assert_eq!(manifest["prefixes"]["AVStream"]["size"], 40);
        assert_eq!(manifest["prefixes"]["AVStream"]["alignment"], 8);
        assert_eq!(manifest["prefixes"]["AVFormatContext"]["size"], 56);
        assert_eq!(manifest["prefixes"]["AVFormatContext"]["alignment"], 8);
        assert_eq!(manifest["prefixes"]["AVIOContext"]["size"], 16);
        assert_eq!(manifest["prefixes"]["AVIOContext"]["alignment"], 8);
        assert_eq!(
            manifest["prefixes"]["AVFrame"]["offsets"],
            serde_json::json!({
                "data": 0, "linesize": 64, "extended_data": 96, "width": 104,
                "height": 108, "nb_samples": 112, "format": 116, "key_frame": 120,
                "pict_type": 124, "sample_aspect_ratio": 128, "pts": 136
            })
        );
        assert_eq!(
            manifest["prefixes"]["AVPacket"]["offsets"],
            serde_json::json!({
                "buf": 0, "pts": 8, "dts": 16, "data": 24, "size": 32,
                "stream_index": 36, "flags": 40, "side_data": 48,
                "side_data_elems": 56, "duration": 64
            })
        );
        assert_eq!(
            manifest["prefixes"]["AVStream"]["offsets"],
            serde_json::json!({
                "av_class": 0, "index": 8, "id": 12, "codecpar": 16,
                "priv_data": 24, "time_base": 32
            })
        );
        assert_eq!(
            manifest["prefixes"]["AVFormatContext"]["offsets"],
            serde_json::json!({
                "av_class": 0, "iformat": 8, "oformat": 16, "priv_data": 24,
                "pb": 32, "ctx_flags": 40, "nb_streams": 44, "streams": 48
            })
        );
        assert_eq!(
            manifest["prefixes"]["AVIOContext"]["offsets"],
            serde_json::json!({"av_class": 0, "buffer": 8})
        );
        assert_eq!(
            manifest["constants"],
            serde_json::json!({
                "AV_NUM_DATA_POINTERS": AV_NUM_DATA_POINTERS,
                "AVMEDIA_TYPE_VIDEO": AV_MEDIA_TYPE_VIDEO,
                "AV_CODEC_ID_H264": AV_CODEC_ID_H264,
                "AV_CODEC_ID_VP9": AV_CODEC_ID_VP9,
                "AV_CODEC_ID_AV1": AV_CODEC_ID_AV1,
                "AV_CODEC_ID_AAC": AV_CODEC_ID_AAC,
                "AV_PIX_FMT_YUV420P": AV_PIXEL_FORMAT_YUV420P,
                "AV_PIX_FMT_RGB24": AV_PIXEL_FORMAT_RGB24,
                "AV_NOPTS_VALUE": AV_NO_PRESENTATION_TIMESTAMP,
                "AVSEEK_SIZE": AV_SEEK_SIZE,
                "AVSEEK_FORCE": AV_SEEK_FORCE,
                "AVFMT_FLAG_CUSTOM_IO": AV_FORMAT_FLAG_CUSTOM_IO,
                "AV_CODEC_FLAG_GLOBAL_HEADER": AV_CODEC_FLAG_GLOBAL_HEADER,
                "AV_OPT_SEARCH_CHILDREN": AV_OPTION_SEARCH_CHILDREN,
                "SWS_BILINEAR": SWS_BILINEAR,
                "AVERROR_EAGAIN": AV_ERROR_TRY_AGAIN,
                "AVERROR_ENOMEM": AV_ERROR_OUT_OF_MEMORY,
                "AVERROR_EINVAL": AV_ERROR_INVALID_ARGUMENT,
                "AVERROR_ENOSPC": AV_ERROR_NO_SPACE,
                "AVERROR_EOF": AV_ERROR_END_OF_FILE,
                "AVERROR_EXIT": AV_ERROR_EXIT
            })
        );
    }

    #[test]
    fn ffmpeg_7_1_container_metadata_projection_is_exact() {
        assert_eq!(mem::size_of::<AvFormatContext>(), 56);
        assert_eq!(mem::offset_of!(AvFormatContext, streams), 48);
        assert_eq!(mem::size_of::<AvFormatContextMetadataProjection>(), 200);
        assert_eq!(mem::align_of::<AvFormatContextMetadataProjection>(), 8);
        assert_eq!(
            mem::offset_of!(AvFormatContextMetadataProjection, metadata),
            192
        );
    }

    #[test]
    fn container_metadata_manifest_matches_compiled_projection_contract() {
        let manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../abi/video-codec/ffmpeg-7.1-x86_64-gnu-container-metadata-v1.json"
        ))
        .expect("reviewed container metadata ABI manifest must be valid JSON");
        assert_eq!(
            manifest["source"]["archive_sha256"],
            FFMPEG_7_1_SOURCE_ARCHIVE_SHA256
        );
        assert_eq!(manifest["target"], "x86_64-unknown-linux-gnu");
        assert_eq!(manifest["contract"]["symbol_count"], 54);
        assert_eq!(
            manifest["projection"],
            serde_json::json!({
                "type": "AVFormatContext",
                "size": 200,
                "alignment": 8,
                "metadata_offset": 192,
                "historical_prefix_size": 56
            })
        );
    }
}
