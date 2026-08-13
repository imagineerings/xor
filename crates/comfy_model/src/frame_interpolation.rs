use comfy_tensor::{CancellationToken, DType, DeviceId, StorageId, StreamId, Tensor, TensorError};
use comfy_types::CancellationError;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

const FILM_MARKER: &str = "extract.extract_sublevels.convs.0.0.conv.weight";
const FRAME_INTERPOLATION_SOURCE_SHA256: &str =
    "038762ff4e248c91e168685796f590a2e5aa0dc3b3c2922aa5f9d936b1fff369";
const FILM_SOURCE_SHA256: &str = "e4efa6666846cecb5dc83cb4668410b37b6c4ffae6b08e48b74184bc037c4ab1";
const RIFE_SOURCE_SHA256: &str = "854b808a425d01a82df2395cb925d7a5dab86669c62485f95fb790736ced11a3";
const MAXIMUM_INTERPOLATION_MULTIPLIER: u64 = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameInterpolationProfile {
    Film,
    Rife {
        head_channels: u64,
        block_channels: [u64; 5],
    },
}

impl FrameInterpolationProfile {
    pub const fn alignment(&self) -> u64 {
        match self {
            Self::Film => 1,
            Self::Rife { .. } => 64,
        }
    }

    pub const fn identifier(&self) -> &'static str {
        match self {
            Self::Film => "film",
            Self::Rife { .. } => "rife",
        }
    }
}

