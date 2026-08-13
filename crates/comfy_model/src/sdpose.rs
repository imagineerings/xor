use comfy_media::{NativePoseKeypoint, NativePosePerson};
use comfy_tensor::{CpuBackend, ExecutionContext, TensorError};
use comfy_types::CancellationError;
use thiserror::Error;

pub const SDPOSE_HEATMAP_CHANNELS: usize = 133;
pub const SDPOSE_HEATMAP_HEIGHT: usize = 256;
pub const SDPOSE_HEATMAP_WIDTH: usize = 192;
pub const SDPOSE_INPUT_HEIGHT: f32 = 1024.0;
pub const SDPOSE_INPUT_WIDTH: f32 = 768.0;

const GAUSSIAN_RADIUS: isize = 5;
const GAUSSIAN_SIGMA: f32 = 2.0;
const OPENPOSE_KEYPOINTS: usize = 134;
const MMPOSE_INDICES: [usize; 15] = [17, 6, 8, 10, 7, 9, 12, 14, 16, 13, 15, 2, 1, 4, 3];
const OPENPOSE_INDICES: [usize; 15] = [1, 2, 3, 4, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SdPoseRawKeypoint {
    x: f32,
    y: f32,
    score: f32,
}

impl SdPoseRawKeypoint {
    pub fn checked(x: f32, y: f32, score: f32) -> Result<Self, SdPoseProjectionError> {
        if !x.is_finite() || !y.is_finite() || !score.is_finite() {
            return Err(SdPoseProjectionError::NonFiniteInput);
        }
        Ok(Self { x, y, score })
    }

    pub const fn x(self) -> f32 {
        self.x
    }

    pub const fn y(self) -> f32 {
        self.y
    }

    pub const fn score(self) -> f32 {
        self.score
    }
}

#[derive(Debug, Error)]
pub enum SdPoseProjectionError {
    #[error("SDPose heatmaps must have shape [batch, 133, 256, 192]")]
    InvalidHeatmapShape,
    #[error("SDPose projection received a non-finite value")]
    NonFiniteInput,
    #[error("SDPose DARK refinement encountered a singular Hessian")]
    SingularHessian,
    #[error("SDPose projection allocation failed: {0}")]
    AllocationFailed(String),
    #[error(transparent)]
    Cancellation(#[from] CancellationError),
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    Media(#[from] comfy_media::NativeMediaPayloadError),
}

pub fn decode_sdpose_heatmaps(
    heatmaps: &[f32],
    batch_size: usize,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<Vec<Vec<SdPoseRawKeypoint>>, SdPoseProjectionError> {
    context.check()?;
    let plane_length = SDPOSE_HEATMAP_HEIGHT
        .checked_mul(SDPOSE_HEATMAP_WIDTH)
        .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
    let expected = batch_size
        .checked_mul(SDPOSE_HEATMAP_CHANNELS)
        .and_then(|value| value.checked_mul(plane_length))
        .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
    if batch_size == 0 || heatmaps.len() != expected {
        return Err(SdPoseProjectionError::InvalidHeatmapShape);
    }
    if heatmaps.iter().any(|value| !value.is_finite()) {
        return Err(SdPoseProjectionError::NonFiniteInput);
    }

    let mut batches = Vec::new();
    batches
        .try_reserve_exact(batch_size)
        .map_err(|error| SdPoseProjectionError::AllocationFailed(error.to_string()))?;
    for batch_index in 0..batch_size {
        context.check()?;
        let mut points = Vec::new();
        points
            .try_reserve_exact(SDPOSE_HEATMAP_CHANNELS)
            .map_err(|error| SdPoseProjectionError::AllocationFailed(error.to_string()))?;
        for channel in 0..SDPOSE_HEATMAP_CHANNELS {
            context.check()?;
            let plane_index = batch_index
                .checked_mul(SDPOSE_HEATMAP_CHANNELS)
                .and_then(|value| value.checked_add(channel))
                .and_then(|value| value.checked_mul(plane_length))
                .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
            let plane_end = plane_index
                .checked_add(plane_length)
                .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
            let plane = heatmaps
                .get(plane_index..plane_end)
                .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
            points.push(decode_plane(plane, backend, context)?);
        }
        batches.push(points);
    }
    context.check()?;
    Ok(batches)
}

fn decode_plane(
    plane: &[f32],
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<SdPoseRawKeypoint, SdPoseProjectionError> {
    let (maximum_index, score) = plane.iter().copied().enumerate().fold(
        (0usize, f32::NEG_INFINITY),
        |current, candidate| {
            if candidate.1 > current.1 {
                candidate
            } else {
                current
            }
        },
    );
    let invalid = score <= 0.0;

    let maximum_y = maximum_index / SDPOSE_HEATMAP_WIDTH;
    let maximum_x = maximum_index % SDPOSE_HEATMAP_WIDTH;
    let radius =
        usize::try_from(GAUSSIAN_RADIUS).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?;
    let padded_height = SDPOSE_HEATMAP_HEIGHT + 2 * radius;
    let padded_width = SDPOSE_HEATMAP_WIDTH + 2 * radius;
    let padded_length = padded_height
        .checked_mul(padded_width)
        .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
    let mut horizontal = backend.workspace_vec::<f32>(context, padded_length)?;
    let mut blurred = backend.workspace_vec::<f32>(context, padded_length)?;
    for _ in 0..padded_length {
        horizontal.try_push(0.0)?;
        blurred.try_push(0.0)?;
    }

    for y in 0..SDPOSE_HEATMAP_HEIGHT {
        context.check()?;
        for x in 0..SDPOSE_HEATMAP_WIDTH {
            let source = y * SDPOSE_HEATMAP_WIDTH + x;
            let destination = (y + radius) * padded_width + x + radius;
            blurred[destination] = plane[source];
        }
    }
    let kernel = gaussian_kernel()?;
    for y in 0..padded_height {
        context.check()?;
        for x in 0..padded_width {
            let mut value = 0.0f32;
            for (kernel_index, weight) in kernel.iter().copied().enumerate() {
                let delta = isize::try_from(kernel_index)
                    .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?
                    - GAUSSIAN_RADIUS;
                if let Some(source_x) = x
                    .checked_add_signed(delta)
                    .filter(|value| *value < padded_width)
                {
                    value += blurred[y * padded_width + source_x] * weight;
                }
            }
            horizontal[y * padded_width + x] = value;
        }
    }
    for y in 0..padded_height {
        context.check()?;
        for x in 0..padded_width {
            let mut value = 0.0f32;
            for (kernel_index, weight) in kernel.iter().copied().enumerate() {
                let delta = isize::try_from(kernel_index)
                    .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?
                    - GAUSSIAN_RADIUS;
                if let Some(source_y) = y
                    .checked_add_signed(delta)
                    .filter(|value| *value < padded_height)
                {
                    value += horizontal[source_y * padded_width + x] * weight;
                }
            }
            blurred[y * padded_width + x] = value;
        }
    }

    let mut current_maximum = f32::NEG_INFINITY;
    for y in 0..SDPOSE_HEATMAP_HEIGHT {
        for x in 0..SDPOSE_HEATMAP_WIDTH {
            current_maximum =
                current_maximum.max(blurred[(y + radius) * padded_width + x + radius]);
        }
    }
    if current_maximum > 0.0 {
        let scale = score / current_maximum;
        for y in 0..SDPOSE_HEATMAP_HEIGHT {
            context.check()?;
            for x in 0..SDPOSE_HEATMAP_WIDTH {
                let index = (y + radius) * padded_width + x + radius;
                blurred[index] *= scale;
            }
        }
    }
    for y in 0..SDPOSE_HEATMAP_HEIGHT {
        context.check()?;
        for x in 0..SDPOSE_HEATMAP_WIDTH {
            let index = (y + radius) * padded_width + x + radius;
            blurred[index] = blurred[index].clamp(1.0e-3, 50.0).ln();
        }
    }

    let sample = |x: isize, y: isize| -> Result<f32, SdPoseProjectionError> {
        let clamped_x = x.clamp(
            0,
            isize::try_from(SDPOSE_HEATMAP_WIDTH - 1)
                .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
        );
        let clamped_y = y.clamp(
            0,
            isize::try_from(SDPOSE_HEATMAP_HEIGHT - 1)
                .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
        );
        let source_x = usize::try_from(clamped_x)
            .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?
            + radius;
        let source_y = usize::try_from(clamped_y)
            .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?
            + radius;
        Ok(blurred[source_y * padded_width + source_x])
    };
    let x = isize::try_from(maximum_x).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?;
    let y = isize::try_from(maximum_y).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?;
    let center = sample(x, y)?;
    let right = sample(x + 1, y)?;
    let left = sample(x - 1, y)?;
    let down = sample(x, y + 1)?;
    let up = sample(x, y - 1)?;
    let down_right = sample(x + 1, y + 1)?;
    let up_left = sample(x - 1, y - 1)?;
    let derivative_x = 0.5 * (right - left);
    let derivative_y = 0.5 * (down - up);
    let hessian_xx = right - 2.0 * center + left + f32::EPSILON;
    let hessian_yy = down - 2.0 * center + up + f32::EPSILON;
    let hessian_xy = 0.5 * (down_right - right - down + 2.0 * center - left - up + up_left);
    let correction = checked_hessian_correction(
        hessian_xx,
        hessian_xy,
        hessian_yy,
        derivative_x,
        derivative_y,
    )?;
    let maximum_x = f32::from(
        u16::try_from(maximum_x).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
    );
    let maximum_y = f32::from(
        u16::try_from(maximum_y).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
    );
    let refined_x = maximum_x - correction[0];
    let refined_y = maximum_y - correction[1];
    let heatmap_width = f32::from(
        u16::try_from(SDPOSE_HEATMAP_WIDTH)
            .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
    );
    let heatmap_height = f32::from(
        u16::try_from(SDPOSE_HEATMAP_HEIGHT)
            .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
    );
    let scale_x = (SDPOSE_INPUT_WIDTH - 1.0) / (heatmap_width - 1.0);
    let scale_y = (SDPOSE_INPUT_HEIGHT - 1.0) / (heatmap_height - 1.0);
    if invalid {
        SdPoseRawKeypoint::checked(-1.0, -1.0, score)
    } else {
        SdPoseRawKeypoint::checked(refined_x * scale_x, refined_y * scale_y, score)
    }
}

fn checked_hessian_correction(
    hessian_xx: f32,
    hessian_xy: f32,
    hessian_yy: f32,
    derivative_x: f32,
    derivative_y: f32,
) -> Result<[f32; 2], SdPoseProjectionError> {
    let determinant = hessian_xx * hessian_yy - hessian_xy * hessian_xy;
    if !determinant.is_finite() || determinant == 0.0 {
        return Err(SdPoseProjectionError::SingularHessian);
    }
    let inverse_xx = hessian_yy / determinant;
    let inverse_xy = -hessian_xy / determinant;
    let inverse_yy = hessian_xx / determinant;
    Ok([
        inverse_xx * derivative_x + inverse_xy * derivative_y,
        inverse_xy * derivative_x + inverse_yy * derivative_y,
    ])
}

fn gaussian_kernel() -> Result<[f32; 11], SdPoseProjectionError> {
    let mut kernel = [0.0; 11];
    let mut total = 0.0f32;
    let radius = f32::from(
        i16::try_from(GAUSSIAN_RADIUS).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
    );
    for (index, value) in kernel.iter_mut().enumerate() {
        let index = f32::from(
            u16::try_from(index).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
        );
        let distance = index - radius;
        let weight = (-0.5 * (distance / GAUSSIAN_SIGMA).powi(2)).exp();
        *value = weight;
        total += weight;
    }
    for value in &mut kernel {
        *value /= total;
    }
    Ok(kernel)
}

pub fn project_sdpose_openpose_person(
    raw: &[SdPoseRawKeypoint],
) -> Result<NativePosePerson, SdPoseProjectionError> {
    if raw.len() != SDPOSE_HEATMAP_CHANNELS {
        return Err(SdPoseProjectionError::InvalidHeatmapShape);
    }
    let mut points = Vec::new();
    points
        .try_reserve_exact(OPENPOSE_KEYPOINTS)
        .map_err(|error| SdPoseProjectionError::AllocationFailed(error.to_string()))?;
    points.extend_from_slice(&raw[..17]);
    let left_shoulder = raw[5];
    let right_shoulder = raw[6];
    points.push(SdPoseRawKeypoint::checked(
        (left_shoulder.x + right_shoulder.x) * 0.5,
        (left_shoulder.y + right_shoulder.y) * 0.5,
        if left_shoulder.score > 0.3 && right_shoulder.score > 0.3 {
            left_shoulder.score.min(right_shoulder.score)
        } else {
            0.0
        },
    )?);
    points.extend_from_slice(&raw[17..]);
    let original = points.clone();
    for (&source, &destination) in MMPOSE_INDICES.iter().zip(OPENPOSE_INDICES.iter()) {
        points[destination] = original[source];
    }

    let convert = |point: SdPoseRawKeypoint| {
        NativePoseKeypoint::checked(point.x.into(), point.y.into(), point.score.into())
    };
    let collect = |slice: &[SdPoseRawKeypoint]| {
        slice
            .iter()
            .copied()
            .map(convert)
            .collect::<Result<Vec<_>, _>>()
    };
    let mut face = collect(&points[24..92])?;
    face.try_reserve_exact(2)
        .map_err(|error| SdPoseProjectionError::AllocationFailed(error.to_string()))?;
    face.push(convert(points[14])?);
    face.push(convert(points[15])?);
    Ok(NativePosePerson::checked(
        collect(&points[0..18])?,
        collect(&points[18..24])?,
        face,
        collect(&points[92..113])?,
        collect(&points[113..134])?,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singular_hessian_is_typed() {
        assert!(matches!(
            checked_hessian_correction(f32::EPSILON, f32::EPSILON, f32::EPSILON, 1.0, 1.0),
            Err(SdPoseProjectionError::SingularHessian)
        ));
    }
}
