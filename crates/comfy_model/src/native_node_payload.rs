use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    mem,
    num::NonZeroU64,
    sync::Arc,
};
use thiserror::Error;

use comfy_tensor::{CancellationToken, DType, StorageId, Tensor, TensorError};

use crate::{
    GEMMA3_FOUR_B_MULTIMODAL_SOURCE_SHA256, GEMMA3_MULTIMODAL_SOURCE_SHA256,
    GEMMA4_MULTIMODAL_SOURCE_SHA256, LLAMA_SOURCE_SHA256, NativeAudioEncoder,
    NativeBackgroundRemovalResource, NativeDecoderTextEncoder, NativeDepthAnything3Resource,
    NativeFrameInterpolationModel, NativeGemmaMultimodal, NativeLatentUpscaleModelResource,
    NativePromptTokenizer, NativeQwenMultimodal, NativeRaftLarge, NativeSdPoseModel,
    NativeStructuredVae, NativeVae, QWEN_MULTIMODAL_ROUTING_SOURCE_SHA256, QWEN_VL_SOURCE_SHA256,
    QWEN3VL_SOURCE_SHA256, QWEN35_SOURCE_SHA256,
    clip::{LoadedSd1Clip, NativeClipResidentOwnerKind, NativeClipResource, NativeTokenizer},
    clip_vision::NativeClipVision,
    generated_native_diffusion::{Sd1Tokenizer, Sd15TinyModel},
    model_family::{NativeFamilyModelResidentOwnerKind, NativeFamilyModelResource},
};
const NATIVE_MODEL_RESOURCE_SCHEMA_VERSION: u16 = 1;
const MAX_IDENTITY_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeModelResourceRole {
    AudioEncoder,
    BackgroundRemoval,
    Model,
    Clip,
    Vae,
    ClipVision,
    ControlNet,
    Da3Model,
    FaceDetection,
    FrameInterpolation,
    Gligen,
    Hooks,
    HookKeyframes,
    LatentOperation,
    LatentUpscaleModel,
    LoraModel,
    ModelPatch,
    MogeModel,
    OpticalFlow,
    Photomaker,
    StyleModel,
    UpscaleModel,
}

