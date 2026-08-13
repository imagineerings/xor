use crate::{
    AttentionBackend, AttentionError, AttentionFallbackPolicy, AttentionRequest,
    BidirectionalTextError, BidirectionalTextOutput, BidirectionalTextRequest, ClipTextError,
    ClipTextOutput, ClipTextRequest, ClipVisionError, ClipVisionIntermediate, ClipVisionOutput,
    DecoderArchitecture, DecoderLayerKind, DecoderPreparedDeepstack,
    DecoderPreparedGenerationPrompt, DecoderRopePositions, DecoderTextConfiguration,
    DecoderTextError, DecoderTextOutput, DecoderTextRequest, GeluApproximation, NativeClipText,
    NativeClipVision, NativeDecoderTextEncoder, NativeModule, NativeOpsError,
    NativePromptTokenizer, NativeT5TextEncoder, NativeTextGenerationRequest,
    NativeTextGenerationResult, NativeTokenizerError, QWEN25_TOKENIZER_ARTIFACT_DIGEST,
    QWEN35_SOURCE_SHA256, QWEN35_TOKENIZER_ARTIFACT_DIGEST, Qwen2PretokenizerProfile,
    SD1_CLIP_SOURCE_SHA256, decoder_profile_fact, scaled_dot_product_attention_with_context,
};
use comfy_tensor::{
    CpuBackend, CpuWorkspaceVec, DType, DeviceId, ExecutionContext, ImageTensor, ResizeCrop,
    ResizeMode, RngTransaction, StreamId, Tensor, TensorDescriptor, TensorError,
    generated_indexing_masking_01::{IndexingMaskingPartOneError, narrow_method_exact_native},
    generated_native_diffusion::{
        NativeDiffusionTensorError, add as native_tensor_add, tensor_from_f32, tensor_to_f32,
    },
    generated_shape_layout_transform_02::{
        ShapeLayoutTransformPartTwoError, torch_cat_with_context_exact_native,
        torch_reshape_with_context_exact_native,
    },
    generated_shape_layout_transform_03::{
        ShapeLayoutTransformPartThreeError, tensor_permute_exact_native,
    },
    generated_spatial_functional_kernel_01::{
        InterpolateConfiguration, InterpolateMode, SpatialFunctionalKernelError,
        interpolate_with_context_exact_native,
    },
    generated_spectral_transform_01::{SpectralTransformError, fftn_with_context_exact_native},
};
use sha2::{Digest, Sha256};
use std::{mem, sync::Arc};
use thiserror::Error;

pub const IDEOGRAM4_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/text_encoders/ideogram4.py";
pub const IDEOGRAM4_SOURCE_SHA256: &str =
    "dfe31ad9c3204bf2f98c81cbc498284f661f4162f35486baed38a9a847c52343";
pub const JINA_CLIP2_SOURCE_PATH: &str =
    "projects/comfy/ComfyUI/comfy/text_encoders/jina_clip_2.py";
pub const JINA_CLIP2_SOURCE_SHA256: &str =
    "d5f8f32dc9ebcdc55956bc6249f115c78483555bdd895b82814376d06a6fb3d1";
pub const OVIS_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/text_encoders/ovis.py";
pub const OVIS_SOURCE_SHA256: &str =
    "80749123bfa24b2947fef548101a17cd73085e6743b07dd1cb3f30490f45b65f";
pub const QWEN3VL_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/text_encoders/qwen3vl.py";
pub const QWEN3VL_SOURCE_SHA256: &str =
    "b2dce382d1319926af148ed65939c3f731bd4c9a277fc39f8ef0ca3ce8af7d06";
pub const QWEN_VL_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/text_encoders/qwen_vl.py";
pub const QWEN_VL_SOURCE_SHA256: &str =
    "7edda1550592057ebf71deae3b5e7d5577502d68f26e578e41b367ca8824070f";
pub const SAM3_CLIP_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/text_encoders/sam3_clip.py";
pub const SAM3_CLIP_SOURCE_SHA256: &str =
    "86f1f28b88cc364adc585efdfc6b72947c09c16c1fe52c81ac543f9717fa60df";

pub const IDEOGRAM4_TAP_LAYERS: [usize; 13] = [1, 4, 7, 10, 13, 16, 19, 22, 25, 28, 31, 34, 36];
pub const QWEN3VL_4B_DEEPSTACK_LAYERS: [usize; 3] = [5, 11, 17];
pub const QWEN3VL_8B_DEEPSTACK_LAYERS: [usize; 3] = [8, 16, 24];
pub const QWEN2VL_FULL_ATTENTION_LAYERS: [usize; 4] = [7, 15, 23, 31];
pub const QWEN3VL_IMAGE_PAD_TOKEN: i64 = 151_655;
pub const QWEN35_IMAGE_PAD_TOKEN: i64 = 248_056;
pub const QWEN35_IMAGE_MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
pub const QWEN35_IMAGE_STANDARD_DEVIATION: [f32; 3] = [0.26862954, 0.26130258, 0.27577711];
pub const QWEN_MULTIMODAL_ROUTING_SOURCE_PATH: &str = "projects/comfy/ComfyUI/comfy/sd.py";
pub const QWEN_MULTIMODAL_ROUTING_SOURCE_SHA256: &str =
    "9c378edbcaab01d00397cc0ef1cab7d37d25fca49c707a87cde451030ac6bf42";
pub const QWEN3VL_IMAGE_MINIMUM_PIXELS: u64 = 3_136;
pub const QWEN3VL_IMAGE_MAXIMUM_PIXELS: u64 = 12_845_056;
pub const QWEN3VL_IMAGE_PATCH_SIZE: usize = 16;
pub const QWEN3VL_IMAGE_TEMPORAL_PATCH_SIZE: usize = 2;
pub const QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE: usize = 2;
pub const GEMMA3_IMAGE_AREA_PIXELS: u64 = 896 * 896;
pub const GEMMA3_MAXIMUM_PREPARED_PIXELS: u64 = GEMMA3_IMAGE_AREA_PIXELS * 4;
pub const GEMMA4_IMAGE_PATCH_SIZE: u64 = 16;
pub const GEMMA4_IMAGE_POOLING_SIZE: u64 = 3;
pub const GEMMA4_IMAGE_SOFT_TOKENS: usize = 280;
pub const GEMMA4_VIDEO_SOFT_TOKENS: usize = 70;
pub const GEMMA4_VIDEO_SOURCE_FPS: usize = 24;
pub const GEMMA4_AUDIO_SAMPLE_RATE: u32 = 16_000;
pub const GEMMA4_AUDIO_MINIMUM_SAMPLE_RATE: u32 = 8_000;
pub const GEMMA4_AUDIO_MAXIMUM_SAMPLE_RATE: u32 = 384_000;
pub const GEMMA4_AUDIO_FRAME_LENGTH: usize = 320;
pub const GEMMA4_AUDIO_FRAME_STEP: usize = 160;
pub const GEMMA4_AUDIO_FFT_LENGTH: usize = 512;
pub const GEMMA4_AUDIO_MEL_BINS: usize = 128;
pub const GEMMA4_AUDIO_MAXIMUM_TOKENS: usize = 750;

const GEMMA4_AUDIO_PADDING_MULTIPLE: usize = 128;
const GEMMA4_AUDIO_KAISER_BETA: f64 = 6.5;
const GEMMA4_AUDIO_FILTER_HALF_WIDTH: usize = 80;
const GEMMA4_AUDIO_MAXIMUM_FILTER_TAPS: usize = 2_000_001;
const GEMMA4_AUDIO_MAXIMUM_RESAMPLE_MULTIPLY_ADDS: usize = 250_000_000;

