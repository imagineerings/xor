use crate::{SamplingPlan, SamplingProfile, SamplingProfileError};
use comfy_model::conditioning::{
    ConditioningError, ConditioningIdentity, ConditioningSet, ConditioningValue,
    ResolvedConditioningEntry,
};
use comfy_tensor::{
    CpuBackend, DType, DeviceId, ExecutionContext, Tensor, TensorError,
    generated_elementwise_or_runtime_operation_03::ElementwiseOperand,
    generated_elementwise_or_runtime_operation_16::{
        ElementwiseRuntimePartSixteenError, add_method_with_context_exact_native,
    },
    generated_indexing_masking_01::{IndexingMaskingPartOneError, narrow_method_exact_native},
    generated_native_diffusion::{NativeDiffusionTensorError, tensor_from_f32, tensor_to_f32},
};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const GUIDANCE_ADAPTER_ID: &str = "sim.comfy.guidance.v1";
const PYTHON_MATH_ISCLOSE_RELATIVE_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuidanceBranch {
    Conditional,
    Unconditional,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuidanceOptions {
    pub disable_unconditional_skip: bool,
    pub maximum_batch_bytes: u64,
}

impl Default for GuidanceOptions {
    fn default() -> Self {
        Self {
            disable_unconditional_skip: false,
            maximum_batch_bytes: u64::MAX,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GuidanceEvaluation {
    branch: GuidanceBranch,
    conditioning_identity: ConditioningIdentity,
    conditioning_digest: String,
    entry: ResolvedConditioningEntry,
    latent: Tensor,
    sigma: f32,
    sampling_percent: f64,
}

impl GuidanceEvaluation {
    pub fn branch(&self) -> GuidanceBranch {
        self.branch
    }

    pub fn conditioning_identity(&self) -> &ConditioningIdentity {
        &self.conditioning_identity
    }

    pub fn conditioning_digest(&self) -> &str {
        &self.conditioning_digest
    }

    pub fn entry(&self) -> &ResolvedConditioningEntry {
        &self.entry
    }

    pub fn latent(&self) -> &Tensor {
        &self.latent
    }

    pub fn sigma(&self) -> f32 {
        self.sigma
    }

    pub fn sampling_percent(&self) -> f64 {
        self.sampling_percent
    }
}

pub trait GuidanceDenoiser {
    fn evaluate_batch(
        &mut self,
        evaluations: &[GuidanceEvaluation],
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<Tensor>, GuidanceError>;
}

#[derive(Clone, Debug)]
pub struct GuidancePredictions {
    conditional: Tensor,
    unconditional: Option<Tensor>,
}

impl GuidancePredictions {
    pub fn conditional(&self) -> &Tensor {
        &self.conditional
    }

    pub fn unconditional(&self) -> Option<&Tensor> {
        self.unconditional.as_ref()
    }

    pub fn replace_conditional(&mut self, conditional: Tensor) {
        self.conditional = conditional;
    }

    pub fn replace_unconditional(&mut self, unconditional: Option<Tensor>) {
        self.unconditional = unconditional;
    }
}

pub struct GuidanceHookContext<'a> {
    pub plan: &'a SamplingPlan,
    pub sigma: f32,
    pub sampling_percent: f64,
    pub conditional_identity: &'a ConditioningIdentity,
    pub unconditional_identity: &'a ConditioningIdentity,
}

pub trait GuidanceHook {
    fn pre_cfg(
        &mut self,
        _hook: &GuidanceHookContext<'_>,
        _predictions: &mut GuidancePredictions,
        _context: &ExecutionContext<'_>,
    ) -> Result<(), GuidanceError> {
        Ok(())
    }

    fn override_cfg(
        &mut self,
        _hook: &GuidanceHookContext<'_>,
        _predictions: &GuidancePredictions,
        _current: &Tensor,
        _context: &ExecutionContext<'_>,
    ) -> Result<Option<Tensor>, GuidanceError> {
        Ok(None)
    }

    fn post_cfg(
        &mut self,
        _hook: &GuidanceHookContext<'_>,
        _predictions: &GuidancePredictions,
        _current: &Tensor,
        _context: &ExecutionContext<'_>,
    ) -> Result<Option<Tensor>, GuidanceError> {
        Ok(None)
    }
}

#[derive(Clone, Debug)]
pub struct GuidanceResult {
    guided: Tensor,
    identity: String,
    denoiser_evaluations: usize,
    denoiser_batches: usize,
    unconditional_skipped: bool,
    sampling_percent: f64,
}

impl GuidanceResult {
    pub fn guided(&self) -> &Tensor {
        &self.guided
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn denoiser_evaluations(&self) -> usize {
        self.denoiser_evaluations
    }

    pub fn denoiser_batches(&self) -> usize {
        self.denoiser_batches
    }

    pub fn unconditional_skipped(&self) -> bool {
        self.unconditional_skipped
    }

    pub fn sampling_percent(&self) -> f64 {
        self.sampling_percent
    }
}

#[derive(Debug, Error)]
pub enum GuidanceError {
    #[error(transparent)]
    Conditioning(#[from] ConditioningError),
    #[error(transparent)]
    Profile(#[from] SamplingProfileError),
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    Indexing(#[from] IndexingMaskingPartOneError),
    #[error(transparent)]
    Elementwise(#[from] ElementwiseRuntimePartSixteenError),
    #[error(transparent)]
    NativeDiffusion(#[from] NativeDiffusionTensorError),
    #[error("guidance execution was cancelled")]
    Cancelled,
    #[error("guidance contract is invalid: {0}")]
    Invalid(String),
    #[error("guidance denoiser returned {actual} outputs for a batch of {expected}")]
    DenoiserOutputCount { expected: usize, actual: usize },
    #[error("guidance batch requires {required} bytes, above its {limit}-byte limit")]
    BatchMemoryLimit { required: u64, limit: u64 },
    #[error("guidance shape arithmetic overflowed while computing {0}")]
    ShapeOverflow(&'static str),
    #[error("guidance has no active conditioning at sampling percent {0}")]
    NoActiveConditioning(f64),
    #[error("guidance regional output leaves an uncovered latent element")]
    UncoveredRegion,
}

pub fn sampling_percent_for_sigma(
    profile: &dyn SamplingProfile,
    sigma: f32,
) -> Result<f64, GuidanceError> {
    if sigma == 0.0 {
        return Ok(1.0);
    }
    let model_time = profile.model_time_for_sigma(sigma)?;
    let maximum_model_time = profile
        .sigma_count()
        .checked_sub(1)
        .ok_or(GuidanceError::ShapeOverflow("sampling profile extent"))?;
    if maximum_model_time == 0 {
        return Err(GuidanceError::Invalid(
            "sampling profile requires at least two sigma points".to_owned(),
        ));
    }
    Ok((1.0 - f64::from(model_time) / maximum_model_time as f64).clamp(0.0, 1.0))
}

pub fn execute_guidance(
    backend: &CpuBackend,
    latent: &Tensor,
    sigma: f32,
    profile: &dyn SamplingProfile,
    plan: &SamplingPlan,
    conditional: &ConditioningSet,
    unconditional: &ConditioningSet,
    options: GuidanceOptions,
    denoiser: &mut dyn GuidanceDenoiser,
    hooks: &mut [&mut dyn GuidanceHook],
    context: &ExecutionContext<'_>,
) -> Result<GuidanceResult, GuidanceError> {
    check_cancel(context)?;
    validate_boundary(latent, profile, plan, conditional, unconditional, options)?;
    let sampling_percent = sampling_percent_for_sigma(profile, sigma)?;
    let unconditional_skipped =
        !options.disable_unconditional_skip && source_math_isclose_to_one(plan.guidance());

    let mut evaluations = build_evaluations(
        backend,
        latent,
        sigma,
        sampling_percent,
        GuidanceBranch::Conditional,
        conditional,
        context,
    )?;
    if evaluations.is_empty() {
        return Err(GuidanceError::NoActiveConditioning(sampling_percent));
    }
    if !unconditional_skipped {
        let unconditional_evaluations = build_evaluations(
            backend,
            latent,
            sigma,
            sampling_percent,
            GuidanceBranch::Unconditional,
            unconditional,
            context,
        )?;
        if unconditional_evaluations.is_empty() {
            return Err(GuidanceError::NoActiveConditioning(sampling_percent));
        }
        evaluations
            .try_reserve_exact(unconditional_evaluations.len())
            .map_err(|_| GuidanceError::ShapeOverflow("guidance evaluations"))?;
        evaluations.extend(unconditional_evaluations);
    }

    let (outputs, denoiser_batches) = evaluate_in_compatible_batches(
        &evaluations,
        options.maximum_batch_bytes,
        denoiser,
        context,
    )?;
    check_cancel(context)?;

    let conditional_prediction = accumulate_branch(
        backend,
        latent,
        GuidanceBranch::Conditional,
        &evaluations,
        &outputs,
        context,
    )?;
    let unconditional_prediction = if unconditional_skipped {
        None
    } else {
        Some(accumulate_branch(
            backend,
            latent,
            GuidanceBranch::Unconditional,
            &evaluations,
            &outputs,
            context,
        )?)
    };
    let mut predictions = GuidancePredictions {
        conditional: conditional_prediction,
        unconditional: unconditional_prediction,
    };
    let hook_context = GuidanceHookContext {
        plan,
        sigma,
        sampling_percent,
        conditional_identity: conditional.identity(),
        unconditional_identity: unconditional.identity(),
    };

    for hook in hooks.iter_mut() {
        check_cancel(context)?;
        hook.pre_cfg(&hook_context, &mut predictions, context)?;
        check_cancel(context)?;
        validate_prediction_descriptors(latent, &predictions)?;
    }
    check_cancel(context)?;
    let mut guided = default_cfg(backend, &predictions, plan.guidance(), context)?;
    for hook in hooks.iter_mut() {
        check_cancel(context)?;
        let overridden = hook.override_cfg(&hook_context, &predictions, &guided, context)?;
        check_cancel(context)?;
        if let Some(overridden) = overridden {
            require_same_descriptor(latent, &overridden, "CFG override")?;
            guided = overridden;
        }
    }
    for hook in hooks.iter_mut() {
        check_cancel(context)?;
        let post_processed = hook.post_cfg(&hook_context, &predictions, &guided, context)?;
        check_cancel(context)?;
        if let Some(post_processed) = post_processed {
            require_same_descriptor(latent, &post_processed, "post-CFG hook")?;
            guided = post_processed;
        }
    }
    check_cancel(context)?;
    let identity = guidance_identity(
        plan,
        sigma,
        conditional,
        unconditional,
        &guided,
        evaluations.len(),
        denoiser_batches,
        unconditional_skipped,
        backend,
        context,
    )?;
    check_cancel(context)?;
    Ok(GuidanceResult {
        guided,
        identity,
        denoiser_evaluations: evaluations.len(),
        denoiser_batches,
        unconditional_skipped,
        sampling_percent,
    })
}

fn validate_boundary(
    latent: &Tensor,
    profile: &dyn SamplingProfile,
    plan: &SamplingPlan,
    conditional: &ConditioningSet,
    unconditional: &ConditioningSet,
    options: GuidanceOptions,
) -> Result<(), GuidanceError> {
    if latent.descriptor().rank() < 3
        || latent.descriptor().shape().contains(&0)
        || latent.descriptor().dtype() != DType::F32
        || latent.descriptor().device() != DeviceId::CPU
    {
        return Err(GuidanceError::Invalid(
            "guidance requires a nonempty rank-three-or-greater f32 CPU latent".to_owned(),
        ));
    }
    if latent.descriptor().stream() != comfy_tensor::StreamId::DEFAULT {
        return Err(GuidanceError::Invalid(
            "guidance CPU execution currently requires the default stream".to_owned(),
        ));
    }
    if plan.profile() != profile.identity() {
        return Err(GuidanceError::Invalid(
            "sampling plan and sampling profile identities differ".to_owned(),
        ));
    }
    for (subject, identity) in [
        ("conditional", conditional.identity()),
        ("unconditional", unconditional.identity()),
    ] {
        if identity.model_family() != conditional.identity().model_family()
            || identity.latent_format() != conditional.identity().latent_format()
        {
            return Err(GuidanceError::Invalid(format!(
                "{subject} conditioning targets a different model or latent format"
            )));
        }
    }
    if options.maximum_batch_bytes == 0 {
        return Err(GuidanceError::Invalid(
            "guidance batch memory limit must be nonzero".to_owned(),
        ));
    }
    Ok(())
}

fn build_evaluations(
    backend: &CpuBackend,
    latent: &Tensor,
    sigma: f32,
    sampling_percent: f64,
    branch: GuidanceBranch,
    conditioning: &ConditioningSet,
    context: &ExecutionContext<'_>,
) -> Result<Vec<GuidanceEvaluation>, GuidanceError> {
    let resolved = conditioning.resolve(latent.descriptor(), backend, context)?;
    let mut evaluations = Vec::new();
    evaluations
        .try_reserve_exact(resolved.len())
        .map_err(|_| GuidanceError::ShapeOverflow("active conditioning evaluations"))?;
    for entry in resolved {
        check_cancel(context)?;
        if !entry.window().contains(sampling_percent) {
            continue;
        }
        let latent = crop_to_region(latent, &entry, context)?;
        evaluations.push(GuidanceEvaluation {
            branch,
            conditioning_identity: conditioning.identity().clone(),
            conditioning_digest: conditioning.digest().to_owned(),
            entry,
            latent,
            sigma,
            sampling_percent,
        });
    }
    check_cancel(context)?;
    Ok(evaluations)
}

fn crop_to_region(
    latent: &Tensor,
    entry: &ResolvedConditioningEntry,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, GuidanceError> {
    if entry.region().is_full() {
        return Ok(latent.clone());
    }
    let mut cropped = latent.clone();
    for (spatial_axis, (&offset, &size)) in entry
        .region()
        .offsets()
        .iter()
        .zip(entry.region().sizes())
        .enumerate()
    {
        check_cancel(context)?;
        let axis = i64::try_from(spatial_axis + 2)
            .map_err(|_| GuidanceError::ShapeOverflow("regional crop axis"))?;
        let offset = i64::try_from(offset)
            .map_err(|_| GuidanceError::ShapeOverflow("regional crop offset"))?;
        cropped = narrow_method_exact_native(&cropped, axis, offset, size, context.cancellation)?;
    }
    Ok(cropped)
}

fn evaluate_in_compatible_batches(
    evaluations: &[GuidanceEvaluation],
    maximum_batch_bytes: u64,
    denoiser: &mut dyn GuidanceDenoiser,
    context: &ExecutionContext<'_>,
) -> Result<(Vec<Tensor>, usize), GuidanceError> {
    let mut outputs = Vec::new();
    outputs
        .try_reserve_exact(evaluations.len())
        .map_err(|_| GuidanceError::ShapeOverflow("guidance denoiser outputs"))?;
    let mut batch_start = 0;
    let mut batch_count = 0_usize;
    while batch_start < evaluations.len() {
        check_cancel(context)?;
        let first = evaluations
            .get(batch_start)
            .ok_or(GuidanceError::ShapeOverflow("guidance batch start"))?;
        let first_bytes = evaluation_bytes(first)?;
        if first_bytes > maximum_batch_bytes {
            return Err(GuidanceError::BatchMemoryLimit {
                required: first_bytes,
                limit: maximum_batch_bytes,
            });
        }
        let mut batch_bytes = first_bytes;
        let mut batch_end = batch_start + 1;
        while let Some(candidate) = evaluations.get(batch_end) {
            let candidate_bytes = evaluation_bytes(candidate)?;
            let next_bytes = batch_bytes
                .checked_add(candidate_bytes)
                .ok_or(GuidanceError::ShapeOverflow("guidance batch bytes"))?;
            if next_bytes > maximum_batch_bytes || !evaluations_compatible(first, candidate) {
                break;
            }
            batch_bytes = next_bytes;
            batch_end += 1;
        }
        let batch = evaluations
            .get(batch_start..batch_end)
            .ok_or(GuidanceError::ShapeOverflow("guidance batch range"))?;
        let batch_outputs = denoiser.evaluate_batch(batch, context)?;
        check_cancel(context)?;
        if batch_outputs.len() != batch.len() {
            return Err(GuidanceError::DenoiserOutputCount {
                expected: batch.len(),
                actual: batch_outputs.len(),
            });
        }
        for (evaluation, output) in batch.iter().zip(&batch_outputs) {
            require_same_descriptor(evaluation.latent(), output, "denoiser output")?;
        }
        outputs.extend(batch_outputs);
        batch_count = batch_count
            .checked_add(1)
            .ok_or(GuidanceError::ShapeOverflow("guidance batch count"))?;
        batch_start = batch_end;
    }
    Ok((outputs, batch_count))
}

fn evaluation_bytes(evaluation: &GuidanceEvaluation) -> Result<u64, GuidanceError> {
    let mut bytes = evaluation.latent.descriptor().byte_len()?;
    bytes = bytes
        .checked_add(conditioning_value_bytes(evaluation.entry.value())?)
        .ok_or(GuidanceError::ShapeOverflow("guidance evaluation bytes"))?;
    if let Some(mask) = evaluation.entry.mask() {
        bytes = bytes
            .checked_add(mask.tensor().descriptor().byte_len()?)
            .ok_or(GuidanceError::ShapeOverflow(
                "guidance evaluation mask bytes",
            ))?;
    }
    Ok(bytes)
}

fn conditioning_value_bytes(value: &ConditioningValue) -> Result<u64, GuidanceError> {
    match value {
        ConditioningValue::Regular(tensor)
        | ConditioningValue::NoiseShape(tensor)
        | ConditioningValue::CrossAttention(tensor) => Ok(tensor.descriptor().byte_len()?),
        ConditioningValue::Constant(_) => Ok(0),
        ConditioningValue::List(tensors) => tensors.iter().try_fold(0_u64, |bytes, tensor| {
            bytes
                .checked_add(
                    tensor
                        .descriptor()
                        .byte_len()
                        .map_err(GuidanceError::from)?,
                )
                .ok_or(GuidanceError::ShapeOverflow(
                    "guidance conditioning-list bytes",
                ))
        }),
    }
}

fn evaluations_compatible(left: &GuidanceEvaluation, right: &GuidanceEvaluation) -> bool {
    tensor_contract_matches(&left.latent, &right.latent)
        && left.entry.value().can_concat(right.entry.value())
}

fn accumulate_branch(
    backend: &CpuBackend,
    latent: &Tensor,
    branch: GuidanceBranch,
    evaluations: &[GuidanceEvaluation],
    outputs: &[Tensor],
    context: &ExecutionContext<'_>,
) -> Result<Tensor, GuidanceError> {
    let element_count_u64 = latent.descriptor().element_count()?;
    let element_count = usize::try_from(element_count_u64)
        .map_err(|_| GuidanceError::ShapeOverflow("guidance accumulation size"))?;
    let mut sum = zero_workspace(backend, context, element_count)?;
    let mut weight = zero_workspace(backend, context, element_count)?;
    let mut default_sum = zero_workspace(backend, context, element_count)?;
    let mut default_weight = zero_workspace(backend, context, element_count)?;
    let mut saw_entry = false;
    for (evaluation, output) in evaluations.iter().zip(outputs) {
        if evaluation.branch != branch {
            continue;
        }
        saw_entry = true;
        let (target_sum, target_weight) = if evaluation.entry.is_default_region() {
            (&mut default_sum, &mut default_weight)
        } else {
            (&mut sum, &mut weight)
        };
        accumulate_entry(
            backend,
            latent,
            evaluation.entry(),
            output,
            target_sum,
            target_weight,
            context,
        )?;
    }
    if !saw_entry {
        return Err(GuidanceError::NoActiveConditioning(0.0));
    }
    let mut values = backend.workspace_vec(context, element_count)?;
    for index in 0..element_count {
        if index.is_multiple_of(1024) {
            check_cancel(context)?;
        }
        let (sum_value, weight_value) = if weight[index] != 0.0 {
            (sum[index], weight[index])
        } else if default_weight[index] != 0.0 {
            (default_sum[index], default_weight[index])
        } else {
            return Err(GuidanceError::UncoveredRegion);
        };
        let value = sum_value / weight_value;
        if !value.is_finite() {
            return Err(GuidanceError::Invalid(
                "regional guidance produced a non-finite value".to_owned(),
            ));
        }
        values.try_push(value)?;
    }
    check_cancel(context)?;
    Ok(tensor_from_f32(
        backend,
        latent.descriptor().shape(),
        &values,
        context,
    )?)
}

fn zero_workspace(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    length: usize,
) -> Result<comfy_tensor::CpuWorkspaceVec<f32>, GuidanceError> {
    let mut values = backend.workspace_vec(context, length)?;
    for _ in 0..length {
        values.try_push(0.0)?;
    }
    Ok(values)
}

fn accumulate_entry(
    backend: &CpuBackend,
    full: &Tensor,
    entry: &ResolvedConditioningEntry,
    output: &Tensor,
    sum: &mut [f32],
    weight: &mut [f32],
    context: &ExecutionContext<'_>,
) -> Result<(), GuidanceError> {
    let output_values = tensor_to_f32(backend, output, context)?;
    let mask_values = entry
        .mask()
        .map(|mask| tensor_to_f32(backend, mask.tensor(), context))
        .transpose()?;
    if let Some(mask) = &mask_values {
        if mask.len() != output_values.len() {
            return Err(GuidanceError::Invalid(
                "resolved mask and denoiser output shapes differ".to_owned(),
            ));
        }
    }
    let local_shape = output.descriptor().shape();
    let full_shape = full.descriptor().shape();
    let mut local_coordinates = Vec::new();
    local_coordinates
        .try_reserve_exact(local_shape.len())
        .map_err(|_| GuidanceError::ShapeOverflow("regional coordinates"))?;
    local_coordinates.resize(local_shape.len(), 0_u64);
    for (local_index, output_value) in output_values.iter().copied().enumerate() {
        if local_index.is_multiple_of(1024) {
            check_cancel(context)?;
        }
        decode_row_major_index(local_index, local_shape, &mut local_coordinates)?;
        let spatial_coordinates = local_coordinates
            .get(2..)
            .ok_or(GuidanceError::ShapeOverflow("regional spatial coordinates"))?;
        let mask_weight = match (entry.mask(), mask_values.as_ref()) {
            (Some(mask), Some(mask_values)) => {
                mask.feather_weight(spatial_coordinates)? * mask_values[local_index]
            }
            (None, None) => 1.0,
            _ => {
                return Err(GuidanceError::Invalid(
                    "resolved mask state is inconsistent".to_owned(),
                ));
            }
        };
        let contribution_weight = entry.strength() * mask_weight;
        if !contribution_weight.is_finite() {
            return Err(GuidanceError::Invalid(
                "regional guidance weight must be finite".to_owned(),
            ));
        }
        if contribution_weight == 0.0 {
            continue;
        }
        let full_index =
            regional_full_index(&local_coordinates, full_shape, entry.region().offsets())?;
        let sum_slot = sum
            .get_mut(full_index)
            .ok_or(GuidanceError::ShapeOverflow("regional sum index"))?;
        *sum_slot = output_value.mul_add(contribution_weight, *sum_slot);
        let weight_slot = weight
            .get_mut(full_index)
            .ok_or(GuidanceError::ShapeOverflow("regional weight index"))?;
        *weight_slot += contribution_weight;
    }
    check_cancel(context)?;
    Ok(())
}

fn decode_row_major_index(
    index: usize,
    shape: &[u64],
    coordinates: &mut [u64],
) -> Result<(), GuidanceError> {
    if shape.len() != coordinates.len() {
        return Err(GuidanceError::ShapeOverflow("regional coordinate rank"));
    }
    let mut remainder =
        u64::try_from(index).map_err(|_| GuidanceError::ShapeOverflow("regional linear index"))?;
    for (coordinate, dimension) in coordinates.iter_mut().zip(shape).rev() {
        if *dimension == 0 {
            return Err(GuidanceError::ShapeOverflow("regional zero dimension"));
        }
        *coordinate = remainder % dimension;
        remainder /= dimension;
    }
    Ok(())
}

fn regional_full_index(
    local_coordinates: &[u64],
    full_shape: &[u64],
    offsets: &[u64],
) -> Result<usize, GuidanceError> {
    if local_coordinates.len() != full_shape.len() || offsets.len() + 2 != full_shape.len() {
        return Err(GuidanceError::ShapeOverflow("regional full index rank"));
    }
    let mut index = 0_u64;
    for (axis, (&coordinate, &dimension)) in local_coordinates.iter().zip(full_shape).enumerate() {
        let coordinate = if axis < 2 {
            coordinate
        } else {
            coordinate
                .checked_add(offsets[axis - 2])
                .ok_or(GuidanceError::ShapeOverflow("regional full coordinate"))?
        };
        if coordinate >= dimension {
            return Err(GuidanceError::ShapeOverflow(
                "regional full coordinate bounds",
            ));
        }
        index = index
            .checked_mul(dimension)
            .and_then(|index| index.checked_add(coordinate))
            .ok_or(GuidanceError::ShapeOverflow("regional full linear index"))?;
    }
    usize::try_from(index).map_err(|_| GuidanceError::ShapeOverflow("regional host index"))
}

fn default_cfg(
    backend: &CpuBackend,
    predictions: &GuidancePredictions,
    scale: f32,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, GuidanceError> {
    let Some(unconditional) = predictions.unconditional() else {
        return Ok(predictions.conditional().clone());
    };
    let delta = add_method_with_context_exact_native(
        backend,
        predictions.conditional(),
        ElementwiseOperand::Tensor(unconditional),
        -1.0,
        context,
    )?;
    Ok(add_method_with_context_exact_native(
        backend,
        unconditional,
        ElementwiseOperand::Tensor(&delta),
        scale,
        context,
    )?)
}

fn validate_prediction_descriptors(
    latent: &Tensor,
    predictions: &GuidancePredictions,
) -> Result<(), GuidanceError> {
    require_same_descriptor(latent, predictions.conditional(), "conditional prediction")?;
    if let Some(unconditional) = predictions.unconditional() {
        require_same_descriptor(latent, unconditional, "unconditional prediction")?;
    }
    Ok(())
}

fn require_same_descriptor(
    expected: &Tensor,
    actual: &Tensor,
    subject: &str,
) -> Result<(), GuidanceError> {
    if !tensor_contract_matches(expected, actual) {
        return Err(GuidanceError::Invalid(format!(
            "{subject} descriptor does not match the guidance latent"
        )));
    }
    Ok(())
}

fn tensor_contract_matches(left: &Tensor, right: &Tensor) -> bool {
    left.descriptor().shape() == right.descriptor().shape()
        && left.descriptor().dtype() == right.descriptor().dtype()
        && left.descriptor().device() == right.descriptor().device()
        && left.descriptor().stream() == right.descriptor().stream()
}

#[allow(clippy::too_many_arguments)]
fn guidance_identity(
    plan: &SamplingPlan,
    sigma: f32,
    conditional: &ConditioningSet,
    unconditional: &ConditioningSet,
    guided: &Tensor,
    evaluations: usize,
    batches: usize,
    unconditional_skipped: bool,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<String, GuidanceError> {
    let mut hasher = Sha256::new();
    hasher.update(GUIDANCE_ADAPTER_ID.as_bytes());
    hash_string(&mut hasher, plan.profile().as_str())?;
    hasher.update(plan.guidance().to_bits().to_le_bytes());
    hasher.update(sigma.to_bits().to_le_bytes());
    hash_string(&mut hasher, conditional.digest())?;
    hash_string(&mut hasher, unconditional.digest())?;
    hasher.update(
        u64::try_from(evaluations)
            .map_err(|_| GuidanceError::ShapeOverflow("guidance identity evaluations"))?
            .to_le_bytes(),
    );
    hasher.update(
        u64::try_from(batches)
            .map_err(|_| GuidanceError::ShapeOverflow("guidance identity batches"))?
            .to_le_bytes(),
    );
    hasher.update([u8::from(unconditional_skipped)]);
    for value in tensor_to_f32(backend, guided, context)?.iter() {
        hasher.update(value.to_bits().to_le_bytes());
    }
    check_cancel(context)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_string(hasher: &mut Sha256, value: &str) -> Result<(), GuidanceError> {
    hasher.update(
        u64::try_from(value.len())
            .map_err(|_| GuidanceError::ShapeOverflow("guidance identity string"))?
            .to_le_bytes(),
    );
    hasher.update(value.as_bytes());
    Ok(())
}

fn check_cancel(context: &ExecutionContext<'_>) -> Result<(), GuidanceError> {
    context
        .cancellation
        .check()
        .map_err(|_| GuidanceError::Cancelled)
}

fn source_math_isclose_to_one(value: f32) -> bool {
    let value = f64::from(value);
    (value - 1.0).abs() <= PYTHON_MATH_ISCLOSE_RELATIVE_TOLERANCE * value.abs().max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DiscreteSamplingProfile, PredictionInterpretation, SamplingProfileIdentity, SamplingSession,
    };
    use comfy_model::conditioning::{
        ConditioningEntry, ConditioningEntryOptions, ConditioningMask, ConditioningRegion,
        ConditioningValue, ConditioningWindow,
    };
    use comfy_model::{LatentFormatIdentity, ModelFamilyIdentity};
    use comfy_tensor::{CancellationToken, CpuWorkspaceAuthority, StreamId, TensorDescriptor};
    use std::{cell::RefCell, error::Error, rc::Rc, sync::Arc};

    type TestResult = Result<(), Box<dyn Error>>;

    struct Harness {
        backend: CpuBackend,
        authority: CpuWorkspaceAuthority,
        cancellation: CancellationToken,
        scratch_bytes: u64,
    }

    impl Harness {
        fn new(memory_bytes: u64, scratch_bytes: u64) -> Result<Self, TensorError> {
            let (backend, authority) = CpuWorkspaceAuthority::create_backend(memory_bytes)?;
            Ok(Self {
                backend,
                authority,
                cancellation: CancellationToken::default(),
                scratch_bytes,
            })
        }

        fn context(&self) -> Result<ExecutionContext<'_>, TensorError> {
            Ok(self.backend.execution_context(
                StreamId::DEFAULT,
                self.authority.authorize_workspace(self.scratch_bytes)?,
                &self.cancellation,
            ))
        }

        fn tensor(
            &self,
            shape: &[u64],
            values: &[f32],
            context: &ExecutionContext<'_>,
        ) -> Result<Tensor, NativeDiffusionTensorError> {
            tensor_from_f32(&self.backend, shape, values, context)
        }
    }

    fn profile() -> Result<DiscreteSamplingProfile, SamplingProfileError> {
        DiscreteSamplingProfile::new(
            SamplingProfileIdentity::new("guidance-test-profile")?,
            PredictionInterpretation::Epsilon,
            Arc::from([1.0_f32, 2.0_f32]),
        )
    }

    fn plan(
        profile: &DiscreteSamplingProfile,
        guidance: f32,
    ) -> Result<SamplingPlan, crate::SamplingError> {
        SamplingPlan::new(
            "euler",
            "normal",
            profile.identity().clone(),
            7,
            1,
            guidance,
            1.0,
        )
    }

    fn identity(namespace: &str) -> Result<ConditioningIdentity, Box<dyn Error>> {
        Ok(ConditioningIdentity::new(
            namespace,
            ModelFamilyIdentity::new("COMFY-MODEL-0001", "guidance_test", "v1")?,
            LatentFormatIdentity::new("COMFY-MODEL-0002", "guidance_latent")?,
        )?)
    }

    fn conditioning(
        harness: &Harness,
        namespace: &str,
        entries: Vec<(&str, ConditioningEntryOptions)>,
        context: &ExecutionContext<'_>,
    ) -> Result<ConditioningSet, Box<dyn Error>> {
        let value = harness.tensor(&[1, 1], &[1.0], context)?;
        let mut checked = Vec::new();
        checked.try_reserve_exact(entries.len())?;
        for (identifier, options) in entries {
            checked.push(ConditioningEntry::checked(
                identifier,
                ConditioningValue::regular(value.clone())?,
                options,
            )?);
        }
        Ok(ConditioningSet::checked(
            identity(namespace)?,
            checked,
            &harness.cancellation,
        )?)
    }

    struct ValueDenoiser<'a> {
        backend: &'a CpuBackend,
        batches: Vec<Vec<GuidanceBranch>>,
        cancel_after_evaluation: bool,
    }

    impl GuidanceDenoiser for ValueDenoiser<'_> {
        fn evaluate_batch(
            &mut self,
            evaluations: &[GuidanceEvaluation],
            context: &ExecutionContext<'_>,
        ) -> Result<Vec<Tensor>, GuidanceError> {
            self.batches
                .push(evaluations.iter().map(GuidanceEvaluation::branch).collect());
            let mut outputs = Vec::new();
            outputs
                .try_reserve_exact(evaluations.len())
                .map_err(|_| GuidanceError::ShapeOverflow("test denoiser outputs"))?;
            for evaluation in evaluations {
                let value = match evaluation.entry().identifier() {
                    "left" => 2.0,
                    "right" => 4.0,
                    "strong" => 6.0,
                    "default" => 8.0,
                    _ => match evaluation.branch() {
                        GuidanceBranch::Conditional => 3.0,
                        GuidanceBranch::Unconditional => 1.0,
                    },
                };
                let count = usize::try_from(evaluation.latent().descriptor().element_count()?)
                    .map_err(|_| GuidanceError::ShapeOverflow("test output"))?;
                let values = vec![value; count];
                outputs.push(tensor_from_f32(
                    self.backend,
                    evaluation.latent().descriptor().shape(),
                    &values,
                    context,
                )?);
            }
            if self.cancel_after_evaluation {
                context.cancellation.cancel();
            }
            Ok(outputs)
        }
    }

    fn values(
        harness: &Harness,
        tensor: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<f32>, NativeDiffusionTensorError> {
        Ok(tensor_to_f32(&harness.backend, tensor, context)?.to_vec())
    }

    fn full_options() -> ConditioningEntryOptions {
        ConditioningEntryOptions::default()
    }

    #[test]
    fn cfg_scale_zero_one_and_above_one_follow_exact_formula() -> TestResult {
        for (scale, expected, skipped) in [(0.0, 1.0, false), (1.0, 3.0, true), (2.5, 6.0, false)] {
            let harness = Harness::new(1 << 20, 1 << 18)?;
            let context = harness.context()?;
            let latent = harness.tensor(&[1, 1, 2], &[0.0, 0.0], &context)?;
            let conditional = conditioning(
                &harness,
                "positive",
                vec![("full", full_options())],
                &context,
            )?;
            let unconditional = conditioning(
                &harness,
                "negative",
                vec![("full", full_options())],
                &context,
            )?;
            let profile = profile()?;
            let plan = plan(&profile, scale)?;
            let mut denoiser = ValueDenoiser {
                backend: &harness.backend,
                batches: Vec::new(),
                cancel_after_evaluation: false,
            };
            let result = execute_guidance(
                &harness.backend,
                &latent,
                2.0,
                &profile,
                &plan,
                &conditional,
                &unconditional,
                GuidanceOptions::default(),
                &mut denoiser,
                &mut [],
                &context,
            )?;
            assert_eq!(
                values(&harness, result.guided(), &context)?,
                vec![expected; 2]
            );
            assert_eq!(result.unconditional_skipped(), skipped);
        }
        Ok(())
    }

    #[test]
    fn source_exact_skip_can_be_disabled_and_batching_respects_memory() -> TestResult {
        let harness = Harness::new(1 << 20, 1 << 18)?;
        let context = harness.context()?;
        let latent = harness.tensor(&[1, 1, 2], &[0.0, 0.0], &context)?;
        let conditional = conditioning(
            &harness,
            "positive",
            vec![("full", full_options())],
            &context,
        )?;
        let unconditional = conditioning(
            &harness,
            "negative",
            vec![("full", full_options())],
            &context,
        )?;
        let profile = profile()?;
        let adjacent_to_one = f32::from_bits(1.0_f32.to_bits() + 1);
        let adjacent_plan = plan(&profile, adjacent_to_one)?;
        let mut denoiser = ValueDenoiser {
            backend: &harness.backend,
            batches: Vec::new(),
            cancel_after_evaluation: false,
        };
        let adjacent_result = execute_guidance(
            &harness.backend,
            &latent,
            2.0,
            &profile,
            &adjacent_plan,
            &conditional,
            &unconditional,
            GuidanceOptions::default(),
            &mut denoiser,
            &mut [],
            &context,
        )?;
        assert!(!adjacent_result.unconditional_skipped());

        let plan = plan(&profile, 1.0)?;
        denoiser.batches.clear();
        let options = GuidanceOptions {
            disable_unconditional_skip: true,
            maximum_batch_bytes: 16,
        };
        let result = execute_guidance(
            &harness.backend,
            &latent,
            2.0,
            &profile,
            &plan,
            &conditional,
            &unconditional,
            options,
            &mut denoiser,
            &mut [],
            &context,
        )?;
        assert!(!result.unconditional_skipped());
        assert_eq!(result.denoiser_batches(), 2);
        assert_eq!(
            denoiser.batches,
            vec![
                vec![GuidanceBranch::Conditional],
                vec![GuidanceBranch::Unconditional]
            ]
        );
        Ok(())
    }

    #[test]
    fn compatible_evaluations_share_one_batch() -> TestResult {
        let harness = Harness::new(1 << 20, 1 << 18)?;
        let context = harness.context()?;
        let latent = harness.tensor(&[1, 1, 2], &[0.0, 0.0], &context)?;
        let conditional = conditioning(
            &harness,
            "positive",
            vec![("full", full_options())],
            &context,
        )?;
        let unconditional = conditioning(
            &harness,
            "negative",
            vec![("full", full_options())],
            &context,
        )?;
        let profile = profile()?;
        let plan = plan(&profile, 2.0)?;
        let mut denoiser = ValueDenoiser {
            backend: &harness.backend,
            batches: Vec::new(),
            cancel_after_evaluation: false,
        };
        let result = execute_guidance(
            &harness.backend,
            &latent,
            2.0,
            &profile,
            &plan,
            &conditional,
            &unconditional,
            GuidanceOptions::default(),
            &mut denoiser,
            &mut [],
            &context,
        )?;
        assert_eq!(result.denoiser_batches(), 1);
        assert_eq!(denoiser.batches[0].len(), 2);
        Ok(())
    }

    #[test]
    fn regional_accumulation_normalizes_overlap_and_uses_default_fill() -> TestResult {
        let harness = Harness::new(1 << 20, 1 << 18)?;
        let context = harness.context()?;
        let latent = harness.tensor(&[1, 1, 4], &[0.0; 4], &context)?;
        let region = |sizes, offsets, strength, default_region| -> Result<_, ConditioningError> {
            Ok(ConditioningEntryOptions {
                strength,
                region: Some(ConditioningRegion::absolute(sizes, offsets)?),
                mask: None,
                window: ConditioningWindow::full(),
                default_region,
            })
        };
        let overlap = conditioning(
            &harness,
            "overlap",
            vec![
                ("left", region(vec![4], vec![0], 1.0, false)?),
                ("strong", region(vec![4], vec![0], 3.0, false)?),
            ],
            &context,
        )?;
        let fallback = conditioning(
            &harness,
            "fallback",
            vec![
                ("left", region(vec![2], vec![0], 1.0, false)?),
                (
                    "default",
                    ConditioningEntryOptions {
                        default_region: true,
                        ..full_options()
                    },
                ),
            ],
            &context,
        )?;
        let mask = harness.tensor(&[4], &[1.0, 0.0, 1.0, 0.0], &context)?;
        let masked = conditioning(
            &harness,
            "masked",
            vec![
                (
                    "left",
                    ConditioningEntryOptions {
                        mask: Some(ConditioningMask::new(mask, 1.0, vec![0], false)?),
                        ..full_options()
                    },
                ),
                (
                    "default",
                    ConditioningEntryOptions {
                        default_region: true,
                        ..full_options()
                    },
                ),
            ],
            &context,
        )?;
        let profile = profile()?;
        let plan = plan(&profile, 1.0)?;
        let unconditional = conditioning(
            &harness,
            "negative",
            vec![("full", full_options())],
            &context,
        )?;
        for (conditional, expected) in [
            (&overlap, vec![5.0; 4]),
            (&fallback, vec![2.0, 2.0, 8.0, 8.0]),
            (&masked, vec![2.0, 8.0, 2.0, 8.0]),
        ] {
            let mut denoiser = ValueDenoiser {
                backend: &harness.backend,
                batches: Vec::new(),
                cancel_after_evaluation: false,
            };
            let result = execute_guidance(
                &harness.backend,
                &latent,
                2.0,
                &profile,
                &plan,
                conditional,
                &unconditional,
                GuidanceOptions::default(),
                &mut denoiser,
                &mut [],
                &context,
            )?;
            assert_eq!(values(&harness, result.guided(), &context)?, expected);
        }
        Ok(())
    }

    struct OrderedHook {
        name: &'static str,
        events: Rc<RefCell<Vec<String>>>,
        override_with_conditional: bool,
    }

    impl GuidanceHook for OrderedHook {
        fn pre_cfg(
            &mut self,
            _hook: &GuidanceHookContext<'_>,
            _predictions: &mut GuidancePredictions,
            _context: &ExecutionContext<'_>,
        ) -> Result<(), GuidanceError> {
            self.events.borrow_mut().push(format!("pre:{}", self.name));
            Ok(())
        }

        fn override_cfg(
            &mut self,
            _hook: &GuidanceHookContext<'_>,
            predictions: &GuidancePredictions,
            _current: &Tensor,
            _context: &ExecutionContext<'_>,
        ) -> Result<Option<Tensor>, GuidanceError> {
            self.events
                .borrow_mut()
                .push(format!("override:{}", self.name));
            Ok(self
                .override_with_conditional
                .then(|| predictions.conditional().clone()))
        }

        fn post_cfg(
            &mut self,
            _hook: &GuidanceHookContext<'_>,
            _predictions: &GuidancePredictions,
            current: &Tensor,
            _context: &ExecutionContext<'_>,
        ) -> Result<Option<Tensor>, GuidanceError> {
            self.events.borrow_mut().push(format!("post:{}", self.name));
            Ok(Some(current.clone()))
        }
    }

    #[test]
    fn hooks_run_pre_override_post_in_registration_order() -> TestResult {
        let harness = Harness::new(1 << 20, 1 << 18)?;
        let context = harness.context()?;
        let latent = harness.tensor(&[1, 1, 1], &[0.0], &context)?;
        let conditional = conditioning(
            &harness,
            "positive",
            vec![("full", full_options())],
            &context,
        )?;
        let unconditional = conditioning(
            &harness,
            "negative",
            vec![("full", full_options())],
            &context,
        )?;
        let profile = profile()?;
        let plan = plan(&profile, 4.0)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut first = OrderedHook {
            name: "a",
            events: events.clone(),
            override_with_conditional: true,
        };
        let mut second = OrderedHook {
            name: "b",
            events: events.clone(),
            override_with_conditional: false,
        };
        let mut hooks: [&mut dyn GuidanceHook; 2] = [&mut first, &mut second];
        let mut denoiser = ValueDenoiser {
            backend: &harness.backend,
            batches: Vec::new(),
            cancel_after_evaluation: false,
        };
        let result = execute_guidance(
            &harness.backend,
            &latent,
            2.0,
            &profile,
            &plan,
            &conditional,
            &unconditional,
            GuidanceOptions::default(),
            &mut denoiser,
            &mut hooks,
            &context,
        )?;
        assert_eq!(values(&harness, result.guided(), &context)?, vec![3.0]);
        assert_eq!(
            &*events.borrow(),
            &[
                "pre:a",
                "pre:b",
                "override:a",
                "override:b",
                "post:a",
                "post:b"
            ]
        );
        Ok(())
    }

    #[derive(Clone, Copy, Eq, PartialEq)]
    enum CancelPhase {
        Pre,
        Override,
        Post,
    }

    struct CancellingHook {
        phase: CancelPhase,
        events: Rc<RefCell<Vec<&'static str>>>,
    }

    impl GuidanceHook for CancellingHook {
        fn pre_cfg(
            &mut self,
            _hook: &GuidanceHookContext<'_>,
            _predictions: &mut GuidancePredictions,
            context: &ExecutionContext<'_>,
        ) -> Result<(), GuidanceError> {
            self.events.borrow_mut().push("pre");
            if self.phase == CancelPhase::Pre {
                context.cancellation.cancel();
            }
            Ok(())
        }

        fn override_cfg(
            &mut self,
            _hook: &GuidanceHookContext<'_>,
            _predictions: &GuidancePredictions,
            _current: &Tensor,
            context: &ExecutionContext<'_>,
        ) -> Result<Option<Tensor>, GuidanceError> {
            self.events.borrow_mut().push("override");
            if self.phase == CancelPhase::Override {
                context.cancellation.cancel();
            }
            Ok(None)
        }

        fn post_cfg(
            &mut self,
            _hook: &GuidanceHookContext<'_>,
            _predictions: &GuidancePredictions,
            _current: &Tensor,
            context: &ExecutionContext<'_>,
        ) -> Result<Option<Tensor>, GuidanceError> {
            self.events.borrow_mut().push("post");
            if self.phase == CancelPhase::Post {
                context.cancellation.cancel();
            }
            Ok(None)
        }
    }

    #[test]
    fn cancellation_after_each_hook_phase_stops_before_the_next_phase() -> TestResult {
        for (phase, expected_events) in [
            (CancelPhase::Pre, vec!["pre"]),
            (CancelPhase::Override, vec!["pre", "override"]),
            (CancelPhase::Post, vec!["pre", "override", "post"]),
        ] {
            let harness = Harness::new(1 << 20, 1 << 18)?;
            let context = harness.context()?;
            let latent = harness.tensor(&[1, 1, 1], &[0.0], &context)?;
            let conditional = conditioning(
                &harness,
                "positive",
                vec![("full", full_options())],
                &context,
            )?;
            let unconditional = conditioning(
                &harness,
                "negative",
                vec![("full", full_options())],
                &context,
            )?;
            let profile = profile()?;
            let plan = plan(&profile, 2.0)?;
            let events = Rc::new(RefCell::new(Vec::new()));
            let mut hook = CancellingHook {
                phase,
                events: events.clone(),
            };
            let mut hooks: [&mut dyn GuidanceHook; 1] = [&mut hook];
            let mut denoiser = ValueDenoiser {
                backend: &harness.backend,
                batches: Vec::new(),
                cancel_after_evaluation: false,
            };
            assert!(matches!(
                execute_guidance(
                    &harness.backend,
                    &latent,
                    2.0,
                    &profile,
                    &plan,
                    &conditional,
                    &unconditional,
                    GuidanceOptions::default(),
                    &mut denoiser,
                    &mut hooks,
                    &context,
                ),
                Err(GuidanceError::Cancelled)
            ));
            assert_eq!(&*events.borrow(), &expected_events);
            assert_eq!(context.scratch.in_use_bytes(), 0);
        }
        Ok(())
    }

    #[test]
    fn percent_windows_and_result_identity_are_deterministic() -> TestResult {
        let harness = Harness::new(1 << 20, 1 << 18)?;
        let context = harness.context()?;
        let latent = harness.tensor(&[1, 1, 1], &[0.0], &context)?;
        let early = ConditioningEntryOptions {
            window: ConditioningWindow::new(0.0, 0.4)?,
            ..full_options()
        };
        let late = ConditioningEntryOptions {
            window: ConditioningWindow::new(0.6, 1.0)?,
            ..full_options()
        };
        let conditional = conditioning(
            &harness,
            "positive",
            vec![("left", early), ("right", late)],
            &context,
        )?;
        let unconditional = conditioning(
            &harness,
            "negative",
            vec![("full", full_options())],
            &context,
        )?;
        let profile = profile()?;
        let plan = plan(&profile, 1.0)?;
        assert_eq!(sampling_percent_for_sigma(&profile, 2.0)?, 0.0);
        assert_eq!(sampling_percent_for_sigma(&profile, 1.0)?, 1.0);
        let mut identities = Vec::new();
        for _ in 0..2 {
            let mut denoiser = ValueDenoiser {
                backend: &harness.backend,
                batches: Vec::new(),
                cancel_after_evaluation: false,
            };
            let result = execute_guidance(
                &harness.backend,
                &latent,
                2.0,
                &profile,
                &plan,
                &conditional,
                &unconditional,
                GuidanceOptions::default(),
                &mut denoiser,
                &mut [],
                &context,
            )?;
            assert_eq!(result.denoiser_evaluations(), 1);
            identities.push(result.identity().to_owned());
        }
        assert_eq!(identities[0], identities[1]);
        Ok(())
    }

    #[test]
    fn cancellation_batch_limit_dtype_and_workspace_fail_without_sampler_commit() -> TestResult {
        let harness = Harness::new(1 << 20, 1 << 18)?;
        let context = harness.context()?;
        let latent = harness.tensor(&[1, 1, 2], &[0.0, 0.0], &context)?;
        let conditional = conditioning(
            &harness,
            "positive",
            vec![("full", full_options())],
            &context,
        )?;
        let unconditional = conditioning(
            &harness,
            "negative",
            vec![("full", full_options())],
            &context,
        )?;
        let profile = profile()?;
        let plan = plan(&profile, 2.0)?;
        let session = SamplingSession::new(plan.clone(), vec![2.0, 0.0], latent.clone())?;

        let mut denoiser = ValueDenoiser {
            backend: &harness.backend,
            batches: Vec::new(),
            cancel_after_evaluation: false,
        };
        let limited = GuidanceOptions {
            maximum_batch_bytes: 1,
            ..GuidanceOptions::default()
        };
        assert!(matches!(
            execute_guidance(
                &harness.backend,
                &latent,
                2.0,
                &profile,
                &plan,
                &conditional,
                &unconditional,
                limited,
                &mut denoiser,
                &mut [],
                &context,
            ),
            Err(GuidanceError::BatchMemoryLimit { .. })
        ));
        assert_eq!(session.next_step(), 0);

        let f16_descriptor = TensorDescriptor::contiguous(
            vec![1, 1, 2],
            DType::F16,
            DeviceId::CPU,
            StreamId::DEFAULT,
        )?;
        let (f16_latent, _) = harness
            .backend
            .upload_bytes(f16_descriptor, &[0; 4], &context)?;
        assert!(matches!(
            execute_guidance(
                &harness.backend,
                &f16_latent,
                2.0,
                &profile,
                &plan,
                &conditional,
                &unconditional,
                GuidanceOptions::default(),
                &mut denoiser,
                &mut [],
                &context,
            ),
            Err(GuidanceError::Invalid(_))
        ));

        let cancelled_harness = Harness::new(1 << 20, 1 << 18)?;
        let cancelled_context = cancelled_harness.context()?;
        let cancelled_latent =
            cancelled_harness.tensor(&[1, 1, 2], &[0.0, 0.0], &cancelled_context)?;
        let cancelled_conditional = conditioning(
            &cancelled_harness,
            "positive",
            vec![("full", full_options())],
            &cancelled_context,
        )?;
        let cancelled_unconditional = conditioning(
            &cancelled_harness,
            "negative",
            vec![("full", full_options())],
            &cancelled_context,
        )?;
        let mut cancelling = ValueDenoiser {
            backend: &cancelled_harness.backend,
            batches: Vec::new(),
            cancel_after_evaluation: true,
        };
        assert!(matches!(
            execute_guidance(
                &cancelled_harness.backend,
                &cancelled_latent,
                2.0,
                &profile,
                &plan,
                &cancelled_conditional,
                &cancelled_unconditional,
                GuidanceOptions::default(),
                &mut cancelling,
                &mut [],
                &cancelled_context,
            ),
            Err(GuidanceError::Cancelled)
        ));
        assert_eq!(session.next_step(), 0);

        let low_scratch = Harness::new(1 << 20, 4)?;
        let build_context = low_scratch.backend.execution_context(
            StreamId::DEFAULT,
            low_scratch.authority.authorize_workspace(1 << 18)?,
            &low_scratch.cancellation,
        );
        let low_latent = low_scratch.tensor(&[1, 1, 2], &[0.0, 0.0], &build_context)?;
        let low_conditional = conditioning(
            &low_scratch,
            "positive",
            vec![("full", full_options())],
            &build_context,
        )?;
        let low_unconditional = conditioning(
            &low_scratch,
            "negative",
            vec![("full", full_options())],
            &build_context,
        )?;
        drop(build_context);
        let low_context = low_scratch.context()?;
        let mut low_denoiser = ValueDenoiser {
            backend: &low_scratch.backend,
            batches: Vec::new(),
            cancel_after_evaluation: false,
        };
        assert!(
            execute_guidance(
                &low_scratch.backend,
                &low_latent,
                2.0,
                &profile,
                &plan,
                &low_conditional,
                &low_unconditional,
                GuidanceOptions::default(),
                &mut low_denoiser,
                &mut [],
                &low_context,
            )
            .is_err()
        );
        assert_eq!(session.next_step(), 0);
        assert_eq!(context.scratch.in_use_bytes(), 0);
        Ok(())
    }
}