impl NativeModelResourceRole {
    pub const fn source_type_id(self) -> &'static str {
        match self {
            Self::AudioEncoder => "AUDIO_ENCODER",
            Self::BackgroundRemoval => "BACKGROUND_REMOVAL",
            Self::Model => "MODEL",
            Self::Clip => "CLIP",
            Self::Vae => "VAE",
            Self::ClipVision => "CLIP_VISION",
            Self::ControlNet => "CONTROL_NET",
            Self::Da3Model => "DA3_MODEL",
            Self::FaceDetection => "FACE_DETECTION_MODEL",
            Self::FrameInterpolation => "INTERP_MODEL",
            Self::Gligen => "GLIGEN",
            Self::Hooks => "HOOKS",
            Self::HookKeyframes => "HOOK_KEYFRAMES",
            Self::LatentOperation => "LATENT_OPERATION",
            Self::LatentUpscaleModel => "LATENT_UPSCALE_MODEL",
            Self::LoraModel => "LORA_MODEL",
            Self::ModelPatch => "MODEL_PATCH",
            Self::MogeModel => "MOGE_MODEL",
            Self::OpticalFlow => "OPTICAL_FLOW",
            Self::Photomaker => "PHOTOMAKER",
            Self::StyleModel => "STYLE_MODEL",
            Self::UpscaleModel => "UPSCALE_MODEL",
        }
    }

    const fn digest_tag(self) -> u8 {
        match self {
            Self::AudioEncoder => 1,
            Self::BackgroundRemoval => 2,
            Self::Model => 3,
            Self::Clip => 4,
            Self::Vae => 5,
            Self::ClipVision => 6,
            Self::ControlNet => 7,
            Self::Da3Model => 8,
            Self::FaceDetection => 9,
            Self::FrameInterpolation => 10,
            Self::Gligen => 11,
            Self::Hooks => 12,
            Self::HookKeyframes => 13,
            Self::LatentOperation => 14,
            Self::LatentUpscaleModel => 15,
            Self::LoraModel => 16,
            Self::ModelPatch => 17,
            Self::MogeModel => 18,
            Self::OpticalFlow => 19,
            Self::Photomaker => 20,
            Self::StyleModel => 21,
            Self::UpscaleModel => 22,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NativeModelResourceIdentity {
    schema_version: u16,
    role: NativeModelResourceRole,
    identifier: String,
    format: String,
    artifact_sha256: String,
    execution_sha256: String,
    digest_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeModelResourceIdentityWire {
    schema_version: u16,
    role: NativeModelResourceRole,
    identifier: String,
    format: String,
    artifact_sha256: String,
    execution_sha256: String,
    digest_sha256: String,
}

impl<'de> Deserialize<'de> for NativeModelResourceIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = NativeModelResourceIdentityWire::deserialize(deserializer)?;
        let identity = Self {
            schema_version: wire.schema_version,
            role: wire.role,
            identifier: wire.identifier,
            format: wire.format,
            artifact_sha256: wire.artifact_sha256,
            execution_sha256: wire.execution_sha256,
            digest_sha256: wire.digest_sha256,
        };
        identity.validate().map_err(serde::de::Error::custom)?;
        Ok(identity)
    }
}

impl NativeModelResourceIdentity {
    pub fn checked(
        role: NativeModelResourceRole,
        identifier: impl Into<String>,
        format: impl Into<String>,
        artifact_sha256: impl Into<String>,
        execution_sha256: impl Into<String>,
    ) -> Result<Self, NativeModelPayloadError> {
        let mut identity = Self {
            schema_version: NATIVE_MODEL_RESOURCE_SCHEMA_VERSION,
            role,
            identifier: identifier.into(),
            format: format.into(),
            artifact_sha256: artifact_sha256.into(),
            execution_sha256: execution_sha256.into(),
            digest_sha256: String::new(),
        };
        identity.digest_sha256 = identity.canonical_digest()?;
        identity.validate()?;
        Ok(identity)
    }

    pub const fn role(&self) -> NativeModelResourceRole {
        self.role
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn format(&self) -> &str {
        &self.format
    }

    pub fn artifact_sha256(&self) -> &str {
        &self.artifact_sha256
    }

    pub fn execution_sha256(&self) -> &str {
        &self.execution_sha256
    }

    pub fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }

    pub(crate) fn resident_owned_bytes(&self) -> Result<u64, NativeModelPayloadError> {
        [
            &self.identifier,
            &self.format,
            &self.artifact_sha256,
            &self.execution_sha256,
            &self.digest_sha256,
        ]
        .into_iter()
        .try_fold(0_u64, |total, value| {
            total
                .checked_add(
                    u64::try_from(value.capacity())
                        .map_err(|_| NativeModelPayloadError::LengthOverflow)?,
                )
                .ok_or(NativeModelPayloadError::LengthOverflow)
        })
    }

    pub fn validate(&self) -> Result<(), NativeModelPayloadError> {
        if self.schema_version != NATIVE_MODEL_RESOURCE_SCHEMA_VERSION {
            return Err(NativeModelPayloadError::UnsupportedSchema(
                self.schema_version,
            ));
        }
        validate_identity_text("identifier", &self.identifier)?;
        validate_identity_text("format", &self.format)?;
        if !valid_sha256(&self.artifact_sha256)
            || !valid_sha256(&self.execution_sha256)
            || !valid_sha256(&self.digest_sha256)
        {
            return Err(NativeModelPayloadError::InvalidDigest);
        }
        if self.digest_sha256 != self.canonical_digest()? {
            return Err(NativeModelPayloadError::IdentityMismatch);
        }
        Ok(())
    }

    fn canonical_digest(&self) -> Result<String, NativeModelPayloadError> {
        let mut hasher = Sha256::new();
        hash_field(&mut hasher, b"zed.comfy.native-model-resource.v1")?;
        hash_field(&mut hasher, &[self.role.digest_tag()])?;
        hash_field(&mut hasher, self.identifier.as_bytes())?;
        hash_field(&mut hasher, self.format.as_bytes())?;
        hash_field(&mut hasher, self.artifact_sha256.as_bytes())?;
        hash_field(&mut hasher, self.execution_sha256.as_bytes())?;
        Ok(format!("{:x}", hasher.finalize()))
    }
}

#[derive(Clone)]
pub struct NativeModelPayload {
    identity: NativeModelResourceIdentity,
    resident_bytes: u64,
    resource: NativeModelResource,
}

#[derive(Clone)]
enum NativeModelResource {
    Sd15Model {
        model: Arc<Sd15TinyModel>,
    },
    NativeFamilyModel {
        resource: Arc<NativeFamilyModelResource>,
    },
    AudioEncoder {
        resource: Arc<NativeAudioEncoder>,
    },
    Sd1Clip {
        tokenizer: Arc<Sd1Tokenizer>,
        clip: Arc<LoadedSd1Clip>,
    },
    NativeVae {
        vae: Arc<NativeVae>,
    },
    NativeStructuredVae {
        vae: Arc<NativeStructuredVae>,
    },
    OpticalFlow {
        raft: Arc<NativeRaftLarge>,
    },
    ClipVision {
        clip_vision: Arc<NativeClipVision>,
    },
    DecoderClip {
        tokenizer: Arc<NativePromptTokenizer>,
        decoder: Arc<NativeDecoderTextEncoder>,
    },
    QwenMultimodalClip {
        resource: Arc<NativeQwenMultimodal>,
    },
    GemmaMultimodalClip {
        resource: Arc<NativeGemmaMultimodal>,
    },
    NativeClip {
        resource: Arc<NativeClipResource>,
    },
    SdPoseModel {
        resource: Arc<NativeSdPoseModel>,
    },
    FrameInterpolation {
        resource: Arc<NativeFrameInterpolationModel>,
    },
    LatentUpscaleModel {
        resource: Arc<NativeLatentUpscaleModelResource>,
    },
    BackgroundRemoval {
        resource: Arc<NativeBackgroundRemovalResource>,
    },
    DepthAnything3 {
        resource: Arc<NativeDepthAnything3Resource>,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NativeModelBackingKind {
    Sd15Model,
    NativeFamilyModelResource,
    NativeFamilyModelMaterialized,
    NativeFamilyModelMappedWeights,
    NativeFamilyModelPatchGraph,
    NativeAudioEncoder,
    Sd1Tokenizer,
    Sd1Clip,
    NativeVae,
    NativeStructuredVae,
    OpticalFlow,
    ClipVision,
    NativePromptTokenizer,
    NativeDecoderTextEncoder,
    NativeQwenMultimodal,
    NativeQwenVisionEncoder,
    NativeGemmaMultimodal,
    NativeGemma3VisionProjector,
    NativeGemma4VisionEncoder,
    NativeGemma4AudioEncoder,
    NativeClipResource,
    NativeClipComponent,
    NativeClipTokenizer,
    NativeClipEncoder,
    NativeClipMappedWeights,
    NativeSdPoseModel,
    NativeFrameInterpolationModel,
    NativeLatentUpscaleModel,
    NativeBackgroundRemovalResource,
    NativeDepthAnything3Resource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeModelResidentAllocation {
    kind: NativeModelBackingKind,
    address: usize,
    resident_bytes: u64,
}

impl NativeModelResidentAllocation {
    pub const fn kind(&self) -> NativeModelBackingKind {
        self.kind
    }

    pub const fn address(&self) -> usize {
        self.address
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeModelTensorResidentAllocation {
    storage_id: StorageId,
    resident_bytes: u64,
}

#[derive(Clone, Copy)]
pub enum NativeExecutableModel<'a> {
    Sd15(&'a Sd15TinyModel),
    Family(&'a NativeFamilyModelResource),
}

impl NativeExecutableModel<'_> {
    pub fn sd15(&self) -> Option<&Sd15TinyModel> {
        match self {
            Self::Sd15(model) => Some(model),
            Self::Family(_) => None,
        }
    }

    pub fn is_family(&self) -> bool {
        matches!(self, Self::Family(_))
    }

    pub fn patch_identity(&self) -> crate::PatchGraphIdentity {
        match self {
            Self::Sd15(model) => model.patch_identity().clone(),
            Self::Family(resource) => resource.patch_graph().identity(),
        }
    }

    pub fn execution_digest(&self) -> &str {
        match self {
            Self::Sd15(model) => model.patch_execution_digest(),
            Self::Family(resource) => resource.semantic_digest_sha256(),
        }
    }

    pub fn conditioning_identity(&self) -> Option<&crate::conditioning::ConditioningIdentity> {
        match self {
            Self::Sd15(_) => None,
            Self::Family(resource) => Some(resource.conditioning_identity()),
        }
    }
}

impl NativeModelTensorResidentAllocation {
    pub const fn storage_id(&self) -> StorageId {
        self.storage_id
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeModelResidentParts {
    owned_bytes: u64,
    backing_allocations: Vec<NativeModelResidentAllocation>,
    tensor_allocations: Vec<NativeModelTensorResidentAllocation>,
}

impl NativeModelResidentParts {
    pub const fn owned_bytes(&self) -> u64 {
        self.owned_bytes
    }

    pub fn backing_allocations(&self) -> &[NativeModelResidentAllocation] {
        &self.backing_allocations
    }

    pub fn tensor_allocations(&self) -> &[NativeModelTensorResidentAllocation] {
        &self.tensor_allocations
    }

    pub fn resident_bytes(&self) -> Result<u64, NativeModelPayloadError> {
        let bytes =
            self.backing_allocations
                .iter()
                .try_fold(self.owned_bytes, |bytes, allocation| {
                    bytes
                        .checked_add(allocation.resident_bytes)
                        .ok_or(NativeModelPayloadError::LengthOverflow)
                })?;
        self.tensor_allocations
            .iter()
            .try_fold(bytes, |bytes, allocation| {
                bytes
                    .checked_add(allocation.resident_bytes)
                    .ok_or(NativeModelPayloadError::LengthOverflow)
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeStructuredResidentParts {
    owned_bytes: u64,
    tensor_allocations: Vec<NativeModelTensorResidentAllocation>,
}

impl NativeStructuredResidentParts {
    pub const fn owned_bytes(&self) -> u64 {
        self.owned_bytes
    }

    pub fn tensor_allocations(&self) -> &[NativeModelTensorResidentAllocation] {
        &self.tensor_allocations
    }

    pub fn resident_bytes(&self) -> Result<u64, NativeModelPayloadError> {
        self.tensor_allocations
            .iter()
            .try_fold(self.owned_bytes, |bytes, allocation| {
                bytes
                    .checked_add(allocation.resident_bytes)
                    .ok_or(NativeModelPayloadError::StructuredResidentBytesOverflow)
            })
    }
}

impl NativeModelPayload {
    pub fn sd15_model(model: Arc<Sd15TinyModel>) -> Result<Self, NativeModelPayloadError> {
        let patch_identity = model.patch_identity();
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Model,
            patch_identity.base_artifact_digest.clone(),
            "zed-native-sd15-model-v1",
            patch_identity.base_artifact_digest.clone(),
            model.patch_execution_digest(),
        )?;
        let resident_bytes = payload_resident_bytes(
            &identity,
            model
                .resident_bytes()
                .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?,
        )?;
        Ok(Self {
            identity,
            resident_bytes,
            resource: NativeModelResource::Sd15Model { model },
        })
    }

    pub fn native_family_model(
        resource: Arc<NativeFamilyModelResource>,
    ) -> Result<Self, NativeModelPayloadError> {
        let cancellation = CancellationToken::default();
        resource
            .validate(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let family = resource
            .family_identity()
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Model,
            format!(
                "{}:{}:{}",
                family.feature_id(),
                family.identifier(),
                family.architecture_version()
            ),
            "zed-native-family-model-v1",
            resource.artifact_sha256(),
            resource.semantic_digest_sha256(),
        )?;
        let backing_bytes = resource
            .resident_bytes()
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, backing_bytes)?,
            identity,
            resource: NativeModelResource::NativeFamilyModel { resource },
        })
    }

    pub fn audio_encoder(
        resource: Arc<NativeAudioEncoder>,
    ) -> Result<Self, NativeModelPayloadError> {
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::AudioEncoder,
            resource.identifier(),
            "zed-native-audio-encoder-v1",
            resource.artifact_sha256(),
            resource.semantic_state_digest_sha256(),
        )?;
        let backing_bytes = resource
            .resident_bytes()
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, backing_bytes)?,
            identity,
            resource: NativeModelResource::AudioEncoder { resource },
        })
    }

    pub fn sd1_clip(
        tokenizer: Arc<Sd1Tokenizer>,
        clip: Arc<LoadedSd1Clip>,
    ) -> Result<Self, NativeModelPayloadError> {
        let plan = clip.plan();
        if plan.tokenizer_identity() != tokenizer.identity() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "CLIP tokenizer identity",
            ));
        }
        let execution_sha256 = digest_fields(
            b"zed.comfy.native-sd1-clip-payload.v1",
            [plan.digest(), clip.architecture().digest()],
        )?;
        let backing_bytes =
            tokenizer
                .resident_bytes()
                .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?
                .checked_add(clip.resident_bytes().map_err(|error| {
                    NativeModelPayloadError::ResourceAccounting(error.to_string())
                })?)
                .ok_or(NativeModelPayloadError::LengthOverflow)?;
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Clip,
            plan.clip_model_identifier(),
            "zed-native-sd1-clip-v1",
            plan.artifact_identity().as_str(),
            execution_sha256,
        )?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, backing_bytes)?,
            identity,
            resource: NativeModelResource::Sd1Clip { tokenizer, clip },
        })
    }

    pub fn native_vae(vae: Arc<NativeVae>) -> Result<Self, NativeModelPayloadError> {
        let descriptor = vae.descriptor();
        let resource_identity = descriptor.identity();
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Vae,
            resource_identity.digest(),
            resource_identity.architecture().as_str(),
            resource_identity.artifact_sha256(),
            vae.execution_digest(),
        )?;
        let resident_bytes = payload_resident_bytes(
            &identity,
            vae.resident_bytes()
                .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?,
        )?;
        Ok(Self {
            identity,
            resident_bytes,
            resource: NativeModelResource::NativeVae { vae },
        })
    }

    pub fn native_structured_vae(
        vae: Arc<NativeStructuredVae>,
    ) -> Result<Self, NativeModelPayloadError> {
        vae.validate()
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let descriptor = vae.descriptor();
        let resource_identity = descriptor.identity();
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Vae,
            resource_identity.digest(),
            resource_identity.architecture().as_str(),
            resource_identity.artifact_sha256(),
            vae.execution_digest(),
        )?;
        let resident_bytes = payload_resident_bytes(
            &identity,
            vae.resident_bytes()
                .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?,
        )?;
        Ok(Self {
            identity,
            resident_bytes,
            resource: NativeModelResource::NativeStructuredVae { vae },
        })
    }

    pub fn optical_flow(raft: Arc<NativeRaftLarge>) -> Result<Self, NativeModelPayloadError> {
        if raft.is_training() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "OPTICAL_FLOW evaluation state",
            ));
        }
        let cancellation = CancellationToken::default();
        raft.validate(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let identity = raft
            .semantic_identity()
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?
            .clone();
        if identity.role() != NativeModelResourceRole::OpticalFlow {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "OPTICAL_FLOW resource role",
            ));
        }
        let resident_bytes = payload_resident_bytes(
            &identity,
            raft.resident_bytes()
                .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?,
        )?;
        Ok(Self {
            identity,
            resident_bytes,
            resource: NativeModelResource::OpticalFlow { raft },
        })
    }

    pub fn clip_vision(
        clip_vision: Arc<NativeClipVision>,
    ) -> Result<Self, NativeModelPayloadError> {
        if clip_vision.is_training() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "CLIP_VISION evaluation state",
            ));
        }
        let cancellation = CancellationToken::default();
        clip_vision
            .validate(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let identity = clip_vision.semantic_identity().clone();
        if identity.role() != NativeModelResourceRole::ClipVision {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "CLIP_VISION resource role",
            ));
        }
        let resident_bytes = payload_resident_bytes(
            &identity,
            clip_vision
                .resident_bytes()
                .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?,
        )?;
        Ok(Self {
            identity,
            resident_bytes,
            resource: NativeModelResource::ClipVision { clip_vision },
        })
    }

    pub fn decoder_clip(
        tokenizer: Arc<NativePromptTokenizer>,
        decoder: Arc<NativeDecoderTextEncoder>,
    ) -> Result<Self, NativeModelPayloadError> {
        let cancellation = CancellationToken::default();
        let tokenizer_digest = tokenizer
            .semantic_digest(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let decoder_digest = decoder
            .semantic_state_digest(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Clip,
            format!("native-decoder-{:?}", decoder.configuration().architecture),
            "zed-native-decoder-clip-v1",
            tokenizer_digest,
            decoder_digest,
        )?;
        let backing_bytes =
            tokenizer
                .resident_bytes()
                .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?
                .checked_add(decoder.resident_bytes().map_err(|error| {
                    NativeModelPayloadError::ResourceAccounting(error.to_string())
                })?)
                .ok_or(NativeModelPayloadError::LengthOverflow)?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, backing_bytes)?,
            identity,
            resource: NativeModelResource::DecoderClip { tokenizer, decoder },
        })
    }

    pub fn qwen_multimodal_clip(
        resource: Arc<NativeQwenMultimodal>,
    ) -> Result<Self, NativeModelPayloadError> {
        let cancellation = CancellationToken::default();
        resource
            .validate(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        if !resource.is_source_exact_profile() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "Qwen production source-exact profile",
            ));
        }
        let family_source_sha256 = match resource.family() {
            crate::QwenVisionFamily::Qwen3Vl4B | crate::QwenVisionFamily::Qwen3Vl8B => {
                QWEN3VL_SOURCE_SHA256
            }
            crate::QwenVisionFamily::Qwen35_08B
            | crate::QwenVisionFamily::Qwen35_2B
            | crate::QwenVisionFamily::Qwen35_4B
            | crate::QwenVisionFamily::Qwen35_9B
            | crate::QwenVisionFamily::Qwen35_27B => QWEN35_SOURCE_SHA256,
        };
        let artifact_sha256 = digest_fields(
            b"zed.comfy.native-qwen-multimodal-artifacts.v1",
            [
                family_source_sha256,
                QWEN_VL_SOURCE_SHA256,
                resource.tokenizer().qwen2_artifact_digest().ok_or(
                    NativeModelPayloadError::ResourceMismatch("Qwen tokenizer artifact identity"),
                )?,
            ],
        )?;
        let execution_sha256 = resource
            .semantic_state_digest(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Clip,
            format!("native-qwen-multimodal-{:?}", resource.family()),
            "zed-native-qwen-multimodal-clip-v1",
            artifact_sha256,
            execution_sha256,
        )?;
        let backing_bytes = resource
            .resident_bytes()
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, backing_bytes)?,
            identity,
            resource: NativeModelResource::QwenMultimodalClip { resource },
        })
    }

    pub fn gemma_multimodal_clip(
        resource: Arc<NativeGemmaMultimodal>,
    ) -> Result<Self, NativeModelPayloadError> {
        let cancellation = CancellationToken::default();
        resource
            .validate(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        if !resource.is_source_exact_profile() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "Gemma production source-exact profile",
            ));
        }
        let artifact_sha256 = digest_fields(
            b"zed.comfy.native-gemma-multimodal-artifacts.v1",
            [
                QWEN_MULTIMODAL_ROUTING_SOURCE_SHA256,
                LLAMA_SOURCE_SHA256,
                match resource.family() {
                    crate::GemmaMultimodalFamily::Gemma3FourBVision => {
                        GEMMA3_FOUR_B_MULTIMODAL_SOURCE_SHA256
                    }
                    crate::GemmaMultimodalFamily::Gemma3TwelveB => GEMMA3_MULTIMODAL_SOURCE_SHA256,
                    crate::GemmaMultimodalFamily::Gemma4E2B
                    | crate::GemmaMultimodalFamily::Gemma4E4B
                    | crate::GemmaMultimodalFamily::Gemma4ThirtyOneB => {
                        GEMMA4_MULTIMODAL_SOURCE_SHA256
                    }
                },
                resource.tokenizer().gemma_artifact_digest().ok_or(
                    NativeModelPayloadError::ResourceMismatch("Gemma tokenizer artifact identity"),
                )?,
            ],
        )?;
        let execution_sha256 = resource
            .semantic_state_digest(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Clip,
            format!("native-gemma-multimodal-{:?}", resource.family()),
            "zed-native-gemma-multimodal-clip-v1",
            artifact_sha256,
            execution_sha256,
        )?;
        let backing_bytes = resource
            .resident_bytes()
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, backing_bytes)?,
            identity,
            resource: NativeModelResource::GemmaMultimodalClip { resource },
        })
    }

    pub fn native_clip(resource: Arc<NativeClipResource>) -> Result<Self, NativeModelPayloadError> {
        let cancellation = CancellationToken::default();
        resource
            .validate(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let mut artifact_hasher = Sha256::new();
        artifact_hasher.update(b"zed.comfy.native-clip-artifacts.v1\0");
        for component in resource.components() {
            hash_field(&mut artifact_hasher, component.artifact_sha256().as_bytes())?;
        }
        let artifact_sha256 = format!("{:x}", artifact_hasher.finalize());
        let profile = match resource.profile() {
            crate::clip::NativeClipProfile::Sd1 => "sd1",
            crate::clip::NativeClipProfile::Sdxl => "sdxl",
            crate::clip::NativeClipProfile::Sd3 => "sd3",
            crate::clip::NativeClipProfile::PixArt => "pixart",
            crate::clip::NativeClipProfile::Lumina => "lumina",
            crate::clip::NativeClipProfile::HiDream => "hidream",
            crate::clip::NativeClipProfile::Qwen => "qwen25-image",
            crate::clip::NativeClipProfile::Gemma => "gemma4",
        };
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Clip,
            format!("native-clip-{profile}"),
            "zed-native-clip-resource-v1",
            artifact_sha256,
            resource.semantic_digest_sha256(),
        )?;
        let backing_bytes = resource
            .resident_bytes()
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, backing_bytes)?,
            identity,
            resource: NativeModelResource::NativeClip { resource },
        })
    }

    pub fn sdpose_model(resource: Arc<NativeSdPoseModel>) -> Result<Self, NativeModelPayloadError> {
        if !resource.is_source_exact_profile() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "SDPose production source-exact profile",
            ));
        }
        Self::sdpose_model_checked(resource)
    }

    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub fn sdpose_model_test_fixture(
        resource: Arc<NativeSdPoseModel>,
    ) -> Result<Self, NativeModelPayloadError> {
        if resource.is_source_exact_profile() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "SDPose reduced test fixture profile",
            ));
        }
        Self::sdpose_model_checked(resource)
    }

    fn sdpose_model_checked(
        resource: Arc<NativeSdPoseModel>,
    ) -> Result<Self, NativeModelPayloadError> {
        let cancellation = CancellationToken::default();
        resource
            .validate(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Model,
            "native-sdpose-lotusd",
            "zed-native-sdpose-model-v1",
            resource.artifact_sha256(),
            resource.semantic_state_digest_sha256(),
        )?;
        let backing_bytes = resource
            .resident_bytes()
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, backing_bytes)?,
            identity,
            resource: NativeModelResource::SdPoseModel { resource },
        })
    }

    pub fn frame_interpolation(
        resource: Arc<NativeFrameInterpolationModel>,
    ) -> Result<Self, NativeModelPayloadError> {
        let cancellation = CancellationToken::default();
        resource
            .validate(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let identifier = format!(
            "native-frame-interpolation-{}",
            resource.profile().identifier()
        );
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::FrameInterpolation,
            identifier,
            "zed-native-frame-interpolation-v1",
            resource.artifact_sha256(),
            resource.semantic_state_digest_sha256(),
        )?;
        let backing_bytes = resource
            .resident_bytes()
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, backing_bytes)?,
            identity,
            resource: NativeModelResource::FrameInterpolation { resource },
        })
    }

    pub fn latent_upscale_model(
        resource: Arc<NativeLatentUpscaleModelResource>,
    ) -> Result<Self, NativeModelPayloadError> {
        let cancellation = CancellationToken::default();
        resource
            .validate(&cancellation)
            .map_err(|error| NativeModelPayloadError::ResourceAccounting(error.to_string()))?;
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::LatentUpscaleModel,
            resource.identifier(),
            "zed-native-latent-upscale-model-v1",
            resource.artifact_sha256(),
            resource.semantic_digest_sha256(),
        )?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, resource.resident_bytes())?,
            identity,
            resource: NativeModelResource::LatentUpscaleModel { resource },
        })
    }

    pub fn background_removal(
        resource: Arc<NativeBackgroundRemovalResource>,
        cancellation: &CancellationToken,
    ) -> Result<Self, NativeModelPayloadError> {
        if !resource.is_source_exact_profile() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "BiRefNet production source-exact profile",
            ));
        }
        Self::background_removal_checked(resource, cancellation)
    }

    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub fn background_removal_test_fixture(
        resource: Arc<NativeBackgroundRemovalResource>,
        cancellation: &CancellationToken,
    ) -> Result<Self, NativeModelPayloadError> {
        if resource.is_source_exact_profile() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "BiRefNet reduced test fixture profile",
            ));
        }
        Self::background_removal_checked(resource, cancellation)
    }

    fn background_removal_checked(
        resource: Arc<NativeBackgroundRemovalResource>,
        cancellation: &CancellationToken,
    ) -> Result<Self, NativeModelPayloadError> {
        resource
            .validate(cancellation)
            .map_err(|error| match error {
                crate::background_removal::NativeBackgroundRemovalError::Cancelled => {
                    NativeModelPayloadError::Tensor(TensorError::Cancelled)
                }
                crate::background_removal::NativeBackgroundRemovalError::Tensor(error) => {
                    NativeModelPayloadError::Tensor(error)
                }
                error => NativeModelPayloadError::ResourceAccounting(error.to_string()),
            })?;
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::BackgroundRemoval,
            resource.identifier(),
            "zed-native-background-removal-v1",
            resource.artifact_sha256(),
            resource.semantic_digest_sha256(),
        )?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, resource.resident_bytes())?,
            identity,
            resource: NativeModelResource::BackgroundRemoval { resource },
        })
    }

    pub fn depth_anything_3(
        resource: Arc<NativeDepthAnything3Resource>,
        cancellation: &CancellationToken,
    ) -> Result<Self, NativeModelPayloadError> {
        if !resource.is_source_exact_profile() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "Depth Anything 3 production source-exact profile",
            ));
        }
        Self::depth_anything_3_checked(resource, cancellation)
    }

    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub fn depth_anything_3_test_fixture(
        resource: Arc<NativeDepthAnything3Resource>,
        cancellation: &CancellationToken,
    ) -> Result<Self, NativeModelPayloadError> {
        if resource.is_source_exact_profile() {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "Depth Anything 3 reduced test fixture profile",
            ));
        }
        Self::depth_anything_3_checked(resource, cancellation)
    }

    fn depth_anything_3_checked(
        resource: Arc<NativeDepthAnything3Resource>,
        cancellation: &CancellationToken,
    ) -> Result<Self, NativeModelPayloadError> {
        resource
            .validate(cancellation)
            .map_err(|error| match error {
                crate::depth_anything_3::NativeDepthAnything3Error::Cancelled => {
                    NativeModelPayloadError::Tensor(TensorError::Cancelled)
                }
                crate::depth_anything_3::NativeDepthAnything3Error::Tensor(error) => {
                    NativeModelPayloadError::Tensor(error)
                }
                error => NativeModelPayloadError::ResourceAccounting(error.to_string()),
            })?;
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Da3Model,
            resource.identifier(),
            "zed-native-depth-anything-3-v1",
            resource.artifact_sha256(),
            resource.semantic_digest_sha256(),
        )?;
        Ok(Self {
            resident_bytes: payload_resident_bytes(&identity, resource.resident_bytes())?,
            identity,
            resource: NativeModelResource::DepthAnything3 { resource },
        })
    }

    pub fn identity(&self) -> &NativeModelResourceIdentity {
        &self.identity
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn resident_parts(&self) -> Result<NativeModelResidentParts, NativeModelPayloadError> {
        let owned_bytes = u64::try_from(mem::size_of::<Self>())
            .map_err(|_| NativeModelPayloadError::LengthOverflow)?
            .checked_add(self.identity.resident_owned_bytes()?)
            .ok_or(NativeModelPayloadError::LengthOverflow)?;
        let mut tensor_allocations = Vec::new();
        let backing_allocations = match &self.resource {
            NativeModelResource::Sd15Model { model } => vec![NativeModelResidentAllocation {
                kind: NativeModelBackingKind::Sd15Model,
                address: Arc::as_ptr(model) as usize,
                resident_bytes: model.resident_bytes().map_err(|error| {
                    NativeModelPayloadError::ResourceAccounting(error.to_string())
                })?,
            }],
            NativeModelResource::NativeFamilyModel { resource } => {
                tensor_allocations.extend(
                    resource
                        .resident_tensor_allocations()
                        .map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?
                        .into_iter()
                        .map(
                            |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                                storage_id,
                                resident_bytes,
                            },
                        ),
                );
                resource
                    .resident_owned_allocations()
                    .map_err(|error| {
                        NativeModelPayloadError::ResourceAccounting(error.to_string())
                    })?
                    .into_iter()
                    .map(|allocation| NativeModelResidentAllocation {
                        kind: match allocation.kind() {
                            NativeFamilyModelResidentOwnerKind::Resource => {
                                NativeModelBackingKind::NativeFamilyModelResource
                            }
                            NativeFamilyModelResidentOwnerKind::MaterializedModel => {
                                NativeModelBackingKind::NativeFamilyModelMaterialized
                            }
                            NativeFamilyModelResidentOwnerKind::MappedWeights => {
                                NativeModelBackingKind::NativeFamilyModelMappedWeights
                            }
                            NativeFamilyModelResidentOwnerKind::PatchGraph => {
                                NativeModelBackingKind::NativeFamilyModelPatchGraph
                            }
                        },
                        address: allocation.address(),
                        resident_bytes: allocation.resident_bytes(),
                    })
                    .collect()
            }
            NativeModelResource::AudioEncoder { resource } => {
                tensor_allocations.extend(
                    resource
                        .resident_tensor_allocations()
                        .map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?
                        .into_iter()
                        .map(
                            |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                                storage_id,
                                resident_bytes,
                            },
                        ),
                );
                vec![NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::NativeAudioEncoder,
                    address: Arc::as_ptr(resource) as usize,
                    resident_bytes: resource.resident_owned_bytes().map_err(|error| {
                        NativeModelPayloadError::ResourceAccounting(error.to_string())
                    })?,
                }]
            }
            NativeModelResource::Sd1Clip { tokenizer, clip } => vec![
                NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::Sd1Tokenizer,
                    address: Arc::as_ptr(tokenizer) as usize,
                    resident_bytes: tokenizer.resident_bytes().map_err(|error| {
                        NativeModelPayloadError::ResourceAccounting(error.to_string())
                    })?,
                },
                NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::Sd1Clip,
                    address: Arc::as_ptr(clip) as usize,
                    resident_bytes: clip.resident_bytes().map_err(|error| {
                        NativeModelPayloadError::ResourceAccounting(error.to_string())
                    })?,
                },
            ],
            NativeModelResource::NativeVae { vae } => vec![NativeModelResidentAllocation {
                kind: NativeModelBackingKind::NativeVae,
                address: Arc::as_ptr(vae) as usize,
                resident_bytes: vae.resident_bytes().map_err(|error| {
                    NativeModelPayloadError::ResourceAccounting(error.to_string())
                })?,
            }],
            NativeModelResource::NativeStructuredVae { vae } => {
                vec![NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::NativeStructuredVae,
                    address: Arc::as_ptr(vae) as usize,
                    resident_bytes: vae.resident_bytes().map_err(|error| {
                        NativeModelPayloadError::ResourceAccounting(error.to_string())
                    })?,
                }]
            }
            NativeModelResource::OpticalFlow { raft } => {
                let parts = raft.resident_parts().map_err(|error| {
                    NativeModelPayloadError::ResourceAccounting(error.to_string())
                })?;
                tensor_allocations.extend(parts.tensor_allocations().iter().map(|allocation| {
                    NativeModelTensorResidentAllocation {
                        storage_id: allocation.storage_id(),
                        resident_bytes: allocation.resident_bytes(),
                    }
                }));
                vec![NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::OpticalFlow,
                    address: Arc::as_ptr(raft) as usize,
                    resident_bytes: parts.owned_bytes(),
                }]
            }
            NativeModelResource::ClipVision { clip_vision } => {
                let parts = clip_vision.resident_parts().map_err(|error| {
                    NativeModelPayloadError::ResourceAccounting(error.to_string())
                })?;
                tensor_allocations.extend(parts.tensor_allocations().iter().map(|allocation| {
                    NativeModelTensorResidentAllocation {
                        storage_id: allocation.storage_id(),
                        resident_bytes: allocation.resident_bytes(),
                    }
                }));
                vec![NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::ClipVision,
                    address: Arc::as_ptr(clip_vision) as usize,
                    resident_bytes: parts.owned_bytes(),
                }]
            }
            NativeModelResource::DecoderClip { tokenizer, decoder } => {
                tensor_allocations.extend(decoder.resident_tensor_allocations().into_iter().map(
                    |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                        storage_id,
                        resident_bytes,
                    },
                ));
                vec![
                    NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativePromptTokenizer,
                        address: Arc::as_ptr(tokenizer) as usize,
                        resident_bytes: tokenizer.resident_bytes().map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?,
                    },
                    NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativeDecoderTextEncoder,
                        address: Arc::as_ptr(decoder) as usize,
                        resident_bytes: decoder.resident_owned_bytes().map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?,
                    },
                ]
            }
            NativeModelResource::QwenMultimodalClip { resource } => {
                tensor_allocations.extend(
                    resource
                        .resident_tensor_allocations()
                        .map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?
                        .into_iter()
                        .map(
                            |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                                storage_id,
                                resident_bytes,
                            },
                        ),
                );
                vec![
                    NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativeQwenMultimodal,
                        address: Arc::as_ptr(resource) as usize,
                        resident_bytes: resource.resident_owned_bytes().map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?,
                    },
                    NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativePromptTokenizer,
                        address: Arc::as_ptr(resource.tokenizer()) as usize,
                        resident_bytes: resource.tokenizer().resident_bytes().map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?,
                    },
                    NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativeDecoderTextEncoder,
                        address: Arc::as_ptr(resource.decoder()) as usize,
                        resident_bytes: resource.decoder().resident_owned_bytes().map_err(
                            |error| NativeModelPayloadError::ResourceAccounting(error.to_string()),
                        )?,
                    },
                    NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativeQwenVisionEncoder,
                        address: Arc::as_ptr(resource.vision()) as usize,
                        resident_bytes: resource.vision().resident_owned_bytes().map_err(
                            |error| NativeModelPayloadError::ResourceAccounting(error.to_string()),
                        )?,
                    },
                ]
            }
            NativeModelResource::GemmaMultimodalClip { resource } => {
                tensor_allocations.extend(
                    resource
                        .resident_tensor_allocations()
                        .map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?
                        .into_iter()
                        .map(
                            |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                                storage_id,
                                resident_bytes,
                            },
                        ),
                );
                let mut allocations = vec![
                    NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativeGemmaMultimodal,
                        address: Arc::as_ptr(resource) as usize,
                        resident_bytes: resource.resident_owned_bytes().map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?,
                    },
                    NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativePromptTokenizer,
                        address: Arc::as_ptr(resource.tokenizer()) as usize,
                        resident_bytes: resource.tokenizer().resident_bytes().map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?,
                    },
                    NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativeDecoderTextEncoder,
                        address: Arc::as_ptr(resource.decoder()) as usize,
                        resident_bytes: resource.decoder().resident_owned_bytes().map_err(
                            |error| NativeModelPayloadError::ResourceAccounting(error.to_string()),
                        )?,
                    },
                ];
                if let Some(vision) = resource.gemma3_vision() {
                    allocations.push(NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativeGemma3VisionProjector,
                        address: Arc::as_ptr(vision) as usize,
                        resident_bytes: vision.resident_owned_bytes().map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?,
                    });
                }
                if let Some(vision) = resource.gemma4_vision() {
                    allocations.push(NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativeGemma4VisionEncoder,
                        address: Arc::as_ptr(vision) as usize,
                        resident_bytes: vision.resident_owned_bytes().map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?,
                    });
                }
                if let Some(audio) = resource.audio() {
                    allocations.push(NativeModelResidentAllocation {
                        kind: NativeModelBackingKind::NativeGemma4AudioEncoder,
                        address: Arc::as_ptr(audio) as usize,
                        resident_bytes: audio.resident_owned_bytes().map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?,
                    });
                }
                allocations
            }
            NativeModelResource::NativeClip { resource } => {
                let allocations = resource.resident_tensor_allocations().map_err(|error| {
                    NativeModelPayloadError::ResourceAccounting(error.to_string())
                })?;
                tensor_allocations.extend(allocations.into_iter().map(
                    |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                        storage_id,
                        resident_bytes,
                    },
                ));
                let clip_allocations = resource.resident_owned_allocations().map_err(|error| {
                    NativeModelPayloadError::ResourceAccounting(error.to_string())
                })?;
                let mut backing_allocations = Vec::new();
                backing_allocations
                    .try_reserve_exact(clip_allocations.len())
                    .map_err(|_| NativeModelPayloadError::LengthOverflow)?;
                for allocation in clip_allocations {
                    backing_allocations.push(NativeModelResidentAllocation {
                        kind: match allocation.kind() {
                            NativeClipResidentOwnerKind::Resource => {
                                NativeModelBackingKind::NativeClipResource
                            }
                            NativeClipResidentOwnerKind::Component => {
                                NativeModelBackingKind::NativeClipComponent
                            }
                            NativeClipResidentOwnerKind::Tokenizer => {
                                NativeModelBackingKind::NativeClipTokenizer
                            }
                            NativeClipResidentOwnerKind::Encoder => {
                                NativeModelBackingKind::NativeClipEncoder
                            }
                            NativeClipResidentOwnerKind::MappedWeights => {
                                NativeModelBackingKind::NativeClipMappedWeights
                            }
                        },
                        address: allocation.address(),
                        resident_bytes: allocation.resident_bytes(),
                    });
                }
                backing_allocations
            }
            NativeModelResource::SdPoseModel { resource } => {
                tensor_allocations.extend(
                    resource
                        .resident_tensor_allocations()
                        .map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?
                        .into_iter()
                        .map(
                            |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                                storage_id,
                                resident_bytes,
                            },
                        ),
                );
                vec![NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::NativeSdPoseModel,
                    address: Arc::as_ptr(resource) as usize,
                    resident_bytes: resource.resident_owned_bytes().map_err(|error| {
                        NativeModelPayloadError::ResourceAccounting(error.to_string())
                    })?,
                }]
            }
            NativeModelResource::FrameInterpolation { resource } => {
                tensor_allocations.extend(
                    resource
                        .resident_tensor_allocations()
                        .map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?
                        .into_iter()
                        .map(
                            |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                                storage_id,
                                resident_bytes,
                            },
                        ),
                );
                vec![NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::NativeFrameInterpolationModel,
                    address: Arc::as_ptr(resource) as usize,
                    resident_bytes: resource.resident_owned_bytes().map_err(|error| {
                        NativeModelPayloadError::ResourceAccounting(error.to_string())
                    })?,
                }]
            }
            NativeModelResource::LatentUpscaleModel { resource } => {
                tensor_allocations.extend(
                    resource
                        .resident_tensor_allocations()
                        .map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?
                        .into_iter()
                        .map(
                            |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                                storage_id,
                                resident_bytes,
                            },
                        ),
                );
                vec![NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::NativeLatentUpscaleModel,
                    address: Arc::as_ptr(resource) as usize,
                    resident_bytes: resource.resident_owned_bytes().map_err(|error| {
                        NativeModelPayloadError::ResourceAccounting(error.to_string())
                    })?,
                }]
            }
            NativeModelResource::BackgroundRemoval { resource } => {
                tensor_allocations.extend(
                    resource
                        .resident_tensor_allocations()
                        .map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?
                        .into_iter()
                        .map(
                            |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                                storage_id,
                                resident_bytes,
                            },
                        ),
                );
                vec![NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::NativeBackgroundRemovalResource,
                    address: Arc::as_ptr(resource) as usize,
                    resident_bytes: resource.resident_owned_bytes().map_err(|error| {
                        NativeModelPayloadError::ResourceAccounting(error.to_string())
                    })?,
                }]
            }
            NativeModelResource::DepthAnything3 { resource } => {
                tensor_allocations.extend(
                    resource
                        .resident_tensor_allocations()
                        .map_err(|error| {
                            NativeModelPayloadError::ResourceAccounting(error.to_string())
                        })?
                        .into_iter()
                        .map(
                            |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                                storage_id,
                                resident_bytes,
                            },
                        ),
                );
                vec![NativeModelResidentAllocation {
                    kind: NativeModelBackingKind::NativeDepthAnything3Resource,
                    address: Arc::as_ptr(resource) as usize,
                    resident_bytes: resource.resident_owned_bytes().map_err(|error| {
                        NativeModelPayloadError::ResourceAccounting(error.to_string())
                    })?,
                }]
            }
        };
        let parts = NativeModelResidentParts {
            owned_bytes,
            backing_allocations,
            tensor_allocations,
        };
        if parts.resident_bytes()? != self.resident_bytes {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "model payload resident projection",
            ));
        }
        Ok(parts)
    }

    pub fn model(&self) -> Option<NativeExecutableModel<'_>> {
        match &self.resource {
            NativeModelResource::Sd15Model { model } => {
                Some(NativeExecutableModel::Sd15(model.as_ref()))
            }
            NativeModelResource::NativeFamilyModel { resource } => {
                Some(NativeExecutableModel::Family(resource.as_ref()))
            }
            NativeModelResource::Sd1Clip { .. }
            | NativeModelResource::AudioEncoder { .. }
            | NativeModelResource::NativeVae { .. }
            | NativeModelResource::NativeStructuredVae { .. }
            | NativeModelResource::OpticalFlow { .. }
            | NativeModelResource::ClipVision { .. }
            | NativeModelResource::DecoderClip { .. }
            | NativeModelResource::QwenMultimodalClip { .. }
            | NativeModelResource::GemmaMultimodalClip { .. }
            | NativeModelResource::NativeClip { .. }
            | NativeModelResource::SdPoseModel { .. }
            | NativeModelResource::FrameInterpolation { .. }
            | NativeModelResource::LatentUpscaleModel { .. }
            | NativeModelResource::BackgroundRemoval { .. }
            | NativeModelResource::DepthAnything3 { .. } => None,
        }
    }

    pub fn native_family_model_resource(&self) -> Option<&Arc<NativeFamilyModelResource>> {
        match &self.resource {
            NativeModelResource::NativeFamilyModel { resource } => Some(resource),
            _ => None,
        }
    }

    pub fn clip(&self) -> Option<(&Arc<Sd1Tokenizer>, &Arc<LoadedSd1Clip>)> {
        match &self.resource {
            NativeModelResource::Sd1Clip {
                tokenizer, clip, ..
            } => Some((tokenizer, clip)),
            NativeModelResource::Sd15Model { .. }
            | NativeModelResource::NativeFamilyModel { .. }
            | NativeModelResource::AudioEncoder { .. }
            | NativeModelResource::NativeVae { .. }
            | NativeModelResource::NativeStructuredVae { .. }
            | NativeModelResource::OpticalFlow { .. }
            | NativeModelResource::ClipVision { .. }
            | NativeModelResource::DecoderClip { .. }
            | NativeModelResource::QwenMultimodalClip { .. }
            | NativeModelResource::GemmaMultimodalClip { .. }
            | NativeModelResource::NativeClip { .. }
            | NativeModelResource::SdPoseModel { .. }
            | NativeModelResource::FrameInterpolation { .. }
            | NativeModelResource::LatentUpscaleModel { .. }
            | NativeModelResource::BackgroundRemoval { .. }
            | NativeModelResource::DepthAnything3 { .. } => None,
        }
    }

    pub fn vae(&self) -> Option<&Arc<NativeVae>> {
        match &self.resource {
            NativeModelResource::NativeVae { vae } => Some(vae),
            NativeModelResource::Sd15Model { .. }
            | NativeModelResource::NativeFamilyModel { .. }
            | NativeModelResource::AudioEncoder { .. }
            | NativeModelResource::Sd1Clip { .. }
            | NativeModelResource::NativeStructuredVae { .. }
            | NativeModelResource::OpticalFlow { .. }
            | NativeModelResource::ClipVision { .. }
            | NativeModelResource::DecoderClip { .. }
            | NativeModelResource::QwenMultimodalClip { .. }
            | NativeModelResource::GemmaMultimodalClip { .. }
            | NativeModelResource::NativeClip { .. }
            | NativeModelResource::SdPoseModel { .. }
            | NativeModelResource::FrameInterpolation { .. }
            | NativeModelResource::LatentUpscaleModel { .. }
            | NativeModelResource::BackgroundRemoval { .. }
            | NativeModelResource::DepthAnything3 { .. } => None,
        }
    }

    pub fn structured_vae(&self) -> Option<&Arc<NativeStructuredVae>> {
        match &self.resource {
            NativeModelResource::NativeStructuredVae { vae } => Some(vae),
            _ => None,
        }
    }

    pub fn audio_encoder_resource(&self) -> Option<&Arc<NativeAudioEncoder>> {
        match &self.resource {
            NativeModelResource::AudioEncoder { resource } => Some(resource),
            _ => None,
        }
    }

    pub fn optical_flow_resource(&self) -> Option<&Arc<NativeRaftLarge>> {
        match &self.resource {
            NativeModelResource::OpticalFlow { raft } => Some(raft),
            NativeModelResource::Sd15Model { .. }
            | NativeModelResource::NativeFamilyModel { .. }
            | NativeModelResource::AudioEncoder { .. }
            | NativeModelResource::Sd1Clip { .. }
            | NativeModelResource::NativeVae { .. }
            | NativeModelResource::NativeStructuredVae { .. }
            | NativeModelResource::ClipVision { .. }
            | NativeModelResource::DecoderClip { .. }
            | NativeModelResource::QwenMultimodalClip { .. }
            | NativeModelResource::GemmaMultimodalClip { .. }
            | NativeModelResource::NativeClip { .. }
            | NativeModelResource::SdPoseModel { .. }
            | NativeModelResource::FrameInterpolation { .. }
            | NativeModelResource::LatentUpscaleModel { .. }
            | NativeModelResource::BackgroundRemoval { .. }
            | NativeModelResource::DepthAnything3 { .. } => None,
        }
    }

    pub fn clip_vision_resource(&self) -> Option<&Arc<NativeClipVision>> {
        match &self.resource {
            NativeModelResource::ClipVision { clip_vision } => Some(clip_vision),
            NativeModelResource::Sd15Model { .. }
            | NativeModelResource::NativeFamilyModel { .. }
            | NativeModelResource::AudioEncoder { .. }
            | NativeModelResource::Sd1Clip { .. }
            | NativeModelResource::NativeVae { .. }
            | NativeModelResource::NativeStructuredVae { .. }
            | NativeModelResource::OpticalFlow { .. }
            | NativeModelResource::DecoderClip { .. }
            | NativeModelResource::QwenMultimodalClip { .. }
            | NativeModelResource::GemmaMultimodalClip { .. }
            | NativeModelResource::NativeClip { .. }
            | NativeModelResource::SdPoseModel { .. }
            | NativeModelResource::FrameInterpolation { .. }
            | NativeModelResource::LatentUpscaleModel { .. }
            | NativeModelResource::BackgroundRemoval { .. }
            | NativeModelResource::DepthAnything3 { .. } => None,
        }
    }

    pub fn decoder_clip_resource(
        &self,
    ) -> Option<(&Arc<NativePromptTokenizer>, &Arc<NativeDecoderTextEncoder>)> {
        match &self.resource {
            NativeModelResource::DecoderClip { tokenizer, decoder } => Some((tokenizer, decoder)),
            _ => None,
        }
    }

    pub fn qwen_multimodal_resource(&self) -> Option<&Arc<NativeQwenMultimodal>> {
        match &self.resource {
            NativeModelResource::QwenMultimodalClip { resource } => Some(resource),
            _ => None,
        }
    }

    pub fn gemma_multimodal_resource(&self) -> Option<&Arc<NativeGemmaMultimodal>> {
        match &self.resource {
            NativeModelResource::GemmaMultimodalClip { resource } => Some(resource),
            _ => None,
        }
    }

    pub fn native_clip_resource(&self) -> Option<&Arc<NativeClipResource>> {
        match &self.resource {
            NativeModelResource::NativeClip { resource } => Some(resource),
            _ => None,
        }
    }

    pub fn sdpose_model_resource(&self) -> Option<&Arc<NativeSdPoseModel>> {
        match &self.resource {
            NativeModelResource::SdPoseModel { resource } => Some(resource),
            _ => None,
        }
    }

    pub fn frame_interpolation_resource(&self) -> Option<&Arc<NativeFrameInterpolationModel>> {
        match &self.resource {
            NativeModelResource::FrameInterpolation { resource } => Some(resource),
            _ => None,
        }
    }

    pub fn latent_upscale_model_resource(&self) -> Option<&Arc<NativeLatentUpscaleModelResource>> {
        match &self.resource {
            NativeModelResource::LatentUpscaleModel { resource } => Some(resource),
            _ => None,
        }
    }

    pub fn background_removal_resource(&self) -> Option<&Arc<NativeBackgroundRemovalResource>> {
        match &self.resource {
            NativeModelResource::BackgroundRemoval { resource } => Some(resource),
            _ => None,
        }
    }

    pub fn depth_anything_3_resource(&self) -> Option<&Arc<NativeDepthAnything3Resource>> {
        match &self.resource {
            NativeModelResource::DepthAnything3 { resource } => Some(resource),
            _ => None,
        }
    }

    pub fn validate(&self) -> Result<(), NativeModelPayloadError> {
        let expected = match &self.resource {
            NativeModelResource::Sd15Model { model } => Self::sd15_model(model.clone())?,
            NativeModelResource::NativeFamilyModel { resource } => {
                Self::native_family_model(resource.clone())?
            }
            NativeModelResource::AudioEncoder { resource } => {
                Self::audio_encoder(resource.clone())?
            }
            NativeModelResource::Sd1Clip {
                tokenizer, clip, ..
            } => Self::sd1_clip(tokenizer.clone(), clip.clone())?,
            NativeModelResource::NativeVae { vae } => Self::native_vae(vae.clone())?,
            NativeModelResource::NativeStructuredVae { vae } => {
                Self::native_structured_vae(vae.clone())?
            }
            NativeModelResource::OpticalFlow { raft } => Self::optical_flow(raft.clone())?,
            NativeModelResource::ClipVision { clip_vision } => {
                Self::clip_vision(clip_vision.clone())?
            }
            NativeModelResource::DecoderClip { tokenizer, decoder } => {
                Self::decoder_clip(tokenizer.clone(), decoder.clone())?
            }
            NativeModelResource::QwenMultimodalClip { resource } => {
                Self::qwen_multimodal_clip(resource.clone())?
            }
            NativeModelResource::GemmaMultimodalClip { resource } => {
                Self::gemma_multimodal_clip(resource.clone())?
            }
            NativeModelResource::NativeClip { resource } => Self::native_clip(resource.clone())?,
            NativeModelResource::SdPoseModel { resource } => {
                if resource.is_source_exact_profile() {
                    Self::sdpose_model(resource.clone())?
                } else {
                    #[cfg(feature = "test-support")]
                    {
                        Self::sdpose_model_test_fixture(resource.clone())?
                    }
                    #[cfg(not(feature = "test-support"))]
                    {
                        return Err(NativeModelPayloadError::ResourceMismatch(
                            "SDPose reduced test fixture profile",
                        ));
                    }
                }
            }
            NativeModelResource::FrameInterpolation { resource } => {
                Self::frame_interpolation(resource.clone())?
            }
            NativeModelResource::LatentUpscaleModel { resource } => {
                Self::latent_upscale_model(resource.clone())?
            }
            NativeModelResource::BackgroundRemoval { resource } => {
                if resource.is_source_exact_profile() {
                    Self::background_removal(resource.clone(), &CancellationToken::default())?
                } else {
                    #[cfg(feature = "test-support")]
                    {
                        Self::background_removal_test_fixture(
                            resource.clone(),
                            &CancellationToken::default(),
                        )?
                    }
                    #[cfg(not(feature = "test-support"))]
                    {
                        return Err(NativeModelPayloadError::ResourceMismatch(
                            "BiRefNet reduced test fixture profile",
                        ));
                    }
                }
            }
            NativeModelResource::DepthAnything3 { resource } => {
                if resource.is_source_exact_profile() {
                    Self::depth_anything_3(resource.clone(), &CancellationToken::default())?
                } else {
                    #[cfg(feature = "test-support")]
                    {
                        Self::depth_anything_3_test_fixture(
                            resource.clone(),
                            &CancellationToken::default(),
                        )?
                    }
                    #[cfg(not(feature = "test-support"))]
                    {
                        return Err(NativeModelPayloadError::ResourceMismatch(
                            "Depth Anything 3 reduced test fixture profile",
                        ));
                    }
                }
            }
        };
        if self.identity() != expected.identity() || self.resident_bytes != expected.resident_bytes
        {
            return Err(NativeModelPayloadError::ResourceMismatch(
                "model payload identity",
            ));
        }
        Ok(())
    }
}