#[derive(Debug, Error)]
pub enum FrameInterpolationError {
    #[error("frame interpolation checkpoint format is unrecognized")]
    Unrecognized,
    #[error("frame interpolation checkpoint contains colliding normalized key `{0}`")]
    KeyCollision(String),
    #[error(
        "frame interpolation checkpoint tensor `{key}` has shape {actual:?}, expected {expected:?}"
    )]
    Shape {
        key: String,
        expected: Vec<u64>,
        actual: Vec<u64>,
    },
    #[error("frame interpolation checkpoint tensor set is incomplete or contains unexpected state")]
    StateMismatch,
    #[error("frame interpolation checkpoint tensors do not share supported CPU placement")]
    Placement,
    #[error("frame interpolation artifact digest is invalid")]
    ArtifactDigest,
    #[error("frame interpolation accounting overflow")]
    Overflow,
    #[error("frame interpolation invocation is invalid: {0}")]
    InvalidInvocation(&'static str),
    #[error("frame interpolation tensor error: {0}")]
    Tensor(#[from] TensorError),
    #[error("frame interpolation operation was cancelled")]
    Cancelled,
    #[cfg(any(test, feature = "test-support"))]
    #[error("frame interpolation test fixture failed: {0}")]
    TestFixture(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameInterpolationInvocationPlan {
    frame_count: u64,
    multiplier: u64,
    output_frame_count: u64,
    output_element_count: u64,
    total_interpolation_steps: u64,
    padded_height: u64,
    padded_width: u64,
    padding_bottom: u64,
    padding_right: u64,
    timesteps: Vec<f32>,
}

impl FrameInterpolationInvocationPlan {
    pub fn checked(
        profile: &FrameInterpolationProfile,
        frame_count: u64,
        multiplier: u64,
        height: u64,
        width: u64,
        cancellation: &CancellationToken,
    ) -> Result<Self, FrameInterpolationError> {
        cancellation.check()?;
        if height == 0 || width == 0 {
            return Err(FrameInterpolationError::InvalidInvocation(
                "frame extent is zero",
            ));
        }
        if multiplier > MAXIMUM_INTERPOLATION_MULTIPLIER {
            return Err(FrameInterpolationError::InvalidInvocation(
                "multiplier exceeds the source schema maximum",
            ));
        }
        let bypass = frame_count < 2 || multiplier < 2;
        let interpolation_count = if bypass { 0 } else { multiplier - 1 };
        let pair_count = frame_count.saturating_sub(1);
        let total_interpolation_steps = pair_count
            .checked_mul(interpolation_count)
            .ok_or(FrameInterpolationError::Overflow)?;
        let output_frame_count = if bypass {
            frame_count
        } else {
            pair_count
                .checked_mul(multiplier)
                .and_then(|count| count.checked_add(1))
                .ok_or(FrameInterpolationError::Overflow)?
        };
        let output_element_count = output_frame_count
            .checked_mul(3)
            .and_then(|count| count.checked_mul(height))
            .and_then(|count| count.checked_mul(width))
            .ok_or(FrameInterpolationError::Overflow)?;
        let alignment = profile.alignment();
        let padding_bottom = (alignment - height % alignment) % alignment;
        let padding_right = (alignment - width % alignment) % alignment;
        if alignment > 1 && (padding_bottom >= height || padding_right >= width) {
            return Err(FrameInterpolationError::InvalidInvocation(
                "reflection padding is not representable",
            ));
        }
        let padded_height = height
            .checked_add(padding_bottom)
            .ok_or(FrameInterpolationError::Overflow)?;
        let padded_width = width
            .checked_add(padding_right)
            .ok_or(FrameInterpolationError::Overflow)?;
        let timestep_capacity =
            usize::try_from(interpolation_count).map_err(|_| FrameInterpolationError::Overflow)?;
        let mut timesteps = Vec::new();
        timesteps
            .try_reserve_exact(timestep_capacity)
            .map_err(|_| FrameInterpolationError::Overflow)?;
        for index in 1..=interpolation_count {
            cancellation.check()?;
            let index = u16::try_from(index).map_err(|_| FrameInterpolationError::Overflow)?;
            let multiplier =
                u16::try_from(multiplier).map_err(|_| FrameInterpolationError::Overflow)?;
            timesteps.push(f32::from(index) / f32::from(multiplier));
        }
        Ok(Self {
            frame_count,
            multiplier,
            output_frame_count,
            output_element_count,
            total_interpolation_steps,
            padded_height,
            padded_width,
            padding_bottom,
            padding_right,
            timesteps,
        })
    }

    pub const fn is_bypass(&self) -> bool {
        self.frame_count < 2 || self.multiplier < 2
    }
    pub const fn frame_count(&self) -> u64 {
        self.frame_count
    }
    pub const fn multiplier(&self) -> u64 {
        self.multiplier
    }
    pub const fn output_frame_count(&self) -> u64 {
        self.output_frame_count
    }
    pub const fn output_element_count(&self) -> u64 {
        self.output_element_count
    }
    pub const fn total_interpolation_steps(&self) -> u64 {
        self.total_interpolation_steps
    }
    pub const fn padded_height(&self) -> u64 {
        self.padded_height
    }
    pub const fn padded_width(&self) -> u64 {
        self.padded_width
    }
    pub const fn padding_bottom(&self) -> u64 {
        self.padding_bottom
    }
    pub const fn padding_right(&self) -> u64 {
        self.padding_right
    }
    pub fn timesteps(&self) -> &[f32] {
        &self.timesteps
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameInterpolationFallbackState {
    multi_timestep_enabled: bool,
    single_timestep_batch: u64,
}

impl FrameInterpolationFallbackState {
    pub fn for_plan(
        plan: &FrameInterpolationInvocationPlan,
        multi_timestep_available: bool,
    ) -> Result<Self, FrameInterpolationError> {
        if plan.is_bypass() {
            return Ok(Self {
                multi_timestep_enabled: false,
                single_timestep_batch: 0,
            });
        }
        Ok(Self {
            multi_timestep_enabled: multi_timestep_available,
            single_timestep_batch: plan.multiplier - 1,
        })
    }

    pub const fn multi_timestep_enabled(&self) -> bool {
        self.multi_timestep_enabled
    }
    pub const fn single_timestep_batch(&self) -> u64 {
        self.single_timestep_batch
    }
    pub fn record_multi_timestep_oom(&mut self) {
        self.multi_timestep_enabled = false;
    }
    pub fn record_single_timestep_oom(&mut self) -> Result<(), FrameInterpolationError> {
        if self.single_timestep_batch <= 1 {
            return Err(FrameInterpolationError::InvalidInvocation(
                "single-timestep batch one exhausted memory",
            ));
        }
        self.single_timestep_batch = (self.single_timestep_batch / 2).max(1);
        Ok(())
    }
}

impl From<CancellationError> for FrameInterpolationError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

#[derive(Clone)]
pub struct NativeFrameInterpolationModel {
    profile: FrameInterpolationProfile,
    artifact_sha256: String,
    weights: BTreeMap<String, Tensor>,
    dtype: DType,
    stream: StreamId,
    semantic_state_digest_sha256: String,
}

impl NativeFrameInterpolationModel {
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn reduced_rife_test_fixture(
        backend: &comfy_tensor::CpuBackend,
        context: &comfy_tensor::ExecutionContext<'_>,
    ) -> Result<Self, FrameInterpolationError> {
        let manifest = rife_manifest(2, [2; 5])?;
        let mut weights = BTreeMap::new();
        let mut tensors_by_shape: BTreeMap<Vec<u64>, Tensor> = BTreeMap::new();
        for (key, shape) in manifest {
            context.cancellation.check()?;
            let tensor = if let Some(tensor) = tensors_by_shape.get(&shape) {
                tensor.clone()
            } else {
                let count = shape.iter().try_fold(1_usize, |total, dimension| {
                    total.checked_mul(usize::try_from(*dimension).ok()?)
                });
                let count = count.ok_or(FrameInterpolationError::Overflow)?;
                let tensor = crate::native_ops::tensor_from_f32(
                    backend,
                    &shape,
                    &vec![0.25; count],
                    DType::F32,
                    DeviceId::CPU,
                    context,
                )
                .map_err(|error| FrameInterpolationError::TestFixture(error.to_string()))?;
                tensors_by_shape.insert(shape.clone(), tensor.clone());
                tensor
            };
            weights.insert(key, tensor);
        }
        Self::from_checkpoint("f".repeat(64), weights, context.cancellation)
    }

    pub fn from_checkpoint(
        artifact_sha256: impl Into<String>,
        weights: BTreeMap<String, Tensor>,
        cancellation: &CancellationToken,
    ) -> Result<Self, FrameInterpolationError> {
        cancellation.check()?;
        let artifact_sha256 = artifact_sha256.into();
        if !valid_sha256(&artifact_sha256) {
            return Err(FrameInterpolationError::ArtifactDigest);
        }
        let (profile, weights) = normalize_and_detect(weights, cancellation)?;
        let manifest = weight_manifest(&profile)?;
        if weights.len() != manifest.len() {
            return Err(FrameInterpolationError::StateMismatch);
        }
        let first = weights
            .values()
            .next()
            .ok_or(FrameInterpolationError::StateMismatch)?;
        let dtype = first.descriptor().dtype();
        let stream = first.descriptor().stream();
        if !matches!(dtype, DType::F16 | DType::Bf16 | DType::F32) {
            return Err(FrameInterpolationError::Placement);
        }
        for (key, expected) in &manifest {
            cancellation.check()?;
            let tensor = weights
                .get(key)
                .ok_or(FrameInterpolationError::StateMismatch)?;
            if tensor.descriptor().shape() != expected {
                return Err(FrameInterpolationError::Shape {
                    key: key.clone(),
                    expected: expected.clone(),
                    actual: tensor.descriptor().shape().to_vec(),
                });
            }
            if tensor.descriptor().dtype() != dtype
                || tensor.descriptor().device() != DeviceId::CPU
                || tensor.descriptor().stream() != stream
                || !tensor.descriptor().is_contiguous()?
            {
                return Err(FrameInterpolationError::Placement);
            }
        }
        let semantic_state_digest_sha256 =
            semantic_digest(&profile, &artifact_sha256, &weights, cancellation)?;
        Ok(Self {
            profile,
            artifact_sha256,
            weights,
            dtype,
            stream,
            semantic_state_digest_sha256,
        })
    }

    pub fn profile(&self) -> &FrameInterpolationProfile {
        &self.profile
    }
    pub fn artifact_sha256(&self) -> &str {
        &self.artifact_sha256
    }
    pub fn dtype(&self) -> DType {
        self.dtype
    }
    pub fn stream(&self) -> StreamId {
        self.stream
    }
    pub fn semantic_state_digest_sha256(&self) -> &str {
        &self.semantic_state_digest_sha256
    }
    pub fn weight_count(&self) -> usize {
        self.weights.len()
    }

    pub fn validate(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<(), FrameInterpolationError> {
        cancellation.check()?;
        if !valid_sha256(&self.artifact_sha256) {
            return Err(FrameInterpolationError::ArtifactDigest);
        }
        if !matches!(self.dtype, DType::F16 | DType::Bf16 | DType::F32) {
            return Err(FrameInterpolationError::Placement);
        }
        let manifest = weight_manifest(&self.profile)?;
        if manifest.len() != self.weights.len() {
            return Err(FrameInterpolationError::StateMismatch);
        }
        for (key, expected) in manifest {
            cancellation.check()?;
            let tensor = self
                .weights
                .get(&key)
                .ok_or(FrameInterpolationError::StateMismatch)?;
            if tensor.descriptor().shape() != expected {
                return Err(FrameInterpolationError::Shape {
                    key,
                    expected,
                    actual: tensor.descriptor().shape().to_vec(),
                });
            }
            if tensor.descriptor().dtype() != self.dtype
                || tensor.descriptor().device() != DeviceId::CPU
                || tensor.descriptor().stream() != self.stream
                || !tensor.descriptor().is_contiguous()?
            {
                return Err(FrameInterpolationError::Placement);
            }
        }
        if semantic_digest(
            &self.profile,
            &self.artifact_sha256,
            &self.weights,
            cancellation,
        )? != self.semantic_state_digest_sha256
        {
            return Err(FrameInterpolationError::StateMismatch);
        }
        Ok(())
    }

    pub fn resident_tensor_allocations(
        &self,
    ) -> Result<Vec<(StorageId, u64)>, FrameInterpolationError> {
        let mut allocations = HashMap::new();
        for tensor in self.weights.values() {
            let storage = tensor.storage_id();
            let bytes = tensor.storage_byte_len();
            if let Some(existing) = allocations.insert(storage, bytes)
                && existing != bytes
            {
                return Err(FrameInterpolationError::StateMismatch);
            }
        }
        let mut allocations = allocations.into_iter().collect::<Vec<_>>();
        allocations.sort_unstable_by_key(|(storage, _)| storage.get());
        Ok(allocations)
    }

    pub fn resident_owned_bytes(&self) -> Result<u64, FrameInterpolationError> {
        let keys = self.weights.keys().try_fold(0usize, |total, key| {
            total
                .checked_add(key.capacity())
                .ok_or(FrameInterpolationError::Overflow)
        })?;
        let entries = self
            .weights
            .len()
            .checked_mul(std::mem::size_of::<(String, Tensor)>())
            .ok_or(FrameInterpolationError::Overflow)?;
        let bytes = std::mem::size_of::<Self>()
            .checked_add(keys)
            .and_then(|value| value.checked_add(entries))
            .and_then(|value| value.checked_add(self.artifact_sha256.capacity()))
            .and_then(|value| value.checked_add(self.semantic_state_digest_sha256.capacity()))
            .ok_or(FrameInterpolationError::Overflow)?;
        u64::try_from(bytes).map_err(|_| FrameInterpolationError::Overflow)
    }

    pub fn resident_bytes(&self) -> Result<u64, FrameInterpolationError> {
        self.resident_tensor_allocations()?.into_iter().try_fold(
            self.resident_owned_bytes()?,
            |total, (_, bytes)| {
                total
                    .checked_add(bytes)
                    .ok_or(FrameInterpolationError::Overflow)
            },
        )
    }
}

fn normalize_and_detect(
    weights: BTreeMap<String, Tensor>,
    cancellation: &CancellationToken,
) -> Result<(FrameInterpolationProfile, BTreeMap<String, Tensor>), FrameInterpolationError> {
    if weights.contains_key(FILM_MARKER) {
        return Ok((FrameInterpolationProfile::Film, weights));
    }
    let mut normalized = BTreeMap::new();
    for (key, tensor) in weights {
        cancellation.check()?;
        let key = key.strip_prefix("module.").unwrap_or(&key).to_owned();
        let key = key.strip_prefix("flownet.").unwrap_or(&key).to_owned();
        if key.starts_with("teacher.") || key.starts_with("caltime.") {
            continue;
        }
        let key = (0..5)
            .find_map(|index| {
                let prefix = format!("block{index}.");
                key.strip_prefix(&prefix)
                    .map(|suffix| format!("blocks.{index}.{suffix}"))
            })
            .unwrap_or(key);
        if normalized.insert(key.clone(), tensor).is_some() {
            return Err(FrameInterpolationError::KeyCollision(key));
        }
    }
    let head = normalized
        .get("encode.cnn3.weight")
        .ok_or(FrameInterpolationError::Unrecognized)?;
    let head_channels = *head
        .descriptor()
        .shape()
        .get(1)
        .ok_or(FrameInterpolationError::Unrecognized)?;
    if head.descriptor().shape().len() != 4 || head_channels == 0 {
        return Err(FrameInterpolationError::Unrecognized);
    }
    let mut block_channels = [0; 5];
    for (index, channel) in block_channels.iter_mut().enumerate() {
        let key = format!("blocks.{index}.conv0.1.0.weight");
        let tensor = normalized
            .get(&key)
            .ok_or(FrameInterpolationError::Unrecognized)?;
        *channel = *tensor
            .descriptor()
            .shape()
            .first()
            .ok_or(FrameInterpolationError::Unrecognized)?;
        if tensor.descriptor().shape().len() != 4 || *channel == 0 || !(*channel).is_multiple_of(2)
        {
            return Err(FrameInterpolationError::Unrecognized);
        }
    }
    Ok((
        FrameInterpolationProfile::Rife {
            head_channels,
            block_channels,
        },
        normalized,
    ))
}

fn weight_manifest(
    profile: &FrameInterpolationProfile,
) -> Result<BTreeMap<String, Vec<u64>>, FrameInterpolationError> {
    match profile {
        FrameInterpolationProfile::Film => film_manifest(),
        FrameInterpolationProfile::Rife {
            head_channels,
            block_channels,
        } => rife_manifest(*head_channels, *block_channels),
    }
}

fn insert_parameter(
    manifest: &mut BTreeMap<String, Vec<u64>>,
    prefix: String,
    weight: Vec<u64>,
    bias: u64,
) {
    manifest.insert(format!("{prefix}.weight"), weight);
    manifest.insert(format!("{prefix}.bias"), vec![bias]);
}

fn rife_manifest(
    head_channels: u64,
    channels: [u64; 5],
) -> Result<BTreeMap<String, Vec<u64>>, FrameInterpolationError> {
    let mut manifest = BTreeMap::new();
    insert_parameter(&mut manifest, "encode.cnn0".into(), vec![16, 3, 3, 3], 16);
    insert_parameter(&mut manifest, "encode.cnn1".into(), vec![16, 16, 3, 3], 16);
    insert_parameter(&mut manifest, "encode.cnn2".into(), vec![16, 16, 3, 3], 16);
    insert_parameter(
        &mut manifest,
        "encode.cnn3".into(),
        vec![16, head_channels, 4, 4],
        head_channels,
    );
    for (index, channel) in channels.into_iter().enumerate() {
        let input = if index == 0 {
            7_u64
                .checked_add(
                    head_channels
                        .checked_mul(2)
                        .ok_or(FrameInterpolationError::Overflow)?,
                )
                .ok_or(FrameInterpolationError::Overflow)?
        } else {
            20_u64
                .checked_add(
                    head_channels
                        .checked_mul(2)
                        .ok_or(FrameInterpolationError::Overflow)?,
                )
                .ok_or(FrameInterpolationError::Overflow)?
        };
        insert_parameter(
            &mut manifest,
            format!("blocks.{index}.conv0.0.0"),
            vec![channel / 2, input, 3, 3],
            channel / 2,
        );
        insert_parameter(
            &mut manifest,
            format!("blocks.{index}.conv0.1.0"),
            vec![channel, channel / 2, 3, 3],
            channel,
        );
        for residual in 0..8 {
            insert_parameter(
                &mut manifest,
                format!("blocks.{index}.convblock.{residual}.conv"),
                vec![channel, channel, 3, 3],
                channel,
            );
            manifest.insert(
                format!("blocks.{index}.convblock.{residual}.beta"),
                vec![1, channel, 1, 1],
            );
        }
        insert_parameter(
            &mut manifest,
            format!("blocks.{index}.lastconv.0"),
            vec![channel, 52, 4, 4],
            52,
        );
    }
    Ok(manifest)
}

fn film_manifest() -> Result<BTreeMap<String, Vec<u64>>, FrameInterpolationError> {
    let mut manifest = BTreeMap::new();
    let mut input = 3;
    for level in 0..4 {
        let output = 64_u64
            .checked_shl(u32::try_from(level).map_err(|_| FrameInterpolationError::Overflow)?)
            .ok_or(FrameInterpolationError::Overflow)?;
        insert_parameter(
            &mut manifest,
            format!("extract.extract_sublevels.convs.{level}.0.conv"),
            vec![output, input, 3, 3],
            output,
        );
        insert_parameter(
            &mut manifest,
            format!("extract.extract_sublevels.convs.{level}.1.conv"),
            vec![output, output, 3, 3],
            output,
        );
        input = output;
    }
    let predictors = [
        ("_predictor", 1920, 256),
        ("_predictors.0", 896, 128),
        ("_predictors.1", 384, 64),
        ("_predictors.2", 128, 32),
    ];
    for (name, input, filter) in predictors {
        for convolution in 0..3 {
            insert_parameter(
                &mut manifest,
                format!("predict_flow.{name}._convs.{convolution}.conv"),
                vec![filter, if convolution == 0 { input } else { filter }, 3, 3],
                filter,
            );
        }
        insert_parameter(
            &mut manifest,
            format!("predict_flow.{name}._convs.3.conv"),
            vec![filter / 2, filter, 1, 1],
            filter / 2,
        );
        insert_parameter(
            &mut manifest,
            format!("predict_flow.{name}._convs.4.conv"),
            vec![2, filter / 2, 1, 1],
            2,
        );
    }
    insert_parameter(
        &mut manifest,
        "fuse.output_conv".into(),
        vec![3, 64, 1, 1],
        3,
    );
    for (index, input, joined, output) in [
        (0, 1930, 2442, 512),
        (1, 512, 1162, 256),
        (2, 256, 522, 128),
        (3, 128, 202, 64),
    ] {
        insert_parameter(
            &mut manifest,
            format!("fuse.convs.{index}.0.conv"),
            vec![output, input, 2, 2],
            output,
        );
        insert_parameter(
            &mut manifest,
            format!("fuse.convs.{index}.1.conv"),
            vec![output, joined, 3, 3],
            output,
        );
        insert_parameter(
            &mut manifest,
            format!("fuse.convs.{index}.2.conv"),
            vec![output, output, 3, 3],
            output,
        );
    }
    Ok(manifest)
}

fn semantic_digest(
    profile: &FrameInterpolationProfile,
    artifact: &str,
    weights: &BTreeMap<String, Tensor>,
    cancellation: &CancellationToken,
) -> Result<String, FrameInterpolationError> {
    let mut hasher = Sha256::new();
    hasher.update(b"sim.comfy.frame-interpolation-model.v1");
    hasher.update(FRAME_INTERPOLATION_SOURCE_SHA256.as_bytes());
    match profile {
        FrameInterpolationProfile::Film => hasher.update(FILM_SOURCE_SHA256.as_bytes()),
        FrameInterpolationProfile::Rife {
            head_channels,
            block_channels,
        } => {
            hasher.update(RIFE_SOURCE_SHA256.as_bytes());
            hasher.update(head_channels.to_le_bytes());
            for channel in block_channels {
                hasher.update(channel.to_le_bytes());
            }
        }
    }
    hasher.update(artifact.as_bytes());
    hasher.update([
        match weights
            .values()
            .next()
            .map(|tensor| tensor.descriptor().dtype())
        {
            Some(DType::F16) => 0,
            Some(DType::Bf16) => 1,
            Some(DType::F32) => 2,
            _ => return Err(FrameInterpolationError::Placement),
        },
    ]);
    for (key, tensor) in weights {
        cancellation.check()?;
        hasher.update(
            u64::try_from(key.len())
                .map_err(|_| FrameInterpolationError::Overflow)?
                .to_le_bytes(),
        );
        hasher.update(key.as_bytes());
        for dimension in tensor.descriptor().shape() {
            hasher.update(dimension.to_le_bytes());
        }
        hasher.update(tensor.contiguous_bytes()?);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invocation_plans_order_midpoints_align_rife_and_persist_oom_downgrades()
    -> Result<(), FrameInterpolationError> {
        let cancellation = CancellationToken::default();
        let rife = FrameInterpolationProfile::Rife {
            head_channels: 4,
            block_channels: [192, 128, 96, 64, 32],
        };
        let plan = FrameInterpolationInvocationPlan::checked(&rife, 3, 4, 65, 66, &cancellation)?;
        assert!(!plan.is_bypass());
        assert_eq!(plan.output_frame_count(), 9);
        assert_eq!(plan.output_element_count(), 115_830);
        assert_eq!(plan.total_interpolation_steps(), 6);
        assert_eq!((plan.padded_height(), plan.padded_width()), (128, 128));
        assert_eq!((plan.padding_bottom(), plan.padding_right()), (63, 62));
        assert_eq!(plan.timesteps(), &[0.25, 0.5, 0.75]);

        let mut fallback = FrameInterpolationFallbackState::for_plan(&plan, true)?;
        assert!(fallback.multi_timestep_enabled());
        assert_eq!(fallback.single_timestep_batch(), 3);
        fallback.record_multi_timestep_oom();
        assert!(!fallback.multi_timestep_enabled());
        fallback.record_single_timestep_oom()?;
        assert_eq!(fallback.single_timestep_batch(), 1);
        assert!(matches!(
            fallback.record_single_timestep_oom(),
            Err(FrameInterpolationError::InvalidInvocation(_))
        ));

        let film = FrameInterpolationInvocationPlan::checked(
            &FrameInterpolationProfile::Film,
            5,
            2,
            65,
            66,
            &cancellation,
        )?;
        assert_eq!((film.padded_height(), film.padded_width()), (65, 66));
        assert_eq!(film.output_frame_count(), 9);
        assert_eq!(film.timesteps(), &[0.5]);
        Ok(())
    }

    #[test]
    fn invocation_plans_bypass_and_reject_unbounded_or_unreflectable_requests()
    -> Result<(), FrameInterpolationError> {
        let cancellation = CancellationToken::default();
        let rife = FrameInterpolationProfile::Rife {
            head_channels: 2,
            block_channels: [2; 5],
        };
        let bypass = FrameInterpolationInvocationPlan::checked(&rife, 1, 2, 64, 64, &cancellation)?;
        assert!(bypass.is_bypass());
        assert_eq!(bypass.output_frame_count(), 1);
        assert!(bypass.timesteps().is_empty());
        assert_eq!(
            FrameInterpolationFallbackState::for_plan(&bypass, true)?,
            FrameInterpolationFallbackState {
                multi_timestep_enabled: false,
                single_timestep_batch: 0,
            }
        );
        assert!(matches!(
            FrameInterpolationInvocationPlan::checked(&rife, 2, 17, 64, 64, &cancellation),
            Err(FrameInterpolationError::InvalidInvocation(_))
        ));
        assert!(matches!(
            FrameInterpolationInvocationPlan::checked(&rife, 2, 2, 1, 1, &cancellation),
            Err(FrameInterpolationError::InvalidInvocation(_))
        ));
        assert!(matches!(
            FrameInterpolationInvocationPlan::checked(
                &FrameInterpolationProfile::Film,
                2,
                16,
                u64::MAX,
                u64::MAX,
                &cancellation
            ),
            Err(FrameInterpolationError::Overflow)
        ));
        let cancelled = CancellationToken::default();
        cancelled.cancel();
        assert!(matches!(
            FrameInterpolationInvocationPlan::checked(&rife, 2, 2, 64, 64, &cancelled),
            Err(FrameInterpolationError::Cancelled)
        ));
        Ok(())
    }

    #[test]
    fn manifests_close_exact_source_tensor_counts() -> Result<(), FrameInterpolationError> {
        assert_eq!(film_manifest()?.len(), 82);
        assert_eq!(rife_manifest(4, [192, 128, 96, 64, 32])?.len(), 158);
        Ok(())
    }

    #[test]
    fn rife_normalization_is_sequential_filtered_and_collision_safe()
    -> Result<(), FrameInterpolationError> {
        use crate::native_ops::tensor_from_f32;
        use comfy_tensor::{CpuWorkspaceAuthority, ExecutionContext};

        let manifest = rife_manifest(4, [2, 2, 2, 2, 2])?;
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(1 << 20)
            .map_err(|_| FrameInterpolationError::Overflow)?;
        let cancellation = CancellationToken::default();
        let context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority
                .authorize_workspace(1 << 20)
                .map_err(|_| FrameInterpolationError::Overflow)?,
            rng_phase: None,
            cancellation: &cancellation,
        };
        let scalar = tensor_from_f32(&backend, &[1], &[1.0], DType::F32, DeviceId::CPU, &context)
            .map_err(|_| FrameInterpolationError::Overflow)?;
        let mut state = BTreeMap::new();
        for key in manifest.keys() {
            state.insert(format!("module.flownet.{key}"), scalar.clone());
        }
        let head = tensor_from_f32(
            &backend,
            &[16, 4, 1, 1],
            &vec![1.0; 64],
            DType::F32,
            DeviceId::CPU,
            &context,
        )
        .map_err(|_| FrameInterpolationError::Overflow)?;
        state.insert("module.flownet.encode.cnn3.weight".into(), head);
        for index in 0..5 {
            let detector = tensor_from_f32(
                &backend,
                &[2, 1, 1, 1],
                &[1.0, 1.0],
                DType::F32,
                DeviceId::CPU,
                &context,
            )
            .map_err(|_| FrameInterpolationError::Overflow)?;
            state.insert(
                format!("module.flownet.blocks.{index}.conv0.1.0.weight"),
                detector,
            );
        }
        state.insert("teacher.discarded".into(), scalar);
        let (profile, normalized) = normalize_and_detect(state, &cancellation)?;
        assert_eq!(
            profile,
            FrameInterpolationProfile::Rife {
                head_channels: 4,
                block_channels: [2; 5]
            }
        );
        assert!(!normalized.keys().any(|key| key.starts_with("teacher.")));
        Ok(())
    }

    #[test]
    fn reduced_rife_checkpoint_is_strict_content_bound_and_alias_aware()
    -> Result<(), FrameInterpolationError> {
        use crate::native_ops::tensor_from_f32;
        use comfy_tensor::{CpuWorkspaceAuthority, ExecutionContext};

        let (backend, authority) = CpuWorkspaceAuthority::create_backend(16 << 20)
            .map_err(|_| FrameInterpolationError::Overflow)?;
        let cancellation = CancellationToken::default();
        let context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority
                .authorize_workspace(4 << 20)
                .map_err(|_| FrameInterpolationError::Overflow)?,
            rng_phase: None,
            cancellation: &cancellation,
        };
        let manifest = rife_manifest(2, [2; 5])?;
        let mut state = BTreeMap::new();
        let mut tensors_by_shape: BTreeMap<Vec<u64>, Tensor> = BTreeMap::new();
        for (key, shape) in &manifest {
            let tensor = if let Some(tensor) = tensors_by_shape.get(shape) {
                tensor.clone()
            } else {
                let count = shape.iter().try_fold(1_usize, |total, dimension| {
                    total.checked_mul(usize::try_from(*dimension).ok()?)
                });
                let count = count.ok_or(FrameInterpolationError::Overflow)?;
                let tensor = tensor_from_f32(
                    &backend,
                    shape,
                    &vec![0.25; count],
                    DType::F32,
                    DeviceId::CPU,
                    &context,
                )
                .map_err(|_| FrameInterpolationError::Overflow)?;
                tensors_by_shape.insert(shape.clone(), tensor.clone());
                tensor
            };
            state.insert(key.clone(), tensor);
        }
        let model = NativeFrameInterpolationModel::from_checkpoint(
            "a".repeat(64),
            state.clone(),
            &cancellation,
        )?;
        assert_eq!(
            model.profile(),
            &FrameInterpolationProfile::Rife {
                head_channels: 2,
                block_channels: [2; 5],
            }
        );
        assert_eq!(model.weight_count(), 158);
        assert_eq!(model.profile().alignment(), 64);
        assert_eq!(model.semantic_state_digest_sha256().len(), 64);
        assert!(model.resident_tensor_allocations()?.len() < 158);
        assert!(
            model.resident_owned_bytes()?
                > std::mem::size_of::<NativeFrameInterpolationModel>() as u64
        );
        assert!(model.resident_bytes()? > model.resident_owned_bytes()?);
        let payload =
            crate::NativeModelPayload::frame_interpolation(std::sync::Arc::new(model.clone()))
                .map_err(|_| FrameInterpolationError::StateMismatch)?;
        assert_eq!(
            payload.identity().role(),
            crate::NativeModelResourceRole::FrameInterpolation
        );
        assert_eq!(
            payload.identity().identifier(),
            "native-frame-interpolation-rife"
        );
        assert_eq!(
            payload.identity().format(),
            "sim-native-frame-interpolation-v1"
        );
        assert!(payload.frame_interpolation_resource().is_some());
        assert!(payload.model().is_none());
        assert!(payload.clip().is_none());
        assert!(payload.vae().is_none());
        let parts = payload
            .resident_parts()
            .map_err(|_| FrameInterpolationError::StateMismatch)?;
        assert!(parts.backing_allocations().iter().any(|allocation| {
            allocation.kind() == crate::NativeModelBackingKind::NativeFrameInterpolationModel
        }));
        payload
            .validate()
            .map_err(|_| FrameInterpolationError::StateMismatch)?;

        let mut missing = state.clone();
        missing.remove("encode.cnn0.weight");
        assert!(matches!(
            NativeFrameInterpolationModel::from_checkpoint("a".repeat(64), missing, &cancellation),
            Err(FrameInterpolationError::StateMismatch)
        ));

        let changed_shape = vec![16, 3, 3, 3];
        let changed = tensor_from_f32(
            &backend,
            &changed_shape,
            &vec![0.5; 16 * 3 * 3 * 3],
            DType::F32,
            DeviceId::CPU,
            &context,
        )
        .map_err(|_| FrameInterpolationError::Overflow)?;
        state.insert("encode.cnn0.weight".into(), changed);
        let changed =
            NativeFrameInterpolationModel::from_checkpoint("a".repeat(64), state, &cancellation)?;
        assert_ne!(
            model.semantic_state_digest_sha256(),
            changed.semantic_state_digest_sha256()
        );

        let cancelled = CancellationToken::default();
        cancelled.cancel();
        assert!(matches!(
            NativeFrameInterpolationModel::from_checkpoint(
                "a".repeat(64),
                BTreeMap::new(),
                &cancelled
            ),
            Err(FrameInterpolationError::Cancelled)
        ));
        Ok(())
    }

    #[test]
    fn film_precedence_and_rife_normalization_collisions_fail_closed()
    -> Result<(), FrameInterpolationError> {
        use crate::native_ops::tensor_from_f32;
        use comfy_tensor::{CpuWorkspaceAuthority, ExecutionContext};

        let (backend, authority) = CpuWorkspaceAuthority::create_backend(1 << 20)
            .map_err(|_| FrameInterpolationError::Overflow)?;
        let cancellation = CancellationToken::default();
        let context = ExecutionContext {
            stream: StreamId::DEFAULT,
            scratch: authority
                .authorize_workspace(1 << 20)
                .map_err(|_| FrameInterpolationError::Overflow)?,
            rng_phase: None,
            cancellation: &cancellation,
        };
        let scalar = tensor_from_f32(&backend, &[1], &[1.0], DType::F32, DeviceId::CPU, &context)
            .map_err(|_| FrameInterpolationError::Overflow)?;

        let mut film = BTreeMap::new();
        film.insert(FILM_MARKER.into(), scalar.clone());
        film.insert("encode.cnn3.weight".into(), scalar.clone());
        let (profile, _) = normalize_and_detect(film, &cancellation)?;
        assert_eq!(profile, FrameInterpolationProfile::Film);

        let mut collision = BTreeMap::new();
        collision.insert("module.flownet.block0.duplicate".into(), scalar.clone());
        collision.insert("module.flownet.blocks.0.duplicate".into(), scalar);
        assert!(matches!(
            normalize_and_detect(collision, &cancellation),
            Err(FrameInterpolationError::KeyCollision(key)) if key == "blocks.0.duplicate"
        ));
        Ok(())
    }
}
