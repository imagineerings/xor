use crate::{
    CancellationToken, CpuBackend, DecodedScalar, DeviceId, ExecutionContext, MutationWitness,
    NumericClass, Scalar, Tensor, TensorBackend, TensorError, TensorId,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

pub mod breadth;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct TapeNodeId(u64);

impl TapeNodeId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct OutputSlot {
    pub node: TapeNodeId,
    pub output: u32,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "LeafIdWire")]
pub struct LeafId(String);

#[derive(Deserialize)]
struct LeafIdWire(String);

impl TryFrom<LeafIdWire> for LeafId {
    type Error = AutogradError;

    fn try_from(value: LeafIdWire) -> Result<Self, Self::Error> {
        Self::new(value.0)
    }
}

impl LeafId {
    pub fn new(value: impl Into<String>) -> Result<Self, AutogradError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(AutogradError::InvalidGraph {
                reason: "leaf identifier is empty".to_owned(),
            });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum AutogradInput {
    Node(OutputSlot),
    Leaf(LeafId),
    Constant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradientMode {
    Enabled,
    NoGrad,
    Inference,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradScalerConfig {
    pub initial_scale: f64,
    pub growth_factor: f64,
    pub backoff_factor: f64,
    pub growth_interval: u64,
    pub enabled: bool,
}

impl Default for GradScalerConfig {
    fn default() -> Self {
        Self {
            initial_scale: 65_536.0,
            growth_factor: 2.0,
            backoff_factor: 0.5,
            growth_interval: 2_000,
            enabled: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradScalerOptimizerDecision {
    Run,
    SkipNonFinite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GradScalerStage {
    Ready,
    Unscaled { found_non_finite: bool },
    Stepped { found_non_finite: bool },
}

#[derive(Clone, Debug)]
pub struct NativeGradScaler {
    scale: f64,
    growth_factor: f64,
    backoff_factor: f64,
    growth_interval: u64,
    growth_tracker: u64,
    enabled: bool,
    stage: GradScalerStage,
}

impl NativeGradScaler {
    pub fn new(config: GradScalerConfig) -> Result<Self, GradScalerError> {
        if !config.initial_scale.is_finite()
            || config.initial_scale <= 0.0
            || !config.growth_factor.is_finite()
            || config.growth_factor <= 1.0
            || !config.backoff_factor.is_finite()
            || !(0.0..1.0).contains(&config.backoff_factor)
            || config.growth_interval == 0
        {
            return Err(GradScalerError::InvalidConfiguration);
        }
        Ok(Self {
            scale: config.initial_scale,
            growth_factor: config.growth_factor,
            backoff_factor: config.backoff_factor,
            growth_interval: config.growth_interval,
            growth_tracker: 0,
            enabled: config.enabled,
            stage: GradScalerStage::Ready,
        })
    }

    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    pub fn growth_tracker(&self) -> u64 {
        self.growth_tracker
    }

    pub fn scale_loss_exact_native(
        &self,
        loss: &Tensor,
        cancellation: &CancellationToken,
    ) -> Result<Tensor, GradScalerError> {
        if !self.enabled {
            cancellation_check_scaler(cancellation)?;
            return Ok(loss.clone());
        }
        map_floating_tensor(loss, self.scale, cancellation)
    }

    pub fn unscale_gradients_exact_native(
        &mut self,
        gradients: &mut [Tensor],
        cancellation: &CancellationToken,
    ) -> Result<bool, GradScalerError> {
        if self.stage != GradScalerStage::Ready {
            return Err(GradScalerError::InvalidStage);
        }
        cancellation_check_scaler(cancellation)?;
        if !self.enabled {
            self.stage = GradScalerStage::Unscaled {
                found_non_finite: false,
            };
            return Ok(false);
        }
        if gradients.is_empty() {
            return Err(GradScalerError::NoGradients);
        }
        let reciprocal = self.scale.recip();
        let mut staged = Vec::new();
        staged
            .try_reserve_exact(gradients.len())
            .map_err(|_| GradScalerError::AllocationFailed)?;
        let mut found_non_finite = false;
        for gradient in gradients.iter() {
            let (candidate, candidate_non_finite) =
                unscale_floating_tensor(gradient, reciprocal, cancellation)?;
            found_non_finite |= candidate_non_finite;
            staged.push(candidate);
        }
        cancellation_check_scaler(cancellation)?;
        for (gradient, candidate) in gradients.iter_mut().zip(staged) {
            gradient.commit_in_place(candidate)?;
        }
        self.stage = GradScalerStage::Unscaled { found_non_finite };
        Ok(found_non_finite)
    }

    pub fn optimizer_step_decision_exact_native(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<GradScalerOptimizerDecision, GradScalerError> {
        cancellation_check_scaler(cancellation)?;
        let GradScalerStage::Unscaled { found_non_finite } = self.stage else {
            return Err(GradScalerError::InvalidStage);
        };
        self.stage = GradScalerStage::Stepped { found_non_finite };
        Ok(if found_non_finite {
            GradScalerOptimizerDecision::SkipNonFinite
        } else {
            GradScalerOptimizerDecision::Run
        })
    }

    pub fn update_exact_native(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<(), GradScalerError> {
        cancellation_check_scaler(cancellation)?;
        let GradScalerStage::Stepped { found_non_finite } = self.stage else {
            return Err(GradScalerError::InvalidStage);
        };
        let mut next_scale = self.scale;
        let mut next_growth_tracker = self.growth_tracker;
        if self.enabled {
            if found_non_finite {
                next_scale *= self.backoff_factor;
                if next_scale == 0.0 {
                    return Err(GradScalerError::ScaleUnderflow);
                }
                next_growth_tracker = 0;
            } else {
                next_growth_tracker = next_growth_tracker
                    .checked_add(1)
                    .ok_or(GradScalerError::CounterOverflow)?;
                if next_growth_tracker >= self.growth_interval {
                    let grown_scale = next_scale * self.growth_factor;
                    if !grown_scale.is_finite() {
                        return Err(GradScalerError::ScaleOverflow);
                    }
                    next_scale = grown_scale;
                    next_growth_tracker = 0;
                }
            }
        }
        cancellation_check_scaler(cancellation)?;
        self.scale = next_scale;
        self.growth_tracker = next_growth_tracker;
        self.stage = GradScalerStage::Ready;
        Ok(())
    }
}

fn map_floating_tensor(
    input: &Tensor,
    factor: f64,
    cancellation: &CancellationToken,
) -> Result<Tensor, GradScalerError> {
    let (candidate, _) = map_floating_tensor_checked(input, factor, cancellation)?;
    Ok(candidate)
}

fn unscale_floating_tensor(
    input: &Tensor,
    factor: f64,
    cancellation: &CancellationToken,
) -> Result<(Tensor, bool), GradScalerError> {
    map_floating_tensor_checked(input, factor, cancellation)
}

fn map_floating_tensor_checked(
    input: &Tensor,
    factor: f64,
    cancellation: &CancellationToken,
) -> Result<(Tensor, bool), GradScalerError> {
    let descriptor = input.descriptor();
    if descriptor.device() != DeviceId::CPU {
        return Err(GradScalerError::UnsupportedDevice(descriptor.device()));
    }
    if descriptor.dtype().class() != NumericClass::FloatingPoint {
        return Err(GradScalerError::UnsupportedDType(descriptor.dtype()));
    }
    let shape = descriptor.shape().to_vec();
    let element_count = shape.iter().try_fold(1_u64, |product, dimension| {
        product
            .checked_mul(*dimension)
            .ok_or(GradScalerError::ShapeOverflow)
    })?;
    let mut candidate = input.clone();
    let mut write = candidate.write()?;
    let mut found_non_finite = false;
    for linear_index in 0..element_count {
        if linear_index % 1_024 == 0 {
            cancellation_check_scaler(cancellation)?;
        }
        let indices = scaler_unravel_index(linear_index, &shape)?;
        let value = match descriptor
            .dtype()
            .decode_scalar(input.element_bytes(&indices)?)?
        {
            DecodedScalar::Real(value) => value,
            _ => return Err(GradScalerError::UnsupportedDType(descriptor.dtype())),
        };
        found_non_finite |= !value.is_finite();
        let encoded = descriptor.dtype().encode_scalar(
            Scalar::Float(value * factor),
            "torch.amp.GradScaler",
            DeviceId::CPU,
        )?;
        write.element_bytes_mut(&indices)?.copy_from_slice(&encoded);
    }
    drop(write);
    cancellation_check_scaler(cancellation)?;
    Ok((candidate, found_non_finite))
}

fn scaler_unravel_index(mut linear_index: u64, shape: &[u64]) -> Result<Vec<u64>, GradScalerError> {
    let mut indices = Vec::new();
    indices
        .try_reserve_exact(shape.len())
        .map_err(|_| GradScalerError::AllocationFailed)?;
    indices.resize(shape.len(), 0);
    for dimension_index in (0..shape.len()).rev() {
        let dimension = shape[dimension_index];
        if dimension == 0 {
            return Err(GradScalerError::ShapeOverflow);
        }
        indices[dimension_index] = linear_index % dimension;
        linear_index /= dimension;
    }
    Ok(indices)
}

fn cancellation_check_scaler(cancellation: &CancellationToken) -> Result<(), GradScalerError> {
    if cancellation.is_cancelled() {
        Err(GradScalerError::Cancelled)
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum GradScalerError {
    #[error("gradient scaler configuration is invalid")]
    InvalidConfiguration,
    #[error("gradient scaler method order is invalid for its current optimizer stage")]
    InvalidStage,
    #[error("gradient scaler supports no native operation for device {0:?}")]
    UnsupportedDevice(DeviceId),
    #[error("gradient scaler requires a floating-point tensor, received {0:?}")]
    UnsupportedDType(crate::DType),
    #[error("gradient scaler tensor shape overflowed")]
    ShapeOverflow,
    #[error("gradient scaler allocation failed")]
    AllocationFailed,
    #[error("gradient scaler growth tracker overflowed")]
    CounterOverflow,
    #[error("gradient scaler scale overflowed")]
    ScaleOverflow,
    #[error("gradient scaler scale underflowed to zero")]
    ScaleUnderflow,
    #[error("gradient scaler received no gradients for non-finite checking")]
    NoGradients,
    #[error("gradient scaler operation was cancelled")]
    Cancelled,
    #[error(transparent)]
    Tensor(#[from] TensorError),
}

#[derive(Clone, Debug)]
pub struct SavedTensor {
    tensor: Tensor,
    witness: MutationWitness,
}

impl SavedTensor {
    pub fn capture(tensor: &Tensor) -> Self {
        Self {
            tensor: tensor.clone(),
            witness: tensor.mutation_witness(),
        }
    }

    pub fn tensor(&self) -> &Tensor {
        &self.tensor
    }

    pub fn validate(&self) -> Result<(), AutogradError> {
        let actual_version = self.witness.actual_epoch();
        if !self.witness.is_current() {
            return Err(AutogradError::SavedTensorModified {
                tensor: self.witness.tensor_id(),
                expected_version: self.witness.expected_epoch(),
                actual_version,
            });
        }
        Ok(())
    }
}

pub trait BackwardRule: Send + Sync {
    fn vjp(
        &self,
        output_gradients: &[Option<Tensor>],
        saved_tensors: &[SavedTensor],
        cancellation: &CancellationToken,
    ) -> Result<Vec<Option<Tensor>>, AutogradError>;

    fn vjp_with_context(
        &self,
        output_gradients: &[Option<Tensor>],
        saved_tensors: &[SavedTensor],
        _backend: &CpuBackend,
        execution: &ExecutionContext<'_>,
    ) -> Result<Vec<Option<Tensor>>, AutogradError> {
        self.vjp(output_gradients, saved_tensors, execution.cancellation)
    }

    fn higher_order_policy(&self) -> breadth::HigherOrderPolicy {
        breadth::HigherOrderPolicy::FirstOrderOnly
    }

    fn symbol(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn vjp_higher_order(
        &self,
        _output_gradients: &[Option<Tensor>],
        _saved_tensors: &[SavedTensor],
        _context: &mut HigherOrderContext<'_, '_>,
    ) -> Result<Vec<Option<Tensor>>, AutogradError> {
        match self.higher_order_policy() {
            breadth::HigherOrderPolicy::Analytical => Err(AutogradError::MissingHigherOrderRule {
                symbol: self.symbol(),
            }),
            policy => Err(AutogradError::HigherOrderUnavailable {
                symbol: self.symbol(),
                policy,
            }),
        }
    }
}

pub trait GradientReducer {
    fn add(
        &self,
        left: Tensor,
        right: Tensor,
        cancellation: &CancellationToken,
    ) -> Result<Tensor, AutogradError>;

    fn add_higher_order(
        &self,
        _left: Tensor,
        _right: Tensor,
        _context: &mut HigherOrderContext<'_, '_>,
    ) -> Result<Tensor, AutogradError> {
        Err(AutogradError::MissingHigherOrderRule {
            symbol: "gradient accumulation",
        })
    }
}

#[derive(Clone)]
struct TapeNode {
    id: TapeNodeId,
    inputs: Vec<AutogradInput>,
    output_count: u32,
    saved_tensors: Vec<SavedTensor>,
    rule: Arc<dyn BackwardRule>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
pub enum TapeState {
    Active,
    Completed,
    Cancelled(String),
    Faulted(String),
}

pub struct AutogradTape {
    mode: GradientMode,
    state: TapeState,
    nodes: Vec<TapeNode>,
    leaf_bindings: HashMap<TensorId, LeafId>,
    value_bindings: HashMap<TensorId, AutogradInput>,
    next_node: u64,
}

pub struct HigherOrderContext<'a, 'execution> {
    backend: &'a CpuBackend,
    execution: &'a ExecutionContext<'execution>,
    tape: &'a mut AutogradTape,
}

impl<'a, 'execution> HigherOrderContext<'a, 'execution> {
    pub fn backend(&self) -> &CpuBackend {
        self.backend
    }

    pub fn execution(&self) -> &ExecutionContext<'execution> {
        self.execution
    }

    pub fn record_operation(
        &mut self,
        inputs: &[&Tensor],
        outputs: &[&Tensor],
        differentiable_outputs: &[bool],
        saved_tensors: Vec<SavedTensor>,
        rule: Arc<dyn BackwardRule>,
    ) -> Result<Option<Vec<OutputSlot>>, AutogradError> {
        cancellation_check(self.execution.cancellation)?;
        self.tape
            .record_operation(inputs, outputs, differentiable_outputs, saved_tensors, rule)
    }
}

impl AutogradTape {
    pub fn new(mode: GradientMode) -> Self {
        Self {
            mode,
            state: TapeState::Active,
            nodes: Vec::new(),
            leaf_bindings: HashMap::new(),
            value_bindings: HashMap::new(),
            next_node: 0,
        }
    }

    pub fn mode(&self) -> GradientMode {
        self.mode
    }

    pub fn with_mode<T>(
        &mut self,
        mode: GradientMode,
        cancellation: &CancellationToken,
        operation: impl FnOnce(&mut Self) -> Result<T, AutogradError>,
    ) -> Result<T, AutogradError> {
        self.require_active()?;
        cancellation_check(cancellation)?;
        let previous = self.mode;
        self.mode = mode;
        let result = operation(self);
        self.mode = previous;
        cancellation_check(cancellation)?;
        result
    }

    pub fn state(&self) -> &TapeState {
        &self.state
    }

    pub fn retained_node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn set_requires_grad(
        &mut self,
        tensor: &Tensor,
        leaf: Option<LeafId>,
        requires_grad: bool,
        cancellation: &CancellationToken,
    ) -> Result<(), AutogradError> {
        self.require_active()?;
        cancellation_check(cancellation)?;
        if requires_grad {
            if !matches!(
                tensor.descriptor().dtype().class(),
                NumericClass::FloatingPoint | NumericClass::Complex
            ) {
                return Err(AutogradError::InvalidRequiresGradDType {
                    dtype: tensor.descriptor().dtype(),
                });
            }
            let leaf = leaf.ok_or_else(|| AutogradError::InvalidGraph {
                reason: "enabling gradients requires a checked leaf identifier".to_owned(),
            })?;
            if let Some(existing) = self.leaf_bindings.get(&tensor.tensor_id())
                && existing != &leaf
            {
                return Err(AutogradError::InvalidGraph {
                    reason: format!(
                        "logical tensor {} is already bound to leaf {:?}",
                        tensor.tensor_id().get(),
                        existing.as_str()
                    ),
                });
            }
            cancellation_check(cancellation)?;
            self.leaf_bindings.insert(tensor.tensor_id(), leaf.clone());
            self.value_bindings
                .insert(tensor.tensor_id(), AutogradInput::Leaf(leaf));
        } else {
            cancellation_check(cancellation)?;
            self.leaf_bindings.remove(&tensor.tensor_id());
            self.value_bindings.remove(&tensor.tensor_id());
        }
        Ok(())
    }

    pub fn requires_grad(&self, tensor: &Tensor) -> bool {
        self.leaf_bindings.contains_key(&tensor.tensor_id())
    }

    pub fn leaf_binding(&self, tensor: &Tensor) -> Option<&LeafId> {
        self.leaf_bindings.get(&tensor.tensor_id())
    }

    pub fn output_slot(&self, tensor: &Tensor) -> Option<OutputSlot> {
        match self.value_bindings.get(&tensor.tensor_id()) {
            Some(AutogradInput::Node(slot)) => Some(*slot),
            Some(AutogradInput::Leaf(_) | AutogradInput::Constant) | None => None,
        }
    }

    pub fn record_operation(
        &mut self,
        input_tensors: &[&Tensor],
        output_tensors: &[&Tensor],
        differentiable_outputs: &[bool],
        saved_tensors: Vec<SavedTensor>,
        rule: Arc<dyn BackwardRule>,
    ) -> Result<Option<Vec<OutputSlot>>, AutogradError> {
        if output_tensors.len() != differentiable_outputs.len() {
            return Err(AutogradError::InvalidGraph {
                reason: "recorded output and differentiability arity differ".to_owned(),
            });
        }
        let output_count =
            u32::try_from(output_tensors.len()).map_err(|_| AutogradError::InvalidGraph {
                reason: "recorded output arity exceeds the native tape limit".to_owned(),
            })?;
        let inputs = input_tensors
            .iter()
            .map(|tensor| {
                self.value_bindings
                    .get(&tensor.tensor_id())
                    .cloned()
                    .unwrap_or(AutogradInput::Constant)
            })
            .collect::<Vec<_>>();
        self.value_bindings
            .try_reserve(output_tensors.len())
            .map_err(|_| AutogradError::AllocationFailed)?;
        let Some(slots) = self.record(inputs, output_count, saved_tensors, rule)? else {
            return Ok(None);
        };
        for ((tensor, differentiable), slot) in output_tensors
            .iter()
            .zip(differentiable_outputs)
            .zip(&slots)
        {
            if *differentiable {
                self.value_bindings
                    .insert(tensor.tensor_id(), AutogradInput::Node(*slot));
            } else {
                self.value_bindings.remove(&tensor.tensor_id());
            }
        }
        Ok(Some(slots))
    }

    pub fn record(
        &mut self,
        inputs: Vec<AutogradInput>,
        output_count: u32,
        saved_tensors: Vec<SavedTensor>,
        rule: Arc<dyn BackwardRule>,
    ) -> Result<Option<Vec<OutputSlot>>, AutogradError> {
        self.require_active()?;
        if self.mode != GradientMode::Enabled {
            return Ok(None);
        }
        if output_count == 0 {
            return Err(AutogradError::InvalidGraph {
                reason: "a recorded autograd node must have at least one output".to_owned(),
            });
        }
        let id = TapeNodeId(self.next_node);
        let next_node = self
            .next_node
            .checked_add(1)
            .ok_or(AutogradError::IdentifierOverflow)?;
        for saved in &saved_tensors {
            saved.validate()?;
        }
        for input in &inputs {
            if let AutogradInput::Node(slot) = input {
                self.validate_slot(*slot)?;
                if slot.node.get() >= id.get() {
                    return Err(AutogradError::InvalidGraph {
                        reason: "autograd inputs must reference an earlier node".to_owned(),
                    });
                }
            }
        }
        self.next_node = next_node;
        self.nodes.push(TapeNode {
            id,
            inputs,
            output_count,
            saved_tensors,
            rule,
        });
        let outputs = (0..output_count)
            .map(|output| OutputSlot { node: id, output })
            .collect();
        Ok(Some(outputs))
    }

    pub fn backward(
        &mut self,
        seeds: Vec<(OutputSlot, Tensor)>,
        reducer: &dyn GradientReducer,
        cancellation: &CancellationToken,
    ) -> Result<HashMap<LeafId, Tensor>, AutogradError> {
        self.reverse(seeds, reducer, cancellation, false)
    }

    pub fn backward_retain_graph(
        &mut self,
        seeds: Vec<(OutputSlot, Tensor)>,
        reducer: &dyn GradientReducer,
        cancellation: &CancellationToken,
        retain_graph: bool,
    ) -> Result<HashMap<LeafId, Tensor>, AutogradError> {
        self.reverse(seeds, reducer, cancellation, retain_graph)
    }

    pub fn reverse(
        &mut self,
        seeds: Vec<(OutputSlot, Tensor)>,
        reducer: &dyn GradientReducer,
        cancellation: &CancellationToken,
        retain_graph: bool,
    ) -> Result<HashMap<LeafId, Tensor>, AutogradError> {
        self.require_active()?;
        let result = self.backward_active(seeds, reducer, cancellation);
        match result {
            Ok(gradients) if retain_graph => Ok(gradients),
            Ok(gradients) => {
                self.release_recorded_nodes();
                self.state = TapeState::Completed;
                Ok(gradients)
            }
            Err(error) => {
                self.release_recorded_nodes();
                self.state = if error == AutogradError::Cancelled {
                    TapeState::Cancelled(error.to_string())
                } else {
                    TapeState::Faulted(error.to_string())
                };
                Err(error)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reverse_with_context(
        &mut self,
        seeds: Vec<(OutputSlot, Tensor)>,
        reducer: &dyn GradientReducer,
        retain_graph: bool,
        create_graph: bool,
        backend: &CpuBackend,
        execution: &ExecutionContext<'_>,
    ) -> Result<HashMap<LeafId, Tensor>, AutogradError> {
        if !create_graph {
            self.require_active()?;
            let result = self.backward_active_with_context(seeds, reducer, backend, execution);
            return match result {
                Ok(gradients) if retain_graph => Ok(gradients),
                Ok(gradients) => {
                    self.release_recorded_nodes();
                    self.state = TapeState::Completed;
                    Ok(gradients)
                }
                Err(error) => {
                    self.release_recorded_nodes();
                    self.state = if error == AutogradError::Cancelled {
                        TapeState::Cancelled(error.to_string())
                    } else {
                        TapeState::Faulted(error.to_string())
                    };
                    Err(error)
                }
            };
        }
        self.require_active()?;
        cancellation_check(execution.cancellation)?;
        let mut derivative_graph = self.fork_active()?;
        let result = self.backward_active_higher_order(
            seeds,
            reducer,
            backend,
            execution,
            &mut derivative_graph,
        );
        match result {
            Ok(gradients) => {
                if let Err(error) = cancellation_check(execution.cancellation) {
                    self.release_recorded_nodes();
                    self.state = TapeState::Cancelled(error.to_string());
                    return Err(error);
                }
                *self = derivative_graph;
                Ok(gradients)
            }
            Err(error) => {
                self.release_recorded_nodes();
                self.state = if error == AutogradError::Cancelled {
                    TapeState::Cancelled(error.to_string())
                } else {
                    TapeState::Faulted(error.to_string())
                };
                Err(error)
            }
        }
    }

    pub fn reverse_and_publish(
        &mut self,
        seeds: Vec<(OutputSlot, Tensor)>,
        reducer: &dyn GradientReducer,
        cancellation: &CancellationToken,
        retain_graph: bool,
        inputs: Option<&[LeafId]>,
        store: &mut GradientStore,
    ) -> Result<(), AutogradError> {
        self.require_active()?;
        let result = self.backward_active(seeds, reducer, cancellation);
        match result {
            Ok(mut gradients) => {
                if let Some(inputs) = inputs {
                    gradients.retain(|leaf, _| inputs.contains(leaf));
                }
                let publication = cancellation_check(cancellation)
                    .and_then(|()| store.accumulate(gradients, reducer, cancellation));
                match publication {
                    Ok(()) => {
                        if !retain_graph {
                            self.release_recorded_nodes();
                            self.state = TapeState::Completed;
                        }
                        Ok(())
                    }
                    Err(error) => {
                        self.release_recorded_nodes();
                        self.state = if error == AutogradError::Cancelled {
                            TapeState::Cancelled(error.to_string())
                        } else {
                            TapeState::Faulted(error.to_string())
                        };
                        Err(error)
                    }
                }
            }
            Err(error) => {
                self.release_recorded_nodes();
                self.state = if error == AutogradError::Cancelled {
                    TapeState::Cancelled(error.to_string())
                } else {
                    TapeState::Faulted(error.to_string())
                };
                Err(error)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reverse_and_publish_with_context(
        &mut self,
        seeds: Vec<(OutputSlot, Tensor)>,
        reducer: &dyn GradientReducer,
        retain_graph: bool,
        create_graph: bool,
        inputs: Option<&[LeafId]>,
        store: &mut GradientStore,
        backend: &CpuBackend,
        execution: &ExecutionContext<'_>,
    ) -> Result<(), AutogradError> {
        if !create_graph {
            self.require_active()?;
            let result = self.backward_active_with_context(seeds, reducer, backend, execution);
            return match result {
                Ok(mut gradients) => {
                    if let Some(inputs) = inputs {
                        gradients.retain(|leaf, _| inputs.contains(leaf));
                    }
                    let publication = cancellation_check(execution.cancellation).and_then(|()| {
                        store.accumulate(gradients, reducer, execution.cancellation)
                    });
                    match publication {
                        Ok(()) => {
                            if !retain_graph {
                                self.release_recorded_nodes();
                                self.state = TapeState::Completed;
                            }
                            Ok(())
                        }
                        Err(error) => {
                            self.release_recorded_nodes();
                            self.state = if error == AutogradError::Cancelled {
                                TapeState::Cancelled(error.to_string())
                            } else {
                                TapeState::Faulted(error.to_string())
                            };
                            Err(error)
                        }
                    }
                }
                Err(error) => {
                    self.release_recorded_nodes();
                    self.state = if error == AutogradError::Cancelled {
                        TapeState::Cancelled(error.to_string())
                    } else {
                        TapeState::Faulted(error.to_string())
                    };
                    Err(error)
                }
            };
        }
        self.require_active()?;
        cancellation_check(execution.cancellation)?;
        let mut derivative_graph = self.fork_active()?;
        let result = self.backward_active_higher_order(
            seeds,
            reducer,
            backend,
            execution,
            &mut derivative_graph,
        );
        match result {
            Ok(mut gradients) => {
                if let Some(inputs) = inputs {
                    gradients.retain(|leaf, _| inputs.contains(leaf));
                }
                let publication = store.accumulate_higher_order(
                    gradients,
                    reducer,
                    backend,
                    execution,
                    &mut derivative_graph,
                );
                match publication {
                    Ok(staged) => {
                        if let Err(error) = cancellation_check(execution.cancellation) {
                            self.release_recorded_nodes();
                            self.state = TapeState::Cancelled(error.to_string());
                            return Err(error);
                        }
                        store.gradients = staged;
                        *self = derivative_graph;
                        Ok(())
                    }
                    Err(error) => {
                        self.release_recorded_nodes();
                        self.state = if error == AutogradError::Cancelled {
                            TapeState::Cancelled(error.to_string())
                        } else {
                            TapeState::Faulted(error.to_string())
                        };
                        Err(error)
                    }
                }
            }
            Err(error) => {
                self.release_recorded_nodes();
                self.state = if error == AutogradError::Cancelled {
                    TapeState::Cancelled(error.to_string())
                } else {
                    TapeState::Faulted(error.to_string())
                };
                Err(error)
            }
        }
    }

    pub fn cancel(&mut self, reason: impl Into<String>) -> Result<(), AutogradError> {
        self.require_active()?;
        self.release_recorded_nodes();
        self.state = TapeState::Cancelled(reason.into());
        Ok(())
    }

    pub fn fault(&mut self, reason: impl Into<String>) -> Result<(), AutogradError> {
        self.require_active()?;
        self.release_recorded_nodes();
        self.state = TapeState::Faulted(reason.into());
        Ok(())
    }

    fn backward_active(
        &self,
        seeds: Vec<(OutputSlot, Tensor)>,
        reducer: &dyn GradientReducer,
        cancellation: &CancellationToken,
    ) -> Result<HashMap<LeafId, Tensor>, AutogradError> {
        cancellation_check(cancellation)?;
        let mut node_gradients = HashMap::new();
        for (slot, gradient) in seeds {
            self.validate_slot(slot)?;
            accumulate(&mut node_gradients, slot, gradient, reducer, cancellation)?;
        }
        let mut leaf_gradients = HashMap::new();
        for node in self.nodes.iter().rev() {
            cancellation_check(cancellation)?;
            for saved in &node.saved_tensors {
                saved.validate()?;
            }
            let output_gradients = (0..node.output_count)
                .map(|output| {
                    node_gradients.remove(&OutputSlot {
                        node: node.id,
                        output,
                    })
                })
                .collect::<Vec<_>>();
            if output_gradients.iter().all(Option::is_none) {
                continue;
            }
            let input_gradients =
                node.rule
                    .vjp(&output_gradients, &node.saved_tensors, cancellation)?;
            if input_gradients.len() != node.inputs.len() {
                return Err(AutogradError::GradientArity {
                    expected: node.inputs.len(),
                    actual: input_gradients.len(),
                });
            }
            for (input, gradient) in node.inputs.iter().zip(input_gradients) {
                let Some(gradient) = gradient else {
                    continue;
                };
                match input {
                    AutogradInput::Node(slot) => {
                        accumulate(&mut node_gradients, *slot, gradient, reducer, cancellation)?;
                    }
                    AutogradInput::Leaf(leaf) => {
                        accumulate(
                            &mut leaf_gradients,
                            leaf.clone(),
                            gradient,
                            reducer,
                            cancellation,
                        )?;
                    }
                    AutogradInput::Constant => {}
                }
            }
        }
        Ok(leaf_gradients)
    }

    fn backward_active_higher_order(
        &self,
        seeds: Vec<(OutputSlot, Tensor)>,
        reducer: &dyn GradientReducer,
        backend: &CpuBackend,
        execution: &ExecutionContext<'_>,
        derivative_graph: &mut AutogradTape,
    ) -> Result<HashMap<LeafId, Tensor>, AutogradError> {
        cancellation_check(execution.cancellation)?;
        let mut node_gradients = HashMap::new();
        for (slot, gradient) in seeds {
            self.validate_slot(slot)?;
            accumulate_higher_order(
                &mut node_gradients,
                slot,
                gradient,
                reducer,
                backend,
                execution,
                derivative_graph,
            )?;
        }
        let mut leaf_gradients = HashMap::new();
        for node in self.nodes.iter().rev() {
            cancellation_check(execution.cancellation)?;
            for saved in &node.saved_tensors {
                saved.validate()?;
            }
            let output_gradients = (0..node.output_count)
                .map(|output| {
                    node_gradients.remove(&OutputSlot {
                        node: node.id,
                        output,
                    })
                })
                .collect::<Vec<_>>();
            if output_gradients.iter().all(Option::is_none) {
                continue;
            }
            let policy = node.rule.higher_order_policy();
            if policy != breadth::HigherOrderPolicy::Analytical {
                return Err(AutogradError::HigherOrderUnavailable {
                    symbol: node.rule.symbol(),
                    policy,
                });
            }
            let input_gradients = {
                let mut context = HigherOrderContext {
                    backend,
                    execution,
                    tape: derivative_graph,
                };
                node.rule
                    .vjp_higher_order(&output_gradients, &node.saved_tensors, &mut context)?
            };
            if input_gradients.len() != node.inputs.len() {
                return Err(AutogradError::GradientArity {
                    expected: node.inputs.len(),
                    actual: input_gradients.len(),
                });
            }
            for (input, gradient) in node.inputs.iter().zip(input_gradients) {
                let Some(gradient) = gradient else {
                    continue;
                };
                match input {
                    AutogradInput::Node(slot) => accumulate_higher_order(
                        &mut node_gradients,
                        *slot,
                        gradient,
                        reducer,
                        backend,
                        execution,
                        derivative_graph,
                    )?,
                    AutogradInput::Leaf(leaf) => accumulate_higher_order(
                        &mut leaf_gradients,
                        leaf.clone(),
                        gradient,
                        reducer,
                        backend,
                        execution,
                        derivative_graph,
                    )?,
                    AutogradInput::Constant => {}
                }
            }
        }
        Ok(leaf_gradients)
    }

    fn backward_active_with_context(
        &self,
        seeds: Vec<(OutputSlot, Tensor)>,
        reducer: &dyn GradientReducer,
        backend: &CpuBackend,
        execution: &ExecutionContext<'_>,
    ) -> Result<HashMap<LeafId, Tensor>, AutogradError> {
        cancellation_check(execution.cancellation)?;
        let mut node_gradients = HashMap::new();
        for (slot, gradient) in seeds {
            self.validate_slot(slot)?;
            accumulate(
                &mut node_gradients,
                slot,
                gradient,
                reducer,
                execution.cancellation,
            )?;
        }
        let mut leaf_gradients = HashMap::new();
        for node in self.nodes.iter().rev() {
            cancellation_check(execution.cancellation)?;
            for saved in &node.saved_tensors {
                saved.validate()?;
            }
            let output_gradients = (0..node.output_count)
                .map(|output| {
                    node_gradients.remove(&OutputSlot {
                        node: node.id,
                        output,
                    })
                })
                .collect::<Vec<_>>();
            if output_gradients.iter().all(Option::is_none) {
                continue;
            }
            let input_gradients = node.rule.vjp_with_context(
                &output_gradients,
                &node.saved_tensors,
                backend,
                execution,
            )?;
            if input_gradients.len() != node.inputs.len() {
                return Err(AutogradError::GradientArity {
                    expected: node.inputs.len(),
                    actual: input_gradients.len(),
                });
            }
            for (input, gradient) in node.inputs.iter().zip(input_gradients) {
                let Some(gradient) = gradient else {
                    continue;
                };
                match input {
                    AutogradInput::Node(slot) => accumulate(
                        &mut node_gradients,
                        *slot,
                        gradient,
                        reducer,
                        execution.cancellation,
                    )?,
                    AutogradInput::Leaf(leaf) => accumulate(
                        &mut leaf_gradients,
                        leaf.clone(),
                        gradient,
                        reducer,
                        execution.cancellation,
                    )?,
                    AutogradInput::Constant => {}
                }
            }
        }
        Ok(leaf_gradients)
    }

    fn fork_active(&self) -> Result<Self, AutogradError> {
        self.require_active()?;
        let mut nodes = Vec::new();
        nodes
            .try_reserve_exact(self.nodes.len())
            .map_err(|_| AutogradError::AllocationFailed)?;
        nodes.extend(self.nodes.iter().cloned());
        let mut leaf_bindings = HashMap::new();
        leaf_bindings
            .try_reserve(self.leaf_bindings.len())
            .map_err(|_| AutogradError::AllocationFailed)?;
        leaf_bindings.extend(
            self.leaf_bindings
                .iter()
                .map(|(tensor, leaf)| (*tensor, leaf.clone())),
        );
        let mut value_bindings = HashMap::new();
        value_bindings
            .try_reserve(self.value_bindings.len())
            .map_err(|_| AutogradError::AllocationFailed)?;
        value_bindings.extend(
            self.value_bindings
                .iter()
                .map(|(tensor, input)| (*tensor, input.clone())),
        );
        Ok(Self {
            mode: self.mode,
            state: TapeState::Active,
            nodes,
            leaf_bindings,
            value_bindings,
            next_node: self.next_node,
        })
    }

    fn release_recorded_nodes(&mut self) {
        self.nodes.clear();
        self.value_bindings
            .retain(|_, input| matches!(input, AutogradInput::Leaf(_)));
    }

    fn validate_slot(&self, slot: OutputSlot) -> Result<(), AutogradError> {
        let node = self.nodes.iter().find(|node| node.id == slot.node).ok_or(
            AutogradError::InvalidGraph {
                reason: format!("seed references unknown node {}", slot.node.get()),
            },
        )?;
        if slot.output >= node.output_count {
            return Err(AutogradError::InvalidGraph {
                reason: format!(
                    "seed output {} exceeds node {} output count {}",
                    slot.output,
                    slot.node.get(),
                    node.output_count
                ),
            });
        }
        Ok(())
    }

    fn require_active(&self) -> Result<(), AutogradError> {
        if self.state == TapeState::Active {
            Ok(())
        } else {
            Err(AutogradError::TerminalTape {
                state: self.state.clone(),
            })
        }
    }
}

#[derive(Debug)]
pub struct CheckpointRecord {
    saved: Vec<SavedTensor>,
    needs_input_grad: Vec<bool>,
    released: bool,
}

impl CheckpointRecord {
    pub fn capture(saved: &[Tensor], needs_input_grad: Vec<bool>) -> Result<Self, AutogradError> {
        if needs_input_grad.len() != saved.len() {
            return Err(AutogradError::GradientArity {
                expected: saved.len(),
                actual: needs_input_grad.len(),
            });
        }
        Ok(Self {
            saved: saved.iter().map(SavedTensor::capture).collect(),
            needs_input_grad,
            released: false,
        })
    }

    pub fn saved_tensors(&self) -> Result<Vec<Tensor>, AutogradError> {
        if self.released {
            return Err(AutogradError::ReleasedCheckpoint);
        }
        self.saved
            .iter()
            .map(|saved| {
                saved.validate()?;
                Ok(saved.tensor().clone())
            })
            .collect()
    }

    pub fn needs_input_grad(&self, index: usize) -> bool {
        self.needs_input_grad.get(index).copied().unwrap_or(false)
    }

    pub fn saved_tensor_count(&self) -> usize {
        self.saved.len()
    }

    pub fn release(&mut self) {
        self.saved.clear();
        self.needs_input_grad.clear();
        self.released = true;
    }
}

#[derive(Default)]
pub struct GradientStore {
    gradients: HashMap<LeafId, Tensor>,
}

impl GradientStore {
    pub fn publish(
        &mut self,
        gradients: HashMap<LeafId, Tensor>,
        cancellation: &CancellationToken,
    ) -> Result<(), AutogradError> {
        cancellation_check(cancellation)?;
        self.gradients = gradients;
        Ok(())
    }

    pub fn gradient(&self, leaf: &LeafId) -> Option<&Tensor> {
        self.gradients.get(leaf)
    }

    pub fn accumulate(
        &mut self,
        gradients: HashMap<LeafId, Tensor>,
        reducer: &dyn GradientReducer,
        cancellation: &CancellationToken,
    ) -> Result<(), AutogradError> {
        cancellation_check(cancellation)?;
        let mut staged = self.gradients.clone();
        for (leaf, gradient) in gradients {
            accumulate(&mut staged, leaf, gradient, reducer, cancellation)?;
        }
        cancellation_check(cancellation)?;
        self.gradients = staged;
        Ok(())
    }

    fn accumulate_higher_order(
        &self,
        gradients: HashMap<LeafId, Tensor>,
        reducer: &dyn GradientReducer,
        backend: &CpuBackend,
        execution: &ExecutionContext<'_>,
        derivative_graph: &mut AutogradTape,
    ) -> Result<HashMap<LeafId, Tensor>, AutogradError> {
        cancellation_check(execution.cancellation)?;
        let mut staged = self.gradients.clone();
        for (leaf, gradient) in gradients {
            accumulate_higher_order(
                &mut staged,
                leaf,
                gradient,
                reducer,
                backend,
                execution,
                derivative_graph,
            )?;
        }
        cancellation_check(execution.cancellation)?;
        Ok(staged)
    }

    pub fn zero_grad(
        &mut self,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
        set_to_none: bool,
    ) -> Result<(), AutogradError> {
        context.check()?;
        if set_to_none {
            self.gradients.clear();
            return Ok(());
        }
        let mut staged = HashMap::with_capacity(self.gradients.len());
        for (leaf, gradient) in &self.gradients {
            let (zero, _) = backend.fill(
                crate::Scalar::Float(0.0),
                gradient.descriptor().clone(),
                context,
            )?;
            staged.insert(leaf.clone(), zero);
        }
        context.check()?;
        self.gradients = staged;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.gradients.len()
    }

    pub fn is_empty(&self) -> bool {
        self.gradients.is_empty()
    }
}

fn accumulate<Key: std::hash::Hash + Eq>(
    gradients: &mut HashMap<Key, Tensor>,
    key: Key,
    gradient: Tensor,
    reducer: &dyn GradientReducer,
    cancellation: &CancellationToken,
) -> Result<(), AutogradError> {
    cancellation_check(cancellation)?;
    let value = if let Some(existing) = gradients.remove(&key) {
        if existing.descriptor() != gradient.descriptor() {
            return Err(AutogradError::GradientDescriptorMismatch);
        }
        let descriptor = existing.descriptor().clone();
        let reduced = reducer.add(existing, gradient, cancellation)?;
        if reduced.descriptor() != &descriptor {
            return Err(AutogradError::GradientDescriptorMismatch);
        }
        reduced
    } else {
        gradient
    };
    gradients.insert(key, value);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn accumulate_higher_order<Key: std::hash::Hash + Eq>(
    gradients: &mut HashMap<Key, Tensor>,
    key: Key,
    gradient: Tensor,
    reducer: &dyn GradientReducer,
    backend: &CpuBackend,
    execution: &ExecutionContext<'_>,
    derivative_graph: &mut AutogradTape,
) -> Result<(), AutogradError> {
    cancellation_check(execution.cancellation)?;
    let value = if let Some(existing) = gradients.remove(&key) {
        if existing.descriptor() != gradient.descriptor() {
            return Err(AutogradError::GradientDescriptorMismatch);
        }
        let descriptor = existing.descriptor().clone();
        let reduced = {
            let mut context = HigherOrderContext {
                backend,
                execution,
                tape: derivative_graph,
            };
            reducer.add_higher_order(existing, gradient, &mut context)?
        };
        if reduced.descriptor() != &descriptor {
            return Err(AutogradError::GradientDescriptorMismatch);
        }
        reduced
    } else {
        gradient
    };
    gradients.insert(key, value);
    Ok(())
}

fn cancellation_check(cancellation: &CancellationToken) -> Result<(), AutogradError> {
    if cancellation.is_cancelled() {
        Err(AutogradError::Cancelled)
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum AutogradError {
    #[error("autograd graph is invalid: {reason}")]
    InvalidGraph { reason: String },
    #[error("requires_grad is valid only for floating-point or complex tensors, not {dtype:?}")]
    InvalidRequiresGradDType { dtype: crate::DType },
    #[error("autograd node identifier overflowed")]
    IdentifierOverflow,
    #[error("autograd allocation failed")]
    AllocationFailed,
    #[error("autograd rule {symbol} rejects create_graph under {policy:?} policy")]
    HigherOrderUnavailable {
        symbol: &'static str,
        policy: breadth::HigherOrderPolicy,
    },
    #[error("analytical autograd rule {symbol} has no recorded higher-order implementation")]
    MissingHigherOrderRule { symbol: &'static str },
    #[error(
        "saved logical tensor {tensor:?} changed from version {expected_version} to {actual_version}"
    )]
    SavedTensorModified {
        tensor: TensorId,
        expected_version: u64,
        actual_version: u64,
    },
    #[error("checkpoint state has already been released")]
    ReleasedCheckpoint,
    #[error("VJP returned {actual} input gradients, expected {expected}")]
    GradientArity { expected: usize, actual: usize },
    #[error("gradient accumulation changed or combined incompatible tensor descriptors")]
    GradientDescriptorMismatch,
    #[error("autograd tape is terminal: {state:?}")]
    TerminalTape { state: TapeState },
    #[error("autograd execution was cancelled")]
    Cancelled,
    #[error(transparent)]
    Tensor(#[from] TensorError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DType, DeviceId, StreamId, TensorDescriptor};
    use std::{
        collections::{BTreeMap, BTreeSet},
        error::Error,
    };

    fn scalar(value: f32) -> Tensor {
        let descriptor = match TensorDescriptor::contiguous(
            vec![],
            DType::F32,
            DeviceId::CPU,
            StreamId::DEFAULT,
        ) {
            Ok(value) => value,
            Err(error) => panic!("scalar descriptor failed: {error}"),
        };
        match Tensor::from_bytes(descriptor, value.to_le_bytes().to_vec()) {
            Ok(value) => value,
            Err(error) => panic!("scalar tensor failed: {error}"),
        }
    }

    fn scalar_value(tensor: &Tensor) -> f32 {
        let bytes = match tensor.contiguous_bytes() {
            Ok(value) => value,
            Err(error) => panic!("scalar read failed: {error}"),
        };
        let Some(bytes) = bytes.get(..4) else {
            panic!("scalar requires four bytes");
        };
        let Ok(bytes) = <[u8; 4]>::try_from(bytes) else {
            panic!("scalar requires four bytes");
        };
        f32::from_le_bytes(bytes)
    }

    struct AddReducer;

    impl GradientReducer for AddReducer {
        fn add(
            &self,
            left: Tensor,
            right: Tensor,
            cancellation: &CancellationToken,
        ) -> Result<Tensor, AutogradError> {
            cancellation_check(cancellation)?;
            Ok(scalar(scalar_value(&left) + scalar_value(&right)))
        }
    }

    struct IdentityRule;

    impl BackwardRule for IdentityRule {
        fn vjp(
            &self,
            output_gradients: &[Option<Tensor>],
            _saved_tensors: &[SavedTensor],
            cancellation: &CancellationToken,
        ) -> Result<Vec<Option<Tensor>>, AutogradError> {
            cancellation_check(cancellation)?;
            Ok(vec![output_gradients.first().cloned().flatten()])
        }
    }

    #[test]
    fn leaf_identifier_wire_adapter_revalidates_invariants() {
        assert!(LeafId::try_from(LeafIdWire("\t".to_owned())).is_err());
        assert!(matches!(
            LeafId::try_from(LeafIdWire("weight".to_owned())),
            Ok(identifier) if identifier.as_str() == "weight"
        ));
    }

    #[test]
    fn reverse_mode_traverses_nodes_and_accumulates_leaf_gradients() {
        let leaf = match LeafId::new("x") {
            Ok(value) => value,
            Err(error) => panic!("leaf failed: {error}"),
        };
        let mut tape = AutogradTape::new(GradientMode::Enabled);
        let first = match tape.record(
            vec![AutogradInput::Leaf(leaf.clone())],
            1,
            vec![],
            Arc::new(IdentityRule),
        ) {
            Ok(Some(value)) => value,
            Ok(None) => panic!("enabled tape should record"),
            Err(error) => panic!("record failed: {error}"),
        };
        let Some(first_output) = first.first().copied() else {
            panic!("first node should have an output");
        };
        let second = match tape.record(
            vec![AutogradInput::Node(first_output)],
            1,
            vec![],
            Arc::new(IdentityRule),
        ) {
            Ok(Some(value)) => value,
            Ok(None) => panic!("enabled tape should record"),
            Err(error) => panic!("record failed: {error}"),
        };
        let Some(second_output) = second.first().copied() else {
            panic!("second node should have an output");
        };
        let gradients = match tape.backward(
            vec![(second_output, scalar(3.0))],
            &AddReducer,
            &CancellationToken::default(),
        ) {
            Ok(value) => value,
            Err(error) => panic!("backward failed: {error}"),
        };
        let Some(gradient) = gradients.get(&leaf) else {
            panic!("leaf gradient should exist");
        };
        assert_eq!(scalar_value(gradient), 3.0);
        assert_eq!(tape.state(), &TapeState::Completed);
        assert_eq!(tape.retained_node_count(), 0);
    }

    #[test]
    fn branches_accumulate_with_the_declared_reducer() {
        let leaf = match LeafId::new("x") {
            Ok(value) => value,
            Err(error) => panic!("leaf failed: {error}"),
        };
        let mut tape = AutogradTape::new(GradientMode::Enabled);
        let mut outputs = Vec::new();
        for _ in 0..2 {
            let recorded = match tape.record(
                vec![AutogradInput::Leaf(leaf.clone())],
                1,
                vec![],
                Arc::new(IdentityRule),
            ) {
                Ok(Some(value)) => value,
                Ok(None) => panic!("enabled tape should record"),
                Err(error) => panic!("record failed: {error}"),
            };
            let Some(output) = recorded.first().copied() else {
                panic!("recorded node should have output");
            };
            outputs.push(output);
        }
        let seeds = outputs
            .into_iter()
            .map(|output| (output, scalar(2.0)))
            .collect();
        let gradients = match tape.backward(seeds, &AddReducer, &CancellationToken::default()) {
            Ok(value) => value,
            Err(error) => panic!("backward failed: {error}"),
        };
        let Some(gradient) = gradients.get(&leaf) else {
            panic!("leaf gradient should exist");
        };
        assert_eq!(scalar_value(gradient), 4.0);
    }

    #[test]
    fn cancellation_releases_saved_tensors_and_is_terminal() {
        let mut tape = AutogradTape::new(GradientMode::Enabled);
        let recorded = tape.record(
            vec![AutogradInput::Constant],
            1,
            vec![SavedTensor::capture(&scalar(1.0))],
            Arc::new(IdentityRule),
        );
        assert!(recorded.is_ok());
        let token = CancellationToken::default();
        token.cancel();
        let result = tape.backward(vec![], &AddReducer, &token);
        assert!(matches!(result, Err(AutogradError::Cancelled)));
        assert!(matches!(tape.state(), TapeState::Cancelled(_)));
        assert_eq!(tape.retained_node_count(), 0);
        assert!(matches!(
            tape.record(
                vec![AutogradInput::Constant],
                1,
                vec![],
                Arc::new(IdentityRule),
            ),
            Err(AutogradError::TerminalTape { .. })
        ));
    }

    #[test]
    fn no_grad_and_inference_modes_do_not_retain_rules() {
        for mode in [GradientMode::NoGrad, GradientMode::Inference] {
            let mut tape = AutogradTape::new(mode);
            let result = tape.record(
                vec![AutogradInput::Constant],
                1,
                vec![],
                Arc::new(IdentityRule),
            );
            assert!(matches!(result, Ok(None)));
            assert_eq!(tape.retained_node_count(), 0);
        }
    }

    #[test]
    fn recording_rejects_unknown_or_forward_edges_without_mutation() {
        let mut tape = AutogradTape::new(GradientMode::Enabled);
        let result = tape.record(
            vec![AutogradInput::Node(OutputSlot {
                node: TapeNodeId(9),
                output: 0,
            })],
            1,
            vec![],
            Arc::new(IdentityRule),
        );
        assert!(matches!(result, Err(AutogradError::InvalidGraph { .. })));
        assert_eq!(tape.retained_node_count(), 0);
        let valid = tape.record(
            vec![AutogradInput::Constant],
            1,
            vec![],
            Arc::new(IdentityRule),
        );
        assert!(matches!(
            valid,
            Ok(Some(outputs)) if outputs.first().is_some_and(|slot| slot.node.get() == 0)
        ));
    }

    #[test]
    fn core_autograd_fixture_registry_and_state_contracts_are_complete()
    -> Result<(), Box<dyn Error>> {
        let leaf = LeafId::new("validation-leaf")?;
        let mut tape = AutogradTape::new(GradientMode::Enabled);
        let first = tape
            .record(
                vec![AutogradInput::Leaf(leaf.clone())],
                1,
                vec![SavedTensor::capture(&scalar(1.0))],
                Arc::new(IdentityRule),
            )?
            .ok_or_else(|| AutogradError::InvalidGraph {
                reason: "enabled validation tape did not record".to_owned(),
            })?;
        let first_output = first
            .first()
            .copied()
            .ok_or_else(|| AutogradError::InvalidGraph {
                reason: "validation node has no output".to_owned(),
            })?;
        let second = tape
            .record(
                vec![
                    AutogradInput::Node(first_output),
                    AutogradInput::Leaf(leaf.clone()),
                ],
                1,
                vec![],
                Arc::new(PassthroughBothRule),
            )?
            .ok_or_else(|| AutogradError::InvalidGraph {
                reason: "enabled validation tape did not record its branch".to_owned(),
            })?;
        let second_output = second
            .first()
            .copied()
            .ok_or_else(|| AutogradError::InvalidGraph {
                reason: "validation branch has no output".to_owned(),
            })?;
        let gradients = tape.backward(
            vec![(second_output, scalar(2.0))],
            &AddReducer,
            &CancellationToken::default(),
        )?;
        let accumulated = gradients
            .get(&leaf)
            .is_some_and(|gradient| scalar_value(gradient) == 4.0);

        let mut cancelled_tape = AutogradTape::new(GradientMode::Enabled);
        cancelled_tape.record(
            vec![AutogradInput::Constant],
            1,
            vec![SavedTensor::capture(&scalar(1.0))],
            Arc::new(IdentityRule),
        )?;
        let cancellation = CancellationToken::default();
        cancellation.cancel();
        let cancelled = matches!(
            cancelled_tape.backward(vec![], &AddReducer, &cancellation),
            Err(AutogradError::Cancelled)
        ) && matches!(cancelled_tape.state(), TapeState::Cancelled(_))
            && cancelled_tape.retained_node_count() == 0;

        let modes_do_not_record = [GradientMode::NoGrad, GradientMode::Inference]
            .into_iter()
            .all(|mode| {
                let mut tape = AutogradTape::new(mode);
                matches!(
                    tape.record(
                        vec![AutogradInput::Constant],
                        1,
                        vec![],
                        Arc::new(IdentityRule),
                    ),
                    Ok(None)
                ) && tape.retained_node_count() == 0
            });

        let breadth_ids = breadth::AUTOGRAD_CONSTRUCTS
            .iter()
            .map(|contract| contract.id)
            .collect::<BTreeSet<_>>();
        let breadth_symbols = breadth::AUTOGRAD_CONSTRUCTS
            .iter()
            .map(|contract| contract.symbol)
            .collect::<BTreeSet<_>>();
        let breadth_catalog_is_closed = breadth::AUTOGRAD_CONSTRUCTS.len() == 36
            && breadth_ids.len() == 36
            && breadth_symbols.len() == 36;
        let custom_function_catalog_is_closed = breadth::AUTOGRAD_CONSTRUCTS
            .iter()
            .filter(|contract| contract.construct == "custom-autograd-function")
            .count()
            == 7;
        let breadth_fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../comfy_test_support/fixtures/autograd/breadth-v1.json"
        ))?;
        let fixture_cases = breadth_fixture["catalog_cases"]
            .as_array()
            .ok_or("autograd breadth fixture catalog_cases is not an array")?;
        let fixture_ids = fixture_cases
            .iter()
            .filter_map(|case| case["id"].as_str())
            .collect::<BTreeSet<_>>();
        let fixture_symbols = fixture_cases
            .iter()
            .filter_map(|case| case["symbol"].as_str())
            .collect::<BTreeSet<_>>();
        let fixture_execution_cases = fixture_cases
            .iter()
            .filter_map(|case| case["execution_case"].as_str())
            .collect::<BTreeSet<_>>();
        let strict_fixture_registry = fixture_cases.len() == 36
            && fixture_ids == breadth_ids
            && fixture_symbols == breadth_symbols
            && fixture_execution_cases.len() == 36
            && breadth_fixture["schema_version"].as_u64() == Some(1)
            && breadth_fixture["owner_task_id"].as_str()
                == Some("comfy-parity-native-autograd-breadth")
            && breadth_fixture["oracle"]["development_only"].as_bool() == Some(true)
            && breadth_fixture["oracle"]["source_files"]
                .as_object()
                .is_some_and(|files| files.len() == 6)
            && fixture_cases.iter().all(|case| {
                case["execution_case"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
                    && case["source_observations"]
                        .as_array()
                        .is_some_and(|observations| !observations.is_empty())
            });
        let exact_custom_contracts = breadth::CUSTOM_FUNCTIONS
            .iter()
            .map(|contract| {
                (
                    contract.symbol,
                    contract.forward_arity,
                    contract.variadic_inputs,
                    contract.forward_outputs,
                    contract.backward_inputs,
                    contract.backward_outputs,
                    contract.higher_order,
                    contract.fixture,
                )
            })
            .eq([
                (
                    "vector_quantize",
                    2,
                    false,
                    2,
                    2,
                    2,
                    breadth::HigherOrderPolicy::Analytical,
                    "breadth-v1.json#vector_quantize",
                ),
                (
                    "CheckpointFunction",
                    3,
                    true,
                    1,
                    1,
                    3,
                    breadth::HigherOrderPolicy::FirstOrderOnly,
                    "breadth-v1.json#checkpoint_function",
                ),
                (
                    "QuantLinearFunc",
                    6,
                    false,
                    1,
                    1,
                    6,
                    breadth::HigherOrderPolicy::OnceDifferentiable,
                    ".agents/specs/comfy-parity/fixtures/quant-linear-source-oracle.json#callable",
                ),
                (
                    "HadaWeightTucker",
                    7,
                    false,
                    1,
                    1,
                    7,
                    breadth::HigherOrderPolicy::Analytical,
                    "breadth-v1.json#hada_weight_tucker",
                ),
                (
                    "AddAuxLoss",
                    2,
                    false,
                    1,
                    1,
                    2,
                    breadth::HigherOrderPolicy::Analytical,
                    "breadth-v1.json#add_aux_loss",
                ),
                (
                    "OffloadCheckpointFunction",
                    2,
                    false,
                    1,
                    1,
                    2,
                    breadth::HigherOrderPolicy::FirstOrderOnly,
                    "breadth-v1.json#offload_checkpoint",
                ),
                (
                    "HadaWeight",
                    5,
                    false,
                    1,
                    1,
                    5,
                    breadth::HigherOrderPolicy::Analytical,
                    "breadth-v1.json#hada_weight",
                ),
            ]);

        let cases = BTreeMap::from([
            (
                "autograd_breadth_catalog_has_exact_unique_coverage",
                breadth_catalog_is_closed,
            ),
            ("branch_gradients_accumulate", accumulated),
            (
                "cancellation_is_terminal_and_releases_saved_tensors",
                cancelled,
            ),
            (
                "completed_tapes_release_nodes",
                tape.retained_node_count() == 0,
            ),
            (
                "invariant_bearing_leaf_ids_use_checked_wire_conversion",
                LeafId::try_from(LeafIdWire(" ".to_owned())).is_err(),
            ),
            (
                "strict_36_row_source_fixture_registry_is_closed",
                strict_fixture_registry,
            ),
            (
                "seven_custom_function_contracts_are_exact",
                custom_function_catalog_is_closed && exact_custom_contracts,
            ),
            (
                "fixture_COMFY_AUTOGRAD_0164A83D79F9",
                fixture_ids.contains("COMFY-AUTOGRAD-0164A83D79F9"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_08DA3A226CB4",
                fixture_ids.contains("COMFY-AUTOGRAD-08DA3A226CB4"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_0BDFE52B87F3",
                fixture_ids.contains("COMFY-AUTOGRAD-0BDFE52B87F3"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_0C5FA58D517B",
                fixture_ids.contains("COMFY-AUTOGRAD-0C5FA58D517B"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_104D91298DF9",
                fixture_ids.contains("COMFY-AUTOGRAD-104D91298DF9"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_1691472B873D",
                fixture_ids.contains("COMFY-AUTOGRAD-1691472B873D"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_2682346109CE",
                fixture_ids.contains("COMFY-AUTOGRAD-2682346109CE"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_285F07173F3E",
                fixture_ids.contains("COMFY-AUTOGRAD-285F07173F3E"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_30043B9C2264",
                fixture_ids.contains("COMFY-AUTOGRAD-30043B9C2264"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_304CC342AC2A",
                fixture_ids.contains("COMFY-AUTOGRAD-304CC342AC2A"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_35DAFB8F8753",
                fixture_ids.contains("COMFY-AUTOGRAD-35DAFB8F8753"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_3CBCCC7F6931",
                fixture_ids.contains("COMFY-AUTOGRAD-3CBCCC7F6931"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_4CF4D676FFBB",
                fixture_ids.contains("COMFY-AUTOGRAD-4CF4D676FFBB"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_58A5B3D9CFE8",
                fixture_ids.contains("COMFY-AUTOGRAD-58A5B3D9CFE8"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_617621E1EEBE",
                fixture_ids.contains("COMFY-AUTOGRAD-617621E1EEBE"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_619FFDF53F34",
                fixture_ids.contains("COMFY-AUTOGRAD-619FFDF53F34"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_640C4BF17167",
                fixture_ids.contains("COMFY-AUTOGRAD-640C4BF17167"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_75400A23E6BE",
                fixture_ids.contains("COMFY-AUTOGRAD-75400A23E6BE"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_77E715FA8F5B",
                fixture_ids.contains("COMFY-AUTOGRAD-77E715FA8F5B"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_885F94147CD4",
                fixture_ids.contains("COMFY-AUTOGRAD-885F94147CD4"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_97F154ABF757",
                fixture_ids.contains("COMFY-AUTOGRAD-97F154ABF757"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_9A036C261AF5",
                fixture_ids.contains("COMFY-AUTOGRAD-9A036C261AF5"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_A1ACCD3F23F9",
                fixture_ids.contains("COMFY-AUTOGRAD-A1ACCD3F23F9"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_A1FE605E0A41",
                fixture_ids.contains("COMFY-AUTOGRAD-A1FE605E0A41"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_A50883A5EA1D",
                fixture_ids.contains("COMFY-AUTOGRAD-A50883A5EA1D"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_ABC8AAD8B0B5",
                fixture_ids.contains("COMFY-AUTOGRAD-ABC8AAD8B0B5"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_B16B6C3AAC27",
                fixture_ids.contains("COMFY-AUTOGRAD-B16B6C3AAC27"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_B575430CB29A",
                fixture_ids.contains("COMFY-AUTOGRAD-B575430CB29A"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_B6C63329EB83",
                fixture_ids.contains("COMFY-AUTOGRAD-B6C63329EB83"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_B93A2676328D",
                fixture_ids.contains("COMFY-AUTOGRAD-B93A2676328D"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_BC03B0A46C6A",
                fixture_ids.contains("COMFY-AUTOGRAD-BC03B0A46C6A"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_C235B5282FB7",
                fixture_ids.contains("COMFY-AUTOGRAD-C235B5282FB7"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_CBC045CBB408",
                fixture_ids.contains("COMFY-AUTOGRAD-CBC045CBB408"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_E31FB2A11AFF",
                fixture_ids.contains("COMFY-AUTOGRAD-E31FB2A11AFF"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_E50C96639633",
                fixture_ids.contains("COMFY-AUTOGRAD-E50C96639633"),
            ),
            (
                "fixture_COMFY_AUTOGRAD_F5EB56FAE2E4",
                fixture_ids.contains("COMFY-AUTOGRAD-F5EB56FAE2E4"),
            ),
            ("no_grad_and_inference_do_not_record", modes_do_not_record),
        ]);
        let failed = cases
            .iter()
            .filter_map(|(case, passed)| (!passed).then_some(*case))
            .collect::<Vec<_>>();
        assert!(failed.is_empty(), "core autograd cases failed: {failed:?}");
        Ok(())
    }

    struct PassthroughBothRule;

    impl BackwardRule for PassthroughBothRule {
        fn vjp(
            &self,
            output_gradients: &[Option<Tensor>],
            _saved_tensors: &[SavedTensor],
            cancellation: &CancellationToken,
        ) -> Result<Vec<Option<Tensor>>, AutogradError> {
            cancellation_check(cancellation)?;
            let gradient = output_gradients.first().cloned().flatten();
            Ok(vec![gradient.clone(), gradient])
        }
    }
}