const MAX_AUDIO_ENCODER_LAYERS: usize = 65_536;
const MAX_LOSS_MAP_ENTRIES: usize = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioEncoderOutputKind {
    Layered,
    WanDancer,
}

#[derive(Clone, Debug)]
pub struct AudioEncoderOutput {
    resource: AudioEncoderOutputResource,
    semantic_digest_sha256: [u8; 32],
    resident_bytes: u64,
}

#[derive(Clone, Debug)]
enum AudioEncoderOutputResource {
    Layered {
        encoded_audio: Tensor,
        encoded_audio_all_layers: Box<[Tensor]>,
        audio_samples: u64,
    },
    WanDancer {
        audio_feature: Tensor,
        fps: f64,
        audio_inject_scale: f64,
    },
}

impl AudioEncoderOutput {
    pub const SOURCE_TYPE_ID: &'static str = "AUDIO_ENCODER_OUTPUT";

    pub fn layered(
        encoded_audio: Tensor,
        encoded_audio_all_layers: Vec<Tensor>,
        audio_samples: u64,
    ) -> Result<Self, NativeModelPayloadError> {
        Self::layered_inner(encoded_audio, encoded_audio_all_layers, audio_samples, None)
    }

    pub fn layered_with_cancellation(
        encoded_audio: Tensor,
        encoded_audio_all_layers: Vec<Tensor>,
        audio_samples: u64,
        cancellation: &CancellationToken,
    ) -> Result<Self, NativeModelPayloadError> {
        Self::layered_inner(
            encoded_audio,
            encoded_audio_all_layers,
            audio_samples,
            Some(cancellation),
        )
    }

