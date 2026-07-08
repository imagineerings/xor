use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ConditioningMode, DeviceBackend, GuidanceMode, LatentFormat, ModelFamilyExecutionProfile,
    ModelFamilyKind, SamplerKind,
};

pub const EMPTY_BUNDLE_CODE: &str = "world_model.conditioning.empty_bundle";
pub const EMPTY_TENSOR_CODE: &str = "world_model.conditioning.empty_tensor";
pub const INVALID_REGION_CODE: &str = "world_model.conditioning.invalid_region";
pub const UNSUPPORTED_CONDITIONING_CODE: &str = "world_model.conditioning.unsupported_mode";
pub const LATENT_MISMATCH_CODE: &str = "world_model.conditioning.latent_mismatch";
pub const SAMPLER_MISMATCH_CODE: &str = "world_model.conditioning.sampler_mismatch";
pub const BACKEND_MISMATCH_CODE: &str = "world_model.conditioning.backend_mismatch";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum TensorDtype {
    F16,
    F32,
    Bf16,
    I64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TensorDescriptor {
    pub id: String,
    pub shape: Vec<u32>,
    pub dtype: TensorDtype,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum EncoderKind {
    Clip,
    OpenClip,
    T5,
    ClipVision,
    Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EncoderIdentity {
    pub kind: EncoderKind,
    pub model_family: ModelFamilyKind,
    pub model_ref: String,
    pub tokenizer: Option<String>,
    pub layer_skip: Option<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PromptRole {
    Positive,
    Negative,
    Style,
    Reference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptMetadata {
    pub node_id: String,
    pub role: PromptRole,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttentionMetadata {
    pub clip_layer: Option<i32>,
    pub token_weights: BTreeMap<String, f32>,
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConditioningArea {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConditioningMask {
    pub tensor: TensorDescriptor,
    pub strength: f32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InpaintConditioning {
    pub image_ref: String,
    pub mask_ref: String,
    pub preserve_masked_latent: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConditioningRegion {
    pub area: Option<ConditioningArea>,
    pub mask: Option<ConditioningMask>,
    pub strength: f32,
    pub start_percent: Option<f32>,
    pub end_percent: Option<f32>,
    pub inpaint: Option<InpaintConditioning>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ControlAttachmentKind {
    ControlNet,
    Gligen,
    StyleModel,
    UnClip,
    IpAdapter,
    ReferenceImage,
    Pose,
    Depth,
    Segmentation,
    Camera,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlAttachment {
    pub kind: ControlAttachmentKind,
    pub source_ref: String,
    pub strength: f32,
    pub start_percent: Option<f32>,
    pub end_percent: Option<f32>,
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ConditioningTransformKind {
    Combine,
    Average,
    Concatenate,
    Multiply,
    Zero,
    SetArea,
    SetMask,
    SetRange,
    AttachInpaint,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConditioningTransform {
    pub kind: ConditioningTransformKind,
    pub source_node_id: String,
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConditioningBundle {
    pub id: String,
    pub encoder: EncoderIdentity,
    pub token_embeddings: TensorDescriptor,
    pub pooled_output: Option<TensorDescriptor>,
    pub attention_metadata: AttentionMetadata,
    pub source_prompts: Vec<PromptMetadata>,
    pub regions: Vec<ConditioningRegion>,
    pub control_attachments: Vec<ControlAttachment>,
    pub transforms: Vec<ConditioningTransform>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConditioningRuntimeContext {
    pub sampler: SamplerKind,
    pub guidance: GuidanceMode,
    pub latent_format: LatentFormat,
    pub backend: DeviceBackend,
    pub worker_supports_control_attachments: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConditioningValidationDiagnostic {
    pub code: String,
    pub node_id: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyConditioningRuntime;

impl ComfyConditioningRuntime {
    pub fn new() -> Self {
        Self
    }

    pub fn required_modes(&self, bundle: &ConditioningBundle) -> BTreeSet<ConditioningMode> {
        let mut modes = BTreeSet::from([ConditioningMode::Text]);

        if matches!(bundle.encoder.kind, EncoderKind::ClipVision) {
            modes.insert(ConditioningMode::Image);
        }
        if bundle.regions.iter().any(|region| region.mask.is_some()) {
            modes.insert(ConditioningMode::Mask);
        }
        if bundle.regions.iter().any(|region| region.inpaint.is_some()) {
            modes.insert(ConditioningMode::Image);
            modes.insert(ConditioningMode::Mask);
        }
        for attachment in &bundle.control_attachments {
            modes.extend(modes_for_attachment(attachment.kind));
        }
        modes
    }

    pub fn validate(
        &self,
        bundle: &ConditioningBundle,
        family_profile: &ModelFamilyExecutionProfile,
        context: &ConditioningRuntimeContext,
    ) -> Result<(), Vec<ConditioningValidationDiagnostic>> {
        let mut diagnostics = Vec::new();

        if bundle.source_prompts.is_empty()
            && bundle.control_attachments.is_empty()
            && bundle.regions.is_empty()
        {
            diagnostics.push(diagnostic(
                EMPTY_BUNDLE_CODE,
                None,
                "conditioning bundle must preserve at least one prompt, region, or control attachment",
            ));
        }
        validate_tensor(
            &bundle.token_embeddings,
            None,
            "token embeddings must include a non-empty tensor shape",
            &mut diagnostics,
        );
        if let Some(pooled_output) = &bundle.pooled_output {
            validate_tensor(
                pooled_output,
                None,
                "pooled output must include a non-empty tensor shape",
                &mut diagnostics,
            );
        }

        for prompt in &bundle.source_prompts {
            if prompt.text.trim().is_empty() {
                diagnostics.push(diagnostic(
                    EMPTY_BUNDLE_CODE,
                    Some(prompt.node_id.clone()),
                    "source prompt metadata cannot be empty",
                ));
            }
        }
        for region in &bundle.regions {
            validate_region(region, &mut diagnostics);
        }
        for attachment in &bundle.control_attachments {
            validate_attachment(attachment, &mut diagnostics);
        }

        for mode in self.required_modes(bundle) {
            if !supported_conditioning_modes(family_profile.family).contains(&mode) {
                diagnostics.push(diagnostic(
                    UNSUPPORTED_CONDITIONING_CODE,
                    None,
                    format!(
                        "conditioning mode {:?} is not supported by model family {:?}",
                        mode, family_profile.family
                    ),
                ));
            }
        }

        if family_profile.latent_format != context.latent_format {
            diagnostics.push(diagnostic(
                LATENT_MISMATCH_CODE,
                None,
                format!(
                    "conditioning latent format {:?} does not match model family latent format {:?}",
                    context.latent_format, family_profile.latent_format
                ),
            ));
        }
        if !family_profile.supported_samplers.contains(&context.sampler) {
            diagnostics.push(diagnostic(
                SAMPLER_MISMATCH_CODE,
                None,
                format!(
                    "conditioning context sampler {:?} is not supported by model family {:?}",
                    context.sampler, family_profile.family
                ),
            ));
        }
        if !family_profile
            .supported_guidance
            .contains(&context.guidance)
        {
            diagnostics.push(diagnostic(
                SAMPLER_MISMATCH_CODE,
                None,
                format!(
                    "conditioning context guidance {:?} is not supported by model family {:?}",
                    context.guidance, family_profile.family
                ),
            ));
        }
        if !context.worker_supports_control_attachments && !bundle.control_attachments.is_empty() {
            diagnostics.push(diagnostic(
                BACKEND_MISMATCH_CODE,
                None,
                "remote worker capabilities for control attachments must be negotiated before execution",
            ));
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }
}

fn validate_tensor(
    tensor: &TensorDescriptor,
    node_id: Option<String>,
    message: &str,
    diagnostics: &mut Vec<ConditioningValidationDiagnostic>,
) {
    if tensor.id.trim().is_empty() || tensor.shape.is_empty() || tensor.shape.contains(&0) {
        diagnostics.push(diagnostic(EMPTY_TENSOR_CODE, node_id, message));
    }
}

fn validate_region(
    region: &ConditioningRegion,
    diagnostics: &mut Vec<ConditioningValidationDiagnostic>,
) {
    if !(0.0..=1.0).contains(&region.strength) {
        diagnostics.push(diagnostic(
            INVALID_REGION_CODE,
            None,
            "conditioning region strength must be between 0.0 and 1.0",
        ));
    }
    if let (Some(start_percent), Some(end_percent)) = (region.start_percent, region.end_percent)
        && start_percent > end_percent
    {
        diagnostics.push(diagnostic(
            INVALID_REGION_CODE,
            None,
            "conditioning region start percent cannot be after end percent",
        ));
    }
    if let Some(area) = region.area
        && (area.width == 0 || area.height == 0)
    {
        diagnostics.push(diagnostic(
            INVALID_REGION_CODE,
            None,
            "conditioning area width and height must be greater than zero",
        ));
    }
    if let Some(mask) = &region.mask {
        validate_tensor(
            &mask.tensor,
            None,
            "conditioning mask must include a non-empty tensor shape",
            diagnostics,
        );
        if !(0.0..=1.0).contains(&mask.strength) {
            diagnostics.push(diagnostic(
                INVALID_REGION_CODE,
                None,
                "conditioning mask strength must be between 0.0 and 1.0",
            ));
        }
    }
}

fn validate_attachment(
    attachment: &ControlAttachment,
    diagnostics: &mut Vec<ConditioningValidationDiagnostic>,
) {
    if attachment.source_ref.trim().is_empty() {
        diagnostics.push(diagnostic(
            UNSUPPORTED_CONDITIONING_CODE,
            None,
            "control attachment source cannot be empty",
        ));
    }
    if !(0.0..=1.0).contains(&attachment.strength) {
        diagnostics.push(diagnostic(
            UNSUPPORTED_CONDITIONING_CODE,
            None,
            "control attachment strength must be between 0.0 and 1.0",
        ));
    }
    if let (Some(start_percent), Some(end_percent)) =
        (attachment.start_percent, attachment.end_percent)
        && start_percent > end_percent
    {
        diagnostics.push(diagnostic(
            UNSUPPORTED_CONDITIONING_CODE,
            None,
            "control attachment start percent cannot be after end percent",
        ));
    }
}

fn modes_for_attachment(kind: ControlAttachmentKind) -> BTreeSet<ConditioningMode> {
    match kind {
        ControlAttachmentKind::ControlNet => {
            BTreeSet::from([ConditioningMode::Control, ConditioningMode::Image])
        }
        ControlAttachmentKind::Gligen => {
            BTreeSet::from([ConditioningMode::Detection, ConditioningMode::Image])
        }
        ControlAttachmentKind::StyleModel => {
            BTreeSet::from([ConditioningMode::Style, ConditioningMode::Image])
        }
        ControlAttachmentKind::UnClip
        | ControlAttachmentKind::IpAdapter
        | ControlAttachmentKind::ReferenceImage
        | ControlAttachmentKind::Pose
        | ControlAttachmentKind::Camera => BTreeSet::from([ConditioningMode::Image]),
        ControlAttachmentKind::Depth => {
            BTreeSet::from([ConditioningMode::Depth, ConditioningMode::Image])
        }
        ControlAttachmentKind::Segmentation => {
            BTreeSet::from([ConditioningMode::Segmentation, ConditioningMode::Image])
        }
    }
}

fn supported_conditioning_modes(family: ModelFamilyKind) -> BTreeSet<ConditioningMode> {
    match family {
        ModelFamilyKind::StableDiffusion1
        | ModelFamilyKind::StableDiffusion2
        | ModelFamilyKind::StableDiffusionXl
        | ModelFamilyKind::StableDiffusion3
        | ModelFamilyKind::Flux => BTreeSet::from([
            ConditioningMode::Text,
            ConditioningMode::Image,
            ConditioningMode::Control,
            ConditioningMode::Style,
            ConditioningMode::Mask,
            ConditioningMode::Depth,
            ConditioningMode::Segmentation,
            ConditioningMode::Detection,
        ]),
        ModelFamilyKind::WanVideo | ModelFamilyKind::HunyuanVideo | ModelFamilyKind::LtxVideo => {
            BTreeSet::from([
                ConditioningMode::Text,
                ConditioningMode::Image,
                ConditioningMode::Video,
                ConditioningMode::Control,
                ConditioningMode::Depth,
                ConditioningMode::Mask,
            ])
        }
        ModelFamilyKind::Audio => BTreeSet::from([ConditioningMode::Text, ConditioningMode::Audio]),
        ModelFamilyKind::ThreeD => BTreeSet::from([
            ConditioningMode::Text,
            ConditioningMode::Image,
            ConditioningMode::Geometry,
        ]),
        ModelFamilyKind::Segmentation => {
            BTreeSet::from([ConditioningMode::Image, ConditioningMode::Segmentation])
        }
        ModelFamilyKind::Depth => {
            BTreeSet::from([ConditioningMode::Image, ConditioningMode::Depth])
        }
        ModelFamilyKind::Detection => {
            BTreeSet::from([ConditioningMode::Image, ConditioningMode::Detection])
        }
        ModelFamilyKind::Adapter => BTreeSet::from([
            ConditioningMode::Text,
            ConditioningMode::Image,
            ConditioningMode::Control,
            ConditioningMode::Style,
        ]),
    }
}

fn diagnostic(
    code: &str,
    node_id: Option<String>,
    message: impl Into<String>,
) -> ConditioningValidationDiagnostic {
    ConditioningValidationDiagnostic {
        code: code.to_string(),
        node_id,
        message: message.into(),
    }
}
