use crate::{
    AdapterFamily, AdapterTensor, PatchComputeBoundary, PatchGraph, PatchGraphError, PatchPayload,
    PatchSlice, PatchValueTransform, QuantLinearScale, QuantizationError, QuantizedLinearMatrix,
    QuantizedMatrix, SemanticPatchOperation, WeightAdapterError, WeightAdapterLoadRequest,
    WeightAdapterRegistry, quantize_linear_matrix, quantize_matrix,
};
use comfy_tensor::{DType, ExecutionContext, TensorBackend, TensorDescriptor};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const MAX_PATCH_KEY_BYTES: usize = 64 * 1024;
const PREFETCH_ALIGNMENT: u64 = 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchKeyMapping {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub slices: Vec<PatchSlice>,
}

impl PatchKeyMapping {
    pub fn direct(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            slices: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatchKeyMap {
    mappings: Vec<PatchKeyMapping>,
    by_source: BTreeMap<String, usize>,
}

impl PatchKeyMap {
    pub fn checked(mappings: Vec<PatchKeyMapping>) -> Result<Self, PatchAdapterError> {
        let mut map = Self {
            mappings: Vec::new(),
            by_source: BTreeMap::new(),
        };
        for mapping in mappings {
            map.insert(mapping)?;
        }
        Ok(map)
    }

    fn insert(&mut self, mapping: PatchKeyMapping) -> Result<(), PatchAdapterError> {
        validate_key(&mapping.source)?;
        validate_key(&mapping.target)?;
        validate_slices(&mapping.slices)?;
        if let Some(index) = self.by_source.get(&mapping.source).copied() {
            let current = self
                .mappings
                .get(index)
                .ok_or(PatchAdapterError::MappingIndexCorrupt)?;
            if current.target == mapping.target && current.slices == mapping.slices {
                return Ok(());
            }
            return Err(PatchAdapterError::ConflictingSource(mapping.source));
        }
        let index = self.mappings.len();
        self.by_source.insert(mapping.source.clone(), index);
        self.mappings.push(mapping);
        Ok(())
    }

    pub fn binding_for(&self, source: &str) -> Option<&PatchKeyMapping> {
        self.by_source
            .get(source)
            .and_then(|index| self.mappings.get(*index))
    }

    pub fn mappings(&self) -> &[PatchKeyMapping] {
        &self.mappings
    }

    pub fn target_for(&self, source: &str) -> Option<&str> {
        self.binding_for(source)
            .map(|mapping| mapping.target.as_str())
    }

    pub fn translate(
        &self,
        mut operation: SemanticPatchOperation,
    ) -> Result<SemanticPatchOperation, PatchAdapterError> {
        let mapping = self
            .binding_for(&operation.target_key)
            .ok_or_else(|| PatchAdapterError::UnmappedKey(operation.target_key.clone()))?;
        operation.target_key.clone_from(&mapping.target);
        operation.slices.clone_from(&mapping.slices);
        Ok(operation)
    }
}

pub fn model_lora_keys_clip(
    state_keys: impl IntoIterator<Item = String>,
    seeded: Vec<PatchKeyMapping>,
) -> Result<PatchKeyMap, PatchAdapterError> {
    const COMPONENTS: [(&str, &str); 6] = [
        ("mlp.fc1", "mlp_fc1"),
        ("mlp.fc2", "mlp_fc2"),
        ("self_attn.k_proj", "self_attn_k_proj"),
        ("self_attn.q_proj", "self_attn_q_proj"),
        ("self_attn.v_proj", "self_attn_v_proj"),
        ("self_attn.out_proj", "self_attn_out_proj"),
    ];
    let keys: Vec<String> = state_keys.into_iter().collect();
    let key_set: BTreeSet<&str> = keys.iter().map(String::as_str).collect();
    let mut map = PatchKeyMap::checked(seeded)?;
    let mut prefix_set = BTreeSet::new();
    for key in &keys {
        validate_key(key)?;
        let Some(without_weight) = key.strip_suffix(".weight") else {
            continue;
        };
        map.insert(PatchKeyMapping::direct(
            format!("text_encoders.{without_weight}"),
            key.clone(),
        ))?;
        if !key.starts_with("clip_")
            && let Some(transformer) = key.find(".transformer.")
            && transformer > 0
        {
            map.insert(PatchKeyMapping::direct(
                format!("text_encoders.{}", &key[transformer + 1..key.len() - 7]),
                key.clone(),
            ))?;
        }
        if let Some(prefix) = key.split('.').next() {
            prefix_set.insert(prefix.to_owned());
        }
    }

    let mut clip_l_present = false;
    let mut clip_g_present = false;
    for block in 0..32 {
        for (component, alias_component) in COMPONENTS {
            let clip_h =
                format!("clip_h.transformer.text_model.encoder.layers.{block}.{component}.weight");
            if key_set.contains(clip_h.as_str()) {
                for alias in [
                    format!("lora_te_text_model_encoder_layers_{block}_{alias_component}"),
                    format!("lora_te1_text_model_encoder_layers_{block}_{alias_component}"),
                    format!("text_encoder.text_model.encoder.layers.{block}.{component}"),
                ] {
                    map.insert(PatchKeyMapping::direct(alias, clip_h.clone()))?;
                }
            }
            let clip_l =
                format!("clip_l.transformer.text_model.encoder.layers.{block}.{component}.weight");
            if key_set.contains(clip_l.as_str()) {
                for alias in [
                    format!("lora_te_text_model_encoder_layers_{block}_{alias_component}"),
                    format!("lora_te1_text_model_encoder_layers_{block}_{alias_component}"),
                    format!("text_encoder.text_model.encoder.layers.{block}.{component}"),
                ] {
                    map.insert(PatchKeyMapping::direct(alias, clip_l.clone()))?;
                }
                clip_l_present = true;
            }
            let clip_g =
                format!("clip_g.transformer.text_model.encoder.layers.{block}.{component}.weight");
            if key_set.contains(clip_g.as_str()) {
                clip_g_present = true;
                let aliases = if clip_l_present {
                    vec![
                        format!("lora_te2_text_model_encoder_layers_{block}_{alias_component}"),
                        format!("text_encoder_2.text_model.encoder.layers.{block}.{component}"),
                    ]
                } else {
                    vec![
                        format!("lora_te_text_model_encoder_layers_{block}_{alias_component}"),
                        format!("text_encoder.text_model.encoder.layers.{block}.{component}"),
                        format!(
                            "lora_prior_te_text_model_encoder_layers_{block}_{alias_component}"
                        ),
                    ]
                };
                for alias in aliases {
                    map.insert(PatchKeyMapping::direct(alias, clip_g.clone()))?;
                }
            }
        }
    }

    for key in &keys {
        let Some(without_weight) = key.strip_suffix(".weight") else {
            continue;
        };
        if let Some(t5) = without_weight.strip_prefix("t5xxl.transformer.") {
            let component = t5.replace('.', "_");
            let mut index = 1;
            if clip_g_present {
                index += 1;
            }
            if clip_l_present {
                index += 1;
                if index == 2 {
                    map.insert(PatchKeyMapping::direct(
                        format!("lora_te{index}_{component}"),
                        key.clone(),
                    ))?;
                    index += 1;
                }
            }
            map.insert(PatchKeyMapping::direct(
                format!("lora_te{index}_{component}"),
                key.clone(),
            ))?;
        } else if let Some(bert) = without_weight.strip_prefix("hydit_clip.transformer.bert.") {
            map.insert(PatchKeyMapping::direct(
                format!("lora_te1_{}", bert.replace('.', "_")),
                key.clone(),
            ))?;
        }
    }
    if prefix_set.len() == 1
        && let Some(prefix) = prefix_set.first()
    {
        let full_prefix = format!("{prefix}.transformer.model.");
        for key in &keys {
            if let Some(value) = key
                .strip_prefix(&full_prefix)
                .and_then(|value| value.strip_suffix(".weight"))
            {
                map.insert(PatchKeyMapping::direct(
                    format!("lora_te_{}", value.replace('.', "_")),
                    key.clone(),
                ))?;
            }
        }
    }
    if key_set.contains("clip_g.transformer.text_projection.weight") {
        for alias in ["lora_prior_te_text_projection", "lora_te2_text_projection"] {
            map.insert(PatchKeyMapping::direct(
                alias,
                "clip_g.transformer.text_projection.weight",
            ))?;
        }
    }
    if key_set.contains("clip_l.transformer.text_projection.weight") {
        map.insert(PatchKeyMapping::direct(
            "lora_te1_text_projection",
            "clip_l.transformer.text_projection.weight",
        ))?;
    }
    Ok(map)
}

pub fn model_lora_keys_unet(
    state_keys: impl IntoIterator<Item = String>,
    canonical_diffusers_mappings: Vec<PatchKeyMapping>,
    family_aliases: Vec<PatchKeyMapping>,
    seeded: Vec<PatchKeyMapping>,
) -> Result<PatchKeyMap, PatchAdapterError> {
    let mut map = PatchKeyMap::checked(seeded)?;
    for key in state_keys {
        validate_key(&key)?;
        let Some(native) = key.strip_prefix("diffusion_model.") else {
            continue;
        };
        if let Some(without_weight) = native.strip_suffix(".weight") {
            map.insert(PatchKeyMapping::direct(
                format!("lora_unet_{}", without_weight.replace('.', "_")),
                key.clone(),
            ))?;
            map.insert(PatchKeyMapping::direct(
                format!("diffusion_model.{without_weight}"),
                key,
            ))?;
        } else {
            map.insert(PatchKeyMapping::direct(key.clone(), key))?;
        }
    }
    for mapping in canonical_diffusers_mappings {
        let Some(without_weight) = mapping.source.strip_suffix(".weight") else {
            continue;
        };
        let flattened = without_weight.replace('.', "_");
        for alias in [
            format!("lora_unet_{flattened}"),
            format!("lycoris_{flattened}"),
        ] {
            map.insert(PatchKeyMapping {
                source: alias,
                target: mapping.target.clone(),
                slices: mapping.slices.clone(),
            })?;
        }
        let mut diffusers = without_weight.replace(".to_", ".processor.to_");
        if diffusers.ends_with(".to_out.0") {
            diffusers.truncate(diffusers.len() - 2);
        }
        for alias in [diffusers.clone(), format!("unet.{diffusers}")] {
            map.insert(PatchKeyMapping {
                source: alias,
                target: mapping.target.clone(),
                slices: mapping.slices.clone(),
            })?;
        }
    }
    for alias in family_aliases {
        map.insert(alias)?;
    }
    Ok(map)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PatchMergeMode {
    Interpolate { ratio: f32 },
    Add,
    Subtract { multiplier: f32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchMergeRatio {
    pub prefix: String,
    pub ratio: f32,
}

impl PatchMergeRatio {
    pub fn checked(prefix: impl Into<String>, ratio: f32) -> Result<Self, PatchAdapterError> {
        let prefix = prefix.into();
        validate_key(&prefix)?;
        validate_ratio(ratio)?;
        Ok(Self { prefix, ratio })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchMergeKind {
    Model,
    Clip,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchMergeRule {
    pub kind: PatchMergeKind,
    pub source_prefix: String,
    pub target_prefix: String,
    pub mode: PatchMergeMode,
    pub block_ratios: Vec<PatchMergeRatio>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatchMergeMapping {
    pub target_key: String,
    pub patch_strength: f32,
    pub model_strength: f32,
}

impl PatchMergeRule {
    pub fn model_simple(ratio: f32) -> Result<Self, PatchAdapterError> {
        Self::checked(
            PatchMergeKind::Model,
            "diffusion_model.",
            "diffusion_model.",
            PatchMergeMode::Interpolate { ratio },
            Vec::new(),
        )
    }

    pub fn model_add() -> Result<Self, PatchAdapterError> {
        Self::checked(
            PatchMergeKind::Model,
            "diffusion_model.",
            "diffusion_model.",
            PatchMergeMode::Add,
            Vec::new(),
        )
    }

    pub fn model_subtract(multiplier: f32) -> Result<Self, PatchAdapterError> {
        Self::checked(
            PatchMergeKind::Model,
            "diffusion_model.",
            "diffusion_model.",
            PatchMergeMode::Subtract { multiplier },
            Vec::new(),
        )
    }

    pub fn clip_simple(ratio: f32) -> Result<Self, PatchAdapterError> {
        Self::checked(
            PatchMergeKind::Clip,
            "",
            "",
            PatchMergeMode::Interpolate { ratio },
            Vec::new(),
        )
    }

    pub fn clip_add() -> Result<Self, PatchAdapterError> {
        Self::checked(
            PatchMergeKind::Clip,
            "",
            "",
            PatchMergeMode::Add,
            Vec::new(),
        )
    }

    pub fn clip_subtract(multiplier: f32) -> Result<Self, PatchAdapterError> {
        Self::checked(
            PatchMergeKind::Clip,
            "",
            "",
            PatchMergeMode::Subtract { multiplier },
            Vec::new(),
        )
    }

    pub fn model_blocks(
        default_ratio: f32,
        block_ratios: Vec<PatchMergeRatio>,
    ) -> Result<Self, PatchAdapterError> {
        Self::checked(
            PatchMergeKind::Model,
            "diffusion_model.",
            "diffusion_model.",
            PatchMergeMode::Interpolate {
                ratio: default_ratio,
            },
            block_ratios,
        )
    }

    pub fn checked(
        kind: PatchMergeKind,
        source_prefix: impl Into<String>,
        target_prefix: impl Into<String>,
        mode: PatchMergeMode,
        block_ratios: Vec<PatchMergeRatio>,
    ) -> Result<Self, PatchAdapterError> {
        let source_prefix = source_prefix.into();
        let target_prefix = target_prefix.into();
        validate_optional_prefix(&source_prefix)?;
        validate_optional_prefix(&target_prefix)?;
        validate_merge_mode(&mode)?;
        let mut prefixes = BTreeSet::new();
        for override_ratio in &block_ratios {
            validate_key(&override_ratio.prefix)?;
            validate_ratio(override_ratio.ratio)?;
            if !prefixes.insert(override_ratio.prefix.clone()) {
                return Err(PatchAdapterError::DuplicateMergePrefix(
                    override_ratio.prefix.clone(),
                ));
            }
        }
        if kind == PatchMergeKind::Clip && !block_ratios.is_empty() {
            return Err(PatchAdapterError::MalformedMergeRule(
                "CLIP merges do not accept model block ratios".into(),
            ));
        }
        if !block_ratios.is_empty() && !matches!(mode, PatchMergeMode::Interpolate { .. }) {
            return Err(PatchAdapterError::MalformedMergeRule(
                "block ratios require interpolation mode".into(),
            ));
        }
        Ok(Self {
            kind,
            source_prefix,
            target_prefix,
            mode,
            block_ratios,
        })
    }

    pub fn map(&self, source: &str) -> Result<Option<PatchMergeMapping>, PatchAdapterError> {
        validate_key(source)?;
        let Some(suffix) = source.strip_prefix(&self.source_prefix) else {
            return Ok(None);
        };
        if self.kind == PatchMergeKind::Clip && is_excluded_clip_merge_key(source) {
            return Ok(None);
        }
        let target_key = format!("{}{}", self.target_prefix, suffix);
        validate_key(&target_key)?;
        let (patch_strength, model_strength) = match self.mode {
            PatchMergeMode::Interpolate { ratio } => {
                let ratio = self
                    .block_ratios
                    .iter()
                    .filter(|override_ratio| suffix.starts_with(&override_ratio.prefix))
                    .max_by_key(|override_ratio| override_ratio.prefix.len())
                    .map_or(ratio, |override_ratio| override_ratio.ratio);
                (1.0 - ratio, ratio)
            }
            PatchMergeMode::Add => (1.0, 1.0),
            PatchMergeMode::Subtract { multiplier } => (-multiplier, multiplier),
        };
        Ok(Some(PatchMergeMapping {
            target_key,
            patch_strength,
            model_strength,
        }))
    }
}

#[derive(Clone, Debug)]
pub struct PatchLoadReport {
    pub graph: PatchGraph,
    pub loaded_keys: BTreeSet<String>,
    pub unused_keys: BTreeSet<String>,
    pub loaded_families: BTreeMap<String, AdapterFamily>,
}

#[allow(clippy::too_many_arguments)]
pub fn load_lora_patch_graph(
    base_artifact_digest: impl Into<String>,
    key_map: &PatchKeyMap,
    tensors: &BTreeMap<String, AdapterTensor>,
    target_shapes: &BTreeMap<String, Vec<u64>>,
    strength: f32,
    strength_model: f32,
    backend: &dyn TensorBackend,
    context: &ExecutionContext<'_>,
) -> Result<PatchLoadReport, PatchAdapterError> {
    context.check()?;
    validate_finite("patch strength", strength)?;
    validate_finite("model strength", strength_model)?;
    let mut operations = Vec::new();
    let mut operation_targets = BTreeSet::new();
    let mut loaded_keys = BTreeSet::new();
    let mut loaded_families = BTreeMap::new();
    for mapping in key_map.mappings() {
        context.check()?;
        let prefix = &mapping.source;
        let alpha_key = format!("{prefix}.alpha");
        let alpha = tensors
            .get(&alpha_key)
            .map(|tensor| tensor.scalar_f32(backend, context))
            .transpose()?;
        if alpha.is_some() {
            loaded_keys.insert(alpha_key);
        }
        let dora_key = format!("{prefix}.dora_scale");
        let dora_scale = tensors.get(&dora_key).cloned();
        if dora_scale.is_some() {
            loaded_keys.insert(dora_key);
        }
        let request = WeightAdapterLoadRequest {
            prefix: prefix.clone(),
            tensors: tensors.clone(),
            alpha,
            dora_scale,
        };
        let loaded = WeightAdapterRegistry.load_unique(&request)?;
        let mut payload = match loaded {
            Some(loaded) => {
                loaded_keys.extend(loaded.loaded_keys().iter().cloned());
                loaded_families.insert(prefix.clone(), loaded.adapter().family());
                Some(loaded.adapter().to_patch_payload(backend, context)?)
            }
            None => None,
        };
        let mut bias_payload = None;
        if let Some(tensor) = tensors.get(&format!("{prefix}.w_norm")) {
            loaded_keys.insert(format!("{prefix}.w_norm"));
            payload = Some(PatchPayload::DenseDiff {
                tensor: tensor.to_patch_tensor(backend, context)?,
                pad_weight: false,
            });
            if let Some(bias) = tensors.get(&format!("{prefix}.b_norm")) {
                loaded_keys.insert(format!("{prefix}.b_norm"));
                bias_payload = Some(bias);
            }
        }
        if let Some(tensor) = tensors.get(&format!("{prefix}.diff")) {
            loaded_keys.insert(format!("{prefix}.diff"));
            payload = Some(PatchPayload::DenseDiff {
                tensor: tensor.to_patch_tensor(backend, context)?,
                pad_weight: false,
            });
        }
        if let Some(bias) = tensors.get(&format!("{prefix}.diff_b")) {
            loaded_keys.insert(format!("{prefix}.diff_b"));
            bias_payload = Some(bias);
        }
        if let Some(tensor) = tensors.get(&format!("{prefix}.set_weight")) {
            loaded_keys.insert(format!("{prefix}.set_weight"));
            payload = Some(PatchPayload::Set {
                tensor: tensor.to_patch_tensor(backend, context)?,
            });
        }
        if let Some(payload) = payload {
            let expected_shape = target_shapes
                .get(&mapping.target)
                .cloned()
                .ok_or_else(|| PatchAdapterError::MissingTargetShape(mapping.target.clone()))?;
            insert_operation(
                &mut operations,
                &mut operation_targets,
                SemanticPatchOperation {
                    identifier: format!("lora:{prefix}"),
                    target_key: mapping.target.clone(),
                    expected_shape,
                    strength,
                    strength_model,
                    slices: mapping.slices.clone(),
                    transform: PatchValueTransform::default(),
                    payload,
                },
            )?;
        }
        if let Some(bias) = bias_payload {
            insert_operation(
                &mut operations,
                &mut operation_targets,
                operation_for_bias(
                    prefix,
                    &mapping.target,
                    bias,
                    target_shapes,
                    strength,
                    strength_model,
                    backend,
                    context,
                )?,
            )?;
        }
    }
    context.check()?;
    let unused_keys = tensors
        .keys()
        .filter(|key| !loaded_keys.contains(*key))
        .cloned()
        .collect();
    let graph = PatchGraph::checked_semantic(base_artifact_digest, operations)?;
    Ok(PatchLoadReport {
        graph,
        loaded_keys,
        unused_keys,
        loaded_families,
    })
}

pub fn add_patches(
    graph: Option<&PatchGraph>,
    base_artifact_digest: impl Into<String>,
    available_keys: &BTreeSet<String>,
    operations: Vec<SemanticPatchOperation>,
) -> Result<(PatchGraph, BTreeSet<String>), PatchAdapterError> {
    let base_artifact_digest = base_artifact_digest.into();
    let mut accepted = Vec::new();
    let mut accepted_keys = BTreeSet::new();
    for operation in operations {
        if available_keys.contains(&operation.target_key) {
            accepted_keys.insert(operation.target_key.clone());
            accepted.push(operation);
        }
    }
    let graph = match graph {
        Some(graph) => {
            if graph.identity().base_artifact_digest != base_artifact_digest {
                return Err(PatchAdapterError::BaseDigestMismatch);
            }
            graph.append_semantic(accepted)?
        }
        None => PatchGraph::checked_semantic(base_artifact_digest, accepted)?,
    };
    Ok((graph, accepted_keys))
}

#[derive(Clone, Debug)]
pub enum QuantizedPatchValue {
    Matrix(QuantizedMatrix),
    Linear(QuantizedLinearMatrix),
}

#[derive(Clone, Debug)]
pub enum PatchableWeight {
    Dense(comfy_tensor::Tensor),
    Quantized(QuantizedPatchValue),
}

#[derive(Clone, Debug)]
pub struct PatchKeyPatches {
    key: String,
    base: PatchableWeight,
    graph: PatchGraph,
}

impl PatchKeyPatches {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn base(&self) -> &PatchableWeight {
        &self.base
    }

    pub fn operations(&self) -> Vec<&SemanticPatchOperation> {
        self.graph.operations_for_key(&self.key)
    }
}

pub fn get_key_patches(
    graph: &PatchGraph,
    key: &str,
    base: &PatchableWeight,
) -> Result<PatchKeyPatches, PatchAdapterError> {
    validate_key(key)?;
    Ok(PatchKeyPatches {
        key: key.to_owned(),
        base: base.clone(),
        graph: graph.clone(),
    })
}

pub fn merge_key_patches(
    rule: &PatchMergeRule,
    source: &PatchKeyPatches,
    target_expected_shape: Vec<u64>,
    compute_boundary: PatchComputeBoundary,
    backend: &dyn TensorBackend,
    context: &ExecutionContext<'_>,
) -> Result<Option<SemanticPatchOperation>, PatchAdapterError> {
    context.check()?;
    let Some(mapping) = rule.map(source.key())? else {
        return Ok(None);
    };
    let patched = patch_weight_to_device(
        &source.graph,
        source.key(),
        source.base(),
        false,
        compute_boundary,
        backend,
        context,
    )?;
    let base = patchable_to_patch_tensor(&patched, backend, context)?;
    if base.shape != target_expected_shape {
        return Err(PatchAdapterError::MergeShape {
            expected: target_expected_shape,
            actual: base.shape,
        });
    }
    Ok(Some(SemanticPatchOperation {
        identifier: format!("merge:{}", source.key()),
        target_key: mapping.target_key,
        expected_shape: base.shape.clone(),
        strength: mapping.patch_strength,
        strength_model: mapping.model_strength,
        slices: Vec::new(),
        transform: PatchValueTransform::default(),
        payload: PatchPayload::DenseDiff {
            tensor: base,
            pad_weight: false,
        },
    }))
}

pub fn patch_weight_to_device(
    graph: &PatchGraph,
    target_key: &str,
    value: &PatchableWeight,
    force_cast: bool,
    compute_boundary: PatchComputeBoundary,
    backend: &dyn TensorBackend,
    context: &ExecutionContext<'_>,
) -> Result<PatchableWeight, PatchAdapterError> {
    context.check()?;
    let has_patches = !graph.operations_for_key(target_key).is_empty();
    match value {
        PatchableWeight::Dense(tensor) if !has_patches && !force_cast => {
            if tensor.descriptor().device() != backend.device() {
                return Err(PatchAdapterError::UnsupportedPrefetchDevice {
                    expected: backend.device(),
                    actual: tensor.descriptor().device(),
                });
            }
            Ok(PatchableWeight::Dense(tensor.clone()))
        }
        PatchableWeight::Dense(tensor) => {
            if !has_patches {
                let descriptor = comfy_tensor::TensorDescriptor::contiguous(
                    tensor.descriptor().shape().to_vec(),
                    tensor.descriptor().dtype(),
                    backend.device(),
                    context.stream,
                )?;
                let (copied, event) = backend.copy(tensor, descriptor, context)?;
                backend.wait_event(event, context)?;
                return Ok(PatchableWeight::Dense(copied));
            }
            Ok(PatchableWeight::Dense(graph.apply_single_tensor(
                backend,
                target_key,
                tensor,
                compute_boundary,
                context,
            )?))
        }
        PatchableWeight::Quantized(value) if !has_patches && !force_cast => {
            Ok(PatchableWeight::Quantized(value.clone()))
        }
        PatchableWeight::Quantized(value) => Ok(PatchableWeight::Quantized(patch_quantized_value(
            graph,
            target_key,
            value,
            compute_boundary,
            backend,
            context,
        )?)),
    }
}

pub fn patch_quantized_value(
    graph: &PatchGraph,
    target_key: &str,
    value: &QuantizedPatchValue,
    compute_boundary: PatchComputeBoundary,
    backend: &dyn TensorBackend,
    context: &ExecutionContext<'_>,
) -> Result<QuantizedPatchValue, PatchAdapterError> {
    context.check()?;
    let (rows, columns, dtype, values) = match value {
        QuantizedPatchValue::Matrix(matrix) => (
            matrix.rows(),
            matrix.columns(),
            matrix.original_dtype(),
            matrix.materialize(backend, context)?.values().to_vec(),
        ),
        QuantizedPatchValue::Linear(matrix) => (
            matrix.rows(),
            matrix.columns(),
            matrix.original_dtype(),
            matrix.materialize(backend, context)?.values().to_vec(),
        ),
    };
    let shape = vec![checked_u64(rows)?, checked_u64(columns)?];
    let (tensor, event) = backend.upload_f32_payload(&shape, &values, DType::F32, context)?;
    backend.wait_event(event, context)?;
    let patched =
        graph.apply_single_tensor(backend, target_key, &tensor, compute_boundary, context)?;
    let patched = comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_backend_exact_native(
        backend,
        &patched,
        context,
    )?;
    context.check()?;
    Ok(match value {
        QuantizedPatchValue::Matrix(matrix) => QuantizedPatchValue::Matrix(quantize_matrix(
            matrix.kind(),
            dtype,
            &patched,
            rows,
            columns,
            context.cancellation,
        )?),
        QuantizedPatchValue::Linear(matrix) => QuantizedPatchValue::Linear(quantize_linear_matrix(
            matrix.layout(),
            dtype,
            &patched,
            rows,
            columns,
            QuantLinearScale::Recalculate,
            context.cancellation,
        )?),
    })
}

fn patchable_to_patch_tensor(
    value: &PatchableWeight,
    backend: &dyn TensorBackend,
    context: &ExecutionContext<'_>,
) -> Result<crate::PatchTensor, PatchAdapterError> {
    let (shape, values) = match value {
        PatchableWeight::Dense(tensor) => (
            tensor.descriptor().shape().to_vec(),
            comfy_tensor::generated_comfy_operator_indirection_01::tensor_to_f32_with_backend_exact_native(
                backend,
                tensor,
                context,
            )?,
        ),
        PatchableWeight::Quantized(QuantizedPatchValue::Matrix(matrix)) => (
            vec![checked_u64(matrix.rows())?, checked_u64(matrix.columns())?],
            matrix.materialize(backend, context)?.values().to_vec(),
        ),
        PatchableWeight::Quantized(QuantizedPatchValue::Linear(matrix)) => (
            vec![checked_u64(matrix.rows())?, checked_u64(matrix.columns())?],
            matrix.materialize(backend, context)?.values().to_vec(),
        ),
    };
    Ok(crate::PatchTensor::checked(shape, values)?)
}

#[derive(Clone, Debug)]
pub enum PatchPreparedValue {
    Tensor(AdapterTensor),
    Adapter {
        family: AdapterFamily,
        loaded_keys: BTreeSet<String>,
        weights: Box<PatchPreparedValue>,
    },
    Tuple(Vec<PatchPreparedValue>),
    List(Vec<PatchPreparedValue>),
    Scalar(String),
}

#[derive(Clone, Debug)]
pub struct PrefetchedPatchValue {
    pub value: PatchPreparedValue,
    pub aligned_bytes: u64,
}

pub fn prefetch_prepared_value(
    value: &PatchPreparedValue,
    destination_capacity: Option<u64>,
    copy: bool,
    backend: &dyn TensorBackend,
    context: &ExecutionContext<'_>,
) -> Result<PrefetchedPatchValue, PatchAdapterError> {
    context.check()?;
    let aligned_bytes = measure_prepared_value(value, context)?;
    if destination_capacity.is_some_and(|capacity| capacity < aligned_bytes) {
        return Err(PatchAdapterError::PrefetchCapacity {
            required: aligned_bytes,
            available: destination_capacity.unwrap_or_default(),
        });
    }
    let value = if destination_capacity.is_some() {
        copy_prepared_value(value, copy, backend, context)?
    } else {
        value.clone()
    };
    context.check()?;
    Ok(PrefetchedPatchValue {
        value,
        aligned_bytes,
    })
}

fn copy_prepared_value(
    value: &PatchPreparedValue,
    copy: bool,
    backend: &dyn TensorBackend,
    context: &ExecutionContext<'_>,
) -> Result<PatchPreparedValue, PatchAdapterError> {
    context.check()?;
    Ok(match value {
        PatchPreparedValue::Tensor(AdapterTensor::Dense(tensor)) if copy => {
            if tensor.descriptor().device() != backend.device() {
                return Err(PatchAdapterError::UnsupportedPrefetchDevice {
                    expected: backend.device(),
                    actual: tensor.descriptor().device(),
                });
            }
            let descriptor = TensorDescriptor::contiguous(
                tensor.descriptor().shape().to_vec(),
                tensor.descriptor().dtype(),
                backend.device(),
                context.stream,
            )?;
            let (copied, event) = backend.copy(tensor, descriptor, context)?;
            backend.wait_event(event, context)?;
            PatchPreparedValue::Tensor(AdapterTensor::Dense(copied))
        }
        PatchPreparedValue::Tensor(tensor) => PatchPreparedValue::Tensor(tensor.clone()),
        PatchPreparedValue::Adapter {
            family,
            loaded_keys,
            weights,
        } => PatchPreparedValue::Adapter {
            family: *family,
            loaded_keys: loaded_keys.clone(),
            weights: Box::new(copy_prepared_value(weights, copy, backend, context)?),
        },
        PatchPreparedValue::Tuple(values) => PatchPreparedValue::Tuple(
            values
                .iter()
                .map(|value| copy_prepared_value(value, copy, backend, context))
                .collect::<Result<_, _>>()?,
        ),
        PatchPreparedValue::List(values) => PatchPreparedValue::List(
            values
                .iter()
                .map(|value| copy_prepared_value(value, copy, backend, context))
                .collect::<Result<_, _>>()?,
        ),
        PatchPreparedValue::Scalar(value) => PatchPreparedValue::Scalar(value.clone()),
    })
}

fn measure_prepared_value(
    value: &PatchPreparedValue,
    context: &ExecutionContext<'_>,
) -> Result<u64, PatchAdapterError> {
    context.check()?;
    match value {
        PatchPreparedValue::Tensor(tensor) => align_prefetch(tensor.storage_bytes()?),
        PatchPreparedValue::Adapter { weights, .. } => measure_prepared_value(weights, context),
        PatchPreparedValue::Tuple(values) | PatchPreparedValue::List(values) => {
            values.iter().try_fold(0_u64, |total, value| {
                total
                    .checked_add(measure_prepared_value(value, context)?)
                    .ok_or(PatchAdapterError::PrefetchOverflow)
            })
        }
        PatchPreparedValue::Scalar(_) => Ok(0),
    }
}

fn operation_for_bias(
    prefix: &str,
    weight_target: &str,
    tensor: &AdapterTensor,
    target_shapes: &BTreeMap<String, Vec<u64>>,
    strength: f32,
    strength_model: f32,
    backend: &dyn TensorBackend,
    context: &ExecutionContext<'_>,
) -> Result<SemanticPatchOperation, PatchAdapterError> {
    let stem = weight_target
        .strip_suffix(".weight")
        .ok_or_else(|| PatchAdapterError::BiasTarget(weight_target.to_owned()))?;
    let target_key = format!("{stem}.bias");
    let expected_shape = target_shapes
        .get(&target_key)
        .cloned()
        .ok_or_else(|| PatchAdapterError::MissingTargetShape(target_key.clone()))?;
    Ok(SemanticPatchOperation {
        identifier: format!("lora:{prefix}:bias"),
        target_key,
        expected_shape,
        strength,
        strength_model,
        slices: Vec::new(),
        transform: PatchValueTransform::default(),
        payload: PatchPayload::DenseDiff {
            tensor: tensor.to_patch_tensor(backend, context)?,
            pad_weight: false,
        },
    })
}

fn insert_operation(
    operations: &mut Vec<SemanticPatchOperation>,
    targets: &mut BTreeSet<String>,
    operation: SemanticPatchOperation,
) -> Result<(), PatchAdapterError> {
    if !targets.insert(operation.target_key.clone()) {
        return Err(PatchAdapterError::AmbiguousTarget);
    }
    operations.push(operation);
    Ok(())
}

fn is_excluded_clip_merge_key(key: &str) -> bool {
    key.ends_with(".position_ids") || key.ends_with(".logit_scale")
}

fn validate_slices(slices: &[PatchSlice]) -> Result<(), PatchAdapterError> {
    for slice in slices {
        if slice.length == 0 {
            return Err(PatchAdapterError::InvalidSlice);
        }
        slice
            .start
            .checked_add(slice.length)
            .ok_or(PatchAdapterError::InvalidSlice)?;
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<(), PatchAdapterError> {
    if key.is_empty() || key.len() > MAX_PATCH_KEY_BYTES || key.chars().any(char::is_control) {
        Err(PatchAdapterError::InvalidKey(key.to_owned()))
    } else {
        Ok(())
    }
}

fn validate_optional_prefix(prefix: &str) -> Result<(), PatchAdapterError> {
    if prefix.len() > MAX_PATCH_KEY_BYTES || prefix.chars().any(char::is_control) {
        Err(PatchAdapterError::InvalidKey(prefix.to_owned()))
    } else {
        Ok(())
    }
}

fn validate_ratio(ratio: f32) -> Result<(), PatchAdapterError> {
    if ratio.is_finite() && (0.0..=1.0).contains(&ratio) {
        Ok(())
    } else {
        Err(PatchAdapterError::InvalidMergeRatio(ratio))
    }
}

fn validate_merge_mode(mode: &PatchMergeMode) -> Result<(), PatchAdapterError> {
    match *mode {
        PatchMergeMode::Interpolate { ratio } => validate_ratio(ratio),
        PatchMergeMode::Add => Ok(()),
        PatchMergeMode::Subtract { multiplier }
            if multiplier.is_finite() && (-10.0..=10.0).contains(&multiplier) =>
        {
            Ok(())
        }
        PatchMergeMode::Subtract { multiplier } => {
            Err(PatchAdapterError::InvalidMergeMultiplier(multiplier))
        }
    }
}

fn validate_finite(name: &'static str, value: f32) -> Result<(), PatchAdapterError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(PatchAdapterError::NonFinite(name))
    }
}

fn align_prefetch(bytes: u64) -> Result<u64, PatchAdapterError> {
    bytes
        .checked_add(PREFETCH_ALIGNMENT - 1)
        .map(|value| value / PREFETCH_ALIGNMENT * PREFETCH_ALIGNMENT)
        .ok_or(PatchAdapterError::PrefetchOverflow)
}

fn checked_u64(value: usize) -> Result<u64, PatchAdapterError> {
    u64::try_from(value).map_err(|_| PatchAdapterError::ShapeOverflow)
}

#[derive(Debug, Error)]
pub enum PatchAdapterError {
    #[error("invalid patch key: {0}")]
    InvalidKey(String),
    #[error("a patch source alias has conflicting canonical bindings: {0}")]
    ConflictingSource(String),
    #[error("patch mapping index is corrupt")]
    MappingIndexCorrupt,
    #[error("duplicate patch merge prefix: {0}")]
    DuplicateMergePrefix(String),
    #[error("patch key has no checked model mapping: {0}")]
    UnmappedKey(String),
    #[error("invalid patch slice")]
    InvalidSlice,
    #[error("invalid patch merge ratio: {0}")]
    InvalidMergeRatio(f32),
    #[error("invalid patch merge multiplier: {0}")]
    InvalidMergeMultiplier(f32),
    #[error("malformed patch merge rule: {0}")]
    MalformedMergeRule(String),
    #[error("patch target is missing a canonical shape: {0}")]
    MissingTargetShape(String),
    #[error("multiple loaded source aliases target the same canonical parameter")]
    AmbiguousTarget,
    #[error("merge source shape {actual:?} does not match target shape {expected:?}")]
    MergeShape {
        expected: Vec<u64>,
        actual: Vec<u64>,
    },
    #[error("cannot derive a bias target from non-weight key: {0}")]
    BiasTarget(String),
    #[error("patch adapter value {0} must be finite")]
    NonFinite(&'static str),
    #[error("patch prefetch size overflowed")]
    PrefetchOverflow,
    #[error("patch prefetch requires {required} bytes but only {available} are authorized")]
    PrefetchCapacity { required: u64, available: u64 },
    #[error("patch prefetch device mismatch: expected {expected:?}, got {actual:?}")]
    UnsupportedPrefetchDevice {
        expected: comfy_tensor::DeviceId,
        actual: comfy_tensor::DeviceId,
    },
    #[error("patch adapter shape conversion overflowed")]
    ShapeOverflow,
    #[error("an appended patch graph uses a different base artifact digest")]
    BaseDigestMismatch,
    #[error(transparent)]
    WeightAdapter(#[from] WeightAdapterError),
    #[error(transparent)]
    PatchGraph(#[from] PatchGraphError),
    #[error(transparent)]
    Quantization(#[from] QuantizationError),
    #[error(transparent)]
    Tensor(#[from] comfy_tensor::TensorError),
    #[error(transparent)]
    Operator(
        #[from] comfy_tensor::generated_comfy_operator_indirection_01::OperatorIndirectionError,
    ),
    #[error(transparent)]
    Cancellation(#[from] comfy_types::CancellationError),
}
