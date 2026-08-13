use crate::{
    MappedModelWeights, NativeModule,
    attention::{
        AttentionBackend, AttentionError, AttentionFallbackPolicy, AttentionRequest,
        scaled_dot_product_attention_tensor_with_context,
    },
    generated_lotusd_comfy_model_0106,
    native_ops::{
        GeluApproximation, NativeOpsError, tensor_from_f32 as tensor_from_values,
        tensor_to_f32 as tensor_to_values,
    },
};
use comfy_media::{NativePoseKeypoint, NativePosePerson};
use comfy_tensor::{
    CpuBackend, DType, DecodedScalar, DeviceId, ExecutionContext, StorageId, StreamId, Tensor,
    TensorError,
    generated_comfy_operator_indirection_01::{ConvolutionGeometry, OperatorIndirectionError},
    generated_native_diffusion::NativeDiffusionTensorError,
    generated_shape_layout_transform_02::{
        ShapeLayoutTransformPartTwoError, tensor_reshape_with_context_exact_native,
    },
};
use comfy_types::{CancellationError, CancellationToken};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use thiserror::Error;

pub const SDPOSE_HEATMAP_CHANNELS: usize = 133;
pub const SDPOSE_HEATMAP_HEIGHT: usize = 256;
pub const SDPOSE_HEATMAP_WIDTH: usize = 192;
pub const SDPOSE_INPUT_HEIGHT: f32 = 1024.0;
pub const SDPOSE_INPUT_WIDTH: f32 = 768.0;

const GAUSSIAN_RADIUS: isize = 5;
const GAUSSIAN_SIGMA: f32 = 2.0;
const OPENPOSE_KEYPOINTS: usize = 134;
const MMPOSE_INDICES: [usize; 15] = [17, 6, 8, 10, 7, 9, 12, 14, 16, 13, 15, 2, 1, 4, 3];
const OPENPOSE_INDICES: [usize; 15] = [1, 2, 3, 4, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17];

const SDPOSE_SD2_SOURCE_DOMAIN: &[u8] = b"sim.comfy.sdpose-sd2-capture.v1\0";
const SDPOSE_MODEL_SOURCE_DOMAIN: &[u8] = b"sim.comfy.sdpose-model-resource.v1\0";
const SDPOSE_HEATMAP_HEAD_SOURCE_DOMAIN: &[u8] = b"sim.comfy.sdpose-heatmap-head.v1\0";
pub const SDPOSE_HEAD_SOURCE_SHA256: &str =
    "19a55d1ecf16796226ed204241852b9b237a563addf636ff738167d9273cf97a";
pub const SDPOSE_MODEL_DETECTION_SOURCE_SHA256: &str =
    "f13b11988fccf9fa4d878ef5f63313c23c5f1400ec8cde04a502584e157c5072";
const OPENAI_MODEL_SOURCE_SHA256: &str =
    "9d27fb036cab8a262ef3d866a643f7fdc40994022616f1b8be14b7d919f57f96";
const ATTENTION_SOURCE_SHA256: &str =
    "436e1d91f8d5d84c5667e051cdf3ab2f91d8db25b66d88a084c89a202de0579e";
const MODEL_BASE_SOURCE_SHA256: &str =
    "99dc53baee665eca1a6aea70cfb9ab071d55784dff339b5e919dc14ae4fde8bd";
const SUPPORTED_MODELS_SOURCE_SHA256: &str =
    "3801a60d15fe0abf8573cfa60f90e796d773450370f80784f2e0603cda3ffd69";
const SD2_CHANNEL_MULTIPLIERS: [usize; 4] = [1, 2, 4, 4];
const SD2_RESIDUAL_BLOCKS: [usize; 4] = [2, 2, 2, 2];
const SD2_INPUT_TRANSFORMER_DEPTHS: [usize; 8] = [1, 1, 1, 1, 1, 1, 0, 0];
const SD2_OUTPUT_TRANSFORMER_DEPTHS: [usize; 12] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SdPoseSd2Configuration {
    source_exact_profile: bool,
    model_channels: usize,
    context_dimension: usize,
    attention_head_channels: usize,
    normalization_groups: usize,
    latent_height: usize,
    latent_width: usize,
}

impl SdPoseSd2Configuration {
    pub const fn source() -> Self {
        Self {
            source_exact_profile: true,
            model_channels: 320,
            context_dimension: 1_024,
            attention_head_channels: 64,
            normalization_groups: 32,
            latent_height: 128,
            latent_width: 96,
        }
    }

    pub fn reduced_fixture(
        model_channels: usize,
        context_dimension: usize,
        attention_head_channels: usize,
        normalization_groups: usize,
        latent_height: usize,
        latent_width: usize,
    ) -> Result<Self, SdPoseSd2Error> {
        let configuration = Self {
            source_exact_profile: false,
            model_channels,
            context_dimension,
            attention_head_channels,
            normalization_groups,
            latent_height,
            latent_width,
        };
        configuration.validate()?;
        Ok(configuration)
    }

    pub const fn is_source_exact(&self) -> bool {
        self.source_exact_profile
    }

    pub const fn model_channels(&self) -> usize {
        self.model_channels
    }

    pub const fn context_dimension(&self) -> usize {
        self.context_dimension
    }

    pub const fn latent_height(&self) -> usize {
        self.latent_height
    }

    pub const fn latent_width(&self) -> usize {
        self.latent_width
    }

    pub const fn capture_channels(&self) -> usize {
        self.model_channels * 2
    }

