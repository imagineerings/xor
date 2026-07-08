use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::DataType;

pub const DUPLICATE_NODE_CODE: &str = "world_model.comfy_nodes.duplicate";
pub const DISABLED_NODE_CODE: &str = "world_model.comfy_nodes.disabled";
pub const UNKNOWN_NODE_CODE: &str = "world_model.comfy_nodes.unknown";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ComfyNodeSource {
    Core,
    Extra,
    ApiProvider,
    Custom,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyNodeInput {
    pub name: String,
    pub data_type: DataType,
    pub required: bool,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyNodeOutput {
    pub name: String,
    pub data_type: DataType,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyNodeDefinition {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub source: ComfyNodeSource,
    pub api_node: bool,
    pub search_aliases: BTreeSet<String>,
    pub inputs: Vec<ComfyNodeInput>,
    pub outputs: Vec<ComfyNodeOutput>,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyObjectInfoNode {
    pub display_name: String,
    pub category: String,
    pub source: ComfyNodeSource,
    pub api_node: bool,
    pub search_aliases: Vec<String>,
    pub inputs: Vec<ComfyNodeInput>,
    pub outputs: Vec<ComfyNodeOutput>,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyObjectInfoResponse {
    pub nodes: BTreeMap<String, ComfyObjectInfoNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfyNodeDiagnostic {
    pub code: String,
    pub node_id: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfyNodeRegistry {
    nodes: BTreeMap<String, ComfyNodeDefinition>,
    disabled_nodes: BTreeSet<String>,
}

impl ComfyNodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_core_nodes() -> Self {
        let mut registry = Self::new();
        for node in core_nodes() {
            registry
                .register(node)
                .expect("core Comfy node ids are unique");
        }
        registry
    }

    pub fn register(&mut self, node: ComfyNodeDefinition) -> Result<(), ComfyNodeDiagnostic> {
        if self.nodes.contains_key(&node.id) {
            return Err(diagnostic(
                DUPLICATE_NODE_CODE,
                &node.id,
                "node definition id is already registered",
            ));
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub fn disable(&mut self, node_id: &str) {
        self.disabled_nodes.insert(node_id.to_string());
    }

    pub fn get(&self, node_id: &str) -> Option<&ComfyNodeDefinition> {
        self.nodes
            .get(node_id)
            .filter(|_| !self.disabled_nodes.contains(node_id))
    }

    pub fn object_info(&self, node_id: Option<&str>) -> ComfyObjectInfoResponse {
        let nodes = self
            .nodes
            .iter()
            .filter(|(id, _)| node_id.is_none_or(|requested| requested == id.as_str()))
            .filter(|(id, _)| !self.disabled_nodes.contains(*id))
            .map(|(id, node)| (id.clone(), object_info_node(node)))
            .collect();
        ComfyObjectInfoResponse { nodes }
    }

    pub fn availability(&self, node_id: &str) -> Result<(), ComfyNodeDiagnostic> {
        if self.disabled_nodes.contains(node_id) {
            return Err(diagnostic(
                DISABLED_NODE_CODE,
                node_id,
                "node is disabled by Sim launch policy",
            ));
        }
        if self.nodes.contains_key(node_id) {
            Ok(())
        } else {
            Err(diagnostic(
                UNKNOWN_NODE_CODE,
                node_id,
                "node definition is not registered",
            ))
        }
    }

    pub fn search(&self, query: &str) -> Vec<&ComfyNodeDefinition> {
        let query = normalize(query);
        self.nodes
            .values()
            .filter(|node| !self.disabled_nodes.contains(&node.id))
            .filter(|node| {
                normalize(&node.id).contains(&query)
                    || normalize(&node.display_name).contains(&query)
                    || node
                        .search_aliases
                        .iter()
                        .any(|alias| normalize(alias).contains(&query))
            })
            .collect()
    }
}

fn object_info_node(node: &ComfyNodeDefinition) -> ComfyObjectInfoNode {
    ComfyObjectInfoNode {
        display_name: node.display_name.clone(),
        category: node.category.clone(),
        source: node.source,
        api_node: node.api_node,
        search_aliases: node.search_aliases.iter().cloned().collect(),
        inputs: node.inputs.clone(),
        outputs: node.outputs.clone(),
        tooltip: node.tooltip.clone(),
    }
}

fn core_nodes() -> Vec<ComfyNodeDefinition> {
    vec![
        node(
            "CheckpointLoaderSimple",
            "Load Checkpoint",
            "loaders",
            [
                input("ckpt_name", DataType::String, true),
                output("MODEL", DataType::Model),
                output("CLIP", DataType::Clip),
                output("VAE", DataType::Vae),
            ],
            ["checkpoint", "model loader"],
        ),
        node(
            "CLIPTextEncode",
            "CLIP Text Encode",
            "conditioning",
            [
                input("text", DataType::String, true),
                input("clip", DataType::Clip, true),
                output("CONDITIONING", DataType::Conditioning),
            ],
            ["prompt", "conditioning"],
        ),
        node(
            "KSampler",
            "KSampler",
            "sampling",
            [
                input("model", DataType::Model, true),
                input("positive", DataType::Conditioning, true),
                input("negative", DataType::Conditioning, true),
                input("latent_image", DataType::Latent, true),
                output("LATENT", DataType::Latent),
            ],
            ["sampler", "diffusion"],
        ),
        node(
            "VAEDecode",
            "VAE Decode",
            "latent",
            [
                input("samples", DataType::Latent, true),
                input("vae", DataType::Vae, true),
                output("IMAGE", DataType::Image),
            ],
            ["vae", "decode"],
        ),
        node(
            "VAEEncode",
            "VAE Encode",
            "latent",
            [
                input("pixels", DataType::Image, true),
                input("vae", DataType::Vae, true),
                output("LATENT", DataType::Latent),
            ],
            ["vae", "encode"],
        ),
        node(
            "LoadImage",
            "Load Image",
            "image",
            [
                input("image", DataType::String, true),
                output("IMAGE", DataType::Image),
                output("MASK", DataType::Any),
            ],
            ["image loader"],
        ),
        node(
            "SaveImage",
            "Save Image",
            "image",
            [
                input("images", DataType::Image, true),
                output("ui", DataType::Any),
            ],
            ["image output", "artifact"],
        ),
        node(
            "LoraLoader",
            "Load LoRA",
            "loaders",
            [
                input("model", DataType::Model, true),
                input("clip", DataType::Clip, true),
                input("lora_name", DataType::String, true),
                output("MODEL", DataType::Model),
                output("CLIP", DataType::Clip),
            ],
            ["lora", "patch"],
        ),
        node(
            "ControlNetLoader",
            "Load ControlNet",
            "loaders",
            [
                input("control_net_name", DataType::String, true),
                output("CONTROL_NET", DataType::ControlNet),
            ],
            ["controlnet", "control"],
        ),
    ]
}

fn node<const N: usize, const A: usize>(
    id: &str,
    display_name: &str,
    category: &str,
    ports: [PortSpec; N],
    aliases: [&str; A],
) -> ComfyNodeDefinition {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for port in ports {
        match port {
            PortSpec::Input(input) => inputs.push(input),
            PortSpec::Output(output) => outputs.push(output),
        }
    }

    ComfyNodeDefinition {
        id: id.to_string(),
        display_name: display_name.to_string(),
        category: category.to_string(),
        source: ComfyNodeSource::Core,
        api_node: false,
        search_aliases: aliases.into_iter().map(str::to_string).collect(),
        inputs,
        outputs,
        tooltip: Some(format!("{display_name} native Sim Comfy node definition")),
    }
}

enum PortSpec {
    Input(ComfyNodeInput),
    Output(ComfyNodeOutput),
}

fn input(name: &str, data_type: DataType, required: bool) -> PortSpec {
    PortSpec::Input(ComfyNodeInput {
        name: name.to_string(),
        data_type,
        required,
        tooltip: None,
    })
}

fn output(name: &str, data_type: DataType) -> PortSpec {
    PortSpec::Output(ComfyNodeOutput {
        name: name.to_string(),
        data_type,
        tooltip: None,
    })
}

fn diagnostic(code: &str, node_id: &str, message: impl Into<String>) -> ComfyNodeDiagnostic {
    ComfyNodeDiagnostic {
        code: code.to_string(),
        node_id: node_id.to_string(),
        message: message.into(),
    }
}

fn normalize(value: &str) -> String {
    value.trim().replace([' ', '-'], "_").to_ascii_lowercase()
}
