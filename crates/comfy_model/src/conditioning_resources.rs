use comfy_tensor::{
    CancellationToken, CpuBackend, DType, DecodedScalar, DeviceId, ExecutionContext, Scalar,
    StorageId, StreamId, Tensor, TensorBackend, TensorError,
    generated_activation_normalization_functional_01::{
        FunctionalError, GeluApproximation, gelu_with_context_exact_native,
        layer_norm_with_context_exact_native, silu_with_context_exact_native,
    },
    generated_comfy_operator_indirection_01::{
        OperatorIndirectionError, cast_to_with_context_exact_native,
        tensor_from_f32_with_context_exact_native, tensor_to_f32_with_context_exact_native,
    },
    generated_elementwise_or_runtime_operation_03::{
        ElementwiseRuntimePartThreeError, real_add_with_context_exact_native,
        sigmoid_with_context_exact_native,
    },
    generated_elementwise_or_runtime_operation_09::{
        ElementwiseRuntimePartNineError, full_like_with_context_exact_native,
    },
    generated_indexing_masking_01::{
        IndexingMaskingPartOneError, NonzeroOutput, gather_method_with_context_exact_native,
        narrow_method_exact_native, nonzero_with_context_exact_native,
        scatter_method_with_context_exact_native,
    },
    generated_linear_algebra_01::{LinearAlgebraPartOneError, matmul_with_context_exact_native},
    generated_neural_network_functional_01::{
        NeuralNetworkFunctionalError, linear_with_context_exact_native,
    },
    generated_reduction_02::tensor_sum_with_context_exact_native,
    generated_shape_layout_transform_02::{
        ShapeLayoutTransformPartTwoError, tensor_repeat_with_context_exact_native,
        torch_cat_with_context_exact_native, torch_reshape_with_context_exact_native,
    },
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    mem,
};
use thiserror::Error;

use crate::{
    attention::{
        AttentionBackend, AttentionError, AttentionFallbackPolicy, AttentionRequest,
        scaled_dot_product_attention_tensor_with_context,
    },
    clip_vision::{
        ClipVisionActivation, ClipVisionConfiguration, ClipVisionError, ClipVisionIntermediate,
        ClipVisionLayerWeights, ClipVisionModelType, ClipVisionOutput, ClipVisionWeights,
        NativeClipVision,
    },
    model_family::{
        ModelDetectionRule, ModelFamilyIdentity, ModelProbe, detect_model_family_rules,
    },
};

pub const STYLE_MODEL_NODES_SOURCE_SHA256: &str =
    "b8dfdde1de8975be762b085048143cc2dda8fc9202695e460ecc2c8dfe44bc4b";
pub const STYLE_MODEL_SD_SOURCE_SHA256: &str =
    "9c378edbcaab01d00397cc0ef1cab7d37d25fca49c707a87cde451030ac6bf42";
pub const STYLE_MODEL_OPS_SOURCE_SHA256: &str =
    "9d8a4ec8357a9bfcd98dddbf06fcc2a0244643a392aacbe0970d945462c86a42";
pub const STYLE_ADAPTER_SOURCE_SHA256: &str =
    "efc52cc85f941e11b509c0339e8950a6680d031b09bec36736d8834b9ccfa1af";
pub const FLUX_REDUX_SOURCE_SHA256: &str =
    "3b7abf43e15fc7b9613a64e701635f943351fa47f5036d87427ea672f93c7952";
pub const PHOTOMAKER_SOURCE_SHA256: &str =
    "c7e86d4684d2884eda20250bd7122e31a8aaf7bfa003245ad0e0431a9ef1957e";
pub const PHOTOMAKER_CLIP_VISION_SOURCE_SHA256: &str =
    "8e3cc5d5d257b52d120885ba7427ff3fdf56129a485fdadac3b6215ae2c67b20";

const MAX_STATE_KEY_BYTES: usize = 1_024;
const DIGEST_CHUNK_BYTES: usize = 64 * 1_024;
const STYLE_LAYER_COUNT: usize = 3;
const STYLE_SOURCE_WIDTH: usize = 1_024;
const STYLE_SOURCE_CONTEXT_WIDTH: usize = 768;
const STYLE_SOURCE_HEADS: usize = 8;
const STYLE_SOURCE_TOKENS: usize = 8;
const REDUX_SOURCE_INPUT_WIDTH: usize = 1_152;
const REDUX_SOURCE_HIDDEN_WIDTH: usize = 12_288;
const REDUX_SOURCE_OUTPUT_WIDTH: usize = 4_096;
const REDUCED_STYLE_WIDTH: usize = 8;
const REDUCED_STYLE_CONTEXT_WIDTH: usize = 6;
const REDUCED_STYLE_HEADS: usize = 2;
const REDUCED_STYLE_TOKENS: usize = 2;
const REDUCED_REDUX_INPUT_WIDTH: usize = 4;
const REDUCED_REDUX_HIDDEN_WIDTH: usize = 12;
const REDUCED_REDUX_OUTPUT_WIDTH: usize = 6;
const LAYER_NORM_EPSILON: f32 = 1.0e-5;
const QUICK_GELU_SCALE: f32 = 1.702;
const PHOTOMAKER_CLIP_STATE_COUNT: usize = 392;
const PHOTOMAKER_STATE_COUNT: usize = 407;
const PHOTOMAKER_SOURCE_HIDDEN: usize = 1_024;
const PHOTOMAKER_SOURCE_INTERMEDIATE: usize = 4_096;
const PHOTOMAKER_SOURCE_HEADS: usize = 16;
const PHOTOMAKER_SOURCE_LAYERS: usize = 24;
const PHOTOMAKER_SOURCE_IMAGE: usize = 224;
const PHOTOMAKER_SOURCE_PATCH: usize = 14;
const PHOTOMAKER_SOURCE_PROJECTION: usize = 768;
const PHOTOMAKER_SOURCE_EXTRA_PROJECTION: usize = 1_280;
#[cfg(any(test, feature = "test-support"))]
const PHOTOMAKER_REDUCED_HIDDEN: usize = 4;
#[cfg(any(test, feature = "test-support"))]
const PHOTOMAKER_REDUCED_INTERMEDIATE: usize = 8;
#[cfg(any(test, feature = "test-support"))]
const PHOTOMAKER_REDUCED_HEADS: usize = 2;
#[cfg(any(test, feature = "test-support"))]
const PHOTOMAKER_REDUCED_IMAGE: usize = 4;
#[cfg(any(test, feature = "test-support"))]
const PHOTOMAKER_REDUCED_PATCH: usize = 2;
#[cfg(any(test, feature = "test-support"))]
const PHOTOMAKER_REDUCED_PROJECTION: usize = 3;
#[cfg(any(test, feature = "test-support"))]
const PHOTOMAKER_REDUCED_EXTRA_PROJECTION: usize = 5;

const STYLE_DETECTION_RULES: [ModelDetectionRule; 1] = [ModelDetectionRule::KeyPresent {
    key: "style_embedding",
    score: 1,
}];
const REDUX_DETECTION_RULES: [ModelDetectionRule; 1] = [ModelDetectionRule::KeyPresent {
    key: "redux_down.weight",
    score: 1,
}];

