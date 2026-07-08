use serde::{Deserialize, Serialize};

use crate::{SimImageArtifact, SimImageShape};

pub const SIM_MASK_SHAPE_MISMATCH_CODE: &str = "world_model.mask_nodes.shape_mismatch";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimMaskShape {
    pub width: u32,
    pub height: u32,
    pub batch: u32,
}

impl SimMaskShape {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            batch: 1,
        }
    }

    pub fn with_batch(mut self, batch: u32) -> Self {
        self.batch = batch;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimMaskArtifact {
    pub shape: SimMaskShape,
    pub inverted: bool,
    pub feather_radius: u32,
    pub threshold_milli: Option<u16>,
}

impl SimMaskArtifact {
    pub fn new(shape: SimMaskShape) -> Self {
        Self {
            shape,
            inverted: false,
            feather_radius: 0,
            threshold_milli: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimMaskNodeDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SimMaskNodeAdapter;

impl SimMaskNodeAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn image_to_mask(&self, image: &SimImageArtifact) -> SimMaskArtifact {
        SimMaskArtifact::new(SimMaskShape {
            width: image.shape.width,
            height: image.shape.height,
            batch: image.shape.batch,
        })
    }

    pub fn mask_to_image(&self, mask: &SimMaskArtifact) -> SimImageArtifact {
        SimImageArtifact::new(SimImageShape {
            width: mask.shape.width,
            height: mask.shape.height,
            channels: 4,
            batch: mask.shape.batch,
        })
    }

    pub fn invert(&self, mask: &SimMaskArtifact) -> SimMaskArtifact {
        let mut mask = mask.clone();
        mask.inverted = !mask.inverted;
        mask
    }

    pub fn feather(&self, mask: &SimMaskArtifact, radius: u32) -> SimMaskArtifact {
        let mut mask = mask.clone();
        mask.feather_radius = radius;
        mask
    }

    pub fn threshold(&self, mask: &SimMaskArtifact, threshold_milli: u16) -> SimMaskArtifact {
        let mut mask = mask.clone();
        mask.threshold_milli = Some(threshold_milli.min(1000));
        mask
    }

    pub fn composite_over_image(
        &self,
        image: &SimImageArtifact,
        mask: &SimMaskArtifact,
    ) -> Result<SimImageArtifact, SimMaskNodeDiagnostic> {
        if image.shape.width != mask.shape.width
            || image.shape.height != mask.shape.height
            || image.shape.batch != mask.shape.batch
        {
            return Err(SimMaskNodeDiagnostic {
                code: SIM_MASK_SHAPE_MISMATCH_CODE.to_string(),
                message: "mask composite requires matching image and mask dimensions".to_string(),
            });
        }
        let mut image = image.clone();
        image
            .metadata
            .insert("sim.mask_composited".to_string(), "true".to_string());
        Ok(image)
    }
}
