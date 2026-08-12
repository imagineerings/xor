use std::{collections::BTreeMap, num::NonZeroU64};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{AttemptId, DeviceKind, PromptId};

pub const WORKER_PROTOCOL_VERSION: u16 = 7;
pub const LEGACY_WORKER_PROTOCOL_VERSION: u16 = 6;
pub const WORKER_REGISTRY_DEPLOYMENT_REJECTION_VERSION: u16 = 1;
pub const MAX_WORKER_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_ENCODED_PREVIEW_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_WORKER_OUTPUT_METADATA_BYTES: usize = 64 * 1024;
pub const MAX_WORKER_OUTPUT_CONTENT_BYTES: usize = 12 * 1024 * 1024;
pub const MAX_WORKER_BACKEND_CAPABILITY_ENTRIES: usize = 4_096;
pub const MAX_WORKER_DEVICE_PROPERTY_BYTES: usize = 256;
pub const MAX_WORKER_COMPONENT_COUNT: usize = 256;
pub const MAX_WORKER_COMPONENT_IDENTITY_BYTES: usize = 256;
pub const MAX_WORKER_COMPONENT_VERSION_BYTES: usize = 128;
pub const MAX_WORKER_COMPONENT_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_WORKER_COMPONENT_AUTHORIZATION_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_WORKER_COMPONENT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_WORKER_COMPONENT_CHUNK_BYTES: usize = 1024 * 1024;
pub const MAX_WORKER_REGISTRY_DEPLOYMENT_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_WORKER_PLUGIN_INVOCATION_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES: usize = 72 * 1024 * 1024;
pub const MAX_WORKER_PLUGIN_RESULT_BYTES: usize = 12 * 1024 * 1024;
pub const MAX_WORKER_PLUGIN_DIAGNOSTIC_CHARS: usize = 4_096;
pub const MAX_WORKER_FATAL_CODE_BYTES: usize = 128;
pub const MAX_WORKER_FATAL_MESSAGE_BYTES: usize = 4_096;
pub const WORKER_OPERATION_SUPPORT_VERSION: u16 = 2;
pub const LEGACY_WORKER_OPERATION_SUPPORT_VERSION: u16 = 1;
pub const WORKER_REGISTRY_DIGEST_DOMAIN: &[u8] = b"sim-comfy-worker-registry-v1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkerRegistryGeneration(NonZeroU64);