pub const MULTIMODAL_TEXT_ENCODER_CATALOG_SYMBOLS: [&str; 53] = [
    "Qwen3VLTokenizer",
    "Ideogram4Tokenizer",
    "Qwen3VL8BModel",
    "Ideogram4TEModel",
    "te",
    "Ideogram4Qwen3VLClipModel",
    "Ideogram4Qwen3VLTEModel",
    "Ideogram4Qwen3VLTokenizer",
    "te_qwen3vl",
    "JinaClip2Tokenizer",
    "JinaClip2TokenizerWrapper",
    "XLMRobertaConfig",
    "XLMRobertaEmbeddings",
    "RotaryEmbedding",
    "MHA",
    "MLP",
    "Block",
    "XLMRobertaEncoder",
    "XLMRobertaModel_",
    "XLMRobertaModel",
    "JinaClip2TextModel",
    "JinaClip2TextModelWrapper",
    "Qwen3Tokenizer",
    "OvisTokenizer",
    "Ovis25_2BModel",
    "OvisTEModel",
    "te",
    "Qwen3VLDeepstackMerger",
    "Qwen3VLVisionModel",
    "Qwen3VL",
    "_make_qwen3vl_model",
    "Qwen3VLClipModel",
    "Qwen3VLTEModel",
    "Qwen3VLSDTokenizer",
    "Qwen3VLTokenizer",
    "tokenizer",
    "te",
    "process_qwen2vl_images",
    "qwen2vl_mrope_position_ids",
    "VisionPatchEmbed",
    "rotate_half",
    "apply_rotary_pos_emb_vision",
    "VisionRotaryEmbedding",
    "PatchMerger",
    "VisionAttention",
    "VisionMLP",
    "VisionBlock",
    "Qwen2VLVisionTransformer",
    "SAM3ClipModel",
    "SAM3Tokenizer",
    "_parse_prompts",
    "SAM3TokenizerWrapper",
    "SAM3ClipModelWrapper",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MultimodalSymbolBehavior {
    Profile,
    ProfileFactory,
    TokenizerAdapter,
    BidirectionalTextDelegation,
    DecoderTextDelegation,
    ClipTextDelegation,
    VisionDelegation,
    ImagePreprocessDelegation,
    PositionConstruction,
    ModalityJoin,
    Projection,
    PromptPacking,
    ModelAdapter,
}

pub fn multimodal_symbol_behavior(
    source_path: &str,
    symbol: &str,
) -> Option<MultimodalSymbolBehavior> {
    use MultimodalSymbolBehavior as Behavior;
    Some(match (source_path, symbol) {
        (IDEOGRAM4_SOURCE_PATH, "Qwen3VLTokenizer")
        | (IDEOGRAM4_SOURCE_PATH, "Ideogram4Tokenizer")
        | (IDEOGRAM4_SOURCE_PATH, "Ideogram4Qwen3VLTokenizer")
        | (JINA_CLIP2_SOURCE_PATH, "JinaClip2Tokenizer")
        | (JINA_CLIP2_SOURCE_PATH, "JinaClip2TokenizerWrapper")
        | (OVIS_SOURCE_PATH, "Qwen3Tokenizer")
        | (OVIS_SOURCE_PATH, "OvisTokenizer")
        | (QWEN3VL_SOURCE_PATH, "Qwen3VLSDTokenizer")
        | (QWEN3VL_SOURCE_PATH, "Qwen3VLTokenizer")
        | (QWEN3VL_SOURCE_PATH, "tokenizer")
        | (SAM3_CLIP_SOURCE_PATH, "SAM3Tokenizer") => Behavior::TokenizerAdapter,
        (JINA_CLIP2_SOURCE_PATH, "XLMRobertaConfig") => Behavior::Profile,
        (QWEN3VL_SOURCE_PATH, "_make_qwen3vl_model") => Behavior::ProfileFactory,
        (JINA_CLIP2_SOURCE_PATH, "XLMRobertaEmbeddings")
        | (JINA_CLIP2_SOURCE_PATH, "MHA")
        | (JINA_CLIP2_SOURCE_PATH, "MLP")
        | (JINA_CLIP2_SOURCE_PATH, "Block")
        | (JINA_CLIP2_SOURCE_PATH, "XLMRobertaEncoder")
        | (JINA_CLIP2_SOURCE_PATH, "XLMRobertaModel_")
        | (JINA_CLIP2_SOURCE_PATH, "XLMRobertaModel") => Behavior::BidirectionalTextDelegation,
        (IDEOGRAM4_SOURCE_PATH, "Qwen3VL8BModel") | (OVIS_SOURCE_PATH, "Ovis25_2BModel") => {
            Behavior::DecoderTextDelegation
        }
        (SAM3_CLIP_SOURCE_PATH, "SAM3ClipModel") => Behavior::ClipTextDelegation,
        (QWEN3VL_SOURCE_PATH, "Qwen3VLVisionModel")
        | (QWEN_VL_SOURCE_PATH, "VisionPatchEmbed")
        | (QWEN_VL_SOURCE_PATH, "VisionAttention")
        | (QWEN_VL_SOURCE_PATH, "VisionMLP")
        | (QWEN_VL_SOURCE_PATH, "VisionBlock")
        | (QWEN_VL_SOURCE_PATH, "Qwen2VLVisionTransformer") => Behavior::VisionDelegation,
        (QWEN_VL_SOURCE_PATH, "process_qwen2vl_images") => Behavior::ImagePreprocessDelegation,
        (JINA_CLIP2_SOURCE_PATH, "RotaryEmbedding")
        | (QWEN_VL_SOURCE_PATH, "qwen2vl_mrope_position_ids")
        | (QWEN_VL_SOURCE_PATH, "rotate_half")
        | (QWEN_VL_SOURCE_PATH, "apply_rotary_pos_emb_vision")
        | (QWEN_VL_SOURCE_PATH, "VisionRotaryEmbedding") => Behavior::PositionConstruction,
        (QWEN3VL_SOURCE_PATH, "Qwen3VL") => Behavior::ModalityJoin,
        (IDEOGRAM4_SOURCE_PATH, "Ideogram4TEModel")
        | (IDEOGRAM4_SOURCE_PATH, "Ideogram4Qwen3VLTEModel")
        | (QWEN3VL_SOURCE_PATH, "Qwen3VLDeepstackMerger")
        | (QWEN_VL_SOURCE_PATH, "PatchMerger") => Behavior::Projection,
        (SAM3_CLIP_SOURCE_PATH, "_parse_prompts")
        | (SAM3_CLIP_SOURCE_PATH, "SAM3TokenizerWrapper")
        | (SAM3_CLIP_SOURCE_PATH, "SAM3ClipModelWrapper") => Behavior::PromptPacking,
        (IDEOGRAM4_SOURCE_PATH, "te")
        | (IDEOGRAM4_SOURCE_PATH, "Ideogram4Qwen3VLClipModel")
        | (IDEOGRAM4_SOURCE_PATH, "te_qwen3vl")
        | (JINA_CLIP2_SOURCE_PATH, "JinaClip2TextModel")
        | (JINA_CLIP2_SOURCE_PATH, "JinaClip2TextModelWrapper")
        | (OVIS_SOURCE_PATH, "OvisTEModel")
        | (OVIS_SOURCE_PATH, "te")
        | (QWEN3VL_SOURCE_PATH, "Qwen3VLClipModel")
        | (QWEN3VL_SOURCE_PATH, "Qwen3VLTEModel")
        | (QWEN3VL_SOURCE_PATH, "te") => Behavior::ModelAdapter,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MultimodalFamily {
    Ideogram4,
    JinaClip2,
    Ovis25,
    Qwen3Vl4B,
    Qwen3Vl8B,
    Qwen2Vl,
    Sam3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MultimodalTextOwner {
    Bidirectional,
    Decoder,
    ClipText,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MultimodalProfileFact {
    pub family: MultimodalFamily,
    pub text_owner: MultimodalTextOwner,
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub layer_count: usize,
    pub attention_heads: usize,
    pub maximum_tokens: usize,
    pub vocabulary_size: usize,
    pub pad_token: i64,
    pub projection_size: Option<usize>,
    pub patch_size: Option<usize>,
    pub temporal_patch_size: Option<usize>,
    pub spatial_merge_size: Option<usize>,
    pub visual_layer_count: Option<usize>,
    pub rope_theta_bits: Option<u32>,
    pub layer_taps: &'static [usize],
    pub deepstack_layers: &'static [usize],
}

impl MultimodalProfileFact {
    pub fn rope_theta(&self) -> Option<f32> {
        self.rope_theta_bits.map(f32::from_bits)
    }
}

pub const MULTIMODAL_PROFILE_FACTS: [MultimodalProfileFact; 7] = [
    MultimodalProfileFact {
        family: MultimodalFamily::Ideogram4,
        text_owner: MultimodalTextOwner::Decoder,
        hidden_size: 4096,
        intermediate_size: 12_288,
        layer_count: 36,
        attention_heads: 32,
        maximum_tokens: 99_999_999,
        vocabulary_size: 151_936,
        pad_token: 151_643,
        projection_size: Some(53_248),
        patch_size: Some(16),
        temporal_patch_size: Some(2),
        spatial_merge_size: Some(2),
        visual_layer_count: Some(27),
        rope_theta_bits: Some(5_000_000.0_f32.to_bits()),
        layer_taps: &IDEOGRAM4_TAP_LAYERS,
        deepstack_layers: &QWEN3VL_8B_DEEPSTACK_LAYERS,
    },
    MultimodalProfileFact {
        family: MultimodalFamily::JinaClip2,
        text_owner: MultimodalTextOwner::Bidirectional,
        hidden_size: 1024,
        intermediate_size: 4096,
        layer_count: 24,
        attention_heads: 16,
        maximum_tokens: 8192,
        vocabulary_size: 250_002,
        pad_token: 1,
        projection_size: None,
        patch_size: None,
        temporal_patch_size: None,
        spatial_merge_size: None,
        visual_layer_count: None,
        rope_theta_bits: Some(20_000.0_f32.to_bits()),
        layer_taps: &[],
        deepstack_layers: &[],
    },
    MultimodalProfileFact {
        family: MultimodalFamily::Ovis25,
        text_owner: MultimodalTextOwner::Decoder,
        hidden_size: 2048,
        intermediate_size: 6144,
        layer_count: 28,
        attention_heads: 16,
        maximum_tokens: 99_999_999,
        vocabulary_size: 151_936,
        pad_token: 151_643,
        projection_size: None,
        patch_size: None,
        temporal_patch_size: None,
        spatial_merge_size: None,
        visual_layer_count: None,
        rope_theta_bits: Some(1_000_000.0_f32.to_bits()),
        layer_taps: &[],
        deepstack_layers: &[],
    },
    MultimodalProfileFact {
        family: MultimodalFamily::Qwen3Vl4B,
        text_owner: MultimodalTextOwner::Decoder,
        hidden_size: 2560,
        intermediate_size: 9728,
        layer_count: 36,
        attention_heads: 32,
        maximum_tokens: 262_144,
        vocabulary_size: 151_936,
        pad_token: 151_643,
        projection_size: None,
        patch_size: Some(16),
        temporal_patch_size: Some(2),
        spatial_merge_size: Some(2),
        visual_layer_count: Some(24),
        rope_theta_bits: Some(5_000_000.0_f32.to_bits()),
        layer_taps: &[],
        deepstack_layers: &QWEN3VL_4B_DEEPSTACK_LAYERS,
    },
    MultimodalProfileFact {
        family: MultimodalFamily::Qwen3Vl8B,
        text_owner: MultimodalTextOwner::Decoder,
        hidden_size: 4096,
        intermediate_size: 12_288,
        layer_count: 36,
        attention_heads: 32,
        maximum_tokens: 262_144,
        vocabulary_size: 151_936,
        pad_token: 151_643,
        projection_size: None,
        patch_size: Some(16),
        temporal_patch_size: Some(2),
        spatial_merge_size: Some(2),
        visual_layer_count: Some(27),
        rope_theta_bits: Some(5_000_000.0_f32.to_bits()),
        layer_taps: &[],
        deepstack_layers: &QWEN3VL_8B_DEEPSTACK_LAYERS,
    },
    MultimodalProfileFact {
        family: MultimodalFamily::Qwen2Vl,
        text_owner: MultimodalTextOwner::Decoder,
        hidden_size: 3584,
        intermediate_size: 3420,
        layer_count: 32,
        attention_heads: 16,
        maximum_tokens: 128_000,
        vocabulary_size: 152_064,
        pad_token: 151_643,
        projection_size: Some(3584),
        patch_size: Some(14),
        temporal_patch_size: Some(2),
        spatial_merge_size: Some(2),
        visual_layer_count: Some(32),
        rope_theta_bits: Some(10_000.0_f32.to_bits()),
        layer_taps: &QWEN2VL_FULL_ATTENTION_LAYERS,
        deepstack_layers: &[],
    },
    MultimodalProfileFact {
        family: MultimodalFamily::Sam3,
        text_owner: MultimodalTextOwner::ClipText,
        hidden_size: 1024,
        intermediate_size: 4096,
        layer_count: 24,
        attention_heads: 16,
        maximum_tokens: 32,
        vocabulary_size: 49_408,
        pad_token: 0,
        projection_size: Some(512),
        patch_size: None,
        temporal_patch_size: None,
        spatial_merge_size: None,
        visual_layer_count: None,
        rope_theta_bits: None,
        layer_taps: &[],
        deepstack_layers: &[],
    },
];

pub fn multimodal_profile(family: MultimodalFamily) -> &'static MultimodalProfileFact {
    MULTIMODAL_PROFILE_FACTS
        .iter()
        .find(|profile| profile.family == family)
        .expect("every closed multimodal family has one immutable profile")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MultimodalSpan {
    pub start: usize,
    pub size: usize,
    pub grid_thw: [usize; 3],
}

#[derive(Clone, Debug)]
pub struct MultimodalImageEmbedding<'a> {
    pub span: MultimodalSpan,
    pub embedding: &'a Tensor,
    pub deepstack: &'a [Tensor],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultimodalPositionIds {
    temporal: Vec<i64>,
    height: Vec<i64>,
    width: Vec<i64>,
}

impl MultimodalPositionIds {
    pub fn temporal(&self) -> &[i64] {
        &self.temporal
    }

    pub fn height(&self) -> &[i64] {
        &self.height
    }

    pub fn width(&self) -> &[i64] {
        &self.width
    }

    pub fn len(&self) -> usize {
        self.temporal.len()
    }

    pub fn is_empty(&self) -> bool {
        self.temporal.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct MultimodalDeepstackJoin {
    pub visual_position_mask: Vec<bool>,
    pub layers: Vec<Tensor>,
}

#[derive(Clone, Debug)]
pub struct Qwen3VlPreparedImage {
    patches: Tensor,
    grid_thw: [usize; 3],
    merged_tokens: usize,
    family: QwenVisionFamily,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GemmaPreparedVisualKind {
    Gemma3Image,
    Gemma4Image,
    Gemma4VideoFrame,
}

#[derive(Clone, Debug)]
pub struct GemmaPreparedVisual {
    image: ImageTensor,
    kind: GemmaPreparedVisualKind,
    maximum_soft_tokens: usize,
    source_frame_index: usize,
    timestamp_seconds: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct GemmaPreparedAudio {
    log_mel: Tensor,
    frame_mask: Tensor,
    marker_tokens: usize,
    original_sample_rate: u32,
    original_samples: usize,
    resampled_samples: usize,
}

impl GemmaPreparedAudio {
    pub fn log_mel(&self) -> &Tensor {
        &self.log_mel
    }

    pub fn frame_mask(&self) -> &Tensor {
        &self.frame_mask
    }

    pub const fn marker_tokens(&self) -> usize {
        self.marker_tokens
    }

    pub const fn original_sample_rate(&self) -> u32 {
        self.original_sample_rate
    }

    pub const fn original_samples(&self) -> usize {
        self.original_samples
    }

    pub const fn resampled_samples(&self) -> usize {
        self.resampled_samples
    }
}

impl GemmaPreparedVisual {
    pub fn image(&self) -> &ImageTensor {
        &self.image
    }

    pub const fn kind(&self) -> GemmaPreparedVisualKind {
        self.kind
    }

    pub const fn maximum_soft_tokens(&self) -> usize {
        self.maximum_soft_tokens
    }

    pub const fn source_frame_index(&self) -> usize {
        self.source_frame_index
    }

    pub const fn timestamp_seconds(&self) -> Option<usize> {
        self.timestamp_seconds
    }
}

impl Qwen3VlPreparedImage {
    pub fn patches(&self) -> &Tensor {
        &self.patches
    }

    pub const fn grid_thw(&self) -> [usize; 3] {
        self.grid_thw
    }

    pub const fn merged_tokens(&self) -> usize {
        self.merged_tokens
    }

    pub const fn family(&self) -> QwenVisionFamily {
        self.family
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QwenVisionFamily {
    Qwen3Vl4B,
    Qwen3Vl8B,
    Qwen35_08B,
    Qwen35_2B,
    Qwen35_4B,
    Qwen35_9B,
    Qwen35_27B,
}

impl QwenVisionFamily {
    pub const fn image_pad_token(self) -> i64 {
        match self {
            Self::Qwen3Vl4B | Self::Qwen3Vl8B => QWEN3VL_IMAGE_PAD_TOKEN,
            Self::Qwen35_08B
            | Self::Qwen35_2B
            | Self::Qwen35_4B
            | Self::Qwen35_9B
            | Self::Qwen35_27B => QWEN35_IMAGE_PAD_TOKEN,
        }
    }

    pub const fn normalization(self) -> ([f32; 3], [f32; 3]) {
        match self {
            Self::Qwen3Vl4B | Self::Qwen3Vl8B => ([0.5; 3], [0.5; 3]),
            Self::Qwen35_08B
            | Self::Qwen35_2B
            | Self::Qwen35_4B
            | Self::Qwen35_9B
            | Self::Qwen35_27B => (QWEN35_IMAGE_MEAN, QWEN35_IMAGE_STANDARD_DEVIATION),
        }
    }

    pub const fn deepstack_layers(self) -> &'static [usize] {
        match self {
            Self::Qwen3Vl4B => &QWEN3VL_4B_DEEPSTACK_LAYERS,
            Self::Qwen3Vl8B => &QWEN3VL_8B_DEEPSTACK_LAYERS,
            _ => &[],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QwenVisionConfiguration {
    pub family: QwenVisionFamily,
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub layer_count: usize,
    pub attention_heads: usize,
    pub output_hidden_size: usize,
    pub position_embeddings: usize,
    pub spatial_merge_size: usize,
    pub source_exact_profile: bool,
}

impl QwenVisionConfiguration {
    pub fn source(family: QwenVisionFamily) -> Self {
        let (hidden_size, intermediate_size, layer_count, attention_heads, output_hidden_size) =
            match family {
                QwenVisionFamily::Qwen3Vl4B => (1024, 4096, 24, 16, 2560),
                QwenVisionFamily::Qwen3Vl8B => (1152, 4304, 27, 16, 4096),
                QwenVisionFamily::Qwen35_08B => (768, 3072, 12, 12, 1024),
                QwenVisionFamily::Qwen35_2B => (1024, 4096, 24, 16, 2048),
                QwenVisionFamily::Qwen35_4B => (1024, 4096, 24, 16, 2560),
                QwenVisionFamily::Qwen35_9B => (1152, 4304, 27, 16, 4096),
                QwenVisionFamily::Qwen35_27B => (1152, 4304, 27, 16, 5120),
            };
        Self {
            family,
            hidden_size,
            intermediate_size,
            layer_count,
            attention_heads,
            output_hidden_size,
            position_embeddings: 2304,
            spatial_merge_size: 2,
            source_exact_profile: true,
        }
    }

    pub fn reduced_fixture(
        family: QwenVisionFamily,
        hidden_size: usize,
        intermediate_size: usize,
        layer_count: usize,
        attention_heads: usize,
        output_hidden_size: usize,
    ) -> Self {
        Self {
            family,
            hidden_size,
            intermediate_size,
            layer_count,
            attention_heads,
            output_hidden_size,
            position_embeddings: 2304,
            spatial_merge_size: 2,
            source_exact_profile: false,
        }
    }

    fn validate(&self) -> Result<(), MultimodalTextError> {
        if self.hidden_size == 0
            || self.intermediate_size == 0
            || self.layer_count == 0
            || self.attention_heads == 0
            || !self.hidden_size.is_multiple_of(self.attention_heads)
            || self.output_hidden_size == 0
            || self.position_embeddings != 2304
            || self.spatial_merge_size != 2
        {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen vision configuration dimensions are invalid",
            ));
        }
        if self.source_exact_profile && self != &Self::source(self.family) {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen production vision configuration does not match its closed source profile",
            ));
        }
        let deepstack = self.family.deepstack_layers();
        if !deepstack.is_empty() && deepstack.iter().any(|layer| *layer >= self.layer_count) {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen deepstack capture layers exceed the vision depth",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct QwenVisionBlockWeights {
    pub normalization_one_weight: Tensor,
    pub normalization_one_bias: Tensor,
    pub query_key_value_weight: Tensor,
    pub query_key_value_bias: Tensor,
    pub attention_output_weight: Tensor,
    pub attention_output_bias: Tensor,
    pub normalization_two_weight: Tensor,
    pub normalization_two_bias: Tensor,
    pub feed_forward_up_weight: Tensor,
    pub feed_forward_up_bias: Tensor,
    pub feed_forward_down_weight: Tensor,
    pub feed_forward_down_bias: Tensor,
}

#[derive(Clone, Debug)]
pub struct QwenVisionMergerWeights {
    pub normalization_weight: Tensor,
    pub normalization_bias: Tensor,
    pub first_weight: Tensor,
    pub first_bias: Tensor,
    pub second_weight: Tensor,
    pub second_bias: Tensor,
}

#[derive(Clone, Debug)]
pub struct QwenVisionWeights {
    pub patch_weight: Tensor,
    pub patch_bias: Tensor,
    pub position_embedding: Tensor,
    pub blocks: Vec<QwenVisionBlockWeights>,
    pub merger: QwenVisionMergerWeights,
    pub deepstack_mergers: Vec<QwenVisionMergerWeights>,
}

#[derive(Clone, Debug)]
struct NativeQwenVisionBlock {
    normalization_one: NativeModule,
    query_key_value: NativeModule,
    attention_output: NativeModule,
    normalization_two: NativeModule,
    feed_forward_up: NativeModule,
    feed_forward_activation: NativeModule,
    feed_forward_down: NativeModule,
}

#[derive(Clone, Debug)]
struct NativeQwenVisionMerger {
    normalization: NativeModule,
    first: NativeModule,
    activation: NativeModule,
    second: NativeModule,
    normalization_after_merge: bool,
}

#[derive(Clone, Debug)]
pub struct NativeQwenVisionEncoder {
    configuration: QwenVisionConfiguration,
    patch_projection: NativeModule,
    position_embedding: Tensor,
    blocks: Vec<NativeQwenVisionBlock>,
    merger: NativeQwenVisionMerger,
    deepstack_mergers: Vec<NativeQwenVisionMerger>,
}

#[derive(Clone, Debug)]
pub struct QwenVisionProjection {
    pub embedding: Tensor,
    pub deepstack: Vec<Tensor>,
    pub grid_thw: [usize; 3],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Qwen3VlMarkerPlan {
    expanded_tokens: Vec<i64>,
    spans: Vec<MultimodalSpan>,
    visual_position_mask: Vec<bool>,
}

pub struct QwenMultimodalGenerationRequest<'a> {
    pub text: NativeTextGenerationRequest<'a>,
    pub prepared_images: &'a [Qwen3VlPreparedImage],
    pub transaction: &'a RngTransaction,
}

impl Qwen3VlMarkerPlan {
    pub fn expanded_tokens(&self) -> &[i64] {
        &self.expanded_tokens
    }

    pub fn spans(&self) -> &[MultimodalSpan] {
        &self.spans
    }

    pub fn visual_position_mask(&self) -> &[bool] {
        &self.visual_position_mask
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sam3Prompt {
    pub text: String,
    pub maximum_detections: usize,
}

#[derive(Clone, Debug)]
pub struct Sam3EncodedCondition {
    pub condition: Tensor,
    pub attention_mask: Option<Tensor>,
    pub maximum_detections: usize,
}

#[derive(Clone, Debug)]
pub struct Sam3ConditionPack {
    pub main_condition: Tensor,
    pub first_pooled: Option<Tensor>,
    pub conditions: Vec<Sam3EncodedCondition>,
}

#[derive(Debug, Error)]
pub enum MultimodalTextError {
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error(transparent)]
    Indexing(#[from] IndexingMaskingPartOneError),
    #[error(transparent)]
    ShapeLayoutTwo(#[from] ShapeLayoutTransformPartTwoError),
    #[error(transparent)]
    ShapeLayoutThree(#[from] ShapeLayoutTransformPartThreeError),
    #[error(transparent)]
    Bidirectional(#[from] BidirectionalTextError),
    #[error(transparent)]
    Decoder(#[from] DecoderTextError),
    #[error(transparent)]
    Tokenizer(#[from] NativeTokenizerError),
    #[error(transparent)]
    ClipText(#[from] ClipTextError),
    #[error(transparent)]
    ClipVision(#[from] ClipVisionError),
    #[error(transparent)]
    Attention(#[from] AttentionError),
    #[error(transparent)]
    NativeOps(#[from] NativeOpsError),
    #[error(transparent)]
    NativeTensor(#[from] NativeDiffusionTensorError),
    #[error(transparent)]
    Spatial(#[from] SpatialFunctionalKernelError),
    #[error(transparent)]
    Spectral(#[from] SpectralTransformError),
    #[error("multimodal text input is invalid: {0}")]
    InvalidInput(&'static str),
    #[error("multimodal text arithmetic overflowed while computing {0}")]
    Overflow(&'static str),
    #[error("SAM3 prompt limit {0:?} is not a finite nonnegative number")]
    InvalidPromptLimit(String),
    #[error("multimodal text execution was cancelled")]
    Cancelled,
}

impl From<comfy_types::CancellationError> for MultimodalTextError {
    fn from(_: comfy_types::CancellationError) -> Self {
        Self::Cancelled
    }
}

pub fn run_bidirectional_text_owner(
    owner: &NativeT5TextEncoder,
    backend: &CpuBackend,
    request: BidirectionalTextRequest<'_>,
    context: &ExecutionContext<'_>,
) -> Result<BidirectionalTextOutput, MultimodalTextError> {
    context.cancellation.check()?;
    Ok(owner.forward(backend, request, context)?)
}

pub fn run_decoder_text_owner(
    owner: &NativeDecoderTextEncoder,
    backend: &CpuBackend,
    request: DecoderTextRequest<'_>,
    context: &ExecutionContext<'_>,
) -> Result<DecoderTextOutput, MultimodalTextError> {
    context.cancellation.check()?;
    Ok(owner.forward(backend, request, context)?)
}

pub fn run_clip_text_owner(
    owner: &NativeClipText,
    backend: &CpuBackend,
    request: ClipTextRequest<'_>,
    context: &ExecutionContext<'_>,
) -> Result<ClipTextOutput, MultimodalTextError> {
    context.cancellation.check()?;
    Ok(owner.forward(backend, request, context)?)
}

pub fn run_clip_vision_owner(
    owner: &mut NativeClipVision,
    backend: &CpuBackend,
    image: &Tensor,
    crop: bool,
    intermediate: ClipVisionIntermediate,
    context: &ExecutionContext<'_>,
) -> Result<ClipVisionOutput, MultimodalTextError> {
    context.cancellation.check()?;
    let pixels = owner.preprocess(backend, image, crop, context)?;
    Ok(owner.forward(backend, &pixels, intermediate, context)?)
}

pub fn qwen2vl_mrope_position_ids(
    sequence_length: usize,
    spans: &[MultimodalSpan],
    cancellation: &comfy_types::CancellationToken,
) -> Result<Option<MultimodalPositionIds>, MultimodalTextError> {
    cancellation.check()?;
    if spans.is_empty() {
        return Ok(None);
    }
    let mut temporal = sequential_positions(sequence_length)?;
    let mut height = clone_positions(&temporal)?;
    let mut width = clone_positions(&temporal)?;
    let mut previous_end = 0_usize;
    let mut offset = 0_i64;
    for (span_index, span) in spans.iter().enumerate() {
        if span_index.is_multiple_of(32) {
            cancellation.check()?;
        }
        let end = span
            .start
            .checked_add(span.size)
            .ok_or(MultimodalTextError::Overflow("image span end"))?;
        if span.size == 0
            || span.start < previous_end
            || end > sequence_length
            || span.grid_thw.contains(&0)
            || span.grid_thw[1] / 2 == 0
            || span.grid_thw[2] / 2 == 0
        {
            return Err(MultimodalTextError::InvalidInput(
                "image spans must be ordered, nonempty, in bounds, and have positive half grids",
            ));
        }
        let start = usize_to_i64(span.start, "image start")?;
        let size = usize_to_i64(span.size, "image size")?;
        let len_max = usize_to_i64(
            *span
                .grid_thw
                .iter()
                .max()
                .ok_or(MultimodalTextError::InvalidInput("image grid is empty"))?
                / 2,
            "image grid maximum",
        )?;
        let start_value = start
            .checked_add(offset)
            .ok_or(MultimodalTextError::Overflow("image position start"))?;
        let height_extent = span.grid_thw[1] / 2;
        let width_extent = span.grid_thw[2] / 2;
        let height_repeat = span
            .size
            .checked_add(height_extent - 1)
            .and_then(|value| value.checked_div(height_extent))
            .ok_or(MultimodalTextError::Overflow("height repeat"))?;
        for local in 0..span.size {
            let index = span.start + local;
            temporal[index] = start_value;
            height[index] = start_value
                .checked_add(usize_to_i64(local / height_repeat, "height position")?)
                .ok_or(MultimodalTextError::Overflow("height position"))?;
            width[index] = start_value
                .checked_add(usize_to_i64(local % width_extent, "width position")?)
                .ok_or(MultimodalTextError::Overflow("width position"))?;
        }
        offset = offset
            .checked_add(len_max)
            .and_then(|value| value.checked_sub(size))
            .ok_or(MultimodalTextError::Overflow("multimodal position offset"))?;
        let suffix_start = start
            .checked_add(len_max)
            .and_then(|value| value.checked_add(offset - (len_max - size)))
            .ok_or(MultimodalTextError::Overflow("suffix position"))?;
        for index in end..sequence_length {
            let delta = usize_to_i64(index - end, "suffix offset")?;
            let value = suffix_start
                .checked_add(delta)
                .ok_or(MultimodalTextError::Overflow("suffix position"))?;
            temporal[index] = value;
            height[index] = value;
            width[index] = value;
        }
        previous_end = end;
    }
    cancellation.check()?;
    Ok(Some(MultimodalPositionIds {
        temporal,
        height,
        width,
    }))
}

pub fn qwen3vl_target_dimensions(
    height: u64,
    width: u64,
) -> Result<(u64, u64), MultimodalTextError> {
    if height == 0 || width == 0 {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL images require nonzero dimensions",
        ));
    }
    let factor = u64::try_from(
        QWEN3VL_IMAGE_PATCH_SIZE
            .checked_mul(QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE)
            .ok_or(MultimodalTextError::Overflow("Qwen3-VL resize factor"))?,
    )
    .map_err(|_| MultimodalTextError::Overflow("Qwen3-VL resize factor"))?;
    let mut target_height = round_to_factor(height, factor)?;
    let mut target_width = round_to_factor(width, factor)?;
    let rounded_pixels = target_height
        .checked_mul(target_width)
        .ok_or(MultimodalTextError::Overflow("Qwen3-VL rounded pixels"))?;
    let source_pixels = height
        .checked_mul(width)
        .ok_or(MultimodalTextError::Overflow("Qwen3-VL source pixels"))?;
    if rounded_pixels > QWEN3VL_IMAGE_MAXIMUM_PIXELS {
        let beta = ((source_pixels as f64) / (QWEN3VL_IMAGE_MAXIMUM_PIXELS as f64)).sqrt();
        target_height = floor_scaled_to_factor(height, beta, factor)?;
        target_width = floor_scaled_to_factor(width, beta, factor)?;
    } else if rounded_pixels < QWEN3VL_IMAGE_MINIMUM_PIXELS {
        let beta = ((QWEN3VL_IMAGE_MINIMUM_PIXELS as f64) / (source_pixels as f64)).sqrt();
        target_height = ceil_scaled_to_factor(height, beta, factor)?;
        target_width = ceil_scaled_to_factor(width, beta, factor)?;
    }
    if target_height == 0
        || target_width == 0
        || !target_height.is_multiple_of(factor)
        || !target_width.is_multiple_of(factor)
    {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL target dimensions are not positive factor-aligned values",
        ));
    }
    if target_height
        .checked_mul(target_width)
        .ok_or(MultimodalTextError::Overflow("Qwen3-VL target pixels"))?
        > QWEN3VL_IMAGE_MAXIMUM_PIXELS
    {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL target dimensions exceed the native bounded pixel limit",
        ));
    }
    Ok((target_height, target_width))
}

pub fn gemma3_target_dimensions(
    height: u64,
    width: u64,
) -> Result<(u64, u64), MultimodalTextError> {
    let source_pixels = height
        .checked_mul(width)
        .ok_or(MultimodalTextError::Overflow("Gemma3 source pixels"))?;
    if source_pixels == 0 {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma3 images require nonzero dimensions",
        ));
    }
    let scale = ((GEMMA3_IMAGE_AREA_PIXELS as f64) / (source_pixels as f64)).sqrt();
    let target_height = checked_f64_to_u64(
        ((height as f64) * scale).round_ties_even().max(1.0),
        "Gemma3 target height",
    )?;
    let target_width = checked_f64_to_u64(
        ((width as f64) * scale).round_ties_even().max(1.0),
        "Gemma3 target width",
    )?;
    let target_pixels = target_height
        .checked_mul(target_width)
        .ok_or(MultimodalTextError::Overflow("Gemma3 target pixels"))?;
    if target_pixels > GEMMA3_MAXIMUM_PREPARED_PIXELS {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma3 target dimensions exceed the native bounded pixel limit",
        ));
    }
    Ok((target_height, target_width))
}

pub fn prepare_gemma3_image(
    backend: &CpuBackend,
    image: &ImageTensor,
    context: &ExecutionContext<'_>,
) -> Result<GemmaPreparedVisual, MultimodalTextError> {
    context.cancellation.check()?;
    let (batch, height, width, channels) = image.dimensions()?;
    if batch == 0 || channels < 3 {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma3 requires a nonempty RGB or RGBA IMAGE batch",
        ));
    }
    let image = project_rgb_channels(backend, image, context)?;
    let (target_height, target_width) = gemma3_target_dimensions(height, width)?;
    let image = image.resize(
        target_width,
        target_height,
        ResizeMode::Area,
        ResizeCrop::Disabled,
        backend,
        context,
    )?;
    context.cancellation.check()?;
    Ok(GemmaPreparedVisual {
        image,
        kind: GemmaPreparedVisualKind::Gemma3Image,
        maximum_soft_tokens: 256,
        source_frame_index: 0,
        timestamp_seconds: None,
    })
}

pub fn gemma4_target_dimensions(
    height: u64,
    width: u64,
    maximum_soft_tokens: usize,
) -> Result<(u64, u64), MultimodalTextError> {
    let source_pixels = height
        .checked_mul(width)
        .ok_or(MultimodalTextError::Overflow("Gemma4 source pixels"))?;
    if source_pixels == 0 || maximum_soft_tokens == 0 {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 visual dimensions and soft-token budget must be nonzero",
        ));
    }
    let maximum_soft_tokens = u64::try_from(maximum_soft_tokens)
        .map_err(|_| MultimodalTextError::Overflow("Gemma4 soft-token budget"))?;
    let side_multiple = GEMMA4_IMAGE_PATCH_SIZE
        .checked_mul(GEMMA4_IMAGE_POOLING_SIZE)
        .ok_or(MultimodalTextError::Overflow("Gemma4 side multiple"))?;
    let target_pixels = maximum_soft_tokens
        .checked_mul(GEMMA4_IMAGE_POOLING_SIZE)
        .and_then(|value| value.checked_mul(GEMMA4_IMAGE_POOLING_SIZE))
        .and_then(|value| value.checked_mul(GEMMA4_IMAGE_PATCH_SIZE))
        .and_then(|value| value.checked_mul(GEMMA4_IMAGE_PATCH_SIZE))
        .ok_or(MultimodalTextError::Overflow("Gemma4 target pixels"))?;
    let scale = ((target_pixels as f64) / (source_pixels as f64)).sqrt();
    let target_height = checked_f64_to_u64(
        (((height as f64) * scale) / (side_multiple as f64)).floor(),
        "Gemma4 target height units",
    )?
    .checked_mul(side_multiple)
    .ok_or(MultimodalTextError::Overflow("Gemma4 target height"))?
    .max(side_multiple);
    let target_width = checked_f64_to_u64(
        (((width as f64) * scale) / (side_multiple as f64)).floor(),
        "Gemma4 target width units",
    )?
    .checked_mul(side_multiple)
    .ok_or(MultimodalTextError::Overflow("Gemma4 target width"))?
    .max(side_multiple);
    let prepared_pixels = target_height
        .checked_mul(target_width)
        .ok_or(MultimodalTextError::Overflow("Gemma4 target dimensions"))?;
    let maximum_prepared_pixels =
        target_pixels
            .checked_mul(4)
            .ok_or(MultimodalTextError::Overflow(
                "Gemma4 maximum prepared pixels",
            ))?;
    if prepared_pixels > maximum_prepared_pixels {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 target dimensions exceed the native bounded pixel limit",
        ));
    }
    Ok((target_height, target_width))
}

pub fn prepare_gemma4_visuals(
    backend: &CpuBackend,
    image: Option<&ImageTensor>,
    video: Option<&ImageTensor>,
    context: &ExecutionContext<'_>,
) -> Result<Vec<GemmaPreparedVisual>, MultimodalTextError> {
    context.cancellation.check()?;
    let (source, kind, maximum_soft_tokens, frame_step) = match (image, video) {
        (_, Some(video)) => (
            video,
            GemmaPreparedVisualKind::Gemma4VideoFrame,
            GEMMA4_VIDEO_SOFT_TOKENS,
            GEMMA4_VIDEO_SOURCE_FPS,
        ),
        (Some(image), None) => (
            image,
            GemmaPreparedVisualKind::Gemma4Image,
            GEMMA4_IMAGE_SOFT_TOKENS,
            1,
        ),
        (None, None) => return Ok(Vec::new()),
    };
    let (batch, height, width, channels) = source.dimensions()?;
    if batch == 0 || channels < 3 {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 requires a nonempty RGB or RGBA IMAGE batch",
        ));
    }
    let source = project_rgb_channels(backend, source, context)?;
    let (target_height, target_width) =
        gemma4_target_dimensions(height, width, maximum_soft_tokens)?;
    let batch = u64_to_usize(batch, "Gemma4 visual batch")?;
    let frame_elements = height
        .checked_mul(width)
        .and_then(|value| value.checked_mul(3))
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(MultimodalTextError::Overflow("Gemma4 frame elements"))?;
    let source_values = source.as_f32_slice()?;
    let selected_count = batch
        .checked_add(frame_step - 1)
        .ok_or(MultimodalTextError::Overflow("Gemma4 selected frame count"))?
        / frame_step;
    let mut prepared = Vec::new();
    prepared
        .try_reserve_exact(selected_count)
        .map_err(|_| MultimodalTextError::Overflow("Gemma4 prepared frames"))?;
    for (timestamp_seconds, source_frame_index) in (0..batch).step_by(frame_step).enumerate() {
        context.cancellation.check()?;
        let start = source_frame_index
            .checked_mul(frame_elements)
            .ok_or(MultimodalTextError::Overflow("Gemma4 frame offset"))?;
        let end = start
            .checked_add(frame_elements)
            .ok_or(MultimodalTextError::Overflow("Gemma4 frame end"))?;
        let values = source_values
            .get(start..end)
            .ok_or(MultimodalTextError::InvalidInput(
                "Gemma4 frame storage is incomplete",
            ))?;
        let image = gemma4_quantized_bicubic_resize(
            backend,
            values,
            height,
            width,
            target_height,
            target_width,
            context,
        )?;
        prepared.push(GemmaPreparedVisual {
            image,
            kind,
            maximum_soft_tokens,
            source_frame_index,
            timestamp_seconds: (kind == GemmaPreparedVisualKind::Gemma4VideoFrame)
                .then_some(timestamp_seconds),
        });
    }
    context.cancellation.check()?;
    Ok(prepared)
}

pub fn prepare_gemma4_audio(
    backend: &CpuBackend,
    waveform: &Tensor,
    sample_rate: u32,
    context: &ExecutionContext<'_>,
) -> Result<GemmaPreparedAudio, MultimodalTextError> {
    context.cancellation.check()?;
    if !(GEMMA4_AUDIO_MINIMUM_SAMPLE_RATE..=GEMMA4_AUDIO_MAXIMUM_SAMPLE_RATE).contains(&sample_rate)
    {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 audio sample rate is outside the checked AUDIO range",
        ));
    }
    let descriptor = waveform.descriptor();
    let shape = descriptor.shape();
    if shape.len() != 3 || shape[0] != 1 || shape[1] == 0 || shape[2] == 0 {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 audio requires contiguous [1, channels, samples] AUDIO",
        ));
    }
    if descriptor.dtype() != DType::F32
        || descriptor.device() != DeviceId::CPU
        || descriptor.stream() != context.stream
        || !descriptor.is_contiguous()?
    {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 audio must be contiguous CPU F32 on the execution stream",
        ));
    }
    let channels = u64_to_usize(shape[1], "Gemma4 audio channels")?;
    let original_samples = u64_to_usize(shape[2], "Gemma4 audio samples")?;
    let source = tensor_to_f32(backend, waveform, context)?;
    let expected_source_values = channels
        .checked_mul(original_samples)
        .ok_or(MultimodalTextError::Overflow("Gemma4 audio source values"))?;
    if source.len() != expected_source_values {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 audio storage does not match its descriptor",
        ));
    }
    let mut mono = backend.workspace_vec(context, original_samples)?;
    for sample in 0..original_samples {
        check_gemma_audio_periodically(sample, context)?;
        let mut sum = 0.0_f64;
        for channel in 0..channels {
            let index = channel
                .checked_mul(original_samples)
                .and_then(|value| value.checked_add(sample))
                .ok_or(MultimodalTextError::Overflow("Gemma4 mono sample index"))?;
            let value = *source.get(index).ok_or(MultimodalTextError::InvalidInput(
                "Gemma4 audio channel storage is incomplete",
            ))?;
            if !value.is_finite() {
                return Err(MultimodalTextError::InvalidInput(
                    "Gemma4 audio samples must be finite",
                ));
            }
            sum += f64::from(value);
        }
        mono.try_push((sum / channels as f64) as f32)?;
    }
    let resampled = gemma4_resample_polyphase(backend, &mono, sample_rate, context)?;
    let resampled_samples = resampled.len();
    let marker_tokens = gemma4_audio_marker_tokens(original_samples, sample_rate)?;
    let padded_samples = resampled_samples
        .checked_add(GEMMA4_AUDIO_PADDING_MULTIPLE - 1)
        .ok_or(MultimodalTextError::Overflow("Gemma4 padded audio samples"))?
        / GEMMA4_AUDIO_PADDING_MULTIPLE
        * GEMMA4_AUDIO_PADDING_MULTIPLE;
    let semicausal_samples = padded_samples.checked_add(GEMMA4_AUDIO_FRAME_STEP).ok_or(
        MultimodalTextError::Overflow("Gemma4 semicausal audio samples"),
    )?;
    let required_frame_samples = GEMMA4_AUDIO_FRAME_LENGTH
        .checked_add(1)
        .ok_or(MultimodalTextError::Overflow("Gemma4 frame samples"))?;
    let frame_count = if semicausal_samples < required_frame_samples {
        0
    } else {
        (semicausal_samples - required_frame_samples) / GEMMA4_AUDIO_FRAME_STEP + 1
    };
    let (log_mel, frame_mask) =
        gemma4_log_mel(backend, &resampled, resampled_samples, frame_count, context)?;
    context.cancellation.check()?;
    Ok(GemmaPreparedAudio {
        log_mel,
        frame_mask,
        marker_tokens,
        original_sample_rate: sample_rate,
        original_samples,
        resampled_samples,
    })
}

pub fn gemma4_audio_marker_tokens(
    original_samples: usize,
    sample_rate: u32,
) -> Result<usize, MultimodalTextError> {
    if !(GEMMA4_AUDIO_MINIMUM_SAMPLE_RATE..=GEMMA4_AUDIO_MAXIMUM_SAMPLE_RATE).contains(&sample_rate)
    {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 audio sample rate is outside the checked AUDIO range",
        ));
    }
    let projected_samples = if sample_rate == GEMMA4_AUDIO_SAMPLE_RATE {
        original_samples
    } else {
        original_samples
            .checked_mul(GEMMA4_AUDIO_SAMPLE_RATE as usize)
            .ok_or(MultimodalTextError::Overflow(
                "Gemma4 projected audio samples",
            ))?
            / sample_rate as usize
    };
    let projected_with_padding = projected_samples
        .checked_add(GEMMA4_AUDIO_FRAME_STEP)
        .ok_or(MultimodalTextError::Overflow(
            "Gemma4 projected audio frame count",
        ))?;
    let frame_count = if projected_with_padding < GEMMA4_AUDIO_FRAME_LENGTH + 1 {
        0
    } else {
        (projected_with_padding - (GEMMA4_AUDIO_FRAME_LENGTH + 1)) / GEMMA4_AUDIO_FRAME_STEP + 1
    };
    let once = frame_count.div_ceil(2);
    Ok(once.div_ceil(2).min(GEMMA4_AUDIO_MAXIMUM_TOKENS))
}

