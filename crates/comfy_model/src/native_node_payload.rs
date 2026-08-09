use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{mem, sync::Arc};
use thiserror::Error;

use crate::{
    NativeVae,
    clip::{LoadedSd1Clip, NativeTokenizer},
    generated_native_diffusion::{Sd1Tokenizer, Sd15TinyModel},
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

    fn resident_owned_bytes(&self) -> Result<u64, NativeModelPayloadError> {
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
        hash_field(&mut hasher, b"sim.comfy.native-model-resource.v1")?;
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
    Sd1Clip {
        tokenizer: Arc<Sd1Tokenizer>,
        clip: Arc<LoadedSd1Clip>,
    },
    NativeVae {
        vae: Arc<NativeVae>,
    },
}

impl NativeModelPayload {
    pub fn sd15_model(model: Arc<Sd15TinyModel>) -> Result<Self, NativeModelPayloadError> {
        let patch_identity = model.patch_identity();
        let identity = NativeModelResourceIdentity::checked(
            NativeModelResourceRole::Model,
            patch_identity.base_artifact_digest.clone(),
            "sim-native-sd15-model-v1",
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
            b"sim.comfy.native-sd1-clip-payload.v1",
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
            "sim-native-sd1-clip-v1",
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

    pub fn identity(&self) -> &NativeModelResourceIdentity {
        &self.identity
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn model(&self) -> Option<&Arc<Sd15TinyModel>> {
        match &self.resource {
            NativeModelResource::Sd15Model { model } => Some(model),
            NativeModelResource::Sd1Clip { .. } | NativeModelResource::NativeVae { .. } => None,
        }
    }

    pub fn clip(&self) -> Option<(&Arc<Sd1Tokenizer>, &Arc<LoadedSd1Clip>)> {
        match &self.resource {
            NativeModelResource::Sd1Clip {
                tokenizer, clip, ..
            } => Some((tokenizer, clip)),
            NativeModelResource::Sd15Model { .. } | NativeModelResource::NativeVae { .. } => None,
        }
    }

    pub fn vae(&self) -> Option<&Arc<NativeVae>> {
        match &self.resource {
            NativeModelResource::NativeVae { vae } => Some(vae),
            NativeModelResource::Sd15Model { .. } | NativeModelResource::Sd1Clip { .. } => None,
        }
    }

    pub fn validate(&self) -> Result<(), NativeModelPayloadError> {
        let expected = match &self.resource {
            NativeModelResource::Sd15Model { model } => Self::sd15_model(model.clone())?,
            NativeModelResource::Sd1Clip {
                tokenizer, clip, ..
            } => Self::sd1_clip(tokenizer.clone(), clip.clone())?,
            NativeModelResource::NativeVae { vae } => Self::native_vae(vae.clone())?,
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

#[derive(Debug, Error)]
pub enum NativeModelPayloadError {
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
    use std::error::Error;

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
}
