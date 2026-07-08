use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ModelCategory {
    Checkpoints,
    Configs,
    Loras,
    Vae,
    TextEncoders,
    DiffusionModels,
    ClipVision,
    StyleModels,
    Embeddings,
    Diffusers,
    VaeApprox,
    ControlNet,
    Gligen,
    UpscaleModels,
    LatentUpscaleModels,
    Hypernetworks,
    ModelPatches,
    AudioEncoders,
    BackgroundRemoval,
    FrameInterpolation,
    GeometryEstimation,
    OpticalFlow,
    Detection,
}

impl ModelCategory {
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Checkpoints => "checkpoints",
            Self::Configs => "configs",
            Self::Loras => "loras",
            Self::Vae => "vae",
            Self::TextEncoders => "text_encoders",
            Self::DiffusionModels => "diffusion_models",
            Self::ClipVision => "clip_vision",
            Self::StyleModels => "style_models",
            Self::Embeddings => "embeddings",
            Self::Diffusers => "diffusers",
            Self::VaeApprox => "vae_approx",
            Self::ControlNet => "controlnet",
            Self::Gligen => "gligen",
            Self::UpscaleModels => "upscale_models",
            Self::LatentUpscaleModels => "latent_upscale_models",
            Self::Hypernetworks => "hypernetworks",
            Self::ModelPatches => "model_patches",
            Self::AudioEncoders => "audio_encoders",
            Self::BackgroundRemoval => "background_removal",
            Self::FrameInterpolation => "frame_interpolation",
            Self::GeometryEstimation => "geometry_estimation",
            Self::OpticalFlow => "optical_flow",
            Self::Detection => "detection",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Checkpoints,
            Self::Configs,
            Self::Loras,
            Self::Vae,
            Self::TextEncoders,
            Self::DiffusionModels,
            Self::ClipVision,
            Self::StyleModels,
            Self::Embeddings,
            Self::Diffusers,
            Self::VaeApprox,
            Self::ControlNet,
            Self::Gligen,
            Self::UpscaleModels,
            Self::LatentUpscaleModels,
            Self::Hypernetworks,
            Self::ModelPatches,
            Self::AudioEncoders,
            Self::BackgroundRemoval,
            Self::FrameInterpolation,
            Self::GeometryEstimation,
            Self::OpticalFlow,
            Self::Detection,
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelFolderInfo {
    pub category: ModelCategory,
    pub canonical_name: String,
    pub roots: Vec<PathBuf>,
    pub allowed_extensions: BTreeSet<String>,
    pub legacy_names: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelFileRef {
    pub category: ModelCategory,
    pub root_index: usize,
    pub root: PathBuf,
    pub relative_path: PathBuf,
    pub full_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelFolderError {
    UnknownCategory(String),
    MissingRoot {
        category: ModelCategory,
    },
    UnsafeRelativePath {
        relative_path: PathBuf,
    },
    ExtensionNotAllowed {
        category: ModelCategory,
        extension: Option<String>,
    },
}

impl fmt::Display for ModelFolderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCategory(category) => {
                write!(formatter, "unknown model category `{category}`")
            }
            Self::MissingRoot { category } => {
                write!(
                    formatter,
                    "model category `{}` has no registered roots",
                    category.canonical_name()
                )
            }
            Self::UnsafeRelativePath { relative_path } => {
                write!(
                    formatter,
                    "model path `{}` escapes the registered roots",
                    relative_path.display()
                )
            }
            Self::ExtensionNotAllowed {
                category,
                extension,
            } => {
                let extension = extension.as_deref().unwrap_or("<none>");
                write!(
                    formatter,
                    "extension `{extension}` is not allowed for model category `{}`",
                    category.canonical_name()
                )
            }
        }
    }
}

impl std::error::Error for ModelFolderError {}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtraModelPathConfig {
    pub roots: Vec<ExtraModelPathRoot>,
}

