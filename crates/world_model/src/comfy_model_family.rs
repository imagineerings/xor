use std::{collections::BTreeSet, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{ModelCategory, ModelFileRef, SafetensorsHeaderMetadata};

pub const UNSUPPORTED_MODEL_FAMILY_CODE: &str = "world_model.model_family.unsupported";
pub const INCOMPATIBLE_ADAPTER_CODE: &str = "world_model.model_family.incompatible_adapter";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ModelFamilyKind {
    StableDiffusion1,
    StableDiffusion2,
    StableDiffusionXl,
    StableDiffusion3,
    Flux,
    WanVideo,
    HunyuanVideo,
    LtxVideo,
    Audio,
    ThreeD,
    Segmentation,
    Depth,
    Detection,
    Adapter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ModelMediaCapability {
    Image,
    Video,
    Audio,
    ThreeD,
    Adapter,
    Segmentation,
    Depth,
    Detection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum LatentFormat {
    StableDiffusion,
    StableDiffusionXl,
    StableDiffusion3,
    Flux,
    Video,
    Audio,
    Geometry,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum TextEncoderRequirement {
    Clip,
    OpenClip,
    T5,
    DualClip,
    TripleClip,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum VaeRequirement {
    Required,
    Optional,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ConditioningMode {
    Text,
    Image,
    Video,
    Audio,
    Control,
    Style,
    Mask,
    Depth,
    Segmentation,
    Detection,
    Geometry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum AdapterKind {
    Lora,
    ControlNet,
    StyleModel,
    Gligen,
    Hypernetwork,
    ModelPatch,
    Embedding,
    ClipVision,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelFamilyCapability {
    pub media: BTreeSet<ModelMediaCapability>,
    pub latent_format: LatentFormat,
    pub text_encoders: BTreeSet<TextEncoderRequirement>,
    pub vae: VaeRequirement,
    pub conditioning: BTreeSet<ConditioningMode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelFamilyProfile {
    pub category: ModelCategory,
    pub relative_path: PathBuf,
    pub family: ModelFamilyKind,
    pub adapter_kind: Option<AdapterKind>,
    pub capability: ModelFamilyCapability,
    pub compatible_base_families: BTreeSet<ModelFamilyKind>,
}

impl ModelFamilyProfile {
    pub fn supports_media(&self, capability: ModelMediaCapability) -> bool {
        self.capability.media.contains(&capability)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelFamilyDiagnostic {
    pub code: String,
    pub category: ModelCategory,
    pub relative_path: PathBuf,
    pub missing_capability: Option<ModelMediaCapability>,
    pub message: String,
}

impl ModelFamilyDiagnostic {
    fn unsupported(
        category: ModelCategory,
        relative_path: PathBuf,
        missing_capability: ModelMediaCapability,
    ) -> Self {
        Self {
            code: UNSUPPORTED_MODEL_FAMILY_CODE.to_string(),
            category,
            relative_path,
            missing_capability: Some(missing_capability),
            message: "model family is not supported by Sim's native world-model runtime"
                .to_string(),
        }
    }

    fn incompatible_adapter(adapter: &ModelFamilyProfile, base: &ModelFamilyProfile) -> Self {
        Self {
            code: INCOMPATIBLE_ADAPTER_CODE.to_string(),
            category: adapter.category,
            relative_path: adapter.relative_path.clone(),
            missing_capability: Some(ModelMediaCapability::Adapter),
            message: format!(
                "adapter {:?} is not compatible with base model family {:?}",
                adapter.adapter_kind, base.family
            ),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyModelFamilyDetector;

impl ComfyModelFamilyDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect(
        &self,
        file: &ModelFileRef,
        metadata: Option<&SafetensorsHeaderMetadata>,
    ) -> Result<ModelFamilyProfile, ModelFamilyDiagnostic> {
        if let Some(adapter_kind) = adapter_kind_for_category(file.category) {
            return Ok(adapter_profile(file, adapter_kind));
        }

        if let Some(profile) = category_profile(file) {
            return Ok(profile);
        }

        if matches!(
            file.category,
            ModelCategory::Checkpoints | ModelCategory::DiffusionModels | ModelCategory::Diffusers
        ) {
            return self.detect_base_model(file, metadata);
        }

        Err(ModelFamilyDiagnostic::unsupported(
            file.category,
            file.relative_path.clone(),
            ModelMediaCapability::Image,
        ))
    }

    pub fn validate_adapter_compatibility(
        &self,
        base: &ModelFamilyProfile,
        adapter: &ModelFamilyProfile,
    ) -> Result<(), ModelFamilyDiagnostic> {
        if adapter.adapter_kind.is_none() {
            return Err(ModelFamilyDiagnostic::incompatible_adapter(adapter, base));
        }

        if adapter.compatible_base_families.is_empty()
            || adapter.compatible_base_families.contains(&base.family)
        {
            return Ok(());
        }

        Err(ModelFamilyDiagnostic::incompatible_adapter(adapter, base))
    }

    fn detect_base_model(
        &self,
        file: &ModelFileRef,
        metadata: Option<&SafetensorsHeaderMetadata>,
    ) -> Result<ModelFamilyProfile, ModelFamilyDiagnostic> {
        let metadata_text = metadata
            .map(metadata_search_text)
            .unwrap_or_else(|| file.relative_path.to_string_lossy().to_ascii_lowercase());

        let family = if contains_any(&metadata_text, &["flux"]) {
            ModelFamilyKind::Flux
        } else if contains_any(&metadata_text, &["stable-diffusion-3", "sd3", "sd_3"]) {
            ModelFamilyKind::StableDiffusion3
        } else if contains_any(&metadata_text, &["stable-diffusion-xl", "sdxl", "sd_xl"]) {
            ModelFamilyKind::StableDiffusionXl
        } else if contains_any(
            &metadata_text,
            &["stable-diffusion-v2", "stable diffusion v2", "sd2"],
        ) {
            ModelFamilyKind::StableDiffusion2
        } else if contains_any(
            &metadata_text,
            &["stable-diffusion", "stable diffusion", "sd1"],
        ) {
            ModelFamilyKind::StableDiffusion1
        } else if contains_any(&metadata_text, &["wan", "wan2"]) {
            ModelFamilyKind::WanVideo
        } else if contains_any(&metadata_text, &["hunyuan"]) {
            ModelFamilyKind::HunyuanVideo
        } else if contains_any(&metadata_text, &["ltxv", "ltx-video", "ltx video"]) {
            ModelFamilyKind::LtxVideo
        } else {
            return Err(ModelFamilyDiagnostic::unsupported(
                file.category,
                file.relative_path.clone(),
                ModelMediaCapability::Image,
            ));
        };

        Ok(base_profile(file, family))
    }
}

fn metadata_search_text(metadata: &SafetensorsHeaderMetadata) -> String {
    metadata
        .metadata
        .iter()
        .flat_map(|(key, value)| [key.as_str(), value.as_str()])
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn adapter_kind_for_category(category: ModelCategory) -> Option<AdapterKind> {
    match category {
        ModelCategory::Loras => Some(AdapterKind::Lora),
        ModelCategory::ControlNet => Some(AdapterKind::ControlNet),
        ModelCategory::StyleModels => Some(AdapterKind::StyleModel),
        ModelCategory::Gligen => Some(AdapterKind::Gligen),
        ModelCategory::Hypernetworks => Some(AdapterKind::Hypernetwork),
        ModelCategory::ModelPatches => Some(AdapterKind::ModelPatch),
        ModelCategory::Embeddings => Some(AdapterKind::Embedding),
        ModelCategory::ClipVision => Some(AdapterKind::ClipVision),
        _ => None,
    }
}

fn adapter_profile(file: &ModelFileRef, adapter_kind: AdapterKind) -> ModelFamilyProfile {
    ModelFamilyProfile {
        category: file.category,
        relative_path: file.relative_path.clone(),
        family: ModelFamilyKind::Adapter,
        adapter_kind: Some(adapter_kind),
        capability: capability(
            [ModelMediaCapability::Adapter],
            LatentFormat::None,
            [TextEncoderRequirement::None],
            VaeRequirement::None,
            adapter_conditioning(adapter_kind),
        ),
        compatible_base_families: adapter_base_families(adapter_kind),
    }
}

fn category_profile(file: &ModelFileRef) -> Option<ModelFamilyProfile> {
    let family = match file.category {
        ModelCategory::AudioEncoders => ModelFamilyKind::Audio,
        ModelCategory::FrameInterpolation | ModelCategory::OpticalFlow => ModelFamilyKind::LtxVideo,
        ModelCategory::BackgroundRemoval => ModelFamilyKind::Segmentation,
        ModelCategory::GeometryEstimation => ModelFamilyKind::Depth,
        ModelCategory::Detection => ModelFamilyKind::Detection,
        _ => return None,
    };

    Some(base_profile(file, family))
}

fn base_profile(file: &ModelFileRef, family: ModelFamilyKind) -> ModelFamilyProfile {
    ModelFamilyProfile {
        category: file.category,
        relative_path: file.relative_path.clone(),
        family,
        adapter_kind: None,
        capability: base_capability(family),
        compatible_base_families: BTreeSet::new(),
    }
}

fn base_capability(family: ModelFamilyKind) -> ModelFamilyCapability {
    match family {
        ModelFamilyKind::StableDiffusion1 | ModelFamilyKind::StableDiffusion2 => capability(
            [ModelMediaCapability::Image],
            LatentFormat::StableDiffusion,
            [TextEncoderRequirement::Clip],
            VaeRequirement::Required,
            [ConditioningMode::Text, ConditioningMode::Image],
        ),
        ModelFamilyKind::StableDiffusionXl => capability(
            [ModelMediaCapability::Image],
            LatentFormat::StableDiffusionXl,
            [
                TextEncoderRequirement::Clip,
                TextEncoderRequirement::OpenClip,
            ],
            VaeRequirement::Required,
            [ConditioningMode::Text, ConditioningMode::Image],
        ),
        ModelFamilyKind::StableDiffusion3 => capability(
            [ModelMediaCapability::Image],
            LatentFormat::StableDiffusion3,
            [
                TextEncoderRequirement::TripleClip,
                TextEncoderRequirement::T5,
            ],
            VaeRequirement::Required,
            [ConditioningMode::Text, ConditioningMode::Image],
        ),
        ModelFamilyKind::Flux => capability(
            [ModelMediaCapability::Image],
            LatentFormat::Flux,
            [TextEncoderRequirement::DualClip, TextEncoderRequirement::T5],
            VaeRequirement::Required,
            [ConditioningMode::Text, ConditioningMode::Image],
        ),
        ModelFamilyKind::WanVideo | ModelFamilyKind::HunyuanVideo | ModelFamilyKind::LtxVideo => {
            capability(
                [ModelMediaCapability::Video],
                LatentFormat::Video,
                [TextEncoderRequirement::T5],
                VaeRequirement::Required,
                [
                    ConditioningMode::Text,
                    ConditioningMode::Image,
                    ConditioningMode::Video,
                ],
            )
        }
        ModelFamilyKind::Audio => capability(
            [ModelMediaCapability::Audio],
            LatentFormat::Audio,
            [TextEncoderRequirement::None],
            VaeRequirement::None,
            [ConditioningMode::Audio],
        ),
        ModelFamilyKind::ThreeD => capability(
            [ModelMediaCapability::ThreeD],
            LatentFormat::Geometry,
            [TextEncoderRequirement::None],
            VaeRequirement::Optional,
            [ConditioningMode::Geometry],
        ),
        ModelFamilyKind::Segmentation => capability(
            [ModelMediaCapability::Segmentation],
            LatentFormat::None,
            [TextEncoderRequirement::None],
            VaeRequirement::None,
            [ConditioningMode::Image, ConditioningMode::Segmentation],
        ),
        ModelFamilyKind::Depth => capability(
            [ModelMediaCapability::Depth, ModelMediaCapability::ThreeD],
            LatentFormat::Geometry,
            [TextEncoderRequirement::None],
            VaeRequirement::None,
            [
                ConditioningMode::Image,
                ConditioningMode::Depth,
                ConditioningMode::Geometry,
            ],
        ),
        ModelFamilyKind::Detection => capability(
            [ModelMediaCapability::Detection],
            LatentFormat::None,
            [TextEncoderRequirement::None],
            VaeRequirement::None,
            [ConditioningMode::Image, ConditioningMode::Detection],
        ),
        ModelFamilyKind::Adapter => capability(
            [ModelMediaCapability::Adapter],
            LatentFormat::None,
            [TextEncoderRequirement::None],
            VaeRequirement::None,
            [ConditioningMode::Control],
        ),
    }
}

fn adapter_conditioning(adapter_kind: AdapterKind) -> impl IntoIterator<Item = ConditioningMode> {
    match adapter_kind {
        AdapterKind::Lora | AdapterKind::Hypernetwork | AdapterKind::ModelPatch => {
            vec![ConditioningMode::Text, ConditioningMode::Image]
        }
        AdapterKind::ControlNet => vec![ConditioningMode::Control, ConditioningMode::Image],
        AdapterKind::StyleModel | AdapterKind::ClipVision => {
            vec![ConditioningMode::Style, ConditioningMode::Image]
        }
        AdapterKind::Gligen => vec![ConditioningMode::Detection, ConditioningMode::Image],
        AdapterKind::Embedding => vec![ConditioningMode::Text],
    }
}

fn adapter_base_families(adapter_kind: AdapterKind) -> BTreeSet<ModelFamilyKind> {
    match adapter_kind {
        AdapterKind::Lora | AdapterKind::Hypernetwork | AdapterKind::ModelPatch => {
            image_base_families()
        }
        AdapterKind::ControlNet => [
            ModelFamilyKind::StableDiffusion1,
            ModelFamilyKind::StableDiffusion2,
            ModelFamilyKind::StableDiffusionXl,
            ModelFamilyKind::StableDiffusion3,
        ]
        .into_iter()
        .collect(),
        AdapterKind::StyleModel
        | AdapterKind::ClipVision
        | AdapterKind::Gligen
        | AdapterKind::Embedding => image_base_families(),
    }
}

fn image_base_families() -> BTreeSet<ModelFamilyKind> {
    [
        ModelFamilyKind::StableDiffusion1,
        ModelFamilyKind::StableDiffusion2,
        ModelFamilyKind::StableDiffusionXl,
        ModelFamilyKind::StableDiffusion3,
        ModelFamilyKind::Flux,
    ]
    .into_iter()
    .collect()
}

fn capability(
    media: impl IntoIterator<Item = ModelMediaCapability>,
    latent_format: LatentFormat,
    text_encoders: impl IntoIterator<Item = TextEncoderRequirement>,
    vae: VaeRequirement,
    conditioning: impl IntoIterator<Item = ConditioningMode>,
) -> ModelFamilyCapability {
    ModelFamilyCapability {
        media: media.into_iter().collect(),
        latent_format,
        text_encoders: text_encoders.into_iter().collect(),
        vae,
        conditioning: conditioning.into_iter().collect(),
    }
}
