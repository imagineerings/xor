use crate::{
    GENERATED_SCHEDULER_DEFINITIONS, SamplingError, SamplingPlan, SamplingProfile,
    SamplingProfileError, validate_identifier,
};
use comfy_tensor::{CpuBackend, CpuWorkspaceVec, ExecutionContext, TensorError};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const NORMAL_SCHEDULER_FEATURE_ID: &str = "COMFY-MODEL-0209";
pub const SIMPLE_SCHEDULER_FEATURE_ID: &str = "COMFY-MODEL-0211";
pub const NORMAL_SCHEDULER_ID: &str = "normal";
pub const SIMPLE_SCHEDULER_ID: &str = "simple";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SchedulerIdentity(String);

impl SchedulerIdentity {
    pub fn new(value: impl Into<String>) -> Result<Self, SchedulerError> {
        let value = value.into();
        validate_identifier("scheduler", &value)
            .map_err(|_| SchedulerError::InvalidIdentity(value.clone()))?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SchedulerIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedulerDefinition {
    pub identity: &'static str,
    pub feature_id: &'static str,
    pub source_ordinal: u16,
    pub aliases: &'static [&'static str],
    pub implementation_module: &'static str,
}

pub const SIMPLE_FOUNDATION_DEFINITION: SchedulerDefinition = SchedulerDefinition {
    identity: SIMPLE_SCHEDULER_ID,
    feature_id: SIMPLE_SCHEDULER_FEATURE_ID,
    source_ordinal: 0,
    aliases: &[],
    implementation_module: "catalog/simple",
};

pub const NORMAL_FOUNDATION_DEFINITION: SchedulerDefinition = SchedulerDefinition {
    identity: NORMAL_SCHEDULER_ID,
    feature_id: NORMAL_SCHEDULER_FEATURE_ID,
    source_ordinal: 6,
    aliases: &[],
    implementation_module: "schedulers/native_diffusion",
};

#[derive(Clone, Debug)]
pub struct SchedulerRegistry {
    definitions: Vec<SchedulerDefinition>,
    lookup: BTreeMap<String, usize>,
}

impl SchedulerRegistry {
    pub fn foundational() -> Result<Self, SchedulerError> {
        let mut definitions = GENERATED_SCHEDULER_DEFINITIONS.to_vec();
        for fallback in [SIMPLE_FOUNDATION_DEFINITION, NORMAL_FOUNDATION_DEFINITION] {
            if !definitions
                .iter()
                .any(|definition| definition.identity == fallback.identity)
            {
                definitions.push(fallback);
            }
        }
        Self::new(definitions)
    }

    pub fn new(mut definitions: Vec<SchedulerDefinition>) -> Result<Self, SchedulerError> {
        definitions.sort_by_key(|definition| definition.source_ordinal);
        let mut identities = BTreeSet::new();
        let mut features = BTreeSet::new();
        let mut ordinals = BTreeSet::new();
        let mut modules = BTreeSet::new();
        let mut lookup = BTreeMap::new();
        for (index, definition) in definitions.iter().enumerate() {
            SchedulerIdentity::new(definition.identity)?;
            validate_feature_id(definition.feature_id)?;
            if !identities.insert(definition.identity) {
                return Err(SchedulerError::DuplicateIdentity(
                    definition.identity.to_owned(),
                ));
            }
            if !features.insert(definition.feature_id) {
                return Err(SchedulerError::DuplicateFeatureId(
                    definition.feature_id.to_owned(),
                ));
            }
            if !ordinals.insert(definition.source_ordinal) {
                return Err(SchedulerError::DuplicateSourceOrdinal(
                    definition.source_ordinal,
                ));
            }
            if !modules.insert(definition.implementation_module) {
                return Err(SchedulerError::DuplicateImplementationModule(
                    definition.implementation_module.to_owned(),
                ));
            }
            insert_lookup(&mut lookup, definition.identity, index)?;
            for alias in definition.aliases {
                SchedulerIdentity::new(*alias)?;
                insert_lookup(&mut lookup, alias, index)?;
            }
        }
        if definitions.first().map(|definition| definition.identity) != Some(SIMPLE_SCHEDULER_ID) {
            return Err(SchedulerError::InvalidDefault);
        }
        Ok(Self {
            definitions,
            lookup,
        })
    }

    pub fn default_definition(&self) -> &SchedulerDefinition {
        &self.definitions[0]
    }

    pub fn resolve(
        &self,
        identity: &SchedulerIdentity,
    ) -> Result<&SchedulerDefinition, SchedulerError> {
        self.lookup
            .get(identity.as_str())
            .and_then(|index| self.definitions.get(*index))
            .ok_or_else(|| SchedulerError::Unknown(identity.as_str().to_owned()))
    }

