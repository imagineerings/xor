use crate::{
    CONFLICTING_REPLACEMENT_MAPPING_CODE, ComfyNodeRegistry, ComfyReplacementCatalog,
    ComfyReplacementEntry, ComfyReplacementSource, DUPLICATE_REPLACEMENT_MAPPING_CODE, DataType,
    DiffusionGraph, GraphEdge, GraphNode, NodePort, NodeReplacementRule, PortDirection,
};

#[test]
fn replacement_catalog_dedupes_identical_mappings_as_native_records() {
    let entry = ComfyReplacementEntry::new(
        NodeReplacementRule::new("LegacyTextEncode", "CLIPTextEncode")
            .with_input_mapping("prompt", "text")
            .with_output_mapping("old_conditioning", "CONDITIONING"),
        ComfyReplacementSource::BuiltIn,
    )
    .with_metadata("reason", "legacy workflow import");
    let mut catalog = ComfyReplacementCatalog::default();

    catalog.register(entry.clone());
    catalog.register(entry);

    assert_eq!(catalog.len(), 1);
    assert!(catalog.rule_for("LegacyTextEncode").is_some());
    assert_eq!(
        catalog
            .entry_for("LegacyTextEncode")
            .and_then(|entry| entry.metadata.get("reason"))
            .map(String::as_str),
        Some("legacy workflow import")
    );
    assert!(catalog.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == DUPLICATE_REPLACEMENT_MAPPING_CODE
            && diagnostic.from_node_type == "LegacyTextEncode"
    }));
}

#[test]
fn replacement_catalog_reports_conflicts_and_keeps_first_mapping() {
    let mut catalog = ComfyReplacementCatalog::default();
    catalog.register(ComfyReplacementEntry::new(
        NodeReplacementRule::new("LegacySampler", "KSampler"),
        ComfyReplacementSource::CustomNode {
            node_pack_name: "pack-a".to_string(),
            source_path: "custom_nodes/pack-a/replacements.json".to_string(),
        },
    ));
    catalog.register(ComfyReplacementEntry::new(
        NodeReplacementRule::new("LegacySampler", "SamplerCustom"),
        ComfyReplacementSource::ImportedWorkflow {
            source_path: "workflows/old.json".to_string(),
        },
    ));

    assert_eq!(catalog.len(), 1);
    assert_eq!(
        catalog
            .rule_for("LegacySampler")
            .map(|rule| rule.to_node_type.as_str()),
        Some("KSampler")
    );
    assert!(catalog.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == CONFLICTING_REPLACEMENT_MAPPING_CODE
            && diagnostic.from_node_type == "LegacySampler"
    }));
}

#[test]
fn replacement_catalog_exposes_rules_to_graph_validation_and_import() {
    let catalog = ComfyReplacementCatalog::new([ComfyReplacementEntry::new(
        NodeReplacementRule::new("LegacyTextEncode", "CLIPTextEncode")
            .with_input_mapping("prompt", "text")
            .with_output_mapping("old_conditioning", "CONDITIONING"),
        ComfyReplacementSource::BuiltIn,
    )]);
    let registry = ComfyNodeRegistry::with_core_nodes();
    let graph = DiffusionGraph::new()
        .with_node(
            GraphNode::new(1, "LegacyTextEncode")
                .with_port(NodePort::new(
                    "prompt",
                    PortDirection::Input,
                    DataType::String,
                    true,
                ))
                .with_port(NodePort::new(
                    "old_conditioning",
                    PortDirection::Output,
                    DataType::Conditioning,
                    false,
                ))
                .with_metadata("prompt", "a castle"),
        )
        .with_node(GraphNode::new(2, "KSampler").with_port(NodePort::new(
            "positive",
            PortDirection::Input,
            DataType::Conditioning,
            true,
        )))
        .with_edge(GraphEdge::new(1, 1, "old_conditioning", 2, "positive"));

    let report = catalog.apply_to_graph(&graph, &registry);

    assert_eq!(report.replaced_nodes, vec![1]);
    assert!(report.diagnostics.is_empty());
    let node = report.graph.node_by_id(1).expect("node should exist");
    assert_eq!(node.node_type, "CLIPTextEncode");
    assert_eq!(
        node.metadata.get("text").map(String::as_str),
        Some("a castle")
    );
    assert_eq!(
        report
            .graph
            .edge_by_id(1)
            .map(|edge| edge.source_port.as_str()),
        Some("CONDITIONING")
    );
}

#[test]
fn replacement_catalog_preserves_custom_node_source_metadata() {
    let catalog = ComfyReplacementCatalog::new([ComfyReplacementEntry::new(
        NodeReplacementRule::new("PackLegacyNode", "PackNativeNode"),
        ComfyReplacementSource::CustomNode {
            node_pack_name: "pack-a".to_string(),
            source_path: "custom_nodes/pack-a/replacements.json".to_string(),
        },
    )]);

    let entry = catalog
        .entry_for("PackLegacyNode")
        .expect("entry should exist");

    assert_eq!(
        entry.source,
        ComfyReplacementSource::CustomNode {
            node_pack_name: "pack-a".to_string(),
            source_path: "custom_nodes/pack-a/replacements.json".to_string(),
        }
    );
}
