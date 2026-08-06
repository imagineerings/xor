use crate::{
    LatentFormatIdentity, MODEL_DESCRIPTOR_SCHEMA_VERSION, MemoryEstimatorDescriptor,
    ModelComponentDescriptor, ModelDescriptor, ModelDescriptorError, TensorKeyRule,
    native_ops::{NativeModule, NativeOpsError, conv1d_module_exact_native},
};
use comfy_tensor::{
    CpuBackend, DType, DecodedScalar, DeviceId, ExecutionContext, Layout, MemoryFormatReference,
    Scalar, Tensor, TensorError,
    generated_comfy_operator_indirection_01::{
        OperatorIndirectionError, cast_to_with_context_exact_native,
        tensor_from_f32_with_context_exact_native, tensor_to_f32_with_context_exact_native,
    },
    generated_elementwise_or_runtime_operation_03::ElementwiseOperand,
    generated_elementwise_or_runtime_operation_06::{
        ElementwiseRuntimePartSixError, round_method_with_context_exact_native,
    },
    generated_elementwise_or_runtime_operation_08::{
        ElementwiseRuntimePartEightError, concatenate_with_context_exact_native,
    },
    generated_elementwise_or_runtime_operation_16::{
        ElementwiseRuntimePartSixteenError, add_method_with_context_exact_native,
        mul_method_with_context_exact_native,
    },
    generated_elementwise_or_runtime_operation_17::{
        ElementwiseRuntimePartSeventeenError, TensorSplitSpec, tensor_split_exact_native,
    },
    generated_external_tensor_kernel_01::ExternalTensorKernelPartOneError,
    generated_indexing_masking_01::{IndexingMaskingPartOneError, narrow_method_exact_native},
    generated_neural_network_module_02::{
        NeuralNetworkModulePartTwoError, multihead_attention_projected_with_context_exact_native,
    },
    generated_random_number_generation_01::RandomNumberGenerationPartOneError,
    generated_reduction_01::{ReductionPartOneError, torch_std_with_context_exact_native},
    generated_shape_layout_transform_01::{
        ShapeLayoutTransformPartOneError, tensor_expand_exact_native,
    },
    generated_shape_layout_transform_02::{
        ShapeLayoutTransformPartTwoError, tensor_reshape_with_context_exact_native,
    },
    generated_shape_layout_transform_03::{
        ShapeLayoutTransformPartThreeError, tensor_permute_exact_native,
        tensor_transpose_exact_native,
    },
    generated_storage_dtype_device_01::{
        StorageDTypeDeviceError, contiguous_with_context_exact_native,
    },
    generated_tensor_creation_01::arange_with_context_exact_native,
    generated_tensor_creation_01::{TensorCreationPartOneError, full_with_context_exact_native},
};
use comfy_types::{CancellationToken, DeviceKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use thiserror::Error;

pub const MODEL_FAMILY_SCHEMA_VERSION: u16 = 1;
pub const MAX_MODEL_WEIGHT_STATISTIC_REQUESTS: usize = 16;
const MAX_IDENTITY_BYTES: usize = 1024;
const MAX_STATE_DICTIONARY_OPERATIONS: usize = 4_096;
const MAX_STATE_DICTIONARY_MATCHES: usize = 16_384;
const MAX_STATE_DICTIONARY_SOURCE_TENSORS: usize = 16_384;
const MAX_DIMENSION_EXPRESSION_DEPTH: usize = 32;
const MAX_TENSOR_RANK: usize = 32;
const MAX_CLIP_TARGET_CANDIDATES: usize = 16;
const MAX_CLIP_CONFIGURATION_FACTS: usize = 64;
const MAX_MODEL_DETECTION_KEY_ALTERNATIVES: usize = 16;
const MAX_MODEL_DETECTION_DIMENSION_VALUES: usize = 16;
const MAX_MODEL_DETECTION_TENSOR_PREDICATES: usize = 16;
const MAX_MODEL_LAYOUT_SIGNATURES: usize = 3;
const MAX_MODEL_LAYOUT_SIGNATURE_FACTS: usize = 16;
const MAX_MODEL_PROBE_TENSORS: usize = 1_000_000;
const MAX_MODEL_PROBE_FORMATS: usize = 64;
const MAX_MODEL_PROBE_NAME_BYTES: usize = 64 * 1024;
const MAX_MODEL_PROBE_METADATA_BYTES: usize = 16 * 1024 * 1024;
const MODEL_PROBE_INTERNAL_PREFIX: &str = "__sim.model_probe.v1.";
const MODEL_PROBE_FORMAT_PREFIX: &str = "__sim.model_probe.v1.format.";
const MODEL_PROBE_DTYPE_PREFIX: &str = "__sim.model_probe.v1.dtype.";
const MODEL_PROBE_UNET_PREFIX: &str = "__sim.model_probe.v1.unet_prefix";
const UNET_PREFIX_CANDIDATES: [&str; 3] = ["model.diffusion_model.", "model.model.", "net."];

pub const MODEL_CLIP_TARGET_SCHEMA_VERSION: u16 = 2;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModelNativeTargetIdentifier(String);

impl ModelNativeTargetIdentifier {
    pub fn checked(identifier: impl Into<String>) -> Result<Self, ModelFamilyError> {
        let identifier = identifier.into();
        validate_qualified_model_symbol("native target", &identifier)?;
        Ok(Self(identifier))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for ModelNativeTargetIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModelNativeTargetIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::checked(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelTokenizerDescriptor {
    identifier: ModelNativeTargetIdentifier,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelTokenizerDescriptorWire {
    schema_version: u16,
    identifier: ModelNativeTargetIdentifier,
}

impl ModelTokenizerDescriptor {
    pub fn checked(identifier: impl Into<String>) -> Result<Self, ModelFamilyError> {
        Ok(Self {
            identifier: ModelNativeTargetIdentifier::checked(identifier)?,
        })
    }

    pub fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    pub fn target(&self) -> &ModelNativeTargetIdentifier {
        &self.identifier
    }
}

impl Serialize for ModelTokenizerDescriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ModelTokenizerDescriptorWire {
            schema_version: MODEL_CLIP_TARGET_SCHEMA_VERSION,
            identifier: self.identifier.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModelTokenizerDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ModelTokenizerDescriptorWire::deserialize(deserializer)?;
        if wire.schema_version != MODEL_CLIP_TARGET_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(ModelFamilyError::SchemaVersion(
                wire.schema_version,
            )));
        }
        Self::checked(wire.identifier.0).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ModelClipConfigurationFact {
    Bind {
        parameter: ModelNativeTargetIdentifier,
        source: ModelNativeTargetIdentifier,
    },
    Expand {
        source: ModelNativeTargetIdentifier,
    },
}

impl ModelClipConfigurationFact {
    pub fn bind(
        parameter: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<Self, ModelFamilyError> {
        Ok(Self::Bind {
            parameter: ModelNativeTargetIdentifier::checked(parameter)?,
            source: ModelNativeTargetIdentifier::checked(source)?,
        })
    }

    pub fn expand(source: impl Into<String>) -> Result<Self, ModelFamilyError> {
        Ok(Self::Expand {
            source: ModelNativeTargetIdentifier::checked(source)?,
        })
    }

    fn validate(&self) -> Result<(), ModelFamilyError> {
        match self {
            Self::Bind { parameter, source } => {
                validate_qualified_model_symbol(
                    "CLIP configuration parameter",
                    parameter.as_str(),
                )?;
                validate_qualified_model_symbol("CLIP configuration source", source.as_str())
            }
            Self::Expand { source } => {
                validate_qualified_model_symbol("CLIP configuration expansion", source.as_str())
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ModelClipModelInvocation {
    Reference,
    Factory {
        configuration: Vec<ModelClipConfigurationFact>,
    },
}

impl ModelClipModelInvocation {
    fn validate(&self) -> Result<(), ModelFamilyError> {
        let Self::Factory { configuration } = self else {
            return Ok(());
        };
        if configuration.len() > MAX_CLIP_CONFIGURATION_FACTS {
            return Err(ModelFamilyError::InvalidClipTarget(format!(
                "CLIP model configuration has {} facts, exceeding {MAX_CLIP_CONFIGURATION_FACTS}",
                configuration.len()
            )));
        }
        let mut parameters = BTreeSet::new();
        let mut expansions = BTreeSet::new();
        for fact in configuration {
            fact.validate()?;
            match fact {
                ModelClipConfigurationFact::Bind { parameter, .. } => {
                    if !parameters.insert(parameter.as_str()) {
                        return Err(ModelFamilyError::InvalidClipTarget(format!(
                            "CLIP model configuration repeats parameter {}",
                            parameter.as_str()
                        )));
                    }
                }
                ModelClipConfigurationFact::Expand { source } => {
                    if !expansions.insert(source.as_str()) {
                        return Err(ModelFamilyError::InvalidClipTarget(format!(
                            "CLIP model configuration repeats expansion {}",
                            source.as_str()
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelClipModelDescriptor {
    target: ModelNativeTargetIdentifier,
    invocation: ModelClipModelInvocation,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelClipModelDescriptorWire {
    schema_version: u16,
    target: ModelNativeTargetIdentifier,
    invocation: ModelClipModelInvocation,
}

impl ModelClipModelDescriptor {
    pub fn checked(
        target: impl Into<String>,
        invocation: ModelClipModelInvocation,
    ) -> Result<Self, ModelFamilyError> {
        let target = ModelNativeTargetIdentifier::checked(target)?;
        invocation.validate()?;
        Ok(Self { target, invocation })
    }

    pub fn target(&self) -> &ModelNativeTargetIdentifier {
        &self.target
    }

    pub fn invocation(&self) -> &ModelClipModelInvocation {
        &self.invocation
    }
}

impl Serialize for ModelClipModelDescriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ModelClipModelDescriptorWire {
            schema_version: MODEL_CLIP_TARGET_SCHEMA_VERSION,
            target: self.target.clone(),
            invocation: self.invocation.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModelClipModelDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ModelClipModelDescriptorWire::deserialize(deserializer)?;
        if wire.schema_version != MODEL_CLIP_TARGET_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(ModelFamilyError::SchemaVersion(
                wire.schema_version,
            )));
        }
        Self::checked(wire.target.0, wire.invocation).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelClipTargetCandidateDescriptor {
    tokenizer: ModelTokenizerDescriptor,
    clip_model: ModelClipModelDescriptor,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelClipTargetCandidateDescriptorWire {
    tokenizer: ModelTokenizerDescriptor,
    clip_model: ModelClipModelDescriptor,
}

impl ModelClipTargetCandidateDescriptor {
    pub fn checked(
        tokenizer: impl Into<String>,
        clip_model: impl Into<String>,
    ) -> Result<Self, ModelFamilyError> {
        Self::checked_with_invocation(tokenizer, clip_model, ModelClipModelInvocation::Reference)
    }

    pub fn checked_with_invocation(
        tokenizer: impl Into<String>,
        clip_model: impl Into<String>,
        invocation: ModelClipModelInvocation,
    ) -> Result<Self, ModelFamilyError> {
        Ok(Self {
            tokenizer: ModelTokenizerDescriptor::checked(tokenizer)?,
            clip_model: ModelClipModelDescriptor::checked(clip_model, invocation)?,
        })
    }

    pub fn tokenizer(&self) -> &ModelTokenizerDescriptor {
        &self.tokenizer
    }

    pub fn clip_model(&self) -> &ModelClipModelDescriptor {
        &self.clip_model
    }
}

impl Serialize for ModelClipTargetCandidateDescriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ModelClipTargetCandidateDescriptorWire {
            tokenizer: self.tokenizer.clone(),
            clip_model: self.clip_model.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModelClipTargetCandidateDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ModelClipTargetCandidateDescriptorWire::deserialize(deserializer)?;
        Self::checked_with_invocation(
            wire.tokenizer.identifier.0,
            wire.clip_model.target.0,
            wire.clip_model.invocation,
        )
        .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelClipTargetDescriptor {
    candidates: Vec<ModelClipTargetCandidateDescriptor>,
    dynamic_selection: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelClipTargetDescriptorWire {
    schema_version: u16,
    candidates: Vec<ModelClipTargetCandidateDescriptor>,
    dynamic_selection: bool,
}

impl ModelClipTargetDescriptor {
    pub fn checked(
        candidates: Vec<ModelClipTargetCandidateDescriptor>,
        dynamic_selection: bool,
    ) -> Result<Self, ModelFamilyError> {
        if candidates.len() > MAX_CLIP_TARGET_CANDIDATES {
            return Err(ModelFamilyError::InvalidClipTarget(format!(
                "CLIP target has {} candidates, exceeding {MAX_CLIP_TARGET_CANDIDATES}",
                candidates.len()
            )));
        }
        if dynamic_selection && candidates.is_empty() {
            return Err(ModelFamilyError::InvalidClipTarget(
                "dynamic CLIP target selection requires at least one candidate".to_owned(),
            ));
        }
        let mut identities = BTreeSet::new();
        for candidate in &candidates {
            validate_qualified_model_symbol("tokenizer", candidate.tokenizer.identifier())?;
            validate_qualified_model_symbol("CLIP model", candidate.clip_model.target().as_str())?;
            candidate.clip_model.invocation().validate()?;
            if !identities.insert((
                candidate.tokenizer.identifier(),
                candidate.clip_model.target().as_str(),
                candidate.clip_model.invocation(),
            )) {
                return Err(ModelFamilyError::InvalidClipTarget(
                    "CLIP target repeats a tokenizer/model candidate".to_owned(),
                ));
            }
        }
        Ok(Self {
            candidates,
            dynamic_selection,
        })
    }

    pub fn candidates(&self) -> &[ModelClipTargetCandidateDescriptor] {
        &self.candidates
    }

    pub fn dynamic_selection(&self) -> bool {
        self.dynamic_selection
    }
}

impl Serialize for ModelClipTargetDescriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ModelClipTargetDescriptorWire {
            schema_version: MODEL_CLIP_TARGET_SCHEMA_VERSION,
            candidates: self.candidates.clone(),
            dynamic_selection: self.dynamic_selection,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModelClipTargetDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ModelClipTargetDescriptorWire::deserialize(deserializer)?;
        if wire.schema_version != MODEL_CLIP_TARGET_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(ModelFamilyError::SchemaVersion(
                wire.schema_version,
            )));
        }
        Self::checked(wire.candidates, wire.dynamic_selection).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelClipConfigurationFactDefinition {
    Bind {
        parameter: &'static str,
        source: &'static str,
    },
    Expand {
        source: &'static str,
    },
}

impl ModelClipConfigurationFactDefinition {
    fn compile(self) -> Result<ModelClipConfigurationFact, ModelFamilyError> {
        match self {
            Self::Bind { parameter, source } => ModelClipConfigurationFact::bind(parameter, source),
            Self::Expand { source } => ModelClipConfigurationFact::expand(source),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelClipModelInvocationDefinition {
    Reference,
    Factory {
        configuration: &'static [ModelClipConfigurationFactDefinition],
    },
}

impl ModelClipModelInvocationDefinition {
    fn compile(self) -> Result<ModelClipModelInvocation, ModelFamilyError> {
        match self {
            Self::Reference => Ok(ModelClipModelInvocation::Reference),
            Self::Factory { configuration } => Ok(ModelClipModelInvocation::Factory {
                configuration: configuration
                    .iter()
                    .copied()
                    .map(ModelClipConfigurationFactDefinition::compile)
                    .collect::<Result<Vec<_>, _>>()?,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelClipTargetCandidateDefinition {
    pub tokenizer: &'static str,
    pub clip_model: &'static str,
    pub invocation: ModelClipModelInvocationDefinition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelClipTargetDefinition {
    pub candidates: &'static [ModelClipTargetCandidateDefinition],
    pub dynamic_selection: bool,
}

impl ModelClipTargetDefinition {
    fn compile(self) -> Result<ModelClipTargetDescriptor, ModelFamilyError> {
        ModelClipTargetDescriptor::checked(
            self.candidates
                .iter()
                .map(|candidate| {
                    ModelClipTargetCandidateDescriptor::checked_with_invocation(
                        candidate.tokenizer,
                        candidate.clip_model,
                        candidate.invocation.compile()?,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
            self.dynamic_selection,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ModelClipTargetCase {
    pub metadata_value: &'static str,
    pub target: &'static ModelClipTargetDefinition,
}

#[derive(Clone, Copy, Debug)]
pub enum ModelClipTargetSelector {
    Profile,
    Static(&'static ModelClipTargetDefinition),
    Metadata {
        key: &'static str,
        cases: &'static [ModelClipTargetCase],
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelFamilyComponent {
    pub identifier: &'static str,
    pub role: &'static str,
    pub required: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelWeightRule {
    pub source_prefix: &'static str,
    pub target_prefix: &'static str,
    pub required: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelTensorFactSubject {
    Rank,
    Dimension(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelTensorFactRelation {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelTensorFactPredicate {
    pub subject: ModelTensorFactSubject,
    pub relation: ModelTensorFactRelation,
    pub value: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelDetectionRule {
    ExactShape {
        key: &'static str,
        shape: &'static [u64],
        score: u32,
    },
    KeyPresent {
        key: &'static str,
        score: u32,
    },
    AnyKeyPresent {
        keys: &'static [&'static str],
        score: u32,
    },
    AnyTensorDimensionValue {
        keys: &'static [&'static str],
        dimension: usize,
        values: &'static [u64],
        score: u32,
    },
    AnyTensorFact {
        keys: &'static [&'static str],
        predicates: &'static [ModelTensorFactPredicate],
        score: u32,
    },
    KeyPrefix {
        prefix: &'static str,
        minimum_matches: usize,
        score: u32,
    },
    Metadata {
        key: &'static str,
        value: &'static str,
        score: u32,
    },
}

impl ModelDetectionRule {
    fn score(self) -> u32 {
        match self {
            Self::ExactShape { score, .. }
            | Self::KeyPresent { score, .. }
            | Self::AnyKeyPresent { score, .. }
            | Self::AnyTensorDimensionValue { score, .. }
            | Self::AnyTensorFact { score, .. }
            | Self::KeyPrefix { score, .. }
            | Self::Metadata { score, .. } => score,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModelForwardOperation {
    AddWeight(&'static str),
    MultiplyWeight(&'static str),
    AddScalar(f32),
    MultiplyScalar(f32),
    Linear {
        weight: &'static str,
        bias: Option<&'static str>,
        input_features: usize,
        output_features: usize,
    },
    Convolution1d {
        weight: &'static str,
        bias: Option<&'static str>,
        input_channels: usize,
        output_channels: usize,
        kernel_size: usize,
        stride: usize,
        padding: usize,
        dilation: usize,
        groups: usize,
    },
    Convolution2d {
        weight: &'static str,
        bias: Option<&'static str>,
        input_channels: usize,
        output_channels: usize,
        kernel_size: [usize; 2],
        stride: [usize; 2],
        padding: [usize; 2],
        dilation: [usize; 2],
        groups: usize,
    },
    Convolution3d {
        weight: &'static str,
        bias: Option<&'static str>,
        input_channels: usize,
        output_channels: usize,
        kernel_size: [usize; 3],
        stride: [usize; 3],
        padding: [usize; 3],
        dilation: [usize; 3],
        groups: usize,
    },
    LayerNorm {
        normalized_shape: &'static [usize],
        weight: Option<&'static str>,
        bias: Option<&'static str>,
        epsilon: f32,
    },
    SelfAttention {
        heads: usize,
    },
    Silu,
    Tanh,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelForwardStep {
    pub checkpoint: &'static str,
    pub operation: ModelForwardOperation,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelFamilyDefinition {
    pub feature_id: &'static str,
    pub identifier: &'static str,
    pub architecture_version: &'static str,
    pub latent_feature_id: &'static str,
    pub latent_identifier: &'static str,
    pub clip_target: &'static ModelClipTargetDefinition,
    pub components: &'static [ModelFamilyComponent],
    pub detection_rules: &'static [ModelDetectionRule],
    pub weight_rules: &'static [ModelWeightRule],
    pub required_keys: &'static [&'static str],
    pub optional_keys: &'static [&'static str],
    pub supported_dtypes: &'static [DType],
    pub supported_devices: &'static [DeviceKind],
    pub memory_estimator: MemoryEstimatorDescriptor,
    pub forward_program: &'static [ModelForwardStep],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelSourceConfigurationRule {
    Metadata {
        key: &'static str,
        value: &'static str,
    },
    ExactTensorShape {
        key: &'static str,
        shape: &'static [u64],
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ModelFamilyProfile {
    pub latent_feature_id: &'static str,
    pub latent_identifier: &'static str,
    pub clip_target: &'static ModelClipTargetDefinition,
    pub supported_dtypes: &'static [DType],
    pub supported_devices: &'static [DeviceKind],
    pub memory_estimator: MemoryEstimatorDescriptor,
    pub forward_program: &'static [ModelForwardStep],
}

impl ModelFamilyProfile {
    pub const fn from_definition(definition: &'static ModelFamilyDefinition) -> Self {
        Self {
            latent_feature_id: definition.latent_feature_id,
            latent_identifier: definition.latent_identifier,
            clip_target: definition.clip_target,
            supported_dtypes: definition.supported_dtypes,
            supported_devices: definition.supported_devices,
            memory_estimator: definition.memory_estimator,
            forward_program: definition.forward_program,
        }
    }
}

pub type ModelFamilyProfileSelector =
    fn(&ModelProbe) -> Result<ModelFamilyProfile, ModelFamilyError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelWeightStatistic {
    PopulationStandardDeviation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelWeightStatisticRequest {
    tensor_name: String,
    statistic: ModelWeightStatistic,
    device: DeviceKind,
}

impl ModelWeightStatisticRequest {
    pub fn population_standard_deviation(
        tensor_name: impl Into<String>,
        device: DeviceKind,
    ) -> Result<Self, ModelFamilyError> {
        let tensor_name = tensor_name.into();
        validate_probe_tensor_name(&tensor_name)?;
        Ok(Self {
            tensor_name,
            statistic: ModelWeightStatistic::PopulationStandardDeviation,
            device,
        })
    }

    pub fn tensor_name(&self) -> &str {
        &self.tensor_name
    }

    pub fn statistic(&self) -> ModelWeightStatistic {
        self.statistic
    }

    pub fn device(&self) -> DeviceKind {
        self.device
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelWeightStatisticObservation {
    tensor_name: String,
    statistic: ModelWeightStatistic,
    value_bits: u64,
}

impl ModelWeightStatisticObservation {
    pub(crate) fn checked(
        request: &ModelWeightStatisticRequest,
        value: f64,
    ) -> Result<Self, ModelFamilyError> {
        validate_probe_tensor_name(request.tensor_name())?;
        if !value.is_finite() {
            return Err(ModelFamilyError::NonFiniteWeightStatistic {
                tensor: request.tensor_name().to_owned(),
                value_bits: value.to_bits(),
            });
        }
        Ok(Self {
            tensor_name: request.tensor_name.clone(),
            statistic: request.statistic,
            value_bits: value.to_bits(),
        })
    }

    pub fn tensor_name(&self) -> &str {
        &self.tensor_name
    }

    pub fn statistic(&self) -> ModelWeightStatistic {
        self.statistic
    }

    pub fn value(&self) -> f64 {
        f64::from_bits(self.value_bits)
    }

    pub fn exceeds_checked(&self, threshold: f64) -> Result<bool, ModelFamilyError> {
        if !threshold.is_finite() {
            return Err(ModelFamilyError::NonFiniteWeightStatisticThreshold(
                threshold.to_bits(),
            ));
        }
        Ok(self.value() > threshold)
    }
}

pub(crate) fn validate_model_weight_statistic_requests(
    requests: &[ModelWeightStatisticRequest],
) -> Result<(), ModelFamilyError> {
    if requests.len() > MAX_MODEL_WEIGHT_STATISTIC_REQUESTS {
        return Err(ModelFamilyError::WeightStatisticRequestLimit {
            actual: requests.len(),
            maximum: MAX_MODEL_WEIGHT_STATISTIC_REQUESTS,
        });
    }
    let mut tensor_names = BTreeSet::new();
    for request in requests {
        validate_probe_tensor_name(request.tensor_name())?;
        if !tensor_names.insert(request.tensor_name()) {
            return Err(ModelFamilyError::DuplicateWeightStatisticRequest(
                request.tensor_name().to_owned(),
            ));
        }
        if request.device() != DeviceKind::Cpu {
            return Err(ModelFamilyError::UnsupportedDevice(request.device()));
        }
    }
    Ok(())
}

pub(crate) fn observe_loaded_model_weight_tensor(
    request: &ModelWeightStatisticRequest,
    backend: &CpuBackend,
    tensor: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<ModelWeightStatisticObservation, ModelFamilyError> {
    context.check()?;
    if tensor.descriptor().device().kind() != request.device() {
        return Err(ModelFamilyError::DeviceMismatch {
            expected: request.device(),
            actual: tensor.descriptor().device().kind(),
        });
    }
    let result = match request.statistic() {
        ModelWeightStatistic::PopulationStandardDeviation => {
            torch_std_with_context_exact_native(backend, tensor, None, 0, false, context)?
        }
    };
    let value = match result
        .descriptor()
        .dtype()
        .decode_scalar(result.contiguous_bytes()?)?
    {
        DecodedScalar::Real(value) => value,
        _ => {
            return Err(ModelFamilyError::InvalidWeightStatisticResult(
                request.tensor_name().to_owned(),
            ));
        }
    };
    context.check()?;
    ModelWeightStatisticObservation::checked(request, value)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ModelStateLayout {
    PrefixedNative,
    StandaloneNative,
    Diffusers,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelLayoutSignature {
    pub layout: ModelStateLayout,
    pub required_keys: &'static [&'static str],
    pub required_prefixes: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
pub struct ModelFamilyStatePlanCase {
    pub layout: ModelStateLayout,
    pub plan: &'static ModelStateTransformPlanDefinition,
}

pub type ModelFamilyStatePlanProbeSelector =
    fn(&ModelProbe) -> Result<ModelStateTransformPlan, ModelFamilyError>;

#[derive(Clone, Copy, Debug)]
pub enum ModelFamilyStatePlanSelector {
    LegacyDefinitionRules,
    Static(&'static ModelStateTransformPlanDefinition),
    Layout {
        signatures: &'static [ModelLayoutSignature],
        cases: &'static [ModelFamilyStatePlanCase],
    },
    Probe(ModelFamilyStatePlanProbeSelector),
}

#[derive(Clone, Copy, Debug)]
pub struct ModelFamilyComponentStateSchema {
    pub component: &'static str,
    pub required_keys: &'static [&'static str],
    pub optional_keys: &'static [&'static str],
    pub allow_unexpected: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelFamilyRegistration {
    pub definition: &'static ModelFamilyDefinition,
    pub source_ordinal: u16,
    pub source_architecture: &'static str,
    pub source_configuration: &'static [ModelSourceConfigurationRule],
    pub required_state_keys: &'static [&'static str],
    pub profile_selector: Option<ModelFamilyProfileSelector>,
    pub clip_target_selector: ModelClipTargetSelector,
    pub state_plan_selector: ModelFamilyStatePlanSelector,
    pub component_state_schemas: &'static [ModelFamilyComponentStateSchema],
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModelFamilyIdentity {
    feature_id: String,
    identifier: String,
    architecture_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelFamilyIdentityWire {
    schema_version: u16,
    feature_id: String,
    identifier: String,
    architecture_version: String,
}

impl ModelFamilyIdentity {
    pub fn new(
        feature_id: impl Into<String>,
        identifier: impl Into<String>,
        architecture_version: impl Into<String>,
    ) -> Result<Self, ModelFamilyError> {
        let feature_id = feature_id.into();
        let identifier = identifier.into();
        let architecture_version = architecture_version.into();
        validate_feature_id(&feature_id)?;
        validate_identifier("identifier", &identifier)?;
        validate_identifier("architecture_version", &architecture_version)?;
        Ok(Self {
            feature_id,
            identifier,
            architecture_version,
        })
    }

    pub fn feature_id(&self) -> &str {
        &self.feature_id
    }
    pub fn identifier(&self) -> &str {
        &self.identifier
    }
    pub fn architecture_version(&self) -> &str {
        &self.architecture_version
    }
}

impl Serialize for ModelFamilyIdentity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ModelFamilyIdentityWire {
            schema_version: MODEL_FAMILY_SCHEMA_VERSION,
            feature_id: self.feature_id.clone(),
            identifier: self.identifier.clone(),
            architecture_version: self.architecture_version.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModelFamilyIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ModelFamilyIdentityWire::deserialize(deserializer)?;
        if wire.schema_version != MODEL_FAMILY_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(ModelFamilyError::SchemaVersion(
                wire.schema_version,
            )));
        }
        Self::new(wire.feature_id, wire.identifier, wire.architecture_version)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelProbe {
    pub tensor_shapes: BTreeMap<String, Vec<u64>>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelParsedTensorFact {
    pub shape: Vec<u64>,
    pub storage_dtype: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelParsedFormatFact {
    pub identity: String,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelParsedFacts {
    pub tensors: BTreeMap<String, ModelParsedTensorFact>,
    pub formats: Vec<ModelParsedFormatFact>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelStorageDType {
    Tensor(DType),
    Ggml(u32),
    TorchQInt8,
    TorchQUInt8,
    TorchQInt32,
}

impl ModelStorageDType {
    pub fn normalized_name(self) -> String {
        match self {
            Self::Tensor(dtype) => dtype.catalog_name().to_owned(),
            Self::Ggml(identifier) => format!("ggml_type_{identifier}"),
            Self::TorchQInt8 => "torch_qint8".to_owned(),
            Self::TorchQUInt8 => "torch_quint8".to_owned(),
            Self::TorchQInt32 => "torch_qint32".to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelConfigurationKind {
    Native,
    Diffusers,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelConfigurationValue {
    None,
    Boolean(bool),
    Signed(i64),
    Unsigned(u64),
    FloatBits(u64),
    Text(String),
    UnsignedList(Vec<u64>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelNormalizedConfiguration {
    kind: ModelConfigurationKind,
    unet_prefix: String,
    facts: BTreeMap<String, ModelConfigurationValue>,
}

impl ModelNormalizedConfiguration {
    pub fn kind(&self) -> ModelConfigurationKind {
        self.kind
    }

    pub fn unet_prefix(&self) -> &str {
        &self.unet_prefix
    }

    pub fn facts(&self) -> &BTreeMap<String, ModelConfigurationValue> {
        &self.facts
    }

    pub fn fact(&self, key: &str) -> Option<&ModelConfigurationValue> {
        self.facts.get(key)
    }

    fn has_architecture_facts(&self) -> bool {
        !self.facts.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelUnetPrefixSelection {
    prefix: String,
    candidate_counts: BTreeMap<String, usize>,
    sam3_top_level: bool,
}

impl ModelUnetPrefixSelection {
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn candidate_counts(&self) -> &BTreeMap<String, usize> {
        &self.candidate_counts
    }

    pub fn is_sam3_top_level(&self) -> bool {
        self.sam3_top_level
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelDetectionPolicy {
    RegisteredOnly,
    AllowBaseFallback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelBaseFallback {
    configuration: ModelNormalizedConfiguration,
}

impl ModelBaseFallback {
    pub fn configuration(&self) -> &ModelNormalizedConfiguration {
        &self.configuration
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelDetectionOutcome {
    Registered(ModelDetection),
    BaseFallback(ModelBaseFallback),
}

impl ModelProbe {
    pub fn from_parsed_facts(facts: ModelParsedFacts) -> Result<Self, ModelFamilyError> {
        Self::from_parsed_facts_cancellable(facts, &CancellationToken::default())
    }

    pub fn from_parsed_facts_cancellable(
        facts: ModelParsedFacts,
        cancellation: &CancellationToken,
    ) -> Result<Self, ModelFamilyError> {
        Self::from_parsed_facts_with_checkpoint(facts, cancellation, None)
    }

    fn from_parsed_facts_with_checkpoint(
        facts: ModelParsedFacts,
        cancellation: &CancellationToken,
        cancel_at_tensor: Option<usize>,
    ) -> Result<Self, ModelFamilyError> {
        cancellation.check()?;
        if facts.tensors.len() > MAX_MODEL_PROBE_TENSORS {
            return Err(ModelFamilyError::ProbeTensorLimit {
                actual: facts.tensors.len(),
                maximum: MAX_MODEL_PROBE_TENSORS,
            });
        }
        if facts.formats.len() > MAX_MODEL_PROBE_FORMATS {
            return Err(ModelFamilyError::ProbeFormatLimit {
                actual: facts.formats.len(),
                maximum: MAX_MODEL_PROBE_FORMATS,
            });
        }

        let mut tensor_shapes = BTreeMap::new();
        let mut metadata = BTreeMap::new();
        let mut metadata_bytes = 0_usize;
        for (index, (name, tensor)) in facts.tensors.into_iter().enumerate() {
            if cancel_at_tensor == Some(index) {
                cancellation.cancel();
            }
            if index % 1_024 == 0 {
                cancellation.check()?;
            }
            validate_probe_tensor_name(&name)?;
            validate_probe_shape(&name, &tensor.shape)?;
            let storage_dtype = normalize_storage_dtype(&tensor.storage_dtype)?;
            let dtype_key = format!("{MODEL_PROBE_DTYPE_PREFIX}{name}");
            let dtype_value = storage_dtype.normalized_name();
            metadata_bytes =
                checked_probe_metadata_bytes(metadata_bytes, dtype_key.len(), dtype_value.len())?;
            metadata.insert(dtype_key, dtype_value);
            tensor_shapes.insert(name, tensor.shape);
        }

        for (index, format) in facts.formats.into_iter().enumerate() {
            cancellation.check()?;
            validate_probe_format_identity(&format.identity)?;
            let format_key = format!("{MODEL_PROBE_FORMAT_PREFIX}{index:02}");
            metadata_bytes = checked_probe_metadata_bytes(
                metadata_bytes,
                format_key.len(),
                format.identity.len(),
            )?;
            metadata.insert(format_key, format.identity);
            for (key, value) in format.metadata {
                validate_probe_metadata(&key, &value)?;
                metadata_bytes =
                    checked_probe_metadata_bytes(metadata_bytes, key.len(), value.len())?;
                match metadata.entry(key) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(value);
                    }
                    std::collections::btree_map::Entry::Occupied(entry)
                        if entry.get() == &value => {}
                    std::collections::btree_map::Entry::Occupied(entry) => {
                        return Err(ModelFamilyError::ConflictingProbeMetadata(
                            entry.key().clone(),
                        ));
                    }
                }
            }
        }

        let mut probe = Self {
            tensor_shapes,
            metadata,
        };
        let prefix = select_unet_prefix_cancellable(&probe, Some(cancellation))?.prefix;
        checked_probe_metadata_bytes(metadata_bytes, MODEL_PROBE_UNET_PREFIX.len(), prefix.len())?;
        probe
            .metadata
            .insert(MODEL_PROBE_UNET_PREFIX.to_owned(), prefix);
        cancellation.check()?;
        Ok(probe)
    }

    pub fn tensor_shapes(&self) -> &BTreeMap<String, Vec<u64>> {
        &self.tensor_shapes
    }

    pub fn metadata(&self) -> &BTreeMap<String, String> {
        &self.metadata
    }

    pub fn storage_dtype(&self, tensor_name: &str) -> Option<ModelStorageDType> {
        self.metadata
            .get(&format!("{MODEL_PROBE_DTYPE_PREFIX}{tensor_name}"))
            .and_then(|value| normalized_storage_dtype(value))
    }

    pub fn format_identities(&self) -> Vec<&str> {
        self.metadata
            .range(MODEL_PROBE_FORMAT_PREFIX.to_owned()..)
            .take_while(|(key, _)| key.starts_with(MODEL_PROBE_FORMAT_PREFIX))
            .map(|(_, value)| value.as_str())
            .collect()
    }

    pub fn unet_prefix_selection(&self) -> Result<ModelUnetPrefixSelection, ModelFamilyError> {
        validate_model_probe(self)?;
        select_unet_prefix(self)
    }

    pub fn consecutive_block_count(&self, pattern: &str) -> Result<usize, ModelFamilyError> {
        validate_model_probe(self)?;
        count_consecutive_blocks(self, pattern)
    }

    pub fn normalized_configuration(
        &self,
    ) -> Result<ModelNormalizedConfiguration, ModelFamilyError> {
        validate_model_probe(self)?;
        normalize_model_configuration(self)
    }

    pub fn select_layout(
        &self,
        signatures: &[ModelLayoutSignature],
    ) -> Result<ModelStateLayout, ModelFamilyError> {
        validate_model_probe(self)?;
        select_state_layout(self, signatures)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelDetection {
    pub identity: ModelFamilyIdentity,
    pub score: u32,
    pub evidence: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
struct RegisteredModelFamily {
    definition: &'static ModelFamilyDefinition,
    source_ordinal: u16,
    source_architecture: &'static str,
    source_configuration: &'static [ModelSourceConfigurationRule],
    required_state_keys: &'static [&'static str],
    profile_selector: Option<ModelFamilyProfileSelector>,
    clip_target_selector: ModelClipTargetSelector,
    state_plan_selector: ModelFamilyStatePlanSelector,
    component_state_schemas: &'static [ModelFamilyComponentStateSchema],
    implicit_registration: bool,
}

impl RegisteredModelFamily {
    fn legacy(
        definition: &'static ModelFamilyDefinition,
        source_ordinal: u16,
    ) -> RegisteredModelFamily {
        Self {
            definition,
            source_ordinal,
            source_architecture: definition.identifier,
            source_configuration: &[],
            required_state_keys: &[],
            profile_selector: None,
            clip_target_selector: ModelClipTargetSelector::Profile,
            state_plan_selector: ModelFamilyStatePlanSelector::LegacyDefinitionRules,
            component_state_schemas: &[],
            implicit_registration: true,
        }
    }

    fn profile(self, probe: &ModelProbe) -> Result<ModelFamilyProfile, ModelFamilyError> {
        let profile = match self.profile_selector {
            Some(selector) => selector(probe)?,
            None => ModelFamilyProfile::from_definition(self.definition),
        };
        validate_profile(self.definition, &profile)?;
        Ok(profile)
    }

    fn state_plan(
        self,
        probe: &ModelProbe,
    ) -> Result<Option<ModelStateTransformPlan>, ModelFamilyError> {
        match self.state_plan_selector {
            ModelFamilyStatePlanSelector::LegacyDefinitionRules => Ok(None),
            ModelFamilyStatePlanSelector::Static(plan) => Ok(Some(plan.compile()?)),
            ModelFamilyStatePlanSelector::Layout { signatures, cases } => {
                let layout = select_state_layout(probe, signatures)?;
                cases
                    .iter()
                    .find(|case| case.layout == layout)
                    .map(|case| case.plan.compile().map(Some))
                    .transpose()?
                    .ok_or_else(|| {
                        ModelFamilyError::StatePlanSelection(format!(
                            "selected layout {layout:?} has no state plan"
                        ))
                    })
            }
            ModelFamilyStatePlanSelector::Probe(selector) => Ok(Some(selector(probe)?)),
        }
    }

    fn clip_target(
        self,
        profile: ModelFamilyProfile,
        probe: &ModelProbe,
    ) -> Result<ModelClipTargetDescriptor, ModelFamilyError> {
        match self.clip_target_selector {
            ModelClipTargetSelector::Profile => profile.clip_target.compile(),
            ModelClipTargetSelector::Static(target) => target.compile(),
            ModelClipTargetSelector::Metadata { key, cases } => {
                let value = probe.metadata.get(key).ok_or_else(|| {
                    ModelFamilyError::ClipTargetSelection(format!(
                        "probe is missing CLIP-target metadata {key}"
                    ))
                })?;
                cases
                    .iter()
                    .find(|case| case.metadata_value == value)
                    .map(|case| case.target.compile())
                    .transpose()?
                    .ok_or_else(|| {
                        ModelFamilyError::ClipTargetSelection(format!(
                            "CLIP-target metadata {key} has unsupported value {value}"
                        ))
                    })
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedModelFamily {
    detection: ModelDetection,
    registration: RegisteredModelFamily,
    profile: ModelFamilyProfile,
    clip_target: ModelClipTargetDescriptor,
    state_plan: Option<ModelStateTransformPlan>,
    probe_tensor_shapes: BTreeMap<String, Vec<u64>>,
    probe_identity: String,
}

impl ResolvedModelFamily {
    pub fn detection(&self) -> &ModelDetection {
        &self.detection
    }

    pub fn definition(&self) -> &'static ModelFamilyDefinition {
        self.registration.definition
    }

    pub fn source_ordinal(&self) -> u16 {
        self.registration.source_ordinal
    }

    pub fn source_architecture(&self) -> &'static str {
        self.registration.source_architecture
    }

    pub fn profile(&self) -> ModelFamilyProfile {
        self.profile
    }

    pub fn clip_target(&self) -> &ModelClipTargetDescriptor {
        &self.clip_target
    }

    pub fn state_plan(&self) -> Option<&ModelStateTransformPlan> {
        self.state_plan.as_ref()
    }

    pub fn map_state_dictionary(
        &self,
        transaction: &ModelStateTransaction<'_>,
        base_artifact_digest: impl Into<String>,
        source: &BTreeMap<String, Tensor>,
    ) -> Result<MappedModelComponents, ModelFamilyError> {
        validate_mapping_probe_snapshot(&self.probe_tensor_shapes, source)?;
        let base_artifact_digest = base_artifact_digest.into();
        let mut mapped = match self.state_plan.as_ref() {
            Some(plan) => transaction.execute(plan, base_artifact_digest, source)?,
            None => {
                transaction.context.cancellation.check()?;
                let legacy = map_model_weights(
                    self.registration.definition,
                    base_artifact_digest,
                    source.clone(),
                )?;
                let component =
                    self.registration
                        .definition
                        .components
                        .first()
                        .ok_or_else(|| {
                            ModelFamilyError::InvalidDefinition(
                                "model family has no component for legacy mapping".to_owned(),
                            )
                        })?;
                let mut components = BTreeMap::new();
                components.insert(
                    component.identifier.to_owned(),
                    Arc::unwrap_or_clone(legacy.tensors),
                );
                transaction.context.cancellation.check()?;
                finish_state_transaction(legacy.base_artifact_digest, components)
            }
        };
        validate_mapped_components(
            self.registration.definition,
            self.registration.component_state_schemas,
            &mapped,
        )?;
        transaction.context.cancellation.check()?;
        mapped.binding = Some(Arc::new(ModelFamilyWeightBinding {
            family: self.detection.identity.clone(),
            profile_identity: model_profile_identity(&self.profile),
            state_plan_identity: self
                .state_plan
                .as_ref()
                .map(|plan| plan.identity().to_owned())
                .unwrap_or_else(|| "legacy-definition-rules-v1".to_owned()),
            probe_identity: Some(self.probe_identity.clone()),
        }));
        Ok(mapped)
    }

    pub fn map_primary_weights(
        &self,
        transaction: &ModelStateTransaction<'_>,
        base_artifact_digest: impl Into<String>,
        source: &BTreeMap<String, Tensor>,
    ) -> Result<MappedModelWeights, ModelFamilyError> {
        let mapped = self.map_state_dictionary(transaction, base_artifact_digest, source)?;
        let primary = self
            .registration
            .definition
            .components
            .first()
            .ok_or_else(|| {
                ModelFamilyError::InvalidDefinition(
                    "model family has no primary component".to_owned(),
                )
            })?;
        let tensors = mapped
            .components
            .get(primary.identifier)
            .cloned()
            .ok_or_else(|| {
                ModelFamilyError::MissingRequiredComponent(primary.identifier.to_owned())
            })?;
        let binding = mapped.binding.as_deref().cloned().ok_or_else(|| {
            ModelFamilyError::WeightBindingMismatch(
                "resolved component mapping has no family binding".to_owned(),
            )
        })?;
        MappedModelWeights::from_parts(mapped.base_artifact_digest, tensors, Vec::new())
            .bind(binding)
    }
}

#[derive(Clone, Debug)]
pub struct ModelFamilyRegistry {
    definitions: BTreeMap<ModelFamilyIdentity, RegisteredModelFamily>,
    source_order: BTreeMap<u16, ModelFamilyIdentity>,
}

impl ModelFamilyRegistry {
    pub fn checked(
        definitions: &'static [ModelFamilyDefinition],
    ) -> Result<Self, ModelFamilyError> {
        let registrations = definitions
            .iter()
            .enumerate()
            .map(|(index, definition)| {
                let source_ordinal = u16::try_from(index)
                    .map_err(|_| ModelFamilyError::SourceOrdinalOverflow(index))?;
                Ok(RegisteredModelFamily::legacy(definition, source_ordinal))
            })
            .collect::<Result<Vec<_>, ModelFamilyError>>()?;
        Self::checked_entries(registrations)
    }

    pub fn checked_registrations(
        registrations: &'static [ModelFamilyRegistration],
    ) -> Result<Self, ModelFamilyError> {
        let registrations = registrations
            .iter()
            .map(|registration| RegisteredModelFamily {
                definition: registration.definition,
                source_ordinal: registration.source_ordinal,
                source_architecture: registration.source_architecture,
                source_configuration: registration.source_configuration,
                required_state_keys: registration.required_state_keys,
                profile_selector: registration.profile_selector,
                clip_target_selector: registration.clip_target_selector,
                state_plan_selector: registration.state_plan_selector,
                component_state_schemas: registration.component_state_schemas,
                implicit_registration: false,
            })
            .collect();
        Self::checked_entries(registrations)
    }

    fn checked_entries(
        registrations: Vec<RegisteredModelFamily>,
    ) -> Result<Self, ModelFamilyError> {
        let mut registered = BTreeMap::new();
        let mut source_order = BTreeMap::new();
        let mut feature_ids = BTreeSet::new();
        let mut identifiers = BTreeSet::new();
        for registration in registrations {
            validate_registration(registration)?;
            let definition = registration.definition;
            if !feature_ids.insert(definition.feature_id) {
                return Err(ModelFamilyError::DuplicateFeatureId(
                    definition.feature_id.to_owned(),
                ));
            }
            if !identifiers.insert(definition.identifier) {
                return Err(ModelFamilyError::DuplicateIdentifier(
                    definition.identifier.to_owned(),
                ));
            }
            let identity = identity_for(definition)?;
            if source_order
                .insert(registration.source_ordinal, identity.clone())
                .is_some()
            {
                return Err(ModelFamilyError::DuplicateSourceOrdinal(
                    registration.source_ordinal,
                ));
            }
            registered.insert(identity, registration);
        }
        Ok(Self {
            definitions: registered,
            source_order,
        })
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn definition(
        &self,
        identity: &ModelFamilyIdentity,
    ) -> Option<&'static ModelFamilyDefinition> {
        self.definitions
            .get(identity)
            .map(|registration| registration.definition)
    }

    pub fn definitions_in_source_order(&self) -> Vec<&'static ModelFamilyDefinition> {
        self.source_order
            .values()
            .filter_map(|identity| self.definition(identity))
            .collect()
    }

    pub fn detect(&self, probe: &ModelProbe) -> Result<ModelDetection, ModelFamilyError> {
        validate_model_probe(probe)?;
        let mut candidates = Vec::new();
        let mut rejected = Vec::new();
        for (identity, registration) in &self.definitions {
            let Some((score, mut evidence)) =
                detection_score(registration.definition.detection_rules, probe)?
            else {
                continue;
            };
            match validate_probe_registration(*registration, probe) {
                Ok(()) => {
                    evidence.extend(source_configuration_evidence(
                        registration.source_configuration,
                    ));
                    evidence.extend(
                        registration
                            .required_state_keys
                            .iter()
                            .map(|key| format!("required state key {key}")),
                    );
                }
                Err(error) => {
                    rejected.push(error);
                    continue;
                }
            }
            candidates.push((score, identity.clone(), evidence));
        }
        candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        let Some((score, identity, evidence)) = candidates.first().cloned() else {
            return match rejected.len() {
                1 => Err(rejected.remove(0)),
                _ => Err(ModelFamilyError::NoDetectionMatch),
            };
        };
        if candidates
            .get(1)
            .is_some_and(|candidate| candidate.0 == score)
        {
            return Err(ModelFamilyError::AmbiguousDetection {
                score,
                families: candidates
                    .into_iter()
                    .take_while(|candidate| candidate.0 == score)
                    .map(|candidate| candidate.1.identifier)
                    .collect(),
            });
        }
        Ok(ModelDetection {
            identity,
            score,
            evidence,
        })
    }

    pub fn detect_with_policy(
        &self,
        probe: &ModelProbe,
        policy: ModelDetectionPolicy,
    ) -> Result<ModelDetectionOutcome, ModelFamilyError> {
        apply_detection_policy(self.detect(probe), probe, policy)
    }

    pub fn resolve(&self, probe: &ModelProbe) -> Result<ResolvedModelFamily, ModelFamilyError> {
        let detection = self.detect(probe)?;
        let registration = self
            .definitions
            .get(&detection.identity)
            .copied()
            .ok_or(ModelFamilyError::NoDetectionMatch)?;
        validate_probe_registration(registration, probe)?;
        let profile = registration.profile(probe)?;
        let clip_target = registration.clip_target(profile, probe)?;
        let state_plan = registration.state_plan(probe)?;
        if let Some(state_plan) = state_plan.as_ref() {
            validate_state_plan_for_definition(registration.definition, state_plan)?;
        }
        Ok(ResolvedModelFamily {
            detection,
            registration,
            profile,
            clip_target,
            state_plan,
            probe_tensor_shapes: probe.tensor_shapes.clone(),
            probe_identity: model_probe_identity(probe),
        })
    }
}

fn apply_detection_policy(
    detection: Result<ModelDetection, ModelFamilyError>,
    probe: &ModelProbe,
    policy: ModelDetectionPolicy,
) -> Result<ModelDetectionOutcome, ModelFamilyError> {
    match detection {
        Ok(detection) => Ok(ModelDetectionOutcome::Registered(detection)),
        Err(ModelFamilyError::NoDetectionMatch)
            if policy == ModelDetectionPolicy::AllowBaseFallback =>
        {
            let configuration = probe.normalized_configuration()?;
            if !configuration.has_architecture_facts() {
                return Err(ModelFamilyError::NoDetectionMatch);
            }
            Ok(ModelDetectionOutcome::BaseFallback(ModelBaseFallback {
                configuration,
            }))
        }
        Err(error) => Err(error),
    }
}

pub fn describe_model_family(
    definition: &ModelFamilyDefinition,
) -> Result<ModelDescriptor, ModelFamilyError> {
    validate_definition(definition)?;
    LatentFormatIdentity::new(definition.latent_feature_id, definition.latent_identifier)
        .map_err(|error| ModelFamilyError::InvalidLatentIdentity(error.to_string()))?;
    let descriptor = ModelDescriptor {
        schema_version: MODEL_DESCRIPTOR_SCHEMA_VERSION,
        identifier: definition.identifier.to_owned(),
        family: definition.feature_id.to_owned(),
        architecture_version: definition.architecture_version.to_owned(),
        latent_format: definition.latent_identifier.to_owned(),
        component_graph: definition
            .components
            .iter()
            .map(|component| ModelComponentDescriptor {
                identifier: component.identifier.to_owned(),
                role: component.role.to_owned(),
                required: component.required,
            })
            .collect(),
        tensor_key_rules: definition
            .weight_rules
            .iter()
            .map(|rule| TensorKeyRule {
                source_prefix: rule.source_prefix.to_owned(),
                target_prefix: rule.target_prefix.to_owned(),
                required: rule.required,
            })
            .collect(),
        required_keys: definition
            .required_keys
            .iter()
            .map(|key| (*key).to_owned())
            .collect(),
        optional_keys: definition
            .optional_keys
            .iter()
            .map(|key| (*key).to_owned())
            .collect(),
        supported_dtypes: definition
            .supported_dtypes
            .iter()
            .map(|dtype| dtype.catalog_name().to_owned())
            .collect(),
        supported_devices: definition.supported_devices.to_vec(),
        memory_estimator: definition.memory_estimator,
    };
    descriptor.validate()?;
    Ok(descriptor)
}

pub fn detect_model_family_rules(
    identity: ModelFamilyIdentity,
    rules: &[ModelDetectionRule],
    probe: &ModelProbe,
) -> Result<ModelDetection, ModelFamilyError> {
    validate_model_probe(probe)?;
    validate_detection_rules(rules)?;
    let Some((score, evidence)) = detection_score(rules, probe)? else {
        return Err(ModelFamilyError::NoDetectionMatch);
    };
    Ok(ModelDetection {
        identity,
        score,
        evidence,
    })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ModelKeyPredicate {
    Exact(String),
    Prefix(String),
    Suffix(String),
    Contains(String),
    All(Vec<Self>),
    Any(Vec<Self>),
    Not(Box<Self>),
}

impl ModelKeyPredicate {
    pub fn exact(value: impl Into<String>) -> Result<Self, ModelFamilyError> {
        Self::checked(Self::Exact(value.into()))
    }

    pub fn prefix(value: impl Into<String>) -> Result<Self, ModelFamilyError> {
        Self::checked(Self::Prefix(value.into()))
    }

    pub fn suffix(value: impl Into<String>) -> Result<Self, ModelFamilyError> {
        Self::checked(Self::Suffix(value.into()))
    }

    pub fn contains(value: impl Into<String>) -> Result<Self, ModelFamilyError> {
        Self::checked(Self::Contains(value.into()))
    }

    pub fn all(predicates: Vec<Self>) -> Result<Self, ModelFamilyError> {
        Self::checked(Self::All(predicates))
    }

    pub fn any(predicates: Vec<Self>) -> Result<Self, ModelFamilyError> {
        Self::checked(Self::Any(predicates))
    }

    pub fn negate(predicate: Self) -> Result<Self, ModelFamilyError> {
        Self::checked(Self::Not(Box::new(predicate)))
    }

    fn checked(predicate: Self) -> Result<Self, ModelFamilyError> {
        predicate.validate(0)?;
        Ok(predicate)
    }

    fn validate(&self, depth: usize) -> Result<(), ModelFamilyError> {
        if depth >= MAX_DIMENSION_EXPRESSION_DEPTH {
            return Err(ModelFamilyError::InvalidStateTransform(
                "key predicate nesting exceeds its bound".to_owned(),
            ));
        }
        match self {
            Self::Exact(value)
            | Self::Prefix(value)
            | Self::Suffix(value)
            | Self::Contains(value) => validate_state_key_fragment(value),
            Self::All(predicates) | Self::Any(predicates) => {
                if predicates.is_empty() || predicates.len() > 64 {
                    return Err(ModelFamilyError::InvalidStateTransform(
                        "composite key predicate must contain 1..=64 children".to_owned(),
                    ));
                }
                for predicate in predicates {
                    predicate.validate(depth + 1)?;
                }
                Ok(())
            }
            Self::Not(predicate) => predicate.validate(depth + 1),
        }
    }

    fn matches(&self, key: &str) -> bool {
        match self {
            Self::Exact(value) => key == value,
            Self::Prefix(value) => key.starts_with(value),
            Self::Suffix(value) => key.ends_with(value),
            Self::Contains(value) => key.contains(value),
            Self::All(predicates) => predicates.iter().all(|predicate| predicate.matches(key)),
            Self::Any(predicates) => predicates.iter().any(|predicate| predicate.matches(key)),
            Self::Not(predicate) => !predicate.matches(key),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModelKeySelector {
    predicate: ModelKeyPredicate,
    minimum_matches: usize,
    maximum_matches: usize,
}

impl ModelKeySelector {
    pub fn exact(key: impl Into<String>) -> Result<Self, ModelFamilyError> {
        Ok(Self {
            predicate: ModelKeyPredicate::exact(key)?,
            minimum_matches: 1,
            maximum_matches: 1,
        })
    }

    pub fn bounded(
        predicate: ModelKeyPredicate,
        minimum_matches: usize,
        maximum_matches: usize,
    ) -> Result<Self, ModelFamilyError> {
        let selector = Self {
            predicate,
            minimum_matches,
            maximum_matches,
        };
        selector.validate()?;
        Ok(selector)
    }

    pub fn validate(&self) -> Result<(), ModelFamilyError> {
        self.predicate.validate(0)?;
        if self.maximum_matches == 0
            || self.maximum_matches > MAX_STATE_DICTIONARY_MATCHES
            || self.minimum_matches > self.maximum_matches
        {
            return Err(ModelFamilyError::InvalidKeySelectorBounds {
                minimum: self.minimum_matches,
                maximum: self.maximum_matches,
            });
        }
        Ok(())
    }

    fn select<'a>(
        &self,
        source: &'a BTreeMap<String, Tensor>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<&'a str>, ModelFamilyError> {
        self.validate()?;
        cancellation.check()?;
        let mut matches = Vec::new();
        for (index, key) in source.keys().enumerate() {
            if index % 256 == 0 {
                cancellation.check()?;
            }
            if self.predicate.matches(key) {
                matches.push(key.as_str());
            }
        }
        cancellation.check()?;
        if matches.len() < self.minimum_matches || matches.len() > self.maximum_matches {
            return Err(ModelFamilyError::KeySelectorCardinality {
                predicate: format!("{:?}", self.predicate),
                minimum: self.minimum_matches,
                maximum: self.maximum_matches,
                actual: matches.len(),
            });
        }
        Ok(matches)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ModelOptionalKeyReplacement {
    Prefix { from: String, to: String },
    Suffix { from: String, to: String },
    Contains { from: String, to: String },
}

impl ModelOptionalKeyReplacement {
    fn apply(&self, source: &str) -> String {
        match self {
            Self::Prefix { from, to } => source
                .strip_prefix(from)
                .map_or_else(|| source.to_owned(), |suffix| format!("{to}{suffix}")),
            Self::Suffix { from, to } => source
                .strip_suffix(from)
                .map_or_else(|| source.to_owned(), |prefix| format!("{prefix}{to}")),
            Self::Contains { from, to } => {
                if source.contains(from) {
                    source.replacen(from, to, 1)
                } else {
                    source.to_owned()
                }
            }
        }
    }

    fn validate(&self) -> Result<(), ModelFamilyError> {
        let (from, to) = match self {
            Self::Prefix { from, to } | Self::Suffix { from, to } | Self::Contains { from, to } => {
                (from, to)
            }
        };
        validate_state_key_fragment(from)?;
        if to.len() > MAX_IDENTITY_BYTES || to.contains('\0') {
            return Err(ModelFamilyError::InvalidStateKey(to.clone()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ModelKeyRewrite {
    Identity,
    Exact(String),
    Prefix { from: String, to: String },
    Suffix { from: String, to: String },
    Contains { from: String, to: String },
    Pipeline(Vec<Self>),
    OrderedOptional(Vec<ModelOptionalKeyReplacement>),
}

impl ModelKeyRewrite {
    pub fn exact(value: impl Into<String>) -> Result<Self, ModelFamilyError> {
        let value = value.into();
        validate_state_key(&value)?;
        Ok(Self::Exact(value))
    }

    pub fn prefix(
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Result<Self, ModelFamilyError> {
        Self::checked_pair(from.into(), to.into(), |from, to| Self::Prefix { from, to })
    }

    pub fn suffix(
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Result<Self, ModelFamilyError> {
        Self::checked_pair(from.into(), to.into(), |from, to| Self::Suffix { from, to })
    }

    pub fn contains(
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Result<Self, ModelFamilyError> {
        Self::checked_pair(from.into(), to.into(), |from, to| Self::Contains {
            from,
            to,
        })
    }

    pub fn pipeline(rewrites: Vec<Self>) -> Result<Self, ModelFamilyError> {
        let rewrite = Self::Pipeline(rewrites);
        rewrite.validate(0)?;
        Ok(rewrite)
    }

    pub fn ordered_optional(
        replacements: Vec<ModelOptionalKeyReplacement>,
    ) -> Result<Self, ModelFamilyError> {
        let rewrite = Self::OrderedOptional(replacements);
        rewrite.validate(0)?;
        Ok(rewrite)
    }

    fn checked_pair(
        from: String,
        to: String,
        make: impl FnOnce(String, String) -> Self,
    ) -> Result<Self, ModelFamilyError> {
        validate_state_key_fragment(&from)?;
        if to.len() > MAX_IDENTITY_BYTES || to.contains('\0') {
            return Err(ModelFamilyError::InvalidStateKey(to));
        }
        Ok(make(from, to))
    }

    fn apply(&self, source: &str) -> Result<String, ModelFamilyError> {
        let target = match self {
            Self::Identity => source.to_owned(),
            Self::Exact(target) => target.clone(),
            Self::Prefix { from, to } => format!(
                "{to}{}",
                source.strip_prefix(from).ok_or_else(|| {
                    ModelFamilyError::KeyRewriteMismatch {
                        key: source.to_owned(),
                        rewrite: format!("{self:?}"),
                    }
                })?
            ),
            Self::Suffix { from, to } => format!(
                "{}{to}",
                source.strip_suffix(from).ok_or_else(|| {
                    ModelFamilyError::KeyRewriteMismatch {
                        key: source.to_owned(),
                        rewrite: format!("{self:?}"),
                    }
                })?
            ),
            Self::Contains { from, to } => {
                if !source.contains(from) {
                    return Err(ModelFamilyError::KeyRewriteMismatch {
                        key: source.to_owned(),
                        rewrite: format!("{self:?}"),
                    });
                }
                source.replacen(from, to, 1)
            }
            Self::Pipeline(rewrites) => {
                let mut value = source.to_owned();
                for rewrite in rewrites {
                    value = rewrite.apply(&value)?;
                }
                value
            }
            Self::OrderedOptional(replacements) => {
                let mut value = source.to_owned();
                for replacement in replacements {
                    value = replacement.apply(&value);
                }
                value
            }
        };
        validate_state_key(&target)?;
        Ok(target)
    }

    fn validate(&self, depth: usize) -> Result<(), ModelFamilyError> {
        if depth >= MAX_DIMENSION_EXPRESSION_DEPTH {
            return Err(ModelFamilyError::InvalidStateTransform(
                "key rewrite nesting exceeds its bound".to_owned(),
            ));
        }
        match self {
            Self::Identity => Ok(()),
            Self::Exact(target) => validate_state_key(target),
            Self::Prefix { from, to } | Self::Suffix { from, to } | Self::Contains { from, to } => {
                validate_state_key_fragment(from)?;
                if to.len() > MAX_IDENTITY_BYTES || to.contains('\0') {
                    return Err(ModelFamilyError::InvalidStateKey(to.clone()));
                }
                Ok(())
            }
            Self::Pipeline(rewrites) => {
                if rewrites.is_empty() || rewrites.len() > 64 {
                    return Err(ModelFamilyError::InvalidStateTransform(
                        "rewrite pipeline must contain 1..=64 steps".to_owned(),
                    ));
                }
                for rewrite in rewrites {
                    rewrite.validate(depth + 1)?;
                }
                Ok(())
            }
            Self::OrderedOptional(replacements) => {
                if replacements.is_empty() || replacements.len() > 64 {
                    return Err(ModelFamilyError::InvalidStateTransform(
                        "ordered optional replacement must contain 1..=64 steps".to_owned(),
                    ));
                }
                for replacement in replacements {
                    replacement.validate()?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ModelDimensionExpression {
    Literal(u64),
    SourceDimension { key: String, dimension: usize },
    CurrentTensorDimension { dimension: usize },
    Add(Box<Self>, Box<Self>),
    Multiply(Box<Self>, Box<Self>),
    DivideExact(Box<Self>, Box<Self>),
}

#[derive(Clone, Copy, Debug)]
pub struct ModelDimensionEvaluationContext<'a> {
    source: &'a BTreeMap<String, Tensor>,
    current_tensor_shape: Option<&'a [u64]>,
}

impl<'a> ModelDimensionEvaluationContext<'a> {
    pub fn source(source: &'a BTreeMap<String, Tensor>) -> Self {
        Self {
            source,
            current_tensor_shape: None,
        }
    }

    pub fn selected(source: &'a BTreeMap<String, Tensor>, tensor: &'a Tensor) -> Self {
        Self {
            source,
            current_tensor_shape: Some(tensor.descriptor().shape()),
        }
    }

    fn selected_shape(source: &'a BTreeMap<String, Tensor>, shape: &'a [u64]) -> Self {
        Self {
            source,
            current_tensor_shape: Some(shape),
        }
    }
}

impl ModelDimensionExpression {
    pub fn source_dimension(
        key: impl Into<String>,
        dimension: usize,
    ) -> Result<Self, ModelFamilyError> {
        let key = key.into();
        validate_state_key(&key)?;
        validate_dimension_axis(dimension)?;
        Ok(Self::SourceDimension { key, dimension })
    }

    pub fn current_tensor_dimension(dimension: usize) -> Self {
        Self::CurrentTensorDimension { dimension }
    }

    pub fn evaluate(
        &self,
        context: ModelDimensionEvaluationContext<'_>,
    ) -> Result<u64, ModelFamilyError> {
        self.evaluate_at_depth(context, 0)
    }

    pub fn validate(&self) -> Result<(), ModelFamilyError> {
        self.validate_at_depth(0, false)
    }

    fn validate_for_selected_tensor(&self) -> Result<(), ModelFamilyError> {
        self.validate_at_depth(0, true)
    }

    fn validate_at_depth(
        &self,
        depth: usize,
        selected_tensor_available: bool,
    ) -> Result<(), ModelFamilyError> {
        if depth >= MAX_DIMENSION_EXPRESSION_DEPTH {
            return Err(ModelFamilyError::DimensionExpressionTooDeep);
        }
        match self {
            Self::Literal(_) => Ok(()),
            Self::SourceDimension { key, dimension } => {
                validate_state_key(key)?;
                validate_dimension_axis(*dimension)
            }
            Self::CurrentTensorDimension { dimension } => {
                if !selected_tensor_available {
                    return Err(ModelFamilyError::CurrentTensorDimensionUnavailable);
                }
                validate_dimension_axis(*dimension)
            }
            Self::Add(left, right)
            | Self::Multiply(left, right)
            | Self::DivideExact(left, right) => {
                left.validate_at_depth(depth + 1, selected_tensor_available)?;
                right.validate_at_depth(depth + 1, selected_tensor_available)
            }
        }
    }

    fn evaluate_at_depth(
        &self,
        context: ModelDimensionEvaluationContext<'_>,
        depth: usize,
    ) -> Result<u64, ModelFamilyError> {
        if depth >= MAX_DIMENSION_EXPRESSION_DEPTH {
            return Err(ModelFamilyError::DimensionExpressionTooDeep);
        }
        match self {
            Self::Literal(value) => Ok(*value),
            Self::SourceDimension { key, dimension } => context
                .source
                .get(key)
                .ok_or_else(|| ModelFamilyError::MissingTransformSource(key.clone()))?
                .descriptor()
                .shape()
                .get(*dimension)
                .copied()
                .ok_or_else(|| ModelFamilyError::DimensionOutOfBounds {
                    key: key.clone(),
                    dimension: *dimension,
                }),
            Self::CurrentTensorDimension { dimension } => context
                .current_tensor_shape
                .ok_or(ModelFamilyError::CurrentTensorDimensionUnavailable)?
                .get(*dimension)
                .copied()
                .ok_or(ModelFamilyError::CurrentTensorDimensionOutOfBounds(
                    *dimension,
                )),
            Self::Add(left, right) => left
                .evaluate_at_depth(context, depth + 1)?
                .checked_add(right.evaluate_at_depth(context, depth + 1)?)
                .ok_or(ModelFamilyError::DimensionExpressionOverflow),
            Self::Multiply(left, right) => left
                .evaluate_at_depth(context, depth + 1)?
                .checked_mul(right.evaluate_at_depth(context, depth + 1)?)
                .ok_or(ModelFamilyError::DimensionExpressionOverflow),
            Self::DivideExact(left, right) => {
                let numerator = left.evaluate_at_depth(context, depth + 1)?;
                let denominator = right.evaluate_at_depth(context, depth + 1)?;
                if denominator == 0 {
                    return Err(ModelFamilyError::DimensionDivisionByZero);
                }
                if numerator % denominator != 0 {
                    return Err(ModelFamilyError::DimensionDivisionRemainder {
                        numerator,
                        denominator,
                    });
                }
                numerator
                    .checked_div(denominator)
                    .ok_or(ModelFamilyError::DimensionExpressionOverflow)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ModelStateTarget {
    pub component: String,
    pub key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ModelStateTensorReference {
    Source(String),
    Staged(ModelStateTarget),
}

impl ModelStateTensorReference {
    pub fn source(key: impl Into<String>) -> Result<Self, ModelFamilyError> {
        let key = key.into();
        validate_state_key(&key)?;
        Ok(Self::Source(key))
    }

    pub fn staged(target: ModelStateTarget) -> Self {
        Self::Staged(target)
    }
}

impl ModelStateTarget {
    pub fn checked(
        component: impl Into<String>,
        key: impl Into<String>,
    ) -> Result<Self, ModelFamilyError> {
        let component = component.into();
        let key = key.into();
        validate_identifier("model component", &component)?;
        validate_state_key(&key)?;
        Ok(Self { component, key })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ModelRoundCondition {
    Always,
    DType(DType),
    Rank(usize),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ModelPerTensorTransform {
    Sequence(Vec<Self>),
    Contiguous,
    Narrow {
        dimension: usize,
        start: i64,
        length: u64,
    },
    Transpose {
        first_dimension: usize,
        second_dimension: usize,
    },
    Permute {
        dimensions: Vec<usize>,
    },
    Reshape {
        shape: Vec<ModelDimensionExpression>,
    },
    Expand {
        shape: Vec<ModelDimensionExpression>,
    },
    ConditionalRound {
        decimals: i32,
        condition: ModelRoundCondition,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModelSplitOutputRule {
    pub component: String,
    pub rewrite: ModelKeyRewrite,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ModelTransformBranchOutputRule {
    pub component: String,
    pub rewrite: ModelKeyRewrite,
    pub transform: ModelPerTensorTransform,
}

impl ModelRoundCondition {
    fn matches(self, tensor: &Tensor) -> bool {
        match self {
            Self::Always => true,
            Self::DType(dtype) => tensor.descriptor().dtype() == dtype,
            Self::Rank(rank) => tensor.descriptor().rank() == rank,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ModelStateTransformOperation {
    Move {
        selector: ModelKeySelector,
        rewrite: ModelKeyRewrite,
        component: String,
    },
    Copy {
        selector: ModelKeySelector,
        rewrite: ModelKeyRewrite,
        component: String,
    },
    Drop {
        selector: ModelKeySelector,
    },
    Route {
        selector: ModelKeySelector,
        rewrite: ModelKeyRewrite,
        component: String,
    },
    Split {
        source: ModelStateTensorReference,
        dimension: usize,
        sizes: Vec<ModelDimensionExpression>,
        outputs: Vec<ModelStateTarget>,
    },
    Assemble {
        sources: Vec<ModelStateTensorReference>,
        dimension: usize,
        output: ModelStateTarget,
    },
    Transpose {
        source: ModelStateTensorReference,
        first_dimension: usize,
        second_dimension: usize,
        output: ModelStateTarget,
    },
    Reshape {
        source: ModelStateTensorReference,
        shape: Vec<ModelDimensionExpression>,
        output: ModelStateTarget,
    },
    ConditionalRound {
        source: ModelStateTensorReference,
        decimals: i32,
        condition: ModelRoundCondition,
        output: ModelStateTarget,
    },
    Generate {
        shape: Vec<ModelDimensionExpression>,
        fill: Scalar,
        dtype: DType,
        output: ModelStateTarget,
    },
    GenerateArange {
        start: Scalar,
        end: Scalar,
        step: Scalar,
        dtype: DType,
        shape: Vec<ModelDimensionExpression>,
        output: ModelStateTarget,
    },
    Narrow {
        source: ModelStateTensorReference,
        dimension: usize,
        start: i64,
        length: u64,
        output: ModelStateTarget,
    },
    Permute {
        source: ModelStateTensorReference,
        dimensions: Vec<usize>,
        output: ModelStateTarget,
    },
    Expand {
        source: ModelStateTensorReference,
        shape: Vec<ModelDimensionExpression>,
        output: ModelStateTarget,
    },
    TransformEach {
        selector: ModelKeySelector,
        rewrite: ModelKeyRewrite,
        component: String,
        transform: ModelPerTensorTransform,
    },
    TransformBranchesEach {
        selector: ModelKeySelector,
        pre_transform: ModelPerTensorTransform,
        outputs: Vec<ModelTransformBranchOutputRule>,
    },
    SplitEach {
        selector: ModelKeySelector,
        dimension: usize,
        sizes: Vec<ModelDimensionExpression>,
        outputs: Vec<ModelSplitOutputRule>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ModelUnmatchedKeyDisposition {
    Reject,
    Drop,
    Route {
        component: String,
        rewrite: ModelKeyRewrite,
    },
}

pub const MODEL_STATE_TRANSFORM_PLAN_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug)]
pub struct ModelStateTransformPlanDefinition {
    pub schema_version: u16,
    pub encoded_plan: &'static str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelStateTransformPlanWire {
    operations: Vec<ModelStateTransformOperation>,
    unmatched: ModelUnmatchedKeyDisposition,
}

impl ModelStateTransformPlanDefinition {
    pub fn compile(&self) -> Result<ModelStateTransformPlan, ModelFamilyError> {
        if self.schema_version != MODEL_STATE_TRANSFORM_PLAN_SCHEMA_VERSION {
            return Err(ModelFamilyError::StatePlanSchemaVersion(
                self.schema_version,
            ));
        }
        if self.encoded_plan.len() > 1024 * 1024 {
            return Err(ModelFamilyError::StatePlanDefinitionTooLarge(
                self.encoded_plan.len(),
            ));
        }
        let wire: ModelStateTransformPlanWire = serde_json::from_str(self.encoded_plan)
            .map_err(|error| ModelFamilyError::StatePlanDefinition(error.to_string()))?;
        ModelStateTransformPlan::checked(wire.operations, wire.unmatched)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelStateTransformPlan {
    operations: Vec<ModelStateTransformOperation>,
    unmatched: ModelUnmatchedKeyDisposition,
    identity: String,
}

impl ModelStateTransformPlan {
    pub fn checked(
        operations: Vec<ModelStateTransformOperation>,
        unmatched: ModelUnmatchedKeyDisposition,
    ) -> Result<Self, ModelFamilyError> {
        if operations.len() > MAX_STATE_DICTIONARY_OPERATIONS {
            return Err(ModelFamilyError::StatePlanTooLarge(operations.len()));
        }
        for operation in &operations {
            validate_state_operation(operation)?;
        }
        if let ModelUnmatchedKeyDisposition::Route { component, rewrite } = &unmatched {
            validate_identifier("model component", component)?;
            rewrite.validate(0)?;
        }
        let mut digest = Sha256::new();
        digest.update(b"sim.comfy.model-state-transform-plan.v1\0");
        digest.update(format!("{operations:?}\0{unmatched:?}").as_bytes());
        Ok(Self {
            operations,
            unmatched,
            identity: format!("{:x}", digest.finalize()),
        })
    }

    pub fn operations(&self) -> &[ModelStateTransformOperation] {
        &self.operations
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }
}

#[derive(Clone, Debug)]
pub struct MappedModelComponents {
    base_artifact_digest: String,
    binding: Option<Arc<ModelFamilyWeightBinding>>,
    components: Arc<BTreeMap<String, BTreeMap<String, Tensor>>>,
}

impl MappedModelComponents {
    pub fn base_artifact_digest(&self) -> &str {
        &self.base_artifact_digest
    }

    pub fn components(&self) -> &BTreeMap<String, BTreeMap<String, Tensor>> {
        &self.components
    }

    pub fn component(&self, identifier: &str) -> Option<&BTreeMap<String, Tensor>> {
        self.components.get(identifier)
    }

    pub fn binding(&self) -> Option<&ModelFamilyWeightBinding> {
        self.binding.as_deref()
    }
}

pub struct ModelStateTransaction<'a> {
    backend: &'a CpuBackend,
    context: &'a ExecutionContext<'a>,
}

impl<'a> ModelStateTransaction<'a> {
    pub fn new(backend: &'a CpuBackend, context: &'a ExecutionContext<'a>) -> Self {
        Self { backend, context }
    }

    pub fn execute(
        &self,
        plan: &ModelStateTransformPlan,
        base_artifact_digest: impl Into<String>,
        source: &BTreeMap<String, Tensor>,
    ) -> Result<MappedModelComponents, ModelFamilyError> {
        self.context.cancellation.check()?;
        let base_artifact_digest = base_artifact_digest.into();
        validate_digest(&base_artifact_digest)?;
        validate_state_source_bound(source.len())?;
        let preflight = preflight_state_transaction(plan, source, self.context.cancellation)?;
        let mut components = BTreeMap::<String, BTreeMap<String, Tensor>>::new();
        for (operation_index, operation) in plan.operations.iter().enumerate() {
            if operation_index % 64 == 0 {
                self.context.cancellation.check()?;
            }
            if let Err(error) = execute_state_operation(
                self.backend,
                self.context,
                operation,
                source,
                &mut components,
            ) {
                self.context.cancellation.check()?;
                return Err(error);
            }
        }
        apply_unmatched(
            &plan.unmatched,
            source,
            &preflight.handled,
            &mut components,
            self.context.cancellation,
        )?;
        self.context.cancellation.check()?;
        Ok(finish_state_transaction(base_artifact_digest, components))
    }
}

struct ModelStatePreflight {
    handled: BTreeSet<String>,
}

fn finish_state_transaction(
    base_artifact_digest: String,
    staged_components: BTreeMap<String, BTreeMap<String, Tensor>>,
) -> MappedModelComponents {
    MappedModelComponents {
        base_artifact_digest,
        binding: None,
        components: Arc::new(staged_components),
    }
}

fn validate_state_operation(
    operation: &ModelStateTransformOperation,
) -> Result<(), ModelFamilyError> {
    let validate_component = |component: &str| validate_identifier("model component", component);
    let validate_source = |source: &ModelStateTensorReference| match source {
        ModelStateTensorReference::Source(key) => validate_state_key(key),
        ModelStateTensorReference::Staged(target) => {
            validate_component(&target.component)?;
            validate_state_key(&target.key)
        }
    };
    let validate_shape = |shape: &[ModelDimensionExpression], selected_tensor_available: bool| {
        if shape.len() > MAX_TENSOR_RANK {
            return Err(ModelFamilyError::InvalidStateTransform(
                "tensor rank exceeds 32".to_owned(),
            ));
        }
        for expression in shape {
            if selected_tensor_available {
                expression.validate_for_selected_tensor()?;
            } else {
                expression.validate()?;
            }
        }
        Ok(())
    };
    match operation {
        ModelStateTransformOperation::Move {
            selector,
            component,
            rewrite,
        }
        | ModelStateTransformOperation::Copy {
            selector,
            component,
            rewrite,
        }
        | ModelStateTransformOperation::Route {
            selector,
            component,
            rewrite,
        } => {
            selector.validate()?;
            rewrite.validate(0)?;
            validate_component(component)
        }
        ModelStateTransformOperation::Drop { selector } => selector.validate(),
        ModelStateTransformOperation::Split {
            source,
            dimension,
            sizes,
            outputs,
        } => {
            validate_source(source)?;
            validate_dimension_axis(*dimension)?;
            if sizes.is_empty() || sizes.len() != outputs.len() {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "split sizes and outputs must be nonempty and have equal length".to_owned(),
                ));
            }
            validate_shape(sizes, false)?;
            for output in outputs {
                validate_component(&output.component)?;
                validate_state_key(&output.key)?;
            }
            Ok(())
        }
        ModelStateTransformOperation::Assemble {
            sources,
            dimension,
            output,
        } => {
            if sources.is_empty() {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "assembly sources cannot be empty".to_owned(),
                ));
            }
            for source in sources {
                validate_source(source)?;
            }
            validate_dimension_axis(*dimension)?;
            validate_component(&output.component)?;
            validate_state_key(&output.key)
        }
        ModelStateTransformOperation::Transpose {
            source,
            first_dimension,
            second_dimension,
            output,
        } => {
            validate_source(source)?;
            validate_dimension_axis(*first_dimension)?;
            validate_dimension_axis(*second_dimension)?;
            validate_component(&output.component)?;
            validate_state_key(&output.key)
        }
        ModelStateTransformOperation::ConditionalRound {
            source,
            condition,
            output,
            ..
        } => {
            validate_source(source)?;
            validate_round_condition(*condition)?;
            validate_component(&output.component)?;
            validate_state_key(&output.key)
        }
        ModelStateTransformOperation::Reshape {
            source,
            shape,
            output,
        } => {
            validate_source(source)?;
            validate_shape(shape, false)?;
            validate_component(&output.component)?;
            validate_state_key(&output.key)
        }
        ModelStateTransformOperation::Generate { shape, output, .. } => {
            validate_shape(shape, false)?;
            validate_component(&output.component)?;
            validate_state_key(&output.key)
        }
        ModelStateTransformOperation::GenerateArange { shape, output, .. } => {
            validate_shape(shape, false)?;
            if shape.is_empty() {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "arange output shape cannot be scalar".to_owned(),
                ));
            }
            validate_component(&output.component)?;
            validate_state_key(&output.key)
        }
        ModelStateTransformOperation::Narrow {
            source,
            dimension,
            length,
            output,
            ..
        } => {
            validate_source(source)?;
            validate_dimension_axis(*dimension)?;
            if *length == 0 {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "narrow length must be nonzero".to_owned(),
                ));
            }
            validate_component(&output.component)?;
            validate_state_key(&output.key)
        }
        ModelStateTransformOperation::Permute {
            source,
            dimensions,
            output,
        } => {
            validate_source(source)?;
            if dimensions.is_empty() || dimensions.len() > 32 {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "permutation rank must be between one and 32".to_owned(),
                ));
            }
            validate_permutation_definition(dimensions)?;
            validate_component(&output.component)?;
            validate_state_key(&output.key)
        }
        ModelStateTransformOperation::Expand {
            source,
            shape,
            output,
        } => {
            validate_source(source)?;
            validate_shape(shape, false)?;
            validate_component(&output.component)?;
            validate_state_key(&output.key)
        }
        ModelStateTransformOperation::TransformEach {
            selector,
            rewrite,
            component,
            transform,
        } => {
            selector.validate()?;
            rewrite.validate(0)?;
            validate_component(component)?;
            validate_per_tensor_transform_definition(transform)
        }
        ModelStateTransformOperation::TransformBranchesEach {
            selector,
            pre_transform,
            outputs,
        } => {
            selector.validate()?;
            validate_per_tensor_transform_definition(pre_transform)?;
            if outputs.is_empty() || outputs.len() > 64 {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "selector transform branches must contain 1..=64 outputs".to_owned(),
                ));
            }
            for output in outputs {
                validate_component(&output.component)?;
                output.rewrite.validate(0)?;
                validate_per_tensor_transform_definition(&output.transform)?;
            }
            Ok(())
        }
        ModelStateTransformOperation::SplitEach {
            selector,
            dimension,
            sizes,
            outputs,
        } => {
            selector.validate()?;
            validate_dimension_axis(*dimension)?;
            if sizes.is_empty() || sizes.len() != outputs.len() {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "selector split sizes and outputs must be nonempty and equal".to_owned(),
                ));
            }
            validate_shape(sizes, true)?;
            for output in outputs {
                validate_component(&output.component)?;
                output.rewrite.validate(0)?;
            }
            Ok(())
        }
    }
}

fn validate_state_plan_for_definition(
    definition: &ModelFamilyDefinition,
    plan: &ModelStateTransformPlan,
) -> Result<(), ModelFamilyError> {
    let declared = definition
        .components
        .iter()
        .map(|component| component.identifier)
        .collect::<BTreeSet<_>>();
    let mut targeted = BTreeSet::new();
    for operation in &plan.operations {
        let component = match operation {
            ModelStateTransformOperation::Move { component, .. }
            | ModelStateTransformOperation::Copy { component, .. }
            | ModelStateTransformOperation::Route { component, .. } => Some(component.as_str()),
            ModelStateTransformOperation::Split { outputs, .. } => {
                for output in outputs {
                    validate_declared_component(&declared, &output.component)?;
                    targeted.insert(output.component.as_str());
                }
                None
            }
            ModelStateTransformOperation::Assemble { output, .. }
            | ModelStateTransformOperation::Transpose { output, .. }
            | ModelStateTransformOperation::Reshape { output, .. }
            | ModelStateTransformOperation::ConditionalRound { output, .. }
            | ModelStateTransformOperation::Generate { output, .. }
            | ModelStateTransformOperation::GenerateArange { output, .. }
            | ModelStateTransformOperation::Narrow { output, .. }
            | ModelStateTransformOperation::Permute { output, .. }
            | ModelStateTransformOperation::Expand { output, .. } => {
                Some(output.component.as_str())
            }
            ModelStateTransformOperation::TransformEach { component, .. } => {
                Some(component.as_str())
            }
            ModelStateTransformOperation::TransformBranchesEach { outputs, .. } => {
                for output in outputs {
                    validate_declared_component(&declared, &output.component)?;
                    targeted.insert(output.component.as_str());
                }
                None
            }
            ModelStateTransformOperation::SplitEach { outputs, .. } => {
                for output in outputs {
                    validate_declared_component(&declared, &output.component)?;
                    targeted.insert(output.component.as_str());
                }
                None
            }
            ModelStateTransformOperation::Drop { .. } => None,
        };
        if let Some(component) = component {
            validate_declared_component(&declared, component)?;
            targeted.insert(component);
        }
    }
    if let ModelUnmatchedKeyDisposition::Route { component, .. } = &plan.unmatched {
        validate_declared_component(&declared, component)?;
        targeted.insert(component);
    }
    for component in definition
        .components
        .iter()
        .filter(|component| component.required)
    {
        if !targeted.contains(component.identifier) {
            return Err(ModelFamilyError::MissingRequiredComponent(
                component.identifier.to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_per_tensor_transform_definition(
    transform: &ModelPerTensorTransform,
) -> Result<(), ModelFamilyError> {
    validate_per_tensor_transform_at_depth(transform, 0)
}

fn validate_per_tensor_transform_at_depth(
    transform: &ModelPerTensorTransform,
    depth: usize,
) -> Result<(), ModelFamilyError> {
    if depth >= MAX_DIMENSION_EXPRESSION_DEPTH {
        return Err(ModelFamilyError::InvalidStateTransform(
            "per-tensor transform sequence nesting exceeds its bound".to_owned(),
        ));
    }
    match transform {
        ModelPerTensorTransform::Sequence(transforms) => {
            if transforms.is_empty() || transforms.len() > 64 {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "per-tensor transform sequence must contain 1..=64 steps".to_owned(),
                ));
            }
            for transform in transforms {
                validate_per_tensor_transform_at_depth(transform, depth + 1)?;
            }
            Ok(())
        }
        ModelPerTensorTransform::Contiguous => Ok(()),
        ModelPerTensorTransform::Narrow {
            dimension, length, ..
        } => {
            validate_dimension_axis(*dimension)?;
            if *length == 0 {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "selector narrow length must be nonzero".to_owned(),
                ));
            }
            Ok(())
        }
        ModelPerTensorTransform::Transpose {
            first_dimension,
            second_dimension,
        } => {
            validate_dimension_axis(*first_dimension)?;
            validate_dimension_axis(*second_dimension)
        }
        ModelPerTensorTransform::Permute { dimensions } => {
            if dimensions.is_empty() || dimensions.len() > MAX_TENSOR_RANK {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "selector permutation rank must be between one and 32".to_owned(),
                ));
            }
            validate_permutation_definition(dimensions)
        }
        ModelPerTensorTransform::Reshape { shape } | ModelPerTensorTransform::Expand { shape } => {
            if shape.len() > MAX_TENSOR_RANK {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "selector transform rank exceeds 32".to_owned(),
                ));
            }
            for expression in shape {
                expression.validate_for_selected_tensor()?;
            }
            Ok(())
        }
        ModelPerTensorTransform::ConditionalRound { condition, .. } => {
            validate_round_condition(*condition)
        }
    }
}

fn validate_dimension_axis(dimension: usize) -> Result<(), ModelFamilyError> {
    if dimension >= MAX_TENSOR_RANK {
        return Err(ModelFamilyError::InvalidStateTransform(format!(
            "tensor dimension {dimension} exceeds the maximum supported rank {MAX_TENSOR_RANK}"
        )));
    }
    Ok(())
}

fn validate_permutation_definition(dimensions: &[usize]) -> Result<(), ModelFamilyError> {
    let mut seen = BTreeSet::new();
    for dimension in dimensions {
        validate_dimension_axis(*dimension)?;
        if !seen.insert(*dimension) {
            return Err(ModelFamilyError::InvalidStateTransform(format!(
                "permutation repeats dimension {dimension}"
            )));
        }
    }
    Ok(())
}

fn validate_round_condition(condition: ModelRoundCondition) -> Result<(), ModelFamilyError> {
    if let ModelRoundCondition::Rank(rank) = condition
        && rank > MAX_TENSOR_RANK
    {
        return Err(ModelFamilyError::InvalidStateTransform(format!(
            "round rank {rank} exceeds the maximum supported rank {MAX_TENSOR_RANK}"
        )));
    }
    Ok(())
}

fn validate_declared_component(
    declared: &BTreeSet<&str>,
    component: &str,
) -> Result<(), ModelFamilyError> {
    if !declared.contains(component) {
        return Err(ModelFamilyError::UndeclaredComponent(component.to_owned()));
    }
    Ok(())
}

fn validate_mapped_components(
    definition: &ModelFamilyDefinition,
    schemas: &[ModelFamilyComponentStateSchema],
    mapped: &MappedModelComponents,
) -> Result<(), ModelFamilyError> {
    let declared = definition
        .components
        .iter()
        .map(|component| component.identifier)
        .collect::<BTreeSet<_>>();
    for component in mapped.components.keys() {
        validate_declared_component(&declared, component)?;
        if !schemas.is_empty()
            && !schemas
                .iter()
                .any(|schema| schema.component == component.as_str())
        {
            return Err(ModelFamilyError::MissingComponentSchema(
                component.to_owned(),
            ));
        }
    }
    for component in definition
        .components
        .iter()
        .filter(|component| component.required)
    {
        if mapped
            .components
            .get(component.identifier)
            .is_none_or(BTreeMap::is_empty)
        {
            return Err(ModelFamilyError::MissingRequiredComponent(
                component.identifier.to_owned(),
            ));
        }
    }
    if schemas.is_empty() {
        let primary = definition.components.first().ok_or_else(|| {
            ModelFamilyError::InvalidDefinition("model family has no primary component".to_owned())
        })?;
        let component = mapped
            .components
            .get(primary.identifier)
            .ok_or_else(|| ModelFamilyError::MissingRequiredComponent(primary.identifier.into()))?;
        for key in definition.required_keys {
            if !component.contains_key(*key) {
                return Err(ModelFamilyError::MissingRequiredKey((*key).to_owned()));
            }
        }
    } else {
        for schema in schemas {
            let component_definition = definition
                .components
                .iter()
                .find(|component| component.identifier == schema.component)
                .ok_or_else(|| {
                    ModelFamilyError::UndeclaredComponent(schema.component.to_owned())
                })?;
            let Some(component) = mapped.components.get(schema.component) else {
                if component_definition.required {
                    return Err(ModelFamilyError::MissingRequiredComponent(
                        schema.component.to_owned(),
                    ));
                }
                continue;
            };
            for key in schema.required_keys {
                if !component.contains_key(*key) {
                    return Err(ModelFamilyError::MissingComponentKey {
                        component: schema.component.to_owned(),
                        key: (*key).to_owned(),
                    });
                }
            }
            if !schema.allow_unexpected {
                let allowed = schema
                    .required_keys
                    .iter()
                    .chain(schema.optional_keys)
                    .copied()
                    .collect::<BTreeSet<_>>();
                if let Some(key) = component.keys().find(|key| !allowed.contains(key.as_str())) {
                    return Err(ModelFamilyError::UnexpectedComponentKey {
                        component: schema.component.to_owned(),
                        key: key.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_mapping_probe_snapshot(
    expected: &BTreeMap<String, Vec<u64>>,
    source: &BTreeMap<String, Tensor>,
) -> Result<(), ModelFamilyError> {
    if expected.len() != source.len() {
        return Err(ModelFamilyError::ResolvedProbeDrift(
            "source tensor key count changed after resolution".to_owned(),
        ));
    }
    for (key, expected_shape) in expected {
        let tensor = source.get(key).ok_or_else(|| {
            ModelFamilyError::ResolvedProbeDrift(format!(
                "source tensor {key} disappeared after resolution"
            ))
        })?;
        if tensor.descriptor().shape() != expected_shape {
            return Err(ModelFamilyError::ResolvedProbeDrift(format!(
                "source tensor {key} shape changed from {expected_shape:?} to {:?}",
                tensor.descriptor().shape()
            )));
        }
    }
    Ok(())
}

fn model_profile_identity(profile: &ModelFamilyProfile) -> String {
    let mut digest = Sha256::new();
    digest.update(b"sim.comfy.model-family-profile.v1\0");
    digest.update(format!("{profile:?}").as_bytes());
    format!("{:x}", digest.finalize())
}

fn model_probe_identity(probe: &ModelProbe) -> String {
    let mut digest = Sha256::new();
    digest.update(b"sim.comfy.model-family-probe.v1\0");
    digest.update(format!("{:?}\0{:?}", probe.tensor_shapes, probe.metadata).as_bytes());
    format!("{:x}", digest.finalize())
}

fn validate_weight_binding(
    weights: &MappedModelWeights,
    definition: &ModelFamilyDefinition,
    profile: &ModelFamilyProfile,
    state_plan_identity: &str,
    probe_identity: Option<&str>,
) -> Result<(), ModelFamilyError> {
    let binding = weights.binding.as_deref().ok_or_else(|| {
        ModelFamilyError::WeightBindingMismatch("weights have no family binding".to_owned())
    })?;
    let expected_family = identity_for(definition)?;
    if binding.family != expected_family {
        return Err(ModelFamilyError::WeightBindingMismatch(format!(
            "weights belong to {}, build requested {}",
            binding.family.identifier(),
            expected_family.identifier()
        )));
    }
    if binding.profile_identity != model_profile_identity(profile) {
        return Err(ModelFamilyError::WeightBindingMismatch(
            "selected profile differs from mapped profile".to_owned(),
        ));
    }
    if binding.state_plan_identity != state_plan_identity {
        return Err(ModelFamilyError::WeightBindingMismatch(
            "selected state plan differs from mapped state plan".to_owned(),
        ));
    }
    if let Some(probe_identity) = probe_identity {
        if binding.probe_identity.as_deref() != Some(probe_identity) {
            return Err(ModelFamilyError::WeightBindingMismatch(
                "resolved model probe differs from mapped probe".to_owned(),
            ));
        }
    }
    Ok(())
}

fn preflight_state_transaction(
    plan: &ModelStateTransformPlan,
    source: &BTreeMap<String, Tensor>,
    cancellation: &CancellationToken,
) -> Result<ModelStatePreflight, ModelFamilyError> {
    cancellation.check()?;
    let mut handled = BTreeSet::new();
    let mut consumed = BTreeSet::<ModelStateTensorReference>::new();
    let mut output_keys = BTreeSet::<(String, String)>::new();
    for (operation_index, operation) in plan.operations.iter().enumerate() {
        if operation_index % 64 == 0 {
            cancellation.check()?;
        }
        match operation {
            ModelStateTransformOperation::Move {
                selector,
                rewrite,
                component,
            }
            | ModelStateTransformOperation::Route {
                selector,
                rewrite,
                component,
            } => {
                for source_key in selector.select(source, cancellation)? {
                    claim_consumed_reference(
                        &mut consumed,
                        ModelStateTensorReference::Source(source_key.to_owned()),
                    )?;
                    handled.insert(source_key.to_owned());
                    claim_output(
                        &mut output_keys,
                        component.clone(),
                        rewrite.apply(source_key)?,
                    )?;
                }
            }
            ModelStateTransformOperation::Copy {
                selector,
                rewrite,
                component,
            } => {
                for source_key in selector.select(source, cancellation)? {
                    handled.insert(source_key.to_owned());
                    claim_output(
                        &mut output_keys,
                        component.clone(),
                        rewrite.apply(source_key)?,
                    )?;
                }
            }
            ModelStateTransformOperation::Drop { selector } => {
                for source_key in selector.select(source, cancellation)? {
                    claim_consumed_reference(
                        &mut consumed,
                        ModelStateTensorReference::Source(source_key.to_owned()),
                    )?;
                    handled.insert(source_key.to_owned());
                }
            }
            ModelStateTransformOperation::Split {
                source: source_key,
                dimension,
                sizes,
                outputs,
            } => {
                let tensor = preflight_tensor_reference(
                    source_key,
                    source,
                    &output_keys,
                    &mut consumed,
                    &mut handled,
                )?;
                let evaluated = evaluate_shape(sizes, source)?;
                let covered = evaluated.iter().try_fold(0_u64, |total, size| {
                    total
                        .checked_add(*size)
                        .ok_or(ModelFamilyError::DimensionExpressionOverflow)
                })?;
                if let Some(tensor) = tensor {
                    let axis_size = tensor
                        .descriptor()
                        .shape()
                        .get(*dimension)
                        .copied()
                        .ok_or_else(|| ModelFamilyError::DimensionOutOfBounds {
                            key: tensor_reference_label(source_key),
                            dimension: *dimension,
                        })?;
                    if covered != axis_size {
                        return Err(ModelFamilyError::IncompleteAssembly {
                            key: tensor_reference_label(source_key),
                            expected: axis_size,
                            actual: covered,
                        });
                    }
                }
                for output in outputs {
                    claim_output(
                        &mut output_keys,
                        output.component.clone(),
                        output.key.clone(),
                    )?;
                }
            }
            ModelStateTransformOperation::Assemble {
                sources,
                dimension,
                output,
            } => {
                let mut first: Option<&Tensor> = None;
                let mut assembled_axis = 0_u64;
                let mut all_source_tensors = true;
                for source_key in sources {
                    let Some(tensor) = preflight_tensor_reference(
                        source_key,
                        source,
                        &output_keys,
                        &mut consumed,
                        &mut handled,
                    )?
                    else {
                        all_source_tensors = false;
                        continue;
                    };
                    let axis_size = tensor
                        .descriptor()
                        .shape()
                        .get(*dimension)
                        .copied()
                        .ok_or_else(|| ModelFamilyError::DimensionOutOfBounds {
                            key: tensor_reference_label(source_key),
                            dimension: *dimension,
                        })?;
                    if let Some(first) = first {
                        validate_assembly_tensor(
                            first,
                            tensor,
                            *dimension,
                            &tensor_reference_label(source_key),
                        )?;
                    } else {
                        first = Some(tensor);
                    }
                    assembled_axis = assembled_axis
                        .checked_add(axis_size)
                        .ok_or(ModelFamilyError::DimensionExpressionOverflow)?;
                }
                if all_source_tensors && assembled_axis == 0 {
                    return Err(ModelFamilyError::InvalidStateTransform(
                        "assembly cannot have zero total coverage".to_owned(),
                    ));
                }
                claim_output(
                    &mut output_keys,
                    output.component.clone(),
                    output.key.clone(),
                )?;
            }
            ModelStateTransformOperation::Transpose {
                source: source_key,
                first_dimension,
                second_dimension,
                output,
            } => {
                if let Some(tensor) = preflight_tensor_reference(
                    source_key,
                    source,
                    &output_keys,
                    &mut consumed,
                    &mut handled,
                )? {
                    if *first_dimension >= tensor.descriptor().rank()
                        || *second_dimension >= tensor.descriptor().rank()
                    {
                        return Err(ModelFamilyError::DimensionOutOfBounds {
                            key: tensor_reference_label(source_key),
                            dimension: (*first_dimension).max(*second_dimension),
                        });
                    }
                }
                checked_i64(*first_dimension)?;
                checked_i64(*second_dimension)?;
                claim_output(
                    &mut output_keys,
                    output.component.clone(),
                    output.key.clone(),
                )?;
            }
            ModelStateTransformOperation::Reshape {
                source: source_key,
                shape,
                output,
            } => {
                let tensor = preflight_tensor_reference(
                    source_key,
                    source,
                    &output_keys,
                    &mut consumed,
                    &mut handled,
                )?;
                let shape = evaluate_shape(shape, source)?;
                if let Some(tensor) = tensor {
                    if checked_element_count(&shape)? != tensor.descriptor().element_count()? {
                        return Err(ModelFamilyError::ReshapeElementCount {
                            key: tensor_reference_label(source_key),
                            source_elements: tensor.descriptor().element_count()?,
                            target_elements: checked_element_count(&shape)?,
                        });
                    }
                }
                for dimension in &shape {
                    checked_i64_u64(*dimension)?;
                }
                claim_output(
                    &mut output_keys,
                    output.component.clone(),
                    output.key.clone(),
                )?;
            }
            ModelStateTransformOperation::ConditionalRound {
                source: source_key,
                output,
                ..
            } => {
                preflight_tensor_reference(
                    source_key,
                    source,
                    &output_keys,
                    &mut consumed,
                    &mut handled,
                )?;
                claim_output(
                    &mut output_keys,
                    output.component.clone(),
                    output.key.clone(),
                )?;
            }
            ModelStateTransformOperation::Generate { shape, output, .. } => {
                let shape = evaluate_shape(shape, source)?;
                checked_element_count(&shape)?;
                claim_output(
                    &mut output_keys,
                    output.component.clone(),
                    output.key.clone(),
                )?;
            }
            ModelStateTransformOperation::GenerateArange { shape, output, .. } => {
                let shape = evaluate_shape(shape, source)?;
                checked_element_count(&shape)?;
                claim_output(
                    &mut output_keys,
                    output.component.clone(),
                    output.key.clone(),
                )?;
            }
            ModelStateTransformOperation::Narrow {
                source: source_key,
                dimension,
                start,
                length,
                output,
            } => {
                if let Some(tensor) = preflight_tensor_reference(
                    source_key,
                    source,
                    &output_keys,
                    &mut consumed,
                    &mut handled,
                )? {
                    tensor
                        .descriptor()
                        .narrowed_view(*dimension, *start, *length)?;
                }
                checked_i64(*dimension)?;
                claim_output(
                    &mut output_keys,
                    output.component.clone(),
                    output.key.clone(),
                )?;
            }
            ModelStateTransformOperation::Permute {
                source: source_key,
                dimensions,
                output,
            } => {
                if let Some(tensor) = preflight_tensor_reference(
                    source_key,
                    source,
                    &output_keys,
                    &mut consumed,
                    &mut handled,
                )? {
                    validate_permutation(dimensions, tensor.descriptor().rank())?;
                }
                claim_output(
                    &mut output_keys,
                    output.component.clone(),
                    output.key.clone(),
                )?;
            }
            ModelStateTransformOperation::Expand {
                source: source_key,
                shape,
                output,
            } => {
                preflight_tensor_reference(
                    source_key,
                    source,
                    &output_keys,
                    &mut consumed,
                    &mut handled,
                )?;
                let shape = evaluate_shape(shape, source)?;
                for dimension in shape {
                    checked_i64_u64(dimension)?;
                }
                claim_output(
                    &mut output_keys,
                    output.component.clone(),
                    output.key.clone(),
                )?;
            }
            ModelStateTransformOperation::TransformEach {
                selector,
                rewrite,
                component,
                transform,
            } => {
                for source_key in selector.select(source, cancellation)? {
                    claim_consumed_reference(
                        &mut consumed,
                        ModelStateTensorReference::Source(source_key.to_owned()),
                    )?;
                    handled.insert(source_key.to_owned());
                    preflight_per_tensor_transform(
                        transform_source(source, source_key)?,
                        transform,
                        source,
                        source_key,
                    )?;
                    claim_output(
                        &mut output_keys,
                        component.clone(),
                        rewrite.apply(source_key)?,
                    )?;
                }
            }
            ModelStateTransformOperation::TransformBranchesEach {
                selector,
                pre_transform,
                outputs,
            } => {
                for source_key in selector.select(source, cancellation)? {
                    claim_consumed_reference(
                        &mut consumed,
                        ModelStateTensorReference::Source(source_key.to_owned()),
                    )?;
                    handled.insert(source_key.to_owned());
                    let selected = transform_source(source, source_key)?;
                    let intermediate_shape = preflight_per_tensor_transform_shape(
                        selected.descriptor().shape(),
                        pre_transform,
                        source,
                        source_key,
                    )?;
                    for output in outputs {
                        preflight_per_tensor_transform_shape(
                            &intermediate_shape,
                            &output.transform,
                            source,
                            source_key,
                        )?;
                        claim_output(
                            &mut output_keys,
                            output.component.clone(),
                            output.rewrite.apply(source_key)?,
                        )?;
                    }
                }
            }
            ModelStateTransformOperation::SplitEach {
                selector,
                dimension,
                sizes,
                outputs,
            } => {
                for source_key in selector.select(source, cancellation)? {
                    claim_consumed_reference(
                        &mut consumed,
                        ModelStateTensorReference::Source(source_key.to_owned()),
                    )?;
                    handled.insert(source_key.to_owned());
                    let tensor = transform_source(source, source_key)?;
                    let sizes = evaluate_shape_for_tensor(sizes, source, tensor)?;
                    let covered = sizes.iter().try_fold(0_u64, |total, size| {
                        total
                            .checked_add(*size)
                            .ok_or(ModelFamilyError::DimensionExpressionOverflow)
                    })?;
                    let expected = tensor
                        .descriptor()
                        .shape()
                        .get(*dimension)
                        .copied()
                        .ok_or_else(|| ModelFamilyError::DimensionOutOfBounds {
                            key: source_key.to_owned(),
                            dimension: *dimension,
                        })?;
                    if covered != expected {
                        return Err(ModelFamilyError::IncompleteAssembly {
                            key: source_key.to_owned(),
                            expected,
                            actual: covered,
                        });
                    }
                    for output in outputs {
                        claim_output(
                            &mut output_keys,
                            output.component.clone(),
                            output.rewrite.apply(source_key)?,
                        )?;
                    }
                }
            }
        }
    }
    let unmatched = source
        .keys()
        .filter(|key| !handled.contains(*key))
        .collect::<Vec<_>>();
    match &plan.unmatched {
        ModelUnmatchedKeyDisposition::Reject if !unmatched.is_empty() => {
            return Err(ModelFamilyError::UnexpectedKeys(
                unmatched.into_iter().cloned().collect(),
            ));
        }
        ModelUnmatchedKeyDisposition::Route { component, rewrite } => {
            for key in unmatched {
                claim_output(&mut output_keys, component.clone(), rewrite.apply(key)?)?;
            }
        }
        ModelUnmatchedKeyDisposition::Reject | ModelUnmatchedKeyDisposition::Drop => {}
    }
    cancellation.check()?;
    Ok(ModelStatePreflight { handled })
}

fn execute_state_operation(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    operation: &ModelStateTransformOperation,
    source: &BTreeMap<String, Tensor>,
    components: &mut BTreeMap<String, BTreeMap<String, Tensor>>,
) -> Result<(), ModelFamilyError> {
    match operation {
        ModelStateTransformOperation::Move {
            selector,
            rewrite,
            component,
        }
        | ModelStateTransformOperation::Copy {
            selector,
            rewrite,
            component,
        }
        | ModelStateTransformOperation::Route {
            selector,
            rewrite,
            component,
        } => {
            for source_key in selector.select(source, context.cancellation)? {
                insert_state_output(
                    components,
                    component,
                    rewrite.apply(source_key)?,
                    transform_source(source, source_key)?.clone(),
                )?;
            }
        }
        ModelStateTransformOperation::Drop { .. } => {}
        ModelStateTransformOperation::Split {
            source: source_key,
            dimension,
            sizes,
            outputs,
        } => {
            let sizes = evaluate_shape(sizes, source)?;
            let dimension = checked_i64(*dimension)?;
            let tensors = tensor_split_exact_native(
                resolve_state_tensor(source_key, source, components)?,
                &TensorSplitSpec::Sizes(sizes),
                dimension,
                context.cancellation,
            )?;
            for (output, tensor) in outputs.iter().zip(tensors) {
                insert_state_output(components, &output.component, output.key.clone(), tensor)?;
            }
        }
        ModelStateTransformOperation::Assemble {
            sources,
            dimension,
            output,
        } => {
            let tensors = sources
                .iter()
                .map(|key| resolve_state_tensor(key, source, components).cloned())
                .collect::<Result<Vec<_>, _>>()?;
            let tensor = concatenate_with_context_exact_native(
                backend,
                &tensors,
                checked_i64(*dimension)?,
                context,
            )?;
            insert_state_output(components, &output.component, output.key.clone(), tensor)?;
        }
        ModelStateTransformOperation::Transpose {
            source: source_key,
            first_dimension,
            second_dimension,
            output,
        } => {
            let tensor = tensor_transpose_exact_native(
                resolve_state_tensor(source_key, source, components)?,
                checked_i64(*first_dimension)?,
                checked_i64(*second_dimension)?,
                context.cancellation,
            )?;
            insert_state_output(components, &output.component, output.key.clone(), tensor)?;
        }
        ModelStateTransformOperation::Reshape {
            source: source_key,
            shape,
            output,
        } => {
            let shape = evaluate_shape(shape, source)?
                .into_iter()
                .map(checked_i64_u64)
                .collect::<Result<Vec<_>, _>>()?;
            let tensor = tensor_reshape_with_context_exact_native(
                backend,
                resolve_state_tensor(source_key, source, components)?,
                &shape,
                context,
            )?;
            insert_state_output(components, &output.component, output.key.clone(), tensor)?;
        }
        ModelStateTransformOperation::ConditionalRound {
            source: source_key,
            decimals,
            condition,
            output,
        } => {
            let source_tensor = resolve_state_tensor(source_key, source, components)?;
            let tensor = if condition.matches(source_tensor) {
                round_method_with_context_exact_native(backend, source_tensor, *decimals, context)?
            } else {
                source_tensor.clone()
            };
            insert_state_output(components, &output.component, output.key.clone(), tensor)?;
        }
        ModelStateTransformOperation::Generate {
            shape,
            fill,
            dtype,
            output,
        } => {
            let shape = evaluate_shape(shape, source)?;
            let tensor = full_with_context_exact_native(
                backend,
                &shape,
                *fill,
                Some(*dtype),
                Layout::Strided,
                DeviceId::CPU,
                false,
                None,
                context,
            )?;
            insert_state_output(components, &output.component, output.key.clone(), tensor)?;
        }
        ModelStateTransformOperation::GenerateArange {
            start,
            end,
            step,
            dtype,
            shape,
            output,
        } => {
            let tensor = arange_with_context_exact_native(
                backend,
                *start,
                *end,
                *step,
                Some(*dtype),
                Layout::Strided,
                DeviceId::CPU,
                false,
                None,
                context,
            )?;
            let shape = evaluate_shape(shape, source)?
                .into_iter()
                .map(checked_i64_u64)
                .collect::<Result<Vec<_>, _>>()?;
            let tensor = tensor_expand_exact_native(&tensor, &shape, context.cancellation)?;
            insert_state_output(components, &output.component, output.key.clone(), tensor)?;
        }
        ModelStateTransformOperation::Narrow {
            source: source_key,
            dimension,
            start,
            length,
            output,
        } => {
            let tensor = narrow_method_exact_native(
                resolve_state_tensor(source_key, source, components)?,
                checked_i64(*dimension)?,
                *start,
                *length,
                context.cancellation,
            )?;
            insert_state_output(components, &output.component, output.key.clone(), tensor)?;
        }
        ModelStateTransformOperation::Permute {
            source: source_key,
            dimensions,
            output,
        } => {
            let dimensions = dimensions
                .iter()
                .copied()
                .map(checked_i64)
                .collect::<Result<Vec<_>, _>>()?;
            let tensor = tensor_permute_exact_native(
                resolve_state_tensor(source_key, source, components)?,
                &dimensions,
                context.cancellation,
            )?;
            insert_state_output(components, &output.component, output.key.clone(), tensor)?;
        }
        ModelStateTransformOperation::Expand {
            source: source_key,
            shape,
            output,
        } => {
            let shape = evaluate_shape(shape, source)?
                .into_iter()
                .map(checked_i64_u64)
                .collect::<Result<Vec<_>, _>>()?;
            let tensor = tensor_expand_exact_native(
                resolve_state_tensor(source_key, source, components)?,
                &shape,
                context.cancellation,
            )?;
            insert_state_output(components, &output.component, output.key.clone(), tensor)?;
        }
        ModelStateTransformOperation::TransformEach {
            selector,
            rewrite,
            component,
            transform,
        } => {
            for source_key in selector.select(source, context.cancellation)? {
                let tensor = execute_per_tensor_transform(
                    backend,
                    context,
                    transform_source(source, source_key)?,
                    transform,
                    source,
                )?;
                insert_state_output(components, component, rewrite.apply(source_key)?, tensor)?;
            }
        }
        ModelStateTransformOperation::TransformBranchesEach {
            selector,
            pre_transform,
            outputs,
        } => {
            for source_key in selector.select(source, context.cancellation)? {
                context.cancellation.check()?;
                let intermediate = execute_per_tensor_transform(
                    backend,
                    context,
                    transform_source(source, source_key)?,
                    pre_transform,
                    source,
                )?;
                for output in outputs {
                    context.cancellation.check()?;
                    let tensor = execute_per_tensor_transform(
                        backend,
                        context,
                        &intermediate,
                        &output.transform,
                        source,
                    )?;
                    insert_state_output(
                        components,
                        &output.component,
                        output.rewrite.apply(source_key)?,
                        tensor,
                    )?;
                }
            }
        }
        ModelStateTransformOperation::SplitEach {
            selector,
            dimension,
            sizes,
            outputs,
        } => {
            for source_key in selector.select(source, context.cancellation)? {
                let selected = transform_source(source, source_key)?;
                let sizes = evaluate_shape_for_tensor(sizes, source, selected)?;
                let tensors = tensor_split_exact_native(
                    selected,
                    &TensorSplitSpec::Sizes(sizes.clone()),
                    checked_i64(*dimension)?,
                    context.cancellation,
                )?;
                for (output, tensor) in outputs.iter().zip(tensors) {
                    insert_state_output(
                        components,
                        &output.component,
                        output.rewrite.apply(source_key)?,
                        tensor,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn execute_per_tensor_transform(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    tensor: &Tensor,
    transform: &ModelPerTensorTransform,
    source: &BTreeMap<String, Tensor>,
) -> Result<Tensor, ModelFamilyError> {
    Ok(match transform {
        ModelPerTensorTransform::Sequence(transforms) => {
            let mut output = tensor.clone();
            for transform in transforms {
                output =
                    execute_per_tensor_transform(backend, context, &output, transform, source)?;
            }
            output
        }
        ModelPerTensorTransform::Contiguous => contiguous_with_context_exact_native(
            backend,
            tensor,
            MemoryFormatReference::Layout(Layout::Contiguous),
            context,
        )?,
        ModelPerTensorTransform::Narrow {
            dimension,
            start,
            length,
        } => narrow_method_exact_native(
            tensor,
            checked_i64(*dimension)?,
            *start,
            *length,
            context.cancellation,
        )?,
        ModelPerTensorTransform::Transpose {
            first_dimension,
            second_dimension,
        } => tensor_transpose_exact_native(
            tensor,
            checked_i64(*first_dimension)?,
            checked_i64(*second_dimension)?,
            context.cancellation,
        )?,
        ModelPerTensorTransform::Permute { dimensions } => {
            let dimensions = dimensions
                .iter()
                .copied()
                .map(checked_i64)
                .collect::<Result<Vec<_>, _>>()?;
            tensor_permute_exact_native(tensor, &dimensions, context.cancellation)?
        }
        ModelPerTensorTransform::Reshape { shape } => {
            let shape = evaluate_shape_for_tensor(shape, source, tensor)?
                .into_iter()
                .map(checked_i64_u64)
                .collect::<Result<Vec<_>, _>>()?;
            tensor_reshape_with_context_exact_native(backend, tensor, &shape, context)?
        }
        ModelPerTensorTransform::Expand { shape } => {
            let shape = evaluate_shape_for_tensor(shape, source, tensor)?
                .into_iter()
                .map(checked_i64_u64)
                .collect::<Result<Vec<_>, _>>()?;
            tensor_expand_exact_native(tensor, &shape, context.cancellation)?
        }
        ModelPerTensorTransform::ConditionalRound {
            decimals,
            condition,
        } => {
            if condition.matches(tensor) {
                round_method_with_context_exact_native(backend, tensor, *decimals, context)?
            } else {
                tensor.clone()
            }
        }
    })
}

fn apply_unmatched(
    disposition: &ModelUnmatchedKeyDisposition,
    source: &BTreeMap<String, Tensor>,
    handled: &BTreeSet<String>,
    components: &mut BTreeMap<String, BTreeMap<String, Tensor>>,
    cancellation: &CancellationToken,
) -> Result<(), ModelFamilyError> {
    cancellation.check()?;
    if let ModelUnmatchedKeyDisposition::Route { component, rewrite } = disposition {
        for (index, (key, tensor)) in source
            .iter()
            .filter(|(key, _)| !handled.contains(*key))
            .enumerate()
        {
            if index % 256 == 0 {
                cancellation.check()?;
            }
            insert_state_output(components, component, rewrite.apply(key)?, tensor.clone())?;
        }
    }
    cancellation.check()?;
    Ok(())
}

fn validate_assembly_tensor(
    first: &Tensor,
    current: &Tensor,
    dimension: usize,
    key: &str,
) -> Result<(), ModelFamilyError> {
    if first.descriptor().dtype() != current.descriptor().dtype()
        || first.descriptor().device() != current.descriptor().device()
        || first.descriptor().rank() != current.descriptor().rank()
        || first
            .descriptor()
            .shape()
            .iter()
            .zip(current.descriptor().shape())
            .enumerate()
            .any(|(axis, (left, right))| axis != dimension && left != right)
    {
        return Err(ModelFamilyError::AssemblyShapeMismatch(key.to_owned()));
    }
    Ok(())
}

fn claim_consumed_reference(
    consumed: &mut BTreeSet<ModelStateTensorReference>,
    reference: ModelStateTensorReference,
) -> Result<(), ModelFamilyError> {
    if !consumed.insert(reference.clone()) {
        return Err(ModelFamilyError::OverlappingStateSelection(
            tensor_reference_label(&reference),
        ));
    }
    Ok(())
}

fn preflight_tensor_reference<'a>(
    reference: &ModelStateTensorReference,
    source: &'a BTreeMap<String, Tensor>,
    staged_outputs: &BTreeSet<(String, String)>,
    consumed: &mut BTreeSet<ModelStateTensorReference>,
    handled: &mut BTreeSet<String>,
) -> Result<Option<&'a Tensor>, ModelFamilyError> {
    claim_consumed_reference(consumed, reference.clone())?;
    match reference {
        ModelStateTensorReference::Source(key) => {
            handled.insert(key.clone());
            Ok(Some(transform_source(source, key)?))
        }
        ModelStateTensorReference::Staged(target) => {
            if !staged_outputs.contains(&(target.component.clone(), target.key.clone())) {
                return Err(ModelFamilyError::StagedOutputUnavailable {
                    component: target.component.clone(),
                    key: target.key.clone(),
                });
            }
            Ok(None)
        }
    }
}

fn tensor_reference_label(reference: &ModelStateTensorReference) -> String {
    match reference {
        ModelStateTensorReference::Source(key) => key.clone(),
        ModelStateTensorReference::Staged(target) => {
            format!("staged:{}:{}", target.component, target.key)
        }
    }
}

fn claim_output(
    outputs: &mut BTreeSet<(String, String)>,
    component: String,
    key: String,
) -> Result<(), ModelFamilyError> {
    if !outputs.insert((component.clone(), key.clone())) {
        return Err(ModelFamilyError::DuplicateComponentKey { component, key });
    }
    Ok(())
}

fn insert_state_output(
    components: &mut BTreeMap<String, BTreeMap<String, Tensor>>,
    component: &str,
    key: String,
    tensor: Tensor,
) -> Result<(), ModelFamilyError> {
    if components
        .entry(component.to_owned())
        .or_default()
        .insert(key.clone(), tensor)
        .is_some()
    {
        return Err(ModelFamilyError::DuplicateComponentKey {
            component: component.to_owned(),
            key,
        });
    }
    Ok(())
}

fn transform_source<'a>(
    source: &'a BTreeMap<String, Tensor>,
    key: &str,
) -> Result<&'a Tensor, ModelFamilyError> {
    source
        .get(key)
        .ok_or_else(|| ModelFamilyError::MissingTransformSource(key.to_owned()))
}

fn resolve_state_tensor<'a>(
    reference: &ModelStateTensorReference,
    source: &'a BTreeMap<String, Tensor>,
    components: &'a BTreeMap<String, BTreeMap<String, Tensor>>,
) -> Result<&'a Tensor, ModelFamilyError> {
    match reference {
        ModelStateTensorReference::Source(key) => transform_source(source, key),
        ModelStateTensorReference::Staged(target) => components
            .get(&target.component)
            .and_then(|component| component.get(&target.key))
            .ok_or_else(|| ModelFamilyError::StagedOutputUnavailable {
                component: target.component.clone(),
                key: target.key.clone(),
            }),
    }
}

fn validate_permutation(dimensions: &[usize], rank: usize) -> Result<(), ModelFamilyError> {
    if dimensions.len() != rank {
        return Err(ModelFamilyError::InvalidStateTransform(
            "permutation length must equal tensor rank".to_owned(),
        ));
    }
    let values = dimensions.iter().copied().collect::<BTreeSet<_>>();
    if values.len() != rank || values.last().copied().is_some_and(|axis| axis >= rank) {
        return Err(ModelFamilyError::InvalidStateTransform(
            "permutation must contain every tensor axis exactly once".to_owned(),
        ));
    }
    Ok(())
}

fn preflight_per_tensor_transform(
    tensor: &Tensor,
    transform: &ModelPerTensorTransform,
    source: &BTreeMap<String, Tensor>,
    key: &str,
) -> Result<(), ModelFamilyError> {
    preflight_per_tensor_transform_shape(tensor.descriptor().shape(), transform, source, key)?;
    Ok(())
}

fn preflight_per_tensor_transform_shape(
    input_shape: &[u64],
    transform: &ModelPerTensorTransform,
    source: &BTreeMap<String, Tensor>,
    key: &str,
) -> Result<Vec<u64>, ModelFamilyError> {
    let mut shape = input_shape.to_vec();
    match transform {
        ModelPerTensorTransform::Sequence(transforms) => {
            for transform in transforms {
                shape = preflight_per_tensor_transform_shape(&shape, transform, source, key)?;
            }
        }
        ModelPerTensorTransform::Contiguous => {}
        ModelPerTensorTransform::Narrow {
            dimension,
            start,
            length,
        } => {
            let extent = shape.get(*dimension).copied().ok_or_else(|| {
                ModelFamilyError::DimensionOutOfBounds {
                    key: key.to_owned(),
                    dimension: *dimension,
                }
            })?;
            let canonical_start = if *start < 0 {
                i128::from(extent) + i128::from(*start)
            } else {
                i128::from(*start)
            };
            let end = canonical_start
                .checked_add(i128::from(*length))
                .ok_or(ModelFamilyError::DimensionExpressionOverflow)?;
            if canonical_start < 0 || end > i128::from(extent) {
                return Err(ModelFamilyError::InvalidStateTransform(format!(
                    "selector narrow {start}..{end} exceeds dimension {dimension} extent {extent}"
                )));
            }
            shape[*dimension] = *length;
        }
        ModelPerTensorTransform::Transpose {
            first_dimension,
            second_dimension,
        } => {
            if *first_dimension >= shape.len() || *second_dimension >= shape.len() {
                return Err(ModelFamilyError::DimensionOutOfBounds {
                    key: key.to_owned(),
                    dimension: (*first_dimension).max(*second_dimension),
                });
            }
            shape.swap(*first_dimension, *second_dimension);
        }
        ModelPerTensorTransform::Permute { dimensions } => {
            validate_permutation(dimensions, shape.len())?;
            shape = dimensions
                .iter()
                .map(|dimension| shape[*dimension])
                .collect();
        }
        ModelPerTensorTransform::Reshape {
            shape: target_shape,
        } => {
            let target_shape = evaluate_shape_for_selected_shape(target_shape, source, &shape)?;
            let target = checked_element_count(&target_shape)?;
            let source_elements = checked_element_count(&shape)?;
            if target != source_elements {
                return Err(ModelFamilyError::ReshapeElementCount {
                    key: key.to_owned(),
                    source_elements,
                    target_elements: target,
                });
            }
            shape = target_shape;
        }
        ModelPerTensorTransform::Expand {
            shape: target_shape,
        } => {
            let target_shape = evaluate_shape_for_selected_shape(target_shape, source, &shape)?;
            if target_shape.len() < shape.len() {
                return Err(ModelFamilyError::InvalidStateTransform(
                    "selector expand cannot reduce tensor rank".to_owned(),
                ));
            }
            let rank_offset = target_shape.len() - shape.len();
            for (index, source_dimension) in shape.iter().enumerate() {
                let target_dimension = target_shape[rank_offset + index];
                if *source_dimension != 1 && *source_dimension != target_dimension {
                    return Err(ModelFamilyError::InvalidStateTransform(format!(
                        "selector expand dimension {source_dimension} cannot become {target_dimension}"
                    )));
                }
            }
            for dimension in &target_shape {
                checked_i64_u64(*dimension)?;
            }
            shape = target_shape;
        }
        ModelPerTensorTransform::ConditionalRound { .. } => {}
    }
    Ok(shape)
}

fn evaluate_shape(
    expressions: &[ModelDimensionExpression],
    source: &BTreeMap<String, Tensor>,
) -> Result<Vec<u64>, ModelFamilyError> {
    evaluate_shape_with_context(expressions, ModelDimensionEvaluationContext::source(source))
}

fn evaluate_shape_for_tensor(
    expressions: &[ModelDimensionExpression],
    source: &BTreeMap<String, Tensor>,
    tensor: &Tensor,
) -> Result<Vec<u64>, ModelFamilyError> {
    evaluate_shape_with_context(
        expressions,
        ModelDimensionEvaluationContext::selected(source, tensor),
    )
}

fn evaluate_shape_for_selected_shape(
    expressions: &[ModelDimensionExpression],
    source: &BTreeMap<String, Tensor>,
    shape: &[u64],
) -> Result<Vec<u64>, ModelFamilyError> {
    evaluate_shape_with_context(
        expressions,
        ModelDimensionEvaluationContext::selected_shape(source, shape),
    )
}

fn evaluate_shape_with_context(
    expressions: &[ModelDimensionExpression],
    context: ModelDimensionEvaluationContext<'_>,
) -> Result<Vec<u64>, ModelFamilyError> {
    expressions
        .iter()
        .map(|expression| expression.evaluate(context))
        .collect()
}

fn checked_element_count(shape: &[u64]) -> Result<u64, ModelFamilyError> {
    shape.iter().try_fold(1_u64, |count, dimension| {
        count
            .checked_mul(*dimension)
            .ok_or(ModelFamilyError::DimensionExpressionOverflow)
    })
}

fn checked_i64(value: usize) -> Result<i64, ModelFamilyError> {
    i64::try_from(value).map_err(|_| ModelFamilyError::DimensionExpressionOverflow)
}

fn checked_i64_u64(value: u64) -> Result<i64, ModelFamilyError> {
    i64::try_from(value).map_err(|_| ModelFamilyError::DimensionExpressionOverflow)
}

fn validate_state_key(value: &str) -> Result<(), ModelFamilyError> {
    if value.is_empty()
        || value.len() > MAX_IDENTITY_BYTES
        || value.contains('\0')
        || value.starts_with('.')
        || value.ends_with('.')
        || value.split('.').any(str::is_empty)
    {
        return Err(ModelFamilyError::InvalidStateKey(value.to_owned()));
    }
    Ok(())
}

fn validate_state_key_fragment(value: &str) -> Result<(), ModelFamilyError> {
    if value.is_empty() || value.len() > MAX_IDENTITY_BYTES || value.contains('\0') {
        return Err(ModelFamilyError::InvalidStateKey(value.to_owned()));
    }
    Ok(())
}

fn validate_state_source_bound(source_count: usize) -> Result<(), ModelFamilyError> {
    if source_count > MAX_STATE_DICTIONARY_SOURCE_TENSORS {
        return Err(ModelFamilyError::StateSourceTooLarge(source_count));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ModelFamilyWeightBinding {
    family: ModelFamilyIdentity,
    profile_identity: String,
    state_plan_identity: String,
    probe_identity: Option<String>,
}

impl ModelFamilyWeightBinding {
    pub fn family(&self) -> &ModelFamilyIdentity {
        &self.family
    }
    pub fn profile_identity(&self) -> &str {
        &self.profile_identity
    }
    pub fn state_plan_identity(&self) -> &str {
        &self.state_plan_identity
    }
    pub fn probe_identity(&self) -> Option<&str> {
        self.probe_identity.as_deref()
    }
}

#[derive(Clone, Debug)]
pub struct MappedModelWeights {
    base_artifact_digest: String,
    cache_identity: String,
    binding: Option<Arc<ModelFamilyWeightBinding>>,
    unpatched_tensors: Arc<BTreeMap<String, Tensor>>,
    tensors: Arc<BTreeMap<String, Tensor>>,
    unexpected_keys: Arc<[String]>,
}

impl MappedModelWeights {
    #[cfg(feature = "test-support")]
    pub fn from_test_parts(
        base_artifact_digest: String,
        tensors: BTreeMap<String, Tensor>,
        unexpected_keys: Vec<String>,
    ) -> Result<Self, ModelFamilyError> {
        validate_digest(&base_artifact_digest)?;
        Ok(Self::from_parts(
            base_artifact_digest,
            tensors,
            unexpected_keys,
        ))
    }

    pub fn base_artifact_digest(&self) -> &str {
        &self.base_artifact_digest
    }
    pub fn cache_identity(&self) -> &str {
        &self.cache_identity
    }
    pub fn binding(&self) -> Option<&ModelFamilyWeightBinding> {
        self.binding.as_deref()
    }
    pub fn tensors(&self) -> &BTreeMap<String, Tensor> {
        &self.tensors
    }
    pub(crate) fn unpatched_tensors(&self) -> &BTreeMap<String, Tensor> {
        &self.unpatched_tensors
    }
    pub fn unexpected_keys(&self) -> &[String] {
        &self.unexpected_keys
    }

    pub(crate) fn from_parts(
        base_artifact_digest: String,
        tensors: BTreeMap<String, Tensor>,
        unexpected_keys: Vec<String>,
    ) -> Self {
        let tensors = Arc::new(tensors);
        Self {
            cache_identity: base_artifact_digest.clone(),
            base_artifact_digest,
            binding: None,
            unpatched_tensors: tensors.clone(),
            tensors,
            unexpected_keys: unexpected_keys.into(),
        }
    }

    pub(crate) fn with_patch_graph_identity(
        mut self,
        ordered_patch_graph_digest: &str,
    ) -> Result<Self, ModelFamilyError> {
        validate_digest(ordered_patch_graph_digest)?;
        let mut digest = Sha256::new();
        digest.update(b"sim.comfy.model-weights-cache-identity.v1\0");
        digest.update(self.cache_identity.as_bytes());
        digest.update([0]);
        digest.update(ordered_patch_graph_digest.as_bytes());
        self.cache_identity = format!("{:x}", digest.finalize());
        Ok(self)
    }

    pub(crate) fn with_tensors_preserving_identity(
        &self,
        tensors: BTreeMap<String, Tensor>,
    ) -> Self {
        Self {
            base_artifact_digest: self.base_artifact_digest.clone(),
            cache_identity: self.cache_identity.clone(),
            binding: self.binding.clone(),
            unpatched_tensors: self.unpatched_tensors.clone(),
            tensors: Arc::new(tensors),
            unexpected_keys: self.unexpected_keys.clone(),
        }
    }

    fn bind(mut self, binding: ModelFamilyWeightBinding) -> Result<Self, ModelFamilyError> {
        let mut digest = Sha256::new();
        digest.update(b"sim.comfy.model-weights-binding.v1\0");
        for value in [
            self.base_artifact_digest.as_str(),
            binding.family.feature_id(),
            binding.family.identifier(),
            binding.family.architecture_version(),
            binding.profile_identity.as_str(),
            binding.state_plan_identity.as_str(),
            binding.probe_identity.as_deref().unwrap_or("none"),
        ] {
            let length = u64::try_from(value.len())
                .map_err(|_| ModelFamilyError::DimensionExpressionOverflow)?;
            digest.update(length.to_le_bytes());
            digest.update(value.as_bytes());
        }
        self.cache_identity = format!("{:x}", digest.finalize());
        self.binding = Some(Arc::new(binding));
        Ok(self)
    }
}

pub fn map_model_weights(
    definition: &'static ModelFamilyDefinition,
    base_artifact_digest: impl Into<String>,
    source: BTreeMap<String, Tensor>,
) -> Result<MappedModelWeights, ModelFamilyError> {
    validate_definition(definition)?;
    let base_artifact_digest = base_artifact_digest.into();
    validate_digest(&base_artifact_digest)?;
    validate_state_source_bound(source.len())?;
    let mut unexpected = Vec::<String>::new();
    let mut matched_rules = BTreeSet::new();
    let mut staged_components = BTreeMap::new();
    for (source_key, tensor) in &source {
        let Some((rule_index, rule)) = definition
            .weight_rules
            .iter()
            .enumerate()
            .find(|(_, rule)| source_key.starts_with(rule.source_prefix))
        else {
            unexpected.push(source_key.clone());
            continue;
        };
        matched_rules.insert(rule_index);
        let rewrite = ModelKeyRewrite::prefix(rule.source_prefix, rule.target_prefix)?;
        insert_state_output(
            &mut staged_components,
            "model",
            rewrite.apply(source_key)?,
            tensor.clone(),
        )
        .map_err(|error| match error {
            ModelFamilyError::DuplicateComponentKey { key, .. } => {
                ModelFamilyError::DuplicateMappedKey(key)
            }
            error => error,
        })?;
    }
    for (rule_index, rule) in definition.weight_rules.iter().enumerate() {
        if rule.required && !matched_rules.contains(&rule_index) {
            return Err(ModelFamilyError::MissingRequiredPrefix(
                rule.source_prefix.to_owned(),
            ));
        }
    }
    let committed = finish_state_transaction(base_artifact_digest, staged_components);
    let mut components = Arc::unwrap_or_clone(committed.components);
    let mapped = components.remove("model").unwrap_or_default();
    for required in definition.required_keys {
        if !mapped.contains_key(*required) {
            return Err(ModelFamilyError::MissingRequiredKey((*required).to_owned()));
        }
    }
    unexpected.sort();
    MappedModelWeights::from_parts(committed.base_artifact_digest, mapped, unexpected).bind(
        ModelFamilyWeightBinding {
            family: identity_for(definition)?,
            profile_identity: model_profile_identity(&ModelFamilyProfile::from_definition(
                definition,
            )),
            state_plan_identity: "legacy-definition-rules-v1".to_owned(),
            probe_identity: None,
        },
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeFamilyBuildOptions {
    pub dtype: DType,
    pub device: DeviceKind,
    pub activation_elements: u64,
    pub memory_budget_bytes: u64,
    pub allow_unexpected_weights: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelMemoryEstimate {
    pub parameter_elements: u64,
    pub weight_bytes: u64,
    pub activation_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct NativeFamilyModel {
    definition: &'static ModelFamilyDefinition,
    profile: ModelFamilyProfile,
    source: Option<(u16, &'static str)>,
    weights: MappedModelWeights,
    options: NativeFamilyBuildOptions,
    memory: ModelMemoryEstimate,
}

impl NativeFamilyModel {
    pub fn identity(&self) -> Result<ModelFamilyIdentity, ModelFamilyError> {
        identity_for(self.definition)
    }
    pub fn memory_estimate(&self) -> ModelMemoryEstimate {
        self.memory
    }
    pub fn profile(&self) -> ModelFamilyProfile {
        self.profile
    }
    pub fn source_ordinal(&self) -> Option<u16> {
        self.source.map(|source| source.0)
    }
    pub fn source_architecture(&self) -> Option<&'static str> {
        self.source.map(|source| source.1)
    }
    pub fn weights(&self) -> &MappedModelWeights {
        &self.weights
    }

    pub fn with_weights(&self, weights: MappedModelWeights) -> Result<Self, ModelFamilyError> {
        build_model_family_with_profile(
            self.definition,
            self.profile,
            self.source,
            weights,
            self.options,
        )
    }

    pub fn forward_checkpoints(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<ModelForwardCheckpoint>, ModelFamilyError> {
        context.cancellation.check()?;
        if self.options.device != DeviceKind::Cpu {
            return Err(ModelFamilyError::BackendUnavailable(self.options.device));
        }
        if input.descriptor().dtype() != self.options.dtype {
            return Err(ModelFamilyError::DTypeMismatch {
                expected: self.options.dtype,
                actual: input.descriptor().dtype(),
            });
        }
        if input.descriptor().device().kind() != self.options.device {
            return Err(ModelFamilyError::DeviceMismatch {
                expected: self.options.device,
                actual: input.descriptor().device().kind(),
            });
        }
        let mut current = input.clone();
        let mut checkpoints = Vec::new();
        for (step_index, step) in self.profile.forward_program.iter().enumerate() {
            context.cancellation.check()?;
            let next = self.execute_step(backend, &current, *step, context)?;
            context.cancellation.check()?;
            checkpoints.push(ModelForwardCheckpoint {
                name: step.checkpoint.to_owned(),
                step_index,
                tensor: next.clone(),
            });
            current = next;
        }
        Ok(checkpoints)
    }

    fn execute_step(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        step: ModelForwardStep,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, ModelFamilyError> {
        match step.operation {
            ModelForwardOperation::AddWeight(key) => {
                self.execute_elementwise(backend, input, Some(key), 0.0, false, context)
            }
            ModelForwardOperation::MultiplyWeight(key) => {
                self.execute_elementwise(backend, input, Some(key), 1.0, true, context)
            }
            ModelForwardOperation::AddScalar(value) => {
                self.execute_elementwise(backend, input, None, value, false, context)
            }
            ModelForwardOperation::MultiplyScalar(value) => {
                self.execute_elementwise(backend, input, None, value, true, context)
            }
            ModelForwardOperation::Linear {
                weight,
                bias,
                input_features,
                output_features,
            } => {
                let mut module = NativeModule::linear(
                    step.checkpoint,
                    input_features,
                    output_features,
                    bias.is_some(),
                    false,
                )?;
                module.load_dense_parameters(
                    self.program_weight(weight)?.clone(),
                    bias.map(|key| self.program_weight(key).cloned())
                        .transpose()?,
                )?;
                Ok(module.forward_with_context(backend, input, context)?)
            }
            ModelForwardOperation::Convolution1d {
                weight,
                bias,
                input_channels,
                output_channels,
                kernel_size,
                stride,
                padding,
                dilation,
                groups,
            } => {
                let mut module = conv1d_module_exact_native(
                    step.checkpoint,
                    input_channels,
                    output_channels,
                    kernel_size,
                    stride,
                    padding,
                    dilation,
                    groups,
                    bias.is_some(),
                    context.cancellation,
                )?;
                module.load_dense_parameters(
                    self.program_weight(weight)?.clone(),
                    bias.map(|key| self.program_weight(key).cloned())
                        .transpose()?,
                )?;
                Ok(module.forward_with_context(backend, input, context)?)
            }
            ModelForwardOperation::Convolution2d {
                weight,
                bias,
                input_channels,
                output_channels,
                kernel_size,
                stride,
                padding,
                dilation,
                groups,
            } => {
                let mut module = NativeModule::conv_2d(
                    step.checkpoint,
                    input_channels,
                    output_channels,
                    kernel_size,
                    stride,
                    padding,
                    dilation,
                    groups,
                    bias.is_some(),
                    false,
                )?;
                module.load_dense_parameters(
                    self.program_weight(weight)?.clone(),
                    bias.map(|key| self.program_weight(key).cloned())
                        .transpose()?,
                )?;
                Ok(module.forward_with_context(backend, input, context)?)
            }
            ModelForwardOperation::Convolution3d {
                weight,
                bias,
                input_channels,
                output_channels,
                kernel_size,
                stride,
                padding,
                dilation,
                groups,
            } => {
                let mut module = NativeModule::conv_3d(
                    step.checkpoint,
                    input_channels,
                    output_channels,
                    kernel_size,
                    stride,
                    padding,
                    dilation,
                    groups,
                    bias.is_some(),
                    false,
                )?;
                module.load_dense_parameters(
                    self.program_weight(weight)?.clone(),
                    bias.map(|key| self.program_weight(key).cloned())
                        .transpose()?,
                )?;
                Ok(module.forward_with_context(backend, input, context)?)
            }
            ModelForwardOperation::LayerNorm {
                normalized_shape,
                weight,
                bias,
                epsilon,
            } => {
                let mut module = NativeModule::layer_norm(
                    step.checkpoint,
                    normalized_shape.to_vec(),
                    epsilon,
                    weight.is_some(),
                    bias.is_some(),
                    false,
                )?;
                if let Some(weight) = weight {
                    module.load_dense_parameters(
                        self.program_weight(weight)?.clone(),
                        bias.map(|key| self.program_weight(key).cloned())
                            .transpose()?,
                    )?;
                }
                Ok(module.forward_with_context(backend, input, context)?)
            }
            ModelForwardOperation::SelfAttention { heads } => {
                self.execute_self_attention(backend, input, heads, context)
            }
            ModelForwardOperation::Silu => {
                let mut module = NativeModule::silu(step.checkpoint)?;
                Ok(module.forward_with_context(backend, input, context)?)
            }
            ModelForwardOperation::Tanh => {
                let mut module = NativeModule::tanh(step.checkpoint)?;
                Ok(module.forward_with_context(backend, input, context)?)
            }
        }
    }

    fn execute_elementwise(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        weight: Option<&str>,
        scalar: f32,
        multiply: bool,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, ModelFamilyError> {
        let original_dtype = input.descriptor().dtype();
        let input = cast_to_with_context_exact_native(
            backend,
            input,
            DType::F32,
            DeviceId::CPU,
            false,
            false,
            context,
        )?;
        let weight = weight
            .map(|key| {
                cast_to_with_context_exact_native(
                    backend,
                    self.program_weight(key)?,
                    DType::F32,
                    DeviceId::CPU,
                    false,
                    false,
                    context,
                )
                .map_err(ModelFamilyError::from)
            })
            .transpose()?;
        let output = match (weight.as_ref(), multiply) {
            (Some(weight), false) => add_method_with_context_exact_native(
                backend,
                &input,
                ElementwiseOperand::Tensor(weight),
                1.0,
                context,
            )?,
            (Some(weight), true) => {
                mul_method_with_context_exact_native(backend, &input, weight, context)?
            }
            (None, false) => add_method_with_context_exact_native(
                backend,
                &input,
                ElementwiseOperand::Scalar(Scalar::Float(f64::from(scalar))),
                1.0,
                context,
            )?,
            (None, true) => {
                let scalar = tensor_from_f32_with_context_exact_native(
                    backend,
                    &[1],
                    &[scalar],
                    DType::F32,
                    DeviceId::CPU,
                    context,
                )?;
                mul_method_with_context_exact_native(backend, &input, &scalar, context)?
            }
        };
        Ok(cast_to_with_context_exact_native(
            backend,
            &output,
            original_dtype,
            DeviceId::CPU,
            false,
            false,
            context,
        )?)
    }

    fn execute_self_attention(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        heads: usize,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, ModelFamilyError> {
        let shape = input
            .descriptor()
            .shape()
            .iter()
            .map(|dimension| {
                usize::try_from(*dimension).map_err(|_| ModelFamilyError::ForwardShapeOverflow)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let values = tensor_to_f32_with_context_exact_native(backend, input, context)?;
        let output = multihead_attention_projected_with_context_exact_native(
            backend, &values, &shape, &values, &shape, &values, &shape, heads, context,
        )?;
        let output_shape = output
            .shape
            .iter()
            .map(|dimension| {
                u64::try_from(*dimension).map_err(|_| ModelFamilyError::ForwardShapeOverflow)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tensor_from_f32_with_context_exact_native(
            backend,
            &output_shape,
            &output.values,
            self.options.dtype,
            DeviceId::CPU,
            context,
        )?)
    }

    fn program_weight(&self, key: &str) -> Result<&Tensor, ModelFamilyError> {
        self.weights
            .tensors
            .get(key)
            .ok_or_else(|| ModelFamilyError::MissingProgramWeight(key.to_owned()))
    }
}

#[derive(Clone, Debug)]
pub struct ModelForwardCheckpoint {
    pub name: String,
    pub step_index: usize,
    pub tensor: Tensor,
}

pub fn build_model_family(
    definition: &'static ModelFamilyDefinition,
    weights: MappedModelWeights,
    options: NativeFamilyBuildOptions,
) -> Result<NativeFamilyModel, ModelFamilyError> {
    build_model_family_with_profile(
        definition,
        ModelFamilyProfile::from_definition(definition),
        None,
        weights,
        options,
    )
}

pub fn build_model_family_for_probe(
    registry: &ModelFamilyRegistry,
    probe: &ModelProbe,
    weights: MappedModelWeights,
    options: NativeFamilyBuildOptions,
) -> Result<NativeFamilyModel, ModelFamilyError> {
    let resolved = registry.resolve(probe)?;
    validate_weight_binding(
        &weights,
        resolved.definition(),
        &resolved.profile(),
        resolved
            .state_plan()
            .map(ModelStateTransformPlan::identity)
            .unwrap_or("legacy-definition-rules-v1"),
        Some(&resolved.probe_identity),
    )?;
    build_model_family_with_profile(
        resolved.definition(),
        resolved.profile(),
        Some((resolved.source_ordinal(), resolved.source_architecture())),
        weights,
        options,
    )
}

fn build_model_family_with_profile(
    definition: &'static ModelFamilyDefinition,
    profile: ModelFamilyProfile,
    source: Option<(u16, &'static str)>,
    weights: MappedModelWeights,
    options: NativeFamilyBuildOptions,
) -> Result<NativeFamilyModel, ModelFamilyError> {
    validate_definition(definition)?;
    validate_profile(definition, &profile)?;
    if source.is_none() {
        validate_weight_binding(
            &weights,
            definition,
            &profile,
            "legacy-definition-rules-v1",
            None,
        )?;
    }
    if !profile.supported_dtypes.contains(&options.dtype) {
        return Err(ModelFamilyError::UnsupportedDType(options.dtype));
    }
    if !profile.supported_devices.contains(&options.device) {
        return Err(ModelFamilyError::UnsupportedDevice(options.device));
    }
    if !options.allow_unexpected_weights {
        let allowed = definition
            .required_keys
            .iter()
            .chain(definition.optional_keys)
            .copied()
            .collect::<BTreeSet<_>>();
        let mut unexpected = weights.unexpected_keys.to_vec();
        unexpected.extend(
            weights
                .tensors
                .keys()
                .filter(|key| !allowed.contains(key.as_str()))
                .cloned(),
        );
        unexpected.sort();
        unexpected.dedup();
        if !unexpected.is_empty() {
            return Err(ModelFamilyError::UnexpectedKeys(unexpected));
        }
    }
    for (key, tensor) in weights.tensors.iter() {
        if tensor.descriptor().dtype() != options.dtype {
            return Err(ModelFamilyError::WeightDType {
                key: key.clone(),
                expected: options.dtype,
                actual: tensor.descriptor().dtype(),
            });
        }
        if tensor.descriptor().device().kind() != options.device {
            return Err(ModelFamilyError::WeightDevice {
                key: key.clone(),
                expected: options.device,
                actual: tensor.descriptor().device().kind(),
            });
        }
    }
    let memory = estimate_model_memory_with_estimator(
        profile.memory_estimator,
        &weights,
        options.activation_elements,
    )?;
    if memory.total_bytes > options.memory_budget_bytes {
        return Err(ModelFamilyError::OutOfMemory {
            required: memory.total_bytes,
            budget: options.memory_budget_bytes,
        });
    }
    Ok(NativeFamilyModel {
        definition,
        profile,
        source,
        weights,
        options,
        memory,
    })
}

pub fn estimate_model_memory(
    definition: &ModelFamilyDefinition,
    weights: &MappedModelWeights,
    activation_elements: u64,
) -> Result<ModelMemoryEstimate, ModelFamilyError> {
    validate_definition(definition)?;
    estimate_model_memory_with_estimator(definition.memory_estimator, weights, activation_elements)
}

fn estimate_model_memory_with_estimator(
    estimator: MemoryEstimatorDescriptor,
    weights: &MappedModelWeights,
    activation_elements: u64,
) -> Result<ModelMemoryEstimate, ModelFamilyError> {
    let parameter_elements = weights.tensors.values().try_fold(0_u64, |total, tensor| {
        total
            .checked_add(tensor.descriptor().element_count()?)
            .ok_or(ModelFamilyError::MemoryOverflow)
    })?;
    let weight_bytes = parameter_elements
        .checked_mul(u64::from(estimator.bytes_per_parameter))
        .ok_or(ModelFamilyError::MemoryOverflow)?;
    let activation_bytes = activation_elements
        .checked_mul(u64::from(estimator.activation_bytes_per_element))
        .ok_or(ModelFamilyError::MemoryOverflow)?;
    let total_bytes = estimator
        .fixed_bytes
        .checked_add(weight_bytes)
        .and_then(|value| value.checked_add(activation_bytes))
        .ok_or(ModelFamilyError::MemoryOverflow)?;
    Ok(ModelMemoryEstimate {
        parameter_elements,
        weight_bytes,
        activation_bytes,
        total_bytes,
    })
}

fn detection_score(
    rules: &[ModelDetectionRule],
    probe: &ModelProbe,
) -> Result<Option<(u32, Vec<String>)>, ModelFamilyError> {
    let mut total = 0_u32;
    let mut evidence = Vec::new();
    for rule in rules {
        let matched = match *rule {
            ModelDetectionRule::ExactShape { key, shape, .. } => probe
                .tensor_shapes
                .get(key)
                .is_some_and(|actual| actual == shape),
            ModelDetectionRule::KeyPresent { key, .. } => probe.tensor_shapes.contains_key(key),
            ModelDetectionRule::AnyKeyPresent { keys, .. } => keys
                .iter()
                .any(|key| probe.tensor_shapes.contains_key(*key)),
            ModelDetectionRule::AnyTensorDimensionValue {
                keys,
                dimension,
                values,
                ..
            } => keys.iter().any(|key| {
                probe
                    .tensor_shapes
                    .get(*key)
                    .and_then(|shape| shape.get(dimension))
                    .is_some_and(|value| values.contains(value))
            }),
            ModelDetectionRule::AnyTensorFact {
                keys, predicates, ..
            } => keys.iter().any(|key| {
                probe
                    .tensor_shapes
                    .get(*key)
                    .is_some_and(|shape| tensor_fact_matches(shape, predicates))
            }),
            ModelDetectionRule::KeyPrefix {
                prefix,
                minimum_matches,
                ..
            } => {
                probe
                    .tensor_shapes
                    .keys()
                    .filter(|key| key.starts_with(prefix))
                    .count()
                    >= minimum_matches
            }
            ModelDetectionRule::Metadata { key, value, .. } => probe
                .metadata
                .get(key)
                .is_some_and(|actual| actual == value),
        };
        if !matched {
            return Ok(None);
        }
        total = total
            .checked_add(rule.score())
            .ok_or(ModelFamilyError::DetectionScoreOverflow)?;
        evidence.push(format!("{rule:?}"));
    }
    Ok(Some((total, evidence)))
}

fn validate_probe_tensor_name(name: &str) -> Result<(), ModelFamilyError> {
    if name.is_empty()
        || name.len() > MAX_MODEL_PROBE_NAME_BYTES
        || name.contains('\0')
        || name.starts_with(MODEL_PROBE_INTERNAL_PREFIX)
    {
        return Err(ModelFamilyError::InvalidProbeTensorName(name.to_owned()));
    }
    Ok(())
}

fn validate_probe_shape(name: &str, shape: &[u64]) -> Result<(), ModelFamilyError> {
    if shape.len() > MAX_TENSOR_RANK {
        return Err(ModelFamilyError::InvalidProbeShape {
            tensor: name.to_owned(),
            shape: shape.to_vec(),
        });
    }
    shape
        .iter()
        .try_fold(1_u64, |elements, dimension| {
            elements.checked_mul(*dimension)
        })
        .ok_or(ModelFamilyError::ProbeDimensionOverflow)?;
    Ok(())
}

fn validate_probe_format_identity(identity: &str) -> Result<(), ModelFamilyError> {
    if !matches!(
        identity,
        "safetensors"
            | "pytorch_archive"
            | "gguf"
            | "json_config"
            | "json_tokenizer"
            | "yaml_config"
            | "sentence_piece"
            | "tiktoken"
    ) {
        return Err(ModelFamilyError::UnsupportedProbeFormat(
            identity.to_owned(),
        ));
    }
    Ok(())
}

fn validate_probe_metadata(key: &str, value: &str) -> Result<(), ModelFamilyError> {
    if key.is_empty()
        || key.len() > MAX_MODEL_PROBE_NAME_BYTES
        || key.contains('\0')
        || key.starts_with(MODEL_PROBE_INTERNAL_PREFIX)
        || value.contains('\0')
    {
        return Err(ModelFamilyError::InvalidProbeMetadata(key.to_owned()));
    }
    Ok(())
}

fn checked_probe_metadata_bytes(
    current: usize,
    key_bytes: usize,
    value_bytes: usize,
) -> Result<usize, ModelFamilyError> {
    let next = current
        .checked_add(key_bytes)
        .and_then(|value| value.checked_add(value_bytes))
        .ok_or(ModelFamilyError::ProbeMetadataOverflow)?;
    if next > MAX_MODEL_PROBE_METADATA_BYTES {
        return Err(ModelFamilyError::ProbeMetadataLimit {
            actual: next,
            maximum: MAX_MODEL_PROBE_METADATA_BYTES,
        });
    }
    Ok(next)
}

fn validate_model_probe(probe: &ModelProbe) -> Result<(), ModelFamilyError> {
    if probe.tensor_shapes.len() > MAX_MODEL_PROBE_TENSORS {
        return Err(ModelFamilyError::ProbeTensorLimit {
            actual: probe.tensor_shapes.len(),
            maximum: MAX_MODEL_PROBE_TENSORS,
        });
    }
    let mut metadata_bytes = 0_usize;
    for (name, shape) in &probe.tensor_shapes {
        validate_probe_tensor_name(name)?;
        validate_probe_shape(name, shape)?;
    }
    let mut format_indices = BTreeSet::new();
    for (key, value) in &probe.metadata {
        metadata_bytes = checked_probe_metadata_bytes(metadata_bytes, key.len(), value.len())?;
        if let Some(tensor_name) = key.strip_prefix(MODEL_PROBE_DTYPE_PREFIX) {
            if !probe.tensor_shapes.contains_key(tensor_name)
                || normalized_storage_dtype(value).is_none()
            {
                return Err(ModelFamilyError::InvalidProbeMetadata(key.clone()));
            }
        } else if let Some(index) = key.strip_prefix(MODEL_PROBE_FORMAT_PREFIX) {
            validate_probe_format_identity(value)?;
            let index = index
                .parse::<usize>()
                .map_err(|_| ModelFamilyError::InvalidProbeMetadata(key.clone()))?;
            if index >= MAX_MODEL_PROBE_FORMATS || !format_indices.insert(index) {
                return Err(ModelFamilyError::InvalidProbeMetadata(key.clone()));
            }
        } else if key == MODEL_PROBE_UNET_PREFIX {
            if value != select_unet_prefix(probe)?.prefix() {
                return Err(ModelFamilyError::InvalidProbeConfiguration(
                    "stored UNet prefix does not match tensor facts".to_owned(),
                ));
            }
        } else {
            validate_probe_metadata(key, value)?;
        }
    }
    if format_indices.iter().copied().ne(0..format_indices.len()) {
        return Err(ModelFamilyError::InvalidProbeMetadata(
            "format document ordinals are not consecutive".to_owned(),
        ));
    }
    Ok(())
}

fn normalize_storage_dtype(value: &str) -> Result<ModelStorageDType, ModelFamilyError> {
    let normalized = match value {
        "BOOL" | "bool" | "torch.BoolStorage" => ModelStorageDType::Tensor(DType::Bool),
        "U8" | "uint8" | "torch.ByteStorage" => ModelStorageDType::Tensor(DType::U8),
        "I8" | "int8" | "torch.CharStorage" => ModelStorageDType::Tensor(DType::I8),
        "U16" | "uint16" => ModelStorageDType::Tensor(DType::U16),
        "I16" | "int16" | "torch.ShortStorage" => ModelStorageDType::Tensor(DType::I16),
        "F16" | "float16" | "torch.HalfStorage" => ModelStorageDType::Tensor(DType::F16),
        "BF16" | "bfloat16" | "torch.BFloat16Storage" => ModelStorageDType::Tensor(DType::Bf16),
        "U32" | "uint32" => ModelStorageDType::Tensor(DType::U32),
        "I32" | "int32" | "torch.IntStorage" => ModelStorageDType::Tensor(DType::I32),
        "F32" | "float32" | "torch.FloatStorage" => ModelStorageDType::Tensor(DType::F32),
        "F8_E4M3" | "float8_e4m3fn" => ModelStorageDType::Tensor(DType::Float8E4m3Fn),
        "F8_E5M2" | "float8_e5m2" => ModelStorageDType::Tensor(DType::Float8E5m2),
        "float8_e4m3fnuz" => ModelStorageDType::Tensor(DType::Float8E4m3Fnuz),
        "float8_e5m2fnuz" => ModelStorageDType::Tensor(DType::Float8E5m2Fnuz),
        "float8_e8m0fnu" => ModelStorageDType::Tensor(DType::Float8E8m0Fnu),
        "U64" | "uint64" => ModelStorageDType::Tensor(DType::U64),
        "I64" | "int64" | "torch.LongStorage" => ModelStorageDType::Tensor(DType::I64),
        "F64" | "float64" | "torch.DoubleStorage" => ModelStorageDType::Tensor(DType::F64),
        "C64" | "complex64" | "torch.ComplexFloatStorage" => {
            ModelStorageDType::Tensor(DType::Complex64)
        }
        "complex128" | "torch.ComplexDoubleStorage" => ModelStorageDType::Tensor(DType::Complex128),
        "torch.QInt8Storage" | "torch_qint8" => ModelStorageDType::TorchQInt8,
        "torch.QUInt8Storage" | "torch_quint8" => ModelStorageDType::TorchQUInt8,
        "torch.QInt32Storage" | "torch_qint32" => ModelStorageDType::TorchQInt32,
        value => {
            if let Some(identifier) = value
                .strip_prefix("GGML_TYPE_")
                .or_else(|| value.strip_prefix("ggml_type_"))
                .and_then(|identifier| identifier.parse::<u32>().ok())
                && matches!(
                    identifier,
                    0 | 1
                        | 2
                        | 3
                        | 6
                        | 7
                        | 8
                        | 9
                        | 10
                        | 11
                        | 12
                        | 13
                        | 14
                        | 15
                        | 16
                        | 17
                        | 18
                        | 19
                        | 20
                        | 21
                        | 22
                        | 23
                        | 24
                        | 25
                        | 26
                        | 27
                        | 28
                        | 29
                        | 30
                        | 34
                        | 35
                        | 39
                        | 40
                        | 41
                        | 42
                )
            {
                ModelStorageDType::Ggml(identifier)
            } else {
                return Err(ModelFamilyError::UnknownStorageDType(value.to_owned()));
            }
        }
    };
    Ok(normalized)
}

pub(crate) fn model_weight_statistic_dtype(
    tensor_name: &str,
    value: &str,
) -> Result<DType, ModelFamilyError> {
    match normalize_storage_dtype(value)? {
        ModelStorageDType::Tensor(dtype @ (DType::F64 | DType::F32 | DType::F16 | DType::Bf16)) => {
            Ok(dtype)
        }
        ModelStorageDType::Tensor(dtype) => Err(ModelFamilyError::WeightStatisticDType {
            tensor: tensor_name.to_owned(),
            storage_dtype: dtype.catalog_name().to_owned(),
        }),
        storage_dtype => Err(ModelFamilyError::WeightStatisticDType {
            tensor: tensor_name.to_owned(),
            storage_dtype: storage_dtype.normalized_name(),
        }),
    }
}

fn normalized_storage_dtype(value: &str) -> Option<ModelStorageDType> {
    normalize_storage_dtype(value).ok()
}

fn select_unet_prefix(probe: &ModelProbe) -> Result<ModelUnetPrefixSelection, ModelFamilyError> {
    select_unet_prefix_cancellable(probe, None)
}

fn select_unet_prefix_cancellable(
    probe: &ModelProbe,
    cancellation: Option<&CancellationToken>,
) -> Result<ModelUnetPrefixSelection, ModelFamilyError> {
    if probe.tensor_shapes.len() > MAX_MODEL_PROBE_TENSORS {
        return Err(ModelFamilyError::ProbeTensorLimit {
            actual: probe.tensor_shapes.len(),
            maximum: MAX_MODEL_PROBE_TENSORS,
        });
    }
    let mut has_sam3_detector = false;
    let mut has_sam3_tracker = false;
    let mut candidate_counts = UNET_PREFIX_CANDIDATES
        .into_iter()
        .map(|prefix| (prefix.to_owned(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    for (index, key) in probe.tensor_shapes.keys().enumerate() {
        if index % 1_024 == 0
            && let Some(cancellation) = cancellation
        {
            cancellation.check()?;
        }
        has_sam3_detector |= key.starts_with("detector.");
        has_sam3_tracker |= key.starts_with("tracker.");
        for candidate in UNET_PREFIX_CANDIDATES {
            if key.starts_with(candidate) {
                let count = candidate_counts.get_mut(candidate).ok_or_else(|| {
                    ModelFamilyError::InvalidProbeConfiguration(
                        "UNet prefix candidate disappeared".to_owned(),
                    )
                })?;
                *count = count
                    .checked_add(1)
                    .ok_or(ModelFamilyError::ProbeDimensionOverflow)?;
                break;
            }
        }
    }
    if let Some(cancellation) = cancellation {
        cancellation.check()?;
    }
    let sam3_top_level = has_sam3_detector && has_sam3_tracker;
    let prefix = if sam3_top_level {
        String::new()
    } else {
        let mut selected = UNET_PREFIX_CANDIDATES[0];
        let mut selected_count = candidate_counts.get(selected).copied().unwrap_or_default();
        for candidate in UNET_PREFIX_CANDIDATES.into_iter().skip(1) {
            let candidate_count = candidate_counts.get(candidate).copied().unwrap_or_default();
            if candidate_count > selected_count {
                selected = candidate;
                selected_count = candidate_count;
            }
        }
        if selected_count > 5 {
            selected.to_owned()
        } else {
            "model.".to_owned()
        }
    };
    Ok(ModelUnetPrefixSelection {
        prefix,
        candidate_counts,
        sam3_top_level,
    })
}

fn count_consecutive_blocks(probe: &ModelProbe, pattern: &str) -> Result<usize, ModelFamilyError> {
    if pattern.is_empty() || pattern.len() > MAX_MODEL_PROBE_NAME_BYTES {
        return Err(ModelFamilyError::InvalidBlockPattern(pattern.to_owned()));
    }
    let Some((before, after)) = pattern.split_once("{}") else {
        return Err(ModelFamilyError::InvalidBlockPattern(pattern.to_owned()));
    };
    if after.contains("{}") {
        return Err(ModelFamilyError::InvalidBlockPattern(pattern.to_owned()));
    }
    let mut count = 0_usize;
    loop {
        if count > probe.tensor_shapes.len() {
            return Err(ModelFamilyError::ProbeDimensionOverflow);
        }
        let prefix = format!("{before}{count}{after}");
        if !has_tensor_prefix(probe, &prefix) {
            return Ok(count);
        }
        count = count
            .checked_add(1)
            .ok_or(ModelFamilyError::ProbeDimensionOverflow)?;
    }
}

fn has_tensor_prefix(probe: &ModelProbe, prefix: &str) -> bool {
    probe
        .tensor_shapes
        .range(prefix.to_owned()..)
        .next()
        .is_some_and(|(key, _)| key.starts_with(prefix))
}

fn validate_state_layout_signatures(
    signatures: &[ModelLayoutSignature],
) -> Result<(), ModelFamilyError> {
    if signatures.is_empty() || signatures.len() > MAX_MODEL_LAYOUT_SIGNATURES {
        return Err(ModelFamilyError::ModelLayoutSelection(format!(
            "layout selector has {} signatures; expected 1..={MAX_MODEL_LAYOUT_SIGNATURES}",
            signatures.len()
        )));
    }
    let mut layouts = BTreeSet::new();
    for signature in signatures {
        if !layouts.insert(signature.layout) {
            return Err(ModelFamilyError::ModelLayoutSelection(format!(
                "layout selector repeats {:?}",
                signature.layout
            )));
        }
        let fact_count = signature
            .required_keys
            .len()
            .checked_add(signature.required_prefixes.len())
            .ok_or(ModelFamilyError::ProbeDimensionOverflow)?;
        if fact_count == 0 || fact_count > MAX_MODEL_LAYOUT_SIGNATURE_FACTS {
            return Err(ModelFamilyError::ModelLayoutSelection(format!(
                "layout {:?} has {fact_count} key facts; expected 1..={MAX_MODEL_LAYOUT_SIGNATURE_FACTS}",
                signature.layout
            )));
        }
        let mut facts = BTreeSet::new();
        for key in signature.required_keys {
            validate_probe_tensor_name(key)?;
            if !facts.insert((false, *key)) {
                return Err(ModelFamilyError::ModelLayoutSelection(format!(
                    "layout {:?} repeats required key {key:?}",
                    signature.layout
                )));
            }
        }
        for prefix in signature.required_prefixes {
            validate_probe_tensor_name(prefix)?;
            if !facts.insert((true, *prefix)) {
                return Err(ModelFamilyError::ModelLayoutSelection(format!(
                    "layout {:?} repeats required prefix {prefix:?}",
                    signature.layout
                )));
            }
        }
    }
    Ok(())
}

fn select_state_layout(
    probe: &ModelProbe,
    signatures: &[ModelLayoutSignature],
) -> Result<ModelStateLayout, ModelFamilyError> {
    validate_state_layout_signatures(signatures)?;
    let matches = signatures
        .iter()
        .filter(|signature| {
            signature
                .required_keys
                .iter()
                .all(|key| probe.tensor_shapes.contains_key(*key))
                && signature
                    .required_prefixes
                    .iter()
                    .all(|prefix| has_tensor_prefix(probe, prefix))
        })
        .map(|signature| signature.layout)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [layout] => Ok(*layout),
        [] => Err(ModelFamilyError::ModelLayoutSelection(
            "parsed tensor keys match no supported layout signature".to_owned(),
        )),
        layouts => Err(ModelFamilyError::ModelLayoutSelection(format!(
            "parsed tensor keys ambiguously match layouts {layouts:?}"
        ))),
    }
}

fn normalize_model_configuration(
    probe: &ModelProbe,
) -> Result<ModelNormalizedConfiguration, ModelFamilyError> {
    let selection = select_unet_prefix(probe)?;
    let has_native_signature = probe
        .tensor_shapes
        .contains_key(&format!("{}input_blocks.0.0.weight", selection.prefix))
        || probe.tensor_shapes.keys().any(|key| {
            key.starts_with(&selection.prefix)
                && (key.contains("joint_blocks.0.")
                    || key.contains("double_blocks.0.")
                    || key.contains("transformer.layers.0."))
        });
    let has_diffusers_signature = probe.tensor_shapes.contains_key("conv_in.weight")
        && (probe.tensor_shapes.contains_key("conv_out.weight")
            || probe
                .tensor_shapes
                .keys()
                .any(|key| key.starts_with("down_blocks.0.") || key.starts_with("mid_block.")));
    if has_diffusers_signature && !has_native_signature {
        normalize_diffusers_configuration(probe, String::new())
    } else {
        normalize_native_configuration(probe, selection.prefix)
    }
}

fn normalize_native_configuration(
    probe: &ModelProbe,
    prefix: String,
) -> Result<ModelNormalizedConfiguration, ModelFamilyError> {
    let mut facts = BTreeMap::new();
    let input_key = format!("{prefix}input_blocks.0.0.weight");
    if !probe.tensor_shapes.contains_key(&input_key) {
        return Ok(ModelNormalizedConfiguration {
            kind: ModelConfigurationKind::Native,
            unet_prefix: prefix,
            facts,
        });
    }
    facts.insert(
        "use_checkpoint".to_owned(),
        ModelConfigurationValue::Boolean(false),
    );
    facts.insert(
        "image_size".to_owned(),
        ModelConfigurationValue::Unsigned(32),
    );
    facts.insert(
        "use_spatial_transformer".to_owned(),
        ModelConfigurationValue::Boolean(true),
    );
    facts.insert("legacy".to_owned(), ModelConfigurationValue::Boolean(false));
    let model_channels = probe_dimension(probe, &input_key, 0)?;
    let in_channels = probe_dimension(probe, &input_key, 1)?;
    facts.insert(
        "model_channels".to_owned(),
        ModelConfigurationValue::Unsigned(model_channels),
    );
    facts.insert(
        "in_channels".to_owned(),
        ModelConfigurationValue::Unsigned(in_channels),
    );
    let output_key = format!("{prefix}out.2.weight");
    let out_channels = if probe.tensor_shapes.contains_key(&output_key) {
        probe_dimension(probe, &output_key, 0)?
    } else {
        4
    };
    facts.insert(
        "out_channels".to_owned(),
        ModelConfigurationValue::Unsigned(out_channels),
    );
    let label_key = format!("{prefix}label_emb.0.0.weight");
    if probe.tensor_shapes.contains_key(&label_key) {
        facts.insert(
            "num_classes".to_owned(),
            ModelConfigurationValue::Text("sequential".to_owned()),
        );
        facts.insert(
            "adm_in_channels".to_owned(),
            ModelConfigurationValue::Unsigned(probe_dimension(probe, &label_key, 1)?),
        );
    } else {
        facts.insert("adm_in_channels".to_owned(), ModelConfigurationValue::None);
    }

    let input_blocks = count_consecutive_blocks(probe, &format!("{prefix}input_blocks.{{}}."))?;
    facts.insert(
        "input_block_count".to_owned(),
        ModelConfigurationValue::Unsigned(
            u64::try_from(input_blocks).map_err(|_| ModelFamilyError::ProbeDimensionOverflow)?,
        ),
    );
    let mut num_res_blocks = Vec::new();
    let mut channel_mult = Vec::new();
    let mut transformer_depth = Vec::new();
    let mut transformer_depth_output = Vec::new();
    let mut last_res_blocks = 0_u64;
    let mut last_channel_mult = 0_u64;
    let mut context_dimension = None;
    let mut use_linear_in_transformer = false;
    let mut video_model = false;
    let mut video_model_cross = false;
    for block in 0..input_blocks {
        let input_prefix = format!("{prefix}input_blocks.{block}.");
        let output_block = input_blocks
            .checked_sub(block)
            .and_then(|value| value.checked_sub(1))
            .ok_or(ModelFamilyError::ProbeDimensionOverflow)?;
        let output_prefix = format!("{prefix}output_blocks.{output_block}.");
        if probe
            .tensor_shapes
            .contains_key(&format!("{input_prefix}0.op.weight"))
        {
            num_res_blocks.push(last_res_blocks);
            channel_mult.push(last_channel_mult);
            last_res_blocks = 0;
            last_channel_mult = 0;
            transformer_depth_output.push(
                transformer_block_facts(probe, &output_prefix)?
                    .map(|facts| facts.depth)
                    .unwrap_or(0),
            );
            continue;
        }

        if probe
            .tensor_shapes
            .contains_key(&format!("{input_prefix}0.in_layers.0.weight"))
        {
            last_res_blocks = last_res_blocks
                .checked_add(1)
                .ok_or(ModelFamilyError::ProbeDimensionOverflow)?;
            let block_channels =
                probe_dimension(probe, &format!("{input_prefix}0.out_layers.3.weight"), 0)?;
            last_channel_mult = block_channels.checked_div(model_channels).ok_or_else(|| {
                ModelFamilyError::InvalidProbeConfiguration(
                    "standard UNet model_channels is zero".to_owned(),
                )
            })?;
            if let Some(block_facts) = transformer_block_facts(probe, &input_prefix)? {
                transformer_depth.push(block_facts.depth);
                if context_dimension.is_none() {
                    context_dimension = Some(block_facts.context_dimension);
                    use_linear_in_transformer = block_facts.use_linear_projection;
                    video_model = block_facts.time_stack;
                    video_model_cross = block_facts.time_stack_cross;
                }
            } else {
                transformer_depth.push(0);
            }
        }

        if probe
            .tensor_shapes
            .contains_key(&format!("{output_prefix}0.in_layers.0.weight"))
        {
            transformer_depth_output.push(
                transformer_block_facts(probe, &output_prefix)?
                    .map(|facts| facts.depth)
                    .unwrap_or(0),
            );
        }
    }
    num_res_blocks.push(last_res_blocks);
    channel_mult.push(last_channel_mult);
    let middle_depth = if probe
        .tensor_shapes
        .contains_key(&format!("{prefix}middle_block.1.proj_in.weight"))
    {
        i64::try_from(count_consecutive_blocks(
            probe,
            &format!("{prefix}middle_block.1.transformer_blocks.{{}}"),
        )?)
        .map_err(|_| ModelFamilyError::ProbeDimensionOverflow)?
    } else if probe
        .tensor_shapes
        .contains_key(&format!("{prefix}middle_block.0.in_layers.0.weight"))
    {
        -1
    } else {
        -2
    };
    facts.insert(
        "num_res_blocks".to_owned(),
        ModelConfigurationValue::UnsignedList(num_res_blocks),
    );
    facts.insert(
        "channel_mult".to_owned(),
        ModelConfigurationValue::UnsignedList(channel_mult),
    );
    facts.insert(
        "transformer_depth".to_owned(),
        ModelConfigurationValue::UnsignedList(transformer_depth),
    );
    facts.insert(
        "transformer_depth_output".to_owned(),
        ModelConfigurationValue::UnsignedList(transformer_depth_output),
    );
    facts.insert(
        "transformer_depth_middle".to_owned(),
        ModelConfigurationValue::Signed(middle_depth),
    );
    facts.insert(
        "use_linear_in_transformer".to_owned(),
        ModelConfigurationValue::Boolean(use_linear_in_transformer),
    );
    facts.insert(
        "context_dim".to_owned(),
        context_dimension.map_or(
            ModelConfigurationValue::None,
            ModelConfigurationValue::Unsigned,
        ),
    );
    if video_model {
        facts.insert(
            "extra_ff_mix_layer".to_owned(),
            ModelConfigurationValue::Boolean(true),
        );
        facts.insert(
            "use_spatial_context".to_owned(),
            ModelConfigurationValue::Boolean(true),
        );
        facts.insert(
            "merge_strategy".to_owned(),
            ModelConfigurationValue::Text("learned_with_images".to_owned()),
        );
        facts.insert(
            "merge_factor".to_owned(),
            ModelConfigurationValue::FloatBits(0.0_f64.to_bits()),
        );
        facts.insert(
            "video_kernel_size".to_owned(),
            ModelConfigurationValue::UnsignedList(vec![3, 1, 1]),
        );
        facts.insert(
            "use_temporal_resblock".to_owned(),
            ModelConfigurationValue::Boolean(true),
        );
        facts.insert(
            "use_temporal_attention".to_owned(),
            ModelConfigurationValue::Boolean(true),
        );
        facts.insert(
            "disable_temporal_crossattention".to_owned(),
            ModelConfigurationValue::Boolean(!video_model_cross),
        );
    } else {
        facts.insert(
            "use_temporal_resblock".to_owned(),
            ModelConfigurationValue::Boolean(false),
        );
        facts.insert(
            "use_temporal_attention".to_owned(),
            ModelConfigurationValue::Boolean(false),
        );
    }
    if probe
        .tensor_shapes
        .contains_key(&format!("{prefix}heatmap_head.conv_layers.0.weight"))
    {
        facts.insert(
            "heatmap_head".to_owned(),
            ModelConfigurationValue::Boolean(true),
        );
    }
    Ok(ModelNormalizedConfiguration {
        kind: ModelConfigurationKind::Native,
        unet_prefix: prefix,
        facts,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StandardTransformerBlockFacts {
    depth: u64,
    context_dimension: u64,
    use_linear_projection: bool,
    time_stack: bool,
    time_stack_cross: bool,
}

fn transformer_block_facts(
    probe: &ModelProbe,
    block_prefix: &str,
) -> Result<Option<StandardTransformerBlockFacts>, ModelFamilyError> {
    let transformer_prefix = format!("{block_prefix}1.transformer_blocks.");
    if !has_tensor_prefix(probe, &transformer_prefix) {
        return Ok(None);
    }
    let depth = u64::try_from(count_consecutive_blocks(
        probe,
        &format!("{transformer_prefix}{{}}"),
    )?)
    .map_err(|_| ModelFamilyError::ProbeDimensionOverflow)?;
    let context_dimension = probe_dimension(
        probe,
        &format!("{transformer_prefix}0.attn2.to_k.weight"),
        1,
    )?;
    let projection_key = format!("{block_prefix}1.proj_in.weight");
    let projection_rank = probe
        .tensor_shapes
        .get(&projection_key)
        .ok_or_else(|| {
            ModelFamilyError::InvalidProbeConfiguration(format!(
                "standard UNet transformer is missing {projection_key}"
            ))
        })?
        .len();
    let time_stack = probe
        .tensor_shapes
        .contains_key(&format!("{block_prefix}1.time_stack.0.attn1.to_q.weight"))
        || probe.tensor_shapes.contains_key(&format!(
            "{block_prefix}1.time_mix_blocks.0.attn1.to_q.weight"
        ));
    let time_stack_cross = probe
        .tensor_shapes
        .contains_key(&format!("{block_prefix}1.time_stack.0.attn2.to_q.weight"))
        || probe.tensor_shapes.contains_key(&format!(
            "{block_prefix}1.time_mix_blocks.0.attn2.to_q.weight"
        ));
    Ok(Some(StandardTransformerBlockFacts {
        depth,
        context_dimension,
        use_linear_projection: projection_rank == 2,
        time_stack,
        time_stack_cross,
    }))
}

fn normalize_diffusers_configuration(
    probe: &ModelProbe,
    prefix: String,
) -> Result<ModelNormalizedConfiguration, ModelFamilyError> {
    let mut facts = BTreeMap::new();
    add_weight_dimensions(
        &mut facts,
        probe,
        "conv_in.weight",
        "model_channels",
        "in_channels",
    )?;
    add_weight_output_dimension(&mut facts, probe, "conv_out.weight", "out_channels")?;
    let down_blocks = count_consecutive_blocks(probe, "down_blocks.{}")?;
    let mut transformer_depth = Vec::new();
    let mut residual_blocks = Vec::new();
    let mut context_dimension = None;
    for block in 0..down_blocks {
        let attention_count =
            count_consecutive_blocks(probe, &format!("down_blocks.{block}.attentions.{{}}"))?;
        let residual_count =
            count_consecutive_blocks(probe, &format!("down_blocks.{block}.resnets.{{}}"))?;
        residual_blocks.push(
            u64::try_from(residual_count).map_err(|_| ModelFamilyError::ProbeDimensionOverflow)?,
        );
        if attention_count == 0 {
            transformer_depth.extend(std::iter::repeat_n(0, residual_count));
        } else {
            for attention in 0..attention_count {
                let depth = count_consecutive_blocks(
                    probe,
                    &format!("down_blocks.{block}.attentions.{attention}.transformer_blocks.{{}}"),
                )?;
                transformer_depth.push(
                    u64::try_from(depth).map_err(|_| ModelFamilyError::ProbeDimensionOverflow)?,
                );
                if depth > 0 {
                    let key = format!(
                        "down_blocks.{block}.attentions.{attention}.transformer_blocks.0.attn2.to_k.weight"
                    );
                    context_dimension = Some(probe_dimension(probe, &key, 1)?);
                }
            }
        }
    }
    if down_blocks > 0 {
        facts.insert(
            "down_block_count".to_owned(),
            ModelConfigurationValue::Unsigned(
                u64::try_from(down_blocks).map_err(|_| ModelFamilyError::ProbeDimensionOverflow)?,
            ),
        );
        facts.insert(
            "num_res_blocks".to_owned(),
            ModelConfigurationValue::UnsignedList(residual_blocks),
        );
        facts.insert(
            "transformer_depth".to_owned(),
            ModelConfigurationValue::UnsignedList(transformer_depth),
        );
    }
    let adm_key = if probe
        .tensor_shapes
        .contains_key("class_embedding.linear_1.weight")
    {
        Some("class_embedding.linear_1.weight")
    } else if probe
        .tensor_shapes
        .contains_key("add_embedding.linear_1.weight")
    {
        Some("add_embedding.linear_1.weight")
    } else {
        None
    };
    match adm_key {
        Some(key) => {
            facts.insert(
                "adm_in_channels".to_owned(),
                ModelConfigurationValue::Unsigned(probe_dimension(probe, key, 1)?),
            );
        }
        None => {
            facts.insert("adm_in_channels".to_owned(), ModelConfigurationValue::None);
        }
    }
    if let Some(context_dimension) = context_dimension {
        facts.insert(
            "context_dim".to_owned(),
            ModelConfigurationValue::Unsigned(context_dimension),
        );
    }
    Ok(ModelNormalizedConfiguration {
        kind: ModelConfigurationKind::Diffusers,
        unet_prefix: prefix,
        facts,
    })
}

fn add_weight_dimensions(
    facts: &mut BTreeMap<String, ModelConfigurationValue>,
    probe: &ModelProbe,
    tensor: &str,
    output_name: &str,
    input_name: &str,
) -> Result<(), ModelFamilyError> {
    if probe.tensor_shapes.contains_key(tensor) {
        facts.insert(
            output_name.to_owned(),
            ModelConfigurationValue::Unsigned(probe_dimension(probe, tensor, 0)?),
        );
        facts.insert(
            input_name.to_owned(),
            ModelConfigurationValue::Unsigned(probe_dimension(probe, tensor, 1)?),
        );
    }
    Ok(())
}

fn add_weight_output_dimension(
    facts: &mut BTreeMap<String, ModelConfigurationValue>,
    probe: &ModelProbe,
    tensor: &str,
    output_name: &str,
) -> Result<(), ModelFamilyError> {
    if probe.tensor_shapes.contains_key(tensor) {
        facts.insert(
            output_name.to_owned(),
            ModelConfigurationValue::Unsigned(probe_dimension(probe, tensor, 0)?),
        );
    }
    Ok(())
}

fn probe_dimension(
    probe: &ModelProbe,
    tensor: &str,
    dimension: usize,
) -> Result<u64, ModelFamilyError> {
    probe
        .tensor_shapes
        .get(tensor)
        .and_then(|shape| shape.get(dimension))
        .copied()
        .ok_or_else(|| ModelFamilyError::ProbeDimensionOutOfBounds {
            tensor: tensor.to_owned(),
            dimension,
        })
}

fn validate_registration(registration: RegisteredModelFamily) -> Result<(), ModelFamilyError> {
    validate_definition(registration.definition)?;
    validate_identifier("source architecture", registration.source_architecture)?;
    validate_source_configuration(registration.source_configuration)?;
    validate_keys(registration.required_state_keys)?;
    if registration.profile_selector.is_none() {
        validate_profile(
            registration.definition,
            &ModelFamilyProfile::from_definition(registration.definition),
        )?;
    }
    match registration.clip_target_selector {
        ModelClipTargetSelector::Profile => {}
        ModelClipTargetSelector::Static(target) => {
            target.compile()?;
        }
        ModelClipTargetSelector::Metadata { key, cases } => {
            validate_state_key_fragment(key)?;
            if cases.is_empty() {
                return Err(ModelFamilyError::ClipTargetSelection(
                    "metadata CLIP-target selector has no cases".to_owned(),
                ));
            }
            let mut values = BTreeSet::new();
            for case in cases {
                validate_state_key_fragment(case.metadata_value)?;
                if !values.insert(case.metadata_value) {
                    return Err(ModelFamilyError::ClipTargetSelection(format!(
                        "metadata CLIP-target selector repeats value {}",
                        case.metadata_value
                    )));
                }
                case.target.compile()?;
            }
        }
    }
    match registration.state_plan_selector {
        ModelFamilyStatePlanSelector::LegacyDefinitionRules => {}
        ModelFamilyStatePlanSelector::Static(definition) => {
            let plan = definition.compile()?;
            validate_state_plan_for_definition(registration.definition, &plan)?;
        }
        ModelFamilyStatePlanSelector::Layout { signatures, cases } => {
            validate_state_layout_signatures(signatures)?;
            if cases.is_empty() {
                return Err(ModelFamilyError::StatePlanSelection(
                    "layout state-plan selector has no cases".to_owned(),
                ));
            }
            let mut layouts = BTreeSet::new();
            for case in cases {
                if !layouts.insert(case.layout) {
                    return Err(ModelFamilyError::StatePlanSelection(format!(
                        "layout state-plan selector repeats {:?}",
                        case.layout
                    )));
                }
                let plan = case.plan.compile()?;
                validate_state_plan_for_definition(registration.definition, &plan)?;
            }
            for signature in signatures {
                if !layouts.contains(&signature.layout) {
                    return Err(ModelFamilyError::StatePlanSelection(format!(
                        "layout signature {:?} has no state plan",
                        signature.layout
                    )));
                }
            }
            for layout in layouts {
                if !signatures
                    .iter()
                    .any(|signature| signature.layout == layout)
                {
                    return Err(ModelFamilyError::StatePlanSelection(format!(
                        "state plan for {layout:?} has no layout signature"
                    )));
                }
            }
        }
        ModelFamilyStatePlanSelector::Probe(_) => {
            // Probe-derived plans are checked when their immutable row selector is
            // invoked during resolution, before the plan can reach a transaction.
        }
    }
    let declared_components = registration
        .definition
        .components
        .iter()
        .map(|component| component.identifier)
        .collect::<BTreeSet<_>>();
    let mut schema_components = BTreeSet::new();
    for schema in registration.component_state_schemas {
        validate_declared_component(&declared_components, schema.component)?;
        if !schema_components.insert(schema.component) {
            return Err(ModelFamilyError::DuplicateDefinitionValue(
                schema.component.to_owned(),
            ));
        }
        let required = validate_keys(schema.required_keys)?;
        let optional = validate_keys(schema.optional_keys)?;
        if required.intersection(&optional).next().is_some() {
            return Err(ModelFamilyError::InvalidDefinition(format!(
                "component {} repeats a required key as optional",
                schema.component
            )));
        }
    }
    if !registration.implicit_registration {
        for component in registration.definition.components {
            if !schema_components.contains(component.identifier) {
                return Err(ModelFamilyError::MissingComponentSchema(
                    component.identifier.to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_source_configuration(
    rules: &[ModelSourceConfigurationRule],
) -> Result<(), ModelFamilyError> {
    let mut keys = BTreeSet::new();
    for rule in rules {
        let (key, valid) = match rule {
            ModelSourceConfigurationRule::Metadata { key, value } => {
                (*key, !key.is_empty() && !value.is_empty())
            }
            ModelSourceConfigurationRule::ExactTensorShape { key, shape } => (
                *key,
                !key.is_empty() && !shape.is_empty() && !shape.contains(&0),
            ),
        };
        if !valid {
            return Err(ModelFamilyError::InvalidDefinition(
                "invalid source configuration rule".to_owned(),
            ));
        }
        if !keys.insert(key) {
            return Err(ModelFamilyError::DuplicateDefinitionValue(key.to_owned()));
        }
    }
    Ok(())
}

fn validate_probe_registration(
    registration: RegisteredModelFamily,
    probe: &ModelProbe,
) -> Result<(), ModelFamilyError> {
    for key in registration.required_state_keys {
        if !probe.tensor_shapes.contains_key(*key) {
            return Err(ModelFamilyError::MissingRequiredStateKey((*key).to_owned()));
        }
    }
    for rule in registration.source_configuration {
        let matched = match rule {
            ModelSourceConfigurationRule::Metadata { key, value } => probe
                .metadata
                .get(*key)
                .is_some_and(|actual| actual == value),
            ModelSourceConfigurationRule::ExactTensorShape { key, shape } => probe
                .tensor_shapes
                .get(*key)
                .is_some_and(|actual| actual == shape),
        };
        if !matched {
            return Err(ModelFamilyError::SourceConfigurationMismatch {
                architecture: registration.source_architecture.to_owned(),
                rule: format!("{rule:?}"),
            });
        }
    }
    Ok(())
}

fn source_configuration_evidence(rules: &[ModelSourceConfigurationRule]) -> Vec<String> {
    rules
        .iter()
        .map(|rule| format!("source configuration {rule:?}"))
        .collect()
}

fn validate_profile(
    definition: &ModelFamilyDefinition,
    profile: &ModelFamilyProfile,
) -> Result<(), ModelFamilyError> {
    profile
        .clip_target
        .compile()
        .map_err(|error| ModelFamilyError::InvalidSelectorOutput(error.to_string()))?;
    LatentFormatIdentity::new(profile.latent_feature_id, profile.latent_identifier)
        .map_err(|error| ModelFamilyError::InvalidSelectorOutput(error.to_string()))?;
    if profile.supported_dtypes.is_empty()
        || profile.supported_devices.is_empty()
        || profile.forward_program.is_empty()
        || profile.memory_estimator.bytes_per_parameter == 0
        || profile.memory_estimator.activation_bytes_per_element == 0
    {
        return Err(ModelFamilyError::InvalidSelectorOutput(
            "selected profile contains an empty required collection or zero memory multiplier"
                .to_owned(),
        ));
    }
    let mut dtypes = BTreeSet::new();
    for dtype in profile.supported_dtypes {
        if !definition.supported_dtypes.contains(dtype) || !dtypes.insert(dtype.catalog_name()) {
            return Err(ModelFamilyError::InvalidSelectorOutput(format!(
                "selected dtype {} is duplicated or not admitted by the family",
                dtype.catalog_name()
            )));
        }
    }
    let mut devices = BTreeSet::new();
    for device in profile.supported_devices {
        if !definition.supported_devices.contains(device) || !devices.insert(format!("{device:?}"))
        {
            return Err(ModelFamilyError::InvalidSelectorOutput(format!(
                "selected device {device:?} is duplicated or not admitted by the family"
            )));
        }
    }
    validate_forward_program(definition, profile.forward_program)
        .map_err(|error| ModelFamilyError::InvalidSelectorOutput(error.to_string()))
}

fn validate_forward_program(
    definition: &ModelFamilyDefinition,
    program: &[ModelForwardStep],
) -> Result<(), ModelFamilyError> {
    let required = validate_keys(definition.required_keys)?;
    let optional = validate_keys(definition.optional_keys)?;
    let known_weights = required.union(&optional).copied().collect::<BTreeSet<_>>();
    let mut checkpoints = BTreeSet::new();
    for step in program {
        validate_identifier("checkpoint", step.checkpoint)?;
        if !checkpoints.insert(step.checkpoint) {
            return Err(ModelFamilyError::DuplicateDefinitionValue(
                step.checkpoint.to_owned(),
            ));
        }
        validate_forward_operation(step.operation, &known_weights)?;
    }
    Ok(())
}

fn validate_forward_operation(
    operation: ModelForwardOperation,
    known_weights: &BTreeSet<&str>,
) -> Result<(), ModelFamilyError> {
    let require_weight = |key: &str| {
        if known_weights.contains(key) {
            Ok(())
        } else {
            Err(ModelFamilyError::MissingProgramWeight(key.to_owned()))
        }
    };
    match operation {
        ModelForwardOperation::AddWeight(key) | ModelForwardOperation::MultiplyWeight(key) => {
            require_weight(key)
        }
        ModelForwardOperation::AddScalar(value) | ModelForwardOperation::MultiplyScalar(value) => {
            if value.is_finite() {
                Ok(())
            } else {
                Err(ModelFamilyError::InvalidDefinition(
                    "non-finite forward scalar".to_owned(),
                ))
            }
        }
        ModelForwardOperation::Linear {
            weight,
            bias,
            input_features,
            output_features,
        } => {
            require_weight(weight)?;
            if let Some(bias) = bias {
                require_weight(bias)?;
            }
            if input_features == 0 || output_features == 0 {
                return Err(ModelFamilyError::InvalidDefinition(
                    "linear dimensions must be nonzero".to_owned(),
                ));
            }
            Ok(())
        }
        ModelForwardOperation::Convolution1d {
            weight,
            bias,
            input_channels,
            output_channels,
            kernel_size,
            stride,
            dilation,
            groups,
            ..
        } => {
            require_weight(weight)?;
            if let Some(bias) = bias {
                require_weight(bias)?;
            }
            validate_convolution_dimensions(
                input_channels,
                output_channels,
                &[kernel_size],
                &[stride],
                &[dilation],
                groups,
            )
        }
        ModelForwardOperation::Convolution2d {
            weight,
            bias,
            input_channels,
            output_channels,
            kernel_size,
            stride,
            dilation,
            groups,
            ..
        } => {
            require_weight(weight)?;
            if let Some(bias) = bias {
                require_weight(bias)?;
            }
            validate_convolution_dimensions(
                input_channels,
                output_channels,
                &kernel_size,
                &stride,
                &dilation,
                groups,
            )
        }
        ModelForwardOperation::Convolution3d {
            weight,
            bias,
            input_channels,
            output_channels,
            kernel_size,
            stride,
            dilation,
            groups,
            ..
        } => {
            require_weight(weight)?;
            if let Some(bias) = bias {
                require_weight(bias)?;
            }
            validate_convolution_dimensions(
                input_channels,
                output_channels,
                &kernel_size,
                &stride,
                &dilation,
                groups,
            )
        }
        ModelForwardOperation::LayerNorm {
            normalized_shape,
            weight,
            bias,
            epsilon,
        } => {
            if let Some(weight) = weight {
                require_weight(weight)?;
            }
            if let Some(bias) = bias {
                require_weight(bias)?;
            }
            if normalized_shape.is_empty()
                || normalized_shape.contains(&0)
                || !epsilon.is_finite()
                || epsilon <= 0.0
                || (bias.is_some() && weight.is_none())
            {
                return Err(ModelFamilyError::InvalidDefinition(
                    "layer-normalization configuration is invalid".to_owned(),
                ));
            }
            Ok(())
        }
        ModelForwardOperation::SelfAttention { heads: 0 } => Err(
            ModelFamilyError::InvalidDefinition("attention heads must be nonzero".to_owned()),
        ),
        ModelForwardOperation::SelfAttention { .. }
        | ModelForwardOperation::Silu
        | ModelForwardOperation::Tanh => Ok(()),
    }
}

fn validate_convolution_dimensions(
    input_channels: usize,
    output_channels: usize,
    kernel_size: &[usize],
    stride: &[usize],
    dilation: &[usize],
    groups: usize,
) -> Result<(), ModelFamilyError> {
    if input_channels == 0
        || output_channels == 0
        || kernel_size.contains(&0)
        || stride.contains(&0)
        || dilation.contains(&0)
        || groups == 0
        || !input_channels.is_multiple_of(groups)
        || !output_channels.is_multiple_of(groups)
    {
        return Err(ModelFamilyError::InvalidDefinition(
            "convolution dimensions are invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_definition(definition: &ModelFamilyDefinition) -> Result<(), ModelFamilyError> {
    ModelFamilyIdentity::new(
        definition.feature_id,
        definition.identifier,
        definition.architecture_version,
    )?;
    LatentFormatIdentity::new(definition.latent_feature_id, definition.latent_identifier)
        .map_err(|error| ModelFamilyError::InvalidLatentIdentity(error.to_string()))?;
    definition.clip_target.compile()?;
    if definition.components.is_empty()
        || definition.detection_rules.is_empty()
        || definition.weight_rules.is_empty()
        || definition.required_keys.is_empty()
        || definition.supported_dtypes.is_empty()
        || definition.supported_devices.is_empty()
        || definition.forward_program.is_empty()
    {
        return Err(ModelFamilyError::InvalidDefinition(
            "required collection is empty".to_owned(),
        ));
    }
    if definition.memory_estimator.bytes_per_parameter == 0
        || definition.memory_estimator.activation_bytes_per_element == 0
    {
        return Err(ModelFamilyError::InvalidDefinition(
            "memory estimator has a zero multiplier".to_owned(),
        ));
    }
    let mut component_ids = BTreeSet::new();
    for component in definition.components {
        validate_identifier("component identifier", component.identifier)?;
        validate_identifier("component role", component.role)?;
        if !component_ids.insert(component.identifier) {
            return Err(ModelFamilyError::DuplicateDefinitionValue(
                component.identifier.to_owned(),
            ));
        }
    }
    let mut prefixes = BTreeSet::new();
    for rule in definition.weight_rules {
        if rule.source_prefix.is_empty() || rule.target_prefix.is_empty() {
            return Err(ModelFamilyError::InvalidDefinition(
                "empty weight prefix".to_owned(),
            ));
        }
        if !prefixes.insert(rule.source_prefix) {
            return Err(ModelFamilyError::DuplicateDefinitionValue(
                rule.source_prefix.to_owned(),
            ));
        }
    }
    let required = validate_keys(definition.required_keys)?;
    let optional = validate_keys(definition.optional_keys)?;
    if let Some(key) = required.intersection(&optional).next() {
        return Err(ModelFamilyError::DuplicateDefinitionValue(
            (*key).to_owned(),
        ));
    }
    let mut dtypes = BTreeSet::new();
    for dtype in definition.supported_dtypes {
        if !dtypes.insert(dtype.catalog_name()) {
            return Err(ModelFamilyError::DuplicateDefinitionValue(
                dtype.catalog_name().to_owned(),
            ));
        }
    }
    let mut devices = BTreeSet::new();
    for device in definition.supported_devices {
        if !devices.insert(format!("{device:?}")) {
            return Err(ModelFamilyError::DuplicateDefinitionValue(format!(
                "{device:?}"
            )));
        }
    }
    validate_forward_program(definition, definition.forward_program)?;
    validate_detection_rules(definition.detection_rules)?;
    Ok(())
}

fn validate_detection_rules(rules: &[ModelDetectionRule]) -> Result<(), ModelFamilyError> {
    if rules.is_empty() {
        return Err(ModelFamilyError::InvalidDefinition(
            "detection rules are empty".to_owned(),
        ));
    }
    for rule in rules {
        if rule.score() == 0 {
            return Err(ModelFamilyError::InvalidDefinition(
                "zero detection score".to_owned(),
            ));
        }
        match rule {
            ModelDetectionRule::ExactShape { key, shape, .. }
                if key.is_empty() || shape.is_empty() || shape.contains(&0) =>
            {
                return Err(ModelFamilyError::InvalidDefinition(
                    "invalid exact-shape detector".to_owned(),
                ));
            }
            ModelDetectionRule::KeyPresent { key: "", .. } => {
                return Err(ModelFamilyError::InvalidDefinition(
                    "empty key detector".to_owned(),
                ));
            }
            ModelDetectionRule::AnyKeyPresent { keys, .. } => {
                if keys.is_empty() || keys.len() > MAX_MODEL_DETECTION_KEY_ALTERNATIVES {
                    return Err(ModelFamilyError::InvalidDefinition(format!(
                        "any-key detector has {} alternatives; expected 1..={MAX_MODEL_DETECTION_KEY_ALTERNATIVES}",
                        keys.len()
                    )));
                }
                validate_keys(keys)?;
            }
            ModelDetectionRule::AnyTensorDimensionValue {
                keys,
                dimension,
                values,
                ..
            } => {
                if keys.is_empty() || keys.len() > MAX_MODEL_DETECTION_KEY_ALTERNATIVES {
                    return Err(ModelFamilyError::InvalidDefinition(format!(
                        "tensor-dimension detector has {} keys; expected 1..={MAX_MODEL_DETECTION_KEY_ALTERNATIVES}",
                        keys.len()
                    )));
                }
                validate_keys(keys)?;
                if *dimension >= MAX_TENSOR_RANK {
                    return Err(ModelFamilyError::InvalidDefinition(format!(
                        "tensor-dimension detector dimension {dimension} exceeds maximum rank {MAX_TENSOR_RANK}"
                    )));
                }
                if values.is_empty()
                    || values.len() > MAX_MODEL_DETECTION_DIMENSION_VALUES
                    || values.contains(&0)
                {
                    return Err(ModelFamilyError::InvalidDefinition(format!(
                        "tensor-dimension detector has {} values; expected 1..={MAX_MODEL_DETECTION_DIMENSION_VALUES} nonzero values",
                        values.len()
                    )));
                }
                let unique_values = values.iter().copied().collect::<BTreeSet<_>>();
                if unique_values.len() != values.len() {
                    return Err(ModelFamilyError::DuplicateDefinitionValue(
                        "tensor-dimension detector value".to_owned(),
                    ));
                }
            }
            ModelDetectionRule::AnyTensorFact {
                keys, predicates, ..
            } => {
                if keys.is_empty() || keys.len() > MAX_MODEL_DETECTION_KEY_ALTERNATIVES {
                    return Err(ModelFamilyError::InvalidDefinition(format!(
                        "tensor-fact detector has {} keys; expected 1..={MAX_MODEL_DETECTION_KEY_ALTERNATIVES}",
                        keys.len()
                    )));
                }
                validate_keys(keys)?;
                validate_tensor_fact_predicates(predicates)?;
            }
            ModelDetectionRule::KeyPrefix {
                prefix,
                minimum_matches,
                ..
            } if prefix.is_empty() || *minimum_matches == 0 => {
                return Err(ModelFamilyError::InvalidDefinition(
                    "invalid prefix detector".to_owned(),
                ));
            }
            ModelDetectionRule::Metadata { key, value, .. }
                if key.is_empty() || value.is_empty() =>
            {
                return Err(ModelFamilyError::InvalidDefinition(
                    "invalid metadata detector".to_owned(),
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_tensor_fact_predicates(
    predicates: &[ModelTensorFactPredicate],
) -> Result<(), ModelFamilyError> {
    if predicates.is_empty() || predicates.len() > MAX_MODEL_DETECTION_TENSOR_PREDICATES {
        return Err(ModelFamilyError::InvalidDefinition(format!(
            "tensor-fact detector has {} predicates; expected 1..={MAX_MODEL_DETECTION_TENSOR_PREDICATES}",
            predicates.len()
        )));
    }
    for (index, predicate) in predicates.iter().enumerate() {
        match predicate.subject {
            ModelTensorFactSubject::Rank if predicate.value > MAX_TENSOR_RANK as u64 => {
                return Err(ModelFamilyError::InvalidDefinition(format!(
                    "tensor-fact rank {} exceeds maximum rank {MAX_TENSOR_RANK}",
                    predicate.value
                )));
            }
            ModelTensorFactSubject::Dimension(dimension) if dimension >= MAX_TENSOR_RANK => {
                return Err(ModelFamilyError::InvalidDefinition(format!(
                    "tensor-fact dimension {dimension} exceeds maximum rank {MAX_TENSOR_RANK}"
                )));
            }
            _ => {}
        }
        if predicates[..index].contains(predicate) {
            return Err(ModelFamilyError::DuplicateDefinitionValue(
                "tensor-fact detector predicate".to_owned(),
            ));
        }
    }
    Ok(())
}

fn tensor_fact_matches(shape: &[u64], predicates: &[ModelTensorFactPredicate]) -> bool {
    predicates.iter().all(|predicate| {
        let actual = match predicate.subject {
            ModelTensorFactSubject::Rank => shape.len() as u64,
            ModelTensorFactSubject::Dimension(dimension) => {
                let Some(value) = shape.get(dimension) else {
                    return false;
                };
                *value
            }
        };
        match predicate.relation {
            ModelTensorFactRelation::Equal => actual == predicate.value,
            ModelTensorFactRelation::NotEqual => actual != predicate.value,
            ModelTensorFactRelation::LessThan => actual < predicate.value,
            ModelTensorFactRelation::LessThanOrEqual => actual <= predicate.value,
            ModelTensorFactRelation::GreaterThan => actual > predicate.value,
            ModelTensorFactRelation::GreaterThanOrEqual => actual >= predicate.value,
        }
    })
}

fn validate_keys<'a>(keys: &'a [&'a str]) -> Result<BTreeSet<&'a str>, ModelFamilyError> {
    let mut values = BTreeSet::new();
    for key in keys {
        if key.is_empty() || key.len() > MAX_IDENTITY_BYTES || key.chars().any(char::is_control) {
            return Err(ModelFamilyError::InvalidDefinition(
                "invalid model key".to_owned(),
            ));
        }
        if !values.insert(*key) {
            return Err(ModelFamilyError::DuplicateDefinitionValue(
                (*key).to_owned(),
            ));
        }
    }
    Ok(values)
}

fn identity_for(
    definition: &ModelFamilyDefinition,
) -> Result<ModelFamilyIdentity, ModelFamilyError> {
    ModelFamilyIdentity::new(
        definition.feature_id,
        definition.identifier,
        definition.architecture_version,
    )
}

fn validate_feature_id(value: &str) -> Result<(), ModelFamilyError> {
    let suffix = value
        .strip_prefix("COMFY-MODEL-")
        .ok_or_else(|| ModelFamilyError::InvalidFeatureId(value.to_owned()))?;
    if suffix.len() != 4 || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ModelFamilyError::InvalidFeatureId(value.to_owned()));
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ModelFamilyError> {
    if value.is_empty() || value.len() > MAX_IDENTITY_BYTES || value.chars().any(char::is_control) {
        return Err(ModelFamilyError::InvalidIdentity {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_qualified_model_symbol(
    kind: &'static str,
    value: &str,
) -> Result<(), ModelFamilyError> {
    if value.is_empty()
        || value.len() > MAX_IDENTITY_BYTES
        || value.trim() != value
        || value.chars().any(char::is_control)
        || value.split('.').any(|segment| {
            let mut characters = segment.chars();
            characters
                .next()
                .is_none_or(|first| !(first.is_ascii_alphabetic() || first == '_'))
                || characters
                    .any(|character| !(character.is_ascii_alphanumeric() || character == '_'))
        })
    {
        return Err(ModelFamilyError::InvalidClipTarget(format!(
            "invalid {kind} identifier {value:?}"
        )));
    }
    Ok(())
}

pub(crate) fn validate_digest(value: &str) -> Result<(), ModelFamilyError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ModelFamilyError::InvalidArtifactDigest(value.to_owned()));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ModelFamilyError {
    #[error("unsupported model-family schema version {0}")]
    SchemaVersion(u16),
    #[error("invalid model-family feature id: {0}")]
    InvalidFeatureId(String),
    #[error("invalid model-family {field}: {value}")]
    InvalidIdentity { field: &'static str, value: String },
    #[error("invalid model-family definition: {0}")]
    InvalidDefinition(String),
    #[error("model-family definition repeats {0}")]
    DuplicateDefinitionValue(String),
    #[error("model-family registry repeats feature id {0}")]
    DuplicateFeatureId(String),
    #[error("model-family registry repeats identifier {0}")]
    DuplicateIdentifier(String),
    #[error("model-family registry repeats source ordinal {0}")]
    DuplicateSourceOrdinal(u16),
    #[error("model-family source ordinal index {0} does not fit the registry format")]
    SourceOrdinalOverflow(usize),
    #[error("model-family detection matched no family")]
    NoDetectionMatch,
    #[error("model-family detection tied at score {score}: {families:?}")]
    AmbiguousDetection { score: u32, families: Vec<String> },
    #[error("model-family detection score overflowed")]
    DetectionScoreOverflow,
    #[error("model-family probe has {actual} tensors, exceeding {maximum}")]
    ProbeTensorLimit { actual: usize, maximum: usize },
    #[error("model-family probe has {actual} formats, exceeding {maximum}")]
    ProbeFormatLimit { actual: usize, maximum: usize },
    #[error("invalid model-family probe tensor name {0:?}")]
    InvalidProbeTensorName(String),
    #[error("invalid model-family probe shape for {tensor}: {shape:?}")]
    InvalidProbeShape { tensor: String, shape: Vec<u64> },
    #[error("model-family probe dimension arithmetic overflowed")]
    ProbeDimensionOverflow,
    #[error("model-family probe tensor {tensor} has no dimension {dimension}")]
    ProbeDimensionOutOfBounds { tensor: String, dimension: usize },
    #[error("unsupported model-family probe storage dtype {0}")]
    UnknownStorageDType(String),
    #[error("model weight statistic tensor {tensor} has unsupported storage dtype {storage_dtype}")]
    WeightStatisticDType {
        tensor: String,
        storage_dtype: String,
    },
    #[error("model weight statistic request repeats tensor {0}")]
    DuplicateWeightStatisticRequest(String),
    #[error("model weight statistic request has {actual} tensors, exceeding {maximum}")]
    WeightStatisticRequestLimit { actual: usize, maximum: usize },
    #[error("model weight statistic result for {0} is not a real scalar")]
    InvalidWeightStatisticResult(String),
    #[error("model weight statistic result for {tensor} is non-finite ({value_bits:#018x})")]
    NonFiniteWeightStatistic { tensor: String, value_bits: u64 },
    #[error("model weight statistic threshold is non-finite ({0:#018x})")]
    NonFiniteWeightStatisticThreshold(u64),
    #[error("unsupported model-family probe format {0}")]
    UnsupportedProbeFormat(String),
    #[error("invalid model-family probe metadata key {0:?}")]
    InvalidProbeMetadata(String),
    #[error("conflicting model-family probe metadata for key {0}")]
    ConflictingProbeMetadata(String),
    #[error("model-family probe metadata arithmetic overflowed")]
    ProbeMetadataOverflow,
    #[error("model-family probe metadata uses {actual} bytes, exceeding {maximum}")]
    ProbeMetadataLimit { actual: usize, maximum: usize },
    #[error("invalid consecutive model block pattern {0:?}")]
    InvalidBlockPattern(String),
    #[error("invalid model-family probe configuration: {0}")]
    InvalidProbeConfiguration(String),
    #[error("source configuration for architecture {architecture} did not match {rule}")]
    SourceConfigurationMismatch { architecture: String, rule: String },
    #[error("model-family probe is missing required state key {0}")]
    MissingRequiredStateKey(String),
    #[error("model-family selector returned an invalid profile: {0}")]
    InvalidSelectorOutput(String),
    #[error("model-family layout selection failed: {0}")]
    ModelLayoutSelection(String),
    #[error("model-family state-plan selection failed: {0}")]
    StatePlanSelection(String),
    #[error("model-family CLIP-target selection failed: {0}")]
    ClipTargetSelection(String),
    #[error("invalid model-family CLIP target: {0}")]
    InvalidClipTarget(String),
    #[error("unsupported model-family state-plan schema version {0}")]
    StatePlanSchemaVersion(u16),
    #[error("model-family state-plan definition has {0} bytes, exceeding its bound")]
    StatePlanDefinitionTooLarge(usize),
    #[error("model-family state-plan definition is invalid: {0}")]
    StatePlanDefinition(String),
    #[error("model-family source tensor facts drifted after resolution: {0}")]
    ResolvedProbeDrift(String),
    #[error("model-family mapped-weight binding mismatch: {0}")]
    WeightBindingMismatch(String),
    #[error("model-family state plan targets undeclared component {0}")]
    UndeclaredComponent(String),
    #[error("model-family state mapping is missing required component {0}")]
    MissingRequiredComponent(String),
    #[error("model-family registration is missing schema for declared component {0}")]
    MissingComponentSchema(String),
    #[error("model-family component {component} is missing required key {key}")]
    MissingComponentKey { component: String, key: String },
    #[error("model-family component {component} contains unexpected key {key}")]
    UnexpectedComponentKey { component: String, key: String },
    #[error("model-family latent identity is invalid: {0}")]
    InvalidLatentIdentity(String),
    #[error("invalid base artifact digest: {0}")]
    InvalidArtifactDigest(String),
    #[error("model weight mapping produced duplicate key {0}")]
    DuplicateMappedKey(String),
    #[error("model weight mapping is missing required key {0}")]
    MissingRequiredKey(String),
    #[error("model weight mapping is missing required source prefix {0}")]
    MissingRequiredPrefix(String),
    #[error("invalid model state key: {0}")]
    InvalidStateKey(String),
    #[error("model state key selector bounds are invalid: minimum {minimum}, maximum {maximum}")]
    InvalidKeySelectorBounds { minimum: usize, maximum: usize },
    #[error(
        "model state key selector {predicate} expected {minimum}..={maximum} matches, got {actual}"
    )]
    KeySelectorCardinality {
        predicate: String,
        minimum: usize,
        maximum: usize,
        actual: usize,
    },
    #[error("model state rewrite {rewrite} does not match source key {key}")]
    KeyRewriteMismatch { key: String, rewrite: String },
    #[error("model state transform plan has {0} operations, exceeding its bound")]
    StatePlanTooLarge(usize),
    #[error("model state dictionary has {0} source tensors, exceeding its bound")]
    StateSourceTooLarge(usize),
    #[error("invalid model state transform: {0}")]
    InvalidStateTransform(String),
    #[error("model state transform selects source key {0} more than once")]
    OverlappingStateSelection(String),
    #[error("model state transform is missing source key {0}")]
    MissingTransformSource(String),
    #[error("model state transform references unavailable staged component {component} key {key}")]
    StagedOutputUnavailable { component: String, key: String },
    #[error("model state transform repeats component {component} key {key}")]
    DuplicateComponentKey { component: String, key: String },
    #[error("model dimension expression nesting exceeds its bound")]
    DimensionExpressionTooDeep,
    #[error("model dimension expression arithmetic overflowed")]
    DimensionExpressionOverflow,
    #[error("current selected tensor dimension is unavailable in this expression context")]
    CurrentTensorDimensionUnavailable,
    #[error("current selected tensor has no dimension {0}")]
    CurrentTensorDimensionOutOfBounds(usize),
    #[error("model dimension expression divides by zero")]
    DimensionDivisionByZero,
    #[error(
        "model dimension expression division {numerator}/{denominator} has a nonzero remainder"
    )]
    DimensionDivisionRemainder { numerator: u64, denominator: u64 },
    #[error("model state key {key} has no dimension {dimension}")]
    DimensionOutOfBounds { key: String, dimension: usize },
    #[error("model state assembly for {key} covers {actual} elements, expected {expected}")]
    IncompleteAssembly {
        key: String,
        expected: u64,
        actual: u64,
    },
    #[error("model state assembly input {0} has an incompatible shape, dtype, or device")]
    AssemblyShapeMismatch(String),
    #[error(
        "model state reshape for {key} changes element count from {source_elements} to {target_elements}"
    )]
    ReshapeElementCount {
        key: String,
        source_elements: u64,
        target_elements: u64,
    },
    #[error("model forward program is missing weight {0}")]
    MissingProgramWeight(String),
    #[error("model contains unexpected keys: {0:?}")]
    UnexpectedKeys(Vec<String>),
    #[error("model family does not support dtype {0:?}")]
    UnsupportedDType(DType),
    #[error("model family does not support device {0:?}")]
    UnsupportedDevice(DeviceKind),
    #[error("native model-family backend is unavailable for {0:?}")]
    BackendUnavailable(DeviceKind),
    #[error("model input dtype mismatch: expected {expected:?}, got {actual:?}")]
    DTypeMismatch { expected: DType, actual: DType },
    #[error("model input device mismatch: expected {expected:?}, got {actual:?}")]
    DeviceMismatch {
        expected: DeviceKind,
        actual: DeviceKind,
    },
    #[error("model weight {key} dtype mismatch: expected {expected:?}, got {actual:?}")]
    WeightDType {
        key: String,
        expected: DType,
        actual: DType,
    },
    #[error("model weight {key} device mismatch: expected {expected:?}, got {actual:?}")]
    WeightDevice {
        key: String,
        expected: DeviceKind,
        actual: DeviceKind,
    },
    #[error("model memory arithmetic overflowed")]
    MemoryOverflow,
    #[error("model requires {required} bytes but budget is {budget} bytes")]
    OutOfMemory { required: u64, budget: u64 },
    #[error("model forward shape arithmetic overflowed")]
    ForwardShapeOverflow,
    #[error(transparent)]
    Descriptor(#[from] ModelDescriptorError),
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    TensorOperation(#[from] OperatorIndirectionError),
    #[error(transparent)]
    ReductionOperation(#[from] ReductionPartOneError),
    #[error(transparent)]
    ElementwiseOperation(#[from] ElementwiseRuntimePartSixteenError),
    #[error(transparent)]
    ResizeOperation(#[from] ExternalTensorKernelPartOneError),
    #[error(transparent)]
    RandomOperation(#[from] RandomNumberGenerationPartOneError),
    #[error(transparent)]
    RoundOperation(#[from] ElementwiseRuntimePartSixError),
    #[error(transparent)]
    ConcatenateOperation(#[from] ElementwiseRuntimePartEightError),
    #[error(transparent)]
    SplitOperation(#[from] ElementwiseRuntimePartSeventeenError),
    #[error(transparent)]
    ReshapeOperation(#[from] ShapeLayoutTransformPartTwoError),
    #[error(transparent)]
    TransposeOperation(#[from] ShapeLayoutTransformPartThreeError),
    #[error(transparent)]
    TensorCreationOperation(#[from] TensorCreationPartOneError),
    #[error(transparent)]
    IndexingOperation(#[from] IndexingMaskingPartOneError),
    #[error(transparent)]
    ShapeOperation(#[from] ShapeLayoutTransformPartOneError),
    #[error(transparent)]
    StorageOperation(#[from] StorageDTypeDeviceError),
    #[error(transparent)]
    NativeOperation(#[from] NativeOpsError),
    #[error(transparent)]
    AttentionOperation(#[from] NeuralNetworkModulePartTwoError),
    #[error(transparent)]
    Cancelled(#[from] comfy_types::CancellationError),
}

#[cfg(test)]
mod model_probe_tests {
    use super::*;

    fn tensor(shape: &[u64], storage_dtype: &str) -> ModelParsedTensorFact {
        ModelParsedTensorFact {
            shape: shape.to_vec(),
            storage_dtype: storage_dtype.to_owned(),
        }
    }

    #[test]
    fn parsed_probe_preserves_format_order_and_normalizes_storage_dtypes() {
        let probe = ModelProbe::from_parsed_facts(ModelParsedFacts {
            tensors: BTreeMap::from([
                ("scalar".to_owned(), tensor(&[], "torch.FloatStorage")),
                ("empty".to_owned(), tensor(&[0, 4], "GGML_TYPE_2")),
            ]),
            formats: vec![
                ModelParsedFormatFact {
                    identity: "json_config".to_owned(),
                    metadata: BTreeMap::new(),
                },
                ModelParsedFormatFact {
                    identity: "safetensors".to_owned(),
                    metadata: BTreeMap::new(),
                },
                ModelParsedFormatFact {
                    identity: "safetensors".to_owned(),
                    metadata: BTreeMap::new(),
                },
            ],
        })
        .expect("valid parsed facts should produce a probe");

        assert_eq!(
            probe.format_identities(),
            vec!["json_config", "safetensors", "safetensors"]
        );
        assert_eq!(
            probe.storage_dtype("scalar"),
            Some(ModelStorageDType::Tensor(DType::F32))
        );
        assert_eq!(
            probe.storage_dtype("empty"),
            Some(ModelStorageDType::Ggml(2))
        );
    }

    #[test]
    fn parsed_probe_rejects_unknown_storage_dtype_and_dimension_overflow() {
        assert!(matches!(
            ModelProbe::from_parsed_facts(ModelParsedFacts {
                tensors: BTreeMap::from([("weight".to_owned(), tensor(&[1], "unknown-dtype"))]),
                formats: Vec::new(),
            }),
            Err(ModelFamilyError::UnknownStorageDType(_))
        ));
        assert!(matches!(
            ModelProbe::from_parsed_facts(ModelParsedFacts {
                tensors: BTreeMap::from([("weight".to_owned(), tensor(&[u64::MAX, 2], "F32"))]),
                formats: Vec::new(),
            }),
            Err(ModelFamilyError::ProbeDimensionOverflow)
        ));
    }

    #[test]
    fn unet_prefix_selection_matches_source_order_threshold_and_sam3() {
        let mut tie_shapes = BTreeMap::new();
        for index in 0..6 {
            tie_shapes.insert(format!("model.diffusion_model.layer.{index}"), vec![1]);
            tie_shapes.insert(format!("model.model.layer.{index}"), vec![1]);
        }
        let tie = ModelProbe {
            tensor_shapes: tie_shapes,
            metadata: BTreeMap::new(),
        }
        .unet_prefix_selection()
        .expect("prefix selection should succeed");
        assert_eq!(tie.prefix(), "model.diffusion_model.");

        let fallback = ModelProbe {
            tensor_shapes: BTreeMap::from([("net.only".to_owned(), vec![1])]),
            metadata: BTreeMap::new(),
        }
        .unet_prefix_selection()
        .expect("prefix selection should succeed");
        assert_eq!(fallback.prefix(), "model.");

        let sam3 = ModelProbe {
            tensor_shapes: BTreeMap::from([
                ("detector.weight".to_owned(), vec![1]),
                ("tracker.weight".to_owned(), vec![1]),
            ]),
            metadata: BTreeMap::new(),
        }
        .unet_prefix_selection()
        .expect("prefix selection should succeed");
        assert_eq!(sam3.prefix(), "");
        assert!(sam3.is_sam3_top_level());
    }

    #[test]
    fn consecutive_blocks_stop_at_the_first_gap() {
        let probe = ModelProbe {
            tensor_shapes: BTreeMap::from([
                ("blocks.0.weight".to_owned(), vec![1]),
                ("blocks.1.bias".to_owned(), vec![1]),
                ("blocks.3.weight".to_owned(), vec![1]),
            ]),
            metadata: BTreeMap::new(),
        };
        assert_eq!(
            probe
                .consecutive_block_count("blocks.{}.")
                .expect("valid block pattern should be counted"),
            2
        );
        assert!(matches!(
            probe.consecutive_block_count("blocks"),
            Err(ModelFamilyError::InvalidBlockPattern(_))
        ));
    }

    #[test]
    fn diffusers_configuration_is_unprefixed_and_uses_last_attention_context() {
        let probe = ModelProbe {
            tensor_shapes: BTreeMap::from([
                ("conv_in.weight".to_owned(), vec![320, 4, 3, 3]),
                ("conv_out.weight".to_owned(), vec![4, 320, 3, 3]),
                (
                    "down_blocks.0.resnets.0.conv1.weight".to_owned(),
                    vec![320, 320],
                ),
                (
                    "down_blocks.0.attentions.0.transformer_blocks.0.attn2.to_k.weight".to_owned(),
                    vec![320, 768],
                ),
                (
                    "down_blocks.0.attentions.1.transformer_blocks.0.attn2.to_k.weight".to_owned(),
                    vec![320, 1024],
                ),
            ]),
            metadata: BTreeMap::new(),
        };
        let configuration = probe
            .normalized_configuration()
            .expect("diffusers configuration should normalize");
        assert_eq!(configuration.kind(), ModelConfigurationKind::Diffusers);
        assert_eq!(configuration.unet_prefix(), "");
        assert_eq!(
            configuration.fact("context_dim"),
            Some(&ModelConfigurationValue::Unsigned(1024))
        );
    }

    #[test]
    fn native_signature_wins_over_an_unrelated_diffusers_key() {
        let mut tensor_shapes = BTreeMap::from([
            (
                "model.diffusion_model.input_blocks.0.0.weight".to_owned(),
                vec![320, 4, 3, 3],
            ),
            ("conv_in.weight".to_owned(), vec![1, 1]),
            ("conv_out.weight".to_owned(), vec![1, 1]),
        ]);
        for index in 0..6 {
            tensor_shapes.insert(
                format!("model.diffusion_model.input_blocks.{index}.weight"),
                vec![1],
            );
        }
        let configuration = ModelProbe {
            tensor_shapes,
            metadata: BTreeMap::new(),
        }
        .normalized_configuration()
        .expect("native configuration should normalize");
        assert_eq!(configuration.kind(), ModelConfigurationKind::Native);
        assert_eq!(configuration.unet_prefix(), "model.diffusion_model.");
    }

    #[test]
    fn standard_native_unet_normalization_matches_common_source_traversal() {
        let prefix = "model.diffusion_model.";
        let mut tensor_shapes = BTreeMap::from([
            (
                format!("{prefix}input_blocks.0.0.weight"),
                vec![320, 9, 3, 3],
            ),
            (format!("{prefix}label_emb.0.0.weight"), vec![1280, 2816]),
            (
                format!("{prefix}input_blocks.1.0.in_layers.0.weight"),
                vec![320],
            ),
            (
                format!("{prefix}input_blocks.1.0.out_layers.3.weight"),
                vec![641],
            ),
            (
                format!("{prefix}input_blocks.1.1.transformer_blocks.0.attn2.to_k.weight"),
                vec![320, 2048],
            ),
            (
                format!("{prefix}input_blocks.1.1.transformer_blocks.1.attn1.to_q.weight"),
                vec![320, 320],
            ),
            (
                format!("{prefix}input_blocks.1.1.proj_in.weight"),
                vec![320, 320],
            ),
            (
                format!("{prefix}input_blocks.1.1.time_stack.0.attn1.to_q.weight"),
                vec![320, 320],
            ),
            (
                format!("{prefix}input_blocks.2.0.op.weight"),
                vec![640, 320],
            ),
            (
                format!("{prefix}input_blocks.3.0.in_layers.0.weight"),
                vec![640],
            ),
            (
                format!("{prefix}input_blocks.3.0.out_layers.3.weight"),
                vec![1280],
            ),
            (
                format!("{prefix}output_blocks.3.0.in_layers.0.weight"),
                vec![320],
            ),
            (
                format!("{prefix}output_blocks.3.1.transformer_blocks.0.attn2.to_k.weight"),
                vec![320, 2048],
            ),
            (
                format!("{prefix}output_blocks.3.1.proj_in.weight"),
                vec![320, 320, 1, 1],
            ),
            (
                format!("{prefix}output_blocks.2.0.in_layers.0.weight"),
                vec![320],
            ),
            (
                format!("{prefix}output_blocks.1.0.in_layers.0.weight"),
                vec![640],
            ),
            (
                format!("{prefix}output_blocks.1.1.transformer_blocks.0.attn2.to_k.weight"),
                vec![640, 2048],
            ),
            (
                format!("{prefix}output_blocks.1.1.transformer_blocks.1.attn1.to_q.weight"),
                vec![640, 640],
            ),
            (
                format!("{prefix}output_blocks.1.1.proj_in.weight"),
                vec![640, 640],
            ),
            (
                format!("{prefix}output_blocks.0.0.in_layers.0.weight"),
                vec![1280],
            ),
            (
                format!("{prefix}middle_block.1.proj_in.weight"),
                vec![1280, 1280],
            ),
            (
                format!("{prefix}middle_block.1.transformer_blocks.0.attn2.to_k.weight"),
                vec![1280, 2048],
            ),
            (
                format!("{prefix}middle_block.1.transformer_blocks.1.attn1.to_q.weight"),
                vec![1280, 1280],
            ),
            (
                format!("{prefix}heatmap_head.conv_layers.0.weight"),
                vec![1],
            ),
        ]);
        for index in 0..6 {
            tensor_shapes.insert(format!("{prefix}prefix_evidence.{index}"), vec![1]);
        }
        let configuration = ModelProbe {
            tensor_shapes,
            metadata: BTreeMap::new(),
        }
        .normalized_configuration()
        .expect("standard native UNet configuration should normalize");

        assert_eq!(
            configuration.fact("out_channels"),
            Some(&ModelConfigurationValue::Unsigned(4))
        );
        assert_eq!(
            configuration.fact("num_classes"),
            Some(&ModelConfigurationValue::Text("sequential".to_owned()))
        );
        assert_eq!(
            configuration.fact("adm_in_channels"),
            Some(&ModelConfigurationValue::Unsigned(2816))
        );
        assert_eq!(
            configuration.fact("num_res_blocks"),
            Some(&ModelConfigurationValue::UnsignedList(vec![1, 1]))
        );
        assert_eq!(
            configuration.fact("channel_mult"),
            Some(&ModelConfigurationValue::UnsignedList(vec![2, 4]))
        );
        assert_eq!(
            configuration.fact("transformer_depth"),
            Some(&ModelConfigurationValue::UnsignedList(vec![2, 0]))
        );
        assert_eq!(
            configuration.fact("transformer_depth_output"),
            Some(&ModelConfigurationValue::UnsignedList(vec![1, 0, 2, 0]))
        );
        assert_eq!(
            configuration.fact("transformer_depth_middle"),
            Some(&ModelConfigurationValue::Signed(2))
        );
        assert_eq!(
            configuration.fact("context_dim"),
            Some(&ModelConfigurationValue::Unsigned(2048))
        );
        assert_eq!(
            configuration.fact("use_linear_in_transformer"),
            Some(&ModelConfigurationValue::Boolean(true))
        );
        assert_eq!(
            configuration.fact("use_temporal_attention"),
            Some(&ModelConfigurationValue::Boolean(true))
        );
        assert_eq!(
            configuration.fact("disable_temporal_crossattention"),
            Some(&ModelConfigurationValue::Boolean(true))
        );
        assert_eq!(
            configuration.fact("heatmap_head"),
            Some(&ModelConfigurationValue::Boolean(true))
        );
    }

    #[test]
    fn parsed_probe_observes_deterministic_mid_admission_cancellation() {
        let tensors = (0..2_048)
            .map(|index| (format!("weight.{index:04}"), tensor(&[1], "F32")))
            .collect();
        let cancellation = CancellationToken::default();
        assert!(matches!(
            ModelProbe::from_parsed_facts_with_checkpoint(
                ModelParsedFacts {
                    tensors,
                    formats: Vec::new(),
                },
                &cancellation,
                Some(1_024),
            ),
            Err(ModelFamilyError::Cancelled(_))
        ));
        assert!(cancellation.is_cancelled());
    }

    #[test]
    fn registry_fallback_is_opt_in_and_direct_probe_bounds_are_enforced() {
        let registry = ModelFamilyRegistry::checked(&[])
            .expect("an empty synthetic registry should be structurally valid");
        let probe = ModelProbe {
            tensor_shapes: BTreeMap::from([
                ("conv_in.weight".to_owned(), vec![320, 4, 3, 3]),
                ("conv_out.weight".to_owned(), vec![4, 320, 3, 3]),
            ]),
            metadata: BTreeMap::new(),
        };
        assert!(matches!(
            registry.detect_with_policy(&probe, ModelDetectionPolicy::RegisteredOnly),
            Err(ModelFamilyError::NoDetectionMatch)
        ));
        assert!(matches!(
            registry.detect_with_policy(&probe, ModelDetectionPolicy::AllowBaseFallback),
            Ok(ModelDetectionOutcome::BaseFallback(_))
        ));
        let ambiguity = apply_detection_policy(
            Err(ModelFamilyError::AmbiguousDetection {
                score: 10,
                families: vec!["first".to_owned(), "second".to_owned()],
            }),
            &probe,
            ModelDetectionPolicy::AllowBaseFallback,
        );
        assert!(matches!(
            ambiguity,
            Err(ModelFamilyError::AmbiguousDetection { score: 10, .. })
        ));

        let malformed = ModelProbe {
            tensor_shapes: BTreeMap::from([("weight".to_owned(), vec![u64::MAX, 2])]),
            metadata: BTreeMap::new(),
        };
        assert!(matches!(
            registry.detect(&malformed),
            Err(ModelFamilyError::ProbeDimensionOverflow)
        ));
        assert!(matches!(
            malformed.unet_prefix_selection(),
            Err(ModelFamilyError::ProbeDimensionOverflow)
        ));
        assert!(matches!(
            malformed.consecutive_block_count("blocks.{}"),
            Err(ModelFamilyError::ProbeDimensionOverflow)
        ));
        assert!(matches!(
            malformed.normalized_configuration(),
            Err(ModelFamilyError::ProbeDimensionOverflow)
        ));
    }
}
