use comfy_tensor::{
    CancellationToken, CpuBackend, DType, DecodedScalar, DeviceId, ExecutionContext, Scalar,
    StorageId, StreamId, Tensor, TensorError,
    generated_activation_normalization_functional_01::{
        FunctionalError, layer_norm_with_context_exact_native, silu_with_context_exact_native,
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
    generated_indexing_masking_01::{IndexingMaskingPartOneError, narrow_method_exact_native},
    generated_linear_algebra_01::{LinearAlgebraPartOneError, matmul_with_context_exact_native},
    generated_neural_network_functional_01::{
        NeuralNetworkFunctionalError, linear_with_context_exact_native,
    },
    generated_shape_layout_transform_02::{
        ShapeLayoutTransformPartTwoError, tensor_repeat_with_context_exact_native,
        torch_cat_with_context_exact_native,
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
    clip_vision::{ClipVisionError, ClipVisionOutput},
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
    ClipVision(#[from] ClipVisionError),
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
