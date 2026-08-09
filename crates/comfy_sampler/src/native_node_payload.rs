use crate::{
    GuidanceDenoiser, GuidanceError, GuidanceHook, GuidanceOptions, GuidanceResult, NoiseError,
    NoiseRequest, SamplerIdentity, SamplerRegistry, SamplingPlan, SamplingProfile, SamplingTrace,
    execute_guidance,
    generated_native_diffusion::{NativeDiffusionSamplerError, sample_euler},
};
use comfy_model::conditioning::{ConditioningError, ConditioningSet};
use comfy_model::{NativeModelPayload, NativeModelPayloadError, NativeModelResourceRole};
use comfy_tensor::{
    CpuBackend, DType, DeviceId, ExecutionContext, RngCheckpoint, RngError, Tensor, TensorError,
    generated_native_diffusion::{NativeDiffusionTensorError, tensor_from_f32},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    mem,
    sync::Arc,
};
use thiserror::Error;

const MAX_BATCH_NOISE_INDEX: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeNoiseSourceKind {
    Zero,
    Random,
    FixedLatent,
}

#[derive(Clone)]
enum NativeNoiseResource {
    Zero,
    Random { seed: u64 },
    FixedLatent { seed: u64, samples: Tensor },
}

#[derive(Clone)]
pub struct NativeNoisePayload {
    resource: NativeNoiseResource,
    semantic_digest_sha256: String,
    resident_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct NativeNoiseGeneration {
    pub noise: Tensor,
    pub before: Option<RngCheckpoint>,
    pub after: Option<RngCheckpoint>,
}

impl NativeNoisePayload {
    pub fn zero() -> Result<Self, NativeSamplerPayloadError> {
        Self::checked(NativeNoiseResource::Zero)
    }

    pub fn random(seed: u64) -> Result<Self, NativeSamplerPayloadError> {
        Self::checked(NativeNoiseResource::Random { seed })
    }

    pub fn fixed_latent(seed: u64, samples: Tensor) -> Result<Self, NativeSamplerPayloadError> {
        Self::checked(NativeNoiseResource::FixedLatent { seed, samples })
    }

    pub const fn kind(&self) -> NativeNoiseSourceKind {
        match &self.resource {
            NativeNoiseResource::Zero => NativeNoiseSourceKind::Zero,
            NativeNoiseResource::Random { .. } => NativeNoiseSourceKind::Random,
            NativeNoiseResource::FixedLatent { .. } => NativeNoiseSourceKind::FixedLatent,
        }
    }

    pub const fn seed(&self) -> u64 {
        match &self.resource {
            NativeNoiseResource::Zero => 0,
            NativeNoiseResource::Random { seed }
            | NativeNoiseResource::FixedLatent { seed, .. } => *seed,
        }
    }

