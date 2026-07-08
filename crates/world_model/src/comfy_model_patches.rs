use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    ModelCategory, ModelComponentSet, ModelFamilyExecutionProfile, ModelFamilyKind, ModelFileRef,
};

pub const PATCH_UNSUPPORTED_CODE: &str = "world_model.patches.unsupported";
pub const PATCH_CATEGORY_MISMATCH_CODE: &str = "world_model.patches.category_mismatch";
pub const PATCH_FAMILY_MISMATCH_CODE: &str = "world_model.patches.family_mismatch";
pub const PATCH_DUPLICATE_CODE: &str = "world_model.patches.duplicate";
pub const PATCH_STRENGTH_MISMATCH_CODE: &str = "world_model.patches.strength_mismatch";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ModelPatchKind {
    Lora,
    Hypernetwork,
    ControlNet,
    Gligen,
    ModelPatch,
    ModelMerge,
    EditModel,
}

impl ModelPatchKind {
    pub fn expected_category(self) -> ModelCategory {
        match self {
            Self::Lora => ModelCategory::Loras,
            Self::Hypernetwork => ModelCategory::Hypernetworks,
            Self::ControlNet => ModelCategory::ControlNet,
            Self::Gligen => ModelCategory::Gligen,
            Self::ModelPatch | Self::ModelMerge | Self::EditModel => ModelCategory::ModelPatches,
        }
    }

    fn priority(self) -> u8 {
        match self {
            Self::ModelMerge => 0,
            Self::EditModel => 1,
            Self::ModelPatch => 2,
            Self::Lora => 3,
            Self::Hypernetwork => 4,
            Self::ControlNet => 5,
            Self::Gligen => 6,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelPatchRecord {
    pub id: String,
    pub kind: ModelPatchKind,
    pub file: ModelFileRef,
    pub compatible_families: BTreeSet<ModelFamilyKind>,
    pub strength_model: f32,
    pub strength_clip: f32,
    pub order: u32,
    pub provenance: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppliedModelPatch {
    pub sequence: u32,
    pub record: ModelPatchRecord,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelPatchPlan {
    pub component_set: ModelComponentSet,
    pub patches: Vec<AppliedModelPatch>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelPatchDiagnostic {
    pub code: String,
    pub patch_id: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyModelPatchPipeline;

impl ComfyModelPatchPipeline {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(
        &self,
        component_set: ModelComponentSet,
        family_profile: &ModelFamilyExecutionProfile,
        mut patches: Vec<ModelPatchRecord>,
    ) -> Result<ModelPatchPlan, Vec<ModelPatchDiagnostic>> {
        let mut diagnostics = Vec::new();

        if !family_profile.supports_patches && !patches.is_empty() {
            diagnostics.push(diagnostic(
                PATCH_UNSUPPORTED_CODE,
                None,
                format!(
                    "model family {:?} does not support patches",
                    family_profile.family
                ),
            ));
        }

        let mut patch_ids = BTreeSet::new();
        for patch in &patches {
            if !patch_ids.insert(patch.id.clone()) {
                diagnostics.push(diagnostic(
                    PATCH_DUPLICATE_CODE,
                    Some(patch.id.clone()),
                    "patch ids must be unique within a model patch plan",
                ));
            }
            if patch.file.category != patch.kind.expected_category() {
                diagnostics.push(diagnostic(
                    PATCH_CATEGORY_MISMATCH_CODE,
                    Some(patch.id.clone()),
                    format!(
                        "patch kind {:?} expects category {:?}, got {:?}",
                        patch.kind,
                        patch.kind.expected_category(),
                        patch.file.category
                    ),
                ));
            }
            if !patch.compatible_families.is_empty()
                && !patch.compatible_families.contains(&family_profile.family)
            {
                diagnostics.push(diagnostic(
                    PATCH_FAMILY_MISMATCH_CODE,
                    Some(patch.id.clone()),
                    format!(
                        "patch is not compatible with model family {:?}",
                        family_profile.family
                    ),
                ));
            }
            if !(-10.0..=10.0).contains(&patch.strength_model)
                || !(-10.0..=10.0).contains(&patch.strength_clip)
            {
                diagnostics.push(diagnostic(
                    PATCH_STRENGTH_MISMATCH_CODE,
                    Some(patch.id.clone()),
                    "patch strengths must be within the supported Sim range",
                ));
            }
        }

        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }

        patches.sort_by(|left, right| {
            left.kind
                .priority()
                .cmp(&right.kind.priority())
                .then_with(|| left.order.cmp(&right.order))
                .then_with(|| left.id.cmp(&right.id))
        });

        Ok(ModelPatchPlan {
            component_set,
            patches: patches
                .into_iter()
                .enumerate()
                .map(|(index, record)| AppliedModelPatch {
                    sequence: index as u32,
                    record,
                })
                .collect(),
        })
    }
}

fn diagnostic(
    code: &str,
    patch_id: Option<String>,
    message: impl Into<String>,
) -> ModelPatchDiagnostic {
    ModelPatchDiagnostic {
        code: code.to_string(),
        patch_id,
        message: message.into(),
    }
}