#[derive(Clone, Debug)]
pub struct NativeStyleModelCheckpoint {
    pub artifact_sha256: String,
    pub ordered_state: Vec<(String, Tensor)>,
    pub memory_budget_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeStyleModelArchitecture {
    StyleAdapter,
    FluxRedux,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NativeStyleModelConfiguration {
    architecture: NativeStyleModelArchitecture,
    input_width: usize,
    hidden_width: usize,
    output_width: usize,
    heads: usize,
    layers: usize,
    output_tokens: usize,
    source_exact: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StateSpecification {
    key: String,
    shape: Vec<u64>,
}

#[derive(Clone, Debug)]
struct NativeStyleAttentionProjection {
    query_weight: Tensor,
    key_weight: Tensor,
    value_weight: Tensor,
    query_bias: Tensor,
    key_bias: Tensor,
    value_bias: Tensor,
}

#[derive(Debug)]
pub struct NativeStyleModelResource {
    configuration: NativeStyleModelConfiguration,
    artifact_sha256: String,
    source_state: BTreeMap<String, Tensor>,
    execution_state: BTreeMap<String, Tensor>,
    style_attention: Box<[NativeStyleAttentionProjection]>,
    source_dtype: DType,
    stream: StreamId,
    memory_budget_bytes: u64,
    resident_bytes: u64,
    semantic_digest_sha256: String,
}

#[derive(Debug, Error)]
pub enum NativeStyleModelError {
    #[error("style-model execution was cancelled")]
    Cancelled,
    #[error("style-model architecture is unsupported")]
    UnsupportedArchitecture,
    #[error("style-model checkpoint is invalid: {0}")]
    InvalidCheckpoint(String),
    #[error("style-model state contains duplicate key {0}")]
    DuplicateStateKey(String),
    #[error("style-model state is missing key {0}")]
    MissingState(String),
    #[error("style-model state is unexpected: {0}")]
    UnexpectedState(String),
    #[error("style-model state {key} expected {expected:?}, got {actual:?} {actual_dtype:?}")]
    StateShape {
        key: String,
        expected: Vec<u64>,
        actual: Vec<u64>,
        actual_dtype: DType,
    },
    #[error("style-model retained semantic state changed")]
    SemanticStateChanged,
    #[error("style-model CLIP-vision input is invalid: {0}")]
    InvalidInput(String),
    #[error("style-model shape arithmetic overflowed")]
    ShapeOverflow,
    #[error("style-model allocation failed")]
    Allocation,
    #[error("style-model memory requirement {required} exceeds budget {budget}")]
    OutOfMemory { required: u64, budget: u64 },
    #[error("style-model canonical owner failed: {0}")]
    Canonical(String),
    #[error(transparent)]
    ClipVision(ClipVisionError),
    #[error(transparent)]
    Tensor(TensorError),
}

impl From<comfy_types::CancellationError> for NativeStyleModelError {
    fn from(_: comfy_types::CancellationError) -> Self {
        Self::Cancelled
    }
}

impl From<TensorError> for NativeStyleModelError {
    fn from(error: TensorError) -> Self {
        match error {
            TensorError::Cancelled => Self::Cancelled,
            error => Self::Tensor(error),
        }
    }
}

impl From<ClipVisionError> for NativeStyleModelError {
    fn from(error: ClipVisionError) -> Self {
        match error {
            ClipVisionError::Cancelled | ClipVisionError::Tensor(TensorError::Cancelled) => {
                Self::Cancelled
            }
            error => Self::ClipVision(error),
        }
    }
}

impl NativeStyleModelResource {
    pub fn from_checkpoint(
        backend: &CpuBackend,
        checkpoint: NativeStyleModelCheckpoint,
        context: &ExecutionContext<'_>,
    ) -> Result<Self, NativeStyleModelError> {
        Self::checked(backend, checkpoint, true, context)
    }

    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn from_reduced_fixture(
        backend: &CpuBackend,
        checkpoint: NativeStyleModelCheckpoint,
        context: &ExecutionContext<'_>,
    ) -> Result<Self, NativeStyleModelError> {
        Self::checked(backend, checkpoint, false, context)
    }

    fn checked(
        backend: &CpuBackend,
        checkpoint: NativeStyleModelCheckpoint,
        source_exact: bool,
        context: &ExecutionContext<'_>,
    ) -> Result<Self, NativeStyleModelError> {
        context.check()?;
        validate_sha256(&checkpoint.artifact_sha256)?;
        if checkpoint.memory_budget_bytes == 0 || checkpoint.ordered_state.is_empty() {
            return Err(NativeStyleModelError::InvalidCheckpoint(
                "state cardinality or memory budget is invalid".to_owned(),
            ));
        }
        raw_checkpoint_preflight(
            &checkpoint.artifact_sha256,
            checkpoint.artifact_sha256.capacity(),
            &checkpoint.ordered_state,
            checkpoint.ordered_state.capacity(),
            checkpoint.memory_budget_bytes,
        )?;
        let configuration = detect_configuration(&checkpoint.ordered_state, source_exact)?;
        let specifications = state_manifest(configuration)?;
        validate_ordered_keys(&checkpoint.ordered_state, &specifications)?;
        let source_dtype = validate_source_state(
            &checkpoint.ordered_state,
            &specifications,
            context.stream,
            context.cancellation,
        )?;
        let construction_peak = construction_memory_preflight(
            &checkpoint.artifact_sha256,
            checkpoint.artifact_sha256.capacity(),
            &checkpoint.ordered_state,
            &specifications,
            configuration,
        )?;
        if construction_peak > checkpoint.memory_budget_bytes {
            return Err(NativeStyleModelError::OutOfMemory {
                required: construction_peak,
                budget: checkpoint.memory_budget_bytes,
            });
        }

        let source_state = checkpoint
            .ordered_state
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        let mut execution_state = BTreeMap::new();
        for (index, specification) in specifications.iter().enumerate() {
            if index.is_multiple_of(8) {
                context.check()?;
            }
            let source = source_state
                .get(&specification.key)
                .ok_or_else(|| NativeStyleModelError::MissingState(specification.key.clone()))?;
            let projected = cast_to_with_context_exact_native(
                backend,
                source,
                DType::F32,
                DeviceId::CPU,
                false,
                true,
                context,
            )
            .map_err(canonical)?;
            validate_finite_tensor(&specification.key, &projected, context.cancellation)?;
            execution_state.insert(specification.key.clone(), projected);
        }
        validate_distinct_projection_storage(&source_state, &execution_state)?;
        let style_attention =
            build_style_attention(backend, configuration, &execution_state, context)?;
        let semantic_digest_sha256 = semantic_digest(
            configuration,
            &checkpoint.artifact_sha256,
            source_dtype,
            &source_state,
            &execution_state,
            &specifications,
            context.cancellation,
        )?;
        let resident_bytes =
            resident_tensor_bytes([&source_state, &execution_state], context.cancellation)?
                .checked_add(resident_owned_bytes(
                    &checkpoint.artifact_sha256,
                    checkpoint.artifact_sha256.capacity(),
                    &semantic_digest_sha256,
                    semantic_digest_sha256.capacity(),
                    &source_state,
                    &execution_state,
                    style_attention.len(),
                )?)
                .ok_or(NativeStyleModelError::ShapeOverflow)?;
        if resident_bytes > checkpoint.memory_budget_bytes {
            return Err(NativeStyleModelError::OutOfMemory {
                required: resident_bytes,
                budget: checkpoint.memory_budget_bytes,
            });
        }
        context.check()?;
        Ok(Self {
            configuration,
            artifact_sha256: checkpoint.artifact_sha256,
            source_state,
            execution_state,
            style_attention,
            source_dtype,
            stream: context.stream,
            memory_budget_bytes: checkpoint.memory_budget_bytes,
            resident_bytes,
            semantic_digest_sha256,
        })
    }

    pub fn identifier(&self) -> &'static str {
        match self.configuration.architecture {
            NativeStyleModelArchitecture::StyleAdapter => "style-adapter",
            NativeStyleModelArchitecture::FluxRedux => "flux-redux",
        }
    }

    pub fn artifact_sha256(&self) -> &str {
        &self.artifact_sha256
    }

    pub fn semantic_digest_sha256(&self) -> &str {
        &self.semantic_digest_sha256
    }

    pub const fn source_dtype(&self) -> DType {
        self.source_dtype
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub const fn is_source_exact_profile(&self) -> bool {
        self.configuration.source_exact
    }

    pub fn resident_owned_bytes(&self) -> Result<u64, NativeStyleModelError> {
        resident_owned_bytes(
            &self.artifact_sha256,
            self.artifact_sha256.capacity(),
            &self.semantic_digest_sha256,
            self.semantic_digest_sha256.capacity(),
            &self.source_state,
            &self.execution_state,
            self.style_attention.len(),
        )
    }

    pub fn resident_tensor_allocations(
        &self,
    ) -> Result<Vec<(StorageId, u64)>, NativeStyleModelError> {
        resident_tensor_allocations(
            [&self.source_state, &self.execution_state],
            &CancellationToken::default(),
        )
    }

    pub fn reconstruct_checkpoint(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<NativeStyleModelCheckpoint, NativeStyleModelError> {
        self.validate(cancellation)?;
        let specifications = state_manifest(self.configuration)?;
        let mut ordered_state = Vec::new();
        ordered_state
            .try_reserve_exact(specifications.len())
            .map_err(|_| NativeStyleModelError::Allocation)?;
        for (index, specification) in specifications.iter().enumerate() {
            if index.is_multiple_of(8) {
                cancellation.check()?;
            }
            ordered_state.push((
                specification.key.clone(),
                self.source_state
                    .get(&specification.key)
                    .ok_or_else(|| NativeStyleModelError::MissingState(specification.key.clone()))?
                    .clone(),
            ));
        }
        cancellation.check()?;
        Ok(NativeStyleModelCheckpoint {
            artifact_sha256: self.artifact_sha256.clone(),
            ordered_state,
            memory_budget_bytes: self.memory_budget_bytes,
        })
    }

    pub fn validate(&self, cancellation: &CancellationToken) -> Result<(), NativeStyleModelError> {
        cancellation.check()?;
        let specifications = state_manifest(self.configuration)?;
        let source_order = specifications
            .iter()
            .map(|specification| {
                self.source_state
                    .get(&specification.key)
                    .cloned()
                    .map(|tensor| (specification.key.clone(), tensor))
                    .ok_or_else(|| NativeStyleModelError::MissingState(specification.key.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let dtype =
            validate_source_state(&source_order, &specifications, self.stream, cancellation)?;
        if dtype != self.source_dtype
            || self.source_state.len() != specifications.len()
            || self.execution_state.len() != specifications.len()
        {
            return Err(NativeStyleModelError::SemanticStateChanged);
        }
        for specification in &specifications {
            let projected = self
                .execution_state
                .get(&specification.key)
                .ok_or_else(|| NativeStyleModelError::MissingState(specification.key.clone()))?;
            if projected.descriptor().shape() != specification.shape
                || projected.descriptor().dtype() != DType::F32
                || projected.descriptor().device() != DeviceId::CPU
                || projected.descriptor().stream() != self.stream
                || !projected.descriptor().is_contiguous()?
            {
                return Err(NativeStyleModelError::SemanticStateChanged);
            }
            validate_finite_tensor(&specification.key, projected, cancellation)?;
        }
        validate_distinct_projection_storage(&self.source_state, &self.execution_state)?;
        validate_attention_views(self, cancellation)?;
        let digest = semantic_digest(
            self.configuration,
            &self.artifact_sha256,
            self.source_dtype,
            &self.source_state,
            &self.execution_state,
            &specifications,
            cancellation,
        )?;
        let resident =
            resident_tensor_bytes([&self.source_state, &self.execution_state], cancellation)?
                .checked_add(self.resident_owned_bytes()?)
                .ok_or(NativeStyleModelError::ShapeOverflow)?;
        if digest != self.semantic_digest_sha256 || resident != self.resident_bytes {
            return Err(NativeStyleModelError::SemanticStateChanged);
        }
        cancellation.check()?;
        Ok(())
    }

    pub fn get_cond(
        &self,
        backend: &CpuBackend,
        input: &ClipVisionOutput,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, NativeStyleModelError> {
        context.check()?;
        self.validate(context.cancellation)?;
        input.validate()?;
        let hidden = &input.last_hidden_state;
        let [batch, tokens, width] = hidden.descriptor().shape() else {
            return Err(NativeStyleModelError::InvalidInput(
                "last_hidden_state must have rank three".to_owned(),
            ));
        };
        if *batch == 0 || *tokens == 0 || *width != u64_from(self.configuration.input_width)? {
            return Err(NativeStyleModelError::InvalidInput(
                "last_hidden_state has the wrong shape".to_owned(),
            ));
        }
        if hidden.descriptor().dtype() != DType::F32
            || hidden.descriptor().device() != DeviceId::CPU
            || hidden.descriptor().stream() != context.stream
            || !hidden.descriptor().is_contiguous()?
        {
            return Err(NativeStyleModelError::InvalidInput(
                "last_hidden_state must be contiguous F32 CPU state on the execution stream"
                    .to_owned(),
            ));
        }
        let batch = usize_from(*batch)?;
        let tokens = usize_from(*tokens)?;
        preflight_invocation_memory(self, batch, tokens)?;
        let input_values =
            tensor_to_f32_with_context_exact_native(backend, hidden, context).map_err(canonical)?;
        if input_values.iter().any(|value| !value.is_finite()) {
            return Err(NativeStyleModelError::InvalidInput(
                "last_hidden_state contains a non-finite value".to_owned(),
            ));
        }
        let output = match self.configuration.architecture {
            NativeStyleModelArchitecture::StyleAdapter => {
                self.execute_style_adapter(backend, hidden, batch, tokens, context)?
            }
            NativeStyleModelArchitecture::FluxRedux => {
                self.execute_flux_redux(backend, &input_values, batch, tokens, context)?
            }
        };
        context.check()?;
        Ok(output)
    }

    fn execute_style_adapter(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        batch: usize,
        input_tokens: usize,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, NativeStyleModelError> {
        let width = self.configuration.input_width;
        let style_embedding = self.execution_tensor("style_embedding")?;
        let repeated = tensor_repeat_with_context_exact_native(
            backend,
            style_embedding,
            &[i64_from(batch)?, 1, 1],
            context,
        )
        .map_err(canonical)?;
        let zeros = full_like_with_context_exact_native(
            backend,
            &repeated,
            Scalar::Float(0.0),
            Some(DType::F32),
            context,
        )
        .map_err(canonical)?;
        let repeated = real_add_with_context_exact_native(backend, &repeated, &zeros, context)
            .map_err(canonical)?;
        let concatenated =
            torch_cat_with_context_exact_native(backend, &[input.clone(), repeated], 1, context)
                .map_err(canonical)?;
        let sequence = input_tokens
            .checked_add(self.configuration.output_tokens)
            .ok_or(NativeStyleModelError::ShapeOverflow)?;
        let mut values = tensor_to_f32_with_context_exact_native(backend, &concatenated, context)
            .map_err(canonical)?;
        values = self.layer_norm(
            backend,
            &values,
            &[batch, sequence, width],
            "ln_pre.weight",
            "ln_pre.bias",
            context,
        )?;

        for layer in 0..self.configuration.layers {
            context.check()?;
            let normalized = self.layer_norm(
                backend,
                &values,
                &[batch, sequence, width],
                &format!("transformer_layes.{layer}.ln_1.weight"),
                &format!("transformer_layes.{layer}.ln_1.bias"),
                context,
            )?;
            let attention =
                self.execute_attention(backend, layer, &normalized, batch, sequence, context)?;
            let projected = self.linear_state(
                backend,
                &attention,
                &[batch, sequence, width],
                &format!("transformer_layes.{layer}.attn.out_proj.weight"),
                Some(&format!("transformer_layes.{layer}.attn.out_proj.bias")),
                context,
            )?;
            add_in_place(&mut values, &projected, context.cancellation)?;

            let normalized = self.layer_norm(
                backend,
                &values,
                &[batch, sequence, width],
                &format!("transformer_layes.{layer}.ln_2.weight"),
                &format!("transformer_layes.{layer}.ln_2.bias"),
                context,
            )?;
            let expanded = self.linear_state(
                backend,
                &normalized,
                &[batch, sequence, width],
                &format!("transformer_layes.{layer}.mlp.c_fc.weight"),
                Some(&format!("transformer_layes.{layer}.mlp.c_fc.bias")),
                context,
            )?;
            let activated = quick_gelu(
                backend,
                &expanded,
                &[
                    batch,
                    sequence,
                    width
                        .checked_mul(4)
                        .ok_or(NativeStyleModelError::ShapeOverflow)?,
                ],
                context,
            )?;
            let projected = self.linear_state(
                backend,
                &activated,
                &[
                    batch,
                    sequence,
                    width
                        .checked_mul(4)
                        .ok_or(NativeStyleModelError::ShapeOverflow)?,
                ],
                &format!("transformer_layes.{layer}.mlp.c_proj.weight"),
                Some(&format!("transformer_layes.{layer}.mlp.c_proj.bias")),
                context,
            )?;
            add_in_place(&mut values, &projected, context.cancellation)?;
            context.check()?;
        }

        let selected_count = batch
            .checked_mul(self.configuration.output_tokens)
            .and_then(|value| value.checked_mul(width))
            .ok_or(NativeStyleModelError::ShapeOverflow)?;
        let mut selected = Vec::new();
        selected
            .try_reserve_exact(selected_count)
            .map_err(|_| NativeStyleModelError::Allocation)?;
        for batch_index in 0..batch {
            context.check()?;
            let start = batch_index
                .checked_mul(sequence)
                .and_then(|value| value.checked_add(input_tokens))
                .and_then(|value| value.checked_mul(width))
                .ok_or(NativeStyleModelError::ShapeOverflow)?;
            let end = start
                .checked_add(
                    self.configuration
                        .output_tokens
                        .checked_mul(width)
                        .ok_or(NativeStyleModelError::ShapeOverflow)?,
                )
                .ok_or(NativeStyleModelError::ShapeOverflow)?;
            selected.extend_from_slice(
                values
                    .get(start..end)
                    .ok_or(NativeStyleModelError::ShapeOverflow)?,
            );
        }
        let selected = self.layer_norm(
            backend,
            &selected,
            &[batch, self.configuration.output_tokens, width],
            "ln_post.weight",
            "ln_post.bias",
            context,
        )?;
        let selected = tensor_from_f32_with_context_exact_native(
            backend,
            &[
                u64_from(batch)?,
                u64_from(self.configuration.output_tokens)?,
                u64_from(width)?,
            ],
            &selected,
            DType::F32,
            DeviceId::CPU,
            context,
        )
        .map_err(canonical)?;
        let output = matmul_with_context_exact_native(
            backend,
            &selected,
            self.execution_tensor("proj")?,
            context,
        )
        .map_err(canonical)?;
        context.check()?;
        Ok(output)
    }

    fn execute_flux_redux(
        &self,
        backend: &CpuBackend,
        input: &[f32],
        batch: usize,
        tokens: usize,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, NativeStyleModelError> {
        let expanded = self.linear_state(
            backend,
            input,
            &[batch, tokens, self.configuration.input_width],
            "redux_up.weight",
            Some("redux_up.bias"),
            context,
        )?;
        let activated = silu_with_context_exact_native(backend, &expanded, DeviceId::CPU, context)
            .map_err(canonical)?;
        let output = self.linear_state(
            backend,
            &activated,
            &[batch, tokens, self.configuration.hidden_width],
            "redux_down.weight",
            Some("redux_down.bias"),
            context,
        )?;
        let output = tensor_from_f32_with_context_exact_native(
            backend,
            &[
                u64_from(batch)?,
                u64_from(tokens)?,
                u64_from(self.configuration.output_width)?,
            ],
            &output,
            DType::F32,
            DeviceId::CPU,
            context,
        )
        .map_err(canonical)?;
        context.check()?;
        Ok(output)
    }

    fn execute_attention(
        &self,
        backend: &CpuBackend,
        layer: usize,
        input: &[f32],
        batch: usize,
        tokens: usize,
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<f32>, NativeStyleModelError> {
        let projection = self
            .style_attention
            .get(layer)
            .ok_or(NativeStyleModelError::SemanticStateChanged)?;
        let width = self.configuration.input_width;
        let query = linear_tensor(
            backend,
            input,
            &[batch, tokens, width],
            &projection.query_weight,
            Some(&projection.query_bias),
            context,
        )?;
        let key = linear_tensor(
            backend,
            input,
            &[batch, tokens, width],
            &projection.key_weight,
            Some(&projection.key_bias),
            context,
        )?;
        let value = linear_tensor(
            backend,
            input,
            &[batch, tokens, width],
            &projection.value_weight,
            Some(&projection.value_bias),
            context,
        )?;
        let head_dimension = width
            .checked_div(self.configuration.heads)
            .filter(|dimension| *dimension > 0)
            .ok_or(NativeStyleModelError::ShapeOverflow)?;
        let attention_workspace_bytes = u64_from(batch)?
            .checked_mul(u64_from(self.configuration.heads)?)
            .and_then(|value| value.checked_mul(u64_from(tokens).ok()?))
            .and_then(|value| value.checked_mul(u64_from(tokens).ok()?))
            .and_then(|value| value.checked_mul(8))
            .ok_or(NativeStyleModelError::ShapeOverflow)?;
        let shape = [
            u64_from(batch)?,
            u64_from(tokens)?,
            u64_from(self.configuration.heads)?,
            u64_from(head_dimension)?,
        ];
        let query = tensor_from_f32_with_context_exact_native(
            backend,
            &shape,
            &query,
            DType::F32,
            DeviceId::CPU,
            context,
        )
        .map_err(canonical)?;
        let key = tensor_from_f32_with_context_exact_native(
            backend,
            &shape,
            &key,
            DType::F32,
            DeviceId::CPU,
            context,
        )
        .map_err(canonical)?;
        let value = tensor_from_f32_with_context_exact_native(
            backend,
            &shape,
            &value,
            DType::F32,
            DeviceId::CPU,
            context,
        )
        .map_err(canonical)?;
        let output = scaled_dot_product_attention_tensor_with_context(
            backend,
            AttentionRequest {
                backend: AttentionBackend::PytorchSdp,
                fallback: AttentionFallbackPolicy::AllowExactNative,
                batch,
                query_tokens: tokens,
                key_tokens: tokens,
                heads: self.configuration.heads,
                head_dimension,
                value_dimension: head_dimension,
                scale: None,
                workspace_limit_bytes: usize::try_from(attention_workspace_bytes)
                    .map_err(|_| NativeStyleModelError::ShapeOverflow)?,
            },
            &query,
            &key,
            &value,
            None,
            context,
        )
        .map_err(attention_error)?;
        tensor_to_f32_with_context_exact_native(backend, &output, context).map_err(canonical)
    }

    fn layer_norm(
        &self,
        backend: &CpuBackend,
        input: &[f32],
        shape: &[usize],
        weight_key: &str,
        bias_key: &str,
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<f32>, NativeStyleModelError> {
        let weight = tensor_to_f32_with_context_exact_native(
            backend,
            self.execution_tensor(weight_key)?,
            context,
        )
        .map_err(canonical)?;
        let bias = tensor_to_f32_with_context_exact_native(
            backend,
            self.execution_tensor(bias_key)?,
            context,
        )
        .map_err(canonical)?;
        layer_norm_with_context_exact_native(
            backend,
            input,
            shape,
            &[self.configuration.input_width],
            Some(&weight),
            Some(&bias),
            LAYER_NORM_EPSILON,
            DeviceId::CPU,
            context,
        )
        .map_err(canonical)
    }

    fn linear_state(
        &self,
        backend: &CpuBackend,
        input: &[f32],
        input_shape: &[usize],
        weight_key: &str,
        bias_key: Option<&str>,
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<f32>, NativeStyleModelError> {
        let weight = self.execution_tensor(weight_key)?;
        let bias = bias_key.map(|key| self.execution_tensor(key)).transpose()?;
        linear_tensor(backend, input, input_shape, weight, bias, context)
    }

    fn execution_tensor(&self, key: &str) -> Result<&Tensor, NativeStyleModelError> {
        self.execution_state
            .get(key)
            .ok_or_else(|| NativeStyleModelError::MissingState(key.to_owned()))
    }
}

fn detect_configuration(
    ordered_state: &[(String, Tensor)],
    source_exact: bool,
) -> Result<NativeStyleModelConfiguration, NativeStyleModelError> {
    let mut tensor_shapes = BTreeMap::new();
    for (key, tensor) in ordered_state {
        if tensor_shapes
            .insert(key.clone(), tensor.descriptor().shape().to_vec())
            .is_some()
        {
            return Err(NativeStyleModelError::DuplicateStateKey(key.clone()));
        }
    }
    let probe = ModelProbe {
        tensor_shapes,
        metadata: BTreeMap::new(),
    };
    if probe.tensor_shapes.contains_key("style_embedding") {
        detect_model_family_rules(
            ModelFamilyIdentity::new("COMFY-MODEL-0393", "style-adapter", "1")
                .map_err(canonical)?,
            &STYLE_DETECTION_RULES,
            &probe,
        )
        .map_err(canonical)?;
        Ok(if source_exact {
            style_configuration(
                STYLE_SOURCE_WIDTH,
                STYLE_SOURCE_CONTEXT_WIDTH,
                STYLE_SOURCE_HEADS,
                STYLE_SOURCE_TOKENS,
                true,
            )
        } else {
            style_configuration(
                REDUCED_STYLE_WIDTH,
                REDUCED_STYLE_CONTEXT_WIDTH,
                REDUCED_STYLE_HEADS,
                REDUCED_STYLE_TOKENS,
                false,
            )
        })
    } else if probe.tensor_shapes.contains_key("redux_down.weight") {
        detect_model_family_rules(
            ModelFamilyIdentity::new("COMFY-MODEL-0393", "flux-redux", "1").map_err(canonical)?,
            &REDUX_DETECTION_RULES,
            &probe,
        )
        .map_err(canonical)?;
        Ok(if source_exact {
            redux_configuration(
                REDUX_SOURCE_INPUT_WIDTH,
                REDUX_SOURCE_HIDDEN_WIDTH,
                REDUX_SOURCE_OUTPUT_WIDTH,
                true,
            )
        } else {
            redux_configuration(
                REDUCED_REDUX_INPUT_WIDTH,
                REDUCED_REDUX_HIDDEN_WIDTH,
                REDUCED_REDUX_OUTPUT_WIDTH,
                false,
            )
        })
    } else {
        Err(NativeStyleModelError::UnsupportedArchitecture)
    }
}

fn raw_checkpoint_preflight(
    artifact: &str,
    artifact_capacity: usize,
    ordered_state: &[(String, Tensor)],
    ordered_state_capacity: usize,
    memory_budget_bytes: u64,
) -> Result<(), NativeStyleModelError> {
    if !matches!(ordered_state.len(), 4 | 42) {
        return Err(NativeStyleModelError::UnexpectedState(format!(
            "expected 4 or 42 entries, got {}",
            ordered_state.len()
        )));
    }
    let key_bytes = ordered_state.iter().try_fold(0_u64, |total, (key, _)| {
        if key.is_empty() || key.len() > MAX_STATE_KEY_BYTES {
            return Err(NativeStyleModelError::InvalidCheckpoint(
                "state key length is invalid".to_owned(),
            ));
        }
        total
            .checked_add(u64_from(key.capacity())?)
            .ok_or(NativeStyleModelError::ShapeOverflow)
    })?;
    for (index, (key, _)) in ordered_state.iter().enumerate() {
        if ordered_state
            .iter()
            .skip(index + 1)
            .any(|(candidate, _)| candidate == key)
        {
            return Err(NativeStyleModelError::DuplicateStateKey(key.clone()));
        }
    }
    let tensor_bytes = ordered_state.iter().try_fold(0_u64, |total, (_, tensor)| {
        total
            .checked_add(tensor.storage_byte_len())
            .ok_or(NativeStyleModelError::ShapeOverflow)
    })?;
    let tuple_bytes = u64_from(ordered_state_capacity)?
        .checked_mul(
            u64::try_from(mem::size_of::<(String, Tensor)>())
                .map_err(|_| NativeStyleModelError::ShapeOverflow)?,
        )
        .ok_or(NativeStyleModelError::ShapeOverflow)?;
    let manifest_bytes = u64_from(ordered_state.len())?
        .checked_mul(
            u64::try_from(mem::size_of::<StateSpecification>())
                .map_err(|_| NativeStyleModelError::ShapeOverflow)?
                .checked_add(16)
                .ok_or(NativeStyleModelError::ShapeOverflow)?,
        )
        .ok_or(NativeStyleModelError::ShapeOverflow)?;
    let detector_and_manifest_keys = key_bytes
        .checked_mul(2)
        .ok_or(NativeStyleModelError::ShapeOverflow)?;
    let required = tensor_bytes
        .checked_add(u64_from(artifact_capacity.max(artifact.len()))?)
        .and_then(|value| value.checked_add(tuple_bytes))
        .and_then(|value| value.checked_add(detector_and_manifest_keys))
        .and_then(|value| value.checked_add(manifest_bytes))
        .and_then(|value| value.checked_add(map_node_estimate(ordered_state.len()).ok()?))
        .ok_or(NativeStyleModelError::ShapeOverflow)?;
    if required > memory_budget_bytes {
        return Err(NativeStyleModelError::OutOfMemory {
            required,
            budget: memory_budget_bytes,
        });
    }
    Ok(())
}

const fn style_configuration(
    width: usize,
    context_width: usize,
    heads: usize,
    output_tokens: usize,
    source_exact: bool,
) -> NativeStyleModelConfiguration {
    NativeStyleModelConfiguration {
        architecture: NativeStyleModelArchitecture::StyleAdapter,
        input_width: width,
        hidden_width: width * 4,
        output_width: context_width,
        heads,
        layers: STYLE_LAYER_COUNT,
        output_tokens,
        source_exact,
    }
}

const fn redux_configuration(
    input_width: usize,
    hidden_width: usize,
    output_width: usize,
    source_exact: bool,
) -> NativeStyleModelConfiguration {
    NativeStyleModelConfiguration {
        architecture: NativeStyleModelArchitecture::FluxRedux,
        input_width,
        hidden_width,
        output_width,
        heads: 0,
        layers: 0,
        output_tokens: 0,
        source_exact,
    }
}

fn state_manifest(
    configuration: NativeStyleModelConfiguration,
) -> Result<Vec<StateSpecification>, NativeStyleModelError> {
    let mut specifications = Vec::new();
    let mut push = |key: String, shape: Vec<u64>| {
        specifications.push(StateSpecification { key, shape });
    };
    match configuration.architecture {
        NativeStyleModelArchitecture::StyleAdapter => {
            let width = u64_from(configuration.input_width)?;
            let hidden = u64_from(configuration.hidden_width)?;
            push(
                "style_embedding".to_owned(),
                vec![1, u64_from(configuration.output_tokens)?, width],
            );
            push(
                "proj".to_owned(),
                vec![width, u64_from(configuration.output_width)?],
            );
            for layer in 0..configuration.layers {
                let prefix = format!("transformer_layes.{layer}");
                push(
                    format!("{prefix}.attn.in_proj_weight"),
                    vec![width * 3, width],
                );
                push(format!("{prefix}.attn.in_proj_bias"), vec![width * 3]);
                push(format!("{prefix}.attn.out_proj.weight"), vec![width, width]);
                push(format!("{prefix}.attn.out_proj.bias"), vec![width]);
                push(format!("{prefix}.ln_1.weight"), vec![width]);
                push(format!("{prefix}.ln_1.bias"), vec![width]);
                push(format!("{prefix}.mlp.c_fc.weight"), vec![hidden, width]);
                push(format!("{prefix}.mlp.c_fc.bias"), vec![hidden]);
                push(format!("{prefix}.mlp.c_proj.weight"), vec![width, hidden]);
                push(format!("{prefix}.mlp.c_proj.bias"), vec![width]);
                push(format!("{prefix}.ln_2.weight"), vec![width]);
                push(format!("{prefix}.ln_2.bias"), vec![width]);
            }
            push("ln_post.weight".to_owned(), vec![width]);
            push("ln_post.bias".to_owned(), vec![width]);
            push("ln_pre.weight".to_owned(), vec![width]);
            push("ln_pre.bias".to_owned(), vec![width]);
        }
        NativeStyleModelArchitecture::FluxRedux => {
            let input = u64_from(configuration.input_width)?;
            let hidden = u64_from(configuration.hidden_width)?;
            let output = u64_from(configuration.output_width)?;
            push("redux_up.weight".to_owned(), vec![hidden, input]);
            push("redux_up.bias".to_owned(), vec![hidden]);
            push("redux_down.weight".to_owned(), vec![output, hidden]);
            push("redux_down.bias".to_owned(), vec![output]);
        }
    }
    Ok(specifications)
}

fn validate_ordered_keys(
    ordered_state: &[(String, Tensor)],
    specifications: &[StateSpecification],
) -> Result<(), NativeStyleModelError> {
    if ordered_state.len() != specifications.len() {
        return Err(NativeStyleModelError::UnexpectedState(format!(
            "expected {} entries, got {}",
            specifications.len(),
            ordered_state.len()
        )));
    }
    for ((actual, _), expected) in ordered_state.iter().zip(specifications) {
        if actual != &expected.key {
            return Err(NativeStyleModelError::UnexpectedState(format!(
                "expected {}, got {actual}",
                expected.key
            )));
        }
    }
    Ok(())
}

fn validate_source_state(
    ordered_state: &[(String, Tensor)],
    specifications: &[StateSpecification],
    stream: StreamId,
    cancellation: &CancellationToken,
) -> Result<DType, NativeStyleModelError> {
    validate_ordered_keys(ordered_state, specifications)?;
    let mut source_dtype = None;
    let mut storage_ids = HashSet::new();
    for (index, ((key, tensor), specification)) in
        ordered_state.iter().zip(specifications).enumerate()
    {
        if index.is_multiple_of(8) {
            cancellation.check()?;
        }
        if key.len() > MAX_STATE_KEY_BYTES || key.is_empty() {
            return Err(NativeStyleModelError::InvalidCheckpoint(
                "state key length is invalid".to_owned(),
            ));
        }
        let descriptor = tensor.descriptor();
        if descriptor.shape() != specification.shape
            || !matches!(descriptor.dtype(), DType::F32 | DType::F16 | DType::Bf16)
        {
            return Err(NativeStyleModelError::StateShape {
                key: key.clone(),
                expected: specification.shape.clone(),
                actual: descriptor.shape().to_vec(),
                actual_dtype: descriptor.dtype(),
            });
        }
        if descriptor.device() != DeviceId::CPU
            || descriptor.stream() != stream
            || !descriptor.is_contiguous()?
        {
            return Err(NativeStyleModelError::InvalidCheckpoint(format!(
                "state {key} must be contiguous CPU storage on the construction stream"
            )));
        }
        if source_dtype
            .replace(descriptor.dtype())
            .is_some_and(|dtype| dtype != descriptor.dtype())
        {
            return Err(NativeStyleModelError::InvalidCheckpoint(
                "all source tensors must use one storage dtype".to_owned(),
            ));
        }
        if !storage_ids.insert(tensor.storage_id()) {
            return Err(NativeStyleModelError::InvalidCheckpoint(format!(
                "state {key} aliases another source tensor"
            )));
        }
        validate_finite_source_tensor(key, tensor, cancellation)?;
    }
    source_dtype
        .ok_or_else(|| NativeStyleModelError::InvalidCheckpoint("state is empty".to_owned()))
}

fn validate_finite_source_tensor(
    key: &str,
    tensor: &Tensor,
    cancellation: &CancellationToken,
) -> Result<(), NativeStyleModelError> {
    let element_count = usize::try_from(tensor.descriptor().element_count()?)
        .map_err(|_| NativeStyleModelError::ShapeOverflow)?;
    for index in 0..element_count {
        if index.is_multiple_of(16_384) {
            cancellation.check()?;
        }
        match tensor
            .descriptor()
            .dtype()
            .decode_scalar(tensor.linear_element_bytes(u64_from(index)?)?)?
        {
            DecodedScalar::Real(value) if value.is_finite() => {}
            _ => {
                return Err(NativeStyleModelError::InvalidCheckpoint(format!(
                    "state {key} contains a non-finite or non-real value"
                )));
            }
        }
    }
    Ok(())
}

fn validate_distinct_projection_storage(
    source_state: &BTreeMap<String, Tensor>,
    execution_state: &BTreeMap<String, Tensor>,
) -> Result<(), NativeStyleModelError> {
    let source_storage_ids = source_state
        .values()
        .map(Tensor::storage_id)
        .collect::<HashSet<_>>();
    let mut execution_storage_ids = HashSet::new();
    for tensor in execution_state.values() {
        if source_storage_ids.contains(&tensor.storage_id())
            || !execution_storage_ids.insert(tensor.storage_id())
        {
            return Err(NativeStyleModelError::SemanticStateChanged);
        }
    }
    Ok(())
}

fn build_style_attention(
    backend: &CpuBackend,
    configuration: NativeStyleModelConfiguration,
    execution_state: &BTreeMap<String, Tensor>,
    context: &ExecutionContext<'_>,
) -> Result<Box<[NativeStyleAttentionProjection]>, NativeStyleModelError> {
    if configuration.architecture != NativeStyleModelArchitecture::StyleAdapter {
        return Ok(Vec::new().into_boxed_slice());
    }
    let width = u64_from(configuration.input_width)?;
    let mut layers = Vec::new();
    layers
        .try_reserve_exact(configuration.layers)
        .map_err(|_| NativeStyleModelError::Allocation)?;
    for layer in 0..configuration.layers {
        context.check()?;
        let prefix = format!("transformer_layes.{layer}.attn");
        let weight = execution_state
            .get(&format!("{prefix}.in_proj_weight"))
            .ok_or_else(|| {
                NativeStyleModelError::MissingState(format!("{prefix}.in_proj_weight"))
            })?;
        let bias = execution_state
            .get(&format!("{prefix}.in_proj_bias"))
            .ok_or_else(|| NativeStyleModelError::MissingState(format!("{prefix}.in_proj_bias")))?;
        let weight_parts = split_three(weight, width, context.cancellation)?;
        let bias_parts = split_three(bias, width, context.cancellation)?;
        require_exact_fused_reconstruction(backend, weight, &weight_parts, context)?;
        require_exact_fused_reconstruction(backend, bias, &bias_parts, context)?;
        layers.push(NativeStyleAttentionProjection {
            query_weight: weight_parts[0].clone(),
            key_weight: weight_parts[1].clone(),
            value_weight: weight_parts[2].clone(),
            query_bias: bias_parts[0].clone(),
            key_bias: bias_parts[1].clone(),
            value_bias: bias_parts[2].clone(),
        });
    }
    Ok(layers.into_boxed_slice())
}

fn split_three(
    tensor: &Tensor,
    width: u64,
    cancellation: &CancellationToken,
) -> Result<[Tensor; 3], NativeStyleModelError> {
    Ok([
        narrow_method_exact_native(tensor, 0, 0, width, cancellation).map_err(canonical)?,
        narrow_method_exact_native(
            tensor,
            0,
            i64::try_from(width).map_err(|_| NativeStyleModelError::ShapeOverflow)?,
            width,
            cancellation,
        )
        .map_err(canonical)?,
        narrow_method_exact_native(
            tensor,
            0,
            i64::try_from(
                width
                    .checked_mul(2)
                    .ok_or(NativeStyleModelError::ShapeOverflow)?,
            )
            .map_err(|_| NativeStyleModelError::ShapeOverflow)?,
            width,
            cancellation,
        )
        .map_err(canonical)?,
    ])
}

fn require_exact_fused_reconstruction(
    backend: &CpuBackend,
    fused: &Tensor,
    parts: &[Tensor; 3],
    context: &ExecutionContext<'_>,
) -> Result<(), NativeStyleModelError> {
    let reconstructed =
        torch_cat_with_context_exact_native(backend, parts, 0, context).map_err(canonical)?;
    if reconstructed.descriptor().shape() != fused.descriptor().shape()
        || reconstructed.contiguous_bytes()? != fused.contiguous_bytes()?
    {
        return Err(NativeStyleModelError::SemanticStateChanged);
    }
    Ok(())
}

fn validate_attention_views(
    resource: &NativeStyleModelResource,
    cancellation: &CancellationToken,
) -> Result<(), NativeStyleModelError> {
    if resource.configuration.architecture != NativeStyleModelArchitecture::StyleAdapter {
        if !resource.style_attention.is_empty() {
            return Err(NativeStyleModelError::SemanticStateChanged);
        }
        return Ok(());
    }
    if resource.style_attention.len() != resource.configuration.layers {
        return Err(NativeStyleModelError::SemanticStateChanged);
    }
    let width = u64_from(resource.configuration.input_width)?;
    for (layer, projection) in resource.style_attention.iter().enumerate() {
        cancellation.check()?;
        let prefix = format!("transformer_layes.{layer}.attn");
        let fused_weight = resource.execution_tensor(&format!("{prefix}.in_proj_weight"))?;
        let fused_bias = resource.execution_tensor(&format!("{prefix}.in_proj_bias"))?;
        for tensor in [
            &projection.query_weight,
            &projection.key_weight,
            &projection.value_weight,
        ] {
            if tensor.storage_id() != fused_weight.storage_id()
                || tensor.descriptor().shape() != [width, width]
            {
                return Err(NativeStyleModelError::SemanticStateChanged);
            }
        }
        for tensor in [
            &projection.query_bias,
            &projection.key_bias,
            &projection.value_bias,
        ] {
            if tensor.storage_id() != fused_bias.storage_id()
                || tensor.descriptor().shape() != [width]
            {
                return Err(NativeStyleModelError::SemanticStateChanged);
            }
        }
    }
    Ok(())
}

fn linear_tensor(
    backend: &CpuBackend,
    input: &[f32],
    input_shape: &[usize],
    weight: &Tensor,
    bias: Option<&Tensor>,
    context: &ExecutionContext<'_>,
) -> Result<Vec<f32>, NativeStyleModelError> {
    let weight_values =
        tensor_to_f32_with_context_exact_native(backend, weight, context).map_err(canonical)?;
    let bias_values = bias
        .map(|bias| tensor_to_f32_with_context_exact_native(backend, bias, context))
        .transpose()
        .map_err(canonical)?;
    let weight_shape = weight
        .descriptor()
        .shape()
        .iter()
        .map(|dimension| usize_from(*dimension))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(linear_with_context_exact_native(
        backend,
        input,
        input_shape,
        &weight_values,
        &weight_shape,
        bias_values.as_deref(),
        DeviceId::CPU,
        context,
    )
    .map_err(canonical)?
    .values)
}

fn quick_gelu(
    backend: &CpuBackend,
    input: &[f32],
    shape: &[usize],
    context: &ExecutionContext<'_>,
) -> Result<Vec<f32>, NativeStyleModelError> {
    let mut scaled = Vec::new();
    scaled
        .try_reserve_exact(input.len())
        .map_err(|_| NativeStyleModelError::Allocation)?;
    for (index, value) in input.iter().copied().enumerate() {
        if index.is_multiple_of(16_384) {
            context.check()?;
        }
        scaled.push(QUICK_GELU_SCALE * value);
    }
    let shape = shape
        .iter()
        .map(|dimension| u64_from(*dimension))
        .collect::<Result<Vec<_>, _>>()?;
    let scaled = tensor_from_f32_with_context_exact_native(
        backend,
        &shape,
        &scaled,
        DType::F32,
        DeviceId::CPU,
        context,
    )
    .map_err(canonical)?;
    let sigmoid =
        sigmoid_with_context_exact_native(backend, &scaled, context).map_err(canonical)?;
    let sigmoid =
        tensor_to_f32_with_context_exact_native(backend, &sigmoid, context).map_err(canonical)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(input.len())
        .map_err(|_| NativeStyleModelError::Allocation)?;
    for (index, (value, sigmoid)) in input.iter().zip(sigmoid).enumerate() {
        if index.is_multiple_of(16_384) {
            context.check()?;
        }
        output.push(*value * sigmoid);
    }
    Ok(output)
}

fn add_in_place(
    left: &mut [f32],
    right: &[f32],
    cancellation: &CancellationToken,
) -> Result<(), NativeStyleModelError> {
    if left.len() != right.len() {
        return Err(NativeStyleModelError::ShapeOverflow);
    }
    for (index, (left, right)) in left.iter_mut().zip(right).enumerate() {
        if index.is_multiple_of(16_384) {
            cancellation.check()?;
        }
        *left += *right;
    }
    cancellation.check()?;
    Ok(())
}

fn construction_memory_preflight(
    artifact: &str,
    artifact_capacity: usize,
    ordered_state: &[(String, Tensor)],
    specifications: &[StateSpecification],
    configuration: NativeStyleModelConfiguration,
) -> Result<u64, NativeStyleModelError> {
    let source_bytes = ordered_state.iter().try_fold(0_u64, |total, (_, tensor)| {
        total
            .checked_add(tensor.storage_byte_len())
            .ok_or(NativeStyleModelError::ShapeOverflow)
    })?;
    let projected_bytes = specifications
        .iter()
        .try_fold(0_u64, |total, specification| {
            total
                .checked_add(checked_tensor_bytes(&specification.shape, DType::F32)?)
                .ok_or(NativeStyleModelError::ShapeOverflow)
        })?;
    let maximum_projected = specifications
        .iter()
        .map(|specification| checked_tensor_bytes(&specification.shape, DType::F32))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    let maximum_fused = if configuration.architecture == NativeStyleModelArchitecture::StyleAdapter
    {
        u64_from(configuration.input_width)?
            .checked_mul(u64_from(configuration.input_width)?)
            .and_then(|value| value.checked_mul(3))
            .and_then(|value| value.checked_mul(4))
            .ok_or(NativeStyleModelError::ShapeOverflow)?
    } else {
        0
    };
    let owned = construction_owned_estimate(
        artifact,
        artifact_capacity,
        ordered_state,
        specifications,
        configuration,
    )?;
    source_bytes
        .checked_add(projected_bytes)
        .and_then(|value| value.checked_add(maximum_projected.max(maximum_fused)))
        .and_then(|value| value.checked_add(owned))
        .ok_or(NativeStyleModelError::ShapeOverflow)
}

fn construction_owned_estimate(
    artifact: &str,
    artifact_capacity: usize,
    ordered_state: &[(String, Tensor)],
    specifications: &[StateSpecification],
    configuration: NativeStyleModelConfiguration,
) -> Result<u64, NativeStyleModelError> {
    let base = u64::try_from(mem::size_of::<NativeStyleModelResource>())
        .map_err(|_| NativeStyleModelError::ShapeOverflow)?;
    let source_key_bytes = ordered_state.iter().try_fold(0_u64, |total, (key, _)| {
        total
            .checked_add(u64_from(key.capacity())?)
            .ok_or(NativeStyleModelError::ShapeOverflow)
    })?;
    let projected_key_bytes = specifications
        .iter()
        .try_fold(0_u64, |total, specification| {
            total
                .checked_add(u64_from(specification.key.capacity())?)
                .and_then(|value| {
                    value.checked_add(
                        u64_from(specification.shape.capacity())
                            .ok()?
                            .checked_mul(8)?,
                    )
                })
                .ok_or(NativeStyleModelError::ShapeOverflow)
        })?;
    let temporary_container_bytes = u64_from(ordered_state.len())?
        .checked_mul(
            u64::try_from(mem::size_of::<(String, Tensor)>())
                .map_err(|_| NativeStyleModelError::ShapeOverflow)?,
        )
        .and_then(|value| {
            value.checked_add(
                u64_from(specifications.len())
                    .ok()?
                    .checked_mul(u64::try_from(mem::size_of::<StateSpecification>()).ok()?)?,
            )
        })
        .ok_or(NativeStyleModelError::ShapeOverflow)?;
    base.checked_add(u64_from(artifact_capacity.max(artifact.len()))?)
        .and_then(|value| value.checked_add(source_key_bytes))
        .and_then(|value| value.checked_add(projected_key_bytes))
        .and_then(|value| value.checked_add(temporary_container_bytes))
        .and_then(|value| value.checked_add(map_node_estimate(specifications.len()).ok()?))
        .ok_or(NativeStyleModelError::ShapeOverflow)?
        .checked_add(
            u64_from(configuration.layers)?
                .checked_mul(
                    u64::try_from(mem::size_of::<NativeStyleAttentionProjection>())
                        .map_err(|_| NativeStyleModelError::ShapeOverflow)?,
                )
                .ok_or(NativeStyleModelError::ShapeOverflow)?,
        )
        .ok_or(NativeStyleModelError::ShapeOverflow)
}

fn preflight_invocation_memory(
    resource: &NativeStyleModelResource,
    batch: usize,
    tokens: usize,
) -> Result<(), NativeStyleModelError> {
    let scalar_bytes = 4_u64;
    let input_elements = u64_from(batch)?
        .checked_mul(u64_from(tokens)?)
        .and_then(|value| value.checked_mul(u64_from(resource.configuration.input_width).ok()?))
        .ok_or(NativeStyleModelError::ShapeOverflow)?;
    let input_bytes = input_elements
        .checked_mul(scalar_bytes)
        .ok_or(NativeStyleModelError::ShapeOverflow)?;
    let phase_bytes = match resource.configuration.architecture {
        NativeStyleModelArchitecture::StyleAdapter => {
            let sequence = tokens
                .checked_add(resource.configuration.output_tokens)
                .ok_or(NativeStyleModelError::ShapeOverflow)?;
            let hidden_elements = u64_from(batch)?
                .checked_mul(u64_from(sequence)?)
                .and_then(|value| {
                    value.checked_mul(u64_from(resource.configuration.input_width).ok()?)
                })
                .ok_or(NativeStyleModelError::ShapeOverflow)?;
            let hidden_bytes = hidden_elements
                .checked_mul(scalar_bytes)
                .ok_or(NativeStyleModelError::ShapeOverflow)?;
            let score_bytes = u64_from(batch)?
                .checked_mul(u64_from(resource.configuration.heads)?)
                .and_then(|value| value.checked_mul(u64_from(sequence).ok()?))
                .and_then(|value| value.checked_mul(u64_from(sequence).ok()?))
                .and_then(|value| value.checked_mul(scalar_bytes))
                .ok_or(NativeStyleModelError::ShapeOverflow)?;
            hidden_bytes
                .checked_mul(24)
                .and_then(|value| value.checked_add(score_bytes.checked_mul(2)?))
                .and_then(|value| value.checked_add(input_bytes))
                .ok_or(NativeStyleModelError::ShapeOverflow)?
        }
        NativeStyleModelArchitecture::FluxRedux => {
            let hidden_bytes = u64_from(batch)?
                .checked_mul(u64_from(tokens)?)
                .and_then(|value| {
                    value.checked_mul(u64_from(resource.configuration.hidden_width).ok()?)
                })
                .and_then(|value| value.checked_mul(scalar_bytes))
                .ok_or(NativeStyleModelError::ShapeOverflow)?;
            let output_bytes = u64_from(batch)?
                .checked_mul(u64_from(tokens)?)
                .and_then(|value| {
                    value.checked_mul(u64_from(resource.configuration.output_width).ok()?)
                })
                .and_then(|value| value.checked_mul(scalar_bytes))
                .ok_or(NativeStyleModelError::ShapeOverflow)?;
            input_bytes
                .checked_mul(2)
                .and_then(|value| value.checked_add(hidden_bytes.checked_mul(3)?))
                .and_then(|value| value.checked_add(output_bytes))
                .ok_or(NativeStyleModelError::ShapeOverflow)?
        }
    };
    let decoded_weight_bytes = resource
        .execution_state
        .values()
        .map(Tensor::storage_byte_len)
        .max()
        .unwrap_or(0)
        .checked_mul(2)
        .ok_or(NativeStyleModelError::ShapeOverflow)?;
    let required = resource
        .resident_bytes
        .checked_add(phase_bytes)
        .and_then(|value| value.checked_add(decoded_weight_bytes))
        .ok_or(NativeStyleModelError::ShapeOverflow)?;
    if required > resource.memory_budget_bytes {
        return Err(NativeStyleModelError::OutOfMemory {
            required,
            budget: resource.memory_budget_bytes,
        });
    }
    Ok(())
}

fn semantic_digest(
    configuration: NativeStyleModelConfiguration,
    artifact: &str,
    source_dtype: DType,
    source_state: &BTreeMap<String, Tensor>,
    execution_state: &BTreeMap<String, Tensor>,
    specifications: &[StateSpecification],
    cancellation: &CancellationToken,
) -> Result<String, NativeStyleModelError> {
    let mut hasher = Sha256::new();
    hasher.update(b"zed.comfy.style-model-resource.v1\0");
    hasher.update([match configuration.architecture {
        NativeStyleModelArchitecture::StyleAdapter => 1,
        NativeStyleModelArchitecture::FluxRedux => 2,
    }]);
    for value in [
        configuration.input_width,
        configuration.hidden_width,
        configuration.output_width,
        configuration.heads,
        configuration.layers,
        configuration.output_tokens,
    ] {
        hasher.update(u64_from(value)?.to_le_bytes());
    }
    hasher.update([u8::from(configuration.source_exact)]);
    hasher.update(artifact.as_bytes());
    hasher.update(source_dtype.catalog_name().as_bytes());
    for (index, specification) in specifications.iter().enumerate() {
        if index.is_multiple_of(8) {
            cancellation.check()?;
        }
        hash_tensor(
            &mut hasher,
            &specification.key,
            source_state
                .get(&specification.key)
                .ok_or_else(|| NativeStyleModelError::MissingState(specification.key.clone()))?,
            cancellation,
        )?;
        hash_tensor(
            &mut hasher,
            &specification.key,
            execution_state
                .get(&specification.key)
                .ok_or_else(|| NativeStyleModelError::MissingState(specification.key.clone()))?,
            cancellation,
        )?;
    }
    cancellation.check()?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_tensor(
    hasher: &mut Sha256,
    key: &str,
    tensor: &Tensor,
    cancellation: &CancellationToken,
) -> Result<(), NativeStyleModelError> {
    hasher.update(u64_from(key.len())?.to_le_bytes());
    hasher.update(key.as_bytes());
    hasher.update(tensor.descriptor().dtype().catalog_name().as_bytes());
    for dimension in tensor.descriptor().shape() {
        hasher.update(dimension.to_le_bytes());
    }
    for chunk in tensor.contiguous_bytes()?.chunks(DIGEST_CHUNK_BYTES) {
        cancellation.check()?;
        hasher.update(chunk);
    }
    Ok(())
}

fn validate_finite_tensor(
    key: &str,
    tensor: &Tensor,
    cancellation: &CancellationToken,
) -> Result<(), NativeStyleModelError> {
    let bytes = tensor.contiguous_bytes()?;
    if !bytes.len().is_multiple_of(mem::size_of::<f32>()) {
        return Err(NativeStyleModelError::InvalidCheckpoint(format!(
            "state {key} has invalid F32 storage"
        )));
    }
    for (index, chunk) in bytes.chunks_exact(mem::size_of::<f32>()).enumerate() {
        if index.is_multiple_of(16_384) {
            cancellation.check()?;
        }
        let encoded: [u8; 4] = chunk
            .try_into()
            .map_err(|_| NativeStyleModelError::SemanticStateChanged)?;
        if !f32::from_ne_bytes(encoded).is_finite() {
            return Err(NativeStyleModelError::InvalidCheckpoint(format!(
                "state {key} contains a non-finite value"
            )));
        }
    }
    Ok(())
}

fn resident_tensor_bytes<'a>(
    maps: impl IntoIterator<Item = &'a BTreeMap<String, Tensor>>,
    cancellation: &CancellationToken,
) -> Result<u64, NativeStyleModelError> {
    resident_tensor_allocations(maps, cancellation)?
        .into_iter()
        .try_fold(0_u64, |total, (_, bytes)| {
            total
                .checked_add(bytes)
                .ok_or(NativeStyleModelError::ShapeOverflow)
        })
}

fn resident_tensor_allocations<'a>(
    maps: impl IntoIterator<Item = &'a BTreeMap<String, Tensor>>,
    cancellation: &CancellationToken,
) -> Result<Vec<(StorageId, u64)>, NativeStyleModelError> {
    let mut allocations = Vec::new();
    for map in maps {
        for (index, tensor) in map.values().enumerate() {
            if index.is_multiple_of(16) {
                cancellation.check()?;
            }
            let storage_id = tensor.storage_id();
            let bytes = tensor.storage_byte_len();
            if let Some((_, existing)) = allocations
                .iter()
                .find(|(existing_id, _)| *existing_id == storage_id)
            {
                if *existing != bytes {
                    return Err(NativeStyleModelError::SemanticStateChanged);
                }
            } else {
                allocations.push((storage_id, bytes));
            }
        }
    }
    Ok(allocations)
}

fn resident_owned_bytes(
    artifact: &str,
    artifact_capacity: usize,
    digest: &str,
    digest_capacity: usize,
    source_state: &BTreeMap<String, Tensor>,
    execution_state: &BTreeMap<String, Tensor>,
    attention_layers: usize,
) -> Result<u64, NativeStyleModelError> {
    let base = u64::try_from(mem::size_of::<NativeStyleModelResource>())
        .map_err(|_| NativeStyleModelError::ShapeOverflow)?;
    let keys =
        source_state
            .keys()
            .chain(execution_state.keys())
            .try_fold(0_u64, |total, key| {
                total
                    .checked_add(u64_from(key.capacity())?)
                    .ok_or(NativeStyleModelError::ShapeOverflow)
            })?;
    base.checked_add(u64_from(artifact_capacity.max(artifact.len()))?)
        .and_then(|value| value.checked_add(u64_from(digest_capacity.max(digest.len())).ok()?))
        .and_then(|value| value.checked_add(keys))
        .and_then(|value| value.checked_add(map_node_estimate(source_state.len()).ok()?))
        .and_then(|value| {
            value.checked_add(u64_from(attention_layers).ok()?.checked_mul(
                u64::try_from(mem::size_of::<NativeStyleAttentionProjection>()).ok()?,
            )?)
        })
        .ok_or(NativeStyleModelError::ShapeOverflow)
}

fn map_node_estimate(entries: usize) -> Result<u64, NativeStyleModelError> {
    let per_entry = u64::try_from(mem::size_of::<(String, Tensor)>())
        .map_err(|_| NativeStyleModelError::ShapeOverflow)?
        .checked_add(64)
        .ok_or(NativeStyleModelError::ShapeOverflow)?;
    u64_from(entries)?
        .checked_mul(per_entry)
        .and_then(|value| value.checked_mul(2))
        .ok_or(NativeStyleModelError::ShapeOverflow)
}

fn checked_tensor_bytes(shape: &[u64], dtype: DType) -> Result<u64, NativeStyleModelError> {
    shape
        .iter()
        .try_fold(1_u64, |count, dimension| {
            count
                .checked_mul(*dimension)
                .ok_or(NativeStyleModelError::ShapeOverflow)
        })?
        .checked_mul(dtype.byte_width())
        .ok_or(NativeStyleModelError::ShapeOverflow)
}

fn validate_sha256(value: &str) -> Result<(), NativeStyleModelError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(NativeStyleModelError::InvalidCheckpoint(
            "artifact identity must be canonical lowercase SHA-256".to_owned(),
        ));
    }
    Ok(())
}

fn attention_error(error: AttentionError) -> NativeStyleModelError {
    match error {
        AttentionError::Cancelled | AttentionError::Tensor(TensorError::Cancelled) => {
            NativeStyleModelError::Cancelled
        }
        error => canonical(error),
    }
}

fn canonical(error: impl std::error::Error + 'static) -> NativeStyleModelError {
    let mut source: &(dyn std::error::Error + 'static) = &error;
    loop {
        if is_cancellation_error(source) {
            return NativeStyleModelError::Cancelled;
        }
        let Some(next) = source.source() else {
            break;
        };
        source = next;
    }
    NativeStyleModelError::Canonical(error.to_string())
}

fn is_cancellation_error(error: &(dyn std::error::Error + 'static)) -> bool {
    error
        .downcast_ref::<TensorError>()
        .is_some_and(|error| matches!(error, TensorError::Cancelled))
        || error
            .downcast_ref::<comfy_types::CancellationError>()
            .is_some()
        || error
            .downcast_ref::<OperatorIndirectionError>()
            .is_some_and(|error| matches!(error, OperatorIndirectionError::Cancelled))
        || error
            .downcast_ref::<ElementwiseRuntimePartThreeError>()
            .is_some_and(|error| matches!(error, ElementwiseRuntimePartThreeError::Cancelled))
        || error
            .downcast_ref::<ElementwiseRuntimePartNineError>()
            .is_some_and(|error| matches!(error, ElementwiseRuntimePartNineError::Cancelled))
        || error
            .downcast_ref::<IndexingMaskingPartOneError>()
            .is_some_and(|error| matches!(error, IndexingMaskingPartOneError::Cancelled))
        || error
            .downcast_ref::<LinearAlgebraPartOneError>()
            .is_some_and(|error| matches!(error, LinearAlgebraPartOneError::Cancelled))
        || error
            .downcast_ref::<NeuralNetworkFunctionalError>()
            .is_some_and(|error| matches!(error, NeuralNetworkFunctionalError::Cancelled))
        || error
            .downcast_ref::<ShapeLayoutTransformPartTwoError>()
            .is_some_and(|error| matches!(error, ShapeLayoutTransformPartTwoError::Cancelled))
        || error
            .downcast_ref::<FunctionalError>()
            .is_some_and(|error| matches!(error, FunctionalError::Cancelled))
        || error
            .downcast_ref::<AttentionError>()
            .is_some_and(|error| matches!(error, AttentionError::Cancelled))
}

fn u64_from(value: usize) -> Result<u64, NativeStyleModelError> {
    u64::try_from(value).map_err(|_| NativeStyleModelError::ShapeOverflow)
}

fn usize_from(value: u64) -> Result<usize, NativeStyleModelError> {
    usize::try_from(value).map_err(|_| NativeStyleModelError::ShapeOverflow)
}

fn i64_from(value: usize) -> Result<i64, NativeStyleModelError> {
    i64::try_from(value).map_err(|_| NativeStyleModelError::ShapeOverflow)
}

#[derive(Clone, Debug)]
pub enum NativePhotoMakerCheckpointEntry {
    Tensor {
        key: String,
        tensor: Tensor,
    },
    Mapping {
        key: String,
        ordered_state: Vec<(String, Tensor)>,
    },
}

#[derive(Clone, Debug)]
pub struct NativePhotoMakerCheckpoint {
    pub artifact_sha256: String,
    pub ordered_entries: Vec<NativePhotoMakerCheckpointEntry>,
    pub memory_budget_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativePhotoMakerWrapper {
    Flat,
    IdEncoder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NativePhotoMakerConfiguration {
    hidden: usize,
    intermediate: usize,
    heads: usize,
    layers: usize,
    image_size: usize,
    patch_size: usize,
    projection: usize,
    extra_projection: usize,
    prompt_width: usize,
    source_exact: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativePhotoMakerPhase {
    Entry,
    Wrapper,
    Schema,
    SourceValidation,
    Projection,
    ClipVision,
    SemanticDigest,
    Validation,
    Return,
    InvocationAdmission,
    ClipForward,
    ExtraProjection,
    MaskIndices,
    Gather,
    Mlp1LayerNorm,
    Mlp1Linear1,
    Mlp1Gelu,
    Mlp1Linear2,
    PromptResidual,
    Mlp2LayerNorm,
    Mlp2Linear1,
    Mlp2Gelu,
    Mlp2Linear2,
    Mlp2Residual,
    FinalLayerNorm,
    Scatter,
    InvocationReturn,
}

#[derive(Debug)]
pub struct NativePhotoMakerResource {
    configuration: NativePhotoMakerConfiguration,
    wrapper: NativePhotoMakerWrapper,
    artifact_sha256: String,
    source_state: BTreeMap<String, Tensor>,
    execution_state: BTreeMap<String, Tensor>,
    clip_vision: NativeClipVision,
    source_dtype: DType,
    stream: StreamId,
    memory_budget_bytes: u64,
    resident_bytes: u64,
    semantic_digest_sha256: String,
}

#[derive(Debug, Error)]
pub enum NativePhotoMakerError {
    #[error("PhotoMaker execution was cancelled")]
    Cancelled,
    #[error("PhotoMaker checkpoint is invalid: {0}")]
    InvalidCheckpoint(String),
    #[error("PhotoMaker state is missing key {0}")]
    MissingState(String),
    #[error("PhotoMaker state is unexpected: {0}")]
    UnexpectedState(String),
    #[error("PhotoMaker state {key} expected {expected:?}, got {actual:?} {actual_dtype:?}")]
    StateShape {
        key: String,
        expected: Vec<u64>,
        actual: Vec<u64>,
        actual_dtype: DType,
    },
    #[error("PhotoMaker input is invalid: {0}")]
    InvalidInput(String),
    #[error("PhotoMaker retained semantic state changed")]
    SemanticStateChanged,
    #[error("PhotoMaker shape arithmetic overflowed")]
    ShapeOverflow,
    #[error("PhotoMaker allocation failed")]
    Allocation,
    #[error("PhotoMaker memory requirement {required} exceeds budget {budget}")]
    OutOfMemory { required: u64, budget: u64 },
    #[error("PhotoMaker canonical owner failed: {0}")]
    Canonical(String),
    #[error(transparent)]
    ClipVision(ClipVisionError),
    #[error(transparent)]
    Tensor(TensorError),
}

impl From<comfy_types::CancellationError> for NativePhotoMakerError {
    fn from(_: comfy_types::CancellationError) -> Self {
        Self::Cancelled
    }
}

impl From<TensorError> for NativePhotoMakerError {
    fn from(error: TensorError) -> Self {
        match error {
            TensorError::Cancelled => Self::Cancelled,
            error => Self::Tensor(error),
        }
    }
}

impl From<ClipVisionError> for NativePhotoMakerError {
    fn from(error: ClipVisionError) -> Self {
        match error {
            ClipVisionError::Cancelled | ClipVisionError::Tensor(TensorError::Cancelled) => {
                Self::Cancelled
            }
            error => Self::ClipVision(error),
        }
    }
}

impl NativePhotoMakerResource {
    pub fn from_checkpoint(
        backend: &CpuBackend,
        checkpoint: NativePhotoMakerCheckpoint,
        context: &ExecutionContext<'_>,
    ) -> Result<Self, NativePhotoMakerError> {
        Self::checked(
            backend,
            checkpoint,
            photo_source_configuration(),
            context,
            &mut |_, _| {},
        )
    }

    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn from_reduced_fixture(
        backend: &CpuBackend,
        checkpoint: NativePhotoMakerCheckpoint,
        context: &ExecutionContext<'_>,
    ) -> Result<Self, NativePhotoMakerError> {
        Self::checked(
            backend,
            checkpoint,
            photo_reduced_configuration(),
            context,
            &mut |_, _| {},
        )
    }

    fn checked(
        backend: &CpuBackend,
        checkpoint: NativePhotoMakerCheckpoint,
        configuration: NativePhotoMakerConfiguration,
        context: &ExecutionContext<'_>,
        phase_hook: &mut impl FnMut(NativePhotoMakerPhase, &CancellationToken),
    ) -> Result<Self, NativePhotoMakerError> {
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Entry,
            phase_hook,
        )?;
        photo_validate_sha256(&checkpoint.artifact_sha256)?;
        if checkpoint.memory_budget_bytes == 0 || checkpoint.ordered_entries.is_empty() {
            return Err(NativePhotoMakerError::InvalidCheckpoint(
                "state cardinality or memory budget is invalid".to_owned(),
            ));
        }
        photo_raw_checkpoint_preflight(&checkpoint)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Wrapper,
            phase_hook,
        )?;
        let (wrapper, ordered_state) = photo_normalize_checkpoint(checkpoint.ordered_entries)?;
        let specifications = photo_state_manifest(configuration)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Schema,
            phase_hook,
        )?;
        photo_validate_ordered_keys(&ordered_state, &specifications)?;
        let source_dtype = photo_validate_source_state(
            &ordered_state,
            &specifications,
            context.stream,
            context.cancellation,
        )?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::SourceValidation,
            phase_hook,
        )?;
        let construction_peak = photo_construction_peak(
            &checkpoint.artifact_sha256,
            checkpoint.artifact_sha256.capacity(),
            &ordered_state,
            &specifications,
        )?;
        if construction_peak > checkpoint.memory_budget_bytes {
            return Err(NativePhotoMakerError::OutOfMemory {
                required: construction_peak,
                budget: checkpoint.memory_budget_bytes,
            });
        }

        let source_state = ordered_state.into_iter().collect::<BTreeMap<_, _>>();
        let mut projected_state = BTreeMap::new();
        for (index, specification) in specifications.iter().enumerate() {
            if index.is_multiple_of(8) {
                photo_checkpoint(
                    context.cancellation,
                    NativePhotoMakerPhase::Projection,
                    phase_hook,
                )?;
            }
            let source = source_state
                .get(&specification.key)
                .ok_or_else(|| NativePhotoMakerError::MissingState(specification.key.clone()))?;
            let projected = cast_to_with_context_exact_native(
                backend,
                source,
                DType::F32,
                DeviceId::CPU,
                false,
                true,
                context,
            )
            .map_err(photo_canonical)?;
            photo_validate_finite_tensor(&specification.key, &projected, context.cancellation)?;
            projected_state.insert(specification.key.clone(), projected);
        }
        photo_validate_distinct_storage(&source_state, &projected_state)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::ClipVision,
            phase_hook,
        )?;
        let clip_vision = NativeClipVision::new_with_cancellation(
            photo_clip_configuration(configuration),
            photo_clip_weights(&projected_state, configuration)?,
            context.cancellation,
        )?;
        photo_require_clip_storage_from_execution(&clip_vision, &projected_state)?;
        let mut execution_state = BTreeMap::new();
        for specification in specifications.iter().skip(PHOTOMAKER_CLIP_STATE_COUNT) {
            execution_state.insert(
                specification.key.clone(),
                projected_state.remove(&specification.key).ok_or_else(|| {
                    NativePhotoMakerError::MissingState(specification.key.clone())
                })?,
            );
        }
        photo_validate_resource_storage(&source_state, &execution_state, &clip_vision)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::SemanticDigest,
            phase_hook,
        )?;
        let semantic_digest_sha256 = photo_semantic_digest(
            configuration,
            wrapper,
            &checkpoint.artifact_sha256,
            source_dtype,
            &source_state,
            &execution_state,
            clip_vision.semantic_digest_sha256(),
            &specifications,
            context.cancellation,
        )?;
        let owned_bytes = photo_resident_owned_bytes(
            &checkpoint.artifact_sha256,
            checkpoint.artifact_sha256.capacity(),
            &semantic_digest_sha256,
            semantic_digest_sha256.capacity(),
            &source_state,
            &execution_state,
            &clip_vision,
        )?;
        let resident_bytes = photo_resource_tensor_allocations(
            &source_state,
            &execution_state,
            &clip_vision,
            context.cancellation,
        )?
        .into_iter()
        .try_fold(owned_bytes, |total, (_, bytes)| {
            total
                .checked_add(bytes)
                .ok_or(NativePhotoMakerError::ShapeOverflow)
        })?;
        if resident_bytes > checkpoint.memory_budget_bytes {
            return Err(NativePhotoMakerError::OutOfMemory {
                required: resident_bytes,
                budget: checkpoint.memory_budget_bytes,
            });
        }
        let resource = Self {
            configuration,
            wrapper,
            artifact_sha256: checkpoint.artifact_sha256,
            source_state,
            execution_state,
            clip_vision,
            source_dtype,
            stream: context.stream,
            memory_budget_bytes: checkpoint.memory_budget_bytes,
            resident_bytes,
            semantic_digest_sha256,
        };
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Validation,
            phase_hook,
        )?;
        resource.validate(context.cancellation)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Return,
            phase_hook,
        )?;
        Ok(resource)
    }

    pub const fn identifier(&self) -> &'static str {
        "photomaker-id-encoder"
    }

    pub fn artifact_sha256(&self) -> &str {
        &self.artifact_sha256
    }

    pub fn semantic_digest_sha256(&self) -> &str {
        &self.semantic_digest_sha256
    }

    pub const fn source_dtype(&self) -> DType {
        self.source_dtype
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub const fn is_source_exact_profile(&self) -> bool {
        self.configuration.source_exact
    }

    pub fn resident_owned_bytes(&self) -> Result<u64, NativePhotoMakerError> {
        photo_resident_owned_bytes(
            &self.artifact_sha256,
            self.artifact_sha256.capacity(),
            &self.semantic_digest_sha256,
            self.semantic_digest_sha256.capacity(),
            &self.source_state,
            &self.execution_state,
            &self.clip_vision,
        )
    }

    pub fn resident_tensor_allocations(
        &self,
    ) -> Result<Vec<(StorageId, u64)>, NativePhotoMakerError> {
        photo_resource_tensor_allocations(
            &self.source_state,
            &self.execution_state,
            &self.clip_vision,
            &CancellationToken::default(),
        )
    }

    pub fn reconstruct_checkpoint(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<NativePhotoMakerCheckpoint, NativePhotoMakerError> {
        self.validate(cancellation)?;
        self.clip_vision.reconstruct(cancellation)?;
        let specifications = photo_state_manifest(self.configuration)?;
        let mut ordered_state = Vec::new();
        ordered_state
            .try_reserve_exact(specifications.len())
            .map_err(|_| NativePhotoMakerError::Allocation)?;
        for (index, specification) in specifications.iter().enumerate() {
            if index.is_multiple_of(8) {
                cancellation.check()?;
            }
            ordered_state.push((
                specification.key.clone(),
                self.source_state
                    .get(&specification.key)
                    .ok_or_else(|| NativePhotoMakerError::MissingState(specification.key.clone()))?
                    .clone(),
            ));
        }
        let ordered_entries = match self.wrapper {
            NativePhotoMakerWrapper::Flat => ordered_state
                .into_iter()
                .map(|(key, tensor)| NativePhotoMakerCheckpointEntry::Tensor { key, tensor })
                .collect(),
            NativePhotoMakerWrapper::IdEncoder => {
                vec![NativePhotoMakerCheckpointEntry::Mapping {
                    key: "id_encoder".to_owned(),
                    ordered_state,
                }]
            }
        };
        cancellation.check()?;
        Ok(NativePhotoMakerCheckpoint {
            artifact_sha256: self.artifact_sha256.clone(),
            ordered_entries,
            memory_budget_bytes: self.memory_budget_bytes,
        })
    }

    pub fn validate(&self, cancellation: &CancellationToken) -> Result<(), NativePhotoMakerError> {
        cancellation.check()?;
        let specifications = photo_state_manifest(self.configuration)?;
        let source_order = specifications
            .iter()
            .map(|specification| {
                self.source_state
                    .get(&specification.key)
                    .cloned()
                    .map(|tensor| (specification.key.clone(), tensor))
                    .ok_or_else(|| NativePhotoMakerError::MissingState(specification.key.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let source_dtype =
            photo_validate_source_state(&source_order, &specifications, self.stream, cancellation)?;
        if source_dtype != self.source_dtype
            || self.source_state.len() != PHOTOMAKER_STATE_COUNT
            || self.execution_state.len() != PHOTOMAKER_STATE_COUNT - PHOTOMAKER_CLIP_STATE_COUNT
        {
            return Err(NativePhotoMakerError::SemanticStateChanged);
        }
        for specification in specifications.iter().skip(PHOTOMAKER_CLIP_STATE_COUNT) {
            let tensor = self
                .execution_state
                .get(&specification.key)
                .ok_or_else(|| NativePhotoMakerError::MissingState(specification.key.clone()))?;
            if tensor.descriptor().shape() != specification.shape
                || tensor.descriptor().dtype() != DType::F32
                || tensor.descriptor().device() != DeviceId::CPU
                || tensor.descriptor().stream() != self.stream
                || !tensor.descriptor().is_contiguous()?
            {
                return Err(NativePhotoMakerError::SemanticStateChanged);
            }
            photo_validate_finite_tensor(&specification.key, tensor, cancellation)?;
        }
        photo_validate_resource_storage(
            &self.source_state,
            &self.execution_state,
            &self.clip_vision,
        )?;
        self.clip_vision.validate(cancellation)?;
        let reconstructed_clip = self.clip_vision.reconstruct(cancellation)?;
        if reconstructed_clip.semantic_identity() != self.clip_vision.semantic_identity()
            || reconstructed_clip.resident_parts()? != self.clip_vision.resident_parts()?
        {
            return Err(NativePhotoMakerError::SemanticStateChanged);
        }
        let digest = photo_semantic_digest(
            self.configuration,
            self.wrapper,
            &self.artifact_sha256,
            self.source_dtype,
            &self.source_state,
            &self.execution_state,
            self.clip_vision.semantic_digest_sha256(),
            &specifications,
            cancellation,
        )?;
        let resident = self.resident_tensor_allocations()?.into_iter().try_fold(
            self.resident_owned_bytes()?,
            |total, (_, bytes)| {
                total
                    .checked_add(bytes)
                    .ok_or(NativePhotoMakerError::ShapeOverflow)
            },
        )?;
        if digest != self.semantic_digest_sha256 || resident != self.resident_bytes {
            return Err(NativePhotoMakerError::SemanticStateChanged);
        }
        cancellation.check()?;
        Ok(())
    }

    pub fn fuse_conditioning(
        &self,
        backend: &CpuBackend,
        image: &Tensor,
        prompt: &Tensor,
        class_tokens_mask: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, NativePhotoMakerError> {
        self.fuse_conditioning_with_phase_hook(
            backend,
            image,
            prompt,
            class_tokens_mask,
            context,
            &mut |_, _| {},
        )
    }

    fn fuse_conditioning_with_phase_hook(
        &self,
        backend: &CpuBackend,
        image: &Tensor,
        prompt: &Tensor,
        class_tokens_mask: &Tensor,
        context: &ExecutionContext<'_>,
        phase_hook: &mut impl FnMut(NativePhotoMakerPhase, &CancellationToken),
    ) -> Result<Tensor, NativePhotoMakerError> {
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::InvocationAdmission,
            phase_hook,
        )?;
        self.validate(context.cancellation)?;
        photo_validate_invocation(
            self.configuration,
            image,
            prompt,
            class_tokens_mask,
            context,
        )?;
        let image_shape = image.descriptor().shape();
        let prompt_shape = prompt.descriptor().shape();
        let image_count = pm_usize(image_shape[1])?;
        let sequence = pm_usize(prompt_shape[1])?;
        photo_invocation_peak(self, image_count, sequence)?;
        let selected = photo_mask_true_count(backend, class_tokens_mask, context)?;
        if selected != image_count {
            return Err(NativePhotoMakerError::InvalidInput(format!(
                "mask contains {selected} true entries for {image_count} images"
            )));
        }

        let flattened_image = torch_reshape_with_context_exact_native(
            backend,
            image,
            &[
                pm_i64(image_shape[1])?,
                3,
                pm_i64(image_shape[3])?,
                pm_i64(image_shape[4])?,
            ],
            context,
        )
        .map_err(photo_canonical)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::ClipForward,
            phase_hook,
        )?;
        let mut clip_vision = self.clip_vision.clone();
        let checked = clip_vision.forward_checked(
            backend,
            &flattened_image,
            ClipVisionIntermediate::None,
            context,
        )?;
        let first_projection = checked.output.image_embeds;
        let pooled_hidden = checked.pooled_hidden;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::ExtraProjection,
            phase_hook,
        )?;
        let pooled_values =
            tensor_to_f32_with_context_exact_native(backend, &pooled_hidden, context)
                .map_err(photo_canonical)?;
        let second_projection = photo_linear(
            backend,
            &pooled_values,
            &[image_count, self.configuration.hidden],
            self.photo_tensor("visual_projection_2.weight")?,
            None,
            context,
        )?;
        let second_projection = tensor_from_f32_with_context_exact_native(
            backend,
            &[
                pm_u64(image_count)?,
                pm_u64(self.configuration.extra_projection)?,
            ],
            &second_projection,
            DType::F32,
            DeviceId::CPU,
            context,
        )
        .map_err(photo_canonical)?;
        let identity_embeddings = torch_cat_with_context_exact_native(
            backend,
            &[first_projection, second_projection],
            1,
            context,
        )
        .map_err(photo_canonical)?;

        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::MaskIndices,
            phase_hook,
        )?;
        let positions = photo_mask_positions(backend, class_tokens_mask, context)?;
        if positions.len() != image_count {
            return Err(NativePhotoMakerError::InvalidInput(format!(
                "mask contains {} true entries for {image_count} images",
                positions.len()
            )));
        }
        let index = photo_row_index_tensor(
            backend,
            &positions,
            self.configuration.prompt_width,
            context,
        )?;
        let flattened_prompt = torch_reshape_with_context_exact_native(
            backend,
            prompt,
            &[pm_i64(sequence)?, pm_i64(self.configuration.prompt_width)?],
            context,
        )
        .map_err(photo_canonical)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Gather,
            phase_hook,
        )?;
        let prompt_rows =
            gather_method_with_context_exact_native(backend, &flattened_prompt, 0, &index, context)
                .map_err(photo_canonical)?;
        let identity_values =
            tensor_to_f32_with_context_exact_native(backend, &identity_embeddings, context)
                .map_err(photo_canonical)?;
        let prompt_values = tensor_to_f32_with_context_exact_native(backend, &prompt_rows, context)
            .map_err(photo_canonical)?;
        let mut stacked = Vec::new();
        stacked
            .try_reserve_exact(
                image_count
                    .checked_mul(self.configuration.prompt_width)
                    .and_then(|value| value.checked_mul(2))
                    .ok_or(NativePhotoMakerError::ShapeOverflow)?,
            )
            .map_err(|_| NativePhotoMakerError::Allocation)?;
        for row in 0..image_count {
            context.check()?;
            let start = row
                .checked_mul(self.configuration.prompt_width)
                .ok_or(NativePhotoMakerError::ShapeOverflow)?;
            let end = start
                .checked_add(self.configuration.prompt_width)
                .ok_or(NativePhotoMakerError::ShapeOverflow)?;
            stacked.extend_from_slice(
                prompt_values
                    .get(start..end)
                    .ok_or(NativePhotoMakerError::ShapeOverflow)?,
            );
            stacked.extend_from_slice(
                identity_values
                    .get(start..end)
                    .ok_or(NativePhotoMakerError::ShapeOverflow)?,
            );
        }

        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Mlp1LayerNorm,
            phase_hook,
        )?;
        let stacked_width = self
            .configuration
            .prompt_width
            .checked_mul(2)
            .ok_or(NativePhotoMakerError::ShapeOverflow)?;
        let normalized = self.photo_layer_norm(
            backend,
            &stacked,
            &[image_count, stacked_width],
            "fuse_module.mlp1.layernorm.weight",
            "fuse_module.mlp1.layernorm.bias",
            context,
        )?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Mlp1Linear1,
            phase_hook,
        )?;
        let expanded = self.photo_linear_state(
            backend,
            &normalized,
            &[image_count, stacked_width],
            "fuse_module.mlp1.fc1.weight",
            Some("fuse_module.mlp1.fc1.bias"),
            context,
        )?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Mlp1Gelu,
            phase_hook,
        )?;
        let activated = gelu_with_context_exact_native(
            backend,
            &expanded,
            GeluApproximation::None,
            DeviceId::CPU,
            context,
        )
        .map_err(photo_canonical)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Mlp1Linear2,
            phase_hook,
        )?;
        let mut fused = self.photo_linear_state(
            backend,
            &activated,
            &[image_count, self.configuration.prompt_width],
            "fuse_module.mlp1.fc2.weight",
            Some("fuse_module.mlp1.fc2.bias"),
            context,
        )?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::PromptResidual,
            phase_hook,
        )?;
        photo_add_in_place(&mut fused, &prompt_values, context.cancellation)?;

        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Mlp2LayerNorm,
            phase_hook,
        )?;
        let normalized = self.photo_layer_norm(
            backend,
            &fused,
            &[image_count, self.configuration.prompt_width],
            "fuse_module.mlp2.layernorm.weight",
            "fuse_module.mlp2.layernorm.bias",
            context,
        )?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Mlp2Linear1,
            phase_hook,
        )?;
        let expanded = self.photo_linear_state(
            backend,
            &normalized,
            &[image_count, self.configuration.prompt_width],
            "fuse_module.mlp2.fc1.weight",
            Some("fuse_module.mlp2.fc1.bias"),
            context,
        )?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Mlp2Gelu,
            phase_hook,
        )?;
        let activated = gelu_with_context_exact_native(
            backend,
            &expanded,
            GeluApproximation::None,
            DeviceId::CPU,
            context,
        )
        .map_err(photo_canonical)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Mlp2Linear2,
            phase_hook,
        )?;
        let residual = self.photo_linear_state(
            backend,
            &activated,
            &[image_count, self.configuration.prompt_width],
            "fuse_module.mlp2.fc2.weight",
            Some("fuse_module.mlp2.fc2.bias"),
            context,
        )?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Mlp2Residual,
            phase_hook,
        )?;
        photo_add_in_place(&mut fused, &residual, context.cancellation)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::FinalLayerNorm,
            phase_hook,
        )?;
        let fused = self.photo_layer_norm(
            backend,
            &fused,
            &[image_count, self.configuration.prompt_width],
            "fuse_module.layer_norm.weight",
            "fuse_module.layer_norm.bias",
            context,
        )?;
        let fused = tensor_from_f32_with_context_exact_native(
            backend,
            &[
                pm_u64(image_count)?,
                pm_u64(self.configuration.prompt_width)?,
            ],
            &fused,
            DType::F32,
            DeviceId::CPU,
            context,
        )
        .map_err(photo_canonical)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::Scatter,
            phase_hook,
        )?;
        let scattered = scatter_method_with_context_exact_native(
            backend,
            &flattened_prompt,
            0,
            &index,
            &fused,
            context,
        )
        .map_err(photo_canonical)?;
        let output = torch_reshape_with_context_exact_native(
            backend,
            &scattered,
            &[
                1,
                pm_i64(sequence)?,
                pm_i64(self.configuration.prompt_width)?,
            ],
            context,
        )
        .map_err(photo_canonical)?;
        photo_require_finite_invocation("output", &output, context.cancellation)?;
        photo_checkpoint(
            context.cancellation,
            NativePhotoMakerPhase::InvocationReturn,
            phase_hook,
        )?;
        Ok(output)
    }

    fn photo_tensor(&self, key: &str) -> Result<&Tensor, NativePhotoMakerError> {
        self.execution_state
            .get(key)
            .ok_or_else(|| NativePhotoMakerError::MissingState(key.to_owned()))
    }

    fn photo_linear_state(
        &self,
        backend: &CpuBackend,
        input: &[f32],
        input_shape: &[usize],
        weight: &str,
        bias: Option<&str>,
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<f32>, NativePhotoMakerError> {
        photo_linear(
            backend,
            input,
            input_shape,
            self.photo_tensor(weight)?,
            bias.map(|key| self.photo_tensor(key)).transpose()?,
            context,
        )
    }

    fn photo_layer_norm(
        &self,
        backend: &CpuBackend,
        input: &[f32],
        shape: &[usize],
        weight: &str,
        bias: &str,
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<f32>, NativePhotoMakerError> {
        let width = *shape.last().ok_or(NativePhotoMakerError::ShapeOverflow)?;
        let weight =
            tensor_to_f32_with_context_exact_native(backend, self.photo_tensor(weight)?, context)
                .map_err(photo_canonical)?;
        let bias =
            tensor_to_f32_with_context_exact_native(backend, self.photo_tensor(bias)?, context)
                .map_err(photo_canonical)?;
        layer_norm_with_context_exact_native(
            backend,
            input,
            shape,
            &[width],
            Some(&weight),
            Some(&bias),
            LAYER_NORM_EPSILON,
            DeviceId::CPU,
            context,
        )
        .map_err(photo_canonical)
    }
}

fn photo_source_configuration() -> NativePhotoMakerConfiguration {
    NativePhotoMakerConfiguration {
        hidden: PHOTOMAKER_SOURCE_HIDDEN,
        intermediate: PHOTOMAKER_SOURCE_INTERMEDIATE,
        heads: PHOTOMAKER_SOURCE_HEADS,
        layers: PHOTOMAKER_SOURCE_LAYERS,
        image_size: PHOTOMAKER_SOURCE_IMAGE,
        patch_size: PHOTOMAKER_SOURCE_PATCH,
        projection: PHOTOMAKER_SOURCE_PROJECTION,
        extra_projection: PHOTOMAKER_SOURCE_EXTRA_PROJECTION,
        prompt_width: PHOTOMAKER_SOURCE_PROJECTION + PHOTOMAKER_SOURCE_EXTRA_PROJECTION,
        source_exact: true,
    }
}

#[cfg(any(test, feature = "test-support"))]
fn photo_reduced_configuration() -> NativePhotoMakerConfiguration {
    NativePhotoMakerConfiguration {
        hidden: PHOTOMAKER_REDUCED_HIDDEN,
        intermediate: PHOTOMAKER_REDUCED_INTERMEDIATE,
        heads: PHOTOMAKER_REDUCED_HEADS,
        layers: PHOTOMAKER_SOURCE_LAYERS,
        image_size: PHOTOMAKER_REDUCED_IMAGE,
        patch_size: PHOTOMAKER_REDUCED_PATCH,
        projection: PHOTOMAKER_REDUCED_PROJECTION,
        extra_projection: PHOTOMAKER_REDUCED_EXTRA_PROJECTION,
        prompt_width: PHOTOMAKER_REDUCED_PROJECTION + PHOTOMAKER_REDUCED_EXTRA_PROJECTION,
        source_exact: false,
    }
}

fn photo_checkpoint(
    cancellation: &CancellationToken,
    phase: NativePhotoMakerPhase,
    phase_hook: &mut impl FnMut(NativePhotoMakerPhase, &CancellationToken),
) -> Result<(), NativePhotoMakerError> {
    phase_hook(phase, cancellation);
    cancellation.check()?;
    Ok(())
}

fn photo_raw_checkpoint_preflight(
    checkpoint: &NativePhotoMakerCheckpoint,
) -> Result<(), NativePhotoMakerError> {
    if checkpoint
        .ordered_entries
        .iter()
        .all(|entry| matches!(entry, NativePhotoMakerCheckpointEntry::Tensor { .. }))
    {
        return photo_raw_state_preflight(
            checkpoint,
            checkpoint.ordered_entries.len(),
            checkpoint
                .ordered_entries
                .iter()
                .filter_map(|entry| match entry {
                    NativePhotoMakerCheckpointEntry::Tensor { key, tensor } => Some((key, tensor)),
                    NativePhotoMakerCheckpointEntry::Mapping { .. } => None,
                }),
        );
    }
    let [NativePhotoMakerCheckpointEntry::Mapping { key, ordered_state }] =
        checkpoint.ordered_entries.as_slice()
    else {
        return Err(NativePhotoMakerError::InvalidCheckpoint(
            "checkpoint outer mapping must contain only id_encoder".to_owned(),
        ));
    };
    if key != "id_encoder" {
        return Err(NativePhotoMakerError::InvalidCheckpoint(
            "checkpoint nested mapping must be named id_encoder".to_owned(),
        ));
    }
    photo_raw_state_preflight(
        checkpoint,
        ordered_state.len(),
        ordered_state.iter().map(|(key, tensor)| (key, tensor)),
    )
}

fn photo_raw_state_preflight<'a>(
    checkpoint: &NativePhotoMakerCheckpoint,
    state_count: usize,
    state: impl Iterator<Item = (&'a String, &'a Tensor)>,
) -> Result<(), NativePhotoMakerError> {
    if state_count != PHOTOMAKER_STATE_COUNT {
        return Err(NativePhotoMakerError::UnexpectedState(format!(
            "expected {PHOTOMAKER_STATE_COUNT} entries, got {}",
            state_count
        )));
    }
    let mut source_bytes = 0_u64;
    let mut projected_bytes = 0_u64;
    let mut maximum_scratch = 0_u64;
    let mut key_bytes = 0_u64;
    for (key, tensor) in state {
        if key.is_empty() || key.len() > MAX_STATE_KEY_BYTES {
            return Err(NativePhotoMakerError::InvalidCheckpoint(
                "state key length is invalid".to_owned(),
            ));
        }
        let source = tensor.storage_byte_len();
        let projected = tensor
            .descriptor()
            .element_count()?
            .checked_mul(DType::F32.byte_width())
            .ok_or(NativePhotoMakerError::ShapeOverflow)?;
        source_bytes = source_bytes
            .checked_add(source)
            .ok_or(NativePhotoMakerError::ShapeOverflow)?;
        projected_bytes = projected_bytes
            .checked_add(projected)
            .ok_or(NativePhotoMakerError::ShapeOverflow)?;
        maximum_scratch = maximum_scratch.max(source.max(projected));
        key_bytes = key_bytes
            .checked_add(pm_u64(key.capacity())?)
            .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    }
    let container_bytes = pm_u64(PHOTOMAKER_STATE_COUNT)?
        .checked_mul(
            u64::try_from(mem::size_of::<(String, Tensor)>())
                .map_err(|_| NativePhotoMakerError::ShapeOverflow)?,
        )
        .and_then(|bytes| bytes.checked_mul(3))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let required = source_bytes
        .checked_add(projected_bytes)
        .and_then(|bytes| bytes.checked_add(maximum_scratch.checked_mul(2)?))
        .and_then(|bytes| bytes.checked_add(key_bytes.checked_mul(3)?))
        .and_then(|bytes| bytes.checked_add(container_bytes))
        .and_then(|bytes| {
            bytes.checked_add(u64::try_from(mem::size_of::<NativePhotoMakerResource>()).ok()?)
        })
        .and_then(|bytes| bytes.checked_add(pm_u64(checkpoint.artifact_sha256.capacity()).ok()?))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    if required > checkpoint.memory_budget_bytes {
        return Err(NativePhotoMakerError::OutOfMemory {
            required,
            budget: checkpoint.memory_budget_bytes,
        });
    }
    Ok(())
}

