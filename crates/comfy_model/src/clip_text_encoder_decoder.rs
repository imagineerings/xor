use crate::{
    AttentionBackend, AttentionError, AttentionFallbackPolicy, AttentionMask, AttentionMaskShape,
    AttentionRequest, EmbeddingOptions, NativeExecutionRequirements, NativeModule, NativeOpsError,
    NativePromptTokenizer, NativeTokenizedPrompt, scaled_dot_product_attention_with_context,
};
use comfy_tensor::{
    BinaryOperation, CpuBackend, CpuWorkspaceVec, DType, DeviceId, ExecutionContext, Layout,
    LinearAlgebraOperation, OperationSupport, RngError, RngTransaction, StreamId, Tensor,
    TensorError, UnaryOperation,
    generated_activation_normalization_functional_01::{
        FunctionalError, rms_norm_with_context_exact_native,
    },
    generated_native_diffusion::{NativeDiffusionTensorError, add, tensor_from_f32, tensor_to_f32},
};
use comfy_types::CancellationToken;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use thiserror::Error;

pub const LLAMA_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/text_encoders/llama.py";
pub const LLAMA_SOURCE_SHA256: &str =
    "f4adf96f7ff8d320da909038285eebd1b36714123865cb5a8748276f718b345a";
pub const TEXT_GENERATION_SOURCE_PATH: &str =
    "projects/comfy/ComfyUI/comfy_extras/nodes_textgen.py";
pub const TEXT_GENERATION_SOURCE_SHA256: &str =
    "b328e8a2dc89cfd3a93ab49c1be880a3e89ec4521eef506823753617c86c99e9";
pub const GEMMA4_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/text_encoders/gemma4.py";
pub const GEMMA4_SOURCE_SHA256: &str =
    "c6ffbb2fbecd8f97e781a654a06ccf3910dc670867d38c0ce30542312f00cde6";
pub const GPT_OSS_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/text_encoders/gpt_oss.py";
pub const GPT_OSS_SOURCE_SHA256: &str =
    "2bff01e891f6e3c00e13a610006021cbce040f116ffd01d0680476c4a39d66dc";
pub const QWEN35_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/text_encoders/qwen35.py";
pub const QWEN35_SOURCE_SHA256: &str =
    "7cacfe7df0bd72f00e96d73ca9c4c82cf37b02aa03c7df7bcb875f611b826929";

pub const DECODER_TEXT_ENCODER_CATALOG_SYMBOLS: [&str; 127] = [
    "Gemma4Config",
    "Gemma4_E2B_Config",
    "Gemma4_31B_Config",
    "_apply_rotary_pos_emb",
    "Gemma4Attention",
    "TransformerBlockGemma4",
    "Gemma4Transformer",
    "Gemma4Base",
    "Gemma4AudioMixin",
    "_compute_vision_2d_rope",
    "_apply_vision_2d_rope",
    "ClippedLinear",
    "Gemma4VisionMLP",
    "Gemma4VisionAttention",
    "Gemma4VisionLayer",
    "Gemma4PatchEmbedder",
    "Gemma4VisionEncoderLayers",
    "Gemma4VisionEncoder",
    "Gemma4RMSNormProjector",
    "Gemma4MultiModalProjector",
    "Gemma4AudioConvSubsampler",
    "Gemma4AudioFeedForward",
    "Gemma4AudioRelPositionalEncoding",
    "Gemma4AudioAttention",
    "Gemma4AudioLConv1d",
    "Gemma4AudioLayer",
    "Gemma4AudioEncoder",
    "Gemma4AudioProjector",
    "Gemma4_Tokenizer",
    "_Gemma4Tokenizer",
    "Gemma4SDTokenizer",
    "Gemma4Tokenizer",
    "Gemma4Model",
    "gemma4_te",
    "_make_variant",
    "GptOss20BConfig",
    "_yarn_inv_freq",
    "_build_freqs_cis",
    "_attention_with_sinks",
    "GptOssAttention",
    "GptOssTopKRouter",
    "GptOssExperts",
    "GptOssMLP",
    "GptOssDecoderLayer",
    "_make_full_causal_mask",
    "_make_sliding_causal_mask",
    "GptOssModel",
    "_lens_render_chat",
    "_GptOssRawTokenizer",
    "LensGptOssTokenizer",
    "LensTokenizer",
    "LensGptOssClipModel",
    "LensTEModel",
    "lens_te",
    "Llama2Config",
    "Mistral3Small24BConfig",
    "Ministral3_3BConfig",
    "Qwen25_3BConfig",
    "Qwen3_06BConfig",
    "Qwen3_06B_ACE15_Config",
    "Qwen3_2B_ACE15_lm_Config",
    "Qwen3_4B_ACE15_lm_Config",
    "Qwen3_4BConfig",
    "Qwen3_8BConfig",
    "Qwen3VL_8BConfig",
    "Qwen3VL_4BConfig",
    "Ovis25_2BConfig",
    "Qwen25_7BVLI_Config",
    "Gemma2_2B_Config",
    "Gemma3_4B_Config",
    "Gemma3_4B_Vision_Config",
    "Gemma3_12B_Config",
    "RMSNorm",
    "precompute_freqs_cis",
    "apply_rope",
    "Attention",
    "MLP",
    "TransformerBlock",
    "TransformerBlockGemma2",
    "_make_scaled_embedding",
    "Llama2_",
    "Gemma3MultiModalProjector",
    "BaseLlama",
    "BaseGenerate",
    "BaseQwen3",
    "Llama2",
    "Mistral3Small24B",
    "Ministral3_3B",
    "Qwen25_3B",
    "Qwen3_06B",
    "Qwen3_06B_ACE15",
    "Qwen3_2B_ACE15_lm",
    "Qwen3_4B",
    "Qwen3_4B_ACE15_lm",
    "Qwen3_8B",
    "Ovis25_2B",
    "Qwen25_7BVLI",
    "Gemma2_2B",
    "Gemma3_4B",
    "Gemma3_4B_Vision",
    "Gemma3_12B",
    "_qwen35_layer_types",
    "Qwen35Config",
    "_make_config",
    "RMSNormGated",
    "torch_chunk_gated_delta_rule",
    "torch_causal_conv1d_update",
    "GatedDeltaNet",
    "precompute_partial_rope",
    "apply_partial_rope",
    "GatedAttention",
    "Qwen35TransformerBlock",
    "Qwen35Transformer",
    "Qwen35VisionPatchEmbed",
    "Qwen35VisionMLP",
    "Qwen35VisionRotaryEmbedding",
    "Qwen35VisionAttention",
    "Qwen35VisionBlock",
    "Qwen35VisionPatchMerger",
    "Qwen35VisionModel",
    "Qwen35",
    "Qwen35Tokenizer",
    "Qwen35ImageTokenizer",
    "Qwen35ClipModel",
    "Qwen35TEModel",
    "tokenizer",
    "te",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecoderArchitecture {
    Llama,
    Gemma,
    GptOss,
    Qwen35,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecoderLayerKind {
    FullAttention,
    SlidingAttention,
    LinearAttention,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecoderActivation {
    Silu,
    GeluTanh,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecoderVisionProfileFact {
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub layers: usize,
    pub attention_heads: usize,
    pub head_dimension: Option<usize>,
    pub image_size: Option<usize>,
    pub patch_size: usize,
    pub temporal_patch_size: Option<usize>,
    pub spatial_merge_size: Option<usize>,
    pub position_embeddings: Option<usize>,
    pub pooling_kernel_size: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecoderAudioProfileFact {
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub layers: usize,
    pub attention_heads: usize,
    pub convolution_kernel_size: usize,
    pub attention_chunk_size: usize,
    pub context_left: usize,
    pub context_right: usize,
    pub attention_logit_cap_bits: u32,
    pub output_projection_size: usize,
    pub residual_weight_bits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecoderProfileFact {
    pub source_symbol: &'static str,
    pub architecture: DecoderArchitecture,
    pub transformer_type: &'static str,
    pub vocabulary_size: usize,
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub hidden_layers: usize,
    pub attention_heads: usize,
    pub key_value_heads: usize,
    pub head_dimension: usize,
    pub maximum_positions: usize,
    pub normalization_epsilon_bits: u32,
    pub rope_theta_bits: [Option<u32>; 2],
    pub rope_scale_bits: [Option<u32>; 2],
    pub rope_sections: &'static [usize],
    pub interleaved_multidimensional_rope: bool,
    pub sliding_pattern: &'static [usize],
    pub linear_attention_period: Option<usize>,
    pub partial_rotary_factor_bits: u32,
    pub rms_norm_add: bool,
    pub activation: DecoderActivation,
    pub qkv_bias: bool,
    pub query_key_norm: bool,
    pub final_norm: bool,
    pub untied_output_head: bool,
    pub stop_tokens: &'static [i64],
    pub global_head_dimension: Option<usize>,
    pub linear_key_heads: Option<usize>,
    pub linear_value_heads: Option<usize>,
    pub linear_key_head_dimension: Option<usize>,
    pub linear_value_head_dimension: Option<usize>,
    pub convolution_kernel_size: Option<usize>,
    pub local_experts: Option<usize>,
    pub experts_per_token: Option<usize>,
    pub final_logit_soft_cap_bits: Option<u32>,
    pub hidden_size_per_layer_input: usize,
    pub shared_key_value_layers: usize,
    pub double_wide_mlp: bool,
    pub vision: Option<DecoderVisionProfileFact>,
    pub audio: Option<DecoderAudioProfileFact>,
    pub multimodal_tokens_per_image: Option<usize>,
}

impl DecoderProfileFact {
    pub fn normalization_epsilon(&self) -> f32 {
        f32::from_bits(self.normalization_epsilon_bits)
    }

    pub fn rope_theta(&self) -> impl Iterator<Item = f32> + '_ {
        self.rope_theta_bits
            .iter()
            .filter_map(|value| value.map(f32::from_bits))
    }

    pub fn rope_scale(&self) -> impl Iterator<Item = f32> + '_ {
        self.rope_scale_bits
            .iter()
            .filter_map(|value| value.map(f32::from_bits))
    }

    pub fn partial_rotary_factor(&self) -> f32 {
        f32::from_bits(self.partial_rotary_factor_bits)
    }

    pub fn final_logit_soft_cap(&self) -> Option<f32> {
        self.final_logit_soft_cap_bits.map(f32::from_bits)
    }
}

const EMPTY_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "",
    architecture: DecoderArchitecture::Llama,
    transformer_type: "llama",
    vocabulary_size: 0,
    hidden_size: 0,
    intermediate_size: 0,
    hidden_layers: 0,
    attention_heads: 0,
    key_value_heads: 0,
    head_dimension: 0,
    maximum_positions: 0,
    normalization_epsilon_bits: 1.0e-5_f32.to_bits(),
    rope_theta_bits: [None, None],
    rope_scale_bits: [None, None],
    rope_sections: &[],
    interleaved_multidimensional_rope: false,
    sliding_pattern: &[],
    linear_attention_period: None,
    partial_rotary_factor_bits: 1.0_f32.to_bits(),
    rms_norm_add: false,
    activation: DecoderActivation::Silu,
    qkv_bias: false,
    query_key_norm: false,
    final_norm: true,
    untied_output_head: false,
    stop_tokens: &[],
    global_head_dimension: None,
    linear_key_heads: None,
    linear_value_heads: None,
    linear_key_head_dimension: None,
    linear_value_head_dimension: None,
    convolution_kernel_size: None,
    local_experts: None,
    experts_per_token: None,
    final_logit_soft_cap_bits: None,
    hidden_size_per_layer_input: 0,
    shared_key_value_layers: 0,
    double_wide_mlp: false,
    vision: None,
    audio: None,
    multimodal_tokens_per_image: None,
};

const GEMMA3_VISION_PROFILE: DecoderVisionProfileFact = DecoderVisionProfileFact {
    hidden_size: 1152,
    intermediate_size: 4304,
    layers: 27,
    attention_heads: 16,
    head_dimension: None,
    image_size: Some(896),
    patch_size: 14,
    temporal_patch_size: None,
    spatial_merge_size: None,
    position_embeddings: None,
    pooling_kernel_size: None,
};

const GEMMA4_VISION_PROFILE: DecoderVisionProfileFact = DecoderVisionProfileFact {
    hidden_size: 768,
    intermediate_size: 3072,
    layers: 16,
    attention_heads: 12,
    head_dimension: Some(64),
    image_size: Some(896),
    patch_size: 16,
    temporal_patch_size: None,
    spatial_merge_size: None,
    position_embeddings: Some(10_240),
    pooling_kernel_size: Some(3),
};

const GEMMA4_31B_VISION_PROFILE: DecoderVisionProfileFact = DecoderVisionProfileFact {
    hidden_size: 1152,
    intermediate_size: 4304,
    layers: 27,
    attention_heads: 16,
    head_dimension: Some(72),
    ..GEMMA4_VISION_PROFILE
};

const GEMMA4_AUDIO_PROFILE: DecoderAudioProfileFact = DecoderAudioProfileFact {
    hidden_size: 1024,
    intermediate_size: 4096,
    layers: 12,
    attention_heads: 8,
    convolution_kernel_size: 5,
    attention_chunk_size: 12,
    context_left: 13,
    context_right: 0,
    attention_logit_cap_bits: 50.0_f32.to_bits(),
    output_projection_size: 1536,
    residual_weight_bits: 0.5_f32.to_bits(),
};

const QWEN35_VISION_PROFILE: DecoderVisionProfileFact = DecoderVisionProfileFact {
    hidden_size: 1024,
    intermediate_size: 4096,
    layers: 24,
    attention_heads: 16,
    head_dimension: None,
    image_size: None,
    patch_size: 16,
    temporal_patch_size: Some(2),
    spatial_merge_size: Some(2),
    position_embeddings: Some(2304),
    pooling_kernel_size: None,
};

const LLAMA2_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Llama2Config",
    vocabulary_size: 128_320,
    hidden_size: 4096,
    intermediate_size: 14_336,
    hidden_layers: 32,
    attention_heads: 32,
    key_value_heads: 8,
    head_dimension: 128,
    maximum_positions: 8192,
    rope_theta_bits: [Some(500_000.0_f32.to_bits()), None],
    ..EMPTY_PROFILE
};

const MISTRAL3_SMALL_24B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Mistral3Small24BConfig",
    vocabulary_size: 131_072,
    hidden_size: 5120,
    intermediate_size: 32_768,
    hidden_layers: 40,
    attention_heads: 32,
    key_value_heads: 8,
    head_dimension: 128,
    maximum_positions: 8192,
    rope_theta_bits: [Some(1_000_000_000.0_f32.to_bits()), None],
    ..EMPTY_PROFILE
};

const MINISTRAL3_3B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Ministral3_3BConfig",
    vocabulary_size: 131_072,
    hidden_size: 3072,
    intermediate_size: 9216,
    hidden_layers: 26,
    attention_heads: 32,
    key_value_heads: 8,
    head_dimension: 128,
    maximum_positions: 262_144,
    rope_theta_bits: [Some(1_000_000.0_f32.to_bits()), None],
    stop_tokens: &[2],
    ..EMPTY_PROFILE
};

const QWEN25_3B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen25_3BConfig",
    vocabulary_size: 151_936,
    hidden_size: 2048,
    intermediate_size: 11_008,
    hidden_layers: 36,
    attention_heads: 16,
    key_value_heads: 2,
    head_dimension: 128,
    maximum_positions: 128_000,
    normalization_epsilon_bits: 1.0e-6_f32.to_bits(),
    rope_theta_bits: [Some(1_000_000.0_f32.to_bits()), None],
    qkv_bias: true,
    ..EMPTY_PROFILE
};

const QWEN3_06B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen3_06BConfig",
    vocabulary_size: 151_936,
    hidden_size: 1024,
    intermediate_size: 3072,
    hidden_layers: 28,
    attention_heads: 16,
    key_value_heads: 8,
    head_dimension: 128,
    maximum_positions: 32_768,
    normalization_epsilon_bits: 1.0e-6_f32.to_bits(),
    rope_theta_bits: [Some(1_000_000.0_f32.to_bits()), None],
    query_key_norm: true,
    stop_tokens: &[151_643, 151_645],
    ..EMPTY_PROFILE
};

const QWEN3_06B_ACE15_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen3_06B_ACE15_Config",
    vocabulary_size: 151_669,
    ..QWEN3_06B_PROFILE
};

const QWEN3_2B_ACE15_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen3_2B_ACE15_lm_Config",
    vocabulary_size: 217_204,
    hidden_size: 2048,
    intermediate_size: 6144,
    maximum_positions: 40_960,
    ..QWEN3_06B_PROFILE
};

const QWEN3_4B_ACE15_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen3_4B_ACE15_lm_Config",
    vocabulary_size: 217_204,
    hidden_size: 2560,
    intermediate_size: 9728,
    hidden_layers: 36,
    attention_heads: 32,
    maximum_positions: 40_960,
    ..QWEN3_06B_PROFILE
};

const QWEN3_4B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen3_4BConfig",
    vocabulary_size: 151_936,
    ..QWEN3_4B_ACE15_PROFILE
};

const QWEN3_8B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen3_8BConfig",
    hidden_size: 4096,
    intermediate_size: 12_288,
    untied_output_head: true,
    ..QWEN3_4B_PROFILE
};

const QWEN3_VL_8B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen3VL_8BConfig",
    maximum_positions: 262_144,
    rope_theta_bits: [Some(5_000_000.0_f32.to_bits()), None],
    rope_sections: &[24, 20, 20],
    interleaved_multidimensional_rope: true,
    ..QWEN3_8B_PROFILE
};

const QWEN3_VL_4B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen3VL_4BConfig",
    hidden_size: 2560,
    intermediate_size: 9728,
    untied_output_head: false,
    ..QWEN3_VL_8B_PROFILE
};

const OVIS25_2B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Ovis25_2BConfig",
    vocabulary_size: 151_936,
    hidden_size: 2048,
    intermediate_size: 6144,
    hidden_layers: 28,
    attention_heads: 16,
    key_value_heads: 8,
    head_dimension: 128,
    maximum_positions: 40_960,
    normalization_epsilon_bits: 1.0e-6_f32.to_bits(),
    rope_theta_bits: [Some(1_000_000.0_f32.to_bits()), None],
    query_key_norm: true,
    ..EMPTY_PROFILE
};

const QWEN25_7B_VLI_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen25_7BVLI_Config",
    vocabulary_size: 152_064,
    hidden_size: 3584,
    intermediate_size: 18_944,
    hidden_layers: 28,
    attention_heads: 28,
    key_value_heads: 4,
    head_dimension: 128,
    maximum_positions: 128_000,
    normalization_epsilon_bits: 1.0e-6_f32.to_bits(),
    rope_theta_bits: [Some(1_000_000.0_f32.to_bits()), None],
    rope_sections: &[16, 24, 24],
    qkv_bias: true,
    ..EMPTY_PROFILE
};

const GEMMA2_2B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Gemma2_2B_Config",
    architecture: DecoderArchitecture::Gemma,
    transformer_type: "gemma2",
    vocabulary_size: 256_000,
    hidden_size: 2304,
    intermediate_size: 9216,
    hidden_layers: 26,
    attention_heads: 8,
    key_value_heads: 4,
    head_dimension: 256,
    maximum_positions: 8192,
    normalization_epsilon_bits: 1.0e-6_f32.to_bits(),
    rope_theta_bits: [Some(10_000.0_f32.to_bits()), None],
    rms_norm_add: true,
    activation: DecoderActivation::GeluTanh,
    stop_tokens: &[1],
    ..EMPTY_PROFILE
};

const GEMMA3_4B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Gemma3_4B_Config",
    architecture: DecoderArchitecture::Gemma,
    transformer_type: "gemma3",
    vocabulary_size: 262_208,
    hidden_size: 2560,
    intermediate_size: 10_240,
    hidden_layers: 34,
    attention_heads: 8,
    key_value_heads: 4,
    head_dimension: 256,
    maximum_positions: 131_072,
    normalization_epsilon_bits: 1.0e-6_f32.to_bits(),
    rope_theta_bits: [
        Some(1_000_000.0_f32.to_bits()),
        Some(10_000.0_f32.to_bits()),
    ],
    rope_scale_bits: [Some(8.0_f32.to_bits()), Some(1.0_f32.to_bits())],
    sliding_pattern: &[1024, 1024, 1024, 1024, 1024, 0],
    rms_norm_add: true,
    activation: DecoderActivation::GeluTanh,
    query_key_norm: true,
    stop_tokens: &[1, 106],
    ..EMPTY_PROFILE
};

const GEMMA3_4B_VISION_PROFILE_FACT: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Gemma3_4B_Vision_Config",
    vision: Some(GEMMA3_VISION_PROFILE),
    multimodal_tokens_per_image: Some(256),
    ..GEMMA3_4B_PROFILE
};

const GEMMA3_12B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Gemma3_12B_Config",
    hidden_size: 3840,
    intermediate_size: 15_360,
    hidden_layers: 48,
    attention_heads: 16,
    key_value_heads: 8,
    vision: Some(GEMMA3_VISION_PROFILE),
    multimodal_tokens_per_image: Some(256),
    ..GEMMA3_4B_PROFILE
};