    pub fn definitions(&self) -> &[SchedulerDefinition] {
        &self.definitions
    }
}

fn insert_lookup(
    lookup: &mut BTreeMap<String, usize>,
    value: &str,
    index: usize,
) -> Result<(), SchedulerError> {
    if lookup.insert(value.to_owned(), index).is_some() {
        return Err(SchedulerError::DuplicateAlias(value.to_owned()));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SchedulerRequestWire")]
pub struct SchedulerRequest {
    pub identity: SchedulerIdentity,
    pub steps: u32,
    pub denoise: f32,
    pub start_step: Option<u32>,
    pub end_step: Option<u32>,
    #[serde(default)]
    pub penultimate_sigma_policy: PenultimateSigmaPolicy,
}

#[derive(Deserialize)]
struct SchedulerRequestWire {
    identity: SchedulerIdentity,
    steps: u32,
    denoise: f32,
    start_step: Option<u32>,
    end_step: Option<u32>,
    #[serde(default)]
    penultimate_sigma_policy: PenultimateSigmaPolicy,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PenultimateSigmaPolicy {
    #[default]
    Keep,
    Discard,
}

impl SchedulerRequest {
    pub fn new(
        identity: impl Into<String>,
        steps: u32,
        denoise: f32,
    ) -> Result<Self, SchedulerError> {
        let request = Self {
            identity: SchedulerIdentity::new(identity)?,
            steps,
            denoise,
            start_step: None,
            end_step: None,
            penultimate_sigma_policy: PenultimateSigmaPolicy::Keep,
        };
        request.validate_bounds()?;
        Ok(request)
    }

    pub fn for_sampling_plan(plan: &SamplingPlan) -> Result<Self, SchedulerError> {
        let policy = match plan.sampler().as_str() {
            "dpm_2" | "dpm_2_ancestral" | "uni_pc" | "uni_pc_bh2" => {
                PenultimateSigmaPolicy::Discard
            }
            _ => PenultimateSigmaPolicy::Keep,
        };
        Ok(
            Self::new(plan.scheduler().as_str(), plan.steps(), plan.denoise())?
                .with_penultimate_sigma_policy(policy),
        )
    }

    pub fn with_window(
        mut self,
        start_step: Option<u32>,
        end_step: Option<u32>,
    ) -> Result<Self, SchedulerError> {
        self.start_step = start_step;
        self.end_step = end_step;
        self.validate_bounds()?;
        Ok(self)
    }

    pub fn with_penultimate_sigma_policy(mut self, policy: PenultimateSigmaPolicy) -> Self {
        self.penultimate_sigma_policy = policy;
        self
    }

    pub fn validate(
        &self,
        registry: &SchedulerRegistry,
        profile: &impl SamplingProfile,
    ) -> Result<(), SchedulerError> {
        registry.resolve(&self.identity)?;
        self.validate_bounds()?;
        if profile.sigma_count() < 2 {
            return Err(SchedulerError::InvalidProfile(
                "profile must contain at least two sigmas".to_owned(),
            ));
        }
        Ok(())
    }

    fn validate_bounds(&self) -> Result<(), SchedulerError> {
        if self.steps == 0 {
            return Err(SchedulerError::ZeroSteps);
        }
        if !self.denoise.is_finite() || self.denoise <= 0.0 || self.denoise > 1.0 {
            return Err(SchedulerError::InvalidDenoise(self.denoise));
        }
        let start = self.start_step.unwrap_or(0);
        let end = self.end_step.unwrap_or(self.steps);
        if start >= end || end > self.steps {
            return Err(SchedulerError::InvalidWindow {
                start,
                end,
                steps: self.steps,
            });
        }
        Ok(())
    }
}

impl TryFrom<SchedulerRequestWire> for SchedulerRequest {
    type Error = SchedulerError;

    fn try_from(wire: SchedulerRequestWire) -> Result<Self, Self::Error> {
        Ok(Self::new(wire.identity.as_str(), wire.steps, wire.denoise)?
            .with_window(wire.start_step, wire.end_step)?
            .with_penultimate_sigma_policy(wire.penultimate_sigma_policy))
    }
}

pub fn normal_schedule(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    registry: &SchedulerRegistry,
    profile: &impl SamplingProfile,
    request: &SchedulerRequest,
) -> Result<Vec<f32>, SchedulerError> {
    normal_schedule_with_mode(
        backend,
        context,
        registry,
        profile,
        request,
        NORMAL_SCHEDULER_ID,
        false,
    )
}

pub fn normal_schedule_with_mode<P: SamplingProfile>(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    registry: &SchedulerRegistry,
    profile: &P,
    request: &SchedulerRequest,
    expected_identity: &'static str,
    sgm_uniform: bool,
) -> Result<Vec<f32>, SchedulerError> {
    build_scheduler_schedule(
        backend,
        context,
        registry,
        profile,
        request,
        expected_identity,
        |effective_steps, profile, context, full| {
            let last_time = (profile.sigma_count() - 1) as f32;
            let end_is_zero = profile.sigma_at_index(0)?.abs() <= 0.00001;
            let schedule_steps = if !sgm_uniform && end_is_zero {
                effective_steps
                    .checked_add(1)
                    .ok_or(SchedulerError::StepOverflow)?
            } else {
                effective_steps
            };
            if schedule_steps == 1 {
                full.try_push(profile.sigma_at_model_time(last_time)?)?;
            } else {
                for index in 0..schedule_steps {
                    if index.is_multiple_of(256) {
                        context
                            .cancellation
                            .check()
                            .map_err(|_| SchedulerError::Cancelled)?;
                    }
                    let denominator = if sgm_uniform {
                        schedule_steps
                    } else {
                        schedule_steps - 1
                    };
                    let fraction = index as f32 / denominator as f32;
                    let model_time = (0.0 - last_time).mul_add(fraction, last_time);
                    full.try_push(profile.sigma_at_model_time(model_time)?)?;
                }
            }
            if sgm_uniform || !end_is_zero {
                full.try_push(0.0)?;
            }
            Ok(())
        },
    )
}

pub fn build_scheduler_schedule<P, F>(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    registry: &SchedulerRegistry,
    profile: &P,
    request: &SchedulerRequest,
    expected_identity: &'static str,
    equation: F,
) -> Result<Vec<f32>, SchedulerError>
where
    P: SamplingProfile,
    F: FnOnce(
        usize,
        &P,
        &ExecutionContext<'_>,
        &mut CpuWorkspaceVec<f32>,
    ) -> Result<(), SchedulerError>,
{
    build_scheduler_schedule_with_capacity(
        backend,
        context,
        registry,
        profile,
        request,
        expected_identity,
        1,
        equation,
    )
}

pub(crate) fn build_scheduler_schedule_with_capacity<P, F>(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
    registry: &SchedulerRegistry,
    profile: &P,
    request: &SchedulerRequest,
    expected_identity: &'static str,
    equation_extra_capacity: usize,
    equation: F,
) -> Result<Vec<f32>, SchedulerError>
where
    P: SamplingProfile,
    F: FnOnce(
        usize,
        &P,
        &ExecutionContext<'_>,
        &mut CpuWorkspaceVec<f32>,
    ) -> Result<(), SchedulerError>,
{
    if !(1..=2).contains(&equation_extra_capacity) {
        return Err(SchedulerError::InvalidEquationCapacity(
            equation_extra_capacity,
        ));
    }
    request.validate(registry, profile)?;
    if request.identity.as_str() != expected_identity {
        return Err(SchedulerError::AlgorithmMismatch {
            expected: expected_identity,
            actual: request.identity.as_str().to_owned(),
        });
    }
    context
        .cancellation
        .check()
        .map_err(|_| SchedulerError::Cancelled)?;
    let mut effective_steps = if request.denoise > 0.9999 {
        usize::try_from(request.steps).map_err(|_| SchedulerError::StepOverflow)?
    } else {
        let expanded = ((f64::from(request.steps)) / f64::from(request.denoise)).floor();
        if !expanded.is_finite() || expanded < 1.0 || expanded > usize::MAX as f64 {
            return Err(SchedulerError::StepOverflow);
        }
        expanded as usize
    };
    if request.penultimate_sigma_policy == PenultimateSigmaPolicy::Discard {
        effective_steps = effective_steps
            .checked_add(1)
            .ok_or(SchedulerError::StepOverflow)?;
    }
    let capacity = effective_steps
        .checked_add(equation_extra_capacity)
        .ok_or(SchedulerError::StepOverflow)?;
    let mut full = backend.workspace_vec::<f32>(context, capacity)?;
    equation(effective_steps, profile, context, &mut full)?;
    context
        .cancellation
        .check()
        .map_err(|_| SchedulerError::Cancelled)?;
    if full.is_empty() {
        return Err(SchedulerError::EmptySchedule);
    }
    if let Some((index, value)) = full
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(SchedulerError::NonFiniteSigma { index, value });
    }

    let discarded_index = if request.penultimate_sigma_policy == PenultimateSigmaPolicy::Discard {
        if full.len() < 3 {
            return Err(SchedulerError::PenultimateSigmaUnavailable);
        }
        Some(full.len() - 2)
    } else {
        None
    };
    let post_policy_length = full.len() - usize::from(discarded_index.is_some());
    let mut post_policy = Vec::new();
    post_policy
        .try_reserve_exact(post_policy_length)
        .map_err(|_| SchedulerError::OutOfMemory("penultimate sigma policy"))?;
    post_policy.extend(
        full.iter()
            .enumerate()
            .filter_map(|(index, value)| (Some(index) != discarded_index).then_some(*value)),
    );

    let requested_steps =
        usize::try_from(request.steps).map_err(|_| SchedulerError::StepOverflow)?;
    let requested_schedule_length = requested_steps
        .checked_add(1)
        .ok_or(SchedulerError::StepOverflow)?;
    let denoise_start = if request.denoise > 0.9999 {
        0
    } else {
        post_policy.len().saturating_sub(requested_schedule_length)
    };
    let denoised = post_policy
        .get(denoise_start..)
        .ok_or(SchedulerError::StepOverflow)?;
    let start = request
        .start_step
        .map(usize::try_from)
        .transpose()
        .map_err(|_| SchedulerError::StepOverflow)?;
    let end = request
        .end_step
        .map(usize::try_from)
        .transpose()
        .map_err(|_| SchedulerError::StepOverflow)?;
    let end_exclusive = match end {
        Some(end) if end < denoised.len().saturating_sub(1) => {
            end.checked_add(1).ok_or(SchedulerError::StepOverflow)?
        }
        _ => denoised.len(),
    };
    let ended = denoised
        .get(..end_exclusive)
        .ok_or(SchedulerError::StepOverflow)?;
    let selected = match start {
        Some(start) if start < ended.len().saturating_sub(1) => {
            ended.get(start..).ok_or(SchedulerError::StepOverflow)?
        }
        Some(_) => &[],
        None => ended,
    };
    let mut selected_schedule = Vec::new();
    selected_schedule
        .try_reserve_exact(selected.len())
        .map_err(|_| SchedulerError::OutOfMemory("selected sigma schedule"))?;
    selected_schedule.extend_from_slice(selected);
    context
        .cancellation
        .check()
        .map_err(|_| SchedulerError::Cancelled)?;
    Ok(selected_schedule)
}

fn validate_feature_id(value: &str) -> Result<(), SchedulerError> {
    if value.len() != "COMFY-MODEL-0000".len()
        || !value.starts_with("COMFY-MODEL-")
        || !value["COMFY-MODEL-".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_digit())
    {
        return Err(SchedulerError::InvalidFeatureId(value.to_owned()));
    }
    Ok(())
}

impl From<SchedulerError> for SamplingError {
    fn from(error: SchedulerError) -> Self {
        SamplingError::Scheduler(error.to_string())
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum SchedulerError {
    #[error("invalid scheduler identity {0:?}")]
    InvalidIdentity(String),
    #[error("invalid scheduler feature ID {0:?}")]
    InvalidFeatureId(String),
    #[error("duplicate scheduler identity {0:?}")]
    DuplicateIdentity(String),
    #[error("duplicate scheduler feature ID {0:?}")]
    DuplicateFeatureId(String),
    #[error("duplicate scheduler source ordinal {0}")]
    DuplicateSourceOrdinal(u16),
    #[error("duplicate scheduler implementation module {0:?}")]
    DuplicateImplementationModule(String),
    #[error("duplicate scheduler identity or alias {0:?}")]
    DuplicateAlias(String),
    #[error("the source-order scheduler default must be simple")]
    InvalidDefault,
    #[error("unknown scheduler {0:?}")]
    Unknown(String),
    #[error("scheduler steps must be nonzero")]
    ZeroSteps,
    #[error("scheduler denoise must be finite and in (0, 1], got {0}")]
    InvalidDenoise(f32),
    #[error("scheduler window {start}..{end} is invalid for {steps} steps")]
    InvalidWindow { start: u32, end: u32, steps: u32 },
    #[error("scheduler algorithm mismatch: expected {expected}, got {actual}")]
    AlgorithmMismatch {
        expected: &'static str,
        actual: String,
    },
    #[error("invalid sampling profile: {0}")]
    InvalidProfile(String),
    #[error("scheduler step arithmetic overflowed")]
    StepOverflow,
    #[error("scheduler cannot discard a penultimate sigma from fewer than three values")]
    PenultimateSigmaUnavailable,
    #[error("scheduler equation produced an empty sigma schedule")]
    EmptySchedule,
    #[error("scheduler equation produced non-finite sigma {value} at index {index}")]
    NonFiniteSigma { index: usize, value: f32 },
    #[error("scheduler equation requested invalid extra capacity {0}; expected one or two")]
    InvalidEquationCapacity(usize),
    #[error("scheduler allocation failed for {0}")]
    OutOfMemory(&'static str),
    #[error("scheduler operation was cancelled")]
    Cancelled,
    #[error(transparent)]
    Profile(#[from] SamplingProfileError),
    #[error(transparent)]
    Tensor(#[from] TensorError),
}