    fn layered_inner(
        encoded_audio: Tensor,
        encoded_audio_all_layers: Vec<Tensor>,
        audio_samples: u64,
        cancellation: Option<&CancellationToken>,
    ) -> Result<Self, NativeModelPayloadError> {
        let resource = AudioEncoderOutputResource::Layered {
            encoded_audio,
            encoded_audio_all_layers: encoded_audio_all_layers.into_boxed_slice(),
            audio_samples,
        };
        let (semantic_digest_sha256, resident_bytes) =
            project_audio_encoder_output_inner(&resource, cancellation)?;
        Ok(Self {
            resource,
            semantic_digest_sha256,
            resident_bytes,
        })
    }

    pub fn wan_dancer(
        audio_feature: Tensor,
        fps: f64,
        audio_inject_scale: f64,
    ) -> Result<Self, NativeModelPayloadError> {
        let resource = AudioEncoderOutputResource::WanDancer {
            audio_feature,
            fps,
            audio_inject_scale,
        };
        let (semantic_digest_sha256, resident_bytes) = project_audio_encoder_output(&resource)?;
        Ok(Self {
            resource,
            semantic_digest_sha256,
            resident_bytes,
        })
    }

    pub const fn kind(&self) -> AudioEncoderOutputKind {
        match &self.resource {
            AudioEncoderOutputResource::Layered { .. } => AudioEncoderOutputKind::Layered,
            AudioEncoderOutputResource::WanDancer { .. } => AudioEncoderOutputKind::WanDancer,
        }
    }