impl WorkerRegistryGeneration {
    pub fn new(value: u64) -> Result<Self, WorkerComponentDeploymentError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(WorkerComponentDeploymentError::ZeroGeneration)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct WorkerSha256Digest(String);

impl WorkerSha256Digest {
    pub fn new(value: impl Into<String>) -> Result<Self, WorkerComponentDeploymentError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(WorkerComponentDeploymentError::InvalidSha256);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn bytes(&self) -> [u8; 32] {
        let mut digest = [0_u8; 32];
        for (index, pair) in self.0.as_bytes().chunks_exact(2).enumerate() {
            digest[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
        }
        digest
    }
}

impl<'de> Deserialize<'de> for WorkerSha256Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl TryFrom<String> for WorkerSha256Digest {
    type Error = WorkerComponentDeploymentError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerComponentContent {
    Manifest,
    Authorization,
    Component,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerComponentIdentityField {
    ExtensionId,
    ExtensionVersion,
    PluginIdentifier,
    PluginVersion,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerComponentDescriptor {
    extension_id: String,
    extension_version: String,
    plugin_identifier: String,
    plugin_version: String,
    authorization_generation: WorkerSha256Digest,
    manifest_digest_sha256: WorkerSha256Digest,
    component_digest_sha256: WorkerSha256Digest,
    manifest_bytes: u64,
    authorization_bytes: u64,
    component_bytes: u64,
    manifest_chunk_count: u32,
    authorization_chunk_count: u32,
    component_chunk_count: u32,
}

impl WorkerComponentDescriptor {
    pub fn new(
        extension_id: impl Into<String>,
        extension_version: impl Into<String>,
        plugin_identifier: impl Into<String>,
        plugin_version: impl Into<String>,
        authorization_generation: WorkerSha256Digest,
        manifest_digest_sha256: WorkerSha256Digest,
        component_digest_sha256: WorkerSha256Digest,
        manifest_bytes: u64,
        authorization_bytes: u64,
        component_bytes: u64,
    ) -> Result<Self, WorkerComponentDeploymentError> {
        let extension_id = checked_component_identity(
            extension_id.into(),
            MAX_WORKER_COMPONENT_IDENTITY_BYTES,
            WorkerComponentIdentityField::ExtensionId,
        )?;
        let extension_version = checked_component_identity(
            extension_version.into(),
            MAX_WORKER_COMPONENT_VERSION_BYTES,
            WorkerComponentIdentityField::ExtensionVersion,
        )?;
        let plugin_identifier = checked_component_identity(
            plugin_identifier.into(),
            MAX_WORKER_COMPONENT_IDENTITY_BYTES,
            WorkerComponentIdentityField::PluginIdentifier,
        )?;
        let plugin_version = checked_component_identity(
            plugin_version.into(),
            MAX_WORKER_COMPONENT_VERSION_BYTES,
            WorkerComponentIdentityField::PluginVersion,
        )?;
        let manifest_chunk_count = checked_chunk_count(
            manifest_bytes,
            MAX_WORKER_COMPONENT_MANIFEST_BYTES,
            WorkerComponentContent::Manifest,
        )?;
        let component_chunk_count = checked_chunk_count(
            component_bytes,
            MAX_WORKER_COMPONENT_BYTES,
            WorkerComponentContent::Component,
        )?;
        let authorization_chunk_count = checked_chunk_count(
            authorization_bytes,
            MAX_WORKER_COMPONENT_AUTHORIZATION_BYTES,
            WorkerComponentContent::Authorization,
        )?;
        Ok(Self {
            extension_id,
            extension_version,
            plugin_identifier,
            plugin_version,
            authorization_generation,
            manifest_digest_sha256,
            component_digest_sha256,
            manifest_bytes,
            authorization_bytes,
            component_bytes,
            manifest_chunk_count,
            authorization_chunk_count,
            component_chunk_count,
        })
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub fn extension_version(&self) -> &str {
        &self.extension_version
    }

    pub fn plugin_identifier(&self) -> &str {
        &self.plugin_identifier
    }

    pub fn plugin_version(&self) -> &str {
        &self.plugin_version
    }

    pub fn authorization_generation(&self) -> &WorkerSha256Digest {
        &self.authorization_generation
    }

    pub fn manifest_digest_sha256(&self) -> &WorkerSha256Digest {
        &self.manifest_digest_sha256
    }

    pub fn component_digest_sha256(&self) -> &WorkerSha256Digest {
        &self.component_digest_sha256
    }

    pub const fn manifest_bytes(&self) -> u64 {
        self.manifest_bytes
    }

    pub const fn component_bytes(&self) -> u64 {
        self.component_bytes
    }

    pub const fn authorization_bytes(&self) -> u64 {
        self.authorization_bytes
    }

    pub const fn manifest_chunk_count(&self) -> u32 {
        self.manifest_chunk_count
    }

    pub const fn component_chunk_count(&self) -> u32 {
        self.component_chunk_count
    }

    pub const fn authorization_chunk_count(&self) -> u32 {
        self.authorization_chunk_count
    }

    fn validate(&self) -> Result<(), WorkerComponentDeploymentError> {
        let checked = Self::new(
            self.extension_id.clone(),
            self.extension_version.clone(),
            self.plugin_identifier.clone(),
            self.plugin_version.clone(),
            self.authorization_generation.clone(),
            self.manifest_digest_sha256.clone(),
            self.component_digest_sha256.clone(),
            self.manifest_bytes,
            self.authorization_bytes,
            self.component_bytes,
        )?;
        if checked.manifest_chunk_count != self.manifest_chunk_count
            || checked.authorization_chunk_count != self.authorization_chunk_count
            || checked.component_chunk_count != self.component_chunk_count
        {
            return Err(WorkerComponentDeploymentError::InvalidChunkCount);
        }
        Ok(())
    }
}

fn checked_component_identity(
    value: String,
    maximum_bytes: usize,
    field: WorkerComponentIdentityField,
) -> Result<String, WorkerComponentDeploymentError> {
    if value.is_empty()
        || value.len() > maximum_bytes
        || value.trim() != value
        || value.chars().any(char::is_control)
        || value.contains(['/', '\\'])
        || matches!(value.as_str(), "." | "..")
    {
        return Err(WorkerComponentDeploymentError::InvalidIdentity(field));
    }
    Ok(value)
}

fn checked_chunk_count(
    byte_length: u64,
    maximum: usize,
    content: WorkerComponentContent,
) -> Result<u32, WorkerComponentDeploymentError> {
    if byte_length == 0 {
        return Err(WorkerComponentDeploymentError::EmptyContent(content));
    }
    let maximum =
        u64::try_from(maximum).map_err(|_| WorkerComponentDeploymentError::LengthOverflow)?;
    if byte_length > maximum {
        return Err(WorkerComponentDeploymentError::ContentTooLarge(content));
    }
    let chunk_bytes = u64::try_from(MAX_WORKER_COMPONENT_CHUNK_BYTES)
        .map_err(|_| WorkerComponentDeploymentError::LengthOverflow)?;
    let count = byte_length
        .checked_add(chunk_bytes - 1)
        .ok_or(WorkerComponentDeploymentError::LengthOverflow)?
        / chunk_bytes;
    u32::try_from(count).map_err(|_| WorkerComponentDeploymentError::LengthOverflow)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerRegistryDeploymentBegin {
    generation: WorkerRegistryGeneration,
    registry_digest_sha256: WorkerSha256Digest,
    components: Vec<WorkerComponentDescriptor>,
}

impl WorkerRegistryDeploymentBegin {
    pub fn new(
        generation: WorkerRegistryGeneration,
        registry_digest_sha256: WorkerSha256Digest,
        components: Vec<WorkerComponentDescriptor>,
    ) -> Result<Self, WorkerComponentDeploymentError> {
        let value = Self {
            generation,
            registry_digest_sha256,
            components,
        };
        value.validate()?;
        Ok(value)
    }

    pub const fn generation(&self) -> WorkerRegistryGeneration {
        self.generation
    }

    pub fn registry_digest_sha256(&self) -> &WorkerSha256Digest {
        &self.registry_digest_sha256
    }

    pub fn components(&self) -> &[WorkerComponentDescriptor] {
        &self.components
    }

    pub fn digest_material(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            WORKER_REGISTRY_DIGEST_DOMAIN.len() + 4 + self.components.len() * 256,
        );
        bytes.extend_from_slice(WORKER_REGISTRY_DIGEST_DOMAIN);
        let component_count = u32::try_from(self.components.len()).unwrap_or(u32::MAX);
        bytes.extend_from_slice(&component_count.to_le_bytes());
        for component in &self.components {
            extend_digest_string(&mut bytes, component.extension_id());
            extend_digest_string(&mut bytes, component.extension_version());
            extend_digest_string(&mut bytes, component.plugin_identifier());
            extend_digest_string(&mut bytes, component.plugin_version());
            bytes.extend_from_slice(&component.authorization_generation.bytes());
            bytes.extend_from_slice(&component.manifest_digest_sha256.bytes());
            bytes.extend_from_slice(&component.component_digest_sha256.bytes());
            bytes.extend_from_slice(&component.manifest_bytes.to_le_bytes());
            bytes.extend_from_slice(&component.authorization_bytes.to_le_bytes());
            bytes.extend_from_slice(&component.component_bytes.to_le_bytes());
        }
        bytes
    }

    fn validate(&self) -> Result<(), WorkerComponentDeploymentError> {
        if self.components.len() > MAX_WORKER_COMPONENT_COUNT {
            return Err(WorkerComponentDeploymentError::TooManyComponents);
        }
        let mut previous_key: Option<(&str, &WorkerSha256Digest)> = None;
        let mut total_bytes = 0_u64;
        for component in &self.components {
            component.validate()?;
            let current_key = (
                component.extension_id(),
                component.component_digest_sha256(),
            );
            if let Some(previous_key) = previous_key {
                if previous_key.0 == current_key.0 {
                    return Err(WorkerComponentDeploymentError::DuplicateExtensionIdentity);
                }
                if previous_key >= current_key {
                    return Err(WorkerComponentDeploymentError::NonCanonicalComponentOrder);
                }
            }
            previous_key = Some(current_key);
            total_bytes = total_bytes
                .checked_add(component.manifest_bytes)
                .and_then(|total| total.checked_add(component.authorization_bytes))
                .and_then(|total| total.checked_add(component.component_bytes))
                .ok_or(WorkerComponentDeploymentError::LengthOverflow)?;
        }
        let maximum = u64::try_from(MAX_WORKER_REGISTRY_DEPLOYMENT_BYTES)
            .map_err(|_| WorkerComponentDeploymentError::LengthOverflow)?;
        if total_bytes > maximum {
            return Err(WorkerComponentDeploymentError::DeploymentTooLarge);
        }
        Ok(())
    }
}

fn extend_digest_string(bytes: &mut Vec<u8>, value: &str) {
    let length = u32::try_from(value.len()).unwrap_or(u32::MAX);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerRegistryDeploymentChunk {
    generation: WorkerRegistryGeneration,
    component_index: u32,
    content: WorkerComponentContent,
    chunk_index: u32,
    bytes: Vec<u8>,
}

impl WorkerRegistryDeploymentChunk {
    pub fn new(
        generation: WorkerRegistryGeneration,
        component_index: u32,
        content: WorkerComponentContent,
        chunk_index: u32,
        bytes: Vec<u8>,
    ) -> Result<Self, WorkerComponentDeploymentError> {
        let value = Self {
            generation,
            component_index,
            content,
            chunk_index,
            bytes,
        };
        value.validate()?;
        Ok(value)
    }

    pub const fn generation(&self) -> WorkerRegistryGeneration {
        self.generation
    }

    pub const fn component_index(&self) -> u32 {
        self.component_index
    }

    pub const fn content(&self) -> WorkerComponentContent {
        self.content
    }

    pub const fn chunk_index(&self) -> u32 {
        self.chunk_index
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn validate(&self) -> Result<(), WorkerComponentDeploymentError> {
        if self.bytes.is_empty() {
            return Err(WorkerComponentDeploymentError::EmptyChunk);
        }
        if self.bytes.len() > MAX_WORKER_COMPONENT_CHUNK_BYTES {
            return Err(WorkerComponentDeploymentError::ChunkTooLarge);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerRegistryDeploymentCommit {
    generation: WorkerRegistryGeneration,
    registry_digest_sha256: WorkerSha256Digest,
}

impl WorkerRegistryDeploymentCommit {
    pub fn new(
        generation: WorkerRegistryGeneration,
        registry_digest_sha256: WorkerSha256Digest,
    ) -> Self {
        Self {
            generation,
            registry_digest_sha256,
        }
    }

    pub const fn generation(&self) -> WorkerRegistryGeneration {
        self.generation
    }

    pub fn registry_digest_sha256(&self) -> &WorkerSha256Digest {
        &self.registry_digest_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerRegistryDeploymentAck {
    generation: WorkerRegistryGeneration,
    registry_digest_sha256: WorkerSha256Digest,
    component_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerRegistryDeploymentRejectionReason {
    VerificationUnavailable,
    InvalidCandidate,
    InvalidAuthorization,
    ComponentCompilationFailed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkerRegistryDeploymentRejection {
    version: u16,
    generation: WorkerRegistryGeneration,
    registry_digest_sha256: WorkerSha256Digest,
    reason: WorkerRegistryDeploymentRejectionReason,
}

impl WorkerRegistryDeploymentRejection {
    pub fn new(
        generation: WorkerRegistryGeneration,
        registry_digest_sha256: WorkerSha256Digest,
        reason: WorkerRegistryDeploymentRejectionReason,
    ) -> Self {
        Self {
            version: WORKER_REGISTRY_DEPLOYMENT_REJECTION_VERSION,
            generation,
            registry_digest_sha256,
            reason,
        }
    }

    pub const fn version(&self) -> u16 {
        self.version
    }

    pub const fn generation(&self) -> WorkerRegistryGeneration {
        self.generation
    }

    pub fn registry_digest_sha256(&self) -> &WorkerSha256Digest {
        &self.registry_digest_sha256
    }

    pub const fn reason(&self) -> WorkerRegistryDeploymentRejectionReason {
        self.reason
    }

    fn validate(&self) -> Result<(), WorkerComponentDeploymentError> {
        if self.version != WORKER_REGISTRY_DEPLOYMENT_REJECTION_VERSION {
            return Err(
                WorkerComponentDeploymentError::UnsupportedDeploymentRejectionVersion {
                    expected: WORKER_REGISTRY_DEPLOYMENT_REJECTION_VERSION,
                    actual: self.version,
                },
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerPluginExecutionFailure {
    Cancelled,
    TimedOut,
    Trap { diagnostic: String },
    InvalidInvocation,
    CapabilityDenied,
    HostFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerPluginExecutionOutcome {
    Succeeded(Vec<u8>),
    Failed(WorkerPluginExecutionFailure),
}

impl WorkerPluginExecutionOutcome {
    pub fn result_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Succeeded(bytes) => Some(bytes),
            Self::Failed(_) => None,
        }
    }
}

impl WorkerRegistryDeploymentAck {
    pub fn new(
        generation: WorkerRegistryGeneration,
        registry_digest_sha256: WorkerSha256Digest,
        component_count: u32,
    ) -> Result<Self, WorkerComponentDeploymentError> {
        if usize::try_from(component_count)
            .map_err(|_| WorkerComponentDeploymentError::TooManyComponents)?
            > MAX_WORKER_COMPONENT_COUNT
        {
            return Err(WorkerComponentDeploymentError::TooManyComponents);
        }
        Ok(Self {
            generation,
            registry_digest_sha256,
            component_count,
        })
    }

    pub const fn generation(&self) -> WorkerRegistryGeneration {
        self.generation
    }

    pub fn registry_digest_sha256(&self) -> &WorkerSha256Digest {
        &self.registry_digest_sha256
    }

    pub const fn component_count(&self) -> u32 {
        self.component_count
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum WorkerComponentDeploymentError {
    #[error("worker registry generation must be nonzero")]
    ZeroGeneration,
    #[error(
        "worker registry deployment rejection version {actual} is unsupported; expected {expected}"
    )]
    UnsupportedDeploymentRejectionVersion { expected: u16, actual: u16 },
    #[error("worker content address must be exactly 64 lowercase hexadecimal characters")]
    InvalidSha256,
    #[error("worker component {0:?} is empty, oversized, malformed, or path-bearing")]
    InvalidIdentity(WorkerComponentIdentityField),
    #[error("worker {0:?} content is empty")]
    EmptyContent(WorkerComponentContent),
    #[error("worker {0:?} content exceeds its bound")]
    ContentTooLarge(WorkerComponentContent),
    #[error("worker component chunk is empty")]
    EmptyChunk,
    #[error("worker component chunk exceeds {MAX_WORKER_COMPONENT_CHUNK_BYTES} bytes")]
    ChunkTooLarge,
    #[error("worker component chunk count does not match its declared byte length")]
    InvalidChunkCount,
    #[error("worker component count exceeds {MAX_WORKER_COMPONENT_COUNT}")]
    TooManyComponents,
    #[error("worker component descriptors are not in canonical identity and digest order")]
    NonCanonicalComponentOrder,
    #[error("worker component deployment repeats a lifecycle extension identifier")]
    DuplicateExtensionIdentity,
    #[error("worker registry deployment exceeds {MAX_WORKER_REGISTRY_DEPLOYMENT_BYTES} bytes")]
    DeploymentTooLarge,
    #[error("worker component deployment length overflowed")]
    LengthOverflow,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProfileId(pub Uuid);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkerId(pub Uuid);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(pub Uuid);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerOperationCategory {
    Allocation,
    Copy,
    Event,
    Scalar,
    Unary,
    Binary,
    Reduction,
    Indexing,
    Resize,
    Convolution,
    LinearAlgebra,
    CustomKernel,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerDType {
    F64,
    F32,
    F16,
    Bf16,
    I64,
    I32,
    I16,
    I8,
    U64,
    U32,
    U16,
    U8,
    Bool,
    Complex64,
    Complex128,
    Float8E4m3Fn,
    Float8E5m2,
    #[serde(rename = "float8_e4m3fnuz", alias = "float8_e4m3_fnuz")]
    Float8E4m3Fnuz,
    #[serde(rename = "float8_e5m2fnuz", alias = "float8_e5m2_fnuz")]
    Float8E5m2Fnuz,
    #[serde(rename = "float8_e8m0fnu", alias = "float8_e8m0_fnu")]
    Float8E8m0Fnu,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerLayout {
    Contiguous,
    ChannelsLast,
    ChannelsLast3d,
    Strided,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerUnaryOperationV1 {
    Absolute,
    Negate,
    Exponential,
    NaturalLogarithm,
    SquareRoot,
    Reciprocal,
    Sine,
    Cosine,
    HyperbolicTangent,
    Sigmoid,
    Round,
    Sinc,
    Log1p,
    ReciprocalSquareRoot,
    Relu,
    IsFinite,
    InvertUnitInterval,
    LogarithmBaseTwo,
    Signum,
    Tangent,
    ArcTangent,
    ArcHyperbolicTangent,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerBinaryOperationV1 {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Power,
    Minimum,
    Maximum,
    Equal,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LogicalAnd,
    LogicalOr,
    FloatingRemainder,
    Atan2,
    LogAddExp,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerReductionOperationV1 {
    Sum,
    Product,
    Mean,
    Minimum,
    Maximum,
    ArgMinimum,
    ArgMaximum,
    All,
    Any,
    Variance,
    StandardDeviation,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerResizeModeV1 {
    NearestExact,
    Bilinear,
    Area,
    Bicubic,
    Lanczos,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerLinearAlgebraOperationV1 {
    MatrixMultiply,
    BatchMatrixMultiply,
    MatrixVectorMultiply,
    Dot,
    Outer,
    Solve,
    SingularValueDecomposition,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerTensorRoleV1 {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerPrimitiveOperationV1 {
    Allocation,
    Copy,
    Fill,
    Unary(WorkerUnaryOperationV1),
    Binary(WorkerBinaryOperationV1),
    BinaryScalar(WorkerBinaryOperationV1),
    Reduction(WorkerReductionOperationV1),
    Select,
    Narrow,
    Resize(WorkerResizeModeV1),
    RecordEvent,
    WaitEvent,
}

impl WorkerPrimitiveOperationV1 {
    pub const fn category(self) -> WorkerOperationCategory {
        match self {
            Self::Allocation => WorkerOperationCategory::Allocation,
            Self::Copy => WorkerOperationCategory::Copy,
            Self::Fill => WorkerOperationCategory::Scalar,
            Self::Unary(_) => WorkerOperationCategory::Unary,
            Self::Binary(_) | Self::BinaryScalar(_) => WorkerOperationCategory::Binary,
            Self::Reduction(_) => WorkerOperationCategory::Reduction,
            Self::Select | Self::Narrow => WorkerOperationCategory::Indexing,
            Self::Resize(_) => WorkerOperationCategory::Resize,
            Self::RecordEvent | Self::WaitEvent => WorkerOperationCategory::Event,
        }
    }

    pub const fn is_event(self) -> bool {
        matches!(self, Self::RecordEvent | Self::WaitEvent)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerPrimitiveOperationV2 {
    Allocation,
    Copy,
    Fill,
    Unary(WorkerUnaryOperationV1),
    Binary(WorkerBinaryOperationV1),
    BinaryScalar(WorkerBinaryOperationV1),
    Reduction(WorkerReductionOperationV1),
    Select,
    Narrow,
    Resize(WorkerResizeModeV1),
    RecordEvent,
    WaitEvent,
    LinearAlgebra(WorkerLinearAlgebraOperationV1),
    Gather,
    Scatter,
    MaskedSelect,
    Convolution,
    CustomKernel,
}

impl WorkerPrimitiveOperationV2 {
    pub const fn category(self) -> WorkerOperationCategory {
        match self {
            Self::Allocation => WorkerOperationCategory::Allocation,
            Self::Copy => WorkerOperationCategory::Copy,
            Self::Fill => WorkerOperationCategory::Scalar,
            Self::Unary(_) => WorkerOperationCategory::Unary,
            Self::Binary(_) | Self::BinaryScalar(_) => WorkerOperationCategory::Binary,
            Self::Reduction(_) => WorkerOperationCategory::Reduction,
            Self::Select | Self::Narrow | Self::Gather | Self::Scatter | Self::MaskedSelect => {
                WorkerOperationCategory::Indexing
            }
            Self::Resize(_) => WorkerOperationCategory::Resize,
            Self::LinearAlgebra(_) => WorkerOperationCategory::LinearAlgebra,
            Self::Convolution => WorkerOperationCategory::Convolution,
            Self::CustomKernel => WorkerOperationCategory::CustomKernel,
            Self::RecordEvent | Self::WaitEvent => WorkerOperationCategory::Event,
        }
    }

    pub const fn is_event(self) -> bool {
        matches!(self, Self::RecordEvent | Self::WaitEvent)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(into = "WorkerOperationSupportV1Wire")]
pub struct WorkerOperationSupportV1 {
    operation: WorkerPrimitiveOperationV1,
    role: Option<WorkerTensorRoleV1>,
    dtype: Option<WorkerDType>,
    layout: Option<WorkerLayout>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkerOperationSupportV1Wire {
    version: u16,
    operation: WorkerPrimitiveOperationV1,
    role: Option<WorkerTensorRoleV1>,
    dtype: Option<WorkerDType>,
    layout: Option<WorkerLayout>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(into = "WorkerOperationSupportWire")]
pub struct WorkerOperationSupport {
    operation: WorkerPrimitiveOperationV2,
    role: Option<WorkerTensorRoleV1>,
    dtype: Option<WorkerDType>,
    layout: Option<WorkerLayout>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkerOperationSupportWire {
    version: u16,
    operation: WorkerPrimitiveOperationV2,
    role: Option<WorkerTensorRoleV1>,
    dtype: Option<WorkerDType>,
    layout: Option<WorkerLayout>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum WorkerOperationSupportError {
    #[error("unsupported worker operation-support version {0}")]
    UnsupportedVersion(u16),
    #[error("tensor primitive capability requires role, dtype, and layout")]
    MissingTensorSignature,
    #[error("event primitive capability cannot declare a role, dtype, or layout")]
    EventTensorSignature,
}

impl WorkerOperationSupportV1 {
    pub const fn for_tensor(
        operation: WorkerPrimitiveOperationV1,
        role: WorkerTensorRoleV1,
        dtype: WorkerDType,
        layout: WorkerLayout,
    ) -> Result<Self, WorkerOperationSupportError> {
        if operation.is_event() {
            Err(WorkerOperationSupportError::EventTensorSignature)
        } else {
            Ok(Self {
                operation,
                role: Some(role),
                dtype: Some(dtype),
                layout: Some(layout),
            })
        }
    }

    pub const fn for_event(
        operation: WorkerPrimitiveOperationV1,
    ) -> Result<Self, WorkerOperationSupportError> {
        if operation.is_event() {
            Ok(Self {
                operation,
                role: None,
                dtype: None,
                layout: None,
            })
        } else {
            Err(WorkerOperationSupportError::MissingTensorSignature)
        }
    }

    pub const fn version(&self) -> u16 {
        LEGACY_WORKER_OPERATION_SUPPORT_VERSION
    }

    pub const fn operation(&self) -> WorkerPrimitiveOperationV1 {
        self.operation
    }
}

impl From<WorkerOperationSupportV1> for WorkerOperationSupportV1Wire {
    fn from(value: WorkerOperationSupportV1) -> Self {
        Self {
            version: LEGACY_WORKER_OPERATION_SUPPORT_VERSION,
            operation: value.operation,
            role: value.role,
            dtype: value.dtype,
            layout: value.layout,
        }
    }
}

impl TryFrom<WorkerOperationSupportV1Wire> for WorkerOperationSupportV1 {
    type Error = WorkerOperationSupportError;

    fn try_from(value: WorkerOperationSupportV1Wire) -> Result<Self, Self::Error> {
        if value.version != LEGACY_WORKER_OPERATION_SUPPORT_VERSION {
            return Err(WorkerOperationSupportError::UnsupportedVersion(
                value.version,
            ));
        }
        match (
            value.operation.is_event(),
            value.role,
            value.dtype,
            value.layout,
        ) {
            (true, None, None, None) => Self::for_event(value.operation),
            (true, _, _, _) => Err(WorkerOperationSupportError::EventTensorSignature),
            (false, Some(role), Some(dtype), Some(layout)) => {
                Self::for_tensor(value.operation, role, dtype, layout)
            }
            (false, _, _, _) => Err(WorkerOperationSupportError::MissingTensorSignature),
        }
    }
}

impl<'de> Deserialize<'de> for WorkerOperationSupportV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        WorkerOperationSupportV1Wire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl WorkerOperationSupport {
    pub const fn for_tensor_v2(
        operation: WorkerPrimitiveOperationV2,
        role: WorkerTensorRoleV1,
        dtype: WorkerDType,
        layout: WorkerLayout,
    ) -> Result<Self, WorkerOperationSupportError> {
        if operation.is_event() {
            Err(WorkerOperationSupportError::EventTensorSignature)
        } else {
            Ok(Self {
                operation,
                role: Some(role),
                dtype: Some(dtype),
                layout: Some(layout),
            })
        }
    }

    pub const fn for_event_v2(
        operation: WorkerPrimitiveOperationV2,
    ) -> Result<Self, WorkerOperationSupportError> {
        if operation.is_event() {
            Ok(Self {
                operation,
                role: None,
                dtype: None,
                layout: None,
            })
        } else {
            Err(WorkerOperationSupportError::MissingTensorSignature)
        }
    }

    pub const fn version(&self) -> u16 {
        WORKER_OPERATION_SUPPORT_VERSION
    }

    pub const fn operation(&self) -> WorkerPrimitiveOperationV2 {
        self.operation
    }

    pub const fn category(&self) -> WorkerOperationCategory {
        self.operation.category()
    }

    pub const fn role(&self) -> Option<WorkerTensorRoleV1> {
        self.role
    }

    pub const fn dtype(&self) -> Option<WorkerDType> {
        self.dtype
    }

    pub const fn layout(&self) -> Option<WorkerLayout> {
        self.layout
    }
}

impl From<WorkerOperationSupport> for WorkerOperationSupportWire {
    fn from(value: WorkerOperationSupport) -> Self {
        Self {
            version: WORKER_OPERATION_SUPPORT_VERSION,
            operation: value.operation,
            role: value.role,
            dtype: value.dtype,
            layout: value.layout,
        }
    }
}

impl TryFrom<WorkerOperationSupportWire> for WorkerOperationSupport {
    type Error = WorkerOperationSupportError;

    fn try_from(value: WorkerOperationSupportWire) -> Result<Self, Self::Error> {
        if value.version != WORKER_OPERATION_SUPPORT_VERSION {
            return Err(WorkerOperationSupportError::UnsupportedVersion(
                value.version,
            ));
        }
        match (
            value.operation.is_event(),
            value.role,
            value.dtype,
            value.layout,
        ) {
            (true, None, None, None) => Self::for_event_v2(value.operation),
            (true, _, _, _) => Err(WorkerOperationSupportError::EventTensorSignature),
            (false, Some(role), Some(dtype), Some(layout)) => {
                Self::for_tensor_v2(value.operation, role, dtype, layout)
            }
            (false, _, _, _) => Err(WorkerOperationSupportError::MissingTensorSignature),
        }
    }
}

impl<'de> Deserialize<'de> for WorkerOperationSupport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        WorkerOperationSupportWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "WorkerBackendCapabilitiesWire",
    into = "WorkerBackendCapabilitiesWire"
)]
pub struct WorkerBackendCapabilities {
    device: DeviceKind,
    ordinal: u32,
    supported: Vec<WorkerOperationSupport>,
    deterministic: Vec<WorkerOperationSupport>,
    properties: Option<WorkerNativeDeviceProperties>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WorkerBackendCapabilitiesWire {
    device: DeviceKind,
    ordinal: u32,
    supported: Vec<WorkerOperationSupport>,
    deterministic: Vec<WorkerOperationSupport>,
    properties: Option<WorkerNativeDeviceProperties>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "WorkerNativeDevicePropertiesWire",
    into = "WorkerNativeDevicePropertiesWire"
)]
pub struct WorkerNativeDeviceProperties {
    name: String,
    total_memory_bytes: u64,
    allocation_limit_bytes: u64,
    major: u32,
    minor: u32,
    architecture: Option<String>,
    has_fp16: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WorkerNativeDevicePropertiesWire {
    name: String,
    total_memory_bytes: u64,
    major: u32,
    minor: u32,
    architecture: Option<String>,
    has_fp16: bool,
    allocation_limit_bytes: u64,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum WorkerNativeDevicePropertiesError {
    #[error("worker native device name exceeds {MAX_WORKER_DEVICE_PROPERTY_BYTES} bytes")]
    NameOversized,
    #[error("worker native device architecture exceeds {MAX_WORKER_DEVICE_PROPERTY_BYTES} bytes")]
    ArchitectureOversized,
    #[error("worker native device memory and allocation limits are invalid")]
    InvalidMemoryLimit,
}

impl WorkerNativeDeviceProperties {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        total_memory_bytes: u64,
        major: u32,
        minor: u32,
        architecture: Option<String>,
        has_fp16: bool,
    ) -> Result<Self, WorkerNativeDevicePropertiesError> {
        Self::new_with_allocation_limit(
            name,
            total_memory_bytes,
            total_memory_bytes,
            major,
            minor,
            architecture,
            has_fp16,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_allocation_limit(
        name: impl Into<String>,
        total_memory_bytes: u64,
        allocation_limit_bytes: u64,
        major: u32,
        minor: u32,
        architecture: Option<String>,
        has_fp16: bool,
    ) -> Result<Self, WorkerNativeDevicePropertiesError> {
        let name = name.into();
        if name.len() > MAX_WORKER_DEVICE_PROPERTY_BYTES {
            return Err(WorkerNativeDevicePropertiesError::NameOversized);
        }
        if architecture
            .as_ref()
            .is_some_and(|value| value.len() > MAX_WORKER_DEVICE_PROPERTY_BYTES)
        {
            return Err(WorkerNativeDevicePropertiesError::ArchitectureOversized);
        }
        if total_memory_bytes == 0
            || allocation_limit_bytes == 0
            || allocation_limit_bytes > total_memory_bytes
        {
            return Err(WorkerNativeDevicePropertiesError::InvalidMemoryLimit);
        }
        Ok(Self {
            name,
            total_memory_bytes,
            allocation_limit_bytes,
            major,
            minor,
            architecture,
            has_fp16,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn total_memory_bytes(&self) -> u64 {
        self.total_memory_bytes
    }

    pub const fn allocation_limit_bytes(&self) -> u64 {
        self.allocation_limit_bytes
    }

    pub const fn major(&self) -> u32 {
        self.major
    }

    pub const fn minor(&self) -> u32 {
        self.minor
    }

    pub fn architecture(&self) -> Option<&str> {
        self.architecture.as_deref()
    }

    pub const fn has_fp16(&self) -> bool {
        self.has_fp16
    }
}

impl From<WorkerNativeDeviceProperties> for WorkerNativeDevicePropertiesWire {
    fn from(value: WorkerNativeDeviceProperties) -> Self {
        Self {
            name: value.name,
            total_memory_bytes: value.total_memory_bytes,
            allocation_limit_bytes: value.allocation_limit_bytes,
            major: value.major,
            minor: value.minor,
            architecture: value.architecture,
            has_fp16: value.has_fp16,
        }
    }
}

impl TryFrom<WorkerNativeDevicePropertiesWire> for WorkerNativeDeviceProperties {
    type Error = WorkerNativeDevicePropertiesError;

    fn try_from(value: WorkerNativeDevicePropertiesWire) -> Result<Self, Self::Error> {
        Self::new_with_allocation_limit(
            value.name,
            value.total_memory_bytes,
            value.allocation_limit_bytes,
            value.major,
            value.minor,
            value.architecture,
            value.has_fp16,
        )
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum WorkerBackendCapabilityError {
    #[error("worker backend capability declaration is empty")]
    Empty,
    #[error(
        "worker backend capability declaration exceeds {MAX_WORKER_BACKEND_CAPABILITY_ENTRIES} entries"
    )]
    Oversized,
    #[error("worker backend capability declaration contains a duplicate primitive support")]
    Duplicate,
}

impl WorkerBackendCapabilities {
    pub fn new(
        device: DeviceKind,
        ordinal: u32,
        supported: Vec<WorkerOperationSupport>,
        deterministic: Vec<WorkerOperationSupport>,
    ) -> Result<Self, WorkerBackendCapabilityError> {
        Self::new_with_properties(device, ordinal, supported, deterministic, None)
    }

    pub fn new_with_properties(
        device: DeviceKind,
        ordinal: u32,
        mut supported: Vec<WorkerOperationSupport>,
        mut deterministic: Vec<WorkerOperationSupport>,
        properties: Option<WorkerNativeDeviceProperties>,
    ) -> Result<Self, WorkerBackendCapabilityError> {
        if supported.is_empty() {
            return Err(WorkerBackendCapabilityError::Empty);
        }
        if supported.len() > MAX_WORKER_BACKEND_CAPABILITY_ENTRIES
            || deterministic.len() > MAX_WORKER_BACKEND_CAPABILITY_ENTRIES
        {
            return Err(WorkerBackendCapabilityError::Oversized);
        }
        canonicalize_worker_support(&mut supported)?;
        canonicalize_worker_support(&mut deterministic)?;
        Ok(Self {
            device,
            ordinal,
            supported,
            deterministic,
            properties,
        })
    }

    pub const fn device(&self) -> DeviceKind {
        self.device
    }

    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }

    pub fn supported(&self) -> &[WorkerOperationSupport] {
        &self.supported
    }

    pub fn deterministic(&self) -> &[WorkerOperationSupport] {
        &self.deterministic
    }

    pub fn properties(&self) -> Option<&WorkerNativeDeviceProperties> {
        self.properties.as_ref()
    }
}

fn canonicalize_worker_support(
    support: &mut [WorkerOperationSupport],
) -> Result<(), WorkerBackendCapabilityError> {
    support.sort_unstable();
    if support.windows(2).any(|entries| entries[0] == entries[1]) {
        Err(WorkerBackendCapabilityError::Duplicate)
    } else {
        Ok(())
    }
}

impl From<WorkerBackendCapabilities> for WorkerBackendCapabilitiesWire {
    fn from(value: WorkerBackendCapabilities) -> Self {
        Self {
            device: value.device,
            ordinal: value.ordinal,
            supported: value.supported,
            deterministic: value.deterministic,
            properties: value.properties,
        }
    }
}

impl TryFrom<WorkerBackendCapabilitiesWire> for WorkerBackendCapabilities {
    type Error = WorkerBackendCapabilityError;

    fn try_from(value: WorkerBackendCapabilitiesWire) -> Result<Self, Self::Error> {
        Self::new_with_properties(
            value.device,
            value.ordinal,
            value.supported,
            value.deterministic,
            value.properties,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerEnvelope {
    pub version: u16,
    pub profile_id: ProfileId,
    pub worker_id: WorkerId,
    pub request_id: RequestId,
    pub prompt_id: Option<PromptId>,
    pub attempt_id: Option<AttemptId>,
    pub sequence: u64,
    pub registry_version: String,
    pub message: WorkerMessage,
    pub extensions: BTreeMap<String, Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerLifecycleEvent {
    ExecutionStarted,
    CancellationRequested { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "WorkerOutputProposalWire")]
pub struct WorkerOutputProposal {
    proposal_id: Uuid,
    metadata: Vec<u8>,
    content: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkerOutputProposalWire {
    proposal_id: Uuid,
    metadata: Vec<u8>,
    content: Vec<u8>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum WorkerOutputProposalError {
    #[error("worker output metadata exceeds {MAX_WORKER_OUTPUT_METADATA_BYTES} bytes")]
    MetadataTooLarge,
    #[error("worker output content exceeds {MAX_WORKER_OUTPUT_CONTENT_BYTES} bytes")]
    ContentTooLarge,
}

impl WorkerOutputProposal {
    pub fn new(
        proposal_id: Uuid,
        metadata: Vec<u8>,
        content: Vec<u8>,
    ) -> Result<Self, WorkerOutputProposalError> {
        if metadata.len() > MAX_WORKER_OUTPUT_METADATA_BYTES {
            return Err(WorkerOutputProposalError::MetadataTooLarge);
        }
        if content.len() > MAX_WORKER_OUTPUT_CONTENT_BYTES {
            return Err(WorkerOutputProposalError::ContentTooLarge);
        }
        Ok(Self {
            proposal_id,
            metadata,
            content,
        })
    }

    pub const fn proposal_id(&self) -> Uuid {
        self.proposal_id
    }

    pub fn metadata(&self) -> &[u8] {
        &self.metadata
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }

    pub fn into_parts(self) -> (Uuid, Vec<u8>, Vec<u8>) {
        (self.proposal_id, self.metadata, self.content)
    }
}

impl From<WorkerOutputProposal> for WorkerOutputProposalWire {
    fn from(value: WorkerOutputProposal) -> Self {
        Self {
            proposal_id: value.proposal_id,
            metadata: value.metadata,
            content: value.content,
        }
    }
}

impl TryFrom<WorkerOutputProposalWire> for WorkerOutputProposal {
    type Error = WorkerOutputProposalError;

    fn try_from(value: WorkerOutputProposalWire) -> Result<Self, Self::Error> {
        Self::new(value.proposal_id, value.metadata, value.content)
    }
}

impl<'de> Deserialize<'de> for WorkerOutputProposal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        WorkerOutputProposalWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkerMessage {
    Hello {
        backend: WorkerBackendCapabilities,
    },
    HelloAck {
        accepted_backend: WorkerBackendCapabilities,
    },
    Ready,
    Execute {
        plan: Vec<u8>,
    },
    Cancel {
        reason: String,
    },
    Event {
        event: Vec<u8>,
    },
    OutputProposal {
        proposal: WorkerOutputProposal,
    },
    Heartbeat,
    Shutdown,
    Fatal {
        code: String,
        message: String,
    },
    Lifecycle {
        event: WorkerLifecycleEvent,
    },
    RegistryDeploymentBegin {
        deployment: WorkerRegistryDeploymentBegin,
    },
    RegistryDeploymentChunk {
        chunk: WorkerRegistryDeploymentChunk,
    },
    RegistryDeploymentCommit {
        commit: WorkerRegistryDeploymentCommit,
    },
    RegistryDeploymentAck {
        acknowledgement: WorkerRegistryDeploymentAck,
    },
    RegistryDeploymentRejected {
        rejection: WorkerRegistryDeploymentRejection,
    },
    ExecutePlugin {
        invocation: Vec<u8>,
    },
    PluginCapabilityRequest {
        call_id: u64,
        request: Vec<u8>,
    },
    PluginCapabilityResponse {
        call_id: u64,
        response: Vec<u8>,
    },
    PluginResult {
        outcome: WorkerPluginExecutionOutcome,
    },
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum WorkerProtocolError {
    #[error("worker frame exceeds {MAX_WORKER_FRAME_BYTES} bytes")]
    Oversized,
    #[error("encoded worker event exceeds {MAX_ENCODED_PREVIEW_BYTES} bytes")]
    OversizedEvent,
    #[error("worker frame length prefix does not match its payload")]
    LengthMismatch,
    #[error("unsupported worker protocol version {0}")]
    UnsupportedVersion(u16),
    #[error("invalid worker payload: {0}")]
    InvalidPayload(String),
    #[error("worker envelope contains unsupported opaque extensions")]
    OpaqueExtensions,
    #[error("worker component deployment is invalid: {0}")]
    InvalidComponentDeployment(String),
    #[error("worker plugin invocation exceeds {MAX_WORKER_PLUGIN_INVOCATION_BYTES} bytes")]
    OversizedPluginInvocation,
    #[error(
        "worker plugin capability payload exceeds {MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES} bytes"
    )]
    OversizedPluginCapabilityPayload,
    #[error("worker plugin result exceeds {MAX_WORKER_PLUGIN_RESULT_BYTES} bytes")]
    OversizedPluginResult,
    #[error("worker plugin capability call identifier must be nonzero")]
    InvalidPluginCapabilityCallId,
}

pub fn encode_worker_frame(message: &WorkerEnvelope) -> Result<Vec<u8>, WorkerProtocolError> {
    if message.version != WORKER_PROTOCOL_VERSION {
        return Err(WorkerProtocolError::UnsupportedVersion(message.version));
    }
    validate_envelope_bounds(message)?;
    let payload = postcard::to_stdvec(message)
        .map_err(|error| WorkerProtocolError::InvalidPayload(error.to_string()))?;
    if payload.len() > MAX_WORKER_FRAME_BYTES {
        return Err(WorkerProtocolError::Oversized);
    }
    let length = u32::try_from(payload.len()).map_err(|_| WorkerProtocolError::Oversized)?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_worker_frame(frame: &[u8]) -> Result<WorkerEnvelope, WorkerProtocolError> {
    let length_bytes: [u8; 4] = frame
        .get(..4)
        .ok_or(WorkerProtocolError::LengthMismatch)?
        .try_into()
        .map_err(|_| WorkerProtocolError::LengthMismatch)?;
    let declared = u32::from_le_bytes(length_bytes) as usize;
    let payload = frame.get(4..).ok_or(WorkerProtocolError::LengthMismatch)?;
    if declared != payload.len() {
        return Err(WorkerProtocolError::LengthMismatch);
    }
    if declared > MAX_WORKER_FRAME_BYTES {
        return Err(WorkerProtocolError::Oversized);
    }
    let (version, _) = postcard::take_from_bytes::<u16>(payload)
        .map_err(|error| WorkerProtocolError::InvalidPayload(error.to_string()))?;
    if version != WORKER_PROTOCOL_VERSION {
        return Err(WorkerProtocolError::UnsupportedVersion(version));
    }
    let message: WorkerEnvelope = postcard::from_bytes(payload)
        .map_err(|error| WorkerProtocolError::InvalidPayload(error.to_string()))?;
    validate_envelope_bounds(&message)?;
    Ok(message)
}

fn validate_envelope_bounds(message: &WorkerEnvelope) -> Result<(), WorkerProtocolError> {
    if !message.extensions.is_empty() {
        return Err(WorkerProtocolError::OpaqueExtensions);
    }
    validate_message_bounds(&message.message)
}

fn validate_message_bounds(message: &WorkerMessage) -> Result<(), WorkerProtocolError> {
    match message {
        WorkerMessage::Fatal { code, message }
            if code.is_empty()
                || code.len() > MAX_WORKER_FATAL_CODE_BYTES
                || !code.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'-' | b'.')
                })
                || message.is_empty()
                || message.len() > MAX_WORKER_FATAL_MESSAGE_BYTES
                || message
                    .chars()
                    .any(|character| character.is_control() && character != '\t') =>
        {
            Err(WorkerProtocolError::InvalidPayload(
                "worker fatal diagnostic is invalid".to_owned(),
            ))
        }
        WorkerMessage::Event { event } if event.len() > MAX_ENCODED_PREVIEW_BYTES => {
            Err(WorkerProtocolError::OversizedEvent)
        }
        WorkerMessage::RegistryDeploymentBegin { deployment } => deployment
            .validate()
            .map_err(|error| WorkerProtocolError::InvalidComponentDeployment(error.to_string())),
        WorkerMessage::RegistryDeploymentChunk { chunk } => chunk
            .validate()
            .map_err(|error| WorkerProtocolError::InvalidComponentDeployment(error.to_string())),
        WorkerMessage::RegistryDeploymentAck { acknowledgement }
            if usize::try_from(acknowledgement.component_count)
                .map_or(true, |count| count > MAX_WORKER_COMPONENT_COUNT) =>
        {
            Err(WorkerProtocolError::InvalidComponentDeployment(
                WorkerComponentDeploymentError::TooManyComponents.to_string(),
            ))
        }
        WorkerMessage::RegistryDeploymentRejected { rejection } => rejection
            .validate()
            .map_err(|error| WorkerProtocolError::InvalidComponentDeployment(error.to_string())),
        WorkerMessage::ExecutePlugin { invocation }
            if invocation.is_empty() || invocation.len() > MAX_WORKER_PLUGIN_INVOCATION_BYTES =>
        {
            Err(WorkerProtocolError::OversizedPluginInvocation)
        }
        WorkerMessage::PluginCapabilityRequest { call_id, request }
        | WorkerMessage::PluginCapabilityResponse {
            call_id,
            response: request,
        } if *call_id == 0 => Err(WorkerProtocolError::InvalidPluginCapabilityCallId),
        WorkerMessage::PluginCapabilityRequest { request, .. }
        | WorkerMessage::PluginCapabilityResponse {
            response: request, ..
        } if request.is_empty() || request.len() > MAX_WORKER_PLUGIN_CAPABILITY_PAYLOAD_BYTES => {
            Err(WorkerProtocolError::OversizedPluginCapabilityPayload)
        }
        WorkerMessage::PluginResult {
            outcome: WorkerPluginExecutionOutcome::Succeeded(result),
        } if result.is_empty() || result.len() > MAX_WORKER_PLUGIN_RESULT_BYTES => {
            Err(WorkerProtocolError::OversizedPluginResult)
        }
        WorkerMessage::PluginResult {
            outcome:
                WorkerPluginExecutionOutcome::Failed(WorkerPluginExecutionFailure::Trap { diagnostic }),
        } if diagnostic.is_empty()
            || diagnostic.chars().count() > MAX_WORKER_PLUGIN_DIAGNOSTIC_CHARS
            || diagnostic
                .chars()
                .any(|character| character.is_control() && !matches!(character, '\n' | '\t')) =>
        {
            Err(WorkerProtocolError::InvalidPayload(
                "worker plugin trap diagnostic is invalid".to_owned(),
            ))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(message: WorkerMessage) -> WorkerEnvelope {
        WorkerEnvelope {
            version: WORKER_PROTOCOL_VERSION,
            profile_id: ProfileId(Uuid::nil()),
            worker_id: WorkerId(Uuid::nil()),
            request_id: RequestId(Uuid::nil()),
            prompt_id: None,
            attempt_id: None,
            sequence: 7,
            registry_version: "fixture-v1".into(),
            message,
            extensions: BTreeMap::new(),
        }
    }

    fn cpu_backend() -> WorkerBackendCapabilities {
        let allocation = WorkerOperationSupport::for_tensor_v2(
            WorkerPrimitiveOperationV2::Allocation,
            WorkerTensorRoleV1::Output,
            WorkerDType::F32,
            WorkerLayout::Contiguous,
        )
        .expect("allocation is a tensor primitive");
        WorkerBackendCapabilities::new(DeviceKind::Cpu, 0, vec![allocation], vec![])
            .expect("valid CPU fixture")
    }

    fn digest(byte: char) -> WorkerSha256Digest {
        WorkerSha256Digest::new(std::iter::repeat_n(byte, 64).collect::<String>())
            .expect("valid digest")
    }

    fn component_descriptor(
        extension_id: &str,
        component_digest: char,
        manifest_bytes: u64,
        component_bytes: u64,
    ) -> Result<WorkerComponentDescriptor, WorkerComponentDeploymentError> {
        WorkerComponentDescriptor::new(
            extension_id,
            "1.2.3",
            "plugin.identifier",
            "4.5.6",
            digest('d'),
            digest('a'),
            digest(component_digest),
            manifest_bytes,
            1,
            component_bytes,
        )
    }

    fn deployment_begin() -> WorkerRegistryDeploymentBegin {
        WorkerRegistryDeploymentBegin::new(
            WorkerRegistryGeneration::new(1).expect("nonzero generation"),
            digest('c'),
            vec![component_descriptor("extension-a", 'b', 2, 3).expect("bounded descriptor")],
        )
        .expect("bounded deployment")
    }

    #[test]
    fn every_worker_message_variant_round_trips() {
        let messages = [
            WorkerMessage::Hello {
                backend: cpu_backend(),
            },
            WorkerMessage::HelloAck {
                accepted_backend: cpu_backend(),
            },
            WorkerMessage::Ready,
            WorkerMessage::Execute { plan: vec![1] },
            WorkerMessage::Cancel {
                reason: "test".into(),
            },
            WorkerMessage::Event { event: vec![2] },
            WorkerMessage::OutputProposal {
                proposal: WorkerOutputProposal::new(Uuid::nil(), vec![3], vec![4])
                    .expect("bounded proposal"),
            },
            WorkerMessage::Heartbeat,
            WorkerMessage::Shutdown,
            WorkerMessage::Fatal {
                code: "fault".into(),
                message: "fixture".into(),
            },
            WorkerMessage::Lifecycle {
                event: WorkerLifecycleEvent::ExecutionStarted,
            },
            WorkerMessage::RegistryDeploymentBegin {
                deployment: deployment_begin(),
            },
            WorkerMessage::RegistryDeploymentChunk {
                chunk: WorkerRegistryDeploymentChunk::new(
                    WorkerRegistryGeneration::new(1).expect("nonzero generation"),
                    0,
                    WorkerComponentContent::Manifest,
                    0,
                    vec![1, 2],
                )
                .expect("bounded chunk"),
            },
            WorkerMessage::RegistryDeploymentCommit {
                commit: WorkerRegistryDeploymentCommit::new(
                    WorkerRegistryGeneration::new(1).expect("nonzero generation"),
                    digest('c'),
                ),
            },
            WorkerMessage::RegistryDeploymentAck {
                acknowledgement: WorkerRegistryDeploymentAck::new(
                    WorkerRegistryGeneration::new(1).expect("nonzero generation"),
                    digest('c'),
                    1,
                )
                .expect("bounded acknowledgement"),
            },
            WorkerMessage::RegistryDeploymentRejected {
                rejection: WorkerRegistryDeploymentRejection::new(
                    WorkerRegistryGeneration::new(1).expect("nonzero generation"),
                    digest('c'),
                    WorkerRegistryDeploymentRejectionReason::InvalidAuthorization,
                ),
            },
        ];
        for message in messages {
            let original = envelope(message);
            let frame = encode_worker_frame(&original).expect("encodable frame");
            assert_eq!(decode_worker_frame(&frame), Ok(original));
        }
    }

    #[test]
    fn fatal_diagnostics_are_bounded_typed_and_control_free() {
        for (code, message) in [
            (String::new(), "message".to_owned()),
            ("UPPERCASE".to_owned(), "message".to_owned()),
            ("path/code".to_owned(), "message".to_owned()),
            (
                "a".repeat(MAX_WORKER_FATAL_CODE_BYTES + 1),
                "message".to_owned(),
            ),
            ("backend_unavailable".to_owned(), String::new()),
            (
                "backend_unavailable".to_owned(),
                "m".repeat(MAX_WORKER_FATAL_MESSAGE_BYTES + 1),
            ),
            ("backend_unavailable".to_owned(), "line\nfeed".to_owned()),
        ] {
            let message = envelope(WorkerMessage::Fatal { code, message });
            assert!(matches!(
                encode_worker_frame(&message),
                Err(WorkerProtocolError::InvalidPayload(reason))
                    if reason == "worker fatal diagnostic is invalid"
            ));
        }
        let bounded = envelope(WorkerMessage::Fatal {
            code: "backend_unavailable".to_owned(),
            message: "typed\tdiagnostic".to_owned(),
        });
        let frame = encode_worker_frame(&bounded).expect("bounded fatal diagnostic");
        assert_eq!(decode_worker_frame(&frame), Ok(bounded));
    }

    #[test]
    fn registry_deployment_rejection_requires_its_exact_wire_version() {
        let mut rejection = WorkerRegistryDeploymentRejection::new(
            WorkerRegistryGeneration::new(1).expect("nonzero generation"),
            digest('c'),
            WorkerRegistryDeploymentRejectionReason::InvalidCandidate,
        );
        rejection.version = WORKER_REGISTRY_DEPLOYMENT_REJECTION_VERSION + 1;
        let malformed = envelope(WorkerMessage::RegistryDeploymentRejected { rejection });
        assert!(matches!(
            encode_worker_frame(&malformed),
            Err(WorkerProtocolError::InvalidComponentDeployment(message))
                if message.contains("rejection version")
        ));
        let payload = postcard::to_stdvec(&malformed).expect("malformed fixture serializes");
        let mut frame = u32::try_from(payload.len())
            .expect("fixture length fits the frame prefix")
            .to_le_bytes()
            .to_vec();
        frame.extend_from_slice(&payload);
        assert!(matches!(
            decode_worker_frame(&frame),
            Err(WorkerProtocolError::InvalidComponentDeployment(message))
                if message.contains("rejection version")
        ));
    }

    #[test]
    fn component_deployment_contract_is_bounded_canonical_and_path_free() {
        assert_eq!(
            WorkerRegistryGeneration::new(0),
            Err(WorkerComponentDeploymentError::ZeroGeneration)
        );
        for invalid in ["a".repeat(63), "A".repeat(64), "g".repeat(64)] {
            assert_eq!(
                WorkerSha256Digest::new(invalid),
                Err(WorkerComponentDeploymentError::InvalidSha256)
            );
        }
        assert_eq!(
            component_descriptor("extension-a", 'b', 0, 1),
            Err(WorkerComponentDeploymentError::EmptyContent(
                WorkerComponentContent::Manifest
            ))
        );
        assert_eq!(
            component_descriptor(
                "extension-a",
                'b',
                1,
                u64::try_from(MAX_WORKER_COMPONENT_BYTES).expect("bounded maximum") + 1,
            ),
            Err(WorkerComponentDeploymentError::ContentTooLarge(
                WorkerComponentContent::Component
            ))
        );
        assert_eq!(
            component_descriptor("../escape", 'b', 2, 3),
            Err(WorkerComponentDeploymentError::InvalidIdentity(
                WorkerComponentIdentityField::ExtensionId
            ))
        );
        let descriptor =
            component_descriptor("extension-a", 'b', 2, 3).expect("bounded descriptor");
        assert_eq!(descriptor.extension_id(), "extension-a");
        assert_eq!(descriptor.extension_version(), "1.2.3");
        assert_eq!(descriptor.plugin_identifier(), "plugin.identifier");
        assert_eq!(descriptor.plugin_version(), "4.5.6");
        assert_eq!(descriptor.authorization_generation(), &digest('d'));
        assert_eq!(
            WorkerRegistryDeploymentBegin::new(
                WorkerRegistryGeneration::new(1).expect("nonzero generation"),
                digest('c'),
                vec![descriptor.clone(), descriptor],
            ),
            Err(WorkerComponentDeploymentError::DuplicateExtensionIdentity)
        );
        assert_eq!(
            WorkerRegistryDeploymentBegin::new(
                WorkerRegistryGeneration::new(1).expect("nonzero generation"),
                digest('c'),
                vec![
                    component_descriptor("extension-b", 'c', 2, 3).expect("bounded descriptor"),
                    component_descriptor("extension-a", 'b', 2, 3).expect("bounded descriptor"),
                ],
            ),
            Err(WorkerComponentDeploymentError::NonCanonicalComponentOrder)
        );
        assert_eq!(
            WorkerRegistryDeploymentChunk::new(
                WorkerRegistryGeneration::new(1).expect("nonzero generation"),
                0,
                WorkerComponentContent::Manifest,
                0,
                vec![0; MAX_WORKER_COMPONENT_CHUNK_BYTES + 1],
            ),
            Err(WorkerComponentDeploymentError::ChunkTooLarge)
        );

        let source = include_str!("worker_protocol.rs");
        let deployment_start = source
            .find("pub struct WorkerComponentDescriptor")
            .expect("deployment declaration");
        let envelope_start = source
            .find("pub struct WorkerEnvelope")
            .expect("worker envelope declaration");
        let deployment_wire = &source[deployment_start..envelope_start];
        for prohibited in ["Path", "PathBuf", "AssetRoot", "TypedRoot"] {
            assert!(!deployment_wire.contains(prohibited));
        }
    }

    #[test]
    fn registry_digest_material_binds_component_identity_and_authorization() {
        let generation = WorkerRegistryGeneration::new(1).expect("nonzero generation");
        let original = deployment_begin();
        let changed_identity = WorkerRegistryDeploymentBegin::new(
            generation,
            digest('c'),
            vec![
                WorkerComponentDescriptor::new(
                    "extension-a",
                    "1.2.3",
                    "different.plugin",
                    "4.5.6",
                    digest('d'),
                    digest('a'),
                    digest('b'),
                    2,
                    1,
                    3,
                )
                .expect("bounded descriptor"),
            ],
        )
        .expect("bounded deployment");
        let changed_authorization = WorkerRegistryDeploymentBegin::new(
            generation,
            digest('c'),
            vec![
                WorkerComponentDescriptor::new(
                    "extension-a",
                    "1.2.3",
                    "plugin.identifier",
                    "4.5.6",
                    digest('e'),
                    digest('a'),
                    digest('b'),
                    2,
                    1,
                    3,
                )
                .expect("bounded descriptor"),
            ],
        )
        .expect("bounded deployment");

        assert_ne!(
            original.digest_material(),
            changed_identity.digest_material()
        );
        assert_ne!(
            original.digest_material(),
            changed_authorization.digest_material()
        );
    }

    #[test]
    fn opaque_worker_extensions_are_rejected() {
        let mut message = envelope(WorkerMessage::Ready);
        message.extensions.insert("escape".into(), vec![1]);
        assert_eq!(
            encode_worker_frame(&message),
            Err(WorkerProtocolError::OpaqueExtensions)
        );
    }

    #[test]
    fn invalid_length_and_version_are_rejected() {
        assert_eq!(
            decode_worker_frame(&[0, 0, 0]),
            Err(WorkerProtocolError::LengthMismatch)
        );
        let mut unsupported = envelope(WorkerMessage::Ready);
        unsupported.version = WORKER_PROTOCOL_VERSION + 1;
        assert_eq!(
            encode_worker_frame(&unsupported),
            Err(WorkerProtocolError::UnsupportedVersion(
                WORKER_PROTOCOL_VERSION + 1
            ))
        );

        let legacy = envelope(WorkerMessage::Ready);
        let mut payload = postcard::to_stdvec(&legacy).expect("current envelope serializes");
        assert_eq!(
            payload.first().copied(),
            Some(WORKER_PROTOCOL_VERSION as u8)
        );
        payload[0] = LEGACY_WORKER_PROTOCOL_VERSION as u8;
        let mut frame = u32::try_from(payload.len())
            .expect("fixture length fits")
            .to_le_bytes()
            .to_vec();
        frame.extend_from_slice(&payload);
        assert_eq!(
            decode_worker_frame(&frame),
            Err(WorkerProtocolError::UnsupportedVersion(
                LEGACY_WORKER_PROTOCOL_VERSION
            ))
        );
    }

    #[test]
    fn legacy_protocol_is_rejected_before_changed_payload_decode() {
        let payload = [LEGACY_WORKER_PROTOCOL_VERSION as u8, 0xff];
        let mut frame = u32::try_from(payload.len())
            .expect("fixture length fits")
            .to_le_bytes()
            .to_vec();
        frame.extend_from_slice(&payload);
        assert_eq!(
            decode_worker_frame(&frame),
            Err(WorkerProtocolError::UnsupportedVersion(
                LEGACY_WORKER_PROTOCOL_VERSION
            ))
        );
    }

    #[test]
    fn oversized_frames_are_rejected_before_payload_decoding() {
        let oversized = envelope(WorkerMessage::Execute {
            plan: vec![0; MAX_WORKER_FRAME_BYTES],
        });
        assert_eq!(
            encode_worker_frame(&oversized),
            Err(WorkerProtocolError::Oversized)
        );

        let mut frame = (u32::try_from(MAX_WORKER_FRAME_BYTES).expect("bounded maximum") + 1)
            .to_le_bytes()
            .to_vec();
        frame.resize(MAX_WORKER_FRAME_BYTES + 5, 0);
        assert_eq!(
            decode_worker_frame(&frame),
            Err(WorkerProtocolError::Oversized)
        );

        let oversized_event = envelope(WorkerMessage::Event {
            event: vec![0; MAX_ENCODED_PREVIEW_BYTES + 1],
        });
        assert_eq!(
            encode_worker_frame(&oversized_event),
            Err(WorkerProtocolError::OversizedEvent)
        );
    }

    #[test]
    fn plugin_results_round_trip_through_the_postcard_envelope() {
        let succeeded = envelope(WorkerMessage::PluginResult {
            outcome: WorkerPluginExecutionOutcome::Succeeded(vec![1, 2, 3]),
        });
        let succeeded_frame =
            encode_worker_frame(&succeeded).expect("plugin result envelope must encode");
        assert_eq!(decode_worker_frame(&succeeded_frame), Ok(succeeded));

        let failed = envelope(WorkerMessage::PluginResult {
            outcome: WorkerPluginExecutionOutcome::Failed(
                WorkerPluginExecutionFailure::CapabilityDenied,
            ),
        });
        let failed_frame =
            encode_worker_frame(&failed).expect("plugin failure envelope must encode");
        assert_eq!(decode_worker_frame(&failed_frame), Ok(failed));

        let trapped = envelope(WorkerMessage::PluginResult {
            outcome: WorkerPluginExecutionOutcome::Failed(WorkerPluginExecutionFailure::Trap {
                diagnostic: "wasm trap: interrupt".to_owned(),
            }),
        });
        let trapped_frame =
            encode_worker_frame(&trapped).expect("bounded trap diagnostic must encode");
        assert_eq!(decode_worker_frame(&trapped_frame), Ok(trapped));

        for diagnostic in [
            String::new(),
            "x".repeat(MAX_WORKER_PLUGIN_DIAGNOSTIC_CHARS + 1),
            "wasm trap\0secret".to_owned(),
        ] {
            assert!(matches!(
                encode_worker_frame(&envelope(WorkerMessage::PluginResult {
                    outcome: WorkerPluginExecutionOutcome::Failed(
                        WorkerPluginExecutionFailure::Trap { diagnostic },
                    ),
                })),
                Err(WorkerProtocolError::InvalidPayload(_))
            ));
        }
    }

    #[test]
    fn output_proposals_are_bounded_on_construction_and_decode() {
        let proposal = WorkerOutputProposal::new(
            Uuid::from_u128(0xfeed),
            b"metadata".to_vec(),
            b"content".to_vec(),
        )
        .expect("bounded proposal must construct");
        let encoded = postcard::to_stdvec(&proposal).expect("bounded proposal must encode");
        assert_eq!(
            postcard::from_bytes::<WorkerOutputProposal>(&encoded),
            Ok(proposal)
        );

        assert_eq!(
            WorkerOutputProposal::new(
                Uuid::nil(),
                vec![0; MAX_WORKER_OUTPUT_METADATA_BYTES + 1],
                Vec::new(),
            ),
            Err(WorkerOutputProposalError::MetadataTooLarge)
        );
        assert_eq!(
            WorkerOutputProposal::new(
                Uuid::nil(),
                Vec::new(),
                vec![0; MAX_WORKER_OUTPUT_CONTENT_BYTES + 1],
            ),
            Err(WorkerOutputProposalError::ContentTooLarge)
        );

        let wire = WorkerOutputProposalWire {
            proposal_id: Uuid::nil(),
            metadata: Vec::new(),
            content: vec![0; MAX_WORKER_OUTPUT_CONTENT_BYTES + 1],
        };
        let encoded = postcard::to_stdvec(&wire).expect("wire fixture serializes");
        assert!(postcard::from_bytes::<WorkerOutputProposal>(&encoded).is_err());
    }

    #[test]
    fn val_cancel_001_worker_projection_and_bounds_are_explicit() {
        let cancellation = crate::CancellationToken::default();
        assert!(cancellation.cancel());
        let cancel = envelope(WorkerMessage::Cancel {
            reason: "operator requested cancellation".to_owned(),
        });
        let frame = encode_worker_frame(&cancel).expect("cancel projection must encode");
        assert_eq!(decode_worker_frame(&frame), Ok(cancel));

        let oversized_event = envelope(WorkerMessage::Event {
            event: vec![0; MAX_ENCODED_PREVIEW_BYTES + 1],
        });
        assert_eq!(
            encode_worker_frame(&oversized_event),
            Err(WorkerProtocolError::OversizedEvent)
        );

        let oversized_frame = envelope(WorkerMessage::Execute {
            plan: vec![0; MAX_WORKER_FRAME_BYTES],
        });
        assert_eq!(
            encode_worker_frame(&oversized_frame),
            Err(WorkerProtocolError::Oversized)
        );
    }

    #[test]
    fn worker_wire_schema_declares_no_host_path_or_typed_root() {
        let source = include_str!("worker_protocol.rs");
        let message_start = source
            .find("pub enum WorkerMessage")
            .expect("worker message declaration");
        let error_start = source[message_start..]
            .find("pub enum WorkerProtocolError")
            .map(|offset| message_start + offset)
            .expect("worker protocol error declaration");
        let wire_declarations = &source[..error_start];

        for prohibited in ["Path", "PathBuf", "AssetRoot", "AssetRoots", "TypedRoot"] {
            assert!(
                !wire_declarations.contains(prohibited),
                "worker wire schema contains prohibited host type {prohibited}"
            );
        }
    }

    #[test]
    fn backend_capabilities_are_bounded_canonical_and_checked_on_decode() {
        let duplicate = WorkerOperationSupport::for_tensor_v2(
            WorkerPrimitiveOperationV2::Unary(WorkerUnaryOperationV1::Absolute),
            WorkerTensorRoleV1::Input,
            WorkerDType::F32,
            WorkerLayout::Contiguous,
        )
        .expect("unary is a tensor primitive");
        assert_eq!(
            WorkerBackendCapabilities::new(DeviceKind::Cpu, 0, vec![duplicate, duplicate], vec![],),
            Err(WorkerBackendCapabilityError::Duplicate)
        );
        let canonical =
            WorkerBackendCapabilities::new(DeviceKind::Cpu, 0, vec![duplicate], vec![duplicate])
                .expect("unique declarations canonicalize");
        assert_eq!(canonical.supported(), [duplicate]);
        assert_eq!(canonical.deterministic(), [duplicate]);

        let empty = WorkerBackendCapabilitiesWire {
            device: DeviceKind::Cpu,
            ordinal: 0,
            supported: vec![],
            deterministic: vec![],
            properties: None,
        };
        assert_eq!(
            WorkerBackendCapabilities::try_from(empty.clone()),
            Err(WorkerBackendCapabilityError::Empty)
        );
        let malformed_payload = postcard::to_stdvec(&empty).expect("encode malformed wire fixture");
        assert!(postcard::from_bytes::<WorkerBackendCapabilities>(&malformed_payload).is_err());

        let oversized = WorkerBackendCapabilitiesWire {
            device: DeviceKind::Cpu,
            ordinal: 0,
            supported: vec![duplicate; MAX_WORKER_BACKEND_CAPABILITY_ENTRIES + 1],
            deterministic: vec![],
            properties: None,
        };
        assert_eq!(
            WorkerBackendCapabilities::try_from(oversized.clone()),
            Err(WorkerBackendCapabilityError::Oversized)
        );
        let oversized_payload =
            postcard::to_stdvec(&oversized).expect("encode oversized wire fixture");
        assert!(postcard::from_bytes::<WorkerBackendCapabilities>(&oversized_payload).is_err());
    }

    #[test]
    fn primitive_support_is_versioned_closed_and_event_formats_are_absent() {
        let event = WorkerOperationSupport::for_event_v2(WorkerPrimitiveOperationV2::RecordEvent)
            .expect("record-event is an event primitive");
        assert_eq!(event.version(), WORKER_OPERATION_SUPPORT_VERSION);
        assert_eq!(event.category(), WorkerOperationCategory::Event);
        assert_eq!(event.role(), None);
        assert_eq!(event.dtype(), None);
        assert_eq!(event.layout(), None);

        assert_eq!(
            WorkerOperationSupport::for_tensor_v2(
                WorkerPrimitiveOperationV2::WaitEvent,
                WorkerTensorRoleV1::Output,
                WorkerDType::F32,
                WorkerLayout::Contiguous,
            ),
            Err(WorkerOperationSupportError::EventTensorSignature)
        );
        assert_eq!(
            WorkerOperationSupport::for_event_v2(WorkerPrimitiveOperationV2::Allocation),
            Err(WorkerOperationSupportError::MissingTensorSignature)
        );

        let unsupported_version = WorkerOperationSupportWire {
            version: WORKER_OPERATION_SUPPORT_VERSION + 1,
            operation: WorkerPrimitiveOperationV2::Allocation,
            role: Some(WorkerTensorRoleV1::Output),
            dtype: Some(WorkerDType::F32),
            layout: Some(WorkerLayout::Contiguous),
        };
        assert_eq!(
            WorkerOperationSupport::try_from(unsupported_version),
            Err(WorkerOperationSupportError::UnsupportedVersion(
                WORKER_OPERATION_SUPPORT_VERSION + 1
            ))
        );

        let missing_role = WorkerOperationSupportWire {
            version: WORKER_OPERATION_SUPPORT_VERSION,
            operation: WorkerPrimitiveOperationV2::Allocation,
            role: None,
            dtype: Some(WorkerDType::F32),
            layout: Some(WorkerLayout::Contiguous),
        };
        assert_eq!(
            WorkerOperationSupport::try_from(missing_role),
            Err(WorkerOperationSupportError::MissingTensorSignature)
        );

        let unknown_operation = r#"{
            "version": 2,
            "operation": "future_primitive",
            "role": "input",
            "dtype": "f32",
            "layout": "contiguous"
        }"#;
        assert!(serde_json::from_str::<WorkerOperationSupport>(unknown_operation).is_err());
        let unknown_field = r#"{
            "version": 2,
            "operation": "allocation",
            "role": "output",
            "dtype": "f32",
            "layout": "contiguous",
            "future": true
        }"#;
        assert!(serde_json::from_str::<WorkerOperationSupport>(unknown_field).is_err());
    }

    #[test]
    fn worker_primitive_operation_v2_wire_round_trip() {
        for operation in [
            WorkerPrimitiveOperationV2::Unary(WorkerUnaryOperationV1::Tangent),
            WorkerPrimitiveOperationV2::Unary(WorkerUnaryOperationV1::ArcTangent),
            WorkerPrimitiveOperationV2::Unary(WorkerUnaryOperationV1::ArcHyperbolicTangent),
            WorkerPrimitiveOperationV2::Binary(WorkerBinaryOperationV1::Atan2),
            WorkerPrimitiveOperationV2::Binary(WorkerBinaryOperationV1::LogAddExp),
            WorkerPrimitiveOperationV2::LinearAlgebra(
                WorkerLinearAlgebraOperationV1::MatrixMultiply,
            ),
            WorkerPrimitiveOperationV2::Gather,
            WorkerPrimitiveOperationV2::Scatter,
            WorkerPrimitiveOperationV2::MaskedSelect,
            WorkerPrimitiveOperationV2::Convolution,
            WorkerPrimitiveOperationV2::CustomKernel,
        ] {
            let encoded = postcard::to_stdvec(&operation).expect("primitive operation encodes");
            let decoded: WorkerPrimitiveOperationV2 =
                postcard::from_bytes(&encoded).expect("primitive operation decodes");
            assert_eq!(decoded, operation);
        }
    }

    #[test]
    fn worker_primitive_operation_postcard_discriminants_are_append_only() {
        let fixtures: &[(WorkerPrimitiveOperationV1, &[u8])] = &[
            (WorkerPrimitiveOperationV1::Allocation, &[0]),
            (WorkerPrimitiveOperationV1::Copy, &[1]),
            (WorkerPrimitiveOperationV1::Fill, &[2]),
            (
                WorkerPrimitiveOperationV1::Unary(WorkerUnaryOperationV1::Tangent),
                &[3, 19],
            ),
            (
                WorkerPrimitiveOperationV1::Binary(WorkerBinaryOperationV1::Atan2),
                &[4, 16],
            ),
            (
                WorkerPrimitiveOperationV1::BinaryScalar(WorkerBinaryOperationV1::LogAddExp),
                &[5, 17],
            ),
            (
                WorkerPrimitiveOperationV1::Reduction(
                    WorkerReductionOperationV1::StandardDeviation,
                ),
                &[6, 10],
            ),
            (WorkerPrimitiveOperationV1::Select, &[7]),
            (WorkerPrimitiveOperationV1::Narrow, &[8]),
            (
                WorkerPrimitiveOperationV1::Resize(WorkerResizeModeV1::Lanczos),
                &[9, 4],
            ),
            (WorkerPrimitiveOperationV1::RecordEvent, &[10]),
            (WorkerPrimitiveOperationV1::WaitEvent, &[11]),
        ];

        for (operation, expected) in fixtures {
            let encoded = postcard::to_stdvec(operation).expect("primitive operation encodes");
            assert_eq!(encoded, *expected, "wire bytes changed for {operation:?}");
            let decoded: WorkerPrimitiveOperationV1 =
                postcard::from_bytes(expected).expect("pinned primitive operation decodes");
            assert_eq!(decoded, *operation);
        }

        let linear_algebra = WorkerPrimitiveOperationV2::LinearAlgebra(
            WorkerLinearAlgebraOperationV1::MatrixMultiply,
        );
        assert_eq!(
            postcard::to_stdvec(&linear_algebra).expect("linear algebra operation encodes"),
            [12, 0]
        );
        for (operation, expected) in [
            (WorkerPrimitiveOperationV2::Gather, vec![13]),
            (WorkerPrimitiveOperationV2::Scatter, vec![14]),
            (WorkerPrimitiveOperationV2::MaskedSelect, vec![15]),
            (WorkerPrimitiveOperationV2::Convolution, vec![16]),
            (WorkerPrimitiveOperationV2::CustomKernel, vec![17]),
        ] {
            assert_eq!(
                postcard::to_stdvec(&operation).expect("appended V2 operation encodes"),
                expected
            );
        }
    }

    #[test]
    fn worker_native_device_properties_round_trip_with_only_wire_bounds() {
        let support = WorkerOperationSupport::for_tensor_v2(
            WorkerPrimitiveOperationV2::Allocation,
            WorkerTensorRoleV1::Output,
            WorkerDType::F16,
            WorkerLayout::Contiguous,
        )
        .expect("allocation is a tensor primitive");
        let properties = WorkerNativeDeviceProperties::new_with_allocation_limit(
            "Cambricon MLU fixture",
            24 * 1024 * 1024,
            20 * 1024 * 1024,
            11,
            0,
            Some("Neuware 1.20".to_owned()),
            true,
        )
        .expect("bounded properties");
        let backend = WorkerBackendCapabilities::new_with_properties(
            DeviceKind::Mlu,
            2,
            vec![support],
            vec![support],
            Some(properties.clone()),
        )
        .expect("bounded backend declaration");
        let message = envelope(WorkerMessage::HelloAck {
            accepted_backend: backend,
        });
        let frame = encode_worker_frame(&message).expect("properties encode in protocol 7");
        assert_eq!(decode_worker_frame(&frame), Ok(message));
        assert_eq!(properties.name(), "Cambricon MLU fixture");
        assert_eq!(properties.total_memory_bytes(), 24 * 1024 * 1024);
        assert_eq!(properties.allocation_limit_bytes(), 20 * 1024 * 1024);
        assert_eq!(properties.major(), 11);
        assert_eq!(properties.minor(), 0);
        assert_eq!(properties.architecture(), Some("Neuware 1.20"));
        assert!(properties.has_fp16());

        assert_eq!(
            WorkerNativeDeviceProperties::new_with_allocation_limit(
                "fixture", 24, 0, 0, 0, None, false,
            ),
            Err(WorkerNativeDevicePropertiesError::InvalidMemoryLimit)
        );
        assert_eq!(
            WorkerNativeDeviceProperties::new_with_allocation_limit(
                "fixture", 24, 25, 0, 0, None, false,
            ),
            Err(WorkerNativeDevicePropertiesError::InvalidMemoryLimit)
        );

        assert_eq!(
            WorkerNativeDeviceProperties::new(
                "x".repeat(MAX_WORKER_DEVICE_PROPERTY_BYTES + 1),
                1,
                0,
                0,
                None,
                false,
            ),
            Err(WorkerNativeDevicePropertiesError::NameOversized)
        );
        let oversized_wire = WorkerNativeDevicePropertiesWire {
            name: "fixture".to_owned(),
            total_memory_bytes: 1,
            allocation_limit_bytes: 1,
            major: 0,
            minor: 0,
            architecture: Some("x".repeat(MAX_WORKER_DEVICE_PROPERTY_BYTES + 1)),
            has_fp16: false,
        };
        let payload = postcard::to_stdvec(&oversized_wire).expect("malformed fixture encodes");
        assert!(postcard::from_bytes::<WorkerNativeDeviceProperties>(&payload).is_err());
    }

    #[test]
    fn directml_native_device_properties_round_trip_through_the_generic_wire_dto() {
        let support = WorkerOperationSupport::for_tensor_v2(
            WorkerPrimitiveOperationV2::Allocation,
            WorkerTensorRoleV1::Output,
            WorkerDType::F16,
            WorkerLayout::Contiguous,
        )
        .expect("allocation is a tensor primitive");
        let properties = WorkerNativeDeviceProperties::new_with_allocation_limit(
            "DirectML fixture adapter",
            24 * 1024 * 1024 * 1024,
            18 * 1024 * 1024 * 1024,
            1,
            13,
            Some("DXGI adapter LUID 0x0123456789abcdef".to_owned()),
            true,
        )
        .expect("bounded DirectML properties");
        let backend = WorkerBackendCapabilities::new_with_properties(
            DeviceKind::DirectMl,
            0,
            vec![support],
            vec![support],
            Some(properties.clone()),
        )
        .expect("bounded DirectML backend declaration");
        let message = envelope(WorkerMessage::HelloAck {
            accepted_backend: backend,
        });
        let frame = encode_worker_frame(&message).expect("DirectML properties encode");
        assert_eq!(decode_worker_frame(&frame), Ok(message));
        assert_eq!(properties.name(), "DirectML fixture adapter");
        assert_eq!(properties.total_memory_bytes(), 24 * 1024 * 1024 * 1024);
        assert_eq!(properties.allocation_limit_bytes(), 18 * 1024 * 1024 * 1024);
        assert_eq!(properties.major(), 1);
        assert_eq!(properties.minor(), 13);
        assert_eq!(
            properties.architecture(),
            Some("DXGI adapter LUID 0x0123456789abcdef")
        );
        assert!(properties.has_fp16());
    }

    #[test]
    fn npu_native_device_properties_and_effective_ceiling_round_trip_through_the_generic_wire_dto()
    {
        let support = WorkerOperationSupport::for_tensor_v2(
            WorkerPrimitiveOperationV2::Allocation,
            WorkerTensorRoleV1::Output,
            WorkerDType::F16,
            WorkerLayout::Contiguous,
        )
        .expect("allocation is a tensor primitive");
        let properties = WorkerNativeDeviceProperties::new_with_allocation_limit(
            "Huawei Ascend fixture",
            32 * 1024 * 1024 * 1024,
            12 * 1024 * 1024 * 1024,
            8,
            0,
            Some("AscendCL 8.0.0".to_owned()),
            true,
        )
        .expect("bounded NPU properties");
        let backend = WorkerBackendCapabilities::new_with_properties(
            DeviceKind::Npu,
            2,
            vec![support],
            vec![support],
            Some(properties.clone()),
        )
        .expect("bounded NPU backend declaration");
        let message = envelope(WorkerMessage::HelloAck {
            accepted_backend: backend,
        });
        let frame = encode_worker_frame(&message).expect("NPU properties encode");
        assert_eq!(decode_worker_frame(&frame), Ok(message));
        assert_eq!(properties.name(), "Huawei Ascend fixture");
        assert_eq!(properties.total_memory_bytes(), 32 * 1024 * 1024 * 1024);
        assert_eq!(properties.allocation_limit_bytes(), 12 * 1024 * 1024 * 1024);
        assert_eq!(properties.major(), 8);
        assert_eq!(properties.minor(), 0);
        assert_eq!(properties.architecture(), Some("AscendCL 8.0.0"));
        assert!(properties.has_fp16());
    }

    #[test]
    fn xpu_native_device_properties_and_effective_ceiling_round_trip_through_the_generic_wire_dto()
    {
        let support = WorkerOperationSupport::for_tensor_v2(
            WorkerPrimitiveOperationV2::Allocation,
            WorkerTensorRoleV1::Output,
            WorkerDType::F16,
            WorkerLayout::Contiguous,
        )
        .expect("allocation is a tensor primitive");
        let properties = WorkerNativeDeviceProperties::new_with_allocation_limit(
            "Intel XPU fixture",
            16 * 1024 * 1024 * 1024,
            6 * 1024 * 1024 * 1024,
            1,
            11,
            Some("Intel 0x8086:0x56a0; oneDNN 3.5.0".to_owned()),
            true,
        )
        .expect("bounded XPU properties");
        let backend = WorkerBackendCapabilities::new_with_properties(
            DeviceKind::Xpu,
            3,
            vec![support],
            vec![support],
            Some(properties.clone()),
        )
        .expect("bounded XPU backend declaration");
        let message = envelope(WorkerMessage::HelloAck {
            accepted_backend: backend,
        });
        let frame = encode_worker_frame(&message).expect("XPU properties encode");
        assert_eq!(decode_worker_frame(&frame), Ok(message));
        assert_eq!(properties.name(), "Intel XPU fixture");
        assert_eq!(properties.total_memory_bytes(), 16 * 1024 * 1024 * 1024);
        assert_eq!(properties.allocation_limit_bytes(), 6 * 1024 * 1024 * 1024);
        assert_eq!(properties.major(), 1);
        assert_eq!(properties.minor(), 11);
        assert_eq!(
            properties.architecture(),
            Some("Intel 0x8086:0x56a0; oneDNN 3.5.0")
        );
        assert!(properties.has_fp16());
    }

    #[test]
    fn cuda_native_device_properties_and_effective_ceiling_round_trip_through_the_generic_wire_dto()
    {
        let support = WorkerOperationSupport::for_tensor_v2(
            WorkerPrimitiveOperationV2::Allocation,
            WorkerTensorRoleV1::Output,
            WorkerDType::F16,
            WorkerLayout::Contiguous,
        )
        .expect("allocation is a tensor primitive");
        let properties = WorkerNativeDeviceProperties::new_with_allocation_limit(
            "NVIDIA CUDA fixture",
            24 * 1024 * 1024 * 1024,
            18 * 1024 * 1024 * 1024,
            12,
            8,
            Some("CUDA driver 12080; NVRTC 12.8".to_owned()),
            true,
        )
        .expect("bounded CUDA properties");
        let backend = WorkerBackendCapabilities::new_with_properties(
            DeviceKind::Cuda,
            3,
            vec![support],
            vec![support],
            Some(properties.clone()),
        )
        .expect("bounded CUDA backend declaration");
        let message = envelope(WorkerMessage::HelloAck {
            accepted_backend: backend,
        });
        let frame = encode_worker_frame(&message).expect("CUDA properties encode");
        assert_eq!(decode_worker_frame(&frame), Ok(message));
        assert_eq!(properties.name(), "NVIDIA CUDA fixture");
        assert_eq!(properties.total_memory_bytes(), 24 * 1024 * 1024 * 1024);
        assert_eq!(properties.allocation_limit_bytes(), 18 * 1024 * 1024 * 1024);
        assert_eq!(properties.major(), 12);
        assert_eq!(properties.minor(), 8);
        assert_eq!(
            properties.architecture(),
            Some("CUDA driver 12080; NVRTC 12.8")
        );
        assert!(properties.has_fp16());
    }

    #[test]
    fn operation_support_schema_versions_reject_mixed_peers_before_use() {
        let legacy = WorkerOperationSupportV1::for_tensor(
            WorkerPrimitiveOperationV1::Allocation,
            WorkerTensorRoleV1::Output,
            WorkerDType::F32,
            WorkerLayout::Contiguous,
        )
        .expect("legacy allocation support is valid");
        let legacy_bytes = postcard::to_stdvec(&legacy).expect("legacy support serializes");
        assert_eq!(legacy_bytes, [1, 0, 1, 1, 1, 1, 1, 0]);
        assert_eq!(
            postcard::from_bytes::<WorkerOperationSupportV1>(&legacy_bytes),
            Ok(legacy)
        );
        assert!(postcard::from_bytes::<WorkerOperationSupport>(&legacy_bytes).is_err());

        let legacy_schema_with_current_operation = WorkerOperationSupportWire {
            version: LEGACY_WORKER_OPERATION_SUPPORT_VERSION,
            operation: WorkerPrimitiveOperationV2::Allocation,
            role: Some(WorkerTensorRoleV1::Output),
            dtype: Some(WorkerDType::F32),
            layout: Some(WorkerLayout::Contiguous),
        };
        assert_eq!(
            WorkerOperationSupport::try_from(legacy_schema_with_current_operation),
            Err(WorkerOperationSupportError::UnsupportedVersion(
                LEGACY_WORKER_OPERATION_SUPPORT_VERSION
            ))
        );

        let current = WorkerOperationSupport::for_tensor_v2(
            WorkerPrimitiveOperationV2::LinearAlgebra(
                WorkerLinearAlgebraOperationV1::MatrixMultiply,
            ),
            WorkerTensorRoleV1::Input,
            WorkerDType::F32,
            WorkerLayout::Contiguous,
        )
        .expect("current linear algebra support is valid");
        let current_bytes = postcard::to_stdvec(&current).expect("current support serializes");
        assert!(postcard::from_bytes::<WorkerOperationSupportV1>(&current_bytes).is_err());
    }
}