const GEMMA4_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Gemma4Config",
    architecture: DecoderArchitecture::Gemma,
    transformer_type: "gemma4",
    vocabulary_size: 262_144,
    hidden_size: 2560,
    intermediate_size: 10_240,
    hidden_layers: 42,
    attention_heads: 8,
    key_value_heads: 2,
    head_dimension: 256,
    maximum_positions: 131_072,
    normalization_epsilon_bits: 1.0e-6_f32.to_bits(),
    rope_theta_bits: [
        Some(1_000_000.0_f32.to_bits()),
        Some(10_000.0_f32.to_bits()),
    ],
    sliding_pattern: &[512, 512, 512, 512, 512, 0],
    partial_rotary_factor_bits: 0.25_f32.to_bits(),
    activation: DecoderActivation::GeluTanh,
    query_key_norm: true,
    stop_tokens: &[1, 50, 106],
    global_head_dimension: Some(512),
    final_logit_soft_cap_bits: Some(30.0_f32.to_bits()),
    hidden_size_per_layer_input: 256,
    shared_key_value_layers: 18,
    vision: Some(GEMMA4_VISION_PROFILE),
    audio: Some(GEMMA4_AUDIO_PROFILE),
    multimodal_tokens_per_image: Some(280),
    ..EMPTY_PROFILE
};

const GEMMA4_E2B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Gemma4_E2B_Config",
    hidden_size: 1536,
    intermediate_size: 6144,
    hidden_layers: 35,
    key_value_heads: 1,
    sliding_pattern: &[512, 512, 512, 512, 0],
    shared_key_value_layers: 20,
    double_wide_mlp: true,
    ..GEMMA4_PROFILE
};

const GEMMA4_31B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Gemma4_31B_Config",
    hidden_size: 5376,
    intermediate_size: 21_504,
    hidden_layers: 60,
    attention_heads: 32,
    key_value_heads: 16,
    sliding_pattern: &[1024, 1024, 1024, 1024, 1024, 0],
    hidden_size_per_layer_input: 0,
    shared_key_value_layers: 0,
    vision: Some(GEMMA4_31B_VISION_PROFILE),
    audio: None,
    ..GEMMA4_PROFILE
};

const GPT_OSS_20B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "GptOss20BConfig",
    architecture: DecoderArchitecture::GptOss,
    transformer_type: "gpt_oss",
    vocabulary_size: 201_088,
    hidden_size: 2880,
    intermediate_size: 2880,
    hidden_layers: 24,
    attention_heads: 64,
    key_value_heads: 8,
    head_dimension: 64,
    maximum_positions: 4096,
    rope_theta_bits: [Some(150_000.0_f32.to_bits()), None],
    rope_scale_bits: [Some(32.0_f32.to_bits()), None],
    sliding_pattern: &[128, 0],
    qkv_bias: true,
    local_experts: Some(32),
    experts_per_token: Some(4),
    ..EMPTY_PROFILE
};

const QWEN35_2B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "Qwen35Config",
    architecture: DecoderArchitecture::Qwen35,
    transformer_type: "qwen35_2b",
    vocabulary_size: 248_320,
    hidden_size: 2048,
    intermediate_size: 6144,
    hidden_layers: 24,
    attention_heads: 8,
    key_value_heads: 2,
    head_dimension: 256,
    maximum_positions: 32_768,
    normalization_epsilon_bits: 1.0e-6_f32.to_bits(),
    rope_theta_bits: [Some(10_000_000.0_f32.to_bits()), None],
    rope_sections: &[11, 11, 10],
    linear_attention_period: Some(4),
    partial_rotary_factor_bits: 0.25_f32.to_bits(),
    rms_norm_add: true,
    query_key_norm: true,
    stop_tokens: &[248_044, 248_046],
    linear_key_heads: Some(16),
    linear_value_heads: Some(16),
    linear_key_head_dimension: Some(128),
    linear_value_head_dimension: Some(128),
    convolution_kernel_size: Some(4),
    vision: Some(QWEN35_VISION_PROFILE),
    ..EMPTY_PROFILE
};

const QWEN35_08B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "_make_config:qwen35_08b",
    transformer_type: "qwen35_08b",
    hidden_size: 1024,
    intermediate_size: 3584,
    vision: Some(DecoderVisionProfileFact {
        hidden_size: 768,
        intermediate_size: 3072,
        layers: 12,
        ..QWEN35_VISION_PROFILE
    }),
    ..QWEN35_2B_PROFILE
};

const QWEN35_4B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "_make_config:qwen35_4b",
    transformer_type: "qwen35_4b",
    hidden_size: 2560,
    intermediate_size: 9216,
    hidden_layers: 32,
    attention_heads: 16,
    key_value_heads: 4,
    linear_value_heads: Some(32),
    ..QWEN35_2B_PROFILE
};

const QWEN35_9B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "_make_config:qwen35_9b",
    transformer_type: "qwen35_9b",
    hidden_size: 4096,
    intermediate_size: 12_288,
    hidden_layers: 32,
    attention_heads: 16,
    key_value_heads: 4,
    linear_value_heads: Some(32),
    untied_output_head: true,
    vision: Some(DecoderVisionProfileFact {
        hidden_size: 1152,
        intermediate_size: 4304,
        layers: 27,
        ..QWEN35_VISION_PROFILE
    }),
    ..QWEN35_2B_PROFILE
};

const QWEN35_27B_PROFILE: DecoderProfileFact = DecoderProfileFact {
    source_symbol: "_make_config:qwen35_27b",
    transformer_type: "qwen35_27b",
    hidden_size: 5120,
    intermediate_size: 17_408,
    hidden_layers: 64,
    attention_heads: 24,
    key_value_heads: 4,
    linear_value_heads: Some(48),
    untied_output_head: true,
    vision: Some(DecoderVisionProfileFact {
        hidden_size: 1152,
        intermediate_size: 4304,
        layers: 27,
        ..QWEN35_VISION_PROFILE
    }),
    ..QWEN35_2B_PROFILE
};

pub const DECODER_PROFILE_FACTS: &[DecoderProfileFact] = &[
    LLAMA2_PROFILE,
    MISTRAL3_SMALL_24B_PROFILE,
    MINISTRAL3_3B_PROFILE,
    QWEN25_3B_PROFILE,
    QWEN3_06B_PROFILE,
    QWEN3_06B_ACE15_PROFILE,
    QWEN3_2B_ACE15_PROFILE,
    QWEN3_4B_ACE15_PROFILE,
    QWEN3_4B_PROFILE,
    QWEN3_8B_PROFILE,
    QWEN3_VL_8B_PROFILE,
    QWEN3_VL_4B_PROFILE,
    OVIS25_2B_PROFILE,
    QWEN25_7B_VLI_PROFILE,
    GEMMA2_2B_PROFILE,
    GEMMA3_4B_PROFILE,
    GEMMA3_4B_VISION_PROFILE_FACT,
    GEMMA3_12B_PROFILE,
    GEMMA4_PROFILE,
    GEMMA4_E2B_PROFILE,
    GEMMA4_31B_PROFILE,
    GPT_OSS_20B_PROFILE,
    QWEN35_2B_PROFILE,
    QWEN35_08B_PROFILE,
    QWEN35_4B_PROFILE,
    QWEN35_9B_PROFILE,
    QWEN35_27B_PROFILE,
];

pub fn decoder_profile_fact(name: &str) -> Option<&'static DecoderProfileFact> {
    let source_symbol = match name {
        "Llama2" => "Llama2Config",
        "Mistral3Small24B" => "Mistral3Small24BConfig",
        "Ministral3_3B" => "Ministral3_3BConfig",
        "Qwen25_3B" => "Qwen25_3BConfig",
        "Qwen3_06B" => "Qwen3_06BConfig",
        "Qwen3_06B_ACE15" => "Qwen3_06B_ACE15_Config",
        "Qwen3_2B_ACE15_lm" => "Qwen3_2B_ACE15_lm_Config",
        "Qwen3_4B_ACE15_lm" => "Qwen3_4B_ACE15_lm_Config",
        "Qwen3_4B" => "Qwen3_4BConfig",
        "Qwen3_8B" => "Qwen3_8BConfig",
        "Ovis25_2B" => "Ovis25_2BConfig",
        "Qwen25_7BVLI" => "Qwen25_7BVLI_Config",
        "Gemma2_2B" => "Gemma2_2B_Config",
        "Gemma3_4B" => "Gemma3_4B_Config",
        "Gemma3_4B_Vision" => "Gemma3_4B_Vision_Config",
        "Gemma3_12B" => "Gemma3_12B_Config",
        "Gemma4_E2B" => "Gemma4_E2B_Config",
        "Gemma4_31B" => "Gemma4_31B_Config",
        "qwen35_2b" => "Qwen35Config",
        other => other,
    };
    DECODER_PROFILE_FACTS
        .iter()
        .find(|profile| profile.source_symbol == source_symbol || profile.transformer_type == name)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecoderSymbolBehavior {
    Profile,
    ProfileFactory,
    Normalization,
    Rope,
    Attention,
    FeedForward,
    DecoderGraph,
    Generation,
    Mask,
    Router,
    Experts,
    Projection,
    VisionRope,
    VisionPatch,
    VisionFeedForward,
    VisionAttention,
    VisionGraph,
    AudioSubsample,
    AudioFeedForward,
    AudioRelativePosition,
    AudioAttention,
    AudioConvolution,
    AudioGraph,
    LinearRecurrence,
    CausalConvolution,
    VisionMerge,
    TokenizerAdapter,
    ModelAdapter,
}

