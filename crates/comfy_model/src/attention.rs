use comfy_tensor::{
    CpuBackend, DType, DecodedScalar, DeviceId, ExecutionContext, Scalar, Tensor, TensorDescriptor,
    TensorError,
    generated_accelerated_attention_kernel_01::{
        AttentionKernelError, AttentionKernelKind, AttentionKernelRequest,
        AttentionLayout as TensorAttentionLayout, AttentionMask as TensorAttentionMask,
        AttentionMaskShape as TensorAttentionMaskShape, AttentionShape, CheckedAttentionInvocation,
    },
};
use comfy_types::CancellationToken;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionBackend {
    PytorchSdp,
    SageOrFlash,
    SplitOrSubQuadratic,
    Xformers,
}

impl AttentionBackend {
    pub const fn feature_id(self) -> &'static str {
        match self {
            Self::PytorchSdp => "COMFY-MODEL-0001",
            Self::SageOrFlash => "COMFY-MODEL-0002",
            Self::SplitOrSubQuadratic => "COMFY-MODEL-0003",
            Self::Xformers => "COMFY-MODEL-0004",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttentionFallbackPolicy {
    AllowExactNative,
    Forbid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathSdpSelection {
    Enabled(AttentionKernelKind),
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathSdpReductionPolicy {
    allow_fp16_bf16: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdpaBackend {
    CudnnAttention,
    FlashAttention,
    EfficientAttention,
    Math,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SdpaKernelSelection {
    backends: Vec<SdpaBackend>,
    set_priority: bool,
}

impl SdpaKernelSelection {
    pub fn backends(&self) -> &[SdpaBackend] {
        &self.backends
    }

    pub const fn set_priority(&self) -> bool {
        self.set_priority
    }
}

impl MathSdpReductionPolicy {
    pub const fn allow_fp16_bf16(self) -> bool {
        self.allow_fp16_bf16
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttentionMaskShape {
    KeyTokens,
    QueryByKey,
    BatchQueryByKey,
    BatchHeadQueryByKey,
}

#[derive(Clone, Copy, Debug)]
pub enum AttentionMask<'a> {
    Boolean {
        values: &'a [bool],
        shape: AttentionMaskShape,
    },
    Additive {
        values: &'a [f32],
        shape: AttentionMaskShape,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct AttentionRequest {
    pub backend: AttentionBackend,
    pub fallback: AttentionFallbackPolicy,
    pub batch: usize,
    pub query_tokens: usize,
    pub key_tokens: usize,
    pub heads: usize,
    pub head_dimension: usize,
    pub value_dimension: usize,
    pub scale: Option<f32>,
    pub workspace_limit_bytes: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttentionOutcome {
    pub values: Vec<f32>,
    pub requested_backend: AttentionBackend,
    pub effective_backend: AttentionBackend,
    pub fallback_reason: Option<String>,
    pub query_chunk_size: usize,
    pub peak_workspace_bytes: usize,
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum AttentionError {
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error("attention dimensions or workspace size overflowed")]
    ShapeOverflow,
    #[error("attention allocation for {name} failed")]
    AllocationFailed { name: &'static str },
    #[error("attention dimension {name} must be non-zero")]
    EmptyDimension { name: &'static str },
    #[error("attention {name} expected {expected} values, got {actual}")]
    ValueCount {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("attention mask expected {expected} values, got {actual}")]
    MaskValueCount { expected: usize, actual: usize },
    #[error("attention scale must be finite and greater than zero")]
    InvalidScale,
    #[error("attention tensor {name} expected shape {expected:?}, got {actual:?}")]
    TensorShape {
        name: &'static str,
        expected: Vec<u64>,
        actual: Vec<u64>,
    },
    #[error("attention tensor dtype {dtype:?} is unsupported; expected F32, F16, or BF16")]
    UnsupportedTensorDType { dtype: DType },
    #[error("attention workspace requires at least {required} bytes, got {available}")]
    WorkspaceTooSmall { required: usize, available: usize },
    #[error("attention backend {backend:?} is unavailable: {reason}")]
    UnsupportedBackend {
        backend: AttentionBackend,
        reason: String,
    },
    #[error("attention was cancelled")]
    Cancelled,
    #[error("SDPA kernel selection must contain between one and four unique backends")]
    InvalidSdpaSelection,
}

pub fn enable_math_sdp_exact_native(
    enabled: bool,
    cancellation: &CancellationToken,
) -> Result<MathSdpSelection, AttentionError> {
    cancellation.check()?;
    Ok(if enabled {
        MathSdpSelection::Enabled(AttentionKernelKind::ReferenceSdp)
    } else {
        MathSdpSelection::Disabled
    })
}

pub fn enable_flash_sdp_exact_native(
    enabled: bool,
    cancellation: &CancellationToken,
) -> Result<MathSdpSelection, AttentionError> {
    cancellation.check()?;
    Ok(if enabled {
        MathSdpSelection::Enabled(AttentionKernelKind::FlashAttention)
    } else {
        MathSdpSelection::Disabled
    })
}

pub fn enable_mem_efficient_sdp_exact_native(
    enabled: bool,
    cancellation: &CancellationToken,
) -> Result<MathSdpSelection, AttentionError> {
    cancellation.check()?;
    Ok(if enabled {
        MathSdpSelection::Enabled(AttentionKernelKind::ReferenceSdp)
    } else {
        MathSdpSelection::Disabled
    })
}

pub fn allow_fp16_bf16_reduction_math_sdp_exact_native(
    enabled: bool,
    cancellation: &CancellationToken,
) -> Result<MathSdpReductionPolicy, AttentionError> {
    cancellation.check()?;
    Ok(MathSdpReductionPolicy {
        allow_fp16_bf16: enabled,
    })
}

pub fn sdpa_kernel_exact_native(
    backends: &[SdpaBackend],
    set_priority: bool,
    cancellation: &CancellationToken,
) -> Result<SdpaKernelSelection, AttentionError> {
    cancellation.check()?;
    if backends.is_empty() || backends.len() > 4 {
        return Err(AttentionError::InvalidSdpaSelection);
    }
    for (index, backend) in backends.iter().enumerate() {
        cancellation.check()?;
        if backends
            .get(..index)
            .is_some_and(|prior| prior.contains(backend))
        {
            return Err(AttentionError::InvalidSdpaSelection);
        }
    }
    cancellation.check()?;
    Ok(SdpaKernelSelection {
        backends: backends.to_vec(),
        set_priority,
    })
}

impl From<comfy_types::CancellationError> for AttentionError {
    fn from(_: comfy_types::CancellationError) -> Self {
        Self::Cancelled
    }
}

pub fn scaled_dot_product_attention_with_context(
    backend: &CpuBackend,
    request: AttentionRequest,
    query: &[f32],
    key: &[f32],
    value: &[f32],
    mask: Option<AttentionMask<'_>>,
    context: &ExecutionContext<'_>,
) -> Result<AttentionOutcome, AttentionError> {
    let prepared = prepare_attention(request, query, key, value, mask, context.cancellation)?;
    let output = prepared
        .invocation
        .execute_with_context(backend, prepared.query_chunk_size, context)
        .map_err(|error| map_kernel_error(error, request.backend))?;
    Ok(prepared.outcome(output))
}

pub fn scaled_dot_product_attention_tensor_with_context(
    backend: &CpuBackend,
    request: AttentionRequest,
    query: &Tensor,
    key: &Tensor,
    value: &Tensor,
    mask: Option<AttentionMask<'_>>,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, AttentionError> {
    context.cancellation.check()?;
    validate_tensor_inputs(request, query, key, value, context)?;

    let query_values = tensor_to_f32_workspace(backend, query, context)?;
    let key_values = tensor_to_f32_workspace(backend, key, context)?;
    let value_values = tensor_to_f32_workspace(backend, value, context)?;
    let prepared = prepare_attention(
        request,
        &query_values,
        &key_values,
        &value_values,
        mask,
        context.cancellation,
    )?;
    let output_length = checked_product(&[
        request.batch,
        request.query_tokens,
        request.heads,
        request.value_dimension,
    ])?;
    let output_temporary_bytes = checked_bytes::<f32>(output_length)?;
    let output_workspace = backend.reserve_workspace(context, output_temporary_bytes)?;
    let output = prepared
        .invocation
        .execute_with_context(backend, prepared.query_chunk_size, context)
        .map_err(|error| map_kernel_error(error, request.backend))?;
    drop(query_values);
    drop(key_values);
    drop(value_values);

    let dtype = query.descriptor().dtype();
    let encoded_length = output_length
        .checked_mul(
            usize::try_from(dtype.byte_width()).map_err(|_| AttentionError::ShapeOverflow)?,
        )
        .ok_or(AttentionError::ShapeOverflow)?;
    let mut encoded = backend.workspace_vec::<u8>(context, encoded_length)?;
    for value in output {
        context.cancellation.check()?;
        for byte in dtype.encode_scalar(
            Scalar::Float(f64::from(value)),
            "zed.comfy-model.scaled-dot-product-attention",
            query.descriptor().device(),
        )? {
            encoded.try_push(byte)?;
        }
    }
    context.cancellation.check()?;
    let output_shape = expected_tensor_shape(
        request.batch,
        request.query_tokens,
        request.heads,
        request.value_dimension,
    )?;
    let descriptor = TensorDescriptor::contiguous(
        output_shape,
        dtype,
        query.descriptor().device(),
        query.descriptor().stream(),
    )?;
    let tensor = backend.upload_bytes(descriptor, &encoded, context)?.0;
    drop(encoded);
    drop(output_workspace);
    context.cancellation.check()?;
    Ok(tensor)
}

fn validate_tensor_inputs(
    request: AttentionRequest,
    query: &Tensor,
    key: &Tensor,
    value: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<(), AttentionError> {
    let expected_query = expected_tensor_shape(
        request.batch,
        request.query_tokens,
        request.heads,
        request.head_dimension,
    )?;
    let expected_key = expected_tensor_shape(
        request.batch,
        request.key_tokens,
        request.heads,
        request.head_dimension,
    )?;
    let expected_value = expected_tensor_shape(
        request.batch,
        request.key_tokens,
        request.heads,
        request.value_dimension,
    )?;
    for (name, tensor, expected) in [
        ("query", query, expected_query),
        ("key", key, expected_key),
        ("value", value, expected_value),
    ] {
        if tensor.descriptor().shape() != expected {
            return Err(AttentionError::TensorShape {
                name,
                expected,
                actual: tensor.descriptor().shape().to_vec(),
            });
        }
        if tensor.descriptor().device() != DeviceId::CPU {
            return Err(TensorError::DeviceMismatch {
                expected: DeviceId::CPU,
                actual: tensor.descriptor().device(),
            }
            .into());
        }
        if tensor.descriptor().stream() != context.stream {
            return Err(TensorError::StreamMismatch {
                expected: context.stream,
                actual: tensor.descriptor().stream(),
            }
            .into());
        }
    }
    let dtype = query.descriptor().dtype();
    if !matches!(dtype, DType::F32 | DType::F16 | DType::Bf16) {
        return Err(AttentionError::UnsupportedTensorDType { dtype });
    }
    for tensor in [key, value] {
        if tensor.descriptor().dtype() != dtype {
            return Err(TensorError::DTypeMismatch {
                expected: dtype,
                actual: tensor.descriptor().dtype(),
            }
            .into());
        }
        if tensor.descriptor().device() != query.descriptor().device() {
            return Err(TensorError::DeviceMismatch {
                expected: query.descriptor().device(),
                actual: tensor.descriptor().device(),
            }
            .into());
        }
        if tensor.descriptor().stream() != query.descriptor().stream() {
            return Err(TensorError::StreamMismatch {
                expected: query.descriptor().stream(),
                actual: tensor.descriptor().stream(),
            }
            .into());
        }
    }
    Ok(())
}

fn expected_tensor_shape(
    batch: usize,
    tokens: usize,
    heads: usize,
    dimension: usize,
) -> Result<Vec<u64>, AttentionError> {
    [batch, tokens, heads, dimension]
        .into_iter()
        .map(|value| u64::try_from(value).map_err(|_| AttentionError::ShapeOverflow))
        .collect()
}

fn tensor_to_f32_workspace(
    backend: &CpuBackend,
    tensor: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<comfy_tensor::CpuWorkspaceVec<f32>, AttentionError> {
    let length = usize::try_from(tensor.descriptor().element_count()?)
        .map_err(|_| AttentionError::ShapeOverflow)?;
    let mut values = backend.workspace_vec(context, length)?;
    for index in 0..length {
        context.cancellation.check()?;
        let index = u64::try_from(index).map_err(|_| AttentionError::ShapeOverflow)?;
        let value = match tensor
            .descriptor()
            .dtype()
            .decode_scalar(tensor.linear_element_bytes(index)?)?
        {
            DecodedScalar::Real(value) => value as f32,
            _ => {
                return Err(AttentionError::UnsupportedTensorDType {
                    dtype: tensor.descriptor().dtype(),
                });
            }
        };
        values.try_push(value)?;
    }
    Ok(values)
}

fn checked_product(values: &[usize]) -> Result<usize, AttentionError> {
    values.iter().try_fold(1_usize, |product, value| {
        product
            .checked_mul(*value)
            .ok_or(AttentionError::ShapeOverflow)
    })
}

fn checked_bytes<T>(elements: usize) -> Result<u64, AttentionError> {
    u64::try_from(elements)
        .ok()
        .and_then(|elements| {
            u64::try_from(std::mem::size_of::<T>())
                .ok()
                .and_then(|width| elements.checked_mul(width))
        })
        .ok_or(AttentionError::ShapeOverflow)
}

struct PreparedAttention<'a> {
    invocation: CheckedAttentionInvocation<'a>,
    requested_backend: AttentionBackend,
    effective_backend: AttentionBackend,
    fallback_reason: Option<String>,
    query_chunk_size: usize,
    peak_workspace_bytes: usize,
}

impl PreparedAttention<'_> {
    fn outcome(self, values: Vec<f32>) -> AttentionOutcome {
        AttentionOutcome {
            values,
            requested_backend: self.requested_backend,
            effective_backend: self.effective_backend,
            fallback_reason: self.fallback_reason,
            query_chunk_size: self.query_chunk_size,
            peak_workspace_bytes: self.peak_workspace_bytes,
        }
    }
}

fn prepare_attention<'a>(
    request: AttentionRequest,
    query: &'a [f32],
    key: &'a [f32],
    value: &'a [f32],
    mask: Option<AttentionMask<'a>>,
    cancellation: &CancellationToken,
) -> Result<PreparedAttention<'a>, AttentionError> {
    let tensor_mask = mask.map(map_mask);
    let invocation = CheckedAttentionInvocation::new(
        AttentionKernelRequest {
            kind: AttentionKernelKind::ReferenceSdp,
            device: DeviceId::CPU,
            layout: TensorAttentionLayout::Nhd,
            shape: AttentionShape {
                batch: request.batch,
                query_tokens: request.query_tokens,
                key_tokens: request.key_tokens,
                heads: request.heads,
                head_dimension: request.head_dimension,
                value_dimension: request.value_dimension,
            },
            scale: request.scale,
            causal: false,
            dropout_probability: 0.0,
        },
        query,
        key,
        value,
        tensor_mask,
    )
    .map_err(|error| map_kernel_error(error, request.backend))?;
    cancellation.check()?;
    let (effective_backend, fallback_reason) = select_backend(request.backend, request.fallback)?;
    let score_row_bytes = invocation
        .score_row_bytes()
        .map_err(|error| map_kernel_error(error, request.backend))?;
    if request.workspace_limit_bytes < score_row_bytes {
        return Err(AttentionError::WorkspaceTooSmall {
            required: score_row_bytes,
            available: request.workspace_limit_bytes,
        });
    }
    let maximum_rows = request.workspace_limit_bytes / score_row_bytes;
    let query_chunk_size = if effective_backend == AttentionBackend::SplitOrSubQuadratic {
        maximum_rows.max(1).min(request.query_tokens)
    } else {
        1
    };
    Ok(PreparedAttention {
        invocation,
        requested_backend: request.backend,
        effective_backend,
        fallback_reason,
        query_chunk_size,
        peak_workspace_bytes: score_row_bytes,
    })
}

fn select_backend(
    backend: AttentionBackend,
    fallback: AttentionFallbackPolicy,
) -> Result<(AttentionBackend, Option<String>), AttentionError> {
    match backend {
        AttentionBackend::PytorchSdp | AttentionBackend::SplitOrSubQuadratic => Ok((backend, None)),
        AttentionBackend::SageOrFlash | AttentionBackend::Xformers
            if fallback == AttentionFallbackPolicy::AllowExactNative =>
        {
            Ok((
                AttentionBackend::PytorchSdp,
                Some(format!(
                    "{} has no certified native kernel on this device; used exact native SDP",
                    backend.feature_id()
                )),
            ))
        }
        AttentionBackend::SageOrFlash | AttentionBackend::Xformers => {
            Err(AttentionError::UnsupportedBackend {
                backend,
                reason: "no certified native kernel is registered for this device".to_owned(),
            })
        }
    }
}

fn map_mask(mask: AttentionMask<'_>) -> TensorAttentionMask<'_> {
    match mask {
        AttentionMask::Boolean { values, shape } => TensorAttentionMask::Boolean {
            values,
            shape: map_mask_shape(shape),
        },
        AttentionMask::Additive { values, shape } => TensorAttentionMask::Additive {
            values,
            shape: map_mask_shape(shape),
        },
    }
}

const fn map_mask_shape(shape: AttentionMaskShape) -> TensorAttentionMaskShape {
    match shape {
        AttentionMaskShape::KeyTokens => TensorAttentionMaskShape::KeyTokens,
        AttentionMaskShape::QueryByKey => TensorAttentionMaskShape::QueryByKey,
        AttentionMaskShape::BatchQueryByKey => TensorAttentionMaskShape::BatchQueryByKey,
        AttentionMaskShape::BatchHeadQueryByKey => TensorAttentionMaskShape::BatchHeadQueryByKey,
    }
}

fn map_kernel_error(error: AttentionKernelError, backend: AttentionBackend) -> AttentionError {
    match error {
        AttentionKernelError::Tensor(error) => AttentionError::Tensor(error),
        AttentionKernelError::ShapeOverflow => AttentionError::ShapeOverflow,
        AttentionKernelError::AllocationFailed { name } => {
            AttentionError::AllocationFailed { name }
        }
        AttentionKernelError::EmptyDimension { name } => AttentionError::EmptyDimension { name },
        AttentionKernelError::ValueCount {
            name,
            expected,
            actual,
        } => AttentionError::ValueCount {
            name,
            expected,
            actual,
        },
        AttentionKernelError::MaskValueCount { expected, actual } => {
            AttentionError::MaskValueCount { expected, actual }
        }
        AttentionKernelError::InvalidScale => AttentionError::InvalidScale,
        AttentionKernelError::Cancelled => AttentionError::Cancelled,
        AttentionKernelError::UnsupportedDropout
        | AttentionKernelError::UnsupportedLayout { .. }
        | AttentionKernelError::UnsupportedMask { .. }
        | AttentionKernelError::UnsupportedDevice { .. }
        | AttentionKernelError::GradientValueCount { .. } => AttentionError::UnsupportedBackend {
            backend,
            reason: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tensor(
        backend: &CpuBackend,
        dtype: DType,
        shape: Vec<u64>,
        values: &[f32],
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        let mut bytes = Vec::new();
        for value in values {
            bytes.extend_from_slice(&dtype.encode_scalar(
                Scalar::Float(f64::from(*value)),
                "zed.comfy-model.attention-test",
                DeviceId::CPU,
            )?);
        }
        let descriptor = TensorDescriptor::contiguous(shape, dtype, DeviceId::CPU, context.stream)?;
        Ok(backend.upload_bytes(descriptor, &bytes, context)?.0)
    }

    fn real_values(tensor: &Tensor) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let length = tensor.descriptor().element_count()?;
        let mut values = Vec::new();
        for index in 0..length {
            match tensor
                .descriptor()
                .dtype()
                .decode_scalar(tensor.linear_element_bytes(index)?)?
            {
                DecodedScalar::Real(value) => values.push(value as f32),
                value => return Err(format!("expected real tensor value, got {value:?}").into()),
            }
        }
        Ok(values)
    }

    fn request(backend: AttentionBackend) -> AttentionRequest {
        AttentionRequest {
            backend,
            fallback: AttentionFallbackPolicy::AllowExactNative,
            batch: 1,
            query_tokens: 2,
            key_tokens: 2,
            heads: 1,
            head_dimension: 2,
            value_dimension: 2,
            scale: Some(1.0),
            workspace_limit_bytes: 8,
        }
    }

    #[test]
    fn exact_backends_preserve_masks_and_explicit_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        let query = [1.0, 0.0, 0.0, 1.0];
        let key = query;
        let value = [2.0, 0.0, 0.0, 4.0];
        let mask_values = [true, false, true, true];
        let mask = Some(AttentionMask::Boolean {
            values: &mask_values,
            shape: AttentionMaskShape::QueryByKey,
        });
        let token = CancellationToken::default();
        let (backend, workspace_authority) =
            comfy_tensor::CpuWorkspaceAuthority::create_backend(1024)?;
        let context = backend.execution_context(
            comfy_tensor::StreamId::DEFAULT,
            workspace_authority.authorize_workspace(8)?,
            &token,
        );
        let exact = scaled_dot_product_attention_with_context(
            &backend,
            request(AttentionBackend::PytorchSdp),
            &query,
            &key,
            &value,
            mask,
            &context,
        )
        .expect("native SDP succeeds");
        assert_eq!(&exact.values[..2], &[2.0, 0.0]);
        let split = scaled_dot_product_attention_with_context(
            &backend,
            request(AttentionBackend::SplitOrSubQuadratic),
            &query,
            &key,
            &value,
            mask,
            &context,
        )
        .expect("split attention succeeds");
        assert_eq!(split.values, exact.values);
        let fallback = scaled_dot_product_attention_with_context(
            &backend,
            request(AttentionBackend::Xformers),
            &query,
            &key,
            &value,
            mask,
            &context,
        )
        .expect("explicit exact fallback succeeds");
        assert_eq!(fallback.effective_backend, AttentionBackend::PytorchSdp);
        assert!(fallback.fallback_reason.is_some());
        Ok(())
    }

    #[test]
    fn model_attention_adapter_preserves_exact_workspace_authority()
    -> Result<(), Box<dyn std::error::Error>> {
        let (backend, workspace_authority) =
            comfy_tensor::CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let cancellation = CancellationToken::default();
        let query = [1.0, 0.0, 0.0, 1.0];
        let key = query;
        let value = [2.0, 0.0, 0.0, 4.0];
        let required = 2 * u64::try_from(std::mem::size_of::<f32>())?;
        let context = backend.execution_context(
            comfy_tensor::StreamId::DEFAULT,
            workspace_authority.authorize_workspace(required)?,
            &cancellation,
        );
        let outcome = scaled_dot_product_attention_with_context(
            &backend,
            request(AttentionBackend::PytorchSdp),
            &query,
            &key,
            &value,
            None,
            &context,
        )?;
        assert_eq!(outcome.values.len(), 4);
        assert_eq!(context.scratch.peak_bytes(), required);
        assert_eq!(context.scratch.in_use_bytes(), 0);

        let insufficient = backend.execution_context(
            comfy_tensor::StreamId::DEFAULT,
            workspace_authority.authorize_workspace(required - 1)?,
            &cancellation,
        );
        assert!(
            scaled_dot_product_attention_with_context(
                &backend,
                request(AttentionBackend::PytorchSdp),
                &query,
                &key,
                &value,
                None,
                &insufficient,
            )
            .is_err()
        );
        assert_eq!(insufficient.scratch.in_use_bytes(), 0);

        let cancelled = CancellationToken::default();
        cancelled.cancel();
        let cancelled_context = backend.execution_context(
            comfy_tensor::StreamId::DEFAULT,
            workspace_authority.authorize_workspace(required)?,
            &cancelled,
        );
        assert!(
            scaled_dot_product_attention_with_context(
                &backend,
                request(AttentionBackend::PytorchSdp),
                &query,
                &key,
                &value,
                None,
                &cancelled_context,
            )
            .is_err()
        );
        assert_eq!(cancelled_context.scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn tensor_attention_supports_self_and_cross_attention_with_explicit_scale()
    -> Result<(), Box<dyn std::error::Error>> {
        let (backend, workspace_authority) =
            comfy_tensor::CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let cancellation = CancellationToken::default();
        let context = backend.execution_context(
            comfy_tensor::StreamId::DEFAULT,
            workspace_authority.authorize_workspace(256)?,
            &cancellation,
        );
        let query = tensor(
            &backend,
            DType::F32,
            vec![1, 2, 1, 2],
            &[1.0, 0.0, 0.0, 1.0],
            &context,
        )?;
        let key = tensor(
            &backend,
            DType::F32,
            vec![1, 3, 1, 2],
            &[1.0, 0.0, 0.0, 1.0, -1.0, 0.0],
            &context,
        )?;
        let value = tensor(
            &backend,
            DType::F32,
            vec![1, 3, 1, 2],
            &[1.0, 0.0, 0.0, 2.0, 3.0, 0.0],
            &context,
        )?;
        let input_storage = [query.storage_id(), key.storage_id(), value.storage_id()];
        let cross = scaled_dot_product_attention_tensor_with_context(
            &backend,
            AttentionRequest {
                backend: AttentionBackend::SplitOrSubQuadratic,
                fallback: AttentionFallbackPolicy::AllowExactNative,
                batch: 1,
                query_tokens: 2,
                key_tokens: 3,
                heads: 1,
                head_dimension: 2,
                value_dimension: 2,
                scale: Some(0.5),
                workspace_limit_bytes: 12,
            },
            &query,
            &key,
            &value,
            None,
            &context,
        )?;
        assert_eq!(cross.descriptor().shape(), &[1, 2, 1, 2]);
        assert_eq!(cross.descriptor().dtype(), DType::F32);
        assert_eq!(cross.descriptor().device(), DeviceId::CPU);
        assert_eq!(cross.descriptor().stream(), context.stream);
        assert!(!input_storage.contains(&cross.storage_id()));
        assert_eq!(
            [query.storage_id(), key.storage_id(), value.storage_id()],
            input_storage
        );
        let cross_values = real_values(&cross)?;
        let first_denominator = 0.5_f32.exp() + 1.0 + (-0.5_f32).exp();
        let expected_first = (0.5_f32.exp() + 3.0 * (-0.5_f32).exp()) / first_denominator;
        assert!(
            cross_values
                .first()
                .is_some_and(|value| (*value - expected_first).abs() < 1.0e-6)
        );
        assert!(
            cross_values
                .get(1)
                .is_some_and(|value| (*value - (2.0 / first_denominator)).abs() < 1.0e-6)
        );

        let self_attention = scaled_dot_product_attention_tensor_with_context(
            &backend,
            AttentionRequest {
                backend: AttentionBackend::PytorchSdp,
                fallback: AttentionFallbackPolicy::AllowExactNative,
                batch: 1,
                query_tokens: 2,
                key_tokens: 2,
                heads: 1,
                head_dimension: 2,
                value_dimension: 2,
                scale: Some(1.0),
                workspace_limit_bytes: 8,
            },
            &query,
            &query,
            &query,
            None,
            &context,
        )?;
        assert_eq!(self_attention.descriptor().shape(), &[1, 2, 1, 2]);
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }

    #[test]
    fn tensor_attention_preserves_supported_float_dtypes_and_accounts_temporaries()
    -> Result<(), Box<dyn std::error::Error>> {
        for dtype in [DType::F32, DType::F16, DType::Bf16] {
            let (backend, workspace_authority) =
                comfy_tensor::CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
            let cancellation = CancellationToken::default();
            let context = backend.execution_context(
                comfy_tensor::StreamId::DEFAULT,
                workspace_authority.authorize_workspace(92)?,
                &cancellation,
            );
            let query = tensor(
                &backend,
                dtype,
                vec![1, 2, 1, 2],
                &[1.0, 0.0, 0.0, 1.0],
                &context,
            )?;
            let key = tensor(
                &backend,
                dtype,
                vec![1, 3, 1, 2],
                &[1.0, 0.0, 0.0, 1.0, -1.0, 0.0],
                &context,
            )?;
            let value = tensor(
                &backend,
                dtype,
                vec![1, 3, 1, 2],
                &[1.0, 0.0, 0.0, 2.0, 3.0, 0.0],
                &context,
            )?;
            let output = scaled_dot_product_attention_tensor_with_context(
                &backend,
                AttentionRequest {
                    backend: AttentionBackend::SplitOrSubQuadratic,
                    fallback: AttentionFallbackPolicy::AllowExactNative,
                    batch: 1,
                    query_tokens: 2,
                    key_tokens: 3,
                    heads: 1,
                    head_dimension: 2,
                    value_dimension: 2,
                    scale: Some(1.0),
                    workspace_limit_bytes: 12,
                },
                &query,
                &key,
                &value,
                None,
                &context,
            )?;
            assert_eq!(output.descriptor().dtype(), dtype);
            assert_eq!(context.scratch.peak_bytes(), 92);
            assert_eq!(context.scratch.in_use_bytes(), 0);
        }
        Ok(())
    }

    #[test]
    fn tensor_attention_releases_workspace_on_exhaustion_and_cancellation()
    -> Result<(), Box<dyn std::error::Error>> {
        let (backend, workspace_authority) =
            comfy_tensor::CpuWorkspaceAuthority::create_backend(1024 * 1024)?;
        let cancellation = CancellationToken::default();
        let context = backend.execution_context(
            comfy_tensor::StreamId::DEFAULT,
            workspace_authority.authorize_workspace(91)?,
            &cancellation,
        );
        let query = tensor(
            &backend,
            DType::F32,
            vec![1, 2, 1, 2],
            &[1.0, 0.0, 0.0, 1.0],
            &context,
        )?;
        let key = tensor(
            &backend,
            DType::F32,
            vec![1, 3, 1, 2],
            &[1.0, 0.0, 0.0, 1.0, -1.0, 0.0],
            &context,
        )?;
        let value = tensor(
            &backend,
            DType::F32,
            vec![1, 3, 1, 2],
            &[1.0, 0.0, 0.0, 2.0, 3.0, 0.0],
            &context,
        )?;
        let request = AttentionRequest {
            backend: AttentionBackend::SplitOrSubQuadratic,
            fallback: AttentionFallbackPolicy::AllowExactNative,
            batch: 1,
            query_tokens: 2,
            key_tokens: 3,
            heads: 1,
            head_dimension: 2,
            value_dimension: 2,
            scale: Some(1.0),
            workspace_limit_bytes: 12,
        };
        assert!(matches!(
            scaled_dot_product_attention_tensor_with_context(
                &backend, request, &query, &key, &value, None, &context,
            ),
            Err(AttentionError::Tensor(
                TensorError::WorkspaceAuthorizationExceeded { .. }
            ))
        ));
        assert_eq!(context.scratch.in_use_bytes(), 0);

        let cancelled = CancellationToken::default();
        cancelled.cancel();
        let cancelled_context = backend.execution_context(
            comfy_tensor::StreamId::DEFAULT,
            workspace_authority.authorize_workspace(92)?,
            &cancelled,
        );
        assert!(matches!(
            scaled_dot_product_attention_tensor_with_context(
                &backend,
                request,
                &query,
                &key,
                &value,
                None,
                &cancelled_context,
            ),
            Err(AttentionError::Cancelled)
        ));
        assert_eq!(cancelled_context.scratch.peak_bytes(), 0);
        assert_eq!(cancelled_context.scratch.in_use_bytes(), 0);
        Ok(())
    }
}
