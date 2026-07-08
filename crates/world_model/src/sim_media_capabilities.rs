use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const SIM_MEDIA_DEPENDENCY_REVIEW_REQUIRED_CODE: &str =
    "world_model.media_capability.dependency_review_required";
pub const SIM_MEDIA_UNSUPPORTED_BACKEND_CODE: &str =
    "world_model.media_capability.unsupported_backend";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimMediaCapabilityGroup {
    ImageMask,
    Video,
    Audio,
    ThreeDGeometry,
    AnalysisControl,
    Utility,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SimMediaPortType {
    Image,
    Mask,
    Video,
    Audio,
    Latent,
    Text,
    Number,
    Boolean,
    Json,
    Mesh,
    PointCloud,
    GaussianSplat,
    DepthMap,
    Pose,
    BoundingBoxes,
    Segmentation,
    ControlSignal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimMediaBackendRequirement {
    NativeSim { service: String },
    SimAssetService,
    SimMediaService,
    MeshPipelineDelegation,
    DependencyReviewRequired { reason: String },
    Unsupported { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimMediaNodeCapability {
    pub node_type: String,
    pub source_module: String,
    pub group: SimMediaCapabilityGroup,
    pub inputs: Vec<SimMediaPortType>,
    pub outputs: Vec<SimMediaPortType>,
    pub backend: SimMediaBackendRequirement,
    pub native_sim_handler: String,
    pub schema_ref: String,
    pub developer_only: bool,
}

impl SimMediaNodeCapability {
    pub fn new(
        node_type: impl Into<String>,
        source_module: impl Into<String>,
        group: SimMediaCapabilityGroup,
        backend: SimMediaBackendRequirement,
        native_sim_handler: impl Into<String>,
    ) -> Self {
        let node_type = node_type.into();
        Self {
            schema_ref: format!("#/media_nodes/{node_type}"),
            node_type,
            source_module: source_module.into(),
            group,
            inputs: Vec::new(),
            outputs: Vec::new(),
            backend,
            native_sim_handler: native_sim_handler.into(),
            developer_only: false,
        }
    }

    pub fn with_inputs(mut self, inputs: impl IntoIterator<Item = SimMediaPortType>) -> Self {
        self.inputs = inputs.into_iter().collect();
        self
    }

    pub fn with_outputs(mut self, outputs: impl IntoIterator<Item = SimMediaPortType>) -> Self {
        self.outputs = outputs.into_iter().collect();
        self
    }

    pub fn developer_only(mut self) -> Self {
        self.developer_only = true;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimMediaCapabilityDiagnostic {
    pub code: String,
    pub node_type: String,
    pub group: SimMediaCapabilityGroup,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimMediaNodeCapabilityRegistry {
    capabilities: BTreeMap<String, SimMediaNodeCapability>,
}

impl Default for SimMediaNodeCapabilityRegistry {
    fn default() -> Self {
        Self::new(default_capabilities())
    }
}

impl SimMediaNodeCapabilityRegistry {
    pub fn new(capabilities: impl IntoIterator<Item = SimMediaNodeCapability>) -> Self {
        Self {
            capabilities: capabilities
                .into_iter()
                .map(|capability| (capability.node_type.clone(), capability))
                .collect(),
        }
    }

    pub fn default_capabilities() -> Self {
        Self::default()
    }

    pub fn capability(&self, node_type: &str) -> Option<&SimMediaNodeCapability> {
        self.capabilities.get(node_type)
    }

    pub fn capabilities(&self) -> impl Iterator<Item = &SimMediaNodeCapability> {
        self.capabilities.values()
    }

    pub fn by_group(&self, group: SimMediaCapabilityGroup) -> Vec<&SimMediaNodeCapability> {
        self.capabilities()
            .filter(|capability| capability.group == group)
            .collect()
    }

    pub fn visible_capabilities(&self, developer_mode: bool) -> Vec<&SimMediaNodeCapability> {
        self.capabilities()
            .filter(|capability| developer_mode || !capability.developer_only)
            .collect()
    }

    pub fn groups(&self) -> BTreeSet<SimMediaCapabilityGroup> {
        self.capabilities()
            .map(|capability| capability.group)
            .collect()
    }

    pub fn diagnostics(&self) -> Vec<SimMediaCapabilityDiagnostic> {
        self.capabilities()
            .filter_map(capability_diagnostic)
            .collect()
    }
}

fn capability_diagnostic(
    capability: &SimMediaNodeCapability,
) -> Option<SimMediaCapabilityDiagnostic> {
    match &capability.backend {
        SimMediaBackendRequirement::DependencyReviewRequired { reason } => {
            Some(SimMediaCapabilityDiagnostic {
                code: SIM_MEDIA_DEPENDENCY_REVIEW_REQUIRED_CODE.to_string(),
                node_type: capability.node_type.clone(),
                group: capability.group,
                message: reason.clone(),
            })
        }
        SimMediaBackendRequirement::Unsupported { reason } => Some(SimMediaCapabilityDiagnostic {
            code: SIM_MEDIA_UNSUPPORTED_BACKEND_CODE.to_string(),
            node_type: capability.node_type.clone(),
            group: capability.group,
            message: reason.clone(),
        }),
        SimMediaBackendRequirement::NativeSim { .. }
        | SimMediaBackendRequirement::SimAssetService
        | SimMediaBackendRequirement::SimMediaService
        | SimMediaBackendRequirement::MeshPipelineDelegation => None,
    }
}

fn default_capabilities() -> Vec<SimMediaNodeCapability> {
    vec![
        image("LoadImage", &[], &[SimMediaPortType::Image]),
        image("SaveImage", &[SimMediaPortType::Image], &[]),
        image(
            "PreviewImage",
            &[SimMediaPortType::Image],
            &[SimMediaPortType::Image],
        ),
        image(
            "ImageResize",
            &[SimMediaPortType::Image, SimMediaPortType::Number],
            &[SimMediaPortType::Image],
        ),
        image(
            "ImageCrop",
            &[SimMediaPortType::Image],
            &[SimMediaPortType::Image],
        ),
        image(
            "MaskToImage",
            &[SimMediaPortType::Mask],
            &[SimMediaPortType::Image],
        ),
        image(
            "ImageToMask",
            &[SimMediaPortType::Image],
            &[SimMediaPortType::Mask],
        ),
        image("SolidMask", &[], &[SimMediaPortType::Mask]),
        image(
            "ImageBlend",
            &[
                SimMediaPortType::Image,
                SimMediaPortType::Image,
                SimMediaPortType::Mask,
            ],
            &[SimMediaPortType::Image],
        ),
        video("LoadVideo", &[], &[SimMediaPortType::Video]),
        video("SaveVideo", &[SimMediaPortType::Video], &[]),
        video(
            "VideoSlice",
            &[SimMediaPortType::Video, SimMediaPortType::Number],
            &[SimMediaPortType::Video],
        ),
        reviewed_video(
            "FrameInterpolation",
            &[SimMediaPortType::Video],
            &[SimMediaPortType::Video],
            "frame interpolation requires an approved native/video backend",
        ),
        audio("LoadAudio", &[], &[SimMediaPortType::Audio]),
        audio("SaveAudio", &[SimMediaPortType::Audio], &[]),
        audio(
            "PreviewAudio",
            &[SimMediaPortType::Audio],
            &[SimMediaPortType::Audio],
        ),
        audio(
            "AudioVAEEncode",
            &[SimMediaPortType::Audio],
            &[SimMediaPortType::Latent],
        ),
        audio(
            "AudioVAEDecode",
            &[SimMediaPortType::Latent],
            &[SimMediaPortType::Audio],
        ),
        three_d("Load3D", &[], &[SimMediaPortType::Mesh]),
        three_d(
            "Preview3D",
            &[SimMediaPortType::Mesh],
            &[SimMediaPortType::Mesh],
        ),
        three_d("Save3D", &[SimMediaPortType::Mesh], &[]),
        three_d(
            "GaussianSplatPreview",
            &[SimMediaPortType::GaussianSplat],
            &[SimMediaPortType::Image],
        ),
        mesh_delegated(
            "TexturedMeshExport",
            &[SimMediaPortType::Mesh, SimMediaPortType::Image],
            &[SimMediaPortType::Mesh],
        ),
        analysis(
            "CannyEdgePreprocessor",
            &[SimMediaPortType::Image],
            &[SimMediaPortType::ControlSignal],
        ),
        analysis(
            "OpenPosePreprocessor",
            &[SimMediaPortType::Image],
            &[SimMediaPortType::Pose],
        ),
        analysis(
            "DepthAnythingPreprocessor",
            &[SimMediaPortType::Image],
            &[SimMediaPortType::DepthMap],
        ),
        analysis(
            "FaceDetection",
            &[SimMediaPortType::Image],
            &[SimMediaPortType::BoundingBoxes],
        ),
        unsupported_analysis(
            "SamDetector",
            &[SimMediaPortType::Image],
            &[SimMediaPortType::Segmentation],
            "SAM3 segmentation backend is not yet available in native Sim",
        ),
        utility("StringPrimitive", &[], &[SimMediaPortType::Text], false),
        utility(
            "RegexExtract",
            &[SimMediaPortType::Text],
            &[SimMediaPortType::Text],
            false,
        ),
        utility(
            "JsonExtract",
            &[SimMediaPortType::Json],
            &[SimMediaPortType::Text],
            false,
        ),
        utility("Seed", &[], &[SimMediaPortType::Number], false),
        utility(
            "DatasetShuffle",
            &[SimMediaPortType::Json],
            &[SimMediaPortType::Json],
            true,
        ),
    ]
}

fn image(
    node_type: &str,
    inputs: &[SimMediaPortType],
    outputs: &[SimMediaPortType],
) -> SimMediaNodeCapability {
    SimMediaNodeCapability::new(
        node_type,
        "projects/comfy/comfy_extras/nodes_images.py",
        SimMediaCapabilityGroup::ImageMask,
        SimMediaBackendRequirement::SimMediaService,
        "sim.media.image",
    )
    .with_inputs(inputs.iter().copied())
    .with_outputs(outputs.iter().copied())
}

fn video(
    node_type: &str,
    inputs: &[SimMediaPortType],
    outputs: &[SimMediaPortType],
) -> SimMediaNodeCapability {
    SimMediaNodeCapability::new(
        node_type,
        "projects/comfy/comfy_extras/nodes_video.py",
        SimMediaCapabilityGroup::Video,
        SimMediaBackendRequirement::SimMediaService,
        "sim.media.video",
    )
    .with_inputs(inputs.iter().copied())
    .with_outputs(outputs.iter().copied())
}

fn reviewed_video(
    node_type: &str,
    inputs: &[SimMediaPortType],
    outputs: &[SimMediaPortType],
    reason: &str,
) -> SimMediaNodeCapability {
    SimMediaNodeCapability::new(
        node_type,
        "projects/comfy/comfy_extras/nodes_video.py",
        SimMediaCapabilityGroup::Video,
        SimMediaBackendRequirement::DependencyReviewRequired {
            reason: reason.to_string(),
        },
        "sim.media.video",
    )
    .with_inputs(inputs.iter().copied())
    .with_outputs(outputs.iter().copied())
}

fn audio(
    node_type: &str,
    inputs: &[SimMediaPortType],
    outputs: &[SimMediaPortType],
) -> SimMediaNodeCapability {
    SimMediaNodeCapability::new(
        node_type,
        "projects/comfy/comfy_extras/nodes_audio.py",
        SimMediaCapabilityGroup::Audio,
        SimMediaBackendRequirement::SimMediaService,
        "sim.media.audio",
    )
    .with_inputs(inputs.iter().copied())
    .with_outputs(outputs.iter().copied())
}

fn three_d(
    node_type: &str,
    inputs: &[SimMediaPortType],
    outputs: &[SimMediaPortType],
) -> SimMediaNodeCapability {
    SimMediaNodeCapability::new(
        node_type,
        "projects/comfy/comfy_extras/nodes_3d.py",
        SimMediaCapabilityGroup::ThreeDGeometry,
        SimMediaBackendRequirement::SimAssetService,
        "sim.assets.3d",
    )
    .with_inputs(inputs.iter().copied())
    .with_outputs(outputs.iter().copied())
}

fn mesh_delegated(
    node_type: &str,
    inputs: &[SimMediaPortType],
    outputs: &[SimMediaPortType],
) -> SimMediaNodeCapability {
    SimMediaNodeCapability::new(
        node_type,
        "projects/comfy/comfy_extras/nodes_3d.py",
        SimMediaCapabilityGroup::ThreeDGeometry,
        SimMediaBackendRequirement::MeshPipelineDelegation,
        "sim.mesh.pipeline",
    )
    .with_inputs(inputs.iter().copied())
    .with_outputs(outputs.iter().copied())
}

fn analysis(
    node_type: &str,
    inputs: &[SimMediaPortType],
    outputs: &[SimMediaPortType],
) -> SimMediaNodeCapability {
    SimMediaNodeCapability::new(
        node_type,
        "projects/comfy/comfy_extras/nodes_control.py",
        SimMediaCapabilityGroup::AnalysisControl,
        SimMediaBackendRequirement::NativeSim {
            service: "sim.control.analysis".to_string(),
        },
        "sim.control.analysis",
    )
    .with_inputs(inputs.iter().copied())
    .with_outputs(outputs.iter().copied())
}

fn unsupported_analysis(
    node_type: &str,
    inputs: &[SimMediaPortType],
    outputs: &[SimMediaPortType],
    reason: &str,
) -> SimMediaNodeCapability {
    SimMediaNodeCapability::new(
        node_type,
        "projects/comfy/comfy_extras/nodes_control.py",
        SimMediaCapabilityGroup::AnalysisControl,
        SimMediaBackendRequirement::Unsupported {
            reason: reason.to_string(),
        },
        "sim.control.analysis",
    )
    .with_inputs(inputs.iter().copied())
    .with_outputs(outputs.iter().copied())
}

fn utility(
    node_type: &str,
    inputs: &[SimMediaPortType],
    outputs: &[SimMediaPortType],
    developer_only: bool,
) -> SimMediaNodeCapability {
    let capability = SimMediaNodeCapability::new(
        node_type,
        "projects/comfy/comfy_extras/nodes_utility.py",
        SimMediaCapabilityGroup::Utility,
        SimMediaBackendRequirement::NativeSim {
            service: "sim.media.utility".to_string(),
        },
        "sim.media.utility",
    )
    .with_inputs(inputs.iter().copied())
    .with_outputs(outputs.iter().copied());

    if developer_only {
        capability.developer_only()
    } else {
        capability
    }
}