    pub fn encoded_audio(&self) -> Option<&Tensor> {
        match &self.resource {
            AudioEncoderOutputResource::Layered { encoded_audio, .. } => Some(encoded_audio),
            AudioEncoderOutputResource::WanDancer { .. } => None,
        }
    }

    pub fn encoded_audio_all_layers(&self) -> Option<&[Tensor]> {
        match &self.resource {
            AudioEncoderOutputResource::Layered {
                encoded_audio_all_layers,
                ..
            } => Some(encoded_audio_all_layers),
            AudioEncoderOutputResource::WanDancer { .. } => None,
        }
    }

    pub const fn audio_samples(&self) -> Option<u64> {
        match &self.resource {
            AudioEncoderOutputResource::Layered { audio_samples, .. } => Some(*audio_samples),
            AudioEncoderOutputResource::WanDancer { .. } => None,
        }
    }

    pub fn audio_feature(&self) -> Option<&Tensor> {
        match &self.resource {
            AudioEncoderOutputResource::WanDancer { audio_feature, .. } => Some(audio_feature),
            AudioEncoderOutputResource::Layered { .. } => None,
        }
    }

    pub const fn fps(&self) -> Option<f64> {
        match &self.resource {
            AudioEncoderOutputResource::WanDancer { fps, .. } => Some(*fps),
            AudioEncoderOutputResource::Layered { .. } => None,
        }
    }

    pub const fn audio_inject_scale(&self) -> Option<f64> {
        match &self.resource {
            AudioEncoderOutputResource::WanDancer {
                audio_inject_scale, ..
            } => Some(*audio_inject_scale),
            AudioEncoderOutputResource::Layered { .. } => None,
        }
    }