    fn validate(&self) -> Result<(), SdPoseSd2Error> {
        if self.model_channels == 0
            || self.context_dimension == 0
            || self.attention_head_channels == 0
            || self.normalization_groups == 0
            || self.latent_height == 0
            || self.latent_width == 0
            || !self.latent_height.is_multiple_of(8)
            || !self.latent_width.is_multiple_of(8)
        {
            return Err(SdPoseSd2Error::InvalidConfiguration);
        }
        for multiplier in SD2_CHANNEL_MULTIPLIERS {
            let channels = self
                .model_channels
                .checked_mul(multiplier)
                .ok_or(SdPoseSd2Error::Overflow("SD2 channels"))?;
            if !channels.is_multiple_of(self.attention_head_channels)
                || !channels.is_multiple_of(self.normalization_groups)
            {
                return Err(SdPoseSd2Error::InvalidConfiguration);
            }
        }
        if self.source_exact_profile && self != &Self::source() {
            return Err(SdPoseSd2Error::InvalidConfiguration);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SdPoseSd2WeightSpec {
    key: String,
    shape: Vec<u64>,
}

impl SdPoseSd2WeightSpec {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn shape(&self) -> &[u64] {
        &self.shape
    }
}

#[derive(Debug, Error)]
pub enum SdPoseSd2Error {
    #[error("SDPose SD2 configuration is invalid")]
    InvalidConfiguration,
    #[error("SDPose SD2 production admission requires the exact LotusD family binding")]
    WrongFamily,
    #[error(
        "SDPose SD2 weights differ from the complete source topology; missing={missing:?}, unexpected={unexpected:?}"
    )]
    WeightKeys {
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    #[error(
        "SDPose SD2 weight {key} expected a supported dense CPU dtype and shape {expected:?}, got {dtype:?} {device:?} {actual:?}"
    )]
    WeightShape {
        key: String,
        expected: Vec<u64>,
        actual: Vec<u64>,
        dtype: DType,
        device: DeviceId,
    },
    #[error("SDPose SD2 tensor {name} has invalid shape {actual:?}")]
    InputShape {
        name: &'static str,
        actual: Vec<u64>,
    },
    #[error("SDPose SD2 tensor stream differs from the retained model stream")]
    StreamMismatch,
    #[error("SDPose SD2 forward did not produce the required last pre-output-block capture")]
    MissingCapture,
    #[error("SDPose SD2 capture has invalid shape {0:?}")]
    InvalidCapture(Vec<u64>),
    #[error("SDPose SD2 arithmetic overflowed while computing {0}")]
    Overflow(&'static str),
    #[error("SDPose SD2 retained storage {0:?} has inconsistent byte lengths")]
    InconsistentStorage(StorageId),
    #[error("SDPose SD2 input or retained weight contains a non-finite value")]
    NonFinite,
    #[error(transparent)]
    Cancellation(#[from] CancellationError),
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    TensorOperation(#[from] NativeDiffusionTensorError),
    #[error(transparent)]
    Attention(#[from] AttentionError),
    #[error(transparent)]
    Shape(#[from] ShapeLayoutTransformPartTwoError),
    #[error(transparent)]
    Module(#[from] NativeOpsError),
    #[error(transparent)]
    Operator(#[from] OperatorIndirectionError),
}

#[derive(Clone, Debug)]
pub struct SdPoseSd2ForwardOutput {
    denoised: Tensor,
    feature_640: Tensor,
    capture_output_block: usize,
}

impl SdPoseSd2ForwardOutput {
    pub fn denoised(&self) -> &Tensor {
        &self.denoised
    }

    pub fn feature_640(&self) -> &Tensor {
        &self.feature_640
    }

    pub const fn capture_output_block(&self) -> usize {
        self.capture_output_block
    }
}

#[derive(Clone, Debug)]
pub struct NativeSdPoseSd2Denoiser {
    configuration: SdPoseSd2Configuration,
    weights: BTreeMap<String, Tensor>,
    dtype: DType,
    stream: StreamId,
    semantic_state_digest_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SdPoseHeatmapHeadConfiguration {
    source_exact_profile: bool,
    input_channels: usize,
    hidden_channels: usize,
    output_channels: usize,
}

impl SdPoseHeatmapHeadConfiguration {
    pub const fn source() -> Self {
        Self {
            source_exact_profile: true,
            input_channels: 640,
            hidden_channels: 640,
            output_channels: SDPOSE_HEATMAP_CHANNELS,
        }
    }

    pub fn reduced_fixture(
        input_channels: usize,
        hidden_channels: usize,
        output_channels: usize,
    ) -> Result<Self, SdPoseModelError> {
        let configuration = Self {
            source_exact_profile: false,
            input_channels,
            hidden_channels,
            output_channels,
        };
        configuration.validate()?;
        Ok(configuration)
    }

    pub const fn is_source_exact(&self) -> bool {
        self.source_exact_profile
    }

    pub const fn input_channels(&self) -> usize {
        self.input_channels
    }

    pub const fn output_channels(&self) -> usize {
        self.output_channels
    }

    fn validate(&self) -> Result<(), SdPoseModelError> {
        if self.input_channels == 0 || self.hidden_channels == 0 || self.output_channels == 0 {
            return Err(SdPoseModelError::InvalidConfiguration);
        }
        if self.source_exact_profile && self != &Self::source() {
            return Err(SdPoseModelError::InvalidConfiguration);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SdPoseHeatmapHeadWeightSpec {
    key: String,
    shape: Vec<u64>,
}

impl SdPoseHeatmapHeadWeightSpec {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn shape(&self) -> &[u64] {
        &self.shape
    }
}

#[derive(Clone, Debug)]
pub struct NativeSdPoseHeatmapHead {
    configuration: SdPoseHeatmapHeadConfiguration,
    weights: BTreeMap<String, Tensor>,
    dtype: DType,
    stream: StreamId,
    semantic_state_digest_sha256: String,
}

#[derive(Clone, Debug)]
pub struct NativeSdPoseModel {
    artifact_sha256: String,
    denoiser: NativeSdPoseSd2Denoiser,
    heatmap_head: NativeSdPoseHeatmapHead,
    semantic_state_digest_sha256: String,
}

#[derive(Debug, Error)]
pub enum SdPoseModelError {
    #[error("SDPose model or heatmap-head configuration is invalid")]
    InvalidConfiguration,
    #[error("SDPose model production admission requires the exact source profile")]
    ReducedProductionResource,
    #[error("SDPose model artifact identity is invalid")]
    InvalidArtifactIdentity,
    #[error(
        "SDPose heatmap-head weights differ from the complete source topology; missing={missing:?}, unexpected={unexpected:?}"
    )]
    WeightKeys {
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    #[error("SDPose heatmap-head weight {key} expected shape {expected:?}, got {actual:?}")]
    WeightShape {
        key: String,
        expected: Vec<u64>,
        actual: Vec<u64>,
    },
    #[error("SDPose denoiser and heatmap head target different dtype, device, stream, or channels")]
    ComponentMismatch,
    #[error("SDPose retained storage {0:?} has inconsistent byte lengths")]
    InconsistentStorage(StorageId),
    #[error("SDPose model arithmetic overflowed while computing {0}")]
    Overflow(&'static str),
    #[error("SDPose heatmap-head weight contains a non-finite value")]
    NonFinite,
    #[error(transparent)]
    Denoiser(#[from] SdPoseSd2Error),
    #[error(transparent)]
    Cancellation(#[from] CancellationError),
    #[error(transparent)]
    Tensor(#[from] TensorError),
}

pub fn sdpose_heatmap_head_weight_manifest(
    configuration: &SdPoseHeatmapHeadConfiguration,
) -> Result<Vec<SdPoseHeatmapHeadWeightSpec>, SdPoseModelError> {
    configuration.validate()?;
    let input = u64::try_from(configuration.input_channels)
        .map_err(|_| SdPoseModelError::Overflow("heatmap input channels"))?;
    let hidden = u64::try_from(configuration.hidden_channels)
        .map_err(|_| SdPoseModelError::Overflow("heatmap hidden channels"))?;
    let output = u64::try_from(configuration.output_channels)
        .map_err(|_| SdPoseModelError::Overflow("heatmap output channels"))?;
    Ok(vec![
        SdPoseHeatmapHeadWeightSpec {
            key: "native.heatmap_head.deconv_layers.0.weight".to_owned(),
            shape: vec![input, hidden, 4, 4],
        },
        SdPoseHeatmapHeadWeightSpec {
            key: "native.heatmap_head.conv_layers.0.weight".to_owned(),
            shape: vec![hidden, hidden, 1, 1],
        },
        SdPoseHeatmapHeadWeightSpec {
            key: "native.heatmap_head.conv_layers.0.bias".to_owned(),
            shape: vec![hidden],
        },
        SdPoseHeatmapHeadWeightSpec {
            key: "native.heatmap_head.final_layer.weight".to_owned(),
            shape: vec![output, hidden, 1, 1],
        },
        SdPoseHeatmapHeadWeightSpec {
            key: "native.heatmap_head.final_layer.bias".to_owned(),
            shape: vec![output],
        },
    ])
}

pub fn sdpose_sd2_weight_manifest(
    configuration: &SdPoseSd2Configuration,
) -> Result<Vec<SdPoseSd2WeightSpec>, SdPoseSd2Error> {
    configuration.validate()?;
    let mut specifications = Vec::new();
    let model_channels = configuration.model_channels;
    let embedding_channels = model_channels
        .checked_mul(4)
        .ok_or(SdPoseSd2Error::Overflow("time embedding channels"))?;
    add_convolution_specifications(
        &mut specifications,
        "native.input_blocks.0.0",
        model_channels,
        4,
        3,
    )?;
    add_linear_specifications(
        &mut specifications,
        "native.time_embed.0",
        embedding_channels,
        model_channels,
        true,
    )?;
    add_linear_specifications(
        &mut specifications,
        "native.time_embed.2",
        embedding_channels,
        embedding_channels,
        true,
    )?;
    add_linear_specifications(
        &mut specifications,
        "native.label_emb.0.0",
        embedding_channels,
        4,
        true,
    )?;
    add_linear_specifications(
        &mut specifications,
        "native.label_emb.0.2",
        embedding_channels,
        embedding_channels,
        true,
    )?;

    let mut channels = model_channels;
    let mut input_block_channels = vec![channels];
    let mut input_block = 1usize;
    let mut transformer_index = 0usize;
    for (level, multiplier) in SD2_CHANNEL_MULTIPLIERS.iter().copied().enumerate() {
        for _ in 0..SD2_RESIDUAL_BLOCKS[level] {
            let output_channels = model_channels
                .checked_mul(multiplier)
                .ok_or(SdPoseSd2Error::Overflow("input block channels"))?;
            add_residual_block_specifications(
                &mut specifications,
                &format!("native.input_blocks.{input_block}.0"),
                channels,
                output_channels,
                embedding_channels,
            )?;
            channels = output_channels;
            if SD2_INPUT_TRANSFORMER_DEPTHS[transformer_index] != 0 {
                add_spatial_transformer_specifications(
                    &mut specifications,
                    &format!("native.input_blocks.{input_block}.1"),
                    channels,
                    configuration.context_dimension,
                )?;
            }
            transformer_index += 1;
            input_block_channels.push(channels);
            input_block += 1;
        }
        if level + 1 < SD2_CHANNEL_MULTIPLIERS.len() {
            add_convolution_specifications(
                &mut specifications,
                &format!("native.input_blocks.{input_block}.0.op"),
                channels,
                channels,
                3,
            )?;
            input_block_channels.push(channels);
            input_block += 1;
        }
    }

    add_residual_block_specifications(
        &mut specifications,
        "native.middle_block.0",
        channels,
        channels,
        embedding_channels,
    )?;
    add_spatial_transformer_specifications(
        &mut specifications,
        "native.middle_block.1",
        channels,
        configuration.context_dimension,
    )?;
    add_residual_block_specifications(
        &mut specifications,
        "native.middle_block.2",
        channels,
        channels,
        embedding_channels,
    )?;

    let mut output_depths = SD2_OUTPUT_TRANSFORMER_DEPTHS.to_vec();
    let mut output_block = 0usize;
    for (level, multiplier) in SD2_CHANNEL_MULTIPLIERS.iter().copied().enumerate().rev() {
        for residual_index in 0..=SD2_RESIDUAL_BLOCKS[level] {
            let skip_channels = input_block_channels
                .pop()
                .ok_or(SdPoseSd2Error::InvalidConfiguration)?;
            let input_channels = channels
                .checked_add(skip_channels)
                .ok_or(SdPoseSd2Error::Overflow("output block channels"))?;
            let output_channels = model_channels
                .checked_mul(multiplier)
                .ok_or(SdPoseSd2Error::Overflow("output block channels"))?;
            add_residual_block_specifications(
                &mut specifications,
                &format!("native.output_blocks.{output_block}.0"),
                input_channels,
                output_channels,
                embedding_channels,
            )?;
            channels = output_channels;
            let transformer_depth = output_depths
                .pop()
                .ok_or(SdPoseSd2Error::InvalidConfiguration)?;
            let mut next_layer = 1usize;
            if transformer_depth != 0 {
                add_spatial_transformer_specifications(
                    &mut specifications,
                    &format!("native.output_blocks.{output_block}.1"),
                    channels,
                    configuration.context_dimension,
                )?;
                next_layer += 1;
            }
            if level != 0 && residual_index == SD2_RESIDUAL_BLOCKS[level] {
                add_convolution_specifications(
                    &mut specifications,
                    &format!("native.output_blocks.{output_block}.{next_layer}.conv"),
                    channels,
                    channels,
                    3,
                )?;
            }
            output_block += 1;
        }
    }
    if !input_block_channels.is_empty() || !output_depths.is_empty() || output_block != 12 {
        return Err(SdPoseSd2Error::InvalidConfiguration);
    }
    add_normalization_specifications(&mut specifications, "native.out.0", model_channels)?;
    add_convolution_specifications(&mut specifications, "native.out.2", 4, model_channels, 3)?;

    let mut keys = BTreeSet::new();
    if specifications
        .iter()
        .any(|specification| !keys.insert(specification.key.clone()))
    {
        return Err(SdPoseSd2Error::InvalidConfiguration);
    }
    Ok(specifications)
}

fn push_weight_specification(
    specifications: &mut Vec<SdPoseSd2WeightSpec>,
    key: String,
    shape: &[usize],
) -> Result<(), SdPoseSd2Error> {
    let shape = shape
        .iter()
        .map(|dimension| {
            u64::try_from(*dimension).map_err(|_| SdPoseSd2Error::Overflow("weight shape"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    specifications.push(SdPoseSd2WeightSpec { key, shape });
    Ok(())
}

fn add_linear_specifications(
    specifications: &mut Vec<SdPoseSd2WeightSpec>,
    prefix: &str,
    output: usize,
    input: usize,
    bias: bool,
) -> Result<(), SdPoseSd2Error> {
    push_weight_specification(specifications, format!("{prefix}.weight"), &[output, input])?;
    if bias {
        push_weight_specification(specifications, format!("{prefix}.bias"), &[output])?;
    }
    Ok(())
}

fn add_convolution_specifications(
    specifications: &mut Vec<SdPoseSd2WeightSpec>,
    prefix: &str,
    output: usize,
    input: usize,
    kernel: usize,
) -> Result<(), SdPoseSd2Error> {
    push_weight_specification(
        specifications,
        format!("{prefix}.weight"),
        &[output, input, kernel, kernel],
    )?;
    push_weight_specification(specifications, format!("{prefix}.bias"), &[output])
}

fn add_normalization_specifications(
    specifications: &mut Vec<SdPoseSd2WeightSpec>,
    prefix: &str,
    channels: usize,
) -> Result<(), SdPoseSd2Error> {
    push_weight_specification(specifications, format!("{prefix}.weight"), &[channels])?;
    push_weight_specification(specifications, format!("{prefix}.bias"), &[channels])
}

fn add_residual_block_specifications(
    specifications: &mut Vec<SdPoseSd2WeightSpec>,
    prefix: &str,
    input: usize,
    output: usize,
    embedding: usize,
) -> Result<(), SdPoseSd2Error> {
    add_normalization_specifications(specifications, &format!("{prefix}.in_layers.0"), input)?;
    add_convolution_specifications(
        specifications,
        &format!("{prefix}.in_layers.2"),
        output,
        input,
        3,
    )?;
    add_linear_specifications(
        specifications,
        &format!("{prefix}.emb_layers.1"),
        output,
        embedding,
        true,
    )?;
    add_normalization_specifications(specifications, &format!("{prefix}.out_layers.0"), output)?;
    add_convolution_specifications(
        specifications,
        &format!("{prefix}.out_layers.3"),
        output,
        output,
        3,
    )?;
    if input != output {
        add_convolution_specifications(
            specifications,
            &format!("{prefix}.skip_connection"),
            output,
            input,
            1,
        )?;
    }
    Ok(())
}

fn add_spatial_transformer_specifications(
    specifications: &mut Vec<SdPoseSd2WeightSpec>,
    prefix: &str,
    channels: usize,
    context: usize,
) -> Result<(), SdPoseSd2Error> {
    add_normalization_specifications(specifications, &format!("{prefix}.norm"), channels)?;
    add_linear_specifications(
        specifications,
        &format!("{prefix}.proj_in"),
        channels,
        channels,
        true,
    )?;
    let block = format!("{prefix}.transformer_blocks.0");
    for normalization in ["norm1", "norm2", "norm3"] {
        add_normalization_specifications(
            specifications,
            &format!("{block}.{normalization}"),
            channels,
        )?;
    }
    for attention in ["attn1", "attn2"] {
        let attention_prefix = format!("{block}.{attention}");
        add_linear_specifications(
            specifications,
            &format!("{attention_prefix}.to_q"),
            channels,
            channels,
            false,
        )?;
        let key_value_input = if attention == "attn1" {
            channels
        } else {
            context
        };
        for projection in ["to_k", "to_v"] {
            add_linear_specifications(
                specifications,
                &format!("{attention_prefix}.{projection}"),
                channels,
                key_value_input,
                false,
            )?;
        }
        add_linear_specifications(
            specifications,
            &format!("{attention_prefix}.to_out.0"),
            channels,
            channels,
            true,
        )?;
    }
    let feed_forward_width = channels
        .checked_mul(4)
        .ok_or(SdPoseSd2Error::Overflow("feed-forward width"))?;
    add_linear_specifications(
        specifications,
        &format!("{block}.ff.net.0.proj"),
        feed_forward_width
            .checked_mul(2)
            .ok_or(SdPoseSd2Error::Overflow("GEGLU width"))?,
        channels,
        true,
    )?;
    add_linear_specifications(
        specifications,
        &format!("{block}.ff.net.2"),
        channels,
        feed_forward_width,
        true,
    )?;
    add_linear_specifications(
        specifications,
        &format!("{prefix}.proj_out"),
        channels,
        channels,
        true,
    )
}

impl NativeSdPoseSd2Denoiser {
    pub fn from_mapped_weights(
        mapped: &MappedModelWeights,
        cancellation: &CancellationToken,
    ) -> Result<Self, SdPoseSd2Error> {
        cancellation.check()?;
        let binding = mapped.binding().ok_or(SdPoseSd2Error::WrongFamily)?;
        if binding.family().feature_id()
            != generated_lotusd_comfy_model_0106::MODEL_FAMILY_FEATURE_ID
            || binding.family().identifier()
                != generated_lotusd_comfy_model_0106::MODEL_FAMILY_IDENTIFIER
        {
            return Err(SdPoseSd2Error::WrongFamily);
        }
        let candidate_weights = mapped
            .tensors()
            .iter()
            .filter(|(key, _)| is_sd2_unet_key(key) || key.starts_with("native.heatmap_head."))
            .map(|(key, tensor)| (key.clone(), tensor.clone()))
            .collect::<BTreeMap<_, _>>();
        Self::checked(
            SdPoseSd2Configuration::source(),
            &candidate_weights,
            true,
            cancellation,
        )
    }

    pub fn from_reduced_fixture(
        configuration: SdPoseSd2Configuration,
        weights: BTreeMap<String, Tensor>,
        cancellation: &CancellationToken,
    ) -> Result<Self, SdPoseSd2Error> {
        if configuration.is_source_exact() {
            return Err(SdPoseSd2Error::InvalidConfiguration);
        }
        Self::checked(configuration, &weights, false, cancellation)
    }

    fn checked(
        configuration: SdPoseSd2Configuration,
        candidate_weights: &BTreeMap<String, Tensor>,
        allow_heatmap_head: bool,
        cancellation: &CancellationToken,
    ) -> Result<Self, SdPoseSd2Error> {
        cancellation.check()?;
        configuration.validate()?;
        let manifest = sdpose_sd2_weight_manifest(&configuration)?;
        let expected = manifest
            .iter()
            .map(|specification| specification.key.as_str())
            .collect::<BTreeSet<_>>();
        let actual = candidate_weights
            .keys()
            .filter(|key| !(allow_heatmap_head && key.starts_with("native.heatmap_head.")))
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if expected != actual {
            return Err(SdPoseSd2Error::WeightKeys {
                missing: expected
                    .difference(&actual)
                    .map(|key| (*key).to_owned())
                    .collect(),
                unexpected: actual
                    .difference(&expected)
                    .map(|key| (*key).to_owned())
                    .collect(),
            });
        }
        let first = manifest
            .first()
            .and_then(|specification| candidate_weights.get(&specification.key))
            .ok_or(SdPoseSd2Error::InvalidConfiguration)?;
        let dtype = first.descriptor().dtype();
        let stream = first.descriptor().stream();
        let mut weights = BTreeMap::new();
        for specification in manifest {
            cancellation.check()?;
            let tensor = candidate_weights.get(&specification.key).ok_or_else(|| {
                SdPoseSd2Error::WeightKeys {
                    missing: vec![specification.key.clone()],
                    unexpected: Vec::new(),
                }
            })?;
            let descriptor = tensor.descriptor();
            if descriptor.shape() != specification.shape
                || descriptor.dtype() != dtype
                || !matches!(dtype, DType::F32 | DType::F16 | DType::Bf16)
                || descriptor.device() != DeviceId::CPU
                || descriptor.stream() != stream
            {
                return Err(SdPoseSd2Error::WeightShape {
                    key: specification.key,
                    expected: specification.shape,
                    actual: descriptor.shape().to_vec(),
                    dtype: descriptor.dtype(),
                    device: descriptor.device(),
                });
            }
            require_finite_tensor(tensor, cancellation)?;
            weights.insert(specification.key, tensor.clone());
        }
        let semantic_state_digest_sha256 =
            sdpose_sd2_semantic_digest(&configuration, &weights, cancellation)?;
        Ok(Self {
            configuration,
            weights,
            dtype,
            stream,
            semantic_state_digest_sha256,
        })
    }

    pub fn configuration(&self) -> &SdPoseSd2Configuration {
        &self.configuration
    }

    pub const fn execution_stream(&self) -> StreamId {
        self.stream
    }

    pub const fn execution_dtype(&self) -> DType {
        self.dtype
    }

    pub fn semantic_state_digest_sha256(&self) -> &str {
        &self.semantic_state_digest_sha256
    }

    pub fn validate(&self, cancellation: &CancellationToken) -> Result<(), SdPoseSd2Error> {
        cancellation.check()?;
        self.configuration.validate()?;
        let digest = sdpose_sd2_semantic_digest(&self.configuration, &self.weights, cancellation)?;
        if digest != self.semantic_state_digest_sha256 {
            return Err(SdPoseSd2Error::InvalidConfiguration);
        }
        let manifest = sdpose_sd2_weight_manifest(&self.configuration)?;
        if manifest.len() != self.weights.len() {
            return Err(SdPoseSd2Error::InvalidConfiguration);
        }
        for specification in manifest {
            cancellation.check()?;
            let tensor = self
                .weights
                .get(&specification.key)
                .ok_or(SdPoseSd2Error::InvalidConfiguration)?;
            if tensor.descriptor().shape() != specification.shape
                || tensor.descriptor().dtype() != self.dtype
                || tensor.descriptor().device() != DeviceId::CPU
                || tensor.descriptor().stream() != self.stream
            {
                return Err(SdPoseSd2Error::InvalidConfiguration);
            }
        }
        cancellation.check()?;
        Ok(())
    }

    pub fn resident_tensor_allocations(&self) -> Result<Vec<(StorageId, u64)>, SdPoseSd2Error> {
        let mut allocations = HashMap::new();
        for tensor in self.weights.values() {
            let storage = tensor.storage_id();
            let bytes = tensor.storage_byte_len();
            if let Some(existing) = allocations.insert(storage, bytes)
                && existing != bytes
            {
                return Err(SdPoseSd2Error::InconsistentStorage(storage));
            }
        }
        Ok(allocations.into_iter().collect())
    }

    pub fn resident_owned_bytes(&self) -> Result<u64, SdPoseSd2Error> {
        let entries = self
            .weights
            .len()
            .checked_mul(std::mem::size_of::<(String, Tensor)>())
            .ok_or(SdPoseSd2Error::Overflow("retained weight entries"))?;
        let keys = self.weights.keys().try_fold(0usize, |total, key| {
            total
                .checked_add(key.capacity())
                .ok_or(SdPoseSd2Error::Overflow("retained weight keys"))
        })?;
        let owned = std::mem::size_of::<Self>()
            .checked_add(entries)
            .and_then(|bytes| bytes.checked_add(keys))
            .and_then(|bytes| bytes.checked_add(self.semantic_state_digest_sha256.capacity()))
            .ok_or(SdPoseSd2Error::Overflow("retained owner bytes"))?;
        u64::try_from(owned).map_err(|_| SdPoseSd2Error::Overflow("retained owner bytes"))
    }

    pub fn resident_bytes(&self) -> Result<u64, SdPoseSd2Error> {
        self.resident_tensor_allocations()?.into_iter().try_fold(
            self.resident_owned_bytes()?,
            |total, (_, bytes)| {
                total
                    .checked_add(bytes)
                    .ok_or(SdPoseSd2Error::Overflow("retained total bytes"))
            },
        )
    }

    fn weight(&self, key: &str) -> Result<&Tensor, SdPoseSd2Error> {
        self.weights
            .get(key)
            .ok_or(SdPoseSd2Error::InvalidConfiguration)
    }

    pub fn forward(
        &self,
        backend: &CpuBackend,
        latent: &Tensor,
        timesteps: &[f32],
        conditioning: &Tensor,
        adm: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<SdPoseSd2ForwardOutput, SdPoseSd2Error> {
        context.cancellation.check()?;
        context.check()?;
        self.validate(context.cancellation)?;
        if context.stream != self.stream {
            return Err(SdPoseSd2Error::StreamMismatch);
        }
        let batch = require_forward_inputs(
            &self.configuration,
            self.dtype,
            self.stream,
            latent,
            timesteps,
            conditioning,
            adm,
        )?;
        require_finite_tensor(latent, context.cancellation)?;
        require_finite_tensor(conditioning, context.cancellation)?;
        require_finite_tensor(adm, context.cancellation)?;

        let timestep_embedding = build_timestep_embedding(
            backend,
            timesteps,
            self.configuration.model_channels,
            self.dtype,
            context,
        )?;
        let mut embedding = self.linear(
            backend,
            &timestep_embedding,
            "native.time_embed.0",
            true,
            context,
        )?;
        embedding = immutable_silu(backend, &embedding, context)?;
        embedding = self.linear(backend, &embedding, "native.time_embed.2", true, context)?;
        let mut label = self.linear(backend, adm, "native.label_emb.0.0", true, context)?;
        label = immutable_silu(backend, &label, context)?;
        label = self.linear(backend, &label, "native.label_emb.0.2", true, context)?;
        embedding = add_tensors(backend, &embedding, &label, context)?;

        let mut hidden =
            self.convolution(backend, latent, "native.input_blocks.0.0", 1, 1, context)?;
        let mut skips = Vec::new();
        skips
            .try_reserve_exact(12)
            .map_err(|_| SdPoseSd2Error::Overflow("skip allocation"))?;
        skips.push(hidden.clone());
        let mut input_block = 1usize;
        let mut transformer_index = 0usize;
        for (level, _) in SD2_CHANNEL_MULTIPLIERS.iter().enumerate() {
            for _ in 0..SD2_RESIDUAL_BLOCKS[level] {
                context.check()?;
                hidden = self.residual_block(
                    backend,
                    &hidden,
                    &embedding,
                    &format!("native.input_blocks.{input_block}.0"),
                    context,
                )?;
                if SD2_INPUT_TRANSFORMER_DEPTHS[transformer_index] != 0 {
                    hidden = self.spatial_transformer(
                        backend,
                        &hidden,
                        conditioning,
                        &format!("native.input_blocks.{input_block}.1"),
                        context,
                    )?;
                }
                transformer_index += 1;
                skips.push(hidden.clone());
                input_block += 1;
            }
            if level + 1 < SD2_CHANNEL_MULTIPLIERS.len() {
                hidden = self.convolution(
                    backend,
                    &hidden,
                    &format!("native.input_blocks.{input_block}.0.op"),
                    2,
                    1,
                    context,
                )?;
                skips.push(hidden.clone());
                input_block += 1;
            }
        }

        hidden = self.residual_block(
            backend,
            &hidden,
            &embedding,
            "native.middle_block.0",
            context,
        )?;
        hidden = self.spatial_transformer(
            backend,
            &hidden,
            conditioning,
            "native.middle_block.1",
            context,
        )?;
        hidden = self.residual_block(
            backend,
            &hidden,
            &embedding,
            "native.middle_block.2",
            context,
        )?;

        let mut capture = None;
        let mut capture_output_block = None;
        let mut output_depths = SD2_OUTPUT_TRANSFORMER_DEPTHS.to_vec();
        let mut output_block = 0usize;
        for (level, _) in SD2_CHANNEL_MULTIPLIERS.iter().enumerate().rev() {
            for residual_index in 0..=SD2_RESIDUAL_BLOCKS[level] {
                context.check()?;
                let hidden_shape = hidden.descriptor().shape();
                if hidden_shape.get(1).copied()
                    == Some(
                        u64::try_from(self.configuration.capture_channels())
                            .map_err(|_| SdPoseSd2Error::Overflow("capture channels"))?,
                    )
                {
                    capture = Some(copy_tensor(backend, &hidden, context)?);
                    capture_output_block = Some(output_block);
                }
                let skip = skips.pop().ok_or(SdPoseSd2Error::MissingCapture)?;
                hidden = concat_channel_tensors(backend, &hidden, &skip, context)?;
                hidden = self.residual_block(
                    backend,
                    &hidden,
                    &embedding,
                    &format!("native.output_blocks.{output_block}.0"),
                    context,
                )?;
                let transformer_depth = output_depths
                    .pop()
                    .ok_or(SdPoseSd2Error::InvalidConfiguration)?;
                let mut next_layer = 1usize;
                if transformer_depth != 0 {
                    hidden = self.spatial_transformer(
                        backend,
                        &hidden,
                        conditioning,
                        &format!("native.output_blocks.{output_block}.1"),
                        context,
                    )?;
                    next_layer += 1;
                }
                if level != 0 && residual_index == SD2_RESIDUAL_BLOCKS[level] {
                    hidden = nearest_upsample_tensor_2x(backend, &hidden, context)?;
                    hidden = self.convolution(
                        backend,
                        &hidden,
                        &format!("native.output_blocks.{output_block}.{next_layer}.conv"),
                        1,
                        1,
                        context,
                    )?;
                }
                output_block += 1;
            }
        }
        if !skips.is_empty() || !output_depths.is_empty() || output_block != 12 {
            return Err(SdPoseSd2Error::InvalidConfiguration);
        }
        hidden = self.normalization(backend, &hidden, "native.out.0", 1.0e-5, context)?;
        hidden = immutable_silu(backend, &hidden, context)?;
        let denoised = self.convolution(backend, &hidden, "native.out.2", 1, 1, context)?;
        let feature_640 = capture.ok_or(SdPoseSd2Error::MissingCapture)?;
        let expected_capture = [
            u64::try_from(batch).map_err(|_| SdPoseSd2Error::Overflow("capture batch"))?,
            u64::try_from(self.configuration.capture_channels())
                .map_err(|_| SdPoseSd2Error::Overflow("capture channels"))?,
            u64::try_from(self.configuration.latent_height)
                .map_err(|_| SdPoseSd2Error::Overflow("capture height"))?,
            u64::try_from(self.configuration.latent_width)
                .map_err(|_| SdPoseSd2Error::Overflow("capture width"))?,
        ];
        if feature_640.descriptor().shape() != expected_capture {
            return Err(SdPoseSd2Error::InvalidCapture(
                feature_640.descriptor().shape().to_vec(),
            ));
        }
        if self.configuration.is_source_exact() && capture_output_block != Some(9) {
            return Err(SdPoseSd2Error::InvalidCapture(
                feature_640.descriptor().shape().to_vec(),
            ));
        }
        context.check()?;
        Ok(SdPoseSd2ForwardOutput {
            denoised,
            feature_640,
            capture_output_block: capture_output_block.ok_or(SdPoseSd2Error::MissingCapture)?,
        })
    }

    fn linear(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        prefix: &str,
        bias: bool,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, SdPoseSd2Error> {
        let weight = self.weight(&format!("{prefix}.weight"))?;
        let shape = weight.descriptor().shape();
        let [output_features, input_features]: [u64; 2] =
            shape.try_into().map_err(|_| SdPoseSd2Error::InputShape {
                name: "linear weight",
                actual: shape.to_vec(),
            })?;
        let mut module = NativeModule::linear(
            prefix,
            usize::try_from(input_features)
                .map_err(|_| SdPoseSd2Error::Overflow("linear input features"))?,
            usize::try_from(output_features)
                .map_err(|_| SdPoseSd2Error::Overflow("linear output features"))?,
            bias,
            false,
        )?;
        module.load_dense_parameters(
            weight.clone(),
            bias.then(|| self.weight(&format!("{prefix}.bias")).cloned())
                .transpose()?,
        )?;
        Ok(module.forward_dense_inference_with_context(backend, input, context)?)
    }

    fn convolution(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        prefix: &str,
        stride: usize,
        padding: usize,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, SdPoseSd2Error> {
        let weight = self.weight(&format!("{prefix}.weight"))?;
        let shape = weight.descriptor().shape();
        let [output_channels, input_channels, kernel_height, kernel_width]: [u64; 4] =
            shape.try_into().map_err(|_| SdPoseSd2Error::InputShape {
                name: "convolution weight",
                actual: shape.to_vec(),
            })?;
        let geometry = ConvolutionGeometry::new(
            2,
            vec![stride; 2],
            vec![padding; 2],
            vec![1; 2],
            1,
            false,
            vec![0; 2],
        )?;
        let mut module = NativeModule::convolution(
            prefix,
            usize::try_from(input_channels)
                .map_err(|_| SdPoseSd2Error::Overflow("convolution input channels"))?,
            usize::try_from(output_channels)
                .map_err(|_| SdPoseSd2Error::Overflow("convolution output channels"))?,
            vec![
                usize::try_from(kernel_height)
                    .map_err(|_| SdPoseSd2Error::Overflow("convolution kernel height"))?,
                usize::try_from(kernel_width)
                    .map_err(|_| SdPoseSd2Error::Overflow("convolution kernel width"))?,
            ],
            true,
            geometry,
            false,
        )?;
        module.load_dense_parameters(
            weight.clone(),
            Some(self.weight(&format!("{prefix}.bias"))?.clone()),
        )?;
        Ok(module.forward_dense_inference_with_context(backend, input, context)?)
    }

    fn normalization(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        prefix: &str,
        epsilon: f32,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, SdPoseSd2Error> {
        let channels = usize::try_from(*input.descriptor().shape().get(1).ok_or(
            SdPoseSd2Error::InputShape {
                name: "group norm input",
                actual: input.descriptor().shape().to_vec(),
            },
        )?)
        .map_err(|_| SdPoseSd2Error::Overflow("group norm channels"))?;
        let mut module = NativeModule::group_norm(
            prefix,
            self.configuration.normalization_groups,
            channels,
            epsilon,
            true,
            false,
        )?;
        module.load_dense_parameters(
            self.weight(&format!("{prefix}.weight"))?.clone(),
            Some(self.weight(&format!("{prefix}.bias"))?.clone()),
        )?;
        Ok(module.forward_dense_inference_with_context(backend, input, context)?)
    }

    fn residual_block(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        embedding: &Tensor,
        prefix: &str,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, SdPoseSd2Error> {
        context.check()?;
        let mut hidden = self.normalization(
            backend,
            input,
            &format!("{prefix}.in_layers.0"),
            1.0e-5,
            context,
        )?;
        hidden = immutable_silu(backend, &hidden, context)?;
        hidden = self.convolution(
            backend,
            &hidden,
            &format!("{prefix}.in_layers.2"),
            1,
            1,
            context,
        )?;
        let embedding = immutable_silu(backend, embedding, context)?;
        let embedding = self.linear(
            backend,
            &embedding,
            &format!("{prefix}.emb_layers.1"),
            true,
            context,
        )?;
        hidden = add_embedding_bias(backend, &hidden, &embedding, context)?;
        hidden = self.normalization(
            backend,
            &hidden,
            &format!("{prefix}.out_layers.0"),
            1.0e-5,
            context,
        )?;
        hidden = immutable_silu(backend, &hidden, context)?;
        hidden = self.convolution(
            backend,
            &hidden,
            &format!("{prefix}.out_layers.3"),
            1,
            1,
            context,
        )?;
        let residual = if input.descriptor().shape().get(1) == hidden.descriptor().shape().get(1) {
            input.clone()
        } else {
            self.convolution(
                backend,
                input,
                &format!("{prefix}.skip_connection"),
                1,
                0,
                context,
            )?
        };
        add_tensors(backend, &residual, &hidden, context)
    }

    fn spatial_transformer(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        conditioning: &Tensor,
        prefix: &str,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, SdPoseSd2Error> {
        context.check()?;
        let shape = require_rank_four(input, "spatial transformer")?;
        let mut hidden =
            self.normalization(backend, input, &format!("{prefix}.norm"), 1.0e-6, context)?;
        hidden = nchw_to_tokens(backend, &hidden, context)?;
        hidden = self.linear(
            backend,
            &hidden,
            &format!("{prefix}.proj_in"),
            true,
            context,
        )?;
        let block = format!("{prefix}.transformer_blocks.0");

        let normalized =
            self.layer_normalization(backend, &hidden, &format!("{block}.norm1"), context)?;
        let attended = self.cross_attention(
            backend,
            &normalized,
            &normalized,
            &format!("{block}.attn1"),
            context,
        )?;
        hidden = add_tensors(backend, &hidden, &attended, context)?;

        let normalized =
            self.layer_normalization(backend, &hidden, &format!("{block}.norm2"), context)?;
        let attended = self.cross_attention(
            backend,
            &normalized,
            conditioning,
            &format!("{block}.attn2"),
            context,
        )?;
        hidden = add_tensors(backend, &hidden, &attended, context)?;

        let normalized =
            self.layer_normalization(backend, &hidden, &format!("{block}.norm3"), context)?;
        let projected = self.linear(
            backend,
            &normalized,
            &format!("{block}.ff.net.0.proj"),
            true,
            context,
        )?;
        let gated = geglu(backend, &projected, context)?;
        let feed_forward =
            self.linear(backend, &gated, &format!("{block}.ff.net.2"), true, context)?;
        hidden = add_tensors(backend, &hidden, &feed_forward, context)?;
        hidden = self.linear(
            backend,
            &hidden,
            &format!("{prefix}.proj_out"),
            true,
            context,
        )?;
        hidden = tokens_to_nchw(backend, &hidden, shape[2], shape[3], context)?;
        add_tensors(backend, input, &hidden, context)
    }

    fn layer_normalization(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        prefix: &str,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, SdPoseSd2Error> {
        let width = usize::try_from(*input.descriptor().shape().last().ok_or(
            SdPoseSd2Error::InputShape {
                name: "layer norm input",
                actual: input.descriptor().shape().to_vec(),
            },
        )?)
        .map_err(|_| SdPoseSd2Error::Overflow("layer norm width"))?;
        let mut module = NativeModule::layer_norm(prefix, vec![width], 1.0e-5, true, true, false)?;
        module.load_dense_parameters(
            self.weight(&format!("{prefix}.weight"))?.clone(),
            Some(self.weight(&format!("{prefix}.bias"))?.clone()),
        )?;
        Ok(module.forward_dense_inference_with_context(backend, input, context)?)
    }

    fn cross_attention(
        &self,
        backend: &CpuBackend,
        query_input: &Tensor,
        key_value_input: &Tensor,
        prefix: &str,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, SdPoseSd2Error> {
        let query = self.linear(
            backend,
            query_input,
            &format!("{prefix}.to_q"),
            false,
            context,
        )?;
        let key = self.linear(
            backend,
            key_value_input,
            &format!("{prefix}.to_k"),
            false,
            context,
        )?;
        let value = self.linear(
            backend,
            key_value_input,
            &format!("{prefix}.to_v"),
            false,
            context,
        )?;
        let query_shape = require_rank_three(&query, "attention query")?;
        let key_shape = require_rank_three(&key, "attention key")?;
        let value_shape = require_rank_three(&value, "attention value")?;
        if query_shape[0] != key_shape[0]
            || key_shape != value_shape
            || query_shape[2] != key_shape[2]
        {
            return Err(SdPoseSd2Error::InputShape {
                name: "attention",
                actual: query.descriptor().shape().to_vec(),
            });
        }
        let channels = usize::try_from(query_shape[2])
            .map_err(|_| SdPoseSd2Error::Overflow("attention channels"))?;
        let heads = channels
            .checked_div(self.configuration.attention_head_channels)
            .ok_or(SdPoseSd2Error::Overflow("attention heads"))?;
        let batch = usize::try_from(query_shape[0])
            .map_err(|_| SdPoseSd2Error::Overflow("attention batch"))?;
        let query_tokens = usize::try_from(query_shape[1])
            .map_err(|_| SdPoseSd2Error::Overflow("attention queries"))?;
        let key_tokens = usize::try_from(key_shape[1])
            .map_err(|_| SdPoseSd2Error::Overflow("attention keys"))?;
        let batch_i64 =
            i64::try_from(batch).map_err(|_| SdPoseSd2Error::Overflow("attention batch"))?;
        let query_tokens_i64 = i64::try_from(query_tokens)
            .map_err(|_| SdPoseSd2Error::Overflow("attention queries"))?;
        let key_tokens_i64 =
            i64::try_from(key_tokens).map_err(|_| SdPoseSd2Error::Overflow("attention keys"))?;
        let heads_i64 =
            i64::try_from(heads).map_err(|_| SdPoseSd2Error::Overflow("attention heads"))?;
        let head_dimension_i64 = i64::try_from(self.configuration.attention_head_channels)
            .map_err(|_| SdPoseSd2Error::Overflow("attention head dimension"))?;
        let query = tensor_reshape_with_context_exact_native(
            backend,
            &query,
            &[batch_i64, query_tokens_i64, heads_i64, head_dimension_i64],
            context,
        )?;
        let key = tensor_reshape_with_context_exact_native(
            backend,
            &key,
            &[batch_i64, key_tokens_i64, heads_i64, head_dimension_i64],
            context,
        )?;
        let value = tensor_reshape_with_context_exact_native(
            backend,
            &value,
            &[batch_i64, key_tokens_i64, heads_i64, head_dimension_i64],
            context,
        )?;
        let attended = scaled_dot_product_attention_tensor_with_context(
            backend,
            AttentionRequest {
                backend: AttentionBackend::SplitOrSubQuadratic,
                fallback: AttentionFallbackPolicy::AllowExactNative,
                batch,
                query_tokens,
                key_tokens,
                heads,
                head_dimension: self.configuration.attention_head_channels,
                value_dimension: self.configuration.attention_head_channels,
                scale: None,
                workspace_limit_bytes: key_tokens
                    .checked_mul(std::mem::size_of::<f32>())
                    .ok_or(SdPoseSd2Error::Overflow("attention workspace"))?,
            },
            &query,
            &key,
            &value,
            None,
            context,
        )?;
        let attended = tensor_reshape_with_context_exact_native(
            backend,
            &attended,
            &[
                batch_i64,
                query_tokens_i64,
                i64::try_from(channels)
                    .map_err(|_| SdPoseSd2Error::Overflow("attention channels"))?,
            ],
            context,
        )?;
        self.linear(
            backend,
            &attended,
            &format!("{prefix}.to_out.0"),
            true,
            context,
        )
    }
}

impl NativeSdPoseHeatmapHead {
    fn from_mapped_weights(
        mapped: &MappedModelWeights,
        cancellation: &CancellationToken,
    ) -> Result<Self, SdPoseModelError> {
        let weights = mapped
            .tensors()
            .iter()
            .filter(|(key, _)| key.starts_with("native.heatmap_head."))
            .map(|(key, tensor)| (key.clone(), tensor.clone()))
            .collect();
        Self::checked(
            SdPoseHeatmapHeadConfiguration::source(),
            weights,
            cancellation,
        )
    }

    pub fn from_reduced_fixture(
        configuration: SdPoseHeatmapHeadConfiguration,
        weights: BTreeMap<String, Tensor>,
        cancellation: &CancellationToken,
    ) -> Result<Self, SdPoseModelError> {
        if configuration.is_source_exact() {
            return Err(SdPoseModelError::InvalidConfiguration);
        }
        Self::checked(configuration, weights, cancellation)
    }

    fn checked(
        configuration: SdPoseHeatmapHeadConfiguration,
        candidate_weights: BTreeMap<String, Tensor>,
        cancellation: &CancellationToken,
    ) -> Result<Self, SdPoseModelError> {
        cancellation.check()?;
        configuration.validate()?;
        let manifest = sdpose_heatmap_head_weight_manifest(&configuration)?;
        let expected = manifest
            .iter()
            .map(|specification| specification.key.as_str())
            .collect::<BTreeSet<_>>();
        let actual = candidate_weights
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if expected != actual {
            return Err(SdPoseModelError::WeightKeys {
                missing: expected
                    .difference(&actual)
                    .map(|key| (*key).to_owned())
                    .collect(),
                unexpected: actual
                    .difference(&expected)
                    .map(|key| (*key).to_owned())
                    .collect(),
            });
        }
        let first = manifest
            .first()
            .and_then(|specification| candidate_weights.get(&specification.key))
            .ok_or(SdPoseModelError::InvalidConfiguration)?;
        let dtype = first.descriptor().dtype();
        let stream = first.descriptor().stream();
        if !matches!(dtype, DType::F32 | DType::F16 | DType::Bf16) {
            return Err(SdPoseModelError::InvalidConfiguration);
        }
        for specification in &manifest {
            cancellation.check()?;
            let tensor = candidate_weights
                .get(&specification.key)
                .ok_or(SdPoseModelError::InvalidConfiguration)?;
            let descriptor = tensor.descriptor();
            if descriptor.shape() != specification.shape
                || descriptor.dtype() != dtype
                || descriptor.device() != DeviceId::CPU
                || descriptor.stream() != stream
            {
                return Err(SdPoseModelError::WeightShape {
                    key: specification.key.clone(),
                    expected: specification.shape.clone(),
                    actual: descriptor.shape().to_vec(),
                });
            }
            require_finite_heatmap_tensor(tensor, cancellation)?;
        }
        let semantic_state_digest_sha256 =
            sdpose_heatmap_head_semantic_digest(&configuration, &candidate_weights, cancellation)?;
        Ok(Self {
            configuration,
            weights: candidate_weights,
            dtype,
            stream,
            semantic_state_digest_sha256,
        })
    }

    pub fn configuration(&self) -> &SdPoseHeatmapHeadConfiguration {
        &self.configuration
    }

    pub const fn execution_dtype(&self) -> DType {
        self.dtype
    }

    pub const fn execution_stream(&self) -> StreamId {
        self.stream
    }

    pub fn semantic_state_digest_sha256(&self) -> &str {
        &self.semantic_state_digest_sha256
    }

    pub fn validate(&self, cancellation: &CancellationToken) -> Result<(), SdPoseModelError> {
        cancellation.check()?;
        self.configuration.validate()?;
        let expected = sdpose_heatmap_head_weight_manifest(&self.configuration)?;
        if expected.len() != self.weights.len() {
            return Err(SdPoseModelError::InvalidConfiguration);
        }
        for specification in expected {
            cancellation.check()?;
            let tensor = self
                .weights
                .get(&specification.key)
                .ok_or(SdPoseModelError::InvalidConfiguration)?;
            if tensor.descriptor().shape() != specification.shape
                || tensor.descriptor().dtype() != self.dtype
                || tensor.descriptor().device() != DeviceId::CPU
                || tensor.descriptor().stream() != self.stream
            {
                return Err(SdPoseModelError::InvalidConfiguration);
            }
        }
        if self.semantic_state_digest_sha256
            != sdpose_heatmap_head_semantic_digest(
                &self.configuration,
                &self.weights,
                cancellation,
            )?
        {
            return Err(SdPoseModelError::InvalidConfiguration);
        }
        self.resident_tensor_allocations()?;
        cancellation.check()?;
        Ok(())
    }

    pub fn resident_tensor_allocations(&self) -> Result<Vec<(StorageId, u64)>, SdPoseModelError> {
        checked_sdpose_storage_union(
            self.weights
                .values()
                .map(|tensor| (tensor.storage_id(), tensor.storage_byte_len())),
        )
    }

    pub fn resident_owned_bytes(&self) -> Result<u64, SdPoseModelError> {
        let entries = self
            .weights
            .len()
            .checked_mul(std::mem::size_of::<(String, Tensor)>())
            .ok_or(SdPoseModelError::Overflow("heatmap retained entries"))?;
        let keys = self.weights.keys().try_fold(0usize, |total, key| {
            total
                .checked_add(key.capacity())
                .ok_or(SdPoseModelError::Overflow("heatmap retained keys"))
        })?;
        let bytes = std::mem::size_of::<Self>()
            .checked_add(entries)
            .and_then(|bytes| bytes.checked_add(keys))
            .and_then(|bytes| bytes.checked_add(self.semantic_state_digest_sha256.capacity()))
            .ok_or(SdPoseModelError::Overflow("heatmap owner residency"))?;
        u64::try_from(bytes).map_err(|_| SdPoseModelError::Overflow("heatmap owner residency"))
    }

    pub fn resident_bytes(&self) -> Result<u64, SdPoseModelError> {
        self.resident_tensor_allocations()?.into_iter().try_fold(
            self.resident_owned_bytes()?,
            |total, (_, bytes)| {
                total
                    .checked_add(bytes)
                    .ok_or(SdPoseModelError::Overflow("heatmap total residency"))
            },
        )
    }
}

impl NativeSdPoseModel {
    pub fn from_mapped_weights(
        mapped: &MappedModelWeights,
        cancellation: &CancellationToken,
    ) -> Result<Self, SdPoseModelError> {
        cancellation.check()?;
        let denoiser = NativeSdPoseSd2Denoiser::from_mapped_weights(mapped, cancellation)?;
        let heatmap_head = NativeSdPoseHeatmapHead::from_mapped_weights(mapped, cancellation)?;
        Self::checked(
            mapped.base_artifact_digest().to_owned(),
            denoiser,
            heatmap_head,
            true,
            cancellation,
        )
    }

    #[doc(hidden)]
    pub fn from_reduced_fixture(
        artifact_sha256: String,
        denoiser: NativeSdPoseSd2Denoiser,
        heatmap_head: NativeSdPoseHeatmapHead,
        cancellation: &CancellationToken,
    ) -> Result<Self, SdPoseModelError> {
        Self::checked(artifact_sha256, denoiser, heatmap_head, false, cancellation)
    }

    fn checked(
        artifact_sha256: String,
        denoiser: NativeSdPoseSd2Denoiser,
        heatmap_head: NativeSdPoseHeatmapHead,
        require_source_exact: bool,
        cancellation: &CancellationToken,
    ) -> Result<Self, SdPoseModelError> {
        cancellation.check()?;
        if !valid_sdpose_sha256(&artifact_sha256) {
            return Err(SdPoseModelError::InvalidArtifactIdentity);
        }
        denoiser.validate(cancellation)?;
        heatmap_head.validate(cancellation)?;
        let source_exact = denoiser.configuration().is_source_exact()
            && heatmap_head.configuration().is_source_exact();
        if require_source_exact && !source_exact {
            return Err(SdPoseModelError::ReducedProductionResource);
        }
        if denoiser.configuration().capture_channels()
            != heatmap_head.configuration().input_channels()
            || denoiser.execution_dtype() != heatmap_head.execution_dtype()
            || denoiser.execution_stream() != heatmap_head.execution_stream()
        {
            return Err(SdPoseModelError::ComponentMismatch);
        }
        let semantic_state_digest_sha256 =
            sdpose_model_semantic_digest(&artifact_sha256, &denoiser, &heatmap_head, cancellation)?;
        let model = Self {
            artifact_sha256,
            denoiser,
            heatmap_head,
            semantic_state_digest_sha256,
        };
        model.resident_tensor_allocations()?;
        cancellation.check()?;
        Ok(model)
    }

    pub fn artifact_sha256(&self) -> &str {
        &self.artifact_sha256
    }

    pub fn denoiser(&self) -> &NativeSdPoseSd2Denoiser {
        &self.denoiser
    }

    pub fn heatmap_head(&self) -> &NativeSdPoseHeatmapHead {
        &self.heatmap_head
    }

    pub fn is_source_exact_profile(&self) -> bool {
        self.denoiser.configuration().is_source_exact()
            && self.heatmap_head.configuration().is_source_exact()
    }

    pub fn semantic_state_digest_sha256(&self) -> &str {
        &self.semantic_state_digest_sha256
    }

    pub fn validate(&self, cancellation: &CancellationToken) -> Result<(), SdPoseModelError> {
        cancellation.check()?;
        if !valid_sdpose_sha256(&self.artifact_sha256) {
            return Err(SdPoseModelError::InvalidArtifactIdentity);
        }
        self.denoiser.validate(cancellation)?;
        self.heatmap_head.validate(cancellation)?;
        if self.denoiser.configuration().capture_channels()
            != self.heatmap_head.configuration().input_channels()
            || self.denoiser.execution_dtype() != self.heatmap_head.execution_dtype()
            || self.denoiser.execution_stream() != self.heatmap_head.execution_stream()
        {
            return Err(SdPoseModelError::ComponentMismatch);
        }
        if self.semantic_state_digest_sha256
            != sdpose_model_semantic_digest(
                &self.artifact_sha256,
                &self.denoiser,
                &self.heatmap_head,
                cancellation,
            )?
        {
            return Err(SdPoseModelError::InvalidConfiguration);
        }
        self.resident_tensor_allocations()?;
        cancellation.check()?;
        Ok(())
    }

    pub fn resident_tensor_allocations(&self) -> Result<Vec<(StorageId, u64)>, SdPoseModelError> {
        checked_sdpose_storage_union(
            self.denoiser
                .resident_tensor_allocations()?
                .into_iter()
                .chain(self.heatmap_head.resident_tensor_allocations()?),
        )
    }

    pub fn resident_owned_bytes(&self) -> Result<u64, SdPoseModelError> {
        let denoiser_inline = u64::try_from(std::mem::size_of::<NativeSdPoseSd2Denoiser>())
            .map_err(|_| SdPoseModelError::Overflow("SDPose denoiser inline residency"))?;
        let head_inline = u64::try_from(std::mem::size_of::<NativeSdPoseHeatmapHead>())
            .map_err(|_| SdPoseModelError::Overflow("SDPose head inline residency"))?;
        let denoiser_owned = self
            .denoiser
            .resident_owned_bytes()?
            .checked_sub(denoiser_inline)
            .ok_or(SdPoseModelError::Overflow(
                "SDPose denoiser owner residency",
            ))?;
        let head_owned = self
            .heatmap_head
            .resident_owned_bytes()?
            .checked_sub(head_inline)
            .ok_or(SdPoseModelError::Overflow("SDPose head owner residency"))?;
        let artifact_capacity = u64::try_from(self.artifact_sha256.capacity())
            .map_err(|_| SdPoseModelError::Overflow("SDPose artifact residency"))?;
        let digest_capacity = u64::try_from(self.semantic_state_digest_sha256.capacity())
            .map_err(|_| SdPoseModelError::Overflow("SDPose digest residency"))?;
        let owned = u64::try_from(std::mem::size_of::<Self>())
            .map_err(|_| SdPoseModelError::Overflow("SDPose owner residency"))?
            .checked_add(denoiser_owned)
            .and_then(|bytes| bytes.checked_add(head_owned))
            .and_then(|bytes| bytes.checked_add(artifact_capacity))
            .and_then(|bytes| bytes.checked_add(digest_capacity))
            .ok_or(SdPoseModelError::Overflow("SDPose owner residency"))?;
        Ok(owned)
    }

    pub fn resident_bytes(&self) -> Result<u64, SdPoseModelError> {
        self.resident_tensor_allocations()?.into_iter().try_fold(
            self.resident_owned_bytes()?,
            |total, (_, bytes)| {
                total
                    .checked_add(bytes)
                    .ok_or(SdPoseModelError::Overflow("SDPose total residency"))
            },
        )
    }
}

fn valid_sdpose_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn checked_sdpose_storage_union(
    allocations: impl IntoIterator<Item = (StorageId, u64)>,
) -> Result<Vec<(StorageId, u64)>, SdPoseModelError> {
    let mut unique = BTreeMap::<u64, (StorageId, u64)>::new();
    for (storage_id, bytes) in allocations {
        if let Some((_, existing)) = unique.get(&storage_id.get()) {
            if *existing != bytes {
                return Err(SdPoseModelError::InconsistentStorage(storage_id));
            }
        } else {
            unique.insert(storage_id.get(), (storage_id, bytes));
        }
    }
    Ok(unique.into_values().collect())
}

fn sdpose_heatmap_head_semantic_digest(
    configuration: &SdPoseHeatmapHeadConfiguration,
    weights: &BTreeMap<String, Tensor>,
    cancellation: &CancellationToken,
) -> Result<String, SdPoseModelError> {
    cancellation.check()?;
    let mut digest = Sha256::new();
    digest.update(SDPOSE_HEATMAP_HEAD_SOURCE_DOMAIN);
    digest.update(SDPOSE_HEAD_SOURCE_SHA256.as_bytes());
    digest.update(SDPOSE_MODEL_DETECTION_SOURCE_SHA256.as_bytes());
    digest.update([u8::from(configuration.source_exact_profile)]);
    for value in [
        configuration.input_channels,
        configuration.hidden_channels,
        configuration.output_channels,
    ] {
        digest.update(
            u64::try_from(value)
                .map_err(|_| SdPoseModelError::Overflow("heatmap configuration digest"))?
                .to_le_bytes(),
        );
    }
    hash_sdpose_weight_map(&mut digest, weights, cancellation)?;
    Ok(format!("{:x}", digest.finalize()))
}

fn sdpose_model_semantic_digest(
    artifact_sha256: &str,
    denoiser: &NativeSdPoseSd2Denoiser,
    heatmap_head: &NativeSdPoseHeatmapHead,
    cancellation: &CancellationToken,
) -> Result<String, SdPoseModelError> {
    cancellation.check()?;
    let mut digest = Sha256::new();
    digest.update(SDPOSE_MODEL_SOURCE_DOMAIN);
    for field in [
        artifact_sha256,
        generated_lotusd_comfy_model_0106::MODEL_FAMILY_FEATURE_ID,
        generated_lotusd_comfy_model_0106::MODEL_FAMILY_IDENTIFIER,
        denoiser.semantic_state_digest_sha256(),
        heatmap_head.semantic_state_digest_sha256(),
        SDPOSE_HEAD_SOURCE_SHA256,
        SDPOSE_MODEL_DETECTION_SOURCE_SHA256,
    ] {
        digest.update(
            u64::try_from(field.len())
                .map_err(|_| SdPoseModelError::Overflow("SDPose model digest"))?
                .to_le_bytes(),
        );
        digest.update(field.as_bytes());
    }
    cancellation.check()?;
    Ok(format!("{:x}", digest.finalize()))
}

fn hash_sdpose_weight_map(
    digest: &mut Sha256,
    weights: &BTreeMap<String, Tensor>,
    cancellation: &CancellationToken,
) -> Result<(), SdPoseModelError> {
    for (index, (key, tensor)) in weights.iter().enumerate() {
        if index.is_multiple_of(8) {
            cancellation.check()?;
        }
        digest.update(
            u64::try_from(key.len())
                .map_err(|_| SdPoseModelError::Overflow("heatmap key digest"))?
                .to_le_bytes(),
        );
        digest.update(key.as_bytes());
        digest.update(
            u64::try_from(tensor.descriptor().shape().len())
                .map_err(|_| SdPoseModelError::Overflow("heatmap shape digest"))?
                .to_le_bytes(),
        );
        for dimension in tensor.descriptor().shape() {
            digest.update(dimension.to_le_bytes());
        }
        digest.update([sdpose_sd2_dtype_tag(tensor.descriptor().dtype())?]);
        let bytes = tensor.contiguous_bytes()?;
        digest.update(
            u64::try_from(bytes.len())
                .map_err(|_| SdPoseModelError::Overflow("heatmap bytes digest"))?
                .to_le_bytes(),
        );
        digest.update(bytes);
    }
    cancellation.check()?;
    Ok(())
}

fn require_finite_heatmap_tensor(
    tensor: &Tensor,
    cancellation: &CancellationToken,
) -> Result<(), SdPoseModelError> {
    let count = tensor.descriptor().element_count()?;
    for index in 0..count {
        if index.is_multiple_of(1_024) {
            cancellation.check()?;
        }
        let value = tensor
            .descriptor()
            .dtype()
            .decode_scalar(tensor.linear_element_bytes(index)?)?;
        if matches!(value, DecodedScalar::Real(value) if !value.is_finite())
            || matches!(value, DecodedScalar::Complex { real, imaginary } if !real.is_finite() || !imaginary.is_finite())
        {
            return Err(SdPoseModelError::NonFinite);
        }
    }
    Ok(())
}

fn sdpose_sd2_semantic_digest(
    configuration: &SdPoseSd2Configuration,
    weights: &BTreeMap<String, Tensor>,
    cancellation: &CancellationToken,
) -> Result<String, SdPoseSd2Error> {
    cancellation.check()?;
    let mut digest = Sha256::new();
    digest.update(SDPOSE_SD2_SOURCE_DOMAIN);
    for source in [
        OPENAI_MODEL_SOURCE_SHA256,
        ATTENTION_SOURCE_SHA256,
        MODEL_BASE_SOURCE_SHA256,
        SUPPORTED_MODELS_SOURCE_SHA256,
    ] {
        digest.update(
            u64::try_from(source.len())
                .map_err(|_| SdPoseSd2Error::Overflow("source digest length"))?
                .to_le_bytes(),
        );
        digest.update(source.as_bytes());
    }
    digest.update([u8::from(configuration.source_exact_profile)]);
    for value in [
        configuration.model_channels,
        configuration.context_dimension,
        configuration.attention_head_channels,
        configuration.normalization_groups,
        configuration.latent_height,
        configuration.latent_width,
    ] {
        digest.update(
            u64::try_from(value)
                .map_err(|_| SdPoseSd2Error::Overflow("configuration digest"))?
                .to_le_bytes(),
        );
    }
    for (index, (key, tensor)) in weights.iter().enumerate() {
        if index.is_multiple_of(8) {
            cancellation.check()?;
        }
        digest.update(
            u64::try_from(key.len())
                .map_err(|_| SdPoseSd2Error::Overflow("weight key digest"))?
                .to_le_bytes(),
        );
        digest.update(key.as_bytes());
        digest.update(
            u64::try_from(tensor.descriptor().shape().len())
                .map_err(|_| SdPoseSd2Error::Overflow("weight shape digest"))?
                .to_le_bytes(),
        );
        for dimension in tensor.descriptor().shape() {
            digest.update(dimension.to_le_bytes());
        }
        digest.update([sdpose_sd2_dtype_tag(tensor.descriptor().dtype())?]);
        let bytes = tensor.contiguous_bytes()?;
        digest.update(
            u64::try_from(bytes.len())
                .map_err(|_| SdPoseSd2Error::Overflow("weight bytes digest"))?
                .to_le_bytes(),
        );
        digest.update(bytes);
    }
    cancellation.check()?;
    Ok(format!("{:x}", digest.finalize()))
}

fn is_sd2_unet_key(key: &str) -> bool {
    [
        "native.input_blocks.",
        "native.time_embed.",
        "native.label_emb.",
        "native.middle_block.",
        "native.output_blocks.",
        "native.out.",
    ]
    .iter()
    .any(|prefix| key.starts_with(prefix))
}

fn sdpose_sd2_dtype_tag(dtype: DType) -> Result<u8, SdPoseSd2Error> {
    match dtype {
        DType::F32 => Ok(1),
        DType::F16 => Ok(2),
        DType::Bf16 => Ok(3),
        _ => Err(SdPoseSd2Error::InvalidConfiguration),
    }
}

fn require_finite_tensor(
    tensor: &Tensor,
    cancellation: &CancellationToken,
) -> Result<(), SdPoseSd2Error> {
    let count = tensor.descriptor().element_count()?;
    for index in 0..count {
        if index.is_multiple_of(1_024) {
            cancellation.check()?;
        }
        let value = tensor
            .descriptor()
            .dtype()
            .decode_scalar(tensor.linear_element_bytes(index)?)?;
        let finite = match value {
            DecodedScalar::Real(value) => value.is_finite(),
            DecodedScalar::Signed(_) | DecodedScalar::Unsigned(_) | DecodedScalar::Boolean(_) => {
                true
            }
            DecodedScalar::Complex { real, imaginary } => real.is_finite() && imaginary.is_finite(),
        };
        if !finite {
            return Err(SdPoseSd2Error::NonFinite);
        }
    }
    Ok(())
}

fn require_forward_inputs(
    configuration: &SdPoseSd2Configuration,
    dtype: DType,
    stream: StreamId,
    latent: &Tensor,
    timesteps: &[f32],
    conditioning: &Tensor,
    adm: &Tensor,
) -> Result<usize, SdPoseSd2Error> {
    for tensor in [latent, conditioning, adm] {
        if tensor.descriptor().dtype() != dtype
            || tensor.descriptor().device() != DeviceId::CPU
            || tensor.descriptor().stream() != stream
        {
            return Err(SdPoseSd2Error::StreamMismatch);
        }
    }
    let latent_shape = require_rank_four(latent, "latent")?;
    let conditioning_shape = require_rank_three(conditioning, "conditioning")?;
    let adm_shape = adm.descriptor().shape();
    let batch =
        usize::try_from(latent_shape[0]).map_err(|_| SdPoseSd2Error::Overflow("input batch"))?;
    let expected_height = u64::try_from(configuration.latent_height)
        .map_err(|_| SdPoseSd2Error::Overflow("latent height"))?;
    let expected_width = u64::try_from(configuration.latent_width)
        .map_err(|_| SdPoseSd2Error::Overflow("latent width"))?;
    let expected_context = u64::try_from(configuration.context_dimension)
        .map_err(|_| SdPoseSd2Error::Overflow("context dimension"))?;
    if batch == 0
        || latent_shape[1..] != [4, expected_height, expected_width]
        || timesteps.len() != batch
        || timesteps.iter().any(|value| !value.is_finite())
        || conditioning_shape[0] != latent_shape[0]
        || conditioning_shape[1] == 0
        || conditioning_shape[2] != expected_context
        || adm_shape != [latent_shape[0], 4]
    {
        return Err(SdPoseSd2Error::InputShape {
            name: "forward request",
            actual: latent.descriptor().shape().to_vec(),
        });
    }
    Ok(batch)
}

fn require_rank_four(tensor: &Tensor, name: &'static str) -> Result<[u64; 4], SdPoseSd2Error> {
    tensor
        .descriptor()
        .shape()
        .try_into()
        .map_err(|_| SdPoseSd2Error::InputShape {
            name,
            actual: tensor.descriptor().shape().to_vec(),
        })
}

fn require_rank_three(tensor: &Tensor, name: &'static str) -> Result<[u64; 3], SdPoseSd2Error> {
    tensor
        .descriptor()
        .shape()
        .try_into()
        .map_err(|_| SdPoseSd2Error::InputShape {
            name,
            actual: tensor.descriptor().shape().to_vec(),
        })
}

fn require_same_target(left: &Tensor, right: &Tensor) -> Result<(), SdPoseSd2Error> {
    if left.descriptor().dtype() != right.descriptor().dtype()
        || left.descriptor().device() != right.descriptor().device()
        || left.descriptor().stream() != right.descriptor().stream()
    {
        return Err(SdPoseSd2Error::StreamMismatch);
    }
    Ok(())
}

fn add_tensors(
    backend: &CpuBackend,
    left: &Tensor,
    right: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, SdPoseSd2Error> {
    require_same_target(left, right)?;
    if left.descriptor().shape() != right.descriptor().shape() {
        return Err(SdPoseSd2Error::InputShape {
            name: "tensor add",
            actual: right.descriptor().shape().to_vec(),
        });
    }
    let left_values = tensor_to_values(backend, left, context)?;
    let right_values = tensor_to_values(backend, right, context)?;
    let mut output = backend.workspace_vec(context, left_values.len())?;
    for (index, (left_value, right_value)) in
        left_values.iter().zip(right_values.iter()).enumerate()
    {
        if index.is_multiple_of(1_024) {
            context.cancellation.check()?;
        }
        output.try_push(left_value + right_value)?;
    }
    Ok(tensor_from_values(
        backend,
        left.descriptor().shape(),
        &output,
        left.descriptor().dtype(),
        left.descriptor().device(),
        context,
    )?)
}

fn concat_channel_tensors(
    backend: &CpuBackend,
    left: &Tensor,
    right: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, SdPoseSd2Error> {
    require_same_target(left, right)?;
    let left_shape = require_rank_four(left, "concat left")?;
    let right_shape = require_rank_four(right, "concat right")?;
    if left_shape[0] != right_shape[0] || left_shape[2..] != right_shape[2..] {
        return Err(SdPoseSd2Error::InputShape {
            name: "concat right",
            actual: right_shape.to_vec(),
        });
    }
    let batch =
        usize::try_from(left_shape[0]).map_err(|_| SdPoseSd2Error::Overflow("concat batch"))?;
    let left_channels = usize::try_from(left_shape[1])
        .map_err(|_| SdPoseSd2Error::Overflow("concat left channels"))?;
    let right_channels = usize::try_from(right_shape[1])
        .map_err(|_| SdPoseSd2Error::Overflow("concat right channels"))?;
    let spatial = usize::try_from(left_shape[2])
        .map_err(|_| SdPoseSd2Error::Overflow("concat height"))?
        .checked_mul(
            usize::try_from(left_shape[3]).map_err(|_| SdPoseSd2Error::Overflow("concat width"))?,
        )
        .ok_or(SdPoseSd2Error::Overflow("concat spatial size"))?;
    let left_values = tensor_to_values(backend, left, context)?;
    let right_values = tensor_to_values(backend, right, context)?;
    let output_channels = left_channels
        .checked_add(right_channels)
        .ok_or(SdPoseSd2Error::Overflow("concat output channels"))?;
    let output_count = batch
        .checked_mul(output_channels)
        .and_then(|value| value.checked_mul(spatial))
        .ok_or(SdPoseSd2Error::Overflow("concat output"))?;
    let mut output = backend.workspace_vec(context, output_count)?;
    for batch_index in 0..batch {
        context.cancellation.check()?;
        let left_start = batch_index * left_channels * spatial;
        for value in &left_values[left_start..left_start + left_channels * spatial] {
            output.try_push(*value)?;
        }
        let right_start = batch_index * right_channels * spatial;
        for value in &right_values[right_start..right_start + right_channels * spatial] {
            output.try_push(*value)?;
        }
    }
    Ok(tensor_from_values(
        backend,
        &[
            left_shape[0],
            u64::try_from(output_channels)
                .map_err(|_| SdPoseSd2Error::Overflow("concat output channels"))?,
            left_shape[2],
            left_shape[3],
        ],
        &output,
        left.descriptor().dtype(),
        left.descriptor().device(),
        context,
    )?)
}

fn nearest_upsample_tensor_2x(
    backend: &CpuBackend,
    input: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, SdPoseSd2Error> {
    let [batch, channels, height, width] = require_rank_four(input, "nearest upsample")?;
    let batch = usize::try_from(batch).map_err(|_| SdPoseSd2Error::Overflow("upsample batch"))?;
    let channels =
        usize::try_from(channels).map_err(|_| SdPoseSd2Error::Overflow("upsample channels"))?;
    let height =
        usize::try_from(height).map_err(|_| SdPoseSd2Error::Overflow("upsample height"))?;
    let width = usize::try_from(width).map_err(|_| SdPoseSd2Error::Overflow("upsample width"))?;
    let output_height = height
        .checked_mul(2)
        .ok_or(SdPoseSd2Error::Overflow("upsample height"))?;
    let output_width = width
        .checked_mul(2)
        .ok_or(SdPoseSd2Error::Overflow("upsample width"))?;
    let source = tensor_to_values(backend, input, context)?;
    let output_count = batch
        .checked_mul(channels)
        .and_then(|value| value.checked_mul(output_height))
        .and_then(|value| value.checked_mul(output_width))
        .ok_or(SdPoseSd2Error::Overflow("upsample output"))?;
    let mut output = backend.workspace_vec(context, output_count)?;
    for batch_index in 0..batch {
        for channel in 0..channels {
            context.cancellation.check()?;
            let source_offset = (batch_index * channels + channel) * height * width;
            for output_y in 0..output_height {
                let source_y = output_y / 2;
                for output_x in 0..output_width {
                    output.try_push(source[source_offset + source_y * width + output_x / 2])?;
                }
            }
        }
    }
    Ok(tensor_from_values(
        backend,
        &[
            u64::try_from(batch).map_err(|_| SdPoseSd2Error::Overflow("upsample batch"))?,
            u64::try_from(channels).map_err(|_| SdPoseSd2Error::Overflow("upsample channels"))?,
            u64::try_from(output_height)
                .map_err(|_| SdPoseSd2Error::Overflow("upsample height"))?,
            u64::try_from(output_width).map_err(|_| SdPoseSd2Error::Overflow("upsample width"))?,
        ],
        &output,
        input.descriptor().dtype(),
        input.descriptor().device(),
        context,
    )?)
}

fn build_timestep_embedding(
    backend: &CpuBackend,
    timesteps: &[f32],
    width: usize,
    dtype: DType,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, SdPoseSd2Error> {
    let half = width / 2;
    if half == 0 || width != half * 2 {
        return Err(SdPoseSd2Error::InvalidConfiguration);
    }
    let count = timesteps
        .len()
        .checked_mul(width)
        .ok_or(SdPoseSd2Error::Overflow("timestep embedding"))?;
    let mut values = backend.workspace_vec(context, count)?;
    for timestep in timesteps {
        context.check()?;
        for index in 0..half {
            let frequency = (-10_000_f32.ln() * index as f32 / half as f32).exp();
            values.try_push((timestep * frequency).cos())?;
        }
        for index in 0..half {
            let frequency = (-10_000_f32.ln() * index as f32 / half as f32).exp();
            values.try_push((timestep * frequency).sin())?;
        }
    }
    Ok(tensor_from_values(
        backend,
        &[
            u64::try_from(timesteps.len())
                .map_err(|_| SdPoseSd2Error::Overflow("timestep batch"))?,
            u64::try_from(width).map_err(|_| SdPoseSd2Error::Overflow("timestep width"))?,
        ],
        &values,
        dtype,
        DeviceId::CPU,
        context,
    )?)
}

fn copy_tensor(
    backend: &CpuBackend,
    input: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, SdPoseSd2Error> {
    context.check()?;
    let values = tensor_to_values(backend, input, context)?;
    let output = tensor_from_values(
        backend,
        input.descriptor().shape(),
        &values,
        input.descriptor().dtype(),
        input.descriptor().device(),
        context,
    )?;
    context.check()?;
    Ok(output)
}

fn immutable_silu(
    backend: &CpuBackend,
    input: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, SdPoseSd2Error> {
    let module = NativeModule::silu("sdpose.silu")?;
    Ok(module.forward_dense_inference_with_context(backend, input, context)?)
}

fn add_embedding_bias(
    backend: &CpuBackend,
    input: &Tensor,
    embedding: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, SdPoseSd2Error> {
    let [batch, channels, height, width] = require_rank_four(input, "embedding input")?;
    if embedding.descriptor().shape() != [batch, channels] {
        return Err(SdPoseSd2Error::InputShape {
            name: "embedding bias",
            actual: embedding.descriptor().shape().to_vec(),
        });
    }
    let batch = usize::try_from(batch).map_err(|_| SdPoseSd2Error::Overflow("embedding batch"))?;
    let channels =
        usize::try_from(channels).map_err(|_| SdPoseSd2Error::Overflow("embedding channels"))?;
    let spatial = usize::try_from(height)
        .map_err(|_| SdPoseSd2Error::Overflow("embedding height"))?
        .checked_mul(
            usize::try_from(width).map_err(|_| SdPoseSd2Error::Overflow("embedding width"))?,
        )
        .ok_or(SdPoseSd2Error::Overflow("embedding spatial size"))?;
    let input_values = tensor_to_values(backend, input, context)?;
    let embedding_values = tensor_to_values(backend, embedding, context)?;
    let mut output = backend.workspace_vec(context, input_values.len())?;
    for batch_index in 0..batch {
        context.check()?;
        for channel in 0..channels {
            let bias = embedding_values[batch_index * channels + channel];
            let offset = (batch_index * channels + channel) * spatial;
            for value in &input_values[offset..offset + spatial] {
                output.try_push(*value + bias)?;
            }
        }
    }
    Ok(tensor_from_values(
        backend,
        input.descriptor().shape(),
        &output,
        input.descriptor().dtype(),
        input.descriptor().device(),
        context,
    )?)
}

fn nchw_to_tokens(
    backend: &CpuBackend,
    input: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, SdPoseSd2Error> {
    let [batch, channels, height, width] = require_rank_four(input, "NCHW tokens")?;
    let batch = usize::try_from(batch).map_err(|_| SdPoseSd2Error::Overflow("token batch"))?;
    let channels =
        usize::try_from(channels).map_err(|_| SdPoseSd2Error::Overflow("token channels"))?;
    let height = usize::try_from(height).map_err(|_| SdPoseSd2Error::Overflow("token height"))?;
    let width = usize::try_from(width).map_err(|_| SdPoseSd2Error::Overflow("token width"))?;
    let source = tensor_to_values(backend, input, context)?;
    let mut output = backend.workspace_vec(context, source.len())?;
    for _ in 0..source.len() {
        output.try_push(0.0)?;
    }
    for batch_index in 0..batch {
        context.check()?;
        for y in 0..height {
            for x in 0..width {
                for channel in 0..channels {
                    let source_index =
                        ((batch_index * channels + channel) * height + y) * width + x;
                    let output_index =
                        ((batch_index * height * width + y * width + x) * channels) + channel;
                    output[output_index] = source[source_index];
                }
            }
        }
    }
    Ok(tensor_from_values(
        backend,
        &[
            u64::try_from(batch).map_err(|_| SdPoseSd2Error::Overflow("token batch"))?,
            u64::try_from(
                height
                    .checked_mul(width)
                    .ok_or(SdPoseSd2Error::Overflow("token count"))?,
            )
            .map_err(|_| SdPoseSd2Error::Overflow("token count"))?,
            u64::try_from(channels).map_err(|_| SdPoseSd2Error::Overflow("token channels"))?,
        ],
        &output,
        input.descriptor().dtype(),
        input.descriptor().device(),
        context,
    )?)
}

fn tokens_to_nchw(
    backend: &CpuBackend,
    input: &Tensor,
    height: u64,
    width: u64,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, SdPoseSd2Error> {
    let [batch, tokens, channels] = require_rank_three(input, "tokens NCHW")?;
    if tokens
        != height
            .checked_mul(width)
            .ok_or(SdPoseSd2Error::Overflow("token geometry"))?
    {
        return Err(SdPoseSd2Error::InputShape {
            name: "tokens NCHW",
            actual: input.descriptor().shape().to_vec(),
        });
    }
    let batch = usize::try_from(batch).map_err(|_| SdPoseSd2Error::Overflow("token batch"))?;
    let channels =
        usize::try_from(channels).map_err(|_| SdPoseSd2Error::Overflow("token channels"))?;
    let height = usize::try_from(height).map_err(|_| SdPoseSd2Error::Overflow("token height"))?;
    let width = usize::try_from(width).map_err(|_| SdPoseSd2Error::Overflow("token width"))?;
    let source = tensor_to_values(backend, input, context)?;
    let mut output = backend.workspace_vec(context, source.len())?;
    for _ in 0..source.len() {
        output.try_push(0.0)?;
    }
    for batch_index in 0..batch {
        context.check()?;
        for y in 0..height {
            for x in 0..width {
                for channel in 0..channels {
                    let source_index =
                        ((batch_index * height * width + y * width + x) * channels) + channel;
                    let output_index =
                        ((batch_index * channels + channel) * height + y) * width + x;
                    output[output_index] = source[source_index];
                }
            }
        }
    }
    Ok(tensor_from_values(
        backend,
        &[
            u64::try_from(batch).map_err(|_| SdPoseSd2Error::Overflow("token batch"))?,
            u64::try_from(channels).map_err(|_| SdPoseSd2Error::Overflow("token channels"))?,
            u64::try_from(height).map_err(|_| SdPoseSd2Error::Overflow("token height"))?,
            u64::try_from(width).map_err(|_| SdPoseSd2Error::Overflow("token width"))?,
        ],
        &output,
        input.descriptor().dtype(),
        input.descriptor().device(),
        context,
    )?)
}

fn geglu(
    backend: &CpuBackend,
    input: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, SdPoseSd2Error> {
    let shape = input.descriptor().shape();
    let width = usize::try_from(*shape.last().ok_or(SdPoseSd2Error::InputShape {
        name: "GEGLU",
        actual: shape.to_vec(),
    })?)
    .map_err(|_| SdPoseSd2Error::Overflow("GEGLU width"))?;
    if width == 0 || !width.is_multiple_of(2) {
        return Err(SdPoseSd2Error::InputShape {
            name: "GEGLU",
            actual: shape.to_vec(),
        });
    }
    let half = width / 2;
    let source = tensor_to_values(backend, input, context)?;
    let rows = source.len() / width;
    let output_count = rows
        .checked_mul(half)
        .ok_or(SdPoseSd2Error::Overflow("GEGLU output"))?;
    let mut left = backend.workspace_vec(context, output_count)?;
    let mut gate = backend.workspace_vec(context, output_count)?;
    for row in source.chunks_exact(width) {
        context.check()?;
        for value in &row[..half] {
            left.try_push(*value)?;
        }
        for value in &row[half..] {
            gate.try_push(*value)?;
        }
    }
    let mut output_shape = shape.to_vec();
    *output_shape.last_mut().ok_or(SdPoseSd2Error::InputShape {
        name: "GEGLU",
        actual: shape.to_vec(),
    })? = u64::try_from(half).map_err(|_| SdPoseSd2Error::Overflow("GEGLU width"))?;
    let gate = tensor_from_values(
        backend,
        &output_shape,
        &gate,
        input.descriptor().dtype(),
        input.descriptor().device(),
        context,
    )?;
    let gelu = NativeModule::gelu("sdpose.geglu", GeluApproximation::None)?;
    let gate = gelu.forward_dense_inference_with_context(backend, &gate, context)?;
    let gate = tensor_to_values(backend, &gate, context)?;
    let mut output = backend.workspace_vec(context, output_count)?;
    for (index, (left, gate)) in left.iter().zip(gate.iter()).enumerate() {
        if index.is_multiple_of(1_024) {
            context.check()?;
        }
        output.try_push(left * gate)?;
    }
    Ok(tensor_from_values(
        backend,
        &output_shape,
        &output,
        input.descriptor().dtype(),
        input.descriptor().device(),
        context,
    )?)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SdPoseRawKeypoint {
    x: f32,
    y: f32,
    score: f32,
}

impl SdPoseRawKeypoint {
    pub fn checked(x: f32, y: f32, score: f32) -> Result<Self, SdPoseProjectionError> {
        if !x.is_finite() || !y.is_finite() || !score.is_finite() {
            return Err(SdPoseProjectionError::NonFiniteInput);
        }
        Ok(Self { x, y, score })
    }

    pub const fn x(self) -> f32 {
        self.x
    }

    pub const fn y(self) -> f32 {
        self.y
    }

    pub const fn score(self) -> f32 {
        self.score
    }
}

#[derive(Debug, Error)]
pub enum SdPoseProjectionError {
    #[error("SDPose heatmaps must have shape [batch, 133, 256, 192]")]
    InvalidHeatmapShape,
    #[error("SDPose projection received a non-finite value")]
    NonFiniteInput,
    #[error("SDPose DARK refinement encountered a singular Hessian")]
    SingularHessian,
    #[error("SDPose projection allocation failed: {0}")]
    AllocationFailed(String),
    #[error(transparent)]
    Cancellation(#[from] CancellationError),
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    Media(#[from] comfy_media::NativeMediaPayloadError),
}

pub fn decode_sdpose_heatmaps(
    heatmaps: &[f32],
    batch_size: usize,
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<Vec<Vec<SdPoseRawKeypoint>>, SdPoseProjectionError> {
    context.check()?;
    let plane_length = SDPOSE_HEATMAP_HEIGHT
        .checked_mul(SDPOSE_HEATMAP_WIDTH)
        .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
    let expected = batch_size
        .checked_mul(SDPOSE_HEATMAP_CHANNELS)
        .and_then(|value| value.checked_mul(plane_length))
        .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
    if batch_size == 0 || heatmaps.len() != expected {
        return Err(SdPoseProjectionError::InvalidHeatmapShape);
    }
    if heatmaps.iter().any(|value| !value.is_finite()) {
        return Err(SdPoseProjectionError::NonFiniteInput);
    }

    let mut batches = Vec::new();
    batches
        .try_reserve_exact(batch_size)
        .map_err(|error| SdPoseProjectionError::AllocationFailed(error.to_string()))?;
    for batch_index in 0..batch_size {
        context.check()?;
        let mut points = Vec::new();
        points
            .try_reserve_exact(SDPOSE_HEATMAP_CHANNELS)
            .map_err(|error| SdPoseProjectionError::AllocationFailed(error.to_string()))?;
        for channel in 0..SDPOSE_HEATMAP_CHANNELS {
            context.check()?;
            let plane_index = batch_index
                .checked_mul(SDPOSE_HEATMAP_CHANNELS)
                .and_then(|value| value.checked_add(channel))
                .and_then(|value| value.checked_mul(plane_length))
                .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
            let plane_end = plane_index
                .checked_add(plane_length)
                .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
            let plane = heatmaps
                .get(plane_index..plane_end)
                .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
            points.push(decode_plane(plane, backend, context)?);
        }
        batches.push(points);
    }
    context.check()?;
    Ok(batches)
}

fn decode_plane(
    plane: &[f32],
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<SdPoseRawKeypoint, SdPoseProjectionError> {
    let (maximum_index, score) = plane.iter().copied().enumerate().fold(
        (0usize, f32::NEG_INFINITY),
        |current, candidate| {
            if candidate.1 > current.1 {
                candidate
            } else {
                current
            }
        },
    );
    let invalid = score <= 0.0;

    let maximum_y = maximum_index / SDPOSE_HEATMAP_WIDTH;
    let maximum_x = maximum_index % SDPOSE_HEATMAP_WIDTH;
    let radius =
        usize::try_from(GAUSSIAN_RADIUS).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?;
    let padded_height = SDPOSE_HEATMAP_HEIGHT + 2 * radius;
    let padded_width = SDPOSE_HEATMAP_WIDTH + 2 * radius;
    let padded_length = padded_height
        .checked_mul(padded_width)
        .ok_or(SdPoseProjectionError::InvalidHeatmapShape)?;
    let mut horizontal = backend.workspace_vec::<f32>(context, padded_length)?;
    let mut blurred = backend.workspace_vec::<f32>(context, padded_length)?;
    for _ in 0..padded_length {
        horizontal.try_push(0.0)?;
        blurred.try_push(0.0)?;
    }

    for y in 0..SDPOSE_HEATMAP_HEIGHT {
        context.check()?;
        for x in 0..SDPOSE_HEATMAP_WIDTH {
            let source = y * SDPOSE_HEATMAP_WIDTH + x;
            let destination = (y + radius) * padded_width + x + radius;
            blurred[destination] = plane[source];
        }
    }
    let kernel = gaussian_kernel()?;
    for y in 0..padded_height {
        context.check()?;
        for x in 0..padded_width {
            let mut value = 0.0f32;
            for (kernel_index, weight) in kernel.iter().copied().enumerate() {
                let delta = isize::try_from(kernel_index)
                    .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?
                    - GAUSSIAN_RADIUS;
                if let Some(source_x) = x
                    .checked_add_signed(delta)
                    .filter(|value| *value < padded_width)
                {
                    value += blurred[y * padded_width + source_x] * weight;
                }
            }
            horizontal[y * padded_width + x] = value;
        }
    }
    for y in 0..padded_height {
        context.check()?;
        for x in 0..padded_width {
            let mut value = 0.0f32;
            for (kernel_index, weight) in kernel.iter().copied().enumerate() {
                let delta = isize::try_from(kernel_index)
                    .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?
                    - GAUSSIAN_RADIUS;
                if let Some(source_y) = y
                    .checked_add_signed(delta)
                    .filter(|value| *value < padded_height)
                {
                    value += horizontal[source_y * padded_width + x] * weight;
                }
            }
            blurred[y * padded_width + x] = value;
        }
    }

    let mut current_maximum = f32::NEG_INFINITY;
    for y in 0..SDPOSE_HEATMAP_HEIGHT {
        for x in 0..SDPOSE_HEATMAP_WIDTH {
            current_maximum =
                current_maximum.max(blurred[(y + radius) * padded_width + x + radius]);
        }
    }
    if current_maximum > 0.0 {
        let scale = score / current_maximum;
        for y in 0..SDPOSE_HEATMAP_HEIGHT {
            context.check()?;
            for x in 0..SDPOSE_HEATMAP_WIDTH {
                let index = (y + radius) * padded_width + x + radius;
                blurred[index] *= scale;
            }
        }
    }
    for y in 0..SDPOSE_HEATMAP_HEIGHT {
        context.check()?;
        for x in 0..SDPOSE_HEATMAP_WIDTH {
            let index = (y + radius) * padded_width + x + radius;
            blurred[index] = blurred[index].clamp(1.0e-3, 50.0).ln();
        }
    }

    let sample = |x: isize, y: isize| -> Result<f32, SdPoseProjectionError> {
        let clamped_x = x.clamp(
            0,
            isize::try_from(SDPOSE_HEATMAP_WIDTH - 1)
                .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
        );
        let clamped_y = y.clamp(
            0,
            isize::try_from(SDPOSE_HEATMAP_HEIGHT - 1)
                .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
        );
        let source_x = usize::try_from(clamped_x)
            .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?
            + radius;
        let source_y = usize::try_from(clamped_y)
            .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?
            + radius;
        Ok(blurred[source_y * padded_width + source_x])
    };
    let x = isize::try_from(maximum_x).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?;
    let y = isize::try_from(maximum_y).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?;
    let center = sample(x, y)?;
    let right = sample(x + 1, y)?;
    let left = sample(x - 1, y)?;
    let down = sample(x, y + 1)?;
    let up = sample(x, y - 1)?;
    let down_right = sample(x + 1, y + 1)?;
    let up_left = sample(x - 1, y - 1)?;
    let derivative_x = 0.5 * (right - left);
    let derivative_y = 0.5 * (down - up);
    let hessian_xx = right - 2.0 * center + left + f32::EPSILON;
    let hessian_yy = down - 2.0 * center + up + f32::EPSILON;
    let hessian_xy = 0.5 * (down_right - right - down + 2.0 * center - left - up + up_left);
    let correction = checked_hessian_correction(
        hessian_xx,
        hessian_xy,
        hessian_yy,
        derivative_x,
        derivative_y,
    )?;
    let maximum_x = f32::from(
        u16::try_from(maximum_x).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
    );
    let maximum_y = f32::from(
        u16::try_from(maximum_y).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
    );
    let refined_x = maximum_x - correction[0];
    let refined_y = maximum_y - correction[1];
    let heatmap_width = f32::from(
        u16::try_from(SDPOSE_HEATMAP_WIDTH)
            .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
    );
    let heatmap_height = f32::from(
        u16::try_from(SDPOSE_HEATMAP_HEIGHT)
            .map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
    );
    let scale_x = (SDPOSE_INPUT_WIDTH - 1.0) / (heatmap_width - 1.0);
    let scale_y = (SDPOSE_INPUT_HEIGHT - 1.0) / (heatmap_height - 1.0);
    if invalid {
        SdPoseRawKeypoint::checked(-1.0, -1.0, score)
    } else {
        SdPoseRawKeypoint::checked(refined_x * scale_x, refined_y * scale_y, score)
    }
}

fn checked_hessian_correction(
    hessian_xx: f32,
    hessian_xy: f32,
    hessian_yy: f32,
    derivative_x: f32,
    derivative_y: f32,
) -> Result<[f32; 2], SdPoseProjectionError> {
    let determinant = hessian_xx * hessian_yy - hessian_xy * hessian_xy;
    if !determinant.is_finite() || determinant == 0.0 {
        return Err(SdPoseProjectionError::SingularHessian);
    }
    let inverse_xx = hessian_yy / determinant;
    let inverse_xy = -hessian_xy / determinant;
    let inverse_yy = hessian_xx / determinant;
    Ok([
        inverse_xx * derivative_x + inverse_xy * derivative_y,
        inverse_xy * derivative_x + inverse_yy * derivative_y,
    ])
}

fn gaussian_kernel() -> Result<[f32; 11], SdPoseProjectionError> {
    let mut kernel = [0.0; 11];
    let mut total = 0.0f32;
    let radius = f32::from(
        i16::try_from(GAUSSIAN_RADIUS).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
    );
    for (index, value) in kernel.iter_mut().enumerate() {
        let index = f32::from(
            u16::try_from(index).map_err(|_| SdPoseProjectionError::InvalidHeatmapShape)?,
        );
        let distance = index - radius;
        let weight = (-0.5 * (distance / GAUSSIAN_SIGMA).powi(2)).exp();
        *value = weight;
        total += weight;
    }
    for value in &mut kernel {
        *value /= total;
    }
    Ok(kernel)
}

pub fn project_sdpose_openpose_person(
    raw: &[SdPoseRawKeypoint],
) -> Result<NativePosePerson, SdPoseProjectionError> {
    if raw.len() != SDPOSE_HEATMAP_CHANNELS {
        return Err(SdPoseProjectionError::InvalidHeatmapShape);
    }
    let mut points = Vec::new();
    points
        .try_reserve_exact(OPENPOSE_KEYPOINTS)
        .map_err(|error| SdPoseProjectionError::AllocationFailed(error.to_string()))?;
    points.extend_from_slice(&raw[..17]);
    let left_shoulder = raw[5];
    let right_shoulder = raw[6];
    points.push(SdPoseRawKeypoint::checked(
        (left_shoulder.x + right_shoulder.x) * 0.5,
        (left_shoulder.y + right_shoulder.y) * 0.5,
        if left_shoulder.score > 0.3 && right_shoulder.score > 0.3 {
            left_shoulder.score.min(right_shoulder.score)
        } else {
            0.0
        },
    )?);
    points.extend_from_slice(&raw[17..]);
    let original = points.clone();
    for (&source, &destination) in MMPOSE_INDICES.iter().zip(OPENPOSE_INDICES.iter()) {
        points[destination] = original[source];
    }

    let convert = |point: SdPoseRawKeypoint| {
        NativePoseKeypoint::checked(point.x.into(), point.y.into(), point.score.into())
    };
    let collect = |slice: &[SdPoseRawKeypoint]| {
        slice
            .iter()
            .copied()
            .map(convert)
            .collect::<Result<Vec<_>, _>>()
    };
    let mut face = collect(&points[24..92])?;
    face.try_reserve_exact(2)
        .map_err(|error| SdPoseProjectionError::AllocationFailed(error.to_string()))?;
    face.push(convert(points[14])?);
    face.push(convert(points[15])?);
    Ok(NativePosePerson::checked(
        collect(&points[0..18])?,
        collect(&points[18..24])?,
        face,
        collect(&points[92..113])?,
        collect(&points[113..134])?,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singular_hessian_is_typed() {
        assert!(matches!(
            checked_hessian_correction(f32::EPSILON, f32::EPSILON, f32::EPSILON, 1.0, 1.0),
            Err(SdPoseProjectionError::SingularHessian)
        ));
    }
}
