use crate::{LatentFormatIdentity, ModelFamilyIdentity};
use comfy_tensor::{
    CpuBackend, DType, DeviceId, ExecutionContext, ResizeMode, Tensor, TensorDescriptor,
    TensorError,
    generated_comfy_operator_indirection_01::{
        OperatorIndirectionError, cast_to_with_context_exact_native,
    },
    generated_external_tensor_kernel_01::{
        ExternalTensorKernelPartOneError, resize_with_context_exact_native,
    },
    generated_indexing_masking_01::{IndexingMaskingPartOneError, narrow_method_exact_native},
    generated_shape_layout_transform_01::{
        ShapeLayoutTransformPartOneError, tensor_unsqueeze_exact_native,
    },
    generated_shape_layout_transform_02::{
        ShapeLayoutTransformPartTwoError, tensor_repeat_with_context_exact_native,
        torch_cat_with_context_exact_native,
    },
};
use comfy_types::CancellationError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const CONDITIONING_SCHEMA_VERSION: u16 = 1;
const CROSS_ATTENTION_MAX_REPEAT_RATIO: u64 = 4;
const MAX_CONDITIONING_CONSTANT_DEPTH: usize = 64;
const MAX_CONDITIONING_CONSTANT_ITEMS: usize = 4_096;
const MAX_CONDITIONING_CONSTANT_BYTES: usize = 16 * 1_048_576;
const MAX_CONDITIONING_LIST_ITEMS: usize = 1_024;
const MAX_CONDITIONING_ENTRIES: usize = 4_096;
const MAX_CONDITIONING_REGION_RANK: usize = 8;
const MAX_CONDITIONING_HOOK_REFERENCES: usize = 64;
const SOURCE_AREA_FEATHER: u64 = 8;