    pub const fn semantic_digest_sha256(&self) -> &[u8; 32] {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn resident_parts(&self) -> Result<NativeStructuredResidentParts, NativeModelPayloadError> {
        let owned_bytes = match &self.resource {
            AudioEncoderOutputResource::Layered {
                encoded_audio_all_layers,
                ..
            } => u64::try_from(mem::size_of::<Self>())
                .map_err(|_| NativeModelPayloadError::StructuredResidentBytesOverflow)?
                .checked_add(
                    u64::try_from(
                        mem::size_of::<Tensor>()
                            .checked_mul(encoded_audio_all_layers.len())
                            .ok_or(NativeModelPayloadError::StructuredResidentBytesOverflow)?,
                    )
                    .map_err(|_| NativeModelPayloadError::StructuredResidentBytesOverflow)?,
                )
                .ok_or(NativeModelPayloadError::StructuredResidentBytesOverflow)?,
            AudioEncoderOutputResource::WanDancer { .. } => {
                u64::try_from(mem::size_of::<Self>())
                    .map_err(|_| NativeModelPayloadError::StructuredResidentBytesOverflow)?
            }
        };
        match &self.resource {
            AudioEncoderOutputResource::Layered {
                encoded_audio,
                encoded_audio_all_layers,
                ..
            } => structured_resident_parts(
                owned_bytes,
                std::iter::once(encoded_audio).chain(encoded_audio_all_layers.iter()),
            ),
            AudioEncoderOutputResource::WanDancer { audio_feature, .. } => {
                structured_resident_parts(owned_bytes, std::iter::once(audio_feature))
            }
        }
    }

    pub fn validate(&self) -> Result<(), NativeModelPayloadError> {
        let (semantic_digest_sha256, resident_bytes) =
            project_audio_encoder_output(&self.resource)?;
        require_structured_projection(
            self.semantic_digest_sha256,
            semantic_digest_sha256,
            self.resident_bytes,
            resident_bytes,
        )?;
        if self.resident_parts()?.resident_bytes()? != self.resident_bytes {
            return Err(NativeModelPayloadError::StructuredProjectionChanged);
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct IcLoraParameters {
    reference_downscale_factor: NonZeroU64,
    semantic_digest_sha256: [u8; 32],
    resident_bytes: u64,
}

impl IcLoraParameters {
    pub const SOURCE_TYPE_ID: &'static str = "IC_LORA_PARAMETERS";

    pub fn checked(reference_downscale_factor: u64) -> Result<Self, NativeModelPayloadError> {
        let reference_downscale_factor = NonZeroU64::new(reference_downscale_factor).ok_or(
            NativeModelPayloadError::InvalidStructuredPayload(
                "IC-LoRA reference downscale factor must be nonzero",
            ),
        )?;
        let (semantic_digest_sha256, resident_bytes) =
            project_ic_lora_parameters(reference_downscale_factor)?;
        Ok(Self {
            reference_downscale_factor,
            semantic_digest_sha256,
            resident_bytes,
        })
    }

    pub const fn reference_downscale_factor(&self) -> u64 {
        self.reference_downscale_factor.get()
    }

    pub const fn semantic_digest_sha256(&self) -> &[u8; 32] {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn validate(&self) -> Result<(), NativeModelPayloadError> {
        let (semantic_digest_sha256, resident_bytes) =
            project_ic_lora_parameters(self.reference_downscale_factor)?;
        require_structured_projection(
            self.semantic_digest_sha256,
            semantic_digest_sha256,
            self.resident_bytes,
            resident_bytes,
        )
    }
}

#[derive(Clone, Debug)]
pub struct LossMap {
    losses: Box<[Tensor]>,
    semantic_digest_sha256: [u8; 32],
    resident_bytes: u64,
}

impl LossMap {
    pub const SOURCE_TYPE_ID: &'static str = "LOSS_MAP";

    pub fn checked(losses: Vec<Tensor>) -> Result<Self, NativeModelPayloadError> {
        let losses = losses.into_boxed_slice();
        let (semantic_digest_sha256, resident_bytes) = project_loss_map(&losses)?;
        Ok(Self {
            losses,
            semantic_digest_sha256,
            resident_bytes,
        })
    }

    pub fn losses(&self) -> &[Tensor] {
        &self.losses
    }

    pub const fn semantic_digest_sha256(&self) -> &[u8; 32] {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn resident_parts(&self) -> Result<NativeStructuredResidentParts, NativeModelPayloadError> {
        let owned_bytes = u64::try_from(mem::size_of::<Self>())
            .map_err(|_| NativeModelPayloadError::StructuredResidentBytesOverflow)?
            .checked_add(
                u64::try_from(
                    mem::size_of::<Tensor>()
                        .checked_mul(self.losses.len())
                        .ok_or(NativeModelPayloadError::StructuredResidentBytesOverflow)?,
                )
                .map_err(|_| NativeModelPayloadError::StructuredResidentBytesOverflow)?,
            )
            .ok_or(NativeModelPayloadError::StructuredResidentBytesOverflow)?;
        structured_resident_parts(owned_bytes, self.losses.iter())
    }

    pub fn validate(&self) -> Result<(), NativeModelPayloadError> {
        let (semantic_digest_sha256, resident_bytes) = project_loss_map(&self.losses)?;
        require_structured_projection(
            self.semantic_digest_sha256,
            semantic_digest_sha256,
            self.resident_bytes,
            resident_bytes,
        )?;
        if self.resident_parts()?.resident_bytes()? != self.resident_bytes {
            return Err(NativeModelPayloadError::StructuredProjectionChanged);
        }
        Ok(())
    }
}

fn structured_resident_parts<'a>(
    owned_bytes: u64,
    tensors: impl IntoIterator<Item = &'a Tensor>,
) -> Result<NativeStructuredResidentParts, NativeModelPayloadError> {
    let mut storages = BTreeMap::new();
    for tensor in tensors {
        let storage_id = tensor.storage_id();
        let resident_bytes = tensor.storage_byte_len();
        if let Some(existing) = storages.insert(storage_id.get(), (storage_id, resident_bytes))
            && existing.1 != resident_bytes
        {
            return Err(NativeModelPayloadError::StructuredProjectionChanged);
        }
    }
    let parts = NativeStructuredResidentParts {
        owned_bytes,
        tensor_allocations: storages
            .into_values()
            .map(
                |(storage_id, resident_bytes)| NativeModelTensorResidentAllocation {
                    storage_id,
                    resident_bytes,
                },
            )
            .collect(),
    };
    parts.resident_bytes()?;
    Ok(parts)
}

fn project_audio_encoder_output(
    resource: &AudioEncoderOutputResource,
) -> Result<([u8; 32], u64), NativeModelPayloadError> {
    project_audio_encoder_output_inner(resource, None)
}

fn project_audio_encoder_output_inner(
    resource: &AudioEncoderOutputResource,
    cancellation: Option<&CancellationToken>,
) -> Result<([u8; 32], u64), NativeModelPayloadError> {
    check_projection_cancellation(cancellation)?;
    let mut projection = NativeStructuredProjection::new::<AudioEncoderOutput>(
        b"zed.comfy.model.audio-encoder-output.v1",
    )?;
    match resource {
        AudioEncoderOutputResource::Layered {
            encoded_audio,
            encoded_audio_all_layers,
            audio_samples,
        } => {
            if encoded_audio_all_layers.is_empty()
                || encoded_audio_all_layers.len() > MAX_AUDIO_ENCODER_LAYERS
                || *audio_samples == 0
            {
                return Err(NativeModelPayloadError::InvalidStructuredPayload(
                    "layered audio encoder cardinality is invalid",
                ));
            }
            require_tensor_shape(encoded_audio, 3, None, "encoded audio")?;
            projection.hash_tag(1);
            projection.hash_float_tensor_inner(encoded_audio, cancellation)?;
            projection.hash_len(encoded_audio_all_layers.len())?;
            projection.add_allocation::<Tensor>(encoded_audio_all_layers.len())?;
            for layer in encoded_audio_all_layers {
                check_projection_cancellation(cancellation)?;
                require_tensor_shape(layer, 3, Some(encoded_audio), "encoded audio layer")?;
                projection.hash_float_tensor_inner(layer, cancellation)?;
            }
            let final_layer = encoded_audio_all_layers.last().ok_or(
                NativeModelPayloadError::InvalidStructuredPayload(
                    "layered audio encoder output has no final layer",
                ),
            )?;
            require_same_tensor_value_inner(
                encoded_audio,
                final_layer,
                "encoded audio final layer",
                cancellation,
            )?;
            projection.hash_u64(*audio_samples);
        }
        AudioEncoderOutputResource::WanDancer {
            audio_feature,
            fps,
            audio_inject_scale,
        } => {
            let shape = audio_feature.descriptor().shape();
            if shape.len() != 3
                || shape.first().copied() != Some(1)
                || shape.get(1).copied() == Some(0)
                || shape.get(2).copied() != Some(35)
                || !fps.is_finite()
                || *fps <= 0.0
                || !audio_inject_scale.is_finite()
                || !(0.0..=10.0).contains(audio_inject_scale)
            {
                return Err(NativeModelPayloadError::InvalidStructuredPayload(
                    "WanDancer audio encoder shape or scalar is invalid",
                ));
            }
            projection.hash_tag(2);
            projection.hash_float_tensor(audio_feature)?;
            projection.hash_f64(*fps);
            projection.hash_f64(*audio_inject_scale);
        }
    }
    check_projection_cancellation(cancellation)?;
    Ok(projection.finish())
}

fn project_ic_lora_parameters(
    reference_downscale_factor: NonZeroU64,
) -> Result<([u8; 32], u64), NativeModelPayloadError> {
    let mut projection = NativeStructuredProjection::new::<IcLoraParameters>(
        b"zed.comfy.model.ic-lora-parameters.v1",
    )?;
    projection.hash_u64(reference_downscale_factor.get());
    Ok(projection.finish())
}

fn project_loss_map(losses: &[Tensor]) -> Result<([u8; 32], u64), NativeModelPayloadError> {
    if losses.is_empty() || losses.len() > MAX_LOSS_MAP_ENTRIES {
        return Err(NativeModelPayloadError::InvalidStructuredPayload(
            "loss map cardinality is invalid",
        ));
    }
    let mut projection =
        NativeStructuredProjection::new::<LossMap>(b"zed.comfy.model.loss-map.v1")?;
    projection.hash_len(losses.len())?;
    projection.add_allocation::<Tensor>(losses.len())?;
    for loss in losses {
        if loss.descriptor().element_count()? == 0 {
            return Err(NativeModelPayloadError::InvalidStructuredPayload(
                "loss map contains an empty tensor",
            ));
        }
        projection.hash_float_tensor(loss)?;
    }
    Ok(projection.finish())
}

#[derive(Debug, Error)]
pub enum NativeModelPayloadError {
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error("native model resource schema {0} is unsupported")]
    UnsupportedSchema(u16),
    #[error("native model resource {0} is invalid")]
    InvalidIdentity(&'static str),
    #[error("native model resource digest is invalid")]
    InvalidDigest,
    #[error("native model resource identity does not match its canonical digest")]
    IdentityMismatch,
    #[error("native model resource identity length exceeds the portable u64 range")]
    LengthOverflow,
    #[error("native model resource {0} does not match its concrete payload")]
    ResourceMismatch(&'static str),
    #[error("native model resource accounting failed: {0}")]
    ResourceAccounting(String),
    #[error("native model structured payload is invalid: {0}")]
    InvalidStructuredPayload(&'static str),
    #[error("native model structured payload projection changed")]
    StructuredProjectionChanged,
    #[error("native model structured payload resident-byte accounting overflowed")]
    StructuredResidentBytesOverflow,
}

pub(crate) struct NativeStructuredProjection {
    hasher: Sha256,
    resident_bytes: u64,
    storage_ids: BTreeSet<u64>,
}

impl NativeStructuredProjection {
    pub(crate) fn new<Payload>(domain: &[u8]) -> Result<Self, NativeModelPayloadError> {
        let mut hasher = Sha256::new();
        hasher.update(domain);
        hasher.update([0]);
        Ok(Self {
            hasher,
            resident_bytes: u64::try_from(mem::size_of::<Payload>())
                .map_err(|_| NativeModelPayloadError::StructuredResidentBytesOverflow)?,
            storage_ids: BTreeSet::new(),
        })
    }

    pub(crate) fn hash_tag(&mut self, tag: u8) {
        self.hasher.update([tag]);
    }

    pub(crate) fn hash_len(&mut self, length: usize) -> Result<(), NativeModelPayloadError> {
        self.hash_u64(
            u64::try_from(length)
                .map_err(|_| NativeModelPayloadError::StructuredResidentBytesOverflow)?,
        );
        Ok(())
    }

    pub(crate) fn hash_u64(&mut self, value: u64) {
        self.hasher.update(value.to_le_bytes());
    }

    pub(crate) fn hash_f64(&mut self, value: f64) {
        self.hasher.update(value.to_bits().to_le_bytes());
    }

    pub(crate) fn add_allocation<Value>(
        &mut self,
        capacity: usize,
    ) -> Result<(), NativeModelPayloadError> {
        let bytes = mem::size_of::<Value>()
            .checked_mul(capacity)
            .ok_or(NativeModelPayloadError::StructuredResidentBytesOverflow)?;
        self.add_bytes(bytes)
    }

    pub(crate) fn hash_float_tensor(
        &mut self,
        tensor: &Tensor,
    ) -> Result<(), NativeModelPayloadError> {
        self.hash_float_tensor_inner(tensor, None)
    }

    fn hash_float_tensor_inner(
        &mut self,
        tensor: &Tensor,
        cancellation: Option<&CancellationToken>,
    ) -> Result<(), NativeModelPayloadError> {
        check_projection_cancellation(cancellation)?;
        let descriptor = tensor.descriptor();
        if !descriptor.is_contiguous()? {
            return Err(NativeModelPayloadError::InvalidStructuredPayload(
                "structured tensor must be contiguous",
            ));
        }
        let dtype_tag = match descriptor.dtype() {
            DType::F64 => 1,
            DType::F32 => 2,
            DType::F16 => 3,
            DType::Bf16 => 4,
            _ => {
                return Err(NativeModelPayloadError::InvalidStructuredPayload(
                    "structured tensor must use a finite floating-point dtype",
                ));
            }
        };
        self.hash_tag(dtype_tag);
        self.hash_len(descriptor.rank())?;
        for dimension in descriptor.shape() {
            self.hash_u64(*dimension);
        }
        let bytes = tensor.contiguous_bytes()?;
        self.hash_len(bytes.len())?;
        hash_finite_float_bytes(&mut self.hasher, descriptor.dtype(), bytes, cancellation)?;
        if self.storage_ids.insert(tensor.storage_id().get()) {
            self.resident_bytes = self
                .resident_bytes
                .checked_add(tensor.storage_byte_len())
                .ok_or(NativeModelPayloadError::StructuredResidentBytesOverflow)?;
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> ([u8; 32], u64) {
        (self.hasher.finalize().into(), self.resident_bytes)
    }

    fn add_bytes(&mut self, bytes: usize) -> Result<(), NativeModelPayloadError> {
        let bytes = u64::try_from(bytes)
            .map_err(|_| NativeModelPayloadError::StructuredResidentBytesOverflow)?;
        self.resident_bytes = self
            .resident_bytes
            .checked_add(bytes)
            .ok_or(NativeModelPayloadError::StructuredResidentBytesOverflow)?;
        Ok(())
    }
}

fn hash_finite_float_bytes(
    hasher: &mut Sha256,
    dtype: DType,
    bytes: &[u8],
    cancellation: Option<&CancellationToken>,
) -> Result<(), NativeModelPayloadError> {
    const CANCELLATION_INTERVAL_BYTES: usize = 64 * 1024;
    match dtype {
        DType::F64 => {
            for (index, chunk) in bytes.chunks_exact(8).enumerate() {
                if index.is_multiple_of(CANCELLATION_INTERVAL_BYTES / 8) {
                    check_projection_cancellation(cancellation)?;
                }
                let bits = u64::from_ne_bytes(copy_array(chunk)?);
                if !f64::from_bits(bits).is_finite() {
                    return Err(NativeModelPayloadError::InvalidStructuredPayload(
                        "structured tensor contains a non-finite value",
                    ));
                }
                hasher.update(bits.to_le_bytes());
            }
            require_exact_chunks(bytes, 8)?;
        }
        DType::F32 => {
            for (index, chunk) in bytes.chunks_exact(4).enumerate() {
                if index.is_multiple_of(CANCELLATION_INTERVAL_BYTES / 4) {
                    check_projection_cancellation(cancellation)?;
                }
                let bits = u32::from_ne_bytes(copy_array(chunk)?);
                if !f32::from_bits(bits).is_finite() {
                    return Err(NativeModelPayloadError::InvalidStructuredPayload(
                        "structured tensor contains a non-finite value",
                    ));
                }
                hasher.update(bits.to_le_bytes());
            }
            require_exact_chunks(bytes, 4)?;
        }
        DType::F16 => {
            for (index, chunk) in bytes.chunks_exact(2).enumerate() {
                if index.is_multiple_of(CANCELLATION_INTERVAL_BYTES / 2) {
                    check_projection_cancellation(cancellation)?;
                }
                let bits = u16::from_ne_bytes(copy_array(chunk)?);
                if bits & 0x7c00 == 0x7c00 {
                    return Err(NativeModelPayloadError::InvalidStructuredPayload(
                        "structured tensor contains a non-finite value",
                    ));
                }
                hasher.update(bits.to_le_bytes());
            }
            require_exact_chunks(bytes, 2)?;
        }
        DType::Bf16 => {
            for (index, chunk) in bytes.chunks_exact(2).enumerate() {
                if index.is_multiple_of(CANCELLATION_INTERVAL_BYTES / 2) {
                    check_projection_cancellation(cancellation)?;
                }
                let bits = u16::from_ne_bytes(copy_array(chunk)?);
                if bits & 0x7f80 == 0x7f80 {
                    return Err(NativeModelPayloadError::InvalidStructuredPayload(
                        "structured tensor contains a non-finite value",
                    ));
                }
                hasher.update(bits.to_le_bytes());
            }
            require_exact_chunks(bytes, 2)?;
        }
        _ => {
            return Err(NativeModelPayloadError::InvalidStructuredPayload(
                "structured tensor must use a finite floating-point dtype",
            ));
        }
    }
    check_projection_cancellation(cancellation)?;
    Ok(())
}

fn check_projection_cancellation(
    cancellation: Option<&CancellationToken>,
) -> Result<(), NativeModelPayloadError> {
    if let Some(cancellation) = cancellation {
        cancellation
            .check()
            .map_err(|error| NativeModelPayloadError::Tensor(error.into()))?;
    }
    Ok(())
}

fn copy_array<const LENGTH: usize>(bytes: &[u8]) -> Result<[u8; LENGTH], NativeModelPayloadError> {
    bytes.try_into().map_err(|_| {
        NativeModelPayloadError::InvalidStructuredPayload(
            "structured tensor byte width does not match its dtype",
        )
    })
}

fn require_exact_chunks(bytes: &[u8], width: usize) -> Result<(), NativeModelPayloadError> {
    if bytes.len().is_multiple_of(width) {
        Ok(())
    } else {
        Err(NativeModelPayloadError::InvalidStructuredPayload(
            "structured tensor byte width does not match its dtype",
        ))
    }
}

fn require_tensor_shape(
    tensor: &Tensor,
    rank: usize,
    reference: Option<&Tensor>,
    field: &'static str,
) -> Result<(), NativeModelPayloadError> {
    let descriptor = tensor.descriptor();
    if descriptor.rank() != rank || descriptor.shape().contains(&0) {
        return Err(NativeModelPayloadError::InvalidStructuredPayload(field));
    }
    if let Some(reference) = reference {
        let reference = reference.descriptor();
        if descriptor.shape() != reference.shape()
            || descriptor.dtype() != reference.dtype()
            || descriptor.device() != reference.device()
            || descriptor.stream() != reference.stream()
        {
            return Err(NativeModelPayloadError::InvalidStructuredPayload(field));
        }
    }
    Ok(())
}

fn require_same_tensor_value_inner(
    left: &Tensor,
    right: &Tensor,
    field: &'static str,
    cancellation: Option<&CancellationToken>,
) -> Result<(), NativeModelPayloadError> {
    if left.descriptor().shape() != right.descriptor().shape()
        || left.descriptor().dtype() != right.descriptor().dtype()
    {
        return Err(NativeModelPayloadError::InvalidStructuredPayload(field));
    }
    let left_bytes = left.contiguous_bytes()?;
    let right_bytes = right.contiguous_bytes()?;
    if left_bytes.len() != right_bytes.len() {
        return Err(NativeModelPayloadError::InvalidStructuredPayload(field));
    }
    for (left_chunk, right_chunk) in left_bytes
        .chunks(64 * 1024)
        .zip(right_bytes.chunks(64 * 1024))
    {
        check_projection_cancellation(cancellation)?;
        if left_chunk != right_chunk {
            return Err(NativeModelPayloadError::InvalidStructuredPayload(field));
        }
    }
    check_projection_cancellation(cancellation)?;
    Ok(())
}

pub(crate) fn require_structured_projection(
    actual_digest: [u8; 32],
    expected_digest: [u8; 32],
    actual_resident_bytes: u64,
    expected_resident_bytes: u64,
) -> Result<(), NativeModelPayloadError> {
    if actual_digest != expected_digest || actual_resident_bytes != expected_resident_bytes {
        return Err(NativeModelPayloadError::StructuredProjectionChanged);
    }
    Ok(())
}

fn payload_resident_bytes(
    identity: &NativeModelResourceIdentity,
    backing_bytes: u64,
) -> Result<u64, NativeModelPayloadError> {
    u64::try_from(mem::size_of::<NativeModelPayload>())
        .map_err(|_| NativeModelPayloadError::LengthOverflow)?
        .checked_add(identity.resident_owned_bytes()?)
        .ok_or(NativeModelPayloadError::LengthOverflow)?
        .checked_add(backing_bytes)
        .ok_or(NativeModelPayloadError::LengthOverflow)
}

fn validate_identity_text(field: &'static str, value: &str) -> Result<(), NativeModelPayloadError> {
    if value.is_empty() || value.len() > MAX_IDENTITY_BYTES || value.chars().any(char::is_control) {
        return Err(NativeModelPayloadError::InvalidIdentity(field));
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn digest_fields<const N: usize>(
    tag: &[u8],
    fields: [&str; N],
) -> Result<String, NativeModelPayloadError> {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, tag)?;
    for field in fields {
        hash_field(&mut hasher, field.as_bytes())?;
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) -> Result<(), NativeModelPayloadError> {
    hasher.update(
        u64::try_from(bytes.len())
            .map_err(|_| NativeModelPayloadError::LengthOverflow)?
            .to_le_bytes(),
    );
    hasher.update(bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArtifactAvailability, ArtifactKey, ArtifactRecord, GENERATED_LATENT_FORMATS,
        ModelFamilyIdentity, NativeAudioEncoderArchitecture, NativeModule, PatchGraph,
        VaeArchitectureIdentity, VaeBoundary, VaeDescriptor, VaeError, VaeKernelProfile,
        VaeStructuredDecodeRequest, VaeStructuredOutputKind, VaeStructuredResult,
        audio_encoder::deterministic_reduced_audio_encoder_fixture, vae::VaeModelBinding,
    };
    use comfy_tensor::{
        CancellationToken, CpuBackend, CpuWorkspaceAuthority, DeviceId, ExecutionContext, StreamId,
        TensorDescriptor,
    };
    use std::{collections::BTreeMap, error::Error, path::PathBuf};

    const TEST_MEMORY_LIMIT_BYTES: u64 = 1024 * 1024;

    fn with_context<ResultType>(
        run: impl FnOnce(&CpuBackend, &ExecutionContext<'_>) -> Result<ResultType, Box<dyn Error>>,
    ) -> Result<ResultType, Box<dyn Error>> {
        let cancellation = CancellationToken::default();
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(TEST_MEMORY_LIMIT_BYTES)?;
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(0)?,
            &cancellation,
        );
        run(&backend, &context)
    }

    fn f32_tensor(
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
        shape: Vec<u64>,
        values: &[f32],
    ) -> Result<Tensor, Box<dyn Error>> {
        let descriptor =
            TensorDescriptor::contiguous(shape, DType::F32, DeviceId::CPU, StreamId::DEFAULT)?;
        let (tensor, _) = backend.upload_f32(descriptor, values, context)?;
        Ok(tensor)
    }

    fn loaded_zero_raft() -> Result<Arc<NativeRaftLarge>, Box<dyn Error>> {
        let cancellation = CancellationToken::default();
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(64 * 1024 * 1024)?;
        let mut raft = crate::raft_large_exact_native(false, false, &cancellation)?;
        let mut shared_tensors: Vec<(Vec<u64>, DType, Tensor)> = Vec::new();
        let mut state = BTreeMap::new();
        for spec in raft.state_schema() {
            let tensor = match shared_tensors
                .iter()
                .find(|(shape, dtype, _)| shape == &spec.shape && *dtype == spec.dtype)
            {
                Some((_, _, tensor)) => tensor.clone(),
                None => {
                    let encoded_bytes = spec
                        .shape
                        .iter()
                        .try_fold(spec.dtype.byte_width(), |bytes, dimension| {
                            bytes.checked_mul(*dimension)
                        })
                        .ok_or(NativeModelPayloadError::LengthOverflow)?;
                    let descriptor = TensorDescriptor::contiguous(
                        spec.shape.clone(),
                        spec.dtype,
                        DeviceId::CPU,
                        StreamId::DEFAULT,
                    )?;
                    let context = backend.execution_context(
                        StreamId::DEFAULT,
                        authority.authorize_workspace(encoded_bytes.max(1))?,
                        &cancellation,
                    );
                    let bytes = vec![
                        0_u8;
                        usize::try_from(encoded_bytes)
                            .map_err(|_| NativeModelPayloadError::LengthOverflow)?
                    ];
                    let tensor = backend.upload_bytes(descriptor, &bytes, &context)?.0;
                    shared_tensors.push((spec.shape.clone(), spec.dtype, tensor.clone()));
                    tensor
                }
            };
            state.insert(spec.name.clone(), tensor);
        }
        raft.load_state_dict(state, &cancellation)?;
        raft.eval();
        Ok(Arc::new(raft))
    }

    fn unavailable_structured_decode(
        _module: &NativeModule,
        _backend: &CpuBackend,
        _latent: &Tensor,
        _request: &VaeStructuredDecodeRequest,
        _context: &ExecutionContext<'_>,
    ) -> Result<VaeStructuredResult, VaeError> {
        Err(VaeError::InvalidStructuredResult(
            "transport fixture does not execute the structured decoder".to_owned(),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn structured_vae(
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
        profile: VaeKernelProfile,
        family_feature_id: &str,
        family_identifier: &str,
        family_architecture_version: &str,
        latent_identifier: &str,
        architecture: &str,
        output_kind: VaeStructuredOutputKind,
        digest_byte: char,
    ) -> Result<Arc<NativeStructuredVae>, Box<dyn Error>> {
        let artifact_sha256: String = std::iter::repeat_n(digest_byte, 64).collect();
        let artifact = ArtifactRecord {
            key: ArtifactKey::new("models", PathBuf::from("vae/structured.safetensors"))?,
            namespace: "vae".to_owned(),
            canonical_path: PathBuf::from("/verified/models/vae/structured.safetensors"),
            byte_size: 4,
            modified_nanoseconds: 1,
            sha256: artifact_sha256.clone(),
            availability: ArtifactAvailability::Present,
        };
        let latent_definition = GENERATED_LATENT_FORMATS
            .iter()
            .find(|definition| definition.identifier == latent_identifier)
            .ok_or("missing structured latent definition")?;
        let descriptor = VaeDescriptor::checked(
            &artifact,
            ModelFamilyIdentity::new(
                family_feature_id,
                family_identifier,
                family_architecture_version,
            )?,
            latent_definition,
            VaeArchitectureIdentity::checked(architecture)?,
            PatchGraph::checked_semantic(artifact_sha256, Vec::new())?.identity(),
            DType::F32,
            DeviceId::CPU,
            VaeBoundary::structured_output(latent_definition.channels, output_kind)?,
            profile,
            [-1.0, 1.0],
        )?;
        let module = NativeModule::buffer(
            "structured_vae",
            f32_tensor(backend, context, vec![1], &[1.0])?,
        )?;
        let binding =
            VaeModelBinding::checked_transport_fixture(&descriptor, module, context.cancellation)?;
        Ok(Arc::new(NativeStructuredVae::checked_kernel(
            descriptor,
            latent_definition,
            binding,
            unavailable_structured_decode,
        )?))
    }

    #[test]
    fn native_model_resource_identity_is_role_bound_and_forgery_checked()
    -> Result<(), Box<dyn Error>> {
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Model,
            "sd15",
            "safetensors",
            "1".repeat(64),
            "2".repeat(64),
        )?;
        assert_eq!(identity.role().source_type_id(), "MODEL");
        identity.validate()?;

        let mut encoded = serde_json::to_value(&identity)?;
        encoded["role"] = serde_json::json!("clip");
        assert!(serde_json::from_value::<NativeModelResourceIdentity>(encoded).is_err());
        let mut unknown = serde_json::to_value(&identity)?;
        unknown["unknown"] = serde_json::json!(true);
        assert!(serde_json::from_value::<NativeModelResourceIdentity>(unknown).is_err());
        Ok(())
    }

    #[test]
    fn semantic_identity_is_independent_from_live_residency_and_detects_role_drift()
    -> Result<(), Box<dyn Error>> {
        let mut reserved_identifier = String::with_capacity(128);
        reserved_identifier.push_str("sd15");
        let sd15 = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Model,
            "sd15",
            "safetensors",
            "1".repeat(64),
            "2".repeat(64),
        )?;
        let same_semantics_different_residency = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Model,
            reserved_identifier,
            "safetensors",
            "1".repeat(64),
            "2".repeat(64),
        )?;
        let clip = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Clip,
            "sd15",
            "safetensors",
            "1".repeat(64),
            "2".repeat(64),
        )?;
        let vae_drift = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Vae,
            "sd15",
            "safetensors",
            "1".repeat(64),
            "3".repeat(64),
        )?;
        assert_ne!(sd15.digest_sha256(), clip.digest_sha256());
        assert_ne!(sd15.digest_sha256(), vae_drift.digest_sha256());
        assert_eq!(
            sd15.digest_sha256(),
            same_semantics_different_residency.digest_sha256()
        );
        assert!(
            same_semantics_different_residency.resident_owned_bytes()?
                > sd15.resident_owned_bytes()?
        );
        assert!(serde_json::to_value(sd15)?.get("resident_bytes").is_none());
        Ok(())
    }

    #[test]
    fn optical_flow_payload_binds_canonical_raft_identity_and_exact_backing_arc()
    -> Result<(), Box<dyn Error>> {
        let raft = loaded_zero_raft()?;
        let payload = NativeModelPayload::optical_flow(raft.clone())?;
        payload.validate()?;
        assert_eq!(payload.identity(), raft.semantic_identity()?);
        assert_eq!(
            payload.identity().role(),
            NativeModelResourceRole::OpticalFlow
        );
        assert_eq!(payload.identity().role().source_type_id(), "OPTICAL_FLOW");
        assert_eq!(
            payload.identity().digest_sha256(),
            raft.semantic_digest_sha256()?
        );
        assert!(
            payload
                .optical_flow_resource()
                .is_some_and(|stored| Arc::ptr_eq(stored, &raft))
        );
        assert!(payload.model().is_none());
        assert!(payload.clip().is_none());
        assert!(payload.vae().is_none());

        let parts = payload.resident_parts()?;
        assert_eq!(parts.backing_allocations().len(), 1);
        let backing = parts.backing_allocations().first().ok_or(
            NativeModelPayloadError::ResourceMismatch("OPTICAL_FLOW backing allocation"),
        )?;
        assert_eq!(backing.kind(), NativeModelBackingKind::OpticalFlow);
        assert_eq!(backing.address(), Arc::as_ptr(&raft) as usize);
        assert_eq!(
            backing.resident_bytes(),
            raft.resident_parts()?.owned_bytes()
        );
        assert_eq!(
            parts.tensor_allocations().len(),
            raft.resident_parts()?.tensor_allocations().len()
        );
        assert_eq!(parts.resident_bytes()?, payload.resident_bytes());

        let mut training_raft = (*raft).clone();
        training_raft.train();
        assert!(matches!(
            NativeModelPayload::optical_flow(Arc::new(training_raft)),
            Err(NativeModelPayloadError::ResourceMismatch(
                "OPTICAL_FLOW evaluation state"
            ))
        ));
        Ok(())
    }

    #[test]
    fn audio_encoder_payload_binds_exact_identity_and_alias_aware_residency()
    -> Result<(), Box<dyn Error>> {
        let cancellation = CancellationToken::default();
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(128 * 1024 * 1024)?;
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(64 * 1024 * 1024)?,
            &cancellation,
        );
        let resource = Arc::new(deterministic_reduced_audio_encoder_fixture(
            &backend,
            &context,
            NativeAudioEncoderArchitecture::Wav2Vec2Base,
            0,
        )?);
        let payload = NativeModelPayload::audio_encoder(resource.clone())?;
        payload.validate()?;

        assert_eq!(
            payload.identity().role(),
            NativeModelResourceRole::AudioEncoder
        );
        assert_eq!(payload.identity().identifier(), resource.identifier());
        assert_eq!(payload.identity().format(), "zed-native-audio-encoder-v1");
        assert_eq!(
            payload.identity().artifact_sha256(),
            resource.artifact_sha256()
        );
        assert_eq!(
            payload.identity().execution_sha256(),
            resource.semantic_state_digest_sha256()
        );
        assert!(
            payload
                .audio_encoder_resource()
                .is_some_and(|stored| Arc::ptr_eq(stored, &resource))
        );
        assert!(payload.model().is_none());
        assert!(payload.clip().is_none());
        assert!(payload.vae().is_none());

        let parts = payload.resident_parts()?;
        assert_eq!(parts.backing_allocations().len(), 1);
        let backing = parts.backing_allocations().first().ok_or(
            NativeModelPayloadError::ResourceMismatch("AUDIO_ENCODER backing allocation"),
        )?;
        assert_eq!(backing.kind(), NativeModelBackingKind::NativeAudioEncoder);
        assert_eq!(backing.address(), Arc::as_ptr(&resource) as usize);
        assert_eq!(backing.resident_bytes(), resource.resident_owned_bytes()?);
        assert_eq!(
            parts
                .tensor_allocations()
                .iter()
                .map(|allocation| (allocation.storage_id(), allocation.resident_bytes()))
                .collect::<Vec<_>>(),
            resource.resident_tensor_allocations()?
        );
        let unique_storage_ids = parts
            .tensor_allocations()
            .iter()
            .map(|allocation| allocation.storage_id().get())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique_storage_ids.len(), parts.tensor_allocations().len());
        assert_eq!(parts.resident_bytes()?, payload.resident_bytes());

        let reconstructed =
            NativeModelPayload::audio_encoder(Arc::new(resource.reconstruct(&cancellation)?))?;
        assert_eq!(reconstructed.identity(), payload.identity());
        assert_eq!(reconstructed.resident_bytes(), payload.resident_bytes());

        let changed = NativeModelPayload::audio_encoder(Arc::new(
            deterministic_reduced_audio_encoder_fixture(
                &backend,
                &context,
                NativeAudioEncoderArchitecture::Wav2Vec2Base,
                1,
            )?,
        ))?;
        assert_ne!(changed.identity(), payload.identity());
        Ok(())
    }

    #[test]
    fn structured_vae_payload_retains_shape_and_splat_resources_under_the_vae_role()
    -> Result<(), Box<dyn Error>> {
        with_context(|backend, context| {
            let cases = [
                (
                    VaeKernelProfile::HunyuanShapeV1,
                    "COMFY-MODEL-0084",
                    "Hunyuan3Dv2",
                    "hunyuan3d-v2-flow-transformer-v1",
                    "Hunyuan3Dv2",
                    "comfy.ldm.hunyuan3d.vae.ShapeVAE.v1",
                    VaeStructuredOutputKind::Shape,
                    'a',
                ),
                (
                    VaeKernelProfile::TripoSplatV1,
                    "COMFY-MODEL-0137",
                    "TripoSplat",
                    "triposplat-latent-sequence-flow-v1",
                    "TripoSplat",
                    "comfy.ldm.triposplat.vae.OctreeGaussianDecoder.v1",
                    VaeStructuredOutputKind::GaussianSplats,
                    'b',
                ),
            ];
            let mut identities = Vec::new();
            for (
                profile,
                family_feature_id,
                family,
                family_architecture_version,
                latent,
                architecture,
                output_kind,
                digest_byte,
            ) in cases
            {
                let structured = structured_vae(
                    backend,
                    context,
                    profile,
                    family_feature_id,
                    family,
                    family_architecture_version,
                    latent,
                    architecture,
                    output_kind,
                    digest_byte,
                )?;
                let payload = NativeModelPayload::native_structured_vae(structured.clone())?;
                payload.validate()?;
                assert_eq!(payload.identity().role(), NativeModelResourceRole::Vae);
                assert!(payload.vae().is_none());
                assert!(
                    payload
                        .structured_vae()
                        .is_some_and(|stored| Arc::ptr_eq(stored, &structured))
                );
                let parts = payload.resident_parts()?;
                assert_eq!(parts.backing_allocations().len(), 1);
                assert_eq!(
                    parts.backing_allocations()[0].kind(),
                    NativeModelBackingKind::NativeStructuredVae
                );
                assert_eq!(parts.resident_bytes()?, payload.resident_bytes());
                identities.push(payload.identity().digest_sha256().to_owned());
            }
            assert_ne!(identities[0], identities[1]);
            Ok(())
        })
    }

    #[test]
    fn audio_encoder_output_preserves_both_exact_source_shapes_and_alias_accounting()
    -> Result<(), Box<dyn Error>> {
        with_context(|backend, context| {
            let first_layer = f32_tensor(backend, context, vec![1, 2, 3], &[0.0; 6])?;
            let encoded_audio = f32_tensor(backend, context, vec![1, 2, 3], &[1.0; 6])?;
            let layered = AudioEncoderOutput::layered(
                encoded_audio.clone(),
                vec![first_layer.clone(), encoded_audio.clone()],
                16_000,
            )?;
            layered.validate()?;
            assert_eq!(layered.kind(), AudioEncoderOutputKind::Layered);
            assert_eq!(layered.audio_samples(), Some(16_000));
            assert_eq!(
                layered
                    .encoded_audio_all_layers()
                    .map(|layers| layers.len()),
                Some(2)
            );
            assert_eq!(
                layered.resident_bytes(),
                u64::try_from(mem::size_of::<AudioEncoderOutput>())?
                    + u64::try_from(2 * mem::size_of::<Tensor>())?
                    + first_layer.storage_byte_len()
                    + encoded_audio.storage_byte_len()
            );
            let layered_parts = layered.resident_parts()?;
            assert_eq!(layered_parts.tensor_allocations().len(), 2);
            assert_eq!(layered_parts.resident_bytes()?, layered.resident_bytes());

            let cancelled = CancellationToken::default();
            assert!(cancelled.cancel());
            assert!(matches!(
                AudioEncoderOutput::layered_with_cancellation(
                    encoded_audio.clone(),
                    vec![encoded_audio],
                    16_000,
                    &cancelled,
                ),
                Err(NativeModelPayloadError::Tensor(TensorError::Cancelled))
            ));

            let audio_feature = f32_tensor(backend, context, vec![1, 2, 35], &[0.25; 70])?;
            let dancer = AudioEncoderOutput::wan_dancer(audio_feature, 30.0, 1.5)?;
            dancer.validate()?;
            assert_eq!(dancer.kind(), AudioEncoderOutputKind::WanDancer);
            assert_eq!(dancer.fps(), Some(30.0));
            assert_eq!(dancer.audio_inject_scale(), Some(1.5));
            assert!(dancer.encoded_audio().is_none());

            let invalid_feature = f32_tensor(backend, context, vec![1, 2, 34], &[0.0; 68])?;
            assert!(AudioEncoderOutput::wan_dancer(invalid_feature, 30.0, 1.0).is_err());
            let invalid_scale = f32_tensor(backend, context, vec![1, 2, 35], &[0.0; 70])?;
            assert!(AudioEncoderOutput::wan_dancer(invalid_scale, 30.0, 10.01).is_err());
            Ok(())
        })
    }

    #[test]
    fn structured_payload_digests_are_residency_independent_and_validate_finite_content()
    -> Result<(), Box<dyn Error>> {
        with_context(|backend, context| {
            let first = f32_tensor(backend, context, vec![1, 1, 2], &[1.0, 2.0])?;
            let second = f32_tensor(backend, context, vec![1, 1, 2], &[1.0, 2.0])?;
            assert_ne!(first.storage_id(), second.storage_id());
            let first_output = AudioEncoderOutput::layered(first.clone(), vec![first], 640)?;
            let second_output = AudioEncoderOutput::layered(second.clone(), vec![second], 640)?;
            assert_eq!(
                first_output.semantic_digest_sha256(),
                second_output.semantic_digest_sha256()
            );
            assert_ne!(
                first_output.resident_parts()?.tensor_allocations(),
                second_output.resident_parts()?.tensor_allocations()
            );
            let shared_output = first_output.clone();
            assert_eq!(
                first_output.resident_parts()?.tensor_allocations(),
                shared_output.resident_parts()?.tensor_allocations()
            );

            let non_finite = f32_tensor(backend, context, vec![1], &[f32::NAN])?;
            assert!(LossMap::checked(vec![non_finite]).is_err());
            Ok(())
        })
    }

    #[test]
    fn ic_lora_and_loss_map_keep_checked_source_cardinality_without_shape_coercion()
    -> Result<(), Box<dyn Error>> {
        let parameters = IcLoraParameters::checked(u64::MAX)?;
        parameters.validate()?;
        assert_eq!(parameters.reference_downscale_factor(), u64::MAX);
        assert_eq!(
            parameters.resident_bytes(),
            u64::try_from(mem::size_of::<IcLoraParameters>())?
        );
        assert!(IcLoraParameters::checked(0).is_err());

        with_context(|backend, context| {
            let scalar = f32_tensor(backend, context, Vec::new(), &[0.5])?;
            let matrix = f32_tensor(backend, context, vec![1, 2], &[0.25, 0.75])?;
            let loss_map = LossMap::checked(vec![scalar.clone(), matrix.clone()])?;
            loss_map.validate()?;
            assert_eq!(loss_map.losses().len(), 2);
            assert_eq!(
                loss_map
                    .losses()
                    .first()
                    .map(|loss| loss.descriptor().rank()),
                Some(0)
            );
            assert_eq!(
                loss_map
                    .losses()
                    .get(1)
                    .map(|loss| loss.descriptor().shape()),
                Some(&[1, 2][..])
            );
            assert_eq!(
                loss_map.resident_bytes(),
                u64::try_from(mem::size_of::<LossMap>())?
                    + u64::try_from(2 * mem::size_of::<Tensor>())?
                    + scalar.storage_byte_len()
                    + matrix.storage_byte_len()
            );
            let loss_parts = loss_map.resident_parts()?;
            assert_eq!(loss_parts.tensor_allocations().len(), 2);
            assert_eq!(loss_parts.resident_bytes()?, loss_map.resident_bytes());

            let aliased = LossMap::checked(vec![scalar.clone(), scalar.clone()])?;
            assert_eq!(aliased.resident_parts()?.tensor_allocations().len(), 1);
            let distinct_scalar = f32_tensor(backend, context, Vec::new(), &[0.5])?;
            let first_scalar = LossMap::checked(vec![scalar])?;
            let second_scalar = LossMap::checked(vec![distinct_scalar])?;
            assert_eq!(
                first_scalar.semantic_digest_sha256(),
                second_scalar.semantic_digest_sha256()
            );
            assert_ne!(
                first_scalar.resident_parts()?.tensor_allocations(),
                second_scalar.resident_parts()?.tensor_allocations()
            );

            let overflow = NativeStructuredResidentParts {
                owned_bytes: u64::MAX,
                tensor_allocations: vec![
                    *loss_parts
                        .tensor_allocations()
                        .first()
                        .ok_or(NativeModelPayloadError::StructuredProjectionChanged)?,
                ],
            };
            assert!(matches!(
                overflow.resident_bytes(),
                Err(NativeModelPayloadError::StructuredResidentBytesOverflow)
            ));
            assert!(LossMap::checked(Vec::new()).is_err());
            Ok(())
        })
    }
}
