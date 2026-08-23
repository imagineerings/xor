use crate::validate_identifier;
use comfy_model::{NativeModelPayload, NativeModelResourceRole};
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::{Arc, OnceLock};
use thiserror::Error;

const SD15_TIMESTEPS: usize = 1_000;
const SD15_LINEAR_START: f64 = 0.00085;
const SD15_LINEAR_END: f64 = 0.012;
pub const SD15_SAMPLING_PROFILE_ID: &str = "sd15-discrete-epsilon-v1";
pub const LOTUS_SDPOSE_SAMPLING_PROFILE_ID: &str = "lotus-sdpose-discrete-denoised-v1";
pub const AURAFLOW_SAMPLING_PROFILE_ID: &str = "auraflow-discrete-flow-shift-1.73-v1";
pub const QWEN_IMAGE_SAMPLING_PROFILE_ID: &str = "qwen-image-flux-shift-1.15-v1";
const AURAFLOW_TIMESTEPS: usize = 1_000;
const AURAFLOW_SHIFT: f32 = 1.73;
const AURAFLOW_MULTIPLIER: f32 = 1.0;
const QWEN_IMAGE_TIMESTEPS: usize = 10_000;
const QWEN_IMAGE_SHIFT: f32 = 1.15;

pub fn exponential_integrator_phi_one(value: f32) -> f32 {
    value.exp_m1()
}

