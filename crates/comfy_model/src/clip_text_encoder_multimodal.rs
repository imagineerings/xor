use crate::{
    BidirectionalTextError, BidirectionalTextOutput, BidirectionalTextRequest, ClipTextError,
    ClipTextOutput, ClipTextRequest, ClipVisionError, ClipVisionIntermediate, ClipVisionOutput,
    DecoderTextError, DecoderTextOutput, DecoderTextRequest, NativeClipText, NativeClipVision,
    NativeDecoderTextEncoder, NativeT5TextEncoder,
};
use comfy_tensor::{
    CpuBackend, DType, DeviceId, ExecutionContext, ImageTensor, ResizeCrop, ResizeMode, Tensor,
    TensorDescriptor, TensorError,
    generated_indexing_masking_01::{IndexingMaskingPartOneError, narrow_method_exact_native},
    generated_shape_layout_transform_02::{
        ShapeLayoutTransformPartTwoError, torch_cat_with_context_exact_native,
        torch_reshape_with_context_exact_native,
    },
    generated_shape_layout_transform_03::{
        ShapeLayoutTransformPartThreeError, tensor_permute_exact_native,
    },
};
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
pub const QWEN3VL_IMAGE_MINIMUM_PIXELS: u64 = 3_136;
pub const QWEN3VL_IMAGE_MAXIMUM_PIXELS: u64 = 12_845_056;
pub const QWEN3VL_IMAGE_PATCH_SIZE: usize = 16;
pub const QWEN3VL_IMAGE_TEMPORAL_PATCH_SIZE: usize = 2;
pub const QWEN3VL_IMAGE_SPATIAL_MERGE_SIZE: usize = 2;

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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Qwen3VlMarkerPlan {
    expanded_tokens: Vec<i64>,
    spans: Vec<MultimodalSpan>,
    visual_position_mask: Vec<bool>,
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
    ClipText(#[from] ClipTextError),
    #[error(transparent)]
    ClipVision(#[from] ClipVisionError),
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

pub fn prepare_qwen3vl_images(
    backend: &CpuBackend,
    images: &ImageTensor,
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
    cancellation.check()?;
    let marker_count = tokens
        .iter()
        .filter(|token| **token == QWEN3VL_IMAGE_PAD_TOKEN)
        .count();
    if marker_count != images.len() {
        return Err(MultimodalTextError::InvalidInput(
            "Qwen3-VL image markers and prepared images must match exactly",
        ));
    }
    let mut expanded_length = 0_usize;
    let mut length_image_index = 0_usize;
    for token in tokens {
        let token_length = if *token == QWEN3VL_IMAGE_PAD_TOKEN {
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
        if token != QWEN3VL_IMAGE_PAD_TOKEN {
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
                                    patches.try_push((value - 0.5) / 0.5)?;
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

fn usize_to_u64(value: usize, name: &'static str) -> Result<u64, MultimodalTextError> {
    u64::try_from(value).map_err(|_| MultimodalTextError::Overflow(name))
}

fn u64_to_usize(value: u64, name: &'static str) -> Result<usize, MultimodalTextError> {
    usize::try_from(value).map_err(|_| MultimodalTextError::Overflow(name))
}

fn u64_to_i64(value: u64, name: &'static str) -> Result<i64, MultimodalTextError> {
    i64::try_from(value).map_err(|_| MultimodalTextError::Overflow(name))
}