fn gemma4_resample_polyphase(
    backend: &CpuBackend,
    mono: &[f32],
    sample_rate: u32,
    context: &ExecutionContext<'_>,
) -> Result<CpuWorkspaceVec<f32>, MultimodalTextError> {
    if sample_rate == GEMMA4_AUDIO_SAMPLE_RATE {
        let mut output = backend.workspace_vec(context, mono.len())?;
        for (index, value) in mono.iter().copied().enumerate() {
            check_gemma_audio_periodically(index, context)?;
            output.try_push(value)?;
        }
        return Ok(output);
    }
    let divisor = greatest_common_divisor(sample_rate, GEMMA4_AUDIO_SAMPLE_RATE);
    let up = usize::try_from(GEMMA4_AUDIO_SAMPLE_RATE / divisor)
        .map_err(|_| MultimodalTextError::Overflow("Gemma4 resample up factor"))?;
    let down = usize::try_from(sample_rate / divisor)
        .map_err(|_| MultimodalTextError::Overflow("Gemma4 resample down factor"))?;
    let ratio_limit = up.max(down);
    let half_length = GEMMA4_AUDIO_FILTER_HALF_WIDTH
        .checked_mul(ratio_limit)
        .ok_or(MultimodalTextError::Overflow(
            "Gemma4 resample filter half length",
        ))?;
    let filter_length = half_length
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or(MultimodalTextError::Overflow(
            "Gemma4 resample filter length",
        ))?;
    if filter_length > GEMMA4_AUDIO_MAXIMUM_FILTER_TAPS {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 resample ratio exceeds the native bounded filter limit",
        ));
    }
    let output_length = mono
        .len()
        .checked_mul(up)
        .and_then(|value| value.checked_add(down - 1))
        .ok_or(MultimodalTextError::Overflow("Gemma4 resampled length"))?
        / down;
    let taps_per_output = filter_length
        .checked_add(up - 1)
        .ok_or(MultimodalTextError::Overflow("Gemma4 resample work"))?
        / up;
    let multiply_adds = output_length
        .checked_mul(taps_per_output)
        .ok_or(MultimodalTextError::Overflow("Gemma4 resample work"))?;
    if multiply_adds > GEMMA4_AUDIO_MAXIMUM_RESAMPLE_MULTIPLY_ADDS {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 audio exceeds the native bounded resample work limit",
        ));
    }
    let cutoff = 0.96 / ratio_limit as f64;
    let kaiser_denominator = gemma4_bessel_i0(GEMMA4_AUDIO_KAISER_BETA);
    let mut normalization = 0.0_f64;
    for index in 0..filter_length {
        check_gemma_audio_periodically(index, context)?;
        normalization += gemma4_firwin_value(index, half_length, cutoff, kaiser_denominator);
    }
    if !normalization.is_finite() || normalization <= 0.0 {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 resample filter normalization is invalid",
        ));
    }
    let pre_padding = down - half_length % down;
    let pre_remove = (half_length + pre_padding) / down;
    let mut output = backend.workspace_vec(context, output_length)?;
    let up_i128 = i128::try_from(up)
        .map_err(|_| MultimodalTextError::Overflow("Gemma4 resample up factor"))?;
    let down_i128 = i128::try_from(down)
        .map_err(|_| MultimodalTextError::Overflow("Gemma4 resample down factor"))?;
    let pre_padding_i128 = i128::try_from(pre_padding)
        .map_err(|_| MultimodalTextError::Overflow("Gemma4 resample pre-padding"))?;
    let filter_last_i128 = i128::try_from(filter_length - 1)
        .map_err(|_| MultimodalTextError::Overflow("Gemma4 resample filter length"))?;
    for output_index in 0..output_length {
        check_gemma_audio_periodically(output_index, context)?;
        let projected_output_index =
            output_index
                .checked_add(pre_remove)
                .ok_or(MultimodalTextError::Overflow(
                    "Gemma4 resample output index",
                ))?;
        let raw_index = i128::try_from(projected_output_index)
            .map_err(|_| MultimodalTextError::Overflow("Gemma4 resample output index"))?
            .checked_mul(down_i128)
            .and_then(|value| value.checked_sub(pre_padding_i128))
            .ok_or(MultimodalTextError::Overflow(
                "Gemma4 resample convolution index",
            ))?;
        let first_input = div_ceil_i128(raw_index - filter_last_i128, up_i128).max(0);
        let last_input = raw_index.div_euclid(up_i128).min(
            i128::try_from(mono.len() - 1)
                .map_err(|_| MultimodalTextError::Overflow("Gemma4 audio samples"))?,
        );
        let mut sum = 0.0_f64;
        if first_input <= last_input {
            for input_index in first_input..=last_input {
                let filter_index = raw_index
                    .checked_sub(input_index.checked_mul(up_i128).ok_or(
                        MultimodalTextError::Overflow("Gemma4 resample filter index"),
                    )?)
                    .ok_or(MultimodalTextError::Overflow(
                        "Gemma4 resample filter index",
                    ))?;
                let filter_index = usize::try_from(filter_index)
                    .map_err(|_| MultimodalTextError::Overflow("Gemma4 resample filter index"))?;
                let input_index = usize::try_from(input_index)
                    .map_err(|_| MultimodalTextError::Overflow("Gemma4 resample input index"))?;
                let input = f64::from(*mono.get(input_index).ok_or(
                    MultimodalTextError::InvalidInput("Gemma4 resample input is incomplete"),
                )?);
                let coefficient =
                    gemma4_firwin_value(filter_index, half_length, cutoff, kaiser_denominator)
                        / normalization
                        * up as f64;
                sum += input * coefficient;
            }
        }
        output.try_push(sum as f32)?;
    }
    Ok(output)
}

