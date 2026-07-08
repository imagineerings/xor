use serde::{Deserialize, Serialize};

use crate::{
    ComfyNodeDefinition, ComfyNodeInput, ComfyNodeOutput, ComfyNodeRegistry, ComfyNodeSource,
    SimExtensionId, SimExtensionRecord,
};

pub const SIM_CUSTOM_NODE_DUPLICATE_CODE: &str = "world_model.custom_nodes.duplicate";
pub const SIM_CUSTOM_NODE_UNSUPPORTED_REGISTRATION_CODE: &str =
    "world_model.custom_nodes.unsupported_registration";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SimCustomNodeRegistrationKind {
    V1Mapping,
    ModernEntrypoint,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCustomNodeModuleMetadata {
    pub extension_id: SimExtensionId,
    pub module_name: String,
    pub relative_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCustomNodeDeclaration {
    pub node_id: String,
    pub class_name: String,
    pub display_name: Option<String>,
    pub category: String,
    pub registration_kind: SimCustomNodeRegistrationKind,
    pub module: SimCustomNodeModuleMetadata,
    pub inputs: Vec<ComfyNodeInput>,
    pub outputs: Vec<ComfyNodeOutput>,
}

impl SimCustomNodeDeclaration {
    pub fn new(
        extension: &SimExtensionRecord,
        node_id: impl Into<String>,
        class_name: impl Into<String>,
        registration_kind: SimCustomNodeRegistrationKind,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            class_name: class_name.into(),
            display_name: None,
            category: extension.display_name.clone(),
            registration_kind,
            module: SimCustomNodeModuleMetadata {
                extension_id: extension.id.clone(),
                module_name: extension.display_name.clone(),
                relative_path: extension.source_path.display().to_string(),
            },
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    pub fn with_input(mut self, input: ComfyNodeInput) -> Self {
        self.inputs.push(input);
        self
    }

    pub fn with_output(mut self, output: ComfyNodeOutput) -> Self {
        self.outputs.push(output);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCustomNodeRegistrationRecord {
    pub extension_id: SimExtensionId,
    pub node_id: String,
    pub registration_kind: SimCustomNodeRegistrationKind,
    pub module: SimCustomNodeModuleMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCustomNodeBridgeDiagnostic {
    pub code: String,
    pub extension_id: SimExtensionId,
    pub node_id: Option<String>,
    pub message: String,
}

impl SimCustomNodeBridgeDiagnostic {
    fn new(
        code: impl Into<String>,
        extension_id: SimExtensionId,
        node_id: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            extension_id,
            node_id,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCustomNodeRegistrationReport {
    pub registered: Vec<SimCustomNodeRegistrationRecord>,
    pub diagnostics: Vec<SimCustomNodeBridgeDiagnostic>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SimCustomNodeBridge;

impl SimCustomNodeBridge {
    pub fn new() -> Self {
        Self
    }

    pub fn register(
        &self,
        registry: &mut ComfyNodeRegistry,
        extension: &SimExtensionRecord,
        declarations: Vec<SimCustomNodeDeclaration>,
    ) -> SimCustomNodeRegistrationReport {
        let mut report = SimCustomNodeRegistrationReport::default();

        if declarations.is_empty() {
            report
                .diagnostics
                .push(SimCustomNodeBridgeDiagnostic::new(
                    SIM_CUSTOM_NODE_UNSUPPORTED_REGISTRATION_CODE,
                    extension.id.clone(),
                    None,
                    "extension does not expose NODE_CLASS_MAPPINGS or a supported native Sim entrypoint",
                ));
            return report;
        }

        for declaration in declarations {
            let node = node_definition(&declaration);
            match registry.register(node) {
                Ok(()) => report.registered.push(SimCustomNodeRegistrationRecord {
                    extension_id: extension.id.clone(),
                    node_id: declaration.node_id,
                    registration_kind: declaration.registration_kind,
                    module: declaration.module,
                }),
                Err(diagnostic) => report.diagnostics.push(SimCustomNodeBridgeDiagnostic::new(
                    SIM_CUSTOM_NODE_DUPLICATE_CODE,
                    extension.id.clone(),
                    Some(declaration.node_id),
                    diagnostic.message,
                )),
            }
        }

        report
    }
}

fn node_definition(declaration: &SimCustomNodeDeclaration) -> ComfyNodeDefinition {
    ComfyNodeDefinition {
        id: declaration.node_id.clone(),
        display_name: declaration
            .display_name
            .clone()
            .unwrap_or_else(|| declaration.class_name.clone()),
        category: declaration.category.clone(),
        source: ComfyNodeSource::Custom,
        api_node: false,
        search_aliases: [
            declaration.class_name.clone(),
            declaration.module.module_name.clone(),
        ]
        .into_iter()
        .collect(),
        inputs: declaration.inputs.clone(),
        outputs: declaration.outputs.clone(),
        tooltip: Some(format!(
            "Custom node from native Sim extension {}",
            declaration.module.extension_id.as_str()
        )),
    }
}