impl ExtraModelPathConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root(mut self, root: ExtraModelPathRoot) -> Self {
        self.roots.push(root);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtraModelPathRoot {
    pub category_name: String,
    pub root: PathBuf,
}

impl ExtraModelPathRoot {
    pub fn new(category_name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            category_name: category_name.into(),
            root: root.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyModelFolderRegistry {
    folders: BTreeMap<ModelCategory, ModelFolderInfo>,
    aliases: BTreeMap<String, ModelCategory>,
}

impl ComfyModelFolderRegistry {
    pub fn new(project_asset_root: impl Into<PathBuf>) -> Self {
        let project_asset_root = project_asset_root.into();
        let mut registry = Self {
            folders: BTreeMap::new(),
            aliases: BTreeMap::new(),
        };

        for category in ModelCategory::all() {
            registry.register_default_folder(*category, &project_asset_root);
        }

        registry
    }

    pub fn folders(&self) -> Vec<ModelFolderInfo> {
        self.folders.values().cloned().collect()
    }

    pub fn folder(&self, category: ModelCategory) -> Option<&ModelFolderInfo> {
        self.folders.get(&category)
    }

    pub fn category_for_name(&self, name: &str) -> Result<ModelCategory, ModelFolderError> {
        normalize_category_name(name)
            .and_then(|name| self.aliases.get(&name).copied())
            .ok_or_else(|| ModelFolderError::UnknownCategory(name.to_string()))
    }

    pub fn add_extra_paths(
        &mut self,
        config: ExtraModelPathConfig,
    ) -> Result<(), ModelFolderError> {
        for root in config.roots {
            let category = self.category_for_name(&root.category_name)?;
            if let Some(folder) = self.folders.get_mut(&category)
                && !folder.roots.contains(&root.root)
            {
                folder.roots.push(root.root);
            }
        }

        Ok(())
    }

    pub fn resolve(
        &self,
        category_name: &str,
        relative_path: impl AsRef<Path>,
    ) -> Result<ModelFileRef, ModelFolderError> {
        let category = self.category_for_name(category_name)?;
        self.resolve_category(category, relative_path)
    }

    pub fn resolve_category(
        &self,
        category: ModelCategory,
        relative_path: impl AsRef<Path>,
    ) -> Result<ModelFileRef, ModelFolderError> {
        let relative_path = relative_path.as_ref();
        if !is_safe_relative_path(relative_path) {
            return Err(ModelFolderError::UnsafeRelativePath {
                relative_path: relative_path.to_path_buf(),
            });
        }

        let folder = self.folders.get(&category).ok_or_else(|| {
            ModelFolderError::UnknownCategory(category.canonical_name().to_string())
        })?;
        if folder.roots.is_empty() {
            return Err(ModelFolderError::MissingRoot { category });
        }

        let extension = relative_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase());
        if let Some(extension) = extension.as_ref()
            && !folder.allowed_extensions.contains(extension)
        {
            return Err(ModelFolderError::ExtensionNotAllowed {
                category,
                extension: Some(extension.clone()),
            });
        } else if extension.is_none() && !folder.allowed_extensions.is_empty() {
            return Err(ModelFolderError::ExtensionNotAllowed {
                category,
                extension: None,
            });
        }

        let root = folder.roots[0].clone();
        Ok(ModelFileRef {
            category,
            root_index: 0,
            full_path: root.join(relative_path),
            root,
            relative_path: relative_path.to_path_buf(),
        })
    }

    fn register_default_folder(&mut self, category: ModelCategory, project_asset_root: &Path) {
        let canonical_name = category.canonical_name().to_string();
        let legacy_names = legacy_names(category)
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        let roots = vec![project_asset_root.join("models").join(&canonical_name)];
        let allowed_extensions = allowed_extensions(category)
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<_>>();

        self.aliases.insert(canonical_name.clone(), category);
        for legacy_name in &legacy_names {
            self.aliases.insert(
                normalize_category_name(legacy_name).unwrap_or_default(),
                category,
            );
        }

        self.folders.insert(
            category,
            ModelFolderInfo {
                category,
                canonical_name,
                roots,
                allowed_extensions,
                legacy_names,
            },
        );
    }
}

impl Default for ComfyModelFolderRegistry {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

fn normalize_category_name(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }

    Some(name.replace('-', "_").to_ascii_lowercase())
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn legacy_names(category: ModelCategory) -> Vec<&'static str> {
    match category {
        ModelCategory::Checkpoints => vec!["ckpt", "checkpoint", "checkpoints"],
        ModelCategory::Configs => vec!["config", "configs"],
        ModelCategory::Loras => vec!["lora", "loras"],
        ModelCategory::Vae => vec!["vae", "vae_models"],
        ModelCategory::TextEncoders => vec!["clip", "clips", "text_encoder", "text_encoders"],
        ModelCategory::DiffusionModels => {
            vec!["diffusion_model", "diffusion_models", "unet", "unets"]
        }
        ModelCategory::ClipVision => vec!["clip_vision", "clip_vision_models"],
        ModelCategory::StyleModels => vec!["style_model", "style_models"],
        ModelCategory::Embeddings => vec!["embedding", "embeddings"],
        ModelCategory::Diffusers => vec!["diffusers"],
        ModelCategory::VaeApprox => vec!["vae_approx", "vae_approximation"],
        ModelCategory::ControlNet => vec!["controlnet", "control_net", "controlnets"],
        ModelCategory::Gligen => vec!["gligen"],
        ModelCategory::UpscaleModels => vec!["upscale_model", "upscale_models"],
        ModelCategory::LatentUpscaleModels => vec!["latent_upscale", "latent_upscale_models"],
        ModelCategory::Hypernetworks => vec!["hypernetwork", "hypernetworks"],
        ModelCategory::ModelPatches => vec!["model_patch", "model_patches"],
        ModelCategory::AudioEncoders => vec!["audio_encoder", "audio_encoders"],
        ModelCategory::BackgroundRemoval => vec!["background_removal", "birefnet", "rembg"],
        ModelCategory::FrameInterpolation => vec!["frame_interpolation", "rife"],
        ModelCategory::GeometryEstimation => {
            vec!["geometry_estimation", "depth_anything", "normal_estimation"]
        }
        ModelCategory::OpticalFlow => vec!["optical_flow", "raft"],
        ModelCategory::Detection => vec!["detection", "detectors", "ultralytics", "yolo"],
    }
}

fn allowed_extensions(category: ModelCategory) -> Vec<&'static str> {
    match category {
        ModelCategory::Configs => vec!["json", "toml", "yaml", "yml"],
        ModelCategory::Embeddings => vec!["bin", "pt", "safetensors"],
        ModelCategory::Diffusers => vec!["bin", "json", "safetensors"],
        _ => vec!["bin", "ckpt", "gguf", "onnx", "pt", "pth", "safetensors"],
    }
}
