use crate::{NativeVideoBitDepth, NativeVideoPayload};
use comfy_types::CancellationToken;
use thiserror::Error;

const SOURCE_RATE_DENOMINATOR: u64 = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeVideoContainer {
    Mp4,
    Webm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeVideoCodec {
    H264,
    Vp9,
    Av1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeVideoPixelFormat {
    Yuv420p,
    Yuv420p10le,
    Yuva420p,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeVideoAlphaPolicy {
    Discard,
    Preserve,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeVideoMetadataPolicy {
    Include,
    Exclude,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeVideoAudioLayout {
    Mono,
    Stereo,
    Surround51,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeVideoEncodeOptions {
    ComponentMp4 { metadata: NativeVideoMetadataPolicy },
    WebmVp9 { crf: u8 },
    WebmAv1 { crf: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeVideoCodecLimits {
    max_frames: u64,
    max_pixels_per_frame: u64,
    max_audio_samples: u64,
    max_encoded_bytes: u64,
}

impl NativeVideoCodecLimits {
    pub fn checked(
        max_frames: u64,
        max_pixels_per_frame: u64,
        max_audio_samples: u64,
        max_encoded_bytes: u64,
    ) -> Result<Self, NativeVideoCodecPlanError> {
        if max_frames == 0
            || max_pixels_per_frame == 0
            || max_audio_samples == 0
            || max_encoded_bytes == 0
        {
            return Err(NativeVideoCodecPlanError::InvalidLimits);
        }
        Ok(Self {
            max_frames,
            max_pixels_per_frame,
            max_audio_samples,
            max_encoded_bytes,
        })
    }

    pub const fn max_frames(self) -> u64 {
        self.max_frames
    }

    pub const fn max_pixels_per_frame(self) -> u64 {
        self.max_pixels_per_frame
    }

    pub const fn max_audio_samples(self) -> u64 {
        self.max_audio_samples
    }

    pub const fn max_encoded_bytes(self) -> u64 {
        self.max_encoded_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeVideoAudioPlan {
    layout: NativeVideoAudioLayout,
    input_channels: u64,
    sample_rate: u32,
    sample_count: u64,
}

impl NativeVideoAudioPlan {
    pub const fn layout(self) -> NativeVideoAudioLayout {
        self.layout
    }

    pub const fn input_channels(self) -> u64 {
        self.input_channels
    }

    pub const fn sample_rate(self) -> u32 {
        self.sample_rate
    }

    pub const fn sample_count(self) -> u64 {
        self.sample_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeVideoEncodePlan {
    container: NativeVideoContainer,
    codec: NativeVideoCodec,
    pixel_format: NativeVideoPixelFormat,
    source_bit_depth: NativeVideoBitDepth,
    output_bit_depth: NativeVideoBitDepth,
    source_frame_rate: (u64, u64),
    encode_frame_rate: (u64, u64),
    frame_count: u64,
    dimensions: (u64, u64),
    alpha: NativeVideoAlphaPolicy,
    audio: Option<NativeVideoAudioPlan>,
    metadata: NativeVideoMetadataPolicy,
    crf: Option<u8>,
    preset: Option<u8>,
    max_encoded_bytes: u64,
}

impl NativeVideoEncodePlan {
    pub const fn container(&self) -> NativeVideoContainer {
        self.container
    }

    pub const fn codec(&self) -> NativeVideoCodec {
        self.codec
    }

    pub const fn pixel_format(&self) -> NativeVideoPixelFormat {
        self.pixel_format
    }

    pub const fn source_bit_depth(&self) -> NativeVideoBitDepth {
        self.source_bit_depth
    }

    pub const fn output_bit_depth(&self) -> NativeVideoBitDepth {
        self.output_bit_depth
    }

    pub const fn source_frame_rate(&self) -> (u64, u64) {
        self.source_frame_rate
    }

    pub const fn encode_frame_rate(&self) -> (u64, u64) {
        self.encode_frame_rate
    }

    pub const fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub const fn dimensions(&self) -> (u64, u64) {
        self.dimensions
    }

    pub const fn alpha(&self) -> NativeVideoAlphaPolicy {
        self.alpha
    }

    pub const fn audio(&self) -> Option<NativeVideoAudioPlan> {
        self.audio
    }

    pub const fn metadata(&self) -> NativeVideoMetadataPolicy {
        self.metadata
    }

    pub const fn crf(&self) -> Option<u8> {
        self.crf
    }

    pub const fn preset(&self) -> Option<u8> {
        self.preset
    }

    pub const fn max_encoded_bytes(&self) -> u64 {
        self.max_encoded_bytes
    }
}

#[derive(Debug, Error)]
pub enum NativeVideoCodecPlanError {
    #[error("video codec planning was cancelled")]
    Cancelled,
    #[error("video component is invalid for the requested codec profile")]
    InvalidVideo,
    #[error("video codec options are invalid")]
    InvalidOptions,
    #[error("video codec limits must be nonzero")]
    InvalidLimits,
    #[error("video component exceeds the caller's codec limits")]
    LimitExceeded,
    #[error("video codec plan arithmetic overflowed")]
    Overflow,
}

pub fn plan_native_video_encode(
    video: &NativeVideoPayload,
    options: NativeVideoEncodeOptions,
    limits: NativeVideoCodecLimits,
    cancellation: &CancellationToken,
) -> Result<NativeVideoEncodePlan, NativeVideoCodecPlanError> {
    check_cancelled(cancellation)?;

    let frame_shape = video.frames().descriptor().shape();
    let [frame_count, height, width, channels] = frame_shape else {
        return Err(NativeVideoCodecPlanError::InvalidVideo);
    };
    if !matches!(channels, 3 | 4) {
        return Err(NativeVideoCodecPlanError::InvalidVideo);
    }
    let pixels_per_frame = height
        .checked_mul(*width)
        .ok_or(NativeVideoCodecPlanError::Overflow)?;
    if *frame_count > limits.max_frames || pixels_per_frame > limits.max_pixels_per_frame {
        return Err(NativeVideoCodecPlanError::LimitExceeded);
    }

    let source_frame_rate = video.frame_rate();
    let encode_frame_rate = rounded_millisecond_frame_rate(source_frame_rate)?;
    let has_alpha = *channels == 4 || video.alpha().is_some();
    let source_bit_depth = video.bit_depth();

    let (container, codec, pixel_format, output_bit_depth, alpha, audio, metadata, crf, preset) =
        match options {
            NativeVideoEncodeOptions::ComponentMp4 { metadata } => {
                let (pixel_format, output_bit_depth) = match source_bit_depth {
                    NativeVideoBitDepth::Eight => {
                        (NativeVideoPixelFormat::Yuv420p, NativeVideoBitDepth::Eight)
                    }
                    NativeVideoBitDepth::Ten => (
                        NativeVideoPixelFormat::Yuv420p10le,
                        NativeVideoBitDepth::Ten,
                    ),
                };
                let audio = plan_audio(video, *frame_count, source_frame_rate, limits)?;
                (
                    NativeVideoContainer::Mp4,
                    NativeVideoCodec::H264,
                    pixel_format,
                    output_bit_depth,
                    NativeVideoAlphaPolicy::Discard,
                    audio,
                    metadata,
                    None,
                    None,
                )
            }
            NativeVideoEncodeOptions::WebmVp9 { crf } => {
                require_webm_options(video, crf)?;
                (
                    NativeVideoContainer::Webm,
                    NativeVideoCodec::Vp9,
                    if has_alpha {
                        NativeVideoPixelFormat::Yuva420p
                    } else {
                        NativeVideoPixelFormat::Yuv420p
                    },
                    NativeVideoBitDepth::Eight,
                    if has_alpha {
                        NativeVideoAlphaPolicy::Preserve
                    } else {
                        NativeVideoAlphaPolicy::Discard
                    },
                    None,
                    NativeVideoMetadataPolicy::Include,
                    Some(crf),
                    None,
                )
            }
            NativeVideoEncodeOptions::WebmAv1 { crf } => {
                require_webm_options(video, crf)?;
                (
                    NativeVideoContainer::Webm,
                    NativeVideoCodec::Av1,
                    NativeVideoPixelFormat::Yuv420p10le,
                    NativeVideoBitDepth::Ten,
                    NativeVideoAlphaPolicy::Discard,
                    None,
                    NativeVideoMetadataPolicy::Include,
                    Some(crf),
                    Some(6),
                )
            }
        };

    check_cancelled(cancellation)?;
    Ok(NativeVideoEncodePlan {
        container,
        codec,
        pixel_format,
        source_bit_depth,
        output_bit_depth,
        source_frame_rate,
        encode_frame_rate,
        frame_count: *frame_count,
        dimensions: (*width, *height),
        alpha,
        audio,
        metadata,
        crf,
        preset,
        max_encoded_bytes: limits.max_encoded_bytes,
    })
}

fn plan_audio(
    video: &NativeVideoPayload,
    frame_count: u64,
    source_frame_rate: (u64, u64),
    limits: NativeVideoCodecLimits,
) -> Result<Option<NativeVideoAudioPlan>, NativeVideoCodecPlanError> {
    let Some(audio) = video.audio() else {
        return Ok(None);
    };
    let shape = audio.waveform().descriptor().shape();
    let [batch, channels, available_samples] = shape else {
        return Err(NativeVideoCodecPlanError::InvalidVideo);
    };
    if *batch != 1 {
        return Err(NativeVideoCodecPlanError::InvalidVideo);
    }
    let requested_numerator = u128::from(audio.sample_rate())
        .checked_mul(u128::from(frame_count))
        .and_then(|value| value.checked_mul(u128::from(source_frame_rate.1)))
        .ok_or(NativeVideoCodecPlanError::Overflow)?;
    let requested_samples = ceiling_divide(requested_numerator, u128::from(source_frame_rate.0))?;
    let requested_samples =
        u64::try_from(requested_samples).map_err(|_| NativeVideoCodecPlanError::Overflow)?;
    let sample_count = (*available_samples).min(requested_samples);
    if sample_count > limits.max_audio_samples {
        return Err(NativeVideoCodecPlanError::LimitExceeded);
    }
    let layout = match channels {
        1 => NativeVideoAudioLayout::Mono,
        2 => NativeVideoAudioLayout::Stereo,
        6 => NativeVideoAudioLayout::Surround51,
        _ => NativeVideoAudioLayout::Stereo,
    };
    Ok(Some(NativeVideoAudioPlan {
        layout,
        input_channels: *channels,
        sample_rate: audio.sample_rate(),
        sample_count,
    }))
}

fn require_webm_options(
    video: &NativeVideoPayload,
    crf: u8,
) -> Result<(), NativeVideoCodecPlanError> {
    if crf > 63 || video.audio().is_some() {
        return Err(NativeVideoCodecPlanError::InvalidOptions);
    }
    Ok(())
}

fn rounded_millisecond_frame_rate(
    source_frame_rate: (u64, u64),
) -> Result<(u64, u64), NativeVideoCodecPlanError> {
    let scaled_numerator = u128::from(source_frame_rate.0)
        .checked_mul(u128::from(SOURCE_RATE_DENOMINATOR))
        .ok_or(NativeVideoCodecPlanError::Overflow)?;
    let rounded = round_ratio_ties_even(scaled_numerator, u128::from(source_frame_rate.1))?;
    let rounded = u64::try_from(rounded).map_err(|_| NativeVideoCodecPlanError::Overflow)?;
    if rounded == 0 {
        return Err(NativeVideoCodecPlanError::InvalidVideo);
    }
    let divisor = greatest_common_divisor(rounded, SOURCE_RATE_DENOMINATOR);
    Ok((rounded / divisor, SOURCE_RATE_DENOMINATOR / divisor))
}

fn round_ratio_ties_even(
    numerator: u128,
    denominator: u128,
) -> Result<u128, NativeVideoCodecPlanError> {
    if denominator == 0 {
        return Err(NativeVideoCodecPlanError::InvalidVideo);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let doubled_remainder = remainder
        .checked_mul(2)
        .ok_or(NativeVideoCodecPlanError::Overflow)?;
    if doubled_remainder > denominator || (doubled_remainder == denominator && quotient % 2 == 1) {
        quotient
            .checked_add(1)
            .ok_or(NativeVideoCodecPlanError::Overflow)
    } else {
        Ok(quotient)
    }
}

fn ceiling_divide(numerator: u128, denominator: u128) -> Result<u128, NativeVideoCodecPlanError> {
    if denominator == 0 {
        return Err(NativeVideoCodecPlanError::InvalidVideo);
    }
    let quotient = numerator / denominator;
    if numerator.is_multiple_of(denominator) {
        Ok(quotient)
    } else {
        quotient
            .checked_add(1)
            .ok_or(NativeVideoCodecPlanError::Overflow)
    }
}

fn greatest_common_divisor(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn check_cancelled(cancellation: &CancellationToken) -> Result<(), NativeVideoCodecPlanError> {
    cancellation
        .check()
        .map_err(|_| NativeVideoCodecPlanError::Cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NativeAudioPayload;
    use comfy_tensor::{
        CpuWorkspaceAuthority, DType, DeviceId, ExecutionContext, StreamId, Tensor,
        TensorDescriptor,
    };
    use std::{collections::BTreeMap, error::Error};

    fn tensor(shape: Vec<u64>, dtype: DType, bytes: Vec<u8>) -> Result<Tensor, Box<dyn Error>> {
        let descriptor =
            TensorDescriptor::contiguous(shape, dtype, DeviceId::CPU, StreamId::DEFAULT)?;
        let byte_count = u64::try_from(bytes.len())?;
        let (backend, authority) =
            CpuWorkspaceAuthority::create_backend(byte_count.saturating_add(64))?;
        let cancellation = CancellationToken::default();
        let context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority.authorize_workspace(0)?,
            rng_phase: None,
            cancellation: &cancellation,
        };
        let (tensor, _) = backend.upload_bytes(descriptor, &bytes, &context)?;
        Ok(tensor)
    }

    fn f32_bytes(values: &[f32]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    fn video(
        channels: u64,
        bit_depth: NativeVideoBitDepth,
        audio: Option<NativeAudioPayload>,
        alpha: bool,
    ) -> Result<NativeVideoPayload, Box<dyn Error>> {
        let alpha = alpha
            .then(|| tensor(vec![3, 2, 2, 1], DType::F32, f32_bytes(&[0.5; 12])))
            .transpose()?;
        Ok(NativeVideoPayload::checked(
            tensor(
                vec![3, 2, 2, channels],
                DType::U8,
                vec![127; usize::try_from(12_u64.checked_mul(channels).ok_or("overflow")?)?],
            )?,
            1_054_475_631_502_295,
            35_184_372_088_832,
            bit_depth,
            audio,
            alpha,
            BTreeMap::from([("prompt".to_owned(), "fixture".to_owned())]),
        )?)
    }

    fn limits() -> Result<NativeVideoCodecLimits, NativeVideoCodecPlanError> {
        NativeVideoCodecLimits::checked(16, 64, 1_000_000, 8_000_000)
    }

    #[test]
    fn codec_plan_preserves_source_profiles_without_executing_a_codec() -> Result<(), Box<dyn Error>>
    {
        let audio = NativeAudioPayload::checked(
            tensor(vec![1, 6, 20_000], DType::F32, vec![0; 480_000])?,
            48_000,
        )?;
        let component = video(4, NativeVideoBitDepth::Ten, Some(audio), true)?;
        let digest_before = *component.semantic_digest_sha256();
        let plan = plan_native_video_encode(
            &component,
            NativeVideoEncodeOptions::ComponentMp4 {
                metadata: NativeVideoMetadataPolicy::Exclude,
            },
            limits()?,
            &CancellationToken::default(),
        )?;
        assert_eq!(plan.container(), NativeVideoContainer::Mp4);
        assert_eq!(plan.codec(), NativeVideoCodec::H264);
        assert_eq!(plan.pixel_format(), NativeVideoPixelFormat::Yuv420p10le);
        assert_eq!(plan.output_bit_depth(), NativeVideoBitDepth::Ten);
        assert_eq!(
            plan.source_frame_rate(),
            (1_054_475_631_502_295, 35_184_372_088_832)
        );
        assert_eq!(plan.encode_frame_rate(), (2_997, 100));
        assert_eq!(plan.alpha(), NativeVideoAlphaPolicy::Discard);
        let audio = plan.audio().ok_or("missing audio plan")?;
        assert_eq!(audio.layout(), NativeVideoAudioLayout::Surround51);
        assert_eq!(audio.input_channels(), 6);
        assert_eq!(audio.sample_count(), 4_805);
        assert_eq!(plan.metadata(), NativeVideoMetadataPolicy::Exclude);
        assert_eq!(component.semantic_digest_sha256(), &digest_before);

        let vp9 = plan_native_video_encode(
            &video(3, NativeVideoBitDepth::Ten, None, true)?,
            NativeVideoEncodeOptions::WebmVp9 { crf: 32 },
            limits()?,
            &CancellationToken::default(),
        )?;
        assert_eq!(vp9.codec(), NativeVideoCodec::Vp9);
        assert_eq!(vp9.pixel_format(), NativeVideoPixelFormat::Yuva420p);
        assert_eq!(vp9.output_bit_depth(), NativeVideoBitDepth::Eight);
        assert_eq!(vp9.alpha(), NativeVideoAlphaPolicy::Preserve);
        assert_eq!(vp9.crf(), Some(32));

        let av1 = plan_native_video_encode(
            &video(4, NativeVideoBitDepth::Eight, None, false)?,
            NativeVideoEncodeOptions::WebmAv1 { crf: 63 },
            limits()?,
            &CancellationToken::default(),
        )?;
        assert_eq!(av1.codec(), NativeVideoCodec::Av1);
        assert_eq!(av1.pixel_format(), NativeVideoPixelFormat::Yuv420p10le);
        assert_eq!(av1.output_bit_depth(), NativeVideoBitDepth::Ten);
        assert_eq!(av1.alpha(), NativeVideoAlphaPolicy::Discard);
        assert_eq!(av1.preset(), Some(6));
        Ok(())
    }

    #[test]
    fn codec_plan_rejects_unsupported_profiles_limits_and_cancellation()
    -> Result<(), Box<dyn Error>> {
        assert_eq!(round_ratio_ties_even(5, 2)?, 2);
        assert_eq!(round_ratio_ties_even(7, 2)?, 4);
        assert_eq!(
            rounded_millisecond_frame_rate((210_895_126_300_459, 8_796_093_022_208))?,
            (2_997, 125)
        );

        let grayscale = video(1, NativeVideoBitDepth::Eight, None, false)?;
        assert!(matches!(
            plan_native_video_encode(
                &grayscale,
                NativeVideoEncodeOptions::WebmVp9 { crf: 0 },
                limits()?,
                &CancellationToken::default(),
            ),
            Err(NativeVideoCodecPlanError::InvalidVideo)
        ));
        let audio =
            NativeAudioPayload::checked(tensor(vec![1, 1, 4], DType::F32, vec![0; 16])?, 48_000)?;
        let with_audio = video(3, NativeVideoBitDepth::Eight, Some(audio), false)?;
        assert!(matches!(
            plan_native_video_encode(
                &with_audio,
                NativeVideoEncodeOptions::WebmVp9 { crf: 64 },
                limits()?,
                &CancellationToken::default(),
            ),
            Err(NativeVideoCodecPlanError::InvalidOptions)
        ));
        assert!(matches!(
            plan_native_video_encode(
                &video(3, NativeVideoBitDepth::Eight, None, false)?,
                NativeVideoEncodeOptions::WebmAv1 { crf: 1 },
                NativeVideoCodecLimits::checked(2, 64, 10, 128)?,
                &CancellationToken::default(),
            ),
            Err(NativeVideoCodecPlanError::LimitExceeded)
        ));
        assert!(NativeVideoCodecLimits::checked(0, 1, 1, 1).is_err());

        let cancellation = CancellationToken::default();
        cancellation.cancel();
        assert!(matches!(
            plan_native_video_encode(
                &video(3, NativeVideoBitDepth::Eight, None, false)?,
                NativeVideoEncodeOptions::ComponentMp4 {
                    metadata: NativeVideoMetadataPolicy::Include,
                },
                limits()?,
                &cancellation,
            ),
            Err(NativeVideoCodecPlanError::Cancelled)
        ));
        Ok(())
    }
}