fn photo_normalize_checkpoint(
    entries: Vec<NativePhotoMakerCheckpointEntry>,
) -> Result<(NativePhotoMakerWrapper, Vec<(String, Tensor)>), NativePhotoMakerError> {
    if entries
        .iter()
        .all(|entry| matches!(entry, NativePhotoMakerCheckpointEntry::Tensor { .. }))
    {
        let state = entries
            .into_iter()
            .map(|entry| match entry {
                NativePhotoMakerCheckpointEntry::Tensor { key, tensor } => Ok((key, tensor)),
                NativePhotoMakerCheckpointEntry::Mapping { .. } => {
                    Err(NativePhotoMakerError::InvalidCheckpoint(
                        "flat checkpoint contains a mapping".to_owned(),
                    ))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        return Ok((NativePhotoMakerWrapper::Flat, state));
    }
    let [NativePhotoMakerCheckpointEntry::Mapping { key, ordered_state }] = entries.as_slice()
    else {
        return Err(NativePhotoMakerError::InvalidCheckpoint(
            "checkpoint outer mapping must contain only id_encoder".to_owned(),
        ));
    };
    if key != "id_encoder" {
        return Err(NativePhotoMakerError::InvalidCheckpoint(
            "checkpoint nested mapping must be named id_encoder".to_owned(),
        ));
    }
    Ok((NativePhotoMakerWrapper::IdEncoder, ordered_state.clone()))
}

fn photo_state_manifest(
    configuration: NativePhotoMakerConfiguration,
) -> Result<Vec<StateSpecification>, NativePhotoMakerError> {
    let hidden = pm_u64(configuration.hidden)?;
    let intermediate = pm_u64(configuration.intermediate)?;
    let positions = configuration
        .image_size
        .checked_div(configuration.patch_size)
        .and_then(|side| side.checked_mul(side))
        .and_then(|patches| patches.checked_add(1))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let mut specifications = Vec::new();
    specifications
        .try_reserve_exact(PHOTOMAKER_STATE_COUNT)
        .map_err(|_| NativePhotoMakerError::Allocation)?;
    let mut push = |key: String, shape: Vec<u64>| {
        specifications.push(StateSpecification { key, shape });
    };
    push(
        "vision_model.embeddings.class_embedding".to_owned(),
        vec![hidden],
    );
    push(
        "vision_model.embeddings.patch_embedding.weight".to_owned(),
        vec![
            hidden,
            3,
            pm_u64(configuration.patch_size)?,
            pm_u64(configuration.patch_size)?,
        ],
    );
    push(
        "vision_model.embeddings.position_embedding.weight".to_owned(),
        vec![pm_u64(positions)?, hidden],
    );
    push("vision_model.pre_layrnorm.weight".to_owned(), vec![hidden]);
    push("vision_model.pre_layrnorm.bias".to_owned(), vec![hidden]);
    for layer in 0..configuration.layers {
        let prefix = format!("vision_model.encoder.layers.{layer}");
        for (suffix, shape) in [
            ("layer_norm1.weight", vec![hidden]),
            ("layer_norm1.bias", vec![hidden]),
            ("self_attn.q_proj.weight", vec![hidden, hidden]),
            ("self_attn.q_proj.bias", vec![hidden]),
            ("self_attn.k_proj.weight", vec![hidden, hidden]),
            ("self_attn.k_proj.bias", vec![hidden]),
            ("self_attn.v_proj.weight", vec![hidden, hidden]),
            ("self_attn.v_proj.bias", vec![hidden]),
            ("self_attn.out_proj.weight", vec![hidden, hidden]),
            ("self_attn.out_proj.bias", vec![hidden]),
            ("layer_norm2.weight", vec![hidden]),
            ("layer_norm2.bias", vec![hidden]),
            ("mlp.fc1.weight", vec![intermediate, hidden]),
            ("mlp.fc1.bias", vec![intermediate]),
            ("mlp.fc2.weight", vec![hidden, intermediate]),
            ("mlp.fc2.bias", vec![hidden]),
        ] {
            push(format!("{prefix}.{suffix}"), shape);
        }
    }
    push(
        "vision_model.post_layernorm.weight".to_owned(),
        vec![hidden],
    );
    push("vision_model.post_layernorm.bias".to_owned(), vec![hidden]);
    push(
        "visual_projection.weight".to_owned(),
        vec![pm_u64(configuration.projection)?, hidden],
    );
    push(
        "visual_projection_2.weight".to_owned(),
        vec![pm_u64(configuration.extra_projection)?, hidden],
    );
    let prompt = pm_u64(configuration.prompt_width)?;
    let stacked = prompt
        .checked_mul(2)
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    for prefix in ["fuse_module.mlp1", "fuse_module.mlp2"] {
        let input = if prefix.ends_with("mlp1") {
            stacked
        } else {
            prompt
        };
        push(format!("{prefix}.layernorm.weight"), vec![input]);
        push(format!("{prefix}.layernorm.bias"), vec![input]);
        push(format!("{prefix}.fc1.weight"), vec![prompt, input]);
        push(format!("{prefix}.fc1.bias"), vec![prompt]);
        push(format!("{prefix}.fc2.weight"), vec![prompt, prompt]);
        push(format!("{prefix}.fc2.bias"), vec![prompt]);
    }
    push("fuse_module.layer_norm.weight".to_owned(), vec![prompt]);
    push("fuse_module.layer_norm.bias".to_owned(), vec![prompt]);
    if specifications.len() != PHOTOMAKER_STATE_COUNT {
        return Err(NativePhotoMakerError::ShapeOverflow);
    }
    Ok(specifications)
}

fn photo_validate_ordered_keys(
    state: &[(String, Tensor)],
    specifications: &[StateSpecification],
) -> Result<(), NativePhotoMakerError> {
    if state.len() != specifications.len() {
        return Err(NativePhotoMakerError::UnexpectedState(format!(
            "expected {} entries, got {}",
            specifications.len(),
            state.len()
        )));
    }
    let mut seen = HashSet::new();
    for ((actual, _), expected) in state.iter().zip(specifications) {
        if !seen.insert(actual) {
            return Err(NativePhotoMakerError::UnexpectedState(format!(
                "duplicate key {actual}"
            )));
        }
        if actual != &expected.key {
            return Err(NativePhotoMakerError::UnexpectedState(format!(
                "expected {}, got {actual}",
                expected.key
            )));
        }
    }
    Ok(())
}

fn photo_validate_source_state(
    state: &[(String, Tensor)],
    specifications: &[StateSpecification],
    stream: StreamId,
    cancellation: &CancellationToken,
) -> Result<DType, NativePhotoMakerError> {
    photo_validate_ordered_keys(state, specifications)?;
    let mut dtype = None;
    let mut storages = HashSet::new();
    for (index, ((key, tensor), specification)) in state.iter().zip(specifications).enumerate() {
        if index.is_multiple_of(8) {
            cancellation.check()?;
        }
        if key.is_empty() || key.len() > MAX_STATE_KEY_BYTES {
            return Err(NativePhotoMakerError::InvalidCheckpoint(
                "state key length is invalid".to_owned(),
            ));
        }
        let descriptor = tensor.descriptor();
        if descriptor.shape() != specification.shape
            || !matches!(descriptor.dtype(), DType::F32 | DType::F16 | DType::Bf16)
        {
            return Err(NativePhotoMakerError::StateShape {
                key: key.clone(),
                expected: specification.shape.clone(),
                actual: descriptor.shape().to_vec(),
                actual_dtype: descriptor.dtype(),
            });
        }
        if descriptor.device() != DeviceId::CPU
            || descriptor.stream() != stream
            || !descriptor.is_contiguous()?
        {
            return Err(NativePhotoMakerError::InvalidCheckpoint(format!(
                "state {key} must be contiguous CPU storage on the construction stream"
            )));
        }
        if dtype
            .replace(descriptor.dtype())
            .is_some_and(|previous| previous != descriptor.dtype())
        {
            return Err(NativePhotoMakerError::InvalidCheckpoint(
                "all source tensors must use one storage dtype".to_owned(),
            ));
        }
        if !storages.insert(tensor.storage_id()) {
            return Err(NativePhotoMakerError::InvalidCheckpoint(format!(
                "state {key} aliases another source tensor"
            )));
        }
        photo_validate_finite_source_tensor(key, tensor, cancellation)?;
    }
    dtype.ok_or_else(|| NativePhotoMakerError::InvalidCheckpoint("state is empty".to_owned()))
}

fn photo_validate_finite_source_tensor(
    key: &str,
    tensor: &Tensor,
    cancellation: &CancellationToken,
) -> Result<(), NativePhotoMakerError> {
    let count = pm_usize(tensor.descriptor().element_count()?)?;
    for index in 0..count {
        if index.is_multiple_of(16_384) {
            cancellation.check()?;
        }
        match tensor
            .descriptor()
            .dtype()
            .decode_scalar(tensor.linear_element_bytes(pm_u64(index)?)?)?
        {
            DecodedScalar::Real(value) if value.is_finite() => {}
            _ => {
                return Err(NativePhotoMakerError::InvalidCheckpoint(format!(
                    "state {key} contains a non-finite or non-real value"
                )));
            }
        }
    }
    cancellation.check()?;
    Ok(())
}

fn photo_validate_finite_tensor(
    key: &str,
    tensor: &Tensor,
    cancellation: &CancellationToken,
) -> Result<(), NativePhotoMakerError> {
    let bytes = tensor.contiguous_bytes()?;
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        if index.is_multiple_of(16_384) {
            cancellation.check()?;
        }
        let raw: [u8; 4] = chunk
            .try_into()
            .map_err(|_| NativePhotoMakerError::SemanticStateChanged)?;
        if !f32::from_ne_bytes(raw).is_finite() {
            return Err(NativePhotoMakerError::InvalidCheckpoint(format!(
                "state {key} contains a non-finite value"
            )));
        }
    }
    cancellation.check()?;
    Ok(())
}

fn photo_validate_distinct_storage(
    source: &BTreeMap<String, Tensor>,
    execution: &BTreeMap<String, Tensor>,
) -> Result<(), NativePhotoMakerError> {
    let source_ids = source
        .values()
        .map(Tensor::storage_id)
        .collect::<HashSet<_>>();
    let mut execution_ids = HashSet::new();
    for tensor in execution.values() {
        if source_ids.contains(&tensor.storage_id()) || !execution_ids.insert(tensor.storage_id()) {
            return Err(NativePhotoMakerError::SemanticStateChanged);
        }
    }
    Ok(())
}

fn photo_clip_configuration(
    configuration: NativePhotoMakerConfiguration,
) -> ClipVisionConfiguration {
    ClipVisionConfiguration {
        model_type: ClipVisionModelType::Clip,
        dtype: DType::F32,
        device: DeviceId::CPU,
        hidden_size: configuration.hidden,
        intermediate_size: configuration.intermediate,
        attention_heads: configuration.heads,
        layer_count: configuration.layers,
        image_size: configuration.image_size,
        patch_size: configuration.patch_size,
        num_channels: 3,
        max_num_patches: 0,
        activation: ClipVisionActivation::QuickGelu,
        projection_dimension: Some(configuration.projection),
        llava_projection_dimension: None,
    }
}

fn photo_clip_weights(
    state: &BTreeMap<String, Tensor>,
    configuration: NativePhotoMakerConfiguration,
) -> Result<ClipVisionWeights, NativePhotoMakerError> {
    let take = |key: &str| {
        state
            .get(key)
            .cloned()
            .ok_or_else(|| NativePhotoMakerError::MissingState(key.to_owned()))
    };
    let mut layers = Vec::new();
    layers
        .try_reserve_exact(configuration.layers)
        .map_err(|_| NativePhotoMakerError::Allocation)?;
    for layer in 0..configuration.layers {
        let prefix = format!("vision_model.encoder.layers.{layer}");
        layers.push(ClipVisionLayerWeights {
            layer_norm_1_weight: take(&format!("{prefix}.layer_norm1.weight"))?,
            layer_norm_1_bias: take(&format!("{prefix}.layer_norm1.bias"))?,
            query_weight: take(&format!("{prefix}.self_attn.q_proj.weight"))?,
            query_bias: take(&format!("{prefix}.self_attn.q_proj.bias"))?,
            key_weight: take(&format!("{prefix}.self_attn.k_proj.weight"))?,
            key_bias: take(&format!("{prefix}.self_attn.k_proj.bias"))?,
            value_weight: take(&format!("{prefix}.self_attn.v_proj.weight"))?,
            value_bias: take(&format!("{prefix}.self_attn.v_proj.bias"))?,
            output_weight: take(&format!("{prefix}.self_attn.out_proj.weight"))?,
            output_bias: take(&format!("{prefix}.self_attn.out_proj.bias"))?,
            layer_norm_2_weight: take(&format!("{prefix}.layer_norm2.weight"))?,
            layer_norm_2_bias: take(&format!("{prefix}.layer_norm2.bias"))?,
            feed_forward_1_weight: take(&format!("{prefix}.mlp.fc1.weight"))?,
            feed_forward_1_bias: take(&format!("{prefix}.mlp.fc1.bias"))?,
            feed_forward_2_weight: take(&format!("{prefix}.mlp.fc2.weight"))?,
            feed_forward_2_bias: take(&format!("{prefix}.mlp.fc2.bias"))?,
        });
    }
    Ok(ClipVisionWeights {
        patch_embedding_weight: take("vision_model.embeddings.patch_embedding.weight")?,
        patch_embedding_bias: None,
        class_embedding: Some(take("vision_model.embeddings.class_embedding")?),
        position_embedding: take("vision_model.embeddings.position_embedding.weight")?,
        pre_layer_norm_weight: Some(take("vision_model.pre_layrnorm.weight")?),
        pre_layer_norm_bias: Some(take("vision_model.pre_layrnorm.bias")?),
        layers,
        post_layer_norm_weight: take("vision_model.post_layernorm.weight")?,
        post_layer_norm_bias: take("vision_model.post_layernorm.bias")?,
        visual_projection_weight: Some(take("visual_projection.weight")?),
        llava_linear_1_weight: None,
        llava_linear_1_bias: None,
        llava_linear_2_weight: None,
        llava_linear_2_bias: None,
    })
}

fn photo_require_clip_storage_from_execution(
    clip: &NativeClipVision,
    execution: &BTreeMap<String, Tensor>,
) -> Result<(), NativePhotoMakerError> {
    let execution_ids = execution
        .values()
        .map(Tensor::storage_id)
        .collect::<HashSet<_>>();
    let parts = clip.resident_parts()?;
    if parts.tensor_allocations().len() != PHOTOMAKER_CLIP_STATE_COUNT
        || parts
            .tensor_allocations()
            .iter()
            .any(|allocation| !execution_ids.contains(&allocation.storage_id()))
    {
        return Err(NativePhotoMakerError::SemanticStateChanged);
    }
    Ok(())
}

fn photo_validate_resource_storage(
    source: &BTreeMap<String, Tensor>,
    execution: &BTreeMap<String, Tensor>,
    clip: &NativeClipVision,
) -> Result<(), NativePhotoMakerError> {
    photo_validate_distinct_storage(source, execution)?;
    let mut retained = source
        .values()
        .chain(execution.values())
        .map(Tensor::storage_id)
        .collect::<HashSet<_>>();
    let parts = clip.resident_parts()?;
    if parts.tensor_allocations().len() != PHOTOMAKER_CLIP_STATE_COUNT {
        return Err(NativePhotoMakerError::SemanticStateChanged);
    }
    for allocation in parts.tensor_allocations() {
        if !retained.insert(allocation.storage_id()) {
            return Err(NativePhotoMakerError::SemanticStateChanged);
        }
    }
    Ok(())
}

fn photo_validate_invocation(
    configuration: NativePhotoMakerConfiguration,
    image: &Tensor,
    prompt: &Tensor,
    mask: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<(), NativePhotoMakerError> {
    let image_shape = image.descriptor().shape();
    let prompt_shape = prompt.descriptor().shape();
    let mask_shape = mask.descriptor().shape();
    if image_shape.len() != 5
        || image_shape[0] != 1
        || image_shape[1] == 0
        || image_shape[2] != 3
        || image_shape[3] != pm_u64(configuration.image_size)?
        || image_shape[4] != pm_u64(configuration.image_size)?
    {
        return Err(NativePhotoMakerError::InvalidInput(
            "image must have [1, N, 3, image_size, image_size] shape with N greater than zero"
                .to_owned(),
        ));
    }
    if prompt_shape.len() != 3
        || prompt_shape[0] != 1
        || prompt_shape[1] == 0
        || prompt_shape[2] != pm_u64(configuration.prompt_width)?
    {
        return Err(NativePhotoMakerError::InvalidInput(
            "prompt must have [1, S, projection_width] shape".to_owned(),
        ));
    }
    if mask_shape != [1, prompt_shape[1]] {
        return Err(NativePhotoMakerError::InvalidInput(
            "class token mask must have [1, S] shape".to_owned(),
        ));
    }
    for (name, tensor, dtype) in [
        ("image", image, DType::F32),
        ("prompt", prompt, DType::F32),
        ("class token mask", mask, DType::Bool),
    ] {
        let descriptor = tensor.descriptor();
        if descriptor.dtype() != dtype
            || descriptor.device() != DeviceId::CPU
            || descriptor.stream() != context.stream
            || !descriptor.is_contiguous()?
        {
            return Err(NativePhotoMakerError::InvalidInput(format!(
                "{name} must be contiguous {dtype:?} CPU state on the execution stream"
            )));
        }
    }
    photo_require_finite_invocation("image", image, context.cancellation)?;
    photo_require_finite_invocation("prompt", prompt, context.cancellation)?;
    Ok(())
}

fn photo_mask_true_count(
    backend: &CpuBackend,
    mask: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<usize, NativePhotoMakerError> {
    let sum = tensor_sum_with_context_exact_native(backend, mask, None, false, None, context)
        .map_err(photo_canonical)?;
    match sum
        .descriptor()
        .dtype()
        .decode_scalar(sum.linear_element_bytes(0)?)?
    {
        DecodedScalar::Signed(value) => {
            usize::try_from(value).map_err(|_| NativePhotoMakerError::ShapeOverflow)
        }
        DecodedScalar::Unsigned(value) => pm_usize(value),
        _ => Err(NativePhotoMakerError::SemanticStateChanged),
    }
}

fn photo_require_finite_invocation(
    name: &str,
    tensor: &Tensor,
    cancellation: &CancellationToken,
) -> Result<(), NativePhotoMakerError> {
    let bytes = tensor.contiguous_bytes()?;
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        if index.is_multiple_of(16_384) {
            cancellation.check()?;
        }
        let raw: [u8; 4] = chunk
            .try_into()
            .map_err(|_| NativePhotoMakerError::InvalidInput(name.to_owned()))?;
        if !f32::from_ne_bytes(raw).is_finite() {
            return Err(NativePhotoMakerError::InvalidInput(format!(
                "{name} contains a non-finite value"
            )));
        }
    }
    cancellation.check()?;
    Ok(())
}

fn photo_mask_positions(
    backend: &CpuBackend,
    mask: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Vec<usize>, NativePhotoMakerError> {
    let matrix = match nonzero_with_context_exact_native(backend, mask, false, context)
        .map_err(photo_canonical)?
    {
        NonzeroOutput::Matrix(matrix) => matrix,
        NonzeroOutput::Tuple(_) => {
            return Err(NativePhotoMakerError::SemanticStateChanged);
        }
    };
    let [rows, columns] = matrix.descriptor().shape() else {
        return Err(NativePhotoMakerError::SemanticStateChanged);
    };
    if *columns != 2 {
        return Err(NativePhotoMakerError::SemanticStateChanged);
    }
    let rows = pm_usize(*rows)?;
    let mut positions = Vec::new();
    positions
        .try_reserve_exact(rows)
        .map_err(|_| NativePhotoMakerError::Allocation)?;
    for row in 0..rows {
        context.check()?;
        let byte_index = row
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(NativePhotoMakerError::ShapeOverflow)?;
        let bytes = matrix.linear_element_bytes(pm_u64(byte_index)?)?;
        let raw: [u8; 8] = bytes
            .try_into()
            .map_err(|_| NativePhotoMakerError::SemanticStateChanged)?;
        let position = i64::from_ne_bytes(raw);
        positions
            .push(usize::try_from(position).map_err(|_| NativePhotoMakerError::ShapeOverflow)?);
    }
    Ok(positions)
}

fn photo_row_index_tensor(
    backend: &CpuBackend,
    positions: &[usize],
    width: usize,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, NativePhotoMakerError> {
    let count = positions
        .len()
        .checked_mul(width)
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(
            count
                .checked_mul(mem::size_of::<i64>())
                .ok_or(NativePhotoMakerError::ShapeOverflow)?,
        )
        .map_err(|_| NativePhotoMakerError::Allocation)?;
    for (row, position) in positions.iter().copied().enumerate() {
        context.check()?;
        let position = i64::try_from(position).map_err(|_| NativePhotoMakerError::ShapeOverflow)?;
        for _ in 0..width {
            bytes.extend_from_slice(&position.to_ne_bytes());
        }
        if row.is_multiple_of(64) {
            context.check()?;
        }
    }
    let descriptor = comfy_tensor::TensorDescriptor::contiguous(
        vec![pm_u64(positions.len())?, pm_u64(width)?],
        DType::I64,
        DeviceId::CPU,
        context.stream,
    )?;
    let (tensor, event) = backend.upload_bytes(descriptor, &bytes, context)?;
    backend.wait_event(event, context)?;
    Ok(tensor)
}

fn photo_linear(
    backend: &CpuBackend,
    input: &[f32],
    input_shape: &[usize],
    weight: &Tensor,
    bias: Option<&Tensor>,
    context: &ExecutionContext<'_>,
) -> Result<Vec<f32>, NativePhotoMakerError> {
    let weight_values = tensor_to_f32_with_context_exact_native(backend, weight, context)
        .map_err(photo_canonical)?;
    let bias_values = bias
        .map(|bias| tensor_to_f32_with_context_exact_native(backend, bias, context))
        .transpose()
        .map_err(photo_canonical)?;
    let weight_shape = weight
        .descriptor()
        .shape()
        .iter()
        .map(|dimension| pm_usize(*dimension))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(linear_with_context_exact_native(
        backend,
        input,
        input_shape,
        &weight_values,
        &weight_shape,
        bias_values.as_deref(),
        DeviceId::CPU,
        context,
    )
    .map_err(photo_canonical)?
    .values)
}

fn photo_add_in_place(
    left: &mut [f32],
    right: &[f32],
    cancellation: &CancellationToken,
) -> Result<(), NativePhotoMakerError> {
    if left.len() != right.len() {
        return Err(NativePhotoMakerError::ShapeOverflow);
    }
    for (index, (left, right)) in left.iter_mut().zip(right).enumerate() {
        if index.is_multiple_of(16_384) {
            cancellation.check()?;
        }
        *left += *right;
    }
    cancellation.check()?;
    Ok(())
}

fn photo_construction_peak(
    artifact: &str,
    artifact_capacity: usize,
    ordered_state: &[(String, Tensor)],
    specifications: &[StateSpecification],
) -> Result<u64, NativePhotoMakerError> {
    let source_bytes = ordered_state.iter().try_fold(0_u64, |total, (_, tensor)| {
        total
            .checked_add(tensor.storage_byte_len())
            .ok_or(NativePhotoMakerError::ShapeOverflow)
    })?;
    let projected_bytes = specifications
        .iter()
        .try_fold(0_u64, |total, specification| {
            total
                .checked_add(photo_tensor_bytes(&specification.shape, DType::F32)?)
                .ok_or(NativePhotoMakerError::ShapeOverflow)
        })?;
    let maximum_projection_scratch = specifications
        .iter()
        .map(|specification| photo_tensor_bytes(&specification.shape, DType::F32))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0)
        .checked_mul(2)
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let key_bytes = ordered_state.iter().try_fold(0_u64, |total, (key, _)| {
        total
            .checked_add(pm_u64(key.capacity())?)
            .ok_or(NativePhotoMakerError::ShapeOverflow)
    })?;
    let container_bytes = pm_u64(ordered_state.len())?
        .checked_mul(
            u64::try_from(mem::size_of::<(String, Tensor)>())
                .map_err(|_| NativePhotoMakerError::ShapeOverflow)?,
        )
        .and_then(|bytes| bytes.checked_mul(3))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    source_bytes
        .checked_add(projected_bytes)
        .and_then(|bytes| bytes.checked_add(maximum_projection_scratch))
        .and_then(|bytes| bytes.checked_add(key_bytes.checked_mul(3)?))
        .and_then(|bytes| bytes.checked_add(container_bytes))
        .and_then(|bytes| {
            bytes.checked_add(u64::try_from(mem::size_of::<NativePhotoMakerResource>()).ok()?)
        })
        .and_then(|bytes| bytes.checked_add(pm_u64(artifact_capacity.max(artifact.len())).ok()?))
        .ok_or(NativePhotoMakerError::ShapeOverflow)
}

fn photo_invocation_peak(
    resource: &NativePhotoMakerResource,
    image_count: usize,
    sequence: usize,
) -> Result<(), NativePhotoMakerError> {
    let configuration = resource.configuration;
    let positions = configuration
        .image_size
        .checked_div(configuration.patch_size)
        .and_then(|side| side.checked_mul(side))
        .and_then(|patches| patches.checked_add(1))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let scalar_bytes = 4_u64;
    let image_bytes = pm_u64(image_count)?
        .checked_mul(3)
        .and_then(|count| count.checked_mul(pm_u64(configuration.image_size).ok()?))
        .and_then(|count| count.checked_mul(pm_u64(configuration.image_size).ok()?))
        .and_then(|count| count.checked_mul(scalar_bytes))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let hidden_bytes = pm_u64(image_count)?
        .checked_mul(pm_u64(positions)?)
        .and_then(|count| count.checked_mul(pm_u64(configuration.hidden).ok()?))
        .and_then(|count| count.checked_mul(scalar_bytes))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let attention_bytes = pm_u64(image_count)?
        .checked_mul(pm_u64(configuration.heads)?)
        .and_then(|count| count.checked_mul(pm_u64(positions).ok()?))
        .and_then(|count| count.checked_mul(pm_u64(positions).ok()?))
        .and_then(|count| count.checked_mul(scalar_bytes))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let prompt_bytes = pm_u64(sequence)?
        .checked_mul(pm_u64(configuration.prompt_width)?)
        .and_then(|count| count.checked_mul(scalar_bytes))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let selected_bytes = pm_u64(image_count)?
        .checked_mul(pm_u64(configuration.prompt_width)?)
        .and_then(|count| count.checked_mul(scalar_bytes))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let decoded_weight_bytes = resource
        .execution_state
        .values()
        .map(Tensor::storage_byte_len)
        .max()
        .unwrap_or(0)
        .checked_mul(2)
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let phase_bytes = image_bytes
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(hidden_bytes.checked_mul(32)?))
        .and_then(|bytes| bytes.checked_add(attention_bytes.checked_mul(2)?))
        .and_then(|bytes| bytes.checked_add(prompt_bytes.checked_mul(3)?))
        .and_then(|bytes| bytes.checked_add(selected_bytes.checked_mul(24)?))
        .and_then(|bytes| bytes.checked_add(decoded_weight_bytes))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let required = resource
        .resident_bytes
        .checked_add(phase_bytes)
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    if required > resource.memory_budget_bytes {
        return Err(NativePhotoMakerError::OutOfMemory {
            required,
            budget: resource.memory_budget_bytes,
        });
    }
    Ok(())
}

fn photo_semantic_digest(
    configuration: NativePhotoMakerConfiguration,
    wrapper: NativePhotoMakerWrapper,
    artifact: &str,
    source_dtype: DType,
    source_state: &BTreeMap<String, Tensor>,
    execution_state: &BTreeMap<String, Tensor>,
    clip_digest: &str,
    specifications: &[StateSpecification],
    cancellation: &CancellationToken,
) -> Result<String, NativePhotoMakerError> {
    let mut hasher = Sha256::new();
    hasher.update(b"zed.comfy.photomaker-resource.v1\0");
    hasher.update([match wrapper {
        NativePhotoMakerWrapper::Flat => 1,
        NativePhotoMakerWrapper::IdEncoder => 2,
    }]);
    for value in [
        configuration.hidden,
        configuration.intermediate,
        configuration.heads,
        configuration.layers,
        configuration.image_size,
        configuration.patch_size,
        configuration.projection,
        configuration.extra_projection,
        configuration.prompt_width,
    ] {
        hasher.update(pm_u64(value)?.to_le_bytes());
    }
    hasher.update([u8::from(configuration.source_exact)]);
    hasher.update(artifact.as_bytes());
    hasher.update(source_dtype.catalog_name().as_bytes());
    hasher.update(clip_digest.as_bytes());
    for (index, specification) in specifications.iter().enumerate() {
        if index.is_multiple_of(8) {
            cancellation.check()?;
        }
        photo_hash_tensor(
            &mut hasher,
            &specification.key,
            source_state
                .get(&specification.key)
                .ok_or_else(|| NativePhotoMakerError::MissingState(specification.key.clone()))?,
            cancellation,
        )?;
        if index >= PHOTOMAKER_CLIP_STATE_COUNT {
            photo_hash_tensor(
                &mut hasher,
                &specification.key,
                execution_state.get(&specification.key).ok_or_else(|| {
                    NativePhotoMakerError::MissingState(specification.key.clone())
                })?,
                cancellation,
            )?;
        }
    }
    cancellation.check()?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn photo_hash_tensor(
    hasher: &mut Sha256,
    key: &str,
    tensor: &Tensor,
    cancellation: &CancellationToken,
) -> Result<(), NativePhotoMakerError> {
    hasher.update(pm_u64(key.len())?.to_le_bytes());
    hasher.update(key.as_bytes());
    hasher.update(tensor.descriptor().dtype().catalog_name().as_bytes());
    for dimension in tensor.descriptor().shape() {
        hasher.update(dimension.to_le_bytes());
    }
    for chunk in tensor.contiguous_bytes()?.chunks(DIGEST_CHUNK_BYTES) {
        cancellation.check()?;
        hasher.update(chunk);
    }
    Ok(())
}

fn photo_resident_owned_bytes(
    artifact: &str,
    artifact_capacity: usize,
    digest: &str,
    digest_capacity: usize,
    source_state: &BTreeMap<String, Tensor>,
    execution_state: &BTreeMap<String, Tensor>,
    clip: &NativeClipVision,
) -> Result<u64, NativePhotoMakerError> {
    let base = u64::try_from(mem::size_of::<NativePhotoMakerResource>())
        .map_err(|_| NativePhotoMakerError::ShapeOverflow)?;
    let keys =
        source_state
            .keys()
            .chain(execution_state.keys())
            .try_fold(0_u64, |total, key| {
                total
                    .checked_add(pm_u64(key.capacity())?)
                    .ok_or(NativePhotoMakerError::ShapeOverflow)
            })?;
    let map_nodes = pm_u64(source_state.len())?
        .checked_mul(
            u64::try_from(mem::size_of::<(String, Tensor)>())
                .map_err(|_| NativePhotoMakerError::ShapeOverflow)?
                .checked_add(64)
                .ok_or(NativePhotoMakerError::ShapeOverflow)?,
        )
        .and_then(|bytes| bytes.checked_mul(2))
        .ok_or(NativePhotoMakerError::ShapeOverflow)?;
    let clip_owned_bytes = clip.resident_parts()?.owned_bytes();
    base.checked_add(pm_u64(artifact_capacity.max(artifact.len()))?)
        .and_then(|bytes| bytes.checked_add(pm_u64(digest_capacity.max(digest.len())).ok()?))
        .and_then(|bytes| bytes.checked_add(keys))
        .and_then(|bytes| bytes.checked_add(map_nodes))
        .and_then(|bytes| bytes.checked_add(clip_owned_bytes))
        .ok_or(NativePhotoMakerError::ShapeOverflow)
}

fn photo_resident_tensor_allocations<'a>(
    maps: impl IntoIterator<Item = &'a BTreeMap<String, Tensor>>,
    cancellation: &CancellationToken,
) -> Result<Vec<(StorageId, u64)>, NativePhotoMakerError> {
    let mut allocations = Vec::new();
    for map in maps {
        for (index, tensor) in map.values().enumerate() {
            if index.is_multiple_of(16) {
                cancellation.check()?;
            }
            let storage_id = tensor.storage_id();
            let bytes = tensor.storage_byte_len();
            if let Some((_, existing)) = allocations
                .iter()
                .find(|(existing_id, _)| *existing_id == storage_id)
            {
                if *existing != bytes {
                    return Err(NativePhotoMakerError::SemanticStateChanged);
                }
            } else {
                allocations.push((storage_id, bytes));
            }
        }
    }
    cancellation.check()?;
    Ok(allocations)
}

fn photo_resource_tensor_allocations(
    source: &BTreeMap<String, Tensor>,
    execution: &BTreeMap<String, Tensor>,
    clip: &NativeClipVision,
    cancellation: &CancellationToken,
) -> Result<Vec<(StorageId, u64)>, NativePhotoMakerError> {
    let mut allocations = photo_resident_tensor_allocations([source, execution], cancellation)?;
    let clip_parts = clip.resident_parts()?;
    if clip_parts.tensor_allocations().len() != PHOTOMAKER_CLIP_STATE_COUNT {
        return Err(NativePhotoMakerError::SemanticStateChanged);
    }
    for (index, allocation) in clip_parts.tensor_allocations().iter().enumerate() {
        if index.is_multiple_of(16) {
            cancellation.check()?;
        }
        if allocations
            .iter()
            .any(|(storage_id, _)| *storage_id == allocation.storage_id())
        {
            return Err(NativePhotoMakerError::SemanticStateChanged);
        }
        allocations.push((allocation.storage_id(), allocation.resident_bytes()));
    }
    cancellation.check()?;
    Ok(allocations)
}

fn photo_tensor_bytes(shape: &[u64], dtype: DType) -> Result<u64, NativePhotoMakerError> {
    shape
        .iter()
        .try_fold(1_u64, |count, dimension| {
            count
                .checked_mul(*dimension)
                .ok_or(NativePhotoMakerError::ShapeOverflow)
        })?
        .checked_mul(dtype.byte_width())
        .ok_or(NativePhotoMakerError::ShapeOverflow)
}

fn photo_validate_sha256(value: &str) -> Result<(), NativePhotoMakerError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(NativePhotoMakerError::InvalidCheckpoint(
            "artifact identity must be canonical lowercase SHA-256".to_owned(),
        ));
    }
    Ok(())
}

fn photo_canonical(error: impl std::error::Error + 'static) -> NativePhotoMakerError {
    let mut source: &(dyn std::error::Error + 'static) = &error;
    loop {
        if is_cancellation_error(source) {
            return NativePhotoMakerError::Cancelled;
        }
        let Some(next) = source.source() else {
            break;
        };
        source = next;
    }
    NativePhotoMakerError::Canonical(error.to_string())
}

fn pm_u64(value: usize) -> Result<u64, NativePhotoMakerError> {
    u64::try_from(value).map_err(|_| NativePhotoMakerError::ShapeOverflow)
}

fn pm_usize(value: u64) -> Result<usize, NativePhotoMakerError> {
    usize::try_from(value).map_err(|_| NativePhotoMakerError::ShapeOverflow)
}

fn pm_i64(value: impl TryInto<i64>) -> Result<i64, NativePhotoMakerError> {
    value
        .try_into()
        .map_err(|_| NativePhotoMakerError::ShapeOverflow)
}

#[cfg(test)]
mod photomaker_tests {
    use super::*;
    use comfy_tensor::{CpuWorkspaceAuthority, TensorDescriptor};

    const TEST_MEMORY: u64 = 64 * 1024 * 1024;

    fn fixture_checkpoint(
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<NativePhotoMakerCheckpoint, NativePhotoMakerError> {
        let configuration = photo_reduced_configuration();
        let specifications = photo_state_manifest(configuration)?;
        let mut ordered_entries = Vec::new();
        ordered_entries
            .try_reserve_exact(specifications.len())
            .map_err(|_| NativePhotoMakerError::Allocation)?;
        for (state_index, specification) in specifications.into_iter().enumerate() {
            context.check()?;
            let count = pm_usize(specification.shape.iter().try_fold(
                1_u64,
                |count, dimension| {
                    count
                        .checked_mul(*dimension)
                        .ok_or(NativePhotoMakerError::ShapeOverflow)
                },
            )?)?;
            let values = (0..count)
                .map(|index| {
                    if specification.key.ends_with("layernorm.weight")
                        || specification.key.ends_with("layrnorm.weight")
                        || specification.key.ends_with("layer_norm1.weight")
                        || specification.key.ends_with("layer_norm2.weight")
                        || specification.key.ends_with("post_layernorm.weight")
                    {
                        1.0
                    } else {
                        ((state_index + index) % 7) as f32 * 0.001
                    }
                })
                .collect::<Vec<_>>();
            let tensor = tensor_from_f32_with_context_exact_native(
                backend,
                &specification.shape,
                &values,
                DType::F32,
                DeviceId::CPU,
                context,
            )
            .map_err(photo_canonical)?;
            ordered_entries.push(NativePhotoMakerCheckpointEntry::Tensor {
                key: specification.key,
                tensor,
            });
        }
        Ok(NativePhotoMakerCheckpoint {
            artifact_sha256: format!("{:x}", Sha256::digest(b"task395-phase-fixture")),
            ordered_entries,
            memory_budget_bytes: TEST_MEMORY,
        })
    }

    fn invocation_inputs(
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<(Tensor, Tensor, Tensor), NativePhotoMakerError> {
        let image = tensor_from_f32_with_context_exact_native(
            backend,
            &[1, 2, 3, 4, 4],
            &[0.03125; 96],
            DType::F32,
            DeviceId::CPU,
            context,
        )
        .map_err(photo_canonical)?;
        let prompt = tensor_from_f32_with_context_exact_native(
            backend,
            &[1, 4, 8],
            &[0.0625; 32],
            DType::F32,
            DeviceId::CPU,
            context,
        )
        .map_err(photo_canonical)?;
        let descriptor =
            TensorDescriptor::contiguous(vec![1, 4], DType::Bool, DeviceId::CPU, context.stream)?;
        let (mask, event) = backend.upload_bytes(descriptor, &[0, 1, 0, 1], context)?;
        backend.wait_event(event, context)?;
        Ok((image, prompt, mask))
    }

    #[test]
    fn photomaker_cancellation_phases_do_not_publish() -> Result<(), NativePhotoMakerError> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(TEST_MEMORY)?;
        let construction_phases = [
            NativePhotoMakerPhase::Entry,
            NativePhotoMakerPhase::Wrapper,
            NativePhotoMakerPhase::Schema,
            NativePhotoMakerPhase::SourceValidation,
            NativePhotoMakerPhase::Projection,
            NativePhotoMakerPhase::ClipVision,
            NativePhotoMakerPhase::SemanticDigest,
            NativePhotoMakerPhase::Validation,
            NativePhotoMakerPhase::Return,
        ];
        let setup_cancellation = CancellationToken::default();
        let setup_workspace = authority.authorize_workspace(TEST_MEMORY)?;
        let setup_context =
            backend.execution_context(StreamId::DEFAULT, setup_workspace, &setup_cancellation);
        let checkpoint = fixture_checkpoint(&backend, &setup_context)?;
        for target in construction_phases {
            let cancellation = CancellationToken::default();
            let workspace = authority.authorize_workspace(TEST_MEMORY)?;
            let context = backend.execution_context(StreamId::DEFAULT, workspace, &cancellation);
            let result = NativePhotoMakerResource::checked(
                &backend,
                checkpoint.clone(),
                photo_reduced_configuration(),
                &context,
                &mut |phase, cancellation| {
                    if phase == target {
                        cancellation.cancel();
                    }
                },
            );
            assert!(matches!(result, Err(NativePhotoMakerError::Cancelled)));
        }

        let resource =
            NativePhotoMakerResource::from_reduced_fixture(&backend, checkpoint, &setup_context)?;
        let (image, prompt, mask) = invocation_inputs(&backend, &setup_context)?;
        let invocation_phases = [
            NativePhotoMakerPhase::InvocationAdmission,
            NativePhotoMakerPhase::ClipForward,
            NativePhotoMakerPhase::ExtraProjection,
            NativePhotoMakerPhase::MaskIndices,
            NativePhotoMakerPhase::Gather,
            NativePhotoMakerPhase::Mlp1LayerNorm,
            NativePhotoMakerPhase::Mlp1Linear1,
            NativePhotoMakerPhase::Mlp1Gelu,
            NativePhotoMakerPhase::Mlp1Linear2,
            NativePhotoMakerPhase::PromptResidual,
            NativePhotoMakerPhase::Mlp2LayerNorm,
            NativePhotoMakerPhase::Mlp2Linear1,
            NativePhotoMakerPhase::Mlp2Gelu,
            NativePhotoMakerPhase::Mlp2Linear2,
            NativePhotoMakerPhase::Mlp2Residual,
            NativePhotoMakerPhase::FinalLayerNorm,
            NativePhotoMakerPhase::Scatter,
            NativePhotoMakerPhase::InvocationReturn,
        ];
        for target in invocation_phases {
            let cancellation = CancellationToken::default();
            let workspace = authority.authorize_workspace(TEST_MEMORY)?;
            let context = backend.execution_context(StreamId::DEFAULT, workspace, &cancellation);
            let result = resource.fuse_conditioning_with_phase_hook(
                &backend,
                &image,
                &prompt,
                &mask,
                &context,
                &mut |phase, cancellation| {
                    if phase == target {
                        cancellation.cancel();
                    }
                },
            );
            assert!(matches!(result, Err(NativePhotoMakerError::Cancelled)));
        }
        Ok(())
    }
}