#[derive(Debug, Error)]
pub enum ConditioningError {
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    Narrow(#[from] IndexingMaskingPartOneError),
    #[error(transparent)]
    ShapeView(#[from] ShapeLayoutTransformPartOneError),
    #[error(transparent)]
    ShapeOperation(#[from] ShapeLayoutTransformPartTwoError),
    #[error(transparent)]
    TensorCast(#[from] OperatorIndirectionError),
    #[error(transparent)]
    TensorResize(#[from] ExternalTensorKernelPartOneError),
    #[error("conditioning operation was cancelled")]
    Cancelled,
    #[error("conditioning input is invalid: {0}")]
    Invalid(String),
    #[error("conditioning shape arithmetic overflowed while computing {0}")]
    ShapeOverflow(&'static str),
    #[error("conditioning resident byte accounting overflowed")]
    ResidentBytesOverflow,
    #[error("conditioning tensor content cannot be read on device {0:?}")]
    UnreadableDevice(DeviceId),
    #[error("conditioning descriptor encoding failed: {0}")]
    Encoding(String),
}

impl From<CancellationError> for ConditioningError {
    fn from(_: CancellationError) -> Self {
        Self::Cancelled
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ConditioningIdentity {
    namespace: String,
    model_family: ModelFamilyIdentity,
    latent_format: LatentFormatIdentity,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConditioningIdentityWire {
    schema_version: u16,
    namespace: String,
    model_family: ModelFamilyIdentity,
    latent_format: LatentFormatIdentity,
}

impl ConditioningIdentity {
    pub fn new(
        namespace: impl Into<String>,
        model_family: ModelFamilyIdentity,
        latent_format: LatentFormatIdentity,
    ) -> Result<Self, ConditioningError> {
        let namespace = namespace.into();
        validate_identifier("conditioning namespace", &namespace)?;
        Ok(Self {
            namespace,
            model_family,
            latent_format,
        })
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn model_family(&self) -> &ModelFamilyIdentity {
        &self.model_family
    }

    pub fn latent_format(&self) -> &LatentFormatIdentity {
        &self.latent_format
    }

    pub fn digest(&self) -> Result<String, ConditioningError> {
        let mut hasher = Sha256::new();
        hasher.update(b"sim.comfy.conditioning-identity.v1");
        hasher.update(CONDITIONING_SCHEMA_VERSION.to_le_bytes());
        hash_string(&mut hasher, &self.namespace)?;
        hash_string(&mut hasher, self.model_family.feature_id())?;
        hash_string(&mut hasher, self.model_family.identifier())?;
        hash_string(&mut hasher, self.model_family.architecture_version())?;
        hash_string(&mut hasher, self.latent_format.feature_id())?;
        hash_string(&mut hasher, self.latent_format.identifier())?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn resident_bytes(&self) -> Result<u64, ConditioningError> {
        let mut bytes = u64::try_from(std::mem::size_of::<Self>())
            .map_err(|_| ConditioningError::ResidentBytesOverflow)?;
        bytes = bytes
            .checked_add(
                u64::try_from(self.namespace.capacity())
                    .map_err(|_| ConditioningError::ResidentBytesOverflow)?,
            )
            .and_then(|bytes| bytes.checked_add(self.model_family.owned_resident_bytes()?))
            .and_then(|bytes| bytes.checked_add(self.latent_format.owned_resident_bytes()?))
            .ok_or(ConditioningError::ResidentBytesOverflow)?;
        Ok(bytes)
    }
}

impl Serialize for ConditioningIdentity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ConditioningIdentityWire {
            schema_version: CONDITIONING_SCHEMA_VERSION,
            namespace: self.namespace.clone(),
            model_family: self.model_family.clone(),
            latent_format: self.latent_format.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ConditioningIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ConditioningIdentityWire::deserialize(deserializer)?;
        if wire.schema_version != CONDITIONING_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported conditioning identity schema version: {}",
                wire.schema_version
            )));
        }
        Self::new(wire.namespace, wire.model_family, wire.latent_format)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ConditioningControlReference(String);

impl ConditioningControlReference {
    pub fn checked(identifier: impl Into<String>) -> Result<Self, ConditioningError> {
        let identifier = identifier.into();
        validate_identifier("conditioning control reference", &identifier)?;
        Ok(Self(identifier))
    }

    pub fn identifier(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ConditioningHookReference(String);

impl ConditioningHookReference {
    pub fn checked(identifier: impl Into<String>) -> Result<Self, ConditioningError> {
        let identifier = identifier.into();
        validate_identifier("conditioning hook reference", &identifier)?;
        Ok(Self(identifier))
    }

    pub fn identifier(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConditioningReferences {
    control: Option<ConditioningControlReference>,
    hooks: Vec<ConditioningHookReference>,
}

impl ConditioningReferences {
    pub fn checked(
        control: Option<ConditioningControlReference>,
        hooks: Vec<ConditioningHookReference>,
    ) -> Result<Self, ConditioningError> {
        if hooks.len() > MAX_CONDITIONING_HOOK_REFERENCES {
            return Err(ConditioningError::Invalid(format!(
                "conditioning entry contains more than {MAX_CONDITIONING_HOOK_REFERENCES} hook references"
            )));
        }
        let mut identifiers = BTreeSet::new();
        for hook in &hooks {
            validate_identifier("conditioning hook reference", hook.identifier())?;
            if !identifiers.insert(hook.identifier()) {
                return Err(ConditioningError::Invalid(format!(
                    "conditioning hook reference is duplicated: {}",
                    hook.identifier()
                )));
            }
        }
        if let Some(control) = &control {
            validate_identifier("conditioning control reference", control.identifier())?;
        }
        Ok(Self { control, hooks })
    }

    pub fn control(&self) -> Option<&ConditioningControlReference> {
        self.control.as_ref()
    }

    pub fn hooks(&self) -> &[ConditioningHookReference] {
        &self.hooks
    }
}

#[derive(Clone, Debug)]
pub enum ConditioningConstant {
    Null,
    Boolean(bool),
    Signed(i64),
    Unsigned(u64),
    FiniteF64Bits(u64),
    Text(String),
    Bytes(Vec<u8>),
    Tensor(Tensor),
    List(Vec<ConditioningConstant>),
    Tuple(Vec<ConditioningConstant>),
    Map(BTreeMap<String, ConditioningConstant>),
}

impl PartialEq for ConditioningConstant {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Signed(left), Self::Signed(right)) => left == right,
            (Self::Unsigned(left), Self::Unsigned(right)) => left == right,
            (Self::FiniteF64Bits(left), Self::FiniteF64Bits(right)) => left == right,
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Bytes(left), Self::Bytes(right)) => left == right,
            (Self::Tensor(left), Self::Tensor(right)) => tensor_constants_equal(left, right),
            (Self::List(left), Self::List(right)) | (Self::Tuple(left), Self::Tuple(right)) => {
                left == right
            }
            (Self::Map(left), Self::Map(right)) => left == right,
            _ => false,
        }
    }
}

impl ConditioningConstant {
    pub fn finite_f64(value: f64) -> Result<Self, ConditioningError> {
        if !value.is_finite() {
            return Err(ConditioningError::Invalid(
                "constant floating-point value must be finite".to_owned(),
            ));
        }
        Ok(Self::FiniteF64Bits(value.to_bits()))
    }

    fn validate(&self) -> Result<(), ConditioningError> {
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(MAX_CONDITIONING_CONSTANT_DEPTH)
            .map_err(|_| ConditioningError::ShapeOverflow("conditioning constant validation"))?;
        pending.push((self, 1_usize));
        let mut item_count = 0_usize;
        let mut byte_count = 0_usize;
        while let Some((value, depth)) = pending.pop() {
            if depth > MAX_CONDITIONING_CONSTANT_DEPTH {
                return Err(ConditioningError::Invalid(format!(
                    "conditioning constant nesting exceeds {MAX_CONDITIONING_CONSTANT_DEPTH}"
                )));
            }
            item_count = item_count
                .checked_add(1)
                .ok_or(ConditioningError::ShapeOverflow(
                    "conditioning constant items",
                ))?;
            if item_count > MAX_CONDITIONING_CONSTANT_ITEMS {
                return Err(ConditioningError::Invalid(format!(
                    "conditioning constant contains more than {MAX_CONDITIONING_CONSTANT_ITEMS} items"
                )));
            }
            match value {
                Self::FiniteF64Bits(bits) if !f64::from_bits(*bits).is_finite() => {
                    return Err(ConditioningError::Invalid(
                        "constant floating-point value must be finite".to_owned(),
                    ));
                }
                Self::Text(value) => {
                    byte_count = byte_count.checked_add(value.len()).ok_or(
                        ConditioningError::ShapeOverflow("conditioning constant bytes"),
                    )?;
                }
                Self::Bytes(value) => {
                    byte_count = byte_count.checked_add(value.len()).ok_or(
                        ConditioningError::ShapeOverflow("conditioning constant bytes"),
                    )?;
                }
                Self::Tensor(tensor) => {
                    validate_constant_tensor(tensor)?;
                    byte_count = byte_count
                        .checked_add(usize::try_from(tensor.storage_byte_len()).map_err(|_| {
                            ConditioningError::ShapeOverflow("conditioning constant bytes")
                        })?)
                        .ok_or(ConditioningError::ShapeOverflow(
                            "conditioning constant bytes",
                        ))?;
                }
                Self::List(values) | Self::Tuple(values) => {
                    if values.len() > MAX_CONDITIONING_CONSTANT_ITEMS {
                        return Err(ConditioningError::Invalid(
                            "conditioning constant list exceeds the item limit".to_owned(),
                        ));
                    }
                    pending.try_reserve(values.len()).map_err(|_| {
                        ConditioningError::ShapeOverflow("conditioning constant validation")
                    })?;
                    for child in values.iter().rev() {
                        pending.push((child, depth.saturating_add(1)));
                    }
                }
                Self::Map(values) => {
                    if values.len() > MAX_CONDITIONING_CONSTANT_ITEMS {
                        return Err(ConditioningError::Invalid(
                            "conditioning constant map exceeds the item limit".to_owned(),
                        ));
                    }
                    pending.try_reserve(values.len()).map_err(|_| {
                        ConditioningError::ShapeOverflow("conditioning constant validation")
                    })?;
                    for (key, child) in values.iter().rev() {
                        byte_count = byte_count.checked_add(key.len()).ok_or(
                            ConditioningError::ShapeOverflow("conditioning constant bytes"),
                        )?;
                        pending.push((child, depth.saturating_add(1)));
                    }
                }
                _ => {}
            }
            if byte_count > MAX_CONDITIONING_CONSTANT_BYTES {
                return Err(ConditioningError::Invalid(format!(
                    "conditioning constant payload exceeds {MAX_CONDITIONING_CONSTANT_BYTES} bytes"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum ConditioningValue {
    Regular(Tensor),
    NoiseShape(Tensor),
    CrossAttention(Tensor),
    Constant(ConditioningConstant),
    List(Vec<Tensor>),
}

impl ConditioningValue {
    pub fn regular(tensor: Tensor) -> Result<Self, ConditioningError> {
        validate_batched_tensor(&tensor, "regular conditioning")?;
        Ok(Self::Regular(tensor))
    }

    pub fn noise_shape(tensor: Tensor) -> Result<Self, ConditioningError> {
        validate_batched_tensor(&tensor, "noise-shape conditioning")?;
        if tensor.descriptor().rank() < 3 {
            return Err(ConditioningError::Invalid(
                "noise-shape conditioning must have batch, channel, and spatial dimensions"
                    .to_owned(),
            ));
        }
        Ok(Self::NoiseShape(tensor))
    }

    pub fn cross_attention(tensor: Tensor) -> Result<Self, ConditioningError> {
        validate_batched_tensor(&tensor, "cross-attention conditioning")?;
        if tensor.descriptor().rank() != 3 {
            return Err(ConditioningError::Invalid(
                "cross-attention conditioning must have [batch, tokens, channels] shape".to_owned(),
            ));
        }
        if tensor.descriptor().shape().get(1).copied() == Some(0) {
            return Err(ConditioningError::Invalid(
                "cross-attention token count must be nonzero".to_owned(),
            ));
        }
        Ok(Self::CrossAttention(tensor))
    }

    pub fn constant(value: ConditioningConstant) -> Result<Self, ConditioningError> {
        value.validate()?;
        Ok(Self::Constant(value))
    }

    pub fn list(tensors: Vec<Tensor>) -> Result<Self, ConditioningError> {
        if tensors.is_empty() {
            return Err(ConditioningError::Invalid(
                "conditioning tensor list must be nonempty".to_owned(),
            ));
        }
        if tensors.len() > MAX_CONDITIONING_LIST_ITEMS {
            return Err(ConditioningError::Invalid(format!(
                "conditioning tensor list contains more than {MAX_CONDITIONING_LIST_ITEMS} items"
            )));
        }
        for tensor in &tensors {
            validate_batched_tensor(tensor, "conditioning list item")?;
        }
        Ok(Self::List(tensors))
    }

    pub fn size(&self) -> Result<Vec<u64>, ConditioningError> {
        self.validate()?;
        match self {
            Self::Regular(tensor) | Self::NoiseShape(tensor) | Self::CrossAttention(tensor) => {
                Ok(tensor.descriptor().shape().to_vec())
            }
            Self::Constant(_) => Ok(vec![1]),
            Self::List(tensors) => {
                let mut elements = 0_u64;
                let mut token_count = 1_u64;
                for tensor in tensors {
                    elements = elements
                        .checked_add(tensor.descriptor().element_count()?)
                        .ok_or(ConditioningError::ShapeOverflow("conditioning list size"))?;
                    if let Some(tokens) = tensor.descriptor().shape().get(1) {
                        token_count = *tokens;
                    }
                }
                if token_count == 0 || !elements.is_multiple_of(token_count) {
                    return Err(ConditioningError::Invalid(
                        "conditioning list size is not token aligned".to_owned(),
                    ));
                }
                Ok(vec![1, token_count, elements / token_count])
            }
        }
    }

    pub fn process(
        &self,
        batch_size: u64,
        region: Option<&ResolvedConditioningRegion>,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<Self, ConditioningError> {
        context.cancellation.check()?;
        self.validate()?;
        if batch_size == 0 {
            return Err(ConditioningError::Invalid(
                "conditioning batch size must be nonzero".to_owned(),
            ));
        }
        let output = match self {
            Self::Regular(tensor) => {
                Self::Regular(repeat_to_batch_size(tensor, batch_size, backend, context)?)
            }
            Self::NoiseShape(tensor) => {
                let tensor = match region {
                    Some(region) => narrow_spatial(tensor, region, context)?,
                    None => tensor.clone(),
                };
                Self::NoiseShape(repeat_to_batch_size(&tensor, batch_size, backend, context)?)
            }
            Self::CrossAttention(tensor) => {
                Self::CrossAttention(repeat_to_batch_size(tensor, batch_size, backend, context)?)
            }
            Self::Constant(value) => Self::Constant(value.clone()),
            Self::List(tensors) => {
                let mut output = Vec::new();
                output
                    .try_reserve_exact(tensors.len())
                    .map_err(|_| ConditioningError::ShapeOverflow("conditioning list output"))?;
                for tensor in tensors {
                    context.cancellation.check()?;
                    output.push(repeat_to_batch_size(tensor, batch_size, backend, context)?);
                }
                Self::List(output)
            }
        };
        context.cancellation.check()?;
        Ok(output)
    }

    pub fn can_concat(&self, other: &Self) -> bool {
        if self.validate().is_err() || other.validate().is_err() {
            return false;
        }
        match (self, other) {
            (Self::Regular(left), Self::Regular(right))
            | (Self::NoiseShape(left), Self::NoiseShape(right)) => {
                tensor_concat_compatible(left, right, None)
            }
            (Self::CrossAttention(left), Self::CrossAttention(right)) => {
                cross_attention_pair_compatible(left, right)
            }
            (Self::Constant(left), Self::Constant(right)) => left == right,
            (Self::List(left), Self::List(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| tensor_concat_compatible(left, right, None))
            }
            _ => false,
        }
    }

    pub fn concat(
        &self,
        others: &[Self],
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<Self, ConditioningError> {
        context.cancellation.check()?;
        self.validate()?;
        for other in others {
            other.validate()?;
        }
        match self {
            Self::Regular(first) => Ok(Self::Regular(concat_regular(
                first,
                others,
                |value| match value {
                    Self::Regular(tensor) => Some(tensor),
                    _ => None,
                },
                backend,
                context,
            )?)),
            Self::NoiseShape(first) => Ok(Self::NoiseShape(concat_regular(
                first,
                others,
                |value| match value {
                    Self::NoiseShape(tensor) => Some(tensor),
                    _ => None,
                },
                backend,
                context,
            )?)),
            Self::CrossAttention(first) => Ok(Self::CrossAttention(concat_cross_attention(
                first, others, backend, context,
            )?)),
            Self::Constant(value) => {
                if others
                    .iter()
                    .all(|other| matches!(other, Self::Constant(other) if other == value))
                {
                    Ok(Self::Constant(value.clone()))
                } else {
                    Err(ConditioningError::Invalid(
                        "constant conditioning values must be equal to concatenate".to_owned(),
                    ))
                }
            }
            Self::List(first) => concat_lists(first, others, backend, context),
        }
    }

    pub fn deterministic_digest(
        &self,
        cancellation: &comfy_types::CancellationToken,
    ) -> Result<String, ConditioningError> {
        cancellation.check()?;
        self.validate()?;
        let mut hasher = Sha256::new();
        match self {
            Self::Regular(tensor) => {
                hasher.update([1]);
                hash_tensor(&mut hasher, tensor, cancellation)?;
            }
            Self::NoiseShape(tensor) => {
                hasher.update([2]);
                hash_tensor(&mut hasher, tensor, cancellation)?;
            }
            Self::CrossAttention(tensor) => {
                hasher.update([3]);
                hash_tensor(&mut hasher, tensor, cancellation)?;
            }
            Self::Constant(value) => {
                hasher.update([4]);
                hash_conditioning_constant(&mut hasher, value, cancellation)?;
            }
            Self::List(tensors) => {
                hasher.update([5]);
                hash_u64(&mut hasher, tensors.len())?;
                for tensor in tensors {
                    hash_tensor(&mut hasher, tensor, cancellation)?;
                }
            }
        }
        cancellation.check()?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn validate(&self) -> Result<(), ConditioningError> {
        match self {
            Self::Regular(tensor) => validate_batched_tensor(tensor, "regular conditioning"),
            Self::NoiseShape(tensor) => {
                validate_batched_tensor(tensor, "noise-shape conditioning")?;
                if tensor.descriptor().rank() < 3 {
                    return Err(ConditioningError::Invalid(
                        "noise-shape conditioning must have batch, channel, and spatial dimensions"
                            .to_owned(),
                    ));
                }
                Ok(())
            }
            Self::CrossAttention(tensor) => {
                validate_batched_tensor(tensor, "cross-attention conditioning")?;
                if tensor.descriptor().rank() != 3
                    || tensor.descriptor().shape().get(1).copied() == Some(0)
                {
                    return Err(ConditioningError::Invalid(
                        "cross-attention conditioning must have nonzero [batch, tokens, channels] shape"
                            .to_owned(),
                    ));
                }
                Ok(())
            }
            Self::Constant(value) => value.validate(),
            Self::List(tensors) => {
                if tensors.is_empty() {
                    return Err(ConditioningError::Invalid(
                        "conditioning tensor list must be nonempty".to_owned(),
                    ));
                }
                if tensors.len() > MAX_CONDITIONING_LIST_ITEMS {
                    return Err(ConditioningError::Invalid(format!(
                        "conditioning tensor list contains more than {MAX_CONDITIONING_LIST_ITEMS} items"
                    )));
                }
                for tensor in tensors {
                    validate_batched_tensor(tensor, "conditioning list item")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConditioningRegion {
    Absolute { sizes: Vec<u64>, offsets: Vec<u64> },
    Percentage { sizes: Vec<f64>, offsets: Vec<f64> },
}

impl ConditioningRegion {
    pub fn absolute(sizes: Vec<u64>, offsets: Vec<u64>) -> Result<Self, ConditioningError> {
        validate_region_rank(sizes.len(), offsets.len())?;
        if sizes.contains(&0) {
            return Err(ConditioningError::Invalid(
                "absolute conditioning region sizes must be nonzero".to_owned(),
            ));
        }
        Ok(Self::Absolute { sizes, offsets })
    }

    pub fn percentage(sizes: Vec<f64>, offsets: Vec<f64>) -> Result<Self, ConditioningError> {
        validate_region_rank(sizes.len(), offsets.len())?;
        for size in &sizes {
            if !size.is_finite() || *size <= 0.0 || *size > 1.0 {
                return Err(ConditioningError::Invalid(
                    "percentage conditioning region sizes must be finite and in (0, 1]".to_owned(),
                ));
            }
        }
        for offset in &offsets {
            if !offset.is_finite() || *offset < 0.0 || *offset >= 1.0 {
                return Err(ConditioningError::Invalid(
                    "percentage conditioning region offsets must be finite and in [0, 1)"
                        .to_owned(),
                ));
            }
        }
        Ok(Self::Percentage { sizes, offsets })
    }

    pub fn rank(&self) -> usize {
        match self {
            Self::Absolute { sizes, .. } => sizes.len(),
            Self::Percentage { sizes, .. } => sizes.len(),
        }
    }

    pub fn resolve(
        &self,
        dimensions: &[u64],
    ) -> Result<ResolvedConditioningRegion, ConditioningError> {
        self.validate()?;
        if dimensions.len() != self.rank() || dimensions.contains(&0) {
            return Err(ConditioningError::Invalid(
                "conditioning region rank must match nonzero target dimensions".to_owned(),
            ));
        }
        let (requested_sizes, offsets) = match self {
            Self::Absolute { sizes, offsets } => (sizes.clone(), offsets.clone()),
            Self::Percentage { sizes, offsets } => {
                let mut resolved_sizes = Vec::new();
                let mut resolved_offsets = Vec::new();
                resolved_sizes
                    .try_reserve_exact(sizes.len())
                    .map_err(|_| ConditioningError::ShapeOverflow("percentage region sizes"))?;
                resolved_offsets
                    .try_reserve_exact(offsets.len())
                    .map_err(|_| ConditioningError::ShapeOverflow("percentage region offsets"))?;
                for ((size, offset), dimension) in sizes.iter().zip(offsets).zip(dimensions) {
                    let resolved_size = (*size * *dimension as f64).round_ties_even().max(1.0);
                    let resolved_offset = (*offset * *dimension as f64).round_ties_even();
                    if resolved_size > u64::MAX as f64 || resolved_offset > u64::MAX as f64 {
                        return Err(ConditioningError::ShapeOverflow("percentage region"));
                    }
                    resolved_sizes.push(resolved_size as u64);
                    resolved_offsets.push(resolved_offset as u64);
                }
                (resolved_sizes, resolved_offsets)
            }
        };
        let mut sizes = Vec::new();
        sizes
            .try_reserve_exact(dimensions.len())
            .map_err(|_| ConditioningError::ShapeOverflow("resolved region sizes"))?;
        for ((requested, offset), dimension) in requested_sizes.iter().zip(&offsets).zip(dimensions)
        {
            if *offset >= *dimension {
                return Err(ConditioningError::Invalid(
                    "conditioning region offset lies outside the target".to_owned(),
                ));
            }
            sizes.push((*requested).min(*dimension - *offset));
        }
        ResolvedConditioningRegion::checked(sizes, offsets, dimensions.to_vec())
    }

    fn validate(&self) -> Result<(), ConditioningError> {
        match self {
            Self::Absolute { sizes, offsets } => {
                validate_region_rank(sizes.len(), offsets.len())?;
                if sizes.contains(&0) {
                    return Err(ConditioningError::Invalid(
                        "absolute conditioning region sizes must be nonzero".to_owned(),
                    ));
                }
            }
            Self::Percentage { sizes, offsets } => {
                validate_region_rank(sizes.len(), offsets.len())?;
                if sizes
                    .iter()
                    .any(|size| !size.is_finite() || *size <= 0.0 || *size > 1.0)
                    || offsets
                        .iter()
                        .any(|offset| !offset.is_finite() || *offset < 0.0 || *offset >= 1.0)
                {
                    return Err(ConditioningError::Invalid(
                        "percentage conditioning region values are outside their checked ranges"
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedConditioningRegion {
    sizes: Vec<u64>,
    offsets: Vec<u64>,
    target_dimensions: Vec<u64>,
}

impl ResolvedConditioningRegion {
    fn checked(
        sizes: Vec<u64>,
        offsets: Vec<u64>,
        target_dimensions: Vec<u64>,
    ) -> Result<Self, ConditioningError> {
        validate_region_rank(sizes.len(), offsets.len())?;
        if target_dimensions.len() != sizes.len() {
            return Err(ConditioningError::Invalid(
                "resolved region rank does not match target rank".to_owned(),
            ));
        }
        for ((size, offset), dimension) in sizes.iter().zip(&offsets).zip(&target_dimensions) {
            let end = offset
                .checked_add(*size)
                .ok_or(ConditioningError::ShapeOverflow("resolved region end"))?;
            if *size == 0 || end > *dimension {
                return Err(ConditioningError::Invalid(
                    "resolved region is outside the target".to_owned(),
                ));
            }
        }
        Ok(Self {
            sizes,
            offsets,
            target_dimensions,
        })
    }

    pub fn full(dimensions: Vec<u64>) -> Result<Self, ConditioningError> {
        if dimensions.is_empty() || dimensions.contains(&0) {
            return Err(ConditioningError::Invalid(
                "full conditioning region requires nonzero dimensions".to_owned(),
            ));
        }
        Self::checked(dimensions.clone(), vec![0; dimensions.len()], dimensions)
    }

    pub fn sizes(&self) -> &[u64] {
        &self.sizes
    }

    pub fn offsets(&self) -> &[u64] {
        &self.offsets
    }

    pub fn target_dimensions(&self) -> &[u64] {
        &self.target_dimensions
    }

    pub fn is_full(&self) -> bool {
        self.offsets.iter().all(|offset| *offset == 0) && self.sizes == self.target_dimensions
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConditioningWindow {
    start_percent: f64,
    end_percent: f64,
}

impl ConditioningWindow {
    pub fn new(start_percent: f64, end_percent: f64) -> Result<Self, ConditioningError> {
        if !start_percent.is_finite()
            || !end_percent.is_finite()
            || !(0.0..=1.0).contains(&start_percent)
            || !(0.0..=1.0).contains(&end_percent)
            || start_percent > end_percent
        {
            return Err(ConditioningError::Invalid(
                "conditioning window must be a finite ordered range within [0, 1]".to_owned(),
            ));
        }
        Ok(Self {
            start_percent,
            end_percent,
        })
    }

    pub fn full() -> Self {
        Self {
            start_percent: 0.0,
            end_percent: 1.0,
        }
    }

    pub fn start_percent(self) -> f64 {
        self.start_percent
    }

    pub fn end_percent(self) -> f64 {
        self.end_percent
    }

    pub fn contains(self, percent: f64) -> bool {
        percent.is_finite() && percent >= self.start_percent && percent <= self.end_percent
    }
}

#[derive(Clone, Debug)]
pub struct ConditioningMask {
    tensor: Tensor,
    strength: f32,
    feather: Vec<u64>,
    set_region_to_nonzero_bounds: bool,
}

impl ConditioningMask {
    pub fn new(
        tensor: Tensor,
        strength: f32,
        feather: Vec<u64>,
        set_region_to_nonzero_bounds: bool,
    ) -> Result<Self, ConditioningError> {
        if !matches!(
            tensor.descriptor().dtype(),
            DType::F32 | DType::F16 | DType::Bf16
        ) {
            return Err(ConditioningError::Invalid(
                "conditioning mask must use a floating-point dtype".to_owned(),
            ));
        }
        if tensor.descriptor().rank() == 0 || tensor.descriptor().shape().contains(&0) {
            return Err(ConditioningError::Invalid(
                "conditioning mask must have nonzero spatial dimensions".to_owned(),
            ));
        }
        if !strength.is_finite() {
            return Err(ConditioningError::Invalid(
                "conditioning mask strength must be finite".to_owned(),
            ));
        }
        if feather.is_empty() {
            return Err(ConditioningError::Invalid(
                "conditioning mask feather rank must be nonzero".to_owned(),
            ));
        }
        if feather.iter().any(|value| *value != 0) {
            return Err(ConditioningError::Invalid(
                "conditioning mask feather must be zero because source feathering applies only to unmasked areas"
                    .to_owned(),
            ));
        }
        Ok(Self {
            tensor,
            strength,
            feather,
            set_region_to_nonzero_bounds,
        })
    }

    pub fn tensor(&self) -> &Tensor {
        &self.tensor
    }

    pub fn strength(&self) -> f32 {
        self.strength
    }

    pub fn feather(&self) -> &[u64] {
        &self.feather
    }

    pub fn set_region_to_nonzero_bounds(&self) -> bool {
        self.set_region_to_nonzero_bounds
    }

    fn normalized_to_target(
        &self,
        target: &TensorDescriptor,
        target_dimensions: &[u64],
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<Self, ConditioningError> {
        context.cancellation.check()?;
        if self.tensor.descriptor().device() != target.device()
            || self.tensor.descriptor().stream() != target.stream()
        {
            return Err(ConditioningError::Invalid(
                "conditioning mask must use the target device and stream".to_owned(),
            ));
        }
        let spatial_rank = target_dimensions.len();
        let mask_rank = self.tensor.descriptor().rank();
        let mut tensor = if mask_rank == spatial_rank {
            tensor_unsqueeze_exact_native(&self.tensor, 0, context.cancellation)?
        } else if mask_rank == spatial_rank + 1 {
            self.tensor.clone()
        } else {
            return Err(ConditioningError::Invalid(
                "conditioning mask must be spatial-only or [batch, spatial...]".to_owned(),
            ));
        };
        if tensor.descriptor().shape()[1..] == *target_dimensions {
            return Ok(Self {
                tensor,
                strength: self.strength,
                feather: self.feather.clone(),
                set_region_to_nonzero_bounds: self.set_region_to_nonzero_bounds,
            });
        }
        if spatial_rank < 2 {
            return Err(ConditioningError::Invalid(
                "one-dimensional conditioning masks cannot be bilinearly resized".to_owned(),
            ));
        }
        let dtype = tensor.descriptor().dtype();
        let resized_input = if dtype == DType::F32 {
            tensor
        } else {
            cast_to_with_context_exact_native(
                backend,
                &tensor,
                DType::F32,
                target.device(),
                false,
                false,
                context,
            )?
        };
        tensor = resize_with_context_exact_native(
            backend,
            &resized_input,
            target_dimensions[spatial_rank - 2],
            target_dimensions[spatial_rank - 1],
            ResizeMode::Bilinear,
            false,
            context,
        )?;
        if dtype != DType::F32 {
            tensor = cast_to_with_context_exact_native(
                backend,
                &tensor,
                dtype,
                target.device(),
                false,
                false,
                context,
            )?;
        }
        if tensor.descriptor().shape()[1..] != *target_dimensions {
            return Err(ConditioningError::Invalid(
                "conditioning mask non-spatial dimensions do not match the target".to_owned(),
            ));
        }
        context.cancellation.check()?;
        Ok(Self {
            tensor,
            strength: self.strength,
            feather: self.feather.clone(),
            set_region_to_nonzero_bounds: self.set_region_to_nonzero_bounds,
        })
    }

    fn nonzero_region(
        &self,
        target_dimensions: &[u64],
        cancellation: &comfy_types::CancellationToken,
    ) -> Result<ResolvedConditioningRegion, ConditioningError> {
        cancellation.check()?;
        if self.tensor.descriptor().device() != DeviceId::CPU {
            return Err(ConditioningError::UnreadableDevice(
                self.tensor.descriptor().device(),
            ));
        }
        let rank = self.tensor.descriptor().rank();
        let spatial_rank = target_dimensions.len();
        let spatial_shape = if rank == spatial_rank {
            self.tensor.descriptor().shape()
        } else if rank == spatial_rank + 1 {
            self.tensor
                .descriptor()
                .shape()
                .get(1..)
                .ok_or(ConditioningError::ShapeOverflow("mask spatial shape"))?
        } else {
            return Err(ConditioningError::Invalid(
                "conditioning mask must be spatial-only or [batch, spatial...]".to_owned(),
            ));
        };
        if spatial_shape != target_dimensions {
            return Err(ConditioningError::Invalid(
                "conditioning mask spatial dimensions must match the target".to_owned(),
            ));
        }
        let mut minimum = vec![u64::MAX; spatial_rank];
        let mut maximum = vec![0_u64; spatial_rank];
        let mut any_nonzero = false;
        let element_count = self.tensor.descriptor().element_count()?;
        for linear_index in 0..element_count {
            if linear_index.is_multiple_of(1024) {
                cancellation.check()?;
            }
            let value = self
                .tensor
                .descriptor()
                .dtype()
                .decode_scalar(self.tensor.linear_element_bytes(linear_index)?)?;
            if !value.is_nonzero() {
                continue;
            }
            any_nonzero = true;
            let mut remainder = linear_index;
            for axis in (0..rank).rev() {
                let dimension = self.tensor.descriptor().shape()[axis];
                let coordinate = remainder % dimension;
                remainder /= dimension;
                if rank == spatial_rank || axis > 0 {
                    let spatial_axis = if rank == spatial_rank { axis } else { axis - 1 };
                    minimum[spatial_axis] = minimum[spatial_axis].min(coordinate);
                    maximum[spatial_axis] = maximum[spatial_axis].max(coordinate);
                }
            }
        }
        cancellation.check()?;
        if !any_nonzero {
            let sizes = target_dimensions
                .iter()
                .map(|dimension| (*dimension).min(8))
                .collect::<Vec<_>>();
            return ResolvedConditioningRegion::checked(
                sizes,
                vec![0; spatial_rank],
                target_dimensions.to_vec(),
            );
        }
        let sizes = minimum
            .iter()
            .zip(&maximum)
            .map(|(minimum, maximum)| maximum - minimum + 1)
            .collect::<Vec<_>>();
        ResolvedConditioningRegion::checked(sizes, minimum, target_dimensions.to_vec())
    }

    pub fn resolve(
        &self,
        target: &TensorDescriptor,
        region: &ResolvedConditioningRegion,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<ResolvedConditioningMask, ConditioningError> {
        context.cancellation.check()?;
        if target.rank() < 3 || target.shape()[2..] != *region.target_dimensions() {
            return Err(ConditioningError::Invalid(
                "conditioning mask target and region dimensions do not match".to_owned(),
            ));
        }
        let spatial_rank = region.sizes().len();
        let normalized =
            self.normalized_to_target(target, region.target_dimensions(), backend, context)?;
        if self.feather.len() != spatial_rank {
            return Err(ConditioningError::Invalid(
                "conditioning mask feather rank must match the spatial rank".to_owned(),
            ));
        }
        let mut mask = normalized.tensor;
        for (dimension, (&offset, &size)) in region.offsets().iter().zip(region.sizes()).enumerate()
        {
            let axis = i64::try_from(dimension + 1)
                .map_err(|_| ConditioningError::ShapeOverflow("mask crop axis"))?;
            let offset = i64::try_from(offset)
                .map_err(|_| ConditioningError::ShapeOverflow("mask crop offset"))?;
            mask = narrow_method_exact_native(&mask, axis, offset, size, context.cancellation)?;
        }
        let batch_size = target.shape()[0];
        let channel_count = target.shape()[1];
        mask = repeat_to_batch_size(&mask, batch_size, backend, context)?;
        mask = tensor_unsqueeze_exact_native(&mask, 1, context.cancellation)?;
        if channel_count != 1 {
            let mut repeats = vec![1_i64; mask.descriptor().rank()];
            *repeats
                .get_mut(1)
                .ok_or(ConditioningError::ShapeOverflow("mask channel repeat"))? =
                i64::try_from(channel_count)
                    .map_err(|_| ConditioningError::ShapeOverflow("mask channel count"))?;
            mask = tensor_repeat_with_context_exact_native(backend, &mask, &repeats, context)?;
        }
        for (&feather, &size) in self.feather.iter().zip(region.sizes()) {
            if feather > size.saturating_sub(1) / 2 {
                return Err(ConditioningError::Invalid(
                    "conditioning mask feather exceeds half the resolved region".to_owned(),
                ));
            }
        }
        context.cancellation.check()?;
        Ok(ResolvedConditioningMask {
            tensor: mask,
            strength: self.strength,
            feather: self.feather.clone(),
            region: region.clone(),
            set_region_to_nonzero_bounds: self.set_region_to_nonzero_bounds,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedConditioningMask {
    tensor: Tensor,
    strength: f32,
    feather: Vec<u64>,
    region: ResolvedConditioningRegion,
    set_region_to_nonzero_bounds: bool,
}

impl ResolvedConditioningMask {
    pub fn tensor(&self) -> &Tensor {
        &self.tensor
    }

    pub fn strength(&self) -> f32 {
        self.strength
    }

    pub fn set_region_to_nonzero_bounds(&self) -> bool {
        self.set_region_to_nonzero_bounds
    }

    pub fn feather_weight(&self, spatial_indices: &[u64]) -> Result<f32, ConditioningError> {
        if spatial_indices.len() != self.region.sizes().len() {
            return Err(ConditioningError::Invalid(
                "feather index rank must match the region rank".to_owned(),
            ));
        }
        let mut weight = self.strength;
        for ((index, size), feather) in spatial_indices
            .iter()
            .zip(self.region.sizes())
            .zip(&self.feather)
        {
            if *index >= *size {
                return Err(ConditioningError::Invalid(
                    "feather index lies outside the resolved region".to_owned(),
                ));
            }
            if *feather == 0 {
                continue;
            }
            let divisor = (*feather + 1) as f32;
            let leading = (*index + 1) as f32 / divisor;
            let trailing = (*size - *index) as f32 / divisor;
            weight *= leading.min(trailing).min(1.0);
        }
        Ok(weight)
    }
}

#[derive(Clone, Debug)]
pub struct ConditioningEntryOptions {
    pub strength: f32,
    pub region: Option<ConditioningRegion>,
    pub mask: Option<ConditioningMask>,
    pub window: ConditioningWindow,
    pub default_region: bool,
}

impl Default for ConditioningEntryOptions {
    fn default() -> Self {
        Self {
            strength: 1.0,
            region: None,
            mask: None,
            window: ConditioningWindow::full(),
            default_region: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConditioningEntry {
    identifier: String,
    value: ConditioningValue,
    options: ConditioningEntryOptions,
    references: ConditioningReferences,
}

impl ConditioningEntry {
    pub fn checked(
        identifier: impl Into<String>,
        value: ConditioningValue,
        options: ConditioningEntryOptions,
    ) -> Result<Self, ConditioningError> {
        Self::checked_with_references(
            identifier,
            value,
            options,
            ConditioningReferences::default(),
        )
    }

    pub fn checked_with_references(
        identifier: impl Into<String>,
        value: ConditioningValue,
        options: ConditioningEntryOptions,
        references: ConditioningReferences,
    ) -> Result<Self, ConditioningError> {
        let identifier = identifier.into();
        validate_identifier("conditioning entry", &identifier)?;
        if !options.strength.is_finite() {
            return Err(ConditioningError::Invalid(
                "conditioning entry strength must be finite".to_owned(),
            ));
        }
        value.validate()?;
        if let Some(region) = &options.region {
            region.validate()?;
        }
        if options.default_region && (options.region.is_some() || options.mask.is_some()) {
            return Err(ConditioningError::Invalid(
                "default-region conditioning cannot own an explicit region or mask".to_owned(),
            ));
        }
        Ok(Self {
            identifier,
            value,
            options,
            references,
        })
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn value(&self) -> &ConditioningValue {
        &self.value
    }

    pub fn options(&self) -> &ConditioningEntryOptions {
        &self.options
    }

    pub fn references(&self) -> &ConditioningReferences {
        &self.references
    }

    pub fn resolve(
        &self,
        target: &TensorDescriptor,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<ResolvedConditioningEntry, ConditioningError> {
        context.cancellation.check()?;
        if target.rank() < 3 || target.shape()[0] == 0 || target.shape()[1] == 0 {
            return Err(ConditioningError::Invalid(
                "conditioning target must have nonzero batch, channel, and spatial dimensions"
                    .to_owned(),
            ));
        }
        let target_dimensions = target.shape()[2..].to_vec();
        let mask = self
            .options
            .mask
            .as_ref()
            .map(|mask| mask.normalized_to_target(target, &target_dimensions, backend, context))
            .transpose()?;
        let mut region = match &self.options.region {
            Some(region) => region.resolve(&target_dimensions)?,
            None => ResolvedConditioningRegion::full(target_dimensions.clone())?,
        };
        if let Some(mask) = &mask
            && mask.set_region_to_nonzero_bounds()
        {
            let mask_region = mask.nonzero_region(&target_dimensions, context.cancellation)?;
            region = intersect_regions(&region, &mask_region)?;
        }
        let crop_value = self.options.region.is_some()
            || self
                .options
                .mask
                .as_ref()
                .is_some_and(ConditioningMask::set_region_to_nonzero_bounds);
        let value = self.value.process(
            target.shape()[0],
            crop_value.then_some(&region),
            backend,
            context,
        )?;
        let mask = mask
            .as_ref()
            .map(|mask| mask.resolve(target, &region, backend, context))
            .transpose()?;
        context.cancellation.check()?;
        Ok(ResolvedConditioningEntry {
            identifier: self.identifier.clone(),
            value,
            strength: self.options.strength,
            region,
            mask,
            window: self.options.window,
            default_region: self.options.default_region,
            references: self.references.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedConditioningEntry {
    identifier: String,
    value: ConditioningValue,
    strength: f32,
    region: ResolvedConditioningRegion,
    mask: Option<ResolvedConditioningMask>,
    window: ConditioningWindow,
    default_region: bool,
    references: ConditioningReferences,
}

impl ResolvedConditioningEntry {
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn value(&self) -> &ConditioningValue {
        &self.value
    }

    pub fn strength(&self) -> f32 {
        self.strength
    }

    pub fn region(&self) -> &ResolvedConditioningRegion {
        &self.region
    }

    pub fn mask(&self) -> Option<&ResolvedConditioningMask> {
        self.mask.as_ref()
    }

    pub fn window(&self) -> ConditioningWindow {
        self.window
    }

    pub fn is_default_region(&self) -> bool {
        self.default_region
    }

    pub fn references(&self) -> &ConditioningReferences {
        &self.references
    }

    pub fn contribution_weight(
        &self,
        spatial_indices: &[u64],
        mask_value: Option<f32>,
    ) -> Result<f32, ConditioningError> {
        if spatial_indices.len() != self.region.sizes().len() {
            return Err(ConditioningError::Invalid(
                "conditioning weight index rank must match the region rank".to_owned(),
            ));
        }
        for (index, size) in spatial_indices.iter().zip(self.region.sizes()) {
            if index >= size {
                return Err(ConditioningError::Invalid(
                    "conditioning weight index lies outside the resolved region".to_owned(),
                ));
            }
        }
        let local_weight = match (&self.mask, mask_value) {
            (Some(mask), Some(value)) if value.is_finite() => {
                mask.feather_weight(spatial_indices)? * value
            }
            (None, None) => area_only_feather_weight(&self.region, spatial_indices)?,
            (Some(_), Some(_)) => {
                return Err(ConditioningError::Invalid(
                    "conditioning mask value must be finite".to_owned(),
                ));
            }
            _ => {
                return Err(ConditioningError::Invalid(
                    "conditioning mask value presence does not match the resolved entry".to_owned(),
                ));
            }
        };
        let weight = self.strength * local_weight;
        if !weight.is_finite() {
            return Err(ConditioningError::Invalid(
                "conditioning contribution weight must be finite".to_owned(),
            ));
        }
        Ok(weight)
    }
}

pub fn default_region_residual_weight(covered_weight: f32) -> Result<f32, ConditioningError> {
    if !covered_weight.is_finite() {
        return Err(ConditioningError::Invalid(
            "default-region covered weight must be finite".to_owned(),
        ));
    }
    Ok((1.0 - covered_weight).max(0.0))
}

#[derive(Clone, Debug)]
pub struct ConditioningSet {
    identity: ConditioningIdentity,
    entries: Vec<ConditioningEntry>,
    digest: String,
}

impl ConditioningSet {
    pub fn checked(
        identity: ConditioningIdentity,
        entries: Vec<ConditioningEntry>,
        cancellation: &comfy_types::CancellationToken,
    ) -> Result<Self, ConditioningError> {
        cancellation.check()?;
        if entries.is_empty() {
            return Err(ConditioningError::Invalid(
                "conditioning set must contain at least one entry".to_owned(),
            ));
        }
        if entries.len() > MAX_CONDITIONING_ENTRIES {
            return Err(ConditioningError::Invalid(format!(
                "conditioning set contains more than {MAX_CONDITIONING_ENTRIES} entries"
            )));
        }
        let mut identifiers = BTreeSet::new();
        for entry in &entries {
            cancellation.check()?;
            if !identifiers.insert(entry.identifier()) {
                return Err(ConditioningError::Invalid(format!(
                    "conditioning entry identifier is duplicated: {}",
                    entry.identifier()
                )));
            }
        }
        let digest = hash_set(&identity, &entries, cancellation)?;
        Ok(Self {
            identity,
            entries,
            digest,
        })
    }

    pub fn identity(&self) -> &ConditioningIdentity {
        &self.identity
    }

    pub fn entries(&self) -> &[ConditioningEntry] {
        &self.entries
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn resolve(
        &self,
        target: &TensorDescriptor,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<ResolvedConditioningEntry>, ConditioningError> {
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(self.entries.len())
            .map_err(|_| ConditioningError::ShapeOverflow("resolved conditioning entries"))?;
        for entry in &self.entries {
            context.cancellation.check()?;
            entries.push(entry.resolve(target, backend, context)?);
        }
        context.cancellation.check()?;
        Ok(entries)
    }
}

fn validate_identifier(subject: &str, value: &str) -> Result<(), ConditioningError> {
    if value.is_empty()
        || value.len() > 256
        || value.starts_with('.')
        || value.ends_with('.')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(ConditioningError::Invalid(format!(
            "{subject} must be 1..=256 ASCII identifier characters"
        )));
    }
    Ok(())
}

fn validate_batched_tensor(tensor: &Tensor, subject: &str) -> Result<(), ConditioningError> {
    if tensor.descriptor().rank() == 0 || tensor.descriptor().shape()[0] == 0 {
        return Err(ConditioningError::Invalid(format!(
            "{subject} must have a nonzero batch dimension"
        )));
    }
    Ok(())
}

fn validate_region_rank(sizes: usize, offsets: usize) -> Result<(), ConditioningError> {
    if sizes == 0 || sizes != offsets || sizes > MAX_CONDITIONING_REGION_RANK {
        return Err(ConditioningError::Invalid(format!(
            "conditioning region sizes and offsets must have the same rank in 1..={MAX_CONDITIONING_REGION_RANK}"
        )));
    }
    Ok(())
}

fn intersect_regions(
    left: &ResolvedConditioningRegion,
    right: &ResolvedConditioningRegion,
) -> Result<ResolvedConditioningRegion, ConditioningError> {
    if left.target_dimensions() != right.target_dimensions() {
        return Err(ConditioningError::Invalid(
            "conditioning region intersection targets differ".to_owned(),
        ));
    }
    let mut sizes = Vec::new();
    let mut offsets = Vec::new();
    sizes
        .try_reserve_exact(left.sizes().len())
        .map_err(|_| ConditioningError::ShapeOverflow("region intersection sizes"))?;
    offsets
        .try_reserve_exact(left.sizes().len())
        .map_err(|_| ConditioningError::ShapeOverflow("region intersection offsets"))?;
    for (((left_size, left_offset), right_size), right_offset) in left
        .sizes()
        .iter()
        .zip(left.offsets())
        .zip(right.sizes())
        .zip(right.offsets())
    {
        let start = (*left_offset).max(*right_offset);
        let left_end = left_offset
            .checked_add(*left_size)
            .ok_or(ConditioningError::ShapeOverflow("left region end"))?;
        let right_end = right_offset
            .checked_add(*right_size)
            .ok_or(ConditioningError::ShapeOverflow("right region end"))?;
        let end = left_end.min(right_end);
        if end <= start {
            return Err(ConditioningError::Invalid(
                "conditioning mask bounds do not intersect the explicit region".to_owned(),
            ));
        }
        offsets.push(start);
        sizes.push(end - start);
    }
    ResolvedConditioningRegion::checked(sizes, offsets, left.target_dimensions().to_vec())
}

fn area_only_feather_weight(
    region: &ResolvedConditioningRegion,
    spatial_indices: &[u64],
) -> Result<f32, ConditioningError> {
    let mut weight = 1.0_f32;
    for (((index, size), offset), target_dimension) in spatial_indices
        .iter()
        .zip(region.sizes())
        .zip(region.offsets())
        .zip(region.target_dimensions())
    {
        let feather = SOURCE_AREA_FEATHER.min(*size / 4);
        if feather == 0 {
            continue;
        }
        if *offset != 0 && *index < feather {
            weight *= (*index + 1) as f32 / feather as f32;
        }
        let end = offset
            .checked_add(*size)
            .ok_or(ConditioningError::ShapeOverflow(
                "conditioning area feather end",
            ))?;
        let trailing_start = size
            .checked_sub(feather)
            .ok_or(ConditioningError::ShapeOverflow(
                "conditioning area feather start",
            ))?;
        if end < *target_dimension && *index >= trailing_start {
            weight *= (*size - *index) as f32 / feather as f32;
        }
    }
    Ok(weight)
}

fn validate_constant_tensor(tensor: &Tensor) -> Result<(), ConditioningError> {
    tensor.descriptor().element_count()?;
    if tensor.storage_byte_len() > MAX_CONDITIONING_CONSTANT_BYTES as u64 {
        return Err(ConditioningError::Invalid(format!(
            "conditioning constant tensor exceeds {MAX_CONDITIONING_CONSTANT_BYTES} bytes"
        )));
    }
    Ok(())
}

fn tensor_constants_equal(left: &Tensor, right: &Tensor) -> bool {
    if left.descriptor() != right.descriptor() {
        return false;
    }
    let Ok(elements) = left.descriptor().element_count() else {
        return false;
    };
    for index in 0..elements {
        let Ok(left) = left.linear_element_bytes(index) else {
            return false;
        };
        let Ok(right) = right.linear_element_bytes(index) else {
            return false;
        };
        if left != right {
            return false;
        }
    }
    true
}

fn tensor_concat_compatible(left: &Tensor, right: &Tensor, ignored_axis: Option<usize>) -> bool {
    let left_descriptor = left.descriptor();
    let right_descriptor = right.descriptor();
    if left_descriptor.rank() != right_descriptor.rank()
        || left_descriptor.dtype() != right_descriptor.dtype()
        || left_descriptor.device() != right_descriptor.device()
        || left_descriptor.stream() != right_descriptor.stream()
    {
        return false;
    }
    left_descriptor
        .shape()
        .iter()
        .zip(right_descriptor.shape())
        .enumerate()
        .all(|(axis, (left, right))| ignored_axis == Some(axis) || left == right)
}

fn cross_attention_pair_compatible(left: &Tensor, right: &Tensor) -> bool {
    if !tensor_concat_compatible(left, right, Some(1)) {
        return false;
    }
    let left_tokens = left.descriptor().shape()[1];
    let right_tokens = right.descriptor().shape()[1];
    checked_lcm(left_tokens, right_tokens)
        .is_some_and(|tokens| tokens / left_tokens <= 4 && tokens / right_tokens <= 4)
}

fn concat_regular<Extract>(
    first: &Tensor,
    others: &[ConditioningValue],
    extract: Extract,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ConditioningError>
where
    Extract: Fn(&ConditioningValue) -> Option<&Tensor>,
{
    let mut tensors = Vec::new();
    tensors
        .try_reserve_exact(others.len() + 1)
        .map_err(|_| ConditioningError::ShapeOverflow("conditioning concatenation inputs"))?;
    tensors.push(first.clone());
    for other in others {
        context.cancellation.check()?;
        let tensor = extract(other).ok_or_else(|| {
            ConditioningError::Invalid("conditioning value kinds cannot be concatenated".to_owned())
        })?;
        if !tensor_concat_compatible(first, tensor, None) {
            return Err(ConditioningError::Invalid(
                "regular conditioning shapes, dtypes, devices, and streams must match".to_owned(),
            ));
        }
        tensors.push(tensor.clone());
    }
    Ok(torch_cat_with_context_exact_native(
        backend, &tensors, 0, context,
    )?)
}

fn concat_cross_attention(
    first: &Tensor,
    others: &[ConditioningValue],
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ConditioningError> {
    let mut tensors = Vec::new();
    tensors
        .try_reserve_exact(others.len() + 1)
        .map_err(|_| ConditioningError::ShapeOverflow("cross-attention inputs"))?;
    tensors.push(first.clone());
    for other in others {
        let ConditioningValue::CrossAttention(tensor) = other else {
            return Err(ConditioningError::Invalid(
                "cross-attention values can only concatenate with cross-attention values"
                    .to_owned(),
            ));
        };
        if !tensor_concat_compatible(first, tensor, Some(1)) {
            return Err(ConditioningError::Invalid(
                "cross-attention batch, channel, dtype, device, and stream contracts differ"
                    .to_owned(),
            ));
        }
        tensors.push(tensor.clone());
    }
    let token_count = tensors.iter().try_fold(1_u64, |token_count, tensor| {
        checked_lcm(token_count, tensor.descriptor().shape()[1]).ok_or(
            ConditioningError::ShapeOverflow("cross-attention token LCM"),
        )
    })?;
    if tensors.iter().any(|tensor| {
        token_count / tensor.descriptor().shape()[1] > CROSS_ATTENTION_MAX_REPEAT_RATIO
    }) {
        return Err(ConditioningError::Invalid(
            "cross-attention token padding exceeds the maximum source repeat ratio of four"
                .to_owned(),
        ));
    }
    let mut padded = Vec::new();
    padded
        .try_reserve_exact(tensors.len())
        .map_err(|_| ConditioningError::ShapeOverflow("padded cross-attention inputs"))?;
    for tensor in &tensors {
        context.cancellation.check()?;
        let ratio = token_count / tensor.descriptor().shape()[1];
        if ratio == 1 {
            padded.push(tensor.clone());
        } else {
            let mut repeats = vec![1_i64; 3];
            repeats[1] = i64::try_from(ratio)
                .map_err(|_| ConditioningError::ShapeOverflow("cross-attention repeat"))?;
            padded.push(tensor_repeat_with_context_exact_native(
                backend, tensor, &repeats, context,
            )?);
        }
    }
    Ok(torch_cat_with_context_exact_native(
        backend, &padded, 0, context,
    )?)
}

fn concat_lists(
    first: &[Tensor],
    others: &[ConditioningValue],
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<ConditioningValue, ConditioningError> {
    let mut lists = Vec::new();
    lists
        .try_reserve_exact(others.len())
        .map_err(|_| ConditioningError::ShapeOverflow("conditioning lists"))?;
    for other in others {
        let ConditioningValue::List(tensors) = other else {
            return Err(ConditioningError::Invalid(
                "conditioning lists can only concatenate with lists".to_owned(),
            ));
        };
        if tensors.len() != first.len() {
            return Err(ConditioningError::Invalid(
                "conditioning lists must have equal lengths".to_owned(),
            ));
        }
        lists.push(tensors);
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(first.len())
        .map_err(|_| ConditioningError::ShapeOverflow("concatenated conditioning list"))?;
    for (index, first_tensor) in first.iter().enumerate() {
        let mut tensors = Vec::new();
        tensors
            .try_reserve_exact(lists.len() + 1)
            .map_err(|_| ConditioningError::ShapeOverflow("conditioning list item inputs"))?;
        tensors.push(first_tensor.clone());
        for list in &lists {
            let tensor = list
                .get(index)
                .ok_or(ConditioningError::ShapeOverflow("conditioning list item"))?;
            if !tensor_concat_compatible(first_tensor, tensor, None) {
                return Err(ConditioningError::Invalid(
                    "conditioning list item shapes, dtypes, devices, and streams must match"
                        .to_owned(),
                ));
            }
            tensors.push(tensor.clone());
        }
        output.push(torch_cat_with_context_exact_native(
            backend, &tensors, 0, context,
        )?);
    }
    Ok(ConditioningValue::List(output))
}

fn repeat_to_batch_size(
    tensor: &Tensor,
    batch_size: u64,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ConditioningError> {
    context.cancellation.check()?;
    validate_batched_tensor(tensor, "repeated conditioning")?;
    let source_batch = tensor.descriptor().shape()[0];
    if source_batch == batch_size {
        return Ok(tensor.clone());
    }
    if source_batch > batch_size {
        return Ok(narrow_method_exact_native(
            tensor,
            0,
            0,
            batch_size,
            context.cancellation,
        )?);
    }
    let repeat_count =
        batch_size
            .checked_add(source_batch - 1)
            .ok_or(ConditioningError::ShapeOverflow(
                "conditioning batch repeat",
            ))?
            / source_batch;
    let mut repeats = vec![1_i64; tensor.descriptor().rank()];
    repeats[0] = i64::try_from(repeat_count)
        .map_err(|_| ConditioningError::ShapeOverflow("conditioning batch repeat"))?;
    let repeated = tensor_repeat_with_context_exact_native(backend, tensor, &repeats, context)?;
    Ok(narrow_method_exact_native(
        &repeated,
        0,
        0,
        batch_size,
        context.cancellation,
    )?)
}

fn narrow_spatial(
    tensor: &Tensor,
    region: &ResolvedConditioningRegion,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, ConditioningError> {
    if tensor.descriptor().rank() != region.sizes().len() + 2 {
        return Err(ConditioningError::Invalid(
            "noise-shape conditioning and region spatial ranks differ".to_owned(),
        ));
    }
    let mut output = tensor.clone();
    for (dimension, (&offset, &size)) in region.offsets().iter().zip(region.sizes()).enumerate() {
        context.cancellation.check()?;
        let axis = i64::try_from(dimension + 2)
            .map_err(|_| ConditioningError::ShapeOverflow("noise-shape crop axis"))?;
        let offset = i64::try_from(offset)
            .map_err(|_| ConditioningError::ShapeOverflow("noise-shape crop offset"))?;
        output = narrow_method_exact_native(&output, axis, offset, size, context.cancellation)?;
    }
    Ok(output)
}

fn checked_lcm(left: u64, right: u64) -> Option<u64> {
    if left == 0 || right == 0 {
        return None;
    }
    left.checked_div(greatest_common_divisor(left, right))?
        .checked_mul(right)
}

fn greatest_common_divisor(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn hash_set(
    identity: &ConditioningIdentity,
    entries: &[ConditioningEntry],
    cancellation: &comfy_types::CancellationToken,
) -> Result<String, ConditioningError> {
    let mut hasher = Sha256::new();
    hasher.update(CONDITIONING_SCHEMA_VERSION.to_le_bytes());
    hash_string(&mut hasher, identity.namespace())?;
    hash_string(&mut hasher, identity.model_family().feature_id())?;
    hash_string(&mut hasher, identity.model_family().identifier())?;
    hash_string(&mut hasher, identity.model_family().architecture_version())?;
    hash_string(&mut hasher, identity.latent_format().feature_id())?;
    hash_string(&mut hasher, identity.latent_format().identifier())?;
    hash_u64(&mut hasher, entries.len())?;
    for entry in entries {
        cancellation.check()?;
        hash_string(&mut hasher, entry.identifier())?;
        hash_string(
            &mut hasher,
            &entry.value().deterministic_digest(cancellation)?,
        )?;
        hasher.update(entry.options().strength.to_bits().to_le_bytes());
        hash_region(&mut hasher, entry.options().region.as_ref())?;
        hasher.update(entry.options().window.start_percent.to_bits().to_le_bytes());
        hasher.update(entry.options().window.end_percent.to_bits().to_le_bytes());
        hasher.update([u8::from(entry.options().default_region)]);
        match entry.references().control() {
            Some(control) => {
                hasher.update([1]);
                hash_string(&mut hasher, control.identifier())?;
            }
            None => hasher.update([0]),
        }
        hash_u64(&mut hasher, entry.references().hooks().len())?;
        for hook in entry.references().hooks() {
            hash_string(&mut hasher, hook.identifier())?;
        }
        match &entry.options().mask {
            Some(mask) => {
                hasher.update([1]);
                hash_tensor(&mut hasher, mask.tensor(), cancellation)?;
                hasher.update(mask.strength().to_bits().to_le_bytes());
                hash_u64(&mut hasher, mask.feather().len())?;
                for feather in mask.feather() {
                    hasher.update(feather.to_le_bytes());
                }
                hasher.update([u8::from(mask.set_region_to_nonzero_bounds())]);
            }
            None => hasher.update([0]),
        }
    }
    cancellation.check()?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_conditioning_constant(
    hasher: &mut Sha256,
    value: &ConditioningConstant,
    cancellation: &comfy_types::CancellationToken,
) -> Result<(), ConditioningError> {
    cancellation.check()?;
    match value {
        ConditioningConstant::Null => hasher.update([0]),
        ConditioningConstant::Boolean(value) => hasher.update([1, u8::from(*value)]),
        ConditioningConstant::Signed(value) => {
            hasher.update([2]);
            hasher.update(value.to_le_bytes());
        }
        ConditioningConstant::Unsigned(value) => {
            hasher.update([3]);
            hasher.update(value.to_le_bytes());
        }
        ConditioningConstant::FiniteF64Bits(value) => {
            hasher.update([4]);
            hasher.update(value.to_le_bytes());
        }
        ConditioningConstant::Text(value) => {
            hasher.update([5]);
            hash_string(hasher, value)?;
        }
        ConditioningConstant::Bytes(value) => {
            hasher.update([6]);
            hash_u64(hasher, value.len())?;
            hasher.update(value);
        }
        ConditioningConstant::Tensor(tensor) => {
            hasher.update([7]);
            hash_tensor(hasher, tensor, cancellation)?;
        }
        ConditioningConstant::List(values) | ConditioningConstant::Tuple(values) => {
            hasher.update([if matches!(value, ConditioningConstant::List(_)) {
                8
            } else {
                9
            }]);
            hash_u64(hasher, values.len())?;
            for value in values {
                hash_conditioning_constant(hasher, value, cancellation)?;
            }
        }
        ConditioningConstant::Map(values) => {
            hasher.update([10]);
            hash_u64(hasher, values.len())?;
            for (key, value) in values {
                hash_string(hasher, key)?;
                hash_conditioning_constant(hasher, value, cancellation)?;
            }
        }
    }
    Ok(())
}

fn hash_region(
    hasher: &mut Sha256,
    region: Option<&ConditioningRegion>,
) -> Result<(), ConditioningError> {
    match region {
        None => hasher.update([0]),
        Some(ConditioningRegion::Absolute { sizes, offsets }) => {
            hasher.update([1]);
            hash_u64(hasher, sizes.len())?;
            for value in sizes.iter().chain(offsets) {
                hasher.update(value.to_le_bytes());
            }
        }
        Some(ConditioningRegion::Percentage { sizes, offsets }) => {
            hasher.update([2]);
            hash_u64(hasher, sizes.len())?;
            for value in sizes.iter().chain(offsets) {
                hasher.update(value.to_bits().to_le_bytes());
            }
        }
    }
    Ok(())
}

fn hash_tensor(
    hasher: &mut Sha256,
    tensor: &Tensor,
    cancellation: &comfy_types::CancellationToken,
) -> Result<(), ConditioningError> {
    cancellation.check()?;
    if tensor.descriptor().device() != DeviceId::CPU {
        return Err(ConditioningError::UnreadableDevice(
            tensor.descriptor().device(),
        ));
    }
    hasher.update(
        serde_json::to_vec(&(
            tensor.descriptor().shape(),
            tensor.descriptor().dtype(),
            tensor.descriptor().device(),
            tensor.descriptor().stream(),
        ))
        .map_err(|error| ConditioningError::Encoding(error.to_string()))?,
    );
    let elements = tensor.descriptor().element_count()?;
    for index in 0..elements {
        if index.is_multiple_of(1024) {
            cancellation.check()?;
        }
        hasher.update(tensor.linear_element_bytes(index)?);
    }
    cancellation.check()?;
    Ok(())
}

fn hash_string(hasher: &mut Sha256, value: &str) -> Result<(), ConditioningError> {
    hash_u64(hasher, value.len())?;
    hasher.update(value.as_bytes());
    Ok(())
}

fn hash_u64(hasher: &mut Sha256, value: usize) -> Result<(), ConditioningError> {
    let value = u64::try_from(value)
        .map_err(|_| ConditioningError::ShapeOverflow("conditioning digest length"))?;
    hasher.update(value.to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use comfy_tensor::{CpuWorkspaceAuthority, DecodedScalar, DeviceId, StreamId};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    fn runtime() -> Result<(CpuBackend, CpuWorkspaceAuthority), TensorError> {
        CpuWorkspaceAuthority::create_backend(16 * 1_048_576)
    }

    fn upload_f32(
        backend: &CpuBackend,
        authority: &CpuWorkspaceAuthority,
        shape: Vec<u64>,
        values: &[f32],
    ) -> Result<Tensor, Box<dyn Error>> {
        let cancellation = comfy_types::CancellationToken::default();
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(0)?,
            &cancellation,
        );
        let descriptor =
            TensorDescriptor::contiguous(shape, DType::F32, DeviceId::CPU, StreamId::DEFAULT)?;
        Ok(backend.upload_f32(descriptor, values, &context)?.0)
    }

    fn tensor_values(tensor: &Tensor) -> Result<Vec<f64>, Box<dyn Error>> {
        let count = tensor.descriptor().element_count()?;
        let mut values = Vec::new();
        values.try_reserve_exact(usize::try_from(count)?)?;
        for index in 0..count {
            let value = tensor
                .descriptor()
                .dtype()
                .decode_scalar(tensor.linear_element_bytes(index)?)?;
            match value {
                DecodedScalar::Real(value) => values.push(value),
                _ => return Err("test tensor is not real-valued".into()),
            }
        }
        Ok(values)
    }

    fn identity() -> Result<ConditioningIdentity, Box<dyn Error>> {
        Ok(ConditioningIdentity::new(
            "native.conditioning.test",
            ModelFamilyIdentity::new("COMFY-MODEL-0999", "conditioning_test", "v1")?,
            LatentFormatIdentity::new("COMFY-MODEL-0045", "SD15")?,
        )?)
    }

    #[test]
    fn identities_and_windows_reuse_canonical_owners_and_reject_invalid_inputs() -> TestResult {
        let identity = identity()?;
        assert_eq!(identity.namespace(), "native.conditioning.test");
        assert_eq!(identity.model_family().identifier(), "conditioning_test");
        assert_eq!(identity.latent_format().identifier(), "SD15");
        assert!(
            ConditioningIdentity::new(
                "bad namespace",
                identity.model_family().clone(),
                identity.latent_format().clone(),
            )
            .is_err()
        );

        let window = ConditioningWindow::new(0.2, 0.8)?;
        assert!(window.contains(0.2));
        assert!(window.contains(0.8));
        assert!(!window.contains(0.81));
        assert!(ConditioningWindow::new(-0.1, 0.8).is_err());
        assert!(ConditioningWindow::new(0.9, 0.8).is_err());
        assert!(ConditioningWindow::new(f64::NAN, 0.8).is_err());

        let encoded = serde_json::to_string(&identity)?;
        let decoded: ConditioningIdentity = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, identity);
        assert_eq!(decoded.digest()?, identity.digest()?);
        assert_eq!(decoded.resident_bytes()?, identity.resident_bytes()?);
        let mut unsupported: serde_json::Value = serde_json::from_str(&encoded)?;
        let object = unsupported
            .as_object_mut()
            .ok_or("conditioning identity did not encode as an object")?;
        object.insert("schema_version".to_owned(), serde_json::Value::from(2));
        assert!(serde_json::from_value::<ConditioningIdentity>(unsupported).is_err());
        let mut unknown: serde_json::Value = serde_json::from_str(&encoded)?;
        unknown
            .as_object_mut()
            .ok_or("conditioning identity did not encode as an object")?
            .insert("unknown".to_owned(), serde_json::Value::Null);
        assert!(serde_json::from_value::<ConditioningIdentity>(unknown).is_err());

        let references = ConditioningReferences::checked(
            Some(ConditioningControlReference::checked("control.primary")?),
            vec![
                ConditioningHookReference::checked("hook.first")?,
                ConditioningHookReference::checked("hook.second")?,
            ],
        )?;
        assert_eq!(
            references.control().map(|reference| reference.identifier()),
            Some("control.primary")
        );
        assert_eq!(references.hooks()[1].identifier(), "hook.second");
        assert!(
            ConditioningReferences::checked(
                None,
                vec![
                    ConditioningHookReference::checked("hook.duplicate")?,
                    ConditioningHookReference::checked("hook.duplicate")?,
                ],
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn regular_repeat_and_concat_delegate_to_canonical_tensor_operations() -> TestResult {
        let (backend, authority) = runtime()?;
        let tensor = upload_f32(&backend, &authority, vec![2, 1], &[1.0, 2.0])?;
        let original_storage = tensor.storage_id();
        let value = ConditioningValue::regular(tensor.clone())?;
        let cancellation = comfy_types::CancellationToken::default();
        let scratch = authority.authorize_workspace(6 * 4)?;
        let context = backend.execution_context(StreamId::DEFAULT, scratch.clone(), &cancellation);
        let processed = value.process(5, None, &backend, &context)?;
        let ConditioningValue::Regular(processed_tensor) = processed else {
            return Err("processed regular value changed kind".into());
        };
        assert_eq!(processed_tensor.descriptor().shape(), &[5, 1]);
        assert_eq!(tensor_values(&processed_tensor)?, [1.0, 2.0, 1.0, 2.0, 1.0]);
        assert_eq!(tensor.storage_id(), original_storage);
        assert_eq!(tensor_values(&tensor)?, [1.0, 2.0]);
        assert_eq!(scratch.in_use_bytes(), 0);

        let other =
            ConditioningValue::regular(upload_f32(&backend, &authority, vec![2, 1], &[3.0, 4.0])?)?;
        assert!(value.can_concat(&other));
        let concatenated = value.concat(&[other], &backend, &context)?;
        let ConditioningValue::Regular(concatenated) = concatenated else {
            return Err("concatenated regular value changed kind".into());
        };
        assert_eq!(concatenated.descriptor().shape(), &[4, 1]);
        assert_eq!(tensor_values(&concatenated)?, [1.0, 2.0, 3.0, 4.0]);
        Ok(())
    }

    #[test]
    fn noise_shape_narrows_every_spatial_axis_before_batch_repeat() -> TestResult {
        let (backend, authority) = runtime()?;
        let values = (0..20).map(|value| value as f32).collect::<Vec<_>>();
        let tensor = upload_f32(&backend, &authority, vec![1, 1, 4, 5], &values)?;
        let value = ConditioningValue::noise_shape(tensor)?;
        let region = ConditioningRegion::absolute(vec![2, 3], vec![1, 1])?.resolve(&[4, 5])?;
        let cancellation = comfy_types::CancellationToken::default();
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(2 * 2 * 3 * 4)?,
            &cancellation,
        );
        let processed = value.process(2, Some(&region), &backend, &context)?;
        let ConditioningValue::NoiseShape(processed) = processed else {
            return Err("noise-shape value changed kind".into());
        };
        assert_eq!(processed.descriptor().shape(), &[2, 1, 2, 3]);
        assert_eq!(
            tensor_values(&processed)?,
            [
                6.0, 7.0, 8.0, 11.0, 12.0, 13.0, 6.0, 7.0, 8.0, 11.0, 12.0, 13.0
            ]
        );
        Ok(())
    }

    #[test]
    fn cross_attention_uses_checked_lcm_padding_and_caps_every_source_ratio() -> TestResult {
        let (backend, authority) = runtime()?;
        let left = ConditioningValue::cross_attention(upload_f32(
            &backend,
            &authority,
            vec![1, 2, 1],
            &[1.0, 2.0],
        )?)?;
        let right = ConditioningValue::cross_attention(upload_f32(
            &backend,
            &authority,
            vec![1, 3, 1],
            &[3.0, 4.0, 5.0],
        )?)?;
        assert!(left.can_concat(&right));
        let cancellation = comfy_types::CancellationToken::default();
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(6 * 4)?,
            &cancellation,
        );
        let output = left.concat(&[right], &backend, &context)?;
        let ConditioningValue::CrossAttention(output) = output else {
            return Err("cross-attention value changed kind".into());
        };
        assert_eq!(output.descriptor().shape(), &[2, 6, 1]);
        assert_eq!(
            tensor_values(&output)?,
            [1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 3.0, 4.0, 5.0, 3.0, 4.0, 5.0]
        );

        let too_long = ConditioningValue::cross_attention(upload_f32(
            &backend,
            &authority,
            vec![1, 10, 1],
            &[0.0; 10],
        )?)?;
        assert!(!left.can_concat(&too_long));
        assert!(left.concat(&[too_long], &backend, &context).is_err());

        let three = ConditioningValue::cross_attention(upload_f32(
            &backend,
            &authority,
            vec![1, 3, 1],
            &[0.0; 3],
        )?)?;
        let four = ConditioningValue::cross_attention(upload_f32(
            &backend,
            &authority,
            vec![1, 4, 1],
            &[0.0; 4],
        )?)?;
        assert!(left.can_concat(&three));
        assert!(left.can_concat(&four));
        assert!(left.concat(&[three, four], &backend, &context).is_err());
        Ok(())
    }

    #[test]
    fn constants_and_lists_preserve_comfy_equality_and_itemwise_concat_contracts() -> TestResult {
        let (backend, authority) = runtime()?;
        assert!(ConditioningConstant::finite_f64(f64::INFINITY).is_err());
        let nullable = ConditioningValue::constant(ConditioningConstant::Null)?;
        assert!(nullable.can_concat(&ConditioningValue::constant(ConditioningConstant::Null)?));
        let mut too_deep = ConditioningConstant::Null;
        for _ in 0..MAX_CONDITIONING_CONSTANT_DEPTH {
            too_deep = ConditioningConstant::List(vec![too_deep]);
        }
        assert!(ConditioningValue::constant(too_deep).is_err());
        assert!(
            ConditioningValue::constant(ConditioningConstant::List(vec![
                ConditioningConstant::Null;
                MAX_CONDITIONING_CONSTANT_ITEMS
                    + 1
            ]))
            .is_err()
        );
        let constant = ConditioningValue::constant(ConditioningConstant::finite_f64(0.5)?)?;
        let same = ConditioningValue::constant(ConditioningConstant::finite_f64(0.5)?)?;
        let different = ConditioningValue::constant(ConditioningConstant::finite_f64(0.25)?)?;
        assert!(constant.can_concat(&same));
        assert!(!constant.can_concat(&different));
        let tensor_constant = ConditioningValue::constant(ConditioningConstant::Tensor(
            upload_f32(&backend, &authority, vec![1, 2], &[1.0, 2.0])?,
        ))?;
        let same_tensor_constant = ConditioningValue::constant(ConditioningConstant::Tensor(
            upload_f32(&backend, &authority, vec![1, 2], &[1.0, 2.0])?,
        ))?;
        let different_tensor_constant = ConditioningValue::constant(ConditioningConstant::Tensor(
            upload_f32(&backend, &authority, vec![1, 2], &[1.0, 3.0])?,
        ))?;
        assert!(tensor_constant.can_concat(&same_tensor_constant));
        assert!(!tensor_constant.can_concat(&different_tensor_constant));
        assert_eq!(
            tensor_constant.deterministic_digest(&comfy_types::CancellationToken::default())?,
            same_tensor_constant.deterministic_digest(&comfy_types::CancellationToken::default())?
        );
        let list_constant = ConditioningValue::constant(ConditioningConstant::List(vec![
            ConditioningConstant::Signed(1),
        ]))?;
        let tuple_constant = ConditioningValue::constant(ConditioningConstant::Tuple(vec![
            ConditioningConstant::Signed(1),
        ]))?;
        assert!(!list_constant.can_concat(&tuple_constant));
        assert_ne!(
            list_constant.deterministic_digest(&comfy_types::CancellationToken::default())?,
            tuple_constant.deterministic_digest(&comfy_types::CancellationToken::default())?
        );
        assert!(
            ConditioningValue::constant(ConditioningConstant::Map(BTreeMap::from([(
                "arbitrary map key with spaces".into(),
                ConditioningConstant::Null,
            ),])))
            .is_ok()
        );
        let cancellation = comfy_types::CancellationToken::default();
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(32)?,
            &cancellation,
        );
        assert!(matches!(
            constant.concat(&[same], &backend, &context)?,
            ConditioningValue::Constant(_)
        ));
        assert!(constant.concat(&[different], &backend, &context).is_err());

        let list = ConditioningValue::list(vec![
            upload_f32(&backend, &authority, vec![1, 2], &[1.0, 2.0])?,
            upload_f32(&backend, &authority, vec![1, 1], &[3.0])?,
        ])?;
        let other = ConditioningValue::list(vec![
            upload_f32(&backend, &authority, vec![1, 2], &[4.0, 5.0])?,
            upload_f32(&backend, &authority, vec![1, 1], &[6.0])?,
        ])?;
        assert!(list.can_concat(&other));
        let concatenated = list.concat(&[other], &backend, &context)?;
        let ConditioningValue::List(items) = concatenated else {
            return Err("conditioning list changed kind".into());
        };
        assert_eq!(items[0].descriptor().shape(), &[2, 2]);
        assert_eq!(tensor_values(&items[0])?, [1.0, 2.0, 4.0, 5.0]);
        assert_eq!(tensor_values(&items[1])?, [3.0, 6.0]);
        Ok(())
    }

    #[test]
    fn multidimensional_regions_resolve_percentage_rounding_clamping_and_bounds() -> TestResult {
        let percentage = ConditioningRegion::percentage(vec![0.5, 0.3, 1.0], vec![0.25, 0.8, 0.0])?;
        let resolved = percentage.resolve(&[8, 10, 3])?;
        assert_eq!(resolved.sizes(), &[4, 2, 3]);
        assert_eq!(resolved.offsets(), &[2, 8, 0]);
        assert!(!resolved.is_full());

        let ties_to_even =
            ConditioningRegion::percentage(vec![0.25], vec![0.25])?.resolve(&[10])?;
        assert_eq!(ties_to_even.sizes(), &[2]);
        assert_eq!(ties_to_even.offsets(), &[2]);

        let absolute = ConditioningRegion::absolute(vec![100, 2], vec![3, 2])?;
        let resolved = absolute.resolve(&[5, 5])?;
        assert_eq!(resolved.sizes(), &[2, 2]);
        assert_eq!(resolved.offsets(), &[3, 2]);
        assert!(ConditioningRegion::absolute(vec![0], vec![0]).is_err());
        assert!(ConditioningRegion::percentage(vec![1.1], vec![0.0]).is_err());
        assert!(ConditioningRegion::percentage(vec![0.5], vec![1.0]).is_err());
        assert!(absolute.resolve(&[5]).is_err());
        let unchecked_invalid = ConditioningRegion::Percentage {
            sizes: vec![f64::NAN],
            offsets: vec![0.0],
        };
        assert!(unchecked_invalid.resolve(&[5]).is_err());
        Ok(())
    }

    #[test]
    fn masks_resize_crop_broadcast_channels_and_do_not_apply_area_feathering() -> TestResult {
        let (backend, authority) = runtime()?;
        let values = (0..25).map(|value| value as f32).collect::<Vec<_>>();
        let mask_tensor = upload_f32(&backend, &authority, vec![5, 5], &values)?;
        assert!(ConditioningMask::new(mask_tensor.clone(), 0.8, vec![1, 1], false).is_err());
        let mask = ConditioningMask::new(mask_tensor, 0.8, vec![0, 0], false)?;
        let target = TensorDescriptor::contiguous(
            vec![2, 3, 5, 5],
            DType::F32,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let region = ConditioningRegion::absolute(vec![3, 3], vec![1, 1])?.resolve(&[5, 5])?;
        let cancellation = comfy_types::CancellationToken::default();
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(2 * 3 * 3 * 3 * 4)?,
            &cancellation,
        );
        let resolved = mask.resolve(&target, &region, &backend, &context)?;
        assert_eq!(resolved.tensor().descriptor().shape(), &[2, 3, 3, 3]);
        assert_eq!(resolved.feather_weight(&[0, 0])?, 0.8);
        assert_eq!(resolved.feather_weight(&[1, 1])?, 0.8);
        assert!(resolved.feather_weight(&[3, 0]).is_err());
        let values = tensor_values(resolved.tensor())?;
        assert_eq!(
            &values[..9],
            &[6.0, 7.0, 8.0, 11.0, 12.0, 13.0, 16.0, 17.0, 18.0]
        );
        assert_eq!(&values[9..18], &values[..9]);
        assert_eq!(&values[27..36], &values[..9]);
        Ok(())
    }

    #[test]
    fn masks_use_source_bilinear_resize_before_crop_and_broadcast() -> TestResult {
        let (backend, authority) = runtime()?;
        let mask = ConditioningMask::new(
            upload_f32(&backend, &authority, vec![1, 2, 2], &[0.0, 1.0, 2.0, 3.0])?,
            1.0,
            vec![0, 0],
            false,
        )?;
        let target = TensorDescriptor::contiguous(
            vec![1, 1, 4, 4],
            DType::F32,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let region = ResolvedConditioningRegion::full(vec![4, 4])?;
        let cancellation = comfy_types::CancellationToken::default();
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(1 << 16)?,
            &cancellation,
        );
        let resolved = mask.resolve(&target, &region, &backend, &context)?;
        assert_eq!(resolved.tensor().descriptor().shape(), &[1, 1, 4, 4]);
        assert_eq!(
            tensor_values(resolved.tensor())?,
            [
                0.0, 0.25, 0.75, 1.0, 0.5, 0.75, 1.25, 1.5, 1.5, 1.75, 2.25, 2.5, 2.0, 2.25, 2.75,
                3.0,
            ]
        );
        let f16_source = upload_f32(&backend, &authority, vec![1, 2, 2], &[0.0, 1.0, 2.0, 3.0])?;
        let f16_source = cast_to_with_context_exact_native(
            &backend,
            &f16_source,
            DType::F16,
            DeviceId::CPU,
            false,
            false,
            &context,
        )?;
        let f16_mask = ConditioningMask::new(f16_source, 1.0, vec![0, 0], false)?;
        let f16_resolved = f16_mask.resolve(&target, &region, &backend, &context)?;
        assert_eq!(f16_resolved.tensor().descriptor().dtype(), DType::F16);
        assert_eq!(
            tensor_values(f16_resolved.tensor())?,
            tensor_values(resolved.tensor())?
        );
        Ok(())
    }

    #[test]
    fn area_only_feather_and_default_subtraction_match_source_weights() -> TestResult {
        let (backend, authority) = runtime()?;
        let entry = ConditioningEntry::checked_with_references(
            "area",
            ConditioningValue::constant(ConditioningConstant::Null)?,
            ConditioningEntryOptions {
                region: Some(ConditioningRegion::absolute(vec![8], vec![2])?),
                ..ConditioningEntryOptions::default()
            },
            ConditioningReferences::checked(
                Some(ConditioningControlReference::checked("control.area")?),
                vec![ConditioningHookReference::checked("hook.area")?],
            )?,
        )?;
        let target = TensorDescriptor::contiguous(
            vec![1, 1, 12],
            DType::F32,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let cancellation = comfy_types::CancellationToken::default();
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(0)?,
            &cancellation,
        );
        let resolved = entry.resolve(&target, &backend, &context)?;
        assert_eq!(resolved.contribution_weight(&[0], None)?, 0.5);
        assert_eq!(resolved.contribution_weight(&[1], None)?, 1.0);
        assert_eq!(resolved.contribution_weight(&[6], None)?, 1.0);
        assert_eq!(resolved.contribution_weight(&[7], None)?, 0.5);
        assert_eq!(
            resolved
                .references()
                .control()
                .map(|reference| reference.identifier()),
            Some("control.area")
        );
        assert_eq!(default_region_residual_weight(0.0)?, 1.0);
        assert_eq!(default_region_residual_weight(0.25)?, 0.75);
        assert_eq!(default_region_residual_weight(1.0)?, 0.0);
        assert_eq!(default_region_residual_weight(2.0)?, 0.0);
        assert_eq!(default_region_residual_weight(-0.5)?, 1.5);
        assert!(default_region_residual_weight(f32::NAN).is_err());
        Ok(())
    }

    #[test]
    fn nonzero_mask_bounds_drive_region_and_default_regions_are_explicit_markers() -> TestResult {
        let (backend, authority) = runtime()?;
        let mut mask_values = vec![0.0_f32; 20];
        mask_values[7] = 1.0;
        mask_values[13] = 1.0;
        let mask = ConditioningMask::new(
            upload_f32(&backend, &authority, vec![1, 4, 5], &mask_values)?,
            1.0,
            vec![0, 0],
            true,
        )?;
        let noise = ConditioningValue::noise_shape(upload_f32(
            &backend,
            &authority,
            vec![1, 1, 4, 5],
            &(0..20).map(|value| value as f32).collect::<Vec<_>>(),
        )?)?;
        let entry = ConditioningEntry::checked(
            "bounded",
            noise,
            ConditioningEntryOptions {
                mask: Some(mask),
                ..ConditioningEntryOptions::default()
            },
        )?;
        let target = TensorDescriptor::contiguous(
            vec![1, 1, 4, 5],
            DType::F32,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let cancellation = comfy_types::CancellationToken::default();
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(32)?,
            &cancellation,
        );
        let resolved = entry.resolve(&target, &backend, &context)?;
        assert_eq!(resolved.region().sizes(), &[2, 2]);
        assert_eq!(resolved.region().offsets(), &[1, 2]);
        let ConditioningValue::NoiseShape(value) = resolved.value() else {
            return Err("resolved bounded entry changed kind".into());
        };
        assert_eq!(value.descriptor().shape(), &[1, 1, 2, 2]);
        assert_eq!(tensor_values(value)?, [7.0, 8.0, 12.0, 13.0]);

        let invalid_default = ConditioningEntry::checked(
            "invalid-default",
            ConditioningValue::constant(ConditioningConstant::Boolean(true))?,
            ConditioningEntryOptions {
                region: Some(ConditioningRegion::absolute(vec![1], vec![0])?),
                default_region: true,
                ..ConditioningEntryOptions::default()
            },
        );
        assert!(invalid_default.is_err());
        let default = ConditioningEntry::checked(
            "default",
            ConditioningValue::constant(ConditioningConstant::Boolean(true))?,
            ConditioningEntryOptions {
                default_region: true,
                ..ConditioningEntryOptions::default()
            },
        )?;
        let resolved_default = default.resolve(&target, &backend, &context)?;
        assert!(resolved_default.is_default_region());
        assert!(resolved_default.region().is_full());
        Ok(())
    }

    #[test]
    fn conditioning_sets_have_content_digests_duplicate_guards_and_cancellation() -> TestResult {
        let (backend, authority) = runtime()?;
        let tensor = upload_f32(&backend, &authority, vec![1, 2], &[1.0, 2.0])?;
        let entry = ConditioningEntry::checked(
            "positive",
            ConditioningValue::regular(tensor)?,
            ConditioningEntryOptions::default(),
        )?;
        let cancellation = comfy_types::CancellationToken::default();
        let first = ConditioningSet::checked(identity()?, vec![entry.clone()], &cancellation)?;
        let second = ConditioningSet::checked(identity()?, vec![entry.clone()], &cancellation)?;
        assert_eq!(first.digest(), second.digest());
        assert_eq!(first.digest().len(), 64);

        let source = upload_f32(&backend, &authority, vec![2, 1], &[0.0, 2.0])?;
        let narrowed = narrow_method_exact_native(&source, 0, 1, 1, &cancellation)?;
        let fresh = upload_f32(&backend, &authority, vec![1, 1], &[2.0])?;
        assert_ne!(narrowed.descriptor(), fresh.descriptor());
        assert_eq!(
            ConditioningValue::regular(narrowed)?.deterministic_digest(&cancellation)?,
            ConditioningValue::regular(fresh)?.deterministic_digest(&cancellation)?,
        );
        assert!(
            ConditioningSet::checked(identity()?, vec![entry.clone(), entry], &cancellation,)
                .is_err()
        );

        let cancelled = comfy_types::CancellationToken::default();
        assert!(cancelled.cancel());
        assert!(matches!(
            ConditioningSet::checked(
                identity()?,
                vec![ConditioningEntry::checked(
                    "negative",
                    ConditioningValue::constant(ConditioningConstant::Boolean(false))?,
                    ConditioningEntryOptions::default(),
                )?],
                &cancelled,
            ),
            Err(ConditioningError::Cancelled)
        ));
        Ok(())
    }

    #[test]
    fn cancellation_and_workspace_failures_propagate_without_private_fallbacks() -> TestResult {
        let (backend, authority) = runtime()?;
        let tensor = upload_f32(&backend, &authority, vec![1, 2], &[1.0, 2.0])?;
        let value = ConditioningValue::regular(tensor.clone())?;

        let cancelled = comfy_types::CancellationToken::default();
        assert!(cancelled.cancel());
        let cancelled_context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(40)?,
            &cancelled,
        );
        assert!(matches!(
            value.process(5, None, &backend, &cancelled_context),
            Err(ConditioningError::Cancelled)
        ));

        let cancellation = comfy_types::CancellationToken::default();
        let scratch = authority.authorize_workspace(4)?;
        let context = backend.execution_context(StreamId::DEFAULT, scratch.clone(), &cancellation);
        assert!(matches!(
            value.process(5, None, &backend, &context),
            Err(ConditioningError::ShapeOperation(_))
        ));
        assert_eq!(scratch.in_use_bytes(), 0);
        assert_eq!(tensor_values(&tensor)?, [1.0, 2.0]);
        Ok(())
    }

    #[test]
    fn mask_dtype_and_cross_attention_device_dtype_contracts_fail_closed() -> TestResult {
        let (backend, authority) = runtime()?;
        let cancellation = comfy_types::CancellationToken::default();
        let context = backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(0)?,
            &cancellation,
        );
        let integer_descriptor =
            TensorDescriptor::contiguous(vec![2, 2], DType::I32, DeviceId::CPU, StreamId::DEFAULT)?;
        let integer_tensor = backend
            .upload_bytes(integer_descriptor, &[0; 16], &context)?
            .0;
        assert!(ConditioningMask::new(integer_tensor, 1.0, vec![0, 0], false).is_err());

        let f32_value = ConditioningValue::cross_attention(upload_f32(
            &backend,
            &authority,
            vec![1, 2, 1],
            &[0.0, 1.0],
        )?)?;
        let f16_descriptor = TensorDescriptor::contiguous(
            vec![1, 2, 1],
            DType::F16,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let f16_tensor = backend.upload_bytes(f16_descriptor, &[0; 4], &context)?.0;
        let f16_value = ConditioningValue::cross_attention(f16_tensor)?;
        assert!(!f32_value.can_concat(&f16_value));
        Ok(())
    }
}
