use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SIM_IMAGE_SHAPE_MISMATCH_CODE: &str = "world_model.image_nodes.shape_mismatch";
pub const SIM_IMAGE_INVALID_REGION_CODE: &str = "world_model.image_nodes.invalid_region";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimImageColorSpace {
    Srgb,
    Linear,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimImageFormat {
    Png,
    Jpeg,
    Webp,
    Svg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimImageShape {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub batch: u32,
}

impl SimImageShape {
    pub fn new(width: u32, height: u32, channels: u8) -> Self {
        Self {
            width,
            height,
            channels,
            batch: 1,
        }
    }

    pub fn with_batch(mut self, batch: u32) -> Self {
        self.batch = batch;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimGlslDependency {
    pub shader_id: String,
    pub reviewed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimImageArtifact {
    pub shape: SimImageShape,
    pub color_space: SimImageColorSpace,
    pub format: Option<SimImageFormat>,
    pub metadata: BTreeMap<String, String>,
    pub glsl_dependencies: Vec<SimGlslDependency>,
}

impl SimImageArtifact {
    pub fn new(shape: SimImageShape) -> Self {
        Self {
            shape,
            color_space: SimImageColorSpace::Srgb,
            format: None,
            metadata: BTreeMap::new(),
            glsl_dependencies: Vec::new(),
        }
    }

    pub fn with_format(mut self, format: SimImageFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn with_glsl_dependency(mut self, shader_id: impl Into<String>, reviewed: bool) -> Self {
        self.glsl_dependencies.push(SimGlslDependency {
            shader_id: shader_id.into(),
            reviewed,
        });
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimImageRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimImageFlipAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimImageNodeDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimImageNodeAdapter;

impl SimImageNodeAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn resize(
        &self,
        image: &SimImageArtifact,
        width: u32,
        height: u32,
    ) -> Result<SimImageArtifact, SimImageNodeDiagnostic> {
        if width == 0 || height == 0 {
            return Err(diagnostic(
                SIM_IMAGE_INVALID_REGION_CODE,
                "resize dimensions must be greater than zero",
            ));
        }
        let mut image = image.clone();
        image.shape.width = width;
        image.shape.height = height;
        image
            .metadata
            .insert("sim.operation".to_string(), "resize".to_string());
        Ok(image)
    }

    pub fn crop(
        &self,
        image: &SimImageArtifact,
        region: SimImageRegion,
    ) -> Result<SimImageArtifact, SimImageNodeDiagnostic> {
        validate_region(image.shape, region)?;
        let mut image = image.clone();
        image.shape.width = region.width;
        image.shape.height = region.height;
        image
            .metadata
            .insert("sim.operation".to_string(), "crop".to_string());
        Ok(image)
    }

    pub fn pad(&self, image: &SimImageArtifact, x: u32, y: u32) -> SimImageArtifact {
        let mut image = image.clone();
        image.shape.width = image.shape.width.saturating_add(x.saturating_mul(2));
        image.shape.height = image.shape.height.saturating_add(y.saturating_mul(2));
        image
            .metadata
            .insert("sim.operation".to_string(), "pad".to_string());
        image
    }

    pub fn flip(&self, image: &SimImageArtifact, axis: SimImageFlipAxis) -> SimImageArtifact {
        let mut image = image.clone();
        image
            .metadata
            .insert("sim.flip_axis".to_string(), format!("{axis:?}"));
        image
    }

    pub fn rotate_right(&self, image: &SimImageArtifact) -> SimImageArtifact {
        let mut image = image.clone();
        std::mem::swap(&mut image.shape.width, &mut image.shape.height);
        image
            .metadata
            .insert("sim.operation".to_string(), "rotate_right".to_string());
        image
    }

    pub fn add_noise_metadata(
        &self,
        image: &SimImageArtifact,
        seed: u64,
        strength_milli: u16,
    ) -> SimImageArtifact {
        let mut image = image.clone();
        image
            .metadata
            .insert("sim.noise_seed".to_string(), seed.to_string());
        image.metadata.insert(
            "sim.noise_strength_milli".to_string(),
            strength_milli.to_string(),
        );
        image
    }

    pub fn blend(
        &self,
        base: &SimImageArtifact,
        overlay: &SimImageArtifact,
        opacity_milli: u16,
    ) -> Result<SimImageArtifact, SimImageNodeDiagnostic> {
        if base.shape != overlay.shape {
            return Err(diagnostic(
                SIM_IMAGE_SHAPE_MISMATCH_CODE,
                "image blend requires matching width, height, channels, and batch",
            ));
        }
        let mut image = base.clone();
        image.metadata.insert(
            "sim.blend_opacity_milli".to_string(),
            opacity_milli.min(1000).to_string(),
        );
        Ok(image)
    }

    pub fn save_as(&self, image: &SimImageArtifact, format: SimImageFormat) -> SimImageArtifact {
        let mut image = image.clone();
        image.format = Some(format);
        image
            .metadata
            .insert("sim.operation".to_string(), "save".to_string());
        image
    }
}

fn validate_region(
    shape: SimImageShape,
    region: SimImageRegion,
) -> Result<(), SimImageNodeDiagnostic> {
    if region.width == 0
        || region.height == 0
        || region.x.saturating_add(region.width) > shape.width
        || region.y.saturating_add(region.height) > shape.height
    {
        Err(diagnostic(
            SIM_IMAGE_INVALID_REGION_CODE,
            "image region must be non-empty and stay inside the source image",
        ))
    } else {
        Ok(())
    }
}

fn diagnostic(code: &str, message: impl Into<String>) -> SimImageNodeDiagnostic {
    SimImageNodeDiagnostic {
        code: code.to_string(),
        message: message.into(),
    }
}