pub fn decoder_symbol_behavior(symbol: &str) -> Option<DecoderSymbolBehavior> {
    use DecoderSymbolBehavior as Behavior;
    Some(match symbol {
        "Gemma4Config"
        | "Gemma4_E2B_Config"
        | "Gemma4_31B_Config"
        | "GptOss20BConfig"
        | "Llama2Config"
        | "Mistral3Small24BConfig"
        | "Ministral3_3BConfig"
        | "Qwen25_3BConfig"
        | "Qwen3_06BConfig"
        | "Qwen3_06B_ACE15_Config"
        | "Qwen3_2B_ACE15_lm_Config"
        | "Qwen3_4B_ACE15_lm_Config"
        | "Qwen3_4BConfig"
        | "Qwen3_8BConfig"
        | "Qwen3VL_8BConfig"
        | "Qwen3VL_4BConfig"
        | "Ovis25_2BConfig"
        | "Qwen25_7BVLI_Config"
        | "Gemma2_2B_Config"
        | "Gemma3_4B_Config"
        | "Gemma3_4B_Vision_Config"
        | "Gemma3_12B_Config"
        | "Qwen35Config" => Behavior::Profile,
        "_make_variant" | "_qwen35_layer_types" | "_make_config" => Behavior::ProfileFactory,
        "RMSNorm" | "RMSNormGated" => Behavior::Normalization,
        "_apply_rotary_pos_emb"
        | "_yarn_inv_freq"
        | "_build_freqs_cis"
        | "precompute_freqs_cis"
        | "apply_rope"
        | "precompute_partial_rope"
        | "apply_partial_rope" => Behavior::Rope,
        "Gemma4Attention"
        | "_attention_with_sinks"
        | "GptOssAttention"
        | "Attention"
        | "GatedAttention" => Behavior::Attention,
        "MLP" | "Gemma4VisionMLP" => Behavior::FeedForward,
        "TransformerBlockGemma4"
        | "Gemma4Transformer"
        | "Gemma4Base"
        | "GptOssDecoderLayer"
        | "GptOssModel"
        | "TransformerBlock"
        | "TransformerBlockGemma2"
        | "Llama2_"
        | "BaseLlama"
        | "BaseQwen3"
        | "Llama2"
        | "Mistral3Small24B"
        | "Ministral3_3B"
        | "Qwen25_3B"
        | "Qwen3_06B"
        | "Qwen3_06B_ACE15"
        | "Qwen3_2B_ACE15_lm"
        | "Qwen3_4B"
        | "Qwen3_4B_ACE15_lm"
        | "Qwen3_8B"
        | "Ovis25_2B"
        | "Qwen25_7BVLI"
        | "Gemma2_2B"
        | "Gemma3_4B"
        | "Gemma3_4B_Vision"
        | "Gemma3_12B"
        | "Qwen35TransformerBlock"
        | "Qwen35Transformer"
        | "Qwen35" => Behavior::DecoderGraph,
        "BaseGenerate" => Behavior::Generation,
        "_make_full_causal_mask" | "_make_sliding_causal_mask" => Behavior::Mask,
        "GptOssTopKRouter" => Behavior::Router,
        "GptOssExperts" | "GptOssMLP" => Behavior::Experts,
        "ClippedLinear"
        | "Gemma4RMSNormProjector"
        | "Gemma4MultiModalProjector"
        | "Gemma4AudioProjector"
        | "_make_scaled_embedding"
        | "Gemma3MultiModalProjector" => Behavior::Projection,
        "_compute_vision_2d_rope" | "_apply_vision_2d_rope" | "Qwen35VisionRotaryEmbedding" => {
            Behavior::VisionRope
        }
        "Gemma4PatchEmbedder" | "Qwen35VisionPatchEmbed" => Behavior::VisionPatch,
        "Qwen35VisionMLP" => Behavior::VisionFeedForward,
        "Gemma4VisionAttention" | "Qwen35VisionAttention" => Behavior::VisionAttention,
        "Gemma4VisionLayer"
        | "Gemma4VisionEncoderLayers"
        | "Gemma4VisionEncoder"
        | "Qwen35VisionBlock"
        | "Qwen35VisionModel" => Behavior::VisionGraph,
        "Gemma4AudioConvSubsampler" => Behavior::AudioSubsample,
        "Gemma4AudioFeedForward" => Behavior::AudioFeedForward,
        "Gemma4AudioRelPositionalEncoding" => Behavior::AudioRelativePosition,
        "Gemma4AudioAttention" => Behavior::AudioAttention,
        "Gemma4AudioLConv1d" => Behavior::AudioConvolution,
        "Gemma4AudioLayer" | "Gemma4AudioEncoder" => Behavior::AudioGraph,
        "torch_chunk_gated_delta_rule" | "GatedDeltaNet" => Behavior::LinearRecurrence,
        "torch_causal_conv1d_update" => Behavior::CausalConvolution,
        "Qwen35VisionPatchMerger" => Behavior::VisionMerge,
        "Gemma4_Tokenizer"
        | "_Gemma4Tokenizer"
        | "Gemma4SDTokenizer"
        | "Gemma4Tokenizer"
        | "_lens_render_chat"
        | "_GptOssRawTokenizer"
        | "LensGptOssTokenizer"
        | "LensTokenizer"
        | "Qwen35Tokenizer"
        | "Qwen35ImageTokenizer"
        | "tokenizer" => Behavior::TokenizerAdapter,
        "Gemma4AudioMixin"
        | "Gemma4Model"
        | "gemma4_te"
        | "LensGptOssClipModel"
        | "LensTEModel"
        | "lens_te"
        | "Qwen35ClipModel"
        | "Qwen35TEModel"
        | "te" => Behavior::ModelAdapter,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RopeScaling {
    None,
    Linear {
        factor: f32,
    },
    Yarn {
        factor: f32,
        beta_fast: f32,
        beta_slow: f32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecoderRopeConfiguration {
    pub theta: f32,
    pub rotary_dimension: usize,
    pub interleaved_sections: Vec<usize>,
    pub scaling: RopeScaling,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecoderTextConfiguration {
    pub architecture: DecoderArchitecture,
    pub dtype: DType,
    pub device: DeviceId,
    pub vocabulary_size: usize,
    pub maximum_tokens: usize,
    pub hidden_size: usize,
    pub feed_forward_size: usize,
    pub layer_kinds: Vec<DecoderLayerKind>,
    pub attention_heads: usize,
    pub key_value_heads: usize,
    pub head_dimension: usize,
    pub normalization_epsilon_bits: u32,
    pub rope: DecoderRopeConfiguration,
    pub sliding_window: Option<usize>,
    pub activation: DecoderActivation,
    pub embedding_scale_bits: u32,
    pub residual_scale_bits: u32,
    pub norm_weight_offset_bits: u32,
    pub logits_soft_cap_bits: Option<u32>,
    pub tied_output_head: bool,
    pub stop_tokens: Vec<i64>,
}

impl DecoderTextConfiguration {
    pub fn normalization_epsilon(&self) -> f32 {
        f32::from_bits(self.normalization_epsilon_bits)
    }

    pub fn embedding_scale(&self) -> f32 {
        f32::from_bits(self.embedding_scale_bits)
    }

    pub fn residual_scale(&self) -> f32 {
        f32::from_bits(self.residual_scale_bits)
    }

    pub fn norm_weight_offset(&self) -> f32 {
        f32::from_bits(self.norm_weight_offset_bits)
    }

    pub fn logits_soft_cap(&self) -> Option<f32> {
        self.logits_soft_cap_bits.map(f32::from_bits)
    }

    pub fn validate(&self) -> Result<(), DecoderTextError> {
        let epsilon = self.normalization_epsilon();
        let embedding_scale = self.embedding_scale();
        let residual_scale = self.residual_scale();
        if self.dtype != DType::F32 || self.device != DeviceId::CPU {
            return Err(DecoderTextError::UnsupportedTarget {
                dtype: self.dtype,
                device: self.device,
            });
        }
        if self.vocabulary_size == 0
            || self.maximum_tokens == 0
            || self.hidden_size == 0
            || self.feed_forward_size == 0
            || self.layer_kinds.is_empty()
            || self.attention_heads == 0
            || self.key_value_heads == 0
            || self.head_dimension == 0
            || !self.attention_heads.is_multiple_of(self.key_value_heads)
            || !epsilon.is_finite()
            || epsilon <= 0.0
            || !embedding_scale.is_finite()
            || embedding_scale <= 0.0
            || !residual_scale.is_finite()
            || residual_scale <= 0.0
            || !self.norm_weight_offset().is_finite()
            || self.rope.rotary_dimension == 0
            || self.rope.rotary_dimension > self.head_dimension
            || !self.rope.rotary_dimension.is_multiple_of(2)
            || !self.rope.theta.is_finite()
            || self.rope.theta <= 0.0
            || self.sliding_window == Some(0)
            || self
                .logits_soft_cap()
                .is_some_and(|cap| !cap.is_finite() || cap <= 0.0)
        {
            return Err(DecoderTextError::InvalidConfiguration(
                "decoder dimensions, target, scales, normalization, or RoPE are invalid",
            ));
        }
        let section_total = self
            .rope
            .interleaved_sections
            .iter()
            .try_fold(0_usize, |sum, section| sum.checked_add(*section));
        if !self.rope.interleaved_sections.is_empty()
            && section_total != Some(self.rope.rotary_dimension / 2)
        {
            return Err(DecoderTextError::InvalidConfiguration(
                "multidimensional RoPE sections must cover every rotary pair",
            ));
        }
        match self.rope.scaling {
            RopeScaling::None => {}
            RopeScaling::Linear { factor } if factor.is_finite() && factor >= 1.0 => {}
            RopeScaling::Yarn {
                factor,
                beta_fast,
                beta_slow,
            } if factor.is_finite()
                && factor >= 1.0
                && beta_fast.is_finite()
                && beta_slow.is_finite()
                && beta_fast > beta_slow
                && beta_slow >= 0.0 => {}
            _ => {
                return Err(DecoderTextError::InvalidConfiguration(
                    "RoPE scaling parameters are invalid",
                ));
            }
        }
        if self
            .layer_kinds
            .contains(&DecoderLayerKind::SlidingAttention)
            && self.sliding_window.is_none()
        {
            return Err(DecoderTextError::InvalidConfiguration(
                "sliding attention requires a nonzero window",
            ));
        }
        if self
            .layer_kinds
            .contains(&DecoderLayerKind::LinearAttention)
            && self.architecture != DecoderArchitecture::Qwen35
        {
            return Err(DecoderTextError::InvalidConfiguration(
                "linear attention is owned only by Qwen3.5 profiles",
            ));
        }
        if self
            .layer_kinds
            .contains(&DecoderLayerKind::LinearAttention)
            && (self.key_value_heads != self.attention_heads
                || self.hidden_size != self.attention_heads.saturating_mul(self.head_dimension))
        {
            return Err(DecoderTextError::InvalidConfiguration(
                "Qwen3.5 linear-attention profiles require equal projected head counts and hidden width",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct DecoderLayerWeights {
    pub attention_norm_weight: Tensor,
    pub query_weight: Tensor,
    pub key_weight: Tensor,
    pub value_weight: Tensor,
    pub attention_output_weight: Tensor,
    pub feed_forward_norm_weight: Tensor,
    pub feed_forward_gate_weight: Tensor,
    pub feed_forward_up_weight: Tensor,
    pub feed_forward_down_weight: Tensor,
    pub post_attention_norm_weight: Option<Tensor>,
    pub post_feed_forward_norm_weight: Option<Tensor>,
    pub attention_sink: Option<Tensor>,
}

#[derive(Clone, Debug)]
pub struct DecoderTextWeights {
    pub token_embedding: Tensor,
    pub layers: Vec<DecoderLayerWeights>,
    pub final_norm_weight: Tensor,
    pub output_head_weight: Option<Tensor>,
}

#[derive(Clone, Debug)]
pub struct DecoderAttentionCache {
    batch: usize,
    key_value_heads: usize,
    head_dimension: usize,
    tokens: usize,
    keys: Tensor,
    values: Tensor,
}

impl DecoderAttentionCache {
    pub fn tokens(&self) -> usize {
        self.tokens
    }

    pub fn keys(&self) -> &Tensor {
        &self.keys
    }

    pub fn values(&self) -> &Tensor {
        &self.values
    }
}

#[derive(Clone, Debug)]
pub struct Qwen35LinearCache {
    pub convolution_state: Tensor,
    pub recurrent_state: Tensor,
}

#[derive(Clone, Debug)]
pub enum DecoderLayerCache {
    Attention(DecoderAttentionCache),
    Linear(Qwen35LinearCache),
}

#[derive(Clone, Debug, Default)]
pub struct DecoderKvState {
    layers: Vec<Option<DecoderLayerCache>>,
}

impl DecoderKvState {
    pub fn new(layer_count: usize) -> Result<Self, DecoderTextError> {
        let mut layers = Vec::new();
        layers
            .try_reserve_exact(layer_count)
            .map_err(|_| DecoderTextError::Allocation("decoder cache layers"))?;
        layers.resize_with(layer_count, || None);
        Ok(Self { layers })
    }

    pub fn layers(&self) -> &[Option<DecoderLayerCache>] {
        &self.layers
    }
}

#[derive(Clone, Debug)]
pub struct DecoderTextRequest<'a> {
    pub tokens: &'a Tensor,
    pub attention_mask: Option<&'a Tensor>,
    pub positions: Option<&'a [usize]>,
    pub cache: Option<&'a DecoderKvState>,
    pub capture_layer: Option<isize>,
}

#[derive(Clone, Copy, Debug)]
pub enum DecoderRopePositions<'a> {
    Scalar(&'a [usize]),
    Multidimensional(&'a [Vec<usize>]),
}

#[derive(Clone, Copy, Debug)]
pub struct DecoderPreparedTextRequest<'a> {
    pub embeddings: &'a Tensor,
    pub attention_mask: Option<&'a Tensor>,
    pub rope_positions: DecoderRopePositions<'a>,
    pub causal_positions: &'a [usize],
    pub cache: Option<&'a DecoderKvState>,
    pub capture_layer: Option<isize>,
}

#[derive(Clone, Copy, Debug)]
pub struct DecoderPreparedGenerationPrompt<'a> {
    pub embeddings: &'a Tensor,
    pub sampling_history: &'a [i64],
    pub attention_mask: Option<&'a Tensor>,
    pub rope_positions: DecoderRopePositions<'a>,
    pub causal_positions: &'a [usize],
}

#[derive(Clone, Debug)]
pub struct DecoderTextOutput {
    last_hidden_state: Tensor,
    intermediate: Option<Tensor>,
    logits: Tensor,
    cache: DecoderKvState,
}

impl DecoderTextOutput {
    pub fn last_hidden_state(&self) -> &Tensor {
        &self.last_hidden_state
    }

    pub fn intermediate(&self) -> Option<&Tensor> {
        self.intermediate.as_ref()
    }

    pub fn logits(&self) -> &Tensor {
        &self.logits
    }

    pub fn cache(&self) -> &DecoderKvState {
        &self.cache
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecoderGenerationConfiguration {
    pub maximum_new_tokens: usize,
    pub temperature_bits: u32,
    pub top_k: Option<usize>,
    pub top_p_bits: Option<u32>,
    pub minimum_p_bits: Option<u32>,
    pub repetition_penalty_bits: u32,
    pub presence_penalty_bits: u32,
}

impl DecoderGenerationConfiguration {
    pub fn temperature(&self) -> f32 {
        f32::from_bits(self.temperature_bits)
    }

    pub fn top_p(&self) -> Option<f32> {
        self.top_p_bits.map(f32::from_bits)
    }

    pub fn minimum_p(&self) -> Option<f32> {
        self.minimum_p_bits.map(f32::from_bits)
    }

    pub fn repetition_penalty(&self) -> f32 {
        f32::from_bits(self.repetition_penalty_bits)
    }

    pub fn presence_penalty(&self) -> f32 {
        f32::from_bits(self.presence_penalty_bits)
    }

    fn validate(&self, vocabulary_size: usize) -> Result<(), DecoderTextError> {
        let temperature = self.temperature();
        let repetition_penalty = self.repetition_penalty();
        let presence_penalty = self.presence_penalty();
        if self.maximum_new_tokens == 0
            || !temperature.is_finite()
            || temperature < 0.0
            || self
                .top_k
                .is_some_and(|top_k| top_k == 0 || top_k > vocabulary_size)
            || self
                .top_p()
                .is_some_and(|value| !value.is_finite() || value < 0.0 || value > 1.0)
            || self
                .minimum_p()
                .is_some_and(|value| !value.is_finite() || value < 0.0 || value > 1.0)
            || !repetition_penalty.is_finite()
            || repetition_penalty < 0.0
            || !presence_penalty.is_finite()
        {
            return Err(DecoderTextError::InvalidInput(
                "generation limits or filters are invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct DecoderGenerationOutcome {
    pub tokens: Vec<i64>,
    pub cache: DecoderKvState,
    pub transaction: RngTransaction,
}

#[derive(Clone)]
pub struct DecoderPreparedGenerationOutcome {
    pub generated_tokens: Vec<i64>,
    pub cache: DecoderKvState,
    pub transaction: RngTransaction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeTextGenerationRequest<'a> {
    pub formatted_prompt: &'a str,
    pub maximum_new_tokens: usize,
    pub do_sample: bool,
    pub temperature_bits: u32,
    pub top_k: usize,
    pub top_p_bits: u32,
    pub minimum_p_bits: u32,
    pub repetition_penalty_bits: u32,
    pub presence_penalty_bits: u32,
}

#[derive(Clone)]
pub struct NativeTextGenerationResult {
    pub text: String,
    pub generated_tokens: Vec<u32>,
    pub transaction: RngTransaction,
}

#[derive(Clone, Debug)]
struct NativeDecoderLayer {
    kind: DecoderLayerKind,
    attention_norm_weight: Tensor,
    query: NativeModule,
    key: NativeModule,
    value: NativeModule,
    attention_output: NativeModule,
    feed_forward_norm_weight: Tensor,
    feed_forward_gate: NativeModule,
    feed_forward_up: NativeModule,
    activation: NativeModule,
    feed_forward_down: NativeModule,
    post_attention_norm_weight: Option<Tensor>,
    post_feed_forward_norm_weight: Option<Tensor>,
    attention_sink: Option<Tensor>,
}

#[derive(Clone, Debug)]
pub struct NativeDecoderTextEncoder {
    configuration: DecoderTextConfiguration,
    token_embedding: NativeModule,
    layers: Vec<NativeDecoderLayer>,
    final_norm_weight: Tensor,
    output_head: NativeModule,
    stream: StreamId,
}

#[derive(Clone, Debug)]
struct ValidatedDecoderPositions {
    rope_axes: Vec<Vec<usize>>,
    causal: Vec<usize>,
}

enum DecoderGenerationPrefill<'a> {
    Tokens(&'a Tensor),
    Prepared(DecoderPreparedTextRequest<'a>),
}

struct InternalDecoderGenerationOutcome {
    history_and_generated: Vec<i64>,
    generated_start: usize,
    cache: DecoderKvState,
    transaction: RngTransaction,
}

impl ValidatedDecoderPositions {
    fn scalar(positions: Vec<usize>) -> Self {
        Self {
            causal: positions.clone(),
            rope_axes: vec![positions],
        }
    }

    fn rope(&self) -> DecoderRopePositions<'_> {
        if self.rope_axes.len() == 1 {
            DecoderRopePositions::Scalar(&self.rope_axes[0])
        } else {
            DecoderRopePositions::Multidimensional(&self.rope_axes)
        }
    }
}

#[derive(Debug, Error)]
pub enum DecoderTextError {
    #[error(transparent)]
    Module(#[from] NativeOpsError),
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    TensorOperation(#[from] NativeDiffusionTensorError),
    #[error(transparent)]
    Functional(#[from] FunctionalError),
    #[error(transparent)]
    Attention(#[from] AttentionError),
    #[error(transparent)]
    Rng(#[from] RngError),
    #[error(transparent)]
    Tokenizer(#[from] crate::NativeTokenizerError),
    #[error(transparent)]
    Cancellation(#[from] comfy_types::CancellationError),
    #[error("decoder target {device:?}/{dtype:?} is unsupported")]
    UnsupportedTarget { dtype: DType, device: DeviceId },
    #[error("decoder configuration is invalid: {0}")]
    InvalidConfiguration(&'static str),
    #[error("decoder input is invalid: {0}")]
    InvalidInput(&'static str),
    #[error("decoder token {0} is outside the configured vocabulary")]
    TokenOutOfRange(i64),
    #[error("decoder capture layer {requested} is outside {available} layers")]
    CaptureOutOfRange { requested: isize, available: usize },
    #[error("decoder arithmetic overflowed while computing {0}")]
    Overflow(&'static str),
    #[error("decoder allocation failed for {0}")]
    Allocation(&'static str),
}

impl NativeDecoderTextEncoder {
    pub fn new(
        configuration: DecoderTextConfiguration,
        weights: DecoderTextWeights,
    ) -> Result<Self, DecoderTextError> {
        configuration.validate()?;
        if weights.layers.len() != configuration.layer_kinds.len() {
            return Err(DecoderTextError::InvalidConfiguration(
                "weight layer count does not match the profile",
            ));
        }
        let stream = weights.token_embedding.descriptor().stream();
        require_parameter(&weights.token_embedding, stream)?;
        let mut token_embedding = NativeModule::embedding(
            "decoder.token_embedding",
            configuration.vocabulary_size,
            configuration.hidden_size,
            EmbeddingOptions::default(),
            false,
        )?;
        token_embedding.load_dense_parameters(weights.token_embedding.clone(), None)?;

        let mut layers = Vec::new();
        layers
            .try_reserve_exact(weights.layers.len())
            .map_err(|_| DecoderTextError::Allocation("decoder layers"))?;
        for (index, (kind, weights)) in configuration
            .layer_kinds
            .iter()
            .copied()
            .zip(weights.layers)
            .enumerate()
        {
            layers.push(build_layer(index, kind, &configuration, weights, stream)?);
        }
        require_vector_parameter(
            &weights.final_norm_weight,
            configuration.hidden_size,
            stream,
        )?;
        let output_weight = match weights.output_head_weight {
            Some(weight) if !configuration.tied_output_head => weight,
            None if configuration.tied_output_head => weights.token_embedding,
            _ => {
                return Err(DecoderTextError::InvalidConfiguration(
                    "tied and untied output-head parameters do not match the profile",
                ));
            }
        };
        require_parameter(&output_weight, stream)?;
        let mut output_head = NativeModule::linear(
            "decoder.output_head",
            configuration.hidden_size,
            configuration.vocabulary_size,
            false,
            false,
        )?;
        output_head.load_dense_parameters(output_weight, None)?;
        Ok(Self {
            configuration,
            token_embedding,
            layers,
            final_norm_weight: weights.final_norm_weight,
            output_head,
            stream,
        })
    }

    pub fn configuration(&self) -> &DecoderTextConfiguration {
        &self.configuration
    }

    pub fn semantic_state_digest(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<String, DecoderTextError> {
        cancellation.check()?;
        let mut hasher = Sha256::new();
        hasher.update(b"sim.comfy.native-decoder-text.v1");
        hasher.update(format!("{:?}", self.configuration).as_bytes());
        for (name, module) in self.named_modules() {
            cancellation.check()?;
            hasher.update([0]);
            hasher.update(name.as_bytes());
            hasher.update([0]);
            hasher.update(module.semantic_state_digest(cancellation)?.as_bytes());
        }
        for (name, tensor) in self.normalization_tensors() {
            cancellation.check()?;
            hasher.update([0]);
            hasher.update(name.as_bytes());
            hasher.update([0]);
            hasher.update(tensor.contiguous_bytes()?);
        }
        cancellation.check()?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn resident_bytes(&self) -> Result<u64, DecoderTextError> {
        self.resident_tensor_allocations().into_iter().try_fold(
            self.resident_owned_bytes()?,
            |bytes, (_, allocation)| {
                bytes
                    .checked_add(allocation)
                    .ok_or(DecoderTextError::Overflow("decoder residency"))
            },
        )
    }

    pub fn resident_owned_bytes(&self) -> Result<u64, DecoderTextError> {
        let mut bytes = u64::try_from(std::mem::size_of::<Self>())
            .map_err(|_| DecoderTextError::Overflow("decoder residency"))?;
        bytes = bytes
            .checked_add(
                u64::try_from(
                    self.layers
                        .capacity()
                        .checked_mul(std::mem::size_of::<NativeDecoderLayer>())
                        .ok_or(DecoderTextError::Overflow("decoder layers"))?,
                )
                .map_err(|_| DecoderTextError::Overflow("decoder layers"))?,
            )
            .ok_or(DecoderTextError::Overflow("decoder residency"))?;
        for (_, module) in self.named_modules() {
            let tensor_bytes = module.resident_tensor_allocations().into_iter().try_fold(
                0_u64,
                |bytes, (_, allocation)| {
                    bytes
                        .checked_add(allocation)
                        .ok_or(DecoderTextError::Overflow("decoder tensor residency"))
                },
            )?;
            let module_bytes = module.resident_storage_bytes()?;
            bytes = bytes
                .checked_add(module_bytes.checked_sub(tensor_bytes).ok_or(
                    DecoderTextError::Overflow("decoder module residency projection"),
                )?)
                .ok_or(DecoderTextError::Overflow("decoder module residency"))?;
        }
        Ok(bytes)
    }

    pub fn resident_tensor_allocations(&self) -> Vec<(comfy_tensor::StorageId, u64)> {
        let mut allocations = Vec::new();
        for (_, module) in self.named_modules() {
            for (storage_id, resident_bytes) in module.resident_tensor_allocations() {
                if !allocations
                    .iter()
                    .any(|(existing, _)| *existing == storage_id)
                {
                    allocations.push((storage_id, resident_bytes));
                }
            }
        }
        for (_, tensor) in self.normalization_tensors() {
            let storage_id = tensor.storage_id();
            if !allocations
                .iter()
                .any(|(existing, _)| *existing == storage_id)
            {
                allocations.push((storage_id, tensor.storage_byte_len()));
            }
        }
        allocations
    }

    fn named_modules(&self) -> Vec<(String, &NativeModule)> {
        let mut modules = vec![
            ("token_embedding".to_owned(), &self.token_embedding),
            ("output_head".to_owned(), &self.output_head),
        ];
        for (index, layer) in self.layers.iter().enumerate() {
            for (name, module) in [
                ("query", &layer.query),
                ("key", &layer.key),
                ("value", &layer.value),
                ("attention_output", &layer.attention_output),
                ("feed_forward_gate", &layer.feed_forward_gate),
                ("feed_forward_up", &layer.feed_forward_up),
                ("activation", &layer.activation),
                ("feed_forward_down", &layer.feed_forward_down),
            ] {
                modules.push((format!("layers.{index}.{name}"), module));
            }
        }
        modules
    }

    fn normalization_tensors(&self) -> Vec<(String, &Tensor)> {
        let mut tensors = vec![("final_norm_weight".to_owned(), &self.final_norm_weight)];
        for (index, layer) in self.layers.iter().enumerate() {
            tensors.push((
                format!("layers.{index}.attention_norm_weight"),
                &layer.attention_norm_weight,
            ));
            tensors.push((
                format!("layers.{index}.feed_forward_norm_weight"),
                &layer.feed_forward_norm_weight,
            ));
            if let Some(tensor) = &layer.post_attention_norm_weight {
                tensors.push((format!("layers.{index}.post_attention_norm_weight"), tensor));
            }
            if let Some(tensor) = &layer.post_feed_forward_norm_weight {
                tensors.push((
                    format!("layers.{index}.post_feed_forward_norm_weight"),
                    tensor,
                ));
            }
            if let Some(tensor) = &layer.attention_sink {
                tensors.push((format!("layers.{index}.attention_sink"), tensor));
            }
        }
        tensors
    }

    pub fn execution_requirements(&self) -> NativeExecutionRequirements {
        let mut requirements = NativeExecutionRequirements::new();
        requirements.extend(
            self.token_embedding
                .execution_requirements(DType::F32)
                .iter(),
        );
        requirements.extend(self.output_head.execution_requirements(DType::F32).iter());
        for layer in &self.layers {
            for module in [
                &layer.query,
                &layer.key,
                &layer.value,
                &layer.attention_output,
                &layer.feed_forward_gate,
                &layer.feed_forward_up,
                &layer.activation,
                &layer.feed_forward_down,
            ] {
                requirements.extend(module.execution_requirements(DType::F32).iter());
            }
        }
        requirements.extend([
            OperationSupport::allocation(DType::F32, Layout::Contiguous),
            OperationSupport::copy_input(DType::F32, Layout::Contiguous),
            OperationSupport::copy_output(DType::F32, Layout::Contiguous),
            OperationSupport::binary_input(BinaryOperation::Add, DType::F32, Layout::Contiguous),
            OperationSupport::binary_output(BinaryOperation::Add, DType::F32, Layout::Contiguous),
            OperationSupport::binary_input(
                BinaryOperation::Multiply,
                DType::F32,
                Layout::Contiguous,
            ),
            OperationSupport::binary_output(
                BinaryOperation::Multiply,
                DType::F32,
                Layout::Contiguous,
            ),
            OperationSupport::linear_algebra_input(
                LinearAlgebraOperation::BatchMatrixMultiply,
                DType::F32,
                Layout::Contiguous,
            ),
            OperationSupport::linear_algebra_output(
                LinearAlgebraOperation::BatchMatrixMultiply,
                DType::F32,
                Layout::Contiguous,
            ),
            OperationSupport::unary_input(
                UnaryOperation::SquareRoot,
                DType::F32,
                Layout::Contiguous,
            ),
            OperationSupport::unary_output(
                UnaryOperation::SquareRoot,
                DType::F32,
                Layout::Contiguous,
            ),
        ]);
        requirements
    }

    pub fn admit_execution_target(
        &self,
        backend: &CpuBackend,
        context: &ExecutionContext<'_>,
    ) -> Result<(), DecoderTextError> {
        self.execution_requirements().admit_backend_target(
            backend,
            self.configuration.device,
            self.configuration.dtype,
            Layout::Contiguous,
            self.stream,
            context,
        )?;
        Ok(())
    }

    pub fn forward(
        &self,
        backend: &CpuBackend,
        request: DecoderTextRequest<'_>,
        context: &ExecutionContext<'_>,
    ) -> Result<DecoderTextOutput, DecoderTextError> {
        self.admit_execution_target(backend, context)?;
        let (batch, query_tokens) = validate_tokens(
            backend,
            request.tokens,
            self.configuration.vocabulary_size,
            self.configuration.maximum_tokens,
            context,
        )?;
        let positions = ValidatedDecoderPositions::scalar(validate_positions(
            request.positions,
            query_tokens,
            request.cache,
        )?);
        let hidden = self.embed_validated_tokens(backend, request.tokens, context)?;
        self.forward_hidden(
            backend,
            hidden,
            request.attention_mask,
            &positions,
            request.cache,
            request.capture_layer,
            batch,
            query_tokens,
            context,
        )
    }

    pub fn embed_tokens(
        &self,
        backend: &CpuBackend,
        tokens: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, DecoderTextError> {
        self.admit_execution_target(backend, context)?;
        validate_tokens(
            backend,
            tokens,
            self.configuration.vocabulary_size,
            self.configuration.maximum_tokens,
            context,
        )?;
        self.embed_validated_tokens(backend, tokens, context)
    }

    pub fn forward_prepared(
        &self,
        backend: &CpuBackend,
        request: DecoderPreparedTextRequest<'_>,
        context: &ExecutionContext<'_>,
    ) -> Result<DecoderTextOutput, DecoderTextError> {
        self.admit_execution_target(backend, context)?;
        let (batch, query_tokens) =
            self.validate_prepared_embeddings(request.embeddings, context)?;
        let positions = validate_prepared_positions(
            request.rope_positions,
            request.causal_positions,
            query_tokens,
            request.cache,
            &self.configuration.rope,
        )?;
        self.forward_hidden(
            backend,
            request.embeddings.clone(),
            request.attention_mask,
            &positions,
            request.cache,
            request.capture_layer,
            batch,
            query_tokens,
            context,
        )
    }

    fn embed_validated_tokens(
        &self,
        backend: &CpuBackend,
        tokens: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, DecoderTextError> {
        let mut embedding = self.token_embedding.clone();
        let mut hidden = embedding.forward_with_context(backend, tokens, context)?;
        if self.configuration.embedding_scale() != 1.0 {
            hidden = scale_tensor(
                backend,
                &hidden,
                self.configuration.embedding_scale(),
                context,
            )?;
        }
        Ok(hidden)
    }

    #[allow(clippy::too_many_arguments)]
    fn forward_hidden(
        &self,
        backend: &CpuBackend,
        mut hidden: Tensor,
        attention_mask: Option<&Tensor>,
        positions: &ValidatedDecoderPositions,
        cache: Option<&DecoderKvState>,
        capture_layer: Option<isize>,
        batch: usize,
        query_tokens: usize,
        context: &ExecutionContext<'_>,
    ) -> Result<DecoderTextOutput, DecoderTextError> {
        let mut staged_cache = match cache {
            Some(cache) => validate_cache(cache, batch, self.stream, &self.configuration)?,
            None => DecoderKvState::new(self.layers.len())?,
        };
        let capture = capture_layer
            .map(|layer| resolve_layer(layer, self.layers.len()))
            .transpose()?;
        let mut intermediate = None;
        for (layer_index, layer) in self.layers.iter().enumerate() {
            context.check()?;
            hidden = layer.forward(
                backend,
                &hidden,
                attention_mask,
                positions,
                batch,
                query_tokens,
                &self.configuration,
                staged_cache
                    .layers
                    .get_mut(layer_index)
                    .ok_or(DecoderTextError::InvalidInput("cache layer is missing"))?,
                context,
            )?;
            if capture == Some(layer_index) {
                intermediate = Some(hidden.clone());
            }
        }
        let last_hidden_state = rms_norm_tensor(
            backend,
            &hidden,
            &self.final_norm_weight,
            self.configuration.hidden_size,
            self.configuration.normalization_epsilon(),
            self.configuration.norm_weight_offset(),
            context,
        )?;
        let mut output_head = self.output_head.clone();
        let logits = output_head.forward_with_context(backend, &last_hidden_state, context)?;
        let logits = apply_logits_soft_cap(
            backend,
            &logits,
            self.configuration.logits_soft_cap(),
            context,
        )?;
        context.check()?;
        Ok(DecoderTextOutput {
            last_hidden_state,
            intermediate,
            logits,
            cache: staged_cache,
        })
    }

    fn validate_prepared_embeddings(
        &self,
        embeddings: &Tensor,
        context: &ExecutionContext<'_>,
    ) -> Result<(usize, usize), DecoderTextError> {
        context.check()?;
        let descriptor = embeddings.descriptor();
        let shape = descriptor.shape();
        if descriptor.dtype() != self.configuration.dtype
            || descriptor.device() != self.configuration.device
            || descriptor.stream() != self.stream
            || !descriptor.is_contiguous()?
            || shape.len() != 3
            || shape[0] != 1
            || shape[1] == 0
            || shape[2] != usize_to_u64(self.configuration.hidden_size, "prepared hidden width")?
        {
            return Err(DecoderTextError::InvalidInput(
                "prepared embeddings must be contiguous [1, tokens, hidden] on the decoder target",
            ));
        }
        let tokens = usize::try_from(shape[1])
            .map_err(|_| DecoderTextError::Overflow("prepared token count"))?;
        if tokens > self.configuration.maximum_tokens {
            return Err(DecoderTextError::InvalidInput(
                "prepared embeddings exceed the decoder token limit",
            ));
        }
        Ok((1, tokens))
    }

    fn prepared_attention_values(
        &self,
        backend: &CpuBackend,
        mask: &Tensor,
        prompt_count: usize,
        context: &ExecutionContext<'_>,
    ) -> Result<Vec<f32>, DecoderTextError> {
        let descriptor = mask.descriptor();
        if descriptor.shape() != [1, usize_to_u64(prompt_count, "prepared attention mask")?]
            || descriptor.dtype() != DType::F32
            || descriptor.device() != self.configuration.device
            || descriptor.stream() != self.stream
            || !descriptor.is_contiguous()?
        {
            return Err(DecoderTextError::InvalidInput(
                "prepared attention mask must be contiguous F32 [1, tokens] on the decoder target",
            ));
        }
        let values = tensor_to_f32(backend, mask, context)?;
        if values.iter().any(|value| !matches!(*value, 0.0 | 1.0)) {
            return Err(DecoderTextError::InvalidInput(
                "prepared attention mask values must be zero or one",
            ));
        }
        Ok(values.iter().copied().collect())
    }

    fn continuation_attention_mask(
        &self,
        backend: &CpuBackend,
        prefix: &[f32],
        generated_count: usize,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, DecoderTextError> {
        let total = prefix
            .len()
            .checked_add(generated_count)
            .ok_or(DecoderTextError::Overflow("continuation attention mask"))?;
        let mut values = prefix.to_vec();
        values.resize(total, 1.0);
        Ok(tensor_from_f32(
            backend,
            &[1, usize_to_u64(total, "continuation attention mask")?],
            &values,
            context,
        )?)
    }

    pub fn generate(
        &self,
        backend: &CpuBackend,
        prompt_tokens: &Tensor,
        configuration: &DecoderGenerationConfiguration,
        transaction: &RngTransaction,
        context: &ExecutionContext<'_>,
    ) -> Result<DecoderGenerationOutcome, DecoderTextError> {
        let (_, prompt_count) = validate_tokens(
            backend,
            prompt_tokens,
            self.configuration.vocabulary_size,
            self.configuration.maximum_tokens,
            context,
        )?;
        if prompt_tokens.descriptor().shape().first() != Some(&1) {
            return Err(DecoderTextError::InvalidInput(
                "generation currently requires one prompt batch",
            ));
        }
        let prompt_values = read_i64_tensor(backend, prompt_tokens, context)?;
        let outcome = self.generate_with_prefill(
            backend,
            DecoderGenerationPrefill::Tokens(prompt_tokens),
            &prompt_values,
            prompt_count,
            configuration,
            transaction,
            None,
            context,
        )?;
        Ok(DecoderGenerationOutcome {
            tokens: outcome.history_and_generated,
            cache: outcome.cache,
            transaction: outcome.transaction,
        })
    }

    pub fn generate_prepared(
        &self,
        backend: &CpuBackend,
        prompt: DecoderPreparedGenerationPrompt<'_>,
        configuration: &DecoderGenerationConfiguration,
        transaction: &RngTransaction,
        context: &ExecutionContext<'_>,
    ) -> Result<DecoderPreparedGenerationOutcome, DecoderTextError> {
        let (_, prompt_count) = self.validate_prepared_embeddings(prompt.embeddings, context)?;
        if prompt_count
            .checked_add(configuration.maximum_new_tokens)
            .ok_or(DecoderTextError::Overflow(
                "prepared generation token limit",
            ))?
            > self.configuration.maximum_tokens
        {
            return Err(DecoderTextError::InvalidInput(
                "prepared prompt and generated tokens exceed the decoder limit",
            ));
        }
        let prepared = DecoderPreparedTextRequest {
            embeddings: prompt.embeddings,
            attention_mask: prompt.attention_mask,
            rope_positions: prompt.rope_positions,
            causal_positions: prompt.causal_positions,
            cache: None,
            capture_layer: None,
        };
        let continuation_attention = prompt
            .attention_mask
            .map(|mask| self.prepared_attention_values(backend, mask, prompt_count, context))
            .transpose()?;
        let next_position = maximum_rope_position(prompt.rope_positions)?
            .checked_add(1)
            .ok_or(DecoderTextError::Overflow("prepared continuation position"))?;
        let outcome = self.generate_with_prefill(
            backend,
            DecoderGenerationPrefill::Prepared(prepared),
            prompt.sampling_history,
            next_position,
            configuration,
            transaction,
            continuation_attention,
            context,
        )?;
        let generated_tokens = outcome
            .history_and_generated
            .get(outcome.generated_start..)
            .ok_or(DecoderTextError::InvalidInput(
                "prepared generation history boundary is invalid",
            ))?
            .to_vec();
        Ok(DecoderPreparedGenerationOutcome {
            generated_tokens,
            cache: outcome.cache,
            transaction: outcome.transaction,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_with_prefill(
        &self,
        backend: &CpuBackend,
        prefill: DecoderGenerationPrefill<'_>,
        sampling_history: &[i64],
        continuation_position: usize,
        configuration: &DecoderGenerationConfiguration,
        transaction: &RngTransaction,
        continuation_attention: Option<Vec<f32>>,
        context: &ExecutionContext<'_>,
    ) -> Result<InternalDecoderGenerationOutcome, DecoderTextError> {
        configuration.validate(self.configuration.vocabulary_size)?;
        transaction.require_device(self.configuration.device)?;
        let generated_start = sampling_history.len();
        let mut generated = sampling_history.to_vec();
        let mut staged_transaction = transaction.clone();
        let mut cache = None;
        let mut current = None;
        let mut prefill = Some(prefill);
        let mut next_position = continuation_position;
        let mut generated_count = 0_usize;
        for _ in 0..configuration.maximum_new_tokens {
            context.check()?;
            let output = match prefill.take() {
                Some(DecoderGenerationPrefill::Tokens(tokens)) => self.forward(
                    backend,
                    DecoderTextRequest {
                        tokens,
                        attention_mask: None,
                        positions: None,
                        cache: None,
                        capture_layer: None,
                    },
                    context,
                )?,
                Some(DecoderGenerationPrefill::Prepared(request)) => {
                    self.forward_prepared(backend, request, context)?
                }
                None => {
                    let token = current.as_ref().ok_or(DecoderTextError::InvalidInput(
                        "decoder continuation token is missing",
                    ))?;
                    let position = [next_position];
                    let continuation_mask = continuation_attention
                        .as_ref()
                        .map(|prefix| {
                            self.continuation_attention_mask(
                                backend,
                                prefix,
                                generated_count,
                                context,
                            )
                        })
                        .transpose()?;
                    let output = self.forward(
                        backend,
                        DecoderTextRequest {
                            tokens: token,
                            attention_mask: continuation_mask.as_ref(),
                            positions: Some(&position),
                            cache: cache.as_ref(),
                            capture_layer: None,
                        },
                        context,
                    )?;
                    next_position = next_position
                        .checked_add(1)
                        .ok_or(DecoderTextError::Overflow("generation position"))?;
                    output
                }
            };
            let logits = tensor_to_f32(backend, output.logits(), context)?;
            let token_logits = logits
                .get(
                    logits
                        .len()
                        .saturating_sub(self.configuration.vocabulary_size)..,
                )
                .ok_or(DecoderTextError::InvalidInput(
                    "generation logits are incomplete",
                ))?;
            let next = sample_token(
                token_logits,
                &generated,
                configuration,
                &mut staged_transaction,
                context.cancellation,
            )?;
            generated.push(next);
            generated_count = generated_count
                .checked_add(1)
                .ok_or(DecoderTextError::Overflow("generated token count"))?;
            cache = Some(output.cache);
            if self.configuration.stop_tokens.contains(&next) {
                break;
            }
            current = Some(tensor_from_i64(backend, &[1, 1], &[next], context)?);
        }
        Ok(InternalDecoderGenerationOutcome {
            history_and_generated: generated,
            generated_start,
            cache: cache.unwrap_or(DecoderKvState::new(self.layers.len())?),
            transaction: staged_transaction,
        })
    }

    pub fn generate_text(
        &self,
        tokenizer: &NativePromptTokenizer,
        backend: &CpuBackend,
        request: NativeTextGenerationRequest<'_>,
        transaction: &RngTransaction,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeTextGenerationResult, DecoderTextError> {
        context.check()?;
        if !(1..=32_768).contains(&request.maximum_new_tokens) {
            return Err(DecoderTextError::InvalidInput(
                "maximum new tokens must be between 1 and 32768",
            ));
        }
        let prompt_tokens =
            tokenizer.encode_numeric(request.formatted_prompt, context.cancellation)?;
        let prompt_length = prompt_tokens.len();
        let maximum_total = prompt_length
            .checked_add(request.maximum_new_tokens)
            .ok_or(DecoderTextError::Overflow("text generation token limit"))?;
        if maximum_total > self.configuration.maximum_tokens {
            return Err(DecoderTextError::InvalidInput(
                "prompt and generated tokens exceed the decoder limit",
            ));
        }
        let prompt_values = prompt_tokens
            .iter()
            .copied()
            .map(i64::from)
            .collect::<Vec<_>>();
        let prompt = tensor_from_i64(
            backend,
            &[
                1,
                u64::try_from(prompt_length)
                    .map_err(|_| DecoderTextError::Overflow("prompt token count"))?,
            ],
            &prompt_values,
            context,
        )?;
        let configuration = DecoderGenerationConfiguration {
            maximum_new_tokens: request.maximum_new_tokens,
            temperature_bits: if request.do_sample {
                request.temperature_bits
            } else {
                0.0_f32.to_bits()
            },
            top_k: request
                .do_sample
                .then_some(request.top_k)
                .filter(|value| *value > 0),
            top_p_bits: request
                .do_sample
                .then_some(request.top_p_bits)
                .filter(|value| f32::from_bits(*value) != 1.0),
            minimum_p_bits: request
                .do_sample
                .then_some(request.minimum_p_bits)
                .filter(|value| f32::from_bits(*value) != 0.0),
            repetition_penalty_bits: request.repetition_penalty_bits,
            presence_penalty_bits: request.presence_penalty_bits,
        };
        let outcome = self.generate(backend, &prompt, &configuration, transaction, context)?;
        let generated =
            outcome
                .tokens
                .get(prompt_length..)
                .ok_or(DecoderTextError::InvalidInput(
                    "generated tokens do not retain the prompt prefix",
                ))?;
        let mut generated_tokens = Vec::new();
        generated_tokens
            .try_reserve_exact(generated.len())
            .map_err(|_| DecoderTextError::Allocation("generated token projection"))?;
        for token in generated {
            generated_tokens.push(
                u32::try_from(*token).map_err(|_| DecoderTextError::TokenOutOfRange(*token))?,
            );
        }
        let text = tokenizer.decode_numeric(&generated_tokens, true, context.cancellation)?;
        context.check()?;
        Ok(NativeTextGenerationResult {
            text,
            generated_tokens,
            transaction: outcome.transaction,
        })
    }
}

impl NativeDecoderLayer {
    #[allow(clippy::too_many_arguments)]
    fn forward(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        padding_mask: Option<&Tensor>,
        positions: &ValidatedDecoderPositions,
        batch: usize,
        query_tokens: usize,
        configuration: &DecoderTextConfiguration,
        cache: &mut Option<DecoderLayerCache>,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, DecoderTextError> {
        let attention_input = rms_norm_tensor(
            backend,
            input,
            &self.attention_norm_weight,
            configuration.hidden_size,
            configuration.normalization_epsilon(),
            configuration.norm_weight_offset(),
            context,
        )?;
        let attention = match self.kind {
            DecoderLayerKind::LinearAttention => self.forward_linear_attention(
                backend,
                &attention_input,
                batch,
                query_tokens,
                configuration,
                cache,
                context,
            )?,
            DecoderLayerKind::FullAttention | DecoderLayerKind::SlidingAttention => self
                .forward_attention(
                    backend,
                    &attention_input,
                    padding_mask,
                    positions,
                    batch,
                    query_tokens,
                    configuration,
                    cache,
                    context,
                )?,
        };
        let attention = if let Some(weight) = &self.post_attention_norm_weight {
            rms_norm_tensor(
                backend,
                &attention,
                weight,
                configuration.hidden_size,
                configuration.normalization_epsilon(),
                configuration.norm_weight_offset(),
                context,
            )?
        } else {
            attention
        };
        let residual = add_scaled(
            backend,
            input,
            &attention,
            configuration.residual_scale(),
            context,
        )?;
        let feed_forward_input = rms_norm_tensor(
            backend,
            &residual,
            &self.feed_forward_norm_weight,
            configuration.hidden_size,
            configuration.normalization_epsilon(),
            configuration.norm_weight_offset(),
            context,
        )?;
        let mut gate = self.feed_forward_gate.clone();
        let mut up = self.feed_forward_up.clone();
        let gate = gate.forward_with_context(backend, &feed_forward_input, context)?;
        let up = up.forward_with_context(backend, &feed_forward_input, context)?;
        let mut activation = self.activation.clone();
        let gate = activation.forward_with_context(backend, &gate, context)?;
        let multiplied = multiply_tensor(backend, &gate, &up, context)?;
        let mut down = self.feed_forward_down.clone();
        let feed_forward = down.forward_with_context(backend, &multiplied, context)?;
        let feed_forward = if let Some(weight) = &self.post_feed_forward_norm_weight {
            rms_norm_tensor(
                backend,
                &feed_forward,
                weight,
                configuration.hidden_size,
                configuration.normalization_epsilon(),
                configuration.norm_weight_offset(),
                context,
            )?
        } else {
            feed_forward
        };
        add_scaled(
            backend,
            &residual,
            &feed_forward,
            configuration.residual_scale(),
            context,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn forward_attention(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        padding_mask: Option<&Tensor>,
        positions: &ValidatedDecoderPositions,
        batch: usize,
        query_tokens: usize,
        configuration: &DecoderTextConfiguration,
        cache: &mut Option<DecoderLayerCache>,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, DecoderTextError> {
        let mut query_module = self.query.clone();
        let mut key_module = self.key.clone();
        let mut value_module = self.value.clone();
        let query = query_module.forward_with_context(backend, input, context)?;
        let key = key_module.forward_with_context(backend, input, context)?;
        let value = value_module.forward_with_context(backend, input, context)?;
        let query = tensor_to_f32(backend, &query, context)?;
        let key = tensor_to_f32(backend, &key, context)?;
        let value = tensor_to_f32(backend, &value, context)?;
        let query = apply_decoder_rope(
            &query,
            batch,
            query_tokens,
            configuration.attention_heads,
            configuration.head_dimension,
            positions.rope(),
            &configuration.rope,
            context.cancellation,
        )?;
        let key = apply_decoder_rope(
            &key,
            batch,
            query_tokens,
            configuration.key_value_heads,
            configuration.head_dimension,
            positions.rope(),
            &configuration.rope,
            context.cancellation,
        )?;
        let (keys, values, key_tokens) = stage_attention_cache(
            backend,
            cache,
            batch,
            configuration.key_value_heads,
            configuration.head_dimension,
            &key,
            &value,
            query_tokens,
            configuration.maximum_tokens,
            context,
        )?;
        let keys = expand_grouped_query(
            &keys,
            batch,
            key_tokens,
            configuration.key_value_heads,
            configuration.attention_heads,
            configuration.head_dimension,
            context.cancellation,
        )?;
        let values = expand_grouped_query(
            &values,
            batch,
            key_tokens,
            configuration.key_value_heads,
            configuration.attention_heads,
            configuration.head_dimension,
            context.cancellation,
        )?;
        let sink_values = self
            .attention_sink
            .as_ref()
            .map(|sink| tensor_to_f32(backend, sink, context))
            .transpose()?;
        let (keys, values, attention_key_tokens) = append_attention_sink_tokens(
            &keys,
            &values,
            batch,
            key_tokens,
            configuration.attention_heads,
            configuration.head_dimension,
            sink_values.as_deref(),
            context.cancellation,
        )?;
        let prepared_mask = build_decoder_mask(
            backend,
            padding_mask,
            batch,
            configuration.attention_heads,
            query_tokens,
            key_tokens,
            &positions.causal,
            (self.kind == DecoderLayerKind::SlidingAttention)
                .then_some(configuration.sliding_window)
                .flatten(),
            sink_values.as_deref(),
            context,
        )?;
        let outcome = scaled_dot_product_attention_with_context(
            backend,
            AttentionRequest {
                backend: AttentionBackend::PytorchSdp,
                fallback: AttentionFallbackPolicy::AllowExactNative,
                batch,
                query_tokens,
                key_tokens: attention_key_tokens,
                heads: configuration.attention_heads,
                head_dimension: configuration.head_dimension,
                value_dimension: configuration.head_dimension,
                scale: None,
                workspace_limit_bytes: attention_key_tokens
                    .checked_mul(std::mem::size_of::<f32>())
                    .ok_or(DecoderTextError::Overflow("attention workspace"))?,
            },
            &query,
            &keys,
            &values,
            Some(prepared_mask.as_attention_mask()),
            context,
        )?;
        let attention = tensor_from_f32(
            backend,
            &[
                usize_to_u64(batch, "attention batch")?,
                usize_to_u64(query_tokens, "attention tokens")?,
                usize_to_u64(configuration.hidden_size, "attention hidden")?,
            ],
            &outcome.values,
            context,
        )?;
        let mut output = self.attention_output.clone();
        output
            .forward_with_context(backend, &attention, context)
            .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    fn forward_linear_attention(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        batch: usize,
        query_tokens: usize,
        configuration: &DecoderTextConfiguration,
        cache: &mut Option<DecoderLayerCache>,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, DecoderTextError> {
        let mut query = self.query.clone();
        let mut key = self.key.clone();
        let mut value = self.value.clone();
        let query = tensor_to_f32(
            backend,
            &query.forward_with_context(backend, input, context)?,
            context,
        )?;
        let key = tensor_to_f32(
            backend,
            &key.forward_with_context(backend, input, context)?,
            context,
        )?;
        let value = tensor_to_f32(
            backend,
            &value.forward_with_context(backend, input, context)?,
            context,
        )?;
        let state_width = configuration
            .attention_heads
            .checked_mul(configuration.head_dimension)
            .ok_or(DecoderTextError::Overflow("linear-attention state"))?;
        let previous = match cache.as_ref() {
            Some(DecoderLayerCache::Linear(state)) => Some(state),
            None => None,
            Some(DecoderLayerCache::Attention(_)) => {
                return Err(DecoderTextError::InvalidInput(
                    "attention cache kind does not match the Qwen3.5 layer",
                ));
            }
        };
        let recurrent = previous
            .map(|state| tensor_to_f32(backend, &state.recurrent_state, context))
            .transpose()?;
        let gate_count = batch
            .checked_mul(query_tokens)
            .and_then(|value| value.checked_mul(configuration.attention_heads))
            .ok_or(DecoderTextError::Overflow("linear-attention gates"))?;
        let log_decay = vec![0.0; gate_count];
        let beta = vec![1.0; gate_count];
        let (output, next_recurrent) = qwen35_chunk_gated_delta_rule_exact(
            &query,
            &key,
            &value,
            &log_decay,
            &beta,
            recurrent.as_deref().unwrap_or(&[]),
            batch,
            query_tokens,
            configuration.attention_heads,
            configuration.head_dimension,
            configuration.head_dimension,
            context.cancellation,
        )?;
        let previous_convolution = previous
            .map(|state| tensor_to_f32(backend, &state.convolution_state, context))
            .transpose()?;
        let mut convolution_weight = Vec::new();
        convolution_weight
            .try_reserve_exact(
                state_width
                    .checked_mul(3)
                    .ok_or(DecoderTextError::Overflow("linear convolution weights"))?,
            )
            .map_err(|_| DecoderTextError::Allocation("linear convolution weights"))?;
        for _ in 0..state_width {
            convolution_weight.extend_from_slice(&[0.25, 0.5, 0.25]);
        }
        let (output, convolution_state) = qwen35_causal_conv1d_update_exact(
            &output,
            previous_convolution.as_deref().unwrap_or(&[]),
            &convolution_weight,
            None,
            batch,
            query_tokens,
            state_width,
            3,
            context.cancellation,
        )?;
        let convolution_state = tensor_from_f32(
            backend,
            &[
                usize_to_u64(batch, "linear convolution batch")?,
                usize_to_u64(state_width, "linear convolution width")?,
                2,
            ],
            &convolution_state,
            context,
        )?;
        let next_recurrent = tensor_from_f32(
            backend,
            &[
                usize_to_u64(batch, "linear recurrent batch")?,
                usize_to_u64(configuration.attention_heads, "linear recurrent heads")?,
                usize_to_u64(configuration.head_dimension, "linear recurrent key width")?,
                usize_to_u64(configuration.head_dimension, "linear recurrent value width")?,
            ],
            &next_recurrent,
            context,
        )?;
        *cache = Some(DecoderLayerCache::Linear(Qwen35LinearCache {
            convolution_state,
            recurrent_state: next_recurrent,
        }));
        let output = tensor_from_f32(
            backend,
            &[
                usize_to_u64(batch, "linear-attention batch")?,
                usize_to_u64(query_tokens, "linear-attention tokens")?,
                usize_to_u64(configuration.hidden_size, "linear-attention hidden")?,
            ],
            &output,
            context,
        )?;
        let mut projection = self.attention_output.clone();
        projection
            .forward_with_context(backend, &output, context)
            .map_err(Into::into)
    }
}

struct PreparedDecoderMask {
    values: CpuWorkspaceVec<f32>,
}

impl PreparedDecoderMask {
    fn as_attention_mask(&self) -> AttentionMask<'_> {
        AttentionMask::Additive {
            values: &self.values,
            shape: AttentionMaskShape::BatchHeadQueryByKey,
        }
    }
}

pub fn precompute_rope(
    positions: &[usize],
    configuration: &DecoderRopeConfiguration,
    cancellation: &CancellationToken,
) -> Result<Vec<[f32; 2]>, DecoderTextError> {
    if configuration.rotary_dimension == 0
        || !configuration.rotary_dimension.is_multiple_of(2)
        || !configuration.theta.is_finite()
        || configuration.theta <= 0.0
    {
        return Err(DecoderTextError::InvalidConfiguration(
            "RoPE dimensions or theta are invalid",
        ));
    }
    let pairs = configuration.rotary_dimension / 2;
    let count = positions
        .len()
        .checked_mul(pairs)
        .ok_or(DecoderTextError::Overflow("RoPE table"))?;
    let mut table = Vec::new();
    table
        .try_reserve_exact(count)
        .map_err(|_| DecoderTextError::Allocation("RoPE table"))?;
    for (position_index, position) in positions.iter().copied().enumerate() {
        for pair in 0..pairs {
            if (position_index * pairs + pair).is_multiple_of(256) {
                cancellation.check()?;
            }
            let exponent = 2.0 * pair as f32 / configuration.rotary_dimension as f32;
            let mut frequency = configuration.theta.powf(-exponent);
            let mut scaled_position = position as f32;
            match configuration.scaling {
                RopeScaling::None => {}
                RopeScaling::Linear { factor } => scaled_position /= factor,
                RopeScaling::Yarn {
                    factor,
                    beta_fast,
                    beta_slow,
                } => {
                    let progress = if pairs <= 1 {
                        0.0
                    } else {
                        pair as f32 / (pairs - 1) as f32
                    };
                    let ramp = ((progress * beta_fast - beta_slow) / (beta_fast - beta_slow))
                        .clamp(0.0, 1.0);
                    frequency *= (1.0 - ramp) / factor + ramp;
                }
            }
            let angle = scaled_position * frequency;
            table.push([angle.cos(), angle.sin()]);
        }
    }
    Ok(table)
}

pub fn precompute_multidimensional_rope(
    position_axes: &[Vec<usize>],
    configuration: &DecoderRopeConfiguration,
    cancellation: &CancellationToken,
) -> Result<Vec<[f32; 2]>, DecoderTextError> {
    if configuration.interleaved_sections.is_empty()
        || position_axes.len() != configuration.interleaved_sections.len()
        || position_axes.is_empty()
    {
        return Err(DecoderTextError::InvalidInput(
            "multidimensional RoPE axes do not match the configured sections",
        ));
    }
    let tokens = position_axes
        .first()
        .map(Vec::len)
        .ok_or(DecoderTextError::InvalidInput("RoPE axes are empty"))?;
    if tokens == 0 || position_axes.iter().any(|axis| axis.len() != tokens) {
        return Err(DecoderTextError::InvalidInput(
            "multidimensional RoPE axes must have one equal nonzero token length",
        ));
    }
    let pairs = configuration.rotary_dimension / 2;
    let section_total = configuration
        .interleaved_sections
        .iter()
        .try_fold(0_usize, |sum, section| sum.checked_add(*section));
    if section_total != Some(pairs) {
        return Err(DecoderTextError::InvalidConfiguration(
            "multidimensional RoPE sections do not cover every rotary pair",
        ));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(
            tokens
                .checked_mul(pairs)
                .ok_or(DecoderTextError::Overflow("multidimensional RoPE"))?,
        )
        .map_err(|_| DecoderTextError::Allocation("multidimensional RoPE"))?;
    for token in 0..tokens {
        let mut section_start = 0_usize;
        for (axis_index, section) in configuration
            .interleaved_sections
            .iter()
            .copied()
            .enumerate()
        {
            let position = *position_axes
                .get(axis_index)
                .and_then(|axis| axis.get(token))
                .ok_or(DecoderTextError::InvalidInput(
                    "RoPE axis position is missing",
                ))?;
            for local_pair in 0..section {
                cancellation.check()?;
                let pair = section_start
                    .checked_add(local_pair)
                    .ok_or(DecoderTextError::Overflow("RoPE section pair"))?;
                let exponent = 2.0 * pair as f32 / configuration.rotary_dimension as f32;
                let mut frequency = configuration.theta.powf(-exponent);
                let mut scaled_position = position as f32;
                match configuration.scaling {
                    RopeScaling::None => {}
                    RopeScaling::Linear { factor } => scaled_position /= factor,
                    RopeScaling::Yarn {
                        factor,
                        beta_fast,
                        beta_slow,
                    } => {
                        let progress = if pairs <= 1 {
                            0.0
                        } else {
                            pair as f32 / (pairs - 1) as f32
                        };
                        let ramp = ((progress * beta_fast - beta_slow) / (beta_fast - beta_slow))
                            .clamp(0.0, 1.0);
                        frequency *= (1.0 - ramp) / factor + ramp;
                    }
                }
                let angle = scaled_position * frequency;
                output.push([angle.cos(), angle.sin()]);
            }
            section_start = section_start
                .checked_add(section)
                .ok_or(DecoderTextError::Overflow("RoPE section"))?;
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub fn apply_rope(
    values: &[f32],
    batch: usize,
    tokens: usize,
    heads: usize,
    head_dimension: usize,
    positions: &[usize],
    configuration: &DecoderRopeConfiguration,
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, DecoderTextError> {
    if positions.len() != tokens || configuration.rotary_dimension > head_dimension {
        return Err(DecoderTextError::InvalidInput(
            "RoPE positions or rotary width are invalid",
        ));
    }
    let expected = batch
        .checked_mul(tokens)
        .and_then(|value| value.checked_mul(heads))
        .and_then(|value| value.checked_mul(head_dimension))
        .ok_or(DecoderTextError::Overflow("RoPE input"))?;
    if values.len() != expected {
        return Err(DecoderTextError::InvalidInput(
            "RoPE input length does not match its dimensions",
        ));
    }
    let table = precompute_rope(positions, configuration, cancellation)?;
    let pairs = configuration.rotary_dimension / 2;
    let mut output = values.to_vec();
    for batch_index in 0..batch {
        for token in 0..tokens {
            for head in 0..heads {
                for pair in 0..pairs {
                    let work = (((batch_index * tokens + token) * heads + head) * pairs) + pair;
                    if work.is_multiple_of(256) {
                        cancellation.check()?;
                    }
                    let base = ((batch_index * tokens + token) * heads + head) * head_dimension;
                    let left_index = base + pair;
                    let right_index = base + pair + pairs;
                    let left = *values
                        .get(left_index)
                        .ok_or(DecoderTextError::InvalidInput(
                            "RoPE left component is missing",
                        ))?;
                    let right = *values
                        .get(right_index)
                        .ok_or(DecoderTextError::InvalidInput(
                            "RoPE right component is missing",
                        ))?;
                    let [cosine, sine] =
                        *table
                            .get(token * pairs + pair)
                            .ok_or(DecoderTextError::InvalidInput(
                                "RoPE table entry is missing",
                            ))?;
                    let rotated_left = left * cosine - right * sine;
                    let rotated_right = right * cosine + left * sine;
                    *output
                        .get_mut(left_index)
                        .ok_or(DecoderTextError::InvalidInput(
                            "RoPE output left component is missing",
                        ))? = rotated_left;
                    *output
                        .get_mut(right_index)
                        .ok_or(DecoderTextError::InvalidInput(
                            "RoPE output right component is missing",
                        ))? = rotated_right;
                }
            }
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn apply_decoder_rope(
    values: &[f32],
    batch: usize,
    tokens: usize,
    heads: usize,
    head_dimension: usize,
    positions: DecoderRopePositions<'_>,
    configuration: &DecoderRopeConfiguration,
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, DecoderTextError> {
    let table = match positions {
        DecoderRopePositions::Scalar(positions) => {
            if positions.len() != tokens {
                return Err(DecoderTextError::InvalidInput(
                    "RoPE position count must equal query token count",
                ));
            }
            precompute_rope(positions, configuration, cancellation)?
        }
        DecoderRopePositions::Multidimensional(position_axes) => {
            if position_axes.iter().any(|axis| axis.len() != tokens) {
                return Err(DecoderTextError::InvalidInput(
                    "multidimensional RoPE position count must equal query token count",
                ));
            }
            precompute_multidimensional_rope(position_axes, configuration, cancellation)?
        }
    };
    apply_rope_table(
        values,
        batch,
        tokens,
        heads,
        head_dimension,
        &table,
        configuration,
        cancellation,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_rope_table(
    values: &[f32],
    batch: usize,
    tokens: usize,
    heads: usize,
    head_dimension: usize,
    table: &[[f32; 2]],
    configuration: &DecoderRopeConfiguration,
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, DecoderTextError> {
    if configuration.rotary_dimension > head_dimension {
        return Err(DecoderTextError::InvalidInput(
            "RoPE rotary width exceeds the attention head width",
        ));
    }
    let expected = batch
        .checked_mul(tokens)
        .and_then(|value| value.checked_mul(heads))
        .and_then(|value| value.checked_mul(head_dimension))
        .ok_or(DecoderTextError::Overflow("RoPE input"))?;
    if values.len() != expected {
        return Err(DecoderTextError::InvalidInput(
            "RoPE input length does not match its dimensions",
        ));
    }
    let pairs = configuration.rotary_dimension / 2;
    if table.len() != tokens.saturating_mul(pairs) {
        return Err(DecoderTextError::InvalidInput(
            "RoPE table length does not match the query tokens",
        ));
    }
    let mut output = values.to_vec();
    for batch_index in 0..batch {
        for token in 0..tokens {
            for head in 0..heads {
                for pair in 0..pairs {
                    let work = (((batch_index * tokens + token) * heads + head) * pairs) + pair;
                    if work.is_multiple_of(256) {
                        cancellation.check()?;
                    }
                    let base = ((batch_index * tokens + token) * heads + head) * head_dimension;
                    let left_index = base + pair;
                    let right_index = base + pair + pairs;
                    let left = *values
                        .get(left_index)
                        .ok_or(DecoderTextError::InvalidInput(
                            "RoPE left component is missing",
                        ))?;
                    let right = *values
                        .get(right_index)
                        .ok_or(DecoderTextError::InvalidInput(
                            "RoPE right component is missing",
                        ))?;
                    let [cosine, sine] =
                        *table
                            .get(token * pairs + pair)
                            .ok_or(DecoderTextError::InvalidInput(
                                "RoPE table entry is missing",
                            ))?;
                    output[left_index] = left * cosine - right * sine;
                    output[right_index] = right * cosine + left * sine;
                }
            }
        }
    }
    Ok(output)
}

pub fn gpt_oss_top_k_route(
    router_logits: &[f32],
    tokens: usize,
    experts: usize,
    top_k: usize,
    cancellation: &CancellationToken,
) -> Result<Vec<Vec<(usize, f32)>>, DecoderTextError> {
    if tokens == 0 || experts == 0 || top_k == 0 || top_k > experts {
        return Err(DecoderTextError::InvalidInput(
            "GPT-OSS router dimensions are invalid",
        ));
    }
    if router_logits.len()
        != tokens
            .checked_mul(experts)
            .ok_or(DecoderTextError::Overflow("router logits"))?
    {
        return Err(DecoderTextError::InvalidInput(
            "GPT-OSS router logits have the wrong length",
        ));
    }
    let mut routes = Vec::new();
    routes
        .try_reserve_exact(tokens)
        .map_err(|_| DecoderTextError::Allocation("GPT-OSS routes"))?;
    for token in 0..tokens {
        cancellation.check()?;
        let start = token * experts;
        let logits =
            router_logits
                .get(start..start + experts)
                .ok_or(DecoderTextError::InvalidInput(
                    "router token logits are missing",
                ))?;
        if logits.iter().any(|value| !value.is_finite()) {
            return Err(DecoderTextError::InvalidInput(
                "GPT-OSS router logits must be finite",
            ));
        }
        let mut indices = (0..experts).collect::<Vec<_>>();
        indices.sort_by(|left, right| {
            logits[*right]
                .partial_cmp(&logits[*left])
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.cmp(right))
        });
        indices.truncate(top_k);
        let maximum = indices
            .iter()
            .filter_map(|index| logits.get(*index))
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        let denominator = indices
            .iter()
            .filter_map(|index| logits.get(*index))
            .map(|value| (*value - maximum).exp())
            .sum::<f32>();
        if !denominator.is_finite() || denominator <= 0.0 {
            return Err(DecoderTextError::InvalidInput(
                "GPT-OSS router normalization is invalid",
            ));
        }
        routes.push(
            indices
                .into_iter()
                .map(|index| (index, (logits[index] - maximum).exp() / denominator))
                .collect(),
        );
    }
    Ok(routes)
}

#[allow(clippy::too_many_arguments)]
pub fn gpt_oss_moe(
    input: &[f32],
    router_logits: &[f32],
    gate_weights: &[f32],
    up_weights: &[f32],
    down_weights: &[f32],
    tokens: usize,
    hidden: usize,
    intermediate: usize,
    experts: usize,
    top_k: usize,
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, DecoderTextError> {
    let input_count = tokens
        .checked_mul(hidden)
        .ok_or(DecoderTextError::Overflow("MoE input"))?;
    let first_weight_count = experts
        .checked_mul(intermediate)
        .and_then(|value| value.checked_mul(hidden))
        .ok_or(DecoderTextError::Overflow("MoE input weights"))?;
    let down_weight_count = experts
        .checked_mul(hidden)
        .and_then(|value| value.checked_mul(intermediate))
        .ok_or(DecoderTextError::Overflow("MoE down weights"))?;
    if input.len() != input_count
        || gate_weights.len() != first_weight_count
        || up_weights.len() != first_weight_count
        || down_weights.len() != down_weight_count
    {
        return Err(DecoderTextError::InvalidInput(
            "GPT-OSS expert tensor lengths are invalid",
        ));
    }
    let routes = gpt_oss_top_k_route(router_logits, tokens, experts, top_k, cancellation)?;
    let mut output = vec![0.0; input_count];
    for token in 0..tokens {
        for (expert, route_weight) in routes
            .get(token)
            .ok_or(DecoderTextError::InvalidInput("expert route is missing"))?
        {
            cancellation.check()?;
            for inner in 0..intermediate {
                let mut gate = 0.0;
                let mut up = 0.0;
                for column in 0..hidden {
                    let input_value = *input
                        .get(token * hidden + column)
                        .ok_or(DecoderTextError::InvalidInput("MoE input value is missing"))?;
                    let weight_index = (expert * intermediate + inner) * hidden + column;
                    gate += input_value
                        * gate_weights
                            .get(weight_index)
                            .ok_or(DecoderTextError::InvalidInput("MoE gate weight is missing"))?;
                    up += input_value
                        * up_weights
                            .get(weight_index)
                            .ok_or(DecoderTextError::InvalidInput("MoE up weight is missing"))?;
                }
                let clipped_gate = gate.min(7.0);
                let glu = clipped_gate / (1.0 + (-(clipped_gate * 1.702)).exp());
                let expert_value = glu + up.clamp(-7.0, 7.0) * glu;
                for row in 0..hidden {
                    let weight_index = (expert * hidden + row) * intermediate + inner;
                    let destination = output
                        .get_mut(token * hidden + row)
                        .ok_or(DecoderTextError::InvalidInput("MoE output is missing"))?;
                    *destination += route_weight
                        * expert_value
                        * down_weights
                            .get(weight_index)
                            .ok_or(DecoderTextError::InvalidInput("MoE down weight is missing"))?;
                }
            }
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub fn qwen35_chunk_gated_delta_rule(
    query: &[f32],
    key: &[f32],
    value: &[f32],
    previous_state: &[f32],
    batch: usize,
    tokens: usize,
    width: usize,
    cancellation: &CancellationToken,
) -> Result<(Vec<f32>, Vec<f32>), DecoderTextError> {
    let count = batch
        .checked_mul(tokens)
        .and_then(|value| value.checked_mul(width))
        .ok_or(DecoderTextError::Overflow("Qwen3.5 gated delta input"))?;
    let state_count = batch
        .checked_mul(width)
        .ok_or(DecoderTextError::Overflow("Qwen3.5 gated delta state"))?;
    if query.len() != count || key.len() != count || value.len() != count {
        return Err(DecoderTextError::InvalidInput(
            "Qwen3.5 gated-delta tensors have the wrong length",
        ));
    }
    if !previous_state.is_empty() && previous_state.len() != state_count {
        return Err(DecoderTextError::InvalidInput(
            "Qwen3.5 recurrent state has the wrong length",
        ));
    }
    let mut state = if previous_state.is_empty() {
        vec![0.0; state_count]
    } else {
        previous_state.to_vec()
    };
    let mut output = vec![0.0; count];
    for batch_index in 0..batch {
        for token in 0..tokens {
            for column in 0..width {
                let index = (batch_index * tokens + token) * width + column;
                if index.is_multiple_of(256) {
                    cancellation.check()?;
                }
                let state_index = batch_index * width + column;
                let query_value = *query
                    .get(index)
                    .ok_or(DecoderTextError::InvalidInput("delta query is missing"))?;
                let key_value = *key
                    .get(index)
                    .ok_or(DecoderTextError::InvalidInput("delta key is missing"))?;
                let value_value = *value
                    .get(index)
                    .ok_or(DecoderTextError::InvalidInput("delta value is missing"))?;
                let prior = *state
                    .get(state_index)
                    .ok_or(DecoderTextError::InvalidInput("delta state is missing"))?;
                let gate = 1.0 / (1.0 + (-(query_value * key_value)).exp());
                let next = prior + gate * (value_value - prior);
                *state
                    .get_mut(state_index)
                    .ok_or(DecoderTextError::InvalidInput(
                        "delta state output is missing",
                    ))? = next;
                *output
                    .get_mut(index)
                    .ok_or(DecoderTextError::InvalidInput("delta output is missing"))? =
                    next * query_value;
            }
        }
    }
    Ok((output, state))
}

#[allow(clippy::too_many_arguments)]
pub fn qwen35_chunk_gated_delta_rule_exact(
    query: &[f32],
    key: &[f32],
    value: &[f32],
    log_decay: &[f32],
    beta: &[f32],
    previous_state: &[f32],
    batch: usize,
    tokens: usize,
    heads: usize,
    key_dimension: usize,
    value_dimension: usize,
    cancellation: &CancellationToken,
) -> Result<(Vec<f32>, Vec<f32>), DecoderTextError> {
    if batch == 0 || tokens == 0 || heads == 0 || key_dimension == 0 || value_dimension == 0 {
        return Err(DecoderTextError::InvalidInput(
            "Qwen3.5 exact gated-delta dimensions are invalid",
        ));
    }
    let query_count = batch
        .checked_mul(tokens)
        .and_then(|value| value.checked_mul(heads))
        .and_then(|value| value.checked_mul(key_dimension))
        .ok_or(DecoderTextError::Overflow("exact gated-delta query"))?;
    let value_count = batch
        .checked_mul(tokens)
        .and_then(|value| value.checked_mul(heads))
        .and_then(|value| value.checked_mul(value_dimension))
        .ok_or(DecoderTextError::Overflow("exact gated-delta value"))?;
    let gate_count = batch
        .checked_mul(tokens)
        .and_then(|value| value.checked_mul(heads))
        .ok_or(DecoderTextError::Overflow("exact gated-delta gates"))?;
    let state_count = batch
        .checked_mul(heads)
        .and_then(|value| value.checked_mul(key_dimension))
        .and_then(|value| value.checked_mul(value_dimension))
        .ok_or(DecoderTextError::Overflow("exact gated-delta state"))?;
    if query.len() != query_count
        || key.len() != query_count
        || value.len() != value_count
        || log_decay.len() != gate_count
        || beta.len() != gate_count
        || (!previous_state.is_empty() && previous_state.len() != state_count)
        || log_decay.iter().any(|value| !value.is_finite())
        || beta
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0 || *value > 1.0)
    {
        return Err(DecoderTextError::InvalidInput(
            "Qwen3.5 exact gated-delta payload lengths or gates are invalid",
        ));
    }
    let mut state = if previous_state.is_empty() {
        vec![0.0; state_count]
    } else {
        previous_state.to_vec()
    };
    let mut output = vec![0.0; value_count];
    let scale = (key_dimension as f32).sqrt().recip();
    let mut normalized_query = vec![0.0; key_dimension];
    let mut normalized_key = vec![0.0; key_dimension];
    let mut delta = vec![0.0; value_dimension];
    for batch_index in 0..batch {
        for token in 0..tokens {
            for head in 0..heads {
                cancellation.check()?;
                let vector_base = ((batch_index * tokens + token) * heads + head) * key_dimension;
                let value_base = ((batch_index * tokens + token) * heads + head) * value_dimension;
                let gate_index = (batch_index * tokens + token) * heads + head;
                let query_norm = query
                    .get(vector_base..vector_base + key_dimension)
                    .ok_or(DecoderTextError::InvalidInput(
                        "exact gated-delta query vector is missing",
                    ))?
                    .iter()
                    .map(|value| value * value)
                    .sum::<f32>()
                    .sqrt()
                    .max(1.0e-12);
                let key_norm = key
                    .get(vector_base..vector_base + key_dimension)
                    .ok_or(DecoderTextError::InvalidInput(
                        "exact gated-delta key vector is missing",
                    ))?
                    .iter()
                    .map(|value| value * value)
                    .sum::<f32>()
                    .sqrt()
                    .max(1.0e-12);
                for column in 0..key_dimension {
                    normalized_query[column] = query[vector_base + column] / query_norm * scale;
                    normalized_key[column] = key[vector_base + column] / key_norm;
                }
                let decay = log_decay[gate_index].exp();
                let update_strength = beta[gate_index];
                let state_base = (batch_index * heads + head) * key_dimension * value_dimension;
                for key_column in 0..key_dimension {
                    for value_column in 0..value_dimension {
                        let state_index = state_base + key_column * value_dimension + value_column;
                        state[state_index] *= decay;
                    }
                }
                for value_column in 0..value_dimension {
                    let prediction = (0..key_dimension)
                        .map(|key_column| {
                            normalized_key[key_column]
                                * state[state_base + key_column * value_dimension + value_column]
                        })
                        .sum::<f32>();
                    delta[value_column] =
                        update_strength * (value[value_base + value_column] - prediction);
                }
                for key_column in 0..key_dimension {
                    for value_column in 0..value_dimension {
                        let state_index = state_base + key_column * value_dimension + value_column;
                        state[state_index] += normalized_key[key_column] * delta[value_column];
                    }
                }
                for value_column in 0..value_dimension {
                    output[value_base + value_column] = (0..key_dimension)
                        .map(|key_column| {
                            normalized_query[key_column]
                                * state[state_base + key_column * value_dimension + value_column]
                        })
                        .sum();
                }
            }
        }
    }
    Ok((output, state))
}

#[allow(clippy::too_many_arguments)]
pub fn qwen35_causal_conv1d_update_exact(
    input: &[f32],
    previous_state: &[f32],
    weight: &[f32],
    bias: Option<&[f32]>,
    batch: usize,
    tokens: usize,
    channels: usize,
    kernel_size: usize,
    cancellation: &CancellationToken,
) -> Result<(Vec<f32>, Vec<f32>), DecoderTextError> {
    if batch == 0 || tokens == 0 || channels == 0 || kernel_size == 0 {
        return Err(DecoderTextError::InvalidInput(
            "Qwen3.5 exact causal-convolution dimensions are invalid",
        ));
    }
    let input_count = batch
        .checked_mul(tokens)
        .and_then(|value| value.checked_mul(channels))
        .ok_or(DecoderTextError::Overflow("exact causal-convolution input"))?;
    let history = kernel_size.saturating_sub(1);
    let state_count = batch
        .checked_mul(channels)
        .and_then(|value| value.checked_mul(history))
        .ok_or(DecoderTextError::Overflow("exact causal-convolution state"))?;
    if input.len() != input_count
        || weight.len()
            != channels
                .checked_mul(kernel_size)
                .ok_or(DecoderTextError::Overflow(
                    "exact causal-convolution weight",
                ))?
        || bias.is_some_and(|bias| bias.len() != channels)
        || (!previous_state.is_empty() && previous_state.len() != state_count)
    {
        return Err(DecoderTextError::InvalidInput(
            "Qwen3.5 exact causal-convolution payload lengths are invalid",
        ));
    }
    let mut state = if previous_state.is_empty() {
        vec![0.0; state_count]
    } else {
        previous_state.to_vec()
    };
    let mut output = vec![0.0; input_count];
    let mut window = vec![0.0; kernel_size];
    for batch_index in 0..batch {
        for token in 0..tokens {
            for channel in 0..channels {
                cancellation.check()?;
                for history_index in 0..history {
                    window[history_index] =
                        state[(batch_index * channels + channel) * history + history_index];
                }
                window[history] = input[(batch_index * tokens + token) * channels + channel];
                let mut value = bias
                    .and_then(|bias| bias.get(channel))
                    .copied()
                    .unwrap_or(0.0);
                for kernel_index in 0..kernel_size {
                    value += window[kernel_index] * weight[channel * kernel_size + kernel_index];
                }
                output[(batch_index * tokens + token) * channels + channel] =
                    value / (1.0 + (-value).exp());
                for history_index in 0..history {
                    state[(batch_index * channels + channel) * history + history_index] =
                        window[history_index + 1];
                }
            }
        }
    }
    Ok((output, state))
}

#[allow(clippy::too_many_arguments)]
pub fn qwen35_causal_conv1d_update(
    input: &[f32],
    previous_state: &[f32],
    batch: usize,
    tokens: usize,
    channels: usize,
    kernel: &[f32],
    cancellation: &CancellationToken,
) -> Result<(Vec<f32>, Vec<f32>), DecoderTextError> {
    if kernel.is_empty() || kernel.iter().any(|weight| !weight.is_finite()) {
        return Err(DecoderTextError::InvalidInput(
            "Qwen3.5 causal-convolution kernel is invalid",
        ));
    }
    let input_count = batch
        .checked_mul(tokens)
        .and_then(|value| value.checked_mul(channels))
        .ok_or(DecoderTextError::Overflow("Qwen3.5 causal convolution"))?;
    let history = kernel.len().saturating_sub(1);
    let state_count = batch
        .checked_mul(history)
        .and_then(|value| value.checked_mul(channels))
        .ok_or(DecoderTextError::Overflow("Qwen3.5 convolution state"))?;
    if input.len() != input_count
        || (!previous_state.is_empty() && previous_state.len() != state_count)
    {
        return Err(DecoderTextError::InvalidInput(
            "Qwen3.5 convolution input or state has the wrong length",
        ));
    }
    let mut combined = vec![0.0; batch * (history + tokens) * channels];
    for batch_index in 0..batch {
        for history_index in 0..history {
            for channel in 0..channels {
                let destination =
                    (batch_index * (history + tokens) + history_index) * channels + channel;
                if !previous_state.is_empty() {
                    let source = (batch_index * history + history_index) * channels + channel;
                    combined[destination] = previous_state[source];
                }
            }
        }
        for token in 0..tokens {
            for channel in 0..channels {
                let source = (batch_index * tokens + token) * channels + channel;
                let destination =
                    (batch_index * (history + tokens) + history + token) * channels + channel;
                combined[destination] = input[source];
            }
        }
    }
    let mut output = vec![0.0; input_count];
    for batch_index in 0..batch {
        for token in 0..tokens {
            for channel in 0..channels {
                let work = (batch_index * tokens + token) * channels + channel;
                if work.is_multiple_of(256) {
                    cancellation.check()?;
                }
                let mut value = 0.0;
                for kernel_index in 0..kernel.len() {
                    let source_token = token + kernel_index;
                    let source =
                        (batch_index * (history + tokens) + source_token) * channels + channel;
                    value += combined[source] * kernel[kernel_index];
                }
                output[work] = value;
            }
        }
    }
    let mut next_state = vec![0.0; state_count];
    for batch_index in 0..batch {
        for history_index in 0..history {
            for channel in 0..channels {
                let source_token = tokens + history_index;
                let source = (batch_index * (history + tokens) + source_token) * channels + channel;
                let destination = (batch_index * history + history_index) * channels + channel;
                next_state[destination] = combined[source];
            }
        }
    }
    Ok((output, next_state))
}

pub fn gemma4_vision_rope(
    rows: usize,
    columns: usize,
    dimension: usize,
    theta: f32,
    cancellation: &CancellationToken,
) -> Result<Vec<[f32; 2]>, DecoderTextError> {
    if rows == 0 || columns == 0 || dimension == 0 || !dimension.is_multiple_of(4) {
        return Err(DecoderTextError::InvalidInput(
            "Gemma4 vision RoPE dimensions are invalid",
        ));
    }
    let half = dimension / 2;
    let row_configuration = DecoderRopeConfiguration {
        theta,
        rotary_dimension: half,
        interleaved_sections: Vec::new(),
        scaling: RopeScaling::None,
    };
    let row_positions = (0..rows).collect::<Vec<_>>();
    let column_positions = (0..columns).collect::<Vec<_>>();
    let row_table = precompute_rope(&row_positions, &row_configuration, cancellation)?;
    let column_table = precompute_rope(&column_positions, &row_configuration, cancellation)?;
    let pair_count = dimension / 2;
    let mut output = Vec::new();
    output
        .try_reserve_exact(rows * columns * pair_count)
        .map_err(|_| DecoderTextError::Allocation("Gemma4 vision RoPE"))?;
    for row in 0..rows {
        for column in 0..columns {
            for pair in 0..pair_count {
                cancellation.check()?;
                let table = if pair < pair_count / 2 {
                    &row_table
                } else {
                    &column_table
                };
                let position = if pair < pair_count / 2 { row } else { column };
                let local_pair = pair % (pair_count / 2);
                output.push(*table.get(position * (pair_count / 2) + local_pair).ok_or(
                    DecoderTextError::InvalidInput("Gemma4 vision RoPE entry is missing"),
                )?);
            }
        }
    }
    Ok(output)
}

pub fn gemma4_audio_relative_positions(
    query_tokens: usize,
    key_tokens: usize,
    maximum_distance: usize,
) -> Result<Vec<usize>, DecoderTextError> {
    if query_tokens == 0 || key_tokens == 0 || maximum_distance == 0 {
        return Err(DecoderTextError::InvalidInput(
            "Gemma4 audio relative-position dimensions are invalid",
        ));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(
            query_tokens
                .checked_mul(key_tokens)
                .ok_or(DecoderTextError::Overflow("audio relative positions"))?,
        )
        .map_err(|_| DecoderTextError::Allocation("audio relative positions"))?;
    for query in 0..query_tokens {
        for key in 0..key_tokens {
            let distance = key as i128 - query as i128;
            let clipped = distance.clamp(-(maximum_distance as i128), maximum_distance as i128);
            output.push(
                usize::try_from(clipped + maximum_distance as i128)
                    .map_err(|_| DecoderTextError::Overflow("audio relative position"))?,
            );
        }
    }
    Ok(output)
}

pub fn tokenize_decoder_prompt(
    tokenizer: &NativePromptTokenizer,
    text: &str,
    cancellation: &CancellationToken,
) -> Result<NativeTokenizedPrompt, DecoderTextError> {
    tokenizer
        .tokenize(text, cancellation)
        .map_err(DecoderTextError::from)
}

#[allow(clippy::too_many_arguments)]
pub fn gemma4_clipped_linear(
    input: &[f32],
    weight: &[f32],
    bias: Option<&[f32]>,
    rows: usize,
    input_width: usize,
    output_width: usize,
    input_bounds: [f32; 2],
    output_bounds: [f32; 2],
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, DecoderTextError> {
    if rows == 0
        || input_width == 0
        || output_width == 0
        || input_bounds.iter().any(|value| value.is_nan())
        || output_bounds.iter().any(|value| value.is_nan())
        || input_bounds[0] > input_bounds[1]
        || output_bounds[0] > output_bounds[1]
    {
        return Err(DecoderTextError::InvalidInput(
            "Gemma4 clipped-linear dimensions or bounds are invalid",
        ));
    }
    let input_count = rows
        .checked_mul(input_width)
        .ok_or(DecoderTextError::Overflow("clipped-linear input"))?;
    let weight_count = output_width
        .checked_mul(input_width)
        .ok_or(DecoderTextError::Overflow("clipped-linear weight"))?;
    if input.len() != input_count
        || weight.len() != weight_count
        || bias.is_some_and(|bias| bias.len() != output_width)
    {
        return Err(DecoderTextError::InvalidInput(
            "Gemma4 clipped-linear payload lengths are invalid",
        ));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(
            rows.checked_mul(output_width)
                .ok_or(DecoderTextError::Overflow("clipped-linear output"))?,
        )
        .map_err(|_| DecoderTextError::Allocation("clipped-linear output"))?;
    for row in 0..rows {
        for output_column in 0..output_width {
            cancellation.check()?;
            let mut value = bias
                .and_then(|bias| bias.get(output_column))
                .copied()
                .unwrap_or(0.0);
            for input_column in 0..input_width {
                let input_value = input
                    .get(row * input_width + input_column)
                    .ok_or(DecoderTextError::InvalidInput(
                        "clipped-linear input value is missing",
                    ))?
                    .clamp(input_bounds[0], input_bounds[1]);
                let weight_value = *weight
                    .get(output_column * input_width + input_column)
                    .ok_or(DecoderTextError::InvalidInput(
                        "clipped-linear weight value is missing",
                    ))?;
                value += input_value * weight_value;
            }
            output.push(value.clamp(output_bounds[0], output_bounds[1]));
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub fn gemma4_vision_patch_embed(
    patches: &[f32],
    position_ids: &[[i64; 2]],
    projection_weight: &[f32],
    position_table: &[f32],
    batch: usize,
    patch_count: usize,
    patch_width: usize,
    hidden: usize,
    position_count: usize,
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, DecoderTextError> {
    let rows = batch
        .checked_mul(patch_count)
        .ok_or(DecoderTextError::Overflow("Gemma4 vision patches"))?;
    if position_ids.len() != rows
        || patches.len()
            != rows
                .checked_mul(patch_width)
                .ok_or(DecoderTextError::Overflow("Gemma4 patch payload"))?
        || projection_weight.len()
            != hidden
                .checked_mul(patch_width)
                .ok_or(DecoderTextError::Overflow("Gemma4 patch projection"))?
        || position_table.len()
            != 2_usize
                .checked_mul(position_count)
                .and_then(|value| value.checked_mul(hidden))
                .ok_or(DecoderTextError::Overflow("Gemma4 position table"))?
    {
        return Err(DecoderTextError::InvalidInput(
            "Gemma4 vision patch payload lengths are invalid",
        ));
    }
    let normalized = patches
        .iter()
        .map(|value| 2.0 * (*value - 0.5))
        .collect::<Vec<_>>();
    let mut output = gemma4_clipped_linear(
        &normalized,
        projection_weight,
        None,
        rows,
        patch_width,
        hidden,
        [f32::NEG_INFINITY, f32::INFINITY],
        [f32::NEG_INFINITY, f32::INFINITY],
        cancellation,
    )?;
    for (row, position) in position_ids.iter().copied().enumerate() {
        cancellation.check()?;
        if position == [-1, -1] {
            continue;
        }
        let x = usize::try_from(position[0]).map_err(|_| {
            DecoderTextError::InvalidInput("Gemma4 vision position must be nonnegative or padding")
        })?;
        let y = usize::try_from(position[1]).map_err(|_| {
            DecoderTextError::InvalidInput("Gemma4 vision position must be nonnegative or padding")
        })?;
        if x >= position_count || y >= position_count {
            return Err(DecoderTextError::InvalidInput(
                "Gemma4 vision position exceeds the learned table",
            ));
        }
        for column in 0..hidden {
            let destination =
                output
                    .get_mut(row * hidden + column)
                    .ok_or(DecoderTextError::InvalidInput(
                        "Gemma4 embedded patch is missing",
                    ))?;
            let x_value =
                *position_table
                    .get(x * hidden + column)
                    .ok_or(DecoderTextError::InvalidInput(
                        "Gemma4 x position embedding is missing",
                    ))?;
            let y_value = *position_table
                .get((position_count + y) * hidden + column)
                .ok_or(DecoderTextError::InvalidInput(
                    "Gemma4 y position embedding is missing",
                ))?;
            *destination += x_value + y_value;
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub fn gemma4_audio_conv2d_subsample(
    input: &[f32],
    weight: &[f32],
    batch: usize,
    input_channels: usize,
    height: usize,
    width: usize,
    output_channels: usize,
    kernel: usize,
    stride: usize,
    padding: usize,
    cancellation: &CancellationToken,
) -> Result<(Vec<f32>, [usize; 2]), DecoderTextError> {
    if batch == 0
        || input_channels == 0
        || height == 0
        || width == 0
        || output_channels == 0
        || kernel == 0
        || stride == 0
    {
        return Err(DecoderTextError::InvalidInput(
            "Gemma4 audio convolution dimensions are invalid",
        ));
    }
    let input_count = batch
        .checked_mul(input_channels)
        .and_then(|value| value.checked_mul(height))
        .and_then(|value| value.checked_mul(width))
        .ok_or(DecoderTextError::Overflow("audio convolution input"))?;
    let weight_count = output_channels
        .checked_mul(input_channels)
        .and_then(|value| value.checked_mul(kernel))
        .and_then(|value| value.checked_mul(kernel))
        .ok_or(DecoderTextError::Overflow("audio convolution weight"))?;
    if input.len() != input_count || weight.len() != weight_count {
        return Err(DecoderTextError::InvalidInput(
            "Gemma4 audio convolution payload lengths are invalid",
        ));
    }
    let padded_height = height
        .checked_add(padding.saturating_mul(2))
        .ok_or(DecoderTextError::Overflow("audio padded height"))?;
    let padded_width = width
        .checked_add(padding.saturating_mul(2))
        .ok_or(DecoderTextError::Overflow("audio padded width"))?;
    if padded_height < kernel || padded_width < kernel {
        return Err(DecoderTextError::InvalidInput(
            "Gemma4 audio convolution kernel exceeds the padded input",
        ));
    }
    let output_height = (padded_height - kernel) / stride + 1;
    let output_width = (padded_width - kernel) / stride + 1;
    let mut output = vec![
        0.0;
        batch
            .checked_mul(output_channels)
            .and_then(|value| value.checked_mul(output_height))
            .and_then(|value| value.checked_mul(output_width))
            .ok_or(DecoderTextError::Overflow("audio convolution output"))?
    ];
    for batch_index in 0..batch {
        for output_channel in 0..output_channels {
            for output_y in 0..output_height {
                for output_x in 0..output_width {
                    cancellation.check()?;
                    let mut sum = 0.0;
                    for input_channel in 0..input_channels {
                        for kernel_y in 0..kernel {
                            for kernel_x in 0..kernel {
                                let padded_y = output_y * stride + kernel_y;
                                let padded_x = output_x * stride + kernel_x;
                                let Some(input_y) = padded_y.checked_sub(padding) else {
                                    continue;
                                };
                                let Some(input_x) = padded_x.checked_sub(padding) else {
                                    continue;
                                };
                                if input_y >= height || input_x >= width {
                                    continue;
                                }
                                let input_index = ((batch_index * input_channels + input_channel)
                                    * height
                                    + input_y)
                                    * width
                                    + input_x;
                                let weight_index =
                                    ((output_channel * input_channels + input_channel) * kernel
                                        + kernel_y)
                                        * kernel
                                        + kernel_x;
                                sum += input.get(input_index).ok_or(
                                    DecoderTextError::InvalidInput(
                                        "audio convolution input is missing",
                                    ),
                                )? * weight.get(weight_index).ok_or(
                                    DecoderTextError::InvalidInput(
                                        "audio convolution weight is missing",
                                    ),
                                )?;
                            }
                        }
                    }
                    let output_index = ((batch_index * output_channels + output_channel)
                        * output_height
                        + output_y)
                        * output_width
                        + output_x;
                    *output
                        .get_mut(output_index)
                        .ok_or(DecoderTextError::InvalidInput(
                            "audio convolution output is missing",
                        ))? = sum;
                }
            }
        }
    }
    Ok((output, [output_height, output_width]))
}

#[allow(clippy::too_many_arguments)]
pub fn qwen35_vision_patch_embed(
    patches: &[f32],
    weight: &[f32],
    bias: &[f32],
    patch_count: usize,
    input_width: usize,
    hidden: usize,
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, DecoderTextError> {
    gemma4_clipped_linear(
        patches,
        weight,
        Some(bias),
        patch_count,
        input_width,
        hidden,
        [f32::NEG_INFINITY, f32::INFINITY],
        [f32::NEG_INFINITY, f32::INFINITY],
        cancellation,
    )
}

pub fn qwen35_vision_patch_merge(
    patches: &[f32],
    patch_count: usize,
    hidden: usize,
    spatial_merge_size: usize,
    first_weight: &[f32],
    first_bias: Option<&[f32]>,
    second_weight: &[f32],
    second_bias: Option<&[f32]>,
    output_hidden: usize,
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, DecoderTextError> {
    if patch_count == 0 || hidden == 0 || spatial_merge_size == 0 || output_hidden == 0 {
        return Err(DecoderTextError::InvalidInput(
            "Qwen3.5 patch-merge dimensions are invalid",
        ));
    }
    let merge_unit = spatial_merge_size
        .checked_mul(spatial_merge_size)
        .ok_or(DecoderTextError::Overflow("Qwen3.5 merge unit"))?;
    if !patch_count.is_multiple_of(merge_unit)
        || patches.len()
            != patch_count
                .checked_mul(hidden)
                .ok_or(DecoderTextError::Overflow("Qwen3.5 patch payload"))?
    {
        return Err(DecoderTextError::InvalidInput(
            "Qwen3.5 patch payload cannot be grouped by the merge size",
        ));
    }
    let merge_width = hidden
        .checked_mul(merge_unit)
        .ok_or(DecoderTextError::Overflow("Qwen3.5 merge width"))?;
    let rows = patch_count / merge_unit;
    let mut normalized_patches = Vec::new();
    normalized_patches
        .try_reserve_exact(patches.len())
        .map_err(|_| DecoderTextError::Allocation("Qwen3.5 normalized patches"))?;
    for patch in 0..patch_count {
        cancellation.check()?;
        let start = patch.checked_mul(hidden).ok_or(DecoderTextError::Overflow(
            "Qwen3.5 patch normalization offset",
        ))?;
        let end = start.checked_add(hidden).ok_or(DecoderTextError::Overflow(
            "Qwen3.5 patch normalization end",
        ))?;
        let values = patches
            .get(start..end)
            .ok_or(DecoderTextError::InvalidInput(
                "Qwen3.5 patch normalization input is missing",
            ))?;
        let mean = values.iter().copied().sum::<f32>() / hidden as f32;
        let variance = values
            .iter()
            .map(|value| (*value - mean) * (*value - mean))
            .sum::<f32>()
            / hidden as f32;
        let inverse_standard_deviation = (variance + 1.0e-6).sqrt().recip();
        normalized_patches.extend(
            values
                .iter()
                .map(|value| (*value - mean) * inverse_standard_deviation),
        );
    }
    let mut merged = Vec::new();
    merged
        .try_reserve_exact(normalized_patches.len())
        .map_err(|_| DecoderTextError::Allocation("Qwen3.5 merged patches"))?;
    for group in 0..patch_count / merge_unit {
        for patch in 0..merge_unit {
            cancellation.check()?;
            let start = (group * merge_unit + patch)
                .checked_mul(hidden)
                .ok_or(DecoderTextError::Overflow("Qwen3.5 patch offset"))?;
            let end = start
                .checked_add(hidden)
                .ok_or(DecoderTextError::Overflow("Qwen3.5 patch end"))?;
            merged.extend_from_slice(normalized_patches.get(start..end).ok_or(
                DecoderTextError::InvalidInput("Qwen3.5 patch group is missing"),
            )?);
        }
    }
    let first = gemma4_clipped_linear(
        &merged,
        first_weight,
        first_bias,
        rows,
        merge_width,
        merge_width,
        [f32::NEG_INFINITY, f32::INFINITY],
        [f32::NEG_INFINITY, f32::INFINITY],
        cancellation,
    )?;
    let activated = first
        .iter()
        .map(|value| {
            let cubic = value * value * value;
            let inner = 0.797_884_6 * (value + 0.044715 * cubic);
            0.5 * value * (1.0 + inner.tanh())
        })
        .collect::<Vec<_>>();
    gemma4_clipped_linear(
        &activated,
        second_weight,
        second_bias,
        rows,
        merge_width,
        output_hidden,
        [f32::NEG_INFINITY, f32::INFINITY],
        [f32::NEG_INFINITY, f32::INFINITY],
        cancellation,
    )
}

fn build_layer(
    index: usize,
    kind: DecoderLayerKind,
    configuration: &DecoderTextConfiguration,
    weights: DecoderLayerWeights,
    stream: StreamId,
) -> Result<NativeDecoderLayer, DecoderTextError> {
    let prefix = format!("decoder.layers.{index}");
    for weight in [
        &weights.attention_norm_weight,
        &weights.feed_forward_norm_weight,
    ] {
        require_vector_parameter(weight, configuration.hidden_size, stream)?;
    }
    for weight in [
        weights.post_attention_norm_weight.as_ref(),
        weights.post_feed_forward_norm_weight.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        require_vector_parameter(weight, configuration.hidden_size, stream)?;
    }
    match (&weights.attention_sink, configuration.architecture) {
        (Some(sink), DecoderArchitecture::GptOss) => {
            require_vector_parameter(sink, configuration.attention_heads, stream)?;
        }
        (None, DecoderArchitecture::GptOss) => {
            return Err(DecoderTextError::InvalidConfiguration(
                "GPT-OSS attention layers require one sink per query head",
            ));
        }
        (None, _) => {}
        (Some(_), _) => {
            return Err(DecoderTextError::InvalidConfiguration(
                "attention sinks are owned only by GPT-OSS profiles",
            ));
        }
    }
    let query_width = configuration
        .attention_heads
        .checked_mul(configuration.head_dimension)
        .ok_or(DecoderTextError::Overflow("query width"))?;
    let key_value_width = configuration
        .key_value_heads
        .checked_mul(configuration.head_dimension)
        .ok_or(DecoderTextError::Overflow("key/value width"))?;
    let query = linear_module(
        format!("{prefix}.query"),
        configuration.hidden_size,
        query_width,
        weights.query_weight,
        stream,
    )?;
    let key = linear_module(
        format!("{prefix}.key"),
        configuration.hidden_size,
        key_value_width,
        weights.key_weight,
        stream,
    )?;
    let value = linear_module(
        format!("{prefix}.value"),
        configuration.hidden_size,
        key_value_width,
        weights.value_weight,
        stream,
    )?;
    let attention_output = linear_module(
        format!("{prefix}.attention_output"),
        query_width,
        configuration.hidden_size,
        weights.attention_output_weight,
        stream,
    )?;
    let feed_forward_gate = linear_module(
        format!("{prefix}.feed_forward_gate"),
        configuration.hidden_size,
        configuration.feed_forward_size,
        weights.feed_forward_gate_weight,
        stream,
    )?;
    let feed_forward_up = linear_module(
        format!("{prefix}.feed_forward_up"),
        configuration.hidden_size,
        configuration.feed_forward_size,
        weights.feed_forward_up_weight,
        stream,
    )?;
    let feed_forward_down = linear_module(
        format!("{prefix}.feed_forward_down"),
        configuration.feed_forward_size,
        configuration.hidden_size,
        weights.feed_forward_down_weight,
        stream,
    )?;
    let activation = match configuration.activation {
        DecoderActivation::Silu => NativeModule::silu(format!("{prefix}.activation"))?,
        DecoderActivation::GeluTanh => NativeModule::gelu(
            format!("{prefix}.activation"),
            crate::GeluApproximation::Tanh,
        )?,
    };
    Ok(NativeDecoderLayer {
        kind,
        attention_norm_weight: weights.attention_norm_weight,
        query,
        key,
        value,
        attention_output,
        feed_forward_norm_weight: weights.feed_forward_norm_weight,
        feed_forward_gate,
        feed_forward_up,
        activation,
        feed_forward_down,
        post_attention_norm_weight: weights.post_attention_norm_weight,
        post_feed_forward_norm_weight: weights.post_feed_forward_norm_weight,
        attention_sink: weights.attention_sink,
    })
}

fn linear_module(
    name: String,
    input: usize,
    output: usize,
    weight: Tensor,
    stream: StreamId,
) -> Result<NativeModule, DecoderTextError> {
    require_parameter(&weight, stream)?;
    let mut module = NativeModule::linear(name, input, output, false, false)?;
    module.load_dense_parameters(weight, None)?;
    Ok(module)
}

fn require_parameter(tensor: &Tensor, stream: StreamId) -> Result<(), DecoderTextError> {
    let descriptor = tensor.descriptor();
    if descriptor.dtype() != DType::F32
        || descriptor.device() != DeviceId::CPU
        || descriptor.stream() != stream
        || !descriptor.is_contiguous()?
    {
        return Err(DecoderTextError::InvalidConfiguration(
            "parameters must be contiguous CPU F32 tensors on one stream",
        ));
    }
    Ok(())
}

fn require_vector_parameter(
    tensor: &Tensor,
    width: usize,
    stream: StreamId,
) -> Result<(), DecoderTextError> {
    require_parameter(tensor, stream)?;
    if tensor.descriptor().shape() != [usize_to_u64(width, "normalization width")?] {
        return Err(DecoderTextError::InvalidConfiguration(
            "normalization parameter width is invalid",
        ));
    }
    Ok(())
}

fn resolve_layer(layer: isize, count: usize) -> Result<usize, DecoderTextError> {
    let resolved = if layer < 0 {
        isize::try_from(count)
            .ok()
            .and_then(|count| count.checked_add(layer))
    } else {
        Some(layer)
    };
    let resolved = resolved
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value < count)
        .ok_or(DecoderTextError::CaptureOutOfRange {
            requested: layer,
            available: count,
        })?;
    Ok(resolved)
}

fn validate_positions(
    positions: Option<&[usize]>,
    query_tokens: usize,
    cache: Option<&DecoderKvState>,
) -> Result<Vec<usize>, DecoderTextError> {
    if let Some(positions) = positions {
        if positions.len() != query_tokens {
            return Err(DecoderTextError::InvalidInput(
                "position count must equal query token count",
            ));
        }
        return Ok(positions.to_vec());
    }
    let start = cache.map(cache_token_count).transpose()?.unwrap_or(0);
    (0..query_tokens)
        .map(|offset| {
            start
                .checked_add(offset)
                .ok_or(DecoderTextError::Overflow("token position"))
        })
        .collect()
}

fn validate_prepared_positions(
    rope_positions: DecoderRopePositions<'_>,
    causal_positions: &[usize],
    query_tokens: usize,
    cache: Option<&DecoderKvState>,
    configuration: &DecoderRopeConfiguration,
) -> Result<ValidatedDecoderPositions, DecoderTextError> {
    if cache.is_some() {
        return Err(DecoderTextError::InvalidInput(
            "prepared embeddings are valid only for an uncached prefill",
        ));
    }
    if causal_positions.len() != query_tokens {
        return Err(DecoderTextError::InvalidInput(
            "prepared causal position count must equal query token count",
        ));
    }
    let rope_axes = match rope_positions {
        DecoderRopePositions::Scalar(positions) => {
            if positions.len() != query_tokens {
                return Err(DecoderTextError::InvalidInput(
                    "prepared scalar positions do not match the decoder RoPE profile",
                ));
            }
            if configuration.interleaved_sections.is_empty() {
                vec![positions.to_vec()]
            } else {
                vec![positions.to_vec(); configuration.interleaved_sections.len()]
            }
        }
        DecoderRopePositions::Multidimensional(position_axes) => {
            if position_axes.len() != configuration.interleaved_sections.len()
                || position_axes.is_empty()
                || position_axes.iter().any(|axis| axis.len() != query_tokens)
            {
                return Err(DecoderTextError::InvalidInput(
                    "prepared multidimensional positions do not match the decoder RoPE profile",
                ));
            }
            position_axes.to_vec()
        }
    };
    Ok(ValidatedDecoderPositions {
        rope_axes,
        causal: causal_positions.to_vec(),
    })
}

fn maximum_rope_position(
    rope_positions: DecoderRopePositions<'_>,
) -> Result<usize, DecoderTextError> {
    match rope_positions {
        DecoderRopePositions::Scalar(positions) => positions.iter().copied().max(),
        DecoderRopePositions::Multidimensional(position_axes) => position_axes
            .iter()
            .flat_map(|axis| axis.iter().copied())
            .max(),
    }
    .ok_or(DecoderTextError::InvalidInput(
        "prepared generation positions are empty",
    ))
}

fn cache_token_count(cache: &DecoderKvState) -> Result<usize, DecoderTextError> {
    let mut tokens = None;
    for layer in cache.layers.iter().flatten() {
        let layer_tokens = match layer {
            DecoderLayerCache::Attention(cache) => cache.tokens,
            DecoderLayerCache::Linear(_) => continue,
        };
        if tokens.is_some_and(|tokens| tokens != layer_tokens) {
            return Err(DecoderTextError::InvalidInput(
                "decoder cache layers disagree on token count",
            ));
        }
        tokens = Some(layer_tokens);
    }
    Ok(tokens.unwrap_or(0))
}

fn validate_cache(
    cache: &DecoderKvState,
    batch: usize,
    stream: StreamId,
    configuration: &DecoderTextConfiguration,
) -> Result<DecoderKvState, DecoderTextError> {
    if cache.layers.len() != configuration.layer_kinds.len() {
        return Err(DecoderTextError::InvalidInput(
            "decoder cache layer count does not match the profile",
        ));
    }
    let mut token_count = None;
    for (kind, cache) in configuration.layer_kinds.iter().zip(&cache.layers) {
        match (kind, cache) {
            (_, None) => {}
            (DecoderLayerKind::LinearAttention, Some(DecoderLayerCache::Linear(cache))) => {
                let state_width = configuration
                    .key_value_heads
                    .checked_mul(configuration.head_dimension)
                    .ok_or(DecoderTextError::Overflow("linear cache width"))?;
                let expected_shapes = [
                    vec![
                        usize_to_u64(batch, "linear cache batch")?,
                        usize_to_u64(state_width, "linear cache width")?,
                        2,
                    ],
                    vec![
                        usize_to_u64(batch, "linear cache batch")?,
                        usize_to_u64(configuration.attention_heads, "linear cache heads")?,
                        usize_to_u64(configuration.head_dimension, "linear cache key width")?,
                        usize_to_u64(configuration.head_dimension, "linear cache value width")?,
                    ],
                ];
                for (tensor, expected_shape) in [&cache.convolution_state, &cache.recurrent_state]
                    .into_iter()
                    .zip(expected_shapes)
                {
                    let descriptor = tensor.descriptor();
                    if descriptor.shape() != expected_shape
                        || descriptor.dtype() != DType::F32
                        || descriptor.device() != DeviceId::CPU
                        || descriptor.stream() != stream
                        || !descriptor.is_contiguous()?
                    {
                        return Err(DecoderTextError::InvalidInput(
                            "Qwen3.5 cache tensors must use the admitted target and stream",
                        ));
                    }
                }
            }
            (
                DecoderLayerKind::FullAttention | DecoderLayerKind::SlidingAttention,
                Some(DecoderLayerCache::Attention(cache)),
            ) => {
                if cache.batch != batch
                    || cache.key_value_heads != configuration.key_value_heads
                    || cache.head_dimension != configuration.head_dimension
                    || cache.tokens > configuration.maximum_tokens
                {
                    return Err(DecoderTextError::InvalidInput(
                        "decoder attention cache dimensions are invalid",
                    ));
                }
                let expected = [
                    usize_to_u64(batch, "cache batch")?,
                    usize_to_u64(cache.tokens, "cache tokens")?,
                    usize_to_u64(cache.key_value_heads, "cache heads")?,
                    usize_to_u64(cache.head_dimension, "cache head dimension")?,
                ];
                for tensor in [&cache.keys, &cache.values] {
                    let descriptor = tensor.descriptor();
                    if descriptor.shape() != expected
                        || descriptor.dtype() != DType::F32
                        || descriptor.device() != DeviceId::CPU
                        || descriptor.stream() != stream
                        || !descriptor.is_contiguous()?
                    {
                        return Err(DecoderTextError::InvalidInput(
                            "decoder attention cache tensor descriptor is invalid",
                        ));
                    }
                }
                if token_count.is_some_and(|tokens| tokens != cache.tokens) {
                    return Err(DecoderTextError::InvalidInput(
                        "decoder attention caches disagree on token count",
                    ));
                }
                token_count = Some(cache.tokens);
            }
            _ => {
                return Err(DecoderTextError::InvalidInput(
                    "decoder cache kind does not match its layer",
                ));
            }
        }
    }
    Ok(cache.clone())
}

#[allow(clippy::too_many_arguments)]
fn stage_attention_cache(
    backend: &CpuBackend,
    cache: &mut Option<DecoderLayerCache>,
    batch: usize,
    key_value_heads: usize,
    head_dimension: usize,
    keys: &[f32],
    values: &[f32],
    query_tokens: usize,
    maximum_tokens: usize,
    context: &ExecutionContext<'_>,
) -> Result<(Vec<f32>, Vec<f32>, usize), DecoderTextError> {
    let prior = match cache.as_ref() {
        Some(DecoderLayerCache::Attention(cache)) => Some(cache),
        None => None,
        Some(DecoderLayerCache::Linear(_)) => {
            return Err(DecoderTextError::InvalidInput(
                "linear cache cannot be used by an attention layer",
            ));
        }
    };
    let prior_tokens = prior.map(|cache| cache.tokens).unwrap_or(0);
    let total_tokens = prior_tokens
        .checked_add(query_tokens)
        .ok_or(DecoderTextError::Overflow("cache tokens"))?;
    if total_tokens > maximum_tokens {
        return Err(DecoderTextError::InvalidInput(
            "decoder cache exceeds the configured token limit",
        ));
    }
    let query_count = batch
        .checked_mul(query_tokens)
        .and_then(|value| value.checked_mul(key_value_heads))
        .and_then(|value| value.checked_mul(head_dimension))
        .ok_or(DecoderTextError::Overflow("cache append"))?;
    if keys.len() != query_count || values.len() != query_count {
        return Err(DecoderTextError::InvalidInput(
            "cache append tensors have the wrong length",
        ));
    }
    let total_count = batch
        .checked_mul(total_tokens)
        .and_then(|value| value.checked_mul(key_value_heads))
        .and_then(|value| value.checked_mul(head_dimension))
        .ok_or(DecoderTextError::Overflow("cache payload"))?;
    let mut next_keys = Vec::new();
    let mut next_values = Vec::new();
    next_keys
        .try_reserve_exact(total_count)
        .map_err(|_| DecoderTextError::Allocation("cache keys"))?;
    next_values
        .try_reserve_exact(total_count)
        .map_err(|_| DecoderTextError::Allocation("cache values"))?;
    let token_width = key_value_heads
        .checked_mul(head_dimension)
        .ok_or(DecoderTextError::Overflow("cache token width"))?;
    let prior_batch_width = prior_tokens
        .checked_mul(token_width)
        .ok_or(DecoderTextError::Overflow("prior cache batch width"))?;
    let query_batch_width = query_tokens
        .checked_mul(token_width)
        .ok_or(DecoderTextError::Overflow("query cache batch width"))?;
    let prior_keys = prior
        .map(|prior| tensor_to_f32(backend, &prior.keys, context))
        .transpose()?;
    let prior_values = prior
        .map(|prior| tensor_to_f32(backend, &prior.values, context))
        .transpose()?;
    for batch_index in 0..batch {
        context.check()?;
        if prior.is_some() {
            let start = batch_index
                .checked_mul(prior_batch_width)
                .ok_or(DecoderTextError::Overflow("prior cache batch offset"))?;
            let end = start
                .checked_add(prior_batch_width)
                .ok_or(DecoderTextError::Overflow("prior cache batch end"))?;
            next_keys.extend_from_slice(
                prior_keys
                    .as_ref()
                    .and_then(|values| values.get(start..end))
                    .ok_or(DecoderTextError::InvalidInput(
                        "prior cache key batch is missing",
                    ))?,
            );
            next_values.extend_from_slice(
                prior_values
                    .as_ref()
                    .and_then(|values| values.get(start..end))
                    .ok_or(DecoderTextError::InvalidInput(
                        "prior cache value batch is missing",
                    ))?,
            );
        }
        let start = batch_index
            .checked_mul(query_batch_width)
            .ok_or(DecoderTextError::Overflow("query cache batch offset"))?;
        let end = start
            .checked_add(query_batch_width)
            .ok_or(DecoderTextError::Overflow("query cache batch end"))?;
        next_keys.extend_from_slice(keys.get(start..end).ok_or(DecoderTextError::InvalidInput(
            "query cache key batch is missing",
        ))?);
        next_values.extend_from_slice(values.get(start..end).ok_or(
            DecoderTextError::InvalidInput("query cache value batch is missing"),
        )?);
    }
    let cache_shape = [
        usize_to_u64(batch, "cache batch")?,
        usize_to_u64(total_tokens, "cache tokens")?,
        usize_to_u64(key_value_heads, "cache heads")?,
        usize_to_u64(head_dimension, "cache head dimension")?,
    ];
    let cached_keys = tensor_from_f32(backend, &cache_shape, &next_keys, context)?;
    let cached_values = tensor_from_f32(backend, &cache_shape, &next_values, context)?;
    *cache = Some(DecoderLayerCache::Attention(DecoderAttentionCache {
        batch,
        key_value_heads,
        head_dimension,
        tokens: total_tokens,
        keys: cached_keys,
        values: cached_values,
    }));
    Ok((next_keys, next_values, total_tokens))
}

#[allow(clippy::too_many_arguments)]
fn expand_grouped_query(
    input: &[f32],
    batch: usize,
    tokens: usize,
    key_value_heads: usize,
    query_heads: usize,
    head_dimension: usize,
    cancellation: &CancellationToken,
) -> Result<Vec<f32>, DecoderTextError> {
    if !query_heads.is_multiple_of(key_value_heads) {
        return Err(DecoderTextError::InvalidConfiguration(
            "query heads must be divisible by key/value heads",
        ));
    }
    let input_count = batch
        .checked_mul(tokens)
        .and_then(|value| value.checked_mul(key_value_heads))
        .and_then(|value| value.checked_mul(head_dimension))
        .ok_or(DecoderTextError::Overflow("grouped-query input"))?;
    if input.len() != input_count {
        return Err(DecoderTextError::InvalidInput(
            "grouped-query tensor has the wrong length",
        ));
    }
    let output_count = batch
        .checked_mul(tokens)
        .and_then(|value| value.checked_mul(query_heads))
        .and_then(|value| value.checked_mul(head_dimension))
        .ok_or(DecoderTextError::Overflow("grouped-query output"))?;
    let repeats = query_heads / key_value_heads;
    let mut output = Vec::new();
    output
        .try_reserve_exact(output_count)
        .map_err(|_| DecoderTextError::Allocation("grouped-query expansion"))?;
    for batch_index in 0..batch {
        for token in 0..tokens {
            for head in 0..query_heads {
                let key_value_head = head / repeats;
                for column in 0..head_dimension {
                    if output.len().is_multiple_of(256) {
                        cancellation.check()?;
                    }
                    let source = (((batch_index * tokens + token) * key_value_heads
                        + key_value_head)
                        * head_dimension)
                        + column;
                    output.push(*input.get(source).ok_or(DecoderTextError::InvalidInput(
                        "grouped-query source value is missing",
                    ))?);
                }
            }
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn append_attention_sink_tokens(
    keys: &[f32],
    values: &[f32],
    batch: usize,
    key_tokens: usize,
    heads: usize,
    head_dimension: usize,
    sinks: Option<&[f32]>,
    cancellation: &CancellationToken,
) -> Result<(Vec<f32>, Vec<f32>, usize), DecoderTextError> {
    let token_width = heads
        .checked_mul(head_dimension)
        .ok_or(DecoderTextError::Overflow("attention sink token width"))?;
    let expected = batch
        .checked_mul(key_tokens)
        .and_then(|value| value.checked_mul(token_width))
        .ok_or(DecoderTextError::Overflow("attention sink input"))?;
    if keys.len() != expected || values.len() != expected {
        return Err(DecoderTextError::InvalidInput(
            "attention sink input payload has the wrong length",
        ));
    }
    let Some(sinks) = sinks else {
        return Ok((keys.to_vec(), values.to_vec(), key_tokens));
    };
    if sinks.len() != heads || sinks.iter().any(|sink| !sink.is_finite()) {
        return Err(DecoderTextError::InvalidInput(
            "GPT-OSS attention sinks must be one finite value per head",
        ));
    }
    let output_tokens = key_tokens
        .checked_add(1)
        .ok_or(DecoderTextError::Overflow("attention sink tokens"))?;
    let output_count = batch
        .checked_mul(output_tokens)
        .and_then(|value| value.checked_mul(token_width))
        .ok_or(DecoderTextError::Overflow("attention sink payload"))?;
    let input_batch_width = key_tokens
        .checked_mul(token_width)
        .ok_or(DecoderTextError::Overflow("attention sink input batch"))?;
    let mut next_keys = Vec::new();
    let mut next_values = Vec::new();
    next_keys
        .try_reserve_exact(output_count)
        .map_err(|_| DecoderTextError::Allocation("attention sink keys"))?;
    next_values
        .try_reserve_exact(output_count)
        .map_err(|_| DecoderTextError::Allocation("attention sink values"))?;
    for batch_index in 0..batch {
        cancellation.check()?;
        let start = batch_index
            .checked_mul(input_batch_width)
            .ok_or(DecoderTextError::Overflow("attention sink batch offset"))?;
        let end = start
            .checked_add(input_batch_width)
            .ok_or(DecoderTextError::Overflow("attention sink batch end"))?;
        next_keys.extend_from_slice(keys.get(start..end).ok_or(DecoderTextError::InvalidInput(
            "attention sink key batch is missing",
        ))?);
        next_values.extend_from_slice(values.get(start..end).ok_or(
            DecoderTextError::InvalidInput("attention sink value batch is missing"),
        )?);
        next_keys.resize(next_keys.len() + token_width, 0.0);
        next_values.resize(next_values.len() + token_width, 0.0);
    }
    Ok((next_keys, next_values, output_tokens))
}

#[allow(clippy::too_many_arguments)]
fn build_decoder_mask(
    backend: &CpuBackend,
    padding_mask: Option<&Tensor>,
    batch: usize,
    heads: usize,
    query_tokens: usize,
    key_tokens: usize,
    positions: &[usize],
    sliding_window: Option<usize>,
    attention_sinks: Option<&[f32]>,
    context: &ExecutionContext<'_>,
) -> Result<PreparedDecoderMask, DecoderTextError> {
    let padding = padding_mask
        .map(|mask| tensor_to_f32(backend, mask, context))
        .transpose()?;
    if let Some(padding) = &padding {
        let expected = batch
            .checked_mul(key_tokens)
            .ok_or(DecoderTextError::Overflow("padding mask"))?;
        if padding.len() != expected || padding.iter().any(|value| !matches!(*value, 0.0 | 1.0)) {
            return Err(DecoderTextError::InvalidInput(
                "padding mask must be [batch, key_tokens] with zero/one values",
            ));
        }
    }
    if attention_sinks.is_some_and(|sinks| sinks.len() != heads) {
        return Err(DecoderTextError::InvalidInput(
            "attention sink count must equal the query-head count",
        ));
    }
    let attention_key_tokens = key_tokens
        .checked_add(usize::from(attention_sinks.is_some()))
        .ok_or(DecoderTextError::Overflow("decoder mask sink token"))?;
    let count = batch
        .checked_mul(heads)
        .and_then(|value| value.checked_mul(query_tokens))
        .and_then(|value| value.checked_mul(attention_key_tokens))
        .ok_or(DecoderTextError::Overflow("decoder mask"))?;
    let mut values = backend.workspace_vec(context, count)?;
    for batch_index in 0..batch {
        for head in 0..heads {
            for query in 0..query_tokens {
                let absolute_query = *positions
                    .get(query)
                    .ok_or(DecoderTextError::InvalidInput("query position is missing"))?;
                for key in 0..key_tokens {
                    context.check()?;
                    let is_future = key > absolute_query;
                    let outside_window = sliding_window
                        .is_some_and(|window| key.saturating_add(window) <= absolute_query);
                    let padded = padding.as_ref().is_some_and(|padding| {
                        padding
                            .get(batch_index * key_tokens + key)
                            .is_some_and(|value| *value == 0.0)
                    });
                    values.try_push(if is_future || outside_window || padded {
                        -f32::MAX
                    } else {
                        0.0
                    })?;
                }
                if let Some(sinks) = attention_sinks {
                    values.try_push(*sinks.get(head).ok_or(DecoderTextError::InvalidInput(
                        "attention sink value is missing",
                    ))?)?;
                }
            }
        }
    }
    Ok(PreparedDecoderMask { values })
}

fn rms_norm_tensor(
    backend: &CpuBackend,
    input: &Tensor,
    weight: &Tensor,
    hidden: usize,
    epsilon: f32,
    weight_offset: f32,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, DecoderTextError> {
    let input_values = tensor_to_f32(backend, input, context)?;
    let mut weight_values = tensor_to_f32(backend, weight, context)?;
    if weight_offset != 0.0 {
        for value in weight_values.iter_mut() {
            *value += weight_offset;
        }
    }
    let shape = input
        .descriptor()
        .shape()
        .iter()
        .map(|dimension| {
            usize::try_from(*dimension)
                .map_err(|_| DecoderTextError::Overflow("normalization shape"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let values = rms_norm_with_context_exact_native(
        backend,
        &input_values,
        &shape,
        &[hidden],
        Some(&weight_values),
        Some(epsilon),
        DeviceId::CPU,
        context,
    )?;
    tensor_from_f32(backend, input.descriptor().shape(), &values, context).map_err(Into::into)
}

fn multiply_tensor(
    backend: &CpuBackend,
    left: &Tensor,
    right: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, DecoderTextError> {
    if left.descriptor().shape() != right.descriptor().shape() {
        return Err(DecoderTextError::InvalidInput(
            "gated feed-forward tensors must have equal shapes",
        ));
    }
    let shape = left.descriptor().shape().to_vec();
    let left = tensor_to_f32(backend, left, context)?;
    let right = tensor_to_f32(backend, right, context)?;
    let mut values = backend.workspace_vec(context, left.len())?;
    for (index, (left, right)) in left.iter().zip(right.iter()).enumerate() {
        if index.is_multiple_of(256) {
            context.check()?;
        }
        values.try_push(left * right)?;
    }
    tensor_from_f32(backend, &shape, &values, context).map_err(Into::into)
}

fn scale_tensor(
    backend: &CpuBackend,
    tensor: &Tensor,
    scale: f32,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, DecoderTextError> {
    let values = tensor_to_f32(backend, tensor, context)?;
    let mut scaled = backend.workspace_vec(context, values.len())?;
    for (index, value) in values.iter().enumerate() {
        if index.is_multiple_of(256) {
            context.check()?;
        }
        scaled.try_push(value * scale)?;
    }
    tensor_from_f32(backend, tensor.descriptor().shape(), &scaled, context).map_err(Into::into)
}

fn add_scaled(
    backend: &CpuBackend,
    left: &Tensor,
    right: &Tensor,
    scale: f32,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, DecoderTextError> {
    if scale == 1.0 {
        return add(backend, left, right, context).map_err(Into::into);
    }
    let right = scale_tensor(backend, right, scale, context)?;
    add(backend, left, &right, context).map_err(Into::into)
}

fn apply_logits_soft_cap(
    backend: &CpuBackend,
    logits: &Tensor,
    cap: Option<f32>,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, DecoderTextError> {
    let Some(cap) = cap else {
        return Ok(logits.clone());
    };
    let values = tensor_to_f32(backend, logits, context)?;
    let mut capped = backend.workspace_vec(context, values.len())?;
    for (index, value) in values.iter().enumerate() {
        if index.is_multiple_of(256) {
            context.check()?;
        }
        capped.try_push(cap * (*value / cap).tanh())?;
    }
    tensor_from_f32(backend, logits.descriptor().shape(), &capped, context).map_err(Into::into)
}

fn validate_tokens(
    backend: &CpuBackend,
    tokens: &Tensor,
    vocabulary_size: usize,
    maximum_tokens: usize,
    context: &ExecutionContext<'_>,
) -> Result<(usize, usize), DecoderTextError> {
    let shape = tokens.descriptor().shape();
    if shape.len() != 2 || tokens.descriptor().dtype() != DType::I64 {
        return Err(DecoderTextError::InvalidInput(
            "decoder tokens must be an I64 [batch, tokens] tensor",
        ));
    }
    let batch = usize::try_from(shape[0]).map_err(|_| DecoderTextError::Overflow("batch"))?;
    let token_count =
        usize::try_from(shape[1]).map_err(|_| DecoderTextError::Overflow("tokens"))?;
    if batch == 0 || token_count == 0 || token_count > maximum_tokens {
        return Err(DecoderTextError::InvalidInput(
            "decoder token dimensions are empty or exceed the profile limit",
        ));
    }
    for token in read_i64_tensor(backend, tokens, context)? {
        if token < 0
            || usize::try_from(token)
                .ok()
                .is_none_or(|token| token >= vocabulary_size)
        {
            return Err(DecoderTextError::TokenOutOfRange(token));
        }
    }
    Ok((batch, token_count))
}

fn read_i64_tensor(
    _backend: &CpuBackend,
    tensor: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Vec<i64>, DecoderTextError> {
    context.check()?;
    if tensor.descriptor().dtype() != DType::I64 || tensor.descriptor().device() != DeviceId::CPU {
        return Err(DecoderTextError::InvalidInput(
            "token tensors must be CPU I64",
        ));
    }
    let bytes = tensor.contiguous_bytes()?;
    if !bytes.len().is_multiple_of(std::mem::size_of::<i64>()) {
        return Err(DecoderTextError::InvalidInput(
            "token tensor bytes are unaligned",
        ));
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(bytes.len() / std::mem::size_of::<i64>())
        .map_err(|_| DecoderTextError::Allocation("token values"))?;
    for (index, chunk) in bytes.chunks_exact(std::mem::size_of::<i64>()).enumerate() {
        if index.is_multiple_of(256) {
            context.check()?;
        }
        let array: [u8; 8] = chunk
            .try_into()
            .map_err(|_| DecoderTextError::InvalidInput("token byte width is invalid"))?;
        values.push(i64::from_ne_bytes(array));
    }
    Ok(values)
}

fn tensor_from_i64(
    backend: &CpuBackend,
    shape: &[u64],
    values: &[i64],
    context: &ExecutionContext<'_>,
) -> Result<Tensor, DecoderTextError> {
    use comfy_tensor::TensorDescriptor;
    let descriptor =
        TensorDescriptor::contiguous(shape.to_vec(), DType::I64, DeviceId::CPU, context.stream)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(
            values
                .len()
                .checked_mul(std::mem::size_of::<i64>())
                .ok_or(DecoderTextError::Overflow("token bytes"))?,
        )
        .map_err(|_| DecoderTextError::Allocation("token bytes"))?;
    for value in values {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
    let (tensor, _) = backend.upload_bytes(descriptor, &bytes, context)?;
    Ok(tensor)
}

fn sample_token(
    logits: &[f32],
    prior_tokens: &[i64],
    configuration: &DecoderGenerationConfiguration,
    transaction: &mut RngTransaction,
    cancellation: &CancellationToken,
) -> Result<i64, DecoderTextError> {
    if logits.is_empty() || logits.iter().any(|value| !value.is_finite()) {
        return Err(DecoderTextError::InvalidInput(
            "generation logits must be nonempty and finite",
        ));
    }
    if configuration.temperature() == 0.0 {
        return logits
            .iter()
            .enumerate()
            .max_by(|(left_index, left), (right_index, right)| {
                left.partial_cmp(right)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| right_index.cmp(left_index))
            })
            .and_then(|(index, _)| i64::try_from(index).ok())
            .ok_or(DecoderTextError::InvalidInput("generation argmax failed"));
    }
    let mut adjusted = logits.to_vec();
    let mut seen_tokens = std::collections::BTreeSet::new();
    for token in prior_tokens
        .iter()
        .copied()
        .filter(|token| seen_tokens.insert(*token))
    {
        let Ok(index) = usize::try_from(token) else {
            continue;
        };
        let Some(value) = adjusted.get_mut(index) else {
            continue;
        };
        let penalty = configuration.repetition_penalty();
        *value = if *value >= 0.0 {
            *value / penalty
        } else {
            *value * penalty
        };
        *value -= configuration.presence_penalty();
    }
    if adjusted.iter().any(|value| !value.is_finite()) {
        return Err(DecoderTextError::InvalidInput(
            "generation penalties produced non-finite logits",
        ));
    }
    for value in &mut adjusted {
        *value /= configuration.temperature();
    }
    let mut candidates = adjusted.iter().copied().enumerate().collect::<Vec<_>>();
    candidates.sort_by(|(left_index, left), (right_index, right)| {
        right
            .partial_cmp(left)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left_index.cmp(right_index))
    });
    if let Some(top_k) = configuration.top_k {
        if let Some((_, threshold)) = candidates.get(top_k.saturating_sub(1)).copied() {
            candidates.retain(|(_, value)| *value >= threshold);
        }
    }
    let maximum =
        candidates
            .first()
            .map(|(_, value)| *value)
            .ok_or(DecoderTextError::InvalidInput(
                "generation candidates are empty",
            ))?;
    let mut probabilities = candidates
        .into_iter()
        .map(|(index, value)| (index, (value - maximum).exp()))
        .collect::<Vec<_>>();
    let total = probabilities.iter().map(|(_, value)| *value).sum::<f32>();
    if !total.is_finite() || total <= 0.0 {
        return Err(DecoderTextError::InvalidInput(
            "generation probability normalization failed",
        ));
    }
    for (_, value) in &mut probabilities {
        *value /= total;
    }
    if let Some(minimum_p) = configuration.minimum_p() {
        let maximum_probability = probabilities
            .iter()
            .map(|(_, value)| *value)
            .fold(0.0_f32, f32::max);
        probabilities.retain(|(_, value)| *value >= maximum_probability * minimum_p);
    }
    let filtered_total = probabilities.iter().map(|(_, value)| *value).sum::<f32>();
    if filtered_total <= 0.0 || !filtered_total.is_finite() {
        return Err(DecoderTextError::InvalidInput(
            "generation filters removed every candidate",
        ));
    }
    for (_, value) in &mut probabilities {
        *value /= filtered_total;
    }
    if let Some(top_p) = configuration.top_p() {
        let mut cumulative = 0.0;
        let mut keep = 0;
        for (_, probability) in &probabilities {
            cumulative += *probability;
            if keep == 0 || cumulative <= top_p {
                keep += 1;
            } else {
                break;
            }
        }
        probabilities.truncate(keep.max(1));
    }
    let retained_total = probabilities.iter().map(|(_, value)| *value).sum::<f32>();
    if retained_total <= 0.0 || !retained_total.is_finite() {
        return Err(DecoderTextError::InvalidInput(
            "generation filters removed every candidate",
        ));
    }
    let draw = transaction.next_unit_f32(cancellation)? * retained_total;
    let mut cumulative = 0.0;
    for (index, probability) in &probabilities {
        cumulative += *probability;
        if draw < cumulative {
            return i64::try_from(*index).map_err(|_| DecoderTextError::Overflow("sampled token"));
        }
    }
    probabilities
        .last()
        .and_then(|(index, _)| i64::try_from(*index).ok())
        .ok_or(DecoderTextError::InvalidInput("generation sampling failed"))
}

fn usize_to_u64(value: usize, name: &'static str) -> Result<u64, DecoderTextError> {
    u64::try_from(value).map_err(|_| DecoderTextError::Overflow(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use comfy_tensor::{
        RetryRngPolicy, RngAlgorithm, RngProfileVersion, RngStream, RngStreamAddress,
    };
    use std::error::Error;

    fn transaction(seed: u64) -> Result<RngTransaction, Box<dyn Error>> {
        let address = RngStreamAddress::new(
            "text-generation-test",
            "attempt-1",
            "node-1",
            0,
            "text-generation",
            0,
            0,
            RetryRngPolicy::Replay,
        )?;
        Ok(RngStream::new(
            RngProfileVersion::V2,
            RngAlgorithm::Philox4x32_10,
            seed,
            address,
        )?
        .begin(None)?)
    }

    fn sampling_configuration() -> DecoderGenerationConfiguration {
        DecoderGenerationConfiguration {
            maximum_new_tokens: 1,
            temperature_bits: 1.0_f32.to_bits(),
            top_k: None,
            top_p_bits: None,
            minimum_p_bits: None,
            repetition_penalty_bits: 1.0_f32.to_bits(),
            presence_penalty_bits: 0.0_f32.to_bits(),
        }
    }

    #[test]
    fn source_sampling_filters_and_penalties_match_base_generate() -> Result<(), Box<dyn Error>> {
        let cancellation = CancellationToken::default();

        let mut unique_penalty = sampling_configuration();
        unique_penalty.top_k = Some(1);
        unique_penalty.repetition_penalty_bits = 2.0_f32.to_bits();
        unique_penalty.presence_penalty_bits = 0.5_f32.to_bits();
        assert_eq!(
            sample_token(
                &[4.0, 1.0],
                &[0, 0],
                &unique_penalty,
                &mut transaction(1)?,
                &cancellation,
            )?,
            0
        );

        let mut top_p = sampling_configuration();
        top_p.top_p_bits = Some(0.7_f32.to_bits());
        for seed in 0..16 {
            assert_eq!(
                sample_token(
                    &[2.0, 1.0, 0.0],
                    &[],
                    &top_p,
                    &mut transaction(seed)?,
                    &cancellation,
                )?,
                0
            );
        }

        let mut top_k_ties = sampling_configuration();
        top_k_ties.top_k = Some(2);
        let mut retained_tied_tail = false;
        for seed in 0..64 {
            retained_tied_tail |= sample_token(
                &[1.0, 1.0, 1.0],
                &[],
                &top_k_ties,
                &mut transaction(seed)?,
                &cancellation,
            )? == 2;
        }
        assert!(retained_tied_tail);

        Ok(())
    }

    #[test]
    fn greedy_generation_ignores_sampling_penalties_and_draws_no_rng() -> Result<(), Box<dyn Error>>
    {
        let cancellation = CancellationToken::default();
        let mut configuration = sampling_configuration();
        configuration.temperature_bits = 0.0_f32.to_bits();
        configuration.repetition_penalty_bits = 10.0_f32.to_bits();
        configuration.presence_penalty_bits = 5.0_f32.to_bits();
        let mut transaction = transaction(7)?;
        let checkpoint = transaction.checkpoint();
        assert_eq!(
            sample_token(
                &[4.0, 3.0],
                &[0],
                &configuration,
                &mut transaction,
                &cancellation,
            )?,
            0
        );
        assert_eq!(transaction.checkpoint(), checkpoint);
        Ok(())
    }

    #[test]
    fn zero_repetition_penalty_fails_typed_without_rng_advance() -> Result<(), Box<dyn Error>> {
        let cancellation = CancellationToken::default();
        let mut configuration = sampling_configuration();
        configuration.repetition_penalty_bits = 0.0_f32.to_bits();
        let mut transaction = transaction(9)?;
        let checkpoint = transaction.checkpoint();
        assert!(matches!(
            sample_token(
                &[4.0, 0.0],
                &[0],
                &configuration,
                &mut transaction,
                &cancellation,
            ),
            Err(DecoderTextError::InvalidInput(
                "generation penalties produced non-finite logits"
            ))
        ));
        assert_eq!(transaction.checkpoint(), checkpoint);
        Ok(())
    }
}
