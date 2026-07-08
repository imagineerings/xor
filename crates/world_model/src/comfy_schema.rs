use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ComfyNodeDefinition, ComfyNodeInput, ComfyNodeOutput, DataType};

pub const SCHEMA_EMPTY_COMBO_CODE: &str = "world_model.comfy_schema.empty_combo";
pub const SCHEMA_UNKNOWN_TYPE_CODE: &str = "world_model.comfy_schema.unknown_type";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ComfyInputSection {
    Required,
    Optional,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ComfyInputDeclaration {
    Primitive {
        data_type: String,
        default: Option<serde_json::Value>,
    },
    Combo {
        values: Vec<String>,
        default: Option<String>,
    },
    List {
        item_type: String,
    },
    Lazy {
        data_type: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComfyInputSchemaDeclaration {
    pub name: String,
    pub section: ComfyInputSection,
    pub declaration: ComfyInputDeclaration,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimNodeInputSchema {
    pub name: String,
    pub data_type: DataType,
    pub required: bool,
    pub hidden: bool,
    pub lazy: bool,
    pub list: bool,
    pub combo_values: Vec<String>,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimNodeSchema {
    pub node_id: String,
    pub inputs: Vec<SimNodeInputSchema>,
    pub outputs: Vec<ComfyNodeOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComfySchemaDiagnostic {
    pub code: String,
    pub input_name: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComfySchemaAdapter;

impl ComfySchemaAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn adapt(
        &self,
        node: &ComfyNodeDefinition,
        declarations: Vec<ComfyInputSchemaDeclaration>,
    ) -> Result<SimNodeSchema, Vec<ComfySchemaDiagnostic>> {
        let mut diagnostics = Vec::new();
        let mut inputs = Vec::new();

        for declaration in declarations {
            match adapt_input(&declaration) {
                Ok(input) => inputs.push(input),
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
        }

        if diagnostics.is_empty() {
            Ok(SimNodeSchema {
                node_id: node.id.clone(),
                inputs,
                outputs: node.outputs.clone(),
            })
        } else {
            Err(diagnostics)
        }
    }

    pub fn from_node_definition(&self, node: &ComfyNodeDefinition) -> SimNodeSchema {
        SimNodeSchema {
            node_id: node.id.clone(),
            inputs: node.inputs.iter().map(input_from_definition).collect(),
            outputs: node.outputs.clone(),
        }
    }
}

fn input_from_definition(input: &ComfyNodeInput) -> SimNodeInputSchema {
    SimNodeInputSchema {
        name: input.name.clone(),
        data_type: input.data_type,
        required: input.required,
        hidden: false,
        lazy: false,
        list: false,
        combo_values: Vec::new(),
        tooltip: input.tooltip.clone(),
    }
}

fn adapt_input(
    input: &ComfyInputSchemaDeclaration,
) -> Result<SimNodeInputSchema, ComfySchemaDiagnostic> {
    let (data_type, lazy, list, combo_values) = match &input.declaration {
        ComfyInputDeclaration::Primitive { data_type, .. } => {
            (data_type_for_name(data_type)?, false, false, Vec::new())
        }
        ComfyInputDeclaration::Combo { values, .. } => {
            if values.is_empty() {
                return Err(diagnostic(
                    SCHEMA_EMPTY_COMBO_CODE,
                    &input.name,
                    "combo input must include at least one value",
                ));
            }
            (DataType::String, false, false, values.clone())
        }
        ComfyInputDeclaration::List { item_type } => {
            (data_type_for_name(item_type)?, false, true, Vec::new())
        }
        ComfyInputDeclaration::Lazy { data_type } => {
            (data_type_for_name(data_type)?, true, false, Vec::new())
        }
    };

    Ok(SimNodeInputSchema {
        name: input.name.clone(),
        data_type,
        required: input.section == ComfyInputSection::Required,
        hidden: input.section == ComfyInputSection::Hidden,
        lazy,
        list,
        combo_values,
        tooltip: input.tooltip.clone(),
    })
}

fn data_type_for_name(name: &str) -> Result<DataType, ComfySchemaDiagnostic> {
    let normalized = normalize_type_name(name);
    match normalized.as_str() {
        "image" => Ok(DataType::Image),
        "latent" | "latent_image" => Ok(DataType::Latent),
        "conditioning" => Ok(DataType::Conditioning),
        "model" => Ok(DataType::Model),
        "control_net" | "controlnet" => Ok(DataType::ControlNet),
        "vae" => Ok(DataType::Vae),
        "clip" => Ok(DataType::Clip),
        "float" => Ok(DataType::Float),
        "int" | "integer" => Ok(DataType::Int),
        "string" | "str" => Ok(DataType::String),
        "bool" | "boolean" => Ok(DataType::Bool),
        "*" | "any" => Ok(DataType::Any),
        _ => Err(diagnostic(
            SCHEMA_UNKNOWN_TYPE_CODE,
            name,
            format!("unknown Comfy input data type `{name}`"),
        )),
    }
}

pub fn declarations_by_section(
    declarations: impl IntoIterator<Item = ComfyInputSchemaDeclaration>,
) -> BTreeMap<ComfyInputSection, Vec<ComfyInputSchemaDeclaration>> {
    let mut sections = BTreeMap::new();
    for declaration in declarations {
        sections
            .entry(declaration.section)
            .or_insert_with(Vec::new)
            .push(declaration);
    }
    sections
}

fn diagnostic(code: &str, input_name: &str, message: impl Into<String>) -> ComfySchemaDiagnostic {
    ComfySchemaDiagnostic {
        code: code.to_string(),
        input_name: input_name.to_string(),
        message: message.into(),
    }
}

fn normalize_type_name(name: &str) -> String {
    name.trim().replace([' ', '-'], "_").to_ascii_lowercase()
}
