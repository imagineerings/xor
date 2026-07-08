use std::path::PathBuf;

use serde_json::json;

use crate::{
    ComfySubgraphIndex, ComfyWorkflowTemplateAdapter, ComfyWorkflowTemplateAsset,
    SIM_EXTENSION_I18N_INVALID_BUNDLE_CODE, SIM_EXTENSION_SUBGRAPH_INDEXED_CODE,
    SIM_EXTENSION_TEMPLATE_INDEXED_CODE, SimExtensionId, SimExtensionLocaleBundle,
    SimExtensionLocaleFileKind, SimExtensionLocaleMerger, SimExtensionRecord,
    SimExtensionSourceKind, SimExtensionSubgraphDeclaration, SimExtensionTemplateDeclaration,
    SimExtensionTemplateIndexer, locale_languages,
};

#[test]
fn extension_locale_merger_merges_supported_locale_files_by_language() {
    let first = extension("Locale Pack");
    let second = extension("Override Pack");
    let report = SimExtensionLocaleMerger::new().merge([
        SimExtensionLocaleBundle::new(first.id.clone(), "EN")
            .with_file(
                SimExtensionLocaleFileKind::Main,
                json!({"menu": {"run": "Run"}, "title": "Original"}),
            )
            .with_file(
                SimExtensionLocaleFileKind::NodeDefs,
                json!({"Sampler": {"display_name": "Sampler"}}),
            ),
        SimExtensionLocaleBundle::new(second.id.clone(), "en").with_file(
            SimExtensionLocaleFileKind::Main,
            json!({"menu": {"stop": "Stop"}, "title": "Override"}),
        ),
    ]);

    assert!(report.diagnostics.is_empty());
    assert!(locale_languages(&report).contains("en"));
    let language = &report.languages[0];
    assert_eq!(language.extension_ids, vec![first.id, second.id]);
    assert_eq!(
        language.files[&SimExtensionLocaleFileKind::Main]["title"],
        "Override"
    );
    assert_eq!(
        language.files[&SimExtensionLocaleFileKind::Main]["menu"]["run"],
        "Run"
    );
    assert_eq!(
        language.files[&SimExtensionLocaleFileKind::Main]["menu"]["stop"],
        "Stop"
    );
    assert_eq!(
        language.files[&SimExtensionLocaleFileKind::NodeDefs]["Sampler"]["display_name"],
        "Sampler"
    );
}

#[test]
fn extension_locale_merger_reports_invalid_bundle_files() {
    let extension = extension("Broken Locale");
    let report = SimExtensionLocaleMerger::new().merge([SimExtensionLocaleBundle::new(
        extension.id.clone(),
        "en",
    )
    .with_file(
        SimExtensionLocaleFileKind::Commands,
        json!(["not", "object"]),
    )]);

    assert!(report.languages[0].files.is_empty());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(
        report.diagnostics[0].code,
        SIM_EXTENSION_I18N_INVALID_BUNDLE_CODE
    );
    assert_eq!(report.diagnostics[0].extension_id, extension.id);
}

#[test]
fn extension_template_indexer_feeds_native_template_and_subgraph_indexes() {
    let extension = extension("Template Pack");
    let mut templates = ComfyWorkflowTemplateAdapter::default();
    let mut subgraphs = ComfySubgraphIndex::default();
    let report = SimExtensionTemplateIndexer::new().index(
        [SimExtensionTemplateDeclaration::new(
            &extension,
            "Starter",
            "custom_nodes/template_pack/example_workflows/starter.json",
            json!({"nodes": [{"id": 1, "type": "KSampler"}], "links": []}),
        )
        .with_asset(ComfyWorkflowTemplateAsset::new(
            "preview",
            "custom_nodes/template_pack/example_workflows/preview.webp",
            "image/webp",
        ))
        .with_metadata(json!({"category": "image", "token": "hidden"}))],
        [SimExtensionSubgraphDeclaration::new(
            &extension,
            "Reusable Sampler",
            "custom_nodes/template_pack/subgraphs/sampler.json",
            json!({"nodes": [{"id": 2, "type": "KSampler"}], "links": []}),
        )
        .with_metadata(json!({"category": "sampling"}))],
        &mut templates,
        &mut subgraphs,
    );

    assert_eq!(report.template_ids.len(), 1);
    assert_eq!(report.subgraph_ids.len(), 1);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_TEMPLATE_INDEXED_CODE
            && diagnostic.extension_id == extension.id
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SIM_EXTENSION_SUBGRAPH_INDEXED_CODE
            && diagnostic.extension_id == extension.id
    }));

    let template = templates
        .open(&report.template_ids[0])
        .expect("template should be indexed");
    assert_eq!(template.source.node_pack_name(), "template-pack");
    assert!(template.metadata.get("token").is_none());

    let subgraph = subgraphs
        .open(&report.subgraph_ids[0])
        .expect("subgraph should be indexed");
    assert_eq!(subgraph.source.node_pack_name(), Some("template-pack"));
    assert_eq!(subgraph.node_count, 1);
}

#[test]
fn extension_template_indexer_surfaces_native_index_diagnostics() {
    let extension = extension("Unsafe Template");
    let mut templates = ComfyWorkflowTemplateAdapter::default();
    let mut subgraphs = ComfySubgraphIndex::default();
    let report = SimExtensionTemplateIndexer::new().index(
        [SimExtensionTemplateDeclaration::new(
            &extension,
            "Unsafe",
            "../outside.json",
            json!({"nodes": [], "links": []}),
        )],
        [],
        &mut templates,
        &mut subgraphs,
    );

    assert!(report.template_ids.is_empty());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].extension_id, extension.id);
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
