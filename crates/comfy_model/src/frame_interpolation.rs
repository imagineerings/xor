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
    #[error("frame interpolation tensor error: {0}")]
    Tensor(#[from] TensorError),
    #[error("frame interpolation operation was cancelled")]
    Cancelled,
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