fn gemma4_log_mel(
    backend: &CpuBackend,
    audio: &[f32],
    real_samples: usize,
    frame_count: usize,
    context: &ExecutionContext<'_>,
) -> Result<(Tensor, Tensor), MultimodalTextError> {
    let frame_values = frame_count
        .checked_mul(GEMMA4_AUDIO_FFT_LENGTH)
        .ok_or(MultimodalTextError::Overflow("Gemma4 FFT frame values"))?;
    let mut framed = backend.workspace_vec(context, frame_values)?;
    let mut mask = backend.workspace_vec::<u8>(context, frame_count)?;
    for frame in 0..frame_count {
        check_gemma_audio_periodically(frame, context)?;
        let frame_start = frame
            .checked_mul(GEMMA4_AUDIO_FRAME_STEP)
            .ok_or(MultimodalTextError::Overflow("Gemma4 frame start"))?;
        let mask_index = frame_start
            .checked_add(GEMMA4_AUDIO_FRAME_LENGTH)
            .ok_or(MultimodalTextError::Overflow("Gemma4 frame mask index"))?;
        let source_mask_index = mask_index.checked_sub(GEMMA4_AUDIO_FRAME_STEP);
        mask.try_push(u8::from(
            source_mask_index.is_some_and(|index| index < real_samples),
        ))?;
        for offset in 0..GEMMA4_AUDIO_FFT_LENGTH {
            let value = if offset < GEMMA4_AUDIO_FRAME_LENGTH {
                let padded_index = frame_start
                    .checked_add(offset)
                    .ok_or(MultimodalTextError::Overflow("Gemma4 frame sample"))?;
                let source_index = padded_index.checked_sub(GEMMA4_AUDIO_FRAME_STEP);
                let sample = source_index
                    .and_then(|index| audio.get(index))
                    .copied()
                    .unwrap_or(0.0);
                let phase =
                    std::f64::consts::TAU * offset as f64 / GEMMA4_AUDIO_FRAME_LENGTH as f64;
                sample * (0.5 - 0.5 * phase.cos()) as f32
            } else {
                0.0
            };
            framed.try_push(value)?;
        }
    }
    let mut features = backend.workspace_vec(
        context,
        frame_count
            .checked_mul(GEMMA4_AUDIO_MEL_BINS)
            .ok_or(MultimodalTextError::Overflow("Gemma4 log-mel values"))?,
    )?;
    if frame_count > 0 {
        let framed = tensor_from_f32(
            backend,
            &[usize_to_u64(frame_count, "Gemma4 audio frames")?, 512],
            &framed,
            context,
        )?;
        let spectrum = fftn_with_context_exact_native(backend, &framed, &[1], context)?;
        let spectrum_bytes = spectrum.contiguous_bytes()?;
        let expected_complex = frame_count
            .checked_mul(GEMMA4_AUDIO_FFT_LENGTH)
            .ok_or(MultimodalTextError::Overflow("Gemma4 spectrum values"))?;
        if spectrum_bytes.len()
            != expected_complex
                .checked_mul(8)
                .ok_or(MultimodalTextError::Overflow("Gemma4 spectrum bytes"))?
        {
            return Err(MultimodalTextError::InvalidInput(
                "Gemma4 spectrum storage is malformed",
            ));
        }
        let filterbank = gemma4_mel_filterbank(backend, context)?;
        let mut magnitudes = [0.0_f64; 257];
        for frame in 0..frame_count {
            check_gemma_audio_periodically(frame, context)?;
            if *mask.get(frame).ok_or(MultimodalTextError::InvalidInput(
                "Gemma4 frame mask is incomplete",
            ))? == 0
            {
                for _ in 0..GEMMA4_AUDIO_MEL_BINS {
                    features.try_push(0.0)?;
                }
                continue;
            }
            for (frequency, magnitude) in magnitudes.iter_mut().enumerate() {
                let complex_index = frame
                    .checked_mul(GEMMA4_AUDIO_FFT_LENGTH)
                    .and_then(|value| value.checked_add(frequency))
                    .ok_or(MultimodalTextError::Overflow("Gemma4 spectrum index"))?;
                let byte_index = complex_index
                    .checked_mul(8)
                    .ok_or(MultimodalTextError::Overflow("Gemma4 spectrum byte index"))?;
                let bytes = spectrum_bytes.get(byte_index..byte_index + 8).ok_or(
                    MultimodalTextError::InvalidInput("Gemma4 spectrum storage is incomplete"),
                )?;
                let real = f32::from_ne_bytes(bytes[0..4].try_into().map_err(|_| {
                    MultimodalTextError::InvalidInput("Gemma4 spectrum real value is malformed")
                })?);
                let imaginary = f32::from_ne_bytes(bytes[4..8].try_into().map_err(|_| {
                    MultimodalTextError::InvalidInput(
                        "Gemma4 spectrum imaginary value is malformed",
                    )
                })?);
                *magnitude = f64::from(real).hypot(f64::from(imaginary));
            }
            for mel in 0..GEMMA4_AUDIO_MEL_BINS {
                let mut sum = 0.0_f64;
                for (frequency, magnitude) in magnitudes.iter().copied().enumerate() {
                    let filter_index = frequency
                        .checked_mul(GEMMA4_AUDIO_MEL_BINS)
                        .and_then(|value| value.checked_add(mel))
                        .ok_or(MultimodalTextError::Overflow("Gemma4 mel filter index"))?;
                    sum += magnitude
                        * *filterbank.get(filter_index).ok_or(
                            MultimodalTextError::InvalidInput(
                                "Gemma4 mel filterbank is incomplete",
                            ),
                        )?;
                }
                features.try_push((sum + 0.001).ln() as f32)?;
            }
        }
    }
    let log_mel = tensor_from_f32(
        backend,
        &[
            1,
            usize_to_u64(frame_count, "Gemma4 audio frames")?,
            GEMMA4_AUDIO_MEL_BINS as u64,
        ],
        &features,
        context,
    )?;
    let mask_descriptor = TensorDescriptor::contiguous(
        vec![1, usize_to_u64(frame_count, "Gemma4 audio frames")?],
        DType::Bool,
        DeviceId::CPU,
        context.stream,
    )?;
    let (frame_mask, _) = backend.upload_bytes(mask_descriptor, &mask, context)?;
    Ok((log_mel, frame_mask))
}

fn gemma4_mel_filterbank(
    backend: &CpuBackend,
    context: &ExecutionContext<'_>,
) -> Result<CpuWorkspaceVec<f64>, MultimodalTextError> {
    let mut filter_frequencies = [0.0_f64; GEMMA4_AUDIO_MEL_BINS + 2];
    let maximum_mel = 2595.0 * (1.0_f64 + 8000.0 / 700.0).log10();
    for (index, value) in filter_frequencies.iter_mut().enumerate() {
        let mel = maximum_mel * index as f64 / (GEMMA4_AUDIO_MEL_BINS + 1) as f64;
        *value = 700.0 * (10.0_f64.powf(mel / 2595.0) - 1.0);
    }
    let mut filterbank = backend.workspace_vec(
        context,
        257_usize
            .checked_mul(GEMMA4_AUDIO_MEL_BINS)
            .ok_or(MultimodalTextError::Overflow("Gemma4 mel filterbank"))?,
    )?;
    for frequency in 0..257 {
        check_gemma_audio_periodically(frequency, context)?;
        let hertz = 8000.0 * frequency as f64 / 256.0;
        for mel in 0..GEMMA4_AUDIO_MEL_BINS {
            let lower = filter_frequencies[mel];
            let center = filter_frequencies[mel + 1];
            let upper = filter_frequencies[mel + 2];
            let rising = (hertz - lower) / (center - lower);
            let falling = (upper - hertz) / (upper - center);
            filterbank.try_push(rising.min(falling).max(0.0))?;
        }
    }
    Ok(filterbank)
}

fn gemma4_firwin_value(
    index: usize,
    half_length: usize,
    cutoff: f64,
    kaiser_denominator: f64,
) -> f64 {
    let distance = index as f64 - half_length as f64;
    let scaled = cutoff * distance;
    let sinc = if scaled == 0.0 {
        1.0
    } else {
        (std::f64::consts::PI * scaled).sin() / (std::f64::consts::PI * scaled)
    };
    let ratio = distance / half_length as f64;
    let window = gemma4_bessel_i0(GEMMA4_AUDIO_KAISER_BETA * (1.0 - ratio * ratio).max(0.0).sqrt())
        / kaiser_denominator;
    cutoff * sinc * window
}

fn gemma4_bessel_i0(value: f64) -> f64 {
    let argument = value * value / 4.0;
    let mut term = 1.0_f64;
    let mut sum = 1.0_f64;
    for order in 1..=64 {
        term *= argument / ((order * order) as f64);
        sum += term;
        if term <= sum * f64::EPSILON {
            break;
        }
    }
    sum
}

const fn greatest_common_divisor(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn div_ceil_i128(value: i128, divisor: i128) -> i128 {
    let quotient = value.div_euclid(divisor);
    if value.rem_euclid(divisor) == 0 {
        quotient
    } else {
        quotient + 1
    }
}

fn check_gemma_audio_periodically(
    index: usize,
    context: &ExecutionContext<'_>,
) -> Result<(), MultimodalTextError> {
    if index.is_multiple_of(1_024) {
        context.cancellation.check()?;
    }
    Ok(())
}

fn project_rgb_channels(
    backend: &CpuBackend,
    image: &ImageTensor,
    context: &ExecutionContext<'_>,
) -> Result<ImageTensor, MultimodalTextError> {
    let (batch, height, width, channels) = image.dimensions()?;
    if channels == 3 {
        return Ok(image.clone());
    }
    if channels < 3 {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma visual preparation requires at least three channels",
        ));
    }
    let pixels = batch
        .checked_mul(height)
        .and_then(|value| value.checked_mul(width))
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(MultimodalTextError::Overflow("Gemma RGB pixels"))?;
    let channels = u64_to_usize(channels, "Gemma source channels")?;
    let source = image.as_f32_slice()?;
    let mut values = backend.workspace_vec(
        context,
        pixels
            .checked_mul(3)
            .ok_or(MultimodalTextError::Overflow("Gemma RGB values"))?,
    )?;
    for pixel in 0..pixels {
        context.cancellation.check()?;
        let start = pixel
            .checked_mul(channels)
            .ok_or(MultimodalTextError::Overflow("Gemma source pixel"))?;
        for channel in 0..3 {
            values.try_push(*source.get(start + channel).ok_or(
                MultimodalTextError::InvalidInput("Gemma source pixel storage is incomplete"),
            )?)?;
        }
    }
    Ok(ImageTensor::from_f32(
        backend, context, batch, height, width, 3, &values,
    )?)
}

fn gemma4_quantized_bicubic_resize(
    backend: &CpuBackend,
    values: &[f32],
    height: u64,
    width: u64,
    target_height: u64,
    target_width: u64,
    context: &ExecutionContext<'_>,
) -> Result<ImageTensor, MultimodalTextError> {
    let height = u64_to_usize(height, "Gemma4 source height")?;
    let width = u64_to_usize(width, "Gemma4 source width")?;
    let target_height = u64_to_usize(target_height, "Gemma4 target height")?;
    let target_width = u64_to_usize(target_width, "Gemma4 target width")?;
    let source_count = height
        .checked_mul(width)
        .and_then(|value| value.checked_mul(3))
        .ok_or(MultimodalTextError::Overflow("Gemma4 quantized source"))?;
    if values.len() != source_count {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 source frame has the wrong length",
        ));
    }
    let mut nchw = backend.workspace_vec(context, source_count)?;
    for channel in 0..3 {
        for y in 0..height {
            context.cancellation.check()?;
            for x in 0..width {
                let source = (y * width + x) * 3 + channel;
                let value = values
                    .get(source)
                    .copied()
                    .ok_or(MultimodalTextError::InvalidInput(
                        "Gemma4 source frame storage is incomplete",
                    ))?
                    .clamp(0.0, 1.0);
                nchw.try_push(f32::from((value * 255.0).trunc() as u8) / 255.0)?;
            }
        }
    }
    let resized = interpolate_with_context_exact_native(
        &nchw,
        &[1, 3, height, width],
        &InterpolateConfiguration {
            output_size: Some(vec![target_height, target_width]),
            scale_factor: None,
            mode: InterpolateMode::Bicubic,
            align_corners: Some(false),
            recompute_scale_factor: None,
            antialias: true,
        },
        DeviceId::CPU,
        context,
    )?;
    if resized.shape != [1, 3, target_height, target_width] {
        return Err(MultimodalTextError::InvalidInput(
            "Gemma4 bicubic output has the wrong shape",
        ));
    }
    let output_count = target_height
        .checked_mul(target_width)
        .and_then(|value| value.checked_mul(3))
        .ok_or(MultimodalTextError::Overflow("Gemma4 resized output"))?;
    let mut bhwc = backend.workspace_vec(context, output_count)?;
    for y in 0..target_height {
        context.cancellation.check()?;
        for x in 0..target_width {
            for channel in 0..3 {
                let source = (channel * target_height + y) * target_width + x;
                let value = resized
                    .values
                    .get(source)
                    .copied()
                    .ok_or(MultimodalTextError::InvalidInput(
                        "Gemma4 bicubic output storage is incomplete",
                    ))?
                    .clamp(0.0, 1.0);
                bhwc.try_push(f32::from((value * 255.0).round_ties_even() as u8) / 255.0)?;
            }
        }
    }
    Ok(ImageTensor::from_f32(
        backend,
        context,
        1,
        u64::try_from(target_height)
            .map_err(|_| MultimodalTextError::Overflow("Gemma4 target height"))?,
        u64::try_from(target_width)
            .map_err(|_| MultimodalTextError::Overflow("Gemma4 target width"))?,
        3,
        &bhwc,
    )?)
}

pub fn prepare_qwen3vl_images(
    backend: &CpuBackend,
    images: &ImageTensor,
    context: &ExecutionContext<'_>,
) -> Result<Vec<Qwen3VlPreparedImage>, MultimodalTextError> {
    prepare_qwen_images(backend, images, QwenVisionFamily::Qwen3Vl8B, context)
}

pub fn prepare_qwen_images(
    backend: &CpuBackend,
    images: &ImageTensor,
    family: QwenVisionFamily,
    context: &ExecutionContext<'_>,
) -> Result<Vec<Qwen3VlPreparedImage>, MultimodalTextError> {
    context.cancellation.check()?;
    let (batch, height, width, channels) = images.dimensions()?;
    if batch == 0 || channels != 3 {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL images must be a nonempty RGB IMAGE batch",
        ));
    }
    let (target_height, target_width) = qwen3vl_target_dimensions(height, width)?;
    let source = images.as_f32_slice()?;
    let image_elements = height
        .checked_mul(width)
        .and_then(|value| value.checked_mul(channels))
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(MultimodalTextError::Overflow(
            "Qwen3-VL source image elements",
        ))?;
    let batch_size = u64_to_usize(batch, "Qwen3-VL image batch")?;
    let mut prepared = Vec::new();
    prepared
        .try_reserve_exact(batch_size)
        .map_err(|_| MultimodalTextError::Overflow("Qwen3-VL prepared image batch"))?;
    for batch_index in 0..batch_size {
        context.cancellation.check()?;
        let start = batch_index
            .checked_mul(image_elements)
            .ok_or(MultimodalTextError::Overflow("Qwen3-VL image offset"))?;
        let end = start
            .checked_add(image_elements)
            .ok_or(MultimodalTextError::Overflow("Qwen3-VL image end"))?;
        let singleton = ImageTensor::from_f32(
            backend,
            context,
            1,
            height,
            width,
            channels,
            source
                .get(start..end)
                .ok_or(MultimodalTextError::InvalidInput(
                    "Qwen3-VL image batch storage is incomplete",
                ))?,
        )?;
        let resized = singleton.resize(
            target_width,
            target_height,
            ResizeMode::Bilinear,
            ResizeCrop::Disabled,
            backend,
            context,
        )?;
        prepared.push(prepare_qwen3vl_resized_image(
            backend,
            &resized,
            target_height,
            target_width,
            family,
            context,
        )?);
    }
    context.cancellation.check()?;
    Ok(prepared)
}

pub fn plan_qwen3vl_markers(
    tokens: &[i64],
    images: &[Qwen3VlPreparedImage],
    cancellation: &comfy_types::CancellationToken,
) -> Result<Qwen3VlMarkerPlan, MultimodalTextError> {
    plan_qwen_markers(tokens, images, QwenVisionFamily::Qwen3Vl8B, cancellation)
}

