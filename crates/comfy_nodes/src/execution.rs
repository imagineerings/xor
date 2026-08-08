use comfy_tensor::ScratchReservation;
use comfy_types::{ApiPrompt, AttemptId, CancellationToken, NodeId, PromptId};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};
use thiserror::Error;
use uuid::Uuid;

pub const NATIVE_NODE_CONTRACT_SCHEMA_VERSION: u16 = 1;
pub const NATIVE_OPAQUE_HANDLE_SCHEMA_VERSION: u16 = 1;

const MAX_IDENTIFIER_BYTES: usize = 4_096;
const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_UNKNOWN_BYTES: usize = 1024 * 1024;
const MAX_VALUE_DEPTH: usize = 32;
const MAX_LIST_VALUES: usize = 1_000_000;
const MAX_PORTS: usize = 65_536;
const MAX_TYPE_UNION_MEMBERS: usize = 128;
const MAX_EFFECT_METADATA_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePrimitiveType {
    Null,
    Boolean,
    Integer,
    Number,
    String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum NativePrimitive {
    Null,
    Boolean(bool),
    Integer(i64),
    UnsignedInteger(u64),
    Number(f64),
    String(String),
}

impl NativePrimitive {
    pub const fn primitive_type(&self) -> NativePrimitiveType {
        match self {
            Self::Null => NativePrimitiveType::Null,
            Self::Boolean(_) => NativePrimitiveType::Boolean,
            Self::Integer(_) | Self::UnsignedInteger(_) => NativePrimitiveType::Integer,
            Self::Number(_) => NativePrimitiveType::Number,
            Self::String(_) => NativePrimitiveType::String,
        }
    }

    fn validate(&self) -> Result<(), NativeNodeContractError> {
        match self {
            Self::Number(value) if !value.is_finite() => {
                Err(NativeNodeContractError::NonFiniteNumber)
            }
            Self::String(value) => validate_text("primitive string", value, MAX_TEXT_BYTES, true),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeHandleKind {
    Tensor,
    Model,
    Clip,
    Vae,
    ControlNet,
    Conditioning,
    Latent,
    Image,
    Mask,
    Audio,
    Video,
    ThreeD,
    Artifact,
    ProviderTask,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeHandleType {
    pub kind: NativeHandleKind,
    pub type_id: String,
}

impl NativeHandleType {
    pub fn new(
        kind: NativeHandleKind,
        type_id: impl Into<String>,
    ) -> Result<Self, NativeNodeContractError> {
        let handle_type = Self {
            kind,
            type_id: type_id.into(),
        };
        handle_type.validate()?;
        Ok(handle_type)
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.type_id.is_empty()
            || self.type_id.len() > MAX_IDENTIFIER_BYTES
            || !self.type_id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':' | b'@')
            })
        {
            return Err(NativeNodeContractError::InvalidHandleType);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeHandleStoreIdentity {
    pub store_id: Uuid,
    pub generation_id: Uuid,
}

impl NativeHandleStoreIdentity {
    pub fn new(store_id: Uuid, generation_id: Uuid) -> Result<Self, NativeNodeContractError> {
        let identity = Self {
            store_id,
            generation_id,
        };
        identity.validate()?;
        Ok(identity)
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.store_id.is_nil() || self.generation_id.is_nil() {
            return Err(NativeNodeContractError::InvalidHandleStoreIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOpaqueHandle {
    schema_version: u16,
    handle_type: NativeHandleType,
    store_identity: NativeHandleStoreIdentity,
    identifier: String,
    generation: u64,
    digest_sha256: Option<String>,
}

impl NativeOpaqueHandle {
    pub fn new(
        handle_type: NativeHandleType,
        store_identity: NativeHandleStoreIdentity,
        identifier: impl Into<String>,
        generation: u64,
        digest_sha256: Option<String>,
    ) -> Result<Self, NativeNodeContractError> {
        let handle = Self {
            schema_version: NATIVE_OPAQUE_HANDLE_SCHEMA_VERSION,
            handle_type,
            store_identity,
            identifier: identifier.into(),
            generation,
            digest_sha256,
        };
        handle.validate()?;
        Ok(handle)
    }

    pub fn handle_type(&self) -> &NativeHandleType {
        &self.handle_type
    }

    pub const fn store_identity(&self) -> NativeHandleStoreIdentity {
        self.store_identity
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn digest_sha256(&self) -> Option<&str> {
        self.digest_sha256.as_deref()
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.schema_version != NATIVE_OPAQUE_HANDLE_SCHEMA_VERSION {
            return Err(NativeNodeContractError::UnsupportedHandleSchema(
                self.schema_version,
            ));
        }
        self.handle_type.validate()?;
        self.store_identity.validate()?;
        validate_identifier("opaque handle identifier", &self.identifier)?;
        if self.generation == 0 {
            return Err(NativeNodeContractError::InvalidHandleGeneration);
        }
        if let Some(digest) = &self.digest_sha256
            && !valid_sha256(digest)
        {
            return Err(NativeNodeContractError::InvalidDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeValue {
    Primitive { value: NativePrimitive },
    Handle { value: NativeOpaqueHandle },
    List { values: Vec<NativeValue> },
    PreservedUnknown { type_name: String, value: Value },
}

impl NativeValue {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        self.validate_at_depth(0)
    }

    fn validate_at_depth(&self, depth: usize) -> Result<(), NativeNodeContractError> {
        if depth > MAX_VALUE_DEPTH {
            return Err(NativeNodeContractError::ValueNestingTooDeep);
        }
        match self {
            Self::Primitive { value } => value.validate(),
            Self::Handle { value } => value.validate(),
            Self::List { values } => {
                if values.len() > MAX_LIST_VALUES {
                    return Err(NativeNodeContractError::TooManyListValues);
                }
                for value in values {
                    value.validate_at_depth(depth.saturating_add(1))?;
                }
                Ok(())
            }
            Self::PreservedUnknown { type_name, value } => {
                validate_identifier("preserved unknown type", type_name)?;
                let encoded = serde_json::to_vec(value)
                    .map_err(NativeNodeContractError::EncodePreservedUnknown)?;
                if encoded.len() > MAX_UNKNOWN_BYTES {
                    return Err(NativeNodeContractError::PreservedUnknownTooLarge);
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum NativeValueType {
    Any,
    Primitive(NativePrimitiveType),
    Handle(NativeHandleType),
    PreservedUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NativeTypeUnion(Vec<NativeValueType>);

impl NativeTypeUnion {
    pub fn new(
        members: impl IntoIterator<Item = NativeValueType>,
    ) -> Result<Self, NativeNodeContractError> {
        let members = members.into_iter().collect::<Vec<_>>();
        let value = Self(members);
        value.validate()?;
        Ok(value)
    }

    pub fn members(&self) -> &[NativeValueType] {
        &self.0
    }

    pub fn accepts(&self, value: &NativeValue) -> bool {
        self.0.iter().any(|member| match (member, value) {
            (NativeValueType::Any, _) => true,
            (NativeValueType::Primitive(expected), NativeValue::Primitive { value }) => {
                *expected == value.primitive_type()
            }
            (NativeValueType::Handle(expected), NativeValue::Handle { value }) => {
                expected == value.handle_type()
            }
            (NativeValueType::PreservedUnknown, NativeValue::PreservedUnknown { .. }) => true,
            _ => false,
        })
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.0.is_empty() || self.0.len() > MAX_TYPE_UNION_MEMBERS {
            return Err(NativeNodeContractError::InvalidTypeUnion);
        }
        if !self.0.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(NativeNodeContractError::InvalidTypeUnion);
        }
        if self.0.contains(&NativeValueType::Any) && self.0.len() != 1 {
            return Err(NativeNodeContractError::InvalidTypeUnion);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePortCardinality {
    Scalar,
    List,
    Mapped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeInputDescriptor {
    pub name: String,
    pub accepted_types: NativeTypeUnion,
    pub required: bool,
    pub hidden: bool,
    pub lazy: bool,
    pub cardinality: NativePortCardinality,
    pub allows_literal: bool,
}

impl NativeInputDescriptor {
    fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_identifier("input name", &self.name)?;
        self.accepted_types.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDynamicInputDescriptor {
    pub name_template: String,
    pub start_index: u32,
    pub minimum_count: u32,
    pub maximum_count: u32,
    pub input: NativeInputDescriptor,
}

impl NativeDynamicInputDescriptor {
    fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_text(
            "dynamic input name template",
            &self.name_template,
            MAX_IDENTIFIER_BYTES,
            false,
        )?;
        if self.name_template.matches("{index}").count() != 1
            || self.minimum_count > self.maximum_count
            || self.maximum_count == 0
        {
            return Err(NativeNodeContractError::InvalidDynamicInput);
        }
        self.start_index
            .checked_add(self.maximum_count)
            .ok_or(NativeNodeContractError::InvalidDynamicInput)?;
        self.input.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOutputDescriptor {
    pub name: String,
    pub produced_type: NativeValueType,
    pub is_list: bool,
}

impl NativeOutputDescriptor {
    fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_identifier("output name", &self.name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeEffectClass {
    Pure,
    ReadsArtifact,
    WritesArtifact,
    Provider,
    ExclusiveDevice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCachePolicy {
    InputIdentity,
    Never,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeNodeDescriptor {
    pub schema_version: u16,
    pub class_type: String,
    pub implementation_version: String,
    pub inputs: Vec<NativeInputDescriptor>,
    pub dynamic_inputs: Vec<NativeDynamicInputDescriptor>,
    pub outputs: Vec<NativeOutputDescriptor>,
    pub output_node: bool,
    pub effect: NativeEffectClass,
    pub cache: NativeCachePolicy,
}

impl NativeNodeDescriptor {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.schema_version != NATIVE_NODE_CONTRACT_SCHEMA_VERSION {
            return Err(NativeNodeContractError::UnsupportedContractSchema(
                self.schema_version,
            ));
        }
        validate_identifier("node class type", &self.class_type)?;
        validate_identifier("node implementation version", &self.implementation_version)?;
        if self.inputs.len() > MAX_PORTS
            || self.dynamic_inputs.len() > MAX_PORTS
            || self.outputs.len() > MAX_PORTS
        {
            return Err(NativeNodeContractError::InvalidPortCount);
        }
        let mut input_names = BTreeSet::new();
        for input in &self.inputs {
            input.validate()?;
            if !input_names.insert(input.name.as_str()) {
                return Err(NativeNodeContractError::DuplicatePort(input.name.clone()));
            }
        }
        let mut templates = BTreeSet::new();
        for input in &self.dynamic_inputs {
            input.validate()?;
            if !templates.insert(input.name_template.as_str()) {
                return Err(NativeNodeContractError::DuplicatePort(
                    input.name_template.clone(),
                ));
            }
        }
        let mut output_names = BTreeSet::new();
        for output in &self.outputs {
            output.validate()?;
            if !output_names.insert(output.name.as_str()) {
                return Err(NativeNodeContractError::DuplicatePort(output.name.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeNodePresentation {
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub output_names: Vec<String>,
    pub search_aliases: Vec<String>,
    pub is_deprecated: bool,
    pub is_experimental: bool,
}

impl NativeNodePresentation {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_text(
            "node display name",
            &self.display_name,
            MAX_IDENTIFIER_BYTES,
            false,
        )?;
        validate_text("node category", &self.category, MAX_IDENTIFIER_BYTES, true)?;
        validate_text("node description", &self.description, MAX_TEXT_BYTES, true)?;
        if self.output_names.len() > MAX_PORTS {
            return Err(NativeNodeContractError::InvalidPresentationOutputs);
        }
        let mut output_names = BTreeSet::new();
        for output_name in &self.output_names {
            validate_identifier("node presentation output name", output_name)?;
            if !output_names.insert(output_name.as_str()) {
                return Err(NativeNodeContractError::InvalidPresentationOutputs);
            }
        }
        let mut aliases = BTreeSet::new();
        for alias in &self.search_aliases {
            validate_text("node search alias", alias, MAX_IDENTIFIER_BYTES, false)?;
            if !aliases.insert(alias.as_str()) {
                return Err(NativeNodeContractError::DuplicateSearchAlias(alias.clone()));
            }
        }
        Ok(())
    }
}

pub type NativeStoredObject = Arc<dyn Any + Send + Sync>;

#[derive(Debug, Error)]
pub enum NativeHandleStoreError {
    #[error("native handle operation was cancelled")]
    Cancelled,
    #[error("native handle belongs to a different store")]
    WrongStore,
    #[error("native handle belongs to a different store generation")]
    WrongGeneration,
    #[error("native handle type `{actual}` does not match `{expected}`")]
    WrongType { expected: String, actual: String },
    #[error("native handle `{0}` is absent")]
    Missing(String),
    #[error("native handle digest does not match the stored object")]
    DigestMismatch,
    #[error("native handle store rejected the operation: {0}")]
    Rejected(String),
    #[error("native handle contract is invalid: {0}")]
    InvalidHandle(#[from] NativeNodeContractError),
}

pub trait NativeHandleStore: Send + Sync + fmt::Debug {
    fn identity(&self) -> NativeHandleStoreIdentity;
    fn attempt_id(&self) -> AttemptId;

    fn resolve(
        &self,
        handle: &NativeOpaqueHandle,
        expected_type: &NativeHandleType,
        cancellation: &CancellationToken,
    ) -> Result<NativeStoredObject, NativeHandleStoreError>;

    fn publish(
        &self,
        handle_type: NativeHandleType,
        value: NativeStoredObject,
        digest_sha256: Option<String>,
        resident_bytes: usize,
        cancellation: &CancellationToken,
    ) -> Result<NativeOpaqueHandle, NativeHandleStoreError>;

    fn revoke(
        &self,
        handle: &NativeOpaqueHandle,
        cancellation: &CancellationToken,
    ) -> Result<(), NativeHandleStoreError>;
}

#[derive(Clone, Debug)]
pub struct NativeNodeContext {
    pub prompt_id: PromptId,
    pub attempt_id: AttemptId,
    pub node_id: NodeId,
    pub cancellation: CancellationToken,
    pub scratch: ScratchReservation,
    handle_store: Arc<dyn NativeHandleStore>,
}

impl NativeNodeContext {
    pub fn new(
        prompt_id: PromptId,
        attempt_id: AttemptId,
        node_id: NodeId,
        cancellation: CancellationToken,
        scratch: ScratchReservation,
        handle_store: Arc<dyn NativeHandleStore>,
    ) -> Result<Self, NativeNodeContractError> {
        let context = Self {
            prompt_id,
            attempt_id,
            node_id,
            cancellation,
            scratch,
            handle_store,
        };
        context.validate()?;
        Ok(context)
    }

    pub fn handle_store(&self) -> &dyn NativeHandleStore {
        self.handle_store.as_ref()
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.prompt_id.0.is_nil()
            || self.attempt_id.0.is_nil()
            || self.handle_store.attempt_id() != self.attempt_id
        {
            return Err(NativeNodeContractError::InvalidNodeContext);
        }
        validate_identifier("native node context node ID", &self.node_id.0)?;
        self.handle_store.identity().validate()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCacheDependencies {
    pub artifact_digests: BTreeMap<String, String>,
    pub plugin_digest: Option<String>,
    pub rng_phase: Option<String>,
}

impl NativeCacheDependencies {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        for (identifier, digest) in &self.artifact_digests {
            validate_identifier("cache artifact identifier", identifier)?;
            if !valid_sha256(digest) {
                return Err(NativeNodeContractError::InvalidDigest);
            }
        }
        if let Some(digest) = &self.plugin_digest
            && !valid_sha256(digest)
        {
            return Err(NativeNodeContractError::InvalidDigest);
        }
        if let Some(phase) = &self.rng_phase {
            validate_identifier("cache RNG phase", phase)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePreparedEffectRequest {
    pub transaction_id: Uuid,
    pub metadata: Vec<u8>,
}

impl NativePreparedEffectRequest {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        if self.transaction_id.is_nil() || self.metadata.len() > MAX_EFFECT_METADATA_BYTES {
            return Err(NativeNodeContractError::InvalidEffectRequest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NativeNodeOutcome {
    Values {
        outputs: Vec<NativeValue>,
        ui: Option<Value>,
        effects: Vec<NativePreparedEffectRequest>,
    },
    Blocked {
        reason: String,
    },
    Expansion {
        prompt: ApiPrompt,
        output_node: NodeId,
    },
}

impl NativeNodeOutcome {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        match self {
            Self::Values {
                outputs,
                ui,
                effects,
            } => {
                if outputs.len() > MAX_PORTS || effects.len() > MAX_PORTS {
                    return Err(NativeNodeContractError::InvalidOutcome);
                }
                for output in outputs {
                    output.validate()?;
                }
                if let Some(ui) = ui {
                    let encoded = serde_json::to_vec(ui)
                        .map_err(NativeNodeContractError::EncodePresentationValue)?;
                    if encoded.len() > MAX_UNKNOWN_BYTES {
                        return Err(NativeNodeContractError::PresentationValueTooLarge);
                    }
                }
                for effect in effects {
                    effect.validate()?;
                }
                Ok(())
            }
            Self::Blocked { reason } => {
                validate_text("blocked reason", reason, MAX_TEXT_BYTES, false)
            }
            Self::Expansion {
                prompt,
                output_node,
            } => {
                validate_identifier("expansion output node", &output_node.0)?;
                if prompt.0.is_empty() || !prompt.0.contains_key(output_node) {
                    return Err(NativeNodeContractError::InvalidExpansion);
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeNodeFailureKind {
    Failure,
    Interrupted,
}

#[derive(Clone, Debug, Error, Eq, PartialEq, Serialize, Deserialize)]
#[error("{code}: {message}")]
#[serde(deny_unknown_fields)]
pub struct NativeNodeFailure {
    pub code: String,
    pub message: String,
    pub kind: NativeNodeFailureKind,
    pub retryable: bool,
}

impl NativeNodeFailure {
    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_identifier("node failure code", &self.code)?;
        validate_text("node failure message", &self.message, MAX_TEXT_BYTES, false)
    }
}

pub trait NativeNode: Send + Sync {
    fn class_type(&self) -> &str;
    fn implementation_version(&self) -> &str;

    fn implementation_namespace(&self) -> &str {
        "sim.native_rust"
    }

    fn demanded_lazy_inputs(
        &self,
        _context: &NativeNodeContext,
        _available_inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<BTreeSet<String>, NativeNodeFailure> {
        Ok(BTreeSet::new())
    }

    fn cache_change_token(
        &self,
        _inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<String, NativeNodeFailure> {
        Ok("stable".to_owned())
    }

    fn cache_dependencies(
        &self,
        _context: &NativeNodeContext,
        _inputs: &BTreeMap<String, NativeValue>,
    ) -> Result<NativeCacheDependencies, NativeNodeFailure> {
        Ok(NativeCacheDependencies::default())
    }

    fn execute<'a>(
        &'a self,
        context: NativeNodeContext,
        inputs: BTreeMap<String, NativeValue>,
    ) -> BoxFuture<'a, Result<NativeNodeOutcome, NativeNodeFailure>>;
}

pub type NativeNodeBindingsFactory =
    fn() -> Result<Vec<NativeNodeBinding>, NativeNodeContractError>;

#[derive(Clone)]
pub enum NativeNodeBinding {
    Executable {
        feature_id: String,
        descriptor: NativeNodeDescriptor,
        presentation: NativeNodePresentation,
        node: Arc<dyn NativeNode>,
    },
    ProviderRequired {
        feature_id: String,
        descriptor: NativeNodeDescriptor,
        presentation: NativeNodePresentation,
        provider: String,
        reason: String,
    },
    Unavailable {
        feature_id: String,
        descriptor: NativeNodeDescriptor,
        presentation: NativeNodePresentation,
        reason: String,
    },
}

impl fmt::Debug for NativeNodeBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeNodeBinding")
            .field("disposition", &self.disposition())
            .field("feature_id", &self.feature_id())
            .field("class_type", &self.descriptor().class_type)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeNodeBindingDisposition {
    Executable,
    ProviderRequired,
    Unavailable,
}

impl NativeNodeBinding {
    pub fn feature_id(&self) -> &str {
        match self {
            Self::Executable { feature_id, .. }
            | Self::ProviderRequired { feature_id, .. }
            | Self::Unavailable { feature_id, .. } => feature_id,
        }
    }

    pub fn descriptor(&self) -> &NativeNodeDescriptor {
        match self {
            Self::Executable { descriptor, .. }
            | Self::ProviderRequired { descriptor, .. }
            | Self::Unavailable { descriptor, .. } => descriptor,
        }
    }

    pub fn presentation(&self) -> &NativeNodePresentation {
        match self {
            Self::Executable { presentation, .. }
            | Self::ProviderRequired { presentation, .. }
            | Self::Unavailable { presentation, .. } => presentation,
        }
    }

    pub const fn disposition(&self) -> NativeNodeBindingDisposition {
        match self {
            Self::Executable { .. } => NativeNodeBindingDisposition::Executable,
            Self::ProviderRequired { .. } => NativeNodeBindingDisposition::ProviderRequired,
            Self::Unavailable { .. } => NativeNodeBindingDisposition::Unavailable,
        }
    }

    pub fn validate(&self) -> Result<(), NativeNodeContractError> {
        validate_feature_id(self.feature_id())?;
        self.descriptor().validate()?;
        self.presentation().validate()?;
        if self
            .descriptor()
            .outputs
            .iter()
            .map(|output| output.name.as_str())
            .ne(self.presentation().output_names.iter().map(String::as_str))
        {
            return Err(NativeNodeContractError::InvalidPresentationOutputs);
        }
        match self {
            Self::Executable {
                descriptor, node, ..
            } => {
                if descriptor.class_type != node.class_type()
                    || descriptor.implementation_version != node.implementation_version()
                    || node.implementation_namespace().trim().is_empty()
                {
                    return Err(NativeNodeContractError::BindingImplementationMismatch);
                }
                Ok(())
            }
            Self::ProviderRequired {
                provider, reason, ..
            } => {
                validate_identifier("native provider", provider)?;
                validate_text("provider reason", reason, MAX_TEXT_BYTES, false)
            }
            Self::Unavailable { reason, .. } => {
                validate_text("unavailable reason", reason, MAX_TEXT_BYTES, false)
            }
        }
    }
}

pub fn validate_generated_family_bindings(
    bindings: &[NativeNodeBinding],
    descriptor_ids: &[&str],
) -> Result<(), NativeNodeContractError> {
    let expected = descriptor_ids.iter().copied().collect::<BTreeSet<_>>();
    if expected.len() != descriptor_ids.len() {
        return Err(NativeNodeContractError::DuplicateGeneratedDescriptor);
    }
    let mut actual = BTreeSet::new();
    for binding in bindings {
        binding.validate()?;
        if !actual.insert(binding.descriptor().class_type.as_str()) {
            return Err(NativeNodeContractError::DuplicateGeneratedDescriptor);
        }
    }
    if actual != expected {
        return Err(NativeNodeContractError::GeneratedBindingMismatch);
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum NativeNodeContractError {
    #[error("native node contract schema {0} is unsupported")]
    UnsupportedContractSchema(u16),
    #[error("native opaque handle schema {0} is unsupported")]
    UnsupportedHandleSchema(u16),
    #[error("native node contract {field} is invalid")]
    InvalidText { field: &'static str },
    #[error("native node feature ID is invalid")]
    InvalidFeatureId,
    #[error("native opaque handle generation must be nonzero")]
    InvalidHandleGeneration,
    #[error("native opaque handle type identity is invalid")]
    InvalidHandleType,
    #[error("native opaque handle store identity is nil")]
    InvalidHandleStoreIdentity,
    #[error("native node SHA-256 digest is invalid")]
    InvalidDigest,
    #[error("native node number must be finite")]
    NonFiniteNumber,
    #[error("native node value nesting exceeds its limit")]
    ValueNestingTooDeep,
    #[error("native node list exceeds its value limit")]
    TooManyListValues,
    #[error("native preserved unknown value exceeds its byte limit")]
    PreservedUnknownTooLarge,
    #[error("native preserved unknown value could not be encoded: {0}")]
    EncodePreservedUnknown(serde_json::Error),
    #[error("native UI presentation value could not be encoded: {0}")]
    EncodePresentationValue(serde_json::Error),
    #[error("native UI presentation value exceeds its byte limit")]
    PresentationValueTooLarge,
    #[error("native type union is empty, duplicated, unsorted, or ambiguous")]
    InvalidTypeUnion,
    #[error("native dynamic input descriptor is invalid")]
    InvalidDynamicInput,
    #[error("native node descriptor has an invalid port count")]
    InvalidPortCount,
    #[error("native node descriptor repeats port `{0}`")]
    DuplicatePort(String),
    #[error("native node presentation repeats search alias `{0}`")]
    DuplicateSearchAlias(String),
    #[error("native node presentation output names do not match its descriptor")]
    InvalidPresentationOutputs,
    #[error("native prepared effect request is invalid")]
    InvalidEffectRequest,
    #[error("native node outcome exceeds limits or is malformed")]
    InvalidOutcome,
    #[error("native node expansion is empty or lacks its output node")]
    InvalidExpansion,
    #[error("native node binding does not match its executable implementation")]
    BindingImplementationMismatch,
    #[error("native node context does not match its attempt-local handle store")]
    InvalidNodeContext,
    #[error("generated native node descriptors contain a duplicate")]
    DuplicateGeneratedDescriptor,
    #[error("generated native node bindings do not exactly match descriptor IDs")]
    GeneratedBindingMismatch,
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), NativeNodeContractError> {
    validate_text(field, value, MAX_IDENTIFIER_BYTES, false)
}

fn validate_text(
    field: &'static str,
    value: &str,
    maximum_bytes: usize,
    allow_empty: bool,
) -> Result<(), NativeNodeContractError> {
    if (!allow_empty && value.is_empty())
        || value.len() > maximum_bytes
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(NativeNodeContractError::InvalidText { field });
    }
    Ok(())
}

fn validate_feature_id(value: &str) -> Result<(), NativeNodeContractError> {
    let suffix = value
        .strip_prefix("COMFY-NODE-")
        .ok_or(NativeNodeContractError::InvalidFeatureId)?;
    let digits = suffix.strip_prefix("INACTIVE-").unwrap_or(suffix);
    if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(NativeNodeContractError::InvalidFeatureId);
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use comfy_tensor::CpuWorkspaceAuthority;
    use std::sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    };

    struct TestHandleStore {
        identity: NativeHandleStoreIdentity,
        attempt_id: AttemptId,
        next_identifier: AtomicU64,
        values: Mutex<BTreeMap<String, (NativeStoredObject, Option<String>)>>,
    }

    impl fmt::Debug for TestHandleStore {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter
                .debug_struct("TestHandleStore")
                .field("identity", &self.identity)
                .field("attempt_id", &self.attempt_id)
                .finish_non_exhaustive()
        }
    }

    impl TestHandleStore {
        fn new(identity: NativeHandleStoreIdentity, attempt_id: AttemptId) -> Self {
            Self {
                identity,
                attempt_id,
                next_identifier: AtomicU64::new(1),
                values: Mutex::new(BTreeMap::new()),
            }
        }

        fn object_count(&self) -> Result<usize, NativeHandleStoreError> {
            self.values
                .lock()
                .map(|values| values.len())
                .map_err(|_| NativeHandleStoreError::Rejected("test store is poisoned".to_owned()))
        }

        fn check_handle(
            &self,
            handle: &NativeOpaqueHandle,
            expected_type: &NativeHandleType,
            cancellation: &CancellationToken,
        ) -> Result<(), NativeHandleStoreError> {
            cancellation
                .check()
                .map_err(|_| NativeHandleStoreError::Cancelled)?;
            handle.validate()?;
            if handle.store_identity.store_id != self.identity.store_id {
                return Err(NativeHandleStoreError::WrongStore);
            }
            if handle.store_identity.generation_id != self.identity.generation_id {
                return Err(NativeHandleStoreError::WrongGeneration);
            }
            if handle.handle_type() != expected_type {
                return Err(NativeHandleStoreError::WrongType {
                    expected: expected_type.type_id.clone(),
                    actual: handle.handle_type().type_id.clone(),
                });
            }
            Ok(())
        }
    }

    impl NativeHandleStore for TestHandleStore {
        fn identity(&self) -> NativeHandleStoreIdentity {
            self.identity
        }

        fn attempt_id(&self) -> AttemptId {
            self.attempt_id
        }

        fn resolve(
            &self,
            handle: &NativeOpaqueHandle,
            expected_type: &NativeHandleType,
            cancellation: &CancellationToken,
        ) -> Result<NativeStoredObject, NativeHandleStoreError> {
            self.check_handle(handle, expected_type, cancellation)?;
            let values = self.values.lock().map_err(|_| {
                NativeHandleStoreError::Rejected("test store is poisoned".to_owned())
            })?;
            let (value, digest) = values
                .get(handle.identifier())
                .ok_or_else(|| NativeHandleStoreError::Missing(handle.identifier().to_owned()))?;
            if digest.as_deref() != handle.digest_sha256() {
                return Err(NativeHandleStoreError::DigestMismatch);
            }
            Ok(value.clone())
        }

        fn publish(
            &self,
            handle_type: NativeHandleType,
            value: NativeStoredObject,
            digest_sha256: Option<String>,
            _resident_bytes: usize,
            cancellation: &CancellationToken,
        ) -> Result<NativeOpaqueHandle, NativeHandleStoreError> {
            cancellation
                .check()
                .map_err(|_| NativeHandleStoreError::Cancelled)?;
            handle_type.validate()?;
            let generation = self.next_identifier.fetch_add(1, Ordering::AcqRel);
            let identifier = format!("handle-{generation}");
            let handle = NativeOpaqueHandle::new(
                handle_type,
                self.identity,
                identifier.clone(),
                generation,
                digest_sha256.clone(),
            )?;
            self.values
                .lock()
                .map_err(|_| NativeHandleStoreError::Rejected("test store is poisoned".to_owned()))?
                .insert(identifier, (value, digest_sha256));
            Ok(handle)
        }

        fn revoke(
            &self,
            handle: &NativeOpaqueHandle,
            cancellation: &CancellationToken,
        ) -> Result<(), NativeHandleStoreError> {
            self.check_handle(handle, handle.handle_type(), cancellation)?;
            let removed = self
                .values
                .lock()
                .map_err(|_| NativeHandleStoreError::Rejected("test store is poisoned".to_owned()))?
                .remove(handle.identifier());
            if removed.is_none() {
                return Err(NativeHandleStoreError::Missing(
                    handle.identifier().to_owned(),
                ));
            }
            Ok(())
        }
    }

    fn model_type() -> Result<NativeHandleType, NativeNodeContractError> {
        NativeHandleType::new(NativeHandleKind::Model, "MODEL")
    }

    fn store_identity(
        store_id: u128,
        generation_id: u128,
    ) -> Result<NativeHandleStoreIdentity, NativeNodeContractError> {
        NativeHandleStoreIdentity::new(Uuid::from_u128(store_id), Uuid::from_u128(generation_id))
    }

    struct IdentityNode;

    impl NativeNode for IdentityNode {
        fn class_type(&self) -> &str {
            "IdentityModel"
        }

        fn implementation_version(&self) -> &str {
            "1"
        }

        fn execute<'a>(
            &'a self,
            context: NativeNodeContext,
            mut inputs: BTreeMap<String, NativeValue>,
        ) -> BoxFuture<'a, Result<NativeNodeOutcome, NativeNodeFailure>> {
            Box::pin(async move {
                context
                    .cancellation
                    .check()
                    .map_err(|_| NativeNodeFailure {
                        code: "execution_interrupted".to_owned(),
                        message: "native node execution was interrupted".to_owned(),
                        kind: NativeNodeFailureKind::Interrupted,
                        retryable: true,
                    })?;
                let output = inputs.remove("model").ok_or_else(|| NativeNodeFailure {
                    code: "missing_input".to_owned(),
                    message: "required model input is missing".to_owned(),
                    kind: NativeNodeFailureKind::Failure,
                    retryable: false,
                })?;
                Ok(NativeNodeOutcome::Values {
                    outputs: vec![output],
                    ui: None,
                    effects: Vec::new(),
                })
            })
        }
    }

    fn identity_descriptor() -> Result<NativeNodeDescriptor, NativeNodeContractError> {
        Ok(NativeNodeDescriptor {
            schema_version: NATIVE_NODE_CONTRACT_SCHEMA_VERSION,
            class_type: "IdentityModel".to_owned(),
            implementation_version: "1".to_owned(),
            inputs: vec![NativeInputDescriptor {
                name: "model".to_owned(),
                accepted_types: NativeTypeUnion::new([NativeValueType::Handle(model_type()?)])?,
                required: true,
                hidden: false,
                lazy: false,
                cardinality: NativePortCardinality::Scalar,
                allows_literal: false,
            }],
            dynamic_inputs: Vec::new(),
            outputs: vec![NativeOutputDescriptor {
                name: "model".to_owned(),
                produced_type: NativeValueType::Handle(model_type()?),
                is_list: false,
            }],
            output_node: false,
            effect: NativeEffectClass::Pure,
            cache: NativeCachePolicy::InputIdentity,
        })
    }

    fn identity_binding() -> Result<NativeNodeBinding, NativeNodeContractError> {
        Ok(NativeNodeBinding::Executable {
            feature_id: "COMFY-NODE-0001".to_owned(),
            descriptor: identity_descriptor()?,
            presentation: NativeNodePresentation {
                display_name: "Identity Model".to_owned(),
                category: String::new(),
                description: "Passes one opaque model handle through unchanged.".to_owned(),
                output_names: vec!["model".to_owned()],
                search_aliases: vec!["model identity".to_owned()],
                is_deprecated: true,
                is_experimental: false,
            },
            node: Arc::new(IdentityNode),
        })
    }

    #[test]
    fn typed_values_cover_handles_lists_and_preserved_unknowns()
    -> Result<(), Box<dyn std::error::Error>> {
        let model_type = model_type()?;
        let model = NativeValue::Handle {
            value: NativeOpaqueHandle::new(
                model_type.clone(),
                store_identity(1, 2)?,
                "model-1",
                1,
                Some("a".repeat(64)),
            )?,
        };
        let value = NativeValue::List {
            values: vec![
                model.clone(),
                NativeValue::Primitive {
                    value: NativePrimitive::Integer(7),
                },
                NativeValue::PreservedUnknown {
                    type_name: "future.socket@2".to_owned(),
                    value: serde_json::json!({"future": true}),
                },
            ],
        };
        value.validate()?;
        assert!(NativeTypeUnion::new([NativeValueType::Handle(model_type)])?.accepts(&model));
        let media_union = NativeTypeUnion::new(
            ["FILE_3D", "KSPLAT", "PLY", "SPLAT", "SPZ"]
                .into_iter()
                .map(|type_id| {
                    NativeHandleType::new(NativeHandleKind::ThreeD, type_id)
                        .map(NativeValueType::Handle)
                })
                .collect::<Result<Vec<_>, _>>()?,
        )?;
        assert_eq!(media_union.members().len(), 5);
        assert_eq!(
            serde_json::from_slice::<NativeValue>(&serde_json::to_vec(&value)?)?,
            value
        );
        Ok(())
    }

    #[test]
    fn integer_primitives_preserve_full_signed_and_unsigned_ranges()
    -> Result<(), Box<dyn std::error::Error>> {
        for value in [
            NativeValue::Primitive {
                value: NativePrimitive::Integer(i64::MIN),
            },
            NativeValue::Primitive {
                value: NativePrimitive::UnsignedInteger(u64::MAX),
            },
        ] {
            assert_eq!(
                serde_json::from_slice::<NativeValue>(&serde_json::to_vec(&value)?)?,
                value
            );
            assert!(matches!(
                &value,
                NativeValue::Primitive { value }
                    if value.primitive_type() == NativePrimitiveType::Integer
            ));
        }
        Ok(())
    }

    #[test]
    fn descriptors_and_bindings_reject_ambiguous_or_mismatched_contracts()
    -> Result<(), Box<dyn std::error::Error>> {
        identity_binding()?.validate()?;
        assert!(
            NativeTypeUnion::new([NativeValueType::Handle(model_type()?), NativeValueType::Any,])
                .is_err()
        );
        let mut descriptor = identity_descriptor()?;
        descriptor.implementation_version = "2".to_owned();
        let binding = NativeNodeBinding::Executable {
            feature_id: "COMFY-NODE-0001".to_owned(),
            descriptor,
            presentation: identity_binding()?.presentation().clone(),
            node: Arc::new(IdentityNode),
        };
        assert!(matches!(
            binding.validate(),
            Err(NativeNodeContractError::BindingImplementationMismatch)
        ));
        let mut binding = identity_binding()?;
        if let NativeNodeBinding::Executable { presentation, .. } = &mut binding {
            presentation.output_names = vec!["wrong".to_owned()];
        }
        assert!(matches!(
            binding.validate(),
            Err(NativeNodeContractError::InvalidPresentationOutputs)
        ));
        Ok(())
    }

    #[test]
    fn portable_execution_checks_cancellation_and_preserves_handle_identity()
    -> Result<(), Box<dyn std::error::Error>> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let scratch = authority.authorize_workspace(1024)?;
        let prompt_id = PromptId(Uuid::from_u128(10));
        let attempt_id = AttemptId(Uuid::from_u128(11));
        let handle_store = Arc::new(TestHandleStore::new(store_identity(12, 13)?, attempt_id));
        let mismatched_store = Arc::new(TestHandleStore::new(
            store_identity(12, 13)?,
            AttemptId(Uuid::from_u128(14)),
        ));
        assert!(matches!(
            NativeNodeContext::new(
                prompt_id,
                attempt_id,
                NodeId::from("identity"),
                CancellationToken::default(),
                authority.authorize_workspace(1024)?,
                mismatched_store,
            ),
            Err(NativeNodeContractError::InvalidNodeContext)
        ));
        let handle = handle_store.publish(
            model_type()?,
            Arc::new("model state".to_owned()),
            Some("a".repeat(64)),
            "model state".len(),
            &CancellationToken::default(),
        )?;
        let model = NativeValue::Handle { value: handle };
        let context = NativeNodeContext::new(
            prompt_id,
            attempt_id,
            NodeId::from("identity"),
            CancellationToken::default(),
            scratch,
            handle_store.clone(),
        )?;
        let outcome = futures::executor::block_on(IdentityNode.execute(
            context,
            BTreeMap::from([("model".to_owned(), model.clone())]),
        ))?;
        assert_eq!(
            outcome,
            NativeNodeOutcome::Values {
                outputs: vec![model.clone()],
                ui: None,
                effects: Vec::new(),
            }
        );
        outcome.validate()?;

        let cancellation = CancellationToken::default();
        assert!(cancellation.cancel());
        let interrupted = futures::executor::block_on(IdentityNode.execute(
            NativeNodeContext::new(
                prompt_id,
                attempt_id,
                NodeId::from("identity"),
                cancellation,
                authority.authorize_workspace(1024)?,
                handle_store.clone(),
            )?,
            BTreeMap::from([("model".to_owned(), model)]),
        ))
        .expect_err("pre-cancelled execution must not publish an output");
        assert_eq!(interrupted.kind, NativeNodeFailureKind::Interrupted);
        assert_eq!(handle_store.object_count()?, 1);
        drop(backend);
        Ok(())
    }

    #[test]
    fn attempt_local_handle_store_rejects_foreign_identity_type_and_cancellation()
    -> Result<(), Box<dyn std::error::Error>> {
        let attempt_id = AttemptId(Uuid::from_u128(20));
        assert!(matches!(
            NativeHandleStoreIdentity::new(Uuid::nil(), Uuid::from_u128(22)),
            Err(NativeNodeContractError::InvalidHandleStoreIdentity)
        ));
        let identity = store_identity(21, 22)?;
        let store = TestHandleStore::new(identity, attempt_id);
        let model_type = model_type()?;
        let handle = store.publish(
            model_type.clone(),
            Arc::new("model state".to_owned()),
            Some("b".repeat(64)),
            "model state".len(),
            &CancellationToken::default(),
        )?;
        let resolved = store.resolve(&handle, &model_type, &CancellationToken::default())?;
        assert_eq!(
            resolved.downcast_ref::<String>().map(String::as_str),
            Some("model state")
        );

        let wrong_store = NativeOpaqueHandle::new(
            model_type.clone(),
            store_identity(23, 22)?,
            handle.identifier(),
            handle.generation(),
            None,
        )?;
        assert!(matches!(
            store.resolve(&wrong_store, &model_type, &CancellationToken::default()),
            Err(NativeHandleStoreError::WrongStore)
        ));
        let wrong_generation = NativeOpaqueHandle::new(
            model_type.clone(),
            store_identity(21, 24)?,
            handle.identifier(),
            handle.generation(),
            None,
        )?;
        assert!(matches!(
            store.resolve(
                &wrong_generation,
                &model_type,
                &CancellationToken::default()
            ),
            Err(NativeHandleStoreError::WrongGeneration)
        ));
        let forged_digest = NativeOpaqueHandle::new(
            model_type.clone(),
            identity,
            handle.identifier(),
            handle.generation(),
            Some("c".repeat(64)),
        )?;
        assert!(matches!(
            store.resolve(&forged_digest, &model_type, &CancellationToken::default()),
            Err(NativeHandleStoreError::DigestMismatch)
        ));
        let image_type = NativeHandleType::new(NativeHandleKind::Image, "IMAGE")?;
        assert!(matches!(
            store.resolve(&handle, &image_type, &CancellationToken::default()),
            Err(NativeHandleStoreError::WrongType { .. })
        ));

        let cancellation = CancellationToken::default();
        assert!(cancellation.cancel());
        let before = store.object_count()?;
        assert!(matches!(
            store.publish(image_type, Arc::new(vec![0_u8; 4]), None, 4, &cancellation,),
            Err(NativeHandleStoreError::Cancelled)
        ));
        assert_eq!(store.object_count()?, before);
        Ok(())
    }

    #[test]
    fn generated_binding_validation_is_exact_and_collision_free()
    -> Result<(), Box<dyn std::error::Error>> {
        let binding = identity_binding()?;
        validate_generated_family_bindings(std::slice::from_ref(&binding), &["IdentityModel"])?;
        assert!(matches!(
            validate_generated_family_bindings(&[binding.clone(), binding], &["IdentityModel"]),
            Err(NativeNodeContractError::DuplicateGeneratedDescriptor)
        ));
        assert!(matches!(
            validate_generated_family_bindings(&[], &["IdentityModel"]),
            Err(NativeNodeContractError::GeneratedBindingMismatch)
        ));
        Ok(())
    }
}
