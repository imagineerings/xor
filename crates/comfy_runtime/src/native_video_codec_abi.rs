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
opaque_ffi_type!(AvDictionary);
opaque_ffi_type!(AvFormatContext);
opaque_ffi_type!(AvFrame);
opaque_ffi_type!(AvInputFormat);
opaque_ffi_type!(AvIoContext);
opaque_ffi_type!(AvOutputFormat);
opaque_ffi_type!(AvPacket);
opaque_ffi_type!(AvStream);
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
}
