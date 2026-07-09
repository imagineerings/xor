use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{LatentFormat, ModelFamilyKind, ModelMediaCapability};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SamplerKind {
    Euler,
    EulerAncestral,
    Heun,
    Dpm2,
    Dpm2Ancestral,
    Dpmpp2M,
    Dpmpp2SAncestral,
    DpmppSde,
    Dpmpp3M,
    Lcm,
    UniPc,
}

impl SamplerKind {
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Euler => "euler",
            Self::EulerAncestral => "euler_ancestral",
            Self::Heun => "heun",
            Self::Dpm2 => "dpm_2",
            Self::Dpm2Ancestral => "dpm_2_ancestral",
            Self::Dpmpp2M => "dpmpp_2m",
            Self::Dpmpp2SAncestral => "dpmpp_2s_ancestral",
            Self::DpmppSde => "dpmpp_sde",
            Self::Dpmpp3M => "dpmpp_3m",
            Self::Lcm => "lcm",
            Self::UniPc => "uni_pc",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SchedulerKind {
    Normal,
    Karras,
    Exponential,
    Simple,
    SgmUniform,
    AlignYourSteps,
    Beta,
}

impl SchedulerKind {
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Karras => "karras",
            Self::Exponential => "exponential",
            Self::Simple => "simple",
            Self::SgmUniform => "sgm_uniform",
            Self::AlignYourSteps => "align_your_steps",
            Self::Beta => "beta",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GuidanceMode {
    ClassifierFree,
    Distilled,
    FluxGuidance,
    VideoGuidance,
    ControlGuidance,
}

impl GuidanceMode {
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::ClassifierFree => "classifier_free",
            Self::Distilled => "distilled",
            Self::FluxGuidance => "flux_guidance",
            Self::VideoGuidance => "video_guidance",
            Self::ControlGuidance => "control_guidance",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SamplerCapability {
    pub kind: SamplerKind,
    pub aliases: BTreeSet<String>,
    pub supports_deterministic_noise: bool,
    pub supports_start_end_steps: bool,
    pub supported_schedulers: BTreeSet<SchedulerKind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SchedulerCapability {
    pub kind: SchedulerKind,
    pub aliases: BTreeSet<String>,
    pub supports_custom_sigmas: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GuidanceCapability {
    pub mode: GuidanceMode,
    pub aliases: BTreeSet<String>,
    pub supports_cfg_scale: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelFamilyExecutionProfile {
    pub family: ModelFamilyKind,
    pub media: BTreeSet<ModelMediaCapability>,
    pub latent_format: LatentFormat,
    pub supported_samplers: BTreeSet<SamplerKind>,
    pub supported_schedulers: BTreeSet<SchedulerKind>,
    pub supported_guidance: BTreeSet<GuidanceMode>,
    pub supports_patches: bool,
    pub supports_vae: bool,
    pub temporal: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ExecutionBehaviorKey {
    pub behavior: String,
}

impl ExecutionBehaviorKey {
    pub fn new(behavior: impl Into<String>) -> Self {
        Self {
            behavior: behavior.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DivergenceReason {
    Safety,
    Security,
    DependencyReview,
    Platform,
    Product,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DivergenceRecord {
    pub behavior: ExecutionBehaviorKey,
    pub comfy_source: String,
    pub reason: DivergenceReason,
    pub sim_behavior: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiffusionWorldModelBacklogCatalog {
    pub schema_version: u32,
    pub source_root: String,
    pub source_category: String,
    pub captured_at: String,
    pub implementation_owner: String,
    pub native_sim_records: bool,
    pub comfyui_passthrough: bool,
    pub requires_downloads: bool,
    pub dependency_review: String,
    pub records: Vec<SimDiffusionWorldModelBacklogRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiffusionWorldModelBacklogRecord {
    pub source_id: String,
    pub source_path: String,
    pub source_kind: String,
    pub node_name: String,
    pub native_surface: String,
    pub evidence_module: String,
    pub evidence_kind: String,
    pub metadata_only: bool,
    pub requires_dependency_review: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimDiffusionWorldModelBacklogDiagnostic {
    pub code: String,
    pub message: String,
}

impl SimDiffusionWorldModelBacklogCatalog {
    pub fn validate(&self) -> Result<(), Vec<SimDiffusionWorldModelBacklogDiagnostic>> {
        let mut diagnostics = Vec::new();

        if self.schema_version != 1 {
            diagnostics.push(sim_backlog_diagnostic(
                "sim.diffusion_world_model_backlog.invalid_schema",
                "diffusion/world-model backlog fixture must use schema version 1",
            ));
        }
        if self.source_root != "projects/comfy" {
            diagnostics.push(sim_backlog_diagnostic(
                "sim.diffusion_world_model_backlog.invalid_source_root",
                "diffusion/world-model backlog fixture must preserve projects/comfy attribution",
            ));
        }
        if !self.native_sim_records || self.comfyui_passthrough {
            diagnostics.push(sim_backlog_diagnostic(
                "sim.diffusion_world_model_backlog.not_native",
                "diffusion/world-model backlog fixture must describe native Sim records only",
            ));
        }
        if self.requires_downloads || self.dependency_review != "not_required" {
            diagnostics.push(sim_backlog_diagnostic(
                "sim.diffusion_world_model_backlog.unsafe_dependency",
                "metadata-only backlog fixture must not require downloads or dependency review",
            ));
        }
        if self.records.is_empty() {
            diagnostics.push(sim_backlog_diagnostic(
                "sim.diffusion_world_model_backlog.empty",
                "diffusion/world-model backlog fixture must include covered source records",
            ));
        }

        let mut source_ids = BTreeSet::new();
        for record in &self.records {
            if !source_ids.insert(&record.source_id) {
                diagnostics.push(sim_backlog_diagnostic(
                    "sim.diffusion_world_model_backlog.duplicate_record",
                    format!("duplicate source id `{}`", record.source_id),
                ));
            }
            if !record.source_path.starts_with("projects/comfy") {
                diagnostics.push(sim_backlog_diagnostic(
                    "sim.diffusion_world_model_backlog.invalid_source_path",
                    format!(
                        "source path `{}` does not preserve projects/comfy attribution",
                        record.source_path
                    ),
                ));
            }
            if record.node_name.is_empty()
                || record.native_surface.is_empty()
                || record.evidence_module.is_empty()
                || record.evidence_kind.is_empty()
            {
                diagnostics.push(sim_backlog_diagnostic(
                    "sim.diffusion_world_model_backlog.missing_evidence",
                    format!(
                        "record `{}` is missing native Sim evidence metadata",
                        record.source_id
                    ),
                ));
            }
            if !record.metadata_only || record.requires_dependency_review {
                diagnostics.push(sim_backlog_diagnostic(
                    "sim.diffusion_world_model_backlog.unsafe_record",
                    format!("record `{}` must stay metadata-only until dependency review enables real workers", record.source_id),
                ));
            }
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }

    pub fn records_by_surface(
        &self,
    ) -> BTreeMap<String, Vec<&SimDiffusionWorldModelBacklogRecord>> {
        let mut records = BTreeMap::new();
        for record in &self.records {
            records
                .entry(record.native_surface.clone())
                .or_insert_with(Vec::new)
                .push(record);
        }
        records
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyExecutionRegistry {
    samplers: BTreeMap<SamplerKind, SamplerCapability>,
    sampler_aliases: BTreeMap<String, SamplerKind>,
    schedulers: BTreeMap<SchedulerKind, SchedulerCapability>,
    scheduler_aliases: BTreeMap<String, SchedulerKind>,
    guidance: BTreeMap<GuidanceMode, GuidanceCapability>,
    guidance_aliases: BTreeMap<String, GuidanceMode>,
    model_families: BTreeMap<ModelFamilyKind, ModelFamilyExecutionProfile>,
    divergences: BTreeMap<ExecutionBehaviorKey, DivergenceRecord>,
}

fn sim_backlog_diagnostic(
    code: impl Into<String>,
    message: impl Into<String>,
) -> SimDiffusionWorldModelBacklogDiagnostic {
    SimDiffusionWorldModelBacklogDiagnostic {
        code: code.into(),
        message: message.into(),
    }
}

impl ComfyExecutionRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            samplers: BTreeMap::new(),
            sampler_aliases: BTreeMap::new(),
            schedulers: BTreeMap::new(),
            scheduler_aliases: BTreeMap::new(),
            guidance: BTreeMap::new(),
            guidance_aliases: BTreeMap::new(),
            model_families: BTreeMap::new(),
            divergences: BTreeMap::new(),
        };

        registry.register_default_schedulers();
        registry.register_default_samplers();
        registry.register_default_guidance();
        registry.register_default_model_families();
        registry.register_default_divergences();
        registry
    }

    pub fn sampler(&self, name: &str) -> Option<&SamplerCapability> {
        self.sampler_aliases
            .get(&normalize_name(name))
            .and_then(|kind| self.samplers.get(kind))
    }

    pub fn scheduler(&self, name: &str) -> Option<&SchedulerCapability> {
        self.scheduler_aliases
            .get(&normalize_name(name))
            .and_then(|kind| self.schedulers.get(kind))
    }

    pub fn guidance(&self, name: &str) -> Option<&GuidanceCapability> {
        self.guidance_aliases
            .get(&normalize_name(name))
            .and_then(|mode| self.guidance.get(mode))
    }

    pub fn model_family(&self, family: ModelFamilyKind) -> Option<&ModelFamilyExecutionProfile> {
        self.model_families.get(&family)
    }

    pub fn divergence(&self, key: &ExecutionBehaviorKey) -> Option<&DivergenceRecord> {
        self.divergences.get(key)
    }

    pub fn model_families(&self) -> Vec<ModelFamilyExecutionProfile> {
        self.model_families.values().cloned().collect()
    }

    fn register_sampler(&mut self, capability: SamplerCapability) {
        let kind = capability.kind;
        self.sampler_aliases
            .insert(kind.canonical_name().to_string(), kind);
        for alias in &capability.aliases {
            self.sampler_aliases.insert(normalize_name(alias), kind);
        }
        self.samplers.insert(kind, capability);
    }

    fn register_scheduler(&mut self, capability: SchedulerCapability) {
        let kind = capability.kind;
        self.scheduler_aliases
            .insert(kind.canonical_name().to_string(), kind);
        for alias in &capability.aliases {
            self.scheduler_aliases.insert(normalize_name(alias), kind);
        }
        self.schedulers.insert(kind, capability);
    }

    fn register_guidance(&mut self, capability: GuidanceCapability) {
        let mode = capability.mode;
        self.guidance_aliases
            .insert(mode.canonical_name().to_string(), mode);
        for alias in &capability.aliases {
            self.guidance_aliases.insert(normalize_name(alias), mode);
        }
        self.guidance.insert(mode, capability);
    }

    fn register_model_family(&mut self, profile: ModelFamilyExecutionProfile) {
        self.model_families.insert(profile.family, profile);
    }

    fn register_divergence(&mut self, record: DivergenceRecord) {
        self.divergences.insert(record.behavior.clone(), record);
    }

    fn register_default_schedulers(&mut self) {
        for (kind, aliases, supports_custom_sigmas) in [
            (SchedulerKind::Normal, vec!["normal"], false),
            (SchedulerKind::Karras, vec!["karras"], false),
            (
                SchedulerKind::Exponential,
                vec!["exponential", "exponential_sgm"],
                false,
            ),
            (SchedulerKind::Simple, vec!["simple"], false),
            (
                SchedulerKind::SgmUniform,
                vec!["sgm_uniform", "sgm uniform"],
                false,
            ),
            (
                SchedulerKind::AlignYourSteps,
                vec!["align_your_steps", "ays"],
                false,
            ),
            (SchedulerKind::Beta, vec!["beta"], true),
        ] {
            self.register_scheduler(SchedulerCapability {
                kind,
                aliases: aliases.into_iter().map(str::to_string).collect(),
                supports_custom_sigmas,
            });
        }
    }

    fn register_default_samplers(&mut self) {
        let all_schedulers = all_schedulers();
        for (kind, aliases, deterministic, start_end) in [
            (SamplerKind::Euler, vec!["euler"], true, true),
            (
                SamplerKind::EulerAncestral,
                vec!["euler_ancestral", "euler a", "euler_a"],
                true,
                true,
            ),
            (SamplerKind::Heun, vec!["heun"], true, true),
            (SamplerKind::Dpm2, vec!["dpm_2", "dpm2"], true, true),
            (
                SamplerKind::Dpm2Ancestral,
                vec!["dpm_2_ancestral", "dpm2 a", "dpm2_ancestral"],
                true,
                true,
            ),
            (
                SamplerKind::Dpmpp2M,
                vec!["dpmpp_2m", "dpm++ 2m"],
                true,
                true,
            ),
            (
                SamplerKind::Dpmpp2SAncestral,
                vec!["dpmpp_2s_ancestral", "dpm++ 2s ancestral"],
                true,
                true,
            ),
            (
                SamplerKind::DpmppSde,
                vec!["dpmpp_sde", "dpm++ sde"],
                true,
                true,
            ),
            (
                SamplerKind::Dpmpp3M,
                vec!["dpmpp_3m", "dpm++ 3m"],
                true,
                true,
            ),
            (SamplerKind::Lcm, vec!["lcm"], true, false),
            (SamplerKind::UniPc, vec!["uni_pc", "uni pc"], true, true),
        ] {
            self.register_sampler(SamplerCapability {
                kind,
                aliases: aliases.into_iter().map(str::to_string).collect(),
                supports_deterministic_noise: deterministic,
                supports_start_end_steps: start_end,
                supported_schedulers: all_schedulers.clone(),
            });
        }
    }

    fn register_default_guidance(&mut self) {
        for (mode, aliases, supports_cfg_scale) in [
            (
                GuidanceMode::ClassifierFree,
                vec!["cfg", "classifier_free", "classifier free guidance"],
                true,
            ),
            (
                GuidanceMode::Distilled,
                vec!["distilled", "distilled_cfg"],
                false,
            ),
            (
                GuidanceMode::FluxGuidance,
                vec!["flux_guidance", "flux guidance"],
                true,
            ),
            (
                GuidanceMode::VideoGuidance,
                vec!["video_guidance", "temporal_guidance"],
                true,
            ),
            (
                GuidanceMode::ControlGuidance,
                vec!["control_guidance", "controlnet_guidance"],
                true,
            ),
        ] {
            self.register_guidance(GuidanceCapability {
                mode,
                aliases: aliases.into_iter().map(str::to_string).collect(),
                supports_cfg_scale,
            });
        }
    }

    fn register_default_model_families(&mut self) {
        for family in [
            ModelFamilyKind::StableDiffusion1,
            ModelFamilyKind::StableDiffusion2,
            ModelFamilyKind::StableDiffusionXl,
            ModelFamilyKind::StableDiffusion3,
            ModelFamilyKind::Flux,
        ] {
            self.register_model_family(image_profile(family));
        }

        for family in [
            ModelFamilyKind::WanVideo,
            ModelFamilyKind::HunyuanVideo,
            ModelFamilyKind::LtxVideo,
        ] {
            self.register_model_family(video_profile(family));
        }

        self.register_model_family(specialized_profile(
            ModelFamilyKind::Audio,
            [ModelMediaCapability::Audio],
            LatentFormat::Audio,
            false,
            false,
        ));
        self.register_model_family(specialized_profile(
            ModelFamilyKind::ThreeD,
            [ModelMediaCapability::ThreeD],
            LatentFormat::Geometry,
            false,
            false,
        ));
        self.register_model_family(specialized_profile(
            ModelFamilyKind::Segmentation,
            [ModelMediaCapability::Segmentation],
            LatentFormat::None,
            false,
            false,
        ));
        self.register_model_family(specialized_profile(
            ModelFamilyKind::Depth,
            [ModelMediaCapability::Depth, ModelMediaCapability::ThreeD],
            LatentFormat::Geometry,
            false,
            false,
        ));
        self.register_model_family(specialized_profile(
            ModelFamilyKind::Detection,
            [ModelMediaCapability::Detection],
            LatentFormat::None,
            false,
            false,
        ));
    }

    fn register_default_divergences(&mut self) {
        self.register_divergence(DivergenceRecord {
            behavior: ExecutionBehaviorKey::new("implicit_model_downloads"),
            comfy_source:
                "Comfy workflows can reference missing weights resolved by external setup"
                    .to_string(),
            reason: DivergenceReason::DependencyReview,
            sim_behavior:
                "Sim blocks execution until model downloads are explicitly approved and reviewed"
                    .to_string(),
        });
    }
}

impl Default for ComfyExecutionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn image_profile(family: ModelFamilyKind) -> ModelFamilyExecutionProfile {
    let guidance = if family == ModelFamilyKind::Flux {
        [GuidanceMode::FluxGuidance, GuidanceMode::Distilled]
            .into_iter()
            .collect()
    } else {
        [GuidanceMode::ClassifierFree, GuidanceMode::ControlGuidance]
            .into_iter()
            .collect()
    };

    ModelFamilyExecutionProfile {
        family,
        media: [ModelMediaCapability::Image].into_iter().collect(),
        latent_format: match family {
            ModelFamilyKind::StableDiffusion1 | ModelFamilyKind::StableDiffusion2 => {
                LatentFormat::StableDiffusion
            }
            ModelFamilyKind::StableDiffusionXl => LatentFormat::StableDiffusionXl,
            ModelFamilyKind::StableDiffusion3 => LatentFormat::StableDiffusion3,
            ModelFamilyKind::Flux => LatentFormat::Flux,
            _ => LatentFormat::None,
        },
        supported_samplers: image_samplers(),
        supported_schedulers: all_schedulers(),
        supported_guidance: guidance,
        supports_patches: true,
        supports_vae: true,
        temporal: false,
    }
}

fn video_profile(family: ModelFamilyKind) -> ModelFamilyExecutionProfile {
    ModelFamilyExecutionProfile {
        family,
        media: [ModelMediaCapability::Video].into_iter().collect(),
        latent_format: LatentFormat::Video,
        supported_samplers: [SamplerKind::Euler, SamplerKind::Dpmpp2M, SamplerKind::UniPc]
            .into_iter()
            .collect(),
        supported_schedulers: [
            SchedulerKind::Normal,
            SchedulerKind::Karras,
            SchedulerKind::SgmUniform,
        ]
        .into_iter()
        .collect(),
        supported_guidance: [GuidanceMode::VideoGuidance, GuidanceMode::ClassifierFree]
            .into_iter()
            .collect(),
        supports_patches: true,
        supports_vae: true,
        temporal: true,
    }
}

fn specialized_profile(
    family: ModelFamilyKind,
    media: impl IntoIterator<Item = ModelMediaCapability>,
    latent_format: LatentFormat,
    supports_patches: bool,
    supports_vae: bool,
) -> ModelFamilyExecutionProfile {
    ModelFamilyExecutionProfile {
        family,
        media: media.into_iter().collect(),
        latent_format,
        supported_samplers: BTreeSet::new(),
        supported_schedulers: BTreeSet::new(),
        supported_guidance: BTreeSet::new(),
        supports_patches,
        supports_vae,
        temporal: false,
    }
}

fn image_samplers() -> BTreeSet<SamplerKind> {
    [
        SamplerKind::Euler,
        SamplerKind::EulerAncestral,
        SamplerKind::Heun,
        SamplerKind::Dpm2,
        SamplerKind::Dpm2Ancestral,
        SamplerKind::Dpmpp2M,
        SamplerKind::Dpmpp2SAncestral,
        SamplerKind::DpmppSde,
        SamplerKind::Dpmpp3M,
        SamplerKind::Lcm,
        SamplerKind::UniPc,
    ]
    .into_iter()
    .collect()
}

fn all_schedulers() -> BTreeSet<SchedulerKind> {
    [
        SchedulerKind::Normal,
        SchedulerKind::Karras,
        SchedulerKind::Exponential,
        SchedulerKind::Simple,
        SchedulerKind::SgmUniform,
        SchedulerKind::AlignYourSteps,
        SchedulerKind::Beta,
    ]
    .into_iter()
    .collect()
}

fn normalize_name(name: &str) -> String {
    name.trim().replace(['-', ' '], "_").to_ascii_lowercase()
}
