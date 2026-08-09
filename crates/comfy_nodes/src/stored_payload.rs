use crate::{
    NativeHandleKind, NativeHandleType, NativeNodeContractError, native_source_type_projection,
};
use comfy_media::{
    NativeBoundingBoxPayload, NativeFaceLandmarksPayload, NativePoseKeypointPayload,
    NativeSam3TrackDataPayload, NativeTracksPayload,
};
use comfy_model::{
    AudioEncoderOutput, IcLoraParameters, LossMap, NativeModelBackingKind, NativeModelPayload,
    clip_vision::ClipVisionOutput,
    conditioning::{ConditioningError, ConditioningSet},
};
use comfy_sampler::{
    NativeControlPayload, NativeDiffusionPayload, NativeGuiderConditioningSets,
    NativeGuiderPayload, NativeNoisePayload, NativeSamplerPayload,
};
use comfy_tensor::{NativeTensorPayload, NativeTensorRole};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, mem, sync::Arc};
use thiserror::Error;

const MAX_PROVIDER_NAMESPACE_BYTES: usize = 4_096;
const MAX_PROVIDER_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone)]
pub struct NativeProviderPayload {
    handle_type: NativeHandleType,
    signed_namespace: String,
    semantic_digest_sha256: String,
    abi_bytes: Vec<u8>,
    abi_digest_sha256: String,
}

impl NativeProviderPayload {
    pub fn checked(
        handle_type: NativeHandleType,
        signed_namespace: impl Into<String>,
        semantic_digest_sha256: impl Into<String>,
        abi_bytes: Vec<u8>,
    ) -> Result<Self, NativeStoredPayloadError> {
        let payload = Self {
            handle_type,
            signed_namespace: signed_namespace.into(),
            semantic_digest_sha256: semantic_digest_sha256.into(),
            abi_digest_sha256: format!("{:x}", Sha256::digest(&abi_bytes)),
            abi_bytes,
        };
        payload.validate()?;
        Ok(payload)
    }

    pub fn handle_type(&self) -> &NativeHandleType {
        &self.handle_type
    }

    pub fn signed_namespace(&self) -> &str {
        &self.signed_namespace
    }

    pub fn semantic_digest_sha256(&self) -> &str {
        &self.semantic_digest_sha256
    }

