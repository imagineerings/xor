use crate::validate_identifier;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::{Arc, OnceLock};
use thiserror::Error;

const SD15_TIMESTEPS: usize = 1_000;
const SD15_LINEAR_START: f64 = 0.00085;
const SD15_LINEAR_END: f64 = 0.012;
pub const SD15_SAMPLING_PROFILE_ID: &str = "sd15-discrete-epsilon-v1";

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
    fn sigma_min(&self) -> f32;
    fn sigma_max(&self) -> f32;
    fn half_log_snr(&self, sigma: f32) -> Result<f32, SamplingProfileError>;
    fn sigma_from_half_log_snr(&self, half_log_snr: f32) -> Result<f32, SamplingProfileError>;
    fn adjust_first_sigma_for_snr(&self, sigmas: &mut [f32]) -> Result<(), SamplingProfileError>;
    fn scale_sampler_noise(&self, sampler_noise_scale: f32) -> Result<f32, SamplingProfileError>;
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
        validate_sigma_grid(&sigmas)?;
        validate_sampling_parameters(snr_mode, noise_scale)?;
        Ok(Self {
            identity,
            prediction,
            sigmas,
            snr_mode,
            noise_scale,
        })
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
        validate_sigma(sigma, false)?;
        Ok(sigma)
    }

    fn model_time_for_sigma(&self, sigma: f32) -> Result<f32, SamplingProfileError> {
        if sigma == 0.0 {
            return Ok(0.0);
        }
        validate_sigma(sigma, false)?;
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
