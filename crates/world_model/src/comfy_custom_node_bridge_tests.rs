use std::path::PathBuf;

use crate::{
    ComfyNodeInput, ComfyNodeOutput, ComfyNodeRegistry, ComfyNodeSource, DataType,
    SIM_CUSTOM_NODE_DUPLICATE_CODE, SIM_CUSTOM_NODE_UNSUPPORTED_REGISTRATION_CODE,
    SimCustomNodeBridge, SimCustomNodeDeclaration, SimCustomNodeRegistrationKind, SimExtensionId,
    SimExtensionRecord, SimExtensionSourceKind,
};

#[test]
fn custom_node_bridge_registers_v1_node_class_mappings_as_native_nodes() {
    let extension = extension("paint_pack");
    let declaration = SimCustomNodeDeclaration::new(
        &extension,
        "PaintMask",
        "PaintMaskNode",
        SimCustomNodeRegistrationKind::V1Mapping,
    )
    .with_display_name("Paint Mask")
    .with_category("masking")
    .with_input(input("image", DataType::Image))
    .with_output(output("MASK", DataType::Any));
    let mut registry = ComfyNodeRegistry::new();

    let report = SimCustomNodeBridge::new().register(&mut registry, &extension, vec![declaration]);

    assert_eq!(report.registered.len(), 1);
    assert!(report.diagnostics.is_empty());
    let node = registry.get("PaintMask").expect("custom node registered");
    assert_eq!(node.display_name, "Paint Mask");
    assert_eq!(node.category, "masking");
    assert_eq!(node.source, ComfyNodeSource::Custom);
    assert!(!node.api_node);
}

#[test]
fn custom_node_bridge_registers_modern_entrypoint_metadata() {
    let extension = extension("modern_pack");
    let declaration = SimCustomNodeDeclaration::new(
        &extension,
        "ModernSampler",
        "ModernSamplerNode",
        SimCustomNodeRegistrationKind::ModernEntrypoint,
    )
    .with_output(output("LATENT", DataType::Latent));
    let mut registry = ComfyNodeRegistry::new();

    let report = SimCustomNodeBridge::new().register(&mut registry, &extension, vec![declaration]);

    assert_eq!(
        report.registered[0].registration_kind,
        SimCustomNodeRegistrationKind::ModernEntrypoint
    );
    assert_eq!(report.registered[0].module.extension_id, extension.id);
    assert!(registry.get("ModernSampler").is_some());
}

#[test]
fn custom_node_bridge_reports_unsupported_registration_mechanism() {
    let extension = extension("unsupported_pack");
    let mut registry = ComfyNodeRegistry::new();

    let report = SimCustomNodeBridge::new().register(&mut registry, &extension, Vec::new());

    assert!(report.registered.is_empty());
    assert_eq!(
        report.diagnostics[0].code,
        SIM_CUSTOM_NODE_UNSUPPORTED_REGISTRATION_CODE
    );
    assert!(registry.object_info(None).nodes.is_empty());
}

#[test]
fn custom_node_bridge_reports_duplicate_node_diagnostics() {
    let extension = extension("duplicate_pack");
    let declaration = SimCustomNodeDeclaration::new(
        &extension,
        "DuplicateNode",
        "DuplicateNodeClass",
        SimCustomNodeRegistrationKind::V1Mapping,
    );
    let mut registry = ComfyNodeRegistry::new();
    registry
        .register(crate::ComfyNodeDefinition {
            id: "DuplicateNode".to_string(),
            display_name: "Duplicate".to_string(),
            category: "test".to_string(),
            source: ComfyNodeSource::Custom,
            api_node: false,
            search_aliases: Default::default(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            tooltip: None,
        })
        .expect("seed node");

    let report = SimCustomNodeBridge::new().register(&mut registry, &extension, vec![declaration]);

    assert!(report.registered.is_empty());
    assert_eq!(report.diagnostics[0].code, SIM_CUSTOM_NODE_DUPLICATE_CODE);
    assert_eq!(
        report.diagnostics[0].node_id.as_deref(),
        Some("DuplicateNode")
    );
}

fn extension(name: &str) -> SimExtensionRecord {
    SimExtensionRecord {
        id: SimExtensionId::new(name),
        display_name: name.to_string(),
        source_path: PathBuf::from(format!("/custom_nodes/{name}")),
        source_kind: SimExtensionSourceKind::Directory,
        root_index: 0,
        load_order: 0,
    }
}

fn input(name: &str, data_type: DataType) -> ComfyNodeInput {
    ComfyNodeInput {
        name: name.to_string(),
        data_type,
        required: true,
        tooltip: None,
    }
}

fn output(name: &str, data_type: DataType) -> ComfyNodeOutput {
    ComfyNodeOutput {
        name: name.to_string(),
        data_type,
        tooltip: None,
    }
}