pub fn plan_qwen_markers(
    tokens: &[i64],
    images: &[Qwen3VlPreparedImage],
    family: QwenVisionFamily,
    cancellation: &comfy_types::CancellationToken,
) -> Result<Qwen3VlMarkerPlan, MultimodalTextError> {
    cancellation.check()?;
    if images.iter().any(|image| image.family != family) {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen prepared image family does not match the marker plan",
        ));
    }
    let image_pad_token = family.image_pad_token();
    let marker_count = tokens
        .iter()
        .filter(|token| **token == image_pad_token)
        .count();
    if marker_count != images.len() {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL image markers and prepared images must match exactly",
        ));
    }
    let mut expanded_length = 0_usize;
    let mut length_image_index = 0_usize;
    for token in tokens {
        let token_length = if *token == image_pad_token {
            let image = images
                .get(length_image_index)
                .ok_or(MultimodalTextError::InvalidInput(
                    "Qwen3-VL image marker has no prepared image",
                ))?;
            length_image_index = length_image_index
                .checked_add(1)
                .ok_or(MultimodalTextError::Overflow("Qwen3-VL image index"))?;
            image.merged_tokens
        } else {
            1
        };
        expanded_length = expanded_length
            .checked_add(token_length)
            .ok_or(MultimodalTextError::Overflow("Qwen3-VL expanded tokens"))?;
    }
    let mut expanded_tokens = Vec::new();
    expanded_tokens
        .try_reserve_exact(expanded_length)
        .map_err(|_| MultimodalTextError::Overflow("Qwen3-VL expanded tokens"))?;
    let mut spans = Vec::new();
    spans
        .try_reserve_exact(images.len())
        .map_err(|_| MultimodalTextError::Overflow("Qwen3-VL image spans"))?;
    let mut image_index = 0_usize;
    for (token_index, token) in tokens.iter().copied().enumerate() {
        if token_index.is_multiple_of(256) {
            cancellation.check()?;
        }
        if token != image_pad_token {
            expanded_tokens.push(token);
            continue;
        }
        let image = images
            .get(image_index)
            .ok_or(MultimodalTextError::InvalidInput(
                "Qwen3-VL image marker has no prepared image",
            ))?;
        let start = expanded_tokens.len();
        expanded_tokens.extend(std::iter::repeat_n(token, image.merged_tokens));
        spans.push(MultimodalSpan {
            start,
            size: image.merged_tokens,
            grid_thw: image.grid_thw,
        });
        image_index = image_index
            .checked_add(1)
            .ok_or(MultimodalTextError::Overflow("Qwen3-VL image index"))?;
    }
    if expanded_tokens.is_empty() {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL expanded prompt cannot be empty",
        ));
    }
    let mut visual_position_mask = Vec::new();
    visual_position_mask
        .try_reserve_exact(expanded_tokens.len())
        .map_err(|_| MultimodalTextError::Overflow("Qwen3-VL visual position mask"))?;
    visual_position_mask.resize(expanded_tokens.len(), false);
    for span in &spans {
        let end = span
            .start
            .checked_add(span.size)
            .ok_or(MultimodalTextError::Overflow("Qwen3-VL visual mask"))?;
        visual_position_mask
            .get_mut(span.start..end)
            .ok_or(MultimodalTextError::InvalidInput(
                "Qwen3-VL visual span is outside the expanded prompt",
            ))?
            .fill(true);
    }
    cancellation.check()?;
    Ok(Qwen3VlMarkerPlan {
        expanded_tokens,
        spans,
        visual_position_mask,
    })
}

impl NativeQwenVisionEncoder {
    pub fn new(
        configuration: QwenVisionConfiguration,
        weights: QwenVisionWeights,
    ) -> Result<Self, MultimodalTextError> {
        configuration.validate()?;
        if weights.blocks.len() != configuration.layer_count
            || weights.deepstack_mergers.len() != configuration.family.deepstack_layers().len()
        {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen vision block or deepstack weight count does not match the profile",
            ));
        }
        let stream = weights.patch_weight.descriptor().stream();
        let patch_width = 3
            * QWEN3VL_IMAGE_TEMPORAL_PATCH_SIZE
            * QWEN3VL_IMAGE_PATCH_SIZE
            * QWEN3VL_IMAGE_PATCH_SIZE;
        let patch_projection = qwen_linear_module(
            "qwen_vision.patch_projection",
            patch_width,
            configuration.hidden_size,
            weights.patch_weight,
            Some(weights.patch_bias),
            stream,
        )?;
        qwen_require_parameter_shape(
            &weights.position_embedding,
            &[configuration.position_embeddings, configuration.hidden_size],
            stream,
        )?;
        let mut blocks = Vec::new();
        blocks
            .try_reserve_exact(configuration.layer_count)
            .map_err(|_| MultimodalTextError::Overflow("Qwen vision blocks"))?;
        for (index, weights) in weights.blocks.into_iter().enumerate() {
            blocks.push(qwen_vision_block(index, &configuration, weights, stream)?);
        }
        let merger = qwen_vision_merger(
            "qwen_vision.merger",
            &configuration,
            weights.merger,
            false,
            stream,
        )?;
        let mut deepstack_mergers = Vec::new();
        deepstack_mergers
            .try_reserve_exact(weights.deepstack_mergers.len())
            .map_err(|_| MultimodalTextError::Overflow("Qwen deepstack mergers"))?;
        for (index, weights) in weights.deepstack_mergers.into_iter().enumerate() {
            deepstack_mergers.push(qwen_vision_merger(
                &format!("qwen_vision.deepstack.{index}"),
                &configuration,
                weights,
                true,
                stream,
            )?);
        }
        Ok(Self {
            configuration,
            patch_projection,
            position_embedding: weights.position_embedding,
            blocks,
            merger,
            deepstack_mergers,
        })
    }

    pub fn configuration(&self) -> &QwenVisionConfiguration {
        &self.configuration
    }

    pub fn execution_stream(&self) -> StreamId {
        self.position_embedding.descriptor().stream()
    }

    pub fn semantic_state_digest(
        &self,
        cancellation: &comfy_types::CancellationToken,
    ) -> Result<String, MultimodalTextError> {
        cancellation.check()?;
        let mut hasher = Sha256::new();
        hasher.update(b"sim.comfy.qwen-vision.v1");
        hasher.update(format!("{:?}", self.configuration).as_bytes());
        hasher.update(self.position_embedding.contiguous_bytes()?);
        for (name, module) in self.named_modules() {
            cancellation.check()?;
            hasher.update([0]);
            hasher.update(name.as_bytes());
            hasher.update(module.semantic_state_digest(cancellation)?.as_bytes());
        }
        cancellation.check()?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn resident_tensor_allocations(&self) -> Vec<(comfy_tensor::StorageId, u64)> {
        let mut allocations = vec![(
            self.position_embedding.storage_id(),
            self.position_embedding.storage_byte_len(),
        )];
        for (_, module) in self.named_modules() {
            for (storage_id, bytes) in module.resident_tensor_allocations() {
                if !allocations
                    .iter()
                    .any(|(existing, _)| *existing == storage_id)
                {
                    allocations.push((storage_id, bytes));
                }
            }
        }
        allocations
    }

    pub fn resident_bytes(&self) -> Result<u64, MultimodalTextError> {
        self.resident_tensor_allocations().into_iter().try_fold(
            self.resident_owned_bytes()?,
            |total, (_, bytes)| {
                total
                    .checked_add(bytes)
                    .ok_or(MultimodalTextError::Overflow("Qwen vision residency"))
            },
        )
    }

    pub fn resident_owned_bytes(&self) -> Result<u64, MultimodalTextError> {
        let block_bytes = self
            .blocks
            .capacity()
            .checked_mul(mem::size_of::<NativeQwenVisionBlock>())
            .ok_or(MultimodalTextError::Overflow("Qwen vision block residency"))?;
        let merger_bytes = self
            .deepstack_mergers
            .capacity()
            .checked_mul(mem::size_of::<NativeQwenVisionMerger>())
            .ok_or(MultimodalTextError::Overflow(
                "Qwen deepstack merger residency",
            ))?;
        let mut bytes = u64::try_from(mem::size_of::<Self>())
            .ok()
            .and_then(|bytes| {
                u64::try_from(block_bytes)
                    .ok()
                    .and_then(|part| bytes.checked_add(part))
            })
            .and_then(|bytes| {
                u64::try_from(merger_bytes)
                    .ok()
                    .and_then(|part| bytes.checked_add(part))
            })
            .ok_or(MultimodalTextError::Overflow("Qwen vision owned residency"))?;
        for (_, module) in self.named_modules() {
            let tensor_bytes = module.resident_tensor_allocations().into_iter().try_fold(
                0_u64,
                |total, (_, allocation)| {
                    total
                        .checked_add(allocation)
                        .ok_or(MultimodalTextError::Overflow(
                            "Qwen vision tensor residency",
                        ))
                },
            )?;
            let module_bytes = module.resident_storage_bytes()?;
            bytes = bytes
                .checked_add(module_bytes.checked_sub(tensor_bytes).ok_or(
                    MultimodalTextError::Overflow("Qwen vision module residency projection"),
                )?)
                .ok_or(MultimodalTextError::Overflow(
                    "Qwen vision module residency",
                ))?;
        }
        Ok(bytes)
    }

    pub fn project(
        &self,
        backend: &CpuBackend,
        prepared: &Qwen3VlPreparedImage,
        context: &ExecutionContext<'_>,
    ) -> Result<QwenVisionProjection, MultimodalTextError> {
        context.cancellation.check()?;
        if prepared.family != self.configuration.family {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen prepared image family does not match the retained vision encoder",
            ));
        }
        let patch_shape = prepared.patches.descriptor().shape();
        let patch_count = patch_shape
            .first()
            .copied()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(MultimodalTextError::InvalidInput(
                "Qwen prepared patch count is invalid",
            ))?;
        if patch_shape != [usize_to_u64(patch_count, "Qwen patch count")?, 3, 2, 16, 16] {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen prepared patches must use the exact flattened Conv3d geometry",
            ));
        }
        let flattened = qwen_tensor(
            backend,
            &[patch_count, 3 * 2 * 16 * 16],
            &tensor_to_f32(backend, &prepared.patches, context)?,
            context,
        )?;
        let mut patch_projection = self.patch_projection.clone();
        let hidden = patch_projection.forward_with_context(backend, &flattened, context)?;
        let mut hidden_values = tensor_to_f32(backend, &hidden, context)?
            .iter()
            .copied()
            .collect::<Vec<_>>();
        qwen_add_interpolated_positions(
            &mut hidden_values,
            prepared.grid_thw,
            &tensor_to_f32(backend, &self.position_embedding, context)?,
            self.configuration.hidden_size,
            self.configuration.spatial_merge_size,
            context.cancellation,
        )?;
        let mut hidden = qwen_tensor(
            backend,
            &[patch_count, self.configuration.hidden_size],
            &hidden_values,
            context,
        )?;
        let mut captured = Vec::new();
        captured
            .try_reserve_exact(self.deepstack_mergers.len())
            .map_err(|_| MultimodalTextError::Overflow("Qwen deepstack captures"))?;
        for (index, block) in self.blocks.iter().enumerate() {
            context.cancellation.check()?;
            hidden = block.forward(
                backend,
                &hidden,
                prepared.grid_thw,
                &self.configuration,
                context,
            )?;
            if let Some(capture_index) = self
                .configuration
                .family
                .deepstack_layers()
                .iter()
                .position(|layer| *layer == index)
            {
                let merger = self.deepstack_mergers.get(capture_index).ok_or(
                    MultimodalTextError::InvalidInput("Qwen deepstack merger is missing"),
                )?;
                captured.push(merger.forward(backend, &hidden, &self.configuration, context)?);
            }
        }
        let embedding = self
            .merger
            .forward(backend, &hidden, &self.configuration, context)?;
        if embedding.descriptor().shape()
            != [
                usize_to_u64(prepared.merged_tokens, "Qwen merged tokens")?,
                usize_to_u64(self.configuration.output_hidden_size, "Qwen output hidden")?,
            ]
        {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen merged vision output does not match the prepared span",
            ));
        }
        context.cancellation.check()?;
        Ok(QwenVisionProjection {
            embedding,
            deepstack: captured,
            grid_thw: prepared.grid_thw,
        })
    }

    fn named_modules(&self) -> Vec<(String, &NativeModule)> {
        let mut modules = vec![("patch_projection".to_owned(), &self.patch_projection)];
        for (index, block) in self.blocks.iter().enumerate() {
            for (name, module) in block.modules() {
                modules.push((format!("blocks.{index}.{name}"), module));
            }
        }
        for (child, module) in self.merger.modules() {
            modules.push((format!("merger.{child}"), module));
        }
        for (index, merger) in self.deepstack_mergers.iter().enumerate() {
            for (child, module) in merger.modules() {
                modules.push((format!("deepstack.{index}.{child}"), module));
            }
        }
        modules
    }
}

#[derive(Clone)]
pub struct NativeQwenMultimodal {
    tokenizer: Arc<NativePromptTokenizer>,
    decoder: Arc<NativeDecoderTextEncoder>,
    vision: Arc<NativeQwenVisionEncoder>,
}

pub fn qwen_multimodal_tokenizer_profile(family: QwenVisionFamily) -> Qwen2PretokenizerProfile {
    match family {
        QwenVisionFamily::Qwen3Vl4B | QwenVisionFamily::Qwen3Vl8B => {
            Qwen2PretokenizerProfile::Qwen2
        }
        QwenVisionFamily::Qwen35_08B
        | QwenVisionFamily::Qwen35_2B
        | QwenVisionFamily::Qwen35_4B
        | QwenVisionFamily::Qwen35_9B
        | QwenVisionFamily::Qwen35_27B => Qwen2PretokenizerProfile::Qwen35Declared,
    }
}

pub fn qwen_multimodal_decoder_configuration(
    family: QwenVisionFamily,
) -> Result<DecoderTextConfiguration, MultimodalTextError> {
    let profile = decoder_profile_fact(qwen_decoder_profile_name(family)).ok_or(
        MultimodalTextError::InvalidInput("Qwen decoder source profile is missing"),
    )?;
    let layer_kinds = if profile.architecture == DecoderArchitecture::Qwen35 {
        let period = profile
            .linear_attention_period
            .ok_or(MultimodalTextError::InvalidInput(
                "Qwen3.5 linear-attention period is missing",
            ))?;
        (0..profile.hidden_layers)
            .map(|index| {
                if (index + 1).is_multiple_of(period) {
                    DecoderLayerKind::FullAttention
                } else {
                    DecoderLayerKind::LinearAttention
                }
            })
            .collect()
    } else {
        vec![DecoderLayerKind::FullAttention; profile.hidden_layers]
    };
    let qwen35_linear = match (
        profile.linear_key_heads,
        profile.linear_value_heads,
        profile.linear_key_head_dimension,
        profile.linear_value_head_dimension,
        profile.convolution_kernel_size,
    ) {
        (
            Some(key_heads),
            Some(value_heads),
            Some(key_head_dimension),
            Some(value_head_dimension),
            Some(convolution_kernel_size),
        ) => Some(crate::Qwen35LinearConfiguration {
            key_heads,
            value_heads,
            key_head_dimension,
            value_head_dimension,
            convolution_kernel_size,
        }),
        (None, None, None, None, None) => None,
        _ => {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen decoder linear-attention profile is incomplete",
            ));
        }
    };
    let theta = profile
        .rope_theta()
        .next()
        .ok_or(MultimodalTextError::InvalidInput(
            "Qwen decoder RoPE theta is missing",
        ))?;
    let rotary_dimension = if profile.architecture == DecoderArchitecture::Qwen35 {
        ((profile.head_dimension as f32) * profile.partial_rotary_factor()).round() as usize
    } else {
        profile.head_dimension
    };
    let configuration = DecoderTextConfiguration {
        architecture: profile.architecture,
        dtype: DType::F32,
        device: DeviceId::CPU,
        vocabulary_size: profile.vocabulary_size,
        maximum_tokens: profile.maximum_positions,
        hidden_size: profile.hidden_size,
        feed_forward_size: profile.intermediate_size,
        layer_kinds,
        attention_heads: profile.attention_heads,
        key_value_heads: profile.key_value_heads,
        head_dimension: profile.head_dimension,
        query_key_norm: profile.query_key_norm,
        qwen35_linear,
        gemma3: None,
        gemma4: None,
        normalization_epsilon_bits: profile.normalization_epsilon_bits,
        rope: crate::DecoderRopeConfiguration {
            theta,
            rotary_dimension,
            interleaved_sections: profile.rope_sections.to_vec(),
            scaling: crate::RopeScaling::None,
        },
        sliding_window: None,
        activation: profile.activation,
        embedding_scale_bits: 1.0_f32.to_bits(),
        residual_scale_bits: 1.0_f32.to_bits(),
        norm_weight_offset_bits: if profile.rms_norm_add {
            1.0_f32.to_bits()
        } else {
            0.0_f32.to_bits()
        },
        logits_soft_cap_bits: profile.final_logit_soft_cap_bits,
        tied_output_head: !profile.untied_output_head,
        stop_tokens: profile.stop_tokens.to_vec(),
    };
    configuration.validate()?;
    Ok(configuration)
}

impl NativeQwenMultimodal {
    pub fn new(
        tokenizer: Arc<NativePromptTokenizer>,
        decoder: Arc<NativeDecoderTextEncoder>,
        vision: Arc<NativeQwenVisionEncoder>,
        cancellation: &comfy_types::CancellationToken,
    ) -> Result<Self, MultimodalTextError> {
        if !vision.configuration().source_exact_profile {
            return Err(MultimodalTextError::InvalidInput(
                "production Qwen multimodal resources require closed source-exact profiles",
            ));
        }
        if vision.configuration() != &QwenVisionConfiguration::source(vision.configuration().family)
        {
            return Err(MultimodalTextError::InvalidInput(
                "production Qwen multimodal vision configuration is not the closed source profile",
            ));
        }
        Self::checked(tokenizer, decoder, vision, cancellation)
    }

    #[doc(hidden)]
    pub fn reduced_fixture(
        tokenizer: Arc<NativePromptTokenizer>,
        decoder: Arc<NativeDecoderTextEncoder>,
        vision: Arc<NativeQwenVisionEncoder>,
        cancellation: &comfy_types::CancellationToken,
    ) -> Result<Self, MultimodalTextError> {
        if vision.configuration().source_exact_profile {
            return Err(MultimodalTextError::InvalidInput(
                "reduced Qwen fixture resources require a reduced vision profile",
            ));
        }
        Self::checked(tokenizer, decoder, vision, cancellation)
    }

    fn checked(
        tokenizer: Arc<NativePromptTokenizer>,
        decoder: Arc<NativeDecoderTextEncoder>,
        vision: Arc<NativeQwenVisionEncoder>,
        cancellation: &comfy_types::CancellationToken,
    ) -> Result<Self, MultimodalTextError> {
        let resource = Self {
            tokenizer,
            decoder,
            vision,
        };
        resource.validate(cancellation)?;
        Ok(resource)
    }

    pub fn family(&self) -> QwenVisionFamily {
        self.vision.configuration().family
    }

    pub fn tokenizer(&self) -> &Arc<NativePromptTokenizer> {
        &self.tokenizer
    }

    pub fn decoder(&self) -> &Arc<NativeDecoderTextEncoder> {
        &self.decoder
    }

    pub fn vision(&self) -> &Arc<NativeQwenVisionEncoder> {
        &self.vision
    }

    pub fn is_source_exact_profile(&self) -> bool {
        self.vision.configuration().source_exact_profile
    }