    pub fn identity_digest_sha256(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"sim.comfy.provider-payload-identity.v1");
        hasher.update([0]);
        hasher.update([native_handle_kind_tag(self.handle_type.kind)]);
        hasher.update([0]);
        hasher.update(self.handle_type.type_id.as_bytes());
        hasher.update([0]);
        hasher.update(self.signed_namespace.as_bytes());
        hasher.update([0]);
        hasher.update(self.semantic_digest_sha256.as_bytes());
        hasher.update([0]);
        hasher.update(self.abi_digest_sha256.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn abi_bytes(&self) -> &[u8] {
        &self.abi_bytes
    }

    pub fn resident_bytes(&self) -> Result<usize, NativeStoredPayloadError> {
        mem::size_of::<Self>()
            .checked_add(self.handle_type.type_id.capacity())
            .and_then(|bytes| bytes.checked_add(self.signed_namespace.capacity()))
            .and_then(|bytes| bytes.checked_add(self.semantic_digest_sha256.capacity()))
            .and_then(|bytes| bytes.checked_add(self.abi_bytes.capacity()))
            .and_then(|bytes| bytes.checked_add(self.abi_digest_sha256.capacity()))
            .ok_or(NativeStoredPayloadError::ResidentBytesOverflow)
    }

    pub fn validate(&self) -> Result<(), NativeStoredPayloadError> {
        self.handle_type.validate()?;
        if self.handle_type.kind != NativeHandleKind::ProviderTask
            || self.signed_namespace.is_empty()
            || self.signed_namespace.len() > MAX_PROVIDER_NAMESPACE_BYTES
            || self.signed_namespace.chars().any(char::is_control)
            || self.abi_bytes.len() > MAX_PROVIDER_PAYLOAD_BYTES
            || !valid_sha256(&self.semantic_digest_sha256)
            || !valid_sha256(&self.abi_digest_sha256)
            || self.abi_digest_sha256 != format!("{:x}", Sha256::digest(&self.abi_bytes))
        {
            return Err(NativeStoredPayloadError::InvalidProviderPayload);
        }
        self.resident_bytes()?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct NativeStoredModelPayload {
    diffusion: Arc<NativeDiffusionPayload>,
}

impl NativeStoredModelPayload {
    pub fn native_diffusion(
        diffusion: Arc<NativeDiffusionPayload>,
    ) -> Result<Self, NativeStoredPayloadError> {
        let payload = Self { diffusion };
        payload.validate()?;
        Ok(payload)
    }

    pub fn diffusion(&self) -> &Arc<NativeDiffusionPayload> {
        &self.diffusion
    }

    pub fn validate(&self) -> Result<(), NativeStoredPayloadError> {
        self.diffusion.validate()?;
        Ok(())
    }

    pub fn handle_type(&self) -> Result<NativeHandleType, NativeStoredPayloadError> {
        let source_type = self.diffusion.role().source_type_id();
        native_source_type_projection(source_type)?
            .handle_type()?
            .ok_or_else(|| NativeStoredPayloadError::MissingHandleType(source_type.to_owned()))
    }

    pub fn digest_sha256(&self) -> &str {
        self.diffusion.digest_sha256()
    }

    pub fn resident_bytes(&self) -> Result<usize, NativeStoredPayloadError> {
        Ok(self.diffusion.resident_bytes()?)
    }
}

#[derive(Clone)]
pub enum NativeStoredPayload {
    Tensor(Arc<NativeTensorPayload>),
    Model(Arc<NativeStoredModelPayload>),
    Control(Arc<NativeControlPayload>),
    Conditioning(Arc<ConditioningSet>),
    Noise(Arc<NativeNoisePayload>),
    Guider(Arc<NativeGuiderPayload>),
    Sampler(Arc<NativeSamplerPayload>),
    BoundingBox(Arc<NativeBoundingBoxPayload>),
    FaceLandmarks(Arc<NativeFaceLandmarksPayload>),
    PoseKeypoint(Arc<NativePoseKeypointPayload>),
    Sam3TrackData(Arc<NativeSam3TrackDataPayload>),
    Tracks(Arc<NativeTracksPayload>),
    AudioEncoderOutput(Arc<AudioEncoderOutput>),
    ClipVisionOutput(Arc<ClipVisionOutput>),
    IcLoraParameters(Arc<IcLoraParameters>),
    LossMap(Arc<LossMap>),
    Provider(Arc<NativeProviderPayload>),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NativeResidentPayloadKind {
    Model,
    Control,
    Noise,
    Guider,
    Sampler,
    BoundingBox,
    FaceLandmarks,
    PoseKeypoint,
    Sam3TrackData,
    Tracks,
    AudioEncoderOutput,
    ClipVisionOutput,
    IcLoraParameters,
    LossMap,
    Provider,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NativeResidentAllocationId {
    PayloadArc {
        kind: NativeResidentPayloadKind,
        address: usize,
    },
    ModelPayloadArc {
        address: usize,
    },
    ModelBacking {
        kind: NativeModelBackingKind,
        address: usize,
    },
    ConditioningSetArc {
        address: usize,
    },
    TensorStorage {
        storage_id: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeResidentAllocation {
    id: NativeResidentAllocationId,
    resident_bytes: usize,
}

impl NativeResidentAllocation {
    pub fn id(&self) -> &NativeResidentAllocationId {
        &self.id
    }

    pub const fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativePayloadResidency {
    exclusive_bytes: usize,
    shared_allocations: Vec<NativeResidentAllocation>,
}

impl NativePayloadResidency {
    fn checked(
        exclusive_bytes: usize,
        allocations: impl IntoIterator<Item = NativeResidentAllocation>,
    ) -> Result<Self, NativeStoredPayloadError> {
        let mut unique = BTreeMap::new();
        for allocation in allocations {
            if let Some(existing) = unique.insert(allocation.id.clone(), allocation.resident_bytes)
                && existing != allocation.resident_bytes
            {
                return Err(NativeStoredPayloadError::ResidentAllocationChanged);
            }
        }
        let shared_allocations = unique
            .into_iter()
            .map(|(id, resident_bytes)| NativeResidentAllocation { id, resident_bytes })
            .collect::<Vec<_>>();
        shared_allocations
            .iter()
            .try_fold(exclusive_bytes, |total, allocation| {
                total.checked_add(allocation.resident_bytes)
            })
            .ok_or(NativeStoredPayloadError::ResidentBytesOverflow)?;
        Ok(Self {
            exclusive_bytes,
            shared_allocations,
        })
    }

    pub const fn exclusive_bytes(&self) -> usize {
        self.exclusive_bytes
    }

    pub fn shared_allocations(&self) -> &[NativeResidentAllocation] {
        &self.shared_allocations
    }

    pub fn resident_bytes(&self) -> Result<usize, NativeStoredPayloadError> {
        self.shared_allocations
            .iter()
            .try_fold(self.exclusive_bytes, |total, allocation| {
                total.checked_add(allocation.resident_bytes)
            })
            .ok_or(NativeStoredPayloadError::ResidentBytesOverflow)
    }
}

impl NativeStoredPayload {
    pub fn validate(&self) -> Result<(), NativeStoredPayloadError> {
        match self {
            Self::Tensor(payload) => {
                payload.validate()?;
                if payload.role() == NativeTensorRole::Conditioning {
                    return Err(NativeStoredPayloadError::NonCanonicalTensorRole);
                }
            }
            Self::Model(payload) => payload.validate()?,
            Self::Control(payload) => payload.validate()?,
            Self::Conditioning(payload) => payload.validate()?,
            Self::Noise(payload) => payload.validate()?,
            Self::Guider(payload) => payload.validate()?,
            Self::Sampler(payload) => payload.validate()?,
            Self::BoundingBox(payload) => payload.validate()?,
            Self::FaceLandmarks(payload) => payload.validate()?,
            Self::PoseKeypoint(payload) => payload.validate()?,
            Self::Sam3TrackData(payload) => payload.validate()?,
            Self::Tracks(payload) => payload.validate()?,
            Self::AudioEncoderOutput(payload) => payload.validate()?,
            Self::ClipVisionOutput(payload) => payload.validate()?,
            Self::IcLoraParameters(payload) => payload.validate()?,
            Self::LossMap(payload) => payload.validate()?,
            Self::Provider(payload) => payload.validate()?,
        }
        let handle_type = self.handle_type()?;
        handle_type.validate()?;
        if !valid_sha256(&self.digest_sha256()) {
            return Err(NativeStoredPayloadError::ProjectionChanged);
        }
        Ok(())
    }

    pub fn handle_type(&self) -> Result<NativeHandleType, NativeStoredPayloadError> {
        let source_type = match self {
            Self::Tensor(payload) => payload.handle_type_id(),
            Self::Model(payload) => return payload.handle_type(),
            Self::Control(payload) => payload.role().source_type_id(),
            Self::Conditioning(_) => "CONDITIONING",
            Self::Noise(_) => "NOISE",
            Self::Guider(_) => "GUIDER",
            Self::Sampler(_) => "SAMPLER",
            Self::BoundingBox(_) => NativeBoundingBoxPayload::SOURCE_TYPE_ID,
            Self::FaceLandmarks(_) => NativeFaceLandmarksPayload::SOURCE_TYPE_ID,
            Self::PoseKeypoint(_) => NativePoseKeypointPayload::SOURCE_TYPE_ID,
            Self::Sam3TrackData(_) => NativeSam3TrackDataPayload::SOURCE_TYPE_ID,
            Self::Tracks(_) => NativeTracksPayload::SOURCE_TYPE_ID,
            Self::AudioEncoderOutput(_) => AudioEncoderOutput::SOURCE_TYPE_ID,
            Self::ClipVisionOutput(_) => ClipVisionOutput::SOURCE_TYPE_ID,
            Self::IcLoraParameters(_) => IcLoraParameters::SOURCE_TYPE_ID,
            Self::LossMap(_) => LossMap::SOURCE_TYPE_ID,
            Self::Provider(payload) => return Ok(payload.handle_type().clone()),
        };
        native_source_type_projection(source_type)?
            .handle_type()?
            .ok_or_else(|| NativeStoredPayloadError::MissingHandleType(source_type.to_owned()))
    }

    pub fn digest_sha256(&self) -> String {
        match self {
            Self::Tensor(payload) => payload.projection().content_digest().to_owned(),
            Self::Model(payload) => payload.digest_sha256().to_owned(),
            Self::Control(payload) => payload.digest_sha256().to_owned(),
            Self::Conditioning(payload) => payload.digest().to_owned(),
            Self::Noise(payload) => payload.semantic_digest_sha256().to_owned(),
            Self::Guider(payload) => payload.semantic_digest_sha256().to_owned(),
            Self::Sampler(payload) => payload.semantic_digest_sha256().to_owned(),
            Self::BoundingBox(payload) => hex_sha256(payload.semantic_digest_sha256()),
            Self::FaceLandmarks(payload) => hex_sha256(payload.semantic_digest_sha256()),
            Self::PoseKeypoint(payload) => hex_sha256(payload.semantic_digest_sha256()),
            Self::Sam3TrackData(payload) => hex_sha256(payload.semantic_digest_sha256()),
            Self::Tracks(payload) => hex_sha256(payload.semantic_digest_sha256()),
            Self::AudioEncoderOutput(payload) => hex_sha256(payload.semantic_digest_sha256()),
            Self::ClipVisionOutput(payload) => hex_sha256(payload.semantic_digest_sha256()),
            Self::IcLoraParameters(payload) => hex_sha256(payload.semantic_digest_sha256()),
            Self::LossMap(payload) => hex_sha256(payload.semantic_digest_sha256()),
            Self::Provider(payload) => payload.identity_digest_sha256(),
        }
    }

    pub fn resident_bytes(&self) -> Result<usize, NativeStoredPayloadError> {
        match self {
            Self::Tensor(payload) => usize::try_from(payload.projection().resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::Model(payload) => payload.resident_bytes(),
            Self::Control(payload) => Ok(payload.resident_bytes()?),
            Self::Conditioning(payload) => usize::try_from(payload.resident_bytes()?)
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::Noise(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::Guider(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::Sampler(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::BoundingBox(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::FaceLandmarks(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::PoseKeypoint(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::Sam3TrackData(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::Tracks(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::AudioEncoderOutput(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::ClipVisionOutput(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::IcLoraParameters(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::LossMap(payload) => usize::try_from(payload.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow),
            Self::Provider(payload) => payload.resident_bytes(),
        }
    }

    pub fn residency(&self) -> Result<NativePayloadResidency, NativeStoredPayloadError> {
        let residency = match self {
            Self::Tensor(payload) => {
                NativePayloadResidency::checked(0, [tensor_storage_allocation(payload)?])?
            }
            Self::Model(payload) => {
                let model_payload = payload.diffusion().model_payload();
                let model_allocations = model_allocations(model_payload)?;
                let model_bytes = model_payload.resident_parts()?.resident_bytes()?;
                let wrapper_bytes = payload
                    .resident_bytes()?
                    .checked_sub(
                        usize::try_from(model_bytes)
                            .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow)?,
                    )
                    .ok_or(NativeStoredPayloadError::ResidentAllocationChanged)?;
                let mut allocations = vec![arc_allocation(
                    NativeResidentPayloadKind::Model,
                    payload,
                    wrapper_bytes,
                )];
                allocations.extend(model_allocations);
                NativePayloadResidency::checked(0, allocations)?
            }
            Self::Conditioning(payload) => conditioning_residency(payload)?,
            Self::Guider(payload) => guider_residency(payload)?,
            Self::Control(payload) => NativePayloadResidency::checked(
                0,
                [arc_allocation(
                    NativeResidentPayloadKind::Control,
                    payload,
                    payload.resident_bytes()?,
                )],
            )?,
            Self::Noise(payload) => NativePayloadResidency::checked(
                0,
                [arc_allocation(
                    NativeResidentPayloadKind::Noise,
                    payload,
                    usize::try_from(payload.resident_bytes())
                        .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow)?,
                )],
            )?,
            Self::Sampler(payload) => NativePayloadResidency::checked(
                0,
                [arc_allocation(
                    NativeResidentPayloadKind::Sampler,
                    payload,
                    usize::try_from(payload.resident_bytes())
                        .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow)?,
                )],
            )?,
            Self::BoundingBox(payload) => single_arc_residency(
                NativeResidentPayloadKind::BoundingBox,
                payload,
                payload.resident_bytes(),
            )?,
            Self::FaceLandmarks(payload) => single_arc_residency(
                NativeResidentPayloadKind::FaceLandmarks,
                payload,
                payload.resident_bytes(),
            )?,
            Self::PoseKeypoint(payload) => single_arc_residency(
                NativeResidentPayloadKind::PoseKeypoint,
                payload,
                payload.resident_bytes(),
            )?,
            Self::Sam3TrackData(payload) => single_arc_residency(
                NativeResidentPayloadKind::Sam3TrackData,
                payload,
                payload.resident_bytes(),
            )?,
            Self::Tracks(payload) => single_arc_residency(
                NativeResidentPayloadKind::Tracks,
                payload,
                payload.resident_bytes(),
            )?,
            Self::AudioEncoderOutput(payload) => single_arc_residency(
                NativeResidentPayloadKind::AudioEncoderOutput,
                payload,
                payload.resident_bytes(),
            )?,
            Self::ClipVisionOutput(payload) => single_arc_residency(
                NativeResidentPayloadKind::ClipVisionOutput,
                payload,
                payload.resident_bytes(),
            )?,
            Self::IcLoraParameters(payload) => single_arc_residency(
                NativeResidentPayloadKind::IcLoraParameters,
                payload,
                payload.resident_bytes(),
            )?,
            Self::LossMap(payload) => single_arc_residency(
                NativeResidentPayloadKind::LossMap,
                payload,
                payload.resident_bytes(),
            )?,
            Self::Provider(payload) => NativePayloadResidency::checked(
                0,
                [arc_allocation(
                    NativeResidentPayloadKind::Provider,
                    payload,
                    payload.resident_bytes()?,
                )],
            )?,
        };
        if residency.resident_bytes()? != self.resident_bytes()? {
            return Err(NativeStoredPayloadError::ResidentAllocationChanged);
        }
        Ok(residency)
    }
}

fn arc_address<T>(payload: &Arc<T>) -> usize {
    Arc::as_ptr(payload) as usize
}

fn arc_allocation<T>(
    kind: NativeResidentPayloadKind,
    payload: &Arc<T>,
    resident_bytes: usize,
) -> NativeResidentAllocation {
    NativeResidentAllocation {
        id: NativeResidentAllocationId::PayloadArc {
            kind,
            address: arc_address(payload),
        },
        resident_bytes,
    }
}

fn single_arc_residency<T>(
    kind: NativeResidentPayloadKind,
    payload: &Arc<T>,
    resident_bytes: u64,
) -> Result<NativePayloadResidency, NativeStoredPayloadError> {
    NativePayloadResidency::checked(
        0,
        [arc_allocation(
            kind,
            payload,
            usize::try_from(resident_bytes)
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow)?,
        )],
    )
}

fn tensor_storage_allocation(
    payload: &Arc<NativeTensorPayload>,
) -> Result<NativeResidentAllocation, NativeStoredPayloadError> {
    Ok(NativeResidentAllocation {
        id: NativeResidentAllocationId::TensorStorage {
            storage_id: payload.tensor().storage_id().get(),
        },
        resident_bytes: usize::try_from(payload.projection().resident_bytes())
            .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow)?,
    })
}

fn conditioning_allocations(
    payload: &Arc<ConditioningSet>,
) -> Result<Vec<NativeResidentAllocation>, NativeStoredPayloadError> {
    let parts = payload.resident_parts()?;
    let mut allocations = Vec::with_capacity(parts.tensor_allocations().len() + 1);
    allocations.push(NativeResidentAllocation {
        id: NativeResidentAllocationId::ConditioningSetArc {
            address: arc_address(payload),
        },
        resident_bytes: usize::try_from(parts.owned_bytes())
            .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow)?,
    });
    for allocation in parts.tensor_allocations() {
        allocations.push(NativeResidentAllocation {
            id: NativeResidentAllocationId::TensorStorage {
                storage_id: allocation.storage_id().get(),
            },
            resident_bytes: usize::try_from(allocation.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow)?,
        });
    }
    Ok(allocations)
}

fn conditioning_residency(
    payload: &Arc<ConditioningSet>,
) -> Result<NativePayloadResidency, NativeStoredPayloadError> {
    NativePayloadResidency::checked(0, conditioning_allocations(payload)?)
}

fn guider_residency(
    payload: &Arc<NativeGuiderPayload>,
) -> Result<NativePayloadResidency, NativeStoredPayloadError> {
    let mut allocations = vec![arc_allocation(
        NativeResidentPayloadKind::Guider,
        payload,
        usize::try_from(payload.owned_resident_bytes()?)
            .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow)?,
    )];
    allocations.extend(model_allocations(payload.model())?);
    match payload.conditioning_sets() {
        NativeGuiderConditioningSets::Basic { conditioning } => {
            allocations.extend(conditioning_allocations(conditioning)?);
        }
        NativeGuiderConditioningSets::Cfg { positive, negative } => {
            allocations.extend(conditioning_allocations(positive)?);
            allocations.extend(conditioning_allocations(negative)?);
        }
    }
    NativePayloadResidency::checked(0, allocations)
}

fn model_allocations(
    payload: &Arc<NativeModelPayload>,
) -> Result<Vec<NativeResidentAllocation>, NativeStoredPayloadError> {
    let parts = payload.resident_parts()?;
    let mut allocations = Vec::with_capacity(parts.backing_allocations().len() + 1);
    allocations.push(NativeResidentAllocation {
        id: NativeResidentAllocationId::ModelPayloadArc {
            address: arc_address(payload),
        },
        resident_bytes: usize::try_from(parts.owned_bytes())
            .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow)?,
    });
    for allocation in parts.backing_allocations() {
        allocations.push(NativeResidentAllocation {
            id: NativeResidentAllocationId::ModelBacking {
                kind: allocation.kind(),
                address: allocation.address(),
            },
            resident_bytes: usize::try_from(allocation.resident_bytes())
                .map_err(|_| NativeStoredPayloadError::ResidentBytesOverflow)?,
        });
    }
    Ok(allocations)
}

impl std::fmt::Debug for NativeStoredPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeStoredPayload")
            .field("handle_type", &self.handle_type().ok())
            .field("digest_sha256", &self.digest_sha256())
            .field("resident_bytes", &self.resident_bytes().ok())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Error)]
pub enum NativeStoredPayloadError {
    #[error(transparent)]
    Contract(#[from] NativeNodeContractError),
    #[error(transparent)]
    Tensor(#[from] comfy_tensor::NativeTensorPayloadError),
    #[error(transparent)]
    Model(#[from] comfy_model::NativeModelPayloadError),
    #[error(transparent)]
    ClipVision(#[from] comfy_model::clip_vision::ClipVisionError),
    #[error(transparent)]
    Diffusion(#[from] comfy_sampler::NativeDiffusionPayloadError),
    #[error(transparent)]
    Sampler(#[from] comfy_sampler::NativeSamplerPayloadError),
    #[error(transparent)]
    Media(#[from] comfy_media::NativeMediaPayloadError),
    #[error(transparent)]
    Conditioning(#[from] ConditioningError),
    #[error(transparent)]
    SourceType(#[from] crate::NativeSourceTypeError),
    #[error("native stored payload resident byte count overflowed")]
    ResidentBytesOverflow,
    #[error("native stored payload resident allocation projection changed")]
    ResidentAllocationChanged,
    #[error("native stored payload projection changed")]
    ProjectionChanged,
    #[error("native stored payload source type `{0}` is not handle-backed")]
    MissingHandleType(String),
    #[error("native provider payload is invalid")]
    InvalidProviderPayload,
    #[error("native tensor role must use its canonical stored payload owner")]
    NonCanonicalTensorRole,
}

const fn native_handle_kind_tag(kind: NativeHandleKind) -> u8 {
    match kind {
        NativeHandleKind::Tensor => 1,
        NativeHandleKind::Model => 2,
        NativeHandleKind::Clip => 3,
        NativeHandleKind::Vae => 4,
        NativeHandleKind::ControlNet => 5,
        NativeHandleKind::Conditioning => 6,
        NativeHandleKind::Latent => 7,
        NativeHandleKind::Image => 8,
        NativeHandleKind::Mask => 9,
        NativeHandleKind::Audio => 10,
        NativeHandleKind::Video => 11,
        NativeHandleKind::ThreeD => 12,
        NativeHandleKind::Artifact => 13,
        NativeHandleKind::ProviderTask => 14,
        NativeHandleKind::StructuredCompute => 15,
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hex_sha256(value: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in value {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_resident_bytes_charge_owned_capacity() -> Result<(), Box<dyn std::error::Error>> {
        let mut type_id = String::with_capacity(8_192);
        type_id.push_str("PROVIDER_TASK");
        let mut namespace = String::with_capacity(16_384);
        namespace.push_str("signed-provider");
        let semantic_digest = "a".repeat(64);
        let mut abi_bytes = Vec::with_capacity(1_048_576);
        abi_bytes.extend_from_slice(b"payload");
        let payload = NativeProviderPayload::checked(
            NativeHandleType::new(NativeHandleKind::ProviderTask, type_id)?,
            namespace,
            semantic_digest,
            abi_bytes,
        )?;

        assert!(payload.resident_bytes()? >= 8_192 + 16_384 + 1_048_576);
        Ok(())
    }
}