    pub fn semantic_digest_sha256(&self) -> &str {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn validate(&self) -> Result<(), NativeSamplerPayloadError> {
        let expected = Self::checked(self.resource.clone())?;
        if self.semantic_digest_sha256 != expected.semantic_digest_sha256
            || self.resident_bytes != expected.resident_bytes
        {
            return Err(NativeSamplerPayloadError::ProjectionChanged("noise"));
        }
        Ok(())
    }

    pub fn generate(
        &self,
        backend: &CpuBackend,
        input_latent: &Tensor,
        batch_indices: Option<&[u64]>,
        request: &NoiseRequest,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeNoiseGeneration, NativeSamplerPayloadError> {
        context.check()?;
        self.validate()?;
        validate_noise_input(input_latent, batch_indices)?;
        match &self.resource {
            NativeNoiseResource::Zero => {
                let count = tensor_element_count(input_latent)?;
                let values = vec![0.0_f32; count];
                Ok(NativeNoiseGeneration {
                    noise: tensor_from_f32(
                        backend,
                        input_latent.descriptor().shape(),
                        &values,
                        context,
                    )?,
                    before: None,
                    after: None,
                })
            }
            NativeNoiseResource::Random { seed } => generate_random_noise(
                backend,
                input_latent,
                batch_indices,
                request,
                *seed,
                context,
            ),
            NativeNoiseResource::FixedLatent { samples, .. } => {
                if samples.descriptor() != input_latent.descriptor() {
                    return Err(NativeSamplerPayloadError::NoiseShapeMismatch);
                }
                context.check()?;
                Ok(NativeNoiseGeneration {
                    noise: samples.clone(),
                    before: None,
                    after: None,
                })
            }
        }
    }

    fn checked(resource: NativeNoiseResource) -> Result<Self, NativeSamplerPayloadError> {
        let mut hasher = Sha256::new();
        hasher.update(b"sim.comfy.native-noise-payload.v1");
        hasher.update([0]);
        let mut resident_bytes = usize_to_u64(mem::size_of::<Self>())?;
        match &resource {
            NativeNoiseResource::Zero => hasher.update(b"zero"),
            NativeNoiseResource::Random { seed } => {
                hasher.update(b"random");
                hasher.update(seed.to_le_bytes());
            }
            NativeNoiseResource::FixedLatent { seed, samples } => {
                validate_noise_tensor(samples)?;
                hasher.update(b"fixed-latent");
                hasher.update(seed.to_le_bytes());
                hash_tensor(&mut hasher, samples)?;
                resident_bytes = checked_add(resident_bytes, samples.storage_byte_len())?;
            }
        }
        let semantic_digest_sha256 = format!("{:x}", hasher.finalize());
        resident_bytes = checked_add(
            resident_bytes,
            usize_to_u64(semantic_digest_sha256.capacity())?,
        )?;
        Ok(Self {
            resource,
            semantic_digest_sha256,
            resident_bytes,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeGuiderKind {
    Basic,
    Cfg,
}

#[derive(Clone, Copy, Debug)]
pub enum NativeGuiderConditioningSets<'a> {
    Basic {
        conditioning: &'a Arc<ConditioningSet>,
    },
    Cfg {
        positive: &'a Arc<ConditioningSet>,
        negative: &'a Arc<ConditioningSet>,
    },
}

#[derive(Clone)]
enum NativeGuiderStrategy {
    Basic {
        conditioning: Arc<ConditioningSet>,
    },
    Cfg {
        positive: Arc<ConditioningSet>,
        negative: Arc<ConditioningSet>,
        guidance: f32,
    },
}

#[derive(Clone)]
pub struct NativeGuiderPayload {
    model: Arc<NativeModelPayload>,
    strategy: NativeGuiderStrategy,
    semantic_digest_sha256: String,
    resident_bytes: u64,
}

impl NativeGuiderPayload {
    pub fn basic(
        model: Arc<NativeModelPayload>,
        conditioning: Arc<ConditioningSet>,
    ) -> Result<Self, NativeSamplerPayloadError> {
        Self::checked(model, NativeGuiderStrategy::Basic { conditioning })
    }

    pub fn cfg(
        model: Arc<NativeModelPayload>,
        positive: Arc<ConditioningSet>,
        negative: Arc<ConditioningSet>,
        guidance: f32,
    ) -> Result<Self, NativeSamplerPayloadError> {
        Self::checked(
            model,
            NativeGuiderStrategy::Cfg {
                positive,
                negative,
                guidance,
            },
        )
    }

    pub const fn kind(&self) -> NativeGuiderKind {
        match &self.strategy {
            NativeGuiderStrategy::Basic { .. } => NativeGuiderKind::Basic,
            NativeGuiderStrategy::Cfg { .. } => NativeGuiderKind::Cfg,
        }
    }

    pub fn model(&self) -> &Arc<NativeModelPayload> {
        &self.model
    }

    pub fn conditioning_sets(&self) -> NativeGuiderConditioningSets<'_> {
        self.strategy.conditioning_sets()
    }

    pub fn owned_resident_bytes(&self) -> Result<u64, NativeSamplerPayloadError> {
        guider_owned_resident_bytes(self.semantic_digest_sha256.capacity())
    }

    pub fn model_execution_digest_sha256(&self) -> &str {
        self.model.identity().execution_sha256()
    }

    pub fn semantic_digest_sha256(&self) -> &str {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn guidance(&self) -> f32 {
        match &self.strategy {
            NativeGuiderStrategy::Basic { .. } => 1.0,
            NativeGuiderStrategy::Cfg { guidance, .. } => *guidance,
        }
    }

    pub fn validate(&self) -> Result<(), NativeSamplerPayloadError> {
        let expected = Self::checked(self.model.clone(), self.strategy.clone())?;
        if expected.semantic_digest_sha256 != self.semantic_digest_sha256
            || expected.resident_bytes != self.resident_bytes
        {
            return Err(NativeSamplerPayloadError::ProjectionChanged("guider"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        backend: &CpuBackend,
        latent: &Tensor,
        sigma: f32,
        profile: &dyn SamplingProfile,
        plan: &SamplingPlan,
        options: GuidanceOptions,
        denoiser: &mut dyn GuidanceDenoiser,
        hooks: &mut [&mut dyn GuidanceHook],
        context: &ExecutionContext<'_>,
    ) -> Result<GuidanceResult, NativeSamplerPayloadError> {
        context.check()?;
        self.validate()?;
        if plan.guidance().to_bits() != self.guidance().to_bits() {
            return Err(NativeSamplerPayloadError::GuiderPlanMismatch);
        }
        Self::execute_strategy(
            &self.strategy,
            backend,
            latent,
            sigma,
            profile,
            plan,
            options,
            denoiser,
            hooks,
            context,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_strategy(
        strategy: &NativeGuiderStrategy,
        backend: &CpuBackend,
        latent: &Tensor,
        sigma: f32,
        profile: &dyn SamplingProfile,
        plan: &SamplingPlan,
        options: GuidanceOptions,
        denoiser: &mut dyn GuidanceDenoiser,
        hooks: &mut [&mut dyn GuidanceHook],
        context: &ExecutionContext<'_>,
    ) -> Result<GuidanceResult, NativeSamplerPayloadError> {
        let (positive, negative) = match strategy {
            NativeGuiderStrategy::Basic { conditioning } => {
                (conditioning.as_ref(), conditioning.as_ref())
            }
            NativeGuiderStrategy::Cfg {
                positive, negative, ..
            } => (positive.as_ref(), negative.as_ref()),
        };
        Ok(execute_guidance(
            backend, latent, sigma, profile, plan, positive, negative, options, denoiser, hooks,
            context,
        )?)
    }

    fn checked(
        model: Arc<NativeModelPayload>,
        strategy: NativeGuiderStrategy,
    ) -> Result<Self, NativeSamplerPayloadError> {
        if model.identity().role() != NativeModelResourceRole::Model || model.model().is_none() {
            return Err(NativeSamplerPayloadError::GuiderModelRoleMismatch);
        }
        model.validate()?;
        let mut hasher = Sha256::new();
        hasher.update(b"sim.comfy.native-guider-payload.v1");
        hash_field(&mut hasher, model.identity().digest_sha256().as_bytes())?;
        let mut resident_bytes = usize_to_u64(mem::size_of::<Self>())?;
        resident_bytes = checked_add(resident_bytes, model.resident_bytes())?;
        match &strategy {
            NativeGuiderStrategy::Basic { conditioning } => {
                hasher.update(b"basic");
                validate_conditioning_digest(conditioning)?;
                hash_field(&mut hasher, conditioning.digest().as_bytes())?;
                resident_bytes = checked_add(resident_bytes, conditioning.resident_bytes()?)?;
            }
            NativeGuiderStrategy::Cfg {
                positive,
                negative,
                guidance,
            } => {
                if !guidance.is_finite() || *guidance < 0.0 {
                    return Err(NativeSamplerPayloadError::InvalidGuidance(*guidance));
                }
                if positive.identity() != negative.identity() {
                    return Err(NativeSamplerPayloadError::ConditioningIdentityMismatch);
                }
                validate_conditioning_digest(positive)?;
                validate_conditioning_digest(negative)?;
                hasher.update(b"cfg");
                hasher.update(guidance.to_bits().to_le_bytes());
                hash_field(&mut hasher, positive.digest().as_bytes())?;
                hash_field(&mut hasher, negative.digest().as_bytes())?;
                resident_bytes = checked_add(
                    resident_bytes,
                    ConditioningSet::combined_resident_bytes(&[
                        positive.as_ref(),
                        negative.as_ref(),
                    ])?,
                )?;
            }
        }
        let semantic_digest_sha256 = format!("{:x}", hasher.finalize());
        resident_bytes = checked_add(
            resident_bytes,
            usize_to_u64(semantic_digest_sha256.capacity())?,
        )?;
        Ok(Self {
            model,
            strategy,
            semantic_digest_sha256,
            resident_bytes,
        })
    }
}

impl NativeGuiderStrategy {
    fn conditioning_sets(&self) -> NativeGuiderConditioningSets<'_> {
        match self {
            Self::Basic { conditioning } => NativeGuiderConditioningSets::Basic { conditioning },
            Self::Cfg {
                positive, negative, ..
            } => NativeGuiderConditioningSets::Cfg { positive, negative },
        }
    }
}

fn guider_owned_resident_bytes(
    semantic_digest_capacity: usize,
) -> Result<u64, NativeSamplerPayloadError> {
    checked_add(
        usize_to_u64(mem::size_of::<NativeGuiderPayload>())?,
        usize_to_u64(semantic_digest_capacity)?,
    )
}

pub trait NativeSamplerDenoiser {
    fn denoise(&mut self, input: &Tensor, sigma: f32, step: usize) -> Result<Tensor, String>;
}

impl<Function> NativeSamplerDenoiser for Function
where
    Function: FnMut(&Tensor, f32, usize) -> Result<Tensor, String>,
{
    fn denoise(&mut self, input: &Tensor, sigma: f32, step: usize) -> Result<Tensor, String> {
        self(input, sigma, step)
    }
}

#[derive(Clone, Copy)]
enum NativeSamplerAlgorithm {
    Euler,
}

#[derive(Clone)]
pub struct NativeSamplerPayload {
    identity: SamplerIdentity,
    implementation_digest_sha256: String,
    semantic_digest_sha256: String,
    resident_bytes: u64,
    algorithm: NativeSamplerAlgorithm,
}

impl NativeSamplerPayload {
    fn checked(algorithm: NativeSamplerAlgorithm) -> Result<Self, NativeSamplerPayloadError> {
        let (sampler_identity, implementation_digest_sha256) = match algorithm {
            NativeSamplerAlgorithm::Euler => (
                "euler",
                "671804051cfd41de9a11f5f01dd9219008009403c29295d636523cc184a48327",
            ),
        };
        let registry = SamplerRegistry::foundational()?;
        let identity = SamplerIdentity::new(sampler_identity)?;
        registry.resolve(&identity)?;
        validate_sha256("sampler implementation", implementation_digest_sha256)?;
        let mut hasher = Sha256::new();
        hasher.update(b"sim.comfy.native-sampler-payload.v1");
        hash_field(&mut hasher, identity.as_str().as_bytes())?;
        hash_field(&mut hasher, implementation_digest_sha256.as_bytes())?;
        let semantic_digest_sha256 = format!("{:x}", hasher.finalize());
        let resident_bytes = [
            usize_to_u64(mem::size_of::<Self>())?,
            usize_to_u64(identity.as_str().len())?,
            usize_to_u64(implementation_digest_sha256.len())?,
            usize_to_u64(semantic_digest_sha256.capacity())?,
            usize_to_u64(mem::size_of::<NativeSamplerAlgorithm>())?,
        ]
        .into_iter()
        .try_fold(0_u64, checked_add)?;
        Ok(Self {
            identity,
            implementation_digest_sha256: implementation_digest_sha256.to_owned(),
            semantic_digest_sha256,
            resident_bytes,
            algorithm,
        })
    }

    pub fn euler() -> Result<Self, NativeSamplerPayloadError> {
        Self::checked(NativeSamplerAlgorithm::Euler)
    }

    pub fn identity(&self) -> &SamplerIdentity {
        &self.identity
    }

    pub fn implementation_digest_sha256(&self) -> &str {
        &self.implementation_digest_sha256
    }

    pub fn semantic_digest_sha256(&self) -> &str {
        &self.semantic_digest_sha256
    }

    pub const fn resident_bytes(&self) -> u64 {
        self.resident_bytes
    }

    pub fn validate(&self) -> Result<(), NativeSamplerPayloadError> {
        let expected = Self::checked(self.algorithm)?;
        if expected.identity != self.identity
            || expected.implementation_digest_sha256 != self.implementation_digest_sha256
            || expected.semantic_digest_sha256 != self.semantic_digest_sha256
            || expected.resident_bytes != self.resident_bytes
        {
            return Err(NativeSamplerPayloadError::ProjectionChanged("sampler"));
        }
        Ok(())
    }

    pub fn execute(
        &self,
        backend: &CpuBackend,
        initial: Tensor,
        sigmas: &[f32],
        context: &ExecutionContext<'_>,
        denoiser: &mut dyn NativeSamplerDenoiser,
    ) -> Result<SamplingTrace, NativeSamplerPayloadError> {
        context.check()?;
        self.validate()?;
        let descriptor = initial.descriptor().clone();
        let trace = match self.algorithm {
            NativeSamplerAlgorithm::Euler => {
                sample_euler(backend, initial, sigmas, context, |input, sigma, step| {
                    denoiser.denoise(input, sigma, step)
                })?
            }
        };
        if trace.sigmas != sigmas
            || trace.latents.is_empty()
            || trace
                .latents
                .iter()
                .chain(&trace.denoiser_evaluations)
                .any(|tensor| tensor.descriptor() != &descriptor)
        {
            return Err(NativeSamplerPayloadError::SamplerOutputMismatch);
        }
        context.check()?;
        Ok(trace)
    }
}

#[derive(Debug, Error)]
pub enum NativeSamplerPayloadError {
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    TensorKernel(#[from] NativeDiffusionTensorError),
    #[error(transparent)]
    Noise(#[from] NoiseError),
    #[error(transparent)]
    Rng(#[from] RngError),
    #[error(transparent)]
    Guidance(#[from] GuidanceError),
    #[error(transparent)]
    Sampling(#[from] crate::SamplingError),
    #[error(transparent)]
    NativeSampler(#[from] NativeDiffusionSamplerError),
    #[error(transparent)]
    Model(#[from] NativeModelPayloadError),
    #[error(transparent)]
    Conditioning(#[from] ConditioningError),
    #[error("native sampler payload {0} is not a SHA-256 digest")]
    InvalidDigest(&'static str),
    #[error("native noise input must be a contiguous CPU f32 tensor with nonzero dimensions")]
    InvalidNoiseInput,
    #[error("native noise batch indices do not match the latent batch")]
    NoiseBatchMismatch,
    #[error("native noise batch index {0} exceeds the bounded skip limit")]
    NoiseBatchIndexLimit(u64),
    #[error("fixed native noise shape or target does not match the input latent")]
    NoiseShapeMismatch,
    #[error("native guider guidance must be finite and nonnegative, got {0}")]
    InvalidGuidance(f32),
    #[error("native guider positive and negative conditioning identities differ")]
    ConditioningIdentityMismatch,
    #[error("native guider conditioning digest is invalid")]
    InvalidConditioningDigest,
    #[error("native guider requires a concrete MODEL payload")]
    GuiderModelRoleMismatch,
    #[error("native guider guidance does not match the sampling plan")]
    GuiderPlanMismatch,
    #[error("native sampler output changed its requested sigma or tensor contract")]
    SamplerOutputMismatch,
    #[error("native {0} payload projection changed")]
    ProjectionChanged(&'static str),
    #[error("native sampler payload resident byte accounting overflowed")]
    ResidentBytesOverflow,
}

fn generate_random_noise(
    backend: &CpuBackend,
    input_latent: &Tensor,
    batch_indices: Option<&[u64]>,
    request: &NoiseRequest,
    seed: u64,
    context: &ExecutionContext<'_>,
) -> Result<NativeNoiseGeneration, NativeSamplerPayloadError> {
    let stream = request.stream(seed, DeviceId::CPU)?;
    if batch_indices.is_none() {
        let trace = crate::noise::normal_noise(
            backend,
            input_latent.descriptor().shape(),
            &stream,
            context,
        )?;
        return Ok(NativeNoiseGeneration {
            noise: trace.noise,
            before: Some(trace.before),
            after: Some(trace.after),
        });
    }
    let batch_indices = batch_indices.ok_or(NativeSamplerPayloadError::NoiseBatchMismatch)?;
    let shape = input_latent.descriptor().shape();
    let batch = shape
        .first()
        .copied()
        .ok_or(NativeSamplerPayloadError::InvalidNoiseInput)?;
    if usize_to_u64(batch_indices.len())? != batch {
        return Err(NativeSamplerPayloadError::NoiseBatchMismatch);
    }
    let maximum = batch_indices
        .iter()
        .copied()
        .max()
        .ok_or(NativeSamplerPayloadError::NoiseBatchMismatch)?;
    if maximum > MAX_BATCH_NOISE_INDEX {
        return Err(NativeSamplerPayloadError::NoiseBatchIndexLimit(maximum));
    }
    let per_batch_count = shape
        .get(1..)
        .ok_or(NativeSamplerPayloadError::InvalidNoiseInput)?
        .iter()
        .try_fold(1_u64, |count, dimension| {
            count
                .checked_mul(*dimension)
                .ok_or(NativeSamplerPayloadError::ResidentBytesOverflow)
        })?;
    let per_batch_count = usize::try_from(per_batch_count)
        .map_err(|_| NativeSamplerPayloadError::ResidentBytesOverflow)?;
    let wanted = batch_indices.iter().copied().collect::<BTreeSet<_>>();
    let before = stream.begin(None)?.commit();
    let mut transaction = stream.begin(Some(before.clone()))?;
    transaction.require_device(DeviceId::CPU)?;
    let mut rows = BTreeMap::new();
    for index in 0..=maximum {
        context.check()?;
        let mut row = Vec::new();
        row.try_reserve_exact(per_batch_count)
            .map_err(|_| NativeSamplerPayloadError::ResidentBytesOverflow)?;
        while row.len() < per_batch_count {
            let pair = transaction.next_standard_normal_pair(context.cancellation)?;
            row.push(pair[0] as f32);
            if row.len() < per_batch_count {
                row.push(pair[1] as f32);
            }
        }
        if wanted.contains(&index) {
            rows.insert(index, row);
        }
    }
    let output_count = tensor_element_count(input_latent)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(output_count)
        .map_err(|_| NativeSamplerPayloadError::ResidentBytesOverflow)?;
    for index in batch_indices {
        values.extend_from_slice(
            rows.get(index)
                .ok_or(NativeSamplerPayloadError::NoiseBatchMismatch)?,
        );
    }
    let after = transaction.commit();
    Ok(NativeNoiseGeneration {
        noise: tensor_from_f32(backend, shape, &values, context)?,
        before: Some(before),
        after: Some(after),
    })
}

fn validate_noise_input(
    input: &Tensor,
    batch_indices: Option<&[u64]>,
) -> Result<(), NativeSamplerPayloadError> {
    validate_noise_tensor(input)?;
    if let Some(indices) = batch_indices {
        let batch = input
            .descriptor()
            .shape()
            .first()
            .copied()
            .ok_or(NativeSamplerPayloadError::InvalidNoiseInput)?;
        if indices.is_empty() || usize_to_u64(indices.len())? != batch {
            return Err(NativeSamplerPayloadError::NoiseBatchMismatch);
        }
    }
    Ok(())
}

fn validate_noise_tensor(tensor: &Tensor) -> Result<(), NativeSamplerPayloadError> {
    let descriptor = tensor.descriptor();
    if descriptor.dtype() != DType::F32
        || descriptor.device() != DeviceId::CPU
        || !descriptor.is_contiguous()?
        || descriptor.shape().is_empty()
        || descriptor.shape().contains(&0)
    {
        return Err(NativeSamplerPayloadError::InvalidNoiseInput);
    }
    Ok(())
}

fn tensor_element_count(tensor: &Tensor) -> Result<usize, NativeSamplerPayloadError> {
    usize::try_from(tensor.descriptor().element_count()?)
        .map_err(|_| NativeSamplerPayloadError::ResidentBytesOverflow)
}

fn hash_tensor(hasher: &mut Sha256, tensor: &Tensor) -> Result<(), NativeSamplerPayloadError> {
    let descriptor = tensor.descriptor();
    hasher.update(b"sim.comfy.native-noise-tensor.v1");
    hash_field(hasher, descriptor.dtype().catalog_name().as_bytes())?;
    hasher.update(usize_to_u64(descriptor.rank())?.to_le_bytes());
    for dimension in descriptor.shape() {
        hasher.update(dimension.to_le_bytes());
    }
    hash_field(hasher, tensor.contiguous_bytes()?)
}

fn validate_conditioning_digest(
    conditioning: &ConditioningSet,
) -> Result<(), NativeSamplerPayloadError> {
    conditioning.validate()?;
    if valid_sha256(conditioning.digest()) {
        Ok(())
    } else {
        Err(NativeSamplerPayloadError::InvalidConditioningDigest)
    }
}

fn hash_field(hasher: &mut Sha256, value: &[u8]) -> Result<(), NativeSamplerPayloadError> {
    hasher.update(usize_to_u64(value.len())?.to_le_bytes());
    hasher.update(value);
    Ok(())
}

fn validate_sha256(subject: &'static str, value: &str) -> Result<(), NativeSamplerPayloadError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(NativeSamplerPayloadError::InvalidDigest(subject))
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn usize_to_u64(value: usize) -> Result<u64, NativeSamplerPayloadError> {
    u64::try_from(value).map_err(|_| NativeSamplerPayloadError::ResidentBytesOverflow)
}

fn checked_add(left: u64, right: u64) -> Result<u64, NativeSamplerPayloadError> {
    left.checked_add(right)
        .ok_or(NativeSamplerPayloadError::ResidentBytesOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DiscreteSamplingProfile;
    use comfy_model::{
        LatentFormatIdentity, ModelFamilyIdentity,
        conditioning::{
            ConditioningEntry, ConditioningEntryOptions, ConditioningIdentity, ConditioningValue,
        },
    };
    use comfy_tensor::{
        CancellationToken, CpuWorkspaceAuthority, StreamId,
        generated_native_diffusion::tensor_to_f32,
    };
    use std::error::Error;

    fn context<'a>(
        backend: &CpuBackend,
        authority: &CpuWorkspaceAuthority,
        cancellation: &'a CancellationToken,
    ) -> Result<ExecutionContext<'a>, TensorError> {
        Ok(backend.execution_context(
            StreamId::DEFAULT,
            authority.authorize_workspace(4 * 1024 * 1024)?,
            cancellation,
        ))
    }

    fn conditioning(
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
        cancellation: &CancellationToken,
        namespace: &str,
        value: f32,
    ) -> Result<Arc<ConditioningSet>, Box<dyn Error>> {
        let identity = ConditioningIdentity::new(
            namespace,
            ModelFamilyIdentity::new("COMFY-MODEL-0001", "payload-test", "v1")?,
            LatentFormatIdentity::new("COMFY-MODEL-0002", "payload_latent")?,
        )?;
        let tensor = tensor_from_f32(backend, &[1, 1, 2], &[value, value], context)?;
        Ok(Arc::new(ConditioningSet::checked(
            identity,
            vec![ConditioningEntry::checked(
                "full",
                ConditioningValue::regular(tensor)?,
                ConditioningEntryOptions::default(),
            )?],
            cancellation,
        )?))
    }

    struct ConstantDenoiser<'a> {
        backend: &'a CpuBackend,
    }

    impl GuidanceDenoiser for ConstantDenoiser<'_> {
        fn evaluate_batch(
            &mut self,
            evaluations: &[crate::GuidanceEvaluation],
            context: &ExecutionContext<'_>,
        ) -> Result<Vec<Tensor>, GuidanceError> {
            evaluations
                .iter()
                .map(|evaluation| {
                    let value = match evaluation.branch() {
                        crate::GuidanceBranch::Conditional => 3.0,
                        crate::GuidanceBranch::Unconditional => 1.0,
                    };
                    tensor_from_f32(
                        self.backend,
                        evaluation.latent().descriptor().shape(),
                        &[value, value],
                        context,
                    )
                    .map_err(GuidanceError::from)
                })
                .collect()
        }
    }

    #[test]
    fn noise_payloads_execute_deterministically_and_validate_batch_indices()
    -> Result<(), Box<dyn Error>> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(8 * 1024 * 1024)?;
        let cancellation = CancellationToken::default();
        let context = context(&backend, &authority, &cancellation)?;
        let latent = tensor_from_f32(&backend, &[3, 2], &[1.0; 6], &context)?;
        let request = NoiseRequest::native_diffusion("prompt", "noise")?;

        let zero = NativeNoisePayload::zero()?;
        let generated = zero.generate(&backend, &latent, None, &request, &context)?;
        assert_eq!(
            &*tensor_to_f32(&backend, &generated.noise, &context)?,
            &[0.0; 6]
        );
        assert!(generated.before.is_none() && generated.after.is_none());

        let random = NativeNoisePayload::random(42)?;
        let first = random.generate(&backend, &latent, Some(&[2, 0, 2]), &request, &context)?;
        let second = random.generate(&backend, &latent, Some(&[2, 0, 2]), &request, &context)?;
        let first_values = tensor_to_f32(&backend, &first.noise, &context)?;
        assert_eq!(
            &*first_values,
            &*tensor_to_f32(&backend, &second.noise, &context)?
        );
        assert_eq!(&first_values[0..2], &first_values[4..6]);
        assert_ne!(&first_values[0..2], &first_values[2..4]);
        assert!(first.before.is_some() && first.after.is_some());
        assert!(matches!(
            random.generate(&backend, &latent, Some(&[0]), &request, &context),
            Err(NativeSamplerPayloadError::NoiseBatchMismatch)
        ));
        assert_ne!(
            zero.semantic_digest_sha256(),
            random.semantic_digest_sha256()
        );
        assert!(random.resident_bytes() > 0);
        Ok(())
    }

    #[test]
    fn fixed_noise_semantic_digest_ignores_stream_and_storage_placement()
    -> Result<(), Box<dyn Error>> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(8 * 1024 * 1024)?;
        let cancellation = CancellationToken::default();
        let default_context = context(&backend, &authority, &cancellation)?;
        let other_context = backend.execution_context(
            StreamId::new(17),
            authority.authorize_workspace(4 * 1024 * 1024)?,
            &cancellation,
        );
        let first_tensor = tensor_from_f32(&backend, &[1, 1, 2], &[0.25, -0.5], &default_context)?;
        let second_tensor = tensor_from_f32(&backend, &[1, 1, 2], &[0.25, -0.5], &other_context)?;
        assert_ne!(first_tensor.storage_id(), second_tensor.storage_id());
        assert_ne!(
            first_tensor.descriptor().stream(),
            second_tensor.descriptor().stream()
        );

        let first = NativeNoisePayload::fixed_latent(19, first_tensor)?;
        let second = NativeNoisePayload::fixed_latent(19, second_tensor)?;
        assert_eq!(
            first.semantic_digest_sha256(),
            second.semantic_digest_sha256()
        );
        assert_ne!(
            first.semantic_digest_sha256(),
            NativeNoisePayload::fixed_latent(
                19,
                tensor_from_f32(&backend, &[1, 1, 2], &[0.25, -0.25], &default_context)?,
            )?
            .semantic_digest_sha256()
        );
        assert_ne!(
            first.semantic_digest_sha256(),
            NativeNoisePayload::fixed_latent(
                19,
                tensor_from_f32(&backend, &[1, 2, 1], &[0.25, -0.5], &default_context)?,
            )?
            .semantic_digest_sha256()
        );
        Ok(())
    }

    #[test]
    fn cfg_guider_strategy_executes_checked_conditioning() -> Result<(), Box<dyn Error>> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(8 * 1024 * 1024)?;
        let cancellation = CancellationToken::default();
        let context = context(&backend, &authority, &cancellation)?;
        let latent = tensor_from_f32(&backend, &[1, 1, 2], &[0.0, 0.0], &context)?;
        let positive = conditioning(&backend, &context, &cancellation, "shared", 1.0)?;
        let negative = conditioning(&backend, &context, &cancellation, "shared", 0.0)?;
        let positive_resident_bytes = positive.resident_bytes()?;
        assert_eq!(
            ConditioningSet::combined_resident_bytes(&[positive.as_ref(), positive.as_ref()])?,
            positive_resident_bytes
        );
        let cfg_resident_bytes =
            ConditioningSet::combined_resident_bytes(&[positive.as_ref(), negative.as_ref()])?;
        let positive_address = Arc::as_ptr(&positive);
        let negative_address = Arc::as_ptr(&negative);
        let strategy = NativeGuiderStrategy::Cfg {
            positive,
            negative,
            guidance: 2.0,
        };
        let NativeGuiderConditioningSets::Cfg { positive, negative } = strategy.conditioning_sets()
        else {
            return Err("CFG guider exposed basic conditioning children".into());
        };
        assert_eq!(Arc::as_ptr(positive), positive_address);
        assert_eq!(Arc::as_ptr(negative), negative_address);
        assert_eq!(
            ConditioningSet::combined_resident_bytes(&[positive.as_ref(), negative.as_ref(),])?,
            cfg_resident_bytes
        );
        let basic_strategy = NativeGuiderStrategy::Basic {
            conditioning: positive.clone(),
        };
        let NativeGuiderConditioningSets::Basic { conditioning } =
            basic_strategy.conditioning_sets()
        else {
            return Err("basic guider exposed CFG conditioning children".into());
        };
        assert_eq!(Arc::as_ptr(conditioning), positive_address);
        assert_eq!(conditioning.resident_bytes()?, positive_resident_bytes);
        assert_eq!(
            guider_owned_resident_bytes(128)?,
            usize_to_u64(mem::size_of::<NativeGuiderPayload>())?
                .checked_add(128)
                .ok_or("guider owned test byte count overflowed")?
        );
        let profile = DiscreteSamplingProfile::sd15()?;
        let plan = SamplingPlan::new(
            "euler",
            "normal",
            profile.identity().clone(),
            7,
            1,
            2.0,
            1.0,
        )?;
        let mut denoiser = ConstantDenoiser { backend: &backend };
        let guided = NativeGuiderPayload::execute_strategy(
            &strategy,
            &backend,
            &latent,
            2.0,
            &profile,
            &plan,
            GuidanceOptions::default(),
            &mut denoiser,
            &mut [],
            &context,
        )?;
        assert_eq!(
            &*tensor_to_f32(&backend, guided.guided(), &context)?,
            &[5.0, 5.0]
        );
        assert!(positive_resident_bytes > 0);
        Ok(())
    }

    #[test]
    fn fixed_noise_and_euler_are_executable_and_fail_closed() -> Result<(), Box<dyn Error>> {
        let (backend, authority) = CpuWorkspaceAuthority::create_backend(16 * 1024 * 1024)?;
        let cancellation = CancellationToken::default();
        let context = context(&backend, &authority, &cancellation)?;
        let latent = tensor_from_f32(&backend, &[1, 1, 2], &[0.0, 0.0], &context)?;
        let fixed_tensor = tensor_from_f32(&backend, &[1, 1, 2], &[0.5, -0.5], &context)?;
        let fixed = NativeNoisePayload::fixed_latent(0, fixed_tensor)?;
        let request = NoiseRequest::native_diffusion("prompt", "fixed")?;
        let generated = fixed.generate(&backend, &latent, None, &request, &context)?;
        assert_eq!(
            &*tensor_to_f32(&backend, &generated.noise, &context)?,
            &[0.5, -0.5]
        );

        let sampler = NativeSamplerPayload::euler()?;
        let initial = tensor_from_f32(&backend, &[2], &[1.0, -1.0], &context)?;
        let cancelled_initial = initial.clone();
        let mut denoiser = |input: &Tensor, _sigma: f32, _step: usize| Ok(input.clone());
        let trace =
            sampler.execute(&backend, initial, &[2.0, 1.0, 0.0], &context, &mut denoiser)?;
        assert_eq!(trace.sigmas, vec![2.0, 1.0, 0.0]);
        assert!(trace.latents.len() >= 2);
        assert!(sampler.resident_bytes() > 0);

        cancellation.cancel();
        assert!(matches!(
            sampler.execute(
                &backend,
                cancelled_initial,
                &[1.0, 0.0],
                &context,
                &mut denoiser,
            ),
            Err(NativeSamplerPayloadError::Tensor(TensorError::Cancelled))
        ));
        Ok(())
    }
}