    pub fn validate(
        &self,
        cancellation: &comfy_types::CancellationToken,
    ) -> Result<(), MultimodalTextError> {
        cancellation.check()?;
        let family = self.family();
        let expected_tokenizer_profile = qwen_multimodal_tokenizer_profile(family);
        let expected_tokenizer_digest = match expected_tokenizer_profile {
            Qwen2PretokenizerProfile::Qwen2 => QWEN25_TOKENIZER_ARTIFACT_DIGEST,
            Qwen2PretokenizerProfile::Qwen35Declared => QWEN35_TOKENIZER_ARTIFACT_DIGEST,
        };
        if self.tokenizer.qwen2_profile() != Some(expected_tokenizer_profile)
            || self.tokenizer.qwen2_artifact_digest() != Some(expected_tokenizer_digest)
            || self.tokenizer.has_textual_inversion_embeddings()
        {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen multimodal resources require the exact canonical Qwen2 tokenizer family",
            ));
        }
        let tokenizer_configuration = self.tokenizer.configuration();
        let expected_pad = match expected_tokenizer_profile {
            Qwen2PretokenizerProfile::Qwen2 => 151_643,
            Qwen2PretokenizerProfile::Qwen35Declared => 248_044,
        };
        if tokenizer_configuration.pad_token != expected_pad
            || tokenizer_configuration.minimum_length != Some(1)
            || tokenizer_configuration.minimum_padding.is_some()
            || tokenizer_configuration.pad_to_maximum_length
            || tokenizer_configuration.pad_left
            || tokenizer_configuration.start_token.is_some()
            || tokenizer_configuration.end_token.is_some()
            || !tokenizer_configuration.disable_weights
        {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen multimodal tokenizer configuration is not source-compatible",
            ));
        }
        let marker = u32::try_from(family.image_pad_token())
            .map_err(|_| MultimodalTextError::Overflow("Qwen image marker"))?;
        if self
            .tokenizer
            .encode_numeric("<|image_pad|>", cancellation)?
            != [marker]
            || self.tokenizer.encode_numeric("", cancellation)? != [expected_pad]
        {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen multimodal tokenizer control-token projection changed",
            ));
        }
        let decoder_configuration = self.decoder.configuration();
        if decoder_configuration.hidden_size != self.vision.configuration().output_hidden_size
            || tokenizer_configuration.maximum_length < decoder_configuration.maximum_tokens
            || tokenizer_configuration.maximum_word_length != 8
            || tokenizer_configuration.embedding_width.is_some()
            || self.decoder.execution_stream() != self.vision.execution_stream()
        {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen tokenizer, decoder, and vision sequence or hidden dimensions differ",
            ));
        }
        if self.vision.configuration().source_exact_profile {
            if decoder_configuration != &qwen_multimodal_decoder_configuration(family)? {
                return Err(MultimodalTextError::InvalidInput(
                    "Qwen decoder configuration does not match the closed source profile",
                ));
            }
        } else if !qwen_reduced_decoder_is_compatible(decoder_configuration, family) {
            return Err(MultimodalTextError::InvalidInput(
                "reduced Qwen decoder fixture is not structurally compatible with its family",
            ));
        }
        cancellation.check()?;
        Ok(())
    }

    pub fn semantic_state_digest(
        &self,
        cancellation: &comfy_types::CancellationToken,
    ) -> Result<String, MultimodalTextError> {
        self.validate(cancellation)?;
        let mut hasher = Sha256::new();
        hasher.update(b"sim.comfy.qwen-multimodal-resource.v2");
        hasher.update(b"standard-comfy-text-generation-adapter");
        hasher.update(format!("{:?}", self.family()).as_bytes());
        hasher.update(QWEN_MULTIMODAL_ROUTING_SOURCE_SHA256.as_bytes());
        hasher.update(crate::LLAMA_SOURCE_SHA256.as_bytes());
        hasher.update(QWEN35_SOURCE_SHA256.as_bytes());
        hasher.update(
            match self.family() {
                QwenVisionFamily::Qwen3Vl4B | QwenVisionFamily::Qwen3Vl8B => QWEN3VL_SOURCE_SHA256,
                QwenVisionFamily::Qwen35_08B
                | QwenVisionFamily::Qwen35_2B
                | QwenVisionFamily::Qwen35_4B
                | QwenVisionFamily::Qwen35_9B
                | QwenVisionFamily::Qwen35_27B => QWEN35_SOURCE_SHA256,
            }
            .as_bytes(),
        );
        hasher.update(QWEN_VL_SOURCE_SHA256.as_bytes());
        hasher.update(SD1_CLIP_SOURCE_SHA256.as_bytes());
        hasher.update(
            self.tokenizer
                .qwen2_artifact_digest()
                .ok_or(MultimodalTextError::InvalidInput(
                    "Qwen tokenizer artifact identity is unavailable",
                ))?
                .as_bytes(),
        );
        hasher.update(self.tokenizer.semantic_digest(cancellation)?.as_bytes());
        hasher.update(self.decoder.semantic_state_digest(cancellation)?.as_bytes());
        hasher.update(self.vision.semantic_state_digest(cancellation)?.as_bytes());
        cancellation.check()?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn resident_owned_bytes(&self) -> Result<u64, MultimodalTextError> {
        u64::try_from(mem::size_of::<Self>())
            .map_err(|_| MultimodalTextError::Overflow("Qwen multimodal owner residency"))
    }

    pub fn resident_tensor_allocations(
        &self,
    ) -> Result<Vec<(comfy_tensor::StorageId, u64)>, MultimodalTextError> {
        let mut allocations = self.decoder.resident_tensor_allocations();
        for (storage_id, resident_bytes) in self.vision.resident_tensor_allocations() {
            if let Some((_, existing_bytes)) = allocations
                .iter()
                .find(|(existing, _)| *existing == storage_id)
            {
                if *existing_bytes != resident_bytes {
                    return Err(MultimodalTextError::InvalidInput(
                        "shared Qwen tensor storage changed resident size",
                    ));
                }
            } else {
                allocations.push((storage_id, resident_bytes));
            }
        }
        Ok(allocations)
    }

    pub fn resident_bytes(&self) -> Result<u64, MultimodalTextError> {
        let mut bytes = self.resident_owned_bytes()?;
        bytes = bytes.checked_add(self.tokenizer.resident_bytes()?).ok_or(
            MultimodalTextError::Overflow("Qwen multimodal backing residency"),
        )?;
        bytes = bytes
            .checked_add(self.decoder.resident_owned_bytes()?)
            .ok_or(MultimodalTextError::Overflow(
                "Qwen multimodal backing residency",
            ))?;
        bytes = bytes
            .checked_add(self.vision.resident_owned_bytes()?)
            .ok_or(MultimodalTextError::Overflow(
                "Qwen multimodal backing residency",
            ))?;
        self.resident_tensor_allocations()?.into_iter().try_fold(
            bytes,
            |total, (_, resident_bytes)| {
                total
                    .checked_add(resident_bytes)
                    .ok_or(MultimodalTextError::Overflow(
                        "Qwen multimodal tensor residency",
                    ))
            },
        )
    }

    pub fn generate(
        &self,
        backend: &CpuBackend,
        request: QwenMultimodalGenerationRequest<'_>,
        context: &ExecutionContext<'_>,
    ) -> Result<NativeTextGenerationResult, MultimodalTextError> {
        context.cancellation.check()?;
        self.validate(context.cancellation)?;
        let configuration = self.decoder.generation_configuration(&request.text)?;
        let prompt_tokens = self
            .tokenizer
            .encode_numeric(request.text.formatted_prompt, context.cancellation)?;
        let prompt_tokens = prompt_tokens.into_iter().map(i64::from).collect::<Vec<_>>();
        let marker_plan = plan_qwen_markers(
            &prompt_tokens,
            request.prepared_images,
            self.family(),
            context.cancellation,
        )?;
        if marker_plan
            .expanded_tokens()
            .len()
            .checked_add(configuration.maximum_new_tokens)
            .ok_or(MultimodalTextError::Overflow(
                "Qwen prompt and generation length",
            ))?
            > self.decoder.configuration().maximum_tokens
        {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen prompt and generation exceed the decoder token limit",
            ));
        }
        let text_embeddings =
            self.decoder
                .embed_token_values(backend, marker_plan.expanded_tokens(), context)?;
        let mut projections = Vec::new();
        projections
            .try_reserve_exact(request.prepared_images.len())
            .map_err(|_| MultimodalTextError::Overflow("Qwen vision projections"))?;
        for prepared in request.prepared_images {
            context.cancellation.check()?;
            projections.push(self.vision.project(backend, prepared, context)?);
        }
        let mut image_embeddings = Vec::new();
        image_embeddings
            .try_reserve_exact(projections.len())
            .map_err(|_| MultimodalTextError::Overflow("Qwen image embeddings"))?;
        for (span, projection) in marker_plan.spans().iter().zip(&projections) {
            if projection.grid_thw != span.grid_thw {
                return Err(MultimodalTextError::InvalidInput(
                    "Qwen projected image grid does not match its marker span",
                ));
            }
            image_embeddings.push(MultimodalImageEmbedding {
                span: *span,
                embedding: &projection.embedding,
                deepstack: &projection.deepstack,
            });
        }
        if image_embeddings.len() != marker_plan.spans().len() {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen projected images do not cover every marker span",
            ));
        }
        let embeddings =
            join_multimodal_embeddings(backend, &text_embeddings, &image_embeddings, context)?;
        let sequence_length = marker_plan.expanded_tokens().len();
        let causal_positions = qwen_causal_positions(sequence_length)?;
        let multidimensional_positions = if matches!(
            self.family(),
            QwenVisionFamily::Qwen3Vl4B | QwenVisionFamily::Qwen3Vl8B
        ) {
            qwen2vl_mrope_position_ids(sequence_length, marker_plan.spans(), context.cancellation)?
                .map(|positions| qwen_decoder_position_axes(&positions, context.cancellation))
                .transpose()?
        } else {
            None
        };
        let rope_positions = multidimensional_positions.as_ref().map_or(
            DecoderRopePositions::Scalar(&causal_positions),
            |positions| DecoderRopePositions::Multidimensional(positions),
        );
        let deepstack_join = if matches!(
            self.family(),
            QwenVisionFamily::Qwen3Vl4B | QwenVisionFamily::Qwen3Vl8B
        ) && !image_embeddings.is_empty()
        {
            join_qwen3vl_deepstack(backend, sequence_length, &image_embeddings, context)?
        } else {
            if image_embeddings
                .iter()
                .any(|image| !image.deepstack.is_empty())
            {
                return Err(MultimodalTextError::InvalidInput(
                    "Qwen3.5 generation cannot admit deepstack inputs",
                ));
            }
            None
        };
        if deepstack_join
            .as_ref()
            .is_some_and(|joined| joined.visual_position_mask != marker_plan.visual_position_mask())
        {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen deepstack mask does not match the marker plan",
            ));
        }
        let deepstack = deepstack_join
            .as_ref()
            .map(|joined| DecoderPreparedDeepstack {
                visual_position_mask: &joined.visual_position_mask,
                layers: &joined.layers,
            });
        let outcome = self.decoder.generate_prepared(
            backend,
            DecoderPreparedGenerationPrompt {
                embeddings: &embeddings,
                sampling_history: &[],
                attention_mask: None,
                rope_positions,
                causal_positions: &causal_positions,
                deepstack,
                initial_input_ids: None,
            },
            &configuration,
            request.transaction,
            context,
        )?;
        context.cancellation.check()?;
        Ok(self
            .decoder
            .finish_prepared_generation(&self.tokenizer, outcome, context)?)
    }
}

fn qwen_causal_positions(sequence_length: usize) -> Result<Vec<usize>, MultimodalTextError> {
    let mut positions = Vec::new();
    positions
        .try_reserve_exact(sequence_length)
        .map_err(|_| MultimodalTextError::Overflow("Qwen causal positions"))?;
    positions.extend(0..sequence_length);
    Ok(positions)
}

fn qwen_decoder_position_axes(
    positions: &MultimodalPositionIds,
    cancellation: &comfy_types::CancellationToken,
) -> Result<Vec<Vec<usize>>, MultimodalTextError> {
    let mut axes = Vec::new();
    axes.try_reserve_exact(3)
        .map_err(|_| MultimodalTextError::Overflow("Qwen position axes"))?;
    for source in [positions.temporal(), positions.height(), positions.width()] {
        cancellation.check()?;
        let mut axis = Vec::new();
        axis.try_reserve_exact(source.len())
            .map_err(|_| MultimodalTextError::Overflow("Qwen position axis"))?;
        for value in source {
            axis.push(usize::try_from(*value).map_err(|_| {
                MultimodalTextError::InvalidInput("Qwen position IDs must be nonnegative")
            })?);
        }
        axes.push(axis);
    }
    Ok(axes)
}

fn qwen_decoder_profile_name(family: QwenVisionFamily) -> &'static str {
    match family {
        QwenVisionFamily::Qwen3Vl4B => "Qwen3VL_4BConfig",
        QwenVisionFamily::Qwen3Vl8B => "Qwen3VL_8BConfig",
        QwenVisionFamily::Qwen35_08B => "qwen35_08b",
        QwenVisionFamily::Qwen35_2B => "qwen35_2b",
        QwenVisionFamily::Qwen35_4B => "qwen35_4b",
        QwenVisionFamily::Qwen35_9B => "qwen35_9b",
        QwenVisionFamily::Qwen35_27B => "qwen35_27b",
    }
}

fn qwen_reduced_decoder_is_compatible(
    configuration: &DecoderTextConfiguration,
    family: QwenVisionFamily,
) -> bool {
    let is_qwen35 = matches!(
        family,
        QwenVisionFamily::Qwen35_08B
            | QwenVisionFamily::Qwen35_2B
            | QwenVisionFamily::Qwen35_4B
            | QwenVisionFamily::Qwen35_9B
            | QwenVisionFamily::Qwen35_27B
    );
    if is_qwen35 {
        configuration.architecture == DecoderArchitecture::Qwen35
            && configuration.query_key_norm
            && configuration.qwen35_linear.is_some()
            && configuration
                .layer_kinds
                .contains(&DecoderLayerKind::LinearAttention)
    } else {
        configuration.architecture == DecoderArchitecture::Llama
            && configuration.query_key_norm
            && configuration.qwen35_linear.is_none()
            && configuration
                .layer_kinds
                .iter()
                .all(|kind| *kind == DecoderLayerKind::FullAttention)
            && configuration.layer_kinds.len() >= family.deepstack_layers().len()
    }
}

