use crate::{MetadataDocument, MetadataError, MetadataLimits, MetadataWritePolicy};
use comfy_tensor::{CpuBackend, ExecutionContext, TensorError};
use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageError, ImageFormat,
    ImageReader, Limits, codecs::png::PngEncoder,
};
use std::{collections::BTreeMap, io::Cursor};
use thiserror::Error;

pub const DEFAULT_MAX_PNG_BYTES: usize = 256 * 1024 * 1024;
pub const DEFAULT_MAX_PNG_DIMENSION: u32 = 32_768;
pub const DEFAULT_MAX_PNG_PIXELS: u64 = 268_435_456;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PngLimits {
    pub max_input_bytes: usize,
    pub max_dimension: u32,
    pub max_pixels: u64,
    pub max_decoder_allocation_bytes: u64,
}

impl Default for PngLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: DEFAULT_MAX_PNG_BYTES,
            max_dimension: DEFAULT_MAX_PNG_DIMENSION,
            max_pixels: DEFAULT_MAX_PNG_PIXELS,
            max_decoder_allocation_bytes: DEFAULT_MAX_PNG_BYTES as u64,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedPng {
    pub width: u32,
    pub height: u32,
    pub pixels_bhwc: Vec<f32>,
    pub mask_bhw: Vec<f32>,
    pub mask_width: u32,
    pub mask_height: u32,
    pub has_alpha: bool,
    pub metadata: MetadataDocument,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PngError {
    #[error("PNG input is {actual} bytes, exceeding the {limit}-byte limit")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("PNG dimensions {width}x{height} exceed configured limits")]
    Dimensions { width: u32, height: u32 },
    #[error("PNG pixel or byte size overflowed")]
    SizeOverflow,
    #[error("PNG IMAGE data has {actual} values, expected {expected}")]
    PixelCount { expected: usize, actual: usize },
    #[error("PNG batch index {index} is outside a batch of {batch}")]
    BatchIndex { index: u64, batch: u64 },
    #[error("PNG codec failed: {0}")]
    Codec(String),
    #[error("PNG metadata failed: {0}")]
    Metadata(String),
    #[error("PNG allocation failed: {0}")]
    Allocation(String),
    #[error(transparent)]
    Tensor(#[from] TensorError),
}

impl From<ImageError> for PngError {
    fn from(error: ImageError) -> Self {
        Self::Codec(error.to_string())
    }
}

impl From<MetadataError> for PngError {
    fn from(error: MetadataError) -> Self {
        Self::Metadata(error.to_string())
    }
}

pub fn decode_png(bytes: &[u8], limits: PngLimits) -> Result<DecodedPng, PngError> {
    if bytes.len() > limits.max_input_bytes {
        return Err(PngError::InputTooLarge {
            actual: bytes.len(),
            limit: limits.max_input_bytes,
        });
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    let mut decoder_limits = Limits::default();
    decoder_limits.max_image_width = Some(limits.max_dimension);
    decoder_limits.max_image_height = Some(limits.max_dimension);
    decoder_limits.max_alloc = Some(limits.max_decoder_allocation_bytes);
    reader.limits(decoder_limits);
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    let width = image.width();
    let height = image.height();
    validate_dimensions(width, height, limits)?;
    let has_alpha = image.color().has_alpha();
    let rgba = image.into_rgba8();
    let pixel_count = checked_pixel_count(width, height)?;
    let rgb_value_count = pixel_count.checked_mul(3).ok_or(PngError::SizeOverflow)?;
    let mut pixels_bhwc = Vec::new();
    pixels_bhwc
        .try_reserve_exact(rgb_value_count)
        .map_err(|error| PngError::Allocation(error.to_string()))?;
    let mask_value_count = if has_alpha { pixel_count } else { 64 * 64 };
    let mut mask_bhw = Vec::new();
    mask_bhw
        .try_reserve_exact(mask_value_count)
        .map_err(|error| PngError::Allocation(error.to_string()))?;
    for pixel in rgba.pixels() {
        pixels_bhwc.extend(pixel.0[..3].iter().map(|value| f32::from(*value) / 255.0));
        if has_alpha {
            mask_bhw.push(1.0 - f32::from(pixel.0[3]) / 255.0);
        }
    }
    if !has_alpha {
        mask_bhw.resize(mask_value_count, 0.0);
    }
    let metadata = MetadataDocument::parse(
        bytes,
        Some("image.png"),
        Some("image/png"),
        MetadataLimits {
            max_input_bytes: limits.max_input_bytes,
            ..MetadataLimits::default()
        },
    )?;
    Ok(DecodedPng {
        width,
        height,
        pixels_bhwc,
        mask_bhw,
        mask_width: if has_alpha { width } else { 64 },
        mask_height: if has_alpha { height } else { 64 },
        has_alpha,
        metadata,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn encode_png_frame(
    pixels_bhwc: &[f32],
    batch: u64,
    height: u64,
    width: u64,
    channels: u64,
    batch_index: u64,
    metadata: &BTreeMap<String, String>,
    limits: PngLimits,
) -> Result<Vec<u8>, PngError> {
    encode_png_frame_with_policy(
        pixels_bhwc,
        batch,
        height,
        width,
        channels,
        batch_index,
        metadata,
        MetadataWritePolicy::default(),
        limits,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_png_frame_with_policy(
    pixels_bhwc: &[f32],
    batch: u64,
    height: u64,
    width: u64,
    channels: u64,
    batch_index: u64,
    metadata: &BTreeMap<String, String>,
    metadata_policy: MetadataWritePolicy,
    limits: PngLimits,
) -> Result<Vec<u8>, PngError> {
    let (frame, width, height, values_per_frame) = checked_png_frame(
        pixels_bhwc,
        batch,
        height,
        width,
        channels,
        batch_index,
        limits,
    )?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(values_per_frame)
        .map_err(|error| PngError::Allocation(error.to_string()))?;
    pixels.extend(frame.iter().map(|value| quantize_channel(*value)));
    encode_pixels_with_metadata(
        &pixels,
        width,
        height,
        channels,
        metadata,
        metadata_policy,
        limits,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn encode_png_frame_with_policy_and_context(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    pixels_bhwc: &[f32],
    batch: u64,
    height: u64,
    width: u64,
    channels: u64,
    batch_index: u64,
    metadata: &BTreeMap<String, String>,
    metadata_policy: MetadataWritePolicy,
    limits: PngLimits,
) -> Result<Vec<u8>, PngError> {
    context.check()?;
    let (frame, width, height, values_per_frame) = checked_png_frame(
        pixels_bhwc,
        batch,
        height,
        width,
        channels,
        batch_index,
        limits,
    )?;
    let mut pixels = backend.workspace_vec::<u8>(context, values_per_frame)?;
    for (index, value) in frame.iter().copied().enumerate() {
        if index.is_multiple_of(4_096) {
            context.check()?;
        }
        pixels.try_push(quantize_channel(value))?;
    }
    encode_pixels_with_metadata(
        &pixels,
        width,
        height,
        channels,
        metadata,
        metadata_policy,
        limits,
    )
}

#[allow(clippy::too_many_arguments)]
fn checked_png_frame(
    pixels_bhwc: &[f32],
    batch: u64,
    height: u64,
    width: u64,
    channels: u64,
    batch_index: u64,
    limits: PngLimits,
) -> Result<(&[f32], u32, u32, usize), PngError> {
    if !matches!(channels, 3 | 4) {
        return Err(PngError::Dimensions {
            width: u32::try_from(width).unwrap_or(u32::MAX),
            height: u32::try_from(height).unwrap_or(u32::MAX),
        });
    }
    if batch_index >= batch {
        return Err(PngError::BatchIndex {
            index: batch_index,
            batch,
        });
    }
    let width = u32::try_from(width).map_err(|_| PngError::SizeOverflow)?;
    let height = u32::try_from(height).map_err(|_| PngError::SizeOverflow)?;
    validate_dimensions(width, height, limits)?;
    let values_per_frame = checked_pixel_count(width, height)?
        .checked_mul(usize::try_from(channels).map_err(|_| PngError::SizeOverflow)?)
        .ok_or(PngError::SizeOverflow)?;
    let expected = values_per_frame
        .checked_mul(usize::try_from(batch).map_err(|_| PngError::SizeOverflow)?)
        .ok_or(PngError::SizeOverflow)?;
    if pixels_bhwc.len() != expected {
        return Err(PngError::PixelCount {
            expected,
            actual: pixels_bhwc.len(),
        });
    }
    let frame_start = values_per_frame
        .checked_mul(usize::try_from(batch_index).map_err(|_| PngError::SizeOverflow)?)
        .ok_or(PngError::SizeOverflow)?;
    let frame_end = frame_start
        .checked_add(values_per_frame)
        .ok_or(PngError::SizeOverflow)?;
    let frame = pixels_bhwc
        .get(frame_start..frame_end)
        .ok_or(PngError::SizeOverflow)?;
    Ok((frame, width, height, values_per_frame))
}

fn quantize_channel(value: f32) -> u8 {
    let scaled = (value * 255.0).clamp(0.0, 255.0);
    scaled as u8
}

fn encode_pixels_with_metadata(
    pixels: &[u8],
    width: u32,
    height: u32,
    channels: u64,
    metadata: &BTreeMap<String, String>,
    metadata_policy: MetadataWritePolicy,
    limits: PngLimits,
) -> Result<Vec<u8>, PngError> {
    let mut encoded = Vec::new();
    let color_type = match channels {
        3 => ExtendedColorType::Rgb8,
        4 => ExtendedColorType::Rgba8,
        _ => return Err(PngError::Dimensions { width, height }),
    };
    PngEncoder::new(&mut encoded).write_image(pixels, width, height, color_type)?;
    if encoded.len() > limits.max_input_bytes {
        return Err(PngError::InputTooLarge {
            actual: encoded.len(),
            limit: limits.max_input_bytes,
        });
    }
    if metadata.is_empty() {
        return Ok(encoded);
    }
    let document = MetadataDocument::parse(
        &encoded,
        Some("image.png"),
        Some("image/png"),
        MetadataLimits {
            max_input_bytes: limits.max_input_bytes,
            ..MetadataLimits::default()
        },
    )?;
    Ok(document.embed_comfy_metadata(
        metadata,
        metadata_policy,
        &MetadataLimits {
            max_input_bytes: limits.max_input_bytes,
            ..MetadataLimits::default()
        },
    )?)
}

fn validate_dimensions(width: u32, height: u32, limits: PngLimits) -> Result<(), PngError> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(PngError::SizeOverflow)?;
    if width == 0
        || height == 0
        || width > limits.max_dimension
        || height > limits.max_dimension
        || pixels > limits.max_pixels
    {
        return Err(PngError::Dimensions { width, height });
    }
    Ok(())
}

fn checked_pixel_count(width: u32, height: u32) -> Result<usize, PngError> {
    usize::try_from(
        u64::from(width)
            .checked_mul(u64::from(height))
            .ok_or(PngError::SizeOverflow)?,
    )
    .map_err(|_| PngError::SizeOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use comfy_tensor::{CancellationToken, CpuWorkspaceAuthority, StreamId};
    use image::RgbaImage;

    #[test]
    fn native_png_encoding_preserves_rgba_preview_alpha() -> Result<(), Box<dyn std::error::Error>>
    {
        let encoded = encode_png_frame(
            &[1.0, 0.0, 0.5, 0.25],
            1,
            1,
            1,
            4,
            0,
            &BTreeMap::new(),
            PngLimits::default(),
        )?;
        let decoded = decode_png(&encoded, PngLimits::default())?;
        assert!(decoded.has_alpha);
        assert_eq!(decoded.pixels_bhwc, vec![1.0, 0.0, 127.0 / 255.0]);
        assert_eq!(decoded.mask_bhw, vec![1.0 - 63.0 / 255.0]);
        Ok(())
    }

    #[test]
    fn native_png_encoding_uses_exact_caller_workspace() -> Result<(), Box<dyn std::error::Error>> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let cancellation = CancellationToken::default();
        let exact = authority.authorize_workspace(6)?;
        let context = backend.execution_context(StreamId::DEFAULT, exact.clone(), &cancellation);
        let encoded = encode_png_frame_with_policy_and_context(
            &backend,
            &context,
            &[1.0, 0.0, 0.5, 0.0, 0.25, 1.0],
            1,
            1,
            2,
            3,
            0,
            &BTreeMap::new(),
            MetadataWritePolicy::default(),
            PngLimits::default(),
        )?;
        assert_eq!(decode_png(&encoded, PngLimits::default())?.width, 2);
        assert_eq!(exact.peak_bytes(), 6);
        assert_eq!(exact.in_use_bytes(), 0);

        let insufficient = authority.authorize_workspace(5)?;
        let insufficient_context =
            backend.execution_context(StreamId::DEFAULT, insufficient.clone(), &cancellation);
        assert!(matches!(
            encode_png_frame_with_policy_and_context(
                &backend,
                &insufficient_context,
                &[1.0, 0.0, 0.5, 0.0, 0.25, 1.0],
                1,
                1,
                2,
                3,
                0,
                &BTreeMap::new(),
                MetadataWritePolicy::default(),
                PngLimits::default(),
            ),
            Err(PngError::Tensor(
                TensorError::WorkspaceAuthorizationExceeded { .. }
            ))
        ));
        assert_eq!(insufficient.in_use_bytes(), 0);

        let cancelled = CancellationToken::default();
        assert!(cancelled.cancel());
        let cancelled_authorization = authority.authorize_workspace(6)?;
        let cancelled_context = backend.execution_context(
            StreamId::DEFAULT,
            cancelled_authorization.clone(),
            &cancelled,
        );
        assert!(matches!(
            encode_png_frame_with_policy_and_context(
                &backend,
                &cancelled_context,
                &[1.0, 0.0, 0.5, 0.0, 0.25, 1.0],
                1,
                1,
                2,
                3,
                0,
                &BTreeMap::new(),
                MetadataWritePolicy::default(),
                PngLimits::default(),
            ),
            Err(PngError::Tensor(TensorError::Cancelled))
        ));
        assert_eq!(cancelled_authorization.in_use_bytes(), 0);

        let (constrained_backend, constrained_authority) =
            CpuWorkspaceAuthority::create_backend(8)?;
        let oom_authorization = constrained_authority.authorize_workspace(6)?;
        let oom_context = constrained_backend.execution_context(
            StreamId::DEFAULT,
            oom_authorization.clone(),
            &cancellation,
        );
        assert!(matches!(
            encode_png_frame_with_policy_and_context(
                &constrained_backend,
                &oom_context,
                &[1.0, 0.0, 0.5, 0.0, 0.25, 1.0],
                1,
                1,
                2,
                3,
                0,
                &BTreeMap::new(),
                MetadataWritePolicy::default(),
                PngLimits::default(),
            ),
            Err(PngError::Tensor(TensorError::AllocationFailed { .. }))
        ));
        assert_eq!(oom_authorization.in_use_bytes(), 0);
        assert_eq!(constrained_backend.memory_snapshot().current_bytes, 0);
        Ok(())
    }

    #[test]
    fn rgba_decode_matches_comfy_rgb_and_inverted_alpha_mask() -> Result<(), PngError> {
        let source = RgbaImage::from_raw(2, 1, vec![255, 0, 128, 255, 0, 64, 255, 0])
            .ok_or(PngError::SizeOverflow)?;
        let mut bytes = Vec::new();
        DynamicImage::ImageRgba8(source)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)?;
        let decoded = decode_png(&bytes, PngLimits::default())?;
        assert_eq!((decoded.width, decoded.height), (2, 1));
        assert_eq!(
            decoded.pixels_bhwc,
            vec![1.0, 0.0, 128.0 / 255.0, 0.0, 64.0 / 255.0, 1.0]
        );
        assert_eq!(decoded.mask_bhw, vec![0.0, 1.0]);
        assert!(decoded.has_alpha);
        Ok(())
    }

    #[test]
    fn rgb_round_trip_embeds_comfy_metadata() -> Result<(), PngError> {
        let metadata = BTreeMap::from([
            ("prompt".to_owned(), "{\"1\":{}}".to_owned()),
            ("workflow".to_owned(), "{\"version\":0.4}".to_owned()),
        ]);
        let encoded = encode_png_frame(
            &[0.0, 0.5, 1.0, 1.0, 0.25, 0.0],
            1,
            1,
            2,
            3,
            0,
            &metadata,
            PngLimits::default(),
        )?;
        let decoded = decode_png(&encoded, PngLimits::default())?;
        assert_eq!(
            decoded.metadata.comfy_metadata().prompt,
            metadata.get("prompt").cloned()
        );
        assert_eq!(
            decoded.metadata.comfy_metadata().workflow,
            metadata.get("workflow").cloned()
        );
        assert_eq!(
            decoded.pixels_bhwc,
            vec![0.0, 127.0 / 255.0, 1.0, 1.0, 63.0 / 255.0, 0.0]
        );
        assert_eq!((decoded.mask_width, decoded.mask_height), (64, 64));
        assert!(decoded.mask_bhw.iter().all(|value| *value == 0.0));
        Ok(())
    }

    #[test]
    fn disabled_metadata_policy_omits_comfy_fields_without_changing_pixels() -> Result<(), PngError>
    {
        let pixels = [0.0, 0.5, 1.0];
        let metadata = BTreeMap::from([
            ("prompt".to_owned(), "{\"1\":{}}".to_owned()),
            ("workflow".to_owned(), "{\"version\":0.4}".to_owned()),
        ]);
        let encoded = encode_png_frame_with_policy(
            &pixels,
            1,
            1,
            1,
            3,
            0,
            &metadata,
            MetadataWritePolicy {
                metadata_enabled: false,
            },
            PngLimits::default(),
        )?;
        let decoded = decode_png(&encoded, PngLimits::default())?;
        assert_eq!(decoded.pixels_bhwc, [0.0, 127.0 / 255.0, 1.0]);
        assert_eq!(decoded.metadata.comfy_metadata(), Default::default());
        Ok(())
    }

    #[test]
    fn decode_rejects_limits_before_unbounded_allocation() {
        let error = decode_png(
            &[0; 9],
            PngLimits {
                max_input_bytes: 8,
                ..PngLimits::default()
            },
        );
        assert_eq!(
            error,
            Err(PngError::InputTooLarge {
                actual: 9,
                limit: 8,
            })
        );
    }
}