pub fn exponential_integrator_phi_two(value: f32) -> f32 {
    (value.exp_m1() - value) / value
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SamplingProfileIdentity(String);

impl SamplingProfileIdentity {
    pub fn new(value: impl Into<String>) -> Result<Self, SamplingProfileError> {
        let value = value.into();
        validate_identifier("sampling profile", &value)
            .map_err(|_| SamplingProfileError::InvalidIdentity(value.clone()))?;
        Ok(Self(value))
    }

    pub fn sd15() -> Self {
        Self(SD15_SAMPLING_PROFILE_ID.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SamplingProfileIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictionInterpretation {
    Epsilon,
    VPrediction,
    Flow,
    Denoised,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SamplingSnrMode {
    Standard,
    ConstantFlow { shift: f32 },
}

pub trait SamplingProfile {
    fn identity(&self) -> &SamplingProfileIdentity;
    fn prediction(&self) -> PredictionInterpretation;
    fn sigma_count(&self) -> usize;
    fn sigma_at_index(&self, index: usize) -> Result<f32, SamplingProfileError> {
        if index >= self.sigma_count() {
            return Err(SamplingProfileError::GridIndex(index));
        }
        self.sigma_at_model_time(index as f32)
    }
    fn sigma_at_model_time(&self, model_time: f32) -> Result<f32, SamplingProfileError>;
    fn model_time_for_sigma(&self, sigma: f32) -> Result<f32, SamplingProfileError>;
    fn sigma_for_model_time(&self, model_time: f32) -> Result<f32, SamplingProfileError> {
        self.sigma_at_model_time(model_time)
    }
    fn sampling_percent_for_sigma(&self, sigma: f32) -> Result<f32, SamplingProfileError> {
        let maximum = self.sigma_count().saturating_sub(1) as f32;
        if maximum == 0.0 {
            return Err(SamplingProfileError::EmptyGrid);
        }
        Ok((1.0 - self.model_time_for_sigma(sigma)? / maximum).clamp(0.0, 1.0))
    }
    fn sigma_min(&self) -> f32;
    fn sigma_max(&self) -> f32;
    fn half_log_snr(&self, sigma: f32) -> Result<f32, SamplingProfileError>;
    fn sigma_from_half_log_snr(&self, half_log_snr: f32) -> Result<f32, SamplingProfileError>;
    fn adjust_first_sigma_for_snr(&self, sigmas: &mut [f32]) -> Result<(), SamplingProfileError>;
    fn scale_sampler_noise(&self, sampler_noise_scale: f32) -> Result<f32, SamplingProfileError>;
    fn is_max_denoise(&self, sigma: f32) -> Result<bool, SamplingProfileError> {
        validate_sigma(sigma, true)?;
        let maximum = self.sigma_max();
        let tolerance = 1.0e-5 * maximum.abs().max(sigma.abs());
        Ok(sigma > maximum || (sigma - maximum).abs() <= tolerance)
    }
    fn scale_initial_noise_in_place(
        &self,
        noise: &mut [f32],
        latent: &[f32],
        sigma: f32,
        max_denoise: bool,
    ) -> Result<(), SamplingProfileError> {
        validate_equal_lengths(noise, latent, "initial noise and latent")?;
        validate_sigma(sigma, false)?;
        if self.prediction() == PredictionInterpretation::Flow {
            let noise_scale = self.scale_sampler_noise(1.0)?;
            for (index, (noise_value, latent_value)) in
                noise.iter_mut().zip(latent.iter()).enumerate()
            {
                let value =
                    (sigma * noise_scale).mul_add(*noise_value, (1.0 - sigma) * *latent_value);
                if !value.is_finite() {
                    return Err(SamplingProfileError::NonFiniteOutput {
                        operation: "initial flow-noise scaling",
                        index,
                    });
                }
                *noise_value = value;
            }
            return Ok(());
        }
        let scale = if max_denoise {
            (1.0 + sigma * sigma).sqrt()
        } else {
            sigma
        };
        for (index, (noise_value, latent_value)) in noise.iter_mut().zip(latent.iter()).enumerate()
        {
            let value = noise_value.mul_add(scale, *latent_value);
            if !value.is_finite() {
                return Err(SamplingProfileError::NonFiniteOutput {
                    operation: "initial noise scaling",
                    index,
                });
            }
            *noise_value = value;
        }
        Ok(())
    }
    fn scale_model_input_in_place(
        &self,
        values: &mut [f32],
        sigma: f32,
    ) -> Result<(), SamplingProfileError> {
        validate_sigma(sigma, false)?;
        if self.prediction() == PredictionInterpretation::Flow {
            return Ok(());
        }
        let divisor = (sigma * sigma + 1.0).sqrt();
        for (index, value) in values.iter_mut().enumerate() {
            *value /= divisor;
            if !value.is_finite() {
                return Err(SamplingProfileError::NonFiniteOutput {
                    operation: "model input scaling",
                    index,
                });
            }
        }
        Ok(())
    }
    fn interpret_prediction_in_place(
        &self,
        model_output: &mut [f32],
        model_input: &[f32],
        sigma: f32,
    ) -> Result<(), SamplingProfileError> {
        validate_equal_lengths(model_output, model_input, "prediction and model input")?;
        validate_sigma(sigma, false)?;
        for (index, (output, input)) in model_output.iter_mut().zip(model_input.iter()).enumerate()
        {
            let denoised = match self.prediction() {
                PredictionInterpretation::Epsilon | PredictionInterpretation::Flow => {
                    output.mul_add(-sigma, *input)
                }
                PredictionInterpretation::VPrediction => {
                    let denominator = (sigma * sigma + 1.0).sqrt();
                    *input / (sigma * sigma + 1.0) - *output * sigma / denominator
                }
                PredictionInterpretation::Denoised => *output,
            };
            if !denoised.is_finite() {
                return Err(SamplingProfileError::NonFiniteOutput {
                    operation: "prediction interpretation",
                    index,
                });
            }
            *output = denoised;
        }
        Ok(())
    }
}

pub fn standard_ancestral_step(
    sigma_from: f32,
    sigma_to: f32,
    eta: f32,
) -> Result<(f32, f32), SamplingProfileError> {
    if eta == 0.0 {
        return Ok((sigma_to, 0.0));
    }
    if !sigma_from.is_finite()
        || sigma_from <= 0.0
        || !sigma_to.is_finite()
        || sigma_to < 0.0
        || sigma_to > sigma_from
        || !eta.is_finite()
    {
        return Err(SamplingProfileError::InvalidAncestralStep {
            sigma_from,
            sigma_to,
            eta,
        });
    }
    let sigma_from_squared = sigma_from * sigma_from;
    let sigma_to_squared = sigma_to * sigma_to;
    let radicand = sigma_to_squared * (sigma_from_squared - sigma_to_squared) / sigma_from_squared;
    let sigma_up = sigma_to.min(eta * radicand.sqrt());
    let sigma_down = (sigma_to_squared - sigma_up * sigma_up).sqrt();
    if !sigma_down.is_finite() || sigma_down < 0.0 || !sigma_up.is_finite() {
        return Err(SamplingProfileError::InvalidAncestralStep {
            sigma_from,
            sigma_to,
            eta,
        });
    }
    Ok((sigma_down, sigma_up))
}

pub fn rectified_flow_ancestral_step(
    sigma_from: f32,
    sigma_to: f32,
    eta: f32,
) -> Result<(f32, f32), SamplingProfileError> {
    if !sigma_from.is_finite()
        || sigma_from <= 0.0
        || !sigma_to.is_finite()
        || sigma_to < 0.0
        || sigma_to > sigma_from
        || !eta.is_finite()
    {
        return Err(SamplingProfileError::InvalidAncestralStep {
            sigma_from,
            sigma_to,
            eta,
        });
    }

    let downstep_ratio = 1.0 + (sigma_to / sigma_from - 1.0) * eta;
    let sigma_down = sigma_to * downstep_ratio;
    let alpha_to = 1.0 - sigma_to;
    let alpha_down = 1.0 - sigma_down;
    if alpha_down == 0.0 {
        return Err(SamplingProfileError::InvalidAncestralStep {
            sigma_from,
            sigma_to,
            eta,
        });
    }
    let sigma_to_squared = sigma_to * sigma_to;
    let sigma_down_squared = sigma_down * sigma_down;
    let alpha_to_squared = alpha_to * alpha_to;
    let alpha_down_squared = alpha_down * alpha_down;
    let radicand = sigma_to_squared - sigma_down_squared * alpha_to_squared / alpha_down_squared;
    let renoise_coefficient = radicand.sqrt();
    if !sigma_down.is_finite()
        || sigma_down < 0.0
        || !renoise_coefficient.is_finite()
        || renoise_coefficient < 0.0
    {
        return Err(SamplingProfileError::InvalidAncestralStep {
            sigma_from,
            sigma_to,
            eta,
        });
    }
    Ok((sigma_down, renoise_coefficient))
}

#[derive(Clone, Debug)]
pub struct DiscreteSamplingProfile {
    identity: SamplingProfileIdentity,
    prediction: PredictionInterpretation,
    sigmas: Arc<[f32]>,
    snr_mode: SamplingSnrMode,
    noise_scale: f32,
    time_mapping: ModelTimeMapping,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ModelTimeMapping {
    DiscreteGrid,
    DiscreteFlow { shift: f32, multiplier: f32 },
    Flux { shift: f32 },
}

impl DiscreteSamplingProfile {
    pub fn new(
        identity: SamplingProfileIdentity,
        prediction: PredictionInterpretation,
        sigmas: Arc<[f32]>,
    ) -> Result<Self, SamplingProfileError> {
        let snr_mode = if prediction == PredictionInterpretation::Flow {
            SamplingSnrMode::ConstantFlow { shift: 1.0 }
        } else {
            SamplingSnrMode::Standard
        };
        Self::new_with_sampling_parameters(identity, prediction, sigmas, snr_mode, 1.0)
    }

    pub fn new_with_sampling_parameters(
        identity: SamplingProfileIdentity,
        prediction: PredictionInterpretation,
        sigmas: Arc<[f32]>,
        snr_mode: SamplingSnrMode,
        noise_scale: f32,
    ) -> Result<Self, SamplingProfileError> {
        Self::new_with_time_mapping(
            identity,
            prediction,
            sigmas,
            snr_mode,
            noise_scale,
            ModelTimeMapping::DiscreteGrid,
        )
    }

    pub fn sd15() -> Result<Self, SamplingProfileError> {
        static SIGMAS: OnceLock<Result<Arc<[f32]>, SamplingProfileError>> = OnceLock::new();
        let sigmas = SIGMAS
            .get_or_init(|| build_sd15_sigmas().map(Arc::from))
            .clone()?;
        Self::new(
            SamplingProfileIdentity::sd15(),
            PredictionInterpretation::Epsilon,
            sigmas,
        )
    }

    pub fn lotus_sdpose() -> Result<Self, SamplingProfileError> {
        let sigmas = Self::sd15()?.sigmas;
        Self::new(
            SamplingProfileIdentity::new(LOTUS_SDPOSE_SAMPLING_PROFILE_ID)?,
            PredictionInterpretation::Denoised,
            sigmas,
        )
    }

    pub fn auraflow() -> Result<Self, SamplingProfileError> {
        static SIGMAS: OnceLock<Result<Arc<[f32]>, SamplingProfileError>> = OnceLock::new();
        let sigmas = SIGMAS
            .get_or_init(|| {
                build_flow_sigmas(AURAFLOW_TIMESTEPS, |time| {
                    time_snr_shift(AURAFLOW_SHIFT, time)
                })
                .map(Arc::from)
            })
            .clone()?;
        Self::new_with_time_mapping(
            SamplingProfileIdentity::new(AURAFLOW_SAMPLING_PROFILE_ID)?,
            PredictionInterpretation::Flow,
            sigmas,
            SamplingSnrMode::ConstantFlow {
                shift: AURAFLOW_SHIFT,
            },
            1.0,
            ModelTimeMapping::DiscreteFlow {
                shift: AURAFLOW_SHIFT,
                multiplier: AURAFLOW_MULTIPLIER,
            },
        )
    }

    pub fn qwen_image() -> Result<Self, SamplingProfileError> {
        static SIGMAS: OnceLock<Result<Arc<[f32]>, SamplingProfileError>> = OnceLock::new();
        let sigmas = SIGMAS
            .get_or_init(|| {
                build_flow_sigmas(QWEN_IMAGE_TIMESTEPS, |time| {
                    flux_time_shift(QWEN_IMAGE_SHIFT, time)
                })
                .map(Arc::from)
            })
            .clone()?;
        Self::new_with_time_mapping(
            SamplingProfileIdentity::new(QWEN_IMAGE_SAMPLING_PROFILE_ID)?,
            PredictionInterpretation::Flow,
            sigmas,
            SamplingSnrMode::ConstantFlow {
                shift: QWEN_IMAGE_SHIFT.exp(),
            },
            1.0,
            ModelTimeMapping::Flux {
                shift: QWEN_IMAGE_SHIFT,
            },
        )
    }

    fn new_with_time_mapping(
        identity: SamplingProfileIdentity,
        prediction: PredictionInterpretation,
        sigmas: Arc<[f32]>,
        snr_mode: SamplingSnrMode,
        noise_scale: f32,
        time_mapping: ModelTimeMapping,
    ) -> Result<Self, SamplingProfileError> {
        validate_sigma_grid(&sigmas)?;
        validate_sampling_parameters(snr_mode, noise_scale)?;
        match time_mapping {
            ModelTimeMapping::DiscreteGrid => {}
            ModelTimeMapping::DiscreteFlow { shift, multiplier } => {
                if !shift.is_finite()
                    || shift <= 0.0
                    || !multiplier.is_finite()
                    || multiplier <= 0.0
                {
                    return Err(SamplingProfileError::InvalidModelTimeMapping);
                }
            }
            ModelTimeMapping::Flux { shift } if !shift.is_finite() => {
                return Err(SamplingProfileError::InvalidModelTimeMapping);
            }
            ModelTimeMapping::Flux { .. } => {}
        }
        Ok(Self {
            identity,
            prediction,
            sigmas,
            snr_mode,
            noise_scale,
            time_mapping,
        })
    }

    pub fn is_max_denoise(&self, sigma: f32) -> Result<bool, SamplingProfileError> {
        validate_sigma(sigma, true)?;
        let maximum = self.sigma_max();
        let tolerance = 1.0e-5 * maximum.abs().max(sigma.abs());
        Ok(sigma > maximum || (sigma - maximum).abs() <= tolerance)
    }

    pub fn scale_initial_noise_in_place(
        &self,
        noise: &mut [f32],
        latent: &[f32],
        sigma: f32,
        max_denoise: bool,
    ) -> Result<(), SamplingProfileError> {
        validate_equal_lengths(noise, latent, "initial noise and latent")?;
        validate_sigma(sigma, false)?;
        if self.prediction == PredictionInterpretation::Flow {
            for (index, (noise_value, latent_value)) in
                noise.iter_mut().zip(latent.iter()).enumerate()
            {
                let value =
                    (sigma * self.noise_scale).mul_add(*noise_value, (1.0 - sigma) * *latent_value);
                if !value.is_finite() {
                    return Err(SamplingProfileError::NonFiniteOutput {
                        operation: "initial flow-noise scaling",
                        index,
                    });
                }
                *noise_value = value;
            }
            return Ok(());
        }
        let scale = if max_denoise {
            (1.0 + sigma * sigma).sqrt()
        } else {
            sigma
        };
        for (index, (noise_value, latent_value)) in noise.iter_mut().zip(latent.iter()).enumerate()
        {
            let value = noise_value.mul_add(scale, *latent_value);
            if !value.is_finite() {
                return Err(SamplingProfileError::NonFiniteOutput {
                    operation: "initial noise scaling",
                    index,
                });
            }
            *noise_value = value;
        }
        Ok(())
    }

    pub fn scale_model_input_in_place(
        &self,
        values: &mut [f32],
        sigma: f32,
    ) -> Result<(), SamplingProfileError> {
        validate_sigma(sigma, false)?;
        if self.prediction == PredictionInterpretation::Flow {
            return Ok(());
        }
        let divisor = (sigma * sigma + 1.0).sqrt();
        for (index, value) in values.iter_mut().enumerate() {
            *value /= divisor;
            if !value.is_finite() {
                return Err(SamplingProfileError::NonFiniteOutput {
                    operation: "model input scaling",
                    index,
                });
            }
        }
        Ok(())
    }

    pub fn interpret_prediction_in_place(
        &self,
        model_output: &mut [f32],
        model_input: &[f32],
        sigma: f32,
    ) -> Result<(), SamplingProfileError> {
        validate_equal_lengths(model_output, model_input, "prediction and model input")?;
        validate_sigma(sigma, false)?;
        for (index, (output, input)) in model_output.iter_mut().zip(model_input.iter()).enumerate()
        {
            let denoised = match self.prediction {
                PredictionInterpretation::Epsilon => output.mul_add(-sigma, *input),
                PredictionInterpretation::VPrediction => {
                    let denominator = (sigma * sigma + 1.0).sqrt();
                    *input / (sigma * sigma + 1.0) - *output * sigma / denominator
                }
                PredictionInterpretation::Flow => output.mul_add(-sigma, *input),
                PredictionInterpretation::Denoised => *output,
            };
            if !denoised.is_finite() {
                return Err(SamplingProfileError::NonFiniteOutput {
                    operation: "prediction interpretation",
                    index,
                });
            }
            *output = denoised;
        }
        Ok(())
    }

    pub fn sigmas(&self) -> &[f32] {
        &self.sigmas
    }
}

pub fn profile_for_model(
    model: &NativeModelPayload,
) -> Result<DiscreteSamplingProfile, SamplingProfileError> {
    if model.identity().role() != NativeModelResourceRole::Model || model.model().is_none() {
        return Err(SamplingProfileError::UnsupportedModel(
            model.identity().identifier().to_owned(),
        ));
    }
    let Some(resource) = model.native_family_model_resource() else {
        return DiscreteSamplingProfile::sd15();
    };
    let family = resource
        .family_identity()
        .map_err(|error| SamplingProfileError::UnsupportedModel(error.to_string()))?;
    match family.feature_id() {
        comfy_model::generated_auraflow_comfy_model_0064::MODEL_FAMILY_FEATURE_ID => {
            DiscreteSamplingProfile::auraflow()
        }
        comfy_model::generated_qwenimage_comfy_model_0113::MODEL_FAMILY_FEATURE_ID => {
            DiscreteSamplingProfile::qwen_image()
        }
        _ => Err(SamplingProfileError::UnsupportedModel(
            family.feature_id().to_owned(),
        )),
    }
}

impl SamplingProfile for DiscreteSamplingProfile {
    fn identity(&self) -> &SamplingProfileIdentity {
        &self.identity
    }

    fn prediction(&self) -> PredictionInterpretation {
        self.prediction
    }

    fn sigma_count(&self) -> usize {
        self.sigmas.len()
    }

    fn sigma_at_index(&self, index: usize) -> Result<f32, SamplingProfileError> {
        self.sigmas
            .get(index)
            .copied()
            .ok_or(SamplingProfileError::GridIndex(index))
    }

    fn sigma_at_model_time(&self, model_time: f32) -> Result<f32, SamplingProfileError> {
        if !model_time.is_finite() {
            return Err(SamplingProfileError::InvalidModelTime(model_time));
        }
        let maximum = (self.sigmas.len() - 1) as f32;
        let model_time = model_time.clamp(0.0, maximum);
        let low = model_time.floor() as usize;
        let high = model_time.ceil() as usize;
        let weight = model_time.fract();
        let low_sigma = *self
            .sigmas
            .get(low)
            .ok_or(SamplingProfileError::GridIndex(low))?;
        let high_sigma = *self
            .sigmas
            .get(high)
            .ok_or(SamplingProfileError::GridIndex(high))?;
        let log_sigma = (high_sigma.ln() - low_sigma.ln()).mul_add(weight, low_sigma.ln());
        let sigma = log_sigma.exp();
        validate_sigma(sigma, true)?;
        Ok(sigma)
    }

    fn model_time_for_sigma(&self, sigma: f32) -> Result<f32, SamplingProfileError> {
        if sigma == 0.0 {
            return Ok(0.0);
        }
        validate_sigma(sigma, false)?;
        match self.time_mapping {
            ModelTimeMapping::DiscreteGrid => {
                let log_sigma = sigma.ln();
                let (_, index) = self
                    .sigmas
                    .iter()
                    .enumerate()
                    .map(|(index, candidate)| ((candidate.ln() - log_sigma).abs(), index))
                    .min_by(|left, right| left.0.total_cmp(&right.0))
                    .ok_or(SamplingProfileError::EmptyGrid)?;
                Ok(index as f32)
            }
            ModelTimeMapping::DiscreteFlow { multiplier, .. } => Ok(sigma * multiplier),
            ModelTimeMapping::Flux { .. } => Ok(sigma),
        }
    }

    fn sigma_for_model_time(&self, model_time: f32) -> Result<f32, SamplingProfileError> {
        if !model_time.is_finite() || model_time < 0.0 {
            return Err(SamplingProfileError::InvalidModelTime(model_time));
        }
        let sigma = match self.time_mapping {
            ModelTimeMapping::DiscreteGrid => return self.sigma_at_model_time(model_time),
            ModelTimeMapping::DiscreteFlow { shift, multiplier } => {
                time_snr_shift(shift, model_time / multiplier)
            }
            ModelTimeMapping::Flux { shift } => flux_time_shift(shift, model_time),
        };
        validate_sigma(sigma, true)?;
        Ok(sigma)
    }

    fn sampling_percent_for_sigma(&self, sigma: f32) -> Result<f32, SamplingProfileError> {
        if sigma == 0.0 {
            return Ok(1.0);
        }
        validate_sigma(sigma, false)?;
        match self.time_mapping {
            ModelTimeMapping::DiscreteGrid => {
                let maximum = (self.sigmas.len() - 1) as f32;
                Ok((1.0 - self.model_time_for_sigma(sigma)? / maximum).clamp(0.0, 1.0))
            }
            ModelTimeMapping::DiscreteFlow { shift, .. } => shifted_sigma_to_percent(shift, sigma),
            ModelTimeMapping::Flux { shift } => shifted_sigma_to_percent(shift.exp(), sigma),
        }
    }

    fn sigma_min(&self) -> f32 {
        self.sigmas[0]
    }

    fn sigma_max(&self) -> f32 {
        self.sigmas[self.sigmas.len() - 1]
    }

    fn half_log_snr(&self, sigma: f32) -> Result<f32, SamplingProfileError> {
        validate_sigma(sigma, false)?;
        let half_log_snr = match self.snr_mode {
            SamplingSnrMode::Standard => -sigma.ln(),
            SamplingSnrMode::ConstantFlow { .. } => {
                if sigma >= 1.0 {
                    return Err(SamplingProfileError::InvalidSnrSigma(sigma));
                }
                (1.0 - sigma).ln() - sigma.ln()
            }
        };
        if !half_log_snr.is_finite() {
            return Err(SamplingProfileError::InvalidHalfLogSnr(half_log_snr));
        }
        Ok(half_log_snr)
    }

    fn sigma_from_half_log_snr(&self, half_log_snr: f32) -> Result<f32, SamplingProfileError> {
        if !half_log_snr.is_finite() {
            return Err(SamplingProfileError::InvalidHalfLogSnr(half_log_snr));
        }
        let sigma = match self.snr_mode {
            SamplingSnrMode::Standard => (-half_log_snr).exp(),
            SamplingSnrMode::ConstantFlow { .. } if half_log_snr >= 0.0 => {
                let exponential = (-half_log_snr).exp();
                exponential / (1.0 + exponential)
            }
            SamplingSnrMode::ConstantFlow { .. } => 1.0 / (1.0 + half_log_snr.exp()),
        };
        if !sigma.is_finite() || sigma <= 0.0 {
            return Err(SamplingProfileError::InvalidSnrSigma(sigma));
        }
        Ok(sigma)
    }

    fn adjust_first_sigma_for_snr(&self, sigmas: &mut [f32]) -> Result<(), SamplingProfileError> {
        for sigma in sigmas.iter().copied() {
            validate_sigma(sigma, true)?;
        }
        if sigmas.len() <= 1 {
            return Ok(());
        }
        if let SamplingSnrMode::ConstantFlow { shift } = self.snr_mode {
            if sigmas[0] >= 1.0 {
                sigmas[0] = constant_flow_percent_to_sigma(shift, 1.0e-4)?;
            }
        }
        Ok(())
    }

    fn scale_sampler_noise(&self, sampler_noise_scale: f32) -> Result<f32, SamplingProfileError> {
        if !sampler_noise_scale.is_finite() {
            return Err(SamplingProfileError::InvalidNoiseScale(sampler_noise_scale));
        }
        let scaled = sampler_noise_scale * self.noise_scale;
        if !scaled.is_finite() {
            return Err(SamplingProfileError::InvalidNoiseScale(scaled));
        }
        Ok(scaled)
    }
}

fn validate_sampling_parameters(
    snr_mode: SamplingSnrMode,
    noise_scale: f32,
) -> Result<(), SamplingProfileError> {
    if let SamplingSnrMode::ConstantFlow { shift } = snr_mode {
        if !shift.is_finite() || shift <= 0.0 {
            return Err(SamplingProfileError::InvalidSnrShift(shift));
        }
    }
    if !noise_scale.is_finite() || noise_scale < 0.0 {
        return Err(SamplingProfileError::InvalidNoiseScale(noise_scale));
    }
    Ok(())
}

fn constant_flow_percent_to_sigma(shift: f32, percent: f32) -> Result<f32, SamplingProfileError> {
    if !percent.is_finite() || !(0.0..=1.0).contains(&percent) {
        return Err(SamplingProfileError::InvalidSamplingPercent(percent));
    }
    if percent <= 0.0 {
        return Ok(1.0);
    }
    if percent >= 1.0 {
        return Ok(0.0);
    }
    let time = 1.0 - percent;
    let sigma = shift * time / (1.0 + (shift - 1.0) * time);
    if !sigma.is_finite() || !(0.0..1.0).contains(&sigma) {
        return Err(SamplingProfileError::InvalidSnrSigma(sigma));
    }
    Ok(sigma)
}

fn shifted_sigma_to_percent(shift: f32, sigma: f32) -> Result<f32, SamplingProfileError> {
    if sigma >= 1.0 {
        return Ok(0.0);
    }
    let denominator = shift - sigma * (shift - 1.0);
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(SamplingProfileError::InvalidSnrSigma(sigma));
    }
    let percent = 1.0 - sigma / denominator;
    if !percent.is_finite() || !(0.0..=1.0).contains(&percent) {
        return Err(SamplingProfileError::InvalidSamplingPercent(percent));
    }
    Ok(percent)
}

fn time_snr_shift(shift: f32, time: f32) -> f32 {
    shift * time / (1.0 + (shift - 1.0) * time)
}

fn flux_time_shift(shift: f32, time: f32) -> f32 {
    let exponential = shift.exp();
    exponential / (exponential + (time.recip() - 1.0))
}

fn build_flow_sigmas(
    timesteps: usize,
    sigma: impl Fn(f32) -> f32,
) -> Result<Vec<f32>, SamplingProfileError> {
    let mut sigmas = Vec::new();
    sigmas
        .try_reserve_exact(timesteps)
        .map_err(|_| SamplingProfileError::OutOfMemory("flow sigma grid"))?;
    for index in 1..=timesteps {
        sigmas.push(sigma(index as f32 / timesteps as f32));
    }
    validate_sigma_grid(&sigmas)?;
    Ok(sigmas)
}

fn build_sd15_sigmas() -> Result<Vec<f32>, SamplingProfileError> {
    let start = SD15_LINEAR_START.sqrt();
    let end = SD15_LINEAR_END.sqrt();
    let mut cumulative_alpha = 1.0_f64;
    let mut sigmas = Vec::new();
    sigmas
        .try_reserve_exact(SD15_TIMESTEPS)
        .map_err(|_| SamplingProfileError::OutOfMemory("SD15 sigma grid"))?;
    for index in 0..SD15_TIMESTEPS {
        let fraction = index as f64 / (SD15_TIMESTEPS - 1) as f64;
        let root_beta = (end - start).mul_add(fraction, start);
        cumulative_alpha *= 1.0 - root_beta * root_beta;
        sigmas.push((((1.0 - cumulative_alpha) / cumulative_alpha).sqrt()) as f32);
    }
    Ok(sigmas)
}

fn validate_sigma_grid(sigmas: &[f32]) -> Result<(), SamplingProfileError> {
    if sigmas.len() < 2 {
        return Err(SamplingProfileError::EmptyGrid);
    }
    for (index, sigma) in sigmas.iter().copied().enumerate() {
        validate_sigma(sigma, false)?;
        if index > 0 && sigma <= sigmas[index - 1] {
            return Err(SamplingProfileError::UnorderedGrid { index });
        }
    }
    Ok(())
}

fn validate_sigma(sigma: f32, allow_zero: bool) -> Result<(), SamplingProfileError> {
    if !sigma.is_finite() || sigma < 0.0 || (!allow_zero && sigma == 0.0) {
        return Err(SamplingProfileError::InvalidSigma(sigma));
    }
    Ok(())
}

fn validate_equal_lengths(
    left: &[f32],
    right: &[f32],
    operation: &'static str,
) -> Result<(), SamplingProfileError> {
    if left.len() != right.len() {
        return Err(SamplingProfileError::LengthMismatch {
            operation,
            left: left.len(),
            right: right.len(),
        });
    }
    Ok(())
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum SamplingProfileError {
    #[error("invalid sampling-profile identity {0:?}")]
    InvalidIdentity(String),
    #[error("sampling profile sigma grid must contain at least two entries")]
    EmptyGrid,
    #[error("sampling profile sigma grid is not strictly increasing at index {index}")]
    UnorderedGrid { index: usize },
    #[error("sampling profile sigma {0} is invalid")]
    InvalidSigma(f32),
    #[error(
        "ancestral step is invalid for sigma_from {sigma_from}, sigma_to {sigma_to}, eta {eta}"
    )]
    InvalidAncestralStep {
        sigma_from: f32,
        sigma_to: f32,
        eta: f32,
    },
    #[error("sampling profile SNR sigma {0} is invalid")]
    InvalidSnrSigma(f32),
    #[error("sampling profile half-log-SNR {0} is invalid")]
    InvalidHalfLogSnr(f32),
    #[error("sampling profile SNR shift {0} is invalid")]
    InvalidSnrShift(f32),
    #[error("sampling profile noise scale {0} is invalid")]
    InvalidNoiseScale(f32),
    #[error("sampling profile percentage {0} is invalid")]
    InvalidSamplingPercent(f32),
    #[error("sampling profile model time {0} is invalid")]
    InvalidModelTime(f32),
    #[error("sampling profile model-time mapping is invalid")]
    InvalidModelTimeMapping,
    #[error("sampling profile is unavailable for MODEL {0}")]
    UnsupportedModel(String),
    #[error("sampling profile grid index {0} is unavailable")]
    GridIndex(usize),
    #[error("sampling profile allocation failed for {0}")]
    OutOfMemory(&'static str),
    #[error("{operation} length mismatch: {left} versus {right}")]
    LengthMismatch {
        operation: &'static str,
        left: usize,
        right: usize,
    },
    #[error("{operation} produced a non-finite value at index {index}")]
    NonFiniteOutput {
        operation: &'static str,
        index: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 1.0e-6,
            "{actual} differs from {expected}",
        );
    }

    #[test]
    fn family_model_profiles_match_source_flow_equations_and_sd15_compatibility()
    -> Result<(), SamplingProfileError> {
        let sd15 = DiscreteSamplingProfile::sd15()?;
        assert_close(sd15.sampling_percent_for_sigma(sd15.sigma_max())?, 0.0);
        assert_close(sd15.sampling_percent_for_sigma(sd15.sigma_min())?, 1.0);

        let aura = DiscreteSamplingProfile::auraflow()?;
        assert_eq!(aura.identity().as_str(), AURAFLOW_SAMPLING_PROFILE_ID);
        assert_eq!(aura.sigma_count(), AURAFLOW_TIMESTEPS);
        assert_eq!(aura.prediction(), PredictionInterpretation::Flow);
        assert_close(aura.sigma_at_index(AURAFLOW_TIMESTEPS - 1)?, 1.0);
        assert_close(aura.model_time_for_sigma(0.5)?, 0.5);
        assert_close(
            aura.sigma_for_model_time(0.5)?,
            time_snr_shift(AURAFLOW_SHIFT, 0.5),
        );
        assert_close(aura.sigma_for_model_time(0.0)?, 0.0);
        assert_close(aura.sampling_percent_for_sigma(1.0)?, 0.0);
        assert_close(aura.sampling_percent_for_sigma(0.0)?, 1.0);
        assert_close(
            aura.sampling_percent_for_sigma(time_snr_shift(AURAFLOW_SHIFT, 0.25))?,
            0.75,
        );

        let qwen = DiscreteSamplingProfile::qwen_image()?;
        assert_eq!(qwen.identity().as_str(), QWEN_IMAGE_SAMPLING_PROFILE_ID);
        assert_eq!(qwen.sigma_count(), QWEN_IMAGE_TIMESTEPS);
        assert_eq!(qwen.prediction(), PredictionInterpretation::Flow);
        assert_close(qwen.sigma_at_index(QWEN_IMAGE_TIMESTEPS - 1)?, 1.0);
        assert_close(qwen.model_time_for_sigma(0.5)?, 0.5);
        assert_close(
            qwen.sigma_for_model_time(0.5)?,
            flux_time_shift(QWEN_IMAGE_SHIFT, 0.5),
        );
        assert_close(qwen.sigma_for_model_time(0.0)?, 0.0);
        assert_close(qwen.sampling_percent_for_sigma(1.0)?, 0.0);
        assert_close(qwen.sampling_percent_for_sigma(0.0)?, 1.0);
        assert_close(
            qwen.sampling_percent_for_sigma(flux_time_shift(QWEN_IMAGE_SHIFT, 0.25))?,
            0.75,
        );

        let mut noise = [2.0, -1.0];
        let latent = [0.25, 0.5];
        aura.scale_initial_noise_in_place(&mut noise, &latent, 0.75, true)?;
        assert_eq!(noise, [1.5625, -0.625]);
        let mut input = [3.0, -2.0];
        qwen.scale_model_input_in_place(&mut input, 0.5)?;
        assert_eq!(input, [3.0, -2.0]);
        let mut output = [0.5, -1.0];
        aura.interpret_prediction_in_place(&mut output, &[2.0, 3.0], 0.25)?;
        assert_eq!(output, [1.875, 3.25]);
        Ok(())
    }
}