impl NativeQwenVisionBlock {
    fn modules(&self) -> [(&'static str, &NativeModule); 7] {
        [
            ("normalization_one", &self.normalization_one),
            ("query_key_value", &self.query_key_value),
            ("attention_output", &self.attention_output),
            ("normalization_two", &self.normalization_two),
            ("feed_forward_up", &self.feed_forward_up),
            ("feed_forward_activation", &self.feed_forward_activation),
            ("feed_forward_down", &self.feed_forward_down),
        ]
    }

    fn forward(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        grid_thw: [usize; 3],
        configuration: &QwenVisionConfiguration,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, MultimodalTextError> {
        let mut normalization_one = self.normalization_one.clone();
        let normalized = normalization_one.forward_with_context(backend, input, context)?;
        let mut query_key_value = self.query_key_value.clone();
        let projected = query_key_value.forward_with_context(backend, &normalized, context)?;
        let projected = tensor_to_f32(backend, &projected, context)?;
        let attention = qwen_vision_attention(
            backend,
            &projected,
            grid_thw,
            configuration.hidden_size,
            configuration.attention_heads,
            context,
        )?;
        let attention = qwen_tensor(
            backend,
            &[
                grid_thw[0] * grid_thw[1] * grid_thw[2],
                configuration.hidden_size,
            ],
            &attention,
            context,
        )?;
        let mut attention_output = self.attention_output.clone();
        let attention = attention_output.forward_with_context(backend, &attention, context)?;
        let residual = qwen_add_tensors(backend, input, &attention, context)?;
        let mut normalization_two = self.normalization_two.clone();
        let normalized = normalization_two.forward_with_context(backend, &residual, context)?;
        let mut feed_forward_up = self.feed_forward_up.clone();
        let up = feed_forward_up.forward_with_context(backend, &normalized, context)?;
        let mut activation = self.feed_forward_activation.clone();
        let up = activation.forward_with_context(backend, &up, context)?;
        let mut feed_forward_down = self.feed_forward_down.clone();
        let down = feed_forward_down.forward_with_context(backend, &up, context)?;
        qwen_add_tensors(backend, &residual, &down, context)
    }
}

impl NativeQwenVisionMerger {
    fn modules(&self) -> [(&'static str, &NativeModule); 4] {
        [
            ("normalization", &self.normalization),
            ("first", &self.first),
            ("activation", &self.activation),
            ("second", &self.second),
        ]
    }

    fn forward(
        &self,
        backend: &CpuBackend,
        input: &Tensor,
        configuration: &QwenVisionConfiguration,
        context: &ExecutionContext<'_>,
    ) -> Result<Tensor, MultimodalTextError> {
        let shape = input.descriptor().shape();
        let tokens = shape
            .first()
            .copied()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(MultimodalTextError::InvalidInput(
                "Qwen merger input token count is invalid",
            ))?;
        let merge_unit = configuration
            .spatial_merge_size
            .checked_mul(configuration.spatial_merge_size)
            .ok_or(MultimodalTextError::Overflow("Qwen merger unit"))?;
        if shape
            != [
                usize_to_u64(tokens, "Qwen merger tokens")?,
                usize_to_u64(configuration.hidden_size, "Qwen merger hidden")?,
            ]
            || !tokens.is_multiple_of(merge_unit)
        {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen merger input does not contain complete spatial groups",
            ));
        }
        let merge_width = configuration
            .hidden_size
            .checked_mul(merge_unit)
            .ok_or(MultimodalTextError::Overflow("Qwen merger width"))?;
        let mut normalization = self.normalization.clone();
        let merged = if self.normalization_after_merge {
            let reshaped = qwen_tensor(
                backend,
                &[tokens / merge_unit, merge_width],
                &tensor_to_f32(backend, input, context)?,
                context,
            )?;
            normalization.forward_with_context(backend, &reshaped, context)?
        } else {
            let normalized = normalization.forward_with_context(backend, input, context)?;
            qwen_tensor(
                backend,
                &[tokens / merge_unit, merge_width],
                &tensor_to_f32(backend, &normalized, context)?,
                context,
            )?
        };
        let mut first = self.first.clone();
        let hidden = first.forward_with_context(backend, &merged, context)?;
        let mut activation = self.activation.clone();
        let hidden = activation.forward_with_context(backend, &hidden, context)?;
        let mut second = self.second.clone();
        Ok(second.forward_with_context(backend, &hidden, context)?)
    }
}

fn qwen_linear_module(
    name: &str,
    input_features: usize,
    output_features: usize,
    weight: Tensor,
    bias: Option<Tensor>,
    stream: comfy_tensor::StreamId,
) -> Result<NativeModule, MultimodalTextError> {
    qwen_require_parameter_shape(&weight, &[output_features, input_features], stream)?;
    if let Some(bias) = bias.as_ref() {
        qwen_require_parameter_shape(bias, &[output_features], stream)?;
    }
    let mut module =
        NativeModule::linear(name, input_features, output_features, bias.is_some(), false)?;
    module.load_dense_parameters(weight, bias)?;
    Ok(module)
}

fn qwen_layer_norm_module(
    name: &str,
    width: usize,
    weight: Tensor,
    bias: Tensor,
    stream: comfy_tensor::StreamId,
) -> Result<NativeModule, MultimodalTextError> {
    qwen_require_parameter_shape(&weight, &[width], stream)?;
    qwen_require_parameter_shape(&bias, &[width], stream)?;
    let mut module = NativeModule::layer_norm(name, vec![width], 1.0e-6, true, true, false)?;
    module.load_dense_parameters(weight, Some(bias))?;
    Ok(module)
}

fn qwen_vision_block(
    index: usize,
    configuration: &QwenVisionConfiguration,
    weights: QwenVisionBlockWeights,
    stream: comfy_tensor::StreamId,
) -> Result<NativeQwenVisionBlock, MultimodalTextError> {
    let hidden_size = configuration.hidden_size;
    let intermediate_size = configuration.intermediate_size;
    let prefix = format!("qwen_vision.blocks.{index}");
    Ok(NativeQwenVisionBlock {
        normalization_one: qwen_layer_norm_module(
            &format!("{prefix}.norm1"),
            hidden_size,
            weights.normalization_one_weight,
            weights.normalization_one_bias,
            stream,
        )?,
        query_key_value: qwen_linear_module(
            &format!("{prefix}.qkv"),
            hidden_size,
            hidden_size
                .checked_mul(3)
                .ok_or(MultimodalTextError::Overflow("Qwen vision QKV width"))?,
            weights.query_key_value_weight,
            Some(weights.query_key_value_bias),
            stream,
        )?,
        attention_output: qwen_linear_module(
            &format!("{prefix}.attention_output"),
            hidden_size,
            hidden_size,
            weights.attention_output_weight,
            Some(weights.attention_output_bias),
            stream,
        )?,
        normalization_two: qwen_layer_norm_module(
            &format!("{prefix}.norm2"),
            hidden_size,
            weights.normalization_two_weight,
            weights.normalization_two_bias,
            stream,
        )?,
        feed_forward_up: qwen_linear_module(
            &format!("{prefix}.mlp_up"),
            hidden_size,
            intermediate_size,
            weights.feed_forward_up_weight,
            Some(weights.feed_forward_up_bias),
            stream,
        )?,
        feed_forward_activation: NativeModule::gelu(
            format!("{prefix}.mlp_activation"),
            GeluApproximation::Tanh,
        )?,
        feed_forward_down: qwen_linear_module(
            &format!("{prefix}.mlp_down"),
            intermediate_size,
            hidden_size,
            weights.feed_forward_down_weight,
            Some(weights.feed_forward_down_bias),
            stream,
        )?,
    })
}

fn qwen_vision_merger(
    name: &str,
    configuration: &QwenVisionConfiguration,
    weights: QwenVisionMergerWeights,
    normalization_after_merge: bool,
    stream: comfy_tensor::StreamId,
) -> Result<NativeQwenVisionMerger, MultimodalTextError> {
    let merge_unit = configuration
        .spatial_merge_size
        .checked_mul(configuration.spatial_merge_size)
        .ok_or(MultimodalTextError::Overflow("Qwen vision merge unit"))?;
    let merge_width = configuration
        .hidden_size
        .checked_mul(merge_unit)
        .ok_or(MultimodalTextError::Overflow("Qwen vision merge width"))?;
    let normalization_width = if normalization_after_merge {
        merge_width
    } else {
        configuration.hidden_size
    };
    Ok(NativeQwenVisionMerger {
        normalization: qwen_layer_norm_module(
            &format!("{name}.norm"),
            normalization_width,
            weights.normalization_weight,
            weights.normalization_bias,
            stream,
        )?,
        first: qwen_linear_module(
            &format!("{name}.linear_one"),
            merge_width,
            merge_width,
            weights.first_weight,
            Some(weights.first_bias),
            stream,
        )?,
        activation: NativeModule::gelu(format!("{name}.activation"), GeluApproximation::None)?,
        second: qwen_linear_module(
            &format!("{name}.linear_two"),
            merge_width,
            configuration.output_hidden_size,
            weights.second_weight,
            Some(weights.second_bias),
            stream,
        )?,
        normalization_after_merge,
    })
}

fn qwen_require_parameter_shape(
    tensor: &Tensor,
    expected: &[usize],
    stream: comfy_tensor::StreamId,
) -> Result<(), MultimodalTextError> {
    let mut expected_shape = Vec::new();
    expected_shape
        .try_reserve_exact(expected.len())
        .map_err(|_| MultimodalTextError::Overflow("Qwen parameter shape"))?;
    for value in expected {
        expected_shape.push(usize_to_u64(*value, "Qwen parameter shape")?);
    }
    let descriptor = tensor.descriptor();
    if descriptor.shape() != expected_shape
        || descriptor.dtype() != DType::F32
        || descriptor.device() != DeviceId::CPU
        || descriptor.stream() != stream
    {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen vision parameter shape or execution target is invalid",
        ));
    }
    for chunk in tensor
        .contiguous_bytes()?
        .chunks_exact(std::mem::size_of::<f32>())
    {
        let bytes: [u8; 4] = chunk.try_into().map_err(|_| {
            MultimodalTextError::InvalidInput("Qwen vision parameter storage is malformed")
        })?;
        if !f32::from_ne_bytes(bytes).is_finite() {
            return Err(MultimodalTextError::InvalidInput(
                "Qwen vision parameters must be finite",
            ));
        }
    }
    Ok(())
}

fn qwen_tensor(
    backend: &CpuBackend,
    shape: &[usize],
    values: &[f32],
    context: &ExecutionContext<'_>,
) -> Result<Tensor, MultimodalTextError> {
    let mut tensor_shape = Vec::new();
    tensor_shape
        .try_reserve_exact(shape.len())
        .map_err(|_| MultimodalTextError::Overflow("Qwen tensor shape"))?;
    for value in shape {
        tensor_shape.push(usize_to_u64(*value, "Qwen tensor shape")?);
    }
    Ok(tensor_from_f32(backend, &tensor_shape, values, context)?)
}

fn qwen_add_tensors(
    backend: &CpuBackend,
    left: &Tensor,
    right: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, MultimodalTextError> {
    Ok(native_tensor_add(backend, left, right, context)?)
}

fn qwen_add_interpolated_positions(
    hidden: &mut [f32],
    grid_thw: [usize; 3],
    position_embedding: &[f32],
    hidden_size: usize,
    merge_size: usize,
    cancellation: &comfy_types::CancellationToken,
) -> Result<(), MultimodalTextError> {
    let [frames, height, width] = grid_thw;
    if frames == 0
        || height == 0
        || width == 0
        || !height.is_multiple_of(merge_size)
        || !width.is_multiple_of(merge_size)
        || position_embedding.len()
            != 48_usize
                .checked_mul(48)
                .and_then(|value| value.checked_mul(hidden_size))
                .ok_or(MultimodalTextError::Overflow("Qwen position table"))?
    {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen vision position interpolation geometry is invalid",
        ));
    }
    let token_count = frames
        .checked_mul(height)
        .and_then(|value| value.checked_mul(width))
        .ok_or(MultimodalTextError::Overflow("Qwen position token count"))?;
    if hidden.len()
        != token_count
            .checked_mul(hidden_size)
            .ok_or(MultimodalTextError::Overflow("Qwen hidden positions"))?
    {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen hidden state does not match its image grid",
        ));
    }
    let denominator_height = height.saturating_sub(1).max(1) as f32;
    let denominator_width = width.saturating_sub(1).max(1) as f32;
    let merged_height = height / merge_size;
    let merged_width = width / merge_size;
    let mut output_token = 0_usize;
    for _frame in 0..frames {
        for block_y in 0..merged_height {
            for block_x in 0..merged_width {
                cancellation.check()?;
                for merge_y in 0..merge_size {
                    for merge_x in 0..merge_size {
                        let y = block_y * merge_size + merge_y;
                        let x = block_x * merge_size + merge_x;
                        let source_y = y as f32 * 47.0 / denominator_height;
                        let source_x = x as f32 * 47.0 / denominator_width;
                        let low_y = source_y.floor() as usize;
                        let low_x = source_x.floor() as usize;
                        let high_y = low_y.saturating_add(1).min(47);
                        let high_x = low_x.saturating_add(1).min(47);
                        let fraction_y = source_y - low_y as f32;
                        let fraction_x = source_x - low_x as f32;
                        let neighbors = [
                            (low_y, low_x, (1.0 - fraction_y) * (1.0 - fraction_x)),
                            (low_y, high_x, (1.0 - fraction_y) * fraction_x),
                            (high_y, low_x, fraction_y * (1.0 - fraction_x)),
                            (high_y, high_x, fraction_y * fraction_x),
                        ];
                        let output_start = output_token
                            .checked_mul(hidden_size)
                            .ok_or(MultimodalTextError::Overflow("Qwen position output"))?;
                        for hidden_index in 0..hidden_size {
                            let mut position_value = 0.0_f32;
                            for (source_y, source_x, weight) in neighbors {
                                let source_index = source_y
                                    .checked_mul(48)
                                    .and_then(|value| value.checked_add(source_x))
                                    .and_then(|value| value.checked_mul(hidden_size))
                                    .and_then(|value| value.checked_add(hidden_index))
                                    .ok_or(MultimodalTextError::Overflow("Qwen position source"))?;
                                position_value += *position_embedding.get(source_index).ok_or(
                                    MultimodalTextError::InvalidInput(
                                        "Qwen position table storage is incomplete",
                                    ),
                                )? * weight;
                            }
                            let output_index = output_start
                                .checked_add(hidden_index)
                                .ok_or(MultimodalTextError::Overflow("Qwen position output"))?;
                            *hidden.get_mut(output_index).ok_or(
                                MultimodalTextError::InvalidInput(
                                    "Qwen hidden state storage is incomplete",
                                ),
                            )? += position_value;
                        }
                        output_token += 1;
                    }
                }
            }
        }
    }
    Ok(())
}

fn qwen_vision_attention(
    backend: &CpuBackend,
    projected: &[f32],
    grid_thw: [usize; 3],
    hidden_size: usize,
    heads: usize,
    context: &ExecutionContext<'_>,
) -> Result<Vec<f32>, MultimodalTextError> {
    let token_count = grid_thw[0]
        .checked_mul(grid_thw[1])
        .and_then(|value| value.checked_mul(grid_thw[2]))
        .ok_or(MultimodalTextError::Overflow("Qwen attention tokens"))?;
    let head_dimension = hidden_size
        .checked_div(heads)
        .ok_or(MultimodalTextError::Overflow(
            "Qwen attention head dimension",
        ))?;
    if token_count == 0
        || heads == 0
        || head_dimension == 0
        || !head_dimension.is_multiple_of(4)
        || projected.len()
            != token_count
                .checked_mul(hidden_size)
                .and_then(|value| value.checked_mul(3))
                .ok_or(MultimodalTextError::Overflow("Qwen QKV projection"))?
    {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen vision attention geometry is invalid",
        ));
    }
    let vector_count = token_count
        .checked_mul(hidden_size)
        .ok_or(MultimodalTextError::Overflow("Qwen attention vectors"))?;
    let mut query = Vec::new();
    let mut key = Vec::new();
    let mut value = Vec::new();
    for output in [&mut query, &mut key, &mut value] {
        output
            .try_reserve_exact(vector_count)
            .map_err(|_| MultimodalTextError::Overflow("Qwen attention vectors"))?;
    }
    let [frames, height, width] = grid_thw;
    let merge_size = QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE;
    let merged_height = height / merge_size;
    let merged_width = width / merge_size;
    let frequency_width = head_dimension / 4;
    let mut token_index = 0_usize;
    for _frame in 0..frames {
        for block_y in 0..merged_height {
            for block_x in 0..merged_width {
                context.cancellation.check()?;
                for merge_y in 0..merge_size {
                    for merge_x in 0..merge_size {
                        let row = block_y * merge_size + merge_y;
                        let column = block_x * merge_size + merge_x;
                        let row_offset = token_index
                            .checked_mul(hidden_size)
                            .and_then(|value| value.checked_mul(3))
                            .ok_or(MultimodalTextError::Overflow("Qwen QKV row"))?;
                        for head in 0..heads {
                            let head_offset = head
                                .checked_mul(head_dimension)
                                .ok_or(MultimodalTextError::Overflow("Qwen attention head"))?;
                            let mut query_head = Vec::new();
                            let mut key_head = Vec::new();
                            query_head
                                .try_reserve_exact(head_dimension)
                                .map_err(|_| MultimodalTextError::Overflow("Qwen query head"))?;
                            key_head
                                .try_reserve_exact(head_dimension)
                                .map_err(|_| MultimodalTextError::Overflow("Qwen key head"))?;
                            for dimension in 0..head_dimension {
                                let query_index = row_offset
                                    .checked_add(head_offset)
                                    .and_then(|value| value.checked_add(dimension))
                                    .ok_or(MultimodalTextError::Overflow("Qwen query index"))?;
                                let key_index = row_offset
                                    .checked_add(hidden_size)
                                    .and_then(|value| value.checked_add(head_offset))
                                    .and_then(|value| value.checked_add(dimension))
                                    .ok_or(MultimodalTextError::Overflow("Qwen key index"))?;
                                query_head.push(*projected.get(query_index).ok_or(
                                    MultimodalTextError::InvalidInput(
                                        "Qwen query projection storage is incomplete",
                                    ),
                                )?);
                                key_head.push(*projected.get(key_index).ok_or(
                                    MultimodalTextError::InvalidInput(
                                        "Qwen key projection storage is incomplete",
                                    ),
                                )?);
                            }
                            qwen_apply_vision_rope(&mut query_head, row, column, frequency_width)?;
                            qwen_apply_vision_rope(&mut key_head, row, column, frequency_width)?;
                            query.extend_from_slice(&query_head);
                            key.extend_from_slice(&key_head);
                            let value_start = row_offset
                                .checked_add(hidden_size.checked_mul(2).ok_or(
                                    MultimodalTextError::Overflow("Qwen value projection"),
                                )?)
                                .and_then(|value| value.checked_add(head_offset))
                                .ok_or(MultimodalTextError::Overflow("Qwen value head"))?;
                            let value_end = value_start
                                .checked_add(head_dimension)
                                .ok_or(MultimodalTextError::Overflow("Qwen value head"))?;
                            value.extend_from_slice(projected.get(value_start..value_end).ok_or(
                                MultimodalTextError::InvalidInput(
                                    "Qwen value projection storage is incomplete",
                                ),
                            )?);
                        }
                        token_index += 1;
                    }
                }
            }
        }
    }
    let workspace_limit_bytes = token_count
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or(MultimodalTextError::Overflow("Qwen attention workspace"))?;
    let outcome = scaled_dot_product_attention_with_context(
        backend,
        AttentionRequest {
            backend: AttentionBackend::PytorchSdp,
            fallback: AttentionFallbackPolicy::AllowExactNative,
            batch: 1,
            query_tokens: token_count,
            key_tokens: token_count,
            heads,
            head_dimension,
            value_dimension: head_dimension,
            scale: None,
            workspace_limit_bytes,
        },
        &query,
        &key,
        &value,
        None,
        context,
    )?;
    Ok(outcome.values)
}

fn qwen_apply_vision_rope(
    values: &mut [f32],
    row: usize,
    column: usize,
    frequency_width: usize,
) -> Result<(), MultimodalTextError> {
    let expected_width = frequency_width
        .checked_mul(4)
        .ok_or(MultimodalTextError::Overflow("Qwen rotary width"))?;
    if frequency_width == 0 || values.len() != expected_width {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen rotary vector width is invalid",
        ));
    }
    let half = values.len() / 2;
    for index in 0..half {
        let coordinate = if index < frequency_width { row } else { column };
        let frequency_index = index % frequency_width;
        let exponent = (frequency_index * 2) as f32 / (frequency_width * 2) as f32;
        let angle = coordinate as f32 / 10_000.0_f32.powf(exponent);
        let cosine = angle.cos();
        let sine = angle.sin();
        let second_index = index
            .checked_add(half)
            .ok_or(MultimodalTextError::Overflow("Qwen rotary index"))?;
        let first = *values.get(index).ok_or(MultimodalTextError::InvalidInput(
            "Qwen rotary vector storage is incomplete",
        ))?;
        let second = *values
            .get(second_index)
            .ok_or(MultimodalTextError::InvalidInput(
                "Qwen rotary vector storage is incomplete",
            ))?;
        *values
            .get_mut(index)
            .ok_or(MultimodalTextError::InvalidInput(
                "Qwen rotary vector storage is incomplete",
            ))? = first * cosine - second * sine;
        *values
            .get_mut(second_index)
            .ok_or(MultimodalTextError::InvalidInput(
                "Qwen rotary vector storage is incomplete",
            ))? = second * cosine + first * sine;
    }
    Ok(())
}

pub fn join_multimodal_embeddings(
    backend: &CpuBackend,
    text_embeddings: &Tensor,
    images: &[MultimodalImageEmbedding<'_>],
    context: &ExecutionContext<'_>,
) -> Result<Tensor, MultimodalTextError> {
    context.cancellation.check()?;
    let shape = text_embeddings.descriptor().shape();
    if shape.len() != 3 || shape[0] == 0 || shape[1] == 0 || shape[2] == 0 {
        return Err(MultimodalTextError::InvalidInput(
            "text embeddings must use nonempty [batch, tokens, hidden] shape",
        ));
    }
    require_cpu_f32(text_embeddings, context)?;
    if images.is_empty() {
        return Ok(text_embeddings.clone());
    }
    let sequence_length = u64_to_usize(shape[1], "sequence length")?;
    let mut pieces = Vec::new();
    pieces
        .try_reserve_exact(images.len().saturating_mul(2).saturating_add(1))
        .map_err(|_| MultimodalTextError::Overflow("joined embedding pieces"))?;
    let mut cursor = 0_usize;
    for image in images {
        context.cancellation.check()?;
        let end = image
            .span
            .start
            .checked_add(image.span.size)
            .ok_or(MultimodalTextError::Overflow("image span end"))?;
        if image.span.size == 0 || image.span.start < cursor || end > sequence_length {
            return Err(MultimodalTextError::InvalidInput(
                "image spans must be ordered, nonempty, and in bounds",
            ));
        }
        if image.span.start > cursor {
            pieces.push(narrow_method_exact_native(
                text_embeddings,
                1,
                usize_to_i64(cursor, "text prefix")?,
                usize_to_u64(image.span.start - cursor, "text prefix")?,
                context.cancellation,
            )?);
        }
        pieces.push(normalize_image_embedding(
            backend,
            image.embedding,
            shape[0],
            usize_to_u64(image.span.size, "image span")?,
            shape[2],
            context,
        )?);
        cursor = end;
    }
    if cursor < sequence_length {
        pieces.push(narrow_method_exact_native(
            text_embeddings,
            1,
            usize_to_i64(cursor, "text suffix")?,
            usize_to_u64(sequence_length - cursor, "text suffix")?,
            context.cancellation,
        )?);
    }
    if pieces.is_empty() {
        return Err(MultimodalTextError::InvalidInput(
            "joined embedding graph produced no pieces",
        ));
    }
    let joined = torch_cat_with_context_exact_native(backend, &pieces, 1, context)?;
    context.cancellation.check()?;
    Ok(joined)
}

pub fn join_qwen3vl_deepstack(
    backend: &CpuBackend,
    sequence_length: usize,
    images: &[MultimodalImageEmbedding<'_>],
    context: &ExecutionContext<'_>,
) -> Result<Option<MultimodalDeepstackJoin>, MultimodalTextError> {
    context.cancellation.check()?;
    if images.is_empty() {
        return Ok(None);
    }
    let layer_count = images[0].deepstack.len();
    if layer_count == 0 {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL images require deepstack layers",
        ));
    }
    let mut mask = vec![false; sequence_length];
    let mut per_layer = (0..layer_count)
        .map(|_| Vec::<Tensor>::new())
        .collect::<Vec<_>>();
    let mut previous_end = 0_usize;
    for image in images {
        context.cancellation.check()?;
        let end = image
            .span
            .start
            .checked_add(image.span.size)
            .ok_or(MultimodalTextError::Overflow("deepstack image span"))?;
        if image.span.size == 0
            || image.span.start < previous_end
            || end > sequence_length
            || image.deepstack.len() != layer_count
        {
            return Err(MultimodalTextError::InvalidInput(
                "deepstack spans or layer counts are invalid",
            ));
        }
        mask[image.span.start..end].fill(true);
        for (layer, tensor) in image.deepstack.iter().enumerate() {
            require_cpu_f32(tensor, context)?;
            let shape = tensor.descriptor().shape();
            if shape.len() != 2 || shape[0] != usize_to_u64(image.span.size, "deepstack span")? {
                return Err(MultimodalTextError::InvalidInput(
                    "deepstack layers must use [image tokens, hidden] shape",
                ));
            }
            per_layer[layer].push(tensor.clone());
        }
        previous_end = end;
    }
    let mut layers = Vec::new();
    layers
        .try_reserve_exact(layer_count)
        .map_err(|_| MultimodalTextError::Overflow("deepstack layers"))?;
    for inputs in per_layer {
        layers.push(torch_cat_with_context_exact_native(
            backend, &inputs, 0, context,
        )?);
    }
    context.cancellation.check()?;
    Ok(Some(MultimodalDeepstackJoin {
        visual_position_mask: mask,
        layers,
    }))
}

