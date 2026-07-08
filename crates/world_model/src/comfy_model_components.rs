use serde::{Deserialize, Serialize};

use crate::{
    LatentFormat, ModelCategory, ModelFamilyExecutionProfile, ModelFamilyKind, ModelFileRef,
};

pub const COMPONENT_MISSING_BASE_CODE: &str = "world_model.components.missing_base";
pub const COMPONENT_CATEGORY_MISMATCH_CODE: &str = "world_model.components.category_mismatch";
pub const COMPONENT_FAMILY_MISMATCH_CODE: &str = "world_model.components.family_mismatch";
pub const COMPONENT_LATENT_MISMATCH_CODE: &str = "world_model.components.latent_mismatch";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ModelComponentRole {
    Checkpoint,
    DiffusionModel,
    Clip,
    Vae,
    Unet,
    TextEncoder,
    Diffusers,
}

impl ModelComponentRole {
    pub fn expected_category(self) -> ModelCategory {
        match self {
            Self::Checkpoint => ModelCategory::Checkpoints,
            Self::DiffusionModel | Self::Unet => ModelCategory::DiffusionModels,
            Self::Clip | Self::TextEncoder => ModelCategory::TextEncoders,
            Self::Vae => ModelCategory::Vae,
            Self::Diffusers => ModelCategory::Diffusers,
        }
    }

    pub fn is_base_model(self) -> bool {
        matches!(
            self,
            Self::Checkpoint | Self::DiffusionModel | Self::Diffusers
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelComponent {
    pub role: ModelComponentRole,
    pub file: ModelFileRef,
    pub family: ModelFamilyKind,
    pub latent_format: LatentFormat,
    pub provenance: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelComponentSet {
    pub id: String,
    pub family: ModelFamilyKind,
    pub latent_format: LatentFormat,
    pub components: Vec<ModelComponent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelComponentDiagnostic {
    pub code: String,
    pub role: Option<ModelComponentRole>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyModelComponentComposer;

impl ComfyModelComponentComposer {
    pub fn new() -> Self {
        Self
    }

    pub fn compose(
        &self,
        id: impl Into<String>,
        family_profile: &ModelFamilyExecutionProfile,
        components: Vec<ModelComponent>,
    ) -> Result<ModelComponentSet, Vec<ModelComponentDiagnostic>> {
        let mut diagnostics = Vec::new();

        if !components
            .iter()
            .any(|component| component.role.is_base_model())
        {
            diagnostics.push(diagnostic(
                COMPONENT_MISSING_BASE_CODE,
                None,
                "model component set requires a checkpoint, diffusion model, or Diffusers base",
            ));
        }

        for component in &components {
            if component.file.category != component.role.expected_category() {
                diagnostics.push(diagnostic(
                    COMPONENT_CATEGORY_MISMATCH_CODE,
                    Some(component.role),
                    format!(
                        "component role {:?} expects category {:?}, got {:?}",
                        component.role,
                        component.role.expected_category(),
                        component.file.category
                    ),
                ));
            }
            if component.family != family_profile.family {
                diagnostics.push(diagnostic(
                    COMPONENT_FAMILY_MISMATCH_CODE,
                    Some(component.role),
                    format!(
                        "component family {:?} does not match model family {:?}",
                        component.family, family_profile.family
                    ),
                ));
            }
            if component.latent_format != family_profile.latent_format {
                diagnostics.push(diagnostic(
                    COMPONENT_LATENT_MISMATCH_CODE,
                    Some(component.role),
                    format!(
                        "component latent format {:?} does not match model family latent format {:?}",
                        component.latent_format, family_profile.latent_format
                    ),
                ));
            }
        }

        if diagnostics.is_empty() {
            Ok(ModelComponentSet {
                id: id.into(),
                family: family_profile.family,
                latent_format: family_profile.latent_format,
                components,
            })
        } else {
            Err(diagnostics)
        }
    }
}

fn diagnostic(
    code: &str,
    role: Option<ModelComponentRole>,
    message: impl Into<String>,
) -> ModelComponentDiagnostic {
    ModelComponentDiagnostic {
        code: code.to_string(),
        role,
        message: message.into(),
    }
}