pub fn ideogram4_project_taps(
    backend: &CpuBackend,
    tapped_layers: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, MultimodalTextError> {
    context.cancellation.check()?;
    require_cpu_f32(tapped_layers, context)?;
    let shape = tapped_layers.descriptor().shape();
    if shape.len() != 4 || shape[1] != IDEOGRAM4_TAP_LAYERS.len() as u64 {
        return Err(MultimodalTextError::InvalidInput(
            "Ideogram4 taps must use [batch, 13, tokens, hidden] shape",
        ));
    }
    let projected_width = shape[3]
        .checked_mul(shape[1])
        .ok_or(MultimodalTextError::Overflow("Ideogram4 projection width"))?;
    let permuted = tensor_permute_exact_native(tapped_layers, &[0, 2, 3, 1], context.cancellation)?;
    let projected = torch_reshape_with_context_exact_native(
        backend,
        &permuted,
        &[
            u64_to_i64(shape[0], "Ideogram4 batch")?,
            u64_to_i64(shape[2], "Ideogram4 tokens")?,
            u64_to_i64(projected_width, "Ideogram4 width")?,
        ],
        context,
    )?;
    context.cancellation.check()?;
    Ok(projected)
}

pub fn ovis_template_end(tokens: &[i64]) -> Result<usize, MultimodalTextError> {
    let mut start =
        tokens
            .iter()
            .position(|token| *token == 4004)
            .ok_or(MultimodalTextError::InvalidInput(
                "Ovis prompt is missing the first im_start token",
            ))?;
    if tokens.get(start + 1) == Some(&25) {
        start += 1;
    }
    Ok(start)
}

pub fn trim_ovis_conditioning(
    conditioning: &Tensor,
    template_end: usize,
    cancellation: &comfy_types::CancellationToken,
) -> Result<Tensor, MultimodalTextError> {
    cancellation.check()?;
    let shape = conditioning.descriptor().shape();
    if shape.len() != 3 || template_end >= u64_to_usize(shape[1], "Ovis tokens")? {
        return Err(MultimodalTextError::InvalidInput(
            "Ovis conditioning must be rank three and retain at least one token",
        ));
    }
    Ok(narrow_method_exact_native(
        conditioning,
        1,
        usize_to_i64(template_end, "Ovis template end")?,
        shape[1] - usize_to_u64(template_end, "Ovis template end")?,
        cancellation,
    )?)
}

pub fn parse_sam3_prompts(text: &str) -> Result<Vec<Sam3Prompt>, MultimodalTextError> {
    const MAX_PROMPTS: usize = 1024;
    const MAX_PROMPT_BYTES: usize = 1 << 20;
    if text.len() > MAX_PROMPT_BYTES {
        return Err(MultimodalTextError::InvalidInput(
            "SAM3 prompt bytes exceed the native bound",
        ));
    }
    let normalized = text.replace(['(', ')'], "");
    let mut prompts = Vec::new();
    for part in normalized
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if prompts.len() == MAX_PROMPTS {
            return Err(MultimodalTextError::InvalidInput(
                "SAM3 prompt count exceeds the native bound",
            ));
        }
        let (prompt, maximum_detections) = parse_sam3_prompt_part(part)?;
        prompts.push(Sam3Prompt {
            text: prompt.to_owned(),
            maximum_detections,
        });
    }
    Ok(prompts)
}

pub fn pack_sam3_conditions(
    conditions: Vec<Sam3EncodedCondition>,
    first_pooled: Option<Tensor>,
) -> Result<Sam3ConditionPack, MultimodalTextError> {
    let first = conditions.first().ok_or(MultimodalTextError::InvalidInput(
        "SAM3 condition packing requires at least one prompt",
    ))?;
    if conditions
        .iter()
        .any(|condition| condition.maximum_detections == 0)
    {
        return Err(MultimodalTextError::InvalidInput(
            "SAM3 maximum detections must be nonzero",
        ));
    }
    Ok(Sam3ConditionPack {
        main_condition: first.condition.clone(),
        first_pooled,
        conditions,
    })
}

pub fn format_ideogram4_prompt(text: &str) -> String {
    if text.starts_with("<|im_start|>") {
        text.to_owned()
    } else {
        format!("<|im_start|>user\n{text}<|im_end|>\n<|im_start|>assistant\n")
    }
}

pub fn format_ovis_prompt(text: &str) -> String {
    format!(
        "<|im_start|>user\nDescribe the image by detailing the color, quantity, text, shape, size, texture, spatial relationships of the objects and background: {text}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
    )
}

pub fn format_qwen3vl_prompt(text: &str, image_count: usize, thinking: bool) -> String {
    if text.starts_with("<|im_start|>") {
        return text.to_owned();
    }
    let vision = "<|vision_start|><|image_pad|><|vision_end|>".repeat(image_count);
    let mut prompt = if image_count == 0 {
        format!("<|im_start|>user\n{text}<|im_end|>\n<|im_start|>assistant\n")
    } else {
        format!("<|im_start|>user\n{vision}{text}<|im_end|>\n<|im_start|>assistant\n")
    };
    if !thinking {
        prompt.push_str("<think>\n\n</think>\n\n");
    }
    prompt
}

fn parse_sam3_prompt_part(part: &str) -> Result<(&str, usize), MultimodalTextError> {
    let Some((text, raw_limit)) = part.rsplit_once(':') else {
        return Ok((part, 1));
    };
    let text = text.trim();
    let raw_limit = raw_limit.trim();
    if text.is_empty()
        || raw_limit.is_empty()
        || !raw_limit
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
    {
        return Ok((part, 1));
    }
    let value = raw_limit
        .parse::<f64>()
        .map_err(|_| MultimodalTextError::InvalidPromptLimit(raw_limit.to_owned()))?;
    if !value.is_finite() || value < 0.0 || value > usize::MAX as f64 {
        return Err(MultimodalTextError::InvalidPromptLimit(
            raw_limit.to_owned(),
        ));
    }
    let rounded = value.round_ties_even().max(1.0);
    Ok((text, rounded as usize))
}

fn prepare_qwen3vl_resized_image(
    backend: &CpuBackend,
    image: &ImageTensor,
    height: u64,
    width: u64,
    family: QwenVisionFamily,
    context: &ExecutionContext<'_>,
) -> Result<Qwen3VlPreparedImage, MultimodalTextError> {
    context.cancellation.check()?;
    let (batch, actual_height, actual_width, channels) = image.dimensions()?;
    if (batch, actual_height, actual_width, channels) != (1, height, width, 3) {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL resized image geometry is inconsistent",
        ));
    }
    let patch_size = u64::try_from(QWEN3VL_IMAGE_PATCH_SIZE)
        .map_err(|_| MultimodalTextError::Overflow("Qwen3-VL patch size"))?;
    let merge_size = u64::try_from(QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE)
        .map_err(|_| MultimodalTextError::Overflow("Qwen3-VL merge size"))?;
    let grid_height = height
        .checked_div(patch_size)
        .ok_or(MultimodalTextError::Overflow("Qwen3-VL grid height"))?;
    let grid_width = width
        .checked_div(patch_size)
        .ok_or(MultimodalTextError::Overflow("Qwen3-VL grid width"))?;
    if grid_height == 0
        || grid_width == 0
        || !grid_height.is_multiple_of(merge_size)
        || !grid_width.is_multiple_of(merge_size)
    {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL patch grid must contain complete spatial merge blocks",
        ));
    }
    let patch_count = grid_height
        .checked_mul(grid_width)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(MultimodalTextError::Overflow("Qwen3-VL patch count"))?;
    let patch_width = 3_usize
        .checked_mul(QWEN3VL_IMAGE_TEMPORAL_PATCH_SIZE)
        .and_then(|value| value.checked_mul(QWEN3VL_IMAGE_PATCH_SIZE))
        .and_then(|value| value.checked_mul(QWEN3VL_IMAGE_PATCH_SIZE))
        .ok_or(MultimodalTextError::Overflow(
            "Qwen3-VL flattened patch width",
        ))?;
    let patch_elements = patch_count
        .checked_mul(patch_width)
        .ok_or(MultimodalTextError::Overflow("Qwen3-VL patch elements"))?;
    let values = image.as_f32_slice()?;
    let (mean, standard_deviation) = family.normalization();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL image pixels must be finite",
        ));
    }
    let height_usize = u64_to_usize(height, "Qwen3-VL target height")?;
    let width_usize = u64_to_usize(width, "Qwen3-VL target width")?;
    let grid_height_usize = u64_to_usize(grid_height, "Qwen3-VL grid height")?;
    let grid_width_usize = u64_to_usize(grid_width, "Qwen3-VL grid width")?;
    let merged_height = grid_height_usize / QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE;
    let merged_width = grid_width_usize / QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE;
    let mut patches = backend.workspace_vec(context, patch_elements)?;
    for block_y in 0..merged_height {
        for block_x in 0..merged_width {
            context.cancellation.check()?;
            for merge_y in 0..QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE {
                for merge_x in 0..QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE {
                    let patch_y = block_y * QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE + merge_y;
                    let patch_x = block_x * QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE + merge_x;
                    for channel in 0..3_usize {
                        for _temporal in 0..QWEN3VL_IMAGE_TEMPORAL_PATCH_SIZE {
                            for local_y in 0..QWEN3VL_IMAGE_PATCH_SIZE {
                                for local_x in 0..QWEN3VL_IMAGE_PATCH_SIZE {
                                    let y = patch_y * QWEN3VL_IMAGE_PATCH_SIZE + local_y;
                                    let x = patch_x * QWEN3VL_IMAGE_PATCH_SIZE + local_x;
                                    let index = y
                                        .checked_mul(width_usize)
                                        .and_then(|value| value.checked_add(x))
                                        .and_then(|value| value.checked_mul(3))
                                        .and_then(|value| value.checked_add(channel))
                                        .ok_or(MultimodalTextError::Overflow(
                                            "Qwen3-VL normalized pixel index",
                                        ))?;
                                    let value = values.get(index).copied().ok_or(
                                        MultimodalTextError::InvalidInput(
                                            "Qwen3-VL resized image storage is incomplete",
                                        ),
                                    )?;
                                    patches.try_push(
                                        (value - mean[channel]) / standard_deviation[channel],
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let reconstructed_height = merged_height
        .checked_mul(
            QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE
                .checked_mul(QWEN3VL_IMAGE_PATCH_SIZE)
                .ok_or(MultimodalTextError::Overflow(
                    "Qwen3-VL reconstructed height",
                ))?,
        )
        .ok_or(MultimodalTextError::Overflow(
            "Qwen3-VL reconstructed height",
        ))?;
    let reconstructed_width = merged_width
        .checked_mul(
            QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE
                .checked_mul(QWEN3VL_IMAGE_PATCH_SIZE)
                .ok_or(MultimodalTextError::Overflow(
                    "Qwen3-VL reconstructed width",
                ))?,
        )
        .ok_or(MultimodalTextError::Overflow(
            "Qwen3-VL reconstructed width",
        ))?;
    if patches.len() != patch_elements
        || height_usize != reconstructed_height
        || width_usize != reconstructed_width
    {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL patch packing did not cover the resized image exactly",
        ));
    }
    let descriptor = TensorDescriptor::contiguous(
        vec![
            usize_to_u64(patch_count, "Qwen3-VL patch count")?,
            3,
            usize_to_u64(
                QWEN3VL_IMAGE_TEMPORAL_PATCH_SIZE,
                "Qwen3-VL temporal patch size",
            )?,
            usize_to_u64(QWEN3VL_IMAGE_PATCH_SIZE, "Qwen3-VL patch height")?,
            usize_to_u64(QWEN3VL_IMAGE_PATCH_SIZE, "Qwen3-VL patch width")?,
        ],
        DType::F32,
        DeviceId::CPU,
        context.stream,
    )?;
    let patches = backend.upload_f32(descriptor, &patches, context)?.0;
    let merged_tokens = merged_height
        .checked_mul(merged_width)
        .ok_or(MultimodalTextError::Overflow("Qwen3-VL merged tokens"))?;
    context.cancellation.check()?;
    Ok(Qwen3VlPreparedImage {
        patches,
        grid_thw: [1, grid_height_usize, grid_width_usize],
        merged_tokens,
        family,
    })
}

fn round_to_factor(value: u64, factor: u64) -> Result<u64, MultimodalTextError> {
    let units = ((value as f64) / (factor as f64)).round_ties_even();
    scaled_units_to_dimension(units, factor, false)
}

fn floor_scaled_to_factor(value: u64, beta: f64, factor: u64) -> Result<u64, MultimodalTextError> {
    if !beta.is_finite() || beta <= 0.0 {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL resize scale must be finite and positive",
        ));
    }
    let units = ((value as f64) / beta / (factor as f64)).floor();
    scaled_units_to_dimension(units, factor, true)
}

fn ceil_scaled_to_factor(value: u64, beta: f64, factor: u64) -> Result<u64, MultimodalTextError> {
    if !beta.is_finite() || beta <= 0.0 {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL resize scale must be finite and positive",
        ));
    }
    let units = ((value as f64) * beta / (factor as f64)).ceil();
    scaled_units_to_dimension(units, factor, false)
}

fn scaled_units_to_dimension(
    units: f64,
    factor: u64,
    minimum_one: bool,
) -> Result<u64, MultimodalTextError> {
    if !units.is_finite() || units < 0.0 || units > (u64::MAX / factor) as f64 {
        return Err(MultimodalTextError::Overflow(
            "Qwen3-VL scaled image dimension",
        ));
    }
    let units = units as u64;
    let units = if minimum_one { units.max(1) } else { units };
    units
        .checked_mul(factor)
        .ok_or(MultimodalTextError::Overflow(
            "Qwen3-VL scaled image dimension",
        ))
}

fn sequential_positions(length: usize) -> Result<Vec<i64>, MultimodalTextError> {
    let mut positions = Vec::new();
    positions
        .try_reserve_exact(length)
        .map_err(|_| MultimodalTextError::Overflow("sequential positions"))?;
    for index in 0..length {
        positions.push(usize_to_i64(index, "sequential position")?);
    }
    Ok(positions)
}

fn clone_positions(positions: &[i64]) -> Result<Vec<i64>, MultimodalTextError> {
    let mut cloned = Vec::new();
    cloned
        .try_reserve_exact(positions.len())
        .map_err(|_| MultimodalTextError::Overflow("multimodal positions"))?;
    cloned.extend_from_slice(positions);
    Ok(cloned)
}

fn normalize_image_embedding(
    backend: &CpuBackend,
    embedding: &Tensor,
    batch: u64,
    tokens: u64,
    hidden: u64,
    context: &ExecutionContext<'_>,
) -> Result<Tensor, MultimodalTextError> {
    require_cpu_f32(embedding, context)?;
    let shape = embedding.descriptor().shape();
    match shape {
        [actual_tokens, actual_hidden]
            if batch == 1 && *actual_tokens == tokens && *actual_hidden == hidden =>
        {
            Ok(torch_reshape_with_context_exact_native(
                backend,
                embedding,
                &[
                    1,
                    u64_to_i64(tokens, "image tokens")?,
                    u64_to_i64(hidden, "hidden")?,
                ],
                context,
            )?)
        }
        [actual_batch, actual_tokens, actual_hidden]
            if *actual_batch == batch && *actual_tokens == tokens && *actual_hidden == hidden =>
        {
            Ok(embedding.clone())
        }
        _ => Err(MultimodalTextError::InvalidInput(
            "image embeddings must match [batch, image tokens, hidden]",
        )),
    }
}

fn require_cpu_f32(
    tensor: &Tensor,
    context: &ExecutionContext<'_>,
) -> Result<(), MultimodalTextError> {
    let descriptor = tensor.descriptor();
    if descriptor.dtype() != DType::F32
        || descriptor.device() != DeviceId::CPU
        || descriptor.stream() != context.stream
    {
        return Err(MultimodalTextError::InvalidInput(
            "multimodal tensors must be CPU F32 values on the caller stream",
        ));
    }
    Ok(())
}

fn usize_to_i64(value: usize, name: &'static str) -> Result<i64, MultimodalTextError> {
    i64::try_from(value).map_err(|_| MultimodalTextError::Overflow(name))
}

fn checked_f64_to_u64(value: f64, name: &'static str) -> Result<u64, MultimodalTextError> {
    if !value.is_finite() || value < 0.0 || value >= 18_446_744_073_709_551_616.0 {
        return Err(MultimodalTextError::Overflow(name));
    }
    Ok(value as u64)
}

fn usize_to_u64(value: usize, name: &'static str) -> Result<u64, MultimodalTextError> {
    u64::try_from(value).map_err(|_| MultimodalTextError::Overflow(name))
}

fn u64_to_usize(value: u64, name: &'static str) -> Result<usize, MultimodalTextError> {
    usize::try_from(value).map_err(|_| MultimodalTextError::Overflow(name))
}

fn u64_to_i64(value: u64, name: &'static str) -> Result<i64, MultimodalTextError> {
    i64::try_from(value).map_err(|_| MultimodalTextError::Overflow(name))
}
